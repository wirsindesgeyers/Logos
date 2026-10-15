//! Erros de contexto/limites, sem promoção a fragmento ou veredicto.

use logos_air::{AirError, SortError};
use logos_canon::CanonError;
use logos_core::Exhausted;

/// Recusa de entrada ou recurso; não é falsidade lógica nem fragmento General.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroundError {
    /// Identidade de V não coincide com a obrigação.
    VocabularyMismatch,
    /// Assinatura global inválida, inclusive quando não utilizada.
    InvalidSignature,
    /// Sort inexistente no vocabulário.
    UnknownSort,
    /// Pedido de axiomas para domínio sem enumeração.
    OpenDomain,
    /// Limite ou falha de reserva; nunca resultado parcial.
    Unknown(Exhausted),
    /// Codificação de entrada sem resultado canônico válido.
    Canon(CanonError),
    /// Contexto/ligação de variável inválido.
    Sort(SortError),
    /// Construção AIR viola forma/limites.
    Air(AirError),
}

impl From<CanonError> for GroundError {
    fn from(e: CanonError) -> Self {
        Self::Canon(e)
    }
}
impl From<SortError> for GroundError {
    fn from(e: SortError) -> Self {
        Self::Sort(e)
    }
}
impl From<AirError> for GroundError {
    fn from(e: AirError) -> Self {
        match e {
            AirError::DepthExceeded { .. } | AirError::NodesExceeded { .. } => {
                Self::Unknown(Exhausted::Size)
            }
            other => Self::Air(other),
        }
    }
}
impl std::fmt::Display for GroundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ground: {self:?}")
    }
}
impl std::error::Error for GroundError {}
