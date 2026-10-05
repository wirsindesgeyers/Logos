//! Testes de comportamento e cobertura de branches do sort-checker (Etapa 1.3).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::thread;

use logos_air::{
    Formula, FormulaNode, Limits, Sort, SortDecl, SortDomain, SortError, SortErrorKind, SortPath,
    Symbol, SymbolDecl, SymbolKind, SymbolResult, Term, TermNode, TermPath, TermPosition,
    Vocabulary, check,
};
use logos_canon::Text;

fn sort(name: &str) -> Sort {
    Sort {
        name: Text::new(name).unwrap(),
    }
}

fn sort_decl(name: &str) -> SortDecl {
    SortDecl {
        name: Text::new(name).unwrap(),
        domain: SortDomain::Open,
    }
}

fn symbol(name: &str, kind: SymbolKind, arity: Vec<Sort>, result: SymbolResult) -> Symbol {
    Symbol {
        name: Text::new(name).unwrap(),
        kind,
        arity,
        result,
    }
}

fn symbol_decl(sym: Symbol) -> SymbolDecl {
    SymbolDecl {
        signature: sym,
        grounding: logos_air::Grounding::Glossed(Text::new("test gloss").unwrap()),
        gloss_pt: Text::new("glosa de teste").unwrap(),
    }
}

fn test_vocab() -> Vocabulary {
    let sorts = vec![sort_decl("U"), sort_decl("V"), sort_decl("W")];
    let symbols = vec![
        symbol_decl(symbol(
            "c_u",
            SymbolKind::Const,
            vec![],
            SymbolResult::Sort(sort("U")),
        )),
        symbol_decl(symbol(
            "c_v",
            SymbolKind::Const,
            vec![],
            SymbolResult::Sort(sort("V")),
        )),
        symbol_decl(symbol(
            "c_w",
            SymbolKind::Const,
            vec![],
            SymbolResult::Sort(sort("W")),
        )),
        symbol_decl(symbol(
            "nullary_fn",
            SymbolKind::Func,
            vec![],
            SymbolResult::Sort(sort("U")),
        )),
        symbol_decl(symbol(
            "f_u",
            SymbolKind::Func,
            vec![sort("U")],
            SymbolResult::Sort(sort("U")),
        )),
        symbol_decl(symbol(
            "f_v",
            SymbolKind::Func,
            vec![sort("V")],
            SymbolResult::Sort(sort("V")),
        )),
        symbol_decl(symbol(
            "f_uv_w",
            SymbolKind::Func,
            vec![sort("U"), sort("V")],
            SymbolResult::Sort(sort("W")),
        )),
        symbol_decl(symbol("P_0", SymbolKind::Pred, vec![], SymbolResult::Bool)),
        symbol_decl(symbol(
            "P_u",
            SymbolKind::Pred,
            vec![sort("U")],
            SymbolResult::Bool,
        )),
        symbol_decl(symbol(
            "P_v",
            SymbolKind::Pred,
            vec![sort("V")],
            SymbolResult::Bool,
        )),
        symbol_decl(symbol(
            "P_w",
            SymbolKind::Pred,
            vec![sort("W")],
            SymbolResult::Bool,
        )),
        symbol_decl(symbol(
            "P_uv",
            SymbolKind::Pred,
            vec![sort("U"), sort("V")],
            SymbolResult::Bool,
        )),
        // Unused symbols with irregular signatures (allowed in vocabulary, but not sort-checked unless used):
        symbol_decl(symbol(
            "unused_pred_sort",
            SymbolKind::Pred,
            vec![],
            SymbolResult::Sort(sort("U")),
        )),
        symbol_decl(symbol(
            "unused_func_bool",
            SymbolKind::Func,
            vec![],
            SymbolResult::Bool,
        )),
        symbol_decl(symbol(
            "unused_const_with_arity",
            SymbolKind::Const,
            vec![sort("U")],
            SymbolResult::Sort(sort("U")),
        )),
    ];
    Vocabulary::new(1, sorts, symbols).unwrap()
}

fn term_var(index: u64, sort_name: &str) -> Term {
    Term::new(
        vec![TermNode::Var {
            index,
            sort: sort(sort_name),
        }],
        Limits::default(),
    )
    .unwrap()
}

fn term_const(sym: Symbol) -> Term {
    Term::new(
        vec![TermNode::App {
            symbol: sym,
            args: 0,
        }],
        Limits::default(),
    )
    .unwrap()
}

fn formula_leaf(node: FormulaNode) -> Formula {
    Formula::new(vec![node], Limits::default()).unwrap()
}

fn formula_pred(sym: Symbol, args: Vec<Term>) -> Formula {
    Formula::new(
        vec![FormulaNode::Pred { symbol: sym, args }],
        Limits::default(),
    )
    .unwrap()
}

fn formula_eq(left: Term, right: Term) -> Formula {
    Formula::new(vec![FormulaNode::Eq(left, right)], Limits::default()).unwrap()
}

