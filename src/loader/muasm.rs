use crate::error::Result;
use crate::expr;
use crate::hir;
use crate::loader::{AssemblyInfo, FunctionInfo, Loader};
use muasm_parser::{ir, parser};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

const MAIN_ADDRESS: u64 = 0;
const MAIN_NAME: &str = "main";

pub struct MuasmLoader {
    file_path: PathBuf,
}

impl MuasmLoader {
    pub fn new(file_path: &Path) -> Self {
        Self {
            file_path: file_path.to_owned(),
        }
    }
}

impl Loader for MuasmLoader {
    fn assembly_info(&self) -> Result<AssemblyInfo> {
        let main = FunctionInfo {
            address: MAIN_ADDRESS,
            name: Some(MAIN_NAME.to_owned()),
        };

        Ok(AssemblyInfo {
            entry: MAIN_ADDRESS,
            functions: vec![main],
            memory_sections: Vec::default(),
        })
    }

    fn load_program(&self) -> Result<hir::Program> {
        let source = fs::read_to_string(&self.file_path)?;
        let ir = parser::parse_program(&source)?;

        let cfg = translate_ir_to_hir(&ir)?;
        let function = hir::Function::new(MAIN_ADDRESS, Some(MAIN_NAME.to_owned()), cfg);

        let mut program = hir::Program::new();
        program.insert_function(function)?;
        program.set_entry(hir::ProgramEntry::Address(MAIN_ADDRESS))?;

        Ok(program)
    }
}

fn translate_ir_to_hir(program: &ir::Program) -> Result<hir::ControlFlowGraph> {
    let mut cfg = hir::ControlFlowGraph::new();

    // Mapping from instruction address to instruction graph entry/exit
    let mut instruction_indices: BTreeMap<u64, (usize, usize)> = BTreeMap::new();

    // Mapping from instruction label to instruction address for target resolving
    let mut label_address: HashMap<&String, u64> = HashMap::new();
    for instruction in program.instructions() {
        if let Some(lbl) = instruction.label() {
            label_address.insert(lbl, instruction.address());
        }
    }

    let resolve_target_address = |target: &ir::Target| -> Result<u64> {
        match target {
            ir::Target::Location(addr) => Ok(*addr),
            ir::Target::Label(lbl) => label_address
                .get(lbl)
                .copied()
                .ok_or_else(|| format!("Unknown label {}", lbl).into()),
        }
    };

    // Add instruction graph for each instruction to CFG
    for instruction in program.instructions() {
        let mut instruction_graph = hir::ControlFlowGraph::new();

        match instruction.operation() {
            ir::Operation::Skip => semantics::skip(&mut instruction_graph),
            ir::Operation::Barrier => semantics::barrier(&mut instruction_graph),
            ir::Operation::Flush => semantics::flush(&mut instruction_graph),
            ir::Operation::Assignment { reg, expr } => {
                semantics::assignment(reg, expr, &mut instruction_graph)
            }
            ir::Operation::ConditionalAssignment { reg, expr, cond } => {
                semantics::conditional_assignment(reg, expr, cond, &mut instruction_graph)
            }
            ir::Operation::Load { reg, addr } => semantics::load(reg, addr, &mut instruction_graph),
            ir::Operation::Store { reg, addr } => {
                semantics::store(reg, addr, &mut instruction_graph)
            }
            ir::Operation::Jump { target } => {
                let target_address = resolve_target_address(target)?;
                semantics::jump(target_address, &mut instruction_graph)
            }
            ir::Operation::BranchIfZero { reg, target } => {
                let target_address = resolve_target_address(target)?;
                semantics::branch_if_zero(reg, target_address, &mut instruction_graph)
            }
        }?;

        let address = instruction.address();
        instruction_graph.set_address(Some(address));

        let block_renamings = cfg.insert(&instruction_graph)?;

        let entry = instruction_graph.entry().unwrap();
        let new_entry = block_renamings.get(&entry).unwrap();

        let exit = instruction_graph.exit().unwrap();
        let new_exit = block_renamings.get(&exit).unwrap();

        instruction_indices.insert(address, (*new_entry, *new_exit));
    }

    // Add a dedicated entry block.
    // This makes sure that the entry block has no predecessors.
    let entry = cfg.new_block().index();
    cfg.unconditional_edge(entry, 0)?;
    cfg.set_entry(entry)?;

    // Add a dedicated exit block.
    // This makes sure that there is only a single end block without successors.
    let exit = cfg.new_block().index();
    cfg.set_exit(exit)?;

    let resolve_edge_block_indices = |from_address: u64, to_address: u64| -> (usize, usize) {
        let (_, from_block) = instruction_indices.get(&from_address).unwrap();
        if let Some((to_block, _)) = instruction_indices.get(&to_address) {
            (*from_block, *to_block)
        } else {
            (*from_block, exit)
        }
    };

    // Add edges between instruction graphs
    for instruction in program.instructions() {
        let address = instruction.address();
        match instruction.operation() {
            ir::Operation::Jump { target } => {
                let target_address = resolve_target_address(target)?;
                let (from, to) = resolve_edge_block_indices(address, target_address);
                cfg.unconditional_edge(from, to)?;
            }
            ir::Operation::BranchIfZero { reg, target } => {
                let cond_not_taken =
                    expr::Expression::unequal(reg.to_hir_expr()?, 0.to_hir_expr()?)?;
                let (from, to) = resolve_edge_block_indices(address, address + 1);
                cfg.conditional_edge(from, to, cond_not_taken)?;

                let cond_taken = expr::Expression::equal(reg.to_hir_expr()?, 0.to_hir_expr()?)?;
                let target_address = resolve_target_address(target)?;
                let (from, to) = resolve_edge_block_indices(address, target_address);
                cfg.conditional_edge(from, to, cond_taken)?
                    .labels_mut()
                    .taken();
            }
            _ => {
                let (from, to) = resolve_edge_block_indices(address, address + 1);
                cfg.unconditional_edge(from, to)?;
            }
        };
    }

    // The only block without successors should be the exit block.
    assert!(cfg.graph().vertices_without_successors() == vec![cfg.exit_block()?]);

    cfg.simplify()?;

    Ok(cfg)
}

