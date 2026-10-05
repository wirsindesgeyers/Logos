//! Testes de comportamento de Vocabulary: DAG, ciclos, grounding, compatibilidade e imutabilidade.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::thread;

use logos_air::{
    AirError, AttestorId, Compatibility, DomainSource, FactRef, Formula, FormulaNode, Grounding,
    Limits, PolicyRef, Sort, SortDecl, SortDomain, Symbol, SymbolDecl, SymbolKind, SymbolResult,
    Term, TermNode, Vocabulary, VocabularyError,
};
use logos_canon::{Text, to_canon};
use logos_crypto::hash;

fn sort(name: &str) -> Sort {
    Sort {
        name: Text::new(name).unwrap(),
    }
}

fn sort_decl_open(name: &str) -> SortDecl {
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

fn const_sym(name: &str, sort_name: &str) -> Symbol {
    symbol(
        name,
        SymbolKind::Const,
        Vec::new(),
        SymbolResult::Sort(sort(sort_name)),
    )
}

fn pred_sym(name: &str, arity: Vec<Sort>) -> Symbol {
    symbol(name, SymbolKind::Pred, arity, SymbolResult::Bool)
}

fn func_sym(name: &str, arity: Vec<Sort>, result_sort: &str) -> Symbol {
    symbol(
        name,
        SymbolKind::Func,
        arity,
        SymbolResult::Sort(sort(result_sort)),
    )
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

fn formula_true() -> Formula {
    Formula::new(vec![FormulaNode::True], Limits::default()).unwrap()
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

fn grounding_gloss(text: &str) -> Grounding {
    Grounding::Glossed(Text::new(text).unwrap())
}

fn grounding_anchored(attestor: &str, procedure: &str) -> Grounding {
    Grounding::Anchored {
        attestor: AttestorId(Text::new(attestor).unwrap()),
        procedure: Text::new(procedure).unwrap(),
    }
}

fn symbol_decl(sym: Symbol, grounding: Grounding, gloss: &str) -> SymbolDecl {
    SymbolDecl {
        signature: sym,
        grounding,
        gloss_pt: Text::new(gloss).unwrap(),
    }
}

#[test]
fn cycle_self_rejected() {
    let p = pred_sym("P", Vec::new());
    let decl = symbol_decl(
        p.clone(),
        Grounding::Defined(formula_pred(p, Vec::new())),
        "self cycle",
    );
    let result = Vocabulary::new(1, Vec::new(), vec![decl]);
    assert_eq!(result, Err(VocabularyError::CircularDefinition));
}

#[test]
fn cycle_mutually_dependent_rejected() {
    let p = pred_sym("P", Vec::new());
    let q = pred_sym("Q", Vec::new());

    let decl_p = symbol_decl(
        p.clone(),
        Grounding::Defined(formula_pred(q.clone(), Vec::new())),
        "P depends on Q",
    );
    let decl_q = symbol_decl(
        q,
        Grounding::Defined(formula_pred(p, Vec::new())),
        "Q depends on P",
    );

    let result = Vocabulary::new(1, Vec::new(), vec![decl_p, decl_q]);
    assert_eq!(result, Err(VocabularyError::CircularDefinition));
}

#[test]
fn cycle_through_eq_terms_rejected() {
    let u = sort_decl_open("U");
    let f = func_sym("f", Vec::new(), "U");
    let g = func_sym("g", Vec::new(), "U");

    // f is defined as Eq(g(), var(0))
    let def_f = formula_eq(term_const(g.clone()), term_var(0, "U"));
    // g is defined as Eq(f(), var(0))
    let def_g = formula_eq(term_const(f.clone()), term_var(0, "U"));

    let decl_f = symbol_decl(f, Grounding::Defined(def_f), "f uses g in eq");
    let decl_g = symbol_decl(g, Grounding::Defined(def_g), "g uses f in eq");

    let result = Vocabulary::new(1, vec![u], vec![decl_f, decl_g]);
    assert_eq!(result, Err(VocabularyError::CircularDefinition));
}

#[test]
fn cycle_through_pred_term_args_rejected() {
    let u = sort_decl_open("U");
    let base = pred_sym("Base", vec![sort("U")]);
    let f = func_sym("f", Vec::new(), "U");
    let p = pred_sym("P", Vec::new());

    // P defined as Base(f()) -> P depends on f
    let def_p = formula_pred(base.clone(), vec![term_const(f.clone())]);
    // f defined as Eq(var(0), var(0)) with And(1, Pred(P)) -> f depends on P
    let def_f = Formula::new(
        vec![
            FormulaNode::And(1),
            FormulaNode::Pred {
                symbol: p.clone(),
                args: Vec::new(),
            },
        ],
        Limits::default(),
    )
    .unwrap();

    let decl_base = symbol_decl(base, grounding_gloss("base"), "base");
    let decl_p = symbol_decl(p, Grounding::Defined(def_p), "P uses f in arg");
    let decl_f = symbol_decl(f, Grounding::Defined(def_f), "f uses P");

    let result = Vocabulary::new(1, vec![u], vec![decl_base, decl_p, decl_f]);
    assert_eq!(result, Err(VocabularyError::CircularDefinition));
}

#[test]
fn dag_out_of_declaration_order_produces_dependency_first() {
    let d0 = pred_sym("D0", Vec::new());
    let d1 = pred_sym("D1", Vec::new());

    // D1 is declared at index 0, but depends on D0 declared at index 1
    let decl_d1 = symbol_decl(
        d1,
        Grounding::Defined(formula_pred(d0.clone(), Vec::new())),
        "D1 depends on D0",
    );
    let decl_d0 = symbol_decl(d0, Grounding::Defined(formula_true()), "D0 is base");

    let vocab = Vocabulary::new(1, Vec::new(), vec![decl_d1, decl_d0]).unwrap();
    // Topological order must place D0 (index 1) before D1 (index 0)
    assert_eq!(vocab.definition_order(), &[1, 0]);
}

#[test]
fn dag_with_repeated_dependencies_accepted() {
    let d0 = pred_sym("D0", Vec::new());
    let d1 = pred_sym("D1", Vec::new());

    // D1 references D0 multiple times in its formula body
    let def_d1 = Formula::new(
        vec![
            FormulaNode::And(2),
            FormulaNode::Pred {
                symbol: d0.clone(),
                args: Vec::new(),
            },
            FormulaNode::Pred {
                symbol: d0.clone(),
                args: Vec::new(),
            },
        ],
        Limits::default(),
    )
    .unwrap();

    let decl_d0 = symbol_decl(d0, Grounding::Defined(formula_true()), "D0");
    let decl_d1 = symbol_decl(d1, Grounding::Defined(def_d1), "D1 references D0 twice");

    let vocab = Vocabulary::new(1, Vec::new(), vec![decl_d0, decl_d1]).unwrap();
    assert_eq!(vocab.definition_order(), &[0, 1]);
}

#[test]
fn topological_order_ignores_anchored_and_glossed() {
    let u = sort_decl_open("U");
    let s_gloss = pred_sym("SGloss", vec![sort("U")]);
    let s_anch = pred_sym("SAnch", vec![sort("U")]);
    let d0 = pred_sym("D0", vec![sort("U")]);
    let d1 = pred_sym("D1", vec![sort("U")]);

    let decl_gloss = symbol_decl(s_gloss.clone(), grounding_gloss("base"), "g");
    let decl_anch = symbol_decl(s_anch.clone(), grounding_anchored("did:1", "p"), "a");

    // D0 depends on base symbols s_gloss and s_anch (which are not Defined)
    let def_d0 = Formula::new(
        vec![
            FormulaNode::And(2),
            FormulaNode::Pred {
                symbol: s_gloss,
                args: vec![term_var(0, "U")],
            },
            FormulaNode::Pred {
                symbol: s_anch,
                args: vec![term_var(0, "U")],
            },
        ],
        Limits::default(),
    )
    .unwrap();

    let decl_d0 = symbol_decl(d0.clone(), Grounding::Defined(def_d0), "D0");
    let decl_d1 = symbol_decl(
        d1,
        Grounding::Defined(formula_pred(d0, vec![term_var(0, "U")])),
        "D1",
    );

    let vocab = Vocabulary::new(1, vec![u], vec![decl_gloss, decl_anch, decl_d0, decl_d1]).unwrap();

    // Only indices 2 and 3 (the Defined symbols) appear in definition_order!
    assert_eq!(vocab.definition_order(), &[2, 3]);
}

#[test]
fn missing_sort_in_symbol_arity_rejected() {
    let p = pred_sym("P", vec![sort("MissingSort")]);
    let decl = symbol_decl(p, grounding_gloss("p"), "p");
    let result = Vocabulary::new(1, Vec::new(), vec![decl]);
    assert_eq!(result, Err(VocabularyError::UnknownSort));
}

#[test]
fn missing_sort_in_symbol_result_rejected() {
    let f = func_sym("f", Vec::new(), "MissingSort");
    let decl = symbol_decl(f, grounding_gloss("f"), "f");
    let result = Vocabulary::new(1, Vec::new(), vec![decl]);
    assert_eq!(result, Err(VocabularyError::UnknownSort));
}

#[test]
fn missing_sort_in_term_var_rejected() {
    let p = pred_sym("P", Vec::new());
    let def_p = formula_eq(term_var(0, "MissingSort"), term_var(0, "MissingSort"));
    let decl = symbol_decl(p, Grounding::Defined(def_p), "p");
    let result = Vocabulary::new(1, Vec::new(), vec![decl]);
    assert_eq!(result, Err(VocabularyError::UnknownSort));
}

#[test]
fn missing_sort_in_binder_forall_rejected() {
    let p = pred_sym("P", Vec::new());
    let def_p = Formula::new(
        vec![FormulaNode::Forall(sort("MissingSort")), FormulaNode::True],
        Limits::default(),
    )
    .unwrap();
    let decl = symbol_decl(p, Grounding::Defined(def_p), "p");
    let result = Vocabulary::new(1, Vec::new(), vec![decl]);
    assert_eq!(result, Err(VocabularyError::UnknownSort));
}

#[test]
fn missing_sort_in_binder_exists_rejected() {
    let p = pred_sym("P", Vec::new());
    let def_p = Formula::new(
        vec![FormulaNode::Exists(sort("MissingSort")), FormulaNode::True],
        Limits::default(),
    )
    .unwrap();
    let decl = symbol_decl(p, Grounding::Defined(def_p), "p");
    let result = Vocabulary::new(1, Vec::new(), vec![decl]);
    assert_eq!(result, Err(VocabularyError::UnknownSort));
}

#[test]
fn missing_symbol_in_formula_pred_rejected() {
    let p = pred_sym("P", Vec::new());
    let undeclared = pred_sym("Undeclared", Vec::new());
    let def_p = formula_pred(undeclared, Vec::new());
    let decl = symbol_decl(p, Grounding::Defined(def_p), "p");
    let result = Vocabulary::new(1, Vec::new(), vec![decl]);
    assert_eq!(result, Err(VocabularyError::UnknownSymbol));
}

#[test]
fn missing_symbol_in_term_app_rejected() {
    let u = sort_decl_open("U");
    let p = pred_sym("P", Vec::new());
    let undeclared = const_sym("UndeclaredConst", "U");
    let def_p = formula_eq(term_const(undeclared), term_var(0, "U"));
    let decl = symbol_decl(p, Grounding::Defined(def_p), "p");
    let result = Vocabulary::new(1, vec![u], vec![decl]);
    assert_eq!(result, Err(VocabularyError::UnknownSymbol));
}

#[test]
fn signature_mismatch_in_formula_pred_rejected() {
    let u = sort_decl_open("U");
    let p0 = pred_sym("P0", Vec::new());
    let p1 = pred_sym("P1", Vec::new());

    // P0 declared with arity 0
    let decl_p0 = symbol_decl(p0, grounding_gloss("p0"), "p0");

    // But P1 definition references P0 with arity 1!
    let p0_wrong_arity = pred_sym("P0", vec![sort("U")]);
    let def_p1 = formula_pred(p0_wrong_arity, vec![term_var(0, "U")]);
    let decl_p1 = symbol_decl(p1, Grounding::Defined(def_p1), "p1");

    let result = Vocabulary::new(1, vec![u], vec![decl_p0, decl_p1]);
    assert_eq!(result, Err(VocabularyError::SignatureMismatch));
}

#[test]
fn signature_mismatch_in_term_app_rejected() {
    let u = sort_decl_open("U");
    let f = func_sym("f", Vec::new(), "U");
    let p = pred_sym("P", Vec::new());

    let decl_f = symbol_decl(f, grounding_gloss("f nullary"), "f");

    // Definition uses f with arity 1
    let f_wrong = func_sym("f", vec![sort("U")], "U");
    let term_f_wrong = Term::new(
        vec![
            TermNode::App {
                symbol: f_wrong,
                args: 1,
            },
            TermNode::Var {
                index: 0,
                sort: sort("U"),
            },
        ],
        Limits::default(),
    )
    .unwrap();
    let def_p = formula_eq(term_f_wrong, term_var(0, "U"));
    let decl_p = symbol_decl(p, Grounding::Defined(def_p), "p");

    let result = Vocabulary::new(1, vec![u], vec![decl_f, decl_p]);
    assert_eq!(result, Err(VocabularyError::SignatureMismatch));
}

#[test]
fn duplicate_sort_name_rejected() {
    let u1 = sort_decl_open("U");
    let u2 = sort_decl_open("U");
    let result = Vocabulary::new(1, vec![u1, u2], Vec::new());
    assert_eq!(result, Err(VocabularyError::DuplicateSort));
}

#[test]
fn duplicate_symbol_name_rejected() {
    let p1 = pred_sym("P", Vec::new());
    let p2 = pred_sym("P", vec![sort("U")]);
    let u = sort_decl_open("U");
    let decl1 = symbol_decl(p1, grounding_gloss("1"), "1");
    let decl2 = symbol_decl(p2, grounding_gloss("2"), "2");
    let result = Vocabulary::new(1, vec![u], vec![decl1, decl2]);
    assert_eq!(result, Err(VocabularyError::DuplicateSymbol));
}

#[test]
fn enumerated_domain_missing_constant_rejected() {
    let c_active = const_sym("Active", "Status");
    let sort_status = SortDecl {
        name: Text::new("Status").unwrap(),
        domain: SortDomain::Enumerated {
            constants: vec![c_active],
            domain_source: DomainSource::Fact(FactRef([0x11; 32])),
        },
    };
    let result = Vocabulary::new(1, vec![sort_status], Vec::new());
    assert_eq!(result, Err(VocabularyError::UnknownSymbol));
}

#[test]
fn enumerated_domain_signature_mismatch_rejected() {
    let c_active = const_sym("Active", "Status");
    let decl_active = symbol_decl(c_active, grounding_gloss("Active"), "a");

    let c_wrong = const_sym("Active", "OtherSort");
    let sort_other = sort_decl_open("OtherSort");
    let sort_status = SortDecl {
        name: Text::new("Status").unwrap(),
        domain: SortDomain::Enumerated {
            constants: vec![c_wrong],
            domain_source: DomainSource::Fact(FactRef([0x11; 32])),
        },
    };

    let result = Vocabulary::new(1, vec![sort_other, sort_status], vec![decl_active]);
    assert_eq!(result, Err(VocabularyError::SignatureMismatch));
}

#[test]
fn enumerated_domain_invalid_constant_kind_rejected() {
    let f_func = func_sym("f_func", Vec::new(), "Status");
    let decl_f = symbol_decl(f_func.clone(), grounding_gloss("f"), "f");

    let sort_status = SortDecl {
        name: Text::new("Status").unwrap(),
        domain: SortDomain::Enumerated {
            constants: vec![f_func],
            domain_source: DomainSource::Fact(FactRef([0x11; 32])),
        },
    };

    let result = Vocabulary::new(1, vec![sort_status], vec![decl_f]);
    assert_eq!(result, Err(VocabularyError::InvalidDomainConstant));
}

#[test]
fn enumerated_domain_invalid_constant_arity_rejected() {
    let u = sort_decl_open("U");
    let c_with_arity = symbol(
        "c_arity",
        SymbolKind::Const,
        vec![sort("U")],
        SymbolResult::Sort(sort("Status")),
    );
    let decl_c = symbol_decl(c_with_arity.clone(), grounding_gloss("c"), "c");

    let sort_status = SortDecl {
        name: Text::new("Status").unwrap(),
        domain: SortDomain::Enumerated {
            constants: vec![c_with_arity],
            domain_source: DomainSource::Fact(FactRef([0x11; 32])),
        },
    };

    let result = Vocabulary::new(1, vec![u, sort_status], vec![decl_c]);
    assert_eq!(result, Err(VocabularyError::InvalidDomainConstant));
}

#[test]
fn enumerated_domain_invalid_constant_result_sort_rejected() {
    let u = sort_decl_open("U");
    let c_wrong_res = const_sym("c_wrong", "U");
    let decl_c = symbol_decl(c_wrong_res.clone(), grounding_gloss("c"), "c");

    let sort_status = SortDecl {
        name: Text::new("Status").unwrap(),
        domain: SortDomain::Enumerated {
            constants: vec![c_wrong_res],
            domain_source: DomainSource::Fact(FactRef([0x11; 32])),
        },
    };

    let result = Vocabulary::new(1, vec![u, sort_status], vec![decl_c]);
    assert_eq!(result, Err(VocabularyError::InvalidDomainConstant));
}

#[test]
fn enumerated_domain_duplicate_constant_rejected() {
    let c_active = const_sym("Active", "Status");
    let decl_active = symbol_decl(c_active.clone(), grounding_gloss("Active"), "a");

    let sort_status = SortDecl {
        name: Text::new("Status").unwrap(),
        domain: SortDomain::Enumerated {
            constants: vec![c_active.clone(), c_active],
            domain_source: DomainSource::Fact(FactRef([0x11; 32])),
        },
    };

    let result = Vocabulary::new(1, vec![sort_status], vec![decl_active]);
    assert_eq!(result, Err(VocabularyError::InvalidDomainConstant));
}

#[test]
fn compatibility_addition_is_compatible() {
    let u = sort_decl_open("U");
    let p = pred_sym("P", vec![sort("U")]);
    let decl_p = symbol_decl(p, grounding_gloss("p"), "p");

    let vocab1 = Vocabulary::new(1, vec![u], vec![decl_p]).unwrap();

    let v = sort_decl_open("V");
    let q = pred_sym("Q", vec![sort("V")]);
    let decl_q = symbol_decl(q, grounding_gloss("q"), "q");

    let (vocab2, compat) = vocab1.extend(vec![v], vec![decl_q]).unwrap();
    assert_eq!(compat, Compatibility::Compatible);
    assert_eq!(vocab2.version(), 2);
    assert_eq!(
        vocab2.compatibility_with(&vocab1),
        Compatibility::Compatible
    );
}

#[test]
fn compatibility_reordering_is_compatible() {
    let u = sort_decl_open("U");
    let v = sort_decl_open("V");
    let p = pred_sym("P", vec![sort("U")]);
    let q = pred_sym("Q", vec![sort("V")]);
    let decl_p = symbol_decl(p, grounding_gloss("p"), "p");
    let decl_q = symbol_decl(q, grounding_gloss("q"), "q");

    let vocab1 = Vocabulary::new(
        1,
        vec![u.clone(), v.clone()],
        vec![decl_p.clone(), decl_q.clone()],
    )
    .unwrap();
    // Candidate has reversed declaration order
    let vocab2 = Vocabulary::new(1, vec![v, u], vec![decl_q, decl_p]).unwrap();

    assert_eq!(
        vocab2.compatibility_with(&vocab1),
        Compatibility::Compatible
    );
    assert_eq!(
        vocab1.compatibility_with(&vocab2),
        Compatibility::Compatible
    );
}

#[test]
fn compatibility_removal_is_incompatible() {
    let u = sort_decl_open("U");
    let v = sort_decl_open("V");
    let p = pred_sym("P", vec![sort("U")]);
    let decl_p = symbol_decl(p, grounding_gloss("p"), "p");

    let vocab1 = Vocabulary::new(1, vec![u.clone(), v], vec![decl_p.clone()]).unwrap();
    // Candidate removed sort V
    let vocab2 = Vocabulary::new(2, vec![u], vec![decl_p]).unwrap();

    assert_eq!(
        vocab2.compatibility_with(&vocab1),
        Compatibility::Incompatible
    );
}

#[test]
fn compatibility_gloss_change_is_incompatible() {
    let u = sort_decl_open("U");
    let p = pred_sym("P", vec![sort("U")]);
    let decl_p1 = symbol_decl(p.clone(), grounding_gloss("original gloss"), "original");
    let vocab1 = Vocabulary::new(1, vec![u.clone()], vec![decl_p1]).unwrap();

    // Patch modifies only gloss_pt
    let decl_p2 = symbol_decl(p, grounding_gloss("original gloss"), "modified gloss");
    let (vocab2, compat) = vocab1.extend(Vec::new(), vec![decl_p2]).unwrap();

    assert_eq!(compat, Compatibility::Incompatible);
    assert_eq!(
        vocab2.compatibility_with(&vocab1),
        Compatibility::Incompatible
    );
}

#[test]
fn compatibility_definition_change_is_incompatible() {
    let p = pred_sym("P", Vec::new());
    let decl_p1 = symbol_decl(p.clone(), Grounding::Defined(formula_true()), "p");
    let vocab1 = Vocabulary::new(1, Vec::new(), vec![decl_p1]).unwrap();

    // Patch modifies definition formula from True to False
    let def_false = Formula::new(vec![FormulaNode::False], Limits::default()).unwrap();
    let decl_p2 = symbol_decl(p, Grounding::Defined(def_false), "p");
    let (vocab2, compat) = vocab1.extend(Vec::new(), vec![decl_p2]).unwrap();

    assert_eq!(compat, Compatibility::Incompatible);
    assert_eq!(
        vocab2.compatibility_with(&vocab1),
        Compatibility::Incompatible
    );
}

#[test]
fn compatibility_sort_domain_change_is_incompatible() {
    let u_open = sort_decl_open("U");
    let c = const_sym("c", "U");
    let decl_c = symbol_decl(c.clone(), grounding_gloss("c"), "c");

    let vocab1 = Vocabulary::new(1, vec![u_open], vec![decl_c.clone()]).unwrap();

    // Patch changes sort U from Open to Enumerated
    let u_enum = SortDecl {
        name: Text::new("U").unwrap(),
        domain: SortDomain::Enumerated {
            constants: vec![c],
            domain_source: DomainSource::Fact(FactRef([0x33; 32])),
        },
    };
    let (vocab2, compat) = vocab1.extend(vec![u_enum], vec![decl_c]).unwrap();

    assert_eq!(compat, Compatibility::Incompatible);
    assert_eq!(
        vocab2.compatibility_with(&vocab1),
        Compatibility::Incompatible
    );
}

#[test]
fn compatibility_is_directional() {
    let u = sort_decl_open("U");
    let p = pred_sym("P", vec![sort("U")]);
    let q = pred_sym("Q", vec![sort("U")]);
    let decl_p = symbol_decl(p, grounding_gloss("p"), "p");
    let decl_q = symbol_decl(q, grounding_gloss("q"), "q");

    let vocab1 = Vocabulary::new(1, vec![u.clone()], vec![decl_p.clone()]).unwrap();
    let vocab2 = Vocabulary::new(2, vec![u], vec![decl_p, decl_q]).unwrap();

    // vocab2 preserves all declarations of vocab1 -> Compatible
    assert_eq!(
        vocab2.compatibility_with(&vocab1),
        Compatibility::Compatible
    );
    // vocab1 does NOT preserve decl_q of vocab2 -> Incompatible
    assert_eq!(
        vocab1.compatibility_with(&vocab2),
        Compatibility::Incompatible
    );
}

#[test]
fn extend_version_overflow_rejected() {
    let vocab = Vocabulary::new(u64::MAX, Vec::new(), Vec::new()).unwrap();
    let result = vocab.extend(Vec::new(), Vec::new());
    assert_eq!(result, Err(VocabularyError::VersionOverflow));
}

#[test]
fn extend_duplicate_patches_rejected() {
    let vocab = Vocabulary::new(1, Vec::new(), Vec::new()).unwrap();
    let u1 = sort_decl_open("U");
    let u2 = sort_decl_open("U");
    let result = vocab.extend(vec![u1, u2], Vec::new());
    assert_eq!(result, Err(VocabularyError::DuplicateSort));

    let p1 = pred_sym("P", Vec::new());
    let p2 = pred_sym("P", Vec::new());
    let decl1 = symbol_decl(p1, grounding_gloss("1"), "1");
    let decl2 = symbol_decl(p2, grounding_gloss("2"), "2");
    let result2 = vocab.extend(Vec::new(), vec![decl1, decl2]);
    assert_eq!(result2, Err(VocabularyError::DuplicateSymbol));
}

#[test]
fn extend_preserves_immutability() {
    let u = sort_decl_open("U");
    let p = pred_sym("P", vec![sort("U")]);
    let decl_p = symbol_decl(p, grounding_gloss("p"), "p");

    let vocab1 = Vocabulary::new(10, vec![u], vec![decl_p]).unwrap();
    let hash1 = hash(&vocab1).unwrap();

    let v = sort_decl_open("V");
    let (vocab2, _) = vocab1.extend(vec![v], Vec::new()).unwrap();

    // Original vocab1 must not change
    assert_eq!(vocab1.version(), 10);
    assert_eq!(vocab1.sorts().len(), 1);
    assert_eq!(hash(&vocab1).unwrap(), hash1);

    // New vocab2 is updated
    assert_eq!(vocab2.version(), 11);
    assert_eq!(vocab2.sorts().len(), 2);
    assert_ne!(hash(&vocab2).unwrap(), hash1);
}

#[test]
fn deep_dependency_chain_in_small_stack_thread() {
    // Run in a thread with a small 64 KiB stack to prove Kahn's topological sort,
    // clone, and drop operate iteratively without recursion or stack overflow.
    let handle = thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(|| {
            const CHAIN_LEN: usize = 1_000;
            let mut symbols = Vec::with_capacity(CHAIN_LEN);

            // D0 defined with True
            let d0 = pred_sym("D0", Vec::new());
            symbols.push(symbol_decl(
                d0,
                Grounding::Defined(formula_true()),
                "base d0",
            ));

            // D_i defined with Pred(D_{i-1})
            for i in 1..CHAIN_LEN {
                let prev_name = format!("D{}", i - 1);
                let prev_sym = pred_sym(&prev_name, Vec::new());
                let cur_name = format!("D{i}");
                let cur_sym = pred_sym(&cur_name, Vec::new());

                symbols.push(symbol_decl(
                    cur_sym,
                    Grounding::Defined(formula_pred(prev_sym, Vec::new())),
                    "chain node",
                ));
            }

            let vocab = Vocabulary::new(1, Vec::new(), symbols).unwrap();
            assert_eq!(vocab.definition_order().len(), CHAIN_LEN);
            assert_eq!(vocab.definition_order()[0], 0);
            assert_eq!(vocab.definition_order()[CHAIN_LEN - 1], CHAIN_LEN - 1);

            let cloned = vocab.clone();
            assert_eq!(cloned.definition_order().len(), CHAIN_LEN);
            drop(cloned);
            drop(vocab);
        })
        .unwrap();

    handle.join().unwrap();
}

#[test]
fn definition_limits_are_exact_and_independent() {
    let a = symbol_decl(
        pred_sym("A", Vec::new()),
        Grounding::Defined(formula_true()),
        "a",
    );
    let b = symbol_decl(
        pred_sym("B", Vec::new()),
        Grounding::Defined(formula_true()),
        "b",
    );
    let vocab = Vocabulary::new(1, Vec::new(), vec![a, b]).unwrap();
    let bytes = to_canon(&vocab).unwrap();
    let limits = Limits {
        max_depth: 1,
        max_nodes: 1,
    };
    assert_eq!(Vocabulary::decode_with_limits(&bytes, limits), Ok(vocab));
    assert_eq!(
        Vocabulary::decode_with_limits(
            &bytes,
            Limits {
                max_depth: 0,
                max_nodes: 1
            }
        ),
        Err(VocabularyError::Air(AirError::DepthExceeded {
            limit: 0,
            actual: 1
        }))
    );
    assert_eq!(
        Vocabulary::decode_with_limits(
            &bytes,
            Limits {
                max_depth: 1,
                max_nodes: 0
            }
        ),
        Err(VocabularyError::Air(AirError::NodesExceeded {
            limit: 0,
            actual: 1
        }))
    );
    let body = formula_eq(term_var(0, "U"), term_var(0, "U"));
    let p = symbol_decl(pred_sym("P", Vec::new()), Grounding::Defined(body), "p");
    let vocab = Vocabulary::new(1, vec![sort_decl_open("U")], vec![p]).unwrap();
    let bytes = to_canon(&vocab).unwrap();
    assert_eq!(
        Vocabulary::decode_with_limits(
            &bytes,
            Limits {
                max_depth: 2,
                max_nodes: 3
            }
        ),
        Ok(vocab)
    );
    assert_eq!(
        Vocabulary::decode_with_limits(
            &bytes,
            Limits {
                max_depth: 2,
                max_nodes: 2
            }
        ),
        Err(VocabularyError::Air(AirError::NodesExceeded {
            limit: 2,
            actual: 3
        }))
    );
}

#[test]
fn grounding_signature_and_domain_authority_changes_are_incompatible() {
    let p = symbol_decl(
        pred_sym("P", Vec::new()),
        grounding_anchored("monitor", "v1"),
        "p",
    );
    let vocab = Vocabulary::new(1, Vec::new(), vec![p.clone()]).unwrap();
    for grounding in [
        grounding_anchored("different", "v1"),
        grounding_anchored("monitor", "v2"),
        grounding_gloss("p"),
    ] {
        let mut changed = p.clone();
        changed.grounding = grounding;
        assert_eq!(
            vocab.extend(Vec::new(), vec![changed]).unwrap().1,
            Compatibility::Incompatible
        );
    }
    let mut changed = p;
    changed.signature.kind = SymbolKind::Func;
    assert_eq!(
        vocab.extend(Vec::new(), vec![changed]).unwrap().1,
        Compatibility::Incompatible
    );
    let domain = SortDecl {
        name: Text::new("U").unwrap(),
        domain: SortDomain::Enumerated {
            constants: Vec::new(),
            domain_source: DomainSource::Fact(FactRef([1; 32])),
        },
    };
    let vocab = Vocabulary::new(1, vec![domain.clone()], Vec::new()).unwrap();
    for source in [
        DomainSource::Fact(FactRef([2; 32])),
        DomainSource::Policy(PolicyRef([1; 32])),
    ] {
        let mut changed = domain.clone();
        changed.domain = SortDomain::Enumerated {
            constants: Vec::new(),
            domain_source: source,
        };
        assert_eq!(
            vocab.extend(vec![changed], Vec::new()).unwrap().1,
            Compatibility::Incompatible
        );
    }
}

#[test]
fn extend_revalidates_cycles_and_existing_references() {
    let p = pred_sym("P", Vec::new());
    let q = pred_sym("Q", Vec::new());
    let original = Vocabulary::new(
        1,
        Vec::new(),
        vec![
            symbol_decl(
                p.clone(),
                Grounding::Defined(formula_pred(q.clone(), Vec::new())),
                "p",
            ),
            symbol_decl(q.clone(), grounding_gloss("q"), "q"),
        ],
    )
    .unwrap();
    let cyclic = symbol_decl(
        q.clone(),
        Grounding::Defined(formula_pred(p, Vec::new())),
        "q",
    );
    assert_eq!(
        original.extend(Vec::new(), vec![cyclic]),
        Err(VocabularyError::CircularDefinition)
    );
    let mut changed = q;
    changed.kind = SymbolKind::Func;
    let patch = symbol_decl(changed, grounding_gloss("q"), "q");
    assert_eq!(
        original.extend(Vec::new(), vec![patch]),
        Err(VocabularyError::SignatureMismatch)
    );
    assert_eq!(original.version(), 1);
}
