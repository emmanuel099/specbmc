use crate::error::*;
use crate::expr::BitVector;
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
        Operation::Branch { target } => {
            let location = BitVector::constant(instruction.address().unwrap_or_default(), 64); // FIXME bit-width
            vec![Effect::unconditional_branch_target(
                location,
                target.clone(),
            )]
        }
        Operation::ConditionalBranch { condition, target } => {
            let location = BitVector::constant(instruction.address().unwrap_or_default(), 64); // FIXME bit-width
            vec![Effect::conditional_branch_target(
                condition.clone(),
                location,
                target.clone(),
            )]
        }
        _ => vec![],
    }
}
