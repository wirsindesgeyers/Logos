//! Etapa 6.1 (ADR0061): fatos assinados, raiz de confiança e janela de validade,
//! pelo ledger, índice, gerador e transporte reais.
#![cfg(target_os = "linux")]
mod support;

use logos_air::{
    Formula, FormulaNode, Grounding, Limits, Symbol, SymbolDecl, SymbolKind, SymbolResult,
    Vocabulary,
};
use logos_canon::{Text, Value, from_canon, to_canon};
use logos_core::{Budget, EpochRange};
use logos_crypto::{FileSigner, Hash, Signer};
use logos_kernel::{
    TransportError,
    certify::CertificationHandlers,
    facts::{TrustRoots, register},
    registry::{CheckerRegistry, LRAT},
    server::Server,
    staging::Staging,
    wire::{Message, ObjectRef, Request, encode_frame},
};
use logos_ledger::{
    Certificate, Event, EventBody, EventProposal, Fact, Ledger, LedgerIndex, TaintedText, Validity,
};
use logos_obligation::{DecodeLimits, EntryKind, Obligation};
use std::{
    io::{Read, Write},
    net::Shutdown,
    os::unix::net::UnixStream,
    sync::Mutex,
};
use support::*;

type L<'a> = Ledger<'a, FileSigner>;

fn items(value: &Value) -> &[Value] {
    let Value::Array(a) = value else {
        panic!("expected array {value:?}")
    };
    a.items()
}

fn exchange(ledger: &mut L<'_>, h: &CertificationHandlers, message: Message) -> Value {
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
            client
                .write_all(&encode_frame(&Request { id: 61, message }).unwrap())
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

fn accepted(response: &Value) -> &[Value] {
    assert_eq!(items(response)[2], Value::Unsigned(1), "{response:?}");
    items(&items(response)[3])
}

fn refused(response: &Value) {
    assert_eq!(items(response)[2], Value::Unsigned(0), "{response:?}");
}

struct World {
    v: Vocabulary,
    hv: Hash<Vocabulary>,
    chove: Formula,
    handlers: CertificationHandlers,
}

fn world(ledger: &mut L<'_>, kernel: &FileSigner) -> World {
    genesis(ledger, kernel);
    let registry = CheckerRegistry::new(kernel.public_key().clone(), None).unwrap();
    registry.install(ledger, kernel).unwrap();
    let symbol = Symbol {
        name: Text::new("chove").unwrap(),
        kind: SymbolKind::Pred,
        arity: vec![],
        result: SymbolResult::Bool,
    };
    let v = Vocabulary::new(
        1,
        vec![],
        vec![SymbolDecl {
            signature: symbol.clone(),
            grounding: Grounding::Glossed(Text::new("está chovendo").unwrap()),
            gloss_pt: Text::new("está chovendo").unwrap(),
        }],
    )
    .unwrap();
    let hv = ledger
        .cas()
        .put::<Vocabulary>(&to_canon(&v).unwrap())
        .unwrap();
    let chove = Formula::new(
        vec![FormulaNode::Pred {
            symbol,
            args: vec![],
        }],
        Limits::default(),
    )
    .unwrap();
    World {
        v,
        hv,
        chove,
        handlers: CertificationHandlers::new(
            registry,
            Budget::new(2_000_000, 32 * 1024 * 1024, 4 * 1024 * 1024, 3000),
        ),
    }
}

fn fact(ledger: &L<'_>, issuer: &FileSigner, w: &World, valid: Validity) -> Hash<Fact> {
    let fact = Fact::sign(
        issuer,
        w.chove.clone(),
        w.hv,
        Text::new("clima").unwrap(),
        1_700_000_000,
        valid,
        None,
    )
    .unwrap();
    ledger.cas().put::<Fact>(&to_canon(&fact).unwrap()).unwrap()
}

fn advance(ledger: &mut L<'_>, epoch: u64) {
    ledger
        .append(EventBody::EpochAdvanced {
            new_epoch: epoch,
            reason: Text::new("teste de janela").unwrap(),
            policy_root: None,
            vocab: None,
        })
        .unwrap();
}

fn propose(ledger: &mut L<'_>, w: &World, cites: Vec<Hash<Event>>) -> Value {
    exchange(
        ledger,
        &w.handlers,
        Message::Propose(EventProposal {
            text_pt: TaintedText::new("está chovendo".into()).unwrap(),
            goal: w.chove.clone(),
            cites,
            speculative_on: vec![],
            vocabulary: w.hv,
        }),
    )
}

/// Prova LRAT por propagação unitária sobre a CNF de O.
fn proof(ledger: &L<'_>, o: Hash<Obligation>, v: &Vocabulary) -> Hash<Certificate> {
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
                .any(|l| assigned.get(&l.abs()) == Some(&(*l > 0)))
            {
                continue;
            }
            let open: Vec<_> = clause
                .iter()
                .filter(|l| !assigned.contains_key(&l.abs()))
                .copied()
                .collect();
            if open.is_empty() {
                hints.push(i + 1);
                break 'propagate;
            }
            if open.len() == 1 {
                hints.push(i + 1);
                assigned.insert(open[0].abs(), open[0] > 0);
                progress = true;
            }
        }
        assert!(progress);
    }
    let hints: Vec<String> = hints.iter().map(ToString::to_string).collect();
    let text = format!("{} 0 {} 0\n", cnf.clauses().len() + 1, hints.join(" "));
    ledger.cas().put::<Certificate>(text.as_bytes()).unwrap()
}

