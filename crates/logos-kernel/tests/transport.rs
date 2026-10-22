//! Socket Unix real, autenticação, limites e assinatura de eventos ao vivo.
#![cfg(target_os = "linux")]
mod support;
use logos_air::{Formula, FormulaNode, Limits, Vocabulary};
use logos_canon::{Canon, CanonDecode, Text, Value, from_canon, to_canon};
use logos_crypto::{FileSigner, Hash, Signer};
use logos_kernel::{
    TransportError,
    server::{ConnectionLimits, Handlers, Peer, ReadOnlyHandlers, Server},
    staging::Staging,
    wire::{Message, ObjectRef, Query, Request, encode_frame},
};
use logos_ledger::{Certificate, Event, EventBody, Ledger, LedgerHead, ModelObject};
use logos_obligation::Obligation;
use std::{
    io::{Read, Write},
    net::Shutdown,
    os::unix::net::UnixStream,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use support::*;

fn response(stream: &mut UnixStream) -> Value {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut prefix = [0; 4];
    stream.read_exact(&mut prefix).unwrap();
    let n = u32::from_be_bytes(prefix) as usize;
    assert!(n <= logos_kernel::wire::MAX_RESPONSE_BYTES);
    let mut payload = vec![0; n];
    stream.read_exact(&mut payload).unwrap();
    from_canon(&payload).unwrap()
}
fn values(v: &Value) -> &[Value] {
    let Value::Array(a) = v else {
        panic!("expected array")
    };
    a.items()
}
fn setup(temp: &Temp, limits: ConnectionLimits) -> (Server, Staging) {
    let uid = rustix::process::geteuid().as_raw();
    let source = temp.dir("source");
    let private = temp.dir("private");
    (
        Server::bind(&temp.0.join("socket"), uid, limits).unwrap(),
        Staging::open(&source, &private, uid, 4 * 1024 * 1024).unwrap(),
    )
}
fn head_request(id: u64) -> Vec<u8> {
    encode_frame(&Request {
        id,
        message: Message::Query(Query::Head),
    })
    .unwrap()
}
fn exchange(
    bytes: &[u8],
    count: usize,
    limits: ConnectionLimits,
) -> (Vec<Value>, Result<(), TransportError>, LedgerHead) {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let (server, staging) = setup(&temp, limits);
    let writer = Mutex::new(&mut ledger);
    let (mut client, stream) = UnixStream::pair().unwrap();
    let (replies, result) = std::thread::scope(|scope| {
        let worker =
            scope.spawn(|| server.serve_stream(stream, &writer, &ReadOnlyHandlers, &staging));
        client.write_all(bytes).unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let replies = (0..count)
            .map(|_| response(&mut client))
            .collect::<Vec<_>>();
        let result = worker.join().unwrap();
        (replies, result)
    });
    let head = writer.into_inner().unwrap().head().unwrap().unwrap();
    (replies, result, head)
}

#[test]
fn coalesced_frames_have_two_responses_and_no_logical_append() {
    let mut bytes = head_request(41);
    bytes.extend(head_request(42));
    let (r, result, head) = exchange(&bytes, 2, ConnectionLimits::default());
    result.unwrap();
    assert_eq!(head.seq, 0);
    for (i, v) in r.iter().enumerate() {
        assert_eq!(values(v)[1], Value::Unsigned(41 + i as u64));
        assert_eq!(values(v)[2], Value::Unsigned(1));
        assert_eq!(values(&values(v)[3])[0], Value::Unsigned(0));
    }
}

#[test]
fn unknown_type_extra_fields_and_partial_frames_fail_without_append() {
    for payload in [vec![0x84, 1, 0, 7, 0x80], vec![0x85, 1, 0, 5, 0x81, 0, 0]] {
        let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
        frame.extend(payload);
        let (replies, result, head) = exchange(&frame, 1, ConnectionLimits::default());
        assert!(result.is_err());
        assert_eq!(head.seq, 0);
        assert_eq!(values(&replies[0])[2], Value::Unsigned(0));
    }
    let (r, result, head) = exchange(&[0, 0], 0, ConnectionLimits::default());
    assert!(r.is_empty());
    assert!(
        matches!(result,Err(TransportError::Io(e)) if e.kind()==std::io::ErrorKind::UnexpectedEof)
    );
    assert_eq!(head.seq, 0);
    let (_, result, _) = exchange(&u32::MAX.to_be_bytes(), 0, ConnectionLimits::default());
    assert!(matches!(result, Err(TransportError::Limit(_))));
}

#[test]
fn frame_rate_total_bytes_and_frame_count_are_enforced_before_dispatch() {
    let frame = head_request(1);
    let mut three = frame.clone();
    three.extend(&frame);
    three.extend(&frame);
    for limits in [
        ConnectionLimits {
            frames_per_second: 2,
            ..Default::default()
        },
        ConnectionLimits {
            frames: 2,
            ..Default::default()
        },
    ] {
        let (_, result, head) = exchange(&three, 2, limits);
        assert!(matches!(result, Err(TransportError::Limit(_))));
        assert_eq!(head.seq, 0);
    }
    let (_, result, head) = exchange(
        &frame,
        0,
        ConnectionLimits {
            bytes: (frame.len() - 1) as u64,
            ..Default::default()
        },
    );
    assert!(matches!(result, Err(TransportError::Limit(_))));
    assert_eq!(head.seq, 0);
}

#[test]
fn peer_uid_is_authenticated_before_reading_any_bytes() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let uid = rustix::process::geteuid().as_raw();
    let server = Server::bind(
        &temp.0.join("socket"),
        uid.wrapping_add(1),
        Default::default(),
    )
    .unwrap();
    let staging = Staging::open(&temp.dir("source"), &temp.dir("private"), uid, 100).unwrap();
    let writer = Mutex::new(&mut ledger);
    let (_client, stream) = UnixStream::pair().unwrap();
    assert!(matches!(
        server.serve_stream(stream, &writer, &ReadOnlyHandlers, &staging),
        Err(TransportError::Unauthorized)
    ));
    assert_eq!(writer.lock().unwrap().head().unwrap().unwrap().seq, 0);
}

