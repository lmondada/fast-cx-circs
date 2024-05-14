use a_star::AStarValue;
use bfs::mitm_bfs;
use cx_circuit::{CXCircuit, CXCircuit16};
use file_io::{parse_cx_circuit, parse_moves};

use clap::Parser;
use fxhash::FxHashMap;
use itertools::Itertools;
use stab_state::StabiliserState;
use std::fs::File;

use crate::{
    a_star::a_star,
    cx::CX,
    file_io::{parse_stabiliser, save_solution},
};

mod a_star;
mod bfs;
mod cx;
mod cx_circuit;
mod file_io;
mod stab_state;

type CircMoves<T> = FxHashMap<T, usize>;
type Moves<T> = Vec<T>;

/// Search algorithm to use
#[derive(clap::ValueEnum, Clone, Default, Debug, PartialEq, Eq)]
enum SearchAlgorithm {
    /// A boosted man-in-the-middle BFS
    ///
    /// Can run in parallel but very memory hungry.
    MITM,
    /// Custom A* search
    ///
    /// Should be leaner, but no parallelism yet
    #[default]
    Astar,
    /// Custom A* search, on stabiliser states
    ///
    /// In this case, input
    /// and output files should be X-stabiliser states.
    ///
    /// Should be leaner, but no parallelism yet.
    AstarStabiliser,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of target circuit or state
    #[arg(short, long, default_value_t = String::from("in"))]
    target: String,

    /// Name of source circuit or state. For circuits, defaults to identity.
    #[arg(short, long)]
    source: Option<String>,

    /// Name of moves file
    #[arg(short, long, default_value_t = String::from("all_to_all"))]
    moves: String,

    /// Name of output file
    #[arg(short, long, default_value_t = String::from("out"))]
    output: String,

    /// Maximum depth of BFS. The maximum gate count will be 3*depth.
    /// Warning: I do not recommend setting this value higher than 5, memory
    /// consumption goes through the roof.
    #[arg(short, long, default_value_t = 5)]
    depth: usize,

    #[arg(short, long, value_enum, default_value_t)]
    algo: SearchAlgorithm,
}

