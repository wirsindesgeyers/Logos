//! Retratação de operador (5.9, ADR0052) com o kernel parado.
//!
//! Abre o ledger com a chave do kernel (writer.lock exclusivo) e usa a mesma
//! autoridade que o `logos-kernel` configura: a própria chave pública. A API da
//! TCB decide quem pode retratar o quê; esta CLI só a chama e imprime o resultado.

use std::path::Path;

use logos_canon::Text;
use logos_crypto::{FileSigner, Hash, Signer};
use logos_kernel::retract::Retractions;
use logos_ledger::Ledger;

pub fn run(
    directory: &Path,
    key: &Path,
    target: [u8; 32],
    reason: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let signer = FileSigner::load(key)?;
    let mut ledger = Ledger::open(directory, &signer)?;
    let retraction = Retractions::new(signer.public_key().clone()).retract(
        &mut ledger,
        &signer,
        Hash::from_bytes(target),
        Text::new(reason)?,
    )?;
    let invalidated: Vec<String> = retraction
        .invalidated
        .iter()
        .map(|h| format!("\"{}\"", h.to_hex()))
        .collect();
    println!(
        "{{\"retracted\":\"{}\",\"invalidated\":[{}]}}",
        retraction.retracted.to_hex(),
        invalidated.join(",")
    );
    Ok(())
}
