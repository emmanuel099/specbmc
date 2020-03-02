use crate::error::*;
use crate::expr;
use crate::hir::{Effect, Instruction, Operation, Program};
use crate::util::Transform;

pub struct ExplicitEffects {}

impl ExplicitEffects {
    pub fn new() -> Self {
        Self {}
    }
}

impl Transform<Program> for ExplicitEffects {
    fn description(&self) -> &'static str {
        "Make instruction effects explicit"
    }

    fn transform(&self, program: &mut Program) -> Result<()> {
        let encoder = InstructionEffectEncoder::new(
            expr::Cache::variable(),
            expr::BranchTargetBuffer::variable(),
            expr::PatternHistoryTable::variable(),
        );

        for block in program.control_flow_graph_mut().blocks_mut() {
            for instruction in block.instructions_mut() {
                encoder.encode_effects(instruction)?;
            }
        }

        Ok(())
    }
}

pub(super) struct InstructionEffectEncoder {
    cache: expr::Variable,
    btb: expr::Variable,
    pht: expr::Variable,
}

impl InstructionEffectEncoder {
    pub fn new(cache: expr::Variable, btb: expr::Variable, pht: expr::Variable) -> Self {
        Self { cache, btb, pht }
    }

    pub fn encode_effects(&self, instruction: &mut Instruction) -> Result<()> {
        if !instruction.has_effects() {
            return Ok(());
        }

        let effect_operations = instruction
            .effects()
            .iter()
            .map(|effect| self.encode_effect(effect))
            .collect::<Result<Vec<Operation>>>()?;

        instruction.add_operations(&effect_operations);

        Ok(())
    }

    fn encode_effect(&self, effect: &Effect) -> Result<Operation> {
        match effect {
            Effect::Conditional { condition, effect } => {
                if let Operation::Assign { variable, expr } = self.encode_effect(effect)? {
                    Operation::assign(
                        variable.clone(),
                        expr::Expression::ite(condition.clone(), expr, variable.into())?,
                    )
                } else {
                    unimplemented!()
                }
            }
            Effect::CacheFetch { address, bit_width } => {
                self.encode_cache_fetch_effect(address, *bit_width)
            }
            Effect::BranchTarget { location, target } => {
                self.encode_branch_target_effect(location, target)
            }
            Effect::BranchCondition {
                location,
                condition,
            } => self.encode_branch_condition_effect(location, condition),
        }
    }

    fn encode_cache_fetch_effect(
        &self,
        address: &expr::Expression,
        bit_width: usize,
    ) -> Result<Operation> {
        let fetch = expr::Cache::fetch(bit_width, self.cache.clone().into(), address.clone())?;
        Operation::assign(self.cache.clone(), fetch)
    }

    fn encode_branch_target_effect(
        &self,
        location: &expr::Expression,
        target: &expr::Expression,
    ) -> Result<Operation> {
        let track = expr::BranchTargetBuffer::track(
            self.btb.clone().into(),
            location.clone(),
            target.clone(),
        )?;
        Operation::assign(self.btb.clone(), track)
    }

    fn encode_branch_condition_effect(
        &self,
        location: &expr::Expression,
        condition: &expr::Expression,
    ) -> Result<Operation> {
        let taken = expr::PatternHistoryTable::taken(self.pht.clone().into(), location.clone())?;
        let not_taken =
            expr::PatternHistoryTable::not_taken(self.pht.clone().into(), location.clone())?;
        Operation::assign(
            self.pht.clone(),
            expr::Expression::ite(condition.clone(), taken, not_taken)?,
        )
    }
}
