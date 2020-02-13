//! `PhiNode` represents a phi node in the SSA form

use crate::expr::Variable;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct PhiNode {
    incoming: BTreeMap<usize, Variable>, // Input from another block
    out: Variable,
}

impl PhiNode {
    pub fn new(out: Variable) -> Self {
        Self {
            incoming: BTreeMap::new(),
            out,
        }
    }

    pub fn add_incoming(&mut self, src: Variable, block_index: usize) {
        self.incoming.insert(block_index, src);
    }

    pub fn incoming_variable(&self, block_index: usize) -> Option<&Variable> {
        self.incoming.get(&block_index)
    }

    pub fn incoming_variable_mut(&mut self, block_index: usize) -> Option<&mut Variable> {
        self.incoming.get_mut(&block_index)
    }

    pub fn out(&self) -> &Variable {
        &self.out
    }

    pub fn out_mut(&mut self) -> &mut Variable {
        &mut self.out
    }
}

impl fmt::Display for PhiNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} = phi", self.out)?;
        for (block_index, src) in &self.incoming {
            write!(f, " [{}, 0x{:X}]", src, block_index)?
        }
        Ok(())
    }
}
