use crate::lir::{bitvector, Expression, Sort};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Constant {
    Boolean(bool),
    BitVector(bitvector::Value),
}

impl Constant {
    pub fn sort(&self) -> Sort {
        match self {
            Constant::Boolean(_) => Sort::boolean(),
            Constant::BitVector(value) => Sort::bit_vector(value.bits()),
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
