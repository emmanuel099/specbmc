use crate::error::Result;
use crate::expr::{ArrayValue, BitVectorValue};
use num_bigint::BigUint;
use std::convert::TryFrom;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum Constant {
    Boolean(bool),
    Integer(u64),
    BitVector(BitVectorValue),
    Array(Box<ArrayValue>),
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
        let bits = value.bits();
        Self::bit_vector(BitVectorValue::new_big(value, bits))
    }

    pub fn array(value: ArrayValue) -> Self {
        Self::Array(Box::new(value))
    }

    pub fn is_boolean(&self) -> bool {
        match self {
            Self::Boolean(_) => true,
            _ => false,
        }
    }

    pub fn is_integer(&self) -> bool {
        match self {
            Self::Integer(_) => true,
            _ => false,
        }
    }

    pub fn is_bit_vector(&self) -> bool {
        match self {
            Self::BitVector(..) => true,
            _ => false,
        }
    }

    pub fn is_array(&self) -> bool {
        match self {
            Self::Array(_) => true,
            _ => false,
        }
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
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Boolean(v) => write!(f, "{}", v),
            Self::Integer(v) => write!(f, "{}", v),
            Self::BitVector(v) => write!(f, "{}", v),
            Self::Array(v) => write!(f, "{}", v),
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

impl TryFrom<&Constant> for u64 {
    type Error = &'static str;

    fn try_from(c: &Constant) -> std::result::Result<u64, Self::Error> {
        match c {
            Constant::Boolean(true) => Ok(1),
            Constant::Boolean(false) => Ok(0),
            Constant::Integer(v) => Ok(*v),
            Constant::BitVector(v) => v.value_u64().ok_or_else(|| "failed to convert to u64"),
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
