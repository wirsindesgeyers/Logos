//! Testes de lógica e contexto; fontes são explicitamente opacas, sem claim.

use std::collections::BTreeSet;

use logos_air::{
    DomainSource, FactRef, Formula, FormulaNode as F, Grounding, Limits, PolicyRef, Sort, SortDecl,
    SortDomain, Symbol, SymbolDecl, SymbolKind, SymbolResult, Term, TermNode, Vocabulary,
};
use logos_canon::{Text, from_canon, to_canon};
use logos_core::{Budget, Exhausted};
use logos_crypto::hash;
use logos_ground::{
    AnalysisLimits, AxiomLimits, Fragment, GroundError, classify, classify_with_limits,
    domain_axioms, domain_axioms_with_limits,
};
use logos_model::{FiniteModel, ModelData, ModelLimits, eval};
use logos_obligation::{
    EntryKind, LedgerEntry, LedgerError, LedgerView, Obligation, PremiseRef, Proposal, generate,
};

fn text(s: &str) -> Text {
    Text::new(s).unwrap()
}
fn sort(s: &str) -> Sort {
    Sort { name: text(s) }
}
fn formula(nodes: Vec<F>) -> Formula {
    Formula::new(nodes, Limits::default()).unwrap()
}
fn constant(name: &str, result: &str) -> Symbol {
    Symbol {
        name: text(name),
        kind: SymbolKind::Const,
        arity: vec![],
        result: SymbolResult::Sort(sort(result)),
    }
}
fn pred(name: &str, arity: Vec<Sort>) -> Symbol {
    Symbol {
        name: text(name),
        kind: SymbolKind::Pred,
        arity,
        result: SymbolResult::Bool,
    }
}
fn primitive(signature: Symbol) -> SymbolDecl {
    SymbolDecl {
        signature,
        grounding: Grounding::Glossed(text("fixture")),
        gloss_pt: text("fixture"),
    }
}
fn defined(signature: Symbol, body: Formula) -> SymbolDecl {
    SymbolDecl {
        signature,
        grounding: Grounding::Defined(body),
        gloss_pt: text("fixture"),
    }
}
fn source(policy: bool) -> DomainSource {
    if policy {
        DomainSource::Policy(PolicyRef([120; 32]))
    } else {
        DomainSource::Fact(FactRef([119; 32]))
    }
}
fn vocabulary(n: usize, enumerated: bool, policy: bool) -> Vocabulary {
    let constants = (0..n)
        .map(|i| constant(&format!("c{i}"), "E"))
        .collect::<Vec<_>>();
    let domain = if enumerated {
        SortDomain::Enumerated {
            constants: constants.clone(),
            domain_source: source(policy),
        }
    } else {
        SortDomain::Open
    };
    Vocabulary::new(
        1,
        vec![SortDecl {
            name: text("E"),
            domain,
        }],
        constants.into_iter().map(primitive).collect(),
    )
    .unwrap()
}
fn extend(v: &Vocabulary, extra: Vec<SymbolDecl>) -> Vocabulary {
    let mut declarations = v.symbols().to_vec();
    declarations.extend(extra);
    Vocabulary::new(v.version(), v.sorts().to_vec(), declarations).unwrap()
}
fn var(index: u64, name: &str) -> Term {
    Term::new(
        vec![TermNode::Var {
            index,
            sort: sort(name),
        }],
        Limits::default(),
    )
    .unwrap()
}
struct View<'a> {
    v: &'a Vocabulary,
    premise: Option<&'a Formula>,
}
impl LedgerView for View<'_> {
    fn current_epoch(&self) -> u64 {
        0
    }
    fn lookup(&self, r: PremiseRef) -> Result<Option<LedgerEntry<'_>>, LedgerError> {
        assert_eq!(r.event_hash(), &[1; 32]);
        Ok(self.premise.map(|f| LedgerEntry {
            kind: EntryKind::Assumption,
            epoch: 0,
            retracted: false,
            vocabulary: self.v,
            formula: f,
        }))
    }
}
fn obligation(v: &Vocabulary, goal: &Formula, premise: Option<&Formula>) -> Obligation {
    let refs = if premise.is_some() {
        vec![PremiseRef::from_event_hash([1; 32])]
    } else {
        vec![]
    };
    generate(
        Proposal {
            vocabulary: v,
            goal,
            cites: &refs,
        },
        &View { v, premise },
    )
    .unwrap()
}
fn classify_goal(v: &Vocabulary, goal: Formula) -> Result<Fragment, GroundError> {
    classify(&obligation(v, &goal, None), v)
}
fn unhex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn open_quantifiers_in_goal_or_premise_are_general_but_unused_open_sort_is_allowed() {
    let v = vocabulary(0, false, false);
    assert_eq!(
        classify_goal(&v, formula(vec![F::True])),
        Ok(Fragment::FiniteDomain)
    );
    for q in [F::Forall(sort("E")), F::Exists(sort("E"))] {
        let f = formula(vec![F::Not, q, F::True]);
        assert_eq!(classify_goal(&v, f.clone()), Ok(Fragment::General));
        let goal = formula(vec![F::True]);
        let o = obligation(&v, &goal, Some(&f));
        let before = to_canon(&o).unwrap();
        assert_eq!(classify(&o, &v), Ok(Fragment::General));
        assert_eq!(to_canon(&o).unwrap(), before);
    }
    let closed = vocabulary(2, true, false);
    assert_eq!(
        classify_goal(
            &closed,
            formula(vec![F::Forall(sort("E")), F::Exists(sort("E")), F::True])
        ),
        Ok(Fragment::FiniteDomain)
    );
}

