//! Testes públicos de comportamento do gerador de obrigações (Etapa 1.5).
//!
//! Contratos normativos: `spec/obligation.md`, ADR 0012, `docs/plans/etapa-1.5.md`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use logos_air::{
    Formula, FormulaNode, Limits, LogicId, Sort, SortDecl, SortDomain, Symbol, SymbolDecl,
    SymbolKind, SymbolResult, Term, TermNode, Vocabulary,
};
use logos_canon::{Text, to_canon};
use logos_obligation::{
    EntryKind, LedgerEntry, LedgerError, LedgerView, PremiseRef, Proposal, Reject, generate,
    generate_speculative,
};

// ---------------------------------------------------------------------------
// Helpers para construção de vocabulário e fórmulas em testes
// ---------------------------------------------------------------------------

fn sort(name: &str) -> Sort {
    Sort {
        name: Text::new(name).unwrap(),
    }
}

fn sort_decl(name: &str) -> SortDecl {
    SortDecl {
        name: Text::new(name).unwrap(),
        domain: SortDomain::Open,
    }
}

fn symbol(name: &str, kind: SymbolKind, arity: Vec<Sort>, result: SymbolResult) -> Symbol {
    Symbol {
        name: Text::new(name).unwrap(),
        kind,
        arity,
        result,
    }
}

fn symbol_decl(sym: Symbol) -> SymbolDecl {
    SymbolDecl {
        signature: sym,
        grounding: logos_air::Grounding::Glossed(Text::new("test gloss").unwrap()),
        gloss_pt: Text::new("glosa de teste").unwrap(),
    }
}

fn base_vocab() -> Vocabulary {
    let sorts = vec![sort_decl("U"), sort_decl("V")];
    let symbols = vec![
        symbol_decl(symbol(
            "c_u",
            SymbolKind::Const,
            vec![],
            SymbolResult::Sort(sort("U")),
        )),
        symbol_decl(symbol(
            "P",
            SymbolKind::Pred,
            vec![sort("U")],
            SymbolResult::Bool,
        )),
    ];
    Vocabulary::new(1, sorts, symbols).unwrap()
}

fn extended_vocab() -> Vocabulary {
    let sorts = vec![sort_decl("U"), sort_decl("V"), sort_decl("W")];
    let symbols = vec![
        symbol_decl(symbol(
            "c_u",
            SymbolKind::Const,
            vec![],
            SymbolResult::Sort(sort("U")),
        )),
        symbol_decl(symbol(
            "P",
            SymbolKind::Pred,
            vec![sort("U")],
            SymbolResult::Bool,
        )),
        symbol_decl(symbol(
            "c_w",
            SymbolKind::Const,
            vec![],
            SymbolResult::Sort(sort("W")),
        )),
    ];
    Vocabulary::new(2, sorts, symbols).unwrap()
}

fn formula_true() -> Formula {
    Formula::new(vec![FormulaNode::True], Limits::default()).unwrap()
}

fn formula_false() -> Formula {
    Formula::new(vec![FormulaNode::False], Limits::default()).unwrap()
}

fn formula_pred_p(term: Term) -> Formula {
    Formula::new(
        vec![FormulaNode::Pred {
            symbol: symbol("P", SymbolKind::Pred, vec![sort("U")], SymbolResult::Bool),
            args: vec![term],
        }],
        Limits::default(),
    )
    .unwrap()
}

fn term_const_u() -> Term {
    Term::new(
        vec![TermNode::App {
            symbol: symbol(
                "c_u",
                SymbolKind::Const,
                vec![],
                SymbolResult::Sort(sort("U")),
            ),
            args: 0,
        }],
        Limits::default(),
    )
    .unwrap()
}

fn make_ref(byte: u8) -> PremiseRef {
    let mut hash = [0u8; 32];
    hash[0] = byte;
    PremiseRef::from_event_hash(hash)
}

// ---------------------------------------------------------------------------
// FakeLedger snapshot confiável em memória (conforme ADR 0012 e roadmap)
// ---------------------------------------------------------------------------

struct FakeRecord {
    kind: EntryKind,
    epoch: u64,
    retracted: bool,
    vocabulary: Vocabulary,
    formula: Formula,
}

struct FakeLedger {
    epoch: u64,
    entries: BTreeMap<PremiseRef, FakeRecord>,
    errors: BTreeMap<PremiseRef, LedgerError>,
    lookup_count: AtomicUsize,
}

