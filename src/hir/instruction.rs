//! An `Instruction` holds an `Operation`.
//!
use crate::error::Result;
use crate::expr::{Expression, Variable};
use crate::hir::{Effect, Operation};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Instruction {
    /// Operations happen in parallel.
    operations: Vec<Operation>,
    effects: Vec<Effect>,
    address: Option<u64>,
}

impl Instruction {
    /// Create a new instruction with the given index and operation.
    pub fn new(operation: Operation) -> Self {
        Self {
            operations: vec![operation],
            effects: vec![],
            address: None,
        }
    }

    /// Create a new `Assign` instruction.
    pub fn assign(variable: Variable, expr: Expression) -> Result<Self> {
        Ok(Self::new(Operation::assign(variable, expr)?))
    }

    /// Create a new `Store` instruction.
    pub fn store(address: Expression, expr: Expression) -> Result<Self> {
        Ok(Self::new(Operation::store(address, expr)?))
    }

    /// Create a new `Load` instruction.
    pub fn load(variable: Variable, address: Expression) -> Result<Self> {
        Ok(Self::new(Operation::load(variable, address)?))
    }

    /// Create a new `Branch` instruction.
    pub fn branch(target: Expression) -> Result<Self> {
        Ok(Self::new(Operation::branch(target)?))
    }

    /// Create a new `ConditionalBranch` instruction.
    pub fn conditional_branch(condition: Expression, target: Expression) -> Result<Self> {
        Ok(Self::new(Operation::conditional_branch(condition, target)?))
    }

    /// Create a new `Barrier` instruction.
    pub fn barrier() -> Self {
        Self::new(Operation::barrier())
    }

    /// Create a new `Assert` instruction.
    pub fn assert(condition: Expression) -> Result<Self> {
        Ok(Self::new(Operation::assert(condition)?))
    }

    /// Create a new `Assume` instruction.
    pub fn assume(condition: Expression) -> Result<Self> {
        Ok(Self::new(Operation::assume(condition)?))
    }

    /// Create a new `Observable` instruction.
    pub fn observable(exprs: Vec<Expression>) -> Self {
        Self::new(Operation::observable(exprs))
    }

    /// Create a new `Indistinguishable` instruction.
    pub fn indistinguishable(exprs: Vec<Expression>) -> Self {
        Self::new(Operation::indistinguishable(exprs))
    }

    /// Get the operations of this `Instruction`
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }

    /// Get a mutable reference to the operations of this `Instruction`
    pub fn operations_mut(&mut self) -> &mut Vec<Operation> {
        &mut self.operations
    }

    /// Add an `Operation` to this `Instruction`
    pub fn add_operation(&mut self, operation: Operation) {
        self.operations.push(operation);
    }

    /// Add multiple operations to this `Instruction`
    pub fn add_operations(&mut self, operations: &[Operation]) {
        self.operations.extend_from_slice(operations);
    }

    /// Add an `Effect` to this `Instruction`
    pub fn add_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    /// Add multiple effects to this `Instruction`
    pub fn add_effects(&mut self, effects: &[Effect]) {
        self.effects.extend_from_slice(effects);
    }

    /// Get the effects of this `Instruction`
    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }

    /// Get a mutable reference to the effects of this `Instruction`
    pub fn effects_mut(&mut self) -> &mut [Effect] {
        &mut self.effects
    }

    /// Returns whether this `Instruction` has effects or not.
    pub fn has_effects(&self) -> bool {
        !self.effects.is_empty()
    }

    /// Get the optional address for this `Instruction`
    pub fn address(&self) -> Option<u64> {
        self.address
    }

    /// Set the optional address for this `Instruction`
    pub fn set_address(&mut self, address: Option<u64>) {
        self.address = address;
    }

    /// Get the variables which will be written by this `Instruction`.
    pub fn variables_written(&self) -> Vec<&Variable> {
        self.operations
            .iter()
            .flat_map(Operation::variables_written)
            .collect()
    }

    /// Get a mutable reference to the variables which will be written by this `Instruction`.
    pub fn variables_written_mut(&mut self) -> Vec<&mut Variable> {
        self.operations
            .iter_mut()
            .flat_map(Operation::variables_written_mut)
            .collect()
    }

    /// Get the variables read by this `Instruction`.
    pub fn variables_read(&self) -> Vec<&Variable> {
        self.operations
            .iter()
            .flat_map(Operation::variables_read)
            .chain(self.effects.iter().flat_map(Effect::variables))
            .collect()
    }

    /// Get a mutable reference to the variables read by this `Instruction`.
    pub fn variables_read_mut(&mut self) -> Vec<&mut Variable> {
        self.operations
            .iter_mut()
            .flat_map(Operation::variables_read_mut)
            .chain(self.effects.iter_mut().flat_map(Effect::variables_mut))
            .collect()
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(address) = self.address {
            write!(f, "{:X} ", address)?;
        }
        if !self.operations.is_empty() {
            write!(f, "{}", self.operations.first().unwrap())?;
            for operation in self.operations.iter().skip(1) {
                write!(f, "\n\t|| {}", operation)?;
            }
        }
        for effect in &self.effects {
            write!(f, "\n\t# {}", effect)?;
        }
        Ok(())
    }
}
