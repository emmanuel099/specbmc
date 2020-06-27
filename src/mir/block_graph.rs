use crate::error::Result;
use crate::mir::{Block, Edge};
use crate::util::RenderGraph;
use falcon::graph::Graph;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct BlockGraph {
    graph: Graph<Block, Edge>,
    // The entry block index of the graph.
    entry: Option<usize>,
    // The exit block index of the graph.
    exit: Option<usize>,
}

impl BlockGraph {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            entry: None,
            exit: None,
        }
    }

    /// Returns the underlying graph
    pub fn graph(&self) -> &Graph<Block, Edge> {
        &self.graph
    }

    /// Get the entry `Block` index of this `BlockGraph`.
    pub fn entry(&self) -> Result<usize> {
        self.entry.ok_or_else(|| "CFG entry must be set".into())
    }

    /// Sets the entry point for this `BlockGraph` to the given `Block` index.
    pub fn set_entry(&mut self, entry: usize) -> Result<()> {
        if !self.graph.has_vertex(entry) {
            return Err("Index does not exist for set_entry".into());
        }
        self.entry = Some(entry);
        Ok(())
    }

    /// Get the exit `Block` index of this `BlockGraph`.
    pub fn exit(&self) -> Result<usize> {
        self.exit.ok_or_else(|| "CFG exit must be set".into())
    }

    /// Sets the exit point for this `BlockGraph` to the given `Block` index.
    pub fn set_exit(&mut self, exit: usize) -> Result<()> {
        if !self.graph.has_vertex(exit) {
            return Err("Index does not exist for set_exit".into());
        }
        self.exit = Some(exit);
        Ok(())
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

    /// Returns the entry block of this `BlockGraph`.
    pub fn entry_block(&self) -> Result<&Block> {
        self.entry().and_then(|entry| self.block(entry))
    }

    /// Returns a mutable reference to the entry block of this `BlockGraph`.
    pub fn entry_block_mut(&mut self) -> Result<&mut Block> {
        self.entry().and_then(move |entry| self.block_mut(entry))
    }

    /// Returns the exit block of this `BlockGraph`.
    pub fn exit_block(&self) -> Result<&Block> {
        self.exit().and_then(|exit| self.block(exit))
    }

    /// Returns a mutable reference to the exit block of this `BlockGraph`.
    pub fn exit_block_mut(&mut self) -> Result<&mut Block> {
        self.exit().and_then(move |exit| self.block_mut(exit))
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

impl Default for BlockGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for BlockGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for block in self.blocks() {
            writeln!(f, "{}", block)?;
        }
        Ok(())
    }
}

impl RenderGraph for BlockGraph {
    fn render_to_str(&self) -> String {
        self.graph().dot_graph()
    }
}
