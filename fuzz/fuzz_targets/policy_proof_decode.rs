//! Provas de política (7.2, ADR0070): decode estrito de folha, caminho de inclusão, não-pertinência
//! e `NoRule`; o aceito reencoda aos mesmos bytes e os verificadores nunca entram em pânico.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_canon::{from_canon, to_canon};
use logos_policy::{
    Class, InclusionProof, IndexKey, Leaf, NoRuleProof, NonMembership, verify_inclusion,
    verify_no_rule, verify_non_membership,
};

/// `[0, "drop", h]` canônico: folha fixa para exercitar `verify_inclusion` com caminhos arbitrários.
const FIXED_LEAF: &[u8] = &[0x83, 0x00, 0x64, 0x64, 0x72, 0x6f, 0x70, 0x58, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

fuzz_target!(|data: &[u8]| {
    if data.len() > 1 << 20 {
        return;
    }
    let root = [0x5a; 32];
    if let Ok(leaf) = from_canon::<Leaf>(data) {
        assert_eq!(to_canon(&leaf).expect("decoded leaf encodes"), data);
    }
    if let Ok(proof) = from_canon::<InclusionProof>(data) {
        assert_eq!(to_canon(&proof).expect("decoded proof encodes"), data);
        assert!(proof.path.len() <= 64);
        let leaf = from_canon::<Leaf>(FIXED_LEAF).expect("fixed leaf");
        let _ = verify_inclusion(&root, &leaf, &proof);
    }
    if let Ok(proof) = from_canon::<NonMembership>(data) {
        assert_eq!(to_canon(&proof).expect("decoded proof encodes"), data);
        for class in [Class::Allow, Class::Deny, Class::Require, Class::Derive] {
            let _ = verify_non_membership(&root, &IndexKey::new(class, "drop"), &proof);
        }
    }
    if let Ok(proof) = from_canon::<NoRuleProof>(data) {
        assert_eq!(to_canon(&proof).expect("decoded proof encodes"), data);
        let _ = verify_no_rule(&root, Class::Deny, "drop", &proof);
        let _ = verify_no_rule(&root, Class::Deny, "", &proof);
    }
});
