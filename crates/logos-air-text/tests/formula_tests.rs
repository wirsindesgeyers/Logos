//! Testes de fórmulas AIR: sintaxe normal e explícita, precedência,
//! escopo nominal, shadowing, alpha-renomeação, tipagem postergada e erros malformados.

use logos_air::{FormulaNode, Vocabulary};
use logos_air_text::{ErrorKind, parse_formula, print_formula};
use logos_canon::to_canon;

fn make_test_vocabulary() -> Vocabulary {
    let text = r#"
vocab v1 {
  sort User
  sort Db
  const alice: User = glossed("Alice") gloss "Alice"
  func f(User): Db = glossed("f") gloss "f"
  pred Admin(User) = glossed("Admin") gloss "Admin"
  pred Owns(User, Db) = glossed("Owns") gloss "Owns"
  pred P() = glossed("P") gloss "P"
  pred Q() = glossed("Q") gloss "Q"
  pred R() = glossed("R") gloss "R"
}
"#;
    logos_air_text::parse_document(text, &logos_air_text::References::new())
        .expect("vocabulary setup should succeed")
        .vocabulary
}

#[test]
fn test_basic_atoms_and_connectives() {
    let vocab = make_test_vocabulary();

    let t = parse_formula("true", &vocab).expect("true");
    assert_eq!(t.nodes(), &[FormulaNode::True]);
    assert_eq!(print_formula(&t), "true");

    let f = parse_formula("false", &vocab).expect("false");
    assert_eq!(f.nodes(), &[FormulaNode::False]);
    assert_eq!(print_formula(&f), "false");

    let p = parse_formula("P()", &vocab).expect("P()");
    assert!(
        matches!(&p.nodes()[0], FormulaNode::Pred { symbol, args } if symbol.name.as_str() == "P" && args.is_empty())
    );

    let not_p = parse_formula("not P()", &vocab).expect("not P()");
    assert_eq!(not_p.nodes()[0], FormulaNode::Not);

    let and_pq = parse_formula("P() and Q()", &vocab).expect("P and Q");
    assert_eq!(and_pq.nodes()[0], FormulaNode::And(2));

    let or_pq = parse_formula("P() or Q()", &vocab).expect("P or Q");
    assert_eq!(or_pq.nodes()[0], FormulaNode::Or(2));

    let imp = parse_formula("P() -> Q()", &vocab).expect("P -> Q");
    assert_eq!(imp.nodes()[0], FormulaNode::Implies);

    let iff = parse_formula("P() <-> Q()", &vocab).expect("P <-> Q");
    assert_eq!(iff.nodes()[0], FormulaNode::Iff);
}

#[test]
fn test_precedence_and_associativity() {
    let vocab = make_test_vocabulary();

    // not P and Q  ==  (not P) and Q
    let f1 = parse_formula("not P() and Q()", &vocab).expect("not P and Q");
    let f1_paren = parse_formula("(not P()) and Q()", &vocab).expect("(not P) and Q");
    assert_eq!(f1, f1_paren);

    // P and Q or R  ==  (P and Q) or R
    let f2 = parse_formula("P() and Q() or R()", &vocab).expect("P and Q or R");
    let f2_paren = parse_formula("(P() and Q()) or R()", &vocab).expect("(P and Q) or R");
    assert_eq!(f2, f2_paren);

    // P or Q and R  ==  P or (Q and R)
    let f3 = parse_formula("P() or Q() and R()", &vocab).expect("P or Q and R");
    let f3_paren = parse_formula("P() or (Q() and R())", &vocab).expect("P or (Q and R)");
    assert_eq!(f3, f3_paren);

    // P -> Q -> R  ==  P -> (Q -> R) (associatividade à direita)
    let f4 = parse_formula("P() -> Q() -> R()", &vocab).expect("P -> Q -> R");
    let f4_paren = parse_formula("P() -> (Q() -> R())", &vocab).expect("P -> (Q -> R)");
    assert_eq!(f4, f4_paren);

    // P <-> Q <-> R  ==  (P <-> Q) <-> R (associatividade à esquerda)
    let f5 = parse_formula("P() <-> Q() <-> R()", &vocab).expect("P <-> Q <-> R");
    let f5_paren = parse_formula("(P() <-> Q()) <-> R()", &vocab).expect("(P <-> Q) <-> R");
    assert_eq!(f5, f5_paren);

    // P <-> Q -> R  ==  P <-> (Q -> R)
    let f6 = parse_formula("P() <-> Q() -> R()", &vocab).expect("P <-> Q -> R");
    let f6_paren = parse_formula("P() <-> (Q() -> R())", &vocab).expect("P <-> (Q -> R)");
    assert_eq!(f6, f6_paren);
}

