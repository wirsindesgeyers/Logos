//! `backup-svc` (mock): registra backups com timestamp e atesta se há um recente.

use logos_air::{SortDecl, SortDomain, SymbolDecl, Vocabulary};
use logos_canon::Text;

use crate::sdk::{
    AttestError, Attester, Observation, anchored, constant, literal, predicate, resource_version,
    sort, until,
};

/// Identificador do atestador.
pub const ID: &str = "backup-svc";
/// Procedimento (documento `docs/attesters/backup-svc.md`).
pub const PROCEDURE: &str = "backup-svc/registry/v1";
/// Tópico atestado.
pub const TOPIC: &str = "backup";

/// Serviço de backup simulado para o banco `db`.
#[derive(Debug)]
pub struct BackupSvc {
    db: String,
    max_age_secs: u64,
    ttl_secs: u64,
    taken: Vec<u64>,
}

impl BackupSvc {
    /// `max_age_secs`: idade máxima do último backup para o fato valer.
    pub fn new(db: impl Into<String>, max_age_secs: u64, ttl_secs: u64) -> Self {
        Self {
            db: db.into(),
            max_age_secs,
            ttl_secs,
            taken: Vec::new(),
        }
    }

    /// Registra um backup concluído em `timestamp` (segundos Unix).
    pub fn record(&mut self, timestamp: u64) {
        self.taken.push(timestamp);
    }
}

/// Símbolos para o banco `db`.
///
/// # Errors
/// Nomes inválidos.
pub fn declarations(db: &str) -> Result<(Vec<SortDecl>, Vec<SymbolDecl>), AttestError> {
    let database = sort("Database")?;
    Ok((
        vec![SortDecl {
            name: Text::new("Database")?,
            domain: SortDomain::Open,
        }],
        vec![
            anchored(
                constant(db, &database)?,
                ID,
                PROCEDURE,
                "banco coberto pelo serviço de backup",
            )?,
            anchored(
                predicate("backup_recent", vec![database])?,
                ID,
                PROCEDURE,
                "existe um backup do banco mais novo que a idade máxima",
            )?,
        ],
    ))
}

impl Attester for BackupSvc {
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
        let (sorts, decls) = declarations(&self.db)?;
        let db = decls[0].signature.clone();
        let recent = decls[1].signature.clone();
        // Backup "do futuro" não conta: o serviço mente ou o relógio está errado.
        let last = self.taken.iter().copied().filter(|t| *t <= now).max();
        let fresh = last.is_some_and(|t| now - t <= self.max_age_secs);
        Ok(Observation {
            vocabulary: Vocabulary::new(1, sorts, decls)?,
            formula: literal(&recent, &[&db], fresh)?,
            version: last
                .map(|t| resource_version(&self.db, &format!("backup_at={t}")))
                .transpose()?,
            valid: until(now, self.ttl_secs)?,
        })
    }
}
