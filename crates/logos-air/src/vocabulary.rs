//! Vocabulário imutável: declarações únicas, dependências acíclicas e extensão explícita.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    error::Error,
    fmt,
};

use logos_canon::{CanonError, Text};
use logos_crypto::{Domain, tags};

use crate::{
    AirError, Formula, FormulaNode, Sort, Symbol, SymbolKind, SymbolResult, Term, TermNode,
};

/// Referência opaca ao digest de um fato; não comprova existência nem atestação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactRef(pub [u8; 32]);

/// Referência opaca ao digest de uma política; não comprova autorização.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyRef(pub [u8; 32]);

/// Nome canônico opaco de atestador; não resolve nem executa seu procedimento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestorId(pub Text);

/// Premissa que declara o fecho de um domínio enumerado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainSource {
    /// Fato atestado, ainda não resolvido nesta etapa.
    Fact(FactRef),
    /// Política, ainda não resolvida nesta etapa.
    Policy(PolicyRef),
}

/// Domínio declarado; enumeração não demonstra por si só o fecho do mundo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortDomain {
    /// Domínio sem enumeração fechada.
    Open,
    /// Constantes distintas declaradas neste vocabulário e sua premissa de fecho.
    Enumerated {
        /// Constantes na ordem declarada, sem deduplicação silenciosa.
        constants: Vec<Symbol>,
        /// Referência obrigatória ao fato ou política de fecho.
        domain_source: DomainSource,
    },
}

/// Declaração nominal de sort, com domínio e premissa de fecho quando enumerado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortDecl {
    /// Nome canônico único no namespace de sorts.
    pub name: Text,
    /// Domínio declarado.
    pub domain: SortDomain,
}

/// Evidência declarada de significado; não é um selo de fidelidade verificada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Grounding {
    /// Corpo definidor em outros símbolos; não inclui a própria cabeça/axioma.
    Defined(Formula),
    /// Atestador e descrição canônica de seu procedimento.
    Anchored {
        /// Identidade opaca de atestador.
        attestor: AttestorId,
        /// Procedimento declarado, não executado aqui.
        procedure: Text,
    },
    /// Explicação canônica sem ancoragem formal.
    Glossed(Text),
}

/// Declaração de símbolo; o nome é o da assinatura, sem campo redundante.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolDecl {
    /// Assinatura completa, única por nome (sem overload).
    pub signature: Symbol,
    /// Grounding declarado.
    pub grounding: Grounding,
    /// Glosa em português que participa da identidade e compatibilidade.
    pub gloss_pt: Text,
}

/// Compatibilidade direcional de um vocabulário candidato com o anterior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compatibility {
    /// Toda declaração anterior foi preservada integralmente por nome.
    Compatible,
    /// Alguma declaração anterior foi removida ou alterada.
    Incompatible,
}

/// Erro de vocabulário; nunca um veredicto lógico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VocabularyError {
    /// Codificação canônica ou layout inválido.
    Canon(CanonError),
    /// Sintaxe/limites de uma definição inválidos.
    Air(AirError),
    /// Nome de sort declarado mais de uma vez.
    DuplicateSort,
    /// Nome de símbolo declarado mais de uma vez.
    DuplicateSymbol,
    /// Sort referenciado não declarado.
    UnknownSort,
    /// Símbolo referenciado não declarado.
    UnknownSymbol,
    /// Assinatura referenciada diverge da declaração com o mesmo nome.
    SignatureMismatch,
    /// Enumeração contém repetição ou símbolo que não é constante deste sort.
    InvalidDomainConstant,
    /// Dependências entre corpos definidores contêm ciclo.
    CircularDefinition,
    /// Não há versão u64 sucessora.
    VersionOverflow,
    /// Template ausente, malformado ou acima dos limites.
    InvalidRenderTemplate,
}

impl From<CanonError> for VocabularyError {
    fn from(error: CanonError) -> Self {
        Self::Canon(error)
    }
}

impl From<AirError> for VocabularyError {
    fn from(error: AirError) -> Self {
        Self::Air(error)
    }
}

