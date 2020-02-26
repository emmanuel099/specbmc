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

    define_predictor(&mut solver, word_size)?;
    define_memory(&mut solver, word_size, &access_widths)?;
    define_cache(&mut solver, word_size, &access_widths)?;
    define_btb(&mut solver, word_size)?;
    define_pht(&mut solver, word_size)?;

    let mut assertions: Vec<expr::Expression> = Vec::new();

    for node in program.nodes() {
        match node {
            lir::Node::Comment(text) => solver.comment(&text)?,
            lir::Node::Let { var, expr } => define_variable(&mut solver, var, expr)?,
            lir::Node::Assert { cond } => {
                let name = format!("_assertion{}", assertions.len());
                let assertion = expr::Variable::new(name, expr::Sort::boolean());
                define_variable(&mut solver, &assertion, &cond)?;
                assertions.push(assertion.into())
            }
            lir::Node::Assume { cond } => solver.assert(&cond)?,
        }
    }

    solver.assert(&expr::Boolean::not(expr::Boolean::conjunction(
        &assertions,
    )?)?)?;

    Ok(())
}

fn define_variable<T>(
    solver: &mut Solver<T>,
    variable: &expr::Variable,
    expr: &expr::Expression,
) -> SmtRes<()> {
    if expr.is_nondet() {
        solver.declare_const(variable, variable.sort())
    } else {
        solver.define_const(variable, variable.sort(), &expr)
    }
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
            Self::Variable(v) => v.sym_to_smt2(w, i),
            Self::Ite => {
                write!(w, "ite")?;
                Ok(())
            }
            Self::Equal => {
                write!(w, "=")?;
                Ok(())
            }
            Self::Nondet => {
                // Nondeterministic assignments are handled in define_variable,
                // everywhere else `nondet` is unexpected.
                Err("Incorrect use of nondet()".into())
            }
            Self::Boolean(op) => op.expr_to_smt2(w, i),
            Self::Integer(op) => op.expr_to_smt2(w, i),
            Self::BitVector(op) => op.expr_to_smt2(w, i),
            Self::Array(op) => op.expr_to_smt2(w, i),
            Self::Set(op) => op.expr_to_smt2(w, i),
            Self::Memory(op) => op.expr_to_smt2(w, i),
            Self::Predictor(op) => op.expr_to_smt2(w, i),
            Self::Cache(op) => op.expr_to_smt2(w, i),
            Self::BranchTargetBuffer(op) => op.expr_to_smt2(w, i),
            Self::PatternHistoryTable(op) => op.expr_to_smt2(w, i),
        }
    }
}

impl Expr2Smt<()> for expr::Boolean {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::True => write!(w, "true")?,
            Self::False => write!(w, "false")?,
            Self::Not => write!(w, "not")?,
            Self::Imply => write!(w, "=>")?,
            Self::And => write!(w, "and")?,
            Self::Or => write!(w, "or")?,
            Self::Xor => write!(w, "xor")?,
        };
        Ok(())
    }
}

impl Expr2Smt<()> for expr::Integer {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::Constant(value) => write!(w, "{}", value)?,
            Self::Lt => write!(w, "<")?,
            Self::Gt => write!(w, ">")?,
            Self::Lte => write!(w, "<=")?,
            Self::Gte => write!(w, ">=")?,
            Self::Mod => write!(w, "mod")?,
            Self::Div => write!(w, "div")?,
            Self::Abs => write!(w, "abs")?,
            Self::Mul => write!(w, "*")?,
            Self::Add => write!(w, "+")?,
            Self::Sub | Self::Neg => write!(w, "-")?,
        };
        Ok(())
    }
}

