//! Consultas e LedgerView reais; assinaturas de fixtures não provam lógica.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[allow(dead_code)]
mod support;

use std::fs;

use logos_air::{Formula, FormulaNode, Limits as AirLimits, Vocabulary};
use logos_canon::{Text, Value, from_canon, to_canon};
use logos_core::{Budget, CheckOutcome, Exhausted};
use logos_crypto::{FileSigner, Hash, Signer, hash, hash_raw};
use logos_ledger::{
    Authority, Cas, Certificate, CheckRecord, Checker, Consistency, Event, EventBody,
    EventProposal, IndexComparison, Ledger, LedgerError, LedgerIndex, Limits, ModelObject, Route,
    TaintedText,
};
use logos_obligation::{EntryKind, LedgerView, Obligation, PremiseRef, Proposal, Reject, generate};
use rusqlite::{Connection, params};
use support::{Temp, epoch, genesis, signer};

struct Fixture {
    temp: Temp,
    signer: FileSigner,
    cas: Cas,
    vocabulary: Vocabulary,
    hv: Hash<Vocabulary>,
}
impl Fixture {
    fn new() -> Self {
        let temp = Temp::new();
        let cas = Cas::open(temp.0.join("cas"), Limits::default()).unwrap();
        let vocabulary = Vocabulary::new(1, vec![], vec![]).unwrap();
        let hv = cas.put(&to_canon(&vocabulary).unwrap()).unwrap();
        cas.put::<Certificate>(b"fixture-cert").unwrap();
        cas.put::<Checker>(b"fixture-checker").unwrap();
        cas.put::<ModelObject>(&[0x80]).unwrap();
        Self {
            temp,
            signer: signer(),
            cas,
            vocabulary,
            hv,
        }
    }
    fn ledger(&self) -> Ledger<'_, FileSigner> {
        let mut ledger = Ledger::open(&self.temp.0, &self.signer).unwrap();
        ledger.append(genesis(&self.signer)).unwrap();
        ledger
    }
    fn assumption(&self, scope: &str) -> EventBody {
        EventBody::AssumptionDeclared {
            vocabulary: self.hv,
            formula: truth(),
            scope: Text::new(scope).unwrap(),
            author: self.signer.key_id().unwrap(),
        }
    }
    fn obligation(
        &self,
        ledger: &Ledger<'_, FileSigner>,
        refs: &[Hash<Event>],
        goal: &Formula,
    ) -> Hash<Obligation> {
        let refs: Vec<_> = refs.iter().map(|h| premise(*h)).collect();
        let index = ledger.index().unwrap();
        let snapshot = index.snapshot(&self.cas, &refs, "local").unwrap();
        let obligation = generate(
            Proposal {
                vocabulary: &self.vocabulary,
                goal,
                cites: &refs,
            },
            &snapshot,
        )
        .unwrap();
        self.cas.put(&to_canon(&obligation).unwrap()).unwrap()
    }
    fn cache(&self) -> std::path::PathBuf {
        self.temp.0.join("index.sqlite3")
    }
}
fn truth() -> Formula {
    Formula::new(vec![FormulaNode::True], AirLimits::default()).unwrap()
}
fn falsity() -> Formula {
    Formula::new(vec![FormulaNode::False], AirLimits::default()).unwrap()
}
fn premise(id: Hash<Event>) -> PremiseRef {
    PremiseRef::from_event_hash(*id.as_bytes())
}
fn check(ho: Hash<Obligation>) -> CheckRecord {
    CheckRecord {
        obligation: ho,
        certificate: hash_raw::<Certificate>(b"fixture-cert"),
        checker: hash_raw::<Checker>(b"fixture-checker"),
        budget: Budget {
            steps: 100,
            mem_bytes: 1024,
            size_bytes: 1024,
            wall_ms: 0,
        },
        outcome: CheckOutcome::Accepted,
    }
}
fn verified(
    ho: Hash<Obligation>,
    mut cited: Vec<Hash<Event>>,
    mut used: Vec<Hash<Event>>,
) -> EventBody {
    cited.sort_unstable();
    used.sort_unstable();
    EventBody::Verified {
        check: check(ho),
        cited,
        used,
        consistency: Consistency::Unknown { check: None },
    }
}
fn receipt(mut body: EventBody, signer: &FileSigner) -> EventBody {
    let bytes = body.authority_message().unwrap().unwrap();
    match &mut body {
        EventBody::Retracted { authority, .. } | EventBody::CheckerRevoked { authority, .. } => {
            authority.sig = signer.sign::<Event>(&bytes).unwrap()
        }
        _ => unreachable!(),
    }
    body.verify_authority(signer.public_key()).unwrap();
    body
}
fn retract(id: Hash<Event>, signer: &FileSigner) -> EventBody {
    receipt(
        EventBody::Retracted {
            target: id,
            reason: Text::new("fixture").unwrap(),
            authority: Authority {
                key: signer.key_id().unwrap(),
                sig: signer.sign::<Event>(b"placeholder").unwrap(),
            },
        },
        signer,
    )
}
fn revoke(signer: &FileSigner) -> EventBody {
    receipt(
        EventBody::CheckerRevoked {
            checker: hash_raw::<Checker>(b"fixture-checker"),
            reason: Text::new("fixture").unwrap(),
            authority: Authority {
                key: signer.key_id().unwrap(),
                sig: signer.sign::<Event>(b"placeholder").unwrap(),
            },
        },
        signer,
    )
}

