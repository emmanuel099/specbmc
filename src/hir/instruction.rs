//! An `Instruction` holds an `Operation`.
//!
use crate::error::Result;
use crate::expr::{Expression, Variable};
use crate::hir::{Effect, Operation};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Instruction {
    operation: Operation,
    effects: Vec<Effect>,
    address: Option<u64>,
}

impl Instruction {
    /// Create a new instruction with the given index and operation.
    pub fn new(operation: Operation) -> Self {
        Self {
            operation,
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
    pub fn observable(variables: Vec<Variable>) -> Self {
        Self::new(Operation::observable(variables))
    }

    /// Create a new `Indistinguishable` instruction.
    pub fn indistinguishable(variables: Vec<Variable>) -> Self {
        Self::new(Operation::indistinguishable(variables))
    }

    /// Returns `true` if the `Operation` for this `Instruction` is `Operation::Assign`
    pub fn is_assign(&self) -> bool {
        self.operation.is_assign()
    }

    /// Returns `true` if the `Operation` for this `Instruction` is `Operation::Store`
    pub fn is_store(&self) -> bool {
        self.operation.is_store()
    }

    /// Returns `true` if the `Operation` for this `Instruction` is `Operation::Load`
    pub fn is_load(&self) -> bool {
        self.operation.is_load()
    }

    /// Returns `true` if the `Operation` for this `Instruction` is `Operation::Branch`
    pub fn is_branch(&self) -> bool {
        self.operation.is_branch()
    }

    /// Returns `true` if the `Operation` for this `Instruction` is `Operation::ConditionalBranch`
    pub fn is_conditional_branch(&self) -> bool {
        self.operation.is_conditional_branch()
    }

    /// Returns `true` if the `Operation` for this `Instruction` is `Operation::Barrier`
    pub fn is_barrier(&self) -> bool {
        self.operation.is_barrier()
    }

    /// Returns `true` if the `Operation` for this `Instruction` is `Operation::Assert`
    pub fn is_assert(&self) -> bool {
        self.operation.is_assert()
    }

    /// Returns `true` if the `Operation` for this `Instruction` is `Operation::Assume`
    pub fn is_assume(&self) -> bool {
        self.operation.is_assume()
    }

    /// Returns `true` if the `Operation` for this `Instruction` is `Operation::Observable`
    pub fn is_observable(&self) -> bool {
        self.operation.is_observable()
    }

    /// Returns `true` if the `Operation` for this `Instruction` is `Operation::Indistinguishable`
    pub fn is_indistinguishable(&self) -> bool {
        self.operation.is_indistinguishable()
    }

    /// Get the `Operation` for this `Instruction`
    pub fn operation(&self) -> &Operation {
        &self.operation
    }

    /// Get a mutable reference to the `Operation` for this `Instruction`
    pub fn operation_mut(&mut self) -> &mut Operation {
        &mut self.operation
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
        self.operation.variables_written()
    }

    /// Get a mutable reference to the variables which will be written by this `Instruction`.
    pub fn variables_written_mut(&mut self) -> Vec<&mut Variable> {
        self.operation.variables_written_mut()
    }

    /// Get the variables read by this `Instruction`.
    pub fn variables_read(&self) -> Vec<&Variable> {
        self.operation.variables_read()
    }

    /// Get a mutable reference to the variables read by this `Instruction`.
    pub fn variables_read_mut(&mut self) -> Vec<&mut Variable> {
        self.operation.variables_read_mut()
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(address) = self.address {
            write!(f, "{:X} ", address)?;
        }
        write!(f, "{}", self.operation)?;
        for effect in &self.effects {
            write!(f, "\n\t# {}", effect)?;
        }
        Ok(())
    }
}
