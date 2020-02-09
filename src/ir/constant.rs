use crate::ir::{bitvector, Expression, Sort};
use std::fmt;

#[derive(Clone, Debug)]
pub enum Constant {
    Boolean(bool),
    BitVector(bitvector::Value),
}

impl Constant {
    pub fn sort(&self) -> Sort {
        match self {
            Constant::Boolean(_) => Sort::Bool,
            Constant::BitVector(value) => Sort::BitVector(value.bits()),
        }
    }
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::Boolean(value) => format!("{}", value),
            Self::BitVector(value) => format!("{}", value),
        };
        write!(f, "${}:{}", s, self.sort())
    }
}

impl Into<Expression> for Constant {
    fn into(self) -> Expression {
        Expression::constant(self)
    }
}
