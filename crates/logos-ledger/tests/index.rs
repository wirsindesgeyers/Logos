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
fn checker_revocation_suspends_and_later_verified_by_it_is_born_invalidated() {
    // 5.10 (ADR0056): revogar suspende a rota (não citável, não morta); uma
    // Verified posterior do checker revogado nasce Invalidated.
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
        for (id, kind) in [
            (proof, EntryKind::Suspended),
            (later, EntryKind::Invalidated),
        ] {
            assert_eq!(index.status(id).unwrap().unwrap().kind, kind);
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
fn live_projection_matches_rebuild_across_appends_epochs_and_invalidations() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let mut references = vec![];
    let compare = |ledger: &Ledger<'_, FileSigner>, references: &[Hash<Event>]| {
        let live = ledger.live_index().unwrap();
        let again = ledger.live_index().unwrap(); // leituras simultâneas não entram em borrow_mut
        let rebuilt = ledger.index().unwrap();
        assert_eq!(live.head(), ledger.head().unwrap());
        assert_eq!(again.head(), rebuilt.head());
        let epoch = live.head().unwrap().epoch;
        for &reference in references {
            assert_eq!(
                live.status(reference).unwrap(),
                rebuilt.status(reference).unwrap()
            );
            assert_eq!(
                live.dependents(reference).unwrap(),
                rebuilt.dependents(reference).unwrap()
            );
            for epoch in [0, 1, epoch, u64::MAX] {
                assert_eq!(
                    live.is_citable(reference, epoch).unwrap(),
                    rebuilt.is_citable(reference, epoch).unwrap()
                );
            }
            assert_eq!(
                live.formula_of(reference, &fixture.cas).unwrap(),
                rebuilt.formula_of(reference, &fixture.cas).unwrap()
            );
        }
        let cites: Vec<_> = references.iter().copied().map(premise).collect();
        let a = live.snapshot(&fixture.cas, &cites, "local").unwrap();
        let b = rebuilt.snapshot(&fixture.cas, &cites, "local").unwrap();
        assert_eq!(a.current_epoch(), b.current_epoch());
        for reference in cites {
            let a = a.lookup(reference).unwrap();
            let b = b.lookup(reference).unwrap();
            assert_eq!(
                a.as_ref()
                    .map(|e| (e.kind, e.epoch, e.retracted, e.formula)),
                b.as_ref()
                    .map(|e| (e.kind, e.epoch, e.retracted, e.formula))
            );
        }
    };
    compare(&ledger, &references); // inicialização em Genesis
    let a = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    references.push(a);
    compare(&ledger, &references);
    let ho = fixture.obligation(&ledger, &[a], &truth());
    let b = hash(&ledger.append(verified(ho, vec![a], vec![a])).unwrap()).unwrap();
    references.push(b);
    compare(&ledger, &references);
    let hc = fixture.obligation(&ledger, &[b], &truth());
    let c = hash(&ledger.append(verified(hc, vec![b], vec![b])).unwrap()).unwrap();
    references.push(c);
    compare(&ledger, &references);
    ledger.append(retract(a, &fixture.signer)).unwrap();
    compare(&ledger, &references);
    ledger.append(epoch(1)).unwrap();
    let fresh = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    references.push(fresh);
    compare(&ledger, &references); // mais de um evento no sufixo
    let ho = fixture.obligation(&ledger, &[fresh], &truth());
    let proof = hash(
        &ledger
            .append(verified(ho, vec![fresh], vec![fresh]))
            .unwrap(),
    )
    .unwrap();
    references.push(proof);
    compare(&ledger, &references);
    ledger.append(revoke(&fixture.signer)).unwrap();
    let revoked = hash(
        &ledger
            .append(verified(ho, vec![fresh], vec![fresh]))
            .unwrap(),
    )
    .unwrap();
    references.push(revoked);
    compare(&ledger, &references);
    ledger
        .append(EventBody::UnknownRecorded {
            check: CheckRecord {
                outcome: CheckOutcome::Unknown(Exhausted::Steps),
                ..check(ho)
            },
            machine: Text::new("fixture").unwrap(),
        })
        .unwrap();
    ledger.append(epoch(u64::MAX)).unwrap();
    compare(&ledger, &references);
    let head = ledger.head().unwrap();
    drop(ledger);
    let reopened = Ledger::open(&fixture.temp.0, &fixture.signer).unwrap();
    assert_eq!(reopened.live_index().unwrap().head(), head);
    compare(&reopened, &references);
}

