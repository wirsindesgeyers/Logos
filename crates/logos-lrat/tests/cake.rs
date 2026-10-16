//! Registry, snapshots e processo cake_lpr real; nunca um fake para o aceite.
use logos_core::{Budget, CheckOutcome, Exhausted};
use logos_crypto::{Domain, Hash, hash_raw};
use logos_ground::Cnf;
use logos_lrat::{CakeRegistry, LratCertificate, decode_cake_output};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "logos-cake-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn binary(&self) -> PathBuf {
        let binary = self.0.join("cake_lpr");
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let status = Command::new("python3")
            .arg(root.join("tools/build_cake_lpr.py"))
            .arg("--output")
            .arg(&binary)
            .output()
            .unwrap();
        assert!(
            status.status.success(),
            "{}",
            String::from_utf8_lossy(&status.stderr)
        );
        binary
    }
    fn put<D: Domain>(&self, name: &str, bytes: &[u8]) -> Hash<D> {
        let h = hash_raw::<D>(bytes);
        let hex = h.to_hex();
        let p = self.0.join(format!("{name}-v1")).join(&hex[..2]);
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join(&hex[2..]), bytes).unwrap();
        h
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
const CNF: &[u8] = b"p cnf 1 2\n-1 0\n1 0\n";
const PROOF: &[u8] = b"3 0 1 2 0\n";
fn budget() -> Budget {
    Budget::new(100_000, 256 * 1024 * 1024, 1_000_000, 10_000)
}
fn check(
    cake: &CakeRegistry,
    cnf: &[u8],
    proof: &[u8],
    b: &Budget,
) -> Result<CheckOutcome, logos_lrat::CakeError> {
    cake.check_bytes(cnf, hash_raw(cnf), proof, hash_raw(proof), b)
}
#[test]
fn real_binary_accepts_and_rejects_without_rust_filter() {
    let t = Temp::new();
    let path = t.binary();
    let cake = CakeRegistry::register(&path).unwrap();
    assert_eq!(cake.identity(), hash_raw(&fs::read(path).unwrap()));
    assert_eq!(
        check(&cake, CNF, PROOF, &budget()).unwrap(),
        CheckOutcome::Accepted
    );
    for proof in [b"3 0 0\n".as_slice(), b"3 0 2 0\n", b"3 1 0 1 2 0\n", b""] {
        assert!(
            matches!(
                check(&cake, CNF, proof, &budget()).unwrap(),
                CheckOutcome::Rejected(_)
            ),
            "{proof:?}"
        );
    }
    // CakeML aceita sintaxe fora do perfil Rust: comparação não é um filtro Rust.
    let spaced = b"p cnf 1 2\n-1 0\n1 0\n";
    assert_eq!(
        check(&cake, spaced, PROOF, &budget()).unwrap(),
        CheckOutcome::Accepted
    );
}
#[test]
fn unknown_and_replaced_binary_never_runs() {
    let t = Temp::new();
    let path = t.binary();
    let cake = CakeRegistry::register(&path).unwrap();
    let original = fs::read(&path).unwrap();
    let mut bad = original.clone();
    bad.push(0);
    fs::write(&path, bad).unwrap();
    assert!(
        check(&cake, CNF, PROOF, &budget())
            .unwrap_err()
            .to_string()
            .contains("hash")
    );
    fs::write(&path, b"#!/bin/sh\ntouch /tmp/LOGOS_MUST_NOT_RUN\n").unwrap();
    assert!(CakeRegistry::register(&path).is_err());
    fs::write(&path, original).unwrap();
    assert_eq!(
        check(&cake, CNF, PROOF, &budget()).unwrap(),
        CheckOutcome::Accepted
    );
    let link = t.0.join("link");
    symlink(&path, &link).unwrap();
    assert!(CakeRegistry::register(&link).is_err());
    assert!(CakeRegistry::register(&t.0).is_err());
}
#[test]
fn cas_and_both_input_identities_are_checked() {
    let t = Temp::new();
    let cake = CakeRegistry::register(&t.binary()).unwrap();
    let cnf = t.put::<Cnf>("cnf", CNF);
    let proof = t.put::<LratCertificate>("certificate", PROOF);
    assert_eq!(
        cake.check_from_cas(&t.0, cnf, proof, &budget()).unwrap(),
        CheckOutcome::Accepted
    );
    let mut small = budget();
    small.size_bytes = (CNF.len() + PROOF.len() - 1) as u64;
    assert_eq!(
        cake.check_from_cas(&t.0, cnf, proof, &small).unwrap(),
        CheckOutcome::Unknown(Exhausted::Size)
    );
    assert!(
        cake.check_bytes(CNF, Hash::from_bytes([0; 32]), PROOF, proof, &budget())
            .is_err()
    );
    assert!(
        cake.check_bytes(CNF, cnf, PROOF, Hash::from_bytes([0; 32]), &budget())
            .is_err()
    );
    let h = proof.to_hex();
    let p = t.0.join("certificate-v1").join(&h[..2]).join(&h[2..]);
    fs::write(p, b"3 0 0\n").unwrap();
    assert!(cake.check_from_cas(&t.0, cnf, proof, &budget()).is_err());
}
#[test]
fn resource_limits_have_positive_controls() {
    let t = Temp::new();
    let cake = CakeRegistry::register(&t.binary()).unwrap();
    let mut b = budget();
    b.size_bytes = (CNF.len() + PROOF.len()) as u64;
    assert_eq!(
        check(&cake, CNF, PROOF, &b).unwrap(),
        CheckOutcome::Accepted
    );
    b.size_bytes -= 1;
    assert_eq!(
        check(&cake, CNF, PROOF, &b).unwrap(),
        CheckOutcome::Unknown(Exhausted::Size)
    );
    b = budget();
    b.mem_bytes = 0;
    assert_eq!(
        check(&cake, CNF, PROOF, &b).unwrap(),
        CheckOutcome::Unknown(Exhausted::Memory)
    );
    b = budget();
    b.wall_ms = 0;
    assert_eq!(
        check(&cake, CNF, PROOF, &b).unwrap(),
        CheckOutcome::Unknown(Exhausted::Time)
    );
    let large = format!(
        "c {}\n{}",
        "a".repeat(8_000_000),
        std::str::from_utf8(PROOF).unwrap()
    );
    b = budget();
    b.size_bytes = large.len() as u64 + CNF.len() as u64;
    b.wall_ms = 1;
    assert_eq!(
        check(&cake, CNF, large.as_bytes(), &b).unwrap(),
        CheckOutcome::Unknown(Exhausted::Time)
    );
    assert_eq!(
        check(&cake, CNF, PROOF, &budget()).unwrap(),
        CheckOutcome::Accepted
    );
}
#[test]
fn output_protocol_is_exact_and_bounded() {
    assert_eq!(
        decode_cake_output(0, b"s VERIFIED UNSAT\n", b"").unwrap(),
        CheckOutcome::Accepted
    );
    for (code, out, err) in [
        (1, b"s VERIFIED UNSAT\n".as_slice(), b"".as_slice()),
        (0, b"s VERIFIED UNSAT\nextra", b""),
        (0, b"s VERIFIED UNSAT\n", b"warning"),
        (0, b"", b""),
        (1, b"", b"bwrap: denied\n"),
    ] {
        assert!(decode_cake_output(code, out, err).is_err());
    }
    assert!(decode_cake_output(0, &vec![b'x'; 65537], b"").is_err());
    assert!(matches!(
        decode_cake_output(0, b"", b"c empty clause not derived at end of proof\n").unwrap(),
        CheckOutcome::Rejected(_)
    ));
    assert_eq!(
        decode_cake_output(3, b"", b"CakeML heap space exhausted.\n").unwrap(),
        CheckOutcome::Unknown(Exhausted::Memory)
    );
}

