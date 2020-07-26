use crate::environment::{Environment, SecurityLevel, WORD_SIZE};
use crate::error::Result;
use crate::expr::{
    BitVector, BranchTargetBuffer, Cache, CacheValue, Expression, Memory, PatternHistoryTable,
    Predictor, Sort,
};
use crate::hir::{analysis, Program};
use crate::ir::Transform;
use std::collections::HashSet;

#[derive(Builder, Debug)]
pub struct InitGlobalVariables {
    cache_available: bool,
    btb_available: bool,
    pht_available: bool,
    memory_default_low: bool,
    low_memory_addresses: HashSet<u64>,
    high_memory_addresses: HashSet<u64>,
    registers_default_low: bool,
    low_registers: HashSet<String>,
    high_registers: HashSet<String>,
    start_with_empty_cache: bool,
}

impl InitGlobalVariables {
    pub fn new_from_env(env: &Environment) -> Self {
        let memory_policy = &env.policy.memory;
        let register_policy = &env.policy.registers;

        Self {
            cache_available: env.architecture.cache,
            btb_available: env.architecture.branch_target_buffer,
            pht_available: env.architecture.pattern_history_table,
            memory_default_low: memory_policy.default_level == SecurityLevel::Low,
            low_memory_addresses: memory_policy.low.clone(),
            high_memory_addresses: memory_policy.high.clone(),
            registers_default_low: register_policy.default_level == SecurityLevel::Low,
            low_registers: register_policy.low.clone(),
            high_registers: register_policy.high.clone(),
            start_with_empty_cache: env.analysis.start_with_empty_cache,
        }
    }

    /// Havoc registers and make low-registers indistinguishable
    fn init_registers(&self, program: &mut Program) -> Result<()> {
        // All variables which are live at the entry block are considered uninitialized and
        // will therefore be havoced.
        let live_variables = analysis::live_variables(&program)?;
        let entry_block_index = program.control_flow_graph().entry()?;
        let uninitialized_regs = live_variables.live_at_entry(entry_block_index)?;

        let entry_block = program.control_flow_graph_mut().entry_block_mut()?;

        for reg in uninitialized_regs {
            entry_block
                .assign(reg.clone(), Expression::nondet(reg.sort().clone()))?
                .labels_mut()
                .pseudo();

            if self.low_registers.contains(reg.name())
                || (self.registers_default_low && !self.high_registers.contains(reg.name()))
            {
                entry_block
                    .indistinguishable(vec![reg.clone().into()])
                    .labels_mut()
                    .pseudo();
            }
        }

        Ok(())
    }

    /// Havoc memory and make low-addresses indistinguishable
    fn init_memory(&self, program: &mut Program) -> Result<()> {
        let entry_block = program.control_flow_graph_mut().entry_block_mut()?;

        entry_block
            .assign(Memory::variable(), Expression::nondet(Sort::memory()))?
            .labels_mut()
            .pseudo();

        if self.memory_default_low {
            println!("low addresses {:?}", self.high_memory_addresses); // TODO
            unimplemented!();
        } else {
            for address in &self.low_memory_addresses {
                entry_block
                    .indistinguishable(vec![Memory::load(
                        8,
                        Memory::variable().into(),
                        BitVector::constant_u64(*address, WORD_SIZE),
                    )?])
                    .labels_mut()
                    .pseudo();
            }
        }

        Ok(())
    }

    fn init_predictor(&self, program: &mut Program) -> Result<()> {
        let entry_block = program.control_flow_graph_mut().entry_block_mut()?;

        entry_block
            .assign(Predictor::variable(), Expression::nondet(Sort::predictor()))?
            .labels_mut()
            .pseudo();
        entry_block
            .indistinguishable(vec![Predictor::variable().into()])
            .labels_mut()
            .pseudo();

        Ok(())
    }

    fn init_cache(&self, program: &mut Program) -> Result<()> {
        if !self.cache_available {
            return Ok(());
        }

        let entry_block = program.control_flow_graph_mut().entry_block_mut()?;

        let initial_content = if self.start_with_empty_cache {
            Expression::constant(CacheValue::empty().into(), Sort::cache())
        } else {
            Expression::nondet(Sort::cache())
        };

        entry_block
            .assign(Cache::variable(), initial_content)?
            .labels_mut()
            .pseudo();
        entry_block
            .indistinguishable(vec![Cache::variable().into()])
            .labels_mut()
            .pseudo();

        Ok(())
    }

    fn init_btb(&self, program: &mut Program) -> Result<()> {
        if !self.btb_available {
            return Ok(());
        }

        let entry_block = program.control_flow_graph_mut().entry_block_mut()?;

        entry_block
            .assign(
                BranchTargetBuffer::variable(),
                Expression::nondet(Sort::branch_target_buffer()),
            )?
            .labels_mut()
            .pseudo();
        entry_block
            .indistinguishable(vec![BranchTargetBuffer::variable().into()])
            .labels_mut()
            .pseudo();

        Ok(())
    }

    fn init_pht(&self, program: &mut Program) -> Result<()> {
        if !self.pht_available {
            return Ok(());
        }

        let entry_block = program.control_flow_graph_mut().entry_block_mut()?;

        entry_block
            .assign(
                PatternHistoryTable::variable(),
                Expression::nondet(Sort::pattern_history_table()),
            )?
            .labels_mut()
            .pseudo();
        entry_block
            .indistinguishable(vec![PatternHistoryTable::variable().into()])
            .labels_mut()
            .pseudo();

        Ok(())
    }
}

impl Default for InitGlobalVariables {
    fn default() -> Self {
        Self {
            cache_available: false,
            btb_available: false,
            pht_available: false,
            memory_default_low: false,
            low_memory_addresses: HashSet::new(),
            high_memory_addresses: HashSet::new(),
            registers_default_low: true,
            low_registers: HashSet::new(),
            high_registers: HashSet::new(),
            start_with_empty_cache: false,
        }
    }
}

impl Transform<Program> for InitGlobalVariables {
    fn name(&self) -> &'static str {
        "InitGlobalVariables"
    }

    fn description(&self) -> String {
        "Set up initial state".to_string()
    }

    fn transform(&self, program: &mut Program) -> Result<()> {
        self.init_memory(program)?;
        self.init_predictor(program)?;
        self.init_cache(program)?;
        self.init_btb(program)?;
        self.init_pht(program)?;
        self.init_registers(program)?;

        Ok(())
    }
}
