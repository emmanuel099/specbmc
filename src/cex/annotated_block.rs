use crate::cex::{AnnotatedElement, AnnotatedInstruction};
use crate::hir;
use falcon::graph;
use std::fmt;

#[derive(Clone, Debug)]
pub struct Annotation {
    /// Is this block executed?
    executed: bool,
}

impl Annotation {
    /// Marks this `Block` as executed.
    pub fn mark_as_executed(&mut self) {
        self.executed = true;
    }

    /// Returns whether this `Block` is executed.
    pub fn executed(&self) -> bool {
        self.executed
    }
}

impl Default for Annotation {
    fn default() -> Self {
        Self { executed: false }
    }
}

#[derive(Clone, Debug)]
pub struct Block {
    /// The index of the block.
    index: usize,
    /// The instructions for this block.
    instructions: Vec<AnnotatedInstruction>,
    /// Is this block part of transient execution?
    transient: bool,
}

impl Block {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            instructions: Vec::new(),
            transient: false,
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
}

impl From<&hir::Block> for Block {
    fn from(hir_block: &hir::Block) -> Self {
        let mut cex_block = Block::new(hir_block.index());
        cex_block.set_transient(hir_block.is_transient());

        for inst in hir_block.instructions() {
            if inst.labels().is_pseudo() {
                continue;
            }

            let cex_inst = AnnotatedInstruction::new(inst.clone());
            cex_block.add_instructions(cex_inst);
        }

        cex_block
    }
}

pub type AnnotatedBlock = AnnotatedElement<Block, Annotation>;

impl AnnotatedBlock {
    /// Returns the index of this `AnnotatedBlock`
    pub fn index(&self) -> usize {
        self.element.index()
    }

    /// Returns the actual `AnnotatedBlock`.
    pub fn block(&self) -> &Block {
        &self.element
    }

    /// Returns a mutable reference to the actual `AnnotatedBlock`.
    pub fn block_mut(&mut self) -> &mut Block {
        &mut self.element
    }

    /// Returns whether this `AnnotatedBlock` is executed in any composition.
    pub fn executed(&self) -> bool {
        self.annotations
            .iter()
            .any(|(_, annotation)| annotation.executed())
    }

    /// Returns whether this `AnnotatedBlock` is transient.
    pub fn is_transient(&self) -> bool {
        self.element.is_transient()
    }
}

impl graph::Vertex for AnnotatedBlock {
    fn index(&self) -> usize {
        self.block().index()
    }

    fn dot_label(&self) -> String {
        format!("{}", self)
    }

    fn dot_fill_color(&self) -> String {
        match (self.executed(), self.block().is_transient()) {
            (true, true) => "#e1e1e1ff".to_string(),
            (true, false) => "#ffddccff".to_string(),
            (false, true) => "#e1e1e155".to_string(),
            (false, false) => "#ffddcc55".to_string(),
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

impl fmt::Display for AnnotatedBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[ Block: 0x{:X}", self.block().index())?;
        if self.block().is_transient() {
            write!(f, ", Transient")?;
        }
        writeln!(f, " ]")?;
        for instruction in self.block().instructions() {
            write!(f, "{}", instruction)?;
        }
        Ok(())
    }
}
