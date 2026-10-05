//! Erros de entrada e exaustão; nenhum deles é falsidade lógica.

use std::{error::Error, fmt};

use logos_air::SortError;
use logos_canon::CanonError;
use logos_core::Exhausted;

/// Falha da avaliação contextual, sem conversão para veredicto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalError {
    /// Exaustão de recurso, nunca resultado booleano.
    Unknown(Exhausted),
    /// Vocabulário não possui identidade canônica válida.
    Canon(CanonError),
    /// Modelo pertence a outro vocabulário.
    VocabularyMismatch,
    /// Fórmula ou corpo Defined mal tipado ou com variável livre não parametrizada.
    Sort(SortError),
    /// Relação Defined não tem exatamente um resultado para alguma entrada.
    InvalidDefinition {
        /// Índice do símbolo no vocabulário exato.
        symbol: u32,
    },
    /// Constante Defined enumerada não denota seu ordinal intrínseco.
    EnumerationMismatch {
        /// Índice do símbolo no vocabulário exato.
        symbol: u32,
    },
}

impl From<Exhausted> for EvalError {
    fn from(error: Exhausted) -> Self {
        Self::Unknown(error)
    }
}

impl From<CanonError> for EvalError {
    fn from(error: CanonError) -> Self {
        Self::Canon(error)
    }
}

impl From<SortError> for EvalError {
    fn from(error: SortError) -> Self {
        Self::Sort(error)
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(reason) => write!(f, "evaluation exhausted: {reason:?}"),
            Self::Canon(error) => write!(f, "evaluation canon: {error}"),
            Self::VocabularyMismatch => f.write_str("evaluation vocabulary mismatch"),
            Self::Sort(error) => write!(f, "evaluation {error}"),
            Self::InvalidDefinition { symbol } => write!(f, "non-functional definition: {symbol}"),
            Self::EnumerationMismatch { symbol } => {
                write!(f, "defined enumeration mismatch: {symbol}")
            }
        }
    }
}

impl Error for EvalError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Canon(error) => Some(error),
            Self::Sort(error) => Some(error),
            _ => None,
        }
    }
}
