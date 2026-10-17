//! Parser do conteúdo do arquivo de chave: aceita só 64 hex minúsculos (+ `\n` opcional).
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_crypto::FileSigner;

fuzz_target!(|data: &[u8]| {
    if FileSigner::from_key_file_bytes(data).is_ok() {
        let body = data.strip_suffix(b"\n").unwrap_or(data);
        assert_eq!(body.len(), 64);
        assert!(
            body.iter()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c))
        );
    }
});
