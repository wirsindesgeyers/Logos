//! Testes de comportamento determinístico para a avaliação finita M ⊨ φ (logos-model::eval).
//!
//! Cobre:
//! - Conectivos e identidades n-árias (True, False, Not, And(0..n), Or(0..n), Implies, Iff)
//! - Avaliação integral sem curto-circuito
//! - Termos, funções densas e predicados com sorts mistos
//! - Quantificadores (Forall, Exists), sombras, escopos irmãos e variáveis externas De Bruijn
//! - Predicados definidos e relações Const/Func (resultado no índice 0, argumentos em 1+)
//! - Deslocamento de De Bruijn sob quantificadores em corpos definidos
//! - Cadeias de símbolos definidos (chamadas aninhadas de predicados e funções)
//! - Erro InvalidDefinition para relações não totais ou multivaloradas (inclusive não utilizadas)
//! - Erro Sort para corpos de definições mal tipados
//! - Erro VocabularyMismatch para hashes divergentes h(V)
//! - Erro EnumerationMismatch para constantes definidas que divergem do ordinal intrínseco (inclusive não utilizadas)
//! - Orçamento S2: Unknown(Steps) preservado em fórmulas verdadeiras e falsas
//! - Fronteira exata de passos e 1000 repetições determinísticas sem dependência de relógio (S3)
//! - Interpretador com pilha explícita em thread com pilha restrita de 64 KiB
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use logos_air::{
    DomainSource, FactRef, Formula, FormulaNode, Grounding, Limits, Sort, SortDecl, SortDomain,
    Symbol, SymbolDecl, SymbolKind, SymbolResult, Term, TermNode, Vocabulary,
};
use logos_canon::Text;
use logos_core::{Budget, Exhausted};
use logos_crypto::hash;
use logos_model::{EvalError, FiniteModel, ModelData, ModelLimits, eval};

// ============================================================================
// Construtores e Helpers
// ============================================================================

fn sort(name: &str) -> Sort {
    Sort {
        name: Text::new(name).expect("sort text"),
    }
}

fn sort_decl_open(name: &str) -> SortDecl {
    SortDecl {
        name: Text::new(name).expect("sort decl text"),
        domain: SortDomain::Open,
    }
}

fn sort_decl_enum(name: &str, constants: Vec<Symbol>, fact_seed: u8) -> SortDecl {
    SortDecl {
        name: Text::new(name).expect("sort decl text"),
        domain: SortDomain::Enumerated {
            constants,
            domain_source: DomainSource::Fact(FactRef([fact_seed; 32])),
        },
    }
}

fn const_sym(name: &str, sort_name: &str) -> Symbol {
    Symbol {
        name: Text::new(name).expect("symbol text"),
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(sort(sort_name)),
    }
}

fn func_sym(name: &str, arity: Vec<Sort>, result_sort: &str) -> Symbol {
    Symbol {
        name: Text::new(name).expect("symbol text"),
        kind: SymbolKind::Func,
        arity,
        result: SymbolResult::Sort(sort(result_sort)),
    }
}

fn pred_sym(name: &str, arity: Vec<Sort>) -> Symbol {
    Symbol {
        name: Text::new(name).expect("symbol text"),
        kind: SymbolKind::Pred,
        arity,
        result: SymbolResult::Bool,
    }
}

fn primitive(sig: Symbol) -> SymbolDecl {
    SymbolDecl {
        signature: sig,
        grounding: Grounding::Glossed(Text::new("primitivo").expect("gloss")),
        gloss_pt: Text::new("glosa").expect("gloss_pt"),
    }
}

fn defined(sig: Symbol, body: Formula) -> SymbolDecl {
    SymbolDecl {
        signature: sig,
        grounding: Grounding::Defined(body),
        gloss_pt: Text::new("definido").expect("gloss_pt"),
    }
}

fn generous_budget() -> Budget {
    Budget::new(10_000_000, 10_000_000, 10_000_000, 10_000)
}

fn form(nodes: Vec<FormulaNode>) -> Formula {
    Formula::new(nodes, Limits::default()).expect("valid formula")
}

fn term_var(index: u64, sort_name: &str) -> Term {
    Term::new(
        vec![TermNode::Var {
            index,
            sort: sort(sort_name),
        }],
        Limits::default(),
    )
    .expect("valid var term")
}

fn term_const(sym: Symbol) -> Term {
    Term::new(
        vec![TermNode::App {
            symbol: sym,
            args: 0,
        }],
        Limits::default(),
    )
    .expect("valid const term")
}

fn term_app1(func: Symbol, arg: Term) -> Term {
    let mut nodes = vec![TermNode::App {
        symbol: func,
        args: 1,
    }];
    nodes.extend(arg.nodes().iter().cloned());
    Term::new(nodes, Limits::default()).expect("valid app1 term")
}

fn term_app2(func: Symbol, arg1: Term, arg2: Term) -> Term {
    let mut nodes = vec![TermNode::App {
        symbol: func,
        args: 2,
    }];
    nodes.extend(arg1.nodes().iter().cloned());
    nodes.extend(arg2.nodes().iter().cloned());
    Term::new(nodes, Limits::default()).expect("valid app2 term")
}

// ============================================================================
// 1. Conectivos e Identidades N-árias (Clássicos e Integrais)
// ============================================================================

#[test]
fn test_eval_truth_constants() {
    let vocab = Vocabulary::new(1, vec![sort_decl_open("S")], Vec::new()).expect("vocab");
    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    let f_true = form(vec![FormulaNode::True]);
    assert_eq!(
        eval(&model, &vocab, &f_true, &generous_budget()),
        Ok(true),
        "True deve avaliar para true"
    );

    let f_false = form(vec![FormulaNode::False]);
    assert_eq!(
        eval(&model, &vocab, &f_false, &generous_budget()),
        Ok(false),
        "False deve avaliar para false"
    );
}

#[test]
fn test_eval_not() {
    let vocab = Vocabulary::new(1, vec![sort_decl_open("S")], Vec::new()).expect("vocab");
    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    let f_not_true = form(vec![FormulaNode::Not, FormulaNode::True]);
    assert_eq!(
        eval(&model, &vocab, &f_not_true, &generous_budget()),
        Ok(false)
    );

    let f_not_false = form(vec![FormulaNode::Not, FormulaNode::False]);
    assert_eq!(
        eval(&model, &vocab, &f_not_false, &generous_budget()),
        Ok(true)
    );
}

