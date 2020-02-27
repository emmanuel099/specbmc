use crate::error::Result;
use crate::util::SelfCompose;
use std::convert::TryFrom;
use std::fmt;

mod arch;
mod array;
mod bitvector;
mod boolean;
mod integer;
mod sort;
mod variable;

pub use self::arch::BranchTargetBuffer;
pub use self::arch::Cache;
pub use self::arch::Memory;
pub use self::arch::PatternHistoryTable;
pub use self::arch::Predictor;
pub use self::array::Array;
pub use self::bitvector::BitVector;
pub use self::bitvector::Value as BitVectorValue;
pub use self::boolean::Boolean;
pub use self::integer::Integer;
pub use self::sort::Sort;
pub use self::variable::Variable;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Operator {
    Variable(Variable),
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
            Operator::Boolean(op) => op.is_constant(),
            Operator::Integer(op) => op.is_constant(),
            Operator::BitVector(op) => op.is_constant(),
            _ => false,
        }
    }

    pub fn is_nondet(&self) -> bool {
        match &self.operator {
            Operator::Nondet => true,
            _ => false,
        }
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
            Operator::Boolean(op) => bool::try_from(op),
            Operator::BitVector(op) => bool::try_from(op),
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
            Operator::Integer(op) => u64::try_from(op),
            Operator::BitVector(op) => u64::try_from(op),
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
            Operator::Boolean(op) => BitVectorValue::try_from(op),
            Operator::BitVector(op) => BitVectorValue::try_from(op),
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

impl SelfCompose for Expression {
    fn self_compose(&self, composition: usize) -> Self {
        let mut expr = self.clone();
        expr.variables_mut().iter_mut().for_each(|var| {
            var.set_composition(Some(composition));
        });
        expr
    }
}
