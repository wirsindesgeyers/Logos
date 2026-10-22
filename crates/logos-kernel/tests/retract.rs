//! Etapa 5.9: retratação assinada, cascata ATMS e registros Invalidated.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use logos_air::{Formula, FormulaNode, Limits as AirLimits, Vocabulary};
use logos_canon::{Text, to_canon};
use logos_core::{Budget, CheckOutcome};
use logos_crypto::{FileSigner, Hash, Signer, hash, hash_raw};
use logos_kernel::{
    TransportError,
    retract::{Retraction, Retractions},
};
use logos_ledger::{
    Cas, Certificate, CheckRecord, Checker, Consistency, Event, EventBody, Ledger, Limits,
};
use logos_obligation::{EntryKind, Obligation, PremiseRef, Proposal, generate};
use support::{Temp, genesis, signer};

struct Fixture {
    temp: Temp,
    admin: FileSigner,
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
        cas.put::<Certificate>(b"cert").unwrap();
        cas.put::<Checker>(b"checker").unwrap();
        Self {
            temp,
            admin: signer(),
            cas,
            vocabulary,
            hv,
        }
    }

    fn ledger(&self) -> Ledger<'_, FileSigner> {
        let mut ledger = Ledger::open(&self.temp.0, &self.admin).unwrap();
        genesis(&mut ledger, &self.admin);
        ledger
    }

    fn assumption(&self, ledger: &mut Ledger<'_, FileSigner>, author: &FileSigner) -> Hash<Event> {
        hash(
            &ledger
                .append(EventBody::AssumptionDeclared {
                    vocabulary: self.hv,
                    formula: Formula::new(vec![FormulaNode::True], AirLimits::default()).unwrap(),
                    scope: Text::new("").unwrap(),
                    author: author.key_id().unwrap(),
                })
                .unwrap(),
        )
        .unwrap()
    }

    fn obligation(
        &self,
        ledger: &Ledger<'_, FileSigner>,
        refs: &[Hash<Event>],
    ) -> Hash<Obligation> {
        let refs: Vec<_> = refs
            .iter()
            .map(|h| PremiseRef::from_event_hash(*h.as_bytes()))
            .collect();
        let index = ledger.index().unwrap();
        let snapshot = index.snapshot(&self.cas, &refs, "").unwrap();
        let goal = Formula::new(vec![FormulaNode::True], AirLimits::default()).unwrap();
        let obligation = generate(
            Proposal {
                vocabulary: &self.vocabulary,
                goal: &goal,
                cites: &refs,
            },
            &snapshot,
        )
        .unwrap();
        self.cas.put(&to_canon(&obligation).unwrap()).unwrap()
    }
}

/// Proposed+ObligationCreated+Verified de uma rota; a prova não importa aqui.
fn route(
    ledger: &mut Ledger<'_, FileSigner>,
    fixture: &Fixture,
    cited: &[Hash<Event>],
    used: &[Hash<Event>],
) -> Hash<Event> {
    let obligation = fixture.obligation(ledger, cited);
    let mut cites = cited.to_vec();
    cites.sort_unstable();
    let mut used = used.to_vec();
    used.sort_unstable();
    ledger
        .append(EventBody::Proposed {
            proposal: logos_ledger::EventProposal {
                text_pt: logos_ledger::TaintedText::new("rota".to_owned()).unwrap(),
                goal: Formula::new(vec![FormulaNode::True], AirLimits::default()).unwrap(),
                cites: cites.clone(),
                speculative_on: vec![],
                vocabulary: fixture.hv,
            },
            author: fixture.admin.key_id().unwrap(),
        })
        .unwrap();
    ledger
        .append(EventBody::ObligationCreated {
            obligation,
            route: logos_ledger::Route::Model,
            artifact: [0; 32],
            cnf: None,
        })
        .unwrap();
    hash(
        &ledger
            .append(EventBody::Verified {
                check: CheckRecord {
                    obligation,
                    certificate: hash_raw::<Certificate>(b"cert"),
                    checker: hash_raw::<Checker>(b"checker"),
                    budget: Budget::new(100, 1024, 1024, 0),
                    outcome: CheckOutcome::Accepted,
                },
                cited: cites,
                used,
                consistency: Consistency::Unknown { check: None },
            })
            .unwrap(),
    )
    .unwrap()
}

