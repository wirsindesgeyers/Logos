//! Sintaxe abstrata FOL-ms-eq/v1 e vocabulário versionado com grounding.
//!
//! `Term`/`Formula` são validados estruturalmente com limites configuráveis.
//! Nomes de binders não entram na identidade: variáveis são índices De Bruijn.
//! `Vocabulary` verifica declarações/ciclos; sort-check ainda não implementado.
//!
//! **TCB** — sem unsafe, unwrap/expect/panic fora de testes.
//! Formatos normativos: `spec/air.md`, `spec/vocabulary.md`; ADRs 0007 e 0009.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod codec;
mod error;
mod shape;
mod syntax;
mod vocabulary;
mod vocabulary_codec;

pub use error::AirError;
pub use syntax::{
    Formula, FormulaNode, Limits, LogicId, Sort, Symbol, SymbolKind, SymbolResult, Term, TermNode,
};
pub use vocabulary::{
    AttestorId, Compatibility, DomainSource, FactRef, Grounding, PolicyRef, SortDecl, SortDomain,
    SymbolDecl, Vocabulary, VocabularyError,
};
