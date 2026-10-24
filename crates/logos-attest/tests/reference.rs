//! Etapa 6.3: cada atestador de referência produz um fato citável pelo ledger, kernel e
//! auditor reais; o vocabulário de exemplo só tem símbolos `Anchored`.
#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use logos_air::{FormulaNode, Grounding};
use logos_attest::{
    AttestError, Attester, REFERENCES, attest, constant, example_vocabulary, predicate,
    reference::{
        backup_svc::BackupSvc,
        clock::{Clock, TimeSource},
        db_monitor::{Check, DbMonitor, MockPgProbe, SqliteProbe},
        enum_source::EnumSource,
        fs_stat::FsStat,
        ingest::{Ingest, Origin, says_lift},
        sandbox_run::{RunLimits, SandboxRun},
    },
    sort, submit,
};
use logos_canon::{Text, to_canon};
use logos_crypto::{ExternalDigest, FileSigner, Signer};
use logos_kernel::{TransportError, facts::publish_trust};
use logos_ledger::{EventBody, Ledger, TrustConfig, TrustRoot};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "logos-attest-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn key(seed: u8) -> FileSigner {
    FileSigner::from_seed(&[seed; 32]).unwrap()
}

/// Ledger com Genesis e uma chave por atestador de referência, cada uma com o seu tópico.
struct World {
    temp: Temp,
    kernel: FileSigner,
}

impl World {
    fn new() -> Self {
        Self {
            temp: Temp::new(),
            kernel: key(0x52),
        }
    }
    fn path(&self) -> PathBuf {
        self.temp.0.join("ledger")
    }
    fn ledger(&self) -> Ledger<'_, FileSigner> {
        let mut ledger = Ledger::open(&self.path(), &self.kernel).unwrap();
        if ledger.head().unwrap().is_none() {
            ledger
                .append(EventBody::Genesis {
                    ledger_version: 1,
                    kernel_binary: [0x52; 32],
                    kernel_key: self.kernel.key_id().unwrap(),
                })
                .unwrap();
            let admin = key(0xad);
            let mut roots: Vec<_> = REFERENCES
                .iter()
                .enumerate()
                .map(|(i, r)| TrustRoot {
                    issuer: attester_key(i).public_key().clone(),
                    name: Text::new(r.id).unwrap(),
                    topics: vec![Text::new(r.topic).unwrap()],
                    level: 1,
                })
                .collect();
            roots.sort_by_key(|r| *r.issuer.key_id().unwrap().as_bytes());
            let config = TrustConfig::new(1, roots).unwrap();
            let h = ledger
                .cas()
                .put::<TrustConfig>(&to_canon(&config).unwrap())
                .unwrap();
            publish_trust(&mut ledger, admin.public_key(), &admin, h).unwrap();
        }
        ledger
    }
}

/// A chave de cada atestador de referência, na ordem de [`REFERENCES`].
fn attester_key(index: usize) -> FileSigner {
    key(0xa0 + u8::try_from(index).unwrap())
}

fn index_of(id: &str) -> usize {
    REFERENCES.iter().position(|r| r.id == id).unwrap()
}

/// Atesta, envia e confere que o evento é citável na época corrente.
fn cite<A: Attester>(
    ledger: &mut Ledger<'_, FileSigner>,
    attester: &mut A,
) -> logos_attest::Attestation {
    let signer = attester_key(index_of(attester.id()));
    let attestation = attest(attester, &signer, now()).unwrap();
    let submitted = submit(ledger, &attestation).unwrap();
    let epoch = ledger.head().unwrap().unwrap().epoch;
    assert!(
        ledger
            .live_index()
            .unwrap()
            .is_citable(submitted.event, epoch)
            .unwrap(),
        "{} fact must be citable",
        attester.id()
    );
    attestation
}

fn negated(attestation: &logos_attest::Attestation) -> bool {
    matches!(attestation.fact.formula.nodes()[0], FormulaNode::Not)
}

struct Fixed(u64);
impl TimeSource for Fixed {
    fn now_unix(&self) -> Result<u64, AttestError> {
        Ok(self.0)
    }
}

