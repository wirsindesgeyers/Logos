//! Eventos, hash chain, assinatura, SQLite e CAS.
//!
//! **TCB** — regras da §4 do roadmap: sem `unsafe`, sem `unwrap`/`expect`/`panic` fora de testes.
//! A lista de crates de TCB vive em `ci/tcb-crates.txt` (e, a partir da Etapa 0.2, em `tcb/manifest.toml`).

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
