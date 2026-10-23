//! fact/v1 (6.1, ADR0061): decode estrito, bytes canônicos preservados e assinatura
//! verificável só sobre o corpo exato.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_canon::{from_canon, to_canon};
use logos_ledger::{Fact, MAX_FACT_BYTES, Validity};

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_FACT_BYTES {
        return;
    }
    let Ok(fact) = from_canon::<Fact>(data) else {
        return;
    };
    let wire = to_canon(&fact).expect("decoded fact must encode");
    assert_eq!(wire, data, "accepted noncanonical fact");
    assert_eq!(from_canon::<Fact>(&wire).expect("encoded fact decodes"), fact);
    if let Validity::Epochs(range) = fact.valid {
        assert!(range.first() <= range.last());
    }
    let signed = fact.signing_bytes().expect("decoded fact body encodes");
    assert!(signed.len() < data.len(), "signature must not be signed");
    // Assinar não é possível aqui; verificar não pode entrar em pânico.
    let _ = fact.verify_signature();
});
