use crate::error::Result;
use crate::expr::{Expression, Memory, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Operation {
    /// Assign the value given in expression to the variable indicated.
    Assign {
        variable: Variable,
        expr: Expression,
    },
    /// Store the value in expr at the address given in index.
    Store {
        address: Expression,
        expr: Expression,
        memory_in: Variable,
        memory_out: Variable,
    },
    /// Load the value in memory at index and place the result in the variable.
    Load {
        variable: Variable,
        address: Expression,
        memory: Variable,
    },
    /// Call the function given by target.
    Call { target: Expression },
    /// Branch to the value given by target.
    Branch { target: Expression },
    /// Branch to the value given by target if the condition holds.
    ConditionalBranch {
        condition: Expression,
        target: Expression,
    },
    /// Does nothing aka. no operation.
    Skip,
    /// Speculation Barrier
    Barrier,
    /// Assert that the condition is true.
    Assert { condition: Expression },
    /// Assume that the condition is true.
    Assume { condition: Expression },
    /// The expression is observable by an adversary.
    Observable { expr: Expression },
    /// The expression is indistinguishable to an adversary.
    Indistinguishable { expr: Expression },
}

impl Operation {
    /// Create a new `Operation::Assign`.
    pub fn assign(variable: Variable, expr: Expression) -> Result<Self> {
        expr.sort().expect_sort(variable.sort())?;
        Ok(Self::Assign { variable, expr })
    }

    /// Create a new `Operation::Store`.
    pub fn store(address: Expression, expr: Expression) -> Result<Self> {
        address.sort().expect_word()?;
        expr.sort().expect_bit_vector()?;
        Ok(Self::Store {
            memory_in: Memory::variable(),
            memory_out: Memory::variable(),
            address,
            expr,
        })
    }

    /// Create a new `Operation::Load`.
    pub fn load(variable: Variable, address: Expression) -> Result<Self> {
        address.sort().expect_word()?;
        variable.sort().expect_bit_vector()?;
        Ok(Self::Load {
            memory: Memory::variable(),
            variable,
            address,
        })
    }

    /// Create a new `Operation::Call`.
    pub fn call(target: Expression) -> Result<Self> {
        target.sort().expect_word()?;
        Ok(Self::Call { target })
    }

    /// Create a new `Operation::Branch`.
    pub fn branch(target: Expression) -> Result<Self> {
        target.sort().expect_word()?;
        Ok(Self::Branch { target })
    }

    /// Create a new `Operation::ConditionalBranch`.
    pub fn conditional_branch(condition: Expression, target: Expression) -> Result<Self> {
        condition.sort().expect_boolean()?;
        target.sort().expect_word()?;
        Ok(Self::ConditionalBranch { condition, target })
    }

    /// Create a new `Operation::Skip`
    pub fn skip() -> Self {
        Self::Skip
    }

    /// Create a new `Operation::Barrier`
    pub fn barrier() -> Self {
        Self::Barrier
    }

    /// Create a new `Operation::Assert`.
    pub fn assert(condition: Expression) -> Result<Self> {
        condition.sort().expect_boolean()?;
        Ok(Self::Assert { condition })
    }

    /// Create a new `Operation::Assume`.
    pub fn assume(condition: Expression) -> Result<Self> {
        condition.sort().expect_boolean()?;
        Ok(Self::Assume { condition })
    }

    /// Create a new `Operation::Observable`
    pub fn observable(expr: Expression) -> Self {
        Self::Observable { expr }
    }

    /// Create a new `Operation::Indistinguishable`
    pub fn indistinguishable(expr: Expression) -> Self {
        Self::Indistinguishable { expr }
    }

    pub fn is_assign(&self) -> bool {
        matches!(self, Self::Assign { .. })
    }

    pub fn is_store(&self) -> bool {
        matches!(self, Self::Store { .. } )
    }

    pub fn is_load(&self) -> bool {
        matches!(self, Self::Load { .. } )
    }

    pub fn is_call(&self) -> bool {
        matches!(self, Self::Call { .. } )
    }

    pub fn is_branch(&self) -> bool {
        matches!(self,  Self::Branch { .. } )
    }

    pub fn is_conditional_branch(&self) -> bool {
        matches!(self,  Self::ConditionalBranch { .. } )
    }

    pub fn is_skip(&self) -> bool {
        matches!(self, Self::Skip)
    }

    pub fn is_barrier(&self) -> bool {
        matches!(self, Self::Barrier)
    }

    pub fn is_assert(&self) -> bool {
        matches!(self,  Self::Assert { .. })
    }

    pub fn is_assume(&self) -> bool {
        matches!(self, Self::Assume { .. })
    }

    pub fn is_observable(&self) -> bool {
        matches!(self, Self::Observable { .. } )
    }

    pub fn is_indistinguishable(&self) -> bool {
        matches!(self,  Self::Indistinguishable { .. } )
    }

    /// Get each `Variable` read by this `Operation`.
    pub fn variables_read(&self) -> Vec<&Variable> {
        match self {
            Self::Assign { expr, .. }
            | Self::Observable { expr }
            | Self::Indistinguishable { expr } => expr.variables(),
            Self::Store {
                memory_in,
                address,
                expr,
                ..
            } => vec![memory_in]
                .into_iter()
                .chain(address.variables().into_iter())
                .chain(expr.variables().into_iter())
                .collect(),
            Self::Load {
                memory, address, ..
            } => vec![memory]
                .into_iter()
                .chain(address.variables().into_iter())
                .collect(),
            Self::Call { target } | Self::Branch { target } => target.variables(),
            Self::ConditionalBranch { condition, target } => condition
                .variables()
                .into_iter()
                .chain(target.variables().into_iter())
                .collect(),
            Self::Assert { condition } | Self::Assume { condition } => condition.variables(),
            Self::Skip | Self::Barrier => Vec::new(),
        }
    }

    /// Get a mutable reference to each `Variable` read by this `Operation`.
    pub fn variables_read_mut(&mut self) -> Vec<&mut Variable> {
        match self {
            Self::Assign { expr, .. }
            | Self::Observable { expr }
            | Self::Indistinguishable { expr } => expr.variables_mut(),
            Self::Store {
                memory_in,
                address,
                expr,
                ..
            } => vec![memory_in]
                .into_iter()
                .chain(address.variables_mut().into_iter())
                .chain(expr.variables_mut().into_iter())
                .collect(),
            Self::Load {
                memory, address, ..
            } => vec![memory]
                .into_iter()
                .chain(address.variables_mut().into_iter())
                .collect(),
            Self::Call { target } | Self::Branch { target } => target.variables_mut(),
            Self::ConditionalBranch { condition, target } => condition
                .variables_mut()
                .into_iter()
                .chain(target.variables_mut().into_iter())
                .collect(),
            Self::Assert { condition } | Self::Assume { condition } => condition.variables_mut(),
            Self::Skip | Self::Barrier => Vec::new(),
        }
    }

    /// Get each `Variable` written by this `Operation`.
    pub fn variables_written(&self) -> Vec<&Variable> {
        match self {
            Self::Assign { variable, .. } | Self::Load { variable, .. } => vec![variable],
            Self::Store { memory_out, .. } => vec![memory_out],
            Self::Call { .. }
            | Self::Branch { .. }
            | Self::ConditionalBranch { .. }
            | Self::Skip
            | Self::Barrier
            | Self::Assert { .. }
            | Self::Assume { .. }
            | Self::Observable { .. }
            | Self::Indistinguishable { .. } => Vec::new(),
        }
    }

    /// Get a mutable reference to each `Variable` written by this `Operation`.
    pub fn variables_written_mut(&mut self) -> Vec<&mut Variable> {
        match self {
            Self::Assign { variable, .. } | Self::Load { variable, .. } => vec![variable],
            Self::Store { memory_out, .. } => vec![memory_out],
            Self::Call { .. }
            | Self::Branch { .. }
            | Self::ConditionalBranch { .. }
            | Self::Skip
            | Self::Barrier
            | Self::Assert { .. }
            | Self::Assume { .. }
            | Self::Observable { .. }
            | Self::Indistinguishable { .. } => Vec::new(),
        }
    }

    /// Get each `Variable` used by this `Operation`.
    pub fn variables(&self) -> Vec<&Variable> {
        self.variables_read()
            .into_iter()
            .chain(self.variables_written().into_iter())
            .collect()
    }

    /// Get each `Expression` of this `Operation`.
    pub fn expressions(&self) -> Vec<&Expression> {
        match self {
            Self::Assign { expr, .. }
            | Self::Observable { expr }
            | Self::Indistinguishable { expr } => vec![expr],
            Self::Store { address, expr, .. } => vec![address, expr],
            Self::Load { address, .. } => vec![address],
            Self::Call { target } | Self::Branch { target } => vec![target],
            Self::ConditionalBranch { condition, target } => vec![condition, target],
            Self::Assert { condition } | Self::Assume { condition } => vec![condition],
            Self::Skip | Self::Barrier => Vec::new(),
        }
    }

    /// Get a mutable reference to each `Expression` of this `Operation`.
    pub fn expressions_mut(&mut self) -> Vec<&mut Expression> {
        match self {
            Self::Assign { expr, .. }
            | Self::Observable { expr }
            | Self::Indistinguishable { expr } => vec![expr],
            Self::Store { address, expr, .. } => vec![address, expr],
            Self::Load { address, .. } => vec![address],
            Self::Call { target } | Self::Branch { target } => vec![target],
            Self::ConditionalBranch { condition, target } => vec![condition, target],
            Self::Assert { condition } | Self::Assume { condition } => vec![condition],
            Self::Skip | Self::Barrier => Vec::new(),
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Assign { variable, expr } => write!(f, "{} = {}", variable, expr),
            Self::Store {
                address,
                expr,
                memory_in,
                memory_out,
            } => write!(
                f,
                "{} = store({}, {}, {})",
                memory_out, memory_in, address, expr
            ),
            Self::Load {
                variable,
                address,
                memory,
            } => write!(f, "{} = load({}, {})", variable, memory, address),
            Self::Call { target } => write!(f, "call {}", target),
            Self::Branch { target } => write!(f, "branch {}", target),
            Self::ConditionalBranch { condition, target } => {
                write!(f, "branch {} if {}", target, condition)
            }
            Self::Assert { condition } => write!(f, "assert {}", condition),
            Self::Assume { condition } => write!(f, "assume {}", condition),
            Self::Skip => write!(f, "skip"),
            Self::Barrier => write!(f, "barrier"),
            Self::Observable { expr } => write!(f, "observable({})", expr),
            Self::Indistinguishable { expr } => write!(f, "indistinguishable({})", expr),
        }
    }
}
