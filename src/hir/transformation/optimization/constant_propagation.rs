//! Constant Propagation
//!
//! Propagates all constant assignments but doesn't remove them.
//! Constant assignment is defined as: `x := c` where c is a constant
//!
//! This optimization requires that the program is in SSA form.

use crate::error::Result;
use crate::expr::{Expression, Operator, Variable};
use crate::hir::transformation::optimization::{Optimization, OptimizationResult};
use crate::hir::{Block, ControlFlowGraph, Edge, Operation};
use std::collections::HashMap;

pub struct ConstantPropagation {}

impl ConstantPropagation {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for ConstantPropagation {
    /// Propagate all constants
    fn optimize(&self, cfg: &mut ControlFlowGraph) -> Result<OptimizationResult> {
        let constants = cfg.determine_constants();
        if constants.is_empty() {
            return Ok(OptimizationResult::Unchanged);
        }

        cfg.propagate_constants(&constants);

        Ok(OptimizationResult::Changed)
    }
}

type ConstantVariables = HashMap<Variable, Expression>;

trait DetermineConstants {
    fn determine_constants(&self) -> ConstantVariables;
}

impl DetermineConstants for ControlFlowGraph {
    fn determine_constants(&self) -> ConstantVariables {
        let mut constants = HashMap::new();

        for block in self.blocks() {
            for inst in block.instructions() {
                if let Operation::Assign { variable, expr } = inst.operation() {
                    if expr.is_constant() {
                        constants.insert(variable.clone(), expr.clone());
                    }
                }
            }
        }

        constants
    }
}

trait PropagateConstants {
    fn propagate_constants(&mut self, constants: &ConstantVariables);
}

impl PropagateConstants for ControlFlowGraph {
    fn propagate_constants(&mut self, constants: &ConstantVariables) {
        self.edges_mut()
            .iter_mut()
            .for_each(|edge| edge.propagate_constants(constants));

        self.blocks_mut()
            .iter_mut()
            .for_each(|block| block.propagate_constants(constants));
    }
}

impl PropagateConstants for Edge {
    fn propagate_constants(&mut self, constants: &ConstantVariables) {
        if let Some(condition) = self.condition_mut() {
            condition.propagate_constants(constants);
        }
    }
}

impl PropagateConstants for Block {
    fn propagate_constants(&mut self, constants: &ConstantVariables) {
        self.expressions_mut()
            .iter_mut()
            .for_each(|expr| expr.propagate_constants(constants));
    }
}

impl PropagateConstants for Expression {
    fn propagate_constants(&mut self, constants: &ConstantVariables) {
        match self.operator() {
            Operator::Variable(var) => {
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