#[test]
fn independent_output_vectors_and_domain_separation() {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../../../spec/vectors/cake-lpr/output.json")).unwrap();
    for v in fixtures["vectors"].as_array().unwrap() {
        let result = decode_cake_output(
            v["status"].as_i64().unwrap() as i32,
            v["stdout"].as_str().unwrap().as_bytes(),
            v["stderr"].as_str().unwrap().as_bytes(),
        );
        match v["outcome"].as_str().unwrap() {
            "Accepted" => assert_eq!(result.unwrap(), CheckOutcome::Accepted),
            "Rejected" => assert!(matches!(result.unwrap(), CheckOutcome::Rejected(_))),
            "Memory" => assert_eq!(result.unwrap(), CheckOutcome::Unknown(Exhausted::Memory)),
            "Error" => assert!(result.is_err()),
            _ => panic!("unknown vector"),
        }
    }
    let t = Temp::new();
    let bytes = fs::read(t.binary()).unwrap();
    let raw = logos_crypto::ExternalDigest::of(&bytes);
    let identity = hash_raw::<logos_lrat::CakeBinary>(&bytes);
    assert_ne!(raw.as_bytes(), identity.as_bytes());
    assert_ne!(
        hash_raw::<LratCertificate>(&bytes).as_bytes(),
        identity.as_bytes()
    );
}
