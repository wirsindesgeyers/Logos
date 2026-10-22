//! Independent vocabulary/v1 fixtures: semantic fields, canonical bytes, hashes and errors.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fmt::Write;

use logos_air::{
    AirError, DomainSource, Formula, FormulaNode, Grounding, Limits, Sort, SortDecl, SortDomain,
    Symbol, SymbolDecl, SymbolKind, SymbolResult, Term, TermNode, Vocabulary, VocabularyError,
};
use logos_canon::{CanonError, from_canon, to_canon};
use logos_crypto::hash;

fn unhex(hex: &str) -> Vec<u8> {
    assert!(
        hex.len().is_multiple_of(2),
        "hex string length must be even, got {}",
        hex.len()
    );
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .unwrap_or_else(|e| panic!("invalid hex byte at {i}: {e}"))
        })
        .collect()
}

fn describe_sort(sort: &Sort) -> String {
    format!("S({})", sort.name.as_str())
}

fn describe_symbol(symbol: &Symbol) -> String {
    let kind = match symbol.kind {
        SymbolKind::Const => "Const",
        SymbolKind::Func => "Func",
        SymbolKind::Pred => "Pred",
    };
    let arity = symbol
        .arity
        .iter()
        .map(describe_sort)
        .collect::<Vec<_>>()
        .join(",");
    let result = match &symbol.result {
        SymbolResult::Sort(s) => describe_sort(s),
        SymbolResult::Bool => "Bool".to_owned(),
    };
    format!("Y({};{kind};[{arity}];{result})", symbol.name.as_str())
}

