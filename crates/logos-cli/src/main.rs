//! Operações explícitas sobre o ledger; esta CLI não pertence à TCB.
#![forbid(unsafe_code)]

mod gc;
#[cfg(target_os = "linux")]
mod recheck;
#[cfg(target_os = "linux")]
mod retract;

use std::ffi::OsString;
use std::path::Path;

use logos_crypto::{PublicKey, SigAlg};
use logos_ledger::LedgerIndex;

fn hex32(text: &std::ffi::OsStr, what: &str) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    let text = text.to_str().ok_or(format!("{what} must be ASCII hex"))?;
    if text.len() != 64 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("{what} must contain exactly 64 hex digits").into());
    }
    let mut bytes = [0; 32];
    for (result, pair) in bytes.iter_mut().zip(text.as_bytes().as_chunks::<2>().0) {
        *result = u8::from_str_radix(std::str::from_utf8(pair)?, 16)?;
    }
    Ok(bytes)
}

fn run(args: Vec<OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let usage = "usage: logos ledger rebuild-index DIRECTORY --public-key ED25519_HEX\n       logos ledger gc --dry-run DIRECTORY --public-key ED25519_HEX\n       logos ledger retract DIRECTORY --key KEYFILE --target EVENT_HEX --reason TEXT\n       logos ledger revoke-checker DIRECTORY --key KEYFILE --checker CHECKER_HEX --reason TEXT\n       logos ledger recheck DIRECTORY --key KEYFILE --checker CHECKER_HEX --with CHECKER_HEX --cake CAKE_BINARY";
    let (directory, public_key, gc) = match args.as_slice() {
        [ledger, command, directory, flag, key]
            if ledger == "ledger" && command == "rebuild-index" && flag == "--public-key" =>
        {
            (directory, key, false)
        }
        [ledger, command, dry, directory, flag, key]
            if ledger == "ledger"
                && command == "gc"
                && dry == "--dry-run"
                && flag == "--public-key" =>
        {
            (directory, key, true)
        }
        #[cfg(target_os = "linux")]
        [
            ledger,
            command,
            directory,
            key_flag,
            key,
            target_flag,
            target,
            reason_flag,
            reason,
        ] if ledger == "ledger"
            && command == "retract"
            && key_flag == "--key"
            && target_flag == "--target"
            && reason_flag == "--reason" =>
        {
            let reason = reason.to_str().ok_or("reason must be UTF-8")?;
            return retract::run(
                Path::new(directory),
                Path::new(key),
                hex32(target, "target")?,
                reason,
            );
        }
        #[cfg(target_os = "linux")]
        [
            ledger,
            command,
            directory,
            key_flag,
            key,
            checker_flag,
            checker,
            reason_flag,
            reason,
        ] if ledger == "ledger"
            && command == "revoke-checker"
            && key_flag == "--key"
            && checker_flag == "--checker"
            && reason_flag == "--reason" =>
        {
            let reason = reason.to_str().ok_or("reason must be UTF-8")?;
            return recheck::revoke(
                Path::new(directory),
                Path::new(key),
                hex32(checker, "checker")?,
                reason,
            );
        }
        #[cfg(target_os = "linux")]
        [
            ledger,
            command,
            directory,
            key_flag,
            key,
            checker_flag,
            checker,
            with_flag,
            with,
            cake_flag,
            cake,
        ] if ledger == "ledger"
            && command == "recheck"
            && key_flag == "--key"
            && checker_flag == "--checker"
            && with_flag == "--with"
            && cake_flag == "--cake" =>
        {
            return recheck::recheck(
                Path::new(directory),
                Path::new(key),
                hex32(checker, "checker")?,
                hex32(with, "with")?,
                Path::new(cake),
            );
        }
        _ => return Err(usage.into()),
    };
    let bytes = hex32(public_key, "public key")?;
    let key = PublicKey::new(SigAlg::Ed25519, &bytes)?;
    if gc {
        return gc::run(Path::new(directory), &key);
    }
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
