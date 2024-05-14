use fxhash::{FxHashMap, FxHashSet};
use itertools::iproduct;

use super::{graph::AEdge, ANodeInd, AStarGraph, AStarValue, CX};

impl<V: AStarValue> AStarGraph<V> {
    /// Add all children of `ind` to the graph.
    ///
    /// There are two types of possible "moves": either a circuit gate
    /// (only CX supported), or a "merge". A merge is a identity operation that
    /// combines two circuits into one, provided their symmetric differences only
    /// affect disjoint qubits.
    ///
    /// This implements the following logic:
    ///  - If `ind` is the target of a CX edge, we can:
    ///      - Add a CX on the same qubits -- it must be in the reverse direction,
    ///        otherwise it cancels out
    ///      - Add merges -- we can add merges between `ind` and any other
    ///        compatible node.
    ///  - If `ind` is the target merge edge, we distinguish between merge
    ///    preceded by two CX edges and merge preceded by at least one other
    ///    merge edge. In the latter case, we cannot add CX edges anymore (the
    ///    only time we allow adding more than one merge in a row is when we have
    ///    already added all necessary CX).
    ///      - If preceded by two CX edges, we can add a CX that spans both sets
    ///        of qubits
    ///      - In every case, we can add merges between `ind` and any other
    ///        compatible node.
    pub(super) fn expand_children(&mut self, ind: ANodeInd) {
        // Find out if and where we can add CXs, and add them
        match self.prev_edge(ind) {
            Some(AEdge::Op {
                op: CX { ctrl, tgt },
                ..
            }) => {
                // We can either add a CX on the same two qubits, or add merges
                let rev_cx = CX {
                    ctrl: *tgt,
                    tgt: *ctrl,
                };
                if self.allowed_moves.contains(&rev_cx) {
                    self.add_cx(ind, rev_cx);
                }
            }
            Some(merge_edge @ AEdge::Merge { .. }) => {
                if let Some((qbs1, qbs2)) = self.get_cx_qbs(merge_edge) {
                    // We can add CX between qbs1 and qbs2
                    for (ctrl, tgt) in iproduct!(qbs1, qbs2) {
                        let cx = CX { ctrl, tgt };
                        if self.allowed_moves.contains(&cx) {
                            self.add_cx(ind, cx);
                        }
                        let rev_cx = CX {
                            ctrl: tgt,
                            tgt: ctrl,
                        };
                        if self.allowed_moves.contains(&rev_cx) {
                            self.add_cx(ind, rev_cx);
                        }
                    }
                }
            }
            None => {}
        }
        // We can add merges if `ind` is not the root
        if self.prev_edge(ind).is_some() {
            let mergeable_nodes = self.find_mergeable_nodes(ind);
            for (node, qbs) in mergeable_nodes {
                self.add_merge(ind, node, &qbs);
            }
        }
    }

    fn find_mergeable_nodes(&self, ind: ANodeInd) -> FxHashMap<ANodeInd, FxHashSet<u8>> {
        // Map each node to the qubits we cannot add CXs to
        let mut disallowed_qbs = FxHashMap::default();

        // First: a backward DFS from `ind` to `root` to populate `allowed_qbs`
        self.backward_dfs(ind, &mut disallowed_qbs);

        // Make sure we have arrived at the root
        assert!(disallowed_qbs.contains_key(&self.root_ind()));

        // Second: a forward pass in topsort order to find all possible merges
        // returns a map from nodes we can merge with `ind` along with the qubits
        // that would be used
        self.propagate_forward(&mut disallowed_qbs)
    }

    fn backward_dfs(&self, ind: ANodeInd, disallowed_qbs: &mut FxHashMap<ANodeInd, FxHashSet<u8>>) {
        let mut dfs_queue = vec![ind];
        while let Some(curr) = dfs_queue.pop() {
            if !disallowed_qbs.contains_key(&curr) {
                disallowed_qbs.insert(curr, self.disallowed_qubits(curr, ind));
                dfs_queue.extend(self.prev_edge(curr).map(|e| e.srcs()).unwrap_or_default());
            }
        }
    }

