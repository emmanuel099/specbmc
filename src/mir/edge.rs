use falcon::graph;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Edge {
    head: usize,
    tail: usize,
}

impl Edge {
    pub fn new(head: usize, tail: usize) -> Self {
        Self { head, tail }
    }

    /// Retrieve the index of the head `Vertex` for this `Edge`.
    pub fn head(&self) -> usize {
        self.head
    }

    /// Retrieve the index of the tail `Vertex` for this `Edge`.
    pub fn tail(&self) -> usize {
        self.tail
    }
}

impl graph::Edge for Edge {
    fn head(&self) -> usize {
        self.head
    }

    fn tail(&self) -> usize {
        self.tail
    }

    fn dot_label(&self) -> String {
        String::default()
    }
}

impl fmt::Display for Edge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(0x{:X}->0x{:X})", self.head, self.tail)
    }
}