fn issuer() -> FileSigner {
    FileSigner::from_seed(&[0x61; 32]).unwrap()
}

#[test]
fn unknown_issuer_bad_signature_and_duplicates_are_refused_without_events() {
    let temp = Temp::new();
    let kernel = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &kernel).unwrap();
    let w = world(&mut ledger, &kernel);
    let trusted = issuer();
    let roots = TrustRoots::new(&[trusted.public_key().clone()]).unwrap();
    let head = ledger.head().unwrap();
    // Emissor desconhecido: assinatura válida, mas fora da raiz.
    let stranger = FileSigner::from_seed(&[0x13; 32]).unwrap();
    let foreign = fact(&ledger, &stranger, &w, Validity::Until(u64::MAX));
    assert!(matches!(
        register(&mut ledger, &roots, foreign),
        Err(TransportError::Unauthorized)
    ));
    // Assinatura que não cobre o corpo publicado: fórmula trocada depois de assinar.
    let mut forged = Fact::sign(
        &trusted,
        w.chove.clone(),
        w.hv,
        Text::new("clima").unwrap(),
        1,
        Validity::Until(u64::MAX),
        None,
    )
    .unwrap();
    forged.topic = Text::new("outro").unwrap();
    let forged = ledger
        .cas()
        .put::<Fact>(&to_canon(&forged).unwrap())
        .unwrap();
    assert!(register(&mut ledger, &roots, forged).is_err());
    // Fato ausente do CAS e bytes que não são fact/v1.
    assert!(register(&mut ledger, &roots, Hash::from_bytes([3; 32])).is_err());
    let junk = ledger
        .cas()
        .put::<Fact>(&to_canon(&w.chove).unwrap())
        .unwrap();
    assert!(register(&mut ledger, &roots, junk).is_err());
    assert_eq!(ledger.head().unwrap(), head);
    // O mesmo fato só é registrado uma vez.
    let good = fact(&ledger, &trusted, &w, Validity::Until(u64::MAX));
    let event = register(&mut ledger, &roots, good).unwrap();
    assert_eq!(
        ledger.live_index().unwrap().fact_event(good).unwrap(),
        Some(event)
    );
    let head = ledger.head().unwrap();
    assert!(register(&mut ledger, &roots, good).is_err());
    assert_eq!(ledger.head().unwrap(), head);
    assert_eq!(
        ledger
            .live_index()
            .unwrap()
            .status(event)
            .unwrap()
            .unwrap()
            .kind,
        EntryKind::Fact
    );
}

