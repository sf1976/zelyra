use super::*;

impl<'program, 'services> Interpreter<'program, 'services> {
    fn eval_scalar_constructor(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if args.len() != 1 {
            return Err(self.runtime_error(span, format!("{name} expects exactly one argument")));
        }
        let argument = self.eval(&args[0], env)?;
        match (name, argument) {
            ("decimal", Value::String(source)) => {
                parse_decimal(&source).map(Value::Decimal).ok_or_else(|| {
                    self.runtime_error(
                        args[0].span,
                        "decimal expects a base-10 value with at most 9 fractional digits",
                    )
                })
            }
            ("decimal", value) => Err(self.runtime_error(
                args[0].span,
                format!("decimal expects String, found {}", value.ty()),
            )),
            ("date", Value::String(value)) if valid_date(&value) => Ok(Value::Date(value)),
            ("time", Value::String(value)) if valid_time(&value) => Ok(Value::Time(value)),
            ("date" | "time", Value::String(value)) => Err(self.runtime_error(
                args[0].span,
                format!(
                    "{name} received invalid value `{value}`; expected {}",
                    if name == "date" {
                        "YYYY-MM-DD"
                    } else {
                        "HH:MM:SS"
                    }
                ),
            )),
            ("date" | "time", value) => Err(self.runtime_error(
                args[0].span,
                format!("{name} expects String, found {}", value.ty()),
            )),
            ("duration_ms", Value::Int(value)) => Ok(Value::Duration(value)),
            ("duration_ms", value) => Err(self.runtime_error(
                args[0].span,
                format!("duration_ms expects Int, found {}", value.ty()),
            )),
            ("bytes", Value::Array(values)) => values
                .into_iter()
                .map(|value| match value {
                    Value::Int(value) => u8::try_from(value).map_err(|_| {
                        self.runtime_error(args[0].span, "bytes values must be between 0 and 255")
                    }),
                    other => Err(self.runtime_error(
                        args[0].span,
                        format!("bytes expects Int values, found {}", other.ty()),
                    )),
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Bytes),
            ("bytes", value) => Err(self.runtime_error(
                args[0].span,
                format!("bytes expects an Int array, found {}", value.ty()),
            )),
            _ => Err(self.runtime_error(span, format!("unknown scalar constructor `{name}`"))),
        }
    }
    pub(crate) fn eval_language_value_builtin(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match name {
            "decimal" | "date" | "time" | "duration_ms" | "bytes" => {
                self.eval_scalar_constructor(name, args, env, span)
            }
            _ => self.eval_collection_builtin(name, args, env, span),
        }
    }
    fn eval_collection_builtin(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match name {
            "map" | "filter" | "find" | "reduce" => {
                self.eval_higher_order_builtin(name, args, env, span)
            }
            "tuple" => {
                if args.len() < 2 {
                    return Err(self.runtime_error(span, "tuple expects at least two values"));
                }
                args.iter()
                    .map(|arg| self.eval(arg, env))
                    .collect::<Result<Vec<_>, _>>()
                    .map(Value::Tuple)
            }
            "set" => {
                if args.len() != 1 {
                    return Err(
                        self.runtime_error(span, format!("{name} expects exactly one array"))
                    );
                }
                match self.eval(&args[0], env)? {
                    Value::Array(values) => Ok(Value::Set(canonical_set(values))),
                    value => Err(self.runtime_error(
                        args[0].span,
                        format!("{name} expects an array, found {}", value.ty()),
                    )),
                }
            }
            "set_add" | "set_remove" => {
                if args.len() != 2 {
                    return Err(
                        self.runtime_error(span, format!("{name} expects a Set and one element"))
                    );
                }
                let collection = self.eval(&args[0], env)?;
                let element = self.eval(&args[1], env)?;
                let Value::Set(mut values) = collection else {
                    return Err(self.runtime_error(
                        args[0].span,
                        format!("{name} expects a Set, found {}", collection.ty()),
                    ));
                };
                if name == "set_add" {
                    if !values.contains(&element) {
                        values.push(element);
                    }
                } else {
                    values.retain(|value| value != &element);
                }
                Ok(Value::Set(canonical_set(values)))
            }
            _ => Err(self.runtime_error(span, format!("unknown collection builtin `{name}`"))),
        }
    }

    fn eval_higher_order_builtin(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let expected_count = if name == "reduce" { 3 } else { 2 };
        if args.len() != expected_count {
            return Err(
                self.runtime_error(span, format!("`{name}` expects {expected_count} arguments"))
            );
        }
        let values = self.eval(&args[0], env)?;
        let Value::Array(values) = values else {
            return Err(self.runtime_error(
                args[0].span,
                format!("`{name}` expects an array, found `{}`", values.ty()),
            ));
        };
        let mut callback_index = 1;
        let mut accumulator = if name == "reduce" {
            callback_index = 2;
            Some(self.eval(&args[1], env)?)
        } else {
            None
        };
        let callback = self.eval(&args[callback_index], env)?;
        let Value::Function(callback) = callback else {
            return Err(self.runtime_error(
                args[callback_index].span,
                format!("`{name}` expects a function or lambda callback"),
            ));
        };
        match name {
            "map" => values
                .into_iter()
                .map(|value| self.invoke_callback(&callback, vec![value], span))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            "filter" => {
                let mut selected = Vec::new();
                for value in values {
                    let keep = self.invoke_callback(&callback, vec![value.clone()], span)?;
                    match keep {
                        Value::Bool(true) => selected.push(value),
                        Value::Bool(false) => {}
                        other => {
                            return Err(self.runtime_error(
                                span,
                                format!("filter callback must return Bool, found `{}`", other.ty()),
                            ));
                        }
                    }
                }
                Ok(Value::Array(selected))
            }
            "find" => {
                for value in values {
                    match self.invoke_callback(&callback, vec![value.clone()], span)? {
                        Value::Bool(true) => return Ok(Value::Option(Some(Box::new(value)))),
                        Value::Bool(false) => {}
                        other => {
                            return Err(self.runtime_error(
                                span,
                                format!("find callback must return Bool, found `{}`", other.ty()),
                            ));
                        }
                    }
                }
                Ok(Value::Option(None))
            }
            "reduce" => {
                let mut current = accumulator
                    .take()
                    .expect("reduce accumulator was evaluated above");
                for value in values {
                    current = self.invoke_callback(&callback, vec![current, value], span)?;
                }
                Ok(current)
            }
            _ => unreachable!(),
        }
    }

    fn invoke_callback(
        &mut self,
        callback: &FunctionValue,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if callback.params.len() != args.len() {
            return Err(self.runtime_error(
                span,
                format!(
                    "callback expects {} argument(s), found {}",
                    callback.params.len(),
                    args.len()
                ),
            ));
        }
        if let Some(caller_capabilities) = self.active_capabilities.last() {
            let declared = caller_capabilities
                .iter()
                .map(String::as_str)
                .collect::<HashSet<_>>();
            if let Some(capability) = callback
                .capabilities
                .iter()
                .find(|capability| !function_declares_capability(&declared, capability))
            {
                return Err(self.runtime_error(
                    span,
                    format!("runtime capability denied: callback requires `{capability}`"),
                ));
            }
        }
        if callback.lambda_body.is_none() {
            return self.call_with_type_args(&callback.name, &[], args, span);
        }
        if self.call_depth >= self.call_depth_limit {
            return Err(self.runtime_error(
                span,
                format!(
                    "function call depth limit exceeded (maximum {})",
                    self.call_depth_limit
                ),
            ));
        }
        self.call_depth += 1;
        let mut lambda_env = Environment::new();
        for (name, value) in &callback.captures {
            lambda_env.declare(name.clone(), value.clone(), false);
        }
        let result = lambda_env.with_scope(|lambda_env| {
            for (name, value) in callback.lambda_params.iter().zip(args) {
                lambda_env.declare(name.clone(), value, false);
            }
            self.eval(
                callback.lambda_body.as_deref().expect("lambda body exists"),
                lambda_env,
            )
        });
        self.call_depth -= 1;
        result
    }
    pub(crate) fn eval_call_expr(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if name == "push_mut" {
            if args.len() != 2 || !type_args.is_empty() {
                return Err(self.runtime_error(
                    span,
                    "`push_mut` expects a mutable array binding and one value",
                ));
            }
            let ExprKind::Variable(binding) = &args[0].kind else {
                return Err(self.runtime_error(
                    args[0].span,
                    "`push_mut` requires the name of a mutable array binding",
                ));
            };
            let value = self.eval(&args[1], env)?;
            env.push_mutable_array(binding, value)
                .map_err(|reason| self.runtime_error(args[0].span, reason))?;
            return Ok(Value::Unit);
        }
        if name == "pop_mut" {
            if args.len() != 1 || !type_args.is_empty() {
                return Err(self.runtime_error(span, "`pop_mut` expects one mutable array binding"));
            }
            let ExprKind::Variable(binding) = &args[0].kind else {
                return Err(self.runtime_error(
                    args[0].span,
                    "`pop_mut` requires the name of a mutable array binding",
                ));
            };
            let value = env
                .pop_mutable_array(binding)
                .map_err(|reason| self.runtime_error(args[0].span, reason))?;
            return Ok(Value::Option(value.map(Box::new)));
        }
        if let Some(Value::Function(function)) = env.get(name) {
            if !type_args.is_empty() {
                return Err(self.runtime_error(
                    span,
                    "indirect function calls do not accept explicit type arguments",
                ));
            }
            if let Some(caller_capabilities) = self.active_capabilities.last() {
                let declared = caller_capabilities
                    .iter()
                    .map(String::as_str)
                    .collect::<HashSet<_>>();
                if let Some(capability) = function
                    .capabilities
                    .iter()
                    .find(|capability| !function_declares_capability(&declared, capability))
                {
                    return Err(self.runtime_error(
                        span,
                        format!(
                            "runtime capability denied: current function does not declare `{capability}` required by indirect call through `{name}`"
                        ),
                    ));
                }
            }
            let values = args
                .iter()
                .map(|argument| self.eval(argument, env))
                .collect::<Result<Vec<_>, _>>()?;
            return self.invoke_callback(&function, values, span);
        }
        if let Some((enum_name, variant)) = name.rsplit_once("::") {
            if let Some(definition) = self.enum_definitions.get(enum_name).cloned() {
                let payload_types = definition
                    .variant_payloads
                    .iter()
                    .find(|(name, _)| name == variant)
                    .map(|(_, types)| types.clone())
                    .ok_or_else(|| {
                        self.runtime_error(
                            span,
                            format!("unknown enum variant `{enum_name}::{variant}`"),
                        )
                    })?;
                if definition.type_params.is_empty() && !type_args.is_empty() {
                    return Err(self.runtime_error(
                        span,
                        format!("enum `{enum_name}` does not accept type arguments"),
                    ));
                }
                if payload_types.is_empty() && definition.type_params.is_empty() {
                    return Err(self.runtime_error(
                        span,
                        format!(
                            "unit enum variant `{enum_name}::{variant}` is written without parentheses"
                        ),
                    ));
                }
                if args.len() != payload_types.len() {
                    return Err(self.runtime_error(
                        span,
                        format!(
                            "enum variant `{enum_name}::{variant}` expects {} payload value(s), found {}",
                            payload_types.len(),
                            args.len()
                        ),
                    ));
                }
                let payloads = args
                    .iter()
                    .map(|argument| self.eval(argument, env))
                    .collect::<Result<Vec<_>, _>>()?;
                if !type_args.is_empty() && type_args.len() != definition.type_params.len() {
                    return Err(self.runtime_error(
                        span,
                        format!(
                            "enum `{enum_name}` expects {} type argument(s), found {}",
                            definition.type_params.len(),
                            type_args.len()
                        ),
                    ));
                }
                let parameters = definition
                    .type_params
                    .iter()
                    .cloned()
                    .collect::<HashSet<_>>();
                let mut substitutions = definition
                    .type_params
                    .iter()
                    .zip(type_args)
                    .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                    .collect::<HashMap<_, _>>();
                if type_args.is_empty() {
                    for (expected, payload) in payload_types.iter().zip(&payloads) {
                        infer_generic_types(
                            expected,
                            &payload.ty(),
                            &parameters,
                            &mut substitutions,
                        );
                    }
                }
                let concrete_args = definition
                    .type_params
                    .iter()
                    .map(|parameter| substitutions.get(parameter).cloned())
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| {
                        self.runtime_error(
                            span,
                            format!("cannot infer generic arguments for enum `{enum_name}`"),
                        )
                    })?;
                let instantiated_payloads = payload_types
                    .iter()
                    .map(|expected| substitute_generic_type(expected, &substitutions))
                    .collect::<Vec<_>>();
                for ((argument, payload), expected) in
                    args.iter().zip(&payloads).zip(&instantiated_payloads)
                {
                    if !compatible(
                        &self.runtime_alias_type(expected, span)?,
                        &self.runtime_alias_type(&payload.ty(), span)?,
                    ) {
                        return Err(self.runtime_error(
                            argument.span,
                            format!(
                                "enum variant `{enum_name}::{variant}` payload expects `{expected}`, found `{}`",
                                payload.ty()
                            ),
                        ));
                    }
                }
                return Ok(Value::Enum {
                    type_name: enum_name.to_owned(),
                    type_args: concrete_args,
                    variant: variant.to_owned(),
                    payloads,
                });
            }
        }
        if !is_runtime_builtin(name) && self.functions.contains_key(name) {
            let values = args
                .iter()
                .map(|arg| self.eval(arg, env))
                .collect::<Result<Vec<_>, _>>()?;
            return self.call_with_type_args(name, type_args, values, span);
        }
        self.eval_builtin_call(name, type_args, args, env, span)
    }
}