#[test]
fn all_value_symbols_are_checked_including_unused_constants_and_nullary_functions() {
    for kind in [SymbolKind::Const, SymbolKind::Func] {
        let c = Symbol {
            kind,
            ..constant("value", "E")
        };
        let open = extend(&vocabulary(0, false, false), vec![primitive(c)]);
        assert_eq!(
            classify_goal(&open, formula(vec![F::True])),
            Ok(Fragment::General)
        );
    }
    let v = vocabulary(2, true, false);
    let finite = extend(
        &v,
        vec![primitive(Symbol {
            name: text("f"),
            kind: SymbolKind::Func,
            arity: vec![sort("E")],
            result: SymbolResult::Sort(sort("E")),
        })],
    );
    assert_eq!(
        classify_goal(&finite, formula(vec![F::True])),
        Ok(Fragment::FiniteDomain)
    );
}

#[test]
fn defined_bodies_and_implicit_parameters_are_examined_globally_without_recursion() {
    let mut v = vocabulary(0, false, false);
    let d = pred("D0", vec![]);
    v = extend(
        &v,
        vec![defined(
            d.clone(),
            formula(vec![F::Forall(sort("E")), F::True]),
        )],
    );
    let mut previous = d;
    for i in 1..160 {
        let next = pred(&format!("D{i}"), vec![]);
        v = extend(
            &v,
            vec![defined(
                next.clone(),
                formula(vec![F::Pred {
                    symbol: previous,
                    args: vec![],
                }]),
            )],
        );
        previous = next;
    }
    // Nem o goal True usa a definição, mas o contrato é global.
    let o = obligation(&v, &formula(vec![F::True]), None);
    std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(move || {
            assert_eq!(classify(&o, &v), Ok(Fragment::General));
        })
        .unwrap()
        .join()
        .unwrap();
    let v = extend(
        &vocabulary(0, false, false),
        vec![defined(
            pred("P", vec![sort("E")]),
            formula(vec![F::Eq(var(0, "E"), var(0, "E"))]),
        )],
    );
    assert_eq!(
        classify_goal(&v, formula(vec![F::True])),
        Ok(Fragment::General)
    );
    let v = extend(
        &vocabulary(1, true, false),
        vec![defined(
            pred("P", vec![sort("E")]),
            formula(vec![F::Eq(var(0, "E"), var(0, "E"))]),
        )],
    );
    assert_eq!(
        classify_goal(&v, formula(vec![F::True])),
        Ok(Fragment::FiniteDomain)
    );
}

