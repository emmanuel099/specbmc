use crate::error::*;
use crate::expr::Memory;
use crate::hir::{Operation, Program};
use crate::util::Transform;

pub struct ExplicitMemory {}

impl ExplicitMemory {
    pub fn new() -> Self {
        Self {}
    }
}

impl Transform<Program> for ExplicitMemory {
    fn description(&self) -> &'static str {
        "Make memory accesses explicit"
    }

    fn transform(&self, program: &mut Program) -> Result<()> {
        for block in program.control_flow_graph_mut().blocks_mut() {
            for instruction in block.instructions_mut() {
                for operation in instruction.operations_mut() {
                    replace_store_load_with_assign(operation)?;
                }
            }
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
