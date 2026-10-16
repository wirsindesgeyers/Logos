#![no_main]
use libfuzzer_sys::fuzz_target;
use logos_core::Budget;
use logos_crypto::hash_raw;
use logos_ground::Cnf;
use logos_lrat::LratChecker;
fuzz_target!(|data: &[u8]| {
    if let Ok(checker)=LratChecker::new() {
        for cnf in [b"p cnf 2 4\n-2 0\n-1 2 0\n1 0\n2 0\n".as_slice(), b"p cnf 0 1\n0\n"] {
            let _=checker.check_with_used(cnf,hash_raw::<Cnf>(cnf),data,&Budget::new(100_000,1_000_000,100_000,0));
        }
    }
});
