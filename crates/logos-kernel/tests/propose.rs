//! P1 e STATE14 pelo socket real; fixtures de replay não autorizam CERTIFY.
#![cfg(target_os = "linux")]
mod support;

use logos_air::{
    Formula, FormulaNode, Grounding, Limits, Sort, Symbol, SymbolDecl, SymbolKind, SymbolResult,
    Vocabulary,
};
use logos_canon::{Array, Canon, Text, Value, from_canon, to_canon};
use logos_core::{Budget, CheckOutcome, Exhausted};
use logos_crypto::{
    CryptoError, FileSigner, Hash, PublicKey, SigDomain, Signature, Signer, hash, hash_raw,
};
use logos_kernel::{
    propose::ProposalHandlers,
    server::Server,
    staging::Staging,
    wire::{Message, Query, Request, encode_frame},
};
use logos_ledger::{
    Authority, Certificate, CheckRecord, Checker, Consistency, Event, EventBody, EventProposal,
    Ledger, ModelObject, Route, TaintedText,
};
use logos_obligation::{
    DecodeLimits, EntryKind, Obligation, PremiseRef, Proposal, decode_for_check, generate,
};
use std::{
    cell::Cell,
    io::{Read, Write},
    net::Shutdown,
    os::unix::net::UnixStream,
    sync::Mutex,
    time::Duration,
};
use support::*;

