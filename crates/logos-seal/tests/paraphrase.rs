//! Contrato paraphrase-pt/v1: textos exatos, ida e volta, recusas explícitas.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{adversarial, decl, sort, symbol, text, vocabulary};
use logos_air::{
    Formula, FormulaNode, Limits, SortDecl, SortDomain, SymbolKind, Term, TermNode, Vocabulary,
};
use logos_seal::{ParaphraseError, ParaphraseLimits, Paraphraser, parse_pt, render_pt};

fn owns_vocabulary() -> Vocabulary {
    vocabulary(
        &["Usuário", "Documento"],
        vec![
            (
                symbol("Owns", SymbolKind::Pred, &["Usuário", "Documento"], None),
                "{0} é dono de {1}",
            ),
            (
                symbol("CanRead", SymbolKind::Pred, &["Usuário", "Documento"], None),
                "{0} pode ler {1}",
            ),
            (
                symbol("alice", SymbolKind::Const, &[], Some("Usuário")),
                "Alice",
            ),
            (
                symbol("autor", SymbolKind::Func, &["Documento"], Some("Usuário")),
                "o autor de {0}",
            ),
        ],
    )
}

fn var(index: u64, s: &str) -> Term {
    Term::new(
        vec![TermNode::Var {
            index,
            sort: sort(s),
        }],
        Limits::default(),
    )
    .unwrap()
}

fn pred(v: &Vocabulary, name: &str, args: Vec<Term>) -> FormulaNode {
    let symbol = v
        .symbols()
        .iter()
        .find(|d| d.signature.name.as_str() == name)
        .unwrap()
        .signature
        .clone();
    FormulaNode::Pred { symbol, args }
}

/// ∀u:Usuário. ∀d:Documento. Owns(u,d) → CanRead(u,d)
fn owner_policy(v: &Vocabulary) -> Formula {
    Formula::new(
        vec![
            FormulaNode::Forall(sort("Usuário")),
            FormulaNode::Forall(sort("Documento")),
            FormulaNode::Implies,
            pred(v, "Owns", vec![var(1, "Usuário"), var(0, "Documento")]),
            pred(v, "CanRead", vec![var(1, "Usuário"), var(0, "Documento")]),
        ],
        Limits::default(),
    )
    .unwrap()
}

const OWNER_TEXT: &str = "para todo x1 do tipo ⟦Usuário⟧, vale que (para todo x2 do tipo ⟦Documento⟧, vale que (se (⟦⟦x1⟧ é dono de ⟦x2⟧⟧), então (⟦⟦x1⟧ pode ler ⟦x2⟧⟧)))";

#[test]
fn owner_policy_renders_exact_text_and_parses_back() {
    let v = owns_vocabulary();
    let formula = owner_policy(&v);
    let paraphrase = render_pt(&formula, &v).unwrap();
    assert_eq!(paraphrase.text(), OWNER_TEXT);
    assert_eq!(parse_pt(OWNER_TEXT, &v).unwrap(), formula);
    assert_eq!(
        paraphrase.vocabulary(),
        &logos_crypto::hash(&v).unwrap(),
        "h(V) ligado à paráfrase"
    );
}

#[test]
fn every_connective_has_its_fixed_phrase() {
    let v = owns_vocabulary();
    let alice = || {
        Term::new(
            vec![TermNode::App {
                symbol: v.symbols()[2].signature.clone(),
                args: 0,
            }],
            Limits::default(),
        )
        .unwrap()
    };
    let author_of = |doc: Term| {
        let mut nodes = vec![TermNode::App {
            symbol: v.symbols()[3].signature.clone(),
            args: 1,
        }];
        nodes.extend(doc.nodes().iter().cloned());
        Term::new(nodes, Limits::default()).unwrap()
    };
    let cases: Vec<(Vec<FormulaNode>, &str)> = vec![
        (vec![FormulaNode::True], "verdadeiro"),
        (vec![FormulaNode::False], "falso"),
        (vec![FormulaNode::And(0)], "vale cada uma de 0 condições"),
        (
            vec![FormulaNode::Or(0)],
            "vale pelo menos uma de 0 condições",
        ),
        (
            vec![FormulaNode::And(1), FormulaNode::True],
            "vale cada uma de 1 condição: (verdadeiro)",
        ),
        (
            vec![FormulaNode::Or(2), FormulaNode::True, FormulaNode::False],
            "vale pelo menos uma de 2 condições: (verdadeiro); (falso)",
        ),
        (
            vec![FormulaNode::Not, FormulaNode::False],
            "não é o caso que (falso)",
        ),
        (
            vec![FormulaNode::Iff, FormulaNode::True, FormulaNode::False],
            "(verdadeiro) se e somente se (falso)",
        ),
        (
            vec![
                FormulaNode::Exists(sort("Documento")),
                FormulaNode::Eq(author_of(var(0, "Documento")), alice()),
            ],
            "existe x1 do tipo ⟦Documento⟧ tal que (o valor ⟦o autor de ⟦x1⟧⟧ é igual a ⟦Alice⟧)",
        ),
    ];
    for (nodes, expected) in cases {
        let formula = Formula::new(nodes, Limits::default()).unwrap();
        let paraphrase = render_pt(&formula, &v).unwrap();
        assert_eq!(paraphrase.text(), expected);
        assert_eq!(parse_pt(expected, &v).unwrap(), formula);
    }
}

