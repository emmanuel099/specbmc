use crate::cex::ControlFlowGraph;
use std::fmt;

#[derive(Clone, Debug)]
pub struct CounterExample {
    control_flow_graph: ControlFlowGraph,
}

impl CounterExample {
    pub fn new(control_flow_graph: ControlFlowGraph) -> Self {
        Self { control_flow_graph }
    }

    pub fn control_flow_graph(&self) -> &ControlFlowGraph {
        &self.control_flow_graph
    }

    pub fn control_flow_graph_mut(&mut self) -> &mut ControlFlowGraph {
        &mut self.control_flow_graph
    }
}

impl fmt::Display for CounterExample {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.control_flow_graph().fmt(f)
    }
}
