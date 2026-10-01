//! Erros da codificação canônica.

use core::fmt;

/// Erro de codificação, decodificação ou construção de valores canônicos.
///
/// Os nomes (`name()`) fazem parte da especificação (`spec/encoding.md`): os vetores de teste
/// e a implementação de referência em Python usam exatamente os mesmos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonError {
    /// A entrada acabou antes do fim do item.
    Eof,
    /// Sobraram bytes depois do item de nível mais alto.
    TrailingBytes,
    /// O argumento do cabeçalho não usa a menor codificação possível.
    NonMinimalHead,
    /// Informação adicional reservada (28–30), ou 31 em tipo que não admite indefinido.
    ReservedInfo,
    /// Comprimento indefinido ou `break`.
    Indefinite,
    /// Tag CBOR (tipo maior 6).
    Tag,
    /// Valor simples fora de `false`, `true` e `null` (inclui `undefined` e floats).
    UnsupportedSimple,
    /// String de texto que não é UTF-8 válido.
    InvalidUtf8,
    /// String de texto com code point não atribuído na versão de Unicode fixada.
    UnassignedCodePoint,
    /// String de texto que não está em NFC.
    NotNfc,
    /// Chaves de mapa fora da ordem lexicográfica dos bytes codificados.
    MapKeyOrder,
    /// Chave de mapa repetida.
    DuplicateKey,
    /// Aninhamento de arrays/mapas maior que [`MAX_DEPTH`](crate::MAX_DEPTH).
    DepthExceeded,
    /// Inteiro fora de `[-2^64, 2^64-1]` (ou fora do tipo Rust de destino).
    IntOutOfRange,
    /// O valor decodificado não tem a forma esperada pelo tipo de destino.
    WrongType,
}

impl CanonError {
    /// Nome estável do erro, igual ao usado em `spec/encoding.md` e nos vetores de teste.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Eof => "Eof",
            Self::TrailingBytes => "TrailingBytes",
            Self::NonMinimalHead => "NonMinimalHead",
            Self::ReservedInfo => "ReservedInfo",
            Self::Indefinite => "Indefinite",
            Self::Tag => "Tag",
            Self::UnsupportedSimple => "UnsupportedSimple",
            Self::InvalidUtf8 => "InvalidUtf8",
            Self::UnassignedCodePoint => "UnassignedCodePoint",
            Self::NotNfc => "NotNfc",
            Self::MapKeyOrder => "MapKeyOrder",
            Self::DuplicateKey => "DuplicateKey",
            Self::DepthExceeded => "DepthExceeded",
            Self::IntOutOfRange => "IntOutOfRange",
            Self::WrongType => "WrongType",
        }
    }
}

impl fmt::Display for CanonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl std::error::Error for CanonError {}