#[test]
fn test_eval_and_identities() {
    let vocab = Vocabulary::new(1, vec![sort_decl_open("S")], Vec::new()).expect("vocab");
    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // And(0) é a identidade de conjunção: True
    let f_and0 = form(vec![FormulaNode::And(0)]);
    assert_eq!(
        eval(&model, &vocab, &f_and0, &generous_budget()),
        Ok(true),
        "And(0) vazio deve ser True"
    );

    // And(1)
    let f_and1_t = form(vec![FormulaNode::And(1), FormulaNode::True]);
    assert_eq!(
        eval(&model, &vocab, &f_and1_t, &generous_budget()),
        Ok(true)
    );
    let f_and1_f = form(vec![FormulaNode::And(1), FormulaNode::False]);
    assert_eq!(
        eval(&model, &vocab, &f_and1_f, &generous_budget()),
        Ok(false)
    );

    // And(2)
    let f_and2_tt = form(vec![
        FormulaNode::And(2),
        FormulaNode::True,
        FormulaNode::True,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_and2_tt, &generous_budget()),
        Ok(true)
    );
    let f_and2_tf = form(vec![
        FormulaNode::And(2),
        FormulaNode::True,
        FormulaNode::False,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_and2_tf, &generous_budget()),
        Ok(false)
    );
    let f_and2_ft = form(vec![
        FormulaNode::And(2),
        FormulaNode::False,
        FormulaNode::True,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_and2_ft, &generous_budget()),
        Ok(false)
    );
    let f_and2_ff = form(vec![
        FormulaNode::And(2),
        FormulaNode::False,
        FormulaNode::False,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_and2_ff, &generous_budget()),
        Ok(false)
    );

    // And(3)
    let f_and3_ttt = form(vec![
        FormulaNode::And(3),
        FormulaNode::True,
        FormulaNode::True,
        FormulaNode::True,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_and3_ttt, &generous_budget()),
        Ok(true)
    );
    let f_and3_tft = form(vec![
        FormulaNode::And(3),
        FormulaNode::True,
        FormulaNode::False,
        FormulaNode::True,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_and3_tft, &generous_budget()),
        Ok(false)
    );
}

#[test]
fn test_eval_or_identities() {
    let vocab = Vocabulary::new(1, vec![sort_decl_open("S")], Vec::new()).expect("vocab");
    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // Or(0) é a identidade de disjunção: False
    let f_or0 = form(vec![FormulaNode::Or(0)]);
    assert_eq!(
        eval(&model, &vocab, &f_or0, &generous_budget()),
        Ok(false),
        "Or(0) vazio deve ser False"
    );

    // Or(1)
    let f_or1_t = form(vec![FormulaNode::Or(1), FormulaNode::True]);
    assert_eq!(eval(&model, &vocab, &f_or1_t, &generous_budget()), Ok(true));
    let f_or1_f = form(vec![FormulaNode::Or(1), FormulaNode::False]);
    assert_eq!(
        eval(&model, &vocab, &f_or1_f, &generous_budget()),
        Ok(false)
    );

    // Or(2)
    let f_or2_tt = form(vec![
        FormulaNode::Or(2),
        FormulaNode::True,
        FormulaNode::True,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_or2_tt, &generous_budget()),
        Ok(true)
    );
    let f_or2_tf = form(vec![
        FormulaNode::Or(2),
        FormulaNode::True,
        FormulaNode::False,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_or2_tf, &generous_budget()),
        Ok(true)
    );
    let f_or2_ft = form(vec![
        FormulaNode::Or(2),
        FormulaNode::False,
        FormulaNode::True,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_or2_ft, &generous_budget()),
        Ok(true)
    );
    let f_or2_ff = form(vec![
        FormulaNode::Or(2),
        FormulaNode::False,
        FormulaNode::False,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_or2_ff, &generous_budget()),
        Ok(false)
    );

    // Or(3)
    let f_or3_fff = form(vec![
        FormulaNode::Or(3),
        FormulaNode::False,
        FormulaNode::False,
        FormulaNode::False,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_or3_fff, &generous_budget()),
        Ok(false)
    );
    let f_or3_ftf = form(vec![
        FormulaNode::Or(3),
        FormulaNode::False,
        FormulaNode::True,
        FormulaNode::False,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_or3_ftf, &generous_budget()),
        Ok(true)
    );
}

#[test]
fn test_eval_implies_and_iff() {
    let vocab = Vocabulary::new(1, vec![sort_decl_open("S")], Vec::new()).expect("vocab");
    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // Implies: T->T=T, T->F=F, F->T=T, F->F=T
    let f_imp_tt = form(vec![
        FormulaNode::Implies,
        FormulaNode::True,
        FormulaNode::True,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_imp_tt, &generous_budget()),
        Ok(true)
    );
    let f_imp_tf = form(vec![
        FormulaNode::Implies,
        FormulaNode::True,
        FormulaNode::False,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_imp_tf, &generous_budget()),
        Ok(false)
    );
    let f_imp_ft = form(vec![
        FormulaNode::Implies,
        FormulaNode::False,
        FormulaNode::True,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_imp_ft, &generous_budget()),
        Ok(true)
    );
    let f_imp_ff = form(vec![
        FormulaNode::Implies,
        FormulaNode::False,
        FormulaNode::False,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_imp_ff, &generous_budget()),
        Ok(true)
    );

    // Iff: T<->T=T, T<->F=F, F<->T=F, F<->F=T
    let f_iff_tt = form(vec![FormulaNode::Iff, FormulaNode::True, FormulaNode::True]);
    assert_eq!(
        eval(&model, &vocab, &f_iff_tt, &generous_budget()),
        Ok(true)
    );
    let f_iff_tf = form(vec![
        FormulaNode::Iff,
        FormulaNode::True,
        FormulaNode::False,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_iff_tf, &generous_budget()),
        Ok(false)
    );
    let f_iff_ft = form(vec![
        FormulaNode::Iff,
        FormulaNode::False,
        FormulaNode::True,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_iff_ft, &generous_budget()),
        Ok(false)
    );
    let f_iff_ff = form(vec![
        FormulaNode::Iff,
        FormulaNode::False,
        FormulaNode::False,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_iff_ff, &generous_budget()),
        Ok(true)
    );
}

#[test]
fn test_integral_evaluation_no_short_circuit() {
    let vocab = Vocabulary::new(1, vec![sort_decl_open("S")], Vec::new()).expect("vocab");
    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // Conjunção And(2) [False, True]
    // Pré-requisito: h(V)=1, formula.node_count()=3 -> 4 passos de preflight.
    // Avaliação: And=1, False=1, True=1 -> 3 passos.
    // Total exato = 7 passos.
    let f_and_integral = form(vec![
        FormulaNode::And(2),
        FormulaNode::False,
        FormulaNode::True,
    ]);

    // Com 6 passos, atinge o False (passo 6), mas não tem orçamento para o segundo filho True.
    // Se houvesse curto-circuito, retornaria Ok(false). Como é integral, DEVE esgotar com Unknown(Steps)!
    let budget_no_sc = Budget::new(6, 10_000_000, 10_000_000, 10_000);
    assert_eq!(
        eval(&model, &vocab, &f_and_integral, &budget_no_sc),
        Err(EvalError::Unknown(Exhausted::Steps)),
        "avaliação integral de And não deve fazer curto-circuito"
    );

    // Com 7 passos exatos, avalia integralmente e retorna Ok(false).
    let budget_exact = Budget::new(7, 10_000_000, 10_000_000, 10_000);
    assert_eq!(
        eval(&model, &vocab, &f_and_integral, &budget_exact),
        Ok(false)
    );

    // Disjunção Or(2) [True, False]
    // Com 6 passos, atinge True, mas deve continuar e esgotar no False.
    let f_or_integral = form(vec![
        FormulaNode::Or(2),
        FormulaNode::True,
        FormulaNode::False,
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_or_integral, &budget_no_sc),
        Err(EvalError::Unknown(Exhausted::Steps)),
        "avaliação integral de Or não deve fazer curto-circuito"
    );
    assert_eq!(
        eval(&model, &vocab, &f_or_integral, &budget_exact),
        Ok(true)
    );
}

