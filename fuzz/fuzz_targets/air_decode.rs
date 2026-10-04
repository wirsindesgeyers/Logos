//! Bytes aceitos por air/v1 precisam reencodar identicamente e respeitar limites lógicos.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_air::{Formula, Limits, Sort, Symbol, Term};
use logos_canon::{Canon, CanonDecode, from_canon, to_canon};
use std::fmt::Debug;

fn roundtrip<T: Canon + CanonDecode + PartialEq + Debug>(data: &[u8]) {
    if let Ok(value) = from_canon::<T>(data) {
        let encoded = to_canon(&value).expect("AIR aceita deve ser codificável");
        assert_eq!(encoded, data, "AIR não canônica aceita");
        let decoded: T = from_canon(&encoded).expect("AIR codificada deve ser decodificável");
        assert_eq!(decoded, value);
    }
}

fuzz_target!(|data: &[u8]| {
    roundtrip::<Sort>(data);
    roundtrip::<Symbol>(data);
    roundtrip::<Term>(data);
    roundtrip::<Formula>(data);
    let limits = Limits {
        max_depth: usize::from(data.last().copied().unwrap_or(0) % 16),
        max_nodes: usize::from(data.get(data.len() / 2).copied().unwrap_or(0)),
    };
    if let Ok(term) = Term::decode_with_limits(data, limits) {
        assert!(term.depth() <= limits.max_depth);
        assert!(term.node_count() <= limits.max_nodes);
        let encoded = to_canon(&term).expect("termo aceito deve ser codificável");
        assert_eq!(encoded, data);
        assert_eq!(
            Term::decode_with_limits(&encoded, limits).expect("roundtrip de termo"),
            term
        );
    }
    if let Ok(formula) = Formula::decode_with_limits(data, limits) {
        assert!(formula.depth() <= limits.max_depth);
        assert!(formula.node_count() <= limits.max_nodes);
        let encoded = to_canon(&formula).expect("fórmula aceita deve ser codificável");
        assert_eq!(encoded, data);
        assert_eq!(
            Formula::decode_with_limits(&encoded, limits).expect("roundtrip de fórmula"),
            formula
        );
    }
});
