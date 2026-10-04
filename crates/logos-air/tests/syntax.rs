//! Behavioral contracts for flat syntax: no textual parser and no sort checker.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use logos_air::{
    AirError, Formula, FormulaNode, Limits, LogicId, Sort, Symbol, SymbolKind, SymbolResult, Term,
    TermNode,
};
use logos_canon::{Canon, CanonDecode, Text, from_canon, to_canon};
use logos_crypto::hash;

fn sort(name: &str) -> Sort {
    Sort {
        name: Text::new(name).expect("fixture text"),
    }
}

fn symbol() -> Symbol {
    Symbol {
        name: Text::new("f").unwrap(),
        kind: SymbolKind::Func,
        arity: vec![sort("U")],
        result: SymbolResult::Sort(sort("U")),
    }
}

fn var(index: u64) -> Term {
    Term::new(
        vec![TermNode::Var {
            index,
            sort: sort("U"),
        }],
        Limits::default(),
    )
    .unwrap()
}

fn term_chain(depth: usize, limits: Limits) -> Result<Term, AirError> {
    assert!(depth > 0);
    let mut nodes = Vec::with_capacity(depth);
    for _ in 1..depth {
        nodes.push(TermNode::App {
            symbol: symbol(),
            args: 1,
        });
    }
    nodes.push(TermNode::Var {
        index: 0,
        sort: sort("U"),
    });
    Term::new(nodes, limits)
}

fn formula_chain(depth: usize, limits: Limits) -> Result<Formula, AirError> {
    assert!(depth > 0);
    let mut nodes = vec![FormulaNode::Not; depth - 1];
    nodes.push(FormulaNode::True);
    Formula::new(nodes, limits)
}

// A consumer resolves names before handing syntax to AIR. This intentionally is
// not a parser, and shadowing is resolved by searching binders from innermost.
enum Named<'a> {
    Forall(&'a str, Box<Named<'a>>),
    Exists(&'a str, Box<Named<'a>>),
    Eq(&'a str, &'a str),
    Pred(&'a str),
}

fn resolve(named: &Named<'_>) -> Formula {
    fn index(binders: &[&str], name: &str) -> u64 {
        u64::try_from(
            binders
                .iter()
                .rev()
                .position(|binder| *binder == name)
                .expect("test variable is bound"),
        )
        .unwrap()
    }
    fn emit<'a>(named: &Named<'a>, binders: &mut Vec<&'a str>, out: &mut Vec<FormulaNode>) {
        match named {
            Named::Forall(name, child) | Named::Exists(name, child) => {
                out.push(match named {
                    Named::Forall(..) => FormulaNode::Forall(sort("U")),
                    _ => FormulaNode::Exists(sort("U")),
                });
                binders.push(name);
                emit(child, binders, out);
                binders.pop();
            }
            Named::Eq(left, right) => {
                out.push(FormulaNode::Eq(
                    var(index(binders, left)),
                    var(index(binders, right)),
                ));
            }
            Named::Pred(name) => {
                out.push(FormulaNode::Pred {
                    symbol: Symbol {
                        name: Text::new("P").unwrap(),
                        kind: SymbolKind::Pred,
                        arity: vec![sort("U")],
                        result: SymbolResult::Bool,
                    },
                    args: vec![var(index(binders, name))],
                });
            }
        }
    }
    let mut nodes = Vec::new();
    emit(named, &mut Vec::new(), &mut nodes);
    Formula::new(nodes, Limits::default()).unwrap()
}

#[test]
fn consumer_alpha_renaming_resolves_to_identical_bytes_and_hashes() {
    let x = resolve(&Named::Forall("x", Box::new(Named::Pred("x"))));
    let y = resolve(&Named::Forall("y", Box::new(Named::Pred("y"))));
    assert_eq!(x, y);
    assert_eq!(to_canon(&x), to_canon(&y));
    assert_eq!(hash(&x).unwrap(), hash(&y).unwrap());
    assert!(matches!(&x.nodes()[1], FormulaNode::Pred { symbol, args }
        if symbol.name.as_str() == "P" && args == &[var(0)]));
    // SHA-256 do envelope de ∀x:U.P(x), obtido por CBOR independente + hashlib.
    assert_eq!(
        hash(&x).unwrap().to_hex(),
        "36eaac5de76e1e4760925fcce2e8ca36a5a5ebab3b138b6a993d8a6a4ce7eaab"
    );
}

