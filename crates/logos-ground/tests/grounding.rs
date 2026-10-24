//! Diferencial exaustivo e regressões do grounder; fontes somente condicionais.
mod support;
use logos_air::{
    FormulaNode as F, Grounding, SortDecl, SortDomain, SymbolKind, SymbolResult, Vocabulary,
};
use logos_canon::to_canon;
use logos_core::Exhausted;
use logos_crypto::hash;
use logos_ground::{
    GroundError, GroundLimits, PropNode, domain_axioms, ground, ground_with_limits,
};
use logos_model::{FiniteModel, ModelData, ModelLimits, eval};
use std::collections::BTreeSet;
use support::*;

fn model(v: &Vocabulary, n: u32, extra: u32, table: Vec<u32>, pred_bits: u32) -> FiniteModel {
    let mut consts = (0..n).map(|i| (i, i)).collect::<Vec<_>>();
    let mut funcs = vec![];
    let mut preds = vec![];
    for (i, d) in v.symbols().iter().enumerate().skip(n as usize) {
        if matches!(d.grounding, Grounding::Defined(_)) {
            continue;
        }
        match d.signature.kind {
            SymbolKind::Const => consts.push((i as u32, extra)),
            SymbolKind::Func => funcs.push((i as u32, table.clone())),
            SymbolKind::Pred => preds.push((
                i as u32,
                (0..n)
                    .filter(|j| (pred_bits >> j) & 1 != 0)
                    .map(|j| vec![j])
                    .collect(),
            )),
        }
    }
    FiniteModel::new(
        ModelData {
            vocab: hash(v).unwrap(),
            domains: vec![n],
            consts,
            funcs,
            preds,
        },
        v,
        ModelLimits::default(),
    )
    .unwrap()
}
#[test]
fn five_thousand_distinct_obligations_match_all_models_and_truth_tables() {
    let e0 = t(c("e0"), vec![]);
    let e1 = t(c("e1"), vec![]);
    let extra = t(c("c"), vec![]);
    let dbody = combine(
        F::Iff,
        &[pred(p("P"), vec![var(0)]), eq(var(0), e0.clone())],
    );
    let gbody = eq(var(0), t(func("f"), vec![var(1)]));
    let v = vocab(
        2,
        vec![
            prim(c("c")),
            prim(func("f")),
            prim(p("P")),
            def(p("D"), dbody),
            def(func("g"), gbody),
            def(
                c("h"),
                combine(
                    F::And(2),
                    &[eq(var(0), e0.clone()), pred(p("P"), vec![e0.clone()])],
                ),
            ),
        ],
    );
    let bases = vec![
        f(vec![F::True]),
        f(vec![F::False]),
        pred(p("P"), vec![e0.clone()]),
        pred(p("P"), vec![e1.clone()]),
        eq(extra.clone(), e0.clone()),
        eq(t(func("f"), vec![e0.clone()]), e0.clone()),
        pred(p("D"), vec![extra.clone()]),
        eq(t(func("g"), vec![extra.clone()]), extra.clone()),
        combine(F::Forall(sort("E")), &[pred(p("P"), vec![var(0)])]),
        combine(F::Exists(sort("E")), &[pred(p("P"), vec![var(0)])]),
        combine(
            F::Forall(sort("E")),
            &[combine(F::Exists(sort("E")), &[eq(var(0), var(1))])],
        ),
        combine(
            F::Not,
            &[eq(t(func("f"), vec![t(func("g"), vec![e1])]), extra)],
        ),
    ];
    let mut ms = Vec::new();
    for cval in 0..2 {
        for table in 0..4 {
            for pb in 0..4 {
                ms.push(model(&v, 2, cval, vec![table & 1, table >> 1], pb));
            }
        }
    }
    assert_eq!(ms.len(), 32);
    let admitted = ms
        .iter()
        .map(|m| match eval(m, &v, &f(vec![F::True]), &budget()) {
            Ok(true) => true,
            Err(logos_model::EvalError::InvalidDefinition { .. }) => false,
            other => panic!("unexpected model outcome {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(admitted.iter().filter(|x| **x).count(), 16);
    let gammas = [
        vec![],
        vec![bases[2].clone()],
        vec![combine(F::Not, &[bases[2].clone()])],
        vec![f(vec![F::False])],
    ];
    let gamma_truths = gammas
        .iter()
        .map(|extra| {
            let o = closed(&v, &f(vec![F::True]), extra);
            ms.iter()
                .enumerate()
                .map(|(j, m)| {
                    !admitted[j]
                        || o.premises()
                            .iter()
                            .all(|(_, f)| eval(m, &v, f, &budget()).unwrap())
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let ops = [F::And(2), F::Or(2), F::Implies, F::Iff];
    let mut seen = BTreeSet::new();
    let mut good = 0;
    let mut bad = 0;
    for i in 0..5000 {
        let mut code = i;
        let a = code % 12;
        code /= 12;
        let b = code % 12;
        code /= 12;
        let c = code % 12;
        code /= 12;
        let x = code % 4;
        code /= 4;
        let y = code % 4;
        let goal = combine(
            ops[x].clone(),
            &[
                bases[a].clone(),
                combine(ops[y].clone(), &[bases[b].clone(), bases[c].clone()]),
            ],
        );
        let o = closed(&v, &goal, &gammas[i % 4]);
        assert!(seen.insert(to_canon(&o).unwrap()));
        let g = ground(&o, &v).unwrap();
        assert_eq!(g.atoms().len(), 8);
        let mut expected = true;
        for (j, m) in ms.iter().enumerate() {
            if admitted[j] {
                let goal_true = eval(m, &v, &goal, &budget()).unwrap();
                expected &= !gamma_truths[i % 4][j] || goal_true;
            }
        }
        assert_eq!(tautology(&g), expected, "case {i}");
        if expected { good += 1 } else { bad += 1 }
    }
    assert_eq!(seen.len(), 5000);
    assert!(good > 100 && bad > 100);
    println!(
        "GROUND_DIFFERENTIAL_OK cases=5000 models_per_case=32 admissible=16 atom_valuations=256 valid={good} invalid={bad}"
    );
}

#[test]
fn values_are_total_congruent_and_additional_constants_are_not_distinct_names() {
    let v = vocab(2, vec![prim(c("c")), prim(func("f")), prim(p("P"))]);
    let a = t(c("e0"), vec![]);
    let b = t(c("e1"), vec![]);
    let cc = t(c("c"), vec![]);
    for (goal, want) in [
        (
            combine(
                F::Not,
                &[combine(
                    F::And(2),
                    &[
                        eq(t(func("f"), vec![a.clone()]), a.clone()),
                        eq(t(func("f"), vec![a.clone()]), b.clone()),
                    ],
                )],
            ),
            true,
        ),
        (eq(cc.clone(), a.clone()), false),
        (eq(t(func("f"), vec![a.clone()]), a.clone()), false),
        (
            combine(
                F::Implies,
                &[
                    eq(cc.clone(), a.clone()),
                    eq(
                        t(func("f"), vec![cc.clone()]),
                        t(func("f"), vec![a.clone()]),
                    ),
                ],
            ),
            true,
        ),
        (
            combine(
                F::Or(2),
                &[
                    eq(t(func("f"), vec![cc.clone()]), a.clone()),
                    eq(t(func("f"), vec![cc.clone()]), b),
                ],
            ),
            true,
        ),
    ] {
        let o = closed(&v, &goal, &[]);
        assert_eq!(tautology(&ground(&o, &v).unwrap()), want);
    }
}

#[test]
fn unused_defined_values_restrict_models_and_enumerated_ordinals() {
    for body in [f(vec![F::False]), f(vec![F::True])] {
        let v = vocab(2, vec![def(c("d"), body)]);
        let m = model(&v, 2, 0, vec![], 0);
        assert!(matches!(
            eval(&m, &v, &f(vec![F::True]), &budget()),
            Err(logos_model::EvalError::InvalidDefinition { .. })
        ));
        let o = closed(&v, &f(vec![F::False]), &[]);
        let g = ground(&o, &v).unwrap();
        assert!(tautology(&g));
        assert!(!prop(&g, 0, g.constraints()));
    }
    let v = vocab(
        2,
        vec![
            prim(p("P")),
            def(
                c("d"),
                combine(
                    F::And(2),
                    &[
                        eq(var(0), t(c("e0"), vec![])),
                        pred(p("P"), vec![t(c("e0"), vec![])]),
                    ],
                ),
            ),
        ],
    );
    let o = closed(&v, &pred(p("P"), vec![t(c("e0"), vec![])]), &[]);
    assert!(tautology(&ground(&o, &v).unwrap()));
    let mut declarations = vocab(2, vec![]).symbols().to_vec();
    declarations[0] = def(c("e0"), eq(var(0), t(c("e1"), vec![])));
    let v = Vocabulary::new(1, vocab(2, vec![]).sorts().to_vec(), declarations).unwrap();
    let o = closed(&v, &f(vec![F::False]), &[]);
    assert!(tautology(&ground(&o, &v).unwrap()));
}

#[test]
fn explicit_axioms_identity_general_and_provenance_are_checked() {
    let v = vocab(2, vec![]);
    let goal = f(vec![F::True]);
    let o = closed(&v, &goal, &[]);
    let before = to_canon(&o).unwrap();
    let g = ground(&o, &v).unwrap();
    assert_eq!(g.obligation(), hash(&o).unwrap());
    assert_eq!(g.vocabulary(), hash(&v).unwrap());
    assert_eq!(to_canon(&o).unwrap(), before);
    assert_eq!(g.domain_axioms().len(), 2);
    assert_eq!(
        g.premises().iter().map(|(r, _)| *r).collect::<Vec<_>>(),
        o.premises().iter().map(|(r, _)| *r).collect::<Vec<_>>()
    );
    let fs = axioms(&v);
    for i in 0..fs.len() {
        let kept = fs
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, f)| f.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            ground(&obligation(&v, &goal, &kept), &v),
            Err(GroundError::MissingDomainAxiom)
        );
    }
    assert_eq!(
        ground(&o, &vocab(1, vec![])),
        Err(GroundError::VocabularyMismatch)
    );
    let open = Vocabulary::new(
        1,
        vec![SortDecl {
            name: text("E"),
            domain: SortDomain::Open,
        }],
        vec![],
    )
    .unwrap();
    let oo = obligation(&open, &combine(F::Forall(sort("E")), &[goal]), &[]);
    assert_eq!(ground(&oo, &open), Err(GroundError::GeneralFragment));
}

#[test]
fn empty_singleton_nullary_and_small_stack_definition_chain() {
    for n in 0..=1 {
        let v = vocab(n, vec![]);
        let o = closed(&v, &f(vec![F::False]), &[]);
        assert_eq!(tautology(&ground(&o, &v).unwrap()), n == 0);
    }
    let mut null = func("n");
    null.arity.clear();
    let mut pp = p("p0");
    pp.arity.clear();
    let v = vocab(2, vec![prim(null.clone()), prim(pp.clone())]);
    let o = closed(&v, &eq(t(null.clone(), vec![]), t(null, vec![])), &[]);
    assert!(tautology(&ground(&o, &v).unwrap()));
    let mut ds = vec![];
    let mut last = pp;
    ds.push(prim(last.clone()));
    for i in 0..200 {
        let mut next = p(&format!("d{i}"));
        next.arity.clear();
        ds.push(def(next.clone(), pred(last, vec![])));
        last = next;
    }
    let v = vocab(1, ds);
    let goal = combine(
        F::Iff,
        &[
            pred(last, vec![]),
            pred(
                {
                    let mut s = p("p0");
                    s.arity.clear();
                    s
                },
                vec![],
            ),
        ],
    );
    let o = closed(&v, &goal, &[]);
    std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(move || assert!(tautology(&ground(&o, &v).unwrap())))
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn resource_limits_are_inclusive_and_no_partial_program_escapes() {
    let v = vocab(2, vec![prim(c("c")), prim(func("f")), prim(p("P"))]);
    let o = closed(&v, &eq(t(c("c"), vec![]), t(c("e0"), vec![])), &[]);
    let p = ground(&o, &v).unwrap();
    let nodes = p.nodes().len();
    let edges = p
        .nodes()
        .iter()
        .map(|n| match n {
            PropNode::Not(_) => 1,
            PropNode::And(x) | PropNode::Or(x) => x.len(),
            _ => 0,
        })
        .sum::<usize>();
    let bytes = to_canon(&p).unwrap().len();
    let exact = GroundLimits {
        max_atoms: p.atoms().len(),
        max_nodes: nodes,
        max_edges: edges,
        max_bytes: bytes,
        ..GroundLimits::default()
    };
    assert_eq!(ground_with_limits(&o, &v, exact).unwrap(), p);
    for l in [
        GroundLimits {
            max_atoms: exact.max_atoms - 1,
            ..exact
        },
        GroundLimits {
            max_nodes: nodes - 1,
            ..exact
        },
        GroundLimits {
            max_edges: edges - 1,
            ..exact
        },
        GroundLimits {
            max_bytes: bytes - 1,
            ..exact
        },
        GroundLimits {
            max_cells: 0,
            ..exact
        },
        GroundLimits {
            max_steps: 0,
            ..exact
        },
    ] {
        assert_eq!(
            ground_with_limits(&o, &v, l),
            Err(GroundError::Unknown(Exhausted::Size))
        );
    }
    for steps in [false, true] {
        let mut low = 0;
        let mut high = if steps {
            exact.max_steps
        } else {
            exact.max_cells
        };
        while low < high {
            let mid = low + (high - low) / 2;
            let l = if steps {
                GroundLimits {
                    max_steps: mid,
                    ..exact
                }
            } else {
                GroundLimits {
                    max_cells: mid,
                    ..exact
                }
            };
            match ground_with_limits(&o, &v, l) {
                Ok(_) => high = mid,
                Err(GroundError::Unknown(Exhausted::Size)) => low = mid + 1,
                other => panic!("unexpected limit outcome {other:?}"),
            }
        }
        assert!(low > 0);
        let yes = if steps {
            GroundLimits {
                max_steps: low,
                ..exact
            }
        } else {
            GroundLimits {
                max_cells: low,
                ..exact
            }
        };
        let no = if steps {
            GroundLimits {
                max_steps: low - 1,
                ..exact
            }
        } else {
            GroundLimits {
                max_cells: low - 1,
                ..exact
            }
        };
        assert_eq!(ground_with_limits(&o, &v, yes).unwrap(), p);
        assert_eq!(
            ground_with_limits(&o, &v, no),
            Err(GroundError::Unknown(Exhausted::Size))
        );
    }
    let mut many = func("big");
    many.arity = vec![sort("E"); usize::BITS as usize];
    let v = vocab(2, vec![prim(many)]);
    let o = closed(&v, &f(vec![F::True]), &[]);
    assert_eq!(ground(&o, &v), Err(GroundError::Unknown(Exhausted::Size)));
}

#[test]
fn different_sorts_and_de_bruijn_parameter_order_are_preserved() {
    let base = vocab(2, vec![]);
    let mut ss = base.sorts().to_vec();
    let mut a = c("t0");
    a.result = SymbolResult::Sort(sort("T"));
    let mut b = c("t1");
    b.result = SymbolResult::Sort(sort("T"));
    ss.push(SortDecl {
        name: text("T"),
        domain: SortDomain::Enumerated {
            constants: vec![a.clone(), b.clone()],
            domain_source: logos_air::DomainSource::Fact(logos_air::FactRef([1; 32])),
        },
    });
    let mut ds = base.symbols().to_vec();
    ds.extend([prim(a.clone()), prim(b)]);
    let mut fun = func("mixed");
    fun.arity = vec![sort("E"), sort("T")];
    fun.result = SymbolResult::Sort(sort("T"));
    let vv = |i| {
        logos_air::Term::new(
            vec![logos_air::TermNode::Var {
                index: i,
                sort: sort("T"),
            }],
            logos_air::Limits::default(),
        )
        .unwrap()
    };
    ds.push(def(fun.clone(), eq(vv(0), vv(2))));
    let v = Vocabulary::new(1, ss, ds).unwrap();
    let goal = eq(
        t(fun, vec![t(c("e1"), vec![]), t(a.clone(), vec![])]),
        t(a, vec![]),
    );
    let o = closed(&v, &goal, &[]);
    assert!(tautology(&ground(&o, &v).unwrap()));
}

#[test]
fn quantifier_polarity_outer_binders_and_false_premises_keep_consequence() {
    let v = vocab(2, vec![prim(p("P"))]);
    let a = t(c("e0"), vec![]);
    let forall_exists = combine(
        F::Forall(sort("E")),
        &[combine(F::Exists(sort("E")), &[eq(var(0), var(1))])],
    );
    let exists_forall = combine(
        F::Exists(sort("E")),
        &[combine(F::Forall(sort("E")), &[eq(var(0), var(1))])],
    );
    let outer = combine(
        F::Forall(sort("E")),
        &[combine(
            F::Exists(sort("E")),
            &[combine(
                F::Or(2),
                &[eq(var(0), a), pred(p("P"), vec![var(1)])],
            )],
        )],
    );
    for (goal, want) in [(forall_exists, true), (exists_forall, false), (outer, true)] {
        let o = closed(&v, &goal, &[]);
        assert_eq!(tautology(&ground(&o, &v).unwrap()), want);
    }
    let o = closed(&v, &f(vec![F::False]), &[f(vec![F::False])]);
    assert!(tautology(&ground(&o, &v).unwrap()));
}

#[test]
fn deterministic_output_repeats_with_larger_limits() {
    let v = vocab(2, vec![prim(c("c")), prim(func("f")), prim(p("P"))]);
    let o = closed(
        &v,
        &combine(
            F::Forall(sort("E")),
            &[pred(p("P"), vec![t(func("f"), vec![var(0)])])],
        ),
        &[],
    );
    let g = ground(&o, &v).unwrap();
    assert!(g.atoms().windows(2).all(|p| p[0] < p[1]));
    let expected = to_canon(&g).unwrap();
    for _ in 0..100 {
        assert_eq!(to_canon(&ground(&o, &v).unwrap()).unwrap(), expected);
    }
    assert_eq!(
        to_canon(
            &ground_with_limits(
                &o,
                &v,
                GroundLimits {
                    max_steps: 2_000_000,
                    ..GroundLimits::default()
                }
            )
            .unwrap()
        )
        .unwrap(),
        expected
    );
}

#[test]
fn independent_vectors_fix_bytes_hashes_and_repeated_process_inputs() {
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../../../spec/vectors/grounding/vectors.json")).unwrap();
    let hex = |s: &str| {
        s.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
            .collect::<Vec<_>>()
    };
    let cases = data["vectors"].as_array().unwrap();
    assert_eq!(cases.len(), 12);
    for case in cases {
        let v: Vocabulary =
            logos_canon::from_canon(&hex(case["vocabulary"].as_str().unwrap())).unwrap();
        let goal: logos_air::Formula =
            logos_canon::from_canon(&hex(case["goal"].as_str().unwrap())).unwrap();
        let fs = case["premises"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| logos_canon::from_canon(&hex(x.as_str().unwrap())).unwrap())
            .collect::<Vec<logos_air::Formula>>();
        let o = obligation(&v, &goal, &fs);
        assert_eq!(
            to_canon(&o).unwrap(),
            hex(case["obligation"].as_str().unwrap())
        );
        for _ in 0..8 {
            let g = ground(&o, &v).unwrap();
            assert_eq!(
                to_canon(&g).unwrap(),
                hex(case["ground"].as_str().unwrap()),
                "{}",
                case["id"]
            );
            assert_eq!(hash(&g).unwrap().to_hex(), case["digest"].as_str().unwrap());
        }
    }
}

#[test]
fn tuple_coordinates_are_charged_before_creating_atom_arguments() {
    let base = vocab(2, vec![]);
    let mut unit = c("unit");
    unit.result = SymbolResult::Sort(sort("T"));
    let mut sorts = base.sorts().to_vec();
    sorts.push(SortDecl {
        name: text("T"),
        domain: SortDomain::Enumerated {
            constants: vec![unit.clone()],
            domain_source: logos_air::DomainSource::Fact(logos_air::FactRef([7; 32])),
        },
    });
    let mut wide = func("wide");
    wide.arity.extend(vec![sort("T"); 4]);
    let mut symbols = base.symbols().to_vec();
    symbols.extend([prim(unit), prim(wide)]);
    let v = Vocabulary::new(1, sorts, symbols).unwrap();
    let o = closed(&v, &f(vec![F::True]), &[]);
    // 4 células dos representantes E +1 de T +2 entradas*2 outputs*(5 coordenadas+1).
    let exact = GroundLimits {
        max_cells: 29,
        ..GroundLimits::default()
    };
    assert!(ground_with_limits(&o, &v, exact).is_ok());
    for cap in [12, 28] {
        assert_eq!(
            ground_with_limits(
                &o,
                &v,
                GroundLimits {
                    max_cells: cap,
                    ..exact
                }
            ),
            Err(GroundError::Unknown(Exhausted::Size))
        );
    }
}

#[test]
fn enumeration_conjunction_is_exact_and_covers_the_sort_as_one_premise() {
    // 6.1b (ADR0062): a fórmula que um fato de fonte afirma.
    let v = vocab(3, vec![]);
    let a = domain_axioms(&v, &sort("E")).unwrap();
    let conjunction = a.conjunction().unwrap();
    let mut expected = vec![F::And(4)];
    for f in std::iter::once(a.closure()).chain(a.distinct()) {
        expected.extend_from_slice(f.nodes());
    }
    assert_eq!(conjunction.nodes(), &expected[..]);
    // Sozinha basta e vira o único suporte do sort; o fecho do sort é teorema.
    let goal = a.closure().clone();
    let g = ground(
        &obligation(&v, &goal, std::slice::from_ref(&conjunction)),
        &v,
    )
    .unwrap();
    assert_eq!(g.domain_axioms().len(), 1);
    assert!(tautology(&g));
    // Com axiomas separados também presentes, a conjunção tem preferência.
    let mut fs = axioms(&v);
    fs.push(conjunction.clone());
    let o = obligation(&v, &goal, &fs);
    let g = ground(&o, &v).unwrap();
    assert_eq!(g.domain_axioms(), &[o.premises().last().unwrap().0][..]);
    // Outra ordem dos conjuntos não é a conjunção: falta o axioma.
    let mut swapped = vec![F::And(4)];
    for f in a.distinct().iter().chain(std::iter::once(a.closure())) {
        swapped.extend_from_slice(f.nodes());
    }
    assert_eq!(
        ground(&obligation(&v, &goal, &[f(swapped)]), &v),
        Err(GroundError::MissingDomainAxiom)
    );
}
