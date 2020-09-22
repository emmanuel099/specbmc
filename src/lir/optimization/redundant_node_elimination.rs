//! Redundant Node Elimination (DCE)
//!
//! Removes duplicated assertions and assumptions.
//!
//! This optimization requires that the program is in SSA form.

use crate::error::Result;
use crate::lir::optimization::{Optimization, OptimizationResult};
use crate::lir::{Node, Program};
use std::collections::{BTreeSet, HashMap};

pub struct RedundantNodeElimination {}

impl RedundantNodeElimination {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for RedundantNodeElimination {
    /// Remove all dead nodes from the given program.
    ///
    /// `Assert` and `Assume` nodes are considered as critical,
    /// meaning that they (including their dependencies) will remain.
    fn optimize(&self, program: &mut Program) -> Result<OptimizationResult> {
        let duplicated_indices = compute_duplicated_node_indices(program);

        if duplicated_indices.is_empty() {
            return Ok(OptimizationResult::Unchanged);
        }

        for index in duplicated_indices.into_iter().rev() {
            program.nodes_mut().remove(index);
        }

        Ok(OptimizationResult::Changed)
    }
}

/// Returns a set of nodes indices which can safely be removed.
fn compute_duplicated_node_indices(program: &Program) -> BTreeSet<usize> {
    // Limit to assertions and assumptions only because assignments are never duplicated (-> SSA)
    let mut available_assertion_assumptions: HashMap<&Node, Vec<usize>> = HashMap::new();
    program
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, node)| node.is_assert() || node.is_assume())
        .for_each(|(index, node)| {
            available_assertion_assumptions
                .entry(node)
                .and_modify(|indices| indices.push(index))
                .or_insert_with(|| vec![index]);
        });

    available_assertion_assumptions
        .iter()
        .filter(|(_, indices)| indices.len() > 1)
        .fold(BTreeSet::new(), |mut acc, (_, indices)| {
            acc.extend(indices.iter().skip(1)); // skip(1) to retain one copy
            acc
        })
}
