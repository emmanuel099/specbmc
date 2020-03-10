use crate::environment;
use crate::error::Result;
use crate::expr;
use crate::lir;
use crate::solver::{AssertionCheck, CheckResult, DumpFormula, Model};
use num_bigint::BigUint;
use rsmt2::parse::*;
use rsmt2::print::{Expr2Smt, Sort2Smt, Sym2Smt};
use rsmt2::{Logic, SmtConf, SmtRes, Solver};
use std::cell::RefCell;
use std::convert::TryInto;
use std::fs::File;
use std::path::Path;
use std::rc::Rc;
use std::str::FromStr;

pub struct RSMTSolver {
    solver: Rc<RefCell<rsmt2::Solver<Parser>>>,
}

impl RSMTSolver {
    pub fn new_from_env(env: &environment::Environment) -> Result<Self> {
        let mut conf = match env.solver() {
            environment::Solver::Z3 => SmtConf::z3(),
            environment::Solver::CVC4 => SmtConf::cvc4(),
            environment::Solver::Yices2 => SmtConf::yices_2(),
        };

        // Activate model production
        conf.models();

        let parser = Parser::new();
        let solver = Rc::new(RefCell::new(Solver::new(conf, parser)?));

        Ok(Self { solver })
    }
}

impl DumpFormula for RSMTSolver {
    fn dump_formula_to_file(&self, path: &Path) -> Result<()> {
        let mut solver = self.solver.borrow_mut();
        let file = File::create(Path::new(path))?;
        Ok(solver.tee(file)?)
    }
}

