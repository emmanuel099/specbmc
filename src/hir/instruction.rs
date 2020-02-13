//! An `Instruction` holds an `Operation`.

use crate::expr::{Expression, Variable};
use crate::hir::*;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Instruction {
    operation: Operation,
    effects: Vec<Effect>,
    index: usize,
    address: Option<u64>,
}

impl Instruction {
    /// Create a new instruction with the given index and operation.
    pub fn new(index: usize, operation: Operation) -> Self {
        Self {
            operation,
            effects: vec![],
            index,
            address: None,
        }
    }

    /// Create a new `Assign` instruction.
    pub fn assign(index: usize, variable: Variable, expr: Expression) -> Instruction {
        Instruction::new(index, Operation::assign(variable, expr))
    }

    /// Create a new `Store` instruction.
    pub fn store(
        index: usize,
        memory: Variable,
        address: Expression,
        expr: Expression,
    ) -> Instruction {
        Instruction::new(index, Operation::store(memory, address, expr))
    }

    /// Create a new `Load` instruction.
    pub fn load(
        index: usize,
        variable: Variable,
        memory: Variable,
        address: Expression,
    ) -> Instruction {
        Instruction::new(index, Operation::load(variable, memory, address))
    }

    /// Create a new `Branch` instruction.
    pub fn branch(index: usize, target: Expression) -> Instruction {
        Instruction::new(index, Operation::branch(target))
    }

    /// Create a new `Barrier` instruction.
    pub fn barrier(index: usize) -> Instruction {
        Instruction::new(index, Operation::barrier())
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

    /// Returns `true` if the `Operation` for this `Instruction` is `Operation::Barrier`
    pub fn is_barrier(&self) -> bool {
        self.operation.is_barrier()
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

    /// Get the effects of this `Instruction`
    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }

    /// Get a mutable reference to the effects of this `Instruction`
    pub fn effects_mut(&mut self) -> &mut [Effect] {
        &mut self.effects
    }

    /// Get the index for this `Instruction`.
    ///
    /// An `Instruction` index is assigned by its parent `Block` and uniquely identifies the
    /// `Instruction` within the `Block`. `Instruction` indices need not be continuous, nor
    /// in order.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Get the optional address for this `Instruction`
    pub fn address(&self) -> Option<u64> {
        self.address
    }

    /// Set the optional address for this `Instruction`
    pub fn set_address(&mut self, address: Option<u64>) {
        self.address = address;
    }

    /// Clone this instruction with a new index.
    pub fn clone_new_index(&self, index: usize) -> Instruction {
        Instruction {
            operation: self.operation.clone(),
            effects: self.effects.clone(),
            index,
            address: self.address,
        }
    }

    /// Get the variables which will be written by this `Instruction`.
    pub fn variables_written(&self) -> Vec<&Variable> {
        self.effects
            .iter()
            .flat_map(|effect| effect.variables_written())
            .chain(self.operation.variables_written())
            .collect()
    }

    /// Get a mutable reference to the variables which will be written by this `Instruction`.
    pub fn variables_written_mut(&mut self) -> Vec<&mut Variable> {
        self.effects
            .iter_mut()
            .flat_map(|effect| effect.variables_written_mut())
            .chain(self.operation.variables_written_mut())
            .collect()
    }

    /// Get the variables read by this `Instruction`.
    pub fn variables_read(&self) -> Vec<&Variable> {
        self.effects
            .iter()
            .flat_map(|effect| effect.variables_read())
            .chain(self.operation.variables_read())
            .collect()
    }

    /// Get a mutable reference to the variables read by this `Instruction`.
    pub fn variables_read_mut(&mut self) -> Vec<&mut Variable> {
        self.effects
            .iter_mut()
            .flat_map(|effect| effect.variables_read_mut())
            .chain(self.operation.variables_read_mut())
            .collect()
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let prefix = match self.address {
            Some(address) => format!("{:X} {:02X} {}", address, self.index, self.operation),
            None => format!("{:02X} {}", self.index, self.operation),
        };
        write!(f, "{}", prefix)?;
        for effect in &self.effects {
            write!(f, "\n\t{}", effect)?;
        }
        Ok(())
    }
}
