use crate::error::Result;
use crate::expr::Memory;
use crate::hir::{Instruction, Operation};
use crate::ir::Transform;

#[derive(Default, Builder, Debug)]
pub struct ExplicitMemory {}

impl Transform<Instruction> for ExplicitMemory {
    fn name(&self) -> &'static str {
        "ExplicitMemory"
    }

    fn description(&self) -> String {
        "Make memory accesses explicit".to_string()
    }

    fn transform(&self, instruction: &mut Instruction) -> Result<()> {
        for operation in instruction.operations_mut() {
            replace_store_load_with_assign(operation)?;
        }

        Ok(())
    }
}

fn replace_store_load_with_assign(operation: &mut Operation) -> Result<()> {
    match operation {
        Operation::Store { address, expr } => {
            // store(address, expr) -> mem := mem-store(mem, address, expr)
            *operation = Operation::assign(
                Memory::variable(),
                Memory::store(Memory::variable().into(), address.clone(), expr.clone())?,
            )?;
        }
        Operation::Load { variable, address } => {
            // variable := load(expr) -> variable := mem-load(mem, expr)
            let bit_width = variable.sort().unwrap_bit_vector();
            *operation = Operation::assign(
                variable.clone(),
                Memory::load(bit_width, Memory::variable().into(), address.clone())?,
            )?;
        }
        _ => (),
    }

    Ok(())
}