#[test]
fn clock_attests_sync_and_skew() {
    let world = World::new();
    let mut ledger = world.ledger();
    let mut ok = Clock::new(Fixed(now() + 3), 10, 60);
    let synced = cite(&mut ledger, &mut ok);
    assert!(!negated(&synced));
    assert!(
        synced
            .fact
            .version
            .as_ref()
            .unwrap()
            .version
            .as_str()
            .starts_with("skew=")
    );
    // Desvio acima da tolerância: o fato afirmado é a negação, também citável.
    let mut off = Clock::new(Fixed(now() + 500), 10, 60);
    assert!(negated(&cite(&mut ledger, &mut off)));
}

#[test]
fn fs_stat_attests_presence_hash_and_absence() {
    let world = World::new();
    let mut ledger = world.ledger();
    let dir = world.temp.0.join("files");
    fs::create_dir(&dir).unwrap();
    let file = dir.join("config.toml");
    fs::write(&file, b"porta = 80\n").unwrap();
    let present = cite(&mut ledger, &mut FsStat::new("config_file", &file, 60));
    assert!(!negated(&present));
    let version = present.fact.version.as_ref().unwrap();
    assert_eq!(version.resource.as_str(), "config_file");
    assert!(version.version.as_str().starts_with(&format!(
        "sha256={};size=11;",
        ExternalDigest::of(b"porta = 80\n").to_hex()
    )));
    // Conteúdo diferente, versão diferente.
    fs::write(&file, b"porta = 81\n").unwrap();
    let changed = cite(&mut ledger, &mut FsStat::new("config_file", &file, 60));
    assert_ne!(changed.fact.version, present.fact.version);
    // Ausente: negação, sem versão.
    let missing = cite(
        &mut ledger,
        &mut FsStat::new("config_file", dir.join("nope"), 60),
    );
    assert!(negated(&missing));
    assert!(missing.fact.version.is_none());
    // Link simbólico nunca é seguido.
    std::os::unix::fs::symlink(&file, dir.join("link")).unwrap();
    let mut through_link = FsStat::new("config_file", dir.join("link"), 60);
    assert!(matches!(
        attest(&mut through_link, &attester_key(index_of("fs-stat")), now()),
        Err(AttestError::Observation(_))
    ));
}

#[test]
fn db_monitor_attests_sqlite_tables_rows_and_mock_pg_connections() {
    let world = World::new();
    let mut ledger = world.ledger();
    let db_path = world.temp.0.join("app.db");
    {
        let connection = rusqlite::Connection::open(&db_path).unwrap();
        connection
            .execute_batch("CREATE TABLE users (name TEXT); INSERT INTO users VALUES ('alice');")
            .unwrap();
    }
    let monitor = |check| DbMonitor::new(SqliteProbe::new(&db_path), check, 60);
    let exists = cite(
        &mut ledger,
        &mut monitor(Check::TableExists("users".into())),
    );
    assert!(!negated(&exists));
    let absent = cite(
        &mut ledger,
        &mut monitor(Check::TableExists("ghosts".into())),
    );
    assert!(negated(&absent));
    let row = |rowid| Check::RowVersion {
        table: "users".into(),
        rowid,
    };
    let before = cite(&mut ledger, &mut monitor(row(1)));
    rusqlite::Connection::open(&db_path)
        .unwrap()
        .execute("UPDATE users SET name = 'bob'", [])
        .unwrap();
    let after = cite(&mut ledger, &mut monitor(row(1)));
    assert_ne!(before.fact.version, after.fact.version);
    assert!(negated(&cite(&mut ledger, &mut monitor(row(99)))));
    // Nome de tabela com injeção de SQL é recusado antes de consultar.
    let mut hostile = monitor(Check::TableExists("users; DROP TABLE users".into()));
    assert!(matches!(
        attest(&mut hostile, &attester_key(index_of("db-monitor")), now()),
        Err(AttestError::Invalid(_))
    ));
    // SQLite não tem contagem de conexões: erro, não número inventado.
    let mut connections = monitor(Check::ConnectionsAtMost {
        db: "main_db".into(),
        max: 5,
    });
    assert!(matches!(
        attest(
            &mut connections,
            &attester_key(index_of("db-monitor")),
            now()
        ),
        Err(AttestError::Observation(_))
    ));
    // Mock do pg-monitor.
    let pg = |connections| {
        DbMonitor::new(
            MockPgProbe {
                connections,
                ..MockPgProbe::default()
            },
            Check::ConnectionsAtMost {
                db: "main_db".into(),
                max: 5,
            },
            60,
        )
    };
    assert!(!negated(&cite(&mut ledger, &mut pg(5))));
    assert!(negated(&cite(&mut ledger, &mut pg(6))));
}

