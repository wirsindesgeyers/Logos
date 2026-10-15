//! Comandos públicos do auditor independente; chave pública sempre externa.

#![forbid(unsafe_code)]

use std::path::Path;

use logos_crypto::{PublicKey, SigAlg};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let usage = "uso: logos-audit verify LEDGER --cas DIR --public-key ED25519_HEX | logos-audit export LEDGER --public-key ED25519_HEX";
    let (command, ledger, rest) = match args.as_slice() {
        [command, ledger, rest @ ..] => (command.as_str(), ledger, rest),
        _ => return Err(usage.into()),
    };
    let mut cas = None;
    let mut key = None;
    let (options, remainder) = rest.as_chunks::<2>();
    for pair in options {
        match pair[0].as_str() {
            "--cas" if cas.is_none() => cas = Some(&pair[1]),
            "--public-key" if key.is_none() => key = Some(&pair[1]),
            _ => return Err(usage.into()),
        }
    }
    if !remainder.is_empty() {
        return Err(usage.into());
    }
    let hex = key.ok_or(usage)?;
    if hex.len() != 64 || !hex.is_ascii() {
        return Err("chave Ed25519 precisa de 64 dígitos hexadecimais".into());
    }
    let bytes = (0..32)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16))
        .collect::<Result<Vec<_>, _>>()?;
    let key = PublicKey::new(SigAlg::Ed25519, &bytes)?;
    match command {
        "verify" => {
            let cas = cas.ok_or(usage)?;
            let report = logos_audit::verify(Path::new(ledger), Path::new(cas), &key)?;
            println!("AUDIT_OK {report}");
        }
        "export" if cas.is_none() => {
            let report =
                logos_audit::export(Path::new(ledger), &key, &mut std::io::stdout().lock())?;
            eprintln!("AUDIT_EXPORTED authenticated_only=true {report}");
        }
        _ => return Err(usage.into()),
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("AUDIT_FAILED failures=1: {error}");
        std::process::exit(1);
    }
}
