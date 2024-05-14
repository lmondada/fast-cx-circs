use std::cmp::max;

use fxhash::{FxBuildHasher, FxHashSet};

use super::AStarValue;
use crate::CX;

pub(super) type ANodeInd = usize;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AEdge {
    Op {
        op: CX,
        src: ANodeInd,
        dst: ANodeInd,
    },
    Merge {
        src1: ANodeInd,
        src2: ANodeInd,
        dst: ANodeInd,
    },
}

/// A node in the A* search graph
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct ANode {
    /// The cost of the path from the root to this node
    cost: usize,
    /// The previous edge in the path from the root to this node
    prev: Option<AEdge>,
    /// The next edges in the path from this node to other nodes
    next: Vec<AEdge>,
    /// The counts of which CX interactions have happened so far
    stats: AStarStats,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct AStarStats {
    cx_count_per_qb: Vec<u16>,
}

type AStarValueMap<V> = bimap::BiHashMap<ANodeInd, V, FxBuildHasher, FxBuildHasher>;

#[derive(Debug)]
pub(super) struct AStarGraph<V> {
    nodes: Vec<ANode>,
    values: AStarValueMap<V>,
    pub(super) allowed_moves: FxHashSet<CX>,
}

impl<V: AStarValue> AStarGraph<V> {
    pub(super) fn new(start: V, allowed_moves: impl IntoIterator<Item = CX>) -> Self {
        let values = AStarValueMap::from_iter([(0, start)]);
        Self {
            nodes: vec![ANode::new_root()],
            values,
            allowed_moves: FxHashSet::from_iter(allowed_moves),
        }
    }

    pub(super) fn root(&self) -> &V {
        self.values.get_by_left(&0).unwrap()
    }

    pub(super) fn root_ind(&self) -> ANodeInd {
        0
    }

    pub(super) fn children(&self, ind: ANodeInd) -> impl Iterator<Item = ANodeInd> + '_ {
        self.nodes[ind].next.iter().map(|edge| edge.dst())
    }

    pub(super) fn prev_edge(&self, ind: ANodeInd) -> Option<&AEdge> {
        self.nodes[ind].prev.as_ref()
    }

    pub(super) fn next_edges(&self, ind: ANodeInd) -> impl Iterator<Item = &AEdge> {
        self.nodes[ind].next.iter()
    }

    pub(super) fn value(&self, ind: ANodeInd) -> Option<&V> {
        self.values.get_by_left(&ind)
    }

    pub(super) fn path(&self, ind: ANodeInd) -> Vec<CX> {
        let mut path = Vec::new();
        let mut curr_nodes = vec![ind];
        let mut seen_nodes = FxHashSet::default();
        while let Some(node) = curr_nodes.pop() {
            if !seen_nodes.insert(node) {
                continue;
            }
            match self.nodes[node].prev.as_ref() {
                Some(AEdge::Op { op, src, .. }) => {
                    path.push(*op);
                    curr_nodes.push(*src);
                }
                Some(AEdge::Merge { src1, src2, .. }) => {
                    curr_nodes.extend([src1, src2]);
                }
                None => {}
            }
        }
        path.reverse();
        path
    }

    pub(super) fn cost(&self, ind: ANodeInd) -> usize {
        self.nodes[ind].cost
    }

    pub(super) fn is_expanded(&self, ind: ANodeInd) -> bool {
        !self.nodes[ind].next.is_empty()
    }

    pub(super) fn add_cx(&mut self, node: ANodeInd, CX { ctrl, tgt }: CX) -> Option<ANodeInd> {
        // Construct new edge
        let edge = AEdge::Op {
            op: CX { ctrl, tgt },
            src: node,
            dst: self.nodes.len(),
        };

        // Update cost
        let cost = self.cost(node) + 1;

        // Update stats, resizing if too small
        let cx_count_per_qb = {
            let mut cx_count_per_qb = self.nodes[node].stats.cx_count_per_qb.clone();
            let max_qb = max(ctrl, tgt) as usize;
            if cx_count_per_qb.len() <= max_qb {
                cx_count_per_qb.resize(max_qb + 1, 0);
            }
            cx_count_per_qb[ctrl as usize] += 1;
            cx_count_per_qb[tgt as usize] += 1;
            cx_count_per_qb
        };

        // Update node value
        let new_value = {
            let node_value = self.values.get_by_left(&node).unwrap();
            node_value.cx(ctrl, tgt)
        };

        if !self.values.contains_right(&new_value) {
            let new_node_ind = self.nodes.len();
            self.values.insert(new_node_ind, new_value);
            self.nodes
                .push(ANode::new_child(edge, cost, cx_count_per_qb));
            self.nodes[node].next.push(edge);
            Some(new_node_ind)
        } else {
            None
        }
    }

    pub(super) fn add_merge(
        &mut self,
        src1: ANodeInd,
        src2: ANodeInd,
        used_qubits: &FxHashSet<u8>,
    ) -> Option<ANodeInd> {
        // Construct new edge
        let edge = AEdge::Merge {
            src1,
            src2,
            dst: self.nodes.len(),
        };

        // Update cost
        let cost = self.cost(src1) + self.cost(src2);

        // Update stats, resizing if too small
        let mut cx_count_per_qb = self.nodes[src1].stats.cx_count_per_qb.clone();
        let cx_count_per_qb2 = &self.nodes[src2].stats.cx_count_per_qb;
        if cx_count_per_qb.len() <= cx_count_per_qb2.len() {
            cx_count_per_qb.resize(cx_count_per_qb2.len(), 0);
        }
        for (qb, count) in cx_count_per_qb2.iter().enumerate() {
            cx_count_per_qb[qb] += count;
        }

        // Update node value
        let src1_value = self.values.get_by_left(&src1).unwrap();
        let src2_value = self.values.get_by_left(&src2).unwrap();
        let new_value = src1_value.merge(src2_value, used_qubits);

        if !self.values.contains_right(&new_value) {
            let new_node_ind = self.nodes.len();
            self.values.insert(new_node_ind, new_value);
            self.nodes
                .push(ANode::new_child(edge, cost, cx_count_per_qb));
            self.nodes[src1].next.push(edge);
            self.nodes[src2].next.push(edge);
            Some(new_node_ind)
        } else {
            None
        }
    }

    /// Find the qubits that have CX ops that are
    ///   i) in the past of `top` but
    ///  ii) not in the past of `ind`
    pub(super) fn disallowed_qubits(&self, ind: ANodeInd, top: ANodeInd) -> FxHashSet<u8> {
        let mut cx_count_per_qb = self.nodes[top].stats.cx_count_per_qb.clone();
        for (qb, count) in self.nodes[ind].stats.cx_count_per_qb.iter().enumerate() {
            cx_count_per_qb[qb] -= count;
        }
        cx_count_per_qb
            .iter()
            .enumerate()
            .filter_map(|(qb, &count)| (count > 0).then_some(qb as u8))
            .collect()
    }
}

