//! SpecBMC HIR
//!
//! The design of HIR has been heavily inspired by Falcon IL (https://github.com/falconre/falcon).
//! Major parts of ControlFlowGraph and SSA-Transformation have been copied and adapted from Falcon
//! (see copyright block of each of those files).

pub mod analysis;
mod block;
mod control_flow_graph;
mod edge;
mod effect;
mod function;
mod inlined_program;
mod instruction;
mod operation;
mod phi_node;
mod program;
pub mod transformation;
mod translation;

pub use self::block::Block;
pub use self::control_flow_graph::{ControlFlowGraph, RemovedEdgeGuard};
pub use self::edge::Edge;
pub use self::effect::Effect;
pub use self::function::Function;
pub use self::inlined_program::InlinedProgram;
pub use self::instruction::Instruction;
pub use self::operation::Operation;
pub use self::phi_node::PhiNode;
pub use self::program::{Program, ProgramEntry};
