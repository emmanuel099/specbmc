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
use std::fs::File;
use std::path::Path;
use std::rc::Rc;
use std::str::FromStr;

type SolverType = environment::Solver;

pub struct RSMTSolver {
    solver: Rc<RefCell<rsmt2::Solver<Parser>>>,
    solver_type: SolverType,
}

impl RSMTSolver {
    pub fn new_from_env(env: &environment::Environment) -> Result<Self> {
        let mut conf = match env.solver {
            environment::Solver::Z3 => SmtConf::default_z3(),
            environment::Solver::CVC4 => SmtConf::default_cvc4(),
            environment::Solver::Yices2 => SmtConf::default_yices_2(),
        };

        // Activate model production
        conf.models();

        let parser = Parser::new();
        let solver = Rc::new(RefCell::new(Solver::new(conf, parser)?));

        Ok(Self {
            solver,
            solver_type: env.solver,
        })
    }
}

impl DumpFormula for RSMTSolver {
    fn dump_formula_to_file(&self, path: &Path) -> Result<()> {
        let mut solver = self.solver.borrow_mut();
        let file = File::create(Path::new(path))?;
        Ok(solver.tee(file)?)
    }
}

// There is no other (easy) way to propagate this information along expr_to_smt2.
// The solver type is required for list encoding.
thread_local!(static SOLVER_TYPE: RefCell<Option<SolverType>> = RefCell::new(None));
fn solver_type() -> SolverType {
    SOLVER_TYPE.with(|type_cell| {
        let solver_type = type_cell.borrow_mut();
        solver_type.unwrap()
    })
}

impl AssertionCheck for RSMTSolver {
    fn encode_program(&mut self, program: &lir::Program) -> Result<()> {
        let mut solver = self.solver.borrow_mut();

        SOLVER_TYPE.with(|type_cell| {
            let mut solver_type = type_cell.borrow_mut();
            *solver_type = Some(self.solver_type);
        });

        if self.solver_type == SolverType::Yices2 {
            solver.set_logic(Logic::QF_AUFBV)?;
        }

        let access_widths = vec![8, 16, 32, 64, 128, 256, 512];

        define_predictor(&mut solver)?;
        define_memory(&mut solver, &access_widths)?;
        define_cache(&mut solver, &access_widths)?;
        define_btb(&mut solver)?;
        define_pht(&mut solver)?;

        match self.solver_type {
            SolverType::Yices2 => {
                // Does not support declare_datatypes
            }
            SolverType::CVC4 => {
                define_tuple(&mut solver)?;
                define_list(&mut solver)?;
            }
            SolverType::Z3 => {
                define_tuple(&mut solver)?;
                // Z3 has builtin theory of lists
            }
        }

        let mut assertions: Vec<expr::Expression> = Vec::new();

        // Declare let variables first to avoid ordering problems because of top-down parsing ...
        for node in program.nodes() {
            if let lir::Node::Let { var, .. } = node {
                declare_variable(&mut solver, var)?;
            }
        }

        for node in program.nodes() {
            match node {
                lir::Node::Comment(text) => solver.comment(text)?,
                lir::Node::Let { var, expr } => {
                    if !expr.is_nondet() {
                        let assignment = expr::Expression::equal(var.clone().into(), expr.clone())?;
                        solver.assert(&assignment)?
                    }
                }
                lir::Node::Assert { condition } => {
                    let name = format!("_assertion{}", assertions.len());
                    let assertion = expr::Variable::new(name, expr::Sort::boolean());
                    define_variable(&mut solver, &assertion, condition)?;
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
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: ()) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        self.expr_to_smt2(w, self.sort())
    }
}

impl Expr2Smt<&expr::Sort> for expr::Expression {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: &expr::Sort) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        if let Some(lowered_expr) = lower_to_smt(self) {
            // Self has been lowered, encode the lowered expression instead.
            return lowered_expr.expr_to_smt2(w, self.sort());
        }

        if self.operands().is_empty() {
            self.operator().expr_to_smt2(w, self.sort())
        } else {
            write!(w, "(")?;
            self.operator().expr_to_smt2(w, self.sort())?;
            for operand in self.operands() {
                write!(w, " ")?;
                operand.expr_to_smt2(w, operand.sort())?;
            }
            write!(w, ")")?;
            Ok(())
        }
    }
}

