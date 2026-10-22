//! Vocabulário adversarial e gerador determinístico de fórmulas fechadas bem tipadas.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)]

use logos_air::{
    Formula, FormulaNode, Grounding, Limits, Sort, SortDecl, SortDomain, Symbol, SymbolDecl,
    SymbolKind, SymbolResult, Term, TermNode, Vocabulary,
};
use logos_canon::Text;

/// SplitMix64, sem dependências externas.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            (self.next_u64() % bound as u64) as usize
        }
    }
}

pub fn text(s: &str) -> Text {
    Text::new(s).unwrap()
}

pub fn sort(name: &str) -> Sort {
    Sort { name: text(name) }
}

pub fn symbol(name: &str, kind: SymbolKind, arity: &[&str], result: Option<&str>) -> Symbol {
    Symbol {
        name: text(name),
        kind,
        arity: arity.iter().map(|s| sort(s)).collect(),
        result: result.map_or(SymbolResult::Bool, |s| SymbolResult::Sort(sort(s))),
    }
}

pub fn decl(signature: Symbol) -> SymbolDecl {
    SymbolDecl {
        signature,
        grounding: Grounding::Glossed(text("glosa de teste")),
        gloss_pt: text("glosa"),
    }
}

/// Monta vocabulary/v2 a partir de `(símbolo, template)`.
pub fn vocabulary(sorts: &[&str], symbols: Vec<(Symbol, &str)>) -> Vocabulary {
    let sorts = sorts
        .iter()
        .map(|name| SortDecl {
            name: text(name),
            domain: SortDomain::Open,
        })
        .collect();
    let (symbols, templates): (Vec<_>, Vec<_>) = symbols
        .into_iter()
        .map(|(signature, template)| (decl(signature), text(template)))
        .unzip();
    Vocabulary::new_with_render_pt(1, sorts, symbols, templates).unwrap()
}

/// Templates escolhidos para colidir com a estrutura: parênteses, conectivos,
/// "verdadeiro", "x1" dentro de texto, ponto e vírgula e acentos.
pub fn adversarial() -> Vocabulary {
    use SymbolKind::{Const, Func, Pred};
    vocabulary(
        &["Usuário", "Documento", "Tipo (x1)"],
        vec![
            (symbol("alice", Const, &[], Some("Usuário")), "Alice"),
            (
                symbol("bob", Const, &[], Some("Usuário")),
                "o usuário x1 e x2",
            ),
            (
                symbol("relatorio", Const, &[], Some("Documento")),
                "o relatório (verdadeiro)",
            ),
            (symbol("t0", Const, &[], Some("Tipo (x1)")), "falso"),
            (
                symbol("autor", Func, &["Documento"], Some("Usuário")),
                "o autor de {0}",
            ),
            (
                symbol("chefe", Func, &["Usuário"], Some("Usuário")),
                "o chefe de {0}",
            ),
            (
                symbol("tipo", Func, &["Documento", "Usuário"], Some("Tipo (x1)")),
                "({0}), então ({1})",
            ),
            (
                symbol("Dono", Pred, &["Usuário", "Documento"], None),
                "{0} é dono de {1}",
            ),
            (
                symbol("Le", Pred, &["Usuário", "Documento"], None),
                "se {0}, então {1}",
            ),
            (
                symbol("Admin", Pred, &["Usuário"], None),
                "{0} é administrador; (verdadeiro)",
            ),
            (symbol("Chove", Pred, &[], None), "verdadeiro"),
            (
                symbol("Feriado", Pred, &[], None),
                "não é o caso que (falso)",
            ),
            (
                symbol("Classe", Pred, &["Tipo (x1)"], None),
                "o valor {0} é igual a nada",
            ),
            (
                symbol("Tres", Pred, &["Usuário", "Usuário", "Documento"], None),
                "{0} e {1} vale cada uma de {2}",
            ),
        ],
    )
}

struct Generator<'v> {
    rng: Rng,
    vocabulary: &'v Vocabulary,
    binders: Vec<Sort>,
    nodes: Vec<FormulaNode>,
}

