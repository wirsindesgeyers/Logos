//! P3/P4 e STATE15–16 através do transporte e do ledger reais.
#![cfg(target_os = "linux")]
mod support;

use logos_air::{Formula, FormulaNode, Limits, Vocabulary};
use logos_canon::{Text, Value, from_canon, to_canon};
use logos_core::{Budget, CheckOutcome};
use logos_crypto::{Hash, Signer, hash, hash_raw};
use logos_kernel::{
    certify::CertificationHandlers,
    registry::{CAKE, CheckerRegistry, LRAT, MODEL},
    server::{Handlers, Server},
    staging::Staging,
    wire::{Message, ObjectRef, Request, encode_frame},
};
use logos_ledger::{
    Authority, Certificate, CheckRecord, Checker, Event, EventBody, EventProposal, Ledger,
    ModelObject, Route, TaintedText,
};
use logos_model::{FiniteModel, ModelData, ModelLimits};
use logos_obligation::{DecodeLimits, EntryKind, Obligation};
use std::{
    io::{Read, Write},
    net::Shutdown,
    os::unix::net::UnixStream,
    sync::Mutex,
};
use support::*;

fn budget() -> Budget {
    Budget::new(2_000_000, 32 * 1024 * 1024, 4 * 1024 * 1024, 3000)
}
fn truth(value: bool) -> Formula {
    Formula::new(
        vec![if value {
            FormulaNode::True
        } else {
            FormulaNode::False
        }],
        Limits::default(),
    )
    .unwrap()
}
fn items(value: &Value) -> &[Value] {
    let Value::Array(a) = value else {
        panic!("expected array {value:?}")
    };
    a.items()
}
fn exchange<H: Handlers<logos_crypto::FileSigner>>(
    ledger: &mut Ledger<'_, logos_crypto::FileSigner>,
    h: &H,
    message: Message,
) -> Value {
    let temp = Temp::new();
    let uid = rustix::process::geteuid().as_raw();
    let server = Server::bind(&temp.0.join("socket"), uid, Default::default()).unwrap();
    let staging = Staging::open(
        &temp.dir("source"),
        &temp.dir("private"),
        uid,
        64 * 1024 * 1024,
    )
    .unwrap();
    let (mut client, stream) = UnixStream::pair().unwrap();
    let writer = Mutex::new(ledger);
    std::thread::scope(|scope| {
        let worker = scope.spawn(move || {
            // A janela WITNESS é lógica (1024 eventos), não um SLA de replay debug.
            // Limite finito de fixture para hosts carregados; protocolo/asserções intactos.
            client
                .set_read_timeout(Some(std::time::Duration::from_secs(600)))
                .unwrap();
            client
                .write_all(&encode_frame(&Request { id: 15, message }).unwrap())
                .unwrap();
            client.shutdown(Shutdown::Write).unwrap();
            let mut len = [0; 4];
            client.read_exact(&mut len).unwrap();
            let mut bytes = vec![0; u32::from_be_bytes(len) as usize];
            client.read_exact(&mut bytes).unwrap();
            from_canon(&bytes).unwrap()
        });
        server.serve_stream(stream, &writer, h, &staging).unwrap();
        worker.join().unwrap()
    })
}
fn setup(
    ledger: &mut Ledger<'_, logos_crypto::FileSigner>,
    signer: &logos_crypto::FileSigner,
    cake: Option<&std::path::Path>,
    b: Budget,
) -> CertificationHandlers {
    let registry = CheckerRegistry::new(signer.public_key().clone(), cake).unwrap();
    registry.install(ledger, signer).unwrap();
    CertificationHandlers::new(registry, b)
}
fn propose(
    ledger: &mut Ledger<'_, logos_crypto::FileSigner>,
    h: &CertificationHandlers,
    value: bool,
) -> (Hash<Obligation>, Vocabulary) {
    let v = Vocabulary::new(1, vec![], vec![]).unwrap();
    let hv = ledger
        .cas()
        .put::<Vocabulary>(&to_canon(&v).unwrap())
        .unwrap();
    let response = exchange(
        ledger,
        h,
        Message::Propose(EventProposal {
            text_pt: TaintedText::new("STATE15".into()).unwrap(),
            goal: truth(value),
            cites: vec![],
            speculative_on: vec![],
            vocabulary: hv,
        }),
    );
    assert_eq!(items(&response)[2], Value::Unsigned(1));
    let ho: Hash<Obligation> =
        logos_canon::CanonDecode::from_value(&items(&items(&response)[3])[1]).unwrap();
    (ho, v)
}
fn proof(
    ledger: &Ledger<'_, logos_crypto::FileSigner>,
    o: Hash<Obligation>,
    v: &Vocabulary,
) -> Hash<Certificate> {
    let bytes = ledger.cas().get(&o).unwrap().unwrap();
    let ground =
        logos_ground::ground_for_check(&bytes, o, v, DecodeLimits::default(), Default::default())
            .unwrap();
    let (cnf, _) = logos_ground::clausify(&ground).unwrap();
    let mut assigned = std::collections::BTreeMap::new();
    let mut hints = vec![];
    'propagate: loop {
        let mut progress = false;
        for (i, clause) in cnf.clauses().iter().enumerate() {
            if clause
                .iter()
                .any(|lit| assigned.get(&lit.abs()) == Some(&(*lit > 0)))
            {
                continue;
            }
            let remaining: Vec<_> = clause
                .iter()
                .filter(|lit| !assigned.contains_key(&lit.abs()))
                .copied()
                .collect();
            if remaining.is_empty() {
                hints.push(i + 1);
                break 'propagate;
            }
            if remaining.len() == 1 {
                hints.push(i + 1);
                assigned.insert(remaining[0].abs(), remaining[0] > 0);
                progress = true;
            }
        }
        assert!(
            progress,
            "fixture formula has no unit-propagation refutation"
        );
    }
    let hints = hints
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    let proof = format!("{} 0 {} 0\n", cnf.clauses().len() + 1, hints);
    ledger.cas().put::<Certificate>(proof.as_bytes()).unwrap()
}
fn certify(o: Hash<Obligation>, c: Hash<Certificate>, format: &str) -> Message {
    Message::Certify {
        obligation: o,
        format: Text::new(format).unwrap(),
        certificate: ObjectRef::Cas(c),
    }
}
fn success(response: &Value, state: u64) -> Hash<Event> {
    assert_eq!(items(response)[2], Value::Unsigned(1), "{response:?}");
    let fields = items(&items(response)[3]);
    assert_eq!(fields[0], Value::Unsigned(1));
    assert_eq!(fields[1], Value::Unsigned(state));
    logos_canon::CanonDecode::from_value(&fields[2]).unwrap()
}
fn refused(response: &Value) {
    assert_eq!(items(response)[2], Value::Unsigned(0), "{response:?}");
}
fn revoke(
    ledger: &mut Ledger<'_, logos_crypto::FileSigner>,
    signer: &logos_crypto::FileSigner,
    checker: Hash<Checker>,
) {
    let mut body = EventBody::CheckerRevoked {
        checker,
        reason: Text::new("operator revoked").unwrap(),
        authority: Authority {
            key: signer.key_id().unwrap(),
            sig: signer.sign::<Event>(b"").unwrap(),
        },
    };
    let message = body.authority_message().unwrap().unwrap();
    if let EventBody::CheckerRevoked { authority, .. } = &mut body {
        authority.sig = signer.sign::<Event>(&message).unwrap();
    }
    ledger.append(body).unwrap();
}

