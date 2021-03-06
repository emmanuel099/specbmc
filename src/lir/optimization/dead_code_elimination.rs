//! Dead Code Elimination (DCE)
//!
//! Mark-and-Sweep like dead code elimination which works like:
//!   1. Phase (Mark): All useful nodes (including their dependencies) are marked
//!   2. Phase (Sweep): All unmarked nodes are removed
//!
//! This algorithm requires that the program is in SSA form.

use crate::error::Result;
use crate::expr::Variable;
use crate::lir::optimization::{Optimization, OptimizationResult};
use crate::lir::{Node, Program};
use bit_vec::BitVec;
use std::collections::HashMap;

pub struct DeadCodeElimination {}

impl DeadCodeElimination {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for DeadCodeElimination {
    /// Remove all dead nodes from the given program.
    ///
    /// `Assert` and `Assume` nodes are considered as critical,
    /// meaning that they (including their dependencies) will remain.
    fn optimize(&self, program: &mut Program) -> Result<OptimizationResult> {
        let marks = mark(program.nodes());
        if marks.all() {
            // No dead nodes
            return Ok(OptimizationResult::Unchanged);
        }

        sweep(program.nodes_mut(), &marks);

        Ok(OptimizationResult::Changed)
    }
}

trait DceCritical {
    /// Determines if Self is critical, meaning that it should not be removed by DCE.
    fn is_critical(&self) -> bool;
}

impl DceCritical for Node {
    fn is_critical(&self) -> bool {
        !self.is_let()
    }
}

/// Get a map from variables to node indices where the variables are defined.
fn variable_definitions(nodes: &[Node]) -> HashMap<&Variable, usize> {
    let mut defs = HashMap::new();

    nodes.iter().enumerate().for_each(|(index, node)| {
        node.variables_defined().iter().for_each(|&var| {
            defs.insert(var, index);
        });
    });

    defs
}

/// Mark useful nodes which should not be removed.
///
/// The `BitVec` contains a single bit for each node.
/// If the bit for a node is not set, the node can safely be removed.
fn mark(nodes: &[Node]) -> BitVec {
    let defs = variable_definitions(nodes);

    let mut marks = BitVec::from_elem(nodes.len(), false);
    let mut work_queue: Vec<usize> = Vec::new();

    // Mark critical nodes first
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.is_critical())
        .for_each(|(index, _)| {
            marks.set(index, true);
            work_queue.push(index);
        });

    // Iteratively mark the dependencies
    while let Some(index) = work_queue.pop() {
        let mark_def = |var| {
            if let Some(def_index) = defs.get(var) {
                if !marks.get(*def_index).unwrap() {
                    marks.set(*def_index, true);
                    work_queue.push(*def_index);
                }
            }
        };

        nodes[index].variables_used().iter().for_each(mark_def);
    }

    marks
}

/// Remove all unmarked nodes.
fn sweep(nodes: &mut Vec<Node>, marks: &BitVec) {
    marks
        .iter()
        .enumerate()
        .filter(|(_, marked)| !*marked)
        .rev()
        .for_each(|(index, _)| {
            nodes.remove(index);
        });
}
