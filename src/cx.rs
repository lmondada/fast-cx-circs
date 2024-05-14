use std::fmt::Debug;

/// A CX gate on two qubits.
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub struct CX {
    pub ctrl: u8,
    pub tgt: u8,
}

impl From<(usize, usize)> for CX {
    fn from((ctrl, tgt): (usize, usize)) -> Self {
        Self {
            ctrl: ctrl as u8,
            tgt: tgt as u8,
        }
    }
}

impl Debug for CX {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CX({}, {})", self.ctrl, self.tgt)
    }
}
