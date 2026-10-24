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
    facts::{publish_trust, register, revoke_issuer},
    registry::{CheckerRegistry, LRAT},
    server::Server,
    staging::Staging,
    wire::{Message, ObjectRef, Request, encode_frame},
};
use logos_ledger::{
    Certificate, Event, EventBody, EventProposal, Fact, Ledger, LedgerIndex, TaintedText,
    TrustConfig, TrustRoot, Validity,
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

fn admin() -> FileSigner {
    FileSigner::from_seed(&[0xad; 32]).unwrap()
}

/// Publica (como admin) uma configuração em que `issuer` atesta os tópicos dados.
fn grant(ledger: &mut L<'_>, roots: &[(&FileSigner, &str, &[&str])], version: u64) -> Hash<Event> {
    let mut roots: Vec<_> = roots
        .iter()
        .map(|(signer, name, topics)| TrustRoot {
            issuer: signer.public_key().clone(),
            name: Text::new(*name).unwrap(),
            topics: topics.iter().map(|t| Text::new(*t).unwrap()).collect(),
            level: 1,
        })
        .collect();
    roots.sort_by_key(|r| *r.issuer.key_id().unwrap().as_bytes());
    let config = TrustConfig::new(version, roots).unwrap();
    let h = ledger
        .cas()
        .put::<TrustConfig>(&to_canon(&config).unwrap())
        .unwrap();
    let admin = admin();
    publish_trust(ledger, admin.public_key(), &admin, h).unwrap()
}

/// Configuração dos testes da 6.1: o emissor atesta clima e o inventário de cores.
fn trust(ledger: &mut L<'_>, issuer: &FileSigner, version: u64) -> Hash<Event> {
    grant(
        ledger,
        &[(issuer, "estacao", &["clima", "inventario/cores"])],
        version,
    )
}

#[test]
fn unknown_issuer_bad_signature_and_duplicates_are_refused_without_events() {
    let temp = Temp::new();
    let kernel = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &kernel).unwrap();
    let w = world(&mut ledger, &kernel);
    let trusted = issuer();
    trust(&mut ledger, &trusted, 1);
    let head = ledger.head().unwrap();
    // Emissor desconhecido: assinatura válida, mas fora da raiz.
    let stranger = FileSigner::from_seed(&[0x13; 32]).unwrap();
    let foreign = fact(&ledger, &stranger, &w, Validity::Until(u64::MAX));
    assert!(matches!(
        register(&mut ledger, foreign),
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
    assert!(register(&mut ledger, forged).is_err());
    // Fato ausente do CAS e bytes que não são fact/v1.
    assert!(register(&mut ledger, Hash::from_bytes([3; 32])).is_err());
    let junk = ledger
        .cas()
        .put::<Fact>(&to_canon(&w.chove).unwrap())
        .unwrap();
    assert!(register(&mut ledger, junk).is_err());
    assert_eq!(ledger.head().unwrap(), head);
    // O mesmo fato só é registrado uma vez.
    let good = fact(&ledger, &trusted, &w, Validity::Until(u64::MAX));
    let event = register(&mut ledger, good).unwrap();
    assert_eq!(
        ledger.live_index().unwrap().fact_event(good).unwrap(),
        Some(event)
    );
    let head = ledger.head().unwrap();
    assert!(register(&mut ledger, good).is_err());
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
    trust(&mut ledger, &trusted, 1);
    advance(&mut ledger, 3);
    let f = fact(
        &ledger,
        &trusted,
        &w,
        Validity::Epochs(EpochRange::new(3, 5).unwrap()),
    );
    let registered = register(&mut ledger, f).unwrap();
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
    trust(&mut ledger, &trusted, 1);
    // Já vencido no registro: recusado.
    let past = fact(&ledger, &trusted, &w, Validity::Until(1));
    assert!(register(&mut ledger, past).is_err());
    let old = fact(
        &ledger,
        &trusted,
        &w,
        Validity::Epochs(EpochRange::new(0, 0).unwrap()),
    );
    let by_epoch = register(&mut ledger, old).unwrap();
    // Vence durante a sessão: citável antes, recusado (Expired) depois.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let soon = fact(&ledger, &trusted, &w, Validity::Until(now + 2));
    let by_time = register(&mut ledger, soon).unwrap();
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
    assert!(register(&mut ledger, stale).is_err());
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
    trust(&mut ledger, &trusted, 1);
    let f = fact(&ledger, &trusted, &w, Validity::Until(u64::MAX));
    register(&mut ledger, f).unwrap();
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
    trust(&mut ledger, &trusted, 1);
    advance(&mut ledger, 3);
    let f = fact(
        &ledger,
        &trusted,
        &w,
        Validity::Epochs(EpochRange::new(3, 5).unwrap()),
    );
    let registered = register(&mut ledger, f).unwrap();
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

/// V com sort `Cor = {vermelho, azul}` cuja fonte é `Fact(r)` e o predicado `ok(Cor)`.
fn colors(source: logos_air::FactRef) -> Vocabulary {
    use logos_air::{DomainSource, Sort, SortDecl, SortDomain};
    let cor = Sort {
        name: Text::new("Cor").unwrap(),
    };
    let constant = |name: &str| Symbol {
        name: Text::new(name).unwrap(),
        kind: SymbolKind::Const,
        arity: vec![],
        result: SymbolResult::Sort(cor.clone()),
    };
    let decl = |signature: Symbol| SymbolDecl {
        signature,
        grounding: Grounding::Glossed(Text::new("cor").unwrap()),
        gloss_pt: Text::new("cor").unwrap(),
    };
    let constants = vec![constant("vermelho"), constant("azul")];
    let mut symbols: Vec<_> = constants.iter().cloned().map(decl).collect();
    symbols.push(decl(Symbol {
        name: Text::new("ok").unwrap(),
        kind: SymbolKind::Pred,
        arity: vec![cor.clone()],
        result: SymbolResult::Bool,
    }));
    Vocabulary::new(
        1,
        vec![SortDecl {
            name: cor.name,
            domain: SortDomain::Enumerated {
                constants,
                domain_source: DomainSource::Fact(source),
            },
        }],
        symbols,
    )
    .unwrap()
}

fn enumeration_conjunction(v: &Vocabulary) -> Formula {
    logos_ground::domain_axioms(
        v,
        &logos_air::Sort {
            name: Text::new("Cor").unwrap(),
        },
    )
    .unwrap()
    .conjunction()
    .unwrap()
}

/// O ciclo V ↔ fato é evitado porque `FactRef` é o hash da fórmula, não do fato.
fn sourced_colors() -> (Vocabulary, Formula) {
    let conjunction = enumeration_conjunction(&colors(logos_air::FactRef([0; 32])));
    let source = logos_air::FactRef(*logos_crypto::hash(&conjunction).unwrap().as_bytes());
    let v = colors(source);
    assert_eq!(enumeration_conjunction(&v), conjunction);
    (v, conjunction)
}

fn ok_of(v: &Vocabulary, constant: Option<&str>) -> Formula {
    use logos_air::{Sort, Term, TermNode};
    let cor = Sort {
        name: Text::new("Cor").unwrap(),
    };
    let ok = v
        .symbols()
        .iter()
        .find(|d| d.signature.name.as_str() == "ok");
    let arg = match constant {
        Some(name) => TermNode::App {
            symbol: v
                .symbols()
                .iter()
                .find(|d| d.signature.name.as_str() == name)
                .unwrap()
                .signature
                .clone(),
            args: 0,
        },
        None => TermNode::Var {
            index: 0,
            sort: cor.clone(),
        },
    };
    let atom = FormulaNode::Pred {
        symbol: ok.unwrap().signature.clone(),
        args: vec![Term::new(vec![arg], Limits::default()).unwrap()],
    };
    let nodes = match constant {
        Some(_) => vec![atom],
        None => vec![FormulaNode::Forall(cor), atom],
    };
    Formula::new(nodes, Limits::default()).unwrap()
}

fn and(parts: &[Formula]) -> Formula {
    let mut nodes = vec![FormulaNode::And(parts.len())];
    for p in parts {
        nodes.extend_from_slice(p.nodes());
    }
    Formula::new(nodes, Limits::default()).unwrap()
}

fn register_formula(
    ledger: &mut L<'_>,
    issuer: &FileSigner,
    formula: Formula,
    hv: Hash<Vocabulary>,
) -> Hash<Event> {
    let fact = Fact::sign(
        issuer,
        formula,
        hv,
        Text::new("inventario/cores").unwrap(),
        1_700_000_000,
        Validity::Until(u64::MAX),
        None,
    )
    .unwrap();
    let h = ledger.cas().put::<Fact>(&to_canon(&fact).unwrap()).unwrap();
    register(ledger, h).unwrap()
}

fn propose_goal(
    ledger: &mut L<'_>,
    h: &CertificationHandlers,
    hv: Hash<Vocabulary>,
    goal: Formula,
    cites: Vec<Hash<Event>>,
) -> Value {
    exchange(
        ledger,
        h,
        Message::Propose(EventProposal {
            text_pt: TaintedText::new("toda cor está ok".into()).unwrap(),
            goal,
            cites,
            speculative_on: vec![],
            vocabulary: hv,
        }),
    )
}

#[test]
fn criterion_enumeration_source_fact_puts_axioms_in_o_and_is_traced_by_used() {
    let temp = Temp::new();
    let kernel = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &kernel).unwrap();
    let w = world(&mut ledger, &kernel);
    let trusted = issuer();
    trust(&mut ledger, &trusted, 1);
    let (v, conjunction) = sourced_colors();
    let hv = ledger
        .cas()
        .put::<Vocabulary>(&to_canon(&v).unwrap())
        .unwrap();
    let goal = ok_of(&v, None);
    let inventory = and(&[ok_of(&v, Some("vermelho")), ok_of(&v, Some("azul"))]);
    let both = register_formula(&mut ledger, &trusted, inventory, hv);

    // Fonte ausente: sem o fato do fecho, PROPOSE recusa e nada é gravado.
    let head = ledger.head().unwrap();
    refused(&propose_goal(
        &mut ledger,
        &w.handlers,
        hv,
        goal.clone(),
        vec![both],
    ));
    assert_eq!(ledger.head().unwrap(), head);

    // Fonte adulterada: fato registrado que omite azul do fecho não é a fonte.
    let closure_red = Formula::new(
        [
            vec![FormulaNode::Forall(logos_air::Sort {
                name: Text::new("Cor").unwrap(),
            })],
            vec![FormulaNode::Or(1)],
            conjunction.nodes()[4..5].to_vec(),
        ]
        .concat(),
        Limits::default(),
    )
    .unwrap();
    let forged = register_formula(&mut ledger, &trusted, closure_red, hv);
    let head = ledger.head().unwrap();
    refused(&propose_goal(
        &mut ledger,
        &w.handlers,
        hv,
        goal.clone(),
        vec![both, forged],
    ));
    // A conjunção exata como hipótese do agente não autentica a fonte.
    let reply = exchange(
        &mut ledger,
        &w.handlers,
        Message::Assume {
            vocabulary: hv,
            formula: conjunction.clone(),
            scope: Text::new("session:cores").unwrap(),
        },
    );
    let assumed: Hash<Event> = logos_canon::CanonDecode::from_value(&accepted(&reply)[1]).unwrap();
    let head_after_assume = ledger.head().unwrap();
    assert_ne!(head, head_after_assume);
    refused(&propose_goal(
        &mut ledger,
        &w.handlers,
        hv,
        goal.clone(),
        vec![both, assumed],
    ));
    assert_eq!(ledger.head().unwrap(), head_after_assume);
    // Fecho e nomes únicos separados como hipóteses: o grounder aceitaria, o kernel não.
    let axioms = logos_ground::domain_axioms(
        &v,
        &logos_air::Sort {
            name: Text::new("Cor").unwrap(),
        },
    )
    .unwrap();
    let mut cites = vec![both];
    for formula in std::iter::once(axioms.closure()).chain(axioms.distinct()) {
        let reply = exchange(
            &mut ledger,
            &w.handlers,
            Message::Assume {
                vocabulary: hv,
                formula: formula.clone(),
                scope: Text::new("session:cores").unwrap(),
            },
        );
        cites.push(logos_canon::CanonDecode::from_value(&accepted(&reply)[1]).unwrap());
    }
    let head = ledger.head().unwrap();
    refused(&propose_goal(
        &mut ledger,
        &w.handlers,
        hv,
        goal.clone(),
        cites,
    ));
    assert_eq!(ledger.head().unwrap(), head);

    // Fonte registrada: axiomas em O, prova LRAT real e used/deps com o fato da fonte.
    let source = register_formula(&mut ledger, &trusted, conjunction.clone(), hv);
    let reply = propose_goal(&mut ledger, &w.handlers, hv, goal, vec![both, source]);
    let o: Hash<Obligation> = logos_canon::CanonDecode::from_value(&accepted(&reply)[1]).unwrap();
    let bytes = ledger.cas().get(&o).unwrap().unwrap();
    let decoded =
        logos_obligation::decode_for_check(&bytes, o, &v, DecodeLimits::default()).unwrap();
    assert!(
        decoded
            .premises()
            .iter()
            .any(|(r, f)| f == &conjunction && r.event_hash() == source.as_bytes())
    );
    let c = proof(&ledger, o, &v);
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
    let EventBody::Verified { used, .. } = ledger.events_from(head, 1).unwrap().pop().unwrap().body
    else {
        panic!("Verified")
    };
    assert!(used.contains(&source), "{used:?}");
    assert_eq!(
        ledger.live_index().unwrap().dependents(source).unwrap(),
        vec![verified]
    );
}

#[test]
fn enumeration_source_must_match_the_vocabulary_and_policy_stays_refused() {
    let temp = Temp::new();
    let kernel = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &kernel).unwrap();
    let w = world(&mut ledger, &kernel);
    let trusted = issuer();
    trust(&mut ledger, &trusted, 1);
    // FactRef que não é o hash da conjunção de V: nenhum fato pode ser a fonte.
    let v = colors(logos_air::FactRef([7; 32]));
    let hv = ledger
        .cas()
        .put::<Vocabulary>(&to_canon(&v).unwrap())
        .unwrap();
    let source = register_formula(&mut ledger, &trusted, enumeration_conjunction(&v), hv);
    let both = register_formula(
        &mut ledger,
        &trusted,
        and(&[ok_of(&v, Some("vermelho")), ok_of(&v, Some("azul"))]),
        hv,
    );
    let head = ledger.head().unwrap();
    refused(&propose_goal(
        &mut ledger,
        &w.handlers,
        hv,
        ok_of(&v, None),
        vec![both, source],
    ));
    assert_eq!(ledger.head().unwrap(), head);
}

/// Snapshot mínimo para reproduzir o O que um kernel anterior à 6.1b gravaria.
struct Entries<'a>(&'a Vocabulary, Vec<(Hash<Event>, EntryKind, Formula)>);

