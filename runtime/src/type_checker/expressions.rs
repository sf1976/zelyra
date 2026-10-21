//! Expression type inference and checking for the shared compiler checker.

use super::*;

impl Checker<'_> {
    pub(super) fn check_expr_inner(
        &mut self,
        expr: &Expr,
        scopes: &[HashMap<String, Variable>],
    ) -> Type {
        match &expr.kind {
            ExprKind::Int(_) => Type::Int,
            ExprKind::UInt(_) => Type::UInt,
            ExprKind::Float(_) => Type::Float,
            ExprKind::Bool(_) => Type::Bool,
            ExprKind::String(_) => Type::String,
            ExprKind::Char(_) => Type::Char,
            ExprKind::FunctionValue(name) => {
                let Some(signature) = self.functions.get(name).cloned() else {
                    self.error_with_code(
                        "E-TYPE-FUNCTION-001",
                        expr.span,
                        format!("unknown function value `{name}`"),
                    );
                    return Type::Unknown;
                };
                if !signature.type_params.is_empty() {
                    self.error_with_code(
                        "E-TYPE-FUNCTION-001",
                        expr.span,
                        format!(
                            "generic function `{name}` cannot be stored as a function value yet"
                        ),
                    );
                    return Type::Unknown;
                }
                Type::Function {
                    params: signature.params,
                    return_type: Box::new(signature.return_type),
                    capabilities: signature.capabilities,
                }
            }
            ExprKind::Lambda { params, body } => {
                let mut lambda_scopes = lambda_capture_scopes(scopes);
                let mut parameter_types = Vec::with_capacity(params.len());
                let mut parameter_scope = HashMap::new();
                for parameter in params {
                    parameter_types.push(parameter.ty.clone());
                    parameter_scope.insert(
                        parameter.name.clone(),
                        Variable {
                            ty: parameter.ty.clone(),
                            mutable: false,
                        },
                    );
                }
                lambda_scopes.push(parameter_scope);
                let return_type = self.check_expr(body, &lambda_scopes);
                Type::Function {
                    params: parameter_types,
                    return_type: Box::new(return_type),
                    capabilities: Vec::new(),
                }
            }
            ExprKind::Array(values) => {
                let mut element = Type::Unknown;
                for value in values {
                    let value_type = self.check_expr(value, scopes);
                    if element == Type::Unknown {
                        element = value_type;
                    } else {
                        self.expect_type(&element, &value_type, value.span);
                    }
                }
                Type::Array(Box::new(element))
            }
            ExprKind::Map(entries) => {
                let mut key_type = Type::Unknown;
                let mut value_type = Type::Unknown;
                for (key, value) in entries {
                    let actual_key = self.check_expr(key, scopes);
                    let actual_value = self.check_expr(value, scopes);
                    if key_type == Type::Unknown {
                        key_type = actual_key;
                    } else {
                        self.expect_type(&key_type, &actual_key, key.span);
                    }
                    if value_type == Type::Unknown {
                        value_type = actual_value;
                    } else {
                        self.expect_type(&value_type, &actual_value, value.span);
                    }
                }
                if key_type != Type::Unknown && !is_map_key_type(&key_type) {
                    self.error(
                        expr.span,
                        format!(
                            "Map keys must be Int, UInt, String, Bool, Char, or Timestamp; found `{key_type}`"
                        ),
                    );
                }
                Type::Map(Box::new(key_type), Box::new(value_type))
            }
            ExprKind::Record {
                type_name,
                type_args,
                fields,
            } => {
                let Some(record) = self
                    ._program
                    .records
                    .iter()
                    .find(|record| record.name == *type_name)
                    .cloned()
                else {
                    for (_, value) in fields {
                        self.check_expr(value, scopes);
                    }
                    self.error(expr.span, format!("unknown record type `{type_name}`"));
                    return Type::Unknown;
                };
                let generic_parameters = record.type_params.iter().cloned().collect::<HashSet<_>>();
                if type_args.len() != record.type_params.len() {
                    if record.type_params.is_empty() && !type_args.is_empty() {
                        self.error_with_code(
                            "E-TYPE-GENERIC-001",
                            expr.span,
                            format!("record `{type_name}` does not accept type arguments"),
                        );
                    } else if !type_args.is_empty() {
                        self.error_with_code(
                            "E-TYPE-GENERIC-001",
                            expr.span,
                            format!(
                                "record `{type_name}` expects {} type argument(s), found {}",
                                record.type_params.len(),
                                type_args.len()
                            ),
                        );
                    }
                }
                for argument in type_args {
                    self.check_type(argument, expr.span);
                }
                let mut substitutions = record
                    .type_params
                    .iter()
                    .zip(type_args)
                    .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                    .collect::<HashMap<_, _>>();
                let mut provided = HashSet::new();
                let mut checked_fields = Vec::new();
                for (field_name, value) in fields {
                    let actual = self.check_expr(value, scopes);
                    if !provided.insert(field_name) {
                        self.error(
                            value.span,
                            format!("record field `{field_name}` is specified more than once"),
                        );
                        continue;
                    }
                    let Some(field) = record.fields.iter().find(|field| field.name == *field_name)
                    else {
                        self.error(
                            value.span,
                            format!("unknown field `{field_name}` in record `{type_name}`"),
                        );
                        continue;
                    };
                    infer_generic_types(
                        &field.ty,
                        &actual,
                        &generic_parameters,
                        &mut substitutions,
                    );
                    checked_fields.push((field, value, actual));
                }
                for (field, value, actual) in checked_fields {
                    let expected = substitute_generic_type(&field.ty, &substitutions);
                    self.expect_type(&expected, &actual, value.span);
                }
                for field in &record.fields {
                    if !provided.contains(&field.name) && !matches!(field.ty, Type::Option(_)) {
                        self.error(
                            expr.span,
                            format!("missing field `{}` in record `{type_name}`", field.name),
                        );
                    }
                }
                let applied_args = record
                    .type_params
                    .iter()
                    .map(|parameter| {
                        substitutions
                            .get(parameter)
                            .cloned()
                            .unwrap_or(Type::Unknown)
                    })
                    .collect::<Vec<_>>();
                if let Some(missing) = record
                    .type_params
                    .iter()
                    .find(|parameter| !substitutions.contains_key(*parameter))
                {
                    self.error_with_code(
                        "E-TYPE-GENERIC-001",
                        expr.span,
                        format!(
                            "cannot infer type parameter `{missing}` for record `{type_name}`; use `{type_name}<...>`"
                        ),
                    );
                }
                if record.type_params.is_empty() {
                    Type::Named(type_name.clone())
                } else {
                    Type::Applied {
                        name: type_name.clone(),
                        args: applied_args,
                    }
                }
            }
            ExprKind::Variable(name) => {
                if let Some((enum_name, variant)) = name.rsplit_once("::") {
                    if let Some(definition) = self
                        ._program
                        .enums
                        .iter()
                        .find(|definition| definition.name == enum_name)
                        .filter(|definition| definition.variants.iter().any(|item| item == variant))
                    {
                        if definition
                            .variant_payloads
                            .iter()
                            .find(|(name, _)| name == variant)
                            .is_some_and(|(_, payloads)| !payloads.is_empty())
                        {
                            self.error_with_code(
                                "E-TYPE-ENUM-001",
                                expr.span,
                                format!(
                                    "enum variant `{enum_name}::{variant}` requires payload values"
                                ),
                            );
                            return Type::Unknown;
                        }
                        if !definition.type_params.is_empty() {
                            self.error_with_code(
                                "E-TYPE-GENERIC-001",
                                expr.span,
                                format!(
                                    "generic enum variant `{name}` requires explicit type arguments; use `{enum_name}::{variant}<...>()`"
                                ),
                            );
                            Type::Unknown
                        } else {
                            Type::Named(enum_name.to_owned())
                        }
                    } else {
                        self.error_with_code(
                            "E-TYPE-ENUM-001",
                            expr.span,
                            format!("unknown enum variant `{name}`"),
                        );
                        Type::Unknown
                    }
                } else {
                    self.lookup(scopes, name)
                        .map(|v| v.ty.clone())
                        .or_else(|| {
                            if name == "None" {
                                Some(Type::Option(Box::new(Type::Unknown)))
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| {
                            self.error(expr.span, format!("unknown variable `{name}`"));
                            Type::Unknown
                        })
                }
            }
            ExprKind::Call {
                name,
                type_args,
                args,
            } => self.check_call_expr(name, type_args, args, expr.span, scopes),
            ExprKind::Unary { op, expr: inner } => {
                let ty = self.check_expr(inner, scopes);
                match op {
                    UnaryOp::Negate => {
                        if !is_numeric(&ty) && ty != Type::Unknown {
                            self.error(expr.span, "unary `-` requires a numeric value");
                        }
                        ty
                    }
                    UnaryOp::Not => {
                        self.expect_type(&Type::Bool, &ty, inner.span);
                        Type::Bool
                    }
                }
            }
            ExprKind::Index { target, index } => {
                let target_type = self.check_expr(target, scopes);
                let index_type = self.check_expr(index, scopes);
                if !matches!(index_type, Type::Int | Type::UInt | Type::Unknown) {
                    self.error(
                        index.span,
                        format!("array index must be an integer, found `{index_type}`"),
                    );
                }
                match target_type {
                    Type::Array(inner) => *inner,
                    Type::Tuple(items) => match index.kind {
                        ExprKind::Int(value) if value >= 0 => {
                            items.get(value as usize).cloned().unwrap_or_else(|| {
                                self.error(
                                    index.span,
                                    format!("tuple index {value} is out of bounds"),
                                );
                                Type::Unknown
                            })
                        }
                        ExprKind::UInt(value) => {
                            items.get(value as usize).cloned().unwrap_or_else(|| {
                                self.error(
                                    index.span,
                                    format!("tuple index {value} is out of bounds"),
                                );
                                Type::Unknown
                            })
                        }
                        _ => {
                            self.error(
                                index.span,
                                "tuple index must be a non-negative integer literal",
                            );
                            Type::Unknown
                        }
                    },
                    Type::Unknown => Type::Unknown,
                    other => {
                        self.error(
                            target.span,
                            format!("indexing requires an array or Tuple, found `{other}`"),
                        );
                        Type::Unknown
                    }
                }
            }
            ExprKind::Field { target, field } => {
                let target_type = self.check_expr(target, scopes);
                match self.record_field_type(&target_type, field) {
                    Some(field_type) => field_type,
                    None if target_type == Type::Unknown => Type::Unknown,
                    None => {
                        self.error(
                            expr.span,
                            format!("unknown field `{field}` on `{target_type}`"),
                        );
                        Type::Unknown
                    }
                }
            }
            ExprKind::Binary { left, op, right } => {
                let _ = (left, op, right);
                self.check_binary_chain(expr, scopes)
            }
            ExprKind::Sql { result_type, .. } => {
                self.check_type(result_type, expr.span);
                result_type.clone()
            }
            ExprKind::Await(inner) => {
                if self.parallel_depth == 0 {
                    self.error(expr.span, "`await` is only valid inside a `parallel` block");
                }
                self.check_expr(inner, scopes)
            }
            ExprKind::Match { value, arms } => {
                let value_type = self.check_expr(value, scopes);
                let mut covered = std::collections::HashSet::new();
                let mut result_type = Type::Unknown;
                for arm in arms {
                    let mut pattern_scope = HashMap::new();
                    self.check_pattern(
                        &arm.pattern,
                        &value_type,
                        &mut pattern_scope,
                        &mut covered,
                        arm.guard.is_none(),
                    );
                    let mut arm_scopes = scopes.to_vec();
                    arm_scopes.push(pattern_scope);
                    if let Some(guard) = &arm.guard {
                        let guard_type = self.check_expr(guard, &arm_scopes);
                        self.expect_type(&Type::Bool, &guard_type, guard.span);
                    }
                    let arm_type = self.check_expr(&arm.value, &arm_scopes);
                    if result_type == Type::Unknown {
                        result_type = arm_type;
                    } else {
                        self.expect_type(&result_type, &arm_type, arm.value.span);
                    }
                }
                self.check_exhaustiveness(&value_type, &covered, expr.span);
                result_type
            }
        }
    }
    pub(super) fn record_field_type(&self, ty: &Type, field: &str) -> Option<Type> {
        if let Type::HttpResult(response) = ty {
            return match field {
                "status" => Some(Type::Int),
                "headers" => Some(Type::Array(Box::new(Type::String))),
                "body" => Some(Type::String),
                "data" => Some(Type::Option(response.clone())),
                "error" => Some(Type::Option(Box::new(Type::Named("HttpError".into())))),
                _ => None,
            };
        }
        let (initial_name, substitutions) = match ty {
            Type::Named(name) => (name.as_str(), HashMap::new()),
            Type::Applied { name, args } => {
                let record = self
                    ._program
                    .records
                    .iter()
                    .find(|record| record.name == *name)?;
                let substitutions = record
                    .type_params
                    .iter()
                    .zip(args)
                    .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                    .collect::<HashMap<_, _>>();
                let field_type = record
                    .fields
                    .iter()
                    .find(|candidate| candidate.name == field)?
                    .ty
                    .clone();
                return Some(substitute_generic_type(&field_type, &substitutions));
            }
            _ => return None,
        };
        if initial_name == "HttpError" {
            return match field {
                "status" => Some(Type::Int),
                "headers" => Some(Type::Array(Box::new(Type::String))),
                "body" | "message" => Some(Type::String),
                _ => None,
            };
        }
        let mut name = initial_name;
        if name == "HttpResponse" {
            return match field {
                "status" => Some(Type::Int),
                "headers" => Some(Type::Array(Box::new(Type::String))),
                "body" => Some(Type::String),
                _ => None,
            };
        }
        if name == "ProcessResult" {
            return match field {
                "exit_code" => Some(Type::Int),
                "stdout" | "stderr" => Some(Type::String),
                _ => None,
            };
        }
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(name) {
                return None;
            }
            if let Some(record) = self
                ._program
                .records
                .iter()
                .find(|record| record.name == name)
            {
                return record
                    .fields
                    .iter()
                    .find(|candidate| candidate.name == field)
                    .map(|candidate| substitute_generic_type(&candidate.ty, &substitutions));
            }
            if let Some(record) = sql_results::definitions(self._program).get(name) {
                return record
                    .fields
                    .iter()
                    .find(|candidate| candidate.name == field)
                    .map(|candidate| candidate.ty.clone());
            }
            let alias = self
                ._program
                .types
                .iter()
                .find(|definition| definition.name == name)?;
            let Type::Named(next_name) = &alias.target else {
                return None;
            };
            name = next_name;
        }
    }
    pub(super) fn check_expr(&mut self, expr: &Expr, scopes: &[HashMap<String, Variable>]) -> Type {
        self.check_expr_expected(expr, scopes, None)
    }

    pub(super) fn check_expr_expected(
        &mut self,
        expr: &Expr,
        scopes: &[HashMap<String, Variable>],
        expected: Option<&Type>,
    ) -> Type {
        let contextual_type = expected.map(|ty| self.normalize_alias(ty, expr.span));
        let ty = match (&expr.kind, contextual_type) {
            (ExprKind::Array(values), Some(Type::Array(expected_element))) => {
                for value in values {
                    let actual = self.check_expr_expected(value, scopes, Some(&expected_element));
                    self.expect_type(&expected_element, &actual, value.span);
                }
                Type::Array(expected_element)
            }
            (ExprKind::Map(entries), Some(Type::Map(expected_key, expected_value))) => {
                for (key, value) in entries {
                    let actual_key = self.check_expr_expected(key, scopes, Some(&expected_key));
                    let actual_value =
                        self.check_expr_expected(value, scopes, Some(&expected_value));
                    self.expect_type(&expected_key, &actual_key, key.span);
                    self.expect_type(&expected_value, &actual_value, value.span);
                }
                Type::Map(expected_key, expected_value)
            }
            (ExprKind::Call { name, args, .. }, Some(Type::Option(expected_value)))
                if name == "Some" && args.len() == 1 =>
            {
                let actual = self.check_expr_expected(&args[0], scopes, Some(&expected_value));
                self.expect_type(&expected_value, &actual, args[0].span);
                Type::Option(expected_value)
            }
            (ExprKind::Call { name, args, .. }, Some(Type::Option(expected_value)))
                if name == "None" && args.is_empty() =>
            {
                Type::Option(expected_value)
            }
            (ExprKind::Variable(name), Some(Type::Option(expected_value))) if name == "None" => {
                Type::Option(expected_value)
            }
            (
                ExprKind::Call { name, args, .. },
                Some(Type::Result(expected_ok, expected_error)),
            ) if (name == "Ok" || name == "Err") && args.len() == 1 => {
                let expected_argument = if name == "Ok" {
                    &expected_ok
                } else {
                    &expected_error
                };
                let actual = self.check_expr_expected(&args[0], scopes, Some(expected_argument));
                self.expect_type(expected_argument, &actual, args[0].span);
                Type::Result(expected_ok, expected_error)
            }
            (ExprKind::Match { value, arms }, Some(expected_result)) => {
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
                    let mut arm_scopes = scopes.to_vec();
                    arm_scopes.push(pattern_scope);
                    if let Some(guard) = &arm.guard {
                        let guard_type = self.check_expr(guard, &arm_scopes);
                        self.expect_type(&Type::Bool, &guard_type, guard.span);
                    }
                    let arm_type =
                        self.check_expr_expected(&arm.value, &arm_scopes, Some(&expected_result));
                    self.expect_type(&expected_result, &arm_type, arm.value.span);
                }
                self.check_exhaustiveness(&value_type, &covered, expr.span);
                expected_result
            }
            (
                ExprKind::Lambda { params, body },
                Some(Type::Function {
                    params: expected_params,
                    return_type: expected_return,
                    capabilities,
                }),
            ) => {
                if params.len() != expected_params.len() {
                    self.error(
                        expr.span,
                        format!(
                            "lambda expects {} parameter(s) from its function type, found {}",
                            expected_params.len(),
                            params.len()
                        ),
                    );
                }
                let mut parameter_scope = HashMap::new();
                let mut parameter_types = Vec::with_capacity(params.len());
                for (index, parameter) in params.iter().enumerate() {
                    let expected_parameter =
                        expected_params.get(index).cloned().unwrap_or(Type::Unknown);
                    let parameter_type = if parameter.ty == Type::Unknown {
                        expected_parameter
                    } else {
                        self.expect_type(&expected_parameter, &parameter.ty, parameter.span);
                        parameter.ty.clone()
                    };
                    parameter_types.push(parameter_type.clone());
                    parameter_scope.insert(
                        parameter.name.clone(),
                        Variable {
                            ty: parameter_type,
                            mutable: false,
                        },
                    );
                }
                let mut lambda_scopes = lambda_capture_scopes(scopes);
                lambda_scopes.push(parameter_scope);
                let inferred_return =
                    self.check_expr_expected(body, &lambda_scopes, Some(&expected_return));
                self.expect_type(&expected_return, &inferred_return, body.span);
                let required_capabilities =
                    self.expression_required_capabilities(body, &lambda_scopes);
                let declared_capabilities = capabilities
                    .iter()
                    .map(String::as_str)
                    .collect::<HashSet<_>>();
                for capability in required_capabilities {
                    if !function_declares_capability(&declared_capabilities, &capability) {
                        self.error_with_code(
                            "E-TYPE-FUNCTION-001",
                            expr.span,
                            format!(
                                "lambda requires capability `{capability}`; declare it in the function type with `uses {{{capability}}}`"
                            ),
                        );
                    }
                }
                Type::Function {
                    params: parameter_types,
                    return_type: Box::new(if inferred_return == Type::Unknown {
                        *expected_return
                    } else {
                        inferred_return
                    }),
                    capabilities,
                }
            }
            _ => self.check_expr_inner(expr, scopes),
        };
        let ty = self.normalize_alias(&ty, expr.span);
        self.record_inferred_expression_type(expr.span, &ty);
        ty
    }

    /// Find effects executed by a lambda body without descending into nested
    /// lambda bodies, which are separate function values with their own
    /// capability contracts.
    fn expression_required_capabilities(
        &self,
        expression: &Expr,
        scopes: &[HashMap<String, Variable>],
    ) -> HashSet<String> {
        let mut required = HashSet::new();
        let mut pending = vec![expression];
        while let Some(expression) = pending.pop() {
            match &expression.kind {
                ExprKind::Sql { query, .. } => {
                    required.insert(sql_required_capability(query).to_owned());
                }
                ExprKind::Call { name, args, .. } => {
                    required.extend(
                        builtin_capability_requirements(name)
                            .iter()
                            .map(|(capability, _)| (*capability).to_owned()),
                    );
                    if let Some(signature) = self.functions.get(name) {
                        required.extend(signature.capabilities.iter().cloned());
                    } else if let Some(Type::Function { capabilities, .. }) =
                        self.lookup(scopes, name).map(|variable| &variable.ty)
                    {
                        required.extend(capabilities.iter().cloned());
                    }
                    pending.extend(args);
                }
                ExprKind::Array(values) => pending.extend(values),
                ExprKind::Map(entries) => {
                    for (key, value) in entries {
                        pending.push(key);
                        pending.push(value);
                    }
                }
                ExprKind::Record { fields, .. } => {
                    pending.extend(fields.iter().map(|(_, value)| value));
                }
                ExprKind::Index { target, index } => {
                    pending.push(target);
                    pending.push(index);
                }
                ExprKind::Field { target, .. } | ExprKind::Unary { expr: target, .. } => {
                    pending.push(target);
                }
                ExprKind::Binary { left, right, .. } => {
                    pending.push(left);
                    pending.push(right);
                }
                ExprKind::Await(inner) => pending.push(inner),
                ExprKind::Match { value, arms } => {
                    pending.push(value);
                    pending.extend(arms.iter().map(|arm| &arm.value));
                }
                ExprKind::Lambda { .. }
                | ExprKind::Int(_)
                | ExprKind::UInt(_)
                | ExprKind::Float(_)
                | ExprKind::Bool(_)
                | ExprKind::String(_)
                | ExprKind::Char(_)
                | ExprKind::FunctionValue(_)
                | ExprKind::Variable(_) => {}
            }
        }
        required
    }

    fn record_inferred_expression_type(&mut self, span: Span, ty: &Type) {
        if self.capture_inferred_expression_types {
            self.inferred_expression_types.push(InferredExpressionType {
                span,
                ty: ty.clone(),
            });
        }
    }
    fn check_callable(
        &mut self,
        expression: &Expr,
        expected_params: &[Type],
        scopes: &[HashMap<String, Variable>],
    ) -> Type {
        if let ExprKind::Lambda { params, body } = &expression.kind {
            if params.len() != expected_params.len() {
                self.error(
                    expression.span,
                    format!(
                        "lambda expects {} parameter(s) in this operation, found {}",
                        expected_params.len(),
                        params.len()
                    ),
                );
            }
            let mut parameter_scope = HashMap::new();
            let mut parameter_types = Vec::new();
            for (index, parameter) in params.iter().enumerate() {
                let expected = expected_params.get(index).cloned().unwrap_or(Type::Unknown);
                let ty = if parameter.ty == Type::Unknown {
                    expected
                } else {
                    self.expect_type(&expected, &parameter.ty, parameter.span);
                    parameter.ty.clone()
                };
                parameter_types.push(ty.clone());
                parameter_scope.insert(parameter.name.clone(), Variable { ty, mutable: false });
            }
            let mut lambda_scopes = lambda_capture_scopes(scopes);
            lambda_scopes.push(parameter_scope);
            let return_type = self.check_expr(body, &lambda_scopes);
            let callback = Type::Function {
                params: parameter_types,
                return_type: Box::new(return_type),
                capabilities: Vec::new(),
            };
            self.record_inferred_expression_type(expression.span, &callback);
            return callback;
        }
        let callable = self.check_expr(expression, scopes);
        let Type::Function {
            params,
            return_type,
            ..
        } = callable
        else {
            self.error(
                expression.span,
                "higher-order operation expects a function or lambda",
            );
            return Type::Function {
                params: expected_params.to_vec(),
                return_type: Box::new(Type::Unknown),
                capabilities: Vec::new(),
            };
        };
        if params.len() != expected_params.len() {
            self.error(
                expression.span,
                format!(
                    "callback expects {} parameter(s), expected {}",
                    params.len(),
                    expected_params.len()
                ),
            );
        }
        for (expected, actual) in expected_params.iter().zip(&params) {
            self.expect_type(expected, actual, expression.span);
        }
        Type::Function {
            params,
            return_type,
            capabilities: Vec::new(),
        }
    }

    pub(super) fn check_higher_order_call(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[Expr],
        span: Span,
        scopes: &[HashMap<String, Variable>],
    ) -> Type {
        if !type_args.is_empty() {
            self.error(
                span,
                format!("`{name}` does not accept explicit type arguments"),
            );
        }
        let (expected_count, callback_index) = match name {
            "map" | "filter" | "find" => (2, 1),
            "reduce" => (3, 2),
            _ => unreachable!(),
        };
        if args.len() != expected_count {
            self.error(
                span,
                format!("`{name}` expects {expected_count} argument(s)"),
            );
            return Type::Unknown;
        }
        let (items, item_type) = match self.check_expr(&args[0], scopes) {
            Type::Array(item) => (true, *item),
            Type::Unknown => (true, Type::Unknown),
            other => {
                self.error(
                    args[0].span,
                    format!("`{name}` expects an array, found `{other}`"),
                );
                (false, Type::Unknown)
            }
        };
        if !items {
            return Type::Unknown;
        }
        let callback_params = if name == "reduce" {
            let accumulator_type = self.check_expr(&args[1], scopes);
            vec![accumulator_type.clone(), item_type.clone()]
        } else {
            vec![item_type.clone()]
        };
        let callback = self.check_callable(&args[callback_index], &callback_params, scopes);
        let Type::Function { return_type, .. } = callback else {
            unreachable!()
        };
        match name {
            "map" => Type::Array(return_type),
            "filter" => {
                self.expect_type(&Type::Bool, &return_type, args[callback_index].span);
                Type::Array(Box::new(item_type))
            }
            "find" => {
                self.expect_type(&Type::Bool, &return_type, args[callback_index].span);
                Type::Option(Box::new(item_type))
            }
            "reduce" => {
                let accumulator_type = self.check_expr(&args[1], scopes);
                self.expect_type(&accumulator_type, &return_type, args[callback_index].span);
                accumulator_type
            }
            _ => Type::Unknown,
        }
    }
}
