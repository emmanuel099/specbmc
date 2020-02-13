use crate::error::Result;
use crate::expr::{Expression, Variable};
use crate::hir::{Instruction, PhiNode};
use falcon::graph;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Block {
    /// The index of the block.
    index: usize,
    /// The instructions for this block.
    instructions: Vec<Instruction>,
    /// The phi nodes for this block.
    phi_nodes: Vec<PhiNode>,
}

impl Block {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            instructions: Vec::new(),
            phi_nodes: Vec::new(),
        }
    }

    fn push(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    /// Appends the contents of another `Block` to this `Block`.
    ///
    /// Instruction indices are updated accordingly.
    pub fn append(&mut self, other: &Block) {
        other.instructions().into_iter().for_each(|instruction| {
            self.instructions.push(instruction.clone());
        });
        other.phi_nodes().into_iter().for_each(|phi_node| {
            self.phi_nodes.push(phi_node.clone());
        });
    }

    /// Get the address of the first instruction in this block
    pub fn address(&self) -> Option<u64> {
        self.instructions
            .first()
            .and_then(|instruction| instruction.address())
    }

    /// Returns the index of this `Block`
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns instructions for this `Block`
    pub fn instructions(&self) -> &Vec<Instruction> {
        &self.instructions
    }

    /// Returns a mutable reference to the instructions for this `Block`.
    pub fn instructions_mut(&mut self) -> &mut Vec<Instruction> {
        &mut self.instructions
    }

    /// Returns try if this `Block` is empty, meaning it has no `Instruction`
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Returns an `Instruction` by index, or `None` if the instruction does not exist.
    pub fn instruction(&self, index: usize) -> Option<&Instruction> {
        self.instructions.get(index)
    }

    /// Returns a mutable reference to an `Instruction` by index, or `None` if
    /// the `Instruction` does not exist.
    pub fn instruction_mut(&mut self, index: usize) -> Option<&mut Instruction> {
        self.instructions.get_mut(index)
    }

    /// Deletes an `Instruction` by its index.
    pub fn remove_instruction(&mut self, index: usize) -> Result<()> {
        if index >= self.instructions.len() {
            return Err(format!("No instruction with index {} found", index).into());
        }
        self.instructions.remove(index);
        Ok(())
    }

    /// Returns phi nodes of this `Block`
    pub fn phi_nodes(&self) -> &Vec<PhiNode> {
        &self.phi_nodes
    }

    /// Returns a mutable reference to the phi nodes of this `Block`.
    pub fn phi_nodes_mut(&mut self) -> &mut Vec<PhiNode> {
        &mut self.phi_nodes
    }

    /// Returns a `PhiNode` by index, or `None` if the `PhiNode` does not exist.
    pub fn phi_node(&self, index: usize) -> Option<&PhiNode> {
        self.phi_nodes.get(index)
    }

    /// Returns a mutable reference to a `PhiNode` by index, or `None` if
    /// the `PhiNode` does not exist.
    pub fn phi_node_mut(&mut self, index: usize) -> Option<&mut PhiNode> {
        self.phi_nodes.get_mut(index)
    }

    /// Adds the phi node to this `Block`.
    pub fn add_phi_node(&mut self, phi_node: PhiNode) {
        self.phi_nodes.push(phi_node);
    }

    /// Adds an assign operation to the end of this block.
    pub fn assign(&mut self, variable: Variable, expr: Expression) {
        self.push(Instruction::assign(variable, expr));
    }

    /// Adds a store operation to the end of this block.
    pub fn store(&mut self, memory: Variable, address: Expression, expr: Expression) {
        self.push(Instruction::store(memory, address, expr))
    }

    /// Adds a load operation to the end of this block.
    pub fn load(&mut self, variable: Variable, memory: Variable, address: Expression) {
        self.push(Instruction::load(variable, memory, address));
    }

    /// Adds a conditional branch operation to the end of this block.
    pub fn branch(&mut self, target: Expression) {
        self.push(Instruction::branch(target));
    }

    /// Adds a barrier operation to the end of this block.
    pub fn barrier(&mut self) {
        self.push(Instruction::barrier());
    }
}

impl graph::Vertex for Block {
    fn index(&self) -> usize {
        self.index
    }

    fn dot_label(&self) -> String {
        format!("{}", self)
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "[ Block: 0x{:X} ]", self.index)?;
        for phi_node in self.phi_nodes() {
            writeln!(f, "{}", phi_node)?;
        }
        for instruction in self.instructions() {
            writeln!(f, "{}", instruction)?;
        }
        Ok(())
    }
}
