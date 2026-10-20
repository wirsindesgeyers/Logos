//! Testes de integração, vetores e invariantes de event/v1 (spec/events.md e ADR 0020).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use logos_air::{Formula, FormulaNode, Limits, Vocabulary};
use logos_canon::{Array, Canon, CanonDecode, Text, Value, from_canon, to_canon};
use logos_core::{Budget, CheckOutcome, Exhausted, Reason};
use logos_crypto::{FileSigner, Hash, PublicKey, SigAlg, Signature, Signer, hash};
use logos_ledger::{
    Certificate, CheckRecord, Checker, Consistency, Event, EventBody, EventError, EventProposal,
    MAX_EVENT_BYTES, Route, TaintedText,
};
use logos_obligation::Obligation;
use serde_json::Value as JsonValue;

fn unhex(hex_str: &str) -> Vec<u8> {
    assert!(hex_str.len().is_multiple_of(2), "hex string must be even");
    (0..hex_str.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16).expect("valid hex byte"))
        .collect()
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn default_budget() -> Budget {
    Budget {
        steps: 1000,
        mem_bytes: 1024 * 1024,
        size_bytes: 64 * 1024,
        wall_ms: 500,
    }
}

#[test]
fn certificate_rejection_has_only_v2_tag13_and_cannot_hold_accepted_or_unknown() {
    let check = sample_check_record(CheckOutcome::Rejected(Reason::new(
        Text::new("bad certificate").unwrap(),
    )));
    let body = EventBody::CertificateRejected {
        check: check.clone(),
    };
    let value = body.to_value().unwrap();
    assert_eq!(EventBody::from_value(&value).unwrap(), body);
    assert_eq!(body.cas_references().len(), 3);
    for outcome in [
        CheckOutcome::Accepted,
        CheckOutcome::Unknown(Exhausted::Steps),
    ] {
        let mut c = check.clone();
        c.outcome = outcome;
        assert!(
            EventBody::CertificateRejected { check: c.clone() }
                .validate()
                .is_err()
        );
        let forged = Value::Array(
            Array::new(vec![
                Value::Unsigned(2),
                Value::Unsigned(13),
                c.to_value().unwrap(),
            ])
            .unwrap(),
        );
        assert!(EventBody::from_value(&forged).is_err());
    }
    let Value::Array(fields) = value else {
        unreachable!()
    };
    let mut fields = fields.items().to_vec();
    fields[0] = Value::Unsigned(1);
    assert!(EventBody::from_value(&Value::Array(Array::new(fields.clone()).unwrap())).is_err());
    fields[0] = Value::Unsigned(2);
    fields.push(Value::Unsigned(0));
    assert!(EventBody::from_value(&Value::Array(Array::new(fields).unwrap())).is_err());
}

fn sample_check_record(outcome: CheckOutcome) -> CheckRecord {
    CheckRecord {
        obligation: Hash::<Obligation>::from_bytes([0xb1; 32]),
        certificate: Hash::<Certificate>::from_bytes([0xc1; 32]),
        checker: Hash::<Checker>::from_bytes([0xd1; 32]),
        budget: default_budget(),
        outcome,
    }
}

// -----------------------------------------------------------------------------
// 1. Vetores Canônicos JSON (spec/vectors/events/vectors.json)
// -----------------------------------------------------------------------------

