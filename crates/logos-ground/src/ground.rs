//! Grounding finito por tabelas simbólicas e pilhas explícitas.
use crate::dag::{Dag, push, size, space};
use crate::{AnalysisLimits, Atom, Fragment, GroundError, GroundProgram, PropNode, domain_axioms};
use logos_air::{
    Formula, FormulaNode as F, Grounding, Sort, SortDomain, Symbol, SymbolResult, Term, TermNode,
    Vocabulary,
};
use logos_canon::to_canon;
use logos_crypto::{Hash, hash};
use logos_obligation::{DecodeLimits, Obligation, PremiseRef, decode_for_check};
use std::collections::BTreeMap;

/// Limites inclusivos da construção; não são hard RSS/wall.
#[derive(Debug, Clone, Copy)]
pub struct GroundLimits {
    /// Limites de entrada para classify.
    pub analysis: AnalysisLimits,
    /// Variáveis primitivas.
    pub max_atoms: usize,
    /// Nós distintos do DAG.
    pub max_nodes: usize,
    /// Arestas do DAG.
    pub max_edges: usize,
    /// Células de tabelas agregadas e limite de vetores temporários.
    pub max_cells: usize,
    /// Trabalho lógico de construção.
    pub max_steps: usize,
    /// Bytes do artefato final.
    pub max_bytes: usize,
}
impl Default for GroundLimits {
    fn default() -> Self {
        Self {
            analysis: AnalysisLimits::default(),
            max_atoms: 10_000,
            max_nodes: 100_000,
            max_edges: 500_000,
            max_cells: 100_000,
            max_steps: 1_000_000,
            max_bytes: 16_777_216,
        }
    }
}
struct Table {
    cards: Vec<usize>,
    rows: Vec<Vec<usize>>,
}
struct Engine<'a> {
    v: &'a Vocabulary,
    ids: BTreeMap<&'a logos_canon::Text, usize>,
    tables: Vec<Option<Table>>,
    atoms: Vec<Atom>,
    guards: Vec<usize>,
    dag: Dag,
    cells: usize,
}
fn card(v: &Vocabulary, s: &Sort) -> Result<usize, GroundError> {
    match &v
        .sorts()
        .iter()
        .find(|d| d.name == s.name)
        .ok_or(GroundError::UnknownSort)?
        .domain
    {
        SortDomain::Enumerated { constants, .. } => Ok(constants.len()),
        SortDomain::Open => Err(GroundError::OpenDomain),
    }
}
fn product(cards: &[usize], max: usize) -> Result<usize, GroundError> {
    let n = cards
        .iter()
        .try_fold(1usize, |n, c| n.checked_mul(*c).ok_or_else(size))?;
    if n > max {
        return Err(size());
    }
    Ok(n)
}
fn tuple(mut i: usize, cards: &[usize]) -> Vec<u32> {
    let mut out = vec![0; cards.len()];
    for (value, card) in out.iter_mut().zip(cards).rev() {
        *value = (i % card) as u32;
        i /= card;
    }
    out
}
fn table_cells(rows: usize, outputs: usize, arity: usize) -> Result<usize, GroundError> {
    arity
        .checked_add(1)
        .and_then(|width| width.checked_mul(outputs.max(1)))
        .and_then(|width| width.checked_mul(rows))
        .ok_or_else(size)
}
fn children(n: &F) -> usize {
    match n {
        F::Not | F::Forall(_) | F::Exists(_) => 1,
        F::And(k) | F::Or(k) => *k,
        F::Implies | F::Iff => 2,
        _ => 0,
    }
}
enum Job {
    Visit(usize),
    Reduce(u8, usize),
    Bind(u32),
    Unbind,
}
impl<'a> Engine<'a> {
    fn new(v: &'a Vocabulary, limits: GroundLimits) -> Result<Self, GroundError> {
        let ids = v
            .symbols()
            .iter()
            .enumerate()
            .map(|(i, d)| (&d.signature.name, i))
            .collect();
        let mut tables = space(v.symbols().len(), limits.analysis.max_declarations)?;
        tables.resize_with(v.symbols().len(), || None);
        Ok(Self {
            v,
            ids,
            tables,
            atoms: Vec::new(),
            guards: Vec::new(),
            dag: Dag::new(limits)?,
            cells: 0,
        })
    }
    fn allocate(&mut self, n: usize) -> Result<(), GroundError> {
        self.cells = self.cells.checked_add(n).ok_or_else(size)?;
        if self.cells > self.dag.limits.max_cells {
            return Err(size());
        }
        self.dag.charge(n)
    }
    fn id(&self, s: &Symbol) -> Result<usize, GroundError> {
        self.ids
            .get(&s.name)
            .copied()
            .ok_or(GroundError::InvalidContext)
    }
    fn ordinal(&self, id: usize) -> Option<usize> {
        let sig = &self.v.symbols()[id].signature;
        self.v.sorts().iter().find_map(|d| match &d.domain {
            SortDomain::Enumerated { constants, .. } => constants.iter().position(|c| c == sig),
            _ => None,
        })
    }
    fn cards(&self, id: usize) -> Result<Vec<usize>, GroundError> {
        self.v.symbols()[id]
            .signature
            .arity
            .iter()
            .map(|s| card(self.v, s))
            .collect()
    }
    fn tables(&mut self) -> Result<(), GroundError> {
        for s in self.v.sorts() {
            if matches!(&s.domain,SortDomain::Enumerated{constants,..} if constants.is_empty()) {
                push(&mut self.guards, 0)?;
            }
        }
        for id in 0..self.v.symbols().len() {
            if matches!(self.v.symbols()[id].grounding, Grounding::Defined(_)) {
                continue;
            }
            let cards = match self.cards(id) {
                Ok(c) => c,
                Err(GroundError::OpenDomain) => continue,
                Err(e) => return Err(e),
            };
            let count = product(&cards, self.dag.limits.max_cells)?;
            let result = &self.v.symbols()[id].signature.result;
            let outputs = match result {
                SymbolResult::Bool => 1,
                SymbolResult::Sort(s) => card(self.v, s)?,
            };
            self.allocate(table_cells(count, outputs, cards.len())?)?;
            let mut rows = space(count, self.dag.limits.max_cells)?;
            for i in 0..count {
                self.dag.charge(1)?;
                let mut row = space(outputs, self.dag.limits.max_cells)?;
                let fixed = self.ordinal(id);
                for r in 0..outputs {
                    let value = if let Some(ordinal) = fixed {
                        usize::from(r == ordinal)
                    } else if matches!(result, SymbolResult::Sort(_)) && outputs == 1 {
                        1
                    } else {
                        if self.atoms.len() >= self.dag.limits.max_atoms {
                            return Err(size());
                        }
                        let atom = self.atoms.len();
                        push(
                            &mut self.atoms,
                            Atom {
                                symbol: u32::try_from(id).map_err(|_| size())?,
                                arguments: tuple(i, &cards),
                                output: if matches!(result, SymbolResult::Bool) {
                                    None
                                } else {
                                    Some(r as u32)
                                },
                            },
                        )?;
                        self.dag.node(PropNode::Atom(atom))?
                    };
                    row.push(value);
                }
                if matches!(result, SymbolResult::Sort(_)) {
                    let guard = self.dag.unique(&row)?;
                    push(&mut self.guards, guard)?;
                }
                rows.push(row);
            }
            self.tables[id] = Some(Table { cards, rows });
        }
        for &id in self.v.definition_order() {
            let Grounding::Defined(body) = &self.v.symbols()[id].grounding else {
                return Err(GroundError::InvalidContext);
            };
            let cards = self.cards(id)?;
            let count = product(&cards, self.dag.limits.max_cells)?;
            let result = &self.v.symbols()[id].signature.result;
            let outputs = match result {
                SymbolResult::Bool => 1,
                SymbolResult::Sort(s) => card(self.v, s)?,
            };
            self.allocate(table_cells(count, outputs, cards.len())?)?;
            let mut rows = space(count, self.dag.limits.max_cells)?;
            for i in 0..count {
                let args = tuple(i, &cards);
                let mut row = space(outputs, self.dag.limits.max_cells)?;
                for r in 0..outputs {
                    let mut env = args.iter().rev().copied().collect::<Vec<_>>();
                    if matches!(result, SymbolResult::Sort(_)) {
                        push(&mut env, r as u32)?;
                    }
                    row.push(self.formula(body, env)?);
                }
                if matches!(result, SymbolResult::Sort(_)) {
                    let guard = self.dag.unique(&row)?;
                    push(&mut self.guards, guard)?;
                    if let Some(r) = self.ordinal(id) {
                        push(
                            &mut self.guards,
                            *row.get(r).ok_or(GroundError::InvalidContext)?,
                        )?;
                    }
                }
                rows.push(row);
            }
            self.tables[id] = Some(Table { cards, rows });
        }
        Ok(())
    }
    fn apply(&mut self, id: usize, args: Vec<Vec<usize>>) -> Result<Vec<usize>, GroundError> {
        let t = self
            .tables
            .get(id)
            .and_then(Option::as_ref)
            .ok_or(GroundError::InvalidContext)?;
        let width = match &self.v.symbols()[id].signature.result {
            SymbolResult::Bool => 1,
            SymbolResult::Sort(s) => card(self.v, s)?,
        };
        let mut options = space::<Vec<usize>>(width, self.dag.limits.max_cells)?;
        options.resize_with(width, Vec::new);
        for (i, row) in t.rows.iter().enumerate() {
            self.dag
                .charge(t.cards.len().checked_add(width).ok_or_else(size)?)?;
            let inputs = tuple(i, &t.cards);
            let condition = inputs
                .iter()
                .enumerate()
                .map(|(j, k)| {
                    args.get(j)
                        .and_then(|a| a.get(*k as usize))
                        .copied()
                        .ok_or(GroundError::InvalidContext)
                })
                .collect::<Result<Vec<_>, _>>()?;
            for (r, out) in options.iter_mut().enumerate() {
                let mut factors = condition.clone();
                push(&mut factors, row[r])?;
                push(out, self.dag.and(factors)?)?;
            }
        }
        options.into_iter().map(|xs| self.dag.or(xs)).collect()
    }
    fn term(&mut self, t: &Term, env: &[u32]) -> Result<Vec<usize>, GroundError> {
        let mut values: Vec<Vec<usize>> = Vec::new();
        for n in t.nodes().iter().rev() {
            self.dag.charge(1)?;
            let value = match n {
                TermNode::Var { index, sort } => {
                    let i = usize::try_from(*index).map_err(|_| GroundError::InvalidContext)?;
                    let x = *env
                        .get(
                            env.len()
                                .checked_sub(i + 1)
                                .ok_or(GroundError::InvalidContext)?,
                        )
                        .ok_or(GroundError::InvalidContext)? as usize;
                    let n = card(self.v, sort)?;
                    self.dag.charge(n)?;
                    let mut v = space(n, self.dag.limits.max_cells)?;
                    v.extend((0..n).map(|k| usize::from(k == x)));
                    v
                }
                TermNode::App { symbol, args } => {
                    let mut inputs = space(*args, self.dag.limits.max_cells)?;
                    for _ in 0..*args {
                        inputs.push(values.pop().ok_or(GroundError::InvalidContext)?);
                    }
                    self.apply(self.id(symbol)?, inputs)?
                }
            };
            push(&mut values, value)?;
        }
        values.pop().ok_or(GroundError::InvalidContext)
    }
    fn formula(&mut self, f: &Formula, mut env: Vec<u32>) -> Result<usize, GroundError> {
        self.dag.charge(f.nodes().len())?;
        let mut ends = space(f.nodes().len(), self.dag.limits.max_cells)?;
        ends.resize(f.nodes().len(), 0);
        let mut stack = Vec::new();
        for (i, n) in f.nodes().iter().enumerate().rev() {
            let mut end = i + 1;
            for _ in 0..children(n) {
                end = stack.pop().ok_or(GroundError::InvalidContext)?;
            }
            ends[i] = end;
            push(&mut stack, end)?;
        }
        let mut jobs = vec![Job::Visit(0)];
        let mut values = Vec::new();
        while let Some(job) = jobs.pop() {
            self.dag.charge(1)?;
            match job {
                Job::Bind(x) => push(&mut env, x)?,
                Job::Unbind => {
                    env.pop().ok_or(GroundError::InvalidContext)?;
                }
                Job::Reduce(op, k) => {
                    let start = values
                        .len()
                        .checked_sub(k)
                        .ok_or(GroundError::InvalidContext)?;
                    let xs = values.split_off(start);
                    let n = match op {
                        0 => self.dag.node(PropNode::Not(xs[0]))?,
                        1 => self.dag.and(xs)?,
                        2 => self.dag.or(xs)?,
                        3 => self.dag.implies(xs[0], xs[1])?,
                        _ => {
                            let a = self.dag.implies(xs[0], xs[1])?;
                            let b = self.dag.implies(xs[1], xs[0])?;
                            self.dag.and(vec![a, b])?
                        }
                    };
                    push(&mut values, n)?;
                }
                Job::Visit(i) => match &f.nodes()[i] {
                    F::True => push(&mut values, 1)?,
                    F::False => push(&mut values, 0)?,
                    F::Eq(a, b) => {
                        let a = self.term(a, &env)?;
                        let b = self.term(b, &env)?;
                        let mut xs = space(a.len(), self.dag.limits.max_cells)?;
                        for (a, b) in a.into_iter().zip(b) {
                            xs.push(self.dag.and(vec![a, b])?);
                        }
                        let n = self.dag.or(xs)?;
                        push(&mut values, n)?;
                    }
                    F::Pred { symbol, args } => {
                        let mut inputs = space(args.len(), self.dag.limits.max_cells)?;
                        for t in args {
                            inputs.push(self.term(t, &env)?);
                        }
                        let out = self.apply(self.id(symbol)?, inputs)?;
                        push(
                            &mut values,
                            *out.first().ok_or(GroundError::InvalidContext)?,
                        )?;
                    }
                    F::Forall(s) | F::Exists(s) => {
                        let n = card(self.v, s)?;
                        let scheduled = n.checked_mul(3).ok_or_else(size)?;
                        self.dag.charge(scheduled)?;
                        if scheduled > self.dag.limits.max_cells {
                            return Err(size());
                        }
                        push(
                            &mut jobs,
                            Job::Reduce(
                                if matches!(&f.nodes()[i], F::Forall(_)) {
                                    1
                                } else {
                                    2
                                },
                                n,
                            ),
                        )?;
                        for x in (0..n).rev() {
                            push(&mut jobs, Job::Unbind)?;
                            push(&mut jobs, Job::Visit(i + 1))?;
                            push(&mut jobs, Job::Bind(x as u32))?;
                        }
                    }
                    node => {
                        let k = children(node);
                        let mut ids = space(k, self.dag.limits.max_cells)?;
                        let mut next = i + 1;
                        for _ in 0..k {
                            ids.push(next);
                            next = ends[next];
                        }
                        let op = match node {
                            F::Not => 0,
                            F::And(_) => 1,
                            F::Or(_) => 2,
                            F::Implies => 3,
                            F::Iff => 4,
                            _ => return Err(GroundError::InvalidContext),
                        };
                        push(&mut jobs, Job::Reduce(op, k))?;
                        for c in ids.into_iter().rev() {
                            push(&mut jobs, Job::Visit(c))?;
                        }
                    }
                },
            }
        }
        values.pop().ok_or(GroundError::InvalidContext)
    }
}

