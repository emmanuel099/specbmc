use crate::ir::Node;
use falcon::graph;
use std::fmt;

#[derive(Clone, Debug)]
pub struct Block {
    /// The index of this block.
    index: usize,
    /// The instructions for this block.
    nodes: Vec<Node>,
    // The execution condition of this block.
    //execution_condition: Expression,
}

impl Block {
    pub fn new(index: usize) -> Self {
        Block {
            index,
            nodes: Vec::new(),
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
        for node in self.nodes() {
            writeln!(f, "{}", node)?;
        }
        Ok(())
    }
}
