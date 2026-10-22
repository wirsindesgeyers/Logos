//! `logos ledger retract`: operador retrata com a chave do kernel; a TCB autoriza.
#![cfg(target_os = "linux")]
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use logos_air::{Formula, FormulaNode, Limits, Vocabulary};
use logos_canon::{Text, to_canon};
use logos_crypto::{FileSigner, Hash, Signer, hash};
use logos_ledger::{Event, EventBody, Ledger};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    directory: PathBuf,
    target: Hash<Event>,
}

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "logos-cli-retract-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        for (name, byte) in [("kernel.key", "11"), ("other.key", "22")] {
            let path = directory.join(name);
            fs::write(&path, byte.repeat(32)).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        let signer = FileSigner::load(&directory.join("kernel.key")).unwrap();
        let mut ledger = Ledger::open(&directory.join("ledger"), &signer).unwrap();
        ledger
            .append(EventBody::Genesis {
                ledger_version: 1,
                kernel_binary: [0; 32],
                kernel_key: signer.key_id().unwrap(),
            })
            .unwrap();
        let vocabulary = Vocabulary::new(1, vec![], vec![]).unwrap();
        let hv = ledger
            .cas()
            .put::<Vocabulary>(&to_canon(&vocabulary).unwrap())
            .unwrap();
        let target = hash(
            &ledger
                .append(EventBody::AssumptionDeclared {
                    vocabulary: hv,
                    formula: Formula::new(vec![FormulaNode::True], Limits::default()).unwrap(),
                    scope: Text::new("session:cli").unwrap(),
                    author: signer.key_id().unwrap(),
                })
                .unwrap(),
        )
        .unwrap();
        Self { directory, target }
    }

    fn retract(&self, key: &str, target: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_logos"))
            .args(["ledger", "retract"])
            .arg(self.directory.join("ledger"))
            .arg("--key")
            .arg(self.directory.join(key))
            .args(["--target", target, "--reason", "fonte retirada"])
            .output()
            .unwrap()
    }

    fn retracted(&self) -> bool {
        let signer = FileSigner::load(&self.directory.join("kernel.key")).unwrap();
        let ledger = Ledger::open(&self.directory.join("ledger"), &signer).unwrap();
        ledger
            .live_index()
            .unwrap()
            .status(self.target)
            .unwrap()
            .unwrap()
            .retracted
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.directory).unwrap();
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn operator_retracts_assumption_once_and_prints_the_records() {
    let fixture = Fixture::new();
    let output = fixture.retract("kernel.key", &fixture.target.to_hex());
    assert!(output.status.success(), "{}", stderr(&output));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.starts_with("{\"retracted\":\""), "{text}");
    assert!(text.trim_end().ends_with("\"invalidated\":[]}"), "{text}");
    assert!(fixture.retracted());
    let again = fixture.retract("kernel.key", &fixture.target.to_hex());
    assert!(!again.status.success());
    assert!(
        stderr(&again).contains("already retracted"),
        "{}",
        stderr(&again)
    );
}

#[test]
fn unknown_target_foreign_key_and_bad_hex_are_refused() {
    let fixture = Fixture::new();
    let unknown = fixture.retract("kernel.key", &"ab".repeat(32));
    assert!(!unknown.status.success());
    assert!(stderr(&unknown).contains("unknown retraction target"));
    let foreign = fixture.retract("other.key", &fixture.target.to_hex());
    assert!(!foreign.status.success());
    // Sem a chave do kernel, o ledger nem abre para escrita.
    assert!(
        stderr(&foreign).contains("kernel KeyId mismatch"),
        "{}",
        stderr(&foreign)
    );
    for bad in ["zz".repeat(32), "ab".into()] {
        let output = fixture.retract("kernel.key", &bad);
        assert!(!output.status.success());
        assert!(
            stderr(&output).contains("target must"),
            "{}",
            stderr(&output)
        );
    }
    assert!(!fixture.retracted());
}
