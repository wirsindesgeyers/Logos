//! 200 CNFs históricas SAT-2002, certificados e corrupções, fixtures offline.
use logos_core::{Budget, CheckOutcome};
use logos_crypto::hash_raw;
use logos_ground::Cnf;
use logos_lrat::{CakeRegistry, LratChecker};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Installed(PathBuf);
impl Drop for Installed {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

/// ADR0057: corpus inteiro no nível completo (padrão); `LOGOS_TIER=fast` checa 1 a cada 10.
fn stride() -> usize {
    if std::env::var("LOGOS_TIER").is_ok_and(|tier| tier == "fast") {
        10
    } else {
        1
    }
}

#[test]
fn official_sat2002_corpus_and_three_corruptions_per_proof() {
    // XZ/JSON apenas no harness não confiável; Python padrão, sem solver/rede na CI.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new("python3")
        .arg(root.join("tools/lrat_corpus.py"))
        .arg("--emit")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let fixture: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert!(cases.len() >= 200);
    let checker = LratChecker::new().unwrap();
    let directory =
        Installed(std::env::temp_dir().join(format!("logos-corpus-cake-{}", std::process::id())));
    std::fs::create_dir(&directory.0).unwrap();
    let binary = directory.0.join("cake_lpr");
    let install = Command::new("python3")
        .arg(root.join("tools/build_cake_lpr.py"))
        .arg("--output")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        install.status.success(),
        "{}",
        String::from_utf8_lossy(&install.stderr)
    );
    let cake = CakeRegistry::register(&binary).unwrap();
    // Concordância do corpus, sem SLA de latência: manter limite finito sob carga.
    // Unknown continua falhando; todas as 600 corrupções precisam de Rejected.
    let budget = Budget::new(10_000_000_000, 256_000_000, 16_000_000, 180_000);
    let mut hashes = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let mut corrupted = 0;
    let mut accepted = 0;
    for (position, case) in cases.iter().enumerate().step_by(stride()) {
        accepted += 1;
        let name = case["source"].as_str().unwrap();
        let cnf = case["cnf"].as_str().unwrap().as_bytes();
        let proof = case["proof"].as_str().unwrap();
        let hash = hash_raw::<Cnf>(cnf);
        assert!(sources.insert(name));
        assert!(hashes.insert(hash));
        assert_eq!(hash.to_hex(), case["h_cnf"].as_str().unwrap());
        let report = checker.check_with_used(cnf, hash, proof.as_bytes(), &budget);
        let external = cake
            .check_bytes(
                cnf,
                hash,
                proof.as_bytes(),
                hash_raw(proof.as_bytes()),
                &budget,
            )
            .unwrap();
        assert_eq!(report.outcome(), &CheckOutcome::Accepted, "{name}");
        assert_eq!(external, CheckOutcome::Accepted, "cake_lpr: {name}");
        let used: Vec<u64> = case["used"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap())
            .collect();
        assert_eq!(
            report.used_original_clauses(),
            Some(used.as_slice()),
            "{name}"
        );
        let (prefix, last) = proof
            .trim_end()
            .rsplit_once('\n')
            .map_or(("", proof.trim_end()), |(a, b)| (a, b));
        let fields: Vec<&str> = last.split_whitespace().collect();
        assert_eq!(fields[1], "0");
        assert!(fields.len() > 3);
        let id = fields[0].parse::<u64>().unwrap();
        let suffix = fields[2..].join(" ");
        let prefix = if prefix.is_empty() {
            String::new()
        } else {
            format!("{prefix}\n")
        };
        let (first, rest) = proof.split_once('\n').unwrap();
        let mut first_fields: Vec<String> = first.split_whitespace().map(str::to_owned).collect();
        let hint_position = first_fields.iter().position(|x| x == "0").unwrap() + 1;
        first_fields[hint_position] = (id + 1).to_string();
        let wrong_hint = format!("{}\n{rest}", first_fields.join(" "));
        let changed_clause = format!("{prefix}{id} 1 0 {suffix}\n");
        let unjustified = format!("{id} 0 0\n");
        for bad in [wrong_hint, changed_clause, unjustified] {
            let r = checker.check_with_used(cnf, hash, bad.as_bytes(), &budget);
            let external = cake
                .check_bytes(cnf, hash, bad.as_bytes(), hash_raw(bad.as_bytes()), &budget)
                .unwrap();
            assert!(
                matches!(external, CheckOutcome::Rejected(_)),
                "cake_lpr: {name}: {external:?}"
            );
            assert!(
                matches!(r.outcome(), CheckOutcome::Rejected(_)),
                "{name}: {r:?}"
            );
            assert_eq!(r.used_original_clauses(), None);
            corrupted += 1;
        }
        if (position + 1) % 25 == 0 {
            println!(
                "LRAT_AGREEMENT_PROGRESS accepted={} rejected={corrupted}",
                position + 1
            );
        }
    }
    println!(
        "LRAT_CORPUS_AGREEMENT_OK rust_and_cake_accepted={accepted}/{} corrupted_rejected={corrupted}",
        cases.len()
    );
    assert_eq!(corrupted, 3 * accepted);
}