#[test]
fn test_all_canonical_vectors_decode_encode_verify() {
    let raw_json = include_str!("../../../spec/vectors/events/vectors.json");
    let root: JsonValue = serde_json::from_str(raw_json).expect("valid JSON vectors");
    let vectors = root["vectors"].as_array().expect("vectors array");

    assert!(
        vectors.len() >= 13,
        "must have at least 13 canonical vectors"
    );

    for vec_val in vectors {
        let id = vec_val["id"].as_str().expect("id string");
        let wire_hex = vec_val["event_wire_hex"].as_str().expect("event_wire_hex");
        let hash_hex = vec_val["event_hash_hex"].as_str().expect("event_hash_hex");
        let pub_hex = vec_val["kernel_public_key_hex"]
            .as_str()
            .expect("kernel_public_key_hex");
        let key_id_hex = vec_val["kernel_key_id_hex"]
            .as_str()
            .expect("kernel_key_id_hex");

        let wire_bytes = unhex(wire_hex);
        let kernel_pub =
            PublicKey::new(SigAlg::Ed25519, &unhex(pub_hex)).expect("valid kernel public key");
        assert_eq!(to_hex(kernel_pub.key_id().unwrap().as_bytes()), key_id_hex);

        // 1. Event::decode dos bytes do wire
        let decoded = Event::decode(&wire_bytes)
            .unwrap_or_else(|e| panic!("vector {id} decode failed: {e:?}"));

        // 2. Validação estrutural do corpo
        decoded
            .body
            .validate()
            .unwrap_or_else(|e| panic!("vector {id} body validate failed: {e:?}"));

        // 3. Verificação criptográfica da assinatura externa
        decoded
            .verify(&kernel_pub)
            .unwrap_or_else(|e| panic!("vector {id} verify failed: {e:?}"));

        // 4. Re-codificação canônica estrita coincide byte a byte
        let recoded =
            to_canon(&decoded).unwrap_or_else(|e| panic!("vector {id} re-encode failed: {e:?}"));
        assert_eq!(
            to_hex(&recoded),
            wire_hex,
            "vector {id} re-encoded bytes differ"
        );

        // 5. Hash integral do evento coincide
        let computed_hash = hash(&decoded).expect("hash calculation");
        assert_eq!(
            to_hex(computed_hash.as_bytes()),
            hash_hex,
            "vector {id} event hash differs"
        );
        let seed: [u8; 32] = unhex(vec_val["kernel_seed_hex"].as_str().unwrap())
            .try_into()
            .unwrap();
        let signer = FileSigner::from_seed(&seed).unwrap();
        let signed = Event::sign(
            decoded.seq,
            decoded.epoch,
            decoded.prev,
            decoded.body.clone(),
            &signer,
        )
        .unwrap();
        assert_eq!(to_canon(&signed).unwrap(), wire_bytes);

        // 6. Se o corpo carrega autoridade, confere authority_message e verify_authority
        if let Some(auth_pub_hex) = vec_val["authority_public_key_hex"].as_str() {
            let auth_pub = PublicKey::new(SigAlg::Ed25519, &unhex(auth_pub_hex))
                .expect("valid authority public key");
            let expected_auth_msg_hex = vec_val["authority_message_hex"]
                .as_str()
                .expect("authority_message_hex");

            let auth_msg = decoded
                .body
                .authority_message()
                .expect("authority message generation")
                .expect("body must have authority message");
            assert_eq!(
                to_hex(&auth_msg),
                expected_auth_msg_hex,
                "vector {id} authority message differs"
            );

            decoded
                .body
                .verify_authority(&auth_pub)
                .unwrap_or_else(|e| panic!("vector {id} authority verify failed: {e:?}"));
        }
    }
    let tags: std::collections::BTreeSet<_> = vectors
        .iter()
        .map(|v| v["body_tag"].as_u64().unwrap())
        .collect();
    assert_eq!(tags, (0..14).collect());
}

// -----------------------------------------------------------------------------
// 2. MUTAÇÃO CRÍTICA: KeyId mutant killer (remocao de public_key.key_id != kernel_key)
// -----------------------------------------------------------------------------

#[test]
fn test_kill_key_id_mutant_cross_signer() {
    // Matador do mutante que remove a verificação `if public_key.key_id()? != self.kernel_key`.
    // Não basta testar chave pública errada (crypto rejeita anyway).
    // Aqui construímos um payload onde `kernel_key` do envelope é a chave B, mas assinado REAL
    // pela chave A.
    // Assim, `crypto::verify` com chave A seria VÁLIDO se a checagem de KeyId for removida!
    // Mas com a checagem, DEVE falhar com Invalid("kernel KeyId mismatch").
    let signer_a = FileSigner::from_seed(&[0x11; 32]).expect("signer A");
    let signer_b = FileSigner::from_seed(&[0x22; 32]).expect("signer B");

    let key_b = signer_b.key_id().expect("key B id");
    let body = EventBody::ObligationCreated {
        obligation: Hash::<Obligation>::from_bytes([0x55; 32]),
        route: Route::Model,
        artifact: [0x66; 32],
        cnf: None,
    };

    // Monta o array unsigned com key_b
    let prev = Some(Hash::<Event>::from_bytes([0x77; 32]));
    let unsigned_items = vec![
        Value::Unsigned(1),
        Value::Unsigned(0),
        1u64.to_value().unwrap(),
        0u64.to_value().unwrap(),
        prev.to_value().unwrap(),
        body.to_value().unwrap(),
        key_b.to_value().unwrap(), // Contém a chave B!
    ];
    let unsigned = Value::Array(Array::new(unsigned_items).unwrap());
    let unsigned_bytes = to_canon(&unsigned).unwrap();

    // Signer A assina esse payload
    let sig_by_a = signer_a.sign::<Event>(&unsigned_bytes).unwrap();

    // Monta o evento resultante com kernel_key de B e sig de A
    let event = Event {
        seq: 1,
        epoch: 0,
        prev,
        body,
        kernel_key: key_b,
        sig: sig_by_a,
    };

    // 1. Verificação contra a chave pública A DEVE falhar por KeyId mismatch,
    //    mesmo que a assinatura Ed25519 em si pertença a A!
    let err = event.verify(signer_a.public_key()).unwrap_err();
    assert!(matches!(err, EventError::Invalid(_)));

    // 2. Verificação contra a chave pública B DEVE falhar por erro criptográfico
    //    (pois os bytes foram assinados por A, não por B)
    let err_b = event.verify(signer_b.public_key()).unwrap_err();
    assert!(
        matches!(err_b, EventError::Crypto(_)),
        "expected Crypto error for signer B, got {err_b:?}"
    );
}