#[test]
fn statuses_dependencies_and_real_generation_bind_event_ids_to_cas_objects() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let a = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let b = hash(&ledger.append(fixture.assumption("other")).unwrap()).unwrap();
    let ho = fixture.obligation(&ledger, &[a], &truth());
    let negative = fixture.obligation(&ledger, &[], &falsity());
    let pending = hash(
        &ledger
            .append(EventBody::ObligationCreated {
                obligation: ho,
                route: Route::Model,
                artifact: [0; 32],
                cnf: None,
            })
            .unwrap(),
    )
    .unwrap();
    let proposed = hash(
        &ledger
            .append(EventBody::Proposed {
                proposal: EventProposal {
                    text_pt: TaintedText::new("não confiável".into()).unwrap(),
                    goal: truth(),
                    cites: vec![a],
                    speculative_on: vec![],
                    vocabulary: fixture.hv,
                },
                author: fixture.signer.key_id().unwrap(),
            })
            .unwrap(),
    )
    .unwrap();
    let unknown = hash(
        &ledger
            .append(EventBody::UnknownRecorded {
                check: CheckRecord {
                    outcome: CheckOutcome::Unknown(Exhausted::Steps),
                    ..check(ho)
                },
                machine: Text::new("fixture").unwrap(),
            })
            .unwrap(),
    )
    .unwrap();
    let refuted = hash(
        &ledger
            .append(EventBody::Refuted {
                check: check(negative),
                model: hash_raw::<ModelObject>(&[0x80]),
            })
            .unwrap(),
    )
    .unwrap();
    let proof = hash(&ledger.append(verified(ho, vec![a, b], vec![a])).unwrap()).unwrap();
    let index = ledger.index().unwrap();
    for (reference, kind, eligible) in [
        (a, EntryKind::Assumption, true),
        (b, EntryKind::Assumption, true),
        (pending, EntryKind::Pending, false),
        (proposed, EntryKind::Proposal, false),
        (unknown, EntryKind::Unknown, false),
        (refuted, EntryKind::Refuted, false),
        (proof, EntryKind::Verified, true),
    ] {
        assert_eq!(index.status(reference).unwrap().unwrap().kind, kind);
        assert_eq!(index.is_citable(reference, 0).unwrap(), eligible);
        assert!(!index.is_citable(reference, 1).unwrap());
        assert_eq!(
            index.formula_of(reference, &fixture.cas).unwrap().unwrap(),
            if reference == refuted {
                falsity()
            } else {
                truth()
            }
        );
    }
    assert_eq!(index.dependents(a).unwrap(), vec![proof]);
    assert!(index.dependents(b).unwrap().is_empty()); // cited, porém não used.
    let refs = [premise(a), premise(proof)];
    let snapshot = index.snapshot(&fixture.cas, &refs, "local").unwrap();
    let generated = generate(
        Proposal {
            vocabulary: &fixture.vocabulary,
            goal: &truth(),
            cites: &refs,
        },
        &snapshot,
    )
    .unwrap();
    assert_eq!(generated.premises().len(), 2);
    assert!(generated.premises().iter().all(|(_, f)| *f == truth()));
    assert_eq!(
        index.formula_of(proof, &fixture.cas).unwrap(),
        Some(truth())
    );
    assert_eq!(index.report().events, 8);
    assert_eq!(index.report().claims, 7);
    assert!(matches!(
        LedgerIndex::open(&fixture.temp.0, fixture.signer.public_key()),
        Err(LedgerError::WriterBusy)
    ));
}

