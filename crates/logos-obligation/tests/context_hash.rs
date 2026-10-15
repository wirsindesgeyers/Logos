//! h(V) do envelope é metadado contextual, nunca obrigação selada ou prova.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use logos_air::Vocabulary;
use logos_crypto::{Hash, hash_raw};
use logos_obligation::{DecodeError, DecodeLimits, Obligation, vocabulary_for_check};

fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn context_hash_matches_independent_vectors_and_rejects_wrong_hash_size_or_envelope() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../spec/vectors/obligation/vectors.json"
    ))
    .unwrap();
    for vector in vectors["vectors"].as_array().unwrap() {
        let expected = &vector["expected"];
        let bytes = unhex(expected["canonical_hex"].as_str().unwrap());
        let digest = hash_raw::<Obligation>(&bytes);
        let expected_v =
            hash_raw::<Vocabulary>(&unhex(vector["target_vocabulary_hex"].as_str().unwrap()));
        assert_eq!(
            vocabulary_for_check(&bytes, digest, DecodeLimits::default()).unwrap(),
            expected_v
        );
        let mut wrong = *digest.as_bytes();
        wrong[0] ^= 1;
        assert!(matches!(
            vocabulary_for_check(&bytes, Hash::from_bytes(wrong), DecodeLimits::default()),
            Err(DecodeError::HashMismatch)
        ));
        assert!(matches!(
            vocabulary_for_check(
                &bytes,
                digest,
                DecodeLimits {
                    max_bytes: bytes.len() - 1,
                    ..DecodeLimits::default()
                }
            ),
            Err(DecodeError::Unknown(logos_core::Exhausted::Size))
        ));
        let mut extra = bytes.clone();
        extra.push(0);
        assert!(
            vocabulary_for_check(
                &extra,
                hash_raw::<Obligation>(&extra),
                DecodeLimits::default()
            )
            .is_err()
        );
    }
    for bytes in [vec![0x80], vec![0xff], vec![0xf6]] {
        assert!(
            vocabulary_for_check(
                &bytes,
                hash_raw::<Obligation>(&bytes),
                DecodeLimits::default()
            )
            .is_err()
        );
    }
}
