//! Interpretador integral: uma pilha de continuções, valores e ambientes.

use crate::{EvalError, FiniteModel};
use logos_air::{
    Formula, FormulaNode, Grounding, Sort, SortDomain, SymbolKind, SymbolResult, Term, TermNode,
    Vocabulary,
};
use logos_core::{Exhausted, StepMeter};

fn push<T>(stack: &mut Vec<T>, value: T) -> Result<(), EvalError> {
    if stack.len() == stack.capacity() {
        stack.try_reserve(1).map_err(|_| Exhausted::Memory)?;
    }
    stack.push(value);
    Ok(())
}

#[derive(Clone, Copy)]
enum Nodes<'a> {
    Formula(&'a [FormulaNode]),
    Term(&'a [TermNode]),
}
impl<'a> Nodes<'a> {
    fn job(self, pc: usize) -> Job<'a> {
        match self {
            Self::Formula(nodes) => Job::Formula(nodes, pc),
            Self::Term(nodes) => Job::Term(nodes, pc),
        }
    }
}

#[derive(Clone, Copy)]
enum Op {
    Not,
    And,
    Or,
    Implies,
    Iff,
    Eq,
    Apply(usize),
}

enum Job<'a> {
    Formula(&'a [FormulaNode], usize),
    Term(&'a [TermNode], usize),
    Children {
        nodes: Nodes<'a>,
        remaining: usize,
        op: Op,
        base: usize,
    },
    Finish {
        op: Op,
        base: usize,
        end: usize,
    },
    Args {
        id: usize,
        terms: &'a [Term],
        next: usize,
        base: usize,
        end: usize,
    },
    Quantifier {
        nodes: &'a [FormulaNode],
        body: usize,
        current: u32,
        size: u32,
        universal: bool,
        acc: bool,
        env: usize,
    },
    Restore {
        env: usize,
        end: usize,
    },
    Candidate {
        id: usize,
        body: &'a Formula,
        current: u32,
        size: u32,
        matches: u32,
        value: u32,
        env: usize,
        end: usize,
    },
}

struct Machine<'a> {
    model: &'a FiniteModel,
    vocabulary: &'a Vocabulary,
    meter: &'a StepMeter,
    jobs: Vec<Job<'a>>,
    values: Vec<u32>, // Bool is 0/1; sort-check fixes the category of each slot.
    env: Vec<u32>,    // Outer → inner, with isolated Defined parameters at the tail.
    end: usize,       // Cursor immediately after the subtree just returned.
}

impl<'a> Machine<'a> {
    fn new(model: &'a FiniteModel, vocabulary: &'a Vocabulary, meter: &'a StepMeter) -> Self {
        Self {
            model,
            vocabulary,
            meter,
            jobs: Vec::new(),
            values: Vec::new(),
            env: Vec::new(),
            end: 0,
        }
    }

    fn domain(&self, sort: &Sort) -> Result<u32, EvalError> {
        let index = self
            .vocabulary
            .sorts()
            .iter()
            .position(|s| s.name == sort.name)
            .ok_or(EvalError::VocabularyMismatch)?;
        Ok(self.model.data().domains[index])
    }

    fn symbol(&self, name: &logos_canon::Text) -> Result<usize, EvalError> {
        self.vocabulary
            .symbols()
            .iter()
            .position(|s| &s.signature.name == name)
            .ok_or(EvalError::VocabularyMismatch)
    }

    fn children(
        &mut self,
        nodes: Nodes<'a>,
        pc: usize,
        count: usize,
        op: Op,
    ) -> Result<(), EvalError> {
        let base = self.values.len();
        self.end = pc;
        if count == 0 {
            return self.finish(op, base);
        }
        push(
            &mut self.jobs,
            Job::Children {
                nodes,
                remaining: count - 1,
                op,
                base,
            },
        )?;
        push(&mut self.jobs, nodes.job(pc))
    }

