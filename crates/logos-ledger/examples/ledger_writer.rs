//! Worker de crash/concurrency: semente pública de fixture, sem chave de produção.

use std::io::{self, Write};
use std::path::Path;

use logos_crypto::{FileSigner, Signer, hash};
use logos_ledger::{EventBody, Ledger, LedgerError, Route};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: ledger_writer DIRECTORY TOTAL_EVENTS".into());
    }
    let total: u64 = args[2].parse()?;
    let signer = FileSigner::from_seed(&[0x11; 32])?;
    let mut ledger = match Ledger::open(Path::new(&args[1]), &signer) {
        Ok(ledger) => ledger,
        Err(LedgerError::WriterBusy) => {
            println!("WRITER_BUSY");
            std::process::exit(23);
        }
        Err(error) => return Err(error.into()),
    };
    let start = ledger.head()?.map_or(0, |head| head.seq + 1);
    println!("READY {start}");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    if line != "GO\n" {
        return Err("missing GO handshake".into());
    }
    for seq in start..total {
        let body = if seq == 0 {
            EventBody::Genesis {
                ledger_version: 1,
                kernel_binary: [0x10; 32],
                kernel_key: signer.key_id()?,
            }
        } else {
            let mut bytes = [0; 32];
            for chunk in bytes.as_chunks_mut::<8>().0 {
                chunk.copy_from_slice(&seq.to_be_bytes());
            }
            EventBody::ObligationCreated {
                obligation: ledger.cas().put(&logos_canon::to_canon(&seq)?)?,
                route: Route::Model,
                artifact: bytes,
                cnf: None,
            }
        };
        let event = ledger.append(body)?;
        println!("ACK {} {}", event.seq, hash(&event)?);
        io::stdout().flush()?;
    }
    Ok(())
}