#[test]
fn test_connectives_including_zero_children() {
    let vocab = test_vocab();

    // Leaf formulas: True and False
    assert_eq!(check(&vocab, &formula_leaf(FormulaNode::True)), Ok(()));
    assert_eq!(check(&vocab, &formula_leaf(FormulaNode::False)), Ok(()));

    // Connectives with zero children: And(0) and Or(0)
    assert_eq!(check(&vocab, &formula_leaf(FormulaNode::And(0))), Ok(()));
    assert_eq!(check(&vocab, &formula_leaf(FormulaNode::Or(0))), Ok(()));

    // Not
    let f_not = Formula::new(vec![FormulaNode::Not, FormulaNode::True], Limits::default()).unwrap();
    assert_eq!(check(&vocab, &f_not), Ok(()));

    // And with children
    let f_and = Formula::new(
        vec![FormulaNode::And(2), FormulaNode::True, FormulaNode::False],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(check(&vocab, &f_and), Ok(()));

    // Or with children
    let f_or = Formula::new(
        vec![FormulaNode::Or(1), FormulaNode::True],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(check(&vocab, &f_or), Ok(()));

    // Implies
    let f_implies = Formula::new(
        vec![FormulaNode::Implies, FormulaNode::True, FormulaNode::False],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(check(&vocab, &f_implies), Ok(()));

    // Iff
    let f_iff = Formula::new(
        vec![FormulaNode::Iff, FormulaNode::False, FormulaNode::True],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(check(&vocab, &f_iff), Ok(()));
}

#[test]
fn test_quantifier_unknown_sort() {
    let vocab = test_vocab();
    let unknown = sort("UNKNOWN_SORT");

    let f_forall = Formula::new(
        vec![FormulaNode::Forall(unknown.clone()), FormulaNode::True],
        Limits::default(),
    )
    .unwrap();
    let err = check(&vocab, &f_forall).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: None,
            },
            kind: SortErrorKind::UnknownSort,
        }
    );

    let f_exists = Formula::new(
        vec![FormulaNode::Exists(unknown), FormulaNode::True],
        Limits::default(),
    )
    .unwrap();
    let err = check(&vocab, &f_exists).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: None,
            },
            kind: SortErrorKind::UnknownSort,
        }
    );

    // Deep in formula prefix
    let f_nested = Formula::new(
        vec![
            FormulaNode::Not,
            FormulaNode::Forall(sort("UNKNOWN")),
            FormulaNode::True,
        ],
        Limits::default(),
    )
    .unwrap();
    let err = check(&vocab, &f_nested).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 1,
                term: None,
            },
            kind: SortErrorKind::UnknownSort,
        }
    );
}

#[test]
fn test_pred_unknown_symbol_and_signature_mismatch() {
    let vocab = test_vocab();

    // Unknown predicate symbol name
    let undeclared_sym = symbol(
        "undeclared_pred",
        SymbolKind::Pred,
        vec![],
        SymbolResult::Bool,
    );
    let f_unknown = formula_pred(undeclared_sym, vec![]);
    let err = check(&vocab, &f_unknown).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: None,
            },
            kind: SortErrorKind::UnknownSymbol,
        }
    );

    // Signature mismatch: arity sort differs (expected [U], provided [V])
    let sym_wrong_arity = symbol("P_u", SymbolKind::Pred, vec![sort("V")], SymbolResult::Bool);
    let c_v = term_const(symbol(
        "c_v",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("V")),
    ));
    let f_mismatch_arity = formula_pred(sym_wrong_arity, vec![c_v]);
    let err = check(&vocab, &f_mismatch_arity).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: None,
            },
            kind: SortErrorKind::SignatureMismatch,
        }
    );

    // Signature mismatch: result differs (expected Bool, provided Sort(U))
    let sym_wrong_result = symbol(
        "P_0",
        SymbolKind::Pred,
        vec![],
        SymbolResult::Sort(sort("U")),
    );
    let f_mismatch_res = formula_pred(sym_wrong_result, vec![]);
    let err = check(&vocab, &f_mismatch_res).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: None,
            },
            kind: SortErrorKind::SignatureMismatch,
        }
    );

    // Signature mismatch: declared kind differs (e.g. Func used where Pred was declared)
    let sym_wrong_kind = symbol("P_0", SymbolKind::Func, vec![], SymbolResult::Bool);
    let f_mismatch_kind = formula_pred(sym_wrong_kind, vec![]);
    let err = check(&vocab, &f_mismatch_kind).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: None,
            },
            kind: SortErrorKind::SignatureMismatch,
        }
    );
}

