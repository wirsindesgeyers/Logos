//! Public surface smoke: cargo run -p logos-air-text --example air_text_smoke.

use logos_air::{Formula, SortErrorKind, Vocabulary, check};
use logos_air_text::{
    ErrorKind, References, parse_document, parse_formula, print_document, print_formula,
};
use logos_canon::{from_canon, to_canon};
use std::error::Error;

const SOURCE: &str = r#"
vocab v1 {
  sort User = enum [alice] from fact "0000000000000000000000000000000000000000000000000000000000000001"
  sort Db
  const alice: User = glossed("usuário Alice") gloss "Alice"
  func dbOf(User): Db = glossed("banco do usuário") gloss "banco"
  pred Registered(User) = defined(Admin(var(0, User))) gloss "cadastrado"
  pred Admin(User) = anchored("authority", "registry") gloss "administrador"
  pred Owns(User, Db) = glossed("posse") gloss "possui"
}
claim C7: forall u:User. Admin(u) -> exists d:Db. (Owns(u, d) and d = dbOf(u))
  cites [F3, P2, C5]
claim C8: forall u:User. Admin(u) -> Registered(u) cites [C7]
"#;

fn require(condition: bool, message: &str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let references = References::new();
    let document = parse_document(SOURCE, &references)?;
    for claim in &document.claims {
        check(&document.vocabulary, &claim.formula)?;
        let bytes = to_canon(&claim.formula)?;
        require(
            from_canon::<Formula>(&bytes)? == claim.formula,
            "binary Formula roundtrip",
        )?;
        require(
            parse_formula(&print_formula(&claim.formula), &document.vocabulary)? == claim.formula,
            "textual Formula roundtrip",
        )?;
    }
    require(
        document.claims[0]
            .cites
            .iter()
            .map(|name| name.as_str())
            .eq(["F3", "P2", "C5"]),
        "citation order changed",
    )?;
    require(
        document.claims[1].cites[0].as_str() == "C7",
        "claim citation changed",
    )?;
    let vocab_bytes = to_canon(&document.vocabulary)?;
    require(
        from_canon::<Vocabulary>(&vocab_bytes)? == document.vocabulary,
        "binary Vocabulary roundtrip",
    )?;
    let printed = print_document(&document);
    let roundtrip = parse_document(&printed, &references)?;
    require(
        roundtrip.vocabulary == document.vocabulary && roundtrip.claims == document.claims,
        "document printer lost metadata",
    )?;
    let alpha_x = parse_formula("forall x:User. Admin(x)", &document.vocabulary)?;
    let alpha_y = parse_formula("forall y:User. Admin(y)", &document.vocabulary)?;
    require(
        to_canon(&alpha_x)? == to_canon(&alpha_y)?,
        "binder spelling changed bytes",
    )?;
    let ill_typed = parse_formula("forall x:Db. Admin(x)", &document.vocabulary)?;
    require(
        matches!(check(&document.vocabulary, &ill_typed), Err(error) if matches!(error.kind, SortErrorKind::SortMismatch { .. })),
        "parser incorrectly certified sort consistency",
    )?;
    require(
        parse_formula(&print_formula(&ill_typed), &document.vocabulary)? == ill_typed,
        "printer silently repaired sort mismatch",
    )?;
    let unknown = parse_formula("Missing(alice)", &document.vocabulary);
    require(
        matches!(unknown, Err(error) if error.kind == ErrorKind::UnknownName),
        "unknown name accepted",
    )?;
    let trailing = parse_formula("true ; false", &document.vocabulary);
    require(
        matches!(trailing, Err(error) if error.kind == ErrorKind::Syntax),
        "trailing input accepted",
    )?;
    println!("{printed}");
    println!(
        "AIR_TEXT_SMOKE_OK: nominal/explicit AIR, forward Defined, enum source, binary/text roundtrips, alpha-renaming, deferred sort-check, source errors ({})",
        std::env::consts::ARCH
    );
    Ok(())
}