fn arr(values: Vec<Value>) -> Value {
    Value::Array(Array::new(values).unwrap())
}
fn items(value: &Value) -> &[Value] {
    let Value::Array(array) = value else {
        panic!("expected array: {value:?}")
    };
    array.items()
}
fn formula(nodes: Vec<FormulaNode>) -> Formula {
    Formula::new(nodes, Limits::default()).unwrap()
}
fn truth() -> Formula {
    formula(vec![FormulaNode::True])
}
fn symbol(name: &str) -> Symbol {
    Symbol {
        name: Text::new(name).unwrap(),
        kind: SymbolKind::Pred,
        arity: vec![],
        result: SymbolResult::Bool,
    }
}
fn pred(name: &str) -> Formula {
    formula(vec![FormulaNode::Pred {
        symbol: symbol(name),
        args: vec![],
    }])
}
fn vocabulary(names: &[&str]) -> Vocabulary {
    Vocabulary::new(
        1,
        vec![],
        names
            .iter()
            .map(|name| SymbolDecl {
                signature: symbol(name),
                grounding: Grounding::Glossed(Text::new("fixture").unwrap()),
                gloss_pt: Text::new("fixture").unwrap(),
            })
            .collect(),
    )
    .unwrap()
}
fn epoch<S: Signer>(ledger: &mut Ledger<'_, S>, n: u64) {
    ledger
        .append(EventBody::EpochAdvanced {
            new_epoch: n,
            reason: Text::new("fixture epoch").unwrap(),
            policy_root: None,
            vocab: None,
        })
        .unwrap();
}
fn publish<S: Signer>(ledger: &Ledger<'_, S>, v: &Vocabulary) -> Hash<Vocabulary> {
    ledger.cas().put(&to_canon(v).unwrap()).unwrap()
}
fn assume<S: Signer>(
    ledger: &mut Ledger<'_, S>,
    v: Hash<Vocabulary>,
    f: Formula,
    scope: &str,
    signer: &S,
) -> Hash<Event> {
    hash(
        &ledger
            .append(EventBody::AssumptionDeclared {
                vocabulary: v,
                formula: f,
                scope: Text::new(scope).unwrap(),
                author: signer.key_id().unwrap(),
            })
            .unwrap(),
    )
    .unwrap()
}
fn candidate(v: Hash<Vocabulary>, goal: Formula, cites: Vec<Hash<Event>>) -> EventProposal {
    EventProposal {
        text_pt: TaintedText::new("C7: cafe\u{301}".into()).unwrap(),
        goal,
        cites,
        speculative_on: vec![],
        vocabulary: v,
    }
}
fn frame(p: EventProposal) -> Vec<u8> {
    encode_frame(&Request {
        id: 14,
        message: Message::Propose(p),
    })
    .unwrap()
}
fn raw_frame(value: Value) -> Vec<u8> {
    let bytes = to_canon(&value).unwrap();
    let mut out = (bytes.len() as u32).to_be_bytes().to_vec();
    out.extend(bytes);
    out
}
fn read_response(client: &mut UnixStream) -> Vec<u8> {
    client
        .set_read_timeout(Some(Duration::from_secs(15)))
        .unwrap();
    let mut length = [0; 4];
    client.read_exact(&mut length).unwrap();
    let length = u32::from_be_bytes(length) as usize;
    assert!(length <= logos_kernel::wire::MAX_RESPONSE_BYTES);
    let mut response = vec![0; length];
    client.read_exact(&mut response).unwrap();
    response
}
fn exchange<S: Signer>(ledger: &mut Ledger<'_, S>, request: &[u8]) -> Vec<u8> {
    let temp = Temp::new();
    let uid = rustix::process::geteuid().as_raw();
    let server = Server::bind(&temp.0.join("socket"), uid, Default::default()).unwrap();
    let staging = Staging::open(&temp.dir("source"), &temp.dir("private"), uid, 100).unwrap();
    let (mut client, stream) = UnixStream::pair().unwrap();
    let writer = Mutex::new(ledger);
    std::thread::scope(|scope| {
        let worker = scope.spawn(move || {
            client.write_all(request).unwrap();
            client.shutdown(Shutdown::Write).unwrap();
            read_response(&mut client)
        });
        // Entradas inválidas são recusadas pelo decoder ou pelo handler.
        let _result = server.serve_stream(stream, &writer, &ProposalHandlers, &staging);
        worker.join().unwrap()
    })
}
fn reply<S: Signer>(ledger: &mut Ledger<'_, S>, message: Message) -> Value {
    from_canon(&exchange(
        ledger,
        &encode_frame(&Request { id: 14, message }).unwrap(),
    ))
    .unwrap()
}
fn result(reply: &Value) -> &[Value] {
    let envelope = items(reply);
    assert_eq!(
        &envelope[..3],
        &[
            1u64.to_value().unwrap(),
            Value::Unsigned(14),
            Value::Unsigned(1)
        ]
    );
    items(&envelope[3])
}
fn obligation(reply: &Value) -> (Hash<Obligation>, Vec<u8>) {
    let [Value::Unsigned(1), Value::Bytes(hash), Value::Bytes(bytes)] = result(reply) else {
        panic!("propose-result/v1")
    };
    (
        Hash::from_bytes(hash.as_slice().try_into().unwrap()),
        bytes.clone(),
    )
}
fn rejected<S: Signer>(ledger: &mut Ledger<'_, S>, frame: &[u8], code: u64) {
    let before = ledger.head().unwrap();
    let reply: Value = from_canon(&exchange(ledger, frame)).unwrap();
    assert_eq!(
        &items(&reply)[2..],
        &[Value::Unsigned(0), Value::Unsigned(code)]
    );
    assert_eq!(ledger.head().unwrap(), before, "rejection cannot append");
}
fn stored_obligation<S: Signer>(
    ledger: &Ledger<'_, S>,
    v: &Vocabulary,
    goal: &Formula,
    cites: &[Hash<Event>],
) -> Hash<Obligation> {
    let refs: Vec<_> = cites
        .iter()
        .map(|h| PremiseRef::from_event_hash(*h.as_bytes()))
        .collect();
    let index = ledger.index().unwrap();
    let view = index.snapshot(ledger.cas(), &refs, "").unwrap();
    let o = generate(
        Proposal {
            vocabulary: v,
            goal,
            cites: &refs,
        },
        &view,
    )
    .unwrap();
    ledger.cas().put(&to_canon(&o).unwrap()).unwrap()
}
fn check<S: Signer>(ledger: &Ledger<'_, S>, ho: Hash<Obligation>) -> CheckRecord {
    CheckRecord {
        obligation: ho,
        certificate: ledger
            .cas()
            .put::<Certificate>(b"fixture certificate")
            .unwrap(),
        checker: ledger.cas().put::<Checker>(b"fixture checker").unwrap(),
        budget: Budget::new(100, 10000, 10000, 10),
        outcome: CheckOutcome::Accepted,
    }
}
fn retract(ledger: &mut Ledger<'_, FileSigner>, signer: &FileSigner, target: Hash<Event>) {
    let mut body = EventBody::Retracted {
        target,
        reason: Text::new("fixture withdrawal").unwrap(),
        authority: Authority {
            key: signer.key_id().unwrap(),
            sig: signer.sign::<Event>(b"placeholder").unwrap(),
        },
    };
    let bytes = body.authority_message().unwrap().unwrap();
    let EventBody::Retracted { authority, .. } = &mut body else {
        unreachable!()
    };
    authority.sig = signer.sign::<Event>(&bytes).unwrap();
    body.verify_authority(signer.public_key()).unwrap();
    ledger.append(body).unwrap();
}

