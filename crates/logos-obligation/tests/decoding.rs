//! Suíte de testes para decodificação contextual de `obligation/v1`.
//!
//! Contrato normativo: `spec/obligation.md` §2; ADR 0016; etapa 2.3.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fmt::Write;

use logos_air::{
    AirError, Formula, FormulaNode, Limits, Sort, SortDecl, SortDomain, SortError, SortErrorKind,
    Term, TermNode, Vocabulary,
};
use logos_canon::{Array, Canon, CanonError, Text, Value, from_canon, to_canon};
use logos_core::Exhausted;
use logos_crypto::{Hash, hash, hash_raw};
use logos_obligation::{DecodeError, DecodeLimits, Obligation, decode_for_check};
use serde_json::Value as JsonValue;

fn unhex(hex: &str) -> Vec<u8> {
    assert!(
        hex.len().is_multiple_of(2),
        "hex string must have even length: {hex}"
    );
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .unwrap_or_else(|_| panic!("byte hex inválido: {}", &hex[i..i + 2]))
        })
        .collect()
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(&mut s, "{:02x}", b).expect("hex write");
    }
    s
}

fn parse_hash<T: logos_crypto::Domain>(hex: &str) -> Hash<T> {
    let bytes = unhex(hex);
    assert_eq!(bytes.len(), 32, "hash deve ter 32 bytes");
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Hash::from_bytes(arr)
}

/// Cria um envelope canônico raw a partir de seus componentes para testes de recusa.
fn craft_envelope(
    version: u64,
    type_tag: u64,
    logic: &str,
    encoder_version: u64,
    vocab_hash: [u8; 32],
    raw_premises: Value,
    goal: Value,
) -> Vec<u8> {
    let envelope = Array::new(vec![
        Value::Unsigned(version),
        Value::Unsigned(type_tag),
        Value::Text(Text::new(logic).expect("valid logic text")),
        Value::Unsigned(encoder_version),
        Value::Bytes(vocab_hash.to_vec()),
        raw_premises,
        goal,
    ])
    .expect("valid envelope array");
    let mut bytes = Vec::new();
    Value::Array(envelope).encode_into(&mut bytes);
    bytes
}

#[test]
fn test_decode_all_20_positive_vectors() {
    let raw_json = include_str!("../../../spec/vectors/obligation/vectors.json");
    let root: JsonValue = serde_json::from_str(raw_json).expect("valid vectors JSON");
    let vectors = root["vectors"].as_array().expect("vectors array");

    for vec_val in vectors {
        let id = vec_val["id"].as_str().expect("id string");
        let description = vec_val["description"].as_str().expect("description");

        let target_v_hex = vec_val["target_vocabulary_hex"]
            .as_str()
            .expect("target_vocab_hex");
        let target_vocab_bytes = unhex(target_v_hex);
        let target_vocab =
            from_canon::<Vocabulary>(&target_vocab_bytes).expect("target vocabulary from_canon");

        let goal_hex = vec_val["goal_hex"].as_str().expect("goal_hex");
        let goal_bytes = unhex(goal_hex);
        let expected_goal = from_canon::<Formula>(&goal_bytes).expect("goal formula from_canon");

        let expected_val = &vec_val["expected"];
        let canonical_hex = expected_val["canonical_hex"]
            .as_str()
            .expect("canonical_hex");
        let canonical_bytes = unhex(canonical_hex);

        let obligation_hash_hex = expected_val["obligation_hash"]
            .as_str()
            .expect("obligation_hash");
        let expected_hash = parse_hash::<Obligation>(obligation_hash_hex);

        let expected_vocab_hash_hex = expected_val["vocab_hash"].as_str().expect("vocab_hash");
        let expected_vocab_hash = parse_hash::<Vocabulary>(expected_vocab_hash_hex);

        // Decodificação sob limites padrão
        let checked = decode_for_check(
            &canonical_bytes,
            expected_hash,
            &target_vocab,
            DecodeLimits::default(),
        )
        .unwrap_or_else(|err| {
            panic!("{id} ({description}) falhou na decodificação com erro: {err:?}")
        });

        // Verificações de integridade do CheckedObligation
        assert_eq!(
            checked.vocab(),
            expected_vocab_hash,
            "{id}: vocab hash diverge"
        );
        assert_eq!(checked.goal(), &expected_goal, "{id}: goal formula diverge");

        let expected_canonical_premises = expected_val["canonical_premises"]
            .as_array()
            .expect("canonical_premises");
        assert_eq!(
            checked.premises().len(),
            expected_canonical_premises.len(),
            "{id}: premises count diverge"
        );

        for (i, (actual_ref, actual_formula)) in checked.premises().iter().enumerate() {
            let exp_premise = &expected_canonical_premises[i];
            let exp_ref_hex = exp_premise["reference"].as_str().expect("exp ref hex");
            assert_eq!(
                to_hex(actual_ref.event_hash()),
                exp_ref_hex,
                "{id}: premise {i} reference diverge"
            );

            let exp_formula_hex = exp_premise["formula_hex"]
                .as_str()
                .expect("exp formula hex");
            let exp_formula =
                from_canon::<Formula>(&unhex(exp_formula_hex)).expect("premise formula");
            assert_eq!(
                actual_formula, &exp_formula,
                "{id}: premise {i} formula diverge"
            );
        }
    }
}

