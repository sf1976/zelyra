//! Zelyra static type checking, type inference, and control-flow validation.
//!
//! The checker remains the single source of truth for AST execution, MIR
//! lowering, CLI checks, and source-mapped editor type information.
use super::*;
mod enum_constructors;
mod expressions;
mod flow;
mod patterns;
mod statements;
mod type_relations;
pub(crate) use enum_constructors::instantiated_enum_payload_types;
use flow::{flow_outcomes, FLOW_FALLTHROUGH};
pub(super) use type_relations::{
    compatible, expand_type_aliases, infer_generic_types, is_map_key_type, is_numeric,
    singular_table_type, substitute_generic_type, validate_type,
};

#[derive(Clone)]
struct Variable {
    ty: Type,
    mutable: bool,
}

#[derive(Clone)]
struct FunctionSignature {
    type_params: Vec<String>,
    params: Vec<Type>,
    return_type: Type,
    capabilities: Vec<String>,
}

pub(super) struct Checker<'a> {
    type_aliases: HashMap<String, Type>,
    functions: HashMap<String, FunctionSignature>,
    known_types: std::collections::HashSet<String>,
    generic_arities: HashMap<String, usize>,
    active_type_parameters: HashSet<String>,
    errors: Vec<TypeError>,
    inferred_expression_types: Vec<InferredExpressionType>,
    capture_inferred_expression_types: bool,
    loop_depth: usize,
    parallel_depth: usize,
    transaction_depth: usize,
    _program: &'a Program,
}

/// Lambdas capture outer values as snapshots. Treat those bindings as
/// immutable while checking the lambda so the checker cannot accept a write
/// that the runtime closure cannot propagate back to its creator.
fn lambda_capture_scopes(scopes: &[HashMap<String, Variable>]) -> Vec<HashMap<String, Variable>> {
    let mut captured = scopes.to_vec();
    for scope in &mut captured {
        for variable in scope.values_mut() {
            variable.mutable = false;
        }
    }
    captured
}

pub fn check(program: &Program) -> Result<(), Vec<TypeError>> {
    check_program(program, false).map(|_| ())
}

/// Type-check a program and return source-mapped expression types for
/// consumers such as the MIR builder, Studio, and language services.
///
/// Types are returned only when the whole program passes the existing type
/// checker. The returned list is ordered by source location and contains one
/// entry per expression span.
pub fn check_with_inferred_types(
    program: &Program,
) -> Result<Vec<InferredExpressionType>, Vec<TypeError>> {
    check_program(program, true)
}

