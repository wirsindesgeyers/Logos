//! Exercício reproduzível da API pública de `logos-obligation`.
//!
//! Comando: `cargo run -p logos-obligation --example obligation_smoke`.
//!
//! Demonstra:
//! 1. Geração bem-sucedida com reordenação e deduplicação de citações;
//! 2. Exclusão da época na identidade canônica e hash da obrigação (invariância temporal);
//! 3. Rejeições determinísticas: UnknownCitation, NotCitable (Pending/Refuted),
//!    Retracted, InvalidEpoch, IncompatibleVocabulary, GoalSort, PremiseSort, Ledger;
//! 4. Precedência determinística guiada pela menor referência ordenada;
//! 5. Saída de terminal normatizada `OBLIGATION_SMOKE_OK`.

use std::collections::HashMap;
use std::error::Error;

use logos_air::{
    Formula, FormulaNode, Limits, LogicId, Sort, SortDecl, SortDomain, Symbol, SymbolDecl,
    SymbolKind, SymbolResult, Term, TermNode, Vocabulary,
};
use logos_canon::{Text, to_canon};
use logos_obligation::{
    EntryKind, LedgerEntry, LedgerError, LedgerView, Obligation, PremiseRef, Proposal, Reject,
    generate,
};

fn require(cond: bool, msg: &str) -> Result<(), Box<dyn Error>> {
    if cond { Ok(()) } else { Err(msg.into()) }
}

struct MockView<'a> {
    epoch: u64,
    entries: HashMap<PremiseRef, Result<Option<LedgerEntry<'a>>, LedgerError>>,
}

impl<'a> LedgerView for MockView<'a> {
    fn current_epoch(&self) -> u64 {
        self.epoch
    }

    fn lookup(&self, reference: PremiseRef) -> Result<Option<LedgerEntry<'_>>, LedgerError> {
        match self.entries.get(&reference) {
            Some(Ok(entry)) => Ok(entry.clone()),
            Some(Err(err)) => Err(*err),
            None => Ok(None),
        }
    }
}

fn make_ref(b: u8) -> PremiseRef {
    PremiseRef::from_event_hash([b; 32])
}

fn build_vocab() -> Result<(Vocabulary, Symbol, Symbol), Box<dyn Error>> {
    let s_item = Sort {
        name: Text::new("Item")?,
    };
    let sd_item = SortDecl {
        name: Text::new("Item")?,
        domain: SortDomain::Open,
    };
    let sym_c = Symbol {
        name: Text::new("c")?,
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(s_item.clone()),
    };
    let sym_p = Symbol {
        name: Text::new("P")?,
        kind: SymbolKind::Pred,
        arity: vec![s_item.clone()],
        result: SymbolResult::Bool,
    };
    let syd_c = SymbolDecl {
        signature: sym_c.clone(),
        grounding: logos_air::Grounding::Glossed(Text::new("c const")?),
        gloss_pt: Text::new("c const pt")?,
    };
    let syd_p = SymbolDecl {
        signature: sym_p.clone(),
        grounding: logos_air::Grounding::Glossed(Text::new("p pred")?),
        gloss_pt: Text::new("p pred pt")?,
    };
    let vocab = Vocabulary::new(1, vec![sd_item], vec![syd_c, syd_p])?;
    Ok((vocab, sym_c, sym_p))
}

