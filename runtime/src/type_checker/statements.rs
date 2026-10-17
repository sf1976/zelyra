//! Statement validation and scope updates for the shared compiler checker.

use super::*;

impl Checker<'_> {
    pub(super) fn check_statement(
        &mut self,
        statement: &Stmt,
        scopes: &mut Vec<HashMap<String, Variable>>,
        expected: &Type,
    ) {
        match statement {
            Stmt::Let {
                name,
                ty,
                value,
                mutable,
                span,
            } => {
                if let Some(declared) = ty {
                    self.check_type(declared, *span);
                }
                let actual = self.check_expr_expected(value, scopes, ty.as_ref());
                if let Some(declared) = ty {
                    self.expect_type(declared, &actual, value.span);
                }
                let final_ty = ty.clone().unwrap_or(actual);
                if scopes
                    .last_mut()
                    .unwrap()
                    .insert(
                        name.clone(),
                        Variable {
                            ty: final_ty,
                            mutable: *mutable,
                        },
                    )
                    .is_some()
                {
                    self.error(
                        *span,
                        format!("variable `{name}` is already declared in this scope"),
                    );
                }
            }
            Stmt::BindOrAssign { name, value, span } => {
                if let Some(existing) = self.lookup(scopes, name).cloned() {
                    if !existing.mutable {
                        self.error(
                            *span,
                            format!("cannot assign to immutable variable `{name}`"),
                        );
                    }
                    let actual = self.check_expr_expected(value, scopes, Some(&existing.ty));
                    self.expect_type(&existing.ty, &actual, value.span);
                } else {
                    let actual = self.check_expr(value, scopes);
                    scopes.last_mut().unwrap().insert(
                        name.clone(),
                        Variable {
                            ty: actual,
                            mutable: false,
                        },
                    );
                }
            }
            Stmt::ArrayIndexAssign {
                name,
                index,
                value,
                operation,
                span,
            } => {
                let Some(target) = self.lookup(scopes, name).cloned() else {
                    self.error(*span, format!("unknown variable `{name}`"));
                    self.check_expr(index, scopes);
                    self.check_expr(value, scopes);
                    return;
                };
                if !target.mutable {
                    self.error(
                        *span,
                        format!("indexed updates require a mutable array binding `{name}`"),
                    );
                }
                let element_type = match target.ty {
                    Type::Array(element) => *element,
                    other => {
                        self.error(
                            *span,
                            format!("indexed update expects an array, found `{other}`"),
                        );
                        self.check_expr(index, scopes);
                        self.check_expr(value, scopes);
                        return;
                    }
                };
                let index_type = self.check_expr(index, scopes);
                self.expect_type(&Type::Int, &index_type, index.span);
                let value_type = self.check_expr(value, scopes);
                self.expect_type(&element_type, &value_type, value.span);
                if *operation == ArrayIndexAssignOp::Add
                    && !matches!(
                        element_type,
                        Type::Int
                            | Type::UInt
                            | Type::Float
                            | Type::Decimal
                            | Type::String
                            | Type::Duration
                    )
                {
                    self.error_with_code(
                        "E-TYPE-ARRAY-001",
                        *span,
                        format!("`+=` is not supported for array element type `{element_type}`"),
                    );
                }
            }
            Stmt::TupleBindOrAssign { names, value, span } => {
                let actual = self.check_expr(value, scopes);
                let item_types = match actual {
                    Type::Tuple(items) if items.len() == names.len() => items,
                    Type::Tuple(items) => {
                        self.error_with_code(
                            "E-TYPE-TUPLE-001",
                            value.span,
                            format!(
                                "tuple assignment has {} variables but the value has {} items",
                                names.len(),
                                items.len()
                            ),
                        );
                        vec![Type::Unknown; names.len()]
                    }
                    Type::Unknown => vec![Type::Unknown; names.len()],
                    other => {
                        self.error_with_code(
                            "E-TYPE-TUPLE-001",
                            value.span,
                            format!("tuple assignment expects a tuple, found `{other}`"),
                        );
                        vec![Type::Unknown; names.len()]
                    }
                };
                for (name, item_type) in names.iter().zip(item_types) {
                    if let Some(existing) = self.lookup(scopes, name).cloned() {
                        if !existing.mutable {
                            self.error(
                                *span,
                                format!("cannot assign to immutable variable `{name}`"),
                            );
                        }
                        self.expect_type(&existing.ty, &item_type, value.span);
                    } else {
                        scopes.last_mut().unwrap().insert(
                            name.clone(),
                            Variable {
                                ty: item_type,
                                mutable: false,
                            },
                        );
                    }
                }
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr, scopes);
            }
            Stmt::Return { value, span } => {
                let actual = value
                    .as_ref()
                    .map(|expr| self.check_expr_expected(expr, scopes, Some(expected)))
                    .unwrap_or(Type::Unit);
                if *expected != Type::Unknown {
                    self.expect_type(expected, &actual, *span);
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let condition_type = self.check_expr(condition, scopes);
                self.expect_type(&Type::Bool, &condition_type, condition.span);
                self.check_block(then_block, scopes, expected);
                if let Some(block) = else_block {
                    self.check_block(block, scopes, expected);
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                let condition_type = self.check_expr(condition, scopes);
                self.expect_type(&Type::Bool, &condition_type, condition.span);
                for invariant in invariants {
                    let invariant_type = self.check_expr(invariant, scopes);
                    self.expect_type(&Type::Bool, &invariant_type, invariant.span);
                }
                self.loop_depth += 1;
                self.check_block(body, scopes, expected);
                self.loop_depth -= 1;
            }
            Stmt::For {
                name,
                additional_names,
                iterable,
                body,
                ..
            } => {
                let iterable_type = self.check_expr(iterable, scopes);
                let element_type = match iterable_type {
                    Type::Array(inner) => *inner,
                    Type::Set(inner) => *inner,
                    Type::Unknown => Type::Unknown,
                    other => {
                        self.error(
                            iterable.span,
                            format!("`for ... in` expects an array or Set, found `{other}`"),
                        );
                        Type::Unknown
                    }
                };
                let binding_count = additional_names.len() + 1;
                let binding_types = if binding_count == 1 {
                    vec![element_type]
                } else {
                    match element_type {
                        Type::Tuple(items) if items.len() == binding_count => items,
                        Type::Tuple(items) => {
                            self.error(
                                iterable.span,
                                format!(
                                    "tuple loop pattern has {binding_count} variables but the value has {} elements",
                                    items.len()
                                ),
                            );
                            vec![Type::Unknown; binding_count]
                        }
                        Type::Unknown => vec![Type::Unknown; binding_count],
                        other => {
                            self.error(
                                iterable.span,
                                format!("tuple loop pattern expects tuple values, found `{other}`"),
                            );
                            vec![Type::Unknown; binding_count]
                        }
                    }
                };
                scopes.push(HashMap::new());
                scopes.last_mut().unwrap().insert(
                    name.clone(),
                    Variable {
                        ty: binding_types[0].clone(),
                        mutable: false,
                    },
                );
                for (name, ty) in additional_names
                    .iter()
                    .zip(binding_types.into_iter().skip(1))
                {
                    scopes
                        .last_mut()
                        .unwrap()
                        .insert(name.clone(), Variable { ty, mutable: false });
                }
                self.loop_depth += 1;
                self.check_block(body, scopes, expected);
                self.loop_depth -= 1;
                scopes.pop();
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    let invariant_type = self.check_expr(invariant, scopes);
                    self.expect_type(&Type::Bool, &invariant_type, invariant.span);
                }
                self.loop_depth += 1;
                self.check_block(body, scopes, expected);
                self.loop_depth -= 1;
            }
            Stmt::Break { span } => {
                if self.loop_depth == 0 {
                    self.error(*span, "`break` is only valid inside a loop");
                }
            }
            Stmt::Continue { span } => {
                if self.loop_depth == 0 {
                    self.error(*span, "`continue` is only valid inside a loop");
                }
            }
            Stmt::Match { value, arms, span } => {
                let value_type = self.check_expr(value, scopes);
                let mut covered = std::collections::HashSet::new();
                for arm in arms {
                    let mut pattern_scope = HashMap::new();
                    self.check_pattern(
                        &arm.pattern,
                        &value_type,
                        &mut pattern_scope,
                        &mut covered,
                        arm.guard.is_none(),
                    );
                    scopes.push(pattern_scope);
                    if let Some(guard) = &arm.guard {
                        let guard_type = self.check_expr(guard, scopes);
                        self.expect_type(&Type::Bool, &guard_type, guard.span);
                    }
                    self.check_block(&arm.body, scopes, expected);
                    scopes.pop();
                }
                self.check_exhaustiveness(&value_type, &covered, *span);
            }
            Stmt::Transaction { body, .. } => {
                self.transaction_depth += 1;
                self.check_block(body, scopes, expected);
                self.transaction_depth -= 1;
            }
            Stmt::Parallel { body, span } => {
                let mut names = HashSet::new();
                let mut new_bindings = Vec::new();
                if body.statements.len() > MAX_PARALLEL_BRANCHES {
                    self.error(
                        *span,
                        format!(
                            "parallel block exceeds the maximum of {MAX_PARALLEL_BRANCHES} branches"
                        ),
                    );
                }
                self.parallel_depth += 1;
                for statement in &body.statements {
                    let Stmt::BindOrAssign { name, value, span } = statement else {
                        self.error(
                            *span,
                            "parallel blocks may contain only `name = await expression` bindings",
                        );
                        continue;
                    };
                    if !matches!(&value.kind, ExprKind::Await(_)) {
                        self.error(value.span, "parallel bindings must await their expression");
                    }
                    if !names.insert(name.clone()) {
                        self.error(
                            *span,
                            format!("parallel binding `{name}` is declared twice"),
                        );
                    }
                    let actual = self.check_expr(value, scopes);
                    if let Some(existing) = self.lookup(scopes, name).cloned() {
                        if !existing.mutable {
                            self.error(
                                *span,
                                format!("cannot assign to immutable variable `{name}`"),
                            );
                        }
                        self.expect_type(&existing.ty, &actual, value.span);
                    } else {
                        new_bindings.push((name.clone(), actual));
                    }
                }
                self.parallel_depth -= 1;
                for (name, ty) in new_bindings {
                    scopes
                        .last_mut()
                        .unwrap()
                        .insert(name, Variable { ty, mutable: false });
                }
            }
        }
    }
}
