//! Sintaxe FOL-ms-eq/v1 em pré-ordem; identidade independente de nomes de binders.

use crate::{
    AirError,
    shape::{Shape, Stats},
};
use logos_canon::Text;
use logos_crypto::{Domain, tags};

/// Identificador da lógica que interpreta termos e fórmulas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicId {
    /// Primeira ordem clássica, multi-sortida, com igualdade, versão 1.
    FolMsEqV1,
}

impl LogicId {
    /// Identificador normativo no fio.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FolMsEqV1 => "FOL-ms-eq/v1",
        }
    }

    /// Lê identificador suportado, sem fallback de lógica.
    ///
    /// # Errors
    /// `UnsupportedLogic` para qualquer outro nome.
    pub fn from_name(name: &str) -> Result<Self, AirError> {
        match name {
            "FOL-ms-eq/v1" => Ok(Self::FolMsEqV1),
            _ => Err(AirError::UnsupportedLogic),
        }
    }
}

/// Identidade nominal de sort; declaração/domínio pertencem ao vocabulário.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sort {
    /// Nome canônico (UTF-8 NFC, Unicode 16).
    pub name: Text,
}

/// Categoria declarada de símbolo, ainda não sort-checada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// Constante.
    Const,
    /// Função.
    Func,
    /// Predicado.
    Pred,
}

/// Resultado declarado de uma assinatura.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolResult {
    /// Sort de um valor individual.
    Sort(Sort),
    /// Valor de verdade de predicado.
    Bool,
}

/// Assinatura completa; identidade é conteúdo, não índice em tabela externa.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol {
    /// Nome canônico do símbolo.
    pub name: Text,
    /// Categoria declarada.
    pub kind: SymbolKind,
    /// Sorts dos argumentos, na ordem.
    pub arity: Vec<Sort>,
    /// Resultado declarado.
    pub result: SymbolResult,
}

/// Limites lógicos; não fazem parte da identidade canônica.
///
/// Não limitam bytes de strings/assinaturas, RSS ou trabalho de outros componentes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Profundidade máxima; raiz = 1, termos em Eq/Pred são filhos da fórmula.
    pub max_depth: usize,
    /// Número máximo de nós de fórmula e termo, incluindo termos embutidos.
    pub max_nodes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_depth: 128,
            max_nodes: 100_000,
        }
    }
}

/// Record de termo em pré-ordem; cada filho ocupa seu subtree imediatamente após o pai.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TermNode {
    /// Variável De Bruijn: índice 0 denota binder mais próximo; nomes ausentes.
    Var {
        /// Distância de binder; variáveis livres só serão conferidas pelo sort-check.
        index: u64,
        /// Sort declarado da variável.
        sort: Sort,
    },
    /// Aplicação; Const é representável com zero filhos.
    App {
        /// Assinatura declarada, sem checar compatibilidade nesta etapa.
        symbol: Symbol,
        /// Número de subtermos filhos em pré-ordem (não a aridade da assinatura).
        args: usize,
    },
}

impl TermNode {
    pub(crate) fn children(&self) -> usize {
        match self {
            Self::Var { .. } => 0,
            Self::App { args, .. } => *args,
        }
    }
}

/// Termo completo validado estruturalmente, com armazenamento/descarte não recursivos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term {
    pub(crate) nodes: Vec<TermNode>,
    pub(crate) stats: Stats,
}

impl Term {
    /// Valida e assume ownership dos records em pré-ordem sem copiá-los.
    ///
    /// # Errors
    /// Prefixo vazio/incompleto, raízes extras ou excesso de profundidade/nós.
    pub fn new(nodes: Vec<TermNode>, limits: Limits) -> Result<Self, AirError> {
        let mut shape = Shape::new(limits);
        for node in &nodes {
            shape.push(node.children(), Stats::default())?;
        }
        Ok(Self {
            nodes,
            stats: shape.finish()?,
        })
    }

    /// Records imutáveis em pré-ordem.
    #[must_use]
    pub fn nodes(&self) -> &[TermNode] {
        &self.nodes
    }

