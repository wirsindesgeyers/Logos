//! Mensagem candidata `proposal/v1`, fora da TCB (ADR 0013).
//!
//! JSON estrito não concede autoridade. Objetivo usa o decoder AIR existente;
//! texto fica tainted. Não há decoder de obrigação, ledger ou especulação.

#![forbid(unsafe_code)]

mod codec;
mod error;
mod types;

pub use codec::{decode, encode};
pub use error::ProtocolError;
pub use types::{ClaimId, Proposal, UntrustedText};

/// Versão inicial do único formato de mensagem de agente definido aqui.
pub const PROPOSAL_VERSION: u32 = 1;
/// Máximo inclusivo de bytes do documento JSON completo.
pub const MAX_MESSAGE_BYTES: usize = 3 * 1024 * 1024;
/// Máximo inclusivo de bytes UTF-8 do texto após decodificar escapes JSON.
pub const MAX_TEXT_BYTES: usize = 64 * 1024;
/// Máximo inclusivo de bytes CBOR do objetivo após decodificar hex.
pub const MAX_GOAL_BYTES: usize = 1024 * 1024;
/// Máximo inclusivo de elementos em cada lista de referências.
pub const MAX_REFERENCES: usize = 1024;
