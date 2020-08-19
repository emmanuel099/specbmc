//! A `ControlFlowGraph` is a directed `Graph` of `Block` and `Edge`.

use crate::error::Result;
use crate::expr::{Boolean, Expression, Variable};
use crate::hir::{Block, Edge};
use crate::util::RenderGraph;
use falcon::graph;
use std::cmp;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;

#[derive(Clone, Copy, Debug)]
pub enum RemovedEdgeGuard {
    Ignore,
    AssumeEdgeNotTaken,
    AssertEdgeNotTaken,
}

/// A directed graph of types `Block` and `Edge`.
///
/// # Entry and Exit
/// A `ControlFlowGraph` has an optional, "Entry," and an optional, "Exit." When these are
/// provided, certain convenience functions become available.
#[derive(Clone, Debug, Derivative)]
#[derivative(Hash, Eq, PartialEq)]
pub struct ControlFlowGraph {
    // The internal graph used to store our blocks.
    graph: graph::Graph<Block, Edge>,
    // An optional entry index for the graph.
    entry: Option<usize>,
    // An optional exit index for the graph.
    exit: Option<usize>,
    // The next index to use when creating a basic block.
    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    next_index: usize,
}

impl ControlFlowGraph {
    pub fn new() -> Self {
        Self {
            graph: graph::Graph::new(),
            next_index: 0,
            entry: None,
            exit: None,
        }
    }

    /// Returns the underlying graph
    pub fn graph(&self) -> &graph::Graph<Block, Edge> {
        &self.graph
    }

    /// Get the entry `Block` index of this `ControlFlowGraph`.
    pub fn entry(&self) -> Result<usize> {
        self.entry.ok_or_else(|| "CFG entry must be set".into())
    }

    /// Sets the entry point for this `ControlFlowGraph` to the given `Block` index.
    pub fn set_entry(&mut self, entry: usize) -> Result<()> {
        if !self.graph.has_vertex(entry) {
            return Err("Index does not exist for set_entry".into());
        }
        self.entry = Some(entry);
        Ok(())
    }

    /// Get the exit `Block` index of this `ControlFlowGraph`.
    pub fn exit(&self) -> Result<usize> {
        self.exit.ok_or_else(|| "CFG exit must be set".into())
    }

    /// Sets the exit point for this `ControlFlowGraph` to the given `Block` index.
    pub fn set_exit(&mut self, exit: usize) -> Result<()> {
        if !self.graph.has_vertex(exit) {
            return Err("Index does not exist for set_exit".into());
        }
        self.exit = Some(exit);
        Ok(())
    }

    /// Get the indices of every predecessor of a `Block` in this `ControlFlowGraph`.
    pub fn predecessor_indices(&self, index: usize) -> Result<Vec<usize>> {
        Ok(self.graph.predecessor_indices(index)?)
    }

    /// Get the indices of every successor of a `Block` in this `ControlFlowGraph`.
    pub fn successor_indices(&self, index: usize) -> Result<Vec<usize>> {
        Ok(self.graph.successor_indices(index)?)
    }

    /// Get a `Block` by index.
    pub fn block(&self, index: usize) -> Result<&Block> {
        Ok(self.graph.vertex(index)?)
    }

    /// Get a mutable reference to a `Block` by index.
    pub fn block_mut(&mut self, index: usize) -> Result<&mut Block> {
        Ok(self.graph.vertex_mut(index)?)
    }

    /// Get every `Block` in this `ControlFlowGraph`.
    pub fn blocks(&self) -> Vec<&Block> {
        self.graph.vertices()
    }

    /// Get a mutable reference to every `Block` in this `ControlFlowGraph`.
    pub fn blocks_mut(&mut self) -> Vec<&mut Block> {
        self.graph.vertices_mut()
    }

    /// Returns the entry block of this `ControlFlowGraph`.
    pub fn entry_block(&self) -> Result<&Block> {
        self.entry().and_then(|entry| self.block(entry))
    }

    /// Returns a mutable reference to the entry block of this `ControlFlowGraph`.
    pub fn entry_block_mut(&mut self) -> Result<&mut Block> {
        self.entry().and_then(move |entry| self.block_mut(entry))
    }

