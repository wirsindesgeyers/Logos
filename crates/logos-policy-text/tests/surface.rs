//! Etapa 7.1: sintaxe de superfície de políticas (fora da TCB): leitura, impressão e erros.

use logos_air::Vocabulary;
use logos_policy::{Class, PolicyError};
use logos_policy_text::{PolicyTextError, parse_policy, print_policy};

const VOCABULARY: &str = r#"
vocab v1 {
  sort Action
  sort Source
  sort Person
  sort Moment
  sort Kind
  const respond: Kind
  const destructive: Kind
  func kind_of(Action): Kind
  func backup_of(Action): Action
  pred Allowed(Action)
  pred says_request(Source, Action)
  pred external(Source)
  pred is_drop(Action)
  pred executed(Action, Moment)
  pred recent(Moment)
  pred person(Person)
  pred signs(Person, Action)
  pred p(Action)
}
"#;

const EXAMPLE: &str = r#"
# Política de exemplo (7.1): responder a pedidos externos, exigir usuário em ações destrutivas,
# negar o drop sem backup recente.
policy v12 {
  rule r1 allow: forall x:Action. forall e:Source.
      kind_of(x) = respond and says_request(e, x) and external(e) -> Allowed(x)
    comment "Responder a pedido externo é permitido."
  rule r7 require: forall x:Action. kind_of(x) = destructive ->
      (Allowed(x) <-> exists u:Person. person(u) and signs(u, x))
    comment "Ação destrutiva exige a assinatura de um usuário."
  rule r9 deny: forall x:Action. is_drop(x) and
      (not exists t:Moment. (executed(backup_of(x), t) and recent(t))) -> not Allowed(x);
}
"#;

fn vocabulary() -> Vocabulary {
    logos_air_text::parse_document(VOCABULARY, &Default::default())
        .unwrap()
        .vocabulary
}

fn parse(text: &str) -> Result<logos_policy::Policy, PolicyTextError> {
    parse_policy(text, &vocabulary())
}

fn syntax(text: &str) -> String {
    match parse(text).unwrap_err() {
        PolicyTextError::Syntax { message, .. } => message,
        other => panic!("expected a syntax error, got {other}"),
    }
}

#[test]
fn example_parses_into_the_expected_rules_and_round_trips() {
    let policy = parse(EXAMPLE).unwrap();
    assert_eq!(policy.version(), 12);
    let rules: Vec<_> = policy
        .rules()
        .iter()
        .map(|r| (r.id.as_str(), r.class, r.comment.as_str()))
        .collect();
    assert_eq!(
        rules,
        [
            (
                "r1",
                Class::Allow,
                "Responder a pedido externo é permitido."
            ),
            (
                "r7",
                Class::Require,
                "Ação destrutiva exige a assinatura de um usuário."
            ),
            ("r9", Class::Deny, ""),
        ]
    );
    // Imprimir e reler devolve a mesma política (mesmos bytes e hash).
    let printed = print_policy(&policy);
    let again = parse(&printed).unwrap();
    assert_eq!(again, policy);
    assert_eq!(print_policy(&again), printed);
    assert_eq!(again.hash().unwrap(), policy.hash().unwrap());
}

