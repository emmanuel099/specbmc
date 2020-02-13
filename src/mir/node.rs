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
