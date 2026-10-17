#![allow(dead_code)]
#[path = "../../../logos-ground/tests/support/mod.rs"]
pub(crate) mod support;
use logos_air::{Formula, FormulaNode as F, Symbol, SymbolKind, SymbolResult, Vocabulary};
use logos_core::{Budget, CheckOutcome};
use logos_crypto::{hash, hash_raw};
use logos_ground::{ClauseOrigin, Cnf, ProvenanceMap, clausify, ground};
use logos_lrat::{LratCertificate, LratChecker, used_premises};
use logos_obligation::{Obligation, PremiseRef};
use support::*;

pub(crate) fn limits() -> Budget {
    Budget::new(10_000_000, 268_435_456, 16_777_216, 10_000)
}
pub(crate) fn boolean(name: &str) -> Symbol {
    Symbol {
        name: text(name),
        kind: SymbolKind::Pred,
        arity: vec![],
        result: SymbolResult::Bool,
    }
}
pub(crate) fn atom(name: &str) -> Formula {
    pred(boolean(name), vec![])
}
pub(crate) fn policy() -> (Vocabulary, Vec<Formula>, Formula) {
    let v = Vocabulary::new(
        1,
        vec![],
        ["P", "Q", "R", "S", "T"]
            .into_iter()
            .map(|s| prim(boolean(s)))
            .collect(),
    )
    .unwrap();
    (
        v,
        vec![
            atom("P"),
            combine(F::Implies, &[atom("P"), atom("Q")]),
            atom("R"),
            atom("S"),
            atom("T"),
        ],
        atom("Q"),
    )
}
// Produtor RUP independente de uma adição: somente propagação e conflito.
pub(crate) fn proof(cnf: &Cnf, allowed: &[bool]) -> Option<Vec<u8>> {
    let mut values = vec![0i8; cnf.variables() + 1];
    let mut hints = Vec::new();
    loop {
        let mut progress = false;
        for (i, c) in cnf.clauses().iter().enumerate() {
            if !allowed[i]
                || c.iter()
                    .any(|x| values[x.unsigned_abs() as usize] == if *x > 0 { 1 } else { -1 })
            {
                continue;
            }
            let mut free: Vec<_> = c
                .iter()
                .filter(|x| values[x.unsigned_abs() as usize] == 0)
                .copied()
                .collect();
            free.dedup();
            if free.len() > 1 {
                continue;
            }
            hints.push(i + 1);
            if free.is_empty() {
                return Some(
                    format!(
                        "{} 0 {} 0\n",
                        cnf.clauses().len() + 1,
                        hints
                            .iter()
                            .map(usize::to_string)
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                    .into_bytes(),
                );
            }
            let x = free[0];
            values[x.unsigned_abs() as usize] = if x > 0 { 1 } else { -1 };
            progress = true;
        }
        if !progress {
            return None;
        }
    }
}
pub(crate) fn chosen(map: &ProvenanceMap, refs: &[PremiseRef]) -> Vec<bool> {
    map.origins()
        .iter()
        .zip(map.clause_support())
        .map(|(o, s)| match o {
            ClauseOrigin::Premise(r) | ClauseOrigin::DomainAxiom(r) => refs.contains(r),
            ClauseOrigin::NegatedGoal => true,
            // Exclui a assertiva global, mas conserva equivalências de Tseitin.
            ClauseOrigin::Definitional => s.is_empty(),
        })
        .collect()
}
pub(crate) fn checked(
    o: &Obligation,
    v: &Vocabulary,
    selected: &[PremiseRef],
) -> (Cnf, ProvenanceMap, Vec<u8>, logos_lrat::UsedPremises) {
    let g = ground(o, v).unwrap();
    let (cnf, map) = clausify(&g).unwrap();
    let p = proof(&cnf, &chosen(&map, selected)).expect("RUP conflict");
    let r = LratChecker::new().unwrap().check_with_used(
        cnf.dimacs(),
        hash_raw(cnf.dimacs()),
        &p,
        &limits(),
    );
    assert_eq!(r.outcome(), &CheckOutcome::Accepted);
    let used = used_premises(&r, &map, hash(o).unwrap(), &limits()).unwrap();
    assert_eq!(used.cnf_hash(), hash_raw::<Cnf>(cnf.dimacs()));
    assert_eq!(used.certificate_hash(), hash_raw::<LratCertificate>(&p));
    assert_eq!(used.obligation(), hash(o).unwrap());
    (cnf, map, p, used)
}

pub(crate) fn o_ref(i: u32) -> PremiseRef {
    let mut r = [0; 32];
    r[..4].copy_from_slice(&i.to_be_bytes());
    PremiseRef::from_event_hash(r)
}