#[test]
fn nested_binders_shadowing_and_distinct_indices_are_not_conflated() {
    let nested = resolve(&Named::Forall(
        "x",
        Box::new(Named::Exists("y", Box::new(Named::Eq("x", "y")))),
    ));
    let renamed = resolve(&Named::Forall(
        "a",
        Box::new(Named::Exists("b", Box::new(Named::Eq("a", "b")))),
    ));
    assert_eq!(nested, renamed);
    assert_eq!(hash(&nested).unwrap(), hash(&renamed).unwrap());
    let shadowed = resolve(&Named::Forall(
        "x",
        Box::new(Named::Exists("x", Box::new(Named::Eq("x", "x")))),
    ));
    let nearest = resolve(&Named::Forall(
        "a",
        Box::new(Named::Exists("b", Box::new(Named::Eq("b", "b")))),
    ));
    assert_eq!(shadowed, nearest);
    assert!(matches!(&nested.nodes()[2], FormulaNode::Eq(a, b) if a == &var(1) && b == &var(0)));
    assert_ne!(to_canon(&nested).unwrap(), to_canon(&shadowed).unwrap());
    assert_ne!(hash(&nested).unwrap(), hash(&shadowed).unwrap());
    let reversed = resolve(&Named::Forall(
        "x",
        Box::new(Named::Exists("y", Box::new(Named::Eq("y", "x")))),
    ));
    assert_ne!(to_canon(&nested).unwrap(), to_canon(&reversed).unwrap());
    assert_ne!(hash(&nested).unwrap(), hash(&reversed).unwrap());
    let one_binder = resolve(&Named::Forall("x", Box::new(Named::Eq("x", "x"))));
    assert_ne!(to_canon(&one_binder).unwrap(), to_canon(&shadowed).unwrap());
}

#[test]
fn connectives_argument_order_and_redundant_syntax_remain_distinct() {
    let make = |nodes| Formula::new(nodes, Limits::default()).unwrap();
    let ordered = make(vec![
        FormulaNode::And(2),
        FormulaNode::True,
        FormulaNode::False,
    ]);
    let reordered = make(vec![
        FormulaNode::And(2),
        FormulaNode::False,
        FormulaNode::True,
    ]);
    let disjunction = make(vec![
        FormulaNode::Or(2),
        FormulaNode::True,
        FormulaNode::False,
    ]);
    let implication = make(vec![
        FormulaNode::Implies,
        FormulaNode::True,
        FormulaNode::False,
    ]);
    let iff = make(vec![
        FormulaNode::Iff,
        FormulaNode::True,
        FormulaNode::False,
    ]);
    for different in [reordered, disjunction, implication, iff] {
        assert_ne!(to_canon(&ordered).unwrap(), to_canon(&different).unwrap());
        assert_ne!(hash(&ordered).unwrap(), hash(&different).unwrap());
    }
    let nested = make(vec![
        FormulaNode::And(2),
        FormulaNode::And(1),
        FormulaNode::True,
        FormulaNode::False,
    ]);
    assert_ne!(to_canon(&ordered).unwrap(), to_canon(&nested).unwrap());
    let double_not = make(vec![FormulaNode::Not, FormulaNode::Not, FormulaNode::True]);
    assert_ne!(double_not, make(vec![FormulaNode::True]));
    let application = |first, second| {
        Term::new(
            vec![
                TermNode::App {
                    symbol: symbol(),
                    args: 2,
                },
                TermNode::Var {
                    index: first,
                    sort: sort("U"),
                },
                TermNode::Var {
                    index: second,
                    sort: sort("U"),
                },
            ],
            Limits::default(),
        )
        .unwrap()
    };
    assert_ne!(
        to_canon(&application(0, 1)).unwrap(),
        to_canon(&application(1, 0)).unwrap()
    );
    assert_eq!(
        from_canon::<Formula>(&to_canon(&nested).unwrap()),
        Ok(nested)
    );
}

