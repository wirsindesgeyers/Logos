//! Endereço e layout cas/v1 contra vetores hashlib derivados somente da especificação.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use logos_crypto::{Domain, tags};
use logos_ledger::{Cas, Limits};
use std::{
    fs, io,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Certificate;
impl Domain for Certificate {
    const TAG: &'static [u8] = tags::CERTIFICATE;
}
struct Obligation;
impl Domain for Obligation {
    const TAG: &'static [u8] = tags::OBLIGATION;
}
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "logos-cas-vectors-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("create temp: {e}"),
            }
        }
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2));
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn check<T: Domain>(temp: &Temp, columns: &[&str]) {
    assert_eq!(unhex(columns[1]), T::TAG);
    let bytes = unhex(columns[2]);
    let cas = Cas::open(&temp.0, Limits::default()).unwrap();
    let hash = cas.put::<T>(&bytes).unwrap();
    assert_eq!(hash.to_hex(), columns[3], "{}", columns[0]);
    assert_eq!(fs::read(temp.0.join(columns[4])).unwrap(), bytes);
    drop(cas);
    let reopened = Cas::open(&temp.0, Limits::default()).unwrap();
    assert_eq!(reopened.get(&hash).unwrap(), Some(bytes));
}
#[test]
fn cas_addresses_match_independent_vectors() {
    let temp = Temp::new();
    let vectors = include_str!("../../../spec/vectors/cas/vectors.tsv");
    for line in vectors
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
    {
        let columns = line.split('\t').collect::<Vec<_>>();
        assert_eq!(columns.len(), 5);
        match columns[0] {
            "certificate-empty" | "certificate-binary" => check::<Certificate>(&temp, &columns),
            "obligation-canonical" => check::<Obligation>(&temp, &columns),
            name => panic!("unknown vector: {name}"),
        }
    }
}
