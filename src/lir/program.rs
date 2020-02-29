use crate::error::Result;
use crate::expr::{Expression, Variable};
use crate::lir::Node;
use crate::util::Validate;
use std::collections::HashSet;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Program {
    nodes: Vec<Node>,
}

impl Program {
    pub fn new() -> Self {
        Self { nodes: vec![] }
    }

    pub fn append_node(&mut self, node: Node) {
        self.nodes.push(node);
    }

    pub fn append_comment<S>(&mut self, text: S)
    where
        S: Into<String>,
    {
        self.append_node(Node::comment(text));
    }

    pub fn append_let(&mut self, var: Variable, expr: Expression) -> Result<()> {
        self.append_node(Node::assign(var, expr)?);
        Ok(())
    }

    pub fn append_assert(&mut self, condition: Expression) -> Result<()> {
        self.append_node(Node::assert(condition)?);
        Ok(())
    }

    pub fn append_assume(&mut self, condition: Expression) -> Result<()> {
        self.append_node(Node::assume(condition)?);
        Ok(())
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut Vec<Node> {
        &mut self.nodes
    }
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for node in self.nodes() {
            writeln!(f, "{}", node)?;
        }
        Ok(())
    }
}

impl Validate for Program {
    /// Validate the given LIR program.
    ///
    /// Checks:
    ///   - No re-assignment to variables
    ///   - No use of undefined variables
    fn validate(&self) -> Result<()> {
        let mut defs: HashSet<&Variable> = HashSet::new();

        // Def
        for (index, node) in self.nodes.iter().enumerate() {
            if let Node::Let { var, .. } = node {
                if !defs.insert(var) {
                    return Err(format!("@{}: Re-assignment of variable `{}`", index, var).into());
                }
            }
        }

        // Use
        for (index, node) in self.nodes.iter().enumerate() {
            match node {
                Node::Let { expr, .. } => {
                    for var in expr.variables() {
                        if !defs.contains(var) {
                            return Err(
                                format!("@{}: Use of undefined variable `{}`", index, var).into()
                            );
                        }
                    }
                }
                Node::Assert { condition } | Node::Assume { condition } => {
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
}
