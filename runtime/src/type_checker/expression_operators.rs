//! Type checking for binary expressions and operator result types.

use super::*;

fn is_money_applied(ty: &Type) -> bool {
    matches!(ty, Type::Applied { name, args } if name == "Money" && args.len() == 1)
}

impl Checker<'_> {
    pub(super) fn check_binary_chain(
        &mut self,
        expr: &Expr,
        scopes: &[HashMap<String, Variable>],
    ) -> Type {
        let mut cursor = expr;
        let mut operations = Vec::new();
        while let ExprKind::Binary { left, op, right } = &cursor.kind {
            operations.push((*op, right.as_ref(), cursor.span, left.span));
            cursor = left;
        }
        let mut left_type = self.check_expr(cursor, scopes);
        for (op, right, span, left_span) in operations.into_iter().rev() {
            let right_type = if matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) {
                self.check_expr_expected(right, scopes, Some(&left_type))
            } else {
                self.check_expr(right, scopes)
            };
            left_type = match op {
                BinaryOp::Add => {
                    if left_type == Type::String && right_type == Type::String {
                        Type::String
                    } else if left_type == Type::Duration && right_type == Type::Duration {
                        Type::Duration
                    } else if is_money_applied(&left_type) || is_money_applied(&right_type) {
                        self.money_binary_result(&left_type, &right_type, span)
                    } else if let (Type::Array(left), Type::Array(right)) =
                        (&left_type, &right_type)
                    {
                        self.expect_type(left, right, span);
                        Type::Array(left.clone())
                    } else {
                        self.numeric_result(&left_type, &right_type, span)
                    }
                }
                BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Remainder => {
                    if op == BinaryOp::Subtract
                        && left_type == Type::Duration
                        && right_type == Type::Duration
                    {
                        Type::Duration
                    } else if op == BinaryOp::Subtract
                        && (is_money_applied(&left_type) || is_money_applied(&right_type))
                    {
                        self.money_binary_result(&left_type, &right_type, span)
                    } else {
                        self.numeric_result(&left_type, &right_type, span)
                    }
                }
                BinaryOp::Equal | BinaryOp::NotEqual => {
                    if matches!(&left_type, Type::Function { .. })
                        || matches!(&right_type, Type::Function { .. })
                    {
                        self.error_with_code(
                            "E-TYPE-FUNCTION-001",
                            span,
                            "function values cannot be compared",
                        );
                    }
                    self.expect_type(&left_type, &right_type, right.span);
                    Type::Bool
                }
                BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual => {
                    if is_money_applied(&left_type) || is_money_applied(&right_type) {
                        self.money_binary_result(&left_type, &right_type, span);
                        Type::Bool
                    } else if left_type == right_type
                        && matches!(
                            left_type,
                            Type::String
                                | Type::Date
                                | Type::Time
                                | Type::Duration
                                | Type::Timestamp
                        )
                    {
                        Type::Bool
                    } else {
                        self.numeric_result(&left_type, &right_type, span);
                        Type::Bool
                    }
                }
                BinaryOp::And | BinaryOp::Or => {
                    self.expect_type(&Type::Bool, &left_type, left_span);
                    self.expect_type(&Type::Bool, &right_type, right.span);
                    Type::Bool
                }
            };
        }
        left_type
    }

    fn money_binary_result(&mut self, left: &Type, right: &Type, span: Span) -> Type {
        if left == &Type::Unknown || right == &Type::Unknown {
            return Type::Unknown;
        }
        if is_money_applied(left) && left == right {
            left.clone()
        } else {
            self.error(
                span,
                format!(
                    "cannot mix currencies `{left}` and `{right}`; convert explicitly with `convert_money`"
                ),
            );
            Type::Unknown
        }
    }

    fn numeric_result(&mut self, left: &Type, right: &Type, span: Span) -> Type {
        if left == &Type::Unknown || right == &Type::Unknown {
            return Type::Unknown;
        }
        if is_numeric(left) && left == right {
            left.clone()
        } else {
            self.error(
                span,
                format!("numeric operands must have the same type, found `{left}` and `{right}`"),
            );
            Type::Unknown
        }
    }
}