trait VariableBuilder {
    fn to_hir_variable(&self) -> expr::Variable;
}

trait ExpressionBuilder {
    fn to_hir_expr(&self) -> Result<expr::Expression>;
}

#[allow(clippy::use_self)]
impl ExpressionBuilder for u64 {
    fn to_hir_expr(&self) -> Result<expr::Expression> {
        Ok(expr::BitVector::word_constant(*self))
    }
}

impl VariableBuilder for ir::Register {
    fn to_hir_variable(&self) -> expr::Variable {
        expr::BitVector::word_variable(self.name())
    }
}

impl ExpressionBuilder for ir::Register {
    fn to_hir_expr(&self) -> Result<expr::Expression> {
        Ok(expr::BitVector::word_variable(self.name()).into())
    }
}

impl ExpressionBuilder for ir::Expression {
    fn to_hir_expr(&self) -> Result<expr::Expression> {
        match self {
            Self::NumberLiteral(lit) => Ok(expr::BitVector::word_constant(*lit)),
            Self::RegisterRef(reg) => reg.to_hir_expr(),
            Self::Unary { op, expr } => {
                let expr = expr.to_hir_expr()?;
                match op {
                    ir::UnaryOperator::Neg => expr::BitVector::neg(expr),
                    ir::UnaryOperator::Not => expr::BitVector::not(expr),
                    ir::UnaryOperator::SExt => expr::BitVector::sign_extend_to_word(expr),
                    ir::UnaryOperator::ZExt => expr::BitVector::zero_extend_to_word(expr),
                }
            }
            Self::Binary { op, lhs, rhs } => {
                let lhs = lhs.to_hir_expr()?;
                let rhs = rhs.to_hir_expr()?;
                match op {
                    ir::BinaryOperator::Add => expr::BitVector::add(lhs, rhs),
                    ir::BinaryOperator::Sub => expr::BitVector::sub(lhs, rhs),
                    ir::BinaryOperator::Mul => expr::BitVector::mul(lhs, rhs),
                    ir::BinaryOperator::UDiv => expr::BitVector::udiv(lhs, rhs),
                    ir::BinaryOperator::URem => expr::BitVector::urem(lhs, rhs),
                    ir::BinaryOperator::SRem => expr::BitVector::srem(lhs, rhs),
                    ir::BinaryOperator::SMod => expr::BitVector::smod(lhs, rhs),
                    ir::BinaryOperator::And => expr::BitVector::and(lhs, rhs),
                    ir::BinaryOperator::Or => expr::BitVector::or(lhs, rhs),
                    ir::BinaryOperator::Xor => expr::BitVector::xor(lhs, rhs),
                    ir::BinaryOperator::Shl => expr::BitVector::shl(lhs, rhs),
                    ir::BinaryOperator::AShr => expr::BitVector::ashr(lhs, rhs),
                    ir::BinaryOperator::LShr => expr::BitVector::lshr(lhs, rhs),
                    ir::BinaryOperator::ULe => {
                        expr::BitVector::word_from_boolean(expr::BitVector::ule(lhs, rhs)?)
                    }
                    ir::BinaryOperator::ULt => {
                        expr::BitVector::word_from_boolean(expr::BitVector::ult(lhs, rhs)?)
                    }
                    ir::BinaryOperator::UGe => {
                        expr::BitVector::word_from_boolean(expr::BitVector::uge(lhs, rhs)?)
                    }
                    ir::BinaryOperator::UGt => {
                        expr::BitVector::word_from_boolean(expr::BitVector::ugt(lhs, rhs)?)
                    }
                    ir::BinaryOperator::SLe => {
                        expr::BitVector::word_from_boolean(expr::BitVector::sle(lhs, rhs)?)
                    }
                    ir::BinaryOperator::SLt => {
                        expr::BitVector::word_from_boolean(expr::BitVector::slt(lhs, rhs)?)
                    }
                    ir::BinaryOperator::SGe => {
                        expr::BitVector::word_from_boolean(expr::BitVector::sge(lhs, rhs)?)
                    }
                    ir::BinaryOperator::SGt => {
                        expr::BitVector::word_from_boolean(expr::BitVector::sgt(lhs, rhs)?)
                    }
                    ir::BinaryOperator::r#Eq => {
                        expr::BitVector::word_from_boolean(expr::Expression::equal(lhs, rhs)?)
                    }
                    ir::BinaryOperator::Neq => {
                        expr::BitVector::word_from_boolean(expr::Expression::unequal(lhs, rhs)?)
                    }
                }
            }
            Self::Conditional { cond, then, r#else } => expr::Expression::ite(
                expr::BitVector::to_boolean(cond.to_hir_expr()?)?,
                then.to_hir_expr()?,
                r#else.to_hir_expr()?,
            ),
        }
    }
}

