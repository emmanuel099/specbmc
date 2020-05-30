use crate::environment::{
    Environment, PredictorStrategy, TransientEncodingStrategy, SPECULATION_WINDOW_SIZE, WORD_SIZE,
};
use crate::error::Result;
use crate::expr::{BitVector, Boolean, Expression, Predictor, Sort, Variable};
use crate::hir::{ControlFlowGraph, Edge, Operation, Program};
use crate::util::Transform;
use std::collections::BTreeMap;
use std::convert::TryInto;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct InstructionRef {
    block: usize,
    index: usize,
    address: u64,
}

impl InstructionRef {
    pub fn new(block: usize, index: usize, address: u64) -> Self {
        Self {
            block,
            index,
            address,
        }
    }
    pub fn block(&self) -> usize {
        self.block
    }
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn address(&self) -> u64 {
        self.address
    }
}

pub struct TransientExecution {
    spectre_pht: bool,
    spectre_stl: bool,
    predictor_strategy: PredictorStrategy,
    transient_encoding_strategy: TransientEncodingStrategy,
    speculation_window: usize,
}

impl TransientExecution {
    pub fn new() -> Self {
        Self {
            spectre_pht: false,
            spectre_stl: false,
            predictor_strategy: PredictorStrategy::default(),
            transient_encoding_strategy: TransientEncodingStrategy::default(),
            speculation_window: 100,
        }
    }

