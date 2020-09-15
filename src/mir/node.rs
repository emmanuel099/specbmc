use crate::error::Result;
use crate::expr::{Expression, Variable};
use std::fmt;

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Node {
    /// A simple comment.
    Comment(String),
    /// Bind the expression to a variable.
    Let { var: Variable, expr: Expression },
    /// Assert that the condition is true in each composition.
    Assert { condition: Expression },
    /// Assume that the condition is true in each composition.
    Assume { condition: Expression },
    /// Assert that the condition is true.
    /// The condition may refer to variables from different compositions.
    HyperAssert { condition: Expression },
    /// Assume that the condition is true.
    /// The condition may refer to variables from different compositions.
    HyperAssume { condition: Expression },
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

        if var.composition().is_some() {
            return Err("Target variable must not refer to a composition.".into());
        }
        if has_variables_with_composition(&expr) {
            return Err("Expression variables must not refer to a composition.".into());
        }

        Ok(Self::Let { var, expr })
    }

    /// Create a new assertion.
    pub fn assert(condition: Expression) -> Result<Self> {
        condition.sort().expect_boolean()?;

        if has_variables_with_composition(&condition) {
            return Err(
                "Condition variables must not refer to a composition, use hyper_assert.".into(),
            );
        }

        Ok(Self::Assert { condition })
    }

    /// Create a new assumption.
    pub fn assume(condition: Expression) -> Result<Self> {
        condition.sort().expect_boolean()?;

        if has_variables_with_composition(&condition) {
            return Err(
                "Condition variables must not refer to a composition, use hyper_assume.".into(),
            );
        }

        Ok(Self::Assume { condition })
    }

    /// Create a new hyper-assertion.
    ///
    /// The condition may refer to variables from different compositions.
    pub fn hyper_assert(condition: Expression) -> Result<Self> {
        condition.sort().expect_boolean()?;

        if has_variables_without_composition(&condition) {
            return Err("All condition variables must refer to a composition.".into());
        }

        Ok(Self::HyperAssert { condition })
    }

    // Create a new hyper-assumption.
    ///
    /// The condition may refer to variables from different compositions.
    pub fn hyper_assume(condition: Expression) -> Result<Self> {
        condition.sort().expect_boolean()?;

        if has_variables_without_composition(&condition) {
            return Err("All condition variables must refer to a composition.".into());
        }

        Ok(Self::HyperAssume { condition })
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
            Self::Comment(_) => Vec::new(),
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
            Self::Comment(_) => Vec::new(),
        }
    }

    /// Get each `Variable` defined by this `Node`.
    pub fn variables_defined(&self) -> Vec<&Variable> {
        match self {
            Self::Let { var, .. } => vec![var],
            Self::Comment(_)
            | Self::Assert { .. }
            | Self::Assume { .. }
            | Self::HyperAssert { .. }
            | Self::HyperAssume { .. } => Vec::new(),
        }
    }

    /// Get a mutable reference to each `Variable` defined by this `Node`.
    pub fn variables_defined_mut(&mut self) -> Vec<&mut Variable> {
        match self {
            Self::Let { var, .. } => vec![var],
            Self::Comment(_)
            | Self::Assert { .. }
            | Self::Assume { .. }
            | Self::HyperAssert { .. }
            | Self::HyperAssume { .. } => Vec::new(),
        }
    }

    /// Get each `Variable` referenced by this `Operation`.
    pub fn variables(&self) -> Vec<&Variable> {
        self.variables_used()
            .into_iter()
            .chain(self.variables_defined().into_iter())
            .collect()
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Comment(text) => write!(f, "// {}", text),
            Self::Let { var, expr } => write!(f, "let {} = {}", var, expr),
            Self::Assert { condition } => write!(f, "assert {}", condition),
            Self::Assume { condition } => write!(f, "assume {}", condition),
            Self::HyperAssert { condition } => write!(f, "hyper-assert {}", condition),
            Self::HyperAssume { condition } => write!(f, "hyper-assume {}", condition),
        }
    }
}

fn has_variables_with_composition(expr: &Expression) -> bool {
    expr.variables()
        .iter()
        .any(|var| var.composition().is_some())
}

fn has_variables_without_composition(expr: &Expression) -> bool {
    expr.variables()
        .iter()
        .any(|var| var.composition().is_none())
}
