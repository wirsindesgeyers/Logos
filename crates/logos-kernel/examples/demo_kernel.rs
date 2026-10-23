//! Etapa 5.12 (ADR0060): kernel mínimo ponta a ponta, demo scriptada.
//!
//! vocabulário → fatos de teste (atestador stub) → 3 propostas pelo transporte
//! real → Verified via LRAT, Refuted via modelo, Unknown por orçamento esgotado →
//! o atestador retrata um fato → cascata. O `logos-audit` independente verifica e
//! recheca o ledger depois (ci/demo-kernel.sh).
//!
//! Uso: `demo_kernel DIR` — cria DIR/ledger e imprime DEMO_KERNEL_OK com a chave
//! pública do kernel e a identidade do checker Rust (este executável) em hex.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(target_os = "linux")]
mod demo {
    use std::io::{Read, Write};
    use std::net::Shutdown;
    use std::os::unix::net::UnixStream;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use logos_air::{
        Formula, FormulaNode, Grounding, Limits, Symbol, SymbolDecl, SymbolKind, SymbolResult,
        Vocabulary,
    };
    use logos_canon::{CanonDecode, Text, Value, from_canon, to_canon};
    use logos_core::Budget;
    use logos_crypto::{ExternalDigest, FileSigner, Hash, Signer, hash, hash_raw};
    use logos_kernel::{
        certify::CertificationHandlers,
        registry::{CheckerRegistry, LRAT},
        retract::Retractions,
        server::Server,
        staging::Staging,
        wire::{Message, ObjectRef, Request, encode_frame},
    };
    use logos_ledger::{
        Certificate, Checker, Event, EventBody, EventProposal, Ledger, ModelObject, TaintedText,
    };
    use logos_model::{FiniteModel, ModelData, ModelLimits};
    use logos_obligation::{DecodeLimits, EntryKind, Obligation};

    /// Orçamento do kernel da demo: pequeno o bastante para a prova longa estourar.
    /// Checkers internos ignoram tempo de relógio; o "timeout" deles é este teto de
    /// passos, determinístico (ADR0060).
    const STEPS: u64 = 50_000;
    const COPIES: usize = 100_000;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn items(value: &Value) -> &[Value] {
        let Value::Array(a) = value else {
            panic!("expected array {value:?}")
        };
        a.items()
    }

    /// Uma mensagem pelo transporte real: socket, frame, handler e resposta.
    fn exchange(
        root: &Path,
        ledger: &mut Ledger<'_, FileSigner>,
        handlers: &CertificationHandlers,
        message: Message,
    ) -> Value {
        let uid = rustix::process::geteuid().as_raw();
        let socket = root.join("socket");
        let _ = std::fs::remove_file(&socket);
        let server = Server::bind(&socket, uid, Default::default()).unwrap();
        let staging = Staging::open(
            &root.join("staging"),
            &root.join("private"),
            uid,
            64 * 1024 * 1024,
        )
        .unwrap();
        let (mut client, stream) = UnixStream::pair().unwrap();
        let writer = Mutex::new(ledger);
        std::thread::scope(|scope| {
            let worker = scope.spawn(move || {
                client
                    .write_all(&encode_frame(&Request { id: 512, message }).unwrap())
                    .unwrap();
                client.shutdown(Shutdown::Write).unwrap();
                let mut len = [0; 4];
                client.read_exact(&mut len).unwrap();
                let mut bytes = vec![0; u32::from_be_bytes(len) as usize];
                client.read_exact(&mut bytes).unwrap();
                from_canon(&bytes).unwrap()
            });
            server
                .serve_stream(stream, &writer, handlers, &staging)
                .unwrap();
            worker.join().unwrap()
        })
    }

    fn ok(response: &Value) -> &[Value] {
        assert_eq!(items(response)[2], Value::Unsigned(1), "{response:?}");
        items(&items(response)[3])
    }

    fn atom(symbol: &Symbol) -> Vec<FormulaNode> {
        vec![FormulaNode::Pred {
            symbol: symbol.clone(),
            args: vec![],
        }]
    }

    fn formula(nodes: Vec<FormulaNode>) -> Formula {
        Formula::new(nodes, Limits::default()).unwrap()
    }

    /// Atestador de teste (stub até a Fase 6): assina fatos como assumptions
    /// `session:fatos-de-teste` com a própria chave, e só ele (ou o admin) retrata.
    struct TestAttester {
        signer: FileSigner,
    }

