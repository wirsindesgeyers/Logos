//! Integra certificado/used com o índice existente, sem implementar kernel.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[allow(dead_code)]
mod support;
use support as ledger_support;
#[path = "../../logos-lrat/tests/used_support/mod.rs"]
mod used_support;
use logos_canon::{Text, to_canon};
use logos_core::CheckOutcome;
use logos_crypto::{Hash, Signer, hash, hash_raw};
use logos_ledger::{Cas, Certificate, CheckRecord, Checker, Consistency, Event, EventBody, Ledger};
use logos_lrat::CakeRegistry;
use logos_obligation::{PremiseRef, Proposal, generate};
use used_support::*;
fn text(s: &str) -> Text {
    Text::new(s).unwrap()
}

#[test]
fn only_certificate_dependencies_enter_the_real_ledger_index() {
    let (v, fs, goal) = policy();
    let temp = ledger_support::Temp::new();
    let signer = ledger_support::signer();
    let cas = Cas::open(temp.0.join("cas"), logos_ledger::Limits::default()).unwrap();
    let hv = cas.put(&to_canon(&v).unwrap()).unwrap();
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    ledger.append(ledger_support::genesis(&signer)).unwrap();
    let events: Vec<Hash<Event>> = fs
        .iter()
        .map(|f| {
            hash(
                &ledger
                    .append(EventBody::AssumptionDeclared {
                        vocabulary: hv,
                        formula: f.clone(),
                        scope: text("local"),
                        author: signer.key_id().unwrap(),
                    })
                    .unwrap(),
            )
            .unwrap()
        })
        .collect();
    let cites: Vec<_> = events
        .iter()
        .map(|h| PremiseRef::from_event_hash(*h.as_bytes()))
        .collect();
    let index = ledger.index().unwrap();
    let snapshot = index.snapshot(&cas, &cites, "local").unwrap();
    let o = generate(
        Proposal {
            vocabulary: &v,
            goal: &goal,
            cites: &cites,
        },
        &snapshot,
    )
    .unwrap();
    let (cnf, _, p, used) = checked(&o, &v, &cites[..2]);
    let cake =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/cake-lpr/cake_lpr");
    let registry = CakeRegistry::register(&cake).unwrap();
    assert_eq!(
        registry
            .check_bytes(
                cnf.dimacs(),
                hash_raw(cnf.dimacs()),
                &p,
                hash_raw(&p),
                &limits()
            )
            .unwrap(),
        CheckOutcome::Accepted
    );
    let missing_index = ledger.index().unwrap();
    let missing_snapshot = missing_index.snapshot(&cas, &cites[1..], "local").unwrap();
    let missing = generate(
        Proposal {
            vocabulary: &v,
            goal: &goal,
            cites: &cites[1..],
        },
        &missing_snapshot,
    )
    .unwrap();
    assert!(missing.premises().iter().all(|(r, _)| *r != cites[0]));
    let (missing_cnf, missing_map) =
        logos_ground::clausify(&logos_ground::ground(&missing, &v).unwrap()).unwrap();
    assert!(matches!(
        logos_lrat::LratChecker::new().unwrap().check_lrat(
            missing_cnf.dimacs(),
            missing_map.cnf_hash(),
            &p,
            &limits()
        ),
        CheckOutcome::Rejected(_)
    ));
    let ho = cas.put(&to_canon(&o).unwrap()).unwrap();
    let certificate = cas.put::<Certificate>(&p).unwrap();
    let checker = cas
        .put::<Checker>(b"fixture: checked Rust+cake certificate; no kernel authority")
        .unwrap();
    let mut cited = events.clone();
    cited.sort_unstable();
    let used_events: Vec<_> = used
        .premises()
        .iter()
        .map(|r| Hash::<Event>::from_bytes(*r.event_hash()))
        .collect();
    let claim = hash(
        &ledger
            .append(EventBody::Verified {
                check: CheckRecord {
                    obligation: ho,
                    certificate,
                    checker,
                    budget: limits(),
                    outcome: CheckOutcome::Accepted,
                },
                cited,
                used: used_events.clone(),
                consistency: Consistency::Unknown { check: None },
            })
            .unwrap(),
    )
    .unwrap();
    let index = ledger.index().unwrap();
    for e in events {
        assert_eq!(
            index.dependents(e).unwrap(),
            if used_events.contains(&e) {
                vec![claim]
            } else {
                vec![]
            }
        );
    }
    assert_eq!(used_events.len(), 2);
    assert_eq!(o.premises().len(), 5);
}
