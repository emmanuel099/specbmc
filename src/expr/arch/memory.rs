use crate::error::Result;
use crate::expr::{Expression, Sort, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Memory {
    Store(usize),
    Load(usize),
}

impl fmt::Display for Memory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Store(bit_width) => write!(f, "(store {})", bit_width),
            Self::Load(bit_width) => write!(f, "(load {})", bit_width),
        }
    }
}

impl Memory {
    pub fn variable() -> Variable {
        Variable::new("_memory", Sort::memory())
    }

    pub fn load(bit_width: usize, memory: Expression, addr: Expression) -> Result<Expression> {
        memory.sort().expect_memory()?;
        addr.sort().expect_word()?;

        Ok(Expression::new(
            Self::Load(bit_width).into(),
            vec![memory, addr],
            Sort::BitVector(bit_width),
        ))
    }

    pub fn store(memory: Expression, addr: Expression, value: Expression) -> Result<Expression> {
        memory.sort().expect_memory()?;
        addr.sort().expect_word()?;
        value.sort().expect_bit_vector()?;

        let bit_width = value.sort().unwrap_bit_vector();

        let result_sort = memory.sort().clone();
        Ok(Expression::new(
            Self::Store(bit_width).into(),
            vec![memory, addr, value],
            result_sort,
        ))
    }
}
