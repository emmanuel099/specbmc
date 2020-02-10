use crate::error::Result;
use crate::lir::{Expression, Operator, Sort, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Memory {
    Store(usize),
    Load(usize),
}

impl Into<Operator> for Memory {
    fn into(self) -> Operator {
        Operator::Memory(self)
    }
}

impl fmt::Display for Memory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Store(bit_width) => write!(f, "(store {})", bit_width),
            Self::Load(bit_width) => write!(f, "(load {})", bit_width),
        }
    }
}

impl Memory {
    pub fn load(bit_width: usize, memory: Variable, addr: Expression) -> Result<Expression> {
        memory.sort().expect_memory()?;
        addr.sort().expect_bit_vector()?;

        Ok(Expression::new(
            Memory::Load(bit_width).into(),
            vec![memory.into(), addr],
            Sort::BitVector(bit_width),
        ))
    }

    pub fn store(memory: Variable, addr: Expression, value: Expression) -> Result<Expression> {
        memory.sort().expect_memory()?;
        addr.sort().expect_bit_vector()?;
        value.sort().expect_bit_vector()?;

        let bit_width = match value.sort() {
            Sort::BitVector(width) => *width,
            _ => 0,
        };

        let result_sort = *memory.sort();
        Ok(Expression::new(
            Memory::Store(bit_width).into(),
            vec![memory.into(), addr, value],
            result_sort,
        ))
    }
}
