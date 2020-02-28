use crate::error::Result;
use crate::expr::{Expression, Sort, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum PatternHistoryTable {
    Init,
    Taken,
    NotTaken,
}

impl fmt::Display for PatternHistoryTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Init => write!(f, "pht-init"),
            Self::Taken => write!(f, "pht-taken"),
            Self::NotTaken => write!(f, "pht-not-taken"),
        }
    }
}

impl PatternHistoryTable {
    pub fn variable() -> Variable {
        Variable::new("_pht", Sort::pattern_history_table())
    }

    pub fn variable_nonspec() -> Variable {
        Variable::new("_pht_ns", Sort::pattern_history_table())
    }

    pub fn init() -> Result<Expression> {
        Ok(Expression::new(
            Self::Init.into(),
            vec![],
            Sort::pattern_history_table(),
        ))
    }

    pub fn taken(pht: Expression, location: Expression) -> Result<Expression> {
        pht.sort().expect_pattern_history_table()?;
        location.sort().expect_bit_vector()?;

        Ok(Expression::new(
            Self::Taken.into(),
            vec![pht, location],
            Sort::pattern_history_table(),
        ))
    }

    pub fn not_taken(pht: Expression, location: Expression) -> Result<Expression> {
        pht.sort().expect_pattern_history_table()?;
        location.sort().expect_bit_vector()?;

        Ok(Expression::new(
            Self::NotTaken.into(),
            vec![pht, location],
            Sort::pattern_history_table(),
        ))
    }
}
