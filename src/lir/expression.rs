use crate::error::Result;
use crate::lir::{BitVector, Boolean, Constant, Memory, Sort, Variable};
use std::fmt;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Operator {
    Variable(Variable),
    Constant(Constant),
    Ite,
    Equal,
    Boolean(Boolean),
    BitVector(BitVector),
    Memory(Memory),
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Variable(v) => v.fmt(f),
            Self::Constant(c) => c.fmt(f),
            Self::Ite => write!(f, "ite"),
            Self::Equal => write!(f, "="),
            Self::Boolean(op) => op.fmt(f),
            Self::BitVector(op) => op.fmt(f),
            Self::Memory(op) => op.fmt(f),
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

    pub fn sort(&self) -> &Sort {
        &self.sort
    }

    pub fn variable(variable: Variable) -> Expression {
        let result_sort = *variable.sort();
        Expression::new(Operator::Variable(variable), vec![], result_sort)
    }

    pub fn constant(constant: Constant) -> Expression {
        let result_sort = constant.sort();
        Expression::new(Operator::Constant(constant), vec![], result_sort)
    }

    pub fn ite(cond: Expression, then: Expression, else_: Expression) -> Result<Expression> {
        cond.sort().expect_bool()?;
        then.sort().expect_sort(else_.sort())?;

        let result_sort = *then.sort();
        Ok(Expression::new(
            Operator::Ite,
            vec![cond, then, else_],
            result_sort,
        ))
    }

    pub fn equal(lhs: Expression, rhs: Expression) -> Result<Expression> {
        lhs.sort().expect_sort(rhs.sort())?;

        Ok(Expression::new(Operator::Equal, vec![lhs, rhs], Sort::Bool))
    }

    pub fn unequal(lhs: Expression, rhs: Expression) -> Result<Expression> {
        Boolean::not(Self::equal(lhs, rhs)?)
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