#[test]
fn ids_and_comments_accept_strings_escapes_and_unicode() {
    let text = r#"policy v1 {
        rule "regra 1" derive: forall x:Action. p(x) -> p(x) comment "linha\ncom \"aspas\" e é"
        rule comment_ok derive: forall x:Action. p(x)
    }"#;
    let policy = parse(text).unwrap();
    assert_eq!(policy.rules()[0].id.as_str(), "comment_ok");
    assert_eq!(policy.rules()[1].id.as_str(), "regra 1");
    assert_eq!(
        policy.rules()[1].comment.as_str(),
        "linha\ncom \"aspas\" e é"
    );
    // Ids que não são identificadores saem entre aspas e voltam iguais.
    let printed = print_policy(&policy);
    assert!(printed.contains("rule \"regra 1\" derive:"), "{printed}");
    assert_eq!(parse(&printed).unwrap(), policy);
    // Um id igual a uma palavra reservada também precisa de aspas ao imprimir.
    let reserved = parse(r#"policy v1 { rule "rule" derive: forall x:Action. p(x) }"#).unwrap();
    assert!(print_policy(&reserved).contains("rule \"rule\" derive:"));
    assert_eq!(parse(&print_policy(&reserved)).unwrap(), reserved);
    // Escapes aceitos.
    let escapes =
        parse(r#"policy v1 { rule a derive: forall x:Action. p(x) comment "\\ \/ \t \r A" }"#)
            .unwrap();
    assert_eq!(escapes.rules()[0].comment.as_str(), "\\ / \t \r A");
}

#[test]
fn structure_errors_have_positions_and_messages() {
    assert!(syntax("rule a derive: p").contains("expected `policy`"));
    assert!(syntax("policy 12 { }").contains("version"));
    assert!(syntax("policy v { }").contains("version"));
    assert!(syntax("policy v99999999999999999999 { }").contains("version"));
    assert!(syntax("policy v1 rule").contains("expected `{`"));
    assert!(syntax("policy v1 { foo }").contains("expected `rule ID CLASS:`"));
    assert!(syntax("policy v1 { rule a bogus: p }").contains("expected `rule ID CLASS:`"));
    assert!(syntax("policy v1 { rule a derive p }").contains("expected `rule ID CLASS:`"));
    assert!(syntax("policy v1 { rule a derive: }").contains("empty rule formula"));
    assert!(syntax("policy v1 { rule a derive: ; }").contains("empty rule formula"));
    assert!(
        syntax("policy v1 { rule a derive: forall x:Action. p(x)")
            .contains("expected `rule ID CLASS:` or `}`")
    );
    assert!(syntax("policy v1 { } extra").contains("unexpected input"));
    assert!(syntax(r#"policy v1 { rule "a derive: p }"#).contains("unterminated string"));
    assert!(
        syntax(r#"policy v1 { rule "a\q" derive: forall x:Action. p(x) }"#)
            .contains("invalid rule id")
    );
    assert!(
        syntax(r#"policy v1 { rule a derive: forall x:Action. p(x) comment "\q" }"#)
            .contains("invalid comment")
    );
    assert!(
        syntax(r#"policy v1 { rule a derive: forall x:Action. p(x) comment "\u00" }"#)
            .contains("invalid comment")
    );
    assert!(
        syntax(r#"policy v1 { rule a derive: forall x:Action. p(x) comment "\uD800" }"#)
            .contains("invalid comment")
    );
    // Texto fora da forma canônica (NFD) é recusado, nunca normalizado em silêncio.
    assert!(
        syntax("policy v1 { rule a derive: forall x:Action. p(x) comment \"e\u{301}\" }")
            .contains("not canonical")
    );
    assert!(
        syntax("policy v1 { rule \"e\u{301}\" derive: forall x:Action. p(x) }")
            .contains("not canonical")
    );
    // Posição 1-based em linha e coluna.
    let error = parse("policy v1 {\n  oops\n}").unwrap_err();
    assert!(
        matches!(
            error,
            PolicyTextError::Syntax {
                line: 2,
                column: 3,
                ..
            }
        ),
        "{error}"
    );
    assert!(error.to_string().starts_with("[2:3] Syntax:"));
}

#[test]
fn formula_and_policy_errors_name_the_rule() {
    let error = parse("policy v1 { rule bad derive: forall x:Action. nope(x) }").unwrap_err();
    assert!(
        matches!(&error, PolicyTextError::Formula { rule, .. } if rule == "bad"),
        "{error}"
    );
    assert!(error.to_string().starts_with("rule bad:"));
    // Recusa da TCB passa intacta: ciclo com negação.
    let error =
        parse("policy v1 { rule a derive: forall x:Action. (not p(x)) -> p(x) }").unwrap_err();
    assert!(
        matches!(
            &error,
            PolicyTextError::Policy(PolicyError::NegativeCycle(_))
        ),
        "{error}"
    );
    assert!(error.to_string().contains("negation cycle"));
    // Forma fora de Horn numa regra allow.
    let error = parse("policy v1 { rule a allow: forall x:Action. (p(x) or p(x)) -> Allowed(x) }")
        .unwrap_err();
    assert!(
        matches!(&error, PolicyTextError::Policy(PolicyError::NotHorn(..))),
        "{error}"
    );
    // Regras repetidas.
    let error = parse(
        "policy v1 { rule a derive: forall x:Action. p(x)  rule a derive: forall x:Action. p(x) }",
    )
    .unwrap_err();
    assert!(
        matches!(
            &error,
            PolicyTextError::Policy(PolicyError::DuplicateRule(_))
        ),
        "{error}"
    );
}

#[test]
fn separators_and_a_symbol_named_like_a_keyword_do_not_confuse_the_rule_splitter() {
    // `;` separa regras; uma constante chamada `comment` (entre aspas) não termina a fórmula.
    let v = logos_air_text::parse_document(
        r#"vocab v1 { sort A  const "comment": A  pred Allowed(A)  pred q(A) }"#,
        &Default::default(),
    )
    .unwrap()
    .vocabulary;
    let text = r#"policy v1 {
        rule a derive: forall x:A. x = "comment" -> q(x);
        ;
        rule b derive: forall x:A. q(x) -> q(x) comment "ok"
    }"#;
    let policy = parse_policy(text, &v).unwrap();
    assert_eq!(policy.rules().len(), 2);
    let printed = print_policy(&policy);
    assert_eq!(parse_policy(&printed, &v).unwrap(), policy);
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn surface_text_reproduces_the_python_vector_bytes() {
    // O vocabulário do vetor tem um predicado `q` a mais; o texto e os bytes vêm do gerador Python.
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../spec/vectors/policy/policy.json"
    );
    let vectors: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let vocabulary = logos_air_text::parse_document(
        vectors["vocabulary_surface"].as_str().unwrap(),
        &Default::default(),
    )
    .unwrap()
    .vocabulary;
    // O parser Rust e o Python chegam ao mesmo vocabulário canônico.
    assert_eq!(
        logos_canon::to_canon(&vocabulary).unwrap(),
        unhex(vectors["vocabulary_canon_hex"].as_str().unwrap())
    );
    let text = r#"policy v12 {
      rule r1 allow: forall x:Action. forall e:Source. kind_of(x) = respond and says_request(e, x) and external(e) -> Allowed(x)
        comment "Responder a pedido externo."
      rule r7 require: forall x:Action. kind_of(x) = destructive -> (Allowed(x) <-> exists u:Person. person(u) and signs(u, x))
        comment "Ação destrutiva exige usuário."
      rule r9 deny: forall x:Action. is_drop(x) and (not exists t:Moment. (executed(backup_of(x), t) and recent(t))) -> not Allowed(x)
    }"#;
    let policy = parse_policy(text, &vocabulary).unwrap();
    let example = vectors["valid"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "example")
        .unwrap();
    assert_eq!(
        logos_canon::to_canon(&policy).unwrap(),
        unhex(example["policy_canon_hex"].as_str().unwrap())
    );
    assert_eq!(
        policy.hash().unwrap().to_hex(),
        example["policy_hash_hex"].as_str().unwrap()
    );
}