#[test]
fn state15_16_certify_persists_verified_citable_and_reopens_pending_privately() {
    let temp = Temp::new();
    let signer = signer();
    let path = temp.0.join("ledger");
    let mut ledger = Ledger::open(&path, &signer).unwrap();
    genesis(&mut ledger, &signer);
    let handlers = setup(&mut ledger, &signer, None, budget());
    ledger
        .append(EventBody::EpochAdvanced {
            new_epoch: 9,
            reason: Text::new("STATE15").unwrap(),
            policy_root: None,
            vocab: None,
        })
        .unwrap();
    let (o, v) = propose(&mut ledger, &handlers, true);
    let certificate = proof(&ledger, o, &v);
    drop(ledger);
    let mut ledger = Ledger::open(&path, &signer).unwrap();
    let verified = success(
        &exchange(&mut ledger, &handlers, certify(o, certificate, LRAT)),
        0,
    );
    let last = ledger
        .events_from(ledger.head().unwrap().unwrap().seq, 1)
        .unwrap()
        .pop()
        .unwrap();
    let EventBody::Verified {
        check,
        cited,
        used,
        consistency,
    } = last.body
    else {
        panic!("expected Verified")
    };
    assert_eq!(check.obligation, o);
    assert_eq!(check.certificate, certificate);
    assert_eq!(check.outcome, CheckOutcome::Accepted);
    assert!(cited.is_empty() && used.is_empty());
    assert_eq!(
        consistency,
        logos_ledger::Consistency::Unknown { check: None }
    );
    assert!(
        ledger
            .live_index()
            .unwrap()
            .is_citable(verified, 9)
            .unwrap()
    );
    let head = ledger.head().unwrap();
    refused(&exchange(
        &mut ledger,
        &handlers,
        certify(o, certificate, LRAT),
    ));
    assert_eq!(ledger.head().unwrap(), head);

    // Auditor independente não depende de kernel/ledger e recheca ambos os checkers.
    let cake = logos_lrat::CakeRegistry::register(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/cake-lpr/cake_lpr"
    )))
    .unwrap();
    let identity = hash_raw::<Checker>(&std::fs::read("/proc/self/exe").unwrap());
    drop(ledger);
    let report = logos_audit::verify_with_checkers(
        &path.join("ledger.sqlite3"),
        &path.join("cas"),
        signer.public_key(),
        Some(&cake),
        &[*identity.as_bytes()],
    )
    .unwrap();
    assert_eq!(report.lrat_proofs, 1);
    assert_eq!(report.unchecked_proofs, 0);
}

#[test]
fn rejected_and_unknown_never_verify_and_rejected_can_retry_after_reopen() {
    let temp = Temp::new();
    let signer = signer();
    let path = temp.0.join("ledger");
    let mut ledger = Ledger::open(&path, &signer).unwrap();
    genesis(&mut ledger, &signer);
    let handlers = setup(&mut ledger, &signer, None, budget());
    let (o, v) = propose(&mut ledger, &handlers, true);
    let good = proof(&ledger, o, &v);
    let bad = ledger.cas().put::<Certificate>(b"invalid proof").unwrap();
    let rejected = success(&exchange(&mut ledger, &handlers, certify(o, bad, LRAT)), 2);
    assert_eq!(
        ledger
            .live_index()
            .unwrap()
            .status(rejected)
            .unwrap()
            .unwrap()
            .kind,
        logos_obligation::EntryKind::Pending
    );
    let last = ledger
        .events_from(ledger.head().unwrap().unwrap().seq, 1)
        .unwrap();
    assert!(matches!(
        last[0].body,
        EventBody::CertificateRejected { .. }
    ));
    let empty = Budget::new(0, 32 * 1024 * 1024, 4 * 1024 * 1024, 3000);
    let limited = setup(&mut ledger, &signer, None, empty);
    let unknown = success(&exchange(&mut ledger, &limited, certify(o, good, LRAT)), 3);
    assert!(!ledger.live_index().unwrap().is_citable(unknown, 0).unwrap());
    drop(ledger);
    let mut ledger = Ledger::open(&path, &signer).unwrap();
    success(&exchange(&mut ledger, &handlers, certify(o, good, LRAT)), 0);
}

#[test]
fn unknown_unregistered_revoked_and_wrong_format_do_not_authorize() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let registry = CheckerRegistry::new(signer.public_key().clone(), None).unwrap();
    let identity = registry.builtin_identity();
    // O objeto existir no CAS não equivale a registro administrativo ativo.
    ledger
        .cas()
        .put::<Checker>(&std::fs::read("/proc/self/exe").unwrap())
        .unwrap();
    let handlers = CertificationHandlers::new(registry, budget());
    let (o, v) = propose(&mut ledger, &handlers, true);
    let c = proof(&ledger, o, &v);
    let head = ledger.head().unwrap();
    for format in [LRAT, CAKE, MODEL, "unknown/v1"] {
        refused(&exchange(&mut ledger, &handlers, certify(o, c, format)));
        assert_eq!(ledger.head().unwrap(), head);
    }
    let handlers = setup(&mut ledger, &signer, None, budget());
    revoke(&mut ledger, &signer, identity);
    // Startup/install does not undo a durable revocation.
    let _again = setup(&mut ledger, &signer, None, budget());
    let head = ledger.head().unwrap();
    refused(&exchange(&mut ledger, &handlers, certify(o, c, LRAT)));
    assert_eq!(ledger.head().unwrap(), head);
}

#[test]
fn stale_epoch_arbitrary_cas_obligation_and_orphan_creation_are_refused() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let handlers = setup(&mut ledger, &signer, None, budget());
    let (o, v) = propose(&mut ledger, &handlers, true);
    let c = proof(&ledger, o, &v);
    ledger
        .append(EventBody::EpochAdvanced {
            new_epoch: 1,
            reason: Text::new("epoch").unwrap(),
            policy_root: None,
            vocab: None,
        })
        .unwrap();
    let head = ledger.head().unwrap();
    refused(&exchange(&mut ledger, &handlers, certify(o, c, LRAT)));
    assert_eq!(ledger.head().unwrap(), head);
    let fake = ledger
        .cas()
        .put::<Obligation>(b"untrusted obligation")
        .unwrap();
    refused(&exchange(&mut ledger, &handlers, certify(fake, c, LRAT)));
    ledger
        .append(EventBody::ObligationCreated {
            obligation: fake,
            route: Route::Model,
            artifact: *fake.as_bytes(),
            cnf: None,
        })
        .unwrap();
    let head = ledger.head().unwrap();
    refused(&exchange(&mut ledger, &handlers, certify(fake, c, LRAT)));
    assert_eq!(ledger.head().unwrap(), head);
}

#[test]
fn state15_refutation_requires_actual_accepted_model_and_active_registry() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let handlers = setup(&mut ledger, &signer, None, budget());
    let (o, v) = propose(&mut ledger, &handlers, false);
    let model = FiniteModel::new(
        ModelData {
            vocab: hash(&v).unwrap(),
            domains: vec![],
            consts: vec![],
            funcs: vec![],
            preds: vec![],
        },
        &v,
        ModelLimits::default(),
    )
    .unwrap();
    let model = ledger
        .cas()
        .put::<ModelObject>(&to_canon(&model).unwrap())
        .unwrap();
    let rejected = ledger.cas().put::<ModelObject>(b"bad model").unwrap();
    success(
        &exchange(
            &mut ledger,
            &handlers,
            Message::Refute {
                obligation: o,
                model: ObjectRef::Cas(rejected),
            },
        ),
        2,
    );
    let refuted = success(
        &exchange(
            &mut ledger,
            &handlers,
            Message::Refute {
                obligation: o,
                model: ObjectRef::Cas(model),
            },
        ),
        1,
    );
    let index = ledger.live_index().unwrap();
    assert_eq!(
        index.status(refuted).unwrap().unwrap().kind,
        logos_obligation::EntryKind::Refuted
    );
    assert!(!index.is_citable(refuted, 0).unwrap());
    drop(index);
    let (o, _) = propose(&mut ledger, &handlers, true);
    success(
        &exchange(
            &mut ledger,
            &handlers,
            Message::Refute {
                obligation: o,
                model: ObjectRef::Cas(model),
            },
        ),
        2,
    );
    let registry = CheckerRegistry::new(signer.public_key().clone(), None).unwrap();
    revoke(&mut ledger, &signer, registry.builtin_identity());
    let head = ledger.head().unwrap();
    refused(&exchange(
        &mut ledger,
        &handlers,
        Message::Refute {
            obligation: o,
            model: ObjectRef::Cas(model),
        },
    ));
    assert_eq!(ledger.head().unwrap(), head);
}