    pub fn new_from_env(env: &Environment) -> Self {
        Self {
            spectre_pht: env.analysis.spectre_pht,
            spectre_stl: env.analysis.spectre_stl,
            predictor_strategy: env.analysis.predictor_strategy,
            transient_encoding_strategy: env.analysis.transient_encoding_strategy,
            speculation_window: env.architecture.speculation_window,
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
    ) -> Result<(ControlFlowGraph, BTreeMap<InstructionRef, (usize, usize)>)> {
        let mut default_cfg = cfg.clone();

        // For each instruction which can start a transient execution,
        // we keep track of the start and rollback blocks.
        // Start and rollback will later be connected to the transient CFG.
        let mut transient_start_rollback_points = BTreeMap::new();

        for block in cfg.blocks() {
            for (inst_index, inst) in block.instructions().iter().enumerate().rev() {
                let address = inst.address().unwrap_or_default();
                let inst_ref = InstructionRef::new(block.index(), inst_index, address);

                for operation in inst.operations() {
                    match operation {
                        Operation::Store { .. } => {
                            if self.spectre_stl {
                                // The `Store` instruction can speculatively be by-passed.
                                add_transient_execution_start(
                                    &mut default_cfg,
                                    &mut transient_start_rollback_points,
                                    &inst_ref,
                                )?;
                            }
                        }
                        Operation::ConditionalBranch { .. } => {
                            if self.spectre_pht {
                                // The `ConditionalBranch` instruction can be mis-predicted.
                                add_transient_execution_start(
                                    &mut default_cfg,
                                    &mut transient_start_rollback_points,
                                    &inst_ref,
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
    ) -> Result<(ControlFlowGraph, BTreeMap<InstructionRef, usize>)> {
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
                let address = inst.address().unwrap_or_default();
                let inst_ref = InstructionRef::new(block.index(), inst_index, address);

                for operation in inst.operations() {
                    match operation {
                        Operation::Store { .. } => {
                            if self.spectre_stl {
                                transient_store(
                                    &mut transient_cfg,
                                    &mut transient_entry_points,
                                    &inst_ref,
                                )?;
                            }
                        }
                        Operation::ConditionalBranch { .. } => {
                            if self.spectre_pht {
                                transient_conditional_branch(
                                    &mut transient_cfg,
                                    &mut transient_entry_points,
                                    &inst_ref,
                                    self.predictor_strategy,
                                )?;
                            }
                        }
                        Operation::Barrier => {
                            transient_barrier(&mut transient_cfg, &inst_ref)?;
                        }
                        _ => (),
                    }
                }
            }
        }

        add_transient_resolve_edges(&mut transient_cfg)?;

        // Mark all blocks as transient
        for block in transient_cfg.blocks_mut() {
            block.set_transient();
        }

        Ok((transient_cfg, transient_entry_points))
    }

    /// All transient behavior is encoded into a single transient graph
    fn encode_unified(&self, program: &mut Program) -> Result<ControlFlowGraph> {
        let (mut cfg, transient_start_rollback_points) =
            self.build_default_cfg(program.control_flow_graph())?;

        let (mut transient_cfg, transient_entry_points) =
            self.build_transient_cfg(program.control_flow_graph())?;

        // Reduce the size of the transient graph (depth limit by max. speculation window)
        remove_unreachable_transient_edges(
            &mut transient_cfg,
            &transient_entry_points.values().cloned().collect(),
            self.speculation_window,
        )?;

        // Add single copy
        let block_map = cfg.insert(&transient_cfg)?;

        // Wire the default and transient CFG together.
        for (inst_ref, (start, rollback)) in transient_start_rollback_points {
            let transient_entry = block_map[transient_entry_points.get(&inst_ref).unwrap()];
            let transient_resolve = block_map[&transient_cfg.exit().unwrap()];
            cfg.unconditional_edge(start, transient_entry).unwrap();

            // The rollback edge is conditional, because transient_resolve contains multiple outgoing rollback edges.
            // Therefore, rollback for the current instruction should only be done if the transient execution was
            // started by the current instruction.
            let transient_exec = Expression::equal(
                Predictor::transient_start(Predictor::variable().into())?,
                BitVector::constant_u64(inst_ref.address(), WORD_SIZE),
            )?;
            cfg.conditional_edge(transient_resolve, rollback, transient_exec)
                .unwrap()
                .labels_mut()
                .rollback();
        }

        Ok(cfg)
    }

    /// One transient graph for each (speculating) instruction
    fn encode_several(&self, program: &mut Program) -> Result<ControlFlowGraph> {
        let (mut cfg, transient_start_rollback_points) =
            self.build_default_cfg(program.control_flow_graph())?;

        let (transient_cfg, transient_entry_points) =
            self.build_transient_cfg(program.control_flow_graph())?;

        for (inst_ref, (start, rollback)) in transient_start_rollback_points {
            let transient_entry_point = transient_entry_points.get(&inst_ref).cloned().unwrap();

            // Reduce the size of the transient graph (depth limit by max. speculation window)
            let mut reduced_transient_cfg = transient_cfg.clone();
            remove_unreachable_transient_edges(
                &mut reduced_transient_cfg,
                &vec![transient_entry_point],
                self.speculation_window,
            )?;

            let block_map = cfg.insert(&reduced_transient_cfg)?;

            let transient_entry = block_map[&transient_entry_point];
            let transient_resolve = block_map[&reduced_transient_cfg.exit().unwrap()];

            cfg.unconditional_edge(start, transient_entry).unwrap();
            cfg.unconditional_edge(transient_resolve, rollback)
                .unwrap()
                .labels_mut()
                .rollback();
        }

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

        cfg.simplify()?;

        append_spec_win_decrease_to_all_transient_blocks(&mut cfg)?;

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
    transient_start_rollback_points: &mut BTreeMap<InstructionRef, (usize, usize)>,
    inst_ref: &InstructionRef,
) -> Result<()> {
    let head_index = inst_ref.block();
    let tail_index = cfg.split_block_at(head_index, inst_ref.index())?;

    let transient_start_index = {
        let transient_start = cfg.new_block()?;
        transient_start.set_transient();

        // initial speculation window size
        let spec_window = Predictor::speculation_window(
            Predictor::variable().into(),
            BitVector::constant_u64(inst_ref.address(), WORD_SIZE),
        )?;
        transient_start.assign(spec_win(), spec_window)?;

        let zero = BitVector::constant_u64(0, SPECULATION_WINDOW_SIZE);
        transient_start.assume(BitVector::sgt(spec_win().into(), zero.clone())?)?;

        transient_start.index()
    };

    let transient_exec = Expression::equal(
        Predictor::transient_start(Predictor::variable().into())?,
        BitVector::constant_u64(inst_ref.address(), WORD_SIZE),
    )?;

    let normal_exec = Boolean::not(transient_exec.clone())?;

    cfg.conditional_edge(head_index, tail_index, normal_exec)?;
    cfg.conditional_edge(head_index, transient_start_index, transient_exec)?
        .labels_mut()
        .speculate();

    // Tail is the rollback point, meaning that on rollback the instruction will be re-executed.
    transient_start_rollback_points.insert(inst_ref.clone(), (transient_start_index, tail_index));

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
    transient_entry_points: &mut BTreeMap<InstructionRef, usize>,
    inst_ref: &InstructionRef,
) -> Result<()> {
    let head_index = inst_ref.block();
    let store_index = cfg.split_block_at(head_index, inst_ref.index())?;
    let tail_index = cfg.split_block_at(store_index, 1)?;

    let bypass = Predictor::speculate(
        Predictor::variable().into(),
        BitVector::constant_u64(inst_ref.address(), WORD_SIZE),
    )?;
    let execute = Boolean::not(bypass.clone())?;

    cfg.conditional_edge(head_index, tail_index, bypass)?
        .labels_mut()
        .speculate();
    cfg.conditional_edge(head_index, store_index, execute)?;
    cfg.unconditional_edge(store_index, tail_index)?;

    // Transient execution will begin in tail (same as on bypass during transient execution).
    transient_entry_points.insert(inst_ref.clone(), tail_index);

    Ok(())
}

/// The `ConditionalBranch` instruction can be mis-predicted during transient execution.
/// Therefore, split the given block into 2 blocks [head] and [branch],
/// and additionally add a new block [speculate].
/// Then add the following edges between them:
///   - Conditional edge with "speculate" from head to speculate -> speculative execution
///   - Conditional edge with "execute correctly" from head to branch -> correct execution
///   - Conditional edges from speculate to each successor of the branch instruction,
///     the conditions of these edges depend on the strategy in use
fn transient_conditional_branch(
    cfg: &mut ControlFlowGraph,
    transient_entry_points: &mut BTreeMap<InstructionRef, usize>,
    inst_ref: &InstructionRef,
    predictor_strategy: PredictorStrategy,
) -> Result<()> {
    let head_index = inst_ref.block();
    let branch_index = cfg.split_block_at(head_index, inst_ref.index())?;
    let speculate_index = cfg.new_block()?.index();

    let speculate = Predictor::speculate(
        Predictor::variable().into(),
        BitVector::constant_u64(inst_ref.address(), WORD_SIZE),
    )?;
    let execute_correctly = Boolean::not(speculate.clone())?;

    cfg.conditional_edge(head_index, speculate_index, speculate)?
        .labels_mut()
        .speculate();
    cfg.conditional_edge(head_index, branch_index, execute_correctly)?;

    match predictor_strategy {
        PredictorStrategy::ChoosePath => {
            // Add taken/not-taken edges from speculate to successors
            let outgoing_edges: Vec<Edge> =
                cfg.edges_out(branch_index)?.into_iter().cloned().collect();
            match outgoing_edges.as_slice() {
                [edge] => {
                    // There is only one successor which is only possible because of loop unwinding.
                    // In this case we add an assumption that the condition of the edge holds.
                    if edge.labels().is_taken() {
                        let taken = Predictor::taken(
                            Predictor::variable().into(),
                            BitVector::constant_u64(inst_ref.address(), WORD_SIZE),
                        )?;
                        cfg.block_mut(speculate_index)?.assume(taken.clone())?;
                        cfg.conditional_edge(speculate_index, edge.tail(), taken)?
                            .labels_mut()
                            .taken();
                    } else {
                        let not_taken = Boolean::not(Predictor::taken(
                            Predictor::variable().into(),
                            BitVector::constant_u64(inst_ref.address(), WORD_SIZE),
                        )?)?;
                        cfg.block_mut(speculate_index)?.assume(not_taken.clone())?;
                        cfg.conditional_edge(speculate_index, edge.tail(), not_taken)?;
                    }
                }
                [edge1, edge2] => {
                    let (taken_edge, not_taken_edge) = if edge1.labels().is_taken() {
                        assert!(!edge2.labels().is_taken());
                        (edge1, edge2)
                    } else {
                        assert!(edge2.labels().is_taken());
                        (edge2, edge1)
                    };

                    let taken = Predictor::taken(
                        Predictor::variable().into(),
                        BitVector::constant_u64(inst_ref.address(), WORD_SIZE),
                    )?;
                    let not_taken = Boolean::not(taken.clone())?;

                    cfg.conditional_edge(speculate_index, not_taken_edge.tail(), not_taken)?;

                    cfg.conditional_edge(speculate_index, taken_edge.tail(), taken)?
                        .labels_mut()
                        .taken();
                }
                _ => {
                    return Err("Expected one or two successors for conditional branch".into());
                }
            }
        }
        PredictorStrategy::InvertCondition => {
            // Add negated conditional edges from speculate to all branch successors
            let outgoing_edges: Vec<Edge> =
                cfg.edges_out(branch_index)?.into_iter().cloned().collect();
            match outgoing_edges.as_slice() {
                [edge] => {
                    // There is only one successor which is only possible because of loop unwinding.
                    // In this case we add an assumption that the condition of the edge holds.
                    if edge.labels().is_taken() {
                        let taken = Boolean::not(edge.condition().unwrap().clone())?;
                        cfg.block_mut(speculate_index)?.assume(taken.clone())?;
                        cfg.conditional_edge(speculate_index, edge.tail(), taken)?
                            .labels_mut()
                            .taken();
                    } else {
                        let not_taken = Boolean::not(edge.condition().unwrap().clone())?;
                        cfg.block_mut(speculate_index)?.assume(not_taken.clone())?;
                        cfg.conditional_edge(speculate_index, edge.tail(), not_taken)?;
                    }
                }
                [edge1, edge2] => {
                    let (taken_edge, not_taken_edge) = if edge1.labels().is_taken() {
                        assert!(!edge2.labels().is_taken());
                        (edge1, edge2)
                    } else {
                        assert!(edge2.labels().is_taken());
                        (edge2, edge1)
                    };

                    let taken = Boolean::not(taken_edge.condition().unwrap().clone())?;
                    let not_taken = Boolean::not(not_taken_edge.condition().unwrap().clone())?;

                    cfg.conditional_edge(speculate_index, not_taken_edge.tail(), not_taken)?;

                    cfg.conditional_edge(speculate_index, taken_edge.tail(), taken)?
                        .labels_mut()
                        .taken();
                }
                _ => {
                    return Err("Expected one or two successors for conditional branch".into());
                }
            }
        }
    }

    // Transient execution will begin in speculate.
    transient_entry_points.insert(inst_ref.clone(), speculate_index);

    Ok(())
}

/// The `Barrier` instruction immediately stops the transient execution.
/// Therefore, split the block and add an unconditional edge from head to the resolve block.
fn transient_barrier(cfg: &mut ControlFlowGraph, inst_ref: &InstructionRef) -> Result<()> {
    let head_index = inst_ref.block();
    let _tail_index = cfg.split_block_at(head_index, inst_ref.index())?;

    let resolve_index = cfg.exit().unwrap();
    cfg.unconditional_edge(head_index, resolve_index)?;

    Ok(())
}

fn split_blocks_at_effectful_instructions(cfg: &mut ControlFlowGraph) -> Result<()> {
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
            cfg.unconditional_edge(block_index, tail_index)?;
        }
    }

    Ok(())
}

/// Add additional resolve edges to the transient control flow graph.
/// This makes sure that the transient execution can stop/resolve at any point in time.
///
/// Instead of adding resolve edges for each single instruction,
/// we limit them to "effect-ful" instructions only.
fn add_transient_resolve_edges(cfg: &mut ControlFlowGraph) -> Result<()> {
    let resolve_block_index = cfg.exit().unwrap();

    split_blocks_at_effectful_instructions(cfg)?;

    let block_indices: Vec<usize> = cfg
        .blocks()
        .iter()
        .filter(|block| {
            block.index() != resolve_block_index && block.instruction_count_by_address() > 0
        })
        .map(|block| block.index())
        .collect();

    for block_index in block_indices {
        let tail_index = cfg.split_block_at_end(block_index)?;

        let zero = BitVector::constant_u64(0, SPECULATION_WINDOW_SIZE);

        let continue_execution = BitVector::sgt(spec_win().into(), zero.clone())?;
        cfg.conditional_edge(block_index, tail_index, continue_execution)?;

        let resolve = BitVector::sle(spec_win().into(), zero)?;
        cfg.conditional_edge(block_index, resolve_block_index, resolve)?;
    }

    Ok(())
}

/// Appends "_spec_win := _spec_win - |instructions in BB|" to the end of each transient basic block.
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

/// Removes all statically unreachable transient edges.
///
/// As the length of the speculative execution is limited by the speculation window,
/// we can simply compute the maximum remaining speculation window for each transient block
/// and remove all transient edges which can never be taken (i.e. remaining window is zero).
///
/// The algorithm works as follows:
///   1. The remaining speculation window of all transient entry points are set to the initial speculation window.
///   2. The remaining speculation window of each transient block is maximized (i.e. maximize remaining_spec_window_in).
///   3. The set of transient blocks `S` which have an empty speculation window after they are executed
///      (i.e. remaining_spec_window_out = 0) is determined.
///   4. For each transient block in the set `S` all outgoing edges, expect the resolve edge, are removed.
///
/// This function may yield unreachable blocks which can be removed by simplifying the CFG.
fn remove_unreachable_transient_edges(
    cfg: &mut ControlFlowGraph,
    transient_entries: &Vec<usize>,
    init_spec_window: usize,
) -> Result<()> {
    let resolve_block_index = cfg.exit().unwrap();

    let mut remaining_spec_window_in: BTreeMap<usize, usize> = BTreeMap::new();
    let mut remaining_spec_window_out: BTreeMap<usize, usize> = BTreeMap::new();

    // Initialize remaining speculation window for transient entry points
    let mut queue = transient_entries.clone();
    transient_entries.iter().for_each(|&index| {
        remaining_spec_window_in.insert(index, init_spec_window);
    });

    // Maximize remaining speculation window
    while let Some(index) = queue.pop() {
        let block = cfg.block(index)?;
        let inst_count = block.instruction_count_by_address();

        let spec_win_in = remaining_spec_window_in.get(&index).cloned().unwrap();
        let spec_win_out = spec_win_in.checked_sub(inst_count).unwrap_or(0);
        remaining_spec_window_out.insert(index, spec_win_out);

        for successor in cfg.successor_indices(index)? {
            let succ_spec_win_in = remaining_spec_window_in.entry(successor).or_default();
            if spec_win_out > *succ_spec_win_in {
                *succ_spec_win_in = spec_win_out;
                queue.push(successor);
            }
        }
    }

    let transient_blocks_rollback: Vec<usize> = cfg
        .blocks()
        .iter()
        .filter(|block| {
            let remaining_spec_window = remaining_spec_window_out
                .get(&block.index())
                .cloned()
                .unwrap_or_default();
            remaining_spec_window == 0
        })
        .map(|block| block.index())
        .collect();

    for index in transient_blocks_rollback {
        for successor in cfg.successor_indices(index)? {
            if successor == resolve_block_index {
                continue;
            }
            cfg.remove_edge(index, successor)?;
        }
    }

    Ok(())
}
