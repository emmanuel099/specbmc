use crate::environment::Check;
use crate::error::Result;
use crate::expr::{BitVector, Expression};
use crate::hir::{Block, Instruction, Operation};
use crate::ir::Transform;

#[derive(Default, Builder, Debug)]
pub struct ProgramCounterModelObservations {
    check: Check,
    observe_program_counter: bool,
    observe_memory_loads: bool,
}

impl Transform<Block> for ProgramCounterModelObservations {
    fn name(&self) -> &'static str {
        "ProgramCounterModelObservations"
    }

    fn description(&self) -> String {
        "Add program-counter model observations".to_string()
    }

    fn transform(&self, block: &mut Block) -> Result<()> {
        let mut observations: Vec<(usize, Instruction)> = Vec::new();

        let mut obs = |index: usize, expr: Expression| match self.check {
            Check::OnlyTransientExecutionLeaks => {
                if block.is_transient() {
                    observations.push((index, Instruction::observable(expr)));
                } else {
                    observations.push((index, Instruction::indistinguishable(expr)));
                }
            }
            Check::OnlyNormalExecutionLeaks | Check::AllLeaks => {
                observations.push((index, Instruction::observable(expr)));
            }
        };

        for (index, inst) in block.instructions().iter().enumerate() {
            match inst.operation() {
                Operation::Load { address, .. } | Operation::Store { address, .. } => {
                    if self.observe_memory_loads {
                        obs(index, address.clone());
                    }
                }
                Operation::Branch { target, .. } => {
                    if self.observe_program_counter {
                        obs(index, target.clone());
                    }
                }
                Operation::ConditionalBranch {
                    condition, target, ..
                } => {
                    if self.observe_program_counter {
                        let next = BitVector::word_constant(inst.address().unwrap_or_default() + 8); // FIXME
                        let pc = Expression::ite(condition.clone(), target.clone(), next)?;
                        obs(index, pc);
                    }
                }
                _ => {}
            }
        }

        for (index, mut obs) in observations.into_iter().rev() {
            obs.labels_mut().pseudo();
            block.insert_instruction(index, obs)?;
        }

        Ok(())
    }
}