// -----------------------------------------------------------------------------
// 3. MUTAÇÃO: verify::<Event> removido em Event::verify
// -----------------------------------------------------------------------------

#[test]
fn test_kill_verify_crypto_mutant() {
    let kernel = FileSigner::from_seed(&[0x31; 32]).unwrap();
    let event = Event::sign(
        0,
        0,
        None,
        EventBody::Genesis {
            ledger_version: 1,
            kernel_binary: [0x10; 32],
            kernel_key: kernel.key_id().unwrap(),
        },
        &kernel,
    )
    .unwrap();

    // Evento original passa
    event.verify(kernel.public_key()).unwrap();

    // Adulterar um byte da assinatura preservando tudo o mais
    let mut bad_sig_bytes = event.sig.as_bytes().to_vec();
    bad_sig_bytes[0] ^= 0xff;
    let corrupted_sig = Signature::new(SigAlg::Ed25519, &bad_sig_bytes).unwrap();

    let tampered_event = Event {
        sig: corrupted_sig,
        ..event
    };

    let err = tampered_event.verify(kernel.public_key()).unwrap_err();
    assert!(
        matches!(err, EventError::Crypto(_)),
        "corrupted signature must fail with Crypto error, got {err:?}"
    );
}

// -----------------------------------------------------------------------------
// 5. ADULTERAÇÃO DE FOLHAS: Verificação com crypto::verify raw
// -----------------------------------------------------------------------------

fn field_mutations(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(array) => {
            let items = array.items();
            let mut changed = items.to_vec();
            changed.push(Value::Null);
            let mut mutations = vec![Value::Array(Array::new(changed).unwrap())];
            if !items.is_empty() {
                mutations.push(Value::Array(
                    Array::new(items[..items.len() - 1].to_vec()).unwrap(),
                ));
            }
            for (index, item) in items.iter().enumerate() {
                for mutation in field_mutations(item) {
                    let mut changed = items.to_vec();
                    changed[index] = mutation;
                    mutations.push(Value::Array(Array::new(changed).unwrap()));
                }
            }
            mutations
        }
        Value::Map(map) => {
            let mut mutations = Vec::new();
            for (index, (_, value)) in map.entries().iter().enumerate() {
                for mutation in field_mutations(value) {
                    let mut entries = map.entries().to_vec();
                    entries[index].1 = mutation;
                    mutations.push(Value::Map(logos_canon::Map::new(entries).unwrap()));
                }
            }
            mutations
        }
        Value::Null => vec![Value::Bool(false)],
        Value::Bool(value) => vec![Value::Bool(!value)],
        Value::Unsigned(value) => vec![Value::Unsigned(value.wrapping_add(1))],
        Value::Negative(value) => vec![Value::Negative(value.wrapping_add(1))],
        Value::Text(text) => vec![Value::Text(
            Text::new(format!("{}x", text.as_str())).unwrap(),
        )],
        Value::Bytes(bytes) => {
            let mut changed = bytes.clone();
            if let Some(first) = changed.first_mut() {
                *first ^= 1;
            } else {
                changed.push(1);
            }
            vec![Value::Bytes(changed)]
        }
    }
}

#[test]
fn every_field_of_every_body_is_signature_bound() {
    let root: JsonValue =
        serde_json::from_str(include_str!("../../../spec/vectors/events/vectors.json")).unwrap();
    let vectors = root["vectors"].as_array().unwrap();
    let mut checked = 0;
    for vector in vectors {
        let wire = unhex(vector["event_wire_hex"].as_str().unwrap());
        let event = Event::decode(&wire).unwrap();
        let public = PublicKey::new(
            SigAlg::Ed25519,
            &unhex(vector["kernel_public_key_hex"].as_str().unwrap()),
        )
        .unwrap();
        let unsigned: Value =
            from_canon(&unhex(vector["unsigned_canon_hex"].as_str().unwrap())).unwrap();
        logos_crypto::verify::<Event>(&to_canon(&unsigned).unwrap(), &event.sig, &public).unwrap();
        for mutation in field_mutations(&unsigned) {
            let bytes = to_canon(&mutation).unwrap();
            assert!(
                logos_crypto::verify::<Event>(&bytes, &event.sig, &public).is_err(),
                "field mutation accepted: {}",
                vector["id"]
            );
            let Value::Array(array) = mutation else {
                unreachable!()
            };
            let mut fields = array.items().to_vec();
            fields.push(event.sig.to_value().unwrap());
            let changed = Value::Array(Array::new(fields).unwrap());
            if let Ok(parsed) = Event::from_value(&changed) {
                assert!(
                    parsed.verify(&public).is_err(),
                    "Event::verify lost signed field"
                );
            }
            checked += 1;
        }
        for mutation in field_mutations(&event.sig.to_value().unwrap()) {
            let Value::Array(array) = event.to_value().unwrap() else {
                unreachable!()
            };
            let mut fields = array.items().to_vec();
            fields[7] = mutation;
            let changed = Value::Array(Array::new(fields).unwrap());
            if let Ok(parsed) = Event::from_value(&changed) {
                assert!(parsed.verify(&public).is_err());
            }
            checked += 1;
        }
    }
    println!(
        "EVENT_FIELDS_TAMPER_OK vectors={} mutations={checked}",
        vectors.len()
    );
}