impl ANode {
    fn new_root() -> Self {
        Self {
            prev: None,
            next: vec![],
            stats: AStarStats {
                cx_count_per_qb: Vec::new(),
            },
            cost: 0,
        }
    }

    fn new_child(prev: AEdge, cost: usize, cx_count_per_qb: Vec<u16>) -> Self {
        Self {
            prev: Some(prev),
            next: vec![],
            stats: AStarStats { cx_count_per_qb },
            cost,
        }
    }
}

impl AEdge {
    fn dst(&self) -> ANodeInd {
        match self {
            &AEdge::Op { dst, .. } => dst,
            &AEdge::Merge { dst, .. } => dst,
        }
    }

    pub(super) fn srcs(&self) -> Vec<ANodeInd> {
        match self {
            &AEdge::Op { src, .. } => vec![src],
            &AEdge::Merge { src1, src2, .. } => vec![src1, src2],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_cx() {
        let mut graph = AStarGraph::new([false; 5], []);
        let child = graph
            .add_cx(graph.root_ind(), CX { ctrl: 0, tgt: 1 })
            .unwrap();
        assert_eq!(graph.cost(child), 1);
        assert_eq!(graph.nodes[child].stats.cx_count_per_qb, vec![1, 1]);
        let grandchild = graph.add_cx(child, CX { ctrl: 0, tgt: 2 }).unwrap();
        assert_eq!(graph.cost(grandchild), 2);
        assert_eq!(graph.nodes[grandchild].stats.cx_count_per_qb, vec![2, 1, 1]);
    }
    #[test]
    fn test_disallowed_qubits() {
        let mut graph = AStarGraph::new([false; 5], []);
        let child1 = graph
            .add_cx(graph.root_ind(), CX { ctrl: 0, tgt: 1 })
            .unwrap();
        let child2 = graph
            .add_cx(graph.root_ind(), CX { ctrl: 2, tgt: 3 })
            .unwrap();
        let grandchild = graph
            .add_merge(child1, child2, &FxHashSet::from_iter([2, 3]))
            .unwrap();
        assert_eq!(
            graph.disallowed_qubits(child1, grandchild),
            FxHashSet::from_iter([2, 3])
        );
        assert_eq!(
            graph.disallowed_qubits(child2, grandchild),
            FxHashSet::from_iter([0, 1])
        );
    }
}
