//! Fixture assinada de escala; INSERTs agrupados preparam dados, não medem fsync.
use std::error::Error;
use std::fs;
use std::path::Path;
use std::time::Instant;

use logos_air::{Formula, FormulaNode, Limits as AirLimits, Vocabulary};
use logos_canon::{Text, to_canon};
use logos_core::{Budget, CheckOutcome, Exhausted};
use logos_crypto::{FileSigner, Hash, Signer, hash};
use logos_ledger::{
    Cas, CheckRecord, Consistency, Event, EventBody, EventProposal, Ledger, LedgerIndex, Limits,
    Route, TaintedText,
};
use logos_obligation::{PremiseRef, Proposal, generate};
use rusqlite::{Connection, params};

fn formula() -> Result<Formula, Box<dyn Error>> {
    Ok(Formula::new(vec![FormulaNode::True], AirLimits::default())?)
}

fn prepare(root: &Path, total: u64, signer: &FileSigner) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(root)?;
    if root.join("ledger.sqlite3").is_file() {
        let existing = Connection::open_with_flags(
            root.join("ledger.sqlite3"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let count: i64 = existing.query_row("SELECT count(*) FROM events", [], |r| r.get(0))?;
        if count == total as i64 {
            return Ok(());
        } // O Index abaixo ainda confere todos os eventos.
    }
    let cas = Cas::open(root.join("cas"), Limits::default())?;
    let vocabulary = Vocabulary::new(1, vec![], vec![])?;
    let hv = cas.put::<Vocabulary>(&to_canon(&vocabulary)?)?;
    let goal = formula()?;
    let mut ledger = Ledger::open(root, signer)?;
    if ledger.head()?.is_none() {
        ledger.append(EventBody::Genesis {
            ledger_version: 1,
            kernel_binary: [0x10; 32],
            kernel_key: signer.key_id()?,
        })?;
        for _ in 0..2 {
            ledger.append(EventBody::AssumptionDeclared {
                vocabulary: hv,
                formula: goal.clone(),
                scope: Text::new("scale")?,
                author: signer.key_id()?,
            })?;
        }
    }
    let count = ledger.head()?.ok_or("missing prefix")?.seq + 1;
    if count == total {
        return Ok(());
    }
    if count != 3 {
        return Err("unexpected scale fixture size; refusing to overwrite".into());
    }
    let database = root.join("ledger.sqlite3");
    let read = Connection::open(&database)?;
    let first = Event::decode(&read.query_row::<Vec<u8>, _, _>(
        "SELECT wire FROM events WHERE seq=1",
        [],
        |r| r.get(0),
    )?)?;
    let second = Event::decode(&read.query_row::<Vec<u8>, _, _>(
        "SELECT wire FROM events WHERE seq=2",
        [],
        |r| r.get(0),
    )?)?;
    let mut refs = [hash(&first)?, hash(&second)?];
    refs.sort_unstable();
    let index = ledger.index()?;
    let premise_refs: Vec<_> = refs
        .iter()
        .map(|h| PremiseRef::from_event_hash(*h.as_bytes()))
        .collect();
    let snapshot = index.snapshot(&cas, &premise_refs, "scale")?;
    let obligation = generate(
        Proposal {
            vocabulary: &vocabulary,
            goal: &goal,
            cites: &premise_refs,
        },
        &snapshot,
    )?;
    let ho = cas.put(&to_canon(&obligation)?)?;
    drop(snapshot);
    drop(index);
    let mut previous = ledger.head()?.map(|h| h.hash);
    drop(ledger);
    drop(read);
    let mut connection = Connection::open(&database)?;
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL")?;
    let transaction =
        connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    {
        let mut insert = transaction.prepare("INSERT INTO events(seq,wire) VALUES(?1,?2)")?;
        for seq in 3..total {
            let check = CheckRecord {
                obligation: ho,
                certificate: Hash::from_bytes([0x51; 32]),
                checker: Hash::from_bytes([0x52; 32]),
                budget: Budget {
                    steps: 100,
                    mem_bytes: 1024,
                    size_bytes: 1024,
                    wall_ms: 0,
                },
                outcome: CheckOutcome::Accepted,
            };
            let body = match seq % 5 {
                0 => EventBody::AssumptionDeclared {
                    vocabulary: hv,
                    formula: goal.clone(),
                    scope: Text::new("scale")?,
                    author: signer.key_id()?,
                },
                1 => EventBody::Proposed {
                    proposal: EventProposal {
                        text_pt: TaintedText::new("fixture pública de escala".into())?,
                        goal: goal.clone(),
                        cites: refs.to_vec(),
                        speculative_on: vec![],
                        vocabulary: hv,
                    },
                    author: signer.key_id()?,
                },
                2 => EventBody::ObligationCreated {
                    obligation: ho,
                    route: Route::Model,
                    artifact: [0x20; 32],
                },
                3 => EventBody::UnknownRecorded {
                    check: CheckRecord {
                        outcome: CheckOutcome::Unknown(Exhausted::Steps),
                        ..check
                    },
                    machine: Text::new("scale")?,
                },
                _ => EventBody::Verified {
                    check,
                    cited: refs.to_vec(),
                    used: vec![refs[0]],
                    consistency: Consistency::Unknown { check: None },
                },
            };
            let event = Event::sign(seq, 0, previous, body, signer)?;
            previous = Some(hash(&event)?);
            insert.execute(params![seq as i64, to_canon(&event)?])?;
            if seq.is_multiple_of(100_000) {
                println!("scale signed={seq}/{total}");
            }
        }
    }
    transaction.commit()?;
    println!("scale fixture_ready events={total}");
    Ok(())
}

fn equal_tables(a: &Path, b: &Path) -> Result<(), Box<dyn Error>> {
    let left = Connection::open(a)?;
    let right = Connection::open(b)?;
    for query in [
        "SELECT * FROM claims ORDER BY id",
        "SELECT * FROM deps ORDER BY claim,premise",
        "SELECT * FROM citable ORDER BY ref,epoch",
        "SELECT * FROM revoked ORDER BY checker",
        "SELECT * FROM metadata ORDER BY singleton",
    ] {
        let mut x = left.prepare(query)?;
        let mut y = right.prepare(query)?;
        let columns = x.column_count();
        assert_eq!(columns, y.column_count());
        let mut xx = x.query([])?;
        let mut yy = y.query([])?;
        loop {
            match (xx.next()?, yy.next()?) {
                (None, None) => break,
                (Some(x), Some(y)) => {
                    for i in 0..columns {
                        assert_eq!(x.get_ref(i)?, y.get_ref(i)?, "{query}");
                    }
                }
                _ => return Err("rebuild row counts differ".into()),
            }
        }
    }
    Ok(())
}

fn measure(
    root: &Path,
    total: u64,
    signer: &FileSigner,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let started = Instant::now();
    prepare(root, total, signer)?;
    let build = Instant::now();
    let index = LedgerIndex::open(root, signer.public_key())?;
    let build_ms = build.elapsed().as_millis();
    assert_eq!(index.report().events, total);
    let expected_head = index.head();
    if total == 100_000 {
        fs::copy(root.join("index.sqlite3"), root.join("before.sqlite3"))?;
        drop(index);
        fs::remove_file(root.join("index.sqlite3"))?;
        let rebuilt = LedgerIndex::open(root, signer.public_key())?;
        assert_eq!(
            rebuilt.report().previous,
            logos_ledger::IndexComparison::Missing
        );
        assert_eq!(rebuilt.head(), expected_head);
        equal_tables(&root.join("before.sqlite3"), &root.join("index.sqlite3"))?;
        println!("LEDGER_INDEX_REBUILD_OK events={total} tables=5 exact=true");
        return Ok(
            serde_json::json!({"events":total,"rebuild_equal":true,"tables":5,"build_ms":build_ms,"elapsed_ms":started.elapsed().as_millis()}),
        );
    }
    let read = Connection::open(root.join("ledger.sqlite3"))?;
    let mut samples = Vec::new();
    for i in 0..20_000u64 {
        let seq = 3 + (i * 49_999) % (total - 3);
        let wire: Vec<u8> =
            read.query_row("SELECT wire FROM events WHERE seq=?1", [seq as i64], |r| {
                r.get(0)
            })?;
        let event = Event::decode(&wire)?;
        let mut reference = hash(&event)?;
        let epoch = if i % 4 == 0 { 1 } else { 0 };
        let mut expected = epoch == 0
            && matches!(
                event.body,
                EventBody::Verified { .. } | EventBody::AssumptionDeclared { .. }
            );
        if i % 7 == 0 {
            let mut absent = *reference.as_bytes();
            absent[0] ^= 0xff;
            reference = Hash::from_bytes(absent);
            expected = false;
        }
        samples.push((reference, epoch, expected));
    }
    let mut timings = Vec::new();
    let mut positive = 0;
    for (reference, epoch, expected) in &samples {
        let start = Instant::now();
        let actual = index.is_citable(*reference, *epoch)?;
        timings.push(start.elapsed().as_nanos());
        assert_eq!(actual, *expected);
        positive += usize::from(actual);
    }
    timings.sort_unstable();
    let p99 = timings[(timings.len() * 99).div_ceil(100) - 1];
    assert!(positive > 0 && positive < samples.len());
    assert!(p99 < 1_000_000, "is_citable p99={p99}ns exceeds 1ms");
    println!(
        "LEDGER_INDEX_LATENCY_OK events={total} samples={} positives={positive} p99_ns={p99} (private SQLite snapshot; cold rebuild_ms={build_ms})",
        samples.len()
    );
    Ok(
        serde_json::json!({"events":total,"samples":samples.len(),"positive":positive,"negative":samples.len()-positive,"p50_ns":timings[timings.len()/2],"p99_ns":p99,"max_ns":timings[timings.len()-1],"build_ms":build_ms,"elapsed_ms":started.elapsed().as_millis(),"mode":"private in-memory SQLite rebuilt from signed ledger; query latency excludes cold build"}),
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [root, report] = args.as_slice() else {
        return Err("usage: ledger_index_scale FIXTURE_ROOT REPORT_JSON".into());
    };
    let root = Path::new(root);
    fs::create_dir_all(root)?;
    let signer = FileSigner::from_seed(&[0x11; 32])?;
    let a = measure(&root.join("100k"), 100_000, &signer)?;
    let b = measure(&root.join("1m"), 1_000_000, &signer)?;
    fs::write(
        report,
        serde_json::to_vec_pretty(
            &serde_json::json!({"fixture":"ledger-index-scale/v1","key":"public deterministic fixture 0x11","rebuild":a,"latency":b}),
        )?,
    )?;
    Ok(())
}
