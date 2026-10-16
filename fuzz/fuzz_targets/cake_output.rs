#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    if let Some((&code, bytes)) = data.split_first() {
        let split = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        let (out, tail) = bytes.split_at(split);
        let err = tail.get(1..).unwrap_or_default();
        let _ = logos_lrat::decode_cake_output(i32::from(code), out, err);
    }
});