    fn finish(&mut self, op: Op, base: usize) -> Result<(), EvalError> {
        if let Op::Apply(id) = op {
            return self.apply(id, base);
        }
        let args = &self.values[base..];
        let result = match op {
            Op::Not => args[0] == 0,
            Op::And => args.iter().all(|&v| v != 0),
            Op::Or => args.iter().any(|&v| v != 0),
            Op::Implies => args[0] == 0 || args[1] != 0,
            Op::Iff | Op::Eq => args[0] == args[1],
            Op::Apply(_) => return Err(EvalError::VocabularyMismatch),
        };
        self.values.truncate(base);
        push(&mut self.values, u32::from(result))
    }

    fn apply(&mut self, id: usize, base: usize) -> Result<(), EvalError> {
        let declaration = &self.vocabulary.symbols()[id];
        let symbol = u32::try_from(id).map_err(|_| Exhausted::Size)?;
        let signature = &declaration.signature;
        let args = &self.values[base..];
        if let Grounding::Defined(body) = &declaration.grounding {
            let before = self.env.len();
            let capacity = args.len().checked_add(1).ok_or(Exhausted::Memory)?;
            self.env
                .try_reserve(capacity)
                .map_err(|_| Exhausted::Memory)?;
            self.env.extend(args.iter().rev().copied());
            self.values.truncate(base);
            match &signature.result {
                SymbolResult::Bool => push(
                    &mut self.jobs,
                    Job::Restore {
                        env: before,
                        end: self.end,
                    },
                )?,
                SymbolResult::Sort(sort) => {
                    self.meter.charge(1)?;
                    self.env.push(0);
                    let size = self.domain(sort)?;
                    push(
                        &mut self.jobs,
                        Job::Candidate {
                            id,
                            body,
                            current: 0,
                            size,
                            matches: 0,
                            value: 0,
                            env: before,
                            end: self.end,
                        },
                    )?;
                }
            }
            return push(&mut self.jobs, Job::Formula(body.nodes(), 0));
        }
        let data = self.model.data();
        let value = match signature.kind {
            SymbolKind::Const => {
                let index = data
                    .consts
                    .binary_search_by_key(&symbol, |&(id, _)| id)
                    .map_err(|_| EvalError::VocabularyMismatch)?;
                data.consts[index].1
            }
            SymbolKind::Func => {
                let index = data
                    .funcs
                    .binary_search_by_key(&symbol, |(id, _)| *id)
                    .map_err(|_| EvalError::VocabularyMismatch)?;
                let mut cell = 0usize;
                for (sort, &arg) in signature.arity.iter().zip(args) {
                    cell = cell
                        .checked_mul(self.domain(sort)? as usize)
                        .and_then(|n| n.checked_add(arg as usize))
                        .ok_or(Exhausted::Size)?;
                }
                *data.funcs[index]
                    .1
                    .get(cell)
                    .ok_or(EvalError::VocabularyMismatch)?
            }
            SymbolKind::Pred => {
                let index = data
                    .preds
                    .binary_search_by_key(&symbol, |(id, _)| *id)
                    .map_err(|_| EvalError::VocabularyMismatch)?;
                u32::from(
                    data.preds[index]
                        .1
                        .binary_search_by(|tuple| tuple.as_slice().cmp(args))
                        .is_ok(),
                )
            }
        };
        self.values.truncate(base);
        push(&mut self.values, value)
    }

