//! Extração real, suporte contextual e projeção no ledger existente.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use logos_air::{
    FormulaNode as F, SortDecl, SortDomain, Symbol, SymbolKind, SymbolResult, Vocabulary,
};
use logos_core::{Budget, CheckOutcome, Exhausted};
use logos_crypto::{Hash, hash};
use logos_ground::{ClauseOrigin, clausify, ground};
use logos_lrat::{LratChecker, UsedError, used_premises};
use logos_obligation::PremiseRef;
use used_support::support::*;

mod used_support;
use used_support::*;

#[test]
fn five_cited_two_used_excludes_unreachable_proof_branches() {
    let (v, fs, goal) = policy();
    let o = obligation(&v, &goal, &fs);
    let refs: Vec<_> = o.premises()[..2].iter().map(|(r, _)| *r).collect();
    let (cnf, map, p, used) = checked(&o, &v, &refs);
    assert_eq!(o.premises().len(), 5);
    assert_eq!(used.premises(), refs);
    let unused = map
        .origins()
        .iter()
        .position(|o| *o == ClauseOrigin::Premise(o_ref(2)))
        .unwrap();
    let next = cnf.clauses().len() + 1;
    let old = String::from_utf8(p).unwrap();
    let extended = format!(
        "{next} {} 0 {} 0\n{}{}",
        cnf.clauses()[unused][0],
        unused + 1,
        next + 1,
        &old[old.find(' ').unwrap()..]
    );
    let r = LratChecker::new().unwrap().check_with_used(
        cnf.dimacs(),
        map.cnf_hash(),
        extended.as_bytes(),
        &limits(),
    );
    assert_eq!(r.outcome(), &CheckOutcome::Accepted);
    assert_eq!(
        used_premises(&r, &map, hash(&o).unwrap(), &limits())
            .unwrap()
            .premises(),
        refs
    );
}

#[test]
fn omitted_necessary_premise_has_a_full_independent_sat_assignment() {
    let (v, fs, goal) = policy();
    let original = obligation(&v, &goal, &fs);
    let (_, _, p, _) = checked(&original, &v, &[o_ref(0), o_ref(1)]);
    let o = obligation(&v, &goal, &fs[1..]);
    let g = ground(&o, &v).unwrap();
    let (cnf, map) = clausify(&g).unwrap();
    let r =
        LratChecker::new()
            .unwrap()
            .check_with_used(cnf.dimacs(), map.cnf_hash(), &p, &limits());
    assert!(matches!(r.outcome(), CheckOutcome::Rejected(_)));
    assert_eq!(
        used_premises(&r, &map, hash(&o).unwrap(), &limits()),
        Err(UsedError::Unaccepted)
    );
    assert!(proof(&cnf, &vec![true; cnf.clauses().len()]).is_none());
    let bits = g.atoms().iter().enumerate().fold(0, |bits, (i, a)| {
        let name = v.symbols()[a.symbol as usize].signature.name.as_str();
        bits | (usize::from(!matches!(name, "P" | "Q")) << i)
    });
    assert!(g.premises().iter().all(|(_, n)| prop(&g, bits, *n)));
    assert!(!prop(&g, bits, g.goal()));
    let mut assignment = vec![false; cnf.variables() + 1];
    let mut next = g.atoms().len();
    for (i, n) in g.nodes().iter().enumerate() {
        let id = if let logos_ground::PropNode::Atom(a) = n {
            a + 1
        } else {
            next += 1;
            next
        };
        assignment[id] = prop(&g, bits, i);
    }
    assert!(cnf.clauses().iter().all(|c| {
        c.iter()
            .any(|x| assignment[x.unsigned_abs() as usize] == (*x > 0))
    }));
    // Uma tentativa explícita de hint externo também é recusada.
    let hidden = format!(
        "{} 0 {} 0\n",
        cnf.clauses().len() + 1,
        cnf.clauses().len() + 2
    );
    assert!(matches!(
        LratChecker::new().unwrap().check_lrat(
            cnf.dimacs(),
            map.cnf_hash(),
            hidden.as_bytes(),
            &limits()
        ),
        CheckOutcome::Rejected(_)
    ));
}

