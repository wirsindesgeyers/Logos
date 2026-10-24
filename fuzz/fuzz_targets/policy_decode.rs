//! policy/v1 (7.1, ADR0069): decode estrito contra um vocabulário fixo; o que é aceito
//! reencoda aos mesmos bytes, e a validação (sort-check, forma de Horn, estratificação) nunca
//! entra em pânico.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_air::{Limits, Vocabulary};
use logos_canon::to_canon;
use logos_policy::{MAX_POLICY_BYTES, Policy};

const VOCABULARY: &[u8] = include_bytes!("../../spec/vectors/policy/vocabulary.bin");

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_POLICY_BYTES {
        return;
    }
    let vocabulary =
        Vocabulary::decode_with_limits(VOCABULARY, Limits::default()).expect("fixed vocabulary");
    let Ok(policy) = Policy::decode(data, &vocabulary) else {
        return;
    };
    let wire = to_canon(&policy).expect("decoded policy must encode");
    assert_eq!(wire, data, "accepted noncanonical policy");
    let again = Policy::decode(&wire, &vocabulary).expect("re-decode of an accepted policy");
    assert_eq!(again, policy);
    assert!(policy.rules().windows(2).all(|w| w[0].id.as_str() < w[1].id.as_str()));
});
