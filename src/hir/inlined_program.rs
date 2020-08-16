use crate::hir::ControlFlowGraph;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct InlinedProgram {
    control_flow_graph: ControlFlowGraph,
}

impl InlinedProgram {
    pub fn new(control_flow_graph: ControlFlowGraph) -> Self {
        Self { control_flow_graph }
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

impl fmt::Display for InlinedProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.control_flow_graph().fmt(f)
    }
}