#[test]
fn noncitable_outcomes_never_become_premises_and_unprepared_lookup_is_unavailable() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let ho = fixture.obligation(&ledger, &[], &truth());
    let pending = hash(
        &ledger
            .append(EventBody::ObligationCreated {
                obligation: ho,
                route: Route::Model,
                artifact: [0; 32],
                cnf: None,
            })
            .unwrap(),
    )
    .unwrap();
    let unknown = hash(
        &ledger
            .append(EventBody::UnknownRecorded {
                check: CheckRecord {
                    outcome: CheckOutcome::Unknown(Exhausted::Time),
                    ..check(ho)
                },
                machine: Text::new("fixture").unwrap(),
            })
            .unwrap(),
    )
    .unwrap();
    let index = ledger.index().unwrap();
    for id in [pending, unknown] {
        let refs = [premise(id)];
        let view = index.snapshot(&fixture.cas, &refs, "local").unwrap();
        assert!(matches!(
            generate(
                Proposal {
                    vocabulary: &fixture.vocabulary,
                    goal: &truth(),
                    cites: &refs
                },
                &view
            ),
            Err(Reject::NotCitable { .. })
        ));
    }
    let absent = premise(Hash::from_bytes([0xff; 32]));
    let view = index.snapshot(&fixture.cas, &[], "local").unwrap();
    assert!(matches!(
        view.lookup(absent),
        Err(logos_obligation::LedgerError::Unavailable)
    ));
    let view = index.snapshot(&fixture.cas, &[absent], "local").unwrap();
    assert!(view.lookup(absent).unwrap().is_none());
    assert!(matches!(
        generate(
            Proposal {
                vocabulary: &fixture.vocabulary,
                goal: &truth(),
                cites: &[absent]
            },
            &view
        ),
        Err(Reject::UnknownCitation(_))
    ));
}

#[test]
fn scope_is_exact_and_includes_used_ancestors() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let a = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let ho = fixture.obligation(&ledger, &[a], &truth());
    let proof = hash(&ledger.append(verified(ho, vec![a], vec![a])).unwrap()).unwrap();
    let index = ledger.index().unwrap();
    for id in [a, proof] {
        assert!(matches!(
            index.snapshot(&fixture.cas, &[premise(id)], "LOCAL"),
            Err(LedgerError::Invalid("assumption scope mismatch"))
        ));
        index
            .snapshot(&fixture.cas, &[premise(id)], "local")
            .unwrap();
    }
}

#[test]
fn retracting_unused_citation_preserves_claim_but_used_transitive_retraction_removes_it() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let a = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let b = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let ho = fixture.obligation(&ledger, &[a, b], &truth());
    let c = hash(&ledger.append(verified(ho, vec![a, b], vec![a])).unwrap()).unwrap();
    let hd = fixture.obligation(&ledger, &[c], &truth());
    let d = hash(&ledger.append(verified(hd, vec![c], vec![c])).unwrap()).unwrap();
    ledger.append(retract(b, &fixture.signer)).unwrap();
    {
        let index = ledger.index().unwrap();
        assert!(!index.is_citable(b, 0).unwrap());
        assert!(index.is_citable(c, 0).unwrap());
        assert!(index.is_citable(d, 0).unwrap());
    }
    ledger.append(retract(a, &fixture.signer)).unwrap();
    let index = ledger.index().unwrap();
    for id in [a, c, d] {
        assert!(!index.is_citable(id, 0).unwrap());
        assert_eq!(
            index.status(id).unwrap().unwrap().kind,
            EntryKind::Invalidated
        );
    }
    assert!(index.status(a).unwrap().unwrap().retracted);
    assert!(!index.status(c).unwrap().unwrap().retracted);
    let refs = [premise(c)];
    let view = index.snapshot(&fixture.cas, &refs, "local").unwrap();
    assert!(matches!(
        generate(
            Proposal {
                vocabulary: &fixture.vocabulary,
                goal: &truth(),
                cites: &refs
            },
            &view
        ),
        Err(Reject::NotCitable { .. })
    ));
}

