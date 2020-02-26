use crate::error::Result;
use crate::expr;
use crate::lir;
use crate::mir;

pub fn translate_program(program: &mir::Program) -> Result<lir::Program> {
    let mut lir_program = lir::Program::new();

    for block in program.block_graph().blocks() {
        translate_block(&mut lir_program, block)?;
    }

    Ok(lir_program)
}

fn translate_block(program: &mut lir::Program, block: &mir::Block) -> Result<()> {
    program.append_comment(format!("Block 0x{:X}", block.index()));

    // make the block's execution condition explicit
    program.append_let(
        block.execution_condition_variable(),
        block.execution_condition().clone(),
    )?;

    for node in block.nodes() {
        match node.operation() {
            mir::Operation::Let { var, expr } => {
                program.append_let(var.clone(), expr.clone())?;
            }
            mir::Operation::Assert { cond } => {
                program.append_assert(expr::Boolean::imply(
                    block.execution_condition_variable().clone().into(), // only if executed
                    cond.clone(),
                )?)?;
            }
            mir::Operation::Assume { cond } => {
                program.append_assume(expr::Boolean::imply(
                    block.execution_condition_variable().clone().into(), // only if executed
                    cond.clone(),
                )?)?;
            }
        }
    }

    Ok(())
}
