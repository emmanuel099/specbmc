use crate::error::Result;
use crate::expr;
use crate::hir;
use crate::loader;
use crate::util::AbsoluteDifference;
use falcon::il;
use falcon::loader::{Elf, Loader};
use falcon::translator;
use std::collections::{BTreeMap, HashSet};
use std::ops::Deref;
use std::path::{Path, PathBuf};

#[rustfmt::skip]
const SPECULATION_BARRIERS: &[&str] = &[
    // Intel
    "mfence", "lfence", "cpuid",
];

pub struct FalconLoader {
    file_path: PathBuf,
}

impl FalconLoader {
    pub fn new(file_path: &Path) -> Self {
        Self {
            file_path: file_path.to_owned(),
        }
    }
}

impl loader::Loader for FalconLoader {
    fn assembly_info(&self) -> Result<loader::AssemblyInfo> {
        let elf = load_elf(&self.file_path)?;

        let mut functions = Vec::new();
        for f in elf.function_entries()? {
            functions.push(loader::FunctionInfo {
                address: f.address(),
                name: f.name().map(String::from),
            });
        }

        let mut memory_sections = Vec::new();
        for (&start_address, section) in elf.memory()?.sections() {
            let end_address = start_address + section.len() as u64;
            let permissions = translate_memory_permissions(section.permissions());
            memory_sections.push(loader::MemorySectionInfo {
                start_address,
                end_address,
                permissions,
            });
        }

        Ok(loader::AssemblyInfo {
            entry: elf.program_entry(),
            functions,
            memory_sections,
        })
    }

    fn load_program(&self) -> Result<hir::Program> {
        let elf = load_elf(&self.file_path)?;
        let program = lift_elf(&elf)?;

        let function_addresses: HashSet<u64> = program
            .functions()
            .into_iter()
            .map(il::Function::address)
            .collect();

        let mut hir_prog = hir::Program::new();

        for function in program.functions() {
            let mut hir_func = translate_function(function)?;
            reconstruct_calls(&mut hir_func, &function_addresses);
            hir_prog.insert_function(hir_func)?;
        }

        if hir_prog
            .set_entry(hir::ProgramEntry::Address(elf.program_entry()))
            .is_err()
        {
            println!("Failed to set ELF program entry, no default program entry will be set");
        }

        for (&start_address, section) in elf.memory()?.sections() {
            let end_address = start_address + section.len() as u64;
            let permissions = translate_memory_permissions(section.permissions());
            let mem_section = hir::MemorySection::new(start_address, end_address, permissions);
            hir_prog.memory_mut().insert_section(mem_section);
        }

        Ok(hir_prog)
    }
}

fn reconstruct_calls(func: &mut hir::Function, function_addresses: &HashSet<u64>) {
    let cfg = func.control_flow_graph_mut();

    let mut calls: BTreeMap<usize, Vec<usize>> = BTreeMap::default();

    for block in cfg.blocks() {
        for (index, inst) in block.instructions().iter().enumerate() {
            if let hir::Operation::Branch { target } = inst.operation() {
                if let Ok(target_address) = target.try_into() {
                    // Check if the branch instruction is a valid jump, by checking if its target
                    // address matches with the address of the subsequent instruction.
                    // The reason for this check is, that while some branch targets are valid function addresses,
                    // the target functions may be "invoked" using a jump instead of a call instruction.
                    let is_last_instruction = index == (block.instruction_count() - 1);
                    let successor_inst_addr = if is_last_instruction {
                        cfg.successor_indices(block.index())
                            .ok()
                            .and_then(|indices| {
                                if indices.len() != 1 {
                                    return None;
                                }
                                let successor_index = indices[0];
                                let successor = cfg.block(successor_index).unwrap();
                                successor.address()
                            })
                    } else {
                        block
                            .instruction(index + 1)
                            .and_then(hir::Instruction::address)
                    };

                    let is_valid_jump = if let Some(inst_addr) = successor_inst_addr {
                        target_address == inst_addr
                    } else {
                        false
                    };
                    if is_valid_jump {
                        continue;
                    }

                    if function_addresses.contains(&target_address) {
                        calls.entry(block.index()).or_default().push(index);
                    }
                }
            }
        }
    }

    for (block_index, inst_indices) in calls {
        let block = cfg.block_mut(block_index).unwrap();
        for inst_index in inst_indices {
            let inst = block.instruction_mut(inst_index).unwrap();
            if let hir::Operation::Branch { target } = inst.operation() {
                *inst.operation_mut() = hir::Operation::call(target.clone()).unwrap();
            }
        }
    }
}

