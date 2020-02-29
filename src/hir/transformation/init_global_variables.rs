use crate::error::Result;
use crate::expr::{
    BitVector, BranchTargetBuffer, Cache, Expression, Memory, PatternHistoryTable, Predictor, Sort,
    Variable,
};
use crate::hir::{analysis, Block, Program};
use std::collections::HashSet;

pub struct InitGlobalVariables {
    cache_available: bool,
    btb_available: bool,
    pht_available: bool,
    nonspec_effects: bool,
    low_memory_addresses: Vec<u64>,
    high_registers: HashSet<String>,
}

impl InitGlobalVariables {
    pub fn new() -> Self {
        Self {
            cache_available: false,
            btb_available: false,
            pht_available: false,
            nonspec_effects: false,
            low_memory_addresses: Vec::new(),
            high_registers: HashSet::new(),
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

    /// Enable or disable non-speculative effects.
    pub fn with_nonspec_effects(&mut self, nonspec_effects: bool) -> &mut Self {
        self.nonspec_effects = nonspec_effects;
        self
    }

    pub fn transform(&self, program: &mut Program) -> Result<()> {
        let global_variables = analysis::global_variables(&program);

        let cfg = program.control_flow_graph_mut();
        let entry_block = cfg.entry_block_mut().ok_or("CFG entry must be set")?;

        self.init_memory(entry_block)?;
        self.init_predictor(entry_block)?;
        self.init_microarchitectual_components(entry_block)?;
        self.init_registers(&global_variables, entry_block)?;

        Ok(())
    }

    /// Havoc registers and make low-registers indistinguishable
    fn init_registers(&self, regs: &HashSet<Variable>, entry_block: &mut Block) -> Result<()> {
        for reg in regs {
            entry_block.assign(reg.clone(), Expression::nondet(reg.sort().clone()))?;

            if !self.high_registers.contains(reg.name()) {
                entry_block.indistinguishable(vec![reg.clone().into()]);
            }
        }

        Ok(())
    }

    /// Havoc memory and make low-addresses indistinguishable
    fn init_memory(&self, entry_block: &mut Block) -> Result<()> {
        entry_block.assign(Memory::variable(), Expression::nondet(Sort::memory()))?;

        for address in &self.low_memory_addresses {
            entry_block.indistinguishable(vec![Memory::load(
                8,
                Memory::variable().into(),
                BitVector::constant_u64(*address, 64),
            )?]);
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

            if self.nonspec_effects {
                entry_block.assign(Cache::variable_nonspec(), Cache::variable().into())?;
            }
        }

        if self.btb_available {
            entry_block.assign(
                BranchTargetBuffer::variable(),
                Expression::nondet(Sort::branch_target_buffer()),
            )?;
            entry_block.indistinguishable(vec![BranchTargetBuffer::variable().into()]);

            if self.nonspec_effects {
                entry_block.assign(
                    BranchTargetBuffer::variable_nonspec(),
                    BranchTargetBuffer::variable().into(),
                )?;
            }
        }

        if self.pht_available {
            entry_block.assign(
                PatternHistoryTable::variable(),
                Expression::nondet(Sort::pattern_history_table()),
            )?;
            entry_block.indistinguishable(vec![PatternHistoryTable::variable().into()]);

            if self.nonspec_effects {
                entry_block.assign(
                    PatternHistoryTable::variable_nonspec(),
                    PatternHistoryTable::variable().into(),
                )?;
            }
        }

        Ok(())
    }
}
