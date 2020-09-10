use crate::environment::SecurityLevel;
use crate::error::Result;
use crate::expr::{BitVector, Expression, Memory, Variable};
use crate::hir::{analysis, Block, ControlFlowGraph};
use crate::ir::Transform;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Builder, Debug)]
pub struct InitGlobalVariables {
    default_memory_security_level: SecurityLevel,
    low_security_memory_addresses: BTreeSet<u64>,
    high_security_memory_addresses: BTreeSet<u64>,
    default_variable_security_level: SecurityLevel,
    low_security_variables: HashSet<String>,
    high_security_variables: HashSet<String>,
    initial_variable_value: HashMap<String, Expression>,
}

impl InitGlobalVariables {
    /// Initialize variables which are live at the entry block and make low-security variables indistinguishable
    fn init_live_variables(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        let live_variables = analysis::live_variables(&cfg)?;
        let entry_block_index = cfg.entry()?;
        let uninitialized_vars = live_variables.live_at_entry(entry_block_index)?;

        let entry_block = cfg.entry_block_mut()?;

        for var in uninitialized_vars {
            if let Some(value) = self.initial_variable_value.get(var.name()) {
                assign_variable(entry_block, var.clone(), value.clone())?;
            } else {
                havoc_variable(entry_block, var.clone())?;
            }

            match self.default_variable_security_level {
                SecurityLevel::Low => {
                    if !self.high_security_variables.contains(var.name()) {
                        low_equivalent(entry_block, var.clone().into());
                    }
                }
                SecurityLevel::High => {
                    if self.low_security_variables.contains(var.name()) {
                        low_equivalent(entry_block, var.clone().into());
                    }
                }
            }
        }

        Ok(())
    }

    /// Initialize memory and make low-addresses indistinguishable
    fn init_memory(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        let entry_block = cfg.entry_block_mut()?;

        havoc_variable(entry_block, Memory::variable())?;

        match self.default_memory_security_level {
            SecurityLevel::Low => {
                low_equivalent(entry_block, Memory::variable().into());
                for address in &self.high_security_memory_addresses {
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
                for address in &self.low_security_memory_addresses {
                    let addr = BitVector::word_constant(*address);
                    let memory_content_at_address =
                        Memory::load(8, Memory::variable().into(), addr)?;
                    low_equivalent(entry_block, memory_content_at_address);
                }
            }
        }

        Ok(())
    }
}

impl Default for InitGlobalVariables {
    fn default() -> Self {
        Self {
            default_memory_security_level: SecurityLevel::High,
            low_security_memory_addresses: BTreeSet::new(),
            high_security_memory_addresses: BTreeSet::new(),
            default_variable_security_level: SecurityLevel::Low,
            low_security_variables: HashSet::new(),
            high_security_variables: HashSet::new(),
            initial_variable_value: HashMap::new(),
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
        self.init_live_variables(cfg)?;

        Ok(())
    }
}

fn havoc_variable(block: &mut Block, var: Variable) -> Result<()> {
    let value = Expression::nondet(var.sort().clone());
    assign_variable(block, var, value)
}

fn assign_variable(block: &mut Block, var: Variable, value: Expression) -> Result<()> {
    block.assign(var, value)?.labels_mut().pseudo();
    Ok(())
}

fn low_equivalent(block: &mut Block, expr: Expression) {
    block.indistinguishable(expr).labels_mut().pseudo();
}
