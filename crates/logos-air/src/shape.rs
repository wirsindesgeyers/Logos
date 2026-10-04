//! Validação de árvore prefixa com stack explícita, sem recursão lógica.

use crate::{AirError, Limits};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Stats {
    pub depth: usize,
    pub nodes: usize,
}

impl Stats {
    pub fn combine(self, other: Self) -> Result<Self, AirError> {
        Ok(Self {
            depth: self.depth.max(other.depth),
            nodes: self
                .nodes
                .checked_add(other.nodes)
                .ok_or(AirError::InvalidStructure)?,
        })
    }
}

pub(crate) struct Shape {
    limits: Limits,
    // Cada frame é um ancestral cujo subtree ainda está sendo consumido.
    // Raiz não precisa de frame/alocação até que tenha filhos.
    frames: Vec<usize>,
    stats: Stats,
}

impl Shape {
    pub fn new(limits: Limits) -> Self {
        Self {
            limits,
            frames: Vec::new(),
            stats: Stats::default(),
        }
    }

    fn close_frames(&mut self) {
        while self.frames.last() == Some(&0) {
            self.frames.pop();
        }
    }

    pub fn next_depth(&mut self) -> Result<usize, AirError> {
        self.close_frames();
        if self.stats.nodes != 0 && self.frames.is_empty() {
            return Err(AirError::InvalidStructure);
        }
        let depth = self
            .frames
            .len()
            .checked_add(1)
            .ok_or(AirError::InvalidStructure)?;
        if depth > self.limits.max_depth {
            return Err(AirError::DepthExceeded {
                limit: self.limits.max_depth,
                actual: depth,
            });
        }
        if self.stats.nodes >= self.limits.max_nodes {
            return Err(AirError::NodesExceeded {
                limit: self.limits.max_nodes,
                actual: self
                    .stats
                    .nodes
                    .checked_add(1)
                    .ok_or(AirError::InvalidStructure)?,
            });
        }
        Ok(depth)
    }

    pub fn child_limits(&mut self) -> Result<Limits, AirError> {
        let depth = self.next_depth()?;
        Ok(Limits {
            max_depth: self.limits.max_depth - depth,
            max_nodes: self.limits.max_nodes - self.stats.nodes - 1,
        })
    }

    pub fn push(&mut self, children: usize, embedded: Stats) -> Result<(), AirError> {
        let depth = self.next_depth()?;
        let actual_depth = depth
            .checked_add(embedded.depth)
            .ok_or(AirError::InvalidStructure)?;
        let actual_nodes = self
            .stats
            .nodes
            .checked_add(1)
            .and_then(|nodes| nodes.checked_add(embedded.nodes))
            .ok_or(AirError::InvalidStructure)?;
        if actual_depth > self.limits.max_depth {
            return Err(AirError::DepthExceeded {
                limit: self.limits.max_depth,
                actual: actual_depth,
            });
        }
        if actual_nodes > self.limits.max_nodes {
            return Err(AirError::NodesExceeded {
                limit: self.limits.max_nodes,
                actual: actual_nodes,
            });
        }
        if let Some(remaining) = self.frames.last_mut() {
            *remaining -= 1; // next_depth retirou frames esgotados.
        }
        if children != 0 {
            self.frames.push(children);
        }
        self.stats = Stats {
            depth: self.stats.depth.max(actual_depth),
            nodes: actual_nodes,
        };
        Ok(())
    }

    pub fn finish(mut self) -> Result<Stats, AirError> {
        self.close_frames();
        if self.stats.nodes == 0 || !self.frames.is_empty() {
            Err(AirError::InvalidStructure)
        } else {
            Ok(self.stats)
        }
    }
}
