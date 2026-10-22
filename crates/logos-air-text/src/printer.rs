//! Pretty-printer determinístico e iterativo para AIR (air-text/v1).

use crate::types::Document;
use logos_air::{
    DomainSource, Formula, FormulaNode, Grounding, SortDomain, Symbol, SymbolKind, SymbolResult,
    Term, TermNode,
};
use std::fmt::Write as _;

struct TermFrame {
    remaining: usize,
}

struct FormulaFrame {
    remaining: usize,
    closing: &'static str,
    separator: &'static str,
}

struct Printer<'a> {
    out: &'a mut String,
    term_stack: Vec<TermFrame>,
    formula_stack: Vec<FormulaFrame>,
}

impl<'a> Printer<'a> {
    fn new(out: &'a mut String) -> Self {
        Self {
            out,
            term_stack: Vec::new(),
            formula_stack: Vec::new(),
        }
    }

    fn append_json_string(&mut self, s: &str) {
        self.out.push('"');
        for c in s.chars() {
            match c {
                '"' => self.out.push_str("\\\""),
                '\\' => self.out.push_str("\\\\"),
                '\x08' => self.out.push_str("\\b"),
                '\x0C' => self.out.push_str("\\f"),
                '\n' => self.out.push_str("\\n"),
                '\r' => self.out.push_str("\\r"),
                '\t' => self.out.push_str("\\t"),
                c if (c as u32) < 0x20 => {
                    let _ = write!(self.out, "\\u{:04x}", c as u32);
                }
                c => self.out.push(c),
            }
        }
        self.out.push('"');
    }

    fn append_symbol(&mut self, symbol: &Symbol) {
        self.out.push_str("symbol(");
        self.append_json_string(symbol.name.as_str());
        self.out.push_str(", ");
        match symbol.kind {
            SymbolKind::Const => self.out.push_str("const"),
            SymbolKind::Func => self.out.push_str("func"),
            SymbolKind::Pred => self.out.push_str("pred"),
        }
        self.out.push_str(", [");
        for (i, sort) in symbol.arity.iter().enumerate() {
            if i > 0 {
                self.out.push_str(", ");
            }
            self.append_json_string(sort.name.as_str());
        }
        self.out.push_str("], ");
        match &symbol.result {
            SymbolResult::Bool => self.out.push_str("bool"),
            SymbolResult::Sort(sort) => self.append_json_string(sort.name.as_str()),
        }
        self.out.push(')');
    }

    fn advance_term_frames(&mut self) {
        while let Some(top) = self.term_stack.last_mut() {
            top.remaining -= 1;
            if top.remaining > 0 {
                self.out.push_str(", ");
                break;
            }
            self.out.push_str("])");
            self.term_stack.pop();
        }
    }

    fn append_term(&mut self, term: &Term) {
        self.term_stack.clear();
        let nodes = term.nodes();

        for node in nodes {
            match node {
                TermNode::Var { index, sort } => {
                    self.out.push_str("var(");
                    let _ = write!(self.out, "{index}");
                    self.out.push_str(", ");
                    self.append_json_string(sort.name.as_str());
                    self.out.push(')');
                    self.advance_term_frames();
                }
                TermNode::App { symbol, args } => {
                    if *args == 0 {
                        self.out.push_str("app(");
                        self.append_symbol(symbol);
                        self.out.push_str(", [])");
                        self.advance_term_frames();
                    } else {
                        self.out.push_str("app(");
                        self.append_symbol(symbol);
                        self.out.push_str(", [");
                        self.term_stack.push(TermFrame { remaining: *args });
                    }
                }
            }
        }
    }

    fn advance_formula_frames(&mut self) {
        while let Some(top) = self.formula_stack.last_mut() {
            top.remaining -= 1;
            if top.remaining > 0 {
                self.out.push_str(top.separator);
                break;
            }
            self.out.push_str(top.closing);
            self.formula_stack.pop();
        }
    }

