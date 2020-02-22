use crate::error::Result;
use crate::lir;

mod copy_propagation;
mod dead_code_elimination;

pub fn optimize(program: &mut lir::Program) -> Result<()> {
    copy_propagation::propagate_copies(program)?;
    dead_code_elimination::eliminate_dead_code(program)?;

    Ok(())
}
