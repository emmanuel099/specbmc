use crate::expr::Sort;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Variable {
    name: String,
    sort: Box<Sort>,
    version: Option<usize>,     // Version in SSA form
    composition: Option<usize>, // Composition Number when self-composed
    rollback_persistent: bool,
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
            rollback_persistent: false,
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
    pub fn is_rollback_persistent(&self) -> bool {
        self.rollback_persistent
    }

    /// Retrieve a mutable reference to the labels of this `Edge`.
    pub fn set_rollback_persistent(&mut self, rollback_persistent: bool) {
        self.rollback_persistent = rollback_persistent;
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
            if self.is_rollback_persistent() {
                write!(f, "[rollback-persistent] ")?;
            }
            write!(f, "{}:{}", self.identifier(), self.sort())
        } else {
            write!(f, "{}", self.identifier())
        }
    }
}