fn load_elf(file_path: &Path) -> Result<Elf> {
    Ok(Elf::from_file(file_path)?)
}

fn lift_elf(elf: &Elf) -> Result<il::Program> {
    let options = translator::OptionsBuilder::default()
        .unsupported_are_intrinsics(true)
        .build();

    let result = elf.program_recursive_verbose(&options);
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

fn translate_memory_permissions(
    perms: falcon::memory::MemoryPermissions,
) -> hir::MemoryPermissions {
    let mut permissions = hir::MemoryPermissions::empty();
    if perms.contains(falcon::memory::MemoryPermissions::READ) {
        permissions |= hir::MemoryPermissions::READ;
    }
    if perms.contains(falcon::memory::MemoryPermissions::WRITE) {
        permissions |= hir::MemoryPermissions::WRITE;
    }
    if perms.contains(falcon::memory::MemoryPermissions::EXECUTE) {
        permissions |= hir::MemoryPermissions::EXECUTE;
    }
    permissions
}

fn translate_function(function: &il::Function) -> Result<hir::Function> {
    let cfg = translate_control_flow_graph(function.control_flow_graph())?;
    Ok(hir::Function::new(
        function.address(),
        Some(function.name()),
        cfg,
    ))
}

fn translate_control_flow_graph(src_cfg: &il::ControlFlowGraph) -> Result<hir::ControlFlowGraph> {
    let mut cfg = hir::ControlFlowGraph::new();

    for block in src_cfg.blocks() {
        cfg.add_block(translate_block(block)?)?;
    }

    for src_edge in src_cfg.edges() {
        match src_edge.condition() {
            Some(condition) => {
                let condition = translate_expr(condition)?;
                let edge = cfg.conditional_edge(src_edge.head(), src_edge.tail(), condition)?;
                if is_taken_edge(src_cfg, src_edge)? {
                    edge.labels_mut().taken();
                }
            }
            None => {
                cfg.unconditional_edge(src_edge.head(), src_edge.tail())?;
            }
        }
    }

    // Add a dedicated entry block.
    // This makes sure that the entry block has no predecessors.
    let start_blocks: Vec<usize> = cfg
        .graph()
        .vertices_without_predecessors()
        .iter()
        .map(|block| block.index())
        .collect();
    let entry = cfg.new_block().index();
    for start_block in start_blocks {
        cfg.unconditional_edge(entry, start_block)?;
    }
    cfg.set_entry(entry)?;

    // Add a dedicated exit block and connect all end blocks (= blocks without successor) to it.
    // This makes sure that there is only a single end block.
    let end_blocks: Vec<usize> = cfg
        .graph()
        .vertices_without_successors()
        .iter()
        .map(|block| block.index())
        .collect();
    let exit = cfg.new_block().index();
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
        let inst = translate_operation(&mut block, instruction.operation())?;
        inst.set_address(instruction.address());
    }

    label_helper_instructions(&mut block);

    Ok(block)
}

