//! Classificação, candidatos de axiomas e grounding finito (Etapas4.1/4.2).
//!
//! Candidatos preservam fonte opaca e não são premissas autenticadas/claims.
//! Grounding condicional usa axiomas explícitos; Tseitin e used ficam nas próximas etapas.
//!
//! **TCB** — regras da §4 do roadmap: sem `unsafe`, sem `unwrap`/`expect`/`panic` fora de testes.
//! A lista de crates de TCB vive em `tcb/manifest.toml`.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(missing_docs)]

mod axioms;
mod classify;
mod dag;
mod error;
mod ground;
mod prop;

pub use axioms::{AxiomCandidates, AxiomLimits, domain_axioms, domain_axioms_with_limits};
pub use classify::{AnalysisLimits, Fragment, classify, classify_with_limits};
pub use error::GroundError;
pub use ground::{GroundLimits, ground, ground_with_limits};
pub use prop::{Atom, GroundProgram, PropFormula, PropNode};
