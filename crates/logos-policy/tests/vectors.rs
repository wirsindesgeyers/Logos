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

fn class_of(tag: u64) -> logos_policy::Class {
    match tag {
        0 => logos_policy::Class::Allow,
        1 => logos_policy::Class::Deny,
        2 => logos_policy::Class::Require,
        _ => logos_policy::Class::Derive,
    }
}

#[test]
fn rust_tree_matches_the_independent_python_merkle_implementation() {
    let vectors = vectors();
    let vocabulary = Vocabulary::decode_with_limits(
        &unhex(vectors["vocabulary_canon_hex"].as_str().unwrap()),
        Limits::default(),
    )
    .unwrap();
    let sections = vectors["merkle"].as_array().unwrap();
    assert_eq!(sections.len(), 4);
    for section in sections {
        let name = section["policy"].as_str().unwrap();
        let case = vectors["valid"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == name)
            .unwrap();
        let policy = Policy::decode(
            &unhex(case["policy_canon_hex"].as_str().unwrap()),
            &vocabulary,
        )
        .unwrap();
        let tree = logos_policy::PolicyTree::new(&policy).unwrap();
        let hex = |bytes: &[u8]| -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() };
        assert_eq!(
            hex(tree.root()),
            section["root_hex"].as_str().unwrap(),
            "{name}: root"
        );
        let leaves = section["leaves"].as_array().unwrap();
        assert_eq!(tree.leaves().len(), leaves.len(), "{name}: size");
        assert_eq!(section["size"].as_u64().unwrap() as usize, leaves.len());
        for (leaf, expected) in tree.leaves().iter().zip(leaves) {
            assert_eq!(
                leaf.key().class().unwrap(),
                class_of(expected["class"].as_u64().unwrap()),
                "{name}: class"
            );
            assert_eq!(
                leaf.key().symbol(),
                expected["symbol"].as_str().unwrap(),
                "{name}: key"
            );
            assert_eq!(
                hex(leaf.rule_hash()),
                expected["rule_hash_hex"].as_str().unwrap(),
                "{name}: rule hash"
            );
            let (_, proof) = tree.prove_inclusion(leaf.rule_hash()).unwrap();
            let path: Vec<String> = proof.path.iter().map(|h| hex(h)).collect();
            let expected_path: Vec<&str> = expected["path_hex"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            assert_eq!(path, expected_path, "{name}: path");
        }
        for absent in section["absent"].as_array().unwrap() {
            let key = logos_policy::IndexKey::new(
                class_of(absent["class"].as_u64().unwrap()),
                absent["symbol"].as_str().unwrap(),
            );
            let proof = tree.prove_non_membership(&key).unwrap();
            let left = proof.left.as_ref().map(|(_, p)| p.index);
            let right = proof.right.as_ref().map(|(_, p)| p.index);
            assert_eq!(left, absent["left"].as_u64(), "{name}: {key:?} left");
            assert_eq!(right, absent["right"].as_u64(), "{name}: {key:?} right");
            assert!(logos_policy::verify_non_membership(
                tree.root(),
                &key,
                &proof
            ));
        }
    }
}
