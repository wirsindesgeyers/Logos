//! Tags CAS aceitas são reservadas, exatas e mapeadas a componentes seguros únicos.
#![no_main]
use libfuzzer_sys::fuzz_target;
#[path = "../../crates/logos-ledger/src/cas_domain.rs"]
mod cas_domain;
use cas_domain::{DOMAINS, resolve};

fuzz_target!(|data: &[u8]| {
    if let Some(index) = resolve(data) {
        assert_eq!(DOMAINS[index].tag, data);
        assert!(
            logos_crypto::tags::RESERVED
                .iter()
                .any(|(_, tag, _)| *tag == data)
        );
    }
    for (index, entry) in DOMAINS.iter().enumerate() {
        assert_eq!(resolve(entry.tag), Some(index));
        assert!(!entry.directory.is_empty());
        assert!(
            entry
                .directory
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        );
        for other in &DOMAINS[index + 1..] {
            assert_ne!(entry.tag, other.tag);
            assert_ne!(entry.directory, other.directory);
        }
    }
    if let Some(&selector) = data.first() {
        let tag = DOMAINS[usize::from(selector) % DOMAINS.len()].tag;
        assert_eq!(resolve(&tag[..tag.len() - 1]), None);
        let mut changed = tag.to_vec();
        changed.extend_from_slice(data);
        assert_eq!(resolve(&changed), None);
        let mut prefixed = data.to_vec();
        prefixed.extend_from_slice(tag);
        assert_eq!(resolve(&prefixed), None);
    }
});