/// Lowers the given expression to an SMT-encodable expression.
///
/// Some expressions need to be re-written (lowered) before they can be encoded,
/// such as `bool2bv`.
///
/// Returns Some(Expression) if given expression has been re-written or None if given expression is already encodable.
fn lower_to_smt(expr: &expr::Expression) -> Option<expr::Expression> {
    match expr.operator() {
        expr::Operator::BitVector(op) => lower_to_smt_bitvec(op, expr.operands()),
        _ => None,
    }
}

fn lower_to_smt_bitvec(
    op: &expr::BitVector,
    operands: &[expr::Expression],
) -> Option<expr::Expression> {
    match (op, operands) {
        (expr::BitVector::ToBoolean, [expr]) => {
            let width = expr.sort().unwrap_bit_vector();
            let zero = expr::BitVector::constant_u64(0, width);
            expr::Expression::unequal(expr.clone(), zero).ok()
        }
        (expr::BitVector::FromBoolean(bits), [expr]) => {
            let zero = expr::BitVector::constant_u64(0, *bits);
            let one = expr::BitVector::constant_u64(1, *bits);
            expr::Expression::ite(expr.clone(), one, zero).ok()
        }
        (expr::BitVector::SaturatingSub, [lhs, rhs]) => {
            let width = lhs.sort().unwrap_bit_vector();
            let result = expr::BitVector::sub(lhs.to_owned(), rhs.to_owned()).unwrap();
            let zero = expr::BitVector::constant_u64(0, width);
            let underflow = expr::BitVector::ugt(result.clone(), lhs.to_owned()).unwrap();
            expr::Expression::ite(underflow, zero, result).ok()
        }
        _ => None,
    }
}

impl Expr2Smt<&expr::Sort> for expr::Operator {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, sort: &expr::Sort) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::Variable(v) => v.sym_to_smt2(w, ()),
            Self::Constant(c) => c.expr_to_smt2(w, sort),
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
            Self::Cast(sort) => {
                write!(w, "(as const ")?;
                sort.sort_to_smt2(w)?;
                write!(w, ")")?;
                Ok(())
            }
            Self::Boolean(op) => op.expr_to_smt2(w, sort),
            Self::Integer(op) => op.expr_to_smt2(w, sort),
            Self::BitVector(op) => op.expr_to_smt2(w, sort),
            Self::Array(op) => op.expr_to_smt2(w, sort),
            Self::List(op) => op.expr_to_smt2(w, sort),
            Self::Tuple(op) => op.expr_to_smt2(w, sort),
            Self::Memory(op) => op.expr_to_smt2(w, sort),
            Self::Predictor(op) => op.expr_to_smt2(w, sort),
            Self::Cache(op) => op.expr_to_smt2(w, sort),
            Self::BranchTargetBuffer(op) => op.expr_to_smt2(w, sort),
            Self::PatternHistoryTable(op) => op.expr_to_smt2(w, sort),
        }
    }
}

impl Expr2Smt<&expr::Sort> for expr::Constant {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, sort: &expr::Sort) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::Boolean(true) => {
                write!(w, "true")?;
                Ok(())
            }
            Self::Boolean(false) => {
                write!(w, "false")?;
                Ok(())
            }
            Self::Integer(value) => {
                write!(w, "{}", value)?;
                Ok(())
            }
            Self::BitVector(bv) => {
                write!(w, "(_ bv{} {})", bv.value(), bv.bits())?;
                Ok(())
            }
            Self::Cache(value) => value.expr_to_smt2(w, sort),
            _ => unimplemented!(),
        }
    }
}

impl Expr2Smt<&expr::Sort> for expr::CacheValue {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: &expr::Sort) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        let cache = match self.addresses() {
            expr::CacheAddresses::EvictedFromFullCache(addresses) => {
                let mut cache = expr::Expression::cast(
                    expr::Sort::cache(),
                    expr::Boolean::constant(true), // full cache
                );

                for address in addresses {
                    cache = expr::Cache::evict(8, cache, expr::BitVector::word_constant(address))
                        .unwrap();
                }

                cache
            }
            expr::CacheAddresses::FetchedIntoEmptyCache(addresses) => {
                let mut cache = expr::Expression::cast(
                    expr::Sort::cache(),
                    expr::Boolean::constant(false), // empty cache
                );

                for address in addresses {
                    cache = expr::Cache::fetch(8, cache, expr::BitVector::word_constant(address))
                        .unwrap();
                }

                cache
            }
        };

        cache.expr_to_smt2(w, ())
    }
}

