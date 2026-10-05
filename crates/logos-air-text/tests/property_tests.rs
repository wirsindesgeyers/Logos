//! Testes baseados em propriedades com gerador determinístico (sem dependências externas):
//! validação estrita de `parse_formula(print_formula(f), V) == f` para todas as variantes de
//! nós de fórmulas, termos, conectivos n-ários/vazios e casos sintáticos mal tipados.

use logos_air::{
    DomainSource, FactRef, Formula, FormulaNode, Grounding, Limits, PolicyRef, Sort, SortDecl,
    SortDomain, Symbol, SymbolKind, SymbolResult, Term, TermNode, Vocabulary,
};
use logos_air_text::{
    Claim, Document, References, parse_document, parse_formula, print_document, print_formula,
};
use logos_canon::Text;

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    fn next_range(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            (self.next_u64() as usize) % bound
        }
    }

    fn next_bool(&mut self) -> bool {
        (self.next_u64() & 1) == 1
    }

    fn next_sort_name(&mut self) -> Text {
        const NAMES: &[&str] = &["User", "Db", "Action", "bool", "Item", "Value", "T_42"];
        let s = NAMES[self.next_range(NAMES.len())];
        Text::new(s).expect("valid sort name")
    }

    fn next_symbol_name(&mut self) -> Text {
        const NAMES: &[&str] = &[
            "Admin",
            "Owns",
            "dbOf",
            "p",
            "q",
            "f",
            "c",
            "WeirdSym",
            "check_status",
        ];
        let s = NAMES[self.next_range(NAMES.len())];
        Text::new(s).expect("valid symbol name")
    }

    fn gen_symbol(&mut self, arity_len: usize) -> Symbol {
        let name = self.next_symbol_name();
        let kind = match self.next_range(3) {
            0 => SymbolKind::Const,
            1 => SymbolKind::Func,
            _ => SymbolKind::Pred,
        };
        let mut arity = Vec::with_capacity(arity_len);
        for _ in 0..arity_len {
            arity.push(Sort {
                name: self.next_sort_name(),
            });
        }
        let result = if self.next_bool() {
            SymbolResult::Bool
        } else {
            SymbolResult::Sort(Sort {
                name: self.next_sort_name(),
            })
        };
        Symbol {
            name,
            kind,
            arity,
            result,
        }
    }

    fn gen_term(&mut self, max_depth: usize) -> Term {
        let mut nodes = Vec::new();
        self.gen_term_nodes(&mut nodes, max_depth);
        Term::new(nodes, Limits::default()).expect("valid term prefix tree")
    }

    fn gen_term_nodes(&mut self, nodes: &mut Vec<TermNode>, max_depth: usize) {
        if max_depth <= 1 || self.next_range(10) < 4 {
            // Leaf: Var
            let index = self.next_range(100) as u64;
            let sort = Sort {
                name: self.next_sort_name(),
            };
            nodes.push(TermNode::Var { index, sort });
        } else {
            // App
            let args_count = self.next_range(4); // 0..=3 args
            let declared_arity = self.next_range(4); // Pode diferir de args_count (mal tipado!)
            let symbol = self.gen_symbol(declared_arity);
            nodes.push(TermNode::App {
                symbol,
                args: args_count,
            });
            for _ in 0..args_count {
                self.gen_term_nodes(nodes, max_depth - 1);
            }
        }
    }

    fn gen_formula(&mut self, max_depth: usize) -> Formula {
        let mut nodes = Vec::new();
        self.gen_formula_nodes(&mut nodes, max_depth);
        Formula::new(nodes, Limits::default()).expect("valid formula prefix tree")
    }

    fn gen_formula_nodes(&mut self, nodes: &mut Vec<FormulaNode>, max_depth: usize) {
        if max_depth <= 1 {
            match self.next_range(4) {
                0 => nodes.push(FormulaNode::True),
                1 => nodes.push(FormulaNode::False),
                2 => {
                    let left = self.gen_term(2);
                    let right = self.gen_term(2);
                    nodes.push(FormulaNode::Eq(left, right));
                }
                _ => {
                    let args_count = self.next_range(3);
                    let symbol = self.gen_symbol(args_count);
                    let mut args = Vec::with_capacity(args_count);
                    for _ in 0..args_count {
                        args.push(self.gen_term(2));
                    }
                    nodes.push(FormulaNode::Pred { symbol, args });
                }
            }
            return;
        }

        match self.next_range(10) {
            0 => nodes.push(FormulaNode::True),
            1 => nodes.push(FormulaNode::False),
            2 => {
                let left = self.gen_term(3);
                let right = self.gen_term(3);
                nodes.push(FormulaNode::Eq(left, right));
            }
            3 => {
                let args_count = self.next_range(4);
                let symbol = self.gen_symbol(args_count);
                let mut args = Vec::with_capacity(args_count);
                for _ in 0..args_count {
                    args.push(self.gen_term(3));
                }
                nodes.push(FormulaNode::Pred { symbol, args });
            }
            4 => {
                nodes.push(FormulaNode::Not);
                self.gen_formula_nodes(nodes, max_depth - 1);
            }
            5 => {
                // And com 0, 1, 2, ou 3 filhos
                let count = match self.next_range(4) {
                    0 => 0,
                    1 => 1,
                    2 => 2,
                    _ => 3,
                };
                nodes.push(FormulaNode::And(count));
                for _ in 0..count {
                    self.gen_formula_nodes(nodes, max_depth - 1);
                }
            }
            6 => {
                // Or com 0, 1, 2, ou 3 filhos
                let count = match self.next_range(4) {
                    0 => 0,
                    1 => 1,
                    2 => 2,
                    _ => 3,
                };
                nodes.push(FormulaNode::Or(count));
                for _ in 0..count {
                    self.gen_formula_nodes(nodes, max_depth - 1);
                }
            }
            7 => {
                nodes.push(FormulaNode::Implies);
                self.gen_formula_nodes(nodes, max_depth - 1);
                self.gen_formula_nodes(nodes, max_depth - 1);
            }
            8 => {
                nodes.push(FormulaNode::Iff);
                self.gen_formula_nodes(nodes, max_depth - 1);
                self.gen_formula_nodes(nodes, max_depth - 1);
            }
            _ => {
                let sort = Sort {
                    name: self.next_sort_name(),
                };
                if self.next_bool() {
                    nodes.push(FormulaNode::Forall(sort));
                } else {
                    nodes.push(FormulaNode::Exists(sort));
                }
                self.gen_formula_nodes(nodes, max_depth - 1);
            }
        }
    }
}

