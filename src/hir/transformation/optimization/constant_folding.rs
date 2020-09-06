//! Constant Folding
//!
//! Tries to evaluate expressions to constants if all their operands are constant,
//! e.g. `1 + 2` will become `3`.

use crate::error::Result;
use crate::expr::Fold;
use crate::hir::transformation::optimization::{Optimization, OptimizationResult};
use crate::hir::{Block, ControlFlowGraph, Edge};

pub struct ConstantFolding {}

impl ConstantFolding {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for ConstantFolding {
    fn optimize(&self, cfg: &mut ControlFlowGraph) -> Result<OptimizationResult> {
        let folded = fold_cfg(cfg);

        if folded {
            Ok(OptimizationResult::Changed)
        } else {
            Ok(OptimizationResult::Unchanged)
        }
    }
}

fn fold_cfg(cfg: &mut ControlFlowGraph) -> bool {
    let mut folded = false;
    for edge in cfg.edges_mut() {
        folded = fold_edge(edge) || folded;
    }
    for block in cfg.blocks_mut() {
        folded = fold_block(block) || folded;
    }
    folded
}

fn fold_edge(edge: &mut Edge) -> bool {
    if let Some(condition) = edge.condition_mut() {
        condition.fold()
    } else {
        false
    }
}

fn fold_block(block: &mut Block) -> bool {
    block
        .expressions_mut()
        .iter_mut()
        .fold(false, |folded, expr| expr.fold() || folded)
}
