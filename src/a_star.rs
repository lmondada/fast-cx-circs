mod expand_children;
mod graph;

use fxhash::FxHashSet;
use priority_queue::PriorityQueue;
use std::{cmp::Reverse, hash::Hash};

use graph::{ANodeInd, AStarGraph};

use crate::CX;

pub trait AStarValue: Hash + Eq + PartialEq + Clone
where
    Self: Sized,
{
    /// An approximate cost from `self` to `other`.
    ///
    /// If this is a lower bound for the true cost from `self` to `other`, then
    /// A* will find the shortest path.
    fn dist(&self, other: &Self) -> usize;

    /// Whether the target was reached on the given qubit
    fn is_complete(&self, qb: u8, target: &Self) -> bool;

    /// Apply a CX gate to the current value
    fn cx(&self, ctrl: u8, tgt: u8) -> Self;

    /// Merge two values
    fn merge(&self, other: &Self, used_qubits: &FxHashSet<u8>) -> Self;
}

type PQ = PriorityQueue<usize, PQCost>;

/// The cost function for the priority queue
/// We want
///  i) low estimated total cost
/// ii) break ties using highest cost already reached
#[derive(Hash, Eq, PartialEq, Clone, PartialOrd, Ord, Debug)]
struct PQCost(Reverse<usize>, usize);
impl PQCost {
    fn new(cost: usize, gates: usize) -> Self {
        PQCost(Reverse(cost), gates)
    }

    fn cost(&self) -> usize {
        self.0 .0
    }
}

pub fn a_star<V: AStarValue>(
    start: V,
    target: &V,
    allowed_moves: impl IntoIterator<Item = CX>,
    max_depth: Option<usize>,
) -> Option<Vec<CX>> {
    let mut graph = AStarGraph::new(start, allowed_moves);

    let mut pq = PQ::new();
    pq.push(graph.root_ind(), PQCost::new(graph.root().dist(target), 0));

    // The solution of the current best solution
    let mut min_solution: Option<Vec<_>> = None;

    // For progress reporting purposes
    let mut max_cost: Option<usize> = None;

    loop {
        let (ind, prio) = pq.pop().expect("Ran out of circuits to explore?!");
        if max_cost.is_none() || graph.cost(ind) > max_cost.unwrap() {
            max_cost = Some(graph.cost(ind));
            println!("Max cost explored: {}", max_cost.unwrap());
            if max_depth.is_some() && max_cost > max_depth {
                println!("Max depth reached, aborting");
                break;
            }
        }
        if let Some(min_solution) = min_solution.as_ref() {
            if prio.cost() > min_solution.len() {
                // No further solution will be cheaper, so we are done
                println!("Found solution is optimal. Terminating");
                break;
            }
        }
        let value = graph.value(ind).unwrap().clone();
        graph.expand_children(ind, |qb| value.is_complete(qb, target));
        for new_child in graph.children(ind) {
            if graph.value(new_child) == Some(target) {
                let new_solution = graph.path(new_child);
                match min_solution {
                    Some(sol) if new_solution.len() < sol.len() => {
                        min_solution = Some(new_solution);
                        println!("New best solution: {:?}", min_solution.as_ref().unwrap());
                    }
                    None => {
                        min_solution = Some(new_solution);
                        println!("New best solution: {:?}", min_solution.as_ref().unwrap());
                    }
                    _ => {}
                }
            }
            let mut cost_estimate = graph.cost(new_child);
            cost_estimate += graph.value(new_child).unwrap().dist(target);
            pq.push(new_child, PQCost::new(cost_estimate, graph.cost(new_child)));
        }
    }
    min_solution
}

#[cfg(test)]
mod tests {
    use crate::cx_circuit::{CXCircuit, CXCircuit16};

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

        fn is_complete(&self, qb: u8, target: &Self) -> bool {
            self[qb as usize] == target[qb as usize]
        }
    }

    #[test]
    fn test_a_star_simple() {
        let mut circuit = CXCircuit16::new();
        circuit.add_cx(0, 9);
        circuit.add_cx(0, 10);
        let moves = vec![CX { ctrl: 0, tgt: 9 }, CX { ctrl: 0, tgt: 10 }];
        let result = a_star(CXCircuit16::new(), &circuit, moves, Some(2)).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_a_star_with_merge() {
        let mut circuit = CXCircuit16::new();
        circuit.add_cx(0, 1);
        circuit.add_cx(2, 3);
        circuit.add_cx(1, 4);
        let moves = vec![
            CX { ctrl: 0, tgt: 1 },
            CX { ctrl: 2, tgt: 3 },
            CX { ctrl: 1, tgt: 4 },
        ];
        let result = a_star(CXCircuit16::new(), &circuit, moves, Some(3)).unwrap();
        assert_eq!(result.len(), 3);
    }
}
