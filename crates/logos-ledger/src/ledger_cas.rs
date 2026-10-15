//! Referências CAS do catálogo; presença antes de assinar não prova validade lógica.

use std::collections::BTreeSet;
use std::fs::{self, File, TryLockError};
use std::path::Path;

use logos_air::Vocabulary;
use logos_crypto::{Hash, PublicKey};
use logos_obligation::Obligation;
use rusqlite::{Connection, OpenFlags};

use crate::{
    Cas, Certificate, CheckRecord, Checker, Consistency, EventBody, LedgerError, LedgerHead,
    ModelObject, ledger, ledger_store,
};

/// Endereço de objeto explicitamente referenciado pelo catálogo atual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CasReference {
    /// Vocabulário de fórmula/proposta/transição.
    Vocabulary(Hash<Vocabulary>),
    /// Obrigação criada ou checada.
    Obligation(Hash<Obligation>),
    /// Certificado cru registrado num check.
    Certificate(Hash<Certificate>),
    /// Binário/identidade de checker com separação de domínio.
    Checker(Hash<Checker>),
    /// Modelo registrado em refutação ou testemunha.
    Model(Hash<ModelObject>),
}

impl CasReference {
    /// Diretório do domínio cas/v1, nunca inferido a partir dos bytes.
    #[must_use]
    pub const fn directory(self) -> &'static str {
        match self {
            Self::Vocabulary(_) => "vocab-v1",
            Self::Obligation(_) => "obligation-v1",
            Self::Certificate(_) => "certificate-v1",
            Self::Checker(_) => "checker-v1",
            Self::Model(_) => "model-v1",
        }
    }
    /// Digest dentro do domínio explícito.
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        match self {
            Self::Vocabulary(h) => *h.as_bytes(),
            Self::Obligation(h) => *h.as_bytes(),
            Self::Certificate(h) => *h.as_bytes(),
            Self::Checker(h) => *h.as_bytes(),
            Self::Model(h) => *h.as_bytes(),
        }
    }
    pub(crate) fn require(self, cas: &Cas) -> Result<(), LedgerError> {
        let present = match self {
            Self::Vocabulary(h) => cas.verified_present(&h),
            Self::Obligation(h) => cas.verified_present(&h),
            Self::Certificate(h) => cas.verified_present(&h),
            Self::Checker(h) => cas.verified_present(&h),
            Self::Model(h) => cas.verified_present(&h),
        }?;
        if !present {
            return Err(LedgerError::MissingObject(self));
        }
        Ok(())
    }
}

impl std::fmt::Display for CasReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/", self.directory())?;
        for byte in self.digest() {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

fn check_refs(check: &CheckRecord, result: &mut BTreeSet<CasReference>) {
    result.extend([
        CasReference::Obligation(check.obligation),
        CasReference::Certificate(check.certificate),
        CasReference::Checker(check.checker),
    ]);
}

impl EventBody {
    /// Inventário completo de objetos CAS, deduplicado por domínio e digest.
    /// Referências de eventos/KeyIds/digests crus/raiz Merkle não são objetos CAS.
    #[must_use]
    pub fn cas_references(&self) -> BTreeSet<CasReference> {
        let mut result = BTreeSet::new();
        match self {
            Self::VocabPublished { vocabulary, .. }
            | Self::AssumptionDeclared { vocabulary, .. } => {
                result.insert(CasReference::Vocabulary(*vocabulary));
            }
            Self::Proposed { proposal, .. } => {
                result.insert(CasReference::Vocabulary(proposal.vocabulary));
            }
            Self::ObligationCreated { obligation, .. } => {
                result.insert(CasReference::Obligation(*obligation));
            }
            Self::Verified {
                check, consistency, ..
            } => {
                check_refs(check, &mut result);
                match consistency {
                    Consistency::Unknown { check: Some(check) } => check_refs(check, &mut result),
                    Consistency::Unknown { check: None } => {}
                    Consistency::Witness { model, check } => {
                        check_refs(check, &mut result);
                        result.insert(CasReference::Model(*model));
                    }
                }
            }
            Self::Refuted { check, model } => {
                check_refs(check, &mut result);
                result.insert(CasReference::Model(*model));
            }
            Self::UnknownRecorded { check, .. } => check_refs(check, &mut result),
            Self::EpochAdvanced {
                vocab: Some(vocab), ..
            } => {
                result.insert(CasReference::Vocabulary(*vocab));
            }
            Self::CheckerRegistered { checker, .. } | Self::CheckerRevoked { checker, .. } => {
                result.insert(CasReference::Checker(*checker));
            }
            Self::Genesis { .. }
            | Self::Retracted { .. }
            | Self::Invalidated { .. }
            | Self::EpochAdvanced { vocab: None, .. } => {}
        }
        result
    }
}

/// Raízes de um histórico conferido; não é relatório de validade de certificados.
pub struct LedgerReferences {
    /// Cabeça que identifica o histórico lido.
    pub head: Option<LedgerHead>,
    /// Referências diretas/aninhadas dos eventos, com domínio.
    pub objects: BTreeSet<CasReference>,
    _lock: File,
}

/// Confere o ledger inteiro sob lock, sem criar lock/cache nem escrever SQLite.
///
/// # Errors
/// Chave/cadeia/schema inválidos, lock ausente/ocupado ou falha de leitura.
pub fn ledger_references(
    directory: &Path,
    key: &PublicKey,
) -> Result<LedgerReferences, LedgerError> {
    let path = fs::canonicalize(directory)?;
    let lock_path = path.join("writer.lock");
    if !fs::symlink_metadata(&lock_path)?.is_file() {
        return Err(LedgerError::Invalid("writer lock is not a regular file"));
    }
    let lock = File::open(lock_path)?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => return Err(LedgerError::WriterBusy),
        Err(TryLockError::Error(error)) => return Err(error.into()),
    }
    let mut source = Connection::open_with_flags(
        path.join("ledger.sqlite3"),
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NOFOLLOW
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    source.execute_batch("PRAGMA trusted_schema=OFF; PRAGMA query_only=ON")?;
    source.set_limit(
        rusqlite::limits::Limit::SQLITE_LIMIT_LENGTH,
        (crate::MAX_EVENT_BYTES + 1024) as i32,
    )?;
    source.busy_timeout(std::time::Duration::ZERO)?;
    let transaction = source.transaction()?;
    ledger_store::schema(&transaction)?;
    let mut objects = BTreeSet::new();
    let head = ledger::scan(&transaction, key, |event, _| {
        objects.extend(event.body.cas_references());
        Ok(())
    })?;
    transaction.commit()?;
    Ok(LedgerReferences {
        head,
        objects,
        _lock: lock,
    })
}
