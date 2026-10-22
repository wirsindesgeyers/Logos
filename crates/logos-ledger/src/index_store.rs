//! Cache reconstruível: schema versionado, comparação integral e backup atômico.

use std::fs::{self, File, OpenOptions};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use rusqlite::{Connection, OpenFlags};

use crate::{LedgerError, LedgerHead};

const APPLICATION_ID: i64 = 0x4c494458;
static NEXT: AtomicU64 = AtomicU64::new(0);
pub(crate) const TABLES: &[(&str, &str)] = &[
    (
        "claims",
        "CREATE TABLE claims (id BLOB PRIMARY KEY CHECK(length(id)=32), h_o BLOB CHECK(h_o IS NULL OR length(h_o)=32), status INTEGER NOT NULL CHECK(status BETWEEN 0 AND 6), epoch BLOB NOT NULL CHECK(length(epoch)=8), retracted INTEGER NOT NULL CHECK(retracted IN (0,1)), vocab BLOB CHECK(vocab IS NULL OR length(vocab)=32), formula BLOB, scope TEXT, checker BLOB CHECK(checker IS NULL OR length(checker)=32)) STRICT, WITHOUT ROWID",
    ),
    (
        "deps",
        "CREATE TABLE deps (claim BLOB NOT NULL CHECK(length(claim)=32), premise BLOB NOT NULL CHECK(length(premise)=32), PRIMARY KEY(claim,premise)) STRICT, WITHOUT ROWID",
    ),
    (
        "citable",
        "CREATE TABLE citable (ref BLOB NOT NULL CHECK(length(ref)=32), epoch BLOB NOT NULL CHECK(length(epoch)=8), PRIMARY KEY(ref,epoch)) STRICT, WITHOUT ROWID",
    ),
    (
        "revoked",
        "CREATE TABLE revoked (checker BLOB PRIMARY KEY CHECK(length(checker)=32)) STRICT, WITHOUT ROWID",
    ),
    (
        "metadata",
        "CREATE TABLE metadata (singleton INTEGER PRIMARY KEY CHECK(singleton=1), seq INTEGER, epoch BLOB, hash BLOB, CHECK((seq IS NULL AND epoch IS NULL AND hash IS NULL) OR (seq>=0 AND length(epoch)=8 AND length(hash)=32))) STRICT",
    ),
];
const INDEXES: &[(&str, &str)] = &[
    (
        "deps_premise",
        "CREATE INDEX deps_premise ON deps(premise,claim)",
    ),
    (
        "claims_checker",
        "CREATE INDEX claims_checker ON claims(checker)",
    ),
    // ledger-index/v2 (5.6): consultas por obrigação (S4) e pelo nó ATMS h(O) (5.9).
    ("claims_h_o", "CREATE INDEX claims_h_o ON claims(h_o)"),
];
const QUERIES: &[&str] = &[
    "SELECT * FROM claims ORDER BY id",
    "SELECT * FROM deps ORDER BY claim,premise",
    "SELECT * FROM citable ORDER BY ref,epoch",
    "SELECT * FROM revoked ORDER BY checker",
    "SELECT * FROM metadata ORDER BY singleton",
];

/// Comparação do cache anterior com a projeção reconstruída, nunca autoridade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexComparison {
    /// Não havia cache no caminho.
    Missing,
    /// Todas as colunas/linhas ordenadas coincidem.
    Equal,
    /// Cache legível mas diferente do histórico conferido.
    Different,
    /// Cache SQLite/schema corrompido ou de versão desconhecida.
    Invalid,
}

/// Resultado explícito de reconstrução e publicação do índice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RebuildReport {
    /// Cabeça da cadeia que originou a projeção.
    pub head: Option<LedgerHead>,
    /// Quantidade de eventos conferidos (incluindo eventos sem claim).
    pub events: u64,
    /// Quantidade de linhas da tabela claims.
    pub claims: u64,
    /// Comparação integral com o cache anterior.
    pub previous: IndexComparison,
}

pub(crate) fn create(connection: &Connection) -> Result<(), LedgerError> {
    connection.execute_batch("PRAGMA trusted_schema=OFF; PRAGMA synchronous=FULL;")?;
    connection.pragma_update(None, "application_id", APPLICATION_ID)?;
    connection.pragma_update(None, "user_version", 2)?;
    for (_, sql) in TABLES.iter().chain(INDEXES) {
        connection.execute_batch(sql)?;
    }
    Ok(())
}

