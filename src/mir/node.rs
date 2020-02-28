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

    pub fn new_let(var: Variable, expr: Expression) -> Result<Self> {
        Ok(Self::new(Operation::new_let(var, expr)?))
    }

    pub fn new_assert(cond: Expression) -> Result<Self> {
        Ok(Self::new(Operation::new_assert(cond)?))
    }

    pub fn new_assume(cond: Expression) -> Result<Self> {
        Ok(Self::new(Operation::new_assume(cond)?))
    }

    pub fn new_assert_equal_in_self_composition(
        compositions: Vec<usize>,
        expr: Expression,
    ) -> Self {
        Self::new(Operation::new_assert_equal_in_self_composition(
            compositions,
            expr,
        ))
    }

    pub fn new_assume_equal_in_self_composition(
        compositions: Vec<usize>,
        expr: Expression,
    ) -> Self {
        Self::new(Operation::new_assume_equal_in_self_composition(
            compositions,
            expr,
        ))
    }

    pub fn operation(&self) -> &Operation {
        &self.operation
    }

    pub fn operation_mut(&mut self) -> &mut Operation {
        &mut self.operation
    }

    pub fn set_operation(&mut self, operation: Operation) {
        self.operation = operation;
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