#[test]
fn explicit_invalidation_and_checker_revocation_cannot_be_undone_by_later_verified() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let a = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let ho = fixture.obligation(&ledger, &[a], &truth());
    let proof = hash(&ledger.append(verified(ho, vec![a], vec![a])).unwrap()).unwrap();
    ledger.append(revoke(&fixture.signer)).unwrap();
    let later = hash(&ledger.append(verified(ho, vec![a], vec![a])).unwrap()).unwrap();
    {
        let index = ledger.index().unwrap();
        assert!(index.is_citable(a, 0).unwrap());
        for id in [proof, later] {
            assert_eq!(
                index.status(id).unwrap().unwrap().kind,
                EntryKind::Invalidated
            );
            assert!(!index.is_citable(id, 0).unwrap());
        }
    }
    ledger
        .append(EventBody::Invalidated {
            claim: a,
            causes: vec![a],
        })
        .unwrap();
    let index = ledger.index().unwrap();
    assert!(!index.is_citable(a, 0).unwrap());
}

#[test]
fn epochs_do_not_truncate_u64_or_promote_old_premises() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let old = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    ledger.append(epoch(u64::MAX)).unwrap();
    let current = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let index = ledger.index().unwrap();
    assert!(!index.is_citable(old, 0).unwrap());
    assert!(!index.is_citable(old, u64::MAX).unwrap());
    assert!(index.is_citable(current, u64::MAX).unwrap());
    assert_eq!(index.status(current).unwrap().unwrap().epoch, u64::MAX);
    let refs = [premise(old)];
    let view = index.snapshot(&fixture.cas, &refs, "local").unwrap();
    assert!(matches!(
        generate(
            Proposal {
                vocabulary: &fixture.vocabulary,
                goal: &truth(),
                cites: &refs
            },
            &view
        ),
        Err(Reject::InvalidEpoch { .. })
    ));
}

#[test]
fn advanced_epoch_rejects_old_verified_and_assumption_in_real_generation() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    ledger.append(epoch(3)).unwrap();
    let a = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let ho = fixture.obligation(&ledger, &[a], &falsity());
    let verified = hash(&ledger.append(verified(ho, vec![a], vec![a])).unwrap()).unwrap();
    let refs = [premise(a), premise(verified)];
    let old_head = ledger.head().unwrap();
    {
        let index = ledger.index().unwrap();
        for reference in [a, verified] {
            assert!(index.is_citable(reference, 3).unwrap());
        }
        let view = index.snapshot(&fixture.cas, &refs, "local").unwrap();
        assert!(
            generate(
                Proposal {
                    vocabulary: &fixture.vocabulary,
                    goal: &truth(),
                    cites: &refs
                },
                &view
            )
            .is_ok()
        );
    }
    for new_epoch in [4, 6] {
        let event = ledger
            .append(EventBody::EpochAdvanced {
                new_epoch,
                reason: Text::new("contexto genérico").unwrap(),
                policy_root: Some([new_epoch as u8; 32]),
                vocab: Some(fixture.hv),
            })
            .unwrap();
        assert_eq!(event.epoch, new_epoch);
        {
            let index = ledger.index().unwrap();
            assert_eq!(index.status(a).unwrap().unwrap().epoch, 3);
            assert_eq!(
                index.status(verified).unwrap().unwrap().kind,
                EntryKind::Verified
            );
            assert_eq!(index.status(verified).unwrap().unwrap().epoch, 3);
            for reference in [a, verified] {
                assert!(!index.is_citable(reference, new_epoch).unwrap());
                let cites = [premise(reference)];
                let view = index.snapshot(&fixture.cas, &cites, "local").unwrap();
                assert!(
                    matches!(generate(Proposal { vocabulary: &fixture.vocabulary, goal: &truth(), cites: &cites }, &view),
                    Err(Reject::InvalidEpoch { expected, actual: 3, .. }) if expected == new_epoch)
                );
            }
            assert_eq!(
                index.formula_of(verified, &fixture.cas).unwrap(),
                Some(falsity())
            );
        }
        let head = ledger.head().unwrap();
        assert_ne!(head, old_head);
        drop(ledger);
        ledger = Ledger::open(&fixture.temp.0, &fixture.signer).unwrap();
        assert_eq!(ledger.head().unwrap(), head);
    }
    // A época nova permite premissa nova, sem promover os eventos anteriores.
    let current = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let cites = [premise(current)];
    let index = ledger.index().unwrap();
    assert!(index.is_citable(current, 6).unwrap());
    let view = index.snapshot(&fixture.cas, &cites, "local").unwrap();
    assert!(
        generate(
            Proposal {
                vocabulary: &fixture.vocabulary,
                goal: &truth(),
                cites: &cites
            },
            &view
        )
        .is_ok()
    );
}