fn main() -> Result<(), Box<dyn Error>> {
    let (target_vocab, sym_c, sym_p) = build_vocab()?;

    let term_c = Term::new(
        vec![TermNode::App {
            symbol: sym_c.clone(),
            args: 0,
        }],
        Limits::default(),
    )?;
    let f_pc = Formula::new(
        vec![FormulaNode::Pred {
            symbol: sym_p.clone(),
            args: vec![term_c.clone()],
        }],
        Limits::default(),
    )?;
    let goal = Formula::new(
        vec![
            FormulaNode::Implies,
            FormulaNode::Pred {
                symbol: sym_p.clone(),
                args: vec![term_c.clone()],
            },
            FormulaNode::Pred {
                symbol: sym_p.clone(),
                args: vec![term_c],
            },
        ],
        Limits::default(),
    )?;

    let ref_a = make_ref(0x10);
    let ref_b = make_ref(0x20);
    let ref_c = make_ref(0x30);

    // --- 1. Good case: ordenação e deduplicação ---
    let mut good_entries = HashMap::new();
    good_entries.insert(
        ref_a,
        Ok(Some(LedgerEntry {
            kind: EntryKind::Fact,
            epoch: 1,
            retracted: false,
            vocabulary: &target_vocab,
            formula: &f_pc,
        })),
    );
    good_entries.insert(
        ref_b,
        Ok(Some(LedgerEntry {
            kind: EntryKind::Policy,
            epoch: 1,
            retracted: false,
            vocabulary: &target_vocab,
            formula: &f_pc,
        })),
    );
    good_entries.insert(
        ref_c,
        Ok(Some(LedgerEntry {
            kind: EntryKind::Verified,
            epoch: 1,
            retracted: false,
            vocabulary: &target_vocab,
            formula: &f_pc,
        })),
    );

    let view_1 = MockView {
        epoch: 1,
        entries: good_entries.clone(),
    };

    // Citações embaralhadas com duplicatas: [ref_c, ref_a, ref_b, ref_c, ref_a]
    let cites_scrambled = [ref_c, ref_a, ref_b, ref_c, ref_a];
    let prop_scrambled = Proposal {
        vocabulary: &target_vocab,
        goal: &goal,
        cites: &cites_scrambled,
    };

    let ob_1: Obligation = generate(prop_scrambled, &view_1)?;
    require(
        ob_1.logic() == LogicId::FolMsEqV1,
        "logic deve ser FOL-ms-eq/v1",
    )?;
    require(ob_1.encoder_version() == 1, "encoder_version deve ser 1")?;
    require(
        ob_1.premises().len() == 3,
        "deduplicacao deve deixar 3 premissas",
    )?;
    require(
        ob_1.premises()[0].0 == ref_a
            && ob_1.premises()[1].0 == ref_b
            && ob_1.premises()[2].0 == ref_c,
        "premissas devem estar ordenadas estritamente pelo digest de evento",
    )?;

    // --- 2. Invariância temporal: mesma proposta sob época diferente gera mesma obrigação e hash ---
    let mut entries_epoch_100 = HashMap::new();
    entries_epoch_100.insert(
        ref_a,
        Ok(Some(LedgerEntry {
            kind: EntryKind::Fact,
            epoch: 100,
            retracted: false,
            vocabulary: &target_vocab,
            formula: &f_pc,
        })),
    );
    entries_epoch_100.insert(
        ref_b,
        Ok(Some(LedgerEntry {
            kind: EntryKind::Policy,
            epoch: 100,
            retracted: false,
            vocabulary: &target_vocab,
            formula: &f_pc,
        })),
    );
    entries_epoch_100.insert(
        ref_c,
        Ok(Some(LedgerEntry {
            kind: EntryKind::Verified,
            epoch: 100,
            retracted: false,
            vocabulary: &target_vocab,
            formula: &f_pc,
        })),
    );
    let view_100 = MockView {
        epoch: 100,
        entries: entries_epoch_100,
    };

    let prop_sorted = Proposal {
        vocabulary: &target_vocab,
        goal: &goal,
        cites: &[ref_a, ref_b, ref_c],
    };
    let ob_100 = generate(prop_sorted, &view_100)?;

    let bytes_1 = to_canon(&ob_1)?;
    let bytes_100 = to_canon(&ob_100)?;
    require(
        bytes_1 == bytes_100,
        "bytes canonicos devem ser independentes da epoca",
    )?;

    let hash_1 = logos_crypto::hash(&ob_1);
    let hash_100 = logos_crypto::hash(&ob_100);
    require(
        hash_1 == hash_100,
        "h(O) deve ser estritamente identico entre epocas",
    )?;

    // --- 3. Rejeições determinísticas ---

    // 3.1 UnknownCitation
    let ref_unknown = make_ref(0xFF);
    let prop_unknown = Proposal {
        vocabulary: &target_vocab,
        goal: &goal,
        cites: &[ref_unknown],
    };
    match generate(prop_unknown, &view_1) {
        Err(Reject::UnknownCitation(r)) if r == ref_unknown => {}
        other => return Err(format!("esperava UnknownCitation, obteve: {other:?}").into()),
    }

    // 3.2 NotCitable (Pending e Refuted)
    let ref_pending = make_ref(0x50);
    let mut pending_entries = good_entries.clone();
    pending_entries.insert(
        ref_pending,
        Ok(Some(LedgerEntry {
            kind: EntryKind::Pending,
            epoch: 1,
            retracted: false,
            vocabulary: &target_vocab,
            formula: &f_pc,
        })),
    );
    let view_pending = MockView {
        epoch: 1,
        entries: pending_entries,
    };
    let prop_pending = Proposal {
        vocabulary: &target_vocab,
        goal: &goal,
        cites: &[ref_pending],
    };
    match generate(prop_pending, &view_pending) {
        Err(Reject::NotCitable { reference, kind })
            if reference == ref_pending && kind == EntryKind::Pending => {}
        other => return Err(format!("esperava NotCitable(Pending), obteve: {other:?}").into()),
    }

    let ref_refuted = make_ref(0x51);
    let mut refuted_entries = good_entries.clone();
    refuted_entries.insert(
        ref_refuted,
        Ok(Some(LedgerEntry {
            kind: EntryKind::Refuted,
            epoch: 1,
            retracted: false,
            vocabulary: &target_vocab,
            formula: &f_pc,
        })),
    );
    let view_refuted = MockView {
        epoch: 1,
        entries: refuted_entries,
    };
    let prop_refuted = Proposal {
        vocabulary: &target_vocab,
        goal: &goal,
        cites: &[ref_refuted],
    };
    match generate(prop_refuted, &view_refuted) {
        Err(Reject::NotCitable { reference, kind })
            if reference == ref_refuted && kind == EntryKind::Refuted => {}
        other => return Err(format!("esperava NotCitable(Refuted), obteve: {other:?}").into()),
    }

    // 3.3 Retracted
    let ref_retracted = make_ref(0x60);
    let mut retracted_entries = good_entries.clone();
    retracted_entries.insert(
        ref_retracted,
        Ok(Some(LedgerEntry {
            kind: EntryKind::Fact,
            epoch: 1,
            retracted: true,
            vocabulary: &target_vocab,
            formula: &f_pc,
        })),
    );
    let view_retracted = MockView {
        epoch: 1,
        entries: retracted_entries,
    };
    let prop_retracted = Proposal {
        vocabulary: &target_vocab,
        goal: &goal,
        cites: &[ref_retracted],
    };
    match generate(prop_retracted, &view_retracted) {
        Err(Reject::Retracted(r)) if r == ref_retracted => {}
        other => return Err(format!("esperava Retracted, obteve: {other:?}").into()),
    }

    // 3.4 InvalidEpoch
    let ref_bad_epoch = make_ref(0x70);
    let mut bad_epoch_entries = good_entries.clone();
    bad_epoch_entries.insert(
        ref_bad_epoch,
        Ok(Some(LedgerEntry {
            kind: EntryKind::Fact,
            epoch: 2, // corrente é 1
            retracted: false,
            vocabulary: &target_vocab,
            formula: &f_pc,
        })),
    );
    let view_bad_epoch = MockView {
        epoch: 1,
        entries: bad_epoch_entries,
    };
    let prop_bad_epoch = Proposal {
        vocabulary: &target_vocab,
        goal: &goal,
        cites: &[ref_bad_epoch],
    };
    match generate(prop_bad_epoch, &view_bad_epoch) {
        Err(Reject::InvalidEpoch {
            reference,
            expected: 1,
            actual: 2,
        }) if reference == ref_bad_epoch => {}
        other => return Err(format!("esperava InvalidEpoch, obteve: {other:?}").into()),
    }

    // 3.5 IncompatibleVocabulary
    // Vocabulário de origem possui sorts e símbolos ausentes no vocabulário alvo
    let sd_extra = SortDecl {
        name: Text::new("ExtraSort")?,
        domain: SortDomain::Open,
    };
    let origin_incompat_vocab = Vocabulary::new(1, vec![sd_extra], Vec::new())?;
    let ref_incompat = make_ref(0x80);
    let mut incompat_entries = good_entries.clone();
    incompat_entries.insert(
        ref_incompat,
        Ok(Some(LedgerEntry {
            kind: EntryKind::Fact,
            epoch: 1,
            retracted: false,
            vocabulary: &origin_incompat_vocab,
            formula: &f_pc,
        })),
    );
    let view_incompat = MockView {
        epoch: 1,
        entries: incompat_entries,
    };
    let prop_incompat = Proposal {
        vocabulary: &target_vocab,
        goal: &goal,
        cites: &[ref_incompat],
    };
    match generate(prop_incompat, &view_incompat) {
        Err(Reject::IncompatibleVocabulary(r)) if r == ref_incompat => {}
        other => return Err(format!("esperava IncompatibleVocabulary, obteve: {other:?}").into()),
    }

    // 3.6 GoalSort
    // Goal possui termo livre de De Bruijn sem quantificador (erro de escopo)
    let bad_goal = Formula::new(
        vec![FormulaNode::Pred {
            symbol: sym_p,
            args: vec![Term::new(
                vec![TermNode::Var {
                    index: 0,
                    sort: Sort {
                        name: Text::new("Item")?,
                    },
                }],
                Limits::default(),
            )?],
        }],
        Limits::default(),
    )?;
    let prop_bad_goal = Proposal {
        vocabulary: &target_vocab,
        goal: &bad_goal,
        cites: &[],
    };
    match generate(prop_bad_goal, &view_1) {
        Err(Reject::GoalSort(_)) => {}
        other => return Err(format!("esperava GoalSort, obteve: {other:?}").into()),
    }

    // 3.7 PremiseSort
    let bad_premise_formula = bad_goal;
    let ref_bad_premise = make_ref(0x95);
    let mut bad_premise_entries = good_entries.clone();
    bad_premise_entries.insert(
        ref_bad_premise,
        Ok(Some(LedgerEntry {
            kind: EntryKind::Fact,
            epoch: 1,
            retracted: false,
            vocabulary: &target_vocab,
            formula: &bad_premise_formula,
        })),
    );
    let view_bad_premise = MockView {
        epoch: 1,
        entries: bad_premise_entries,
    };
    let prop_bad_premise = Proposal {
        vocabulary: &target_vocab,
        goal: &goal,
        cites: &[ref_bad_premise],
    };
    match generate(prop_bad_premise, &view_bad_premise) {
        Err(Reject::PremiseSort { reference, .. }) if reference == ref_bad_premise => {}
        other => return Err(format!("esperava PremiseSort, obteve: {other:?}").into()),
    }

    // 3.8 LedgerError
    let ref_ledger_err = make_ref(0xA0);
    let mut ledger_err_entries = good_entries.clone();
    ledger_err_entries.insert(ref_ledger_err, Err(LedgerError::Unavailable));
    let view_ledger_err = MockView {
        epoch: 1,
        entries: ledger_err_entries,
    };
    let prop_ledger_err = Proposal {
        vocabulary: &target_vocab,
        goal: &goal,
        cites: &[ref_ledger_err],
    };
    match generate(prop_ledger_err, &view_ledger_err) {
        Err(Reject::Ledger {
            reference,
            error: LedgerError::Unavailable,
        }) if reference == ref_ledger_err => {}
        other => return Err(format!("esperava Ledger(Unavailable), obteve: {other:?}").into()),
    }

    // 4. Precedência determinística: menor referência ordenada diagnostica primeiro
    // Citações: [ref_large (unknown), ref_small (unknown)]
    // O erro reportado deve ser ref_small
    let ref_small = make_ref(0x05);
    let ref_large = make_ref(0x99);
    let prop_precedence = Proposal {
        vocabulary: &target_vocab,
        goal: &goal,
        cites: &[ref_large, ref_small],
    };
    match generate(prop_precedence, &view_1) {
        Err(Reject::UnknownCitation(r)) if r == ref_small => {}
        other => {
            return Err(format!("esperava menor referencia ref_small, obteve: {other:?}").into());
        }
    }

    println!("OBLIGATION_SMOKE_OK");
    Ok(())
}