#[test]
fn absolute_deadline_rejects_a_slow_trickle_that_would_renew_per_read_timeouts() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let (server, staging) = setup(
        &temp,
        ConnectionLimits {
            frame_timeout: Duration::from_millis(100),
            ..Default::default()
        },
    );
    let writer = Mutex::new(&mut ledger);
    let (mut client, stream) = UnixStream::pair().unwrap();
    let frame = head_request(1);
    let result = std::thread::scope(|scope| {
        let worker =
            scope.spawn(|| server.serve_stream(stream, &writer, &ReadOnlyHandlers, &staging));
        let feeder = scope.spawn(move || {
            for byte in &frame[..4] {
                if client.write_all(&[*byte]).is_err() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(40));
            }
            if client.write_all(&frame[4..]).is_ok() {
                client.shutdown(Shutdown::Write).unwrap();
                let _ = response(&mut client);
            }
        });
        let result = worker.join().unwrap();
        feeder.join().unwrap();
        result
    });
    assert!(
        matches!(result,Err(TransportError::Io(e)) if matches!(e.kind(),std::io::ErrorKind::TimedOut|std::io::ErrorKind::WouldBlock))
    );
}

#[test]
fn unimplemented_mutation_is_an_error_and_never_emits_verified() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../spec/vectors/kernel-protocol/vectors.json"
    ))
    .unwrap();
    let hex = vectors["vectors"][0]["frame_hex"].as_str().unwrap();
    let frame: Vec<_> = hex
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect();
    let (r, result, head) = exchange(&frame, 1, Default::default());
    result.unwrap();
    assert_eq!(values(&r[0])[2..], [Value::Unsigned(0), Value::Unsigned(3)]);
    assert_eq!(head.seq, 0);
}

// Instrumentação stateless: cada handler retorna sua própria tag, sem claims.
struct Echo;
impl Handlers<FileSigner> for Echo {
    fn propose(
        &self,
        _: &mut Ledger<'_, FileSigner>,
        p: &Peer,
        _: logos_ledger::EventProposal,
    ) -> Result<Value, TransportError> {
        assert_eq!(p.uid(), rustix::process::geteuid().as_raw());
        assert_eq!(p.pid(), std::process::id());
        Ok(Value::Unsigned(0))
    }
    fn certify(
        &self,
        _: &mut Ledger<'_, FileSigner>,
        _: &Peer,
        _: Hash<Obligation>,
        _: Text,
        _: Hash<Certificate>,
    ) -> Result<Value, TransportError> {
        Ok(Value::Unsigned(1))
    }
    fn refute(
        &self,
        _: &mut Ledger<'_, FileSigner>,
        _: &Peer,
        _: Hash<Obligation>,
        _: Hash<ModelObject>,
    ) -> Result<Value, TransportError> {
        Ok(Value::Unsigned(2))
    }
    fn witness(
        &self,
        _: &mut Ledger<'_, FileSigner>,
        _: &Peer,
        _: Hash<Obligation>,
        _: Hash<ModelObject>,
    ) -> Result<Value, TransportError> {
        Ok(Value::Unsigned(3))
    }
    fn assume(
        &self,
        _: &mut Ledger<'_, FileSigner>,
        _: &Peer,
        _: Hash<Vocabulary>,
        _: Formula,
        _: Text,
    ) -> Result<Value, TransportError> {
        Ok(Value::Unsigned(4))
    }
}

