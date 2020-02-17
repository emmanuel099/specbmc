use crate::error::Result;
use crate::expr::{Expression, Operator, Sort, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum BranchTargetBuffer {
    Track,
}

impl Into<Operator> for BranchTargetBuffer {
    fn into(self) -> Operator {
        Operator::BranchTargetBuffer(self)
    }
}

impl fmt::Display for BranchTargetBuffer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Track => write!(f, "btb-track"),
        }
    }
}

impl BranchTargetBuffer {
    pub fn variable() -> Variable {
        Variable::new("_btb", Sort::branch_target_buffer())
    }

    pub fn track(btb: Expression, location: Expression, target: Expression) -> Result<Expression> {
        btb.sort().expect_branch_target_buffer()?;
        location.sort().expect_bit_vector()?;
        target.sort().expect_bit_vector()?;

        Ok(Expression::new(
            Self::Track.into(),
            vec![btb, location, target],
            Sort::branch_target_buffer(),
        ))
    }
}