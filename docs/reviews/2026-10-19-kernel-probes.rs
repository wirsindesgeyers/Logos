// Diagnóstico descartável da revisão de 2026-10-19; fora do runtime e da TCB.
// A chave fixa abaixo é fixture pública. Ver 2026-10-19-kernel.md para execução.
// Exit 0 indica execução completa, não conformidade dos comportamentos observados.
use logos_air::{Formula, FormulaNode, Limits, Vocabulary};
use logos_canon::{Text, Value, from_canon, to_canon};
use logos_crypto::{FileSigner, Hash, Signer, hash_raw};
use logos_kernel::{
    propose::ProposalHandlers,
    server::{ConnectionLimits, Server},
    staging::Staging,
    wire::{self, Message, ObjectRef, Query, Request},
};
use logos_ledger::{Certificate, EventBody, EventProposal, Ledger, TaintedText};
use std::{
    error::Error,
    fs,
    io::{Read, Write},
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::UnixStream,
    },
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
fn dir(path: &Path) -> Result<PathBuf> {
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(path.to_path_buf())
}
fn request(stream: &mut UnixStream, id: u64, message: Message) -> Result<()> {
    stream.write_all(&wire::encode_frame(&Request { id, message })?)?;
    Ok(())
}
fn response(stream: &mut UnixStream) -> Result<Value> {
    let mut size = [0u8; 4];
    stream.read_exact(&mut size)?;
    let n = u32::from_be_bytes(size) as usize;
    assert!(n < wire::MAX_RESPONSE_BYTES);
    let mut bytes = vec![0; n];
    stream.read_exact(&mut bytes)?;
    Ok(from_canon(&bytes)?)
}
fn item(v: &Value, n: usize) -> &Value {
    let Value::Array(v) = v else { panic!("array") };
    &v.items()[n]
}
fn main() -> Result<()> {
    let mode = std::env::args().nth(1).unwrap_or("staging".into());
    let count: usize = std::env::args().nth(2).unwrap_or("32".into()).parse()?;
    let root = dir(&PathBuf::from(format!(
        "/tmp/logos-review-{}-{}",
        mode,
        std::process::id()
    )))?;
    let source = dir(&root.join("source"))?;
    let private = dir(&root.join("private"))?;
    let uid = fs::metadata(&root)?.uid();
    let staging = Staging::open(&source, &private, uid, 64 * 1024 * 1024)?;
    let signer = FileSigner::from_seed(&[0x79; 32])?;
    let mut ledger = Ledger::open(&root.join("ledger"), &signer)?;
    ledger.append(EventBody::Genesis {
        ledger_version: 1,
        kernel_binary: [0x79; 32],
        kernel_key: signer.key_id()?,
    })?;
    if mode == "staging" {
        let reference = ObjectRef::<Certificate>::Staged {
            name: Text::new("a".repeat(64))?,
            hash: hash_raw(b"payload"),
        };
        fs::write(source.join("a".repeat(64)), b"payload")?;
        let mut quota = 1024;
        let valid = staging.import(&reference, ledger.cas(), &mut quota).is_ok();
        assert!(valid);
        assert_eq!(fs::read_dir(&private)?.count(), 0);
        let mut returned = 0;
        for _ in 0..count {
            let bad = dir(&source.join("a".repeat(64)))?;
            fs::write(bad.join("child"), b"private orphan")?;
            let error = staging
                .import(&reference, ledger.cas(), &mut quota)
                .unwrap_err();
            println!("rejected={error}");
            if bad.is_dir() {
                assert_eq!(fs::read(bad.join("child"))?, b"private orphan");
                returned += 1;
                fs::remove_dir_all(bad)?; // o próprio agente limpa sua fixture devolvida
            }
        }
        println!(
            "STAGING directory_attempts={count} returned={returned} private_entries={} quota={quota} root={}",
            fs::read_dir(&private)?.count(),
            root.display()
        );
        return Ok(());
    }
    if mode == "subscribe" {
        let vocabulary = Vocabulary::new(1, vec![], vec![])?;
        let hv = ledger.cas().put::<Vocabulary>(&to_canon(&vocabulary)?)?;
        let goal = Formula::new(vec![FormulaNode::True], Limits::default())?;
        for _ in 0..count {
            ledger.append(EventBody::Proposed {
                proposal: EventProposal {
                    text_pt: TaintedText::new("x".repeat(64 * 1024))?,
                    goal: goal.clone(),
                    cites: vec![],
                    speculative_on: vec![],
                    vocabulary: hv,
                },
                author: signer.key_id()?,
            })?;
        }
    } else if mode == "query" || mode == "propose" {
        for epoch in 1..count as u64 {
            ledger.append(EventBody::EpochAdvanced {
                new_epoch: epoch,
                reason: Text::new("review fixture")?,
                policy_root: None,
                vocab: None,
            })?;
        }
    } else {
        return Err("unknown mode".into());
    }
    let limits = ConnectionLimits {
        subscription_timeout: Duration::from_millis(20),
        frame_timeout: Duration::from_secs(3),
        ..ConnectionLimits::default()
    };
    let server = Server::bind(&root.join("kernel.sock"), uid, limits)?;
    let (mut client, stream) = UnixStream::pair()?;
    client.set_read_timeout(Some(Duration::from_secs(30)))?;
    let is_subscription = mode == "subscribe";
    let proposal = if mode == "propose" {
        let vocabulary = Vocabulary::new(1, vec![], vec![])?;
        Some(EventProposal {
            vocabulary: ledger.cas().put::<Vocabulary>(&to_canon(&vocabulary)?)?,
            goal: Formula::new(vec![FormulaNode::True], Limits::default())?,
            text_pt: TaintedText::new("review proposal".into())?,
            cites: vec![],
            speculative_on: vec![],
        })
    } else {
        None
    };
    let client = std::thread::spawn(move || -> Result<Vec<f64>> {
        if is_subscription {
            request(
                &mut client,
                1,
                Message::Subscribe {
                    from_seq: 1,
                    max_events: count as u64,
                },
            )?;
            client.shutdown(std::net::Shutdown::Write)?;
            let mut received = 0;
            let started = Instant::now();
            loop {
                let v = match response(&mut client) {
                    Ok(value) => value,
                    Err(error)
                        if error.downcast_ref::<std::io::Error>().is_some_and(|e| {
                            matches!(
                                e.kind(),
                                std::io::ErrorKind::UnexpectedEof
                                    | std::io::ErrorKind::ConnectionReset
                            )
                        }) =>
                    {
                        println!("SUBSCRIBE connection_closed={error}");
                        break;
                    }
                    Err(error) => return Err(error),
                };
                assert_eq!(item(&v, 2), &Value::Unsigned(1));
                let payload = item(&v, 3);
                if item(payload, 0) == &Value::Unsigned(1) {
                    break;
                }
                received += 1;
                std::thread::sleep(Duration::from_millis(30));
            }
            println!(
                "SUBSCRIBE received={received} configured_timeout_ms=20 client_ms={:.2}",
                started.elapsed().as_secs_f64() * 1000.0
            );
            return Ok(vec![]);
        }
        let mut times = vec![];
        let messages = if let Some(proposal) = proposal {
            vec![
                (1, Message::Query(Query::Head)),
                (2, Message::Propose(proposal.clone())),
                (3, Message::Propose(proposal.clone())),
                (4, Message::Propose(proposal)),
                (5, Message::Query(Query::Head)),
            ]
        } else {
            vec![
                (1, Message::Query(Query::Head)),
                (
                    2,
                    Message::Query(Query::Status(Hash::from_bytes([0x88; 32]))),
                ),
                (
                    3,
                    Message::Query(Query::Status(Hash::from_bytes([0x88; 32]))),
                ),
                (
                    4,
                    Message::Query(Query::Status(Hash::from_bytes([0x88; 32]))),
                ),
                (5, Message::Query(Query::Head)),
            ]
        };
        for (id, message) in messages {
            let started = Instant::now();
            request(&mut client, id, message)?;
            let v = response(&mut client)?;
            assert_eq!(item(&v, 2), &Value::Unsigned(1));
            times.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        client.shutdown(std::net::Shutdown::Write)?;
        Ok(times)
    });
    let started = Instant::now();
    let writer = Mutex::new(&mut ledger);
    let result = server.serve_stream(stream, &writer, &ProposalHandlers, &staging);
    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
    let times = client.join().unwrap()?;
    if is_subscription {
        match result {
            Ok(()) => (),
            Err(logos_kernel::TransportError::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                println!("SUBSCRIBE server_timeout={error}")
            }
            Err(error) => return Err(error.into()),
        }
    } else {
        result?;
    }
    println!(
        "TRANSPORT mode={mode} events={count} server_ms={elapsed:.2} request_ms={times:?} root={}",
        root.display()
    );
    Ok(())
}
