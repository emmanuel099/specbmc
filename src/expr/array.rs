use crate::error::Result;
use crate::expr::{Constant, Expression};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Array {
    Select,
    Store,
}

impl fmt::Display for Array {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Select => write!(f, "select"),
            Self::Store => write!(f, "store"),
        }
    }
}

impl Array {
    pub fn select(arr: Expression, index: Expression) -> Result<Expression> {
        arr.sort().expect_array()?;
        let (range, domain) = arr.sort().unwrap_array();
        index.sort().expect_sort(range)?;

        let result_sort = domain.clone();
        Ok(Expression::new(
            Self::Select.into(),
            vec![arr, index],
            result_sort,
        ))
    }

    pub fn store(arr: Expression, index: Expression, value: Expression) -> Result<Expression> {
        arr.sort().expect_array()?;
        let (range, domain) = arr.sort().unwrap_array();
        index.sort().expect_sort(range)?;
        value.sort().expect_sort(domain)?;

        let result_sort = arr.sort().clone();
        Ok(Expression::new(
            Self::Store.into(),
            vec![arr, index, value],
            result_sort,
        ))
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct ArrayValue {
    entries: BTreeMap<Constant, Constant>,
    default_value: Option<Constant>,
}

impl ArrayValue {
    pub fn new(default_value: Option<Constant>) -> Self {
        Self {
            entries: BTreeMap::new(),
            default_value,
        }
    }

    pub fn entries(&self) -> &BTreeMap<Constant, Constant> {
        &self.entries
    }

    pub fn default_value(&self) -> Option<&Constant> {
        self.default_value.as_ref()
    }

    pub fn select(&self, index: &Constant) -> Option<&Constant> {
        self.entries
            .get(index)
            .or_else(|| self.default_value.as_ref())
    }

    pub fn store(&mut self, index: Constant, value: Constant) {
        self.entries.insert(index, value);
    }
}

impl fmt::Display for ArrayValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (index, value) in &self.entries {
            write!(f, "{} ↦ {}, ", index, value)?;
        }
        if let Some(value) = &self.default_value {
            write!(f, "… ↦ {}", value)?;
        } else {
            write!(f, "… ↦ ?")?;
        }
        write!(f, "]")
    }
}
