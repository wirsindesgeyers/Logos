//! Contexto de checagem e preparação falível das razões canônicas.

use logos_air::Vocabulary;
use logos_canon::{CanonError, Text};
use logos_core::{CheckOutcome, Reason};
use std::sync::LazyLock;

static REASONS: LazyLock<Result<[Reason; 4], CanonError>> = LazyLock::new(|| {
    Ok([
        Reason::new(Text::new("invalid obligation or model")?),
        Reason::new(Text::new("model falsifies a premise")?),
        Reason::new(Text::new("model satisfies the goal")?),
        Reason::new(Text::new("model certificate absent from CAS")?),
    ])
});

/// Checa modelos contra bytes de obrigações, sem emitir claims ou Verdicts.
///
/// O hash esperado deve vir do monitor; autoconsistência do hash não autentica
/// a origem da obrigação. Preparação de diagnósticos é falível e explícita.
#[derive(Debug)]
pub struct ModelChecker<'a> {
    pub(crate) vocabulary: &'a Vocabulary,
    pub(crate) invalid: &'static Reason,
    pub(crate) premise_false: &'static Reason,
    pub(crate) goal_true: &'static Reason,
    pub(crate) missing_model: &'static Reason,
}

impl<'a> ModelChecker<'a> {
    /// Prepara razões canônicas para execuções reutilizáveis contra `vocabulary`.
    ///
    /// # Errors
    /// Propaga qualquer erro de construção de texto canônico, sem fallback.
    pub fn new(vocabulary: &'a Vocabulary) -> Result<Self, CanonError> {
        let [invalid, premise_false, goal_true, missing_model] = match &*REASONS {
            Ok(reasons) => reasons,
            Err(error) => return Err(*error),
        };
        Ok(Self {
            vocabulary,
            invalid,
            premise_false,
            goal_true,
            missing_model,
        })
    }

    pub(crate) fn reject_invalid(&self) -> CheckOutcome {
        CheckOutcome::Rejected(self.invalid.clone())
    }
}
