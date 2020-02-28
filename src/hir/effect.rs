use crate::expr::Expression;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Effect {
    /// Conditional effect, meaning that the nested effect is only observable if the condition holds
    Conditional {
        condition: Expression,
        effect: Box<Effect>,
    },
    /// Memory at given address is fetched into the Cache
    CacheFetch {
        address: Expression,
        bit_width: usize,
    },
    /// Branch target is tracked in the Branch Target Buffer
    BranchTarget {
        location: Expression,
        target: Expression,
    },
    /// Branch condition (taken/not-taken) is tracked in the Pattern History Table
    BranchCondition {
        location: Expression,
        condition: Expression,
    },
}

impl Effect {
    /// Create a new `Effect::CacheFetch`.
    pub fn cache_fetch(address: Expression, bit_width: usize) -> Self {
        Self::CacheFetch { address, bit_width }
    }

    /// Create a new `Effect::BranchTarget`.
    pub fn branch_target(location: Expression, target: Expression) -> Self {
        Self::BranchTarget { location, target }
    }

    /// Create a new `Effect::BranchTarget`.
    pub fn branch_condition(location: Expression, condition: Expression) -> Self {
        Self::BranchCondition {
            location,
            condition,
        }
    }

    /// Make self conditional
    pub fn only_if(self, condition: Expression) -> Self {
        Self::Conditional {
            condition,
            effect: Box::new(self),
        }
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Conditional { condition, effect } => write!(f, "{} if {}", effect, condition),
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
