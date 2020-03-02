use crate::environment::Environment;
use crate::error::*;
use crate::expr::BitVector;
use crate::hir::{Effect, Instruction, Operation, Program};
use crate::util::Transform;

pub struct InstructionEffects {
    cache_available: bool,
    btb_available: bool,
    pht_available: bool,
}

impl InstructionEffects {
    pub fn new() -> Self {
        Self {
            cache_available: false,
            btb_available: false,
            pht_available: false,
        }
    }

    pub fn new_from_env(env: &Environment) -> Self {
        Self {
            cache_available: env.architecture().cache(),
            btb_available: env.architecture().branch_target_buffer(),
            pht_available: env.architecture().pattern_history_table(),
        }
    }

    /// Enable or disable Cache effects.
    pub fn with_cache(&mut self, available: bool) -> &mut Self {
        self.cache_available = available;
        self
    }

    /// Enable or disable Branch Target Buffer effects.
    pub fn with_branch_target_buffer(&mut self, available: bool) -> &mut Self {
        self.btb_available = available;
        self
    }

    /// Enable or disable Pattern History Table effects.
    pub fn with_pattern_history_table(&mut self, available: bool) -> &mut Self {
        self.pht_available = available;
        self
    }

    fn instruction_effects(&self, instruction: &Instruction) -> Vec<Effect> {
        let mut effects = Vec::new();

        match instruction.operation() {
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
            Operation::Branch { target } => {
                if self.btb_available {
                    let location =
                        BitVector::constant_u64(instruction.address().unwrap_or_default(), 64); // FIXME bit-width
                    effects.push(Effect::branch_target(location, target.clone()));
                }
            }
            Operation::ConditionalBranch { condition, target } => {
                if self.btb_available {
                    let location =
                        BitVector::constant_u64(instruction.address().unwrap_or_default(), 64); // FIXME bit-width
                    effects.push(
                        Effect::branch_target(location, target.clone()).only_if(condition.clone()),
                    );
                }
                if self.pht_available {
                    let location =
                        BitVector::constant_u64(instruction.address().unwrap_or_default(), 64); // FIXME bit-width
                    effects.push(Effect::branch_condition(location, condition.clone()));
                }
            }
            _ => (),
        }

        effects
    }
}

impl Transform<Program> for InstructionEffects {
    fn description(&self) -> &'static str {
        "Add instruction effects"
    }

    fn transform(&self, program: &mut Program) -> Result<()> {
        program
            .control_flow_graph_mut()
            .blocks_mut()
            .iter_mut()
            .for_each(|block| {
                block.instructions_mut().iter_mut().for_each(|instruction| {
                    let effects = self.instruction_effects(instruction);
                    instruction.add_effects(&effects);
                });
            });

        Ok(())
    }
}
