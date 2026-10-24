//! `logos ledger trust-publish|fact-register|revoke-issuer`: o operador publica confiança,
//! registra fatos de atestadores e revoga emissores com a chave do kernel; a TCB decide.
#![cfg(target_os = "linux")]
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use logos_attest::{
    Attester, attest,
    reference::clock::{Clock, SystemClock},
};
use logos_canon::to_canon;
use logos_crypto::{FileSigner, Signer};
use logos_ledger::{EventBody, Ledger};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    directory: PathBuf,
    attester: FileSigner,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "logos-cli-facts-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("kernel.key");
        fs::write(&path, "11".repeat(32)).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let signer = FileSigner::load(&path).unwrap();
        let mut ledger = Ledger::open(&directory.join("ledger"), &signer).unwrap();
        ledger
            .append(EventBody::Genesis {
                ledger_version: 1,
                kernel_binary: [0; 32],
                kernel_key: signer.key_id().unwrap(),
            })
            .unwrap();
        Self {
            directory,
            attester: FileSigner::from_seed(&[0xa1; 32]).unwrap(),
        }
    }

    fn ledger(&self) -> PathBuf {
        self.directory.join("ledger")
    }

    fn logos(&self, command: &str, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_logos"))
            .args(["ledger", command])
            .arg(self.ledger())
            .arg("--key")
            .arg(self.directory.join("kernel.key"))
            .args(args)
            .output()
            .unwrap()
    }

    fn trust(&self, version: &str, topics: &str) -> Output {
        let root = format!(
            "{}:clock:{topics}",
            hex(self.attester.public_key().as_bytes())
        );
        self.logos("trust-publish", &["--version", version, "--root", &root])
    }

    /// Fato do `clock` com `observed_at` = `at`, em arquivos para o `fact-register`.
    fn fact_files(&self, signer: &FileSigner, at: u64) -> (PathBuf, PathBuf) {
        let mut clock = Clock::new(SystemClock, 10, 3600);
        assert_eq!(clock.id(), "clock");
        let attestation = attest(&mut clock, signer, at).unwrap();
        let fact = self.directory.join(format!("fact-{at}.bin"));
        let vocabulary = self.directory.join(format!("vocabulary-{at}.bin"));
        fs::write(&fact, to_canon(&attestation.fact).unwrap()).unwrap();
        fs::write(&vocabulary, to_canon(&attestation.vocabulary).unwrap()).unwrap();
        (fact, vocabulary)
    }

    fn register(&self, signer: &FileSigner, at: u64) -> Output {
        let (fact, vocabulary) = self.fact_files(signer, at);
        self.logos(
            "fact-register",
            &[
                "--fact",
                fact.to_str().unwrap(),
                "--vocabulary",
                vocabulary.to_str().unwrap(),
            ],
        )
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

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[test]
fn operator_publishes_trust_registers_a_fact_and_revokes_the_issuer() {
    let fixture = Fixture::new();
    // Sem configuração de confiança, ninguém atesta.
    let early = fixture.register(&fixture.attester, now());
    assert!(!early.status.success());
    assert!(
        stderr(&early).contains("unauthorized"),
        "{}",
        stderr(&early)
    );
    // Configuração publicada: o atestador registra no tópico concedido.
    let trust = fixture.trust("1", "time");
    assert!(trust.status.success(), "{}", stderr(&trust));
    assert!(
        String::from_utf8_lossy(&trust.stdout).starts_with("{\"config\":\""),
        "{}",
        String::from_utf8_lossy(&trust.stdout)
    );
    let at = now();
    let registered = fixture.register(&fixture.attester, at);
    assert!(registered.status.success(), "{}", stderr(&registered));
    let text = String::from_utf8(registered.stdout).unwrap();
    assert!(text.starts_with("{\"fact\":\""), "{text}");
    // O mesmo fato duas vezes não registra duas vezes.
    let again = fixture.register(&fixture.attester, at);
    assert!(!again.status.success());
    assert!(
        stderr(&again).contains("already registered"),
        "{}",
        stderr(&again)
    );
    // Versão não crescente é recusada.
    let stale = fixture.trust("1", "time");
    assert!(!stale.status.success());
    assert!(
        stderr(&stale).contains("not increasing"),
        "{}",
        stderr(&stale)
    );
    // Revogar o emissor bloqueia fatos novos dele.
    let issuer = hex(fixture.attester.key_id().unwrap().as_bytes());
    let revoked = fixture.logos("revoke-issuer", &["--issuer", &issuer, "--since", "0"]);
    assert!(revoked.status.success(), "{}", stderr(&revoked));
    assert!(
        String::from_utf8_lossy(&revoked.stdout).starts_with("{\"revoked\":\""),
        "{}",
        String::from_utf8_lossy(&revoked.stdout)
    );
    let after = fixture.register(&fixture.attester, at + 1);
    assert!(!after.status.success());
    assert!(
        stderr(&after).contains("unauthorized"),
        "{}",
        stderr(&after)
    );
    let twice = fixture.logos("revoke-issuer", &["--issuer", &issuer, "--since", "0"]);
    assert!(!twice.status.success());
    assert!(
        stderr(&twice).contains("already revoked"),
        "{}",
        stderr(&twice)
    );
}

#[test]
fn malformed_arguments_and_foreign_topics_are_refused() {
    let fixture = Fixture::new();
    // Tópico não concedido.
    assert!(fixture.trust("1", "fs").status.success());
    let crossed = fixture.register(&fixture.attester, now());
    assert!(!crossed.status.success());
    assert!(
        stderr(&crossed).contains("unauthorized"),
        "{}",
        stderr(&crossed)
    );
    // Chave do kernel errada: o ledger nem abre.
    let other = fixture.directory.join("other.key");
    fs::write(&other, "22".repeat(32)).unwrap();
    fs::set_permissions(&other, fs::Permissions::from_mode(0o600)).unwrap();
    let foreign = Command::new(env!("CARGO_BIN_EXE_logos"))
        .args(["ledger", "revoke-issuer"])
        .arg(fixture.ledger())
        .arg("--key")
        .arg(&other)
        .args(["--issuer", &"ab".repeat(32), "--since", "0"])
        .output()
        .unwrap();
    assert!(!foreign.status.success());
    assert!(
        stderr(&foreign).contains("kernel KeyId mismatch"),
        "{}",
        stderr(&foreign)
    );
    // Hex ruim, flag desconhecida e flag repetida onde só cabe uma.
    for args in [
        vec!["--issuer", "zz", "--since", "0"],
        vec!["--issuer", &"ab".repeat(32), "--since", "0", "--bogus", "1"],
        vec!["--issuer", &"ab".repeat(32), "--since", "0", "--since", "1"],
        vec!["--issuer", &"ab".repeat(32)],
    ] {
        let output = fixture.logos("revoke-issuer", &args);
        assert!(!output.status.success(), "{args:?}");
    }
    // Raiz malformada.
    let bad = fixture.logos("trust-publish", &["--version", "2", "--root", "nope"]);
    assert!(!bad.status.success());
    assert!(stderr(&bad).contains("--root must be"), "{}", stderr(&bad));
}