    fn drive(&mut self) -> Result<u32, EvalError> {
        while let Some(job) = self.jobs.pop() {
            match job {
                Job::Formula(nodes, pc) => {
                    self.meter.charge(1)?;
                    self.end = pc + 1;
                    match &nodes[pc] {
                        FormulaNode::True => push(&mut self.values, 1)?,
                        FormulaNode::False => push(&mut self.values, 0)?,
                        FormulaNode::Not => {
                            self.children(Nodes::Formula(nodes), pc + 1, 1, Op::Not)?
                        }
                        FormulaNode::And(n) => {
                            self.children(Nodes::Formula(nodes), pc + 1, *n, Op::And)?
                        }
                        FormulaNode::Or(n) => {
                            self.children(Nodes::Formula(nodes), pc + 1, *n, Op::Or)?
                        }
                        FormulaNode::Implies => {
                            self.children(Nodes::Formula(nodes), pc + 1, 2, Op::Implies)?
                        }
                        FormulaNode::Iff => {
                            self.children(Nodes::Formula(nodes), pc + 1, 2, Op::Iff)?
                        }
                        FormulaNode::Eq(left, right) => {
                            push(
                                &mut self.jobs,
                                Job::Finish {
                                    op: Op::Eq,
                                    base: self.values.len(),
                                    end: self.end,
                                },
                            )?;
                            push(&mut self.jobs, Job::Term(right.nodes(), 0))?;
                            push(&mut self.jobs, Job::Term(left.nodes(), 0))?;
                        }
                        FormulaNode::Pred { symbol, args } => {
                            let id = self.symbol(&symbol.name)?;
                            push(
                                &mut self.jobs,
                                Job::Args {
                                    id,
                                    terms: args,
                                    next: 0,
                                    base: self.values.len(),
                                    end: self.end,
                                },
                            )?;
                        }
                        FormulaNode::Forall(sort) | FormulaNode::Exists(sort) => {
                            self.meter.charge(1)?;
                            let universal = matches!(&nodes[pc], FormulaNode::Forall(_));
                            let size = self.domain(sort)?;
                            let env = self.env.len();
                            push(&mut self.env, 0)?;
                            push(
                                &mut self.jobs,
                                Job::Quantifier {
                                    nodes,
                                    body: pc + 1,
                                    current: 0,
                                    size,
                                    universal,
                                    acc: universal,
                                    env,
                                },
                            )?;
                            push(&mut self.jobs, Job::Formula(nodes, pc + 1))?;
                        }
                    }
                }
                Job::Term(nodes, pc) => {
                    self.meter.charge(1)?;
                    self.end = pc + 1;
                    match &nodes[pc] {
                        TermNode::Var { index, .. } => {
                            // Preflight guarantees index < active binders; Defined parameters
                            // were checked without caller binders, so cannot capture them.
                            let value = self.env[self.env.len() - 1 - *index as usize];
                            push(&mut self.values, value)?;
                        }
                        TermNode::App { symbol, args } => {
                            let id = self.symbol(&symbol.name)?;
                            self.children(Nodes::Term(nodes), pc + 1, *args, Op::Apply(id))?;
                        }
                    }
                }
                Job::Children {
                    nodes,
                    remaining,
                    op,
                    base,
                } => {
                    if remaining == 0 {
                        self.finish(op, base)?;
                    } else {
                        push(
                            &mut self.jobs,
                            Job::Children {
                                nodes,
                                remaining: remaining - 1,
                                op,
                                base,
                            },
                        )?;
                        push(&mut self.jobs, nodes.job(self.end))?;
                    }
                }
                Job::Finish { op, base, end } => {
                    self.end = end;
                    self.finish(op, base)?;
                }
                Job::Args {
                    id,
                    terms,
                    next,
                    base,
                    end,
                } => {
                    if next == terms.len() {
                        self.end = end;
                        self.apply(id, base)?;
                    } else {
                        push(
                            &mut self.jobs,
                            Job::Args {
                                id,
                                terms,
                                next: next + 1,
                                base,
                                end,
                            },
                        )?;
                        push(&mut self.jobs, Job::Term(terms[next].nodes(), 0))?;
                    }
                }
                Job::Quantifier {
                    nodes,
                    body,
                    current,
                    size,
                    universal,
                    acc,
                    env,
                } => {
                    let value = self.values.pop().ok_or(EvalError::VocabularyMismatch)? != 0;
                    let acc = if universal {
                        acc && value
                    } else {
                        acc || value
                    };
                    let current = current + 1;
                    if current == size {
                        self.env.truncate(env);
                        push(&mut self.values, u32::from(acc))?;
                    } else {
                        self.meter.charge(1)?;
                        self.env[env] = current;
                        push(
                            &mut self.jobs,
                            Job::Quantifier {
                                nodes,
                                body,
                                current,
                                size,
                                universal,
                                acc,
                                env,
                            },
                        )?;
                        push(&mut self.jobs, Job::Formula(nodes, body))?;
                    }
                }
                Job::Restore { env, end } => {
                    self.env.truncate(env);
                    self.end = end;
                }
                Job::Candidate {
                    id,
                    body,
                    current,
                    size,
                    mut matches,
                    mut value,
                    env,
                    end,
                } => {
                    if self.values.pop().ok_or(EvalError::VocabularyMismatch)? != 0 {
                        matches += 1;
                        value = current;
                    }
                    let current = current + 1;
                    if current == size {
                        if matches != 1 {
                            return Err(EvalError::InvalidDefinition {
                                symbol: u32::try_from(id).map_err(|_| Exhausted::Size)?,
                            });
                        }
                        self.env.truncate(env);
                        self.end = end;
                        push(&mut self.values, value)?;
                    } else {
                        self.meter.charge(1)?;
                        let last = self.env.len() - 1;
                        self.env[last] = current;
                        push(
                            &mut self.jobs,
                            Job::Candidate {
                                id,
                                body,
                                current,
                                size,
                                matches,
                                value,
                                env,
                                end,
                            },
                        )?;
                        push(&mut self.jobs, Job::Formula(body.nodes(), 0))?;
                    }
                }
            }
        }
        self.values.pop().ok_or(EvalError::VocabularyMismatch)
    }