#[test]
fn test_pred_invalid_symbol_kind_and_result() {
    let vocab = test_vocab();

    // Vocabulary declares f_u as Func. Using exact signature of f_u as Predicate:
    let f_u_sym = symbol(
        "f_u",
        SymbolKind::Func,
        vec![sort("U")],
        SymbolResult::Sort(sort("U")),
    );
    let c_u = term_const(symbol(
        "c_u",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("U")),
    ));
    let f_pred_func = formula_pred(f_u_sym, vec![c_u.clone()]);
    let err = check(&vocab, &f_pred_func).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: None,
            },
            kind: SortErrorKind::InvalidSymbolKind,
        }
    );

    // Using exact signature of c_u (Const) as Predicate:
    let c_u_sym = symbol(
        "c_u",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("U")),
    );
    let f_pred_const = formula_pred(c_u_sym, vec![]);
    let err = check(&vocab, &f_pred_const).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: None,
            },
            kind: SortErrorKind::InvalidSymbolKind,
        }
    );

    // Using declared symbol with kind: Pred but result != Bool (unused_pred_sort declared in test_vocab):
    let bad_pred_sym = symbol(
        "unused_pred_sort",
        SymbolKind::Pred,
        vec![],
        SymbolResult::Sort(sort("U")),
    );
    let f_bad_res = formula_pred(bad_pred_sym, vec![]);
    let err = check(&vocab, &f_bad_res).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: None,
            },
            kind: SortErrorKind::InvalidSymbolResult,
        }
    );
}

#[test]
fn test_pred_arity_short_and_long() {
    let vocab = test_vocab();
    let p_uv = symbol(
        "P_uv",
        SymbolKind::Pred,
        vec![sort("U"), sort("V")],
        SymbolResult::Bool,
    );
    let c_u = term_const(symbol(
        "c_u",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("U")),
    ));
    let c_v = term_const(symbol(
        "c_v",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("V")),
    ));

    // Short arity: expected 2, passed 1
    let f_short = formula_pred(p_uv.clone(), vec![c_u.clone()]);
    let err = check(&vocab, &f_short).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: None,
            },
            kind: SortErrorKind::ArityMismatch {
                expected: 2,
                actual: 1,
            },
        }
    );

    // Long arity: expected 2, passed 3
    let f_long = formula_pred(p_uv, vec![c_u.clone(), c_v.clone(), c_u.clone()]);
    let err = check(&vocab, &f_long).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: None,
            },
            kind: SortErrorKind::ArityMismatch {
                expected: 2,
                actual: 3,
            },
        }
    );

    // Zero vs nonzero: P_0 expects 0, passed 1
    let p_0 = symbol("P_0", SymbolKind::Pred, vec![], SymbolResult::Bool);
    let f_p0_long = formula_pred(p_0, vec![c_u]);
    let err = check(&vocab, &f_p0_long).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: None,
            },
            kind: SortErrorKind::ArityMismatch {
                expected: 0,
                actual: 1,
            },
        }
    );
}

#[test]
fn test_pred_argument_sort_mismatch() {
    let vocab = test_vocab();
    let c_u = term_const(symbol(
        "c_u",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("U")),
    ));
    let c_v = term_const(symbol(
        "c_v",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("V")),
    ));

    // Arg 0 mismatch
    let p_u = symbol("P_u", SymbolKind::Pred, vec![sort("U")], SymbolResult::Bool);
    let f_mismatch_arg0 = formula_pred(p_u, vec![c_v.clone()]);
    let err = check(&vocab, &f_mismatch_arg0).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::PredicateArgument(0),
                    node: 0,
                }),
            },
            kind: SortErrorKind::SortMismatch {
                expected: sort("U"),
                actual: sort("V"),
            },
        }
    );

    // Arg 1 mismatch
    let p_uv = symbol(
        "P_uv",
        SymbolKind::Pred,
        vec![sort("U"), sort("V")],
        SymbolResult::Bool,
    );
    let f_mismatch_arg1 = formula_pred(p_uv, vec![c_u.clone(), c_u]);
    let err = check(&vocab, &f_mismatch_arg1).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::PredicateArgument(1),
                    node: 0,
                }),
            },
            kind: SortErrorKind::SortMismatch {
                expected: sort("V"),
                actual: sort("U"),
            },
        }
    );
}

#[test]
fn test_term_app_unknown_symbol_and_signature_mismatch() {
    let vocab = test_vocab();
    let undeclared_sym = symbol(
        "ghost_fn",
        SymbolKind::Func,
        vec![],
        SymbolResult::Sort(sort("U")),
    );
    let term_ghost = term_const(undeclared_sym);
    let c_u = term_const(symbol(
        "c_u",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("U")),
    ));

    // In Eq left
    let f_eq_left = formula_eq(term_ghost.clone(), c_u.clone());
    let err = check(&vocab, &f_eq_left).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::EqualityLeft,
                    node: 0,
                }),
            },
            kind: SortErrorKind::UnknownSymbol,
        }
    );

    // In Eq right
    let f_eq_right = formula_eq(c_u.clone(), term_ghost);
    let err = check(&vocab, &f_eq_right).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::EqualityRight,
                    node: 0,
                }),
            },
            kind: SortErrorKind::UnknownSymbol,
        }
    );

    // Signature mismatch in App (declared result U, used with result V)
    let f_u_wrong_result = symbol(
        "f_u",
        SymbolKind::Func,
        vec![sort("U")],
        SymbolResult::Sort(sort("V")),
    );
    let term_wrong_sig = Term::new(
        vec![
            TermNode::App {
                symbol: f_u_wrong_result,
                args: 1,
            },
            TermNode::App {
                symbol: symbol(
                    "c_u",
                    SymbolKind::Const,
                    vec![],
                    SymbolResult::Sort(sort("U")),
                ),
                args: 0,
            },
        ],
        Limits::default(),
    )
    .unwrap();
    let f_sig_err = formula_eq(term_wrong_sig, c_u);
    let err = check(&vocab, &f_sig_err).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::EqualityLeft,
                    node: 0,
                }),
            },
            kind: SortErrorKind::SignatureMismatch,
        }
    );
}