// ============================================================================
// 2. Termos, Funções Densas e Predicados com Sorts Mistos
// ============================================================================

#[test]
fn test_eval_equality_and_primitive_symbols() {
    let s = sort("S");
    let c0 = const_sym("c0", "S");
    let c1 = const_sym("c1", "S");
    let f = func_sym("f", vec![s.clone()], "S");
    let p_null = pred_sym("P_null", Vec::new());
    let p_unary = pred_sym("P_unary", vec![s.clone()]);
    let p_bin = pred_sym("P_bin", vec![s.clone(), s.clone()]);

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![
            primitive(c0.clone()),
            primitive(c1.clone()),
            primitive(f.clone()),
            primitive(p_null.clone()),
            primitive(p_unary.clone()),
            primitive(p_bin.clone()),
        ],
    )
    .expect("vocab");

    // Modelo com domínio {0, 1}:
    // c0 = 0, c1 = 1
    // f(0) = 1, f(1) = 0
    // P_null = true (contém tupla vazia vec![])
    // P_unary = { (0) }
    // P_bin = { (0, 1), (1, 0) }
    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: vec![(0, 0), (1, 1)],
        funcs: vec![(2, vec![1, 0])],
        preds: vec![
            (3, vec![vec![]]),
            (4, vec![vec![0]]),
            (5, vec![vec![0, 1], vec![1, 0]]),
        ],
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // Eq(c0, c0) -> true; Eq(c0, c1) -> false
    let f_eq_same = form(vec![FormulaNode::Eq(
        term_const(c0.clone()),
        term_const(c0.clone()),
    )]);
    assert_eq!(
        eval(&model, &vocab, &f_eq_same, &generous_budget()),
        Ok(true)
    );

    let f_eq_diff = form(vec![FormulaNode::Eq(
        term_const(c0.clone()),
        term_const(c1.clone()),
    )]);
    assert_eq!(
        eval(&model, &vocab, &f_eq_diff, &generous_budget()),
        Ok(false)
    );

    // Eq(f(c0), c1) -> true pois f(0) = 1 == c1
    let f_eq_func = form(vec![FormulaNode::Eq(
        term_app1(f.clone(), term_const(c0.clone())),
        term_const(c1.clone()),
    )]);
    assert_eq!(
        eval(&model, &vocab, &f_eq_func, &generous_budget()),
        Ok(true)
    );

    // Predicado nulário: P_null é true
    let f_p_null = form(vec![FormulaNode::Pred {
        symbol: p_null.clone(),
        args: Vec::new(),
    }]);
    assert_eq!(
        eval(&model, &vocab, &f_p_null, &generous_budget()),
        Ok(true)
    );

    // Predicado unário: P_unary(c0) -> true, P_unary(c1) -> false
    let f_p_u0 = form(vec![FormulaNode::Pred {
        symbol: p_unary.clone(),
        args: vec![term_const(c0.clone())],
    }]);
    assert_eq!(eval(&model, &vocab, &f_p_u0, &generous_budget()), Ok(true));

    let f_p_u1 = form(vec![FormulaNode::Pred {
        symbol: p_unary.clone(),
        args: vec![term_const(c1.clone())],
    }]);
    assert_eq!(eval(&model, &vocab, &f_p_u1, &generous_budget()), Ok(false));

    // Predicado binário: P_bin(c0, c1) -> true, P_bin(c0, c0) -> false
    let f_p_b_01 = form(vec![FormulaNode::Pred {
        symbol: p_bin.clone(),
        args: vec![term_const(c0.clone()), term_const(c1.clone())],
    }]);
    assert_eq!(
        eval(&model, &vocab, &f_p_b_01, &generous_budget()),
        Ok(true)
    );

    let f_p_b_00 = form(vec![FormulaNode::Pred {
        symbol: p_bin.clone(),
        args: vec![term_const(c0.clone()), term_const(c0.clone())],
    }]);
    assert_eq!(
        eval(&model, &vocab, &f_p_b_00, &generous_budget()),
        Ok(false)
    );
}

#[test]
fn test_eval_mixed_sort_dense_functions_and_preds() {
    let sa = sort("A");
    let sb = sort("B");
    let sc = sort("C");

    let a0 = const_sym("a0", "A");
    let a1 = const_sym("a1", "A");
    let b0 = const_sym("b0", "B");
    let b1 = const_sym("b1", "B");
    let b2 = const_sym("b2", "B");
    let c0 = const_sym("c0", "C");
    let c1 = const_sym("c1", "C");

    // f: A x B -> C (aridade 2x3 = 6 células densas, último eixo B mais rápido)
    let f = func_sym("f", vec![sa.clone(), sb.clone()], "C");
    // p: A x B x C -> Bool
    let p = pred_sym("P", vec![sa.clone(), sb.clone(), sc.clone()]);

    let vocab = Vocabulary::new(
        1,
        vec![
            sort_decl_open("A"),
            sort_decl_open("B"),
            sort_decl_open("C"),
        ],
        vec![
            primitive(a0.clone()),
            primitive(a1.clone()),
            primitive(b0.clone()),
            primitive(b1.clone()),
            primitive(b2.clone()),
            primitive(c0.clone()),
            primitive(c1.clone()),
            primitive(f.clone()),
            primitive(p.clone()),
        ],
    )
    .expect("vocab");

    // Domínios: |A|=2, |B|=3, |C|=2
    // f outputs (ordem lexicográfica com último eixo mais rápido):
    // (a=0, b=0) -> 0
    // (a=0, b=1) -> 1
    // (a=0, b=2) -> 0
    // (a=1, b=0) -> 1
    // (a=1, b=1) -> 0
    // (a=1, b=2) -> 1
    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2, 3, 2],
        consts: vec![(0, 0), (1, 1), (2, 0), (3, 1), (4, 2), (5, 0), (6, 1)],
        funcs: vec![(7, vec![0, 1, 0, 1, 0, 1])],
        preds: vec![(8, vec![vec![0, 1, 1], vec![1, 2, 0]])],
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // Testa f(a0, b1) == c1 (saída esperada: 1, que é c1)
    let f_app = form(vec![FormulaNode::Eq(
        term_app2(f.clone(), term_const(a0.clone()), term_const(b1.clone())),
        term_const(c1.clone()),
    )]);
    assert_eq!(eval(&model, &vocab, &f_app, &generous_budget()), Ok(true));

    // Testa f(a1, b0) == c1 (saída esperada: 1 == c1)
    let f_app2 = form(vec![FormulaNode::Eq(
        term_app2(f.clone(), term_const(a1.clone()), term_const(b0.clone())),
        term_const(c1.clone()),
    )]);
    assert_eq!(eval(&model, &vocab, &f_app2, &generous_budget()), Ok(true));

    // Testa f(a1, b1) == c1 (saída esperada: 0 != c1)
    let f_app3 = form(vec![FormulaNode::Eq(
        term_app2(f.clone(), term_const(a1.clone()), term_const(b1.clone())),
        term_const(c1.clone()),
    )]);
    assert_eq!(eval(&model, &vocab, &f_app3, &generous_budget()), Ok(false));

    // Testa predicado com sorts mistos P(a0, b1, c1) -> presente
    let f_p1 = form(vec![FormulaNode::Pred {
        symbol: p.clone(),
        args: vec![
            term_const(a0.clone()),
            term_const(b1.clone()),
            term_const(c1.clone()),
        ],
    }]);
    assert_eq!(eval(&model, &vocab, &f_p1, &generous_budget()), Ok(true));

    // P(a0, b0, c0) -> ausente
    let f_p2 = form(vec![FormulaNode::Pred {
        symbol: p.clone(),
        args: vec![
            term_const(a0.clone()),
            term_const(b0.clone()),
            term_const(c0.clone()),
        ],
    }]);
    assert_eq!(eval(&model, &vocab, &f_p2, &generous_budget()), Ok(false));
}