    fn validate_definitions(&mut self) -> Result<(), EvalError> {
        let mut tuple = Vec::new();
        let mut cards = Vec::new();
        for (id, declaration) in self.vocabulary.symbols().iter().enumerate() {
            if !matches!(declaration.grounding, Grounding::Defined(_))
                || declaration.signature.kind == SymbolKind::Pred
            {
                continue;
            }
            let arity = &declaration.signature.arity;
            tuple.clear();
            cards.clear();
            tuple
                .try_reserve(arity.len())
                .map_err(|_| Exhausted::Memory)?;
            cards
                .try_reserve(arity.len())
                .map_err(|_| Exhausted::Memory)?;
            tuple.resize(arity.len(), 0);
            for sort in arity {
                cards.push(self.domain(sort)?);
            }
            let ordinal = self.vocabulary.sorts().iter().find_map(|sort| {
                if let SortDomain::Enumerated { constants, .. } = &sort.domain {
                    constants.iter().position(|s| s == &declaration.signature)
                } else {
                    None
                }
            });
            loop {
                self.meter.charge(1)?;
                self.values
                    .try_reserve(tuple.len())
                    .map_err(|_| Exhausted::Memory)?;
                self.values.extend_from_slice(&tuple);
                self.end = 0;
                self.apply(id, 0)?;
                let value = self.drive()?;
                if let Some(ordinal) = ordinal
                    && value as usize != ordinal
                {
                    return Err(EvalError::EnumerationMismatch {
                        symbol: u32::try_from(id).map_err(|_| Exhausted::Size)?,
                    });
                }
                // Odometer: final axis fastest; empty product has one tuple.
                let mut axis = tuple.len();
                while axis != 0 {
                    axis -= 1;
                    tuple[axis] += 1;
                    if tuple[axis] < cards[axis] {
                        break;
                    }
                    tuple[axis] = 0;
                }
                if axis == 0 && tuple.first().is_none_or(|&value| value == 0) {
                    break;
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn run(
    model: &FiniteModel,
    vocabulary: &Vocabulary,
    formula: &Formula,
    meter: &StepMeter,
) -> Result<bool, EvalError> {
    let mut machine = Machine::new(model, vocabulary, meter);
    machine.validate_definitions()?;
    push(&mut machine.jobs, Job::Formula(formula.nodes(), 0))?;
    machine.drive().map(|value| value != 0)
}