#[test]
fn prefix_trees_require_exactly_one_complete_root() {
    for nodes in [
        vec![],
        vec![FormulaNode::Not],
        vec![FormulaNode::And(2), FormulaNode::True],
        vec![FormulaNode::Or(1)],
        vec![FormulaNode::Implies, FormulaNode::True],
        vec![FormulaNode::Iff, FormulaNode::True],
        vec![FormulaNode::Forall(sort("U"))],
        vec![FormulaNode::Exists(sort("U"))],
        vec![FormulaNode::True, FormulaNode::False],
        vec![FormulaNode::And(1), FormulaNode::True, FormulaNode::False],
        vec![FormulaNode::And(2), FormulaNode::Not, FormulaNode::True],
    ] {
        assert_eq!(
            Formula::new(nodes, Limits::default()),
            Err(AirError::InvalidStructure)
        );
    }
    let leaf = TermNode::Var {
        index: 0,
        sort: sort("U"),
    };
    for nodes in [
        vec![],
        vec![TermNode::App {
            symbol: symbol(),
            args: 1,
        }],
        vec![
            TermNode::App {
                symbol: symbol(),
                args: 2,
            },
            leaf.clone(),
        ],
        vec![leaf.clone(), leaf.clone()],
        vec![
            TermNode::App {
                symbol: symbol(),
                args: 1,
            },
            leaf.clone(),
            leaf.clone(),
        ],
    ] {
        assert_eq!(
            Term::new(nodes, Limits::default()),
            Err(AirError::InvalidStructure)
        );
    }
    for node in [FormulaNode::And(0), FormulaNode::Or(0)] {
        let tree = Formula::new(vec![node], Limits::default()).unwrap();
        assert_eq!((tree.depth(), tree.node_count()), (1, 1));
    }
    let constant = Term::new(
        vec![TermNode::App {
            symbol: symbol(),
            args: 0,
        }],
        Limits::default(),
    )
    .unwrap();
    assert_eq!((constant.depth(), constant.node_count()), (1, 1));
}

#[test]
fn exact_and_zero_limits_apply_to_both_syntax_types() {
    let exact = Limits {
        max_depth: 3,
        max_nodes: 3,
    };
    let formula = formula_chain(3, exact).unwrap();
    let term = term_chain(3, exact).unwrap();
    assert_eq!((formula.depth(), formula.node_count()), (3, 3));
    assert_eq!((term.depth(), term.node_count()), (3, 3));
    for depth in [0, 1, 2] {
        let limits = Limits {
            max_depth: depth,
            max_nodes: 10,
        };
        assert_eq!(
            formula_chain(3, limits),
            Err(AirError::DepthExceeded {
                limit: depth,
                actual: depth + 1
            })
        );
        assert_eq!(
            term_chain(3, limits),
            Err(AirError::DepthExceeded {
                limit: depth,
                actual: depth + 1
            })
        );
    }
    for count in [0, 1, 2] {
        let limits = Limits {
            max_depth: 10,
            max_nodes: count,
        };
        assert_eq!(
            formula_chain(3, limits),
            Err(AirError::NodesExceeded {
                limit: count,
                actual: count + 1
            })
        );
        assert_eq!(
            term_chain(3, limits),
            Err(AirError::NodesExceeded {
                limit: count,
                actual: count + 1
            })
        );
    }
    let broad = Formula::new(
        vec![
            FormulaNode::And(3),
            FormulaNode::True,
            FormulaNode::False,
            FormulaNode::True,
        ],
        Limits {
            max_depth: 2,
            max_nodes: 4,
        },
    )
    .unwrap();
    assert_eq!((broad.depth(), broad.node_count()), (2, 4));
}