#[test]
fn live_queries_and_appends_never_reopen_or_republish_the_disk_cache() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let a = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    assert!(ledger.live_index().unwrap().is_citable(a, 0).unwrap());
    fs::remove_file(fixture.cache()).unwrap();
    fs::create_dir(fixture.cache()).unwrap(); // um rebuild tentaria abrir/publicar aqui e falharia
    fs::write(fixture.cache().join("sentinel"), b"untouched").unwrap();
    for _ in 0..3 {
        let live = ledger.live_index().unwrap();
        assert!(live.is_citable(a, 0).unwrap());
        assert!(live.status(Hash::from_bytes([0xff; 32])).unwrap().is_none());
    }
    ledger.append(retract(a, &fixture.signer)).unwrap();
    assert!(!ledger.live_index().unwrap().is_citable(a, 0).unwrap());
    ledger.append(epoch(2)).unwrap();
    let b = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    assert!(ledger.live_index().unwrap().is_citable(b, 2).unwrap());
    assert_eq!(
        fs::read(fixture.cache().join("sentinel")).unwrap(),
        b"untouched"
    );
    assert_eq!(fs::read_dir(fixture.cache()).unwrap().count(), 1);
}

#[test]
fn live_projection_never_returns_old_snapshot_after_an_invalid_suffix() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let a = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    assert!(ledger.live_index().unwrap().is_citable(a, 0).unwrap());
    let ho = fixture.cas.put::<Obligation>(&[0x80]).unwrap();
    let missing = Hash::from_bytes([0xff; 32]);
    ledger
        .append(verified(ho, vec![missing], vec![missing]))
        .unwrap();
    for _ in 0..2 {
        assert!(matches!(
            ledger.live_index(),
            Err(LedgerError::Invalid(
                "used premise is not an earlier citable claim in this epoch"
            ))
        ));
    }
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

/// 5.9: o nó h(O) sobrevive enquanto alguma rota Verified tem premissas vivas.
#[test]
fn independent_routes_keep_node_and_dependents_until_every_route_falls() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let a = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let b = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let ho = fixture.obligation(&ledger, &[a, b], &truth());
    let via_a = hash(&ledger.append(verified(ho, vec![a, b], vec![a])).unwrap()).unwrap();
    let via_b = hash(&ledger.append(verified(ho, vec![a, b], vec![b])).unwrap()).unwrap();
    let hd = fixture.obligation(&ledger, &[via_a], &truth());
    let dependent = hash(
        &ledger
            .append(verified(hd, vec![via_a], vec![via_a]))
            .unwrap(),
    )
    .unwrap();
    let status =
        |ledger: &Ledger<'_, FileSigner>, id| ledger.index().unwrap().status(id).unwrap().unwrap();

    ledger.append(retract(a, &fixture.signer)).unwrap();
    for id in [via_a, via_b, dependent] {
        assert_eq!(
            status(&ledger, id).kind,
            EntryKind::Verified,
            "rota independente mantém o nó"
        );
        assert!(ledger.index().unwrap().is_citable(id, 0).unwrap());
    }
    assert_eq!(status(&ledger, a).kind, EntryKind::Invalidated);

    ledger.append(retract(b, &fixture.signer)).unwrap();
    for id in [via_a, via_b, dependent] {
        assert_eq!(
            status(&ledger, id).kind,
            EntryKind::Invalidated,
            "todas as rotas caíram"
        );
        assert!(!ledger.index().unwrap().is_citable(id, 0).unwrap());
    }
    drop(ledger);
    let rebuilt = LedgerIndex::open(&fixture.temp.0, fixture.signer.public_key()).unwrap();
    for id in [a, b, via_a, via_b, dependent] {
        assert_eq!(
            rebuilt.status(id).unwrap().unwrap().kind,
            EntryKind::Invalidated
        );
    }
}

/// Retratar a própria rota a mata, sem derrubar o nó sustentado pela outra.
#[test]
fn retracting_one_route_kills_only_that_route_while_a_sibling_supports_the_node() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let a = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let b = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let ho = fixture.obligation(&ledger, &[a, b], &truth());
    let via_a = hash(&ledger.append(verified(ho, vec![a, b], vec![a])).unwrap()).unwrap();
    let via_b = hash(&ledger.append(verified(ho, vec![a, b], vec![b])).unwrap()).unwrap();
    let hd = fixture.obligation(&ledger, &[via_a], &truth());
    let dependent = hash(
        &ledger
            .append(verified(hd, vec![via_a], vec![via_a]))
            .unwrap(),
    )
    .unwrap();
    ledger.append(retract(via_a, &fixture.signer)).unwrap();
    let index = ledger.index().unwrap();
    let retracted = index.status(via_a).unwrap().unwrap();
    assert_eq!(retracted.kind, EntryKind::Invalidated);
    assert!(retracted.retracted && !index.is_citable(via_a, 0).unwrap());
    for id in [via_b, dependent, a, b] {
        assert!(
            index.is_citable(id, 0).unwrap(),
            "nó h(O) segue vivo pela rota via_b"
        );
    }
}