// ============================================================================
// 3. Quantificadores, Sombras, Escopos Irmãos e Variáveis De Bruijn
// ============================================================================

#[test]
fn test_eval_quantifiers_forall_exists() {
    let s = sort("S");
    let c0 = const_sym("c0", "S");
    let p = pred_sym("P", vec![s.clone()]);

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![primitive(c0.clone()), primitive(p.clone())],
    )
    .expect("vocab");

    // Modelo com domínio {0, 1, 2}:
    // P = { (0), (1) } (não contém 2)
    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![3],
        consts: vec![(0, 0)],
        funcs: Vec::new(),
        preds: vec![(1, vec![vec![0], vec![1]])],
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // Forall(S, P(Var(0))) -> falso pois 2 não está em P
    let f_forall = form(vec![
        FormulaNode::Forall(s.clone()),
        FormulaNode::Pred {
            symbol: p.clone(),
            args: vec![term_var(0, "S")],
        },
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_forall, &generous_budget()),
        Ok(false),
        "Forall deve ser falso quando algum elemento do domínio falha"
    );

    // Exists(S, P(Var(0))) -> verdadeiro pois 0 e 1 estão em P
    let f_exists = form(vec![
        FormulaNode::Exists(s.clone()),
        FormulaNode::Pred {
            symbol: p.clone(),
            args: vec![term_var(0, "S")],
        },
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_exists, &generous_budget()),
        Ok(true),
        "Exists deve ser verdadeiro quando pelo menos um elemento satisfaz"
    );

    // Exists(S, Eq(Var(0), c0)) -> verdadeiro
    let f_exists_c0 = form(vec![
        FormulaNode::Exists(s.clone()),
        FormulaNode::Eq(term_var(0, "S"), term_const(c0.clone())),
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_exists_c0, &generous_budget()),
        Ok(true)
    );
}

#[test]
fn test_eval_sibling_binder_scopes() {
    let s = sort("S");
    let p = pred_sym("P", vec![s.clone()]);
    let q = pred_sym("Q", vec![s.clone()]);

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![primitive(p.clone()), primitive(q.clone())],
    )
    .expect("vocab");

    // Modelo com domínio {0, 1}: P = { (0) }, Q = { (1) }
    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: vec![(0, vec![vec![0]]), (1, vec![vec![1]])],
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // And(2) [ Exists(S, P(Var(0))), Exists(S, Q(Var(0))) ]
    // Ambos usam índice 0 em subárvores irmãs independentes.
    let f_siblings = form(vec![
        FormulaNode::And(2),
        FormulaNode::Exists(s.clone()),
        FormulaNode::Pred {
            symbol: p.clone(),
            args: vec![term_var(0, "S")],
        },
        FormulaNode::Exists(s.clone()),
        FormulaNode::Pred {
            symbol: q.clone(),
            args: vec![term_var(0, "S")],
        },
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_siblings, &generous_budget()),
        Ok(true),
        "escopos irmãos com o mesmo índice de binder não devem colidir"
    );
}

#[test]
fn test_eval_quantifier_shadowing_and_outer_variables() {
    let s = sort("S");
    let vocab = Vocabulary::new(1, vec![sort_decl_open("S")], Vec::new()).expect("vocab");

    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // ∀x. ∃y. x = y
    // Forall(S) [ Exists(S) [ Eq(Var(0), Var(1)) ] ]
    // Var(0) é o binder interno (y), Var(1) é o binder externo (x).
    // Em qualquer modelo não vazio, para todo x existe y tal que y = x (o próprio x).
    let f_forall_exists_eq = form(vec![
        FormulaNode::Forall(s.clone()),
        FormulaNode::Exists(s.clone()),
        FormulaNode::Eq(term_var(0, "S"), term_var(1, "S")),
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_forall_exists_eq, &generous_budget()),
        Ok(true),
        "∀x ∃y (y = x) deve ser verdadeiro"
    );

    // ∀x. ∀y. x = y
    // Forall(S) [ Forall(S) [ Eq(Var(0), Var(1)) ] ]
    // Em um domínio de tamanho 2 {0, 1}, isto é falso pois 0 != 1.
    let f_forall_forall_eq = form(vec![
        FormulaNode::Forall(s.clone()),
        FormulaNode::Forall(s.clone()),
        FormulaNode::Eq(term_var(0, "S"), term_var(1, "S")),
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_forall_forall_eq, &generous_budget()),
        Ok(false),
        "∀x ∀y (y = x) deve ser falso para domínio de tamanho 2"
    );

    // Três níveis de binders:
    // ∀x. ∃y. ∀z. (z = y ∨ z = x)
    // Em domínio {0, 1}, para todo x, escolhendo y = 1 - x, {x, y} cobre {0, 1}. Logo ∀z (z=y ∨ z=x) é true!
    let f_three_levels = form(vec![
        FormulaNode::Forall(s.clone()),
        FormulaNode::Exists(s.clone()),
        FormulaNode::Forall(s.clone()),
        FormulaNode::Or(2),
        FormulaNode::Eq(term_var(0, "S"), term_var(1, "S")), // z = y
        FormulaNode::Eq(term_var(0, "S"), term_var(2, "S")), // z = x
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_three_levels, &generous_budget()),
        Ok(true),
        "acesso De Bruijn a 3 níveis (Var(0)=z, Var(1)=y, Var(2)=x) deve ser correto"
    );
}

// ============================================================================
// 4. Símbolos Definidos (Predicados e Const/Func Relacionais)
// ============================================================================

