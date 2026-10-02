//! Veredictos formais e funções de construção a partir de checagens.

use crate::outcome::CheckOutcome;

/// Veredicto formal emitido a partir de uma checagem.
///
/// Enum público e decodificável, não credencial de checker ativo ou autorização.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Verdict {
    /// A obrigação foi verificada por prova formal válida.
    Verified,
    /// A obrigação foi refutada por contramodelo válido.
    Refuted,
    /// O veredicto não pôde ser determinado (recurso esgotado ou inconclusivo).
    Unknown,
}

/// Constrói um veredicto [`Verdict::Verified`] a partir de checagem em contexto de prova formal.
///
/// Aceita estritamente [`CheckOutcome::Accepted`].
///
/// # Erros
/// Retorna o [`CheckOutcome`] original sem alteração caso seja [`CheckOutcome::Rejected`]
/// ou [`CheckOutcome::Unknown`].
pub fn verified_from_proof(outcome: CheckOutcome) -> Result<Verdict, CheckOutcome> {
    match outcome {
        CheckOutcome::Accepted => Ok(Verdict::Verified),
        other @ (CheckOutcome::Rejected(_) | CheckOutcome::Unknown(_)) => Err(other),
    }
}

/// Constrói um veredicto [`Verdict::Refuted`] a partir de checagem em contexto de contramodelo.
///
/// Aceita estritamente [`CheckOutcome::Accepted`].
///
/// # Erros
/// Retorna o [`CheckOutcome`] original sem alteração caso seja [`CheckOutcome::Rejected`]
/// ou [`CheckOutcome::Unknown`].
pub fn refuted_from_countermodel(outcome: CheckOutcome) -> Result<Verdict, CheckOutcome> {
    match outcome {
        CheckOutcome::Accepted => Ok(Verdict::Refuted),
        other @ (CheckOutcome::Rejected(_) | CheckOutcome::Unknown(_)) => Err(other),
    }
}
