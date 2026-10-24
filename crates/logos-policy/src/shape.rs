//! Forma de uma regra: cabeça e dependências do corpo (Horn restrito) em uma fórmula AIR.
//!
//! Uma regra `allow`/`deny`/`derive` é `∀x̄. corpo → cabeça` (ou só a cabeça). O corpo é uma
//! conjunção de átomos, igualdades, átomos negados e **blocos negados** `¬∃ȳ. (conjunção
//! positiva)`; `∃` positivo no corpo só introduz variáveis. Tudo o mais (`∨`, `↔`, `→`
//! aninhado, `∀` no corpo, negação dupla) fica fora do fragmento decidível e só pode ser regra
//! `require`, que exige certificado (L2).

use logos_air::{Formula, FormulaNode, TermNode};

use crate::{ALLOWED, Class, PolicyError};

/// Árvore de uma fórmula em pré-ordem plana: índices dos filhos de cada nó.
///
/// `Formula` já garante pré-ordem bem formada (aridades fecham em uma única raiz), então a
/// reconstrução não tem caminho de erro.
pub(crate) struct Tree<'a> {
    pub(crate) nodes: &'a [FormulaNode],
    kids: Vec<Vec<usize>>,
}

impl<'a> Tree<'a> {
    /// Reconstrói os filhos sem recursão (pré-ordem lida de trás para frente).
    pub(crate) fn new(formula: &'a Formula) -> Self {
        let nodes = formula.nodes();
        let mut kids: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
        let mut stack: Vec<usize> = Vec::new();
        for (index, node) in nodes.iter().enumerate().rev() {
            let arity = match node {
                FormulaNode::Not | FormulaNode::Forall(_) | FormulaNode::Exists(_) => 1,
                FormulaNode::And(n) | FormulaNode::Or(n) => *n,
                FormulaNode::Implies | FormulaNode::Iff => 2,
                FormulaNode::True
                | FormulaNode::False
                | FormulaNode::Eq(..)
                | FormulaNode::Pred { .. } => 0,
            };
            let first = stack.len().saturating_sub(arity);
            // O topo da pilha é o primeiro filho: a ordem sai invertida e é corrigida.
            let mut children: Vec<usize> = stack.drain(first..).collect();
            children.reverse();
            kids[index] = children;
            stack.push(index);
        }
        Self { nodes, kids }
    }

    /// Algum átomo usa o predicado `Allowed`?
    pub(crate) fn mentions_allowed(&self) -> bool {
        self.nodes.iter().any(is_allowed_atom)
    }
}

fn is_allowed_atom(node: &FormulaNode) -> bool {
    matches!(node, FormulaNode::Pred { symbol, .. } if symbol.name.as_str() == ALLOWED)
}

/// Predicados de que o corpo de uma regra depende, com a polaridade.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Shape {
    /// Predicados em posição positiva.
    pub positive: Vec<String>,
    /// Predicados sob negação (átomo negado ou bloco negado).
    pub negative: Vec<String>,
    /// Nome do predicado da cabeça (`Allowed` para `allow` e `deny`).
    pub head: String,
    /// Símbolo-cabeça do termo de ação da cabeça (`Allowed(drop(x))` → `drop`); vazio quando o
    /// argumento é variável ou a cabeça não é `Allowed`. É a chave de indexação (7.2).
    pub action: String,
}

#[derive(Clone, Copy)]
enum Mode {
    Positive,
    Negated,
}

fn atom_name(node: &FormulaNode) -> Option<&str> {
    match node {
        FormulaNode::Pred { symbol, .. } => Some(symbol.name.as_str()),
        _ => None,
    }
}

/// Analisa uma regra de classe `allow`, `deny` ou `derive`.
pub(crate) fn analyse(id: &str, class: Class, tree: &Tree<'_>) -> Result<Shape, PolicyError> {
    let not_horn = |why| PolicyError::NotHorn(id.to_owned(), why);
    let bad_head = |why| PolicyError::BadHead(id.to_owned(), why);
    let mut at = 0;
    while matches!(tree.nodes[at], FormulaNode::Forall(_)) {
        at = tree.kids[at][0];
    }
    let (body, head) = if matches!(tree.nodes[at], FormulaNode::Implies) {
        (Some(tree.kids[at][0]), tree.kids[at][1])
    } else {
        (None, at)
    };
    // `deny` conclui `¬Allowed(...)`: o átomo é o filho da negação.
    let (negated_head, atom) = match tree.nodes[head] {
        FormulaNode::Not => (true, tree.kids[head][0]),
        _ => (false, head),
    };
    let name = atom_name(&tree.nodes[atom]);
    let head_name = match (class, negated_head, name) {
        (Class::Allow, false, Some(ALLOWED)) | (Class::Deny, true, Some(ALLOWED)) => ALLOWED,
        (Class::Allow, _, _) => return Err(bad_head("an allow rule must conclude Allowed(...)")),
        (Class::Deny, _, _) => {
            return Err(bad_head("a deny rule must conclude not Allowed(...)"));
        }
        (Class::Derive, false, Some(ALLOWED)) => {
            return Err(bad_head("a derive rule cannot define Allowed"));
        }
        (Class::Derive, false, Some(other)) => other,
        // `require` não chega aqui (o chamador a trata como fórmula geral).
        (Class::Derive | Class::Require, ..) => {
            return Err(bad_head(
                "a derive rule must conclude a positive predicate atom",
            ));
        }
    };
    // `Allowed` é unário (conferido no vocabulário) e todo termo tem ao menos um nó.
    let action = match &tree.nodes[atom] {
        FormulaNode::Pred { args, .. } if head_name == ALLOWED => match &args[0].nodes()[0] {
            TermNode::App { symbol, .. } => symbol.name.as_str().to_owned(),
            TermNode::Var { .. } => String::new(),
        },
        _ => String::new(),
    };
    let mut shape = Shape {
        head: head_name.to_owned(),
        action,
        ..Shape::default()
    };
    let mut work: Vec<(usize, Mode)> = body.map(|b| (b, Mode::Positive)).into_iter().collect();
    while let Some((index, mode)) = work.pop() {
        match (&tree.nodes[index], mode) {
            (FormulaNode::And(_) | FormulaNode::Exists(_), mode) => {
                work.extend(tree.kids[index].iter().map(|child| (*child, mode)));
            }
            (FormulaNode::Pred { symbol, .. }, Mode::Positive) => {
                shape.positive.push(symbol.name.as_str().to_owned());
            }
            (FormulaNode::Pred { symbol, .. }, Mode::Negated) => {
                shape.negative.push(symbol.name.as_str().to_owned());
            }
            (FormulaNode::Eq(..) | FormulaNode::True, _) => {}
            (FormulaNode::Not, Mode::Positive) => work.push((tree.kids[index][0], Mode::Negated)),
            (FormulaNode::Not, Mode::Negated) => return Err(not_horn("nested negation")),
            _ => return Err(not_horn("connective outside the Horn fragment")),
        }
    }
    shape.positive.sort();
    shape.positive.dedup();
    shape.negative.sort();
    shape.negative.dedup();
    Ok(shape)
}