#[test]
fn each_mutation_has_its_own_handler_after_typed_object_import() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let large = vec![0x52; 3 * 1024 * 1024];
    let cert = logos_crypto::hash_raw::<Certificate>(&large);
    let model = ledger.cas().put::<ModelObject>(b"model candidate").unwrap();
    let (server, staging) = setup(&temp, Default::default());
    std::fs::write(temp.0.join("source").join("a".repeat(64)), &large).unwrap();
    let writer = Mutex::new(&mut ledger);
    let (mut client, stream) = UnixStream::pair().unwrap();
    let formula = Formula::new(vec![FormulaNode::True], Limits::default()).unwrap();
    let vocabulary = Hash::from_bytes([1; 32]);
    let o = Hash::from_bytes([2; 32]);
    let messages = [
        Message::Propose(logos_ledger::EventProposal {
            text_pt: logos_ledger::TaintedText::new("candidate".into()).unwrap(),
            goal: formula.clone(),
            cites: vec![],
            speculative_on: vec![],
            vocabulary,
        }),
        Message::Certify {
            obligation: o,
            format: Text::new("candidate").unwrap(),
            certificate: ObjectRef::Staged {
                name: Text::new("a".repeat(64)).unwrap(),
                hash: cert,
            },
        },
        Message::Refute {
            obligation: o,
            model: ObjectRef::Cas(model),
        },
        Message::Witness {
            obligation: o,
            model: ObjectRef::Cas(model),
        },
        Message::Assume {
            vocabulary,
            formula,
            scope: Text::new("local").unwrap(),
        },
    ];
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| server.serve_stream(stream, &writer, &Echo, &staging));
        for (tag, message) in messages.into_iter().enumerate() {
            let frame = encode_frame(&Request {
                id: tag as u64,
                message,
            })
            .unwrap();
            for byte in frame {
                client.write_all(&[byte]).unwrap();
            }
            let r = response(&mut client);
            assert_eq!(values(&r)[2], Value::Unsigned(1));
            assert_eq!(values(&r)[3], Value::Unsigned(tag as u64));
        }
        client.shutdown(Shutdown::Write).unwrap();
        worker.join().unwrap().unwrap();
    });
    assert_eq!(writer.lock().unwrap().head().unwrap().unwrap().seq, 0);
    assert_eq!(
        writer.lock().unwrap().cas().get(&cert).unwrap(),
        Some(large)
    );
}

// Handler de fixture que acrescenta um pressuposto existente no catálogo;
// não implementa a autorização de ASSUME5.7.
struct AppendFixture {
    author: logos_crypto::KeyId,
}
impl Handlers<FileSigner> for AppendFixture {
    fn assume(
        &self,
        ledger: &mut Ledger<'_, FileSigner>,
        _: &Peer,
        vocabulary: Hash<Vocabulary>,
        formula: Formula,
        scope: Text,
    ) -> Result<Value, TransportError> {
        let event = ledger.append(EventBody::AssumptionDeclared {
            vocabulary,
            formula,
            scope,
            author: self.author,
        })?;
        Ok(event.seq.to_value()?)
    }
}

