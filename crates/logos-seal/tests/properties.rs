//! Critério 9.1: `parse_pt(render_pt(φ)) == φ` em 100k fórmulas até profundidade 6.
//!
//! Também confere, na mesma amostra, que fórmulas distintas têm textos distintos
//! e que todo texto mutado que o parser aceita é exatamente o render da fórmula lida
//! (a gramática não tem duas grafias para a mesma fórmula).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::collections::HashMap;

use common::{Rng, adversarial, random_formula, symbol, vocabulary};
use logos_air::{SymbolKind, Vocabulary};
use logos_canon::to_canon;
use logos_seal::Paraphraser;

fn policy_vocabulary() -> Vocabulary {
    use SymbolKind::{Const, Func, Pred};
    vocabulary(
        &["Usuário", "Recurso", "Papel"],
        vec![
            (
                symbol("admin", Const, &[], Some("Papel")),
                "o papel de administrador",
            ),
            (
                symbol("leitor", Const, &[], Some("Papel")),
                "o papel de leitor",
            ),
            (
                symbol("raiz", Const, &[], Some("Recurso")),
                "o recurso raiz",
            ),
            (symbol("ana", Const, &[], Some("Usuário")), "Ana"),
            (
                symbol("dono", Func, &["Recurso"], Some("Usuário")),
                "o dono de {0}",
            ),
            (
                symbol("pai", Func, &["Recurso"], Some("Recurso")),
                "a pasta que contém {0}",
            ),
            (
                symbol("Tem", Pred, &["Usuário", "Papel"], None),
                "{0} tem {1}",
            ),
            (
                symbol("Pode", Pred, &["Usuário", "Recurso"], None),
                "{0} pode acessar {1}",
            ),
            (symbol("Publico", Pred, &["Recurso"], None), "{0} é público"),
            (
                symbol("Manut", Pred, &[], None),
                "o sistema está em manutenção",
            ),
        ],
    )
}

/// Mutação em fronteira de caractere usando peças da própria gramática.
fn mutate(text: &str, rng: &mut Rng) -> String {
    let boundaries: Vec<usize> = text
        .char_indices()
        .map(|(i, _)| i)
        .chain([text.len()])
        .collect();
    let at = boundaries[rng.below(boundaries.len())];
    let pieces = [
        "⟦",
        "⟧",
        "(",
        ")",
        "; (",
        "x1",
        "x2",
        "0",
        "1",
        " ",
        "não é o caso que (",
        "verdadeiro",
    ];
    let mut out = text.to_owned();
    match rng.below(3) {
        0 => out.insert_str(at, pieces[rng.below(pieces.len())]),
        1 => {
            if let Some(end) = boundaries.iter().find(|&&b| b > at) {
                out.replace_range(at..*end, "");
            }
        }
        _ => {
            if let Some(end) = boundaries.iter().find(|&&b| b > at) {
                out.replace_range(at..*end, pieces[rng.below(pieces.len())]);
            }
        }
    }
    out
}

/// Sorteia até `target` fórmulas distintas (pela codificação canônica).
fn run(vocabulary: &Vocabulary, seed_base: u64, target: usize) -> ([usize; 7], usize, u64) {
    let p = Paraphraser::new(vocabulary).unwrap();
    let mut by_formula: HashMap<Vec<u8>, String> = HashMap::new();
    let mut by_text: HashMap<String, Vec<u8>> = HashMap::new();
    let mut depths = [0usize; 7];
    let mut accepted_mutations = 0;
    let mut rng = Rng::new(seed_base ^ 0xA5A5);
    let mut seed = seed_base;
    while by_formula.len() < target {
        assert!(
            seed - seed_base < 20 * target as u64,
            "gerador sem diversidade suficiente"
        );
        let formula = random_formula(vocabulary, seed, 6);
        seed += 1;
        let bytes = to_canon(&formula).unwrap();
        if by_formula.contains_key(&bytes) {
            continue;
        }
        assert!(formula.depth() <= 6);
        depths[formula.depth()] += 1;
        let paraphrase = p.render(&formula).unwrap();
        assert_eq!(p.parse(paraphrase.text()).unwrap(), formula, "seed {seed}");
        let text = paraphrase.text().to_owned();
        if let Some(other) = by_text.insert(text.clone(), bytes.clone()) {
            panic!("duas fórmulas, um texto: {text} ({} bytes)", other.len());
        }
        if by_formula.len().is_multiple_of(4) {
            let mutated = mutate(&text, &mut rng);
            if let Ok(parsed) = p.parse(&mutated) {
                accepted_mutations += 1;
                assert_eq!(
                    p.render(&parsed).unwrap().text(),
                    mutated,
                    "grafia alternativa"
                );
            }
        }
        by_formula.insert(bytes, text);
    }
    (depths, accepted_mutations, seed - seed_base)
}

#[test]
fn round_trip_100k_distinct_random_formulas_up_to_depth_6() {
    let (depths_a, mutations_a, seeds_a) = run(&policy_vocabulary(), 0, 50_000);
    let (depths_b, mutations_b, seeds_b) = run(&adversarial(), 1 << 40, 50_000);
    let depths: Vec<usize> = depths_a.iter().zip(depths_b).map(|(a, b)| a + b).collect();
    println!(
        "PARAPHRASE_ROUNDTRIP_OK distinct_formulas=100000 distinct_texts=100000 depths={depths:?} seeds={} accepted_mutations={}",
        seeds_a + seeds_b,
        mutations_a + mutations_b
    );
    assert_eq!(depths.iter().sum::<usize>(), 100_000);
    assert!(
        depths[6] > 1000,
        "a amostra precisa atingir profundidade 6: {depths:?}"
    );
}
