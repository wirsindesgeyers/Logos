//! Gerador determinístico de obrigações canônicas.
//!
//! **TCB** — regras da §4 do roadmap: sem `unsafe`, sem `unwrap`/`expect`/`panic` fora de testes.
//! A lista de crates de TCB vive em `tcb/manifest.toml`.
//!
//! Especificação normativa: `spec/obligation.md`. Decisões: ADR 0012.
//!
//! O gerador pertence à fronteira de confiança (TCB de correção) do LOGOS. Ele transforma
//! uma [`Proposal`] e referências validadas por um [`LedgerView`] em um sequente canônico
//! selado [`Obligation`], no formato canônico `obligation/v1`.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(missing_docs)]

mod codec;
mod error;
mod generate;
mod types;

pub use error::Reject;
pub use generate::generate;
pub use types::{
    EntryKind, LedgerEntry, LedgerError, LedgerView, Obligation, PremiseRef, Proposal,
};