#[test]
fn live_subscription_receives_a_new_committed_event_from_another_connection() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let vocab = Vocabulary::new(1, vec![], vec![]).unwrap();
    let hv = ledger.cas().put(&to_canon(&vocab).unwrap()).unwrap();
    let (server, staging) = setup(
        &temp,
        ConnectionLimits {
            subscription_timeout: Duration::from_secs(2),
            ..Default::default()
        },
    );
    let stop = AtomicBool::new(false);
    let handler = AppendFixture {
        author: signer.key_id().unwrap(),
    };
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| server.serve(&mut ledger, &handler, &staging, &stop));
        let mut subscriber = UnixStream::connect(temp.0.join("socket")).unwrap();
        subscriber
            .write_all(
                &encode_frame(&Request {
                    id: 8,
                    message: Message::Subscribe {
                        from_seq: 0,
                        max_events: 2,
                    },
                })
                .unwrap(),
            )
            .unwrap();
        let first = response(&mut subscriber);
        let first_packet = values(&values(&first)[3]);
        let genesis = Event::from_value(&first_packet[1]).unwrap();
        assert_eq!(genesis.seq, 0); // assinatura já está ativa antes do novo append
        let mut producer = UnixStream::connect(temp.0.join("socket")).unwrap();
        let formula = Formula::new(vec![FormulaNode::True], Limits::default()).unwrap();
        producer
            .write_all(
                &encode_frame(&Request {
                    id: 9,
                    message: Message::Assume {
                        vocabulary: hv,
                        formula,
                        scope: Text::new("fixture").unwrap(),
                    },
                })
                .unwrap(),
            )
            .unwrap();
        assert_eq!(values(&response(&mut producer))[2], Value::Unsigned(1));
        let event_reply = response(&mut subscriber);
        let packet = values(&values(&event_reply)[3]);
        assert_eq!(packet[0], Value::Unsigned(0));
        let event = Event::from_value(&packet[1]).unwrap();
        assert_eq!(event.seq, 1);
        event.verify(signer.public_key()).unwrap();
        producer
            .write_all(
                &encode_frame(&Request {
                    id: 10,
                    message: Message::Query(Query::Status(logos_crypto::hash(&event).unwrap())),
                })
                .unwrap(),
            )
            .unwrap();
        let status = response(&mut producer);
        assert_eq!(
            values(&values(&status)[3]),
            [Value::Unsigned(3), Value::Unsigned(0), Value::Bool(false)]
        );
        let end = response(&mut subscriber);
        assert_eq!(
            values(&values(&end)[3]),
            [Value::Unsigned(1), Value::Unsigned(2)]
        );
        drop(producer);
        drop(subscriber);
        stop.store(true, Ordering::Release);
        worker.join().unwrap().unwrap();
    });
    assert_eq!(ledger.events_from(0, 256).unwrap().len(), 2);
    assert_eq!(ledger.events_from(1, 1).unwrap()[0].seq, 1);
    assert!(ledger.events_from(u64::MAX, 1).unwrap().is_empty());
    assert!(ledger.events_from(0, 257).is_err());
}

struct SlowPage;
impl Handlers<FileSigner> for SlowPage {
    fn subscribe(
        &self,
        ledger: &mut Ledger<'_, FileSigner>,
        _: &Peer,
        cursor: u64,
        max: usize,
    ) -> Result<Vec<Event>, TransportError> {
        let events = ledger.events_from(cursor, max)?;
        std::thread::sleep(Duration::from_millis(100));
        Ok(events)
    }
}

struct ReadyPage(Vec<Event>);
impl Handlers<FileSigner> for ReadyPage {
    fn subscribe(
        &self,
        _: &mut Ledger<'_, FileSigner>,
        _: &Peer,
        _: u64,
        _: usize,
    ) -> Result<Vec<Event>, TransportError> {
        Ok(self.0.clone())
    }
}

#[test]
fn subscription_deadline_is_checked_after_fetch_before_sending_the_page() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let (server, staging) = setup(
        &temp,
        ConnectionLimits {
            subscription_timeout: Duration::from_millis(20),
            ..Default::default()
        },
    );
    let writer = Mutex::new(&mut ledger);
    let (mut client, stream) = UnixStream::pair().unwrap();
    client
        .write_all(
            &encode_frame(&Request {
                id: 1,
                message: Message::Subscribe {
                    from_seq: 0,
                    max_events: 1,
                },
            })
            .unwrap(),
        )
        .unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| server.serve_stream(stream, &writer, &SlowPage, &staging));
        let end = response(&mut client);
        worker.join().unwrap().unwrap();
        assert_eq!(
            values(&values(&end)[3]),
            [Value::Unsigned(1), Value::Unsigned(0)]
        );
    });
}

