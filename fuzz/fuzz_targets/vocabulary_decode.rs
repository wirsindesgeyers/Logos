//! Bytes aceitos por vocabulary/v1 precisam reencodar identicamente e respeitar limites lógicos.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_air::{
    DomainSource, FactRef, Formula, FormulaNode, Grounding, Limits, PolicyRef, Sort, SortDecl,
    SortDomain, Symbol, SymbolDecl, SymbolKind, SymbolResult, Vocabulary,
};
use logos_canon::{Text, from_canon, to_canon};
use logos_crypto::hash;
use logos_ground::{
    AxiomLimits, GroundLimits, PropNode, classify, domain_axioms, domain_axioms_with_limits,
    ground_with_limits,
};
use logos_obligation::{
    EntryKind, LedgerEntry, LedgerError, LedgerView, PremiseRef, Proposal, generate,
};

fn roundtrip(data: &[u8]) {
    if let Ok(vocab) = from_canon::<Vocabulary>(data) {
        let encoded = to_canon(&vocab).expect("Vocabulary aceito deve ser codificável");
        assert_eq!(encoded, data, "Vocabulary não canônico aceito");
        let decoded: Vocabulary =
            from_canon(&encoded).expect("Vocabulary codificado deve ser decodificável");
        assert_eq!(decoded, vocab);
        probe(&vocab);
    }
}

struct EmptyView;
impl LedgerView for EmptyView {
    fn current_epoch(&self) -> u64 {
        0
    }
    fn lookup(&self, _: PremiseRef) -> Result<Option<LedgerEntry<'_>>, LedgerError> {
        Ok(None)
    }
}

fn probe(v: &Vocabulary) {
    ground_probe(v);
    let goal = Formula::new(vec![FormulaNode::True], Limits::default()).expect("goal");
    let o = generate(
        Proposal {
            vocabulary: v,
            goal: &goal,
            cites: &[],
        },
        &EmptyView,
    )
    .expect("generated O");
    assert_eq!(classify(&o, v), classify(&o, v));
    for decl in v.sorts().iter().take(4) {
        let sort = Sort {
            name: decl.name.clone(),
        };
        let result = domain_axioms(v, &sort);
        assert_eq!(result, domain_axioms(v, &sort));
        if let Ok(c) = result {
            assert_eq!(c.vocabulary(), hash(v).expect("hV"));
            let SortDomain::Enumerated {
                constants,
                domain_source,
            } = &decl.domain
            else {
                panic!("Open accepted");
            };
            assert_eq!(c.source(), domain_source);
            assert_eq!(
                c.distinct().len(),
                constants.len() * constants.len().saturating_sub(1) / 2
            );
            for f in std::iter::once(c.closure()).chain(c.distinct()) {
                logos_air::check(v, f).expect("typed candidate");
            }
        }
    }
}

struct Assumptions<'a> {
    v: &'a Vocabulary,
    fs: &'a [Formula],
}
impl LedgerView for Assumptions<'_> {
    fn current_epoch(&self) -> u64 {
        0
    }
    fn lookup(&self, r: PremiseRef) -> Result<Option<LedgerEntry<'_>>, LedgerError> {
        let i = usize::from(r.event_hash()[0]);
        Ok(self.fs.get(i).map(|formula| LedgerEntry {
            kind: EntryKind::Assumption,
            epoch: 0,
            retracted: false,
            vocabulary: self.v,
            formula,
        }))
    }
}
fn ground_probe(v: &Vocabulary) {
    if v.sorts().len() > 4 || v.symbols().len() > 64 {
        return;
    }
    let mut fs = Vec::new();
    let mut goal = Formula::new(vec![FormulaNode::True], Limits::default()).expect("goal");
    for s in v.sorts() {
        if let SortDomain::Enumerated { constants, .. } = &s.domain {
            if constants.len() > 4 {
                return;
            }
            let Ok(a) = domain_axioms(
                v,
                &Sort {
                    name: s.name.clone(),
                },
            ) else {
                return;
            };
            fs.push(a.closure().clone());
            fs.extend_from_slice(a.distinct());
            let sort = Sort {
                name: s.name.clone(),
            };
            let var = |index| {
                logos_air::Term::new(
                    vec![logos_air::TermNode::Var {
                        index,
                        sort: sort.clone(),
                    }],
                    Limits::default(),
                )
                .expect("var")
            };
            goal = Formula::new(
                vec![
                    FormulaNode::Forall(sort.clone()),
                    FormulaNode::Exists(sort.clone()),
                    FormulaNode::Eq(var(0), var(1)),
                ],
                Limits::default(),
            )
            .expect("quantifier");
        }
    }
    let refs = (0..fs.len())
        .map(|i| {
            let mut r = [0; 32];
            r[0] = i as u8;
            PremiseRef::from_event_hash(r)
        })
        .collect::<Vec<_>>();
    let o = generate(
        Proposal {
            vocabulary: v,
            goal: &goal,
            cites: &refs,
        },
        &Assumptions { v, fs: &fs },
    )
    .expect("conditional O");
    let before = to_canon(&o).expect("O bytes");
    let limits = GroundLimits {
        max_atoms: 128,
        max_nodes: 2048,
        max_edges: 8192,
        max_cells: 512,
        max_steps: 16384,
        max_bytes: 65536,
        ..GroundLimits::default()
    };
    let result = ground_with_limits(&o, v, limits);
    assert_eq!(result, ground_with_limits(&o, v, limits));
    assert_eq!(to_canon(&o).expect("O bytes"), before);
    if let Ok(g) = result {
        assert_eq!(g.obligation(), hash(&o).expect("hO"));
        assert_eq!(g.vocabulary(), hash(v).expect("hV"));
        assert!(g.atoms().windows(2).all(|p| p[0] < p[1]));
        for (i, n) in g.nodes().iter().enumerate() {
            match n {
                PropNode::Atom(a) => assert!(*a < g.atoms().len()),
                PropNode::Not(a) => assert!(*a < i),
                PropNode::And(xs) | PropNode::Or(xs) => assert!(xs.iter().all(|x| *x < i)),
                _ => {}
            }
        }
        assert!(to_canon(&g).expect("G bytes").len() <= limits.max_bytes);
    }
}

