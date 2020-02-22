use crate::error::{ErrorKind, Result};
use crate::expr::{Expression, Operator, Sort, Variable};
use num_bigint::BigUint;
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Value {
    value: BigUint,
    bits: usize,
}

impl Value {
    /// Create a new `Value` with the given value and bitness.
    pub fn new(value: u64, bits: usize) -> Self {
        Self {
            value: Self::trim_value(BigUint::from_u64(value).unwrap(), bits),
            bits,
        }
    }

    /// Create a new `Value` from the given `BigUint`.
    pub fn new_big(value: BigUint, bits: usize) -> Self {
        Self {
            value: Self::trim_value(value, bits),
            bits,
        }
    }

    /// Crates a constant from a decimal string of the value
    pub fn from_decimal_string(s: &str, bits: usize) -> Result<Self> {
        let constant = Self::new_big(s.parse()?, bits);
        match constant.bits() {
            b if b < bits => constant.zext(bits),
            b if b > bits => constant.trun(bits),
            _ => Ok(constant),
        }
    }

    /// Create a new `Value` with the given bits and a value of zero
    pub fn new_zero(bits: usize) -> Self {
        Value {
            value: BigUint::from_u64(0).unwrap(),
            bits,
        }
    }

    fn trim_value(value: BigUint, bits: usize) -> BigUint {
        let mask = BigUint::from_u64(1).unwrap() << bits;
        let mask = mask - BigUint::from_u64(1).unwrap();
        value & mask
    }

    /// Get the value of this `Value` if it is a `u64`.
    pub fn value_u64(&self) -> Option<u64> {
        self.value.to_u64()
    }

    /// Sign-extend the constant out to 64-bits, and return it as an `i64`
    pub fn value_i64(&self) -> Option<i64> {
        match self.bits() {
            b if b < 64 => self.sext(64).ok()?.value.to_u64().map(|v| v as i64),
            b if b == 64 => self.value.to_u64().map(|v| v as i64),
            _ => None,
        }
    }

    /// Get the value of this `Value` if it is a `BigUint`.
    pub fn value(&self) -> &BigUint {
        &self.value
    }

    /// Get the number of bits for this `Value`.
    pub fn bits(&self) -> usize {
        self.bits
    }

    /// Returns true if the value in this Value is 0, false otherwise.
    pub fn is_zero(&self) -> bool {
        self.value.is_zero()
    }

    /// Returns true if the value in this constant is 1, false otherwise.
    pub fn is_one(&self) -> bool {
        self.value.is_one()
    }

    pub fn trun(&self, bits: usize) -> Result<Value> {
        if bits >= self.bits() {
            Err(ErrorKind::Sort.into())
        } else {
            Ok(Value::new_big(self.value.clone(), bits))
        }
    }

    pub fn zext(&self, bits: usize) -> Result<Value> {
        if bits <= self.bits() {
            Err(ErrorKind::Sort.into())
        } else {
            Ok(Value::new_big(self.value.clone(), bits))
        }
    }

