//! Sintaxe abstrata FOL-ms-eq/v1: sorts, assinaturas e árvores prefixas planas.
//!
//! `Term`/`Formula` são validados estruturalmente com limites configuráveis.
//! Nomes de binders não entram na identidade: variáveis são índices De Bruijn.
//! Vocabulário e sort-check pertencem às etapas seguintes; sintaxe não é credencial.
//!
//! **TCB** — sem unsafe, unwrap/expect/panic fora de testes.
//! Formato normativo: `spec/air.md`; decisão: ADR 0007.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod codec;
mod error;
mod shape;
mod syntax;

pub use error::AirError;
pub use syntax::{
    Formula, FormulaNode, Limits, LogicId, Sort, Symbol, SymbolKind, SymbolResult, Term, TermNode,
};
