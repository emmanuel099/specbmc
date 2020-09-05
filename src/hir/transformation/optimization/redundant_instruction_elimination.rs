//! Redundant Instruction Elimination
//!
//! 1. Computes the set of available instructions for each block (forward must analysis)
//! 2. Iterates through each block and removes all instructions which are already available
//!
//! This optimization requires that the program is in SSA form.
//! This optimization is limited to DAGs only.

use crate::error::Result;
use crate::hir::transformation::optimization::{Optimization, OptimizationResult};
use crate::hir::{ControlFlowGraph, Instruction};
use std::collections::{HashMap, HashSet};

pub struct RedundantInstructionElimination {}

impl RedundantInstructionElimination {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for RedundantInstructionElimination {
    fn optimize(&self, cfg: &mut ControlFlowGraph) -> Result<OptimizationResult> {
        let mut changed = false;

        // Tracks the available instructions for each block.
        let mut available_in: HashMap<usize, HashSet<Instruction>> = HashMap::new();
        let mut available_out: HashMap<usize, HashSet<Instruction>> = HashMap::new();

        // As the analysis is limited to DAGs only, the forward must analysis can be done in
        // top-sort ordering without work list.
        let top_sort = cfg.graph().compute_topological_ordering()?;
        for block_index in top_sort {
            let block = cfg.block(block_index)?;

            // inp = intersection of all predecessor's available instructions
            let mut inp: HashSet<Instruction> = HashSet::new();
            if let Some(predecessor) = cfg.predecessor_indices(block_index)?.first() {
                let available = available_out.get(&predecessor).unwrap();
                inp = available.clone();
            }
            for predecessor in cfg.predecessor_indices(block_index)?.iter().skip(1) {
                let available = available_out.get(&predecessor).unwrap();
                inp = inp.intersection(available).cloned().collect();
            }

            let mut redundant_instructions: Vec<usize> = Vec::new();

            // out = inp union instructions of this block
            let mut out: HashSet<Instruction> = inp.clone();
            for (index, inst) in block.instructions().iter().enumerate() {
                if !inst.variables_written().is_empty() {
                    // In SSA only read-only instructions can be redundant
                    continue;
                }

                let already_available = !out.insert(inst.clone());
                if already_available {
                    redundant_instructions.push(index);
                }
            }

            available_in.insert(block_index, inp);
            available_out.insert(block_index, out);

            if !redundant_instructions.is_empty() {
                let block = cfg.block_mut(block_index)?;
                block.remove_instructions(&redundant_instructions)?;
                changed = true;
            }
        }

        if changed {
            Ok(OptimizationResult::Changed)
        } else {
            Ok(OptimizationResult::Unchanged)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{Boolean, Variable};
    use crate::hir::{Block, Instruction};

    fn ssa_var(name: &str, version: usize) -> Variable {
        let mut var = Boolean::variable(name);
        var.set_version(Some(version));
        var
    }

    #[test]
    fn test_remove_redundant_instructions_within_block() {
        // GIVEN
        let mut cfg = {
            let mut block = Block::new(0);
            block.assert(ssa_var("a", 1).into()).unwrap();
            block.assert(ssa_var("b", 1).into()).unwrap();
            block.assert(ssa_var("a", 1).into()).unwrap();
            block.assert(ssa_var("a", 2).into()).unwrap();
            block.assert(ssa_var("b", 2).into()).unwrap();
            block.assert(ssa_var("b", 2).into()).unwrap();

            let mut cfg = ControlFlowGraph::new();
            cfg.add_block(block).unwrap();
            cfg
        };

        // WHEN
        RedundantInstructionElimination::new()
            .optimize(&mut cfg)
            .unwrap();

        // THEN
        let block = cfg.block(0).unwrap();
        assert_eq!(
            block.instructions(),
            &vec![
                Instruction::assert(ssa_var("a", 1).into()).unwrap(),
                Instruction::assert(ssa_var("b", 1).into()).unwrap(),
                Instruction::assert(ssa_var("a", 2).into()).unwrap(),
                Instruction::assert(ssa_var("b", 2).into()).unwrap(),
            ]
        );
    }

    #[test]
    fn test_remove_redundant_instructions_within_multiple_blocks() {
        // GIVEN
        let mut cfg = {
            let mut block0 = Block::new(0);
            block0.assert(ssa_var("a", 1).into()).unwrap();

            let mut block1 = Block::new(1);
            block1.assert(ssa_var("b", 1).into()).unwrap();
            block1.assert(ssa_var("c", 1).into()).unwrap();

            let mut block2 = Block::new(2);
            block2.assert(ssa_var("b", 1).into()).unwrap();
            block2.assert(ssa_var("c", 2).into()).unwrap();

            let mut block3 = Block::new(3);
            block3.assert(ssa_var("a", 1).into()).unwrap();
            block3.assert(ssa_var("b", 1).into()).unwrap();
            block3.assert(ssa_var("c", 1).into()).unwrap();
            block3.assert(ssa_var("c", 2).into()).unwrap();
            block3.assert(ssa_var("d", 1).into()).unwrap();

            let mut cfg = ControlFlowGraph::new();
            cfg.add_block(block0).unwrap();
            cfg.add_block(block1).unwrap();
            cfg.add_block(block2).unwrap();
            cfg.add_block(block3).unwrap();

            cfg.unconditional_edge(0, 1).unwrap();
            cfg.unconditional_edge(0, 2).unwrap();
            cfg.unconditional_edge(1, 3).unwrap();
            cfg.unconditional_edge(2, 3).unwrap();

            cfg
        };

        // WHEN
        RedundantInstructionElimination::new()
            .optimize(&mut cfg)
            .unwrap();

        // THEN
        let block3 = cfg.block(3).unwrap();
        assert_eq!(
            block3.instructions(),
            &vec![
                Instruction::assert(ssa_var("c", 1).into()).unwrap(),
                Instruction::assert(ssa_var("c", 2).into()).unwrap(),
                Instruction::assert(ssa_var("d", 1).into()).unwrap(),
            ]
        );
    }
}
