use crate::error::Result;
use crate::expr;
use crate::hir;
use falcon::il;

pub fn translate_function(function: &il::Function) -> Result<hir::Program> {
    let cfg = translate_control_flow_graph(function.control_flow_graph())?;

    Ok(hir::Program::new(cfg))
}

fn translate_control_flow_graph(src_cfg: &il::ControlFlowGraph) -> Result<hir::ControlFlowGraph> {
    let mut cfg = hir::ControlFlowGraph::new();

    for block in src_cfg.blocks() {
        cfg.add_block(translate_block(block)?)?;
    }

    for edge in src_cfg.edges() {
        match edge.condition() {
            Some(condition) => {
                let condition = translate_expr(condition)?;
                cfg.conditional_edge(edge.head(), edge.tail(), condition)?;
            }
            None => cfg.unconditional_edge(edge.head(), edge.tail())?,
        }
    }

    // add a dedicated entry block
    let src_entry = src_cfg.entry().ok_or("CFG entry must be set")?;
    let entry = cfg.new_block()?.index();
    cfg.unconditional_edge(entry, src_entry)?;
    cfg.set_entry(entry)?;

    // add a dedicated exit block and connect all blocks without successor to it
    let unconnected_blocks: Vec<usize> = cfg
        .graph()
        .vertices_without_successors()
        .iter()
        .map(|block| block.index())
        .collect();
    let exit = cfg.new_block()?.index();
    for block_index in unconnected_blocks {
        cfg.unconditional_edge(block_index, exit)?;
    }
    cfg.set_exit(exit)?;

    Ok(cfg)
}

fn translate_block(src_block: &il::Block) -> Result<hir::Block> {
    let mut block = hir::Block::new(src_block.index());

    for instruction in src_block.instructions() {
        match instruction.operation() {
            il::Operation::Assign { dst, src } => {
                let variable = translate_scalar(dst)?;
                let expr = translate_expr(src)?;
                let expr = maybe_cast(expr, variable.sort())?;
                let inst = block.assign(variable, expr);
                inst.set_address(instruction.address());
            }
            il::Operation::Store { index, src } => {
                let address = translate_expr(index)?;
                let expr = translate_expr(src)?;
                let inst = block.store(address, expr);
                inst.set_address(instruction.address());
            }
            il::Operation::Load { dst, index } => {
                let variable = translate_scalar(dst)?;
                let address = translate_expr(index)?;
                let inst = block.load(variable, address);
                inst.set_address(instruction.address());
            }
            il::Operation::Branch { target } => {
                let target = translate_expr(target)?;
                let inst = block.branch(target);
                inst.set_address(instruction.address());
            }
            il::Operation::ConditionalBranch { condition, target } => {
                let condition = translate_expr(condition)?;
                let target = translate_expr(target)?;
                let inst = block.conditional_branch(condition, target);
                inst.set_address(instruction.address());
            }
            il::Operation::Intrinsic { intrinsic } => match intrinsic.mnemonic() {
                "mfence" | "lfence" | "spbarr" => {
                    let inst = block.barrier();
                    inst.set_address(instruction.address());
                }
                _ => continue,
            },
            il::Operation::Nop => continue,
        }
    }

    Ok(block)
}

fn maybe_cast(expr: expr::Expression, target_sort: &expr::Sort) -> Result<expr::Expression> {
    match (target_sort, expr.sort()) {
        (expr::Sort::Boolean, expr::Sort::BitVector(1)) => expr::BitVector::to_boolean(expr),
        (expr::Sort::BitVector(bit_width), expr::Sort::Boolean) => {
            expr::BitVector::from_boolean(*bit_width, expr)
        }
        _ => Ok(expr),
    }
}

fn translate_expr(expr: &il::Expression) -> Result<expr::Expression> {
    match expr {
        il::Expression::Scalar(scalar) => {
            let var = translate_scalar(scalar)?;
            Ok(var.into())
        }
        il::Expression::Constant(constant) => {
            let constant = if constant.bits() > 1 {
                expr::BitVector::constant_big(constant.value().clone(), constant.bits())
            } else {
                expr::Boolean::constant(constant.is_one())
            };
            Ok(constant)
        }
        il::Expression::Add(lhs, rhs) => {
            expr::BitVector::add(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Sub(lhs, rhs) => {
            expr::BitVector::sub(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Mul(lhs, rhs) => {
            expr::BitVector::mul(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Divu(lhs, rhs) => {
            expr::BitVector::udiv(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Modu(lhs, rhs) => {
            expr::BitVector::umod(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Divs(lhs, rhs) => {
            expr::BitVector::sdiv(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Mods(lhs, rhs) => {
            expr::BitVector::smod(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::And(lhs, rhs) => {
            let lhs = translate_expr(lhs)?;
            let rhs = translate_expr(rhs)?;
            match lhs.sort() {
                expr::Sort::Boolean => expr::Boolean::and(lhs, rhs),
                _ => expr::BitVector::and(lhs, rhs),
            }
        }
        il::Expression::Or(lhs, rhs) => {
            let lhs = translate_expr(lhs)?;
            let rhs = translate_expr(rhs)?;
            match lhs.sort() {
                expr::Sort::Boolean => expr::Boolean::or(lhs, rhs),
                _ => expr::BitVector::or(lhs, rhs),
            }
        }
        il::Expression::Xor(lhs, rhs) => {
            let lhs = translate_expr(lhs)?;
            let rhs = translate_expr(rhs)?;
            match lhs.sort() {
                expr::Sort::Boolean => expr::Boolean::xor(lhs, rhs),
                _ => expr::BitVector::xor(lhs, rhs),
            }
        }
        il::Expression::Shl(lhs, rhs) => {
            expr::BitVector::shl(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Shr(lhs, rhs) => {
            expr::BitVector::lshr(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Cmpeq(lhs, rhs) => {
            expr::Expression::equal(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Cmpneq(lhs, rhs) => {
            expr::Expression::unequal(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Cmpltu(lhs, rhs) => {
            expr::BitVector::ult(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Cmplts(lhs, rhs) => {
            expr::BitVector::slt(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Zext(bits, src) => {
            let expr = translate_expr(src)?;
            match expr.sort() {
                expr::Sort::Boolean => expr::BitVector::from_boolean(*bits, expr),
                _ => expr::BitVector::zero_extend(*bits, expr),
            }
        }
        il::Expression::Sext(bits, src) => {
            expr::BitVector::sign_extend(*bits, translate_expr(src)?)
        }
        il::Expression::Trun(bits, src) => {
            let expr = translate_expr(src)?;
            if *bits > 1 {
                expr::BitVector::truncate(*bits, expr)
            } else {
                expr::BitVector::to_boolean(expr)
            }
        }
        il::Expression::Ite(cond, then, else_) => expr::Expression::ite(
            translate_expr(cond)?,
            translate_expr(then)?,
            translate_expr(else_)?,
        ),
    }
}

fn translate_scalar(scalar: &il::Scalar) -> Result<expr::Variable> {
    let sort = if scalar.bits() > 1 {
        expr::Sort::bit_vector(scalar.bits())
    } else {
        expr::Sort::boolean()
    };
    Ok(expr::Variable::new(scalar.name(), sort))
}