#[test]
fn embedded_terms_are_revalidated_under_aggregate_parent_limits() {
    let generous = Limits {
        max_depth: 1000,
        max_nodes: 1000,
    };
    let left = term_chain(3, generous).unwrap();
    let right = term_chain(2, generous).unwrap();
    let eq = FormulaNode::Eq(left.clone(), right.clone());
    let exact = Limits {
        max_depth: 4,
        max_nodes: 6,
    };
    let formula = Formula::new(vec![eq.clone()], exact).unwrap();
    assert_eq!((formula.depth(), formula.node_count()), (4, 6));
    assert_eq!(
        Formula::new(
            vec![eq.clone()],
            Limits {
                max_depth: 3,
                max_nodes: 6
            }
        ),
        Err(AirError::DepthExceeded {
            limit: 3,
            actual: 4
        })
    );
    assert_eq!(
        Formula::new(
            vec![eq.clone()],
            Limits {
                max_depth: 4,
                max_nodes: 5
            }
        ),
        Err(AirError::NodesExceeded {
            limit: 5,
            actual: 6
        })
    );
    let nested = vec![FormulaNode::Not, eq];
    let tree = Formula::new(
        nested.clone(),
        Limits {
            max_depth: 5,
            max_nodes: 7,
        },
    )
    .unwrap();
    assert_eq!((tree.depth(), tree.node_count()), (5, 7));
    assert_eq!(
        Formula::new(
            nested,
            Limits {
                max_depth: 4,
                max_nodes: 7
            }
        ),
        Err(AirError::DepthExceeded {
            limit: 4,
            actual: 5
        })
    );
    let pred = FormulaNode::Pred {
        symbol: symbol(),
        args: vec![left, right, var(0)],
    };
    let tree = Formula::new(
        vec![pred.clone()],
        Limits {
            max_depth: 4,
            max_nodes: 7,
        },
    )
    .unwrap();
    assert_eq!((tree.depth(), tree.node_count()), (4, 7));
    assert_eq!(
        Formula::new(
            vec![pred],
            Limits {
                max_depth: 4,
                max_nodes: 6
            }
        ),
        Err(AirError::NodesExceeded {
            limit: 6,
            actual: 7
        })
    );
    // Terms in separate siblings consume the same aggregate budget.
    let siblings = vec![
        FormulaNode::And(2),
        FormulaNode::Eq(var(0), var(1)),
        FormulaNode::Eq(var(2), var(3)),
    ];
    assert_eq!(
        Formula::new(
            siblings.clone(),
            Limits {
                max_depth: 3,
                max_nodes: 7
            }
        )
        .unwrap()
        .node_count(),
        7
    );
    assert_eq!(
        Formula::new(
            siblings,
            Limits {
                max_depth: 3,
                max_nodes: 6
            }
        ),
        Err(AirError::NodesExceeded {
            limit: 6,
            actual: 7
        })
    );
    for (value, limits) in [
        (&formula, exact),
        (
            &tree,
            Limits {
                max_depth: 4,
                max_nodes: 7,
            },
        ),
    ] {
        let bytes = to_canon(value).unwrap();
        assert_eq!(
            Formula::decode_with_limits(&bytes, limits),
            Ok(value.clone())
        );
        assert!(matches!(
            Formula::decode_with_limits(
                &bytes,
                Limits {
                    max_depth: 4,
                    max_nodes: value.node_count() - 1
                }
            ),
            Err(AirError::NodesExceeded { .. })
        ));
        assert!(matches!(
            Formula::decode_with_limits(
                &bytes,
                Limits {
                    max_depth: value.depth() - 1,
                    max_nodes: 10
                }
            ),
            Err(AirError::DepthExceeded { .. })
        ));
    }
}

