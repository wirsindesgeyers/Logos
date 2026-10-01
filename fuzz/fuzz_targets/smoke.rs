//! Target trivial: existe só para provar que o job de fuzz (smoke no PR, ≥ 1h à noite) roda
//! de ponta a ponta. É substituído por targets reais a partir da Etapa 0.3.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = std::str::from_utf8(data);
});
