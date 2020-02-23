use crate::error::Result;
use crate::expr::*;
use crate::lir;
use crate::lir::optimization::OptimizationResult;

pub fn simplify_expressions(program: &mut lir::Program) -> Result<OptimizationResult> {
    for node in program.nodes_mut() {
        match node {
            lir::Node::Let { expr, .. } => {
                if let Some(simplified_expr) = simplified_expression(expr) {
                    *expr = simplified_expr;
                }
            }
            lir::Node::Assert { cond } | lir::Node::Assume { cond } => {
                if let Some(simplified_cond) = simplified_expression(cond) {
                    *cond = simplified_cond;
                }
            }
            _ => (),
        }
    }

    Ok(OptimizationResult::Changed)
}

fn simplified_expression(expr: &mut Expression) -> Option<Expression> {
    // Simplify operands first
    for operand in expr.operands_mut() {
        if let Some(simplified_operand) = simplified_expression(operand) {
            *operand = simplified_operand;
        }
    }

    match (expr.operator(), expr.operands()) {
        (Operator::Equal, [lhs, rhs]) => match (lhs.operator(), rhs.operator()) {
            (_, Operator::Boolean(Boolean::True)) => Some(lhs.clone()), // b = true -> b
            (Operator::Boolean(Boolean::True), _) => Some(rhs.clone()), // true = b -> b
            (_, Operator::Boolean(Boolean::False)) => Boolean::not(lhs.clone()).ok(), // b = false -> not b
            (Operator::Boolean(Boolean::False), _) => Boolean::not(rhs.clone()).ok(), // false = b -> not b
            _ => None,
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
    match (op, operands) {
        (Boolean::Or, []) => Some(Boolean::constant(false)),
        (Boolean::Or, [operand]) => Some(operand.clone()), // (or a) -> a
        (Boolean::And, []) => Some(Boolean::constant(true)),
        (Boolean::And, [operand]) => Some(operand.clone()), // (and a) -> a
        (Boolean::Not, [operand]) => match operand.operator() {
            Operator::Boolean(Boolean::False) => Some(Boolean::constant(true)), // not false -> true
            Operator::Boolean(Boolean::True) => Some(Boolean::constant(false)), // not true -> false
            Operator::Boolean(Boolean::Not) => Some(operand.operands()[0].clone()), // not not a -> a
            _ => None,
        },
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