fn kind(ledger: &Ledger<'_, FileSigner>, id: Hash<Event>) -> EntryKind {
    ledger.index().unwrap().status(id).unwrap().unwrap().kind
}

fn records(ledger: &Ledger<'_, FileSigner>) -> Vec<(Hash<Event>, Vec<Hash<Event>>)> {
    let head = ledger.head().unwrap().unwrap();
    ledger
        .events_from(0, usize::try_from(head.seq + 1).unwrap())
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.body {
            EventBody::Invalidated { claim, causes } => Some((claim, causes)),
            _ => None,
        })
        .collect()
}

#[test]
fn independent_route_survives_and_cascade_is_recorded_when_both_fall() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let a = fixture.assumption(&mut ledger, &fixture.admin);
    let b = fixture.assumption(&mut ledger, &fixture.admin);
    let via_a = route(&mut ledger, &fixture, &[a, b], &[a]);
    let via_b = route(&mut ledger, &fixture, &[a, b], &[b]);
    let dependent = route(&mut ledger, &fixture, &[via_a], &[via_a]);
    let retractions = Retractions::new(fixture.admin.public_key().clone());

    let first = retractions
        .retract(
            &mut ledger,
            &fixture.admin,
            a,
            Text::new("premissa A errada").unwrap(),
        )
        .unwrap();
    assert!(first.invalidated.is_empty(), "rota via B mantém o nó");
    for id in [via_a, via_b, dependent] {
        assert_eq!(kind(&ledger, id), EntryKind::Verified);
    }

    let Retraction {
        retracted: root_b,
        invalidated,
    } = retractions
        .retract(
            &mut ledger,
            &fixture.admin,
            b,
            Text::new("premissa B errada").unwrap(),
        )
        .unwrap();
    assert_eq!(invalidated.len(), 3);
    for id in [via_a, via_b, dependent] {
        assert_eq!(kind(&ledger, id), EntryKind::Invalidated);
    }
    let recorded = records(&ledger);
    let causes = |claim| {
        recorded
            .iter()
            .find(|(c, _)| *c == claim)
            .unwrap()
            .1
            .clone()
    };
    assert_eq!(causes(via_a), vec![first.retracted, a]);
    assert_eq!(causes(via_b), vec![root_b, b]);
    assert_eq!(causes(dependent), vec![first.retracted, via_a]);
    assert!(
        retractions.reconcile(&mut ledger).unwrap().is_empty(),
        "idempotente"
    );
}

#[test]
fn only_admin_or_assumption_author_may_retract() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let author = FileSigner::from_seed(&[0xA1; 32]).unwrap();
    let stranger = FileSigner::from_seed(&[0xB2; 32]).unwrap();
    assert_ne!(author.public_key(), fixture.admin.public_key());
    let mine = fixture.assumption(&mut ledger, &author);
    let theirs = fixture.assumption(&mut ledger, &fixture.admin);
    let proof = route(&mut ledger, &fixture, &[theirs], &[theirs]);
    let retractions = Retractions::new(fixture.admin.public_key().clone());
    let reason = || Text::new("motivo").unwrap();
    assert!(matches!(
        retractions.retract(&mut ledger, &stranger, mine, reason()),
        Err(TransportError::Unauthorized)
    ));
    assert!(
        matches!(
            retractions.retract(&mut ledger, &author, proof, reason()),
            Err(TransportError::Unauthorized)
        ),
        "rota Verified só pelo admin"
    );
    assert!(
        matches!(
            retractions.retract(&mut ledger, &author, theirs, reason()),
            Err(TransportError::Unauthorized)
        ),
        "autor de outra assumption"
    );
    retractions
        .retract(&mut ledger, &author, mine, reason())
        .unwrap();
    assert!(
        matches!(
            retractions.retract(&mut ledger, &fixture.admin, mine, reason()),
            Err(TransportError::Invalid(_))
        ),
        "já retratada"
    );
    let head = hash(&ledger.events_from(0, 1).unwrap()[0]).unwrap();
    assert!(
        matches!(
            retractions.retract(&mut ledger, &fixture.admin, head, reason()),
            Err(TransportError::Invalid(_))
        ),
        "Genesis não é claim"
    );
}

