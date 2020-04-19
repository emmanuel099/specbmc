use crate::environment::{
    Environment, PredictorStrategy, TransientEncodingStrategy, SPECULATION_WINDOW_SIZE, WORD_SIZE,
};
use crate::error::Result;
use crate::expr::{BitVector, Boolean, Expression, Predictor, Sort, Variable};
use crate::hir::{ControlFlowGraph, Instruction, Operation, Program};
use crate::util::Transform;
use std::collections::BTreeMap;
use std::convert::TryInto;

pub struct TransientExecution {
    spectre_pht: bool,
    spectre_stl: bool,
    predictor_strategy: PredictorStrategy,
    transient_encoding_strategy: TransientEncodingStrategy,
}

impl TransientExecution {
    pub fn new() -> Self {
        Self {
            spectre_pht: false,
            spectre_stl: false,
            predictor_strategy: PredictorStrategy::default(),
            transient_encoding_strategy: TransientEncodingStrategy::default(),
        }
    }

    pub fn new_from_env(env: &Environment) -> Self {
        Self {
            spectre_pht: env.analysis.spectre_pht,
            spectre_stl: env.analysis.spectre_stl,
            predictor_strategy: env.analysis.predictor_strategy,
            transient_encoding_strategy: env.analysis.transient_encoding_strategy,
        }
    }

    /// Enable or disable Spectre-PHT encoding.
    ///
    /// If enabled, speculative branch mis-prediction will be encoded.
    pub fn with_spectre_pht(&mut self, enabled: bool) -> &mut Self {
        self.spectre_pht = enabled;
        self
    }

    /// Enable or disable Spectre-STL encoding.
    ///
    /// If enabled, speculative store-bypass will be encoded.
    pub fn with_spectre_stl(&mut self, enabled: bool) -> &mut Self {
        self.spectre_stl = enabled;
        self
    }

    /// Set the predictor strategy.
    pub fn with_predictor_strategy(&mut self, strategy: PredictorStrategy) -> &mut Self {
        self.predictor_strategy = strategy;
        self
    }

    /// Set the transient encoding strategy.
    pub fn with_transient_encoding_strategy(
        &mut self,
        strategy: TransientEncodingStrategy,
    ) -> &mut Self {
        self.transient_encoding_strategy = strategy;
        self
    }

    fn build_default_cfg(
        &self,
        cfg: &ControlFlowGraph,
    ) -> Result<(ControlFlowGraph, BTreeMap<u64, (usize, usize)>)> {
        let mut default_cfg = cfg.clone();

        // For each instruction which can start a transient execution,
        // we keep track of the start and rollback blocks.
        // Start and rollback will later be connected to the transient CFG.
        let mut transient_start_rollback_points = BTreeMap::new();

        for block in cfg.blocks() {
            for (inst_index, inst) in block.instructions().iter().enumerate().rev() {
                for operation in inst.operations() {
                    match operation {
                        Operation::Store { .. } => {
                            if self.spectre_stl {
                                // The `Store` instruction can speculatively be by-passed.
                                add_transient_execution_start(
                                    &mut default_cfg,
                                    &mut transient_start_rollback_points,
                                    block.index(),
                                    inst_index,
                                    inst,
                                )?;
                            }
                        }
                        Operation::ConditionalBranch { .. } => {
                            if self.spectre_pht {
                                // The `ConditionalBranch` instruction can be mis-predicted.
                                add_transient_execution_start(
                                    &mut default_cfg,
                                    &mut transient_start_rollback_points,
                                    block.index(),
                                    inst_index,
                                    inst,
                                )?;
                            }
                        }
                        _ => (),
                    }
                }
            }
        }

        Ok((default_cfg, transient_start_rollback_points))
    }

