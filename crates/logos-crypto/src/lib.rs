//! Hash com separação de domínio, assinaturas com algoritmo identificado e `KeyId`.
//!
//! **TCB** — regras da §4 do roadmap: sem `unsafe`, sem `unwrap`/`expect`/`panic` fora de testes.
//! A lista de crates de TCB vive em `tcb/manifest.toml`.
//!
//! * [`Hash<T>`] / [`hash`] / [`hash_raw`]: `SHA-256(tag_T ‖ bytes)`, com o domínio no tipo.
//! * [`ExternalDigest`]: SHA-256 sem tag, só para conferir binários de terceiros.
//! * [`SigAlg`], [`PublicKey`], [`Signature`], [`KeyId`]: tudo com o algoritmo identificado
//!   (Ed25519 na v1; ES256 reservado para a Etapa 9.2).
//! * [`Signer`] / [`FileSigner`] / [`verify`]: assinaturas também têm domínio ([`SigDomain`]),
//!   então uma assinatura de evento nunca vale como token.
//!
//! Especificação normativa: `spec/domains.md`. Decisões: ADR 0004.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod domain;
mod error;
mod sig;
mod signer;

pub use domain::{Domain, ExternalDigest, Hash, RawHasher, hash, hash_raw, tags};
pub use error::CryptoError;
pub use sig::{KeyId, PublicKey, SigAlg, SigDomain, Signature, sig_tags, verify};
pub use signer::{FileSigner, Signer};
