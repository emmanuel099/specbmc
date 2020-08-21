use crate::environment::Environment;
use crate::error::Result;
use crate::hir::{Block, ControlFlowGraph, InlinedProgram, Operation, Program};
use std::convert::TryInto;

#[derive(Default, Builder, Debug)]
pub struct FunctionInlining {
    recursion_limit: usize,
}

impl FunctionInlining {
    pub fn new_from_env(env: &Environment) -> Self {
        Self {
            recursion_limit: env.analysis.unwind,
        }
    }

    pub fn inline(&self, program: &Program) -> Result<InlinedProgram> {
        let entry_func = program.entry_function().ok_or("no entry function")?;
        let mut cfg = entry_func.control_flow_graph().clone();
        inline_calls(&mut cfg, program)?;
        //cfg.simplify()?; TODO
        Ok(InlinedProgram::new(cfg))
    }
}

fn inline_calls(cfg: &mut ControlFlowGraph, program: &Program) -> Result<()> {
    let mut remaining_block_indices: Vec<usize> =
        cfg.blocks().into_iter().map(Block::index).collect();

    while let Some(block_index) = remaining_block_indices.pop() {
        let block = cfg.block(block_index)?;

        if let Some((instruction_index, address)) = find_next_call_in_block(block) {
            let ret_block_index = cfg.split_block_at(block_index, instruction_index + 1)?;

            let func = program.function_by_address(address).unwrap(); // FIXME
            let mapping = cfg.insert(func.control_flow_graph())?;
            let func_entry_block_index = mapping
                .get(&func.control_flow_graph().entry().unwrap())
                .unwrap(); // FIXME
            let func_exit_block_index = mapping
                .get(&func.control_flow_graph().exit().unwrap())
                .unwrap(); // FIXME

            cfg.unconditional_edge(block_index, *func_entry_block_index)?
                .labels_mut()
                .call();

            cfg.unconditional_edge(*func_exit_block_index, ret_block_index)?
                .labels_mut()
                .r#return();

            // TODO increment call depth of func
            remaining_block_indices.extend(mapping.values());
            remaining_block_indices.push(ret_block_index); // TODO decrement call depth of func when this block is processed
        }
    }

    Ok(())
}

fn find_next_call_in_block(block: &Block) -> Option<(usize, u64)> {
    for (index, inst) in block.instructions().iter().enumerate() {
        if let Operation::Call { target } = inst.operation() {
            if let Some(address) = target.try_into().ok() {
                return Some((index, address));
            }
        }
    }
    None
}
