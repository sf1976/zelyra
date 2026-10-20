use std::collections::{HashMap, HashSet};
use zelyra_ast::{FormDef, FormField, Span, TableDef, Type};
use zelyra_database::Schema;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormError {
    pub message: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationResult {
    pub values: HashMap<String, String>,
    pub errors: Vec<FieldError>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

pub fn check_program(program: &zelyra_ast::Program, schema: &Schema) -> Result<(), Vec<FormError>> {
    let mut errors = Vec::new();
    for table in &program.tables {
        for column in &table.columns {
            if contains_file_type(&column.ty) {
                errors.push(FormError {
                    message: format!(
                        "`File` is a transient form input and cannot be stored as table column `{}.{}`",
                        table.name, column.name
                    ),
                    span: column.span,
                });
            }
        }
    }
    for record in &program.records {
        for field in &record.fields {
            if contains_file_type(&field.ty) {
                errors.push(FormError {
                    message: format!(
                        "`File` is a transient form input and cannot be used as record field `{}.{}`",
                        record.name, field.name
                    ),
                    span: field.span,
                });
            }
        }
    }
    let mut form_names = HashSet::new();
    for form in &program.forms {
        if !form_names.insert(form.name.clone()) {
            errors.push(FormError {
                message: format!("duplicate form definition `{}`", form.name),
                span: form.span,
            });
        }
        let table = form
            .table
            .as_deref()
            .and_then(|name| schema.tables.iter().find(|table| table.name == name));
        let table_definition = form
            .table
            .as_deref()
            .and_then(|name| program.tables.iter().find(|table| table.name == name));
        if form.table.is_some() && table.is_none() {
            errors.push(FormError {
                message: format!(
                    "form `{}` refers to unknown table",
                    form.table.as_deref().unwrap_or_default()
                ),
                span: form.span,
            });
        }
        if let Some(verify) = &form.verify {
            if form.post_only {
                errors.push(FormError {
                    message: "a form `verify` query requires a GET form route; `post_only` delegates GET handling to a page route".into(),
                    span: verify.span,
                });
            }
            match &verify.kind {
                zelyra_ast::ExprKind::Sql { result_type, query } => {
                    let known_tables = schema
                        .tables
                        .iter()
                        .map(|table| table.name.clone())
                        .collect::<Vec<_>>();
                    let (command, _) =
                        zelyra_database::sql::analyze_table_access(query, &known_tables);
                    if command != Some("SELECT") {
                        errors.push(FormError {
                            message: "form `verify` must be a read-only SELECT query".into(),
                            span: verify.span,
                        });
                    }
                    let route_parameters = form
                        .path
                        .as_deref()
                        .unwrap_or_default()
                        .split('/')
                        .filter_map(|segment| {
                            segment
                                .strip_prefix('{')
                                .and_then(|name| name.strip_suffix('}'))
                        })
                        .map(|name| (name.to_owned(), Type::String))
                        .collect::<HashMap<_, _>>();
                    if let Err(sql_errors) = zelyra_database::sql::check_sql_query(
                        query,
                        result_type,
                        schema,
                        &route_parameters,
                        verify.span,
                    ) {
                        errors.extend(sql_errors.into_iter().map(|error| FormError {
                            message: error.message,
                            span: error.span,
                        }));
                    }
                }
                _ => errors.push(FormError {
                    message: "form `verify` requires a SQL query".into(),
                    span: verify.span,
                }),
            }
        }
        let mut names = HashSet::new();
        for field in &form.fields {
            if !names.insert(field.name.clone()) {
                errors.push(FormError {
                    message: format!("duplicate field `{}` in form `{}`", field.name, form.name),
                    span: field.span,
                });
            }
            let schema_column = table.and_then(|table| find_column(table, &field.name));
            let declared_column = table_definition.and_then(|table| {
                table
                    .columns
                    .iter()
                    .find(|column| column.name == field.name)
            });
            if form.table.is_some() && schema_column.is_none() {
                errors.push(FormError {
                    message: format!(
                        "form field `{}` does not exist in the source table",
                        field.name
                    ),
                    span: field.span,
                });
            }
            if field.file_upload.is_some() {
                if form.table.is_some()
                    && !declared_column
                        .is_some_and(|column| is_file_ref_type(&column.ty, &program.types))
                {
                    errors.push(FormError {
                        message: format!(
                            "file field `{}` requires a FileRef column with the same name in a table-backed form",
                            field.name
                        ),
                        span: field.span,
                    });
                }
            } else if declared_column
                .is_some_and(|column| is_file_ref_type(&column.ty, &program.types))
            {
                errors.push(FormError {
                    message: format!(
                        "FileRef column `{}` must use a checked File form field",
                        field.name
                    ),
                    span: field.span,
                });
            }
            if form.table.is_none() && field.ty.is_none() {
                errors.push(FormError {
                    message: format!(
                        "standalone form field `{}` requires an explicit type",
                        field.name
                    ),
                    span: field.span,
                });
            }
            if field.max == Some(0) {
                errors.push(FormError {
                    message: format!("maximum length for `{}` must be positive", field.name),
                    span: field.span,
                });
            }
            check_file_field(field, true, &mut errors);
            let declared_type = declared_column.map(|column| &column.ty);
            validate_constraints(field, declared_type, &mut errors);
        }
        let declared_upload_bytes = form
            .fields
            .iter()
            .filter_map(|field| field.file_upload.as_ref())
            .fold(0_u64, |total, options| {
                total.saturating_add(options.max_size_bytes)
            });
        if declared_upload_bytes > 255 * 1024 * 1024 {
            errors.push(FormError {
                message: format!(
                    "aggregate file size in form `{}` exceeds the 255 MiB request budget",
                    form.name
                ),
                span: form.span,
            });
        }
        let upload_fields = form
            .fields
            .iter()
            .filter(|field| field.file_upload.is_some())
            .collect::<Vec<_>>();
        if !upload_fields.is_empty() {
            if let Some(action) = form.actions.first() {
                let parameters = action
                    .statements
                    .iter()
                    .filter_map(|statement| match statement {
                        zelyra_ast::Stmt::Expr(expression) => match &expression.kind {
                            zelyra_ast::ExprKind::Sql { query, .. } => {
                                zelyra_database::sql_parameter_names(query).ok()
                            }
                            _ => None,
                        },
                        _ => None,
                    })
                    .flatten()
                    .collect::<HashSet<_>>();
                for field in upload_fields {
                    if !parameters.contains(&field.name) {
                        errors.push(FormError {
                            message: format!(
                                "file field `{}` must be stored by the first form action using its named SQL parameter",
                                field.name
                            ),
                            span: field.span,
                        });
                    }
                }
            } else {
                errors.push(FormError {
                    message: format!(
                        "form `{}` declares a file field but has no persistence action",
                        form.name
                    ),
                    span: form.span,
                });
            }
        }
        let mut actions = HashSet::new();
        for action in &form.actions {
            if !actions.insert(action.name.clone()) {
                errors.push(FormError {
                    message: format!("duplicate action `{}` in form `{}`", action.name, form.name),
                    span: action.span,
                });
            }
            for field in &action.fields {
                check_file_field(field, false, &mut errors);
            }
            if action.require_match {
                let first_query = action
                    .statements
                    .first()
                    .and_then(|statement| match statement {
                        zelyra_ast::Stmt::Expr(expression) => match &expression.kind {
                            zelyra_ast::ExprKind::Sql { query, .. } => {
                                Some((query, expression.span))
                            }
                            _ => None,
                        },
                        _ => None,
                    });
                if action.execute_body.is_some() || action.guard.is_some() {
                    errors.push(FormError {
                        message: "`require_match` is supported only for direct SQL form actions"
                            .into(),
                        span: action.span,
                    });
                }
                if let Some((query, span)) = first_query {
                    let known_tables = schema
                        .tables
                        .iter()
                        .map(|table| table.name.clone())
                        .collect::<Vec<_>>();
                    let (command, _) =
                        zelyra_database::sql::analyze_table_access(query, &known_tables);
                    if command != Some("SELECT") {
                        errors.push(FormError {
                            message: "the first statement in a `require_match` action must be a SELECT query".into(),
                            span,
                        });
                    }
                } else {
                    errors.push(FormError {
                        message:
                            "`require_match` requires a SELECT query as the first action statement"
                                .into(),
                        span: action.span,
                    });
                }
            }
        }
    }
    for crud in &program.cruds {
        let table = schema.tables.iter().find(|table| table.name == crud.table);
        if let Some(definition) = program.tables.iter().find(|table| table.name == crud.table) {
            for column in definition
                .columns
                .iter()
                .filter(|column| is_file_ref_type(&column.ty, &program.types))
            {
                errors.push(FormError {
                    message: format!(
                        "CRUD `{}` cannot expose FileRef column `{}` as a text field; file-aware CRUD rendering is not implemented",
                        crud.name, column.name
                    ),
                    span: column.span,
                });
            }
        }
        let mut action_names = HashSet::new();
        for action in &crud.actions {
            if !action_names.insert(action.name.clone()) {
                errors.push(FormError {
                    message: format!(
                        "duplicate action `{}` in CRUD resource `{}`",
                        action.name, crud.name
                    ),
                    span: action.span,
                });
            }
            if action.require_match {
                errors.push(FormError {
                    message: "`require_match` is supported only for standalone form actions".into(),
                    span: action.span,
                });
            }
            let mut field_names = HashSet::new();
            for field in &action.fields {
                if !field_names.insert(field.name.clone()) {
                    errors.push(FormError {
                        message: format!(
                            "duplicate field `{}` in CRUD action `{}`",
                            field.name, action.name
                        ),
                        span: field.span,
                    });
                }
                let schema_column = table.and_then(|table| find_column(table, &field.name));
                if field.ty.is_none() && schema_column.is_none() {
                    errors.push(FormError {
                        message: format!(
                            "CRUD action field `{}` requires an explicit type or a source-table column",
                            field.name
                        ),
                        span: field.span,
                    });
                }
                if field.max == Some(0) {
                    errors.push(FormError {
                        message: format!(
                            "maximum length for CRUD action field `{}` must be positive",
                            field.name
                        ),
                        span: field.span,
                    });
                }
                check_file_field(field, false, &mut errors);
                let declared_type = program
                    .tables
                    .iter()
                    .find(|definition| definition.name == crud.table)
                    .and_then(|definition| {
                        definition
                            .columns
                            .iter()
                            .find(|column| column.name == field.name)
                    })
                    .map(|column| &column.ty);
                validate_constraints(field, declared_type, &mut errors);
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn check_file_field(field: &FormField, can_receive_file: bool, errors: &mut Vec<FormError>) {
    let is_file = matches!(field.ty, Some(Type::Named(ref name)) if name == "File");
    if is_file && !can_receive_file {
        errors.push(FormError {
            message: format!(
                "file field `{}` is not supported in this form action context",
                field.name
            ),
            span: field.span,
        });
        return;
    }
    match (&field.file_upload, is_file) {
        (Some(options), true) => {
            if options.max_size_bytes == 0 || options.max_size_bytes > 100 * 1024 * 1024 {
                errors.push(FormError {
                    message: format!(
                        "maximum file size for `{}` must be between 1 byte and 100 MiB",
                        field.name
                    ),
                    span: field.span,
                });
            }
            if options.allowed_types.is_empty() || options.allowed_types.len() > 8 {
                errors.push(FormError {
                    message: format!(
                        "allowed_types for `{}` must contain between 1 and 8 MIME types",
                        field.name
                    ),
                    span: field.span,
                });
            }
            let mut seen = HashSet::new();
            for mime in &options.allowed_types {
                if !matches!(
                    mime.as_str(),
                    "application/pdf" | "image/png" | "image/jpeg"
                ) {
                    errors.push(FormError {
                        message: format!(
                            "MIME type `{mime}` is not supported for file field `{}`",
                            field.name
                        ),
                        span: field.span,
                    });
                }
                if !seen.insert(mime) {
                    errors.push(FormError {
                        message: format!(
                            "allowed_types for `{}` must not contain duplicates",
                            field.name
                        ),
                        span: field.span,
                    });
                }
            }
            if field.max.is_some()
                || field.min_length.is_some()
                || field.min_value.is_some()
                || field.max_value.is_some()
                || !field.choices.is_empty()
            {
                errors.push(FormError {
                    message: format!(
                        "text, numeric, and choice constraints cannot be used on file field `{}`",
                        field.name
                    ),
                    span: field.span,
                });
            }
        }
        (Some(_), false) => errors.push(FormError {
            message: format!(
                "file upload options require `{}` to have type File",
                field.name
            ),
            span: field.span,
        }),
        (None, true) => errors.push(FormError {
            message: format!(
                "file field `{}` requires max_size and allowed_types",
                field.name
            ),
            span: field.span,
        }),
        (None, false) => {}
    }
}

fn is_file_ref_type(ty: &Type, definitions: &[zelyra_ast::TypeDef]) -> bool {
    fn resolve(
        ty: &Type,
        definitions: &[zelyra_ast::TypeDef],
        visited: &mut HashSet<String>,
    ) -> bool {
        match ty {
            Type::Named(name) if name == "FileRef" => true,
            Type::Option(inner) => resolve(inner, definitions, visited),
            Type::Named(name) if visited.insert(name.clone()) => definitions
                .iter()
                .find(|definition| definition.name == *name)
                .is_some_and(|definition| resolve(&definition.target, definitions, visited)),
            _ => false,
        }
    }
    resolve(ty, definitions, &mut HashSet::new())
}

fn contains_file_type(ty: &Type) -> bool {
    match ty {
        Type::Named(name) => name == "File",
        Type::Option(inner) | Type::Array(inner) | Type::Set(inner) => contains_file_type(inner),
        Type::Result(ok, error) | Type::Map(ok, error) => {
            contains_file_type(ok) || contains_file_type(error)
        }
        Type::Tuple(items) => items.iter().any(contains_file_type),
        Type::Applied { name, args } => name == "File" || args.iter().any(contains_file_type),
        Type::Function {
            params,
            return_type,
            ..
        } => params.iter().any(contains_file_type) || contains_file_type(return_type),
        Type::HttpResult(inner) => contains_file_type(inner),
        _ => false,
    }
}

/// Resolve fixed-point form aliases through the checked storage schema. Domain
/// Money keeps its established four-digit scale; ordinary Decimal keeps nine.
pub fn field_value_type(
    form: &FormDef,
    table: Option<&TableDef>,
    schema: Option<&Schema>,
    field: &FormField,
) -> Option<Type> {
    let storage_column = schema.and_then(|schema| {
        schema
            .tables
            .iter()
            .find(|table| Some(table.name.as_str()) == form.table.as_deref())
            .and_then(|table| find_column(table, &field.name))
    });
    let ty = field.ty.clone().or_else(|| {
        table.and_then(|table| {
            table
                .columns
                .iter()
                .find(|column| column.name == field.name)
                .map(|column| column.ty.clone())
        })
    })?;
    // Storage nullability also applies to ordinary `String`/numeric declarations
    // without an explicit Option. Validation still enforces required form fields.
    let nullable = storage_column
        .map(|column| column.nullable)
        .unwrap_or_else(|| {
            table
                .and_then(|table| {
                    table
                        .columns
                        .iter()
                        .find(|column| column.name == field.name)
                })
                .is_some_and(|column| !column.required && !column.primary_key && !column.auto)
        });
    let ty = if nullable && !matches!(ty, Type::Option(_)) {
        Type::Option(Box::new(ty))
    } else {
        ty
    };
    let inner = match &ty {
        Type::Option(inner) => inner.as_ref(),
        other => other,
    };
    if !matches!(inner, Type::Named(name) if !matches!(name.as_str(), "Money" | "Id" | "Email" | "Url" | "Uuid"))
    {
        return Some(ty);
    }
    let sql_type = schema
        .and_then(|schema| {
            schema
                .tables
                .iter()
                .find(|table| Some(table.name.as_str()) == form.table.as_deref())
        })
        .and_then(|table| find_column(table, &field.name))
        .map(|column| column.sql_type.as_str());
    let resolved = match sql_type {
        Some("DATE") => Type::Date,
        Some("TIME") => Type::Time,
        Some("TIMESTAMP" | "TIMESTAMPTZ") => Type::Timestamp,
        Some(value)
            if value.starts_with("TIMESTAMP(")
                || value.starts_with("TIMESTAMPTZ(")
                || value.starts_with("DATETIME") =>
        {
            Type::Timestamp
        }
        Some("DECIMAL(19,4)" | "NUMERIC(19,4)") => Type::Named("Money".into()),
        Some("DECIMAL(20,0)" | "NUMERIC(20,0)") => Type::UInt,
        Some(value) if value.starts_with("DECIMAL(") || value.starts_with("NUMERIC(") => {
            Type::Decimal
        }
        _ => return Some(ty),
    };
    Some(if matches!(ty, Type::Option(_)) {
        Type::Option(Box::new(resolved))
    } else {
        resolved
    })
}

pub fn validate(
    form: &FormDef,
    table_definition: Option<&TableDef>,
    schema: Option<&Schema>,
    input: &HashMap<String, String>,
) -> ValidationResult {
    validate_with_uploaded_files(form, table_definition, schema, input, &HashSet::new())
}

/// Validate a form while allowing only server-issued opaque references for
/// declared multipart file fields. The caller must provide the exact set of
/// fields whose bytes have already passed storage validation.
pub fn validate_with_uploaded_files(
    form: &FormDef,
    table_definition: Option<&TableDef>,
    schema: Option<&Schema>,
    input: &HashMap<String, String>,
    uploaded_fields: &HashSet<String>,
) -> ValidationResult {
    validate_with_file_values(
        form,
        table_definition,
        schema,
        input,
        uploaded_fields,
        &HashSet::new(),
    )
}

/// Validate a form while accepting server-loaded file references on edit
/// forms. `existing_file_fields` must be derived from the currently authorized
/// database row, never directly from request input. This lets a required file
/// remain unchanged when the browser's file control is left empty.
pub fn validate_with_existing_file_refs(
    form: &FormDef,
    table_definition: Option<&TableDef>,
    schema: Option<&Schema>,
    input: &HashMap<String, String>,
    uploaded_fields: &HashSet<String>,
    existing_file_fields: &HashSet<String>,
) -> ValidationResult {
    validate_with_file_values(
        form,
        table_definition,
        schema,
        input,
        uploaded_fields,
        existing_file_fields,
    )
}

fn validate_with_file_values(
    form: &FormDef,
    table_definition: Option<&TableDef>,
    schema: Option<&Schema>,
    input: &HashMap<String, String>,
    uploaded_fields: &HashSet<String>,
    existing_file_fields: &HashSet<String>,
) -> ValidationResult {
    let mut errors = Vec::new();
    let mut values = HashMap::new();
    let known_fields = form
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect::<HashSet<_>>();

    for name in input.keys() {
        if !known_fields.contains(name.as_str()) {
            errors.push(FieldError {
                field: name.clone(),
                message: "unknown form field".into(),
            });
        }
    }

    for field in &form.fields {
        if field.file_upload.is_some() {
            if uploaded_fields.contains(&field.name) || existing_file_fields.contains(&field.name) {
                match input.get(&field.name) {
                    Some(reference) if is_file_reference(reference) => {
                        values.insert(field.name.clone(), reference.clone());
                    }
                    _ => errors.push(FieldError {
                        field: field.name.clone(),
                        message: "uploaded file reference is invalid".into(),
                    }),
                }
            } else if field.required
                || table_definition.is_some_and(|table| {
                    table.columns.iter().any(|column| {
                        column.name == field.name && (column.required || column.primary_key)
                    })
                })
                || schema.is_some_and(|schema| {
                    form.table.as_deref().is_some_and(|table_name| {
                        schema
                            .tables
                            .iter()
                            .find(|table| table.name == table_name)
                            .is_some_and(|table| {
                                table.columns.iter().any(|column| {
                                    column.name == field.name
                                        && (!column.nullable || column.primary_key)
                                })
                            })
                    })
                })
            {
                errors.push(FieldError {
                    field: field.name.clone(),
                    message: "required file was not uploaded".into(),
                });
            }
            continue;
        }
        if field.readonly && input.contains_key(&field.name) {
            errors.push(FieldError {
                field: field.name.clone(),
                message: "read-only field cannot be submitted".into(),
            });
            continue;
        }
        let value = input.get(&field.name).map(String::as_str).unwrap_or("");
        let table_column = table_definition.and_then(|table| {
            table
                .columns
                .iter()
                .find(|column| column.name == field.name)
        });
        let schema_column = schema.and_then(|schema| {
            form.table.as_deref().and_then(|table_name| {
                schema
                    .tables
                    .iter()
                    .find(|table| table.name == table_name)
                    .and_then(|table| find_column(table, &field.name))
            })
        });
        let required = field.required
            || table_column.is_some_and(|column| column.required || column.primary_key)
            || schema_column.is_some_and(|column| !column.nullable || column.primary_key);
        if required && value.trim().is_empty() {
            errors.push(FieldError {
                field: field.name.clone(),
                message: "value is required".into(),
            });
            continue;
        }
        if value.is_empty() && !required {
            continue;
        }
        let max = field
            .max
            .or_else(|| table_column.and_then(|column| column.length))
            .or_else(|| schema_column.and_then(varchar_length));
        if max.is_some_and(|max| value.chars().count() > max as usize) {
            errors.push(FieldError {
                field: field.name.clone(),
                message: format!("value exceeds maximum length of {}", max.unwrap()),
            });
            continue;
        }
        if field
            .min_length
            .is_some_and(|min| value.chars().count() < min as usize)
        {
            errors.push(FieldError {
                field: field.name.clone(),
                message: format!(
                    "value is shorter than minimum length of {}",
                    field.min_length.unwrap()
                ),
            });
            continue;
        }
        let ty = field_value_type(form, table_definition, schema, field);
        if let Some(ty) = &ty {
            if let Some(message) = type_error(ty, value) {
                errors.push(FieldError {
                    field: field.name.clone(),
                    message: message.into(),
                });
                continue;
            }
        }
        if let Some(ty) = &ty {
            if let Some(message) = numeric_bound_error(
                ty,
                value,
                field.min_value.as_deref(),
                field.max_value.as_deref(),
            ) {
                errors.push(FieldError {
                    field: field.name.clone(),
                    message: message.into(),
                });
                continue;
            }
        }
        if !field.choices.is_empty() && !field.choices.iter().any(|choice| choice == value) {
            errors.push(FieldError {
                field: field.name.clone(),
                message: "value is not an allowed choice".into(),
            });
            continue;
        }
        values.insert(field.name.clone(), value.into());
    }
    ValidationResult { values, errors }
}

fn is_file_reference(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                *byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

fn validate_constraints(
    field: &FormField,
    inferred_type: Option<&Type>,
    errors: &mut Vec<FormError>,
) {
    let span = field.span;
    let ty = field.ty.as_ref().or(inferred_type);
    let numeric = ty.is_some_and(is_numeric_type);
    let text = ty.is_some_and(is_text_type);
    if field.min_length == Some(0) {
        errors.push(FormError {
            message: format!("minimum length for `{}` must be positive", field.name),
            span,
        });
    }
    if field.min_length.is_some() && !text {
        errors.push(FormError {
            message: format!(
                "minimum length for `{}` requires String or Email type",
                field.name
            ),
            span,
        });
    }
    if field.max.is_some() && !text {
        errors.push(FormError {
            message: format!(
                "maximum length for `{}` requires String or Email type",
                field.name
            ),
            span,
        });
    }
    if (field.min_value.is_some() || field.max_value.is_some()) && !numeric {
        errors.push(FormError {
            message: format!("numeric bounds for `{}` require a numeric type", field.name),
            span,
        });
    }
    if !field.choices.is_empty() {
        if field.choices.len() > 64
            || field
                .choices
                .iter()
                .any(|choice| choice.is_empty() || choice.len() > 64)
        {
            errors.push(FormError {
                message: format!("choices for `{}` exceed the supported limit", field.name),
                span,
            });
        }
        if field.choices.iter().collect::<HashSet<_>>().len() != field.choices.len() {
            errors.push(FormError {
                message: format!("choices for `{}` must be unique", field.name),
                span,
            });
        }
        if let Some(ty) = ty {
            for choice in &field.choices {
                if type_error(ty, choice).is_some() {
                    errors.push(FormError {
                        message: format!("choice `{choice}` is invalid for field `{}`", field.name),
                        span,
                    });
                }
            }
        }
    }
    let scale = ty.map(numeric_scale).unwrap_or(0);
    for bound in [&field.min_value, &field.max_value].into_iter().flatten() {
        if parse_numeric_bound(ty.unwrap_or(&Type::Int), bound, scale).is_none() {
            errors.push(FormError {
                message: format!("numeric bound `{bound}` is invalid for `{}`", field.name),
                span,
            });
        }
    }
    if let (Some(min), Some(max)) = (&field.min_value, &field.max_value) {
        if let (Some(min), Some(max)) = (
            parse_numeric_bound(ty.unwrap_or(&Type::Int), min, scale),
            parse_numeric_bound(ty.unwrap_or(&Type::Int), max, scale),
        ) {
            if min > max {
                errors.push(FormError {
                    message: format!("minimum numeric bound exceeds maximum for `{}`", field.name),
                    span,
                });
            }
        }
    }
}

fn is_numeric_type(ty: &Type) -> bool {
    match ty {
        Type::Int | Type::UInt | Type::Float | Type::Decimal | Type::Duration => true,
        Type::Named(name) => name == "Money",
        Type::Option(inner) => is_numeric_type(inner),
        _ => false,
    }
}

fn is_text_type(ty: &Type) -> bool {
    match ty {
        Type::String => true,
        Type::Named(name) => matches!(name.as_str(), "Email" | "Uuid"),
        Type::Option(inner) => is_text_type(inner),
        _ => false,
    }
}

fn numeric_scale(ty: &Type) -> u32 {
    match ty {
        Type::Decimal => 9,
        Type::Named(name) if name == "Money" => 4,
        Type::Option(inner) => numeric_scale(inner),
        _ => 0,
    }
}

fn parse_numeric_bound(ty: &Type, value: &str, scale: u32) -> Option<i128> {
    match ty {
        Type::Int | Type::Duration => value.parse::<i64>().ok().map(i128::from),
        Type::UInt => value.parse::<u64>().ok().map(i128::from),
        Type::Float | Type::Decimal => zelyra_ast::decimal::parse_scaled(value, scale.max(9)),
        Type::Named(name) if name == "Money" => zelyra_ast::decimal::parse_money(value),
        Type::Option(inner) => parse_numeric_bound(inner, value, scale),
        _ => None,
    }
}

fn numeric_bound_error(
    ty: &Type,
    value: &str,
    min: Option<&str>,
    max: Option<&str>,
) -> Option<&'static str> {
    let scalar_ty = match ty {
        Type::Option(inner) => inner.as_ref(),
        other => other,
    };
    if let Type::Float = scalar_ty {
        let actual = value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())?;
        if min
            .and_then(|bound| bound.parse::<f64>().ok())
            .is_some_and(|bound| actual < bound)
        {
            return Some("value is below the minimum");
        }
        if max
            .and_then(|bound| bound.parse::<f64>().ok())
            .is_some_and(|bound| actual > bound)
        {
            return Some("value exceeds the maximum");
        }
        return None;
    }
    let scale = numeric_scale(ty);
    let actual = parse_numeric_bound(ty, value, scale)?;
    if min
        .and_then(|bound| parse_numeric_bound(ty, bound, scale))
        .is_some_and(|bound| actual < bound)
    {
        return Some("value is below the minimum");
    }
    if max
        .and_then(|bound| parse_numeric_bound(ty, bound, scale))
        .is_some_and(|bound| actual > bound)
    {
        return Some("value exceeds the maximum");
    }
    None
}

fn find_column<'a>(
    table: &'a zelyra_database::Table,
    name: &str,
) -> Option<&'a zelyra_database::Column> {
    table
        .columns
        .iter()
        .find(|column| column.name == name || column.name == format!("{name}_id"))
}

fn varchar_length(column: &zelyra_database::Column) -> Option<u32> {
    let upper = column.sql_type.to_ascii_uppercase();
    upper
        .strip_prefix("VARCHAR(")?
        .strip_suffix(')')?
        .parse()
        .ok()
}

fn type_error(ty: &Type, value: &str) -> Option<&'static str> {
    match ty {
        Type::Int => value
            .parse::<i64>()
            .ok()
            .map(|_| ())
            .is_none()
            .then_some("must be an integer"),
        Type::UInt => value
            .parse::<u64>()
            .ok()
            .map(|_| ())
            .is_none()
            .then_some("must be an unsigned integer"),
        Type::Float => value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(|_| ())
            .is_none()
            .then_some("must be a number"),
        Type::Decimal => zelyra_ast::decimal::parse_scaled(value, 9)
            .is_none()
            .then_some("must be an exact Decimal with at most 9 fractional digits"),
        Type::Named(name) if name == "Money" => {
            zelyra_ast::decimal::parse_money(value).is_none().then_some(
                "must be an exact Money amount with at most 15 integer and 4 fractional digits",
            )
        }
        Type::Date => (!zelyra_ast::temporal::valid_date(value)).then_some("must be a valid Date in YYYY-MM-DD format"),
        Type::Time => (!zelyra_ast::temporal::valid_time(value)).then_some("must be a valid Time in HH:MM:SS format"),
        Type::Timestamp => zelyra_ast::temporal::parse_timestamp_input(value).is_none().then_some("must be a Timestamp in UTC ISO format or Unix milliseconds within years 0001 through 9999"),
        Type::Duration => value.parse::<i64>().is_err().then_some("must be an integer duration in milliseconds"),
        Type::Bool => {
            (!matches!(value, "true" | "false" | "1" | "0")).then_some("must be true or false")
        }
        Type::Named(name) if name == "Email" => (!value.contains('@')
            || value.chars().any(char::is_whitespace))
        .then_some("must be a valid email address"),
        Type::Named(name) if name == "Url" => (!value.starts_with("http://")
            && !value.starts_with("https://"))
        .then_some("must be a valid HTTP URL"),
        Type::Named(name) if name == "Id" => value
            .parse::<i64>()
            .ok()
            .map(|_| ())
            .is_none()
            .then_some("must be an integer identifier"),
        Type::Named(name) if name == "Uuid" => (!valid_uuid(value))
            .then_some("must be a UUID in canonical 8-4-4-4-12 hexadecimal form"),
        Type::Option(inner) => type_error(inner, value),
        _ => None,
    }
}

fn valid_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36
        || ![8, 13, 18, 23]
            .into_iter()
            .all(|index| bytes[index] == b'-')
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| [8, 13, 18, 23].contains(&index) || byte.is_ascii_hexdigit())
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use zelyra_ast::{ColumnDef, DefaultValue, TableDef};
    use zelyra_database::build_schema;

    fn field(name: &str, ty: Type, required: bool, max: Option<u32>) -> FormField {
        FormField {
            name: name.into(),
            ty: Some(ty),
            label: None,
            placeholder: None,
            required,
            min_length: None,
            max,
            min_value: None,
            max_value: None,
            choices: Vec::new(),
            widget: None,
            appearance: None,
            file_upload: None,
            readonly: false,
            span: Span::default(),
        }
    }

    fn form() -> FormDef {
        FormDef {
            name: "CustomerCreate".into(),
            table: None,
            path: None,
            post_only: false,
            verify: None,
            fields: vec![
                field("name", Type::String, true, Some(5)),
                field("email", Type::Named("Email".into()), true, None),
            ],
            actions: Vec::new(),
            span: Span::default(),
        }
    }

    #[test]
    fn validates_uuid_format_like_laravel_default_rule() {
        for uuid in [
            "f47ac10b-58cc-11cf-a447-001122334455",
            "550e8400-e29b-41d4-a716-446655440000",
            "01901234-5678-7abc-8def-0123456789ab",
            // Laravel's unversioned `uuid` rule checks canonical hex groups,
            // not the version or RFC variant bits.
            "550e8400-e29b-01d4-c716-446655440000",
        ] {
            assert_eq!(type_error(&Type::Named("Uuid".into()), uuid), None);
        }
        for uuid in [
            "550e8400e29b41d4a716446655440000",
            "550e8400-e29b-41d4-a716-44665544000g",
            "550e8400-e29b-41d4-a716-44665544000",
        ] {
            assert_eq!(
                type_error(&Type::Named("Uuid".into()), uuid),
                Some("must be a UUID in canonical 8-4-4-4-12 hexadecimal form")
            );
        }
    }

    #[test]
    fn temporal_form_fields_reject_invalid_calendar_and_clock_values() {
        for value in [
            "",
            "0000-01-01",
            "1900-02-29",
            "2026-02-30",
            "2026-13-01",
            "2026-1-01",
            "2026-01-00",
        ] {
            assert!(
                type_error(&Type::Date, value).is_some(),
                "accepted Date: {value}"
            );
        }
        for value in ["24:00:00", "12:60:00", "23:59:60", "1:02:03", "12:00:00Z"] {
            assert!(
                type_error(&Type::Time, value).is_some(),
                "accepted Time: {value}"
            );
        }
        assert!(type_error(&Type::Date, "2000-02-29").is_none());
        assert!(type_error(&Type::Time, "23:59:59").is_none());
    }

    #[test]
    fn rejects_duplicate_form_names_in_the_linked_program() {
        let program = zelyra_ast::Program {
            imports: Vec::new(),
            databases: Vec::new(),
            tables: Vec::new(),
            types: Vec::new(),
            records: Vec::new(),
            enums: Vec::new(),
            views: Vec::new(),
            components: Vec::new(),
            pages: Vec::new(),
            tableviews: Vec::new(),
            forms: vec![form(), form()],
            cruds: Vec::new(),
            auth: Vec::new(),
            apis: Vec::new(),
            commands: Vec::new(),
            schedules: Vec::new(),
            functions: Vec::new(),
        };
        let schema = build_schema(&program).unwrap();
        let errors = check_program(&program, &schema).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message == "duplicate form definition `CustomerCreate`"));
    }

    #[test]
    fn form_verify_and_require_match_validate_route_parameters_and_select_order() {
        let source = r#"
            table suppressions {
                id: Id primary auto
                token_hash: String(64) required
                unsubscribed_at: Timestamp?
            }
            form Unsubscribe {
                route: "/csv-mailer/abmelden/{token}"
                verify sql<Int[]> {
                    SELECT id FROM suppressions WHERE token_hash = :token
                }
                action unsubscribe {
                    require_match
                    sql<Int[]> { SELECT id FROM suppressions WHERE token_hash = :token }
                    sql { UPDATE suppressions SET unsubscribed_at = CURRENT_TIMESTAMP WHERE token_hash = :token }
                }
            }
            fn main() {}
        "#;
        let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        check_program(&program, &schema).unwrap();
        zelyra_database::sql::check_program(&program, &schema).unwrap();

        let invalid = source.replace(
            "sql<Int[]> { SELECT id FROM suppressions WHERE token_hash = :token }",
            "sql<Int[]> { UPDATE suppressions SET unsubscribed_at = CURRENT_TIMESTAMP WHERE token_hash = :token }",
        );
        let program = zelyra_parser::parse(&zelyra_lexer::lex(&invalid).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        let errors = check_program(&program, &schema).unwrap_err();
        assert!(errors.iter().any(|error| {
            error
                .message
                .contains("first statement in a `require_match` action must be a SELECT")
        }));
    }

    #[test]
    fn checks_file_field_size_mime_allowlist_and_form_action_context() {
        let mut valid = form();
        valid.fields = vec![FormField {
            name: "document".into(),
            ty: Some(Type::Named("File".into())),
            label: None,
            placeholder: None,
            required: true,
            min_length: None,
            max: None,
            min_value: None,
            max_value: None,
            choices: Vec::new(),
            widget: None,
            appearance: None,
            file_upload: Some(zelyra_ast::FileUploadOptions {
                max_size_bytes: 10 * 1024 * 1024,
                allowed_types: vec!["application/pdf".into()],
            }),
            readonly: false,
            span: Span::default(),
        }];
        valid.actions.push(zelyra_ast::FormAction {
            name: "save".into(),
            label: None,
            icon: None,
            confirm: None,
            confirm_page: None,
            success_page: None,
            error_page: None,
            fields: Vec::new(),
            requires_auth: false,
            permissions: Vec::new(),
            capabilities: Vec::new(),
            require_match: false,
            statements: vec![zelyra_ast::Stmt::Expr(zelyra_ast::Expr {
                kind: zelyra_ast::ExprKind::Sql {
                    result_type: Type::Unit,
                    query: "INSERT INTO uploads (document) VALUES (:document)".into(),
                },
                span: Span::default(),
            })],
            execute_body: None,
            guard: None,
            success: None,
            redirect: None,
            span: Span::default(),
        });
        let valid_program = zelyra_ast::Program {
            imports: Vec::new(),
            databases: Vec::new(),
            tables: Vec::new(),
            types: Vec::new(),
            records: Vec::new(),
            enums: Vec::new(),
            views: Vec::new(),
            components: Vec::new(),
            pages: Vec::new(),
            tableviews: Vec::new(),
            forms: vec![valid.clone()],
            cruds: Vec::new(),
            auth: Vec::new(),
            apis: Vec::new(),
            commands: Vec::new(),
            schedules: Vec::new(),
            functions: Vec::new(),
        };
        check_program(&valid_program, &build_schema(&valid_program).unwrap())
            .expect("declared supported file metadata is accepted in a form");

        let mut no_persistence = valid_program.clone();
        no_persistence.forms[0].actions.clear();
        let errors = check_program(&no_persistence, &build_schema(&no_persistence).unwrap())
            .expect_err("uploaded files must not be published without a persistence action");
        assert!(errors
            .iter()
            .any(|error| error.message.contains("has no persistence action")));

        let mut unreferenced = valid_program.clone();
        unreferenced.forms[0].actions[0].statements.clear();
        let errors = check_program(&unreferenced, &build_schema(&unreferenced).unwrap())
            .expect_err("the persistence action must store the uploaded reference");
        assert!(errors.iter().any(|error| error
            .message
            .contains("must be stored by the first form action")));

        valid.fields[0].file_upload.as_mut().unwrap().allowed_types[0] =
            "application/x-executable".into();
        let invalid_mime = zelyra_ast::Program {
            forms: vec![valid.clone()],
            ..valid_program.clone()
        };
        let errors = check_program(&invalid_mime, &build_schema(&invalid_mime).unwrap())
            .expect_err("unsupported MIME types must be rejected");
        assert!(errors
            .iter()
            .any(|error| error.message.contains("not supported")));

        valid.fields[0].file_upload.as_mut().unwrap().allowed_types[0] = "application/pdf".into();
        valid.actions.push(zelyra_ast::FormAction {
            name: "submit".into(),
            label: None,
            icon: None,
            confirm: None,
            confirm_page: None,
            success_page: None,
            error_page: None,
            fields: vec![valid.fields[0].clone()],
            requires_auth: false,
            permissions: Vec::new(),
            capabilities: Vec::new(),
            require_match: false,
            statements: Vec::new(),
            execute_body: None,
            guard: None,
            success: None,
            redirect: None,
            span: Span::default(),
        });
        let action_file = zelyra_ast::Program {
            forms: vec![valid],
            ..valid_program
        };
        let errors = check_program(&action_file, &build_schema(&action_file).unwrap())
            .expect_err("file uploads are not action parameters");
        assert!(errors.iter().any(|error| error
            .message
            .contains("not supported in this form action context")));

        let mut zero_limit_form = form();
        zero_limit_form.fields = vec![FormField {
            name: "document".into(),
            ty: Some(Type::Named("File".into())),
            label: None,
            placeholder: None,
            required: false,
            min_length: None,
            max: None,
            min_value: None,
            max_value: None,
            choices: Vec::new(),
            widget: None,
            appearance: None,
            file_upload: Some(zelyra_ast::FileUploadOptions {
                max_size_bytes: 0,
                allowed_types: vec!["application/pdf".into()],
            }),
            readonly: false,
            span: Span::default(),
        }];
        let zero_limit = zelyra_ast::Program {
            imports: Vec::new(),
            databases: Vec::new(),
            tables: Vec::new(),
            types: Vec::new(),
            records: Vec::new(),
            enums: Vec::new(),
            views: Vec::new(),
            components: Vec::new(),
            pages: Vec::new(),
            tableviews: Vec::new(),
            forms: vec![zero_limit_form],
            cruds: Vec::new(),
            auth: Vec::new(),
            apis: Vec::new(),
            commands: Vec::new(),
            schedules: Vec::new(),
            functions: Vec::new(),
        };
        let errors = check_program(&zero_limit, &build_schema(&zero_limit).unwrap())
            .expect_err("zero-byte file limits must be rejected");
        assert!(errors
            .iter()
            .any(|error| error.message.contains("must be between 1 byte and 100 MiB")));
    }

    #[test]
    fn table_backed_file_inputs_require_a_matching_file_ref_column() {
        let mut document = field("document", Type::Named("File".into()), false, None);
        document.file_upload = Some(zelyra_ast::FileUploadOptions {
            max_size_bytes: 10 * 1024 * 1024,
            allowed_types: vec!["application/pdf".into()],
        });
        let program = zelyra_ast::Program {
            imports: Vec::new(),
            databases: Vec::new(),
            tables: vec![TableDef {
                name: "documents".into(),
                validations: Vec::new(),
                indexes: Vec::new(),
                uniques: Vec::new(),
                access: zelyra_ast::TableAccessDef::default(),
                tenant_scope: None,
                columns: vec![zelyra_ast::ColumnDef {
                    name: "document".into(),
                    ty: Type::String,
                    length: Some(255),
                    storage_name: None,
                    decimal_precision: None,
                    auto_sequence: None,
                    required: false,
                    primary_key: false,
                    auto: false,
                    unique: false,
                    unique_within_tenant: false,
                    default: None,
                    references: None,
                    on_delete: None,
                    span: Span::default(),
                }],
                span: Span::default(),
            }],
            types: Vec::new(),
            records: Vec::new(),
            enums: Vec::new(),
            views: Vec::new(),
            components: Vec::new(),
            pages: Vec::new(),
            tableviews: Vec::new(),
            forms: vec![FormDef {
                name: "DocumentCreate".into(),
                table: Some("documents".into()),
                path: None,
                post_only: false,
                verify: None,
                fields: vec![document],
                actions: vec![zelyra_ast::FormAction {
                    name: "save".into(),
                    label: None,
                    icon: None,
                    confirm: None,
                    confirm_page: None,
                    success_page: None,
                    error_page: None,
                    fields: Vec::new(),
                    requires_auth: false,
                    permissions: Vec::new(),
                    capabilities: Vec::new(),
                    require_match: false,
                    statements: vec![zelyra_ast::Stmt::Expr(zelyra_ast::Expr {
                        kind: zelyra_ast::ExprKind::Sql {
                            result_type: Type::Unit,
                            query: "INSERT INTO documents (document) VALUES (:document)".into(),
                        },
                        span: Span::default(),
                    })],
                    execute_body: None,
                    guard: None,
                    success: None,
                    redirect: None,
                    span: Span::default(),
                }],
                span: Span::default(),
            }],
            cruds: Vec::new(),
            auth: Vec::new(),
            apis: Vec::new(),
            commands: Vec::new(),
            schedules: Vec::new(),
            functions: Vec::new(),
        };
        let schema = zelyra_database::build_schema(&program).expect("valid table schema");
        let errors = check_program(&program, &schema)
            .expect_err("a text column cannot receive an uploaded file reference");
        assert!(errors
            .iter()
            .any(|error| { error.message.contains("requires a FileRef column") }));

        let mut file_ref_program = program.clone();
        file_ref_program.tables[0].columns[0].ty =
            Type::Option(Box::new(Type::Named("FileRef".into())));
        let schema = zelyra_database::build_schema(&file_ref_program).expect("FileRef schema");
        check_program(&file_ref_program, &schema)
            .expect("a bounded File input may target its matching optional FileRef column");

        file_ref_program.types.push(zelyra_ast::TypeDef {
            name: "DocumentRef".into(),
            is_public: false,
            target: Type::Named("FileRef".into()),
            span: Span::default(),
        });
        file_ref_program.tables[0].columns[0].ty =
            Type::Option(Box::new(Type::Named("DocumentRef".into())));
        let schema =
            zelyra_database::build_schema(&file_ref_program).expect("aliased FileRef schema");
        check_program(&file_ref_program, &schema)
            .expect("aliases of FileRef retain file-field handling");

        file_ref_program.forms[0].fields[0].file_upload = None;
        let schema = zelyra_database::build_schema(&file_ref_program).expect("FileRef schema");
        let errors = check_program(&file_ref_program, &schema)
            .expect_err("FileRef values cannot be submitted as plain text");
        assert!(errors
            .iter()
            .any(|error| error.message.contains("must use a checked File form field")));
    }

    #[test]
    fn generated_crud_does_not_render_file_refs_as_text_controls() {
        let program = zelyra_parser::parse(
            &zelyra_lexer::lex(
                "table documents { id: Id primary auto document: FileRef? } crud Document -> documents {} fn main() {}",
            )
            .unwrap(),
        )
        .unwrap();
        let schema = build_schema(&program).unwrap();
        let errors = check_program(&program, &schema)
            .expect_err("automatic CRUD cannot expose an opaque FileRef as text yet");
        assert!(errors.iter().any(|error| {
            error
                .message
                .contains("cannot expose FileRef column `document` as a text field")
        }));
    }

    #[test]
    fn rejects_aggregate_file_size_beyond_http_hard_limit() {
        let files = (0..3)
            .map(|index| FormField {
                name: format!("document_{index}"),
                ty: Some(Type::Named("File".into())),
                label: None,
                placeholder: None,
                required: false,
                min_length: None,
                max: None,
                min_value: None,
                max_value: None,
                choices: Vec::new(),
                widget: None,
                appearance: None,
                file_upload: Some(zelyra_ast::FileUploadOptions {
                    max_size_bytes: 100 * 1024 * 1024,
                    allowed_types: vec!["application/pdf".into()],
                }),
                readonly: false,
                span: Span::default(),
            })
            .collect();
        let program = zelyra_ast::Program {
            imports: Vec::new(),
            databases: Vec::new(),
            tables: Vec::new(),
            types: Vec::new(),
            records: Vec::new(),
            enums: Vec::new(),
            views: Vec::new(),
            components: Vec::new(),
            pages: Vec::new(),
            tableviews: Vec::new(),
            forms: vec![FormDef {
                name: "BulkUpload".into(),
                table: None,
                path: None,
                post_only: false,
                verify: None,
                fields: files,
                actions: Vec::new(),
                span: Span::default(),
            }],
            cruds: Vec::new(),
            auth: Vec::new(),
            apis: Vec::new(),
            commands: Vec::new(),
            schedules: Vec::new(),
            functions: Vec::new(),
        };
        let errors = check_program(&program, &build_schema(&program).unwrap())
            .expect_err("aggregate uploads larger than the request ceiling must fail");
        assert!(errors
            .iter()
            .any(|error| error.message.contains("aggregate file size")));
    }

    #[test]
    fn string_validation_never_accepts_a_path_as_an_uploaded_file_reference() {
        let mut upload_form = form();
        upload_form.fields = vec![FormField {
            name: "document".into(),
            ty: Some(Type::Named("File".into())),
            label: None,
            placeholder: None,
            required: true,
            min_length: None,
            max: None,
            min_value: None,
            max_value: None,
            choices: Vec::new(),
            widget: None,
            appearance: None,
            file_upload: Some(zelyra_ast::FileUploadOptions {
                max_size_bytes: 1024,
                allowed_types: vec!["application/pdf".into()],
            }),
            readonly: false,
            span: Span::default(),
        }];
        let submitted_path = HashMap::from([("document".into(), "../../invoice.pdf".into())]);
        let result = validate(&upload_form, None, None, &submitted_path);
        assert!(!result.is_valid());
        assert_eq!(result.errors[0].field, "document");
        assert!(result.errors[0]
            .message
            .contains("required file was not uploaded"));
    }

    #[test]
    fn money_and_decimal_inputs_reject_lossy_or_nonfinite_values() {
        let money = Type::Named("Money".into());
        for value in ["0", "0.01", "-12.34", "999999999999999.9999"] {
            assert!(
                type_error(&money, value).is_none(),
                "rejected valid Money: {value}"
            );
        }
        for value in [
            "NaN",
            "inf",
            "-inf",
            "1e3",
            "1.00001",
            "1000000000000000.00",
        ] {
            assert!(
                type_error(&money, value).is_some(),
                "accepted invalid Money: {value}"
            );
        }
        for value in ["0.000000001", "9007199254740993.123456789"] {
            assert!(type_error(&Type::Decimal, value).is_none());
        }
        for value in ["NaN", "inf", "1e-999", "1.0000000001"] {
            assert!(
                type_error(&Type::Decimal, value).is_some(),
                "accepted invalid Decimal: {value}"
            );
        }
        for value in ["NaN", "inf", "-inf", "1e999"] {
            assert!(
                type_error(&Type::Float, value).is_some(),
                "accepted nonfinite Float: {value}"
            );
        }
    }

    #[test]
    fn validates_required_length_and_email() {
        let input = HashMap::from([
            (String::from("name"), String::from("too long")),
            (String::from("email"), String::from("invalid")),
        ]);
        let result = validate(&form(), None, None, &input);
        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn accepts_only_server_marked_opaque_file_references() {
        let mut form = form();
        form.fields = vec![FormField {
            name: "document".into(),
            ty: Some(Type::Named("File".into())),
            label: None,
            placeholder: None,
            required: true,
            min_length: None,
            max: None,
            min_value: None,
            max_value: None,
            choices: Vec::new(),
            widget: None,
            appearance: None,
            file_upload: Some(zelyra_ast::FileUploadOptions {
                max_size_bytes: 1024,
                allowed_types: vec!["application/pdf".into()],
            }),
            readonly: false,
            span: Span::default(),
        }];
        let file_ref = "bca8c39e-d920-4e2a-8f45-2fcdcb3b61a2";
        let input = HashMap::from([("document".to_owned(), file_ref.to_owned())]);
        let uploaded = HashSet::from(["document".to_owned()]);
        let result = validate_with_uploaded_files(&form, None, None, &input, &uploaded);
        assert!(result.is_valid());
        assert_eq!(result.values.get("document"), Some(&file_ref.to_owned()));

        let forged = validate_with_uploaded_files(&form, None, None, &input, &HashSet::new());
        assert!(!forged.is_valid());
        let existing = validate_with_existing_file_refs(
            &form,
            None,
            None,
            &input,
            &HashSet::new(),
            &HashSet::from(["document".to_owned()]),
        );
        assert!(existing.is_valid());
        assert_eq!(existing.values.get("document"), Some(&file_ref.to_owned()));
        assert!(!validate_with_existing_file_refs(
            &form,
            None,
            None,
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::from(["document".to_owned()]),
        )
        .is_valid());
        assert!(!validate_with_uploaded_files(
            &form,
            None,
            None,
            &HashMap::from([("document".to_owned(), "../../outside".to_owned())]),
            &uploaded,
        )
        .is_valid());
    }

    #[test]
    fn enforces_text_lengths_and_exact_numeric_bounds_on_server() {
        let mut name = field("name", Type::String, false, None);
        name.min_length = Some(3);
        let mut amount = field("amount", Type::Decimal, false, None);
        amount.min_value = Some("0.10".into());
        amount.max_value = Some("10.00".into());
        let bounded = FormDef {
            name: "Bounded".into(),
            table: None,
            path: None,
            post_only: false,
            verify: None,
            fields: vec![name, amount],
            actions: Vec::new(),
            span: Span::default(),
        };

        for (name, amount) in [("ab", "1"), ("abc", "0.09"), ("abc", "10.01")] {
            let input = HashMap::from([
                ("name".into(), name.into()),
                ("amount".into(), amount.into()),
            ]);
            assert!(!validate(&bounded, None, None, &input).is_valid());
        }
        let valid = HashMap::from([
            ("name".into(), "abc".into()),
            ("amount".into(), "10.00".into()),
        ]);
        assert!(validate(&bounded, None, None, &valid).is_valid());
    }

    #[test]
    fn form_constraints_must_match_the_declared_type_and_order() {
        let mut invalid = field("name", Type::String, false, None);
        invalid.min_value = Some("1".into());
        let mut inverted = field("amount", Type::Decimal, false, None);
        inverted.min_value = Some("2".into());
        inverted.max_value = Some("1".into());
        let program = zelyra_ast::Program {
            imports: Vec::new(),
            databases: Vec::new(),
            tables: Vec::new(),
            types: Vec::new(),
            records: Vec::new(),
            enums: Vec::new(),
            views: Vec::new(),
            components: Vec::new(),
            pages: Vec::new(),
            tableviews: Vec::new(),
            forms: vec![FormDef {
                name: "Invalid".into(),
                table: None,
                path: None,
                post_only: false,
                verify: None,
                fields: vec![invalid, inverted],
                actions: Vec::new(),
                span: Span::default(),
            }],
            cruds: Vec::new(),
            auth: Vec::new(),
            apis: Vec::new(),
            commands: Vec::new(),
            schedules: Vec::new(),
            functions: Vec::new(),
        };
        let errors = check_program(&program, &build_schema(&program).unwrap()).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("require a numeric type")));
        assert!(errors
            .iter()
            .any(|error| error.message.contains("exceeds maximum")));
    }

    #[test]
    fn static_choices_are_type_checked_and_enforced_on_submission() {
        let mut status = field("status", Type::String, true, None);
        status.choices = vec!["draft".into(), "paid".into()];
        let form = FormDef {
            name: "Invoice".into(),
            table: None,
            path: None,
            post_only: false,
            verify: None,
            fields: vec![status],
            actions: Vec::new(),
            span: Span::default(),
        };
        let accepted = HashMap::from([("status".into(), "paid".into())]);
        assert!(validate(&form, None, None, &accepted).is_valid());
        let rejected = HashMap::from([("status".into(), "padi".into())]);
        let result = validate(&form, None, None, &rejected);
        assert_eq!(result.errors[0].message, "value is not an allowed choice");

        let mut invalid = field("priority", Type::Int, true, None);
        invalid.choices = vec!["high".into()];
        let program = zelyra_ast::Program {
            imports: Vec::new(),
            databases: Vec::new(),
            tables: Vec::new(),
            types: Vec::new(),
            records: Vec::new(),
            enums: Vec::new(),
            views: Vec::new(),
            components: Vec::new(),
            pages: Vec::new(),
            tableviews: Vec::new(),
            forms: vec![FormDef {
                name: "InvalidChoice".into(),
                table: None,
                path: None,
                post_only: false,
                verify: None,
                fields: vec![invalid],
                actions: Vec::new(),
                span: Span::default(),
            }],
            cruds: Vec::new(),
            auth: Vec::new(),
            apis: Vec::new(),
            commands: Vec::new(),
            schedules: Vec::new(),
            functions: Vec::new(),
        };
        let errors = check_program(&program, &build_schema(&program).unwrap()).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("invalid for field")));
    }

    #[test]
    fn accepts_valid_values_and_rejects_unknown_fields() {
        let input = HashMap::from([
            (String::from("name"), String::from("Anna")),
            (String::from("email"), String::from("anna@example.test")),
            (String::from("admin"), String::from("true")),
        ]);
        let result = validate(&form(), None, None, &input);
        assert!(!result.is_valid());
        assert_eq!(result.values.len(), 2);
        assert_eq!(result.errors[0].field, "admin");
    }

    #[test]
    fn inherits_required_and_length_from_table_schema() {
        let form = FormDef {
            name: "CustomerCreate".into(),
            table: Some("customers".into()),
            path: None,
            post_only: false,
            verify: None,
            fields: vec![FormField {
                name: "name".into(),
                ty: None,
                label: None,
                placeholder: None,
                required: false,
                min_length: None,
                max: None,
                min_value: None,
                max_value: None,
                choices: Vec::new(),
                widget: None,
                appearance: None,
                file_upload: None,
                readonly: false,
                span: Span::default(),
            }],
            actions: Vec::new(),
            span: Span::default(),
        };
        let table = TableDef {
            name: "customers".into(),
            validations: Vec::new(),
            tenant_scope: None,
            columns: vec![ColumnDef {
                name: "name".into(),
                ty: Type::String,
                length: Some(4),
                storage_name: None,
                decimal_precision: None,
                auto_sequence: None,
                required: true,
                primary_key: false,
                auto: false,
                unique: false,
                unique_within_tenant: false,
                default: Some(DefaultValue::String("".into())),
                references: None,
                on_delete: None,
                span: Span::default(),
            }],
            indexes: Vec::new(),
            uniques: Vec::new(),
            access: Default::default(),
            span: Span::default(),
        };
        let input = HashMap::from([(String::from("name"), String::from("Anna!"))]);
        let result = validate(&form, Some(&table), None, &input);
        assert_eq!(
            result.errors[0].message,
            "value exceeds maximum length of 4"
        );
    }

    #[test]
    fn rejects_submitted_readonly_values() {
        let mut form = form();
        form.fields[0].readonly = true;
        let input = HashMap::from([(String::from("name"), String::from("Anna"))]);
        let result = validate(&form, None, None, &input);
        assert_eq!(
            result.errors[0].message,
            "read-only field cannot be submitted"
        );
    }
}
