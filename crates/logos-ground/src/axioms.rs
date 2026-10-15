//! Fecho e nomes únicos são candidatos, nunca premissas autenticadas.

use logos_air::{
    DomainSource, Formula, FormulaNode, Limits, Sort, SortDomain, Symbol, Term, TermNode,
    Vocabulary,
};
use logos_core::Exhausted;
use logos_crypto::{Hash, hash};

use crate::GroundError;

/// Tetos lógicos de saída, sem teto físico de RSS ou relógio.
#[derive(Debug, Clone, Copy)]
pub struct AxiomLimits {
    /// Máximo de constantes antes de expansão quadrática.
    pub max_constants: usize,
    /// Quantidade total de fórmulas (um fecho + desigualdades por pares).
    pub max_axioms: usize,
    /// Soma dos nós lógicos de todas as fórmulas produzidas.
    pub max_nodes: usize,
    /// Limites AIR por fórmula, inclusive termos embutidos.
    pub air: Limits,
}
impl Default for AxiomLimits {
    fn default() -> Self {
        Self {
            max_constants: 256,
            max_axioms: 10_000,
            max_nodes: 100_000,
            air: Limits::default(),
        }
    }
}

/// Fórmulas lógicas ligadas a h(V)/sort/fonte opaca, sem autoridade de premissa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxiomCandidates {
    vocabulary: Hash<Vocabulary>,
    sort: Sort,
    source: DomainSource,
    closure: Formula,
    distinct: Vec<Formula>,
}
impl AxiomCandidates {
    /// Identidade do vocabulário completo que originou os candidatos.
    #[must_use]
    pub fn vocabulary(&self) -> Hash<Vocabulary> {
        self.vocabulary
    }
    /// Sort enumerado solicitado.
    #[must_use]
    pub fn sort(&self) -> &Sort {
        &self.sort
    }
    /// Fonte declarada, ainda sem autenticação ou resolução para evento.
    #[must_use]
    pub fn source(&self) -> &DomainSource {
        &self.source
    }
    /// ∀x. x=c0 ∨ ... ∨ x=cn; enumeração vazia dá ∀x.Or(0).
    #[must_use]
    pub fn closure(&self) -> &Formula {
        &self.closure
    }
    /// ¬(ci=cj) para i<j, na ordem lexicográfica dos índices originais.
    #[must_use]
    pub fn distinct(&self) -> &[Formula] {
        &self.distinct
    }
}

/// Constrói candidatos com limites padrão; não modifica nenhuma obrigação.
///
/// # Errors
/// Sort ausente/Open, excesso de limites, reserva ou construção inválida.
pub fn domain_axioms(v: &Vocabulary, sort: &Sort) -> Result<AxiomCandidates, GroundError> {
    domain_axioms_with_limits(v, sort, AxiomLimits::default())
}

/// Constrói fecho e nomes únicos, recusando antes da expansão quadrática.
///
/// # Errors
/// Os de [`domain_axioms`]. Nenhum erro produz lista parcial ou truncada.
pub fn domain_axioms_with_limits(
    v: &Vocabulary,
    sort: &Sort,
    limits: AxiomLimits,
) -> Result<AxiomCandidates, GroundError> {
    let decl = v
        .sorts()
        .iter()
        .find(|s| s.name == sort.name)
        .ok_or(GroundError::UnknownSort)?;
    let SortDomain::Enumerated {
        constants,
        domain_source,
    } = &decl.domain
    else {
        return Err(GroundError::OpenDomain);
    };
    let size = || GroundError::Unknown(Exhausted::Size);
    let n = constants.len();
    if n > limits.max_constants {
        return Err(size());
    }
    let pairs = n.checked_mul(n.saturating_sub(1)).ok_or_else(size)? / 2;
    let count = pairs.checked_add(1).ok_or_else(size)?;
    let nodes = n
        .checked_mul(3)
        .and_then(|n| n.checked_add(2))
        .and_then(|n| pairs.checked_mul(4).and_then(|p| n.checked_add(p)))
        .ok_or_else(size)?;
    if count > limits.max_axioms || nodes > limits.max_nodes {
        return Err(size());
    }
    let mut body = Vec::new();
    body.try_reserve_exact(n.checked_add(2).ok_or_else(size)?)
        .map_err(|_| GroundError::Unknown(Exhausted::Memory))?;
    body.push(FormulaNode::Forall(sort.clone()));
    body.push(FormulaNode::Or(n));
    for constant in constants {
        let var = Term::new(
            vec![TermNode::Var {
                index: 0,
                sort: sort.clone(),
            }],
            limits.air,
        )?;
        body.push(FormulaNode::Eq(var, constant_term(constant, limits.air)?));
    }
    let closure = Formula::new(body, limits.air)?;
    logos_air::check(v, &closure)?;
    let mut distinct = Vec::new();
    distinct
        .try_reserve_exact(pairs)
        .map_err(|_| GroundError::Unknown(Exhausted::Memory))?;
    for i in 0..n {
        for j in i + 1..n {
            let formula = Formula::new(
                vec![
                    FormulaNode::Not,
                    FormulaNode::Eq(
                        constant_term(&constants[i], limits.air)?,
                        constant_term(&constants[j], limits.air)?,
                    ),
                ],
                limits.air,
            )?;
            logos_air::check(v, &formula)?;
            distinct.push(formula);
        }
    }
    Ok(AxiomCandidates {
        vocabulary: hash(v)?,
        sort: sort.clone(),
        source: domain_source.clone(),
        closure,
        distinct,
    })
}

fn constant_term(symbol: &Symbol, limits: Limits) -> Result<Term, GroundError> {
    Ok(Term::new(
        vec![TermNode::App {
            symbol: symbol.clone(),
            args: 0,
        }],
        limits,
    )?)
}
