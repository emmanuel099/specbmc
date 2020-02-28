use crate::error::Result;
use crate::expr::{Expression, Sort, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Cache {
    Init,
    Fetch(usize), // Fetch N bits into the cache
}

impl fmt::Display for Cache {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Init => write!(f, "cache-init"),
            Self::Fetch(bit_width) => write!(f, "(cache-fetch {})", bit_width),
        }
    }
}

impl Cache {
    pub fn variable() -> Variable {
        Variable::new("_cache", Sort::cache())
    }

    pub fn variable_nonspec() -> Variable {
        Variable::new("_cache_ns", Sort::cache())
    }

    pub fn init() -> Result<Expression> {
        Ok(Expression::new(Self::Init.into(), vec![], Sort::cache()))
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
