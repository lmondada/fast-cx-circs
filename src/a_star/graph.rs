use std::cmp::max;

use bimap::BiHashMap;
use fxhash::{FxBuildHasher, FxHashSet, FxHasher};

use super::{AStarValue, CX};

pub(super) type ANodeInd = usize;

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

pub(super) struct AStarStats {
    cx_count_per_qb: Vec<u16>,
}

type AStarValueMap<V> = bimap::BiHashMap<ANodeInd, V, FxBuildHasher, FxBuildHasher>;

pub(super) struct AStarGraph<'m, V> {
    nodes: Vec<ANode>,
    values: AStarValueMap<V>,
    pub(super) allowed_moves: &'m FxHashSet<CX>,
}

impl<'m, V: AStarValue> AStarGraph<'m, V> {
    pub(super) fn new(start: V, allowed_moves: &'m FxHashSet<CX>) -> Self {
        let values = AStarValueMap::from_iter([(0, start)]);
        Self {
            nodes: vec![ANode::new_root()],
            values,
            allowed_moves,
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
        while let Some(node) = curr_nodes.pop() {
            match self.nodes[node].prev.as_ref() {
                Some(AEdge::Op { op, src, dst }) => {
                    path.push(*op);
                    curr_nodes.push(*src);
                }
                Some(AEdge::Merge { src1, src2, dst }) => {
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

    pub(super) fn add_cx(&mut self, node: ANodeInd, CX { ctrl, tgt }: CX) {
        // Construct new edge
        let edge = AEdge::Op {
            op: CX { ctrl, tgt },
            src: node,
            dst: self.nodes.len(),
        };

        // Update cost
        let cost = self.cost(node) + 1;

        // Update stats, resizing if too small
        let mut cx_count_per_qb = self.nodes[node].stats.cx_count_per_qb.clone();
        let max_qb = max(ctrl, tgt) as usize;
        if cx_count_per_qb.len() <= max_qb {
            cx_count_per_qb.resize(max_qb + 1, 0);
        }
        cx_count_per_qb[ctrl as usize] += 1;
        cx_count_per_qb[tgt as usize] += 1;

        // Update node value
        let node_value = self.values.get_by_left(&node).unwrap();
        let new_value = node_value.cx(ctrl, tgt);

        self.values.insert(self.nodes.len(), new_value);
        self.nodes
            .push(ANode::new_child(edge, cost, cx_count_per_qb));
    }

    pub(super) fn add_merge(&mut self, src1: ANodeInd, src2: ANodeInd) {
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
        // let node_value = self.values.get_by_left(&node).unwrap();
        // let new_value = node_value.cx(ctrl, tgt);

        // self.values.insert(self.nodes.len(), new_value);
        // self.nodes
        //     .push(ANode::new_child(edge, cost, cx_count_per_qb));
    }

    /// Find the qubits that have CX ops that are
    ///   i) in the past of `top` but
    ///  ii) not in the past of `ind`
    pub(super) fn disallowed_qubits(&self, top: ANodeInd, ind: ANodeInd) -> FxHashSet<u8> {
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
            &AEdge::Op { op, src, dst } => dst,
            &AEdge::Merge { src1, src2, dst } => dst,
        }
    }

    pub(super) fn srcs(&self) -> Vec<ANodeInd> {
        match self {
            &AEdge::Op { src, .. } => vec![src],
            &AEdge::Merge { src1, src2, .. } => vec![src1, src2],
        }
    }
}
