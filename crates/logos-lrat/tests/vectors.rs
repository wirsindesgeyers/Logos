//! Vetores normativos com hashes e fecho produzidos independentemente em Python.
use logos_core::{Budget, CheckOutcome};
use logos_crypto::hash_raw;
use logos_ground::Cnf;
use logos_lrat::LratChecker;
#[test]
fn independent_vectors() {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../../../spec/vectors/lrat/vectors.json")).unwrap();
    let checker = LratChecker::new().unwrap();
    for v in vectors["vectors"].as_array().unwrap() {
        let cnf = v["cnf"].as_str().unwrap().as_bytes();
        let proof = v["proof"].as_str().unwrap().as_bytes();
        let h = hash_raw::<Cnf>(cnf);
        assert_eq!(h.to_hex(), v["h_cnf"].as_str().unwrap());
        let r = checker.check_with_used(
            cnf,
            h,
            proof,
            &Budget::new(10_000_000, 10_000_000, 10_000_000, 0),
        );
        if v["accepted"].as_bool().unwrap() {
            assert_eq!(r.outcome(), &CheckOutcome::Accepted, "{}", v["name"]);
            let used: Vec<u64> = v["used"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap())
                .collect();
            assert_eq!(
                r.used_original_clauses(),
                Some(used.as_slice()),
                "{}",
                v["name"]
            );
        } else {
            assert!(
                matches!(r.outcome(), CheckOutcome::Rejected(_)),
                "{} {r:?}",
                v["name"]
            );
            assert_eq!(r.used_original_clauses(), None);
        }
    }
}
