use std::{
    fs::File,
    io::{self, BufRead, Write},
};

use crate::{
    cx_circuit::{CXCircuit, CXCircuit16},
    stab_state::StabiliserState,
    Moves, CX,
};

fn parse_file(file: &File) -> io::Result<Vec<(usize, usize)>> {
    let mut res = vec![];
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let parts: Vec<usize> = line?
            .split_whitespace()
            .map(|s| s.parse().expect("Parse error"))
            .collect();
        if parts.len() == 2 {
            res.push((parts[0], parts[1]));
        } else {
            panic!("Each line must contain exactly two numbers");
        }
    }
    Ok(res)
}

/// Parse a circuit from a file.
pub fn parse_cx_circuit(file: &File) -> io::Result<CXCircuit16> {
    let mut circuit = CXCircuit16::new();
    let all_cxs = parse_file(file)?;
    for (a, b) in &all_cxs {
        circuit.add_cx(*a, *b);
    }
    Ok(circuit)
}

pub fn parse_stabiliser(file: &File) -> io::Result<StabiliserState<16>> {
    let reader = io::BufReader::new(file);
    let lines = reader.lines().collect::<io::Result<Vec<_>>>()?;
    Ok(StabiliserState::from_strs(lines.iter().map(|s| s.as_str())))
    // let all_cxs = parse_file(file)?;
    // for (a, b) in &all_cxs {
    //     stabiliser.add_cx(*a, *b);
    // }
    // Ok(stabiliser)
}

/// Parse a list of moves from a file.
///
/// Careful: moves are always as stored as the transpose!
pub fn parse_moves(file: &File) -> io::Result<(Vec<(usize, usize)>, Moves<CXCircuit16>)> {
    let mut moves = Vec::new();
    let mut moves_inds = Vec::new();
    for (a, b) in parse_file(file)? {
        if a >= 16 || b >= 16 {
            panic!("We currently only support qubits indices up to 15");
        }
        let cx_circ = CXCircuit16::from_cxs([(a, b)]);
        moves.push(cx_circ.transpose());
        moves_inds.push((a, b));
        let cx_circ = CXCircuit16::from_cxs([(b, a)]);
        moves.push(cx_circ.transpose());
        moves_inds.push((b, a));
    }
    Ok((moves_inds, moves))
}

pub fn save_solution(file: &mut File, solution: &[CX]) -> io::Result<()> {
    for &CX { ctrl, tgt } in solution {
        writeln!(file, "{} {}", ctrl, tgt)?;
    }
    Ok(())
}
