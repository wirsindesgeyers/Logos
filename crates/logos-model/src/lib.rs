//! Modelos finitos, avaliador e checkers de obrigação (camada 0).
//!
//! **TCB** — regras da §4 do roadmap: sem `unsafe`, sem `unwrap`/`expect`/`panic` fora de testes.
//! A lista de crates de TCB vive em `tcb/manifest.toml`.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(missing_docs)]

mod checker;
mod checker_context;
mod codec;
mod data;
mod error;
mod eval;
mod eval_error;
mod evaluate;
mod exact_size;
mod limits;
mod model;
mod validate;

pub use checker_context::ModelChecker;
pub use data::ModelData;
pub use error::ModelError;
pub use eval::{eval, eval_on_meter};
pub use eval_error::EvalError;
pub use limits::ModelLimits;
pub use model::FiniteModel;
