//! Shared value-level operator semantics for the AST, MIR, and bytecode runtimes.

use super::*;

fn operator_error(span: Span, message: impl Into<String>) -> RuntimeError {
    RuntimeError {
        message: message.into(),
        span,
    }
}

/// Index a Zelyra array or tuple with the same checked semantics in all
/// execution backends. String, map, and JSON indexing have backend-specific
/// representations and are intentionally handled by their respective paths.
pub fn index_collection_value(
    target: Value,
    index: Value,
    span: Span,
) -> Result<Value, RuntimeError> {
    let (kind, values) = match target {
        Value::Array(values) => ("array", values),
        Value::Tuple(values) => ("tuple", values),
        other => {
            return Err(operator_error(
                span,
                format!("indexing requires an array or Tuple, found {}", other.ty()),
            ));
        }
    };
    let index = match index {
        Value::Int(index) if index >= 0 => usize::try_from(index)
            .map_err(|_| operator_error(span, format!("{kind} index is too large")))?,
        Value::Int(_) => {
            return Err(operator_error(
                span,
                format!("{kind} index must not be negative"),
            ));
        }
        Value::UInt(index) => usize::try_from(index)
            .map_err(|_| operator_error(span, format!("{kind} index is too large")))?,
        other => {
            return Err(operator_error(
                span,
                format!("{kind} index must be Int or UInt, found {}", other.ty()),
            ));
        }
    };
    values
        .get(index)
        .cloned()
        .ok_or_else(|| operator_error(span, format!("{kind} index {index} is out of bounds")))
}

