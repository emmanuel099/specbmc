use crate::expr::Constant;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Effect {
    /// Memory at given address is fetched into the Cache
    CacheFetch { address: Constant, bit_width: usize },
    /// Branch target is tracked in the Branch Target Buffer
    BranchTarget {
        location: Constant,
        target: Constant,
    },
    /// Branch condition (taken/not-taken) is tracked in the Pattern History Table
    BranchCondition {
        location: Constant,
        condition: Constant,
    },
}

impl Effect {
    /// Create a new `Effect::CacheFetch`.
    pub fn cache_fetch(address: Constant, bit_width: usize) -> Self {
        Self::CacheFetch { address, bit_width }
    }

    /// Create a new `Effect::BranchTarget`.
    pub fn branch_target(location: Constant, target: Constant) -> Self {
        Self::BranchTarget { location, target }
    }

    /// Create a new `Effect::BranchTarget`.
    pub fn branch_condition(location: Constant, condition: Constant) -> Self {
        Self::BranchCondition {
            location,
            condition,
        }
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::CacheFetch { address, bit_width } => {
                write!(f, "cache_fetch({}, {})", address, bit_width)
            }
            Self::BranchTarget { location, target } => {
                write!(f, "branch_target({}, {})", location, target)
            }
            Self::BranchCondition {
                location,
                condition,
            } => write!(f, "branch_condition({}, {})", location, condition),
        }
    }
}
