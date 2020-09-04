//! An `Instruction` holds an `Operation`.
//!
use crate::error::Result;
use crate::expr::{Expression, Variable};
use crate::hir::{Effect, Operation};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum Label {
    Pseudo, // Instruction isn't part of the assembly
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pseudo => write!(f, "pseudo"),
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Default)]
pub struct Labels {
    labels: BTreeSet<Label>,
}

impl Labels {
    pub fn pseudo(&mut self) -> &mut Self {
        self.labels.insert(Label::Pseudo);
        self
    }

    pub fn is_pseudo(&self) -> bool {
        self.labels.contains(&Label::Pseudo)
    }

    pub fn merge(&mut self, other: &Labels) {
        other.labels.iter().for_each(|&label| {
            self.labels.insert(label);
        });
    }
}

impl fmt::Display for Labels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.labels.is_empty() {
            return Ok(());
        }
        write!(f, "[")?;
        let mut is_first = true;
        for label in &self.labels {
            if !is_first {
                write!(f, ", ")?;
            }
            write!(f, "{}", label)?;
            is_first = false;
        }
        write!(f, "]")
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Instruction {
    operation: Operation,
    effects: Vec<Effect>,
    address: Option<u64>,
    labels: Labels,
}

impl Instruction {
    /// Create a new instruction with the given index and operation.
    pub fn new(operation: Operation) -> Self {
        Self {
            operation,
            effects: vec![],
            address: None,
            labels: Labels::default(),
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

    /// Create a new `Call` instruction.
    pub fn call(target: Expression) -> Result<Self> {
        Ok(Self::new(Operation::call(target)?))
    }

    /// Create a new `Branch` instruction.
    pub fn branch(target: Expression) -> Result<Self> {
        Ok(Self::new(Operation::branch(target)?))
    }

    /// Create a new `ConditionalBranch` instruction.
    pub fn conditional_branch(condition: Expression, target: Expression) -> Result<Self> {
        Ok(Self::new(Operation::conditional_branch(condition, target)?))
    }

    /// Create a new `Skip` instruction.
    pub fn skip() -> Self {
        Self::new(Operation::skip())
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
    pub fn observable(expr: Expression) -> Self {
        Self::new(Operation::observable(expr))
    }

    /// Create a new `Indistinguishable` instruction.
    pub fn indistinguishable(expr: Expression) -> Self {
        Self::new(Operation::indistinguishable(expr))
    }

    /// Get the operation of this `Instruction`
    pub fn operation(&self) -> &Operation {
        &self.operation
    }

    /// Get a mutable reference to the operation of this `Instruction`
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

    /// Retrieve the labels of this `Instruction`.
    pub fn labels(&self) -> &Labels {
        &self.labels
    }

    /// Retrieve a mutable reference to the labels of this `Instruction`.
    pub fn labels_mut(&mut self) -> &mut Labels {
        &mut self.labels
    }

    /// Get the optional address for this `Instruction`
    pub fn address(&self) -> Option<u64> {
        self.address
    }

    /// Set the optional address for this `Instruction`
    pub fn set_address(&mut self, address: Option<u64>) {
        self.address = address;
    }

    pub fn is_assign(&self) -> bool {
        self.operation.is_assign()
    }

    pub fn is_store(&self) -> bool {
        self.operation.is_store()
    }

    pub fn is_load(&self) -> bool {
        self.operation.is_load()
    }

    pub fn is_call(&self) -> bool {
        self.operation.is_call()
    }

    pub fn is_branch(&self) -> bool {
        self.operation.is_branch()
    }

    pub fn is_conditional_branch(&self) -> bool {
        self.operation.is_conditional_branch()
    }

    pub fn is_skip(&self) -> bool {
        self.operation.is_skip()
    }

    pub fn is_barrier(&self) -> bool {
        self.operation.is_barrier()
    }

    pub fn is_assert(&self) -> bool {
        self.operation.is_assert()
    }

    pub fn is_assume(&self) -> bool {
        self.operation.is_assume()
    }

    pub fn is_observable(&self) -> bool {
        self.operation.is_observable()
    }

    pub fn is_indistinguishable(&self) -> bool {
        self.operation.is_indistinguishable()
    }

    /// Get the variables written by this `Instruction`.
    pub fn variables_written(&self) -> Vec<&Variable> {
        self.operation.variables_written()
    }

    /// Get a mutable reference to the variables written by this `Instruction`.
    pub fn variables_written_mut(&mut self) -> Vec<&mut Variable> {
        self.operation.variables_written_mut()
    }

    /// Get the variables read by this `Instruction`.
    pub fn variables_read(&self) -> Vec<&Variable> {
        self.effects
            .iter()
            .flat_map(Effect::variables)
            .chain(self.operation.variables_read())
            .collect()
    }

    /// Get a mutable reference to the variables read by this `Instruction`.
    pub fn variables_read_mut(&mut self) -> Vec<&mut Variable> {
        self.effects
            .iter_mut()
            .flat_map(Effect::variables_mut)
            .chain(self.operation.variables_read_mut())
            .collect()
    }

    /// Get each `Variable` used by this `Instruction`.
    pub fn variables(&self) -> Vec<&Variable> {
        self.variables_read()
            .into_iter()
            .chain(self.variables_written().into_iter())
            .collect()
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
