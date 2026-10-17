//! Suporte sintático da tradução finita, sem alterar o DAG ou seus bytes.
use logos_air::{
    Formula, FormulaNode, Grounding, Sort, Symbol, SymbolResult, Term, TermNode, Vocabulary,
};
use logos_canon::Text;
use logos_obligation::PremiseRef;

use crate::GroundError;
use crate::dag::{Dag, push, size, space};

enum Job<'a> {
    Formula(&'a Formula),
    Term(&'a Term),
    Symbol(&'a Symbol),
    Sort(&'a Sort),
}

/// Visita tipos/corpos alcançados com pilha explícita; não minimiza hipóteses.
pub(crate) fn formula_support<'a>(
    v: &'a Vocabulary,
    formula: &'a Formula,
    sorts: &[(Text, Vec<PremiseRef>)],
    dag: &mut Dag,
) -> Result<Vec<PremiseRef>, GroundError> {
    dag.charge(
        v.symbols()
            .len()
            .checked_add(v.sorts().len())
            .ok_or_else(size)?,
    )?;
    let mut seen_symbols = space(v.symbols().len(), dag.limits.analysis.max_declarations)?;
    seen_symbols.resize(v.symbols().len(), false);
    let mut seen_sorts = space(v.sorts().len(), dag.limits.analysis.max_declarations)?;
    seen_sorts.resize(v.sorts().len(), false);
    let mut jobs = Vec::new();
    let mut result = Vec::new();
    schedule(&mut jobs, Job::Formula(formula), dag)?;
    while let Some(job) = jobs.pop() {
        dag.charge(1)?;
        match job {
            Job::Formula(f) => {
                dag.charge(f.nodes().len())?;
                for node in f.nodes() {
                    match node {
                        FormulaNode::Forall(s) | FormulaNode::Exists(s) => {
                            schedule(&mut jobs, Job::Sort(s), dag)?
                        }
                        FormulaNode::Eq(a, b) => {
                            schedule(&mut jobs, Job::Term(a), dag)?;
                            schedule(&mut jobs, Job::Term(b), dag)?;
                        }
                        FormulaNode::Pred { symbol, args } => {
                            schedule(&mut jobs, Job::Symbol(symbol), dag)?;
                            for t in args {
                                schedule(&mut jobs, Job::Term(t), dag)?;
                            }
                        }
                        _ => {}
                    }
                }
            }
            Job::Term(t) => {
                dag.charge(t.nodes().len())?;
                for node in t.nodes() {
                    match node {
                        TermNode::Var { sort, .. } => schedule(&mut jobs, Job::Sort(sort), dag)?,
                        TermNode::App { symbol, .. } => {
                            schedule(&mut jobs, Job::Symbol(symbol), dag)?
                        }
                    }
                }
            }
            Job::Symbol(symbol) => {
                dag.charge(v.symbols().len())?;
                let id = v
                    .symbols()
                    .iter()
                    .position(|d| d.signature.name == symbol.name)
                    .ok_or(GroundError::InvalidContext)?;
                if seen_symbols[id] {
                    continue;
                }
                seen_symbols[id] = true;
                for sort in &symbol.arity {
                    schedule(&mut jobs, Job::Sort(sort), dag)?;
                }
                if let SymbolResult::Sort(sort) = &symbol.result {
                    schedule(&mut jobs, Job::Sort(sort), dag)?;
                }
                if let Grounding::Defined(body) = &v.symbols()[id].grounding {
                    schedule(&mut jobs, Job::Formula(body), dag)?;
                }
            }
            Job::Sort(sort) => {
                dag.charge(v.sorts().len())?;
                let id = v
                    .sorts()
                    .iter()
                    .position(|d| d.name == sort.name)
                    .ok_or(GroundError::InvalidContext)?;
                if seen_sorts[id] {
                    continue;
                }
                seen_sorts[id] = true;
                dag.charge(sorts.len())?;
                if let Some((_, refs)) = sorts.iter().find(|(name, _)| *name == sort.name) {
                    dag.charge(refs.len())?;
                    for r in refs {
                        if result.len() >= dag.limits.max_cells {
                            return Err(size());
                        }
                        push(&mut result, *r)?;
                    }
                }
            }
        }
    }
    dag.charge(
        result
            .len()
            .checked_mul(result.len().checked_ilog2().unwrap_or(0) as usize + 1)
            .ok_or_else(size)?,
    )?;
    result.sort_unstable();
    result.dedup();
    Ok(result)
}

fn schedule<'a>(jobs: &mut Vec<Job<'a>>, job: Job<'a>, dag: &mut Dag) -> Result<(), GroundError> {
    dag.charge(1)?;
    if jobs.len() >= dag.limits.max_cells {
        return Err(size());
    }
    push(jobs, job)
}