#[test]
fn test_eval_defined_predicate_and_quantifier_shift() {
    let s = sort("S");
    let c0 = const_sym("c0", "S");

    // Predicado primitivo P
    let p = pred_sym("P", vec![s.clone()]);

    // Predicado definido Q(x): x = c0
    // Parâmetro x está no índice De Bruijn 0.
    let q_body = form(vec![FormulaNode::Eq(
        term_var(0, "S"),
        term_const(c0.clone()),
    )]);
    let q = pred_sym("Q", vec![s.clone()]);

    // Predicado definido R(x): ∃y. P(y) ∧ x = y
    // Sob Exists(S), y é Var(0), e o parâmetro x é deslocado para Var(1).
    let r_body = form(vec![
        FormulaNode::Exists(s.clone()),
        FormulaNode::And(2),
        FormulaNode::Pred {
            symbol: p.clone(),
            args: vec![term_var(0, "S")],
        },
        FormulaNode::Eq(term_var(1, "S"), term_var(0, "S")),
    ]);
    let r = pred_sym("R", vec![s.clone()]);

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![
            primitive(c0.clone()),
            primitive(p.clone()),
            defined(q.clone(), q_body),
            defined(r.clone(), r_body),
        ],
    )
    .expect("vocab");

    // Modelo com domínio {0, 1}: c0 = 0, P = { (1) }
    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: vec![(0, 0)],
        funcs: Vec::new(),
        preds: vec![(1, vec![vec![1]])],
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // Q(c0) deve ser verdadeiro (0 = 0)
    let f_q_c0 = form(vec![FormulaNode::Pred {
        symbol: q.clone(),
        args: vec![term_const(c0.clone())],
    }]);
    assert_eq!(eval(&model, &vocab, &f_q_c0, &generous_budget()), Ok(true));

    // R(c0) deve ser falso (P(0) não é verdadeiro, apenas P(1))
    let f_r_c0 = form(vec![FormulaNode::Pred {
        symbol: r.clone(),
        args: vec![term_const(c0.clone())],
    }]);
    assert_eq!(eval(&model, &vocab, &f_r_c0, &generous_budget()), Ok(false));

    // ∃x. R(x) deve ser verdadeiro (para x = 1)
    let f_exists_r = form(vec![
        FormulaNode::Exists(s.clone()),
        FormulaNode::Pred {
            symbol: r.clone(),
            args: vec![term_var(0, "S")],
        },
    ]);
    assert_eq!(
        eval(&model, &vocab, &f_exists_r, &generous_budget()),
        Ok(true)
    );
}

#[test]
fn test_eval_defined_relational_const_and_func() {
    let s = sort("S");
    let c0 = const_sym("c0", "S");
    let c1 = const_sym("c1", "S");

    // Constante definida c_def: resultado no índice 0
    // Corpo relacional: Var(0) = c1
    let c_def_sym = const_sym("c_def", "S");
    let c_def_body = form(vec![FormulaNode::Eq(
        term_var(0, "S"),
        term_const(c1.clone()),
    )]);

    // Função definida f_id(x): identidade
    // Resultado no índice 0, argumento x no índice 1.
    // Corpo relacional: Var(0) = Var(1)
    let f_id_sym = func_sym("f_id", vec![s.clone()], "S");
    let f_id_body = form(vec![FormulaNode::Eq(term_var(0, "S"), term_var(1, "S"))]);

    // Função definida f_choose_first(x, y): retorna x
    // Resultado no índice 0, arg 0 (x) no índice 1, arg 1 (y) no índice 2.
    // Corpo relacional: Var(0) = Var(1)
    let f_choose_sym = func_sym("f_choose", vec![s.clone(), s.clone()], "S");
    let f_choose_body = form(vec![FormulaNode::Eq(term_var(0, "S"), term_var(1, "S"))]);

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![
            primitive(c0.clone()),
            primitive(c1.clone()),
            defined(c_def_sym.clone(), c_def_body),
            defined(f_id_sym.clone(), f_id_body),
            defined(f_choose_sym.clone(), f_choose_body),
        ],
    )
    .expect("vocab");

    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: vec![(0, 0), (1, 1)],
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // Eq(c_def, c1) -> verdadeiro
    let f_test_c_def = form(vec![FormulaNode::Eq(
        term_const(c_def_sym.clone()),
        term_const(c1.clone()),
    )]);
    assert_eq!(
        eval(&model, &vocab, &f_test_c_def, &generous_budget()),
        Ok(true)
    );

    // Eq(f_id(c0), c0) -> verdadeiro
    let f_test_f_id = form(vec![FormulaNode::Eq(
        term_app1(f_id_sym.clone(), term_const(c0.clone())),
        term_const(c0.clone()),
    )]);
    assert_eq!(
        eval(&model, &vocab, &f_test_f_id, &generous_budget()),
        Ok(true)
    );

    // Eq(f_choose(c0, c1), c0) -> verdadeiro
    let f_test_choose = form(vec![FormulaNode::Eq(
        term_app2(
            f_choose_sym.clone(),
            term_const(c0.clone()),
            term_const(c1.clone()),
        ),
        term_const(c0.clone()),
    )]);
    assert_eq!(
        eval(&model, &vocab, &f_test_choose, &generous_budget()),
        Ok(true)
    );
}

#[test]
fn test_eval_chained_definitions() {
    let s = sort("S");
    let c0 = const_sym("c0", "S");

    // f1(x) := id(x): Var(0) = Var(1)
    let f1_sym = func_sym("f1", vec![s.clone()], "S");
    let f1_body = form(vec![FormulaNode::Eq(term_var(0, "S"), term_var(1, "S"))]);

    // f2(x) := f1(x): Var(0) = f1(Var(1))
    let f2_sym = func_sym("f2", vec![s.clone()], "S");
    let f2_body = form(vec![FormulaNode::Eq(
        term_var(0, "S"),
        term_app1(f1_sym.clone(), term_var(1, "S")),
    )]);

    // p1(x) := x = c0: Var(0) = c0
    let p1_sym = pred_sym("p1", vec![s.clone()]);
    let p1_body = form(vec![FormulaNode::Eq(
        term_var(0, "S"),
        term_const(c0.clone()),
    )]);

    // p2(x) := p1(f2(x))
    let p2_sym = pred_sym("p2", vec![s.clone()]);
    let p2_body = form(vec![FormulaNode::Pred {
        symbol: p1_sym.clone(),
        args: vec![term_app1(f2_sym.clone(), term_var(0, "S"))],
    }]);

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![
            primitive(c0.clone()),
            defined(f1_sym.clone(), f1_body),
            defined(f2_sym.clone(), f2_body),
            defined(p1_sym.clone(), p1_body),
            defined(p2_sym.clone(), p2_body),
        ],
    )
    .expect("vocab");

    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: vec![(0, 0)],
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // p2(c0) -> verdadeiro pois f2(c0) = c0 e p1(c0) = true
    let f_test = form(vec![FormulaNode::Pred {
        symbol: p2_sym.clone(),
        args: vec![term_const(c0.clone())],
    }]);
    assert_eq!(
        eval(&model, &vocab, &f_test, &generous_budget()),
        Ok(true),
        "cadeia de definições aninhadas deve avaliar corretamente"
    );
}

// ============================================================================
// 5. Erros de Definição Inválida (InvalidDefinition, inclusive não utilizadas)
// ============================================================================

#[test]
fn test_err_invalid_definition_const_zero_results() {
    let c0 = const_sym("c0", "S");

    // Constante definida com corpo False (0 resultados possíveis)
    let c_bad = const_sym("c_bad", "S");
    let c_bad_body = form(vec![FormulaNode::False]);

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![primitive(c0.clone()), defined(c_bad.clone(), c_bad_body)],
    )
    .expect("vocab");

    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: vec![(0, 0)],
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // Fórmula é True e NÃO menciona c_bad!
    let formula = form(vec![FormulaNode::True]);
    assert_eq!(
        eval(&model, &vocab, &formula, &generous_budget()),
        Err(EvalError::InvalidDefinition { symbol: 1 }),
        "constante definida sem resultados deve retornar InvalidDefinition mesmo que não utilizada"
    );
}