/// Traduz consequência finita com axiomas explícitos, sem autorização.
/// # Errors
/// General, identidade/tipagem, axiomas ausentes e limites de recursos.
pub fn ground(o: &Obligation, v: &Vocabulary) -> Result<GroundProgram, GroundError> {
    ground_with_limits(o, v, GroundLimits::default())
}
/// Variante com limites lógicos inclusivos.
/// # Errors
/// Os de ground; nunca devolve programa parcial.
pub fn ground_with_limits(
    o: &Obligation,
    v: &Vocabulary,
    limits: GroundLimits,
) -> Result<GroundProgram, GroundError> {
    ground_parts(
        || Ok(hash(o)?),
        o.vocab(),
        o.premises(),
        o.goal(),
        v,
        limits,
    )
}

/// Reproduz grounding de bytes/hash de O, sem construir uma obrigação autoritativa.
/// # Errors
/// Erros de decode_for_check, contexto/axiomas/fragmento e limites inclusivos.
pub fn ground_for_check(
    bytes: &[u8],
    expected: Hash<Obligation>,
    v: &Vocabulary,
    decode: DecodeLimits,
    limits: GroundLimits,
) -> Result<GroundProgram, GroundError> {
    let o = decode_for_check(bytes, expected, v, decode)?;
    ground_parts(
        || Ok(expected),
        o.vocab(),
        o.premises(),
        o.goal(),
        v,
        limits,
    )
}

