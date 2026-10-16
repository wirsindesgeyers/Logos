//! Equivalências completas de Tseitin e DIMACS canônico, conforme spec/cnf.md.
use std::fmt::Write;

use logos_crypto::{Domain, tags};
use logos_obligation::PremiseRef;

use crate::dag::{push, size, space};
use crate::{GroundError, GroundProgram, PropNode};

/// Origem de cláusula original, na ordem canônica de desempate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClauseOrigin {
    /// Assertiva de uma premissa explícita de O.
    Premise(PremiseRef),
    /// Assertiva da negação do objetivo.
    NegatedGoal,
    /// Assertiva de axioma explícito exigido pela transformação.
    DomainAxiom(PremiseRef),
    /// Equivalência de Tseitin ou restrição semântica do contexto.
    Definitional,
}

/// Mapa alinhado aos IDs 1-based de cláusulas; não calcula used de prova.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceMap {
    origins: Vec<ClauseOrigin>,
    domain_support: Vec<PremiseRef>,
}
impl ProvenanceMap {
    /// Origem da cláusula i+1 na posição i.
    pub fn origins(&self) -> &[ClauseOrigin] {
        &self.origins
    }
    /// Suporte da transformação, inclusive axiomas que viraram tautologias.
    pub fn domain_support(&self) -> &[PremiseRef] {
        &self.domain_support
    }
}

/// CNF imutável; identidade definida pelos bytes DIMACS, sem envelope CBOR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cnf {
    variables: usize,
    clauses: Vec<Vec<i32>>,
    dimacs: String,
}
impl Domain for Cnf {
    const TAG: &'static [u8] = tags::CNF;
}
impl Cnf {
    /// Quantidade de variáveis primitivas e auxiliares alocadas.
    pub fn variables(&self) -> usize {
        self.variables
    }
    /// Cláusulas em ordem canônica; ID da posição i é i+1.
    pub fn clauses(&self) -> &[Vec<i32>] {
        &self.clauses
    }
    /// Bytes exatos de cnf/v1 para hash_raw/CAS e provadores externos.
    pub fn dimacs(&self) -> &[u8] {
        self.dimacs.as_bytes()
    }
}

/// Limites inclusivos de construção; não são teto de RSS/tempo.
#[derive(Debug, Clone, Copy)]
pub struct CnfLimits {
    /// Máximo de variáveis, também limitado por i32::MAX.
    pub max_variables: usize,
    /// Máximo de cláusulas, contando duplicatas.
    pub max_clauses: usize,
    /// Máximo de ocorrências de literais.
    pub max_literals: usize,
    /// Máximo de bytes DIMACS, incluindo cabeçalho e LF final.
    pub max_bytes: usize,
}
impl Default for CnfLimits {
    fn default() -> Self {
        Self {
            max_variables: 1_000_000,
            max_clauses: 1_000_000,
            max_literals: 5_000_000,
            max_bytes: 16_777_216,
        }
    }
}
struct Builder {
    limits: CnfLimits,
    literals: usize,
    records: Vec<(Vec<i32>, ClauseOrigin)>,
}
impl Builder {
    fn clause(
        &mut self,
        origin: ClauseOrigin,
        count: usize,
        literals: impl IntoIterator<Item = i32>,
    ) -> Result<(), GroundError> {
        if self.records.len() >= self.limits.max_clauses {
            return Err(size());
        }
        self.literals = self.literals.checked_add(count).ok_or_else(size)?;
        if self.literals > self.limits.max_literals {
            return Err(size());
        }
        let mut out = space(count, self.limits.max_literals)?;
        for literal in literals {
            if literal == 0 || out.len() >= count {
                return Err(GroundError::InvalidContext);
            }
            out.push(literal);
        }
        if out.len() != count {
            return Err(GroundError::InvalidContext);
        }
        out.sort_unstable();
        push(&mut self.records, (out, origin))
    }
}

/// Traduz as restrições e Γ junto com a negação de goal, sem solver.
/// # Errors
/// Limite inclusivo/overflow, reserva falível ou DAG inconsistente.
pub fn clausify(g: &GroundProgram) -> Result<(Cnf, ProvenanceMap), GroundError> {
    clausify_with_limits(g, CnfLimits::default())
}

