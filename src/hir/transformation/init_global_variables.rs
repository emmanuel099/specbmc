use crate::environment::{Environment, SecurityLevel};
use crate::error::Result;
use crate::expr::{
    BitVector, BranchTargetBuffer, Cache, Expression, Memory, PatternHistoryTable, Predictor, Sort,
    Variable,
};
use crate::hir::{analysis, Block, Program};
use crate::util::Transform;
use std::collections::HashSet;

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
}

impl InitGlobalVariables {
    pub fn new() -> Self {
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
        }
    }

    pub fn new_from_env(env: &Environment) -> Self {
        let memory_policy = env.policy().memory();
        let register_policy = env.policy().registers();

        Self {
            cache_available: env.architecture().cache(),
            btb_available: env.architecture().branch_target_buffer(),
            pht_available: env.architecture().pattern_history_table(),
            memory_default_low: memory_policy.default_level() == SecurityLevel::Low,
            low_memory_addresses: memory_policy.low().clone(),
            high_memory_addresses: memory_policy.high().clone(),
            registers_default_low: register_policy.default_level() == SecurityLevel::Low,
            low_registers: register_policy.low().clone(),
            high_registers: register_policy.high().clone(),
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

    /// Havoc registers and make low-registers indistinguishable
    fn init_registers(&self, regs: &HashSet<Variable>, entry_block: &mut Block) -> Result<()> {
        for reg in regs {
            entry_block.assign(reg.clone(), Expression::nondet(reg.sort().clone()))?;

            if self.registers_default_low {
                if !self.high_registers.contains(reg.name()) {
                    entry_block.indistinguishable(vec![reg.clone().into()]);
                }
            } else {
                if self.low_registers.contains(reg.name()) {
                    entry_block.indistinguishable(vec![reg.clone().into()]);
                }
            }
        }

        Ok(())
    }

    /// Havoc memory and make low-addresses indistinguishable
    fn init_memory(&self, entry_block: &mut Block) -> Result<()> {
        entry_block.assign(Memory::variable(), Expression::nondet(Sort::memory()))?;

        if self.memory_default_low {
            println!("low addresses {:?}", self.high_memory_addresses); // TODO
            unimplemented!();
        } else {
            for address in &self.low_memory_addresses {
                entry_block.indistinguishable(vec![Memory::load(
                    8,
                    Memory::variable().into(),
                    BitVector::constant_u64(*address, 64),
                )?]);
            }
        }

        Ok(())
    }

    fn init_predictor(&self, entry_block: &mut Block) -> Result<()> {
        entry_block.assign(Predictor::variable(), Expression::nondet(Sort::predictor()))?;
        entry_block.indistinguishable(vec![Predictor::variable().into()]);

        Ok(())
    }

    fn init_microarchitectual_components(&self, entry_block: &mut Block) -> Result<()> {
        if self.cache_available {
            entry_block.assign(Cache::variable(), Expression::nondet(Sort::cache()))?;
            entry_block.indistinguishable(vec![Cache::variable().into()]);
        }

        if self.btb_available {
            entry_block.assign(
                BranchTargetBuffer::variable(),
                Expression::nondet(Sort::branch_target_buffer()),
            )?;
            entry_block.indistinguishable(vec![BranchTargetBuffer::variable().into()]);
        }

        if self.pht_available {
            entry_block.assign(
                PatternHistoryTable::variable(),
                Expression::nondet(Sort::pattern_history_table()),
            )?;
            entry_block.indistinguishable(vec![PatternHistoryTable::variable().into()]);
        }

        Ok(())
    }
}

impl Transform<Program> for InitGlobalVariables {
    fn transform(&self, program: &mut Program) -> Result<()> {
        let global_variables = analysis::global_variables(&program);

        let cfg = program.control_flow_graph_mut();
        let entry_block = cfg.entry_block_mut().ok_or("CFG entry must be set")?;

        self.init_memory(entry_block)?;
        self.init_predictor(entry_block)?;
        self.init_microarchitectual_components(entry_block)?;
        self.init_registers(&global_variables, entry_block)?;

        Ok(())
    }
}
