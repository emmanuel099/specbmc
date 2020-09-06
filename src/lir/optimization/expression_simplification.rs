//! Expression Simplification
//!
//! Tries to simplify expressions, e.g. `x /\ false` will become `false`.

use crate::error::Result;
use crate::expr::Simplify;
use crate::lir::optimization::{Optimization, OptimizationResult};
use crate::lir::{Node, Program};

pub struct ExpressionSimplification {}

impl ExpressionSimplification {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for ExpressionSimplification {
    fn optimize(&self, program: &mut Program) -> Result<OptimizationResult> {
        let simplified = simplify_program(program);

        if simplified {
            Ok(OptimizationResult::Changed)
        } else {
            Ok(OptimizationResult::Unchanged)
        }
    }
}

fn simplify_program(program: &mut Program) -> bool {
    program
        .nodes_mut()
        .iter_mut()
        .fold(false, |simplified, node| simplify_node(node) || simplified)
}

fn simplify_node(node: &mut Node) -> bool {
    node.expressions_mut()
        .iter_mut()
        .fold(false, |simplified, expr| expr.simplify() || simplified)
}
