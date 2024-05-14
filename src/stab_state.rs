use std::fmt::Debug;

use fxhash::FxHashSet;

use crate::a_star::AStarValue;

/// A stabiliser state on N <= 16 qubits, defined by `N` X stabilisers.
///
/// Note that we would also need N Z stabilisers, but they do not matter for
/// the problem of finding CX circuits.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StabiliserState<const N: usize> {
    /// The X stabilisers
    x_stabs: [u16; N],
}

impl<const N: usize> StabiliserState<N> {
    pub fn from_strs<'a>(x_stabs_str: impl IntoIterator<Item = &'a str>) -> Self {
        assert!(N <= 16);
        let mut x_stabs = [0; N];

        for (stab_u16, stab_str) in x_stabs.iter_mut().zip(x_stabs_str) {
            let set_digits = stab_str
                .chars()
                .enumerate()
                .filter_map(|(j, x)| is_set(x).then_some(j));
            for j in set_digits {
                *stab_u16 ^= 1 << j;
            }
        }
        Self { x_stabs }
    }
}

impl<const N: usize> AStarValue for StabiliserState<N> {
    fn dist(&self, other: &Self) -> usize {
        self.x_stabs
            .iter()
            .zip(other.x_stabs.iter())
            .map(|(a, b)| (a != b) as usize)
            .sum()
    }

    fn is_complete(&self, qb: u8, target: &Self) -> bool {
        self.x_stabs[qb as usize] == target.x_stabs[qb as usize]
    }

    fn cx(&self, ctrl: u8, tgt: u8) -> Self {
        let mut new = self.clone();
        new.x_stabs[tgt as usize] ^= self.x_stabs[ctrl as usize];
        new
    }

    fn merge(&self, other: &Self, used_qubits: &FxHashSet<u8>) -> Self {
        let mut new = self.clone();
        for &qb in used_qubits {
            new.x_stabs[qb as usize] = other.x_stabs[qb as usize];
        }
        new
    }
}

fn is_set(x: char) -> bool {
    match x {
        'X' => true,
        'I' => false,
        _ => panic!("Invalid character"),
    }
}

impl<const N: usize> Debug for StabiliserState<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for &stab in &self.x_stabs {
            f.write_str(&(u16_as_str::<N>(stab, 'X') + "\n"))?;
        }
        Ok(())
    }
}

fn u16_as_str<const N: usize>(bits: u16, pauli: char) -> String {
    (0..N)
        .map(|i| bits & (1 << i) != 0)
        .map(|p| if p { pauli } else { 'I' })
        .collect()
}
