//! CAS opaco: os testes demonstram presença/integridade, não validade de provas.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[allow(dead_code)]
mod support;

use std::cell::Cell;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use logos_air::Vocabulary;
use logos_canon::Text;
use logos_core::{Budget, CheckOutcome, Exhausted};
use logos_crypto::{CryptoError, FileSigner, Hash, PublicKey, SigDomain, Signature, Signer, hash};
use logos_ledger::{
    Authority, Cas, CasError, CasReference, Certificate, CheckRecord, Checker, Consistency, Event,
    EventBody, EventProposal, Ledger, LedgerError, Limits, ModelObject, Route, TaintedText,
    ledger_references,
};
use logos_obligation::Obligation;
use rusqlite::Connection;
use support::{Temp, genesis, signer};

struct Counting {
    inner: FileSigner,
    calls: Cell<usize>,
}
impl Signer for Counting {
    fn public_key(&self) -> &PublicKey {
        self.inner.public_key()
    }
    fn sign<D: SigDomain>(&self, bytes: &[u8]) -> Result<Signature, CryptoError> {
        self.calls.set(self.calls.get() + 1);
        self.inner.sign::<D>(bytes)
    }
}
fn path(root: &Path, reference: CasReference) -> PathBuf {
    let hex: String = reference
        .digest()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    root.join("cas")
        .join(reference.directory())
        .join(&hex[..2])
        .join(&hex[2..])
}
fn store(cas: &Cas, reference: CasReference, bytes: &[u8]) {
    let digest = match reference {
        CasReference::Vocabulary(_) => *cas.put::<Vocabulary>(bytes).unwrap().as_bytes(),
        CasReference::Obligation(_) => *cas.put::<Obligation>(bytes).unwrap().as_bytes(),
        CasReference::Certificate(_) => *cas.put::<Certificate>(bytes).unwrap().as_bytes(),
        CasReference::Checker(_) => *cas.put::<Checker>(bytes).unwrap().as_bytes(),
        CasReference::Model(_) => *cas.put::<ModelObject>(bytes).unwrap().as_bytes(),
    };
    assert_eq!(digest, reference.digest());
}
fn receipt(mut body: EventBody, signer: &impl Signer) -> EventBody {
    let bytes = body.authority_message().unwrap().unwrap();
    let sig = signer.sign::<Event>(&bytes).unwrap();
    match &mut body {
        EventBody::VocabPublished { authority, .. }
        | EventBody::Retracted { authority, .. }
        | EventBody::CheckerRegistered { authority, .. }
        | EventBody::CheckerRevoked { authority, .. } => authority.sig = sig,
        _ => unreachable!(),
    }
    body
}