    impl TestAttester {
        fn attest(
            &self,
            ledger: &mut Ledger<'_, FileSigner>,
            vocabulary: Hash<Vocabulary>,
            fact: Formula,
        ) -> Hash<Event> {
            hash(
                &ledger
                    .append(EventBody::AssumptionDeclared {
                        vocabulary,
                        formula: fact,
                        scope: Text::new("session:fatos-de-teste").unwrap(),
                        author: self.signer.key_id().unwrap(),
                    })
                    .unwrap(),
            )
            .unwrap()
        }
    }

    /// CNF de O e cláusulas da refutação por propagação unitária.
    fn cnf(
        ledger: &Ledger<'_, FileSigner>,
        o: Hash<Obligation>,
        v: &Vocabulary,
    ) -> (Vec<Vec<i64>>, Vec<usize>) {
        let bytes = ledger.cas().get(&o).unwrap().unwrap();
        let ground = logos_ground::ground_for_check(
            &bytes,
            o,
            v,
            DecodeLimits::default(),
            Default::default(),
        )
        .unwrap();
        let (cnf, _) = logos_ground::clausify(&ground).unwrap();
        let clauses: Vec<Vec<i64>> = cnf
            .clauses()
            .iter()
            .map(|c| c.iter().map(|l| i64::from(*l)).collect())
            .collect();
        let mut assigned = std::collections::BTreeMap::new();
        let mut hints = vec![];
        'propagate: loop {
            let mut progress = false;
            for (i, clause) in clauses.iter().enumerate() {
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
            assert!(
                progress,
                "demo obligation needs a unit-propagation refutation"
            );
        }
        (clauses, hints)
    }

    /// Prova LRAT: `copies` cópias RUP da primeira cláusula (válidas, mas caras) e
    /// depois a cláusula vazia com as dicas da propagação unitária.
    fn proof(
        ledger: &Ledger<'_, FileSigner>,
        o: Hash<Obligation>,
        v: &Vocabulary,
        copies: usize,
    ) -> Hash<Certificate> {
        let (clauses, hints) = cnf(ledger, o, v);
        let mut text = String::new();
        let first: Vec<String> = clauses[0].iter().map(ToString::to_string).collect();
        for k in 0..copies {
            text.push_str(&format!(
                "{} {} 0 1 0\n",
                clauses.len() + 1 + k,
                first.join(" ")
            ));
        }
        let hints: Vec<String> = hints.iter().map(ToString::to_string).collect();
        text.push_str(&format!(
            "{} 0 {} 0\n",
            clauses.len() + copies + 1,
            hints.join(" ")
        ));
        ledger.cas().put::<Certificate>(text.as_bytes()).unwrap()
    }

