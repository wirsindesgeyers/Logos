//! Selo de cinco campos e renderizador.
//!
//! Etapa 9.1: paráfrase determinística AIR ↔ português controlado
//! (`spec/paraphrase-pt.md`, ADR0050). `render_pt` só devolve texto que
//! `parse_pt` lê de volta como a mesma fórmula; com ambos deterministas,
//! fórmulas diferentes não compartilham paráfrase. O `Seal` chega na 15.1.
//!
//! **TCB** — regras da §4 do roadmap: sem `unsafe`, sem `unwrap`/`expect`/`panic` fora de testes.
//! A lista de crates de TCB vive em `tcb/manifest.toml`.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod error;
mod grammar;
mod paraphrase;
mod parse;
mod render;

pub use error::ParaphraseError;
pub use grammar::{GRAMMAR_ID, RENDERER_VERSION};
pub use paraphrase::{Paraphrase, ParaphraseLimits, Paraphraser, parse_pt, render_pt};
