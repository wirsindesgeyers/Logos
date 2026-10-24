//! Erros de política: sempre explícitos, nunca uma política "parecida".

use std::fmt;

use logos_air::SortError;
use logos_canon::CanonError;

/// Recusa de uma política ou de uma regra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    /// Bytes fora de `policy/v1`.
    Canon(CanonError),
    /// Versão 0.
    ZeroVersion,
    /// Mais regras que o teto.
    TooManyRules,
    /// `h(V)` da política não é o do vocabulário fornecido.
    VocabularyMismatch,
    /// Identificador de regra vazio ou acima do teto.
    BadRuleId,
    /// Dois regras com o mesmo `id`.
    DuplicateRule(String),
    /// Regras fora da ordem estrita por `id` (só no decode).
    UnsortedRules,
    /// Comentário acima do teto.
    CommentTooLong(String),
    /// A fórmula da regra não passa no sort-check do vocabulário.
    Sort(String, SortError),
    /// O vocabulário não declara `Allowed` como predicado unário.
    MissingAllowed,
    /// Regra `allow`/`deny`/`derive` fora da forma de Horn restrita.
    NotHorn(String, &'static str),
    /// Cabeça da regra incompatível com a classe.
    BadHead(String, &'static str),
    /// Regra `require` que não menciona `Allowed`.
    RequireWithoutAllowed(String),
    /// Ciclo com negação entre predicados (política não estratificável).
    NegativeCycle(String),
    /// Fórmula acima dos limites estruturais desta análise.
    TooLarge,
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canon(e) => write!(f, "malformed policy: {e:?}"),
            Self::ZeroVersion => f.write_str("policy version must be positive"),
            Self::TooManyRules => f.write_str("too many rules"),
            Self::VocabularyMismatch => f.write_str("policy vocabulary hash mismatch"),
            Self::BadRuleId => f.write_str("rule id must be non-empty and bounded"),
            Self::DuplicateRule(id) => write!(f, "duplicate rule id {id}"),
            Self::UnsortedRules => f.write_str("rules must be sorted by id"),
            Self::CommentTooLong(id) => write!(f, "rule {id}: comment too long"),
            Self::Sort(id, e) => write!(f, "rule {id}: {e:?}"),
            Self::MissingAllowed => f.write_str("vocabulary must declare pred Allowed(1)"),
            Self::NotHorn(id, why) => {
                write!(f, "rule {id}: not in the restricted Horn form: {why}")
            }
            Self::BadHead(id, why) => write!(f, "rule {id}: bad head: {why}"),
            Self::RequireWithoutAllowed(id) => {
                write!(f, "rule {id}: a require rule must mention Allowed")
            }
            Self::NegativeCycle(p) => write!(f, "negation cycle through predicate {p}"),
            Self::TooLarge => f.write_str("rule formula too large to analyse"),
        }
    }
}

impl std::error::Error for PolicyError {}

impl From<CanonError> for PolicyError {
    fn from(e: CanonError) -> Self {
        Self::Canon(e)
    }
}