// -----------------------------------------------------------------------------
// 6. INVARIANTES DO CATÁLOGO: Outcomes Accepted / Rejected / Unknown
// -----------------------------------------------------------------------------

#[test]
fn test_invariants_rejected_and_invalid_outcomes() {
    let rejected = CheckOutcome::Rejected(Reason::new(Text::new("error").unwrap()));
    let accepted = CheckOutcome::Accepted;
    let unknown = CheckOutcome::Unknown(Exhausted::Steps);

    // 1. Verified com Rejected -> REJEITADO
    let body_verified_rej = EventBody::Verified {
        check: sample_check_record(rejected.clone()),
        cited: vec![],
        used: vec![],
        consistency: Consistency::Unknown { check: None },
    };
    assert!(matches!(
        body_verified_rej.validate(),
        Err(EventError::Invalid(_))
    ));

    // 2. Verified com Unknown -> REJEITADO
    let body_verified_unk = EventBody::Verified {
        check: sample_check_record(unknown.clone()),
        cited: vec![],
        used: vec![],
        consistency: Consistency::Unknown { check: None },
    };
    assert!(body_verified_unk.validate().is_err());

    // 3. Refuted com Rejected -> REJEITADO
    let body_refuted_rej = EventBody::Refuted {
        check: sample_check_record(rejected),
        model: Hash::from_bytes([0x01; 32]),
    };
    assert!(matches!(
        body_refuted_rej.validate(),
        Err(EventError::Invalid(_))
    ));

    // 4. UnknownRecorded com Accepted -> REJEITADO
    let body_unknown_acc = EventBody::UnknownRecorded {
        check: sample_check_record(accepted),
        machine: Text::new("m").unwrap(),
    };
    assert!(matches!(
        body_unknown_acc.validate(),
        Err(EventError::Invalid(_))
    ));
}

// -----------------------------------------------------------------------------
// 7. INVARIANTES DO CATÁLOGO: used subconjunto estrito de cited e ordenação
// -----------------------------------------------------------------------------

#[test]
fn test_invariants_cited_and_used_refs() {
    let h1 = Hash::<Event>::from_bytes([0x10; 32]);
    let h2 = Hash::<Event>::from_bytes([0x20; 32]);
    let h3 = Hash::<Event>::from_bytes([0x30; 32]);

    // 1. used fora de cited -> REJEITADO
    let body_subset_err = EventBody::Verified {
        check: sample_check_record(CheckOutcome::Accepted),
        cited: vec![h1, h2],
        used: vec![h3], // h3 não está em cited!
        consistency: Consistency::Unknown { check: None },
    };
    assert!(matches!(
        body_subset_err.validate(),
        Err(EventError::Invalid(_))
    ));

    // 2. cited com duplicatas -> REJEITADO
    let body_dup = EventBody::Verified {
        check: sample_check_record(CheckOutcome::Accepted),
        cited: vec![h1, h1],
        used: vec![h1],
        consistency: Consistency::Unknown { check: None },
    };
    assert!(matches!(body_dup.validate(), Err(EventError::Invalid(_))));

    // 3. cited desordenado -> REJEITADO
    let body_unsorted = EventBody::Verified {
        check: sample_check_record(CheckOutcome::Accepted),
        cited: vec![h2, h1],
        used: vec![h1],
        consistency: Consistency::Unknown { check: None },
    };
    assert!(matches!(
        body_unsorted.validate(),
        Err(EventError::Invalid(_))
    ));
}

// -----------------------------------------------------------------------------
// 8. INVARIANTES DO CATÁLOGO: Consistência Witness e Unknown
// -----------------------------------------------------------------------------