#[test]
fn adr0030_binary_substitution_same_name_is_refused_by_certify() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let binary = temp.0.join("cake_lpr");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/cake-lpr/cake_lpr"
        ),
        &binary,
    )
    .unwrap();
    let handlers = setup(&mut ledger, &signer, Some(&binary), budget());
    let (o, v) = propose(&mut ledger, &handlers, true);
    let c = proof(&ledger, o, &v);
    // The original registered binary really runs successfully before substitution.
    success(&exchange(&mut ledger, &handlers, certify(o, c, CAKE)), 0);
    let (o, v) = propose(&mut ledger, &handlers, true);
    let c = proof(&ledger, o, &v);
    std::fs::write(&binary, b"different executable with the same name").unwrap();
    let head = ledger.head().unwrap();
    refused(&exchange(&mut ledger, &handlers, certify(o, c, CAKE)));
    assert_eq!(ledger.head().unwrap(), head);
    assert_ne!(
        hash_raw::<Checker>(b"different executable with the same name"),
        hash_raw::<Checker>(&std::fs::read("/proc/self/exe").unwrap())
    );
}

fn candidate(
    ledger: &Ledger<'_, logos_crypto::FileSigner>,
    v: &Vocabulary,
    goal: Formula,
    cites: Vec<Hash<Event>>,
) -> EventProposal {
    EventProposal {
        text_pt: TaintedText::new("context fixture".into()).unwrap(),
        goal,
        cites,
        speculative_on: vec![],
        vocabulary: ledger
            .cas()
            .put::<Vocabulary>(&to_canon(v).unwrap())
            .unwrap(),
    }
}

fn proposed(
    ledger: &mut Ledger<'_, logos_crypto::FileSigner>,
    handlers: &CertificationHandlers,
    p: EventProposal,
) -> Hash<Obligation> {
    let reply = exchange(ledger, handlers, Message::Propose(p));
    assert_eq!(items(&reply)[2], Value::Unsigned(1), "{reply:?}");
    logos_canon::CanonDecode::from_value(&items(&items(&reply)[3])[1]).unwrap()
}

#[test]
fn used_is_derived_from_the_proof_and_revoked_citations_cannot_be_recovered() {
    use logos_air::{Grounding, Symbol, SymbolDecl, SymbolKind, SymbolResult};
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let handlers = setup(&mut ledger, &signer, None, budget());
    let symbols: Vec<_> = ["P", "Q"]
        .into_iter()
        .map(|n| Symbol {
            name: Text::new(n).unwrap(),
            kind: SymbolKind::Pred,
            arity: vec![],
            result: SymbolResult::Bool,
        })
        .collect();
    let v = Vocabulary::new(
        1,
        vec![],
        symbols
            .iter()
            .map(|s| SymbolDecl {
                signature: s.clone(),
                grounding: Grounding::Glossed(Text::new("fixture").unwrap()),
                gloss_pt: Text::new("fixture").unwrap(),
            })
            .collect(),
    )
    .unwrap();
    let formulas: Vec<_> = symbols
        .iter()
        .map(|s| {
            Formula::new(
                vec![FormulaNode::Pred {
                    symbol: s.clone(),
                    args: vec![],
                }],
                Limits::default(),
            )
            .unwrap()
        })
        .collect();
    let hv = ledger
        .cas()
        .put::<Vocabulary>(&to_canon(&v).unwrap())
        .unwrap();
    let mut cites = Vec::new();
    for formula in &formulas {
        cites.push(
            hash(
                &ledger
                    .append(EventBody::AssumptionDeclared {
                        vocabulary: hv,
                        formula: formula.clone(),
                        scope: Text::new("").unwrap(),
                        author: signer.key_id().unwrap(),
                    })
                    .unwrap(),
            )
            .unwrap(),
        );
    }
    let p = candidate(&ledger, &v, formulas[0].clone(), cites.clone());
    let o = proposed(&mut ledger, &handlers, p);
    let c = proof(&ledger, o, &v);
    let verified = success(&exchange(&mut ledger, &handlers, certify(o, c, LRAT)), 0);
    let last = ledger
        .events_from(ledger.head().unwrap().unwrap().seq, 1)
        .unwrap()
        .pop()
        .unwrap();
    let EventBody::Verified { used, .. } = last.body else {
        panic!("Verified")
    };
    assert_eq!(used, vec![cites[0]]);
    let index = ledger.live_index().unwrap();
    assert_eq!(index.dependents(cites[0]).unwrap(), vec![verified]);
    assert!(index.dependents(cites[1]).unwrap().is_empty());
    drop(index);
    // A Pending criado antes da perda de elegibilidade não autoriza com snapshot velho.
    let p = candidate(&ledger, &v, formulas[1].clone(), cites.clone());
    let o = proposed(&mut ledger, &handlers, p);
    let c = proof(&ledger, o, &v);
    ledger
        .append(EventBody::Invalidated {
            claim: cites[1],
            causes: vec![cites[1]],
        })
        .unwrap();
    let head = ledger.head().unwrap();
    refused(&exchange(&mut ledger, &handlers, certify(o, c, LRAT)));
    assert_eq!(ledger.head().unwrap(), head);
}

#[test]
fn enumeration_sources_are_not_promoted_by_hash_or_cast() {
    use logos_air::{
        DomainSource, Grounding, PolicyRef, Sort, SortDecl, SortDomain, Symbol, SymbolDecl,
        SymbolKind, SymbolResult,
    };
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let handlers = setup(&mut ledger, &signer, None, budget());
    let constant = Symbol {
        name: Text::new("c").unwrap(),
        kind: SymbolKind::Const,
        arity: vec![],
        result: SymbolResult::Sort(Sort {
            name: Text::new("S").unwrap(),
        }),
    };
    let v = Vocabulary::new(
        1,
        vec![SortDecl {
            name: Text::new("S").unwrap(),
            domain: SortDomain::Enumerated {
                constants: vec![constant.clone()],
                domain_source: DomainSource::Policy(PolicyRef([7; 32])),
            },
        }],
        vec![SymbolDecl {
            signature: constant,
            grounding: Grounding::Glossed(Text::new("fixture").unwrap()),
            gloss_pt: Text::new("fixture").unwrap(),
        }],
    )
    .unwrap();
    // 6.1b (ADR0062): fonte Policy continua não resolvida até a 7.3; PROPOSE já recusa,
    // pelas duas rotas (LRAT e modelo), sem gravar nada.
    // Com o fecho citado como hipótese o grounder aceitaria; só a recusa de Policy barra.
    let closure = logos_ground::domain_axioms(
        &v,
        &Sort {
            name: Text::new("S").unwrap(),
        },
    )
    .unwrap()
    .closure()
    .clone();
    let message = assume_message(&ledger, &v, closure, "session:fonte");
    let reply = exchange(&mut ledger, &handlers, message);
    let axiom = assumed(&reply);
    for (goal, cites) in [
        (truth(true), vec![]),
        (truth(false), vec![]),
        (truth(true), vec![axiom]),
    ] {
        let head = ledger.head().unwrap();
        let p = candidate(&ledger, &v, goal, cites);
        refused(&exchange(&mut ledger, &handlers, Message::Propose(p)));
        assert_eq!(ledger.head().unwrap(), head);
    }
}

#[test]
fn p3_p4_generated_certificate_corruptions_and_cross_obligation_proofs_never_authorize() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let handlers = setup(&mut ledger, &signer, None, budget());
    let (o, v) = propose(&mut ledger, &handlers, true);
    let good = proof(&ledger, o, &v);
    let original = ledger.cas().get(&good).unwrap().unwrap();
    // A byte substitution at every non-whitespace position produces an invalid proof
    // (ADR0057: `LOGOS_TIER=fast` substitutes every 16th position).
    let stride = if std::env::var("LOGOS_TIER").is_ok_and(|tier| tier == "fast") {
        16
    } else {
        1
    };
    for i in (0..original.len()).step_by(stride) {
        if original[i].is_ascii_whitespace() {
            continue;
        }
        let mut corrupt = original.clone();
        corrupt[i] = b'x';
        let c = ledger.cas().put::<Certificate>(&corrupt).unwrap();
        let event = success(&exchange(&mut ledger, &handlers, certify(o, c, LRAT)), 2);
        assert!(!ledger.live_index().unwrap().is_citable(event, 0).unwrap());
    }
    let (other, _) = propose(&mut ledger, &handlers, false);
    success(
        &exchange(&mut ledger, &handlers, certify(other, good, LRAT)),
        2,
    );
    let head = ledger.head().unwrap();
    refused(&exchange(
        &mut ledger,
        &handlers,
        certify(Hash::from_bytes([9; 32]), good, LRAT),
    ));
    assert_eq!(ledger.head().unwrap(), head);
    success(&exchange(&mut ledger, &handlers, certify(o, good, LRAT)), 0);
    for event in ledger.events_from(0, 256).unwrap() {
        if let EventBody::Verified { check, .. } = event.body {
            assert_eq!(check.outcome, CheckOutcome::Accepted);
            assert_eq!(
                check.checker,
                hash_raw::<Checker>(&std::fs::read("/proc/self/exe").unwrap())
            );
        }
    }
}

