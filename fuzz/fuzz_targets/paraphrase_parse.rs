//! parse_pt: todo texto aceito é exatamente o render da fórmula lida, e toda
//! Formula decodificável que renderiza volta idêntica (Etapa 9.1).
#![no_main]
use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use logos_air::{Formula, Limits, Vocabulary};
use logos_seal::Paraphraser;

/// Vocabulários distintos de `spec/vectors/paraphrase/vectors.tsv`, na ordem de aparição.
fn vocabularies() -> &'static [Vocabulary] {
    static VOCABULARIES: OnceLock<Vec<Vocabulary>> = OnceLock::new();
    VOCABULARIES.get_or_init(|| {
        let mut seen: Vec<&str> = Vec::new();
        include_str!("../../spec/vectors/paraphrase/vectors.tsv")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| line.split('\t').nth(1))
            .filter(|hex| {
                let new = !seen.contains(hex);
                seen.push(hex);
                new
            })
            .map(|hex| {
                let bytes: Vec<u8> = (0..hex.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                    .collect();
                Vocabulary::decode_with_limits(&bytes, Limits::default()).unwrap()
            })
            .collect()
    })
}

fuzz_target!(|data: &[u8]| {
    let Some((&selector, rest)) = data.split_first() else {
        return;
    };
    let all = vocabularies();
    let vocabulary = &all[usize::from(selector) % all.len()];
    let paraphraser = Paraphraser::new(vocabulary).unwrap();
    if let Ok(text) = std::str::from_utf8(rest)
        && let Ok(formula) = paraphraser.parse(text)
    {
        let rendered = paraphraser.render(&formula).unwrap();
        assert_eq!(rendered.text(), text, "grafia alternativa aceita");
    }
    if let Ok(formula) = Formula::decode_with_limits(rest, Limits::default())
        && let Ok(paraphrase) = paraphraser.render(&formula)
    {
        assert_eq!(paraphraser.parse(paraphrase.text()).unwrap(), formula);
    }
});
