//! Eventos, hash chain, assinatura, SQLite e CAS.
//!
//! **TCB** — regras da §4 do roadmap: sem `unsafe`, sem `unwrap`/`expect`/`panic` fora de testes.
//! A lista de crates de TCB vive em `tcb/manifest.toml`.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub mod cas;
pub(crate) mod cas_domain;
mod event;
mod event_body;
mod event_body_codec;
mod index;
mod index_project;
mod index_resolve;
mod index_store;
mod ledger;
mod ledger_cas;
mod ledger_error;
mod ledger_store;

pub use cas::{Cas, CasError, Limits};
pub use event::{Event, EventError, MAX_EVENT_BYTES};
pub use event_body::{
    Authority, Certificate, CheckRecord, Checker, CnfObject, CnfRecord, Consistency, EventBody,
    EventProposal, ModelObject, Route, TaintedText,
};
pub use index::{ClaimStatus, LedgerIndex, LedgerSnapshot};
pub use index_store::{IndexComparison, RebuildReport};
pub use ledger::{Ledger, LedgerHead};
pub use ledger_cas::{CasReference, LedgerReferences, ledger_references};
pub use ledger_error::LedgerError;
