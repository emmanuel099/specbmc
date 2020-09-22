use crate::environment::{Environment, OptimizationLevel};
use crate::error::Result;
use crate::lir::Program;

mod assertion_elimination;
mod constant_folding;
mod constant_propagation;
mod copy_propagation;
mod dead_code_elimination;
mod expression_simplification;
mod redundant_node_elimination;

use assertion_elimination::AssertionElimination;
use constant_folding::ConstantFolding;
use constant_propagation::ConstantPropagation;
use copy_propagation::CopyPropagation;
use dead_code_elimination::DeadCodeElimination;
use expression_simplification::ExpressionSimplification;
use redundant_node_elimination::RedundantNodeElimination;

#[derive(Eq, PartialEq)]
pub enum OptimizationResult {
    Changed,
    Unchanged,
}

pub trait Optimization {
    fn optimize(&self, program: &mut Program) -> Result<OptimizationResult>;
}

pub struct Optimizer {
    pre_optimizations: Vec<Box<dyn Optimization>>,
    repeated_optimizations: Vec<Box<dyn Optimization>>,
    post_optimizations: Vec<Box<dyn Optimization>>,
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
            pre_optimizations: Vec::new(),
            repeated_optimizations: Vec::new(),
            post_optimizations: Vec::new(),
            repetitions: 0,
        }
    }

    pub fn basic() -> Self {
        Self {
            pre_optimizations: Vec::new(),
            repeated_optimizations: vec![Box::new(CopyPropagation::new())],
            post_optimizations: Vec::new(),
            repetitions: 1,
        }
    }

    pub fn full() -> Self {
        Self {
            pre_optimizations: Vec::new(),
            repeated_optimizations: vec![
                Box::new(ConstantFolding::new()),
                Box::new(ConstantPropagation::new()),
                Box::new(ExpressionSimplification::new()),
                Box::new(CopyPropagation::new()),
                Box::new(DeadCodeElimination::new()),
            ],
            post_optimizations: vec![
                Box::new(AssertionElimination::new()),
                Box::new(RedundantNodeElimination::new()),
            ],
            repetitions: 5,
        }
    }

    fn full_without_dce() -> Self {
        Self {
            pre_optimizations: Vec::new(),
            repeated_optimizations: vec![
                Box::new(ConstantFolding::new()),
                Box::new(ConstantPropagation::new()),
                Box::new(ExpressionSimplification::new()),
                Box::new(CopyPropagation::new()),
            ],
            post_optimizations: vec![
                Box::new(AssertionElimination::new()),
                Box::new(RedundantNodeElimination::new()),
            ],
            repetitions: 5,
        }
    }

    pub fn optimize(&self, program: &mut Program) -> Result<()> {
        for optimization in &self.pre_optimizations {
            optimization.optimize(program)?;
        }

        for _ in 1..=self.repetitions {
            let mut unchanged = true;

            for optimization in &self.repeated_optimizations {
                let result = optimization.optimize(program)?;
                if result == OptimizationResult::Changed {
                    unchanged = false;
                }
            }

            if unchanged {
                break;
            }
        }

        for optimization in &self.post_optimizations {
            optimization.optimize(program)?;
        }

        Ok(())
    }
}
