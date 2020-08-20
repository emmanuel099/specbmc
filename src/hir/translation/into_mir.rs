use crate::error::Result;
use crate::expr::{Boolean, Expression};
use crate::hir;
use crate::ir::TryTranslateInto;
use crate::mir;

/// We have a 2-safety hyperproperty
const NUMBER_OF_SELF_COMPOSITIONS: usize = 2;

impl TryTranslateInto<mir::Program> for hir::InlinedProgram {
    fn try_translate_into(&self) -> Result<mir::Program> {
        let block_graph = translate_control_flow_graph(self.control_flow_graph())?;
        Ok(mir::Program::new(block_graph, NUMBER_OF_SELF_COMPOSITIONS))
    }
}

fn translate_control_flow_graph(cfg: &hir::ControlFlowGraph) -> Result<mir::BlockGraph> {
    let mut block_graph = mir::BlockGraph::new();

    for block in cfg.blocks() {
        block_graph.add_block(translate_block(cfg, block)?)?;
    }

    for edge in cfg.edges() {
        block_graph.add_edge(edge.head(), edge.tail())?;
    }

    block_graph.set_entry(cfg.entry()?)?;
    block_graph.set_exit(cfg.exit()?)?;

    Ok(block_graph)
}

/// Translates a block by:
///   - making the control-flow explicit by computing the block execution condition
///   - translating all instructions into corresponding MIR nodes
///   - translating phi nodes into MIR assignments
fn translate_block(cfg: &hir::ControlFlowGraph, src_block: &hir::Block) -> Result<mir::Block> {
    let mut block = mir::Block::new(src_block.index());

    block.set_execution_condition(compute_execution_condition(cfg, block.index())?);

    for phi_node in src_block.phi_nodes() {
        let expr = compute_phi_expr(cfg, block.index(), phi_node)?;
        block.add_node(mir::Node::assign(phi_node.out().clone(), expr)?);
    }

    for instruction in src_block.instructions() {
        if let Some(node) = translate_operation(instruction.operation())? {
            block.add_node(node);
        }
    }

    Ok(block)
}

fn translate_operation(operation: &hir::Operation) -> Result<Option<mir::Node>> {
    use hir::Operation::*;
    let node = match operation {
        Assign { variable, expr } => Some(mir::Node::assign(variable.clone(), expr.clone())?),
        Assert { condition } => Some(mir::Node::assert(condition.clone())?),
        Assume { condition } => Some(mir::Node::assume(condition.clone())?),
        Observable { exprs } => Some(mir::Node::hyper_assert(equal_under_self_composition(
            exprs,
        ))?),
        Indistinguishable { exprs } => Some(mir::Node::hyper_assume(
            equal_under_self_composition(exprs),
        )?),
        Store { .. } => panic!("Unexpected store operation, should have been made explicit"),
        Load { .. } => panic!("Unexpected load operation, should have been made explicit"),
        Call { .. } => panic!("Unexpected call operation, should have been inlined"),
        Skip { .. } => None,
        Branch { .. } | ConditionalBranch { .. } | Barrier => {
            // Ignore because they are already implicitly encoded into the CFG
            None
        }
    };
    Ok(node)
}

/// Gives the transition condition of edge (p, q) which is defined as:
///   - exec(p) /\ true if the edge is an unconditional edge
///   - exec(p) /\ c if the edge is a conditional edge with condition c
fn transition_condition(edge: &hir::Edge) -> Result<Expression> {
    let pred_exec_cond = mir::Block::execution_condition_variable_for_index(edge.head());
    match edge.condition() {
        Some(condition) => Boolean::and(pred_exec_cond.into(), condition.clone()),
        None => Ok(pred_exec_cond.into()), // unconditional transition
    }
}

/// Computes the execution condition of the block.
///
/// For the execution condition of block Q to become true it must hold that:
///   - Either the block is the CFG entry (no predecessors)
///   - Or
///      1. The execution condition of a predecessor block P is true
///      2. The condition of the edge from P to Q evaluates to true
///
/// The execution condition is defined as:
///    exec(b) = true                                               if pred(b) is empty
///              Disjunction of p in pred(b). (exec(p) /\ t(p, b))  otherwise
fn compute_execution_condition(
    cfg: &hir::ControlFlowGraph,
    block_index: usize,
) -> Result<Expression> {
    let mut transitions = vec![];

    let predecessors = cfg.predecessor_indices(block_index)?;
    if predecessors.is_empty() {
        return Ok(Boolean::constant(true));
    }

    for pred_index in predecessors {
        let edge = cfg.edge(pred_index, block_index)?;
        transitions.push(transition_condition(edge)?);
    }

    Boolean::disjunction(&transitions)
}

/// Transforms the phi node into a conditional expression s.t. it can be assigned to a variable.
///
/// For example, let the phi node be `c = phi [c.0, 0x0] [c.1, 0x1] [c.2, 0x2]` then this function
/// will produce `ite(transition_from(0x2), c.2, ite(transition_from(0x1), c.1, c.0))`.
fn compute_phi_expr(
    cfg: &hir::ControlFlowGraph,
    block_index: usize,
    phi_node: &hir::PhiNode,
) -> Result<Expression> {
    let mut phi_expr: Option<Expression> = None;

    for pred_index in cfg.predecessor_indices(block_index)? {
        let edge = cfg.edge(pred_index, block_index)?;
        let trans_cond = transition_condition(edge)?;
        let var = phi_node.incoming_variable(pred_index).unwrap().clone();

        if let Some(expr) = phi_expr {
            phi_expr = Some(Expression::ite(trans_cond, var.into(), expr)?);
        } else {
            phi_expr = Some(var.into());
        }
    }

    Ok(phi_expr.unwrap())
}

/// Create an expression to enforce equality of all given expression under 2-way self composition.
/// For example, let `exprs` be `[x, y]` then this function will produce `x@0 == x@1 /\ y@0 == y@1`.
fn equal_under_self_composition(exprs: &[Expression]) -> Expression {
    Boolean::conjunction(
        &exprs
            .iter()
            .map(|e| Expression::equal(e.self_compose(0), e.self_compose(1)).unwrap())
            .collect::<Vec<Expression>>(),
    )
    .unwrap()
}
