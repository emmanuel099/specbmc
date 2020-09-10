use crate::environment;
use crate::expr;
use std::collections::{BTreeSet, HashMap, HashSet};

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

pub use self::explicit_effects::{ExplicitEffects, ExplicitEffectsBuilder};
pub use self::function_inlining::{FunctionInlining, FunctionInliningBuilder};
pub use self::init_global_variables::{InitGlobalVariables, InitGlobalVariablesBuilder};
pub use self::instruction_effects::{InstructionEffects, InstructionEffectsBuilder};
pub use self::loop_unwinding::{LoopUnwinding, LoopUnwindingBuilder};
pub use self::non_spec_obs_equiv::{NonSpecObsEquivalence, NonSpecObsEquivalenceBuilder};
pub use self::observations::{Observations, ObservationsBuilder};
pub use self::optimization::Optimizer;
pub use self::ssa_transformation::{SSAForm, SSATransformation};
pub use self::transient_execution::{TransientExecution, TransientExecutionBuilder};

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

pub fn create_transformations(
    env: &environment::Environment,
) -> Result<Vec<Box<dyn Transform<InlinedProgram>>>> {
    let mut steps: Vec<Box<dyn Transform<InlinedProgram>>> = Vec::new();

    steps.push(Box::new(
        LoopUnwindingBuilder::default()
            .unwinding_bound(env.analysis.unwind)
            .unwinding_guard(env.analysis.unwinding_guard)
            .build()?,
    ));

    steps.push(Box::new(
        InstructionEffectsBuilder::default()
            .model_cache_effects(env.architecture.cache)
            .model_btb_effects(env.architecture.branch_target_buffer)
            .model_pht_effects(env.architecture.pattern_history_table)
            .build()?,
    ));

    if env.analysis.check != environment::Check::OnlyNormalExecutionLeaks {
        steps.push(Box::new(
            TransientExecutionBuilder::default()
                .spectre_pht(env.analysis.spectre_pht)
                .spectre_stl(env.analysis.spectre_stl)
                .predictor_strategy(env.analysis.predictor_strategy)
                .speculation_window(env.architecture.speculation_window)
                .intermediate_resolve(env.analysis.observe != environment::Observe::Parallel)
                .build()?,
        ));
    }

    match env.analysis.observe {
        environment::Observe::Sequential => {
            steps.push(Box::new(
                ObservationsBuilder::default()
                    .cache_available(env.architecture.cache)
                    .btb_available(env.architecture.branch_target_buffer)
                    .pht_available(env.architecture.pattern_history_table)
                    .obs_end_of_program(true)
                    .obs_effectful_instructions(false)
                    .obs_control_flow_joins(false)
                    .build()?,
            ));
        }
        environment::Observe::Parallel | environment::Observe::Full => {
            steps.push(Box::new(
                ObservationsBuilder::default()
                    .cache_available(env.architecture.cache)
                    .btb_available(env.architecture.branch_target_buffer)
                    .pht_available(env.architecture.pattern_history_table)
                    .obs_end_of_program(true)
                    .obs_effectful_instructions(true)
                    .obs_control_flow_joins(true)
                    .build()?,
            ));
        }
    }

    steps.push(Box::new(ExplicitEffects::default()));

    {
        let low_security_memory_addresses = address_ranges_to_addresses(&env.policy.memory.low);
        let high_security_memory_addresses = address_ranges_to_addresses(&env.policy.memory.high);

        let mut low_security_variables = env.policy.registers.low.clone();
        low_security_variables.insert(expr::Predictor::variable().name().to_owned());
        low_security_variables.insert(expr::Cache::variable().name().to_owned());
        low_security_variables.insert(expr::BranchTargetBuffer::variable().name().to_owned());
        low_security_variables.insert(expr::PatternHistoryTable::variable().name().to_owned());

        let high_security_variables = env.policy.registers.high.clone();

        let mut initial_variable_value = HashMap::new();
        if env.analysis.start_with_empty_cache {
            let empty_cache =
                expr::Expression::constant(expr::CacheValue::empty().into(), expr::Sort::cache());
            initial_variable_value.insert(expr::Cache::variable().name().to_owned(), empty_cache);
        }

        steps.push(Box::new(
            InitGlobalVariablesBuilder::default()
                .default_memory_security_level(env.policy.memory.default_level)
                .low_security_memory_addresses(low_security_memory_addresses)
                .high_security_memory_addresses(high_security_memory_addresses)
                .default_variable_security_level(env.policy.registers.default_level)
                .low_security_variables(low_security_variables)
                .high_security_variables(high_security_variables)
                .initial_variable_value(initial_variable_value)
                .build()?,
        ));
    }

    if env.analysis.check == environment::Check::OnlyTransientExecutionLeaks {
        steps.push(Box::new(
            NonSpecObsEquivalenceBuilder::default()
                .cache_available(env.architecture.cache)
                .btb_available(env.architecture.branch_target_buffer)
                .pht_available(env.architecture.pattern_history_table)
                .build()?,
        ));
    }

    steps.push(Box::new(SSATransformation::new(SSAForm::Pruned)));

    match env.optimization_level {
        environment::OptimizationLevel::Disabled => {}
        environment::OptimizationLevel::Basic => {
            steps.push(Box::new(Optimizer::basic()));
        }
        environment::OptimizationLevel::Full => {
            steps.push(Box::new(Optimizer::full()));
        }
    }

    Ok(steps)
}

fn address_ranges_to_addresses(
    address_ranges: &HashSet<environment::AddressRange>,
) -> BTreeSet<u64> {
    let mut addresses = BTreeSet::new();

    address_ranges.iter().for_each(|range| {
        for addr in range.addresses() {
            addresses.insert(addr);
        }
    });

    addresses
}