#[test]
fn advancing_epoch_never_promotes_unknown_or_invalidated_records() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    ledger.append(epoch(3)).unwrap();
    let ho = fixture.obligation(&ledger, &[], &truth());
    let invalid = hash(&ledger.append(verified(ho, vec![], vec![])).unwrap()).unwrap();
    ledger
        .append(EventBody::Invalidated {
            claim: invalid,
            causes: vec![invalid],
        })
        .unwrap();
    let unknown = hash(
        &ledger
            .append(EventBody::UnknownRecorded {
                check: CheckRecord {
                    outcome: CheckOutcome::Unknown(Exhausted::Memory),
                    ..check(ho)
                },
                machine: Text::new("fixture").unwrap(),
            })
            .unwrap(),
    )
    .unwrap();
    for current_epoch in [3, 4, 6] {
        if current_epoch != 3 {
            ledger.append(epoch(current_epoch)).unwrap();
        }
        let index = ledger.index().unwrap();
        for (reference, kind) in [
            (invalid, EntryKind::Invalidated),
            (unknown, EntryKind::Unknown),
        ] {
            assert_eq!(index.status(reference).unwrap().unwrap().kind, kind);
            assert!(!index.is_citable(reference, current_epoch).unwrap());
            let cites = [premise(reference)];
            let view = index.snapshot(&fixture.cas, &cites, "local").unwrap();
            assert!(
                generate(
                    Proposal {
                        vocabulary: &fixture.vocabulary,
                        goal: &truth(),
                        cites: &cites
                    },
                    &view
                )
                .is_err()
            );
        }
    }
}

#[test]
fn missing_or_corrupt_cas_never_creates_placeholder_formulas() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let missing = fixture.obligation(&ledger, &[], &truth());
    let event = hash(&ledger.append(verified(missing, vec![], vec![])).unwrap()).unwrap();
    let hex = missing.to_hex();
    fs::remove_file(
        fixture
            .temp
            .0
            .join("cas/obligation-v1")
            .join(&hex[..2])
            .join(&hex[2..]),
    )
    .unwrap();
    let a = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let index = ledger.index().unwrap();
    assert!(matches!(
        index.formula_of(event, &fixture.cas),
        Err(LedgerError::Invalid("obligation missing from CAS"))
    ));
    assert!(
        index
            .snapshot(&fixture.cas, &[premise(event)], "local")
            .is_err()
    );
    let empty = Cas::open(fixture.temp.0.join("empty-cas"), Limits::default()).unwrap();
    assert!(matches!(
        index.formula_of(a, &empty),
        Err(LedgerError::Invalid("vocabulary missing from CAS"))
    ));
    let hex = fixture.hv.to_hex();
    let object = fixture
        .temp
        .0
        .join("cas/vocab-v1")
        .join(&hex[..2])
        .join(&hex[2..]);
    fs::write(object, b"corrupt").unwrap();
    assert!(matches!(
        index.formula_of(a, &fixture.cas),
        Err(LedgerError::Cas(logos_ledger::CasError::Integrity))
    ));
}

