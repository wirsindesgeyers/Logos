//! Checker próprio do perfil textual lrat-rup/v1 (spec/lrat.md).
//!
//! **TCB** — regras da §4 do roadmap: sem `unsafe`, sem `unwrap`/`expect`/`panic` fora de testes.
//! A lista de crates de TCB vive em `tcb/manifest.toml`.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod cake;
mod cake_known;
mod cake_output;
mod checker;
mod parse;
mod resources;
mod sandbox;

pub use cake::{CakeBinary, CakeError, CakeRegistry, LratCertificate};
pub use cake_output::decode_cake_output;

use logos_canon::{CanonError, Text};
use logos_core::{CheckOutcome, Reason};

/// Checker RUP com diagnósticos preparados falivelmente; não emite veredictos.
#[derive(Debug)]
pub struct LratChecker {
    invalid: Reason,
}

impl LratChecker {
    /// Prepara a razão canônica de rejeição, sem fallback silencioso.
    ///
    /// # Errors
    /// Propaga a falha de validação canônica do diagnóstico.
    pub fn new() -> Result<Self, CanonError> {
        Ok(Self {
            invalid: Reason::new(Text::new("invalid CNF or RUP certificate")?),
        })
    }
}

/// Resultado bruto e IDs das cláusulas originais alcançáveis da primeira vazia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LratReport {
    outcome: CheckOutcome,
    used: Option<Vec<u64>>,
}

impl LratReport {
    /// Resultado sem conversão para Verified/Refuted.
    #[must_use]
    pub fn outcome(&self) -> &CheckOutcome {
        &self.outcome
    }

    /// IDs 1-based crescentes, presentes somente quando a prova inteira foi aceita.
    #[must_use]
    pub fn used_original_clauses(&self) -> Option<&[u64]> {
        self.used.as_deref()
    }
}
