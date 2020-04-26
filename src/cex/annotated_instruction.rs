use crate::cex::{AnnotatedElement, Effect};
use crate::expr::{Constant, Expression};
use crate::hir::Instruction;
use std::fmt;

#[derive(Clone, Debug)]
pub struct Annotation {
    /// Assignments produced by the underlying instruction.
    /// The target is of type Expression instead of Variable to allow e.g. `mem(50) := 5`
    assignments: Vec<(Expression, Constant)>,
    /// Effects produced by the underlying instruction.
    effects: Vec<Effect>,
}

impl Annotation {
    pub fn add_assignment(&mut self, target: Expression, value: Constant) {
        self.assignments.push((target, value));
    }

    pub fn add_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    pub fn assignments(&self) -> &Vec<(Expression, Constant)> {
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
