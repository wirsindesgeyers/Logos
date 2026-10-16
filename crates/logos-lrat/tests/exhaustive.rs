//! Diferencial exaustivo: todas as CNFs sobre duas variáveis, oráculo independente.
use logos_core::{Budget, CheckOutcome};
use logos_crypto::hash_raw;
use logos_ground::Cnf;
use logos_lrat::LratChecker;
fn sat(clauses: &[Vec<i32>]) -> bool {
    (0..4).any(|bits| {
        clauses.iter().all(|c| {
            c.iter()
                .any(|&l| ((bits >> (l.unsigned_abs() - 1)) & 1 == 1) == (l > 0))
        })
    })
}
#[test]
fn every_accepted_tiny_trace_and_its_used_subset_are_unsatisfiable() {
    let mut universe = [
        vec![-2, -1],
        vec![-2, 1],
        vec![-2],
        vec![-1, 2],
        vec![-1],
        vec![1, 2],
        vec![1],
        vec![2],
    ];
    universe.sort();
    let checker = LratChecker::new().unwrap();
    let mut accepts = 0;
    let mut rejects = 0;
    for mask in 0..256 {
        let clauses: Vec<Vec<i32>> = universe
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, c)| c.clone())
            .collect();
        let mut cnf = format!("p cnf 2 {}\n", clauses.len());
        for clause in &clauses {
            for l in clause {
                cnf.push_str(&format!("{l} "));
            }
            cnf.push_str("0\n");
        }
        let hash = hash_raw::<Cnf>(cnf.as_bytes());
        let satisfiable = sat(&clauses);
        for a in 1..=clauses.len() {
            for b in 0..=clauses.len() {
                for c in 0..=clauses.len() {
                    if b == 0 && c != 0 {
                        continue;
                    }
                    let hints = [a, b, c]
                        .into_iter()
                        .filter(|&v| v != 0)
                        .map(|v| format!("{v} "))
                        .collect::<String>();
                    let proof = format!("{} 0 {hints}0\n", clauses.len() + 1);
                    let r = checker.check_with_used(
                        cnf.as_bytes(),
                        hash,
                        proof.as_bytes(),
                        &Budget::new(100_000, 100_000, 100_000, 0),
                    );
                    match r.outcome() {
                        CheckOutcome::Accepted => {
                            accepts += 1;
                            assert!(!satisfiable, "{cnf} {proof}");
                            let used: Vec<Vec<i32>> = r
                                .used_original_clauses()
                                .unwrap()
                                .iter()
                                .map(|&i| clauses[i as usize - 1].clone())
                                .collect();
                            assert!(!sat(&used), "{r:?}");
                        }
                        CheckOutcome::Rejected(_) => {
                            rejects += 1;
                            assert_eq!(r.used_original_clauses(), None);
                        }
                        other => panic!("unexpected {other:?}"),
                    }
                }
            }
        }
    }
    assert!(accepts > 100 && rejects > 1000);
    println!("LRAT_EXHAUSTIVE_OK accepted={accepts} rejected={rejects}");
}