    fn build_transient_cfg(
        &self,
        cfg: &ControlFlowGraph,
    ) -> Result<(ControlFlowGraph, BTreeMap<u64, usize>)> {
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
                for operation in inst.operations() {
                    match operation {
                        Operation::Store { .. } => {
                            if self.spectre_stl {
                                transient_store(
                                    &mut transient_cfg,
                                    &mut transient_entry_points,
                                    block.index(),
                                    inst_index,
                                    inst,
                                )?;
                            }
                        }
                        Operation::ConditionalBranch { .. } => {
                            if self.spectre_pht {
                                transient_conditional_branch(
                                    &mut transient_cfg,
                                    &mut transient_entry_points,
                                    block.index(),
                                    inst_index,
                                    inst,
                                    self.predictor_strategy,
                                )?;
                            }
                        }
                        Operation::Barrier => {
                            transient_barrier(&mut transient_cfg, block.index(), inst_index)?;
                        }
                        _ => (),
                    }
                }
            }
        }

        add_transient_resolve_edges(&mut transient_cfg)?;

        // Mark all blocks as transient
        for block in transient_cfg.blocks_mut() {
            block.set_transient(true);
        }

        Ok((transient_cfg, transient_entry_points))
    }

    /// All transient behavior is encoded into a single transient graph
    fn encode_unified(&self, program: &mut Program) -> Result<ControlFlowGraph> {
        let (mut cfg, transient_start_rollback_points) =
            self.build_default_cfg(program.control_flow_graph())?;

        let (transient_cfg, transient_entry_points) =
            self.build_transient_cfg(program.control_flow_graph())?;

        // Add single copy
        let block_map = cfg.insert(&transient_cfg)?;

        // Wire the default and transient CFG together.
        for (inst_addr, (start, rollback)) in transient_start_rollback_points {
            let transient_entry = block_map[transient_entry_points.get(&inst_addr).unwrap()];
            let transient_resolve = block_map[&transient_cfg.exit().unwrap()];
            cfg.unconditional_edge(start, transient_entry).unwrap();

            // The rollback edge is conditional, because transient_resolve contains multiple outgoing rollback edges.
            // Therefore, rollback for the current instruction should only be done if the transient execution was
            // started by the current instruction.
            let transient_exec = Expression::equal(
                Predictor::transient_start(Predictor::variable().into())?,
                BitVector::constant_u64(inst_addr, WORD_SIZE),
            )?;
            cfg.conditional_edge(transient_resolve, rollback, transient_exec)
                .unwrap();
        }

        cfg.simplify()?;

        append_spec_win_decrease_to_all_transient_blocks(&mut cfg)?;

        Ok(cfg)
    }

    /// One transient graph for each (speculating) instruction
    fn encode_several(&self, program: &mut Program) -> Result<ControlFlowGraph> {
        let (mut cfg, transient_start_rollback_points) =
            self.build_default_cfg(program.control_flow_graph())?;

        let (transient_cfg, transient_entry_points) =
            self.build_transient_cfg(program.control_flow_graph())?;

        for (inst_addr, (start, rollback)) in transient_start_rollback_points {
            let block_map = cfg.insert(&transient_cfg)?;
            // TODO reduce the size of the inserted graph (depth limit)

            let transient_entry = block_map[transient_entry_points.get(&inst_addr).unwrap()];
            let transient_resolve = block_map[&transient_cfg.exit().unwrap()];

            cfg.unconditional_edge(start, transient_entry).unwrap();
            cfg.unconditional_edge(transient_resolve, rollback).unwrap();
        }

        cfg.simplify()?;

        append_spec_win_decrease_to_all_transient_blocks(&mut cfg)?;

        Ok(cfg)
    }
}

impl Transform<Program> for TransientExecution {
    fn name(&self) -> &'static str {
        "TransientExecution"
    }

    fn description(&self) -> &'static str {
        "Add transient execution behavior"
    }

    fn transform(&self, program: &mut Program) -> Result<()> {
        let mut cfg = match self.transient_encoding_strategy {
            TransientEncodingStrategy::Unified => self.encode_unified(program)?,
            TransientEncodingStrategy::Several => self.encode_several(program)?,
        };

        program.set_control_flow_graph(cfg);

        Ok(())
    }
}

