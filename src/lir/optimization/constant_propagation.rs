//! Constant Propagation
//!
//! Propagates all constant assignments but doesn't remove them.
//! Constant assignment is defined as: `x := c` where c is a constant
//!
//! This algorithm requires that the program is in SSA form.

use crate::error::Result;
use crate::expr::{Expression, Operator, Variable};
use crate::lir::optimization::{Optimization, OptimizationResult};
use crate::lir::{Node, Program};
use std::collections::HashMap;

pub struct ConstantPropagation {}

impl ConstantPropagation {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for ConstantPropagation {
    /// Propagate all constants
    fn optimize(&self, program: &mut Program) -> Result<OptimizationResult> {
        let constants = program.determine_constants();
        if constants.is_empty() {
            return Ok(OptimizationResult::Unchanged);
        }

        program.propagate_constants(&constants);

        Ok(OptimizationResult::Changed)
    }
}

type ConstantVariables = HashMap<Variable, Expression>;

trait DetermineConstants {
    fn determine_constants(&self) -> ConstantVariables;
}

impl DetermineConstants for Program {
    fn determine_constants(&self) -> ConstantVariables {
        let mut constants = HashMap::new();

        self.nodes().iter().for_each(|node| {
            if let Node::Let { var, expr } = node {
                if expr.is_constant() {
                    constants.insert(var.clone(), expr.clone());
                }
            }
        });

        constants
    }
}

trait PropagateConstants {
    fn propagate_constants(&mut self, constants: &ConstantVariables);
}

impl PropagateConstants for Program {
    fn propagate_constants(&mut self, constants: &ConstantVariables) {
        self.nodes_mut()
            .iter_mut()
            .for_each(|node| node.propagate_constants(constants))
    }
}

impl PropagateConstants for Node {
    fn propagate_constants(&mut self, constants: &ConstantVariables) {
        match self {
            Self::Let { expr, .. } => expr.propagate_constants(constants),
            Self::Assert { condition } | Self::Assume { condition } => {
                condition.propagate_constants(constants)
            }
            _ => (),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{Boolean, Sort, Variable};

    #[test]
    fn test_constant_propagation() {
        // GIVEN

        // x := true
        // y := x
        // assume(y /\ x)

        let mut program = Program::new();
        program
            .assign(Variable::new("x", Sort::boolean()), Boolean::constant(true))
            .unwrap();
        program
            .assign(
                Variable::new("y", Sort::boolean()),
                Variable::new("x", Sort::boolean()).into(),
            )
            .unwrap();
        program
            .assume(
                Boolean::and(
                    Variable::new("y", Sort::boolean()).into(),
                    Variable::new("x", Sort::boolean()).into(),
                )
                .unwrap(),
            )
            .unwrap();

        // WHEN
        ConstantPropagation::new().optimize(&mut program).unwrap();

        // THEN

        // x := true
        // y := true
        // assume(y /\ true)

        assert_eq!(
            program.node(0).unwrap(),
            &Node::assign(Variable::new("x", Sort::boolean()), Boolean::constant(true),).unwrap()
        );
        assert_eq!(
            program.node(1).unwrap(),
            &Node::assign(Variable::new("y", Sort::boolean()), Boolean::constant(true),).unwrap()
        );
        assert_eq!(
            program.node(2).unwrap(),
            &Node::assume(
                Boolean::and(
                    Variable::new("y", Sort::boolean()).into(),
                    Boolean::constant(true),
                )
                .unwrap(),
            )
            .unwrap()
        );
    }
}
