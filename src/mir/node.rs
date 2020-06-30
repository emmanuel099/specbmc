use crate::error::Result;
use crate::expr::{Expression, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Node {
    // Bind the expression to a variable.
    Let {
        var: Variable,
        expr: Expression,
    },
    /// Assert that the condition is true in each composition.
    Assert {
        condition: Expression,
    },
    /// Assume that the condition is true in each composition.
    Assume {
        condition: Expression,
    },
    /// Assert that the condition is true.
    /// The condition may refer to variables from different compositions.
    HyperAssert {
        condition: Expression,
    },
    /// Assume that the condition is true.
    /// The condition may refer to variables from different compositions.
    HyperAssume {
        condition: Expression,
    },
}

impl Node {
    /// Create a new `Node::Let`.
    pub fn assign(var: Variable, expr: Expression) -> Result<Self> {
        expr.sort().expect_sort(var.sort())?;
        Ok(Self::Let { var, expr })
    }

    /// Create a new `Node::Assert`.
    pub fn assert(condition: Expression) -> Result<Self> {
        condition.sort().expect_boolean()?;

        if condition
            .variables()
            .iter()
            .any(|variable| variable.composition().is_some())
        {
            return Err(
                "Condition variables must not refer to a composition, use hyper_assert instead."
                    .into(),
            );
        }

        Ok(Self::Assert { condition })
    }

    /// Create a new `Node::Assume`.
    pub fn assume(condition: Expression) -> Result<Self> {
        condition.sort().expect_boolean()?;

        if condition
            .variables()
            .iter()
            .any(|variable| variable.composition().is_some())
        {
            return Err(
                "Condition variables must not refer to a composition, use hyper_assume instead."
                    .into(),
            );
        }

        Ok(Self::Assume { condition })
    }

    /// Create a new `Node::HyperAssert`.
    pub fn hyper_assert(condition: Expression) -> Result<Self> {
        condition.sort().expect_boolean()?;

        if condition
            .variables()
            .iter()
            .any(|variable| variable.composition().is_none())
        {
            return Err("All condition variables must refer to a composition.".into());
        }

        Ok(Self::HyperAssert { condition })
    }

    /// Create a new `Node::HyperAssume`.
    pub fn hyper_assume(condition: Expression) -> Result<Self> {
        condition.sort().expect_boolean()?;

        if condition
            .variables()
            .iter()
            .any(|variable| variable.composition().is_none())
        {
            return Err("All condition variables must refer to a composition.".into());
        }

        Ok(Self::HyperAssume { condition })
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

    /// Returns whether this node is a hyper-assertion.
    pub fn is_hyper_assert(&self) -> bool {
        match self {
            Self::HyperAssert { .. } => true,
            _ => false,
        }
    }

    /// Returns whether this node is a hyper-assumption.
    pub fn is_hyper_assume(&self) -> bool {
        match self {
            Self::HyperAssume { .. } => true,
            _ => false,
        }
    }

    /// Get each `Variable` used by this `Node`.
    pub fn variables_used(&self) -> Vec<&Variable> {
        match self {
            Self::Let { expr, .. } => expr.variables(),
            Self::Assert { condition }
            | Self::Assume { condition }
            | Self::HyperAssert { condition }
            | Self::HyperAssume { condition } => condition.variables(),
        }
    }

    /// Get a mutable reference to each `Variable` used by this `Node`.
    pub fn variables_used_mut(&mut self) -> Vec<&mut Variable> {
        match self {
            Self::Let { expr, .. } => expr.variables_mut(),
            Self::Assert { condition }
            | Self::Assume { condition }
            | Self::HyperAssert { condition }
            | Self::HyperAssume { condition } => condition.variables_mut(),
        }
    }

    /// Get a Vec of the `Variable`s defined by this `Node`
    pub fn variables_defined(&self) -> Vec<&Variable> {
        match self {
            Self::Let { var, .. } => vec![var],
            Self::Assert { .. }
            | Self::Assume { .. }
            | Self::HyperAssert { .. }
            | Self::HyperAssume { .. } => Vec::new(),
        }
    }

    /// Get a Vec of mutable reference to the `Variable`s defined by this `Node`
    pub fn variables_defined_mut(&mut self) -> Vec<&mut Variable> {
        match self {
            Self::Let { var, .. } => vec![var],
            Self::Assert { .. }
            | Self::Assume { .. }
            | Self::HyperAssert { .. }
            | Self::HyperAssume { .. } => Vec::new(),
        }
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Let { var, expr } => write!(f, "let {} = {}", var, expr),
            Self::Assert { condition } => write!(f, "assert {}", condition),
            Self::Assume { condition } => write!(f, "assume {}", condition),
            Self::HyperAssert { condition } => write!(f, "hyper-assert {}", condition),
            Self::HyperAssume { condition } => write!(f, "hyper-assume {}", condition),
        }
    }
}