/// Speculation-Window Variable
fn spec_win() -> Variable {
    Variable::new("_spec_win", Sort::bit_vector(SPECULATION_WINDOW_SIZE))
}

/// For transient execution start/rollback split the given block into 2 blocks [head] and [tail],
/// add an additional [transient] block and add the following three edges between them:
///   - Conditional edge with "mis-predicted" from head to transient -> start transient execution
///   - Conditional edge with "correctly predicted" from head to tail -> normal execution
///   - Unconditional edge from transient to tail -> rollback + re-execution
fn add_transient_execution_start(
    cfg: &mut ControlFlowGraph,
    transient_start_rollback_points: &mut BTreeMap<u64, (usize, usize)>,
    head_index: usize,
    inst_index: usize,
    inst: &Instruction,
) -> Result<()> {
    let tail_index = cfg.split_block_at(head_index, inst_index)?;

    let transient_start_index = {
        let transient_start = cfg.new_block()?;
        transient_start.set_transient(true);

        // initial speculation window size
        let spec_window = Predictor::speculation_window(
            Predictor::variable().into(),
            BitVector::constant_u64(inst.address().unwrap_or_default(), WORD_SIZE),
        )?;
        transient_start.assign(spec_win(), spec_window)?;

        transient_start.index()
    };

    let transient_exec = Expression::equal(
        Predictor::transient_start(Predictor::variable().into())?,
        BitVector::constant_u64(inst.address().unwrap_or_default(), WORD_SIZE),
    )?;

    let normal_exec = Boolean::not(transient_exec.clone())?;

    cfg.conditional_edge(head_index, tail_index, normal_exec)?;
    cfg.conditional_edge(head_index, transient_start_index, transient_exec)?;

    // Tail is the rollback point, meaning that on rollback the instruction will be re-executed.
    transient_start_rollback_points.insert(
        inst.address().unwrap_or_default(),
        (transient_start_index, tail_index),
    );

    Ok(())
}

/// The `Store` instruction can speculatively be by-passed during transient execution.
/// Therefore, split the given block into 3 blocks [head], [store] and [tail]
/// and add the following three edges between them:
///   - Conditional edge with "speculate" from head to tail -> store by-pass
///   - Conditional edge with "not speculate" from head to store -> store execute
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

    let bypass = Predictor::speculate(
        Predictor::variable().into(),
        BitVector::constant_u64(inst.address().unwrap_or_default(), WORD_SIZE),
    )?;
    let execute = Boolean::not(bypass.clone())?;

    cfg.conditional_edge(head_index, tail_index, bypass)?;
    cfg.conditional_edge(head_index, store_index, execute)?;
    cfg.unconditional_edge(store_index, tail_index)?;

    // Transient execution will begin in tail (same as on bypass during transient execution).
    transient_entry_points.insert(inst.address().unwrap(), tail_index);

    Ok(())
}

