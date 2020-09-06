use crate::expr::{Expression, Variable};
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

    /// Get each `Variable` read by this `Effect`.
    pub fn variables(&self) -> Vec<&Variable> {
        match self {
            Self::Conditional { condition, effect } => condition
                .variables()
                .into_iter()
                .chain(effect.variables().into_iter())
                .collect(),
            Self::CacheFetch { address, .. } => address.variables(),
            Self::BranchTarget { location, target } => location
                .variables()
                .into_iter()
                .chain(target.variables().into_iter())
                .collect(),
            Self::BranchCondition {
                location,
                condition,
            } => location
                .variables()
                .into_iter()
                .chain(condition.variables().into_iter())
                .collect(),
        }
    }

    /// Get a mutable reference to each `Variable` read by this `Effect`.
    pub fn variables_mut(&mut self) -> Vec<&mut Variable> {
        match self {
            Self::Conditional { condition, effect } => condition
                .variables_mut()
                .into_iter()
                .chain(effect.variables_mut().into_iter())
                .collect(),
            Self::CacheFetch { address, .. } => address.variables_mut(),
            Self::BranchTarget { location, target } => location
                .variables_mut()
                .into_iter()
                .chain(target.variables_mut().into_iter())
                .collect(),
            Self::BranchCondition {
                location,
                condition,
            } => location
                .variables_mut()
                .into_iter()
                .chain(condition.variables_mut().into_iter())
                .collect(),
        }
    }

    /// Get each `Expression` of this `Effect`.
    pub fn expressions(&self) -> Vec<&Expression> {
        match self {
            Self::Conditional { condition, effect } => vec![condition]
                .into_iter()
                .chain(effect.expressions())
                .collect(),
            Self::CacheFetch { address, .. } => vec![address],
            Self::BranchTarget { location, target } => vec![location, target],
            Self::BranchCondition {
                location,
                condition,
            } => vec![location, condition],
        }
    }

    /// Get a mutable reference to each `Expression` of this `Effect`.
    pub fn expressions_mut(&mut self) -> Vec<&mut Expression> {
        match self {
            Self::Conditional { condition, effect } => vec![condition]
                .into_iter()
                .chain(effect.expressions_mut())
                .collect(),
            Self::CacheFetch { address, .. } => vec![address],
            Self::BranchTarget { location, target } => vec![location, target],
            Self::BranchCondition {
                location,
                condition,
            } => vec![location, condition],
        }
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