fn check_program(
    program: &Program,
    capture_inferred_expression_types: bool,
) -> Result<Vec<InferredExpressionType>, Vec<TypeError>> {
    let mut known_types = [
        "Id",
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
    .collect::<std::collections::HashSet<_>>();
    let mut errors = Vec::new();
    for definition in &program.types {
        if !known_types.insert(definition.name.clone()) {
            errors.push(TypeError {
                code: "E-TYPE-001",
                message: format!("duplicate type `{}`", definition.name),
                span: definition.span,
            });
        }
    }
    for record in &program.records {
        if !known_types.insert(record.name.clone()) {
            errors.push(TypeError {
                code: "E-TYPE-001",
                message: format!("duplicate type `{}`", record.name),
                span: record.span,
            });
        }
    }
    for definition in &program.enums {
        if !known_types.insert(definition.name.clone()) {
            errors.push(TypeError {
                code: "E-TYPE-001",
                message: format!("duplicate type `{}`", definition.name),
                span: definition.span,
            });
        }
    }
    // Schema scalar domains also work in function/decoder annotations. Keep
    // existing explicit definitions authoritative rather than reserving names.
    known_types.extend(["Money", "Email", "Url", "Uuid"].map(String::from));
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
    for table in &program.tables {
        known_types.insert(table.name.clone());
        if let Some(singular) = singular_table_type(&table.name) {
            known_types.insert(singular);
        }
    }
    let aliases: HashMap<_, _> = program
        .types
        .iter()
        .map(|definition| (definition.name.clone(), definition.target.clone()))
        .collect();
    for definition in &program.types {
        if let Err(message) = expand_type_aliases(&Type::Named(definition.name.clone()), &aliases) {
            errors.push(TypeError {
                code: "E-TYPE-ALIAS-001",
                message: message.into(),
                span: definition.span,
            });
        }
    }
    for definition in &program.types {
        validate_type(
            &definition.target,
            &known_types,
            &generic_arities,
            &mut errors,
            definition.span,
        );
    }
    for table in &program.tables {
        let Some(scope) = &table.tenant_scope else {
            continue;
        };
        validate_type(
            &scope.tenant_type,
            &known_types,
            &generic_arities,
            &mut errors,
            scope.span,
        );
        let Some(column) = table
            .columns
            .iter()
            .find(|column| column.name == scope.column)
        else {
            errors.push(TypeError {
                code: "E-TENANT-001",
                message: format!(
                    "tenant scope column `{}` does not exist in table `{}`",
                    scope.column, table.name
                ),
                span: scope.span,
            });
            continue;
        };
        if !column.required {
            errors.push(TypeError {
                code: "E-TENANT-001",
                message: format!(
                    "tenant scope column `{}.{}` must be required",
                    table.name, scope.column
                ),
                span: column.span,
            });
        }
        let scope_type = expand_type_aliases(&scope.tenant_type, &aliases).ok();
        let column_type = expand_type_aliases(&column.ty, &aliases).ok();
        if scope_type != Some(Type::Named("Id".into())) || column_type != scope_type {
            errors.push(TypeError {
                code: "E-TENANT-001",
                message: format!(
                    "tenant scope `{}: {}` must match a nominal Id column",
                    scope.column, scope.tenant_type
                ),
                span: scope.span,
            });
        }
    }
    for record in &program.records {
        let mut fields = HashSet::new();
        let mut record_types = known_types.clone();
        record_types.extend(record.type_params.iter().cloned());
        for parameter in &record.type_params {
            if known_types.contains(parameter) {
                errors.push(TypeError {
                    code: "E-TYPE-GENERIC-001",
                    message: format!(
                        "generic type parameter `{parameter}` shadows a declared type"
                    ),
                    span: record.span,
                });
            }
        }
        for field in &record.fields {
            if !fields.insert(field.name.clone()) {
                errors.push(TypeError {
                    code: "E-TYPE-001",
                    message: format!(
                        "duplicate field `{}` in record `{}`",
                        field.name, record.name
                    ),
                    span: field.span,
                });
            }
            validate_type(
                &field.ty,
                &record_types,
                &generic_arities,
                &mut errors,
                field.span,
            );
        }
    }
    for definition in &program.enums {
        let mut enum_types = known_types.clone();
        enum_types.extend(definition.type_params.iter().cloned());
        for parameter in &definition.type_params {
            if known_types.contains(parameter) {
                errors.push(TypeError {
                    code: "E-TYPE-GENERIC-001",
                    message: format!(
                        "generic type parameter `{parameter}` shadows a declared type"
                    ),
                    span: definition.span,
                });
            }
        }
        for (_, payload_types) in &definition.variant_payloads {
            for ty in payload_types {
                validate_type(
                    ty,
                    &enum_types,
                    &generic_arities,
                    &mut errors,
                    definition.span,
                );
            }
        }
    }
    let mut functions = HashMap::new();
    for function in &program.functions {
        if functions.contains_key(&function.name) {
            errors.push(TypeError {
                code: "E-TYPE-001",
                message: format!("duplicate function `{}`", function.name),
                span: function.span,
            });
        } else {
            functions.insert(
                function.name.clone(),
                FunctionSignature {
                    type_params: function.type_params.clone(),
                    params: function
                        .params
                        .iter()
                        .map(|param| param.ty.clone())
                        .collect(),
                    return_type: function.return_type.clone().unwrap_or(Type::Unknown),
                    capabilities: function.capabilities.clone(),
                },
            );
        }
    }
    let mut command_names = HashSet::new();
    for command in &program.commands {
        if !command_names.insert(command.name.clone()) {
            errors.push(TypeError {
                code: "E-TASK-001",
                message: format!("duplicate command `{}`", command.name),
                span: command.span,
            });
        }
        match program
            .functions
            .iter()
            .find(|function| function.name == command.handler)
        {
            None => errors.push(TypeError {
                code: "E-TASK-001",
                message: format!(
                    "command `{}` references unknown handler function `{}`",
                    command.name, command.handler
                ),
                span: command.span,
            }),
            Some(function) if !function.type_params.is_empty() => errors.push(TypeError {
                code: "E-TASK-001",
                message: format!("command handler `{}` must not be generic", command.handler),
                span: command.span,
            }),
            Some(_) => {}
        }
    }
    let mut schedule_names = HashSet::new();
    for schedule in &program.schedules {
        if !schedule_names.insert(schedule.name.clone()) {
            errors.push(TypeError {
                code: "E-TASK-001",
                message: format!("duplicate schedule `{}`", schedule.name),
                span: schedule.span,
            });
        }
        let scheduled_command = program
            .commands
            .iter()
            .find(|command| command.name == schedule.command);
        if scheduled_command.is_none() {
            errors.push(TypeError {
                code: "E-TASK-001",
                message: format!(
                    "schedule `{}` references unknown command `{}`",
                    schedule.name, schedule.command
                ),
                span: schedule.span,
            });
        } else if let Some(command) = scheduled_command {
            if let Some(handler) = program
                .functions
                .iter()
                .find(|function| function.name == command.handler)
            {
                if !handler.params.is_empty() {
                    errors.push(TypeError {
                        code: "E-TASK-001",
                        message: format!(
                            "scheduled command `{}` must have a zero-argument handler",
                            command.name
                        ),
                        span: schedule.span,
                    });
                }
            }
        }
        if !valid_five_field_cron(&schedule.cron) {
            errors.push(TypeError {
                code: "E-TASK-001",
                message: format!(
                    "schedule `{}` has an invalid cron expression; expected five numeric fields",
                    schedule.name
                ),
                span: schedule.span,
            });
        }
        if schedule
            .timezone
            .as_ref()
            .is_some_and(|timezone| !valid_timezone_name(timezone))
        {
            errors.push(TypeError {
                code: "E-TASK-001",
                message: format!("schedule `{}` has an invalid timezone name", schedule.name),
                span: schedule.span,
            });
        }
    }
    if !functions.contains_key("main") {
        errors.push(TypeError {
            code: "E-TYPE-001",
            message: "program must define `fn main()`".into(),
            span: Span::default(),
        });
    }
    let mut checker = Checker {
        type_aliases: program
            .types
            .iter()
            .map(|definition| (definition.name.clone(), definition.target.clone()))
            .collect(),
        functions,
        known_types,
        generic_arities,
        active_type_parameters: HashSet::new(),
        errors,
        inferred_expression_types: Vec::new(),
        capture_inferred_expression_types,
        loop_depth: 0,
        parallel_depth: 0,
        transaction_depth: 0,
        _program: program,
    };
    for function in &program.functions {
        checker.check_function(function);
    }
    let (validation_functions, _) = zelyra_ast::build_table_validation_functions(program);
    for function in &validation_functions {
        checker.check_function(function);
    }
    if checker.errors.is_empty() {
        checker.inferred_expression_types.sort_by_key(|inferred| {
            (
                inferred.span.source_id,
                inferred.span.start,
                inferred.span.end,
                inferred.span.line,
                inferred.span.column,
            )
        });
        checker
            .inferred_expression_types
            .dedup_by(|current, previous| {
                current.span == previous.span && current.ty == previous.ty
            });
        Ok(checker.inferred_expression_types)
    } else {
        Err(checker.errors)
    }
}

fn valid_five_field_cron(expression: &str) -> bool {
    let fields = expression.split_ascii_whitespace().collect::<Vec<_>>();
    if fields.len() != 5 {
        return false;
    }
    let bounds = [(0, 59), (0, 23), (1, 31), (1, 12), (0, 7)];
    fields
        .iter()
        .zip(bounds)
        .all(|(field, (minimum, maximum))| valid_cron_field(field, minimum, maximum))
}

fn valid_cron_field(field: &str, minimum: u32, maximum: u32) -> bool {
    !field.is_empty()
        && field.split(',').all(|part| {
            let mut step_parts = part.split('/');
            let base = step_parts.next().unwrap_or_default();
            let step = step_parts.next();
            if step_parts.next().is_some()
                || step.is_some_and(|value| match value.parse::<u32>() {
                    Ok(value) => value == 0 || value > maximum.saturating_add(1),
                    Err(_) => true,
                })
            {
                return false;
            }
            if base == "*" {
                return true;
            }
            let mut range = base.split('-');
            let start = range.next().and_then(|value| value.parse::<u32>().ok());
            let end = range
                .next()
                .map(|value| value.parse::<u32>().ok())
                .unwrap_or(start);
            if range.next().is_some() {
                return false;
            }
            matches!((start, end), (Some(start), Some(end))
                if start >= minimum && end <= maximum && start <= end)
        })
}

fn valid_timezone_name(timezone: &str) -> bool {
    timezone == "UTC"
        || (!timezone.is_empty()
            && timezone.len() <= 128
            && timezone.split('/').all(|part| {
                !part.is_empty()
                    && part.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'+')
                    })
            }))
}

