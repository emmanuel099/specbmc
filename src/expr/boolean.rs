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
            Self::True
        } else {
            Self::False
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
        Expression::new(Self::from(value).into(), vec![], Sort::boolean())
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

    pub fn is_constant(&self) -> bool {
        match self {
            Self::True | Self::False => true,
            _ => false,
        }
    }
}
