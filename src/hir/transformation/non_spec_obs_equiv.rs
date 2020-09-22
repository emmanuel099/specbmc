use crate::error::Result;
use crate::expr::Variable;
use crate::hir::{ControlFlowGraph, Instruction, Operation};
use crate::ir::Transform;
use std::collections::HashSet;

#[derive(Default, Builder, Debug)]
pub struct NonSpecObsEquivalence {}

impl NonSpecObsEquivalence {
    /// Initially each microarchitectual component has the same state as their non-speculative counterpart.
    fn add_initial_spec_nonspec_equivalence_constraints(
        &self,
        cfg: &mut ControlFlowGraph,
    ) -> Result<()> {
        let vars: HashSet<Variable> = cfg
            .variables()
            .into_iter()
            .filter(|var| var.labels().is_rollback_persistent())
            .cloned()
            .collect();

        let entry_block = cfg.entry_block_mut()?;
        for var in vars {
            let ns_var = create_nonspec_variable_equivalent(&var);
            entry_block
                .assign(ns_var, var.into())?
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
        for block in cfg
            .blocks_mut()
            .iter_mut()
            .filter(|block| !block.is_transient())
        {
            let mut instructions_to_insert = Vec::new();

            for (index, inst) in block.instructions().iter().enumerate() {
                if inst.is_observable() {
                    let mut nonspec_inst = create_nonspec_indistinguishable_equivalent(inst);
                    nonspec_inst.labels_mut().pseudo();
                    instructions_to_insert.push((index, nonspec_inst));
                } else if instruction_requires_nonspec_equivalent(inst) {
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
        |var: &Variable| -> bool { var.labels().is_rollback_persistent() };

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

fn create_nonspec_variable_equivalent(var: &Variable) -> Variable {
    assert!(var.labels().is_rollback_persistent());

    let name = format!("{}_ns", var.name());
    let sort = var.sort().clone();
    Variable::new(name, sort)
}

fn replace_variable_with_nonspec_equivalent(var: &mut Variable) {
    if var.labels().is_rollback_persistent() {
        *var = create_nonspec_variable_equivalent(var);
    }
}
