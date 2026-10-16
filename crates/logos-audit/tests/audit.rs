//! Fixture Python independente, processos reais e adulterações isoladas.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use logos_canon::{Array, Canon, Value, from_canon, to_canon};
use logos_crypto::{Domain, FileSigner, PublicKey, SigDomain, Signer, hash_raw, sig_tags, tags};
use rusqlite::{Connection, params};

const MAGIC: &[u8] = b"LOGOS-AUDIT/v1\n";
const FIXTURE: &str = include_str!("../../../spec/vectors/audit/vectors.json");
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "logos-audit-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("{e}"),
            }
        }
    }
    fn fixture(&self) -> Vec<Vec<u8>> {
        let fixture: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
        for o in fixture["objects"].as_array().unwrap() {
            let h = o["digest"].as_str().unwrap();
            let p = self
                .0
                .join("cas")
                .join(format!("{}-v1", o["domain"].as_str().unwrap()))
                .join(&h[..2]);
            fs::create_dir_all(&p).unwrap();
            fs::write(p.join(&h[2..]), unhex(o["wire"].as_str().unwrap())).unwrap();
        }
        let events = fixture["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| unhex(w.as_str().unwrap()))
            .collect::<Vec<_>>();
        self.stream(&events);
        events
    }
    fn stream(&self, events: &[Vec<u8>]) -> PathBuf {
        let path = self.0.join("events.logos");
        fs::write(&path, stream(events)).unwrap();
        path
    }
    fn command(&self, command: &str, path: &Path) -> Output {
        let signer = signer();
        let mut c = Command::new(env!("CARGO_BIN_EXE_logos-audit"));
        c.arg(command)
            .arg(path)
            .arg("--public-key")
            .arg(hex(signer.public_key().as_bytes()));
        if command == "verify" {
            c.arg("--cas").arg(self.0.join("cas"));
        }
        c.output().unwrap()
    }
    fn verify(&self, events: &[Vec<u8>]) -> Output {
        self.command("verify", &self.stream(events))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn signer() -> FileSigner {
    FileSigner::from_seed(&[17; 32]).unwrap()
}
fn unhex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect()
}
fn hex(b: impl AsRef<[u8]>) -> String {
    b.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}
fn array(v: Vec<Value>) -> Value {
    Value::Array(Array::new(v).unwrap())
}
fn items(v: &Value) -> &[Value] {
    match v {
        Value::Array(a) => a.items(),
        _ => panic!("array"),
    }
}
fn unsigned(v: u64) -> Value {
    Value::Unsigned(v)
}
fn set(v: &mut Value, path: &[usize], value: Value) {
    if path.is_empty() {
        *v = value;
        return;
    }
    let mut values = items(v).to_vec();
    set(&mut values[path[0]], &path[1..], value);
    *v = array(values);
}
struct EventDomain;
impl Domain for EventDomain {
    const TAG: &'static [u8] = tags::EVENT;
}
impl SigDomain for EventDomain {
    const TAG: &'static [u8] = sig_tags::EVENT;
}
fn resign(events: &mut [Vec<u8>]) {
    let mut previous = None;
    for wire in events {
        let mut v = from_canon::<Value>(wire).unwrap();
        let prev = previous
            .map(|p: [u8; 32]| array(vec![Value::Bytes(p.to_vec())]))
            .unwrap_or_else(|| array(vec![]));
        set(&mut v, &[4], prev);
        let sig = signer()
            .sign::<EventDomain>(&to_canon(&array(items(&v)[..7].to_vec())).unwrap())
            .unwrap();
        set(&mut v, &[7], sig.to_value().unwrap());
        *wire = to_canon(&v).unwrap();
        previous = Some(*hash_raw::<EventDomain>(wire).as_bytes());
    }
}
fn mutate(events: &mut [Vec<u8>], event: usize, path: &[usize], value: Value) {
    let mut v = from_canon::<Value>(&events[event]).unwrap();
    set(&mut v, path, value);
    events[event] = to_canon(&v).unwrap();
    resign(events);
}
fn stream(events: &[Vec<u8>]) -> Vec<u8> {
    let mut output = MAGIC.to_vec();
    for e in events {
        output.extend_from_slice(&(e.len() as u32).to_be_bytes());
        output.extend(e);
    }
    output
}
fn failed(output: Output, expected: &str) {
    assert!(
        !output.status.success(),
        "unexpected success: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stdout.is_empty(), "partial output");
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(
        error.contains("AUDIT_FAILED failures=1") && error.contains(expected),
        "{error}"
    );
}

#[test]
fn independent_fixture_rechecks_every_refutation_and_witness_then_exports_without_database() {
    let t = Temp::new();
    let events = t.fixture();
    let out = t.verify(&events);
    assert!(out.status.success(), "{:?}", out);
    let report = String::from_utf8(out.stdout).unwrap();
    assert!(report.contains("events=8 refutations_rechecked=4 consistencies_rechecked=1 proofs_unchecked=1 failures=0"), "{report}");
    let exported = t.command("export", &t.0.join("events.logos"));
    assert!(exported.status.success());
    let fixture: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(
        exported.stdout,
        unhex(fixture["stream_hex"].as_str().unwrap())
    );
    assert!(
        String::from_utf8(exported.stderr)
            .unwrap()
            .contains("authenticated_only=true")
    );
    // Todos os quatro precisam passar pelo checker, inclusive o último.
    for position in 1..=4 {
        let mut bad = events.clone();
        mutate(&mut bad, position, &[5, 2, 3, 1], unsigned(0));
        failed(t.verify(&bad), "model recheck: Unknown(Steps)");
    }
}

#[test]
fn signed_semantically_invalid_refutations_are_not_accepted_by_integrity_alone() {
    let t = Temp::new();
    let events = t.fixture();
    // A obrigação do Verified tem goal=True; modelo e certificado são válidos.
    let verified = from_canon::<Value>(&events[5]).unwrap();
    let record = items(&items(&verified)[5])[2].clone();
    let witness = items(&items(&items(&verified)[5])[5])[1].clone();
    for position in 1..=4 {
        let mut bad = events.clone();
        mutate(&mut bad, position, &[5, 2], record.clone());
        mutate(&mut bad, position, &[5, 3], witness.clone());
        failed(t.verify(&bad), "model recheck: Rejected");
    }
    let mut unknown = events.clone();
    mutate(
        &mut unknown,
        1,
        &[5, 2, 4],
        array(vec![
            unsigned(1),
            unsigned(2),
            array(vec![unsigned(1), unsigned(1)]),
        ]),
    );
    failed(t.verify(&unknown), "event requires Accepted");
}

#[test]
fn detects_more_than_ten_independent_history_and_structure_tampers() {
    let t = Temp::new();
    let original = t.fixture();
    let mut removed = original.clone();
    removed.remove(2);
    failed(t.verify(&removed), "seq/prev mismatch");
    let mut reordered = original.clone();
    reordered.swap(2, 3);
    failed(t.verify(&reordered), "seq/prev mismatch");
    let mut bad = original.clone();
    bad[2].pop();
    failed(t.verify(&bad), "canon:");
    let mut bad = original.clone();
    bad[2].push(0);
    failed(t.verify(&bad), "canon:");
    let mut bad = original.clone();
    let n = bad[2].len();
    bad[2][n - 1] ^= 1;
    failed(t.verify(&bad), "signature:");
    let changes = [
        (2, vec![2], unsigned(99), "seq/prev mismatch"),
        (2, vec![3], unsigned(3), "epoch changed"),
        (2, vec![6], Value::Bytes(vec![0; 32]), "KeyId mismatch"),
        (2, vec![5, 0], unsigned(5), "unknown event body"),
        (2, vec![5, 1], unsigned(99), "unknown event body"),
        (7, vec![5, 2], unsigned(3), "epoch body/envelope mismatch"),
        (7, vec![5, 4], array(vec![]), ""), // still has vocab: control below
        (
            5,
            vec![5, 4],
            array(vec![Value::Bytes(vec![1; 32])]),
            "used not subset",
        ),
        (
            5,
            vec![5, 5, 2, 0],
            Value::Bytes(vec![1; 32]),
            "consistency obligation mismatch",
        ),
        (
            6,
            vec![5, 2, 4],
            array(vec![unsigned(1), unsigned(0)]),
            "UnknownRecorded requires Unknown",
        ),
    ];
    for (i, path, value, expected) in changes {
        let mut bad = original.clone();
        mutate(&mut bad, i, &path, value);
        if expected.is_empty() {
            assert!(t.verify(&bad).status.success());
        } else {
            failed(t.verify(&bad), expected);
        }
    }
    let mut bad = original.clone();
    mutate(&mut bad, 7, &[5, 4], array(vec![]));
    mutate(&mut bad, 7, &[5, 5], array(vec![]));
    failed(t.verify(&bad), "empty epoch-body/v2");
    // Reassinatura não pode esconder elo errado.
    let mut bad = original.clone();
    let mut value = from_canon::<Value>(&bad[2]).unwrap();
    set(&mut value, &[4], array(vec![Value::Bytes(vec![0; 32])]));
    let sig = signer()
        .sign::<EventDomain>(&to_canon(&array(items(&value)[..7].to_vec())).unwrap())
        .unwrap();
    set(&mut value, &[7], sig.to_value().unwrap());
    bad[2] = to_canon(&value).unwrap();
    failed(t.verify(&bad), "seq/prev mismatch");
    // Prefixo válido permanece válido: não há âncora externa na3.6.
    assert!(t.verify(&original[..4]).status.success());
}

#[test]
fn cas_missing_corrupt_wrong_certificate_and_symlinks_are_explicit_failures() {
    let t = Temp::new();
    let events = t.fixture();
    let fixture: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    for domain in ["vocab", "obligation", "certificate", "checker", "model"] {
        let object = fixture["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["domain"] == domain)
            .unwrap();
        let h = object["digest"].as_str().unwrap();
        let file =
            t.0.join("cas")
                .join(format!("{domain}-v1"))
                .join(&h[..2])
                .join(&h[2..]);
        let wire = fs::read(&file).unwrap();
        fs::remove_file(&file).unwrap();
        failed(t.verify(&events), "I/O:");
        fs::write(&file, b"altered").unwrap();
        failed(t.verify(&events), "CAS integrity:");
        fs::write(&file, &wire).unwrap();
        let dest = t.0.join("linked-object");
        fs::rename(&file, &dest).unwrap();
        std::os::unix::fs::symlink(&dest, &file).unwrap();
        failed(t.verify(&events), "regular file");
        fs::remove_file(&file).unwrap();
        fs::rename(&dest, &file).unwrap();
    }
    let v = from_canon::<Value>(&events[1]).unwrap();
    let first_cert = items(&items(&items(&v)[5])[2])[1].clone();
    let mut bad = events.clone();
    mutate(&mut bad, 2, &[5, 2, 1], first_cert);
    failed(t.verify(&bad), "certificate/model bytes differ");
    for level in ["cas", "cas/model-v1"] {
        let path = t.0.join(level);
        let dest = t.0.join("directory-target");
        fs::rename(&path, &dest).unwrap();
        std::os::unix::fs::symlink(&dest, &path).unwrap();
        failed(t.verify(&events), "CAS directory without symlink");
        fs::remove_file(&path).unwrap();
        fs::rename(&dest, &path).unwrap();
    }
}

fn sqlite(t: &Temp, events: &[Vec<u8>]) -> Connection {
    fs::write(t.0.join("writer.lock"), []).unwrap();
    let c = Connection::open(t.0.join("ledger.sqlite3")).unwrap();
    c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; PRAGMA application_id=1280264019; PRAGMA user_version=1; CREATE TABLE events (seq INTEGER PRIMARY KEY CHECK (seq >= 0), wire BLOB NOT NULL CHECK (length(wire) BETWEEN 1 AND 16777216)) STRICT;").unwrap();
    for (i, e) in events.iter().enumerate() {
        c.execute(
            "INSERT INTO events(seq,wire) VALUES(?1,?2)",
            params![i as i64, e],
        )
        .unwrap();
    }
    c
}

#[test]
fn sqlite_reads_committed_wal_and_rejects_schema_column_and_busy_writer() {
    let t = Temp::new();
    let events = t.fixture();
    let c = sqlite(&t, &events);
    assert!(fs::metadata(t.0.join("ledger.sqlite3-wal")).unwrap().len() > 0);
    let output = t.command("verify", &t.0);
    assert!(output.status.success(), "{output:?}");
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("events=8 refutations_rechecked=4")
    );
    let exported = t.command("export", &t.0);
    assert!(exported.status.success());
    assert_eq!(exported.stdout, stream(&events));
    let lock = fs::File::open(t.0.join("writer.lock")).unwrap();
    lock.try_lock().unwrap();
    failed(t.command("verify", &t.0), "writer lock:");
    drop(lock);
    c.execute("UPDATE events SET seq=99 WHERE seq=3", [])
        .unwrap();
    failed(t.command("verify", &t.0), "seq/prev mismatch");
    c.execute("UPDATE events SET seq=3 WHERE seq=99", [])
        .unwrap();
    c.execute_batch("CREATE VIEW impostor AS SELECT * FROM events;")
        .unwrap();
    failed(
        t.command("verify", &t.0),
        "schema/application/version mismatch",
    );
    drop(c);
}

#[test]
fn export_refuses_tamper_before_output_and_framing_rejects_truncation_limits_or_junk() {
    let t = Temp::new();
    let events = t.fixture();
    let full = stream(&events);
    for cut in [
        1,
        MAGIC.len() - 1,
        MAGIC.len() + 1,
        MAGIC.len() + 3,
        full.len() - 1,
    ] {
        let path = t.0.join("cut.logos");
        fs::write(&path, &full[..cut]).unwrap();
        failed(t.command("export", &path), "");
    }
    for extra in [
        &[0][..],
        &[0, 0, 0, 0],
        &[1, 0, 0, 1],
        &[255, 255, 255, 255],
    ] {
        let path = t.0.join("bad-frame.logos");
        let mut wire = full.clone();
        wire.extend(extra);
        fs::write(&path, wire).unwrap();
        failed(t.command("export", &path), "");
    }
    let mut bad = events.clone();
    bad.remove(6);
    let path = t.stream(&bad);
    failed(t.command("export", &path), "seq/prev mismatch");
    let path = t.stream(&[]);
    assert!(t.command("export", &path).status.success());
}

#[test]
fn cli_requires_external_key_and_reports_wrong_key_without_partial_output() {
    let t = Temp::new();
    t.fixture();
    let input = t.0.join("events.logos");
    let out = Command::new(env!("CARGO_BIN_EXE_logos-audit"))
        .arg("verify")
        .arg(&input)
        .output()
        .unwrap();
    failed(out, "uso:");
    let wrong = FileSigner::from_seed(&[31; 32]).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_logos-audit"))
        .arg("export")
        .arg(&input)
        .arg("--public-key")
        .arg(hex(wrong.public_key().as_bytes()))
        .output()
        .unwrap();
    failed(out, "KeyId mismatch");
    let before = fs::read(&input).unwrap();
    let key: PublicKey = signer().public_key().clone();
    let report = logos_audit::authenticate(&input, &key).unwrap();
    assert_eq!(report.events, 8);
    assert_eq!(report.refutations, 0);
    assert_eq!(fs::read(&input).unwrap(), before);
}

#[test]
fn size_limits_and_missing_lock_fail_without_creating_an_input() {
    let t = Temp::new();
    let events = t.fixture();
    let fixture: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    let o = fixture["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["domain"] == "model")
        .unwrap();
    let h = o["digest"].as_str().unwrap();
    let path = t.0.join("cas/model-v1").join(&h[..2]).join(&h[2..]);
    let original = fs::read(&path).unwrap();
    let file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&path)
        .unwrap();
    file.set_len(16_777_217).unwrap();
    drop(file);
    failed(t.verify(&events), "CAS model exceeds limit");
    fs::write(path, original).unwrap();
    let mut bad = events.clone();
    mutate(&mut bad, 1, &[5, 2, 3, 3], unsigned(1));
    failed(t.verify(&bad), "model recheck: Unknown(Size)");
    let input = t.0.join("bad-length.logos");
    let mut bytes = MAGIC.to_vec();
    bytes.extend_from_slice(&16_777_217u32.to_be_bytes());
    fs::write(&input, bytes).unwrap();
    failed(t.command("export", &input), "invalid frame length");
    let c = sqlite(&t, &events);
    drop(c);
    fs::remove_file(t.0.join("writer.lock")).unwrap();
    failed(t.command("export", &t.0), "I/O:");
    assert!(!t.0.join("writer.lock").exists());
    let missing = t.0.join("missing");
    failed(t.command("export", &missing), "I/O:");
    assert!(!missing.exists());
}

fn cnf_history(t: &Temp) -> Vec<Vec<u8>> {
    let base = t.fixture();
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../../../spec/vectors/cnf/vectors.json")).unwrap();
    let mut events = vec![base[0].clone()];
    for c in data["vectors"].as_array().unwrap().iter().take(3) {
        let vocab = unhex(c["vocabulary"].as_str().unwrap());
        let ob = unhex(c["obligation"].as_str().unwrap());
        let ov = from_canon::<Value>(&ob).unwrap();
        let ho = logos_crypto::hash_raw::<logos_obligation::Obligation>(&ob);
        let Value::Bytes(hv) = &items(&ov)[4] else {
            panic!("vocab hash");
        };
        let hc = unhex(c["cnf_digest"].as_str().unwrap());
        for (domain, h, wire) in [
            ("vocab", hex(hv), vocab),
            ("obligation", ho.to_hex(), ob),
            (
                "cnf",
                hex(&hc),
                c["cnf"].as_str().unwrap().as_bytes().to_vec(),
            ),
        ] {
            let dir = t.0.join("cas").join(format!("{domain}-v1")).join(&h[..2]);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(&h[2..]), wire).unwrap();
        }
        let body = array(vec![
            unsigned(2),
            unsigned(3),
            ho.to_value().unwrap(),
            unsigned(1),
            Value::Bytes(unhex(c["artifact"].as_str().unwrap())),
            array(vec![Value::Bytes(hc), unsigned(1)]),
        ]);
        let mut event = from_canon::<Value>(&base[1]).unwrap();
        set(&mut event, &[2], unsigned(events.len() as u64));
        set(&mut event, &[5], body);
        events.push(to_canon(&event).unwrap());
    }
    resign(&mut events);
    events
}

#[test]
fn cnf_rederivation_covers_every_event_sqlite_and_export_without_proof_promotion() {
    let t = Temp::new();
    let events = cnf_history(&t);
    let out = t.verify(&events);
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("proofs_unchecked=0"));
    assert!(text.contains("cnf_rederived=3"));
    let key = signer().public_key().clone();
    let auth = logos_audit::authenticate(&t.0.join("events.logos"), &key).unwrap();
    assert_eq!(auth.cnf_derivations, 0);
    let db = sqlite(&t, &events);
    let out = t.command("verify", &t.0);
    assert!(out.status.success(), "{out:?}");
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("cnf_rederived=3")
    );
    let out = t.command("export", &t.0);
    assert!(out.status.success());
    assert_eq!(out.stdout, stream(&events));
    drop(db);
    let path = t.0.join("export.logos");
    fs::write(&path, out.stdout).unwrap();
    fs::remove_file(t.0.join("ledger.sqlite3")).unwrap();
    assert!(t.command("verify", &path).status.success());
}

