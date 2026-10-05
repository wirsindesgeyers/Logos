//! Exercício real da API pública de Sort-Check: `cargo run -p logos-air --example sort_check_smoke`.

use std::error::Error;

use logos_air::{
    Formula, FormulaNode, Limits, Sort, SortDecl, SortDomain, SortError, SortErrorKind, SortPath,
    Symbol, SymbolDecl, SymbolKind, SymbolResult, Term, TermNode, TermPath, TermPosition,
    Vocabulary, check,
};
use logos_canon::{Text, from_canon, to_canon};

fn require(condition: bool, message: &str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Construir vocabulário com múltiplos sorts e símbolos
    let sort_user = Sort {
        name: Text::new("User")?,
    };
    let sort_nat = Sort {
        name: Text::new("Nat")?,
    };

    let decl_sort_user = SortDecl {
        name: sort_user.name.clone(),
        domain: SortDomain::Open,
    };
    let decl_sort_nat = SortDecl {
        name: sort_nat.name.clone(),
        domain: SortDomain::Open,
    };

    let sym_zero = Symbol {
        name: Text::new("zero")?,
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(sort_nat.clone()),
    };
    let decl_zero = SymbolDecl {
        signature: sym_zero.clone(),
        grounding: logos_air::Grounding::Glossed(Text::new("Constante zero natural")?),
        gloss_pt: Text::new("Zero")?,
    };

    let sym_succ = Symbol {
        name: Text::new("succ")?,
        kind: SymbolKind::Func,
        arity: vec![sort_nat.clone()],
        result: SymbolResult::Sort(sort_nat.clone()),
    };
    let decl_succ = SymbolDecl {
        signature: sym_succ.clone(),
        grounding: logos_air::Grounding::Glossed(Text::new("Função sucessor natural")?),
        gloss_pt: Text::new("Sucessor")?,
    };

    let sym_admin = Symbol {
        name: Text::new("admin")?,
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(sort_user.clone()),
    };
    let decl_admin = SymbolDecl {
        signature: sym_admin.clone(),
        grounding: logos_air::Grounding::Glossed(Text::new("Usuário administrador padrão")?),
        gloss_pt: Text::new("Administrador")?,
    };

    let sym_is_active = Symbol {
        name: Text::new("is_active")?,
        kind: SymbolKind::Pred,
        arity: vec![sort_user.clone()],
        result: SymbolResult::Bool,
    };
    let decl_is_active = SymbolDecl {
        signature: sym_is_active.clone(),
        grounding: logos_air::Grounding::Glossed(Text::new("Predicado de conta ativa")?),
        gloss_pt: Text::new("Ativo")?,
    };

    let sym_has_score = Symbol {
        name: Text::new("has_score")?,
        kind: SymbolKind::Pred,
        arity: vec![sort_user.clone(), sort_nat.clone()],
        result: SymbolResult::Bool,
    };
    let decl_has_score = SymbolDecl {
        signature: sym_has_score.clone(),
        grounding: logos_air::Grounding::Glossed(Text::new("Pontuação associada ao usuário")?),
        gloss_pt: Text::new("Tem pontuação")?,
    };

    let vocab_raw = Vocabulary::new(
        1,
        vec![decl_sort_user, decl_sort_nat],
        vec![
            decl_zero,
            decl_succ.clone(),
            decl_admin,
            decl_is_active.clone(),
            decl_has_score.clone(),
        ],
    )?;

    // Serializar e deserializar Vocabulário via canon/v1 para exercitar fronteira de bytes
    let vocab_bytes = to_canon(&vocab_raw)?;
    let vocab: Vocabulary = from_canon(&vocab_bytes)?;

    // 2. Caso Positivo: Fórmula fechada e bem tipada
    // ∀u:User. ∃n:Nat. (has_score(u, n) ∧ n = succ(zero))
    //
    // Records de fórmula:
    // 0: Forall(User)
    // 1: Exists(Nat)
    // 2: And(2)
    // 3: Pred has_score(Var(1, User), Var(0, Nat))
    // 4: Eq(Var(0, Nat), succ(zero))
    let term_u_var = Term::new(
        vec![TermNode::Var {
            index: 1,
            sort: sort_user.clone(),
        }],
        Limits::default(),
    )?;
    let term_n_var = Term::new(
        vec![TermNode::Var {
            index: 0,
            sort: sort_nat.clone(),
        }],
        Limits::default(),
    )?;
    let term_succ_zero = Term::new(
        vec![
            TermNode::App {
                symbol: sym_succ.clone(),
                args: 1,
            },
            TermNode::App {
                symbol: sym_zero.clone(),
                args: 0,
            },
        ],
        Limits::default(),
    )?;

    let valid_formula_raw = Formula::new(
        vec![
            FormulaNode::Forall(sort_user.clone()),
            FormulaNode::Exists(sort_nat.clone()),
            FormulaNode::And(2),
            FormulaNode::Pred {
                symbol: sym_has_score.clone(),
                args: vec![term_u_var, term_n_var.clone()],
            },
            FormulaNode::Eq(term_n_var, term_succ_zero),
        ],
        Limits::default(),
    )?;

    // Serializar e deserializar Fórmula via canon/v1
    let formula_bytes = to_canon(&valid_formula_raw)?;
    let valid_formula: Formula = from_canon(&formula_bytes)?;

    let positive_outcome = check(&vocab, &valid_formula);
    require(
        positive_outcome == Ok(()),
        "fórmula fechada e bem tipada deve retornar Ok(()) no sort-check",
    )?;
    println!(
        "0. Positive check: fórmula fechada ∀u.∃n.(has_score(u,n) ∧ n=succ(zero)) aceita com sucesso."
    );

    // 3. Caso Negativo 1: SortMismatch em igualdade (Eq) com path conhecido
    // ∀u:User. u = zero
    // 0: Forall(User)
    // 1: Eq(Var(0, User), zero) -> zero tem sort Nat, divergindo do sort esperado User
    let term_user_0 = Term::new(
        vec![TermNode::Var {
            index: 0,
            sort: sort_user.clone(),
        }],
        Limits::default(),
    )?;
    let term_zero_nat = Term::new(
        vec![TermNode::App {
            symbol: sym_zero.clone(),
            args: 0,
        }],
        Limits::default(),
    )?;
    let neg1_formula = Formula::new(
        vec![
            FormulaNode::Forall(sort_user.clone()),
            FormulaNode::Eq(term_user_0, term_zero_nat),
        ],
        Limits::default(),
    )?;
    let neg1_err = check(&vocab, &neg1_formula).err();
    let expected_neg1 = SortError {
        path: SortPath {
            formula_node: 1,
            term: Some(TermPath {
                position: TermPosition::EqualityRight,
                node: 0,
            }),
        },
        kind: SortErrorKind::SortMismatch {
            expected: sort_user.clone(),
            actual: sort_nat.clone(),
        },
    };
    require(
        neg1_err.as_ref() == Some(&expected_neg1),
        "mismatch de sort na raiz do termo direito de Eq deve produzir erro e path exatos",
    )?;
    println!(
        "1. Negative check (SortMismatch): diagnostic=\"{}\" (path={:?})",
        neg1_err.as_ref().ok_or("expected equality error")?,
        expected_neg1.path
    );

    // 4. Caso Negativo 2: UnboundVariable (variável livre) com path conhecido
    // is_active(Var(0, User)) sem quantificador
    // 0: Pred is_active(Var(0, User)) -> binders ativos: 0, index: 0
    let free_var_term = Term::new(
        vec![TermNode::Var {
            index: 0,
            sort: sort_user.clone(),
        }],
        Limits::default(),
    )?;
    let neg2_formula = Formula::new(
        vec![FormulaNode::Pred {
            symbol: sym_is_active.clone(),
            args: vec![free_var_term],
        }],
        Limits::default(),
    )?;
    let neg2_err = check(&vocab, &neg2_formula).err();
    let expected_neg2 = SortError {
        path: SortPath {
            formula_node: 0,
            term: Some(TermPath {
                position: TermPosition::PredicateArgument(0),
                node: 0,
            }),
        },
        kind: SortErrorKind::UnboundVariable {
            index: 0,
            binders: 0,
        },
    };
    require(
        neg2_err.as_ref() == Some(&expected_neg2),
        "variável livre sem binder deve produzir UnboundVariable na posição exata",
    )?;
    println!(
        "2. Negative check (UnboundVariable): diagnostic=\"{}\" (path={:?})",
        neg2_err.as_ref().ok_or("expected binder error")?,
        expected_neg2.path
    );

    // 5. Caso Negativo 3: ArityMismatch em predicado com path no record da fórmula
    // ∀u:User. has_score(u) -> esperado 2 argumentos, fornecido 1
    // 0: Forall(User)
    // 1: Pred has_score(Var(0, User))
    let user_arg = Term::new(
        vec![TermNode::Var {
            index: 0,
            sort: sort_user.clone(),
        }],
        Limits::default(),
    )?;
    let neg3_formula = Formula::new(
        vec![
            FormulaNode::Forall(sort_user.clone()),
            FormulaNode::Pred {
                symbol: sym_has_score,
                args: vec![user_arg],
            },
        ],
        Limits::default(),
    )?;
    let neg3_err = check(&vocab, &neg3_formula).err();
    let expected_neg3 = SortError {
        path: SortPath {
            formula_node: 1,
            term: None,
        },
        kind: SortErrorKind::ArityMismatch {
            expected: 2,
            actual: 1,
        },
    };
    require(
        neg3_err.as_ref() == Some(&expected_neg3),
        "aridade incorreta em predicado deve ser reportada na raiz da fórmula com term: None",
    )?;
    println!(
        "3. Negative check (ArityMismatch): diagnostic=\"{}\" (path={:?})",
        neg3_err.as_ref().ok_or("expected arity error")?,
        expected_neg3.path
    );

    // 6. Caso Negativo 4: InvalidSymbolKind em termo
    // ∀u:User. u = is_active -> is_active é Pred, usado em posição de termo App
    // 0: Forall(User)
    // 1: Eq(Var(0, User), App(is_active, 0))
    let invalid_pred_app = Term::new(
        vec![TermNode::App {
            symbol: sym_is_active,
            args: 0,
        }],
        Limits::default(),
    )?;
    let neg4_formula = Formula::new(
        vec![
            FormulaNode::Forall(sort_user),
            FormulaNode::Eq(
                Term::new(
                    vec![TermNode::Var {
                        index: 0,
                        sort: Sort {
                            name: Text::new("User")?,
                        },
                    }],
                    Limits::default(),
                )?,
                invalid_pred_app,
            ),
        ],
        Limits::default(),
    )?;
    let neg4_err = check(&vocab, &neg4_formula).err();
    let expected_neg4 = SortError {
        path: SortPath {
            formula_node: 1,
            term: Some(TermPath {
                position: TermPosition::EqualityRight,
                node: 0,
            }),
        },
        kind: SortErrorKind::InvalidSymbolKind,
    };
    require(
        neg4_err.as_ref() == Some(&expected_neg4),
        "uso de predicado em posição de termo deve produzir InvalidSymbolKind",
    )?;
    println!(
        "4. Negative check (InvalidSymbolKind): diagnostic=\"{}\" (path={:?})",
        neg4_err.as_ref().ok_or("expected kind error")?,
        expected_neg4.path
    );

    // 7. Caso Negativo 5: UnknownSort em binder Forall
    // ∀x:Inexistente. True
    // 0: Forall(Inexistente)
    // 1: True
    let ghost_sort = Sort {
        name: Text::new("Inexistente")?,
    };
    let neg5_formula = Formula::new(
        vec![FormulaNode::Forall(ghost_sort), FormulaNode::True],
        Limits::default(),
    )?;
    let neg5_err = check(&vocab, &neg5_formula).err();
    let expected_neg5 = SortError {
        path: SortPath {
            formula_node: 0,
            term: None,
        },
        kind: SortErrorKind::UnknownSort,
    };
    require(
        neg5_err.as_ref() == Some(&expected_neg5),
        "sort inexistente em quantificador deve reportar UnknownSort na raiz do quantificador",
    )?;
    println!(
        "5. Negative check (UnknownSort): diagnostic=\"{}\" (path={:?})",
        neg5_err.as_ref().ok_or("expected sort error")?,
        expected_neg5.path
    );

    println!(
        "SORT_CHECK_SMOKE_OK: 1 valid closed formula and 5 negative cases verified with exact paths ({})",
        std::env::consts::ARCH
    );
    Ok(())
}
