//! Entradas SQLite/framing e CAS, implementadas a partir dos formatos normativos.

use std::fs::{self, File};
use std::io::{Read, Seek};
use std::path::Path;

use logos_crypto::{Domain, Hash, hash_raw, tags};
use rusqlite::{Connection, OpenFlags, limits::Limit};

use crate::event::MAX_WIRE;
use crate::{Result, fail};

pub(crate) const MAGIC: &[u8] = b"LOGOS-AUDIT/v1\n";
const SCHEMA: &str = "CREATE TABLE events (seq INTEGER PRIMARY KEY CHECK (seq >= 0), wire BLOB NOT NULL CHECK (length(wire) BETWEEN 1 AND 16777216)) STRICT";

fn regular(path: &Path) -> Result<File> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(fail("expected regular file"));
    }
    Ok(File::open(path)?)
}

pub(crate) fn visit(
    path: &Path,
    mut f: impl FnMut(Option<u64>, &[u8]) -> Result<()>,
) -> Result<()> {
    if path.is_dir() {
        return sqlite(&path.join("ledger.sqlite3"), &mut f);
    }
    let mut file = regular(path)?;
    let mut prefix = [0; 16];
    let n = file.read(&mut prefix)?;
    file.rewind()?;
    if n == 16 && &prefix == b"SQLite format 3\0" {
        return sqlite(path, &mut f);
    }
    stream(&mut file, &mut f)
}

fn sqlite(path: &Path, f: &mut impl FnMut(Option<u64>, &[u8]) -> Result<()>) -> Result<()> {
    let parent = path.parent().ok_or_else(|| fail("database needs parent"))?;
    let lock = regular(&parent.join("writer.lock"))?;
    lock.try_lock()
        .map_err(|e| fail(format!("writer lock: {e}")))?;
    let _file = regular(path)?;
    let mut connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.busy_timeout(std::time::Duration::ZERO)?;
    connection.set_limit(Limit::SQLITE_LIMIT_LENGTH, (MAX_WIRE + 1024) as i32)?;
    connection.execute_batch("PRAGMA query_only=ON; PRAGMA trusted_schema=OFF;")?;
    let tx = connection.transaction()?;
    let app: i64 = tx.query_row("PRAGMA application_id", [], |r| r.get(0))?;
    let version: i64 = tx.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let objects = tx
        .prepare("SELECT name,type,sql FROM sqlite_schema ORDER BY name LIMIT 2")?
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if app != 0x4c4f4753
        || version != 1
        || objects != [("events".into(), "table".into(), SCHEMA.into())]
    {
        return Err(fail("schema/application/version mismatch"));
    }
    let mut statement = tx.prepare("SELECT seq,wire FROM events ORDER BY seq")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let seq: i64 = row.get(0)?;
        let seq = u64::try_from(seq).map_err(|_| fail("negative SQL seq"))?;
        let wire: Vec<u8> = row.get(1)?;
        f(Some(seq), &wire)?;
    }
    Ok(())
}

fn stream(file: &mut File, f: &mut impl FnMut(Option<u64>, &[u8]) -> Result<()>) -> Result<()> {
    let mut magic = vec![0; MAGIC.len()];
    file.read_exact(&mut magic)?;
    if magic != MAGIC {
        return Err(fail("unknown input format"));
    }
    loop {
        let mut size = [0; 4];
        // EOF só é permitido antes do primeiro byte de um frame.
        let read = loop {
            match file.read(&mut size[..1]) {
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                other => break other?,
            }
        };
        if read == 0 {
            return Ok(());
        }
        file.read_exact(&mut size[1..])?;
        let size = u32::from_be_bytes(size) as usize;
        if size == 0 || size > MAX_WIRE {
            return Err(fail("invalid frame length"));
        }
        let mut wire = vec![0; size];
        file.read_exact(&mut wire)?;
        f(None, &wire)?;
    }
}

pub(crate) fn cas<D: Domain>(root: &Path, hash: Hash<D>) -> Result<Vec<u8>> {
    let name = tags::RESERVED
        .iter()
        .find(|(_, tag, _)| *tag == D::TAG)
        .ok_or_else(|| fail("unknown CAS domain"))?
        .0;
    cas_directory(root)?;
    let domain = root.join(format!("{name}-v1"));
    cas_directory(&domain)?;
    let hex = hash.to_hex();
    let shard = domain.join(&hex[..2]);
    cas_directory(&shard)?;
    let mut file = regular(&shard.join(&hex[2..]))?;
    let limit = if matches!(name, "certificate" | "checker") {
        268_435_456
    } else {
        MAX_WIRE as u64
    };
    if file.metadata()?.len() > limit {
        return Err(fail(format!("CAS {name} exceeds limit")));
    }
    let mut bytes = Vec::new();
    file.by_ref().take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(fail(format!("CAS {name} exceeds limit")));
    }
    if hash_raw::<D>(&bytes) != hash {
        return Err(fail(format!("CAS integrity: {name}/{}", hash.to_hex())));
    }
    Ok(bytes)
}

fn cas_directory(path: &Path) -> Result<()> {
    if !fs::symlink_metadata(path)?.file_type().is_dir() {
        return Err(fail("expected CAS directory without symlink"));
    }
    Ok(())
}
