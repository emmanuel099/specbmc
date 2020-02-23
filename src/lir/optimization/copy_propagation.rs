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
        let copies = determine_copies(program.nodes());
        if copies.is_empty() {
            // No copies
            return Ok(OptimizationResult::Unchanged);
        }

        program.propagate_copies(&copies);

        Ok(OptimizationResult::Changed)
    }
}

type CopiedVariables = HashMap<expr::Variable, expr::Variable>;

fn determine_copies(nodes: &[lir::Node]) -> CopiedVariables {
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

trait PropagateCopies {
    fn propagate_copies(&mut self, copies: &CopiedVariables);
}

impl PropagateCopies for lir::Program {
    fn propagate_copies(&mut self, copies: &CopiedVariables) {
        self.nodes_mut()
            .into_iter()
            .for_each(|node| node.propagate_copies(copies))
    }
}

impl PropagateCopies for lir::Node {
    fn propagate_copies(&mut self, copies: &CopiedVariables) {
        let replace_if_copied = |var: &mut expr::Variable| match copies.get(var) {
            Some(src_var) => *var = src_var.clone(),
            None => (),
        };

        match self {
            Self::Let { expr, .. } => expr.variables_mut().into_iter().for_each(replace_if_copied),
            Self::Assert { cond } | Self::Assume { cond } => {
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
fn resolve_copies_of_copies(copies: &mut CopiedVariables) {
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
