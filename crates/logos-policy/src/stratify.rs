//! Estratificação: nenhum predicado depende de si mesmo através de uma negação.
//!
//! Nós: cada predicado, mais `AllowFires` (algum corpo `allow` dispara) e `DenyFires` (algum
//! corpo `deny` dispara). `Allowed` depende positivamente de `AllowFires` e negativamente de
//! `DenyFires` (deny-overrides), de modo que um corpo `deny` que dependa de `Allowed` fecha um
//! ciclo com negação e é recusado. Componentes fortemente conexos por Kosaraju iterativo.

use std::collections::BTreeMap;

use crate::{ALLOWED, Class, PolicyError, Shape};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Node {
    Pred(String),
    AllowFires,
    DenyFires,
}

impl Node {
    fn label(&self) -> String {
        match self {
            Self::Pred(name) => name.clone(),
            Self::AllowFires => "(allow rules)".to_owned(),
            Self::DenyFires => "(deny rules)".to_owned(),
        }
    }
}

struct Graph {
    index: BTreeMap<Node, usize>,
    names: Vec<Node>,
    edges: Vec<(usize, usize, bool)>,
}

impl Graph {
    fn id(&mut self, node: Node) -> usize {
        if let Some(i) = self.index.get(&node) {
            return *i;
        }
        let i = self.names.len();
        self.index.insert(node.clone(), i);
        self.names.push(node);
        i
    }

    fn edge(&mut self, from: Node, to: Node, negative: bool) {
        let (a, b) = (self.id(from), self.id(to));
        self.edges.push((a, b, negative));
    }
}

/// Iterativo: ordem de término (1ª passada) e atribuição de componentes (2ª, no grafo reverso).
/// Os índices vêm de `Graph::id`, logo estão sempre dentro de `0..count`.
fn components(count: usize, edges: &[(usize, usize, bool)]) -> Vec<usize> {
    let mut forward = vec![Vec::new(); count];
    let mut reverse = vec![Vec::new(); count];
    for (a, b, _) in edges {
        forward[*a].push(*b);
        reverse[*b].push(*a);
    }
    let mut seen = vec![false; count];
    let mut order = Vec::with_capacity(count);
    for start in 0..count {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut stack = vec![(start, 0usize)];
        while let Some((node, next)) = stack.pop() {
            if let Some(to) = forward[node].get(next).copied() {
                stack.push((node, next + 1));
                if !seen[to] {
                    seen[to] = true;
                    stack.push((to, 0));
                }
            } else {
                order.push(node);
            }
        }
    }
    let mut component = vec![usize::MAX; count];
    let mut label = 0;
    for start in order.into_iter().rev() {
        if component[start] != usize::MAX {
            continue;
        }
        component[start] = label;
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            for from in &reverse[node] {
                if component[*from] == usize::MAX {
                    component[*from] = label;
                    stack.push(*from);
                }
            }
        }
        label += 1;
    }
    component
}

/// Recusa a política se algum predicado depende de si mesmo por uma aresta negativa.
pub(crate) fn check(rules: &[(Class, Shape)]) -> Result<(), PolicyError> {
    let mut graph = Graph {
        index: BTreeMap::new(),
        names: Vec::new(),
        edges: Vec::new(),
    };
    let (mut has_allow, mut has_deny) = (false, false);
    for (class, shape) in rules {
        let from = match class {
            Class::Allow => {
                has_allow = true;
                Node::AllowFires
            }
            Class::Deny => {
                has_deny = true;
                Node::DenyFires
            }
            // `require` não chega aqui (não é regra de Horn).
            Class::Derive | Class::Require => Node::Pred(shape.head.clone()),
        };
        graph.id(from.clone());
        for name in &shape.positive {
            graph.edge(from.clone(), Node::Pred(name.clone()), false);
        }
        for name in &shape.negative {
            graph.edge(from.clone(), Node::Pred(name.clone()), true);
        }
    }
    if has_allow {
        graph.edge(Node::Pred(ALLOWED.to_owned()), Node::AllowFires, false);
    }
    if has_deny {
        graph.edge(Node::Pred(ALLOWED.to_owned()), Node::DenyFires, true);
    }
    let component = components(graph.names.len(), &graph.edges);
    match graph
        .edges
        .iter()
        .find(|(from, to, negative)| *negative && component[*from] == component[*to])
    {
        Some((from, ..)) => Err(PolicyError::NegativeCycle(graph.names[*from].label())),
        None => Ok(()),
    }
}