#[test]
fn test_quantifiers_and_binder_scoping() {
    let vocab = make_test_vocabulary();

    // forall u:User. Admin(u)
    let f = parse_formula("forall u:User. Admin(u)", &vocab).expect("forall");
    assert!(matches!(&f.nodes()[0], FormulaNode::Forall(sort) if sort.name.as_str() == "User"));
    assert!(
        matches!(&f.nodes()[1], FormulaNode::Pred { symbol, args } if symbol.name.as_str() == "Admin" && args.len() == 1)
    );

    // Quantifier consumes until external delimiter:
    // forall u:User. Admin(u) -> exists d:Db. Owns(u, d)
    let f_nested = parse_formula("forall u:User. Admin(u) -> exists d:Db. Owns(u, d)", &vocab)
        .expect("nested quantifier");

    assert!(matches!(&f_nested.nodes()[0], FormulaNode::Forall(_)));
    assert_eq!(f_nested.nodes()[1], FormulaNode::Implies);
    assert!(matches!(&f_nested.nodes()[2], FormulaNode::Pred { .. }));
    assert!(matches!(&f_nested.nodes()[3], FormulaNode::Exists(_)));
    assert!(matches!(&f_nested.nodes()[4], FormulaNode::Pred { .. }));
}

#[test]
fn test_nominal_shadowing_and_alpha_renaming() {
    let vocab = make_test_vocabulary();

    // Alpha-renaming: binder names do not affect canonical bytes!
    let fx = parse_formula("forall x:User. Admin(x)", &vocab).expect("alpha x");
    let fy = parse_formula("forall y:User. Admin(y)", &vocab).expect("alpha y");
    assert_eq!(fx, fy);
    assert_eq!(
        to_canon(&fx).expect("canon fx"),
        to_canon(&fy).expect("canon fy")
    );

    // Shadowing: inner x shadows outer x
    // forall x:User. (forall x:Db. P()) and Admin(x)
    let f_shadow =
        parse_formula("forall x:User. (forall x:Db. P()) and Admin(x)", &vocab).expect("shadowing");
    // Admin(x) in outer scope should still refer to outer x (index 0, sort User)
    let printed = print_formula(&f_shadow);
    let roundtrip = parse_formula(&printed, &vocab).expect("shadow roundtrip");
    assert_eq!(f_shadow, roundtrip);
}

#[test]
fn test_typing_deliberately_deferred() {
    let vocab = make_test_vocabulary();

    // Ill-sorted application: Db passed where User expected
    // Sort-check will reject later, but parser must succeed!
    let ill_typed = parse_formula("forall x:Db. Admin(x)", &vocab).expect("ill-typed");
    let printed = print_formula(&ill_typed);
    let roundtrip = parse_formula(&printed, &vocab).expect("ill-typed roundtrip");
    assert_eq!(ill_typed, roundtrip);

    // Undeclared sort in quantifier: accepted nominally for round-trip!
    let undeclared_sort =
        parse_formula("forall x:UnknownSort. P()", &vocab).expect("undeclared sort");
    let printed_u = print_formula(&undeclared_sort);
    let roundtrip_u = parse_formula(&printed_u, &vocab).expect("undeclared sort roundtrip");
    assert_eq!(undeclared_sort, roundtrip_u);

    // Raw explicit symbol and term: arity mismatch, ill-typed combination
    let raw = "pred(symbol(\"Weird\", pred, [\"User\", \"Db\"], bool), [var(99, \"Fake\")])";
    let raw_f = parse_formula(raw, &vocab).expect("raw explicit");
    let printed_raw = print_formula(&raw_f);
    let roundtrip_raw = parse_formula(&printed_raw, &vocab).expect("raw roundtrip");
    assert_eq!(raw_f, roundtrip_raw);
}

