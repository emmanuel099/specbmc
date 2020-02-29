use crate::error::Result;
use crate::expr::{Expression, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Node {
    Comment(String),
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
}

impl Node {
    /// Create a new `Node::Comment`.
    pub fn comment<S>(text: S) -> Self
    where
        S: Into<String>,
    {
        Self::Comment(text.into())
    }

    /// Create a new `Node::Let`.
    pub fn assign(var: Variable, expr: Expression) -> Result<Self> {
        expr.sort().expect_sort(var.sort())?;
        Ok(Self::Let { var, expr })
    }

    /// Create a new `Node::Assert`.
    pub fn assert(condition: Expression) -> Result<Self> {
        condition.sort().expect_boolean()?;
        Ok(Self::Assert { condition })
    }

    /// Create a new `Node::Assume`.
    pub fn assume(condition: Expression) -> Result<Self> {
        condition.sort().expect_boolean()?;
        Ok(Self::Assume { condition })
    }

    pub fn is_comment(&self) -> bool {
        match self {
            Self::Comment(..) => true,
            _ => false,
        }
    }

    pub fn is_let(&self) -> bool {
        match self {
            Self::Let { .. } => true,
            _ => false,
        }
    }

    pub fn is_assert(&self) -> bool {
        match self {
            Self::Assert { .. } => true,
            _ => false,
        }
    }

    pub fn is_assume(&self) -> bool {
        match self {
            Self::Assume { .. } => true,
            _ => false,
        }
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Comment(text) => write!(f, "// {}", text),
            Self::Let { var, expr } => write!(f, "{} = {}", var, expr),
            Self::Assert { condition } => write!(f, "assert {}", condition),
            Self::Assume { condition } => write!(f, "assume {}", condition),
        }
    }
}
