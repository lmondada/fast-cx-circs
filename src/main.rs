use bfs::mitm_bfs;
use file_io::{parse_cx_circuit, parse_moves};

use clap::Parser;
use fxhash::FxHashMap;
use std::fs::File;

use crate::file_io::save_solution;

mod bfs;
mod cx_circuit;
mod file_io;

type CircMoves<T> = FxHashMap<T, usize>;
type Moves<T> = Vec<T>;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of input circuit file
    #[arg(short, long, default_value_t = String::from("in"))]
    input: String,

    /// Name of moves file
    #[arg(short, long, default_value_t = String::from("all_to_all"))]
    moves: String,

    /// Name of output file
    #[arg(short, long, default_value_t = String::from("out"))]
    output: String,

    /// Maximum depth of BFS
    #[arg(short, long, default_value_t = 5)]
    depth: usize,
}

fn main() {
    let args = Args::parse();
    let input_filename = args.input;
    let moves_filename = args.moves;
    let output_filename = args.output;
    let max_depth = args.depth;

    println!("Using input circuit in file \"{input_filename}\"");
    println!("Using moves in file \"{moves_filename}\"");

    let circuit = {
        let file = File::open(input_filename).expect("Unable to open input file");
        parse_cx_circuit(&file).expect("Unable to parse input circuit")
    };
    let (move_inds, moves) = {
        let file = File::open(moves_filename).expect("Unable to open moves file");
        parse_moves(&file).expect("Unable to parse moves files")
    };
    if let Some(solution) = mitm_bfs(circuit, &moves, max_depth) {
        println!("Found a solution: {solution:?}");
        println!("Writing to {output_filename}");
        let mut file = File::create(output_filename).expect("Unable to open solution file");
        save_solution(&mut file, &solution, &move_inds).expect("Unable to save solution");
    } else {
        println!("No solution found");
    }
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
            circuit.cx(ctrl, tgt);
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
        let test_cases = [vec![(0, 2), (2, 0)], vec![(0, 4), (5, 0), (4, 0)]];
        for cx_list in &test_cases {
            run_test_e2e(cx_list, bfs);
            run_test_e2e(cx_list, mitm_bfs);
        }
    }
}
