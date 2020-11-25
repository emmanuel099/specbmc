use crate::hir::{Block, Function, Instruction, Operation, Program};
use falcon::graph;
use std::convert::TryInto;

/// Computes the call graph of the program.
/// Indirect calls are omitted.
pub fn call_graph(program: &Program) -> CallGraph {
    let mut call_graph = CallGraph::new();

    for func in program.functions() {
        call_graph
            .insert_vertex(FunctionInfo {
                address: func.address(),
                name: func.name().unwrap_or_default().to_owned(),
            })
            .unwrap();
    }

    for func in program.functions() {
        let mut call_targets = function_direct_call_targets(func);

        // De-duplicate targets to avoid edge insertion conflicts
        call_targets.sort_unstable();
        call_targets.dedup();

        for target in call_targets {
            if program.function_by_address(target).is_some() {
                call_graph
                    .insert_edge(Call {
                        caller_address: func.address(),
                        callee_address: target,
                    })
                    .unwrap();
            }
        }
    }

    call_graph
}

#[derive(Clone, Debug)]
pub struct FunctionInfo {
    address: u64,
    name: String,
}

impl FunctionInfo {
    pub fn address(&self) -> u64 {
        self.address
    }
}

impl graph::Vertex for FunctionInfo {
    fn index(&self) -> usize {
        self.address.try_into().unwrap()
    }

    fn dot_label(&self) -> String {
        format!("0x{:X}: {}", self.address, self.name)
    }
}

#[derive(Clone, Debug)]
pub struct Call {
    caller_address: u64,
    callee_address: u64,
}

impl Call {
    pub fn caller_address(&self) -> u64 {
        self.caller_address
    }

    pub fn callee_address(&self) -> u64 {
        self.callee_address
    }
}

impl graph::Edge for Call {
    fn head(&self) -> usize {
        self.caller_address.try_into().unwrap()
    }

    fn tail(&self) -> usize {
        self.callee_address.try_into().unwrap()
    }

    fn dot_label(&self) -> String {
        String::default()
    }
}

pub type CallGraph = graph::Graph<FunctionInfo, Call>;

fn function_direct_call_targets(function: &Function) -> Vec<u64> {
    function
        .control_flow_graph()
        .blocks()
        .into_iter()
        .map(block_direct_call_targets)
        .flatten()
        .collect()
}

fn block_direct_call_targets(block: &Block) -> Vec<u64> {
    block
        .instructions()
        .iter()
        .filter_map(instruction_direct_call_targets)
        .collect()
}

fn instruction_direct_call_targets(inst: &Instruction) -> Option<u64> {
    match inst.operation() {
        Operation::Call { target } => target.try_into().ok(),
        _ => None,
    }
}
