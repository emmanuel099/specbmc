use crate::environment;
use crate::error::Result;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Sort {
    Boolean,
    Integer,
    BitVector(usize),
    Array { range: Box<Sort>, domain: Box<Sort> },
    List { domain: Box<Sort> },
    Tuple { fields: Vec<Sort> },
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

    pub fn array(range: Self, domain: Self) -> Self {
        Self::Array {
            range: Box::new(range),
            domain: Box::new(domain),
        }
    }

    pub fn list(domain: Self) -> Self {
        Self::List {
            domain: Box::new(domain),
        }
    }

    pub fn tuple(fields: Vec<Sort>) -> Self {
        Self::Tuple { fields }
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
        matches!(self, Self::Boolean)
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Self::Integer)
    }

    pub fn is_bit_vector(&self) -> bool {
        matches!(self, Self::BitVector(..))
    }

    pub fn is_word(&self) -> bool {
        matches!(self, Self::BitVector(environment::WORD_SIZE))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array { .. })
    }

    pub fn is_list(&self) -> bool {
        matches!(self, Self::List { .. })
    }

    pub fn is_tuple(&self) -> bool {
        matches!(self, Self::Tuple { .. })
    }

    pub fn is_memory(&self) -> bool {
        matches!(self, Self::Memory)
    }

    pub fn is_cache(&self) -> bool {
        matches!(self, Self::Cache)
    }

    pub fn is_predictor(&self) -> bool {
        matches!(self, Self::Predictor)
    }

    pub fn is_branch_target_buffer(&self) -> bool {
        matches!(self, Self::BranchTargetBuffer)
    }

    pub fn is_pattern_history_table(&self) -> bool {
        matches!(self, Self::PatternHistoryTable)
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

    pub fn expect_list(&self) -> Result<()> {
        if self.is_list() {
            Ok(())
        } else {
            Err(format!("Expected List but was {}", self).into())
        }
    }

    pub fn expect_tuple(&self) -> Result<()> {
        if self.is_tuple() {
            Ok(())
        } else {
            Err(format!("Expected Tuple but was {}", self).into())
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

    pub fn unwrap_list(&self) -> &Self {
        match self {
            Self::List { domain } => domain,
            _ => panic!("Expected List"),
        }
    }

    pub fn unwrap_tuple(&self) -> &[Self] {
        match self {
            Self::Tuple { fields } => fields,
            _ => panic!("Expected Tuple"),
        }
    }
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boolean => write!(f, "Boolean"),
            Self::Integer => write!(f, "Integer"),
            Self::BitVector(width) => write!(f, "BitVec<{}>", width),
            Self::Array { range, domain } => write!(f, "Array<{}, {}>", range, domain),
            Self::List { domain } => write!(f, "List<{}>", domain),
            Self::Tuple { fields } => {
                write!(f, "Tuple<")?;
                let mut is_first = true;
                for sort in fields.iter() {
                    if !is_first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", sort)?;
                    is_first = false;
                }
                write!(f, ">")
            }
            Self::Memory => write!(f, "Memory"),
            Self::Predictor => write!(f, "Predictor"),
            Self::Cache => write!(f, "Cache"),
            Self::BranchTargetBuffer => write!(f, "BranchTargetBuffer"),
            Self::PatternHistoryTable => write!(f, "PatternHistoryTable"),
        }
    }
}
