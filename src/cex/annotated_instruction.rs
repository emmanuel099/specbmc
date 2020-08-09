use crate::cex::{AnnotatedElement, Effect};
use crate::expr::{Constant, Expression, Variable};
use crate::hir::Instruction;
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Debug)]
pub struct Annotation {
    /// Assignments produced by the underlying instruction.
    /// The target is of type Expression instead of Variable to allow e.g. `mem(50) := 5`
    assignments: Vec<(Expression, Constant)>,
    /// Effects produced by the underlying instruction.
    effects: Vec<Effect>,
    /// Configuration
    configuration: HashMap<Variable, Constant>,
}

impl Annotation {
    pub fn add_assignment(&mut self, target: Expression, value: Constant) {
        self.assignments.push((target, value));
    }

    pub fn add_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    pub fn add_variable_configuration(&mut self, var: Variable, value: Constant) {
        self.configuration.insert(var, value);
    }

    pub fn assignments(&self) -> &Vec<(Expression, Constant)> {
        &self.assignments
    }

    pub fn effects(&self) -> &Vec<Effect> {
        &self.effects
    }

    pub fn configuration(&self) -> &HashMap<Variable, Constant> {
        &self.configuration
    }
}

impl Default for Annotation {
    fn default() -> Self {
        Self {
            assignments: Vec::new(),
            effects: Vec::new(),
            configuration: HashMap::new(),
        }
    }
}

pub type AnnotatedInstruction = AnnotatedElement<Instruction, Annotation>;

impl AnnotatedInstruction {
    /// Returns the actual `Instruction`.
    pub fn instruction(&self) -> &Instruction {
        &self.element
    }
}

impl fmt::Display for AnnotatedInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.instruction())?;
        for (composition, annotation) in &self.annotations {
            for (target, value) in &annotation.assignments {
                writeln!(f, "${} {} = {}", composition, target, value)?;
            }
            for effect in &annotation.effects {
                writeln!(f, "#{} {}", composition, effect)?;
            }
        }
        Ok(())
    }
}