impl<'a> Checker<'a> {
    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.error_with_code("E-TYPE-001", span, message);
    }
    fn error_with_code(&mut self, code: &'static str, span: Span, message: impl Into<String>) {
        self.errors.push(TypeError {
            code,
            message: message.into(),
            span,
        });
    }

    fn lookup<'b>(
        &self,
        scopes: &'b [HashMap<String, Variable>],
        name: &str,
    ) -> Option<&'b Variable> {
        scopes.iter().rev().find_map(|scope| scope.get(name))
    }
    fn check_function(&mut self, function: &Function) {
        let previous_type_parameters = std::mem::replace(
            &mut self.active_type_parameters,
            function.type_params.iter().cloned().collect(),
        );
        for parameter in &function.type_params {
            if self.known_types.contains(parameter) {
                self.error_with_code(
                    "E-TYPE-GENERIC-001",
                    function.span,
                    format!("generic type parameter `{parameter}` shadows a declared type"),
                );
            }
        }
        for parameter in &function.params {
            self.check_type(&parameter.ty, parameter.span);
        }
        if let Some(return_type) = &function.return_type {
            self.check_type(return_type, function.span);
        }
        let mut scopes = vec![HashMap::new()];
        for parameter in &function.params {
            if scopes[0]
                .insert(
                    parameter.name.clone(),
                    Variable {
                        ty: parameter.ty.clone(),
                        mutable: false,
                    },
                )
                .is_some()
            {
                self.error(
                    parameter.span,
                    format!("duplicate parameter `{}`", parameter.name),
                );
            }
        }
        let expected = function.return_type.clone().unwrap_or(Type::Unknown);
        for contract in &function.requires {
            let actual = self.check_expr(contract, &scopes);
            self.expect_type(&Type::Bool, &actual, contract.span);
        }
        self.check_block(&function.body, &mut scopes, &expected);
        if !function.ensures.is_empty() {
            scopes[0].insert(
                "result".into(),
                Variable {
                    ty: function.return_type.clone().unwrap_or(Type::Unit),
                    mutable: false,
                },
            );
            for contract in &function.ensures {
                let actual = self.check_expr(contract, &scopes);
                self.expect_type(&Type::Bool, &actual, contract.span);
            }
        }
        if function
            .return_type
            .as_ref()
            .is_some_and(|return_type| *return_type != Type::Unit)
            && flow_outcomes(&function.body, self._program) & FLOW_FALLTHROUGH != 0
        {
            let return_type = function.return_type.as_ref().expect("checked above");
            self.error_with_code(
                "E-TYPE-RETURN-001",
                function.body.span,
                format!(
                    "function `{}` can reach its end without returning `{return_type}`",
                    function.name
                ),
            );
        }
        self.active_type_parameters = previous_type_parameters;
    }
    fn check_block(
        &mut self,
        block: &Block,
        scopes: &mut Vec<HashMap<String, Variable>>,
        expected: &Type,
    ) {
        scopes.push(HashMap::new());
        for statement in &block.statements {
            self.check_statement(statement, scopes, expected);
        }
        scopes.pop();
    }
    fn check_type(&mut self, ty: &Type, span: Span) {
        match ty {
            Type::Named(name)
                if !self.known_types.contains(name)
                    && !self.active_type_parameters.contains(name) =>
            {
                self.error(span, format!("unknown type `{name}`"));
            }
            Type::Named(name)
                if self
                    .generic_arities
                    .get(name)
                    .is_some_and(|arity| *arity > 0) =>
            {
                let arity = self.generic_arities[name];
                self.error_with_code(
                    "E-TYPE-GENERIC-001",
                    span,
                    format!("generic type `{name}` requires {arity} type argument(s)"),
                );
            }
            Type::Applied { name, args } => {
                if !self.known_types.contains(name) {
                    self.error(span, format!("unknown type `{name}`"));
                }
                if let Some(arity) = self.generic_arities.get(name) {
                    if *arity != args.len() {
                        self.error_with_code(
                            "E-TYPE-GENERIC-001",
                            span,
                            format!(
                                "generic type `{name}` expects {arity} type argument(s), found {}",
                                args.len()
                            ),
                        );
                    }
                } else {
                    self.error_with_code(
                        "E-TYPE-GENERIC-001",
                        span,
                        format!("type `{name}` is not generic and does not accept type arguments"),
                    );
                }
                for argument in args {
                    self.check_type(argument, span);
                }
            }
            Type::Option(inner) => self.check_type(inner, span),
            Type::Result(ok, error) => {
                self.check_type(ok, span);
                self.check_type(error, span);
            }
            Type::Array(inner) => self.check_type(inner, span),
            Type::Set(inner) => {
                self.check_type(inner, span);
                if !matches!(inner.as_ref(), Type::Unknown) && !is_map_key_type(inner) {
                    self.error(
                        span,
                        format!("Set elements must be scalar comparable types; found `{inner}`"),
                    );
                }
            }
            Type::Tuple(items) => {
                for item in items {
                    self.check_type(item, span);
                }
            }
            Type::Map(key, value) => {
                self.check_type(key, span);
                self.check_type(value, span);
                if !matches!(key.as_ref(), Type::Unknown) && !is_map_key_type(key) {
                    self.error(
                        span,
                        format!(
                            "Map keys must be Int, UInt, String, Bool, Char, or Timestamp; found `{key}`"
                        ),
                    );
                }
            }
            Type::HttpResult(inner) => self.check_type(inner, span),
            Type::Function {
                params,
                return_type,
                capabilities,
            } => {
                for parameter in params {
                    self.check_type(parameter, span);
                }
                self.check_type(return_type, span);
                for capability in capabilities {
                    if !KNOWN_CAPABILITIES.contains(&capability.as_str()) {
                        self.error_with_code(
                            "E-TYPE-FUNCTION-001",
                            span,
                            format!("unknown capability `{capability}` in function type"),
                        );
                    }
                }
            }
            _ => {}
        }
    }
    fn normalize_alias(&mut self, ty: &Type, span: Span) -> Type {
        match expand_type_aliases(ty, &self.type_aliases) {
            Ok(ty) => ty,
            Err(message) => {
                self.error_with_code("E-TYPE-ALIAS-001", span, message);
                Type::Unknown
            }
        }
    }

    fn expect_type(&mut self, expected: &Type, actual: &Type, span: Span) {
        let resolved_expected = self.normalize_alias(expected, span);
        let resolved_actual = self.normalize_alias(actual, span);
        if !compatible(&resolved_expected, &resolved_actual) {
            self.error(
                span,
                format!("type mismatch: expected `{expected}`, found `{actual}`"),
            );
        }
    }
}
