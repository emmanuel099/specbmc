use crate::error::Result;
use crate::ir;
use falcon::il;

pub fn translate_function(function: &il::Function) -> Result<ir::Program> {
    let block_graph = translate_control_flow_graph(function.control_flow_graph())?;

    Ok(ir::Program::new(block_graph))
}

fn translate_control_flow_graph(cfg: &il::ControlFlowGraph) -> Result<ir::BlockGraph> {
    let entry = cfg.entry().ok_or("CFG entry must be set")?;

    let mut block_graph = ir::BlockGraph::new();

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

/// Computes the execution condition of the block.
///
/// The execution condition is defined as:
///    exec(b) = true                                               if pred(b) is empty
///              Disjunction of p in pred(b). (exec(p) /\ t(p, b))  otherwise
fn compute_execution_condition(
    block_graph: &ir::BlockGraph,
    cfg: &il::ControlFlowGraph,
    block_index: usize,
) -> Result<ir::Expression> {
    let mut transitions = vec![];

    let predecessors = cfg.predecessor_indices(block_index)?;
    if predecessors.is_empty() {
        return Ok(ir::Boolean::constant(true).into());
    }

    for pred_index in predecessors {
        let predecessor = block_graph.block(pred_index)?;
        let edge = cfg.edge(pred_index, block_index)?;
        let transition = match edge.condition() {
            Some(condition) => ir::Boolean::and(
                predecessor.execution_condition_variable().clone().into(),
                translate_expr(condition)?,
            )?,
            None => predecessor.execution_condition_variable().clone().into(), // unconditional transition
        };
        transitions.push(transition);
    }

    ir::Boolean::disjunction(&transitions)
}

fn translate_block(
    block_graph: &ir::BlockGraph,
    cfg: &il::ControlFlowGraph,
    block_index: usize,
) -> Result<ir::Block> {
    let mut block = ir::Block::new(block_index);
    block.set_execution_condition(compute_execution_condition(&block_graph, cfg, block_index)?);

    let src_block = cfg.block(block_index)?;

    for phi_node in src_block.phi_nodes() {
        let var = translate_scalar(phi_node.out())?;
        let expr = ir::Expression::constant(ir::BitVector::constant(42, 64));
        block.add_let(var, expr)?;
    }

    for instruction in src_block.instructions() {
        match instruction.operation() {
            il::Operation::Assign { dst, src } => {
                let var = translate_scalar(dst)?;
                let expr = translate_expr(src)?;
                let expr = match (var.sort(), expr.sort()) {
                    (ir::Sort::Bool, ir::Sort::BitVector(1)) => ir::BitVector::to_boolean(expr)?,
                    (ir::Sort::BitVector(bit_width), ir::Sort::Bool) => {
                        ir::BitVector::from_boolean(*bit_width, expr)?
                    }
                    _ => expr,
                };
                let node = block.add_let(var, expr)?;
                node.set_address(instruction.address());
            }
            il::Operation::Store { index, src } => {
                let mem_old = ir::Variable::new("memory", ir::Sort::Memory(64));
                let mem_new = ir::Variable::new("memory", ir::Sort::Memory(64));
                let addr = translate_expr(index)?;
                let value = translate_expr(src)?;
                let node = block.add_let(mem_new, ir::Memory::store(mem_old, addr, value)?)?;
                node.set_address(instruction.address());
            }
            il::Operation::Load { dst, index } => {
                let bit_width = dst.bits();
                let var = translate_scalar(dst)?;
                let mem = ir::Variable::new("memory", ir::Sort::Memory(64));
                let addr = translate_expr(index)?;
                let node = block.add_let(var, ir::Memory::load(bit_width, mem, addr)?)?;
                node.set_address(instruction.address());
            }
            il::Operation::Branch { .. } => continue,
            il::Operation::Intrinsic { .. } => continue,
            il::Operation::Nop => continue,
        }
    }

    Ok(block)
}

fn translate_expr(expr: &il::Expression) -> Result<ir::Expression> {
    match expr {
        il::Expression::Scalar(scalar) => {
            let var = translate_scalar(scalar)?;
            Ok(var.into())
        }
        il::Expression::Constant(constant) => {
            let constant = ir::BitVector::constant_big(constant.value().clone(), constant.bits());
            Ok(constant.into())
        }
        il::Expression::Add(lhs, rhs) => {
            ir::BitVector::add(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Sub(lhs, rhs) => {
            ir::BitVector::sub(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Mul(lhs, rhs) => {
            ir::BitVector::mul(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Divu(lhs, rhs) => {
            ir::BitVector::udiv(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Modu(lhs, rhs) => {
            ir::BitVector::umod(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Divs(lhs, rhs) => {
            ir::BitVector::sdiv(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Mods(lhs, rhs) => {
            ir::BitVector::smod(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::And(lhs, rhs) => {
            ir::BitVector::and(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Or(lhs, rhs) => {
            ir::BitVector::or(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Xor(lhs, rhs) => {
            ir::BitVector::xor(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Shl(lhs, rhs) => {
            ir::BitVector::shl(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Shr(lhs, rhs) => {
            ir::BitVector::lshr(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Cmpeq(lhs, rhs) => {
            ir::Expression::equal(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Cmpneq(lhs, rhs) => {
            ir::Expression::unequal(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Cmpltu(lhs, rhs) => {
            ir::BitVector::ult(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Cmplts(lhs, rhs) => {
            ir::BitVector::slt(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Zext(bits, src) => {
            let expr = translate_expr(src)?;
            match expr.sort() {
                ir::Sort::Bool => ir::BitVector::from_boolean(*bits, expr),
                _ => ir::BitVector::zero_extend(*bits, expr),
            }
        }
        il::Expression::Sext(bits, src) => ir::BitVector::sign_extend(*bits, translate_expr(src)?),
        il::Expression::Trun(bits, src) => ir::BitVector::truncate(*bits, translate_expr(src)?),
        il::Expression::Ite(cond, then, else_) => ir::Expression::ite(
            translate_expr(cond)?,
            translate_expr(then)?,
            translate_expr(else_)?,
        ),
    }
}

fn translate_scalar(scalar: &il::Scalar) -> Result<ir::Variable> {
    let sort = if scalar.bits() > 1 {
        ir::Sort::BitVector(scalar.bits())
    } else {
        ir::Sort::Bool
    };
    let mut var = ir::Variable::new(scalar.name(), sort);
    var.set_version(scalar.ssa());
    Ok(var)
}