    /// Returns the exit block of this `ControlFlowGraph`.
    pub fn exit_block(&self) -> Result<&Block> {
        self.exit().and_then(|exit| self.block(exit))
    }

    /// Returns a mutable reference to the exit block of this `ControlFlowGraph`.
    pub fn exit_block_mut(&mut self) -> Result<&mut Block> {
        self.exit().and_then(move |exit| self.block_mut(exit))
    }

    /// Creates a new basic block, adds it to the graph, and returns it
    pub fn new_block(&mut self) -> &mut Block {
        let next_index = self.next_index;
        self.next_index += 1;
        let block = Block::new(next_index);
        self.graph.insert_vertex(block).unwrap();
        self.graph.vertex_mut(next_index).unwrap()
    }

    /// Clones an existing basic block, adds it to the graph, and returns it
    pub fn duplicate_block(&mut self, index: usize) -> Result<&mut Block> {
        let next_index = self.next_index;
        self.next_index += 1;
        let block = self.block(index)?.clone_new_index(next_index);
        self.graph.insert_vertex(block)?;
        Ok(self.graph.vertex_mut(next_index).unwrap())
    }

    /// Duplicates the blocks with the given indices, including their outgoing edges,
    /// and returns the mapping from the old to the new block indices for the duplicated blocks.
    pub fn duplicate_blocks(
        &mut self,
        block_indices: &BTreeSet<usize>,
    ) -> Result<BTreeMap<usize, usize>> {
        let mut block_map: BTreeMap<usize, usize> = BTreeMap::new();

        for &index in block_indices {
            let duplicated_block = self.duplicate_block(index)?;
            block_map.insert(index, duplicated_block.index());
        }

        let mut new_edges: Vec<Edge> = Vec::new();

        for &index in block_indices {
            for edge in self.edges_out(index)? {
                let new_head = block_map
                    .get(&edge.head())
                    .cloned()
                    .unwrap_or_else(|| edge.head());
                let new_tail = block_map
                    .get(&edge.tail())
                    .cloned()
                    .unwrap_or_else(|| edge.tail());
                new_edges.push(edge.clone_new_head_tail(new_head, new_tail));
            }
        }

        for edge in new_edges {
            self.graph.insert_edge(edge)?;
        }

        Ok(block_map)
    }

    /// Adds the basic block to the graph
    pub fn add_block(&mut self, block: Block) -> Result<()> {
        self.next_index = cmp::max(block.index() + 1, self.next_index);
        Ok(self.graph.insert_vertex(block)?)
    }

    /// Returns true if the block with the given index exists.
    pub fn has_block(&self, index: usize) -> bool {
        self.graph.has_vertex(index)
    }

    /// Removes an `Block` by its index.
    pub fn remove_block(
        &mut self,
        index: usize,
        removed_edge_guard: RemovedEdgeGuard,
    ) -> Result<Block> {
        if self.entry == Some(index) {
            self.entry = None;
        }
        if self.exit == Some(index) {
            self.exit = None;
        }

        // Remove all incoming edges
        for predecessor in self.predecessor_indices(index)? {
            self.remove_edge(predecessor, index, removed_edge_guard)?;
        }

        let block = self.block(index)?.clone();
        self.graph.remove_vertex(index)?;
        Ok(block)
    }

    /// Splits the block with the given index at the specified instruction.
    /// Outgoing edges will be rewired to the new block.
    ///
    /// "Instruction index = 0" will give an empty top block.
    /// "Instruction index = instruction count" will give an empty tail block.
    ///
    /// Doesn't add a new edge between the cut-up blocks!
    pub fn split_block_at(
        &mut self,
        block_index: usize,
        instruction_index: usize,
    ) -> Result<usize> {
        let tail_instructions = {
            let top_block = self.block_mut(block_index)?;
            if top_block.instructions().len() == instruction_index {
                Vec::default()
            } else {
                top_block.split_off_instructions_at(instruction_index)?
            }
        };

        let tail_block_index = {
            let tail_block = self.new_block();
            tail_block.set_instructions(&tail_instructions);
            tail_block.index()
        };

        for successor in self.successor_indices(block_index)? {
            self.rewire_edge(block_index, successor, tail_block_index, successor)?;
        }

        if self.exit == Some(block_index) {
            self.set_exit(tail_block_index)?;
        }

        Ok(tail_block_index)
    }

