//! Tipos de erro e rejeição na geração de obrigações.
//!
//! Contrato normativo: `spec/obligation.md` §3; ADR 0012.

use core::fmt;
use logos_air::SortError;
use logos_canon::CanonError;

use crate::types::{EntryKind, LedgerError, PremiseRef};

/// Motivo de recusa determinística na geração de uma obrigação.
///
/// Conforme `spec/obligation.md` §3, erros de rejeição preservam precedência
/// estrita e diagnósticos determinísticos guiados pela menor referência ordenada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reject {
    /// A citação referenciada não foi encontrada no snapshot do ledger (`lookup == Ok(None)`).
    UnknownCitation(PremiseRef),
    /// A entrada citada foi retratada no ledger.
    Retracted(PremiseRef),
    /// A validade da entrada (fato) já expirou no instante do snapshot (6.1).
    Expired(PremiseRef),
    /// A época de aplicabilidade da entrada diverge da época corrente do snapshot.
    InvalidEpoch {
        /// Referência da premissa.
        reference: PremiseRef,
        /// Época corrente esperada pelo snapshot.
        expected: u64,
        /// Época observada na entrada do ledger.
        actual: u64,
    },
    /// A categoria da entrada não é citável como premissa.
    NotCitable {
        /// Referência da premissa.
        reference: PremiseRef,
        /// Categoria não citável encontrada.
        kind: EntryKind,
    },
    /// O vocabulário alvo não é compatível com o vocabulário de origem da premissa.
    IncompatibleVocabulary(PremiseRef),
    /// O objetivo (goal) falhou na verificação de sorts contra o vocabulário alvo.
    GoalSort(SortError),
    /// A premissa citada falhou na verificação de sorts contra o vocabulário alvo.
    PremiseSort {
        /// Referência da premissa recusada.
        reference: PremiseRef,
        /// Detalhe do erro de sort.
        error: SortError,
    },
    /// Falha de consulta ou integridade no snapshot do ledger.
    Ledger {
        /// Referência consultada.
        reference: PremiseRef,
        /// Erro retornado pelo ledger.
        error: LedgerError,
    },
    /// Falha na codificação canônica (ex.: cálculo de hash do vocabulário).
    Canon(CanonError),
}

impl fmt::Display for Reject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCitation(reference) => {
                write!(f, "unknown premise citation: {reference}")
            }
            Self::Retracted(reference) => {
                write!(f, "premise citation is retracted: {reference}")
            }
            Self::Expired(reference) => {
                write!(f, "premise citation has expired: {reference}")
            }
            Self::InvalidEpoch {
                reference,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "invalid epoch for premise {reference}: expected {expected}, actual {actual}"
                )
            }
            Self::NotCitable { reference, kind } => {
                write!(
                    f,
                    "entry kind {kind} for premise {reference} is not citable"
                )
            }
            Self::IncompatibleVocabulary(reference) => {
                write!(
                    f,
                    "target vocabulary is incompatible with premise {reference}"
                )
            }
            Self::GoalSort(error) => {
                write!(f, "goal sort-check failed: {error}")
            }
            Self::PremiseSort { reference, error } => {
                write!(f, "premise {reference} sort-check failed: {error}")
            }
            Self::Ledger { reference, error } => {
                write!(f, "ledger lookup failed for premise {reference}: {error}")
            }
            Self::Canon(error) => {
                write!(f, "canonical encoding error: {error}")
            }
        }
    }
}

impl std::error::Error for Reject {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::GoalSort(error) => Some(error),
            Self::PremiseSort { error, .. } => Some(error),
            Self::Ledger { error, .. } => Some(error),
            Self::Canon(error) => Some(error),
            Self::UnknownCitation(_)
            | Self::Retracted(_)
            | Self::Expired(_)
            | Self::InvalidEpoch { .. }
            | Self::NotCitable { .. }
            | Self::IncompatibleVocabulary(_) => None,
        }
    }
}

impl From<CanonError> for Reject {
    fn from(error: CanonError) -> Self {
        Self::Canon(error)
    }
}
