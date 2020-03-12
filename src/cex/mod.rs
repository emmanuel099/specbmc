use std::collections::BTreeMap;
use std::fmt;

mod annotated_block;
mod annotated_edge;
mod annotated_instruction;
mod cex_builder;
mod control_flow_graph;
mod counter_example;
mod effect;

pub use self::annotated_block::AnnotatedBlock;
pub use self::annotated_edge::AnnotatedEdge;
pub use self::annotated_instruction::AnnotatedInstruction;
pub use self::cex_builder::build_counter_example;
pub use self::control_flow_graph::ControlFlowGraph;
pub use self::counter_example::CounterExample;
pub use self::effect::Effect;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Composition {
    A = 1,
    B = 2,
}

impl Composition {
    pub fn number(self) -> usize {
        self as usize
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Self::A => "#ed403cff",
            Self::B => "#0465b2ff",
        }
    }
}

impl fmt::Display for Composition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Clone, Debug)]
pub struct AnnotatedElement<Element, Annotation> {
    /// The underlying element.
    element: Element,
    /// Is this edge executed in composition A?
    annotations: BTreeMap<Composition, Annotation>,
}

impl<Element, Annotation: Default> AnnotatedElement<Element, Annotation> {
    pub fn new(element: Element) -> Self {
        Self {
            element,
            annotations: BTreeMap::new(),
        }
    }

    pub fn annotation(&self, composition: &Composition) -> Option<&Annotation> {
        self.annotations.get(composition)
    }

    pub fn annotation_mut(&mut self, composition: Composition) -> &mut Annotation {
        self.annotations.entry(composition).or_default()
    }
}