    /// Splits the block with the given index before the first instruction,
    /// meaning that the resulting top block will be empty.
    ///
    /// Doesn't add a new edge between the cut-up blocks!
    pub fn split_block_at_begin(&mut self, block_index: usize) -> Result<usize> {
        self.split_block_at(block_index, 0)
    }

    /// Splits the block with the given index after the last instruction,
    /// meaning that the resulting tail block will be empty.
    ///
    /// Outgoing edges will be rewired to the new tail block.
    /// Doesn't add a new edge between the cut-up blocks!
    pub fn split_block_at_end(&mut self, block_index: usize) -> Result<usize> {
        let instruction_count = self.block(block_index)?.instruction_count();
        self.split_block_at(block_index, instruction_count)
    }

    /// Get an `Edge` by its head and tail `Block` indices.
    pub fn edge(&self, head: usize, tail: usize) -> Result<&Edge> {
        Ok(self.graph.edge(head, tail)?)
    }

    /// Get a mutable reference to an `Edge` by its head and tail `Block` indices.
    pub fn edge_mut(&mut self, head: usize, tail: usize) -> Result<&mut Edge> {
        Ok(self.graph.edge_mut(head, tail)?)
    }

    /// Get every `Edge` in thie `ControlFlowGraph`.
    pub fn edges(&self) -> Vec<&Edge> {
        self.graph.edges()
    }

    /// Get a mutable reference to every `Edge` in this `ControlFlowGraph`.
    pub fn edges_mut(&mut self) -> Vec<&mut Edge> {
        self.graph.edges_mut()
    }

    /// Get every incoming edge to a block
    pub fn edges_in(&self, index: usize) -> Result<Vec<&Edge>> {
        Ok(self.graph.edges_in(index)?)
    }

    /// Get every outgoing edge from a block
    pub fn edges_out(&self, index: usize) -> Result<Vec<&Edge>> {
        Ok(self.graph.edges_out(index)?)
    }

    /// Creates an unconditional edge from one block to another block
    pub fn unconditional_edge(&mut self, head: usize, tail: usize) -> Result<&mut Edge> {
        let edge = Edge::new(head, tail, None);
        self.graph.insert_edge(edge)?;
        Ok(self.graph.edge_mut(head, tail)?)
    }

    /// Creates a conditional edge from one block to another block
    pub fn conditional_edge(
        &mut self,
        head: usize,
        tail: usize,
        condition: Expression,
    ) -> Result<&mut Edge> {
        let edge = Edge::new(head, tail, Some(condition));
        self.graph.insert_edge(edge)?;
        Ok(self.graph.edge_mut(head, tail)?)
    }

    /// Returns true if the edge with the given head and tail index exists.
    pub fn has_edge(&self, head: usize, tail: usize) -> bool {
        self.graph.has_edge(head, tail)
    }

    /// Removes an `Edge` by its head and tail `Block` indices.
    pub fn remove_edge(
        &mut self,
        head: usize,
        tail: usize,
        removed_edge_guard: RemovedEdgeGuard,
    ) -> Result<Edge> {
        let edge = self.edge(head, tail)?.clone();

        // Add "negated condition" assumption/assertion for removed conditional edges
        // to make sure that the conditional edges aren't taken anymore.
        if let Some(condition) = edge.condition() {
            let predecessor = self.block_mut(head)?;
            let negated_condition = Boolean::not(condition.clone())?;
            match removed_edge_guard {
                RemovedEdgeGuard::AssumeEdgeNotTaken => {
                    predecessor.assume(negated_condition)?.labels_mut().pseudo();
                }
                RemovedEdgeGuard::AssertEdgeNotTaken => {
                    predecessor.assert(negated_condition)?.labels_mut().pseudo();
                }
                RemovedEdgeGuard::Ignore => {}
            }
        }

        self.graph.remove_edge(head, tail)?;
        Ok(edge)
    }

