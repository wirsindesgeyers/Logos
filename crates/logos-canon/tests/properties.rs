//! Testes de propriedade (gerador próprio, determinístico; semente impressa em caso de falha).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use logos_canon::{Array, CanonError, Map, Value, from_canon};
use support::{Rng, hex, mutate, random_value, verdict};

fn cases() -> usize {
    std::env::var("LOGOS_PROP_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20_000)
}

const SEED: u64 = 0x10605;

/// decode(encode(x)) == x, e a codificação é um ponto fixo.
#[test]
fn roundtrip_value() {
    let mut rng = Rng::new(SEED);
    for i in 0..cases() {
        let v = random_value(&mut rng, 4);
        let bytes = v.to_bytes();
        let back: Value =
            from_canon(&bytes).unwrap_or_else(|e| panic!("caso {i}: {e} em {}", hex(&bytes)));
        assert_eq!(back, v, "caso {i}");
        assert_eq!(back.to_bytes(), bytes, "caso {i}");
    }
}

/// Para toda sequência de bytes aceita b: encode(decode(b)) == b (unicidade da codificação).
#[test]
fn accepted_bytes_are_canonical() {
    let mut rng = Rng::new(SEED + 1);
    let (mut accepted, mut rejected) = (0usize, 0usize);
    for _ in 0..cases() {
        let valid = random_value(&mut rng, 4).to_bytes();
        let mutated = mutate(&mut rng, &valid);
        // `verdict` já afirma encode(decode(b)) == b para os aceitos.
        if verdict(&mutated).starts_with("ok:") {
            accepted += 1;
        } else {
            rejected += 1;
        }
    }
    assert!(
        accepted > 100 && rejected > 100,
        "mutação pouco variada: {accepted}/{rejected}"
    );
}

/// Bytes completamente aleatórios.
#[test]
fn random_bytes_never_accepted_non_canonically() {
    let mut rng = Rng::new(SEED + 2);
    for _ in 0..cases() {
        let len = rng.below(12);
        let bytes: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
        let _ = verdict(&bytes);
    }
}

/// A ordem em que as entradas de um mapa são fornecidas não muda a codificação.
#[test]
fn map_encoding_is_independent_of_insertion_order() {
    let mut rng = Rng::new(SEED + 3);
    for _ in 0..2_000 {
        let n = 1 + rng.below(6);
        let mut entries: Vec<(Value, Value)> = Vec::new();
        for _ in 0..n {
            entries.push((random_value(&mut rng, 1), random_value(&mut rng, 1)));
            if Map::new(entries.clone()).is_err() {
                entries.pop();
            }
        }
        let canonical = Map::new(entries.clone()).unwrap();
        entries.reverse();
        assert_eq!(Map::new(entries).unwrap(), canonical);
    }
}

/// Aninhamento no limite aceita; um nível acima é erro (nunca estouro de pilha).
#[test]
fn depth_boundary() {
    let mut v = Value::Null;
    for _ in 0..logos_canon::MAX_DEPTH {
        v = Value::Array(Array::new(vec![v]).unwrap());
    }
    assert_eq!(from_canon::<Value>(&v.to_bytes()), Ok(v.clone()));
    assert_eq!(Array::new(vec![v]), Err(CanonError::DepthExceeded));
    // Entrada hostil muito mais funda que o limite: erro, sem recursão ilimitada.
    let hostile = vec![0x81u8; 1_000_000];
    assert_eq!(
        from_canon::<Value>(&hostile),
        Err(CanonError::DepthExceeded)
    );
}