impl logos_obligation::LedgerView for Entries<'_> {
    fn current_epoch(&self) -> u64 {
        0
    }
    fn lookup(
        &self,
        reference: logos_obligation::PremiseRef,
    ) -> Result<Option<logos_obligation::LedgerEntry<'_>>, logos_obligation::LedgerError> {
        Ok(self
            .1
            .iter()
            .find(|(h, _, _)| h.as_bytes() == reference.event_hash())
            .map(|(_, kind, formula)| logos_obligation::LedgerEntry {
                kind: *kind,
                epoch: 0,
                retracted: false,
                expired: false,
                untrusted: false,
                vocabulary: self.0,
                formula,
            }))
    }
}

#[test]
fn certify_rechecks_the_source_of_a_pending_written_without_it() {
    let temp = Temp::new();
    let kernel = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &kernel).unwrap();
    let w = world(&mut ledger, &kernel);
    let trusted = issuer();
    trust(&mut ledger, &trusted, 1);
    let (v, _) = sourced_colors();
    let hv = ledger
        .cas()
        .put::<Vocabulary>(&to_canon(&v).unwrap())
        .unwrap();
    let goal = ok_of(&v, None);
    let inventory = and(&[ok_of(&v, Some("vermelho")), ok_of(&v, Some("azul"))]);
    let both = register_formula(&mut ledger, &trusted, inventory.clone(), hv);
    let mut entries = vec![(both, EntryKind::Fact, inventory)];
    let axioms = logos_ground::domain_axioms(
        &v,
        &logos_air::Sort {
            name: Text::new("Cor").unwrap(),
        },
    )
    .unwrap();
    for formula in std::iter::once(axioms.closure()).chain(axioms.distinct()) {
        let reply = exchange(
            &mut ledger,
            &w.handlers,
            Message::Assume {
                vocabulary: hv,
                formula: formula.clone(),
                scope: Text::new("session:cores").unwrap(),
            },
        );
        let event = logos_canon::CanonDecode::from_value(&accepted(&reply)[1]).unwrap();
        entries.push((event, EntryKind::Assumption, formula.clone()));
    }
    let cites: Vec<_> = entries.iter().map(|(h, _, _)| *h).collect();
    let refs: Vec<_> = cites
        .iter()
        .map(|h| logos_obligation::PremiseRef::from_event_hash(*h.as_bytes()))
        .collect();
    let o = logos_obligation::generate(
        logos_obligation::Proposal {
            vocabulary: &v,
            goal: &goal,
            cites: &refs,
        },
        &Entries(&v, entries),
    )
    .unwrap();
    // O grounder aceita os axiomas separados; só a fonte barra este O.
    let (cnf, _) = logos_ground::clausify(&logos_ground::ground(&o, &v).unwrap()).unwrap();
    let ho = ledger
        .cas()
        .put::<Obligation>(&to_canon(&o).unwrap())
        .unwrap();
    let cnf_hash = ledger
        .cas()
        .put::<logos_ledger::CnfObject>(cnf.dimacs())
        .unwrap();
    let author = ledger.events_from(0, 1).unwrap()[0].kernel_key;
    ledger
        .append(EventBody::Proposed {
            proposal: EventProposal {
                text_pt: TaintedText::new("toda cor está ok".into()).unwrap(),
                goal,
                cites,
                speculative_on: vec![],
                vocabulary: hv,
            },
            author,
        })
        .unwrap();
    ledger
        .append(EventBody::ObligationCreated {
            obligation: ho,
            route: logos_ledger::Route::Lrat,
            artifact: *logos_crypto::ExternalDigest::of(cnf.dimacs()).as_bytes(),
            cnf: Some(logos_ledger::CnfRecord {
                hash: cnf_hash,
                encoder_version: 1,
            }),
        })
        .unwrap();
    let c = proof(&ledger, ho, &v);
    let head = ledger.head().unwrap();
    refused(&exchange(
        &mut ledger,
        &w.handlers,
        Message::Certify {
            obligation: ho,
            format: Text::new(LRAT).unwrap(),
            certificate: ObjectRef::Cas(c),
        },
    ));
    assert_eq!(ledger.head().unwrap(), head);
}

