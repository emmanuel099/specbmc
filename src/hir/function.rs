use crate::hir::ControlFlowGraph;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Function {
    address: u64,
    name: Option<String>,
    control_flow_graph: ControlFlowGraph,
}

impl Function {
    pub fn new(address: u64, name: Option<String>, control_flow_graph: ControlFlowGraph) -> Self {
        Self {
            address,
            name,
            control_flow_graph,
        }
    }

    pub fn address(&self) -> u64 {
        self.address
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn control_flow_graph(&self) -> &ControlFlowGraph {
        &self.control_flow_graph
    }

    pub fn set_control_flow_graph(&mut self, cfg: ControlFlowGraph) {
        self.control_flow_graph = cfg
    }

    pub fn control_flow_graph_mut(&mut self) -> &mut ControlFlowGraph {
        &mut self.control_flow_graph
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:X}", self.address)?;
        if let Some(name) = &self.name {
            write!(f, ": {}", name)?;
        }
        Ok(())
    }
}
