#![allow(dead_code)]
use logos_air::{
    DomainSource, FactRef, Formula, FormulaNode as F, Grounding, Limits, Sort, SortDecl,
    SortDomain, Symbol, SymbolDecl, SymbolKind, SymbolResult, Term, TermNode, Vocabulary,
};
use logos_canon::Text;
use logos_core::Budget;
use logos_ground::{GroundProgram, PropNode, domain_axioms};
use logos_obligation::{
    EntryKind, LedgerEntry, LedgerError, LedgerView, Obligation, PremiseRef, Proposal, generate,
};
pub fn text(s: &str) -> Text {
    Text::new(s).unwrap()
}
pub fn sort(s: &str) -> Sort {
    Sort { name: text(s) }
}
pub fn f(ns: Vec<F>) -> Formula {
    Formula::new(ns, Limits::default()).unwrap()
}
pub fn c(s: &str) -> Symbol {
    Symbol {
        name: text(s),
        kind: SymbolKind::Const,
        arity: vec![],
        result: SymbolResult::Sort(sort("E")),
    }
}
pub fn p(s: &str) -> Symbol {
    Symbol {
        name: text(s),
        kind: SymbolKind::Pred,
        arity: vec![sort("E")],
        result: SymbolResult::Bool,
    }
}
pub fn func(s: &str) -> Symbol {
    Symbol {
        name: text(s),
        kind: SymbolKind::Func,
        arity: vec![sort("E")],
        result: SymbolResult::Sort(sort("E")),
    }
}
pub fn prim(s: Symbol) -> SymbolDecl {
    SymbolDecl {
        signature: s,
        grounding: Grounding::Glossed(text("test")),
        gloss_pt: text("test"),
    }
}
pub fn def(s: Symbol, b: Formula) -> SymbolDecl {
    SymbolDecl {
        signature: s,
        grounding: Grounding::Defined(b),
        gloss_pt: text("test"),
    }
}
pub fn t(s: Symbol, args: Vec<Term>) -> Term {
    let mut ns = vec![TermNode::App {
        symbol: s,
        args: args.len(),
    }];
    for a in args {
        ns.extend_from_slice(a.nodes());
    }
    Term::new(ns, Limits::default()).unwrap()
}
pub fn var(i: u64) -> Term {
    Term::new(
        vec![TermNode::Var {
            index: i,
            sort: sort("E"),
        }],
        Limits::default(),
    )
    .unwrap()
}
pub fn pred(s: Symbol, args: Vec<Term>) -> Formula {
    f(vec![F::Pred { symbol: s, args }])
}
pub fn eq(a: Term, b: Term) -> Formula {
    f(vec![F::Eq(a, b)])
}
pub fn combine(n: F, fs: &[Formula]) -> Formula {
    let mut ns = vec![n];
    for f in fs {
        ns.extend_from_slice(f.nodes());
    }
    f(ns)
}
pub fn vocab(n: usize, extras: Vec<SymbolDecl>) -> Vocabulary {
    let cs = (0..n).map(|i| c(&format!("e{i}"))).collect::<Vec<_>>();
    let mut ss = cs.iter().cloned().map(prim).collect::<Vec<_>>();
    ss.extend(extras);
    Vocabulary::new(
        1,
        vec![SortDecl {
            name: text("E"),
            domain: SortDomain::Enumerated {
                constants: cs,
                domain_source: DomainSource::Fact(FactRef([9; 32])),
            },
        }],
        ss,
    )
    .unwrap()
}
pub fn axioms(v: &Vocabulary) -> Vec<Formula> {
    let mut fs = Vec::new();
    for s in v.sorts() {
        if matches!(s.domain, SortDomain::Enumerated { .. }) {
            let a = domain_axioms(
                v,
                &Sort {
                    name: s.name.clone(),
                },
            )
            .unwrap();
            fs.push(a.closure().clone());
            fs.extend_from_slice(a.distinct());
        }
    }
    fs
}
pub struct View<'a> {
    pub v: &'a Vocabulary,
    pub fs: &'a [Formula],
}
impl LedgerView for View<'_> {
    fn current_epoch(&self) -> u64 {
        0
    }
    fn lookup(&self, r: PremiseRef) -> Result<Option<LedgerEntry<'_>>, LedgerError> {
        let i = u32::from_be_bytes(r.event_hash()[..4].try_into().unwrap()) as usize;
        Ok(self.fs.get(i).map(|formula| LedgerEntry {
            kind: EntryKind::Assumption,
            epoch: 0,
            retracted: false,
            vocabulary: self.v,
            formula,
        }))
    }
}
pub fn obligation(v: &Vocabulary, goal: &Formula, fs: &[Formula]) -> Obligation {
    let refs = (0..fs.len())
        .map(|i| {
            let mut r = [0; 32];
            r[..4].copy_from_slice(&(i as u32).to_be_bytes());
            PremiseRef::from_event_hash(r)
        })
        .collect::<Vec<_>>();
    generate(
        Proposal {
            vocabulary: v,
            goal,
            cites: &refs,
        },
        &View { v, fs },
    )
    .unwrap()
}
pub fn closed(v: &Vocabulary, goal: &Formula, extra: &[Formula]) -> Obligation {
    let mut fs = axioms(v);
    fs.extend_from_slice(extra);
    obligation(v, goal, &fs)
}
pub fn budget() -> Budget {
    Budget::new(10_000_000, 0, 0, 0)
}
pub fn prop(g: &GroundProgram, bits: usize, root: usize) -> bool {
    let mut values: Vec<bool> = Vec::with_capacity(g.nodes().len());
    for (i, n) in g.nodes().iter().enumerate() {
        let get = |j: usize| {
            assert!(j < i);
            values[j]
        };
        let b = match n {
            PropNode::False => false,
            PropNode::True => true,
            PropNode::Atom(a) => (bits >> a) & 1 != 0,
            PropNode::Not(a) => !get(*a),
            PropNode::And(xs) => xs.iter().all(|x| get(*x)),
            PropNode::Or(xs) => xs.iter().any(|x| get(*x)),
        };
        values.push(b);
    }
    values[root]
}
pub fn tautology(g: &GroundProgram) -> bool {
    assert!(g.atoms().len() <= 8);
    let count = 1usize << g.atoms().len();
    let words = count.div_ceil(64);
    // Tabela verdade integral em blocos de 64 valorações independentes.
    let mut values: Vec<[u64; 4]> = Vec::with_capacity(g.nodes().len());
    for (i, n) in g.nodes().iter().enumerate() {
        let get = |j: usize, w: usize| {
            assert!(j < i);
            values[j][w]
        };
        let mut row = [0; 4];
        for (w, bits) in row.iter_mut().enumerate().take(words) {
            *bits = match n {
                PropNode::False => 0,
                PropNode::True => u64::MAX,
                PropNode::Atom(a) => (0..64).fold(0, |bits, k| {
                    bits | (((((w * 64 + k) >> a) & 1) as u64) << k)
                }),
                PropNode::Not(a) => !get(*a, w),
                PropNode::And(xs) => xs.iter().fold(u64::MAX, |v, x| v & get(*x, w)),
                PropNode::Or(xs) => xs.iter().fold(0, |v, x| v | get(*x, w)),
            };
        }
        values.push(row);
    }
    (0..words).all(|w| {
        let mask = if count >= 64 {
            u64::MAX
        } else {
            (1u64 << count) - 1
        };
        values[g.root()][w] & mask == mask
    })
}