fn fact_on(ledger: &L<'_>, issuer: &FileSigner, w: &World, topic: &str) -> Hash<Fact> {
    let fact = Fact::sign(
        issuer,
        w.chove.clone(),
        w.hv,
        Text::new(topic).unwrap(),
        1_700_000_000,
        Validity::Until(u64::MAX),
        None,
    )
    .unwrap();
    ledger.cas().put::<Fact>(&to_canon(&fact).unwrap()).unwrap()
}

#[test]
fn criterion_db_monitor_signing_a_backup_fact_is_not_citable() {
    let temp = Temp::new();
    let kernel = signer();
    let path = temp.0.join("ledger");
    let mut ledger = Ledger::open(&path, &kernel).unwrap();
    let w = world(&mut ledger, &kernel);
    let db = FileSigner::from_seed(&[0xdb; 32]).unwrap();
    let backup = FileSigner::from_seed(&[0xba; 32]).unwrap();
    // Sem configuração publicada, ninguém atesta.
    let early = fact_on(&ledger, &db, &w, "db");
    assert!(matches!(
        register(&mut ledger, early),
        Err(TransportError::Unauthorized)
    ));
    let v1 = grant(
        &mut ledger,
        &[
            (&db, "db-monitor", &["db"]),
            (&backup, "backup-svc", &["backup"]),
        ],
        1,
    );
    // db-monitor assinando fato de tópico backup: recusado, nada gravado.
    let head = ledger.head().unwrap();
    let crossed = fact_on(&ledger, &db, &w, "backup");
    assert!(matches!(
        register(&mut ledger, crossed),
        Err(TransportError::Unauthorized)
    ));
    assert_eq!(ledger.head().unwrap(), head);
    assert_eq!(
        ledger.live_index().unwrap().fact_event(crossed).unwrap(),
        None
    );
    // Cada emissor no próprio tópico: citável, com a concessão explícita.
    let own = register(&mut ledger, early).unwrap();
    let backup_fact = fact_on(&ledger, &backup, &w, "backup");
    let backups = register(&mut ledger, backup_fact).unwrap();
    let index = ledger.live_index().unwrap();
    assert!(index.is_citable(own, 0).unwrap());
    assert!(index.is_citable(backups, 0).unwrap());
    assert_eq!(index.fact_trust(backups).unwrap(), Some(v1));
    drop(index);
    accepted(&propose(&mut ledger, &w, vec![own]));
    // Configuração nova (v2) tira `db` do db-monitor: o fato dele deixa de ser citável.
    let v2 = grant(
        &mut ledger,
        &[
            (&db, "db-monitor", &["db-replica"]),
            (&backup, "backup-svc", &["backup"]),
        ],
        2,
    );
    let index = ledger.live_index().unwrap();
    assert!(!index.is_citable(own, 0).unwrap());
    assert_eq!(index.fact_trust(own).unwrap(), None);
    assert_eq!(index.fact_trust(backups).unwrap(), Some(v2));
    assert_eq!(index.trust_version().unwrap(), Some(2));
    drop(index);
    let head = ledger.head().unwrap();
    refused(&propose(&mut ledger, &w, vec![own]));
    assert_eq!(ledger.head().unwrap(), head);
    accepted(&propose(&mut ledger, &w, vec![backups]));
    // Auditor independente reaplica as configurações e confere cada fato.
    drop(ledger);
    let report = logos_audit::verify(&path, &path.join("cas"), kernel.public_key()).unwrap();
    assert_eq!((report.facts, report.trust_configs), (2, 2));
    // Índice reconstruído do zero chega ao mesmo estado.
    let rebuilt = LedgerIndex::open(&path, kernel.public_key()).unwrap();
    assert!(!rebuilt.is_citable(own, 0).unwrap());
    assert!(rebuilt.is_citable(backups, 0).unwrap());
}

