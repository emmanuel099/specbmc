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
        if program.fold() {
            Ok(OptimizationResult::Changed)
        } else {
            Ok(OptimizationResult::Unchanged)
        }
    }
}

trait Fold {
    /// Fold `Self`
    ///
    /// Returns true if something changed.
    fn fold(&mut self) -> bool;
}

impl Fold for lir::Program {
    fn fold(&mut self) -> bool {
        self.nodes_mut()
            .into_iter()
            .fold(false, |folded, node| node.fold() || folded)
    }
}

impl Fold for lir::Node {
    fn fold(&mut self) -> bool {
        match self {
            Self::Let { expr, .. } => expr.fold(),
            Self::Assert { cond } | Self::Assume { cond } => cond.fold(),
            _ => false,
        }
    }
}

impl Fold for Expression {
    fn fold(&mut self) -> bool {
        if self.operands().is_empty() {
            // Nothing to fold
            return false;
        }

        // Fold operands first
        let mut folded = self
            .operands_mut()
            .into_iter()
            .fold(false, |folded, operand| operand.fold() || folded);

        if self.operands().iter().any(|operand| !operand.is_constant()) {
            // Not all operands are constant
            return folded;
        }

        // Try to fold `Self`
        match (self.operator(), self.operands()) {
            (Operator::Equal, [lhs, rhs]) => {
                *self = Boolean::constant(lhs == rhs);
                folded = true;
            }
            (Operator::Boolean(op), operands) => {
                let values: Vec<bool> = operands
                    .iter()
                    .map(|o| bool::try_from(o).unwrap())
                    .collect();
                if let Some(result) = evaluate_boolean(op, &values) {
                    *self = result;
                    folded = true;
                }
            }
            (Operator::BitVector(op), operands) => {
                let values: Vec<BitVectorValue> = operands
                    .iter()
                    .map(|o| BitVectorValue::try_from(o).unwrap())
                    .collect();
                if let Some(result) = evaluate_bitvec(op, &values) {
                    *self = result;
                    folded = true;
                }
            }
            _ => (),
        }

        folded
    }
}

fn evaluate_boolean(op: &Boolean, values: &[bool]) -> Option<Expression> {
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

fn evaluate_bitvec(op: &BitVector, values: &[BitVectorValue]) -> Option<Expression> {
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
