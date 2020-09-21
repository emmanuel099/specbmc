use crate::environment::SecurityLevel;
use crate::error::Result;
use crate::expr::{Expression, Variable};
use crate::hir::{analysis, Block, ControlFlowGraph};
use crate::ir::Transform;
use std::collections::{HashMap, HashSet};

#[derive(Builder, Debug)]
pub struct InitGlobalVariables {
    default_variable_security_level: SecurityLevel,
    low_security_variables: HashSet<String>,
    high_security_variables: HashSet<String>,
    initial_variable_value: HashMap<String, Expression>,
}

impl Default for InitGlobalVariables {
    fn default() -> Self {
        Self {
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
        "Set up initial state of global variables".to_string()
    }

    /// Initialize variables which are live at the entry block and make low-security variables indistinguishable
    fn transform(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
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
