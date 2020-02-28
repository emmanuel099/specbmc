use crate::error::Result;
use crate::expr::{Expression, Sort, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum BranchTargetBuffer {
    Init,
    Track,
}

impl fmt::Display for BranchTargetBuffer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Init => write!(f, "btb-init"),
            Self::Track => write!(f, "btb-track"),
        }
    }
}

impl BranchTargetBuffer {
    pub fn variable() -> Variable {
        Variable::new("_btb", Sort::branch_target_buffer())
    }

    pub fn variable_nonspec() -> Variable {
        Variable::new("_btb_ns", Sort::branch_target_buffer())
    }

    pub fn init() -> Result<Expression> {
        Ok(Expression::new(
            Self::Init.into(),
            vec![],
            Sort::branch_target_buffer(),
        ))
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
