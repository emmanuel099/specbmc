use crate::error::Result;
use crate::hir::{InlinedProgram, Program};

pub struct Inlining {}

impl Inlining {
    pub fn new() -> Self {
        Self {}
    }

    pub fn inline(&self, program: &Program) -> Result<InlinedProgram> {
        // TODO
        let main = program.function_by_name("main").ok_or("no main function")?;
        Ok(InlinedProgram::new(main.control_flow_graph().clone()))
    }
}
