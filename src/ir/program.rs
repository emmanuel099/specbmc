use crate::ir::BlockGraph;

#[derive(Clone, Debug)]
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
}