#[test]
fn accepted_report_cannot_be_projected_into_another_cnf_or_obligation() {
    let (v, fs, goal) = policy();
    let o = obligation(&v, &goal, &fs);
    let (cnf, map, p, _) = checked(&o, &v, &[o_ref(0), o_ref(1)]);
    let r =
        LratChecker::new()
            .unwrap()
            .check_with_used(cnf.dimacs(), map.cnf_hash(), &p, &limits());
    assert_eq!(
        used_premises(&r, &map, Hash::from_bytes([3; 32]), &limits()),
        Err(UsedError::ContextMismatch)
    );
    let other = obligation(&v, &atom("R"), &fs);
    let (_, other_map) = clausify(&ground(&other, &v).unwrap()).unwrap();
    assert_eq!(
        used_premises(&r, &other_map, hash(&other).unwrap(), &limits()),
        Err(UsedError::ContextMismatch)
    );
    let mut invalid = p;
    invalid.extend_from_slice(b"garbage\n");
    let rejected = LratChecker::new().unwrap().check_with_used(
        cnf.dimacs(),
        map.cnf_hash(),
        &invalid,
        &limits(),
    );
    assert_eq!(rejected.certificate_hash(), None);
    assert_eq!(rejected.cnf_hash(), None);
    assert_eq!(
        used_premises(&rejected, &map, hash(&o).unwrap(), &limits()),
        Err(UsedError::Unaccepted)
    );
    let unknown = LratChecker::new().unwrap().check_with_used(
        cnf.dimacs(),
        map.cnf_hash(),
        b"",
        &Budget {
            steps: 0,
            ..limits()
        },
    );
    assert_eq!(
        used_premises(&unknown, &map, hash(&o).unwrap(), &limits()),
        Err(UsedError::Unaccepted)
    );
}

#[test]
fn projection_limits_are_inclusive_and_never_return_partial_sets() {
    let (v, fs, goal) = policy();
    let o = obligation(&v, &goal, &fs);
    let (cnf, map, p, _) = checked(&o, &v, &[o_ref(0), o_ref(1)]);
    let r =
        LratChecker::new()
            .unwrap()
            .check_with_used(cnf.dimacs(), map.cnf_hash(), &p, &limits());
    for field in 0..3 {
        let mut low = 0;
        let mut high = 1_000_000;
        while low < high {
            let mid = (low + high) / 2;
            let mut b = limits();
            match field {
                0 => b.steps = mid,
                1 => b.mem_bytes = mid,
                _ => b.size_bytes = mid,
            };
            if used_premises(&r, &map, hash(&o).unwrap(), &b).is_ok() {
                high = mid;
            } else {
                low = mid + 1;
            }
        }
        assert!(low > 0);
        let mut exact = limits();
        match field {
            0 => exact.steps = low,
            1 => exact.mem_bytes = low,
            _ => exact.size_bytes = low,
        };
        assert!(used_premises(&r, &map, hash(&o).unwrap(), &exact).is_ok());
        match field {
            0 => exact.steps -= 1,
            1 => exact.mem_bytes -= 1,
            _ => exact.size_bytes -= 1,
        };
        let reason = match field {
            0 => Exhausted::Steps,
            1 => Exhausted::Memory,
            _ => Exhausted::Size,
        };
        assert_eq!(
            used_premises(&r, &map, hash(&o).unwrap(), &exact),
            Err(UsedError::Unknown(reason))
        );
    }
}

#[test]
fn unrelated_enumerated_domain_is_not_unconditionally_used() {
    let v = vocab(2, vec![prim(boolean("P"))]);
    let o = closed(&v, &atom("P"), &[atom("P")]);
    let r = *o.premises().last().unwrap().0.event_hash();
    let (_, map, _, used) = checked(&o, &v, &[PremiseRef::from_event_hash(r)]);
    assert!(!map.domain_support().is_empty());
    assert_eq!(used.premises(), [PremiseRef::from_event_hash(r)]);
}

#[test]
fn finite_translation_support_survives_tautological_domain_assertions() {
    let v = vocab(2, vec![]);
    let goal = combine(F::Not, &[eq(t(c("e0"), vec![]), t(c("e1"), vec![]))]);
    let o = closed(&v, &goal, &[]);
    let (_, map, _, used) = checked(&o, &v, &[]);
    let mut expected = map.domain_support().to_vec();
    expected.sort_unstable();
    expected.dedup();
    assert_eq!(used.premises(), expected);
    assert_eq!(expected.len(), 2);
}

