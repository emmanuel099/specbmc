use crate::error::Result;
use crate::expr::{BranchTargetBuffer, Cache, Expression, PatternHistoryTable};
use crate::hir::{Block, Effect, Instruction, Operation};
use crate::ir::Transform;

#[derive(Default, Builder, Debug)]
pub struct ExplicitEffects {}

impl Transform<Block> for ExplicitEffects {
    fn name(&self) -> &'static str {
        "ExplicitEffects"
    }

    fn description(&self) -> String {
        "Make instruction effects explicit".to_string()
    }

    fn transform(&self, block: &mut Block) -> Result<()> {
        let mut instructions_to_insert = Vec::new();

        for (index, inst) in block.instructions().iter().enumerate() {
            let effect_operations = inst
                .effects()
                .iter()
                .map(encode_effect)
                .collect::<Result<Vec<Operation>>>()?;

            // Insert explicit effects immediately before the current instruction.
            // Inserting them before is necessary as the instruction may write to variables which are read in the effect.
            for effect_op in effect_operations {
                let mut effect_inst = Instruction::new(effect_op);
                effect_inst.set_address(inst.address());
                effect_inst.labels_mut().pseudo();
                instructions_to_insert.push((index, effect_inst));
            }
        }

        block.insert_instructions(instructions_to_insert)?;

        Ok(())
    }
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
