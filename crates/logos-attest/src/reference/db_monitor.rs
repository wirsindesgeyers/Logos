//! `db-monitor`: existência de tabela, contagem de conexões e versão de linha.
//!
//! [`SqliteProbe`] consulta um arquivo SQLite real (somente leitura). [`MockPgProbe`] é o
//! mock do `pg-monitor`: a integração com Postgres fica fora desta etapa.

use std::path::{Path, PathBuf};

use logos_air::{SortDecl, SortDomain, SymbolDecl, Vocabulary};
use logos_canon::Text;
use logos_crypto::ExternalDigest;
use rusqlite::{Connection, OpenFlags, types::ValueRef};

use crate::sdk::{
    AttestError, Attester, Observation, anchored, constant, literal, predicate, resource_version,
    sort, until,
};

/// Identificador do atestador.
pub const ID: &str = "db-monitor";
/// Procedimento (documento `docs/attesters/db-monitor.md`).
pub const PROCEDURE: &str = "db-monitor/probe/v1";
/// Tópico atestado.
pub const TOPIC: &str = "db";

/// Sonda de um banco: o que o monitor sabe observar.
pub trait DbProbe {
    /// A tabela existe?
    ///
    /// # Errors
    /// Falha da sonda; nunca `false` no lugar de erro.
    fn table_exists(&self, table: &str) -> Result<bool, AttestError>;
    /// Número de conexões abertas.
    ///
    /// # Errors
    /// Falha ou sonda sem esse conceito.
    fn connections(&self) -> Result<u64, AttestError>;
    /// Impressão digital da linha `rowid` de `table` (hash dos valores), se existir.
    ///
    /// # Errors
    /// Falha da sonda.
    fn row_version(&self, table: &str, rowid: i64) -> Result<Option<String>, AttestError>;
}

fn sqlite(e: &rusqlite::Error) -> AttestError {
    AttestError::Sqlite(e.to_string())
}

fn identifier(name: &str) -> Result<&str, AttestError> {
    if !name.is_empty()
        && name.len() <= 128
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        Ok(name)
    } else {
        Err(AttestError::Invalid(
            "table name must be [A-Za-z0-9_]{1,128}",
        ))
    }
}

/// Sonda de um arquivo SQLite aberto somente para leitura.
#[derive(Debug)]
pub struct SqliteProbe {
    path: PathBuf,
}

impl SqliteProbe {
    /// Aponta para `path`; abre a cada consulta (sem conexão retida).
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_owned(),
        }
    }

    fn open(&self) -> Result<Connection, AttestError> {
        Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| sqlite(&e))
    }
}

impl DbProbe for SqliteProbe {
    fn table_exists(&self, table: &str) -> Result<bool, AttestError> {
        let count: i64 = self
            .open()?
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .map_err(|e| sqlite(&e))?;
        Ok(count > 0)
    }

    fn connections(&self) -> Result<u64, AttestError> {
        Err(AttestError::Observation(
            "SQLite has no connection count; use a server probe".into(),
        ))
    }

    fn row_version(&self, table: &str, rowid: i64) -> Result<Option<String>, AttestError> {
        let table = identifier(table)?;
        let connection = self.open()?;
        let mut statement = connection
            .prepare(&format!("SELECT * FROM \"{table}\" WHERE rowid = ?1"))
            .map_err(|e| sqlite(&e))?;
        let columns = statement.column_count();
        let mut rows = statement.query([rowid]).map_err(|e| sqlite(&e))?;
        let Some(row) = rows.next().map_err(|e| sqlite(&e))? else {
            return Ok(None);
        };
        // Cada valor entra com tag de tipo e tamanho: "1" e 1 não colidem.
        let mut bytes = Vec::new();
        for index in 0..columns {
            let value = row.get_ref(index).map_err(|e| sqlite(&e))?;
            let (tag, body): (u8, Vec<u8>) = match value {
                ValueRef::Null => (0, Vec::new()),
                ValueRef::Integer(i) => (1, i.to_be_bytes().to_vec()),
                ValueRef::Real(r) => (2, r.to_bits().to_be_bytes().to_vec()),
                ValueRef::Text(t) => (3, t.to_vec()),
                ValueRef::Blob(b) => (4, b.to_vec()),
            };
            bytes.push(tag);
            bytes.extend_from_slice(&(body.len() as u64).to_be_bytes());
            bytes.extend_from_slice(&body);
        }
        Ok(Some(ExternalDigest::of(&bytes).to_hex()))
    }
}

/// Mock do `pg-monitor`: números configurados à mão, sem rede.
#[derive(Debug, Clone, Default)]
pub struct MockPgProbe {
    /// Tabelas existentes.
    pub tables: Vec<String>,
    /// Conexões abertas.
    pub connections: u64,
    /// Linhas `(tabela, rowid, impressão)`.
    pub rows: Vec<(String, i64, String)>,
}

