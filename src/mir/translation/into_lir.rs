use crate::error::Result;
use crate::expr::{Boolean, Expression};
use crate::ir::TryTranslateInto;
use crate::lir;
use crate::mir;

impl TryTranslateInto<lir::Program> for mir::Program {
    fn try_translate_into(&self) -> Result<lir::Program> {
        let mut program = lir::Program::new();

        for composition in 0..self.self_compositions() {
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
    program.comment(format!("Block 0x{:X}@{}", block.index(), composition));

    // make the block's execution condition explicit
    program.assign(
        block
            .execution_condition_variable()
            .self_compose(composition),
        block.execution_condition().self_compose(composition),
    )?;

    for node in block.nodes() {
        match node {
            mir::Node::Comment(text) => {
                program.comment(text);
            }
            mir::Node::Let { var, expr } => {
                program.assign(
                    var.self_compose(composition),
                    expr.self_compose(composition),
                )?;
            }
            mir::Node::Assert { condition } => {
                program.assert(Boolean::imply(
                    block
                        .execution_condition_variable()
                        .self_compose(composition)
                        .into(), // only if executed
                    condition.self_compose(composition),
                )?)?;
            }
            mir::Node::Assume { condition } => {
                program.assume(Boolean::imply(
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

fn add_self_composition_constraints(
    lir_program: &mut lir::Program,
    mir_program: &mir::Program,
) -> Result<()> {
    lir_program.comment("Self-Composition Constraints");

    for block in mir_program.block_graph().blocks() {
        for node in block.nodes() {
            match node {
                mir::Node::HyperAssert { condition } => {
                    let compositions = involved_compositions(condition)?;
                    lir_program.assert(Boolean::imply(
                        hyper_execution_condition(&block, &compositions)?, // only if executed
                        condition.clone(),
                    )?)?;
                }
                mir::Node::HyperAssume { condition } => {
                    let compositions = involved_compositions(condition)?;
                    lir_program.assume(Boolean::imply(
                        hyper_execution_condition(&block, &compositions)?, // only if executed
                        condition.clone(),
                    )?)?;
                }
                _ => (),
            }
        }
    }

    Ok(())
}

/// Lifts the execution condition of the block to multiple compositions.
/// The resulting execution condition is true only if the block is executed in all compositions.
fn hyper_execution_condition(block: &mir::Block, compositions: &[usize]) -> Result<Expression> {
    let exec_cond_var = block.execution_condition_variable();

    Boolean::conjunction(
        &compositions
            .iter()
            .map(|i| exec_cond_var.self_compose(*i).into())
            .collect::<Vec<Expression>>(),
    )
}

/// Determines the involved compositions for the given `Expression`.
/// For example: An expression `(= x@1 x@2)` will give `[1, 2]`.
fn involved_compositions(expr: &Expression) -> Result<Vec<usize>> {
    let mut compositions = Vec::new();

    for variable in expr.variables() {
        let composition = variable
            .composition()
            .ok_or("Expected variable composition, but was none")?;
        compositions.push(composition);
    }

    compositions.sort();
    compositions.dedup();
    Ok(compositions)
}
