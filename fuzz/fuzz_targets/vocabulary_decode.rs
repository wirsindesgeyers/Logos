//! Bytes aceitos por vocabulary/v1 precisam reencodar identicamente e respeitar limites lógicos.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_air::{
    DomainSource, FactRef, Formula, FormulaNode, Grounding, Limits, PolicyRef, Sort, SortDecl,
    SortDomain, Symbol, SymbolDecl, SymbolKind, SymbolResult, Vocabulary,
};
use logos_canon::{Text, from_canon, to_canon};
use logos_crypto::hash;
use logos_ground::{AxiomLimits, classify, domain_axioms, domain_axioms_with_limits};
use logos_obligation::{LedgerEntry, LedgerError, LedgerView, PremiseRef, Proposal, generate};

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
