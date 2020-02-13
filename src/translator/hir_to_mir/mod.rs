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

    let mut block_graph = mir::BlockGraph::new();

    let topological_ordering = cfg.graph().compute_topological_ordering(entry)?;
    for block_index in topological_ordering {
        let block = translate_block(&block_graph, cfg, block_index)?;
        block_graph.add_block(block)?;
    }

    for edge in cfg.edges() {
        block_graph.add_edge(edge.head(), edge.tail())?;
    }

    block_graph.set_entry(entry)?;

    Ok(block_graph)
}

fn transition_condition(predecessor: &mir::Block, edge: &hir::Edge) -> Result<expr::Expression> {
    match edge.condition() {
        Some(condition) => expr::Boolean::and(
            predecessor.execution_condition_variable().clone().into(),
            condition.clone(),
        ),
        None => Ok(predecessor.execution_condition_variable().clone().into()), // unconditional transition
    }
}

/// Computes the execution condition of the block.
///
/// The execution condition is defined as:
///    exec(b) = true                                               if pred(b) is empty
///              Disjunction of p in pred(b). (exec(p) /\ t(p, b))  otherwise
fn compute_execution_condition(
    block_graph: &mir::BlockGraph,
    cfg: &hir::ControlFlowGraph,
    block_index: usize,
) -> Result<expr::Expression> {
    let mut transitions = vec![];

    let predecessors = cfg.predecessor_indices(block_index)?;
    if predecessors.is_empty() {
        return Ok(expr::Boolean::constant(true));
    }

    for pred_index in predecessors {
        let predecessor = block_graph.block(pred_index)?;
        let edge = cfg.edge(pred_index, block_index)?;
        transitions.push(transition_condition(predecessor, edge)?);
    }

    expr::Boolean::disjunction(&transitions)
}

fn translate_block(
    block_graph: &mir::BlockGraph,
    cfg: &hir::ControlFlowGraph,
    block_index: usize,
) -> Result<mir::Block> {
    let mut block = mir::Block::new(block_index);
    block.set_execution_condition(compute_execution_condition(&block_graph, cfg, block_index)?);

    let src_block = cfg.block(block_index)?;

    for phi_node in src_block.phi_nodes() {
        let mut phi_expr: Option<expr::Expression> = None;

        for pred_index in cfg.predecessor_indices(block_index)? {
            let predecessor = block_graph.block(pred_index)?;
            let edge = cfg.edge(pred_index, block_index)?;
            let phi_cond = transition_condition(predecessor, edge)?;
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
                let bit_width = match variable.sort() {
                    expr::Sort::BitVector(width) => *width,
                    _ => bail!("Expected bit vector sort for load variable"),
                };
                let node = block.add_let(
                    variable.clone(),
                    expr::Memory::load(bit_width, memory.clone().into(), address.clone())?,
                )?;
                node.set_address(instruction.address());
            }
            hir::Operation::Branch { .. } | hir::Operation::Barrier => continue,
        }
    }

    Ok(block)
}