    fn propagate_forward(
        &self,
        disallowed_qbs: &mut FxHashMap<ANodeInd, FxHashSet<u8>>,
    ) -> FxHashMap<ANodeInd, FxHashSet<u8>> {
        let mut queue = vec![self.root_ind()];
        let nodes_in_past = FxHashSet::from_iter(disallowed_qbs.keys().copied());
        let mut mergeable_nodes = FxHashMap::default();
        while let Some(curr) = queue.pop() {
            //  invariant: the source of `edge` has a value in `disallowed_qbs`
            for edge in self.next_edges(curr) {
                match edge {
                    &AEdge::Op {
                        op: CX { ctrl, tgt },
                        src,
                        dst,
                    } => {
                        if nodes_in_past.contains(&dst) {
                            // We have already processed this node in the backward pass, skip
                            queue.push(dst);
                            println!(
                                "dst {:?} was already processed in backward pass -> skip",
                                dst
                            );
                            continue;
                        }
                        // check that we are not using a disallowed qubit
                        let disallowed_src = disallowed_qbs.get(&src).unwrap();
                        if disallowed_src.contains(&ctrl) || disallowed_src.contains(&tgt) {
                            println!("finding disallowed CX({}, {}) -> skip", ctrl, tgt);
                            continue;
                        }
                        // We can directly proceed as `dst` has only `src` as a parent
                        if !disallowed_qbs.contains_key(&dst) {
                            println!("finding new of interest CX({}, {}) -> add", ctrl, tgt);
                            disallowed_qbs.insert(dst, disallowed_src.clone());
                            queue.push(dst);
                            let mut used_qubits: FxHashSet<u8> =
                                mergeable_nodes.get(&src).cloned().unwrap_or_default();
                            used_qubits.extend([ctrl, tgt]);
                            mergeable_nodes.insert(dst, used_qubits);
                        }
                    }
                    &AEdge::Merge { src1, src2, dst } => {
                        if nodes_in_past.contains(&dst) {
                            // We have already processed this node in the backward pass, skip
                            queue.push(dst);
                            continue;
                        }
                        // We can proceed when we have reached both sources
                        let Some(disallowed_src1) = disallowed_qbs.get(&src1) else {
                            continue;
                        };
                        let Some(disallowed_src2) = disallowed_qbs.get(&src2) else {
                            continue;
                        };
                        // Proceed, insert the intersection of the disallowed qubits
                        if !disallowed_qbs.contains_key(&dst) {
                            let intersection = disallowed_src1.intersection(&disallowed_src2);
                            disallowed_qbs.insert(dst, intersection.copied().collect());
                            queue.push(dst);
                            let mut used_qubits: FxHashSet<u8> =
                                mergeable_nodes.get(&src1).cloned().unwrap_or_default();
                            if let Some(qbs) = mergeable_nodes.get(&src2) {
                                used_qubits.extend(qbs);
                            }
                            mergeable_nodes.insert(dst, used_qubits);
                        }
                    }
                }
            }
        }
        mergeable_nodes
    }

    /// Given a Merge edge, returns the qubits of both previous CX edges.
    ///
    /// If at least one of the two previous edges were not CX edges, return None.
    fn get_cx_qbs(&self, edge: &AEdge) -> Option<([u8; 2], [u8; 2])> {
        let &AEdge::Merge { src1, src2, .. } = edge else {
            panic!("Not a merge edge");
        };
        let Some(AEdge::Op { op: op1, .. }) = self.prev_edge(src1) else {
            return None;
        };
        let Some(AEdge::Op { op: op2, .. }) = self.prev_edge(src2) else {
            return None;
        };
        Some(([op1.ctrl, op1.tgt], [op2.ctrl, op2.tgt]))
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;

    /// Tests the `find_mergeable_nodes` function on the following search graph
    ///
    /// ```mermaid
    /// flowchart LR
    ///  0 -->|"CX(0, 1)"| 1
    ///  0 -->|"CX(4, 3)"| 2
    ///  0 -->|"CX(2, 3)"| 3
    ///  1 -->|"CX(1, 2)"| 4
    ///  1 -->|"CX(3, 4)"| 5
    ///  ```
    #[test]
    fn test_find_mergeable_nodes() {
        let mut graph = AStarGraph::new([false; 5], []);
        let children = [
            CX { ctrl: 0, tgt: 1 },
            CX { ctrl: 4, tgt: 3 },
            CX { ctrl: 2, tgt: 3 },
        ]
        .into_iter()
        .map(|cx| graph.add_cx(graph.root_ind(), cx))
        .collect_vec();

        let mergeable_nodes = graph.find_mergeable_nodes(children[0]);
        assert_eq!(
            mergeable_nodes.keys().copied().collect::<FxHashSet<_>>(),
            FxHashSet::from_iter([children[1], children[2]])
        );

        let grandchild = graph.add_cx(children[0], CX { ctrl: 1, tgt: 2 });
        let grandchild2 = graph.add_cx(children[0], CX { ctrl: 3, tgt: 4 });

        let mergeable_nodes = graph.find_mergeable_nodes(grandchild);
        assert_eq!(
            mergeable_nodes.keys().copied().collect::<FxHashSet<_>>(),
            FxHashSet::from_iter([grandchild2, children[1]])
        );
    }
}
