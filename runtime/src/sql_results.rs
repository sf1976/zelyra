//! Opt-in SQL materialization; the raw SQL/JSON compatibility view stays intact.
use super::*;
use zelyra_database::QueryColumnType as Kind;

/// Records and table storage rows share one projection description. Explicit
/// records take precedence; relations expose their stored `<name>_id` column.
pub(crate) fn definitions(program: &Program) -> HashMap<String, RecordDef> {
    let aliases = program
        .types
        .iter()
        .map(|alias| (alias.name.clone(), alias.target.clone()))
        .collect::<HashMap<_, _>>();
    let mut records = program
        .records
        .iter()
        .map(|record| (record.name.clone(), record.clone()))
        .collect::<HashMap<_, _>>();
    for table in &program.tables {
        let fields = table
            .columns
            .iter()
            .map(|column| {
                let mut name = column.name.clone();
                let mut ty = expand_type_aliases(&column.ty, &aliases).unwrap_or(Type::Unknown);
                if let Type::Named(target) = &column.ty {
                    if program.tables.iter().any(|table| {
                        table.name == *target
                            || singular_table_type(&table.name).as_ref() == Some(target)
                    }) {
                        name.push_str("_id");
                        ty = Type::Named("Id".into());
                    }
                }
                if !column.required
                    && !column.primary_key
                    && !column.auto
                    && !matches!(ty, Type::Option(_))
                {
                    ty = Type::Option(Box::new(ty));
                }
                RecordField {
                    name,
                    ty,
                    span: column.span,
                }
            })
            .collect::<Vec<_>>();
        for name in std::iter::once(table.name.clone()).chain(singular_table_type(&table.name)) {
            records.entry(name.clone()).or_insert_with(|| RecordDef {
                name,
                is_public: false,
                is_class: false,
                type_params: Vec::new(),
                fields: fields.clone(),
                span: table.span,
            });
        }
    }
    records
}

/// Return table-derived SQL projection records that are not declared records.
/// Execution backends carry these separately so SQL decoding can materialize
/// storage rows without making table projections constructible as ordinary
/// source records.
pub fn projection_records(program: &Program) -> Vec<RecordDef> {
    let declared = program
        .records
        .iter()
        .map(|record| record.name.as_str())
        .collect::<HashSet<_>>();
    let mut records = definitions(program)
        .into_values()
        .filter(|record| !declared.contains(record.name.as_str()))
        .collect::<Vec<_>>();
    records.sort_by(|left, right| left.name.cmp(&right.name));
    records
}

fn scalar_supported(
    ty: &Type,
    records: &HashMap<String, RecordDef>,
    enums: &HashSet<String>,
) -> bool {
    match ty {
        Type::Option(inner) => scalar_supported(inner, records, enums),
        Type::Int
        | Type::UInt
        | Type::Float
        | Type::Decimal
        | Type::Bool
        | Type::String
        | Type::Json
        | Type::Char
        | Type::Date
        | Type::Time
        | Type::Timestamp
        | Type::Duration => true,
        Type::Named(name) if !records.contains_key(name) && !enums.contains(name) => {
            matches!(
                name.as_str(),
                "Id" | "Money" | "Email" | "Url" | "Uuid" | "FileRef"
            )
        }
        _ => false,
    }
}

struct Projection {
    record: RecordDef,
    args: Vec<Type>,
    types: Vec<Type>,
}