fn valid_schema(connection: &Connection) -> Result<bool, LedgerError> {
    let app: i64 = connection.pragma_query_value(None, "application_id", |r| r.get(0))?;
    let version: i64 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if app != APPLICATION_ID || version != 2 {
        return Ok(false);
    }
    let mut statement =
        connection.prepare("SELECT name, sql FROM sqlite_schema ORDER BY name LIMIT 9")?;
    let mut rows = statement.query([])?;
    let mut expected: Vec<_> = TABLES.iter().chain(INDEXES).copied().collect();
    expected.sort_unstable_by_key(|entry| entry.0);
    for (name, sql) in expected {
        let Some(row) = rows.next()? else {
            return Ok(false);
        };
        if row.get::<_, String>(0)? != name || row.get::<_, String>(1)? != sql {
            return Ok(false);
        }
    }
    Ok(rows.next()?.is_none())
}

fn same_rows(left: &Connection, right: &Connection) -> Result<bool, LedgerError> {
    for query in QUERIES {
        let mut a = left.prepare(query)?;
        let mut b = right.prepare(query)?;
        let columns = a.column_count();
        if columns != b.column_count() {
            return Ok(false);
        }
        let mut aa = a.query([])?;
        let mut bb = b.query([])?;
        loop {
            match (aa.next()?, bb.next()?) {
                (None, None) => break,
                (Some(a), Some(b)) => {
                    for column in 0..columns {
                        if a.get_ref(column)? != b.get_ref(column)? {
                            return Ok(false);
                        }
                    }
                }
                _ => return Ok(false),
            }
        }
    }
    Ok(true)
}

fn compare(path: &Path, memory: &Connection) -> Result<IndexComparison, LedgerError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(IndexComparison::Missing);
        }
        Err(error) => return Err(error.into()),
        Ok(metadata) if !metadata.is_file() => {
            return Err(LedgerError::Invalid("index cache is not a regular file"));
        }
        Ok(_) => {}
    }
    let result = (|| {
        let previous = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_NOFOLLOW
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        previous.set_limit(
            rusqlite::limits::Limit::SQLITE_LIMIT_LENGTH,
            (crate::MAX_EVENT_BYTES + 1024) as i32,
        )?;
        previous.execute_batch("PRAGMA trusted_schema=OFF; PRAGMA query_only=ON")?;
        if !valid_schema(&previous)? {
            return Ok(IndexComparison::Invalid);
        }
        Ok(if same_rows(memory, &previous)? {
            IndexComparison::Equal
        } else {
            IndexComparison::Different
        })
    })();
    match result {
        Err(LedgerError::Sql(rusqlite::Error::SqliteFailure(code, _)))
            if matches!(
                code.code,
                rusqlite::ErrorCode::NotADatabase | rusqlite::ErrorCode::DatabaseCorrupt
            ) =>
        {
            Ok(IndexComparison::Invalid)
        }
        other => other,
    }
}

pub(crate) fn publish(
    path: &Path,
    memory: &Connection,
    head: Option<LedgerHead>,
) -> Result<RebuildReport, LedgerError> {
    let previous = compare(&path.join("index.sqlite3"), memory)?;
    let claims: i64 = memory.query_row("SELECT count(*) FROM claims", [], |r| r.get(0))?;
    let claims = u64::try_from(claims).map_err(|_| LedgerError::Invalid("negative claim count"))?;
    let temporary = loop {
        let file = path.join(format!(
            ".index.{}.{}.sqlite3",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match OpenOptions::new().write(true).create_new(true).open(&file) {
            Ok(_) => break file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    };
    let result = (|| {
        memory.backup("main", &temporary, None)?;
        File::open(&temporary)?.sync_all()?;
        fs::rename(&temporary, path.join("index.sqlite3"))?;
        File::open(path)?.sync_all()?;
        Ok(RebuildReport {
            head,
            events: head.map_or(0, |h| h.seq + 1),
            claims,
            previous,
        })
    })();
    if let Err(primary) = result {
        return Err(match fs::remove_file(&temporary) {
            Ok(()) => primary,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => primary,
            Err(cleanup) => LedgerError::Io(std::io::Error::other(format!(
                "{primary}; index cleanup: {cleanup}"
            ))),
        });
    }
    result
}
