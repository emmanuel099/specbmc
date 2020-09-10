use crate::error::Result;
use crate::expr::Variable;
use crate::hir::{Block, ControlFlowGraph, Instruction};
use crate::ir::Transform;
use std::collections::HashSet;

#[derive(Default, Builder, Debug)]
pub struct Observations {
    observable_variables: HashSet<Variable>,
    observe_variable_writes: bool,
    observe_at_control_flow_joins: bool,
    observe_at_end_of_program: bool,
}

impl Observations {
    fn place_observe_at_variable_writes(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        for block in cfg.blocks_mut() {
            let observable_writes: Vec<(usize, Vec<Variable>)> = block
                .instructions()
                .iter()
                .enumerate()
                .filter_map(|(index, inst)| {
                    let vars: Vec<Variable> = inst
                        .variables_written()
                        .into_iter()
                        .filter(|var| self.observable_variables.contains(var))
                        .cloned()
                        .collect();

                    if vars.is_empty() {
                        None
                    } else {
                        Some((index, vars))
                    }
                })
                .collect();

            for (index, vars) in observable_writes.iter().rev() {
                for var in vars {
                    let mut obs = Instruction::observable(var.clone().into());
                    obs.labels_mut().pseudo();
                    block.insert_instruction(index + 1, obs)?;
                }
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
        for var in &self.observable_variables {
            let mut obs = Instruction::observable(var.clone().into());
            obs.labels_mut().pseudo();
            block.insert_instruction(index, obs)?;
        }

        Ok(())
    }

    fn append_observe_instruction(&self, block: &mut Block) {
        for var in &self.observable_variables {
            let obs = block.observable(var.clone().into());
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
        if self.observe_variable_writes {
            self.place_observe_at_variable_writes(cfg)?;
        }

        if self.observe_at_control_flow_joins {
            self.place_observe_at_control_flow_joins(cfg)?;
        }

        if self.observe_at_end_of_program {
            self.place_observe_at_end_of_program(cfg)?;
        }

        Ok(())
    }
}
