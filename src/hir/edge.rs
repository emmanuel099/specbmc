use crate::expr::Expression;
use falcon::graph;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum EdgeLabel {
    Taken,
    Speculate,
    Rollback,
}

impl fmt::Display for EdgeLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Taken => write!(f, "taken"),
            Self::Speculate => write!(f, "speculate"),
            Self::Rollback => write!(f, "rollback"),
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct EdgeLabels {
    labels: BTreeSet<EdgeLabel>,
}

impl EdgeLabels {
    pub fn taken(&mut self) -> &mut Self {
        self.labels.insert(EdgeLabel::Taken);
        self
    }

    pub fn is_taken(&self) -> bool {
        self.labels.contains(&EdgeLabel::Taken)
    }

    pub fn speculate(&mut self) -> &mut Self {
        self.labels.insert(EdgeLabel::Speculate);
        self
    }

    pub fn is_speculate(&self) -> bool {
        self.labels.contains(&EdgeLabel::Speculate)
    }

    pub fn rollback(&mut self) -> &mut Self {
        self.labels.insert(EdgeLabel::Rollback);
        self
    }

    pub fn is_rollback(&self) -> bool {
        self.labels.contains(&EdgeLabel::Rollback)
    }
}

impl Default for EdgeLabels {
    fn default() -> Self {
        Self {
            labels: BTreeSet::default(),
        }
    }
}

impl fmt::Display for EdgeLabels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.labels.is_empty() {
            return Ok(());
        }
        write!(f, "[")?;
        let mut is_first = true;
        for label in &self.labels {
            if !is_first {
                write!(f, ", ")?;
            }
            write!(f, "{}", label)?;
            is_first = false;
        }
        write!(f, "]")
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Edge {
    head: usize,
    tail: usize,
    condition: Option<Expression>,
    labels: EdgeLabels,
}

impl Edge {
    pub fn new(head: usize, tail: usize, condition: Option<Expression>) -> Self {
        Self {
            head,
            tail,
            condition,
            labels: EdgeLabels::default(),
        }
    }

    /// Clone this `Edge` and set a new head and tail.
    pub fn clone_new_head_tail(&self, head: usize, tail: usize) -> Self {
        let mut clone = self.clone();
        clone.head = head;
        clone.tail = tail;
        clone
    }

    /// Retrieve the condition for this `Edge`.
    pub fn condition(&self) -> Option<&Expression> {
        self.condition.as_ref()
    }

    /// Retrieve a mutable reference to the condition for this `Edge`
    pub fn condition_mut(&mut self) -> Option<&mut Expression> {
        self.condition.as_mut()
    }

    /// Retrieve the labels of this `Edge`.
    pub fn labels(&self) -> &EdgeLabels {
        &self.labels
    }

    /// Retrieve a mutable reference to the labels of this `Edge`.
    pub fn labels_mut(&mut self) -> &mut EdgeLabels {
        &mut self.labels
    }

    /// Retrieve the index of the head `Vertex` for this `Edge`.
    pub fn head(&self) -> usize {
        self.head
    }

    /// Retrieve the index of the tail `Vertex` for this `Edge`.
    pub fn tail(&self) -> usize {
        self.tail
    }

    /// Returns whether this `Edge` is conditional or not.
    pub fn is_conditional(&self) -> bool {
        self.condition.is_some()
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
        let mut label = format!("{}", self.labels);
        if let Some(condition) = &self.condition {
            if !label.is_empty() {
                label.push_str("\n");
            }
            label.push_str(&format!("{}", condition));
        }
        label
    }
}

impl fmt::Display for Edge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(0x{:X}->0x{:X})", self.head, self.tail)?;
        if let Some(ref condition) = self.condition {
            write!(f, " ? ({})", condition)?;
        }
        Ok(())
    }
}