    /// Rewires an `Edge` from its current head and tail `Block`s to the new head and tail `Block`s.
    pub fn rewire_edge(
        &mut self,
        head: usize,
        tail: usize,
        new_head: usize,
        new_tail: usize,
    ) -> Result<()> {
        let edge = self.edge(head, tail)?;
        let new_edge = edge.clone_new_head_tail(new_head, new_tail);
        self.remove_edge(head, tail, RemovedEdgeGuard::Ignore)?;
        self.graph.insert_edge(new_edge)?;
        Ok(())
    }

    /// Appends a control flow graph to this control flow graph.
    ///
    /// In order for this to work, the entry and exit of boths graphs must be
    /// set, which should be the case for all conformant translators. You can
    /// also append to an empty ControlFlowGraph.
    pub fn append(&mut self, other: &Self) -> Result<()> {
        let is_empty = match self.graph.num_vertices() {
            0 => true,
            _ => false,
        };

        // Bring in new blocks
        let block_map = self.insert(other)?;

        if is_empty {
            self.entry = Some(block_map[&other.entry()?]);
        } else {
            // Create an edge from the exit of this graph to the head of the other graph
            self.unconditional_edge(self.exit()?, block_map[&(other.entry()?)])?;
        }

        self.exit = Some(block_map[&other.exit()?]);

        Ok(())
    }

    /// Inserts a control flow graph into this control flow graph, and returns
    /// the mapping from the old to the new block indices for the inserted graph.
    ///
    /// This function causes the `ControlFlowGraph` to become disconnected.
    pub fn insert(&mut self, other: &Self) -> Result<BTreeMap<usize, usize>> {
        // keep track of mapping between old indices and new indices
        let mut block_map: BTreeMap<usize, usize> = BTreeMap::new();

        // insert all the blocks
        for block in other.graph().vertices() {
            let new_block = block.clone_new_index(self.next_index);
            block_map.insert(block.index(), self.next_index);
            self.next_index += 1;
            self.graph.insert_vertex(new_block)?;
        }

        // insert edges
        for edge in other.graph().edges() {
            let new_head = block_map[&edge.head()];
            let new_tail = block_map[&edge.tail()];
            self.graph
                .insert_edge(edge.clone_new_head_tail(new_head, new_tail))?;
        }

        Ok(block_map)
    }

    /// Removes all blocks which are unreachable from CFG entry.
    fn remove_unreachable_blocks(&mut self) -> Result<()> {
        let entry = self.entry()?;
        self.graph.remove_unreachable_vertices(entry)?;
        Ok(())
    }

    /// Merges all blocks which only have one successor, and that successor has only one predecessor.
    fn merge_consecutive_blocks_with_single_successor_and_predecessor(&mut self) -> Result<()> {
        loop {
            let mut blocks_being_merged: HashSet<usize> = HashSet::new();
            let mut merges: Vec<(usize, usize)> = Vec::new();

            for block in self.blocks() {
                // If we are already merging this block this iteration, skip it
                if blocks_being_merged.contains(&block.index()) {
                    continue;
                }

                // Do not merge the entry block
                if self.entry == Some(block.index()) {
                    continue;
                }

                // If we do not have just one successor, we will not merge this block
                let outgoing_edges = self.edges_out(block.index())?;
                if outgoing_edges.len() != 1 {
                    continue;
                }

                // If this successor has a condition, we will not merge this block
                let outgoing_edge = outgoing_edges[0];
                if outgoing_edge.is_conditional() {
                    continue;
                }

                // If this successor is already being merged, skip it
                let successor = outgoing_edge.tail();
                if blocks_being_merged.contains(&successor) {
                    continue;
                }

                // Do not merge the exit block
                if self.exit == Some(successor) {
                    continue;
                }

                // If successor does not have just one predecessor, we will not merge this block
                let successor_incoming_edges = self.graph.edges_in(successor).unwrap();
                if successor_incoming_edges.len() != 1 {
                    continue;
                }

                blocks_being_merged.insert(block.index());
                blocks_being_merged.insert(successor);

                merges.push((block.index(), successor));
            }

            if merges.is_empty() {
                break;
            }

            for (merge_index, successor_index) in merges {
                // merge the blocks
                let successor_block = self.graph.vertex(successor_index)?.clone();
                self.graph.vertex_mut(merge_index)?.append(&successor_block);

                // all of successor's successors become merge_block's successors
                let mut new_edges = Vec::new();
                for edge in self.graph.edges_out(successor_index).unwrap() {
                    let head = merge_index;
                    let tail = edge.tail();
                    new_edges.push(edge.clone_new_head_tail(head, tail));
                }
                for edge in new_edges {
                    self.graph.insert_edge(edge)?;
                }

                // remove the block we just merged
                self.graph.remove_vertex(successor_index)?;
            }
        }
        Ok(())
    }

