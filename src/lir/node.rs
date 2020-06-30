use crate::error::Result;
use crate::expr::{Expression, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Node {
    /// A simple comment.
    Comment(String),
    /// Bind the expression to a variable.
    Let { var: Variable, expr: Expression },
    /// Assert that the condition is true.
    Assert { condition: Expression },
    /// Assume that the condition is true.
    Assume { condition: Expression },
}

impl Node {
    /// Create a new comment.
    pub fn comment<S>(text: S) -> Self
    where
        S: Into<String>,
    {
        Self::Comment(text.into())
    }

    /// Create a new variable binding.
    pub fn assign(var: Variable, expr: Expression) -> Result<Self> {
        expr.sort().expect_sort(var.sort())?;
        Ok(Self::Let { var, expr })
    }

    /// Create a new assertion.
    pub fn assert(condition: Expression) -> Result<Self> {
        condition.sort().expect_boolean()?;
        Ok(Self::Assert { condition })
    }

    /// Create a new assumption.
    pub fn assume(condition: Expression) -> Result<Self> {
        condition.sort().expect_boolean()?;
        Ok(Self::Assume { condition })
    }

    /// Returns whether this node is a comment.
    pub fn is_comment(&self) -> bool {
        match self {
            Self::Comment(..) => true,
            _ => false,
        }
    }

    /// Returns whether this node is a variable binding.
    pub fn is_let(&self) -> bool {
        match self {
            Self::Let { .. } => true,
            _ => false,
        }
    }

    /// Returns whether this node is an assertion.
    pub fn is_assert(&self) -> bool {
        match self {
            Self::Assert { .. } => true,
            _ => false,
        }
    }

    /// Returns whether this node is an assumption.
    pub fn is_assume(&self) -> bool {
        match self {
            Self::Assume { .. } => true,
            _ => false,
        }
    }

    /// Get each `Variable` used by this `Node`.
    pub fn variables_used(&self) -> Vec<&Variable> {
        match self {
            Self::Let { expr, .. } => expr.variables(),
            Self::Assert { condition } | Self::Assume { condition } => condition.variables(),
            Self::Comment(_) => Vec::new(),
        }
    }

    /// Get a mutable reference to each `Variable` used by this `Node`.
    pub fn variables_used_mut(&mut self) -> Vec<&mut Variable> {
        match self {
            Self::Let { expr, .. } => expr.variables_mut(),
            Self::Assert { condition } | Self::Assume { condition } => condition.variables_mut(),
            Self::Comment(_) => Vec::new(),
        }
    }

    /// Get a Vec of the `Variable`s defined by this `Node`
    pub fn variables_defined(&self) -> Vec<&Variable> {
        match self {
            Self::Let { var, .. } => vec![var],
            Self::Assert { .. } | Self::Assume { .. } | Self::Comment(_) => Vec::new(),
        }
    }

    /// Get a Vec of mutable reference to the `Variable`s defined by this `Node`
    pub fn variables_defined_mut(&mut self) -> Vec<&mut Variable> {
        match self {
            Self::Let { var, .. } => vec![var],
            Self::Assert { .. } | Self::Assume { .. } | Self::Comment(_) => Vec::new(),
        }
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Comment(text) => write!(f, "// {}", text),
            Self::Let { var, expr } => write!(f, "let {} = {}", var, expr),
            Self::Assert { condition } => write!(f, "assert {}", condition),
            Self::Assume { condition } => write!(f, "assume {}", condition),
        }
    }
}
