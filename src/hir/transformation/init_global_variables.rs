use crate::error::Result;
use crate::expr;
use crate::hir;
use crate::hir::analysis;

pub fn init_global_variables(program: &mut hir::Program) -> Result<()> {
    let global_variables = analysis::global_variables(&program);

    let cfg = program.control_flow_graph_mut();
    let entry_block = cfg.entry_block_mut().ok_or("CFG entry must be set")?;

    // Havoc all global variables
    for var in global_variables {
        entry_block.assign(var.clone(), expr::Expression::nondet(var.sort().clone()));
    }

    Ok(())
}
