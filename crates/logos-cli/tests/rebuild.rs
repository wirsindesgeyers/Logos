//! CLI real recebe somente chave pública e reconstrói cache descartável.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use logos_crypto::{FileSigner, Hash, Signer};
use logos_ledger::{EventBody, Ledger, Route};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    directory: PathBuf,
    signer: FileSigner,
}
impl Fixture {
    fn new(marker: u8) -> Self {
        let directory = std::env::temp_dir().join(format!(
            "logos-cli-index-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let signer = FileSigner::from_seed(&[0x11; 32]).unwrap();
        let mut ledger = Ledger::open(&directory, &signer).unwrap();
        ledger
            .append(EventBody::Genesis {
                ledger_version: 1,
                kernel_binary: [0; 32],
                kernel_key: signer.key_id().unwrap(),
            })
            .unwrap();
        ledger
            .append(EventBody::ObligationCreated {
                obligation: Hash::from_bytes([marker; 32]),
                route: Route::Model,
                artifact: [0; 32],
            })
            .unwrap();
        drop(ledger);
        Self { directory, signer }
    }
    fn key(&self) -> String {
        hex(self.signer.public_key().as_bytes())
    }
    fn run(&self) -> Output {
        run(&self.directory, &self.key())
    }
    fn cache(&self) -> PathBuf {
        self.directory.join("index.sqlite3")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.directory).unwrap();
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn run(path: &Path, key: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_logos"))
        .args(["ledger", "rebuild-index"])
        .arg(path)
        .args(["--public-key", key])
        .output()
        .unwrap()
}
fn success(output: Output, expected: &str) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("LEDGER_INDEX_REBUILT events=2 claims=1"),
        "{text}"
    );
    assert!(text.contains(expected), "{text}");
}

#[test]
fn rebuild_reports_missing_equal_different_and_invalid_without_private_key() {
    let fixture = Fixture::new(1);
    success(fixture.run(), "previous=Missing");
    success(fixture.run(), "previous=Equal");
    let other = Fixture::new(2);
    success(other.run(), "previous=Missing");
    fs::copy(other.cache(), fixture.cache()).unwrap();
    success(fixture.run(), "previous=Different");
    fs::write(fixture.cache(), b"corrupt SQLite").unwrap();
    success(fixture.run(), "previous=Invalid");
    success(fixture.run(), "previous=Equal");
}

#[test]
fn bad_key_arguments_and_storage_fail_with_nonzero_exit() {
    let fixture = Fixture::new(1);
    let wrong = FileSigner::from_seed(&[0x22; 32]).unwrap();
    for key in [
        "wrong".into(),
        "g".repeat(64),
        hex(wrong.public_key().as_bytes()),
    ] {
        let output = run(&fixture.directory, &key);
        assert!(!output.status.success());
        assert!(!output.stderr.is_empty());
    }
    let output = Command::new(env!("CARGO_BIN_EXE_logos"))
        .args(["ledger", "rebuild-index", "--unknown"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("usage:"));
    fs::create_dir(fixture.cache()).unwrap();
    let output = fixture.run();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not a regular file"));
}
