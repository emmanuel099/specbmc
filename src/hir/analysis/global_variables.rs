use crate::expr::Variable;
use crate::hir::Program;
use std::collections::HashSet;

/// Computes the set of variables that are live on entry of at least one block.
pub fn global_variables(program: &Program) -> HashSet<Variable> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{BitVector, Expression};
    use crate::hir::ControlFlowGraph;

    fn expr_const(value: u64) -> Expression {
        BitVector::constant_u64(value, 64)
    }

    fn variable(name: &str) -> Variable {
        BitVector::variable(name, 64)
    }

    #[test]
    fn test_global_variables() {
        let program = {
            let mut cfg = ControlFlowGraph::new();

            let block0 = cfg.new_block();
            block0.assign(variable("x"), expr_const(1)).unwrap();

            let block1 = cfg.new_block();
            block1.assign(variable("tmp"), expr_const(1)).unwrap();
            block1
                .assign(variable("x"), variable("tmp").into())
                .unwrap();

            let block2 = cfg.new_block();
            block2.load(variable("y"), variable("x").into()).unwrap();

            Program::new(cfg)
        };

        assert_eq!(
            global_variables(&program),
            vec![variable("x")].into_iter().collect()
        );
    }
}