    /// Removes all empty blocks with single successor by rewiring all the
    /// incoming edges of the empty block to its successor.
    fn remove_empty_blocks_with_single_successor(&mut self) -> Result<()> {
        let empty_blocks: Vec<usize> = self
            .blocks()
            .iter()
            .filter(|block| block.is_empty())
            .map(|block| block.index())
            .collect();

        for block_index in empty_blocks {
            // If we do not have just one successor, we will not remove this block
            let successors = self.successor_indices(block_index)?;
            if successors.len() != 1 {
                continue;
            }
            let successor = successors[0];

            // If we do not have any predecessor, we will not remove this block
            let predecessors = self.predecessor_indices(block_index)?;
            if predecessors.is_empty() {
                continue;
            }

            // The labels of the outgoing edge will be merged into all the rewired edges
            let outgoing_edge_labels = self.edge(block_index, successor)?.labels().clone();

            // Rewire predecessor's outgoing edges from self to successor
            for predecessor in predecessors {
                if !self.has_edge(predecessor, successor) {
                    self.rewire_edge(predecessor, block_index, predecessor, successor)?;
                } else {
                    // If the edge exists already, merge their conditions and labels instead
                    let removed_edge =
                        self.remove_edge(predecessor, block_index, RemovedEdgeGuard::Ignore)?;
                    let existing_edge = self.edge_mut(predecessor, successor)?;

                    let combined_condition =
                        match (existing_edge.condition(), removed_edge.condition()) {
                            (Some(c1), Some(c2)) => {
                                Some(Boolean::or(c1.to_owned(), c2.to_owned())?)
                            }
                            (_, _) => None,
                        };
                    existing_edge.set_condition(combined_condition);

                    existing_edge.labels_mut().merge(removed_edge.labels());
                }

                // Finally merge the outgoing edge labels with the rewired edge
                self.edge_mut(predecessor, successor)?
                    .labels_mut()
                    .merge(&outgoing_edge_labels);
            }

            self.remove_block(block_index, RemovedEdgeGuard::Ignore)?;
        }

        Ok(())
    }

    /// Simplifies the control flow graph by removing as well as merging blocks.
    pub fn simplify(&mut self) -> Result<()> {
        self.remove_unreachable_blocks()?;

        loop {
            let block_count_before = self.blocks().len();

            self.merge_consecutive_blocks_with_single_successor_and_predecessor()?;
            self.remove_empty_blocks_with_single_successor()?;

            let block_count_after = self.blocks().len();
            if block_count_before == block_count_after {
                // Fixed-point reached
                break;
            }
        }

        Ok(())
    }

    /// Sets the address for all instructions in this `ControlFlowGraph`.
    ///
    /// Useful for translators to set address information.
    pub fn set_address(&mut self, address: Option<u64>) {
        for block in self.blocks_mut() {
            for instruction in block.instructions_mut() {
                instruction.set_address(address);
            }
        }
    }