/// The `ConditionalBranch` instruction can be mis-predicted during transient execution.
/// Therefore, split the given block into 2 blocks [head] and [branch],
/// and additionally add a new block [speculate].
/// Then add the following edges between them:
///   - Conditional edge with "speculate" from head to speculate -> speculative execution
///   - Conditional edge with "correctly predicted" from head to branch -> correct execution
///   - Conditional edges from speculate to each successor of the branch instruction,
///     the conditions of these edges depend on the strategy in use
fn transient_conditional_branch(
    cfg: &mut ControlFlowGraph,
    transient_entry_points: &mut BTreeMap<u64, usize>,
    head_index: usize,
    inst_index: usize,
    inst: &Instruction,
    predictor_strategy: PredictorStrategy,
) -> Result<()> {
    let branch_index = cfg.split_block_at(head_index, inst_index)?;
    let speculate_index = cfg.new_block()?.index();

    let speculate = Predictor::speculate(
        Predictor::variable().into(),
        BitVector::constant_u64(inst.address().unwrap_or_default(), WORD_SIZE),
    )?;
    let execute_correctly = Boolean::not(speculate.clone())?;

    cfg.conditional_edge(head_index, speculate_index, speculate)?;
    cfg.conditional_edge(head_index, branch_index, execute_correctly)?;

    match predictor_strategy {
        PredictorStrategy::ChoosePath => {
            // Add taken/not-taken edges from speculate to successors
            if let &[not_taken_succ, taken_succ] = cfg.successor_indices(branch_index)?.as_slice() {
                // TODO This assumes that the successors are always ordered like this.
                //      Make this code order independent by checking the target instead.

                let taken = Predictor::taken(
                    Predictor::variable().into(),
                    BitVector::constant_u64(inst.address().unwrap_or_default(), WORD_SIZE),
                )?;
                let not_taken = Boolean::not(taken.clone())?;

                cfg.conditional_edge(speculate_index, not_taken_succ, not_taken)?;
                cfg.conditional_edge(speculate_index, taken_succ, taken)?;
            } else {
                return Err("Expected two successors for conditional branch".into());
            }
        }
        PredictorStrategy::InvertCondition => {
            // Add negated conditional edges from speculate to all branch successors
            for successor in cfg.successor_indices(branch_index)? {
                let edge = cfg.edge(branch_index, successor)?;
                let negated_condition = Boolean::not(edge.condition().unwrap().clone())?;
                cfg.conditional_edge(speculate_index, successor, negated_condition)?;
            }
        }
    }

    // Transient execution will begin in speculate.
    transient_entry_points.insert(inst.address().unwrap(), speculate_index);

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

/// Add additional resolve edges to the transient control flow graph.
/// This makes sure that the transient execution can stop/resolve at any point in time.
///
/// Instead of adding resolve edges for each single instruction,
/// we limit them to "effect-ful" instructions only.
fn add_transient_resolve_edges(cfg: &mut ControlFlowGraph) -> Result<()> {
    let resolve_block_index = cfg.exit().unwrap();

    // effect-ful instructions for each block
    let effectful_instructions: Vec<(usize, Vec<usize>)> = cfg
        .blocks()
        .iter()
        .map(|block| {
            (
                block.index(),
                block
                    .instructions()
                    .iter()
                    .enumerate()
                    .filter(|(_, inst)| inst.has_effects())
                    .map(|(inst_index, _)| inst_index)
                    .collect(),
            )
        })
        .collect();

    for (block_index, instruction_indices) in effectful_instructions {
        for inst_index in instruction_indices.iter().rev() {
            let tail_index = cfg.split_block_at(block_index, *inst_index)?;

            let zero = BitVector::constant_u64(0, SPECULATION_WINDOW_SIZE);

            let continue_execution = BitVector::sgt(spec_win().into(), zero.clone())?;
            cfg.conditional_edge(block_index, tail_index, continue_execution)?;

            let resolve = BitVector::sle(spec_win().into(), zero)?;
            cfg.conditional_edge(block_index, resolve_block_index, resolve)?;
        }
    }

    Ok(())
}

/// Appends "_spec_win := _spec_win - |instructions in BB|" to the end of each basic block.
fn append_spec_win_decrease_to_all_transient_blocks(cfg: &mut ControlFlowGraph) -> Result<()> {
    for block in cfg.blocks_mut() {
        if !block.is_transient() {
            continue;
        }

        let count = block.instruction_count_by_address();
        if count == 0 {
            continue; // Avoid adding useless decrease by zero instructions
        }

        block.assign(
            spec_win(),
            BitVector::sub(
                spec_win().into(),
                BitVector::constant_u64(count.try_into().unwrap(), SPECULATION_WINDOW_SIZE),
            )?,
        )?;
    }

    Ok(())
}