#[test]
fn test_invariants_consistency_rules() {
    let main_check = sample_check_record(CheckOutcome::Accepted);
    let other_obligation = Hash::<Obligation>::from_bytes([0xee; 32]);

    // 1. Witness com obrigação diferente do check principal -> REJEITADO
    let witness_mismatch_ob = Consistency::Witness {
        model: Hash::from_bytes([0x44; 32]),
        check: CheckRecord {
            obligation: other_obligation,
            ..main_check.clone()
        },
    };
    let body_wit_ob = EventBody::Verified {
        check: main_check.clone(),
        cited: vec![],
        used: vec![],
        consistency: witness_mismatch_ob,
    };
    assert!(matches!(
        body_wit_ob.validate(),
        Err(EventError::Invalid(_))
    ));

    // 2. Witness com check NÃO Accepted -> REJEITADO
    let witness_non_acc = Consistency::Witness {
        model: Hash::from_bytes([0x44; 32]),
        check: CheckRecord {
            outcome: CheckOutcome::Unknown(Exhausted::Memory),
            ..main_check.clone()
        },
    };
    let body_wit_non_acc = EventBody::Verified {
        check: main_check.clone(),
        cited: vec![],
        used: vec![],
        consistency: witness_non_acc,
    };
    assert!(matches!(
        body_wit_non_acc.validate(),
        Err(EventError::Invalid(_))
    ));

    // 3. Consistency::Unknown com check Accepted -> REJEITADO
    let unknown_with_accepted = Consistency::Unknown {
        check: Some(main_check.clone()),
    };
    let body_unk_acc = EventBody::Verified {
        check: main_check.clone(),
        cited: vec![],
        used: vec![],
        consistency: unknown_with_accepted,
    };
    assert!(matches!(
        body_unk_acc.validate(),
        Err(EventError::Invalid(_))
    ));
}

// -----------------------------------------------------------------------------
// 9. INVARIANTES DO ENVELOPE: Genesis vs Não-Genesis e EpochAdvanced
// -----------------------------------------------------------------------------

#[test]
fn test_invariants_envelope_rules() {
    let kernel = FileSigner::from_seed(&[0x88; 32]).unwrap();
    let other_signer = FileSigner::from_seed(&[0x99; 32]).unwrap();
    let kernel_key = kernel.key_id().unwrap();

    let genesis_body = EventBody::Genesis {
        ledger_version: 1,
        kernel_binary: [0x11; 32],
        kernel_key,
    };

    // 1. Genesis com seq != 0 -> REJEITADO
    assert!(Event::sign(1, 0, None, genesis_body.clone(), &kernel).is_err());

    // 2. Genesis com epoch != 0 -> REJEITADO
    assert!(Event::sign(0, 1, None, genesis_body.clone(), &kernel).is_err());

    // 3. Genesis com prev Some -> REJEITADO
    assert!(
        Event::sign(
            0,
            0,
            Some(Hash::from_bytes([0xaa; 32])),
            genesis_body.clone(),
            &kernel
        )
        .is_err()
    );

    // 4. Genesis com chave do body diferente da chave do assinante -> REJEITADO
    let genesis_wrong_key = EventBody::Genesis {
        ledger_version: 1,
        kernel_binary: [0x11; 32],
        kernel_key: other_signer.key_id().unwrap(),
    };
    assert!(Event::sign(0, 0, None, genesis_wrong_key, &kernel).is_err());

    // 5. Não-Genesis com seq == 0 -> REJEITADO
    let non_genesis_body = EventBody::ObligationCreated {
        obligation: Hash::from_bytes([0x01; 32]),
        route: Route::Model,
        artifact: [0x02; 32],
        cnf: None,
    };
    assert!(
        Event::sign(
            0,
            0,
            Some(Hash::from_bytes([0xaa; 32])),
            non_genesis_body.clone(),
            &kernel
        )
        .is_err()
    );

    // 6. Não-Genesis com prev None -> REJEITADO
    assert!(Event::sign(1, 0, None, non_genesis_body, &kernel).is_err());

    // 7. EpochAdvanced com new_epoch != epoch do envelope -> REJEITADO
    let epoch_body = EventBody::EpochAdvanced {
        policy_root: None,
        vocab: None,
        new_epoch: 2,
        reason: Text::new("avanco").unwrap(),
    };
    // Tentativa com epoch 1 no envelope e new_epoch 2 no body
    assert!(
        Event::sign(
            5,
            1,
            Some(Hash::from_bytes([0xaa; 32])),
            epoch_body,
            &kernel
        )
        .is_err()
    );
}

// -----------------------------------------------------------------------------
// 10. PRESERVAÇÃO DE TEXTO TAINTED (Não-NFC UTF-8)
// -----------------------------------------------------------------------------

#[test]
fn test_tainted_text_preserves_non_nfc_and_roundtrips() {
    // "não café" com til e acento em forma NFD (decomposta, não NFC)
    let nfd_str = "na\u{0303}o cafe\u{0301}";

    let tainted = TaintedText::new(nfd_str.to_string()).expect("within 64 KiB");
    assert_eq!(tainted.as_untrusted_str(), nfd_str);
    assert_eq!(tainted.as_bytes(), nfd_str.as_bytes());

    // Codificação e decodificação canônica preservam exatamente os bytes sem normalizar
    let val = tainted.to_value().unwrap();
    assert!(matches!(val, Value::Bytes(_)));

    let decoded = TaintedText::from_value(&val).unwrap();
    assert_eq!(decoded.as_untrusted_str(), nfd_str);
}

// -----------------------------------------------------------------------------
// 11. LIMITES INCLUSIVOS E ERROS TIPADOS
// -----------------------------------------------------------------------------

