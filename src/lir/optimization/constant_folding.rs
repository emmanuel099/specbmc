//! Constant Folding
//!
//! Tries to evaluate expressions to constants if all their operands are constant,
//! e.g. `1 + 2` will become `3`.
//!
//! Please note that some operators aren't yet implemented.

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
            .iter_mut()
            .fold(false, |folded, node| node.fold() || folded)
    }
}

impl Fold for lir::Node {
    fn fold(&mut self) -> bool {
        match self {
            Self::Let { expr, .. } => expr.fold(),
            Self::Assert { condition } | Self::Assume { condition } => condition.fold(),
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
            .iter_mut()
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
        (FromBoolean(i), [v]) => v.zext(*i).map(BitVector::constant).ok(),
        (Truncate(i), [v]) => v.trun(*i).map(BitVector::constant).ok(),
        (And, [lhs, rhs]) => lhs.and(rhs).map(BitVector::constant).ok(),
        (Or, [lhs, rhs]) => lhs.or(rhs).map(BitVector::constant).ok(),
        (Add, [lhs, rhs]) => lhs.add(rhs).map(BitVector::constant).ok(),
        (Mul, [lhs, rhs]) => lhs.mul(rhs).map(BitVector::constant).ok(),
        (UDiv, [lhs, rhs]) => lhs.divu(rhs).map(BitVector::constant).ok(),
        (Shl, [lhs, rhs]) => lhs.shl(rhs).map(BitVector::constant).ok(),
        (LShr, [lhs, rhs]) => lhs.shr(rhs).map(BitVector::constant).ok(),
        (Xor, [lhs, rhs]) => lhs.xor(rhs).map(BitVector::constant).ok(),
        (Sub, [lhs, rhs]) => lhs.sub(rhs).map(BitVector::constant).ok(),
        (SDiv, [lhs, rhs]) => lhs.divs(rhs).map(BitVector::constant).ok(),
        (SMod, [lhs, rhs]) => lhs.mods(rhs).map(BitVector::constant).ok(),
        (UMod, [lhs, rhs]) => lhs.modu(rhs).map(BitVector::constant).ok(),
        (ZeroExtend(i), [v]) => v.zext(*i + v.bits()).map(BitVector::constant).ok(),
        (SignExtend(i), [v]) => v.sext(*i + v.bits()).map(BitVector::constant).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold_bitvec_to_boolean_1_should_give_true() {
        // GIVEN
        let mut expr = BitVector::to_boolean(BitVector::constant_u64(1, 32)).unwrap();

        // WHEN
        expr.fold();

        // THEN
        assert_eq!(expr, Boolean::constant(true));
    }

    #[test]
    fn test_fold_bitvec_to_boolean_42_should_give_true() {
        // GIVEN
        let mut expr = BitVector::to_boolean(BitVector::constant_u64(42, 32)).unwrap();

        // WHEN
        expr.fold();

        // THEN
        assert_eq!(expr, Boolean::constant(true));
    }

    #[test]
    fn test_fold_bitvec_to_boolean_0_should_give_false() {
        // GIVEN
        let mut expr = BitVector::to_boolean(BitVector::constant_u64(0, 32)).unwrap();

        // WHEN
        expr.fold();

        // THEN
        assert_eq!(expr, Boolean::constant(false));
    }

    #[test]
    fn test_fold_bitvec_from_boolean_true_should_give_1() {
        // GIVEN
        let mut expr = BitVector::from_boolean(32, Boolean::constant(true)).unwrap();

        // WHEN
        expr.fold();

        // THEN
        assert_eq!(expr, BitVector::constant_u64(1, 32));
    }

    #[test]
    fn test_fold_bitvec_from_boolean_false_should_give_0() {
        // GIVEN
        let mut expr = BitVector::from_boolean(32, Boolean::constant(false)).unwrap();

        // WHEN
        expr.fold();

        // THEN
        assert_eq!(expr, BitVector::constant_u64(0, 32));
    }

    #[test]
    fn test_fold_bitvec_zero_extend() {
        // GIVEN
        let mut expr = BitVector::zero_extend(24, BitVector::constant_u64(42, 8)).unwrap();

        // WHEN
        expr.fold();

        // THEN
        assert_eq!(expr, BitVector::constant_u64(42, 32));
    }

    #[test]
    fn test_fold_bitvec_sign_extend() {
        // GIVEN
        let mut expr = BitVector::sign_extend(24, BitVector::constant_u64(42, 8)).unwrap();

        // WHEN
        expr.fold();

        // THEN
        assert_eq!(expr, BitVector::constant_u64(42, 32));
    }
}
