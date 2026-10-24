//! Etapa 7.1: política `policy/v1` — sort-check, forma de Horn, estratificação e codec estrito.

use logos_air::{Formula, Vocabulary};
use logos_canon::{Text, Value, from_canon, to_canon};
use logos_crypto::hash;
use logos_policy::{Class, Policy, PolicyError, Rule};

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
  pred q(Action)
  pred r(Action)
}
"#;

fn vocabulary() -> Vocabulary {
    logos_air_text::parse_document(VOCABULARY, &Default::default())
        .unwrap()
        .vocabulary
}

fn formula(text: &str) -> Formula {
    logos_air_text::parse_formula(text, &vocabulary()).unwrap()
}

fn rule(id: &str, class: Class, text: &str) -> Rule {
    Rule {
        id: Text::new(id).unwrap(),
        class,
        formula: formula(text),
        comment: Text::new("glosa").unwrap(),
    }
}

const R1: &str = "forall x:Action. forall e:Source. kind_of(x) = respond and says_request(e, x) and external(e) -> Allowed(x)";
const R7: &str = "forall x:Action. kind_of(x) = destructive -> (Allowed(x) <-> exists u:Person. person(u) and signs(u, x))";
const R9: &str = "forall x:Action. is_drop(x) and (not exists t:Moment. (executed(backup_of(x), t) and recent(t))) -> not Allowed(x)";

fn example() -> Vec<Rule> {
    vec![
        rule("r9", Class::Deny, R9),
        rule("r1", Class::Allow, R1),
        rule("r7", Class::Require, R7),
        rule("r0", Class::Derive, "forall x:Action. p(x) -> q(x)"),
    ]
}

fn refused(rules: Vec<Rule>) -> PolicyError {
    Policy::new(1, &vocabulary(), rules).unwrap_err()
}

#[test]
fn example_policy_is_sorted_hashed_and_round_trips_through_canon() {
    let v = vocabulary();
    let policy = Policy::new(12, &v, example()).unwrap();
    let ids: Vec<_> = policy.rules().iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["r0", "r1", "r7", "r9"]);
    assert_eq!(policy.version(), 12);
    assert_eq!(policy.vocabulary(), hash(&v).unwrap());
    assert_eq!(policy.rule("r7").unwrap().class, Class::Require);
    assert!(policy.rule("zz").is_none());
    let bytes = to_canon(&policy).unwrap();
    let back = Policy::decode(&bytes, &v).unwrap();
    assert_eq!(back, policy);
    assert_eq!(back.hash().unwrap(), policy.hash().unwrap());
    // A ordem de entrada não muda a identidade.
    let mut shuffled = example();
    shuffled.reverse();
    assert_eq!(
        Policy::new(12, &v, shuffled).unwrap().hash().unwrap(),
        policy.hash().unwrap()
    );
    // Versão faz parte da identidade.
    assert_ne!(
        Policy::new(13, &v, example()).unwrap().hash().unwrap(),
        policy.hash().unwrap()
    );
    for class in [Class::Allow, Class::Deny, Class::Require, Class::Derive] {
        assert!(!class.name().is_empty());
    }
}

#[test]
fn derive_rules_and_positive_recursion_are_stratifiable() {
    let rules = vec![
        rule("d1", Class::Derive, "forall x:Action. p(x) -> q(x)"),
        rule("d2", Class::Derive, "forall x:Action. q(x) -> p(x)"),
        rule(
            "d3",
            Class::Derive,
            "forall x:Action. (not exists y:Action. (q(y) and p(y))) -> r(x)",
        ),
        rule(
            "a1",
            Class::Allow,
            "forall x:Action. q(x) and not r(x) -> Allowed(x)",
        ),
        rule(
            "a2",
            Class::Allow,
            "forall x:Action. forall y:Action. Allowed(y) and p(x) -> Allowed(x)",
        ),
        rule(
            "a3",
            Class::Allow,
            "forall x:Action. exists t:Moment. executed(x, t)",
        ),
    ];
    // a3 não tem cabeça Allowed: recusada com a regra certa.
    let error = refused(rules.clone());
    assert!(
        matches!(&error, PolicyError::BadHead(id, _) if id == "a3"),
        "{error}"
    );
    let ok: Vec<_> = rules.into_iter().take(5).collect();
    Policy::new(1, &vocabulary(), ok).unwrap();
}

