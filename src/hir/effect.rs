use crate::expr::{BranchTargetBuffer, Cache, Expression, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Effect {
    /// Memory at given address is fetched into the Cache
    CacheFetch {
        new_cache: Variable,
        cache: Variable,
        address: Expression,
        bit_width: usize,
    },
    /// Branch target is tracked in the Branch Target Buffer
    BranchTarget {
        new_btb: Variable,
        btb: Variable,
        condition: Option<Expression>,
        location: Expression,
        target: Expression,
    },
}

impl Effect {
    /// Create a new `Effect::CacheFetch`.
    pub fn cache_fetch(address: Expression, bit_width: usize) -> Self {
        Self::CacheFetch {
            new_cache: Cache::variable(),
            cache: Cache::variable(),
            address,
            bit_width,
        }
    }

    /// Create a new unconditional `Effect::BranchTarget`.
    pub fn unconditional_branch_target(location: Expression, target: Expression) -> Self {
        Self::BranchTarget {
            new_btb: BranchTargetBuffer::variable(),
            btb: BranchTargetBuffer::variable(),
            condition: None,
            location,
            target,
        }
    }

    /// Create a new conditional `Effect::BranchTarget`.
    pub fn conditional_branch_target(
        condition: Expression,
        location: Expression,
        target: Expression,
    ) -> Self {
        Self::BranchTarget {
            new_btb: BranchTargetBuffer::variable(),
            btb: BranchTargetBuffer::variable(),
            condition: Some(condition),
            location,
            target,
        }
    }

    /// Get each `Variable` read by this `Effect`.
    pub fn variables_read(&self) -> Vec<&Variable> {
        match self {
            Self::CacheFetch { cache, address, .. } => vec![cache]
                .into_iter()
                .chain(address.variables().into_iter())
                .collect(),
            Self::BranchTarget {
                btb,
                condition,
                location,
                target,
                ..
            } => vec![btb]
                .into_iter()
                .chain(
                    match condition {
                        Some(condition) => condition.variables(),
                        None => Vec::default(),
                    }
                    .into_iter(),
                )
                .chain(location.variables().into_iter())
                .chain(target.variables().into_iter())
                .collect(),
        }
    }

    /// Get a mutable reference to each `Variable` read by this `Effect`.
    pub fn variables_read_mut(&mut self) -> Vec<&mut Variable> {
        match self {
            Self::CacheFetch { cache, address, .. } => vec![cache]
                .into_iter()
                .chain(address.variables_mut().into_iter())
                .collect(),
            Self::BranchTarget {
                btb,
                condition,
                location,
                target,
                ..
            } => vec![btb]
                .into_iter()
                .chain(
                    match condition {
                        Some(condition) => condition.variables_mut(),
                        None => Vec::default(),
                    }
                    .into_iter(),
                )
                .chain(location.variables_mut().into_iter())
                .chain(target.variables_mut().into_iter())
                .collect(),
        }
    }

    /// Get a Vec of the `Variable`s written by this `Effect`
    pub fn variables_written(&self) -> Vec<&Variable> {
        match self {
            Self::CacheFetch { new_cache, .. } => vec![new_cache],
            Self::BranchTarget { new_btb, .. } => vec![new_btb],
        }
    }

    /// Get a Vec of mutable referencer to the `Variable`s written by this `Effect`
    pub fn variables_written_mut(&mut self) -> Vec<&mut Variable> {
        match self {
            Self::CacheFetch { new_cache, .. } => vec![new_cache],
            Self::BranchTarget { new_btb, .. } => vec![new_btb],
        }
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::CacheFetch {
                new_cache,
                cache,
                address,
                bit_width,
            } => write!(
                f,
                "{} = cache_fetch({}, {}, {})",
                new_cache, cache, address, bit_width
            ),
            Self::BranchTarget {
                new_btb,
                btb,
                condition,
                location,
                target,
            } => {
                write!(
                    f,
                    "{} = branch_target({}, {}, {})",
                    new_btb, btb, location, target
                )?;
                if let Some(cond) = condition {
                    write!(f, " if {}", cond)?;
                }
                Ok(())
            }
        }
    }
}
