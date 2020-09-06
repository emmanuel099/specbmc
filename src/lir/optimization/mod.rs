use crate::environment::{Environment, OptimizationLevel};
use crate::error::Result;
use crate::lir::Program;

mod constant_folding;
mod constant_propagation;
mod copy_propagation;
mod dead_code_elimination;
mod expression_simplification;

use constant_folding::ConstantFolding;
use constant_propagation::ConstantPropagation;
use copy_propagation::CopyPropagation;
use dead_code_elimination::DeadCodeElimination;
use expression_simplification::ExpressionSimplification;

#[derive(Eq, PartialEq)]
pub enum OptimizationResult {
    Changed,
    Unchanged,
}

pub trait Optimization {
    fn optimize(&self, program: &mut Program) -> Result<OptimizationResult>;
}

pub struct Optimizer {
    optimizations: Vec<Box<dyn Optimization>>,
    repetitions: usize,
}

impl Optimizer {
    pub fn new_from_env(env: &Environment) -> Self {
        match env.optimization_level {
            OptimizationLevel::Disabled => Self::none(),
            OptimizationLevel::Basic => Self::basic(),
            OptimizationLevel::Full => {
                if env.generate_counterexample {
                    // Dead code elimination on LIR-level doesn't play nicely with our current CEX construction approach
                    Self::full_without_dce()
                } else {
                    Self::full()
                }
            }
        }
    }

    pub fn none() -> Self {
        Self {
            optimizations: Vec::new(),
            repetitions: 0,
        }
    }

    pub fn basic() -> Self {
        Self {
            optimizations: vec![Box::new(CopyPropagation::new())],
            repetitions: 1,
        }
    }

    pub fn full() -> Self {
        Self {
            optimizations: vec![
                Box::new(ConstantFolding::new()),
                Box::new(ConstantPropagation::new()),
                Box::new(ExpressionSimplification::new()),
                Box::new(CopyPropagation::new()),
                Box::new(DeadCodeElimination::new()),
            ],
            repetitions: 5,
        }
    }

    fn full_without_dce() -> Self {
        Self {
            optimizations: vec![
                Box::new(ConstantFolding::new()),
                Box::new(ConstantPropagation::new()),
                Box::new(ExpressionSimplification::new()),
                Box::new(CopyPropagation::new()),
            ],
            repetitions: 5,
        }
    }

    pub fn optimize(&self, program: &mut Program) -> Result<()> {
        for _ in 1..=self.repetitions {
            let mut unchanged = true;

            for optimization in &self.optimizations {
                let result = optimization.optimize(program)?;
                if result == OptimizationResult::Changed {
                    unchanged = false;
                }
            }

            if unchanged {
                break;
            }
        }

        Ok(())
    }
}
