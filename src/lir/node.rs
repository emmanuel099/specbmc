use crate::lir::Operation;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Node {
    operation: Operation,
    address: Option<u64>,
    // TODO add some pointers to Falcon IL
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
        self.address.clone()
    }

    pub fn set_address(&mut self, address: Option<u64>) {
        self.address = address;
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.operation)
    }
}