#[test]
fn subscription_deadline_caps_a_blocked_frame_and_does_not_hold_the_writer() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let vocabulary = Vocabulary::new(1, vec![], vec![]).unwrap();
    let hv = ledger.cas().put(&to_canon(&vocabulary).unwrap()).unwrap();
    ledger
        .append(EventBody::Proposed {
            proposal: logos_ledger::EventProposal {
                text_pt: logos_ledger::TaintedText::new("x".repeat(64 * 1024)).unwrap(),
                goal: Formula::new(vec![FormulaNode::True], Limits::default()).unwrap(),
                cites: vec![],
                speculative_on: vec![],
                vocabulary: hv,
            },
            author: signer.key_id().unwrap(),
        })
        .unwrap();
    let ready = ReadyPage(ledger.events_from(1, 1).unwrap());
    let (server, staging) = setup(
        &temp,
        ConnectionLimits {
            subscription_timeout: Duration::from_secs(1),
            frame_timeout: Duration::from_secs(5),
            ..Default::default()
        },
    );
    let writer = Mutex::new(&mut ledger);
    let (mut client, stream) = UnixStream::pair().unwrap();
    rustix::net::sockopt::set_socket_send_buffer_size(&stream, 4096).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    client
        .write_all(
            &encode_frame(&Request {
                id: 1,
                message: Message::Subscribe {
                    from_seq: 1,
                    max_events: 1,
                },
            })
            .unwrap(),
        )
        .unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    let (done, received) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            let result = server.serve_stream(stream, &writer, &ready, &staging);
            done.send(()).unwrap();
            result
        });
        let mut prefix = [0; 4];
        client.read_exact(&mut prefix).unwrap(); // frame iniciou, mas cliente não lê o payload
        assert!(
            u32::from_be_bytes(prefix) > 4096,
            "a fixture precisa bloquear um frame de evento, não o cursor final"
        );
        assert!(writer.try_lock().is_ok());
        let within_subscription = received.recv_timeout(Duration::from_secs(3)).is_ok();
        drop(client); // também libera o worker defeituoso antes da asserção
        let result = worker.join().unwrap();
        assert!(
            within_subscription,
            "frame_timeout não pode renovar o prazo da assinatura"
        );
        assert!(
            matches!(&result, Err(TransportError::Io(e)) if matches!(e.kind(), std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock)),
            "unexpected subscription result: {result:?}"
        );
    });
}

#[test]
fn status_reuses_the_private_projection_without_reopening_the_disk_cache() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let (server, staging) = setup(&temp, Default::default());
    let writer = Mutex::new(&mut ledger);
    let (mut client, stream) = UnixStream::pair().unwrap();
    std::thread::scope(|scope| {
        let worker =
            scope.spawn(|| server.serve_stream(stream, &writer, &ReadOnlyHandlers, &staging));
        for id in 1..=3 {
            client
                .write_all(
                    &encode_frame(&Request {
                        id,
                        message: Message::Query(Query::Status(Hash::from_bytes([0xff; 32]))),
                    })
                    .unwrap(),
                )
                .unwrap();
            let reply = response(&mut client);
            assert_eq!(values(&reply)[2], Value::Unsigned(1));
            assert!(values(&values(&reply)[3]).is_empty());
            if id == 1 {
                let cache = temp.0.join("ledger/index.sqlite3");
                std::fs::remove_file(&cache).unwrap();
                std::fs::create_dir(&cache).unwrap();
            }
        }
        client.shutdown(Shutdown::Write).unwrap();
        worker.join().unwrap().unwrap();
    });
}

#[test]
fn expired_subscription_does_not_wait_for_a_final_cursor_on_a_full_socket() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0.join("ledger"), &signer).unwrap();
    genesis(&mut ledger, &signer);
    let (server, staging) = setup(
        &temp,
        ConnectionLimits {
            subscription_timeout: Duration::from_millis(20),
            frame_timeout: Duration::from_secs(3),
            ..Default::default()
        },
    );
    let writer = Mutex::new(&mut ledger);
    let (mut client, mut stream) = UnixStream::pair().unwrap();
    rustix::net::sockopt::set_socket_send_buffer_size(&stream, 4096).unwrap();
    stream.set_nonblocking(true).unwrap();
    loop {
        match stream.write(&[0; 4096]) {
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) => panic!("fixture: {e}"),
        }
    }
    stream.set_nonblocking(false).unwrap();
    client
        .write_all(
            &encode_frame(&Request {
                id: 1,
                message: Message::Subscribe {
                    from_seq: 0,
                    max_events: 1,
                },
            })
            .unwrap(),
        )
        .unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    let (done, received) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            let result = server.serve_stream(stream, &writer, &SlowPage, &staging);
            done.send(()).unwrap();
            result
        });
        let timely = received.recv_timeout(Duration::from_secs(1)).is_ok();
        drop(client);
        let result = worker.join().unwrap();
        assert!(timely, "cursor final não renova a assinatura expirada");
        assert!(
            matches!(result, Err(TransportError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock)
        );
    });
}

