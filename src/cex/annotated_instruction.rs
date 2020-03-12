use crate::cex::{AnnotatedElement, Effect};
use crate::expr::{Constant, Variable};
use crate::hir::Instruction;
use std::fmt;

#[derive(Clone, Debug)]
pub struct Annotation {
    /// Variable assignments produced by the underlying instruction.
    assignments: Vec<(Variable, Constant)>,
    /// Effects produced by the underlying instruction.
    effects: Vec<Effect>,
}

impl Annotation {
    pub fn add_assignment(&mut self, var: Variable, value: Constant) {
        self.assignments.push((var, value));
    }

    pub fn add_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    pub fn assignments(&self) -> &Vec<(Variable, Constant)> {
        &self.assignments
    }

    pub fn effects(&self) -> &Vec<Effect> {
        &self.effects
    }
}

impl Default for Annotation {
    fn default() -> Self {
        Self {
            assignments: Vec::new(),
            effects: Vec::new(),
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.instruction())?;
        for (composition, annotation) in &self.annotations {
            for (var, value) in &annotation.assignments {
                writeln!(f, "${} {} = {}", composition, var, value)?;
            }
            for effect in &annotation.effects {
                writeln!(f, "#{} {}", composition, effect)?;
            }
        }
        Ok(())
    }
}