fn main() {
    let start_time = std::time::Instant::now();

    let args = Args::parse();
    let target_filename = args.target;
    let source_filename = args.source;
    let moves_filename = args.moves;
    let output_filename = args.output;
    let max_depth = args.depth;

    let source;
    let target;
    if args.algo == SearchAlgorithm::AstarStabiliser {
        let source_filename =
            source_filename.expect("For stabiliser search, source must be specified");
        println!("Using source stabiliser in file \"{source_filename}\"");
        let file = File::open(source_filename).expect("Unable to open source file");
        source = CircuitOrStabiliser::Stabiliser(
            parse_stabiliser(&file).expect("Unable to parse source stabiliser"),
        );

        println!("Using target stabiliser in file \"{target_filename}\"");
        let file = File::open(target_filename).expect("Unable to open target file");
        target = CircuitOrStabiliser::Stabiliser(
            parse_stabiliser(&file).expect("Unable to parse target stabiliser"),
        );
    } else {
        if let Some(source_filename) = source_filename {
            println!("Using source circuit in file \"{source_filename}\"");
            let file = File::open(source_filename).expect("Unable to open source file");
            source = CircuitOrStabiliser::Circuit(
                parse_cx_circuit(&file).expect("Unable to parse source circuit"),
            );
        } else {
            println!("Using identity circuit as source");
            source = CircuitOrStabiliser::Circuit(CXCircuit16::new());
        }
        println!("Using target circuit in file \"{target_filename}\"");

        let file = File::open(target_filename).expect("Unable to open target file");
        target = CircuitOrStabiliser::Circuit(
            parse_cx_circuit(&file).expect("Unable to parse target circuit"),
        );
    }
    println!("Using moves in file \"{moves_filename}\"");
    let (move_inds, moves) = {
        let file = File::open(moves_filename).expect("Unable to open moves file");
        parse_moves(&file).expect("Unable to parse moves files")
    };

    // TODO make the function signatures match better
    let solution = match args.algo {
        SearchAlgorithm::MITM => mitm_bfs(
            source.unwrap_circuit_ref(),
            target.unwrap_circuit_ref(),
            &moves,
            max_depth,
            true,
        )
        .map(|moves| moves.iter().map(|mv| move_inds[*mv].into()).collect()),
        SearchAlgorithm::Astar => {
            let moves = move_inds.iter().copied().map_into();
            a_star(
                source.unwrap_circuit_ref(),
                &target.unwrap_circuit_ref(),
                moves,
                Some(max_depth),
            )
        }
        SearchAlgorithm::AstarStabiliser => {
            let moves = move_inds.iter().copied().map_into();
            a_star(
                source.unwrap_stabiliser_ref(),
                &target.unwrap_stabiliser_ref(),
                moves,
                Some(max_depth),
            )
        }
    };

    if let Some(solution) = solution {
        println!("Found a solution: {solution:?}");

        if check_solution_correctness(&solution, source, &target) {
            println!("Correctness check passed");
            println!("Writing to {output_filename}");
            let mut file = File::create(output_filename).expect("Unable to open solution file");
            save_solution(&mut file, &solution).expect("Unable to save solution");
        } else {
            println!("Solution is incorrect! Please report this as a bug. Aborting");
        }
    } else {
        println!("No solution found");
    }

    let elapsed_time = start_time.elapsed();
    println!("\nTotal execution time: {:.2?}", elapsed_time);
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CircuitOrStabiliser {
    Circuit(CXCircuit16),
    Stabiliser(StabiliserState<16>),
}

impl CircuitOrStabiliser {
    fn unwrap_circuit_ref(&self) -> CXCircuit16 {
        match self {
            Self::Circuit(circuit) => circuit.clone(),
            Self::Stabiliser(_) => panic!("Expected circuit"),
        }
    }

    fn unwrap_stabiliser_ref(&self) -> StabiliserState<16> {
        match self {
            Self::Circuit(_) => panic!("Expected stabiliser"),
            Self::Stabiliser(stabiliser) => stabiliser.clone(),
        }
    }
}

impl AStarValue for CircuitOrStabiliser {
    fn dist(&self, other: &Self) -> usize {
        match (self, other) {
            (Self::Circuit(a), Self::Circuit(b)) => a.dist(b),
            (Self::Stabiliser(a), Self::Stabiliser(b)) => a.dist(b),
            _ => panic!("Expected same type"),
        }
    }

    fn is_complete(&self, qb: u8, target: &Self) -> bool {
        match (self, target) {
            (Self::Circuit(a), Self::Circuit(b)) => a.is_complete(qb, b),
            (Self::Stabiliser(a), Self::Stabiliser(b)) => a.is_complete(qb, b),
            _ => panic!("Expected same type"),
        }
    }

    fn cx(&self, ctrl: u8, tgt: u8) -> Self {
        match self {
            Self::Circuit(circuit) => Self::Circuit(circuit.cx(ctrl, tgt)),
            Self::Stabiliser(stabiliser) => Self::Stabiliser(stabiliser.cx(ctrl, tgt)),
        }
    }

    fn merge(&self, other: &Self, used_qubits: &fxhash::FxHashSet<u8>) -> Self {
        match (self, other) {
            (Self::Circuit(a), Self::Circuit(b)) => Self::Circuit(a.merge(b, used_qubits)),
            (Self::Stabiliser(a), Self::Stabiliser(b)) => Self::Stabiliser(a.merge(b, used_qubits)),
            _ => panic!("Expected same type"),
        }
    }
}

fn check_solution_correctness<V: AStarValue>(solution: &[CX], mut source: V, target: &V) -> bool {
    for &CX { ctrl, tgt } in solution {
        source = source.cx(ctrl, tgt);
    }
    source == *target
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use crate::{
        bfs::{bfs, mitm_bfs},
        cx_circuit::{CXCircuit, CXCircuit16},
        file_io::parse_moves,
        Moves,
    };

    fn run_test_e2e(
        cx_list: &[(usize, usize)],
        bfs: impl Fn(CXCircuit16, &Moves<CXCircuit16>, usize) -> Option<Vec<usize>>,
    ) {
        let (move_inds, moves) = {
            let file = File::open("all_to_all").expect("Unable to open moves file");
            parse_moves(&file).expect("Unable to parse moves files")
        };
        let mut circuit = CXCircuit16::new();
        for &(ctrl, tgt) in cx_list {
            circuit.add_cx(ctrl, tgt);
        }
        let solution = bfs(circuit, &moves, 5).unwrap();
        assert_eq!(
            solution
                .iter()
                .map(|&move_ind| move_inds[move_ind])
                .collect::<Vec<_>>(),
            cx_list
        );
    }

    #[test]
    fn simle_case_e2e() {
        let test_cases = [vec![(0, 2), (2, 0)], vec![(0, 4), (4, 5), (5, 0)]];
        for cx_list in &test_cases {
            run_test_e2e(cx_list, bfs);
            run_test_e2e(cx_list, |a, b, c| {
                mitm_bfs(CXCircuit16::new(), a, b, c, false)
            });
        }
    }
}
