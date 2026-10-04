//! Independent air/v1 fixtures: semantic fields, canonical bytes, hashes and errors.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fmt::Debug;

use logos_air::{
    AirError, Formula, FormulaNode, Limits, Sort, Symbol, SymbolKind, SymbolResult, Term, TermNode,
};
use logos_canon::{Canon, CanonDecode, CanonError, from_canon, to_canon};
use logos_crypto::hash;

fn unhex(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("ASCII hex"), 16)
                .expect("valid hex")
        })
        .collect()
}

fn sort_text(sort: &Sort) -> String {
    format!("S({})", sort.name.as_str())
}

fn symbol_text(symbol: &Symbol) -> String {
    let kind = match symbol.kind {
        SymbolKind::Const => "Const",
        SymbolKind::Func => "Func",
        SymbolKind::Pred => "Pred",
    };
    let args = symbol
        .arity
        .iter()
        .map(sort_text)
        .collect::<Vec<_>>()
        .join(",");
    let result = match &symbol.result {
        SymbolResult::Sort(sort) => sort_text(sort),
        SymbolResult::Bool => "Bool".to_owned(),
    };
    format!("Y({};{kind};[{args}];{result})", symbol.name.as_str())
}

fn term_text(term: &Term) -> String {
    let nodes = term
        .nodes()
        .iter()
        .map(|node| match node {
            TermNode::Var { index, sort } => format!("V({index};{})", sort_text(sort)),
            TermNode::App { symbol, args } => format!("A({};{args})", symbol_text(symbol)),
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("T({};[{nodes}])", term.logic_id().as_str())
}

fn formula_text(formula: &Formula) -> String {
    let nodes = formula
        .nodes()
        .iter()
        .map(|node| match node {
            FormulaNode::True => "True".to_owned(),
            FormulaNode::False => "False".to_owned(),
            FormulaNode::Eq(a, b) => format!("Eq({};{})", term_text(a), term_text(b)),
            FormulaNode::Pred { symbol, args } => format!(
                "Pred({};[{}])",
                symbol_text(symbol),
                args.iter().map(term_text).collect::<Vec<_>>().join(",")
            ),
            FormulaNode::Not => "Not".to_owned(),
            FormulaNode::And(count) => format!("And({count})"),
            FormulaNode::Or(count) => format!("Or({count})"),
            FormulaNode::Implies => "Implies".to_owned(),
            FormulaNode::Iff => "Iff".to_owned(),
            FormulaNode::Forall(sort) => format!("Forall({})", sort_text(sort)),
            FormulaNode::Exists(sort) => format!("Exists({})", sort_text(sort)),
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("F({};[{nodes}])", formula.logic_id().as_str())
}

fn air_error_text(error: &AirError) -> String {
    match error {
        AirError::Canon(error) => format!("Canon:{}", error.name()),
        AirError::UnsupportedLogic => "UnsupportedLogic".to_owned(),
        AirError::InvalidStructure => "InvalidStructure".to_owned(),
        AirError::DepthExceeded { limit, actual } => format!("DepthExceeded:{limit}:{actual}"),
        AirError::NodesExceeded { limit, actual } => format!("NodesExceeded:{limit}:{actual}"),
    }
}

fn check_metadata<T: Canon + CanonDecode + Debug>(
    name: &str,
    bytes: &[u8],
    outcome: &str,
    describe: impl FnOnce(&T) -> String,
) {
    if let Some(expected) = outcome.strip_prefix("ok:") {
        let value = from_canon::<T>(bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(describe(&value), expected, "{name}: semantic fields");
        assert_eq!(to_canon(&value).unwrap(), bytes, "{name}: canonical bytes");
    } else {
        let expected = outcome
            .strip_prefix("err:Canon:")
            .expect("metadata Canon error");
        assert_eq!(
            from_canon::<T>(bytes).expect_err(name).name(),
            expected,
            "{name}"
        );
    }
}

fn check_tree<T: Canon + CanonDecode + Debug + PartialEq>(
    name: &str,
    bytes: &[u8],
    outcome: &str,
    decode: impl FnOnce(&[u8]) -> Result<T, AirError>,
    describe: impl FnOnce(&T) -> String,
    accepted: impl FnOnce(&T),
) {
    if let Some(expected) = outcome.strip_prefix("ok:") {
        let value = decode(bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(describe(&value), expected, "{name}: semantic fields");
        assert_eq!(to_canon(&value).unwrap(), bytes, "{name}: canonical bytes");
        assert_eq!(
            from_canon::<T>(bytes).unwrap(),
            value,
            "{name}: default decoder"
        );
        accepted(&value);
    } else {
        let expected = outcome.strip_prefix("err:").expect("error outcome");
        let error = decode(bytes).expect_err(name);
        assert_eq!(air_error_text(&error), expected, "{name}: detailed error");
        let canon_error = match error {
            AirError::Canon(error) => error,
            _ => CanonError::WrongType,
        };
        assert_eq!(
            from_canon::<T>(bytes),
            Err(canon_error),
            "{name}: no default recovery"
        );
    }
}

#[test]
fn independent_air_vectors_cover_syntax_bytes_hashes_and_rejection() {
    let vectors = include_str!("../../../spec/vectors/air/vectors.tsv");
    for line in vectors
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let columns = line.split('\t').collect::<Vec<_>>();
        let [name, target, hex, digest, outcome] = columns.as_slice() else {
            panic!("five TSV columns required: {line}");
        };
        let bytes = unhex(hex);
        match *target {
            "Sort" => {
                assert_eq!(*digest, "-");
                check_metadata::<Sort>(name, &bytes, outcome, sort_text);
            }
            "Symbol" => {
                assert_eq!(*digest, "-");
                check_metadata::<Symbol>(name, &bytes, outcome, symbol_text);
            }
            "Term" => {
                assert_eq!(*digest, "-");
                check_tree::<Term>(
                    name,
                    &bytes,
                    outcome,
                    |bytes| Term::decode_with_limits(bytes, Limits::default()),
                    term_text,
                    |term| {
                        assert_eq!(
                            Term::from_value_with_limits(
                                &term.to_value().unwrap(),
                                Limits::default()
                            ),
                            Ok(term.clone())
                        )
                    },
                );
            }
            "Formula" => {
                if outcome.starts_with("err:") {
                    assert_eq!(*digest, "-");
                }
                check_tree::<Formula>(
                    name,
                    &bytes,
                    outcome,
                    |bytes| Formula::decode_with_limits(bytes, Limits::default()),
                    formula_text,
                    |formula| {
                        assert_eq!(
                            hash(formula).unwrap().to_hex(),
                            *digest,
                            "{name}: independent SHA-256 domain hash"
                        );
                        assert_eq!(
                            Formula::from_value_with_limits(
                                &formula.to_value().unwrap(),
                                Limits::default()
                            ),
                            Ok(formula.clone())
                        );
                    },
                );
            }
            other => panic!("unknown fixture type {other}"),
        }
    }
}