#[test]
fn trust_config_needs_the_admin_an_increasing_version_and_a_valid_object() {
    let temp = Temp::new();
    let kernel = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &kernel).unwrap();
    world(&mut ledger, &kernel);
    let db = FileSigner::from_seed(&[0xdb; 32]).unwrap();
    let config = |version| {
        TrustConfig::new(
            version,
            vec![TrustRoot {
                issuer: db.public_key().clone(),
                name: Text::new("db-monitor").unwrap(),
                topics: vec![Text::new("db").unwrap()],
                level: 1,
            }],
        )
        .unwrap()
    };
    let put = |ledger: &L<'_>, c: &TrustConfig| {
        ledger
            .cas()
            .put::<TrustConfig>(&to_canon(c).unwrap())
            .unwrap()
    };
    let admin = admin();
    let head = ledger.head().unwrap();
    // Outro signer que não o admin configurado.
    let h3 = put(&ledger, &config(3));
    assert!(matches!(
        publish_trust(&mut ledger, admin.public_key(), &db, h3),
        Err(TransportError::Unauthorized)
    ));
    // Objeto ausente e bytes que não são trust/v1.
    assert!(
        publish_trust(
            &mut ledger,
            admin.public_key(),
            &admin,
            Hash::from_bytes([4; 32])
        )
        .is_err()
    );
    let junk = ledger
        .cas()
        .put::<TrustConfig>(&to_canon(&Text::new("x").unwrap()).unwrap())
        .unwrap();
    assert!(publish_trust(&mut ledger, admin.public_key(), &admin, junk).is_err());
    assert_eq!(ledger.head().unwrap(), head);
    // Versão estritamente crescente.
    publish_trust(&mut ledger, admin.public_key(), &admin, h3).unwrap();
    let head = ledger.head().unwrap();
    for version in [2, 3] {
        let h = put(&ledger, &config(version));
        assert!(publish_trust(&mut ledger, admin.public_key(), &admin, h).is_err());
    }
    assert_eq!(ledger.head().unwrap(), head);
    // O evento carrega a assinatura do admin sobre h(config).
    let seq = ledger.head().unwrap().unwrap().seq;
    let event = ledger.events_from(seq, 1).unwrap().pop().unwrap();
    event.body.verify_authority(admin.public_key()).unwrap();
    assert!(event.body.verify_authority(db.public_key()).is_err());
    // Formato: versão 0, tópicos fora de ordem ou repetidos não constroem.
    assert!(TrustConfig::new(0, config(1).roots).is_err());
    let mut unordered = config(1).roots;
    unordered[0].topics = vec![Text::new("z").unwrap(), Text::new("a").unwrap()];
    assert!(TrustConfig::new(1, unordered).is_err());
    let mut twice = config(1).roots;
    twice.push(twice[0].clone());
    assert!(TrustConfig::new(1, twice).is_err());
}

