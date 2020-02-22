use crate::error::Result;
use crate::expr::{Expression, Variable};
use crate::lir::Node;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Program {
    nodes: Vec<Node>,
}

impl Program {
    pub fn new() -> Self {
        Self { nodes: vec![] }
    }

    pub fn append_node(&mut self, node: Node) {
        self.nodes.push(node);
    }

    pub fn append_comment<S>(&mut self, text: S)
    where
        S: Into<String>,
    {
        self.append_node(Node::new_comment(text));
    }

    pub fn append_let(&mut self, var: Variable, expr: Expression) -> Result<()> {
        self.append_node(Node::new_let(var, expr)?);
        Ok(())
    }

    pub fn append_assert(&mut self, cond: Expression) -> Result<()> {
        self.append_node(Node::new_assert(cond)?);
        Ok(())
    }

    pub fn append_assume(&mut self, cond: Expression) -> Result<()> {
        self.append_node(Node::new_assume(cond)?);
        Ok(())
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut Vec<Node> {
        &mut self.nodes
    }
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for node in self.nodes() {
            writeln!(f, "{}", node)?;
        }
        Ok(())
    }
}