#[test]
fn forged_administrative_signature_cannot_activate_a_locally_known_checker() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let registry = CheckerRegistry::new(signer.public_key().clone(), None).unwrap();
    let identity = registry.builtin_identity();
    assert_eq!(
        ledger
            .cas()
            .put::<Checker>(&std::fs::read("/proc/self/exe").unwrap())
            .unwrap(),
        identity
    );
    ledger
        .append(EventBody::CheckerRegistered {
            checker: identity,
            format: Text::new(LRAT).unwrap(),
            authority: Authority {
                key: signer.key_id().unwrap(),
                sig: signer
                    .sign::<Event>(b"wrong administrative payload")
                    .unwrap(),
            },
        })
        .unwrap();
    let handlers = CertificationHandlers::new(registry, budget());
    let (o, v) = propose(&mut ledger, &handlers, true);
    let c = proof(&ledger, o, &v);
    let head = ledger.head().unwrap();
    refused(&exchange(&mut ledger, &handlers, certify(o, c, LRAT)));
    assert_eq!(ledger.head().unwrap(), head);
}

#[test]
fn signed_pending_cannot_bind_another_proposals_obligation() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let handlers = setup(&mut ledger, &signer, None, budget());
    let (o, v) = propose(&mut ledger, &handlers, true);
    let c = proof(&ledger, o, &v);
    let false_proposal = candidate(&ledger, &v, truth(false), vec![]);
    ledger
        .append(EventBody::Proposed {
            proposal: false_proposal,
            author: signer.key_id().unwrap(),
        })
        .unwrap();
    ledger
        .append(EventBody::ObligationCreated {
            obligation: o,
            route: Route::Model,
            artifact: *o.as_bytes(),
            cnf: None,
        })
        .unwrap();
    let head = ledger.head().unwrap();
    refused(&exchange(&mut ledger, &handlers, certify(o, c, LRAT)));
    assert_eq!(ledger.head().unwrap(), head);
}

fn witness(o: Hash<Obligation>, model: Hash<ModelObject>) -> Message {
    Message::Witness {
        obligation: o,
        model: ObjectRef::Cas(model),
    }
}
fn empty_model(ledger: &Ledger<'_, logos_crypto::FileSigner>, v: &Vocabulary) -> Hash<ModelObject> {
    let m = FiniteModel::new(
        ModelData {
            vocab: hash(v).unwrap(),
            domains: vec![],
            consts: vec![],
            funcs: vec![],
            preds: vec![],
        },
        v,
        ModelLimits::default(),
    )
    .unwrap();
    ledger
        .cas()
        .put::<ModelObject>(&to_canon(&m).unwrap())
        .unwrap()
}

#[test]
fn witness_before_certify_survives_restart_and_is_not_a_countermodel() {
    use logos_kernel::witness::PremiseUse;
    let temp = Temp::new();
    let signer = signer();
    let path = temp.0.join("ledger");
    let mut ledger = Ledger::open(&path, &signer).unwrap();
    genesis(&mut ledger, &signer);
    let h = setup(&mut ledger, &signer, None, budget());
    let (o, v) = propose(&mut ledger, &h, true);
    let m = empty_model(&ledger, &v);
    let w = success(&exchange(&mut ledger, &h, witness(o, m)), 0);
    assert!(ledger.live_index().unwrap().status(w).unwrap().is_none());
    drop(ledger);
    let mut ledger = Ledger::open(&path, &signer).unwrap();
    let c = proof(&ledger, o, &v);
    let verified = success(&exchange(&mut ledger, &h, certify(o, c, LRAT)), 0);
    let e = ledger
        .events_from(ledger.head().unwrap().unwrap().seq, 1)
        .unwrap()
        .pop()
        .unwrap();
    assert!(
        matches!(e.body,EventBody::Verified { consistency: logos_ledger::Consistency::Witness { model, .. }, .. } if model==m)
    );
    assert!(
        h.premise_eligible(&ledger, verified, PremiseUse::Logical)
            .unwrap()
    );
    assert!(
        h.premise_eligible(&ledger, verified, PremiseUse::Action)
            .unwrap()
    );
    assert!(matches!(
        h.effective_consistency(&ledger, verified).unwrap(),
        logos_ledger::Consistency::Witness { .. }
    ));
    let (same, _) = propose(&mut ledger, &h, true);
    assert_eq!(same, o);
    assert!(
        h.premise_eligible(&ledger, verified, PremiseUse::Action)
            .unwrap()
    );
}

#[test]
fn witness_after_certify_preserves_signed_bytes_and_failed_retry_cannot_downgrade() {
    use logos_kernel::witness::PremiseUse;
    let temp = Temp::new();
    let signer = signer();
    let path = temp.0.join("ledger");
    let mut ledger = Ledger::open(&path, &signer).unwrap();
    genesis(&mut ledger, &signer);
    let h = setup(&mut ledger, &signer, None, budget());
    let (o, v) = propose(&mut ledger, &h, true);
    let c = proof(&ledger, o, &v);
    let verified = success(&exchange(&mut ledger, &h, certify(o, c, LRAT)), 0);
    let seq = ledger.head().unwrap().unwrap().seq;
    let before = to_canon(&ledger.events_from(seq, 1).unwrap()[0]).unwrap();
    assert!(matches!(
        h.effective_consistency(&ledger, verified).unwrap(),
        logos_ledger::Consistency::Unknown { check: None }
    ));
    assert!(
        !h.premise_eligible(&ledger, verified, PremiseUse::Action)
            .unwrap()
    );
    let m = empty_model(&ledger, &v);
    success(&exchange(&mut ledger, &h, witness(o, m)), 0);
    let bad = ledger.cas().put::<ModelObject>(b"not model").unwrap();
    success(&exchange(&mut ledger, &h, witness(o, bad)), 1);
    assert!(
        h.premise_eligible(&ledger, verified, PremiseUse::Action)
            .unwrap()
    );
    assert_eq!(
        to_canon(&ledger.events_from(seq, 1).unwrap()[0]).unwrap(),
        before
    );
    drop(ledger);
    let ledger = Ledger::open(&path, &signer).unwrap();
    assert!(
        h.premise_eligible(&ledger, verified, PremiseUse::Action)
            .unwrap()
    );
    let cake = logos_lrat::CakeRegistry::register(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/cake-lpr/cake_lpr"
    )))
    .unwrap();
    let checker = hash_raw::<Checker>(&std::fs::read("/proc/self/exe").unwrap());
    drop(ledger);
    let report = logos_audit::verify_with_checkers(
        &path.join("ledger.sqlite3"),
        &path.join("cas"),
        signer.public_key(),
        Some(&cake),
        &[*checker.as_bytes()],
    )
    .unwrap();
    assert_eq!(report.consistencies, 1);
    assert_eq!(report.lrat_proofs, 1);
}

#[test]
fn witness_rejected_unknown_and_no_attempt_remain_distinct_without_refutation() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let h = setup(&mut ledger, &signer, None, budget());
    let (o, v) = propose(&mut ledger, &h, true);
    let bad = ledger.cas().put::<ModelObject>(b"bad").unwrap();
    success(&exchange(&mut ledger, &h, witness(o, bad)), 1);
    let zero = setup(
        &mut ledger,
        &signer,
        None,
        Budget::new(0, 32 * 1024 * 1024, 4 * 1024 * 1024, 3000),
    );
    let m = empty_model(&ledger, &v);
    success(&exchange(&mut ledger, &zero, witness(o, m)), 2);
    let c = proof(&ledger, o, &v);
    let verified = success(&exchange(&mut ledger, &h, certify(o, c, LRAT)), 0);
    assert!(matches!(
        h.effective_consistency(&ledger, verified).unwrap(),
        logos_ledger::Consistency::Unknown {
            check: Some(CheckRecord {
                outcome: CheckOutcome::Unknown(_),
                ..
            })
        }
    ));
    for event in ledger.events_from(0, 100).unwrap() {
        assert!(!matches!(event.body, EventBody::Refuted { .. }));
    }
}

