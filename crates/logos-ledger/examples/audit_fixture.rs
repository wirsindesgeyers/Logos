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
