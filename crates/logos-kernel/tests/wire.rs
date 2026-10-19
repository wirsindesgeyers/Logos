//! Vetores independentes e fronteiras estritas de kernel-protocol/v1.
use logos_canon::{Array, Canon, Value, from_canon, to_canon};
use logos_kernel::wire::{Request, decode_frame, encode_frame};

fn unhex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
fn arr(v: Vec<Value>) -> Value {
    Value::Array(Array::new(v).unwrap())
}

#[test]
fn independent_vectors_cover_every_message_and_preserve_exact_bytes() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../spec/vectors/kernel-protocol/vectors.json"
    ))
    .unwrap();
    let mut valid = 0;
    let mut invalid = 0;
    for v in vectors["vectors"].as_array().unwrap() {
        let bytes = unhex(v["wire_hex"].as_str().unwrap());
        let frame = unhex(v["frame_hex"].as_str().unwrap());
        if v["valid"].as_bool().unwrap() {
            let request = Request::decode(&bytes).unwrap();
            assert_eq!(request.encode().unwrap(), bytes, "{}", v["id"]);
            assert_eq!(decode_frame(&frame).unwrap(), request);
            assert_eq!(encode_frame(&request).unwrap(), frame);
            valid += 1;
        } else {
            assert!(Request::decode(&bytes).is_err(), "{}", v["id"]);
            assert!(decode_frame(&frame).is_err());
            invalid += 1;
        }
    }
    assert_eq!(valid, 14);
    assert!(invalid >= 19);
}

#[test]
fn no_extra_field_or_canonical_alias_is_ignored() {
    // Query(head), independente do encoder de Request.
    let value = arr(vec![
        1u64.to_value().unwrap(),
        7u64.to_value().unwrap(),
        5u64.to_value().unwrap(),
        arr(vec![0u64.to_value().unwrap()]),
    ]);
    let wire = to_canon(&value).unwrap();
    assert!(Request::decode(&wire).is_ok());
    let Value::Array(a) = &value else { panic!() };
    for extra in [Value::Null, Value::Bool(true), Value::Unsigned(0)] {
        let mut fields = a.items().to_vec();
        fields.push(extra);
        assert!(Request::decode(&to_canon(&arr(fields)).unwrap()).is_err());
    }
    for end in 0..wire.len() {
        assert!(Request::decode(&wire[..end]).is_err());
    }
    let mut extra = wire.clone();
    extra.push(0);
    assert!(Request::decode(&extra).is_err());
    let mut nonminimal = vec![0x98, 4];
    nonminimal.extend_from_slice(&wire[1..]);
    assert!(Request::decode(&nonminimal).is_err());
    assert!(Request::decode(&[0x81; 129]).is_err());
    for length in [
        0u32,
        u32::MAX,
        (logos_kernel::wire::MAX_REQUEST_BYTES + 1) as u32,
    ] {
        assert!(decode_frame(&length.to_be_bytes()).is_err());
    }
    let frame = encode_frame(&Request::decode(&wire).unwrap()).unwrap();
    for end in 0..frame.len() {
        assert!(decode_frame(&frame[..end]).is_err());
    }
    let mut combined = frame.clone();
    combined.extend_from_slice(&frame);
    assert!(decode_frame(&combined).is_err());
}

