use crate::error::Result;
use crate::expr;
use crate::hir;
use falcon::il;
use falcon::loader::{Elf, Loader};
use std::path::Path;

#[rustfmt::skip]
const SPECULATION_BARRIERS: &'static [&'static str] = &[
    // Intel
    "mfence", "lfence", "cpuid",
];

pub fn load_program(file_path: &Path, function_name_or_id: Option<&str>) -> Result<hir::Program> {
    let program = load_elf(file_path)?;

    if let Some(name_or_id) = function_name_or_id {
        let function = match name_or_id.trim().parse::<usize>() {
            Ok(id) => program.function(id),
            Err(_) => program.function_by_name(name_or_id),
        };

        let function =
            function.ok_or_else(|| format!("Function '{}' could not be found", name_or_id))?;
        translate_function(function)
    } else {
        Err("Falcon loader is currently limited to a single function".into())
    }
}

fn load_elf(file_path: &Path) -> Result<il::Program> {
    let elf = Elf::from_file(file_path)?;
    let result = elf.program_recursive_verbose();
    match result {
        Ok((program, lifting_errors)) => {
            lifting_errors.iter().for_each(|(func, err)| {
                println!(
                    "Lifting {} failed with: {}",
                    func.name().unwrap_or("unknown"),
                    err
                )
            });
            Ok(program)
        }
        Err(_) => Err("Failed to load ELF file!".into()),
    }
}

fn translate_function(function: &il::Function) -> Result<hir::Program> {
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

    // Add a dedicated entry block.
    // This makes sure that the entry block has no predecessors.
    let src_entry = src_cfg.entry().ok_or("CFG entry must be set")?;
    let entry = cfg.new_block()?.index();
    cfg.unconditional_edge(entry, src_entry)?;
    cfg.set_entry(entry)?;

    // Add a dedicated exit block and connect all end blocks (= blocks without successor) to it.
    // This makes sure that there is only a single end block.
    let end_blocks: Vec<usize> = cfg
        .graph()
        .vertices_without_successors()
        .iter()
        .map(|block| block.index())
        .collect();
    let exit = cfg.new_block()?.index();
    for end_block in end_blocks {
        cfg.unconditional_edge(end_block, exit)?;
    }
    cfg.set_exit(exit)?;

    cfg.simplify()?;

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
                let inst = block.assign(variable, expr)?;
                inst.set_address(instruction.address());
            }
            il::Operation::Store { index, src } => {
                let address = translate_expr(index)?;
                let expr = translate_expr(src)?;
                let inst = block.store(address, expr)?;
                inst.set_address(instruction.address());
            }
            il::Operation::Load { dst, index } => {
                let variable = translate_scalar(dst)?;
                let address = translate_expr(index)?;
                let inst = block.load(variable, address)?;
                inst.set_address(instruction.address());
            }
            il::Operation::Branch { target } => {
                let target = translate_expr(target)?;
                let inst = block.branch(target)?;
                inst.set_address(instruction.address());
            }
            il::Operation::ConditionalBranch { condition, target } => {
                let condition = translate_expr(condition)?;
                let target = translate_expr(target)?;
                let inst = block.conditional_branch(condition, target)?;
                inst.set_address(instruction.address());
            }
            il::Operation::Intrinsic { intrinsic } => {
                if SPECULATION_BARRIERS.contains(&intrinsic.mnemonic()) {
                    let inst = block.barrier();
                    inst.set_address(instruction.address());
                }
            }
            il::Operation::Nop => (),
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
                expr::BitVector::constant(constant.clone())
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
                _ => expr::BitVector::zero_extend_abs(*bits, expr),
            }
        }
        il::Expression::Sext(bits, src) => {
            expr::BitVector::sign_extend_abs(*bits, translate_expr(src)?)
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
