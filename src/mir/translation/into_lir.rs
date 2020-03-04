use crate::error::Result;
use crate::expr;
use crate::lir;
use crate::mir;
use crate::util::TranslateInto;
use std::collections::BTreeSet;

impl TranslateInto<lir::Program> for mir::Program {
    fn translate_into(&self) -> Result<lir::Program> {
        let mut program = lir::Program::new();

        for composition in required_compositions(self) {
            translate_program_composition(&mut program, self, composition)?;
        }

        add_self_composition_constraints(&mut program, self)?;

        Ok(program)
    }
}

fn translate_program_composition(
    lir_program: &mut lir::Program,
    mir_program: &mir::Program,
    composition: usize,
) -> Result<()> {
    for block in mir_program.block_graph().blocks() {
        translate_block(lir_program, block, composition)?;
    }
    Ok(())
}

fn translate_block(
    program: &mut lir::Program,
    block: &mir::Block,
    composition: usize,
) -> Result<()> {
    program.append_comment(format!("Block 0x{:X}@{}", block.index(), composition));

    // make the block's execution condition explicit
    program.append_let(
        block
            .execution_condition_variable()
            .self_compose(composition),
        block.execution_condition().self_compose(composition),
    )?;

    for node in block.nodes() {
        match node.operation() {
            mir::Operation::Let { var, expr } => {
                program.append_let(
                    var.self_compose(composition),
                    expr.self_compose(composition),
                )?;
            }
            mir::Operation::Assert { condition } => {
                program.append_assert(expr::Boolean::imply(
                    block
                        .execution_condition_variable()
                        .self_compose(composition)
                        .into(), // only if executed
                    condition.self_compose(composition),
                )?)?;
            }
            mir::Operation::Assume { condition } => {
                program.append_assume(expr::Boolean::imply(
                    block
                        .execution_condition_variable()
                        .self_compose(composition)
                        .into(), // only if executed
                    condition.self_compose(composition),
                )?)?;
            }
            _ => (),
        }
    }

    Ok(())
}

/// Determines the required compositions for the given `program`.
///
/// For each returned value a copy should be created.
fn required_compositions(program: &mir::Program) -> BTreeSet<usize> {
    let mut result: BTreeSet<usize> = BTreeSet::new();

    for block in program.block_graph().blocks() {
        for node in block.nodes() {
            match node.operation() {
                mir::Operation::SelfCompAssertEqual { compositions, .. }
                | mir::Operation::SelfCompAssumeEqual { compositions, .. } => {
                    compositions.iter().for_each(|composition| {
                        result.insert(*composition);
                    });
                }
                _ => (),
            }
        }
    }

    if result.is_empty() {
        // If program doesn't contain any self-composition operators,
        // require only a single LIR program.
        result.insert(1);
    }

    result
}

fn add_self_composition_constraints(
    lir_program: &mut lir::Program,
    mir_program: &mir::Program,
) -> Result<()> {
    lir_program.append_comment("Self-Composition Constraints");

    for block in mir_program.block_graph().blocks() {
        for node in block.nodes() {
            match node.operation() {
                mir::Operation::SelfCompAssertEqual { compositions, expr } => {
                    lir_program.append_assert(self_composition_equality_constraint(
                        &block.execution_condition_variable(),
                        compositions,
                        expr,
                    )?)?;
                }
                mir::Operation::SelfCompAssumeEqual { compositions, expr } => {
                    lir_program.append_assume(self_composition_equality_constraint(
                        &block.execution_condition_variable(),
                        compositions,
                        expr,
                    )?)?;
                }
                _ => (),
            }
        }
    }

    Ok(())
}

/// Create self-composition equality constraint,
/// requiring that `expr` is equal in all compositions.
///
/// Example:
///   - Suppose `exec_cond` = 'c', `compositions` = [1,2] and `expr` = 'x'
///   - This will give the constraint `(=> (and c@1 c@2) (= x@1 x@2))`
fn self_composition_equality_constraint(
    exec_cond: &expr::Variable,
    compositions: &[usize],
    expr: &expr::Expression,
) -> Result<expr::Expression> {
    expr::Boolean::imply(
        expr::Boolean::conjunction(
            &compositions
                .iter()
                .map(|i| exec_cond.self_compose(*i).into())
                .collect::<Vec<expr::Expression>>(),
        )?, // only if executed in all compositions
        expr::Expression::all_equal(
            &compositions
                .iter()
                .map(|i| expr.self_compose(*i))
                .collect::<Vec<expr::Expression>>(),
        )?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_composition_equality_constraint() {
        // GIVEN
        let exec_cond = expr::Boolean::variable("x");
        let compositions = vec![1, 4];
        let expr: expr::Expression = expr::Boolean::variable("y").into();

        // WHEN
        let constraint =
            self_composition_equality_constraint(&exec_cond, &compositions, &expr).unwrap();

        // THEN
        assert_eq!(
            constraint,
            expr::Boolean::imply(
                expr::Boolean::and(
                    {
                        let mut var = expr::Boolean::variable("x");
                        var.set_composition(Some(1));
                        var.into()
                    },
                    {
                        let mut var = expr::Boolean::variable("x");
                        var.set_composition(Some(4));
                        var.into()
                    }
                )
                .unwrap(),
                expr::Expression::equal(
                    {
                        let mut var = expr::Boolean::variable("y");
                        var.set_composition(Some(1));
                        var.into()
                    },
                    {
                        let mut var = expr::Boolean::variable("y");
                        var.set_composition(Some(4));
                        var.into()
                    }
                )
                .unwrap()
            )
            .unwrap()
        );
    }

    #[test]
    fn test_required_compositions_should_give_1_for_program_without_self_comp_ops() {
        // GIVEN
        let mir_program = {
            let mut block1 = mir::Block::new(1);
            block1.add_node(mir::Node::assert(expr::Boolean::constant(true)).unwrap());
            block1.add_node(
                mir::Node::assign(expr::Boolean::variable("x"), expr::Boolean::constant(true))
                    .unwrap(),
            );

            let mut block2 = mir::Block::new(2);
            block2.add_node(mir::Node::assume(expr::Boolean::constant(true)).unwrap());

            let mut block_graph = mir::BlockGraph::new();
            block_graph.add_block(block1).unwrap();
            block_graph.add_block(block2).unwrap();

            mir::Program::new(block_graph)
        };

        // WHEN
        let compositions = required_compositions(&mir_program);

        // THEN
        assert_eq!(compositions, vec![1].into_iter().collect());
    }

    #[test]
    fn test_required_compositions_should_give_2_3_4_for_program_with_self_comp_ops_using_comp_2_3_and_4(
    ) {
        // GIVEN
        let mir_program = {
            let mut block1 = mir::Block::new(1);
            block1.add_node(mir::Node::assert_equal_in_self_composition(
                vec![2, 3],
                expr::Boolean::variable("x").into(),
            ));

            let mut block2 = mir::Block::new(2);
            block2.add_node(mir::Node::assume_equal_in_self_composition(
                vec![3, 4],
                expr::Boolean::variable("x").into(),
            ));

            let mut block_graph = mir::BlockGraph::new();
            block_graph.add_block(block1).unwrap();
            block_graph.add_block(block2).unwrap();

            mir::Program::new(block_graph)
        };

        // WHEN
        let compositions = required_compositions(&mir_program);

        // THEN
        assert_eq!(compositions, vec![2, 3, 4].into_iter().collect());
    }
}