impl Expr2Smt<()> for expr::BitVector {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::Constant(bv) => write!(w, "(_ bv{} {})", bv.value(), bv.bits())?,
            Self::ToBoolean => write!(w, "bv2bool")?, // FIXME
            Self::FromBoolean(i) => write!(w, "(bool2bv {})", i)?, // FIXME
            Self::Concat => write!(w, "concat")?,
            Self::Extract(i, j) => write!(w, "(_ extract {} {})", i, j)?,
            Self::Truncate(i) => write!(w, "(_ extract {} 0)", i - 1)?,
            Self::Not => write!(w, "bvnot")?,
            Self::And => write!(w, "bvand")?,
            Self::Or => write!(w, "bvor")?,
            Self::Neg => write!(w, "bvneg")?,
            Self::Add => write!(w, "bvadd")?,
            Self::Mul => write!(w, "bvmul")?,
            Self::UDiv => write!(w, "bvudiv")?,
            Self::URem => write!(w, "bvurem")?,
            Self::Shl => write!(w, "bvshl")?,
            Self::LShr => write!(w, "bvlshr")?,
            Self::ULt => write!(w, "bvult")?,
            Self::Nand => write!(w, "bvnand")?,
            Self::Nor => write!(w, "bvnor")?,
            Self::Xor => write!(w, "bvxor")?,
            Self::Xnor => write!(w, "bvxnor")?,
            Self::Comp => write!(w, "bvcomp")?,
            Self::Sub => write!(w, "bvsub")?,
            Self::SDiv => write!(w, "bvsdiv")?,
            Self::SRem => write!(w, "bvsrem")?,
            Self::SMod => write!(w, "bvsmod")?,
            Self::UMod => write!(w, "bvumod")?,
            Self::AShr => write!(w, "bvashr")?,
            Self::Repeat(i) => write!(w, "(_ repeat {})", i)?,
            Self::ZeroExtend(i) => write!(w, "(_ zero_extend {})", i)?,
            Self::SignExtend(i) => write!(w, "(_ sign_extend {})", i)?,
            Self::RotateLeft(i) => write!(w, "(_ rotate_left {})", i)?,
            Self::RotateRight(i) => write!(w, "(_ rotate_right {})", i)?,
            Self::ULe => write!(w, "bvule")?,
            Self::UGt => write!(w, "bvugt")?,
            Self::UGe => write!(w, "bvuge")?,
            Self::SLt => write!(w, "bvslt")?,
            Self::SLe => write!(w, "bvsle")?,
            Self::SGt => write!(w, "bvsgt")?,
            Self::SGe => write!(w, "bvsge")?,
        };
        Ok(())
    }
}

impl Expr2Smt<()> for expr::Array {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::Select => write!(w, "select")?,
            Self::Store => write!(w, "store")?,
        };
        Ok(())
    }
}

impl Expr2Smt<()> for expr::Set {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::Insert => write!(w, "(store set value true)")?, // FIXME
            Self::Remove => write!(w, "(store set value false)")?, // FIXME
            Self::Contains => write!(w, "(select set value)")?,   // FIXME
        };
        Ok(())
    }
}

impl Expr2Smt<()> for expr::Memory {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::Store(width) => write!(w, "store{}", width)?,
            Self::Load(width) => write!(w, "load{}", width)?,
        };
        Ok(())
    }
}

impl Expr2Smt<()> for expr::Predictor {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::TransientStart => write!(w, "transient-start")?,
            Self::MisPredict => write!(w, "mis-predict")?,
            Self::SpeculationWindow => write!(w, "speculation-window")?,
        };
        Ok(())
    }
}

impl Expr2Smt<()> for expr::Cache {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::Init => write!(w, "cache-init")?,
            Self::Fetch(width) => write!(w, "cache-fetch{}", width)?,
        };
        Ok(())
    }
}

impl Expr2Smt<()> for expr::BranchTargetBuffer {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::Init => write!(w, "btb-init")?,
            Self::Track => write!(w, "btb-track")?,
        };
        Ok(())
    }
}

impl Expr2Smt<()> for expr::PatternHistoryTable {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::Init => write!(w, "pht-init")?,
            Self::Taken => write!(w, "pht-taken")?,
            Self::NotTaken => write!(w, "pht-not-taken")?,
        };
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
            Self::Boolean => write!(w, "Bool")?,
            Self::Integer => write!(w, "Integer")?,
            Self::BitVector(width) => write!(w, "(_ BitVec {})", width)?,
            Self::Array { range, domain } => {
                write!(w, "(Array ")?;
                range.sort_to_smt2(w)?;
                write!(w, " ")?;
                domain.sort_to_smt2(w)?;
                write!(w, ")")?
            }
            Self::Set { range } => {
                write!(w, "(Array ")?;
                range.sort_to_smt2(w)?;
                write!(w, " Bool)")?
            }
            Self::Memory => write!(w, "Memory")?,
            Self::Predictor => write!(w, "Predictor")?,
            Self::Cache => write!(w, "Cache")?,
            Self::BranchTargetBuffer => write!(w, "BranchTargetBuffer")?,
            Self::PatternHistoryTable => write!(w, "PatternHistoryTable")?,
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
                    expr::BitVector::constant_u64(byte.try_into().unwrap(), address_bits),
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
                    expr::BitVector::constant_u64(byte.try_into().unwrap(), address_bits),
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

