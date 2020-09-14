use crate::expr::Sort;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum Label {
    RollbackPersistent,
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RollbackPersistent => write!(f, "rollback-persistent"),
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Default)]
pub struct Labels {
    labels: BTreeSet<Label>,
}

impl Labels {
    pub fn rollback_persistent(&mut self) -> &mut Self {
        self.labels.insert(Label::RollbackPersistent);
        self
    }

    /// Returns whether this `Variable` survives a transient-execution rollback or not.
    pub fn is_rollback_persistent(&self) -> bool {
        self.labels.contains(&Label::RollbackPersistent)
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
pub struct Variable {
    name: String,
    sort: Box<Sort>,
    version: Option<usize>,     // Version in SSA form
    composition: Option<usize>, // Composition Number when self-composed
    labels: Labels,
}

impl Variable {
    /// Create a new `Variable` with the given name and sort.
    pub fn new<S>(name: S, sort: Sort) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            sort: Box::new(sort),
            version: None,
            composition: None,
            labels: Labels::default(),
        }
    }

    /// Gets the name of the `Variable`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the sort of the `Variable`.
    pub fn sort(&self) -> &Sort {
        &self.sort
    }

    // Gets the SSA version of the `Variable` or None if no SSA version is set.
    pub fn version(&self) -> Option<usize> {
        self.version
    }

    // Sets the SSA version of the `Variable`.
    pub fn set_version(&mut self, version: Option<usize>) {
        self.version = version;
    }

    // Gets the composition number of the `Variable` or None if not self-composed.
    pub fn composition(&self) -> Option<usize> {
        self.composition
    }

    // Sets the composition number of the `Variable`.
    pub fn set_composition(&mut self, composition: Option<usize>) {
        self.composition = composition;
    }

    /// An identifier for the `Variable`.
    pub fn identifier(&self) -> String {
        let version_str = match self.version() {
            Some(version) => format!(".{}", version),
            None => String::default(),
        };
        let composition_str = match self.composition() {
            Some(composition) => format!("@{}", composition),
            None => String::default(),
        };
        format!("{}{}{}", self.name, version_str, composition_str)
    }

    /// Retrieve the labels of this `Edge`.
    pub fn labels(&self) -> &Labels {
        &self.labels
    }

    /// Retrieve a mutable reference to the labels of this `Edge`.
    pub fn labels_mut(&mut self) -> &mut Labels {
        &mut self.labels
    }

    /// Returns a copy of the variable with the composition number set to `composition`.
    pub fn self_compose(&self, composition: usize) -> Self {
        let mut var = self.clone();
        var.set_composition(Some(composition));
        var
    }
}

impl fmt::Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if cfg!(debug_assertions) {
            write!(f, "{}:{}", self.identifier(), self.sort())
        } else {
            write!(f, "{}", self.identifier())
        }
    }
}
