//! fact/v1 (6.1, ADR0061) contra vetores Python independentes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use logos_canon::{from_canon, to_canon};
use logos_crypto::{FileSigner, Signer, hash_raw};
use logos_ledger::{Fact, Validity};

fn unhex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks(2)
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect()
}

fn vectors() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../spec/vectors/fact/fact.json")).unwrap()
}

#[test]
fn valid_vectors_decode_reencode_hash_and_verify() {
    let root = vectors();
    for v in root["valid"].as_array().unwrap() {
        let wire = unhex(v["fact_canon_hex"].as_str().unwrap());
        let fact: Fact = from_canon(&wire).unwrap();
        assert_eq!(to_canon(&fact).unwrap(), wire, "{}", v["id"]);
        assert_eq!(
            hash_raw::<Fact>(&wire).as_bytes().to_vec(),
            unhex(v["fact_hash_hex"].as_str().unwrap())
        );
        assert_eq!(
            fact.signing_bytes().unwrap(),
            unhex(v["signed_canon_hex"].as_str().unwrap())
        );
        fact.verify_signature().unwrap();
        // O mesmo fato assinado pelo Rust com a mesma semente dá os mesmos bytes.
        let seed: [u8; 32] = unhex(v["issuer_seed_hex"].as_str().unwrap())
            .try_into()
            .unwrap();
        let signer = FileSigner::from_seed(&seed).unwrap();
        assert_eq!(signer.public_key(), &fact.issuer);
        let resigned = Fact::sign(
            &signer,
            fact.formula.clone(),
            fact.vocabulary,
            fact.topic.clone(),
            fact.observed_at,
            fact.valid,
            fact.version.clone(),
        )
        .unwrap();
        assert_eq!(to_canon(&resigned).unwrap(), wire, "{}", v["id"]);
    }
}

#[test]
fn invalid_vectors_are_refused_and_bad_signatures_do_not_verify() {
    let root = vectors();
    for v in root["invalid"].as_array().unwrap() {
        let wire = unhex(v["fact_canon_hex"].as_str().unwrap());
        assert!(from_canon::<Fact>(&wire).is_err(), "{}", v["id"]);
    }
    for v in root["bad_signature"].as_array().unwrap() {
        let wire = unhex(v["fact_canon_hex"].as_str().unwrap());
        let fact: Fact = from_canon(&wire).unwrap();
        assert!(fact.verify_signature().is_err(), "{}", v["id"]);
    }
}

#[test]
fn validity_windows_are_inclusive_and_until_needs_registration_epoch() {
    let range = Validity::Epochs(logos_core::EpochRange::new(3, 5).unwrap());
    for (epoch, expected) in [(2, false), (3, true), (4, true), (5, true), (6, false)] {
        assert_eq!(range.applicable(epoch, 0, 0), expected, "epoch {epoch}");
    }
    let until = Validity::Until(100);
    assert!(until.applicable(7, 100, 7));
    assert!(!until.applicable(7, 101, 7));
    assert!(!until.applicable(6, 50, 7));
}