impl Generator<'_> {
    fn symbols(&self, kind: SymbolKind) -> Vec<Symbol> {
        self.vocabulary
            .symbols()
            .iter()
            .filter(|decl| decl.signature.kind == kind)
            .map(|decl| decl.signature.clone())
            .collect()
    }

    fn term(&mut self, target: &Sort, budget: usize, out: &mut Vec<TermNode>) {
        let vars: Vec<usize> = self
            .binders
            .iter()
            .enumerate()
            .filter(|(_, s)| *s == target)
            .map(|(i, _)| i)
            .collect();
        let candidates: Vec<Symbol> = [SymbolKind::Const, SymbolKind::Func]
            .iter()
            .flat_map(|kind| self.symbols(*kind))
            .filter(|s| s.result == SymbolResult::Sort(target.clone()))
            .filter(|s| budget > 1 || s.arity.is_empty())
            .collect();
        if !vars.is_empty() && (candidates.is_empty() || self.rng.below(2) == 0) {
            let position = vars[self.rng.below(vars.len())];
            out.push(TermNode::Var {
                index: (self.binders.len() - 1 - position) as u64,
                sort: target.clone(),
            });
            return;
        }
        let chosen = candidates[self.rng.below(candidates.len())].clone();
        out.push(TermNode::App {
            symbol: chosen.clone(),
            args: chosen.arity.len(),
        });
        for arg in &chosen.arity {
            self.term(arg, budget - 1, out);
        }
    }

    fn whole_term(&mut self, target: &Sort, budget: usize) -> Term {
        let mut out = Vec::new();
        self.term(target, budget, &mut out);
        Term::new(out, Limits::default()).unwrap()
    }

    fn formula(&mut self, budget: usize) {
        let choice = if budget <= 1 {
            self.rng.below(3)
        } else {
            2 + self.rng.below(10)
        };
        match choice {
            0 => self.nodes.push(FormulaNode::True),
            1 => self.nodes.push(FormulaNode::False),
            2 => {
                let preds: Vec<Symbol> = self
                    .symbols(SymbolKind::Pred)
                    .into_iter()
                    .filter(|s| budget > 1 || s.arity.is_empty())
                    .collect();
                let chosen = preds[self.rng.below(preds.len())].clone();
                let args = chosen
                    .arity
                    .iter()
                    .map(|s| self.whole_term(s, budget - 1))
                    .collect();
                self.nodes.push(FormulaNode::Pred {
                    symbol: chosen,
                    args,
                });
            }
            3 => {
                let sorts: Vec<Sort> = self
                    .vocabulary
                    .sorts()
                    .iter()
                    .map(|d| Sort {
                        name: d.name.clone(),
                    })
                    .collect();
                let target = sorts[self.rng.below(sorts.len())].clone();
                let left = self.whole_term(&target, budget - 1);
                let right = self.whole_term(&target, budget - 1);
                self.nodes.push(FormulaNode::Eq(left, right));
            }
            4 => {
                self.nodes.push(FormulaNode::Not);
                self.formula(budget - 1);
            }
            5 | 6 => {
                let count = self.rng.below(4);
                self.nodes.push(if choice == 5 {
                    FormulaNode::And(count)
                } else {
                    FormulaNode::Or(count)
                });
                for _ in 0..count {
                    self.formula(budget - 1);
                }
            }
            7 | 8 => {
                self.nodes.push(if choice == 7 {
                    FormulaNode::Implies
                } else {
                    FormulaNode::Iff
                });
                self.formula(budget - 1);
                self.formula(budget - 1);
            }
            _ => {
                let decls = self.vocabulary.sorts();
                let s = Sort {
                    name: decls[self.rng.below(decls.len())].name.clone(),
                };
                self.nodes.push(if choice % 2 == 0 {
                    FormulaNode::Forall(s.clone())
                } else {
                    FormulaNode::Exists(s.clone())
                });
                self.binders.push(s);
                self.formula(budget - 1);
                self.binders.pop();
            }
        }
    }
}

/// Fórmula fechada e bem tipada com profundidade AIR ≤ `max_depth`.
pub fn random_formula(vocabulary: &Vocabulary, seed: u64, max_depth: usize) -> Formula {
    let mut generator = Generator {
        rng: Rng::new(seed),
        vocabulary,
        binders: Vec::new(),
        nodes: Vec::new(),
    };
    let budget = if generator.rng.below(5) == 0 {
        1 + generator.rng.below(max_depth)
    } else {
        max_depth
    };
    generator.formula(budget);
    Formula::new(generator.nodes, Limits::default()).unwrap()
}
