//! RUP sequencial e fecho histórico, sem solver ou ledger.

use logos_core::{Budget, CheckOutcome, Exhausted, StepMeter};
use logos_crypto::{Hash, hash_raw};
use logos_ground::Cnf;

use crate::parse::{cnf, literal, natural, proof_lines, tokens};
use crate::resources::{Failure, Resources, Result};
use crate::{LratChecker, LratReport};

pub(crate) struct Clause {
    pub id: u64,
    pub lits: Vec<i32>,
    pub deps: Vec<usize>,
    pub active: bool,
}

impl LratChecker {
    /// Confere P2 e todas as linhas de lrat-rup/v1 sob um único Budget.
    ///
    /// Limites lógicos de memória/tamanho/passos; não impõe RSS nem wall_ms.
    #[must_use]
    pub fn check_lrat(
        &self,
        bytes: &[u8],
        expected: Hash<Cnf>,
        proof: &[u8],
        budget: &Budget,
    ) -> CheckOutcome {
        self.check_with_used(bytes, expected, proof, budget).outcome
    }

    /// Acrescenta fecho de IDs originais; nunca expõe resultado parcial.
    #[must_use]
    pub fn check_with_used(
        &self,
        bytes: &[u8],
        expected: Hash<Cnf>,
        proof: &[u8],
        budget: &Budget,
    ) -> LratReport {
        let mut used = None;
        let outcome = StepMeter::run(budget, |meter| {
            let mut res = Resources::new(meter, budget.mem_bytes);
            match run(bytes, expected, proof, budget, &mut res) {
                Ok(ids) => {
                    used = Some(ids);
                    CheckOutcome::Accepted
                }
                Err(Failure::Unknown(reason)) => CheckOutcome::Unknown(reason),
                Err(Failure::Invalid) => CheckOutcome::Rejected(self.invalid.clone()),
            }
        });
        if outcome != CheckOutcome::Accepted {
            used = None;
        }
        LratReport { outcome, used }
    }
}

fn find(clauses: &[Clause], id: u64, res: &Resources<'_>) -> Result<usize> {
    res.step(1 + clauses.len().checked_ilog2().unwrap_or(0) as usize)?;
    let index = clauses
        .binary_search_by_key(&id, |c| c.id)
        .map_err(|_| Failure::Invalid)?;
    if !clauses[index].active {
        return Err(Failure::Invalid);
    }
    Ok(index)
}

fn run(
    bytes: &[u8],
    expected: Hash<Cnf>,
    proof: &[u8],
    budget: &Budget,
    res: &mut Resources<'_>,
) -> Result<Vec<u64>> {
    let size = bytes
        .len()
        .checked_add(proof.len())
        .ok_or(Exhausted::Size)?;
    if u64::try_from(size).map_err(|_| Exhausted::Size)? > budget.size_bytes {
        return Err(Exhausted::Size.into());
    }
    res.step(size)?;
    res.step(bytes.len())?;
    if hash_raw::<Cnf>(bytes) != expected {
        return Err(Failure::Invalid);
    }
    let (variables, mut clauses) = cnf(bytes, res)?;
    let originals = clauses.len();
    let mut last = originals as u64;
    let mut assignment = res.allocate::<i8>(variables + 1)?;
    assignment.resize(variables + 1, 0);
    let mut empty = None;
    for line in proof_lines(proof)? {
        let mut parts = tokens(line);
        let Some(first) = parts.next() else {
            continue;
        };
        if first == b"c" {
            continue;
        }
        let id = natural(first)?;
        let next = parts.next().ok_or(Failure::Invalid)?;
        if next == b"d" {
            if id != last {
                return Err(Failure::Invalid);
            }
            loop {
                let token = parts.next().ok_or(Failure::Invalid)?;
                if token == b"0" {
                    break;
                }
                let index = find(&clauses, natural(token)?, res)?;
                clauses[index].active = false;
            }
        } else {
            if id <= last {
                return Err(Failure::Invalid);
            }
            let mut lits = Vec::new();
            let mut token = next;
            while token != b"0" {
                res.push(&mut lits, literal(token, variables)?)?;
                token = parts.next().ok_or(Failure::Invalid)?;
            }
            let mut deps = Vec::new();
            loop {
                let token = parts.next().ok_or(Failure::Invalid)?;
                if token == b"0" {
                    break;
                }
                let index = find(&clauses, natural(token)?, res)?;
                res.push(&mut deps, index)?;
            }
            rup(&lits, &deps, &clauses, &mut assignment, res)?;
            if lits.is_empty() && empty.is_none() {
                empty = Some(clauses.len());
            }
            res.push(
                &mut clauses,
                Clause {
                    id,
                    lits,
                    deps,
                    active: true,
                },
            )?;
            last = id;
        }
        if parts.next().is_some() {
            return Err(Failure::Invalid);
        }
    }
    closure(&clauses, originals, empty.ok_or(Failure::Invalid)?, res)
}

fn rup(
    lits: &[i32],
    deps: &[usize],
    clauses: &[Clause],
    assignment: &mut [i8],
    res: &Resources<'_>,
) -> Result<()> {
    res.step(assignment.len())?;
    assignment.fill(0);
    for &lit in lits {
        res.step(1)?;
        let index = lit.unsigned_abs() as usize;
        let value = if lit > 0 { -1 } else { 1 };
        if assignment[index] == -value {
            return Err(Failure::Invalid);
        }
        assignment[index] = value;
    }
    for (position, &index) in deps.iter().enumerate() {
        res.step(1)?;
        let mut unit = None;
        for &lit in &clauses[index].lits {
            res.step(1)?;
            let value = assignment[lit.unsigned_abs() as usize];
            if (lit > 0 && value == 1) || (lit < 0 && value == -1) {
                return Err(Failure::Invalid);
            }
            if value == 0 {
                if unit.is_some_and(|old| old != lit) {
                    return Err(Failure::Invalid);
                }
                unit = Some(lit);
            }
        }
        match unit {
            None if position + 1 == deps.len() => return Ok(()),
            None => return Err(Failure::Invalid),
            Some(lit) => assignment[lit.unsigned_abs() as usize] = if lit > 0 { 1 } else { -1 },
        }
    }
    Err(Failure::Invalid)
}

fn closure(
    clauses: &[Clause],
    originals: usize,
    empty: usize,
    res: &mut Resources<'_>,
) -> Result<Vec<u64>> {
    res.step(clauses.len())?;
    let mut seen = res.allocate::<u8>(clauses.len())?;
    seen.resize(clauses.len(), 0);
    let mut stack = Vec::new();
    res.push(&mut stack, empty)?;
    seen[empty] = 1;
    while let Some(index) = stack.pop() {
        res.step(1)?;
        for &dep in &clauses[index].deps {
            res.step(1)?;
            if seen[dep] == 0 {
                seen[dep] = 1;
                res.push(&mut stack, dep)?;
            }
        }
    }
    let mut result = Vec::new();
    for (index, clause) in clauses[..originals].iter().enumerate() {
        res.step(1)?;
        if seen[index] == 1 {
            res.push(&mut result, clause.id)?;
        }
    }
    Ok(result)
}