/// Variante limitada do encoder cnf/v1. Nunca devolve saída parcial.
/// # Errors
/// Os de clausify, com limites inclusivos explícitos.
pub fn clausify_with_limits(
    g: &GroundProgram,
    limits: CnfLimits,
) -> Result<(Cnf, ProvenanceMap), GroundError> {
    let mut variables = g.atoms().len();
    for n in g.nodes() {
        if !matches!(n, PropNode::Atom(_)) {
            variables = variables.checked_add(1).ok_or_else(size)?;
        }
    }
    if variables > limits.max_variables || variables > i32::MAX as usize {
        return Err(size());
    }
    let mut ids: Vec<i32> = space(g.nodes().len(), limits.max_variables)?;
    let mut next = g.atoms().len();
    let mut b = Builder {
        limits,
        literals: 0,
        records: Vec::new(),
    };
    for n in g.nodes() {
        let id = if let PropNode::Atom(a) = n {
            if *a >= g.atoms().len() {
                return Err(GroundError::InvalidContext);
            }
            i32::try_from(a.checked_add(1).ok_or_else(size)?).map_err(|_| size())?
        } else {
            next = next.checked_add(1).ok_or_else(size)?;
            i32::try_from(next).map_err(|_| size())?
        };
        let child = |i: &usize| ids.get(*i).copied().ok_or(GroundError::InvalidContext);
        let d = ClauseOrigin::Definitional;
        match n {
            PropNode::False => b.clause(d, 1, [-id])?,
            PropNode::True => b.clause(d, 1, [id])?,
            PropNode::Atom(_) => {}
            PropNode::Not(a) => {
                let a = child(a)?;
                b.clause(d, 2, [-id, -a])?;
                b.clause(d, 2, [id, a])?;
            }
            PropNode::And(xs) | PropNode::Or(xs) => {
                let and = matches!(n, PropNode::And(_));
                let mut joined = space(
                    xs.len().checked_add(1).ok_or_else(size)?,
                    limits.max_literals,
                )?;
                joined.push(if and { id } else { -id });
                for x in xs {
                    let x = child(x)?;
                    b.clause(d, 2, if and { [-id, x] } else { [id, -x] })?;
                    joined.push(if and { -x } else { x });
                }
                b.clause(d, joined.len(), joined)?;
            }
        }
        ids.push(id);
    }
    let root = |n: usize| ids.get(n).copied().ok_or(GroundError::InvalidContext);
    b.clause(ClauseOrigin::Definitional, 1, [root(g.constraints())?])?;
    for (r, n) in g.premises() {
        let origin = if g.domain_axioms().contains(r) {
            ClauseOrigin::DomainAxiom(*r)
        } else {
            ClauseOrigin::Premise(*r)
        };
        b.clause(origin, 1, [root(*n)?])?;
    }
    b.clause(ClauseOrigin::NegatedGoal, 1, [-root(g.goal())?])?;
    b.records.sort_unstable();
    let mut bytes = format!("p cnf {variables} {}\n", b.records.len()).len();
    for (c, _) in &b.records {
        bytes = bytes.checked_add(2).ok_or_else(size)?; // zero e LF
        for x in c {
            bytes = bytes
                .checked_add(x.unsigned_abs().ilog10() as usize + 1 + usize::from(*x < 0) + 1)
                .ok_or_else(size)?;
        }
        if bytes > limits.max_bytes {
            return Err(size());
        }
    }
    if bytes > limits.max_bytes {
        return Err(size());
    }
    let mut dimacs = String::new();
    dimacs
        .try_reserve_exact(bytes)
        .map_err(|_| GroundError::Unknown(logos_core::Exhausted::Memory))?;
    writeln!(dimacs, "p cnf {variables} {}", b.records.len())
        .map_err(|_| GroundError::InvalidContext)?;
    let mut clauses = space(b.records.len(), limits.max_clauses)?;
    let mut origins = space(b.records.len(), limits.max_clauses)?;
    for (c, origin) in b.records {
        for x in &c {
            write!(dimacs, "{x} ").map_err(|_| GroundError::InvalidContext)?;
        }
        dimacs.push_str("0\n");
        clauses.push(c);
        origins.push(origin);
    }
    let mut domain_support = space(g.domain_axioms().len(), usize::MAX)?;
    domain_support.extend_from_slice(g.domain_axioms());
    Ok((
        Cnf {
            variables,
            clauses,
            dimacs,
        },
        ProvenanceMap {
            origins,
            domain_support,
        },
    ))
}
