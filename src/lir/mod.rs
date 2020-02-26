//! SpecBMC LIR

mod node;
pub mod optimization;
mod program;
mod validate;

pub use self::node::Node;
pub use self::program::Program;
pub use self::validate::validate_program;
