use crate::error::Result;
use crate::expr::{BitVector, Expression, Variable};
use crate::hir::{Block, Instruction, Operation};
use crate::ir::Transform;

#[derive(Default, Builder, Debug)]
pub struct ExplicitProgramCounter {
    observe_program_counter: bool,
    observe_memory_loads: bool,
}

impl ExplicitProgramCounter {
    pub fn address_variable() -> Variable {
        let mut var = BitVector::word_variable("_address");
        var.labels_mut().rollback_persistent();
        var
    }

    pub fn pc_variable() -> Variable {
        let mut var = BitVector::word_variable("_pc");
        var.labels_mut().rollback_persistent();
        var
    }
}

impl Transform<Block> for ExplicitProgramCounter {
    fn name(&self) -> &'static str {
        "ExplicitProgramCounter"
    }

    fn description(&self) -> String {
        "Add explicit program counter".to_string()
    }

    fn transform(&self, block: &mut Block) -> Result<()> {
        let mut observations: Vec<(usize, Instruction)> = Vec::new();

        for (index, inst) in block.instructions().iter().enumerate() {
            match inst.operation() {
                Operation::Load { address, .. } | Operation::Store { address, .. } => {
                    if self.observe_memory_loads {
                        observations.push((
                            index,
                            Instruction::assign(Self::address_variable(), address.clone())?,
                        ));
                    }
                }
                Operation::Branch { target, .. } => {
                    if self.observe_program_counter {
                        observations.push((
                            index,
                            Instruction::assign(Self::pc_variable(), target.clone())?,
                        ));
                    }
                }
                Operation::ConditionalBranch {
                    condition, target, ..
                } => {
                    if self.observe_program_counter {
                        let next = BitVector::word_constant(inst.address().unwrap_or_default() + 8); // FIXME
                        let pc = Expression::ite(condition.clone(), target.clone(), next)?;
                        observations.push((index, Instruction::assign(Self::pc_variable(), pc)?));
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
