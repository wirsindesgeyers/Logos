//! Geração selada e leitura contextual de obrigações canônicas.
//!
//! **TCB** — regras da §4 do roadmap: sem `unsafe`, sem `unwrap`/`expect`/`panic` fora de testes.
//! A lista de crates de TCB vive em `tcb/manifest.toml`.
//!
//! Especificação normativa: `spec/obligation.md`. Decisões: ADRs 0012/0016.
//!
//! O gerador pertence à fronteira de confiança (TCB de correção) do LOGOS. Ele transforma
//! uma [`Proposal`] e referências validadas por um [`LedgerView`] em um sequente canônico
//! selado [`Obligation`], no formato canônico `obligation/v1`.
//! [`decode_for_check`] lê bytes/hash para [`CheckedObligation`] não autoritativo,
//! sem abrir a construção de [`Obligation`] ou validar proveniência de eventos.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(missing_docs)]

mod codec;
mod decode;
mod error;
mod generate;
mod types;

pub use decode::{CheckedObligation, DecodeError, DecodeLimits, decode_for_check};
pub use error::Reject;
pub use generate::generate;
pub use types::{
    EntryKind, LedgerEntry, LedgerError, LedgerView, Obligation, PremiseRef, Proposal,
};