// --- 5.10 (ADR0056): revogação suspende; Rechecked reativa ou invalida ------

const OTHER: &[u8] = b"other-checker";

fn verified_by(
    ho: Hash<Obligation>,
    cited: Vec<Hash<Event>>,
    used: Vec<Hash<Event>>,
    checker: &[u8],
) -> EventBody {
    let mut body = verified(ho, cited, used);
    if let EventBody::Verified { check, .. } = &mut body {
        check.checker = hash_raw::<Checker>(checker);
    }
    body
}

fn rechecked(route: Hash<Event>, ho: Hash<Obligation>, outcome: CheckOutcome) -> EventBody {
    let mut check = check(ho);
    check.checker = hash_raw::<Checker>(OTHER);
    check.outcome = outcome;
    EventBody::Rechecked { route, check }
}

struct Chain {
    fixture: Fixture,
    a: Hash<Event>,
    ho: Hash<Obligation>,
    route: Hash<Event>,
    dependent: Hash<Event>,
}

/// a ← route (checker revogável) ← dependent (outro checker).
fn chain() -> Chain {
    let fixture = Fixture::new();
    fixture.cas.put::<Checker>(OTHER).unwrap();
    let mut ledger = fixture.ledger();
    let a = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let ho = fixture.obligation(&ledger, &[a], &truth());
    let route = hash(&ledger.append(verified(ho, vec![a], vec![a])).unwrap()).unwrap();
    let hd = fixture.obligation(&ledger, &[route], &truth());
    let dependent = hash(
        &ledger
            .append(verified_by(hd, vec![route], vec![route], OTHER))
            .unwrap(),
    )
    .unwrap();
    drop(ledger);
    Chain {
        fixture,
        a,
        ho,
        route,
        dependent,
    }
}

fn kinds(ledger: &Ledger<'_, FileSigner>, ids: &[Hash<Event>]) -> Vec<(EntryKind, bool)> {
    let index = ledger.index().unwrap();
    ids.iter()
        .map(|id| {
            (
                index.status(*id).unwrap().unwrap().kind,
                index.is_citable(*id, 0).unwrap(),
            )
        })
        .collect()
}

#[test]
fn revocation_suspends_dependents_and_accepted_recheck_restores_them() {
    let c = chain();
    let mut ledger = Ledger::open(&c.fixture.temp.0, &c.fixture.signer).unwrap();
    let ids = [c.route, c.dependent];
    ledger.append(revoke(&c.fixture.signer)).unwrap();
    assert_eq!(
        kinds(&ledger, &ids),
        vec![(EntryKind::Suspended, false); 2],
        "dependente de outro checker também fica suspenso"
    );
    assert!(ledger.index().unwrap().is_citable(c.a, 0).unwrap());
    ledger
        .append(rechecked(
            c.route,
            c.ho,
            CheckOutcome::Unknown(Exhausted::Time),
        ))
        .unwrap();
    assert_eq!(kinds(&ledger, &ids), vec![(EntryKind::Suspended, false); 2]);
    ledger
        .append(rechecked(c.route, c.ho, CheckOutcome::Accepted))
        .unwrap();
    assert_eq!(kinds(&ledger, &ids), vec![(EntryKind::Verified, true); 2]);
    drop(ledger);
    let rebuilt = LedgerIndex::open(&c.fixture.temp.0, c.fixture.signer.public_key()).unwrap();
    for id in ids {
        assert_eq!(
            rebuilt.status(id).unwrap().unwrap().kind,
            EntryKind::Verified
        );
        assert!(rebuilt.is_citable(id, 0).unwrap());
    }
}

#[test]
fn rejected_recheck_invalidates_route_and_cascades() {
    let c = chain();
    let mut ledger = Ledger::open(&c.fixture.temp.0, &c.fixture.signer).unwrap();
    ledger.append(revoke(&c.fixture.signer)).unwrap();
    ledger
        .append(rechecked(
            c.route,
            c.ho,
            CheckOutcome::Rejected(logos_core::Reason::new(Text::new("bad proof").unwrap())),
        ))
        .unwrap();
    assert_eq!(
        kinds(&ledger, &[c.route, c.dependent]),
        vec![(EntryKind::Invalidated, false); 2]
    );
    // Rota morta não pode ser rechecada de novo: a projeção recusa o sufixo.
    ledger
        .append(rechecked(c.route, c.ho, CheckOutcome::Accepted))
        .unwrap();
    assert!(matches!(
        ledger.live_index(),
        Err(LedgerError::Invalid("recheck target is not a live route"))
    ));
}