#[test]
fn test_limits_and_bounds_enforcement() {
    // 1. TaintedText > 64 KiB -> Limit
    let huge_str = "a".repeat(64 * 1024 + 1);
    let err = TaintedText::new(huge_str).unwrap_err();
    assert!(matches!(err, EventError::Limit(_)));

    // 2. Causes vazia em Invalidated -> Invalid
    let empty_causes = EventBody::Invalidated {
        claim: Hash::from_bytes([0x01; 32]),
        causes: vec![],
    };
    assert!(matches!(
        empty_causes.validate(),
        Err(EventError::Invalid(_))
    ));

    // 3. Causes > 1024 -> Limit
    let huge_causes = vec![Hash::from_bytes([0x01; 32]); 1025];
    let body_huge_causes = EventBody::Invalidated {
        claim: Hash::from_bytes([0x01; 32]),
        causes: huge_causes,
    };
    assert!(matches!(
        body_huge_causes.validate().unwrap_err(),
        EventError::Limit(_)
    ));
}

// -----------------------------------------------------------------------------
// 12. BYTES EXTRAS E FORMATOS DESCONHECIDOS SÃO RECUSADOS
// -----------------------------------------------------------------------------

#[test]
fn test_extra_trailing_bytes_and_invalid_formats() {
    let kernel = FileSigner::from_seed(&[0x44; 32]).unwrap();
    let event = Event::sign(
        0,
        0,
        None,
        EventBody::Genesis {
            ledger_version: 1,
            kernel_binary: [0x11; 32],
            kernel_key: kernel.key_id().unwrap(),
        },
        &kernel,
    )
    .unwrap();

    let mut wire = to_canon(&event).unwrap();

    // 1. Bytes sobrando após evento canônico
    wire.push(0x00);
    assert!(
        Event::decode(&wire).is_err(),
        "trailing bytes must be rejected"
    );

    // 2. Wire maior que MAX_EVENT_BYTES (16 MiB)
    assert!(
        Event::decode(&vec![0x00; MAX_EVENT_BYTES + 1]).is_err(),
        "payload exceeding 16 MiB must be rejected before parsing"
    );
}

#[test]
fn decoder_and_encoder_preserve_outcome_boundaries() {
    let outcomes = [
        CheckOutcome::Accepted,
        CheckOutcome::Unknown(Exhausted::Size),
        CheckOutcome::Rejected(Reason::new(Text::new("inválido").unwrap())),
    ];
    for outcome in outcomes {
        let check = sample_check_record(outcome.clone()).to_value().unwrap();
        for tag in [4, 5, 6] {
            let fields = match tag {
                4 => vec![
                    check.clone(),
                    Value::Array(Array::new(vec![]).unwrap()),
                    Value::Array(Array::new(vec![]).unwrap()),
                    Consistency::Unknown { check: None }.to_value().unwrap(),
                ],
                5 => vec![check.clone(), Value::Bytes(vec![3; 32])],
                _ => vec![
                    check.clone(),
                    Text::new("máquina").unwrap().to_value().unwrap(),
                ],
            };
            let value = Value::Array(
                Array::new([vec![Value::Unsigned(1), Value::Unsigned(tag)], fields].concat())
                    .unwrap(),
            );
            let decoded = EventBody::from_value(&value);
            let expected = if tag == 6 {
                matches!(outcome, CheckOutcome::Unknown(_))
            } else {
                outcome == CheckOutcome::Accepted
            };
            assert_eq!(decoded.is_ok(), expected);
            if let Ok(body) = decoded {
                assert_eq!(body.to_value().unwrap(), value);
            }
        }
        let unknown = Value::Array(
            Array::new(vec![
                Value::Unsigned(0),
                Value::Array(Array::new(vec![check.clone()]).unwrap()),
            ])
            .unwrap(),
        );
        assert_eq!(
            Consistency::from_value(&unknown).is_ok(),
            outcome != CheckOutcome::Accepted
        );
        let witness = Value::Array(
            Array::new(vec![Value::Unsigned(1), Value::Bytes(vec![4; 32]), check]).unwrap(),
        );
        assert_eq!(
            Consistency::from_value(&witness).is_ok(),
            outcome == CheckOutcome::Accepted
        );
    }
}

