use crate::error::Result;
use crate::expr::{Expression, Variable};
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
    },
    /// Load the value in memory at index and place the result in the variable.
    Load {
        variable: Variable,
        address: Expression,
    },
    /// Branch to the value given by target.
    Branch { target: Expression },
    /// Branch to the value given by target if the condition holds.
    ConditionalBranch {
        condition: Expression,
        target: Expression,
    },
    /// Speculation Barrier
    Barrier,
    /// Assert that the condition is true.
    Assert { condition: Expression },
    /// Assume that the condition is true.
    Assume { condition: Expression },
    /// The listed variables are observable to an adversary.
    Observable { variables: Vec<Variable> },
    /// The listed variables are indistinguishable for an adversary.
    Indistinguishable { variables: Vec<Variable> },
    /// The nested operations happen in parallel.
    Parallel(Vec<Operation>),
}

impl Operation {
    /// Create a new `Operation::Assign`.
    pub fn assign(variable: Variable, expr: Expression) -> Result<Self> {
        expr.sort().expect_sort(variable.sort())?;
        Ok(Self::Assign { variable, expr })
    }

    /// Create a new `Operation::Store`.
    pub fn store(address: Expression, expr: Expression) -> Result<Self> {
        address.sort().expect_bit_vector()?;
        expr.sort().expect_bit_vector()?;
        Ok(Self::Store { address, expr })
    }

    /// Create a new `Operation::Load`.
    pub fn load(variable: Variable, address: Expression) -> Result<Self> {
        address.sort().expect_bit_vector()?;
        variable.sort().expect_bit_vector()?;
        Ok(Self::Load { variable, address })
    }

    /// Create a new `Operation::Branch`.
    pub fn branch(target: Expression) -> Result<Self> {
        target.sort().expect_bit_vector()?;
        Ok(Self::Branch { target })
    }

    /// Create a new `Operation::ConditionalBranch`.
    pub fn conditional_branch(condition: Expression, target: Expression) -> Result<Self> {
        condition.sort().expect_boolean()?;
        target.sort().expect_bit_vector()?;
        Ok(Self::ConditionalBranch { condition, target })
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
    pub fn observable(variables: Vec<Variable>) -> Self {
        Self::Observable { variables }
    }

    /// Create a new `Operation::Indistinguishable`
    pub fn indistinguishable(variables: Vec<Variable>) -> Self {
        Self::Indistinguishable { variables }
    }

    /// Create a new `Operation::Parallel`
    pub fn parallel(operations: Vec<Operation>) -> Self {
        Self::Parallel(operations)
    }

    /// Get each `Variable` read by this `Operation`.
    pub fn variables_read(&self) -> Vec<&Variable> {
        match self {
            Self::Assign { expr, .. } => expr.variables(),
            Self::Store { address, expr, .. } => address
                .variables()
                .into_iter()
                .chain(expr.variables().into_iter())
                .collect(),
            Self::Load { address, .. } => address.variables(),
            Self::Branch { target } => target.variables(),
            Self::ConditionalBranch { condition, target } => condition
                .variables()
                .into_iter()
                .chain(target.variables().into_iter())
                .collect(),
            Self::Assert { condition } | Self::Assume { condition } => condition.variables(),
            Self::Barrier => Vec::new(),
            Self::Observable { variables } | Self::Indistinguishable { variables } => {
                variables.iter().collect()
            }
            Self::Parallel(operations) => operations
                .iter()
                .flat_map(|op| op.variables_read())
                .collect(),
        }
    }

    /// Get a mutable reference to each `Variable` read by this `Operation`.
    pub fn variables_read_mut(&mut self) -> Vec<&mut Variable> {
        match self {
            Self::Assign { expr, .. } => expr.variables_mut(),
            Self::Store { address, expr, .. } => address
                .variables_mut()
                .into_iter()
                .chain(expr.variables_mut().into_iter())
                .collect(),
            Self::Load { address, .. } => address.variables_mut(),
            Self::Branch { target } => target.variables_mut(),
            Self::ConditionalBranch { condition, target } => condition
                .variables_mut()
                .into_iter()
                .chain(target.variables_mut().into_iter())
                .collect(),
            Self::Assert { condition } | Self::Assume { condition } => condition.variables_mut(),
            Self::Barrier => Vec::new(),
            Self::Observable { variables } | Self::Indistinguishable { variables } => {
                variables.iter_mut().collect()
            }
            Self::Parallel(operations) => operations
                .iter_mut()
                .flat_map(|op| op.variables_read_mut())
                .collect(),
        }
    }

    /// Get a Vec of the `Variable`s written by this `Operation`
    pub fn variables_written(&self) -> Vec<&Variable> {
        match self {
            Self::Assign { variable, .. } | Self::Load { variable, .. } => vec![variable],
            Self::Store { .. }
            | Self::Branch { .. }
            | Self::ConditionalBranch { .. }
            | Self::Barrier
            | Self::Assert { .. }
            | Self::Assume { .. }
            | Self::Observable { .. }
            | Self::Indistinguishable { .. } => Vec::new(),
            Self::Parallel(operations) => operations
                .iter()
                .flat_map(|op| op.variables_written())
                .collect(),
        }
    }

    /// Get a Vec of mutable referencer to the `Variable`s written by this `Operation`
    pub fn variables_written_mut(&mut self) -> Vec<&mut Variable> {
        match self {
            Self::Assign { variable, .. } | Self::Load { variable, .. } => vec![variable],
            Self::Store { .. }
            | Self::Branch { .. }
            | Self::ConditionalBranch { .. }
            | Self::Barrier
            | Self::Assert { .. }
            | Self::Assume { .. }
            | Self::Observable { .. }
            | Self::Indistinguishable { .. } => Vec::new(),
            Self::Parallel(operations) => operations
                .iter_mut()
                .flat_map(|op| op.variables_written_mut())
                .collect(),
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Assign { variable, expr } => write!(f, "{} = {}", variable, expr),
            Self::Store { address, expr } => write!(f, "store({}, {})", address, expr),
            Self::Load { variable, address } => write!(f, "{} = load({})", variable, address),
            Self::Branch { target } => write!(f, "branch {}", target),
            Self::ConditionalBranch { condition, target } => {
                write!(f, "branch {} if {}", target, condition)
            }
            Self::Assert { condition } => write!(f, "assert {}", condition),
            Self::Assume { condition } => write!(f, "assume {}", condition),
            Self::Barrier => write!(f, "barrier"),
            Self::Observable { variables } => {
                write!(f, "observable(")?;
                if !variables.is_empty() {
                    write!(f, "{}", variables.first().unwrap())?;
                    for var in variables.iter().skip(1) {
                        write!(f, ", {}", var)?;
                    }
                }
                write!(f, ")")
            }
            Self::Indistinguishable { variables } => {
                write!(f, "indistinguishable(")?;
                if !variables.is_empty() {
                    write!(f, "{}", variables.first().unwrap())?;
                    for var in variables.iter().skip(1) {
                        write!(f, ", {}", var)?;
                    }
                }
                write!(f, ")")
            }
            Self::Parallel(operations) => {
                if !operations.is_empty() {
                    write!(f, "{}", operations.first().unwrap())?;
                    for operation in operations.iter().skip(1) {
                        write!(f, " || {}", operation)?;
                    }
                }
                Ok(())
            }
        }
    }
}
