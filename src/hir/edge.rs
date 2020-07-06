use crate::expr::Expression;
use falcon::graph;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum Label {
    Taken,
    Speculate,
    Rollback,
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Taken => write!(f, "taken"),
            Self::Speculate => write!(f, "speculate"),
            Self::Rollback => write!(f, "rollback"),
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Default)]
pub struct Labels {
    labels: BTreeSet<Label>,
}

impl Labels {
    pub fn taken(&mut self) -> &mut Self {
        self.labels.insert(Label::Taken);
        self
    }

    pub fn is_taken(&self) -> bool {
        self.labels.contains(&Label::Taken)
    }

    pub fn speculate(&mut self) -> &mut Self {
        self.labels.insert(Label::Speculate);
        self
    }

    pub fn is_speculate(&self) -> bool {
        self.labels.contains(&Label::Speculate)
    }

    pub fn rollback(&mut self) -> &mut Self {
        self.labels.insert(Label::Rollback);
        self
    }

    pub fn is_rollback(&self) -> bool {
        self.labels.contains(&Label::Rollback)
    }

    pub fn merge(&mut self, other: &Labels) {
        other.labels.iter().for_each(|&label| {
            self.labels.insert(label);
        });
    }
}

impl fmt::Display for Labels {
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
    labels: Labels,
}

impl Edge {
    pub fn new(head: usize, tail: usize, condition: Option<Expression>) -> Self {
        Self {
            head,
            tail,
            condition,
            labels: Labels::default(),
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

    /// Sets the condition of this `Edge`.
    pub fn set_condition(&mut self, condition: Option<Expression>) {
        self.condition = condition;
    }

    /// Retrieve the labels of this `Edge`.
    pub fn labels(&self) -> &Labels {
        &self.labels
    }

    /// Retrieve a mutable reference to the labels of this `Edge`.
    pub fn labels_mut(&mut self) -> &mut Labels {
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
