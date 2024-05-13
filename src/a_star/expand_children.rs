use fxhash::{FxHashMap, FxHashSet};
use itertools::iproduct;

use super::{graph::AEdge, ANodeInd, AStarGraph, AStarValue, CX};

impl<'m, V: AStarValue> AStarGraph<'m, V> {
    pub(super) fn expand_children(&mut self, ind: ANodeInd) {
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
                self.add_merges(ind, false);
            }
            Some(merge_edge @ AEdge::Merge { .. }) => {
                if let Some((qbs1, qbs2)) = self.get_cx_qbs(merge_edge) {
                    // We can add CX between qbs1 and qbs2...
                    for (ctrl, tgt) in iproduct!(qbs1, qbs2) {
                        let cx = CX { ctrl, tgt };
                        if self.allowed_moves.contains(&cx) {
                            self.add_cx(ind, cx);
                        }
                    }
                    // or merges but then we switch to only merging
                    self.add_merges(ind, true);
                } else {
                    // We are done searching, we can only add merges
                    self.add_merges(ind, true);
                }
            }
            None => {}
        }
    }

    fn add_merges(&mut self, ind: ANodeInd, only_merging: bool) {
        // Map each node to the qubits we cannot add CXs to
        let mut disallowed_qbs = FxHashMap::default();

        // First: a backward DFS from `ind` to `root` to populate `allowed_qbs`
        self.backward_dfs(ind, &mut disallowed_qbs);

        // Make sure we have arrived at the root
        assert!(disallowed_qbs.contains_key(&self.root_ind()));

        // Second: a forward pass in topsort order to find all possible merges
        let mergeable_nodes = self.propagate_forward(&mut disallowed_qbs);

        for node in mergeable_nodes {
            self.add_merge(ind, node);
        }
    }

    fn backward_dfs(
        &mut self,
        ind: ANodeInd,
        disallowed_qbs: &mut FxHashMap<ANodeInd, FxHashSet<u8>>,
    ) {
        let mut dfs_queue = vec![ind];
        while let Some(curr) = dfs_queue.pop() {
            if !disallowed_qbs.contains_key(&curr) {
                disallowed_qbs.insert(curr, self.disallowed_qubits(curr, ind));
                dfs_queue.extend(self.prev_edge(curr).map(|e| e.srcs()).unwrap_or_default());
            }
        }
    }

    fn propagate_forward(
        &mut self,
        disallowed_qbs: &mut FxHashMap<ANodeInd, FxHashSet<u8>>,
    ) -> Vec<ANodeInd> {
        let mut queue = vec![self.root_ind()];
        let nodes_in_past = FxHashSet::from_iter(disallowed_qbs.keys().copied());
        let mut mergeable_nodes = vec![];
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
                            continue;
                        }
                        // check that we are not using a disallowed qubit
                        let disallowed_src = disallowed_qbs.get(&src).unwrap();
                        if disallowed_src.contains(&ctrl) || disallowed_src.contains(&tgt) {
                            continue;
                        }
                        // We can directly proceed as `dst` has only `src` as a parent
                        if !disallowed_qbs.contains_key(&dst) {
                            disallowed_qbs.insert(dst, disallowed_src.clone());
                            queue.push(dst);
                            mergeable_nodes.push(dst);
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
                            mergeable_nodes.push(dst);
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