    pub fn sext(&self, bits: usize) -> Result<Value> {
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
            Ok(Value::new_big(value, bits))
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{:X}:{}", self.value(), self.bits())
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum BitVector {
    Constant(Value),
    ToBoolean,
    FromBoolean(usize),
    Concat,
    Extract(usize, usize),
    Truncate(usize),
    Not,
    And,
    Or,
    Neg,
    Add,
    Mul,
    UDiv,
    URem,
    Shl,
    LShr,
    ULt,
    Nand,
    Nor,
    Xor,
    Xnor,
    Comp,
    Sub,
    SDiv,
    SRem,
    SMod,
    UMod,
    AShr,
    Repeat(usize),
    ZeroExtend(usize),
    SignExtend(usize),
    RotateLeft(usize),
    RotateRight(usize),
    ULe,
    UGt,
    UGe,
    SLt,
    SLe,
    SGt,
    SGe,
}

impl Into<Operator> for BitVector {
    fn into(self) -> Operator {
        Operator::BitVector(self)
    }
}

impl fmt::Display for BitVector {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::Constant(value) => format!("{}", value),
            Self::ToBoolean => "bv2bool".to_owned(),
            Self::FromBoolean(i) => format!("(bool2bv {})", i),
            Self::Concat => "bvconcat".to_owned(),
            Self::Extract(i, j) => format!("(bvextract {} {})", i, j),
            Self::Truncate(i) => format!("(bvtrunc {})", i),
            Self::Not => "bvnot".to_owned(),
            Self::And => "bvand".to_owned(),
            Self::Or => "bvor".to_owned(),
            Self::Neg => "bvneg".to_owned(),
            Self::Add => "bvadd".to_owned(),
            Self::Mul => "bvmul".to_owned(),
            Self::UDiv => "bvudiv".to_owned(),
            Self::URem => "bvurem".to_owned(),
            Self::Shl => "bvshl".to_owned(),
            Self::LShr => "bvlshr".to_owned(),
            Self::ULt => "bvult".to_owned(),
            Self::Nand => "bvnand".to_owned(),
            Self::Nor => "bvnor".to_owned(),
            Self::Xor => "bvxor".to_owned(),
            Self::Xnor => "bvxnor".to_owned(),
            Self::Comp => "bvcomp".to_owned(),
            Self::Sub => "bvsub".to_owned(),
            Self::SDiv => "bvsdiv".to_owned(),
            Self::SRem => "bvsrem".to_owned(),
            Self::SMod => "bvsmod".to_owned(),
            Self::UMod => "bvumod".to_owned(),
            Self::AShr => "bvashr".to_owned(),
            Self::Repeat(i) => format!("(bvrepeat {})", i),
            Self::ZeroExtend(i) => format!("(bvzext {})", i),
            Self::SignExtend(i) => format!("(bvsext {})", i),
            Self::RotateLeft(i) => format!("(bvrotl {})", i),
            Self::RotateRight(i) => format!("(bvrotr {})", i),
            Self::ULe => "bvule".to_owned(),
            Self::UGt => "bvugt".to_owned(),
            Self::UGe => "bvuge".to_owned(),
            Self::SLt => "bvslt".to_owned(),
            Self::SLe => "bvsle".to_owned(),
            Self::SGt => "bvsgt".to_owned(),
            Self::SGe => "bvsge".to_owned(),
        };
        write!(f, "{}", s)
    }
}

macro_rules! bv_arith {
    ( $name:ident, $op:expr ) => {
        pub fn $name(lhs: Expression, rhs: Expression) -> Result<Expression> {
            lhs.sort().expect_bit_vector()?;
            rhs.sort().expect_sort(lhs.sort())?;

            let result_sort = lhs.sort().clone();
            Ok(Expression::new($op.into(), vec![lhs, rhs], result_sort))
        }
    };
}

macro_rules! bv_comp {
    ( $name:ident, $op:expr ) => {
        pub fn $name(lhs: Expression, rhs: Expression) -> Result<Expression> {
            lhs.sort().expect_bit_vector()?;
            rhs.sort().expect_sort(lhs.sort())?;

            Ok(Expression::new($op.into(), vec![lhs, rhs], Sort::boolean()))
        }
    };
}

impl BitVector {
    pub fn variable(name: &str, bits: usize) -> Variable {
        Variable::new(name, Sort::bit_vector(bits))
    }

    pub fn constant(value: u64, bits: usize) -> Expression {
        let bv = Value::new(value, bits);
        Expression::new(
            BitVector::Constant(bv).into(),
            vec![],
            Sort::bit_vector(bits),
        )
    }

    pub fn constant_big(value: BigUint, bits: usize) -> Expression {
        let bv = Value::new_big(value, bits);
        Expression::new(
            BitVector::Constant(bv).into(),
            vec![],
            Sort::bit_vector(bits),
        )
    }