#[test]
fn state14_uses_epoch9_real_citations_and_two_durable_transitions() {
    let temp = Temp::new();
    let signer = signer();
    let path = temp.0.join("ledger");
    let mut ledger = Ledger::open(&path, &signer).unwrap();
    genesis(&mut ledger, &signer);
    epoch(&mut ledger, 9);
    let v = vocabulary(&["P", "Q", "R"]);
    let hv = publish(&ledger, &v);
    // F3/P2/C5 são rótulos de fixture: sem ingestão Fact/Policy das Fases6/7.
    let f3 = assume(&mut ledger, hv, pred("P"), "", &signer);
    let p2 = assume(&mut ledger, hv, pred("Q"), "", &signer);
    let c5 = assume(&mut ledger, hv, pred("R"), "", &signer);
    let goal = formula(vec![
        FormulaNode::And(3),
        pred("P").nodes()[0].clone(),
        pred("Q").nodes()[0].clone(),
        pred("R").nodes()[0].clone(),
    ]);
    let proposal = candidate(hv, goal.clone(), vec![c5, f3, p2, f3]);
    let before = ledger.head().unwrap().unwrap();
    let response = reply(&mut ledger, Message::Propose(proposal.clone()));
    let (ho, bytes) = obligation(&response);
    assert_eq!(hash_raw::<Obligation>(&bytes), ho);
    assert_eq!(ledger.cas().get(&ho).unwrap(), Some(bytes.clone()));
    let checked = decode_for_check(&bytes, ho, &v, DecodeLimits::default()).unwrap();
    let mut expected = [(f3, pred("P")), (p2, pred("Q")), (c5, pred("R"))];
    expected.sort_unstable_by_key(|(h, _)| *h);
    assert_eq!(checked.goal(), &goal);
    assert_eq!(checked.vocab(), hv);
    assert_eq!(
        checked
            .premises()
            .iter()
            .map(|(r, f)| (*r.event_hash(), f.clone()))
            .collect::<Vec<_>>(),
        expected
            .iter()
            .map(|(h, f)| (*h.as_bytes(), f.clone()))
            .collect::<Vec<_>>()
    );
    let events = ledger.events_from(before.seq + 1, 256).unwrap();
    assert_eq!(events.len(), 2);
    for event in &events {
        event.verify(signer.public_key()).unwrap();
        assert_eq!(event.epoch, 9);
    }
    assert_eq!(events[0].prev, Some(before.hash));
    assert_eq!(events[1].prev, Some(hash(&events[0]).unwrap()));
    assert!(
        matches!(&events[0].body, EventBody::Proposed { proposal: actual, author } if actual == &proposal && *author == signer.key_id().unwrap())
    );
    assert!(
        matches!(events[1].body, EventBody::ObligationCreated { obligation, route: Route::Model, artifact, cnf: None } if obligation == ho && artifact == *ho.as_bytes())
    );
    let pending = hash(&events[1]).unwrap();
    let status = reply_status(&mut ledger, pending);
    assert_eq!(
        status,
        [Value::Unsigned(4), Value::Unsigned(9), Value::Bool(false)]
    );
    assert!(
        result(&reply(
            &mut ledger,
            Message::Query(Query::Status(Hash::from_bytes(*ho.as_bytes())))
        ))
        .is_empty(),
        "h(O) is not an event reference"
    );
    let head = ledger.head().unwrap();
    drop(ledger);
    let ledger = Ledger::open(&path, &signer).unwrap();
    assert_eq!(ledger.head().unwrap(), head);
    assert_eq!(
        ledger
            .index()
            .unwrap()
            .status(pending)
            .unwrap()
            .unwrap()
            .kind,
        EntryKind::Pending
    );
}
fn reply_status<S: Signer>(ledger: &mut Ledger<'_, S>, event: Hash<Event>) -> Vec<Value> {
    result(&reply(ledger, Message::Query(Query::Status(event)))).to_vec()
}

