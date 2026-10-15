//! Intervalo genérico de épocas, sem autenticação ou formato serializado.

use core::fmt;

/// Limites invertidos de um intervalo inclusivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpochError {
    /// Primeiro limite recebido.
    pub first: u64,
    /// Último limite recebido, menor que first.
    pub last: u64,
}

impl fmt::Display for EpochError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "inverted epoch range: {}..={}", self.first, self.last)
    }
}
impl std::error::Error for EpochError {}

/// Janela inclusiva u64. Não confere citabilidade, proveniência ou autoridade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpochRange {
    first: u64,
    last: u64,
}

impl EpochRange {
    /// Constrói intervalo inclusivo, sem aritmética que possa transbordar.
    ///
    /// # Errors
    /// Retorna EpochError quando first > last.
    pub const fn new(first: u64, last: u64) -> Result<Self, EpochError> {
        if first > last {
            return Err(EpochError { first, last });
        }
        Ok(Self { first, last })
    }

    /// Primeiro limite, incluído na janela.
    #[must_use]
    pub const fn first(self) -> u64 {
        self.first
    }

    /// Último limite, incluído na janela.
    #[must_use]
    pub const fn last(self) -> u64 {
        self.last
    }

    /// Indica pertinência à janela; não autentica ou promove uma entrada.
    #[must_use]
    pub const fn contains(self, epoch: u64) -> bool {
        self.first <= epoch && epoch <= self.last
    }
}
