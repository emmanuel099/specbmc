use crate::error::Result;
use crate::expr::*;
use crate::lir;
use crate::lir::optimization::{Optimization, OptimizationResult};
use std::convert::TryFrom;

pub struct ConstantFolding {}

impl ConstantFolding {
    pub fn new() -> Self {
        Self {}
    }
}

impl Optimization for ConstantFolding {
    fn optimize(&self, program: &mut lir::Program) -> Result<OptimizationResult> {
        for node in program.nodes_mut() {
            match node {
                lir::Node::Let { expr, .. } => {
                    if let Some(folded_expr) = folded_expression(expr) {
                        *expr = folded_expr;
                    }
                }
                lir::Node::Assert { cond } | lir::Node::Assume { cond } => {
                    if let Some(folded_expr) = folded_expression(cond) {
                        *cond = folded_expr;
                    }
                }
                _ => (),
            }
        }

        Ok(OptimizationResult::Changed)
    }
}

fn folded_expression(expr: &mut Expression) -> Option<Expression> {
    if expr.operands().is_empty() {
        // Nothing to fold
        return None;
    }

    // Fold operands first
    for operand in expr.operands_mut() {
        if let Some(folded_operand) = folded_expression(operand) {
            *operand = folded_operand;
        }
    }

    if expr.operands().iter().any(|operand| !operand.is_constant()) {
        // Not all operands are const
        return None;
    }

    match (expr.operator(), expr.operands()) {
        (Operator::Equal, [lhs, rhs]) => Some(Boolean::constant(lhs == rhs)),
        (Operator::Boolean(op), operands) => {
            let values: Vec<bool> = operands
                .iter()
                .map(|o| bool::try_from(o).unwrap())
                .collect();
            fold_boolean(op, &values)
        }
        (Operator::BitVector(op), operands) => {
            let values: Vec<BitVectorValue> = operands
                .iter()
                .map(|o| BitVectorValue::try_from(o).unwrap())
                .collect();
            fold_bitvec(op, &values)
        }
        _ => None,
    }
}

fn fold_boolean(op: &Boolean, values: &[bool]) -> Option<Expression> {
    use Boolean::*;
    match (op, values) {
        (Not, [v]) => Some(Boolean::constant(!v)),
        (Imply, [a, b]) => Some(Boolean::constant(!a || *b)),
        (And, values) => Some(Boolean::constant(
            values.iter().fold(true, |res, v| res && *v),
        )),
        (Or, values) => Some(Boolean::constant(
            values.iter().fold(false, |res, v| res || *v),
        )),
        (Xor, [a, b]) => Some(Boolean::constant(a ^ b)),
        _ => None,
    }
}

fn fold_bitvec(op: &BitVector, values: &[BitVectorValue]) -> Option<Expression> {
    use BitVector::*;
    match (op, values) {
        (ToBoolean, [v]) => Some(Boolean::constant(!v.is_zero())),
        (FromBoolean(i), [v]) => v.trun(*i).map(BitVector::constant_value).ok(),
        (Truncate(i), [v]) => v.trun(*i).map(BitVector::constant_value).ok(),
        (And, [lhs, rhs]) => lhs.and(rhs).map(BitVector::constant_value).ok(),
        (Or, [lhs, rhs]) => lhs.or(rhs).map(BitVector::constant_value).ok(),
        (Add, [lhs, rhs]) => lhs.add(rhs).map(BitVector::constant_value).ok(),
        (Mul, [lhs, rhs]) => lhs.mul(rhs).map(BitVector::constant_value).ok(),
        (UDiv, [lhs, rhs]) => lhs.divu(rhs).map(BitVector::constant_value).ok(),
        (Shl, [lhs, rhs]) => lhs.shl(rhs).map(BitVector::constant_value).ok(),
        (LShr, [lhs, rhs]) => lhs.shr(rhs).map(BitVector::constant_value).ok(),
        (Xor, [lhs, rhs]) => lhs.xor(rhs).map(BitVector::constant_value).ok(),
        (Sub, [lhs, rhs]) => lhs.sub(rhs).map(BitVector::constant_value).ok(),
        (SDiv, [lhs, rhs]) => lhs.divs(rhs).map(BitVector::constant_value).ok(),
        (SMod, [lhs, rhs]) => lhs.mods(rhs).map(BitVector::constant_value).ok(),
        (UMod, [lhs, rhs]) => lhs.modu(rhs).map(BitVector::constant_value).ok(),
        (ZeroExtend(i), [v]) => v.zext(*i).map(BitVector::constant_value).ok(),
        (SignExtend(i), [v]) => v.sext(*i).map(BitVector::constant_value).ok(),
        _ => None,
    }
}
