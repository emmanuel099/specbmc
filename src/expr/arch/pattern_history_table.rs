use crate::error::Result;
use crate::expr::{Expression, Sort, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum PatternHistoryTable {
    Taken,
    NotTaken,
}

impl fmt::Display for PatternHistoryTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Taken => write!(f, "pht-taken"),
            Self::NotTaken => write!(f, "pht-not-taken"),
        }
    }
}

impl PatternHistoryTable {
    pub fn variable() -> Variable {
        Variable::new("_pht", Sort::pattern_history_table())
    }

    pub fn taken(pht: Expression, location: Expression) -> Result<Expression> {
        pht.sort().expect_pattern_history_table()?;
        location.sort().expect_word()?;

        Ok(Expression::new(
            Self::Taken.into(),
            vec![pht, location],
            Sort::pattern_history_table(),
        ))
    }

    pub fn not_taken(pht: Expression, location: Expression) -> Result<Expression> {
        pht.sort().expect_pattern_history_table()?;
        location.sort().expect_word()?;

        Ok(Expression::new(
            Self::NotTaken.into(),
            vec![pht, location],
            Sort::pattern_history_table(),
        ))
    }
}
