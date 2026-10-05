//! Regressões consumidor-visíveis do protocolo candidato e fronteira P1.

use logos_air::{Formula, FormulaNode, Limits, Sort, Vocabulary};
use logos_canon::{Text, to_canon};
use logos_crypto::{Hash, hash};
use logos_obligation::{
    EntryKind, LedgerEntry, LedgerError, LedgerView, PremiseRef, Proposal as Resolved, Reject,
    generate,
};
use logos_protocol::{
    ClaimId, MAX_GOAL_BYTES, MAX_MESSAGE_BYTES, MAX_REFERENCES, MAX_TEXT_BYTES, Proposal,
    ProtocolError, UntrustedText, decode, encode,
};
use serde_json::{Value, json};

fn formula(nodes: Vec<FormulaNode>, depth: usize) -> Formula {
    Formula::new(
        nodes,
        Limits {
            max_depth: depth,
            max_nodes: 100_001,
        },
    )
    .expect("fixture")
}

fn candidate() -> Proposal {
    Proposal {
        text_pt: UntrustedText::new("candidato".into()),
        goal: formula(vec![FormulaNode::True], 128),
        cites: vec![],
        speculative_on: vec![],
        vocab: Hash::from_bytes([0; 32]),
    }
}

fn document() -> Value {
    serde_json::from_slice(&encode(&candidate()).expect("fixture encode")).expect("fixture JSON")
}

