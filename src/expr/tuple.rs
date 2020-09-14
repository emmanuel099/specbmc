use crate::error::Result;
use crate::expr::{Expression, Sort};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Tuple {
    Make,
    Get(usize),
}

impl fmt::Display for Tuple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Make => write!(f, "tuple"),
            Self::Get(field) => write!(f, "get-{}", field),
        }
    }
}

impl Tuple {
    pub fn make(values: Vec<Expression>) -> Result<Expression> {
        let sorts: Vec<Sort> = values.iter().map(|val| val.sort().clone()).collect();

        let result_sort = Sort::tuple(sorts);
        Ok(Expression::new(Self::Make.into(), values, result_sort))
    }

    pub fn get(tuple: Expression, index: usize) -> Result<Expression> {
        tuple.sort().expect_tuple()?;
        let fields = tuple.sort().unwrap_tuple();
        if let Some(field_sort) = fields.get(index) {
            let result_sort = field_sort.clone();
            Ok(Expression::new(
                Self::Get(index).into(),
                vec![tuple],
                result_sort,
            ))
        } else {
            Err(format!("Field with index {} doesn't exist in tuple", index).into())
        }
    }
}