/// Apply one already-evaluated binary operation using Zelyra value semantics.
/// Short-circuit evaluation remains the responsibility of each interpreter.
pub fn evaluate_binary_value(
    left: Value,
    op: BinaryOp,
    right: Value,
    span: Span,
) -> Result<Value, RuntimeError> {
    use BinaryOp::*;
    match (left, op, right) {
        (Value::Int(a), Add, Value::Int(b)) => a
            .checked_add(b)
            .map(Value::Int)
            .ok_or_else(|| operator_error(span, "Int overflow")),
        (Value::Int(a), Subtract, Value::Int(b)) => a
            .checked_sub(b)
            .map(Value::Int)
            .ok_or_else(|| operator_error(span, "Int overflow")),
        (Value::Int(a), Multiply, Value::Int(b)) => a
            .checked_mul(b)
            .map(Value::Int)
            .ok_or_else(|| operator_error(span, "Int overflow")),
        (Value::Int(_), Divide | Remainder, Value::Int(0)) => {
            Err(operator_error(span, "division by zero"))
        }
        (Value::Int(i64::MIN), Divide | Remainder, Value::Int(-1)) => {
            Err(operator_error(span, "Int overflow"))
        }
        (Value::Int(a), Divide, Value::Int(b)) => a
            .checked_div(b)
            .map(Value::Int)
            .ok_or_else(|| operator_error(span, "Int overflow")),
        (Value::Int(a), Remainder, Value::Int(b)) => a
            .checked_rem(b)
            .map(Value::Int)
            .ok_or_else(|| operator_error(span, "Int overflow")),
        (Value::UInt(a), Add, Value::UInt(b)) => a
            .checked_add(b)
            .map(Value::UInt)
            .ok_or_else(|| operator_error(span, "UInt overflow")),
        (Value::UInt(a), Subtract, Value::UInt(b)) => a
            .checked_sub(b)
            .map(Value::UInt)
            .ok_or_else(|| operator_error(span, "UInt underflow")),
        (Value::UInt(a), Multiply, Value::UInt(b)) => a
            .checked_mul(b)
            .map(Value::UInt)
            .ok_or_else(|| operator_error(span, "UInt overflow")),
        (Value::UInt(_), Divide | Remainder, Value::UInt(0)) => {
            Err(operator_error(span, "division by zero"))
        }
        (Value::UInt(a), Divide, Value::UInt(b)) => Ok(Value::UInt(a / b)),
        (Value::UInt(a), Remainder, Value::UInt(b)) => Ok(Value::UInt(a % b)),
        (Value::Float(a), Add, Value::Float(b)) => Ok(Value::Float(a + b)),
        (Value::Float(a), Subtract, Value::Float(b)) => Ok(Value::Float(a - b)),
        (Value::Float(a), Multiply, Value::Float(b)) => Ok(Value::Float(a * b)),
        (Value::Float(a), Divide, Value::Float(b)) => Ok(Value::Float(a / b)),
        (Value::Float(a), Remainder, Value::Float(b)) => Ok(Value::Float(a % b)),
        (Value::Decimal(a), Add, Value::Decimal(b)) => a
            .checked_add(b)
            .map(Value::Decimal)
            .ok_or_else(|| operator_error(span, "Decimal overflow")),
        (Value::Decimal(a), Subtract, Value::Decimal(b)) => a
            .checked_sub(b)
            .map(Value::Decimal)
            .ok_or_else(|| operator_error(span, "Decimal overflow")),
        (Value::Decimal(a), Multiply, Value::Decimal(b)) => {
            decimal_math::scaled_product(a, b, DECIMAL_SCALE)
                .map(Value::Decimal)
                .ok_or_else(|| operator_error(span, "Decimal overflow"))
        }
        (Value::Decimal(_), Divide, Value::Decimal(0)) => {
            Err(operator_error(span, "division by zero"))
        }
        (Value::Decimal(a), Divide, Value::Decimal(b)) => {
            decimal_math::scaled_product(a, DECIMAL_SCALE, b)
                .map(Value::Decimal)
                .ok_or_else(|| operator_error(span, "Decimal overflow"))
        }
        (Value::Decimal(_), Remainder, Value::Decimal(0)) => {
            Err(operator_error(span, "division by zero"))
        }
        (Value::Decimal(a), Remainder, Value::Decimal(b)) => {
            // i128::MIN % -1 is mathematically zero, but machine division traps.
            Ok(Value::Decimal(if b == -1 { 0 } else { a % b }))
        }
        (Value::String(a), Add, Value::String(b)) => Ok(Value::String(a + &b)),
        (Value::Duration(a), Add, Value::Duration(b)) => a
            .checked_add(b)
            .map(Value::Duration)
            .ok_or_else(|| operator_error(span, "Duration overflow")),
        (Value::Duration(a), Subtract, Value::Duration(b)) => a
            .checked_sub(b)
            .map(Value::Duration)
            .ok_or_else(|| operator_error(span, "Duration overflow")),
        (Value::Array(mut a), Add, Value::Array(b)) => {
            a.extend(b);
            Ok(Value::Array(a))
        }
        (Value::Function(_), Equal | NotEqual, _) | (_, Equal | NotEqual, Value::Function(_)) => {
            Err(operator_error(span, "function values cannot be compared"))
        }
        (a, Equal, b) => Ok(Value::Bool(a == b)),
        (a, NotEqual, b) => Ok(Value::Bool(a != b)),
        (Value::Int(a), Less, Value::Int(b)) => Ok(Value::Bool(a < b)),
        (Value::Int(a), LessEqual, Value::Int(b)) => Ok(Value::Bool(a <= b)),
        (Value::Int(a), Greater, Value::Int(b)) => Ok(Value::Bool(a > b)),
        (Value::Int(a), GreaterEqual, Value::Int(b)) => Ok(Value::Bool(a >= b)),
        (Value::UInt(a), Less, Value::UInt(b)) => Ok(Value::Bool(a < b)),
        (Value::UInt(a), LessEqual, Value::UInt(b)) => Ok(Value::Bool(a <= b)),
        (Value::UInt(a), Greater, Value::UInt(b)) => Ok(Value::Bool(a > b)),
        (Value::UInt(a), GreaterEqual, Value::UInt(b)) => Ok(Value::Bool(a >= b)),
        (Value::Float(a), Less, Value::Float(b)) => Ok(Value::Bool(a < b)),
        (Value::Float(a), LessEqual, Value::Float(b)) => Ok(Value::Bool(a <= b)),
        (Value::Float(a), Greater, Value::Float(b)) => Ok(Value::Bool(a > b)),
        (Value::Float(a), GreaterEqual, Value::Float(b)) => Ok(Value::Bool(a >= b)),
        (Value::Decimal(a), Less, Value::Decimal(b)) => Ok(Value::Bool(a < b)),
        (Value::Decimal(a), LessEqual, Value::Decimal(b)) => Ok(Value::Bool(a <= b)),
        (Value::Decimal(a), Greater, Value::Decimal(b)) => Ok(Value::Bool(a > b)),
        (Value::Decimal(a), GreaterEqual, Value::Decimal(b)) => Ok(Value::Bool(a >= b)),
        (Value::String(a), Less, Value::String(b)) => Ok(Value::Bool(a < b)),
        (Value::String(a), LessEqual, Value::String(b)) => Ok(Value::Bool(a <= b)),
        (Value::String(a), Greater, Value::String(b)) => Ok(Value::Bool(a > b)),
        (Value::String(a), GreaterEqual, Value::String(b)) => Ok(Value::Bool(a >= b)),
        (Value::Date(a), Less, Value::Date(b)) | (Value::Time(a), Less, Value::Time(b)) => {
            Ok(Value::Bool(a < b))
        }
        (Value::Date(a), LessEqual, Value::Date(b))
        | (Value::Time(a), LessEqual, Value::Time(b)) => Ok(Value::Bool(a <= b)),
        (Value::Date(a), Greater, Value::Date(b)) | (Value::Time(a), Greater, Value::Time(b)) => {
            Ok(Value::Bool(a > b))
        }
        (Value::Date(a), GreaterEqual, Value::Date(b))
        | (Value::Time(a), GreaterEqual, Value::Time(b)) => Ok(Value::Bool(a >= b)),
        (Value::Timestamp(a), Less, Value::Timestamp(b)) => Ok(Value::Bool(a < b)),
        (Value::Timestamp(a), LessEqual, Value::Timestamp(b)) => Ok(Value::Bool(a <= b)),
        (Value::Timestamp(a), Greater, Value::Timestamp(b)) => Ok(Value::Bool(a > b)),
        (Value::Timestamp(a), GreaterEqual, Value::Timestamp(b)) => Ok(Value::Bool(a >= b)),
        (Value::Duration(a), Less, Value::Duration(b)) => Ok(Value::Bool(a < b)),
        (Value::Duration(a), LessEqual, Value::Duration(b)) => Ok(Value::Bool(a <= b)),
        (Value::Duration(a), Greater, Value::Duration(b)) => Ok(Value::Bool(a > b)),
        (Value::Duration(a), GreaterEqual, Value::Duration(b)) => Ok(Value::Bool(a >= b)),
        (Value::Bool(a), And, Value::Bool(b)) => Ok(Value::Bool(a && b)),
        (Value::Bool(a), Or, Value::Bool(b)) => Ok(Value::Bool(a || b)),
        (left, op, right) => Err(operator_error(
            span,
            format!(
                "invalid operation {op:?} for {} and {}",
                left.ty(),
                right.ty()
            ),
        )),
    }
}

/// Apply one already-evaluated unary operation using Zelyra value semantics.
pub fn evaluate_unary_value(op: UnaryOp, value: Value, span: Span) -> Result<Value, RuntimeError> {
    match (op, value) {
        (UnaryOp::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
        (UnaryOp::Negate, Value::Int(value)) => value
            .checked_neg()
            .map(Value::Int)
            .ok_or_else(|| operator_error(span, "Int overflow")),
        (UnaryOp::Negate, Value::UInt(_)) => Err(operator_error(span, "cannot negate UInt")),
        (UnaryOp::Negate, Value::Float(value)) => Ok(Value::Float(-value)),
        (UnaryOp::Negate, Value::Decimal(value)) => value
            .checked_neg()
            .map(Value::Decimal)
            .ok_or_else(|| operator_error(span, "Decimal overflow")),
        (_, value) => Err(operator_error(
            span,
            format!("invalid unary operand of type {}", value.ty()),
        )),
    }
}
