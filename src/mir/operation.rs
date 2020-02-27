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
        cond: Expression,
    },
    /// Assume that the condition is true.
    Assume {
        cond: Expression,
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
    pub fn new_let(var: Variable, expr: Expression) -> Result<Self> {
        expr.sort().expect_sort(var.sort())?;
        Ok(Self::Let { var, expr })
    }

    /// Create a new `Operation::Assert`.
    pub fn new_assert(cond: Expression) -> Result<Self> {
        cond.sort().expect_boolean()?;
        Ok(Self::Assert { cond })
    }

    /// Create a new `Operation::Assume`.
    pub fn new_assume(cond: Expression) -> Result<Self> {
        cond.sort().expect_boolean()?;
        Ok(Self::Assume { cond })
    }

    /// Create a new `Operation::SelfCompAssertEqual`.
    ///
    /// Asserts that `expr` is equal in all self-compositions given by `compositions`.
    pub fn new_assert_equal_in_self_composition(
        compositions: Vec<usize>,
        expr: Expression,
    ) -> Self {
        Self::SelfCompAssertEqual { compositions, expr }
    }

    /// Create a new `Operation::SelfCompAssumeEqual`.
    ///
    /// Assumes that `expr` is equal in all self-compositions given by `compositions`.
    pub fn new_assume_equal_in_self_composition(
        compositions: Vec<usize>,
        expr: Expression,
    ) -> Self {
        Self::SelfCompAssumeEqual { compositions, expr }
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
            Self::Let { var, expr } => write!(f, "{} = {}", var, expr),
            Self::Assert { cond } => write!(f, "assert {}", cond),
            Self::Assume { cond } => write!(f, "assume {}", cond),
            Self::SelfCompAssertEqual { compositions, expr } => {
                write!(f, "sc-assert-eq {} @ {:?}", expr, compositions)
            }
            Self::SelfCompAssumeEqual { compositions, expr } => {
                write!(f, "sc-assume-eq {} @ {:?}", expr, compositions)
            }
        }
    }
}
