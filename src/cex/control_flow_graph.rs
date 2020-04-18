use crate::cex::{AnnotatedBlock, AnnotatedEdge};
use crate::error::Result;
use crate::util::RenderGraph;
use falcon::graph;
use std::fmt;

#[derive(Clone, Debug)]
pub struct ControlFlowGraph {
    // The internal graph used to store our blocks.
    graph: graph::Graph<AnnotatedBlock, AnnotatedEdge>,
}

impl ControlFlowGraph {
    pub fn new() -> Self {
        Self {
            graph: graph::Graph::new(),
        }
    }

    /// Returns the underlying graph
    pub fn graph(&self) -> &graph::Graph<AnnotatedBlock, AnnotatedEdge> {
        &self.graph
    }

    /// Get a `AnnotatedBlock` by index.
    pub fn block(&self, index: usize) -> Result<&AnnotatedBlock> {
        Ok(self.graph.vertex(index)?)
    }

    /// Get a mutable reference to a `AnnotatedBlock` by index.
    pub fn block_mut(&mut self, index: usize) -> Result<&mut AnnotatedBlock> {
        Ok(self.graph.vertex_mut(index)?)
    }

    /// Get every `AnnotatedBlock` in this `ControlFlowGraph`.
    pub fn blocks(&self) -> Vec<&AnnotatedBlock> {
        self.graph.vertices()
    }

    /// Get a mutable reference to every `AnnotatedBlock` in this `ControlFlowGraph`.
    pub fn blocks_mut(&mut self) -> Vec<&mut AnnotatedBlock> {
        self.graph.vertices_mut()
    }

    /// Get an `AnnotatedEdge` by its head and tail block indices.
    pub fn edge(&self, head: usize, tail: usize) -> Result<&AnnotatedEdge> {
        Ok(self.graph.edge(head, tail)?)
    }

    /// Get a mutable reference to an `AnnotatedEdge` by its head and tail block indices.
    pub fn edge_mut(&mut self, head: usize, tail: usize) -> Result<&mut AnnotatedEdge> {
        Ok(self.graph.edge_mut(head, tail)?)
    }

    /// Get every `AnnotatedEdge` in this `ControlFlowGraph`.
    pub fn edges(&self) -> Vec<&AnnotatedEdge> {
        self.graph.edges()
    }

    /// Get a mutable reference to every `AnnotatedEdge` in this `ControlFlowGraph`.
    pub fn edges_mut(&mut self) -> Vec<&mut AnnotatedEdge> {
        self.graph.edges_mut()
    }

    /// Adds the basic block to the graph
    pub fn add_block(&mut self, block: AnnotatedBlock) -> Result<()> {
        Ok(self.graph.insert_vertex(block)?)
    }

    /// Adds the edge to the graph
    pub fn add_edge(&mut self, edge: AnnotatedEdge) -> Result<()> {
        Ok(self.graph.insert_edge(edge)?)
    }
}

impl fmt::Display for ControlFlowGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for block in self.blocks() {
            writeln!(f, "{}", block)?;
        }
        for edge in self.edges() {
            writeln!(f, "edge {}", edge)?;
        }
        Ok(())
    }
}

impl RenderGraph for ControlFlowGraph {
    fn render_to_str(&self) -> String {
        self.graph().dot_graph()
    }
}
