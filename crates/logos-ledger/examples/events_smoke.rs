//! Smoke de assinaturas reais; metadados de check são fixtures, não prova lógica.
use logos_air::{Formula, FormulaNode, Limits, Vocabulary};
use logos_canon::{Array, Canon, Value, to_canon};
use logos_core::{Budget, CheckOutcome};
use logos_crypto::{FileSigner, Hash, Signer, hash};
use logos_ledger::{
    Authority, CheckRecord, Consistency, Event, EventBody, EventProposal, TaintedText,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sementes públicas de exemplo, não credenciais de produção.
    let kernel = FileSigner::from_seed(&[0x31; 32])?;
    let admin = FileSigner::from_seed(&[0x42; 32])?;
    let genesis = Event::sign(
        0,
        0,
        None,
        EventBody::Genesis {
            ledger_version: 1,
            kernel_binary: [0x10; 32],
            kernel_key: kernel.key_id()?,
        },
        &kernel,
    )?;
    genesis.verify(kernel.public_key())?;
    let genesis_hash = hash(&genesis)?;
    let vocabulary = Hash::<Vocabulary>::from_bytes([0x20; 32]);
    let admin_key = admin.key_id()?;
    let authority_body = Value::Array(Array::new(vec![
        Value::Unsigned(1),
        Value::Unsigned(1),
        vocabulary.to_value()?,
        Value::Bool(false),
        admin_key.to_value()?,
    ])?);
    let message = Value::Array(Array::new(vec![
        Value::Unsigned(1),
        Value::Unsigned(1),
        authority_body,
    ])?);
    let authority_bytes = to_canon(&message)?;
    let published_body = EventBody::VocabPublished {
        vocabulary,
        compatible: false,
        authority: Authority {
            key: admin_key,
            sig: admin.sign::<Event>(&authority_bytes)?,
        },
    };
    published_body.verify_authority(admin.public_key())?;
    let published = Event::sign(1, 0, Some(genesis_hash), published_body, &kernel)?;
    published.verify(kernel.public_key())?;
    let proposed = Event::sign(
        2,
        0,
        Some(hash(&published)?),
        EventBody::Proposed {
            proposal: EventProposal {
                text_pt: TaintedText::new("texto tainted cafe\u{301}".into())?,
                goal: Formula::new(vec![FormulaNode::False], Limits::default())?,
                cites: vec![hash(&published)?],
                speculative_on: vec![],
                vocabulary,
            },
            author: admin_key,
        },
        &kernel,
    )?;
    let wire = to_canon(&proposed)?;
    let decoded = Event::decode(&wire)?;
    decoded.verify(kernel.public_key())?;
    if let EventBody::Proposed { proposal, .. } = &decoded.body {
        if proposal.text_pt.as_untrusted_str() != "texto tainted cafe\u{301}" {
            return Err("tainted UTF-8 changed".into());
        }
    } else {
        return Err("proposal body changed".into());
    }
    let checked = Event::sign(
        3,
        0,
        Some(hash(&proposed)?),
        EventBody::Verified {
            check: CheckRecord {
                obligation: Hash::from_bytes([0x50; 32]),
                certificate: Hash::from_bytes([0x51; 32]),
                checker: Hash::from_bytes([0x52; 32]),
                budget: Budget {
                    steps: 100,
                    mem_bytes: 1024,
                    size_bytes: 1024,
                    wall_ms: 0,
                },
                outcome: CheckOutcome::Accepted,
            },
            cited: vec![],
            used: vec![],
            consistency: Consistency::Unknown { check: None },
        },
        &kernel,
    )?;
    checked.verify(kernel.public_key())?;
    let mut tampered = decoded;
    tampered.epoch += 1;
    if tampered.verify(kernel.public_key()).is_ok() {
        return Err("tampered epoch accepted".into());
    }
    if proposed.verify(admin.public_key()).is_ok() {
        return Err("wrong key accepted".into());
    }
    println!(
        "EVENTS_SMOKE_OK genesis={} proposal={} wire={} authority=valid tamper=rejected consistency=unknown (fixtures, not logical authority)",
        genesis_hash,
        hash(&proposed)?,
        wire.len()
    );
    Ok(())
}
