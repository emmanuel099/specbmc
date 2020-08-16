use crate::environment::Environment;
use crate::error::Result;
use crate::expr::{BranchTargetBuffer, Cache, Expression, PatternHistoryTable, Sort, Variable};
use crate::hir::{ControlFlowGraph, Instruction, Operation};
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

    /// Initially each microarchitectual component has the same state as their non-speculative counterpart.
    fn add_initial_spec_nonspec_equivalence_constraints(
        &self,
        cfg: &mut ControlFlowGraph,
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

impl Transform<ControlFlowGraph> for NonSpecObsEquivalence {
    fn name(&self) -> &'static str {
        "NonSpecObsEquivalence"
    }

    fn description(&self) -> String {
        "Add non-speculative observational equivalence constraints".to_string()
    }

    fn transform(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        cfg.blocks_mut().iter_mut().for_each(|block| {
            let is_transient = block.is_transient();
            block.instructions_mut().iter_mut().for_each(|inst| {
                if !is_transient {
                    // Non-speculative counterparts are only affected during non-transient execution.
                    add_nonspec_equivalents_for_all_operations(inst);
                }
                make_observations_nonspec_indistinguishable(inst);
            })
        });

        self.add_initial_spec_nonspec_equivalence_constraints(cfg)?;

        Ok(())
    }
}

/// Observable is a special case as we want observable microarchitectual components to be
/// indistinguishable without speculation. Therefore, we add an indistinguishable operation for
/// each observable operation, requiring that the non-speculative counterparts of all observable
/// microarchitectual components are indistinguishable.
fn make_observations_nonspec_indistinguishable(inst: &mut Instruction) {
    let create_nonspec_indistinguishable_for_observable = |op: &Operation| -> Option<Operation> {
        if let Operation::Observable { exprs } = op {
            let mut nonspec_exprs = exprs.to_owned();
            nonspec_exprs.iter_mut().for_each(|expr| {
                expr.variables_mut()
                    .into_iter()
                    .for_each(replace_variable_with_nonspec_equivalent);
            });
            Some(Operation::indistinguishable(nonspec_exprs))
        } else {
            None
        }
    };

    let operations = inst
        .operations()
        .iter()
        .filter_map(create_nonspec_indistinguishable_for_observable)
        .collect::<Vec<Operation>>();

    inst.add_operations(&operations);
}

/// For each operation which affects microarchitectual components an equivalent operation
/// affecting their non-speculative counterparts will be added.
fn add_nonspec_equivalents_for_all_operations(inst: &mut Instruction) {
    let operations = inst
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

    inst.add_operations(&operations);
}

fn operation_requires_nonspec_equivalent(operation: &Operation) -> bool {
    let requires_nonspec_equivalent =
        |var: &Variable| -> bool { var.sort().is_rollback_persistent() };

    match operation {
        Operation::Observable { .. } => false,
        Operation::Assert { .. }
        | Operation::Assume { .. }
        | Operation::Indistinguishable { .. } => operation
            .variables_read()
            .into_iter()
            .any(requires_nonspec_equivalent),
        _ => operation
            .variables_written()
            .into_iter()
            .any(requires_nonspec_equivalent),
    }
}

/// Simply clone the operation and replace e.g. cache variables with their non-speculative counterparts.
fn create_nonspec_operation_equivalent(op: &Operation) -> Operation {
    assert!(!op.is_observable());

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

fn replace_variable_with_nonspec_equivalent(var: &mut Variable) {
    match var.sort() {
        Sort::Cache => *var = cache_nonspec(),
        Sort::BranchTargetBuffer => *var = btb_nonspec(),
        Sort::PatternHistoryTable => *var = pht_nonspec(),
        _ => {}
    };
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
