use crate::error::Result;
use crate::expr::{Expression, Operator, Sort, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Predictor {
    MisPredict,
    SpeculationWindow,
}

impl Into<Operator> for Predictor {
    fn into(self) -> Operator {
        Operator::Predictor(self)
    }
}

impl fmt::Display for Predictor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::MisPredict => write!(f, "mis-predict"),
            Self::SpeculationWindow => write!(f, "speculation-window"),
        }
    }
}

impl Predictor {
    pub fn variable() -> Variable {
        Variable::new("_predictor", Sort::predictor())
    }

    pub fn mis_predict(predictor: Expression, program_location: Expression) -> Result<Expression> {
        predictor.sort().expect_predictor()?;
        program_location.sort().expect_bit_vector()?;

        Ok(Expression::new(
            Predictor::MisPredict.into(),
            vec![predictor, program_location],
            Sort::boolean(),
        ))
    }

    pub fn speculation_window(
        predictor: Expression,
        program_location: Expression,
    ) -> Result<Expression> {
        predictor.sort().expect_predictor()?;
        program_location.sort().expect_bit_vector()?;

        Ok(Expression::new(
            Predictor::SpeculationWindow.into(),
            vec![predictor, program_location],
            Sort::integer(),
        ))
    }
}