#[test]
fn test_term_app_invalid_kind_and_result() {
    let vocab = test_vocab();
    let c_u = term_const(symbol(
        "c_u",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("U")),
    ));

    // Pred symbol in App position
    let p_u_sym = symbol("P_u", SymbolKind::Pred, vec![sort("U")], SymbolResult::Bool);
    let term_pred_as_app = Term::new(
        vec![
            TermNode::App {
                symbol: p_u_sym,
                args: 1,
            },
            TermNode::App {
                symbol: symbol(
                    "c_u",
                    SymbolKind::Const,
                    vec![],
                    SymbolResult::Sort(sort("U")),
                ),
                args: 0,
            },
        ],
        Limits::default(),
    )
    .unwrap();
    let f_invalid_kind = formula_eq(term_pred_as_app, c_u.clone());
    let err = check(&vocab, &f_invalid_kind).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::EqualityLeft,
                    node: 0,
                }),
            },
            kind: SortErrorKind::InvalidSymbolKind,
        }
    );

    // Func returning Bool in App position (unused_func_bool declared in test_vocab)
    let bad_func_sym = symbol(
        "unused_func_bool",
        SymbolKind::Func,
        vec![],
        SymbolResult::Bool,
    );
    let term_bool_func = term_const(bad_func_sym);
    let f_invalid_res = formula_eq(term_bool_func, c_u);
    let err = check(&vocab, &f_invalid_res).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::EqualityLeft,
                    node: 0,
                }),
            },
            kind: SortErrorKind::InvalidSymbolResult,
        }
    );
}

#[test]
fn test_const_invalid_arity_nonzero() {
    let vocab = test_vocab();
    let c_u = term_const(symbol(
        "c_u",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("U")),
    ));

    // Symbol declared as Const but with arity [U] (unused_const_with_arity in test_vocab)
    let bad_const_sym = symbol(
        "unused_const_with_arity",
        SymbolKind::Const,
        vec![sort("U")],
        SymbolResult::Sort(sort("U")),
    );
    let term_bad_const = Term::new(
        vec![
            TermNode::App {
                symbol: bad_const_sym,
                args: 1,
            },
            TermNode::App {
                symbol: symbol(
                    "c_u",
                    SymbolKind::Const,
                    vec![],
                    SymbolResult::Sort(sort("U")),
                ),
                args: 0,
            },
        ],
        Limits::default(),
    )
    .unwrap();
    let f = formula_eq(term_bad_const, c_u);
    let err = check(&vocab, &f).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::EqualityLeft,
                    node: 0,
                }),
            },
            kind: SortErrorKind::InvalidConstantArity { actual: 1 },
        }
    );
}

#[test]
fn test_term_app_arity_short_and_long() {
    let vocab = test_vocab();
    let c_u_sym = symbol(
        "c_u",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("U")),
    );
    let c_u = term_const(c_u_sym.clone());

    // c_u has arity 0, but node specifies args: 1
    let term_long_const = Term::new(
        vec![
            TermNode::App {
                symbol: c_u_sym,
                args: 1,
            },
            TermNode::App {
                symbol: symbol(
                    "c_u",
                    SymbolKind::Const,
                    vec![],
                    SymbolResult::Sort(sort("U")),
                ),
                args: 0,
            },
        ],
        Limits::default(),
    )
    .unwrap();
    let f_long = formula_eq(term_long_const, c_u.clone());
    let err = check(&vocab, &f_long).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::EqualityLeft,
                    node: 0,
                }),
            },
            kind: SortErrorKind::ArityMismatch {
                expected: 0,
                actual: 1,
            },
        }
    );

    // f_u expects 1 arg, but node specifies args: 0
    let f_u_sym = symbol(
        "f_u",
        SymbolKind::Func,
        vec![sort("U")],
        SymbolResult::Sort(sort("U")),
    );
    let term_short_fn = Term::new(
        vec![TermNode::App {
            symbol: f_u_sym,
            args: 0,
        }],
        Limits::default(),
    )
    .unwrap();
    let f_short = formula_eq(term_short_fn, c_u);
    let err = check(&vocab, &f_short).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::EqualityLeft,
                    node: 0,
                }),
            },
            kind: SortErrorKind::ArityMismatch {
                expected: 1,
                actual: 0,
            },
        }
    );
}

