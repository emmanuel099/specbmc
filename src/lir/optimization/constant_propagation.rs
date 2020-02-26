//! Constant Propagation
//!
//! Propagates all constant assignments but doesn't remove them.
//! Constant assignment is defined as: `x := c` where c is a constant
//!
//! This algorithm requires that the program is in SSA form.

use crate::error::Result;
use crate::expr;
use crate::lir;
use crate::lir::optimization::{Optimization, OptimizationResult};
use std::collections::HashMap;

pub struct ConstantPropagation {}

impl ConstantPropagation {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for ConstantPropagation {
    /// Propagate all constants
    fn optimize(&self, program: &mut lir::Program) -> Result<OptimizationResult> {
        let constants = determine_constants(program.nodes());
        if constants.is_empty() {
            // No constants
            return Ok(OptimizationResult::Unchanged);
        }

        program.propagate_constants(&constants);

        Ok(OptimizationResult::Changed)
    }
}

type ConstantVariables = HashMap<expr::Variable, expr::Expression>;

fn determine_constants(nodes: &[lir::Node]) -> ConstantVariables {
    let mut constants = HashMap::new();

    nodes.iter().for_each(|node| {
        if let lir::Node::Let { var, expr } = node {
            if expr.is_constant() {
                constants.insert(var.clone(), expr.clone());
            }
        }
    });

    constants
}

trait PropagateConstants {
    fn propagate_constants(&mut self, constants: &ConstantVariables);
}

impl PropagateConstants for lir::Program {
    fn propagate_constants(&mut self, constants: &ConstantVariables) {
        self.nodes_mut()
            .iter_mut()
            .for_each(|node| node.propagate_constants(constants))
    }
}

impl PropagateConstants for lir::Node {
    fn propagate_constants(&mut self, constants: &ConstantVariables) {
        match self {
            Self::Let { expr, .. } => expr.propagate_constants(constants),
            Self::Assert { cond } | Self::Assume { cond } => cond.propagate_constants(constants),
            _ => (),
        }
    }
}

impl PropagateConstants for expr::Expression {
    fn propagate_constants(&mut self, constants: &ConstantVariables) {
        match self.operator() {
            expr::Operator::Variable(var) => {
                if let Some(constant) = constants.get(var) {
                    *self = constant.clone();
                }
            }
            _ => {
                self.operands_mut()
                    .iter_mut()
                    .for_each(|operand| operand.propagate_constants(constants));
            }
        }
    }
}
