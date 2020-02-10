use crate::lir::{Expression, Variable};
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
        new_memory: Variable,
        memory: Variable,
        address: Expression,
        expr: Expression,
    },
    /// Load the value in memory at index and place the result in the variable dst.
    Load {
        variable: Variable,
        memory: Variable,
        address: Expression,
    },
    /// Branch to the value given by target.
    Branch { target: Expression },
    /// Speculation Barrier
    Barrier,
}

impl Operation {
    /// Create a new `Operation::Assign`.
    pub fn assign(variable: Variable, expr: Expression) -> Operation {
        Operation::Assign { variable, expr }
    }

    /// Create a new `Operation::Store`.
    pub fn store(memory: Variable, address: Expression, expr: Expression) -> Operation {
        Operation::Store {
            new_memory: memory.clone(),
            memory,
            address,
            expr,
        }
    }

    /// Create a new `Operation::Load`.
    pub fn load(variable: Variable, memory: Variable, address: Expression) -> Operation {
        Operation::Load {
            variable,
            memory,
            address,
        }
    }

    /// Create a new `Operation::Branch`.
    pub fn branch(target: Expression) -> Operation {
        Operation::Branch { target }
    }

    /// Create a new `Operation::Barrier`
    pub fn barrier() -> Operation {
        Operation::Barrier
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

    pub fn is_barrier(&self) -> bool {
        match self {
            Operation::Barrier => true,
            _ => false,
        }
    }

    /// Get each `Variable` read by this `Operation`.
    pub fn variables_read(&self) -> Option<Vec<&Variable>> {
        match self {
            Operation::Assign { expr, .. } => Some(expr.variables()),
            Operation::Store {
                memory,
                address,
                expr,
                ..
            } => Some(
                vec![memory]
                    .into_iter()
                    .chain(address.variables().into_iter())
                    .chain(expr.variables().into_iter())
                    .collect(),
            ),
            Operation::Load {
                memory, address, ..
            } => Some(
                vec![memory]
                    .into_iter()
                    .chain(address.variables().into_iter())
                    .collect(),
            ),
            Operation::Branch { target } => Some(target.variables()),
            Operation::Barrier => Some(Vec::new()),
        }
    }

    /// Get a mutable reference to each `Variable` read by this `Operation`.
    pub fn variables_read_mut(&mut self) -> Option<Vec<&mut Variable>> {
        match self {
            Operation::Assign { expr, .. } => Some(expr.variables_mut()),
            Operation::Store {
                memory,
                address,
                expr,
                ..
            } => Some(
                vec![memory]
                    .into_iter()
                    .chain(address.variables_mut().into_iter())
                    .chain(expr.variables_mut().into_iter())
                    .collect(),
            ),
            Operation::Load {
                memory, address, ..
            } => Some(
                vec![memory]
                    .into_iter()
                    .chain(address.variables_mut().into_iter())
                    .collect(),
            ),
            Operation::Branch { target } => Some(target.variables_mut()),
            Operation::Barrier => Some(Vec::new()),
        }
    }

    /// Get a Vec of the `Variable`s written by this `Operation`
    pub fn variables_written(&self) -> Option<Vec<&Variable>> {
        match self {
            Operation::Assign { variable, .. } | Operation::Load { variable, .. } => {
                Some(vec![variable])
            }
            Operation::Store { new_memory, .. } => Some(vec![new_memory]),
            Operation::Branch { .. } | Operation::Barrier => Some(Vec::new()),
        }
    }

    /// Get a Vec of mutable referencer to the `Variable`s written by this `Operation`
    pub fn variables_written_mut(&mut self) -> Option<Vec<&mut Variable>> {
        match self {
            Operation::Assign { variable, .. } | Operation::Load { variable, .. } => {
                Some(vec![variable])
            }
            Operation::Store { new_memory, .. } => Some(vec![new_memory]),
            Operation::Branch { .. } | Operation::Barrier => Some(Vec::new()),
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Operation::Assign { variable, expr } => write!(f, "{} = {}", variable, expr),
            Operation::Store {
                new_memory,
                memory,
                address,
                expr,
            } => write!(
                f,
                "{} = store({}, {}, {})",
                new_memory, memory, address, expr
            ),
            Operation::Load {
                variable,
                memory,
                address,
            } => write!(f, "{} = load({}, {})", variable, memory, address),
            Operation::Branch { target } => write!(f, "branch {}", target),
            Operation::Barrier => write!(f, "barrier"),
        }
    }
}
