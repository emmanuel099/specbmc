use crate::environment;
use crate::error::Result;
use crate::expr;
use crate::lir;
use rsmt2::print::{Expr2Smt, Sort2Smt, Sym2Smt};
use rsmt2::{SmtRes, Solver};
use std::convert::TryInto;

pub fn encode_program<T>(solver: &mut Solver<T>, program: &lir::Program) -> Result<()> {
    solver.set_custom_logic("QF_AUFBV")?;

    let access_widths = vec![8, 16, 32, 64, 128];

    define_predictor(solver)?;
    define_memory(solver, &access_widths)?;
    define_cache(solver, &access_widths)?;
    define_btb(solver)?;
    define_pht(solver)?;

    let mut assertions: Vec<expr::Expression> = Vec::new();

    // Declare let variables first to avoid ordering problems because of top-down parsing ...
    for node in program.nodes() {
        if let lir::Node::Let { var, .. } = node {
            declare_variable(solver, var)?;
        }
    }

    for node in program.nodes() {
        match node {
            lir::Node::Comment(text) => solver.comment(&text)?,
            lir::Node::Let { var, expr } => {
                if !expr.is_nondet() {
                    let assignment = expr::Expression::equal(var.clone().into(), expr.clone())?;
                    solver.assert(&assignment)?
                }
            }
            lir::Node::Assert { condition } => {
                let name = format!("_assertion{}", assertions.len());
                let assertion = expr::Variable::new(name, expr::Sort::boolean());
                define_variable(solver, &assertion, &condition)?;
                assertions.push(assertion.into())
            }
            lir::Node::Assume { condition } => solver.assert(&condition)?,
        }
    }

    solver.assert(&expr::Boolean::not(expr::Boolean::conjunction(
        &assertions,
    )?)?)?;

    Ok(())
}

fn declare_variable<T>(solver: &mut Solver<T>, variable: &expr::Variable) -> SmtRes<()> {
    solver.declare_const(variable, variable.sort())
}

