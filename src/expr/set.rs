use crate::error::Result;
use crate::expr::{Expression, Operator, Sort};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Set {
    Insert,
    Remove,
    Contains,
}

impl Into<Operator> for Set {
    fn into(self) -> Operator {
        Operator::Set(self)
    }
}

impl fmt::Display for Set {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Insert => write!(f, "insert"),
            Self::Remove => write!(f, "remove"),
            Self::Contains => write!(f, "contains"),
        }
    }
}

impl Set {
    pub fn insert(set: Expression, value: Expression) -> Result<Expression> {
        set.sort().expect_set()?;
        let range = set.sort().unwrap_set();
        value.sort().expect_sort(range)?;

        let result_sort = set.sort().clone();
        Ok(Expression::new(
            Set::Insert.into(),
            vec![set, value],
            result_sort,
        ))
    }

    pub fn remove(set: Expression, value: Expression) -> Result<Expression> {
        set.sort().expect_set()?;
        let range = set.sort().unwrap_set();
        value.sort().expect_sort(range)?;

        let result_sort = set.sort().clone();
        Ok(Expression::new(
            Set::Remove.into(),
            vec![set, value],
            result_sort,
        ))
    }

    pub fn contains(set: Expression, value: Expression) -> Result<Expression> {
        set.sort().expect_set()?;
        let range = set.sort().unwrap_set();
        value.sort().expect_sort(range)?;

        Ok(Expression::new(
            Set::Contains.into(),
            vec![set, value],
            Sort::boolean(),
        ))
    }
}
