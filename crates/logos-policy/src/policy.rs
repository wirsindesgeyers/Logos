//! `policy/v1`: regras AIR classificadas, versionadas e ligadas a um vocabulário.
//!
//! ```text
//! Policy = [1, version_u64, hV, rules]
//! rules  = [Rule…]   em ordem estrita de `id` (bytes)
//! Rule   = [id_Text, class_u64, Formula, comment_Text]
//! class  = 0 allow | 1 deny | 2 require | 3 derive
//! ```
//!
//! Identidade: `h(Policy)` com a tag `logos/policy/v1\0` sobre os bytes canônicos. O
//! comentário é glosa em português e **não** é normativo. Semântica deny-overrides:
//! `Allowed(a)` só é derivável se alguma regra `allow` dispara e nenhuma `deny` aplicável
//! dispara.

use logos_air::{Formula, Limits, Vocabulary};
use logos_canon::{Array, Canon, CanonDecode, CanonError, Decoder, Text, Value};
use logos_crypto::{Domain, Hash, hash, tags};

use crate::{PolicyError, shape, stratify};

/// Nome do predicado de autorização que as regras `allow`/`deny` definem.
pub const ALLOWED: &str = "Allowed";
/// Teto de regras por política.
pub const MAX_RULES: usize = 4096;
/// Teto do `id` de uma regra.
pub const MAX_RULE_ID_BYTES: usize = 256;
/// Teto do comentário de uma regra.
pub const MAX_COMMENT_BYTES: usize = 64 * 1024;
/// Teto dos bytes canônicos de uma política.
pub const MAX_POLICY_BYTES: usize = 4 * 1024 * 1024;

/// Classe de uma regra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    /// Horn: `corpo → Allowed(t)`.
    Allow,
    /// Horn: `corpo → ¬Allowed(t)`; prevalece sobre `allow` (deny-overrides).
    Deny,
    /// FOL geral sobre `Allowed`; exige certificado (rota L2).
    Require,
    /// Horn: `corpo → P(t)` com `P ≠ Allowed` (predicado auxiliar).
    Derive,
}

impl Class {
    pub(crate) const fn tag(self) -> u64 {
        match self {
            Self::Allow => 0,
            Self::Deny => 1,
            Self::Require => 2,
            Self::Derive => 3,
        }
    }

    pub(crate) fn from_tag(tag: u64) -> Result<Self, CanonError> {
        match tag {
            0 => Ok(Self::Allow),
            1 => Ok(Self::Deny),
            2 => Ok(Self::Require),
            3 => Ok(Self::Derive),
            _ => Err(CanonError::WrongType),
        }
    }

    /// Nome de superfície (`allow`, `deny`, `require`, `derive`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::Require => "require",
            Self::Derive => "derive",
        }
    }
}

/// Uma regra: `id` único, classe, fórmula AIR e comentário (glosa, não normativo).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// Identificador único dentro da política.
    pub id: Text,
    /// Classe.
    pub class: Class,
    /// Fórmula AIR da regra.
    pub formula: Formula,
    /// Comentário em português.
    pub comment: Text,
}

/// Política validada: nenhuma instância existe sem passar por [`Policy::new`] ou decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    version: u64,
    vocabulary: Hash<Vocabulary>,
    rules: Vec<Rule>,
}

impl Domain for Policy {
    const TAG: &'static [u8] = tags::POLICY;
}

fn declares_allowed(vocabulary: &Vocabulary) -> bool {
    vocabulary.symbols().iter().any(|d| {
        d.signature.name.as_str() == ALLOWED
            && d.signature.kind == logos_air::SymbolKind::Pred
            && d.signature.arity.len() == 1
    })
}

fn validate(version: u64, vocabulary: &Vocabulary, rules: &[Rule]) -> Result<(), PolicyError> {
    if version == 0 {
        return Err(PolicyError::ZeroVersion);
    }
    if rules.len() > MAX_RULES {
        return Err(PolicyError::TooManyRules);
    }
    let mut previous: Option<&str> = None;
    for rule in rules {
        let id = rule.id.as_str();
        if id.is_empty() || id.len() > MAX_RULE_ID_BYTES {
            return Err(PolicyError::BadRuleId);
        }
        match previous {
            Some(p) if p == id => return Err(PolicyError::DuplicateRule(id.to_owned())),
            Some(p) if p > id => return Err(PolicyError::UnsortedRules),
            _ => {}
        }
        previous = Some(id);
        if rule.comment.as_str().len() > MAX_COMMENT_BYTES {
            return Err(PolicyError::CommentTooLong(id.to_owned()));
        }
        logos_air::check(vocabulary, &rule.formula)
            .map_err(|e| PolicyError::Sort(id.to_owned(), e))?;
    }
    if !rules.is_empty() && !declares_allowed(vocabulary) {
        return Err(PolicyError::MissingAllowed);
    }
    let mut horn = Vec::new();
    for rule in rules {
        let tree = shape::Tree::new(&rule.formula);
        if rule.class == Class::Require {
            if !tree.mentions_allowed() {
                return Err(PolicyError::RequireWithoutAllowed(
                    rule.id.as_str().to_owned(),
                ));
            }
            continue;
        }
        horn.push((
            rule.class,
            shape::analyse(rule.id.as_str(), rule.class, &tree)?,
        ));
    }
    stratify::check(&horn)
}

