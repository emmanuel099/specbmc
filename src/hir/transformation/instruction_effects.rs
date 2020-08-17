use crate::environment::{Environment, WORD_SIZE};
use crate::error::Result;
use crate::expr::BitVector;
use crate::hir::{Effect, Instruction, Operation};
use crate::ir::Transform;

#[derive(Default, Builder, Debug)]
pub struct InstructionEffects {
    cache_available: bool,
    btb_available: bool,
    pht_available: bool,
}

impl InstructionEffects {
    pub fn new_from_env(env: &Environment) -> Self {
        Self {
            cache_available: env.architecture.cache,
            btb_available: env.architecture.branch_target_buffer,
            pht_available: env.architecture.pattern_history_table,
        }
    }

    fn instruction_effects(&self, instruction: &Instruction) -> Vec<Effect> {
        let mut effects = Vec::new();

        instruction
            .operations()
            .iter()
            .for_each(|operation| match operation {
                Operation::Store { address, expr, .. } => {
                    if self.cache_available {
                        let bit_width = expr.sort().unwrap_bit_vector();
                        effects.push(Effect::cache_fetch(address.clone(), bit_width));
                    }
                }
                Operation::Load {
                    variable, address, ..
                } => {
                    if self.cache_available {
                        let bit_width = variable.sort().unwrap_bit_vector();
                        effects.push(Effect::cache_fetch(address.clone(), bit_width));
                    }
                }
                Operation::Call { target } | Operation::Branch { target } => {
                    if self.btb_available {
                        let location = BitVector::constant_u64(
                            instruction.address().unwrap_or_default(),
                            WORD_SIZE,
                        );
                        effects.push(Effect::branch_target(location, target.clone()));
                    }
                }
                Operation::ConditionalBranch { condition, target } => {
                    if self.btb_available {
                        let location = BitVector::constant_u64(
                            instruction.address().unwrap_or_default(),
                            WORD_SIZE,
                        );
                        effects.push(
                            Effect::branch_target(location, target.clone())
                                .only_if(condition.clone()),
                        );
                    }
                    if self.pht_available {
                        let location = BitVector::constant_u64(
                            instruction.address().unwrap_or_default(),
                            WORD_SIZE,
                        );
                        effects.push(Effect::branch_condition(location, condition.clone()));
                    }
                }
                _ => (),
            });

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