#[test]
fn test_decode_hash_mismatch() {
    let raw_json = include_str!("../../../spec/vectors/obligation/vectors.json");
    let root: JsonValue = serde_json::from_str(raw_json).expect("valid vectors JSON");
    let vec_val = &root["vectors"][0];

    let target_vocab =
        from_canon::<Vocabulary>(&unhex(vec_val["target_vocabulary_hex"].as_str().unwrap()))
            .unwrap();
    let canonical_bytes = unhex(vec_val["expected"]["canonical_hex"].as_str().unwrap());
    let mut hash_bytes =
        *parse_hash::<Obligation>(vec_val["expected"]["obligation_hash"].as_str().unwrap())
            .as_bytes();

    // Corrompe 1 bit do hash esperado
    hash_bytes[0] ^= 0x01;
    let corrupted_hash = Hash::<Obligation>::from_bytes(hash_bytes);

    let res = decode_for_check(
        &canonical_bytes,
        corrupted_hash,
        &target_vocab,
        DecodeLimits::default(),
    );
    assert!(
        matches!(res, Err(DecodeError::HashMismatch)),
        "esperava HashMismatch, obteve: {res:?}"
    );
}

#[test]
fn test_decode_vocabulary_mismatch() {
    let raw_json = include_str!("../../../spec/vectors/obligation/vectors.json");
    let root: JsonValue = serde_json::from_str(raw_json).expect("valid vectors JSON");

    let vec0 = &root["vectors"][0];
    let vec1 = &root["vectors"][1];

    let target_vocab_2 =
        from_canon::<Vocabulary>(&unhex(vec1["target_vocabulary_hex"].as_str().unwrap())).unwrap();

    let canonical_bytes_1 = unhex(vec0["expected"]["canonical_hex"].as_str().unwrap());
    let expected_hash_1 =
        parse_hash::<Obligation>(vec0["expected"]["obligation_hash"].as_str().unwrap());

    // Valida obrigação do vocabulário 1 contra o vocabulário 2
    let res = decode_for_check(
        &canonical_bytes_1,
        expected_hash_1,
        &target_vocab_2,
        DecodeLimits::default(),
    );
    assert!(
        matches!(res, Err(DecodeError::VocabularyMismatch)),
        "esperava VocabularyMismatch, obteve: {res:?}"
    );
}

