//! Vetores canônicos independentes de model/v1, conforme `spec/model.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fmt::Write;

use logos_air::Vocabulary;
use logos_canon::from_canon;
use logos_crypto::hash_raw;
use logos_model::{FiniteModel, ModelLimits};
use serde_json::Value;

fn unhex(hex: &str) -> Vec<u8> {
    assert!(hex.len().is_multiple_of(2), "hex length must be even");
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("valid hex byte"))
        .collect()
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(&mut s, "{b:02x}").expect("hex write");
    }
    s
}

#[test]
fn canonical_vectors() {
    let raw_json = include_str!("../../../spec/vectors/model/vectors.json");
    let root: Value = serde_json::from_str(raw_json).expect("valid vectors JSON");

    assert_eq!(
        root["format"].as_str().expect("format string"),
        "model/v1",
        "format must be model/v1"
    );

    let vectors = root["vectors"].as_array().expect("vectors array");

    for vec_val in vectors {
        let id = vec_val["id"].as_str().expect("id string");
        let vocab_hex = vec_val["vocabulary_hex"].as_str().expect("vocabulary_hex");
        let model_hex = vec_val["model_hex"].as_str().expect("model_hex");
        let expected_error = vec_val["expected_error"].as_str();

        let vocab_bytes = unhex(vocab_hex);
        let vocabulary = from_canon::<Vocabulary>(&vocab_bytes).expect("valid vocabulary");
        let model_bytes = unhex(model_hex);
        let limits = ModelLimits::default();

        match expected_error {
            None => {
                let model = FiniteModel::decode_with_limits(&model_bytes, &vocabulary, limits)
                    .unwrap_or_else(|e| panic!("vector {id} expected success, got error: {e:?}"));

                // 1. Re-codificação canônica deve coincidir exatamente com os bytes do vetor
                let encoded = model
                    .encode_with_limits(limits)
                    .unwrap_or_else(|e| panic!("vector {id} encode failed: {e:?}"));
                assert_eq!(
                    to_hex(&encoded),
                    model_hex,
                    "vector {id} encoded bytes mismatch"
                );

                // 2. Hash do modelo deve coincidir com model_hash do vetor
                let expected_hash = vec_val["model_hash"].as_str().expect("model_hash");
                let actual_hash = to_hex(hash_raw::<FiniteModel>(&model_bytes).as_bytes());
                assert_eq!(
                    actual_hash, expected_hash,
                    "vector {id} model hash mismatch"
                );

                // 3. Estrutura dos dados decodificados deve coincidir com expected
                let expected_data = &vec_val["expected"];
                let data = model.data();

                assert_eq!(
                    to_hex(data.vocab.as_bytes()),
                    expected_data["vocab"].as_str().expect("vocab hex"),
                    "vector {id} vocab hash mismatch"
                );

                let exp_domains: Vec<u32> = expected_data["domains"]
                    .as_array()
                    .expect("domains")
                    .iter()
                    .map(|v| v.as_u64().expect("domain u64") as u32)
                    .collect();
                assert_eq!(data.domains, exp_domains, "vector {id} domains mismatch");

                let exp_consts: Vec<(u32, u32)> = expected_data["consts"]
                    .as_array()
                    .expect("consts")
                    .iter()
                    .map(|pair| {
                        let p = pair.as_array().expect("const pair");
                        (
                            p[0].as_u64().expect("symbol id") as u32,
                            p[1].as_u64().expect("value") as u32,
                        )
                    })
                    .collect();
                assert_eq!(data.consts, exp_consts, "vector {id} consts mismatch");

                let exp_funcs: Vec<(u32, Vec<u32>)> = expected_data["funcs"]
                    .as_array()
                    .expect("funcs")
                    .iter()
                    .map(|f_entry| {
                        let arr = f_entry.as_array().expect("func entry");
                        let sym_id = arr[0].as_u64().expect("func symbol id") as u32;
                        let outputs: Vec<u32> = arr[1]
                            .as_array()
                            .expect("func outputs")
                            .iter()
                            .map(|v| v.as_u64().expect("output value") as u32)
                            .collect();
                        (sym_id, outputs)
                    })
                    .collect();
                assert_eq!(data.funcs, exp_funcs, "vector {id} funcs mismatch");

                let exp_preds: Vec<(u32, Vec<Vec<u32>>)> = expected_data["preds"]
                    .as_array()
                    .expect("preds")
                    .iter()
                    .map(|p_entry| {
                        let arr = p_entry.as_array().expect("pred entry");
                        let sym_id = arr[0].as_u64().expect("pred symbol id") as u32;
                        let tuples: Vec<Vec<u32>> = arr[1]
                            .as_array()
                            .expect("tuples")
                            .iter()
                            .map(|t| {
                                t.as_array()
                                    .expect("tuple")
                                    .iter()
                                    .map(|v| v.as_u64().expect("cell value") as u32)
                                    .collect()
                            })
                            .collect();
                        (sym_id, tuples)
                    })
                    .collect();
                assert_eq!(data.preds, exp_preds, "vector {id} preds mismatch");
            }
            Some(expected_err_str) => {
                let err = FiniteModel::decode_with_limits(&model_bytes, &vocabulary, limits)
                    .expect_err(&format!(
                        "vector {id} expected error {expected_err_str}, but succeeded"
                    ));
                let err_debug = format!("{err:?}");
                assert_eq!(
                    err_debug, expected_err_str,
                    "vector {id} error mismatch: expected {expected_err_str}, got {err_debug}"
                );
            }
        }
    }
}
