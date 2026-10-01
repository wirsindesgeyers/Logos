//! Codificação canônica (CBOR determinístico) com decoder estrito — `canon/v1`.
//!
//! **TCB** — regras da §4 do roadmap: sem `unsafe`, sem `unwrap`/`expect`/`panic` fora de testes.
//! A lista de crates de TCB vive em `tcb/manifest.toml`.
//!
//! Uma única forma de transformar qualquer objeto do LOGOS em bytes, e de recusar qualquer outra
//! (princípio P2: o que é hasheado é exatamente o que é checado). A especificação normativa é
//! `spec/encoding.md`; a implementação de referência independente é `py/logos_client/canon.py`.
//!
//! Subconjunto aceito: inteiros em `[-2^64, 2^64-1]` na menor forma, strings de bytes, strings de
//! texto (UTF-8, NFC, só code points atribuídos em Unicode 16.0.0), arrays, mapas com chaves
//! ordenadas pelos bytes codificados, `bool` e `null`. Sem floats, tags, comprimentos indefinidos
//! nem aninhamento acima de [`MAX_DEPTH`].

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod decode;
mod error;
mod text;
mod traits;
mod unicode_assigned;
mod value;

pub use decode::Decoder;
pub use error::CanonError;
pub use text::Text;
pub use traits::{Bytes, Canon, CanonDecode, from_canon, to_canon};
pub use value::{Array, Map, Value};

/// Identificador da versão da codificação; entra na obrigação (spec §4, passo 1).
/// Qualquer mudança de regra é `canon/v2`.
pub const CANON_VERSION: &str = "canon/v1";

/// Versão de Unicode fixada para a checagem de NFC e de code points atribuídos.
pub const UNICODE_VERSION: (u8, u8, u8) = unicode_assigned::UNICODE_VERSION;

/// Aninhamento máximo de arrays/mapas (um escalar tem profundidade 0).
pub const MAX_DEPTH: usize = 128;
