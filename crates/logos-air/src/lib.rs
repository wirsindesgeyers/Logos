//! Sintaxe abstrata FOL-ms-eq/v1 e vocabulário versionado com grounding.
//!
//! `Term`/`Formula` são validados estruturalmente com limites configuráveis.
//! Nomes de binders não entram na identidade: variáveis são índices De Bruijn.
//! `Vocabulary` verifica declarações/ciclos; `check` verifica tipos e variáveis ligadas.
//!
//! **TCB** — sem unsafe, unwrap/expect/panic fora de testes.
//! Contratos: `spec/air.md`, `spec/vocabulary.md`, `spec/sort-check.md`; ADRs 0007–0010.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod codec;
mod error;
mod shape;
mod sort_check;
mod syntax;
mod vocabulary;
mod vocabulary_codec;

pub use error::AirError;
pub use sort_check::{SortError, SortErrorKind, SortPath, TermPath, TermPosition, check};
pub use syntax::{
    Formula, FormulaNode, Limits, LogicId, Sort, Symbol, SymbolKind, SymbolResult, Term, TermNode,
};
pub use vocabulary::{
    AttestorId, Compatibility, DomainSource, FactRef, Grounding, PolicyRef, SortDecl, SortDomain,
    SymbolDecl, Vocabulary, VocabularyError,
};