#[test]
fn test_var_unknown_sort() {
    let vocab = test_vocab();
    let bad_var = term_var(0, "GHOST_SORT");
    let c_u = term_const(symbol(
        "c_u",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("U")),
    ));
    let f = formula_eq(bad_var, c_u);
    let err = check(&vocab, &f).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::EqualityLeft,
                    node: 0,
                }),
            },
            kind: SortErrorKind::UnknownSort,
        }
    );
}

#[test]
fn test_unbound_variable_idx0_and_max() {
    let vocab = test_vocab();
    let c_u = term_const(symbol(
        "c_u",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("U")),
    ));

    // Var index 0 with 0 binders in scope
    let var0 = term_var(0, "U");
    let f0 = formula_eq(var0, c_u.clone());
    let err0 = check(&vocab, &f0).unwrap_err();
    assert_eq!(
        err0,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::EqualityLeft,
                    node: 0,
                }),
            },
            kind: SortErrorKind::UnboundVariable {
                index: 0,
                binders: 0,
            },
        }
    );

    // Var index u64::MAX with 1 binder in scope
    let var_max = term_var(u64::MAX, "U");
    let f_max = Formula::new(
        vec![
            FormulaNode::Forall(sort("U")),
            FormulaNode::Eq(var_max, c_u.clone()),
        ],
        Limits::default(),
    )
    .unwrap();
    let err_max = check(&vocab, &f_max).unwrap_err();
    assert_eq!(
        err_max,
        SortError {
            path: SortPath {
                formula_node: 1,
                term: Some(TermPath {
                    position: TermPosition::EqualityLeft,
                    node: 0,
                }),
            },
            kind: SortErrorKind::UnboundVariable {
                index: u64::MAX,
                binders: 1,
            },
        }
    );

    // Var index 2 with 2 binders in scope
    let var2 = term_var(2, "U");
    let f2 = Formula::new(
        vec![
            FormulaNode::Forall(sort("U")),
            FormulaNode::Forall(sort("U")),
            FormulaNode::Eq(var2, c_u),
        ],
        Limits::default(),
    )
    .unwrap();
    let err2 = check(&vocab, &f2).unwrap_err();
    assert_eq!(
        err2,
        SortError {
            path: SortPath {
                formula_node: 2,
                term: Some(TermPath {
                    position: TermPosition::EqualityLeft,
                    node: 0,
                }),
            },
            kind: SortErrorKind::UnboundVariable {
                index: 2,
                binders: 2,
            },
        }
    );
}

#[test]
fn test_var_binder_sort_mismatch() {
    let vocab = test_vocab();
    let c_u = term_const(symbol(
        "c_u",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("U")),
    ));

    // Binder is U, variable annotated sort is V
    let var_v = term_var(0, "V");
    let f = Formula::new(
        vec![FormulaNode::Forall(sort("U")), FormulaNode::Eq(var_v, c_u)],
        Limits::default(),
    )
    .unwrap();
    let err = check(&vocab, &f).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 1,
                term: Some(TermPath {
                    position: TermPosition::EqualityLeft,
                    node: 0,
                }),
            },
            kind: SortErrorKind::SortMismatch {
                expected: sort("U"),
                actual: sort("V"),
            },
        }
    );
}

#[test]
fn test_eq_well_typed_and_sort_mismatch() {
    let vocab = test_vocab();
    let c_u = term_const(symbol(
        "c_u",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("U")),
    ));
    let c_v = term_const(symbol(
        "c_v",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("V")),
    ));

    // Well-typed Eq
    let f_ok = formula_eq(c_u.clone(), c_u.clone());
    assert_eq!(check(&vocab, &f_ok), Ok(()));

    // Well-typed nullary function and constant
    let nullary_term = term_const(symbol(
        "nullary_fn",
        SymbolKind::Func,
        vec![],
        SymbolResult::Sort(sort("U")),
    ));
    let f_nullary_ok = formula_eq(nullary_term, c_u.clone());
    assert_eq!(check(&vocab, &f_nullary_ok), Ok(()));

    // Sort mismatch: Left is U, Right is V -> reported at Right root (node: 0)
    let f_mismatch = formula_eq(c_u, c_v);
    let err = check(&vocab, &f_mismatch).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::EqualityRight,
                    node: 0,
                }),
            },
            kind: SortErrorKind::SortMismatch {
                expected: sort("U"),
                actual: sort("V"),
            },
        }
    );
}

#[test]
fn test_eq_precedence_intrinsic_precedes_mismatch() {
    let vocab = test_vocab();
    let c_u = term_const(symbol(
        "c_u",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("U")),
    ));

    // Right term has sort V (which would mismatch U), but it is an unbound Var!
    // Intrinsic UnboundVariable MUST be reported, not SortMismatch.
    let unbound_v = term_var(0, "V");
    let f = formula_eq(c_u.clone(), unbound_v);
    let err = check(&vocab, &f).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::EqualityRight,
                    node: 0,
                }),
            },
            kind: SortErrorKind::UnboundVariable {
                index: 0,
                binders: 0,
            },
        }
    );

    // Right term has unknown symbol
    let unknown_sym = term_const(symbol(
        "ghost",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("V")),
    ));
    let f_sym = formula_eq(c_u, unknown_sym);
    let err_sym = check(&vocab, &f_sym).unwrap_err();
    assert_eq!(
        err_sym,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::EqualityRight,
                    node: 0,
                }),
            },
            kind: SortErrorKind::UnknownSymbol,
        }
    );
}