#[test]
fn p1_fifty_distinct_attacks_are_refused_before_any_append() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    epoch(&mut ledger, 8);
    let v = vocabulary(&[]);
    let hv = publish(&ledger, &v);
    let old: Vec<_> = (0..10)
        .map(|_| assume(&mut ledger, hv, truth(), "", &signer))
        .collect();
    epoch(&mut ledger, 9);
    let good = assume(&mut ledger, hv, truth(), "", &signer);
    let withdrawn: Vec<_> = (0..10)
        .map(|_| assume(&mut ledger, hv, truth(), "", &signer))
        .collect();
    for reference in &withdrawn {
        retract(&mut ledger, &signer, *reference);
    }
    let ho = stored_obligation(&ledger, &v, &truth(), &[]);
    let bodies = [
        EventBody::Proposed {
            proposal: candidate(hv, truth(), vec![]),
            author: signer.key_id().unwrap(),
        },
        EventBody::ObligationCreated {
            obligation: ho,
            route: Route::Model,
            artifact: *ho.as_bytes(),
            cnf: None,
        },
        EventBody::Refuted {
            check: check(&ledger, ho),
            model: ledger.cas().put::<ModelObject>(b"fixture model").unwrap(),
        },
        EventBody::UnknownRecorded {
            check: CheckRecord {
                outcome: CheckOutcome::Unknown(Exhausted::Time),
                ..check(&ledger, ho)
            },
            machine: Text::new("fixture").unwrap(),
        },
    ];
    let not_verified: Vec<_> = bodies
        .into_iter()
        .map(|body| hash(&ledger.append(body).unwrap()).unwrap())
        .collect();
    let mut attacks = Vec::new();
    for i in 0..10 {
        let mut p = candidate(hv, truth(), vec![good, not_verified[i % 4]]);
        p.text_pt = TaintedText::new(format!("unverified {i}")).unwrap();
        attacks.push(("unverified", frame(p)));
    }
    let root: Value = from_canon(
        &Request {
            id: 14,
            message: Message::Propose(candidate(hv, truth(), vec![])),
        }
        .encode()
        .unwrap(),
    )
    .unwrap();
    let obligation_bytes = ledger.cas().get(&ho).unwrap().unwrap();
    for i in 0..10 {
        let mut outer = items(&root).to_vec();
        let mut fields = items(&outer[3]).to_vec();
        match i {
            0 => fields.push(Value::Bytes(obligation_bytes.clone())),
            1 => fields[1] = Value::Bytes(obligation_bytes.clone()),
            2 => fields = items(&from_canon::<Value>(&obligation_bytes).unwrap()).to_vec(),
            3 => outer[2] = Value::Unsigned(7),
            4 => outer.push(Value::Bytes(obligation_bytes.clone())),
            5 => {
                fields = vec![
                    ho.to_value().unwrap(),
                    Value::Bytes(obligation_bytes.clone()),
                ]
            }
            6 => fields[2] = arr(vec![Value::Bytes(obligation_bytes.clone())]),
            7 => fields[1] = Text::new("obligation/v1").unwrap().to_value().unwrap(),
            8 => fields[1] = arr(vec![Value::Bytes(obligation_bytes.clone())]),
            _ => outer[0] = Value::Unsigned(0),
        }
        outer[3] = arr(fields);
        attacks.push(("ready obligation", raw_frame(arr(outer))));
    }
    for i in 0..10 {
        let mut outer = items(&root).to_vec();
        let mut fields = items(&outer[3]).to_vec();
        let tag = if i % 2 == 0 { "Fact" } else { "Policy" };
        fields.insert(
            i % 6,
            arr(vec![
                Text::new(tag).unwrap().to_value().unwrap(),
                Value::Unsigned(i as u64),
                truth().to_value().unwrap(),
            ]),
        );
        outer[3] = arr(fields);
        attacks.push(("fact or policy", raw_frame(arr(outer))));
    }
    for reference in old {
        attacks.push((
            "old epoch",
            frame(candidate(hv, truth(), vec![reference, good])),
        ));
    }
    for reference in withdrawn {
        attacks.push((
            "withdrawn",
            frame(candidate(hv, truth(), vec![reference, good])),
        ));
    }
    assert_eq!(attacks.len(), 50);
    let unique: std::collections::BTreeSet<_> = attacks.iter().map(|(_, frame)| frame).collect();
    assert_eq!(unique.len(), 50, "attacks must be distinct");
    for (kind, bytes) in attacks {
        eprintln!("P1: {kind}");
        rejected(&mut ledger, &bytes, 0);
    }
}

