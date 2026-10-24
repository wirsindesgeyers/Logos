//! trust/v1 (6.2, ADR0063) contra vetores Python independentes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use logos_canon::{from_canon, to_canon};
use logos_crypto::{FileSigner, Signer, hash_raw};
use logos_ledger::TrustConfig;

fn unhex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks(2)
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect()
}

fn vectors() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../spec/vectors/trust/trust.json")).unwrap()
}

#[test]
fn valid_vectors_decode_reencode_and_hash() {
    let root = vectors();
    for v in root["valid"].as_array().unwrap() {
        let wire = unhex(v["trust_canon_hex"].as_str().unwrap());
        let config: TrustConfig = from_canon(&wire).unwrap();
        assert_eq!(to_canon(&config).unwrap(), wire, "{}", v["id"]);
        assert_eq!(
            hash_raw::<TrustConfig>(&wire).as_bytes().to_vec(),
            unhex(v["trust_hash_hex"].as_str().unwrap())
        );
    }
}

#[test]
fn invalid_vectors_are_refused() {
    for v in vectors()["invalid"].as_array().unwrap() {
        let wire = unhex(v["trust_canon_hex"].as_str().unwrap());
        assert!(from_canon::<TrustConfig>(&wire).is_err(), "{}", v["id"]);
    }
}

#[test]
fn topic_rule_is_exact_per_issuer() {
    let root = vectors();
    let name = root["rule"]["config"].as_str().unwrap();
    let wire = root["valid"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == name)
        .unwrap()["trust_canon_hex"]
        .as_str()
        .unwrap();
    let config: TrustConfig = from_canon(&unhex(wire)).unwrap();
    for case in root["rule"]["cases"].as_array().unwrap() {
        let seed: [u8; 32] = unhex(case["seed_hex"].as_str().unwrap())
            .try_into()
            .unwrap();
        let issuer = FileSigner::from_seed(&seed).unwrap().key_id().unwrap();
        assert_eq!(
            config.trusts(&issuer, case["topic"].as_str().unwrap()),
            case["trusted"].as_bool().unwrap(),
            "{case}"
        );
    }
}
