//! Critério 9.1: ida e volta em todas as fórmulas de `spec/vectors/`.
//!
//! Varre todo blob hexadecimal dos vetores, decodifica canon/v1 e procura
//! qualquer subvalor que seja uma Formula air/v1. Para cada fórmula distinta
//! deriva um vocabulary/v2 com os sorts/símbolos usados e templates neutros.
//! Fórmula fechada e bem tipada precisa voltar idêntica; aberta, mal tipada ou
//! com assinaturas conflitantes precisa ser recusada com erro explícito.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use logos_air::{
    Formula, FormulaNode, Grounding, Limits, SortDecl, SortDomain, Symbol, SymbolDecl,
    SymbolResult, TermNode, Vocabulary,
};
use logos_canon::{Decoder, Text, Value, to_canon};
use logos_seal::{ParaphraseError, Paraphraser};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn hex_blobs(text: &str) -> Vec<Vec<u8>> {
    text.split(|c: char| !c.is_ascii_hexdigit())
        .filter(|t| t.len() >= 4 && t.len() % 2 == 0)
        .filter_map(|t| {
            (0..t.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&t[i..i + 2], 16).ok())
                .collect()
        })
        .collect()
}

fn walk(value: &Value, out: &mut BTreeSet<Vec<u8>>) {
    if let Ok(formula) = Formula::from_value_with_limits(value, Limits::default()) {
        out.insert(to_canon(&formula).unwrap());
    }
    match value {
        Value::Array(items) => items.items().iter().for_each(|v| walk(v, out)),
        Value::Map(map) => map.entries().iter().for_each(|(k, v)| {
            walk(k, out);
            walk(v, out);
        }),
        _ => {}
    }
}

fn sort_names(formula: &Formula) -> (BTreeSet<String>, Vec<Symbol>) {
    let mut sorts = BTreeSet::new();
    let mut symbols = Vec::new();
    let mut add_symbol = |s: &Symbol, sorts: &mut BTreeSet<String>| {
        sorts.extend(s.arity.iter().map(|a| a.name.as_str().to_owned()));
        if let SymbolResult::Sort(r) = &s.result {
            sorts.insert(r.name.as_str().to_owned());
        }
        symbols.push(s.clone());
    };
    let mut terms = Vec::new();
    for node in formula.nodes() {
        match node {
            FormulaNode::Eq(l, r) => terms.extend([l.clone(), r.clone()]),
            FormulaNode::Pred { symbol, args } => {
                add_symbol(symbol, &mut sorts);
                terms.extend(args.iter().cloned());
            }
            FormulaNode::Forall(s) | FormulaNode::Exists(s) => {
                sorts.insert(s.name.as_str().to_owned());
            }
            _ => {}
        }
    }
    for term in &terms {
        for node in term.nodes() {
            match node {
                TermNode::Var { sort, .. } => {
                    sorts.insert(sort.name.as_str().to_owned());
                }
                TermNode::App { symbol, .. } => add_symbol(symbol, &mut sorts),
            }
        }
    }
    (sorts, symbols)
}

/// vocabulary/v2 derivado: nomes como usados, templates neutros `sK {0} {1}…`.
fn derived_vocabulary(formula: &Formula) -> Option<Vocabulary> {
    let (sorts, symbols) = sort_names(formula);
    let mut unique: BTreeMap<String, Symbol> = BTreeMap::new();
    for s in symbols {
        if let Some(previous) = unique.insert(s.name.as_str().to_owned(), s.clone())
            && previous != s
        {
            return None;
        }
    }
    let sorts = sorts
        .into_iter()
        .map(|name| SortDecl {
            name: Text::new(name).unwrap(),
            domain: SortDomain::Open,
        })
        .collect();
    let (decls, templates): (Vec<_>, Vec<_>) = unique
        .into_values()
        .enumerate()
        .map(|(k, signature)| {
            let mut template = format!("s{k}");
            for i in 0..signature.arity.len() {
                template.push_str(&format!(" {{{i}}}"));
            }
            (
                SymbolDecl {
                    signature,
                    grounding: Grounding::Glossed(Text::new("vetor").unwrap()),
                    gloss_pt: Text::new("vetor").unwrap(),
                },
                Text::new(template).unwrap(),
            )
        })
        .unzip();
    Vocabulary::new_with_render_pt(1, sorts, decls, templates).ok()
}

#[test]
fn every_formula_in_spec_vectors_round_trips_or_is_refused() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors");
    let mut paths = Vec::new();
    files(&root, &mut paths);
    let mut formulas = BTreeSet::new();
    for path in &paths {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        for blob in hex_blobs(&text) {
            let mut decoder = Decoder::new(&blob);
            if let Ok(value) = decoder.value()
                && decoder.finish().is_ok()
            {
                walk(&value, &mut formulas);
            }
        }
    }
    let (mut round_trip, mut sort_refused, mut vocabulary_refused) = (0, 0, 0);
    let mut other: BTreeMap<String, usize> = BTreeMap::new();
    for bytes in &formulas {
        let formula = Formula::decode_with_limits(bytes, Limits::default()).unwrap();
        let Some(vocabulary) = derived_vocabulary(&formula) else {
            vocabulary_refused += 1;
            continue;
        };
        let paraphraser = match Paraphraser::new(&vocabulary) {
            Ok(p) => p,
            Err(error) => {
                *other.entry(format!("{error:?}")).or_default() += 1;
                continue;
            }
        };
        match paraphraser.render(&formula) {
            Ok(paraphrase) => {
                assert_eq!(paraphraser.parse(paraphrase.text()).unwrap(), formula);
                round_trip += 1;
            }
            Err(ParaphraseError::Sort(_)) => sort_refused += 1,
            Err(ParaphraseError::RoundTripMismatch) => panic!("RoundTripMismatch em vetor"),
            Err(error) => *other.entry(format!("{error:?}")).or_default() += 1,
        }
    }
    println!(
        "PARAPHRASE_SPEC_VECTORS_OK files={} formulas={} round_trip={round_trip} sort_refused={sort_refused} inconsistent_signatures={vocabulary_refused} other={other:?}",
        paths.len(),
        formulas.len()
    );
    // Piso medido em 2026-10-21 (82 fórmulas distintas; 70 fechadas e bem
    // tipadas). Cair abaixo indica que a varredura deixou de achar vetores.
    assert!(
        formulas.len() >= 82 && round_trip >= 70,
        "varredura perdeu fórmulas"
    );
    assert_eq!(
        round_trip + sort_refused + vocabulary_refused + other.values().sum::<usize>(),
        formulas.len()
    );
}
