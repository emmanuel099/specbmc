use crate::error::Result;
use crate::expr;
use crate::hir;
use crate::mir;

pub fn translate_program(program: &hir::Program) -> Result<mir::Program> {
    let block_graph = translate_control_flow_graph(program.control_flow_graph())?;

    Ok(mir::Program::new(block_graph))
}

fn translate_control_flow_graph(cfg: &hir::ControlFlowGraph) -> Result<mir::BlockGraph> {
    let entry = cfg.entry().ok_or("CFG entry must be set")?;
    let exit = cfg.exit().ok_or("CFG exit must be set")?;

    let mut block_graph = mir::BlockGraph::new();

    for block in cfg.blocks() {
        block_graph.add_block(translate_block(cfg, block)?)?;
    }

    for edge in cfg.edges() {
        block_graph.add_edge(edge.head(), edge.tail())?;
    }

    block_graph.set_entry(entry)?;
    block_graph.set_exit(exit)?;

    Ok(block_graph)
}

fn transition_condition(edge: &hir::Edge) -> Result<expr::Expression> {
    let pred_exec_cond = mir::Block::execution_condition_variable_for_index(edge.head());
    match edge.condition() {
        Some(condition) => expr::Boolean::and(pred_exec_cond.into(), condition.clone()),
        None => Ok(pred_exec_cond.into()), // unconditional transition
    }
}

/// Computes the execution condition of the block.
///
/// The execution condition is defined as:
///    exec(b) = true                                               if pred(b) is empty
///              Disjunction of p in pred(b). (exec(p) /\ t(p, b))  otherwise
fn compute_execution_condition(
    cfg: &hir::ControlFlowGraph,
    block_index: usize,
) -> Result<expr::Expression> {
    let mut transitions = vec![];

    let predecessors = cfg.predecessor_indices(block_index)?;
    if predecessors.is_empty() {
        return Ok(expr::Boolean::constant(true));
    }

    for pred_index in predecessors {
        let edge = cfg.edge(pred_index, block_index)?;
        transitions.push(transition_condition(edge)?);
    }

    expr::Boolean::disjunction(&transitions)
}

fn translate_block(cfg: &hir::ControlFlowGraph, src_block: &hir::Block) -> Result<mir::Block> {
    let mut block = mir::Block::new(src_block.index());

    block.set_execution_condition(compute_execution_condition(cfg, block.index())?);

    for phi_node in src_block.phi_nodes() {
        let mut phi_expr: Option<expr::Expression> = None;

        for pred_index in cfg.predecessor_indices(block.index())? {
            let edge = cfg.edge(pred_index, block.index())?;
            let phi_cond = transition_condition(edge)?;
            let phi_var = phi_node.incoming_variable(pred_index).unwrap().clone();

            if let Some(expr) = phi_expr {
                phi_expr = Some(expr::Expression::ite(phi_cond, phi_var.into(), expr)?);
            } else {
                phi_expr = Some(phi_var.into());
            }
        }

        if let Some(expr) = phi_expr {
            block.add_let(phi_node.out().clone(), expr)?;
        }
    }

    for instruction in src_block.instructions() {
        match instruction.operation() {
            hir::Operation::Assign { variable, expr } => {
                let node = block.add_let(variable.clone(), expr.clone())?;
                node.set_address(instruction.address());
            }
            hir::Operation::Store {
                new_memory,
                memory,
                address,
                expr,
            } => {
                let node = block.add_let(
                    new_memory.clone(),
                    expr::Memory::store(memory.clone().into(), address.clone(), expr.clone())?,
                )?;
                node.set_address(instruction.address());
            }
            hir::Operation::Load {
                variable,
                memory,
                address,
            } => {
                let bit_width = variable.sort().unwrap_bit_vector();
                let node = block.add_let(
                    variable.clone(),
                    expr::Memory::load(bit_width, memory.clone().into(), address.clone())?,
                )?;
                node.set_address(instruction.address());
            }
            hir::Operation::Branch { .. } | hir::Operation::Barrier => continue,
        }

        for effect in instruction.effects() {
            match effect {
                hir::Effect::CacheFetch {
                    new_cache,
                    cache,
                    address,
                    bit_width,
                } => {
                    block.add_let(
                        new_cache.clone(),
                        expr::Cache::fetch(*bit_width, cache.clone().into(), address.clone())?,
                    )?;
                }
            }
        }
    }

    Ok(block)
}