#[test]
fn empty_citations_duplicates_and_verified_replay_preserve_canonical_obligation() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let v = vocabulary(&[]);
    let hv = publish(&ledger, &v);
    let (empty, _) = obligation(&reply(
        &mut ledger,
        Message::Propose(candidate(hv, truth(), vec![])),
    ));
    let a = assume(&mut ledger, hv, truth(), "", &signer);
    let b = assume(&mut ledger, hv, truth(), "", &signer);
    let (first, _) = obligation(&reply(
        &mut ledger,
        Message::Propose(candidate(hv, truth(), vec![b, a, b])),
    ));
    let (second, _) = obligation(&reply(
        &mut ledger,
        Message::Propose(candidate(hv, truth(), vec![a, b])),
    ));
    assert_eq!(first, second);
    assert_ne!(first, empty);
    // É somente fixture de histórico autenticado; o handler não recebe Accepted.
    let verified = hash(
        &ledger
            .append(EventBody::Verified {
                check: check(&ledger, empty),
                cited: vec![],
                used: vec![],
                consistency: Consistency::Unknown { check: None },
            })
            .unwrap(),
    )
    .unwrap();
    let (ho, bytes) = obligation(&reply(
        &mut ledger,
        Message::Propose(candidate(hv, truth(), vec![verified])),
    ));
    let checked = decode_for_check(&bytes, ho, &v, DecodeLimits::default()).unwrap();
    assert_eq!(
        checked.premises(),
        &[(PremiseRef::from_event_hash(*verified.as_bytes()), truth())]
    );
}

#[test]
fn missing_incompatible_ill_sorted_and_speculative_inputs_never_append() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let v = vocabulary(&[]);
    let hv = publish(&ledger, &v);
    rejected(
        &mut ledger,
        &frame(candidate(Hash::from_bytes([0x88; 32]), truth(), vec![])),
        0,
    );
    rejected(
        &mut ledger,
        &frame(candidate(hv, truth(), vec![Hash::from_bytes([0x77; 32])])),
        0,
    );
    let malformed = ledger.cas().put::<Vocabulary>(&[0xf6]).unwrap();
    rejected(
        &mut ledger,
        &frame(candidate(malformed, truth(), vec![])),
        0,
    );
    let ill_sorted = formula(vec![
        FormulaNode::Forall(Sort {
            name: Text::new("missing").unwrap(),
        }),
        FormulaNode::True,
    ]);
    rejected(
        &mut ledger,
        &frame(candidate(hv, ill_sorted.clone(), vec![])),
        0,
    );
    let bad = assume(&mut ledger, hv, ill_sorted, "", &signer);
    rejected(&mut ledger, &frame(candidate(hv, truth(), vec![bad])), 4);
    let other = vocabulary(&["P"]);
    let other_hash = publish(&ledger, &other);
    let incompatible = assume(&mut ledger, other_hash, pred("P"), "", &signer);
    rejected(
        &mut ledger,
        &frame(candidate(hv, truth(), vec![incompatible])),
        0,
    );
    let mut p = candidate(hv, truth(), vec![]);
    p.speculative_on.push([1; 32]);
    rejected(&mut ledger, &frame(p), 0);
}

#[test]
fn scope_mismatch_in_direct_and_used_ancestor_is_not_silently_ignored() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let v = vocabulary(&[]);
    let hv = publish(&ledger, &v);
    let scoped = assume(&mut ledger, hv, truth(), "private", &signer);
    rejected(&mut ledger, &frame(candidate(hv, truth(), vec![scoped])), 4);
    let ho = stored_obligation(&ledger, &v, &truth(), &[]);
    let verified = hash(
        &ledger
            .append(EventBody::Verified {
                check: check(&ledger, ho),
                cited: vec![scoped],
                used: vec![scoped],
                consistency: Consistency::Unknown { check: None },
            })
            .unwrap(),
    )
    .unwrap();
    rejected(
        &mut ledger,
        &frame(candidate(hv, truth(), vec![verified])),
        4,
    );
}

