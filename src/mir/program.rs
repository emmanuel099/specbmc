use crate::mir::BlockGraph;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Program {
    block_graph: BlockGraph,
}

impl Program {
    pub fn new(block_graph: BlockGraph) -> Self {
        Self { block_graph }
    }

    pub fn block_graph(&self) -> &BlockGraph {
        &self.block_graph
    }

    pub fn block_graph_mut(&mut self) -> &mut BlockGraph {
        &mut self.block_graph
    }
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.block_graph().fmt(f)
    }
}