#[test]
fn inconsistent_gamma_proves_arbitrary_goal_but_never_authorizes_action() {
    use logos_air::{Grounding, Symbol, SymbolDecl, SymbolKind, SymbolResult};
    use logos_kernel::witness::PremiseUse;
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let h = setup(&mut ledger, &signer, None, budget());
    let p = Symbol {
        name: Text::new("P").unwrap(),
        kind: SymbolKind::Pred,
        arity: vec![],
        result: SymbolResult::Bool,
    };
    let v = Vocabulary::new(
        1,
        vec![],
        vec![SymbolDecl {
            signature: p.clone(),
            grounding: Grounding::Glossed(Text::new("P").unwrap()),
            gloss_pt: Text::new("P").unwrap(),
        }],
    )
    .unwrap();
    let hv = ledger
        .cas()
        .put::<Vocabulary>(&to_canon(&v).unwrap())
        .unwrap();
    let atom = FormulaNode::Pred {
        symbol: p,
        args: vec![],
    };
    let mut cites = vec![];
    for nodes in [vec![atom.clone()], vec![FormulaNode::Not, atom]] {
        let f = Formula::new(nodes, Limits::default()).unwrap();
        cites.push(
            hash(
                &ledger
                    .append(EventBody::AssumptionDeclared {
                        vocabulary: hv,
                        formula: f,
                        scope: Text::new("").unwrap(),
                        author: signer.key_id().unwrap(),
                    })
                    .unwrap(),
            )
            .unwrap(),
        );
    }
    // Goal False is stronger than an arbitrary proposition: actual explosion proof.
    let prop = candidate(&ledger, &v, truth(false), cites);
    let o = proposed(&mut ledger, &h, prop);
    for value in [false, true] {
        let m = FiniteModel::new(
            ModelData {
                vocab: hv,
                domains: vec![],
                consts: vec![],
                funcs: vec![],
                preds: vec![(0, if value { vec![vec![]] } else { vec![] })],
            },
            &v,
            ModelLimits::default(),
        )
        .unwrap();
        let m = ledger
            .cas()
            .put::<ModelObject>(&to_canon(&m).unwrap())
            .unwrap();
        success(&exchange(&mut ledger, &h, witness(o, m)), 1);
    }
    let c = proof(&ledger, o, &v);
    let verified = success(&exchange(&mut ledger, &h, certify(o, c, LRAT)), 0);
    assert!(matches!(
        h.effective_consistency(&ledger, verified).unwrap(),
        logos_ledger::Consistency::Unknown {
            check: Some(CheckRecord {
                outcome: CheckOutcome::Rejected(_),
                ..
            })
        }
    ));
    assert!(
        h.premise_eligible(&ledger, verified, PremiseUse::Logical)
            .unwrap()
    );
    assert!(
        !h.premise_eligible(&ledger, verified, PremiseUse::Action)
            .unwrap()
    );
}

#[test]
fn witness_deadline_is_inclusive_and_restart_does_not_renew_it() {
    use logos_kernel::witness::WITNESS_WINDOW_EVENTS;
    let temp = Temp::new();
    let signer = signer();
    let path = temp.0.join("ledger");
    let mut ledger = Ledger::open(&path, &signer).unwrap();
    genesis(&mut ledger, &signer);
    let h = setup(&mut ledger, &signer, None, budget());
    let (o, v) = propose(&mut ledger, &h, true);
    let m = empty_model(&ledger, &v);
    let pending = ledger.head().unwrap().unwrap().seq;
    // Um corpo só: `candidate` grava no CAS (fsync) e a janela tem 1024 eventos.
    let filler = EventBody::Proposed {
        proposal: candidate(&ledger, &v, truth(true), vec![]),
        author: signer.key_id().unwrap(),
    };
    for _ in pending + 1..pending + WITNESS_WINDOW_EVENTS {
        ledger.append(filler.clone()).unwrap();
    }
    success(&exchange(&mut ledger, &h, witness(o, m)), 0);
    assert_eq!(
        ledger.head().unwrap().unwrap().seq,
        pending + WITNESS_WINDOW_EVENTS
    );
    drop(ledger);
    let mut ledger = Ledger::open(&path, &signer).unwrap();
    let head = ledger.head().unwrap();
    refused(&exchange(&mut ledger, &h, witness(o, m)));
    assert_eq!(ledger.head().unwrap(), head);
    let c = proof(&ledger, o, &v);
    let verified = success(&exchange(&mut ledger, &h, certify(o, c, LRAT)), 0);
    assert!(
        h.premise_eligible(&ledger, verified, logos_kernel::witness::PremiseUse::Action)
            .unwrap()
    );
}

#[test]
fn witness_rejects_stale_epoch_revoked_checker_and_other_instance() {
    use logos_kernel::witness::PremiseUse;
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let h = setup(&mut ledger, &signer, None, budget());
    let (o, v) = propose(&mut ledger, &h, true);
    let m = empty_model(&ledger, &v);
    success(&exchange(&mut ledger, &h, witness(o, m)), 0);
    // A second proposal has identical O, but cannot inherit the first instance's witness.
    let (same, _) = propose(&mut ledger, &h, true);
    assert_eq!(same, o);
    let c = proof(&ledger, o, &v);
    let verified = success(&exchange(&mut ledger, &h, certify(o, c, LRAT)), 0);
    assert!(
        !h.premise_eligible(&ledger, verified, PremiseUse::Action)
            .unwrap()
    );
    success(&exchange(&mut ledger, &h, witness(o, m)), 0);
    let registry = CheckerRegistry::new(signer.public_key().clone(), None).unwrap();
    revoke(&mut ledger, &signer, registry.builtin_identity());
    let head = ledger.head().unwrap();
    refused(&exchange(&mut ledger, &h, witness(o, m)));
    assert_eq!(ledger.head().unwrap(), head);
    assert!(
        !h.premise_eligible(&ledger, verified, PremiseUse::Action)
            .unwrap()
    );
    ledger
        .append(EventBody::EpochAdvanced {
            new_epoch: 1,
            reason: Text::new("epoch").unwrap(),
            policy_root: None,
            vocab: None,
        })
        .unwrap();
    let head = ledger.head().unwrap();
    refused(&exchange(&mut ledger, &h, witness(o, m)));
    assert_eq!(ledger.head().unwrap(), head);
}

#[test]
fn witness_missing_context_and_invalidation_never_create_consistency_authority() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let h = setup(&mut ledger, &signer, None, budget());
    let (o, v) = propose(&mut ledger, &h, true);
    let m = empty_model(&ledger, &v);
    for message in [
        witness(Hash::from_bytes([99; 32]), m),
        witness(o, Hash::from_bytes([88; 32])),
    ] {
        let head = ledger.head().unwrap();
        refused(&exchange(&mut ledger, &h, message));
        assert_eq!(ledger.head().unwrap(), head);
    }
    // Consistency is about Gamma, independent of whether the goal is true or false.
    let (other, _) = propose(&mut ledger, &h, false);
    success(&exchange(&mut ledger, &h, witness(other, m)), 0);
    let c = proof(&ledger, o, &v);
    let verified = success(&exchange(&mut ledger, &h, certify(o, c, LRAT)), 0);
    ledger
        .append(EventBody::Invalidated {
            claim: verified,
            causes: vec![verified],
        })
        .unwrap();
    let head = ledger.head().unwrap();
    refused(&exchange(&mut ledger, &h, witness(o, m)));
    assert_eq!(ledger.head().unwrap(), head);
    assert!(
        !h.premise_eligible(&ledger, verified, logos_kernel::witness::PremiseUse::Action)
            .unwrap()
    );
}