#[test]
fn negation_cycles_are_refused() {
    // p depende negativamente de q e q de p.
    let error = refused(vec![
        rule("d1", Class::Derive, "forall x:Action. (not q(x)) -> p(x)"),
        rule("d2", Class::Derive, "forall x:Action. p(x) -> q(x)"),
    ]);
    assert!(matches!(error, PolicyError::NegativeCycle(_)), "{error}");
    // Autonegação direta.
    let error = refused(vec![rule(
        "d1",
        Class::Derive,
        "forall x:Action. (not p(x)) -> p(x)",
    )]);
    assert!(
        matches!(error, PolicyError::NegativeCycle(ref p) if p == "p"),
        "{error}"
    );
    // Allowed sob negação no corpo de uma regra allow.
    let error = refused(vec![rule(
        "a1",
        Class::Allow,
        "forall x:Action. forall y:Action. (not Allowed(y)) and p(x) -> Allowed(x)",
    )]);
    assert!(matches!(error, PolicyError::NegativeCycle(_)), "{error}");
    // deny-overrides: um corpo deny que depende de Allowed fecha ciclo com a negação do deny.
    let error = refused(vec![
        rule("a1", Class::Allow, "forall x:Action. p(x) -> Allowed(x)"),
        rule(
            "d1",
            Class::Deny,
            "forall x:Action. forall y:Action. Allowed(y) and q(x) -> not Allowed(x)",
        ),
    ]);
    assert!(matches!(error, PolicyError::NegativeCycle(_)), "{error}");
    // O ciclo pode começar no corpo deny: deny -neg-> q -> Allowed -neg-> deny.
    let error = refused(vec![
        rule("a1", Class::Derive, "forall x:Action. Allowed(x) -> q(x)"),
        rule(
            "b1",
            Class::Deny,
            "forall x:Action. (not q(x)) and p(x) -> not Allowed(x)",
        ),
    ]);
    assert_eq!(
        error,
        PolicyError::NegativeCycle("(deny rules)".into()),
        "{error}"
    );
    // Mesmo corpo deny sem Allowed é estratificável.
    Policy::new(
        1,
        &vocabulary(),
        vec![
            rule("a1", Class::Allow, "forall x:Action. p(x) -> Allowed(x)"),
            rule("d1", Class::Deny, "forall x:Action. q(x) -> not Allowed(x)"),
        ],
    )
    .unwrap();
}

#[test]
fn rules_outside_the_restricted_form_or_with_the_wrong_head_are_refused() {
    let not_horn = [
        "forall x:Action. (p(x) or q(x)) -> Allowed(x)",
        "forall x:Action. (p(x) <-> q(x)) -> Allowed(x)",
        "forall x:Action. (forall y:Action. p(y)) -> Allowed(x)",
        "forall x:Action. (p(x) -> q(x)) -> Allowed(x)",
        "forall x:Action. false -> Allowed(x)",
        "forall x:Action. (not not p(x)) -> Allowed(x)",
        "forall x:Action. (not exists y:Action. (not q(y))) -> Allowed(x)",
    ];
    for text in not_horn {
        let error = refused(vec![rule("a1", Class::Allow, text)]);
        assert!(matches!(error, PolicyError::NotHorn(..)), "{text}: {error}");
    }
    // Corpo com igualdade e `true` é Horn.
    Policy::new(
        1,
        &vocabulary(),
        vec![rule(
            "a1",
            Class::Allow,
            "forall x:Action. x = x and true and not (x = backup_of(x)) -> Allowed(x)",
        )],
    )
    .unwrap();
    // Cabeças erradas por classe.
    let heads = [
        (Class::Allow, "forall x:Action. p(x) -> q(x)"),
        (Class::Allow, "forall x:Action. p(x) -> not Allowed(x)"),
        (Class::Deny, "forall x:Action. p(x) -> Allowed(x)"),
        (Class::Deny, "forall x:Action. p(x) -> not q(x)"),
        (Class::Derive, "forall x:Action. q(x) -> Allowed(x)"),
        (Class::Derive, "forall x:Action. q(x) -> not p(x)"),
        (Class::Derive, "forall x:Action. q(x) -> x = x"),
    ];
    for (class, text) in heads {
        let error = refused(vec![rule("h1", class, text)]);
        assert!(matches!(error, PolicyError::BadHead(..)), "{text}: {error}");
    }
    // Regra sem corpo: só a cabeça (fato de política).
    Policy::new(
        1,
        &vocabulary(),
        vec![rule("f1", Class::Derive, "forall x:Action. p(x)")],
    )
    .unwrap();
    // `require` precisa falar de Allowed.
    let error = refused(vec![rule(
        "q1",
        Class::Require,
        "forall x:Action. p(x) -> q(x)",
    )]);
    assert!(
        matches!(error, PolicyError::RequireWithoutAllowed(ref id) if id == "q1"),
        "{error}"
    );
}

