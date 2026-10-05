//! Testes baseados em propriedades pseudoaleatórias determinísticas (sem dependências externas).
//!
//! Milhares de fórmulas fechadas bem tipadas sobre vocabulário multi-sortido são geradas e validadas.
//! Em seguida, mutações estruturais válidas que violam tipagem, escopo, assinaturas ou aridades são aplicadas,
//! garantindo que o sort-checker rejeite cada uma delas com diagnóstico adequado.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use logos_air::{
    Formula, FormulaNode, Limits, Sort, SortDecl, SortDomain, SortError, SortErrorKind, Symbol,
    SymbolDecl, SymbolKind, SymbolResult, Term, TermNode, Vocabulary, check,
};
use logos_canon::Text;

/// Gerador pseudoaleatório determinístico SplitMix64 (sem dependências externas).
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_usize(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            (self.next_u64() % bound as u64) as usize
        }
    }
}

fn sort_from_name(name: &str) -> Sort {
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

fn sym(name: &str, kind: SymbolKind, arity: Vec<Sort>, result: SymbolResult) -> Symbol {
    Symbol {
        name: Text::new(name).unwrap(),
        kind,
        arity,
        result,
    }
}

fn sym_decl(sym: Symbol) -> SymbolDecl {
    SymbolDecl {
        signature: sym,
        grounding: logos_air::Grounding::Glossed(Text::new("property test gloss").unwrap()),
        gloss_pt: Text::new("glosa propriedade").unwrap(),
    }
}

struct MultiSortEnv {
    vocab: Vocabulary,
    sorts: [Sort; 3],
    constants: Vec<Symbol>,
    nullary_funcs: Vec<Symbol>,
    unary_funcs: Vec<Symbol>,
    binary_funcs: Vec<Symbol>,
    ternary_funcs: Vec<Symbol>,
    predicates: Vec<Symbol>,
}

impl MultiSortEnv {
    fn new() -> Self {
        let s0 = sort_from_name("S0");
        let s1 = sort_from_name("S1");
        let s2 = sort_from_name("S2");
        let sorts = [s0.clone(), s1.clone(), s2.clone()];

        let sort_decls = vec![
            sort_decl_open("S0"),
            sort_decl_open("S1"),
            sort_decl_open("S2"),
        ];

        let constants = vec![
            sym(
                "c0_a",
                SymbolKind::Const,
                vec![],
                SymbolResult::Sort(s0.clone()),
            ),
            sym(
                "c0_b",
                SymbolKind::Const,
                vec![],
                SymbolResult::Sort(s0.clone()),
            ),
            sym(
                "c1_a",
                SymbolKind::Const,
                vec![],
                SymbolResult::Sort(s1.clone()),
            ),
            sym(
                "c1_b",
                SymbolKind::Const,
                vec![],
                SymbolResult::Sort(s1.clone()),
            ),
            sym(
                "c2_a",
                SymbolKind::Const,
                vec![],
                SymbolResult::Sort(s2.clone()),
            ),
            sym(
                "c2_b",
                SymbolKind::Const,
                vec![],
                SymbolResult::Sort(s2.clone()),
            ),
        ];

        let nullary_funcs = vec![
            sym(
                "fn0",
                SymbolKind::Func,
                vec![],
                SymbolResult::Sort(s0.clone()),
            ),
            sym(
                "fn1",
                SymbolKind::Func,
                vec![],
                SymbolResult::Sort(s1.clone()),
            ),
            sym(
                "fn2",
                SymbolKind::Func,
                vec![],
                SymbolResult::Sort(s2.clone()),
            ),
        ];

        let unary_funcs = vec![
            sym(
                "f00",
                SymbolKind::Func,
                vec![s0.clone()],
                SymbolResult::Sort(s0.clone()),
            ),
            sym(
                "f11",
                SymbolKind::Func,
                vec![s1.clone()],
                SymbolResult::Sort(s1.clone()),
            ),
            sym(
                "f22",
                SymbolKind::Func,
                vec![s2.clone()],
                SymbolResult::Sort(s2.clone()),
            ),
        ];

        let binary_funcs = vec![
            sym(
                "g01_0",
                SymbolKind::Func,
                vec![s0.clone(), s1.clone()],
                SymbolResult::Sort(s0.clone()),
            ),
            sym(
                "g12_1",
                SymbolKind::Func,
                vec![s1.clone(), s2.clone()],
                SymbolResult::Sort(s1.clone()),
            ),
            sym(
                "g02_2",
                SymbolKind::Func,
                vec![s0.clone(), s2.clone()],
                SymbolResult::Sort(s2.clone()),
            ),
        ];

        let ternary_funcs = vec![
            sym(
                "h012_0",
                SymbolKind::Func,
                vec![s0.clone(), s1.clone(), s2.clone()],
                SymbolResult::Sort(s0.clone()),
            ),
            sym(
                "h012_1",
                SymbolKind::Func,
                vec![s0.clone(), s1.clone(), s2.clone()],
                SymbolResult::Sort(s1.clone()),
            ),
            sym(
                "h012_2",
                SymbolKind::Func,
                vec![s0.clone(), s1.clone(), s2.clone()],
                SymbolResult::Sort(s2.clone()),
            ),
        ];

        let predicates = vec![
            sym("P_empty", SymbolKind::Pred, vec![], SymbolResult::Bool),
            sym("P0", SymbolKind::Pred, vec![s0.clone()], SymbolResult::Bool),
            sym("P1", SymbolKind::Pred, vec![s1.clone()], SymbolResult::Bool),
            sym("P2", SymbolKind::Pred, vec![s2.clone()], SymbolResult::Bool),
            sym(
                "Q01",
                SymbolKind::Pred,
                vec![s0.clone(), s1.clone()],
                SymbolResult::Bool,
            ),
            sym(
                "Q12",
                SymbolKind::Pred,
                vec![s1.clone(), s2.clone()],
                SymbolResult::Bool,
            ),
            sym(
                "R012",
                SymbolKind::Pred,
                vec![s0, s1, s2],
                SymbolResult::Bool,
            ),
        ];

        let mut sym_decls = Vec::new();
        for s in &constants {
            sym_decls.push(sym_decl(s.clone()));
        }
        for s in &nullary_funcs {
            sym_decls.push(sym_decl(s.clone()));
        }
        for s in &unary_funcs {
            sym_decls.push(sym_decl(s.clone()));
        }
        for s in &binary_funcs {
            sym_decls.push(sym_decl(s.clone()));
        }
        for s in &ternary_funcs {
            sym_decls.push(sym_decl(s.clone()));
        }
        for s in &predicates {
            sym_decls.push(sym_decl(s.clone()));
        }

        let vocab = Vocabulary::new(1, sort_decls, sym_decls).unwrap();

        Self {
            vocab,
            sorts,
            constants,
            nullary_funcs,
            unary_funcs,
            binary_funcs,
            ternary_funcs,
            predicates,
        }
    }

    fn sort_idx(&self, s: &Sort) -> usize {
        self.sorts.iter().position(|x| x == s).unwrap()
    }

    fn emit_term(
        &self,
        rng: &mut Rng,
        target_sort: usize,
        active_binders: &[usize],
        depth: usize,
        out: &mut Vec<TermNode>,
    ) {
        let matching_binders: Vec<u64> = active_binders
            .iter()
            .enumerate()
            .filter_map(|(idx, &s)| {
                if s == target_sort {
                    Some((active_binders.len() - 1 - idx) as u64)
                } else {
                    None
                }
            })
            .collect();

        let base_choices = 2 + if matching_binders.is_empty() { 0 } else { 1 };
        let can_recurse = depth < 3;

        if !can_recurse || rng.next_usize(3) == 0 {
            let choice = rng.next_usize(base_choices);
            match choice {
                0 => {
                    let const_candidates: Vec<&Symbol> = self
                        .constants
                        .iter()
                        .filter(|c| match &c.result {
                            SymbolResult::Sort(s) => self.sort_idx(s) == target_sort,
                            _ => false,
                        })
                        .collect();
                    let chosen = const_candidates[rng.next_usize(const_candidates.len())];
                    out.push(TermNode::App {
                        symbol: chosen.clone(),
                        args: 0,
                    });
                }
                1 => {
                    let nullary = &self.nullary_funcs[target_sort];
                    out.push(TermNode::App {
                        symbol: nullary.clone(),
                        args: 0,
                    });
                }
                _ => {
                    let de_bruijn = matching_binders[rng.next_usize(matching_binders.len())];
                    out.push(TermNode::Var {
                        index: de_bruijn,
                        sort: self.sorts[target_sort].clone(),
                    });
                }
            }
        } else {
            let mut func_candidates: Vec<&Symbol> = Vec::new();
            for f in &self.unary_funcs {
                if match &f.result {
                    SymbolResult::Sort(s) => self.sort_idx(s) == target_sort,
                    _ => false,
                } {
                    func_candidates.push(f);
                }
            }
            for f in &self.binary_funcs {
                if match &f.result {
                    SymbolResult::Sort(s) => self.sort_idx(s) == target_sort,
                    _ => false,
                } {
                    func_candidates.push(f);
                }
            }
            for f in &self.ternary_funcs {
                if match &f.result {
                    SymbolResult::Sort(s) => self.sort_idx(s) == target_sort,
                    _ => false,
                } {
                    func_candidates.push(f);
                }
            }

            let chosen = func_candidates[rng.next_usize(func_candidates.len())];
            out.push(TermNode::App {
                symbol: chosen.clone(),
                args: chosen.arity.len(),
            });
            for arg_sort in &chosen.arity {
                let arg_idx = self.sort_idx(arg_sort);
                self.emit_term(rng, arg_idx, active_binders, depth + 1, out);
            }
        }
    }

    fn gen_term(&self, rng: &mut Rng, target_sort: usize, active_binders: &[usize]) -> Term {
        let mut nodes = Vec::new();
        self.emit_term(rng, target_sort, active_binders, 0, &mut nodes);
        Term::new(
            nodes,
            Limits {
                max_depth: 64,
                max_nodes: 500,
            },
        )
        .unwrap()
    }

    fn emit_formula(
        &self,
        rng: &mut Rng,
        active_binders: &mut Vec<usize>,
        depth: usize,
        budget: &mut usize,
        out: &mut Vec<FormulaNode>,
    ) {
        let is_base = depth >= 4 || *budget == 0;
        if is_base {
            match rng.next_usize(6) {
                0 => out.push(FormulaNode::True),
                1 => out.push(FormulaNode::False),
                2 => out.push(FormulaNode::And(0)),
                3 => out.push(FormulaNode::Or(0)),
                4 => {
                    let s_idx = rng.next_usize(3);
                    let t1 = self.gen_term(rng, s_idx, active_binders);
                    let t2 = self.gen_term(rng, s_idx, active_binders);
                    out.push(FormulaNode::Eq(t1, t2));
                }
                _ => {
                    let pred = &self.predicates[rng.next_usize(self.predicates.len())];
                    let args: Vec<Term> = pred
                        .arity
                        .iter()
                        .map(|s| self.gen_term(rng, self.sort_idx(s), active_binders))
                        .collect();
                    out.push(FormulaNode::Pred {
                        symbol: pred.clone(),
                        args,
                    });
                }
            }
        } else {
            *budget = budget.saturating_sub(1);
            match rng.next_usize(8) {
                0 => {
                    out.push(FormulaNode::Not);
                    self.emit_formula(rng, active_binders, depth + 1, budget, out);
                }
                1 => {
                    let k = 1 + rng.next_usize(3);
                    out.push(FormulaNode::And(k));
                    for _ in 0..k {
                        self.emit_formula(rng, active_binders, depth + 1, budget, out);
                    }
                }
                2 => {
                    let k = 1 + rng.next_usize(3);
                    out.push(FormulaNode::Or(k));
                    for _ in 0..k {
                        self.emit_formula(rng, active_binders, depth + 1, budget, out);
                    }
                }
                3 => {
                    out.push(FormulaNode::Implies);
                    self.emit_formula(rng, active_binders, depth + 1, budget, out);
                    self.emit_formula(rng, active_binders, depth + 1, budget, out);
                }
                4 => {
                    out.push(FormulaNode::Iff);
                    self.emit_formula(rng, active_binders, depth + 1, budget, out);
                    self.emit_formula(rng, active_binders, depth + 1, budget, out);
                }
                5 => {
                    let s_idx = rng.next_usize(3);
                    out.push(FormulaNode::Forall(self.sorts[s_idx].clone()));
                    active_binders.push(s_idx);
                    self.emit_formula(rng, active_binders, depth + 1, budget, out);
                    active_binders.pop();
                }
                6 => {
                    let s_idx = rng.next_usize(3);
                    out.push(FormulaNode::Exists(self.sorts[s_idx].clone()));
                    active_binders.push(s_idx);
                    self.emit_formula(rng, active_binders, depth + 1, budget, out);
                    active_binders.pop();
                }
                _ => {
                    let s_idx = rng.next_usize(3);
                    let t1 = self.gen_term(rng, s_idx, active_binders);
                    let t2 = self.gen_term(rng, s_idx, active_binders);
                    out.push(FormulaNode::Eq(t1, t2));
                }
            }
        }
    }

    fn gen_formula(&self, rng: &mut Rng) -> Formula {
        let mut nodes = Vec::new();
        let mut active_binders = Vec::new();
        let mut budget = 12;
        self.emit_formula(rng, &mut active_binders, 0, &mut budget, &mut nodes);
        Formula::new(
            nodes,
            Limits {
                max_depth: 64,
                max_nodes: 10_000,
            },
        )
        .unwrap()
    }
}

const PROPERTY_SEED: u64 = 0x5EED_C0DE_1234_5678;
const MUTATION_SEED: u64 = PROPERTY_SEED.wrapping_add(42);

#[test]
fn test_thousands_of_well_typed_closed_formulas_pass() {
    let env = MultiSortEnv::new();
    let mut rng = Rng::new(PROPERTY_SEED);
    let cases = 2000;

    for case in 0..cases {
        let formula = env.gen_formula(&mut rng);
        let res = check(&env.vocab, &formula);
        assert!(
            res.is_ok(),
            "Well-typed formula rejected! Seed: 0x{:X}, Case: {}, Error: {:?}",
            PROPERTY_SEED,
            case,
            res.err()
        );
    }
}

#[test]
fn test_mutations_break_typing_and_fail() {
    let env = MultiSortEnv::new();
    let mut rng = Rng::new(MUTATION_SEED);
    let cases = 1000;
    let mut mutations = [0usize; 6];

    for case in 0..cases {
        let formula = env.gen_formula(&mut rng);

        // Mutação 1: Sort de variável adulterado
        let mut mutated_nodes = formula.nodes().to_vec();
        let mut mutated_var = false;
        for node in &mut mutated_nodes {
            match node {
                FormulaNode::Eq(left, right) => {
                    let mut nodes = left.nodes().to_vec();
                    for t_node in &mut nodes {
                        if let TermNode::Var { sort, .. } = t_node {
                            let curr = env.sort_idx(sort);
                            *sort = env.sorts[(curr + 1) % 3].clone();
                            mutated_var = true;
                            break;
                        }
                    }
                    if mutated_var {
                        *left = Term::new(
                            nodes,
                            Limits {
                                max_depth: 64,
                                max_nodes: 500,
                            },
                        )
                        .unwrap();
                        break;
                    }

                    let mut nodes_r = right.nodes().to_vec();
                    for t_node in &mut nodes_r {
                        if let TermNode::Var { sort, .. } = t_node {
                            let curr = env.sort_idx(sort);
                            *sort = env.sorts[(curr + 1) % 3].clone();
                            mutated_var = true;
                            break;
                        }
                    }
                    if mutated_var {
                        *right = Term::new(
                            nodes_r,
                            Limits {
                                max_depth: 64,
                                max_nodes: 500,
                            },
                        )
                        .unwrap();
                        break;
                    }
                }
                FormulaNode::Pred { args, .. } => {
                    for arg in args.iter_mut() {
                        let mut nodes = arg.nodes().to_vec();
                        for t_node in &mut nodes {
                            if let TermNode::Var { sort, .. } = t_node {
                                let curr = env.sort_idx(sort);
                                *sort = env.sorts[(curr + 1) % 3].clone();
                                mutated_var = true;
                                break;
                            }
                        }
                        if mutated_var {
                            *arg = Term::new(
                                nodes,
                                Limits {
                                    max_depth: 64,
                                    max_nodes: 500,
                                },
                            )
                            .unwrap();
                            break;
                        }
                    }
                    if mutated_var {
                        break;
                    }
                }
                _ => {}
            }
        }
        if mutated_var {
            mutations[0] += 1;
            let mutated_f = Formula::new(
                mutated_nodes,
                Limits {
                    max_depth: 64,
                    max_nodes: 10_000,
                },
            )
            .unwrap();
            let res = check(&env.vocab, &mutated_f);
            assert!(
                matches!(
                    &res,
                    Err(SortError {
                        kind: SortErrorKind::SortMismatch { .. },
                        ..
                    })
                ),
                "Variable sort mutation should fail! Seed: 0x{:X}, Case: {}",
                MUTATION_SEED,
                case
            );
        }

        // Mutação 2: Sort de igualdade adulterado (troca lado direito por termo de sort diferente)
        let mut mutated_eq_nodes = formula.nodes().to_vec();
        let mut mutated_eq = false;
        for node in &mut mutated_eq_nodes {
            if let FormulaNode::Eq(left, right) = node {
                // Descobre sort da raiz de left
                let left_root = &left.nodes()[0];
                let left_sort_idx = match left_root {
                    TermNode::Var { sort, .. } => env.sort_idx(sort),
                    TermNode::App { symbol, .. } => match &symbol.result {
                        SymbolResult::Sort(s) => env.sort_idx(s),
                        _ => 0,
                    },
                };
                let diff_sort = (left_sort_idx + 1) % 3;
                let diff_term = env.gen_term(&mut rng, diff_sort, &[]);
                *right = diff_term;
                mutated_eq = true;
                break;
            }
        }
        if mutated_eq {
            mutations[1] += 1;
            let mutated_f = Formula::new(
                mutated_eq_nodes,
                Limits {
                    max_depth: 64,
                    max_nodes: 10_000,
                },
            )
            .unwrap();
            let res = check(&env.vocab, &mutated_f);
            assert!(
                matches!(
                    &res,
                    Err(SortError {
                        kind: SortErrorKind::SortMismatch { .. },
                        ..
                    })
                ),
                "Equality sort mutation should fail! Seed: 0x{:X}, Case: {}",
                MUTATION_SEED,
                case
            );
        }

        // Mutação 3: Sort de argumento de predicado adulterado
        let mut mutated_pred_nodes = formula.nodes().to_vec();
        let mut mutated_pred = false;
        for node in &mut mutated_pred_nodes {
            if let FormulaNode::Pred { symbol, args } = node
                && !args.is_empty()
            {
                let target_idx = 0;
                let orig_sort_idx = env.sort_idx(&symbol.arity[target_idx]);
                let diff_sort = (orig_sort_idx + 1) % 3;
                let diff_term = env.gen_term(&mut rng, diff_sort, &[]);
                args[target_idx] = diff_term;
                mutated_pred = true;
                break;
            }
        }
        if mutated_pred {
            mutations[2] += 1;
            let mutated_f = Formula::new(
                mutated_pred_nodes,
                Limits {
                    max_depth: 64,
                    max_nodes: 10_000,
                },
            )
            .unwrap();
            let res = check(&env.vocab, &mutated_f);
            assert!(
                matches!(
                    &res,
                    Err(SortError {
                        kind: SortErrorKind::SortMismatch { .. },
                        ..
                    })
                ),
                "Predicate argument sort mutation should fail! Seed: 0x{:X}, Case: {}",
                MUTATION_SEED,
                case
            );
        }

        // Mutação 4: Aridade de predicado adulterada
        let mut mutated_arity_nodes = formula.nodes().to_vec();
        let mut mutated_arity = false;
        for node in &mut mutated_arity_nodes {
            if let FormulaNode::Pred { args, .. } = node
                && !args.is_empty()
            {
                args.pop();
                mutated_arity = true;
                break;
            }
        }
        if mutated_arity {
            mutations[3] += 1;
            let mutated_f = Formula::new(
                mutated_arity_nodes,
                Limits {
                    max_depth: 64,
                    max_nodes: 10_000,
                },
            )
            .unwrap();
            let res = check(&env.vocab, &mutated_f);
            assert!(
                matches!(
                    &res,
                    Err(SortError {
                        kind: SortErrorKind::ArityMismatch { .. },
                        ..
                    })
                ),
                "Predicate arity mutation should fail! Seed: 0x{:X}, Case: {}",
                MUTATION_SEED,
                case
            );
        }

        // Mutação 5: Variável desvinculada (índice livre)
        let mut mutated_unbound_nodes = formula.nodes().to_vec();
        let mut mutated_unbound = false;
        for node in &mut mutated_unbound_nodes {
            if let FormulaNode::Eq(left, _) = node {
                let mut nodes = left.nodes().to_vec();
                for t_node in &mut nodes {
                    if let TermNode::Var { index, .. } = t_node {
                        *index = u64::MAX;
                        mutated_unbound = true;
                        break;
                    }
                }
                if mutated_unbound {
                    *left = Term::new(
                        nodes,
                        Limits {
                            max_depth: 64,
                            max_nodes: 500,
                        },
                    )
                    .unwrap();
                    break;
                }
            }
        }
        if mutated_unbound {
            mutations[4] += 1;
            let mutated_f = Formula::new(
                mutated_unbound_nodes,
                Limits {
                    max_depth: 64,
                    max_nodes: 10_000,
                },
            )
            .unwrap();
            let res = check(&env.vocab, &mutated_f);
            assert!(
                matches!(
                    &res,
                    Err(SortError {
                        kind: SortErrorKind::UnboundVariable { .. },
                        ..
                    })
                ),
                "Unbound variable mutation should fail! Seed: 0x{:X}, Case: {}",
                MUTATION_SEED,
                case
            );
        }

        // Mutação 6: Assinatura adulterada
        let mut mutated_sig_nodes = formula.nodes().to_vec();
        let mut mutated_sig = false;
        for node in &mut mutated_sig_nodes {
            if let FormulaNode::Pred { symbol, .. } = node {
                if symbol.arity.len() >= 2 {
                    symbol.arity.reverse();
                    if symbol.arity[0] != symbol.arity[1] {
                        mutated_sig = true;
                        break;
                    }
                } else if !symbol.arity.is_empty() {
                    let curr = env.sort_idx(&symbol.arity[0]);
                    symbol.arity[0] = env.sorts[(curr + 1) % 3].clone();
                    mutated_sig = true;
                    break;
                }
            }
        }
        if mutated_sig {
            mutations[5] += 1;
            let mutated_f = Formula::new(
                mutated_sig_nodes,
                Limits {
                    max_depth: 64,
                    max_nodes: 10_000,
                },
            )
            .unwrap();
            let res = check(&env.vocab, &mutated_f);
            assert!(
                matches!(
                    &res,
                    Err(SortError {
                        kind: SortErrorKind::SignatureMismatch,
                        ..
                    })
                ),
                "Signature mismatch mutation should fail! Seed: 0x{:X}, Case: {}",
                MUTATION_SEED,
                case
            );
        }
    }
    println!(
        "MUTATION_CASES seed={MUTATION_SEED:#x}: variable_sort, equality, argument, arity, unbound, signature={mutations:?}"
    );
}
