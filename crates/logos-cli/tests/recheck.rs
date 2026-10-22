//! `logos ledger revoke-checker` e `logos ledger recheck` (5.10): fiação da CLI.
//! O cenário com certificados reais está em logos-kernel/tests/certify.rs.
#![cfg(target_os = "linux")]
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use logos_crypto::{FileSigner, Signer};
use logos_kernel::registry::CheckerRegistry;
use logos_ledger::{EventBody, Ledger};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn cake() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/cake-lpr/cake_lpr"
    ))
}

struct Fixture {
    directory: PathBuf,
    builtin: String,
    cake: String,
}

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "logos-cli-recheck-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let key = directory.join("kernel.key");
        fs::write(&key, "11".repeat(32)).unwrap();
        fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
        let signer = FileSigner::load(&key).unwrap();
        let mut ledger = Ledger::open(&directory.join("ledger"), &signer).unwrap();
        ledger
            .append(EventBody::Genesis {
                ledger_version: 1,
                kernel_binary: [0; 32],
                kernel_key: signer.key_id().unwrap(),
            })
            .unwrap();
        let registry = CheckerRegistry::new(signer.public_key().clone(), Some(&cake())).unwrap();
        registry.install(&mut ledger, &signer).unwrap();
        Self {
            directory,
            builtin: registry.builtin_identity().to_hex(),
            cake: registry.cake_identity().unwrap().to_hex(),
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_logos"));
        command
            .args(["ledger", args[0]])
            .arg(self.directory.join("ledger"))
            .arg("--key")
            .arg(self.directory.join("kernel.key"))
            .args(&args[1..]);
        command.output().unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.directory).unwrap();
    }
}

fn text(output: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn operator_revokes_once_and_rechecks_with_the_pinned_cake() {
    let f = Fixture::new();
    let revoke =
        |checker: &str| f.run(&["revoke-checker", "--checker", checker, "--reason", "bug"]);
    let first = revoke(&f.builtin);
    assert!(first.status.success(), "{:?}", text(&first));
    assert!(text(&first).0.starts_with("{\"revoked\":\""));
    let again = revoke(&f.builtin);
    assert!(!again.status.success());
    assert!(
        text(&again).1.contains("already revoked"),
        "{:?}",
        text(&again)
    );
    let unknown = revoke(&"ab".repeat(32));
    assert!(
        text(&unknown).1.contains("never registered"),
        "{:?}",
        text(&unknown)
    );
    let cake = cake();
    let cake = cake.to_str().unwrap();
    let recheck = |with: &str| {
        f.run(&[
            "recheck",
            "--checker",
            &f.builtin,
            "--with",
            with,
            "--cake",
            cake,
        ])
    };
    // --with precisa ser o cake fixado; o checker Rust deste processo não é o do kernel.
    let wrong = recheck(&f.builtin);
    assert!(!wrong.status.success());
    assert!(
        text(&wrong).1.contains("--with must be"),
        "{:?}",
        text(&wrong)
    );
    let ok = recheck(&f.cake);
    assert!(ok.status.success(), "{:?}", text(&ok));
    assert_eq!(
        text(&ok).0.trim_end(),
        "{\"accepted\":0,\"rejected\":0,\"unknown\":0,\"rechecked\":[],\"invalidated\":[]}"
    );
    // Cake revogado não serve para recheck.
    assert!(revoke(&f.cake).status.success());
    let revoked = recheck(&f.cake);
    assert!(!revoked.status.success());
    assert!(text(&revoked).1.contains("revoked"), "{:?}", text(&revoked));
}
