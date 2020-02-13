use crate::error::Result;
use crate::expr::{Boolean, Expression, Sort, Variable};
use crate::mir::{Node, Operation};
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
    /// The variable which refers to the execution condition of this block.
    /// Use the variable instead of the expression to reduce the length of the formulas.
    execution_condition_variable: Variable,
}

impl Block {
    pub fn new(index: usize) -> Self {
        Block {
            index,
            nodes: Vec::new(),
            execution_condition: Boolean::constant(false),
            execution_condition_variable: Variable::new(
                format!("_exec_{}", index),
                Sort::boolean(),
            ),
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

    pub fn execution_condition_variable(&self) -> &Variable {
        &self.execution_condition_variable
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.push(node);
    }

    pub fn add_let(&mut self, var: Variable, expr: Expression) -> Result<&mut Node> {
        self.add_node(Node::new(Operation::new_let(var, expr)?));
        Ok(self.nodes.last_mut().unwrap())
    }

    pub fn add_assert(&mut self, cond: Expression) -> Result<&mut Node> {
        self.add_node(Node::new(Operation::new_assert(cond)?));
        Ok(self.nodes.last_mut().unwrap())
    }

    pub fn add_assume(&mut self, cond: Expression) -> Result<&mut Node> {
        self.add_node(Node::new(Operation::new_assume(cond)?));
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
        writeln!(
            f,
            "Block 0x{:X} [{} = {}]",
            self.index, self.execution_condition_variable, self.execution_condition
        )?;
        for node in self.nodes() {
            writeln!(f, "{}", node)?;
        }
        Ok(())
    }
}