#[test]
fn invalid_definitions_and_signatures_never_fall_back_to_general() {
    let v = vocabulary(0, false, false);
    let v = extend(
        &v,
        vec![
            primitive(constant("open", "E")),
            defined(
                pred("bad", vec![]),
                formula(vec![F::Eq(var(0, "E"), var(0, "E"))]),
            ),
        ],
    );
    assert!(matches!(
        classify_goal(&v, formula(vec![F::True])),
        Err(GroundError::Sort(_))
    ));
    let mut bad = pred("bad", vec![]);
    bad.result = SymbolResult::Sort(sort("E"));
    let v = extend(&vocabulary(0, true, false), vec![primitive(bad)]);
    assert_eq!(
        classify_goal(&v, formula(vec![F::True])),
        Err(GroundError::InvalidSignature)
    );
}

#[test]
fn defined_result_and_arguments_use_the_evaluator_de_bruijn_order() {
    let names = ["E", "T", "U"];
    let v = Vocabulary::new(
        1,
        names
            .iter()
            .map(|n| SortDecl {
                name: text(n),
                domain: SortDomain::Enumerated {
                    constants: vec![],
                    domain_source: source(false),
                },
            })
            .collect(),
        vec![],
    )
    .unwrap();
    let f = Symbol {
        name: text("f"),
        kind: SymbolKind::Func,
        arity: vec![sort("E"), sort("T")],
        result: SymbolResult::Sort(sort("U")),
    };
    let body = formula(vec![
        F::And(3),
        F::Eq(var(0, "U"), var(0, "U")),
        F::Eq(var(1, "E"), var(1, "E")),
        F::Eq(var(2, "T"), var(2, "T")),
    ]);
    assert_eq!(
        classify_goal(
            &extend(&v, vec![defined(f.clone(), body)]),
            formula(vec![F::True])
        ),
        Ok(Fragment::FiniteDomain)
    );
    let wrong = formula(vec![F::Eq(var(1, "T"), var(1, "T"))]);
    assert!(matches!(
        classify_goal(&extend(&v, vec![defined(f, wrong)]), formula(vec![F::True])),
        Err(GroundError::Sort(_))
    ));
}

#[test]
fn vocabulary_identity_and_input_limits_are_errors_with_inclusive_boundaries() {
    let v = vocabulary(0, true, false);
    let goal = formula(vec![F::True]);
    let o = obligation(&v, &goal, None);
    assert_eq!(
        classify(&o, &vocabulary(0, true, true)),
        Err(GroundError::VocabularyMismatch)
    );
    let exact = AnalysisLimits {
        max_declarations: 1,
        max_nodes: 1,
    };
    assert_eq!(
        classify_with_limits(&o, &v, exact),
        Ok(Fragment::FiniteDomain)
    );
    for l in [
        AnalysisLimits {
            max_nodes: 0,
            ..exact
        },
        AnalysisLimits {
            max_declarations: 0,
            ..exact
        },
    ] {
        assert_eq!(
            classify_with_limits(&o, &v, l),
            Err(GroundError::Unknown(Exhausted::Size))
        );
    }
    let o = obligation(&v, &goal, Some(&goal));
    assert_eq!(
        classify_with_limits(&o, &v, exact),
        Err(GroundError::Unknown(Exhausted::Size))
    );
    let v = extend(
        &v,
        vec![defined(pred("P", vec![]), formula(vec![F::Not, F::True]))],
    );
    let o = obligation(&v, &goal, None);
    assert_eq!(
        classify_with_limits(
            &o,
            &v,
            AnalysisLimits {
                max_declarations: 2,
                max_nodes: 2
            }
        ),
        Err(GroundError::Unknown(Exhausted::Size))
    );
}

