mod explicit_effects;
mod function_inlining;
mod init_global_variables;
mod instruction_effects;
mod loop_unwinding;
mod non_spec_obs_equiv;
mod observations;
mod optimization;
mod ssa_transformation;
mod transient_execution;

pub use self::explicit_effects::ExplicitEffects;
pub use self::function_inlining::FunctionInlining;
pub use self::init_global_variables::InitGlobalVariables;
pub use self::instruction_effects::InstructionEffects;
pub use self::loop_unwinding::LoopUnwinding;
pub use self::non_spec_obs_equiv::NonSpecObsEquivalence;
pub use self::observations::Observations;
pub use self::optimization::Optimizer;
pub use self::ssa_transformation::{SSAForm, SSATransformation};
pub use self::transient_execution::TransientExecution;

use crate::error::Result;
use crate::hir::{Block, ControlFlowGraph, Function, InlinedProgram, Instruction};
use crate::ir::Transform;

impl<T: Transform<Instruction>> Transform<Block> for T {
    fn name(&self) -> &'static str {
        self.name()
    }

    fn description(&self) -> String {
        self.description()
    }

    fn transform(&self, block: &mut Block) -> Result<()> {
        for inst in block.instructions_mut() {
            self.transform(inst)?;
        }
        Ok(())
    }
}

impl<T: Transform<Block>> Transform<ControlFlowGraph> for T {
    fn name(&self) -> &'static str {
        self.name()
    }

    fn description(&self) -> String {
        self.description()
    }

    fn transform(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        for block in cfg.blocks_mut() {
            self.transform(block)?;
        }
        Ok(())
    }
}

impl<T: Transform<ControlFlowGraph>> Transform<Function> for T {
    fn name(&self) -> &'static str {
        self.name()
    }

    fn description(&self) -> String {
        self.description()
    }

    fn transform(&self, func: &mut Function) -> Result<()> {
        let cfg = func.control_flow_graph_mut();
        self.transform(cfg)?;
        Ok(())
    }
}

impl<T: Transform<ControlFlowGraph>> Transform<InlinedProgram> for T {
    fn name(&self) -> &'static str {
        self.name()
    }

    fn description(&self) -> String {
        self.description()
    }

    fn transform(&self, program: &mut InlinedProgram) -> Result<()> {
        let cfg = program.control_flow_graph_mut();
        self.transform(cfg)?;
        Ok(())
    }
}