fn define_variable<T>(
    solver: &mut Solver<T>,
    variable: &expr::Variable,
    expr: &expr::Expression,
) -> SmtRes<()> {
    if expr.is_nondet() {
        declare_variable(solver, variable)
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

impl Expr2Smt<()> for expr::Memory {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::Store(width) => write!(w, "mem-store{}", width)?,
            Self::Load(width) => write!(w, "mem-load{}", width)?,
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
            Self::Integer => write!(w, "Int")?,
            Self::BitVector(width) => write!(w, "(_ BitVec {})", width)?,
            Self::Array { range, domain } => {
                write!(w, "(Array ")?;
                range.sort_to_smt2(w)?;
                write!(w, " ")?;
                domain.sort_to_smt2(w)?;
                write!(w, ")")?
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

fn define_memory<T>(solver: &mut Solver<T>, access_widths: &[usize]) -> Result<()> {
    // memory type
    let mem_array_sort = expr::Sort::array(&expr::Sort::word(), &expr::Sort::bit_vector(8));
    solver.define_null_sort(&expr::Sort::memory(), &mem_array_sort)?;

    // memory load functions
    for width in access_widths {
        let mut array_selects = vec![];
        for byte in (0..(width / 8)).rev() {
            array_selects.push(expr::Array::select(
                expr::Variable::new("mem", mem_array_sort.clone()).into(),
                expr::BitVector::add(
                    expr::Variable::new("addr", expr::Sort::word()).into(),
                    expr::BitVector::constant_u64(byte.try_into().unwrap(), environment::WORD_SIZE),
                )?,
            )?);
        }
        solver.define_fun(
            &format!("mem-load{}", width),
            &[("mem", expr::Sort::memory()), ("addr", expr::Sort::word())],
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
                    expr::Variable::new("addr", expr::Sort::word()).into(),
                    expr::BitVector::constant_u64(byte.try_into().unwrap(), environment::WORD_SIZE),
                )?,
                expr::BitVector::extract(
                    bit_offset + 7,
                    bit_offset,
                    expr::Variable::new("val", expr::Sort::bit_vector(*width)).into(),
                )?,
            )?;
        }
        solver.define_fun(
            &format!("mem-store{}", width),
            &[
                ("mem", expr::Sort::memory()),
                ("addr", expr::Sort::word()),
                ("val", expr::Sort::bit_vector(*width)),
            ],
            &expr::Sort::memory(),
            &store_expr,
        )?;
    }

    Ok(())
}

fn define_predictor<T>(solver: &mut Solver<T>) -> Result<()> {
    solver.declare_sort(&expr::Sort::predictor(), 0)?;

    solver.declare_fun(
        "transient-start",
        &[expr::Sort::predictor()],
        &expr::Sort::word(),
    )?;

    solver.declare_fun(
        "mis-predict",
        &[expr::Sort::predictor(), expr::Sort::word()],
        &expr::Sort::boolean(),
    )?;

    solver.declare_fun(
        "speculation-window",
        &[expr::Sort::predictor(), expr::Sort::word()],
        &expr::Sort::bit_vector(environment::SPECULATION_WINDOW_SIZE),
    )?;

    Ok(())
}

fn define_cache<T>(solver: &mut Solver<T>, access_widths: &[usize]) -> Result<()> {
    // cache type
    let cache_set_sort = expr::Sort::array(&expr::Sort::word(), &expr::Sort::boolean());
    solver.define_null_sort(&expr::Sort::cache(), &cache_set_sort)?;

    // cache functions
    for width in access_widths {
        let mut insert_expr: expr::Expression =
            expr::Variable::new("cache", cache_set_sort.clone()).into();
        for byte in 0..(width / 8) {
            insert_expr = expr::Array::store(
                insert_expr,
                expr::BitVector::add(
                    expr::Variable::new("addr", expr::Sort::word()).into(),
                    expr::BitVector::constant_u64(byte.try_into().unwrap(), environment::WORD_SIZE),
                )?,
                expr::Boolean::constant(true),
            )?;
        }
        solver.define_fun(
            &format!("cache-fetch{}", width),
            &[("cache", expr::Sort::cache()), ("addr", expr::Sort::word())],
            &expr::Sort::cache(),
            &insert_expr,
        )?;
    }

    Ok(())
}

fn define_btb<T>(solver: &mut Solver<T>) -> Result<()> {
    // btb type
    let btb_array_sort = expr::Sort::array(&expr::Sort::word(), &expr::Sort::word());
    solver.define_null_sort(&expr::Sort::branch_target_buffer(), &btb_array_sort)?;

    // btb functions
    solver.define_fun(
        "btb-track",
        &[
            ("btb", expr::Sort::branch_target_buffer()),
            ("location", expr::Sort::word()),
            ("target", expr::Sort::word()),
        ],
        &expr::Sort::branch_target_buffer(),
        &expr::Array::store(
            expr::Variable::new("btb", btb_array_sort).into(),
            expr::Variable::new("location", expr::Sort::word()).into(),
            expr::Variable::new("target", expr::Sort::word()).into(),
        )?,
    )?;

    Ok(())
}

fn define_pht<T>(solver: &mut Solver<T>) -> Result<()> {
    // pht type
    solver.define_null_sort(
        &expr::Sort::pattern_history_table(),
        &expr::Sort::array(&expr::Sort::word(), &expr::Sort::boolean()),
    )?;

    // pht functions
    solver.define_fun(
        "pht-taken",
        &[
            ("pht", expr::Sort::pattern_history_table()),
            ("location", expr::Sort::word()),
        ],
        &expr::Sort::pattern_history_table(),
        "(store pht location true)",
    )?;

    solver.define_fun(
        "pht-not-taken",
        &[
            ("pht", expr::Sort::pattern_history_table()),
            ("location", expr::Sort::word()),
        ],
        &expr::Sort::pattern_history_table(),
        "(store pht location false)",
    )?;

    Ok(())
}