fn read_value(value: &Value) -> Result<Proposal, ProtocolError> {
    decode(&serde_json::to_vec(value).expect("JSON input"))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn rejects_authority_and_unknown_fields_at_both_levels() {
    for field in [
        "obligation",
        "Obligation",
        "fact",
        "Fact",
        "policy",
        "Policy",
        "dependencies",
        "status",
        "signature",
        "Signature",
        "certificate",
        "token",
        "future_field",
    ] {
        for nested in [false, true] {
            let mut value = document();
            let object = if nested {
                &mut value["proposal"]
            } else {
                &mut value
            };
            object[field] = json!({"status":"Verified"});
            assert!(
                matches!(read_value(&value), Err(ProtocolError::Json(_))),
                "{field}"
            );
        }
    }
}

#[test]
fn rejects_duplicate_keys_even_when_they_use_json_escapes() {
    let valid = String::from_utf8(encode(&candidate()).expect("encode")).expect("UTF8");
    for (needle, replacement) in [
        ("\"version\":1", "\"version\":1,\"version\":1"),
        ("\"version\":1", "\"version\":1,\"vers\\u0069on\":1"),
        (
            "\"text_pt\":\"candidato\"",
            "\"text_pt\":\"candidato\",\"text_pt\":\"outro\"",
        ),
        ("\"cites\":[]", "\"cites\":[],\"cites\":[]"),
        ("\"goal\":", "\"goal\":\"00\",\"goal\":"),
        (
            "\"speculative_on\":[]",
            "\"speculative_on\":[],\"speculative_on\":[]",
        ),
        ("\"vocab\":", "\"vocab\":null,\"vocab\":"),
    ] {
        let input = valid.replace(needle, replacement);
        assert!(matches!(
            decode(input.as_bytes()),
            Err(ProtocolError::Json(_))
        ));
    }
    let mut input = format!(
        "{{\"version\":1,\"proposal\":{0},\"proposal\":{0}}}",
        document()["proposal"]
    );
    assert!(matches!(
        decode(input.as_bytes()),
        Err(ProtocolError::Json(_))
    ));
    input.push_str("{}");
    assert!(matches!(
        decode(input.as_bytes()),
        Err(ProtocolError::Json(_))
    ));
}

#[test]
fn fields_are_required_and_types_never_coerce() {
    for field in ["text_pt", "goal", "cites", "speculative_on", "vocab"] {
        let mut missing = document();
        missing["proposal"]
            .as_object_mut()
            .expect("object")
            .remove(field);
        assert!(matches!(read_value(&missing), Err(ProtocolError::Json(_))));
        let mut null = document();
        null["proposal"][field] = Value::Null;
        assert!(matches!(read_value(&null), Err(ProtocolError::Json(_))));
        let mut wrong = document();
        wrong["proposal"][field] = json!(false);
        assert!(matches!(read_value(&wrong), Err(ProtocolError::Json(_))));
    }
    for field in ["version", "proposal"] {
        let mut value = document();
        value.as_object_mut().expect("object").remove(field);
        assert!(matches!(read_value(&value), Err(ProtocolError::Json(_))));
    }
    for wrong in [json!([]), json!(null), json!("candidate")] {
        assert!(matches!(read_value(&wrong), Err(ProtocolError::Json(_))));
    }
}

#[test]
fn struct_sequences_are_not_json_objects() {
    let document = document();
    let payload = &document["proposal"];
    let sequence = json!([
        payload["text_pt"],
        payload["goal"],
        payload["cites"],
        payload["speculative_on"],
        payload["vocab"]
    ]);
    for value in [
        json!([1, payload]),
        json!({"version":1,"proposal":sequence}),
    ] {
        assert!(matches!(read_value(&value), Err(ProtocolError::Json(_))));
    }
}

#[test]
fn only_version_one_integer_is_supported() {
    for version in [0, 2, u32::MAX] {
        let mut value = document();
        value["version"] = json!(version);
        assert!(
            matches!(read_value(&value), Err(ProtocolError::UnsupportedVersion(v)) if v == version)
        );
    }
    for version in [
        json!(1.0),
        json!("1"),
        json!(-1),
        json!(4294967296_u64),
        Value::Null,
    ] {
        let mut value = document();
        value["version"] = version;
        assert!(matches!(read_value(&value), Err(ProtocolError::Json(_))));
    }
}

#[test]
fn digests_are_strict_and_unicode_input_cannot_panic() {
    for invalid in [
        "".to_owned(),
        "ab".repeat(31),
        "ab".repeat(33),
        "AB".repeat(32),
        "gg".repeat(32),
        "é".repeat(32),
        "０".repeat(64),
    ] {
        for field in ["cites", "speculative_on", "vocab"] {
            let mut value = document();
            value["proposal"][field] = if field == "vocab" {
                json!(invalid)
            } else {
                json!([invalid])
            };
            assert!(matches!(read_value(&value), Err(ProtocolError::Json(_))));
        }
    }
    for field in ["cites", "speculative_on"] {
        for invalid in [json!([1]), json!([null]), json!([{"hash":"00"}])] {
            let mut value = document();
            value["proposal"][field] = invalid;
            assert!(matches!(read_value(&value), Err(ProtocolError::Json(_))));
        }
    }
}

#[test]
fn goal_hex_and_canonical_air_are_not_interchangeable_with_arbitrary_payloads() {
    for invalid in [
        "".to_owned(),
        "0".into(),
        "é".repeat(32),
        "FF".into(),
        "zz".into(),
    ] {
        let mut value = document();
        value["proposal"]["goal"] = json!(invalid);
        assert!(matches!(read_value(&value), Err(ProtocolError::GoalHex)));
    }
    let goal = hex(&to_canon(&candidate().goal).expect("CBOR"));
    for invalid in [
        "00".to_owned(),
        format!("{goal}00"),
        goal.replacen("8401", "841801", 1),
        goal.replacen("840103", "840102", 1),
        goal.replacen("840103", "840203", 1),
    ] {
        let mut value = document();
        value["proposal"]["goal"] = json!(invalid);
        assert!(matches!(read_value(&value), Err(ProtocolError::Air(_))));
    }
    for wrong in [
        json!({"Obligation":{"goal":true}}),
        json!([1, 3, "FOL-ms-eq/v1", [[0]]]),
    ] {
        let mut value = document();
        value["proposal"]["goal"] = wrong;
        assert!(matches!(read_value(&value), Err(ProtocolError::Json(_))));
    }
}

#[test]
fn rejects_invalid_utf8_trailing_values_and_truncated_json() {
    let valid = encode(&candidate()).expect("encode");
    assert!(matches!(decode(&[0xff]), Err(ProtocolError::Json(_))));
    assert!(matches!(
        decode(&valid[..valid.len() - 1]),
        Err(ProtocolError::Json(_))
    ));
    let mut trailing = valid;
    trailing.extend_from_slice(b" null");
    assert!(matches!(decode(&trailing), Err(ProtocolError::Json(_))));
}

#[test]
fn message_limit_is_inclusive_before_parsing() {
    let expected = candidate();
    let mut wire = encode(&expected).expect("encode");
    wire.resize(MAX_MESSAGE_BYTES, b' ');
    assert_eq!(decode(&wire).expect("inclusive"), expected);
    wire.push(b' ');
    assert!(matches!(
        decode(&wire),
        Err(ProtocolError::Limit {
            field: "message",
            max: MAX_MESSAGE_BYTES
        })
    ));
}

#[test]
fn text_budget_counts_decoded_utf8_bytes_not_codepoints_or_escape_bytes() {
    let mut proposal = candidate();
    proposal.text_pt = UntrustedText::new("é".repeat(MAX_TEXT_BYTES / 2));
    assert_eq!(
        decode(&encode(&proposal).expect("inclusive")).expect("decode"),
        proposal
    );
    let mut value = document();
    value["proposal"]["text_pt"] = json!("é".repeat(MAX_TEXT_BYTES / 2 + 1));
    assert!(matches!(
        read_value(&value),
        Err(ProtocolError::Limit {
            field: "text_pt",
            max: MAX_TEXT_BYTES
        })
    ));
    proposal.text_pt = UntrustedText::new("\0".repeat(MAX_TEXT_BYTES));
    assert_eq!(
        decode(&encode(&proposal).expect("escapes")).expect("decoded bytes"),
        proposal
    );
    proposal.text_pt = UntrustedText::new("a".repeat(MAX_TEXT_BYTES + 1));
    assert!(matches!(
        encode(&proposal),
        Err(ProtocolError::Limit {
            field: "text_pt",
            max: MAX_TEXT_BYTES
        })
    ));
}

#[test]
fn reference_limits_preserve_order_duplicates_and_claim_identity() {
    let mut proposal = candidate();
    proposal.cites = (0..MAX_REFERENCES)
        .map(|n| PremiseRef::from_event_hash([n as u8; 32]))
        .collect();
    proposal.speculative_on = vec![ClaimId::from_bytes([0xab; 32]); MAX_REFERENCES];
    assert_eq!(
        decode(&encode(&proposal).expect("inclusive")).expect("decode"),
        proposal
    );
    for field in ["cites", "speculative_on"] {
        let mut value = document();
        value["proposal"][field] = json!(vec!["ab".repeat(32); MAX_REFERENCES + 1]);
        assert!(matches!(read_value(&value), Err(ProtocolError::Json(_))));
    }
    proposal.cites.push(PremiseRef::from_event_hash([0; 32]));
    assert!(matches!(
        encode(&proposal),
        Err(ProtocolError::Limit { field: "cites", .. })
    ));
    proposal.cites.pop();
    proposal.speculative_on.push(ClaimId::from_bytes([0; 32]));
    assert!(matches!(
        encode(&proposal),
        Err(ProtocolError::Limit {
            field: "speculative_on",
            ..
        })
    ));
}

fn quantified(name_len: usize) -> Formula {
    formula(
        vec![
            FormulaNode::Forall(Sort {
                name: Text::new("S".repeat(name_len)).expect("text"),
            }),
            FormulaNode::True,
        ],
        128,
    )
}

#[test]
fn goal_byte_budget_is_inclusive_and_symmetric() {
    let overhead = to_canon(&quantified(MAX_GOAL_BYTES))
        .expect("large CBOR")
        .len()
        - MAX_GOAL_BYTES;
    let mut proposal = candidate();
    proposal.goal = quantified(MAX_GOAL_BYTES - overhead);
    assert_eq!(
        to_canon(&proposal.goal).expect("CBOR").len(),
        MAX_GOAL_BYTES
    );
    assert_eq!(
        decode(&encode(&proposal).expect("inclusive")).expect("decode"),
        proposal
    );
    proposal.goal = quantified(MAX_GOAL_BYTES - overhead + 1);
    assert!(matches!(
        encode(&proposal),
        Err(ProtocolError::Limit {
            field: "goal",
            max: MAX_GOAL_BYTES
        })
    ));
    let mut value = document();
    value["proposal"]["goal"] = json!(hex(&to_canon(&proposal.goal).expect("too large CBOR")));
    assert!(matches!(
        read_value(&value),
        Err(ProtocolError::Limit {
            field: "goal",
            max: MAX_GOAL_BYTES
        })
    ));
}

#[test]
fn formula_limits_apply_to_decode_and_encode() {
    let mut proposal = candidate();
    let mut nodes = vec![FormulaNode::Not; 127];
    nodes.push(FormulaNode::True);
    proposal.goal = formula(nodes.clone(), 129);
    assert_eq!(
        decode(&encode(&proposal).expect("depth128")).expect("decode"),
        proposal
    );
    nodes.insert(0, FormulaNode::Not);
    proposal.goal = formula(nodes, 129);
    assert!(matches!(
        encode(&proposal),
        Err(ProtocolError::Limit {
            field: "goal_depth",
            max: 128
        })
    ));
    let mut value = document();
    value["proposal"]["goal"] = json!(hex(&to_canon(&proposal.goal).expect("CBOR")));
    assert!(matches!(read_value(&value), Err(ProtocolError::Air(_))));
    let mut nodes = vec![FormulaNode::And(99_999)];
    nodes.extend(vec![FormulaNode::True; 99_999]);
    proposal.goal = formula(nodes, 128);
    assert_eq!(
        decode(&encode(&proposal).expect("100000 nodes")).expect("decode"),
        proposal
    );
    let mut nodes = vec![FormulaNode::And(100_000)];
    nodes.extend(vec![FormulaNode::True; 100_000]);
    proposal.goal = formula(nodes, 128);
    assert!(matches!(
        encode(&proposal),
        Err(ProtocolError::Limit {
            field: "goal_nodes",
            max: 100_000
        })
    ));
    value["proposal"]["goal"] = json!(hex(&to_canon(&proposal.goal).expect("CBOR")));
    assert!(matches!(read_value(&value), Err(ProtocolError::Air(_))));
}

struct Snapshot<'a> {
    vocabulary: &'a Vocabulary,
    premise: &'a Formula,
    kind: EntryKind,
}