impl DbProbe for MockPgProbe {
    fn table_exists(&self, table: &str) -> Result<bool, AttestError> {
        Ok(self.tables.iter().any(|t| t == table))
    }
    fn connections(&self) -> Result<u64, AttestError> {
        Ok(self.connections)
    }
    fn row_version(&self, table: &str, rowid: i64) -> Result<Option<String>, AttestError> {
        Ok(self
            .rows
            .iter()
            .find(|(t, id, _)| t == table && *id == rowid)
            .map(|(_, _, v)| v.clone()))
    }
}

/// O que o monitor atesta nesta observação.
#[derive(Debug, Clone)]
pub enum Check {
    /// `table_exists(tabela)` (ou a negação).
    TableExists(String),
    /// `connections_within_limit(db)`: conexões ≤ `max`.
    ConnectionsAtMost {
        /// Constante que nomeia o banco no vocabulário.
        db: String,
        /// Limite inclusivo.
        max: u64,
    },
    /// `row_observed(tabela)`: a linha existe; a impressão vai na versão.
    RowVersion {
        /// Tabela.
        table: String,
        /// `rowid`.
        rowid: i64,
    },
}

/// Monitor de banco sobre uma sonda.
#[derive(Debug)]
pub struct DbMonitor<P: DbProbe> {
    probe: P,
    check: Check,
    ttl_secs: u64,
}

impl<P: DbProbe> DbMonitor<P> {
    /// Monitor de uma verificação.
    pub const fn new(probe: P, check: Check, ttl_secs: u64) -> Self {
        Self {
            probe,
            check,
            ttl_secs,
        }
    }
}

/// Símbolos para as constantes de tabela e banco indicadas.
///
/// # Errors
/// Nomes inválidos.
pub fn declarations(
    tables: &[&str],
    dbs: &[&str],
) -> Result<(Vec<SortDecl>, Vec<SymbolDecl>), AttestError> {
    let table = sort("Table")?;
    let db = sort("Database")?;
    let mut decls = Vec::new();
    for name in tables {
        decls.push(anchored(
            constant(name, &table)?,
            ID,
            PROCEDURE,
            "tabela vigiada pelo monitor de banco",
        )?);
    }
    for name in dbs {
        decls.push(anchored(
            constant(name, &db)?,
            ID,
            PROCEDURE,
            "banco vigiado pelo monitor",
        )?);
    }
    decls.push(anchored(
        predicate("table_exists", vec![table.clone()])?,
        ID,
        PROCEDURE,
        "a tabela existe no banco",
    )?);
    decls.push(anchored(
        predicate("row_observed", vec![table])?,
        ID,
        PROCEDURE,
        "a linha existe; sua versão é a impressão digital registrada",
    )?);
    decls.push(anchored(
        predicate("connections_within_limit", vec![db])?,
        ID,
        PROCEDURE,
        "o número de conexões abertas não passa do limite",
    )?);
    let open = |name: &str| -> Result<SortDecl, AttestError> {
        Ok(SortDecl {
            name: Text::new(name)?,
            domain: SortDomain::Open,
        })
    };
    Ok((vec![open("Table")?, open("Database")?], decls))
}

impl<P: DbProbe> Attester for DbMonitor<P> {
    fn id(&self) -> &str {
        ID
    }
    fn topic(&self) -> &str {
        TOPIC
    }
    fn procedure(&self) -> &'static str {
        PROCEDURE
    }
    fn observe(&mut self, now: u64) -> Result<Observation, AttestError> {
        let (tables, dbs): (Vec<&str>, Vec<&str>) = match &self.check {
            Check::TableExists(t) | Check::RowVersion { table: t, .. } => {
                (vec![identifier(t)?], vec![])
            }
            Check::ConnectionsAtMost { db, .. } => (vec![], vec![db.as_str()]),
        };
        let (sorts, decls) = declarations(&tables, &dbs)?;
        let find = |name: &str| {
            decls
                .iter()
                .find(|d| d.signature.name.as_str() == name)
                .map(|d| d.signature.clone())
                .ok_or(AttestError::Invalid("symbol missing from declarations"))
        };
        let (formula, version) = match &self.check {
            Check::TableExists(table) => {
                let exists = self.probe.table_exists(table)?;
                (
                    literal(&find("table_exists")?, &[&find(table)?], exists)?,
                    None,
                )
            }
            Check::ConnectionsAtMost { db, max } => {
                let connections = self.probe.connections()?;
                (
                    literal(
                        &find("connections_within_limit")?,
                        &[&find(db)?],
                        connections <= *max,
                    )?,
                    Some(resource_version(
                        db,
                        &format!("connections={connections};max={max}"),
                    )?),
                )
            }
            Check::RowVersion { table, rowid } => {
                let fingerprint = self.probe.row_version(table, *rowid)?;
                (
                    literal(
                        &find("row_observed")?,
                        &[&find(table)?],
                        fingerprint.is_some(),
                    )?,
                    fingerprint
                        .map(|f| {
                            resource_version(&format!("{table}#{rowid}"), &format!("sha256={f}"))
                        })
                        .transpose()?,
                )
            }
        };
        Ok(Observation {
            vocabulary: Vocabulary::new(1, sorts, decls)?,
            formula,
            version,
            valid: until(now, self.ttl_secs)?,
        })
    }
}
