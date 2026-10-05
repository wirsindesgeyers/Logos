//! Testes de documentos AIR: declarações, referências, invariantes de vocabulário,
//! forward references em corpos Defined, enums, citações e round-trip completo.

use logos_air_text::{ErrorKind, References, parse_document, print_document};

#[test]
fn test_valid_document_matching_spec_example() {
    let source = r#"
vocab v1 {
  sort User = enum [alice] from fact "0000000000000000000000000000000000000000000000000000000000000001"
  sort Db
  sort Action
  const alice: User = glossed("usuário Alice") gloss "Alice"
  pred Admin(User)
  pred Owns(User, Db)
  pred Allowed(Action)
}
claim C7: forall u:User. Admin(u) -> exists d:Db. Owns(u, d)
  cites [F3, P2, C5]
"#;
    let references = References::new();
    let doc = parse_document(source, &references).expect("document should parse");

    assert_eq!(doc.vocabulary.version(), 1);
    assert_eq!(doc.vocabulary.sorts().len(), 3);
    assert_eq!(doc.vocabulary.symbols().len(), 4);
    assert_eq!(doc.claims.len(), 1);
    assert_eq!(doc.claims[0].name.as_str(), "C7");
    assert_eq!(doc.claims[0].cites.len(), 3);
    assert_eq!(doc.claims[0].cites[0].as_str(), "F3");
    assert_eq!(doc.claims[0].cites[1].as_str(), "P2");
    assert_eq!(doc.claims[0].cites[2].as_str(), "C5");

    // Round-trip check
    let printed = print_document(&doc);
    let roundtrip = parse_document(&printed, &references).expect("roundtrip should parse");
    assert_eq!(doc.vocabulary, roundtrip.vocabulary);
    assert_eq!(doc.claims, roundtrip.claims);
}

#[test]
fn test_optional_semicolons_and_empty_claims() {
    let source = r#"
vocab v42 {
  sort S;
  const c: S;
}
"#;
    let references = References::new();
    let doc = parse_document(source, &references).expect("document with semicolons");
    assert_eq!(doc.vocabulary.version(), 42);
    assert!(doc.claims.is_empty());

    let printed = print_document(&doc);
    let roundtrip = parse_document(&printed, &references).expect("roundtrip");
    assert_eq!(doc.vocabulary, roundtrip.vocabulary);
    assert_eq!(doc.claims, roundtrip.claims);
}

#[test]
fn test_default_omitted_grounding_and_gloss() {
    let source = r#"
vocab v1 {
  sort S
  const c: S
  func f(S): S
  pred P(S)
}
"#;
    let doc = parse_document(source, &References::new()).expect("omitted grounding and gloss");
    for sym in doc.vocabulary.symbols() {
        assert!(matches!(
            &sym.grounding,
            logos_air::Grounding::Glossed(text) if text.as_str().is_empty()
        ));
        assert!(sym.gloss_pt.as_str().is_empty());
    }
}

#[test]
fn test_forward_references_in_defined_groundings() {
    let source = r#"
vocab v1 {
  sort S
  pred A(S) = defined(B(var(0, S))) gloss "A"
  pred B(S) = glossed("B") gloss "B"
}
"#;
    let doc = parse_document(source, &References::new()).expect("forward defined reference");
    assert_eq!(doc.vocabulary.symbols().len(), 2);
}

#[test]
fn test_duplicate_sort_name_rejected() {
    let source = r#"
vocab v1 {
  sort User
  sort User
}
"#;
    let err = parse_document(source, &References::new()).unwrap_err();
    assert_eq!(err.kind, ErrorKind::DuplicateName);
}

#[test]
fn test_duplicate_symbol_name_rejected() {
    let source = r#"
vocab v1 {
  sort S
  const c: S
  const c: S
}
"#;
    let err = parse_document(source, &References::new()).unwrap_err();
    assert_eq!(err.kind, ErrorKind::DuplicateName);
}

#[test]
fn test_duplicate_claim_name_rejected() {
    let source = r#"
vocab v1 {
  sort S
}
claim C1: true
claim C1: false
"#;
    let err = parse_document(source, &References::new()).unwrap_err();
    assert_eq!(err.kind, ErrorKind::DuplicateName);
}

#[test]
fn test_unknown_reference_rejected() {
    let source = r#"
vocab v1 {
  sort User = enum [alice] from fact MissingRef
  const alice: User
}
"#;
    let err = parse_document(source, &References::new()).unwrap_err();
    assert_eq!(err.kind, ErrorKind::UnknownReference);
}

#[test]
fn test_reference_alias_lookup() {
    let source = r#"
vocab v1 {
  sort User = enum [alice] from fact MyFactAlias
  const alice: User
}
"#;
    let mut refs = References::new();
    let digest = [0x42u8; 32];
    refs.insert("MyFactAlias".to_string(), digest);

    let doc = parse_document(source, &refs).expect("resolved alias");
    match &doc.vocabulary.sorts()[0].domain {
        logos_air::SortDomain::Enumerated { domain_source, .. } => match domain_source {
            logos_air::DomainSource::Fact(fact_ref) => assert_eq!(fact_ref.0, digest),
            _ => panic!("expected FactRef"),
        },
        _ => panic!("expected Enumerated sort"),
    }
}

#[test]
fn test_case_insensitive_hex_reference() {
    let source = r#"
vocab v1 {
  sort User = enum [alice] from policy "AaBbCcDdEeFf00112233445566778899AABBCCDDEEFF00112233445566778899"
  const alice: User
}
"#;
    let doc = parse_document(source, &References::new()).expect("case insensitive hex");
    let printed = print_document(&doc);
    // Printer writes lowercase hex:
    assert!(printed.contains("aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899"));
}

#[test]
fn test_circular_definitions_rejected() {
    let source = r#"
vocab v1 {
  sort S
  pred A(S) = defined(B(var(0, S))) gloss "A"
  pred B(S) = defined(A(var(0, S))) gloss "B"
}
"#;
    let err = parse_document(source, &References::new()).unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidVocabulary);
}

#[test]
fn test_enum_with_missing_constant_declaration() {
    let source = r#"
vocab v1 {
  sort User = enum [missing] from fact "0000000000000000000000000000000000000000000000000000000000000001"
}
"#;
    let err = parse_document(source, &References::new()).unwrap_err();
    assert_eq!(err.kind, ErrorKind::UnknownName);
}

#[test]
fn test_enum_constant_with_wrong_kind() {
    let source = r#"
vocab v1 {
  sort User = enum [notAConst] from fact "0000000000000000000000000000000000000000000000000000000000000001"
  func notAConst(User): User
}
"#;
    let err = parse_document(source, &References::new()).unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidVocabulary);
}

#[test]
fn test_enum_constant_with_wrong_sort() {
    let source = r#"
vocab v1 {
  sort User = enum [wrongSort] from fact "0000000000000000000000000000000000000000000000000000000000000001"
  sort Other
  const wrongSort: Other
}
"#;
    let err = parse_document(source, &References::new()).unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidVocabulary);
}

#[test]
fn test_unicode_reference_is_not_ascii_hex() {
    // 64 bytes, but byte 2 splits é: reference decoding must reject, not panic.
    let reference = format!("aé{}", "a".repeat(61));
    let source = format!("vocab v0 {{ sort S = enum [c] from fact \"{reference}\" const c:S }}");
    let err = parse_document(&source, &References::new()).unwrap_err();
    assert_eq!(err.kind, ErrorKind::UnknownReference);
}
