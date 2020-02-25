use crate::error::Result;
use crate::expr;
use crate::hir::Program;

pub struct Observations {
    cache_available: bool,
    btb_available: bool,
    pht_available: bool,
}

impl Observations {
    pub fn new() -> Self {
        Self {
            cache_available: false,
            btb_available: false,
            pht_available: false,
        }
    }

    /// Enable or disable Cache observations.
    pub fn with_cache(&mut self, available: bool) -> &mut Self {
        self.cache_available = available;
        self
    }

    /// Enable or disable Branch Target Buffer observations.
    pub fn with_branch_target_buffer(&mut self, available: bool) -> &mut Self {
        self.btb_available = available;
        self
    }

    /// Enable or disable Pattern History Table observations.
    pub fn with_pattern_history_table(&mut self, available: bool) -> &mut Self {
        self.pht_available = available;
        self
    }

    pub fn add_observations(&self, program: &mut Program) -> Result<()> {
        let cfg = program.control_flow_graph_mut();

        let mut variables = Vec::new();
        if self.cache_available {
            variables.push(expr::Cache::variable());
        }
        if self.btb_available {
            variables.push(expr::BranchTargetBuffer::variable());
        }
        if self.pht_available {
            variables.push(expr::PatternHistoryTable::variable());
        }

        // Place an observe at the end of the program
        let exit_block = cfg.exit_block_mut().ok_or("CFG exit must be set")?;
        exit_block.observe(variables);

        // TODO add more obs if defined so

        Ok(())
    }
}