impl FakeLedger {
    fn new(epoch: u64) -> Self {
        Self {
            epoch,
            entries: BTreeMap::new(),
            errors: BTreeMap::new(),
            lookup_count: AtomicUsize::new(0),
        }
    }

    fn insert_entry(
        &mut self,
        reference: PremiseRef,
        kind: EntryKind,
        epoch: u64,
        retracted: bool,
        vocabulary: Vocabulary,
        formula: Formula,
    ) {
        self.entries.insert(
            reference,
            FakeRecord {
                kind,
                epoch,
                retracted,
                vocabulary,
                formula,
            },
        );
    }

    fn insert_error(&mut self, reference: PremiseRef, error: LedgerError) {
        self.errors.insert(reference, error);
    }

    fn lookups(&self) -> usize {
        self.lookup_count.load(Ordering::SeqCst)
    }
}

impl LedgerView for FakeLedger {
    fn current_epoch(&self) -> u64 {
        self.epoch
    }

    fn lookup(&self, reference: PremiseRef) -> Result<Option<LedgerEntry<'_>>, LedgerError> {
        self.lookup_count.fetch_add(1, Ordering::SeqCst);
        if let Some(&err) = self.errors.get(&reference) {
            return Err(err);
        }
        if let Some(record) = self.entries.get(&reference) {
            Ok(Some(LedgerEntry {
                kind: record.kind,
                epoch: record.epoch,
                retracted: record.retracted,
                vocabulary: &record.vocabulary,
                formula: &record.formula,
            }))
        } else {
            Ok(None)
        }
    }
}

// ---------------------------------------------------------------------------
// Testes comportamentais públicos
// ---------------------------------------------------------------------------

#[test]
fn test_generate_minimal_empty_cites() {
    let vocab = base_vocab();
    let goal = formula_true();
    let proposal = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &[],
    };
    let ledger = FakeLedger::new(1);

    let obligation = generate(proposal, &ledger).unwrap();
    assert_eq!(obligation.logic(), LogicId::FolMsEqV1);
    assert_eq!(obligation.encoder_version(), 1);
    assert_eq!(obligation.vocab(), logos_crypto::hash(&vocab).unwrap());
    assert!(obligation.premises().is_empty());
    assert_eq!(obligation.goal(), &goal);
}

#[test]
fn test_generate_with_all_four_citable_kinds() {
    let vocab = base_vocab();
    let goal = formula_true();
    let mut ledger = FakeLedger::new(10);

    let citable_kinds = [
        (make_ref(1), EntryKind::Fact),
        (make_ref(2), EntryKind::Policy),
        (make_ref(3), EntryKind::Verified),
        (make_ref(4), EntryKind::Assumption),
    ];

    let mut cites = Vec::new();
    for (r, kind) in &citable_kinds {
        cites.push(*r);
        ledger.insert_entry(*r, *kind, 10, false, vocab.clone(), formula_true());
    }

    let proposal = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &cites,
    };

    let obligation = generate(proposal, &ledger).unwrap();
    assert_eq!(obligation.premises().len(), 4);
}

#[test]
fn test_reject_all_six_non_citable_kinds() {
    let vocab = base_vocab();
    let goal = formula_true();

    let non_citable = [
        EntryKind::Pending,
        EntryKind::Refuted,
        EntryKind::Unknown,
        EntryKind::Rejected,
        EntryKind::Invalidated,
        EntryKind::Proposal,
    ];

    for kind in non_citable {
        let mut ledger = FakeLedger::new(5);
        let ref_id = make_ref(42);
        ledger.insert_entry(ref_id, kind, 5, false, vocab.clone(), formula_true());

        let proposal = Proposal {
            vocabulary: &vocab,
            goal: &goal,
            cites: &[ref_id],
        };

        let err = generate(proposal, &ledger).unwrap_err();
        match err {
            Reject::NotCitable { reference, kind: k } => {
                assert_eq!(reference, ref_id);
                assert_eq!(k, kind);
            }
            other => panic!("expected NotCitable({kind:?}), got {other:?}"),
        }
    }
}

