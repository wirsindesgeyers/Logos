//! Sort-check iterativo de fórmulas fechadas sobre um vocabulário declarado.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use logos_canon::Text;

use crate::{
    Formula, FormulaNode, Sort, Symbol, SymbolKind, SymbolResult, Term, TermNode, Vocabulary,
};

/// Posição de um termo embutido no record de fórmula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermPosition {
    /// Termo esquerdo de Eq.
    EqualityLeft,
    /// Termo direito de Eq.
    EqualityRight,
    /// Argumento zero-based de Pred.
    PredicateArgument(usize),
}

/// Coordenada zero-based dentro do prefixo de um termo embutido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermPath {
    /// Termo embutido que contém o nó.
    pub position: TermPosition,
    /// Índice do record TermNode no prefixo do termo.
    pub node: usize,
}

/// Caminho exato no armazenamento plano; não depende de nomes de binders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortPath {
    /// Índice do record FormulaNode no prefixo da fórmula.
    pub formula_node: usize,
    /// Coordenada de termo embutido, quando o erro ocorre dentro dele.
    pub term: Option<TermPath>,
}

/// Motivo de recusa de tipagem, não veredicto lógico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortErrorKind {
    /// Nome de sort não declarado.
    UnknownSort,
    /// Nome de símbolo não declarado.
    UnknownSymbol,
    /// Assinatura completa diverge da declaração de mesmo nome.
    SignatureMismatch,
    /// Pred usado como termo, ou Const/Func usados como predicado.
    InvalidSymbolKind,
    /// Pred não retorna Bool, ou termo aplicado não retorna Sort.
    InvalidSymbolResult,
    /// Constante declarada com argumentos de entrada.
    InvalidConstantArity {
        /// Quantidade de sorts na assinatura inválida.
        actual: usize,
    },
    /// Quantidade de filhos/argumentos diverge da assinatura.
    ArityMismatch {
        /// Quantidade declarada.
        expected: usize,
        /// Quantidade usada.
        actual: usize,
    },
    /// Índice De Bruijn não encontra binder ativo.
    UnboundVariable {
        /// Índice original, sem truncamento.
        index: u64,
        /// Quantidade de binders ativos nesta posição.
        binders: usize,
    },
    /// Sort anotado/resultante diverge do requerido pelo contexto.
    SortMismatch {
        /// Sort requerido pelo binder, assinatura ou igualdade.
        expected: Sort,
        /// Sort encontrado no nó.
        actual: Sort,
    },
}

/// Falha de sort-check com localização; nunca um Refuted/Unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortError {
    /// Coordenada do primeiro nó recusado em pré-ordem.
    pub path: SortPath,
    /// Motivo da recusa.
    pub kind: SortErrorKind,
}

impl fmt::Display for SortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sort error at {:?}: {:?}", self.path, self.kind)
    }
}

impl Error for SortError {}

/// Confere tipagem e fechamento de uma fórmula estruturalmente válida.
///
/// Apenas assinaturas usadas são conferidas. Não expande nem tipa corpos Defined,
/// não resolve grounding e não prova a fórmula. Percurso iterativo, sem cópia de
/// AST ou nomes; aloca somente índices e stacks de profundidade/declarações.
///
/// # Errors
/// Primeiro erro de declaração, assinatura, kind/resultado, aridade, sort ou
/// ligação de variável em pré-ordem, com coordenada exata na árvore plana.
pub fn check(vocabulary: &Vocabulary, formula: &Formula) -> Result<(), SortError> {
    let mut checker = Checker {
        sorts: vocabulary.sorts().iter().map(|decl| &decl.name).collect(),
        symbols: vocabulary
            .symbols()
            .iter()
            .map(|decl| (&decl.signature.name, &decl.signature))
            .collect(),
        term_frames: Vec::new(),
    };
    let mut frames: Vec<FormulaFrame> = Vec::new();
    let mut binders = Vec::new();
    for (index, node) in formula.nodes().iter().enumerate() {
        while let Some(frame) = frames.last() {
            if frame.remaining != 0 {
                break;
            }
            let before = frame.binders_before;
            frames.pop();
            binders.truncate(before);
        }
        if let Some(parent) = frames.last_mut() {
            parent.remaining -= 1;
        }
        let path = SortPath {
            formula_node: index,
            term: None,
        };
        let before = binders.len();
        match node {
            FormulaNode::Eq(left, right) => {
                let sort = checker.term(left, &binders, path, TermPosition::EqualityLeft, None)?;
                checker.term(
                    right,
                    &binders,
                    path,
                    TermPosition::EqualityRight,
                    Some(sort),
                )?;
            }
            FormulaNode::Pred { symbol, args } => {
                checker.symbol(symbol, path)?;
                if symbol.kind != SymbolKind::Pred {
                    return Err(failure(path, SortErrorKind::InvalidSymbolKind));
                }
                if symbol.result != SymbolResult::Bool {
                    return Err(failure(path, SortErrorKind::InvalidSymbolResult));
                }
                arity(symbol.arity.len(), args.len(), path)?;
                for (arg_index, (term, expected)) in args.iter().zip(&symbol.arity).enumerate() {
                    checker.term(
                        term,
                        &binders,
                        path,
                        TermPosition::PredicateArgument(arg_index),
                        Some(expected),
                    )?;
                }
            }
            FormulaNode::Forall(sort) | FormulaNode::Exists(sort) => {
                checker.sort(sort, path)?;
                binders.push(sort);
            }
            _ => {}
        }
        let children = node.children();
        if children != 0 {
            frames.push(FormulaFrame {
                remaining: children,
                binders_before: before,
            });
        }
    }
    Ok(())
}