impl fmt::Display for VocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canon(error) => write!(f, "vocabulary canon: {error}"),
            Self::Air(error) => write!(f, "vocabulary definition: {error}"),
            Self::DuplicateSort => f.write_str("duplicate vocabulary sort"),
            Self::DuplicateSymbol => f.write_str("duplicate vocabulary symbol"),
            Self::UnknownSort => f.write_str("undeclared vocabulary sort"),
            Self::UnknownSymbol => f.write_str("undeclared vocabulary symbol"),
            Self::SignatureMismatch => f.write_str("vocabulary signature mismatch"),
            Self::InvalidDomainConstant => f.write_str("invalid enumerated domain constant"),
            Self::CircularDefinition => f.write_str("circular vocabulary definition"),
            Self::InvalidRenderTemplate => f.write_str("invalid render_pt templates"),
            Self::VersionOverflow => f.write_str("vocabulary version overflow"),
        }
    }
}

impl Error for VocabularyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Canon(error) => Some(error),
            Self::Air(error) => Some(error),
            _ => None,
        }
    }
}

/// Vocabulário validado e imutável, com versão de conteúdo distinta do formato v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vocabulary {
    version: u64,
    sorts: Vec<SortDecl>,
    symbols: Vec<SymbolDecl>,
    definition_order: Vec<usize>,
    render_pt: Option<Vec<Text>>,
}

impl Vocabulary {
    /// Valida unicidade, referências completas, enumerações e ausência de ciclos.
    /// Preserva ordem das declarações; não verifica sorts/aridades dos usos.
    ///
    /// # Errors
    /// Nomes repetidos, referências ausentes/inconsistentes, constantes inválidas ou ciclos.
    pub fn new(
        version: u64,
        sorts: Vec<SortDecl>,
        symbols: Vec<SymbolDecl>,
    ) -> Result<Self, VocabularyError> {
        let sort_names = unique_names(&sorts, |decl| &decl.name, VocabularyError::DuplicateSort)?;
        let symbol_names = unique_names(
            &symbols,
            |decl| &decl.signature.name,
            VocabularyError::DuplicateSymbol,
        )?;
        let resolver = Resolver {
            sorts: sort_names,
            symbols: symbol_names,
            declarations: &symbols,
        };
        for decl in &symbols {
            for sort in &decl.signature.arity {
                resolver.sort(sort)?;
            }
            if let SymbolResult::Sort(sort) = &decl.signature.result {
                resolver.sort(sort)?;
            }
        }
        for decl in &sorts {
            if let SortDomain::Enumerated { constants, .. } = &decl.domain {
                let mut seen = BTreeSet::new();
                for constant in constants {
                    resolver.symbol(constant)?;
                    if constant.kind != SymbolKind::Const
                        || !constant.arity.is_empty()
                        || !matches!(&constant.result, SymbolResult::Sort(sort) if sort.name == decl.name)
                        || !seen.insert(&constant.name)
                    {
                        return Err(VocabularyError::InvalidDomainConstant);
                    }
                }
            }
        }
        let definition_order = resolver.definition_order()?;
        Ok(Self {
            version,
            sorts,
            symbols,
            definition_order,
            render_pt: None,
        })
    }

    /// Cria vocabulary/v2 com templates explícitos por ordem de símbolo.
    ///
    /// # Errors
    /// Erros de vocabulário ou templates inválidos; nunca usa gloss_pt como fallback.
    pub fn new_with_render_pt(
        version: u64,
        sorts: Vec<SortDecl>,
        symbols: Vec<SymbolDecl>,
        render_pt: Vec<Text>,
    ) -> Result<Self, VocabularyError> {
        let mut vocabulary = Self::new(version, sorts, symbols)?;
        if render_pt.len() != vocabulary.symbols.len() {
            return Err(VocabularyError::InvalidRenderTemplate);
        }
        let mut total = 0usize;
        for (template, symbol) in render_pt.iter().zip(&vocabulary.symbols) {
            crate::render_template::validate(template, symbol.signature.arity.len())?;
            total = total
                .checked_add(template.as_str().len())
                .ok_or(VocabularyError::InvalidRenderTemplate)?;
            if total > 1_048_576 {
                return Err(VocabularyError::InvalidRenderTemplate);
            }
        }
        vocabulary.render_pt = Some(render_pt);
        Ok(vocabulary)
    }

    /// Templates por ordem de símbolo; None identifica o formato histórico v1.
    #[must_use]
    pub fn render_pt(&self) -> Option<&[Text]> {
        self.render_pt.as_deref()
    }

    /// Versão de conteúdo, não versão do formato de transporte.
    #[must_use]
    pub const fn version(&self) -> u64 {
        self.version
    }

