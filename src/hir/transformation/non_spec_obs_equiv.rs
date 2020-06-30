use crate::environment::Environment;
use crate::error::Result;
use crate::expr::{BranchTargetBuffer, Cache, Expression, PatternHistoryTable, Sort, Variable};
use crate::hir::{Operation, Program};
use crate::ir::Transform;

#[derive(Default, Builder, Debug)]
pub struct NonSpecObsEquivalence {
    cache_available: bool,
    btb_available: bool,
    pht_available: bool,
}

impl NonSpecObsEquivalence {
    pub fn new_from_env(env: &Environment) -> Self {
        Self {
            cache_available: env.architecture.cache,
            btb_available: env.architecture.branch_target_buffer,
            pht_available: env.architecture.pattern_history_table,
        }
    }

    fn add_initial_spec_nonspec_equivalence_constraints(
        &self,
        program: &mut Program,
    ) -> Result<()> {
        let mut eq_vars: Vec<(Variable, Variable)> = Vec::new();
        if self.cache_available {
            let cache_spec = Cache::variable();
            eq_vars.push((cache_nonspec(), cache_spec));
        }
        if self.btb_available {
            let btb_spec = BranchTargetBuffer::variable();
            eq_vars.push((btb_nonspec(), btb_spec));
        }
        if self.pht_available {
            let pht_spec = PatternHistoryTable::variable();
            eq_vars.push((pht_nonspec(), pht_spec));
        }

        let cfg = program.control_flow_graph_mut();
        let entry_block = cfg.entry_block_mut()?;

        for (lhs, rhs) in eq_vars {
            entry_block
                .assume(Expression::equal(lhs.into(), rhs.into())?)?
                .labels_mut()
                .pseudo();
        }

        Ok(())
    }
}

impl Transform<Program> for NonSpecObsEquivalence {
    fn name(&self) -> &'static str {
        "NonSpecObsEquivalence"
    }

    fn description(&self) -> &'static str {
        "Add non-speculative observational equivalence constraints"
    }

    fn transform(&self, program: &mut Program) -> Result<()> {
        add_nonspec_operations(program)?;
        self.add_initial_spec_nonspec_equivalence_constraints(program)?;

        Ok(())
    }
}

fn add_nonspec_operations(program: &mut Program) -> Result<()> {
    program
        .control_flow_graph_mut()
        .blocks_mut()
        .iter_mut()
        .filter(|block| !block.is_transient())
        .for_each(|block| {
            block.instructions_mut().iter_mut().for_each(|instruction| {
                let nonspec_operations = instruction
                    .operations()
                    .iter()
                    .filter_map(|op| {
                        if operation_requires_nonspec_equivalent(op) {
                            Some(create_nonspec_operation_equivalent(op))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<Operation>>();

                instruction.add_operations(&nonspec_operations);
            });
        });

    Ok(())
}

fn variable_requires_nonspec_equivalent(var: &Variable) -> bool {
    match var.sort() {
        Sort::Cache | Sort::BranchTargetBuffer | Sort::PatternHistoryTable => true,
        _ => false,
    }
}

fn operation_requires_nonspec_equivalent(operation: &Operation) -> bool {
    match operation {
        Operation::Assert { .. }
        | Operation::Assume { .. }
        | Operation::Observable { .. }
        | Operation::Indistinguishable { .. } => {
            let variables_read = operation.variables_read();
            variables_read
                .into_iter()
                .any(variable_requires_nonspec_equivalent)
        }
        _ => {
            let variables_written = operation.variables_written();
            variables_written
                .into_iter()
                .any(variable_requires_nonspec_equivalent)
        }
    }
}

fn replace_variable_with_nonspec_equivalent(var: &mut Variable) {
    match var.sort() {
        Sort::Cache => *var = cache_nonspec(),
        Sort::BranchTargetBuffer => *var = btb_nonspec(),
        Sort::PatternHistoryTable => *var = pht_nonspec(),
        _ => {}
    };
}

fn create_nonspec_operation_equivalent(op: &Operation) -> Operation {
    match op {
        Operation::Observable { exprs } => {
            // Observable is a special case as we want observable microarchitectual components to be
            // indistinguishable without speculation. Therefore, we emit an indistinguishable operation instead.
            let mut nonspec_exprs = exprs.to_owned();
            nonspec_exprs.iter_mut().for_each(|expr| {
                expr.variables_mut()
                    .into_iter()
                    .for_each(replace_variable_with_nonspec_equivalent);
            });
            Operation::indistinguishable(nonspec_exprs)
        }
        _ => {
            // Simply clone the operation and replace e.g. cache variables with their non-speculative counterparts.
            let mut nonspec_op = op.to_owned();
            nonspec_op
                .variables_read_mut()
                .into_iter()
                .for_each(replace_variable_with_nonspec_equivalent);
            nonspec_op
                .variables_written_mut()
                .into_iter()
                .for_each(replace_variable_with_nonspec_equivalent);
            nonspec_op
        }
    }
}

fn cache_nonspec() -> Variable {
    Variable::new("_cache_ns", Sort::cache())
}

fn btb_nonspec() -> Variable {
    Variable::new("_btb_ns", Sort::branch_target_buffer())
}

fn pht_nonspec() -> Variable {
    Variable::new("_pht_ns", Sort::pattern_history_table())
}
