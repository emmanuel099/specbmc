use crate::error::Result;
use crate::lir::{Expression, Operator, Sort, Variable};
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

impl Into<Operator> for Boolean {
    fn into(self) -> Operator {
        Operator::Boolean(self)
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
        let op = if value { Boolean::True } else { Boolean::False };
        Expression::new(op.into(), vec![], Sort::boolean())
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