    /// Declarações de sorts na ordem canônica original.
    #[must_use]
    pub fn sorts(&self) -> &[SortDecl] {
        &self.sorts
    }

    /// Declarações de símbolos na ordem canônica original.
    #[must_use]
    pub fn symbols(&self) -> &[SymbolDecl] {
        &self.symbols
    }

    /// Índices dos símbolos Defined, em ordem topológica (dependências primeiro).
    /// Informação derivada, não serializada nem incluída em h(V).
    #[must_use]
    pub fn definition_order(&self) -> &[usize] {
        &self.definition_order
    }

    /// Compara preservação exata das declarações anteriores, independentemente de versão/ordem.
    #[must_use]
    pub fn compatibility_with(&self, previous: &Self) -> Compatibility {
        if self.preserves_render_pt(previous)
            && preserves(&self.sorts, &previous.sorts, |decl| &decl.name)
            && preserves(&self.symbols, &previous.symbols, |decl| {
                &decl.signature.name
            })
        {
            Compatibility::Compatible
        } else {
            Compatibility::Incompatible
        }
    }

    fn preserves_render_pt(&self, previous: &Self) -> bool {
        match (&self.render_pt, &previous.render_pt) {
            (None, None) => true,
            (Some(candidate), Some(old)) => {
                let templates: BTreeMap<_, _> = self
                    .symbols
                    .iter()
                    .zip(candidate)
                    .map(|(symbol, template)| (&symbol.signature.name, template))
                    .collect();
                previous.symbols.iter().zip(old).all(|(symbol, template)| {
                    templates.get(&symbol.signature.name).copied() == Some(template)
                })
            }
            _ => false,
        }
    }

    /// Extensão v2 exige a lista completa, inclusive templates dos novos símbolos.
    ///
    /// # Errors
    /// Patches, versão ou templates inválidos.
    pub fn extend_with_render_pt(
        &self,
        sorts: Vec<SortDecl>,
        symbols: Vec<SymbolDecl>,
        render_pt: Vec<Text>,
    ) -> Result<(Self, Compatibility), VocabularyError> {
        let base = Self::new(self.version, self.sorts.clone(), self.symbols.clone())?;
        let (next, _) = base.extend(sorts, symbols)?;
        let next = Self::new_with_render_pt(next.version, next.sorts, next.symbols, render_pt)?;
        let compatibility = next.compatibility_with(self);
        Ok((next, compatibility))
    }

    /// Aplica patches por nome e incrementa a versão, sem modificar o anterior.
    /// Substituições preservam posição; novos nomes são acrescentados em ordem.
    ///
    /// # Errors
    /// Versão esgotada, patches duplicados, vocabulário inválido ou v2 sem lista explícita.
    /// Para v2, usar extend_with_render_pt; nunca inferir templates de símbolos novos.
    pub fn extend(
        &self,
        sorts: Vec<SortDecl>,
        symbols: Vec<SymbolDecl>,
    ) -> Result<(Self, Compatibility), VocabularyError> {
        if self.render_pt.is_some() {
            return Err(VocabularyError::InvalidRenderTemplate);
        }
        let version = self
            .version
            .checked_add(1)
            .ok_or(VocabularyError::VersionOverflow)?;
        let sorts = patched(
            &self.sorts,
            sorts,
            |decl| &decl.name,
            VocabularyError::DuplicateSort,
        )?;
        let symbols = patched(
            &self.symbols,
            symbols,
            |decl| &decl.signature.name,
            VocabularyError::DuplicateSymbol,
        )?;
        let next = Self::new(version, sorts, symbols)?;
        let compatibility = next.compatibility_with(self);
        Ok((next, compatibility))
    }
}

impl Domain for Vocabulary {
    const TAG: &'static [u8] = tags::VOCAB;
}

fn unique_names<T>(
    items: &[T],
    name: impl Fn(&T) -> &Text,
    duplicate: VocabularyError,
) -> Result<BTreeMap<&Text, usize>, VocabularyError> {
    let mut names = BTreeMap::new();
    for (index, item) in items.iter().enumerate() {
        if names.insert(name(item), index).is_some() {
            return Err(duplicate);
        }
    }
    Ok(names)
}

fn preserves<T: PartialEq>(candidate: &[T], previous: &[T], name: impl Fn(&T) -> &Text) -> bool {
    let names: BTreeMap<_, _> = candidate.iter().map(|decl| (name(decl), decl)).collect();
    previous.iter().all(|decl| {
        names
            .get(name(decl))
            .is_some_and(|candidate| *candidate == decl)
    })
}