#[test]
fn test_speculative_bases_must_be_pending_and_enter_gamma() {
    // Etapa 5.8 (ADR 0058): base especulativa só com kind Pending; citação continua
    // exigindo kind citável, inclusive dentro de generate_speculative.
    let vocab = base_vocab();
    let goal = formula_true();
    let (cite, base) = (make_ref(1), make_ref(2));
    let mut ledger = FakeLedger::new(7);
    ledger.insert_entry(
        cite,
        EntryKind::Verified,
        7,
        false,
        vocab.clone(),
        formula_true(),
    );
    ledger.insert_entry(
        base,
        EntryKind::Pending,
        7,
        false,
        vocab.clone(),
        formula_false(),
    );
    let proposal = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &[cite],
    };
    let obligation = generate_speculative(proposal, &[base], &ledger).unwrap();
    assert_eq!(
        obligation.premises(),
        &[(cite, formula_true()), (base, formula_false())]
    );
    // A mesma referência como citação comum continua não citável.
    let pending_cited = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &[base],
    };
    assert!(matches!(
        generate_speculative(pending_cited, &[], &ledger),
        Err(Reject::NotCitable { reference, kind: EntryKind::Pending }) if reference == base
    ));
    // Qualquer kind diferente de Pending é recusado como base, inclusive citáveis.
    for kind in [
        EntryKind::Fact,
        EntryKind::Policy,
        EntryKind::Verified,
        EntryKind::Assumption,
        EntryKind::Refuted,
        EntryKind::Unknown,
        EntryKind::Rejected,
        EntryKind::Invalidated,
        EntryKind::Proposal,
        EntryKind::Suspended,
    ] {
        let mut ledger = FakeLedger::new(7);
        ledger.insert_entry(base, kind, 7, false, vocab.clone(), formula_true());
        let proposal = Proposal {
            vocabulary: &vocab,
            goal: &goal,
            cites: &[],
        };
        assert!(
            matches!(
                generate_speculative(proposal, &[base], &ledger),
                Err(Reject::NotCitable { reference, kind: k }) if reference == base && k == kind
            ),
            "{kind:?}"
        );
    }
    // Época e retratação valem para a base como para citações.
    let mut stale = FakeLedger::new(8);
    stale.insert_entry(
        base,
        EntryKind::Pending,
        7,
        false,
        vocab.clone(),
        formula_true(),
    );
    let mut retracted = FakeLedger::new(7);
    retracted.insert_entry(
        base,
        EntryKind::Pending,
        7,
        true,
        vocab.clone(),
        formula_true(),
    );
    let empty = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &[],
    };
    assert!(matches!(
        generate_speculative(empty, &[base], &stale),
        Err(Reject::InvalidEpoch { .. })
    ));
    let empty = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &[],
    };
    assert!(matches!(
        generate_speculative(empty, &[base], &retracted),
        Err(Reject::Retracted(r)) if r == base
    ));
}

#[test]
fn test_reject_unknown_citation() {
    let vocab = base_vocab();
    let goal = formula_true();
    let ledger = FakeLedger::new(1);
    let missing_ref = make_ref(99);

    let proposal = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &[missing_ref],
    };

    let err = generate(proposal, &ledger).unwrap_err();
    assert_eq!(err, Reject::UnknownCitation(missing_ref));
}

#[test]
fn test_reject_retracted() {
    let vocab = base_vocab();
    let goal = formula_true();
    let mut ledger = FakeLedger::new(1);
    let ref_id = make_ref(7);
    ledger.insert_entry(
        ref_id,
        EntryKind::Fact,
        1,
        true, // retracted!
        vocab.clone(),
        formula_true(),
    );

    let proposal = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &[ref_id],
    };

    let err = generate(proposal, &ledger).unwrap_err();
    assert_eq!(err, Reject::Retracted(ref_id));
}

#[test]
fn test_reject_invalid_epoch() {
    let vocab = base_vocab();
    let goal = formula_true();
    let mut ledger = FakeLedger::new(10);
    let ref_id = make_ref(8);
    ledger.insert_entry(
        ref_id,
        EntryKind::Verified,
        9, // actual epoch 9 != current epoch 10
        false,
        vocab.clone(),
        formula_true(),
    );

    let proposal = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &[ref_id],
    };

    let err = generate(proposal, &ledger).unwrap_err();
    assert_eq!(
        err,
        Reject::InvalidEpoch {
            reference: ref_id,
            expected: 10,
            actual: 9,
        }
    );
}

#[test]
fn test_reject_ledger_errors() {
    let vocab = base_vocab();
    let goal = formula_true();

    for ledger_err in [LedgerError::Unavailable, LedgerError::InvalidRecord] {
        let mut ledger = FakeLedger::new(1);
        let ref_id = make_ref(15);
        ledger.insert_error(ref_id, ledger_err);

        let proposal = Proposal {
            vocabulary: &vocab,
            goal: &goal,
            cites: &[ref_id],
        };

        let err = generate(proposal, &ledger).unwrap_err();
        assert_eq!(
            err,
            Reject::Ledger {
                reference: ref_id,
                error: ledger_err,
            }
        );
    }
}

