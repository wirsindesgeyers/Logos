//! Decoder canônico (Etapa 0.3): para QUALQUER entrada, ou rejeita, ou aceita e então a
//! reencodificação é idêntica byte a byte (nunca aceita codificação não canônica).
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_canon::{Value, from_canon};

fuzz_target!(|data: &[u8]| {
    if let Ok(v) = from_canon::<Value>(data) {
        assert_eq!(v.to_bytes(), data, "aceitou codificação não canônica");
        let again: Value = from_canon(&v.to_bytes()).expect("o que o encoder produz é aceito");
        assert_eq!(again, v);
    }
});
