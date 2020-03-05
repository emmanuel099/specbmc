use crate::error::Result;
use crate::expr::{Expression, Variable};
use crate::mir::Operation;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Node {
    operation: Operation,
    address: Option<u64>,
}

impl Node {
    pub fn new(operation: Operation) -> Self {
        Self {
            operation,
            address: None,
        }
    }

    pub fn assign(var: Variable, expr: Expression) -> Result<Self> {
        Ok(Self::new(Operation::assign(var, expr)?))
    }

    pub fn assert(condition: Expression) -> Result<Self> {
        Ok(Self::new(Operation::assert(condition)?))
    }

    pub fn assume(condition: Expression) -> Result<Self> {
        Ok(Self::new(Operation::assume(condition)?))
    }

    pub fn hyper_assert(condition: Expression) -> Result<Self> {
        Ok(Self::new(Operation::hyper_assert(condition)?))
    }

    pub fn hyper_assume(condition: Expression) -> Result<Self> {
        Ok(Self::new(Operation::hyper_assume(condition)?))
    }

    pub fn operation(&self) -> &Operation {
        &self.operation
    }

    pub fn address(&self) -> Option<u64> {
        self.address
    }

    pub fn set_address(&mut self, address: Option<u64>) {
        self.address = address;
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(address) = self.address {
            write!(f, "{:X} ", address)?;
        }
        write!(f, "{}", self.operation)
    }
}
