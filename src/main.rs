use bfs::mitm_bfs;
use file_io::{parse_cx_circuit, parse_moves};

use std::fs::File;
use clap::Parser;

mod bfs;
mod cx_circuit;
mod file_io;
mod hash_table;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of input circuit file
    #[arg(short, long, default_value_t = String::from("in"))]
    input: String,

    /// Name of moves file
    #[arg(short, long, default_value_t = String::from("moves"))]
    moves: String,
}

fn main() {

    let args = Args::parse();
    let input_filename = args.input;
    let moves_filename = args.moves;

    println!("Using input circuit in file \"{input_filename}\"");
    println!("Using moves in file \"{moves_filename}\"");

    let file = File::open(input_filename).expect("Unable to open input file");
    let circuit = parse_cx_circuit(&file);
    let file = File::open(moves_filename).expect("Unable to open moves file");
    let moves = parse_moves(&file);
    if let Some(circ) = mitm_bfs(circuit, &moves, 4) {
        println!("Found a solution: {circ:?}");
    } else {
        println!("No solution found");
    }
}
