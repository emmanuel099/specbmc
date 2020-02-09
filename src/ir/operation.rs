use crate::error::Result;
use crate::ir::{Expression, Variable};
use std::fmt;

#[derive(Clone, Debug)]
pub enum Operation {
    // Bind the expression to a variable.
    Let {
        var: Variable,
        expr: Expression,
    },
    /// Assert that the condition is true.
    Assert {
        cond: Expression,
    },
    /// Assume that the condition is true.
    Assume {
        cond: Expression,
    },
}

impl Operation {
    /// Create a new `Operation::Let`.
    pub fn new_let(var: Variable, expr: Expression) -> Result<Self> {
        expr.sort().expect_sort(var.sort())?;
        Ok(Self::Let { var, expr })
    }

    /// Create a new `Operation::Assert`.
    pub fn new_assert(cond: Expression) -> Result<Self> {
        cond.sort().expect_bool()?;
        Ok(Self::Assert { cond })
    }

    /// Create a new `Operation::Assume`.
    pub fn new_assume(cond: Expression) -> Result<Self> {
        cond.sort().expect_bool()?;
        Ok(Self::Assume { cond })
    }

    pub fn is_let(&self) -> bool {
        match self {
            Self::Let { .. } => true,
            _ => false,
        }
    }

    pub fn is_assert(&self) -> bool {
        match self {
            Operation::Assert { .. } => true,
            _ => false,
        }
    }

    pub fn is_assume(&self) -> bool {
        match self {
            Operation::Assume { .. } => true,
            _ => false,
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Let { var, expr } => write!(f, "let {} = {}", var, expr),
            Self::Assert { cond } => write!(f, "assert {}", cond),
            Self::Assume { cond } => write!(f, "assume {}", cond),
        }
    }
}
