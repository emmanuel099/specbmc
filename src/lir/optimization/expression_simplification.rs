use crate::error::Result;
use crate::expr::*;
use crate::lir;
use crate::lir::optimization::{Optimization, OptimizationResult};
use std::convert::TryFrom;

pub struct ExpressionSimplification {}

impl ExpressionSimplification {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for ExpressionSimplification {
    fn optimize(&self, program: &mut lir::Program) -> Result<OptimizationResult> {
        if program.simplify() {
            Ok(OptimizationResult::Changed)
        } else {
            Ok(OptimizationResult::Unchanged)
        }
    }
}

trait Simplify {
    /// Simplify `Self`
    ///
    /// Returns true if something changed.
    fn simplify(&mut self) -> bool;
}

impl Simplify for lir::Program {
    fn simplify(&mut self) -> bool {
        self.nodes_mut()
            .into_iter()
            .fold(false, |simplified, node| node.simplify() || simplified)
    }
}

impl Simplify for lir::Node {
    fn simplify(&mut self) -> bool {
        match self {
            Self::Let { expr, .. } => expr.simplify(),
            Self::Assert { cond } | Self::Assume { cond } => cond.simplify(),
            _ => false,
        }
    }
}

impl Simplify for Expression {
    fn simplify(&mut self) -> bool {
        // Simplify operands first
        let mut simplified = self
            .operands_mut()
            .into_iter()
            .fold(false, |simplified, operand| {
                operand.simplify() || simplified
            });

        // Try to simplify `Self`
        if let Some(expr) = simplified_expression(self) {
            *self = expr;
            simplified = true;
        }

        simplified
    }
}

fn simplified_expression(expr: &Expression) -> Option<Expression> {
    match (expr.operator(), expr.operands()) {
        (Operator::Equal, [lhs, rhs]) => match (lhs.operator(), rhs.operator()) {
            (_, Operator::Boolean(Boolean::True)) => Some(lhs.clone()), // b = true -> b
            (Operator::Boolean(Boolean::True), _) => Some(rhs.clone()), // true = b -> b
            (_, Operator::Boolean(Boolean::False)) => Boolean::not(lhs.clone()).ok(), // b = false -> not b
            (Operator::Boolean(Boolean::False), _) => Boolean::not(rhs.clone()).ok(), // false = b -> not b
            _ => {
                if lhs == rhs {
                    // a = a -> true
                    Some(Boolean::constant(true))
                } else {
                    None
                }
            }
        },
        (Operator::Ite, [cond, then, _else]) => match cond.operator() {
            Operator::Boolean(Boolean::True) => Some(then.clone()), // (ite true a b) -> a
            Operator::Boolean(Boolean::False) => Some(_else.clone()), // (ite false a b) -> b
            _ => {
                if then == _else {
                    // (ite c a a) -> a
                    Some(then.clone())
                } else {
                    None
                }
            }
        },
        (Operator::Boolean(op), operands) => simplified_boolean_expression(op, operands),
        (Operator::Integer(op), operands) => simplified_integer_expression(op, operands),
        (Operator::BitVector(op), operands) => simplified_bitvec_expression(op, operands),
        _ => None,
    }
}

fn simplified_boolean_expression(op: &Boolean, operands: &[Expression]) -> Option<Expression> {
    let is_true = |o: &Expression| o.is_constant() && bool::try_from(o).unwrap();
    let is_false = |o: &Expression| o.is_constant() && !bool::try_from(o).unwrap();

    match (op, operands) {
        (Boolean::Or, []) => Some(Boolean::constant(false)),
        (Boolean::Or, [operand]) => Some(operand.clone()), // (or a) -> a
        (Boolean::Or, operands) => {
            if operands.iter().any(is_true) {
                // (or a b true c) -> true
                Some(Boolean::constant(true))
            } else if operands.iter().any(is_false) {
                // (or a b false c) -> (or a b c)
                let ops_without_false: Vec<Expression> = operands
                    .into_iter()
                    .filter(|o| !is_false(o))
                    .cloned()
                    .collect();
                Boolean::disjunction(&ops_without_false).ok()
            } else {
                None
            }
        }
        (Boolean::And, []) => Some(Boolean::constant(true)),
        (Boolean::And, [operand]) => Some(operand.clone()), // (and a) -> a
        (Boolean::And, operands) => {
            if operands.iter().any(is_false) {
                // (and a b false c) -> false
                Some(Boolean::constant(false))
            } else if operands.iter().any(is_true) {
                // (and a b true c) -> (and a b c)
                let ops_without_true: Vec<Expression> = operands
                    .into_iter()
                    .filter(|o| !is_true(o))
                    .cloned()
                    .collect();
                Boolean::conjunction(&ops_without_true).ok()
            } else {
                None
            }
        }
        (Boolean::Not, [operand]) => match operand.operator() {
            Operator::Boolean(Boolean::False) => Some(Boolean::constant(true)), // not false -> true
            Operator::Boolean(Boolean::True) => Some(Boolean::constant(false)), // not true -> false
            Operator::Boolean(Boolean::Not) => Some(operand.operands()[0].clone()), // not not a -> a
            _ => None,
        },
        (Boolean::Imply, [a, b]) => {
            if is_false(a) || is_true(b) {
                // false => b -> true, a => true -> true
                Some(Boolean::constant(true))
            } else if is_true(a) {
                // true => b -> b
                Some(b.clone())
            } else if is_false(b) {
                // a => false -> not a
                Boolean::not(a.clone()).ok()
            } else {
                None
            }
        }
        _ => None,
    }
}

fn simplified_integer_expression(op: &Integer, operands: &[Expression]) -> Option<Expression> {
    match (op, operands) {
        (Integer::Sub, [lhs, rhs]) => match (lhs.operator(), rhs.operator()) {
            (_, Operator::Integer(Integer::Constant(0))) => Some(lhs.clone()), // a - 0 -> a
            (Operator::Integer(Integer::Constant(0)), _) => Integer::neg(rhs.clone()).ok(), // 0 - a -> -a
            _ => {
                if lhs == rhs {
                    // a - a -> 0
                    Some(Integer::constant(0))
                } else {
                    None
                }
            }
        },
        (Integer::Add, [lhs, rhs]) => match (lhs.operator(), rhs.operator()) {
            (_, Operator::Integer(Integer::Constant(0))) => Some(lhs.clone()), // a + 0 -> a
            (Operator::Integer(Integer::Constant(0)), _) => Some(rhs.clone()), // 0 + a -> a
            _ => None,
        },
        (Integer::Mul, [lhs, rhs]) => match (lhs.operator(), rhs.operator()) {
            (_, Operator::Integer(Integer::Constant(0))) => Some(Integer::constant(0)), // a * 0 -> 0
            (Operator::Integer(Integer::Constant(0)), _) => Some(Integer::constant(0)), // 0 * a -> 0
            (_, Operator::Integer(Integer::Constant(1))) => Some(lhs.clone()), // a * 1 -> a
            (Operator::Integer(Integer::Constant(1)), _) => Some(rhs.clone()), // 1 * a -> a
            _ => None,
        },
        _ => None,
    }
}

fn simplified_bitvec_expression(op: &BitVector, operands: &[Expression]) -> Option<Expression> {
    match (op, operands) {
        (BitVector::Add, [lhs, rhs]) => match (lhs.operator(), rhs.operator()) {
            (_, Operator::BitVector(BitVector::Constant(c))) => {
                if c.is_zero() {
                    // a + 0 -> a
                    Some(lhs.clone())
                } else {
                    None
                }
            }
            (Operator::BitVector(BitVector::Constant(c)), _) => {
                if c.is_zero() {
                    // 0 + a -> a
                    Some(rhs.clone())
                } else {
                    None
                }
            }
            _ => None,
        },
        (BitVector::Mul, [lhs, rhs]) => match (lhs.operator(), rhs.operator()) {
            (_, Operator::BitVector(BitVector::Constant(c))) => {
                if c.is_one() {
                    // a * 1 -> a
                    Some(lhs.clone())
                } else {
                    None
                }
            }
            (Operator::BitVector(BitVector::Constant(c)), _) => {
                if c.is_one() {
                    // 1 * a -> a
                    Some(rhs.clone())
                } else {
                    None
                }
            }
            _ => None,
        },
        (BitVector::ZeroExtend(bits), [operand]) => match operand.operator() {
            Operator::BitVector(BitVector::ZeroExtend(_)) => {
                // (zext (zext b)) -> (zext b)
                BitVector::zero_extend(*bits, operand.operands()[0].clone()).ok()
            }
            _ => None,
        },
        (BitVector::SignExtend(bits), [operand]) => match operand.operator() {
            Operator::BitVector(BitVector::SignExtend(_)) => {
                // (sext (sext b)) -> (sext b)
                BitVector::sign_extend(*bits, operand.operands()[0].clone()).ok()
            }
            _ => None,
        },
        _ => None,
    }
}