fn define_predictor<T>(solver: &mut Solver<T>, word_size: usize) -> Result<()> {
    solver.declare_sort(&expr::Sort::predictor(), 0)?;

    solver.declare_fun(
        "transient-start",
        &[expr::Sort::predictor()],
        &expr::Sort::bit_vector(word_size),
    )?;

    solver.declare_fun(
        "mis-predict",
        &[expr::Sort::predictor(), expr::Sort::bit_vector(word_size)],
        &expr::Sort::boolean(),
    )?;

    solver.declare_fun(
        "speculation-window",
        &[expr::Sort::predictor(), expr::Sort::bit_vector(word_size)],
        &expr::Sort::integer(),
    )?;

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

    // cache functions
    for width in access_widths {
        let mut insert_expr: expr::Expression =
            expr::Variable::new("cache", cache_set_sort.clone()).into();
        for byte in 0..(width / 8) {
            insert_expr = expr::Set::insert(
                insert_expr,
                expr::BitVector::add(
                    expr::Variable::new("addr", addr_sort.clone()).into(),
                    expr::BitVector::constant_u64(byte.try_into().unwrap(), address_bits),
                )?,
            )?;
        }
        solver.define_fun(
            &format!("cache-fetch{}", width),
            &[("cache", expr::Sort::cache()), ("addr", addr_sort.clone())],
            &expr::Sort::cache(),
            &insert_expr,
        )?;
    }

    solver.define_const(
        "cache-init",
        &expr::Sort::cache(),
        "((as const Cache) false)",
    )?;

    Ok(())
}

fn define_btb<T>(solver: &mut Solver<T>, address_bits: usize) -> Result<()> {
    let addr_sort = expr::Sort::bit_vector(address_bits);
    let btb_array_sort = expr::Sort::array(&addr_sort, &addr_sort);

    // btb type
    solver.define_null_sort(&expr::Sort::branch_target_buffer(), &btb_array_sort)?;

    // btb functions
    solver.define_fun(
        "btb-track",
        &[
            ("btb", expr::Sort::branch_target_buffer()),
            ("location", addr_sort.clone()),
            ("target", addr_sort.clone()),
        ],
        &expr::Sort::branch_target_buffer(),
        &expr::Array::store(
            expr::Variable::new("btb", btb_array_sort).into(),
            expr::Variable::new("location", addr_sort.clone()).into(),
            expr::Variable::new("target", addr_sort).into(),
        )?,
    )?;

    solver.define_const(
        "btb-init",
        &expr::Sort::branch_target_buffer(),
        "((as const BranchTargetBuffer) 0)",
    )?;

    Ok(())
}

fn define_pht<T>(solver: &mut Solver<T>, address_bits: usize) -> Result<()> {
    // pht type
    solver.declare_datatypes(&[("PhtEntry", 0, [""], ["Bot", "Taken", "NotTaken"])])?;

    solver.define_null_sort(
        &expr::Sort::pattern_history_table(),
        &format!("(Array (_ BitVec {}) PhtEntry)", address_bits),
    )?;

    // pht functions
    let addr_sort = expr::Sort::bit_vector(address_bits);

    solver.define_fun(
        "pht-taken",
        &[
            ("pht", expr::Sort::pattern_history_table()),
            ("location", addr_sort.clone()),
        ],
        &expr::Sort::pattern_history_table(),
        "(store pht location Taken)",
    )?;

    solver.define_fun(
        "pht-not-taken",
        &[
            ("pht", expr::Sort::pattern_history_table()),
            ("location", addr_sort),
        ],
        &expr::Sort::pattern_history_table(),
        "(store pht location NotTaken)",
    )?;

    solver.define_const(
        "pht-init",
        &expr::Sort::pattern_history_table(),
        "((as const PatternHistoryTable) Bot)",
    )?;

    Ok(())
}
