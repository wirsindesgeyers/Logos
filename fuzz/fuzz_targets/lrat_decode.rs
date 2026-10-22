#![no_main]
use libfuzzer_sys::fuzz_target;
use logos_air::{Formula, FormulaNode, Limits, Vocabulary};
use logos_core::Budget;
use logos_crypto::{hash, hash_raw};
use logos_ground::{Cnf, clausify, ground};
use logos_lrat::{LratChecker, used_premises};
use logos_obligation::{LedgerEntry, LedgerError, LedgerView, PremiseRef, Proposal, generate};
struct Empty;
impl LedgerView for Empty {
    fn current_epoch(&self) -> u64 {
        0
    }
    fn lookup(&self, _: PremiseRef) -> Result<Option<LedgerEntry<'_>>, LedgerError> {
        Ok(None)
    }
}
fuzz_target!(|data: &[u8]| {
    if let Ok(checker) = LratChecker::new() {
        let budget = Budget::new(100_000, 1_000_000, 100_000, 0);
        for cnf in [
            b"p cnf 2 4\n-2 0\n-1 2 0\n1 0\n2 0\n".as_slice(),
            b"p cnf 0 1\n0\n",
        ] {
            let _ = checker.check_with_used(cnf, hash_raw::<Cnf>(cnf), data, &budget);
        }
        let v = Vocabulary::new(1, vec![], vec![]).expect("vocabulary");
        let f = Formula::new(vec![FormulaNode::True], Limits::default()).expect("formula");
        let o = generate(
            Proposal {
                vocabulary: &v,
                goal: &f,
                cites: &[],
            },
            &Empty,
        )
        .expect("O");
        let (cnf, map) = clausify(&ground(&o, &v).expect("ground")).expect("CNF");
        let report = checker.check_with_used(cnf.dimacs(), map.cnf_hash(), data, &budget);
        if let Ok(used) = used_premises(&report, &map, hash(&o).expect("hO"), &budget) {
            assert!(used.premises().is_empty());
            assert_eq!(used.cnf_hash(), map.cnf_hash());
            assert_eq!(Some(used.certificate_hash()), report.certificate_hash());
        }
    }
});