#[test]
fn test_decode_bytes_limit() {
    let raw_json = include_str!("../../../spec/vectors/obligation/vectors.json");
    let root: JsonValue = serde_json::from_str(raw_json).expect("valid vectors JSON");
    let vec_val = &root["vectors"][0];

    let target_vocab =
        from_canon::<Vocabulary>(&unhex(vec_val["target_vocabulary_hex"].as_str().unwrap()))
            .unwrap();
    let canonical_bytes = unhex(vec_val["expected"]["canonical_hex"].as_str().unwrap());
    let expected_hash =
        parse_hash::<Obligation>(vec_val["expected"]["obligation_hash"].as_str().unwrap());

    let byte_len = canonical_bytes.len();

    // Limite exato -> sucesso
    let limits_exact = DecodeLimits {
        max_bytes: byte_len,
        ..DecodeLimits::default()
    };
    assert!(decode_for_check(&canonical_bytes, expected_hash, &target_vocab, limits_exact).is_ok());

    // Limite 1 byte a menos -> falha com Size
    let limits_exceeded = DecodeLimits {
        max_bytes: byte_len - 1,
        ..DecodeLimits::default()
    };
    let res = decode_for_check(
        &canonical_bytes,
        expected_hash,
        &target_vocab,
        limits_exceeded,
    );
    assert!(
        matches!(res, Err(DecodeError::Unknown(Exhausted::Size))),
        "esperava Unknown(Size), obteve: {res:?}"
    );

    // Limite 0 -> falha com Size
    let limits_zero = DecodeLimits {
        max_bytes: 0,
        ..DecodeLimits::default()
    };
    let res = decode_for_check(&canonical_bytes, expected_hash, &target_vocab, limits_zero);
    assert!(
        matches!(res, Err(DecodeError::Unknown(Exhausted::Size))),
        "esperava Unknown(Size), obteve: {res:?}"
    );
}