#[test]
fn every_direct_and_nested_reference_is_required_before_signing_or_sql() {
    let temp = Temp::new();
    let signer = Counting {
        inner: signer(),
        calls: Cell::new(0),
    };
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    let first = hash(&ledger.append(genesis(&signer)).unwrap()).unwrap();
    let cas = ledger.cas();
    let hv = cas.put::<Vocabulary>(&[0x80]).unwrap();
    let ho = cas.put::<Obligation>(&[0x80]).unwrap();
    let cert = cas.put::<Certificate>(b"main").unwrap();
    let other_cert = cas.put::<Certificate>(b"consistency").unwrap();
    let checker = cas.put::<Checker>(b"main").unwrap();
    let other_checker = cas.put::<Checker>(b"consistency").unwrap();
    let model = cas.put::<ModelObject>(&[0x80]).unwrap();
    let check = CheckRecord {
        obligation: ho,
        certificate: cert,
        checker,
        budget: Budget {
            steps: 1,
            mem_bytes: 1,
            size_bytes: 1,
            wall_ms: 0,
        },
        outcome: CheckOutcome::Accepted,
    };
    let other = CheckRecord {
        certificate: other_cert,
        checker: other_checker,
        ..check.clone()
    };
    let objects = [
        (CasReference::Vocabulary(hv), vec![0x80]),
        (CasReference::Obligation(ho), vec![0x80]),
        (CasReference::Certificate(cert), b"main".to_vec()),
        (CasReference::Checker(checker), b"main".to_vec()),
        (
            CasReference::Certificate(other_cert),
            b"consistency".to_vec(),
        ),
        (
            CasReference::Checker(other_checker),
            b"consistency".to_vec(),
        ),
        (CasReference::Model(model), vec![0x80]),
    ];
    let authority = Authority {
        key: signer.key_id().unwrap(),
        sig: signer.sign::<Event>(b"placeholder").unwrap(),
    };
    let v = vec![CasReference::Vocabulary(hv)];
    let c = vec![
        CasReference::Obligation(ho),
        CasReference::Certificate(cert),
        CasReference::Checker(checker),
    ];
    let mut witness = c.clone();
    witness.extend([
        CasReference::Certificate(other_cert),
        CasReference::Checker(other_checker),
        CasReference::Model(model),
    ]);
    let mut unknown_consistency = witness.clone();
    unknown_consistency.pop();
    let mut refuted = c.clone();
    refuted.push(CasReference::Model(model));
    let body_cases = vec![
        (
            receipt(
                EventBody::VocabPublished {
                    vocabulary: hv,
                    compatible: true,
                    authority: authority.clone(),
                },
                &signer,
            ),
            v.clone(),
        ),
        (
            EventBody::Proposed {
                proposal: EventProposal {
                    vocabulary: hv,
                    text_pt: TaintedText::new("fixture".into()).unwrap(),
                    goal: logos_air::Formula::new(
                        vec![logos_air::FormulaNode::True],
                        logos_air::Limits::default(),
                    )
                    .unwrap(),
                    cites: vec![first],
                    speculative_on: vec![[0; 32]],
                },
                author: signer.key_id().unwrap(),
            },
            v.clone(),
        ),
        (
            EventBody::ObligationCreated {
                obligation: ho,
                route: Route::Model,
                artifact: [0; 32],
            },
            vec![CasReference::Obligation(ho)],
        ),
        (
            EventBody::Verified {
                check: check.clone(),
                cited: vec![],
                used: vec![],
                consistency: Consistency::Unknown { check: None },
            },
            c.clone(),
        ),
        (
            EventBody::Verified {
                check: check.clone(),
                cited: vec![],
                used: vec![],
                consistency: Consistency::Witness {
                    model,
                    check: other.clone(),
                },
            },
            witness,
        ),
        (
            EventBody::Verified {
                check: check.clone(),
                cited: vec![],
                used: vec![],
                consistency: Consistency::Unknown {
                    check: Some(CheckRecord {
                        outcome: CheckOutcome::Unknown(Exhausted::Memory),
                        ..other
                    }),
                },
            },
            unknown_consistency,
        ),
        (
            EventBody::Refuted {
                check: check.clone(),
                model,
            },
            refuted,
        ),
        (
            EventBody::UnknownRecorded {
                check: CheckRecord {
                    outcome: CheckOutcome::Unknown(Exhausted::Steps),
                    ..check
                },
                machine: Text::new("fixture").unwrap(),
            },
            c,
        ),
        (
            EventBody::AssumptionDeclared {
                vocabulary: hv,
                formula: logos_air::Formula::new(
                    vec![logos_air::FormulaNode::True],
                    logos_air::Limits::default(),
                )
                .unwrap(),
                scope: Text::new("local").unwrap(),
                author: signer.key_id().unwrap(),
            },
            v.clone(),
        ),
        (
            receipt(
                EventBody::Retracted {
                    target: first,
                    reason: Text::new("fixture").unwrap(),
                    authority: authority.clone(),
                },
                &signer,
            ),
            vec![],
        ),
        (
            EventBody::Invalidated {
                claim: first,
                causes: vec![first],
            },
            vec![],
        ),
        (
            EventBody::EpochAdvanced {
                new_epoch: 1,
                reason: Text::new("fixture").unwrap(),
                policy_root: Some([0x99; 32]),
                vocab: Some(hv),
            },
            v,
        ),
        (
            receipt(
                EventBody::CheckerRegistered {
                    checker,
                    format: Text::new("fixture").unwrap(),
                    authority: authority.clone(),
                },
                &signer,
            ),
            vec![CasReference::Checker(checker)],
        ),
        (
            receipt(
                EventBody::CheckerRevoked {
                    checker,
                    reason: Text::new("fixture").unwrap(),
                    authority,
                },
                &signer,
            ),
            vec![CasReference::Checker(checker)],
        ),
    ];
    let mut checked = 0;
    for (body, expected) in body_cases {
        let expected: BTreeSet<_> = expected.into_iter().collect();
        assert_eq!(body.cas_references(), expected);
        for missing in &expected {
            let before = ledger.head().unwrap();
            let count: i64 = Connection::open(temp.database())
                .unwrap()
                .query_row("SELECT count(*) FROM events", [], |r| r.get(0))
                .unwrap();
            fs::remove_file(path(&temp.0, *missing)).unwrap();
            signer.calls.set(0);
            assert!(
                matches!(ledger.append(body.clone()), Err(LedgerError::MissingObject(actual)) if actual == *missing)
            );
            assert_eq!(
                signer.calls.get(),
                0,
                "missing object must fail before signing"
            );
            assert_eq!(ledger.head().unwrap(), before);
            let after: i64 = Connection::open(temp.database())
                .unwrap()
                .query_row("SELECT count(*) FROM events", [], |r| r.get(0))
                .unwrap();
            assert_eq!(after, count);
            let (_, bytes) = objects.iter().find(|(r, _)| r == missing).unwrap();
            store(ledger.cas(), *missing, bytes);
            checked += 1;
        }
        ledger.append(body).unwrap();
    }
    assert_eq!(checked, 28);
    let head = ledger.head().unwrap();
    drop(ledger);
    let references = ledger_references(&temp.0, signer.public_key()).unwrap();
    assert_eq!(references.head, head);
    assert_eq!(
        references.objects,
        objects.iter().map(|(r, _)| *r).collect()
    );
    assert!(matches!(
        Ledger::open(&temp.0, &signer),
        Err(LedgerError::WriterBusy)
    ));
    drop(references);
    Ledger::open(&temp.0, &signer).unwrap();
}