    pub(crate) fn run(root: PathBuf) {
        std::fs::create_dir_all(&root).unwrap();
        for dir in ["staging", "private"] {
            let path = root.join(dir);
            std::fs::create_dir_all(&path).unwrap();
            std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o700))
                .unwrap();
        }
        let signer = FileSigner::from_seed(&[0x51; 32]).unwrap();
        let attester = TestAttester {
            signer: FileSigner::from_seed(&[0x5a; 32]).unwrap(),
        };
        let mut ledger = Ledger::open(&root.join("ledger"), &signer).unwrap();
        assert!(
            ledger.head().unwrap().is_none(),
            "demo needs an empty directory"
        );
        let exe = std::fs::read("/proc/self/exe").unwrap();
        ledger
            .append(EventBody::Genesis {
                ledger_version: 1,
                kernel_binary: *ExternalDigest::of(&exe).as_bytes(),
                kernel_key: signer.key_id().unwrap(),
            })
            .unwrap();
        let registry = CheckerRegistry::new(signer.public_key().clone(), None).unwrap();
        registry.install(&mut ledger, &signer).unwrap();
        let handlers = CertificationHandlers::new(
            registry,
            Budget::new(STEPS, 64 * 1024 * 1024, 16 * 1024 * 1024, 5000),
        );

        // 1. Vocabulário.
        let symbols: Vec<Symbol> = ["chove", "rua_molhada", "sol"]
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
                .zip(["está chovendo", "a rua está molhada", "faz sol"])
                .map(|(s, gloss)| SymbolDecl {
                    signature: s.clone(),
                    grounding: Grounding::Glossed(Text::new(gloss).unwrap()),
                    gloss_pt: Text::new(gloss).unwrap(),
                })
                .collect(),
        )
        .unwrap();
        let hv = ledger
            .cas()
            .put::<Vocabulary>(&to_canon(&v).unwrap())
            .unwrap();
        let (chove, rua, sol) = (atom(&symbols[0]), atom(&symbols[1]), atom(&symbols[2]));

        // 2. Fatos de teste do atestador stub.
        let f1 = attester.attest(&mut ledger, hv, formula(chove.clone()));
        let f2 = attester.attest(
            &mut ledger,
            hv,
            formula([vec![FormulaNode::Implies], chove.clone(), rua.clone()].concat()),
        );

        // 3. Três propostas pelo transporte real.
        let propose = |ledger: &mut Ledger<'_, FileSigner>, text: &str, goal, cites| {
            let reply = exchange(
                &root,
                ledger,
                &handlers,
                Message::Propose(EventProposal {
                    text_pt: TaintedText::new(text.into()).unwrap(),
                    goal,
                    cites,
                    speculative_on: vec![],
                    vocabulary: hv,
                }),
            );
            Hash::<Obligation>::from_value(&ok(&reply)[1]).unwrap()
        };
        let certify = |ledger: &mut Ledger<'_, FileSigner>, o, c| {
            let reply = exchange(
                &root,
                ledger,
                &handlers,
                Message::Certify {
                    obligation: o,
                    format: Text::new(LRAT).unwrap(),
                    certificate: ObjectRef::Cas(c),
                },
            );
            let fields = ok(&reply);
            (
                fields[1].clone(),
                Hash::<Event>::from_value(&fields[2]).unwrap(),
            )
        };
        let oa = propose(
            &mut ledger,
            "a rua está molhada",
            formula(rua.clone()),
            vec![f1, f2],
        );
        let ca = proof(&ledger, oa, &v, 0);
        let (tag, verified) = certify(&mut ledger, oa, ca);
        assert_eq!(tag, Value::Unsigned(0), "A: Verified via LRAT");

        let ob = propose(&mut ledger, "faz sol", formula(sol.clone()), vec![f1]);
        let model = FiniteModel::new(
            ModelData {
                vocab: hash(&v).unwrap(),
                domains: vec![],
                consts: vec![],
                funcs: vec![],
                preds: vec![(0, vec![vec![]]), (1, vec![]), (2, vec![])],
            },
            &v,
            ModelLimits::default(),
        )
        .unwrap();
        let hm = ledger
            .cas()
            .put::<ModelObject>(&to_canon(&model).unwrap())
            .unwrap();
        let reply = exchange(
            &root,
            &mut ledger,
            &handlers,
            Message::Refute {
                obligation: ob,
                model: ObjectRef::Cas(hm),
            },
        );
        let fields = ok(&reply);
        assert_eq!(fields[1], Value::Unsigned(1), "B: Refuted via modelo");
        let refuted = Hash::<Event>::from_value(&fields[2]).unwrap();

        let oc = propose(
            &mut ledger,
            "está chovendo",
            formula(chove.clone()),
            vec![f1],
        );
        let cc = proof(&ledger, oc, &v, COPIES);
        let (tag, unknown) = certify(&mut ledger, oc, cc);
        assert_eq!(tag, Value::Unsigned(3), "C: Unknown por orçamento esgotado");

        // 4. O atestador retrata o fato `chove`; a cascata derruba A.
        let retraction = Retractions::new(signer.public_key().clone())
            .retract(
                &mut ledger,
                &attester.signer,
                f1,
                Text::new("fato de teste retirado").unwrap(),
            )
            .unwrap();
        let index = ledger.live_index().unwrap();
        let kind = |id| index.status(id).unwrap().unwrap().kind;
        assert_eq!(kind(verified), EntryKind::Invalidated);
        assert_eq!(kind(f2), EntryKind::Assumption);
        assert_eq!(retraction.invalidated.len(), 1);
        let head = index.head().unwrap();
        drop(index);
        let rust = hash_raw::<Checker>(&exe);
        println!(
            "DEMO_KERNEL_OK events={} verified={} refuted={} unknown={} retracted={} invalidated={} public_key={} rust_checker={}",
            head.seq + 1,
            hex(verified.as_bytes()),
            hex(refuted.as_bytes()),
            hex(unknown.as_bytes()),
            hex(retraction.retracted.as_bytes()),
            retraction.invalidated.len(),
            hex(signer.public_key().as_bytes()),
            hex(rust.as_bytes()),
        );
    }
}

fn main() {
    let dir = std::env::args_os().nth(1).expect("uso: demo_kernel DIR");
    #[cfg(target_os = "linux")]
    demo::run(std::path::PathBuf::from(dir));
    #[cfg(not(target_os = "linux"))]
    panic!("demo_kernel requer Linux ({dir:?})");
}
