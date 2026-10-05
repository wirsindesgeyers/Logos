//! Falhas de admissão; nenhuma delas é refutação ou resultado lógico.

use core::fmt;
use logos_air::AirError;
use logos_canon::CanonError;

/// Erro de encode/decode da mensagem candidata.
#[derive(Debug)]
pub enum ProtocolError {
    /// JSON inválido, campo extra/duplicado/ausente, tipo ou referência inválidos.
    Json(serde_json::Error),
    /// Versão conhecida sintaticamente, mas não suportada.
    UnsupportedVersion(u32),
    /// Objetivo não é hex ASCII minúsculo, par e não vazio.
    GoalHex,
    /// Objetivo inválido segundo air/v1/canon/v1.
    Air(AirError),
    /// Objetivo em memória não possui codificação canônica válida.
    Canon(CanonError),
    /// Limite inclusivo excedido; nunca truncamento.
    Limit {
        /// Campo ou envelope excedido.
        field: &'static str,
        /// Máximo permitido.
        max: usize,
    },
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "invalid proposal JSON: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported proposal version {version}")
            }
            Self::GoalHex => f.write_str("goal must contain nonempty lowercase even-length hex"),
            Self::Air(error) => write!(f, "invalid goal AIR: {error}"),
            Self::Canon(error) => write!(f, "goal cannot be encoded: {error}"),
            Self::Limit { field, max } => write!(f, "{field} exceeds limit {max}"),
        }
    }
}

impl std::error::Error for ProtocolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Air(error) => Some(error),
            Self::Canon(error) => Some(error),
            _ => None,
        }
    }
}