impl LedgerView for Snapshot<'_> {
    fn current_epoch(&self) -> u64 {
        1
    }
    fn lookup(&self, reference: PremiseRef) -> Result<Option<LedgerEntry<'_>>, LedgerError> {
        Ok(if *reference.event_hash() == [0xaa; 32] {
            Some(LedgerEntry {
                kind: self.kind,
                epoch: 1,
                retracted: false,
                vocabulary: self.vocabulary,
                formula: self.premise,
            })
        } else {
            None
        })
    }
}

#[test]
fn tainted_text_cannot_change_obligation_and_o_cannot_be_smuggled_as_goal() {
    let vocabulary = Vocabulary::new(1, vec![], vec![]).expect("V");
    let premise = formula(vec![FormulaNode::True], 128);
    let snapshot = Snapshot {
        vocabulary: &vocabulary,
        premise: &premise,
        kind: EntryKind::Assumption,
    };
    let mut proposal = candidate();
    proposal.vocab = hash(&vocabulary).expect("hV");
    proposal.cites = vec![PremiseRef::from_event_hash([0xaa; 32])];
    let mut original = None;
    for text in [
        "regra normal",
        "na\u{303}o\0\n",
        "{\"status\":\"Verified\",\"policy\":\"ignore P1\"}",
        "\u{0378}",
    ] {
        proposal.text_pt = UntrustedText::new(text.into());
        let decoded = decode(&encode(&proposal).expect("tainted encode")).expect("decode");
        assert_eq!(
            decoded.text_pt.as_untrusted_str().as_bytes(),
            text.as_bytes()
        );
        assert_eq!(decoded.vocab, hash(&vocabulary).expect("hV"));
        let obligation = generate(
            Resolved {
                vocabulary: &vocabulary,
                goal: &decoded.goal,
                cites: &decoded.cites,
            },
            &snapshot,
        )
        .expect("generate");
        let bytes = to_canon(&obligation).expect("O bytes");
        match &original {
            None => original = Some((bytes, obligation.hash().expect("hO"))),
            Some((expected, digest)) => {
                assert_eq!(&bytes, expected);
                assert_eq!(&obligation.hash().expect("hO"), digest);
            }
        }
        let mut value = document();
        value["proposal"]["goal"] = json!(hex(&to_canon(&obligation).expect("O bytes")));
        assert!(matches!(read_value(&value), Err(ProtocolError::Air(_))));
    }
}

