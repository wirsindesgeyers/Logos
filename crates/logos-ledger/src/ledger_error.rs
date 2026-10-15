//! Falhas do escritor persistente; nenhuma recuperação silenciosa de histórico.

use core::fmt;
use std::io;

use crate::EventError;

/// Falha de abertura, recuperação ou append do ledger.
#[derive(Debug)]
pub enum LedgerError {
    /// Outro handle/processo já detém o lock do escritor.
    WriterBusy,
    /// Uma operação de persistência falhou; é necessário fechar e reabrir.
    Poisoned,
    /// O próximo seq não cabe no INTEGER não negativo do SQLite.
    SequenceExhausted,
    /// Estrutura, configuração ou encadeamento inválido.
    Invalid(&'static str),
    /// Falha explícita de arquivo ou sincronização.
    Io(io::Error),
    /// Falha de SQLite, inclusive commit cujo resultado possa ser ambíguo.
    Sql(rusqlite::Error),
    /// Evento inválido ou assinatura recusada.
    Event(EventError),
    /// Falha de codificação canônica.
    Canon(logos_canon::CanonError),
    /// Identidade de chave inválida.
    Crypto(logos_crypto::CryptoError),
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WriterBusy => f.write_str("ledger writer already locked"),
            Self::Poisoned => f.write_str("ledger persistence failed; reopen required"),
            Self::SequenceExhausted => f.write_str("ledger sequence exhausted"),
            Self::Invalid(reason) => write!(f, "invalid ledger: {reason}"),
            Self::Io(error) => write!(f, "ledger I/O: {error}"),
            Self::Sql(error) => write!(f, "ledger SQLite: {error}"),
            Self::Event(error) => write!(f, "ledger event: {error}"),
            Self::Canon(error) => write!(f, "ledger canon: {error}"),
            Self::Crypto(error) => write!(f, "ledger crypto: {error}"),
        }
    }
}

impl std::error::Error for LedgerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Sql(error) => Some(error),
            Self::Event(error) => Some(error),
            Self::Canon(error) => Some(error),
            Self::Crypto(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for LedgerError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<rusqlite::Error> for LedgerError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error)
    }
}
impl From<EventError> for LedgerError {
    fn from(error: EventError) -> Self {
        Self::Event(error)
    }
}
impl From<logos_canon::CanonError> for LedgerError {
    fn from(error: logos_canon::CanonError) -> Self {
        Self::Canon(error)
    }
}
impl From<logos_crypto::CryptoError> for LedgerError {
    fn from(error: logos_crypto::CryptoError) -> Self {
        Self::Crypto(error)
    }
}
