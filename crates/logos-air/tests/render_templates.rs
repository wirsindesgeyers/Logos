//! Contrato independente vocabulary/v2, limites e identidade de templates.
use logos_air::{
    Compatibility, Grounding, Sort, SortDecl, SortDomain, Symbol, SymbolDecl, SymbolKind,
    SymbolResult, Vocabulary, VocabularyError,
};
use logos_canon::{Text, from_canon, to_canon};
use logos_crypto::hash;

fn text(value: &str) -> Text {
    Text::new(value).unwrap()
}
fn symbol(name: &str, arity: usize) -> SymbolDecl {
    SymbolDecl {
        signature: Symbol {
            name: text(name),
            kind: SymbolKind::Pred,
            arity: vec![Sort { name: text("S") }; arity],
            result: SymbolResult::Bool,
        },
        grounding: Grounding::Glossed(text("glosa livre")),
        gloss_pt: text("documentação"),
    }
}
fn sorts() -> Vec<SortDecl> {
    vec![SortDecl {
        name: text("S"),
        domain: SortDomain::Open,
    }]
}
fn vocabulary(templates: &[&str]) -> Vocabulary {
    Vocabulary::new_with_render_pt(
        7,
        sorts(),
        vec![symbol("P", 2)],
        templates.iter().map(|s| text(s)).collect(),
    )
    .unwrap()
}

#[test]
fn hash_includes_templates_and_v1_remains_byte_exact() {
    let v1 = Vocabulary::new(7, sorts(), vec![symbol("P", 2)]).unwrap();
    let bytes = to_canon(&v1).unwrap();
    assert_eq!(bytes[0..2], [0x84, 1]);
    assert!(v1.render_pt().is_none());
    assert_eq!(
        to_canon(&from_canon::<Vocabulary>(&bytes).unwrap()).unwrap(),
        bytes
    );
    let v2 = vocabulary(&["{0} é dono de {1}"]);
    let changed = vocabulary(&["{0} administra {1}"]);
    assert_ne!(hash(&v1).unwrap(), hash(&v2).unwrap());
    assert_ne!(hash(&v2).unwrap(), hash(&changed).unwrap());
    let encoded = to_canon(&v2).unwrap();
    assert_eq!(encoded[0..2], [0x85, 2]);
    assert_eq!(from_canon::<Vocabulary>(&encoded).unwrap(), v2);
    assert_eq!(v2.compatibility_with(&v1), Compatibility::Incompatible);
    assert_eq!(v1.compatibility_with(&v2), Compatibility::Incompatible);
    assert_eq!(changed.compatibility_with(&v2), Compatibility::Incompatible);
}

#[test]
fn template_counts_order_literals_and_controls_are_checked() {
    for template in [
        "",
        "{0} {1}",
        "{1} tem {0}",
        "{0} tem {0}",
        "{00} tem {1}",
        "{0} tem {2}",
        "{0} tem",
        "{0} tem {x}",
        "{0} tem {1}}",
        "{0} tem ⟦{1}",
        "{0} tem \n{1}",
        "{0} tem \u{202e}{1}",
        "{0} tem \u{200b}{1}",
    ] {
        assert_eq!(
            Vocabulary::new_with_render_pt(7, sorts(), vec![symbol("P", 2)], vec![text(template)]),
            Err(VocabularyError::InvalidRenderTemplate),
            "{template:?}"
        );
    }
    assert_eq!(
        Vocabulary::new_with_render_pt(7, sorts(), vec![symbol("P", 2)], vec![]),
        Err(VocabularyError::InvalidRenderTemplate)
    );
    let empty = Vocabulary::new_with_render_pt(0, vec![], vec![], vec![]).unwrap();
    assert_eq!(empty.render_pt(), Some([].as_slice()));
    assert_eq!(
        from_canon::<Vocabulary>(&to_canon(&empty).unwrap()).unwrap(),
        empty
    );
}

#[test]
fn byte_and_aggregate_limits_are_inclusive() {
    let exact = "é".repeat(2048);
    assert!(
        Vocabulary::new_with_render_pt(1, sorts(), vec![symbol("P", 0)], vec![text(&exact)])
            .is_ok()
    );
    assert!(
        Vocabulary::new_with_render_pt(
            1,
            sorts(),
            vec![symbol("P", 0)],
            vec![text(&(exact + "a"))]
        )
        .is_err()
    );
    let templates = vec![text(&"a".repeat(4096)); 256];
    let mut symbols: Vec<_> = (0..256).map(|i| symbol(&format!("P{i}"), 0)).collect();
    assert!(Vocabulary::new_with_render_pt(1, sorts(), symbols.clone(), templates.clone()).is_ok());
    symbols.push(symbol("extra", 0));
    let mut over = templates;
    over.push(text("a"));
    assert_eq!(
        Vocabulary::new_with_render_pt(1, sorts(), symbols, over),
        Err(VocabularyError::InvalidRenderTemplate)
    );
}

#[test]
fn extension_requires_explicit_templates_and_compares_by_symbol() {
    let old = vocabulary(&["{0} é dono de {1}"]);
    assert_eq!(
        old.extend(vec![], vec![symbol("Q", 0)]),
        Err(VocabularyError::InvalidRenderTemplate)
    );
    let (next, compatible) = old
        .extend_with_render_pt(
            vec![],
            vec![symbol("Q", 0)],
            vec![text("{0} é dono de {1}"), text("chove")],
        )
        .unwrap();
    assert_eq!(compatible, Compatibility::Compatible);
    assert_eq!(next.version(), 8);
    let reordered = Vocabulary::new_with_render_pt(
        8,
        sorts(),
        vec![symbol("Q", 0), symbol("P", 2)],
        vec![text("chove"), text("{0} é dono de {1}")],
    )
    .unwrap();
    assert_eq!(
        reordered.compatibility_with(&next),
        Compatibility::Compatible
    );
    let (_, changed) = old
        .extend_with_render_pt(vec![], vec![], vec![text("{0} administra {1}")])
        .unwrap();
    assert_eq!(changed, Compatibility::Incompatible);
}

#[test]
fn independent_v2_bytes_hashes_and_negative_vectors() {
    for line in include_str!("../../../spec/vectors/vocabulary-v2/vectors.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        let bytes: Vec<_> = (0..fields[1].len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&fields[1][i..i + 2], 16).unwrap())
            .collect();
        let result = from_canon::<Vocabulary>(&bytes);
        if fields[2] == "error" {
            assert!(result.is_err(), "{}", fields[0]);
        } else {
            let vocabulary = result.unwrap();
            assert_eq!(
                hash(&vocabulary).unwrap().to_hex(),
                fields[2],
                "{}",
                fields[0]
            );
            assert_eq!(to_canon(&vocabulary).unwrap(), bytes);
        }
    }
}
