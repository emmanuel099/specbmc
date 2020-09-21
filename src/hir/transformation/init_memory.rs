use crate::environment::SecurityLevel;
use crate::error::Result;
use crate::expr::{BitVector, Expression, Memory, Variable};
use crate::hir::{Block, ControlFlowGraph};
use crate::ir::Transform;
use std::collections::BTreeSet;

#[derive(Builder, Debug)]
pub struct InitMemory {
    default_memory_security_level: SecurityLevel,
    low_security_memory_addresses: BTreeSet<u64>,
    high_security_memory_addresses: BTreeSet<u64>,
}

impl Default for InitMemory {
    fn default() -> Self {
        Self {
            default_memory_security_level: SecurityLevel::High,
            low_security_memory_addresses: BTreeSet::new(),
            high_security_memory_addresses: BTreeSet::new(),
        }
    }
}

impl Transform<ControlFlowGraph> for InitMemory {
    fn name(&self) -> &'static str {
        "InitMemory"
    }

    fn description(&self) -> String {
        "Set up initial memory state".to_string()
    }

    /// Initialize memory and make low-addresses indistinguishable
    fn transform(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
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
