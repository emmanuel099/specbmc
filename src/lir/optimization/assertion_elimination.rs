//! Assertion Elimination
//!
//! Removes assertions whose condition is also assumed.
//!
//! This optimization requires that the program is in SSA form.

use crate::error::Result;
use crate::expr::Expression;
use crate::lir::optimization::{Optimization, OptimizationResult};
use crate::lir::{Node, Program};
use std::collections::HashSet;

pub struct AssertionElimination {}

impl AssertionElimination {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for AssertionElimination {
    fn optimize(&self, program: &mut Program) -> Result<OptimizationResult> {
        let assumed_exprs: HashSet<&Expression> = program
            .nodes()
            .iter()
            .filter(|node| node.is_assume())
            .flat_map(|node| node.expressions())
            .collect();

        let asserts_with_assumed_exprs: Vec<usize> = program
            .nodes()
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                if let Node::Assert { condition } = node {
                    if assumed_exprs.contains(condition) {
                        Some(index)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        if asserts_with_assumed_exprs.is_empty() {
            return Ok(OptimizationResult::Unchanged);
        }

        for index in asserts_with_assumed_exprs.into_iter().rev() {
            program.nodes_mut().remove(index);
        }

        Ok(OptimizationResult::Changed)
    }
}
