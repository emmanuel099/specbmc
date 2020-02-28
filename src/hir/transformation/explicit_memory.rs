use crate::error::*;
use crate::expr::Memory;
use crate::hir::{Operation, Program};

pub struct ExplicitMemory {}

impl ExplicitMemory {
    pub fn new() -> Self {
        Self {}
    }

    pub fn transform(&self, program: &mut Program) -> Result<()> {
        for block in program.control_flow_graph_mut().blocks_mut() {
            for instruction in block.instructions_mut() {
                replace_store_load_operation_with_assign(instruction.operation_mut())?;
            }
        }

        Ok(())
    }
}

fn replace_store_load_operation_with_assign(operation: &mut Operation) -> Result<()> {
    match operation {
        Operation::Store { address, expr } => {
            *operation = Operation::assign(
                Memory::variable(),
                Memory::store(Memory::variable().into(), address.clone(), expr.clone())?,
            );
        }
        Operation::Load { variable, address } => {
            let bit_width = variable.sort().unwrap_bit_vector();
            *operation = Operation::assign(
                variable.clone(),
                Memory::load(bit_width, Memory::variable().into(), address.clone())?,
            );
        }
        Operation::Parallel(operations) => {
            for operation in operations {
                replace_store_load_operation_with_assign(operation)?;
            }
        }
        _ => (),
    }

    Ok(())
}