#[test]
fn interrupted_cascade_is_safe_and_reconcile_completes_records() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let a = fixture.assumption(&mut ledger, &fixture.admin);
    let proof = route(&mut ledger, &fixture, &[a], &[a]);
    let dependent = route(&mut ledger, &fixture, &[proof], &[proof]);
    // Simula crash logo após o Retracted: nenhum Invalidated ainda.
    let mut body = EventBody::Retracted {
        target: a,
        reason: Text::new("crash").unwrap(),
        authority: logos_ledger::Authority {
            key: fixture.admin.key_id().unwrap(),
            sig: fixture.admin.sign::<Event>(b"").unwrap(),
        },
    };
    let message = body.authority_message().unwrap().unwrap();
    if let EventBody::Retracted { authority, .. } = &mut body {
        authority.sig = fixture.admin.sign::<Event>(&message).unwrap();
    }
    let root = hash(&ledger.append(body).unwrap()).unwrap();
    for id in [proof, dependent] {
        assert_eq!(
            kind(&ledger, id),
            EntryKind::Invalidated,
            "índice já derrubou"
        );
        assert!(!ledger.index().unwrap().is_citable(id, 0).unwrap());
    }
    assert!(records(&ledger).is_empty());
    let retractions = Retractions::new(fixture.admin.public_key().clone());
    assert_eq!(retractions.reconcile(&mut ledger).unwrap().len(), 2);
    assert_eq!(
        records(&ledger),
        vec![(proof, vec![root, a]), (dependent, vec![root, proof])]
    );
    assert!(retractions.reconcile(&mut ledger).unwrap().is_empty());
}

#[test]
fn cause_is_the_smallest_dead_premise_when_several_die_together() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let x = fixture.assumption(&mut ledger, &fixture.admin);
    let p = route(&mut ledger, &fixture, &[x], &[x]);
    let q = route(&mut ledger, &fixture, &[x, p], &[x]);
    let claim = route(&mut ledger, &fixture, &[p, q], &[p, q]);
    let retractions = Retractions::new(fixture.admin.public_key().clone());
    let root = retractions
        .retract(&mut ledger, &fixture.admin, x, Text::new("raiz").unwrap())
        .unwrap()
        .retracted;
    let recorded = records(&ledger);
    let causes = recorded
        .iter()
        .find(|(c, _)| *c == claim)
        .unwrap()
        .1
        .clone();
    assert_eq!(
        causes,
        vec![root, p.min(q)],
        "p e q morrem juntos; vale o menor hash"
    );
}

#[test]
fn recorded_root_is_reused_when_a_dependent_falls_later() {
    let fixture = Fixture::new();
    let mut ledger = fixture.ledger();
    let a = fixture.assumption(&mut ledger, &fixture.admin);
    let z = fixture.assumption(&mut ledger, &fixture.admin);
    let y = fixture.assumption(&mut ledger, &fixture.admin);
    let c = route(&mut ledger, &fixture, &[a, z], &[a, z]);
    let e1 = route(&mut ledger, &fixture, &[c, y], &[c]);
    let e2 = route(&mut ledger, &fixture, &[c, y], &[y]);
    let retractions = Retractions::new(fixture.admin.public_key().clone());
    let reason = || Text::new("passo").unwrap();
    // Primeiro o maior hash: recalcular a raiz de c depois apontaria para o menor.
    let (first, second) = if a > z { (a, z) } else { (z, a) };
    let root = retractions
        .retract(&mut ledger, &fixture.admin, first, reason())
        .unwrap()
        .retracted;
    retractions
        .retract(&mut ledger, &fixture.admin, second, reason())
        .unwrap();
    assert_eq!(
        kind(&ledger, e1),
        EntryKind::Verified,
        "nó de e1 vivo por e2"
    );
    let root_y = retractions
        .retract(&mut ledger, &fixture.admin, y, reason())
        .unwrap()
        .retracted;
    let recorded = records(&ledger);
    let causes = |claim| {
        recorded
            .iter()
            .find(|(c, _)| *c == claim)
            .unwrap()
            .1
            .clone()
    };
    assert_eq!(causes(c), vec![root, first]);
    assert_eq!(
        causes(e1),
        vec![root, c],
        "raiz gravada de c é reaproveitada"
    );
    assert_eq!(causes(e2), vec![root_y, y]);
}
