use crate::error::Result;
use crate::expr::{Expression, Sort, Variable};
use crate::util::CompactIterator;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Cache {
    Fetch(usize), // Fetch N bits into the cache
    Evict(usize), // Evict N bits from the cache
}

impl fmt::Display for Cache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fetch(bit_width) => write!(f, "(cache-fetch {})", bit_width),
            Self::Evict(bit_width) => write!(f, "(cache-evict {})", bit_width),
        }
    }
}

impl Cache {
    pub fn variable() -> Variable {
        let mut var = Variable::new("_cache", Sort::cache());
        var.labels_mut().rollback_persistent();
        var
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

    pub fn evict(bit_width: usize, cache: Expression, addr: Expression) -> Result<Expression> {
        cache.sort().expect_cache()?;
        addr.sort().expect_word()?;

        Ok(Expression::new(
            Self::Evict(bit_width).into(),
            vec![cache, addr],
            Sort::cache(),
        ))
    }
}

pub enum CacheAddresses {
    EvictedFromFullCache(BTreeSet<u64>),
    FetchedIntoEmptyCache(BTreeSet<u64>),
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct CacheValue {
    addresses: BTreeSet<u64>, // Holds evicted addresses if default is cached, or fetched addresses if default is not cached.
    default_is_cached: bool,
}

impl CacheValue {
    pub fn empty() -> Self {
        Self {
            addresses: BTreeSet::new(),
            default_is_cached: false,
        }
    }

    pub fn full() -> Self {
        Self {
            addresses: BTreeSet::new(),
            default_is_cached: true,
        }
    }

    pub fn fetch(&mut self, addr: u64) {
        if self.default_is_cached {
            // Remove evicted
            self.addresses.remove(&addr);
        } else {
            // Add fetched
            self.addresses.insert(addr);
        }
    }

    pub fn evict(&mut self, addr: u64) {
        if self.default_is_cached {
            // Add evicted
            self.addresses.insert(addr);
        } else {
            // Removed fetched
            self.addresses.remove(&addr);
        }
    }

    pub fn is_cached(&self, addr: u64) -> bool {
        if self.default_is_cached {
            let evicted = self.addresses.contains(&addr);
            !evicted
        } else {
            self.addresses.contains(&addr)
        }
    }

    pub fn addresses(&self) -> CacheAddresses {
        if self.default_is_cached {
            CacheAddresses::EvictedFromFullCache(self.addresses.clone())
        } else {
            CacheAddresses::FetchedIntoEmptyCache(self.addresses.clone())
        }
    }
}

impl fmt::Display for CacheValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.default_is_cached {
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
