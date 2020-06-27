use crate::cex::{
    AnnotatedBlock, AnnotatedEdge, Composition, ControlFlowGraph, CounterExample, Effect,
};
use crate::error::Result;
use crate::expr::{Constant, Expression, Variable};
use crate::hir;
use crate::solver::Model;

pub fn build_counter_example(program: &hir::Program, model: &dyn Model) -> Result<CounterExample> {
    let mut cex = create_cex_from(program)?;

    let cfg = program.control_flow_graph();

    for &composition in &[Composition::A, Composition::B] {
        let trace = extract_trace(cfg, model, composition)?;
        add_trace_info(&mut cex, model, &trace, composition)?;
    }

    Ok(cex)
}

fn extract_trace(
    cfg: &hir::ControlFlowGraph,
    model: &dyn Model,
    composition: Composition,
) -> Result<Vec<usize>> {
    let mut trace = Vec::new();
    trace.push(cfg.entry()?);

    'outer: loop {
        let last = trace.last().unwrap();

        for edge in cfg.edges_out(*last)? {
            match edge.condition() {
                Some(expr) => {
                    let executed = expr.evaluate(model, composition);
                    if let Some(Constant::Boolean(true)) = executed {
                        trace.push(edge.tail());
                        continue 'outer;
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
    model: &dyn Model,
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
                if let Some(value) = var.evaluate(model, composition) {
                    annotated_inst
                        .annotation_mut(composition)
                        .add_assignment(var.clone().into(), value);
                }
            }

            for operation in inst.operations() {
                if let hir::Operation::Observable { exprs } = operation {
                    for expr in exprs {
                        if let Some(value) = expr.evaluate(model, composition) {
                            annotated_inst
                                .annotation_mut(composition)
                                .add_assignment(expr.clone(), value);
                        }
                    }
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

fn eval_effect(
    effect: &hir::Effect,
    model: &dyn Model,
    composition: Composition,
) -> Option<Effect> {
    match effect {
        hir::Effect::Conditional { condition, effect } => {
            match condition.evaluate(model, composition) {
                Some(Constant::Boolean(true)) => eval_effect(effect, model, composition),
                _ => None,
            }
        }
        hir::Effect::CacheFetch { address, bit_width } => {
            match address.evaluate(model, composition) {
                Some(address) => Some(Effect::cache_fetch(address, *bit_width)),
                _ => None,
            }
        }
        hir::Effect::BranchTarget { location, target } => {
            match (
                location.evaluate(model, composition),
                target.evaluate(model, composition),
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
                location.evaluate(model, composition),
                condition.evaluate(model, composition),
            ) {
                (Some(location), Some(condition)) => {
                    Some(Effect::branch_condition(location, condition))
                }
                _ => None,
            }
        }
    }
}

trait Evaluate {
    fn evaluate(&self, model: &dyn Model, composition: Composition) -> Option<Constant>;
}

impl Evaluate for Variable {
    fn evaluate(&self, model: &dyn Model, composition: Composition) -> Option<Constant> {
        if self.sort().is_predictor() {
            // FIXME
            return None;
        }
        model.get_interpretation(&self.self_compose(composition.number()))
    }
}

impl Evaluate for Expression {
    fn evaluate(&self, model: &dyn Model, composition: Composition) -> Option<Constant> {
        if self.sort().is_predictor() {
            // FIXME
            return None;
        }
        model.evaluate(&self.self_compose(composition.number()))
    }
}
