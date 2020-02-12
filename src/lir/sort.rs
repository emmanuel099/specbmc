use crate::error::Result;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Sort {
    Bool,
    BitVector(usize),
    Array { range: Box<Sort>, domain: Box<Sort> },
    Memory,
}

impl Sort {
    pub fn bit_vector(width: usize) -> Self {
        Self::BitVector(width)
    }

    pub fn array(range: &Sort, domain: &Sort) -> Self {
        Self::Array {
            range: Box::new(range.clone()),
            domain: Box::new(domain.clone()),
        }
    }

    pub fn memory() -> Self {
        Self::Memory
    }

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

    pub fn is_array(&self) -> bool {
        match self {
            Sort::Array { .. } => true,
            _ => false,
        }
    }

    pub fn is_memory(&self) -> bool {
        match self {
            Sort::Memory => true,
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

    pub fn expect_array(&self) -> Result<()> {
        if self.is_array() {
            Ok(())
        } else {
            Err(format!("Expected Array but was {}", self).into())
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

    pub fn unwrap_bit_vector(&self) -> usize {
        match self {
            Sort::BitVector(width) => *width,
            _ => panic!("Expected BitVec"),
        }
    }

    pub fn unwrap_array(&self) -> (&Sort, &Sort) {
        match self {
            Sort::Array { range, domain } => (range, domain),
            _ => panic!("Expected Array"),
        }
    }
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Sort::Bool => write!(f, "Bool"),
            Sort::BitVector(width) => write!(f, "BitVec<{}>", width),
            Sort::Array { range, domain } => write!(f, "Array<{}, {}>", range, domain),
            Sort::Memory => write!(f, "Memory"),
        }
    }
}