#[test]
fn test_eq_precedence_root_mismatch_precedes_children() {
    let vocab = test_vocab();
    let c_u = term_const(symbol(
        "c_u",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("U")),
    ));

    // Right term has root f_v (result V != U) with an unbound variable child!
    // Root sort mismatch (expected U, actual V) MUST be reported before checking child of f_v.
    let term_right = Term::new(
        vec![
            TermNode::App {
                symbol: symbol(
                    "f_v",
                    SymbolKind::Func,
                    vec![sort("V")],
                    SymbolResult::Sort(sort("V")),
                ),
                args: 1,
            },
            TermNode::Var {
                index: 99,
                sort: sort("V"),
            },
        ],
        Limits::default(),
    )
    .unwrap();
    let f = formula_eq(c_u, term_right);
    let err = check(&vocab, &f).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::EqualityRight,
                    node: 0,
                }),
            },
            kind: SortErrorKind::SortMismatch {
                expected: sort("U"),
                actual: sort("V"),
            },
        }
    );
}

#[test]
fn test_nested_app_child_sort_mismatch() {
    let vocab = test_vocab();

    // App f_u(c_v): child node 1 has actual V, expected U
    let term_child_mismatch = Term::new(
        vec![
            TermNode::App {
                symbol: symbol(
                    "f_u",
                    SymbolKind::Func,
                    vec![sort("U")],
                    SymbolResult::Sort(sort("U")),
                ),
                args: 1,
            },
            TermNode::App {
                symbol: symbol(
                    "c_v",
                    SymbolKind::Const,
                    vec![],
                    SymbolResult::Sort(sort("V")),
                ),
                args: 0,
            },
        ],
        Limits::default(),
    )
    .unwrap();
    let f = formula_pred(
        symbol("P_u", SymbolKind::Pred, vec![sort("U")], SymbolResult::Bool),
        vec![term_child_mismatch],
    );
    let err = check(&vocab, &f).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::PredicateArgument(0),
                    node: 1,
                }),
            },
            kind: SortErrorKind::SortMismatch {
                expected: sort("U"),
                actual: sort("V"),
            },
        }
    );

    // Binary function f_uv_w(c_u, c_u): second child (arg 1) expects V, actual is U
    let term_binary_mismatch = Term::new(
        vec![
            TermNode::App {
                symbol: symbol(
                    "f_uv_w",
                    SymbolKind::Func,
                    vec![sort("U"), sort("V")],
                    SymbolResult::Sort(sort("W")),
                ),
                args: 2,
            },
            TermNode::App {
                symbol: symbol(
                    "c_u",
                    SymbolKind::Const,
                    vec![],
                    SymbolResult::Sort(sort("U")),
                ),
                args: 0,
            },
            TermNode::App {
                symbol: symbol(
                    "c_u",
                    SymbolKind::Const,
                    vec![],
                    SymbolResult::Sort(sort("U")),
                ),
                args: 0,
            },
        ],
        Limits::default(),
    )
    .unwrap();
    let f_bin = formula_pred(
        symbol(
            "P_uv",
            SymbolKind::Pred,
            vec![sort("U"), sort("V")],
            SymbolResult::Bool,
        ),
        vec![
            term_const(symbol(
                "c_u",
                SymbolKind::Const,
                vec![],
                SymbolResult::Sort(sort("U")),
            )),
            term_const(symbol(
                "c_v",
                SymbolKind::Const,
                vec![],
                SymbolResult::Sort(sort("V")),
            )),
        ],
    );
    assert_eq!(check(&vocab, &f_bin), Ok(()));

    let f_bin_err = formula_pred(
        symbol("P_v", SymbolKind::Pred, vec![sort("V")], SymbolResult::Bool),
        vec![term_const(symbol(
            "c_v",
            SymbolKind::Const,
            vec![],
            SymbolResult::Sort(sort("V")),
        ))],
    );
    assert_eq!(check(&vocab, &f_bin_err), Ok(()));

    let c_w = term_const(symbol(
        "c_w",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("W")),
    ));
    let f_eq_bin = formula_eq(term_binary_mismatch, c_w);
    let err_bin = check(&vocab, &f_eq_bin).unwrap_err();
    assert_eq!(
        err_bin,
        SortError {
            path: SortPath {
                formula_node: 0,
                term: Some(TermPath {
                    position: TermPosition::EqualityLeft,
                    node: 2,
                }),
            },
            kind: SortErrorKind::SortMismatch {
                expected: sort("V"),
                actual: sort("U"),
            },
        }
    );
}

