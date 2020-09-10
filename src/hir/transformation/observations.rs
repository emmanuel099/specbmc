use crate::error::Result;
use crate::expr;
use crate::hir::{Block, ControlFlowGraph, Instruction};
use crate::ir::Transform;
use std::collections::HashSet;

#[derive(Default, Builder, Debug)]
pub struct Observations {
    cache_available: bool,
    btb_available: bool,
    pht_available: bool,
    obs_end_of_program: bool,
    obs_effectful_instructions: bool,
    obs_control_flow_joins: bool,
}

impl Observations {
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
                self.insert_observe_instruction_at(block, index + 1)?;
            }
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

        for block_index in join_block_indices {
            // Add the observe at the beginning of the block
            let block = cfg.block_mut(block_index)?;
            self.insert_observe_instruction_at(block, 0)?;
        }

        Ok(())
    }

    fn place_observe_at_end_of_program(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        let exit_block = cfg.exit_block_mut()?;
        self.append_observe_instruction(exit_block);

        Ok(())
    }

    fn insert_observe_instruction_at(&self, block: &mut Block, index: usize) -> Result<()> {
        for expr in self.observable_exprs() {
            let mut obs = Instruction::observable(expr);
            obs.labels_mut().pseudo();
            block.insert_instruction(index, obs)?;
        }

        Ok(())
    }

    fn append_observe_instruction(&self, block: &mut Block) {
        for expr in self.observable_exprs() {
            let obs = block.observable(expr);
            obs.labels_mut().pseudo();
        }
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
        if self.obs_effectful_instructions {
            self.place_observe_after_each_effectul_instruction(cfg)?;
        }

        if self.obs_control_flow_joins {
            self.place_observe_at_control_flow_joins(cfg)?;
        }

        if self.obs_end_of_program {
            self.place_observe_at_end_of_program(cfg)?;
        }

        Ok(())
    }
}
