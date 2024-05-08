use crate::{
    cx_circuit::{CXCircuit, ManyCircuits},
    hash_table::HashTable,
};

const PRIME: usize = 1000007;

struct BFS<'m, T> {
    /// Allowed moves (i.e. CX interactions)
    moves: &'m ManyCircuits<T>,
    /// All circuits ever seen
    seen_circs: HashTable<T, PRIME>,
    /// Map from CX count to circuits
    /// At CX count of 0: just the identity circuit
    cx_count_circs: Vec<ManyCircuits<T>>,
}

impl<'m, T: CXCircuit> BFS<'m, T> {
    fn new(start_circ: T, moves: &'m ManyCircuits<T>) -> Self {
        let seen_circs = HashTable::<T, PRIME>::new();
        let cx_count_circs = vec![ManyCircuits::<T>::singleton(start_circ)];
        Self {
            moves,
            seen_circs,
            cx_count_circs,
        }
    }

    fn step(&mut self) {
        let depth = self.cx_count_circs.len();
        let mut new_circs = self.cx_count_circs[depth - 1].mult(self.moves);
        new_circs.retain(|circ| self.seen_circs.insert(*circ));
        println!("Depth {}: {} circuits", depth, new_circs.len());
        println!("{} collisions", self.seen_circs.n_collisions);
        self.cx_count_circs.push(new_circs);
    }
}

/// Breadth-first search, starting from identity circuit.
pub fn _bfs<T: CXCircuit>(moves: &ManyCircuits<T>, max_depth: usize) {
    let mut bfs = BFS::new(T::new(), moves);
    for _ in 1..=max_depth {
        bfs.step();
    }
}

/// Breadth-first search, starting from both ends and meet in the middle.
pub fn mitm_bfs<T: CXCircuit>(
    target_circ: T,
    moves: &ManyCircuits<T>,
    max_depth: usize,
) -> Option<T> {
    // Start one BFS at the identity circuit
    let mut forward = BFS::new(T::new(), moves);
    // Start one BFS at the target circuit
    let mut backward = BFS::new(target_circ, moves);
    for _ in 1..=max_depth {
        println!("forward:");
        forward.step();
        println!("backward:");
        backward.step();
        let intersection = forward.seen_circs.intersection(&backward.seen_circs);
        if !intersection.is_empty() {
            println!("Success! returning early");
            return Some(*intersection[0]);
        }
    }
    println!("No solution found at maximal depth, aborting");
    None
}
