use crate::error::Result;
use crate::ir::{Constant, Expression, Operator, Sort};
use std::fmt;

#[derive(Clone, Debug)]
pub enum Boolean {
    Not,
    Imply,
    And,
    Or,
    Xor,
}

impl Into<Operator> for Boolean {
    fn into(self) -> Operator {
        Operator::Boolean(self)
    }
}

impl fmt::Display for Boolean {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Not => write!(f, "not"),
            Self::Imply => write!(f, "=>"),
            Self::And => write!(f, "ite"),
            Self::Or => write!(f, "or"),
            Self::Xor => write!(f, "xor"),
        }
    }
}

impl Boolean {
    pub fn constant(value: bool) -> Constant {
        Constant::Boolean(value)
    }

    pub fn not(expr: Expression) -> Result<Expression> {
        expr.sort().expect_bool()?;

        Ok(Expression::new(Boolean::Not.into(), vec![expr], Sort::Bool))
    }

    pub fn imply(lhs: Expression, rhs: Expression) -> Result<Expression> {
        lhs.sort().expect_bool()?;
        rhs.sort().expect_bool()?;

        Ok(Expression::new(
            Boolean::Imply.into(),
            vec![lhs, rhs],
            Sort::Bool,
        ))
    }

    pub fn and(lhs: Expression, rhs: Expression) -> Result<Expression> {
        lhs.sort().expect_bool()?;
        rhs.sort().expect_bool()?;

        Ok(Expression::new(
            Boolean::And.into(),
            vec![lhs, rhs],
            Sort::Bool,
        ))
    }

    pub fn or(lhs: Expression, rhs: Expression) -> Result<Expression> {
        lhs.sort().expect_bool()?;
        rhs.sort().expect_bool()?;

        Ok(Expression::new(
            Boolean::Or.into(),
            vec![lhs, rhs],
            Sort::Bool,
        ))
    }

    pub fn xor(lhs: Expression, rhs: Expression) -> Result<Expression> {
        lhs.sort().expect_bool()?;
        rhs.sort().expect_bool()?;

        Ok(Expression::new(
            Boolean::Xor.into(),
            vec![lhs, rhs],
            Sort::Bool,
        ))
    }
}
