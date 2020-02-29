use crate::error::Result;
use crate::expr;
use crate::lir;
use std::collections::HashSet;

/// Validate the given LIR program.
///
/// Checks:
///   - No re-assignment to variables
///   - No use of undefined variables
pub fn validate_program(program: &lir::Program) -> Result<()> {
    let mut defs: HashSet<&expr::Variable> = HashSet::new();

    // Def
    for (index, node) in program.nodes().iter().enumerate() {
        if let lir::Node::Let { var, .. } = node {
            if !defs.insert(var) {
                return Err(format!("@{}: Re-assignment of variable `{}`", index, var).into());
            }
        }
    }

    // Use
    for (index, node) in program.nodes().iter().enumerate() {
        match node {
            lir::Node::Let { expr, .. } => {
                for var in expr.variables() {
                    if !defs.contains(var) {
                        return Err(
                            format!("@{}: Use of undefined variable `{}`", index, var).into()
                        );
                    }
                }
            }
            lir::Node::Assert { condition } | lir::Node::Assume { condition } => {
                for var in condition.variables() {
                    if !defs.contains(var) {
                        return Err(
                            format!("@{}: Use of undefined variable `{}`", index, var).into()
                        );
                    }
                }
            }
            _ => (),
        }
    }

    Ok(())
}