impl Policy {
    /// Constrói a política: ordena por `id`, confere o sort-check de cada regra no vocabulário,
    /// a forma das classes e a estratificação.
    ///
    /// # Errors
    /// Qualquer violação de [`PolicyError`]; nada é aceito parcialmente.
    pub fn new(
        version: u64,
        vocabulary: &Vocabulary,
        mut rules: Vec<Rule>,
    ) -> Result<Self, PolicyError> {
        rules.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        validate(version, vocabulary, &rules)?;
        Ok(Self {
            version,
            vocabulary: hash(vocabulary)?,
            rules,
        })
    }

    /// Decodifica bytes `policy/v1` canônicos e revalida tudo contra `vocabulary`.
    ///
    /// # Errors
    /// Bytes não canônicos, `h(V)` diferente do vocabulário ou qualquer violação de regra.
    pub fn decode(bytes: &[u8], vocabulary: &Vocabulary) -> Result<Self, PolicyError> {
        let policy: Self = logos_canon::from_canon(bytes)?;
        if policy.vocabulary != hash(vocabulary)? {
            return Err(PolicyError::VocabularyMismatch);
        }
        validate(policy.version, vocabulary, &policy.rules)?;
        Ok(policy)
    }

    /// Versão da política.
    #[must_use]
    pub const fn version(&self) -> u64 {
        self.version
    }

    /// `h(V)` do vocabulário das regras.
    #[must_use]
    pub const fn vocabulary(&self) -> Hash<Vocabulary> {
        self.vocabulary
    }

    /// Regras em ordem estrita de `id`.
    #[must_use]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Regra de identificador `id`.
    #[must_use]
    pub fn rule(&self, id: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.id.as_str() == id)
    }

    /// Identidade `h(Policy)`.
    ///
    /// # Errors
    /// Falha de codificação canônica.
    pub fn hash(&self) -> Result<Hash<Self>, CanonError> {
        hash(self)
    }
}

fn array(items: Vec<Value>) -> Result<Value, CanonError> {
    Array::new(items).map(Value::Array)
}

fn items(value: &Value) -> Result<&[Value], CanonError> {
    match value {
        Value::Array(array) => Ok(array.items()),
        _ => Err(CanonError::WrongType),
    }
}

impl Canon for Rule {
    fn to_value(&self) -> Result<Value, CanonError> {
        array(vec![
            self.id.to_value()?,
            Value::Unsigned(self.class.tag()),
            self.formula.to_value()?,
            self.comment.to_value()?,
        ])
    }
}

impl Canon for Policy {
    fn to_value(&self) -> Result<Value, CanonError> {
        let mut rules = Vec::with_capacity(self.rules.len());
        for rule in &self.rules {
            rules.push(rule.to_value()?);
        }
        array(vec![
            Value::Unsigned(1),
            Value::Unsigned(self.version),
            self.vocabulary.to_value()?,
            array(rules)?,
        ])
    }
}

impl CanonDecode for Policy {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let [Value::Unsigned(1), version, vocabulary, rules] = items(value)? else {
            return Err(CanonError::WrongType);
        };
        let rules = items(rules)?;
        if rules.len() > MAX_RULES {
            return Err(CanonError::WrongType);
        }
        let rules = rules
            .iter()
            .map(|rule| {
                let [id, class, formula, comment] = items(rule)? else {
                    return Err(CanonError::WrongType);
                };
                Ok(Rule {
                    id: Text::from_value(id)?,
                    class: Class::from_tag(u64::from_value(class)?)?,
                    formula: Formula::from_value_with_limits(formula, Limits::default())
                        .map_err(|_| CanonError::WrongType)?,
                    comment: Text::from_value(comment)?,
                })
            })
            .collect::<Result<Vec<_>, CanonError>>()?;
        Ok(Self {
            version: u64::from_value(version)?,
            vocabulary: match vocabulary {
                Value::Bytes(bytes) => Hash::from_bytes(
                    <[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| CanonError::WrongType)?,
                ),
                _ => return Err(CanonError::WrongType),
            },
            rules,
        })
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, CanonError> {
        if decoder.remaining() > MAX_POLICY_BYTES {
            return Err(CanonError::WrongType);
        }
        Self::from_value(&decoder.value()?)
    }
}
