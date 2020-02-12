use crate::error::Result;
use crate::lir;
use rsmt2::print::{Expr2Smt, Sort2Smt, Sym2Smt};
use rsmt2::{SmtRes, Solver};
use std::convert::TryInto;
use std::fs::File;
use std::path::Path;

pub fn encode_program(program: &lir::Program, debug_file_path: Option<&Path>) -> Result<()> {
    let parser = ();
    let mut solver = Solver::default_z3(parser)?;

    if let Some(path) = debug_file_path {
        let file = File::create(path)?;
        solver.tee(file)?;
    }

    let word_size = 64;
    let access_widths = vec![8, 16, 32, 64, 128];
    define_memory(&mut solver, word_size, &access_widths)?;
    define_cache(&mut solver, word_size, &access_widths)?;

    for block in program.block_graph().blocks() {
        encode_block(&mut solver, block)?;
    }

    Ok(())
}

fn encode_block<T>(solver: &mut Solver<T>, block: &lir::Block) -> Result<()> {
    solver.comment(&format!("Block 0x{:X}", block.index()))?;

    define_variable(
        solver,
        block.execution_condition_variable(),
        block.execution_condition(),
    )?;

    for node in block.nodes() {
        match node.operation() {
            lir::Operation::Let { var, expr } => {
                define_variable(solver, var, expr)?;
            }
            lir::Operation::Assert { .. } => bail!("not implemented"), // TODO
            lir::Operation::Assume { cond } => solver.assert(&cond)?,
        }
    }

    Ok(())
}

fn define_variable<T>(
    solver: &mut Solver<T>,
    variable: &lir::Variable,
    expr: &lir::Expression,
) -> SmtRes<()> {
    solver.define_const(variable, variable.sort(), &expr)
}

impl Expr2Smt<()> for lir::Expression {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, i: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        if self.operands().is_empty() {
            self.operator().expr_to_smt2(w, i)
        } else {
            write!(w, "(")?;
            self.operator().expr_to_smt2(w, i)?;
            for operand in self.operands() {
                write!(w, " ")?;
                operand.expr_to_smt2(w, i)?;
            }
            write!(w, ")")?;
            Ok(())
        }
    }
}

impl Expr2Smt<()> for lir::Operator {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, i: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            lir::Operator::Variable(v) => v.sym_to_smt2(w, i),
            lir::Operator::Constant(c) => c.expr_to_smt2(w, i),
            lir::Operator::Ite => {
                write!(w, "ite")?;
                Ok(())
            }
            lir::Operator::Equal => {
                write!(w, "=")?;
                Ok(())
            }
            lir::Operator::Boolean(op) => op.expr_to_smt2(w, i),
            lir::Operator::BitVector(op) => op.expr_to_smt2(w, i),
            lir::Operator::Array(op) => op.expr_to_smt2(w, i),
            lir::Operator::Set(op) => op.expr_to_smt2(w, i),
            lir::Operator::Memory(op) => op.expr_to_smt2(w, i),
            lir::Operator::Cache(op) => op.expr_to_smt2(w, i),
        }
    }
}

impl Expr2Smt<()> for lir::Boolean {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        let s = match self {
            lir::Boolean::Not => "not",
            lir::Boolean::Imply => "=>",
            lir::Boolean::And => "and",
            lir::Boolean::Or => "or",
            lir::Boolean::Xor => "xor",
        };
        write!(w, "{}", s)?;
        Ok(())
    }
}

