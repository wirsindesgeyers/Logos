//! `clock`: o relógio local concorda com uma fonte de referência dentro de uma tolerância.

use logos_air::{SortDecl, SymbolDecl, Vocabulary};

use crate::sdk::{
    AttestError, Attester, Observation, anchored, literal, predicate, resource_version, until,
};

/// Identificador do atestador.
pub const ID: &str = "clock";
/// Procedimento (documento `docs/attesters/clock.md`).
pub const PROCEDURE: &str = "clock/skew-vs-reference/v1";
/// Tópico atestado.
pub const TOPIC: &str = "time";

/// Fonte de tempo de referência (outro relógio, NTP já consultado, relógio de hardware).
pub trait TimeSource {
    /// Instante Unix em segundos.
    ///
    /// # Errors
    /// Se a fonte não responde; não há valor padrão.
    fn now_unix(&self) -> Result<u64, AttestError>;
}

/// Relógio do sistema como referência.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl TimeSource for SystemClock {
    fn now_unix(&self) -> Result<u64, AttestError> {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .map_err(|_| AttestError::Observation("system clock before the Unix epoch".into()))
    }
}

/// Atesta `clock_synced` (ou sua negação) conforme o desvio observado.
#[derive(Debug)]
pub struct Clock<T: TimeSource> {
    reference: T,
    tolerance_secs: u64,
    ttl_secs: u64,
}

impl<T: TimeSource> Clock<T> {
    /// `tolerance_secs`: desvio máximo aceito; `ttl_secs`: validade do fato.
    pub const fn new(reference: T, tolerance_secs: u64, ttl_secs: u64) -> Self {
        Self {
            reference,
            tolerance_secs,
            ttl_secs,
        }
    }
}

/// Símbolos que este atestador ancora.
///
/// # Errors
/// Nomes inválidos.
pub fn declarations() -> Result<(Vec<SortDecl>, Vec<SymbolDecl>), AttestError> {
    Ok((
        vec![],
        vec![anchored(
            predicate("clock_synced", vec![])?,
            ID,
            PROCEDURE,
            "o relógio local está dentro da tolerância da referência",
        )?],
    ))
}

impl<T: TimeSource> Attester for Clock<T> {
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
        let reference = self.reference.now_unix()?;
        let skew = now.abs_diff(reference);
        let (sorts, decls) = declarations()?;
        let symbol = decls[0].signature.clone();
        Ok(Observation {
            vocabulary: Vocabulary::new(1, sorts, decls)?,
            formula: literal(&symbol, &[], skew <= self.tolerance_secs)?,
            version: Some(resource_version(
                "clock",
                &format!("skew={skew}s;tolerance={}s", self.tolerance_secs),
            )?),
            valid: until(now, self.ttl_secs)?,
        })
    }
}