fn describe_term(term: &Term) -> String {
    let nodes = term
        .nodes()
        .iter()
        .map(|node| match node {
            TermNode::Var { index, sort } => format!("V({index};{})", describe_sort(sort)),
            TermNode::App { symbol, args } => format!("A({};{args})", describe_symbol(symbol)),
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("T({};[{nodes}])", term.logic_id().as_str())
}

fn describe_formula(formula: &Formula) -> String {
    let nodes = formula
        .nodes()
        .iter()
        .map(|node| match node {
            FormulaNode::True => "True".to_owned(),
            FormulaNode::False => "False".to_owned(),
            FormulaNode::Eq(l, r) => format!("Eq({};{})", describe_term(l), describe_term(r)),
            FormulaNode::Pred { symbol, args } => format!(
                "Pred({};[{}])",
                describe_symbol(symbol),
                args.iter().map(describe_term).collect::<Vec<_>>().join(",")
            ),
            FormulaNode::Not => "Not".to_owned(),
            FormulaNode::And(c) => format!("And({c})"),
            FormulaNode::Or(c) => format!("Or({c})"),
            FormulaNode::Implies => "Implies".to_owned(),
            FormulaNode::Iff => "Iff".to_owned(),
            FormulaNode::Forall(s) => format!("Forall({})", describe_sort(s)),
            FormulaNode::Exists(s) => format!("Exists({})", describe_sort(s)),
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("F({};[{nodes}])", formula.logic_id().as_str())
}

fn hex32(bytes: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}

fn describe_sort_decl(decl: &SortDecl) -> String {
    match &decl.domain {
        SortDomain::Open => format!("S({};Open)", decl.name.as_str()),
        SortDomain::Enumerated {
            constants,
            domain_source,
        } => {
            let consts = constants
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join(",");
            let src = match domain_source {
                DomainSource::Fact(f) => format!("Fact({})", hex32(&f.0)),
                DomainSource::Policy(p) => format!("Policy({})", hex32(&p.0)),
            };
            format!("S({};Enum:[{consts}];{src})", decl.name.as_str())
        }
    }
}

fn describe_grounding(grounding: &Grounding) -> String {
    match grounding {
        Grounding::Defined(f) => format!("Defined({})", describe_formula(f)),
        Grounding::Anchored {
            attestor,
            procedure,
        } => format!("Anchored({};{})", attestor.0.as_str(), procedure.as_str()),
        Grounding::Glossed(text) => format!("Glossed({})", text.as_str()),
    }
}

fn describe_symbol_decl(decl: &SymbolDecl) -> String {
    format!(
        "Y({};{};{})",
        describe_symbol(&decl.signature),
        describe_grounding(&decl.grounding),
        decl.gloss_pt.as_str()
    )
}

fn describe_vocabulary(vocab: &Vocabulary) -> String {
    let sorts = vocab
        .sorts()
        .iter()
        .map(describe_sort_decl)
        .collect::<Vec<_>>()
        .join(",");
    let symbols = vocab
        .symbols()
        .iter()
        .map(describe_symbol_decl)
        .collect::<Vec<_>>()
        .join(",");
    let order = vocab
        .definition_order()
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("V({};[{sorts}];[{symbols}];[{order}])", vocab.version())
}

fn vocabulary_error_text(error: &VocabularyError) -> String {
    match error {
        VocabularyError::Canon(err) => format!("Canon:{}", err.name()),
        VocabularyError::Air(err) => match err {
            AirError::Canon(c) => format!("Air:Canon:{}", c.name()),
            AirError::UnsupportedLogic => "Air:UnsupportedLogic".to_owned(),
            AirError::InvalidStructure => "Air:InvalidStructure".to_owned(),
            AirError::DepthExceeded { limit, actual } => {
                format!("Air:DepthExceeded:{limit}:{actual}")
            }
            AirError::NodesExceeded { limit, actual } => {
                format!("Air:NodesExceeded:{limit}:{actual}")
            }
        },
        VocabularyError::DuplicateSort => "DuplicateSort".to_owned(),
        VocabularyError::DuplicateSymbol => "DuplicateSymbol".to_owned(),
        VocabularyError::UnknownSort => "UnknownSort".to_owned(),
        VocabularyError::UnknownSymbol => "UnknownSymbol".to_owned(),
        VocabularyError::SignatureMismatch => "SignatureMismatch".to_owned(),
        VocabularyError::InvalidDomainConstant => "InvalidDomainConstant".to_owned(),
        VocabularyError::CircularDefinition => "CircularDefinition".to_owned(),
        VocabularyError::InvalidRenderTemplate => "InvalidRenderTemplate".to_owned(),
        VocabularyError::VersionOverflow => "VersionOverflow".to_owned(),
    }
}

#[test]
fn independent_vocabulary_vectors_cover_groundings_bytes_hashes_and_rejection() {
    let vectors = include_str!("../../../spec/vectors/vocabulary/vectors.tsv");
    for line in vectors
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let columns = line.split('\t').collect::<Vec<_>>();
        let [name, target, hex, digest, outcome] = columns.as_slice() else {
            panic!("five TSV columns required: {line}");
        };
        assert_eq!(
            *target, "Vocabulary",
            "{name}: fixture target must be Vocabulary"
        );
        let bytes = unhex(hex);

        if let Some(expected) = outcome.strip_prefix("ok:") {
            let vocab = Vocabulary::decode_with_limits(&bytes, Limits::default())
                .unwrap_or_else(|e| panic!("{name}: decode failed: {e}"));
            assert_eq!(
                describe_vocabulary(&vocab),
                expected,
                "{name}: semantic descriptor"
            );
            assert_eq!(to_canon(&vocab).unwrap(), bytes, "{name}: canonical bytes");
            assert_eq!(
                from_canon::<Vocabulary>(&bytes).unwrap(),
                vocab,
                "{name}: default decoder"
            );
            assert_eq!(
                hash(&vocab).unwrap().to_hex(),
                *digest,
                "{name}: independent domain hash"
            );
        } else {
            let expected_err = outcome.strip_prefix("err:").expect("error outcome");
            assert_eq!(*digest, "-", "{name}: rejected vector must have '-' digest");
            let err = Vocabulary::decode_with_limits(&bytes, Limits::default())
                .expect_err(&format!("{name}: expected failure"));
            assert_eq!(
                vocabulary_error_text(&err),
                expected_err,
                "{name}: error text mismatch"
            );
            let expected_canon_err = match err {
                VocabularyError::Canon(c) => c,
                VocabularyError::Air(AirError::Canon(c)) => c,
                _ => CanonError::WrongType,
            };
            assert_eq!(
                from_canon::<Vocabulary>(&bytes),
                Err(expected_canon_err),
                "{name}: CanonDecode failure"
            );
        }
    }
}
