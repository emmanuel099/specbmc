//! Copy Propagation
//!
//! Propagates all simple assignments but doesn't remove them.
//! Simple assignment is defined as: `x := v` where v is a variable
//!
//! This algorithm requires that the program is in SSA form.

use crate::error::Result;
use crate::expr::{Operator, Variable};
use crate::lir::optimization::{Optimization, OptimizationResult};
use crate::lir::{Node, Program};
use std::collections::HashMap;

pub struct CopyPropagation {}

impl CopyPropagation {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for CopyPropagation {
    /// Propagate all simple assignments
    fn optimize(&self, program: &mut Program) -> Result<OptimizationResult> {
        let copies = program.determine_copies();
        if copies.is_empty() {
            return Ok(OptimizationResult::Unchanged);
        }

        program.propagate_copies(&copies);

        Ok(OptimizationResult::Changed)
    }
}

type CopiedVariables = HashMap<Variable, Variable>;

trait DetermineCopies {
    fn determine_copies(&self) -> CopiedVariables;
}

impl DetermineCopies for Program {
    fn determine_copies(&self) -> CopiedVariables {
        let mut copies = HashMap::new();

        self.nodes().iter().for_each(|node| {
            if let Node::Let { var, expr } = node {
                if let Operator::Variable(src_var) = expr.operator() {
                    copies.insert(var.clone(), src_var.clone());
                }
            }
        });

        resolve_copies_of_copies(&mut copies);

        copies
    }
}

trait PropagateCopies {
    fn propagate_copies(&mut self, copies: &CopiedVariables);
}

impl PropagateCopies for Program {
    fn propagate_copies(&mut self, copies: &CopiedVariables) {
        self.nodes_mut()
            .iter_mut()
            .for_each(|node| node.propagate_copies(copies))
    }
}

impl PropagateCopies for Node {
    fn propagate_copies(&mut self, copies: &CopiedVariables) {
        let replace_if_copied = |var: &mut Variable| {
            if let Some(src_var) = copies.get(var) {
                *var = src_var.clone();
            }
        };

        self.variables_used_mut()
            .into_iter()
            .for_each(replace_if_copied);
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
    use crate::expr::{Sort, Variable};

    #[test]
    fn test_resolve_copies_of_copies() {
        // GIVEN
        let a = Variable::new("a", Sort::boolean());
        let b = Variable::new("b", Sort::boolean());
        let c = Variable::new("c", Sort::boolean());
        let d = Variable::new("d", Sort::boolean());
        let e = Variable::new("e", Sort::boolean());
        let f = Variable::new("f", Sort::boolean());

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

        let mut program = Program::new();
        program
            .assign(
                Variable::new("v", Sort::boolean()),
                Variable::new("u", Sort::boolean()).into(),
            )
            .unwrap();
        program
            .assign(
                Variable::new("y", Sort::boolean()),
                Variable::new("x", Sort::boolean()).into(),
            )
            .unwrap();
        program
            .assign(
                Variable::new("z", Sort::boolean()),
                Variable::new("y", Sort::boolean()).into(),
            )
            .unwrap();

        // WHEN
        CopyPropagation::new().optimize(&mut program).unwrap();

        // THEN

        // v := u
        // y := x
        // z := x

        assert_eq!(
            program.node(0).unwrap(),
            &Node::assign(
                Variable::new("v", Sort::boolean()),
                Variable::new("u", Sort::boolean()).into(),
            )
            .unwrap()
        );
        assert_eq!(
            program.node(1).unwrap(),
            &Node::assign(
                Variable::new("y", Sort::boolean()),
                Variable::new("x", Sort::boolean()).into(),
            )
            .unwrap()
        );
        assert_eq!(
            program.node(2).unwrap(),
            &Node::assign(
                Variable::new("z", Sort::boolean()),
                Variable::new("x", Sort::boolean()).into(),
            )
            .unwrap()
        );
    }
}
