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
        if form.table.is_some() && table.is_none() {
            errors.push(FormError {
                message: format!(
                    "form `{}` refers to unknown table",
                    form.table.as_deref().unwrap_or_default()
                ),
                span: form.span,
            });
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
            if form.table.is_some() && schema_column.is_none() {
                errors.push(FormError {
                    message: format!(
                        "form field `{}` does not exist in the source table",
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
            let declared_type = form
                .table
                .as_deref()
                .and_then(|name| program.tables.iter().find(|table| table.name == name))
                .and_then(|table| {
                    table
                        .columns
                        .iter()
                        .find(|column| column.name == field.name)
                })
                .map(|column| &column.ty);
            validate_constraints(field, declared_type, &mut errors);
        }
        let mut actions = HashSet::new();
        for action in &form.actions {
            if !actions.insert(action.name.clone()) {
                errors.push(FormError {
                    message: format!("duplicate action `{}` in form `{}`", action.name, form.name),
                    span: action.span,
                });
            }
        }
    }
    for crud in &program.cruds {
        let table = schema.tables.iter().find(|table| table.name == crud.table);
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
            readonly: false,
            span: Span::default(),
        }
    }

    fn form() -> FormDef {
        FormDef {
            name: "CustomerCreate".into(),
            table: None,
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
    fn enforces_text_lengths_and_exact_numeric_bounds_on_server() {
        let mut name = field("name", Type::String, false, None);
        name.min_length = Some(3);
        let mut amount = field("amount", Type::Decimal, false, None);
        amount.min_value = Some("0.10".into());
        amount.max_value = Some("10.00".into());
        let bounded = FormDef {
            name: "Bounded".into(),
            table: None,
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
                required: true,
                primary_key: false,
                auto: false,
                unique: false,
                unique_within_tenant: false,
                default: Some(DefaultValue::String("".into())),
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