#[test]
fn independent_auditor_rejects_wrong_annex_target_and_forged_accepted_model() {
    for wrong_target in [true, false] {
        let temp = Temp::new();
        let signer = signer();
        let path = temp.0.join("ledger");
        let mut ledger = Ledger::open(&path, &signer).unwrap();
        genesis(&mut ledger, &signer);
        let h = setup(&mut ledger, &signer, None, budget());
        let (o, v) = propose(&mut ledger, &h, true);
        let target = ledger.head().unwrap().unwrap().hash;
        let m = empty_model(&ledger, &v);
        let good = success(&exchange(&mut ledger, &h, witness(o, m)), 0);
        let event = ledger
            .events_from(ledger.head().unwrap().unwrap().seq, 1)
            .unwrap()
            .pop()
            .unwrap();
        let EventBody::ConsistencyRecorded { mut check, .. } = event.body else {
            panic!("record")
        };
        let bad = ledger.cas().put::<ModelObject>(b"not a model").unwrap();
        let model = if wrong_target { m } else { bad };
        check.certificate = ledger
            .cas()
            .put(&ledger.cas().get(&model).unwrap().unwrap())
            .unwrap();
        ledger
            .append(EventBody::ConsistencyRecorded {
                target: if wrong_target { good } else { target },
                check,
                model,
            })
            .unwrap();
        drop(ledger);
        assert!(
            logos_audit::verify(
                &path.join("ledger.sqlite3"),
                &path.join("cas"),
                signer.public_key()
            )
            .is_err()
        );
    }
}

/// 5.6/S4: refutar e re-propor com citações em outra ordem → AlreadyRefuted(h(M)).
fn refuted_setup() -> (Temp, logos_crypto::FileSigner) {
    (Temp::new(), signer())
}

fn assume(
    ledger: &mut Ledger<'_, logos_crypto::FileSigner>,
    signer: &logos_crypto::FileSigner,
    v: &Vocabulary,
) -> Hash<Event> {
    let hv = ledger
        .cas()
        .put::<Vocabulary>(&to_canon(v).unwrap())
        .unwrap();
    hash(
        &ledger
            .append(EventBody::AssumptionDeclared {
                vocabulary: hv,
                formula: truth(true),
                scope: Text::new("").unwrap(),
                author: signer.key_id().unwrap(),
            })
            .unwrap(),
    )
    .unwrap()
}

fn refute_false_goal(
    ledger: &mut Ledger<'_, logos_crypto::FileSigner>,
    handlers: &CertificationHandlers,
    v: &Vocabulary,
    cites: Vec<Hash<Event>>,
) -> (Hash<Obligation>, Hash<Event>, Hash<ModelObject>) {
    let p = candidate(ledger, v, truth(false), cites);
    let o = proposed(ledger, handlers, p);
    let model = FiniteModel::new(
        ModelData {
            vocab: hash(v).unwrap(),
            domains: vec![],
            consts: vec![],
            funcs: vec![],
            preds: vec![],
        },
        v,
        ModelLimits::default(),
    )
    .unwrap();
    let model = ledger
        .cas()
        .put::<ModelObject>(&to_canon(&model).unwrap())
        .unwrap();
    let refuted = success(
        &exchange(
            ledger,
            handlers,
            Message::Refute {
                obligation: o,
                model: ObjectRef::Cas(model),
            },
        ),
        1,
    );
    (o, refuted, model)
}

#[test]
fn s4_reproposal_with_reordered_citations_is_refused_with_the_countermodel() {
    let (temp, signer) = refuted_setup();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let handlers = setup(&mut ledger, &signer, None, budget());
    let v = Vocabulary::new(1, vec![], vec![]).unwrap();
    let a = assume(&mut ledger, &signer, &v);
    let b = assume(&mut ledger, &signer, &v);
    let (o, refuted, model) = refute_false_goal(&mut ledger, &handlers, &v, vec![a, b]);
    let head = ledger.head().unwrap();
    let objects = std::fs::read_dir(temp.0.join("ledger/cas"))
        .unwrap()
        .count();
    let again = candidate(&ledger, &v, truth(false), vec![b, a]);
    let reply = exchange(&mut ledger, &handlers, Message::Propose(again));
    refused(&reply);
    let detail = items(&items(&reply)[3]);
    assert_eq!(detail[0], Value::Unsigned(5), "AlreadyRefuted");
    use logos_canon::Canon;
    assert_eq!(detail[1], o.to_value().unwrap());
    assert_eq!(detail[2], refuted.to_value().unwrap());
    assert_eq!(detail[3], model.to_value().unwrap());
    assert_eq!(ledger.head().unwrap(), head, "nenhum evento novo");
    assert_eq!(
        std::fs::read_dir(temp.0.join("ledger/cas"))
            .unwrap()
            .count(),
        objects,
        "nenhum objeto órfão no CAS"
    );
    // Outra obrigação (outro objetivo) continua aceita.
    let other = candidate(&ledger, &v, truth(true), vec![b, a]);
    proposed(&mut ledger, &handlers, other);
}

#[test]
fn s4_timeout_on_a_second_attempt_cannot_turn_refutation_into_unknown() {
    let (temp, signer) = refuted_setup();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let handlers = setup(&mut ledger, &signer, None, budget());
    let v = Vocabulary::new(1, vec![], vec![]).unwrap();
    let a = assume(&mut ledger, &signer, &v);
    let (o, refuted, model) = refute_false_goal(&mut ledger, &handlers, &v, vec![a]);
    // Prover que estoura orçamento na segunda tentativa: handlers com 1 passo.
    let starved = CertificationHandlers::new(
        CheckerRegistry::new(signer.public_key().clone(), None).unwrap(),
        Budget::new(1, 1024, 1024, 0),
    );
    let head = ledger.head().unwrap();
    let again = candidate(&ledger, &v, truth(false), vec![a]);
    refused(&exchange(&mut ledger, &starved, Message::Propose(again)));
    let certificate = ledger.cas().put::<Certificate>(b"1 0 1 0\n").unwrap();
    refused(&exchange(
        &mut ledger,
        &starved,
        certify(o, certificate, LRAT),
    ));
    refused(&exchange(
        &mut ledger,
        &starved,
        Message::Refute {
            obligation: o,
            model: ObjectRef::Cas(model),
        },
    ));
    assert_eq!(ledger.head().unwrap(), head, "nenhum Unknown/Pending novo");
    let status = ledger
        .live_index()
        .unwrap()
        .status(refuted)
        .unwrap()
        .unwrap();
    assert_eq!(status.kind, logos_obligation::EntryKind::Refuted);
}

/// 5.7: vocabulário com predicado nulário P, para assumption/goal não triviais.
fn p_vocabulary() -> (Vocabulary, Formula) {
    use logos_air::{Grounding, Symbol, SymbolDecl, SymbolKind, SymbolResult};
    let symbol = Symbol {
        name: Text::new("P").unwrap(),
        kind: SymbolKind::Pred,
        arity: vec![],
        result: SymbolResult::Bool,
    };
    let v = Vocabulary::new(
        1,
        vec![],
        vec![SymbolDecl {
            signature: symbol.clone(),
            grounding: Grounding::Glossed(Text::new("P").unwrap()),
            gloss_pt: Text::new("P").unwrap(),
        }],
    )
    .unwrap();
    let p = Formula::new(
        vec![FormulaNode::Pred {
            symbol,
            args: vec![],
        }],
        Limits::default(),
    )
    .unwrap();
    (v, p)
}

fn assume_message(
    ledger: &Ledger<'_, logos_crypto::FileSigner>,
    v: &Vocabulary,
    formula: Formula,
    scope: &str,
) -> Message {
    Message::Assume {
        vocabulary: ledger
            .cas()
            .put::<Vocabulary>(&to_canon(v).unwrap())
            .unwrap(),
        formula,
        scope: Text::new(scope).unwrap(),
    }
}

fn assumed(response: &Value) -> Hash<Event> {
    assert_eq!(items(response)[2], Value::Unsigned(1), "{response:?}");
    let fields = items(&items(response)[3]);
    assert_eq!(fields[0], Value::Unsigned(1));
    logos_canon::CanonDecode::from_value(&fields[1]).unwrap()
}

fn open_of(
    ledger: &mut Ledger<'_, logos_crypto::FileSigner>,
    h: &CertificationHandlers,
    claim: Hash<Event>,
) -> Vec<Value> {
    let reply = exchange(
        ledger,
        h,
        Message::Query(logos_kernel::wire::Query::Open(claim)),
    );
    let value = items(&reply)[3].clone();
    let fields = items(&value);
    assert_eq!(fields[0], Value::Unsigned(1));
    items(&fields[1]).to_vec()
}

