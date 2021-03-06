use crate::error::Result;
use crate::expr::{Expression, Sort, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum BranchTargetBuffer {
    Track,
}

impl fmt::Display for BranchTargetBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Track => write!(f, "btb-track"),
        }
    }
}

impl BranchTargetBuffer {
    pub fn variable() -> Variable {
        let mut var = Variable::new("_btb", Sort::branch_target_buffer());
        var.set_rollback_persistent(true);
        var
    }

    pub fn track(btb: Expression, location: Expression, target: Expression) -> Result<Expression> {
        btb.sort().expect_branch_target_buffer()?;
        location.sort().expect_word()?;
        target.sort().expect_word()?;

        Ok(Expression::new(
            Self::Track.into(),
            vec![btb, location, target],
            Sort::branch_target_buffer(),
        ))
    }
}
