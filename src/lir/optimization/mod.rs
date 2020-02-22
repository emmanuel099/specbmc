use crate::error::Result;
use crate::lir;

mod dead_code_elimination;

pub fn optimize(program: &mut lir::Program) -> Result<()> {
    dead_code_elimination::eliminate_dead_code(program)?;

    Ok(())
}
