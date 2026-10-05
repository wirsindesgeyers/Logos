//! Exercício real da API pública de logos-model: `cargo run -p logos-model --example eval_smoke`.

use std::error::Error;

use logos_air::{
    Formula, FormulaNode, Grounding, Limits, Sort, SortDecl, SortDomain, Symbol, SymbolDecl,
    SymbolKind, SymbolResult, Term, TermNode, Vocabulary,
};
use logos_canon::Text;
use logos_core::{Budget, Exhausted};
use logos_crypto::hash;
use logos_model::{EvalError, FiniteModel, ModelData, ModelLimits, eval};

fn require(condition: bool, message: &str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

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

fn const_sym(name: &str, sort_name: &str) -> Symbol {
    Symbol {
        name: Text::new(name).expect("symbol text"),
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(sort(sort_name)),
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

fn generous_budget() -> Budget {
    Budget::new(1_000_000, 10_000_000, 10_000_000, 10_000)
}

fn main() -> Result<(), Box<dyn Error>> {
    let s = sort("S");
    let c0 = const_sym("c0", "S");
    let c1 = const_sym("c1", "S");
    let p = pred_sym("P", vec![s.clone()]);

    // Cenário 1: Vocabulário com constantes e predicado primitivo
    let vocab1 = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![
            primitive(c0.clone()),
            primitive(c1.clone()),
            primitive(p.clone()),
        ],
    )?;
    let h_v1 = hash(&vocab1)?;
    let limits = ModelLimits::default();

    // Modelo com domínio de tamanho 2: {0, 1}
    // c0 = 0, c1 = 1, P = { (0) } (verdadeiro apenas para 0)
    let model1_data = ModelData {
        vocab: h_v1,
        domains: vec![2],
        consts: vec![(0, 0), (1, 1)],
        funcs: Vec::new(),
        preds: vec![(2, vec![vec![0]])],
    };
    let model1 = FiniteModel::new(model1_data, &vocab1, limits)?;

    // 1. Cenário TRUE: P(c0) deve ser verdadeiro
    let formula_true = Formula::new(
        vec![FormulaNode::Pred {
            symbol: p.clone(),
            args: vec![Term::new(
                vec![TermNode::App {
                    symbol: c0.clone(),
                    args: 0,
                }],
                Limits::default(),
            )?],
        }],
        Limits::default(),
    )?;
    let res_true = eval(&model1, &vocab1, &formula_true, &generous_budget())?;
    require(res_true, "P(c0) deve ser verdadeiro")?;

    // 2. Cenário FALSE: P(c1) deve ser falso
    let formula_false = Formula::new(
        vec![FormulaNode::Pred {
            symbol: p.clone(),
            args: vec![Term::new(
                vec![TermNode::App {
                    symbol: c1.clone(),
                    args: 0,
                }],
                Limits::default(),
            )?],
        }],
        Limits::default(),
    )?;
    let res_false = eval(&model1, &vocab1, &formula_false, &generous_budget())?;
    require(!res_false, "P(c1) deve ser falso")?;

    // 3. Cenário DEFINED: Predicado definido Q(x) := Eq(x, c0)
    let q_def_sym = pred_sym("Q", vec![s.clone()]);
    let q_def_body = Formula::new(
        vec![FormulaNode::Eq(
            Term::new(
                vec![TermNode::Var {
                    index: 0,
                    sort: s.clone(),
                }],
                Limits::default(),
            )?,
            Term::new(
                vec![TermNode::App {
                    symbol: c0.clone(),
                    args: 0,
                }],
                Limits::default(),
            )?,
        )],
        Limits::default(),
    )?;
    let q_decl = SymbolDecl {
        signature: q_def_sym.clone(),
        grounding: Grounding::Defined(q_def_body),
        gloss_pt: Text::new("Q definido").expect("gloss"),
    };

    let vocab2 = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![
            primitive(c0.clone()),
            primitive(c1.clone()),
            primitive(p.clone()),
            q_decl,
        ],
    )?;
    let h_v2 = hash(&vocab2)?;
    let model2_data = ModelData {
        vocab: h_v2,
        domains: vec![2],
        consts: vec![(0, 0), (1, 1)],
        funcs: Vec::new(),
        preds: vec![(2, vec![vec![0]])],
    };
    let model2 = FiniteModel::new(model2_data, &vocab2, limits)?;

    // Q(c0) usando predicado definido deve avaliar para verdadeiro
    let formula_defined = Formula::new(
        vec![FormulaNode::Pred {
            symbol: q_def_sym.clone(),
            args: vec![Term::new(
                vec![TermNode::App {
                    symbol: c0.clone(),
                    args: 0,
                }],
                Limits::default(),
            )?],
        }],
        Limits::default(),
    )?;
    let res_defined = eval(&model2, &vocab2, &formula_defined, &generous_budget())?;
    require(res_defined, "Q(c0) definido deve ser verdadeiro")?;

    // 4. Cenário UNKNOWN: Orçamento com 0 passos deve esgotar
    let exhausted_budget = Budget::new(0, 10_000_000, 10_000_000, 10_000);
    let res_unknown = eval(&model1, &vocab1, &formula_true, &exhausted_budget);
    require(
        res_unknown == Err(EvalError::Unknown(Exhausted::Steps)),
        "avaliação com 0 passos deve retornar Unknown(Steps)",
    )?;

    // 5. Cenário INVALID: Definição não funcional (constante definida com 0 resultados)
    let c_bad_sym = const_sym("c_bad", "S");
    let c_bad_body = Formula::new(vec![FormulaNode::False], Limits::default())?;
    let c_bad_decl = SymbolDecl {
        signature: c_bad_sym,
        grounding: Grounding::Defined(c_bad_body),
        gloss_pt: Text::new("c_bad invalido").expect("gloss"),
    };
    let vocab_bad = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![primitive(c0.clone()), c_bad_decl],
    )?;
    let h_vbad = hash(&vocab_bad)?;
    let model_bad_data = ModelData {
        vocab: h_vbad,
        domains: vec![2],
        consts: vec![(0, 0)],
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model_bad = FiniteModel::new(model_bad_data, &vocab_bad, limits)?;
    let unrelated_true = Formula::new(vec![FormulaNode::True], Limits::default())?;
    let res_invalid = eval(&model_bad, &vocab_bad, &unrelated_true, &generous_budget());
    require(
        matches!(res_invalid, Err(EvalError::InvalidDefinition { .. })),
        "definição não funcional deve retornar InvalidDefinition",
    )?;

    // 6. Cenário INVALID: VocabularyMismatch
    let res_mismatch = eval(&model1, &vocab2, &formula_true, &generous_budget());
    require(
        res_mismatch == Err(EvalError::VocabularyMismatch),
        "modelo com hash de outro vocabulário deve retornar VocabularyMismatch",
    )?;

    println!("EVAL_SMOKE_OK");
    Ok(())
}
