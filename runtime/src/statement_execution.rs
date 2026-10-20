use super::*;

impl<'program, 'services> Interpreter<'program, 'services> {
    fn check_loop_invariants(
        &mut self,
        invariants: &[Expr],
        env: &mut Environment,
        span: Span,
        phase: &str,
    ) -> Result<(), RuntimeError> {
        for invariant in invariants {
            let value = self.eval(invariant, env)?;
            if !self.expect_bool(value, invariant.span)? {
                return Err(self.runtime_error(
                    span,
                    format!("loop invariant failed {phase} loop iteration"),
                ));
            }
        }
        Ok(())
    }
    pub(super) fn exec_block(
        &mut self,
        block: &Block,
        env: &mut Environment,
    ) -> Result<Flow, RuntimeError> {
        env.with_scope(|env| {
            for statement in &block.statements {
                match self.exec_statement(statement, env)? {
                    Flow::Continue => {}
                    flow => return Ok(flow),
                }
            }
            Ok(Flow::Continue)
        })
    }
    fn exec_statement(
        &mut self,
        statement: &Stmt,
        env: &mut Environment,
    ) -> Result<Flow, RuntimeError> {
        self.consume_step(statement_span(statement))?;
        match statement {
            Stmt::Let {
                name,
                value,
                mutable,
                ..
            } => {
                let value = self.eval(value, env)?;
                env.declare(name.clone(), value, *mutable);
                Ok(Flow::Continue)
            }
            Stmt::BindOrAssign { name, value, span } => {
                let value = self.eval(value, env)?;
                if env.contains(name) {
                    env.assign(name, value).map_err(|reason| {
                        self.runtime_error(
                            *span,
                            if reason == "immutable" {
                                format!("cannot assign to immutable variable `{name}`")
                            } else {
                                format!("unknown variable `{name}`")
                            },
                        )
                    })?;
                } else {
                    env.declare(name.clone(), value, false);
                }
                Ok(Flow::Continue)
            }
            Stmt::ArrayIndexAssign {
                name,
                index,
                value,
                operation,
                span,
            } => {
                let index_value = self.eval(index, env)?;
                let Value::Int(index_value) = index_value else {
                    return Err(self.runtime_error(*span, "mutable array index must be Int"));
                };
                let index = usize::try_from(index_value).map_err(|_| {
                    self.runtime_error(index.span, "array index cannot be negative")
                })?;
                let current = if *operation == ArrayIndexAssignOp::Add {
                    Some(
                        env.mutable_array_element(name, index)
                            .map_err(|reason| self.runtime_error(*span, reason))?,
                    )
                } else {
                    None
                };
                let value = self.eval(value, env)?;
                let updated = if let Some(current) = current {
                    self.binary(current, BinaryOp::Add, value, *span)?
                } else {
                    value
                };
                env.assign_mutable_array_element(name, index, updated)
                    .map_err(|reason| self.runtime_error(*span, reason))?;
                Ok(Flow::Continue)
            }
            Stmt::TupleBindOrAssign { names, value, span } => {
                let value = self.eval(value, env)?;
                let Value::Tuple(values) = value else {
                    return Err(self.runtime_error(
                        *span,
                        format!("tuple assignment expects a tuple, found `{}`", value.ty()),
                    ));
                };
                if values.len() != names.len() {
                    return Err(self.runtime_error(
                        *span,
                        format!(
                            "tuple assignment has {} variables but the value has {} items",
                            names.len(),
                            values.len()
                        ),
                    ));
                }
                for name in names {
                    if env.is_mutable(name) == Some(false) {
                        return Err(self.runtime_error(
                            *span,
                            format!("cannot assign to immutable variable `{name}`"),
                        ));
                    }
                }
                for (name, value) in names.iter().zip(values) {
                    if env.contains(name) {
                        env.assign(name, value).map_err(|_| {
                            self.runtime_error(
                                *span,
                                format!("cannot assign to immutable variable `{name}`"),
                            )
                        })?;
                    } else {
                        env.declare(name.clone(), value, false);
                    }
                }
                Ok(Flow::Continue)
            }
            Stmt::Expr(expr) => {
                self.eval(expr, env)?;
                Ok(Flow::Continue)
            }
            Stmt::Return { value, .. } => Ok(Flow::Return(
                value
                    .as_ref()
                    .map(|expr| self.eval(expr, env))
                    .transpose()?
                    .unwrap_or(Value::Unit),
            )),
            Stmt::If {
                condition,
                then_block,
                else_block,
                span,
            } => {
                let condition_value = self.eval(condition, env)?;
                if self.expect_bool(condition_value, *span)? {
                    self.exec_block(then_block, env)
                } else if let Some(block) = else_block {
                    self.exec_block(block, env)
                } else {
                    Ok(Flow::Continue)
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                span,
            } => {
                loop {
                    self.consume_step(*span)?;
                    self.check_loop_invariants(invariants, env, *span, "before")?;
                    let condition_value = self.eval(condition, env)?;
                    if !self.expect_bool(condition_value, *span)? {
                        break;
                    }
                    let flow = self.exec_block(body, env)?;
                    self.check_loop_invariants(invariants, env, *span, "after")?;
                    match flow {
                        Flow::Continue => {}
                        Flow::LoopContinue => continue,
                        Flow::Break => break,
                        flow @ Flow::Return(_) => return Ok(flow),
                    }
                }
                Ok(Flow::Continue)
            }
            Stmt::For {
                name,
                additional_names,
                iterable,
                body,
                span,
            } => {
                let values: Box<dyn Iterator<Item = Value>> = match &iterable.kind {
                    ExprKind::Call {
                        name: call,
                        type_args,
                        args,
                    } if call == "range" => {
                        if !type_args.is_empty() || !(2..=3).contains(&args.len()) {
                            return Err(self.runtime_error(
                                *span,
                                "range expects two Int bounds and an optional Int step",
                            ));
                        }
                        let start = self.eval(&args[0], env)?;
                        let end = self.eval(&args[1], env)?;
                        let (Value::Int(start), Value::Int(end)) = (start, end) else {
                            return Err(self.runtime_error(
                                *span,
                                "range expects two Int bounds and an optional Int step",
                            ));
                        };
                        let step = match args.get(2) {
                            Some(expression) => match self.eval(expression, env)? {
                                Value::Int(step) => step,
                                _ => {
                                    return Err(self.runtime_error(
                                        expression.span,
                                        "range step must be an Int",
                                    ));
                                }
                            },
                            None => 1,
                        };
                        if step == 0 {
                            return Err(self.runtime_error(
                                args.get(2).map_or(*span, |step| step.span),
                                "range step must not be zero",
                            ));
                        }
                        Box::new(integer_range(start, end, step).map(Value::Int))
                    }
                    _ => {
                        let iterable_value = self.eval(iterable, env)?;
                        match iterable_value {
                            Value::Array(values) | Value::Set(values) => {
                                Box::new(values.into_iter())
                            }
                            value => {
                                return Err(self.runtime_error(
                                    *span,
                                    format!(
                                        "`for ... in` expects an array or Set, found {}",
                                        value.ty()
                                    ),
                                ));
                            }
                        }
                    }
                };
                for value in values {
                    self.consume_step(*span)?;
                    let flow = env.with_scope(|env| {
                        if additional_names.is_empty() {
                            env.declare(name.clone(), value, false);
                        } else {
                            let Value::Tuple(values) = value else {
                                return Err(self.runtime_error(
                                    *span,
                                    "tuple loop pattern received a non-tuple value",
                                ));
                            };
                            if values.len() != additional_names.len() + 1 {
                                return Err(self.runtime_error(
                                    *span,
                                    format!(
                                        "tuple loop pattern expects {} values, found {}",
                                        additional_names.len() + 1,
                                        values.len()
                                    ),
                                ));
                            }
                            let mut values = values.into_iter();
                            env.declare(
                                name.clone(),
                                values.next().expect("tuple arity checked"),
                                false,
                            );
                            for name in additional_names {
                                env.declare(
                                    name.clone(),
                                    values.next().expect("tuple arity checked"),
                                    false,
                                );
                            }
                        }
                        self.exec_block(body, env)
                    })?;
                    match flow {
                        Flow::Continue | Flow::LoopContinue => {}
                        Flow::Break => break,
                        flow @ Flow::Return(_) => return Ok(flow),
                    }
                }
                Ok(Flow::Continue)
            }
            Stmt::Loop {
                invariants,
                body,
                span,
            } => {
                loop {
                    self.consume_step(*span)?;
                    self.check_loop_invariants(invariants, env, *span, "before")?;
                    let flow = self.exec_block(body, env)?;
                    self.check_loop_invariants(invariants, env, *span, "after")?;
                    match flow {
                        Flow::Continue => {}
                        Flow::LoopContinue => continue,
                        Flow::Break => break,
                        flow @ Flow::Return(_) => return Ok(flow),
                    }
                }
                Ok(Flow::Continue)
            }
            Stmt::Break { .. } => Ok(Flow::Break),
            Stmt::Continue { .. } => Ok(Flow::LoopContinue),
            Stmt::Match { value, arms, span } => {
                let scrutinee = self.eval(value, env)?;
                for arm in arms {
                    if let Some(bindings) = match_pattern(&scrutinee, &arm.pattern) {
                        let arm_result = env.with_scope(|env| {
                            for (name, value) in bindings {
                                env.declare(name, value, false);
                            }
                            if let Some(guard) = &arm.guard {
                                let guard_value = self.eval(guard, env)?;
                                if !self.expect_bool(guard_value, guard.span)? {
                                    return Ok(None);
                                }
                            }
                            self.exec_block(&arm.body, env).map(Some)
                        })?;
                        if let Some(flow) = arm_result {
                            return Ok(flow);
                        }
                    }
                }
                Err(self.runtime_error(*span, "non-exhaustive match at runtime"))
            }
            Stmt::Transaction { body, span } => {
                if self.transactional_query.is_some() {
                    // A transaction block inside a caller-owned transaction is
                    // a logical batch, not a second transaction. The outer
                    // request owns commit/rollback and every queued query uses
                    // its borrowed executor.
                    let queries = transaction_queries(self, body, env, *span)?;
                    self.ensure_database_storage(*span)?;
                    for query in queries {
                        let result = match self.transactional_query.as_deref_mut() {
                            Some(query_executor) => query_executor(&query),
                            None => {
                                return Err(RuntimeError {
                                    message: "borrowed database transaction is unavailable".into(),
                                    span: *span,
                                });
                            }
                        };
                        if let Err(error) = result {
                            return Err(RuntimeError {
                                message: format!("database transaction failed: {error}"),
                                span: *span,
                            });
                        }
                    }
                    return Ok(Flow::Continue);
                }
                let Some(database_url) = self.database_url.clone() else {
                    return Err(self.runtime_error(
                        *span,
                        "transaction execution requires a database runtime",
                    ));
                };
                let queries = transaction_queries(self, body, env, *span)?;
                self.ensure_database_storage(*span)?;
                if let Some(transaction) = &self.database_transaction {
                    transaction
                        .lock()
                        .map_err(|_| {
                            self.runtime_error(
                                *span,
                                "interactive database transaction is unavailable",
                            )
                        })?
                        .execute_batch(&queries)
                        .map_err(|error| {
                            self.runtime_error(
                                *span,
                                format!("database transaction failed: {error}"),
                            )
                        })?;
                } else {
                    self.host_services
                        .execute_database_batch(&database_url, &queries, true)
                        .map_err(|error| {
                            self.runtime_error(
                                *span,
                                format!("database transaction failed: {error}"),
                            )
                        })?;
                }
                Ok(Flow::Continue)
            }
            Stmt::Parallel { body, span } => {
                if self.transactional_query.is_some() {
                    return Err(self.runtime_error(
                        *span,
                        "parallel execution is unavailable inside a borrowed database transaction",
                    ));
                }
                self.exec_parallel(body, *span, env)
            }
        }
    }
    pub(crate) fn expect_bool(&self, value: Value, span: Span) -> Result<bool, RuntimeError> {
        match value {
            Value::Bool(v) => Ok(v),
            other => Err(self.runtime_error(span, format!("expected Bool, found {}", other.ty()))),
        }
    }
}