    /// Removes all dead-end blocks.
    ///
    /// A block is considered dead-end if no path from the block to the CFG exit exists.
    ///
    /// All such blocks are removed and assumptions/assertions are added which make sure
    /// that the removed edges aren't taken.
    pub fn remove_dead_end_blocks(&mut self, removed_edge_guard: RemovedEdgeGuard) -> Result<()> {
        let exit = self.exit()?;

        let mut queue: Vec<usize> = self
            .graph
            .vertices_without_successors()
            .into_iter()
            .map(Block::index)
            .filter(|&block_index| block_index != exit)
            .collect();

        // Repeatedly remove blocks (!= exit) without successors
        while let Some(block_index) = queue.pop() {
            assert!(block_index != exit);
            assert!(self.successor_indices(block_index)?.is_empty());

            let predecessors = self.predecessor_indices(block_index)?;

            self.remove_block(block_index, removed_edge_guard)?;

            for predecessor in predecessors {
                if self.successor_indices(predecessor)?.is_empty() {
                    queue.push(predecessor);
                }
            }
        }

        Ok(())
    }

    /// Get the variables written by this `ControlFlowGraph`.
    pub fn variables_written(&self) -> Vec<&Variable> {
        self.blocks()
            .into_iter()
            .flat_map(Block::variables_written)
            .collect()
    }

    /// Get the variables read by this `ControlFlowGraph`.
    pub fn variables_read(&self) -> Vec<&Variable> {
        self.blocks()
            .into_iter()
            .flat_map(Block::variables_read)
            .chain(self.edges().into_iter().flat_map(Edge::variables_read))
            .collect()
    }

    /// Get each `Variable` used by this `ControlFlowGraph`.
    pub fn variables(&self) -> Vec<&Variable> {
        self.variables_read()
            .into_iter()
            .chain(self.variables_written().into_iter())
            .collect()
    }
}

