use super::*;

impl<'program> Interpreter<'program> {
    pub(crate) fn eval(
        &mut self,
        expr: &Expr,
        env: &mut Environment,
    ) -> Result<Value, RuntimeError> {
        self.consume_step(expr.span)?;
        match &expr.kind {
            ExprKind::Int(v) => Ok(Value::Int(*v)),
            ExprKind::UInt(v) => Ok(Value::UInt(*v)),
            ExprKind::Float(v) => Ok(Value::Float(*v)),
            ExprKind::Bool(v) => Ok(Value::Bool(*v)),
            ExprKind::String(v) => Ok(Value::String(v.clone())),
            ExprKind::Char(v) => Ok(Value::Char(*v)),
            ExprKind::FunctionValue(name) => {
                let function = self.functions.get(name).copied().ok_or_else(|| {
                    self.runtime_error(expr.span, format!("unknown function value `{name}`"))
                })?;
                if !function.type_params.is_empty() {
                    return Err(self.runtime_error(
                        expr.span,
                        format!(
                            "generic function `{name}` cannot be stored as a function value yet"
                        ),
                    ));
                }
                Ok(Value::Function(Box::new(FunctionValue {
                    name: name.clone(),
                    params: function
                        .params
                        .iter()
                        .map(|param| param.ty.clone())
                        .collect(),
                    return_type: function.return_type.clone().unwrap_or(Type::Unit),
                    capabilities: function.capabilities.clone(),
                    lambda_params: Vec::new(),
                    lambda_body: None,
                    captures: Vec::new(),
                    bytecode_target: None,
                    bytecode_captures: Vec::new(),
                })))
            }
            ExprKind::Lambda { params, body } => Ok(Value::Function(Box::new(FunctionValue {
                name: "<lambda>".into(),
                params: params
                    .iter()
                    .map(|parameter| parameter.ty.clone())
                    .collect(),
                return_type: Type::Unknown,
                capabilities: Vec::new(),
                lambda_params: params
                    .iter()
                    .map(|parameter| parameter.name.clone())
                    .collect(),
                lambda_body: Some(body.clone()),
                captures: env.visible_values(),
                bytecode_target: None,
                bytecode_captures: Vec::new(),
            }))),
            ExprKind::Array(values) => values
                .iter()
                .map(|value| self.eval(value, env))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            ExprKind::Map(entries) => {
                let mut values = Vec::with_capacity(entries.len());
                for (key, value) in entries {
                    let key = self.eval(key, env)?;
                    let value = self.eval(value, env)?;
                    values.push((key, value));
                }
                Ok(Value::Map(normalize_map_entries(values)))
            }
            ExprKind::Record {
                type_name,
                type_args,
                fields,
            } => self.eval_record_value(type_name, type_args, fields, expr.span, env),
            ExprKind::Variable(name) => {
                if let Some(value) = env.get(name) {
                    return Ok(value);
                }
                if name == "None" {
                    return Ok(Value::Option(None));
                }
                if let Some((type_name, variant)) = name.rsplit_once("::") {
                    if let Some(definition) = self.enum_definitions.get(type_name) {
                        let payload_types = definition
                            .variant_payloads
                            .iter()
                            .find(|(candidate, _)| candidate == variant)
                            .map(|(_, types)| types.as_slice())
                            .ok_or_else(|| {
                                self.runtime_error(
                                    expr.span,
                                    format!("unknown enum variant `{type_name}::{variant}`"),
                                )
                            })?;
                        if !payload_types.is_empty() {
                            return Err(self.runtime_error(
                                expr.span,
                                format!(
                                    "enum variant `{type_name}::{variant}` requires payload values"
                                ),
                            ));
                        }
                        return Ok(Value::Enum {
                            type_name: type_name.to_owned(),
                            type_args: Vec::new(),
                            variant: variant.to_owned(),
                            payloads: Vec::new(),
                        });
                    }
                }
                Err(self.runtime_error(expr.span, format!("unknown variable `{name}`")))
            }
            ExprKind::Call {
                name,
                type_args,
                args,
            } => self.eval_call_expr(name, type_args, args, env, expr.span),
            ExprKind::Unary { op, expr: inner } => {
                let value = self.eval(inner, env)?;
                evaluate_unary_value(*op, value, expr.span)
            }
            ExprKind::Index { target, index } => {
                let target = self.eval(target, env)?;
                let index = self.eval(index, env)?;
                let index = match index {
                    Value::Int(value) if value >= 0 => value as usize,
                    Value::UInt(value) => value as usize,
                    value => {
                        return Err(self.runtime_error(
                            expr.span,
                            format!("array index must be Int or UInt, found {}", value.ty()),
                        ));
                    }
                };
                match target {
                    Value::Array(values) => values.get(index).cloned().ok_or_else(|| {
                        self.runtime_error(
                            expr.span,
                            format!("array index {index} is out of bounds"),
                        )
                    }),
                    Value::Tuple(values) => values.get(index).cloned().ok_or_else(|| {
                        self.runtime_error(
                            expr.span,
                            format!("tuple index {index} is out of bounds"),
                        )
                    }),
                    value => Err(self.runtime_error(
                        expr.span,
                        format!("indexing requires an array or Tuple, found {}", value.ty()),
                    )),
                }
            }
            ExprKind::Field { target, field } => {
                let value = self.eval(target, env)?;
                match value {
                    Value::Object { fields, .. } => fields.get(field).cloned().ok_or_else(|| {
                        self.runtime_error(expr.span, format!("record field `{field}` is missing"))
                    }),
                    Value::ClassRef { reference, .. } => {
                        let object = reference.object.lock().map_err(|_| {
                            self.runtime_error(expr.span, "class object is unavailable")
                        })?;
                        object.fields.get(field).cloned().ok_or_else(|| {
                            self.runtime_error(
                                expr.span,
                                format!("class field `{field}` is missing"),
                            )
                        })
                    }
                    value => Err(self.runtime_error(
                        expr.span,
                        format!(
                            "field access requires a record or class, found {}",
                            value.ty()
                        ),
                    )),
                }
            }
            ExprKind::Binary { .. } => {
                let mut cursor = expr;
                let mut operations = Vec::new();
                while let ExprKind::Binary { left, op, right } = &cursor.kind {
                    operations.push((*op, right.as_ref(), cursor.span));
                    cursor = left;
                }
                let mut value = self.eval(cursor, env)?;
                for (op, right, span) in operations.into_iter().rev() {
                    if op == BinaryOp::And && matches!(&value, Value::Bool(false)) {
                        value = Value::Bool(false);
                        continue;
                    }
                    if op == BinaryOp::Or && matches!(&value, Value::Bool(true)) {
                        value = Value::Bool(true);
                        continue;
                    }
                    let right = self.eval(right, env)?;
                    value = self.binary(value, op, right, span)?;
                }
                Ok(value)
            }
            ExprKind::Sql { query, result_type } => {
                check_scoped_sql(query, &self.scoped_tables)
                    .map_err(|error| self.runtime_error(expr.span, error))?;
                let capability = sql_required_capability(query);
                self.require_runtime_capability(capability, "SQL access", expr.span)?;
                let Some(database_url) = self.database_url.clone() else {
                    return Err(
                        self.runtime_error(expr.span, "SQL execution requires a database runtime")
                    );
                };
                let params = query_parameters(query, env, expr.span)?;
                self.ensure_database_storage(expr.span)?;
                let database_query = zelyra_database::Query {
                    sql: query.clone(),
                    params,
                };
                let result = execute_database_query_with_host(
                    &database_url,
                    &database_query,
                    self.host_services.as_ref(),
                    expr.span,
                )?;
                Ok(Value::Rows {
                    columns: result.columns,
                    rows: result.rows,
                    metadata: Box::new(SqlRowsMetadata {
                        nulls: result.nulls,
                        column_types: result.column_types,
                    }),
                    result_type: Box::new(result_type.clone()),
                })
            }
            ExprKind::Await(inner) => self.eval(inner, env),
            ExprKind::Match { value, arms } => {
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
                            self.eval(&arm.value, env).map(Some)
                        })?;
                        if let Some(value) = arm_result {
                            return Ok(value);
                        }
                    }
                }
                Err(self.runtime_error(expr.span, "non-exhaustive match at runtime"))
            }
        }
    }
    pub(crate) fn binary(
        &self,
        left: Value,
        op: BinaryOp,
        right: Value,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        evaluate_binary_value(left, op, right, span)
    }
}
