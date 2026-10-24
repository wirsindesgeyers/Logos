//! `logos user init` e `logos ledger user-trust` (6.5): chave do usuário local e raiz de
//! confiança com os tópicos `assume`, `authorize` e `confirm`, preservando as demais raízes.
#![cfg(target_os = "linux")]
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use logos_crypto::{FileSigner, Signer};
use logos_ledger::{EventBody, Ledger};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "logos-cli-user-{}-{}",
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
        Self(directory)
    }

    fn logos(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_logos"))
            .args(args)
            .output()
            .unwrap()
    }

    fn ledger_command(&self, command: &str, extra: &[&str]) -> Output {
        let ledger = self.0.join("ledger");
        let key = self.0.join("kernel.key");
        let mut args = vec![
            "ledger",
            command,
            ledger.to_str().unwrap(),
            "--key",
            key.to_str().unwrap(),
        ];
        args.extend_from_slice(extra);
        self.logos(&args)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn field(json: &str, name: &str) -> String {
    let marker = format!("\"{name}\":\"");
    let start = json.find(&marker).unwrap() + marker.len();
    json[start..start + json[start..].find('"').unwrap()].to_owned()
}

#[test]
fn init_creates_a_private_key_once_and_never_overwrites_or_follows_links() {
    let fixture = Fixture::new();
    let key = fixture.0.join("user.key");
    let output = fixture.logos(&["user", "init", "--key", key.to_str().unwrap()]);
    assert!(output.status.success(), "{}", stderr(&output));
    let json = String::from_utf8(output.stdout).unwrap();
    let mode = fs::metadata(&key).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    // O arquivo é carregável pelo mesmo código do kernel e a chave pública impressa é a dele.
    let signer = FileSigner::load(&key).unwrap();
    let public: String = signer
        .public_key()
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(field(&json, "public_key"), public);
    // Duas chaves novas nunca coincidem.
    let other = fixture.0.join("other.key");
    let second = fixture.logos(&["user", "init", "--key", other.to_str().unwrap()]);
    assert_ne!(
        field(&String::from_utf8(second.stdout).unwrap(), "public_key"),
        public
    );
    // Não sobrescreve a chave existente nem segue link simbólico.
    let before = fs::read(&key).unwrap();
    let again = fixture.logos(&["user", "init", "--key", key.to_str().unwrap()]);
    assert!(!again.status.success());
    assert_eq!(fs::read(&key).unwrap(), before);
    let link = fixture.0.join("link.key");
    let victim = fixture.0.join("victim");
    symlink(&victim, &link).unwrap();
    let through = fixture.logos(&["user", "init", "--key", link.to_str().unwrap()]);
    assert!(!through.status.success());
    assert!(!victim.exists());
}

#[test]
fn user_becomes_a_trust_root_with_three_topics_and_keeps_existing_roots() {
    let fixture = Fixture::new();
    let key = fixture.0.join("user.key");
    let init = fixture.logos(&["user", "init", "--key", key.to_str().unwrap()]);
    let user_public = field(&String::from_utf8(init.stdout).unwrap(), "public_key");
    // Já existe uma raiz de atestador (versão 1).
    let attester = FileSigner::from_seed(&[0xa1; 32]).unwrap();
    let attester_hex: String = attester
        .public_key()
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let root = format!("{attester_hex}:clock:time");
    assert!(
        fixture
            .ledger_command("trust-publish", &["--version", "1", "--root", &root])
            .status
            .success()
    );
    let registered = fixture.ledger_command("user-trust", &["--user-public", &user_public]);
    assert!(registered.status.success(), "{}", stderr(&registered));
    assert!(
        String::from_utf8_lossy(&registered.stdout).contains("\"version\":2"),
        "{}",
        String::from_utf8_lossy(&registered.stdout)
    );
    let kernel = FileSigner::load(&fixture.0.join("kernel.key")).unwrap();
    let ledger = Ledger::open(&fixture.0.join("ledger"), &kernel).unwrap();
    let index = ledger.live_index().unwrap();
    let user = FileSigner::load(&key).unwrap().key_id().unwrap();
    for topic in ["assume", "authorize", "confirm"] {
        assert!(
            index.trust_grant(&user, topic).unwrap().is_some(),
            "{topic}"
        );
    }
    // Outro tópico não, e a raiz do atestador continua valendo.
    assert!(index.trust_grant(&user, "time").unwrap().is_none());
    assert!(
        index
            .trust_grant(&attester.key_id().unwrap(), "time")
            .unwrap()
            .is_some()
    );
    assert!(
        index
            .trust_grant(&attester.key_id().unwrap(), "authorize")
            .unwrap()
            .is_none()
    );
    drop(index);
    drop(ledger);
    // Registrar duas vezes ou com chave malformada é recusado.
    let twice = fixture.ledger_command("user-trust", &["--user-public", &user_public]);
    assert!(!twice.status.success());
    assert!(
        stderr(&twice).contains("already a trust root"),
        "{}",
        stderr(&twice)
    );
    let bad = fixture.ledger_command("user-trust", &["--user-public", "zz"]);
    assert!(!bad.status.success());
}

#[test]
fn first_trust_config_can_be_the_users() {
    let fixture = Fixture::new();
    let key = fixture.0.join("user.key");
    let init = fixture.logos(&["user", "init", "--key", key.to_str().unwrap()]);
    let user_public = field(&String::from_utf8(init.stdout).unwrap(), "public_key");
    let registered = fixture.ledger_command("user-trust", &["--user-public", &user_public]);
    assert!(registered.status.success(), "{}", stderr(&registered));
    assert!(String::from_utf8_lossy(&registered.stdout).contains("\"version\":1"));
}
