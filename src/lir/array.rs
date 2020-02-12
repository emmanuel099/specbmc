use crate::error::Result;
use crate::lir::{Expression, Operator};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Array {
    Select,
    Store,
}

impl Into<Operator> for Array {
    fn into(self) -> Operator {
        Operator::Array(self)
    }
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
            Array::Select.into(),
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
            Array::Store.into(),
            vec![arr, index, value],
            result_sort,
        ))
    }
}
