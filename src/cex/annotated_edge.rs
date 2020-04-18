use crate::cex::AnnotatedElement;
use crate::hir::Edge;
use falcon::graph;
use std::fmt;

#[derive(Clone, Debug)]
pub struct Annotation {
    /// Is this edge executed?
    executed: bool,
}

impl Annotation {
    /// Marks this `Edge` as executed.
    pub fn mark_as_executed(&mut self) {
        self.executed = true;
    }

    /// Returns whether this `Edge` is executed.
    pub fn executed(&self) -> bool {
        self.executed
    }
}

impl Default for Annotation {
    fn default() -> Self {
        Self { executed: false }
    }
}

pub type AnnotatedEdge = AnnotatedElement<Edge, Annotation>;

impl AnnotatedEdge {
    /// Returns the actual `Edge`.
    pub fn edge(&self) -> &Edge {
        &self.element
    }

    /// Returns whether this `Edge` is executed in any composition.
    pub fn executed(&self) -> bool {
        self.annotations
            .iter()
            .any(|(_, annotation)| annotation.executed())
    }
}

impl graph::Edge for AnnotatedEdge {
    fn head(&self) -> usize {
        self.edge().head()
    }

    fn tail(&self) -> usize {
        self.edge().tail()
    }

    fn dot_label(&self) -> String {
        self.edge().dot_label()
    }

    fn dot_font_color(&self) -> String {
        if self.executed() {
            "#343434ff".to_string()
        } else {
            "#34343455".to_string()
        }
    }

    fn dot_fill_color(&self) -> String {
        self.annotations
            .iter()
            .filter(|(_, annotation)| annotation.executed())
            .map(|(composition, _)| composition.color())
            .next() // can be a single color ...
            .unwrap_or("#00000055")
            .to_string()
    }

    fn dot_pen_width(&self) -> f64 {
        if self.executed() {
            8.5
        } else {
            1.0
        }
    }
}

impl fmt::Display for AnnotatedEdge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.edge())
    }
}