#[test]
fn binding_never_removes_existing_paths_and_rejects_writable_parent() {
    let temp = Temp::new();
    let path = temp.0.join("socket");
    std::fs::write(&path, b"existing").unwrap();
    let uid = rustix::process::geteuid().as_raw();
    assert!(Server::bind(&path, uid, Default::default()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"existing");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&temp.0, std::fs::Permissions::from_mode(0o777)).unwrap();
    assert!(Server::bind(&temp.0.join("other"), uid, Default::default()).is_err());
}

#[test]
fn binary_serves_propose_certify_refute_and_persists_its_checker_identity() {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    struct Process(std::process::Child);
    impl Drop for Process {
        fn drop(&mut self) {
            self.0.kill().unwrap();
            self.0.wait().unwrap();
        }
    }
    let temp = Temp::new();
    let source = temp.dir("source");
    let private = temp.dir("private");
    let key = temp.0.join("fixture-key");
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&key)
        .unwrap();
    file.write_all("52".repeat(32).as_bytes()).unwrap();
    drop(file);
    let executable = env!("CARGO_BIN_EXE_logos-kernel");
    let socket = temp.0.join("socket");
    let ledger_path = temp.dir("ledger");
    let cas = logos_ledger::Cas::open(ledger_path.join("cas"), Default::default()).unwrap();
    let vocabulary = Vocabulary::new(1, vec![], vec![]).unwrap();
    let hv = cas.put(&to_canon(&vocabulary).unwrap()).unwrap();
    let mut process = Process(
        std::process::Command::new(executable)
            .arg(&socket)
            .arg(&ledger_path)
            .arg(&key)
            .arg(&source)
            .arg(&private)
            .arg(rustix::process::geteuid().as_raw().to_string())
            .spawn()
            .unwrap(),
    );
    let start = std::time::Instant::now();
    let mut client = loop {
        if let Ok(stream) = UnixStream::connect(&socket) {
            break stream;
        }
        assert!(process.0.try_wait().unwrap().is_none());
        // Startup debug includes executable hashing and checker registration, not an RPC SLA.
        // Keep the fixture bounded without changing any protocol/checker deadline.
        assert!(start.elapsed() < Duration::from_secs(120));
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(
        std::fs::metadata(&socket).unwrap().permissions().mode() & 0o777,
        0o660
    );
    client.write_all(&head_request(17)).unwrap();
    let reply = response(&mut client);
    assert_eq!(values(&reply)[1], Value::Unsigned(17));
    assert_eq!(values(&values(&reply)[3])[0], Value::Unsigned(2));
    client
        .write_all(
            &encode_frame(&Request {
                id: 18,
                message: Message::Propose(logos_ledger::EventProposal {
                    text_pt: logos_ledger::TaintedText::new("real binary proposal".into()).unwrap(),
                    goal: Formula::new(vec![FormulaNode::True], Limits::default()).unwrap(),
                    cites: vec![],
                    speculative_on: vec![],
                    vocabulary: hv,
                }),
            })
            .unwrap(),
        )
        .unwrap();
    let proposed = response(&mut client);
    assert_eq!(
        &values(&proposed)[..3],
        &[Value::Unsigned(1), Value::Unsigned(18), Value::Unsigned(1)]
    );
    let [Value::Unsigned(1), Value::Bytes(h), Value::Bytes(bytes)] = values(&values(&proposed)[3])
    else {
        panic!("propose-result/v1")
    };
    let ho = logos_crypto::hash_raw::<Obligation>(bytes);
    assert_eq!(h.as_slice(), ho.as_bytes());
    assert_eq!(cas.get(&ho).unwrap().as_deref(), Some(bytes.as_slice()));
    let ground = logos_ground::ground_for_check(
        bytes,
        ho,
        &vocabulary,
        Default::default(),
        Default::default(),
    )
    .unwrap();
    let (cnf, _) = logos_ground::clausify(&ground).unwrap();
    let (a, b) = cnf
        .clauses()
        .iter()
        .enumerate()
        .find_map(|(i, c)| {
            if c.len() != 1 {
                return None;
            }
            cnf.clauses()
                .iter()
                .position(|d| d == &[-c[0]])
                .map(|j| (i + 1, j + 1))
        })
        .unwrap();
    let proof = format!("{} 0 {a} {b} 0\n", cnf.clauses().len() + 1);
    let certificate = logos_crypto::hash_raw::<Certificate>(proof.as_bytes());
    let name = "ab".repeat(32);
    std::fs::write(source.join(&name), proof).unwrap();
    client
        .write_all(
            &encode_frame(&Request {
                id: 19,
                message: Message::Certify {
                    obligation: ho,
                    format: Text::new(logos_kernel::registry::LRAT).unwrap(),
                    certificate: ObjectRef::Staged {
                        name: Text::new(name).unwrap(),
                        hash: certificate,
                    },
                },
            })
            .unwrap(),
        )
        .unwrap();
    let certified = response(&mut client);
    assert_eq!(values(&certified)[2], Value::Unsigned(1));
    assert_eq!(values(&values(&certified)[3])[1], Value::Unsigned(0));
    client
        .write_all(
            &encode_frame(&Request {
                id: 20,
                message: Message::Propose(logos_ledger::EventProposal {
                    text_pt: logos_ledger::TaintedText::new("refutation".into()).unwrap(),
                    goal: Formula::new(vec![FormulaNode::False], Limits::default()).unwrap(),
                    cites: vec![],
                    speculative_on: vec![],
                    vocabulary: hv,
                }),
            })
            .unwrap(),
        )
        .unwrap();
    let second = response(&mut client);
    let false_o = Hash::<Obligation>::from_value(&values(&values(&second)[3])[1]).unwrap();
    let model = logos_model::FiniteModel::new(
        logos_model::ModelData {
            vocab: hv,
            domains: vec![],
            consts: vec![],
            funcs: vec![],
            preds: vec![],
        },
        &vocabulary,
        Default::default(),
    )
    .unwrap();
    let model_bytes = to_canon(&model).unwrap();
    let model = logos_crypto::hash_raw::<ModelObject>(&model_bytes);
    let name = "cd".repeat(32);
    std::fs::write(source.join(&name), model_bytes).unwrap();
    client
        .write_all(
            &encode_frame(&Request {
                id: 21,
                message: Message::Refute {
                    obligation: false_o,
                    model: ObjectRef::Staged {
                        name: Text::new(name).unwrap(),
                        hash: model,
                    },
                },
            })
            .unwrap(),
        )
        .unwrap();
    let refuted = response(&mut client);
    assert_eq!(values(&refuted)[2], Value::Unsigned(1));
    assert_eq!(values(&values(&refuted)[3])[1], Value::Unsigned(1));
    drop(client);
    drop(process);
    let s = signer();
    let ledger = Ledger::open(&ledger_path, &s).unwrap();
    let events = ledger.events_from(0, 256).unwrap();
    assert_eq!(events.len(), 9);
    assert!(matches!(
        events[1].body,
        EventBody::CheckerRegistered { .. }
    ));
    assert!(matches!(
        events[2].body,
        EventBody::CheckerRegistered { .. }
    ));
    assert!(matches!(events[3].body, EventBody::Proposed { .. }));
    assert!(
        matches!(events[4].body, EventBody::ObligationCreated { obligation, .. } if obligation == ho)
    );
    assert!(
        matches!(&events[5].body,EventBody::Verified{check,..} if check.obligation==ho && check.certificate==certificate && check.checker==logos_crypto::hash_raw::<logos_ledger::Checker>(&std::fs::read(executable).unwrap()))
    );
    assert!(
        matches!(&events[8].body,EventBody::Refuted{check,model:m} if check.obligation==false_o && *m==model)
    );
    let EventBody::Genesis {
        kernel_binary,
        kernel_key,
        ..
    } = &events[0].body
    else {
        panic!("expected genesis")
    };
    assert_eq!(kernel_key, &s.key_id().unwrap());
    assert_eq!(
        kernel_binary,
        logos_crypto::ExternalDigest::of(&std::fs::read(executable).unwrap()).as_bytes()
    );
}
