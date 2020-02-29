use crate::error::Result;
use crate::expr::{Expression, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Operation {
    // Bind the expression to a variable.
    Let {
        var: Variable,
        expr: Expression,
    },
    /// Assert that the condition is true.
    Assert {
        condition: Expression,
    },
    /// Assume that the condition is true.
    Assume {
        condition: Expression,
    },
    /// Assert equality of `expr` in self-compositions.
    SelfCompAssertEqual {
        compositions: Vec<usize>,
        expr: Expression,
    },
    /// Assume equality of `expr` in self-compositions.
    SelfCompAssumeEqual {
        compositions: Vec<usize>,
        expr: Expression,
    },
}

impl Operation {
    /// Create a new `Operation::Let`.
    pub fn assign(var: Variable, expr: Expression) -> Result<Self> {
        expr.sort().expect_sort(var.sort())?;
        Ok(Self::Let { var, expr })
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

    /// Create a new `Operation::SelfCompAssertEqual`.
    ///
    /// Asserts that `expr` is equal in all self-compositions given by `compositions`.
    pub fn assert_equal_in_self_composition(compositions: Vec<usize>, expr: Expression) -> Self {
        Self::SelfCompAssertEqual { compositions, expr }
    }

    /// Create a new `Operation::SelfCompAssumeEqual`.
    ///
    /// Assumes that `expr` is equal in all self-compositions given by `compositions`.
    pub fn assume_equal_in_self_composition(compositions: Vec<usize>, expr: Expression) -> Self {
        Self::SelfCompAssumeEqual { compositions, expr }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Let { var, expr } => write!(f, "{} = {}", var, expr),
            Self::Assert { condition } => write!(f, "assert {}", condition),
            Self::Assume { condition } => write!(f, "assume {}", condition),
            Self::SelfCompAssertEqual { compositions, expr } => {
                write!(f, "sc-assert-eq {} @ {:?}", expr, compositions)
            }
            Self::SelfCompAssumeEqual { compositions, expr } => {
                write!(f, "sc-assume-eq {} @ {:?}", expr, compositions)
            }
        }
    }
}
