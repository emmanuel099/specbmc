use crate::error::Result;
use crate::lir;

mod copy_propagation;
mod dead_code_elimination;
mod expression_simplification;

pub fn optimize(program: &mut lir::Program) -> Result<()> {
    expression_simplification::simplify_expressions(program)?;
    copy_propagation::propagate_copies(program)?;
    dead_code_elimination::eliminate_dead_code(program)?;

    Ok(())
}
