//! trust/v1 (6.2, ADR0063): decode estrito, bytes canônicos preservados e invariantes
//! de ordem/unicidade que a regra de confiança assume.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_canon::{from_canon, to_canon};
use logos_ledger::{MAX_TRUST_BYTES, TrustConfig};

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_TRUST_BYTES {
        return;
    }
    let Ok(config) = from_canon::<TrustConfig>(data) else {
        return;
    };
    let wire = to_canon(&config).expect("decoded trust config must encode");
    assert_eq!(wire, data, "accepted noncanonical trust config");
    assert_eq!(
        from_canon::<TrustConfig>(&wire).expect("encoded trust config decodes"),
        config
    );
    assert!(config.version >= 1 && !config.roots.is_empty());
    let ids: Vec<_> = config
        .roots
        .iter()
        .map(|r| r.issuer.key_id().expect("decoded key has an id"))
        .collect();
    assert!(ids.windows(2).all(|w| w[0].as_bytes() < w[1].as_bytes()));
    for (root, id) in config.roots.iter().zip(&ids) {
        assert!(!root.topics.is_empty());
        for topic in &root.topics {
            assert!(config.trusts(id, topic.as_str()));
        }
    }
});