fn translate_operation<'a>(
    block: &'a mut hir::Block,
    operation: &il::Operation,
) -> Result<&'a mut hir::Instruction> {
    match operation {
        il::Operation::Assign { dst, src } => {
            let variable = translate_scalar(dst)?;
            let expr = translate_expr(src)?;
            let expr = maybe_cast(expr, variable.sort())?;
            block.assign(variable, expr)
        }
        il::Operation::Store { index, src } => {
            let address = translate_expr(index)?;
            let address = maybe_cast(address, &expr::Sort::word())?;
            let expr = translate_expr(src)?;
            block.store(address, expr)
        }
        il::Operation::Load { dst, index } => {
            let variable = translate_scalar(dst)?;
            let address = translate_expr(index)?;
            let address = maybe_cast(address, &expr::Sort::word())?;
            block.load(variable, address)
        }
        il::Operation::Branch { target } => {
            let target = translate_expr(target)?;
            block.branch(target)
        }
        il::Operation::Conditional {
            condition,
            operation,
        } => {
            let condition = translate_expr(condition)?;
            match operation.deref() {
                il::Operation::Assign { dst, src } => {
                    let variable = translate_scalar(dst)?;
                    let expr = translate_expr(src)?;
                    let expr = maybe_cast(expr, variable.sort())?;
                    // Only assign new value if condition holds, otherwise assign identity
                    let expr = expr::Expression::ite(condition, expr, variable.clone().into())?;
                    block.assign(variable, expr)
                }
                il::Operation::Branch { target } => {
                    let target = translate_expr(target)?;
                    block.conditional_branch(condition, target)
                }
                _ => unimplemented!(
                    "Translation for conditional {:#?} is not implemented",
                    operation
                ),
            }
        }
        il::Operation::Intrinsic { intrinsic } => {
            if SPECULATION_BARRIERS.contains(&intrinsic.mnemonic()) {
                Ok(block.barrier())
            } else {
                Ok(block.skip())
            }
        }
        il::Operation::Nop { placeholder } => {
            if let Some(operation) = placeholder {
                translate_operation(block, operation)
            } else {
                Ok(block.skip())
            }
        }
    }
}

/// If multiple consecutive instructions have the same address, then all but the first
/// instruction will be labeled as helper instructions.
///
/// This is necessary because Falcon may add multiple IL instructions for a single assembly instruction,
/// e.g. to encode the status register modifications.
fn label_helper_instructions(block: &mut hir::Block) {
    let mut last_address: Option<u64> = None;
    for inst in block.instructions_mut() {
        if last_address == inst.address() {
            inst.labels_mut().helper();
        }
        last_address = inst.address();
    }
}

fn maybe_cast(expr: expr::Expression, target_sort: &expr::Sort) -> Result<expr::Expression> {
    match (target_sort, expr.sort()) {
        (expr::Sort::Boolean, expr::Sort::BitVector(1)) => expr::BitVector::to_boolean(expr),
        (expr::Sort::BitVector(bit_width), expr::Sort::Boolean) => {
            expr::BitVector::from_boolean(*bit_width, expr)
        }
        (expr::Sort::BitVector(target_bit_width), expr::Sort::BitVector(src_bit_width)) => {
            if src_bit_width < target_bit_width {
                expr::BitVector::zero_extend_abs(*target_bit_width, expr)
            } else {
                Ok(expr)
            }
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
        il::Expression::AShr(lhs, rhs) => {
            expr::BitVector::ashr(translate_expr(lhs)?, translate_expr(rhs)?)
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
            let expr = translate_expr(src)?;
            let expr = match expr.sort() {
                expr::Sort::Boolean => expr::BitVector::from_boolean(1, expr)?,
                _ => expr,
            };
            expr::BitVector::sign_extend_abs(*bits, expr)
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

/// Try to determine if the given edge is a "taken" edge.
/// This function assumes that the taken edge is the edge with the greatest distance from
/// the last instruction of the head block to first instruction of the tail block.
fn is_taken_edge(cfg: &il::ControlFlowGraph, edge: &il::Edge) -> Result<bool> {
    let tail_address = match cfg.block(edge.tail())?.address() {
        Some(address) => address,
        None => {
            return Ok(false);
        }
    };

    let last_inst = match cfg.block(edge.head())?.instructions().last() {
        Some(inst) => inst,
        None => {
            return Ok(false);
        }
    };
    let start_address = match last_inst.address() {
        Some(address) => address,
        None => {
            return Ok(false);
        }
    };

    let distance = tail_address.abs_diff(start_address);

    for out_edge in cfg.edges_out(edge.head())? {
        if let Some(target_address) = cfg.block(out_edge.tail())?.address() {
            if target_address.abs_diff(start_address) > distance {
                return Ok(false);
            }
        }
    }

    Ok(true)
}
