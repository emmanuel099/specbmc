use crate::environment::Environment;
use crate::error::Result;
use crate::hir::{Block, ControlFlowGraph, InlinedProgram, Operation, Program};
use std::collections::BTreeMap;
use std::convert::TryInto;

#[derive(Default, Builder, Debug)]
pub struct FunctionInlining {
    recursion_limit: usize,
}

type CallDepth = BTreeMap<u64, usize>;

impl FunctionInlining {
    pub fn new_from_env(env: &Environment) -> Self {
        Self {
            recursion_limit: env.analysis.unwind,
        }
    }

    pub fn inline(&self, program: &Program) -> Result<InlinedProgram> {
        let entry_func = program
            .entry_function()
            .ok_or("no entry function defined")?;
        let mut cfg = entry_func.control_flow_graph().clone();
        self.inline_calls(&mut cfg, program)?;
        cfg.simplify()?;
        Ok(InlinedProgram::new(cfg))
    }

    fn inline_calls(&self, cfg: &mut ControlFlowGraph, program: &Program) -> Result<()> {
        let mut remaining_block_indices: Vec<(usize, CallDepth)> = Vec::new();

        cfg.blocks()
            .into_iter()
            .map(Block::index)
            .for_each(|block_index| {
                remaining_block_indices.push((block_index, CallDepth::default()));
            });

        while let Some((block_index, call_depth_in_caller)) = remaining_block_indices.pop() {
            let block = cfg.block(block_index)?;

            if let Some((instruction_index, address)) = find_next_call_in_block(block) {
                if let Some(func) = program.function_by_address(address) {
                    let func_call_depth = call_depth_in_caller
                        .get(&address)
                        .cloned()
                        .unwrap_or_default();
                    if func_call_depth >= self.recursion_limit {
                        continue;
                    }

                    let ret_block_index = cfg.split_block_at(block_index, instruction_index + 1)?;

                    let func_block_index_mapping = cfg.insert(func.control_flow_graph())?;
                    let func_entry_block_index = func_block_index_mapping
                        .get(&func.control_flow_graph().entry()?)
                        .unwrap();
                    let func_exit_block_index = func_block_index_mapping
                        .get(&func.control_flow_graph().exit()?)
                        .unwrap();

                    cfg.unconditional_edge(block_index, *func_entry_block_index)?
                        .labels_mut()
                        .call();

                    cfg.unconditional_edge(*func_exit_block_index, ret_block_index)?
                        .labels_mut()
                        .r#return();

                    // Increase call depth for callee
                    let mut call_depth_in_callee = call_depth_in_caller.clone();
                    call_depth_in_callee
                        .entry(address)
                        .and_modify(|depth| *depth += 1)
                        .or_insert(1);

                    for &callee_block_index in func_block_index_mapping.values() {
                        remaining_block_indices
                            .push((callee_block_index, call_depth_in_callee.clone()));
                    }

                    // Continue at the return block with the caller call depth
                    remaining_block_indices.push((ret_block_index, call_depth_in_caller));
                }
            }
        }

        Ok(())
    }
}