    pub fn to_boolean(expr: Expression) -> Result<Expression> {
        expr.sort().expect_bit_vector()?;

        Ok(Expression::new(
            BitVector::ToBoolean.into(),
            vec![expr],
            Sort::boolean(),
        ))
    }

    pub fn from_boolean(bits: usize, expr: Expression) -> Result<Expression> {
        expr.sort().expect_boolean()?;

        Ok(Expression::new(
            BitVector::FromBoolean(bits).into(),
            vec![expr],
            Sort::bit_vector(bits),
        ))
    }

    bv_arith!(and, BitVector::And);
    bv_arith!(or, BitVector::Or);
    bv_arith!(mul, BitVector::Mul);
    bv_arith!(add, BitVector::Add);
    bv_arith!(udiv, BitVector::UDiv);
    bv_arith!(urem, BitVector::URem);
    bv_arith!(shl, BitVector::Shl);
    bv_arith!(lshr, BitVector::LShr);
    bv_arith!(nand, BitVector::Nand);
    bv_arith!(nor, BitVector::Nor);
    bv_arith!(xor, BitVector::Xor);
    bv_arith!(xnor, BitVector::Xnor);
    bv_arith!(sub, BitVector::Sub);
    bv_arith!(sdiv, BitVector::SDiv);
    bv_arith!(srem, BitVector::SRem);
    bv_arith!(smod, BitVector::SMod);
    bv_arith!(umod, BitVector::UMod);
    bv_arith!(ashr, BitVector::AShr);

    bv_comp!(ult, BitVector::ULt);
    bv_comp!(ule, BitVector::ULe);
    bv_comp!(ugt, BitVector::UGt);
    bv_comp!(uge, BitVector::UGe);
    bv_comp!(slt, BitVector::SLt);
    bv_comp!(sle, BitVector::SLe);
    bv_comp!(sgt, BitVector::SGt);
    bv_comp!(sge, BitVector::SGe);

    pub fn zero_extend(bits: usize, expr: Expression) -> Result<Expression> {
        expr.sort().expect_bit_vector()?;

        Ok(Expression::new(
            BitVector::ZeroExtend(bits).into(),
            vec![expr],
            Sort::bit_vector(bits),
        ))
    }

    pub fn sign_extend(bits: usize, expr: Expression) -> Result<Expression> {
        expr.sort().expect_bit_vector()?;

        Ok(Expression::new(
            BitVector::SignExtend(bits).into(),
            vec![expr],
            Sort::bit_vector(bits),
        ))
    }

    pub fn extract(highest_bit: usize, lowest_bit: usize, expr: Expression) -> Result<Expression> {
        expr.sort().expect_bit_vector()?;

        Ok(Expression::new(
            BitVector::Extract(highest_bit, lowest_bit).into(),
            vec![expr],
            Sort::bit_vector(highest_bit - lowest_bit + 1),
        ))
    }

    pub fn truncate(bits: usize, expr: Expression) -> Result<Expression> {
        expr.sort().expect_bit_vector()?;

        Ok(Expression::new(
            BitVector::Truncate(bits).into(),
            vec![expr],
            Sort::bit_vector(bits),
        ))
    }

    pub fn concat(exprs: &[Expression]) -> Result<Expression> {
        for expr in exprs {
            expr.sort().expect_bit_vector()?;
        }

        if exprs.len() == 1 {
            return Ok(exprs[0].clone());
        }

        let result_width = exprs
            .iter()
            .map(|expr| expr.sort().unwrap_bit_vector())
            .sum();

        Ok(Expression::new(
            BitVector::Concat.into(),
            exprs.to_vec(),
            Sort::bit_vector(result_width),
        ))
    }

    // TODO not, neg, comp, repeat, rotateleft, rotateright
}
