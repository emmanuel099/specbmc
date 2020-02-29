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

#[cfg(test)]
mod tests {
    use super::*;

    fn expr_const(value: u64) -> expr::Expression {
        expr::BitVector::constant_u64(value, 64)
    }

    fn variable(name: &str) -> expr::Variable {
        expr::BitVector::variable(name, 64)
    }

    #[test]
    fn test_global_variables() {
        let program = {
            let mut cfg = hir::ControlFlowGraph::new();

            let block0 = cfg.new_block().unwrap();
            block0.assign(variable("x"), expr_const(1)).unwrap();

            let block1 = cfg.new_block().unwrap();
            block1.assign(variable("tmp"), expr_const(1)).unwrap();
            block1
                .assign(variable("x"), variable("tmp").into())
                .unwrap();

            let block2 = cfg.new_block().unwrap();
            block2.load(variable("y"), variable("x").into()).unwrap();

            hir::Program::new(cfg)
        };

        assert_eq!(
            global_variables(&program),
            vec![variable("x")].into_iter().collect()
        );
    }
}
