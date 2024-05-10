use fxhash::{FxHashMap, FxHashSet};

use crate::{cx_circuit::CXCircuit, CircMoves, Moves};

// const PRIME: usize = 10000007;

struct BFS<'m, T> {
    /// Allowed moves (i.e. CX interactions)
    moves: &'m Moves<T>,
    /// Map from CX count to circuits
    /// At CX count of 0: just the identity circuit
    cx_count_circs: Vec<CircMoves<T>>,
}

impl<'m, T: CXCircuit> BFS<'m, T> {
    fn new(start_circ: T, moves: &'m Moves<T>) -> Self {
        let cx_count_circs = vec![CircMoves::from_iter([(start_circ, usize::MAX)])];
        Self {
            moves,
            cx_count_circs,
        }
    }

    /// Apply the valid moves to every circuit reached in the previous step.
    ///
    /// Returns the newly discovered circuits
    fn step(&mut self) -> FxHashSet<T> {
        let depth = self.cx_count_circs.len();
        let frontiers = {
            let mut frontiers = Vec::from_iter([(&self.cx_count_circs[depth - 1])]);
            if depth > 1 {
                frontiers.push(&self.cx_count_circs[depth - 2]);
            }
            frontiers
        };
        let new_moves = collect_moves(&frontiers[0], self.moves, |circ| {
            !frontiers.iter().any(|f| f.contains_key(circ))
        });
        println!("With {} CX gates: {} circuits", depth, new_moves.len());
        let new_circs = new_moves.keys().copied().collect();
        self.cx_count_circs.push(new_moves);
        new_circs
    }

    fn backtrack(&self, circ: &T) -> Vec<usize> {
        let mut moves = Vec::new();
        let mut curr = *circ;
        for curr_depth in (1..self.cx_count_circs.len()).rev() {
            dbg!(curr_depth);
            let Some(move_id) = self.cx_count_circs[curr_depth].get(&curr).copied() else {
                // It's possible that the circuit is not at the highest depth, in which case
                // we hope to find it in a future iteration
                continue;
            };
            moves.push(move_id);
            let mv = self
                .moves
                .get(move_id)
                .expect("found an unknown move whilst backtracking");
            curr = curr.mult_transpose(&mv);
        }
        if !self.cx_count_circs[0].contains_key(&curr) {
            panic!(
                "invalid backtracking: we have reached depth 0 without successfully backtracking"
            );
        }
        moves
    }

    fn depth(&self) -> usize {
        self.cx_count_circs.len() - 1
    }
}

/// Breadth-first search, starting from identity circuit.
#[cfg(test)]
pub fn bfs<T: CXCircuit>(target_circ: T, moves: &Moves<T>, max_steps: usize) -> Option<Vec<usize>> {
    let mut bfs = BFS::new(T::new(), moves);
    for _ in 1..=max_steps {
        let frontier = bfs.step();
        if frontier.contains(&target_circ) {
            let moves = Vec::from_iter(bfs.backtrack(&target_circ));
            return Some(moves);
        }
    }
    None
}