#[test]
fn hash_binds_grammar_renderer_vocabulary_and_text() {
    let v = owns_vocabulary();
    let p = render_pt(&owner_policy(&v), &v).unwrap();
    let q = render_pt(
        &Formula::new(vec![FormulaNode::True], Limits::default()).unwrap(),
        &v,
    )
    .unwrap();
    assert_ne!(p.digest().unwrap(), q.digest().unwrap());
    let other = vocabulary(
        &["Usuário", "Documento"],
        vec![
            (
                symbol("Owns", SymbolKind::Pred, &["Usuário", "Documento"], None),
                "{0} é dono de {1}",
            ),
            (
                symbol("CanRead", SymbolKind::Pred, &["Usuário", "Documento"], None),
                "{0} pode ler {1}",
            ),
            (
                symbol("alice", SymbolKind::Const, &[], Some("Usuário")),
                "Alice",
            ),
            (
                symbol("autor", SymbolKind::Func, &["Documento"], Some("Usuário")),
                "quem escreveu {0}",
            ),
        ],
    );
    let r = render_pt(&owner_policy(&other), &other).unwrap();
    assert_eq!(p.text(), r.text(), "mesmo texto, vocabulários diferentes");
    assert_ne!(
        p.digest().unwrap(),
        r.digest().unwrap(),
        "h(V) entra no hash"
    );
}

#[test]
fn vocabulary_without_templates_or_ambiguous_is_refused() {
    let v1 = Vocabulary::new(
        1,
        vec![SortDecl {
            name: text("U"),
            domain: SortDomain::Open,
        }],
        vec![decl(symbol("P", SymbolKind::Pred, &[], None))],
    )
    .unwrap();
    assert_eq!(
        Paraphraser::new(&v1).unwrap_err(),
        ParaphraseError::MissingTemplates
    );
    let same = vocabulary(
        &["U"],
        vec![
            (symbol("P", SymbolKind::Pred, &["U"], None), "{0} é ativo"),
            (symbol("Q", SymbolKind::Pred, &["U"], None), "{0} é ativo"),
        ],
    );
    assert_eq!(
        Paraphraser::new(&same).unwrap_err(),
        ParaphraseError::AmbiguousTemplates
    );
    let terms = vocabulary(
        &["U"],
        vec![
            (symbol("a", SymbolKind::Const, &[], Some("U")), "Ana"),
            (symbol("f", SymbolKind::Func, &[], Some("U")), "Ana"),
        ],
    );
    assert_eq!(
        Paraphraser::new(&terms).unwrap_err(),
        ParaphraseError::AmbiguousTemplates
    );
    let contexts = vocabulary(
        &["U"],
        vec![
            (symbol("a", SymbolKind::Const, &[], Some("U")), "Ana"),
            (symbol("P", SymbolKind::Pred, &[], None), "Ana"),
        ],
    );
    assert!(
        Paraphraser::new(&contexts).is_ok(),
        "predicado e termo vivem em contextos distintos"
    );
    let variable = vocabulary(
        &["U"],
        vec![(symbol("a", SymbolKind::Const, &[], Some("U")), "x7")],
    );
    assert_eq!(
        Paraphraser::new(&variable).unwrap_err(),
        ParaphraseError::VariableNameCollision
    );
    let bracket = vocabulary(&["U⟧"], vec![]);
    assert_eq!(
        Paraphraser::new(&bracket).unwrap_err(),
        ParaphraseError::UnrenderableSortName
    );
}