fn find_next_call_in_block(block: &Block) -> Option<(usize, u64)> {
    for (index, inst) in block.instructions().iter().enumerate() {
        if let Operation::Call { target } = inst.operation() {
            if let Ok(address) = target.try_into() {
                return Some((index, address));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::expr::{BitVector, Boolean};
    use crate::hir::{Function, ProgramEntry};
    use crate::util::RenderGraph;

    use std::path::Path;

    fn debug_cfg(test_name: &str, inlined_cfg: &ControlFlowGraph, expected_cfg: &ControlFlowGraph) {
        inlined_cfg
            .render_to_file(Path::new(&format!("{}_inlined.dot", test_name)))
            .unwrap();
        expected_cfg
            .render_to_file(Path::new(&format!("{}_expected.dot", test_name)))
            .unwrap();
    }

    #[test]
    fn test_inline_function_a_in_b() {
        // Given: Two functions a and b; a calls b
        let program = {
            let cfg_a = {
                let mut cfg = ControlFlowGraph::new();

                let mut block = Block::new(0);
                block
                    .assign(Boolean::variable("a"), Boolean::constant(false))
                    .unwrap();
                block.call(BitVector::constant_u64(10, 64)).unwrap();
                block
                    .assign(Boolean::variable("c"), Boolean::constant(false))
                    .unwrap();
                cfg.add_block(block).unwrap();

                cfg.set_entry(0).unwrap();
                cfg.set_exit(0).unwrap();

                cfg
            };

            let cfg_b = {
                let mut cfg = ControlFlowGraph::new();

                let mut block = Block::new(0);
                block
                    .assign(Boolean::variable("b"), Boolean::constant(true))
                    .unwrap();
                cfg.add_block(block).unwrap();

                cfg.set_entry(0).unwrap();
                cfg.set_exit(0).unwrap();

                cfg
            };

            let mut program = Program::new();
            program
                .insert_function(Function::new(0, Some("a".to_owned()), cfg_a))
                .unwrap();
            program
                .insert_function(Function::new(10, Some("b".to_owned()), cfg_b))
                .unwrap();

            program
                .set_entry(ProgramEntry::Name("a".to_owned()))
                .unwrap();

            program
        };

        // When: Inline
        let inliner = FunctionInliningBuilder::default()
            .recursion_limit(2)
            .build()
            .unwrap();

        let inlined_program = inliner.inline(&program).unwrap();

        // Then:
        let expected_program = {
            let mut cfg = ControlFlowGraph::new();

            let mut block0 = Block::new(0);
            block0
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            block0.call(BitVector::constant_u64(10, 64)).unwrap();
            cfg.add_block(block0).unwrap();

            let mut block1 = Block::new(1);
            block1
                .assign(Boolean::variable("c"), Boolean::constant(false))
                .unwrap();
            cfg.add_block(block1).unwrap();

            let mut block2 = Block::new(2);
            block2
                .assign(Boolean::variable("b"), Boolean::constant(true))
                .unwrap();
            cfg.add_block(block2).unwrap();

            cfg.unconditional_edge(0, 2).unwrap().labels_mut().call();
            cfg.unconditional_edge(2, 1)
                .unwrap()
                .labels_mut()
                .r#return();

            cfg.set_entry(0).unwrap();
            cfg.set_exit(1).unwrap();

            InlinedProgram::new(cfg)
        };

        /*debug_cfg(
            "test_inline_function_a_in_b",
            inlined_program.control_flow_graph(),
            expected_program.control_flow_graph(),
        );*/

        assert_eq!(expected_program, inlined_program);
    }

    #[test]
    fn test_inline_function_a_in_b_twice() {
        // Given: Two functions a and b; a calls b twice
        let program = {
            let cfg_a = {
                let mut cfg = ControlFlowGraph::new();

                let mut block = Block::new(0);
                block
                    .assign(Boolean::variable("a"), Boolean::constant(false))
                    .unwrap();
                block.call(BitVector::constant_u64(10, 64)).unwrap();
                block
                    .assign(Boolean::variable("c"), Boolean::constant(false))
                    .unwrap();
                block.call(BitVector::constant_u64(10, 64)).unwrap();
                block
                    .assign(Boolean::variable("d"), Boolean::constant(false))
                    .unwrap();
                cfg.add_block(block).unwrap();

                cfg.set_entry(0).unwrap();
                cfg.set_exit(0).unwrap();

                cfg
            };

            let cfg_b = {
                let mut cfg = ControlFlowGraph::new();

                let mut block = Block::new(0);
                block
                    .assign(Boolean::variable("b"), Boolean::constant(true))
                    .unwrap();
                cfg.add_block(block).unwrap();

                cfg.set_entry(0).unwrap();
                cfg.set_exit(0).unwrap();

                cfg
            };

            let mut program = Program::new();
            program
                .insert_function(Function::new(0, Some("a".to_owned()), cfg_a))
                .unwrap();
            program
                .insert_function(Function::new(10, Some("b".to_owned()), cfg_b))
                .unwrap();

            program
                .set_entry(ProgramEntry::Name("a".to_owned()))
                .unwrap();

            program
        };

        // When: Inline
        let inliner = FunctionInliningBuilder::default()
            .recursion_limit(2)
            .build()
            .unwrap();

        let inlined_program = inliner.inline(&program).unwrap();

        // Then:
        let expected_program = {
            let mut cfg = ControlFlowGraph::new();

            let mut block0 = Block::new(0);
            block0
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            block0.call(BitVector::constant_u64(10, 64)).unwrap();
            cfg.add_block(block0).unwrap();

            let mut block1 = Block::new(1);
            block1
                .assign(Boolean::variable("c"), Boolean::constant(false))
                .unwrap();
            block1.call(BitVector::constant_u64(10, 64)).unwrap();
            cfg.add_block(block1).unwrap();

            let mut block2 = Block::new(2);
            block2
                .assign(Boolean::variable("b"), Boolean::constant(true))
                .unwrap();
            cfg.add_block(block2).unwrap();

            let mut block3 = Block::new(3);
            block3
                .assign(Boolean::variable("d"), Boolean::constant(false))
                .unwrap();
            cfg.add_block(block3).unwrap();

            let mut block4 = Block::new(4);
            block4
                .assign(Boolean::variable("b"), Boolean::constant(true))
                .unwrap();
            cfg.add_block(block4).unwrap();

            cfg.unconditional_edge(0, 2).unwrap().labels_mut().call();
            cfg.unconditional_edge(2, 1)
                .unwrap()
                .labels_mut()
                .r#return();

            cfg.unconditional_edge(1, 4).unwrap().labels_mut().call();
            cfg.unconditional_edge(4, 3)
                .unwrap()
                .labels_mut()
                .r#return();

            cfg.set_entry(0).unwrap();
            cfg.set_exit(3).unwrap();

            InlinedProgram::new(cfg)
        };

        /*debug_cfg(
            "test_inline_function_a_in_b_twice",
            inlined_program.control_flow_graph(),
            expected_program.control_flow_graph(),
        );*/

        assert_eq!(expected_program, inlined_program);
    }

    #[test]
    fn test_inline_function_a_in_b_and_c_in_b() {
        // Given: Three functions a, b and c; a calls b and b calls c
        let program = {
            let cfg_a = {
                let mut cfg = ControlFlowGraph::new();

                let mut block = Block::new(0);
                block
                    .assign(Boolean::variable("a"), Boolean::constant(false))
                    .unwrap();
                block.call(BitVector::constant_u64(10, 64)).unwrap();
                block
                    .assign(Boolean::variable("a"), Boolean::constant(false))
                    .unwrap();
                cfg.add_block(block).unwrap();

                cfg.set_entry(0).unwrap();
                cfg.set_exit(0).unwrap();

                cfg
            };

            let cfg_b = {
                let mut cfg = ControlFlowGraph::new();

                let mut block = Block::new(0);
                block
                    .assign(Boolean::variable("b"), Boolean::constant(true))
                    .unwrap();
                block.call(BitVector::constant_u64(20, 64)).unwrap();
                cfg.add_block(block).unwrap();

                cfg.set_entry(0).unwrap();
                cfg.set_exit(0).unwrap();

                cfg
            };

            let cfg_c = {
                let mut cfg = ControlFlowGraph::new();

                let mut block = Block::new(0);
                block
                    .assign(Boolean::variable("c"), Boolean::constant(true))
                    .unwrap();
                cfg.add_block(block).unwrap();

                cfg.set_entry(0).unwrap();
                cfg.set_exit(0).unwrap();

                cfg
            };

            let mut program = Program::new();
            program
                .insert_function(Function::new(0, Some("a".to_owned()), cfg_a))
                .unwrap();
            program
                .insert_function(Function::new(10, Some("b".to_owned()), cfg_b))
                .unwrap();
            program
                .insert_function(Function::new(20, Some("c".to_owned()), cfg_c))
                .unwrap();

            program
                .set_entry(ProgramEntry::Name("a".to_owned()))
                .unwrap();

            program
        };

        // When: Inline
        let inliner = FunctionInliningBuilder::default()
            .recursion_limit(2)
            .build()
            .unwrap();

        let inlined_program = inliner.inline(&program).unwrap();

        // Then:
        let expected_program = {
            let mut cfg = ControlFlowGraph::new();

            let mut block0 = Block::new(0);
            block0
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            block0.call(BitVector::constant_u64(10, 64)).unwrap();
            cfg.add_block(block0).unwrap();

            let mut block1 = Block::new(1);
            block1
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            cfg.add_block(block1).unwrap();

            let mut block2 = Block::new(2);
            block2
                .assign(Boolean::variable("b"), Boolean::constant(true))
                .unwrap();
            block2.call(BitVector::constant_u64(20, 64)).unwrap();
            cfg.add_block(block2).unwrap();

            let block3 = Block::new(3);
            cfg.add_block(block3).unwrap();

            let mut block4 = Block::new(4);
            block4
                .assign(Boolean::variable("c"), Boolean::constant(true))
                .unwrap();
            cfg.add_block(block4).unwrap();

            cfg.unconditional_edge(0, 2).unwrap().labels_mut().call();
            cfg.unconditional_edge(2, 4).unwrap().labels_mut().call();
            cfg.unconditional_edge(4, 3)
                .unwrap()
                .labels_mut()
                .r#return();
            cfg.unconditional_edge(3, 1)
                .unwrap()
                .labels_mut()
                .r#return();

            cfg.set_entry(0).unwrap();
            cfg.set_exit(1).unwrap();

            InlinedProgram::new(cfg)
        };

        /*debug_cfg(
            "test_inline_function_a_in_b_and_c_in_b",
            inlined_program.control_flow_graph(),
            expected_program.control_flow_graph(),
        );*/

        assert_eq!(expected_program, inlined_program);
    }

    #[test]
    fn test_inline_function_a_in_a_with_recursion_limit_one() {
        // Given: One function a; a calls a
        let program = {
            let cfg_a = {
                let mut cfg = ControlFlowGraph::new();

                let mut block = Block::new(0);
                block
                    .assign(Boolean::variable("a"), Boolean::constant(false))
                    .unwrap();
                block.call(BitVector::constant_u64(1, 64)).unwrap();
                block
                    .assign(Boolean::variable("b"), Boolean::constant(false))
                    .unwrap();
                cfg.add_block(block).unwrap();

                cfg.set_entry(0).unwrap();
                cfg.set_exit(0).unwrap();

                cfg
            };

            let mut program = Program::new();
            program
                .insert_function(Function::new(1, Some("a".to_owned()), cfg_a))
                .unwrap();

            program
                .set_entry(ProgramEntry::Name("a".to_owned()))
                .unwrap();

            program
        };

        // When: Inline
        let inliner = FunctionInliningBuilder::default()
            .recursion_limit(1)
            .build()
            .unwrap();

        let inlined_program = inliner.inline(&program).unwrap();

        // Then:
        let expected_program = {
            let mut cfg = ControlFlowGraph::new();

            let mut block0 = Block::new(0);
            block0
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            block0.call(BitVector::constant_u64(1, 64)).unwrap();
            cfg.add_block(block0).unwrap();

            let mut block1 = Block::new(1);
            block1
                .assign(Boolean::variable("b"), Boolean::constant(false))
                .unwrap();
            cfg.add_block(block1).unwrap();

            let mut block2 = Block::new(2);
            block2
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            block2.call(BitVector::constant_u64(1, 64)).unwrap();
            block2
                .assign(Boolean::variable("b"), Boolean::constant(false))
                .unwrap();
            cfg.add_block(block2).unwrap();

            cfg.unconditional_edge(0, 2).unwrap().labels_mut().call();
            cfg.unconditional_edge(2, 1)
                .unwrap()
                .labels_mut()
                .r#return();

            cfg.set_entry(0).unwrap();
            cfg.set_exit(1).unwrap();

            InlinedProgram::new(cfg)
        };

        /*debug_cfg(
            "test_inline_function_a_in_a_with_recursion_limit_one",
            inlined_program.control_flow_graph(),
            expected_program.control_flow_graph(),
        );*/

        assert_eq!(expected_program, inlined_program);
    }

    #[test]
    fn test_inline_function_a_in_a_with_recursion_limit_two() {
        // Given: One function a; a calls a
        let program = {
            let cfg_a = {
                let mut cfg = ControlFlowGraph::new();

                let mut block = Block::new(0);
                block
                    .assign(Boolean::variable("a"), Boolean::constant(false))
                    .unwrap();
                block.call(BitVector::constant_u64(1, 64)).unwrap();
                block
                    .assign(Boolean::variable("b"), Boolean::constant(false))
                    .unwrap();
                cfg.add_block(block).unwrap();

                cfg.set_entry(0).unwrap();
                cfg.set_exit(0).unwrap();

                cfg
            };

            let mut program = Program::new();
            program
                .insert_function(Function::new(1, Some("a".to_owned()), cfg_a))
                .unwrap();

            program
                .set_entry(ProgramEntry::Name("a".to_owned()))
                .unwrap();

            program
        };

        // When: Inline
        let inliner = FunctionInliningBuilder::default()
            .recursion_limit(2)
            .build()
            .unwrap();

        let inlined_program = inliner.inline(&program).unwrap();

        // Then:
        let expected_program = {
            let mut cfg = ControlFlowGraph::new();

            let mut block0 = Block::new(0);
            block0
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            block0.call(BitVector::constant_u64(1, 64)).unwrap();
            cfg.add_block(block0).unwrap();

            let mut block1 = Block::new(1);
            block1
                .assign(Boolean::variable("b"), Boolean::constant(false))
                .unwrap();
            cfg.add_block(block1).unwrap();

            let mut block2 = Block::new(2);
            block2
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            block2.call(BitVector::constant_u64(1, 64)).unwrap();
            cfg.add_block(block2).unwrap();

            let mut block3 = Block::new(3);
            block3
                .assign(Boolean::variable("b"), Boolean::constant(false))
                .unwrap();
            cfg.add_block(block3).unwrap();

            let mut block4 = Block::new(4);
            block4
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            block4.call(BitVector::constant_u64(1, 64)).unwrap();
            block4
                .assign(Boolean::variable("b"), Boolean::constant(false))
                .unwrap();
            cfg.add_block(block4).unwrap();

            cfg.unconditional_edge(0, 2).unwrap().labels_mut().call();
            cfg.unconditional_edge(2, 4).unwrap().labels_mut().call();
            cfg.unconditional_edge(4, 3)
                .unwrap()
                .labels_mut()
                .r#return();
            cfg.unconditional_edge(3, 1)
                .unwrap()
                .labels_mut()
                .r#return();

            cfg.set_entry(0).unwrap();
            cfg.set_exit(1).unwrap();

            InlinedProgram::new(cfg)
        };

        /*debug_cfg(
            "test_inline_function_a_in_a_with_recursion_limit_two",
            inlined_program.control_flow_graph(),
            expected_program.control_flow_graph(),
        );*/

        assert_eq!(expected_program, inlined_program);
    }

    #[test]
    fn test_inline_function_a_in_a_twice_with_recursion_limit_one() {
        // Given: One functions a and a; a calls a twice
        let program = {
            let cfg_a = {
                let mut cfg = ControlFlowGraph::new();

                let mut block = Block::new(0);
                block
                    .assign(Boolean::variable("a"), Boolean::constant(false))
                    .unwrap();
                block.call(BitVector::constant_u64(1, 64)).unwrap();
                block
                    .assign(Boolean::variable("c"), Boolean::constant(false))
                    .unwrap();
                block.call(BitVector::constant_u64(1, 64)).unwrap();
                block
                    .assign(Boolean::variable("d"), Boolean::constant(false))
                    .unwrap();
                cfg.add_block(block).unwrap();

                cfg.set_entry(0).unwrap();
                cfg.set_exit(0).unwrap();

                cfg
            };

            let mut program = Program::new();
            program
                .insert_function(Function::new(1, Some("a".to_owned()), cfg_a))
                .unwrap();

            program
                .set_entry(ProgramEntry::Name("a".to_owned()))
                .unwrap();

            program
        };

        // When: Inline
        let inliner = FunctionInliningBuilder::default()
            .recursion_limit(1)
            .build()
            .unwrap();

        let inlined_program = inliner.inline(&program).unwrap();

        // Then:
        let expected_program = {
            let mut cfg = ControlFlowGraph::new();

            let mut block0 = Block::new(0);
            block0
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            block0.call(BitVector::constant_u64(1, 64)).unwrap();
            cfg.add_block(block0).unwrap();

            let mut block1 = Block::new(1);
            block1
                .assign(Boolean::variable("c"), Boolean::constant(false))
                .unwrap();
            block1.call(BitVector::constant_u64(1, 64)).unwrap();
            cfg.add_block(block1).unwrap();

            let mut block2 = Block::new(2);
            block2
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            block2.call(BitVector::constant_u64(1, 64)).unwrap();
            block2
                .assign(Boolean::variable("c"), Boolean::constant(false))
                .unwrap();
            block2.call(BitVector::constant_u64(1, 64)).unwrap();
            block2
                .assign(Boolean::variable("d"), Boolean::constant(false))
                .unwrap();
            cfg.add_block(block2).unwrap();

            let mut block3 = Block::new(3);
            block3
                .assign(Boolean::variable("d"), Boolean::constant(false))
                .unwrap();
            cfg.add_block(block3).unwrap();

            let mut block4 = Block::new(4);
            block4
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            block4.call(BitVector::constant_u64(1, 64)).unwrap();
            block4
                .assign(Boolean::variable("c"), Boolean::constant(false))
                .unwrap();
            block4.call(BitVector::constant_u64(1, 64)).unwrap();
            block4
                .assign(Boolean::variable("d"), Boolean::constant(false))
                .unwrap();
            cfg.add_block(block4).unwrap();

            cfg.unconditional_edge(0, 2).unwrap().labels_mut().call();
            cfg.unconditional_edge(2, 1)
                .unwrap()
                .labels_mut()
                .r#return();

            cfg.unconditional_edge(1, 4).unwrap().labels_mut().call();
            cfg.unconditional_edge(4, 3)
                .unwrap()
                .labels_mut()
                .r#return();

            cfg.set_entry(0).unwrap();
            cfg.set_exit(3).unwrap();

            InlinedProgram::new(cfg)
        };

        /*debug_cfg(
            "test_inline_function_a_in_a_twice_with_recursion_limit_one",
            inlined_program.control_flow_graph(),
            expected_program.control_flow_graph(),
        );*/

        assert_eq!(expected_program, inlined_program);
    }

    #[test]
    fn test_inline_function_a_in_b_with_recursion_limit_two() {
        // Given: Two function a and b; a calls b and b calls a
        let program = {
            let cfg_a = {
                let mut cfg = ControlFlowGraph::new();

                let mut block = Block::new(0);
                block
                    .assign(Boolean::variable("a"), Boolean::constant(false))
                    .unwrap();
                block.call(BitVector::constant_u64(2, 64)).unwrap();
                block
                    .assign(Boolean::variable("a"), Boolean::constant(false))
                    .unwrap();
                cfg.add_block(block).unwrap();

                cfg.set_entry(0).unwrap();
                cfg.set_exit(0).unwrap();

                cfg
            };

            let cfg_b = {
                let mut cfg = ControlFlowGraph::new();

                let mut block = Block::new(0);
                block
                    .assign(Boolean::variable("b"), Boolean::constant(false))
                    .unwrap();
                block.call(BitVector::constant_u64(1, 64)).unwrap();
                block
                    .assign(Boolean::variable("b"), Boolean::constant(false))
                    .unwrap();
                cfg.add_block(block).unwrap();

                cfg.set_entry(0).unwrap();
                cfg.set_exit(0).unwrap();

                cfg
            };

            let mut program = Program::new();
            program
                .insert_function(Function::new(1, Some("a".to_owned()), cfg_a))
                .unwrap();
            program
                .insert_function(Function::new(2, Some("b".to_owned()), cfg_b))
                .unwrap();

            program
                .set_entry(ProgramEntry::Name("a".to_owned()))
                .unwrap();

            program
        };

        // When: Inline
        let inliner = FunctionInliningBuilder::default()
            .recursion_limit(2)
            .build()
            .unwrap();

        let inlined_program = inliner.inline(&program).unwrap();

        // Then:
        let expected_program = {
            let mut cfg = ControlFlowGraph::new();

            let mut block0 = Block::new(0);
            block0
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            block0.call(BitVector::constant_u64(2, 64)).unwrap();
            cfg.add_block(block0).unwrap();

            let mut block1 = Block::new(1);
            block1
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            cfg.add_block(block1).unwrap();

            let mut block2 = Block::new(2);
            block2
                .assign(Boolean::variable("b"), Boolean::constant(false))
                .unwrap();
            block2.call(BitVector::constant_u64(1, 64)).unwrap();
            cfg.add_block(block2).unwrap();

            let mut block3 = Block::new(3);
            block3
                .assign(Boolean::variable("b"), Boolean::constant(false))
                .unwrap();
            cfg.add_block(block3).unwrap();

            let mut block4 = Block::new(4);
            block4
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            block4.call(BitVector::constant_u64(2, 64)).unwrap();
            cfg.add_block(block4).unwrap();

            let mut block5 = Block::new(5);
            block5
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            cfg.add_block(block5).unwrap();

            let mut block6 = Block::new(6);
            block6
                .assign(Boolean::variable("b"), Boolean::constant(false))
                .unwrap();
            block6.call(BitVector::constant_u64(1, 64)).unwrap();
            cfg.add_block(block6).unwrap();

            let mut block7 = Block::new(7);
            block7
                .assign(Boolean::variable("b"), Boolean::constant(false))
                .unwrap();
            cfg.add_block(block7).unwrap();

            let mut block8 = Block::new(8);
            block8
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            block8.call(BitVector::constant_u64(2, 64)).unwrap();
            block8
                .assign(Boolean::variable("a"), Boolean::constant(false))
                .unwrap();
            cfg.add_block(block8).unwrap();

            cfg.unconditional_edge(0, 2).unwrap().labels_mut().call();
            cfg.unconditional_edge(2, 4).unwrap().labels_mut().call();
            cfg.unconditional_edge(4, 6).unwrap().labels_mut().call();
            cfg.unconditional_edge(6, 8).unwrap().labels_mut().call();
            cfg.unconditional_edge(8, 7)
                .unwrap()
                .labels_mut()
                .r#return();
            cfg.unconditional_edge(7, 5)
                .unwrap()
                .labels_mut()
                .r#return();
            cfg.unconditional_edge(5, 3)
                .unwrap()
                .labels_mut()
                .r#return();
            cfg.unconditional_edge(3, 1)
                .unwrap()
                .labels_mut()
                .r#return();

            cfg.set_entry(0).unwrap();
            cfg.set_exit(1).unwrap();

            InlinedProgram::new(cfg)
        };

        /*debug_cfg(
            "test_inline_function_a_in_b_with_recursion_limit_two",
            inlined_program.control_flow_graph(),
            expected_program.control_flow_graph(),
        );*/

        assert_eq!(expected_program, inlined_program);
    }
}
