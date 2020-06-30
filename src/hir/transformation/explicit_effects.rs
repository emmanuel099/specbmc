use crate::error::*;
use crate::expr::{BranchTargetBuffer, Cache, Expression, PatternHistoryTable};
use crate::hir::{Effect, Instruction, Operation, Program};
use crate::ir::Transform;

#[derive(Default, Builder, Debug)]
pub struct ExplicitEffects {}

impl Transform<Program> for ExplicitEffects {
    fn name(&self) -> &'static str {
        "ExplicitEffects"
    }

    fn description(&self) -> &'static str {
        "Make instruction effects explicit"
    }

    fn transform(&self, program: &mut Program) -> Result<()> {
        for block in program.control_flow_graph_mut().blocks_mut() {
            for instruction in block.instructions_mut() {
                encode_effects(instruction)?;
            }
        }

        Ok(())
    }
}

fn encode_effects(instruction: &mut Instruction) -> Result<()> {
    if !instruction.has_effects() {
        return Ok(());
    }

    let effect_operations = instruction
        .effects()
        .iter()
        .map(encode_effect)
        .collect::<Result<Vec<Operation>>>()?;

    instruction.add_operations(&effect_operations);

    Ok(())
}

fn encode_effect(effect: &Effect) -> Result<Operation> {
    match effect {
        Effect::Conditional { condition, effect } => {
            if let Operation::Assign { variable, expr } = encode_effect(effect)? {
                Operation::assign(
                    variable.clone(),
                    Expression::ite(condition.clone(), expr, variable.into())?,
                )
            } else {
                unimplemented!()
            }
        }
        Effect::CacheFetch { address, bit_width } => encode_cache_fetch_effect(address, *bit_width),
        Effect::BranchTarget { location, target } => encode_branch_target_effect(location, target),
        Effect::BranchCondition {
            location,
            condition,
        } => encode_branch_condition_effect(location, condition),
    }
}

fn encode_cache_fetch_effect(address: &Expression, bit_width: usize) -> Result<Operation> {
    let cache = Cache::variable();
    let fetch = Cache::fetch(bit_width, cache.clone().into(), address.clone())?;
    Operation::assign(cache, fetch)
}

fn encode_branch_target_effect(location: &Expression, target: &Expression) -> Result<Operation> {
    let btb = BranchTargetBuffer::variable();
    let track = BranchTargetBuffer::track(btb.clone().into(), location.clone(), target.clone())?;
    Operation::assign(btb, track)
}

fn encode_branch_condition_effect(
    location: &Expression,
    condition: &Expression,
) -> Result<Operation> {
    let pht = PatternHistoryTable::variable();
    let taken = PatternHistoryTable::taken(pht.clone().into(), location.clone())?;
    let not_taken = PatternHistoryTable::not_taken(pht.clone().into(), location.clone())?;
    Operation::assign(pht, Expression::ite(condition.clone(), taken, not_taken)?)
}