#[test]
fn test_err_invalid_definition_const_multiple_results() {
    let c0 = const_sym("c0", "S");

    // Constante definida com corpo True sobre domínio de tamanho 2 (2 resultados possíveis)
    let c_bad = const_sym("c_bad", "S");
    let c_bad_body = form(vec![FormulaNode::True]);

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![primitive(c0.clone()), defined(c_bad.clone(), c_bad_body)],
    )
    .expect("vocab");

    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: vec![(0, 0)],
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    let formula = form(vec![FormulaNode::True]);
    assert_eq!(
        eval(&model, &vocab, &formula, &generous_budget()),
        Err(EvalError::InvalidDefinition { symbol: 1 }),
        "constante definida com múltiplos resultados deve retornar InvalidDefinition"
    );
}

#[test]
fn test_err_invalid_definition_func_partial_and_multivalued() {
    let s = sort("S");
    let c0 = const_sym("c0", "S");

    // 1. Função parcial: f(x) só tem resultado para x = c0 (quando Var(1) = c0 e Var(0) = c0).
    // Para x = 1, não há nenhum Var(0) que satisfaça a relação.
    let f_partial = func_sym("f_partial", vec![s.clone()], "S");
    let f_partial_body = form(vec![
        FormulaNode::And(2),
        FormulaNode::Eq(term_var(1, "S"), term_const(c0.clone())),
        FormulaNode::Eq(term_var(0, "S"), term_const(c0.clone())),
    ]);

    let vocab_p = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![
            primitive(c0.clone()),
            defined(f_partial.clone(), f_partial_body),
        ],
    )
    .expect("vocab");

    let model_data_p = ModelData {
        vocab: hash(&vocab_p).expect("hash"),
        domains: vec![2],
        consts: vec![(0, 0)],
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model_p = FiniteModel::new(model_data_p, &vocab_p, ModelLimits::default()).expect("model");
    let formula = form(vec![FormulaNode::True]);

    assert_eq!(
        eval(&model_p, &vocab_p, &formula, &generous_budget()),
        Err(EvalError::InvalidDefinition { symbol: 1 }),
        "função definida parcial deve retornar InvalidDefinition"
    );

    // 2. Função multivalorada: para x = c0, qualquer Var(0) satisfaz a relação
    let f_multi = func_sym("f_multi", vec![s.clone()], "S");
    let f_multi_body = form(vec![FormulaNode::True]);

    let vocab_m = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![
            primitive(c0.clone()),
            defined(f_multi.clone(), f_multi_body),
        ],
    )
    .expect("vocab");

    let model_data_m = ModelData {
        vocab: hash(&vocab_m).expect("hash"),
        domains: vec![2],
        consts: vec![(0, 0)],
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model_m = FiniteModel::new(model_data_m, &vocab_m, ModelLimits::default()).expect("model");

    assert_eq!(
        eval(&model_m, &vocab_m, &formula, &generous_budget()),
        Err(EvalError::InvalidDefinition { symbol: 1 }),
        "função definida multivalorada deve retornar InvalidDefinition"
    );
}

// ============================================================================
// 6. Erro Sort em Corpos de Definição
// ============================================================================

#[test]
fn test_err_sort_in_defined_body() {
    let sa = sort("A");

    // Predicado com aridade [A], mas corpo compara Var(0, A) com Var(0, B)
    let p_bad = pred_sym("p_bad", vec![sa.clone()]);
    let p_bad_body = form(vec![FormulaNode::Eq(term_var(0, "A"), term_var(0, "B"))]);

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("A"), sort_decl_open("B")],
        vec![defined(p_bad.clone(), p_bad_body)],
    )
    .expect("vocab");

    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2, 2],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");
    let formula = form(vec![FormulaNode::True]);

    assert!(
        matches!(
            eval(&model, &vocab, &formula, &generous_budget()),
            Err(EvalError::Sort(_))
        ),
        "corpo de definição com sort incorreto deve retornar EvalError::Sort"
    );
}

// ============================================================================
// 7. Mismatch de Vocabulário h(V)
// ============================================================================

#[test]
fn test_err_vocabulary_mismatch() {
    let vocab1 = Vocabulary::new(1, vec![sort_decl_open("S")], Vec::new()).expect("vocab1");
    let vocab2 = Vocabulary::new(2, vec![sort_decl_open("S")], Vec::new()).expect("vocab2");

    let model_data = ModelData {
        vocab: hash(&vocab1).expect("hash"),
        domains: vec![2],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab1, ModelLimits::default()).expect("model");

    let formula = form(vec![FormulaNode::True]);
    assert_eq!(
        eval(&model, &vocab2, &formula, &generous_budget()),
        Err(EvalError::VocabularyMismatch),
        "modelo avaliado contra vocabulário com hash divergente deve retornar VocabularyMismatch"
    );
}

// ============================================================================
// 8. Enumeração Definida Incompatível (inclusive não utilizada)
// ============================================================================

#[test]
fn test_err_enumeration_mismatch_used_and_unused() {
    let e0 = const_sym("e0", "E"); // ordinal 0
    let e1 = const_sym("e1", "E"); // ordinal 1

    // e0 é definido relacionalmente como e1 (valor 1 != ordinal 0)
    let e0_bad_body = form(vec![FormulaNode::Eq(
        term_var(0, "E"),
        term_const(e1.clone()),
    )]);

    let enum_decl = sort_decl_enum("E", vec![e0.clone(), e1.clone()], 42);
    let vocab = Vocabulary::new(
        1,
        vec![enum_decl],
        vec![defined(e0.clone(), e0_bad_body), primitive(e1.clone())],
    )
    .expect("vocab");

    // Modelo com cardinalidade 2 para o sort enumerado E
    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: vec![(1, 1)], // apenas e1 é primitivo, com valor 1
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // 1. Teste com fórmula não utilizada (formula é True)
    let formula_unused = form(vec![FormulaNode::True]);
    assert_eq!(
        eval(&model, &vocab, &formula_unused, &generous_budget()),
        Err(EvalError::EnumerationMismatch { symbol: 0 }),
        "constante enumerada definida discordante deve retornar EnumerationMismatch mesmo não utilizada"
    );

    // 2. Teste com fórmula que usa a constante
    let formula_used = form(vec![FormulaNode::Eq(
        term_const(e0.clone()),
        term_const(e0.clone()),
    )]);
    assert_eq!(
        eval(&model, &vocab, &formula_used, &generous_budget()),
        Err(EvalError::EnumerationMismatch { symbol: 0 }),
        "constante enumerada definida discordante deve retornar EnumerationMismatch"
    );
}

// ============================================================================
// 9. Orçamento S2: Unknown(Steps) Preservado em Fórmulas True e False
// ============================================================================

