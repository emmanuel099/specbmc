use crate::error::Result;
use crate::expr::{Expression, Sort};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum List {
    Nil,
    Insert,
    Head,
    Tail,
}

impl fmt::Display for List {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => write!(f, "nil"),
            Self::Insert => write!(f, "insert"),
            Self::Head => write!(f, "head"),
            Self::Tail => write!(f, "tail"),
        }
    }
}

impl List {
    pub fn nil(sort: Sort) -> Expression {
        Expression::new(Self::Nil.into(), vec![], sort)
    }

    pub fn insert(head: Expression, tail: Expression) -> Result<Expression> {
        tail.sort().expect_list()?;
        let domain = tail.sort().unwrap_list();
        head.sort().expect_sort(domain)?;

        let result_sort = tail.sort().clone();
        Ok(Expression::new(
            Self::Insert.into(),
            vec![head, tail],
            result_sort,
        ))
    }

    pub fn head(list: Expression) -> Result<Expression> {
        list.sort().expect_list()?;
        let domain = list.sort().unwrap_list();

        let result_sort = domain.clone();
        Ok(Expression::new(Self::Head.into(), vec![list], result_sort))
    }

    pub fn tail(list: Expression) -> Result<Expression> {
        list.sort().expect_list()?;

        let result_sort = list.sort().clone();
        Ok(Expression::new(Self::Tail.into(), vec![list], result_sort))
    }
}
