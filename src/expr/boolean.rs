use crate::error::Result;
use crate::expr::{Constant, Expression, Sort, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Boolean {
    Not,
    Imply,
    And,
    Or,
    Xor,
}

impl fmt::Display for Boolean {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Not => write!(f, "not"),
            Self::Imply => write!(f, "=>"),
            Self::And => write!(f, "and"),
            Self::Or => write!(f, "or"),
            Self::Xor => write!(f, "xor"),
        }
    }
}

macro_rules! boolean_unary {
    ( $name:ident, $op:expr ) => {
        pub fn $name(expr: Expression) -> Result<Expression> {
            expr.sort().expect_boolean()?;

            Ok(Expression::new($op.into(), vec![expr], Sort::boolean()))
        }
    };
}

macro_rules! boolean_binary {
    ( $name:ident, $op:expr ) => {
        pub fn $name(lhs: Expression, rhs: Expression) -> Result<Expression> {
            lhs.sort().expect_boolean()?;
            rhs.sort().expect_boolean()?;

            Ok(Expression::new($op.into(), vec![lhs, rhs], Sort::boolean()))
        }
    };
}

impl Boolean {
    pub fn variable(name: &str) -> Variable {
        Variable::new(name, Sort::boolean())
    }

    pub fn constant(value: bool) -> Expression {
        Expression::constant(Constant::boolean(value), Sort::boolean())
    }

    boolean_unary!(not, Self::Not);

    boolean_binary!(imply, Self::Imply);
    boolean_binary!(and, Self::And);
    boolean_binary!(or, Self::Or);
    boolean_binary!(xor, Self::Xor);

    pub fn conjunction(formulas: &[Expression]) -> Result<Expression> {
        if formulas.is_empty() {
            return Ok(Self::constant(true));
        }

        for formula in formulas {
            formula.sort().expect_boolean()?;
        }

        Ok(Expression::new(
            Self::And.into(),
            formulas.to_vec(),
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
            Self::Or.into(),
            formulas.to_vec(),
            Sort::boolean(),
        ))
    }
}
