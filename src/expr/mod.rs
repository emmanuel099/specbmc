use crate::error::Result;
use std::convert::TryFrom;
use std::fmt;

mod arch;
mod array;
mod bitvector;
mod boolean;
mod constant;
mod integer;
mod sort;
mod variable;

pub use self::arch::BranchTargetBuffer;
pub use self::arch::Memory;
pub use self::arch::PatternHistoryTable;
pub use self::arch::Predictor;
pub use self::arch::{Cache, CacheValue};
pub use self::array::{Array, ArrayValue};
pub use self::bitvector::{BitVector, BitVectorValue};
pub use self::boolean::Boolean;
pub use self::constant::Constant;
pub use self::integer::Integer;
pub use self::sort::Sort;
pub use self::variable::Variable;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Operator {
    Variable(Variable),
    Constant(Constant),
    Ite,
    Equal,
    Nondet, // Nondeterministic value
    Boolean(Boolean),
    Integer(Integer),
    BitVector(BitVector),
    Array(Array),
    // Arch
    Memory(Memory),
    Predictor(Predictor),
    Cache(Cache),
    BranchTargetBuffer(BranchTargetBuffer),
    PatternHistoryTable(PatternHistoryTable),
}

macro_rules! impl_operator_from {
    ( $name:ident ) => {
        impl From<$name> for Operator {
            fn from(op: $name) -> Self {
                Self::$name(op)
            }
        }
    };
}

impl_operator_from!(Boolean);
impl_operator_from!(Integer);
impl_operator_from!(BitVector);
impl_operator_from!(Array);
impl_operator_from!(Memory);
impl_operator_from!(Predictor);
impl_operator_from!(Cache);
impl_operator_from!(BranchTargetBuffer);
impl_operator_from!(PatternHistoryTable);

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Variable(v) => v.fmt(f),
            Self::Constant(c) => c.fmt(f),
            Self::Ite => write!(f, "ite"),
            Self::Equal => write!(f, "="),
            Self::Nondet => write!(f, "nondet()"),
            Self::Boolean(op) => op.fmt(f),
            Self::Integer(op) => op.fmt(f),
            Self::BitVector(op) => op.fmt(f),
            Self::Array(op) => op.fmt(f),
            Self::Memory(op) => op.fmt(f),
            Self::Predictor(op) => op.fmt(f),
            Self::Cache(op) => op.fmt(f),
            Self::BranchTargetBuffer(op) => op.fmt(f),
            Self::PatternHistoryTable(op) => op.fmt(f),
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Expression {
    operator: Operator,
    operands: Vec<Expression>,
    sort: Sort,
}

impl Expression {
    pub fn new(operator: Operator, operands: Vec<Expression>, sort: Sort) -> Self {
        Self {
            operator,
            operands,
            sort,
        }
    }

    pub fn operator(&self) -> &Operator {
        &self.operator
    }

    pub fn operands(&self) -> &[Expression] {
        &self.operands
    }

    pub fn operands_mut(&mut self) -> &mut Vec<Expression> {
        &mut self.operands
    }

    pub fn sort(&self) -> &Sort {
        &self.sort
    }

    pub fn variable(variable: Variable) -> Expression {
        let result_sort = variable.sort().clone();
        Expression::new(Operator::Variable(variable), vec![], result_sort)
    }

    pub fn constant(constant: Constant, sort: Sort) -> Expression {
        Expression::new(Operator::Constant(constant), vec![], sort)
    }

    pub fn ite(cond: Expression, then: Expression, else_: Expression) -> Result<Expression> {
        cond.sort().expect_boolean()?;
        then.sort().expect_sort(else_.sort())?;

        let result_sort = then.sort().clone();
        Ok(Expression::new(
            Operator::Ite,
            vec![cond, then, else_],
            result_sort,
        ))
    }

    pub fn equal(lhs: Expression, rhs: Expression) -> Result<Expression> {
        lhs.sort().expect_sort(rhs.sort())?;

        Ok(Expression::new(
            Operator::Equal,
            vec![lhs, rhs],
            Sort::boolean(),
        ))
    }

    pub fn all_equal(exprs: &[Expression]) -> Result<Expression> {
        if exprs.is_empty() {
            return Ok(Boolean::constant(true));
        }

        // Make sure that all have the same sort
        let sort = exprs.first().unwrap().sort();
        for expr in exprs {
            expr.sort().expect_sort(sort)?;
        }

        Ok(Expression::new(
            Operator::Equal,
            exprs.to_vec(),
            Sort::boolean(),
        ))
    }

    pub fn unequal(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Boolean::not(Self::equal(lhs, rhs)?)
    }

    pub fn nondet(sort: Sort) -> Expression {
        Expression::new(Operator::Nondet, vec![], sort)
    }

    /// Returns all `Variables` used in this `Expression`
    pub fn variables(&self) -> Vec<&Variable> {
        let mut variables: Vec<&Variable> = Vec::new();
        match &self.operator {
            Operator::Variable(variable) => variables.push(variable),
            _ => {
                for operand in &self.operands {
                    variables.append(&mut operand.variables())
                }
            }
        }
        variables
    }

    /// Return mutable references to all `Variables` in this `Expression`.
    pub fn variables_mut(&mut self) -> Vec<&mut Variable> {
        let mut variables: Vec<&mut Variable> = Vec::new();
        match &mut self.operator {
            Operator::Variable(variable) => variables.push(variable),
            _ => {
                for operand in &mut self.operands {
                    variables.append(&mut operand.variables_mut())
                }
            }
        }
        variables
    }

    pub fn is_constant(&self) -> bool {
        match &self.operator {
            Operator::Constant(_) => true,
            _ => false,
        }
    }

    pub fn is_nondet(&self) -> bool {
        match &self.operator {
            Operator::Nondet => true,
            _ => false,
        }
    }

    /// Returns a copy of the expression with the composition number set to `composition` for all variables.
    pub fn self_compose(&self, composition: usize) -> Self {
        let mut expr = self.clone();
        expr.variables_mut().iter_mut().for_each(|var| {
            var.set_composition(Some(composition));
        });
        expr
    }
}

impl From<Variable> for Expression {
    fn from(var: Variable) -> Self {
        Self::variable(var)
    }
}

impl TryFrom<&Expression> for bool {
    type Error = &'static str;

    fn try_from(e: &Expression) -> std::result::Result<bool, Self::Error> {
        if !e.operands().is_empty() {
            return Err("cannot convert");
        }
        match e.operator() {
            Operator::Constant(c) => bool::try_from(c),
            _ => Err("cannot convert"),
        }
    }
}

impl TryFrom<&Expression> for u64 {
    type Error = &'static str;

    fn try_from(e: &Expression) -> std::result::Result<u64, Self::Error> {
        if !e.operands().is_empty() {
            return Err("cannot convert");
        }
        match e.operator() {
            Operator::Constant(c) => u64::try_from(c),
            _ => Err("cannot convert"),
        }
    }
}

impl TryFrom<&Expression> for BitVectorValue {
    type Error = &'static str;

    fn try_from(e: &Expression) -> std::result::Result<BitVectorValue, Self::Error> {
        if !e.operands().is_empty() {
            return Err("cannot convert");
        }
        match e.operator() {
            Operator::Constant(c) => BitVectorValue::try_from(c),
            _ => Err("cannot convert"),
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.operands.is_empty() {
            self.operator.fmt(f)
        } else {
            write!(f, "({}", self.operator)?;
            for operand in &self.operands {
                write!(f, " {}", operand)?;
            }
            write!(f, ")")
        }
    }
}
