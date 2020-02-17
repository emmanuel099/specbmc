use crate::error::Result;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Sort {
    Boolean,
    Integer,
    BitVector(usize),
    Array { range: Box<Sort>, domain: Box<Sort> },
    Set { range: Box<Sort> },
    Memory,
    Cache,
    Predictor,
    BranchTargetBuffer,
}

impl Sort {
    pub fn boolean() -> Self {
        Self::Boolean
    }

    pub fn integer() -> Self {
        Self::Integer
    }

    pub fn bit_vector(width: usize) -> Self {
        Self::BitVector(width)
    }

    pub fn array(range: &Sort, domain: &Sort) -> Self {
        Self::Array {
            range: Box::new(range.clone()),
            domain: Box::new(domain.clone()),
        }
    }

    pub fn set(range: &Sort) -> Self {
        Self::Set {
            range: Box::new(range.clone()),
        }
    }

    pub fn memory() -> Self {
        Self::Memory
    }

    pub fn cache() -> Self {
        Self::Cache
    }

    pub fn predictor() -> Self {
        Self::Predictor
    }

    pub fn branch_target_buffer() -> Self {
        Self::BranchTargetBuffer
    }

    pub fn is_boolean(&self) -> bool {
        match self {
            Sort::Boolean => true,
            _ => false,
        }
    }

    pub fn is_integer(&self) -> bool {
        match self {
            Sort::Integer => true,
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

    pub fn is_set(&self) -> bool {
        match self {
            Sort::Set { .. } => true,
            _ => false,
        }
    }

    pub fn is_memory(&self) -> bool {
        match self {
            Sort::Memory => true,
            _ => false,
        }
    }

    pub fn is_cache(&self) -> bool {
        match self {
            Sort::Cache => true,
            _ => false,
        }
    }

    pub fn is_predictor(&self) -> bool {
        match self {
            Sort::Predictor => true,
            _ => false,
        }
    }

    pub fn is_branch_target_buffer(&self) -> bool {
        match self {
            Sort::BranchTargetBuffer => true,
            _ => false,
        }
    }

    pub fn expect_boolean(&self) -> Result<()> {
        if self.is_boolean() {
            Ok(())
        } else {
            Err(format!("Expected Boolean but was {}", self).into())
        }
    }

    pub fn expect_integer(&self) -> Result<()> {
        if self.is_integer() {
            Ok(())
        } else {
            Err(format!("Expected Integer but was {}", self).into())
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

    pub fn expect_set(&self) -> Result<()> {
        if self.is_set() {
            Ok(())
        } else {
            Err(format!("Expected Set but was {}", self).into())
        }
    }

    pub fn expect_memory(&self) -> Result<()> {
        if self.is_memory() {
            Ok(())
        } else {
            Err(format!("Expected Memory but was {}", self).into())
        }
    }

    pub fn expect_cache(&self) -> Result<()> {
        if self.is_cache() {
            Ok(())
        } else {
            Err(format!("Expected Cache but was {}", self).into())
        }
    }

    pub fn expect_predictor(&self) -> Result<()> {
        if self.is_predictor() {
            Ok(())
        } else {
            Err(format!("Expected Predictor but was {}", self).into())
        }
    }

    pub fn expect_branch_target_buffer(&self) -> Result<()> {
        if self.is_branch_target_buffer() {
            Ok(())
        } else {
            Err(format!("Expected BranchTargetBuffer but was {}", self).into())
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

    pub fn unwrap_set(&self) -> &Sort {
        match self {
            Sort::Set { range } => range,
            _ => panic!("Expected Set"),
        }
    }

    /// Returns whether this `Sort` survives a transient-execution rollback or not.
    pub fn is_rollback_persistent(&self) -> bool {
        match self {
            Sort::Cache => true,
            _ => false,
        }
    }
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Sort::Boolean => write!(f, "Boolean"),
            Sort::Integer => write!(f, "Integer"),
            Sort::BitVector(width) => write!(f, "BitVec<{}>", width),
            Sort::Array { range, domain } => write!(f, "Array<{}, {}>", range, domain),
            Sort::Set { range } => write!(f, "Set<{}>", range),
            Sort::Memory => write!(f, "Memory"),
            Sort::Cache => write!(f, "Cache"),
            Sort::Predictor => write!(f, "Predictor"),
            Sort::BranchTargetBuffer => write!(f, "BranchTargetBuffer"),
        }
    }
}
