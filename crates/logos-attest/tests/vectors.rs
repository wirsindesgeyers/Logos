//! Vetores `spec/vectors/attest/clock.json`: o `clock` de Python (gerador independente) e o
//! de Rust produzem os mesmos bytes de fato e de vocabulário; o fato do Python decodifica,
//! verifica e vira citável no ledger real.
#![cfg(target_os = "linux")]

use std::fs;

use logos_attest::{
    Attestation, attest,
    reference::clock::{Clock, TimeSource},
    submit,
};
use logos_canon::{Text, from_canon, to_canon};
use logos_crypto::{FileSigner, Hash, Signer, hash};
use logos_ledger::{EventBody, Fact, Ledger, TrustConfig, TrustRoot};
use serde_json::Value;

struct Fixed(u64);
impl TimeSource for Fixed {
    fn now_unix(&self) -> Result<u64, logos_attest::AttestError> {
        Ok(self.0)
    }
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

fn vectors() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../spec/vectors/attest/clock.json"
    );
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn seed(vectors: &Value) -> [u8; 32] {
    unhex(vectors["attester_seed_hex"].as_str().unwrap())
        .try_into()
        .unwrap()
}

#[test]
fn rust_clock_reproduces_the_python_bytes() {
    let vectors = vectors();
    let signer = FileSigner::from_seed(&seed(&vectors)).unwrap();
    let now = vectors["now"].as_u64().unwrap();
    let cases = vectors["cases"].as_array().unwrap();
    assert!(cases.len() >= 4);
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let mut clock = Clock::new(
            Fixed(case["reference_unix"].as_u64().unwrap()),
            vectors["tolerance_secs"].as_u64().unwrap(),
            vectors["ttl_secs"].as_u64().unwrap(),
        );
        let ours = attest(&mut clock, &signer, now).unwrap();
        assert_eq!(
            to_canon(&ours.fact).unwrap(),
            unhex(case["fact_canon_hex"].as_str().unwrap()),
            "{id}: fact bytes"
        );
        assert_eq!(
            to_canon(&ours.vocabulary).unwrap(),
            unhex(case["vocabulary_canon_hex"].as_str().unwrap()),
            "{id}: vocabulary bytes"
        );
        assert_eq!(
            hash(&ours.fact).unwrap().to_hex(),
            case["fact_hash_hex"].as_str().unwrap(),
            "{id}: fact hash"
        );
        assert_eq!(
            Hash::<logos_air::Vocabulary>::to_hex(&hash(&ours.vocabulary).unwrap()),
            case["vocabulary_hash_hex"].as_str().unwrap(),
            "{id}: vocabulary hash"
        );
    }
}

#[test]
fn python_facts_decode_verify_and_become_citable() {
    let vectors = vectors();
    let signer = FileSigner::from_seed(&seed(&vectors)).unwrap();
    let kernel = FileSigner::from_seed(&[0x52; 32]).unwrap();
    let admin = FileSigner::from_seed(&[0xad; 32]).unwrap();
    let directory =
        std::env::temp_dir().join(format!("logos-attest-vectors-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir(&directory).unwrap();
    {
        let mut ledger = Ledger::open(&directory.join("ledger"), &kernel).unwrap();
        ledger
            .append(EventBody::Genesis {
                ledger_version: 1,
                kernel_binary: [0x52; 32],
                kernel_key: kernel.key_id().unwrap(),
            })
            .unwrap();
        let config = TrustConfig::new(
            1,
            vec![TrustRoot {
                issuer: signer.public_key().clone(),
                name: Text::new("clock").unwrap(),
                topics: vec![Text::new("time").unwrap()],
                level: 1,
            }],
        )
        .unwrap();
        let h = ledger
            .cas()
            .put::<TrustConfig>(&to_canon(&config).unwrap())
            .unwrap();
        logos_kernel::facts::publish_trust(&mut ledger, admin.public_key(), &admin, h).unwrap();
        // Só o primeiro caso: o mesmo `now` e o mesmo emissor repetiriam o fato com outra janela.
        let case = &vectors["cases"][0];
        let fact: Fact = from_canon(&unhex(case["fact_canon_hex"].as_str().unwrap())).unwrap();
        fact.verify_signature().unwrap();
        let vocabulary = logos_air::Vocabulary::decode_with_limits(
            &unhex(case["vocabulary_canon_hex"].as_str().unwrap()),
            logos_air::Limits::default(),
        )
        .unwrap();
        assert_eq!(fact.vocabulary, hash(&vocabulary).unwrap());
        let submitted = submit(&mut ledger, &Attestation { fact, vocabulary }).unwrap();
        let epoch = ledger.head().unwrap().unwrap().epoch;
        assert!(
            ledger
                .live_index()
                .unwrap()
                .is_citable(submitted.event, epoch)
                .unwrap()
        );
    }
    fs::remove_dir_all(&directory).unwrap();
}
