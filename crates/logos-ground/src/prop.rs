//! DAG proposicional canônico de saída; não é certificado ou CNF.
use logos_air::Vocabulary;
use logos_canon::{Array, Canon, CanonError, Value};
use logos_crypto::{Domain, Hash, tags};
use logos_obligation::{Obligation, PremiseRef};

/// Índice topológico de fórmula no DAG de um GroundProgram.
pub type PropFormula = usize;

/// Variável de interpretação, ordenada por símbolo/argumentos/resultado.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Atom {
    /// Índice original do símbolo em V.
    pub symbol: u32,
    /// Ordinais de entrada, último eixo mais rápido.
    pub arguments: Vec<u32>,
    /// None para Pred; Some para escolha de valor.
    pub output: Option<u32>,
}

/// Nó lógico cujos filhos são índices anteriores no mesmo programa.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PropNode {
    /// Falsidade.
    False,
    /// Verdade.
    True,
    /// Índice na tabela de átomos.
    Atom(usize),
    /// Negação.
    Not(PropFormula),
    /// Conjunção em ordem de construção.
    And(Vec<PropFormula>),
    /// Disjunção em ordem de construção.
    Or(Vec<PropFormula>),
}

/// Tradução lógica condicionada às premissas; sem autoridade/claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundProgram {
    pub(crate) obligation: Hash<Obligation>,
    pub(crate) vocabulary: Hash<Vocabulary>,
    pub(crate) atoms: Vec<Atom>,
    pub(crate) nodes: Vec<PropNode>,
    pub(crate) constraints: PropFormula,
    pub(crate) premises: Vec<(PremiseRef, PropFormula)>,
    pub(crate) goal: PropFormula,
    pub(crate) root: PropFormula,
    pub(crate) domain_axioms: Vec<PremiseRef>,
    // Métadados somente em memória; não fazem parte de ground/v1.
    pub(crate) premise_support: Vec<Vec<PremiseRef>>,
    pub(crate) goal_support: Vec<PremiseRef>,
}
impl GroundProgram {
    /// Obrigação exata que foi traduzida.
    #[must_use]
    pub fn obligation(&self) -> Hash<Obligation> {
        self.obligation
    }
    /// Vocabulário exato utilizado.
    #[must_use]
    pub fn vocabulary(&self) -> Hash<Vocabulary> {
        self.vocabulary
    }
    /// Tabela canônica, sem variáveis SAT.
    #[must_use]
    pub fn atoms(&self) -> &[Atom] {
        &self.atoms
    }
    /// DAG topológico imutável.
    #[must_use]
    pub fn nodes(&self) -> &[PropNode] {
        &self.nodes
    }
    /// Guardas de interpretações admissíveis.
    #[must_use]
    pub fn constraints(&self) -> PropFormula {
        self.constraints
    }
    /// Premissas com suas identidades preservadas.
    #[must_use]
    pub fn premises(&self) -> &[(PremiseRef, PropFormula)] {
        &self.premises
    }
    /// Objetivo traduzido.
    #[must_use]
    pub fn goal(&self) -> PropFormula {
        self.goal
    }
    /// (Restrições ∧ Γ) → goal.
    #[must_use]
    pub fn root(&self) -> PropFormula {
        self.root
    }
    /// Referências dos axiomas explícitos exigidos, sem fonte autenticada implícita.
    #[must_use]
    pub fn domain_axioms(&self) -> &[PremiseRef] {
        &self.domain_axioms
    }
}
impl Domain for GroundProgram {
    const TAG: &'static [u8] = tags::GROUND;
}
fn array(v: Vec<Value>) -> Result<Value, CanonError> {
    Ok(Value::Array(Array::new(v)?))
}
fn number(n: usize) -> Result<Value, CanonError> {
    Value::int(n as i128)
}
impl Canon for GroundProgram {
    fn to_value(&self) -> Result<Value, CanonError> {
        let atoms = self
            .atoms
            .iter()
            .map(|a| {
                array(vec![
                    a.symbol.to_value()?,
                    a.arguments.to_value()?,
                    a.output.to_value()?,
                ])
            })
            .collect::<Result<Vec<_>, _>>()?;
        let nodes = self
            .nodes
            .iter()
            .map(|n| match n {
                PropNode::False => array(vec![Value::int(0)?]),
                PropNode::True => array(vec![Value::int(1)?]),
                PropNode::Atom(i) => array(vec![Value::int(2)?, number(*i)?]),
                PropNode::Not(i) => array(vec![Value::int(3)?, number(*i)?]),
                PropNode::And(xs) | PropNode::Or(xs) => array(vec![
                    Value::int(if matches!(n, PropNode::And(_)) { 4 } else { 5 })?,
                    array(
                        xs.iter()
                            .map(|n| number(*n))
                            .collect::<Result<Vec<_>, _>>()?,
                    )?,
                ]),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let premises = self
            .premises
            .iter()
            .map(|(r, n)| array(vec![r.event_hash().to_value()?, number(*n)?]))
            .collect::<Result<Vec<_>, _>>()?;
        array(vec![
            Value::int(1)?,
            self.obligation.to_value()?,
            self.vocabulary.to_value()?,
            array(atoms)?,
            array(nodes)?,
            number(self.constraints)?,
            array(premises)?,
            number(self.goal)?,
            number(self.root)?,
            array(
                self.domain_axioms
                    .iter()
                    .map(|r| r.event_hash().to_value())
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
        ])
    }
}
