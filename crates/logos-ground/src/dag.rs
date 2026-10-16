//! Construção limitada e internamento determinístico do DAG.
use crate::{GroundError, GroundLimits, PropNode};
use logos_core::Exhausted;
use std::collections::BTreeMap;

pub(crate) fn size() -> GroundError {
    GroundError::Unknown(Exhausted::Size)
}
pub(crate) fn push<T>(v: &mut Vec<T>, item: T) -> Result<(), GroundError> {
    v.try_reserve(1)
        .map_err(|_| GroundError::Unknown(Exhausted::Memory))?;
    v.push(item);
    Ok(())
}
pub(crate) fn space<T>(n: usize, max: usize) -> Result<Vec<T>, GroundError> {
    if n > max {
        return Err(size());
    }
    let mut v = Vec::new();
    v.try_reserve_exact(n)
        .map_err(|_| GroundError::Unknown(Exhausted::Memory))?;
    Ok(v)
}
pub(crate) struct Dag {
    pub nodes: Vec<PropNode>,
    cache: BTreeMap<PropNode, usize>,
    pub limits: GroundLimits,
    steps: usize,
    edges: usize,
}
impl Dag {
    pub fn new(limits: GroundLimits) -> Result<Self, GroundError> {
        let mut s = Self {
            nodes: Vec::new(),
            cache: BTreeMap::new(),
            limits,
            steps: 0,
            edges: 0,
        };
        s.node(PropNode::False)?;
        s.node(PropNode::True)?;
        Ok(s)
    }
    pub fn charge(&mut self, n: usize) -> Result<(), GroundError> {
        self.steps = self.steps.checked_add(n).ok_or_else(size)?;
        if self.steps > self.limits.max_steps {
            return Err(size());
        }
        Ok(())
    }
    pub fn node(&mut self, n: PropNode) -> Result<usize, GroundError> {
        self.charge(1)?;
        if let Some(id) = self.cache.get(&n) {
            return Ok(*id);
        }
        let edges = match &n {
            PropNode::Not(_) => 1,
            PropNode::And(xs) | PropNode::Or(xs) => xs.len(),
            _ => 0,
        };
        self.edges = self.edges.checked_add(edges).ok_or_else(size)?;
        if self.nodes.len() >= self.limits.max_nodes || self.edges > self.limits.max_edges {
            return Err(size());
        }
        let id = self.nodes.len();
        self.cache.insert(n.clone(), id);
        push(&mut self.nodes, n)?;
        Ok(id)
    }
    pub fn and(&mut self, xs: Vec<usize>) -> Result<usize, GroundError> {
        match xs.as_slice() {
            [] => Ok(1),
            [x] => Ok(*x),
            _ => self.node(PropNode::And(xs)),
        }
    }
    pub fn or(&mut self, xs: Vec<usize>) -> Result<usize, GroundError> {
        match xs.as_slice() {
            [] => Ok(0),
            [x] => Ok(*x),
            _ => self.node(PropNode::Or(xs)),
        }
    }
    pub fn implies(&mut self, a: usize, b: usize) -> Result<usize, GroundError> {
        let n = self.node(PropNode::Not(a))?;
        self.or(vec![n, b])
    }
    pub fn unique(&mut self, vs: &[usize]) -> Result<usize, GroundError> {
        let pairs = vs
            .len()
            .checked_mul(vs.len().saturating_sub(1))
            .ok_or_else(size)?
            / 2;
        self.charge(pairs)?;
        let mut clauses = space(
            pairs.checked_add(1).ok_or_else(size)?,
            self.limits.max_cells,
        )?;
        let some = self.or(vs.to_vec())?;
        push(&mut clauses, some)?;
        for i in 0..vs.len() {
            for j in i + 1..vs.len() {
                let both = self.and(vec![vs[i], vs[j]])?;
                let not = self.node(PropNode::Not(both))?;
                push(&mut clauses, not)?;
            }
        }
        self.and(clauses)
    }
}
