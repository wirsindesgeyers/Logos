//! Bytes aceitos por vocabulary/v1 precisam reencodar identicamente e respeitar limites lógicos.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_air::{Limits, Vocabulary};
use logos_canon::{from_canon, to_canon};

fn roundtrip(data: &[u8]) {
    if let Ok(vocab) = from_canon::<Vocabulary>(data) {
        let encoded = to_canon(&vocab).expect("Vocabulary aceito deve ser codificável");
        assert_eq!(encoded, data, "Vocabulary não canônico aceito");
        let decoded: Vocabulary =
            from_canon(&encoded).expect("Vocabulary codificado deve ser decodificável");
        assert_eq!(decoded, vocab);
    }
}

fuzz_target!(|data: &[u8]| {
    roundtrip(data);
    let limits = Limits {
        max_depth: usize::from(data.last().copied().unwrap_or(0) % 16),
        max_nodes: usize::from(data.get(data.len() / 2).copied().unwrap_or(0)),
    };
    if let Ok(vocab) = Vocabulary::decode_with_limits(data, limits) {
        let encoded = to_canon(&vocab).expect("Vocabulary aceito deve ser codificável sob limites");
        assert_eq!(encoded, data, "Vocabulary sob limites não canônico aceito");
        let decoded = Vocabulary::decode_with_limits(&encoded, limits)
            .expect("roundtrip de Vocabulary sob limites");
        assert_eq!(decoded, vocab);

        for declaration in vocab.symbols() {
            if let logos_air::Grounding::Defined(formula) = &declaration.grounding {
                assert!(formula.depth() <= limits.max_depth);
                assert!(formula.node_count() <= limits.max_nodes);
            }
        }
    }
});