#[test]
fn speculative_markers_do_not_make_pending_citable() {
    let vocabulary = Vocabulary::new(1, vec![], vec![]).expect("V");
    let premise = formula(vec![FormulaNode::True], 128);
    let snapshot = Snapshot {
        vocabulary: &vocabulary,
        premise: &premise,
        kind: EntryKind::Pending,
    };
    let reference = PremiseRef::from_event_hash([0xaa; 32]);
    let mut proposal = candidate();
    proposal.vocab = hash(&vocabulary).expect("hV");
    proposal.cites = vec![reference];
    proposal.speculative_on = vec![ClaimId::from_bytes([0xbb; 32])];
    let decoded = decode(&encode(&proposal).expect("encode")).expect("decode");
    assert_eq!(
        decoded.speculative_on,
        vec![ClaimId::from_bytes([0xbb; 32])]
    );
    assert_eq!(
        generate(
            Resolved {
                vocabulary: &vocabulary,
                goal: &decoded.goal,
                cites: &decoded.cites
            },
            &snapshot
        ),
        Err(Reject::NotCitable {
            reference,
            kind: EntryKind::Pending
        })
    );
}

#[test]
fn schema_has_one_closed_candidate_without_authority_types() {
    let schema: Value =
        serde_json::from_str(include_str!("../../../spec/protocol.schema.json")).expect("schema");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["proposal"]["$ref"], "#/$defs/Proposal");
    let root = schema["properties"].as_object().expect("root properties");
    assert_eq!(
        root.keys().map(String::as_str).collect::<Vec<_>>(),
        ["proposal", "version"]
    );
    let proposal = &schema["$defs"]["Proposal"];
    assert_eq!(proposal["additionalProperties"], false);
    assert_eq!(
        proposal["properties"]
            .as_object()
            .expect("payload fields")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["cites", "goal", "speculative_on", "text_pt", "vocab"]
    );
    fn inspect(value: &Value) {
        match value {
            Value::Object(object) => {
                for (key, child) in object {
                    assert!(
                        ![
                            "obligation",
                            "fact",
                            "policy",
                            "signature",
                            "status",
                            "dependencies",
                            "certificate",
                            "token"
                        ]
                        .contains(&key.to_ascii_lowercase().as_str()),
                        "forbidden schema type/field {key}"
                    );
                    if key == "$ref" {
                        let target = child
                            .as_str()
                            .expect("ref")
                            .rsplit('/')
                            .next()
                            .expect("target");
                        assert!(
                            !["obligation", "fact", "policy", "signature"]
                                .contains(&target.to_ascii_lowercase().as_str())
                        );
                    }
                    inspect(child);
                }
            }
            Value::Array(items) => items.iter().for_each(inspect),
            _ => {}
        }
    }
    inspect(&schema);
}