impl AssertionCheck for RSMTSolver {
    fn encode_program(&mut self, program: &lir::Program) -> Result<()> {
        let mut solver = self.solver.borrow_mut();

        solver.set_logic(Logic::QF_AUFBV)?;

        let access_widths = vec![8, 16, 32, 64, 128, 256, 512];

        define_predictor(&mut solver)?;
        define_memory(&mut solver, &access_widths)?;
        define_cache(&mut solver, &access_widths)?;
        define_btb(&mut solver)?;
        define_pht(&mut solver)?;

        let mut assertions: Vec<expr::Expression> = Vec::new();

        // Declare let variables first to avoid ordering problems because of top-down parsing ...
        for node in program.nodes() {
            if let lir::Node::Let { var, .. } = node {
                declare_variable(&mut solver, var)?;
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
                    define_variable(&mut solver, &assertion, &condition)?;
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

    fn check_assertions(&mut self) -> Result<CheckResult> {
        let mut solver = self.solver.borrow_mut();

        let is_sat = solver.check_sat()?;
        if is_sat {
            let model = Box::new(RSMTModel::new(Rc::clone(&self.solver)));
            Ok(CheckResult::AssertionViolated { model })
        } else {
            Ok(CheckResult::AssertionsHold)
        }
    }
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
            Self::Constant(c) => c.expr_to_smt2(w, i),
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

impl Expr2Smt<()> for expr::Constant {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::Boolean(true) => write!(w, "true")?,
            Self::Boolean(false) => write!(w, "false")?,
            Self::Integer(value) => write!(w, "{}", value)?,
            Self::BitVector(bv) => write!(w, "(_ bv{} {})", bv.value(), bv.bits())?,
            _ => unimplemented!(),
        };
        Ok(())
    }
}

impl Expr2Smt<()> for expr::Boolean {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
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
            Self::ToBoolean | Self::FromBoolean(_) => {
                panic!("ToBoolean/FromBoolean should have been lowered")
            }
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
            Self::SpeculationWindow => write!(w, "speculation-window")?,
            Self::Speculate => write!(w, "predictor-speculate")?,
            Self::Taken => write!(w, "predictor-taken")?,
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
        "speculation-window",
        &[expr::Sort::predictor(), expr::Sort::word()],
        &expr::Sort::bit_vector(environment::SPECULATION_WINDOW_SIZE),
    )?;

    solver.declare_fun(
        "predictor-speculate",
        &[expr::Sort::predictor(), expr::Sort::word()],
        &expr::Sort::boolean(),
    )?;

    solver.declare_fun(
        "predictor-taken",
        &[expr::Sort::predictor(), expr::Sort::word()],
        &expr::Sort::boolean(),
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

struct RSMTModel {
    solver: Rc<RefCell<rsmt2::Solver<Parser>>>,
}

impl RSMTModel {
    pub fn new(solver: Rc<RefCell<rsmt2::Solver<Parser>>>) -> Self {
        Self { solver }
    }
}

impl Model for RSMTModel {
    fn get_interpretation(&self, variable: &expr::Variable) -> Option<expr::Constant> {
        self.evaluate(&variable.clone().into())
    }

    fn evaluate(&self, expr: &expr::Expression) -> Option<expr::Constant> {
        let mut solver = self.solver.borrow_mut();

        if let Ok(result) = solver.get_values(&[expr]) {
            if let Some((_, value)) = result.first() {
                if value.is_array() {
                    match expr.sort() {
                        expr::Sort::Cache => array_to_cache(value.unwrap_array()).ok(),
                        expr::Sort::Memory => array_to_memory(value.unwrap_array()).ok(),
                        _ => Some(value.clone()),
                    }
                } else {
                    Some(value.clone())
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

fn array_to_cache(array: &expr::ArrayValue) -> Result<expr::Constant> {
    let default_is_cached: bool = if let Some(value) = array.default_value() {
        value.try_into()?
    } else {
        true
    };

    let mut cache = if default_is_cached {
        expr::CacheValue::new_full()
    } else {
        expr::CacheValue::new_empty()
    };
    for (address, is_cached) in array.entries() {
        if is_cached.try_into()? {
            cache.fetch(address.try_into()?);
        } else {
            cache.evict(address.try_into()?);
        }
    }
    Ok(expr::Constant::cache(cache))
}

fn array_to_memory(array: &expr::ArrayValue) -> Result<expr::Constant> {
    let default_value: u8 = if let Some(value) = array.default_value() {
        value.try_into()?
    } else {
        0x00
    };

    let mut memory = expr::MemoryValue::new(default_value);
    for (address, value) in array.entries() {
        memory.store(address.try_into()?, value.try_into()?)
    }
    Ok(expr::Constant::memory(memory))
}

mod parser {
    use super::*;
    use nom::{
        branch::alt,
        bytes::complete::{tag, take_while1},
        character::complete::{char, digit1, hex_digit1, multispace1},
        combinator::{all_consuming, map, map_res, value},
        sequence::{preceded, terminated, tuple},
        IResult,
    };

    fn bit_vec_sort(input: &str) -> IResult<&str, expr::Sort> {
        map(
            tuple((
                tag("(_"),
                multispace1,
                tag("BitVec"),
                multispace1,
                map_res(digit1, FromStr::from_str),
                char(')'),
            )),
            |(_, _, _, _, bits, _)| expr::Sort::bit_vector(bits),
        )(input)
    }

    fn array_sort(input: &str) -> IResult<&str, expr::Sort> {
        map(
            tuple((
                tag("(Array"),
                multispace1,
                sort,
                multispace1,
                sort,
                char(')'),
            )),
            |(_, _, range, _, domain, _)| expr::Sort::array(&range, &domain),
        )(input)
    }

    fn sort(input: &str) -> IResult<&str, expr::Sort> {
        alt((
            value(expr::Sort::Boolean, tag("Bool")),
            value(expr::Sort::Integer, tag("Int")),
            value(expr::Sort::Memory, tag("Memory")),
            value(expr::Sort::Predictor, tag("Predictor")),
            value(expr::Sort::Cache, tag("Cache")),
            value(expr::Sort::BranchTargetBuffer, tag("BranchTargetBuffer")),
            value(expr::Sort::PatternHistoryTable, tag("PatternHistoryTable")),
            bit_vec_sort,
            array_sort,
        ))(input)
    }

    fn bin_digit1(input: &str) -> IResult<&str, &str> {
        take_while1(|c| c == '0' || c == '1')(input)
    }

    fn boolean_literal(input: &str) -> IResult<&str, expr::Constant> {
        alt((
            value(expr::Constant::boolean(false), tag("false")),
            value(expr::Constant::boolean(true), tag("true")),
        ))(input)
    }

    fn int_literal(input: &str) -> IResult<&str, expr::Constant> {
        map(map_res(digit1, FromStr::from_str), expr::Constant::integer)(input)
    }

    fn bitvec_literal_hex(input: &str) -> IResult<&str, expr::Constant> {
        let from_str = |s: &str| {
            expr::Constant::bit_vector_big_uint(BigUint::parse_bytes(s.as_bytes(), 16).unwrap())
        };
        map(preceded(tag("#x"), hex_digit1), from_str)(input)
    }

    fn bitvec_literal_binary(input: &str) -> IResult<&str, expr::Constant> {
        let from_str = |s: &str| {
            expr::Constant::bit_vector_big_uint(BigUint::parse_bytes(s.as_bytes(), 2).unwrap())
        };
        map(preceded(tag("#b"), bin_digit1), from_str)(input)
    }

    fn bitvec_literal_smt(input: &str) -> IResult<&str, expr::Constant> {
        // (_ bv42 64)
        map(
            tuple((tag("(_ bv"), digit1, char(' '), digit1, char(')'))),
            |(_, value, _, _, _)| {
                let value: &str = value;
                expr::Constant::bit_vector_big_uint(
                    BigUint::parse_bytes(value.as_bytes(), 10).unwrap(),
                )
            },
        )(input)
    }

    fn bitvec_literal(input: &str) -> IResult<&str, expr::Constant> {
        alt((
            bitvec_literal_hex,
            bitvec_literal_binary,
            bitvec_literal_smt,
        ))(input)
    }

    fn as_const(input: &str) -> IResult<&str, expr::Sort> {
        // (as const (Array (_ BitVec 64) (_ BitVec 8)))
        preceded(tag("(as const "), terminated(sort, char(')')))(input)
    }

    fn array_init(input: &str) -> IResult<&str, expr::ArrayValue> {
        // ((as const (Array (_ BitVec 64) (_ BitVec 8))) (_ bv0 8))
        map(
            tuple((char('('), as_const, multispace1, literal, char(')'))),
            |(_, _, _, value, _)| expr::ArrayValue::new(Some(value)),
        )(input)
    }

    fn array_store(input: &str) -> IResult<&str, expr::ArrayValue> {
        // (store mem addr value)
        map(
            tuple((
                tag("(store"),
                multispace1,
                array_nested,
                multispace1,
                literal,
                multispace1,
                literal,
                char(')'),
            )),
            |(_, _, mut arr, _, addr, _, value, _)| {
                arr.store(addr, value);
                arr
            },
        )(input)
    }

    fn array_nested(input: &str) -> IResult<&str, expr::ArrayValue> {
        alt((array_init, array_store))(input)
    }

    fn array_literal(input: &str) -> IResult<&str, expr::Constant> {
        map(array_nested, expr::Constant::array)(input)
    }

    fn literal(input: &str) -> IResult<&str, expr::Constant> {
        alt((boolean_literal, bitvec_literal, int_literal, array_literal))(input)
    }

    pub(super) fn parse_literal(input: &str) -> SmtRes<expr::Constant> {
        match all_consuming(literal)(input) {
            Ok((_, lit)) => Ok(lit),
            Err(_) => Err("Failed to parse literal!".into()),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Parser;

impl Parser {
    pub fn new() -> Self {
        Self {}
    }
}

impl<'a> ValueParser<expr::Constant, &'a str> for Parser {
    fn parse_value(self, input: &'a str) -> SmtRes<expr::Constant> {
        //println!("ValueParser::parse_value: {}", input);
        parser::parse_literal(input)
    }
}

impl<'a> ExprParser<String, (), &'a str> for Parser {
    fn parse_expr(self, input: &'a str, _: ()) -> SmtRes<String> {
        //println!("ExprParser::parse_expr: {}", input);
        Ok(input.into())
    }
}
