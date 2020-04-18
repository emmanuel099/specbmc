use crate::error::Result;
use crate::expr::{Expression, Variable};
use crate::lir::Node;
use crate::util::TranslateInto;
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

    pub fn from<Src: TranslateInto<Self>>(src: &Src) -> Result<Self> {
        src.translate_into()
    }

    /// Returns a reference to the node at the given index.
    pub fn node(&self, index: usize) -> Option<&Node> {
        self.nodes.get(index)
    }

    /// Returns a mutable reference to the node at the given index.
    pub fn node_mut(&mut self, index: usize) -> Option<&mut Node> {
        self.nodes.get_mut(index)
    }

    /// Returns a reference to all nodes of this program.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Returns a mutable reference to all nodes of this program.
    pub fn nodes_mut(&mut self) -> &mut Vec<Node> {
        &mut self.nodes
    }

    /// Adds a comment to the end of this program.
    pub fn comment<S>(&mut self, text: S)
    where
        S: Into<String>,
    {
        self.nodes.push(Node::comment(text));
    }

    /// Adds an assignment to the end of this program.
    pub fn assign(&mut self, var: Variable, expr: Expression) -> Result<()> {
        self.nodes.push(Node::assign(var, expr)?);
        Ok(())
    }

    /// Adds an assertion to the end of this program.
    pub fn assert(&mut self, condition: Expression) -> Result<()> {
        self.nodes.push(Node::assert(condition)?);
        Ok(())
    }

    /// Adds an assumption to the end of this program.
    pub fn assume(&mut self, condition: Expression) -> Result<()> {
        self.nodes.push(Node::assume(condition)?);
        Ok(())
    }
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for node in self.nodes() {
            writeln!(f, "{}", node)?;
        }
        Ok(())
    }
}

impl Validate for Program {
    /// Validates the program.
    ///
    /// Checks:
    ///   - No re-assignment to variables
    ///   - No use of undefined variables
    fn validate(&self) -> Result<()> {
        let mut defs: HashSet<&Variable> = HashSet::new();

        // Def
        for (index, node) in self.nodes.iter().enumerate() {
            for var in node.variables_defined() {
                if !defs.insert(var) {
                    return Err(format!("@{}: Re-assignment of variable `{}`", index, var).into());
                }
            }
        }

        // Use
        for (index, node) in self.nodes.iter().enumerate() {
            for var in node.variables_used() {
                if !defs.contains(var) {
                    return Err(format!("@{}: Use of undefined variable `{}`", index, var).into());
                }
            }
        }

        Ok(())
    }
}
