//! Vetores normativos de `spec/vectors/canon/vectors.tsv`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

#[test]
fn normative_vectors() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../spec/vectors/canon/vectors.tsv"
    );
    let text = std::fs::read_to_string(path).unwrap();
    let mut count = 0;
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let mut cols = line.split('\t');
        let (name, hex, expected) = (
            cols.next().unwrap(),
            cols.next().unwrap(),
            cols.next().unwrap(),
        );
        assert_eq!(
            support::verdict(&support::unhex(hex)),
            expected,
            "vetor {name}"
        );
        count += 1;
    }
    assert!(count >= 30, "esperado ≥ 30 vetores, achei {count}");
    assert!(
        text.matches("\terr:").count() >= 10,
        "poucos vetores de rejeição"
    );
}