fn failure(path: SortPath, kind: SortErrorKind) -> SortError {
    SortError { path, kind }
}

fn arity(expected: usize, actual: usize, path: SortPath) -> Result<(), SortError> {
    if expected != actual {
        return Err(failure(
            path,
            SortErrorKind::ArityMismatch { expected, actual },
        ));
    }
    Ok(())
}

fn expect_sort(actual: &Sort, expected: &Sort, path: SortPath) -> Result<(), SortError> {
    if actual != expected {
        return Err(failure(
            path,
            SortErrorKind::SortMismatch {
                expected: expected.clone(),
                actual: actual.clone(),
            },
        ));
    }
    Ok(())
}

struct FormulaFrame {
    remaining: usize,
    binders_before: usize,
}

struct TermFrame<'a> {
    arity: &'a [Sort],
    next: usize,
}

struct Checker<'a> {
    sorts: BTreeSet<&'a Text>,
    symbols: BTreeMap<&'a Text, &'a Symbol>,
    term_frames: Vec<TermFrame<'a>>,
}

impl<'a> Checker<'a> {
    fn sort(&self, sort: &Sort, path: SortPath) -> Result<(), SortError> {
        if !self.sorts.contains(&sort.name) {
            return Err(failure(path, SortErrorKind::UnknownSort));
        }
        Ok(())
    }

    fn symbol(&self, symbol: &Symbol, path: SortPath) -> Result<(), SortError> {
        let declared = self
            .symbols
            .get(&symbol.name)
            .ok_or_else(|| failure(path, SortErrorKind::UnknownSymbol))?;
        if **declared != *symbol {
            return Err(failure(path, SortErrorKind::SignatureMismatch));
        }
        Ok(())
    }

    fn term_node(
        &mut self,
        node: &'a TermNode,
        binders: &[&Sort],
        path: SortPath,
    ) -> Result<&'a Sort, SortError> {
        match node {
            TermNode::Var { index, sort } => {
                self.sort(sort, path)?;
                if u128::from(*index) >= binders.len() as u128 {
                    return Err(failure(
                        path,
                        SortErrorKind::UnboundVariable {
                            index: *index,
                            binders: binders.len(),
                        },
                    ));
                }
                // index < binders.len(): conversion and subtraction cannot truncate/underflow.
                let expected = binders[binders.len() - 1 - *index as usize];
                expect_sort(sort, expected, path)?;
                Ok(sort)
            }
            TermNode::App { symbol, args } => {
                self.symbol(symbol, path)?;
                if symbol.kind == SymbolKind::Pred {
                    return Err(failure(path, SortErrorKind::InvalidSymbolKind));
                }
                let SymbolResult::Sort(sort) = &symbol.result else {
                    return Err(failure(path, SortErrorKind::InvalidSymbolResult));
                };
                if symbol.kind == SymbolKind::Const && !symbol.arity.is_empty() {
                    return Err(failure(
                        path,
                        SortErrorKind::InvalidConstantArity {
                            actual: symbol.arity.len(),
                        },
                    ));
                }
                arity(symbol.arity.len(), *args, path)?;
                if *args != 0 {
                    self.term_frames.push(TermFrame {
                        arity: &symbol.arity,
                        next: 0,
                    });
                }
                Ok(sort)
            }
        }
    }

    fn term(
        &mut self,
        term: &'a Term,
        binders: &[&Sort],
        mut path: SortPath,
        position: TermPosition,
        expected: Option<&Sort>,
    ) -> Result<&'a Sort, SortError> {
        self.term_frames.clear();
        path.term = Some(TermPath { position, node: 0 });
        // Term construction guarantees a nonempty, single-root prefix.
        let root_sort = self.term_node(&term.nodes()[0], binders, path)?;
        if let Some(expected) = expected {
            expect_sort(root_sort, expected, path)?;
        }
        for (index, node) in term.nodes().iter().enumerate().skip(1) {
            loop {
                let last = self.term_frames.len() - 1;
                if self.term_frames[last].next != self.term_frames[last].arity.len() {
                    break;
                }
                self.term_frames.pop();
            }
            // Valid prefix and checked application arities guarantee a parent slot.
            let last = self.term_frames.len() - 1;
            let frame = &mut self.term_frames[last];
            let expected = &frame.arity[frame.next];
            frame.next += 1;
            path.term = Some(TermPath {
                position,
                node: index,
            });
            let actual = self.term_node(node, binders, path)?;
            expect_sort(actual, expected, path)?;
        }
        Ok(root_sort)
    }
}
