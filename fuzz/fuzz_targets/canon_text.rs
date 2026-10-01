//! Validação de texto (NFC + code points atribuídos): o que `Text::new` aceita sobrevive ao
//! round-trip codificar → decodificar sem mudar.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_canon::{Text, Value, from_canon};

fuzz_target!(|s: &str| {
    if let Ok(t) = Text::new(s) {
        let bytes = Value::Text(t.clone()).to_bytes();
        let back: Value = from_canon(&bytes).expect("texto válido é aceito");
        assert_eq!(back, Value::Text(t));
    }
});