#[test]
fn corruption_limits_wrong_domain_and_nonregular_paths_are_errors_not_absence() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    ledger.append(genesis(&signer)).unwrap();
    let bytes = [0x80];
    let ho = ledger.cas().put::<Obligation>(&bytes).unwrap();
    let reference = CasReference::Obligation(ho);
    let object = path(&temp.0, reference);
    let body = EventBody::ObligationCreated {
        obligation: ho,
        route: Route::Model,
        artifact: [0; 32],
    };
    let head = ledger.head().unwrap();
    fs::write(&object, b"corrupt").unwrap();
    assert!(matches!(
        ledger.append(body.clone()),
        Err(LedgerError::Cas(CasError::Integrity))
    ));
    let file = fs::OpenOptions::new().write(true).open(&object).unwrap();
    file.set_len(16 * 1024 * 1024 + 1).unwrap();
    drop(file);
    assert!(matches!(
        ledger.append(body.clone()),
        Err(LedgerError::Cas(CasError::TooLarge { .. }))
    ));
    fs::remove_file(&object).unwrap();
    fs::create_dir(&object).unwrap();
    assert!(matches!(
        ledger.append(body.clone()),
        Err(LedgerError::Cas(CasError::Io(_)))
    ));
    fs::remove_dir(&object).unwrap();
    let certificate = ledger.cas().put::<Certificate>(&bytes).unwrap();
    let confused = EventBody::ObligationCreated {
        obligation: Hash::from_bytes(*certificate.as_bytes()),
        route: Route::Model,
        artifact: [0; 32],
    };
    assert!(matches!(
        ledger.append(confused),
        Err(LedgerError::MissingObject(_))
    ));
    assert_eq!(ledger.head().unwrap(), head);
    ledger.cas().put::<Obligation>(&bytes).unwrap();
    ledger.append(body).unwrap();
}

#[test]
fn symlinks_at_object_shard_domain_and_root_are_not_followed() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    ledger.append(genesis(&signer)).unwrap();
    let ho = ledger.cas().put::<Obligation>(&[0x80]).unwrap();
    let object = path(&temp.0, CasReference::Obligation(ho));
    let body = EventBody::ObligationCreated {
        obligation: ho,
        route: Route::Model,
        artifact: [0; 32],
    };
    for original in [
        object.clone(),
        object.parent().unwrap().to_path_buf(),
        temp.0.join("cas/obligation-v1"),
    ] {
        let moved = temp.0.join("moved");
        fs::rename(&original, &moved).unwrap();
        std::os::unix::fs::symlink(&moved, &original).unwrap();
        assert!(matches!(
            ledger.append(body.clone()),
            Err(LedgerError::Cas(CasError::Io(_)))
        ));
        fs::remove_file(&original).unwrap();
        fs::rename(moved, original).unwrap();
    }
    ledger.append(body).unwrap();
    drop(ledger);
    let root = temp.0.join("cas");
    let moved = temp.0.join("moved");
    fs::rename(&root, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &root).unwrap();
    assert!(Ledger::open(&temp.0, &signer).is_err());
    assert!(Cas::open_existing(&root, Limits::default()).is_err());
    fs::remove_file(&root).unwrap();
    fs::rename(moved, root).unwrap();
}

#[test]
fn reference_reader_observes_committed_wal_and_does_not_create_missing_paths() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    let first = ledger.append(genesis(&signer)).unwrap();
    drop(ledger);
    let source = Connection::open(temp.database()).unwrap();
    source.execute_batch("PRAGMA wal_autocheckpoint=0").unwrap();
    let body = support::ordinary(1);
    let event = Event::sign(1, 0, Some(hash(&first).unwrap()), body.clone(), &signer).unwrap();
    source
        .execute(
            "INSERT INTO events(seq,wire) VALUES(1,?1)",
            [logos_canon::to_canon(&event).unwrap()],
        )
        .unwrap();
    let wal = temp.0.join("ledger.sqlite3-wal");
    let wal_bytes = fs::read(&wal).unwrap();
    assert!(!wal_bytes.is_empty());
    let database_bytes = fs::read(temp.database()).unwrap();
    let references = ledger_references(&temp.0, signer.public_key()).unwrap();
    assert_eq!(references.head.unwrap().hash, hash(&event).unwrap());
    assert_eq!(references.objects, body.cas_references());
    assert_eq!(fs::read(&wal).unwrap(), wal_bytes);
    assert_eq!(fs::read(temp.database()).unwrap(), database_bytes);
    drop(references);
    drop(source);
    let absent = temp.0.join("missing");
    assert!(Cas::open_existing(&absent, Limits::default()).is_err());
    assert!(!absent.exists());
    fs::remove_file(temp.0.join("writer.lock")).unwrap();
    assert!(ledger_references(&temp.0, signer.public_key()).is_err());
    assert!(!temp.0.join("writer.lock").exists());
}
