use crate::error::Result;
use crate::ir::{Boolean, Expression, Node, Operation, Variable};
use falcon::graph;
use std::fmt;

#[derive(Clone, Debug)]
pub struct Block {
    /// The index of this block.
    index: usize,
    /// The instructions for this block.
    nodes: Vec<Node>,
    // The execution condition of this block.
    execution_condition: Expression,
}

impl Block {
    pub fn new(index: usize) -> Self {
        Block {
            index,
            nodes: Vec::new(),
            execution_condition: Boolean::constant(false).into(),
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut [Node] {
        &mut self.nodes
    }

    pub fn execution_condition(&self) -> &Expression {
        &self.execution_condition
    }

    pub fn set_execution_condition(&mut self, expr: Expression) {
        self.execution_condition = expr;
    }

    pub fn add_let(&mut self, var: Variable, expr: Expression) -> Result<&mut Node> {
        self.nodes.push(Node::new(Operation::new_let(var, expr)?));
        Ok(self.nodes.last_mut().unwrap())
    }

    pub fn add_assert(&mut self, cond: Expression) -> Result<&mut Node> {
        self.nodes.push(Node::new(Operation::new_assert(cond)?));
        Ok(self.nodes.last_mut().unwrap())
    }

    pub fn add_assume(&mut self, cond: Expression) -> Result<&mut Node> {
        self.nodes.push(Node::new(Operation::new_assume(cond)?));
        Ok(self.nodes.last_mut().unwrap())
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
        writeln!(f, "Block 0x{:X} [{}]", self.index, self.execution_condition)?;
        for node in self.nodes() {
            writeln!(f, "{}", node)?;
        }
        Ok(())
    }
}
