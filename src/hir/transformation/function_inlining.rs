use crate::error::Result;
use crate::hir::{InlinedProgram, Program};

pub struct FunctionInlining {}

impl FunctionInlining {
    pub fn new() -> Self {
        Self {}
    }

    pub fn inline(&self, program: &Program) -> Result<InlinedProgram> {
        // TODO
        let main = program.entry_function().ok_or("no entry function")?;
        Ok(InlinedProgram::new(main.control_flow_graph().clone()))
    }
}