#[test]
fn test_raw_empty_and_nary_connectives() {
    let vocab = make_test_vocabulary();

    // and([]) and or([])
    let and_empty = parse_formula("and([])", &vocab).expect("and empty");
    assert_eq!(and_empty.nodes(), &[FormulaNode::And(0)]);
    assert_eq!(print_formula(&and_empty), "and([])");
    assert_eq!(parse_formula("and([])", &vocab).unwrap(), and_empty);

    let or_empty = parse_formula("or([])", &vocab).expect("or empty");
    assert_eq!(or_empty.nodes(), &[FormulaNode::Or(0)]);
    assert_eq!(print_formula(&or_empty), "or([])");

    // and([P, Q, R]) (3 children)
    let and3 = parse_formula("and([P(), Q(), R()])", &vocab).expect("and 3");
    assert_eq!(and3.nodes()[0], FormulaNode::And(3));
    let printed3 = print_formula(&and3);
    assert_eq!(parse_formula(&printed3, &vocab).unwrap(), and3);
}

#[test]
fn test_malformed_inputs() {
    let vocab = make_test_vocabulary();

    // Unrecognized token
    let err1 = parse_formula("P() @ Q()", &vocab).unwrap_err();
    assert_eq!(err1.kind, ErrorKind::Syntax);

    // Incomplete operator '-'
    let err2 = parse_formula("P() - Q()", &vocab).unwrap_err();
    assert_eq!(err2.kind, ErrorKind::Syntax);

    // Unterminated string
    let err3 = parse_formula("pred(symbol(\"unclosed, pred, [], bool), [])", &vocab).unwrap_err();
    assert_eq!(err3.kind, ErrorKind::Syntax);

    // Invalid escape sequence \q
    let err4 = parse_formula("pred(symbol(\"bad\\q\", pred, [], bool), [])", &vocab).unwrap_err();
    assert_eq!(err4.kind, ErrorKind::Syntax);

    // Unescaped control character in string literal
    let err5 = parse_formula("pred(symbol(\"ctrl\x01\", pred, [], bool), [])", &vocab).unwrap_err();
    assert_eq!(err5.kind, ErrorKind::InvalidText);

    // Isolated high surrogate
    let err6 = parse_formula("pred(symbol(\"\\uD800\", pred, [], bool), [])", &vocab).unwrap_err();
    assert_eq!(err6.kind, ErrorKind::InvalidText);

    // Isolated low surrogate
    let err7 = parse_formula("pred(symbol(\"\\uDC00\", pred, [], bool), [])", &vocab).unwrap_err();
    assert_eq!(err7.kind, ErrorKind::InvalidText);

    // Non-NFC text (decomposed e + acute)
    let err8 = parse_formula("pred(symbol(\"e\\u0301\", pred, [], bool), [])", &vocab).unwrap_err();
    assert_eq!(err8.kind, ErrorKind::InvalidText);

    // Integer overflow > u64::MAX
    let err9 = parse_formula("var(18446744073709551616, User) = var(0, User)", &vocab).unwrap_err();
    assert_eq!(err9.kind, ErrorKind::LimitExceeded);

    // Unknown name without binder and not in vocabulary
    let err10 = parse_formula("MissingSymbol(alice)", &vocab).unwrap_err();
    assert_eq!(err10.kind, ErrorKind::UnknownName);

    // Free variable without binder
    let err11 = parse_formula("Admin(free_variable)", &vocab).unwrap_err();
    assert_eq!(err11.kind, ErrorKind::UnknownName);

    // Trailing input
    let err12 = parse_formula("true ; false", &vocab).unwrap_err();
    assert_eq!(err12.kind, ErrorKind::Syntax);
}

