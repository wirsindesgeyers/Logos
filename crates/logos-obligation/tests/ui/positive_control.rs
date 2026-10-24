//! Controle positivo de compilação via trybuild.
//!
//! Demonstra o fluxo canônico de construção pública via `generate`,
//! consumo de accessors públicos, codificação Canon (somente saída)
//! e cálculo de hash via trait Domain.

use std::collections::HashMap;

use logos_air::{
    Formula, FormulaNode, Limits, LogicId, Sort, SortDecl, SortDomain, Symbol, SymbolDecl,
    SymbolKind, SymbolResult, Term, TermNode, Vocabulary,
};
use logos_canon::to_canon;
use logos_obligation::{
    EntryKind, LedgerEntry, LedgerError, LedgerView, PremiseRef, Proposal, generate,
};

struct PositiveView<'a> {
    epoch: u64,
    entries: HashMap<PremiseRef, LedgerEntry<'a>>,
}

impl<'a> LedgerView for PositiveView<'a> {
    fn current_epoch(&self) -> u64 {
        self.epoch
    }

    fn lookup(&self, reference: PremiseRef) -> Result<Option<LedgerEntry<'_>>, LedgerError> {
        Ok(self.entries.get(&reference).cloned())
    }
}

fn main() {
    let sort_item = Sort {
        name: logos_canon::Text::new("Item").unwrap(),
    };
    let sort_decl = SortDecl {
        name: logos_canon::Text::new("Item").unwrap(),
        domain: SortDomain::Open,
    };
    let sym_c = Symbol {
        name: logos_canon::Text::new("c").unwrap(),
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(sort_item.clone()),
    };
    let sym_decl = SymbolDecl {
        signature: sym_c.clone(),
        grounding: logos_air::Grounding::Glossed(logos_canon::Text::new("gloss").unwrap()),
        gloss_pt: logos_canon::Text::new("gloss pt").unwrap(),
    };
    let vocab = Vocabulary::new(1, vec![sort_decl], vec![sym_decl]).unwrap();

    let term_c = Term::new(
        vec![TermNode::App {
            symbol: sym_c,
            args: 0,
        }],
        Limits::default(),
    )
    .unwrap();

    let goal = Formula::new(
        vec![FormulaNode::Eq(term_c.clone(), term_c)],
        Limits::default(),
    )
    .unwrap();

    let reference = PremiseRef::from_event_hash([0x42; 32]);
    let mut entries = HashMap::new();
    entries.insert(
        reference,
        LedgerEntry {
            kind: EntryKind::Fact,
            epoch: 1,
            retracted: false,
            expired: false,
            untrusted: false,
            vocabulary: &vocab,
            formula: &goal,
        },
    );

    let view = PositiveView { epoch: 1, entries };
    let cites = [reference];
    let proposal = Proposal {
        vocabulary: &vocab,
        goal: &goal,
        cites: &cites,
    };

    let obligation = generate(proposal, &view).unwrap();

    assert_eq!(obligation.logic(), LogicId::FolMsEqV1);
    assert_eq!(obligation.encoder_version(), 1);
    let _v_hash = obligation.vocab();
    assert_eq!(obligation.premises().len(), 1);
    assert_eq!(obligation.goal(), &goal);

    let _bytes = to_canon(&obligation).unwrap();
    let _hash = logos_crypto::hash(&obligation);
}