#[test]
fn proposal_json_stays_outside_the_kernel_and_roundtrips_the_same_candidate() {
    let json: serde_json::Value =
        serde_json::from_str(include_str!("../../../spec/vectors/protocol/vectors.json")).unwrap();
    let wire: serde_json::Value = serde_json::from_str(include_str!(
        "../../../spec/vectors/kernel-protocol/vectors.json"
    ))
    .unwrap();
    for (old, new) in json["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .zip(wire["vectors"].as_array().unwrap())
    {
        let candidate = logos_protocol::decode(old["wire"].as_str().unwrap().as_bytes()).unwrap();
        let request = Request::decode(&unhex(new["wire_hex"].as_str().unwrap())).unwrap();
        let logos_kernel::wire::Message::Propose(p) = request.message else {
            panic!()
        };
        assert_eq!(
            p.text_pt.as_bytes(),
            candidate.text_pt.as_untrusted_str().as_bytes()
        );
        assert_eq!(p.goal, candidate.goal);
        assert_eq!(
            p.cites.iter().map(|h| *h.as_bytes()).collect::<Vec<_>>(),
            candidate
                .cites
                .iter()
                .map(|r| *r.event_hash())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            p.speculative_on,
            candidate
                .speculative_on
                .iter()
                .map(|r| *r.as_bytes())
                .collect::<Vec<_>>()
        );
        assert_eq!(p.vocabulary, candidate.vocab);
    }
}

#[test]
fn boundary_lengths_are_inclusive_without_defaults_or_truncation() {
    let mut value: Value = from_canon(&unhex("840100068200190100")).unwrap(); // Subscribe(0,256)
    assert!(Request::decode(&to_canon(&value).unwrap()).is_ok());
    for n in [0, 257, u64::MAX] {
        value = arr(vec![
            Value::Unsigned(1),
            Value::Unsigned(0),
            Value::Unsigned(6),
            arr(vec![Value::Unsigned(0), Value::Unsigned(n)]),
        ]);
        assert!(Request::decode(&to_canon(&value).unwrap()).is_err());
    }
    let bytes = vec![0; logos_kernel::wire::MAX_REQUEST_BYTES + 1];
    assert!(Request::decode(&bytes).is_err());
    assert!(!logos_kernel::wire::valid_staging_name(&"A".repeat(64)));
    assert!(!logos_kernel::wire::valid_staging_name(&"a".repeat(63)));
    assert!(logos_kernel::wire::valid_staging_name(&"a".repeat(64)));
}

#[test]
fn proposal_text_air_references_format_and_scope_enforce_real_byte_limits() {
    use logos_air::{Formula, FormulaNode, Limits, Symbol, SymbolKind, SymbolResult};
    use logos_canon::Text;
    use logos_crypto::Hash;
    use logos_kernel::wire::{Message, ObjectRef};
    use logos_ledger::{EventProposal, TaintedText};
    let pred = |name: String| {
        Formula::new(
            vec![FormulaNode::Pred {
                symbol: Symbol {
                    name: Text::new(name).unwrap(),
                    kind: SymbolKind::Pred,
                    arity: vec![],
                    result: SymbolResult::Bool,
                },
                args: vec![],
            }],
            Limits::default(),
        )
        .unwrap()
    };
    let base = to_canon(&pred(String::new())).unwrap().len();
    // O cabeçalho da string passa de um para cinco bytes neste tamanho.
    let goal = pred("p".repeat(logos_kernel::wire::MAX_AIR_BYTES - base - 4));
    assert_eq!(
        to_canon(&goal).unwrap().len(),
        logos_kernel::wire::MAX_AIR_BYTES
    );
    let p = EventProposal {
        text_pt: TaintedText::new("x".repeat(65536)).unwrap(),
        goal,
        cites: vec![Hash::from_bytes([1; 32]); 1024],
        speculative_on: vec![[2; 32]; 1024],
        vocabulary: Hash::from_bytes([3; 32]),
    };
    let bytes = Request {
        id: 1,
        message: Message::Propose(p),
    }
    .encode()
    .unwrap();
    assert!(Request::decode(&bytes).is_ok());
    let root: Value = from_canon(&bytes).unwrap();
    let Value::Array(root) = root else { panic!() };
    let outer = root.items();
    let Value::Array(fields) = &outer[3] else {
        panic!()
    };
    for (index, value) in [
        (0, Value::Bytes(vec![b'x'; 65537])),
        (
            1,
            Value::Bytes(
                to_canon(&pred(
                    "p".repeat(logos_kernel::wire::MAX_AIR_BYTES - base - 3),
                ))
                .unwrap(),
            ),
        ),
        (2, arr(vec![Value::Bytes(vec![1; 32]); 1025])),
        (3, arr(vec![Value::Bytes(vec![2; 32]); 1025])),
    ] {
        let mut fields = fields.items().to_vec();
        fields[index] = value;
        let mut outer = outer.to_vec();
        outer[3] = arr(fields);
        assert!(Request::decode(&to_canon(&arr(outer)).unwrap()).is_err());
    }
    for length in [256, 257] {
        let request = Request {
            id: 2,
            message: Message::Certify {
                obligation: Hash::from_bytes([1; 32]),
                format: Text::new("f".repeat(length)).unwrap(),
                certificate: ObjectRef::Cas(Hash::from_bytes([2; 32])),
            },
        };
        assert_eq!(request.encode().is_ok(), length == 256);
    }
    let truth = Formula::new(vec![FormulaNode::True], Limits::default()).unwrap();
    for length in [65536, 65537] {
        let request = Request {
            id: 3,
            message: Message::Assume {
                vocabulary: Hash::from_bytes([1; 32]),
                formula: truth.clone(),
                scope: Text::new("s".repeat(length)).unwrap(),
            },
        };
        assert_eq!(request.encode().is_ok(), length == 65536);
    }
}