#[test]
fn test_reject_goal_sort() {
    let vocab = base_vocab();
    // Formula referencing undeclared symbol
    let invalid_symbol = symbol(
        "undeclared_sym",
        SymbolKind::Pred,
        vec![sort("U")],
        SymbolResult::Bool,
    );
    let invalid_goal = Formula::new(
        vec![FormulaNode::Pred {
            symbol: invalid_symbol,
            args: vec![term_const_u()],
        }],
        Limits::default(),
    )
    .unwrap();

    let ledger = FakeLedger::new(1);
    let proposal = Proposal {
        vocabulary: &vocab,
        goal: &invalid_goal,
        cites: &[],
    };

    let err = generate(proposal, &ledger).unwrap_err();
    assert_eq!(
        err,
        Reject::GoalSort(logos_air::SortError {
            path: logos_air::SortPath {
                formula_node: 0,
                term: None
            },
            kind: logos_air::SortErrorKind::UnknownSymbol,
        })
    );
}

#[test]
fn test_reject_premise_sort() {
    let vocab = base_vocab();
    let goal = formula_true();
    let mut ledger = FakeLedger::new(1);

    let invalid_symbol = symbol(
        "undeclared_sym",
        SymbolKind::Pred,
        vec![sort("U")],
        SymbolResult::Bool,
    );
    let invalid_premise_formula = Formula::new(
        vec![FormulaNode::Pred {
            symbol: invalid_symbol,
            args: vec![term_const_u()],
        }],
        Limits::default(),
    )
    .unwrap();

    let ref_id = make_ref(20);
    ledger.insert_entry(
        ref_id,
        EntryKind::Fact,
        1,
        false,
        vocab.clone(),
        invalid_premise_formula,
    );

    let proposal = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &[ref_id],
    };

    let err = generate(proposal, &ledger).unwrap_err();
    assert_eq!(
        err,
        Reject::PremiseSort {
            reference: ref_id,
            error: logos_air::SortError {
                path: logos_air::SortPath {
                    formula_node: 0,
                    term: None
                },
                kind: logos_air::SortErrorKind::UnknownSymbol,
            },
        }
    );
}

#[test]
fn test_directional_vocabulary_compatibility() {
    let base = base_vocab();
    let extended = extended_vocab();
    let goal = formula_true();

    // Caso A: Alvo = extended, Origem = base.
    // Extended preserva base => Compatível => Sucesso!
    {
        let mut ledger = FakeLedger::new(1);
        let ref_id = make_ref(1);
        ledger.insert_entry(
            ref_id,
            EntryKind::Fact,
            1,
            false,
            base.clone(),
            formula_true(),
        );

        let proposal = Proposal {
            vocabulary: &extended,
            goal: &goal,
            cites: &[ref_id],
        };
        let obligation = generate(proposal, &ledger).unwrap();
        assert_eq!(obligation.vocab(), logos_crypto::hash(&extended).unwrap());
        assert_eq!(obligation.premises(), &[(ref_id, formula_true())]);
    }

    // Caso B: Alvo = base, Origem = extended.
    // Base NÃO preserva extended (falta 'W') => Incompatível => Rejeição!
    {
        let mut ledger = FakeLedger::new(1);
        let ref_id = make_ref(2);
        ledger.insert_entry(
            ref_id,
            EntryKind::Fact,
            1,
            false,
            extended.clone(),
            formula_true(),
        );

        let proposal = Proposal {
            vocabulary: &base,
            goal: &goal,
            cites: &[ref_id],
        };
        let err = generate(proposal, &ledger).unwrap_err();
        assert_eq!(err, Reject::IncompatibleVocabulary(ref_id));
    }
}

