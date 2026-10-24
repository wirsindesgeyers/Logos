//! SDK de atestadores e atestadores de referência (6.3).
//!
//! Fora da TCB de correção: um atestador é um processo com chave própria que observa o
//! mundo, assina um fato fact/v1 e o entrega ao kernel; quem decide se o fato é citável é
//! a configuração de confiança do ledger (6.2). O agente/LLM nunca tem essas chaves.

#![forbid(unsafe_code)]

pub mod reference;
mod sdk;

pub use reference::{REFERENCES, Reference, example_vocabulary};
pub use sdk::{
    AttestError, Attestation, Attester, Observation, Submitted, anchored, atom, attest, constant,
    literal, negate, predicate, resource_version, sort, submit, until,
};
