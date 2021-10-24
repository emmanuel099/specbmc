use crate::error::Result;
use crate::expr::{ArrayValue, BitVectorValue, CacheValue, MemoryValue};
use num_bigint::BigUint;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum Constant {
    Boolean(bool),
    Integer(u64),
    BitVector(BitVectorValue),
    Array(Box<ArrayValue>),
    // Arch
    Cache(Box<CacheValue>),
    Memory(Box<MemoryValue>),
}

impl Constant {
    pub fn boolean(value: bool) -> Self {
        Self::Boolean(value)
    }

    pub fn integer(value: u64) -> Self {
        Self::Integer(value)
    }

    pub fn bit_vector(value: BitVectorValue) -> Self {
        Self::BitVector(value)
    }

    pub fn bit_vector_u64(value: u64, bits: usize) -> Self {
        Self::bit_vector(BitVectorValue::new(value, bits))
    }

    pub fn bit_vector_big_uint(value: BigUint) -> Self {
        let bits: usize = value.bits().try_into().unwrap();
        Self::bit_vector(BitVectorValue::new_big(value, bits))
    }

    pub fn array(value: ArrayValue) -> Self {
        Self::Array(Box::new(value))
    }

    pub fn cache(value: CacheValue) -> Self {
        Self::Cache(Box::new(value))
    }

    pub fn memory(value: MemoryValue) -> Self {
        Self::Memory(Box::new(value))
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Boolean(_))
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Self::Integer(_))
    }

    pub fn is_bit_vector(&self) -> bool {
        matches!(self, Self::BitVector(..))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    pub fn is_cache(&self) -> bool {
        matches!(self, Self::Cache(_))
    }

    pub fn is_memory(&self) -> bool {
        matches!(self, Self::Memory(_))
    }

    pub fn expect_boolean(&self) -> Result<()> {
        if self.is_boolean() {
            Ok(())
        } else {
            Err("Expected Boolean".into())
        }
    }

    pub fn expect_integer(&self) -> Result<()> {
        if self.is_integer() {
            Ok(())
        } else {
            Err("Expected Integer".into())
        }
    }

    pub fn expect_bit_vector(&self) -> Result<()> {
        if self.is_bit_vector() {
            Ok(())
        } else {
            Err("Expected BitVec".into())
        }
    }

    pub fn expect_array(&self) -> Result<()> {
        if self.is_array() {
            Ok(())
        } else {
            Err("Expected Array".into())
        }
    }

    pub fn expect_cache(&self) -> Result<()> {
        if self.is_cache() {
            Ok(())
        } else {
            Err("Expected Cache".into())
        }
    }

    pub fn expect_memory(&self) -> Result<()> {
        if self.is_memory() {
            Ok(())
        } else {
            Err("Expected Memory".into())
        }
    }

    pub fn unwrap_boolean(&self) -> bool {
        match self {
            Self::Boolean(v) => *v,
            _ => panic!("Expected Boolean"),
        }
    }

    pub fn unwrap_integer(&self) -> u64 {
        match self {
            Self::Integer(v) => *v,
            _ => panic!("Expected Integer"),
        }
    }

    pub fn unwrap_bit_vector(&self) -> &BitVectorValue {
        match self {
            Self::BitVector(v) => v,
            _ => panic!("Expected BitVec"),
        }
    }

    pub fn unwrap_array(&self) -> &ArrayValue {
        match self {
            Self::Array(v) => v,
            _ => panic!("Expected Array"),
        }
    }

    pub fn unwrap_cache(&self) -> &CacheValue {
        match self {
            Self::Cache(v) => v,
            _ => panic!("Expected Cache"),
        }
    }

    pub fn unwrap_memory(&self) -> &MemoryValue {
        match self {
            Self::Memory(v) => v,
            _ => panic!("Expected Memory"),
        }
    }
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boolean(v) => write!(f, "{}", v),
            Self::Integer(v) => write!(f, "{}", v),
            Self::BitVector(v) => write!(f, "{}", v),
            Self::Array(v) => write!(f, "{}", v),
            Self::Cache(v) => write!(f, "{}", v),
            Self::Memory(v) => write!(f, "{}", v),
        }
    }
}

impl From<bool> for Constant {
    fn from(value: bool) -> Self {
        Self::boolean(value)
    }
}

impl From<u64> for Constant {
    fn from(value: u64) -> Self {
        Self::integer(value)
    }
}

impl From<BitVectorValue> for Constant {
    fn from(value: BitVectorValue) -> Self {
        Self::bit_vector(value)
    }
}

impl From<BigUint> for Constant {
    fn from(value: BigUint) -> Self {
        Self::bit_vector_big_uint(value)
    }
}

impl From<ArrayValue> for Constant {
    fn from(value: ArrayValue) -> Self {
        Self::array(value)
    }
}

impl From<CacheValue> for Constant {
    fn from(value: CacheValue) -> Self {
        Self::cache(value)
    }
}

impl From<MemoryValue> for Constant {
    fn from(value: MemoryValue) -> Self {
        Self::memory(value)
    }
}

impl TryFrom<&Constant> for bool {
    type Error = &'static str;

    fn try_from(c: &Constant) -> std::result::Result<bool, Self::Error> {
        match c {
            Constant::Boolean(v) => Ok(*v),
            Constant::Integer(v) => Ok(*v != 0),
            Constant::BitVector(v) => Ok(!v.is_zero()),
            _ => Err("Cannot convert constant to bool"),
        }
    }
}

impl TryFrom<&Constant> for u8 {
    type Error = &'static str;

    fn try_from(c: &Constant) -> std::result::Result<u8, Self::Error> {
        match c {
            Constant::Boolean(true) => Ok(1),
            Constant::Boolean(false) => Ok(0),
            Constant::Integer(v) => {
                if *v < 256 {
                    Ok(*v as u8)
                } else {
                    Err("Does not fit into u8")
                }
            }
            Constant::BitVector(v) => match v.value_u64() {
                Some(value) => {
                    if value < 256 {
                        Ok(value as u8)
                    } else {
                        Err("Does not fit into u8")
                    }
                }
                None => Err("Cannot convert constant to u8"),
            },
            _ => Err("Cannot convert constant to u8"),
        }
    }
}

impl TryFrom<&Constant> for u64 {
    type Error = &'static str;

    fn try_from(c: &Constant) -> std::result::Result<u64, Self::Error> {
        match c {
            Constant::Boolean(true) => Ok(1),
            Constant::Boolean(false) => Ok(0),
            Constant::Integer(v) => Ok(*v),
            Constant::BitVector(v) => v.value_u64().ok_or("failed to convert to u64"),
            _ => Err("Cannot convert constant to u64"),
        }
    }
}

impl TryFrom<&Constant> for BitVectorValue {
    type Error = &'static str;

    fn try_from(c: &Constant) -> std::result::Result<BitVectorValue, Self::Error> {
        match c {
            Constant::Boolean(true) => Ok(BitVectorValue::new(1, 1)),
            Constant::Boolean(false) => Ok(BitVectorValue::new(0, 1)),
            Constant::Integer(v) => Ok(BitVectorValue::new(*v, 64)),
            Constant::BitVector(v) => Ok(v.clone()),
            _ => Err("Cannot convert constant to Bit-Vector"),
        }
    }
}
