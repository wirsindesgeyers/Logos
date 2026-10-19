//! Erros explícitos do transporte; nenhum deles é veredicto lógico.

/// Erro de codificação, limite, autenticação ou armazenamento.
#[derive(Debug)]
pub enum TransportError {
    /// I/O ou socket.
    Io(std::io::Error),
    /// Canon inválido.
    Canon(logos_canon::CanonError),
    /// CAS recusou a operação.
    Cas(logos_ledger::CasError),
    /// Ledger recusou a operação.
    Ledger(logos_ledger::LedgerError),
    /// Estrutura/contexto inválidos.
    Invalid(&'static str),
    /// Limite explícito excedido.
    Limit(&'static str),
    /// UID não permitido.
    Unauthorized,
    /// Handler ainda não implementado.
    Unavailable,
}
impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "transport I/O: {e}"),
            Self::Canon(e) => write!(f, "transport canon: {e}"),
            Self::Cas(e) => write!(f, "transport CAS: {e}"),
            Self::Ledger(e) => write!(f, "transport ledger: {e}"),
            Self::Invalid(e) => write!(f, "invalid transport: {e}"),
            Self::Limit(e) => write!(f, "transport limit: {e}"),
            Self::Unauthorized => f.write_str("unauthorized peer UID"),
            Self::Unavailable => f.write_str("handler unavailable"),
        }
    }
}
impl std::error::Error for TransportError {}
impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<logos_canon::CanonError> for TransportError {
    fn from(e: logos_canon::CanonError) -> Self {
        Self::Canon(e)
    }
}
impl From<logos_ledger::CasError> for TransportError {
    fn from(e: logos_ledger::CasError) -> Self {
        Self::Cas(e)
    }
}
impl From<logos_ledger::LedgerError> for TransportError {
    fn from(e: logos_ledger::LedgerError) -> Self {
        Self::Ledger(e)
    }
}

impl TransportError {
    pub(crate) fn code(&self) -> u64 {
        match self {
            Self::Invalid(_) | Self::Canon(_) => 0,
            Self::Limit(_) => 1,
            Self::Unauthorized => 2,
            Self::Unavailable => 3,
            Self::Io(_) | Self::Cas(_) | Self::Ledger(_) => 4,
        }
    }
}