mod semantics {
    use super::*;

    pub fn skip(cfg: &mut hir::ControlFlowGraph) -> Result<()> {
        let block_index = {
            let block = cfg.new_block();

            block.skip();

            block.index()
        };

        cfg.set_entry(block_index)?;
        cfg.set_exit(block_index)?;

        Ok(())
    }

    pub fn barrier(cfg: &mut hir::ControlFlowGraph) -> Result<()> {
        let block_index = {
            let block = cfg.new_block();

            block.barrier();

            block.index()
        };

        cfg.set_entry(block_index)?;
        cfg.set_exit(block_index)?;

        Ok(())
    }

    pub fn flush(cfg: &mut hir::ControlFlowGraph) -> Result<()> {
        let block_index = {
            let block = cfg.new_block();

            let empty_cache =
                expr::Expression::constant(expr::CacheValue::empty().into(), expr::Sort::cache());
            block.assign(expr::Cache::variable(), empty_cache)?;

            block.index()
        };

        cfg.set_entry(block_index)?;
        cfg.set_exit(block_index)?;

        Ok(())
    }

    pub fn assignment(
        reg: &ir::Register,
        expr: &ir::Expression,
        cfg: &mut hir::ControlFlowGraph,
    ) -> Result<()> {
        let reg = reg.to_hir_variable();
        let expr = expr.to_hir_expr()?;

        let block_index = {
            let block = cfg.new_block();

            block.assign(reg, expr)?;

            block.index()
        };

        cfg.set_entry(block_index)?;
        cfg.set_exit(block_index)?;

        Ok(())
    }

    pub fn conditional_assignment(
        reg: &ir::Register,
        expr: &ir::Expression,
        cond: &ir::Expression,
        cfg: &mut hir::ControlFlowGraph,
    ) -> Result<()> {
        let var = reg.to_hir_variable();
        let cond_expr = expr::Expression::ite(
            expr::BitVector::to_boolean(cond.to_hir_expr()?)?,
            expr.to_hir_expr()?,
            reg.to_hir_expr()?,
        )?;

        let block_index = {
            let block = cfg.new_block();

            block.assign(var, cond_expr)?;

            block.index()
        };

        cfg.set_entry(block_index)?;
        cfg.set_exit(block_index)?;

        Ok(())
    }

    pub fn load(
        reg: &ir::Register,
        addr: &ir::Expression,
        cfg: &mut hir::ControlFlowGraph,
    ) -> Result<()> {
        let var = reg.to_hir_variable();
        let address = addr.to_hir_expr()?;

        let block_index = {
            let block = cfg.new_block();

            block.load(var, address)?;

            block.index()
        };

        cfg.set_entry(block_index)?;
        cfg.set_exit(block_index)?;

        Ok(())
    }

    pub fn store(
        reg: &ir::Register,
        addr: &ir::Expression,
        cfg: &mut hir::ControlFlowGraph,
    ) -> Result<()> {
        let expr = reg.to_hir_expr()?;
        let address = addr.to_hir_expr()?;

        let block_index = {
            let block = cfg.new_block();

            block.store(address, expr)?;

            block.index()
        };

        cfg.set_entry(block_index)?;
        cfg.set_exit(block_index)?;

        Ok(())
    }

    pub fn jump(target_address: u64, cfg: &mut hir::ControlFlowGraph) -> Result<()> {
        let target = target_address.to_hir_expr()?;

        let block_index = {
            let block = cfg.new_block();

            block.branch(target)?;

            block.index()
        };

        cfg.set_entry(block_index)?;
        cfg.set_exit(block_index)?;

        Ok(())
    }

    pub fn branch_if_zero(
        reg: &ir::Register,
        target_address: u64,
        cfg: &mut hir::ControlFlowGraph,
    ) -> Result<()> {
        let zero = 0.to_hir_expr()?;
        let cond = expr::Expression::equal(reg.to_hir_expr()?, zero)?;
        let target = target_address.to_hir_expr()?;

        let block_index = {
            let block = cfg.new_block();

            block.conditional_branch(cond, target)?;

            block.index()
        };

        cfg.set_entry(block_index)?;
        cfg.set_exit(block_index)?;

        Ok(())
    }
}