fn ground_parts(
    identify: impl FnOnce() -> Result<Hash<Obligation>, GroundError>,
    vocabulary: Hash<Vocabulary>,
    source: &[(PremiseRef, Formula)],
    goal_formula: &Formula,
    v: &Vocabulary,
    limits: GroundLimits,
) -> Result<GroundProgram, GroundError> {
    if crate::classify::classify_parts(v, vocabulary, source, goal_formula, limits.analysis)?
        != Fragment::FiniteDomain
    {
        return Err(GroundError::GeneralFragment);
    }
    let obligation = identify()?;
    let mut e = Engine::new(v, limits)?;
    let mut references = Vec::new();
    let mut sort_support = Vec::new();
    for sort in v.sorts() {
        if matches!(sort.domain, SortDomain::Enumerated { .. }) {
            let c = domain_axioms(
                v,
                &Sort {
                    name: sort.name.clone(),
                },
            )?;
            let start = references.len();
            // 6.1b: uma premissa com a conjunção (fato de fonte) cobre o sort inteiro.
            let conjunction = c.conjunction()?;
            let mut whole = None;
            for (r, p) in source {
                e.dag.charge(conjunction.node_count())?;
                if *p == conjunction {
                    whole = Some(*r);
                    break;
                }
            }
            if let Some(r) = whole {
                push(&mut references, r)?;
            } else {
                for f in std::iter::once(c.closure()).chain(c.distinct()) {
                    let mut found = None;
                    for (r, p) in source {
                        e.dag.charge(f.node_count())?;
                        if p == f {
                            found = Some(*r);
                            break;
                        }
                    }
                    push(
                        &mut references,
                        found.ok_or(GroundError::MissingDomainAxiom)?,
                    )?;
                }
            }
            let mut refs = space(references.len() - start, limits.max_cells)?;
            refs.extend_from_slice(&references[start..]);
            push(&mut sort_support, (sort.name.clone(), refs))?;
        }
    }
    e.tables()?;
    let constraints = e.dag.and(std::mem::take(&mut e.guards))?;
    let mut premises = Vec::new();
    let mut premise_support = Vec::new();
    let mut support_count = 0usize;
    let mut antecedents = vec![constraints];
    for (r, f) in source {
        let n = e.formula(f, Vec::new())?;
        let support = crate::support::formula_support(v, f, &sort_support, &mut e.dag)?;
        support_count = support_count.checked_add(support.len()).ok_or_else(size)?;
        if support_count > limits.max_cells {
            return Err(size());
        }
        push(&mut premise_support, support)?;
        push(&mut premises, (*r, n))?;
        push(&mut antecedents, n)?;
    }
    let goal = e.formula(goal_formula, Vec::new())?;
    let goal_support = crate::support::formula_support(v, goal_formula, &sort_support, &mut e.dag)?;
    if support_count
        .checked_add(goal_support.len())
        .ok_or_else(size)?
        > limits.max_cells
    {
        return Err(size());
    }
    let antecedent = e.dag.and(antecedents)?;
    let root = e.dag.implies(antecedent, goal)?;
    let p = GroundProgram {
        obligation,
        vocabulary,
        atoms: e.atoms,
        nodes: e.dag.nodes,
        constraints,
        premises,
        goal,
        root,
        domain_axioms: references,
        premise_support,
        goal_support,
    };
    if to_canon(&p)?.len() > limits.max_bytes {
        return Err(size());
    }
    Ok(p)
}
