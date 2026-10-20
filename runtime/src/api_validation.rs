//! Validation for declarative API route and JSON boundary definitions.

use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ApiDiagnostic {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for ApiDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub fn check_apis(program: &Program) -> Result<(), Vec<ApiDiagnostic>> {
    let mut errors = Vec::new();
    let mut known_types = [
        "Id",
        "Email",
        "Url",
        "Uuid",
        "Money",
        "Int",
        "UInt",
        "Float",
        "Decimal",
        "Bool",
        "String",
        "Char",
        "Bytes",
        "Timestamp",
        "Date",
        "Time",
        "Duration",
        "Unit",
        "HttpResponse",
        "HttpError",
        "HttpResult",
    ]
    .into_iter()
    .map(String::from)
    .collect::<HashSet<_>>();
    for definition in &program.types {
        known_types.insert(definition.name.clone());
    }
    for record in &program.records {
        known_types.insert(record.name.clone());
    }
    for definition in &program.enums {
        known_types.insert(definition.name.clone());
    }
    for table in &program.tables {
        known_types.insert(table.name.clone());
        if let Some(singular) = singular_table_type(&table.name) {
            known_types.insert(singular);
        }
    }
    let generic_arities = program
        .records
        .iter()
        .map(|record| (record.name.clone(), record.type_params.len()))
        .chain(
            program
                .enums
                .iter()
                .map(|definition| (definition.name.clone(), definition.type_params.len())),
        )
        .collect::<HashMap<_, _>>();

    let mut routes = HashSet::new();
    let functions = program
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function))
        .collect::<HashMap<_, _>>();
    let computed_api_handlers = program
        .apis
        .iter()
        .filter_map(|api| {
            let handler = api.handler.as_ref()?;
            let table_name = api.validates_table.as_ref()?;
            let table = program
                .tables
                .iter()
                .find(|table| &table.name == table_name)?;
            table
                .columns
                .iter()
                .any(|column| column.computed.is_some())
                .then(|| handler.clone())
        })
        .collect::<HashSet<_>>();
    for api in &program.apis {
        if !routes.insert((api.method.clone(), api.path.clone())) {
            errors.push(ApiDiagnostic {
                message: format!("duplicate API route `{}` `{}`", api.method, api.path),
                span: api.span,
            });
        }
        let parameters = match api_path_parameters(&api.path) {
            Ok(parameters) => parameters,
            Err(message) => {
                errors.push(ApiDiagnostic {
                    message,
                    span: api.span,
                });
                Vec::new()
            }
        };
        let mut fields = HashSet::new();
        for field in &api.input {
            if !fields.insert(field.name.clone()) {
                errors.push(ApiDiagnostic {
                    message: format!("duplicate API input field `{}`", field.name),
                    span: field.span,
                });
            }
            if !api_type_known(&field.ty, &known_types, &generic_arities) {
                errors.push(ApiDiagnostic {
                    message: format!("unknown API input type `{}`", field.ty),
                    span: field.span,
                });
            } else if let Some(message) =
                api_json_type_error(&field.ty, program, &mut HashSet::new())
            {
                errors.push(ApiDiagnostic {
                    message: format!("API input field `{}`: {message}", field.name),
                    span: field.span,
                });
            }
        }
        for parameter in parameters {
            if !fields.contains(&parameter) {
                errors.push(ApiDiagnostic {
                    message: format!(
                        "path parameter `{parameter}` must be declared in the API input"
                    ),
                    span: api.span,
                });
            }
        }
        let mut trusted_tenant_column = None;
        if let Some(table_name) = &api.validates_table {
            if api.handler.is_none() {
                errors.push(ApiDiagnostic {
                    message: "API table validation requires a handler".into(),
                    span: api.span,
                });
            }
            let Some(table) = program
                .tables
                .iter()
                .find(|table| &table.name == table_name)
            else {
                errors.push(ApiDiagnostic {
                    message: format!("API validation refers to unknown table `{table_name}`"),
                    span: api.span,
                });
                continue;
            };
            if !matches!(api.method.as_str(), "POST" | "PUT" | "PATCH") {
                errors.push(ApiDiagnostic {
                    message: "table validation is only supported for POST, PUT, or PATCH APIs"
                        .into(),
                    span: api.span,
                });
            }
            if let Some(scope) = &table.tenant_scope {
                if !api.requires_auth {
                    errors.push(ApiDiagnostic {
                        message: format!(
                            "API table validation for scoped table `{table_name}` requires `requires auth`"
                        ),
                        span: api.span,
                    });
                } else if !program.auth.iter().any(|auth| {
                    auth.membership_table.is_some() || auth.tenant_owner_table.is_some()
                }) {
                    errors.push(ApiDiagnostic {
                        message: format!(
                            "API table validation for scoped table `{table_name}` requires auth with `memberships` or `tenant_owner`"
                        ),
                        span: api.span,
                    });
                }
                let tenant_column = table
                    .columns
                    .iter()
                    .find(|column| column.name == scope.column);
                if let Some(column) = tenant_column {
                    trusted_tenant_column = Some(column);
                    if api.input.iter().any(|field| field.name == column.name) {
                        errors.push(ApiDiagnostic {
                            message: format!(
                                "scoped API must not accept tenant key `{}` from request input; Zelyra supplies it from verified membership context",
                                column.name
                            ),
                            span: api.span,
                        });
                    }
                }
            }
            if table.validations.is_empty() {
                errors.push(ApiDiagnostic {
                    message: format!("table `{table_name}` has no validation rules"),
                    span: api.span,
                });
            }
            let computed_columns = table
                .columns
                .iter()
                .filter(|column| column.computed.is_some())
                .collect::<Vec<_>>();
            for column in &computed_columns {
                if api.input.iter().any(|field| field.name == column.name) {
                    errors.push(ApiDiagnostic {
                        message: format!(
                            "API must not accept computed field `{}` from request input; computed fields are passed to the handler by Zelyra",
                            column.name
                        ),
                        span: api.span,
                    });
                }
            }
            for column in table.columns.iter().filter(|column| {
                !column.auto
                    && Some(column.name.as_str())
                        != table
                            .tenant_scope
                            .as_ref()
                            .map(|scope| scope.column.as_str())
                    && column.computed.is_none()
            }) {
                let Some(input) = api.input.iter().find(|field| field.name == column.name) else {
                    if !matches!(column.ty, Type::Option(_)) && column.default.is_none() {
                        errors.push(ApiDiagnostic {
                            message: format!(
                                "API validating `{table_name}` must declare required field `{}` in input",
                                column.name
                            ),
                            span: api.span,
                        });
                    }
                    continue;
                };
                if input.ty != column.ty {
                    errors.push(ApiDiagnostic {
                        message: format!(
                            "API validation field `{}` has type `{}`, expected `{}` from table `{table_name}`",
                            column.name, input.ty, column.ty
                        ),
                        span: input.span,
                    });
                }
            }
            let table_index = program
                .tables
                .iter()
                .position(|candidate| candidate.name == *table_name)
                .expect("table was resolved above");
            let (_, computed_mappings) = zelyra_ast::build_computed_column_functions(program);
            for mapping in computed_mappings
                .iter()
                .filter(|mapping| mapping.table_index == table_index)
            {
                for dependency in &mapping.input_columns {
                    let Some(column) = table
                        .columns
                        .iter()
                        .find(|column| column.name == *dependency)
                    else {
                        continue;
                    };
                    if column.computed.is_some() {
                        continue;
                    }
                    let supplied_by_tenant = table
                        .tenant_scope
                        .as_ref()
                        .is_some_and(|scope| scope.column == column.name);
                    if !supplied_by_tenant
                        && !api.input.iter().any(|field| field.name == column.name)
                    {
                        errors.push(ApiDiagnostic {
                            message: format!(
                                "API validating `{table_name}` must declare computed input dependency `{}`",
                                column.name
                            ),
                            span: api.span,
                        });
                    }
                }
            }
            if !api.errors.iter().any(|error| {
                error.status == 422
                    && error.name == "ValidationError"
                    && error
                        .payload
                        .as_ref()
                        .is_some_and(|payload| compatible(payload, &Type::String))
            }) {
                errors.push(ApiDiagnostic {
                    message:
                        "API table validation requires `errors { 422 ValidationError: String }`"
                            .into(),
                    span: api.span,
                });
            }
        }
        if !api_type_known(&api.output, &known_types, &generic_arities) {
            errors.push(ApiDiagnostic {
                message: format!("unknown API output type `{}`", api.output),
                span: api.span,
            });
        } else if let Some(message) = api_json_type_error(&api.output, program, &mut HashSet::new())
        {
            errors.push(ApiDiagnostic {
                message: format!("API output: {message}"),
                span: api.span,
            });
        }
        if let Some(handler) = &api.handler {
            let Some(function) = functions.get(handler.as_str()) else {
                errors.push(ApiDiagnostic {
                    message: format!("API handler function `{handler}` does not exist"),
                    span: api.span,
                });
                continue;
            };
            if !function.type_params.is_empty() {
                errors.push(ApiDiagnostic {
                    message: format!(
                        "API handler `{handler}` must be non-generic because API routes cannot supply type arguments"
                    ),
                    span: function.span,
                });
                continue;
            }
            let computed_columns = api
                .validates_table
                .as_ref()
                .and_then(|table_name| {
                    program
                        .tables
                        .iter()
                        .find(|table| table.name == *table_name)
                })
                .map(|table| {
                    table
                        .columns
                        .iter()
                        .filter(|column| column.computed.is_some())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let expected_count = api.input.len()
                + computed_columns.len()
                + usize::from(trusted_tenant_column.is_some());
            if function.params.len() != expected_count {
                errors.push(ApiDiagnostic {
                    message: format!(
                        "API handler `{handler}` expects {} parameter(s), but the API declares {} request input field(s), {} server-computed field(s){}",
                        function.params.len(),
                        api.input.len(),
                        computed_columns.len(),
                        if trusted_tenant_column.is_some() { " plus one verified tenant parameter" } else { "" }
                    ),
                    span: api.span,
                });
            }
            for (parameter, field) in function.params.iter().zip(&api.input) {
                if parameter.name != field.name || parameter.ty != field.ty {
                    errors.push(ApiDiagnostic {
                        message: format!(
                            "API handler parameter `{}` must match input field `{}` with type `{}`",
                            parameter.name, field.name, field.ty
                        ),
                        span: field.span,
                    });
                }
            }
            for (parameter, column) in function
                .params
                .iter()
                .skip(api.input.len())
                .take(computed_columns.len())
                .zip(&computed_columns)
            {
                if parameter.name != column.name || parameter.ty != column.ty {
                    errors.push(ApiDiagnostic {
                        message: format!(
                            "API handler parameter `{}` must match server-computed field `{}` with type `{}`",
                            parameter.name, column.name, column.ty
                        ),
                        span: parameter.span,
                    });
                }
            }
            if let Some(column) = trusted_tenant_column {
                if let Some(parameter) = function
                    .params
                    .get(api.input.len() + computed_columns.len())
                {
                    if parameter.name != column.name || parameter.ty != column.ty {
                        errors.push(ApiDiagnostic {
                            message: format!(
                                "API handler final parameter must be verified tenant `{}` with type `{}`",
                                column.name, column.ty
                            ),
                            span: parameter.span,
                        });
                    }
                }
            }
            if function.return_type.as_ref() != Some(&api.output) {
                errors.push(ApiDiagnostic {
                    message: format!(
                        "API handler `{handler}` must return `{}`, found `{}`",
                        api.output,
                        function
                            .return_type
                            .as_ref()
                            .map_or_else(|| "Unit".into(), ToString::to_string)
                    ),
                    span: api.span,
                });
            }
        }
        let mut statuses = HashSet::new();
        for error in &api.errors {
            if !statuses.insert(error.status) {
                errors.push(ApiDiagnostic {
                    message: format!("duplicate API error status `{}`", error.status),
                    span: error.span,
                });
            }
            if let Some(payload) = &error.payload {
                if !api_type_known(payload, &known_types, &generic_arities) {
                    errors.push(ApiDiagnostic {
                        message: format!(
                            "API error `{}` refers to unknown payload type `{payload}`",
                            error.name
                        ),
                        span: error.span,
                    });
                } else if let Some(message) =
                    api_json_type_error(payload, program, &mut HashSet::new())
                {
                    errors.push(ApiDiagnostic {
                        message: format!("API error `{}`: {message}", error.name),
                        span: error.span,
                    });
                }
                match &api.output {
                    Type::Result(_, error_type) if compatible(error_type, payload) => {}
                    Type::Result(_, error_type) => errors.push(ApiDiagnostic {
                        message: format!(
                            "API error `{}` payload type `{payload}` does not match handler error type `{error_type}`",
                            error.name
                        ),
                        span: error.span,
                    }),
                    _ => errors.push(ApiDiagnostic {
                        message: format!(
                            "API error `{}` declares a payload but API output is not a Result",
                            error.name
                        ),
                        span: error.span,
                    }),
                }
            }
        }
    }
    for function in &program.functions {
        if let Some((handler, span)) =
            find_computed_api_handler_reference(&function.body, &computed_api_handlers)
        {
            errors.push(ApiDiagnostic {
                message: format!(
                    "computed API handler `{handler}` cannot be called from Zelyra functions; its computed parameters are injected only at the validated API boundary"
                ),
                span,
            });
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn find_computed_api_handler_reference(
    block: &Block,
    handlers: &HashSet<String>,
) -> Option<(String, Span)> {
    fn expression(expr: &Expr, handlers: &HashSet<String>) -> Option<(String, Span)> {
        match &expr.kind {
            ExprKind::FunctionValue(name) if handlers.contains(name) => {
                Some((name.clone(), expr.span))
            }
            ExprKind::Call { name, args, .. } => {
                if handlers.contains(name) {
                    return Some((name.clone(), expr.span));
                }
                args.iter().find_map(|arg| expression(arg, handlers))
            }
            ExprKind::Array(items) => items.iter().find_map(|item| expression(item, handlers)),
            ExprKind::Map(entries) => entries.iter().find_map(|(key, value)| {
                expression(key, handlers).or_else(|| expression(value, handlers))
            }),
            ExprKind::Record { fields, .. } => fields
                .iter()
                .find_map(|(_, value)| expression(value, handlers)),
            ExprKind::Lambda { body, .. }
            | ExprKind::Field { target: body, .. }
            | ExprKind::Unary { expr: body, .. }
            | ExprKind::Await(body) => expression(body, handlers),
            ExprKind::Index { target, index } => {
                expression(target, handlers).or_else(|| expression(index, handlers))
            }
            ExprKind::Binary { left, right, .. } => {
                expression(left, handlers).or_else(|| expression(right, handlers))
            }
            ExprKind::Match { value, arms } => expression(value, handlers).or_else(|| {
                arms.iter().find_map(|arm| {
                    arm.guard
                        .as_ref()
                        .and_then(|guard| expression(guard, handlers))
                        .or_else(|| expression(&arm.value, handlers))
                })
            }),
            ExprKind::Int(_)
            | ExprKind::UInt(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::String(_)
            | ExprKind::Char(_)
            | ExprKind::FunctionValue(_)
            | ExprKind::Variable(_)
            | ExprKind::Sql { .. } => None,
        }
    }

    fn nested_block(block: &Block, handlers: &HashSet<String>) -> Option<(String, Span)> {
        block
            .statements
            .iter()
            .find_map(|statement| match statement {
                Stmt::Let { value, .. }
                | Stmt::BindOrAssign { value, .. }
                | Stmt::TupleBindOrAssign { value, .. }
                | Stmt::Expr(value) => expression(value, handlers),
                Stmt::ArrayIndexAssign { index, value, .. } => {
                    expression(index, handlers).or_else(|| expression(value, handlers))
                }
                Stmt::Return { value, .. } => {
                    value.as_ref().and_then(|value| expression(value, handlers))
                }
                Stmt::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => expression(condition, handlers)
                    .or_else(|| nested_block(then_block, handlers))
                    .or_else(|| {
                        else_block
                            .as_ref()
                            .and_then(|block| nested_block(block, handlers))
                    }),
                Stmt::While {
                    condition,
                    invariants,
                    body,
                    ..
                } => expression(condition, handlers)
                    .or_else(|| {
                        invariants
                            .iter()
                            .find_map(|item| expression(item, handlers))
                    })
                    .or_else(|| nested_block(body, handlers)),
                Stmt::For { iterable, body, .. } => {
                    expression(iterable, handlers).or_else(|| nested_block(body, handlers))
                }
                Stmt::Loop {
                    invariants, body, ..
                } => invariants
                    .iter()
                    .find_map(|item| expression(item, handlers))
                    .or_else(|| nested_block(body, handlers)),
                Stmt::Match { value, arms, .. } => expression(value, handlers).or_else(|| {
                    arms.iter()
                        .find_map(|arm| nested_block(&arm.body, handlers))
                }),
                Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
                    nested_block(body, handlers)
                }
                Stmt::Break { .. } | Stmt::Continue { .. } => None,
            })
    }

    nested_block(block, handlers)
}

fn api_type_known(
    ty: &Type,
    known_types: &HashSet<String>,
    generic_arities: &HashMap<String, usize>,
) -> bool {
    match ty {
        Type::Named(name) => {
            known_types.contains(name) && generic_arities.get(name).is_none_or(|arity| *arity == 0)
        }
        Type::Applied { name, args } => {
            known_types.contains(name)
                && generic_arities
                    .get(name)
                    .is_some_and(|arity| *arity == args.len())
                && args
                    .iter()
                    .all(|argument| api_type_known(argument, known_types, generic_arities))
        }
        Type::Option(inner) | Type::Array(inner) | Type::Set(inner) | Type::HttpResult(inner) => {
            api_type_known(inner, known_types, generic_arities)
        }
        Type::Tuple(items) => items
            .iter()
            .all(|item| api_type_known(item, known_types, generic_arities)),
        Type::Map(key, value) => {
            api_type_known(key, known_types, generic_arities)
                && api_type_known(value, known_types, generic_arities)
        }
        Type::Result(ok, error) => {
            api_type_known(ok, known_types, generic_arities)
                && api_type_known(error, known_types, generic_arities)
        }
        _ => true,
    }
}

/// Returns an error for values that cannot cross the JSON API boundary without
/// losing their declared shape. JSON objects have string keys, so a Zelyra map
/// exposed through an API must use `String` keys. The traversal also checks
/// aliases and nested records so the guarantee is enforced at the boundary,
/// rather than only for the top-level API type.
fn api_json_type_error(
    ty: &Type,
    program: &Program,
    visiting: &mut HashSet<String>,
) -> Option<String> {
    match ty {
        Type::Option(inner) | Type::Array(inner) | Type::Set(inner) | Type::HttpResult(inner) => {
            api_json_type_error(inner, program, visiting)
        }
        Type::Tuple(items) => items
            .iter()
            .find_map(|item| api_json_type_error(item, program, visiting)),
        Type::Map(key, value) => {
            if **key != Type::String {
                return Some(format!(
                    "JSON API maps require `String` keys, found `{}`",
                    key
                ));
            }
            api_json_type_error(value, program, visiting)
        }
        Type::Result(ok, error) => api_json_type_error(ok, program, visiting)
            .or_else(|| api_json_type_error(error, program, visiting)),
        Type::Function { .. } => Some("function values cannot cross an API JSON boundary".into()),
        Type::Applied { name, args } => {
            if program
                .records
                .iter()
                .any(|record| record.name == *name && record.is_class)
            {
                return Some(format!(
                    "class references of type `{ty}` cannot cross an API JSON boundary"
                ));
            }
            if let Some(error) = args
                .iter()
                .find_map(|argument| api_json_type_error(argument, program, visiting))
            {
                return Some(error);
            }
            let key = ty.to_string();
            if !visiting.insert(key.clone()) {
                return None;
            }
            let substitutions = program
                .records
                .iter()
                .find(|record| record.name == *name)
                .map(|record| {
                    record
                        .type_params
                        .iter()
                        .zip(args)
                        .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                        .collect::<HashMap<_, _>>()
                })
                .or_else(|| {
                    program
                        .enums
                        .iter()
                        .find(|definition| definition.name == *name)
                        .map(|definition| {
                            definition
                                .type_params
                                .iter()
                                .zip(args)
                                .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                                .collect::<HashMap<_, _>>()
                        })
                });
            let result = if let Some(substitutions) = substitutions {
                if let Some(record) = program.records.iter().find(|record| record.name == *name) {
                    record.fields.iter().find_map(|field| {
                        let field_type = substitute_generic_type(&field.ty, &substitutions);
                        api_json_type_error(&field_type, program, visiting)
                    })
                } else if let Some(definition) = program
                    .enums
                    .iter()
                    .find(|definition| definition.name == *name)
                {
                    definition
                        .variant_payloads
                        .iter()
                        .flat_map(|(_, payloads)| payloads)
                        .find_map(|payload| {
                            let payload_type = substitute_generic_type(payload, &substitutions);
                            api_json_type_error(&payload_type, program, visiting)
                        })
                } else {
                    None
                }
            } else {
                None
            };
            visiting.remove(&key);
            result
        }
        Type::Named(name) => {
            if program
                .records
                .iter()
                .any(|record| record.name == *name && record.is_class)
            {
                return Some(format!(
                    "class references of type `{name}` cannot cross an API JSON boundary"
                ));
            }
            if !visiting.insert(name.clone()) {
                return None;
            }
            let result = program
                .types
                .iter()
                .find(|definition| definition.name == *name)
                .and_then(|definition| api_json_type_error(&definition.target, program, visiting))
                .or_else(|| {
                    program
                        .records
                        .iter()
                        .find(|record| record.name == *name)
                        .and_then(|record| {
                            record
                                .fields
                                .iter()
                                .find_map(|field| api_json_type_error(&field.ty, program, visiting))
                        })
                })
                .or_else(|| {
                    program
                        .tables
                        .iter()
                        .find(|table| {
                            table.name == *name
                                || singular_table_type(&table.name).as_deref() == Some(name)
                        })
                        .and_then(|table| {
                            table.columns.iter().find_map(|column| {
                                api_json_type_error(&column.ty, program, visiting)
                            })
                        })
                });
            visiting.remove(name);
            result
        }
        _ => None,
    }
}

pub(crate) fn contains_class_reference_type(ty: &Type, program: &Program) -> bool {
    fn contains(ty: &Type, program: &Program, visiting: &mut HashSet<String>) -> bool {
        let record = |name: &str| program.records.iter().find(|item| item.name == name);
        match ty {
            Type::Named(name) => {
                if record(name).is_some_and(|definition| definition.is_class) {
                    return true;
                }
                if !visiting.insert(ty.to_string()) {
                    return false;
                }
                let found = program
                    .types
                    .iter()
                    .find(|alias| alias.name == *name)
                    .is_some_and(|alias| contains(&alias.target, program, visiting))
                    || record(name).is_some_and(|definition| {
                        definition
                            .fields
                            .iter()
                            .any(|field| contains(&field.ty, program, visiting))
                    })
                    || program
                        .enums
                        .iter()
                        .find(|definition| definition.name == *name)
                        .is_some_and(|definition| {
                            definition
                                .variant_payloads
                                .iter()
                                .flat_map(|(_, payloads)| payloads)
                                .any(|payload| contains(payload, program, visiting))
                        });
                visiting.remove(&ty.to_string());
                found
            }
            Type::Applied { name, args } => {
                if record(name).is_some_and(|definition| definition.is_class)
                    || args
                        .iter()
                        .any(|argument| contains(argument, program, visiting))
                {
                    return true;
                }
                let key = ty.to_string();
                if !visiting.insert(key.clone()) {
                    return false;
                }
                let substitutions = record(name)
                    .map(|definition| {
                        definition
                            .type_params
                            .iter()
                            .zip(args)
                            .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                            .collect::<HashMap<_, _>>()
                    })
                    .or_else(|| {
                        program
                            .enums
                            .iter()
                            .find(|definition| definition.name == *name)
                            .map(|definition| {
                                definition
                                    .type_params
                                    .iter()
                                    .zip(args)
                                    .map(|(parameter, argument)| {
                                        (parameter.clone(), argument.clone())
                                    })
                                    .collect::<HashMap<_, _>>()
                            })
                    });
                let found = if let Some(substitutions) = substitutions {
                    record(name)
                        .map(|definition| {
                            definition.fields.iter().any(|field| {
                                let field_type = substitute_generic_type(&field.ty, &substitutions);
                                contains(&field_type, program, visiting)
                            })
                        })
                        .or_else(|| {
                            program
                                .enums
                                .iter()
                                .find(|definition| definition.name == *name)
                                .map(|definition| {
                                    definition
                                        .variant_payloads
                                        .iter()
                                        .flat_map(|(_, payloads)| payloads)
                                        .any(|payload| {
                                            let payload_type =
                                                substitute_generic_type(payload, &substitutions);
                                            contains(&payload_type, program, visiting)
                                        })
                                })
                        })
                        .unwrap_or(false)
                } else {
                    false
                };
                visiting.remove(&key);
                found
            }
            Type::Option(inner)
            | Type::Array(inner)
            | Type::Set(inner)
            | Type::HttpResult(inner) => contains(inner, program, visiting),
            Type::Result(ok, error) => {
                contains(ok, program, visiting) || contains(error, program, visiting)
            }
            Type::Tuple(items) => items.iter().any(|item| contains(item, program, visiting)),
            Type::Map(key, value) => {
                contains(key, program, visiting) || contains(value, program, visiting)
            }
            Type::Function {
                params,
                return_type,
                ..
            } => {
                params
                    .iter()
                    .any(|parameter| contains(parameter, program, visiting))
                    || contains(return_type, program, visiting)
            }
            _ => false,
        }
    }

    contains(ty, program, &mut HashSet::new())
}

fn api_path_parameters(path: &str) -> Result<Vec<String>, String> {
    if !path.starts_with('/') || path.contains(['?', '#']) {
        return Err("API path must start with `/` and must not contain a query or fragment".into());
    }
    let mut parameters = Vec::new();
    for segment in path.split('/').skip(1) {
        if segment.is_empty() {
            continue;
        }
        if let Some(name) = segment
            .strip_prefix('{')
            .and_then(|segment| segment.strip_suffix('}'))
        {
            if name.is_empty()
                || !name.chars().enumerate().all(|(index, character)| {
                    if index == 0 {
                        character == '_' || character.is_ascii_alphabetic()
                    } else {
                        character == '_' || character.is_ascii_alphanumeric()
                    }
                })
            {
                return Err(format!("invalid API path parameter `{segment}`"));
            }
            if parameters.iter().any(|existing| existing == name) {
                return Err(format!("duplicate API path parameter `{name}`"));
            }
            parameters.push(name.to_owned());
        } else if segment.contains(['{', '}']) {
            return Err(format!("invalid API path segment `{segment}`"));
        }
    }
    Ok(parameters)
}
