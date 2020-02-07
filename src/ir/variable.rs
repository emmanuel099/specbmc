use crate::ir::{Expression, Sort};
use std::fmt;

#[derive(Clone, Debug)]
pub struct Variable {
    name: String,
    sort: Sort,
}

impl Variable {
    /// Create a new `Variable` with the given name and sort.
    pub fn new<S>(name: S, sort: Sort) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            sort,
        }
    }

    /// Gets the name of the `Variable`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the sort of the `Variable`.
    pub fn sort(&self) -> &Sort {
        &self.sort
    }
}

impl fmt::Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.name(), self.sort())
    }
}

impl Into<Expression> for Variable {
    fn into(self) -> Expression {
        Expression::variable(self)
    }
}