impl Expr2Smt<&expr::Sort> for expr::Boolean {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: &expr::Sort) -> SmtRes<()>
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

impl Expr2Smt<&expr::Sort> for expr::Integer {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: &expr::Sort) -> SmtRes<()>
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

impl Expr2Smt<&expr::Sort> for expr::BitVector {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: &expr::Sort) -> SmtRes<()>
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
            Self::ToBoolean => panic!("ToBoolean should have been lowered"),
            Self::FromBoolean(_) => panic!("FromBoolean should have been lowered"),
            Self::SaturatingSub => panic!("SaturatingSub should have been lowered"),
        };
        Ok(())
    }
}

impl Expr2Smt<&expr::Sort> for expr::Array {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: &expr::Sort) -> SmtRes<()>
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

impl Expr2Smt<&expr::Sort> for expr::List {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, sort: &expr::Sort) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::Nil => {
                if solver_type() == SolverType::CVC4 {
                    write!(w, "(as nil ")?;
                    sort.sort_to_smt2(w)?;
                    write!(w, ")")?
                } else {
                    write!(w, "nil")?;
                }
            }
            Self::Cons => {
                if solver_type() == SolverType::Z3 {
                    write!(w, "insert")?
                } else {
                    write!(w, "cons")?
                }
            }
            Self::Head => write!(w, "head")?,
            Self::Tail => write!(w, "tail")?,
        };
        Ok(())
    }
}

impl Expr2Smt<&expr::Sort> for expr::Tuple {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, sort: &expr::Sort) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        let field_count = sort.unwrap_tuple().len();

        match self {
            Self::Make => write!(w, "tuple{}", field_count)?,
            Self::Get(field) => write!(w, "tuple{}-field{}", field_count, field)?,
        };
        Ok(())
    }
}

impl Expr2Smt<&expr::Sort> for expr::Memory {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: &expr::Sort) -> SmtRes<()>
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

impl Expr2Smt<&expr::Sort> for expr::Predictor {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: &expr::Sort) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::SpeculationWindow => write!(w, "speculation-window")?,
            Self::Speculate => write!(w, "predictor-speculate")?,
            Self::Taken => write!(w, "predictor-taken")?,
        };
        Ok(())
    }
}

impl Expr2Smt<&expr::Sort> for expr::Cache {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: &expr::Sort) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::Fetch(width) => write!(w, "cache-fetch{}", width)?,
            Self::Evict(width) => write!(w, "cache-evict{}", width)?,
        };
        Ok(())
    }
}

impl Expr2Smt<&expr::Sort> for expr::BranchTargetBuffer {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: &expr::Sort) -> SmtRes<()>
    where
        Writer: ::std::io::Write,
    {
        match self {
            Self::Track => write!(w, "btb-track")?,
        };
        Ok(())
    }
}

