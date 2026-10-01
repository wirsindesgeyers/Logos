//! Erros de criptografia.

use core::fmt;
use logos_canon::CanonError;

use crate::sig::SigAlg;

/// Erro de hash, assinatura, verificação ou carregamento de chave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoError {
    /// Nome de algoritmo que não existe.
    UnknownAlgorithm,
    /// Algoritmo reservado mas ainda não implementado (ex.: ES256, Etapa 9.2).
    UnsupportedAlgorithm(SigAlg),
    /// Tamanho de chave pública inválido para o algoritmo.
    BadKeyLength,
    /// Chave pública inválida (ponto fora da curva ou de ordem pequena).
    InvalidPublicKey,
    /// Tamanho de assinatura inválido para o algoritmo.
    BadSignatureLength,
    /// A assinatura e a chave declaram algoritmos diferentes.
    AlgorithmMismatch,
    /// A assinatura não verifica para esta chave, domínio e mensagem.
    BadSignature,
    /// Arquivo de chave com permissões mais abertas que `0600` (modo octal informado).
    InsecurePermissions(u32),
    /// O caminho da chave não é um arquivo regular (ex.: link simbólico, diretório).
    KeyFileNotRegular,
    /// Conteúdo do arquivo de chave fora do formato (64 hex minúsculos e `\n` opcional).
    KeyFileFormat,
    /// Falha de E/S ao ler o arquivo de chave.
    Io(std::io::ErrorKind),
    /// Falha ao codificar/decodificar um objeto canônico.
    Canon(CanonError),
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAlgorithm => f.write_str("UnknownAlgorithm"),
            Self::UnsupportedAlgorithm(a) => write!(f, "UnsupportedAlgorithm({})", a.name()),
            Self::BadKeyLength => f.write_str("BadKeyLength"),
            Self::InvalidPublicKey => f.write_str("InvalidPublicKey"),
            Self::BadSignatureLength => f.write_str("BadSignatureLength"),
            Self::AlgorithmMismatch => f.write_str("AlgorithmMismatch"),
            Self::BadSignature => f.write_str("BadSignature"),
            Self::InsecurePermissions(m) => write!(f, "InsecurePermissions({m:o})"),
            Self::KeyFileNotRegular => f.write_str("KeyFileNotRegular"),
            Self::KeyFileFormat => f.write_str("KeyFileFormat"),
            Self::Io(k) => write!(f, "Io({k})"),
            Self::Canon(e) => write!(f, "Canon({e})"),
        }
    }
}

impl std::error::Error for CryptoError {}

impl From<CanonError> for CryptoError {
    fn from(e: CanonError) -> Self {
        Self::Canon(e)
    }
}