#[test]
fn projection_and_auditor_refuse_a_fact_registered_without_trust() {
    let temp = Temp::new();
    let kernel = signer();
    let path = temp.0.join("ledger");
    let mut ledger = Ledger::open(&path, &kernel).unwrap();
    let w = world(&mut ledger, &kernel);
    let db = FileSigner::from_seed(&[0xdb; 32]).unwrap();
    grant(&mut ledger, &[(&db, "db-monitor", &["db"])], 1);
    // FactRegistered gravado sem o kernel para um tópico que o emissor não tem.
    let crossed = fact_on(&ledger, &db, &w, "backup");
    ledger
        .append(EventBody::FactRegistered { fact: crossed })
        .unwrap();
    assert!(ledger.live_index().is_err());
    drop(ledger);
    let error = logos_audit::verify(&path, &path.join("cas"), kernel.public_key()).unwrap_err();
    assert!(error.to_string().contains("not trusted"), "{error}");
}

#[test]
fn projection_refuses_used_facts_after_trust_is_withdrawn_and_stale_versions() {
    let temp = Temp::new();
    let kernel = signer();
    let path = temp.0.join("ledger");
    let mut ledger = Ledger::open(&path, &kernel).unwrap();
    let w = world(&mut ledger, &kernel);
    let backup = FileSigner::from_seed(&[0xba; 32]).unwrap();
    grant(&mut ledger, &[(&backup, "backup-svc", &["backup"])], 1);
    let f = fact_on(&ledger, &backup, &w, "backup");
    let registered = register(&mut ledger, f).unwrap();
    let reply = propose(&mut ledger, &w, vec![registered]);
    let o: Hash<Obligation> = logos_canon::CanonDecode::from_value(&accepted(&reply)[1]).unwrap();
    let c = proof(&ledger, o, &w.v);
    accepted(&exchange(
        &mut ledger,
        &w.handlers,
        Message::Certify {
            obligation: o,
            format: Text::new(LRAT).unwrap(),
            certificate: ObjectRef::Cas(c),
        },
    ));
    let seq = ledger.head().unwrap().unwrap().seq;
    let verified = ledger.events_from(seq, 1).unwrap().pop().unwrap().body;
    // v2 retira `backup`; a mesma rota gravada sem o kernel depois disso é recusada.
    let other = FileSigner::from_seed(&[0x0b; 32]).unwrap();
    grant(&mut ledger, &[(&other, "outro", &["outro"])], 2);
    ledger.live_index().unwrap();
    let mut stale = ledger.append(verified);
    assert!(stale.is_ok());
    assert!(ledger.live_index().is_err());
    drop(ledger);
    assert!(LedgerIndex::open(&path, kernel.public_key()).is_err());

    // Versão igual ou menor, gravada sem o kernel: projeção e auditor recusam.
    for version in [5, 4] {
        let temp = Temp::new();
        let path = temp.0.join("ledger");
        let mut ledger = Ledger::open(&path, &kernel).unwrap();
        world(&mut ledger, &kernel);
        grant(&mut ledger, &[(&backup, "backup-svc", &["backup"])], 5);
        let config = TrustConfig::new(
            version,
            vec![TrustRoot {
                issuer: other.public_key().clone(),
                name: Text::new("outro").unwrap(),
                topics: vec![Text::new("outro").unwrap()],
                level: 1,
            }],
        )
        .unwrap();
        let h = ledger
            .cas()
            .put::<TrustConfig>(&to_canon(&config).unwrap())
            .unwrap();
        let admin = admin();
        let mut body = EventBody::TrustConfigPublished {
            config: h,
            authority: logos_ledger::Authority {
                key: admin.key_id().unwrap(),
                sig: admin.sign::<Event>(b"").unwrap(),
            },
        };
        let message = body.authority_message().unwrap().unwrap();
        if let EventBody::TrustConfigPublished { authority, .. } = &mut body {
            authority.sig = admin.sign::<Event>(&message).unwrap();
        }
        stale = ledger.append(body);
        assert!(stale.is_ok());
        assert!(ledger.live_index().is_err(), "version {version}");
        drop(ledger);
        let error = logos_audit::verify(&path, &path.join("cas"), kernel.public_key()).unwrap_err();
        assert!(error.to_string().contains("version"), "{error}");
    }
}