#[test]
fn presentation_fields_have_symmetric_inclusive_limits() {
    let goal = Formula::new(vec![FormulaNode::True], Limits::default()).unwrap();
    let mut proposal = EventProposal {
        text_pt: TaintedText::new("á".repeat(32768)).unwrap(),
        goal,
        cites: vec![Hash::from_bytes([3; 32]); 1024],
        speculative_on: vec![[4; 32]; 1024],
        vocabulary: Hash::<Vocabulary>::from_bytes([5; 32]),
    };
    let wire = to_canon(&proposal).unwrap();
    assert_eq!(from_canon::<EventProposal>(&wire).unwrap(), proposal);
    proposal.cites.push(Hash::from_bytes([6; 32]));
    assert!(to_canon(&proposal).is_err());
    proposal.cites.pop();
    proposal.speculative_on.push([7; 32]);
    assert!(to_canon(&proposal).is_err());
    let raw = Value::Bytes(vec![b'a'; 65537]);
    assert!(TaintedText::from_value(&raw).is_err());
    assert!(TaintedText::from_value(&Value::Bytes(vec![0xff])).is_err());

    let mut nodes = vec![FormulaNode::Not; 128];
    nodes.push(FormulaNode::True);
    proposal.goal = Formula::new(
        nodes,
        Limits {
            max_depth: 129,
            max_nodes: 1000,
        },
    )
    .unwrap();
    proposal.speculative_on.pop();
    assert!(
        to_canon(&proposal).is_err(),
        "encoder must enforce AIR defaults too"
    );

    let bad_record = sample_check_record(CheckOutcome::Rejected(Reason::new(
        Text::new("x".repeat(65537)).unwrap(),
    )));
    assert!(to_canon(&bad_record).is_err());
    let raw_record = Value::Array(
        Array::new(vec![
            Value::Bytes(vec![1; 32]),
            Value::Bytes(vec![2; 32]),
            Value::Bytes(vec![3; 32]),
            default_budget().to_value().unwrap(),
            bad_record.outcome.to_value().unwrap(),
        ])
        .unwrap(),
    );
    assert!(CheckRecord::from_value(&raw_record).is_err());
}

#[test]
fn unknown_versions_tags_routes_and_extra_fields_are_not_accepted() {
    let root: JsonValue =
        serde_json::from_str(include_str!("../../../spec/vectors/events/vectors.json")).unwrap();
    let wire = unhex(root["vectors"][0]["event_wire_hex"].as_str().unwrap());
    let value: Value = from_canon(&wire).unwrap();
    let Value::Array(array) = value else {
        unreachable!()
    };
    for (index, replacement) in [(0, Value::Unsigned(2)), (1, Value::Unsigned(1))] {
        let mut fields = array.items().to_vec();
        fields[index] = replacement;
        assert!(Event::from_value(&Value::Array(Array::new(fields).unwrap())).is_err());
    }
    let mut extra = array.items().to_vec();
    extra.push(Value::Null);
    assert!(Event::from_value(&Value::Array(Array::new(extra).unwrap())).is_err());
    assert!(
        EventBody::from_value(&Value::Array(
            Array::new(vec![Value::Unsigned(1), Value::Unsigned(13)]).unwrap()
        ))
        .is_err()
    );
    assert!(Route::from_value(&Value::Unsigned(4)).is_err());
}

#[test]
fn extended_epoch_body_has_one_strict_encoding_and_preserves_legacy_bytes() {
    let root: JsonValue =
        serde_json::from_str(include_str!("../../../spec/vectors/events/vectors.json")).unwrap();
    let mut cases = 0;
    for vector in root["vectors"].as_array().unwrap() {
        if vector["body_tag"] != 10 {
            continue;
        }
        let wire = unhex(vector["event_wire_hex"].as_str().unwrap());
        let event = Event::decode(&wire).unwrap();
        let value = event.body.to_value().unwrap();
        let Value::Array(fields) = &value else {
            unreachable!()
        };
        let EventBody::EpochAdvanced {
            policy_root, vocab, ..
        } = event.body
        else {
            unreachable!()
        };
        let extended = policy_root.is_some() || vocab.is_some();
        assert_eq!(
            fields.items()[0],
            Value::Unsigned(if extended { 2 } else { 1 })
        );
        assert_eq!(fields.items().len(), if extended { 6 } else { 4 });
        assert_eq!(
            to_canon(&EventBody::from_value(&value).unwrap()).unwrap(),
            to_canon(&value).unwrap()
        );
        if !extended {
            assert_eq!(to_canon(&event).unwrap(), wire);
        }
        cases += 1;
    }
    assert_eq!(cases, 5); // v1, root, vocab, ambos, limites máximos.

    let array = |fields| Value::Array(Array::new(fields).unwrap());
    let empty = array(vec![]);
    let some = array(vec![Value::Bytes(vec![7; 32])]);
    let fields = vec![
        Value::Unsigned(2),
        Value::Unsigned(10),
        Value::Unsigned(4),
        Text::new("época").unwrap().to_value().unwrap(),
        some.clone(),
        some.clone(),
    ];
    for slot in [4, 5] {
        for bad in [
            Value::Null,
            Value::Bool(false),
            Value::Unsigned(0),
            Value::Bytes(vec![7; 32]),
            array(vec![Value::Bytes(vec![7; 31])]),
            array(vec![Value::Bytes(vec![7; 33])]),
            array(vec![some.clone(), some.clone()]),
        ] {
            let mut malformed = fields.clone();
            malformed[slot] = bad;
            assert!(EventBody::from_value(&array(malformed)).is_err());
        }
    }
    let mut absent = fields.clone();
    absent[4] = empty.clone();
    absent[5] = empty;
    assert!(EventBody::from_value(&array(absent)).is_err());
    for version in [0, 1, 3, u64::MAX] {
        let mut malformed = fields.clone();
        malformed[0] = Value::Unsigned(version);
        assert!(EventBody::from_value(&array(malformed)).is_err());
    }
    for tag in 0..=13 {
        if tag == 10 {
            continue;
        }
        let mut malformed = fields.clone();
        malformed[1] = Value::Unsigned(tag);
        assert!(EventBody::from_value(&array(malformed)).is_err());
    }
    for length in 0..fields.len() {
        assert!(EventBody::from_value(&array(fields[..length].to_vec())).is_err());
    }
    let mut extra = fields.clone();
    extra.push(Value::Null);
    assert!(EventBody::from_value(&array(extra)).is_err());
    let mut huge = fields;
    huge[3] = Text::new("x".repeat(65537)).unwrap().to_value().unwrap();
    assert!(EventBody::from_value(&array(huge)).is_err());
}