#[test]
fn criterion_fact_valid_3_to_5_is_citable_in_4_and_refused_in_6() {
    let temp = Temp::new();
    let kernel = signer();
    let path = temp.0.join("ledger");
    let mut ledger = Ledger::open(&path, &kernel).unwrap();
    let w = world(&mut ledger, &kernel);
    let trusted = issuer();
    let roots = TrustRoots::new(&[trusted.public_key().clone()]).unwrap();
    advance(&mut ledger, 3);
    let f = fact(
        &ledger,
        &trusted,
        &w,
        Validity::Epochs(EpochRange::new(3, 5).unwrap()),
    );
    let registered = register(&mut ledger, &roots, f).unwrap();
    // Época 4: citável pelo gerador real; a prova LRAT usa o fato e o used o rastreia.
    advance(&mut ledger, 4);
    assert!(
        ledger
            .live_index()
            .unwrap()
            .is_citable(registered, 4)
            .unwrap()
    );
    let reply = propose(&mut ledger, &w, vec![registered]);
    let o: Hash<Obligation> = logos_canon::CanonDecode::from_value(&accepted(&reply)[1]).unwrap();
    let c = proof(&ledger, o, &w.v);
    let reply = exchange(
        &mut ledger,
        &w.handlers,
        Message::Certify {
            obligation: o,
            format: Text::new(LRAT).unwrap(),
            certificate: ObjectRef::Cas(c),
        },
    );
    let verified: Hash<Event> = logos_canon::CanonDecode::from_value(&accepted(&reply)[2]).unwrap();
    let head = ledger.head().unwrap().unwrap().seq;
    let last = ledger.events_from(head, 1).unwrap().pop().unwrap();
    let EventBody::Verified { used, .. } = last.body else {
        panic!("Verified")
    };
    assert_eq!(used, vec![registered]);
    assert_eq!(
        ledger.live_index().unwrap().dependents(registered).unwrap(),
        vec![verified]
    );
    // Época 6: a janela passou; o gerador real recusa a citação e nada é gravado.
    advance(&mut ledger, 6);
    assert!(
        !ledger
            .live_index()
            .unwrap()
            .is_citable(registered, 6)
            .unwrap()
    );
    let head = ledger.head().unwrap();
    refused(&propose(&mut ledger, &w, vec![registered]));
    assert_eq!(ledger.head().unwrap(), head);
    // A janela vem do fato no CAS, não de cache: índice reconstruído do zero igual.
    drop(ledger);
    let rebuilt = LedgerIndex::open(&path, kernel.public_key()).unwrap();
    assert_eq!(
        rebuilt.status(registered).unwrap().unwrap().kind,
        EntryKind::Fact
    );
    assert!(!rebuilt.is_citable(registered, 6).unwrap());
}

#[test]
fn criterion_expired_fact_is_not_citable() {
    let temp = Temp::new();
    let kernel = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &kernel).unwrap();
    let w = world(&mut ledger, &kernel);
    let trusted = issuer();
    let roots = TrustRoots::new(&[trusted.public_key().clone()]).unwrap();
    // Já vencido no registro: recusado.
    let past = fact(&ledger, &trusted, &w, Validity::Until(1));
    assert!(register(&mut ledger, &roots, past).is_err());
    let old = fact(
        &ledger,
        &trusted,
        &w,
        Validity::Epochs(EpochRange::new(0, 0).unwrap()),
    );
    let by_epoch = register(&mut ledger, &roots, old).unwrap();
    // Vence durante a sessão: citável antes, recusado (Expired) depois.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let soon = fact(&ledger, &trusted, &w, Validity::Until(now + 2));
    let by_time = register(&mut ledger, &roots, soon).unwrap();
    accepted(&propose(&mut ledger, &w, vec![by_time]));
    std::thread::sleep(std::time::Duration::from_secs(4));
    assert!(!ledger.live_index().unwrap().is_citable(by_time, 0).unwrap());
    let head = ledger.head().unwrap();
    refused(&propose(&mut ledger, &w, vec![by_time]));
    // Janela por época vencida: época 1 fora de [0,0].
    advance(&mut ledger, 1);
    let head_after = ledger.head().unwrap();
    assert_ne!(head, head_after);
    refused(&propose(&mut ledger, &w, vec![by_epoch]));
    assert_eq!(ledger.head().unwrap(), head_after);
    // Registrar um fato cuja janela de épocas já passou também é recusado.
    let stale = Fact::sign(
        &trusted,
        w.chove.clone(),
        w.hv,
        Text::new("clima").unwrap(),
        9,
        Validity::Epochs(EpochRange::new(0, 0).unwrap()),
        None,
    )
    .unwrap();
    let stale = ledger
        .cas()
        .put::<Fact>(&to_canon(&stale).unwrap())
        .unwrap();
    assert!(register(&mut ledger, &roots, stale).is_err());
    assert_eq!(ledger.head().unwrap(), head_after);
}