/// Breadth-first search, starting from both ends and meet in the middle.
///
/// Optionally, extrapolate to circuits with up to 3 * `max_steps` gates. This
/// has no additional memory costs.
pub fn mitm_bfs<T: CXCircuit>(
    target_circ: T,
    moves: &Moves<T>,
    max_steps: usize,
    extrapolate: bool,
) -> Option<Vec<usize>> {
    if max_steps < 1 {
        return None;
    }

    // Start one BFS at the identity circuit
    let mut forward = BFS::new(T::new(), moves);
    // Start one BFS at the target circuit
    let mut backward = BFS::new(target_circ, moves);

    let mut forward_frontier = None;
    let mut backward_frontier = None;

    for n_cx in 1..=max_steps {
        println!("forward:");
        forward_frontier = Some(forward.step());
        if let Some(circ) = intersect(forward_frontier.as_ref(), backward_frontier.as_ref()) {
            println!("Found solution using {} CXs", 2 * n_cx - 1,);
            return Some(backtrack_mitm(&forward, &backward, circ));
        }
        println!("backward:");
        backward_frontier = Some(backward.step());
        if let Some(circ) = intersect(forward_frontier.as_ref(), backward_frontier.as_ref()) {
            println!("Found solution using {} CXs", 2 * n_cx);
            return Some(backtrack_mitm(&forward, &backward, circ));
        }
    }

    if extrapolate {
        // Now we extrapolate
        // TODO: use hash explicitly?
        let forward_frontier = forward_frontier.expect("max_steps > 0");
        let backward_frontier = backward_frontier.expect("max_steps > 0");
        for extra_depth in 1..=forward.depth() {
            let moves: Vec<_> = forward.cx_count_circs[extra_depth]
                .keys()
                // Always transpose moves!
                .map(|mv| mv.transpose())
                .collect();
            println!(
                "Extrapolating to {} CX gates...",
                2 * max_steps + extra_depth
            );
            if let Some((mv_id, circ_backward)) = apply_moves(&forward_frontier, &moves)
                .find(|(_, circ)| backward_frontier.contains(&circ))
            {
                println!("Found solution!");
                let extra_moves = moves[mv_id];
                // The first third of the circuit is the last third without the
                // middle moves
                let circ_forward = circ_backward.mult_transpose(&extra_moves);
                // Transpose back!
                let circ_mid = extra_moves.transpose();
                return Some(backtrack_mitm_extra(
                    &forward,
                    &backward,
                    circ_forward,
                    circ_mid,
                    circ_backward,
                ));
            };
        }
    }

    println!("No solution found at maximal depth, aborting");
    None
}

fn apply_moves<'a, T: CXCircuit + 'a>(
    circs: impl IntoIterator<Item = &'a T> + 'a,
    moves: impl IntoIterator<Item = &'a T> + Clone + 'a,
) -> impl Iterator<Item = (usize, T)> + 'a {
    circs.into_iter().flat_map(move |circ| {
        moves
            .clone()
            .into_iter()
            .enumerate()
            .map(|(mv_id, mv)| (mv_id, circ.mult_transpose(mv)))
    })
}

fn collect_moves<T: CXCircuit, V>(
    circs: &FxHashMap<T, V>,
    moves: &Moves<T>,
    mut retain_f: impl FnMut(&T) -> bool,
) -> CircMoves<T> {
    // A rough estimate of the capacity required
    let mut circuits =
        CircMoves::with_capacity_and_hasher(circs.len() * moves.len() / 3, Default::default());

    apply_moves(circs.keys(), moves).for_each(|(i, mv)| {
        if retain_f(&mv) {
            circuits.insert(mv, i);
        }
    });

    circuits.shrink_to_fit();
    circuits
}

fn intersect<T: CXCircuit>(
    frontier1: Option<&FxHashSet<T>>,
    frontier2: Option<&FxHashSet<T>>,
) -> Option<T> {
    let Some(frontier1) = frontier1 else {
        return None;
    };
    let Some(frontier2) = frontier2 else {
        return None;
    };
    frontier1.intersection(frontier2).next().copied()
}

fn backtrack_mitm<T: CXCircuit>(forward: &BFS<T>, backward: &BFS<T>, circ: T) -> Vec<usize> {
    let mut moves = Vec::new();
    moves.extend(backward.backtrack(&circ).into_iter().rev());
    moves.extend(forward.backtrack(&circ));
    moves
}

fn backtrack_mitm_extra<T: CXCircuit>(
    forward: &BFS<T>,
    backward: &BFS<T>,
    circ_forward: T,
    circ_mid: T,
    circ_backward: T,
) -> Vec<usize> {
    let mut moves = Vec::new();
    println!("backtracking backward");
    moves.extend(backward.backtrack(&circ_backward).into_iter().rev());
    println!("backtracking extra moves");
    moves.extend(forward.backtrack(&circ_mid));
    println!("backtracking forward");
    moves.extend(forward.backtrack(&circ_forward));
    moves
}
