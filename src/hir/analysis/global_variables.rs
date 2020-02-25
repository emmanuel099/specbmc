use crate::expr;
use crate::hir;
use std::collections::HashSet;

/// Computes the set of variables that are live on entry of at least one block.
pub fn global_variables(program: &hir::Program) -> HashSet<expr::Variable> {
    let mut globals = HashSet::new();

    for block in program.control_flow_graph().blocks() {
        let mut killed = HashSet::new();

        block.instructions().iter().for_each(|inst| {
            inst.variables_read()
                .into_iter()
                .filter(|variable| !killed.contains(variable))
                .for_each(|variable| {
                    globals.insert(variable.clone());
                });

            inst.variables_written().into_iter().for_each(|variable| {
                killed.insert(variable);
            });
        });
    }

    globals
}
