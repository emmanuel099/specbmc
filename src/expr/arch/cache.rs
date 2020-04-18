use crate::error::Result;
use crate::expr::{Expression, Sort, Variable};
use crate::util::CompactIterator;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Cache {
    Fetch(usize), // Fetch N bits into the cache
}

impl fmt::Display for Cache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fetch(bit_width) => write!(f, "(cache-fetch {})", bit_width),
        }
    }
}

impl Cache {
    pub fn variable() -> Variable {
        Variable::new("_cache", Sort::cache())
    }

    pub fn fetch(bit_width: usize, cache: Expression, addr: Expression) -> Result<Expression> {
        cache.sort().expect_cache()?;
        addr.sort().expect_word()?;

        Ok(Expression::new(
            Self::Fetch(bit_width).into(),
            vec![cache, addr],
            Sort::cache(),
        ))
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct CacheValue {
    addresses: BTreeSet<u64>,
    default_empty: bool,
}

impl CacheValue {
    pub fn new_empty() -> Self {
        Self {
            addresses: BTreeSet::new(),
            default_empty: true,
        }
    }

    pub fn new_full() -> Self {
        Self {
            addresses: BTreeSet::new(),
            default_empty: false,
        }
    }

    pub fn fetch(&mut self, addr: u64) {
        if self.default_empty {
            self.addresses.insert(addr);
        } else {
            self.addresses.remove(&addr);
        }
    }

    pub fn evict(&mut self, addr: u64) {
        if self.default_empty {
            self.addresses.remove(&addr);
        } else {
            self.addresses.insert(addr);
        }
    }

    pub fn is_cached(&self, addr: u64) -> bool {
        let present = self.addresses.contains(&addr);
        if self.default_empty {
            present
        } else {
            !present
        }
    }
}

impl fmt::Display for CacheValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.default_empty {
            write!(f, "⊤ ∖ ")?;
        }
        write!(f, "{{")?;
        let mut is_first = true;
        for (addr_start, addr_end) in self.addresses.iter().compact(|(a, b)| a + 1 == *b) {
            if !is_first {
                write!(f, ", ")?;
            }
            if addr_start == addr_end {
                write!(f, "0x{:X}", addr_start)?;
            } else {
                write!(f, "0x{:X}…0x{:X}", addr_start, addr_end)?;
            }
            is_first = false;
        }
        write!(f, "}}")
    }
}
