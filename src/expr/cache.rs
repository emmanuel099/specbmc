use crate::error::Result;
use crate::expr::{Expression, Operator, Sort, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Cache {
    Fetch(usize), // Fetch N bits into the cache
}

impl Into<Operator> for Cache {
    fn into(self) -> Operator {
        Operator::Cache(self)
    }
}

impl fmt::Display for Cache {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Fetch(bit_width) => write!(f, "(cache-fetch {})", bit_width),
        }
    }
}

impl Cache {
    pub fn variable() -> Variable {
        Variable::new("_cache", Sort::cache())
    }

    pub fn fetch(bit_width: usize, cache: Expression, addr: Expression) -> Result<Expression> {
        cache.sort().expect_cache()?;
        addr.sort().expect_bit_vector()?;

        Ok(Expression::new(
            Self::Fetch(bit_width).into(),
            vec![cache, addr],
            Sort::cache(),
        ))
    }
}
