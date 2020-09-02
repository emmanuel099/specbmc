use crate::environment::{Environment, Observe};
use crate::error::Result;
use crate::expr;
use crate::hir::{ControlFlowGraph, Instruction};
use crate::ir::Transform;
use std::collections::HashSet;

#[derive(Default, Builder, Debug)]
pub struct Observations {
    cache_available: bool,
    btb_available: bool,
    pht_available: bool,
    obs_end_of_program: bool,
    obs_each_effectful_instruction: bool,
    obs_after_rollback: bool,
    obs_control_flow_joins: bool,
    obs_locations: Vec<u64>,
}

impl Observations {
    pub fn new_from_env(env: &Environment) -> Self {
        let default = Self {
            cache_available: env.architecture.cache,
            btb_available: env.architecture.branch_target_buffer,
            pht_available: env.architecture.pattern_history_table,
            obs_end_of_program: false,
            obs_each_effectful_instruction: false,
            obs_after_rollback: false,
            obs_control_flow_joins: false,
            obs_locations: Vec::default(),
        };

        match env.analysis.observe {
            Observe::Sequential => Self {
                obs_end_of_program: true,
                ..default
            },
            Observe::Parallel => Self {
                obs_end_of_program: true,
                obs_each_effectful_instruction: true,
                obs_after_rollback: true,
                obs_control_flow_joins: true,
                ..default
            },
            Observe::Custom {
                end_of_program,
                each_effectful_instruction,
                after_rollback,
                control_flow_joins,
                ref locations,
            } => Self {
                obs_end_of_program: end_of_program,
                obs_each_effectful_instruction: each_effectful_instruction,
                obs_after_rollback: after_rollback,
                obs_control_flow_joins: control_flow_joins,
                obs_locations: locations.to_owned(),
                ..default
            },
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
        let rollback_block_indices: HashSet<usize> = cfg
            .edges()
            .iter()
            .filter_map(|edge| {
                if edge.labels().is_rollback() {
                    Some(edge.tail())
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

    fn place_observe_at_control_flow_joins(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        let join_block_indices: HashSet<usize> = cfg
            .blocks()
            .iter()
            .filter_map(|block| {
                let edges_in = cfg.edges_in(block.index()).unwrap();
                if edges_in.len() > 1 {
                    Some(block.index())
                } else {
                    None
                }
            })
            .collect();

        let obs_exprs = self.observable_exprs();

        for block_index in join_block_indices {
            let block = cfg.block_mut(block_index)?;

            // Add the observe at the beginning of the block
            let mut obs = Instruction::observable(obs_exprs.clone());
            obs.labels_mut().pseudo();
            block.insert_instruction(0, obs)?;
        }

        Ok(())
    }

    fn place_observe_at_program_locations(
        &self,
        cfg: &mut ControlFlowGraph,
        locations: &[u64],
    ) -> Result<()> {
        let obs_exprs = self.observable_exprs();

        for block in cfg.blocks_mut() {
            let effectful_inst_indices: Vec<usize> = block
                .instructions()
                .iter()
                .enumerate()
                .filter_map(|(index, inst)| {
                    if let Some(address) = inst.address() {
                        if locations.contains(&address) {
                            Some(index)
                        } else {
                            None
                        }
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

    fn place_observe_at_end_of_program(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        let exit_block = cfg.exit_block_mut()?;
        exit_block
            .observable(self.observable_exprs())
            .labels_mut()
            .pseudo();

        Ok(())
    }
}

impl Transform<ControlFlowGraph> for Observations {
    fn name(&self) -> &'static str {
        "Observations"
    }

    fn description(&self) -> String {
        "Add observations".to_string()
    }

    fn transform(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        if self.obs_each_effectful_instruction {
            self.place_observe_after_each_effectul_instruction(cfg)?;
        }

        if self.obs_after_rollback {
            self.place_observe_after_rollback(cfg)?;
        }

        if self.obs_control_flow_joins {
            self.place_observe_at_control_flow_joins(cfg)?;
        }

        if self.obs_end_of_program {
            self.place_observe_at_end_of_program(cfg)?;
        }

        if !self.obs_locations.is_empty() {
            self.place_observe_at_program_locations(cfg, &self.obs_locations)?;
        }

        Ok(())
    }
}