fn expected_obligation_value(v: Hash<Vocabulary>, goal: &Formula) -> Value {
    arr(vec![
        Value::Unsigned(1),
        Value::Unsigned(0),
        Text::new("FOL-ms-eq/v1").unwrap().to_value().unwrap(),
        Value::Unsigned(1),
        v.to_value().unwrap(),
        arr(vec![]),
        goal.to_value().unwrap(),
    ])
}
#[test]
fn obligation_limit_is_inclusive_and_checked_before_publication_or_append() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let base = to_canon(&expected_obligation_value(
        Hash::from_bytes([0; 32]),
        &pred(""),
    ))
    .unwrap()
    .len();
    for delta in [0, 1] {
        let name = "p".repeat(DecodeLimits::DEFAULT_MAX_BYTES - base - 4 + delta);
        let v = vocabulary(&[&name]);
        let hv = publish(&ledger, &v);
        let goal = pred(&name);
        let expected = to_canon(&expected_obligation_value(hv, &goal)).unwrap();
        assert_eq!(expected.len(), DecodeLimits::DEFAULT_MAX_BYTES + delta);
        let expected_hash = hash_raw::<Obligation>(&expected);
        let p = candidate(hv, goal, vec![]);
        if delta == 0 {
            let (ho, bytes) = obligation(&reply(&mut ledger, Message::Propose(p)));
            assert_eq!(bytes, expected);
            assert_eq!(ho, expected_hash);
        } else {
            rejected(&mut ledger, &frame(p), 1);
            assert_eq!(ledger.cas().get(&expected_hash).unwrap(), None);
        }
    }
}

struct FailingSigner {
    inner: FileSigner,
    remaining: Cell<Option<usize>>,
    calls: Cell<usize>,
}
impl Signer for FailingSigner {
    fn public_key(&self) -> &PublicKey {
        self.inner.public_key()
    }
    fn sign<D: SigDomain>(&self, bytes: &[u8]) -> Result<Signature, CryptoError> {
        self.calls.set(self.calls.get() + 1);
        if let Some(remaining) = self.remaining.get() {
            if remaining == 0 {
                return Err(CryptoError::Io(std::io::ErrorKind::Other));
            }
            self.remaining.set(Some(remaining - 1));
        }
        self.inner.sign::<D>(bytes)
    }
}
#[test]
fn append_failures_return_storage_error_without_retry_or_fictitious_pending() {
    for successful_appends in [0, 1] {
        let temp = Temp::new();
        let signer = FailingSigner {
            inner: signer(),
            remaining: Cell::new(None),
            calls: Cell::new(0),
        };
        let path = temp.0.join("ledger");
        let mut ledger = Ledger::open(&path, &signer).unwrap();
        ledger
            .append(EventBody::Genesis {
                ledger_version: 1,
                kernel_binary: [1; 32],
                kernel_key: signer.key_id().unwrap(),
            })
            .unwrap();
        let hv = publish(&ledger, &vocabulary(&[]));
        let calls = signer.calls.get();
        let before = ledger.head().unwrap().unwrap();
        signer.remaining.set(Some(successful_appends));
        let response: Value = from_canon(&exchange(
            &mut ledger,
            &frame(candidate(hv, truth(), vec![])),
        ))
        .unwrap();
        assert_eq!(
            &items(&response)[2..],
            &[Value::Unsigned(0), Value::Unsigned(4)]
        );
        assert_eq!(signer.calls.get(), calls + successful_appends + 1);
        let after = ledger.head().unwrap().unwrap();
        assert_eq!(after.seq, before.seq + successful_appends as u64);
        if successful_appends == 1 {
            let index = ledger.index().unwrap();
            assert_eq!(
                index.status(after.hash).unwrap().unwrap().kind,
                EntryKind::Proposal
            );
            assert!(!index.is_citable(after.hash, 0).unwrap());
        }
        signer.remaining.set(None);
        drop(ledger);
        assert_eq!(
            Ledger::open(&path, &signer).unwrap().head().unwrap(),
            Some(after)
        );
    }
}

#[test]
fn independent_success_vectors_fix_the_new_result_version_hash_and_bytes() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../spec/vectors/kernel-protocol/propose-result-v1.json"
    ))
    .unwrap();
    for vector in vectors["vectors"].as_array().unwrap() {
        let unhex = |s: &str| -> Vec<u8> {
            s.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
                .collect()
        };
        let temp = Temp::new();
        let signer = signer();
        let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
        genesis(&mut ledger, &signer);
        let bytes = unhex(vector["vocabulary_hex"].as_str().unwrap());
        ledger.cas().put::<Vocabulary>(&bytes).unwrap();
        let actual = exchange(
            &mut ledger,
            &unhex(vector["request_frame_hex"].as_str().unwrap()),
        );
        assert_eq!(actual, unhex(vector["response_hex"].as_str().unwrap()));
    }
}
