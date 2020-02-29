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
    /// Is this block part of transient execution?
    transient: bool,
}

impl Block {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            instructions: Vec::new(),
            phi_nodes: Vec::new(),
            transient: false,
        }
    }

    /// Clone this block and set a new index.
    pub fn clone_new_index(&self, index: usize) -> Block {
        let mut clone = self.clone();
        clone.index = index;
        clone
    }

    /// Appends the contents of another `Block` to this `Block`.
    ///
    /// Instruction indices are updated accordingly.
    pub fn append(&mut self, other: &Block) {
        self.instructions.extend_from_slice(other.instructions());
        self.phi_nodes.extend_from_slice(other.phi_nodes());
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

    /// Sets whether this `Block` is part of transient execution or not.
    pub fn set_transient(&mut self, transient: bool) {
        self.transient = transient;
    }

    /// Returns whether this `Block` is part of transient execution or not.
    pub fn is_transient(&self) -> bool {
        self.transient
    }

    /// Returns instructions for this `Block`
    pub fn instructions(&self) -> &Vec<Instruction> {
        &self.instructions
    }

    /// Returns a mutable reference to the instructions for this `Block`.
    pub fn instructions_mut(&mut self) -> &mut Vec<Instruction> {
        &mut self.instructions
    }

    /// Overwrites the instructions of this `Block`
    pub fn set_instructions(&mut self, instructions: &Vec<Instruction>) {
        self.instructions.clone_from(instructions);
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

    /// Splits off the instructions at the given index.
    /// Only instructions with smaller index will remain in this `Block`.
    pub fn split_off_instructions_at(&mut self, index: usize) -> Result<Vec<Instruction>> {
        if index >= self.instructions.len() {
            return Err(format!("No instruction with index {} found", index).into());
        }
        Ok(self.instructions.split_off(index))
    }

    /// Returns the number of instructions in this `Block`.
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// Returns the number of instructions with distinct addresses in this `Block`.
    /// Meaning that two consecutive instructions with same address are actually counted as one instruction.
    pub fn instruction_count_by_address(&self) -> usize {
        let mut addresses: Vec<u64> = self
            .instructions
            .iter()
            .map(|inst| inst.address().unwrap_or_default())
            .collect();
        addresses.dedup();
        addresses.len()
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
    pub fn add_phi_node(&mut self, phi_node: PhiNode) -> &mut PhiNode {
        self.phi_nodes.push(phi_node);
        self.phi_nodes.last_mut().unwrap()
    }

    /// Adds an assign operation to the end of this block.
    pub fn assign(&mut self, variable: Variable, expr: Expression) -> &mut Instruction {
        self.instructions.push(Instruction::assign(variable, expr));
        self.instructions.last_mut().unwrap()
    }

    /// Adds a store operation to the end of this block.
    pub fn store(&mut self, address: Expression, expr: Expression) -> &mut Instruction {
        self.instructions.push(Instruction::store(address, expr));
        self.instructions.last_mut().unwrap()
    }

    /// Adds a load operation to the end of this block.
    pub fn load(&mut self, variable: Variable, address: Expression) -> &mut Instruction {
        self.instructions.push(Instruction::load(variable, address));
        self.instructions.last_mut().unwrap()
    }

    /// Adds a unconditional branch operation to the end of this block.
    pub fn branch(&mut self, target: Expression) -> &mut Instruction {
        self.instructions.push(Instruction::branch(target));
        self.instructions.last_mut().unwrap()
    }

    /// Adds a conditional branch operation to the end of this block.
    pub fn conditional_branch(
        &mut self,
        condition: Expression,
        target: Expression,
    ) -> &mut Instruction {
        self.instructions
            .push(Instruction::conditional_branch(condition, target));
        self.instructions.last_mut().unwrap()
    }

    /// Adds a barrier operation to the end of this block.
    pub fn barrier(&mut self) -> &mut Instruction {
        self.instructions.push(Instruction::barrier());
        self.instructions.last_mut().unwrap()
    }

    /// Adds an observe operation to the end of this block.
    pub fn observe(&mut self, variables: Vec<Variable>) -> &mut Instruction {
        self.instructions.push(Instruction::observe(variables));
        self.instructions.last_mut().unwrap()
    }

    /// Adds an indistinguishable operation to the end of this block.
    pub fn indistinguishable(&mut self, variables: Vec<Variable>) -> &mut Instruction {
        self.instructions
            .push(Instruction::indistinguishable(variables));
        self.instructions.last_mut().unwrap()
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
        if self.transient {
            "#e1e1e1".to_string()
        } else {
            "#ffddcc".to_string()
        }
    }

    fn dot_font_color(&self) -> String {
        "#000000".to_string()
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[ Block: 0x{:X}", self.index)?;
        if self.transient {
            write!(f, ", Transient")?;
        }
        writeln!(f, " ]")?;
        for phi_node in self.phi_nodes() {
            writeln!(f, "{}", phi_node)?;
        }
        for instruction in self.instructions() {
            writeln!(f, "{}", instruction)?;
        }
        Ok(())
    }
}
