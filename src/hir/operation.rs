use crate::expr::{Expression, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Operation {
    /// Assign the value given in expression to the variable indicated.
    Assign {
        variable: Variable,
        expr: Expression,
    },
    /// Store the value in src at the address given in index.
    Store {
        address: Expression,
        expr: Expression,
    },
    /// Load the value in memory at index and place the result in the variable dst.
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
    /// The listed variables are observable to an adversary.
    Observable { variables: Vec<Variable> },
    /// The listed variables are indistinguishable for an adversary.
    Indistinguishable { variables: Vec<Variable> },
    /// Parallel operation, meaning that the nested operations happen in parallel.
    Parallel(Vec<Operation>),
}

impl Operation {
    /// Create a new `Operation::Assign`.
    pub fn assign(variable: Variable, expr: Expression) -> Operation {
        Operation::Assign { variable, expr }
    }

    /// Create a new `Operation::Store`.
    pub fn store(address: Expression, expr: Expression) -> Operation {
        Operation::Store { address, expr }
    }

    /// Create a new `Operation::Load`.
    pub fn load(variable: Variable, address: Expression) -> Operation {
        Operation::Load { variable, address }
    }

    /// Create a new `Operation::Branch`.
    pub fn branch(target: Expression) -> Operation {
        Operation::Branch { target }
    }

    /// Create a new `Operation::ConditionalBranch`.
    pub fn conditional_branch(condition: Expression, target: Expression) -> Operation {
        Operation::ConditionalBranch { condition, target }
    }

    /// Create a new `Operation::Barrier`
    pub fn barrier() -> Operation {
        Operation::Barrier
    }

    /// Create a new `Operation::Observable`
    pub fn observable(variables: Vec<Variable>) -> Operation {
        Operation::Observable { variables }
    }

    /// Create a new `Operation::Indistinguishable`
    pub fn indistinguishable(variables: Vec<Variable>) -> Operation {
        Operation::Indistinguishable { variables }
    }

    pub fn parallel(operations: Vec<Operation>) -> Operation {
        Operation::Parallel(operations)
    }

    pub fn is_assign(&self) -> bool {
        match self {
            Operation::Assign { .. } => true,
            _ => false,
        }
    }

    pub fn is_store(&self) -> bool {
        match self {
            Operation::Store { .. } => true,
            _ => false,
        }
    }

    pub fn is_load(&self) -> bool {
        match self {
            Operation::Load { .. } => true,
            _ => false,
        }
    }

    pub fn is_branch(&self) -> bool {
        match self {
            Operation::Branch { .. } => true,
            _ => false,
        }
    }

    pub fn is_conditional_branch(&self) -> bool {
        match self {
            Operation::ConditionalBranch { .. } => true,
            _ => false,
        }
    }

    pub fn is_barrier(&self) -> bool {
        match self {
            Operation::Barrier => true,
            _ => false,
        }
    }

    pub fn is_observable(&self) -> bool {
        match self {
            Operation::Observable { .. } => true,
            _ => false,
        }
    }

    pub fn is_indistinguishable(&self) -> bool {
        match self {
            Operation::Indistinguishable { .. } => true,
            _ => false,
        }
    }

    pub fn is_parallel(&self) -> bool {
        match self {
            Operation::Parallel(_) => true,
            _ => false,
        }
    }

    /// Get each `Variable` read by this `Operation`.
    pub fn variables_read(&self) -> Vec<&Variable> {
        match self {
            Operation::Assign { expr, .. } => expr.variables(),
            Operation::Store { address, expr, .. } => address
                .variables()
                .into_iter()
                .chain(expr.variables().into_iter())
                .collect(),
            Operation::Load { address, .. } => address.variables(),
            Operation::Branch { target } => target.variables(),
            Operation::ConditionalBranch { condition, target } => condition
                .variables()
                .into_iter()
                .chain(target.variables().into_iter())
                .collect(),
            Operation::Barrier => Vec::new(),
            Operation::Observable { variables } | Operation::Indistinguishable { variables } => {
                variables.iter().collect()
            }
            Operation::Parallel(operations) => operations
                .iter()
                .flat_map(|op| op.variables_read())
                .collect(),
        }
    }

    /// Get a mutable reference to each `Variable` read by this `Operation`.
    pub fn variables_read_mut(&mut self) -> Vec<&mut Variable> {
        match self {
            Operation::Assign { expr, .. } => expr.variables_mut(),
            Operation::Store { address, expr, .. } => address
                .variables_mut()
                .into_iter()
                .chain(expr.variables_mut().into_iter())
                .collect(),
            Operation::Load { address, .. } => address.variables_mut(),
            Operation::Branch { target } => target.variables_mut(),
            Operation::ConditionalBranch { condition, target } => condition
                .variables_mut()
                .into_iter()
                .chain(target.variables_mut().into_iter())
                .collect(),
            Operation::Barrier => Vec::new(),
            Operation::Observable { variables } | Operation::Indistinguishable { variables } => {
                variables.iter_mut().collect()
            }
            Operation::Parallel(operations) => operations
                .iter_mut()
                .flat_map(|op| op.variables_read_mut())
                .collect(),
        }
    }

    /// Get a Vec of the `Variable`s written by this `Operation`
    pub fn variables_written(&self) -> Vec<&Variable> {
        match self {
            Operation::Assign { variable, .. } | Operation::Load { variable, .. } => vec![variable],
            Operation::Store { .. }
            | Operation::Branch { .. }
            | Operation::ConditionalBranch { .. }
            | Operation::Barrier
            | Operation::Observable { .. }
            | Operation::Indistinguishable { .. } => Vec::new(),
            Operation::Parallel(operations) => operations
                .iter()
                .flat_map(|op| op.variables_written())
                .collect(),
        }
    }

    /// Get a Vec of mutable referencer to the `Variable`s written by this `Operation`
    pub fn variables_written_mut(&mut self) -> Vec<&mut Variable> {
        match self {
            Operation::Assign { variable, .. } | Operation::Load { variable, .. } => vec![variable],
            Operation::Store { .. }
            | Operation::Branch { .. }
            | Operation::ConditionalBranch { .. }
            | Operation::Barrier
            | Operation::Observable { .. }
            | Operation::Indistinguishable { .. } => Vec::new(),
            Operation::Parallel(operations) => operations
                .iter_mut()
                .flat_map(|op| op.variables_written_mut())
                .collect(),
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Operation::Assign { variable, expr } => write!(f, "{} = {}", variable, expr),
            Operation::Store { address, expr } => write!(f, "store({}, {})", address, expr),
            Operation::Load { variable, address } => write!(f, "{} = load({})", variable, address),
            Operation::Branch { target } => write!(f, "branch {}", target),
            Operation::ConditionalBranch { condition, target } => {
                write!(f, "branch {} if {}", target, condition)
            }
            Operation::Barrier => write!(f, "barrier"),
            Operation::Observable { variables } => {
                write!(f, "observable(")?;
                for var in variables {
                    write!(f, "{}, ", var)?;
                }
                write!(f, ")")
            }
            Operation::Indistinguishable { variables } => {
                write!(f, "indistinguishable(")?;
                for var in variables {
                    write!(f, "{}, ", var)?;
                }
                write!(f, ")")
            }
            Operation::Parallel(operations) => {
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
