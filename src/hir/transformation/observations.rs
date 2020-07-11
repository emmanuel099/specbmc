use crate::environment::Environment;
use crate::error::Result;
use crate::expr;
use crate::hir::{ControlFlowGraph, Instruction, Program};
use crate::ir::Transform;

#[derive(Default, Builder, Debug)]
pub struct Observations {
    cache_available: bool,
    btb_available: bool,
    pht_available: bool,
    obs_end_of_program: bool,
    obs_each_effectful_instruction: bool,
    obs_after_rollback: bool,
}

impl Observations {
    pub fn new_from_env(env: &Environment) -> Self {
        Self {
            cache_available: env.architecture.cache,
            btb_available: env.architecture.branch_target_buffer,
            pht_available: env.architecture.pattern_history_table,
            obs_end_of_program: env.analysis.observe.end_of_program,
            obs_each_effectful_instruction: env.analysis.observe.each_effectful_instruction,
            obs_after_rollback: env.analysis.observe.after_rollback,
        }
    }

    fn observable_exprs(&self) -> Vec<expr::Expression> {
        let mut exprs = Vec::new();

        if self.cache_available {
            exprs.push(expr::Cache::variable().into());
        }
        if self.btb_available {
            exprs.push(expr::BranchTargetBuffer::variable().into());
        }
        if self.pht_available {
            exprs.push(expr::PatternHistoryTable::variable().into());
        }

        exprs
    }

    fn place_observe_after_each_effectul_instruction(
        &self,
        cfg: &mut ControlFlowGraph,
    ) -> Result<()> {
        let obs_exprs = self.observable_exprs();

        for block in cfg.blocks_mut() {
            let effectful_inst_indices: Vec<usize> = block
                .instructions()
                .iter()
                .enumerate()
                .filter_map(|(index, inst)| {
                    if inst.has_effects() {
                        Some(index)
                    } else {
                        None
                    }
                })
                .collect();

            for index in effectful_inst_indices.iter().rev() {
                let mut obs = Instruction::observable(obs_exprs.clone());
                obs.labels_mut().pseudo();
                block.insert_instruction(index + 1, obs)?;
            }
        }

        Ok(())
    }

    fn place_observe_after_rollback(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        let rollback_block_indices: Vec<usize> = cfg
            .blocks()
            .iter()
            .filter_map(|block| {
                let has_incoming_rollback_edge: bool = cfg
                    .edges_in(block.index())
                    .unwrap()
                    .iter()
                    .any(|edge| edge.labels().is_rollback());

                if has_incoming_rollback_edge {
                    Some(block.index())
                } else {
                    None
                }
            })
            .collect();

        let obs_exprs = self.observable_exprs();

        for block_index in rollback_block_indices {
            let block = cfg.block_mut(block_index)?;

            // Add the observe at the beginning of the block
            let mut obs = Instruction::observable(obs_exprs.clone());
            obs.labels_mut().pseudo();
            block.insert_instruction(0, obs)?;
        }

        Ok(())
    }

    fn place_observe_at_end_of_program(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        let exit_block = cfg.exit_block_mut()?;
        exit_block
            .observable(self.observable_exprs())
            .labels_mut()
            .pseudo();

        Ok(())
    }
}

impl Transform<Program> for Observations {
    fn name(&self) -> &'static str {
        "Observations"
    }

    fn description(&self) -> &'static str {
        "Add observations"
    }

    fn transform(&self, program: &mut Program) -> Result<()> {
        let cfg = program.control_flow_graph_mut();

        if self.obs_each_effectful_instruction {
            self.place_observe_after_each_effectul_instruction(cfg)?;
        }

        if self.obs_after_rollback {
            self.place_observe_after_rollback(cfg)?;
        }

        if self.obs_end_of_program {
            self.place_observe_at_end_of_program(cfg)?;
        }

        Ok(())
    }
}
