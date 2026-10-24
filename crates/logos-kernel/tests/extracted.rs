//! Etapa 6.6 (ADR0068): `Extracted` é premissa "fonte diz φ", nunca fato; `Says_P` não é `P`.
#![cfg(target_os = "linux")]
mod support;

use logos_air::{
    AttestorId, Formula, FormulaNode, Grounding, Limits, Sort, SortDecl, SortDomain, Symbol,
    SymbolDecl, SymbolKind, SymbolResult, Term, TermNode, Vocabulary,
};
use logos_canon::{Text, Value, from_canon, to_canon};
use logos_core::Budget;
use logos_crypto::{FileSigner, Hash, Signer};
use logos_kernel::{
    TransportError,
    certify::CertificationHandlers,
    extracted::{EXTRACTION_TOPIC, SAYS_ATTESTOR, SAYS_PROCEDURE, register},
    facts::{publish_trust, revoke_issuer},
    registry::{CheckerRegistry, LRAT},
    retract::Retractions,
    server::Server,
    staging::Staging,
    wire::{Message, ObjectRef, Request, encode_frame},
};
use logos_ledger::{
    Certificate, Event, EventBody, EventProposal, Ledger, LedgerIndex, TaintedText, TrustConfig,
    TrustRoot,
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

struct World {
    v: Vocabulary,
    hv: Hash<Vocabulary>,
    handlers: CertificationHandlers,
    source: Symbol,
    obj: Symbol,
    pedido: Symbol,
    says_pedido: Symbol,
    closures: Vec<Hash<Event>>,
}

fn sort(name: &str) -> Sort {
    Sort {
        name: Text::new(name).unwrap(),
    }
}

fn symbol(name: &str, kind: SymbolKind, arity: Vec<Sort>, result: SymbolResult) -> Symbol {
    Symbol {
        name: Text::new(name).unwrap(),
        kind,
        arity,
        result,
    }
}

fn decl(signature: Symbol, grounding: Grounding) -> SymbolDecl {
    SymbolDecl {
        signature,
        grounding,
        gloss_pt: Text::new("símbolo de teste").unwrap(),
    }
}

fn anchored(attestor: &str, procedure: &str) -> Grounding {
    Grounding::Anchored {
        attestor: AttestorId(Text::new(attestor).unwrap()),
        procedure: Text::new(procedure).unwrap(),
    }
}

fn app(constant: &Symbol) -> Term {
    Term::new(
        vec![TermNode::App {
            symbol: constant.clone(),
            args: 0,
        }],
        Limits::default(),
    )
    .unwrap()
}

fn atom(predicate: &Symbol, args: &[&Symbol]) -> Formula {
    Formula::new(
        vec![FormulaNode::Pred {
            symbol: predicate.clone(),
            args: args.iter().map(|c| app(c)).collect(),
        }],
        Limits::default(),
    )
    .unwrap()
}

fn not(formula: &Formula) -> Formula {
    let mut nodes = vec![FormulaNode::Not];
    nodes.extend_from_slice(formula.nodes());
    Formula::new(nodes, Limits::default()).unwrap()
}

fn implies(a: &Formula, b: &Formula) -> Formula {
    let mut nodes = vec![FormulaNode::Implies];
    nodes.extend_from_slice(a.nodes());
    nodes.extend_from_slice(b.nodes());
    Formula::new(nodes, Limits::default()).unwrap()
}

/// `Source = {email1, email2}` e `Obj = {o1}` enumerados (fonte: fatos de fecho registrados);
/// `pedido(Obj)` glosado; `says_pedido` ancorado a `ingest`/`says-lift/v1`; `fake_says`
/// ancorado a outro atestador; `says_wrong` com primeiro argumento de sort errado.
fn vocabulary(sources: [logos_air::FactRef; 2]) -> (Vocabulary, [Symbol; 7]) {
    let loose = symbol(
        "says_loose",
        SymbolKind::Pred,
        vec![sort("Source"), sort("Obj")],
        SymbolResult::Bool,
    );
    let (source_sort, obj_sort) = (sort("Source"), sort("Obj"));
    let constant = |name: &str, of: &Sort| {
        symbol(
            name,
            SymbolKind::Const,
            vec![],
            SymbolResult::Sort(of.clone()),
        )
    };
    let predicate =
        |name: &str, arity: Vec<Sort>| symbol(name, SymbolKind::Pred, arity, SymbolResult::Bool);
    let source = constant("email1", &source_sort);
    let other = constant("email2", &source_sort);
    let obj = constant("o1", &obj_sort);
    let pedido = predicate("pedido", vec![obj_sort.clone()]);
    let says_pedido = predicate("says_pedido", vec![source_sort.clone(), obj_sort.clone()]);
    let fake = predicate("fake_says", vec![source_sort.clone(), obj_sort.clone()]);
    let wrong_first = predicate("says_wrong", vec![obj_sort.clone(), obj_sort.clone()]);
    let glossed = || Grounding::Glossed(Text::new("teste").unwrap());
    let enumerated = |name: &str, constants: Vec<Symbol>, r| SortDecl {
        name: Text::new(name).unwrap(),
        domain: SortDomain::Enumerated {
            constants,
            domain_source: logos_air::DomainSource::Fact(r),
        },
    };
    let v = Vocabulary::new(
        1,
        vec![
            enumerated("Source", vec![source.clone(), other.clone()], sources[0]),
            enumerated("Obj", vec![obj.clone()], sources[1]),
        ],
        vec![
            decl(
                source.clone(),
                anchored("ingest", "ingest/content-origin/v1"),
            ),
            decl(
                other.clone(),
                anchored("ingest", "ingest/content-origin/v1"),
            ),
            decl(obj.clone(), glossed()),
            decl(pedido.clone(), glossed()),
            decl(says_pedido.clone(), anchored(SAYS_ATTESTOR, SAYS_PROCEDURE)),
            decl(
                fake.clone(),
                anchored("clock", "clock/skew-vs-reference/v1"),
            ),
            decl(wrong_first.clone(), anchored(SAYS_ATTESTOR, SAYS_PROCEDURE)),
            decl(loose, anchored(SAYS_ATTESTOR, "ingest/content-origin/v1")),
        ],
    )
    .unwrap();
    (
        v,
        [source, other, obj, pedido, says_pedido, fake, wrong_first],
    )
}

fn closure(v: &Vocabulary, name: &str) -> Formula {
    logos_ground::domain_axioms(v, &sort(name))
        .unwrap()
        .conjunction()
        .unwrap()
}

fn fact_issuer() -> FileSigner {
    FileSigner::from_seed(&[0x61; 32]).unwrap()
}

/// Mundo de teste: confiança v1 (fatos de fecho em `domain`, extrator em `extractor_topic`) e os
/// dois fatos de fecho já registrados, para que as obrigações usem os sorts enumerados.
fn world(ledger: &mut L<'_>, kernel: &FileSigner, extractor_topic: &str) -> World {
    genesis(ledger, kernel);
    let registry = CheckerRegistry::new(kernel.public_key().clone(), None).unwrap();
    registry.install(ledger, kernel).unwrap();
    let (draft, _) = vocabulary([logos_air::FactRef([0; 32]); 2]);
    let refs = ["Source", "Obj"]
        .map(|n| logos_air::FactRef(*logos_crypto::hash(&closure(&draft, n)).unwrap().as_bytes()));
    let (v, [source, _other, obj, pedido, says_pedido, _, _]) = vocabulary(refs);
    let hv = ledger
        .cas()
        .put::<Vocabulary>(&to_canon(&v).unwrap())
        .unwrap();
    grant(ledger, &extractor(), extractor_topic, 1);
    let closures = ["Source", "Obj"]
        .map(|n| {
            let fact = logos_ledger::Fact::sign(
                &fact_issuer(),
                closure(&v, n),
                hv,
                Text::new("domain").unwrap(),
                1_700_000_000,
                logos_ledger::Validity::Until(u64::MAX),
                None,
            )
            .unwrap();
            let h = ledger
                .cas()
                .put::<logos_ledger::Fact>(&to_canon(&fact).unwrap())
                .unwrap();
            logos_kernel::facts::register(ledger, h).unwrap()
        })
        .to_vec();
    World {
        v,
        hv,
        handlers: CertificationHandlers::new(
            registry,
            Budget::new(2_000_000, 32 * 1024 * 1024, 4 * 1024 * 1024, 3000),
        ),
        source,
        obj,
        pedido,
        says_pedido,
        closures,
    }
}

fn extractor() -> FileSigner {
    FileSigner::from_seed(&[0xe1; 32]).unwrap()
}

fn admin() -> FileSigner {
    FileSigner::from_seed(&[0xad; 32]).unwrap()
}

/// Publica a confiança `version`: o emissor dos fatos de fecho em `domain` e o extrator em `topic`.
fn grant(ledger: &mut L<'_>, who: &FileSigner, topic: &str, version: u64) {
    let mut roots = vec![
        TrustRoot {
            issuer: fact_issuer().public_key().clone(),
            name: Text::new("dominio").unwrap(),
            topics: vec![Text::new("domain").unwrap()],
            level: 1,
        },
        TrustRoot {
            issuer: who.public_key().clone(),
            name: Text::new("extrator").unwrap(),
            topics: vec![Text::new(topic).unwrap()],
            level: 1,
        },
    ];
    roots.sort_by_key(|r| *r.issuer.key_id().unwrap().as_bytes());
    let config = TrustConfig::new(version, roots).unwrap();
    let h = ledger
        .cas()
        .put::<TrustConfig>(&to_canon(&config).unwrap())
        .unwrap();
    let admin = admin();
    publish_trust(ledger, admin.public_key(), &admin, h).unwrap();
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

fn says(w: &World) -> Formula {
    atom(&w.says_pedido, &[&w.source, &w.obj])
}

fn extract(
    ledger: &mut L<'_>,
    w: &World,
    who: &FileSigner,
    source: &str,
    formula: Formula,
) -> Result<Hash<Event>, TransportError> {
    register(
        ledger,
        who.key_id().unwrap(),
        w.hv,
        source,
        [0xc0; 32],
        formula,
    )
}

fn propose_goal(
    ledger: &mut L<'_>,
    w: &World,
    goal: Formula,
    mut cites: Vec<Hash<Event>>,
) -> Value {
    cites.extend(w.closures.iter().copied());
    exchange(
        ledger,
        &w.handlers,
        Message::Propose(EventProposal {
            text_pt: TaintedText::new("o pedido do e-mail vale".into()).unwrap(),
            goal,
            cites,
            speculative_on: vec![],
            vocabulary: w.hv,
        }),
    )
}

fn assume(ledger: &mut L<'_>, w: &World, formula: Formula, scope: &str) -> Hash<Event> {
    let reply = exchange(
        ledger,
        &w.handlers,
        Message::Assume {
            vocabulary: w.hv,
            formula,
            scope: Text::new(scope).unwrap(),
        },
    );
    logos_canon::CanonDecode::from_value(&accepted(&reply)[1]).unwrap()
}

fn obligation_of(reply: &Value) -> Hash<Obligation> {
    logos_canon::CanonDecode::from_value(&accepted(reply)[1]).unwrap()
}

/// DPLL mínimo (propagação unitária + ramificação): a CNF é pequena.
fn dpll(clauses: &[Vec<i64>], assignment: &mut std::collections::BTreeMap<i64, bool>) -> bool {
    loop {
        let mut unit = None;
        for clause in clauses {
            if clause
                .iter()
                .any(|l| assignment.get(&l.abs()) == Some(&(*l > 0)))
            {
                continue;
            }
            let open: Vec<i64> = clause
                .iter()
                .filter(|l| !assignment.contains_key(&l.abs()))
                .copied()
                .collect();
            match open.as_slice() {
                [] => return false,
                [only] => {
                    unit = Some(*only);
                    break;
                }
                _ => {}
            }
        }
        match unit {
            Some(l) => {
                assignment.insert(l.abs(), l > 0);
            }
            None => break,
        }
    }
    let Some(branch) = clauses
        .iter()
        .flatten()
        .find(|l| !assignment.contains_key(&l.abs()))
        .map(|l| l.abs())
    else {
        return true;
    };
    for value in [true, false] {
        let mut attempt = assignment.clone();
        attempt.insert(branch, value);
        if dpll(clauses, &mut attempt) {
            return true;
        }
    }
    false
}

/// Há atribuição que satisfaça a CNF de O? Se há, nenhuma prova LRAT de O existe.
fn satisfiable(ledger: &L<'_>, o: Hash<Obligation>, v: &Vocabulary) -> bool {
    let bytes = ledger.cas().get(&o).unwrap().unwrap();
    let ground =
        logos_ground::ground_for_check(&bytes, o, v, DecodeLimits::default(), Default::default())
            .unwrap();
    let (cnf, _) = logos_ground::clausify(&ground).unwrap();
    let clauses: Vec<Vec<i64>> = cnf
        .clauses()
        .iter()
        .map(|c| c.iter().map(|l| i64::from(*l)).collect())
        .collect();
    dpll(&clauses, &mut std::collections::BTreeMap::new())
}

fn certify(ledger: &mut L<'_>, w: &World, o: Hash<Obligation>) -> Hash<Event> {
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

#[test]
fn criterion_says_never_satisfies_the_plain_premise_without_the_endorsing_rule() {
    let temp = Temp::new();
    let kernel = signer();
    let path = temp.0.join("ledger");
    let mut ledger = Ledger::open(&path, &kernel).unwrap();
    let who = extractor();
    let w = world(&mut ledger, &kernel, EXTRACTION_TOPIC);
    let extracted = extract(&mut ledger, &w, &who, "email1", says(&w)).unwrap();
    let goal = atom(&w.pedido, &[&w.obj]);

    // Sozinha, `Says_pedido(email1, o1)` é citável mas não sustenta `pedido(o1)`: a CNF de O
    // (premissa ∧ ¬objetivo) é satisfatível, logo nenhuma prova LRAT dela existe.
    let alone = obligation_of(&propose_goal(
        &mut ledger,
        &w,
        goal.clone(),
        vec![extracted],
    ));
    assert!(satisfiable(&ledger, alone, &w.v));

    // Com a regra de política que endossa (aqui, uma assumption no mesmo escopo), prova real.
    let rule = assume(
        &mut ledger,
        &w,
        implies(&says(&w), &goal),
        "session:extracted",
    );
    let endorsed = obligation_of(&propose_goal(
        &mut ledger,
        &w,
        goal.clone(),
        vec![extracted, rule],
    ));
    assert!(!satisfiable(&ledger, endorsed, &w.v));
    let verified = certify(&mut ledger, &w, endorsed);

    // O certificado mostra a premissa Says em `used`, e a conclusão fica condicional a ela.
    let head = ledger.head().unwrap().unwrap().seq;
    let EventBody::Verified { used, .. } = ledger.events_from(head, 1).unwrap().pop().unwrap().body
    else {
        panic!("Verified")
    };
    assert!(used.contains(&extracted), "{used:?}");
    assert!(used.contains(&rule), "{used:?}");
    assert_eq!(
        ledger.live_index().unwrap().dependents(extracted).unwrap(),
        vec![verified]
    );
    let open = ledger
        .live_index()
        .unwrap()
        .open_assumptions(verified)
        .unwrap()
        .unwrap();
    assert!(
        open.iter()
            .any(|(event, scope)| *event == extracted && scope == "session:extracted"),
        "{open:?}"
    );
    // Premissa de confiança, não fato.
    assert_eq!(kind(&ledger, extracted), EntryKind::Assumption);

    // Auditor independente e índice reconstruído concordam.
    drop(ledger);
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
    // Os dois fatos de fecho são fatos; a extração não é.
    assert_eq!((report.facts, report.lrat_proofs), (2, 1));
    let rebuilt = LedgerIndex::open(&path, kernel.public_key()).unwrap();
    assert_eq!(
        rebuilt.status(extracted).unwrap().unwrap().kind,
        EntryKind::Assumption
    );
}

#[test]
fn extraction_needs_a_trusted_extractor_and_a_closed_says_literal_of_the_named_source() {
    let temp = Temp::new();
    let kernel = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &kernel).unwrap();
    let who = extractor();
    // Confiança só em outro tópico: o extrator não extrai.
    let w = world(&mut ledger, &kernel, "time");
    assert!(matches!(
        extract(&mut ledger, &w, &who, "email1", says(&w)),
        Err(TransportError::Unauthorized)
    ));
    grant(&mut ledger, &who, EXTRACTION_TOPIC, 2);
    let head = ledger.head().unwrap();
    let bad: Vec<(&str, &str, Formula)> = vec![
        // Predicado simples: seria um fato disfarçado de extração.
        ("plain predicate", "email1", atom(&w.pedido, &[&w.obj])),
        // Ancorado a outro atestador que não o levantamento Says.
        ("foreign grounding", "email1", {
            let fake =
                w.v.symbols()
                    .iter()
                    .find(|d| d.signature.name.as_str() == "fake_says")
                    .unwrap()
                    .signature
                    .clone();
            atom(&fake, &[&w.source, &w.obj])
        }),
        // Atestador certo, procedimento errado: não é o levantamento Says.
        ("right attestor, wrong procedure", "email1", {
            let loose =
                w.v.symbols()
                    .iter()
                    .find(|d| d.signature.name.as_str() == "says_loose")
                    .unwrap()
                    .signature
                    .clone();
            atom(&loose, &[&w.source, &w.obj])
        }),
        // Mal tipada: constante de sort Source onde se espera Obj.
        (
            "ill-sorted",
            "email1",
            atom(&w.says_pedido, &[&w.source, &w.source]),
        ),
        // A fonte nomeada não é a do primeiro argumento.
        ("other source", "email2", says(&w)),
        // Primeiro argumento de sort errado.
        ("first arg not Source", "email1", {
            let wrong =
                w.v.symbols()
                    .iter()
                    .find(|d| d.signature.name.as_str() == "says_wrong")
                    .unwrap()
                    .signature
                    .clone();
            atom(&wrong, &[&w.obj, &w.obj])
        }),
        // Fórmula composta.
        ("conjunction", "email1", {
            let mut nodes = vec![FormulaNode::And(2)];
            nodes.extend_from_slice(says(&w).nodes());
            nodes.extend_from_slice(says(&w).nodes());
            Formula::new(nodes, Limits::default()).unwrap()
        }),
        // Termo com variável livre.
        ("open term", "email1", {
            Formula::new(
                vec![FormulaNode::Pred {
                    symbol: w.says_pedido.clone(),
                    args: vec![
                        app(&w.source),
                        Term::new(
                            vec![TermNode::Var {
                                index: 0,
                                sort: sort("Obj"),
                            }],
                            Limits::default(),
                        )
                        .unwrap(),
                    ],
                }],
                Limits::default(),
            )
            .unwrap()
        }),
    ];
    for (name, source, formula) in bad {
        assert!(
            extract(&mut ledger, &w, &who, source, formula).is_err(),
            "{name} must be refused"
        );
        assert_eq!(ledger.head().unwrap(), head, "{name} wrote an event");
    }
    // A negação do literal levantado é aceita (a fonte disse que NÃO é pedido).
    let negated = extract(&mut ledger, &w, &who, "email1", not(&says(&w))).unwrap();
    assert_eq!(kind(&ledger, negated), EntryKind::Assumption);
    // Extrator revogado não extrai mais.
    let admin = admin();
    revoke_issuer(
        &mut ledger,
        admin.public_key(),
        &admin,
        who.key_id().unwrap(),
        0,
    )
    .unwrap();
    assert!(matches!(
        extract(&mut ledger, &w, &who, "email1", says(&w)),
        Err(TransportError::Unauthorized)
    ));
}

#[test]
fn only_the_extractor_or_the_admin_retracts_and_dependents_fall() {
    let temp = Temp::new();
    let kernel = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &kernel).unwrap();
    let who = extractor();
    let w = world(&mut ledger, &kernel, EXTRACTION_TOPIC);
    let goal = atom(&w.pedido, &[&w.obj]);
    let extracted = extract(&mut ledger, &w, &who, "email1", says(&w)).unwrap();
    let rule = assume(
        &mut ledger,
        &w,
        implies(&says(&w), &goal),
        "session:extracted",
    );
    let o = obligation_of(&propose_goal(&mut ledger, &w, goal, vec![extracted, rule]));
    let verified = certify(&mut ledger, &w, o);
    let admin = admin();
    let retractions = Retractions::new(admin.public_key().clone());
    let stranger = FileSigner::from_seed(&[0x13; 32]).unwrap();
    assert!(
        retractions
            .retract(
                &mut ledger,
                &stranger,
                extracted,
                Text::new("não é meu").unwrap()
            )
            .is_err()
    );
    assert_eq!(kind(&ledger, verified), EntryKind::Verified);
    // O próprio extrator retrata a extração; a conclusão que dependia dela cai.
    let retraction = retractions
        .retract(
            &mut ledger,
            &who,
            extracted,
            Text::new("erro de leitura").unwrap(),
        )
        .unwrap();
    assert_eq!(retraction.invalidated.len(), 1);
    assert_eq!(kind(&ledger, verified), EntryKind::Invalidated);
    // Retratada, a extração deixa de ser citável.
    assert!(
        !ledger
            .live_index()
            .unwrap()
            .is_citable(extracted, ledger.head().unwrap().unwrap().epoch)
            .unwrap()
    );
}
