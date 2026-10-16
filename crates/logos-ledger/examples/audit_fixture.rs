//! Prepara fixture com o appender/CAS reais para o auditor compilado à parte.

use logos_crypto::{FileSigner, Signer};
use logos_ledger::{Certificate, Checker, Event, Ledger, ModelObject};

fn unhex(s: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|v| Ok(u8::from_str_radix(std::str::from_utf8(v)?, 16)?))
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args()
        .nth(1)
        .ok_or("usage: audit_fixture DIRECTORY")?;
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../spec/vectors/audit/vectors.json"))?;
    let signer = FileSigner::from_seed(&[17; 32])?;
    let mut ledger = Ledger::open(std::path::Path::new(&directory), &signer)?;
    for o in fixture["objects"].as_array().ok_or("objects")? {
        let wire = unhex(o["wire"].as_str().ok_or("wire")?)?;
        match o["domain"].as_str().ok_or("domain")? {
            "vocab" => {
                ledger.cas().put::<logos_air::Vocabulary>(&wire)?;
            }
            "obligation" => {
                ledger.cas().put::<logos_obligation::Obligation>(&wire)?;
            }
            "certificate" => {
                ledger.cas().put::<Certificate>(&wire)?;
            }
            "model" => {
                ledger.cas().put::<ModelObject>(&wire)?;
            }
            "checker" => {
                ledger.cas().put::<Checker>(&wire)?;
            }
            _ => return Err("unknown fixture domain".into()),
        }
    }
    for value in fixture["events"].as_array().ok_or("events")? {
        let wire = unhex(value.as_str().ok_or("event")?)?;
        let event = Event::decode(&wire)?;
        let appended = ledger.append(event.body)?;
        if logos_canon::to_canon(&appended)? != wire {
            return Err("appender differs from independent vector".into());
        }
    }
    let cnfs: serde_json::Value =
        serde_json::from_str(include_str!("../../../spec/vectors/cnf/vectors.json"))?;
    for c in cnfs["vectors"].as_array().ok_or("CNFs")?.iter().take(3) {
        ledger
            .cas()
            .put::<logos_air::Vocabulary>(&unhex(c["vocabulary"].as_str().ok_or("V")?)?)?;
        let obligation = ledger
            .cas()
            .put::<logos_obligation::Obligation>(&unhex(c["obligation"].as_str().ok_or("O")?)?)?;
        let bytes = c["cnf"].as_str().ok_or("DIMACS")?.as_bytes();
        let cnf = ledger.cas().put::<logos_ledger::CnfObject>(bytes)?;
        ledger.append(logos_ledger::EventBody::ObligationCreated {
            obligation,
            route: logos_ledger::Route::Lrat,
            artifact: *logos_crypto::ExternalDigest::of(bytes).as_bytes(),
            cnf: Some(logos_ledger::CnfRecord {
                hash: cnf,
                encoder_version: 1,
            }),
        })?;
    }
    // Fixture opcional4.5: provas reais, produzidas manualmente e rechecadas pelo auditor.
    if let Some(binary) = std::env::args().nth(2) {
        let checker_bytes = std::fs::read(binary)?;
        let checker = ledger.cas().put::<Checker>(&checker_bytes)?;
        for (which, proof) in [
            (0, b"10 0 4 7 0\n".as_slice()),
            (0, b"11 0 4 8 0\n"),
            (10, b"12 0 4 7 0\n"),
        ] {
            let c = &cnfs["vectors"][which];
            let obligation = ledger.cas().put::<logos_obligation::Obligation>(&unhex(
                c["obligation"].as_str().ok_or("O")?,
            )?)?;
            let bytes = c["cnf"].as_str().ok_or("CNF")?.as_bytes();
            let cnf = ledger.cas().put::<logos_ledger::CnfObject>(bytes)?;
            let certificate = ledger.cas().put::<Certificate>(proof)?;
            ledger.append(logos_ledger::EventBody::ObligationCreated {
                obligation,
                route: logos_ledger::Route::Lrat,
                artifact: *logos_crypto::ExternalDigest::of(bytes).as_bytes(),
                cnf: Some(logos_ledger::CnfRecord {
                    hash: cnf,
                    encoder_version: 1,
                }),
            })?;
            ledger.append(logos_ledger::EventBody::Verified {
                check: logos_ledger::CheckRecord {
                    obligation,
                    certificate,
                    checker,
                    budget: logos_core::Budget::new(1_000_000, 268_435_456, 1_000_000, 10_000),
                    outcome: logos_core::CheckOutcome::Accepted,
                },
                cited: vec![],
                used: vec![],
                consistency: logos_ledger::Consistency::Unknown { check: None },
            })?;
        }
    }
    println!(
        "{}",
        signer
            .public_key()
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    Ok(())
}