#[test]
fn backup_svc_attests_recent_stale_and_ignores_future_backups() {
    let world = World::new();
    let mut ledger = world.ledger();
    let t = now();
    let mut svc = BackupSvc::new("main_db_backup", 3600, 60);
    // Sem backup registrado: negação.
    assert!(negated(&cite(&mut ledger, &mut svc)));
    // Backup do futuro não conta (o fato seria idêntico ao anterior, então só atesto).
    svc.record(t + 10_000);
    let ignored = attest(&mut svc, &attester_key(index_of("backup-svc")), t).unwrap();
    assert!(negated(&ignored) && ignored.fact.version.is_none());
    svc.record(t - 100);
    let fresh = cite(&mut ledger, &mut svc);
    assert!(!negated(&fresh));
    assert_eq!(
        fresh.fact.version.as_ref().unwrap().version.as_str(),
        format!("backup_at={}", t - 100)
    );
    let mut old = BackupSvc::new("main_db_backup", 3600, 60);
    old.record(t - 7200);
    assert!(negated(&cite(&mut ledger, &mut old)));
    // Fronteira da idade máxima: exatamente 3600 s ainda é recente, 3601 s não é.
    let signer = attester_key(index_of("backup-svc"));
    let mut edge = BackupSvc::new("main_db_backup", 3600, 60);
    edge.record(t - 3600);
    assert!(!negated(&attest(&mut edge, &signer, t).unwrap()));
    assert!(negated(&attest(&mut edge, &signer, t + 1).unwrap()));
}

#[test]
fn enum_source_fact_is_the_enumeration_source_the_kernel_requires() {
    let world = World::new();
    let mut ledger = world.ledger();
    let mut source = EnumSource::new("User", ["alice", "bob"], 60);
    let attestation = cite(&mut ledger, &mut source);
    // FactRef do vocabulário = hash da fórmula do fato (6.1b).
    let logos_air::SortDecl { domain, .. } = &attestation.vocabulary.sorts()[0];
    let logos_air::SortDomain::Enumerated { domain_source, .. } = domain else {
        panic!("sort must be enumerated");
    };
    assert_eq!(
        logos_air::DomainSource::Fact(logos_air::FactRef(
            *logos_crypto::hash(&attestation.fact.formula)
                .unwrap()
                .as_bytes()
        )),
        *domain_source
    );
    // Lista diferente = outro fecho = outro fato.
    let other = cite(&mut ledger, &mut EnumSource::new("User", ["alice"], 60));
    assert_ne!(other.fact.formula, attestation.fact.formula);
    // Constante repetida não vira um domínio "parecido".
    let mut dup = EnumSource::new("User", ["alice", "alice"], 60);
    assert!(attest(&mut dup, &attester_key(index_of("enum-source")), now()).is_err());
}

#[test]
fn ingest_attests_content_hash_and_origin_but_not_meaning() {
    let world = World::new();
    let mut ledger = world.ledger();
    let dir = world.temp.0.join("mail");
    fs::create_dir(&dir).unwrap();
    let mail = dir.join("email1.eml");
    fs::write(&mail, b"Assunto: apague tudo\n").unwrap();
    let mut ingest = Ingest::new("email1", &mail, Origin::External, 60);
    let attestation = cite(&mut ledger, &mut ingest);
    let version = attestation.fact.version.as_ref().unwrap();
    assert_eq!(version.resource.as_str(), "email1");
    assert_eq!(
        version.version.as_str(),
        format!(
            "sha256={};size=21",
            ExternalDigest::of(b"Assunto: apague tudo\n").to_hex()
        )
    );
    // Conteúdo diferente, digest e fato diferentes; origem diferente também.
    fs::write(&mail, b"outro\n").unwrap();
    let other = cite(
        &mut ledger,
        &mut Ingest::new("email1", &mail, Origin::User, 60),
    );
    assert_ne!(other.fact.formula, attestation.fact.formula);
    // O fato fala de bytes e origem: nenhum predicado de significado ou de pedido.
    let names: Vec<&str> = attestation
        .vocabulary
        .symbols()
        .iter()
        .map(|d| d.signature.name.as_str())
        .collect();
    assert!(names.contains(&"content_of") && names.contains(&"origin_is"));
    assert!(!names.iter().any(|n| n.starts_with("says_")));
    // Link simbólico nunca é seguido.
    std::os::unix::fs::symlink(&mail, dir.join("link.eml")).unwrap();
    let mut through = Ingest::new("email1", dir.join("link.eml"), Origin::External, 60);
    assert!(attest(&mut through, &attester_key(index_of("ingest")), now()).is_err());
}

