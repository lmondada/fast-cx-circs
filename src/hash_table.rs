use crate::cx_circuit::CXCircuit;

/// A hash table for 16-qubit circuits.
pub struct HashTable<T, const N: usize> {
    /// Array of static size N, but using Vec for simple heap allocation.
    table: Vec<Option<T>>,
    pub n_collisions: usize,
}

impl<T: CXCircuit, const N: usize> HashTable<T, N> {
    /// Create a new hash table.
    pub fn new() -> Self {
        Self {
            table: vec![None; N],
            n_collisions: 0,
        }
    }

    /// Insert a circuit into the hash table.
    ///
    /// Returns `true` if the circuit was inserted successfully, `false` if the circuit was already in the table.
    pub fn insert(&mut self, circuit: T) -> bool {
        let hash = circuit.hash() % (N as u32);
        if self.table[hash as usize].is_some() {
            if self.table[hash as usize].as_ref().unwrap() != &circuit {
                self.n_collisions += 1;
            }
            false
        } else {
            self.table[hash as usize] = Some(circuit);
            true
        }
    }

    /// Intersection of two hash tables.
    pub fn intersection(&self, other: &Self) -> Vec<&T> {
        let mut intersection = Vec::new();
        for (a, b) in self.table.iter().zip(other.table.iter()) {
            let Some(a) = a else { continue };
            let Some(b) = b else { continue };
            if a == b {
                intersection.push(a);
            }
        }
        intersection
    }
}

#[cfg(test)]
mod tests {
    use crate::cx_circuit::CXCircuit16;

    use super::*;

    #[test]
    fn test_intersection() {
        let mut a = HashTable::<CXCircuit16, 10>::new();
        let mut b = HashTable::<CXCircuit16, 10>::new();
        let c = CXCircuit16::from_cxs([(0, 1), (1, 2), (4, 5)]);
        a.insert(CXCircuit16::new());
        b.insert(c);
        assert!(a.intersection(&b).is_empty());
        b.insert(CXCircuit16::new());
        assert_eq!(a.intersection(&b), vec![&CXCircuit16::new()]);
        a.insert(c);
        assert_eq!(a.intersection(&b), vec![&CXCircuit16::new(), &c]);
    }
}