impl Expr2Smt<()> for lir::BitVector {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        let s = match self {
            lir::BitVector::ToBoolean => "bv2bool".to_owned(), // FIXME
            lir::BitVector::FromBoolean(i) => format!("(bool2bv {})", i), // FIXME
            lir::BitVector::Concat => "concat".to_owned(),
            lir::BitVector::Extract(i, j) => format!("(_ extract {} {})", i, j),
            lir::BitVector::Truncate(i) => format!("(_ extract {} 0)", i - 1),
            lir::BitVector::Not => "bvnot".to_owned(),
            lir::BitVector::And => "bvand".to_owned(),
            lir::BitVector::Or => "bvor".to_owned(),
            lir::BitVector::Neg => "bvneg".to_owned(),
            lir::BitVector::Add => "bvadd".to_owned(),
            lir::BitVector::Mul => "bvmul".to_owned(),
            lir::BitVector::UDiv => "bvudiv".to_owned(),
            lir::BitVector::URem => "bvurem".to_owned(),
            lir::BitVector::Shl => "bvshl".to_owned(),
            lir::BitVector::LShr => "bvlshr".to_owned(),
            lir::BitVector::ULt => "bvult".to_owned(),
            lir::BitVector::Nand => "bvnand".to_owned(),
            lir::BitVector::Nor => "bvnor".to_owned(),
            lir::BitVector::Xor => "bvxor".to_owned(),
            lir::BitVector::Xnor => "bvxnor".to_owned(),
            lir::BitVector::Comp => "bvcomp".to_owned(),
            lir::BitVector::Sub => "bvsub".to_owned(),
            lir::BitVector::SDiv => "bvsdiv".to_owned(),
            lir::BitVector::SRem => "bvsrem".to_owned(),
            lir::BitVector::SMod => "bvsmod".to_owned(),
            lir::BitVector::UMod => "bvumod".to_owned(),
            lir::BitVector::AShr => "bvashr".to_owned(),
            lir::BitVector::Repeat(i) => format!("(_ repeat {})", i),
            lir::BitVector::ZeroExtend(i) => format!("(_ zero_extend {})", i),
            lir::BitVector::SignExtend(i) => format!("(_ sign_extend {})", i),
            lir::BitVector::RotateLeft(i) => format!("(_ rotate_left {})", i),
            lir::BitVector::RotateRight(i) => format!("(_ rotate_right {})", i),
            lir::BitVector::ULe => "bvule".to_owned(),
            lir::BitVector::UGt => "bvugt".to_owned(),
            lir::BitVector::UGe => "bvuge".to_owned(),
            lir::BitVector::SLt => "bvslt".to_owned(),
            lir::BitVector::SLe => "bvsle".to_owned(),
            lir::BitVector::SGt => "bvsgt".to_owned(),
            lir::BitVector::SGe => "bvsge".to_owned(),
        };
        write!(w, "{}", s)?;
        Ok(())
    }
}

impl Expr2Smt<()> for lir::Array {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            lir::Array::Select => write!(w, "select"),
            lir::Array::Store => write!(w, "store"),
        }?;
        Ok(())
    }
}

impl Expr2Smt<()> for lir::Set {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            lir::Set::Insert => write!(w, "(store set value true)"), // FIXME
            lir::Set::Remove => write!(w, "(store set value false)"), // FIXME
            lir::Set::Contains => write!(w, "(select set value)"),   // FIXME
        }?;
        Ok(())
    }
}

impl Expr2Smt<()> for lir::Memory {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            lir::Memory::Store(width) => write!(w, "store{}", width),
            lir::Memory::Load(width) => write!(w, "load{}", width),
        }?;
        Ok(())
    }
}

impl Expr2Smt<()> for lir::Cache {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            lir::Cache::Fetch(width) => write!(w, "fetch{}", width),
        }?;
        Ok(())
    }
}

impl Expr2Smt<()> for lir::Constant {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            lir::Constant::Boolean(value) => write!(w, "{}", value),
            lir::Constant::BitVector(bv) => write!(w, "(_ bv{} {})", bv.value(), bv.bits()),
        }?;
        Ok(())
    }
}

impl Sym2Smt<()> for lir::Variable {
    fn sym_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        write!(w, "{}", self.identifier())?;
        Ok(())
    }
}

