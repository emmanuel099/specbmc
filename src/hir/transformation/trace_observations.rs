use crate::error::Result;
use crate::expr::{List, Sort, Tuple, Variable};
use crate::hir::{Block, ControlFlowGraph, Instruction};
use crate::ir::Transform;
use std::collections::HashSet;

#[derive(Default, Builder, Debug)]
pub struct TraceObservations {
    observable_variables: HashSet<Variable>,
}

impl TraceObservations {
    fn trace_observable_variables(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        for block in cfg.blocks_mut() {
            let observable_writes: Vec<usize> = block
                .instructions()
                .iter()
                .enumerate()
                .filter_map(|(index, inst)| {
                    let vars: Vec<Variable> = inst
                        .variables_written()
                        .into_iter()
                        .filter(|var| self.observable_variables.contains(var))
                        .cloned()
                        .collect();

                    if vars.is_empty() {
                        None
                    } else {
                        Some(index)
                    }
                })
                .collect();

            for index in observable_writes.iter().rev() {
                self.trace_observable_variables_at(block, index + 1)?;
            }
        }

        Ok(())
    }

    fn trace_observable_variables_at(&self, block: &mut Block, index: usize) -> Result<()> {
        let elements = self
            .observable_variables
            .iter()
            .map(|var| var.clone().into())
            .collect();
        let state = Tuple::make(elements)?;

        let trace = self.trace_var();
        let mut trace_append =
            Instruction::assign(trace.clone(), List::cons(state, trace.into())?)?;
        trace_append.labels_mut().pseudo();

        block.insert_instruction(index, trace_append)
    }

    fn init_trace_at_entry(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        let entry_block = cfg.entry_block_mut()?;

        entry_block
            .assign(self.trace_var(), List::nil(self.trace_sort()))?
            .labels_mut()
            .pseudo();

        entry_block
            .indistinguishable(self.trace_var().into())
            .labels_mut()
            .pseudo();

        Ok(())
    }

    fn observe_trace_at_exit(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        let exit_block = cfg.exit_block_mut()?;

        exit_block
            .observable(self.trace_var().into())
            .labels_mut()
            .pseudo();

        Ok(())
    }

    fn state_sort(&self) -> Sort {
        let fields: Vec<Sort> = self
            .observable_variables
            .iter()
            .map(|var| var.sort().clone())
            .collect();
        Sort::tuple(fields)
    }

    fn trace_sort(&self) -> Sort {
        Sort::list(self.state_sort())
    }

    fn trace_var(&self) -> Variable {
        let mut var = Variable::new("_trace", self.trace_sort());
        var.labels_mut().rollback_persistent();
        var
    }
}

impl Transform<ControlFlowGraph> for TraceObservations {
    fn name(&self) -> &'static str {
        "TraceObservations"
    }

    fn description(&self) -> String {
        "Add observations trace".to_string()
    }

    fn transform(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        self.init_trace_at_entry(cfg)?;
        self.trace_observable_variables(cfg)?;
        self.observe_trace_at_exit(cfg)?;

        Ok(())
    }
}
