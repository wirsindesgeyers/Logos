//! Máquina de estados, protocolo e registry de checkers.
//!
//! **TCB** — regras da §4 do roadmap: sem `unsafe`, sem `unwrap`/`expect`/`panic` fora de testes.
//! A lista de crates de TCB vive em `tcb/manifest.toml`.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(target_os = "linux")]
pub mod propose;
#[cfg(target_os = "linux")]
pub mod server;
#[cfg(target_os = "linux")]
pub mod staging;
pub mod state;
mod transitions;
mod transport_error;
pub mod wire;

pub use transitions::{Claim, SignedEvent};
pub use transport_error::TransportError;
