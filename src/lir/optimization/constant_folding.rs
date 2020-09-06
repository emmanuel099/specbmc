//! Constant Folding
//!
//! Tries to evaluate expressions to constants if all their operands are constant,
//! e.g. `1 + 2` will become `3`.

use crate::error::Result;
use crate::expr::Fold;
use crate::lir::optimization::{Optimization, OptimizationResult};
use crate::lir::{Node, Program};

pub struct ConstantFolding {}

impl ConstantFolding {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for ConstantFolding {
    fn optimize(&self, program: &mut Program) -> Result<OptimizationResult> {
        let folded = program
            .nodes_mut()
            .iter_mut()
            .fold(false, |folded, node| fold_node(node) || folded);

        if folded {
            Ok(OptimizationResult::Changed)
        } else {
            Ok(OptimizationResult::Unchanged)
        }
    }
}

fn fold_node(node: &mut Node) -> bool {
    match node {
        Node::Let { expr, .. } => expr.fold(),
        Node::Assert { condition } | Node::Assume { condition } => condition.fold(),
        _ => false,
    }
}
