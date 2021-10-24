use crate::environment;
use crate::error::Result;
use crate::expr::{Constant, Expression, Sort, Variable};
pub use falcon::il::Constant as BitVectorValue;
use num_bigint::BigUint;
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum BitVector {
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
    /// Subtraction which yields zero on underflow.
    SaturatingSub,
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

impl fmt::Display for BitVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
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
            Self::SaturatingSub => "bvsatsub".to_owned(),
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

macro_rules! bv_unary {
    ( $name:ident, $op:expr ) => {
        pub fn $name(expr: Expression) -> Result<Expression> {
            expr.sort().expect_bit_vector()?;

            let result_sort = expr.sort().clone();
            Ok(Expression::new($op.into(), vec![expr], result_sort))
        }
    };
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

    pub fn word_variable(name: &str) -> Variable {
        Variable::new(name, Sort::word())
    }

    pub fn constant(value: BitVectorValue) -> Expression {
        let bits = value.bits();
        Expression::constant(Constant::bit_vector(value), Sort::bit_vector(bits))
    }

    pub fn word_constant(value: u64) -> Expression {
        Self::constant_u64(value, environment::WORD_SIZE)
    }

    pub fn constant_u64(value: u64, bits: usize) -> Expression {
        Self::constant(BitVectorValue::new(value, bits))
    }

    pub fn constant_big_uint(value: BigUint) -> Expression {
        let bits: usize = value.bits().try_into().unwrap();
        Self::constant(BitVectorValue::new_big(value, bits))
    }

    #[allow(clippy::wrong_self_convention)]
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

    pub fn word_from_boolean(expr: Expression) -> Result<Expression> {
        Self::from_boolean(environment::WORD_SIZE, expr)
    }

    bv_unary!(not, Self::Not);
    bv_unary!(neg, Self::Neg);

    bv_arith!(and, Self::And);
    bv_arith!(or, Self::Or);
    bv_arith!(mul, Self::Mul);
    bv_arith!(add, Self::Add);
    bv_arith!(udiv, Self::UDiv);
    bv_arith!(urem, Self::URem);
    bv_arith!(shl, Self::Shl);
    bv_arith!(lshr, Self::LShr);
    bv_arith!(nand, Self::Nand);
    bv_arith!(nor, Self::Nor);
    bv_arith!(xor, Self::Xor);
    bv_arith!(xnor, Self::Xnor);
    bv_arith!(sub, Self::Sub);
    bv_arith!(saturating_sub, Self::SaturatingSub);
    bv_arith!(sdiv, Self::SDiv);
    bv_arith!(srem, Self::SRem);
    bv_arith!(smod, Self::SMod);
    bv_arith!(umod, Self::UMod);
    bv_arith!(ashr, Self::AShr);

    bv_comp!(ult, Self::ULt);
    bv_comp!(ule, Self::ULe);
    bv_comp!(ugt, Self::UGt);
    bv_comp!(uge, Self::UGe);
    bv_comp!(slt, Self::SLt);
    bv_comp!(sle, Self::SLe);
    bv_comp!(sgt, Self::SGt);
    bv_comp!(sge, Self::SGe);

    /// Extend the bit-vector given by `expr` with `n` additional zero-bits.
    ///
    /// The width of the resulting bit vector is bit-width of `expr` + `n`.
    pub fn zero_extend(n: usize, expr: Expression) -> Result<Expression> {
        expr.sort().expect_bit_vector()?;
        let width = expr.sort().unwrap_bit_vector();

        Ok(Expression::new(
            Self::ZeroExtend(n).into(),
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

    pub fn zero_extend_to_word(expr: Expression) -> Result<Expression> {
        Self::zero_extend_abs(environment::WORD_SIZE, expr)
    }

    /// Sign-extend the bit-vector given by `expr` with `n` additional bits.
    ///
    /// The width of the resulting bit vector is bit-width of `expr` + `n`.
    pub fn sign_extend(n: usize, expr: Expression) -> Result<Expression> {
        expr.sort().expect_bit_vector()?;
        let width = expr.sort().unwrap_bit_vector();

        Ok(Expression::new(
            Self::SignExtend(n).into(),
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

    pub fn sign_extend_to_word(expr: Expression) -> Result<Expression> {
        Self::sign_extend_abs(environment::WORD_SIZE, expr)
    }

    pub fn extract(highest_bit: usize, lowest_bit: usize, expr: Expression) -> Result<Expression> {
        expr.sort().expect_bit_vector()?;

        Ok(Expression::new(
            Self::Extract(highest_bit, lowest_bit).into(),
            vec![expr],
            Sort::bit_vector(highest_bit - lowest_bit + 1),
        ))
    }

    pub fn truncate(bits: usize, expr: Expression) -> Result<Expression> {
        expr.sort().expect_bit_vector()?;

        Ok(Expression::new(
            Self::Truncate(bits).into(),
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
            Self::Concat.into(),
            exprs.to_vec(),
            Sort::bit_vector(result_width),
        ))
    }

    pub fn repeat(n: usize, expr: Expression) -> Result<Expression> {
        expr.sort().expect_bit_vector()?;
        let width = expr.sort().unwrap_bit_vector();

        Ok(Expression::new(
            Self::Repeat(n).into(),
            vec![expr],
            Sort::bit_vector(width * n),
        ))
    }

    pub fn rotate_left(n: usize, expr: Expression) -> Result<Expression> {
        expr.sort().expect_bit_vector()?;

        let result_sort = expr.sort().clone();
        Ok(Expression::new(
            Self::RotateLeft(n).into(),
            vec![expr],
            result_sort,
        ))
    }

    pub fn rotate_right(n: usize, expr: Expression) -> Result<Expression> {
        expr.sort().expect_bit_vector()?;

        let result_sort = expr.sort().clone();
        Ok(Expression::new(
            Self::RotateRight(n).into(),
            vec![expr],
            result_sort,
        ))
    }
}
