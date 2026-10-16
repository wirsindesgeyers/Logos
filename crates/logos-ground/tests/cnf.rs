//! Enumeração completa de variáveis auxiliares e vetores independentes cnf/v1.
mod support;
use logos_air::{FormulaNode as F, Vocabulary};
use logos_canon::{from_canon, to_canon};
use logos_core::Exhausted;
use logos_crypto::{hash, hash_raw};
use logos_ground::{
    ClauseOrigin, Cnf, CnfLimits, GroundError, GroundLimits, clausify, clausify_with_limits,
    ground, ground_for_check,
};
use logos_obligation::{DecodeLimits, Obligation, PremiseRef};
use support::*;

fn hex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect()
}
fn vectors() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../spec/vectors/cnf/vectors.json")).unwrap()
}
fn satisfies(c: &[i32], bits: usize) -> bool {
    c.iter()
        .any(|x| ((bits >> (x.unsigned_abs() - 1)) & 1 != 0) == (*x > 0))
}
fn sat(cnf: &Cnf) -> bool {
    fn search(cs: &[Vec<i32>], mut values: Vec<i8>, work: &mut usize) -> bool {
        *work += 1;
        assert!(*work < 100_000, "test oracle exhausted");
        loop {
            let mut unit = None;
            let mut complete = true;
            for c in cs {
                if c.iter()
                    .any(|x| values[x.unsigned_abs() as usize] == if *x > 0 { 1 } else { -1 })
                {
                    continue;
                }
                complete = false;
                let free = c
                    .iter()
                    .filter(|x| values[x.unsigned_abs() as usize] == 0)
                    .copied()
                    .collect::<Vec<_>>();
                if free.is_empty() {
                    return false;
                }
                if free.len() == 1 {
                    unit = Some(free[0]);
                    break;
                }
            }
            if complete {
                return true;
            }
            if let Some(x) = unit {
                values[x.unsigned_abs() as usize] = if x > 0 { 1 } else { -1 };
            } else {
                break;
            }
        }
        let i = (1..values.len())
            .find(|i| values[*i] == 0)
            .expect("free variable");
        for value in [-1, 1] {
            let mut branch = values.clone();
            branch[i] = value;
            if search(cs, branch, work) {
                return true;
            }
        }
        false
    }
    let result = search(cnf.clauses(), vec![0; cnf.variables() + 1], &mut 0);
    if cnf.variables() <= 16 {
        let exhaustive = (0..1usize << cnf.variables())
            .any(|bits| cnf.clauses().iter().all(|c| satisfies(c, bits)));
        assert_eq!(result, exhaustive);
    }
    result
}

#[test]
fn independent_vectors_fix_every_literal_origin_hash_and_repetition() {
    for c in vectors()["vectors"].as_array().unwrap() {
        let v: Vocabulary = from_canon(&hex(c["vocabulary"].as_str().unwrap())).unwrap();
        let ob = hex(c["obligation"].as_str().unwrap());
        let expected = hash_raw::<Obligation>(&ob);
        let g = ground_for_check(
            &ob,
            expected,
            &v,
            DecodeLimits::default(),
            GroundLimits::default(),
        )
        .unwrap();
        for _ in 0..8 {
            let (cnf, map) = clausify(&g).unwrap();
            assert_eq!(cnf.dimacs(), c["cnf"].as_str().unwrap().as_bytes());
            assert_eq!(
                hash_raw::<Cnf>(cnf.dimacs()).to_hex(),
                c["cnf_digest"].as_str().unwrap()
            );
            let origins = map
                .origins()
                .iter()
                .map(|o| match o {
                    ClauseOrigin::Premise(r) => serde_json::json!([
                        0,
                        r.event_hash()
                            .iter()
                            .map(|x| format!("{x:02x}"))
                            .collect::<String>()
                    ]),
                    ClauseOrigin::NegatedGoal => serde_json::json!([1]),
                    ClauseOrigin::DomainAxiom(_) => panic!("no enum in boolean vectors"),
                    ClauseOrigin::Definitional => serde_json::json!([3]),
                })
                .collect::<Vec<_>>();
            assert_eq!(serde_json::json!(origins), c["origins"]);
            assert!(map.domain_support().is_empty());
            assert_eq!(cnf.clauses().len(), map.origins().len());
        }
        let (cnf, _) = clausify(&g).unwrap();
        assert_eq!(!sat(&cnf), tautology(&g));
        assert!(cnf.clauses().windows(2).all(|cs| cs[0] <= cs[1]));
        assert!(
            cnf.clauses()
                .iter()
                .all(|c| c.windows(2).all(|p| p[0] <= p[1]))
        );
    }
}

