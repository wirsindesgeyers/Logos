//! Vetores `spec/vectors/policy/policy.json` (gerador Python independente): bytes canônicos,
//! hash `policy/v1` e recusas pelo motivo nomeado.

use logos_air::{Limits, Vocabulary};
use logos_canon::to_canon;
use logos_crypto::hash;
use logos_policy::{Policy, PolicyError};
use serde_json::Value;

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

fn vectors() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../spec/vectors/policy/policy.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn kind(error: &PolicyError) -> &'static str {
    match error {
        PolicyError::Canon(_) => "Canon",
        PolicyError::ZeroVersion => "ZeroVersion",
        PolicyError::TooManyRules => "TooManyRules",
        PolicyError::VocabularyMismatch => "VocabularyMismatch",
        PolicyError::BadRuleId => "BadRuleId",
        PolicyError::DuplicateRule(_) => "DuplicateRule",
        PolicyError::UnsortedRules => "UnsortedRules",
        PolicyError::CommentTooLong(_) => "CommentTooLong",
        PolicyError::Sort(..) => "Sort",
        PolicyError::MissingAllowed => "MissingAllowed",
        PolicyError::NotHorn(..) => "NotHorn",
        PolicyError::BadHead(..) => "BadHead",
        PolicyError::RequireWithoutAllowed(_) => "RequireWithoutAllowed",
        PolicyError::NegativeCycle(_) => "NegativeCycle",
        PolicyError::TooLarge => "TooLarge",
    }
}

#[test]
fn rust_reads_the_python_vocabulary_and_policies_byte_for_byte() {
    let vectors = vectors();
    let vocabulary_bytes = unhex(vectors["vocabulary_canon_hex"].as_str().unwrap());
    let vocabulary = Vocabulary::decode_with_limits(&vocabulary_bytes, Limits::default()).unwrap();
    assert_eq!(
        hash(&vocabulary).unwrap().to_hex(),
        vectors["vocabulary_hash_hex"].as_str().unwrap()
    );
    let valid = vectors["valid"].as_array().unwrap();
    assert!(valid.len() >= 3);
    for case in valid {
        let id = case["id"].as_str().unwrap();
        let wire = unhex(case["policy_canon_hex"].as_str().unwrap());
        let policy = Policy::decode(&wire, &vocabulary).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(to_canon(&policy).unwrap(), wire, "{id}: reencode");
        assert_eq!(
            policy.hash().unwrap().to_hex(),
            case["policy_hash_hex"].as_str().unwrap(),
            "{id}: hash"
        );
        let ids: Vec<&str> = policy.rules().iter().map(|r| r.id.as_str()).collect();
        let expected: Vec<&str> = case["rules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(ids, expected, "{id}: rules");
    }
}

#[test]
fn every_invalid_vector_is_refused_for_the_named_reason() {
    let vectors = vectors();
    let vocabulary = Vocabulary::decode_with_limits(
        &unhex(vectors["vocabulary_canon_hex"].as_str().unwrap()),
        Limits::default(),
    )
    .unwrap();
    let invalid = vectors["invalid"].as_array().unwrap();
    assert!(invalid.len() >= 10);
    for case in invalid {
        let id = case["id"].as_str().unwrap();
        let wire = unhex(case["policy_canon_hex"].as_str().unwrap());
        let error = Policy::decode(&wire, &vocabulary).unwrap_err();
        assert_eq!(
            kind(&error),
            case["reason"].as_str().unwrap(),
            "{id}: {error}"
        );
    }
}
