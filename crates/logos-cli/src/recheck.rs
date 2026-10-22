//! Revogação de checker e recheck (5.10, ADR0056) com o kernel parado.
//!
//! Mesma autoridade do `logos-kernel` (sua chave). O recheck só usa checkers que
//! este processo executa de fato: o cake_lpr fixado, conferido por hash. O
//! checker Rust do kernel não tem a identidade deste executável.

use std::path::Path;

use logos_canon::Text;
use logos_core::Budget;
use logos_crypto::{FileSigner, Hash, Signer};
use logos_kernel::{
    recheck::Rechecks,
    registry::{CAKE, CheckerRegistry},
};
use logos_ledger::Ledger;

pub fn revoke(
    directory: &Path,
    key: &Path,
    checker: [u8; 32],
    reason: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let signer = FileSigner::load(key)?;
    let mut ledger = Ledger::open(directory, &signer)?;
    let event = Rechecks::new(signer.public_key().clone()).revoke(
        &mut ledger,
        &signer,
        Hash::from_bytes(checker),
        Text::new(reason)?,
    )?;
    println!("{{\"revoked\":\"{}\"}}", event.to_hex());
    Ok(())
}

pub fn recheck(
    directory: &Path,
    key: &Path,
    from: [u8; 32],
    with: [u8; 32],
    cake: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let signer = FileSigner::load(key)?;
    let registry = CheckerRegistry::new(signer.public_key().clone(), Some(cake))?;
    if registry.cake_identity().map(|h| *h.as_bytes()) != Some(with) {
        return Err("--with must be the hash of the configured cake_lpr checker".into());
    }
    let mut ledger = Ledger::open(directory, &signer)?;
    // Mesmo orçamento por checagem do logos-kernel.
    let budget = Budget::new(20_000_000, 128 * 1024 * 1024, 64 * 1024 * 1024, 5000);
    let report = Rechecks::new(signer.public_key().clone()).recheck(
        &mut ledger,
        &registry,
        Hash::from_bytes(from),
        CAKE,
        budget,
    )?;
    let list = |items: &[Hash<logos_ledger::Event>]| {
        items
            .iter()
            .map(|h| format!("\"{}\"", h.to_hex()))
            .collect::<Vec<_>>()
            .join(",")
    };
    println!(
        "{{\"accepted\":{},\"rejected\":{},\"unknown\":{},\"rechecked\":[{}],\"invalidated\":[{}]}}",
        report.accepted,
        report.rejected,
        report.unknown,
        list(&report.rechecked),
        list(&report.invalidated)
    );
    Ok(())
}
