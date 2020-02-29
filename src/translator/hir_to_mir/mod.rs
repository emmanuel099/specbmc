use crate::error::Result;
use crate::expr;
use crate::hir;
use crate::mir;

pub fn translate_program(program: &hir::Program) -> Result<mir::Program> {
    let block_graph = translate_control_flow_graph(program.control_flow_graph())?;

    Ok(mir::Program::new(block_graph))
}

fn translate_control_flow_graph(cfg: &hir::ControlFlowGraph) -> Result<mir::BlockGraph> {
    let mut block_graph = mir::BlockGraph::new();

    for block in cfg.blocks() {
        block_graph.add_block(translate_block(cfg, block)?)?;
    }

    for edge in cfg.edges() {
        block_graph.add_edge(edge.head(), edge.tail())?;
    }

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
            if !phi_node.has_incoming(pred_index) {
                // The SSA transformation doesn't add phi inputs for variables which
                // don't survive the rollback (e.g. memory). Therefore, it's possible
                // that an incoming edge has no corresponding phi-node input.
                continue;
            }

            let edge = cfg.edge(pred_index, block.index())?;
            let phi_cond = transition_condition(edge)?;
            let phi_var = phi_node.incoming_variable(pred_index).unwrap().clone();

            if let Some(expr) = phi_expr {
                phi_expr = Some(expr::Expression::ite(phi_cond, phi_var.into(), expr)?);
            } else {
                phi_expr = Some(phi_var.into());
            }
        }

        block.add_node(mir::Node::new_let(
            phi_node.out().clone(),
            phi_expr.unwrap(),
        )?);
    }

    for instruction in src_block.instructions() {
        let mut nodes = translate_operation(instruction.operation())?;
        nodes
            .iter_mut()
            .for_each(|node| node.set_address(instruction.address()));
        block.append_nodes(&mut nodes);
    }

    Ok(block)
}

fn translate_operation(operation: &hir::Operation) -> Result<Vec<mir::Node>> {
    let mut nodes = Vec::new();

    match operation {
        hir::Operation::Assign { variable, expr } => {
            nodes.push(mir::Node::new_let(variable.clone(), expr.clone())?);
        }
        hir::Operation::Observable { variables } => {
            for variable in variables {
                nodes.push(mir::Node::new_assert_equal_in_self_composition(
                    vec![1, 2],
                    variable.clone().into(),
                ));
            }
        }
        hir::Operation::Indistinguishable { variables } => {
            for variable in variables {
                nodes.push(mir::Node::new_assume_equal_in_self_composition(
                    vec![1, 2],
                    variable.clone().into(),
                ));
            }
        }
        hir::Operation::Parallel(operations) => {
            for operation in operations {
                nodes.append(&mut translate_operation(operation)?);
            }
        }
        hir::Operation::Store { .. } => {
            panic!("Unexpected store operation, should have been made explicit")
        }
        hir::Operation::Load { .. } => {
            panic!("Unexpected load operation, should have been made explicit")
        }
        _ => (),
    }

    Ok(nodes)
}
