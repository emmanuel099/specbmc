use crate::expr::{Boolean, Expression, Sort, Variable};
use crate::mir::Node;
use falcon::graph;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Block {
    /// The index of this block.
    index: usize,
    /// The instructions for this block.
    nodes: Vec<Node>,
    /// The execution condition of this block.
    execution_condition: Expression,
}

impl Block {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            nodes: Vec::new(),
            execution_condition: Boolean::constant(false),
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

    /// The variable which refers to the execution condition of this block.
    /// Use the variable instead of the expression to reduce the length of the formulas.
    pub fn execution_condition_variable(&self) -> Variable {
        Self::execution_condition_variable_for_index(self.index)
    }

    pub fn execution_condition_variable_for_index(index: usize) -> Variable {
        Variable::new(format!("_exec_{}", index), Sort::boolean())
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.push(node);
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Block 0x{:X} [{} = {}]",
            self.index,
            self.execution_condition_variable(),
            self.execution_condition
        )?;
        for node in self.nodes() {
            writeln!(f, "{}", node)?;
        }
        Ok(())
    }
}