impl Expr2Smt<&expr::Sort> for expr::PatternHistoryTable {
    fn expr_to_smt2<Writer>(&self, w: &mut Writer, _: &expr::Sort) -> SmtRes<()>
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
            Self::List { domain } => {
                write!(w, "(List ")?;
                domain.sort_to_smt2(w)?;
                write!(w, ")")?
            }
            Self::Tuple { fields } => {
                write!(w, "(Tuple{}", fields.len())?;
                for field in fields {
                    write!(w, " ")?;
                    field.sort_to_smt2(w)?;
                }
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
    let mem_array_sort = expr::Sort::array(expr::Sort::word(), expr::Sort::bit_vector(8));
    solver.define_null_sort(&expr::Sort::memory(), &mem_array_sort)?;

    // memory load functions
    for width in access_widths {
        let mut array_selects = vec![];
        for byte in (0..(width / 8)).rev() {
            array_selects.push(expr::Array::select(
                expr::Variable::new("mem", mem_array_sort.clone()).into(),
                expr::BitVector::add(
                    expr::Variable::new("addr", expr::Sort::word()).into(),
                    expr::BitVector::word_constant(byte.try_into().unwrap()),
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
                    expr::BitVector::word_constant(byte.try_into().unwrap()),
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
    let cache_set_sort = expr::Sort::array(expr::Sort::word(), expr::Sort::boolean());
    solver.define_null_sort(&expr::Sort::cache(), &cache_set_sort)?;

    // cache fetch
    for width in access_widths {
        let mut insert_expr: expr::Expression =
            expr::Variable::new("cache", cache_set_sort.clone()).into();
        for byte in 0..(width / 8) {
            insert_expr = expr::Array::store(
                insert_expr,
                expr::BitVector::add(
                    expr::Variable::new("addr", expr::Sort::word()).into(),
                    expr::BitVector::word_constant(byte.try_into().unwrap()),
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

    // cache evict
    for width in access_widths {
        let mut insert_expr: expr::Expression =
            expr::Variable::new("cache", cache_set_sort.clone()).into();
        for byte in 0..(width / 8) {
            insert_expr = expr::Array::store(
                insert_expr,
                expr::BitVector::add(
                    expr::Variable::new("addr", expr::Sort::word()).into(),
                    expr::BitVector::word_constant(byte.try_into().unwrap()),
                )?,
                expr::Boolean::constant(false),
            )?;
        }
        solver.define_fun(
            &format!("cache-evict{}", width),
            &[("cache", expr::Sort::cache()), ("addr", expr::Sort::word())],
            &expr::Sort::cache(),
            &insert_expr,
        )?;
    }

    Ok(())
}

fn define_btb<T>(solver: &mut Solver<T>) -> Result<()> {
    // btb type
    let btb_array_sort = expr::Sort::array(expr::Sort::word(), expr::Sort::word());
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
        &expr::Sort::array(expr::Sort::word(), expr::Sort::boolean()),
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

fn define_tuple<T>(solver: &mut Solver<T>) -> Result<()> {
    for field_count in 1..10 {
        let sort_name = format!("Tuple{}", field_count);
        let types: Vec<String> = (0..field_count).map(|i| format!("T{}", i)).collect();
        let fun: String = (0..field_count).fold(format!("tuple{}", field_count), |acc, i| {
            acc + &format!(" (tuple{}-field{} T{})", field_count, i, i)
        });

        solver.declare_datatypes(&[(sort_name, field_count, types, vec![format!("({})", fun)])])?;
    }

    Ok(())
}

fn define_list<T>(solver: &mut Solver<T>) -> Result<()> {
    solver.declare_datatypes(&[(
        "List",
        1,
        vec!["T"],
        vec!["(nil)", "(cons (head T) (tail (List T)))"],
    )])?;

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
                    let arr = value.unwrap_array();
                    match expr.sort() {
                        expr::Sort::Cache => array_to_cache(arr).ok().map(expr::Constant::cache),
                        expr::Sort::Memory => array_to_memory(arr).ok().map(expr::Constant::memory),
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

fn array_to_cache(array: &expr::ArrayValue) -> Result<expr::CacheValue> {
    let default_is_cached: bool = if let Some(value) = array.default_value() {
        value.try_into()?
    } else {
        true
    };

    let mut cache = if default_is_cached {
        expr::CacheValue::full()
    } else {
        expr::CacheValue::empty()
    };
    for (address, is_cached) in array.entries() {
        if is_cached.try_into()? {
            cache.fetch(address.try_into()?);
        } else {
            cache.evict(address.try_into()?);
        }
    }
    Ok(cache)
}

fn array_to_memory(array: &expr::ArrayValue) -> Result<expr::MemoryValue> {
    let default_value: u8 = if let Some(value) = array.default_value() {
        value.try_into()?
    } else {
        0x00
    };

    let mut memory = expr::MemoryValue::new(default_value);
    for (address, value) in array.entries() {
        memory.store(address.try_into()?, value.try_into()?)
    }
    Ok(memory)
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
            |(_, _, range, _, domain, _)| expr::Sort::array(range, domain),
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
