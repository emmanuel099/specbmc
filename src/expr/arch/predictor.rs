use crate::environment;
use crate::error::Result;
use crate::expr::{Expression, Sort, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Predictor {
    TransientStart,
    MisPredict,
    SpeculationWindow,
}

impl fmt::Display for Predictor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::TransientStart => write!(f, "transient-start"),
            Self::MisPredict => write!(f, "mis-predict"),
            Self::SpeculationWindow => write!(f, "speculation-window"),
        }
    }
}

impl Predictor {
    pub fn variable() -> Variable {
        Variable::new("_predictor", Sort::predictor())
    }

    pub fn transient_start(predictor: Expression) -> Result<Expression> {
        predictor.sort().expect_predictor()?;

        Ok(Expression::new(
            Self::TransientStart.into(),
            vec![predictor],
            Sort::word(),
        ))
    }

    pub fn mis_predict(predictor: Expression, program_location: Expression) -> Result<Expression> {
        predictor.sort().expect_predictor()?;
        program_location.sort().expect_word()?;

        Ok(Expression::new(
            Self::MisPredict.into(),
            vec![predictor, program_location],
            Sort::boolean(),
        ))
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
}
