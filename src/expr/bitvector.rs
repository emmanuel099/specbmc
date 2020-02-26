use crate::error::Result;
use crate::expr::{Expression, Sort, Variable};
pub use falcon::il::Constant as Value;
use num_bigint::BigUint;
use std::convert::TryFrom;
use std::fmt;

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

impl From<Value> for BitVector {
    fn from(value: Value) -> Self {
        Self::Constant(value)
    }
}

impl TryFrom<&BitVector> for bool {
    type Error = &'static str;

    fn try_from(b: &BitVector) -> std::result::Result<bool, Self::Error> {
        match b {
            BitVector::Constant(value) => Ok(!value.is_zero()),
            _ => Err("not a constant"),
        }
    }
}

impl TryFrom<&BitVector> for u64 {
    type Error = &'static str;

    fn try_from(b: &BitVector) -> std::result::Result<u64, Self::Error> {
        match b {
            BitVector::Constant(value) => {
                value.value_u64().ok_or_else(|| "failed to convert to u64")
            }
            _ => Err("not a constant"),
        }
    }
}

impl TryFrom<&BitVector> for Value {
    type Error = &'static str;

    fn try_from(b: &BitVector) -> std::result::Result<Value, Self::Error> {
        match b {
            BitVector::Constant(value) => Ok(value.clone()),
            _ => Err("not a constant"),
        }
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

    pub fn constant(value: Value) -> Expression {
        let bits = value.bits();
        Expression::new(
            BitVector::Constant(value).into(),
            vec![],
            Sort::bit_vector(bits),
        )
    }

    pub fn constant_u64(value: u64, bits: usize) -> Expression {
        Self::constant(Value::new(value, bits))
    }

    pub fn constant_big_uint(value: BigUint, bits: usize) -> Expression {
        Self::constant(Value::new_big(value, bits))
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

    /// Extend the bit-vector given by `expr` with `n` additional zero-bits.
    ///
    /// The width of the resulting bit vector is bit-width of `expr` + `n`.
    pub fn zero_extend(n: usize, expr: Expression) -> Result<Expression> {
        expr.sort().expect_bit_vector()?;
        let width = expr.sort().unwrap_bit_vector();

        Ok(Expression::new(
            BitVector::ZeroExtend(n).into(),
            vec![expr],
            Sort::bit_vector(width + n),
        ))
    }

    /// Extend the bit-vector given by `expr` with zero-bits such that the resulting width is `bits`.
    ///
    /// The width of the resulting bit vector is `bits`.
    pub fn zero_extend_abs(bits: usize, expr: Expression) -> Result<Expression> {
        expr.sort().expect_bit_vector()?;
        let width = expr.sort().unwrap_bit_vector();
        Self::zero_extend(bits - width, expr)
    }

    /// Sign-extend the bit-vector given by `expr` with `n` additional bits.
    ///
    /// The width of the resulting bit vector is bit-width of `expr` + `n`.
    pub fn sign_extend(n: usize, expr: Expression) -> Result<Expression> {
        expr.sort().expect_bit_vector()?;
        let width = expr.sort().unwrap_bit_vector();

        Ok(Expression::new(
            BitVector::SignExtend(n).into(),
            vec![expr],
            Sort::bit_vector(width + n),
        ))
    }

    /// Sign-extend the bit-vector given by `expr` with additional bits such that the resulting width is `bits`.
    ///
    /// The width of the resulting bit vector is `bits`.
    pub fn sign_extend_abs(bits: usize, expr: Expression) -> Result<Expression> {
        expr.sort().expect_bit_vector()?;
        let width = expr.sort().unwrap_bit_vector();
        Self::sign_extend(bits - width, expr)
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

    pub fn is_constant(&self) -> bool {
        match self {
            Self::Constant(_) => true,
            _ => false,
        }
    }
}