fn projection(
    target: &Type,
    records: &HashMap<String, RecordDef>,
    aliases: &HashMap<String, Type>,
    enums: &HashSet<String>,
) -> Result<Option<Projection>, String> {
    let (name, args) = match target {
        Type::Named(name)
            if records.contains_key(name) || !scalar_supported(target, records, enums) =>
        {
            (name, Vec::new())
        }
        Type::Applied { name, args } => (name, args.clone()),
        _ if scalar_supported(target, records, enums) => return Ok(None),
        _ => return Err(format!("unsupported SQL row type `{target}`")),
    };
    let record = records
        .get(name)
        .ok_or_else(|| format!("unsupported SQL record type `{name}`"))?;
    if record.is_class {
        return Err("SQL decoding does not support class references".into());
    }
    if record.type_params.len() != args.len() {
        return Err(format!(
            "SQL record `{name}` requires {} type arguments",
            record.type_params.len()
        ));
    }
    let substitutions = record
        .type_params
        .iter()
        .cloned()
        .zip(args.iter().cloned())
        .collect::<HashMap<_, _>>();
    let types = record
        .fields
        .iter()
        .map(|field| {
            let ty =
                expand_type_aliases(&substitute_generic_type(&field.ty, &substitutions), aliases)
                    .map_err(str::to_owned)?;
            if !scalar_supported(&ty, records, enums) {
                return Err(format!(
                    "unsupported SQL field `{}.{}` of type `{ty}`; use a flat scalar projection",
                    record.name, field.name
                ));
            }
            Ok(ty)
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Some(Projection {
        record: record.clone(),
        args,
        types,
    }))
}

pub(crate) fn validate_target(target: &Type, program: &Program) -> Result<(), String> {
    let aliases = program
        .types
        .iter()
        .map(|alias| (alias.name.clone(), alias.target.clone()))
        .collect::<HashMap<_, _>>();
    let target = expand_type_aliases(target, &aliases).map_err(str::to_owned)?;
    let row = match &target {
        Type::Array(inner) | Type::Option(inner) => inner.as_ref(),
        _ => &target,
    };
    projection(
        row,
        &definitions(program),
        &aliases,
        &program
            .enums
            .iter()
            .map(|definition| definition.name.clone())
            .collect(),
    )
    .map(|_| ())
}

impl Interpreter<'_, '_> {
    pub(crate) fn eval_sql_decode(
        &mut self,
        type_args: &[Type],
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if type_args.len() != 1 || args.len() != 1 {
            return Err(self.runtime_error(
                span,
                "sql_decode expects one type argument and one SQL expression",
            ));
        }
        let target = self.active_type_substitutions.last().map_or_else(
            || type_args[0].clone(),
            |active| substitute_generic_type(&type_args[0], active),
        );
        if let ExprKind::Sql { query, .. } = &args[0].kind {
            if zelyra_database::sql::analyze_table_access(query, &[]).0 != Some("SELECT") {
                return Err(self.runtime_error(args[0].span, "sql_decode requires a SELECT query"));
            }
        }
        let rows = self.eval(&args[0], env)?;
        self.decode_sql_rows(rows, &target, span)
    }

    pub(crate) fn decode_sql_rows(
        &self,
        value: Value,
        target: &Type,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let enums = self
            .program_context
            .enum_definitions
            .keys()
            .cloned()
            .collect::<HashSet<_>>();
        decode_native_sql_rows(
            value,
            target,
            &self.program_context.sql_record_definitions,
            &self.program_context.type_aliases,
            &enums,
            span,
        )
    }
}

pub fn decode_native_sql_rows(
    value: Value,
    target: &Type,
    records: &HashMap<String, RecordDef>,
    aliases: &HashMap<String, Type>,
    enums: &HashSet<String>,
    span: Span,
) -> Result<Value, RuntimeError> {
    let Value::Rows {
        columns,
        rows,
        metadata,
        ..
    } = value
    else {
        return Err(sql_decode_error(
            span,
            "sql_decode requires native SQL rows",
        ));
    };
    let SqlRowsMetadata {
        nulls,
        column_types,
    } = *metadata;
    let target =
        expand_type_aliases(target, aliases).map_err(|message| sql_decode_error(span, message))?;
    let row_type = match &target {
        Type::Array(inner) | Type::Option(inner) => inner.as_ref(),
        _ => &target,
    };
    let record = projection(row_type, records, aliases, enums)
        .map_err(|message| sql_decode_error(span, format!("SQL decoding: {message}")))?;
    if columns.len() != column_types.len()
        || rows.len() != nulls.len()
        || rows
            .iter()
            .zip(&nulls)
            .any(|(row, nulls)| row.len() != columns.len() || nulls.len() != columns.len())
    {
        return Err(sql_decode_error(
            span,
            "SQL decoding: inconsistent native result metadata",
        ));
    }
    let mut seen = HashSet::new();
    for column in &columns {
        if !seen.insert(column) {
            return Err(sql_decode_error(
                span,
                format!("SQL decoding: duplicate column `{column}`; use explicit aliases"),
            ));
        }
    }
    let expected = match &record {
        Some(Projection { record, .. }) => record
            .fields
            .iter()
            .map(|field| field.name.clone())
            .collect::<Vec<_>>(),
        None if columns.len() == 1 => columns.clone(),
        None => {
            return Err(sql_decode_error(
                span,
                format!(
                    "SQL decoding: scalar row requires exactly one column, found {}",
                    columns.len()
                ),
            ))
        }
    };
    let indices = expected
        .iter()
        .map(|name| {
            columns
                .iter()
                .position(|column| column == name)
                .ok_or_else(|| {
                    sql_decode_error(span, format!("SQL decoding: missing column `{name}`"))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(column) = columns.iter().find(|column| !expected.contains(column)) {
        return Err(sql_decode_error(
            span,
            format!("SQL decoding: unexpected column `{column}`"),
        ));
    }
    let types = match &record {
        Some(Projection { types, .. }) => types.clone(),
        None => vec![row_type.clone()],
    };
    // Check native column families even for empty arrays/optional results.
    for (index, ty) in indices.iter().zip(&types) {
        if !column_compatible(ty, column_types[*index]) {
            return Err(sql_decode_error(
                span,
                format!(
                    "SQL decoding: column `{}` has native type {:?}, expected `{ty}`",
                    columns[*index], column_types[*index]
                ),
            ));
        }
    }
    match &target {
        Type::Array(_) => {}
        Type::Option(_) if rows.len() > 1 => {
            return Err(sql_decode_error(
                span,
                format!(
                    "SQL decoding: optional result requires at most one row, found {}",
                    rows.len()
                ),
            ))
        }
        Type::Option(_) => {}
        _ if rows.len() != 1 => {
            return Err(sql_decode_error(
                span,
                format!(
                    "SQL decoding: result requires exactly one row, found {}",
                    rows.len()
                ),
            ))
        }
        _ => {}
    }
    let values = rows.iter().zip(&nulls).enumerate().map(|(row_index, (row, nulls))| {
            let fields = indices.iter().zip(&types).map(|(index, ty)| {
                decode_cell(&row[*index], nulls[*index], ty).ok_or_else(|| sql_decode_error(span, format!("SQL decoding: row {}, column `{}` is NULL, invalid, out of range, or inexact for `{ty}`", row_index + 1, columns[*index])))
            }).collect::<Result<Vec<_>, _>>()?;
            match &record {
                Some(Projection { record, args, .. }) => Ok(Value::Object { type_name: record.name.clone(), type_args: args.clone(), fields: expected.iter().cloned().zip(fields).collect() }),
                None => Ok(fields.into_iter().next().expect("scalar projection has one column")),
            }
        }).collect::<Result<Vec<_>, RuntimeError>>()?;
    Ok(match target {
        Type::Array(_) => Value::Array(values),
        Type::Option(_) => Value::Option(values.into_iter().next().map(Box::new)),
        _ => values
            .into_iter()
            .next()
            .expect("required cardinality was checked"),
    })
}

fn sql_decode_error(span: Span, message: impl Into<String>) -> RuntimeError {
    RuntimeError {
        message: message.into(),
        span,
    }
}

fn column_compatible(target: &Type, kind: Kind) -> bool {
    match target {
        Type::Option(inner) => kind == Kind::Null || column_compatible(inner, kind),
        Type::Int | Type::UInt | Type::Decimal | Type::Duration => {
            matches!(kind, Kind::Integer | Kind::Decimal)
        }
        Type::Float => matches!(kind, Kind::Integer | Kind::Decimal | Kind::Float),
        Type::Bool => matches!(kind, Kind::Bool | Kind::Integer),
        Type::String | Type::Char => kind == Kind::Text,
        Type::Json => matches!(kind, Kind::Json | Kind::Text),
        Type::Date => kind == Kind::Date,
        Type::Time => kind == Kind::Time,
        Type::Timestamp => kind == Kind::Timestamp,
        Type::Named(name) if name == "Id" || name == "Money" => {
            matches!(kind, Kind::Integer | Kind::Decimal)
        }
        Type::Named(name) if matches!(name.as_str(), "Email" | "Url" | "Uuid" | "FileRef") => {
            kind == Kind::Text
        }
        _ => false,
    }
}

fn integer_text(text: &str) -> Option<&str> {
    match text.split_once('.') {
        Some((integer, fraction))
            if !fraction.is_empty() && fraction.bytes().all(|byte| byte == b'0') =>
        {
            Some(integer)
        }
        Some(_) => None,
        None => Some(text),
    }
}

fn decode_cell(text: &str, null: bool, ty: &Type) -> Option<Value> {
    if let Type::Option(inner) = ty {
        return if null {
            Some(Value::Option(None))
        } else {
            decode_cell(text, false, inner).map(|value| Value::Option(Some(Box::new(value))))
        };
    }
    if null {
        return None;
    }
    match ty {
        Type::Int => integer_text(text)?.parse().ok().map(Value::Int),
        Type::UInt => integer_text(text)?.parse().ok().map(Value::UInt),
        Type::Float => text
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(Value::Float),
        Type::Decimal => parse_decimal(text).map(Value::Decimal),
        Type::Bool => match text {
            "true" | "1" => Some(Value::Bool(true)),
            "false" | "0" => Some(Value::Bool(false)),
            _ => None,
        },
        Type::String => Some(Value::String(text.into())),
        Type::Json => serde_json::from_str(text).ok().map(Value::Json),
        Type::Char => {
            let mut chars = text.chars();
            let character = chars.next()?;
            chars.next().is_none().then_some(Value::Char(character))
        }
        Type::Date => valid_date(text).then(|| Value::Date(text.into())),
        Type::Time => valid_time(text).then(|| Value::Time(text.into())),
        Type::Timestamp => sql_timestamp(text).map(Value::Timestamp),
        Type::Duration => integer_text(text)?.parse().ok().map(Value::Duration),
        Type::Named(name) if name == "Id" => integer_text(text)?.parse().ok().map(Value::Int),
        Type::Named(name) if name == "Money" => parse_money(text).map(Value::Decimal),
        Type::Named(name) if matches!(name.as_str(), "Email" | "Url" | "Uuid" | "FileRef") => {
            Some(Value::String(text.into()))
        }
        _ => None,
    }
}

pub(crate) fn parse_money(text: &str) -> Option<i128> {
    let value = parse_decimal(text)?;
    // Match the declared DECIMAL(19,4) storage contract without rounding.
    (value % 100_000 == 0 && value.unsigned_abs() < 1_000_000_000_000_000_000_000_000)
        .then_some(value)
}

fn sql_timestamp(text: &str) -> Option<i64> {
    use zelyra_ast::temporal::parse_timestamp_input;
    if text.contains('T') {
        return parse_timestamp_input(text);
    }
    if valid_date(text) {
        return parse_timestamp_input(&format!("{text}T00:00:00Z"));
    }
    let (date, clock) = text.split_once(' ')?;
    // The native MySQL driver emits six fractional digits; discarded digits
    // must all be zero. Never round a sub-millisecond value.
    let clock = if let Some((time, fraction)) = clock.split_once('.') {
        if fraction.len() > 6
            || fraction.is_empty()
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
            || fraction.bytes().skip(3).any(|byte| byte != b'0')
        {
            return None;
        }
        format!("{time}.{}", &fraction[..fraction.len().min(3)])
    } else {
        clock.to_owned()
    };
    parse_timestamp_input(&format!("{date}T{clock}Z"))
}