#[test]
fn six_independent_vectors_preserve_wire_and_decoded_meaning() {
    let fixtures: Value =
        serde_json::from_str(include_str!("../../../spec/vectors/protocol/vectors.json"))
            .expect("vectors");
    for fixture in fixtures["vectors"].as_array().expect("cases") {
        let wire = fixture["wire"].as_str().expect("wire");
        let decoded = decode(wire.as_bytes()).expect("vector decode");
        let expected = &fixture["expected"];
        assert_eq!(
            decoded.text_pt.as_untrusted_str(),
            expected["text_pt"].as_str().expect("text")
        );
        assert_eq!(
            hex(&to_canon(&decoded.goal).expect("goal CBOR")),
            expected["goal"].as_str().expect("goal")
        );
        assert_eq!(
            decoded
                .cites
                .iter()
                .map(|r| hex(r.event_hash()))
                .collect::<Vec<_>>(),
            expected["cites"]
                .as_array()
                .expect("cites")
                .iter()
                .map(|v| v.as_str().expect("hex").to_owned())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            decoded
                .speculative_on
                .iter()
                .map(|r| hex(r.as_bytes()))
                .collect::<Vec<_>>(),
            expected["speculative_on"]
                .as_array()
                .expect("spec")
                .iter()
                .map(|v| v.as_str().expect("hex").to_owned())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            hex(decoded.vocab.as_bytes()),
            expected["vocab"].as_str().expect("vocab")
        );
        assert_eq!(encode(&decoded).expect("vector encode"), wire.as_bytes());
    }
}