impl Default for ControlFlowGraph {
    fn default() -> Self {
        Self::new()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::Boolean;

    #[test]
    fn test_split_block_at_should_correctly_rewire_outgoing_edges_to_new_tail_block() {
        // Given: Block with two incoming and two outgoing edges.
        let mut cfg = ControlFlowGraph::new();

        let pred1 = cfg.new_block().index();
        let pred2 = cfg.new_block().index();

        let block_index = {
            let block = cfg.new_block();
            block.barrier(); // inst 0
            block.barrier(); // inst 1
            block.index()
        };

        let succ1 = cfg.new_block().index();
        let succ2 = cfg.new_block().index();

        cfg.unconditional_edge(pred1, block_index).unwrap();
        cfg.unconditional_edge(pred2, block_index).unwrap();

        cfg.unconditional_edge(block_index, succ1).unwrap();
        cfg.conditional_edge(block_index, succ2, Boolean::constant(true))
            .unwrap();

        // When: Splitting block at instruction 1
        let tail_index = cfg.split_block_at(block_index, 1).unwrap();

        // Then: Incoming edges should still end in head block, but outgoing edges should originate from new tail block.
        assert_eq!(cfg.edges().len(), 4);
        assert!(cfg.edge(pred1, block_index).is_ok(), "Expect Pred1 -> Head");
        assert!(cfg.edge(pred2, block_index).is_ok(), "Expect Pred2 -> Head");
        assert!(
            cfg.edge(block_index, succ1).is_err(),
            "Not expect Head -> Succ1"
        );
        assert!(
            cfg.edge(block_index, succ2).is_err(),
            "Not expect Head -> Succ2"
        );
        assert!(cfg.edge(tail_index, succ1).is_ok(), "Expect Tail -> Succ1");
        assert!(cfg.edge(tail_index, succ2).is_ok(), "Expect Tail -> Succ2");

        // conditional edge should be handled properly
        assert_eq!(
            cfg.edge(tail_index, succ2).unwrap().condition(),
            Some(&Boolean::constant(true)),
        );
    }

    #[test]
    fn test_split_block_at_should_correctly_move_instructions_to_new_tail_block() {
        // Given: Block with 3 instructions.
        let mut cfg = ControlFlowGraph::new();

        let block_index = {
            let block = cfg.new_block();
            block.barrier().set_address(Some(0)); // inst 0
            block.barrier().set_address(Some(1)); // inst 1
            block.barrier().set_address(Some(2)); // inst 2
            block.index()
        };

        // When: Splitting block at instruction 1
        let tail_index = cfg.split_block_at(block_index, 1).unwrap();

        // Then:
        // 1. Instruction 0 should remain in the existing block.
        let head_instructions = cfg.block(block_index).unwrap().instructions();
        assert_eq!(head_instructions.len(), 1);
        assert_eq!(head_instructions[0].address(), Some(0));
        // 2. Instruction 1 and 2 should end up in the new tail block.
        let tail_instructions = cfg.block(tail_index).unwrap().instructions();
        assert_eq!(tail_instructions.len(), 2);
        assert_eq!(tail_instructions[0].address(), Some(1));
        assert_eq!(tail_instructions[1].address(), Some(2));
    }

    #[test]
    fn test_split_block_at_zero_should_give_empty_head_block() {
        // Given: Block with 2 instructions.
        let mut cfg = ControlFlowGraph::new();

        let block_index = {
            let block = cfg.new_block();
            block.barrier(); // inst 0
            block.barrier(); // inst 1
            block.index()
        };

        // When: Splitting block at instruction 0
        let tail_index = cfg.split_block_at(block_index, 0).unwrap();

        // Then:
        let head_instructions = cfg.block(block_index).unwrap().instructions();
        assert_eq!(head_instructions.len(), 0);
        let tail_instructions = cfg.block(tail_index).unwrap().instructions();
        assert_eq!(tail_instructions.len(), 2);
    }

    #[test]
    fn test_split_block_at_two_with_two_instructions_should_give_empty_tail_block() {
        // Given: Block with 2 instructions.
        let mut cfg = ControlFlowGraph::new();

        let block_index = {
            let block = cfg.new_block();
            block.barrier(); // inst 0
            block.barrier(); // inst 1
            block.index()
        };

        // When: Splitting block at instruction 2
        let tail_index = cfg.split_block_at(block_index, 2).unwrap();

        // Then:
        let head_instructions = cfg.block(block_index).unwrap().instructions();
        assert_eq!(head_instructions.len(), 2);
        let tail_instructions = cfg.block(tail_index).unwrap().instructions();
        assert_eq!(tail_instructions.len(), 0);
    }

    #[test]
    fn test_split_block_at_should_update_exit_block_to_new_tail_block_if_exit_block_is_split() {
        // Given: Block which is exit block
        let mut cfg = ControlFlowGraph::new();

        let block_index = {
            let block = cfg.new_block();
            block.barrier(); // inst 0
            block.barrier(); // inst 1
            block.index()
        };

        cfg.set_exit(block_index).unwrap();

        // When: Splitting block at instruction 1
        let tail_index = cfg.split_block_at(block_index, 1).unwrap();

        // Then: Should change exit to new tail block
        assert_eq!(cfg.exit().unwrap(), tail_index);
    }

    #[test]
    fn test_simplify() {
        // GIVEN
        let given_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block0 = cfg.new_block().index();

            let block1 = cfg.new_block().index(); // unreachable block

            let block2 = {
                let block = cfg.new_block();
                block
                    .assign(Boolean::variable("b2"), Boolean::constant(true))
                    .unwrap();
                block.index()
            };

            let block3 = cfg.new_block().index();

            let block4 = cfg.new_block().index();

            let block5 = cfg.new_block().index();

            let block6 = {
                let block = cfg.new_block();
                block
                    .assign(Boolean::variable("b6"), Boolean::constant(true))
                    .unwrap();
                block.index()
            };

            let block7 = {
                let block = cfg.new_block();
                block
                    .assign(Boolean::variable("b7"), Boolean::constant(true))
                    .unwrap();
                block.index()
            };

            let block8 = cfg.new_block().index();

            cfg.unconditional_edge(block0, block2).unwrap();
            cfg.unconditional_edge(block0, block6).unwrap();
            cfg.unconditional_edge(block1, block2).unwrap();
            cfg.unconditional_edge(block2, block3).unwrap();
            cfg.unconditional_edge(block2, block5).unwrap();
            cfg.unconditional_edge(block3, block4).unwrap();
            cfg.unconditional_edge(block3, block5).unwrap();
            cfg.unconditional_edge(block4, block5).unwrap();
            cfg.unconditional_edge(block5, block6).unwrap();
            cfg.unconditional_edge(block6, block7).unwrap();
            cfg.unconditional_edge(block7, block8).unwrap();

            cfg.set_entry(block0).unwrap();
            cfg.set_exit(block8).unwrap();

            cfg
        };

        let mut simplified_cfg = given_cfg.clone();

        // WHEN
        simplified_cfg.simplify().unwrap();

        // THEN
        let expected_cfg = {
            let mut cfg = ControlFlowGraph::new();

            cfg.add_block(Block::new(0)).unwrap();

            cfg.add_block({
                let mut block = Block::new(2);
                block
                    .assign(Boolean::variable("b2"), Boolean::constant(true))
                    .unwrap();
                block
            })
            .unwrap();

            cfg.add_block({
                let mut block = Block::new(6);
                block
                    .assign(Boolean::variable("b6"), Boolean::constant(true))
                    .unwrap();
                block
                    .assign(Boolean::variable("b7"), Boolean::constant(true))
                    .unwrap();
                block
            })
            .unwrap();

            cfg.add_block(Block::new(8)).unwrap();

            cfg.unconditional_edge(0, 2).unwrap();
            cfg.unconditional_edge(0, 6).unwrap();
            cfg.unconditional_edge(2, 6).unwrap();
            cfg.unconditional_edge(6, 8).unwrap();

            cfg.set_entry(0).unwrap();
            cfg.set_exit(8).unwrap();

            cfg
        };

        assert_eq!(expected_cfg, simplified_cfg);
    }

