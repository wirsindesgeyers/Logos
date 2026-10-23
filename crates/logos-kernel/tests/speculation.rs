//! Etapa 5.8 (ADR0058): especulação sobre Pending pelo transporte e ledger reais.
#![cfg(target_os = "linux")]
mod support;

use logos_air::{
    Formula, FormulaNode, Grounding, Limits, Symbol, SymbolDecl, SymbolKind, SymbolResult,
    Vocabulary,
};
use logos_canon::{Text, Value, from_canon, to_canon};
use logos_core::Budget;
use logos_crypto::{Hash, Signer, hash};
use logos_kernel::{
    certify::CertificationHandlers,
    registry::{CheckerRegistry, LRAT},
    retract::Retractions,
    server::{Handlers, Server},
    staging::Staging,
    wire::{Message, ObjectRef, Request, encode_frame},
    witness::PremiseUse,
};
use logos_ledger::{
    Certificate, Event, EventBody, EventProposal, Ledger, ModelObject, TaintedText,
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

type L<'a> = Ledger<'a, logos_crypto::FileSigner>;

fn budget() -> Budget {
    Budget::new(2_000_000, 32 * 1024 * 1024, 4 * 1024 * 1024, 3000)
}
fn items(value: &Value) -> &[Value] {
    let Value::Array(a) = value else {
        panic!("expected array {value:?}")
    };
    a.items()
}
fn exchange<H: Handlers<logos_crypto::FileSigner>>(
    ledger: &mut L<'_>,
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
            client
                .set_read_timeout(Some(std::time::Duration::from_secs(600)))
                .unwrap();
            client
                .write_all(&encode_frame(&Request { id: 58, message }).unwrap())
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
fn ok(response: &Value) -> &Value {
    assert_eq!(items(response)[2], Value::Unsigned(1), "{response:?}");
    &items(response)[3]
}
fn refused(response: &Value) {
    assert_eq!(items(response)[2], Value::Unsigned(0), "{response:?}");
}

/// Ledger com checkers registrados e vocabulário proposicional P, Q.
struct World {
    _temp: Temp,
    key: logos_crypto::KeyId,
    handlers: Option<CertificationHandlers>,
    v: Vocabulary,
    atoms: Vec<Formula>,
}

impl World {
    fn new(signer: &logos_crypto::FileSigner) -> (Self, std::path::PathBuf) {
        let temp = Temp::new();
        let path = temp.0.join("ledger");
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
        let atoms = symbols
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
        (
            Self {
                _temp: temp,
                key: signer.key_id().unwrap(),
                handlers: None,
                v,
                atoms,
            },
            path,
        )
    }

    fn start(&mut self, ledger: &mut L<'_>, signer: &logos_crypto::FileSigner) {
        genesis(ledger, signer);
        let registry = CheckerRegistry::new(signer.public_key().clone(), None).unwrap();
        registry.install(ledger, signer).unwrap();
        self.handlers = Some(CertificationHandlers::new(registry, budget()));
    }

    fn h(&self) -> &CertificationHandlers {
        self.handlers.as_ref().unwrap()
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

    fn proposal(
        &self,
        ledger: &L<'_>,
        goal: Formula,
        cites: Vec<Hash<Event>>,
        speculative_on: Vec<Hash<Event>>,
    ) -> EventProposal {
        EventProposal {
            text_pt: TaintedText::new("especulação".into()).unwrap(),
            goal,
            cites,
            speculative_on: speculative_on.iter().map(|h| *h.as_bytes()).collect(),
            vocabulary: ledger
                .cas()
                .put::<Vocabulary>(&to_canon(&self.v).unwrap())
                .unwrap(),
        }
    }

    /// PROPOSE aceito: (h(O), h(ObligationCreated)) — o Pending vem do ledger,
    /// como o agente o obteria por Subscribe.
    fn propose(&self, ledger: &mut L<'_>, p: EventProposal) -> (Hash<Obligation>, Hash<Event>) {
        let reply = exchange(ledger, self.h(), Message::Propose(p));
        let o = logos_canon::CanonDecode::from_value(&items(ok(&reply))[1]).unwrap();
        let head = ledger.head().unwrap().unwrap();
        let last = ledger.events_from(head.seq, 1).unwrap().pop().unwrap();
        assert!(
            matches!(last.body, EventBody::ObligationCreated { obligation, .. } if obligation == o)
        );
        (o, head.hash)
    }

    fn try_propose(&self, ledger: &mut L<'_>, p: EventProposal) -> Value {
        exchange(ledger, self.h(), Message::Propose(p))
    }

    fn assume(&self, ledger: &mut L<'_>, formula: Formula, scope: &str) -> Hash<Event> {
        let hv = ledger
            .cas()
            .put::<Vocabulary>(&to_canon(&self.v).unwrap())
            .unwrap();
        hash(
            &ledger
                .append(EventBody::AssumptionDeclared {
                    vocabulary: hv,
                    formula,
                    scope: Text::new(scope).unwrap(),
                    author: self.key,
                })
                .unwrap(),
        )
        .unwrap()
    }

    /// Prova LRAT por propagação unitária sobre a CNF de O.
    fn proof(&self, ledger: &L<'_>, o: Hash<Obligation>) -> Hash<Certificate> {
        let bytes = ledger.cas().get(&o).unwrap().unwrap();
        let ground = logos_ground::ground_for_check(
            &bytes,
            o,
            &self.v,
            DecodeLimits::default(),
            Default::default(),
        )
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

    fn certify(&self, ledger: &mut L<'_>, o: Hash<Obligation>) -> Value {
        let c = self.proof(ledger, o);
        exchange(
            ledger,
            self.h(),
            Message::Certify {
                obligation: o,
                format: Text::new(LRAT).unwrap(),
                certificate: ObjectRef::Cas(c),
            },
        )
    }

    fn verified(&self, ledger: &mut L<'_>, o: Hash<Obligation>) -> Hash<Event> {
        let reply = self.certify(ledger, o);
        let fields = items(ok(&reply));
        assert_eq!(fields[1], Value::Unsigned(0), "{reply:?}");
        logos_canon::CanonDecode::from_value(&fields[2]).unwrap()
    }

    /// Contramodelo: todo átomo falso.
    fn model(&self, ledger: &L<'_>) -> Hash<ModelObject> {
        let model = FiniteModel::new(
            ModelData {
                vocab: hash(&self.v).unwrap(),
                domains: vec![],
                consts: vec![],
                funcs: vec![],
                preds: (0..self.atoms.len() as u32)
                    .map(|id| (id, vec![]))
                    .collect(),
            },
            &self.v,
            ModelLimits::default(),
        )
        .unwrap();
        ledger
            .cas()
            .put::<ModelObject>(&to_canon(&model).unwrap())
            .unwrap()
    }

    fn refute(&self, ledger: &mut L<'_>, o: Hash<Obligation>) -> Hash<Event> {
        let model = self.model(ledger);
        let reply = exchange(
            ledger,
            self.h(),
            Message::Refute {
                obligation: o,
                model: ObjectRef::Cas(model),
            },
        );
        let fields = items(ok(&reply));
        assert_eq!(fields[1], Value::Unsigned(1), "{reply:?}");
        logos_canon::CanonDecode::from_value(&fields[2]).unwrap()
    }
}

fn kind(ledger: &L<'_>, id: Hash<Event>) -> EntryKind {
    ledger
        .live_index()
        .unwrap()
        .status(id)
        .unwrap()
        .unwrap()
        .kind
}

/// Invalidated gravados no ledger: alvo -> causes.
fn invalidations(ledger: &L<'_>) -> Vec<(Hash<Event>, Vec<Hash<Event>>)> {
    let head = ledger.head().unwrap().unwrap().seq;
    let mut out = Vec::new();
    let mut next = 0;
    while next <= head {
        let page = ledger.events_from(next, 256).unwrap();
        next += page.len() as u64;
        for event in page {
            if let EventBody::Invalidated { claim, causes } = event.body {
                out.push((claim, causes));
            }
        }
    }
    out
}

fn last_verified(ledger: &L<'_>) -> (Vec<Hash<Event>>, Vec<Hash<Event>>) {
    let head = ledger.head().unwrap().unwrap().seq;
    let event = ledger.events_from(head, 1).unwrap().pop().unwrap();
    let EventBody::Verified { cited, used, .. } = event.body else {
        panic!("expected Verified, got {:?}", event.body)
    };
    (cited, used)
}

#[test]
fn criterion_c8_speculates_on_c7_and_refuting_c7_invalidates_c8_with_speculation_failed() {
    let s = signer();
    let (mut w, path) = World::new(&s);
    let mut ledger = Ledger::open(&path, &s).unwrap();
    w.start(&mut ledger, &s);
    let q = w.atoms[1].clone();
    // C7: Q sem citações (refutável); C8 especula sobre C7; C9 especula sobre C8.
    let p7 = w.proposal(&ledger, q.clone(), vec![], vec![]);
    let (o7, c7) = w.propose(&mut ledger, p7);
    let p8 = w.proposal(&ledger, q.clone(), vec![], vec![c7]);
    let (o8, c8) = w.propose(&mut ledger, p8);
    let p9 = w.proposal(&ledger, q.clone(), vec![], vec![c8]);
    let (o9, c9) = w.propose(&mut ledger, p9);
    for id in [c7, c8, c9] {
        assert_eq!(kind(&ledger, id), EntryKind::Pending);
    }
    // Γ8 contém a meta de C7 (referenciada pelo Pending de C7).
    let o8_bytes = ledger.cas().get(&o8).unwrap().unwrap();
    assert!(o8_bytes.windows(32).any(|w| w == c7.as_bytes()));

    // Base não Verified: CERTIFY do especulativo é recusado sem evento.
    let head = ledger.head().unwrap();
    refused(&w.certify(&mut ledger, o8));
    assert_eq!(ledger.head().unwrap(), head);

    // P5: Pending especulativo não é citável, nem por PROPOSE nem por ação.
    let cite_pending = w.proposal(&ledger, q.clone(), vec![c8], vec![]);
    refused(&w.try_propose(&mut ledger, cite_pending));
    assert_eq!(ledger.head().unwrap(), head);
    for id in [c7, c8, c9] {
        for usage in [PremiseUse::Action, PremiseUse::Logical] {
            assert!(!w.h().premise_eligible(&ledger, id, usage).unwrap());
        }
    }

    // C7 refutado: C8 e C9 caem com SpeculationFailed, em cascata transitiva.
    let refuted = w.refute(&mut ledger, o7);
    assert_eq!(
        invalidations(&ledger),
        vec![(c8, vec![refuted, c7]), (c9, vec![refuted, c8])]
    );
    assert_eq!(kind(&ledger, c8), EntryKind::Invalidated);
    assert_eq!(kind(&ledger, c9), EntryKind::Invalidated);
    let head = ledger.head().unwrap();
    for o in [o8, o9] {
        refused(&w.certify(&mut ledger, o));
    }
    // Nova especulação sobre base refutada/invalidada é recusada.
    for base in [c7, c8] {
        let p = w.proposal(&ledger, q.clone(), vec![], vec![base]);
        refused(&w.try_propose(&mut ledger, p));
    }
    assert_eq!(ledger.head().unwrap(), head);

    // Idempotente e reconstruível: reconcile não grava nada; índice do zero igual.
    let authority = s.public_key().clone();
    assert!(
        Retractions::new(authority.clone())
            .reconcile(&mut ledger)
            .unwrap()
            .is_empty()
    );
    drop(ledger);
    let mut ledger = Ledger::open(&path, &s).unwrap();
    assert!(
        Retractions::new(authority)
            .reconcile(&mut ledger)
            .unwrap()
            .is_empty()
    );
    assert_eq!(ledger.head().unwrap(), head);
    assert_eq!(kind(&ledger, c9), EntryKind::Invalidated);
}

#[test]
fn verified_base_lets_the_speculative_certify_with_used_mapped_to_the_base_route() {
    let s = signer();
    let (mut w, path) = World::new(&s);
    let mut ledger = Ledger::open(&path, &s).unwrap();
    w.start(&mut ledger, &s);
    let p_atom = w.atoms[0].clone();
    let a = w.assume(&mut ledger, p_atom.clone(), "");
    // C7: P citando a assumption P; C8: P especulando sobre C7 (a prova usa a base).
    let p7 = w.proposal(&ledger, p_atom.clone(), vec![a], vec![]);
    let (o7, c7) = w.propose(&mut ledger, p7);
    let p8 = w.proposal(&ledger, p_atom.clone(), vec![], vec![c7]);
    let (o8, _c8) = w.propose(&mut ledger, p8);
    refused(&w.certify(&mut ledger, o8));

    let r7 = w.verified(&mut ledger, o7);
    // Base concluída não aceita nova especulação: deve ser citada.
    let late = w.proposal(&ledger, p_atom.clone(), vec![], vec![c7]);
    refused(&w.try_propose(&mut ledger, late));
    // Rota Verified também não é base especulativa válida.
    let wrong = w.proposal(&ledger, p_atom.clone(), vec![], vec![r7]);
    refused(&w.try_propose(&mut ledger, wrong));

    let r8 = w.verified(&mut ledger, o8);
    assert_eq!(last_verified(&ledger), (vec![r7], vec![r7]));
    assert_eq!(
        ledger.live_index().unwrap().dependents(r7).unwrap(),
        vec![r8]
    );
    // r8 é um Verified comum e citável (Action pede ainda testemunha de consistência, 5.5).
    assert!(
        w.h()
            .premise_eligible(&ledger, r8, PremiseUse::Logical)
            .unwrap()
    );

    // Depois de Verified vale o ATMS comum: retratar a assumption derruba r7 e r8
    // pelas arestas used, sem nenhum SpeculationFailed.
    let retraction = Retractions::new(s.public_key().clone())
        .retract(&mut ledger, &s, a, Text::new("fixture").unwrap())
        .unwrap();
    assert_eq!(
        invalidations(&ledger),
        vec![
            (r7, vec![retraction.retracted, a]),
            (r8, vec![retraction.retracted, r7])
        ]
    );
    assert_eq!(kind(&ledger, r8), EntryKind::Invalidated);
}

#[test]
fn base_dying_after_verification_drops_a_still_pending_speculative() {
    let s = signer();
    let (mut w, path) = World::new(&s);
    let mut ledger = Ledger::open(&path, &s).unwrap();
    w.start(&mut ledger, &s);
    let p_atom = w.atoms[0].clone();
    let a = w.assume(&mut ledger, p_atom.clone(), "");
    let p7 = w.proposal(&ledger, p_atom.clone(), vec![a], vec![]);
    let (o7, c7) = w.propose(&mut ledger, p7);
    let p8 = w.proposal(&ledger, p_atom.clone(), vec![], vec![c7]);
    let (o8, c8) = w.propose(&mut ledger, p8);
    let r7 = w.verified(&mut ledger, o7);
    let retraction = Retractions::new(s.public_key().clone())
        .retract(&mut ledger, &s, a, Text::new("fixture").unwrap())
        .unwrap();
    // r7 cai pelo ATMS; C8, ainda Pending, cai por especulação com a mesma raiz.
    let r7_invalidated = (r7, vec![retraction.retracted, a]);
    let c8_invalidated = (c8, vec![retraction.retracted, c7]);
    assert_eq!(invalidations(&ledger), vec![r7_invalidated, c8_invalidated]);
    assert_eq!(retraction.invalidated.len(), 2);
    refused(&w.certify(&mut ledger, o8));
}

#[test]
fn speculative_proposals_refuse_unknown_bases_mixed_scopes_and_consistency_witness() {
    let s = signer();
    let (mut w, path) = World::new(&s);
    let mut ledger = Ledger::open(&path, &s).unwrap();
    w.start(&mut ledger, &s);
    let p_atom = w.atoms[0].clone();
    let head = ledger.head().unwrap();
    // Base que não é instância Pending: hash arbitrário e assumption.
    let a = w.assume(&mut ledger, p_atom.clone(), "session:a");
    let b = w.assume(&mut ledger, p_atom.clone(), "session:b");
    let head_after = ledger.head().unwrap();
    assert_ne!(head, head_after);
    for base in [Hash::from_bytes([7; 32]), a] {
        let p = w.proposal(&ledger, p_atom.clone(), vec![], vec![base]);
        refused(&w.try_propose(&mut ledger, p));
    }
    // Escopo derivado inclui as citações da base: session:a ∪ session:b é recusado.
    let p7 = w.proposal(&ledger, p_atom.clone(), vec![a], vec![]);
    let (_o7, c7) = w.propose(&mut ledger, p7);
    let mixed = w.proposal(&ledger, p_atom.clone(), vec![b], vec![c7]);
    let head = ledger.head().unwrap();
    refused(&w.try_propose(&mut ledger, mixed));
    assert_eq!(ledger.head().unwrap(), head);
    // Mesmo escopo é aceito; WITNESS de proposta especulativa é recusado.
    let same = w.proposal(&ledger, p_atom.clone(), vec![a], vec![c7]);
    let (o8, _) = w.propose(&mut ledger, same);
    let model = w.model(&ledger);
    let head = ledger.head().unwrap();
    refused(&exchange(
        &mut ledger,
        w.h(),
        Message::Witness {
            obligation: o8,
            model: ObjectRef::Cas(model),
        },
    ));
    assert_eq!(ledger.head().unwrap(), head);
}

/// Profundidade de propagação de erro (Apêndice D; §3 "intercalado"): quantos
/// claims dependem de um nó inválido no momento em que ele é detectado. Cadeias
/// aleatórias de passos `true` com um passo errado (`false`) em posição aleatória.
#[test]
fn criterion_error_propagation_depth_is_zero_without_speculation_over_random_chains() {
    let chains: usize = if std::env::var("LOGOS_TIER").is_ok_and(|t| t == "fast") {
        40
    } else {
        1000
    };
    // Lotes independentes (um ledger cada) rodam em paralelo; sementes por lote
    // mantêm o resultado determinístico.
    const BATCH: usize = 25;
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    let next_batch = std::sync::atomic::AtomicUsize::new(0);
    let totals = Mutex::new((0, 0, 0));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let batch = next_batch.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let first = batch * BATCH;
                    if first >= chains {
                        break;
                    }
                    let result = depth_batch(first, BATCH.min(chains - first));
                    let mut t = totals.lock().unwrap();
                    t.0 += result.0;
                    t.1 += result.1;
                    t.2 += result.2;
                }
            });
        }
    });
    let (interleaved_depth, speculative_depth, speculative_chains) = *totals.lock().unwrap();
    println!(
        "ERROR_PROPAGATION_DEPTH chains={chains} interleaved_depth={interleaved_depth} \
         speculative_chains={speculative_chains} speculative_depth={speculative_depth}"
    );
    assert_eq!(interleaved_depth, 0);
    assert!(speculative_depth > 0);
}