#[test]
fn malformed_obligation_with_authentic_hash_is_rejected_by_complete_reader() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let ho = fixture.obligation(&ledger, &[], &truth());
    let mut value: Value = from_canon(&fixture.cas.get(&ho).unwrap().unwrap()).unwrap();
    let Value::Array(array) = &mut value else {
        unreachable!()
    };
    let mut items = array.items().to_vec();
    items[5] = Value::Bool(true);
    value = Value::Array(logos_canon::Array::new(items).unwrap());
    let malformed = fixture
        .cas
        .put::<Obligation>(&to_canon(&value).unwrap())
        .unwrap();
    let event = hash(&ledger.append(verified(malformed, vec![], vec![])).unwrap()).unwrap();
    let index = ledger.index().unwrap();
    assert!(matches!(
        index.formula_of(event, &fixture.cas),
        Err(LedgerError::Obligation(_))
    ));
}

#[test]
fn ill_sorted_embedded_formula_and_invalid_vocabulary_are_not_usable() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let wrong = Formula::new(
        vec![
            FormulaNode::Forall(logos_air::Sort {
                name: Text::new("missing").unwrap(),
            }),
            FormulaNode::True,
        ],
        AirLimits::default(),
    )
    .unwrap();
    let bad_formula = hash(
        &ledger
            .append(EventBody::AssumptionDeclared {
                vocabulary: fixture.hv,
                formula: wrong,
                scope: Text::new("local").unwrap(),
                author: fixture.signer.key_id().unwrap(),
            })
            .unwrap(),
    )
    .unwrap();
    let bad_vocabulary = fixture.cas.put::<Vocabulary>(&[0xf6]).unwrap();
    let bad_context = hash(
        &ledger
            .append(EventBody::AssumptionDeclared {
                vocabulary: bad_vocabulary,
                formula: truth(),
                scope: Text::new("local").unwrap(),
                author: fixture.signer.key_id().unwrap(),
            })
            .unwrap(),
    )
    .unwrap();
    let index = ledger.index().unwrap();
    assert!(matches!(
        index.formula_of(bad_formula, &fixture.cas),
        Err(LedgerError::Invalid("claim formula sort"))
    ));
    assert!(
        index
            .snapshot(&fixture.cas, &[premise(bad_formula)], "local")
            .is_err()
    );
    assert!(matches!(
        index.formula_of(bad_context, &fixture.cas),
        Err(LedgerError::Invalid("invalid CAS vocabulary"))
    ));
}

#[test]
fn cache_tampering_is_reported_and_never_becomes_authority() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let ho = fixture.obligation(&ledger, &[], &truth());
    let unknown = hash(
        &ledger
            .append(EventBody::UnknownRecorded {
                check: CheckRecord {
                    outcome: CheckOutcome::Unknown(Exhausted::Steps),
                    ..check(ho)
                },
                machine: Text::new("fixture").unwrap(),
            })
            .unwrap(),
    )
    .unwrap();
    let index = ledger.index().unwrap();
    let cache = Connection::open(fixture.cache()).unwrap();
    cache
        .execute(
            "UPDATE claims SET status=2 WHERE id=?1",
            [unknown.as_bytes().as_slice()],
        )
        .unwrap();
    cache
        .execute(
            "INSERT INTO citable(ref,epoch) VALUES(?1,?2)",
            params![unknown.as_bytes().as_slice(), 0u64.to_be_bytes().as_slice()],
        )
        .unwrap();
    assert!(!index.is_citable(unknown, 0).unwrap()); // cache modificado enquanto o snapshot existe.
    drop(cache);
    drop(index);
    let index = ledger.index().unwrap();
    assert_eq!(index.report().previous, IndexComparison::Different);
    assert!(!index.is_citable(unknown, 0).unwrap());
    drop(index);
    let index = ledger.index().unwrap();
    assert_eq!(index.report().previous, IndexComparison::Equal);
}

