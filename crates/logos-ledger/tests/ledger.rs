//! Appender real: bytes persistidos, recuperação estrita e falhas sem promoção.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use std::cell::Cell;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use logos_canon::to_canon;
use logos_crypto::{CryptoError, FileSigner, Hash, PublicKey, SigDomain, Signature, Signer, hash};
use logos_ledger::{Event, Ledger, LedgerError, LedgerHead};
use rusqlite::Connection;
use support::{Temp, epoch, genesis, ordinary, signer, unhex};

fn populated(temp: &Temp, signer: &FileSigner) -> Vec<Event> {
    let mut ledger = Ledger::open(&temp.0, signer).unwrap();
    vec![
        ledger.append(genesis(signer)).unwrap(),
        ledger.append(ordinary(1)).unwrap(),
        ledger.append(epoch(3)).unwrap(),
        ledger.append(ordinary(2)).unwrap(),
    ]
}

#[test]
fn persists_exact_signed_bytes_and_resumes_head() {
    let temp = Temp::new();
    let signer = signer();
    let events = populated(&temp, &signer);
    let mut previous = None;
    let connection = Connection::open(temp.database()).unwrap();
    for (seq, event) in events.iter().enumerate() {
        assert_eq!(event.seq, seq as u64);
        assert_eq!(event.prev, previous);
        assert_eq!(event.epoch, if seq < 2 { 0 } else { 3 });
        event.verify(signer.public_key()).unwrap();
        let bytes: Vec<u8> = connection
            .query_row("SELECT wire FROM events WHERE seq=?1", [seq as i64], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(bytes, to_canon(event).unwrap());
        previous = Some(hash(event).unwrap());
    }
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    assert_eq!(
        ledger.head().unwrap(),
        Some(LedgerHead {
            seq: 3,
            epoch: 3,
            hash: previous.unwrap()
        })
    );
    let appended = ledger.append(ordinary(3)).unwrap();
    assert_eq!(appended.seq, 4);
    assert_eq!(appended.prev, previous);
    assert_eq!(appended.epoch, 3);
}

#[test]
fn invalid_body_does_not_consume_sequence_and_epoch_is_derived() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    assert_eq!(ledger.head().unwrap(), None);
    assert!(ledger.append(ordinary(1)).is_err());
    let first = ledger.append(genesis(&signer)).unwrap();
    let head = ledger.head().unwrap();
    assert!(ledger.append(genesis(&signer)).is_err());
    assert!(ledger.append(epoch(0)).is_err());
    assert_eq!(ledger.head().unwrap(), head);
    let advanced = ledger.append(epoch(7)).unwrap();
    assert_eq!(advanced.prev, Some(hash(&first).unwrap()));
    assert_eq!(advanced.epoch, 7);
    assert!(ledger.append(epoch(7)).is_err());
    assert!(ledger.append(epoch(6)).is_err());
    assert_eq!(ledger.append(ordinary(1)).unwrap().seq, 2);
}

#[test]
fn every_catalog_vector_is_stored_byte_for_byte() {
    let temp = Temp::new();
    let signer = signer();
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../../../spec/vectors/events/vectors.json")).unwrap();
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    for vector in vectors["vectors"].as_array().unwrap() {
        let expected = Event::decode(&unhex(vector["event_wire_hex"].as_str().unwrap())).unwrap();
        let actual = ledger.append(expected.body.clone()).unwrap();
        actual.verify(signer.public_key()).unwrap();
        assert_eq!(to_canon(&actual).unwrap(), to_canon(&expected).unwrap());
    }
    assert_eq!(ledger.head().unwrap().unwrap().seq, 21);
    drop(ledger);
    Ledger::open(&temp.0, &signer).unwrap();
}

#[test]
fn storage_layout_matches_normative_vector() {
    let temp = Temp::new();
    let signer = signer();
    let _ledger = Ledger::open(&temp.0, &signer).unwrap();
    let vector: serde_json::Value =
        serde_json::from_str(include_str!("../../../spec/vectors/ledger/layout.json")).unwrap();
    let connection = Connection::open(temp.database()).unwrap();
    let app: i64 = connection
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .unwrap();
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    let schema: String = connection
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE name='events'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(app, vector["application_id"].as_i64().unwrap());
    assert_eq!(version, vector["user_version"].as_i64().unwrap());
    assert_eq!(schema, vector["schema"].as_str().unwrap());
}

