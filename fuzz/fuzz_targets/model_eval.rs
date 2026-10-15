//! Avaliação de AIR arbitrário e corpus estruturado com Defined e orçamento.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_air::{
    Formula, FormulaNode, Grounding, Limits, Sort, SortDecl, SortDomain, Symbol, SymbolDecl,
    SymbolKind, SymbolResult, Term, TermNode, Vocabulary,
};
use logos_canon::{Text, from_canon, to_canon};
use logos_core::{Budget, CheckOutcome, Exhausted};
use logos_crypto::{Hash, hash, hash_raw};
use logos_model::{EvalError, FiniteModel, ModelChecker, ModelData, ModelLimits, eval};
use std::sync::LazyLock;

fn sort() -> Sort {
    Sort {
        name: Text::new("S").expect("sort"),
    }
}

fn var(index: u64) -> Term {
    Term::new(
        vec![TermNode::Var {
            index,
            sort: sort(),
        }],
        Limits::default(),
    )
    .expect("var")
}

fn app(symbol: &Symbol, index: u64) -> Term {
    Term::new(
        vec![
            TermNode::App {
                symbol: symbol.clone(),
                args: 1,
            },
            TermNode::Var {
                index,
                sort: sort(),
            },
        ],
        Limits::default(),
    )
    .expect("app")
}

static VOCAB: LazyLock<Vocabulary> = LazyLock::new(|| {
    let f = Symbol {
        name: Text::new("f").expect("name"),
        kind: SymbolKind::Func,
        arity: vec![sort()],
        result: SymbolResult::Sort(sort()),
    };
    let d = Symbol {
        name: Text::new("D").expect("name"),
        kind: SymbolKind::Pred,
        arity: vec![sort()],
        result: SymbolResult::Bool,
    };
    let g = Symbol {
        name: Text::new("g").expect("name"),
        ..f.clone()
    };
    let relation = Formula::new(vec![FormulaNode::Eq(var(0), app(&f, 1))], Limits::default())
        .expect("relation");
    let predicate = Formula::new(vec![FormulaNode::Eq(var(0), app(&f, 0))], Limits::default())
        .expect("predicate");
    Vocabulary::new(
        1,
        vec![SortDecl {
            name: sort().name,
            domain: SortDomain::Open,
        }],
        vec![
            SymbolDecl {
                signature: f,
                grounding: Grounding::Glossed(Text::new("primitive").expect("text")),
                gloss_pt: Text::new("").expect("text"),
            },
            SymbolDecl {
                signature: d,
                grounding: Grounding::Defined(predicate),
                gloss_pt: Text::new("").expect("text"),
            },
            SymbolDecl {
                signature: g,
                grounding: Grounding::Defined(relation),
                gloss_pt: Text::new("").expect("text"),
            },
        ],
    )
    .expect("vocab")
});

struct EmptyView;
impl logos_obligation::LedgerView for EmptyView {
    fn current_epoch(&self) -> u64 {
        0
    }
    fn lookup(
        &self,
        _: logos_obligation::PremiseRef,
    ) -> Result<Option<logos_obligation::LedgerEntry<'_>>, logos_obligation::LedgerError> {
        Ok(None)
    }
}

fn probe(model: &FiniteModel, formula: &Formula, steps: u64) {
    // Exercita também a fonte de bytes não confiável do checker (ADR0025).
    let obligation = logos_obligation::generate(
        logos_obligation::Proposal {
            vocabulary: &VOCAB,
            goal: formula,
            cites: &[],
        },
        &EmptyView,
    )
    .expect("generated obligation");
    let wire = to_canon(&obligation).expect("O");
    let h = obligation.hash().expect("hO");
    let model_bytes = to_canon(model).expect("M");
    let mh = hash_raw::<FiniteModel>(&model_bytes);
    let checker = ModelChecker::new(&VOCAB).expect("checker");
    let source_budget = Budget::new(steps * 4, 0, 1_000_000, 0);
    let run = |expected| {
        checker
            .check_refutation_from_source(
                &wire,
                h,
                |_: &Hash<FiniteModel>| Ok::<_, ()>(Some(model_bytes.clone())),
                expected,
                &source_budget,
            )
            .expect("source")
    };
    assert_eq!(run(mh), run(mh));
    let mut bad = *mh.as_bytes();
    bad[0] ^= 1;
    assert_ne!(run(Hash::from_bytes(bad)), CheckOutcome::Accepted);
    let budget = Budget::new(steps, 0, 0, 0);
    let first = eval(model, &VOCAB, formula, &budget);
    assert_eq!(first, eval(model, &VOCAB, formula, &budget));
    assert_eq!(
        eval(model, &VOCAB, formula, &Budget::new(0, 0, 0, 0)),
        Err(EvalError::Unknown(Exhausted::Steps))
    );
    if let Ok(value) = first {
        let mut nodes = vec![FormulaNode::Not];
        nodes.extend_from_slice(formula.nodes());
        let limits = Limits {
            max_depth: formula.depth() + 1,
            max_nodes: formula.node_count() + 1,
        };
        let negated = Formula::new(nodes, limits).expect("negated");
        if let Ok(opposite) = eval(model, &VOCAB, &negated, &budget) {
            assert_eq!(opposite, !value);
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let byte = |index: usize| data.get(index).copied().unwrap_or(0);
    let n = u32::from(byte(0) % 8) + 1;
    let model = FiniteModel::new(
        ModelData {
            vocab: hash(&*VOCAB).expect("hash"),
            domains: vec![n],
            consts: vec![],
            funcs: vec![(
                0,
                (0..n)
                    .map(|i| u32::from(byte(i as usize + 1)) % n)
                    .collect(),
            )],
            preds: vec![],
        },
        &VOCAB,
        ModelLimits::default(),
    )
    .expect("model");
    let mut nodes = Vec::new();
    let mut binders = 0;
    for i in 0..usize::from(byte(9) % 16 + 1) {
        nodes.push(match byte(10 + i) % 3 {
            0 => {
                binders += 1;
                FormulaNode::Forall(sort())
            }
            1 => {
                binders += 1;
                FormulaNode::Exists(sort())
            }
            _ => FormulaNode::Not,
        });
    }
    if binders == 0 {
        nodes.push(FormulaNode::Forall(sort()));
        binders = 1;
    }
    let index = u64::from(byte(30)) % binders;
    nodes.push(if byte(31) % 2 == 0 {
        FormulaNode::Pred {
            symbol: VOCAB.symbols()[1].signature.clone(),
            args: vec![app(&VOCAB.symbols()[2].signature, index)],
        }
    } else {
        FormulaNode::Eq(
            app(&VOCAB.symbols()[2].signature, index),
            app(&VOCAB.symbols()[0].signature, index),
        )
    });
    let formula = Formula::new(nodes, Limits::default()).expect("formula");
    probe(&model, &formula, u64::from(byte(32)) * 8);
    if data.len() <= 4096 {
        if let Ok(arbitrary) = from_canon::<Formula>(data) {
            probe(&model, &arbitrary, 1024);
        }
    }
});
