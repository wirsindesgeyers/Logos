//! Schema versionado e configuração da conexão privada do appender.

use rusqlite::{Connection, Transaction, limits::Limit};

use crate::{LedgerError, MAX_EVENT_BYTES};

const APPLICATION_ID: i64 = 0x4c4f4753;
// Comparação exata também recusa triggers/views capazes de mudar o append.
const SCHEMA: &str = "CREATE TABLE events (seq INTEGER PRIMARY KEY CHECK (seq >= 0), wire BLOB NOT NULL CHECK (length(wire) BETWEEN 1 AND 16777216)) STRICT";

pub(crate) fn configure(connection: &Connection) -> Result<(), LedgerError> {
    connection.busy_timeout(std::time::Duration::ZERO)?;
    // Limita também o decoder SQLite antes de materializar BLOBs em Rust.
    connection.set_limit(Limit::SQLITE_LIMIT_LENGTH, (MAX_EVENT_BYTES + 1024) as i32)?;
    connection.execute_batch(
        "PRAGMA trusted_schema=OFF; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA fullfsync=ON; PRAGMA checkpoint_fullfsync=ON;",
    )?;
    let mode: String = connection.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
    let sync: i64 = connection.query_row("PRAGMA synchronous", [], |row| row.get(0))?;
    let full: i64 = connection.query_row("PRAGMA fullfsync", [], |row| row.get(0))?;
    let checkpoint: i64 =
        connection.query_row("PRAGMA checkpoint_fullfsync", [], |row| row.get(0))?;
    let trusted: i64 = connection.query_row("PRAGMA trusted_schema", [], |row| row.get(0))?;
    if mode != "wal" || sync != 2 || full != 1 || checkpoint != 1 || trusted != 0 {
        return Err(LedgerError::Invalid("durability configuration"));
    }
    Ok(())
}

pub(crate) fn schema(transaction: &Transaction<'_>) -> Result<(), LedgerError> {
    let app: i64 = transaction.query_row("PRAGMA application_id", [], |row| row.get(0))?;
    let version: i64 = transaction.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let mut statement =
        transaction.prepare("SELECT name, type, sql FROM sqlite_schema ORDER BY name LIMIT 2")?;
    let objects = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if app == 0 && version == 0 && objects.is_empty() {
        transaction.execute_batch(SCHEMA)?;
        transaction.pragma_update(None, "application_id", APPLICATION_ID)?;
        transaction.pragma_update(None, "user_version", 1)?;
    } else if app != APPLICATION_ID
        || version != 1
        || objects != [("events".into(), "table".into(), SCHEMA.into())]
    {
        return Err(LedgerError::Invalid("schema/application/version"));
    }
    Ok(())
}
