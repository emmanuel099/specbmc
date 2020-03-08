use crate::error::Result;
use crate::expr::{Constant, Expression, Sort, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Integer {
    Lt,
    Gt,
    Lte,
    Gte,
    Mod,
    Div,
    Abs,
    Mul,
    Add,
    Sub,
    Neg,
}

impl fmt::Display for Integer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Lt => write!(f, "<"),
            Self::Gt => write!(f, ">"),
            Self::Lte => write!(f, "<="),
            Self::Gte => write!(f, ">="),
            Self::Mod => write!(f, "mod"),
            Self::Div => write!(f, "div"),
            Self::Abs => write!(f, "abs"),
            Self::Mul => write!(f, "*"),
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::Neg => write!(f, "-"),
        }
    }
}

macro_rules! int_arith_unary {
    ( $name:ident, $op:expr ) => {
        pub fn $name(expr: Expression) -> Result<Expression> {
            expr.sort().expect_integer()?;

            Ok(Expression::new($op.into(), vec![expr], Sort::integer()))
        }
    };
}

macro_rules! int_arith_binary {
    ( $name:ident, $op:expr ) => {
        pub fn $name(lhs: Expression, rhs: Expression) -> Result<Expression> {
            lhs.sort().expect_integer()?;
            rhs.sort().expect_integer()?;

            Ok(Expression::new($op.into(), vec![lhs, rhs], Sort::integer()))
        }
    };
}

macro_rules! int_comp {
    ( $name:ident, $op:expr ) => {
        pub fn $name(lhs: Expression, rhs: Expression) -> Result<Expression> {
            lhs.sort().expect_integer()?;
            rhs.sort().expect_integer()?;

            Ok(Expression::new($op.into(), vec![lhs, rhs], Sort::boolean()))
        }
    };
}

impl Integer {
    pub fn variable(name: &str) -> Variable {
        Variable::new(name, Sort::integer())
    }

    pub fn constant(value: u64) -> Expression {
        Expression::constant(Constant::integer(value), Sort::integer())
    }

    int_arith_unary!(abs, Self::Abs);
    int_arith_unary!(neg, Self::Neg);

    int_arith_binary!(modulo, Self::Mod);
    int_arith_binary!(div, Self::Div);
    int_arith_binary!(mul, Self::Mul);
    int_arith_binary!(add, Self::Add);
    int_arith_binary!(sub, Self::Sub);

    int_comp!(lt, Self::Lt);
    int_comp!(gt, Self::Gt);
    int_comp!(lte, Self::Lte);
    int_comp!(gte, Self::Gte);
}
