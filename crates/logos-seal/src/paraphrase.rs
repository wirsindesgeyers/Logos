//! Tabelas de templates por vocabulário e registro hasheável da paráfrase.

use std::collections::BTreeMap;

use logos_air::{Formula, Limits, Sort, SymbolKind, Vocabulary};
use logos_canon::{Array, Canon, CanonError, Text, Value};
use logos_crypto::{Domain, Hash, hash, tags};

use crate::{
    ParaphraseError,
    grammar::{self, CLOSE_CHAR, GRAMMAR_ID, OPEN_CHAR, RENDERER_VERSION},
    parse, render,
};

/// Limites da paráfrase; não fazem parte do hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParaphraseLimits {
    /// Profundidade e nós aceitos, iguais aos limites AIR padrão.
    pub air: Limits,
    /// Bytes UTF-8 máximos do texto renderizado ou lido.
    pub max_bytes: usize,
}

impl Default for ParaphraseLimits {
    fn default() -> Self {
        Self {
            air: Limits::default(),
            max_bytes: 1 << 20,
        }
    }
}

/// Template de um símbolo, separado em segmentos literais.
#[derive(Debug)]
pub(crate) struct Template<'v> {
    pub(crate) segments: Vec<&'v str>,
}

/// Renderizador/parser paraphrase-pt/v1 para um vocabulário vocabulary/v2.
///
/// A construção recusa vocabulários cujo texto não identifica univocamente o
/// símbolo: a injetividade não depende do conteúdo dos templates.
#[derive(Debug)]
pub struct Paraphraser<'v> {
    pub(crate) vocabulary: &'v Vocabulary,
    pub(crate) limits: ParaphraseLimits,
    pub(crate) by_name: BTreeMap<&'v str, Template<'v>>,
    pub(crate) predicates: BTreeMap<Vec<&'v str>, usize>,
    pub(crate) terms: BTreeMap<Vec<&'v str>, usize>,
    pub(crate) sorts: BTreeMap<&'v str, Sort>,
}

impl<'v> Paraphraser<'v> {
    /// Prepara tabelas com os limites padrão.
    ///
    /// # Errors
    /// `MissingTemplates`, `AmbiguousTemplates`, `VariableNameCollision` ou
    /// `UnrenderableSortName`.
    pub fn new(vocabulary: &'v Vocabulary) -> Result<Self, ParaphraseError> {
        Self::with_limits(vocabulary, ParaphraseLimits::default())
    }

    /// Prepara tabelas com limites explícitos.
    ///
    /// # Errors
    /// Os de [`Paraphraser::new`].
    pub fn with_limits(
        vocabulary: &'v Vocabulary,
        limits: ParaphraseLimits,
    ) -> Result<Self, ParaphraseError> {
        let templates = vocabulary
            .render_pt()
            .ok_or(ParaphraseError::MissingTemplates)?;
        let mut by_name = BTreeMap::new();
        let mut predicates = BTreeMap::new();
        let mut terms = BTreeMap::new();
        for (index, (decl, template)) in vocabulary.symbols().iter().zip(templates).enumerate() {
            let segments = grammar::segments(template.as_str());
            if segments.len() == 1
                && decl.signature.kind != SymbolKind::Pred
                && grammar::variable_ordinal(segments[0]).is_some()
            {
                return Err(ParaphraseError::VariableNameCollision);
            }
            let table = if decl.signature.kind == SymbolKind::Pred {
                &mut predicates
            } else {
                &mut terms
            };
            if table.insert(segments.clone(), index).is_some() {
                return Err(ParaphraseError::AmbiguousTemplates);
            }
            by_name.insert(decl.signature.name.as_str(), Template { segments });
        }
        let mut sorts = BTreeMap::new();
        for decl in vocabulary.sorts() {
            let name = decl.name.as_str();
            if name.contains(OPEN_CHAR) || name.contains(CLOSE_CHAR) {
                return Err(ParaphraseError::UnrenderableSortName);
            }
            sorts.insert(
                name,
                Sort {
                    name: decl.name.clone(),
                },
            );
        }
        Ok(Self {
            vocabulary,
            limits,
            by_name,
            predicates,
            terms,
            sorts,
        })
    }

    /// Renderiza `formula`; só devolve texto que `parse` lê de volta como `formula`.
    ///
    /// # Errors
    /// `Sort` se a fórmula não for fechada/bem tipada, `DepthExceeded`,
    /// `TooLarge`, `Canon` ou `RoundTripMismatch`; nunca um texto aproximado.
    pub fn render(&self, formula: &Formula) -> Result<Paraphrase, ParaphraseError> {
        let text = render::render(self, formula)?;
        if self.parse(&text)? != *formula {
            return Err(ParaphraseError::RoundTripMismatch);
        }
        Ok(Paraphrase {
            text: Text::new(text)?,
            vocabulary: hash(self.vocabulary)?,
        })
    }

    /// Lê texto paraphrase-pt/v1 e devolve a fórmula fechada e bem tipada.
    ///
    /// # Errors
    /// `TooLarge`, `DepthExceeded`, `Syntax`, `UnknownTemplate`, `UnknownSort`,
    /// `UnboundVariable`, `Air` ou `Sort`.
    pub fn parse(&self, text: &str) -> Result<Formula, ParaphraseError> {
        parse::parse(self, text)
    }
}

/// Paraphrase produzida pelo renderizador, ligada ao vocabulário que a interpreta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paraphrase {
    text: Text,
    vocabulary: Hash<Vocabulary>,
}

impl Paraphrase {
    /// Texto exibido ao usuário, byte a byte.
    #[must_use]
    pub fn text(&self) -> &str {
        self.text.as_str()
    }

    /// `h(V)` do vocabulário que interpreta o texto.
    #[must_use]
    pub const fn vocabulary(&self) -> &Hash<Vocabulary> {
        &self.vocabulary
    }

    /// `h(P) = SHA256("logos/paraphrase/v1\0" || canon(P))`.
    ///
    /// # Errors
    /// Os de [`Canon::to_value`].
    pub fn digest(&self) -> Result<Hash<Self>, CanonError> {
        hash(self)
    }
}

impl Domain for Paraphrase {
    const TAG: &'static [u8] = tags::PARAPHRASE;
}

/// `paraphrase/v1 = [1, GRAMMAR_ID, RENDERER_VERSION, h(V), text]`.
impl Canon for Paraphrase {
    fn to_value(&self) -> Result<Value, CanonError> {
        Ok(Value::Array(Array::new(vec![
            Value::Unsigned(1),
            Value::Text(Text::new(GRAMMAR_ID)?),
            Value::Text(Text::new(RENDERER_VERSION)?),
            Value::Bytes(self.vocabulary.as_bytes().to_vec()),
            Value::Text(self.text.clone()),
        ])?))
    }
}

/// Renderiza com limites padrão (`render_pt(φ, V)` do roadmap).
///
/// # Errors
/// Os de [`Paraphraser::new`] e [`Paraphraser::render`].
pub fn render_pt(
    formula: &Formula,
    vocabulary: &Vocabulary,
) -> Result<Paraphrase, ParaphraseError> {
    Paraphraser::new(vocabulary)?.render(formula)
}

/// Lê com limites padrão (`parse_pt(String, V)` do roadmap).
///
/// # Errors
/// Os de [`Paraphraser::new`] e [`Paraphraser::parse`].
pub fn parse_pt(text: &str, vocabulary: &Vocabulary) -> Result<Formula, ParaphraseError> {
    Paraphraser::new(vocabulary)?.parse(text)
}
