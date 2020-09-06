//! Expression Simplification
//!
//! Tries to simplify expressions, e.g. `x /\ false` will become `false`.

use crate::error::Result;
use crate::expr::Simplify;
use crate::hir::transformation::optimization::{Optimization, OptimizationResult};
use crate::hir::{Block, ControlFlowGraph, Edge};

pub struct ExpressionSimplification {}

impl ExpressionSimplification {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for ExpressionSimplification {
    fn optimize(&self, cfg: &mut ControlFlowGraph) -> Result<OptimizationResult> {
        let simplified = simplify_cfg(cfg);

        if simplified {
            Ok(OptimizationResult::Changed)
        } else {
            Ok(OptimizationResult::Unchanged)
        }
    }
}

fn simplify_cfg(cfg: &mut ControlFlowGraph) -> bool {
    let mut simplified = false;
    for edge in cfg.edges_mut() {
        simplified = simplify_edge(edge) || simplified;
    }
    for block in cfg.blocks_mut() {
        simplified = simplify_block(block) || simplified;
    }
    simplified
}

fn simplify_edge(edge: &mut Edge) -> bool {
    if let Some(condition) = edge.condition_mut() {
        condition.simplify()
    } else {
        false
    }
}

fn simplify_block(block: &mut Block) -> bool {
    block
        .expressions_mut()
        .iter_mut()
        .fold(false, |simplified, expr| expr.simplify() || simplified)
}
