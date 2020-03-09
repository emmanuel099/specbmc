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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
    elements: BTreeMap<Constant, Constant>,
    default: Option<Constant>,
}

impl ArrayValue {
    pub fn new(default: Option<Constant>) -> Self {
        Self {
            elements: BTreeMap::new(),
            default,
        }
    }

    pub fn select(&self, index: &Constant) -> Option<&Constant> {
        self.elements.get(index).or(self.default.as_ref())
    }

    pub fn store(&mut self, index: Constant, value: Constant) {
        self.elements.insert(index, value);
    }
}

impl fmt::Display for ArrayValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        for (index, value) in &self.elements {
            write!(f, "{} ↦ {}, ", index, value)?;
        }
        if let Some(value) = &self.default {
            write!(f, "… ↦ {}", value)?;
        } else {
            write!(f, "… ↦ ?")?;
        }
        write!(f, "]")
    }
}
