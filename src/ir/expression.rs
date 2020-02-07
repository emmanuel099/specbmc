use crate::error::Result;
use crate::ir::{Constant, Variable};
use std::fmt;

#[derive(Clone, Debug)]
pub enum Expression {
    Variable(Variable),
    Constant(Constant),

    Add(Box<Expression>, Box<Expression>),
    Sub(Box<Expression>, Box<Expression>),
    Mul(Box<Expression>, Box<Expression>),
    Divu(Box<Expression>, Box<Expression>),
    Modu(Box<Expression>, Box<Expression>),
    Divs(Box<Expression>, Box<Expression>),
    Mods(Box<Expression>, Box<Expression>),
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Xor(Box<Expression>, Box<Expression>),
    Shl(Box<Expression>, Box<Expression>),
    Shr(Box<Expression>, Box<Expression>),

    Cmpeq(Box<Expression>, Box<Expression>),
    Cmpneq(Box<Expression>, Box<Expression>),
    Cmplts(Box<Expression>, Box<Expression>),
    Cmpltu(Box<Expression>, Box<Expression>),

    Zext(usize, Box<Expression>),
    Sext(usize, Box<Expression>),
    Trun(usize, Box<Expression>),

    Ite(Box<Expression>, Box<Expression>, Box<Expression>),
}

impl Expression {
    /// Create a new `Expression` from a `Variable`.
    pub fn variable(variable: Variable) -> Expression {
        Expression::Variable(variable)
    }

    /// Create a new `Expression` from a `Constant`.
    pub fn constant(constant: Constant) -> Expression {
        Expression::Constant(constant)
    }

    /// Create an addition `Expression`.
    /// # Error
    /// The sort of the lhs and the rhs are not the same
    pub fn add(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Ok(Expression::Add(Box::new(lhs), Box::new(rhs)))
    }

    /// Create a subtraction `Expression`.
    /// # Error
    /// The sort of the lhs and the rhs are not the same.
    pub fn sub(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Ok(Expression::Sub(Box::new(lhs), Box::new(rhs)))
    }

    /// Create an unsigned multiplication `Expression`.
    /// # Error
    /// The sort of the lhs and the rhs are not the same.
    pub fn mul(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Ok(Expression::Mul(Box::new(lhs), Box::new(rhs)))
    }

    /// Create an unsigned division `Expression`.
    /// # Error
    /// The sort of the lhs and the rhs are not the same.
    pub fn divu(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Ok(Expression::Divu(Box::new(lhs), Box::new(rhs)))
    }

    /// Create an unsigned modulus `Expression`.
    /// # Error
    /// The sort of the lhs and the rhs are not the same.
    pub fn modu(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Ok(Expression::Modu(Box::new(lhs), Box::new(rhs)))
    }

    /// Create a signed division `Expression`.
    /// # Error
    /// The sort of the lhs and the rhs are not the same.
    pub fn divs(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Ok(Expression::Divs(Box::new(lhs), Box::new(rhs)))
    }

    /// Create a signed modulus `Expression`.
    /// # Error
    /// The sort of the lhs and the rhs are not the same.
    pub fn mods(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Ok(Expression::Mods(Box::new(lhs), Box::new(rhs)))
    }

    /// Create a binary and `Expression`.
    /// # Error
    /// The sort of the lhs and the rhs are not the same.
    pub fn and(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Ok(Expression::And(Box::new(lhs), Box::new(rhs)))
    }

    /// Create a binary or `Expression`.
    /// # Error
    /// The sort of the lhs and the rhs are not the same.
    pub fn or(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Ok(Expression::Or(Box::new(lhs), Box::new(rhs)))
    }

    /// Create a binary xor `Expression`.
    /// # Error
    /// The sort of the lhs and the rhs are not the same.
    pub fn xor(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Ok(Expression::Xor(Box::new(lhs), Box::new(rhs)))
    }

    /// Create a logical shift-left `Expression`.
    /// # Error
    /// The sort of the lhs and the rhs are not the same.
    pub fn shl(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Ok(Expression::Shl(Box::new(lhs), Box::new(rhs)))
    }

    /// Create a logical shift-right `Expression`.
    /// # Error
    /// The sort of the lhs and the rhs are not the same.
    pub fn shr(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Ok(Expression::Shr(Box::new(lhs), Box::new(rhs)))
    }