#[test]
fn auditor_decodes_trust_configs_strictly() {
    use logos_canon::{Array, Canon};
    let db = FileSigner::from_seed(&[0xdb; 32]).unwrap();
    let text = |s: &str| Value::Text(Text::new(s).unwrap());
    let array = |items: Vec<Value>| Value::Array(Array::new(items).unwrap());
    let root = |topics: &[&str]| {
        array(vec![
            db.public_key().to_value().unwrap(),
            text("db-monitor"),
            array(topics.iter().map(|t| text(t)).collect()),
            Value::Unsigned(1),
        ])
    };
    // Formas que o decoder do ledger recusa; o auditor também, por conta própria.
    let bad = [
        (
            "topics not in strict order",
            array(vec![
                Value::Unsigned(1),
                Value::Unsigned(1),
                array(vec![root(&["z", "a"])]),
            ]),
        ),
        (
            "KeyId order",
            array(vec![
                Value::Unsigned(1),
                Value::Unsigned(1),
                array(vec![root(&["db"]), root(&["db"])]),
            ]),
        ),
        (
            "topics",
            array(vec![
                Value::Unsigned(1),
                Value::Unsigned(1),
                array(vec![root(&[])]),
            ]),
        ),
        (
            "roots",
            array(vec![Value::Unsigned(1), Value::Unsigned(1), array(vec![])]),
        ),
        (
            "text",
            array(vec![
                Value::Unsigned(1),
                Value::Unsigned(1),
                array(vec![root(&[""])]),
            ]),
        ),
        (
            "version",
            array(vec![
                Value::Unsigned(1),
                Value::Unsigned(0),
                array(vec![root(&["db"])]),
            ]),
        ),
    ];
    let kernel = signer();
    let admin = admin();
    for (reason, config) in bad {
        let temp = Temp::new();
        let path = temp.0.join("ledger");
        let mut ledger = Ledger::open(&path, &kernel).unwrap();
        world(&mut ledger, &kernel);
        let h = ledger
            .cas()
            .put::<TrustConfig>(&to_canon(&config).unwrap())
            .unwrap();
        assert!(logos_canon::from_canon::<TrustConfig>(&to_canon(&config).unwrap()).is_err());
        let mut body = EventBody::TrustConfigPublished {
            config: h,
            authority: logos_ledger::Authority {
                key: admin.key_id().unwrap(),
                sig: admin.sign::<Event>(b"").unwrap(),
            },
        };
        let message = body.authority_message().unwrap().unwrap();
        if let EventBody::TrustConfigPublished { authority, .. } = &mut body {
            authority.sig = admin.sign::<Event>(&message).unwrap();
        }
        let appended = ledger.append(body);
        assert!(appended.is_ok());
        drop(ledger);
        let error = logos_audit::verify(&path, &path.join("cas"), kernel.public_key()).unwrap_err();
        assert!(error.to_string().contains(reason), "{reason}: {error}");
    }
}

