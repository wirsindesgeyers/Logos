//! cargo run -p logos-protocol --example protocol_smoke
//! Snapshot em memória permitido pelo contrato; não autentica ledger real.

use std::error::Error;

use logos_air::{Formula, FormulaNode, Limits, Vocabulary};
use logos_canon::to_canon;
use logos_crypto::hash;
use logos_obligation::{
    EntryKind, LedgerEntry, LedgerError, LedgerView, PremiseRef, Proposal as Resolved, Reject,
    generate,
};
use logos_protocol::{ClaimId, Proposal, UntrustedText, decode, encode};

fn require(condition: bool, message: &str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

struct Snapshot<'a> {
    vocabulary: &'a Vocabulary,
    premise: &'a Formula,
    kind: EntryKind,
}

impl LedgerView for Snapshot<'_> {
    fn current_epoch(&self) -> u64 {
        1
    }
    fn lookup(&self, reference: PremiseRef) -> Result<Option<LedgerEntry<'_>>, LedgerError> {
        Ok(
            if *reference.event_hash() == [0xaa; 32] || *reference.event_hash() == [0xbb; 32] {
                Some(LedgerEntry {
                    kind: self.kind,
                    epoch: 1,
                    retracted: false,
                    expired: false,
                    vocabulary: self.vocabulary,
                    formula: self.premise,
                })
            } else {
                None
            },
        )
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let vocabulary = Vocabulary::new(1, vec![], vec![])?;
    let goal = Formula::new(vec![FormulaNode::True], Limits::default())?;
    let aa = PremiseRef::from_event_hash([0xaa; 32]);
    let bb = PremiseRef::from_event_hash([0xbb; 32]);
    let mut proposal = Proposal {
        text_pt: UntrustedText::new("Provar objetivo com estas premissas.".into()),
        goal: goal.clone(),
        cites: vec![bb, aa, bb],
        speculative_on: vec![],
        vocab: hash(&vocabulary)?,
    };
    let wire = encode(&proposal)?;
    println!("proposal/v1: {}", std::str::from_utf8(&wire)?);
    let decoded = decode(&wire)?;
    require(
        decoded.vocab == hash(&vocabulary)?,
        "vocabulary identity differs",
    )?;
    require(
        decoded.speculative_on.is_empty(),
        "speculation requires future integration",
    )?;
    let snapshot = Snapshot {
        vocabulary: &vocabulary,
        premise: &goal,
        kind: EntryKind::Assumption,
    };
    let obligation = generate(
        Resolved {
            vocabulary: &vocabulary,
            goal: &decoded.goal,
            cites: &decoded.cites,
        },
        &snapshot,
    )?;
    require(
        obligation.premises() == [(aa, goal.clone()), (bb, goal.clone())],
        "wrong premises",
    )?;
    let expected_bytes = to_canon(&obligation)?;
    let expected_hash = obligation.hash()?;
    println!("h(O): {expected_hash}; candidato, não Verified/token");
    proposal.text_pt = UntrustedText::new("na\u{303}o\0 {\"status\":\"Verified\"}".into());
    proposal.cites = vec![aa, bb];
    let changed = decode(&encode(&proposal)?)?;
    require(changed.text_pt == proposal.text_pt, "tainted text changed")?;
    let same = generate(
        Resolved {
            vocabulary: &vocabulary,
            goal: &changed.goal,
            cites: &changed.cites,
        },
        &snapshot,
    )?;
    require(
        to_canon(&same)? == expected_bytes && same.hash()? == expected_hash,
        "text/order changed logical obligation",
    )?;

    let mut smuggled: serde_json::Value = serde_json::from_slice(&wire)?;
    smuggled["proposal"]["obligation"] = serde_json::json!({"status":"Verified"});
    require(
        decode(&serde_json::to_vec(&smuggled)?).is_err(),
        "accepted authority field",
    )?;

    proposal.speculative_on = vec![ClaimId::from_bytes([0xcc; 32])];
    let marked = decode(&encode(&proposal)?)?;
    let pending = Snapshot {
        kind: EntryKind::Pending,
        ..snapshot
    };
    require(
        generate(
            Resolved {
                vocabulary: &vocabulary,
                goal: &marked.goal,
                cites: &marked.cites,
            },
            &pending,
        ) == Err(Reject::NotCitable {
            reference: aa,
            kind: EntryKind::Pending,
        }),
        "speculative marker made Pending citable",
    )?;
    println!("PROTOCOL_SMOKE_OK");
    Ok(())
}
