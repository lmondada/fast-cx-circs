use std::{
    fs::File,
    io::{self, BufRead},
};

use crate::cx_circuit::{CXCircuit, CXCircuit16, ManyCircuits};

fn parse_file(file: &File) -> Vec<(usize, usize)> {
    let mut res = vec![];
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line.expect("Unable to read line");
        let parts: Vec<usize> = line
            .split_whitespace()
            .map(|s| s.parse().expect("Parse error"))
            .collect();
        if parts.len() == 2 {
            res.push((parts[0], parts[1]));
        } else {
            panic!("Each line must contain exactly two numbers");
        }
    }
    res
}

/// Parse a circuit from a file.
pub fn parse_cx_circuit(file: &File) -> CXCircuit16 {
    let mut circuit = CXCircuit16::new();
    let all_cxs = parse_file(file);
    for (a, b) in &all_cxs {
        circuit.cx(*a, *b);
    }
    circuit
}

/// Parse a list of moves from a file.
pub fn parse_moves(file: &File) -> ManyCircuits<CXCircuit16> {
    let mut moves = ManyCircuits::new();
    let all_moves = parse_file(file);
    for (a, b) in all_moves {
        let cx_circ = CXCircuit16::from_cxs([(a, b)]);
        moves.push(cx_circ);
    }
    moves
}