/// PROPOSE + CERTIFY LRAT de `chove` citando `cites`; devolve o Verified.
fn verify_citing(ledger: &mut L<'_>, w: &World, cites: Vec<Hash<Event>>) -> Hash<Event> {
    let reply = propose(ledger, w, cites);
    let o: Hash<Obligation> = logos_canon::CanonDecode::from_value(&accepted(&reply)[1]).unwrap();
    let c = proof(ledger, o, &w.v);
    let reply = exchange(
        ledger,
        &w.handlers,
        Message::Certify {
            obligation: o,
            format: Text::new(LRAT).unwrap(),
            certificate: ObjectRef::Cas(c),
        },
    );
    logos_canon::CanonDecode::from_value(&accepted(&reply)[2]).unwrap()
}

fn kind(ledger: &L<'_>, event: Hash<Event>) -> EntryKind {
    ledger
        .live_index()
        .unwrap()
        .status(event)
        .unwrap()
        .unwrap()
        .kind
}

#[test]
fn criterion_revoking_backup_svc_invalidates_exactly_the_claims_that_depended_on_it() {
    let temp = Temp::new();
    let kernel = signer();
    let path = temp.0.join("ledger");
    let mut ledger = Ledger::open(&path, &kernel).unwrap();
    let w = world(&mut ledger, &kernel);
    let db = FileSigner::from_seed(&[0xdb; 32]).unwrap();
    let backup = FileSigner::from_seed(&[0xba; 32]).unwrap();
    grant(
        &mut ledger,
        &[
            (&db, "db-monitor", &["db"]),
            (&backup, "backup-svc", &["backup"]),
        ],
        1,
    );
    // Fato antigo do backup-svc (época 0), anterior ao comprometimento.
    let early = fact_on(&ledger, &backup, &w, "backup");
    let early = register(&mut ledger, early).unwrap();
    advance(&mut ledger, 1);
    let late = Fact::sign(
        &backup,
        w.chove.clone(),
        w.hv,
        Text::new("backup").unwrap(),
        1_700_000_001,
        Validity::Until(u64::MAX),
        None,
    )
    .unwrap();
    let late = ledger.cas().put::<Fact>(&to_canon(&late).unwrap()).unwrap();
    let late = register(&mut ledger, late).unwrap();
    let from_db = fact_on(&ledger, &db, &w, "db");
    let from_db = register(&mut ledger, from_db).unwrap();
    // Claims: a depende do fato comprometido; d depende de a; b do db-monitor; e do
    // fato antigo do backup-svc.
    let a = verify_citing(&mut ledger, &w, vec![late]);
    let d = verify_citing(&mut ledger, &w, vec![a]);
    let b = verify_citing(&mut ledger, &w, vec![from_db]);
    let e = verify_citing(&mut ledger, &w, vec![early]);
    let used = |ledger: &L<'_>, claim| ledger.live_index().unwrap().dependents(claim).unwrap();
    assert_eq!(used(&ledger, late), vec![a]);
    assert_eq!(used(&ledger, a), vec![d]);
    // Só o admin revoga.
    let admin = admin();
    let issuer = backup.key_id().unwrap();
    assert!(matches!(
        revoke_issuer(&mut ledger, admin.public_key(), &db, issuer, 1),
        Err(TransportError::Unauthorized)
    ));
    let revocation = revoke_issuer(&mut ledger, admin.public_key(), &admin, issuer, 1).unwrap();
    // Invalidated gravados para a e d (causa: a revogação), e nada mais.
    assert_eq!(revocation.invalidated.len(), 2);
    for (claim, expected) in [
        (late, EntryKind::Invalidated),
        (a, EntryKind::Invalidated),
        (d, EntryKind::Invalidated),
        (early, EntryKind::Fact),
        (from_db, EntryKind::Fact),
        (b, EntryKind::Verified),
        (e, EntryKind::Verified),
    ] {
        assert_eq!(kind(&ledger, claim), expected);
    }
    let causes: Vec<_> = ledger
        .events_from(0, 256)
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.body {
            EventBody::Invalidated { claim, causes } => Some((claim, causes)),
            _ => None,
        })
        .collect();
    assert_eq!(
        causes,
        vec![
            (a, vec![revocation.revoked, late]),
            (d, vec![revocation.revoked, a])
        ]
    );
    // Idempotente; segunda revogação recusada; fatos novos dele recusados.
    assert!(
        logos_kernel::retract::Retractions::new(admin.public_key().clone())
            .reconcile(&mut ledger)
            .unwrap()
            .is_empty()
    );
    assert!(revoke_issuer(&mut ledger, admin.public_key(), &admin, issuer, 0).is_err());
    let after = Fact::sign(
        &backup,
        w.chove.clone(),
        w.hv,
        Text::new("backup").unwrap(),
        1_700_000_002,
        Validity::Until(u64::MAX),
        None,
    )
    .unwrap();
    let after = ledger
        .cas()
        .put::<Fact>(&to_canon(&after).unwrap())
        .unwrap();
    assert!(matches!(
        register(&mut ledger, after),
        Err(TransportError::Unauthorized)
    ));
    refused(&propose(&mut ledger, &w, vec![late]));
    // Índice reconstruído e auditor independente concordam.
    drop(ledger);
    let rebuilt = LedgerIndex::open(&path, kernel.public_key()).unwrap();
    assert_eq!(
        rebuilt.status(d).unwrap().unwrap().kind,
        EntryKind::Invalidated
    );
    assert_eq!(
        rebuilt.status(e).unwrap().unwrap().kind,
        EntryKind::Verified
    );
    drop(rebuilt);
    let cake = logos_lrat::CakeRegistry::register(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/cake-lpr/cake_lpr"
    )))
    .unwrap();
    let identity =
        logos_crypto::hash_raw::<logos_ledger::Checker>(&std::fs::read("/proc/self/exe").unwrap());
    let report = logos_audit::verify_with_checkers(
        &path,
        &path.join("cas"),
        kernel.public_key(),
        Some(&cake),
        &[*identity.as_bytes()],
    )
    .unwrap();
    assert_eq!((report.facts, report.lrat_proofs), (3, 4));
}