#[test]
fn test_stable_canonical_order_and_deduplication() {
    let vocab = base_vocab();
    let goal = formula_true();
    let mut ledger = FakeLedger::new(1);

    let ref_a = make_ref(10);
    let ref_b = make_ref(20);
    let ref_c = make_ref(30);

    ledger.insert_entry(
        ref_a,
        EntryKind::Fact,
        1,
        false,
        vocab.clone(),
        formula_true(),
    );
    ledger.insert_entry(
        ref_b,
        EntryKind::Policy,
        1,
        false,
        vocab.clone(),
        formula_true(),
    );
    ledger.insert_entry(
        ref_c,
        EntryKind::Verified,
        1,
        false,
        vocab.clone(),
        formula_true(),
    );

    // Proposta com ordem arbitrária e repetições: [c, a, b, a, c]
    let proposal1 = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &[ref_c, ref_a, ref_b, ref_a, ref_c],
    };

    let obligation1 = generate(proposal1, &ledger).unwrap();

    // Premissas devem estar estritamente na ordem crescente das referências: [a, b, c]
    let refs_in_ob: Vec<PremiseRef> = obligation1.premises().iter().map(|(r, _)| *r).collect();
    assert_eq!(refs_in_ob, vec![ref_a, ref_b, ref_c]);

    // Lookup deve ter sido chamado exatamente uma vez por referência distinta (3 vezes total)
    assert_eq!(ledger.lookups(), 3);

    // Proposta já ordenada: [a, b, c]
    let proposal2 = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &[ref_a, ref_b, ref_c],
    };
    let obligation2 = generate(proposal2, &ledger).unwrap();

    // Bytes e hash devem ser rigorosamente idênticos
    let bytes1 = to_canon(&obligation1).unwrap();
    let bytes2 = to_canon(&obligation2).unwrap();
    assert_eq!(bytes1, bytes2);
    assert_eq!(obligation1.hash().unwrap(), obligation2.hash().unwrap());
}

#[test]
fn test_deterministic_precedence_multiple_invalid_citations() {
    let vocab = base_vocab();
    let goal = formula_true();
    let mut ledger = FakeLedger::new(1);

    let ref_small = make_ref(5);
    let ref_large = make_ref(50);

    // ref_small é UnknownCitation (não inserido no ledger)
    // ref_large é Retracted
    ledger.insert_entry(
        ref_large,
        EntryKind::Fact,
        1,
        true, // retracted
        vocab.clone(),
        formula_true(),
    );

    // Proposta 1: citas na ordem [large, small]
    let proposal1 = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &[ref_large, ref_small],
    };
    let err1 = generate(proposal1, &ledger).unwrap_err();

    // Proposta 2: citas na ordem [small, large]
    let proposal2 = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &[ref_small, ref_large],
    };
    let err2 = generate(proposal2, &ledger).unwrap_err();

    // Ambas devem acusar deterministamente ref_small primeiro (menor referência ordenada)
    assert_eq!(err1, Reject::UnknownCitation(ref_small));
    assert_eq!(err2, Reject::UnknownCitation(ref_small));
}

#[test]
fn test_precedence_goal_before_citations() {
    let vocab = base_vocab();
    // Objetivo inválido
    let invalid_goal = Formula::new(
        vec![FormulaNode::Pred {
            symbol: symbol(
                "nonexistent",
                SymbolKind::Pred,
                vec![sort("U")],
                SymbolResult::Bool,
            ),
            args: vec![term_const_u()],
        }],
        Limits::default(),
    )
    .unwrap();

    let ledger = FakeLedger::new(1);
    // Citação inexistente
    let missing_ref = make_ref(99);

    let proposal = Proposal {
        vocabulary: &vocab,
        goal: &invalid_goal,
        cites: &[missing_ref],
    };

    // GoalSort tem precedência absoluta sobre UnknownCitation
    let err = generate(proposal, &ledger).unwrap_err();
    assert!(matches!(err, Reject::GoalSort(_)));
}

#[test]
fn test_precedence_within_entry() {
    let vocab = base_vocab();
    let other_vocab = extended_vocab();
    let goal = formula_true();

    // 1. Retracted antes de InvalidEpoch
    {
        let mut ledger = FakeLedger::new(10);
        let ref_id = make_ref(1);
        ledger.insert_entry(
            ref_id,
            EntryKind::Fact,
            9,    // época errada
            true, // retratado
            vocab.clone(),
            formula_true(),
        );

        let proposal = Proposal {
            vocabulary: &vocab,
            goal: &goal,
            cites: &[ref_id],
        };
        let err = generate(proposal, &ledger).unwrap_err();
        assert_eq!(err, Reject::Retracted(ref_id));
    }

    // 2. InvalidEpoch antes de NotCitable
    {
        let mut ledger = FakeLedger::new(10);
        let ref_id = make_ref(2);
        ledger.insert_entry(
            ref_id,
            EntryKind::Pending, // não citável
            9,                  // época errada
            false,
            vocab.clone(),
            formula_true(),
        );

        let proposal = Proposal {
            vocabulary: &vocab,
            goal: &goal,
            cites: &[ref_id],
        };
        let err = generate(proposal, &ledger).unwrap_err();
        assert_eq!(
            err,
            Reject::InvalidEpoch {
                reference: ref_id,
                expected: 10,
                actual: 9,
            }
        );
    }

    // 3. NotCitable antes de IncompatibleVocabulary
    {
        let mut ledger = FakeLedger::new(10);
        let ref_id = make_ref(3);
        ledger.insert_entry(
            ref_id,
            EntryKind::Pending, // não citável
            10,                 // época correta
            false,
            other_vocab.clone(), // vocabulário incompatível com base
            formula_true(),
        );

        let proposal = Proposal {
            vocabulary: &vocab,
            goal: &goal,
            cites: &[ref_id],
        };
        let err = generate(proposal, &ledger).unwrap_err();
        assert_eq!(
            err,
            Reject::NotCitable {
                reference: ref_id,
                kind: EntryKind::Pending,
            }
        );
    }
}

