use crate::error::Result;
use crate::expr::BitVector;
use crate::hir::{Effect, Instruction, Operation};
use crate::ir::Transform;

#[derive(Default, Builder, Debug)]
pub struct InstructionEffects {
    model_cache_effects: bool,
    model_btb_effects: bool,
    model_pht_effects: bool,
}

impl InstructionEffects {
    fn instruction_effects(&self, instruction: &Instruction) -> Vec<Effect> {
        let mut effects = Vec::new();

        match instruction.operation() {
            Operation::Store { address, expr, .. } => {
                if self.model_cache_effects {
                    let bit_width = expr.sort().unwrap_bit_vector();
                    effects.push(Effect::cache_fetch(address.clone(), bit_width));
                }
            }
            Operation::Load {
                variable, address, ..
            } => {
                if self.model_cache_effects {
                    let bit_width = variable.sort().unwrap_bit_vector();
                    effects.push(Effect::cache_fetch(address.clone(), bit_width));
                }
            }
            Operation::Call { target } | Operation::Branch { target } => {
                if self.model_btb_effects {
                    let location =
                        BitVector::word_constant(instruction.address().unwrap_or_default());
                    effects.push(Effect::branch_target(location, target.clone()));
                }
            }
            Operation::ConditionalBranch { condition, target } => {
                if self.model_btb_effects {
                    let location =
                        BitVector::word_constant(instruction.address().unwrap_or_default());
                    effects.push(
                        Effect::branch_target(location, target.clone()).only_if(condition.clone()),
                    );
                }
                if self.model_pht_effects {
                    let location =
                        BitVector::word_constant(instruction.address().unwrap_or_default());
                    effects.push(Effect::branch_condition(location, condition.clone()));
                }
            }
            _ => (),
        }

        effects
    }
}

impl Transform<Instruction> for InstructionEffects {
    fn name(&self) -> &'static str {
        "InstructionEffects"
    }

    fn description(&self) -> String {
        "Add instruction effects".to_string()
    }

    fn transform(&self, instruction: &mut Instruction) -> Result<()> {
        let effects = self.instruction_effects(instruction);
        instruction.add_effects(&effects);

        Ok(())
    }
}