#[test]
fn independent_air_vectors_preserve_order_source_identity_and_exact_hashes() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../spec/vectors/fragment/vectors.json")).unwrap();
    let vectors = fixture["vectors"].as_array().unwrap();
    assert_eq!(vectors.len(), 8);
    for case in vectors {
        let v: Vocabulary = from_canon(&unhex(case["vocabulary"].as_str().unwrap())).unwrap();
        let c = domain_axioms(&v, &sort("E")).unwrap();
        assert_eq!(c.vocabulary().to_hex(), case["hv"].as_str().unwrap());
        assert_eq!(c.sort(), &sort("E"));
        match c.source() {
            DomainSource::Fact(r) => {
                assert_eq!(case["source"], "Fact");
                assert_eq!(r.0.to_vec(), unhex(case["payload"].as_str().unwrap()));
            }
            DomainSource::Policy(r) => {
                assert_eq!(case["source"], "Policy");
                assert_eq!(r.0.to_vec(), unhex(case["payload"].as_str().unwrap()));
            }
        }
        let formulas = std::iter::once(c.closure()).chain(c.distinct());
        for (i, f) in formulas.enumerate() {
            logos_air::check(&v, f).unwrap();
            assert_eq!(
                to_canon(f).unwrap(),
                unhex(case["formulas"][i].as_str().unwrap())
            );
            assert_eq!(
                hash(f).unwrap().to_hex(),
                case["hashes"][i].as_str().unwrap()
            );
        }
        assert_eq!(
            c.distinct().len() + 1,
            case["formulas"].as_array().unwrap().len()
        );
    }
}

#[test]
fn input_budget_counts_enumerations_and_signature_arities() {
    let v = vocabulary(2, true, false);
    let goal = formula(vec![F::True]);
    let o = obligation(&v, &goal, None);
    let exact = AnalysisLimits {
        max_declarations: 3,
        max_nodes: 3,
    };
    assert_eq!(
        classify_with_limits(&o, &v, exact),
        Ok(Fragment::FiniteDomain)
    );
    assert_eq!(
        classify_with_limits(
            &o,
            &v,
            AnalysisLimits {
                max_nodes: 2,
                ..exact
            }
        ),
        Err(GroundError::Unknown(Exhausted::Size))
    );
    let f = Symbol {
        name: text("f"),
        kind: SymbolKind::Func,
        arity: vec![sort("E")],
        result: SymbolResult::Sort(sort("E")),
    };
    let v = extend(&v, vec![primitive(f)]);
    let o = obligation(&v, &goal, None);
    let exact = AnalysisLimits {
        max_declarations: 4,
        max_nodes: 4,
    };
    assert_eq!(
        classify_with_limits(&o, &v, exact),
        Ok(Fragment::FiniteDomain)
    );
    assert_eq!(
        classify_with_limits(
            &o,
            &v,
            AnalysisLimits {
                max_nodes: 3,
                ..exact
            }
        ),
        Err(GroundError::Unknown(Exhausted::Size))
    );
}

