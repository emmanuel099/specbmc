use crate::error::*;
use crate::hir::{Effect, Instruction, Operation, Program};

pub fn add_effects(program: &mut Program) -> Result<()> {
    program
        .control_flow_graph_mut()
        .blocks_mut()
        .iter_mut()
        .for_each(|block| {
            block.instructions_mut().iter_mut().for_each(|instruction| {
                let effects = instruction_effects(instruction);
                instruction.add_effects(&effects);
            });
        });
    Ok(())
}

fn instruction_effects(instruction: &Instruction) -> Vec<Effect> {
    match instruction.operation() {
        Operation::Store { address, expr, .. } => {
            let bit_width = expr.sort().unwrap_bit_vector();
            vec![Effect::cache_fetch(address.clone(), bit_width)]
        }
        Operation::Load {
            variable, address, ..
        } => {
            let bit_width = variable.sort().unwrap_bit_vector();
            vec![Effect::cache_fetch(address.clone(), bit_width)]
        }
        _ => vec![],
    }
}