    #[test]
    fn test_simplify_remove_empty_blocks_with_single_successor_should_combine_existing_edges() {
        // GIVEN
        let given_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block0 = cfg.new_block().index();

            let block1 = {
                let block = cfg.new_block();
                block
                    .assign(Boolean::variable("c"), Boolean::constant(true))
                    .unwrap();
                block.index()
            };

            let block2 = cfg.new_block().index(); // empty -> should remove block2 and rewire edge

            let block3 = cfg.new_block().index();

            cfg.conditional_edge(block0, block1, Boolean::variable("a").into())
                .unwrap();

            cfg.conditional_edge(
                block0,
                block3,
                Boolean::not(Boolean::variable("a").into()).unwrap(),
            )
            .unwrap();

            cfg.conditional_edge(block1, block3, Boolean::variable("b").into())
                .unwrap();

            cfg.conditional_edge(
                block1,
                block2,
                Boolean::not(Boolean::variable("b").into()).unwrap(),
            )
            .unwrap();

            cfg.unconditional_edge(block2, block3).unwrap();

            cfg.set_entry(block0).unwrap();
            cfg.set_exit(block3).unwrap();

            cfg
        };

        let mut simplified_cfg = given_cfg.clone();

        // WHEN
        simplified_cfg.simplify().unwrap();

        // THEN
        let expected_cfg = {
            let mut cfg = ControlFlowGraph::new();

            cfg.add_block(Block::new(0)).unwrap();

            cfg.add_block({
                let mut block = Block::new(1);
                block
                    .assign(Boolean::variable("c"), Boolean::constant(true))
                    .unwrap();
                block
            })
            .unwrap();

            cfg.add_block(Block::new(3)).unwrap();

            cfg.conditional_edge(0, 1, Boolean::variable("a").into())
                .unwrap();

            cfg.conditional_edge(0, 3, Boolean::not(Boolean::variable("a").into()).unwrap())
                .unwrap();

            cfg.conditional_edge(
                1,
                3,
                Boolean::or(
                    Boolean::variable("b").into(),
                    Boolean::not(Boolean::variable("b").into()).unwrap(),
                )
                .unwrap(),
            )
            .unwrap();

            cfg.set_entry(0).unwrap();
            cfg.set_exit(3).unwrap();

            cfg
        };

        assert_eq!(expected_cfg, simplified_cfg);
    }
}
