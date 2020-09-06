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
        let folded = fold_program(program);

        if folded {
            Ok(OptimizationResult::Changed)
        } else {
            Ok(OptimizationResult::Unchanged)
        }
    }
}

fn fold_program(program: &mut Program) -> bool {
    program
        .nodes_mut()
        .iter_mut()
        .fold(false, |folded, node| fold_node(node) || folded)
}

fn fold_node(node: &mut Node) -> bool {
    node.expressions_mut()
        .iter_mut()
        .fold(false, |folded, expr| expr.fold() || folded)
}
