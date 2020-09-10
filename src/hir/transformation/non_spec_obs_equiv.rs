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
        for block in cfg.blocks_mut() {
            let mut instructions_to_insert = Vec::new();

            let is_transient = block.is_transient();
            for (index, inst) in block.instructions().iter().enumerate() {
                if inst.is_observable() {
                    let mut nonspec_inst = create_nonspec_indistinguishable_equivalent(inst);
                    nonspec_inst.labels_mut().pseudo();
                    instructions_to_insert.push((index, nonspec_inst));
                } else if !is_transient && instruction_requires_nonspec_equivalent(inst) {
                    // Non-speculative counterparts are only affected during non-transient execution.
                    let mut nonspec_inst = create_nonspec_instruction_equivalent(inst);
                    nonspec_inst.labels_mut().pseudo();
                    instructions_to_insert.push((index, nonspec_inst));
                }
            }

            block.insert_instructions(instructions_to_insert)?;
        }

        self.add_initial_spec_nonspec_equivalence_constraints(cfg)?;

        Ok(())
    }
}

/// Observable is a special case as we want observable microarchitectual components to be
/// indistinguishable without speculation. Therefore, we add an indistinguishable instruction for
/// each observable instruction, requiring that the non-speculative counterparts of all observable
/// microarchitectual components are indistinguishable.
fn create_nonspec_indistinguishable_equivalent(inst: &Instruction) -> Instruction {
    assert!(inst.is_observable());

    if let Operation::Observable { expr } = inst.operation() {
        let mut nonspec_expr = expr.clone();
        nonspec_expr
            .variables_mut()
            .into_iter()
            .for_each(replace_variable_with_nonspec_equivalent);
        Instruction::indistinguishable(nonspec_expr)
    } else {
        unreachable!()
    }
}

/// For each operation which affects microarchitectual components an equivalent operation
/// affecting their non-speculative counterparts will be added.
fn instruction_requires_nonspec_equivalent(inst: &Instruction) -> bool {
    let requires_nonspec_equivalent =
        |var: &Variable| -> bool { var.sort().is_rollback_persistent() };

    inst.variables()
        .into_iter()
        .any(requires_nonspec_equivalent)
}

/// Simply clone the instruction and replace e.g. cache variables with their non-speculative counterparts.
fn create_nonspec_instruction_equivalent(inst: &Instruction) -> Instruction {
    assert!(!inst.is_observable());

    let mut nonspec_inst = inst.clone();
    nonspec_inst
        .variables_read_mut()
        .into_iter()
        .for_each(replace_variable_with_nonspec_equivalent);
    nonspec_inst
        .variables_written_mut()
        .into_iter()
        .for_each(replace_variable_with_nonspec_equivalent);
    nonspec_inst
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
