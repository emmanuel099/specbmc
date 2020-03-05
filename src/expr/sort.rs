use crate::environment;
use crate::error::Result;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Sort {
    Boolean,
    Integer,
    BitVector(usize),
    Array { range: Box<Sort>, domain: Box<Sort> },
    // Arch
    Memory,
    Predictor,
    Cache,
    BranchTargetBuffer,
    PatternHistoryTable,
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

    pub fn word() -> Self {
        Self::bit_vector(environment::WORD_SIZE)
    }

    pub fn array(range: &Self, domain: &Self) -> Self {
        Self::Array {
            range: Box::new(range.clone()),
            domain: Box::new(domain.clone()),
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

    pub fn pattern_history_table() -> Self {
        Self::PatternHistoryTable
    }

    pub fn is_boolean(&self) -> bool {
        match self {
            Self::Boolean => true,
            _ => false,
        }
    }

    pub fn is_integer(&self) -> bool {
        match self {
            Self::Integer => true,
            _ => false,
        }
    }

    pub fn is_bit_vector(&self) -> bool {
        match self {
            Self::BitVector(..) => true,
            _ => false,
        }
    }

    pub fn is_word(&self) -> bool {
        match self {
            Self::BitVector(environment::WORD_SIZE) => true,
            _ => false,
        }
    }

    pub fn is_array(&self) -> bool {
        match self {
            Self::Array { .. } => true,
            _ => false,
        }
    }

    pub fn is_memory(&self) -> bool {
        match self {
            Self::Memory => true,
            _ => false,
        }
    }

    pub fn is_cache(&self) -> bool {
        match self {
            Self::Cache => true,
            _ => false,
        }
    }

    pub fn is_predictor(&self) -> bool {
        match self {
            Self::Predictor => true,
            _ => false,
        }
    }

    pub fn is_branch_target_buffer(&self) -> bool {
        match self {
            Self::BranchTargetBuffer => true,
            _ => false,
        }
    }

    pub fn is_pattern_history_table(&self) -> bool {
        match self {
            Self::PatternHistoryTable => true,
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

    pub fn expect_word(&self) -> Result<()> {
        if self.is_word() {
            Ok(())
        } else {
            Err(format!("Expected Word but was {}", self).into())
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

    pub fn expect_pattern_history_table(&self) -> Result<()> {
        if self.is_pattern_history_table() {
            Ok(())
        } else {
            Err(format!("Expected PatternHistoryTable but was {}", self).into())
        }
    }

    pub fn expect_sort(&self, sort: &Self) -> Result<()> {
        if self == sort {
            Ok(())
        } else {
            Err(format!("Expected {} but was {}", sort, self).into())
        }
    }

    pub fn unwrap_bit_vector(&self) -> usize {
        match self {
            Self::BitVector(width) => *width,
            _ => panic!("Expected BitVec"),
        }
    }

    pub fn unwrap_array(&self) -> (&Self, &Self) {
        match self {
            Self::Array { range, domain } => (range, domain),
            _ => panic!("Expected Array"),
        }
    }

    /// Returns whether this `Sort` survives a transient-execution rollback or not.
    pub fn is_rollback_persistent(&self) -> bool {
        match self {
            Self::Cache | Self::BranchTargetBuffer | Self::PatternHistoryTable => true,
            _ => false,
        }
    }
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Boolean => write!(f, "Boolean"),
            Self::Integer => write!(f, "Integer"),
            Self::BitVector(width) => write!(f, "BitVec<{}>", width),
            Self::Array { range, domain } => write!(f, "Array<{}, {}>", range, domain),
            Self::Memory => write!(f, "Memory"),
            Self::Predictor => write!(f, "Predictor"),
            Self::Cache => write!(f, "Cache"),
            Self::BranchTargetBuffer => write!(f, "BranchTargetBuffer"),
            Self::PatternHistoryTable => write!(f, "PatternHistoryTable"),
        }
    }
}
