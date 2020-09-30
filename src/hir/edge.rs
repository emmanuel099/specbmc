use crate::expr::{Expression, Variable};
use bitflags::bitflags;
use falcon::graph;
use std::fmt;

bitflags! {
    #[derive(Default)]
    pub struct Labels: u32 {
        const TAKEN     = 0b00001;
        const SPECULATE = 0b00010;
        const ROLLBACK  = 0b00100;
        const CALL      = 0b01000;
        const RETURN    = 0b10000;
    }
}

impl Labels {
    pub fn taken(&mut self) -> &mut Self {
        *self |= Labels::TAKEN;
        self
    }

    pub fn is_taken(&self) -> bool {
        self.contains(Labels::TAKEN)
    }

    pub fn speculate(&mut self) -> &mut Self {
        *self |= Labels::SPECULATE;
        self
    }

    pub fn is_speculate(&self) -> bool {
        self.contains(Labels::SPECULATE)
    }

    pub fn rollback(&mut self) -> &mut Self {
        *self |= Labels::ROLLBACK;
        self
    }

    pub fn is_rollback(&self) -> bool {
        self.contains(Labels::ROLLBACK)
    }

    pub fn call(&mut self) -> &mut Self {
        *self |= Labels::CALL;
        self
    }

    pub fn is_call(&self) -> bool {
        self.contains(Labels::CALL)
    }

    pub fn r#return(&mut self) -> &mut Self {
        *self |= Labels::RETURN;
        self
    }

    pub fn is_return(&self) -> bool {
        self.contains(Labels::RETURN)
    }

    pub fn merge(&mut self, other: &Labels) {
        *self |= *other;
    }
}

impl fmt::Display for Labels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return Ok(());
        }
        write!(f, "[")?;
        let mut is_first = true;
        if self.is_taken() {
            write!(f, "taken")?;
            is_first = false;
        }
        if self.is_speculate() {
            if !is_first {
                write!(f, ", ")?;
            }
            write!(f, "speculate")?;
            is_first = false;
        }
        if self.is_rollback() {
            if !is_first {
                write!(f, ", ")?;
            }
            write!(f, "rollback")?;
            is_first = false;
        }
        if self.is_call() {
            if !is_first {
                write!(f, ", ")?;
            }
            write!(f, "call")?;
            is_first = false;
        }
        if self.is_return() {
            if !is_first {
                write!(f, ", ")?;
            }
            write!(f, "return")?;
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

    /// Get the variables read by this `Edge`.
    pub fn variables_read(&self) -> Vec<&Variable> {
        if let Some(condition) = &self.condition {
            condition.variables()
        } else {
            Vec::default()
        }
    }

    /// Get a mutable reference to the variables read by this `Edge`.
    pub fn variables_read_mut(&mut self) -> Vec<&mut Variable> {
        if let Some(condition) = &mut self.condition {
            condition.variables_mut()
        } else {
            Vec::default()
        }
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