impl Sort2Smt for lir::Sort {
    fn sort_to_smt2<Writer>(&self, w: &mut Writer) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            lir::Sort::Bool => write!(w, "Bool")?,
            lir::Sort::BitVector(width) => write!(w, "(_ BitVec {})", width)?,
            lir::Sort::Array { range, domain } => {
                write!(w, "(Array ")?;
                range.sort_to_smt2(w)?;
                write!(w, " ")?;
                domain.sort_to_smt2(w)?;
                write!(w, ")")?
            }
            lir::Sort::Set { range } => {
                write!(w, "(Array ")?;
                range.sort_to_smt2(w)?;
                write!(w, " Bool)")?
            }
            lir::Sort::Memory => write!(w, "Memory")?,
            lir::Sort::Cache => write!(w, "Cache")?,
        };
        Ok(())
    }
}

fn define_memory<T>(
    solver: &mut Solver<T>,
    address_bits: usize,
    access_widths: &[usize],
) -> Result<()> {
    let addr_sort = lir::Sort::bit_vector(address_bits);
    let mem_array_sort = lir::Sort::array(&addr_sort, &lir::Sort::bit_vector(8));

    // memory type
    solver.define_null_sort(&lir::Sort::memory(), &mem_array_sort)?;

    // memory load functions
    for width in access_widths {
        let mut array_selects = vec![];
        for byte in (0..(width / 8)).rev() {
            array_selects.push(lir::Array::select(
                lir::Variable::new("mem", mem_array_sort.clone()).into(),
                lir::BitVector::add(
                    lir::Variable::new("addr", addr_sort.clone()).into(),
                    lir::BitVector::constant(byte.try_into().unwrap(), address_bits).into(),
                )?,
            )?);
        }
        solver.define_fun(
            &format!("load{}", width),
            &[("mem", lir::Sort::memory()), ("addr", addr_sort.clone())],
            &lir::Sort::bit_vector(*width),
            &lir::BitVector::concat(&array_selects)?,
        )?;
    }

    // memory store functions
    for width in access_widths {
        let mut store_expr: lir::Expression =
            lir::Variable::new("mem", mem_array_sort.clone()).into();
        for byte in (0..(width / 8)).rev() {
            let bit_offset = byte * 8;
            store_expr = lir::Array::store(
                store_expr,
                lir::BitVector::add(
                    lir::Variable::new("addr", addr_sort.clone()).into(),
                    lir::BitVector::constant(byte.try_into().unwrap(), address_bits).into(),
                )?,
                lir::BitVector::extract(
                    bit_offset + 7,
                    bit_offset,
                    lir::Variable::new("val", lir::Sort::bit_vector(*width)).into(),
                )?,
            )?;
        }
        solver.define_fun(
            &format!("store{}", width),
            &[
                ("mem", lir::Sort::memory()),
                ("addr", addr_sort.clone()),
                ("val", lir::Sort::bit_vector(*width)),
            ],
            &lir::Sort::memory(),
            &store_expr,
        )?;
    }

    Ok(())
}

fn define_cache<T>(
    solver: &mut Solver<T>,
    address_bits: usize,
    access_widths: &[usize],
) -> Result<()> {
    let addr_sort = lir::Sort::bit_vector(address_bits);
    let cache_set_sort = lir::Sort::set(&addr_sort);

    // cache type
    solver.define_null_sort(&lir::Sort::cache(), &cache_set_sort)?;

    // cache fetch functions
    for width in access_widths {
        let mut insert_expr: lir::Expression =
            lir::Variable::new("cache", cache_set_sort.clone()).into();
        for byte in 0..(width / 8) {
            insert_expr = lir::Set::insert(
                insert_expr,
                lir::BitVector::add(
                    lir::Variable::new("addr", addr_sort.clone()).into(),
                    lir::BitVector::constant(byte.try_into().unwrap(), address_bits).into(),
                )?,
            )?;
        }
        solver.define_fun(
            &format!("fetch{}", width),
            &[("cache", lir::Sort::cache()), ("addr", addr_sort.clone())],
            &lir::Sort::cache(),
            &insert_expr,
        )?;
    }

    Ok(())
}
