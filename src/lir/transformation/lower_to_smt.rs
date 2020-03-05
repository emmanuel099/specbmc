use crate::error::Result;
use crate::expr::*;
use crate::lir::{Node, Program};
use crate::util::Transform;

pub struct LowerToSMT {}

impl LowerToSMT {
    pub fn new() -> Self {
        Self {}
    }
}

impl Transform<Program> for LowerToSMT {
    fn description(&self) -> &'static str {
        "Lowering expressions to SMT"
    }

    fn transform(&self, program: &mut Program) -> Result<()> {
        program.nodes_mut().iter_mut().for_each(|node| node.lower());

        Ok(())
    }
}

trait Lower {
    fn lower(&mut self);
}

impl Lower for Node {
    fn lower(&mut self) {
        match self {
            Self::Let { expr, .. } => expr.lower(),
            Self::Assert { condition } | Self::Assume { condition } => condition.lower(),
            _ => (),
        };
    }
}

impl Lower for Expression {
    fn lower(&mut self) {
        // Lower operands
        self.operands_mut()
            .iter_mut()
            .for_each(|operand| operand.lower());

        // Try to lower `Self`
        if let Some(expr) = lowered_expression(self) {
            *self = expr;
        }
    }
}

fn lowered_expression(expr: &Expression) -> Option<Expression> {
    match expr.operator() {
        Operator::BitVector(op) => lowered_bitvec_expression(op, expr.operands()),
        _ => None,
    }
}

fn lowered_bitvec_expression(op: &BitVector, operands: &[Expression]) -> Option<Expression> {
    match (op, operands) {
        (BitVector::ToBoolean, [expr]) => {
            let width = expr.sort().unwrap_bit_vector();
            let zero = BitVector::constant_u64(0, width);
            Expression::unequal(expr.clone(), zero).ok()
        }
        (BitVector::FromBoolean(bits), [expr]) => {
            let zero = BitVector::constant_u64(0, *bits);
            let one = BitVector::constant_u64(1, *bits);
            Expression::ite(expr.clone(), one, zero).ok()
        }
        _ => None,
    }
}
