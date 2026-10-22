//! Erros de paráfrase; nunca um veredicto lógico nem uma paráfrase "parecida".

use std::{error::Error, fmt};

use logos_air::{AirError, SortError};
use logos_canon::CanonError;

/// Recusa de renderização, de parsing ou de vocabulário para paráfrase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParaphraseError {
    /// Vocabulário vocabulary/v1, sem templates `render_pt` (ADR0046).
    MissingTemplates,
    /// Dois símbolos do mesmo contexto (predicado ou termo) com o mesmo texto literal.
    AmbiguousTemplates,
    /// Template nulário de termo com a forma reservada de variável (`x1`, `x2`…).
    VariableNameCollision,
    /// Nome de sort contém `⟦` ou `⟧`, que delimitam a estrutura.
    UnrenderableSortName,
    /// Fórmula não fechada ou mal tipada para o vocabulário.
    Sort(SortError),
    /// Estrutura/limites AIR inválidos.
    Air(AirError),
    /// Texto não canônico (NFC/Unicode) ao montar a paráfrase.
    Canon(CanonError),
    /// Fórmula ou texto acima da profundidade limite.
    DepthExceeded,
    /// Texto acima do limite de bytes.
    TooLarge,
    /// Texto fora da gramática paraphrase-pt/v1, na posição em bytes indicada.
    Syntax {
        /// Offset em bytes do primeiro ponto recusado.
        offset: usize,
    },
    /// Átomo ou termo sem template correspondente no vocabulário.
    UnknownTemplate {
        /// Offset em bytes do início do conteúdo recusado.
        offset: usize,
    },
    /// Sort de quantificador não declarado no vocabulário.
    UnknownSort {
        /// Offset em bytes do nome recusado.
        offset: usize,
    },
    /// Nome de variável sem binder ativo correspondente.
    UnboundVariable {
        /// Offset em bytes do nome recusado.
        offset: usize,
    },
    /// Releitura da paráfrase produzida não devolveu a mesma fórmula.
    RoundTripMismatch,
}

impl From<SortError> for ParaphraseError {
    fn from(error: SortError) -> Self {
        Self::Sort(error)
    }
}

impl From<AirError> for ParaphraseError {
    fn from(error: AirError) -> Self {
        Self::Air(error)
    }
}

impl From<CanonError> for ParaphraseError {
    fn from(error: CanonError) -> Self {
        Self::Canon(error)
    }
}

impl fmt::Display for ParaphraseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTemplates => f.write_str("vocabulary has no render_pt templates"),
            Self::AmbiguousTemplates => f.write_str("two symbols share the same template text"),
            Self::VariableNameCollision => {
                f.write_str("nullary term template collides with variable names")
            }
            Self::UnrenderableSortName => f.write_str("sort name contains reserved brackets"),
            Self::Sort(error) => write!(f, "paraphrase sort-check: {error}"),
            Self::Air(error) => write!(f, "paraphrase formula: {error}"),
            Self::Canon(error) => write!(f, "paraphrase text: {error}"),
            Self::DepthExceeded => f.write_str("paraphrase depth exceeded"),
            Self::TooLarge => f.write_str("paraphrase text too large"),
            Self::Syntax { offset } => write!(f, "paraphrase syntax error at byte {offset}"),
            Self::UnknownTemplate { offset } => write!(f, "unknown template at byte {offset}"),
            Self::UnknownSort { offset } => write!(f, "unknown sort at byte {offset}"),
            Self::UnboundVariable { offset } => write!(f, "unbound variable at byte {offset}"),
            Self::RoundTripMismatch => f.write_str("paraphrase does not parse back to formula"),
        }
    }
}

impl Error for ParaphraseError {}
