//! SpecBMC LIR

mod node;
pub mod optimization;
mod program;
pub mod transformation;

pub use self::node::Node;
pub use self::program::Program;