#[test]
fn defined_transitive_body_contributes_its_domain_support() {
    let body = combine(F::Not, &[eq(t(c("e0"), vec![]), t(c("e1"), vec![]))]);
    let v = vocab(
        2,
        vec![def(boolean("A"), body), def(boolean("B"), atom("A"))],
    );
    let o = closed(&v, &atom("B"), &[]);
    let (_, map, _, used) = checked(&o, &v, &[]);
    let mut expected = map.domain_support().to_vec();
    expected.sort_unstable();
    expected.dedup();
    assert_eq!(used.premises(), expected);
}

#[test]
fn separate_sort_support_does_not_import_an_unused_sort() {
    let base = vocab(2, vec![]);
    let mut sorts = base.sorts().to_vec();
    let other = sort("F");
    let b = Symbol {
        name: text("b"),
        kind: SymbolKind::Const,
        arity: vec![],
        result: SymbolResult::Sort(other.clone()),
    };
    sorts.push(SortDecl {
        name: other.name.clone(),
        domain: SortDomain::Enumerated {
            constants: vec![b.clone()],
            domain_source: logos_air::DomainSource::Fact(logos_air::FactRef([7; 32])),
        },
    });
    let mut symbols = base.symbols().to_vec();
    symbols.push(prim(b));
    let v = Vocabulary::new(1, sorts, symbols).unwrap();
    let goal = combine(F::Not, &[eq(t(c("e0"), vec![]), t(c("e1"), vec![]))]);
    let o = closed(&v, &goal, &[]);
    let (_, map, _, used) = checked(&o, &v, &[]);
    assert_eq!(map.domain_support().len(), 3);
    assert_eq!(used.premises(), [o_ref(0), o_ref(1)]);
}

#[test]
fn global_inconsistent_defined_guard_keeps_its_domain_requirements() {
    let bad = def(c("bad"), f(vec![F::False]));
    let v = vocab(1, vec![bad]);
    let o = closed(&v, &f(vec![F::False]), &[]);
    let (cnf, map) = clausify(&ground(&o, &v).unwrap()).unwrap();
    let allowed = map
        .origins()
        .iter()
        .map(|o| !matches!(o, ClauseOrigin::Premise(_) | ClauseOrigin::DomainAxiom(_)))
        .collect::<Vec<_>>();
    let p = proof(&cnf, &allowed).unwrap();
    let r =
        LratChecker::new()
            .unwrap()
            .check_with_used(cnf.dimacs(), map.cnf_hash(), &p, &limits());
    assert_eq!(r.outcome(), &CheckOutcome::Accepted);
    assert_eq!(
        used_premises(&r, &map, hash(&o).unwrap(), &limits())
            .unwrap()
            .premises(),
        map.domain_support()
    );
}

#[test]
fn empty_domain_constraint_does_not_lose_its_explicit_closure() {
    let v = vocab(0, vec![]);
    let o = closed(&v, &f(vec![F::False]), &[]);
    let (cnf, map) = clausify(&ground(&o, &v).unwrap()).unwrap();
    let allowed = map
        .origins()
        .iter()
        .map(|o| !matches!(o, ClauseOrigin::DomainAxiom(_)))
        .collect::<Vec<_>>();
    let p = proof(&cnf, &allowed).unwrap();
    let r =
        LratChecker::new()
            .unwrap()
            .check_with_used(cnf.dimacs(), map.cnf_hash(), &p, &limits());
    assert_eq!(
        used_premises(&r, &map, hash(&o).unwrap(), &limits())
            .unwrap()
            .premises(),
        map.domain_support()
    );
}

#[test]
fn repeated_formula_support_is_deduplicated_with_explicit_premise() {
    let v = vocab(2, vec![prim(p("P"))]);
    let f = pred(p("P"), vec![t(c("e0"), vec![])]);
    let o = closed(&v, &f, std::slice::from_ref(&f));
    let (_, map, _, used) = checked(&o, &v, &[o_ref(2)]);
    assert_eq!(used.premises(), [o_ref(0), o_ref(1), o_ref(2)]);
    assert!(
        map.clause_support()
            .iter()
            .filter(|s| !s.is_empty())
            .count()
            > 1
    );
}
