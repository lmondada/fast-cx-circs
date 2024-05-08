use bfs::mitm_bfs;
use file_io::{parse_cx_circuit, parse_moves};

use std::fs::File;

mod bfs;
mod cx_circuit;
mod file_io;
mod hash_table;

fn main() {
    let file = File::open("in").expect("Unable to open file");
    let circuit = parse_cx_circuit(&file);
    let file = File::open("moves").expect("Unable to open file");
    let moves = parse_moves(&file);
    mitm_bfs(circuit, &moves, 4);
}