#[test]
fn test_budget_unknown_steps_preserved_on_true_and_false() {
    let s = sort("S");
    let vocab = Vocabulary::new(1, vec![sort_decl_open("S")], Vec::new()).expect("vocab");
    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![3],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // Fórmula verdadeira: ∀x (x = x)
    let f_true = form(vec![
        FormulaNode::Forall(s.clone()),
        FormulaNode::Eq(term_var(0, "S"), term_var(0, "S")),
    ]);
    assert_eq!(eval(&model, &vocab, &f_true, &generous_budget()), Ok(true));

    // Fórmula falsa: ∀x False
    let f_false = form(vec![FormulaNode::Forall(s.clone()), FormulaNode::False]);
    assert_eq!(
        eval(&model, &vocab, &f_false, &generous_budget()),
        Ok(false)
    );

    // Com 0 passos, AMBAS devem retornar Unknown(Steps), NUNCA true nem false
    let budget_zero = Budget::new(0, 10_000_000, 10_000_000, 10_000);
    assert_eq!(
        eval(&model, &vocab, &f_true, &budget_zero),
        Err(EvalError::Unknown(Exhausted::Steps)),
        "fórmula verdadeira com 0 passos não pode virar true"
    );
    assert_eq!(
        eval(&model, &vocab, &f_false, &budget_zero),
        Err(EvalError::Unknown(Exhausted::Steps)),
        "fórmula falsa com 0 passos não pode virar false"
    );

    // Com passos parciais insuficientes (ex: 1, 2, 3), o medidor deve preservar Unknown(Steps)
    for steps in [1, 2, 3, 4] {
        let budget_insufficient = Budget::new(steps, 10_000_000, 10_000_000, 10_000);
        assert_eq!(
            eval(&model, &vocab, &f_true, &budget_insufficient),
            Err(EvalError::Unknown(Exhausted::Steps))
        );
        assert_eq!(
            eval(&model, &vocab, &f_false, &budget_insufficient),
            Err(EvalError::Unknown(Exhausted::Steps))
        );
    }
}

// ============================================================================
// 10. Fronteira Exata de Passos e 1000 Repetições S3 (Sem Wall Clock)
// ============================================================================

#[test]
fn test_exact_step_boundary() {
    let s = sort("S");
    let vocab = Vocabulary::new(1, vec![sort_decl_open("S")], Vec::new()).expect("vocab");
    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // 1. True/False sem símbolos:
    // Preflight: 1 para h(V) + 1 para formula.node_count() = 2 passos.
    // Avaliação: 1 passo para o nó True/False.
    // Total exato = 3 passos.
    let f_true = form(vec![FormulaNode::True]);
    assert_eq!(
        eval(
            &model,
            &vocab,
            &f_true,
            &Budget::new(3, 10_000_000, 10_000_000, 10_000)
        ),
        Ok(true),
        "3 passos devem ser exatamente suficientes para True"
    );
    assert_eq!(
        eval(
            &model,
            &vocab,
            &f_true,
            &Budget::new(2, 10_000_000, 10_000_000, 10_000)
        ),
        Err(EvalError::Unknown(Exhausted::Steps)),
        "2 passos devem esgotar para True"
    );

    let f_false = form(vec![FormulaNode::False]);
    assert_eq!(
        eval(
            &model,
            &vocab,
            &f_false,
            &Budget::new(3, 10_000_000, 10_000_000, 10_000)
        ),
        Ok(false),
        "3 passos devem ser exatamente suficientes para False"
    );
    assert_eq!(
        eval(
            &model,
            &vocab,
            &f_false,
            &Budget::new(2, 10_000_000, 10_000_000, 10_000)
        ),
        Err(EvalError::Unknown(Exhausted::Steps)),
        "2 passos devem esgotar para False"
    );

    // 2. Forall(S) True sobre domínio de tamanho n=2 sem símbolos:
    // Fórmula: 4 + 2*n = 4 + 4 = 8 passos exatos.
    // (h(V)=1, formula.node_count()=2, Forall=1, 2 iterações com (iteração=1, True=1)).
    let f_forall_true = form(vec![FormulaNode::Forall(s.clone()), FormulaNode::True]);
    assert_eq!(
        eval(
            &model,
            &vocab,
            &f_forall_true,
            &Budget::new(8, 10_000_000, 10_000_000, 10_000)
        ),
        Ok(true),
        "8 passos devem ser exatamente suficientes para Forall(S) True em domínio de tamanho 2"
    );
    assert_eq!(
        eval(
            &model,
            &vocab,
            &f_forall_true,
            &Budget::new(7, 10_000_000, 10_000_000, 10_000)
        ),
        Err(EvalError::Unknown(Exhausted::Steps)),
        "7 passos devem esgotar para Forall(S) True em domínio de tamanho 2"
    );
}

#[test]
fn test_1000_repeats_s3_no_wall_reliance() {
    let s = sort("S");
    let c0 = const_sym("c0", "S");
    let c1 = const_sym("c1", "S");
    let p = pred_sym("P", vec![s.clone()]);

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![
            primitive(c0.clone()),
            primitive(c1.clone()),
            primitive(p.clone()),
        ],
    )
    .expect("vocab");

    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: vec![(0, 0), (1, 1)],
        funcs: Vec::new(),
        preds: vec![(2, vec![vec![0]])],
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    let formula = form(vec![
        FormulaNode::And(2),
        FormulaNode::Pred {
            symbol: p.clone(),
            args: vec![term_const(c0.clone())],
        },
        FormulaNode::Not,
        FormulaNode::Pred {
            symbol: p.clone(),
            args: vec![term_const(c1.clone())],
        },
    ]);

    let budget = Budget::new(10_000_000, 0, 0, 0);
    let false_formula = form(vec![FormulaNode::Not, FormulaNode::And(0)]);
    let exhausted = Budget::new(1, 0, 0, 0);

    // 1000 repetições exatas (S3): resultado estritamente determinístico
    for _ in 0..1000 {
        let res = eval(&model, &vocab, &formula, &budget);
        assert_eq!(res, Ok(true));
        assert_eq!(eval(&model, &vocab, &false_formula, &budget), Ok(false));
        assert_eq!(
            eval(&model, &vocab, &formula, &exhausted),
            Err(EvalError::Unknown(Exhausted::Steps))
        );
    }
}

// ============================================================================
// 11. Interpretador com Pilha Explícita em Thread com Pilha de 64 KiB
// ============================================================================