#[test]
fn independent_auditor_reverifies_fact_signatures() {
    let temp = Temp::new();
    let kernel = signer();
    let path = temp.0.join("ledger");
    let mut ledger = Ledger::open(&path, &kernel).unwrap();
    let w = world(&mut ledger, &kernel);
    let trusted = issuer();
    let roots = TrustRoots::new(&[trusted.public_key().clone()]).unwrap();
    let f = fact(&ledger, &trusted, &w, Validity::Until(u64::MAX));
    register(&mut ledger, &roots, f).unwrap();
    drop(ledger);
    let report = logos_audit::verify(&path, &path.join("cas"), kernel.public_key()).unwrap();
    assert_eq!(report.facts, 1);
    // Um FactRegistered gravado sem passar pelo kernel, com assinatura que não cobre
    // o corpo: a projeção aceita o objeto, mas o auditor independente recusa.
    let mut ledger = Ledger::open(&path, &kernel).unwrap();
    let mut forged = Fact::sign(
        &trusted,
        w.chove.clone(),
        w.hv,
        Text::new("clima").unwrap(),
        2,
        Validity::Until(u64::MAX),
        None,
    )
    .unwrap();
    forged.observed_at = 3;
    let forged = ledger
        .cas()
        .put::<Fact>(&to_canon(&forged).unwrap())
        .unwrap();
    ledger
        .append(EventBody::FactRegistered { fact: forged })
        .unwrap();
    drop(ledger);
    let error = logos_audit::verify(&path, &path.join("cas"), kernel.public_key()).unwrap_err();
    assert!(error.to_string().contains("fact signature"), "{error}");
}

#[test]
fn projection_refuses_a_used_fact_outside_its_window() {
    let temp = Temp::new();
    let kernel = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &kernel).unwrap();
    let w = world(&mut ledger, &kernel);
    let trusted = issuer();
    let roots = TrustRoots::new(&[trusted.public_key().clone()]).unwrap();
    advance(&mut ledger, 3);
    let f = fact(
        &ledger,
        &trusted,
        &w,
        Validity::Epochs(EpochRange::new(3, 5).unwrap()),
    );
    let registered = register(&mut ledger, &roots, f).unwrap();
    advance(&mut ledger, 4);
    let reply = propose(&mut ledger, &w, vec![registered]);
    let o: Hash<Obligation> = logos_canon::CanonDecode::from_value(&accepted(&reply)[1]).unwrap();
    let c = proof(&ledger, o, &w.v);
    let reply = exchange(
        &mut ledger,
        &w.handlers,
        Message::Certify {
            obligation: o,
            format: Text::new(LRAT).unwrap(),
            certificate: ObjectRef::Cas(c),
        },
    );
    accepted(&reply);
    let head = ledger.head().unwrap().unwrap().seq;
    let verified = ledger.events_from(head, 1).unwrap().pop().unwrap().body;
    // A mesma rota gravada sem o kernel na época 6: a projeção recusa o used vencido.
    advance(&mut ledger, 6);
    ledger.append(verified).unwrap();
    assert!(ledger.live_index().is_err());
}
