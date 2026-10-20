use super::*;

impl<'program, 'services> Interpreter<'program, 'services> {
    pub(crate) fn eval_builtin_call(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if let Some(result) = self.eval_path_builtin(name, args, env, span) {
            return result;
        }
        if let Some(result) = self.eval_host_builtin(name, args, env, span) {
            return result;
        }
        if let Some(result) = self.eval_filesystem_metadata_builtin(name, args, env, span) {
            return result;
        }
        if let Some(result) = self.eval_filesystem_io_builtin(name, args, env, span) {
            return result;
        }
        if name == "module_setting" {
            if type_args.len() != 1 || args.len() != 1 {
                return Err(self.runtime_error(
                    span,
                    "module_setting expects one type argument and one setting name",
                ));
            }
            let setting_name = match self.eval(&args[0], env)? {
                Value::String(name) => name,
                value => {
                    return Err(self.runtime_error(
                        args[0].span,
                        format!("module_setting expects a String name, found {}", value.ty()),
                    ));
                }
            };
            let settings = self
                .package_settings
                .as_ref()
                .ok_or_else(|| self.runtime_error(span, "package settings are unavailable"))?;
            return settings
                .get_value(span.source_id, &setting_name, &type_args[0])
                .map_err(|error| self.runtime_error(span, error));
        }
        if type_args.is_empty() && is_pure_builtin(name) {
            let values = args
                .iter()
                .map(|argument| self.eval(argument, env))
                .collect::<Result<Vec<_>, _>>()?;
            return execute_pure_builtin(name, values, span);
        }
        if matches!(
            name,
            "map"
                | "filter"
                | "find"
                | "reduce"
                | "tuple"
                | "set"
                | "set_add"
                | "set_remove"
                | "decimal"
                | "date"
                | "time"
                | "duration_ms"
                | "bytes"
        ) {
            return self.eval_language_value_builtin(name, args, env, span);
        }
        if name == "render_pdf" {
            if args.len() != 2 || !type_args.is_empty() {
                return Err(self
                    .runtime_error(span, "render_pdf expects a named View and one record value"));
            }
            let view_name = match self.eval(&args[0], env)? {
                Value::String(name) => name,
                value => {
                    return Err(self.runtime_error(
                        args[0].span,
                        format!("render_pdf expects a View name, found {}", value.ty()),
                    ));
                }
            };
            let view = self
                .view_definitions
                .get(&view_name)
                .cloned()
                .ok_or_else(|| {
                    self.runtime_error(args[0].span, format!("unknown PDF view `{view_name}`"))
                })?;
            let data = self.eval(&args[1], env)?;
            render_pdf_view(&view, &data)
                .map(Value::Bytes)
                .map_err(|error| self.runtime_error(span, error))
        } else if name == "print" {
            let value = self.eval(
                args.first()
                    .ok_or_else(|| self.runtime_error(span, "`print` expects one argument"))?,
                env,
            )?;
            self.output.push(value.output());
            Ok(Value::Unit)
        } else if name == "assert" {
            if !(1..=2).contains(&args.len()) {
                return Err(self.runtime_error(
                    span,
                    "`assert` expects a Bool condition and an optional String message",
                ));
            }
            let condition = self.eval(&args[0], env)?;
            let Value::Bool(condition) = condition else {
                return Err(self.runtime_error(args[0].span, "`assert` expects a Bool condition"));
            };
            let message = if let Some(message_expr) = args.get(1) {
                let value = self.eval(message_expr, env)?;
                let Value::String(message) = value else {
                    return Err(
                        self.runtime_error(message_expr.span, "`assert` message must be a String")
                    );
                };
                Some(message)
            } else {
                None
            };
            if condition {
                Ok(Value::Unit)
            } else {
                Err(self.runtime_error(span, message.unwrap_or_else(|| "assertion failed".into())))
            }
        } else if name == "object_set_field" {
            if self.parallel_worker {
                return Err(self.runtime_error(
                    span,
                    "class field mutation is not allowed inside a `parallel` block",
                ));
            }
            if !type_args.is_empty() || args.len() != 3 {
                return Err(self.runtime_error(
                    span,
                    "class field update expects an object, literal field name, and value",
                ));
            }
            let object_value = self.eval(&args[0], env)?;
            let field_name = self.eval(&args[1], env)?;
            let value = self.eval(&args[2], env)?;
            let (reference, type_name, type_args, field_name) = match (object_value, field_name) {
                (
                    Value::ClassRef {
                        reference,
                        type_name,
                        type_args,
                    },
                    Value::String(field_name),
                ) => (reference, type_name, type_args, field_name),
                _ => {
                    return Err(self.runtime_error(
                        span,
                        "field assignment requires a class instance and literal field name",
                    ))
                }
            };
            let definition = self.record_definitions.get(&type_name).ok_or_else(|| {
                self.runtime_error(span, format!("unknown class type `{type_name}`"))
            })?;
            if !definition.is_class {
                return Err(self.runtime_error(span, "field assignment requires a class instance"));
            }
            let field = definition
                .fields
                .iter()
                .find(|field| field.name == field_name)
                .ok_or_else(|| {
                    self.runtime_error(
                        args[1].span,
                        format!("unknown field `{field_name}` in class `{type_name}`"),
                    )
                })?;
            let substitutions = definition
                .type_params
                .iter()
                .zip(&type_args)
                .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                .collect::<HashMap<_, _>>();
            let expected = substitute_generic_type(&field.ty, &substitutions);
            if !compatible(
                &self.runtime_alias_type(&expected, span)?,
                &self.runtime_alias_type(&value.ty(), span)?,
            ) {
                return Err(self.runtime_error(
                    args[2].span,
                    format!(
                        "class field `{field_name}` expects `{expected}`, found `{}`",
                        value.ty()
                    ),
                ));
            }
            if class_reference_would_cycle(&value, &reference)
                .map_err(|message| self.runtime_error(span, message))?
            {
                return Err(self.runtime_error(
                    args[2].span,
                    "class field update would create a reference cycle",
                ));
            }
            let mut object = reference
                .object
                .lock()
                .map_err(|_| self.runtime_error(span, "class object is unavailable"))?;
            object.fields.insert(field_name, value);
            Ok(Value::Unit)
        } else if name == "record_update" {
            if type_args.len() != 1 || args.is_empty() || args.len().is_multiple_of(2) {
                return Err(self.runtime_error(
                    span,
                    "record update expects a base record followed by field/value pairs",
                ));
            }
            let expected = self.runtime_alias_type(&type_args[0], span)?;
            let base = self.eval(&args[0], env)?;
            let (type_name, type_arguments, mut fields) = match base {
                Value::Object {
                    type_name,
                    type_args,
                    fields,
                } => (type_name, type_args, fields),
                value => {
                    return Err(self.runtime_error(
                        args[0].span,
                        format!("record update expects a record, found `{}`", value.ty()),
                    ))
                }
            };
            let actual = if type_arguments.is_empty() {
                Type::Named(type_name.clone())
            } else {
                Type::Applied {
                    name: type_name.clone(),
                    args: type_arguments.clone(),
                }
            };
            if !compatible(&expected, &actual) {
                return Err(self.runtime_error(
                    args[0].span,
                    format!("record update expects `{expected}`, found `{actual}`"),
                ));
            }
            let definition = self
                .record_definitions
                .get(&type_name)
                .cloned()
                .ok_or_else(|| {
                    self.runtime_error(span, format!("unknown record type `{type_name}`"))
                })?;
            if definition.is_class {
                return Err(self.runtime_error(
                    args[0].span,
                    "record update cannot copy or update a class reference",
                ));
            }
            let substitutions = definition
                .type_params
                .iter()
                .zip(&type_arguments)
                .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                .collect::<HashMap<_, _>>();
            for pair in args[1..].chunks(2) {
                let Value::String(field_name) = self.eval(&pair[0], env)? else {
                    return Err(self.runtime_error(
                        pair[0].span,
                        "record update field name must be a String literal",
                    ));
                };
                let field = definition
                    .fields
                    .iter()
                    .find(|field| field.name == field_name)
                    .ok_or_else(|| {
                        self.runtime_error(
                            pair[0].span,
                            format!("unknown field `{field_name}` in record `{type_name}`"),
                        )
                    })?;
                let value = self.eval(&pair[1], env)?;
                let field_type = substitute_generic_type(&field.ty, &substitutions);
                if !compatible(
                    &self.runtime_alias_type(&field_type, pair[1].span)?,
                    &self.runtime_alias_type(&value.ty(), pair[1].span)?,
                ) {
                    return Err(self.runtime_error(
                        pair[1].span,
                        format!(
                            "record field `{field_name}` expects `{field_type}`, found `{}`",
                            value.ty()
                        ),
                    ));
                }
                fields.insert(field_name, value);
            }
            Ok(Value::Object {
                type_name,
                type_args: type_arguments,
                fields,
            })
        } else if name == "record_with" {
            if !type_args.is_empty() || args.len() != 3 {
                return Err(self.runtime_error(
                    span,
                    "record_with expects a record, literal field name, and value",
                ));
            }
            let record = self.eval(&args[0], env)?;
            let field_name = self.eval(&args[1], env)?;
            let value = self.eval(&args[2], env)?;
            let (
                Value::Object {
                    type_name,
                    type_args,
                    mut fields,
                },
                Value::String(field_name),
            ) = (record, field_name)
            else {
                return Err(
                    self.runtime_error(span, "record_with expects a record and String field name")
                );
            };
            let definition = self.record_definitions.get(&type_name).ok_or_else(|| {
                self.runtime_error(span, format!("unknown record type `{type_name}`"))
            })?;
            let field = definition
                .fields
                .iter()
                .find(|field| field.name == field_name)
                .ok_or_else(|| {
                    self.runtime_error(
                        args[1].span,
                        format!("unknown field `{field_name}` in record `{type_name}`"),
                    )
                })?;
            let substitutions = definition
                .type_params
                .iter()
                .zip(&type_args)
                .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                .collect::<HashMap<_, _>>();
            let field_type = substitute_generic_type(&field.ty, &substitutions);
            if !compatible(
                &self.runtime_alias_type(&field_type, span)?,
                &self.runtime_alias_type(&value.ty(), span)?,
            ) {
                return Err(self.runtime_error(
                    args[2].span,
                    format!(
                        "record field `{field_name}` expects `{}`, found `{}`",
                        field_type,
                        value.ty()
                    ),
                ));
            }
            fields.insert(field_name, value);
            Ok(Value::Object {
                type_name,
                type_args,
                fields,
            })
        } else if name == "range" {
            if !type_args.is_empty() || !(2..=3).contains(&args.len()) {
                return Err(self.runtime_error(
                    span,
                    "range expects two Int bounds and an optional Int step",
                ));
            }
            let start = self.eval(&args[0], env)?;
            let end = self.eval(&args[1], env)?;
            let (Value::Int(start), Value::Int(end)) = (start, end) else {
                return Err(self.runtime_error(
                    span,
                    "range expects two Int bounds and an optional Int step",
                ));
            };
            let step = match args.get(2) {
                Some(expression) => match self.eval(expression, env)? {
                    Value::Int(step) => step,
                    _ => {
                        return Err(self.runtime_error(expression.span, "range step must be an Int"))
                    }
                },
                None => 1,
            };
            if step == 0 {
                return Err(self.runtime_error(
                    args.get(2).map_or(span, |step| step.span),
                    "range step must not be zero",
                ));
            }
            let mut values = Vec::new();
            for value in integer_range(start, end, step) {
                if values.len() == 1_000_000 {
                    return Err(
                        self.runtime_error(span, "range array exceeds the 1,000,000 element limit")
                    );
                }
                values.push(Value::Int(value));
            }
            Ok(Value::Array(values))
        } else if name == "len" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "`len` expects exactly one argument"));
            }
            match self.eval(&args[0], env)? {
                Value::Array(values) | Value::Tuple(values) | Value::Set(values) => {
                    Ok(Value::Int(values.len() as i64))
                }
                Value::Map(entries) => Ok(Value::Int(entries.len() as i64)),
                Value::String(s) => Ok(Value::Int(s.chars().count() as i64)),
                Value::Bytes(b) => Ok(Value::Int(b.len() as i64)),
                value => Err(self.runtime_error(
                    span,
                    format!(
                        "`len` expects an array, Tuple, Set, Map, String or Bytes, found {}",
                        value.ty()
                    ),
                )),
            }
        } else if name == "append" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "`append` expects an array and one value"));
            }
            let array = self.eval(&args[0], env)?;
            let value = self.eval(&args[1], env)?;
            match array {
                Value::Array(mut values) => {
                    values.push(value);
                    Ok(Value::Array(values))
                }
                value => Err(self.runtime_error(
                    span,
                    format!("`append` expects an array, found {}", value.ty()),
                )),
            }
        } else if name == "contains" {
            if args.len() != 2 {
                return Err(self.runtime_error(
                    span,
                    "`contains` expects an array, Set or Map and one value",
                ));
            }
            let collection = self.eval(&args[0], env)?;
            let value = self.eval(&args[1], env)?;
            match collection {
                Value::Array(values) => Ok(Value::Bool(values.iter().any(|item| item == &value))),
                Value::Set(values) => Ok(Value::Bool(values.contains(&value))),
                Value::Map(entries) => {
                    Ok(Value::Bool(entries.iter().any(|(key, _)| key == &value)))
                }
                value => Err(self.runtime_error(
                    span,
                    format!(
                        "`contains` expects an array, Set or Map, found {}",
                        value.ty()
                    ),
                )),
            }
        } else if name == "abs" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "abs expects exactly one numeric argument"));
            }
            match self.eval(&args[0], env)? {
                Value::Int(val) => val
                    .checked_abs()
                    .map(Value::Int)
                    .ok_or_else(|| self.runtime_error(args[0].span, "Int overflow in abs")),
                Value::Float(val) if val.is_finite() => Ok(Value::Float(val.abs())),
                Value::Float(_) => {
                    Err(self.runtime_error(args[0].span, "abs requires a finite Float"))
                }
                Value::Decimal(val) => val
                    .checked_abs()
                    .map(Value::Decimal)
                    .ok_or_else(|| self.runtime_error(args[0].span, "Decimal overflow in abs")),
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("abs expects numeric value, found {}", other.ty()),
                )),
            }
        } else if name == "min" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "min expects exactly two arguments"));
            }
            let a = self.eval(&args[0], env)?;
            let b = self.eval(&args[1], env)?;
            match (&a, &b) {
                (Value::Int(va), Value::Int(vb)) => Ok(Value::Int(*va.min(vb))),
                (Value::UInt(va), Value::UInt(vb)) => Ok(Value::UInt(*va.min(vb))),
                (Value::Float(va), Value::Float(vb)) if va.is_finite() && vb.is_finite() => {
                    Ok(Value::Float(va.min(*vb)))
                }
                (Value::Float(_), Value::Float(_)) => {
                    Err(self.runtime_error(span, "min requires finite Float arguments"))
                }
                (Value::Decimal(va), Value::Decimal(vb)) => Ok(Value::Decimal(*va.min(vb))),
                (Value::String(va), Value::String(vb)) => {
                    if va <= vb {
                        Ok(a)
                    } else {
                        Ok(b)
                    }
                }
                _ => {
                    Err(self
                        .runtime_error(span, "min expects matching numeric or String arguments"))
                }
            }
        } else if name == "max" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "max expects exactly two arguments"));
            }
            let a = self.eval(&args[0], env)?;
            let b = self.eval(&args[1], env)?;
            match (&a, &b) {
                (Value::Int(va), Value::Int(vb)) => Ok(Value::Int(*va.max(vb))),
                (Value::UInt(va), Value::UInt(vb)) => Ok(Value::UInt(*va.max(vb))),
                (Value::Float(va), Value::Float(vb)) if va.is_finite() && vb.is_finite() => {
                    Ok(Value::Float(va.max(*vb)))
                }
                (Value::Float(_), Value::Float(_)) => {
                    Err(self.runtime_error(span, "max requires finite Float arguments"))
                }
                (Value::Decimal(va), Value::Decimal(vb)) => Ok(Value::Decimal(*va.max(vb))),
                (Value::String(va), Value::String(vb)) => {
                    if va >= vb {
                        Ok(a)
                    } else {
                        Ok(b)
                    }
                }
                _ => {
                    Err(self
                        .runtime_error(span, "max expects matching numeric or String arguments"))
                }
            }
        } else if name == "clamp" {
            if args.len() != 3 {
                return Err(self.runtime_error(span, "clamp expects val, min, and max"));
            }
            let val = self.eval(&args[0], env)?;
            let min_val = self.eval(&args[1], env)?;
            let max_val = self.eval(&args[2], env)?;
            match (&val, &min_val, &max_val) {
                (Value::Int(v), Value::Int(mn), Value::Int(mx)) => {
                    if mn > mx {
                        return Err(
                            self.runtime_error(span, "clamp minimum must not exceed maximum")
                        );
                    }
                    Ok(Value::Int(*v.max(mn).min(mx)))
                }
                (Value::UInt(v), Value::UInt(mn), Value::UInt(mx)) => {
                    if mn > mx {
                        return Err(
                            self.runtime_error(span, "clamp minimum must not exceed maximum")
                        );
                    }
                    Ok(Value::UInt(*v.max(mn).min(mx)))
                }
                (Value::Float(v), Value::Float(mn), Value::Float(mx))
                    if v.is_finite() && mn.is_finite() && mx.is_finite() =>
                {
                    if mn > mx {
                        return Err(
                            self.runtime_error(span, "clamp minimum must not exceed maximum")
                        );
                    }
                    Ok(Value::Float(v.max(*mn).min(*mx)))
                }
                (Value::Float(_), Value::Float(_), Value::Float(_)) => {
                    Err(self.runtime_error(span, "clamp requires finite Float arguments"))
                }
                (Value::Decimal(v), Value::Decimal(mn), Value::Decimal(mx)) => {
                    if mn > mx {
                        return Err(
                            self.runtime_error(span, "clamp minimum must not exceed maximum")
                        );
                    }
                    Ok(Value::Decimal(*v.max(mn).min(mx)))
                }
                _ => Err(self.runtime_error(span, "clamp expects matching numeric arguments")),
            }
        } else if name == "to_string" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "to_string expects exactly one argument"));
            }
            let val = self.eval(&args[0], env)?;
            Ok(Value::String(val.output()))
        } else if name == "sort" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "sort expects exactly one array"));
            }
            match self.eval(&args[0], env)? {
                Value::Array(mut items) => {
                    items.sort_by(|a, b| match (a, b) {
                        (Value::Int(x), Value::Int(y)) => x.cmp(y),
                        (Value::UInt(x), Value::UInt(y)) => x.cmp(y),
                        (Value::Float(x), Value::Float(y)) => {
                            x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        (Value::Decimal(x), Value::Decimal(y)) => x.cmp(y),
                        (Value::String(x), Value::String(y)) => x.cmp(y),
                        _ => std::cmp::Ordering::Equal,
                    });
                    Ok(Value::Array(items))
                }
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("sort expects array, found {}", other.ty()),
                )),
            }
        } else if name == "sort_desc" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "sort_desc expects exactly one array"));
            }
            match self.eval(&args[0], env)? {
                Value::Array(mut items) => {
                    items.sort_by(|a, b| match (a, b) {
                        (Value::Int(x), Value::Int(y)) => y.cmp(x),
                        (Value::UInt(x), Value::UInt(y)) => y.cmp(x),
                        (Value::Float(x), Value::Float(y)) => {
                            y.partial_cmp(x).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        (Value::Decimal(x), Value::Decimal(y)) => y.cmp(x),
                        (Value::String(x), Value::String(y)) => y.cmp(x),
                        _ => std::cmp::Ordering::Equal,
                    });
                    Ok(Value::Array(items))
                }
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("sort_desc expects array, found {}", other.ty()),
                )),
            }
        } else if name == "reverse" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "reverse expects exactly one array"));
            }
            match self.eval(&args[0], env)? {
                Value::Array(mut items) => {
                    items.reverse();
                    Ok(Value::Array(items))
                }
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("reverse expects array, found {}", other.ty()),
                )),
            }
        } else if name == "slice" {
            if args.len() != 3 {
                return Err(
                    self.runtime_error(span, "slice expects array, start index, and length")
                );
            }
            let arr = self.eval(&args[0], env)?;
            let start = self.eval(&args[1], env)?;
            let len_val = self.eval(&args[2], env)?;
            match (arr, start, len_val) {
                (Value::Array(items), Value::Int(start_idx), Value::Int(length)) => {
                    if start_idx < 0 || length < 0 {
                        return Err(self.runtime_error(span, "slice indices must be non-negative"));
                    }
                    let start_usize = start_idx as usize;
                    let len_usize = length as usize;
                    if start_usize > items.len() {
                        return Ok(Value::Array(Vec::new()));
                    }
                    let end_usize = (start_usize + len_usize).min(items.len());
                    Ok(Value::Array(items[start_usize..end_usize].to_vec()))
                }
                _ => {
                    Err(self.runtime_error(span, "slice expects array, Int start, and Int length"))
                }
            }
        } else if name == "concat" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "concat expects two arrays"));
            }
            let a = self.eval(&args[0], env)?;
            let b = self.eval(&args[1], env)?;
            match (a, b) {
                (Value::Array(mut a_items), Value::Array(b_items)) => {
                    a_items.extend(b_items);
                    Ok(Value::Array(a_items))
                }
                _ => Err(self.runtime_error(span, "concat expects two arrays")),
            }
        } else if name == "join" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "join expects array of Strings and delimiter"));
            }
            let arr = self.eval(&args[0], env)?;
            let sep = self.eval(&args[1], env)?;
            match (arr, sep) {
                (Value::Array(items), Value::String(delimiter)) => {
                    let mut pieces = Vec::new();
                    for item in items {
                        pieces.push(item.output());
                    }
                    Ok(Value::String(pieces.join(&delimiter)))
                }
                _ => Err(self.runtime_error(span, "join expects array and String delimiter")),
            }
        } else if name == "sqrt" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "sqrt expects one Float"));
            }
            match self.eval(&args[0], env)? {
                Value::Float(f) => {
                    if !f.is_finite() || f < 0.0 {
                        return Err(
                            self.runtime_error(span, "sqrt requires a finite non-negative Float")
                        );
                    }
                    Ok(Value::Float(f.sqrt()))
                }
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("sqrt expects Float, found {}", other.ty()),
                )),
            }
        } else if name == "pow" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "pow expects base and exponent"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::Float(b), Value::Float(e)) => {
                    let result = b.powf(e);
                    if !b.is_finite() || !e.is_finite() || !result.is_finite() {
                        return Err(self.runtime_error(
                            span,
                            "pow is undefined or outside the finite Float range",
                        ));
                    }
                    Ok(Value::Float(result))
                }
                _ => Err(self.runtime_error(span, "pow expects Float arguments")),
            }
        } else if name == "floor" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "floor expects one Float"));
            }
            match self.eval(&args[0], env)? {
                Value::Float(f) if f.is_finite() => Ok(Value::Float(f.floor())),
                Value::Float(_) => Err(self.runtime_error(span, "floor requires a finite Float")),
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("floor expects Float, found {}", other.ty()),
                )),
            }
        } else if name == "ceil" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "ceil expects one Float"));
            }
            match self.eval(&args[0], env)? {
                Value::Float(f) if f.is_finite() => Ok(Value::Float(f.ceil())),
                Value::Float(_) => Err(self.runtime_error(span, "ceil requires a finite Float")),
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("ceil expects Float, found {}", other.ty()),
                )),
            }
        } else if name == "round" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "round expects number and decimal places"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::Float(f), Value::Int(d)) => {
                    if !f.is_finite() {
                        return Err(
                            self.runtime_error(args[0].span, "round requires a finite Float")
                        );
                    }
                    if !(-15..=15).contains(&d) {
                        return Err(self.runtime_error(
                            args[1].span,
                            "round decimal places must be between -15 and 15",
                        ));
                    }
                    let factor = (10.0_f64).powi(d as i32);
                    let rounded = (f * factor).round() / factor;
                    if !rounded.is_finite() {
                        return Err(self.runtime_error(
                            span,
                            "round result is outside the finite Float range",
                        ));
                    }
                    Ok(Value::Float(rounded))
                }
                _ => Err(self.runtime_error(span, "round expects Float and Int")),
            }
        } else if name == "panic" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "panic expects one String message"));
            }
            let msg = self.eval(&args[0], env)?;
            Err(self.runtime_error(span, format!("panic: {}", msg.output())))
        } else if name == "path_join" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "path_join expects two String arguments"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::String(a), Value::String(b)) => {
                    let p = Path::new(&a).join(Path::new(&b));
                    Ok(Value::String(p.to_string_lossy().to_string()))
                }
                _ => Err(self.runtime_error(span, "path_join expects two Strings")),
            }
        } else if name == "path_filename" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "path_filename expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(s) => {
                    let p = Path::new(&s);
                    Ok(Value::String(
                        p.file_name()
                            .and_then(|f| f.to_str())
                            .unwrap_or("")
                            .to_string(),
                    ))
                }
                _ => Err(self.runtime_error(span, "path_filename expects a String")),
            }
        } else if name == "path_extension" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "path_extension expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(s) => {
                    let p = Path::new(&s);
                    Ok(Value::String(
                        p.extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap_or("")
                            .to_string(),
                    ))
                }
                _ => Err(self.runtime_error(span, "path_extension expects a String")),
            }
        } else if name == "path_dir" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "path_dir expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(s) => {
                    let p = Path::new(&s);
                    Ok(Value::String(
                        p.parent()
                            .and_then(|parent| parent.to_str())
                            .unwrap_or("")
                            .to_string(),
                    ))
                }
                _ => Err(self.runtime_error(span, "path_dir expects a String")),
            }
        } else if name == "unwrap_or" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "unwrap_or expects Option and default"));
            }
            let opt_val = self.eval(&args[0], env)?;
            let default_val = self.eval(&args[1], env)?;
            match opt_val {
                Value::Option(Some(inner)) => Ok(*inner),
                Value::Option(None) => Ok(default_val),
                _ => Err(self.runtime_error(args[0].span, "unwrap_or expects an Option")),
            }
        } else if name == "get" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "`get` expects a Map and one key"));
            }
            let map = self.eval(&args[0], env)?;
            let key = self.eval(&args[1], env)?;
            match map {
                Value::Map(entries) => Ok(Value::Option(
                    entries
                        .iter()
                        .find(|(existing, _)| existing == &key)
                        .map(|(_, value)| Box::new(value.clone())),
                )),
                value => {
                    Err(self
                        .runtime_error(span, format!("`get` expects a Map, found {}", value.ty())))
                }
            }
        } else if name == "put" {
            if args.len() != 3 {
                return Err(self.runtime_error(span, "`put` expects a Map, a key, and a value"));
            }
            let map = self.eval(&args[0], env)?;
            let key = self.eval(&args[1], env)?;
            let value = self.eval(&args[2], env)?;
            match map {
                Value::Map(mut entries) => {
                    if let Some((_, existing_value)) =
                        entries.iter_mut().find(|(existing, _)| existing == &key)
                    {
                        *existing_value = value;
                    } else {
                        entries.push((key, value));
                    }
                    Ok(Value::Map(entries))
                }
                value => {
                    Err(self
                        .runtime_error(span, format!("`put` expects a Map, found {}", value.ty())))
                }
            }
        } else if name == "first" || name == "last" {
            if args.len() != 1 {
                return Err(
                    self.runtime_error(span, format!("`{name}` expects exactly one argument"))
                );
            }
            match self.eval(&args[0], env)? {
                Value::Array(values) => {
                    let value = if name == "first" {
                        values.first()
                    } else {
                        values.last()
                    };
                    Ok(Value::Option(value.cloned().map(Box::new)))
                }
                value => Err(self.runtime_error(
                    span,
                    format!("`{name}` expects an array, found {}", value.ty()),
                )),
            }
        } else if name == "Some" || name == "Ok" || name == "Err" {
            let argument = args.first().ok_or_else(|| {
                self.runtime_error(span, format!("`{name}` expects one argument"))
            })?;
            if args.len() != 1 {
                return Err(
                    self.runtime_error(span, format!("`{name}` expects exactly one argument"))
                );
            }
            let value = Box::new(self.eval(argument, env)?);
            match name {
                "Some" => Ok(Value::Option(Some(value))),
                "Ok" => Ok(Value::Result(Ok(value))),
                _ => Ok(Value::Result(Err(value))),
            }
        } else if matches!(name, "decimal" | "date" | "time" | "duration_ms" | "bytes") {
            self.eval_language_value_builtin(name, args, env, span)
        } else if name == "format_timestamp" {
            if args.len() != 1 {
                return Err(
                    self.runtime_error(span, "format_timestamp expects exactly one Timestamp")
                );
            }
            match self.eval(&args[0], env)? {
                Value::Timestamp(timestamp)
                    if zelyra_ast::temporal::valid_timestamp_millis(timestamp) =>
                {
                    Ok(Value::String(format_timestamp_utc(timestamp)))
                }
                Value::Timestamp(_) => {
                    Err(self.runtime_error(span, "Timestamp is outside years 0001 through 9999"))
                }
                value => Err(self.runtime_error(
                    args[0].span,
                    format!("format_timestamp expects a Timestamp, found {}", value.ty()),
                )),
            }
        } else if name == "format_text" {
            if args.len() != 1 {
                return Err(self.runtime_error(
                    span,
                    "format_text expects one scalar value and no type arguments",
                ));
            }
            let value = self.eval(&args[0], env)?;
            if matches!(
                value,
                Value::Int(_)
                    | Value::UInt(_)
                    | Value::Float(_)
                    | Value::Decimal(_)
                    | Value::Bool(_)
                    | Value::Char(_)
                    | Value::String(_)
                    | Value::Timestamp(_)
                    | Value::Date(_)
                    | Value::Time(_)
                    | Value::Duration(_)
            ) {
                Ok(Value::String(value.output()))
            } else {
                Err(self.runtime_error(
                    args[0].span,
                    format!("format_text does not support `{}`", value.ty()),
                ))
            }
        } else if name == "http_get" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "http_get expects exactly one String URL"));
            }
            self.require_runtime_capability("Network", "network access", span)?;
            let url = self.eval(&args[0], env)?;
            let Value::String(url) = url else {
                return Err(self.runtime_error(args[0].span, "http_get expects a String URL"));
            };
            self.http_get(&url, span).map(Value::String)
        } else if name == "json_encode" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "json_encode expects exactly one value"));
            }
            let value = self.eval(&args[0], env)?;
            self.json_encode(&value, span).map(Value::String)
        } else if name == "json_encode_tagged_result" {
            if !type_args.is_empty() || args.len() != 1 {
                return Err(self.runtime_error(
                    span,
                    "json_encode_tagged_result expects one Result value and no type arguments",
                ));
            }
            let value = self.eval(&args[0], env)?;
            self.json_encode_tagged_result(&value, span)
                .map(Value::String)
        } else if name == "sql_decode" {
            self.eval_sql_decode(type_args, args, env, span)
        } else if name == "json_decode" {
            if type_args.len() != 1 || args.len() != 1 {
                return Err(self.runtime_error(
                    span,
                    "json_decode expects one type argument and one String value",
                ));
            }
            let value = self.eval(&args[0], env)?;
            let Value::String(source) = value else {
                return Err(self.runtime_error(args[0].span, "json_decode expects a String value"));
            };
            self.json_decode(&source, &type_args[0], span)
        } else if name == "json_decode_result" {
            if type_args.len() != 1 || args.len() != 1 {
                return Err(self.runtime_error(
                    span,
                    "json_decode_result expects one type argument and one String value",
                ));
            }
            let value = self.eval(&args[0], env)?;
            let Value::String(source) = value else {
                return Err(
                    self.runtime_error(args[0].span, "json_decode_result expects a String value")
                );
            };
            Ok(Value::Result(
                match self.json_decode(&source, &type_args[0], span) {
                    Ok(value) => Ok(Box::new(value)),
                    Err(error) => Err(Box::new(Value::String(error.message))),
                },
            ))
        } else if name == "json_decode_tagged_result" {
            if type_args.len() != 2 || args.len() != 1 {
                return Err(self.runtime_error(
                    span,
                    "json_decode_tagged_result expects Ok and Err type arguments and one String value",
                ));
            }
            let value = self.eval(&args[0], env)?;
            let Value::String(source) = value else {
                return Err(self.runtime_error(
                    args[0].span,
                    "json_decode_tagged_result expects a String value",
                ));
            };
            let decoded =
                self.json_decode_tagged_result(&source, &type_args[0], &type_args[1], span);
            Ok(Value::Result(
                decoded
                    .map(Box::new)
                    .map_err(|error| Box::new(Value::String(error))),
            ))
        } else if name == "csv_decode_result" {
            if !type_args.is_empty() || args.len() != 2 {
                return Err(self.runtime_error(span, "csv_decode_result expects a String and Char"));
            }
            let source = self.eval(&args[0], env)?;
            let delimiter = self.eval(&args[1], env)?;
            let (Value::String(source), Value::Char(delimiter)) = (source, delimiter) else {
                return Err(self.runtime_error(span, "csv_decode_result expects a String and Char"));
            };
            let decoded = csv::decode(&source, delimiter).map(|rows| {
                Value::Array(
                    rows.into_iter()
                        .map(|row| {
                            Value::Array(row.into_iter().map(Value::String).collect::<Vec<_>>())
                        })
                        .collect::<Vec<_>>(),
                )
            });
            Ok(Value::Result(
                decoded
                    .map(Box::new)
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        } else if name == "csv_encode" {
            if !type_args.is_empty() || args.len() != 2 {
                return Err(
                    self.runtime_error(span, "csv_encode expects String[][] rows and a Char")
                );
            }
            let rows = self.eval(&args[0], env)?;
            let delimiter = self.eval(&args[1], env)?;
            let (Value::Array(rows), Value::Char(delimiter)) = (rows, delimiter) else {
                return Err(
                    self.runtime_error(span, "csv_encode expects String[][] rows and a Char")
                );
            };
            let rows = rows
                .into_iter()
                .map(|row| match row {
                    Value::Array(fields) => fields
                        .into_iter()
                        .map(|field| match field {
                            Value::String(value) => Ok(value),
                            _ => Err("csv_encode expects String[][] rows".to_string()),
                        })
                        .collect::<Result<Vec<_>, _>>(),
                    _ => Err("csv_encode expects String[][] rows".to_string()),
                })
                .collect::<Result<Vec<_>, _>>();
            let encoded = rows
                .and_then(|rows| csv::encode(&rows, delimiter))
                .map(Value::String);
            Ok(Value::Result(
                encoded
                    .map(Box::new)
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        } else if name == "decimal_scaled_result" {
            if !type_args.is_empty() || args.len() != 2 {
                return Err(self
                    .runtime_error(span, "decimal_scaled_result expects a String and scale Int"));
            }
            let source = self.eval(&args[0], env)?;
            let scale = self.eval(&args[1], env)?;
            let (Value::String(source), Value::Int(scale)) = (source, scale) else {
                return Err(self
                    .runtime_error(span, "decimal_scaled_result expects a String and scale Int"));
            };
            Ok(Value::Result(
                csv::decimal_to_scaled(&source, scale)
                    .map(|value| Box::new(Value::Int(value)))
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        } else if name == "format_scaled_result" {
            if !type_args.is_empty() || args.len() != 2 {
                return Err(
                    self.runtime_error(span, "format_scaled_result expects an Int and scale Int")
                );
            }
            let value = self.eval(&args[0], env)?;
            let scale = self.eval(&args[1], env)?;
            let (Value::Int(value), Value::Int(scale)) = (value, scale) else {
                return Err(self.runtime_error(span, "format_scaled_result expects Int arguments"));
            };
            Ok(Value::Result(
                csv::format_scaled(value, scale)
                    .map(|value| Box::new(Value::String(value)))
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        } else if matches!(name, "date_add_days_result" | "date_days_between_result") {
            if !type_args.is_empty() || args.len() != 2 {
                return Err(self.runtime_error(
                    span,
                    format!("{name} expects two arguments and no type arguments"),
                ));
            }
            let left = self.eval(&args[0], env)?;
            let right = self.eval(&args[1], env)?;
            let result = if name == "date_add_days_result" {
                match (left, right) {
                    (Value::Date(date), Value::Int(days)) => date_epoch_days(&date)
                        .and_then(|start| start.checked_add(days))
                        .and_then(epoch_days_date)
                        .map(Value::Date)
                        .ok_or_else(|| {
                            "date arithmetic is outside years 0001 through 9999".to_owned()
                        }),
                    _ => Err("date_add_days_result expects Date and Int".to_owned()),
                }
            } else {
                match (left, right) {
                    (Value::Date(start), Value::Date(end)) => date_epoch_days(&end)
                        .and_then(|end| {
                            date_epoch_days(&start).and_then(|start| end.checked_sub(start))
                        })
                        .map(Value::Int)
                        .ok_or_else(|| "date difference overflow".to_owned()),
                    _ => Err("date_days_between_result expects two Dates".to_owned()),
                }
            };
            Ok(Value::Result(
                result
                    .map(Box::new)
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        } else if matches!(name, "timestamp_add_result" | "timestamp_difference_result") {
            if !type_args.is_empty() || args.len() != 2 {
                return Err(self.runtime_error(
                    span,
                    format!("{name} expects two arguments and no type arguments"),
                ));
            }
            let left = self.eval(&args[0], env)?;
            let right = self.eval(&args[1], env)?;
            let result = if name == "timestamp_add_result" {
                match (left, right) {
                    (Value::Timestamp(timestamp), Value::Duration(duration)) => timestamp
                        .checked_add(duration)
                        .filter(|value| {
                            timestamp_bounds_millis()
                                .is_some_and(|(min, max)| (min..=max).contains(value))
                        })
                        .map(Value::Timestamp)
                        .ok_or_else(|| {
                            "timestamp arithmetic is outside years 0001 through 9999".to_owned()
                        }),
                    _ => Err("timestamp_add_result expects Timestamp and Duration".to_owned()),
                }
            } else {
                match (left, right) {
                    (Value::Timestamp(end), Value::Timestamp(start)) => end
                        .checked_sub(start)
                        .map(Value::Duration)
                        .ok_or_else(|| "timestamp difference overflow".to_owned()),
                    _ => Err("timestamp_difference_result expects two Timestamps".to_owned()),
                }
            };
            Ok(Value::Result(
                result
                    .map(Box::new)
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        } else if name == "decimal_quantize_result" {
            if !type_args.is_empty() || args.len() != 3 {
                return Err(self.runtime_error(
                    span,
                    "decimal_quantize_result expects Decimal, scale Int, and mode String",
                ));
            }
            let value = self.eval(&args[0], env)?;
            let scale = self.eval(&args[1], env)?;
            let mode = self.eval(&args[2], env)?;
            let result = match (value, scale, mode) {
                (Value::Decimal(value), Value::Int(scale), Value::String(mode)) => {
                    u32::try_from(scale)
                        .map_err(|_| "decimal scale must be between 0 and 9".to_owned())
                        .and_then(|scale| quantize_decimal(value, scale, &mode))
                        .map(Value::Decimal)
                }
                _ => Err("decimal_quantize_result expects Decimal, Int, and String".to_owned()),
            };
            Ok(Value::Result(
                result
                    .map(Box::new)
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        } else if name == "http_json" {
            if type_args.len() != 2 || args.len() != 4 {
                return Err(self.runtime_error(
                    span,
                    "http_json expects request and response type arguments plus four values",
                ));
            }
            self.require_runtime_capability("Network", "network access", span)?;
            let method = self.eval(&args[0], env)?;
            let url = self.eval(&args[1], env)?;
            let headers = self.eval(&args[2], env)?;
            let body = self.eval(&args[3], env)?;
            let Value::String(method) = method else {
                return Err(self.runtime_error(args[0].span, "http_json expects a String method"));
            };
            let Value::String(url) = url else {
                return Err(self.runtime_error(args[1].span, "http_json expects a String URL"));
            };
            let Value::Array(headers) = headers else {
                return Err(self.runtime_error(args[2].span, "http_json expects String[] headers"));
            };
            let mut header_strings = Vec::with_capacity(headers.len() + 1);
            let mut has_content_type = false;
            for header in headers {
                let Value::String(header) = header else {
                    return Err(
                        self.runtime_error(args[2].span, "http_json expects String[] headers")
                    );
                };
                if header
                    .split_once(':')
                    .is_some_and(|(name, _)| name.trim().eq_ignore_ascii_case("content-type"))
                {
                    has_content_type = true;
                }
                header_strings.push(header);
            }
            if !has_content_type {
                header_strings.push("Content-Type: application/json".into());
            }
            let body = match body {
                Value::Option(Some(body)) => Some(body),
                Value::Option(None) => None,
                _ => {
                    return Err(
                        self.runtime_error(args[3].span, "http_json expects a Request? body")
                    );
                }
            };
            self.http_json(
                &method,
                &url,
                &header_strings,
                body.as_deref(),
                &type_args[1],
                span,
            )
        } else if name == "http_result" {
            if type_args.len() != 2 || args.len() != 4 {
                return Err(self.runtime_error(
                    span,
                    "http_result expects request and response type arguments plus four values",
                ));
            }
            self.require_runtime_capability("Network", "network access", span)?;
            let method = self.eval(&args[0], env)?;
            let url = self.eval(&args[1], env)?;
            let headers = self.eval(&args[2], env)?;
            let body = self.eval(&args[3], env)?;
            let Value::String(method) = method else {
                return Err(self.runtime_error(args[0].span, "http_result expects a String method"));
            };
            let Value::String(url) = url else {
                return Err(self.runtime_error(args[1].span, "http_result expects a String URL"));
            };
            let Value::Array(headers) = headers else {
                return Err(
                    self.runtime_error(args[2].span, "http_result expects String[] headers")
                );
            };
            let mut header_strings = Vec::with_capacity(headers.len() + 1);
            let mut has_content_type = false;
            for header in headers {
                let Value::String(header) = header else {
                    return Err(
                        self.runtime_error(args[2].span, "http_result expects String[] headers")
                    );
                };
                if header
                    .split_once(':')
                    .is_some_and(|(name, _)| name.trim().eq_ignore_ascii_case("content-type"))
                {
                    has_content_type = true;
                }
                header_strings.push(header);
            }
            if !has_content_type {
                header_strings.push("Content-Type: application/json".into());
            }
            let body = match body {
                Value::Option(Some(body)) => Some(body),
                Value::Option(None) => None,
                _ => {
                    return Err(
                        self.runtime_error(args[3].span, "http_result expects a Request? body")
                    );
                }
            };
            self.http_result(
                &method,
                &url,
                &header_strings,
                body.as_deref(),
                &type_args[1],
                span,
            )
        } else if name == "http_request" {
            if args.len() != 4 {
                return Err(self.runtime_error(
                            span,
                            "http_request expects String method, String URL, String[] headers, and String? body",
                        ));
            }
            self.require_runtime_capability("Network", "network access", span)?;
            let method = self.eval(&args[0], env)?;
            let url = self.eval(&args[1], env)?;
            let headers = self.eval(&args[2], env)?;
            let body = self.eval(&args[3], env)?;
            let Value::String(method) = method else {
                return Err(
                    self.runtime_error(args[0].span, "http_request expects a String method")
                );
            };
            let Value::String(url) = url else {
                return Err(self.runtime_error(args[1].span, "http_request expects a String URL"));
            };
            let Value::Array(headers) = headers else {
                return Err(
                    self.runtime_error(args[2].span, "http_request expects String[] headers")
                );
            };
            let mut header_strings = Vec::with_capacity(headers.len());
            for header in headers {
                let Value::String(header) = header else {
                    return Err(
                        self.runtime_error(args[2].span, "http_request expects String[] headers")
                    );
                };
                header_strings.push(header);
            }
            let body = match body {
                Value::Option(Some(body)) => match *body {
                    Value::String(body) => Some(body),
                    _ => {
                        return Err(
                            self.runtime_error(args[3].span, "http_request expects a String? body")
                        );
                    }
                },
                Value::Option(None) => None,
                _ => {
                    return Err(
                        self.runtime_error(args[3].span, "http_request expects a String? body")
                    );
                }
            };
            self.http_request(&method, &url, &header_strings, body.as_deref(), span)
        } else if name == "mail_queue" {
            Err(self.runtime_error(span, "mail_queue is only allowed inside a transaction"))
        } else if name == "mail_deliver" {
            if args.len() != 2 {
                return Err(self.runtime_error(
                    span,
                    "mail_deliver expects an outbox event ID and payload JSON",
                ));
            }
            self.require_runtime_capability("Email", "email delivery", span)?;
            let event_id = self.eval(&args[0], env)?;
            let payload_json = self.eval(&args[1], env)?;
            let (Value::String(event_id), Value::String(payload_json)) = (event_id, payload_json)
            else {
                return Err(self.runtime_error(span, "mail_deliver expects String arguments"));
            };
            let result = deliver_outbox_message_with_host(
                &event_id,
                &payload_json,
                self.mail_policy.as_ref(),
                self.network_policy.as_ref(),
                self.host_services.as_ref(),
            )
            .map(|()| Box::new(Value::Unit))
            .map_err(|error| Box::new(Value::String(error.to_owned())));
            Ok(Value::Result(result))
        } else if name == "run_process" {
            if args.len() != 2 {
                return Err(self.runtime_error(
                    span,
                    "run_process expects a String command and String[] arguments",
                ));
            }
            self.require_runtime_capability("Process", "process execution", span)?;
            let command = self.eval(&args[0], env)?;
            let arguments = self.eval(&args[1], env)?;
            let Value::String(command) = command else {
                return Err(
                    self.runtime_error(args[0].span, "run_process expects a String command")
                );
            };
            let Value::Array(arguments) = arguments else {
                return Err(
                    self.runtime_error(args[1].span, "run_process expects String[] arguments")
                );
            };
            let mut argument_strings = Vec::with_capacity(arguments.len());
            for argument in arguments {
                let Value::String(argument) = argument else {
                    return Err(
                        self.runtime_error(args[1].span, "run_process expects String[] arguments")
                    );
                };
                argument_strings.push(argument);
            }
            self.run_process(&command, &argument_strings, span)
                .map(Value::String)
        } else {
            Err(self.runtime_error(span, format!("unknown function `{name}`")))
        }
    }
}