#[test]
fn test_multiple_nested_arguments_and_exact_paths() {
    let vocab = test_vocab();
    // Pred P_uv with:
    // arg 0: f_u(c_u) -> valid (2 nodes)
    // arg 1: f_v(c_u) -> node 0 is f_v, node 1 is c_u (expected V, actual U)
    let term_arg0 = Term::new(
        vec![
            TermNode::App {
                symbol: symbol(
                    "f_u",
                    SymbolKind::Func,
                    vec![sort("U")],
                    SymbolResult::Sort(sort("U")),
                ),
                args: 1,
            },
            TermNode::App {
                symbol: symbol(
                    "c_u",
                    SymbolKind::Const,
                    vec![],
                    SymbolResult::Sort(sort("U")),
                ),
                args: 0,
            },
        ],
        Limits::default(),
    )
    .unwrap();
    let term_arg1 = Term::new(
        vec![
            TermNode::App {
                symbol: symbol(
                    "f_v",
                    SymbolKind::Func,
                    vec![sort("V")],
                    SymbolResult::Sort(sort("V")),
                ),
                args: 1,
            },
            TermNode::App {
                symbol: symbol(
                    "c_u",
                    SymbolKind::Const,
                    vec![],
                    SymbolResult::Sort(sort("U")),
                ),
                args: 0,
            },
        ],
        Limits::default(),
    )
    .unwrap();

    let f = Formula::new(
        vec![
            FormulaNode::Not,
            FormulaNode::Not,
            FormulaNode::Pred {
                symbol: symbol(
                    "P_uv",
                    SymbolKind::Pred,
                    vec![sort("U"), sort("V")],
                    SymbolResult::Bool,
                ),
                args: vec![term_arg0, term_arg1],
            },
        ],
        Limits::default(),
    )
    .unwrap();

    let err = check(&vocab, &f).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 2,
                term: Some(TermPath {
                    position: TermPosition::PredicateArgument(1),
                    node: 1,
                }),
            },
            kind: SortErrorKind::SortMismatch {
                expected: sort("V"),
                actual: sort("U"),
            },
        }
    );
}