#[test]
fn decoder_defaults_and_custom_limits_do_not_change_identity() {
    let generous = Limits {
        max_depth: 129,
        max_nodes: 100_001,
    };
    let formula = formula_chain(129, generous).unwrap();
    let term = term_chain(129, generous).unwrap();
    let fb = to_canon(&formula).unwrap();
    let tb = to_canon(&term).unwrap();
    assert!(from_canon::<Formula>(&fb).is_err());
    assert!(from_canon::<Term>(&tb).is_err());
    assert_eq!(
        Formula::decode_with_limits(&fb, Limits::default()),
        Err(AirError::DepthExceeded {
            limit: 128,
            actual: 129
        })
    );
    assert_eq!(
        Term::decode_with_limits(&tb, Limits::default()),
        Err(AirError::DepthExceeded {
            limit: 128,
            actual: 129
        })
    );
    assert_eq!(
        Formula::decode_with_limits(&fb, generous),
        Ok(formula.clone())
    );
    assert_eq!(Term::decode_with_limits(&tb, generous), Ok(term.clone()));
    assert_eq!(
        Formula::from_value_with_limits(&formula.to_value().unwrap(), generous),
        Ok(formula)
    );
    assert_eq!(
        Term::from_value_with_limits(&term.to_value().unwrap(), generous),
        Ok(term)
    );
    let exact = Limits {
        max_depth: 2,
        max_nodes: 2,
    };
    let a = formula_chain(2, exact).unwrap();
    let b = formula_chain(2, generous).unwrap();
    assert_eq!(a, b);
    assert_eq!(to_canon(&a), to_canon(&b));
    assert_eq!(hash(&a).unwrap(), hash(&b).unwrap());
    assert_eq!(a.logic_id(), LogicId::FolMsEqV1);
    assert_eq!(var(99).logic_id().as_str(), "FOL-ms-eq/v1");
    assert_eq!(LogicId::from_name("FOL-ms-eq/v1"), Ok(LogicId::FolMsEqV1));
    assert_eq!(
        LogicId::from_name("FOL-ms-eq/v2"),
        Err(AirError::UnsupportedLogic)
    );
    assert_eq!(from_canon::<Formula>(&to_canon(&a).unwrap()), Ok(a.clone()));
    assert_eq!(Formula::from_value(&a.to_value().unwrap()), Ok(a));
}

#[test]
fn syntax_deliberately_accepts_free_indices_and_ill_sorted_signatures() {
    let predicate = Symbol {
        name: Text::new("P").unwrap(),
        kind: SymbolKind::Pred,
        arity: vec![sort("Declared")],
        result: SymbolResult::Bool,
    };
    // A predicate used as an application, with a signature-arity mismatch.
    let wrong_term = Term::new(
        vec![TermNode::App {
            symbol: predicate.clone(),
            args: 0,
        }],
        Limits::default(),
    )
    .unwrap();
    let free = Term::new(
        vec![TermNode::Var {
            index: u64::MAX,
            sort: sort("Other"),
        }],
        Limits::default(),
    )
    .unwrap();
    // Eq with unequal sorts, an unbound variable, and a Func used as Pred are
    // all structural syntax. Stage 1.3, not AIR construction, judges sorts.
    let formula = Formula::new(
        vec![
            FormulaNode::And(2),
            FormulaNode::Eq(wrong_term.clone(), free.clone()),
            FormulaNode::Pred {
                symbol: symbol(),
                args: vec![free, wrong_term],
            },
        ],
        Limits::default(),
    )
    .unwrap();
    assert_eq!((formula.depth(), formula.node_count()), (3, 7));
    assert_eq!(
        from_canon::<Formula>(&to_canon(&formula).unwrap()),
        Ok(formula)
    );
    let bool_constant = Symbol {
        kind: SymbolKind::Const,
        result: SymbolResult::Bool,
        ..predicate
    };
    assert_eq!(
        from_canon::<Symbol>(&to_canon(&bool_constant).unwrap()),
        Ok(bool_constant)
    );
}

