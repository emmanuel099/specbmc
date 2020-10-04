use crate::environment::{PredictorStrategy, SPECULATION_WINDOW_SIZE};
use crate::error::Result;
use crate::expr::{BitVector, Boolean, Expression, Predictor, Sort, Variable};
use crate::hir::{Block, ControlFlowGraph, Edge, Operation, RemovedEdgeGuard};
use crate::ir::Transform;
use std::collections::{BTreeMap, HashSet};
use std::convert::TryInto;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Builder)]
struct InstructionRef {
    block: usize,
    index: usize,
    address: u64,
}

impl InstructionRef {
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

#[derive(Builder, Debug)]
pub struct TransientExecution {
    spectre_pht: bool,
    spectre_stl: bool,
    // Allows to skip STL speculation for specific variables.
    // If the address only contains ignored variables, then the STL encoding for the store instruction will be skipped.
    stl_ignored_variables: HashSet<String>,
    predictor_strategy: PredictorStrategy,
    speculation_window: usize,
    // If disabled, no intermediate resolve edges will be added, meaning
    // that transient execution continues until max. speculation window is reached.
    // This may miss some leaks, esp. when using the sequential observe type.
    //
    // With parallel observe the following type of leaks will be missed (assume only cache is visible):
    //     <transient execution>
    //     ...
    //     beqz secret, Else
    // Then:
    //     load tmp, 21
    //     <intermediate resolve with spec win X>
    //     load tmp, 42
    //     jmp End
    // Else:
    //     load tmp, 42
    //     <intermediate resolve with spec win X>
    //     load tmp, 21
    // End:
    //     ...
    //     <resolve>
    // Without intermediate resolve cache will always contain {21,42}.
    // With intermediate resolve there may exists a spec win X, s.t. {21} and {42} is in cache,
    // therefore we get an control-flow leak because of secret condition.
    intermediate_resolve: bool,
}

impl TransientExecution {
    #[allow(clippy::clippy::type_complexity)]
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
                let inst_ref = InstructionRefBuilder::default()
                    .block(block.index())
                    .index(inst_index)
                    .address(address)
                    .build()
                    .unwrap();