#[test]
fn deleting_or_corrupting_cache_reconstructs_without_changing_source_head() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let id = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let head = ledger.head().unwrap();
    let index = ledger.index().unwrap();
    drop(index);
    fs::remove_file(fixture.cache()).unwrap();
    let index = ledger.index().unwrap();
    assert_eq!(index.report().previous, IndexComparison::Missing);
    assert_eq!(index.head(), head);
    assert!(index.is_citable(id, 0).unwrap());
    drop(index);
    fs::write(fixture.cache(), b"not sqlite").unwrap();
    let index = ledger.index().unwrap();
    assert_eq!(index.report().previous, IndexComparison::Invalid);
    assert_eq!(index.head(), head);
}

#[test]
fn invalid_source_or_used_premise_cannot_publish_or_repair_signed_history() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    ledger.append(fixture.assumption("local")).unwrap();
    let index = ledger.index().unwrap();
    drop(index);
    let before = fs::read(fixture.cache()).unwrap();
    let ho = fixture.cas.put::<Obligation>(&[0x80]).unwrap();
    ledger
        .append(verified(
            ho,
            vec![Hash::from_bytes([0xff; 32])],
            vec![Hash::from_bytes([0xff; 32])],
        ))
        .unwrap();
    assert!(matches!(
        ledger.index(),
        Err(LedgerError::Invalid(
            "used premise is not an earlier citable claim in this epoch"
        ))
    ));
    assert_eq!(fs::read(fixture.cache()).unwrap(), before);
    drop(ledger);
    let connection = Connection::open(fixture.temp.database()).unwrap();
    connection
        .execute("UPDATE events SET wire=x'ff' WHERE seq=0", [])
        .unwrap();
    assert!(LedgerIndex::open(&fixture.temp.0, fixture.signer.public_key()).is_err());
    assert_eq!(fs::read(fixture.cache()).unwrap(), before);
}

#[test]
fn standalone_index_uses_external_key_and_holds_writer_lock() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    ledger.append(fixture.assumption("local")).unwrap();
    drop(ledger);
    let wrong = FileSigner::from_seed(&[0x22; 32]).unwrap();
    assert!(matches!(
        LedgerIndex::open(&fixture.temp.0, wrong.public_key()),
        Err(LedgerError::Event(_))
    ));
    let index = LedgerIndex::open(&fixture.temp.0, fixture.signer.public_key()).unwrap();
    assert!(matches!(
        Ledger::open(&fixture.temp.0, &fixture.signer),
        Err(LedgerError::WriterBusy)
    ));
    drop(index);
    Ledger::open(&fixture.temp.0, &fixture.signer).unwrap();
}

#[test]
fn index_layout_and_unsigned_epoch_match_normative_vector() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    ledger.append(epoch(u64::MAX)).unwrap();
    let id = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let _index = ledger.index().unwrap();
    let vector: serde_json::Value = serde_json::from_str(include_str!(
        "../../../spec/vectors/ledger-index/layout.json"
    ))
    .unwrap();
    let cache = Connection::open(fixture.cache()).unwrap();
    let app: i64 = cache
        .pragma_query_value(None, "application_id", |r| r.get(0))
        .unwrap();
    let version: i64 = cache
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(
        format!("{app:08x}"),
        vector["application_id_hex"].as_str().unwrap()
    );
    assert_eq!(version, vector["user_version"].as_i64().unwrap());
    let schema = vector["schema"].as_object().unwrap();
    let rows = cache
        .prepare("SELECT name,sql FROM sqlite_schema ORDER BY name")
        .unwrap()
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), schema.len());
    for (name, sql) in rows {
        assert_eq!(sql, schema[&name].as_str().unwrap());
    }
    let stored: Vec<u8> = cache
        .query_row(
            "SELECT epoch FROM claims WHERE id=?1",
            [id.as_bytes().as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        stored,
        support::unhex(vector["epochs"][2]["hex"].as_str().unwrap())
    );
}
