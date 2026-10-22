#![no_main]
use libfuzzer_sys::fuzz_target;
use logos_core::Budget;
use logos_crypto::hash_raw;
use logos_ground::Cnf;
use logos_lrat::LratChecker;
fuzz_target!(|data: &[u8]| {
    if let Ok(checker) = LratChecker::new() {
        let _ = checker.check_with_used(
            data,
            hash_raw::<Cnf>(data),
            b"5 0 1 2 0\n",
            &Budget::new(100_000, 1_000_000, 100_000, 0),
        );
    }
});