#[test]
fn signed_wrong_cnf_cannot_pass_with_valid_cas_hashes_at_any_position() {
    let t = Temp::new();
    let events = cnf_history(&t);
    for position in 1..=3 {
        let other = from_canon::<Value>(&events[position % 3 + 1]).unwrap();
        let fields = items(&items(&other)[5]);
        let mut bad = events.clone();
        mutate(&mut bad, position, &[5, 4], fields[4].clone());
        mutate(&mut bad, position, &[5, 5], fields[5].clone());
        failed(t.verify(&bad), "CNF rederivation mismatch");
    }
    let mut bad = events.clone();
    mutate(&mut bad, 1, &[5, 4], Value::Bytes(vec![0; 32]));
    failed(t.verify(&bad), "CNF rederivation mismatch");
    let mut bad = events.clone();
    mutate(&mut bad, 1, &[5, 5, 1], unsigned(2));
    failed(t.verify(&bad), "CNF encoder version/record");
    let v = from_canon::<Value>(&events[1]).unwrap();
    let b = items(&items(&v)[5]);
    let old = array(vec![
        unsigned(1),
        unsigned(3),
        b[2].clone(),
        unsigned(1),
        b[4].clone(),
    ]);
    let mut bad = events.clone();
    mutate(&mut bad, 1, &[5], old);
    failed(t.verify(&bad), "LRAT derivation metadata missing");
    assert!(t.command("export", &t.stream(&bad)).status.success());
    let Value::Bytes(h) = &items(&b[5])[0] else {
        panic!("hash");
    };
    let h = hex(h);
    let p = t.0.join("cas/cnf-v1").join(&h[..2]).join(&h[2..]);
    let saved = fs::read(&p).unwrap();
    fs::remove_file(&p).unwrap();
    failed(t.verify(&events), "I/O:");
    fs::write(&p, b"p cnf 0 0\n").unwrap();
    failed(t.verify(&events), "CAS integrity:");
    fs::write(p, saved).unwrap();
}