/// Um lote de cadeias num ledger só: o registro do checker (rehash do executável)
/// e o replay ficam amortizados; cada cadeia ancora numa assumption própria, então
/// nenhuma obrigação se repete entre cadeias (S4 recusaria um ⊢false repetido).
/// Devolve (profundidade intercalada, profundidade especulativa, cadeias especulativas).
fn depth_batch(first: usize, count: usize) -> (usize, usize, usize) {
    let mut seed: u64 = 0x5eed_0058 ^ (first as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let mut next = |bound: u64| {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) % bound
    };
    let (mut interleaved_depth, mut speculative_depth, mut speculative_chains) = (0, 0, 0);
    let s = signer();
    let (mut w, path) = World::new(&s);
    let mut ledger = Ledger::open(&path, &s).unwrap();
    w.start(&mut ledger, &s);
    for chain in first..first + count {
        let anchor = w.assume(&mut ledger, World::truth(true), "");
        let length = 2 + next(4) as usize;
        let wrong = next(length as u64) as usize;
        if chain % 10 == 0 {
            // Controle positivo: com especulação a métrica não é trivialmente zero.
            let mut steps = Vec::new();
            let mut base = None;
            for i in 0..length {
                let p = w.proposal(
                    &ledger,
                    World::truth(i != wrong),
                    if i == 0 { vec![anchor] } else { vec![] },
                    base.into_iter().collect(),
                );
                let (o, pending) = w.propose(&mut ledger, p);
                steps.push(o);
                base = Some(pending);
            }
            for (i, o) in steps.iter().enumerate() {
                if i == wrong {
                    let refuted = w.refute(&mut ledger, *o);
                    let depth = invalidations(&ledger)
                        .iter()
                        .filter(|(_, causes)| causes[0] == refuted)
                        .count();
                    assert_eq!(depth, length - wrong - 1);
                    speculative_depth += depth;
                    break;
                }
                w.verified(&mut ledger, *o);
            }
            speculative_chains += 1;
            continue;
        }
        // Intercalado sem especulação: o passo i só cita o passo i-1 Verified.
        let mut previous = anchor;
        for i in 0..length {
            let p = w.proposal(&ledger, World::truth(i != wrong), vec![previous], vec![]);
            let (o, pending) = w.propose(&mut ledger, p);
            if i == wrong {
                // O passo seguinte tenta citar o errado ainda Pending: recusado.
                let next_step = w.proposal(&ledger, World::truth(true), vec![pending], vec![]);
                refused(&w.try_propose(&mut ledger, next_step));
                let refuted = w.refute(&mut ledger, o);
                let index = ledger.live_index().unwrap();
                let depth = index.dependents(pending).unwrap().len()
                    + index.dependents(refuted).unwrap().len()
                    + invalidations(&ledger)
                        .iter()
                        .filter(|(_, causes)| causes[0] == refuted)
                        .count();
                interleaved_depth += depth;
                break;
            }
            previous = w.verified(&mut ledger, o);
        }
    }
    (interleaved_depth, speculative_depth, speculative_chains)
}
