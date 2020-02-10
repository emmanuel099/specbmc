use crate::error::Result;
use std::fmt;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub enum Sort {
    Bool,
    BitVector(usize),
    Memory(usize),
}

impl Sort {
    pub fn is_bool(&self) -> bool {
        match self {
            Sort::Bool => true,
            _ => false,
        }
    }

    pub fn is_bit_vector(&self) -> bool {
        match self {
            Sort::BitVector(..) => true,
            _ => false,
        }
    }

    pub fn is_memory(&self) -> bool {
        match self {
            Sort::Memory(..) => true,
            _ => false,
        }
    }

    pub fn expect_bool(&self) -> Result<()> {
        if self.is_bool() {
            Ok(())
        } else {
            Err(format!("Expected Bool but was {}", self).into())
        }
    }

    pub fn expect_bit_vector(&self) -> Result<()> {
        if self.is_bit_vector() {
            Ok(())
        } else {
            Err(format!("Expected BitVec but was {}", self).into())
        }
    }

    pub fn expect_memory(&self) -> Result<()> {
        if self.is_memory() {
            Ok(())
        } else {
            Err(format!("Expected Memory but was {}", self).into())
        }
    }

    pub fn expect_sort(&self, sort: &Sort) -> Result<()> {
        if self == sort {
            Ok(())
        } else {
            Err(format!("Expected {} but was {}", sort, self).into())
        }
    }
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Sort::Bool => write!(f, "Bool"),
            Sort::BitVector(width) => write!(f, "BitVec<{}>", width),
            Sort::Memory(width) => write!(f, "Memory<{}>", width),
        }
    }
}
