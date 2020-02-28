use crate::error::*;
use crate::expr;
use crate::hir::{Effect, Instruction, Operation, Program};

pub struct ExplicitEffects {
    nonspec_effects: bool,
}

impl ExplicitEffects {
    pub fn new() -> Self {
        Self {
            nonspec_effects: false,
        }
    }

    /// Enable or disable non-speculative effects.
    pub fn with_nonspec_effects(&mut self, nonspec_effects: bool) -> &mut Self {
        self.nonspec_effects = nonspec_effects;
        self
    }

    pub fn transform(&self, program: &mut Program) -> Result<()> {
        for block in program.control_flow_graph_mut().blocks_mut() {
            let additionally_encode_nonspec = self.nonspec_effects && !block.is_transient();

            for instruction in block.instructions_mut() {
                if instruction.has_effects() {
                    encode_instruction_effects(instruction, false)?;
                    if additionally_encode_nonspec {
                        encode_instruction_effects(instruction, true)?;
                    }
                }
            }
        }

        Ok(())
    }
}

fn encode_instruction_effects(instruction: &mut Instruction, nonspec: bool) -> Result<()> {
    let mut operations = vec![instruction.operation().clone()];

    for effect in instruction.effects() {
        operations.push(encode_effect(effect, nonspec)?);
    }

    *instruction.operation_mut() = Operation::parallel(operations);

    Ok(())
}

fn encode_effect(effect: &Effect, nonspec: bool) -> Result<Operation> {
    match effect {
        Effect::Conditional { condition, effect } => {
            if let Operation::Assign { variable, expr } = encode_effect(effect, nonspec)? {
                Ok(Operation::assign(
                    variable.clone(),
                    expr::Expression::ite(condition.clone(), expr, variable.into())?,
                ))
            } else {
                unimplemented!()
            }
        }
        Effect::CacheFetch { address, bit_width } => {
            let cache = if nonspec {
                expr::Cache::variable_nonspec()
            } else {
                expr::Cache::variable()
            };
            encode_cache_fetch_effect(cache, address, *bit_width)
        }
        Effect::BranchTarget { location, target } => {
            let btb = if nonspec {
                expr::BranchTargetBuffer::variable_nonspec()
            } else {
                expr::BranchTargetBuffer::variable()
            };
            encode_branch_target_effect(btb, location, target)
        }
        Effect::BranchCondition {
            location,
            condition,
        } => {
            let pht = if nonspec {
                expr::PatternHistoryTable::variable_nonspec()
            } else {
                expr::PatternHistoryTable::variable()
            };
            encode_branch_condition_effect(pht, location, condition)
        }
    }
}

fn encode_cache_fetch_effect(
    cache: expr::Variable,
    address: &expr::Expression,
    bit_width: usize,
) -> Result<Operation> {
    let fetch = expr::Cache::fetch(bit_width, cache.clone().into(), address.clone())?;
    Ok(Operation::assign(cache, fetch))
}

fn encode_branch_target_effect(
    btb: expr::Variable,
    location: &expr::Expression,
    target: &expr::Expression,
) -> Result<Operation> {
    let track =
        expr::BranchTargetBuffer::track(btb.clone().into(), location.clone(), target.clone())?;
    Ok(Operation::assign(btb, track))
}

fn encode_branch_condition_effect(
    pht: expr::Variable,
    location: &expr::Expression,
    condition: &expr::Expression,
) -> Result<Operation> {
    let taken = expr::PatternHistoryTable::taken(pht.clone().into(), location.clone())?;
    let not_taken = expr::PatternHistoryTable::not_taken(pht.clone().into(), location.clone())?;
    Ok(Operation::assign(
        pht,
        expr::Expression::ite(condition.clone(), taken, not_taken)?,
    ))
}