#[test]
fn active_independent_route_keeps_node_citable_after_revocation() {
    let fixture = Fixture::new();
    fixture.cas.put::<Checker>(OTHER).unwrap();
    let mut ledger = fixture.ledger();
    let a = hash(&ledger.append(fixture.assumption("local")).unwrap()).unwrap();
    let ho = fixture.obligation(&ledger, &[a], &truth());
    let revocable = hash(&ledger.append(verified(ho, vec![a], vec![a])).unwrap()).unwrap();
    let other = hash(
        &ledger
            .append(verified_by(ho, vec![a], vec![a], OTHER))
            .unwrap(),
    )
    .unwrap();
    let hd = fixture.obligation(&ledger, &[revocable], &truth());
    let dependent = hash(
        &ledger
            .append(verified_by(hd, vec![revocable], vec![revocable], OTHER))
            .unwrap(),
    )
    .unwrap();
    ledger.append(revoke(&fixture.signer)).unwrap();
    assert_eq!(
        kinds(&ledger, &[revocable, other, dependent]),
        vec![(EntryKind::Verified, true); 3],
        "nó ativo pela rota do checker não revogado"
    );
}

#[test]
fn new_route_from_active_checker_reactivates_suspended_node() {
    let c = chain();
    let mut ledger = Ledger::open(&c.fixture.temp.0, &c.fixture.signer).unwrap();
    ledger.append(revoke(&c.fixture.signer)).unwrap();
    // Reproposta da mesma h(O) certificada por checker ativo: nova rota.
    let fresh = hash(
        &ledger
            .append(verified_by(c.ho, vec![c.a], vec![c.a], OTHER))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        kinds(&ledger, &[c.route, c.dependent, fresh]),
        vec![(EntryKind::Verified, true); 3]
    );
}

#[test]
fn retracting_premise_of_suspended_route_kills_it() {
    let c = chain();
    let mut ledger = Ledger::open(&c.fixture.temp.0, &c.fixture.signer).unwrap();
    ledger.append(revoke(&c.fixture.signer)).unwrap();
    ledger.append(retract(c.a, &c.fixture.signer)).unwrap();
    assert_eq!(
        kinds(&ledger, &[c.route, c.dependent]),
        vec![(EntryKind::Invalidated, false); 2],
        "suspensão não protege contra morte"
    );
}

#[test]
fn speculation_queries_skip_suspended_routes_and_report_only_fully_dead_nodes() {
    // 5.8 (ADR0058): base especulativa se resolve só por rota Verified ativa; rota
    // suspensa não resolve nem conta como morta.
    let c = chain();
    let mut ledger = Ledger::open(&c.fixture.temp.0, &c.fixture.signer).unwrap();
    let index = ledger.index().unwrap();
    assert_eq!(index.active_route(c.ho).unwrap(), Some(c.route));
    assert!(index.dead_routes(c.ho).unwrap().is_empty());
    drop(index);
    ledger.append(revoke(&c.fixture.signer)).unwrap();
    let index = ledger.index().unwrap();
    assert_eq!(index.active_route(c.ho).unwrap(), None);
    assert!(index.dead_routes(c.ho).unwrap().is_empty());
    drop(index);
    ledger.append(retract(c.a, &c.fixture.signer)).unwrap();
    let index = ledger.index().unwrap();
    assert_eq!(index.active_route(c.ho).unwrap(), None);
    assert_eq!(index.dead_routes(c.ho).unwrap(), vec![c.route]);
    let without_routes = Hash::<Obligation>::from_bytes([9; 32]);
    assert_eq!(index.active_route(without_routes).unwrap(), None);
    assert!(index.dead_routes(without_routes).unwrap().is_empty());
}

type BuildBody = fn(&Chain, &mut Ledger<'_, FileSigner>) -> EventBody;

#[test]
fn recheck_projection_refuses_wrong_targets_and_revoked_checkers() {
    let cases: [(&str, BuildBody); 3] = [
        ("recheck target is not a Verified route", |c, _| {
            rechecked(c.a, c.ho, CheckOutcome::Accepted)
        }),
        ("recheck obligation differs from route", |c, ledger| {
            let other = c.fixture.obligation(ledger, &[], &falsity());
            rechecked(c.route, other, CheckOutcome::Accepted)
        }),
        ("recheck by a revoked checker", |c, ledger| {
            ledger.append(revoke(&c.fixture.signer)).unwrap();
            let mut body = rechecked(c.route, c.ho, CheckOutcome::Accepted);
            if let EventBody::Rechecked { check, .. } = &mut body {
                check.checker = hash_raw::<Checker>(b"fixture-checker");
            }
            body
        }),
    ];
    for (message, build) in cases {
        let c = chain();
        let mut ledger = Ledger::open(&c.fixture.temp.0, &c.fixture.signer).unwrap();
        let body = build(&c, &mut ledger);
        ledger.append(body).unwrap();
        assert!(
            matches!(ledger.live_index(), Err(LedgerError::Invalid(m)) if m == message),
            "{message}"
        );
    }
}
