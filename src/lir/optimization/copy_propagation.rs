//! Copy Propagation
//!
//! Propagates all simple assignments but doesn't remove them.
//! Simple assignment is defined as: `x := v` where v is a variable
//!
//! This algorithm requires that the program is in SSA form.

use crate::error::Result;
use crate::expr;
use crate::lir;
use crate::lir::optimization::{Optimization, OptimizationResult};
use std::collections::HashMap;

pub struct CopyPropagation {}

impl CopyPropagation {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for CopyPropagation {
    /// Propagate all simple assignments
    fn optimize(&self, program: &mut lir::Program) -> Result<OptimizationResult> {
        let copies = determine_copied_variables(program.nodes());
        if copies.is_empty() {
            // No copies
            return Ok(OptimizationResult::Unchanged);
        }

        replace_copied_variables(&mut program.nodes_mut(), &copies);

        Ok(OptimizationResult::Changed)
    }
}

fn determine_copied_variables(nodes: &[lir::Node]) -> HashMap<expr::Variable, expr::Variable> {
    let mut copies = HashMap::new();

    nodes.iter().for_each(|node| match node {
        lir::Node::Let { var, expr } => match expr.operator() {
            expr::Operator::Variable(src_var) => {
                copies.insert(var.clone(), src_var.clone());
            }
            _ => (),
        },
        _ => (),
    });

    resolve_copies_of_copies(&mut copies);

    copies
}

fn replace_copied_variables(
    nodes: &mut Vec<lir::Node>,
    copies: &HashMap<expr::Variable, expr::Variable>,
) {
    let replace_if_copied = |var: &mut expr::Variable| match copies.get(var) {
        Some(src_var) => *var = src_var.clone(),
        None => (),
    };

    for node in nodes {
        match node {
            lir::Node::Let { expr, .. } => {
                expr.variables_mut().into_iter().for_each(replace_if_copied)
            }
            lir::Node::Assert { cond } | lir::Node::Assume { cond } => {
                cond.variables_mut().into_iter().for_each(replace_if_copied)
            }
            _ => (),
        }
    }
}

/// Resolves copies of copies to avoid that replace_copied_variables needs to be called multiple times.
///
/// Given:
/// b = a
/// c = b
///
/// Resolved:
/// b = a
/// c = a
fn resolve_copies_of_copies(copies: &mut HashMap<expr::Variable, expr::Variable>) {
    loop {
        let mut prop: Option<(expr::Variable, expr::Variable)> = None;
        for (copy, var) in copies.iter() {
            match copies.get(var) {
                Some(src_var) => {
                    prop = Some((copy.clone(), src_var.clone()));
                    break;
                }
                None => (),
            }
        }

        match prop {
            Some((copy, src_var)) => {
                copies.insert(copy, src_var);
                continue;
            }
            None => break,
        }
    }
}
