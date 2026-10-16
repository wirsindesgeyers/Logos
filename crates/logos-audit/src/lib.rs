//! Verificador independente do ledger (não depende de logos-kernel nem de logos-ledger).
//!
//! Fora da TCB de correção.

#![forbid(unsafe_code)]

mod event;
mod source;
mod verify;

use std::fmt;
use std::path::Path;

use logos_crypto::PublicKey;

pub use verify::{export, verify};

/// Falha de leitura, autenticação, formato ou recheck, nunca veredicto lógico.
#[derive(Debug)]
pub struct AuditError(pub(crate) String);

impl fmt::Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for AuditError {}

pub(crate) type Result<T> = std::result::Result<T, AuditError>;

pub(crate) fn fail(message: impl Into<String>) -> AuditError {
    AuditError(message.into())
}

impl From<std::io::Error> for AuditError {
    fn from(error: std::io::Error) -> Self {
        fail(format!("I/O: {error}"))
    }
}

impl From<rusqlite::Error> for AuditError {
    fn from(error: rusqlite::Error) -> Self {
        fail(format!("SQLite: {error}"))
    }
}

impl From<logos_canon::CanonError> for AuditError {
    fn from(error: logos_canon::CanonError) -> Self {
        fail(format!("canon: {error}"))
    }
}

/// Contagens explícitas: autenticar evento não verifica sua prova lógica.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Eventos com estrutura, cadeia e assinatura conferidas.
    pub events: u64,
    /// Refutações aceitas novamente pelo checker de modelos.
    pub refutations: u64,
    /// Testemunhas explícitas de consistência aceitas novamente.
    pub consistencies: u64,
    /// Provas Verified sem checker de prova nesta etapa.
    pub unchecked_proofs: u64,
    /// Derivações O→CNF reemitidas e conferidas (não provas LRAT).
    pub cnf_derivations: u64,
    /// Digest integral do último evento; None no vazio.
    pub head: Option<[u8; 32]>,
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let head = self.head.map(hex).unwrap_or_else(|| "none".into());
        write!(
            f,
            "events={} refutations_rechecked={} consistencies_rechecked={} proofs_unchecked={} failures=0 head={head} cnf_rederived={}",
            self.events,
            self.refutations,
            self.consistencies,
            self.unchecked_proofs,
            self.cnf_derivations
        )
    }
}

pub(crate) fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// Confere somente envelope/cadeia, sem disponibilidade ou lógica do CAS.
///
/// # Errors
/// Recusa fonte, estrutura, assinatura ou histórico inválidos.
pub fn authenticate(path: &Path, key: &PublicKey) -> std::result::Result<Report, AuditError> {
    verify::walk(path, key, |_, _| Ok(()))
}
