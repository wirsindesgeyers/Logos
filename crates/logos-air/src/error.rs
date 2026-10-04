//! Erros de formato e de limites estruturais da AIR.

use logos_canon::CanonError;
use std::{error::Error, fmt};

/// Falha de leitura/construção de sintaxe, não um veredicto lógico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AirError {
    /// Violação de canon/v1 ou forma de um campo.
    Canon(CanonError),
    /// LogicId não implementado.
    UnsupportedLogic,
    /// Prefixo vazio, filhos faltantes, raízes extras ou contagem irrepresentável.
    InvalidStructure,
    /// Profundidade lógica excedeu limite configurado.
    DepthExceeded {
        /// Profundidade máxima permitida.
        limit: usize,
        /// Profundidade observada até a recusa.
        actual: usize,
    },
    /// Número de nós lógicos excedeu limite configurado.
    NodesExceeded {
        /// Número máximo de nós permitidos.
        limit: usize,
        /// Número de nós observado até a recusa.
        actual: usize,
    },
}

impl From<CanonError> for AirError {
    fn from(error: CanonError) -> Self {
        Self::Canon(error)
    }
}

impl fmt::Display for AirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canon(error) => write!(f, "AIR canon: {error}"),
            Self::UnsupportedLogic => f.write_str("unsupported AIR logic"),
            Self::InvalidStructure => f.write_str("invalid AIR prefix tree"),
            Self::DepthExceeded { limit, actual } => {
                write!(f, "AIR depth {actual} exceeds limit {limit}")
            }
            Self::NodesExceeded { limit, actual } => {
                write!(f, "AIR nodes {actual} exceeds limit {limit}")
            }
        }
    }
}

impl Error for AirError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Canon(error) => Some(error),
            _ => None,
        }
    }
}