#[test]
fn test_decode_premises_limit() {
    let raw_json = include_str!("../../../spec/vectors/obligation/vectors.json");
    let root: JsonValue = serde_json::from_str(raw_json).expect("valid vectors JSON");

    // Vetor 6 possui 4 premissas
    let vec6 = root["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == "obligation-06-all-four-citable-kinds")
        .expect("vector 06");

    let target_vocab =
        from_canon::<Vocabulary>(&unhex(vec6["target_vocabulary_hex"].as_str().unwrap())).unwrap();
    let canonical_bytes = unhex(vec6["expected"]["canonical_hex"].as_str().unwrap());
    let expected_hash =
        parse_hash::<Obligation>(vec6["expected"]["obligation_hash"].as_str().unwrap());

    // Limite exato de 4 premissas -> sucesso
    let limits_exact = DecodeLimits {
        max_premises: 4,
        ..DecodeLimits::default()
    };
    assert!(decode_for_check(&canonical_bytes, expected_hash, &target_vocab, limits_exact).is_ok());

    // Limite de 3 premissas -> falha com Size
    let limits_tight = DecodeLimits {
        max_premises: 3,
        ..DecodeLimits::default()
    };
    let res = decode_for_check(&canonical_bytes, expected_hash, &target_vocab, limits_tight);
    assert!(
        matches!(res, Err(DecodeError::Unknown(Exhausted::Size))),
        "esperava Unknown(Size), obteve: {res:?}"
    );

    // Limite 0 premissas -> falha com Size
    let limits_zero = DecodeLimits {
        max_premises: 0,
        ..DecodeLimits::default()
    };
    let res = decode_for_check(&canonical_bytes, expected_hash, &target_vocab, limits_zero);
    assert!(
        matches!(res, Err(DecodeError::Unknown(Exhausted::Size))),
        "esperava Unknown(Size), obteve: {res:?}"
    );
}

#[test]
fn test_decode_formula_depth_limit() {
    let raw_json = include_str!("../../../spec/vectors/obligation/vectors.json");
    let root: JsonValue = serde_json::from_str(raw_json).expect("valid vectors JSON");
    let vec_val = &root["vectors"][0];

    let target_vocab =
        from_canon::<Vocabulary>(&unhex(vec_val["target_vocabulary_hex"].as_str().unwrap()))
            .unwrap();
    let canonical_bytes = unhex(vec_val["expected"]["canonical_hex"].as_str().unwrap());
    let expected_hash =
        parse_hash::<Obligation>(vec_val["expected"]["obligation_hash"].as_str().unwrap());

    let goal = from_canon::<Formula>(&unhex(vec_val["goal_hex"].as_str().unwrap())).unwrap();
    let goal_depth = goal.depth();

    // Profundidade exata -> sucesso
    let limits_exact = DecodeLimits {
        air: Limits {
            max_depth: goal_depth,
            max_nodes: 100_000,
        },
        ..DecodeLimits::default()
    };
    assert!(decode_for_check(&canonical_bytes, expected_hash, &target_vocab, limits_exact).is_ok());

    // Profundidade 1 a menos -> falha com Size
    let limits_exceeded = DecodeLimits {
        air: Limits {
            max_depth: goal_depth.saturating_sub(1),
            max_nodes: 100_000,
        },
        ..DecodeLimits::default()
    };
    let res = decode_for_check(
        &canonical_bytes,
        expected_hash,
        &target_vocab,
        limits_exceeded,
    );
    assert!(
        matches!(res, Err(DecodeError::Unknown(Exhausted::Size))),
        "esperava Unknown(Size), obteve: {res:?}"
    );
}

#[test]
fn test_decode_formula_nodes_aggregate_limit() {
    let raw_json = include_str!("../../../spec/vectors/obligation/vectors.json");
    let root: JsonValue = serde_json::from_str(raw_json).expect("valid vectors JSON");

    let vec6 = root["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == "obligation-06-all-four-citable-kinds")
        .expect("vector 06");

    let target_vocab =
        from_canon::<Vocabulary>(&unhex(vec6["target_vocabulary_hex"].as_str().unwrap())).unwrap();
    let canonical_bytes = unhex(vec6["expected"]["canonical_hex"].as_str().unwrap());
    let expected_hash =
        parse_hash::<Obligation>(vec6["expected"]["obligation_hash"].as_str().unwrap());

    let goal = from_canon::<Formula>(&unhex(vec6["goal_hex"].as_str().unwrap())).unwrap();
    let mut total_nodes = goal.node_count();

    for p in vec6["expected"]["canonical_premises"].as_array().unwrap() {
        let pf = from_canon::<Formula>(&unhex(p["formula_hex"].as_str().unwrap())).unwrap();
        total_nodes += pf.node_count();
    }

    // Orçamento agregado exato cobrindo todas as premissas e o goal -> sucesso
    let limits_exact = DecodeLimits {
        air: Limits {
            max_depth: 128,
            max_nodes: total_nodes,
        },
        ..DecodeLimits::default()
    };
    assert!(decode_for_check(&canonical_bytes, expected_hash, &target_vocab, limits_exact).is_ok());

    // Orçamento agregado com 1 nó a menos -> falha com Size mesmo que cada fórmula individual caiba
    let limits_tight = DecodeLimits {
        air: Limits {
            max_depth: 128,
            max_nodes: total_nodes - 1,
        },
        ..DecodeLimits::default()
    };
    let res = decode_for_check(&canonical_bytes, expected_hash, &target_vocab, limits_tight);
    assert!(
        matches!(res, Err(DecodeError::Unknown(Exhausted::Size))),
        "esperava Unknown(Size), obteve: {res:?}"
    );
}

#[test]
fn test_decode_invalid_premises_duplicate() {
    let raw_json = include_str!("../../../spec/vectors/obligation/vectors.json");
    let root: JsonValue = serde_json::from_str(raw_json).expect("valid vectors JSON");
    let vec6 = root["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == "obligation-06-all-four-citable-kinds")
        .unwrap();

    let target_vocab =
        from_canon::<Vocabulary>(&unhex(vec6["target_vocabulary_hex"].as_str().unwrap())).unwrap();
    let canonical_bytes = unhex(vec6["expected"]["canonical_hex"].as_str().unwrap());

    let ref_a = unhex("0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a");
    let ref_b = unhex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");

    // Substitui ref_b por ref_a criando duplicata
    let mut modified = canonical_bytes.clone();
    let pos_b = modified
        .windows(32)
        .position(|w| w == ref_b.as_slice())
        .expect("found ref_b");
    modified[pos_b..pos_b + 32].copy_from_slice(&ref_a);

    // Recalcula hash exato para passar pelo check inicial de hash
    let recomputed_hash = hash_raw::<Obligation>(&modified);

    let res = decode_for_check(
        &modified,
        recomputed_hash,
        &target_vocab,
        DecodeLimits::default(),
    );
    assert!(
        matches!(res, Err(DecodeError::InvalidPremises)),
        "esperava InvalidPremises para referências duplicadas, obteve: {res:?}"
    );
}

#[test]
fn test_decode_invalid_premises_unsorted() {
    let raw_json = include_str!("../../../spec/vectors/obligation/vectors.json");
    let root: JsonValue = serde_json::from_str(raw_json).expect("valid vectors JSON");
    let vec6 = root["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == "obligation-06-all-four-citable-kinds")
        .unwrap();

    let target_vocab =
        from_canon::<Vocabulary>(&unhex(vec6["target_vocabulary_hex"].as_str().unwrap())).unwrap();
    let canonical_bytes = unhex(vec6["expected"]["canonical_hex"].as_str().unwrap());

    let ref_a = unhex("0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a");
    let ref_b = unhex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");

    // Troca ref_a e ref_b de posição para tornar a ordem estritamente decrescente
    let mut modified = canonical_bytes.clone();
    let pos_a = modified
        .windows(32)
        .position(|w| w == ref_a.as_slice())
        .expect("found ref_a");
    let pos_b = modified
        .windows(32)
        .position(|w| w == ref_b.as_slice())
        .expect("found ref_b");

    modified[pos_a..pos_a + 32].copy_from_slice(&ref_b);
    modified[pos_b..pos_b + 32].copy_from_slice(&ref_a);

    let recomputed_hash = hash_raw::<Obligation>(&modified);

    let res = decode_for_check(
        &modified,
        recomputed_hash,
        &target_vocab,
        DecodeLimits::default(),
    );
    assert!(
        matches!(res, Err(DecodeError::InvalidPremises)),
        "esperava InvalidPremises para referências fora de ordem, obteve: {res:?}"
    );
}

#[test]
fn test_decode_unknown_sort_in_goal() {
    // Vocabulário simples com apenas sort "S"
    let s = Sort {
        name: Text::new("S").unwrap(),
    };
    let s_decl = SortDecl {
        name: s.name.clone(),
        domain: SortDomain::Open,
    };
    let vocab = Vocabulary::new(1, vec![s_decl], vec![]).unwrap();
    let vocab_h = hash(&vocab).unwrap();

    // Goal usando sort não declarado "GhostSort"
    let ghost_sort = Sort {
        name: Text::new("GhostSort").unwrap(),
    };
    let var_term = Term::new(
        vec![TermNode::Var {
            index: 0,
            sort: ghost_sort.clone(),
        }],
        Limits::default(),
    )
    .unwrap();
    let goal_form = Formula::new(
        vec![
            FormulaNode::Forall(ghost_sort),
            FormulaNode::Eq(var_term.clone(), var_term),
        ],
        Limits::default(),
    )
    .unwrap();

    let goal_val = goal_form.to_value().unwrap();
    let raw_premises = Value::Array(Array::new(vec![]).unwrap());

    let bytes = craft_envelope(
        1,
        0,
        "FOL-ms-eq/v1",
        1,
        *vocab_h.as_bytes(),
        raw_premises,
        goal_val,
    );
    let h = hash_raw::<Obligation>(&bytes);

    let res = decode_for_check(&bytes, h, &vocab, DecodeLimits::default());
    assert!(
        matches!(
            res,
            Err(DecodeError::Sort(SortError {
                kind: SortErrorKind::UnknownSort,
                ..
            }))
        ),
        "esperava Sort(UnknownSort), obteve: {res:?}"
    );
}

#[test]
fn test_decode_unbound_variable_in_goal() {
    // Vocabulário simples com sort "S"
    let s = Sort {
        name: Text::new("S").unwrap(),
    };
    let s_decl = SortDecl {
        name: s.name.clone(),
        domain: SortDomain::Open,
    };
    let vocab = Vocabulary::new(1, vec![s_decl], vec![]).unwrap();
    let vocab_h = hash(&vocab).unwrap();

    // Goal com variável livre (índice 0 sem quantificador Forall/Exists envolvente)
    let var_term = Term::new(
        vec![TermNode::Var {
            index: 0,
            sort: s.clone(),
        }],
        Limits::default(),
    )
    .unwrap();
    let goal_form = Formula::new(
        vec![FormulaNode::Eq(var_term.clone(), var_term)],
        Limits::default(),
    )
    .unwrap();

    let goal_val = goal_form.to_value().unwrap();
    let raw_premises = Value::Array(Array::new(vec![]).unwrap());

    let bytes = craft_envelope(
        1,
        0,
        "FOL-ms-eq/v1",
        1,
        *vocab_h.as_bytes(),
        raw_premises,
        goal_val,
    );
    let h = hash_raw::<Obligation>(&bytes);

    let res = decode_for_check(&bytes, h, &vocab, DecodeLimits::default());
    assert!(
        matches!(
            res,
            Err(DecodeError::Sort(SortError {
                kind: SortErrorKind::UnboundVariable { .. },
                ..
            }))
        ),
        "esperava Sort(UnboundVariable), obteve: {res:?}"
    );
}

#[test]
fn test_decode_trailing_bytes() {
    let raw_json = include_str!("../../../spec/vectors/obligation/vectors.json");
    let root: JsonValue = serde_json::from_str(raw_json).expect("valid vectors JSON");
    let vec_val = &root["vectors"][0];

    let target_vocab =
        from_canon::<Vocabulary>(&unhex(vec_val["target_vocabulary_hex"].as_str().unwrap()))
            .unwrap();
    let mut canonical_bytes = unhex(vec_val["expected"]["canonical_hex"].as_str().unwrap());
    canonical_bytes.push(0x00); // trailing byte

    let recomputed_hash = hash_raw::<Obligation>(&canonical_bytes);

    let res = decode_for_check(
        &canonical_bytes,
        recomputed_hash,
        &target_vocab,
        DecodeLimits::default(),
    );
    assert!(
        matches!(res, Err(DecodeError::Canon(CanonError::TrailingBytes))),
        "esperava Canon(TrailingBytes), obteve: {res:?}"
    );
}

#[test]
fn test_decode_truncated_bytes() {
    let raw_json = include_str!("../../../spec/vectors/obligation/vectors.json");
    let root: JsonValue = serde_json::from_str(raw_json).expect("valid vectors JSON");
    let vec_val = &root["vectors"][0];

    let target_vocab =
        from_canon::<Vocabulary>(&unhex(vec_val["target_vocabulary_hex"].as_str().unwrap()))
            .unwrap();
    let mut canonical_bytes = unhex(vec_val["expected"]["canonical_hex"].as_str().unwrap());
    canonical_bytes.pop(); // trunca 1 byte

    let recomputed_hash = hash_raw::<Obligation>(&canonical_bytes);

    let res = decode_for_check(
        &canonical_bytes,
        recomputed_hash,
        &target_vocab,
        DecodeLimits::default(),
    );
    assert!(
        matches!(res, Err(DecodeError::Canon(CanonError::Eof))),
        "esperava Canon(Eof), obteve: {res:?}"
    );
}

#[test]
fn test_decode_envelope_wrong_types() {
    let raw_json = include_str!("../../../spec/vectors/obligation/vectors.json");
    let root: JsonValue = serde_json::from_str(raw_json).expect("valid vectors JSON");
    let vec_val = &root["vectors"][0];

    let target_vocab =
        from_canon::<Vocabulary>(&unhex(vec_val["target_vocabulary_hex"].as_str().unwrap()))
            .unwrap();
    let canonical_bytes = unhex(vec_val["expected"]["canonical_hex"].as_str().unwrap());

    // 1. Root não é um array CBOR (ex.: inteiro unsigned)
    let non_array_bytes = vec![0x01];
    let h_non_array = hash_raw::<Obligation>(&non_array_bytes);
    let res = decode_for_check(
        &non_array_bytes,
        h_non_array,
        &target_vocab,
        DecodeLimits::default(),
    );
    assert!(matches!(
        res,
        Err(DecodeError::Canon(CanonError::WrongType))
    ));

    // 2. Array de 6 elementos (curto demais)
    let Value::Array(envelope) = from_canon::<Value>(&canonical_bytes).unwrap() else {
        panic!("fixture must be an array");
    };
    let short_bytes = to_canon(&Value::Array(
        Array::new(envelope.items()[..6].to_vec()).unwrap(),
    ))
    .unwrap();
    let h_short = hash_raw::<Obligation>(&short_bytes);
    let res = decode_for_check(
        &short_bytes,
        h_short,
        &target_vocab,
        DecodeLimits::default(),
    );
    assert!(matches!(
        res,
        Err(DecodeError::Canon(CanonError::WrongType))
    ));

    // 3. Versão 2 em vez de 1
    let mut v2_bytes = canonical_bytes.clone();
    v2_bytes[1] = 0x02; // Unsigned(2)
    let h_v2 = hash_raw::<Obligation>(&v2_bytes);
    let res = decode_for_check(&v2_bytes, h_v2, &target_vocab, DecodeLimits::default());
    assert!(matches!(
        res,
        Err(DecodeError::Canon(CanonError::WrongType))
    ));

    // 4. Tipo de objeto 1 em vez de 0
    let mut type_bytes = canonical_bytes.clone();
    type_bytes[2] = 0x01; // Unsigned(1)
    let h_type = hash_raw::<Obligation>(&type_bytes);
    let res = decode_for_check(&type_bytes, h_type, &target_vocab, DecodeLimits::default());
    assert!(matches!(
        res,
        Err(DecodeError::Canon(CanonError::WrongType))
    ));

    // 5. LogicId desconhecido ("FOL-ms-eq/v2")
    let mut vlogic_bytes = canonical_bytes.clone();
    let logic = b"FOL-ms-eq/v1";
    let at = vlogic_bytes
        .windows(logic.len())
        .position(|w| w == logic)
        .unwrap();
    vlogic_bytes[at + logic.len() - 1] = b'2';
    let h_vlogic = hash_raw::<Obligation>(&vlogic_bytes);
    let res = decode_for_check(
        &vlogic_bytes,
        h_vlogic,
        &target_vocab,
        DecodeLimits::default(),
    );
    assert!(matches!(
        res,
        Err(DecodeError::Air(AirError::UnsupportedLogic))
    ));
}

#[test]
fn test_decode_from_decoder_json_vectors() {
    let raw_json = include_str!("../../../spec/vectors/obligation/decoder.json");
    let root: JsonValue = serde_json::from_str(raw_json).expect("valid decoder JSON");
    let vectors = root["vectors"].as_array().expect("vectors array");

    for vec_val in vectors {
        let id = vec_val["id"].as_str().expect("id string");
        let description = vec_val["description"].as_str().expect("description");

        let target_v_hex = vec_val["target_vocabulary_hex"]
            .as_str()
            .expect("target_vocab_hex");
        let target_vocab_bytes = unhex(target_v_hex);
        let target_vocab =
            from_canon::<Vocabulary>(&target_vocab_bytes).expect("target vocabulary from_canon");

        let bytes_hex = vec_val["bytes_hex"].as_str().expect("bytes_hex");
        let bytes = unhex(bytes_hex);

        let expected_hash_hex = vec_val["expected_hash_hex"]
            .as_str()
            .expect("expected_hash_hex");
        let expected_hash = parse_hash::<Obligation>(expected_hash_hex);

        let expected_error = vec_val["expected_error"].as_str().expect("expected_error");

        let res = decode_for_check(
            &bytes,
            expected_hash,
            &target_vocab,
            DecodeLimits::default(),
        );

        match expected_error {
            "HashMismatch" => {
                assert!(
                    matches!(res, Err(DecodeError::HashMismatch)),
                    "{id} ({description}): esperava HashMismatch, obteve {res:?}"
                );
            }
            "VocabularyMismatch" => {
                assert!(
                    matches!(res, Err(DecodeError::VocabularyMismatch)),
                    "{id} ({description}): esperava VocabularyMismatch, obteve {res:?}"
                );
            }
            "Canon(TrailingBytes)" => {
                assert!(
                    matches!(res, Err(DecodeError::Canon(CanonError::TrailingBytes))),
                    "{id} ({description}): esperava Canon(TrailingBytes), obteve {res:?}"
                );
            }
            "Canon(Eof)" => {
                assert!(
                    matches!(res, Err(DecodeError::Canon(CanonError::Eof))),
                    "{id} ({description}): esperava Canon(Eof), obteve {res:?}"
                );
            }
            "Canon(WrongType)" => {
                assert!(
                    matches!(res, Err(DecodeError::Canon(CanonError::WrongType))),
                    "{id} ({description}): esperava Canon(WrongType), obteve {res:?}"
                );
            }
            "Air(UnsupportedLogic)" => {
                assert!(
                    matches!(res, Err(DecodeError::Air(AirError::UnsupportedLogic))),
                    "{id} ({description}): esperava Air(UnsupportedLogic), obteve {res:?}"
                );
            }
            "InvalidPremises" => {
                assert!(
                    matches!(res, Err(DecodeError::InvalidPremises)),
                    "{id} ({description}): esperava InvalidPremises, obteve {res:?}"
                );
            }
            other => panic!("{id}: tipo de erro esperado não mapeado no teste: {other}"),
        }
    }
}
