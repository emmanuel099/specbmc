use crate::error::Result;
use crate::expr::{BitVector, Boolean, Predictor};
use crate::hir::{ControlFlowGraph, Instruction, Operation, Program};

pub fn add_transient_execution(src_program: &Program) -> Result<Program> {
    let mut cfg = build_default_cfg(src_program.control_flow_graph())?;

    let transient_cfg = build_transient_cfg(src_program.control_flow_graph())?;
    cfg.insert(&transient_cfg)?;

    Ok(Program::new(cfg))
}

fn build_default_cfg(cfg: &ControlFlowGraph) -> Result<ControlFlowGraph> {
    let mut default_cfg = cfg.clone();

    for block in cfg.blocks() {
        for (inst_index, inst) in block.instructions().iter().enumerate().rev() {
            match inst.operation() {
                Operation::Store { .. } => {
                    default_store(&mut default_cfg, block.index(), inst_index, inst)?;
                }
                _ => continue,
            }
        }
    }

    Ok(default_cfg)
}

/// The `Store` instruction can speculatively be by-passed.
/// Therefore, split the given block into 2 blocks [head] and [tail],
/// add an additional [transient] block and add the following three edges between them:
///   - Conditional edge with "mis-predicted" from head to transient -> store by-pass
///   - Conditional edge with "correctly predicted" from head to tail -> store execute
///   - Unconditional edge from transient to tail -> rollback + store execute
fn default_store(
    cfg: &mut ControlFlowGraph,
    head_index: usize,
    inst_index: usize,
    inst: &Instruction,
) -> Result<()> {
    let tail_index = cfg.split_block_at(head_index, inst_index)?;

    let transient_index = cfg.new_block()?.index();

    let mis_predicted = Predictor::mis_predict(
        Predictor::variable().into(),
        BitVector::constant(inst.address().unwrap_or_default(), 64), // FIXME bit-width
    )?;

    let correctly_predicted = Boolean::not(mis_predicted.clone())?;

    cfg.conditional_edge(head_index, tail_index, correctly_predicted)?;
    cfg.conditional_edge(head_index, transient_index, mis_predicted)?;
    cfg.unconditional_edge(transient_index, tail_index)?; // rollback

    Ok(())
}

fn build_transient_cfg(cfg: &ControlFlowGraph) -> Result<ControlFlowGraph> {
    let mut transient_cfg = cfg.clone();

    // Add resolve block as exit
    let resolve_block_index = transient_cfg.new_block()?.index();
    if let Some(exit) = transient_cfg.exit() {
        transient_cfg.unconditional_edge(exit, resolve_block_index)?;
    }
    transient_cfg.set_exit(resolve_block_index)?;

    for block in cfg.blocks() {
        for (inst_index, inst) in block.instructions().iter().enumerate().rev() {
            match inst.operation() {
                Operation::Store { .. } => {
                    transient_store(&mut transient_cfg, block.index(), inst_index, inst)?;
                }
                Operation::Barrier => {
                    transient_barrier(&mut transient_cfg, block.index(), inst_index)?;
                }
                _ => continue,
            }
        }
    }

    Ok(transient_cfg)
}

/// The `Store` instruction can speculatively be by-passed during transient execution.
/// Therefore, split the given block into 3 blocks [head], [store] and [tail]
/// and add the following three edges between them:
///   - Conditional edge with "mis-predicted" from head to tail -> store by-pass
///   - Conditional edge with "correctly predicted" from head to store -> store execute
///   - Unconditional edge from store to tail
fn transient_store(
    cfg: &mut ControlFlowGraph,
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
