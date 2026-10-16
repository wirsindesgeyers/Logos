//! Processos reais: diagnóstico de objetos opacos, sem autoridade lógica ou remoção.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::SystemTime;

use logos_canon::{Array, Map, Value, to_canon};
use logos_crypto::{Domain, FileSigner, Signer, tags};
use logos_ledger::{CasReference, EventBody, Ledger, Route};

static NEXT: AtomicU64 = AtomicU64::new(0);
// Um spawn em outro teste pode herdar o flock até exec/close, mesmo com
// CLOEXEC. Não sobrepor subprocessos de uma fixture ao drop/reopen de outra.
static FIXTURES: Mutex<()> = Mutex::new(());
struct Policy;
impl Domain for Policy {
    const TAG: &'static [u8] = tags::POLICY;
}
struct Fixture {
    _guard: MutexGuard<'static, ()>,
    root: PathBuf,
    signer: FileSigner,
    obligation: CasReference,
    orphan: CasReference,
}
fn label(reference: CasReference) -> PathBuf {
    let hex: String = reference
        .digest()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Path::new(reference.directory())
        .join(&hex[..2])
        .join(&hex[2..])
}
impl Fixture {
    fn new() -> Self {
        let guard = FIXTURES.lock().unwrap();
        let root = std::env::temp_dir().join(format!(
            "logos-cli-gc-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let signer = FileSigner::from_seed(&[0x11; 32]).unwrap();
        let mut ledger = Ledger::open(&root, &signer).unwrap();
        ledger
            .append(EventBody::Genesis {
                ledger_version: 1,
                kernel_binary: [0; 32],
                kernel_key: signer.key_id().unwrap(),
            })
            .unwrap();
        let cas = ledger.cas();
        let vocabulary = CasReference::Vocabulary(cas.put(&[0x80]).unwrap());
        let checker = CasReference::Checker(cas.put(b"raw checker, not CBOR").unwrap());
        // Hash em chave de mapa e em array aninhado: retenção conservadora.
        let body = Value::Map(
            Map::new(vec![(
                Value::Bytes(checker.digest().to_vec()),
                Value::Array(Array::new(vec![Value::Bytes(vocabulary.digest().to_vec())]).unwrap()),
            )])
            .unwrap(),
        );
        let obligation = cas.put(&to_canon(&body).unwrap()).unwrap();
        let orphan = CasReference::Certificate(cas.put(b"unreferenced").unwrap());
        cas.put::<Policy>(&[0x80]).unwrap();
        ledger
            .append(EventBody::ObligationCreated {
                obligation,
                route: Route::Model,
                artifact: [0; 32],
            })
            .unwrap();
        drop(ledger);
        Self {
            _guard: guard,
            root,
            signer,
            obligation: CasReference::Obligation(obligation),
            orphan,
        }
    }
    fn key(&self) -> String {
        self.signer
            .public_key()
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
    fn path(&self, reference: CasReference) -> PathBuf {
        self.root.join("cas").join(label(reference))
    }
    fn run(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_logos"))
            .args(["ledger", "gc", "--dry-run"])
            .arg(&self.root)
            .args(["--public-key", &self.key()])
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

type Tree = BTreeMap<PathBuf, (Vec<u8>, SystemTime)>;
fn snapshot(root: &Path) -> Tree {
    fn walk(root: &Path, path: &Path, result: &mut Tree) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            // SQLite read-only pode criar/manter seus sidecars de coordenação.
            if path == root.join("ledger.sqlite3-wal") || path == root.join("ledger.sqlite3-shm") {
                continue;
            }
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.is_dir() {
                walk(root, &path, result);
            } else {
                let bytes = if metadata.is_symlink() {
                    format!("{:?}", fs::read_link(&path).unwrap()).into_bytes()
                } else {
                    fs::read(&path).unwrap()
                };
                result.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    (bytes, metadata.modified().unwrap()),
                );
            }
        }
    }
    let mut result = Tree::new();
    walk(root, root, &mut result);
    result
}
fn failure(output: Output) {
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "no partial orphan report on failure"
    );
    assert!(!output.stderr.is_empty());
}

fn unchanged(root: &Path, before: &Tree) {
    let after = snapshot(root);
    assert_eq!(
        after.keys().collect::<Vec<_>>(),
        before.keys().collect::<Vec<_>>()
    );
    for (path, (bytes, modified)) in before {
        let (actual, actual_modified) = &after[path];
        assert!(actual == bytes, "content changed: {}", path.display());
        assert_eq!(
            actual_modified,
            modified,
            "mtime changed: {}",
            path.display()
        );
    }
}

#[test]
fn dry_run_lists_candidates_retains_transitive_links_and_preserves_published_files() {
    let fixture = Fixture::new();
    let temporary = fixture
        .path(fixture.orphan)
        .parent()
        .unwrap()
        .join(".tmp.999.0");
    fs::write(&temporary, b"partial object").unwrap();
    let before = snapshot(&fixture.root);
    let output = fixture.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains(
            "events=2 objects=5 referenced=1 retained=3 orphan_candidates=2 temporaries=1"
        ),
        "{text}"
    );
    assert!(text.contains(&format!("ORPHAN {}", label(fixture.orphan).display())));
    assert!(text.contains("ORPHAN policy-v1/"));
    assert!(
        !text.contains("ORPHAN vocab-v1/")
            && !text.contains("ORPHAN checker-v1/")
            && !text.contains("ORPHAN obligation-v1/")
    );
    assert!(text.contains("TEMP ") && text.contains(".tmp.999.0"));
    unchanged(&fixture.root, &before);
    let again = fixture.run();
    assert_eq!(again.stdout, text.as_bytes());
    unchanged(&fixture.root, &before);
}

