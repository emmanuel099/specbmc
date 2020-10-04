use crate::error::Result;
use crate::expr::{Expression, Variable};
use crate::hir::{Instruction, PhiNode};
use falcon::graph;
use std::cmp;
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
    /// If this block is a loop header, the loop id will be tracked
    loop_id: Option<usize>,
}

impl Block {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            instructions: Vec::new(),
            phi_nodes: Vec::new(),
            transient: false,
            loop_id: None,
        }
    }

    /// Clone this block and set a new index.
    pub fn clone_new_index(&self, index: usize) -> Self {
        let mut clone = self.clone();
        clone.index = index;
        clone
    }

    /// Appends the contents of another `Block` to this `Block`.
    ///
    /// Instruction indices are updated accordingly.
    pub fn append(&mut self, other: &Self) {
        self.instructions.extend_from_slice(other.instructions());
        self.phi_nodes.extend_from_slice(other.phi_nodes());
    }

    /// Get the address of the first instruction in this block
    pub fn address(&self) -> Option<u64> {
        self.instructions.first().and_then(Instruction::address)
    }

    /// Returns the index of this `Block`
    pub fn index(&self) -> usize {
        self.index
    }

    /// Marks this `Block` as transient.
    pub fn set_transient(&mut self) {
        self.transient = true;
    }

    /// Returns whether this `Block` is part of transient execution or not.
    pub fn is_transient(&self) -> bool {
        self.transient
    }

    /// Set the loop id of this `Block`.
    /// None if the block is not a loop header.
    pub fn set_loop_id(&mut self, loop_id: Option<usize>) {
        self.loop_id = loop_id;
    }

    /// Returns the loop id of this `Block`.
    /// None if the block is not a loop header.
    pub fn loop_id(&self) -> Option<usize> {
        self.loop_id
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
    pub fn set_instructions(&mut self, instructions: &[Instruction]) {
        self.instructions = instructions.to_owned();
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
    pub fn remove_instruction(&mut self, index: usize) -> Result<Instruction> {
        if index >= self.instructions.len() {
            return Err(format!("No instruction with index {} found", index).into());
        }
        Ok(self.instructions.remove(index))
    }

    /// Deletes multiple `Instruction`s by their indices.
    pub fn remove_instructions(&mut self, indices: &[usize]) -> Result<()> {
        let mut indices = indices.to_owned();
        indices.sort();
        for index in indices.into_iter().rev() {
            self.remove_instruction(index)?;
        }
        Ok(())
    }

    /// Inserts an `Instruction` at the given index.
    pub fn insert_instruction(&mut self, index: usize, instruction: Instruction) -> Result<()> {
        if index > self.instructions.len() {
            return Err(format!("Index {} is invalid", index).into());
        }
        self.instructions.insert(index, instruction);
        Ok(())
    }

    /// Inserts multiple instructions at the given indices.
    pub fn insert_instructions(
        &mut self,
        indexed_instructions: Vec<(usize, Instruction)>,
    ) -> Result<()> {
        let mut sorted_instructions = indexed_instructions;
        sorted_instructions.sort_by_key(|&(index, _)| cmp::Reverse(index));
        for (index, inst) in sorted_instructions.into_iter() {
            self.insert_instruction(index, inst)?;
        }
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

    /// Returns the number of real instructions (i.e. instructions not marked as pseudo) in this `Block`.
    pub fn instruction_count_ignoring_pseudo_instructions(&self) -> usize {
        self.instructions
            .iter()
            .filter(|inst| !inst.labels().is_pseudo() && !inst.labels().is_helper())
            .count()
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

    /// Deletes a `PhiNode` by its index.
    pub fn remove_phi_node(&mut self, index: usize) -> Result<PhiNode> {
        if index >= self.phi_nodes.len() {
            return Err(format!("No phi node with index {} found", index).into());
        }
        Ok(self.phi_nodes.remove(index))
    }

    /// Deletes multiple `PhiNode`s by their indices.
    pub fn remove_phi_nodes(&mut self, indices: &[usize]) -> Result<()> {
        let mut indices = indices.to_owned();
        indices.sort();
        for index in indices.into_iter().rev() {
            self.remove_phi_node(index)?;
        }
        Ok(())
    }

    /// Adds an assign operation to the end of this block.
    pub fn assign(&mut self, variable: Variable, expr: Expression) -> Result<&mut Instruction> {
        self.instructions.push(Instruction::assign(variable, expr)?);
        Ok(self.instructions.last_mut().unwrap())
    }

    /// Adds a store operation to the end of this block.
    pub fn store(&mut self, address: Expression, expr: Expression) -> Result<&mut Instruction> {
        self.instructions.push(Instruction::store(address, expr)?);
        Ok(self.instructions.last_mut().unwrap())
    }

    /// Adds a load operation to the end of this block.
    pub fn load(&mut self, variable: Variable, address: Expression) -> Result<&mut Instruction> {
        self.instructions
            .push(Instruction::load(variable, address)?);
        Ok(self.instructions.last_mut().unwrap())
    }

    /// Adds a call operation to the end of this block.
    pub fn call(&mut self, target: Expression) -> Result<&mut Instruction> {
        self.instructions.push(Instruction::call(target)?);
        Ok(self.instructions.last_mut().unwrap())
    }

    /// Adds a unconditional branch operation to the end of this block.
    pub fn branch(&mut self, target: Expression) -> Result<&mut Instruction> {
        self.instructions.push(Instruction::branch(target)?);
        Ok(self.instructions.last_mut().unwrap())
    }

    /// Adds a conditional branch operation to the end of this block.
    pub fn conditional_branch(
        &mut self,
        condition: Expression,
        target: Expression,
    ) -> Result<&mut Instruction> {
        self.instructions
            .push(Instruction::conditional_branch(condition, target)?);
        Ok(self.instructions.last_mut().unwrap())
    }

    /// Adds a skip operation to the end of this block.
    pub fn skip(&mut self) -> &mut Instruction {
        self.instructions.push(Instruction::skip());
        self.instructions.last_mut().unwrap()
    }

    /// Adds a barrier operation to the end of this block.
    pub fn barrier(&mut self) -> &mut Instruction {
        self.instructions.push(Instruction::barrier());
        self.instructions.last_mut().unwrap()
    }

    /// Adds an assert operation to the end of this block.
    pub fn assert(&mut self, condition: Expression) -> Result<&mut Instruction> {
        self.instructions.push(Instruction::assert(condition)?);
        Ok(self.instructions.last_mut().unwrap())
    }

    /// Adds an assume operation to the end of this block.
    pub fn assume(&mut self, condition: Expression) -> Result<&mut Instruction> {
        self.instructions.push(Instruction::assume(condition)?);
        Ok(self.instructions.last_mut().unwrap())
    }

    /// Adds an observable operation to the end of this block.
    pub fn observable(&mut self, expr: Expression) -> &mut Instruction {
        self.instructions.push(Instruction::observable(expr));
        self.instructions.last_mut().unwrap()
    }

    /// Adds an indistinguishable operation to the end of this block.
    pub fn indistinguishable(&mut self, expr: Expression) -> &mut Instruction {
        self.instructions.push(Instruction::indistinguishable(expr));
        self.instructions.last_mut().unwrap()
    }

    /// Get the variables written by this `Block`.
    pub fn variables_written(&self) -> Vec<&Variable> {
        self.instructions
            .iter()
            .flat_map(Instruction::variables_written)
            .chain(self.phi_nodes.iter().map(PhiNode::out))
            .collect()
    }

    /// Get a mutable reference to the variables written by this `Block`.
    pub fn variables_written_mut(&mut self) -> Vec<&mut Variable> {
        self.instructions
            .iter_mut()
            .flat_map(Instruction::variables_written_mut)
            .chain(self.phi_nodes.iter_mut().map(PhiNode::out_mut))
            .collect()
    }

    /// Get the variables read by this `Block`.
    pub fn variables_read(&self) -> Vec<&Variable> {
        self.instructions
            .iter()
            .flat_map(Instruction::variables_read)
            .chain(self.phi_nodes.iter().flat_map(PhiNode::incoming_variables))
            .collect()
    }

    /// Get a mutable reference to the variables read by this `Block`.
    pub fn variables_read_mut(&mut self) -> Vec<&mut Variable> {
        self.instructions
            .iter_mut()
            .flat_map(Instruction::variables_read_mut)
            .chain(
                self.phi_nodes
                    .iter_mut()
                    .flat_map(PhiNode::incoming_variables_mut),
            )
            .collect()
    }

    /// Get each `Variable` used by this `Block`.
    pub fn variables(&self) -> Vec<&Variable> {
        self.variables_read()
            .into_iter()
            .chain(self.variables_written().into_iter())
            .collect()
    }

    /// Get each `Expression` of this `Block`.
    pub fn expressions(&self) -> Vec<&Expression> {
        self.instructions
            .iter()
            .flat_map(Instruction::expressions)
            .collect()
    }

    /// Get a mutable reference to each `Expression` of this `Block`.
    pub fn expressions_mut(&mut self) -> Vec<&mut Expression> {
        self.instructions
            .iter_mut()
            .flat_map(Instruction::expressions_mut)
            .collect()
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[ Block: 0x{:X}", self.index)?;
        if self.transient {
            write!(f, ", Transient")?;
        }
        if let Some(id) = self.loop_id {
            write!(f, ", Loop 0x{:X}", id)?;
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
