//! Harness do teste diferencial Rust × Python (`py/tests/test_canon_differential.py`).
//!
//! * `diff check`      lê linhas `hex<TAB>veredito` (veredito do Python) e confere o do Rust.
//! * `diff gen N SEED` imprime N linhas `hex<TAB>veredito` geradas e julgadas pelo Rust.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "../tests/support/mod.rs"]
mod support;

use std::io::{BufRead, Write};
use support::{Rng, hex, mutate, random_raw_string, random_value, unhex, verdict};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check") => check(),
        Some("gen") => {
            let n: usize = args[1].parse().unwrap();
            let seed: u64 = args[2].parse().unwrap();
            generate(n, seed);
        }
        _ => {
            eprintln!("uso: diff check | diff gen N SEED");
            std::process::exit(2);
        }
    }
}

fn check() {
    let (mut total, mut bad) = (0usize, 0usize);
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let (h, expected) = line.split_once('\t').unwrap();
        let got = verdict(&unhex(h));
        total += 1;
        if got != expected {
            bad += 1;
            if bad <= 20 {
                eprintln!("DIVERGÊNCIA {h}\n  python: {expected}\n  rust:   {got}");
            }
        }
    }
    println!("{total} casos, {bad} divergências");
    std::process::exit(i32::from(bad > 0));
}

fn generate(n: usize, seed: u64) {
    let mut rng = Rng::new(seed);
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    for i in 0..n {
        let bytes = match i % 3 {
            0 => random_value(&mut rng, 4).to_bytes(),
            1 => {
                let valid = random_value(&mut rng, 4).to_bytes();
                mutate(&mut rng, &valid)
            }
            _ => {
                // texto cru, possivelmente fora de NFC ou com code points não atribuídos
                let s = random_raw_string(&mut rng);
                let mut b = vec![0x60 | s.len() as u8];
                if s.len() >= 24 {
                    b = vec![0x78, s.len() as u8];
                }
                b.extend_from_slice(s.as_bytes());
                b
            }
        };
        writeln!(out, "{}\t{}", hex(&bytes), verdict(&bytes)).unwrap();
    }
}