#[test]
fn candidate_limits_are_inclusive_and_refuse_before_quadratic_expansion() {
    let v = vocabulary(3, true, false);
    let exact = AxiomLimits {
        max_constants: 3,
        max_axioms: 4,
        max_nodes: 23,
        air: Limits {
            max_depth: 4,
            max_nodes: 11,
        },
    };
    let c = domain_axioms_with_limits(&v, &sort("E"), exact).unwrap();
    assert_eq!(
        std::iter::once(c.closure())
            .chain(c.distinct())
            .map(Formula::node_count)
            .sum::<usize>(),
        23
    );
    for l in [
        AxiomLimits {
            max_constants: 2,
            ..exact
        },
        AxiomLimits {
            max_axioms: 3,
            ..exact
        },
        AxiomLimits {
            max_nodes: 22,
            ..exact
        },
        AxiomLimits {
            air: Limits {
                max_depth: 3,
                max_nodes: 11,
            },
            ..exact
        },
        AxiomLimits {
            air: Limits {
                max_depth: 4,
                max_nodes: 10,
            },
            ..exact
        },
    ] {
        assert_eq!(
            domain_axioms_with_limits(&v, &sort("E"), l),
            Err(GroundError::Unknown(Exhausted::Size))
        );
    }
    let big = vocabulary(1000, true, false);
    assert_eq!(
        domain_axioms(&big, &sort("E")),
        Err(GroundError::Unknown(Exhausted::Size))
    );
    let unrestricted = AxiomLimits {
        max_constants: usize::MAX,
        max_axioms: 0,
        max_nodes: usize::MAX,
        air: Limits::default(),
    };
    assert_eq!(
        domain_axioms_with_limits(&big, &sort("E"), unrestricted),
        Err(GroundError::Unknown(Exhausted::Size))
    );
    assert_eq!(
        domain_axioms(&v, &sort("absent")),
        Err(GroundError::UnknownSort)
    );
    assert_eq!(
        domain_axioms(&vocabulary(0, false, false), &sort("E")),
        Err(GroundError::OpenDomain)
    );
    assert_eq!(
        domain_axioms_with_limits(
            &vocabulary(0, true, false),
            &sort("E"),
            AxiomLimits {
                max_axioms: 0,
                ..AxiomLimits::default()
            }
        ),
        Err(GroundError::Unknown(Exhausted::Size))
    );
}

#[test]
fn exhaustive_open_control_models_check_closure_and_names_independently() {
    let mut checked = 0;
    let mut false_closures = 0;
    let mut false_names = 0;
    for policy in [false, true] {
        for k in 0..=3 {
            let declared = vocabulary(k, true, policy);
            let open = vocabulary(k, false, policy);
            let candidates = domain_axioms(&declared, &sort("E")).unwrap();
            for n in 1u32..=4 {
                for code in 0..n.pow(k as u32) {
                    let mut value = code;
                    let mut images = Vec::new();
                    for _ in 0..k {
                        images.push(value % n);
                        value /= n;
                    }
                    let m = FiniteModel::new(
                        ModelData {
                            vocab: hash(&open).unwrap(),
                            domains: vec![n],
                            consts: images
                                .iter()
                                .enumerate()
                                .map(|(i, v)| (i as u32, *v))
                                .collect(),
                            funcs: vec![],
                            preds: vec![],
                        },
                        &open,
                        ModelLimits::default(),
                    )
                    .unwrap();
                    let image = images.iter().copied().collect::<BTreeSet<_>>();
                    let budget = Budget::new(1_000_000, 0, 0, 0);
                    let closure = eval(&m, &open, candidates.closure(), &budget).unwrap();
                    let names = candidates
                        .distinct()
                        .iter()
                        .all(|f| eval(&m, &open, f, &budget).unwrap());
                    assert_eq!(closure, image.len() == n as usize);
                    assert_eq!(names, image.len() == k);
                    false_closures += usize::from(!closure);
                    false_names += usize::from(!names);
                    checked += 1;
                }
            }
            if k > 0 {
                let m = FiniteModel::new(
                    ModelData {
                        vocab: hash(&declared).unwrap(),
                        domains: vec![k as u32],
                        consts: (0..k as u32).map(|i| (i, i)).collect(),
                        funcs: vec![],
                        preds: vec![],
                    },
                    &declared,
                    ModelLimits::default(),
                )
                .unwrap();
                for f in std::iter::once(candidates.closure()).chain(candidates.distinct()) {
                    assert_eq!(
                        eval(&m, &declared, f, &Budget::new(1_000_000, 0, 0, 0)),
                        Ok(true)
                    );
                }
            } else {
                assert!(
                    FiniteModel::new(
                        ModelData {
                            vocab: hash(&declared).unwrap(),
                            domains: vec![0],
                            consts: vec![],
                            funcs: vec![],
                            preds: vec![]
                        },
                        &declared,
                        ModelLimits::default()
                    )
                    .is_err()
                );
            }
        }
    }
    assert_eq!(checked, 288);
    assert!(false_closures > 0 && false_names > 0);
}