#[test]
fn failed_insert_rolls_back_and_requires_reopening() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    ledger.append(genesis(&signer)).unwrap();
    let connection = Connection::open(temp.database()).unwrap();
    connection.execute_batch("CREATE TRIGGER injected_failure AFTER INSERT ON events BEGIN SELECT RAISE(ABORT, 'injected storage failure'); END").unwrap();
    assert!(matches!(
        ledger.append(ordinary(1)),
        Err(LedgerError::Sql(_))
    ));
    let count: i64 = connection
        .query_row("SELECT count(*) FROM events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
    assert!(matches!(
        ledger.append(ordinary(1)),
        Err(LedgerError::Poisoned)
    ));
    connection
        .execute_batch("DROP TRIGGER injected_failure")
        .unwrap();
    drop(ledger);
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    assert_eq!(ledger.append(ordinary(1)).unwrap().seq, 1);
}

#[test]
fn recovery_rejects_holes_signed_wrong_links_row_seq_and_signatures() {
    for corruption in 0..8 {
        let temp = Temp::new();
        let signer = signer();
        let events = populated(&temp, &signer);
        let connection = Connection::open(temp.database()).unwrap();
        match corruption {
            0 => {
                connection
                    .execute("DELETE FROM events WHERE seq=1", [])
                    .unwrap();
            }
            1 => {
                connection
                    .execute("DELETE FROM events WHERE seq=0", [])
                    .unwrap();
            }
            2 => {
                let mut event = events[3].clone();
                event.sig = events[0].sig.clone();
                connection
                    .execute(
                        "UPDATE events SET wire=?1 WHERE seq=3",
                        [to_canon(&event).unwrap()],
                    )
                    .unwrap();
            }
            3 => {
                let event = Event::sign(
                    1,
                    0,
                    Some(Hash::from_bytes([0xff; 32])),
                    ordinary(1),
                    &signer,
                )
                .unwrap();
                connection
                    .execute(
                        "UPDATE events SET wire=?1 WHERE seq=1",
                        [to_canon(&event).unwrap()],
                    )
                    .unwrap();
            }
            4 => {
                connection
                    .execute("UPDATE events SET seq=8 WHERE seq=3", [])
                    .unwrap();
            }
            5 => {
                let event = Event::sign(3, 2, events[3].prev, ordinary(1), &signer).unwrap();
                connection
                    .execute(
                        "UPDATE events SET wire=?1 WHERE seq=3",
                        [to_canon(&event).unwrap()],
                    )
                    .unwrap();
            }
            6 => {
                connection
                    .execute("UPDATE events SET wire=x'ff' WHERE seq=1", [])
                    .unwrap();
            }
            7 => {
                connection
                    .execute("DELETE FROM events WHERE seq>0", [])
                    .unwrap();
                let event =
                    Event::sign(2, 0, Some(hash(&events[0]).unwrap()), ordinary(1), &signer)
                        .unwrap();
                connection
                    .execute(
                        "INSERT INTO events(seq, wire) VALUES (2, ?1)",
                        [to_canon(&event).unwrap()],
                    )
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let read_rows = || {
            connection
                .prepare("SELECT seq, wire FROM events ORDER BY seq")
                .unwrap()
                .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?)))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };
        let before = read_rows();
        assert!(
            Ledger::open(&temp.0, &signer).is_err(),
            "corruption={corruption}"
        );
        // Erro não autoriza remover eventos para continuar.
        assert_eq!(read_rows(), before);
    }
}

#[test]
fn recovery_checks_all_events_and_fixed_external_key() {
    let temp = Temp::new();
    let signer = signer();
    populated(&temp, &signer);
    let wrong = FileSigner::from_seed(&[0x22; 32]).unwrap();
    assert!(matches!(
        Ledger::open(&temp.0, &wrong),
        Err(LedgerError::Event(_))
    ));
    Ledger::open(&temp.0, &signer).unwrap();
}

#[test]
fn unknown_schema_versions_and_triggers_are_rejected() {
    for sql in [
        "PRAGMA user_version=2",
        "PRAGMA application_id=10",
        "CREATE TABLE alien(value BLOB)",
        "CREATE TRIGGER swallow BEFORE INSERT ON events BEGIN SELECT RAISE(IGNORE); END",
        "ALTER TABLE events ADD COLUMN extra INTEGER",
    ] {
        let temp = Temp::new();
        let signer = signer();
        populated(&temp, &signer);
        let connection = Connection::open(temp.database()).unwrap();
        connection.execute_batch(sql).unwrap();
        assert!(
            matches!(Ledger::open(&temp.0, &signer), Err(LedgerError::Invalid(_))),
            "{sql}"
        );
    }
    let temp = Temp::new();
    Connection::open(temp.database())
        .unwrap()
        .execute_batch("CREATE TABLE alien(x)")
        .unwrap();
    assert!(Ledger::open(&temp.0, &signer()).is_err());
}