    fn append_formula(&mut self, formula: &Formula) {
        self.formula_stack.clear();
        let nodes = formula.nodes();

        for (node_index, node) in nodes.iter().enumerate() {
            match node {
                FormulaNode::True => {
                    self.out.push_str("true");
                    self.advance_formula_frames();
                }
                FormulaNode::False => {
                    self.out.push_str("false");
                    self.advance_formula_frames();
                }
                FormulaNode::Eq(left, right) => {
                    self.out.push_str("eq(");
                    self.append_term(left);
                    self.out.push_str(", ");
                    self.append_term(right);
                    self.out.push(')');
                    self.advance_formula_frames();
                }
                FormulaNode::Pred { symbol, args } => {
                    self.out.push_str("pred(");
                    self.append_symbol(symbol);
                    self.out.push_str(", [");
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            self.out.push_str(", ");
                        }
                        self.append_term(arg);
                    }
                    self.out.push_str("])");
                    self.advance_formula_frames();
                }
                FormulaNode::Not => {
                    self.out.push_str("not(");
                    self.formula_stack.push(FormulaFrame {
                        remaining: 1,
                        closing: ")",
                        separator: "",
                    });
                }
                FormulaNode::And(count) => {
                    if *count == 0 {
                        self.out.push_str("and([])");
                        self.advance_formula_frames();
                    } else {
                        self.out.push_str("and([");
                        self.formula_stack.push(FormulaFrame {
                            remaining: *count,
                            closing: "])",
                            separator: ", ",
                        });
                    }
                }
                FormulaNode::Or(count) => {
                    if *count == 0 {
                        self.out.push_str("or([])");
                        self.advance_formula_frames();
                    } else {
                        self.out.push_str("or([");
                        self.formula_stack.push(FormulaFrame {
                            remaining: *count,
                            closing: "])",
                            separator: ", ",
                        });
                    }
                }
                FormulaNode::Implies => {
                    self.out.push_str("implies(");
                    self.formula_stack.push(FormulaFrame {
                        remaining: 2,
                        closing: ")",
                        separator: ", ",
                    });
                }
                FormulaNode::Iff => {
                    self.out.push_str("iff(");
                    self.formula_stack.push(FormulaFrame {
                        remaining: 2,
                        closing: ")",
                        separator: ", ",
                    });
                }
                FormulaNode::Forall(sort) => {
                    self.out.push_str("forall \"_q");
                    let _ = write!(self.out, "{node_index}");
                    self.out.push_str("\": ");
                    self.append_json_string(sort.name.as_str());
                    self.out.push_str(". ");
                    self.formula_stack.push(FormulaFrame {
                        remaining: 1,
                        closing: "",
                        separator: "",
                    });
                }
                FormulaNode::Exists(sort) => {
                    self.out.push_str("exists \"_q");
                    let _ = write!(self.out, "{node_index}");
                    self.out.push_str("\": ");
                    self.append_json_string(sort.name.as_str());
                    self.out.push_str(". ");
                    self.formula_stack.push(FormulaFrame {
                        remaining: 1,
                        closing: "",
                        separator: "",
                    });
                }
            }
        }
    }
}

/// Escapa uma string como literal JSON entre aspas duplas.
#[must_use]
pub fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    let mut printer = Printer::new(&mut out);
    printer.append_json_string(s);
    out
}

/// Imprime uma assinatura `Symbol` no formato explícito `symbol(...)`.
#[must_use]
pub fn print_symbol(symbol: &Symbol) -> String {
    let mut out = String::new();
    let mut printer = Printer::new(&mut out);
    printer.append_symbol(symbol);
    out
}

/// Imprime um `Term` de forma plana e iterativa sem recursão.
#[must_use]
pub fn print_term(term: &Term) -> String {
    let mut out = String::new();
    let mut printer = Printer::new(&mut out);
    printer.append_term(term);
    out
}

