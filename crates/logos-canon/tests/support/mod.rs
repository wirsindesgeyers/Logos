//! Utilitários de teste: PRNG determinístico (SplitMix64), geradores de valores e mutações.
//! Sem dependências externas: a mesma semente produz os mesmos casos em qualquer máquina (S3).
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use logos_canon::{Array, Map, Text, Value};
use unicode_normalization::UnicodeNormalization;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Inteiro em `0..n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    pub fn chance(&mut self, num: usize, den: usize) -> bool {
        self.below(den) < num
    }
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2), "hex de tamanho ímpar: {s}");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

/// Intervalos de code points usados para montar strings (inclui atribuídos, não atribuídos,
/// combinantes, Hangul, exclusões de composição e caracteres acrescentados em Unicode 16/17).
const POOL: &[(u32, u32)] = &[
    (0x20, 0x7e),
    (0x00, 0x1f),
    (0x7f, 0x7f),
    (0xa0, 0xff),
    (0x300, 0x36f),
    (0x391, 0x3c9),
    (0x378, 0x379),
    (0x1100, 0x1112),
    (0x1161, 0x1175),
    (0x11a8, 0x11c2),
    (0xac00, 0xd7a3),
    (0x4e00, 0x4e50),
    (0x958, 0x95f),
    (0x2126, 0x2126),
    (0x212a, 0x212b),
    (0xfb00, 0xfb06),
    (0x1e00, 0x1eff),
    (0x1c89, 0x1c8a),
    (0x20c0, 0x20c1),
    (0xe000, 0xe010),
    (0xfdd0, 0xfdef),
    (0xfffe, 0xffff),
    (0x1f300, 0x1f340),
    (0x2fffe, 0x2ffff),
    (0x10fffe, 0x10ffff),
];

fn random_char(rng: &mut Rng) -> char {
    let (lo, hi) = POOL[rng.below(POOL.len())];
    let cp = lo + (rng.next() % u64::from(hi - lo + 1)) as u32;
    char::from_u32(cp).unwrap_or('a')
}

/// Qualquer string (não necessariamente canônica): pode ter NFD, não atribuídos etc.
pub fn random_raw_string(rng: &mut Rng) -> String {
    let len = rng.below(10);
    (0..len).map(|_| random_char(rng)).collect()
}

/// Texto válido: tira os não atribuídos e normaliza para NFC.
pub fn random_text(rng: &mut Rng) -> Text {
    let raw = random_raw_string(rng);
    let kept: String = raw
        .chars()
        .filter(|c| Text::new(c.to_string()).is_ok())
        .collect();
    let nfc: String = kept.nfc().collect();
    Text::new(nfc).unwrap()
}

fn random_u64(rng: &mut Rng) -> u64 {
    const EDGES: [u64; 12] = [
        0,
        1,
        23,
        24,
        255,
        256,
        65_535,
        65_536,
        0xFFFF_FFFF,
        0x1_0000_0000,
        u64::MAX - 1,
        u64::MAX,
    ];
    if rng.chance(1, 3) {
        EDGES[rng.below(EDGES.len())]
    } else {
        let bits = 1 + rng.below(64);
        rng.next() >> (64 - bits)
    }
}

pub fn random_value(rng: &mut Rng, depth_left: usize) -> Value {
    let kinds = if depth_left == 0 { 6 } else { 8 };
    match rng.below(kinds) {
        0 => Value::Null,
        1 => Value::Bool(rng.chance(1, 2)),
        2 => Value::Unsigned(random_u64(rng)),
        3 => Value::Negative(random_u64(rng)),
        4 => {
            let len = rng.below(20);
            Value::Bytes((0..len).map(|_| rng.next() as u8).collect())
        }
        5 => Value::Text(random_text(rng)),
        6 => {
            let n = rng.below(5);
            let items = (0..n).map(|_| random_value(rng, depth_left - 1)).collect();
            Value::Array(Array::new(items).unwrap())
        }
        _ => {
            let n = rng.below(5);
            let mut entries: Vec<(Value, Value)> = Vec::new();
            for _ in 0..n {
                let k = random_value(rng, depth_left - 1);
                let v = random_value(rng, depth_left - 1);
                entries.push((k, v));
                if Map::new(entries.clone()).is_err() {
                    entries.pop(); // chave repetida: descarta
                }
            }
            Value::Map(Map::new(entries).unwrap())
        }
    }
}

/// Aplica 1–3 edições aleatórias (troca, inserção, remoção, truncamento, duplicação).
pub fn mutate(rng: &mut Rng, input: &[u8]) -> Vec<u8> {
    const INTERESTING: [u8; 14] = [
        0x00, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1f, 0x5f, 0xc0, 0xf4, 0xf7, 0xf9, 0xff,
    ];
    let mut b = input.to_vec();
    for _ in 0..=rng.below(3) {
        match rng.below(6) {
            0 if !b.is_empty() => {
                let i = rng.below(b.len());
                b[i] ^= 1 << rng.below(8);
            }
            1 if !b.is_empty() => {
                let i = rng.below(b.len());
                b[i] = INTERESTING[rng.below(INTERESTING.len())];
            }
            2 => {
                let i = rng.below(b.len() + 1);
                b.insert(i, rng.next() as u8);
            }
            3 if !b.is_empty() => {
                let i = rng.below(b.len());
                b.remove(i);
            }
            4 if !b.is_empty() => {
                let n = rng.below(b.len());
                b.truncate(n);
            }
            5 if !b.is_empty() => {
                let i = rng.below(b.len());
                let j = i + rng.below(b.len() - i) + 1;
                let seg = b[i..j].to_vec();
                b.splice(j..j, seg);
            }
            _ => {}
        }
    }
    b
}

/// Veredito no formato dos vetores: `ok:<dump>` ou `err:<Kind>`; confere também o round-trip.
pub fn verdict(bytes: &[u8]) -> String {
    match logos_canon::from_canon::<Value>(bytes) {
        Ok(v) => {
            assert_eq!(
                v.to_bytes(),
                bytes,
                "aceito mas não canônico: {}",
                hex(bytes)
            );
            format!("ok:{v}")
        }
        Err(e) => format!("err:{}", e.name()),
    }
}
