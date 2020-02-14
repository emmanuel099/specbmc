use crate::error::Result;
use crate::expr;
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

    for node in program.nodes() {
        match node {
            lir::Node::Comment(text) => solver.comment(&text)?,
            lir::Node::Let { var, expr } => define_variable(&mut solver, var, expr)?,
            lir::Node::Assert { .. } => bail!("not implemented"), // TODO
            lir::Node::Assume { cond } => solver.assert(&cond)?,
        }
    }

    Ok(())
}

fn define_variable<T>(
    solver: &mut Solver<T>,
    variable: &expr::Variable,
    expr: &expr::Expression,
) -> SmtRes<()> {
    solver.define_const(variable, variable.sort(), &expr)
}

impl Expr2Smt<()> for expr::Expression {
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

impl Expr2Smt<()> for expr::Operator {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, i: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            expr::Operator::Variable(v) => v.sym_to_smt2(w, i),
            expr::Operator::Ite => {
                write!(w, "ite")?;
                Ok(())
            }
            expr::Operator::Equal => {
                write!(w, "=")?;
                Ok(())
            }
            expr::Operator::Boolean(op) => op.expr_to_smt2(w, i),
            expr::Operator::Integer(op) => op.expr_to_smt2(w, i),
            expr::Operator::BitVector(op) => op.expr_to_smt2(w, i),
            expr::Operator::Array(op) => op.expr_to_smt2(w, i),
            expr::Operator::Set(op) => op.expr_to_smt2(w, i),
            expr::Operator::Memory(op) => op.expr_to_smt2(w, i),
            expr::Operator::Cache(op) => op.expr_to_smt2(w, i),
        }
    }
}

impl Expr2Smt<()> for expr::Boolean {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        let s = match self {
            expr::Boolean::True => "true",
            expr::Boolean::False => "false",
            expr::Boolean::Not => "not",
            expr::Boolean::Imply => "=>",
            expr::Boolean::And => "and",
            expr::Boolean::Or => "or",
            expr::Boolean::Xor => "xor",
        };
        write!(w, "{}", s)?;
        Ok(())
    }
}

impl Expr2Smt<()> for expr::Integer {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::Constant(value) => write!(w, "{}", value),
            Self::Lt => write!(w, "<"),
            Self::Gt => write!(w, ">"),
            Self::Lte => write!(w, "<="),
            Self::Gte => write!(w, ">="),
            Self::Mod => write!(w, "mod"),
            Self::Div => write!(w, "div"),
            Self::Abs => write!(w, "abs"),
            Self::Mul => write!(w, "*"),
            Self::Add => write!(w, "+"),
            Self::Sub | Self::Neg => write!(w, "-"),
        }?;
        Ok(())
    }
}

impl Expr2Smt<()> for expr::BitVector {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        let s = match self {
            expr::BitVector::Constant(bv) => format!("(_ bv{} {})", bv.value(), bv.bits()),
            expr::BitVector::ToBoolean => "bv2bool".to_owned(), // FIXME
            expr::BitVector::FromBoolean(i) => format!("(bool2bv {})", i), // FIXME
            expr::BitVector::Concat => "concat".to_owned(),
            expr::BitVector::Extract(i, j) => format!("(_ extract {} {})", i, j),
            expr::BitVector::Truncate(i) => format!("(_ extract {} 0)", i - 1),
            expr::BitVector::Not => "bvnot".to_owned(),
            expr::BitVector::And => "bvand".to_owned(),
            expr::BitVector::Or => "bvor".to_owned(),
            expr::BitVector::Neg => "bvneg".to_owned(),
            expr::BitVector::Add => "bvadd".to_owned(),
            expr::BitVector::Mul => "bvmul".to_owned(),
            expr::BitVector::UDiv => "bvudiv".to_owned(),
            expr::BitVector::URem => "bvurem".to_owned(),
            expr::BitVector::Shl => "bvshl".to_owned(),
            expr::BitVector::LShr => "bvlshr".to_owned(),
            expr::BitVector::ULt => "bvult".to_owned(),
            expr::BitVector::Nand => "bvnand".to_owned(),
            expr::BitVector::Nor => "bvnor".to_owned(),
            expr::BitVector::Xor => "bvxor".to_owned(),
            expr::BitVector::Xnor => "bvxnor".to_owned(),
            expr::BitVector::Comp => "bvcomp".to_owned(),
            expr::BitVector::Sub => "bvsub".to_owned(),
            expr::BitVector::SDiv => "bvsdiv".to_owned(),
            expr::BitVector::SRem => "bvsrem".to_owned(),
            expr::BitVector::SMod => "bvsmod".to_owned(),
            expr::BitVector::UMod => "bvumod".to_owned(),
            expr::BitVector::AShr => "bvashr".to_owned(),
            expr::BitVector::Repeat(i) => format!("(_ repeat {})", i),
            expr::BitVector::ZeroExtend(i) => format!("(_ zero_extend {})", i),
            expr::BitVector::SignExtend(i) => format!("(_ sign_extend {})", i),
            expr::BitVector::RotateLeft(i) => format!("(_ rotate_left {})", i),
            expr::BitVector::RotateRight(i) => format!("(_ rotate_right {})", i),
            expr::BitVector::ULe => "bvule".to_owned(),
            expr::BitVector::UGt => "bvugt".to_owned(),
            expr::BitVector::UGe => "bvuge".to_owned(),
            expr::BitVector::SLt => "bvslt".to_owned(),
            expr::BitVector::SLe => "bvsle".to_owned(),
            expr::BitVector::SGt => "bvsgt".to_owned(),
            expr::BitVector::SGe => "bvsge".to_owned(),
        };
        write!(w, "{}", s)?;
        Ok(())
    }
}