#[test]
fn structural_errors_are_explicit() {
    let v = vocabulary();
    assert_eq!(
        Policy::new(0, &v, example()).unwrap_err(),
        PolicyError::ZeroVersion
    );
    let duplicate = vec![rule("a", Class::Derive, "forall x:Action. p(x)"); 2];
    assert!(matches!(refused(duplicate), PolicyError::DuplicateRule(id) if id == "a"));
    assert_eq!(
        refused(vec![rule("", Class::Derive, "forall x:Action. p(x)")]),
        PolicyError::BadRuleId
    );
    let long = "x".repeat(logos_policy::MAX_RULE_ID_BYTES + 1);
    assert_eq!(
        refused(vec![rule(&long, Class::Derive, "forall x:Action. p(x)")]),
        PolicyError::BadRuleId
    );
    let mut rules = vec![rule("a", Class::Derive, "forall x:Action. p(x)")];
    rules[0].comment = Text::new("c".repeat(logos_policy::MAX_COMMENT_BYTES + 1)).unwrap();
    assert!(matches!(refused(rules), PolicyError::CommentTooLong(id) if id == "a"));
    let many: Vec<_> = (0..=logos_policy::MAX_RULES)
        .map(|i| rule(&format!("r{i:05}"), Class::Derive, "forall x:Action. p(x)"))
        .collect();
    assert_eq!(refused(many), PolicyError::TooManyRules);
    // Fórmula mal tipada no vocabulário da política (sort errado em Allowed).
    let wrong = Formula::new(
        vec![logos_air::FormulaNode::Pred {
            symbol: v
                .symbols()
                .iter()
                .find(|d| d.signature.name.as_str() == "person")
                .unwrap()
                .signature
                .clone(),
            args: vec![
                logos_air::Term::new(
                    vec![logos_air::TermNode::Var {
                        index: 0,
                        sort: logos_air::Sort {
                            name: Text::new("Action").unwrap(),
                        },
                    }],
                    logos_air::Limits::default(),
                )
                .unwrap(),
            ],
        }],
        logos_air::Limits::default(),
    )
    .unwrap();
    let ill = Rule {
        id: Text::new("w").unwrap(),
        class: Class::Derive,
        formula: wrong,
        comment: Text::new("").unwrap(),
    };
    let sort_error = refused(vec![ill]);
    assert!(matches!(&sort_error, PolicyError::Sort(id, _) if id == "w"));
    assert!(sort_error.to_string().starts_with("rule w:"));
    // Sem `Allowed` unário no vocabulário.
    let no_allowed =
        logos_air_text::parse_document("vocab v1 { sort A  pred p(A) }", &Default::default())
            .unwrap()
            .vocabulary;
    let f = logos_air_text::parse_formula("forall x:A. p(x)", &no_allowed).unwrap();
    let r = Rule {
        id: Text::new("r").unwrap(),
        class: Class::Derive,
        formula: f,
        comment: Text::new("").unwrap(),
    };
    assert_eq!(
        Policy::new(1, &no_allowed, vec![r]).unwrap_err(),
        PolicyError::MissingAllowed
    );
    // `Allowed` precisa ser predicado unário: função ou predicado binário não servem.
    for declaration in ["func Allowed(A): A", "pred Allowed(A, A)"] {
        let wrong_kind = logos_air_text::parse_document(
            &format!("vocab v1 {{ sort A  pred p(A)  {declaration} }}"),
            &Default::default(),
        )
        .unwrap()
        .vocabulary;
        let f = logos_air_text::parse_formula("forall x:A. p(x)", &wrong_kind).unwrap();
        let r = Rule {
            id: Text::new("r").unwrap(),
            class: Class::Derive,
            formula: f,
            comment: Text::new("").unwrap(),
        };
        assert_eq!(
            Policy::new(1, &wrong_kind, vec![r]).unwrap_err(),
            PolicyError::MissingAllowed,
            "{declaration}"
        );
    }
    // Política vazia é válida mesmo sem `Allowed` (nada a conferir).
    Policy::new(1, &no_allowed, vec![]).unwrap();
    for error in [
        PolicyError::ZeroVersion,
        PolicyError::TooManyRules,
        PolicyError::VocabularyMismatch,
        PolicyError::BadRuleId,
        PolicyError::DuplicateRule("a".into()),
        PolicyError::UnsortedRules,
        PolicyError::CommentTooLong("a".into()),
        PolicyError::MissingAllowed,
        PolicyError::NotHorn("a".into(), "x"),
        PolicyError::BadHead("a".into(), "x"),
        PolicyError::RequireWithoutAllowed("a".into()),
        PolicyError::NegativeCycle("p".into()),
        PolicyError::TooLarge,
        PolicyError::Canon(logos_canon::CanonError::WrongType),
    ] {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn decode_is_strict_and_revalidates_against_the_vocabulary() {
    let v = vocabulary();
    let policy = Policy::new(3, &v, example()).unwrap();
    let wire = to_canon(&policy).unwrap();
    // Outro vocabulário: h(V) não confere.
    let other = logos_air_text::parse_document("vocab v1 { sort A }", &Default::default())
        .unwrap()
        .vocabulary;
    assert_eq!(
        Policy::decode(&wire, &other).unwrap_err(),
        PolicyError::VocabularyMismatch
    );
    // Bytes extras ou truncados.
    let mut extra = wire.clone();
    extra.push(0);
    assert!(Policy::decode(&extra, &v).is_err());
    assert!(Policy::decode(&wire[..wire.len() - 1], &v).is_err());
    let value: Value = from_canon(&wire).unwrap();
    let Value::Array(top) = &value else {
        panic!("array")
    };
    let mutate = |change: &dyn Fn(&mut Vec<Value>)| {
        let mut items = top.items().to_vec();
        change(&mut items);
        to_canon(&Value::Array(logos_canon::Array::new(items).unwrap())).unwrap()
    };
    // Versão do formato, versão 0, aridade, tipo do h(V).
    assert!(Policy::decode(&mutate(&|i| i[0] = Value::Unsigned(2)), &v).is_err());
    assert_eq!(
        Policy::decode(&mutate(&|i| i[1] = Value::Unsigned(0)), &v).unwrap_err(),
        PolicyError::ZeroVersion
    );
    assert!(Policy::decode(&mutate(&|i| i.push(Value::Unsigned(0))), &v).is_err());
    assert!(Policy::decode(&mutate(&|i| i[2] = Value::Unsigned(1)), &v).is_err());
    assert!(Policy::decode(&mutate(&|i| i[2] = Value::Bytes(vec![0; 31])), &v).is_err());
    assert!(Policy::decode(&mutate(&|i| i[3] = Value::Unsigned(1)), &v).is_err());
    // Regras: fora de ordem, classe desconhecida, aridade, id não-texto.
    let rules_of = |change: &dyn Fn(&mut Vec<Value>)| {
        mutate(&|i| {
            let Value::Array(rules) = &i[3] else { return };
            let mut rules = rules.items().to_vec();
            change(&mut rules);
            i[3] = Value::Array(logos_canon::Array::new(rules).unwrap());
        })
    };
    assert_eq!(
        Policy::decode(&rules_of(&|r| r.reverse()), &v).unwrap_err(),
        PolicyError::UnsortedRules
    );
    let edit_rule = |index: usize, field: usize, value: Value| {
        rules_of(&|r| {
            let Value::Array(rule) = &r[index] else {
                return;
            };
            let mut fields = rule.items().to_vec();
            fields[field] = value.clone();
            r[index] = Value::Array(logos_canon::Array::new(fields).unwrap());
        })
    };
    // Raiz que não é array, versão que não é inteiro, classe/comentário de tipo errado.
    assert!(Policy::decode(&to_canon(&Value::Unsigned(1)).unwrap(), &v).is_err());
    assert!(Policy::decode(&mutate(&|i| i[1] = Value::Bytes(vec![])), &v).is_err());
    assert!(Policy::decode(&edit_rule(0, 1, Value::Bytes(vec![])), &v).is_err());
    assert!(Policy::decode(&edit_rule(0, 3, Value::Unsigned(1)), &v).is_err());
    assert!(Policy::decode(&edit_rule(0, 1, Value::Unsigned(9)), &v).is_err());
    assert!(Policy::decode(&edit_rule(0, 0, Value::Unsigned(1)), &v).is_err());
    assert!(Policy::decode(&edit_rule(0, 2, Value::Unsigned(1)), &v).is_err());
    assert!(Policy::decode(&rules_of(&|r| r[0] = Value::Unsigned(1)), &v).is_err());
    assert!(
        Policy::decode(
            &rules_of(&|r| {
                let Value::Array(rule) = &r[0] else { return };
                let mut fields = rule.items().to_vec();
                fields.pop();
                r[0] = Value::Array(logos_canon::Array::new(fields).unwrap());
            }),
            &v
        )
        .is_err()
    );
    // Acima do teto de bytes o decoder recusa antes de olhar o conteúdo.
    let oversized = vec![0u8; logos_policy::MAX_POLICY_BYTES + 1];
    assert!(Policy::decode(&oversized, &v).is_err());
    // Regras demais para o teto são recusadas já no decode.
    let template = top.items()[3].clone();
    let Value::Array(template) = template else {
        panic!("rules")
    };
    let rule0 = template.items()[0].clone();
    let many = rules_of(&|r| {
        *r = vec![rule0.clone(); logos_policy::MAX_RULES + 1];
    });
    assert!(Policy::decode(&many, &v).is_err());
    // Uma regra que deixou de ser válida (classe trocada) é recusada na revalidação.
    assert!(Policy::decode(&edit_rule(0, 1, Value::Unsigned(1)), &v).is_err());
}
