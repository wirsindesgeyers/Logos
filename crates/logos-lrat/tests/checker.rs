//! Regressões adversariais do checker RUP.
use logos_core::{Budget, CheckOutcome, Exhausted};
use logos_crypto::{ExternalDigest, Hash, hash_raw};
use logos_ground::Cnf;
use logos_lrat::LratChecker;

const CNF: &[u8] = b"p cnf 2 4\n-2 0\n-1 2 0\n1 0\n2 0\n";
const PROOF: &[u8] = b"5 2 0 3 2 0\n5 d 2 3 0\n6 0 5 1 0\n6 d 5 0\n";
fn budget() -> Budget {
    Budget::new(10_000_000, 100_000_000, 10_000_000, 0)
}
fn accepted(cnf: &[u8], proof: &[u8]) -> bool {
    LratChecker::new()
        .unwrap()
        .check_lrat(cnf, hash_raw::<Cnf>(cnf), proof, &budget())
        == CheckOutcome::Accepted
}
fn rejected(cnf: &[u8], proof: &[u8]) {
    let r =
        LratChecker::new()
            .unwrap()
            .check_with_used(cnf, hash_raw::<Cnf>(cnf), proof, &budget());
    assert!(
        matches!(r.outcome(), CheckOutcome::Rejected(_)),
        "{cnf:?} {proof:?}: {r:?}"
    );
    assert_eq!(r.used_original_clauses(), None);
}
#[test]
fn history_survives_deletion_and_excludes_unused_original() {
    let r =
        LratChecker::new()
            .unwrap()
            .check_with_used(CNF, hash_raw::<Cnf>(CNF), PROOF, &budget());
    assert_eq!(r.outcome(), &CheckOutcome::Accepted);
    assert_eq!(r.used_original_clauses(), Some([1, 2, 3].as_slice()));
}
#[test]
fn p2_binds_exact_bytes_and_domain() {
    let checker = LratChecker::new().unwrap();
    for h in [
        hash_raw::<Cnf>(b"other"),
        Hash::from_bytes(*ExternalDigest::of(CNF).as_bytes()),
    ] {
        assert!(matches!(
            checker.check_lrat(CNF, h, PROOF, &budget()),
            CheckOutcome::Rejected(_)
        ));
    }
    let changed = b"p cnf 2 4\n-2 0\n-1 2 0\n1 0\n2 0\n\n";
    assert!(matches!(
        checker.check_lrat(changed, hash_raw::<Cnf>(CNF), PROOF, &budget()),
        CheckOutcome::Rejected(_)
    ));
}
#[test]
fn wrong_hint_changed_clause_and_unjustified_empty_are_rejected() {
    for p in [
        b"5 2 0 4 2 0\n6 0 5 1 0\n".as_slice(),
        b"5 1 0 3 2 0\n6 0 5 1 0\n",
        b"5 0 0\n",
        b"5 2 0 3 2 0\n",
    ] {
        rejected(CNF, p);
    }
}
#[test]
fn rejects_deleted_forward_self_reused_ids_and_invalid_deletions() {
    for p in [
        "5 d 1 0\n6 0 4 1 0\n",              // wrong deletion prefix
        "4 d 1 0\n5 0 4 1 0\n",              // deleted hint
        "5 2 0 6 2 0\n6 0 5 1 0\n",          // forward
        "5 2 0 5 2 0\n6 0 5 1 0\n",          // self
        "4 0 4 1 0\n",                       // original id
        "5 2 0 3 2 0\n5 d 5 0\n5 0 4 1 0\n", // reuse
        "4 d 99 0\n5 0 4 1 0\n",
        "4 d 1 1 0\n5 0 4 1 0\n",
    ] {
        rejected(CNF, p.as_bytes());
    }
}
#[test]
fn all_lines_are_checked_after_empty_and_on_unreachable_branches() {
    let good = "5 0 4 1 0\n";
    assert!(accepted(CNF, good.as_bytes()));
    for tail in ["garbage\n", "6 0 0\n", "5 d 99 0\n", "6 1 0 4 0\n"] {
        rejected(CNF, format!("{good}{tail}").as_bytes());
    }
    rejected(CNF, b"5 1 0 4 0\n6 0 4 1 0\n");
}
#[test]
fn sparse_ids_do_not_allocate_by_identifier_and_last_u64_is_valid() {
    let p = b"18446744073709551614 2 0 3 2 0\n18446744073709551615 0 18446744073709551614 1 0\n";
    let r = LratChecker::new()
        .unwrap()
        .check_with_used(CNF, hash_raw::<Cnf>(CNF), p, &budget());
    assert_eq!(r.outcome(), &CheckOutcome::Accepted);
    assert_eq!(r.used_original_clauses(), Some([1, 2, 3].as_slice()));
}
#[test]
fn duplicates_do_not_count_as_distinct_units_and_tautologies_do_not_conflict() {
    rejected(b"p cnf 2 2\n-2 0\n1 2 0\n", b"3 0 2 1 0\n");
    assert!(accepted(b"p cnf 1 2\n-1 -1 0\n1 1 0\n", b"3 0 1 2 0\n"));
    rejected(b"p cnf 1 2\n-1 1 0\n1 0\n", b"3 0 2 1 0\n");
    rejected(b"p cnf 2 1\n1 1 2 0\n", b"2 0 1 1 0\n");
    rejected(CNF, b"5 -1 1 0 0\n6 0 4 1 0\n");
}
#[test]
fn existing_empty_requires_explicit_derivation_and_first_empty_defines_used() {
    let cnf = b"p cnf 1 3\n0\n-1 0\n1 0\n";
    rejected(cnf, b"");
    assert!(accepted(cnf, b"4 0 1 0\n"));
    let r = LratChecker::new().unwrap().check_with_used(
        cnf,
        hash_raw::<Cnf>(cnf),
        b"4 0 2 3 0\n5 0 1 0\n5 d 4 0\n",
        &budget(),
    );
    assert_eq!(r.outcome(), &CheckOutcome::Accepted);
    assert_eq!(r.used_original_clauses(), Some([2, 3].as_slice()));
}
#[test]
fn syntax_rejects_noncanonical_cnf_and_malformed_or_unsupported_proofs() {
    for c in [
        "p cnf 02 4\n-2 0\n-1 2 0\n1 0\n2 0\n",
        "p cnf 2 1\n2 1 0\n",
        "p cnf 2 2\n1 0\n-1 0\n",
        "p cnf 2 1\n3 0\n",
        "p cnf 0 0",
        "p cnf 0 1\n0 0\n",
        "p cnf 2 0\n0\n",
        "p cnf 2 1\n-0 0\n",
    ] {
        rejected(c.as_bytes(), PROOF);
    }
    for p in [
        "5 0 4 1 0",
        "5 0 4 1 0 extra\n",
        "05 0 4 1 0\n",
        "5 +0 4 1 0\n",
        "5 0 -4 1 0\n",
        "5 0 18446744073709551616 0\n",
        "5 0 4 1\n",
        "5 d 1\n",
        "5 0 4 1 0\r\n",
        "c bad\0\n5 0 4 1 0\n",
        "5 3 0 4 1 0\n",
        "5 0 4 1 2 0\n",
    ] {
        rejected(CNF, p.as_bytes());
    }
    assert!(accepted(CNF, b"c proof\n \t\n5\t0 4 1 0\n5 d 0\n"));
}
#[test]
fn all_resource_exhaustions_are_unknown_and_never_have_used() {
    let checker = LratChecker::new().unwrap();
    for (b, e) in [
        (
            Budget {
                steps: 0,
                ..budget()
            },
            Exhausted::Steps,
        ),
        (
            Budget {
                mem_bytes: 0,
                ..budget()
            },
            Exhausted::Memory,
        ),
        (
            Budget {
                size_bytes: (CNF.len() + PROOF.len() - 1) as u64,
                ..budget()
            },
            Exhausted::Size,
        ),
    ] {
        let r = checker.check_with_used(CNF, hash_raw::<Cnf>(CNF), PROOF, &b);
        assert_eq!(r.outcome(), &CheckOutcome::Unknown(e));
        assert_eq!(r.used_original_clauses(), None);
    }
    let b = Budget {
        size_bytes: (CNF.len() + PROOF.len()) as u64,
        ..budget()
    };
    assert_eq!(
        checker.check_lrat(CNF, hash_raw::<Cnf>(CNF), PROOF, &b),
        CheckOutcome::Accepted
    );
    assert_eq!(
        checker.check_lrat(
            b"p cnf 1000001 0\n",
            hash_raw::<Cnf>(b"p cnf 1000001 0\n"),
            b"",
            &budget()
        ),
        CheckOutcome::Unknown(Exhausted::Size)
    );
    // Exact minimum step budget is inclusive; one less must be Unknown.
    let mut low = 0;
    let mut high = 10_000;
    while low < high {
        let mid = (low + high) / 2;
        let b = Budget {
            steps: mid,
            ..budget()
        };
        if checker.check_lrat(CNF, hash_raw::<Cnf>(CNF), PROOF, &b) == CheckOutcome::Accepted {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    assert!(low > 0);
    assert_eq!(
        checker.check_lrat(
            CNF,
            hash_raw::<Cnf>(CNF),
            PROOF,
            &Budget {
                steps: low - 1,
                ..budget()
            }
        ),
        CheckOutcome::Unknown(Exhausted::Steps)
    );
    assert_eq!(
        checker.check_lrat(
            CNF,
            hash_raw::<Cnf>(CNF),
            PROOF,
            &Budget {
                steps: low,
                ..budget()
            }
        ),
        CheckOutcome::Accepted
    );
}
#[test]
fn shared_budget_covers_multiple_additions_and_memory_is_inclusive() {
    let long_proof = format!("c {}\n5 0 4 1 0\n", "x".repeat(10_000));
    assert_eq!(
        LratChecker::new().unwrap().check_lrat(
            CNF,
            hash_raw::<Cnf>(CNF),
            long_proof.as_bytes(),
            &Budget {
                steps: 1000,
                ..budget()
            }
        ),
        CheckOutcome::Unknown(Exhausted::Steps)
    );

    let checker = LratChecker::new().unwrap();
    let mut low = 0;
    let mut high = 100_000;
    while low < high {
        let mid = (low + high) / 2;
        let b = Budget {
            mem_bytes: mid,
            ..budget()
        };
        if checker.check_lrat(CNF, hash_raw::<Cnf>(CNF), PROOF, &b) == CheckOutcome::Accepted {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    assert_eq!(
        checker.check_lrat(
            CNF,
            hash_raw::<Cnf>(CNF),
            PROOF,
            &Budget {
                mem_bytes: low - 1,
                ..budget()
            }
        ),
        CheckOutcome::Unknown(Exhausted::Memory)
    );
    assert_eq!(
        checker.check_lrat(
            CNF,
            hash_raw::<Cnf>(CNF),
            PROOF,
            &Budget {
                mem_bytes: low,
                ..budget()
            }
        ),
        CheckOutcome::Accepted
    );
    let repetitions = (5..105)
        .map(|id| format!("{id} 2 0 3 2 0\n"))
        .collect::<String>();
    let proof = format!("{repetitions}105 0 4 1 0\n");
    assert_eq!(
        checker.check_lrat(
            CNF,
            hash_raw::<Cnf>(CNF),
            proof.as_bytes(),
            &Budget {
                steps: 2000,
                ..budget()
            }
        ),
        CheckOutcome::Unknown(Exhausted::Steps)
    );
}

#[test]
fn deep_dependency_closure_uses_a_small_stack() {
    let result = std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(|| {
            let cnf = b"p cnf 1 2\n-1 0\n1 0\n";
            let mut proof = String::new();
            for id in 3..5003 {
                proof.push_str(&format!("{id} 1 0 {} 0\n", id - 1));
            }
            proof.push_str("5003 0 5002 1 0\n");
            LratChecker::new().unwrap().check_with_used(
                cnf,
                hash_raw::<Cnf>(cnf),
                proof.as_bytes(),
                &budget(),
            )
        })
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(result.outcome(), &CheckOutcome::Accepted);
    assert_eq!(result.used_original_clauses(), Some([1, 2].as_slice()));
}