#[test]
fn test_scope_and_sibling_isolation() {
    let vocab = test_vocab();
    let p_u = symbol("P_u", SymbolKind::Pred, vec![sort("U")], SymbolResult::Bool);
    let p_v = symbol("P_v", SymbolKind::Pred, vec![sort("V")], SymbolResult::Bool);

    // Independent siblings: And([ Forall(U, P_u(x0)), Forall(V, P_v(x0)) ])
    let f_siblings_ok = Formula::new(
        vec![
            FormulaNode::And(2),
            FormulaNode::Forall(sort("U")),
            FormulaNode::Pred {
                symbol: p_u.clone(),
                args: vec![term_var(0, "U")],
            },
            FormulaNode::Forall(sort("V")),
            FormulaNode::Pred {
                symbol: p_v,
                args: vec![term_var(0, "V")],
            },
        ],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(check(&vocab, &f_siblings_ok), Ok(()));

    // Leaking across siblings: And([ Forall(U, P_u(x0)), P_u(x0) ])
    // Second sibling P_u(x0) must be unbound!
    let f_sibling_leak = Formula::new(
        vec![
            FormulaNode::And(2),
            FormulaNode::Forall(sort("U")),
            FormulaNode::Pred {
                symbol: p_u.clone(),
                args: vec![term_var(0, "U")],
            },
            FormulaNode::Pred {
                symbol: p_u.clone(),
                args: vec![term_var(0, "U")],
            },
        ],
        Limits::default(),
    )
    .unwrap();
    let err = check(&vocab, &f_sibling_leak).unwrap_err();
    assert_eq!(
        err,
        SortError {
            path: SortPath {
                formula_node: 3,
                term: Some(TermPath {
                    position: TermPosition::PredicateArgument(0),
                    node: 0,
                }),
            },
            kind: SortErrorKind::UnboundVariable {
                index: 0,
                binders: 0,
            },
        }
    );

    // Implies leaking: Implies(Forall(U, P_u(x0)), P_u(x0))
    let f_implies_leak = Formula::new(
        vec![
            FormulaNode::Implies,
            FormulaNode::Forall(sort("U")),
            FormulaNode::Pred {
                symbol: p_u.clone(),
                args: vec![term_var(0, "U")],
            },
            FormulaNode::Pred {
                symbol: p_u,
                args: vec![term_var(0, "U")],
            },
        ],
        Limits::default(),
    )
    .unwrap();
    let err_imp = check(&vocab, &f_implies_leak).unwrap_err();
    assert_eq!(
        err_imp,
        SortError {
            path: SortPath {
                formula_node: 3,
                term: Some(TermPath {
                    position: TermPosition::PredicateArgument(0),
                    node: 0,
                }),
            },
            kind: SortErrorKind::UnboundVariable {
                index: 0,
                binders: 0,
            },
        }
    );
}

#[test]
fn test_nesting_shadowing_and_bound_variables() {
    let vocab = test_vocab();
    let p_u = symbol("P_u", SymbolKind::Pred, vec![sort("U")], SymbolResult::Bool);
    let p_v = symbol("P_v", SymbolKind::Pred, vec![sort("V")], SymbolResult::Bool);
    let p_w = symbol("P_w", SymbolKind::Pred, vec![sort("W")], SymbolResult::Bool);

    // 3 nested quantifiers: Forall(U), Exists(V), Forall(W)
    // x0 is W, x1 is V, x2 is U
    let f_nested_ok = Formula::new(
        vec![
            FormulaNode::Forall(sort("U")),
            FormulaNode::Exists(sort("V")),
            FormulaNode::Forall(sort("W")),
            FormulaNode::And(3),
            FormulaNode::Pred {
                symbol: p_w,
                args: vec![term_var(0, "W")],
            },
            FormulaNode::Pred {
                symbol: p_v,
                args: vec![term_var(1, "V")],
            },
            FormulaNode::Pred {
                symbol: p_u.clone(),
                args: vec![term_var(2, "U")],
            },
        ],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(check(&vocab, &f_nested_ok), Ok(()));

    // Shadowing: Forall(U), Forall(U), x0 refers to inner U, x1 refers to outer U
    let f_shadowing = Formula::new(
        vec![
            FormulaNode::Forall(sort("U")),
            FormulaNode::Forall(sort("U")),
            FormulaNode::Eq(term_var(0, "U"), term_var(1, "U")),
        ],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(check(&vocab, &f_shadowing), Ok(()));

    // Bound variables in Eq
    let f_bound_eq = Formula::new(
        vec![
            FormulaNode::Forall(sort("U")),
            FormulaNode::Eq(term_var(0, "U"), term_var(0, "U")),
        ],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(check(&vocab, &f_bound_eq), Ok(()));
}

#[test]
fn test_unused_declarations_in_vocabulary_ignored() {
    let vocab = test_vocab();
    // test_vocab contains unused_pred_sort, unused_func_bool, unused_const_with_arity.
    // Checking a formula that does not use them must succeed without error!
    let c_u = term_const(symbol(
        "c_u",
        SymbolKind::Const,
        vec![],
        SymbolResult::Sort(sort("U")),
    ));
    let f = formula_eq(c_u.clone(), c_u);
    assert_eq!(check(&vocab, &f), Ok(()));
}

#[test]
fn test_deep_formula_and_term_chains_in_64kib_stack() {
    let vocab = test_vocab();

    let builder = thread::Builder::new()
        .name("sort-check-stack64kib".into())
        .stack_size(64 * 1024);

    let handle = builder
        .spawn(move || {
            let limits = Limits {
                max_depth: 3000,
                max_nodes: 10_000,
            };

            // Deep formula chain: 1000 Not ending in True
            let depth = 1000;
            let mut not_nodes = vec![FormulaNode::Not; depth];
            not_nodes.push(FormulaNode::True);
            let f_deep_not = Formula::new(not_nodes, limits).unwrap();
            assert_eq!(check(&vocab, &f_deep_not), Ok(()));

            // Deep binder chain: 500 Forall(U) ending in P_u(x499)
            let binders_count = 500;
            let mut binder_nodes = vec![FormulaNode::Forall(sort("U")); binders_count];
            binder_nodes.push(FormulaNode::Pred {
                symbol: symbol("P_u", SymbolKind::Pred, vec![sort("U")], SymbolResult::Bool),
                args: vec![
                    Term::new(
                        vec![TermNode::Var {
                            index: (binders_count - 1) as u64,
                            sort: sort("U"),
                        }],
                        limits,
                    )
                    .unwrap(),
                ],
            });
            let f_deep_binders = Formula::new(binder_nodes, limits).unwrap();
            assert_eq!(check(&vocab, &f_deep_binders), Ok(()));

            // Deep term chain: 500 f_u unary applications ending in c_u
            let app_count = 500;
            let mut term_nodes = Vec::with_capacity(app_count + 1);
            for _ in 0..app_count {
                term_nodes.push(TermNode::App {
                    symbol: symbol(
                        "f_u",
                        SymbolKind::Func,
                        vec![sort("U")],
                        SymbolResult::Sort(sort("U")),
                    ),
                    args: 1,
                });
            }
            term_nodes.push(TermNode::App {
                symbol: symbol(
                    "c_u",
                    SymbolKind::Const,
                    vec![],
                    SymbolResult::Sort(sort("U")),
                ),
                args: 0,
            });
            let deep_term = Term::new(term_nodes, limits).unwrap();
            let c_u = term_const(symbol(
                "c_u",
                SymbolKind::Const,
                vec![],
                SymbolResult::Sort(sort("U")),
            ));
            let f_deep_term = Formula::new(vec![FormulaNode::Eq(deep_term, c_u)], limits).unwrap();
            assert_eq!(check(&vocab, &f_deep_term), Ok(()));
        })
        .unwrap();

    handle.join().unwrap();
}
