//! Tipos de erro retornados durante operações com modelos finitos.

use core::fmt;
use logos_canon::CanonError;
use logos_core::Exhausted;

/// Erros de modelo finito; nunca um veredicto lógico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    /// Erro de codificação ou decodificação canônica (CBOR determinístico).
    Canon(CanonError),
    /// Hash do vocabulário no modelo diverge do hash canônico do vocabulário fornecido.
    VocabularyMismatch,
    /// Assinatura de símbolo no vocabulário é inválida para sua categoria ou referencia sort não declarado.
    InvalidSignature,
    /// Vetor de domínios tem tamanho divergente do número de sorts ou contém cardinalidade zero.
    InvalidDomains,
    /// Interpretação de símbolo ausente, extra, duplicada, de categoria errada ou definida indevidamente.
    InvalidInterpretation,
    /// Sort enumerado tem cardinalidade divergente das constantes declaradas ou constante primitiva não denota seu ordinal.
    InvalidEnumeration,
    /// Função não total: número de saídas diverge do produto das cardinalidades dos argumentos.
    NonTotalFunction,
    /// Valor de constante, saída de função ou coordenada de tupla de predicado fora do limite do domínio do sort.
    ValueOutOfDomain,
    /// Tupla de predicado com aridade incorreta ou conjunto de tuplas não ordenado estritamente.
    InvalidTuple,
    /// Esgotamento de recurso computacional (tamanho ou memória).
    Unknown(Exhausted),
}

impl From<CanonError> for ModelError {
    fn from(err: CanonError) -> Self {
        Self::Canon(err)
    }
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canon(err) => write!(f, "canonical encoding error: {err}"),
            Self::VocabularyMismatch => f.write_str("model vocabulary hash mismatch"),
            Self::InvalidSignature => f.write_str("invalid symbol signature in vocabulary"),
            Self::InvalidDomains => f.write_str("invalid model domain cardinalities"),
            Self::InvalidInterpretation => f.write_str("invalid model symbol interpretation"),
            Self::InvalidEnumeration => f.write_str("invalid enumerated sort interpretation"),
            Self::NonTotalFunction => f.write_str("non-total function interpretation"),
            Self::ValueOutOfDomain => f.write_str("model value out of domain bounds"),
            Self::InvalidTuple => f.write_str("invalid predicate tuple"),
            Self::Unknown(exhausted) => write!(f, "model resource exhausted: {exhausted:?}"),
        }
    }
}

impl std::error::Error for ModelError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Canon(err) => Some(err),
            _ => None,
        }
    }
}
