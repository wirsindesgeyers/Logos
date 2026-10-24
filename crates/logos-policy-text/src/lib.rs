//! Sintaxe de superfície de políticas (7.1, `spec/policy.md`), fora da TCB.
//!
//! ```text
//! policy v12 {
//!   rule r1 allow: <fórmula air-text> comment "glosa em português"
//!   rule r9 deny:  <fórmula air-text>
//! }
//! ```
//!
//! O parser só produz uma [`Policy`]; quem decide se ela vale é `logos-policy` (sort-check,
//! forma de Horn, estratificação). O texto nunca é autoridade: o kernel consome os bytes
//! `policy/v1`.

#![forbid(unsafe_code)]

mod parse;
mod print;

pub use parse::{PolicyTextError, parse_policy};
pub use print::print_policy;

#[doc(hidden)]
pub use logos_policy::Policy;
