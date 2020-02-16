use crate::error::Result;
use crate::expr::{BitVector, Boolean, Predictor};
use crate::hir::{ControlFlowGraph, Instruction, Operation, Program};
use std::collections::BTreeMap;

pub fn add_transient_execution(src_program: &Program) -> Result<Program> {
    let (mut cfg, transient_start_rollback_points) =
        build_default_cfg(src_program.control_flow_graph())?;

    let (transient_cfg, transient_entry_points) =
        build_transient_cfg(src_program.control_flow_graph())?;

    let block_map = cfg.insert(&transient_cfg)?;

    // Wire the default and transient CFG together.
    for (inst_addr, (start, rollback)) in transient_start_rollback_points {
        let transient_entry = block_map[transient_entry_points.get(&inst_addr).unwrap()];
        let transient_resolve = block_map[&transient_cfg.exit().unwrap()];
        cfg.unconditional_edge(start, transient_entry).unwrap();

        // The rollback edge is conditional, because transient_resolve contains multiple outgoing rollback edges.
        // Therefore, rollback for the current instruction should only be done if the transient execution was
        // started by the current instruction (-> mis-predict(inst_addr) == true).
        let mis_predicted = Predictor::mis_predict(
            Predictor::variable().into(),
            BitVector::constant(inst_addr, 64), // FIXME bit-width
        )?;
        cfg.conditional_edge(transient_resolve, rollback, mis_predicted)
            .unwrap();
    }

    cfg.remove_unreachable_blocks()?;

    Ok(Program::new(cfg))
}

fn build_default_cfg(
    cfg: &ControlFlowGraph,
) -> Result<(ControlFlowGraph, BTreeMap<u64, (usize, usize)>)> {
    let mut default_cfg = cfg.clone();

    // For each instruction which can start a transient execution,
    // we keep track of the start and rollback blocks.
    // Start and rollback will later be connected to the transient CFG.
    let mut transient_start_rollback_points = BTreeMap::new();

    for block in cfg.blocks() {
        for (inst_index, inst) in block.instructions().iter().enumerate().rev() {
            match inst.operation() {
                Operation::Store { .. } => {
                    default_store(
                        &mut default_cfg,
                        &mut transient_start_rollback_points,
                        block.index(),
                        inst_index,
                        inst,
                    )?;
                }
                _ => continue,
            }
        }
    }

    Ok((default_cfg, transient_start_rollback_points))
}

/// The `Store` instruction can speculatively be by-passed.
/// Therefore, split the given block into 2 blocks [head] and [tail],
/// add an additional [transient] block and add the following three edges between them:
///   - Conditional edge with "mis-predicted" from head to transient -> store by-pass
///   - Conditional edge with "correctly predicted" from head to tail -> store execute
///   - Unconditional edge from transient to tail -> rollback + store execute
fn default_store(
    cfg: &mut ControlFlowGraph,
    transient_start_rollback_points: &mut BTreeMap<u64, (usize, usize)>,
    head_index: usize,
    inst_index: usize,
    inst: &Instruction,
) -> Result<()> {
    let tail_index = cfg.split_block_at(head_index, inst_index)?;

    let transient_start_index = cfg.new_block()?.index();

    let mis_predicted = Predictor::mis_predict(
        Predictor::variable().into(),
        BitVector::constant(inst.address().unwrap_or_default(), 64), // FIXME bit-width
    )?;

    let correctly_predicted = Boolean::not(mis_predicted.clone())?;

    cfg.conditional_edge(head_index, tail_index, correctly_predicted)?;
    cfg.conditional_edge(head_index, transient_start_index, mis_predicted)?;

    // Tail is the rollback point, meaning that on rollback the store will be executed.
    transient_start_rollback_points
        .insert(inst.address().unwrap(), (transient_start_index, tail_index));

    Ok(())
}

fn build_transient_cfg(cfg: &ControlFlowGraph) -> Result<(ControlFlowGraph, BTreeMap<u64, usize>)> {
    let mut transient_cfg = cfg.clone();

    // For each instruction which can start a transient execution,
    // we keep track of the entry point for transient execution.
    // Later the start block of a transient execution (default CFG)
    // will be connected to this transient entry point.
    let mut transient_entry_points = BTreeMap::new();

    // Add resolve block as exit
    let resolve_block_index = transient_cfg.new_block()?.index();
    transient_cfg.unconditional_edge(cfg.exit().unwrap(), resolve_block_index)?; // end of program -> resolve
    transient_cfg.set_exit(resolve_block_index)?;

    for block in cfg.blocks() {
        for (inst_index, inst) in block.instructions().iter().enumerate().rev() {
            match inst.operation() {
                Operation::Store { .. } => {
                    transient_store(
                        &mut transient_cfg,
                        &mut transient_entry_points,
                        block.index(),
                        inst_index,
                        inst,
                    )?;
                }
                Operation::Barrier => {
                    transient_barrier(&mut transient_cfg, block.index(), inst_index)?;
                }
                _ => continue,
            }
        }
    }

    Ok((transient_cfg, transient_entry_points))
}

/// The `Store` instruction can speculatively be by-passed during transient execution.
/// Therefore, split the given block into 3 blocks [head], [store] and [tail]
/// and add the following three edges between them:
///   - Conditional edge with "mis-predicted" from head to tail -> store by-pass
///   - Conditional edge with "correctly predicted" from head to store -> store execute
///   - Unconditional edge from store to tail
fn transient_store(
    cfg: &mut ControlFlowGraph,
    transient_entry_points: &mut BTreeMap<u64, usize>,
    head_index: usize,
    inst_index: usize,
    inst: &Instruction,
) -> Result<()> {
    let store_index = cfg.split_block_at(head_index, inst_index)?;
    let tail_index = cfg.split_block_at(store_index, 1)?;

    let mis_predicted = Predictor::mis_predict(
        Predictor::variable().into(),
        BitVector::constant(inst.address().unwrap_or_default(), 64), // FIXME bit-width
    )?;

    let correctly_predicted = Boolean::not(mis_predicted.clone())?;

    cfg.conditional_edge(head_index, tail_index, mis_predicted)?;
    cfg.conditional_edge(head_index, store_index, correctly_predicted)?;
    cfg.unconditional_edge(store_index, tail_index)?;

    // Transient execution will begin in tail (same as on mis-predict during transient execution).
    transient_entry_points.insert(inst.address().unwrap(), tail_index);

    Ok(())
}

/// The `Barrier` instruction immediately stops the transient execution.
/// Therefore, replace the `Barrier` with an unconditional edge to the resolve block.
fn transient_barrier(
    cfg: &mut ControlFlowGraph,
    block_index: usize,
    inst_index: usize,
) -> Result<()> {
    let tail_index = cfg.split_block_at(block_index, inst_index)?;

    // drop barrier instruction from tail
    cfg.block_mut(tail_index)?.remove_instruction(0)?;

    let resolve_index = cfg.exit().unwrap();
    cfg.unconditional_edge(block_index, resolve_index)?;

    Ok(())
}