fn structured(data: &[u8]) {
    let byte = data.first().copied().unwrap_or(0);
    let sort = Sort {
        name: Text::new("E").expect("sort"),
    };
    let constants = (0..byte % 9)
        .map(|i| Symbol {
            name: Text::new(format!("c{i}")).expect("name"),
            kind: SymbolKind::Const,
            arity: vec![],
            result: SymbolResult::Sort(sort.clone()),
        })
        .collect::<Vec<_>>();
    let source = if byte & 16 == 0 {
        DomainSource::Fact(FactRef([byte; 32]))
    } else {
        DomainSource::Policy(PolicyRef([byte; 32]))
    };
    let mut declarations = constants
        .iter()
        .map(|s| SymbolDecl {
            signature: s.clone(),
            grounding: Grounding::Glossed(Text::new("").expect("text")),
            gloss_pt: Text::new("").expect("text"),
        })
        .collect::<Vec<_>>();
    if byte & 64 != 0 {
        declarations.push(SymbolDecl {
            signature: Symbol {
                name: Text::new("f").expect("f"),
                kind: SymbolKind::Func,
                arity: vec![sort.clone()],
                result: SymbolResult::Sort(sort.clone()),
            },
            grounding: Grounding::Glossed(Text::new("").expect("text")),
            gloss_pt: Text::new("").expect("text"),
        });
    }
    if byte & 128 != 0 {
        let variable = logos_air::Term::new(
            vec![logos_air::TermNode::Var {
                index: 0,
                sort: sort.clone(),
            }],
            Limits::default(),
        )
        .expect("variable");
        declarations.push(SymbolDecl {
            signature: Symbol {
                name: Text::new("D").expect("D"),
                kind: SymbolKind::Pred,
                arity: vec![sort.clone()],
                result: SymbolResult::Bool,
            },
            grounding: Grounding::Defined(
                Formula::new(
                    vec![FormulaNode::Eq(variable.clone(), variable)],
                    Limits::default(),
                )
                .expect("definition"),
            ),
            gloss_pt: Text::new("").expect("text"),
        });
    }
    if byte & 32 != 0 {
        declarations.push(SymbolDecl {
            signature: Symbol {
                name: Text::new("open").expect("name"),
                kind: SymbolKind::Func,
                arity: vec![],
                result: SymbolResult::Sort(Sort {
                    name: Text::new("S").expect("S"),
                }),
            },
            grounding: Grounding::Glossed(Text::new("").expect("text")),
            gloss_pt: Text::new("").expect("text"),
        });
    }
    let v = Vocabulary::new(
        1,
        vec![
            SortDecl {
                name: sort.name.clone(),
                domain: SortDomain::Enumerated {
                    constants,
                    domain_source: source,
                },
            },
            SortDecl {
                name: Text::new("S").expect("S"),
                domain: SortDomain::Open,
            },
        ],
        declarations,
    )
    .expect("vocab");
    probe(&v);
    let limits = AxiomLimits {
        max_constants: usize::from(byte % 10),
        max_axioms: usize::from(byte),
        max_nodes: usize::from(byte),
        ..AxiomLimits::default()
    };
    assert_eq!(
        domain_axioms_with_limits(&v, &sort, limits),
        domain_axioms_with_limits(&v, &sort, limits)
    );
}

fuzz_target!(|data: &[u8]| {
    roundtrip(data);
    structured(data);
    let limits = Limits {
        max_depth: usize::from(data.last().copied().unwrap_or(0) % 16),
        max_nodes: usize::from(data.get(data.len() / 2).copied().unwrap_or(0)),
    };
    if let Ok(vocab) = Vocabulary::decode_with_limits(data, limits) {
        let encoded = to_canon(&vocab).expect("Vocabulary aceito deve ser codificável sob limites");
        assert_eq!(encoded, data, "Vocabulary sob limites não canônico aceito");
        let decoded = Vocabulary::decode_with_limits(&encoded, limits)
            .expect("roundtrip de Vocabulary sob limites");
        assert_eq!(decoded, vocab);

        for declaration in vocab.symbols() {
            if let logos_air::Grounding::Defined(formula) = &declaration.grounding {
                assert!(formula.depth() <= limits.max_depth);
                assert!(formula.node_count() <= limits.max_nodes);
            }
        }
    }
});