impl Expr2Smt<()> for expr::Array {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            expr::Array::Select => write!(w, "select"),
            expr::Array::Store => write!(w, "store"),
        }?;
        Ok(())
    }
}

impl Expr2Smt<()> for expr::Set {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            expr::Set::Insert => write!(w, "(store set value true)"), // FIXME
            expr::Set::Remove => write!(w, "(store set value false)"), // FIXME
            expr::Set::Contains => write!(w, "(select set value)"),   // FIXME
        }?;
        Ok(())
    }
}

impl Expr2Smt<()> for expr::Memory {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            expr::Memory::Store(width) => write!(w, "store{}", width),
            expr::Memory::Load(width) => write!(w, "load{}", width),
        }?;
        Ok(())
    }
}

impl Expr2Smt<()> for expr::Cache {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            expr::Cache::Fetch(width) => write!(w, "fetch{}", width),
        }?;
        Ok(())
    }
}

impl Sym2Smt<()> for expr::Variable {
    fn sym_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        write!(w, "{}", self.identifier())?;
        Ok(())
    }
}

impl Sort2Smt for expr::Sort {
    fn sort_to_smt2<Writer>(&self, w: &mut Writer) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            expr::Sort::Boolean => write!(w, "Bool")?,
            expr::Sort::Integer => write!(w, "Integer")?,
            expr::Sort::BitVector(width) => write!(w, "(_ BitVec {})", width)?,
            expr::Sort::Array { range, domain } => {
                write!(w, "(Array ")?;
                range.sort_to_smt2(w)?;
                write!(w, " ")?;
                domain.sort_to_smt2(w)?;
                write!(w, ")")?
            }
            expr::Sort::Set { range } => {
                write!(w, "(Array ")?;
                range.sort_to_smt2(w)?;
                write!(w, " Bool)")?
            }
            expr::Sort::Memory => write!(w, "Memory")?,
            expr::Sort::Cache => write!(w, "Cache")?,
        };
        Ok(())
    }
}

fn define_memory<T>(
    solver: &mut Solver<T>,
    address_bits: usize,
    access_widths: &[usize],
) -> Result<()> {
    let addr_sort = expr::Sort::bit_vector(address_bits);
    let mem_array_sort = expr::Sort::array(&addr_sort, &expr::Sort::bit_vector(8));

    // memory type
    solver.define_null_sort(&expr::Sort::memory(), &mem_array_sort)?;

    // memory load functions
    for width in access_widths {
        let mut array_selects = vec![];
        for byte in (0..(width / 8)).rev() {
            array_selects.push(expr::Array::select(
                expr::Variable::new("mem", mem_array_sort.clone()).into(),
                expr::BitVector::add(
                    expr::Variable::new("addr", addr_sort.clone()).into(),
                    expr::BitVector::constant(byte.try_into().unwrap(), address_bits),
                )?,
            )?);
        }
        solver.define_fun(
            &format!("load{}", width),
            &[("mem", expr::Sort::memory()), ("addr", addr_sort.clone())],
            &expr::Sort::bit_vector(*width),
            &expr::BitVector::concat(&array_selects)?,
        )?;
    }

    // memory store functions
    for width in access_widths {
        let mut store_expr: expr::Expression =
            expr::Variable::new("mem", mem_array_sort.clone()).into();
        for byte in (0..(width / 8)).rev() {
            let bit_offset = byte * 8;
            store_expr = expr::Array::store(
                store_expr,
                expr::BitVector::add(
                    expr::Variable::new("addr", addr_sort.clone()).into(),
                    expr::BitVector::constant(byte.try_into().unwrap(), address_bits),
                )?,
                expr::BitVector::extract(
                    bit_offset + 7,
                    bit_offset,
                    expr::Variable::new("val", expr::Sort::bit_vector(*width)).into(),
                )?,
            )?;
        }
        solver.define_fun(
            &format!("store{}", width),
            &[
                ("mem", expr::Sort::memory()),
                ("addr", addr_sort.clone()),
                ("val", expr::Sort::bit_vector(*width)),
            ],
            &expr::Sort::memory(),
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
    let addr_sort = expr::Sort::bit_vector(address_bits);
    let cache_set_sort = expr::Sort::set(&addr_sort);

    // cache type
    solver.define_null_sort(&expr::Sort::cache(), &cache_set_sort)?;

    // cache fetch functions
    for width in access_widths {
        let mut insert_expr: expr::Expression =
            expr::Variable::new("cache", cache_set_sort.clone()).into();
        for byte in 0..(width / 8) {
            insert_expr = expr::Set::insert(
                insert_expr,
                expr::BitVector::add(
                    expr::Variable::new("addr", addr_sort.clone()).into(),
                    expr::BitVector::constant(byte.try_into().unwrap(), address_bits),
                )?,
            )?;
        }
        solver.define_fun(
            &format!("fetch{}", width),
            &[("cache", expr::Sort::cache()), ("addr", addr_sort.clone())],
            &expr::Sort::cache(),
            &insert_expr,
        )?;
    }

    Ok(())
}
