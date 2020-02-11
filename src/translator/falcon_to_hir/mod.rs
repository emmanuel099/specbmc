use crate::error::Result;
use crate::hir;
use crate::lir;
use falcon::il;

pub fn translate_function(function: &il::Function) -> Result<hir::Program> {
    let cfg = translate_control_flow_graph(function.control_flow_graph())?;

    Ok(hir::Program::new(cfg))
}

fn translate_control_flow_graph(src_cfg: &il::ControlFlowGraph) -> Result<hir::ControlFlowGraph> {
    let entry = src_cfg.entry().ok_or("CFG entry must be set")?;

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

    if src_cfg.predecessor_indices(entry)?.is_empty() {
        cfg.set_entry(entry)?;
    } else {
        // add a dedicated entry block, because the original entry block contains input edges
        let new_entry = cfg.new_block()?.index();
        cfg.unconditional_edge(new_entry, entry)?;
        cfg.set_entry(new_entry)?;
    }

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
                block.assign(variable, expr);
            }
            il::Operation::Store { index, src } => {
                let memory = lir::Memory::variable(64); // FIXME get address width from Falcon
                let address = translate_expr(index)?;
                let expr = translate_expr(src)?;
                block.store(memory, address, expr);
            }
            il::Operation::Load { dst, index } => {
                let variable = translate_scalar(dst)?;
                let memory = lir::Memory::variable(64); // FIXME get address width from Falcon
                let address = translate_expr(index)?;
                block.load(variable, memory, address);
            }
            il::Operation::Branch { target } => {
                let target = translate_expr(target)?;
                block.branch(target);
            }
            il::Operation::Intrinsic { intrinsic } => match intrinsic.mnemonic() {
                "mfence" | "lfence" | "spbarr" => block.barrier(),
                _ => continue,
            },
            il::Operation::Nop => continue,
        }
    }

    Ok(block)
}

fn maybe_cast(expr: lir::Expression, target_sort: &lir::Sort) -> Result<lir::Expression> {
    match (target_sort, expr.sort()) {
        (lir::Sort::Bool, lir::Sort::BitVector(1)) => lir::BitVector::to_boolean(expr),
        (lir::Sort::BitVector(bit_width), lir::Sort::Bool) => {
            lir::BitVector::from_boolean(*bit_width, expr)
        }
        _ => Ok(expr),
    }
}

fn translate_expr(expr: &il::Expression) -> Result<lir::Expression> {
    match expr {
        il::Expression::Scalar(scalar) => {
            let var = translate_scalar(scalar)?;
            Ok(var.into())
        }
        il::Expression::Constant(constant) => {
            let constant = if constant.bits() > 1 {
                lir::BitVector::constant_big(constant.value().clone(), constant.bits())
            } else {
                lir::Boolean::constant(constant.is_one())
            };
            Ok(constant.into())
        }
        il::Expression::Add(lhs, rhs) => {
            lir::BitVector::add(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Sub(lhs, rhs) => {
            lir::BitVector::sub(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Mul(lhs, rhs) => {
            lir::BitVector::mul(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Divu(lhs, rhs) => {
            lir::BitVector::udiv(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Modu(lhs, rhs) => {
            lir::BitVector::umod(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Divs(lhs, rhs) => {
            lir::BitVector::sdiv(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Mods(lhs, rhs) => {
            lir::BitVector::smod(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::And(lhs, rhs) => {
            let lhs = translate_expr(lhs)?;
            let rhs = translate_expr(rhs)?;
            match lhs.sort() {
                lir::Sort::Bool => lir::Boolean::and(lhs, rhs),
                _ => lir::BitVector::and(lhs, rhs),
            }
        }
        il::Expression::Or(lhs, rhs) => {
            let lhs = translate_expr(lhs)?;
            let rhs = translate_expr(rhs)?;
            match lhs.sort() {
                lir::Sort::Bool => lir::Boolean::or(lhs, rhs),
                _ => lir::BitVector::or(lhs, rhs),
            }
        }
        il::Expression::Xor(lhs, rhs) => {
            let lhs = translate_expr(lhs)?;
            let rhs = translate_expr(rhs)?;
            match lhs.sort() {
                lir::Sort::Bool => lir::Boolean::xor(lhs, rhs),
                _ => lir::BitVector::xor(lhs, rhs),
            }
        }
        il::Expression::Shl(lhs, rhs) => {
            lir::BitVector::shl(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Shr(lhs, rhs) => {
            lir::BitVector::lshr(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Cmpeq(lhs, rhs) => {
            lir::Expression::equal(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Cmpneq(lhs, rhs) => {
            lir::Expression::unequal(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Cmpltu(lhs, rhs) => {
            lir::BitVector::ult(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Cmplts(lhs, rhs) => {
            lir::BitVector::slt(translate_expr(lhs)?, translate_expr(rhs)?)
        }
        il::Expression::Zext(bits, src) => {
            let expr = translate_expr(src)?;
            match expr.sort() {
                lir::Sort::Bool => lir::BitVector::from_boolean(*bits, expr),
                _ => lir::BitVector::zero_extend(*bits, expr),
            }
        }
        il::Expression::Sext(bits, src) => lir::BitVector::sign_extend(*bits, translate_expr(src)?),
        il::Expression::Trun(bits, src) => lir::BitVector::truncate(*bits, translate_expr(src)?),
        il::Expression::Ite(cond, then, else_) => lir::Expression::ite(
            translate_expr(cond)?,
            translate_expr(then)?,
            translate_expr(else_)?,
        ),
    }
}

fn translate_scalar(scalar: &il::Scalar) -> Result<lir::Variable> {
    let sort = if scalar.bits() > 1 {
        lir::Sort::BitVector(scalar.bits())
    } else {
        lir::Sort::Bool
    };
    Ok(lir::Variable::new(scalar.name(), sort))
}