                match inst.operation() {
                    Operation::Store { address, .. } => {
                        if self.spectre_stl && !self.skip_stl(address) {
                            // The `Store` instruction can speculatively be by-passed.
                            add_transient_execution_start(
                                &mut default_cfg,
                                &mut transient_start_rollback_points,
                                &inst_ref,
                                self.speculation_window,
                                self.intermediate_resolve,
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
                                self.speculation_window,
                                self.intermediate_resolve,
                            )?;
                        }
                    }
                    _ => (),
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
        let resolve_block_index = transient_cfg.new_block().index();
        transient_cfg.unconditional_edge(cfg.exit().unwrap(), resolve_block_index)?; // end of program -> resolve
        transient_cfg.set_exit(resolve_block_index)?;

        for block in cfg.blocks() {
            for (inst_index, inst) in block.instructions().iter().enumerate().rev() {
                let address = inst.address().unwrap_or_default();
                let inst_ref = InstructionRefBuilder::default()
                    .block(block.index())
                    .index(inst_index)
                    .address(address)
                    .build()
                    .unwrap();

                match inst.operation() {
                    Operation::Store { address, .. } => {
                        if self.spectre_stl && !self.skip_stl(address) {
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

        if self.intermediate_resolve {
            add_transient_resolve_edges(&mut transient_cfg)?;
            append_spec_win_decrease_to_all_blocks(&mut transient_cfg)?;
        }

        // Mark all blocks as transient
        for block in transient_cfg.blocks_mut() {
            block.set_transient();
        }

        Ok((transient_cfg, transient_entry_points))
    }

    fn skip_stl(&self, address: &Expression) -> bool {
        address
            .variables()
            .iter()
            .all(|var| self.stl_ignored_variables.contains(var.name()))
    }
}

impl Default for TransientExecution {
    fn default() -> Self {
        Self {
            spectre_pht: false,
            spectre_stl: false,
            stl_ignored_variables: HashSet::default(),
            predictor_strategy: PredictorStrategy::default(),
            speculation_window: 100,
            intermediate_resolve: true,
        }
    }
}

impl Transform<ControlFlowGraph> for TransientExecution {
    fn name(&self) -> &'static str {
        "TransientExecution"
    }

    fn description(&self) -> String {
        format!(
            "Add transient execution behavior (max. speculation window={})",
            self.speculation_window
        )
    }

    fn transform(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        const MAX_SPEC_WINDOW: usize = 1 << (SPECULATION_WINDOW_SIZE - 1);
        if self.speculation_window >= MAX_SPEC_WINDOW {
            return Err(format!(
                "Expected speculation window < {}, but was {}",
                MAX_SPEC_WINDOW, self.speculation_window
            )
            .into());
        }

        let (mut default_cfg, transient_start_rollback_points) = self.build_default_cfg(cfg)?;

        let (transient_cfg, transient_entry_points) = self.build_transient_cfg(cfg)?;

        // Add copy of the transient graph for each speculating instruction into the default graph.
        // The transient graph is embedded into the default graph by adding transient start and
        // resolve edges between the transient and default graph.
        for (inst_ref, (start, rollback)) in transient_start_rollback_points {
            let transient_entry_point = transient_entry_points.get(&inst_ref).cloned().unwrap();

            // Reduce the size of the transient graph (depth limit by max. speculation window)
            let mut reduced_transient_cfg = transient_cfg.clone();
            remove_unreachable_transient_edges(
                &mut reduced_transient_cfg,
                &[transient_entry_point],
                self.speculation_window,
            )?;

            let saved_vars = reorder_buffer_vars(&reduced_transient_cfg);

            let block_map = default_cfg.insert(&reduced_transient_cfg)?;
            let transient_entry = block_map[&transient_entry_point];
            let transient_resolve = block_map[&reduced_transient_cfg.exit().unwrap()];

            // Save modified variables (registers + memory) for restore on rollback
            let transient_entry_block = default_cfg.block_mut(transient_entry)?;
            save_variables(transient_entry_block, &saved_vars)?;

            // "Discard mis-predicted reorder buffer entries" by restoring the saved variables
            let transient_resolve_block = default_cfg.block_mut(transient_resolve)?;
            restore_variables(transient_resolve_block, &saved_vars)?;

            default_cfg
                .unconditional_edge(start, transient_entry)
                .unwrap();
            default_cfg
                .unconditional_edge(transient_resolve, rollback)
                .unwrap()
                .labels_mut()
                .rollback();
        }

        default_cfg.remove_dead_end_blocks(RemovedEdgeGuard::Ignore)?;
        default_cfg.simplify()?;

        *cfg = default_cfg;

        Ok(())
    }
}

/// The set of variables (registers & memory) which would usually end up in the reorder buffer.
fn reorder_buffer_vars(cfg: &ControlFlowGraph) -> HashSet<&Variable> {
    cfg.variables_written()
        .into_iter()
        .filter(|var| !var.is_rollback_persistent())
        .collect()
}

fn saved_variable_for(var: &Variable) -> Variable {
    Variable::new(format!("_RB_{}", var.name()), var.sort().clone())
}

fn save_variables(block: &mut Block, variables: &HashSet<&Variable>) -> Result<()> {
    for &var in variables.iter() {
        let saved_var = saved_variable_for(var);
        block
            .assign(saved_var, var.clone().into())?
            .labels_mut()
            .pseudo();
    }
    Ok(())
}

fn restore_variables(block: &mut Block, variables: &HashSet<&Variable>) -> Result<()> {
    for &var in variables.iter() {
        let saved_var = saved_variable_for(var);
        block
            .assign(var.clone(), saved_var.into())?
            .labels_mut()
            .pseudo();
    }
    Ok(())
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
    max_spec_window: usize,
    intermediate_resolve: bool,
) -> Result<()> {
    let head_index = inst_ref.block();
    let tail_index = cfg.split_block_at(head_index, inst_ref.index())?;

    let transient_start_index = {
        let transient_start = cfg.new_block();
        transient_start.set_transient();

        if intermediate_resolve {
            // initial speculation window size
            let spec_window = Predictor::speculation_window(
                Predictor::variable().into(),
                BitVector::word_constant(inst_ref.address()),
            )?;
            transient_start
                .assign(spec_win(), spec_window)?
                .labels_mut()
                .pseudo();

            let zero = BitVector::constant_u64(0, SPECULATION_WINDOW_SIZE);
            transient_start
                .assume(BitVector::sgt(spec_win().into(), zero)?)?
                .labels_mut()
                .pseudo();

            transient_start
                .assume(BitVector::sle(
                    spec_win().into(),
                    BitVector::constant_u64(
                        max_spec_window.try_into().unwrap(),
                        SPECULATION_WINDOW_SIZE,
                    ),
                )?)?
                .labels_mut()
                .pseudo();
        }

        transient_start.index()
    };

    let transient_exec = Predictor::speculate(
        Predictor::variable().into(),
        BitVector::word_constant(inst_ref.address()),
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
        BitVector::word_constant(inst_ref.address()),
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
    let speculate_index = cfg.new_block().index();

    let speculate = Predictor::speculate(
        Predictor::variable().into(),
        BitVector::word_constant(inst_ref.address()),
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
                            BitVector::word_constant(inst_ref.address()),
                        )?;
                        cfg.block_mut(speculate_index)?
                            .assume(taken.clone())?
                            .labels_mut()
                            .pseudo();
                        cfg.conditional_edge(speculate_index, edge.tail(), taken)?
                            .labels_mut()
                            .taken();
                    } else {
                        let not_taken = Boolean::not(Predictor::taken(
                            Predictor::variable().into(),
                            BitVector::word_constant(inst_ref.address()),
                        )?)?;
                        cfg.block_mut(speculate_index)?
                            .assume(not_taken.clone())?
                            .labels_mut()
                            .pseudo();
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
                        BitVector::word_constant(inst_ref.address()),
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
                        cfg.block_mut(speculate_index)?
                            .assume(taken.clone())?
                            .labels_mut()
                            .pseudo();
                        cfg.conditional_edge(speculate_index, edge.tail(), taken)?
                            .labels_mut()
                            .taken();
                    } else {
                        let not_taken = Boolean::not(edge.condition().unwrap().clone())?;
                        cfg.block_mut(speculate_index)?
                            .assume(not_taken.clone())?
                            .labels_mut()
                            .pseudo();
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
        .filter_map(|block| {
            let has_instructions = block.instruction_count_ignoring_pseudo_instructions() > 0;
            if has_instructions && block.index() != resolve_block_index {
                Some(block.index())
            } else {
                None
            }
        })
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
fn append_spec_win_decrease_to_all_blocks(cfg: &mut ControlFlowGraph) -> Result<()> {
    for block in cfg.blocks_mut() {
        let count = block.instruction_count_ignoring_pseudo_instructions();
        if count == 0 {
            continue; // Avoid adding useless decrease by zero instructions
        }

        block
            .assign(
                spec_win(),
                BitVector::sub(
                    spec_win().into(),
                    BitVector::constant_u64(count.try_into().unwrap(), SPECULATION_WINDOW_SIZE),
                )?,
            )?
            .labels_mut()
            .pseudo();
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
///   2. The remaining speculation window of each transient block is maximized (i.e. maximize `remaining_spec_window_in`).
///   3. The set of transient blocks `S` which have an empty speculation window after they are executed
///      (i.e. `remaining_spec_window_out` = 0) is determined.
///   4. For each transient block in the set `S` all outgoing edges, expect the resolve edge, are removed.
///
/// This function may yield unreachable blocks which can be removed by simplifying the CFG.
fn remove_unreachable_transient_edges(
    cfg: &mut ControlFlowGraph,
    transient_entries: &[usize],
    init_spec_window: usize,
) -> Result<()> {
    let resolve_block_index = cfg.exit().unwrap();

    let mut remaining_spec_window_in: BTreeMap<usize, usize> = BTreeMap::new();
    let mut remaining_spec_window_out: BTreeMap<usize, usize> = BTreeMap::new();

    // Initialize remaining speculation window for transient entry points
    let mut queue = transient_entries.to_owned();
    transient_entries.iter().for_each(|&index| {
        remaining_spec_window_in.insert(index, init_spec_window);
    });

    // Maximize remaining speculation window
    while let Some(index) = queue.pop() {
        let block = cfg.block(index)?;
        let inst_count = block.instruction_count_ignoring_pseudo_instructions();

        let spec_win_in = remaining_spec_window_in.get(&index).cloned().unwrap();
        let spec_win_out = spec_win_in.saturating_sub(inst_count);
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
        .filter_map(|block| {
            let remaining_spec_window = remaining_spec_window_out
                .get(&block.index())
                .cloned()
                .unwrap_or_default();

            if remaining_spec_window == 0 {
                Some(block.index())
            } else {
                None
            }
        })
        .collect();

    // Replace all outgoing edges of rollback blocks with an unconditional edge to resolve
    for index in transient_blocks_rollback {
        for successor in cfg.successor_indices(index)? {
            cfg.remove_edge(index, successor, RemovedEdgeGuard::Ignore)?;
        }
        cfg.unconditional_edge(index, resolve_block_index)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::expr::{BitVector, Boolean, Expression, Sort, Variable};
    use crate::util::RenderGraph;

    use std::path::Path;

    fn debug_cfg(
        test_name: &str,
        given_cfg: &ControlFlowGraph,
        encoded_cfg: &ControlFlowGraph,
        expected_cfg: &ControlFlowGraph,
    ) {
        given_cfg
            .render_to_file(Path::new(&format!("{}_given.dot", test_name)))
            .unwrap();
        encoded_cfg
            .render_to_file(Path::new(&format!("{}_encoded.dot", test_name)))
            .unwrap();
        expected_cfg
            .render_to_file(Path::new(&format!("{}_expected.dot", test_name)))
            .unwrap();
    }

    #[test]
    fn test_transient_store() {
        let addr = BitVector::word_constant(42);
        let var = Variable::new("x", Sort::word());

        // Given:
        let given_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block = cfg.new_block();
            block
                .assign(var.clone(), BitVector::word_constant(0))
                .unwrap()
                .set_address(Some(1));
            block
                .store(addr.clone(), var.clone().into())
                .unwrap()
                .set_address(Some(2));
            block
                .load(var.clone(), addr.clone())
                .unwrap()
                .set_address(Some(3));

            cfg
        };

        let inst_ref = InstructionRefBuilder::default()
            .block(0)
            .index(1)
            .address(2)
            .build()
            .unwrap();

        // When:
        let mut encoded_cfg = given_cfg.clone();
        let mut transient_entry_points = BTreeMap::new();
        transient_store(&mut encoded_cfg, &mut transient_entry_points, &inst_ref).unwrap();

        // Then:
        let expected_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block0_index = {
                let block = cfg.new_block();
                block
                    .assign(var.clone(), BitVector::word_constant(0))
                    .unwrap()
                    .set_address(Some(1));
                block.index()
            };

            let block1_index = {
                let block = cfg.new_block();
                block
                    .store(addr.clone(), var.clone().into())
                    .unwrap()
                    .set_address(Some(2));
                block.index()
            };

            let block2_index = {
                let block = cfg.new_block();
                block
                    .load(var.clone(), addr.clone())
                    .unwrap()
                    .set_address(Some(3));
                block.index()
            };

            let store_bypass =
                Predictor::speculate(Predictor::variable().into(), BitVector::word_constant(2))
                    .unwrap();
            let store_exec = Boolean::not(store_bypass.clone()).unwrap();

            cfg.conditional_edge(block0_index, block1_index, store_exec)
                .unwrap();
            cfg.conditional_edge(block0_index, block2_index, store_bypass)
                .unwrap()
                .labels_mut()
                .speculate();
            cfg.unconditional_edge(block1_index, block2_index).unwrap();

            cfg
        };

        let mut expected_transient_entry_points = BTreeMap::new();
        expected_transient_entry_points.insert(inst_ref.clone(), 2); // should bypass store and therefore start in block 2

        /*debug_cfg(
            "transient_execution_test_transient_store",
            &given_cfg,
            &encoded_cfg,
            &expected_cfg,
        );*/

        assert_eq!(expected_cfg, encoded_cfg);
        assert_eq!(expected_transient_entry_points, transient_entry_points);
    }

    #[test]
    fn test_transient_conditional_branch_with_choose_path_predictor() {
        let cond: Expression = Boolean::variable("c").into();
        let neg_cond = Boolean::not(cond.clone()).unwrap();

        // Given:
        let given_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block0_index = {
                let block = cfg.new_block();
                block
                    .conditional_branch(cond.clone(), BitVector::word_constant(4))
                    .unwrap()
                    .set_address(Some(4));
                block.index()
            };

            let block1_index = {
                let block = cfg.new_block();
                block
                    .assign(Variable::new("x", Sort::boolean()), Boolean::constant(true))
                    .unwrap()
                    .set_address(Some(4));
                block.index()
            };

            let block2_index = {
                let block = cfg.new_block();
                block
                    .assign(
                        Variable::new("x", Sort::boolean()),
                        Boolean::constant(false),
                    )
                    .unwrap()
                    .set_address(Some(2));
                block
                    .branch(BitVector::word_constant(5))
                    .unwrap()
                    .set_address(Some(3));
                block.index()
            };

            let block3_index = {
                let block = cfg.new_block();
                block.index()
            };

            cfg.conditional_edge(block0_index, block1_index, cond.clone())
                .unwrap()
                .labels_mut()
                .taken();
            cfg.conditional_edge(block0_index, block2_index, neg_cond.clone())
                .unwrap();
            cfg.unconditional_edge(block1_index, block3_index).unwrap();
            cfg.unconditional_edge(block2_index, block3_index).unwrap();

            cfg
        };

        let inst_ref = InstructionRefBuilder::default()
            .block(0)
            .index(0)
            .address(1)
            .build()
            .unwrap();

        // When:
        let mut encoded_cfg = given_cfg.clone();
        let mut transient_entry_points = BTreeMap::new();
        transient_conditional_branch(
            &mut encoded_cfg,
            &mut transient_entry_points,
            &inst_ref,
            PredictorStrategy::ChoosePath,
        )
        .unwrap();

        // Then:
        let expected_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block0_index = {
                let block = cfg.new_block();
                block.index()
            };

            let block1_index = {
                let block = cfg.new_block();
                block
                    .assign(Variable::new("x", Sort::boolean()), Boolean::constant(true))
                    .unwrap()
                    .set_address(Some(4));
                block.index()
            };

            let block2_index = {
                let block = cfg.new_block();
                block
                    .assign(
                        Variable::new("x", Sort::boolean()),
                        Boolean::constant(false),
                    )
                    .unwrap()
                    .set_address(Some(2));
                block
                    .branch(BitVector::word_constant(5))
                    .unwrap()
                    .set_address(Some(3));
                block.index()
            };

            let block3_index = {
                let block = cfg.new_block();
                block.index()
            };

            let block4_index = {
                let block = cfg.new_block();
                block
                    .conditional_branch(cond.clone(), BitVector::word_constant(4))
                    .unwrap()
                    .set_address(Some(4));
                block.index()
            };

            let block5_index = {
                let block = cfg.new_block();
                block.index()
            };

            cfg.conditional_edge(block4_index, block1_index, cond.clone())
                .unwrap()
                .labels_mut()
                .taken();
            cfg.conditional_edge(block4_index, block2_index, neg_cond.clone())
                .unwrap();
            cfg.unconditional_edge(block1_index, block3_index).unwrap();
            cfg.unconditional_edge(block2_index, block3_index).unwrap();

            let speculate =
                Predictor::speculate(Predictor::variable().into(), BitVector::word_constant(1))
                    .unwrap();
            let not_speculate = Boolean::not(speculate.clone()).unwrap();

            let speculate_taken =
                Predictor::taken(Predictor::variable().into(), BitVector::word_constant(1))
                    .unwrap();
            let speculate_not_taken = Boolean::not(speculate_taken.clone()).unwrap();

            cfg.conditional_edge(block0_index, block4_index, not_speculate)
                .unwrap();
            cfg.conditional_edge(block0_index, block5_index, speculate)
                .unwrap()
                .labels_mut()
                .speculate();
            cfg.conditional_edge(block5_index, block1_index, speculate_taken)
                .unwrap()
                .labels_mut()
                .taken();
            cfg.conditional_edge(block5_index, block2_index, speculate_not_taken)
                .unwrap();

            cfg
        };

        let mut expected_transient_entry_points = BTreeMap::new();
        expected_transient_entry_points.insert(inst_ref.clone(), 5); // block 5 encodes the speculative behavior

        /*debug_cfg(
            "transient_execution_test_transient_conditional_branch_with_choose_path_predictor",
            &given_cfg,
            &encoded_cfg,
            &expected_cfg,
        );*/

        assert_eq!(expected_cfg, encoded_cfg);
        assert_eq!(expected_transient_entry_points, transient_entry_points);
    }

    #[test]
    fn test_transient_conditional_branch_with_invert_condition_predictor() {
        let cond: Expression = Boolean::variable("c").into();
        let neg_cond = Boolean::not(cond.clone()).unwrap();

        // Given:
        let given_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block0_index = {
                let block = cfg.new_block();
                block
                    .conditional_branch(cond.clone(), BitVector::word_constant(4))
                    .unwrap()
                    .set_address(Some(4));
                block.index()
            };

            let block1_index = {
                let block = cfg.new_block();
                block
                    .assign(Variable::new("x", Sort::boolean()), Boolean::constant(true))
                    .unwrap()
                    .set_address(Some(4));
                block.index()
            };

            let block2_index = {
                let block = cfg.new_block();
                block
                    .assign(
                        Variable::new("x", Sort::boolean()),
                        Boolean::constant(false),
                    )
                    .unwrap()
                    .set_address(Some(2));
                block
                    .branch(BitVector::word_constant(5))
                    .unwrap()
                    .set_address(Some(3));
                block.index()
            };

            let block3_index = {
                let block = cfg.new_block();
                block.index()
            };

            cfg.conditional_edge(block0_index, block1_index, cond.clone())
                .unwrap()
                .labels_mut()
                .taken();
            cfg.conditional_edge(block0_index, block2_index, neg_cond.clone())
                .unwrap();
            cfg.unconditional_edge(block1_index, block3_index).unwrap();
            cfg.unconditional_edge(block2_index, block3_index).unwrap();

            cfg
        };

        let inst_ref = InstructionRefBuilder::default()
            .block(0)
            .index(0)
            .address(1)
            .build()
            .unwrap();

        // When:
        let mut encoded_cfg = given_cfg.clone();
        let mut transient_entry_points = BTreeMap::new();
        transient_conditional_branch(
            &mut encoded_cfg,
            &mut transient_entry_points,
            &inst_ref,
            PredictorStrategy::InvertCondition,
        )
        .unwrap();

        // Then:
        let expected_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block0_index = {
                let block = cfg.new_block();
                block.index()
            };

            let block1_index = {
                let block = cfg.new_block();
                block
                    .assign(Variable::new("x", Sort::boolean()), Boolean::constant(true))
                    .unwrap()
                    .set_address(Some(4));
                block.index()
            };

            let block2_index = {
                let block = cfg.new_block();
                block
                    .assign(
                        Variable::new("x", Sort::boolean()),
                        Boolean::constant(false),
                    )
                    .unwrap()
                    .set_address(Some(2));
                block
                    .branch(BitVector::word_constant(5))
                    .unwrap()
                    .set_address(Some(3));
                block.index()
            };

            let block3_index = {
                let block = cfg.new_block();
                block.index()
            };

            let block4_index = {
                let block = cfg.new_block();
                block
                    .conditional_branch(cond.clone(), BitVector::word_constant(4))
                    .unwrap()
                    .set_address(Some(4));
                block.index()
            };

            let block5_index = {
                let block = cfg.new_block();
                block.index()
            };

            cfg.conditional_edge(block4_index, block1_index, cond.clone())
                .unwrap()
                .labels_mut()
                .taken();
            cfg.conditional_edge(block4_index, block2_index, neg_cond.clone())
                .unwrap();
            cfg.unconditional_edge(block1_index, block3_index).unwrap();
            cfg.unconditional_edge(block2_index, block3_index).unwrap();

            let speculate =
                Predictor::speculate(Predictor::variable().into(), BitVector::word_constant(1))
                    .unwrap();
            let not_speculate = Boolean::not(speculate.clone()).unwrap();

            let speculate_neg_cond = Boolean::not(cond.clone()).unwrap();
            let speculate_neg_neg_cond = Boolean::not(neg_cond.clone()).unwrap();

            cfg.conditional_edge(block0_index, block4_index, not_speculate)
                .unwrap();
            cfg.conditional_edge(block0_index, block5_index, speculate)
                .unwrap()
                .labels_mut()
                .speculate();
            cfg.conditional_edge(block5_index, block1_index, speculate_neg_cond.clone())
                .unwrap()
                .labels_mut()
                .taken();
            cfg.conditional_edge(block5_index, block2_index, speculate_neg_neg_cond.clone())
                .unwrap();

            cfg
        };

        let mut expected_transient_entry_points = BTreeMap::new();
        expected_transient_entry_points.insert(inst_ref.clone(), 5); // block 5 encodes the speculative behavior

        /*debug_cfg(
            "transient_execution_test_transient_conditional_branch_with_invert_condition_predictor",
            &given_cfg,
            &encoded_cfg,
            &expected_cfg,
        );*/

        assert_eq!(expected_cfg, encoded_cfg);
        assert_eq!(expected_transient_entry_points, transient_entry_points);
    }

    #[test]
    fn test_transient_barrier() {
        let var = Variable::new("x", Sort::boolean());

        // Given:
        let given_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block0_index = {
                let block = cfg.new_block();
                block
                    .assign(var.clone(), Boolean::constant(false))
                    .unwrap()
                    .set_address(Some(1));
                block.barrier().set_address(Some(2));
                block
                    .assign(var.clone(), Boolean::constant(true))
                    .unwrap()
                    .set_address(Some(3));
                block.index()
            };

            let block1_index = {
                let block = cfg.new_block();
                block.index()
            };

            let block2_index = {
                let block = cfg.new_block();
                block.index()
            };

            cfg.set_exit(block2_index).unwrap();

            cfg.unconditional_edge(block0_index, block1_index).unwrap();
            cfg.unconditional_edge(block1_index, block2_index).unwrap();

            cfg
        };

        let inst_ref = InstructionRefBuilder::default()
            .block(0)
            .index(1)
            .address(2)
            .build()
            .unwrap();

        // When:
        let mut encoded_cfg = given_cfg.clone();
        transient_barrier(&mut encoded_cfg, &inst_ref).unwrap();

        // Then:
        let expected_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block0_index = {
                let block = cfg.new_block();
                block
                    .assign(var.clone(), Boolean::constant(false))
                    .unwrap()
                    .set_address(Some(1));
                block.index()
            };

            let block1_index = {
                let block = cfg.new_block();
                block.index()
            };

            let block2_index = {
                let block = cfg.new_block();
                block.index()
            };

            let block3_index = {
                let block = cfg.new_block();
                block.barrier().set_address(Some(2));
                block
                    .assign(var.clone(), Boolean::constant(true))
                    .unwrap()
                    .set_address(Some(3));
                block.index()
            };

            cfg.set_exit(block2_index).unwrap();

            cfg.unconditional_edge(block0_index, block2_index).unwrap(); // resolve
            cfg.unconditional_edge(block1_index, block2_index).unwrap();
            cfg.unconditional_edge(block3_index, block1_index).unwrap();

            cfg
        };

        /*debug_cfg(
            "transient_execution_test_transient_barrier",
            &given_cfg,
            &encoded_cfg,
            &expected_cfg,
        );*/

        assert_eq!(expected_cfg, encoded_cfg);
    }
}