#[test]
fn admin_retracts_a_single_fact_and_its_dependents_fall() {
    let temp = Temp::new();
    let kernel = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &kernel).unwrap();
    let w = world(&mut ledger, &kernel);
    let trusted = issuer();
    trust(&mut ledger, &trusted, 1);
    let one = fact(&ledger, &trusted, &w, Validity::Until(u64::MAX));
    let one = register(&mut ledger, one).unwrap();
    let other = Fact::sign(
        &trusted,
        w.chove.clone(),
        w.hv,
        Text::new("clima").unwrap(),
        2,
        Validity::Until(u64::MAX),
        None,
    )
    .unwrap();
    let other = ledger
        .cas()
        .put::<Fact>(&to_canon(&other).unwrap())
        .unwrap();
    let other = register(&mut ledger, other).unwrap();
    let a = verify_citing(&mut ledger, &w, vec![one]);
    let b = verify_citing(&mut ledger, &w, vec![other]);
    let admin = admin();
    let retractions = logos_kernel::retract::Retractions::new(admin.public_key().clone());
    // O emissor do fato não retrata pelo kernel; só o admin.
    assert!(matches!(
        retractions.retract(&mut ledger, &trusted, one, Text::new("x").unwrap()),
        Err(TransportError::Unauthorized)
    ));
    let done = retractions
        .retract(
            &mut ledger,
            &admin,
            one,
            Text::new("leitura errada").unwrap(),
        )
        .unwrap();
    assert_eq!(done.invalidated.len(), 1);
    assert_eq!(kind(&ledger, one), EntryKind::Invalidated);
    assert_eq!(kind(&ledger, a), EntryKind::Invalidated);
    assert_eq!(kind(&ledger, b), EntryKind::Verified);
    assert!(!ledger.live_index().unwrap().is_citable(one, 0).unwrap());
}

#[test]
fn projection_and_auditor_refuse_facts_of_a_revoked_issuer_and_double_revocations() {
    let kernel = signer();
    let admin = admin();
    let backup = FileSigner::from_seed(&[0xba; 32]).unwrap();
    let issuer = backup.key_id().unwrap();
    let signed = |body: EventBody| {
        let mut body = body;
        let message = body.authority_message().unwrap().unwrap();
        if let EventBody::IssuerRevoked { authority, .. } = &mut body {
            authority.sig = admin.sign::<Event>(&message).unwrap();
        }
        body
    };
    let revocation = || {
        signed(EventBody::IssuerRevoked {
            issuer,
            since: 0,
            authority: logos_ledger::Authority {
                key: admin.key_id().unwrap(),
                sig: admin.sign::<Event>(b"").unwrap(),
            },
        })
    };
    for (case, reason) in [("fact", "revoked issuer"), ("twice", "revoked twice")] {
        let temp = Temp::new();
        let path = temp.0.join("ledger");
        let mut ledger = Ledger::open(&path, &kernel).unwrap();
        let w = world(&mut ledger, &kernel);
        grant(&mut ledger, &[(&backup, "backup-svc", &["backup"])], 1);
        revoke_issuer(&mut ledger, admin.public_key(), &admin, issuer, 0).unwrap();
        // Gravado sem o kernel depois da revogação.
        let forged = if case == "fact" {
            let f = fact_on(&ledger, &backup, &w, "backup");
            EventBody::FactRegistered { fact: f }
        } else {
            revocation()
        };
        let appended = ledger.append(forged);
        assert!(appended.is_ok());
        assert!(ledger.live_index().is_err(), "{case}");
        drop(ledger);
        let error = logos_audit::verify(&path, &path.join("cas"), kernel.public_key()).unwrap_err();
        assert!(error.to_string().contains(reason), "{case}: {error}");
    }
}
