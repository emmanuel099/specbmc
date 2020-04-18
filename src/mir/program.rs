use crate::error::Result;
use crate::mir::BlockGraph;
use crate::util::TranslateInto;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Program {
    block_graph: BlockGraph,
    self_compositions: usize, // The number of required self-compositions
}

impl Program {
    pub fn new(block_graph: BlockGraph, self_compositions: usize) -> Self {
        Self {
            block_graph,
            self_compositions,
        }
    }

    pub fn from<Src: TranslateInto<Self>>(src: &Src) -> Result<Self> {
        src.translate_into()
    }

    pub fn block_graph(&self) -> &BlockGraph {
        &self.block_graph
    }

    pub fn block_graph_mut(&mut self) -> &mut BlockGraph {
        &mut self.block_graph
    }

    pub fn self_compositions(&self) -> usize {
        self.self_compositions
    }
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.block_graph().fmt(f)
    }
}