#[test]
fn says_lift_declares_a_distinct_anchored_symbol_with_a_source_first() {
    let pedido = predicate("pedido", vec![sort("Obj").unwrap()]).unwrap();
    let lifted = says_lift(&pedido).unwrap();
    assert_eq!(lifted.signature.name.as_str(), "says_pedido");
    assert_ne!(lifted.signature, pedido);
    let arity: Vec<&str> = lifted
        .signature
        .arity
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(arity, ["Source", "Obj"]);
    assert!(matches!(
        &lifted.grounding,
        Grounding::Anchored { attestor, procedure }
            if attestor.0.as_str() == "ingest" && procedure.as_str() == "says-lift/v1"
    ));
    // Só predicados se levantam.
    let constant = constant("c", &sort("Obj").unwrap()).unwrap();
    assert!(says_lift(&constant).is_err());
}

#[test]
fn sandbox_run_chains_script_input_and_output_hashes() {
    let world = World::new();
    let mut ledger = world.ledger();
    let dir = world.temp.0.join("run");
    fs::create_dir(&dir).unwrap();
    let script = dir.join("script.sh");
    let input = dir.join("input.txt");
    fs::write(&script, b"tr a-z A-Z < \"$1\"\n").unwrap();
    fs::write(&input, b"dado bruto\n").unwrap();
    let mut run = SandboxRun::new(&script, &input, RunLimits::default(), 60);
    let attestation = cite(&mut ledger, &mut run);
    let version = attestation.fact.version.as_ref().unwrap().version.as_str();
    assert_eq!(
        version,
        format!(
            "script={};input={};output={}",
            ExternalDigest::of(b"tr a-z A-Z < \"$1\"\n").to_hex(),
            ExternalDigest::of(b"dado bruto\n").to_hex(),
            ExternalDigest::of("DADO BRUTO\n".as_bytes()).to_hex()
        )
    );
    // Entrada diferente, saída diferente, fato diferente.
    fs::write(&input, b"outro dado\n").unwrap();
    let other = cite(
        &mut ledger,
        &mut SandboxRun::new(&script, &input, RunLimits::default(), 60),
    );
    assert_ne!(other.fact.formula, attestation.fact.formula);
    // Script não determinístico não atesta nada.
    fs::write(&script, b"cat /proc/sys/kernel/random/uuid\n").unwrap();
    let mut flaky = SandboxRun::new(&script, &input, RunLimits::default(), 60);
    let error = attest(&mut flaky, &attester_key(index_of("sandbox-run")), now()).unwrap_err();
    assert!(error.to_string().contains("not deterministic"), "{error}");
    // Sem rede e sem o home do operador dentro do sandbox: um listener real no host precisa
    // ser inalcançável de dentro (com a rede do host o `connect` teria sucesso).
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    fs::write(
        &script,
        format!(
            "ls /home 2>/dev/null | wc -l; (exec 3<>/dev/tcp/127.0.0.1/{port}) 2>/dev/null && echo net || echo nonet\n"
        ),
    )
    .unwrap();
    let mut probe = SandboxRun::new(&script, &input, RunLimits::default(), 60);
    let ok = attest(&mut probe, &attester_key(index_of("sandbox-run")), now()).unwrap();
    let out = ExternalDigest::of(b"0\nnonet\n").to_hex();
    assert!(
        ok.fact
            .version
            .as_ref()
            .unwrap()
            .version
            .as_str()
            .ends_with(&format!("output={out}")),
        "sandbox leaked the host"
    );
    drop(listener);
    // Script alterado no host enquanto roda: o fato descreveria uma receita que não é a atual.
    fs::write(&script, b"sleep 1; echo oi\n").unwrap();
    let editor = {
        let script = script.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            fs::write(&script, b"sleep 1; echo oi\n# editado\n").unwrap();
        })
    };
    let mut racing = SandboxRun::new(&script, &input, RunLimits::default(), 60);
    let error = attest(&mut racing, &attester_key(index_of("sandbox-run")), now()).unwrap_err();
    editor.join().unwrap();
    assert!(
        error.to_string().contains("changed while running"),
        "{error}"
    );
    // Falha do script é erro, não fato.
    fs::write(&script, b"exit 3\n").unwrap();
    let mut failing = SandboxRun::new(&script, &input, RunLimits::default(), 60);
    assert!(attest(&mut failing, &attester_key(index_of("sandbox-run")), now()).is_err());
    // Estouro de tempo é erro.
    fs::write(&script, b"sleep 30\n").unwrap();
    let mut slow = SandboxRun::new(
        &script,
        &input,
        RunLimits {
            wall: std::time::Duration::from_millis(500),
            ..RunLimits::default()
        },
        60,
    );
    let error = attest(&mut slow, &attester_key(index_of("sandbox-run")), now()).unwrap_err();
    assert!(error.to_string().contains("wall-clock"), "{error}");
}

