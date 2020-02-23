use crate::error::Result;
use crate::expr::{BitVectorValue, Expression, Sort, Variable};
use std::convert::TryFrom;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Boolean {
    True,
    False,
    Not,
    Imply,
    And,
    Or,
    Xor,
}

impl From<bool> for Boolean {
    fn from(value: bool) -> Self {
        if value {
            Boolean::True
        } else {
            Boolean::False
        }
    }
}

impl TryFrom<&Boolean> for bool {
    type Error = &'static str;

    fn try_from(b: &Boolean) -> std::result::Result<bool, Self::Error> {
        match b {
            Boolean::True => Ok(true),
            Boolean::False => Ok(false),
            _ => Err("not a constant"),
        }
    }
}

impl TryFrom<&Boolean> for BitVectorValue {
    type Error = &'static str;

    fn try_from(b: &Boolean) -> std::result::Result<BitVectorValue, Self::Error> {
        match b {
            Boolean::True => Ok(BitVectorValue::new(1, 1)),
            Boolean::False => Ok(BitVectorValue::new(0, 1)),
            _ => Err("not a constant"),
        }
    }
}

impl fmt::Display for Boolean {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::True => write!(f, "true"),
            Self::False => write!(f, "false"),
            Self::Not => write!(f, "not"),
            Self::Imply => write!(f, "=>"),
            Self::And => write!(f, "and"),
            Self::Or => write!(f, "or"),
            Self::Xor => write!(f, "xor"),
        }
    }
}

impl Boolean {
    pub fn variable(name: &str) -> Variable {
        Variable::new(name, Sort::boolean())
    }

    pub fn constant(value: bool) -> Expression {
        Expression::new(Self::from(value).into(), vec![], Sort::boolean())
    }

    pub fn not(expr: Expression) -> Result<Expression> {
        expr.sort().expect_boolean()?;

        Ok(Expression::new(
            Boolean::Not.into(),
            vec![expr],
            Sort::boolean(),
        ))
    }

    pub fn imply(lhs: Expression, rhs: Expression) -> Result<Expression> {
        lhs.sort().expect_boolean()?;
        rhs.sort().expect_boolean()?;

        Ok(Expression::new(
            Boolean::Imply.into(),
            vec![lhs, rhs],
            Sort::boolean(),
        ))
    }

    pub fn and(lhs: Expression, rhs: Expression) -> Result<Expression> {
        lhs.sort().expect_boolean()?;
        rhs.sort().expect_boolean()?;

        Ok(Expression::new(
            Boolean::And.into(),
            vec![lhs, rhs],
            Sort::boolean(),
        ))
    }

    pub fn conjunction(formulas: &[Expression]) -> Result<Expression> {
        if formulas.is_empty() {
            return Ok(Self::constant(true));
        }

        for formula in formulas {
            formula.sort().expect_boolean()?;
        }

        Ok(Expression::new(
            Boolean::And.into(),
            formulas.to_vec(),
            Sort::boolean(),
        ))
    }

    pub fn or(lhs: Expression, rhs: Expression) -> Result<Expression> {
        lhs.sort().expect_boolean()?;
        rhs.sort().expect_boolean()?;

        Ok(Expression::new(
            Boolean::Or.into(),
            vec![lhs, rhs],
            Sort::boolean(),
        ))
    }

    pub fn disjunction(formulas: &[Expression]) -> Result<Expression> {
        if formulas.is_empty() {
            return Ok(Self::constant(false));
        }

        for formula in formulas {
            formula.sort().expect_boolean()?;
        }

        Ok(Expression::new(
            Boolean::Or.into(),
            formulas.to_vec(),
            Sort::boolean(),
        ))
    }

    pub fn xor(lhs: Expression, rhs: Expression) -> Result<Expression> {
        lhs.sort().expect_boolean()?;
        rhs.sort().expect_boolean()?;

        Ok(Expression::new(
            Boolean::Xor.into(),
            vec![lhs, rhs],
            Sort::boolean(),
        ))
    }
}
