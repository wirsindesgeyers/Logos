//! Uso público com P2, fecho e rejeição explícita.
use logos_core::{Budget, CheckOutcome};
use logos_crypto::hash_raw;
use logos_ground::Cnf;
use logos_lrat::LratChecker;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let checker = LratChecker::new()?;
    let cnf = b"p cnf 1 2\n-1 0\n1 0\n";
    let budget = Budget::new(100_000, 100_000, 100_000, 0);
    let report = checker.check_with_used(cnf, hash_raw::<Cnf>(cnf), b"3 0 1 2 0\n", &budget);
    assert_eq!(report.outcome(), &CheckOutcome::Accepted);
    assert_eq!(report.used_original_clauses(), Some([1, 2].as_slice()));
    assert!(matches!(
        checker.check_lrat(cnf, hash_raw::<Cnf>(cnf), b"3 0 0\n", &budget),
        CheckOutcome::Rejected(_)
    ));
    println!("LRAT_SMOKE_OK used=1,2");
    Ok(())
}