#[test]
fn epoch_commitments_are_bound_to_signature_without_conferring_authority() {
    let signer = FileSigner::from_seed(&[0x11; 32]).unwrap();
    let event = Event::sign(
        1,
        4,
        Some(Hash::from_bytes([1; 32])),
        EventBody::EpochAdvanced {
            new_epoch: 4,
            reason: Text::new("contexto").unwrap(),
            policy_root: Some([2; 32]),
            vocab: Some(Hash::from_bytes([3; 32])),
        },
        &signer,
    )
    .unwrap();
    event.verify(signer.public_key()).unwrap();
    assert!(event.body.authority_message().unwrap().is_none());
    for slot in 0..4 {
        let mut tampered = event.clone();
        let EventBody::EpochAdvanced {
            policy_root, vocab, ..
        } = &mut tampered.body
        else {
            unreachable!()
        };
        match slot {
            0 => *policy_root = Some([9; 32]),
            1 => *vocab = Some(Hash::from_bytes([9; 32])),
            2 => *policy_root = None,
            _ => *vocab = None,
        }
        assert_ne!(hash(&tampered).unwrap(), hash(&event).unwrap());
        assert!(tampered.verify(signer.public_key()).is_err());
    }
}

#[test]
fn authority_receipt_binds_body_identity_and_is_separate_from_kernel_signature() {
    let root: JsonValue =
        serde_json::from_str(include_str!("../../../spec/vectors/events/vectors.json")).unwrap();
    for vector in root["vectors"].as_array().unwrap() {
        let Some(public_hex) = vector["authority_public_key_hex"].as_str() else {
            continue;
        };
        let public = PublicKey::new(SigAlg::Ed25519, &unhex(public_hex)).unwrap();
        let event = Event::decode(&unhex(vector["event_wire_hex"].as_str().unwrap())).unwrap();
        let body = event.body.to_value().unwrap();
        for mutated in field_mutations(&body) {
            if let Ok(mutated) = EventBody::from_value(&mutated) {
                assert!(mutated.verify_authority(&public).is_err());
            }
        }

        let seed: [u8; 32] = unhex(vector["authority_seed_hex"].as_str().unwrap())
            .try_into()
            .unwrap();
        let admin = FileSigner::from_seed(&seed).unwrap();
        let other = FileSigner::from_seed(&[0x71; 32]).unwrap();
        let mut body = event.body;
        match &mut body {
            EventBody::VocabPublished { authority, .. }
            | EventBody::Retracted { authority, .. }
            | EventBody::CheckerRegistered { authority, .. }
            | EventBody::CheckerRevoked { authority, .. } => {
                authority.key = other.key_id().unwrap()
            }
            _ => unreachable!(),
        }
        // Signature is cryptographically valid under admin A, but receipt claims B.
        let message = body.authority_message().unwrap().unwrap();
        let signature = admin.sign::<Event>(&message).unwrap();
        logos_crypto::verify::<Event>(&message, &signature, admin.public_key()).unwrap();
        match &mut body {
            EventBody::VocabPublished { authority, .. }
            | EventBody::Retracted { authority, .. }
            | EventBody::CheckerRegistered { authority, .. }
            | EventBody::CheckerRevoked { authority, .. } => authority.sig = signature,
            _ => unreachable!(),
        }
        assert!(matches!(
            body.verify_authority(admin.public_key()),
            Err(EventError::Invalid(_))
        ));
        assert!(matches!(
            body.verify_authority(other.public_key()),
            Err(EventError::Crypto(_))
        ));
        let kernel_seed: [u8; 32] = unhex(vector["kernel_seed_hex"].as_str().unwrap())
            .try_into()
            .unwrap();
        let kernel = FileSigner::from_seed(&kernel_seed).unwrap();
        let event = Event::sign(event.seq, event.epoch, event.prev, body, &kernel).unwrap();
        event.verify(kernel.public_key()).unwrap();
        assert!(event.body.verify_authority(admin.public_key()).is_err());
    }
}
