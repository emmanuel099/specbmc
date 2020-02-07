use crate::error::{ErrorKind, Result};
use crate::ir::Expression;
use num_bigint::BigUint;
use num_traits::{FromPrimitive, ToPrimitive};
use std::fmt;

#[derive(Clone, Debug)]
pub struct Constant {
    value: BigUint,
    bits: usize,
}

impl Constant {
    /// Create a new `Constant` with the given value and bitness.
    pub fn new(value: u64, bits: usize) -> Self {
        Self {
            value: Self::trim_value(BigUint::from_u64(value).unwrap(), bits),
            bits: bits,
        }
    }

    /// Create a new `Constant` from the given `BigUint`.
    pub fn new_big(value: BigUint, bits: usize) -> Self {
        Self {
            value: Self::trim_value(value, bits),
            bits: bits,
        }
    }

    /// Crates a constant from a decimal string of the value
    pub fn from_decimal_string(s: &String, bits: usize) -> Result<Self> {
        let constant = Self::new_big(s.parse()?, bits);
        Ok(if constant.bits() < bits {
            constant.zext(bits)?
        } else if constant.bits() > bits {
            constant.trun(bits)?
        } else {
            constant
        })
    }

    /// Create a new `Constant` with the given bits and a value of zero
    pub fn new_zero(bits: usize) -> Self {
        Constant {
            value: BigUint::from_u64(0).unwrap(),
            bits: bits,
        }
    }

    fn trim_value(value: BigUint, bits: usize) -> BigUint {
        let mask = BigUint::from_u64(1).unwrap() << bits;
        let mask = mask - BigUint::from_u64(1).unwrap();
        value & mask
    }

    /// Get the value of this `Constant` if it is a `u64`.
    pub fn value_u64(&self) -> Option<u64> {
        self.value.to_u64()
    }

    /// Sign-extend the constant out to 64-bits, and return it as an `i64`
    pub fn value_i64(&self) -> Option<i64> {
        if self.bits() > 64 {
            None
        } else if self.bits() == 64 {
            self.value.to_u64().map(|v| v as i64)
        } else {
            self.sext(64).ok()?.value.to_u64().map(|v| v as i64)
        }
    }

    /// Get the value of this `Constant` if it is a `BigUint`.
    pub fn value(&self) -> &BigUint {
        &self.value
    }

    /// Get the number of bits for this `Constant`.
    pub fn bits(&self) -> usize {
        self.bits
    }

    /// Returns true if the value in this Constant is 0, false otherwise.
    pub fn is_zero(&self) -> bool {
        self.value_u64().map(|v| v == 0).unwrap_or(false)
    }

    /// Returns true if the value in this constant is 1, false otherwise.
    pub fn is_one(&self) -> bool {
        self.value_u64().map(|v| v == 1).unwrap_or(false)
    }

    pub fn trun(&self, bits: usize) -> Result<Constant> {
        if bits >= self.bits() {
            Err(ErrorKind::Sort.into())
        } else {
            Ok(Constant::new_big(self.value.clone(), bits))
        }
    }

    pub fn zext(&self, bits: usize) -> Result<Constant> {
        if bits <= self.bits() {
            Err(ErrorKind::Sort.into())
        } else {
            Ok(Constant::new_big(self.value.clone(), bits))
        }
    }

    pub fn sext(&self, bits: usize) -> Result<Constant> {
        if bits <= self.bits() || bits % 8 > 0 {
            Err(ErrorKind::Sort.into())
        } else {
            let sign_bit = self.value.clone() >> (self.bits - 1);
            let value = if sign_bit == BigUint::from_u64(1).unwrap() {
                let mask = BigUint::from_u64(1).unwrap() << bits;
                let mask = mask - BigUint::from_u64(1).unwrap();
                let mask = mask << self.bits;
                self.value.clone() | mask
            } else {
                self.value.clone()
            };
            Ok(Constant::new_big(value, bits))
        }
    }
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{:X}:{}", self.value, self.bits)
    }
}

impl Into<Expression> for Constant {
    fn into(self) -> Expression {
        Expression::constant(self)
    }
}