#[test]
fn missing_or_corrupt_roots_and_corrupt_orphans_abort_without_partial_results() {
    for kind in 0..3 {
        let fixture = Fixture::new();
        match kind {
            0 => fs::remove_file(fixture.path(fixture.obligation)).unwrap(),
            1 => fs::write(fixture.path(fixture.obligation), b"corrupt").unwrap(),
            _ => fs::write(fixture.path(fixture.orphan), b"corrupt").unwrap(),
        }
        let before = snapshot(&fixture.root);
        failure(fixture.run());
        unchanged(&fixture.root, &before);
    }
}

#[test]
fn reachable_noncanonical_payload_is_error_instead_of_empty_graph() {
    let fixture = Fixture::new();
    let mut ledger = Ledger::open(&fixture.root, &fixture.signer).unwrap();
    let obligation = ledger.cas().put(b"not canonical").unwrap();
    ledger
        .append(EventBody::ObligationCreated {
            obligation,
            route: Route::Model,
            artifact: [0; 32],
        })
        .unwrap();
    drop(ledger);
    let before = snapshot(&fixture.root);
    failure(fixture.run());
    unchanged(&fixture.root, &before);
}

#[test]
fn symlinks_at_every_level_unknown_layout_and_special_names_are_refused() {
    for level in 0..7 {
        let fixture = Fixture::new();
        if level < 4 {
            let object = fixture.path(fixture.orphan);
            let path = match level {
                0 => object.clone(),
                1 => object.parent().unwrap().to_path_buf(),
                2 => fixture.root.join("cas/certificate-v1"),
                _ => fixture.root.join("cas"),
            };
            let moved = fixture.root.join("moved");
            fs::rename(&path, &moved).unwrap();
            std::os::unix::fs::symlink(&moved, &path).unwrap();
        } else {
            match level {
                4 => fs::write(fixture.root.join("cas/unknown"), b"unrecognized").unwrap(),
                5 => fs::write(
                    fixture
                        .path(fixture.orphan)
                        .parent()
                        .unwrap()
                        .join(".tmp.bad.0"),
                    b"bad name",
                )
                .unwrap(),
                _ => {
                    use std::os::unix::ffi::OsStringExt;
                    fs::create_dir(
                        fixture
                            .root
                            .join("cas")
                            .join(std::ffi::OsString::from_vec(vec![0xff])),
                    )
                    .unwrap();
                }
            }
        }
        let before = snapshot(&fixture.root);
        failure(fixture.run());
        unchanged(&fixture.root, &before);
    }
}

#[test]
fn dry_run_is_mandatory_and_no_destructive_flag_is_accepted() {
    let fixture = Fixture::new();
    let before = snapshot(&fixture.root);
    for flags in [
        vec![],
        vec!["--delete"],
        vec!["--force"],
        vec!["--dry-run", "--delete"],
        vec!["--dry-run", "--dry-run"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_logos"))
            .args(["ledger", "gc"])
            .args(flags)
            .arg(&fixture.root)
            .args(["--public-key", &fixture.key()])
            .output()
            .unwrap();
        failure(output);
        unchanged(&fixture.root, &before);
    }
}

#[test]
fn wrong_key_locked_writer_and_tampered_history_are_refused() {
    let fixture = Fixture::new();
    let output = Command::new(env!("CARGO_BIN_EXE_logos"))
        .args(["ledger", "gc", "--dry-run"])
        .arg(&fixture.root)
        .args(["--public-key", &"22".repeat(32)])
        .output()
        .unwrap();
    failure(output);
    let ledger = Ledger::open(&fixture.root, &fixture.signer).unwrap();
    failure(fixture.run());
    drop(ledger);
    // Fonte adulterada, sem usar índices/cache como autoridade.
    let database = fixture.root.join("ledger.sqlite3");
    let bytes = fs::read(&database).unwrap();
    fs::write(&database, b"not SQLite").unwrap();
    let before = snapshot(&fixture.root);
    failure(fixture.run());
    unchanged(&fixture.root, &before);
    fs::write(database, bytes).unwrap();
}