    /// Create an equals comparison `Expression`.
    /// # Error
    /// The sort of the lhs and the rhs are not the same.
    pub fn cmpeq(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Ok(Expression::Cmpeq(Box::new(lhs), Box::new(rhs)))
    }

    /// Create an not equals comparison `Expression`.
    /// # Error
    /// The sort of the lhs and the rhs are not the same.
    pub fn cmpneq(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Ok(Expression::Cmpneq(Box::new(lhs), Box::new(rhs)))
    }

    /// Create an unsigned less-than comparison `Expression`.
    /// # Error
    /// The sort of the lhs and the rhs are not the same.
    pub fn cmpltu(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Ok(Expression::Cmpltu(Box::new(lhs), Box::new(rhs)))
    }

    /// Create a signed less-than comparison `Expression`.
    /// # Error
    /// The sort of the lhs and the rhs are not the same.
    pub fn cmplts(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Ok(Expression::Cmplts(Box::new(lhs), Box::new(rhs)))
    }

    /// Create an expression to zero-extend src to the number of bits specified
    /// in bits.
    /// # Error
    /// src has more or equal number of bits than bits
    pub fn zext(bits: usize, src: Expression) -> Result<Expression> {
        Ok(Expression::Zext(bits, Box::new(src)))
    }

    /// Create an expression to sign-extend src to the number of bits specified
    /// # Error
    /// src has more or equal number of bits than bits
    pub fn sext(bits: usize, src: Expression) -> Result<Expression> {
        Ok(Expression::Sext(bits, Box::new(src)))
    }

    /// Create an expression to truncate the number of bits in src to the number
    /// of bits given.
    /// # Error
    /// src has less-than or equal bits than bits
    pub fn trun(bits: usize, src: Expression) -> Result<Expression> {
        Ok(Expression::Trun(bits, Box::new(src)))
    }

    /// Create an if-than-else expression
    /// # Error
    /// condition is not 1-bit, or bitness of then and else_ do not match.
    pub fn ite(cond: Expression, then: Expression, else_: Expression) -> Result<Expression> {
        Ok(Expression::Ite(
            Box::new(cond),
            Box::new(then),
            Box::new(else_),
        ))
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Expression::Variable(ref v) => v.fmt(f),
            Expression::Constant(ref c) => c.fmt(f),
            Expression::Add(ref lhs, ref rhs) => write!(f, "({} + {})", lhs, rhs),
            Expression::Sub(ref lhs, ref rhs) => write!(f, "({} - {})", lhs, rhs),
            Expression::Mul(ref lhs, ref rhs) => write!(f, "({} * {})", lhs, rhs),
            Expression::Divu(ref lhs, ref rhs) => write!(f, "({} /u {})", lhs, rhs),
            Expression::Modu(ref lhs, ref rhs) => write!(f, "({} %u {})", lhs, rhs),
            Expression::Divs(ref lhs, ref rhs) => write!(f, "({} /s {})", lhs, rhs),
            Expression::Mods(ref lhs, ref rhs) => write!(f, "({} %s {})", lhs, rhs),
            Expression::And(ref lhs, ref rhs) => write!(f, "({} & {})", lhs, rhs),
            Expression::Or(ref lhs, ref rhs) => write!(f, "({} | {})", lhs, rhs),
            Expression::Xor(ref lhs, ref rhs) => write!(f, "({} ^ {})", lhs, rhs),
            Expression::Shl(ref lhs, ref rhs) => write!(f, "({} << {})", lhs, rhs),
            Expression::Shr(ref lhs, ref rhs) => write!(f, "({} >> {})", lhs, rhs),
            Expression::Cmpeq(ref lhs, ref rhs) => write!(f, "({} == {})", lhs, rhs),
            Expression::Cmpneq(ref lhs, ref rhs) => write!(f, "({} != {})", lhs, rhs),
            Expression::Cmplts(ref lhs, ref rhs) => write!(f, "({} <s {})", lhs, rhs),
            Expression::Cmpltu(ref lhs, ref rhs) => write!(f, "({} <u {})", lhs, rhs),
            Expression::Zext(ref bits, ref src) => write!(f, "zext.{}({})", bits, src),
            Expression::Sext(ref bits, ref src) => write!(f, "sext.{}({})", bits, src),
            Expression::Trun(ref bits, ref src) => write!(f, "trun.{}({})", bits, src),
            Expression::Ite(ref cond, ref then, ref else_) => {
                write!(f, "ite({}, {}, {})", cond, then, else_)
            }
        }
    }
}
