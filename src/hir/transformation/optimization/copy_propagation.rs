//! Copy Propagation
//!
//! Propagates all simple assignments but doesn't remove them.
//! Simple assignment is defined as: `x := v` where v is a variable
//!
//! This optimization requires that the program is in SSA form.

use crate::error::Result;
use crate::expr::{Operator, Variable};
use crate::hir::transformation::optimization::{Optimization, OptimizationResult};
use crate::hir::{ControlFlowGraph, Operation};
use std::collections::HashMap;

pub struct CopyPropagation {}

impl CopyPropagation {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for CopyPropagation {
    /// Propagate all simple assignments
    fn optimize(&self, cfg: &mut ControlFlowGraph) -> Result<OptimizationResult> {
        let copies = cfg.determine_copies();
        if copies.is_empty() {
            return Ok(OptimizationResult::Unchanged);
        }

        cfg.propagate_copies(&copies);

        Ok(OptimizationResult::Changed)
    }
}

type CopiedVariables = HashMap<Variable, Variable>;

trait DetermineCopies {
    fn determine_copies(&self) -> CopiedVariables;
}

impl DetermineCopies for ControlFlowGraph {
    fn determine_copies(&self) -> CopiedVariables {
        let mut copies = HashMap::new();

        for block in self.blocks() {
            for inst in block.instructions() {
                if let Operation::Assign { variable, expr } = inst.operation() {
                    if let Operator::Variable(src_var) = expr.operator() {
                        copies.insert(variable.clone(), src_var.clone());
                    }
                }
            }
        }

        resolve_copies_of_copies(&mut copies);

        copies
    }
}

trait PropagateCopies {
    fn propagate_copies(&mut self, copies: &CopiedVariables);
}

impl PropagateCopies for ControlFlowGraph {
    fn propagate_copies(&mut self, copies: &CopiedVariables) {
        let replace_if_copied = |var: &mut Variable| {
            if let Some(src_var) = copies.get(var) {
                *var = src_var.clone();
            }
        };

        for edge in self.edges_mut() {
            edge.variables_read_mut()
                .into_iter()
                .for_each(replace_if_copied);
        }

        for block in self.blocks_mut() {
            block
                .variables_read_mut()
                .into_iter()
                .for_each(replace_if_copied);
        }
    }
}

/// Resolves copies of copies to avoid that `replace_copied_variables` needs to be called multiple times.
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
        let mut prop: Option<(Variable, Variable)> = None;
        for (copy, var) in copies.iter() {
            if let Some(src_var) = copies.get(var) {
                prop = Some((copy.clone(), src_var.clone()));
                break;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::Boolean;
    use crate::hir::{Block, Instruction};

    #[test]
    fn test_resolve_copies_of_copies() {
        // GIVEN
        let a = Boolean::variable("a");
        let b = Boolean::variable("b");
        let c = Boolean::variable("c");
        let d = Boolean::variable("d");
        let e = Boolean::variable("e");
        let f = Boolean::variable("f");

        let mut copies = CopiedVariables::new();
        copies.insert(b.clone(), a.clone());
        copies.insert(c.clone(), b.clone());
        copies.insert(d.clone(), c.clone());
        copies.insert(f.clone(), e.clone());

        // WHEN
        resolve_copies_of_copies(&mut copies);

        // THEN
        assert_eq!(copies.get(&b), Some(&a));
        assert_eq!(copies.get(&c), Some(&a));
        assert_eq!(copies.get(&d), Some(&a));
        assert_eq!(copies.get(&f), Some(&e));
    }

    #[test]
    fn test_copy_propagation() {
        // GIVEN

        // v := u
        // y := x
        // z := y
        //   ->  z
        // a := z
        // b := v
        // c := a
        // d := e

        let mut cfg = {
            let mut block0 = Block::new(0);
            block0
                .assign(Boolean::variable("v"), Boolean::variable("u").into())
                .unwrap();
            block0
                .assign(Boolean::variable("y"), Boolean::variable("x").into())
                .unwrap();
            block0
                .assign(Boolean::variable("z"), Boolean::variable("y").into())
                .unwrap();

            let mut block1 = Block::new(1);
            block1
                .assign(Boolean::variable("a"), Boolean::variable("z").into())
                .unwrap();
            block1
                .assign(Boolean::variable("b"), Boolean::variable("v").into())
                .unwrap();
            block1
                .assign(Boolean::variable("c"), Boolean::variable("a").into())
                .unwrap();
            block1
                .assign(Boolean::variable("d"), Boolean::variable("e").into())
                .unwrap();

            let mut cfg = ControlFlowGraph::new();
            cfg.add_block(block0).unwrap();
            cfg.add_block(block1).unwrap();
            cfg.conditional_edge(0, 1, Boolean::variable("z").into())
                .unwrap();

            cfg
        };

        // WHEN
        CopyPropagation::new().optimize(&mut cfg).unwrap();

        // THEN

        // v := u
        // y := x
        // z := x
        //   ->  x
        // a := x
        // b := u
        // c := x
        // d := e

        let block0 = cfg.block(0).unwrap();
        assert_eq!(
            block0.instruction(0).unwrap(),
            &Instruction::assign(Boolean::variable("v"), Boolean::variable("u").into()).unwrap()
        );
        assert_eq!(
            block0.instruction(1).unwrap(),
            &Instruction::assign(Boolean::variable("y"), Boolean::variable("x").into()).unwrap()
        );
        assert_eq!(
            block0.instruction(2).unwrap(),
            &Instruction::assign(Boolean::variable("z"), Boolean::variable("x").into()).unwrap()
        );

        let block1 = cfg.block(1).unwrap();
        assert_eq!(
            block1.instruction(0).unwrap(),
            &Instruction::assign(Boolean::variable("a"), Boolean::variable("x").into()).unwrap()
        );
        assert_eq!(
            block1.instruction(1).unwrap(),
            &Instruction::assign(Boolean::variable("b"), Boolean::variable("u").into()).unwrap()
        );
        assert_eq!(
            block1.instruction(2).unwrap(),
            &Instruction::assign(Boolean::variable("c"), Boolean::variable("x").into()).unwrap()
        );
        assert_eq!(
            block1.instruction(3).unwrap(),
            &Instruction::assign(Boolean::variable("d"), Boolean::variable("e").into()).unwrap()
        );

        let edge = cfg.edge(0, 1).unwrap();
        assert_eq!(edge.condition().unwrap(), &Boolean::variable("x").into());
    }
}
