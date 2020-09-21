use crate::environment;
use crate::error::Result;
use crate::expr::{BitVector, Expression, Memory, Variable};
use crate::hir::{Block, ControlFlowGraph};
use crate::ir::Transform;

const STACK_BASE: u64 = 0xffff_0000_0000;

#[derive(Default, Builder, Debug)]
pub struct InitStack {}

impl Transform<ControlFlowGraph> for InitStack {
    fn name(&self) -> &'static str {
        "InitStack"
    }

    fn description(&self) -> String {
        "Set up initial state of the stack".to_string()
    }

    fn transform(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        let entry_block = cfg.entry_block_mut()?;

        let base_pointer = BitVector::word_variable(environment::BASE_POINTER);
        havoc_variable(entry_block, base_pointer.clone())?;
        low_equivalent(entry_block, base_pointer.clone().into());

        let stack_pointer = BitVector::word_variable(environment::STACK_POINTER);
        havoc_variable(entry_block, stack_pointer.clone())?;
        low_equivalent(entry_block, stack_pointer.clone().into());

        entry_block
            .assume(BitVector::ult(
                stack_pointer.clone().into(),
                base_pointer.into(),
            )?)?
            .labels_mut()
            .pseudo();

        entry_block
            .assume(BitVector::ugt(
                stack_pointer.clone().into(),
                BitVector::word_constant(STACK_BASE),
            )?)?
            .labels_mut()
            .pseudo();

        let return_address = Memory::load(
            environment::WORD_SIZE,
            Memory::variable().into(),
            stack_pointer.into(),
        )?;
        low_equivalent(entry_block, return_address);

        Ok(())
    }
}

fn havoc_variable(block: &mut Block, var: Variable) -> Result<()> {
    let value = Expression::nondet(var.sort().clone());
    assign_variable(block, var, value)
}

fn assign_variable(block: &mut Block, var: Variable, value: Expression) -> Result<()> {
    block.assign(var, value)?.labels_mut().pseudo();
    Ok(())
}

fn low_equivalent(block: &mut Block, expr: Expression) {
    block.indistinguishable(expr).labels_mut().pseudo();
}
