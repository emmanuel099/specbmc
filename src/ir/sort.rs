use std::fmt;

#[derive(Clone, Debug)]
pub enum Sort {
    Bool,
    BitVector(usize),
    Memory,
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Sort::Bool => write!(f, "Bool"),
            Sort::BitVector(width) => write!(f, "BitVec<{}>", width),
            Sort::Memory => write!(f, "Bool"),
        }
    }
}
