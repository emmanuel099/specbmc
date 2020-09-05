//! Phi Node Elimination
//!
//! Replaces unnecessary phi nodes with assignments.
//! `x2 = phi[x1, x1, ..., x1]` -> `x2 := x1`
//!
//! This optimization requires that the program is in SSA form.

use crate::error::Result;
use crate::hir::transformation::optimization::{Optimization, OptimizationResult};
use crate::hir::{Block, ControlFlowGraph, Instruction, PhiNode};

pub struct PhiElimination {}

impl PhiElimination {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for PhiElimination {
    fn optimize(&self, cfg: &mut ControlFlowGraph) -> Result<OptimizationResult> {
        let mut changed = false;

        for block in cfg.blocks_mut() {
            let unnecessary_phi_nodes =
                phi_nodes_with_all_same_incoming_variables(block.phi_nodes());

            for phi_node_index in unnecessary_phi_nodes.into_iter().rev() {
                replace_phi_node_with_assignment(block, phi_node_index)?;
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

// Find phi nodes where all incoming variables are the same, i.e., `x2 = phi[x1, x1, ..., x1]`
fn phi_nodes_with_all_same_incoming_variables(phi_nodes: &[PhiNode]) -> Vec<usize> {
    phi_nodes
        .iter()
        .enumerate()
        .filter_map(|(index, phi_node)| {
            if all_same(&phi_node.incoming_variables()) {
                Some(index)
            } else {
                None
            }
        })
        .collect()
}

// Replace `x2 = phi[x1, x1, ..., x1]` with `x2 := x1` assignment at the top of the block
fn replace_phi_node_with_assignment(block: &mut Block, phi_node_index: usize) -> Result<()> {
    let phi_node = block.remove_phi_node(phi_node_index)?;
    if let Some(&in_var) = phi_node.incoming_variables().first() {
        let replacement = Instruction::assign(phi_node.out().clone(), in_var.clone().into())?;
        block.insert_instruction(0, replacement)?;
    }
    Ok(())
}

fn all_same<T: PartialEq>(elements: &[T]) -> bool {
    elements.windows(2).all(|pair| pair[0] == pair[1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{Boolean, Variable};

    fn ssa_var(name: &str, version: usize) -> Variable {
        let mut var = Boolean::variable(name);
        var.set_version(Some(version));
        var
    }

    #[test]
    fn test_should_replace_phi_node_with_assignment_if_all_incoming_variables_are_the_same() {
        // GIVEN
        let mut cfg = {
            let mut phi_node = PhiNode::new(ssa_var("x", 2));
            phi_node.add_incoming(ssa_var("x", 1), 1);
            phi_node.add_incoming(ssa_var("x", 1), 2);

            let mut block = Block::new(0);
            block.add_phi_node(phi_node);

            let mut cfg = ControlFlowGraph::new();
            cfg.add_block(block).unwrap();
            cfg
        };

        // WHEN
        let result = PhiElimination::new().optimize(&mut cfg).unwrap();

        // THEN
        assert_eq!(result, OptimizationResult::Changed);

        let block = cfg.block(0).unwrap();
        assert_eq!(0, block.phi_nodes().len());
        assert_eq!(1, block.instructions().len());
        assert_eq!(
            &Instruction::assign(ssa_var("x", 2), ssa_var("x", 1).into()).unwrap(),
            block.instruction(0).unwrap()
        );
    }

    #[test]
    fn test_should_not_replace_phi_node_with_assignment_if_incoming_variables_are_different() {
        // GIVEN
        let mut cfg = {
            let mut phi_node = PhiNode::new(ssa_var("x", 3));
            phi_node.add_incoming(ssa_var("x", 1), 1);
            phi_node.add_incoming(ssa_var("x", 2), 2);

            let mut block = Block::new(0);
            block.add_phi_node(phi_node);

            let mut cfg = ControlFlowGraph::new();
            cfg.add_block(block).unwrap();
            cfg
        };

        // WHEN
        let result = PhiElimination::new().optimize(&mut cfg).unwrap();

        // THEN
        assert_eq!(result, OptimizationResult::Unchanged);

        let block = cfg.block(0).unwrap();
        assert_eq!(1, block.phi_nodes().len());
        assert_eq!(0, block.instructions().len());
    }
}