#[test]
fn test_ref_vs_formula_identity() {
    let vocab = base_vocab();
    let goal = formula_true();
    let mut ledger = FakeLedger::new(1);

    let ref_1 = make_ref(1);
    let ref_2 = make_ref(2);

    // Duas referências distintas apontando para exatamente a mesma fórmula
    let shared_formula = formula_pred_p(term_const_u());
    ledger.insert_entry(
        ref_1,
        EntryKind::Fact,
        1,
        false,
        vocab.clone(),
        shared_formula.clone(),
    );
    ledger.insert_entry(
        ref_2,
        EntryKind::Fact,
        1,
        false,
        vocab.clone(),
        shared_formula,
    );

    let proposal = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &[ref_1, ref_2],
    };

    let obligation = generate(proposal, &ledger).unwrap();
    // Ambas as referências são preservadas; não colapsam por igualdade de fórmula
    assert_eq!(obligation.premises().len(), 2);
    assert_eq!(obligation.premises()[0].0, ref_1);
    assert_eq!(obligation.premises()[1].0, ref_2);
}

#[test]
fn test_epoch_excluded_from_obligation_identity() {
    let vocab = base_vocab();
    let goal = formula_true();
    let ref_1 = make_ref(1);
    let ref_2 = make_ref(2);

    // Ledger na época 5
    let mut ledger_epoch5 = FakeLedger::new(5);
    ledger_epoch5.insert_entry(
        ref_1,
        EntryKind::Fact,
        5,
        false,
        vocab.clone(),
        formula_true(),
    );
    ledger_epoch5.insert_entry(
        ref_2,
        EntryKind::Policy,
        5,
        false,
        vocab.clone(),
        formula_true(),
    );

    // Ledger na época 99
    let mut ledger_epoch99 = FakeLedger::new(99);
    ledger_epoch99.insert_entry(
        ref_1,
        EntryKind::Fact,
        99,
        false,
        vocab.clone(),
        formula_true(),
    );
    ledger_epoch99.insert_entry(
        ref_2,
        EntryKind::Policy,
        99,
        false,
        vocab.clone(),
        formula_true(),
    );

    let proposal = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &[ref_1, ref_2],
    };

    let ob_epoch5 = generate(proposal.clone(), &ledger_epoch5).unwrap();
    let ob_epoch99 = generate(proposal, &ledger_epoch99).unwrap();

    // Canon bytes e hash são idênticos: época governa aplicabilidade, não identidade da obrigação
    assert_eq!(
        to_canon(&ob_epoch5).unwrap(),
        to_canon(&ob_epoch99).unwrap()
    );
    assert_eq!(ob_epoch5.hash().unwrap(), ob_epoch99.hash().unwrap());
}

#[test]
fn test_false_goal_and_contradictory_premises_accepted_as_obligation_not_proof() {
    let vocab = base_vocab();
    let mut ledger = FakeLedger::new(1);

    let ref_false = make_ref(1);
    // Premissa com falsidade explícita
    ledger.insert_entry(
        ref_false,
        EntryKind::Assumption,
        1,
        false,
        vocab.clone(),
        formula_false(),
    );

    // Meta com falsidade explícita
    let goal_false = formula_false();

    let proposal = Proposal {
        vocabulary: &vocab,
        goal: &goal_false,
        cites: &[ref_false],
    };

    // Gerador de obrigação constrói o sequente formal puro, não tenta provar ou verificar consistência
    let obligation = generate(proposal, &ledger).unwrap();
    assert_eq!(obligation.premises().len(), 1);
    assert_eq!(obligation.premises()[0].1, formula_false());
    assert_eq!(obligation.goal(), &formula_false());
}
