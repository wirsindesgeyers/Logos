//! Classificação conservadora e candidatos lógicos de axiomas (Etapa4.1).
//!
//! Candidatos preservam fonte opaca e não são premissas autenticadas/claims.
//! Grounder, Tseitin e extração de dependências pertencem às próximas etapas.
//!
//! **TCB** — regras da §4 do roadmap: sem `unsafe`, sem `unwrap`/`expect`/`panic` fora de testes.
//! A lista de crates de TCB vive em `tcb/manifest.toml`.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(missing_docs)]

mod axioms;
mod classify;
mod error;

pub use axioms::{AxiomCandidates, AxiomLimits, domain_axioms, domain_axioms_with_limits};
pub use classify::{AnalysisLimits, Fragment, classify, classify_with_limits};
pub use error::GroundError;
