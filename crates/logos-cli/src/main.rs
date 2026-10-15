//! Operações explícitas sobre o ledger; esta CLI não pertence à TCB.
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::Path;

use logos_crypto::{PublicKey, SigAlg};
use logos_ledger::LedgerIndex;

fn run(args: Vec<OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let usage = "usage: logos ledger rebuild-index DIRECTORY --public-key ED25519_HEX";
    let [ledger, command, directory, flag, public_key] = args.as_slice() else {
        return Err(usage.into());
    };
    if ledger != "ledger" || command != "rebuild-index" || flag != "--public-key" {
        return Err(usage.into());
    }
    let public_key = public_key.to_str().ok_or("public key must be ASCII hex")?;
    if public_key.len() != 64 || !public_key.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("public key must contain exactly 64 hex digits".into());
    }
    let mut bytes = [0; 32];
    for (result, pair) in bytes
        .iter_mut()
        .zip(public_key.as_bytes().as_chunks::<2>().0)
    {
        *result = u8::from_str_radix(std::str::from_utf8(pair)?, 16)?;
    }
    let key = PublicKey::new(SigAlg::Ed25519, &bytes)?;
    let index = LedgerIndex::open(Path::new(directory), &key)?;
    let report = index.report();
    println!(
        "LEDGER_INDEX_REBUILT events={} claims={} previous={:?} head={}",
        report.events,
        report.claims,
        report.previous,
        report
            .head
            .map_or_else(|| "empty".into(), |h| h.hash.to_hex())
    );
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run(std::env::args_os().skip(1).collect()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("logos: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