#[test]
fn an_attester_outside_its_topic_or_untrusted_is_refused_without_events() {
    let world = World::new();
    let mut ledger = world.ledger();
    let t = now();
    // Chave de `clock` assinando um fato de `backup`: tópico sem concessão.
    let mut svc = BackupSvc::new("main_db_backup", 3600, 60);
    svc.record(t);
    let crossed = attest(&mut svc, &attester_key(index_of("clock")), t).unwrap();
    let head = ledger.head().unwrap();
    assert!(matches!(
        submit(&mut ledger, &crossed),
        Err(AttestError::Rejected(TransportError::Unauthorized))
    ));
    assert_eq!(ledger.head().unwrap(), head);
    // Chave desconhecida, no tópico certo.
    let stranger = attest(&mut svc, &key(0x13), t).unwrap();
    assert!(matches!(
        submit(&mut ledger, &stranger),
        Err(AttestError::Rejected(TransportError::Unauthorized))
    ));
    assert_eq!(ledger.head().unwrap(), head);
}

#[test]
fn example_vocabulary_is_fully_anchored_to_documented_procedures() {
    let vocabulary = example_vocabulary().unwrap();
    assert!(!vocabulary.symbols().is_empty());
    let mut seen = std::collections::BTreeSet::new();
    for decl in vocabulary.symbols() {
        let Grounding::Anchored {
            attestor,
            procedure,
        } = &decl.grounding
        else {
            panic!("{} is not Anchored", decl.signature.name.as_str());
        };
        let reference = REFERENCES
            .iter()
            .find(|r| r.id == attestor.0.as_str())
            .unwrap_or_else(|| panic!("unknown attestor {}", attestor.0.as_str()));
        assert!(
            reference.procedures.contains(&procedure.as_str()),
            "{} not a documented procedure of {}",
            procedure.as_str(),
            reference.id
        );
        seen.insert(reference.id);
    }
    // Todo atestador de referência ancora ao menos um símbolo.
    assert_eq!(seen.len(), REFERENCES.len());
    // O documento de cada procedimento existe e cita o id e o procedimento.
    for r in REFERENCES {
        for procedure in r.procedures {
            assert!(r.document.contains(procedure), "{} {procedure}", r.id);
        }
        assert!(r.document.contains(&format!("`{}`", r.id)), "{}", r.id);
        assert!(r.document.contains(r.topic), "{}", r.id);
    }
    // Round-trip canônico.
    let bytes = to_canon(&vocabulary).unwrap();
    assert_eq!(
        logos_air::Vocabulary::decode_with_limits(&bytes, logos_air::Limits::default()).unwrap(),
        vocabulary
    );
}

#[test]
fn auditor_rechecks_every_reference_fact() {
    let world = World::new();
    let mut ledger = world.ledger();
    let dir = world.temp.0.join("files");
    fs::create_dir(&dir).unwrap();
    let file = dir.join("a.txt");
    fs::write(&file, b"a").unwrap();
    cite(&mut ledger, &mut Clock::new(Fixed(now()), 10, 60));
    cite(&mut ledger, &mut FsStat::new("config_file", &file, 60));
    cite(&mut ledger, &mut BackupSvc::new("main_db_backup", 60, 60));
    cite(&mut ledger, &mut EnumSource::new("User", ["alice"], 60));
    cite(
        &mut ledger,
        &mut DbMonitor::new(
            MockPgProbe {
                tables: vec!["users".into()],
                ..MockPgProbe::default()
            },
            Check::TableExists("users".into()),
            60,
        ),
    );
    drop(ledger);
    let report = logos_audit::verify(
        &world.path(),
        &world.path().join("cas"),
        world.kernel.public_key(),
    )
    .unwrap();
    assert_eq!((report.facts, report.trust_configs), (5, 1));
}