#[test]
fn test_deep_stack_explicit_interpreter() {
    // Constrói uma fórmula com 500 negações aninhadas: Not(Not(...(True)...))
    // Com 500 negações, o resultado final deve ser True (paridade par).
    // Usamos limites customizados permitindo profundidade de 1000.
    let depth = 500;
    let mut nodes = Vec::with_capacity(depth + 1);
    for _ in 0..depth {
        nodes.push(FormulaNode::Not);
    }
    nodes.push(FormulaNode::True);

    let deep_limits = Limits {
        max_depth: 1000,
        max_nodes: 100_000,
    };
    let formula = Formula::new(nodes, deep_limits).expect("deep formula");

    let vocab = Vocabulary::new(1, vec![sort_decl_open("S")], Vec::new()).expect("vocab");
    let model_data = ModelData {
        vocab: hash(&vocab).expect("hash"),
        domains: vec![2],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(model_data, &vocab, ModelLimits::default()).expect("model");

    // Executa em thread com pilha restrita de 64 KiB para garantir que o interpretador
    // com pilhas explícitas não transborde a pilha nativa do Rust.
    let handle = std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(move || {
            let res = eval(&model, &vocab, &formula, &generous_budget());
            assert_eq!(
                res,
                Ok(true),
                "500 negações aninhadas devem avaliar para true em pilha de 64 KiB"
            );
        })
        .expect("spawn thread");

    handle.join().expect("thread join");
}

#[test]
fn defined_validation_exhaustion_never_becomes_boolean() {
    let c = const_sym("c", "S");
    let d = const_sym("d", "S");
    let body = form(vec![FormulaNode::Eq(
        term_var(0, "S"),
        term_const(c.clone()),
    )]);
    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![primitive(c), defined(d, body)],
    )
    .expect("vocab");
    let model = FiniteModel::new(
        ModelData {
            vocab: hash(&vocab).expect("hash"),
            domains: vec![2],
            consts: vec![(0, 1)],
            funcs: vec![],
            preds: vec![],
        },
        &vocab,
        ModelLimits::default(),
    )
    .expect("model");
    let formula = form(vec![FormulaNode::True]);
    for steps in 0..12 {
        assert_eq!(
            eval(&model, &vocab, &formula, &Budget::new(steps, 0, 0, 0)),
            Err(EvalError::Unknown(Exhausted::Steps))
        );
    }
    assert_eq!(
        eval(&model, &vocab, &formula, &Budget::new(1000, 0, 0, 0)),
        Ok(true)
    );
}

#[test]
fn ill_typed_final_formula_is_not_a_value() {
    let vocab = Vocabulary::new(1, vec![sort_decl_open("S")], vec![]).expect("vocab");
    let model = FiniteModel::new(
        ModelData {
            vocab: hash(&vocab).expect("hash"),
            domains: vec![1],
            consts: vec![],
            funcs: vec![],
            preds: vec![],
        },
        &vocab,
        ModelLimits::default(),
    )
    .expect("model");
    let free = form(vec![FormulaNode::Eq(
        term_var(u64::MAX, "S"),
        term_var(0, "S"),
    )]);
    assert!(matches!(
        eval(&model, &vocab, &free, &generous_budget()),
        Err(EvalError::Sort(_))
    ));
}

#[test]
fn deep_terms_and_definition_chain_do_not_use_native_recursion() {
    let c = const_sym("c", "S");
    let f = func_sym("f", vec![sort("S")], "S");
    let limits = Limits {
        max_depth: 1000,
        max_nodes: 100_000,
    };
    let mut declarations = vec![primitive(c.clone()), primitive(f.clone())];
    let mut previous = None;
    for index in 0..250 {
        let symbol = pred_sym(&format!("D{index}"), vec![]);
        let body = form(vec![previous.map_or(FormulaNode::True, |symbol| {
            FormulaNode::Pred {
                symbol,
                args: vec![],
            }
        })]);
        declarations.push(defined(symbol.clone(), body));
        previous = Some(symbol);
    }
    let vocab = Vocabulary::new(1, vec![sort_decl_open("S")], declarations).expect("vocab");
    let model = FiniteModel::new(
        ModelData {
            vocab: hash(&vocab).expect("hash"),
            domains: vec![1],
            consts: vec![(0, 0)],
            funcs: vec![(1, vec![0])],
            preds: vec![],
        },
        &vocab,
        ModelLimits::default(),
    )
    .expect("model");
    let mut term_nodes = vec![TermNode::App { symbol: f, args: 1 }; 500];
    term_nodes.push(TermNode::App {
        symbol: c.clone(),
        args: 0,
    });
    let term = Term::new(term_nodes, limits).expect("term");
    let formula = Formula::new(
        vec![
            FormulaNode::And(2),
            FormulaNode::Eq(term, term_const(c)),
            FormulaNode::Pred {
                symbol: previous.expect("chain"),
                args: vec![],
            },
        ],
        limits,
    )
    .expect("formula");
    std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(move || {
            assert_eq!(eval(&model, &vocab, &formula, &generous_budget()), Ok(true));
        })
        .expect("thread")
        .join()
        .expect("join");
}

#[test]
fn compatible_defined_enumeration_and_nullary_function_are_evaluated() {
    let e0 = const_sym("e0", "E");
    let e1 = const_sym("e1", "E");
    let f = func_sym("f", vec![], "E");
    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_enum("E", vec![e0.clone(), e1.clone()], 7)],
        vec![
            defined(
                e0.clone(),
                form(vec![
                    FormulaNode::Not,
                    FormulaNode::Eq(term_var(0, "E"), term_const(e1.clone())),
                ]),
            ),
            primitive(e1.clone()),
            defined(
                f.clone(),
                form(vec![FormulaNode::Eq(
                    term_var(0, "E"),
                    term_const(e1.clone()),
                )]),
            ),
        ],
    )
    .expect("vocab");
    let model = FiniteModel::new(
        ModelData {
            vocab: hash(&vocab).expect("hash"),
            domains: vec![2],
            consts: vec![(1, 1)],
            funcs: vec![],
            preds: vec![],
        },
        &vocab,
        ModelLimits::default(),
    )
    .expect("model");
    let formula = form(vec![
        FormulaNode::And(2),
        FormulaNode::Not,
        FormulaNode::Eq(term_const(e0), term_const(e1.clone())),
        FormulaNode::Eq(term_const(f), term_const(e1)),
    ]);
    assert_eq!(eval(&model, &vocab, &formula, &generous_budget()), Ok(true));
}

#[test]
fn nested_defined_value_completes_primitive_parent() {
    let c0 = const_sym("c0", "S");
    let c1 = const_sym("c1", "S");
    let f = func_sym("f", vec![sort("S")], "S");
    let g = func_sym("g", vec![sort("S")], "S");
    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![
            primitive(c0.clone()),
            primitive(c1.clone()),
            primitive(f.clone()),
            defined(
                g.clone(),
                form(vec![FormulaNode::Eq(term_var(0, "S"), term_var(1, "S"))]),
            ),
        ],
    )
    .expect("vocab");
    let model = FiniteModel::new(
        ModelData {
            vocab: hash(&vocab).expect("hash"),
            domains: vec![2],
            consts: vec![(0, 0), (1, 1)],
            funcs: vec![(2, vec![1, 0])],
            preds: vec![],
        },
        &vocab,
        ModelLimits::default(),
    )
    .expect("model");
    let formula = form(vec![FormulaNode::Eq(
        term_app1(f, term_app1(g, term_const(c0))),
        term_const(c1),
    )]);
    assert_eq!(eval(&model, &vocab, &formula, &generous_budget()), Ok(true));
}

#[test]
fn primitive_nullary_function_uses_function_table() {
    let c = const_sym("c", "S");
    let f = func_sym("f", vec![], "S");
    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![primitive(c.clone()), primitive(f.clone())],
    )
    .expect("vocab");
    let model = FiniteModel::new(
        ModelData {
            vocab: hash(&vocab).expect("hash"),
            domains: vec![2],
            consts: vec![(0, 1)],
            funcs: vec![(1, vec![1])],
            preds: vec![],
        },
        &vocab,
        ModelLimits::default(),
    )
    .expect("model");
    let formula = form(vec![FormulaNode::Eq(term_const(f), term_const(c))]);
    assert_eq!(eval(&model, &vocab, &formula, &generous_budget()), Ok(true));
}
