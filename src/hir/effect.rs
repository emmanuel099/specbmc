use crate::expr::{Cache, Expression, Variable};
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

    pub fn is_cache_fetch(&self) -> bool {
        match self {
            Self::CacheFetch { .. } => true,
            _ => false,
        }
    }

    /// Get each `Variable` read by this `Effect`.
    pub fn variables_read(&self) -> Vec<&Variable> {
        match self {
            Self::CacheFetch { cache, address, .. } => vec![cache]
                .into_iter()
                .chain(address.variables().into_iter())
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
        }
    }

    /// Get a Vec of the `Variable`s written by this `Effect`
    pub fn variables_written(&self) -> Vec<&Variable> {
        match self {
            Self::CacheFetch { new_cache, .. } => vec![new_cache],
        }
    }

    /// Get a Vec of mutable referencer to the `Variable`s written by this `Effect`
    pub fn variables_written_mut(&mut self) -> Vec<&mut Variable> {
        match self {
            Self::CacheFetch { new_cache, .. } => vec![new_cache],
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
        }
    }
}
