//! Circuits with only CX gates.

use std::hash::{Hash, Hasher};
use std::num::NonZeroU16;

use crate::a_star::AStarValue;

/// A trait for a CX circuit with a fixed number of qubits.
pub trait CXCircuit: Copy + Eq + Sized + Hash + Send + Sync {
    /// A new CX circuit.
    fn new() -> Self;

    /// Apply a CX gate to the circuit.
    fn add_cx(&mut self, ctrl: usize, tgt: usize);

    /// Compose two CX circuits together.
    fn mult(&self, other: &Self) -> Self {
        let other_t = other.transpose();
        self.mult_transpose(&other_t)
    }

    fn mult_transpose(&self, other: &Self) -> Self;
    fn transpose(&self) -> Self;

    /// Construct a CX circuit from a list of CX gates.
    fn from_cxs(cxs: impl IntoIterator<Item = (usize, usize)>) -> Self {
        let mut cx = Self::new();
        for (ctrl, tgt) in cxs {
            cx.add_cx(ctrl, tgt);
        }
        cx
    }
}

/// A 16-qubit CX circuit.
///
/// Represented by a boolean matrix.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CXCircuit16 {
    matrix: [NonZeroU16; 16],
}

impl Hash for CXCircuit16 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Pointer cast [NonZeroU16; 16] to [u64; 4]
        let matrix = unsafe { &*self.matrix.as_ptr().cast::<[u64; 4]>() };
        for &elem in matrix {
            state.write_u64(elem);
        }
    }
}

impl AStarValue for CXCircuit16 {
    fn dist(&self, other: &Self) -> usize {
        self.matrix
            .iter()
            .zip(other.matrix.iter())
            .map(|(&a, &b)| (a != b) as usize)
            .sum()
    }

    fn cx(&self, ctrl: u8, tgt: u8) -> Self {
        let mut cx = self.clone();
        cx.add_cx(ctrl as usize, tgt as usize);
        cx
    }

    fn merge(&self, other: &Self, used_qubits: &fxhash::FxHashSet<u8>) -> Self {
        let mut merge = self.clone();
        for &qb in used_qubits {
            merge.matrix[qb as usize] = other.matrix[qb as usize];
        }
        merge
    }

    fn is_complete(&self, qb: u8, target: &Self) -> bool {
        self.matrix[qb as usize] == target.matrix[qb as usize]
    }
}

fn eye<const N: usize>() -> [NonZeroU16; N] {
    let mut matrix: [NonZeroU16; N] = [NonZeroU16::new(1).unwrap(); N];
    for i in 0..N {
        matrix[i] = NonZeroU16::new(1 << i).unwrap();
    }
    matrix
}

impl CXCircuit for CXCircuit16 {
    fn new() -> Self {
        Self { matrix: eye() }
    }

    fn add_cx(&mut self, ctrl: usize, tgt: usize) {
        let ctrl_value = self.matrix[ctrl].get();
        let tgt_value = self.matrix[tgt].get();
        let new_tgt_value = tgt_value ^ ctrl_value;
        self.matrix[tgt] = NonZeroU16::new(new_tgt_value).unwrap();
    }

    fn mult_transpose(&self, other: &Self) -> Self {
        let mut result = [0; 16];
        for i in 0..16 {
            for j in 0..16 {
                let elem_wise_mult = self.matrix[i].get() & other.matrix[j].get();
                let bit = (elem_wise_mult.count_ones() % 2) as u16;
                if bit == 1 {
                    result[i] += bit << j;
                }
            }
        }
        Self::from_mat(result)
    }

    fn transpose(&self) -> Self {
        let mut transposed = Self::new();
        for i in 0..16 {
            let mut row = 0;
            for j in 0..16 {
                if self.matrix[j].get() & (1 << i) != 0 {
                    row += 1 << j;
                }
            }
            transposed.matrix[i] = NonZeroU16::new(row).unwrap();
        }
        transposed
    }
}

impl CXCircuit16 {
    fn from_mat(matrix: [u16; 16]) -> Self {
        let matrix = matrix.map(|x| NonZeroU16::new(x).unwrap());
        Self { matrix }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sum_pow_two(vals: impl IntoIterator<Item = u16>) -> NonZeroU16 {
        let mut sum = 0;
        for val in vals {
            sum += 1 << val;
        }
        NonZeroU16::new(sum).unwrap()
    }

    #[test]
    fn test_cx_16() {
        let mut cx = CXCircuit16::new();
        cx.add_cx(0, 1);
        cx.add_cx(3, 2);
        cx.add_cx(2, 6);

        let mut res = eye();
        res[1] = sum_pow_two([0, 1]);
        res[2] = sum_pow_two([3, 2]);
        res[6] = sum_pow_two([2, 3, 6]);
        assert_eq!(cx.matrix, res);
    }

    #[test]
    fn test_cx_cx() {
        let mut cx_cx = CXCircuit16::new();
        cx_cx.add_cx(0, 1);
        cx_cx.add_cx(0, 1);
        assert_eq!(cx_cx.matrix, eye());
    }

    #[test]
    fn transpose_16() {
        let mat = [
            0b0000010000000001,
            0b0000100000000010,
            0b0001000000000100,
            0b0010000000001000,
            0b0100000000010000,
            0b1000000000100000,
            0b0000000001000000,
            0b0000000010000000,
            0b0000000100000000,
            0b0000001000000000,
            0b0000010000000000,
            0b0000100000000000,
            0b0001000000000000,
            0b0010000000001000,
            0b0100000000000000,
            0b1000000000000000,
        ];
        let mat_t = [
            0b0000000000000001,
            0b0000000000000010,
            0b0000000000000100,
            0b0010000000001000,
            0b0000000000010000,
            0b0000000000100000,
            0b0000000001000000,
            0b0000000010000000,
            0b0000000100000000,
            0b0000001000000000,
            0b0000010000000001,
            0b0000100000000010,
            0b0001000000000100,
            0b0010000000001000,
            0b0100000000010000,
            0b1000000000100000,
        ];
        let cx = CXCircuit16::from_mat(mat);
        let t = cx.transpose();
        assert_eq!(t, CXCircuit16::from_mat(mat_t));
    }
}
