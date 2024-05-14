mod expand_children;
mod graph;

use fxhash::FxHashSet;
use priority_queue::PriorityQueue;
use std::{cmp::Reverse, hash::Hash};

use graph::{ANodeInd, AStarGraph};

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub struct CX {
    ctrl: u8,
    tgt: u8,
}

pub trait AStarValue: Hash + Eq + PartialEq
where
    Self: Sized,
{
    /// An approximate cost from `self` to `other`.
    ///
    /// If this is a lower bound for the true cost from `self` to `other`, then
    /// A* will find the shortest path.
    fn dist(&self, other: &Self) -> usize;

    /// Apply a CX gate to the current value
    fn cx(&self, ctrl: u8, tgt: u8) -> Self;

    /// Merge two values
    fn merge(&self, other: &Self, used_qubits: &FxHashSet<u8>) -> Self;
}

type PQ = PriorityQueue<usize, Reverse<usize>>;

pub fn a_star<V: AStarValue>(
    start: V,
    goal: &V,
    allowed_moves: impl IntoIterator<Item = CX>,
) -> Vec<CX> {
    let mut graph = AStarGraph::new(start, allowed_moves);

    let mut pq = PQ::new();
    pq.push(graph.root_ind(), Reverse(graph.root().dist(goal)));

    // The solution of the current best solution
    let mut min_solution: Option<Vec<_>> = None;

    while let Some((ind, Reverse(cost))) = pq.pop() {
        if let Some(min_solution) = min_solution.as_ref() {
            if cost > min_solution.len() {
                // No further solution will be cheaper, so we are done
                break;
            }
        }
        graph.expand_children(ind);
        for new_child in graph.children(ind) {
            if graph.value(new_child) == Some(goal) {
                let new_solution = graph.path(new_child);
                match min_solution {
                    Some(sol) if new_solution.len() < sol.len() => {
                        min_solution = Some(new_solution);
                    }
                    None => {
                        min_solution = Some(new_solution);
                    }
                    _ => {}
                }
            }
            let mut cost_estimate = graph.cost(new_child);
            cost_estimate += graph.value(new_child).unwrap().dist(goal);
            pq.push(new_child, Reverse(cost_estimate));
        }
    }
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    impl AStarValue for [bool; 5] {
        fn dist(&self, other: &[bool; 5]) -> usize {
            self.iter()
                .zip(other)
                .map(|(&a, &b)| (a != b) as usize)
                .sum()
        }

        fn cx(&self, ctrl: u8, tgt: u8) -> Self {
            let mut new = self.clone();
            new[ctrl as usize] = true;
            new[tgt as usize] = true;
            new
        }

        fn merge(&self, other: &Self, used_qubits: &FxHashSet<u8>) -> Self {
            let mut new = self.clone();
            for qb in used_qubits {
                new[*qb as usize] = other[*qb as usize];
            }
            new
        }
    }
}
