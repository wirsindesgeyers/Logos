//! Templates explícitos sobrevivem ao round-trip documental.
use logos_air_text::{References, parse_document, print_document};
use logos_canon::to_canon;

#[test]
fn templates_survive_surface_roundtrip_and_are_never_inferred() {
    let source = "vocab v7 { sort S pred P(S,S) render_pt [\"{0} é dono de {1}\"]; } claim C: forall x:S. P(x,x)";
    let document = parse_document(source, &References::new()).unwrap();
    assert!(document.vocabulary.render_pt().is_some());
    let printed = print_document(&document);
    let again = parse_document(&printed, &References::new()).unwrap();
    assert_eq!(
        to_canon(&document.vocabulary).unwrap(),
        to_canon(&again.vocabulary).unwrap()
    );
    assert_eq!(document.claims, again.claims);
    assert!(
        parse_document("vocab v1 { pred P() }", &References::new())
            .unwrap()
            .vocabulary
            .render_pt()
            .is_none()
    );
    for bad in [
        "vocab v1 { pred P() render_pt [] }",
        "vocab v1 { render_pt [] render_pt [] }",
        "vocab v1 { sort S pred P(S) render_pt [\"glosa sem argumento\"] }",
        "vocab v1 { pred P() render_pt [P] }",
    ] {
        assert!(parse_document(bad, &References::new()).is_err(), "{bad}");
    }
}
