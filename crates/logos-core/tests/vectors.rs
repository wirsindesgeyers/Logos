//! Conformidade semântica e de bytes com vetores gerados independentemente em Python.

use std::fmt::Debug;

use logos_canon::{Canon, CanonDecode, from_canon, to_canon};
use logos_core::{Budget, CheckOutcome, Exhausted, Reason, Verdict};
use serde_json::{Value, json};

fn unhex(hex: &str) -> Vec<u8> {
    assert!(hex.len().is_multiple_of(2));
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("ASCII hex"), 16)
                .expect("hex válido")
        })
        .collect()
}

fn check<T: Canon + CanonDecode + Debug>(
    name: &str,
    bytes: &[u8],
    expected: &str,
    semantic: impl FnOnce(T) -> Value,
) {
    match expected.split_once(':').expect("prefixo de expectativa") {
        ("ok", text) => {
            let value = from_canon::<T>(bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!(to_canon(&value).expect("encode"), bytes, "{name}: bytes");
            assert_eq!(
                semantic(value),
                serde_json::from_str::<Value>(text).expect("JSON"),
                "{name}: valor"
            );
        }
        ("err", error) => {
            assert_eq!(
                from_canon::<T>(bytes).expect_err(name).name(),
                error,
                "{name}"
            );
        }
        other => panic!("expectativa inválida {other:?}"),
    }
}

fn exhausted_value(value: Exhausted) -> Value {
    json!(match value {
        Exhausted::Time => "Time",
        Exhausted::Steps => "Steps",
        Exhausted::Memory => "Memory",
        Exhausted::Size => "Size",
    })
}

#[test]
fn core_normative_vectors() {
    let vectors = include_str!("../../../spec/vectors/core/vectors.tsv");
    for line in vectors
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let columns: Vec<_> = line.split('\t').collect();
        assert_eq!(columns.len(), 4, "{line}");
        let [name, target, hex, expected] = columns.as_slice() else {
            unreachable!()
        };
        let bytes = unhex(hex);
        match *target {
            "Budget" => check::<Budget>(name, &bytes, expected, |b| {
                json!({
                    "steps": b.steps, "mem_bytes": b.mem_bytes,
                    "size_bytes": b.size_bytes, "wall_ms": b.wall_ms,
                })
            }),
            "Exhausted" => check::<Exhausted>(name, &bytes, expected, exhausted_value),
            "Reason" => check::<Reason>(name, &bytes, expected, |r| json!({"text": r.as_str()})),
            "CheckOutcome" => {
                check::<CheckOutcome>(name, &bytes, expected, |outcome| match outcome {
                    CheckOutcome::Accepted => json!("Accepted"),
                    CheckOutcome::Rejected(r) => json!({"Rejected": {"text": r.as_str()}}),
                    CheckOutcome::Unknown(e) => json!({"Unknown": exhausted_value(e)}),
                })
            }
            "Verdict" => check::<Verdict>(name, &bytes, expected, |v| {
                json!(match v {
                    Verdict::Verified => "Verified",
                    Verdict::Refuted => "Refuted",
                    Verdict::Unknown => "Unknown",
                })
            }),
            other => panic!("tipo desconhecido {other}"),
        }
    }
}