#[test]
fn assumption_derived_claim_is_conditional_in_query_and_falls_when_retracted() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let handlers = setup(&mut ledger, &signer, None, budget());
    let (v, p) = p_vocabulary();
    let message = assume_message(&ledger, &v, p.clone(), "session:estudo");
    let assumption = assumed(&exchange(&mut ledger, &handlers, message));
    let candidate_p = candidate(&ledger, &v, p.clone(), vec![assumption]);
    let o = proposed(&mut ledger, &handlers, candidate_p);
    let certificate = proof(&ledger, o, &v);
    let verified = success(
        &exchange(&mut ledger, &handlers, certify(o, certificate, LRAT)),
        0,
    );
    let open = open_of(&mut ledger, &handlers, verified);
    assert_eq!(
        open,
        vec![Value::Array(
            logos_canon::Array::new(vec![
                logos_canon::Canon::to_value(&assumption).unwrap(),
                Value::Text(Text::new("session:estudo").unwrap()),
            ])
            .unwrap()
        )],
        "claim derivado aparece como condicional"
    );
    // Claim sem assumptions é incondicional.
    let truth_candidate = candidate(&ledger, &v, truth(true), vec![]);
    let plain = proposed(&mut ledger, &handlers, truth_candidate);
    let plain_proof = proof(&ledger, plain, &v);
    let plain_verified = success(
        &exchange(&mut ledger, &handlers, certify(plain, plain_proof, LRAT)),
        0,
    );
    assert!(open_of(&mut ledger, &handlers, plain_verified).is_empty());
    // Assumption do agente: autor é a chave do kernel; retratar derruba o dependente.
    logos_kernel::retract::Retractions::new(signer.public_key().clone())
        .retract(
            &mut ledger,
            &signer,
            assumption,
            Text::new("hipótese descartada").unwrap(),
        )
        .unwrap();
    let index = ledger.live_index().unwrap();
    assert_eq!(
        index.status(verified).unwrap().unwrap().kind,
        logos_obligation::EntryKind::Invalidated
    );
    assert_eq!(
        index.status(plain_verified).unwrap().unwrap().kind,
        logos_obligation::EntryKind::Verified
    );
}

#[test]
fn assume_scopes_are_normative_and_proposals_derive_a_single_scope() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let handlers = setup(&mut ledger, &signer, None, budget());
    let (v, p) = p_vocabulary();
    for bad in ["", "local", "session:", "until:01", "claim:00"] {
        let message = assume_message(&ledger, &v, p.clone(), bad);
        refused(&exchange(&mut ledger, &handlers, message));
    }
    let open = Formula::new(
        vec![FormulaNode::Eq(
            logos_air::Term::new(
                vec![logos_air::TermNode::Var {
                    index: 0,
                    sort: logos_air::Sort {
                        name: Text::new("S").unwrap(),
                    },
                }],
                Limits::default(),
            )
            .unwrap(),
            logos_air::Term::new(
                vec![logos_air::TermNode::Var {
                    index: 0,
                    sort: logos_air::Sort {
                        name: Text::new("S").unwrap(),
                    },
                }],
                Limits::default(),
            )
            .unwrap(),
        )],
        Limits::default(),
    )
    .unwrap();
    let message = assume_message(&ledger, &v, open, "session:x");
    refused(&exchange(&mut ledger, &handlers, message));

    let one = assume_message(&ledger, &v, p.clone(), "session:um");
    let a = assumed(&exchange(&mut ledger, &handlers, one));
    let two = assume_message(&ledger, &v, p.clone(), "session:dois");
    let b = assumed(&exchange(&mut ledger, &handlers, two));
    let mixed = candidate(&ledger, &v, p.clone(), vec![a, b]);
    refused(&exchange(&mut ledger, &handlers, Message::Propose(mixed)));

    // claim:<h> só é citável junto com o claim h.
    let base_candidate = candidate(&ledger, &v, truth(true), vec![]);
    let base = proposed(&mut ledger, &handlers, base_candidate);
    let base_proof = proof(&ledger, base, &v);
    let anchor = success(
        &exchange(&mut ledger, &handlers, certify(base, base_proof, LRAT)),
        0,
    );
    let hex: String = anchor
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let local = assume_message(&ledger, &v, p.clone(), &format!("claim:{hex}"));
    let bound = assumed(&exchange(&mut ledger, &handlers, local));
    let alone = candidate(&ledger, &v, p.clone(), vec![bound]);
    refused(&exchange(&mut ledger, &handlers, Message::Propose(alone)));
    let together = candidate(&ledger, &v, p.clone(), vec![bound, anchor]);
    let o = proposed(&mut ledger, &handlers, together);
    // A regeneração do CERTIFY deriva o mesmo escopo e aceita a prova.
    let certificate = proof(&ledger, o, &v);
    success(
        &exchange(&mut ledger, &handlers, certify(o, certificate, LRAT)),
        0,
    );

    // until:<e> já vencido é recusado na declaração.
    let head = ledger.head().unwrap().unwrap();
    assert_eq!(head.epoch, 0);
    ledger
        .append(EventBody::EpochAdvanced {
            new_epoch: 3,
            reason: Text::new("fixture").unwrap(),
            policy_root: None,
            vocab: None,
        })
        .unwrap();
    let late = assume_message(&ledger, &v, p.clone(), "until:2");
    refused(&exchange(&mut ledger, &handlers, late));
    let fresh = assume_message(&ledger, &v, p, "until:3");
    assumed(&exchange(&mut ledger, &handlers, fresh));
}

// --- 5.10 (ADR0056): revogação do checker Rust e recheck com cake_lpr ---------

fn cake_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/cake-lpr/cake_lpr"
    ))
}

fn pq_vocabulary() -> (Vocabulary, Vec<Formula>) {
    use logos_air::{Grounding, Symbol, SymbolDecl, SymbolKind, SymbolResult};
    let symbols: Vec<_> = ["P", "Q"]
        .into_iter()
        .map(|n| Symbol {
            name: Text::new(n).unwrap(),
            kind: SymbolKind::Pred,
            arity: vec![],
            result: SymbolResult::Bool,
        })
        .collect();
    let v = Vocabulary::new(
        1,
        vec![],
        symbols
            .iter()
            .map(|s| SymbolDecl {
                signature: s.clone(),
                grounding: Grounding::Glossed(Text::new("fixture").unwrap()),
                gloss_pt: Text::new("fixture").unwrap(),
            })
            .collect(),
    )
    .unwrap();
    let formulas = symbols
        .iter()
        .map(|s| {
            Formula::new(
                vec![FormulaNode::Pred {
                    symbol: s.clone(),
                    args: vec![],
                }],
                Limits::default(),
            )
            .unwrap()
        })
        .collect();
    (v, formulas)
}

struct Revocation {
    temp: Temp,
    signer: logos_crypto::FileSigner,
    handlers: CertificationHandlers,
    good: Vec<Hash<Event>>,
    planted: Hash<Event>,
    on_planted: Hash<Event>,
    assumption: Hash<Event>,
}

/// a: P (assumption) ← c1: P ← c2: P (reais, Rust LRAT); planted: Q sobre a, com
/// certificado inválido "aceito" pelo checker Rust (bug simulado) ← d: Q (real).
fn revocation_fixture() -> Revocation {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let cake = cake_path();
    let handlers = setup(&mut ledger, &signer, Some(&cake), budget());
    let (v, formulas) = pq_vocabulary();
    let hv = ledger
        .cas()
        .put::<Vocabulary>(&to_canon(&v).unwrap())
        .unwrap();
    let assumption = hash(
        &ledger
            .append(EventBody::AssumptionDeclared {
                vocabulary: hv,
                formula: formulas[0].clone(),
                scope: Text::new("session:5.10").unwrap(),
                author: signer.key_id().unwrap(),
            })
            .unwrap(),
    )
    .unwrap();
    let prove = |ledger: &mut Ledger<'_, logos_crypto::FileSigner>, goal: usize, cite| {
        let p = candidate(ledger, &v, formulas[goal].clone(), vec![cite]);
        let o = proposed(ledger, &handlers, p);
        let c = proof(ledger, o, &v);
        success(&exchange(ledger, &handlers, certify(o, c, LRAT)), 0)
    };
    let c1 = prove(&mut ledger, 0, assumption);
    let c2 = prove(&mut ledger, 0, c1);
    // Falso aceite: Q não segue de P, mas um Verified do checker Rust é gravado.
    let p = candidate(&ledger, &v, formulas[1].clone(), vec![assumption]);
    let o = proposed(&mut ledger, &handlers, p);
    let bogus = ledger.cas().put::<Certificate>(b"1 0 0\n").unwrap();
    let builtin = hash_raw::<Checker>(&std::fs::read("/proc/self/exe").unwrap());
    let planted = hash(
        &ledger
            .append(EventBody::Verified {
                check: CheckRecord {
                    obligation: o,
                    certificate: bogus,
                    checker: builtin,
                    budget: budget(),
                    outcome: CheckOutcome::Accepted,
                },
                cited: vec![assumption],
                used: vec![assumption],
                consistency: logos_ledger::Consistency::Unknown { check: None },
            })
            .unwrap(),
    )
    .unwrap();
    let on_planted = prove(&mut ledger, 1, planted);
    drop(ledger);
    Revocation {
        temp,
        signer,
        handlers,
        good: vec![c1, c2],
        planted,
        on_planted,
        assumption,
    }
}

