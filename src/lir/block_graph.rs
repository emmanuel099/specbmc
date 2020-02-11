use crate::error::*;
use crate::lir::{Block, Edge};
use falcon::graph::Graph;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct BlockGraph {
    graph: Graph<Block, Edge>,
    entry: Option<usize>,
}

impl BlockGraph {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            entry: None,
        }
    }

    /// Returns the underlying graph
    pub fn graph(&self) -> &Graph<Block, Edge> {
        &self.graph
    }

    /// Sets the entry point for this `BlockGraph` to the given `Block` index.
    pub fn set_entry(&mut self, entry: usize) -> Result<()> {
        if self.graph.has_vertex(entry) {
            self.entry = Some(entry);
            return Ok(());
        }
        Err("Index does not exist for set_entry".into())
    }

    /// Get the entry `Block` index for this `BlockGraph`.
    pub fn entry(&self) -> Option<usize> {
        self.entry
    }

    /// Get a `Block` by index.
    pub fn block(&self, index: usize) -> Result<&Block> {
        Ok(self.graph.vertex(index)?)
    }

    /// Get a mutable reference to a `Block` by index.
    pub fn block_mut(&mut self, index: usize) -> Result<&mut Block> {
        Ok(self.graph.vertex_mut(index)?)
    }

    /// Get every `Block` in this `BlockGraph`.
    pub fn blocks(&self) -> Vec<&Block> {
        self.graph.vertices()
    }

    /// Get a mutable reference to every `Block` in this `BlockGraph`.
    pub fn blocks_mut(&mut self) -> Vec<&mut Block> {
        self.graph.vertices_mut()
    }

    /// Returns the entry block for this `BlockGraph`.
    pub fn entry_block(&self) -> Option<Result<&Block>> {
        if self.entry.is_none() {
            None
        } else {
            Some(self.block(self.entry.unwrap()))
        }
    }

    /// Get the indices of every successor of a `Block` in this `BlockGraph`.
    pub fn successor_indices(&self, index: usize) -> Result<Vec<usize>> {
        Ok(self.graph.successor_indices(index)?)
    }

    /// Adds the basic block to the graph
    pub fn add_block(&mut self, block: Block) -> Result<()> {
        Ok(self.graph.insert_vertex(block)?)
    }

    /// Creates an edge from one block to another block
    pub fn add_edge(&mut self, head: usize, tail: usize) -> Result<()> {
        let edge = Edge::new(head, tail);
        Ok(self.graph.insert_edge(edge)?)
    }
}

impl fmt::Display for BlockGraph {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for block in self.blocks() {
            writeln!(f, "{}", block)?;
        }
        Ok(())
    }
}