fn empty_vocab() -> Vocabulary {
    Vocabulary::new(1, Vec::new(), Vec::new()).expect("empty vocab is valid")
}

#[test]
fn test_property_formula_roundtrip_500_cases() {
    let mut rng = Lcg::new(0xCAFE_BABE_1234_5678);
    let vocab = empty_vocab();

    for i in 0..500 {
        // Gera fórmulas com profundidade entre 1 e 12
        let depth = 1 + (i % 8);
        let formula = rng.gen_formula(depth);

        let printed = print_formula(&formula);
        let roundtrip = parse_formula(&printed, &vocab).unwrap_or_else(|e| {
            panic!("Roundtrip parse failed for formula #{i}: {e}\nPrinted: {printed}")
        });

        assert_eq!(
            formula, roundtrip,
            "Formula #{i} identity changed across round-trip!\nOriginal: {formula:?}\nParsed: {roundtrip:?}\nPrinted: {printed}"
        );
    }
}

#[test]
fn test_property_document_roundtrip_diverse_variants() {
    let mut rng = Lcg::new(0xDEAD_BEEF_8765_4321);
    let references = References::new();

    for i in 0..50 {
        let version = (i as u64) + 1;
        let mut sorts = Vec::new();
        let s_name = Text::new(format!("Sort_{i}")).unwrap();
        sorts.push(SortDecl {
            name: s_name.clone(),
            domain: SortDomain::Open,
        });

        // Sort com enum
        let enum_name = Text::new(format!("EnumSort_{i}")).unwrap();
        let const_name = Text::new(format!("c_{i}")).unwrap();
        let const_sym = Symbol {
            name: const_name.clone(),
            kind: SymbolKind::Const,
            arity: Vec::new(),
            result: SymbolResult::Sort(Sort {
                name: enum_name.clone(),
            }),
        };

        let mut digest = [0u8; 32];
        for b in &mut digest {
            *b = rng.next_range(256) as u8;
        }

        let domain_source = if rng.next_bool() {
            DomainSource::Fact(FactRef(digest))
        } else {
            DomainSource::Policy(PolicyRef(digest))
        };

        sorts.push(SortDecl {
            name: enum_name.clone(),
            domain: SortDomain::Enumerated {
                constants: vec![const_sym.clone()],
                domain_source,
            },
        });

        let mut symbols = Vec::new();
        symbols.push(logos_air::SymbolDecl {
            signature: const_sym,
            grounding: Grounding::Glossed(Text::new("const").unwrap()),
            gloss_pt: Text::new("constante").unwrap(),
        });

        // Anchored grounding
        symbols.push(logos_air::SymbolDecl {
            signature: Symbol {
                name: Text::new(format!("anchored_pred_{i}")).unwrap(),
                kind: SymbolKind::Pred,
                arity: vec![Sort { name: s_name }],
                result: SymbolResult::Bool,
            },
            grounding: Grounding::Anchored {
                attestor: logos_air::AttestorId(Text::new("attestor").unwrap()),
                procedure: Text::new("check").unwrap(),
            },
            gloss_pt: Text::new("ancorado").unwrap(),
        });

        let vocab = Vocabulary::new(version, sorts, symbols).expect("valid vocabulary");

        let mut claims = Vec::new();
        let claim_count = rng.next_range(4);
        for c in 0..claim_count {
            let formula = rng.gen_formula(3);
            let mut cites = Vec::new();
            let cites_count = rng.next_range(3);
            for k in 0..cites_count {
                cites.push(Text::new(format!("Cite_{k}")).unwrap());
            }
            claims.push(Claim {
                name: Text::new(format!("Claim_{c}")).unwrap(),
                formula,
                cites,
            });
        }

        let doc = Document {
            vocabulary: vocab,
            claims,
        };

        let printed = print_document(&doc);
        let roundtrip = parse_document(&printed, &references)
            .unwrap_or_else(|e| panic!("Document #{i} parse failed: {e}\nPrinted:\n{printed}"));

        assert_eq!(
            doc.vocabulary, roundtrip.vocabulary,
            "Document #{i} vocab mismatch"
        );
        assert_eq!(
            doc.claims, roundtrip.claims,
            "Document #{i} claims mismatch"
        );
    }
}