#[test]
fn test_excessive_textual_nesting_is_rejected_without_stack_overflow() {
    let vocab = make_test_vocabulary();

    // Grupos transparentes válidos: 200 e 500 parênteses são aceitos sem estouro de pilha.
    let mut text200 = String::new();
    for _ in 0..200 {
        text200.push('(');
    }
    text200.push_str("true");
    for _ in 0..200 {
        text200.push(')');
    }
    let f200 = parse_formula(&text200, &vocab).expect("200 nested parentheses accepted");
    assert_eq!(f200.nodes(), &[FormulaNode::True]);

    let mut text500 = String::new();
    for _ in 0..500 {
        text500.push('(');
    }
    text500.push_str("true");
    for _ in 0..500 {
        text500.push(')');
    }
    let f500 = parse_formula(&text500, &vocab).expect("500 nested parentheses accepted");
    assert_eq!(f500.nodes(), &[FormulaNode::True]);

    // Caso adversário grande (10.000 parênteses): nesting > 4096 retorna LimitExceeded sem abortar.
    let mut text10000 = String::new();
    for _ in 0..10_000 {
        text10000.push('(');
    }
    text10000.push_str("true");
    for _ in 0..10_000 {
        text10000.push(')');
    }
    let err10000 = parse_formula(&text10000, &vocab).unwrap_err();
    assert_eq!(err10000.kind, ErrorKind::LimitExceeded);

    // Cadeia de implicação associativa à direita com 10.000 conectivos retorna erro controlado sem abortar.
    let mut text_imp = String::from("P()");
    for _ in 0..10_000 {
        text_imp.push_str(" -> P()");
    }
    let err_imp = parse_formula(&text_imp, &vocab).unwrap_err();
    assert_eq!(err_imp.kind, ErrorKind::LimitExceeded);

    // Cadeia unária de 10.000 'not's retorna erro controlado sem abortar.
    let mut text_unary = String::new();
    for _ in 0..10_000 {
        text_unary.push_str("not ");
    }
    text_unary.push_str("true");
    let err_unary = parse_formula(&text_unary, &vocab).unwrap_err();
    assert_eq!(err_unary.kind, ErrorKind::LimitExceeded);
}

#[test]
fn test_logical_depth_limits() {
    let vocab = make_test_vocabulary();

    // Depth 128: 127 'not' seguido de 'true' tem profundidade lógica 128 (raiz profundidade 1).
    let mut depth128_text = String::new();
    for _ in 0..127 {
        depth128_text.push_str("not(");
    }
    depth128_text.push_str("true");
    for _ in 0..127 {
        depth128_text.push(')');
    }
    let f128 = parse_formula(&depth128_text, &vocab).expect("depth 128 Not must be accepted");
    assert_eq!(f128.depth(), 128);

    // Depth 129: 128 'not' seguido de 'true' tem profundidade lógica 129 -> rejeitado com LimitExceeded!
    let mut depth129_text = String::new();
    for _ in 0..128 {
        depth129_text.push_str("not(");
    }
    depth129_text.push_str("true");
    for _ in 0..128 {
        depth129_text.push(')');
    }
    let err129 = parse_formula(&depth129_text, &vocab).unwrap_err();
    assert_eq!(err129.kind, ErrorKind::LimitExceeded);

    // Quantificadores depth 128: 127 'forall' seguido de 'true'.
    let mut q128_text = String::new();
    for i in 0..127 {
        q128_text.push_str(&format!("forall u{i}:User. "));
    }
    q128_text.push_str("true");
    let q128 = parse_formula(&q128_text, &vocab).expect("depth 128 Quantifier must be accepted");
    assert_eq!(q128.depth(), 128);

    // Quantificadores depth 129: 128 'forall' seguido de 'true' -> rejeitado com LimitExceeded!
    let mut q129_text = String::new();
    for i in 0..128 {
        q129_text.push_str(&format!("forall u{i}:User. "));
    }
    q129_text.push_str("true");
    let q_err = parse_formula(&q129_text, &vocab).unwrap_err();
    assert_eq!(q_err.kind, ErrorKind::LimitExceeded);

    // Embedded App chains depth 128:
    // Admin(f(f(...(alice)...)))
    // Admin é Pred (profundidade 1 da fórmula).
    // O termo embutido tem 126 funções f aninhadas sobre alice -> profundidade do termo = 127.
    // Profundidade lógica total = 1 + 127 = 128 -> aceito!
    let mut app128_text = String::from("Admin(");
    for _ in 0..126 {
        app128_text.push_str("f(");
    }
    app128_text.push_str("alice");
    for _ in 0..126 {
        app128_text.push(')');
    }
    app128_text.push(')');
    let f128_app =
        parse_formula(&app128_text, &vocab).expect("depth 128 App chain must be accepted");
    assert_eq!(f128_app.depth(), 128);

    // Embedded App chains depth 129:
    // 127 funções f aninhadas sobre alice -> profundidade do termo = 128.
    // Profundidade lógica total = 1 + 128 = 129 -> rejeitado com LimitExceeded!
    let mut app129_text = String::from("Admin(");
    for _ in 0..127 {
        app129_text.push_str("f(");
    }
    app129_text.push_str("alice");
    for _ in 0..127 {
        app129_text.push(')');
    }
    app129_text.push(')');
    let app_err = parse_formula(&app129_text, &vocab).unwrap_err();
    assert_eq!(app_err.kind, ErrorKind::LimitExceeded);
}

