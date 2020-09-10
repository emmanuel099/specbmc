use crate::environment::SecurityLevel;
use crate::error::Result;
use crate::expr::{
    BitVector, BranchTargetBuffer, Cache, CacheValue, Expression, Memory, PatternHistoryTable,
    Predictor, Sort, Variable,
};
use crate::hir::{analysis, Block, ControlFlowGraph};
use crate::ir::Transform;
use std::collections::{BTreeSet, HashSet};

#[derive(Builder, Debug)]
pub struct InitGlobalVariables {
    cache_available: bool,
    btb_available: bool,
    pht_available: bool,
    memory_default_security_level: SecurityLevel,
    low_memory_addresses: BTreeSet<u64>,
    high_memory_addresses: BTreeSet<u64>,
    registers_default_security_level: SecurityLevel,
    low_registers: HashSet<String>,
    high_registers: HashSet<String>,
    start_with_empty_cache: bool,
}

impl InitGlobalVariables {
    /// Havoc registers and make low-registers indistinguishable
    fn init_registers(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        // All variables which are live at the entry block are considered uninitialized and
        // will therefore be havoced.
        let live_variables = analysis::live_variables(&cfg)?;
        let entry_block_index = cfg.entry()?;
        let uninitialized_regs = live_variables.live_at_entry(entry_block_index)?;

        let entry_block = cfg.entry_block_mut()?;

        for reg in uninitialized_regs {
            havoc_variable(entry_block, reg.clone())?;

            match self.registers_default_security_level {
                SecurityLevel::Low => {
                    if !self.high_registers.contains(reg.name()) {
                        low_equivalent(entry_block, reg.clone().into());
                    }
                }
                SecurityLevel::High => {
                    if self.low_registers.contains(reg.name()) {
                        low_equivalent(entry_block, reg.clone().into());
                    }
                }
            }
        }

        Ok(())
    }

    /// Havoc memory and make low-addresses indistinguishable
    fn init_memory(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        let entry_block = cfg.entry_block_mut()?;

        havoc_variable(entry_block, Memory::variable())?;

        match self.memory_default_security_level {
            SecurityLevel::Low => {
                low_equivalent(entry_block, Memory::variable().into());
                for address in &self.high_memory_addresses {
                    let secret_var = BitVector::variable("_secret", 8);
                    havoc_variable(entry_block, secret_var.clone())?;
                    let addr = BitVector::word_constant(*address);
                    entry_block
                        .store(addr, secret_var.into())?
                        .labels_mut()
                        .pseudo();
                }
            }
            SecurityLevel::High => {
                for address in &self.low_memory_addresses {
                    let addr = BitVector::word_constant(*address);
                    let memory_content_at_address =
                        Memory::load(8, Memory::variable().into(), addr)?;
                    low_equivalent(entry_block, memory_content_at_address);
                }
            }
        }

        Ok(())
    }

    fn init_predictor(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        let entry_block = cfg.entry_block_mut()?;

        havoc_variable(entry_block, Predictor::variable())?;
        low_equivalent(entry_block, Predictor::variable().into());

        Ok(())
    }

    fn init_cache(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        if !self.cache_available {
            return Ok(());
        }

        let entry_block = cfg.entry_block_mut()?;

        if self.start_with_empty_cache {
            let empty_cache = Expression::constant(CacheValue::empty().into(), Sort::cache());
            entry_block
                .assign(Cache::variable(), empty_cache)?
                .labels_mut()
                .pseudo();
        } else {
            havoc_variable(entry_block, Cache::variable())?;
        }

        low_equivalent(entry_block, Cache::variable().into());

        Ok(())
    }

    fn init_btb(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        if !self.btb_available {
            return Ok(());
        }

        let entry_block = cfg.entry_block_mut()?;

        havoc_variable(entry_block, BranchTargetBuffer::variable())?;
        low_equivalent(entry_block, BranchTargetBuffer::variable().into());

        Ok(())
    }

    fn init_pht(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        if !self.pht_available {
            return Ok(());
        }

        let entry_block = cfg.entry_block_mut()?;

        havoc_variable(entry_block, PatternHistoryTable::variable())?;
        low_equivalent(entry_block, PatternHistoryTable::variable().into());

        Ok(())
    }
}

impl Default for InitGlobalVariables {
    fn default() -> Self {
        Self {
            cache_available: false,
            btb_available: false,
            pht_available: false,
            memory_default_security_level: SecurityLevel::High,
            low_memory_addresses: BTreeSet::new(),
            high_memory_addresses: BTreeSet::new(),
            registers_default_security_level: SecurityLevel::Low,
            low_registers: HashSet::new(),
            high_registers: HashSet::new(),
            start_with_empty_cache: false,
        }
    }
}

impl Transform<ControlFlowGraph> for InitGlobalVariables {
    fn name(&self) -> &'static str {
        "InitGlobalVariables"
    }

    fn description(&self) -> String {
        "Set up initial state".to_string()
    }

    fn transform(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        self.init_memory(cfg)?;
        self.init_predictor(cfg)?;
        self.init_cache(cfg)?;
        self.init_btb(cfg)?;
        self.init_pht(cfg)?;
        self.init_registers(cfg)?;

        Ok(())
    }
}

fn havoc_variable(block: &mut Block, var: Variable) -> Result<()> {
    block
        .assign(var.clone(), Expression::nondet(var.sort().to_owned()))?
        .labels_mut()
        .pseudo();

    Ok(())
}

fn low_equivalent(block: &mut Block, expr: Expression) {
    block.indistinguishable(expr).labels_mut().pseudo();
}
