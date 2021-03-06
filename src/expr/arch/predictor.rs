use crate::environment;
use crate::error::Result;
use crate::expr::{Expression, Sort, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Predictor {
    SpeculationWindow,
    Speculate,
    Taken,
}

impl fmt::Display for Predictor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SpeculationWindow => write!(f, "speculation-window"),
            Self::Speculate => write!(f, "speculate"),
            Self::Taken => write!(f, "taken"),
        }
    }
}

impl Predictor {
    pub fn variable() -> Variable {
        Variable::new("_predictor", Sort::predictor())
    }

    pub fn speculation_window(
        predictor: Expression,
        program_location: Expression,
    ) -> Result<Expression> {
        predictor.sort().expect_predictor()?;
        program_location.sort().expect_word()?;

        Ok(Expression::new(
            Self::SpeculationWindow.into(),
            vec![predictor, program_location],
            Sort::bit_vector(environment::SPECULATION_WINDOW_SIZE),
        ))
    }

    pub fn speculate(predictor: Expression, program_location: Expression) -> Result<Expression> {
        predictor.sort().expect_predictor()?;
        program_location.sort().expect_word()?;

        Ok(Expression::new(
            Self::Speculate.into(),
            vec![predictor, program_location],
            Sort::boolean(),
        ))
    }

    pub fn taken(predictor: Expression, program_location: Expression) -> Result<Expression> {
        predictor.sort().expect_predictor()?;
        program_location.sort().expect_word()?;

        Ok(Expression::new(
            Self::Taken.into(),
            vec![predictor, program_location],
            Sort::boolean(),
        ))
    }
}
