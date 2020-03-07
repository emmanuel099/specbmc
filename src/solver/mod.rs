use crate::environment::Environment;
use crate::error::Result;
use crate::expr::{Expression, Variable};
use crate::lir::Program;
use std::path::Path;

mod rsmt;

pub trait Model {
    fn get_interpretation(&self, variable: &Variable) -> Option<Expression>;
    fn evaluate(&self, expr: &Expression) -> Option<Expression>;
}

pub enum CheckResult {
    AssertionsHold,
    AssertionViolated { model: Box<dyn Model> },
}

pub trait AssertionCheck {
    fn encode_program(&mut self, program: &Program) -> Result<()>;
    fn check_assertions(&mut self) -> Result<CheckResult>;
}

pub trait DumpFormula {
    fn dump_formula_to_file(&self, path: &Path) -> Result<()>;
}

pub trait Solver: AssertionCheck + DumpFormula {}
impl<T: AssertionCheck + DumpFormula> Solver for T {}

pub fn create_solver(env: &Environment) -> Result<Box<dyn Solver>> {
    let solver = rsmt::RSMTSolver::new_from_env(env)?;
    Ok(Box::new(solver))
}
