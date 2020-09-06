//! Dead Code Elimination (DCE)
//!
//! Mark-and-Sweep like dead code elimination which works like:
//!   1. Phase (Mark): All useless instructions and phi nodes are marked
//!   2. Phase (Sweep): All marked instructions and phi nodes are removed
//!
//! This optimization requires that the program is in SSA form.

use crate::error::Result;
use crate::expr::Variable;
use crate::hir::transformation::optimization::{Optimization, OptimizationResult};
use crate::hir::{ControlFlowGraph, Instruction};
use std::collections::{BTreeSet, HashMap};

pub struct DeadCodeElimination {}

impl DeadCodeElimination {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for DeadCodeElimination {
    /// Remove all dead instruction and phi nodes from the given program.
    fn optimize(&self, cfg: &mut ControlFlowGraph) -> Result<OptimizationResult> {
        let dead_marks = mark(cfg);
        if dead_marks.is_empty() {
            return Ok(OptimizationResult::Unchanged);
        }

        sweep(cfg, &dead_marks);

        Ok(OptimizationResult::Changed)
    }
}

trait DceCritical {
    /// Determines if Self is critical, meaning that it should not be removed by DCE.
    fn is_critical(&self) -> bool;
}

impl DceCritical for Instruction {
    fn is_critical(&self) -> bool {
        self.variables_written().is_empty() || self.has_effects()
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum Reference {
    Instruction { block: usize, index: usize },
    PhiNode { block: usize, index: usize },
}

/// Get a map from variables to instruction resp. phi node references where the variables are defined.
fn variable_definitions(cfg: &ControlFlowGraph) -> HashMap<&Variable, Reference> {
    let mut defs = HashMap::new();

    cfg.blocks().iter().for_each(|block| {
        block
            .phi_nodes()
            .iter()
            .enumerate()
            .for_each(|(index, phi_node)| {
                let phi_ref = Reference::PhiNode {
                    block: block.index(),
                    index,
                };
                defs.insert(phi_node.out(), phi_ref);
            });

        block
            .instructions()
            .iter()
            .enumerate()
            .for_each(|(index, inst)| {
                let inst_ref = Reference::Instruction {
                    block: block.index(),
                    index,
                };
                inst.variables_written().iter().for_each(|var| {
                    defs.insert(var, inst_ref.clone());
                })
            });
    });

    defs
}

/// Mark useless instructions and phi nodes which should be removed.
fn mark(cfg: &ControlFlowGraph) -> BTreeSet<Reference> {
    let defs = variable_definitions(cfg);

    let mut marks = BTreeSet::new();
    let mut work_queue: Vec<Reference> = Vec::new();

    // Mark non-critical instructions and phi nodes
    cfg.blocks().iter().for_each(|block| {
        block
            .instructions()
            .iter()
            .enumerate()
            .for_each(|(index, inst)| {
                let reference = Reference::Instruction {
                    block: block.index(),
                    index,
                };
                if inst.is_critical() {
                    work_queue.push(reference);
                } else {
                    marks.insert(reference);
                }
            });

        block.phi_nodes().iter().enumerate().for_each(|(index, _)| {
            let reference = Reference::PhiNode {
                block: block.index(),
                index,
            };
            marks.insert(reference);
        });
    });

    // Unmark edge dependencies
    cfg.edges().iter().for_each(|edge| {
        let mark_def = |var| {
            // duplicated to satisfy the borrow checker
            if let Some(def_index) = defs.get(var) {
                if marks.remove(def_index) {
                    work_queue.push(def_index.clone());
                }
            }
        };
        edge.variables_read().iter().for_each(mark_def);
    });

    // Iteratively unmark the dependencies
    while let Some(reference) = work_queue.pop() {
        let mark_def = |var| {
            // duplicated to satisfy the borrow checker
            if let Some(def_index) = defs.get(var) {
                if marks.remove(def_index) {
                    work_queue.push(def_index.clone());
                }
            }
        };
        match reference {
            Reference::Instruction { block, index } => {
                cfg.block(block)
                    .unwrap()
                    .instruction(index)
                    .unwrap()
                    .variables_read()
                    .iter()
                    .for_each(mark_def);
            }
            Reference::PhiNode { block, index } => {
                cfg.block(block)
                    .unwrap()
                    .phi_node(index)
                    .unwrap()
                    .incoming_variables()
                    .iter()
                    .for_each(mark_def);
            }
        }
    }

    marks
}

/// Remove all marked instructions and phi nodes.
fn sweep(cfg: &mut ControlFlowGraph, marks: &BTreeSet<Reference>) {
    marks.iter().rev().for_each(|reference| match reference {
        Reference::Instruction { block, index } => {
            cfg.block_mut(*block)
                .unwrap()
                .remove_instruction(*index)
                .unwrap();
        }
        Reference::PhiNode { block, index } => {
            cfg.block_mut(*block)
                .unwrap()
                .remove_phi_node(*index)
                .unwrap();
        }
    });
}
