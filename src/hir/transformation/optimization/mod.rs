use crate::environment::{Environment, OptimizationLevel};
use crate::error::Result;
use crate::hir::ControlFlowGraph;
use crate::ir::Transform;

mod constant_folding;
mod constant_propagation;
mod copy_propagation;
mod dead_code_elimination;
mod expression_simplification;
mod phi_elimination;
mod redundant_instruction_elimination;

use constant_folding::ConstantFolding;
use constant_propagation::ConstantPropagation;
use copy_propagation::CopyPropagation;
use dead_code_elimination::DeadCodeElimination;
use expression_simplification::ExpressionSimplification;
use phi_elimination::PhiElimination;
use redundant_instruction_elimination::RedundantInstructionElimination;

#[derive(Debug, Eq, PartialEq)]
pub enum OptimizationResult {
    Changed,
    Unchanged,
}

pub trait Optimization {
    fn optimize(&self, cfg: &mut ControlFlowGraph) -> Result<OptimizationResult>;
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
            OptimizationLevel::Full => Self::full(),
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
            optimizations: vec![
                Box::new(CopyPropagation::new()),
                Box::new(PhiElimination::new()),
                Box::new(RedundantInstructionElimination::new()),
            ],
            repetitions: 3,
        }
    }

    pub fn full() -> Self {
        Self {
            optimizations: vec![
                Box::new(ConstantFolding::new()),
                Box::new(ConstantPropagation::new()),
                Box::new(CopyPropagation::new()),
                Box::new(ExpressionSimplification::new()),
                Box::new(PhiElimination::new()),
                Box::new(RedundantInstructionElimination::new()),
                Box::new(DeadCodeElimination::new()),
            ],
            repetitions: 5,
        }
    }
}

impl Transform<ControlFlowGraph> for Optimizer {
    fn name(&self) -> &'static str {
        "Optimization"
    }

    fn description(&self) -> String {
        "Optimize".to_string()
    }

    fn transform(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        for _ in 1..=self.repetitions {
            let mut unchanged = true;

            for optimization in &self.optimizations {
                let result = optimization.optimize(cfg)?;
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