/// Imprime uma `Formula` de forma determinística, plana e iterativa sem recursão.
///
/// Produz sintaxe explícita com strings JSON para identificadores, construtores
/// canônicos e quantificadores nominais indexados por `_q<índice do record>`.
#[must_use]
pub fn print_formula(formula: &Formula) -> String {
    let mut out = String::new();
    let mut printer = Printer::new(&mut out);
    printer.append_formula(formula);
    out
}

/// Imprime um `Document` completo contendo vocabulário e claims.
///
/// Preserva namespaces, sorts, ordens canônicas, fontes de fecho em minúsculas hex
/// e citações de claims.
#[must_use]
pub fn print_document(document: &Document) -> String {
    let mut out = String::new();
    let mut printer = Printer::new(&mut out);
    let _ = writeln!(printer.out, "vocab v{} {{", document.vocabulary.version());

    // Sort declarations
    for sort_decl in document.vocabulary.sorts() {
        printer.out.push_str("  sort ");
        printer.append_json_string(sort_decl.name.as_str());
        match &sort_decl.domain {
            SortDomain::Open => {}
            SortDomain::Enumerated {
                constants,
                domain_source,
            } => {
                printer.out.push_str(" = enum [");
                for (i, c) in constants.iter().enumerate() {
                    if i > 0 {
                        printer.out.push_str(", ");
                    }
                    printer.append_json_string(c.name.as_str());
                }
                printer.out.push_str("] from ");
                match domain_source {
                    DomainSource::Fact(fact_ref) => {
                        printer.out.push_str("fact \"");
                        for b in &fact_ref.0 {
                            let _ = write!(printer.out, "{:02x}", b);
                        }
                        printer.out.push('"');
                    }
                    DomainSource::Policy(policy_ref) => {
                        printer.out.push_str("policy \"");
                        for b in &policy_ref.0 {
                            let _ = write!(printer.out, "{:02x}", b);
                        }
                        printer.out.push('"');
                    }
                }
            }
        }
        printer.out.push('\n');
    }

    // Symbol declarations
    for sym_decl in document.vocabulary.symbols() {
        printer.out.push_str("  decl ");
        printer.append_symbol(&sym_decl.signature);
        printer.out.push_str(" = ");
        match &sym_decl.grounding {
            Grounding::Glossed(text) => {
                printer.out.push_str("glossed(");
                printer.append_json_string(text.as_str());
                printer.out.push(')');
            }
            Grounding::Anchored {
                attestor,
                procedure,
            } => {
                printer.out.push_str("anchored(");
                printer.append_json_string(attestor.0.as_str());
                printer.out.push_str(", ");
                printer.append_json_string(procedure.as_str());
                printer.out.push(')');
            }
            Grounding::Defined(formula) => {
                printer.out.push_str("defined(");
                printer.append_formula(formula);
                printer.out.push(')');
            }
        }
        printer.out.push_str(" gloss ");
        printer.append_json_string(sym_decl.gloss_pt.as_str());
        printer.out.push('\n');
    }

    if let Some(templates) = document.vocabulary.render_pt() {
        printer.out.push_str("  render_pt [");
        for (index, template) in templates.iter().enumerate() {
            if index != 0 {
                printer.out.push_str(", ");
            }
            printer.append_json_string(template.as_str());
        }
        printer.out.push_str("]\n");
    }
    printer.out.push_str("}\n");

    // Claims
    for claim in &document.claims {
        printer.out.push_str("claim ");
        printer.append_json_string(claim.name.as_str());
        printer.out.push_str(": ");
        printer.append_formula(&claim.formula);
        if !claim.cites.is_empty() {
            printer.out.push_str(" cites [");
            for (i, cite) in claim.cites.iter().enumerate() {
                if i > 0 {
                    printer.out.push_str(", ");
                }
                printer.append_json_string(cite.as_str());
            }
            printer.out.push(']');
        }
        printer.out.push('\n');
    }

    out
}