#[test]
fn all_auxiliary_assignments_enforce_exact_equivalences_and_consequence() {
    for c in vectors()["vectors"].as_array().unwrap() {
        let v: Vocabulary = from_canon(&hex(c["vocabulary"].as_str().unwrap())).unwrap();
        let ob = hex(c["obligation"].as_str().unwrap());
        let g = ground_for_check(
            &ob,
            hash_raw(&ob),
            &v,
            DecodeLimits::default(),
            GroundLimits::default(),
        )
        .unwrap();
        let (cnf, map) = clausify(&g).unwrap();
        let mut extensions = vec![0; 1usize << g.atoms().len()];
        let mask = extensions.len() - 1;
        let truth = (0..extensions.len())
            .map(|bits| {
                (0..g.nodes().len())
                    .map(|n| prop(&g, bits, n))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut next = g.atoms().len();
        let ids = g
            .nodes()
            .iter()
            .map(|n| match n {
                logos_ground::PropNode::Atom(a) => a + 1,
                _ => {
                    next += 1;
                    next
                }
            })
            .collect::<Vec<_>>();
        for bits in 0..1usize << cnf.variables() {
            let definitions = cnf
                .clauses()
                .iter()
                .zip(map.origins())
                .filter(|(_, o)| **o == ClauseOrigin::Definitional)
                .all(|(c, _)| satisfies(c, bits));
            if definitions {
                for (n, id) in ids.iter().enumerate() {
                    assert_eq!(
                        (bits >> (id - 1)) & 1 != 0,
                        truth[bits & mask][n],
                        "node {n} assignment {bits}"
                    );
                }
                extensions[bits & mask] += 1;
            }
        }
        // Boolean fixtures have constraints=True, so their assertive adds no restriction.
        assert_eq!(extensions, vec![1; extensions.len()]);
    }
}

#[test]
fn domain_axioms_remain_transformation_support_even_when_tautological() {
    let v = vocab(2, vec![prim(p("P"))]);
    let o = closed(
        &v,
        &eq(t(c("e0"), vec![]), t(c("e0"), vec![])),
        &[pred(p("P"), vec![t(c("e1"), vec![])])],
    );
    let g = ground(&o, &v).unwrap();
    let (cnf, map) = clausify(&g).unwrap();
    assert_eq!(map.domain_support(), g.domain_axioms());
    assert_eq!(map.domain_support().len(), 2);
    for r in g.domain_axioms() {
        assert!(map.origins().contains(&ClauseOrigin::DomainAxiom(*r)));
    }
    assert!(
        map.origins()
            .contains(&ClauseOrigin::Premise(PremiseRef::from_event_hash({
                let mut r = [0; 32];
                r[..4].copy_from_slice(&2u32.to_be_bytes());
                r
            })))
    );
    assert!(!sat(&cnf));
    // Guardas primitivas também são assertadas, não só Γ/goal.
    let v = vocab(2, vec![prim(c("c"))]);
    let a = t(c("e0"), vec![]);
    let cc = t(c("c"), vec![]);
    let goal = combine(F::Or(2), &[eq(cc.clone(), a), eq(cc, t(c("e1"), vec![]))]);
    let g = ground(&closed(&v, &goal, &[]), &v).unwrap();
    let (cnf, _) = clausify(&g).unwrap();
    assert!(!sat(&cnf));
}

#[test]
fn checked_input_matches_sealed_translation_and_never_accepts_wrong_identity() {
    let v = vocab(2, vec![]);
    let o = closed(&v, &f(vec![F::True]), &[]);
    let bytes = to_canon(&o).unwrap();
    let h = hash(&o).unwrap();
    let g = ground(&o, &v).unwrap();
    assert_eq!(
        g,
        ground_for_check(
            &bytes,
            h,
            &v,
            DecodeLimits::default(),
            GroundLimits::default()
        )
        .unwrap()
    );
    assert!(
        ground_for_check(
            &bytes,
            logos_crypto::Hash::from_bytes([7; 32]),
            &v,
            DecodeLimits::default(),
            GroundLimits::default()
        )
        .is_err()
    );
    let mut bad = bytes.clone();
    bad.push(0);
    assert!(
        ground_for_check(
            &bad,
            hash_raw(&bad),
            &v,
            DecodeLimits::default(),
            GroundLimits::default()
        )
        .is_err()
    );
    assert!(
        ground_for_check(
            &bytes,
            h,
            &vocab(1, vec![]),
            DecodeLimits::default(),
            GroundLimits::default()
        )
        .is_err()
    );
    let dl = DecodeLimits {
        max_bytes: bytes.len() - 1,
        ..DecodeLimits::default()
    };
    assert_eq!(
        ground_for_check(&bytes, h, &v, dl, GroundLimits::default()),
        Err(GroundError::Unknown(Exhausted::Size))
    );
}

#[test]
fn limits_are_inclusive_and_larger_budgets_do_not_change_bytes_or_origins() {
    let v = vocab(1, vec![prim(p("P"))]);
    let g = ground(
        &closed(&v, &pred(p("P"), vec![t(c("e0"), vec![])]), &[]),
        &v,
    )
    .unwrap();
    let (cnf, map) = clausify(&g).unwrap();
    let exact = CnfLimits {
        max_variables: cnf.variables(),
        max_clauses: cnf.clauses().len(),
        max_literals: cnf.clauses().iter().map(Vec::len).sum(),
        max_bytes: cnf.dimacs().len(),
    };
    assert_eq!(clausify_with_limits(&g, exact).unwrap(), (cnf, map));
    for k in 0..4 {
        let mut below = exact;
        match k {
            0 => below.max_variables -= 1,
            1 => below.max_clauses -= 1,
            2 => below.max_literals -= 1,
            _ => below.max_bytes -= 1,
        }
        assert_eq!(
            clausify_with_limits(&g, below),
            Err(GroundError::Unknown(Exhausted::Size))
        );
    }
}
