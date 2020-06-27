//! Low-Level Intermediate Representation (LIR)
//!
//! This intermediate representation is very simple and close to SMT.
//! It only consists of variable bindings, assertions and assumptions,
//! which makes the SMT encoding of LIR relatively easy and efficient.

mod node;
pub mod optimization;
mod program;

pub use self::node::Node;
pub use self::program::Program;