#[test]
fn test_long_left_associative_chain_with_bad_tail_returns_controlled_error() {
    let vocab = make_test_vocabulary();

    // 5000 'and's em cadeia associativa à esquerda com token inválido no final.
    // Testa que o descarte da árvore/arena em caso de erro não causa estouro de pilha.
    let mut text = String::from("P()");
    for _ in 0..5000 {
        text.push_str(" and P()");
    }
    text.push_str(" and @");

    let err = parse_formula(&text, &vocab).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Syntax);
}

#[test]
fn test_identifiers_with_v_prefix() {
    let vocab = make_test_vocabulary();

    // Identificadores iniciados por v (v1, v01, v12name) são nomes válidos sem aspas.
    let f1 = parse_formula("forall v1:User. Admin(v1)", &vocab).expect("v1 binder");
    let f01 = parse_formula("forall v01:User. Admin(v01)", &vocab).expect("v01 binder");
    let f12 = parse_formula("forall v12name:User. Admin(v12name)", &vocab).expect("v12name binder");

    assert_eq!(f1, f01);
    assert_eq!(f1, f12);
}

#[test]
fn test_binder_and_symbol_v1_scoping() {
    let doc_text = r#"
vocab v1 {
  sort User
  const v1: User = glossed("symbol v1") gloss "symbol v1"
  pred P(User)
}
"#;
    let doc = logos_air_text::parse_document(doc_text, &logos_air_text::References::new())
        .expect("doc with symbol v1");

    // Fora de binder: v1 resolve para o símbolo declarado v1
    let outside = parse_formula("P(v1)", &doc.vocabulary).expect("outside");
    assert!(
        matches!(&outside.nodes()[0], FormulaNode::Pred { symbol, args } if symbol.name.as_str() == "P" && args.len() == 1)
    );

    // Dentro do binder: v1 sem parênteses resolve para a variável ligada v1
    let inside = parse_formula("forall v1:User. P(v1)", &doc.vocabulary).expect("inside");
    assert!(matches!(&inside.nodes()[0], FormulaNode::Forall(_)));

    // Aplicação com parênteses v1() não é interceptada por binder e resolve para o símbolo
    let paren = parse_formula("forall v1:User. P(v1())", &doc.vocabulary).expect("paren");
    assert_ne!(inside, paren);
}

#[test]
fn test_grouping_semantics() {
    let vocab = make_test_vocabulary();

    // ((true) and false)
    let f1 = parse_formula("((true) and false)", &vocab).expect("((true) and false)");
    assert_eq!(
        f1.nodes(),
        &[FormulaNode::And(2), FormulaNode::True, FormulaNode::False]
    );

    // ((true) -> false) <-> true
    let f2 =
        parse_formula("((true) -> false) <-> true", &vocab).expect("((true) -> false) <-> true");
    assert_eq!(
        f2.nodes(),
        &[
            FormulaNode::Iff,
            FormulaNode::Implies,
            FormulaNode::True,
            FormulaNode::False,
            FormulaNode::True,
        ]
    );
}

#[test]
fn test_shadowing_sibling_and_scoping() {
    let vocab = make_test_vocabulary();

    // Irmãos de quantificador restauram escopo e não vazam binders
    let f_sibling = parse_formula(
        "(forall u:User. Admin(u)) and (forall u:Db. exists d:Db. Owns(u, d))",
        &vocab,
    )
    .expect("sibling quantifiers");
    assert!(matches!(&f_sibling.nodes()[0], FormulaNode::And(2)));
    assert!(matches!(&f_sibling.nodes()[1], FormulaNode::Forall(_)));
    assert!(matches!(&f_sibling.nodes()[3], FormulaNode::Forall(_)));

    // Shadowing aninhado com irmãos subsequentes
    let f_nested_shadow = parse_formula(
        "(forall u:User. forall u:Db. Owns(u, u)) and forall u:User. Admin(u)",
        &vocab,
    )
    .expect("nested shadow with sibling");
    assert!(matches!(&f_nested_shadow.nodes()[0], FormulaNode::And(2)));
}
