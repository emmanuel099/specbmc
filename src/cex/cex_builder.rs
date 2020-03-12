use crate::cex::{
    AnnotatedBlock, AnnotatedEdge, Composition, ControlFlowGraph, CounterExample, Effect,
};
use crate::error::Result;
use crate::expr::{Constant, Expression, Variable};
use crate::hir;
use crate::solver::Model;

pub fn build_counter_example(
    program: &hir::Program,
    model: &Box<dyn Model>,
) -> Result<CounterExample> {
    let cfg = program.control_flow_graph();

    let trace_a = extract_trace(cfg, model, Composition::A)?;
    let trace_b = extract_trace(cfg, model, Composition::B)?;

    let mut cex = create_cex_from(program)?;
    add_trace_info(&mut cex, model, &trace_a, Composition::A)?;
    add_trace_info(&mut cex, model, &trace_b, Composition::B)?;

    Ok(cex)
}

fn extract_trace(
    cfg: &hir::ControlFlowGraph,
    model: &Box<dyn Model>,
    composition: Composition,
) -> Result<Vec<usize>> {
    let entry = cfg.entry().ok_or("CFG entry must be set")?;

    let mut trace = Vec::new();
    trace.push(entry);

    'outer: loop {
        let last = trace.last().unwrap();

        for edge in cfg.edges_out(*last)? {
            match edge.condition() {
                Some(expr) => {
                    let executed = eval_expr(expr, model, composition);
                    match executed {
                        Some(Constant::Boolean(true)) => {
                            trace.push(edge.tail());
                            continue 'outer;
                        }
                        _ => (),
                    }
                }
                None => {
                    trace.push(edge.tail());
                    continue 'outer;
                }
            }
        }

        // No edge was executed, done.
        return Ok(trace);
    }
}

fn create_cex_from(program: &hir::Program) -> Result<CounterExample> {
    let mut cex_cfg = ControlFlowGraph::new();

    let cfg = program.control_flow_graph();
    for block in cfg.blocks() {
        let cex_block = AnnotatedBlock::new(block.into());
        cex_cfg.add_block(cex_block)?;
    }

    for edge in cfg.edges() {
        let cex_edge = AnnotatedEdge::new(edge.clone());
        cex_cfg.add_edge(cex_edge)?;
    }

    Ok(CounterExample::new(cex_cfg))
}

fn add_trace_info(
    cex: &mut CounterExample,
    model: &Box<dyn Model>,
    trace: &[usize],
    composition: Composition,
) -> Result<()> {
    let cfg = cex.control_flow_graph_mut();

    for index in trace {
        let annotated_block = cfg.block_mut(*index)?;

        annotated_block
            .annotation_mut(composition)
            .mark_as_executed();

        for annotated_inst in annotated_block.block_mut().instructions_mut() {
            let inst = annotated_inst.instruction().clone();

            inst.effects()
                .iter()
                .filter_map(|effect| eval_effect(effect, model, composition))
                .for_each(|effect| {
                    annotated_inst
                        .annotation_mut(composition)
                        .add_effect(effect);
                });

            for var in inst.variables_written() {
                if let Some(value) = eval_var(var, model, composition) {
                    annotated_inst
                        .annotation_mut(composition)
                        .add_assignment(var.clone(), value);
                }
            }

            for operation in inst.operations() {
                match operation {
                    hir::Operation::Observable { exprs } => {
                        for expr in exprs {
                            for var in expr.variables() {
                                if let Some(value) = eval_var(var, model, composition) {
                                    annotated_inst
                                        .annotation_mut(composition)
                                        .add_assignment(var.clone(), value);
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    for (b1, b2) in trace.iter().zip(trace.iter().skip(1)) {
        let edge = cfg.edge_mut(*b1, *b2)?;
        edge.annotation_mut(composition).mark_as_executed();
    }

    Ok(())
}

fn eval_var(var: &Variable, model: &Box<dyn Model>, composition: Composition) -> Option<Constant> {
    if var.sort().is_predictor() {
        // FIXME
        return None;
    }
    model.get_interpretation(&var.self_compose(composition.number()))
}

fn eval_expr(
    expr: &Expression,
    model: &Box<dyn Model>,
    composition: Composition,
) -> Option<Constant> {
    if expr.sort().is_predictor() {
        // FIXME
        return None;
    }
    model.evaluate(&expr.self_compose(composition.number()))
}

fn eval_effect(
    effect: &hir::Effect,
    model: &Box<dyn Model>,
    composition: Composition,
) -> Option<Effect> {
    match effect {
        hir::Effect::Conditional { condition, effect } => {
            match eval_expr(condition, model, composition) {
                Some(Constant::Boolean(true)) => eval_effect(effect, model, composition),
                _ => None,
            }
        }
        hir::Effect::CacheFetch { address, bit_width } => {
            match eval_expr(address, model, composition) {
                Some(address) => Some(Effect::cache_fetch(address, *bit_width)),
                _ => None,
            }
        }
        hir::Effect::BranchTarget { location, target } => {
            match (
                eval_expr(location, model, composition),
                eval_expr(target, model, composition),
            ) {
                (Some(location), Some(target)) => Some(Effect::branch_target(location, target)),
                _ => None,
            }
        }
        hir::Effect::BranchCondition {
            location,
            condition,
        } => {
            match (
                eval_expr(location, model, composition),
                eval_expr(condition, model, composition),
            ) {
                (Some(location), Some(condition)) => {
                    Some(Effect::branch_condition(location, condition))
                }
                _ => None,
            }
        }
    }
}
