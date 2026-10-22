//! Vetores independentes paraphrase-pt/v1 (`tools/gen_paraphrase_vectors.py`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use logos_air::{Formula, Limits, Vocabulary};
use logos_seal::Paraphraser;

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn rust_matches_independent_python_renderer_and_hash() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors/paraphrase/vectors.tsv");
    let contents = std::fs::read_to_string(path).unwrap();
    let (mut positive, mut negative) = (0, 0);
    for line in contents.lines().filter(|l| !l.starts_with('#')) {
        let fields: Vec<&str> = line.split('\t').collect();
        let [name, vocabulary_hex, formula_hex, text, outcome] = fields[..] else {
            panic!("linha malformada: {line}");
        };
        let vocabulary =
            Vocabulary::decode_with_limits(&hex(vocabulary_hex), Limits::default()).unwrap();
        let p = Paraphraser::new(&vocabulary).unwrap();
        if let Some(error) = outcome.strip_prefix("err:") {
            let actual = format!("{:?}", p.parse(text).unwrap_err());
            assert!(
                actual.starts_with(error),
                "{name}: esperado {error}, obtido {actual}"
            );
            negative += 1;
            continue;
        }
        let formula = Formula::decode_with_limits(&hex(formula_hex), Limits::default()).unwrap();
        let paraphrase = p.render(&formula).unwrap();
        assert_eq!(paraphrase.text(), text, "{name}");
        let digest: String = paraphrase
            .digest()
            .unwrap()
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(digest, outcome, "{name}: h(P)");
        assert_eq!(p.parse(text).unwrap(), formula, "{name}");
        positive += 1;
    }
    println!("PARAPHRASE_RUST_VECTORS_OK positive={positive} negative={negative}");
    assert_eq!((positive, negative), (17, 12));
}
