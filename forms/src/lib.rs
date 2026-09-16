use std::collections::{HashMap, HashSet};
use zelyra_ast::{FormDef, Span, TableDef, Type};
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
    for form in &program.forms {
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
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
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
        let ty = field
            .ty
            .as_ref()
            .or_else(|| table_column.map(|column| &column.ty));
        if let Some(ty) = ty {
            if let Some(message) = type_error(ty, value) {
                errors.push(FieldError {
                    field: field.name.clone(),
                    message: message.into(),
                });
                continue;
            }
        }
        values.insert(field.name.clone(), value.into());
    }
    ValidationResult { values, errors }
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
        Type::Float | Type::Decimal => value
            .parse::<f64>()
            .ok()
            .map(|_| ())
            .is_none()
            .then_some("must be a number"),
        Type::Named(name) if name == "Money" => value
            .parse::<f64>()
            .ok()
            .map(|_| ())
            .is_none()
            .then_some("must be a number"),
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
        Type::Named(name) if name == "Uuid" => (value.len() != 36).then_some("must be a UUID"),
        Type::Option(inner) => type_error(inner, value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zelyra_ast::{ColumnDef, DefaultValue, FormField, TableDef};

    fn field(name: &str, ty: Type, required: bool, max: Option<u32>) -> FormField {
        FormField {
            name: name.into(),
            ty: Some(ty),
            label: None,
            placeholder: None,
            required,
            max,
            widget: None,
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
                max: None,
                widget: None,
                readonly: false,
                span: Span::default(),
            }],
            actions: Vec::new(),
            span: Span::default(),
        };
        let table = TableDef {
            name: "customers".into(),
            columns: vec![ColumnDef {
                name: "name".into(),
                ty: Type::String,
                length: Some(4),
                required: true,
                primary_key: false,
                auto: false,
                unique: false,
                default: Some(DefaultValue::String("".into())),
                span: Span::default(),
            }],
            indexes: Vec::new(),
            uniques: Vec::new(),
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