#[test]
fn sqlite_failure_poisoned_handle_requires_replay() {
    let temp = Temp::new();
    let signer = signer();
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    ledger.append(genesis(&signer)).unwrap();
    let mut other = Connection::open(temp.database()).unwrap();
    let busy = other
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    assert!(matches!(
        ledger.append(ordinary(1)),
        Err(LedgerError::Sql(_))
    ));
    assert!(matches!(ledger.head(), Err(LedgerError::Poisoned)));
    assert!(matches!(
        ledger.append(ordinary(1)),
        Err(LedgerError::Poisoned)
    ));
    busy.rollback().unwrap();
    drop(ledger);
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    assert_eq!(ledger.head().unwrap().unwrap().seq, 0);
    assert_eq!(ledger.append(ordinary(1)).unwrap().seq, 1);
}

struct FailingSigner {
    inner: FileSigner,
    fail: Cell<bool>,
    wrong: FileSigner,
    forge: Cell<bool>,
}
impl Signer for FailingSigner {
    fn public_key(&self) -> &PublicKey {
        self.inner.public_key()
    }
    fn sign<D: SigDomain>(&self, bytes: &[u8]) -> Result<Signature, CryptoError> {
        if self.fail.get() {
            return Err(CryptoError::KeyFileFormat);
        }
        if self.forge.get() {
            return self.wrong.sign::<D>(bytes);
        }
        self.inner.sign::<D>(bytes)
    }
}

#[test]
fn faulty_signer_never_persists_invalid_event_or_consumes_sequence() {
    let temp = Temp::new();
    let signer = FailingSigner {
        inner: signer(),
        fail: Cell::new(true),
        wrong: FileSigner::from_seed(&[0x22; 32]).unwrap(),
        forge: Cell::new(false),
    };
    let mut ledger = Ledger::open(&temp.0, &signer).unwrap();
    assert!(ledger.append(genesis(&signer)).is_err());
    assert_eq!(ledger.head().unwrap(), None);
    signer.fail.set(false);
    signer.forge.set(true);
    assert!(ledger.append(genesis(&signer)).is_err());
    assert_eq!(ledger.head().unwrap(), None);
    signer.forge.set(false);
    assert_eq!(ledger.append(genesis(&signer)).unwrap().seq, 0);
}

#[test]
fn lock_is_held_between_transactions_and_released_on_drop() {
    let temp = Temp::new();
    let signer = signer();
    let ledger = Ledger::open(&temp.0, &signer).unwrap();
    assert!(matches!(
        Ledger::open(&temp.0, &signer),
        Err(LedgerError::WriterBusy)
    ));
    assert!(matches!(
        Ledger::open(&temp.0.join("."), &signer),
        Err(LedgerError::WriterBusy)
    ));
    drop(ledger);
    Ledger::open(&temp.0, &signer).unwrap();
}

#[test]
fn subprocess_lock_holder() {
    let Some(path) = std::env::var_os("LOGOS_LEDGER_LOCK_TEST") else {
        return;
    };
    let signer = signer();
    let _ledger = Ledger::open(std::path::Path::new(&path), &signer).unwrap();
    println!("LEDGER_LOCK_READY");
    std::io::stdout().flush().unwrap();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
}

#[test]
fn second_process_fails_to_open_writer() {
    let temp = Temp::new();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "subprocess_lock_holder", "--nocapture"])
        .env("LOGOS_LEDGER_LOCK_TEST", &temp.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    loop {
        line.clear();
        assert!(
            reader.read_line(&mut line).unwrap() > 0,
            "child exited before lock"
        );
        if line.contains("LEDGER_LOCK_READY") {
            break;
        }
    }
    assert!(matches!(
        Ledger::open(&temp.0, &signer()),
        Err(LedgerError::WriterBusy)
    ));
    child.stdin.take().unwrap().write_all(b"EXIT\n").unwrap();
    assert!(child.wait().unwrap().success());
    Ledger::open(&temp.0, &signer()).unwrap();
}

#[cfg(unix)]
#[test]
fn directory_symlink_uses_same_lock_and_database_symlink_is_rejected() {
    let temp = Temp::new();
    let aliases = Temp::new();
    let alias = aliases.0.join("alias");
    std::os::unix::fs::symlink(&temp.0, &alias).unwrap();
    let signer = signer();
    let ledger = Ledger::open(&temp.0, &signer).unwrap();
    assert!(matches!(
        Ledger::open(&alias, &signer),
        Err(LedgerError::WriterBusy)
    ));
    drop(ledger);
    let renamed = temp.0.join("moved.sqlite3");
    std::fs::rename(temp.database(), &renamed).unwrap();
    std::os::unix::fs::symlink(&renamed, temp.database()).unwrap();
    assert!(Ledger::open(&temp.0, &signer).is_err());
}
