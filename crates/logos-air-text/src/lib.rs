//! Sintaxe de superfície textual AIR (`air-text/v1`) — parser e pretty-printer reversível.
//!
//! Implementação completa da especificação normativa `spec/air-surface.md`.
//! Este componente vive **fora da TCB** do sistema LOGOS (ADR 0011 e Roadmap §1.4).
//!
//! Fornece:
//! - [`parse_formula`]: parsing de fórmulas textuais com resolução de binders e vocabulário.
//! - [`print_formula`]: pretty-printer determinístico, plano e reversível (`parse(print(f)) == f`).
//! - [`parse_document`]: parsing em duas passagens de documentos com vocabulário e claims.
//! - [`print_document`]: impressão determinística de documentos completos.
//! - [`Document`], [`Claim`], [`References`]: estruturas de representação em memória.
//! - [`SurfaceError`], [`ErrorKind`]: erros com categoria normativa e coordenadas 1-based.

#![forbid(unsafe_code)]

mod error;
mod lexer;
mod parser;
mod printer;
mod types;

pub use error::{ErrorKind, SurfaceError};
pub use parser::{parse_document, parse_formula};
pub use printer::{escape_json_string, print_document, print_formula, print_symbol, print_term};
pub use types::{Claim, Document, References};