#[test]
fn ill_typed_or_open_formulas_are_not_rendered() {
    let v = owns_vocabulary();
    let open = Formula::new(
        vec![pred(
            &v,
            "Owns",
            vec![var(0, "Usuário"), var(1, "Documento")],
        )],
        Limits::default(),
    )
    .unwrap();
    assert!(matches!(
        render_pt(&open, &v),
        Err(ParaphraseError::Sort(_))
    ));
    let swapped = Formula::new(
        vec![
            FormulaNode::Forall(sort("Usuário")),
            FormulaNode::Forall(sort("Documento")),
            pred(&v, "Owns", vec![var(0, "Documento"), var(1, "Usuário")]),
        ],
        Limits::default(),
    )
    .unwrap();
    assert!(matches!(
        render_pt(&swapped, &v),
        Err(ParaphraseError::Sort(_))
    ));
}

#[test]
fn non_canonical_or_foreign_text_is_refused_with_offset() {
    let v = owns_vocabulary();
    let p = Paraphraser::new(&v).unwrap();
    let syntax = |s: &str| matches!(p.parse(s), Err(ParaphraseError::Syntax { .. }));
    assert!(syntax(&format!("{OWNER_TEXT} ")), "byte extra");
    assert!(syntax("Verdadeiro"), "maiúscula");
    assert!(
        syntax("vale cada uma de 01 condição: (verdadeiro)"),
        "zero à esquerda"
    );
    assert!(
        syntax("vale cada uma de 1 condições: (verdadeiro)"),
        "plural errado"
    );
    assert!(
        syntax("vale cada uma de 2 condições: (verdadeiro)"),
        "contagem"
    );
    assert!(syntax("não é o caso que verdadeiro"), "sem parênteses");
    assert!(
        syntax("para todo x2 do tipo ⟦Usuário⟧, vale que (verdadeiro)"),
        "ordinal"
    );
    assert!(syntax("(verdadeiro)"), "agrupamento solto");
    assert!(syntax(""), "vazio");
    assert!(syntax("⟦Alice"), "colchete não fechado");
    assert!(
        syntax("o valor ⟦Alice⟧⟧ é igual a ⟦Alice⟧"),
        "colchete sobrando"
    );
    assert_eq!(
        p.parse("verdadeiro e falso"),
        Err(ParaphraseError::Syntax { offset: 10 })
    );
    assert!(matches!(
        p.parse("⟦Alice é chefe⟧"),
        Err(ParaphraseError::UnknownTemplate { .. })
    ));
    assert!(matches!(
        p.parse("existe x1 do tipo ⟦Pessoa⟧ tal que (verdadeiro)"),
        Err(ParaphraseError::UnknownSort { .. })
    ));
    assert!(matches!(
        p.parse("existe x1 do tipo ⟦Usuário⟧ tal que (⟦⟦x2⟧ é dono de ⟦x1⟧⟧)"),
        Err(ParaphraseError::UnboundVariable { .. })
    ));
    assert!(
        matches!(
            p.parse("existe x1 do tipo ⟦Usuário⟧ tal que (⟦⟦x1⟧ é dono de ⟦x1⟧⟧)"),
            Err(ParaphraseError::Sort(_))
        ),
        "variável de outro sort"
    );
}

#[test]
fn byte_and_depth_limits_are_errors() {
    let v = owns_vocabulary();
    let small = ParaphraseLimits {
        max_bytes: 20,
        ..ParaphraseLimits::default()
    };
    let p = Paraphraser::with_limits(&v, small).unwrap();
    assert_eq!(p.parse(OWNER_TEXT), Err(ParaphraseError::TooLarge));
    assert_eq!(p.render(&owner_policy(&v)), Err(ParaphraseError::TooLarge));
    let deep: String = "não é o caso que (".repeat(200) + "verdadeiro" + &")".repeat(200);
    assert_eq!(parse_pt(&deep, &v), Err(ParaphraseError::DepthExceeded));
    let mut nodes = vec![FormulaNode::Not; 200];
    nodes.push(FormulaNode::True);
    let formula = Formula::new(
        nodes,
        Limits {
            max_depth: 1000,
            max_nodes: 1000,
        },
    )
    .unwrap();
    assert_eq!(render_pt(&formula, &v), Err(ParaphraseError::DepthExceeded));
}

#[test]
fn adversarial_templates_still_round_trip() {
    let v = adversarial();
    let p = Paraphraser::new(&v).unwrap();
    for seed in 0..2000 {
        let formula = common::random_formula(&v, seed, 6);
        let paraphrase = p.render(&formula).unwrap();
        assert_eq!(
            p.parse(paraphrase.text()).unwrap(),
            formula,
            "{}",
            paraphrase.text()
        );
    }
}
