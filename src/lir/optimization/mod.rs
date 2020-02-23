use crate::error::Result;
use crate::lir;

mod copy_propagation;
mod dead_code_elimination;
mod expression_simplification;

use copy_propagation::CopyPropagation;
use dead_code_elimination::DeadCodeElimination;
use expression_simplification::ExpressionSimplification;

#[derive(Eq, PartialEq)]
pub enum OptimizationResult {
    Changed,
    Unchanged,
}

pub trait Optimization {
    fn optimize(&self, program: &mut lir::Program) -> Result<OptimizationResult>;
}

pub struct Optimizer {
    optimizations: Vec<Box<dyn Optimization>>,
    repetitions: usize,
}

impl Optimizer {
    pub fn basic() -> Self {
        Self {
            optimizations: vec![
                Box::new(CopyPropagation::new()),
                Box::new(DeadCodeElimination::new()),
            ],
            repetitions: 1,
        }
    }

    pub fn full() -> Self {
        Self {
            optimizations: vec![
                Box::new(ExpressionSimplification::new()),
                Box::new(CopyPropagation::new()),
                Box::new(DeadCodeElimination::new()),
            ],
            repetitions: 5,
        }
    }

    pub fn optimize(&self, program: &mut lir::Program) -> Result<()> {
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