fn kind_of(ledger: &Ledger<'_, logos_crypto::FileSigner>, id: Hash<Event>) -> (EntryKind, bool) {
    let index = ledger.live_index().unwrap();
    (
        index.status(id).unwrap().unwrap().kind,
        index.is_citable(id, 0).unwrap(),
    )
}

#[test]
fn criterion_revoke_rust_recheck_with_cake_keeps_good_claims_and_invalidates_planted() {
    use logos_kernel::recheck::Rechecks;
    let f = revocation_fixture();
    let mut ledger = Ledger::open(&f.temp.0.join("ledger"), &f.signer).unwrap();
    let builtin = hash_raw::<Checker>(&std::fs::read("/proc/self/exe").unwrap());
    let rechecks = Rechecks::new(f.signer.public_key().clone());
    rechecks
        .revoke(
            &mut ledger,
            &f.signer,
            builtin,
            Text::new("bug no checker Rust").unwrap(),
        )
        .unwrap();
    for id in [f.good[0], f.good[1], f.planted, f.on_planted] {
        assert_eq!(kind_of(&ledger, id), (EntryKind::Suspended, false));
    }
    assert_eq!(
        kind_of(&ledger, f.assumption),
        (EntryKind::Assumption, true)
    );
    // Revogado não certifica mais (nem no modo cake, que depende do Rust para used).
    let (v, formulas) = pq_vocabulary();
    let p = candidate(&ledger, &v, formulas[0].clone(), vec![f.assumption]);
    let o = proposed(&mut ledger, &f.handlers, p);
    let c = proof(&ledger, o, &v);
    for format in [LRAT, CAKE] {
        refused(&exchange(&mut ledger, &f.handlers, certify(o, c, format)));
    }

    let registry = CheckerRegistry::new(f.signer.public_key().clone(), Some(&cake_path())).unwrap();
    let report = rechecks
        .recheck(&mut ledger, &registry, builtin, CAKE, budget())
        .unwrap();
    assert_eq!(
        (report.accepted, report.rejected, report.unknown),
        (2, 1, 0)
    );
    assert_eq!(report.rechecked.len(), 3, "d já caiu antes da sua vez");
    for id in &f.good {
        assert_eq!(
            kind_of(&ledger, *id),
            (EntryKind::Verified, true),
            "nenhum claim bom perdido"
        );
    }
    for id in [f.planted, f.on_planted] {
        assert_eq!(kind_of(&ledger, id), (EntryKind::Invalidated, false));
    }
    // Cascata registrada: d cai com causa [Rechecked rejeitado, planted].
    assert_eq!(report.invalidated.len(), 1);
    let record = ledger
        .events_from(ledger.head().unwrap().unwrap().seq, 1)
        .unwrap()
        .pop()
        .unwrap();
    let EventBody::Invalidated { claim, causes } = record.body else {
        panic!("Invalidated")
    };
    assert_eq!(claim, f.on_planted);
    assert_eq!(causes[1], f.planted);
    // Idempotente: nada a rechecar nem a registrar de novo.
    let again = rechecks
        .recheck(&mut ledger, &registry, builtin, CAKE, budget())
        .unwrap();
    assert!(again.rechecked.is_empty() && again.invalidated.is_empty());
    // Reabertura do índice do zero reproduz o mesmo estado.
    drop(ledger);
    let rebuilt =
        logos_ledger::LedgerIndex::open(&f.temp.0.join("ledger"), f.signer.public_key()).unwrap();
    for id in &f.good {
        assert_eq!(
            rebuilt.status(*id).unwrap().unwrap().kind,
            EntryKind::Verified
        );
    }
    assert_eq!(
        rebuilt.status(f.planted).unwrap().unwrap().kind,
        EntryKind::Invalidated
    );
}

#[test]
fn revocation_and_recheck_refuse_unauthorized_unknown_and_degenerate_requests() {
    use logos_kernel::{TransportError, recheck::Rechecks};
    let f = revocation_fixture();
    let mut ledger = Ledger::open(&f.temp.0.join("ledger"), &f.signer).unwrap();
    let builtin = hash_raw::<Checker>(&std::fs::read("/proc/self/exe").unwrap());
    let rechecks = Rechecks::new(f.signer.public_key().clone());
    let reason = || Text::new("teste").unwrap();
    let other = logos_crypto::FileSigner::from_seed(&[0x33; 32]).unwrap();
    assert!(matches!(
        rechecks.revoke(&mut ledger, &other, builtin, reason()),
        Err(TransportError::Unauthorized)
    ));
    let unknown = hash_raw::<Checker>(b"never registered");
    assert!(
        rechecks
            .revoke(&mut ledger, &f.signer, unknown, reason())
            .is_err()
    );
    let registry = CheckerRegistry::new(f.signer.public_key().clone(), Some(&cake_path())).unwrap();
    // Mesmo checker de origem e destino: recusado sem gravar nada.
    let head = ledger.head().unwrap();
    assert!(
        rechecks
            .recheck(&mut ledger, &registry, builtin, LRAT, budget())
            .is_err()
    );
    assert!(
        rechecks
            .recheck(&mut ledger, &registry, builtin, MODEL, budget())
            .is_err()
    );
    assert_eq!(ledger.head().unwrap(), head);
    rechecks
        .revoke(&mut ledger, &f.signer, builtin, reason())
        .unwrap();
    assert!(
        rechecks
            .revoke(&mut ledger, &f.signer, builtin, reason())
            .is_err()
    );
    // Checker de destino revogado não serve para recheck.
    let cake = registry.cake_identity().unwrap();
    rechecks
        .revoke(&mut ledger, &f.signer, cake, reason())
        .unwrap();
    let head = ledger.head().unwrap();
    assert!(
        rechecks
            .recheck(&mut ledger, &registry, builtin, CAKE, budget())
            .is_err()
    );
    assert_eq!(ledger.head().unwrap(), head);
    for id in [f.good[0], f.planted] {
        assert_eq!(kind_of(&ledger, id), (EntryKind::Suspended, false));
    }
}

#[test]
fn independent_auditor_accepts_planted_route_only_after_rejected_recheck() {
    use logos_kernel::recheck::Rechecks;
    let cake = logos_lrat::CakeRegistry::register(&cake_path()).unwrap();
    let builtin = hash_raw::<Checker>(&std::fs::read("/proc/self/exe").unwrap());
    let f = revocation_fixture();
    let path = f.temp.0.join("ledger");
    let audit = || {
        logos_audit::verify_with_checkers(
            &path.join("ledger.sqlite3"),
            &path.join("cas"),
            f.signer.public_key(),
            Some(&cake),
            &[*builtin.as_bytes()],
        )
    };
    // Antes do recheck o falso aceite é falha de auditoria.
    assert!(audit().is_err());
    let mut ledger = Ledger::open(&path, &f.signer).unwrap();
    let rechecks = Rechecks::new(f.signer.public_key().clone());
    rechecks
        .revoke(&mut ledger, &f.signer, builtin, Text::new("bug").unwrap())
        .unwrap();
    let registry = CheckerRegistry::new(f.signer.public_key().clone(), Some(&cake_path())).unwrap();
    rechecks
        .recheck(&mut ledger, &registry, builtin, CAKE, budget())
        .unwrap();
    drop(ledger);
    let report = audit().unwrap();
    assert_eq!(report.invalidated_by_recheck, 1);
    assert_eq!(report.unchecked_proofs, 0);
    assert_eq!(
        report.lrat_proofs, 3,
        "c1, c2 e d rechecados pelos dois checkers"
    );
}