fn patched<T: Clone>(
    previous: &[T],
    patches: Vec<T>,
    name: impl Fn(&T) -> &Text,
    duplicate: VocabularyError,
) -> Result<Vec<T>, VocabularyError> {
    unique_names(&patches, &name, duplicate)?;
    let names: BTreeMap<_, _> = previous
        .iter()
        .enumerate()
        .map(|(i, decl)| (name(decl), i))
        .collect();
    let mut replacements = BTreeMap::new();
    let mut appended = Vec::new();
    for patch in patches {
        if let Some(&index) = names.get(name(&patch)) {
            replacements.insert(index, patch);
        } else {
            appended.push(patch);
        }
    }
    let mut next = Vec::with_capacity(previous.len() + appended.len());
    for (index, decl) in previous.iter().enumerate() {
        next.push(replacements.remove(&index).unwrap_or_else(|| decl.clone()));
    }
    next.extend(appended);
    Ok(next)
}

struct Resolver<'a> {
    sorts: BTreeMap<&'a Text, usize>,
    symbols: BTreeMap<&'a Text, usize>,
    declarations: &'a [SymbolDecl],
}

impl Resolver<'_> {
    fn sort(&self, sort: &Sort) -> Result<(), VocabularyError> {
        if self.sorts.contains_key(&sort.name) {
            Ok(())
        } else {
            Err(VocabularyError::UnknownSort)
        }
    }

    fn symbol(&self, symbol: &Symbol) -> Result<usize, VocabularyError> {
        let &index = self
            .symbols
            .get(&symbol.name)
            .ok_or(VocabularyError::UnknownSymbol)?;
        if self.declarations[index].signature != *symbol {
            return Err(VocabularyError::SignatureMismatch);
        }
        Ok(index)
    }

    fn dependency(
        &self,
        symbol: &Symbol,
        dependencies: &mut BTreeSet<usize>,
    ) -> Result<(), VocabularyError> {
        let index = self.symbol(symbol)?;
        if matches!(self.declarations[index].grounding, Grounding::Defined(_)) {
            dependencies.insert(index);
        }
        Ok(())
    }

    fn term(&self, term: &Term, dependencies: &mut BTreeSet<usize>) -> Result<(), VocabularyError> {
        for node in term.nodes() {
            match node {
                TermNode::Var { sort, .. } => self.sort(sort)?,
                TermNode::App { symbol, .. } => self.dependency(symbol, dependencies)?,
            }
        }
        Ok(())
    }

    fn formula(
        &self,
        formula: &Formula,
        dependencies: &mut BTreeSet<usize>,
    ) -> Result<(), VocabularyError> {
        for node in formula.nodes() {
            match node {
                FormulaNode::Eq(left, right) => {
                    self.term(left, dependencies)?;
                    self.term(right, dependencies)?;
                }
                FormulaNode::Pred { symbol, args } => {
                    self.dependency(symbol, dependencies)?;
                    for term in args {
                        self.term(term, dependencies)?;
                    }
                }
                FormulaNode::Forall(sort) | FormulaNode::Exists(sort) => self.sort(sort)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn definition_order(&self) -> Result<Vec<usize>, VocabularyError> {
        let mut dependents = vec![Vec::new(); self.declarations.len()];
        let mut remaining = vec![0usize; self.declarations.len()];
        let mut ready = VecDeque::new();
        let mut count = 0;
        for (index, decl) in self.declarations.iter().enumerate() {
            if let Grounding::Defined(formula) = &decl.grounding {
                count += 1;
                let mut dependencies = BTreeSet::new();
                self.formula(formula, &mut dependencies)?;
                remaining[index] = dependencies.len();
                if dependencies.is_empty() {
                    ready.push_back(index);
                }
                for dependency in dependencies {
                    dependents[dependency].push(index);
                }
            }
        }
        let mut order = Vec::with_capacity(count);
        while let Some(index) = ready.pop_front() {
            order.push(index);
            for &dependent in &dependents[index] {
                // Each distinct edge is counted once and removed once, only after its source.
                remaining[dependent] -= 1;
                if remaining[dependent] == 0 {
                    ready.push_back(dependent);
                }
            }
        }
        if order.len() != count {
            return Err(VocabularyError::CircularDefinition);
        }
        Ok(order)
    }
}
