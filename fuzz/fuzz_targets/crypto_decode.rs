//! Decodificação de `PublicKey`, `Signature` e `Hash<_>` a partir de bytes quaisquer: aceita só
//! o que o encoder produziria (round-trip idêntico) e nunca entra em pânico.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_canon::{from_canon, to_canon};
use logos_crypto::{Domain, Hash, PublicKey, Signature};

struct Fuzz;
impl Domain for Fuzz {
    const TAG: &'static [u8] = b"logos/fuzz/v1\0";
}

fuzz_target!(|data: &[u8]| {
    if let Ok(k) = from_canon::<PublicKey>(data) {
        assert_eq!(to_canon(&k).expect("chave válida é codificável"), data);
        let _ = k.key_id();
    }
    if let Ok(s) = from_canon::<Signature>(data) {
        assert_eq!(to_canon(&s).expect("assinatura válida é codificável"), data);
    }
    if let Ok(h) = from_canon::<Hash<Fuzz>>(data) {
        assert_eq!(to_canon(&h).expect("hash é codificável"), data);
    }
});
