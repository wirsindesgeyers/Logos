//! `spec/domains.md` e as constantes do crate dizem a mesma coisa.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use logos_crypto::{sig_tags, tags};

fn rows() -> Vec<(String, String, Option<String>)> {
    let text = std::fs::read_to_string(format!(
        "{}/../../spec/domains.md",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    text.lines()
        .filter_map(|l| {
            let cols: Vec<&str> = l.split('|').map(str::trim).collect();
            // | nome | `tag` | codificação/uso | introduzido |
            if cols.len() == 6 && cols[2].starts_with("`logos/") {
                Some((
                    cols[1].to_string(),
                    cols[2].trim_matches('`').to_string(),
                    Some(cols[3].to_string()),
                ))
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn hash_tags_match_the_spec_table() {
    let rows: Vec<_> = rows()
        .into_iter()
        .filter(|r| !r.1.starts_with("logos/sig/"))
        .collect();
    assert_eq!(rows.len(), tags::RESERVED.len());
    for (name, tag, encoding) in tags::RESERVED {
        let row = rows
            .iter()
            .find(|r| r.0 == name)
            .unwrap_or_else(|| panic!("{name} ausente no spec"));
        assert_eq!(format!("{}\0", row.1).as_bytes(), tag, "tag de {name}");
        assert_eq!(row.2.as_deref(), Some(encoding), "codificação de {name}");
    }
}

#[test]
fn signature_tags_match_the_spec_table() {
    let rows: Vec<_> = rows()
        .into_iter()
        .filter(|r| r.1.starts_with("logos/sig/"))
        .collect();
    assert_eq!(rows.len(), sig_tags::RESERVED.len());
    for (name, tag) in sig_tags::RESERVED {
        let row = rows
            .iter()
            .find(|r| r.0 == name)
            .unwrap_or_else(|| panic!("{name} ausente no spec"));
        assert_eq!(format!("{}\0", row.1).as_bytes(), tag, "tag de {name}");
    }
}

#[test]
fn signature_tags_are_prefix_free_and_disjoint_from_hash_tags() {
    let all: Vec<&[u8]> = tags::RESERVED
        .iter()
        .map(|r| r.1)
        .chain(sig_tags::RESERVED.iter().map(|r| r.1))
        .collect();
    for (i, a) in all.iter().enumerate() {
        assert_eq!(a.iter().filter(|&&b| b == 0).count(), 1);
        for b in &all[i + 1..] {
            assert!(!a.starts_with(b) && !b.starts_with(a));
        }
    }
}