    /// Profundidade lógica verificada.
    #[must_use]
    pub const fn depth(&self) -> usize {
        self.stats.depth
    }

    /// Número de nós lógicos verificado.
    #[must_use]
    pub const fn node_count(&self) -> usize {
        self.stats.nodes
    }

    /// Lógica do envelope canônico.
    #[must_use]
    pub const fn logic_id(&self) -> LogicId {
        LogicId::FolMsEqV1
    }
}

/// Record de fórmula em pré-ordem; Eq/Pred possuem termos planos separados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormulaNode {
    /// Verdade.
    True,
    /// Falsidade.
    False,
    /// Igualdade de dois termos; compatibilidade de sorts será checada na 1.3.
    Eq(Term, Term),
    /// Aplicação de predicado; assinatura/argumentos ainda não sort-checados.
    Pred {
        /// Assinatura declarada.
        symbol: Symbol,
        /// Termos argumentos, na ordem original.
        args: Vec<Term>,
    },
    /// Negação, com um filho de fórmula.
    Not,
    /// Conjunção com número explícito de filhos; zero filhos admitido.
    And(usize),
    /// Disjunção com número explícito de filhos; zero filhos admitido.
    Or(usize),
    /// Implicação, dois filhos na ordem antecedente/consequente.
    Implies,
    /// Bicondicional, dois filhos na ordem original.
    Iff,
    /// Quantificador universal com sort; corpo é o próximo subtree, sem nome de binder.
    Forall(Sort),
    /// Quantificador existencial com sort; corpo é o próximo subtree, sem nome de binder.
    Exists(Sort),
}

impl FormulaNode {
    pub(crate) fn children(&self) -> usize {
        match self {
            Self::Not | Self::Forall(_) | Self::Exists(_) => 1,
            Self::And(children) | Self::Or(children) => *children,
            Self::Implies | Self::Iff => 2,
            Self::True | Self::False | Self::Eq(..) | Self::Pred { .. } => 0,
        }
    }

    pub(crate) fn embedded_stats(&self) -> Result<Stats, AirError> {
        match self {
            Self::Eq(left, right) => left.stats.combine(right.stats),
            Self::Pred { args, .. } => args
                .iter()
                .try_fold(Stats::default(), |stats, term| stats.combine(term.stats)),
            _ => Ok(Stats::default()),
        }
    }
}

/// Fórmula completa, com LogicId e limites de todos os termos embutidos.
///
/// Validação estrutural não é prova lógica, credencial ou sort-check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formula {
    pub(crate) nodes: Vec<FormulaNode>,
    pub(crate) stats: Stats,
}

impl Formula {
    /// Valida prefixo e limites agregados, assumindo ownership sem cópia.
    ///
    /// # Errors
    /// Prefixo vazio/incompleto, raízes extras ou excesso de profundidade/nós,
    /// inclusive quando termos filhos foram construídos com limites maiores.
    pub fn new(nodes: Vec<FormulaNode>, limits: Limits) -> Result<Self, AirError> {
        let mut shape = Shape::new(limits);
        for node in &nodes {
            shape.push(node.children(), node.embedded_stats()?)?;
        }
        Ok(Self {
            nodes,
            stats: shape.finish()?,
        })
    }

    /// Records imutáveis em pré-ordem.
    #[must_use]
    pub fn nodes(&self) -> &[FormulaNode] {
        &self.nodes
    }

    /// Profundidade lógica, incluindo termos embutidos.
    #[must_use]
    pub const fn depth(&self) -> usize {
        self.stats.depth
    }

    /// Número total de nós de fórmula e termos embutidos.
    #[must_use]
    pub const fn node_count(&self) -> usize {
        self.stats.nodes
    }

    /// Lógica do envelope canônico.
    #[must_use]
    pub const fn logic_id(&self) -> LogicId {
        LogicId::FolMsEqV1
    }
}

impl Domain for Formula {
    const TAG: &'static [u8] = tags::FORMULA;
}
