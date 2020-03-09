use crate::cex::{AnnotatedInstruction, Composition};
use falcon::graph;
use std::fmt;

#[derive(Clone, Debug)]
pub struct Block {
    /// The index of the block.
    index: usize,
    /// The instructions for this block.
    instructions: Vec<AnnotatedInstruction>,
    /// Is this block part of transient execution?
    transient: bool,
    /// Is this block executed in composition A?
    executed_a: bool,
    /// Is this block executed in composition B?
    executed_b: bool,
}

impl Block {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            instructions: Vec::new(),
            transient: false,
            executed_a: false,
            executed_b: false,
        }
    }

    /// Returns the index of this `Block`
    pub fn index(&self) -> usize {
        self.index
    }

    /// Sets whether this `Block` is part of transient execution or not.
    pub fn set_transient(&mut self, transient: bool) {
        self.transient = transient;
    }

    /// Returns whether this `Block` is part of transient execution or not.
    pub fn is_transient(&self) -> bool {
        self.transient
    }

    /// Adds the instructions to this `Block`
    pub fn add_instructions(&mut self, instruction: AnnotatedInstruction) {
        self.instructions.push(instruction);
    }

    /// Returns instructions for this `Block`
    pub fn instructions(&self) -> &Vec<AnnotatedInstruction> {
        &self.instructions
    }

    /// Returns a mutable reference to the instructions for this `Block`.
    pub fn instructions_mut(&mut self) -> &mut Vec<AnnotatedInstruction> {
        &mut self.instructions
    }

    /// Returns try if this `Block` is empty, meaning it has no `Instruction`
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Returns an `Instruction` by index, or `None` if the instruction does not exist.
    pub fn instruction(&self, index: usize) -> Option<&AnnotatedInstruction> {
        self.instructions.get(index)
    }

    /// Returns a mutable reference to an `Instruction` by index, or `None` if
    /// the `Instruction` does not exist.
    pub fn instruction_mut(&mut self, index: usize) -> Option<&mut AnnotatedInstruction> {
        self.instructions.get_mut(index)
    }

    /// Marks this `Block` as executed in the given composition.
    pub fn mark_as_executed(&mut self, composition: Composition) {
        match composition {
            Composition::A => {
                self.executed_a = true;
            }
            Composition::B => {
                self.executed_b = true;
            }
        }
    }

    /// Returns whether this `Block` is executed in any composition.
    pub fn executed(&self) -> bool {
        self.executed_a || self.executed_b
    }
}

impl graph::Vertex for Block {
    fn index(&self) -> usize {
        self.index
    }

    fn dot_label(&self) -> String {
        format!("{}", self)
    }

    fn dot_fill_color(&self) -> String {
        if self.executed() {
            if self.transient {
                "#e1e1e1ff".to_string()
            } else {
                "#ffddccff".to_string()
            }
        } else {
            if self.transient {
                "#e1e1e155".to_string()
            } else {
                "#ffddcc55".to_string()
            }
        }
    }

    fn dot_font_color(&self) -> String {
        if self.executed() {
            "#343434ff".to_string()
        } else {
            "#34343455".to_string()
        }
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[ Block: 0x{:X}", self.index)?;
        if self.transient {
            write!(f, ", Transient")?;
        }
        writeln!(f, " ]")?;
        for instruction in &self.instructions {
            write!(f, "{}", instruction)?;
        }
        Ok(())
    }
}
