//! Máquina de estados, protocolo, registry de checkers e retratação (5.9).
//!
//! **TCB** — regras da §4 do roadmap: sem `unsafe`, sem `unwrap`/`expect`/`panic` fora de testes.
//! A lista de crates de TCB vive em `tcb/manifest.toml`.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(target_os = "linux")]
mod assume;
#[cfg(target_os = "linux")]
pub mod certify;
#[cfg(target_os = "linux")]
pub mod propose;
#[cfg(target_os = "linux")]
pub mod recheck;
#[cfg(target_os = "linux")]
pub mod registry;
#[cfg(target_os = "linux")]
mod replay;
#[cfg(target_os = "linux")]
pub mod retract;
#[cfg(target_os = "linux")]
mod scope;
#[cfg(target_os = "linux")]
pub mod server;
#[cfg(target_os = "linux")]
mod speculation;
#[cfg(target_os = "linux")]
pub mod staging;
pub mod state;
mod transitions;
mod transport_error;
pub mod wire;
#[cfg(target_os = "linux")]
pub mod witness;

pub use transitions::{Claim, SignedEvent};
pub use transport_error::TransportError;