#[test]
fn default_node_budget_and_custom_override_apply_at_exact_boundary() {
    let custom = Limits {
        max_depth: 2,
        max_nodes: 100_001,
    };
    let mut formula_nodes = vec![FormulaNode::And(100_000)];
    formula_nodes.extend(std::iter::repeat_n(FormulaNode::True, 100_000));
    let formula = Formula::new(formula_nodes, custom).unwrap();
    assert_eq!((formula.depth(), formula.node_count()), (2, 100_001));
    let bytes = to_canon(&formula).unwrap();
    assert_eq!(
        Formula::decode_with_limits(&bytes, Limits::default()),
        Err(AirError::NodesExceeded {
            limit: 100_000,
            actual: 100_001
        })
    );
    assert!(from_canon::<Formula>(&bytes).is_err());
    assert_eq!(Formula::decode_with_limits(&bytes, custom), Ok(formula));
    let mut term_nodes = vec![TermNode::App {
        symbol: symbol(),
        args: 100_000,
    }];
    term_nodes.extend(std::iter::repeat_n(
        TermNode::Var {
            index: 0,
            sort: sort("U"),
        },
        100_000,
    ));
    let term = Term::new(term_nodes, custom).unwrap();
    assert_eq!((term.depth(), term.node_count()), (2, 100_001));
    let bytes = to_canon(&term).unwrap();
    assert_eq!(
        Term::decode_with_limits(&bytes, Limits::default()),
        Err(AirError::NodesExceeded {
            limit: 100_000,
            actual: 100_001
        })
    );
    assert!(from_canon::<Term>(&bytes).is_err());
    assert_eq!(Term::decode_with_limits(&bytes, custom), Ok(term));
}

// Independent fixed-format CBOR construction, never encoding the candidate AST.
fn flat_formula_bytes(depth: usize) -> Vec<u8> {
    let mut bytes = vec![0x84, 1, 3, 0x6c];
    bytes.extend_from_slice(b"FOL-ms-eq/v1");
    if depth < 24 {
        bytes.push(0x80 | u8::try_from(depth).unwrap());
    } else if let Ok(count) = u8::try_from(depth) {
        bytes.extend_from_slice(&[0x98, count]);
    } else if let Ok(count) = u16::try_from(depth) {
        bytes.push(0x99);
        bytes.extend_from_slice(&count.to_be_bytes());
    } else {
        bytes.push(0x9a);
        bytes.extend_from_slice(&u32::try_from(depth).unwrap().to_be_bytes());
    }
    for _ in 1..depth {
        bytes.extend_from_slice(&[0x81, 4]);
    }
    bytes.extend_from_slice(&[0x81, 0]);
    bytes
}

#[test]
fn million_formula_and_4096_deep_terms_are_stack_safe() {
    std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(|| {
            let limits = Limits {
                max_depth: 128,
                max_nodes: 1_000_000,
            };
            // Exactly one million flat prefix nodes; error paths drop the whole Vec.
            assert_eq!(
                formula_chain(1_000_000, limits),
                Err(AirError::DepthExceeded {
                    limit: 128,
                    actual: 129
                })
            );
            let bytes = flat_formula_bytes(1_000_000);
            assert_eq!(
                Formula::decode_with_limits(&bytes, limits),
                Err(AirError::DepthExceeded {
                    limit: 128,
                    actual: 129
                })
            );
            // A corresponding accepted shallow byte stream makes the handcrafted
            // envelope meaningful, rather than passing by being malformed CBOR.
            let shallow = formula_chain(3, Limits::default()).unwrap();
            assert_eq!(to_canon(&shallow).unwrap(), flat_formula_bytes(3));
            assert_eq!(
                Formula::decode_with_limits(&flat_formula_bytes(3), Limits::default()),
                Ok(shallow)
            );
            let deep_term = term_chain(
                4096,
                Limits {
                    max_depth: 4096,
                    max_nodes: 4096,
                },
            )
            .unwrap();
            assert_eq!(deep_term.clone(), deep_term);
            let term_bytes = to_canon(&deep_term).unwrap();
            assert!(matches!(
                Term::decode_with_limits(&term_bytes, limits),
                Err(AirError::DepthExceeded { .. })
            ));
            drop(deep_term);
        })
        .expect("small-stack thread")
        .join()
        .expect("no overflow or assertion failure");
}
