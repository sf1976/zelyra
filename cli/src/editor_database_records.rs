use super::{editor_database_schemas, EditorState};
use serde_json::{json, Map, Value};
use zelyra_database::{Backend, Column, QueryValue};

const MAX_MUTATION_FIELDS: usize = 40;

pub(super) fn mutate(
    state: &EditorState,
    request: &Value,
) -> Result<(u16, &'static str, String), (u16, String)> {
    if request.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err((
            400,
            "explicit confirmation is required for database changes".into(),
        ));
    }
    let action = required_string(request, "action")?;
    let table_name = required_string(request, "table")?;
    if !matches!(action, "create" | "update" | "delete") {
        return Err((400, "action must be create, update, or delete".into()));
    }

    let (declared, live, _) = editor_database_schemas(state)?;
    let declared_table = declared
        .tables
        .iter()
        .find(|table| table.name == table_name)
        .ok_or((
            403,
            "Studio can only change tables declared in the Zelyra project".into(),
        ))?;
    let live_table = live
        .tables
        .iter()
        .find(|table| table.name == table_name)
        .ok_or((
            409,
            "the declared table is not present in the live database".into(),
        ))?;
    let backend = declared.backend();
    if backend == Backend::Sqlite {
        return Err((
            501,
            "record changes currently support MariaDB and PostgreSQL projects".into(),
        ));
    }
    let db_name = declared
        .database
        .as_ref()
        .map(|database| database.name.as_str());
    let database_url = super::database_url_for_project(&state.root, db_name)
        .map_err(|_| (422, "the project environment file could not be read".into()))?
        .ok_or((422, "database connection is not configured".into()))?;

    let declared_columns = declared_table
        .columns
        .iter()
        .map(|column| (column.name.as_str(), column))
        .collect::<std::collections::HashMap<_, _>>();
    let live_columns = live_table
        .columns
        .iter()
        .map(|column| (column.name.as_str(), column))
        .collect::<std::collections::HashMap<_, _>>();
    let writable = live_table
        .columns
        .iter()
        .filter(|column| declared_columns.contains_key(column.name.as_str()) && !column.auto)
        .collect::<Vec<_>>();
    let primary = live_table
        .columns
        .iter()
        .filter(|column| column.primary_key)
        .collect::<Vec<_>>();
    if primary.is_empty() && action != "create" {
        return Err((409, "updates and deletes require a primary key".into()));
    }

    let quote = |name: &str| match backend {
        Backend::Postgres => format!("\"{}\"", name.replace('"', "\"\"")),
        Backend::MariaDb => format!("`{}`", name.replace('`', "``")),
        Backend::Sqlite => unreachable!(),
    };
    let mut params: Vec<(String, QueryValue)> = Vec::new();
    let sql = match action {
        "create" => {
            let values = request
                .get("values")
                .and_then(Value::as_object)
                .ok_or((400, "values must be an object".into()))?;
            if values.len() > MAX_MUTATION_FIELDS {
                return Err((400, "too many fields".into()));
            }
            for column in &writable {
                if !column.nullable
                    && column.default.is_none()
                    && !values.contains_key(&column.name)
                {
                    return Err((422, format!("required field `{}` is missing", column.name)));
                }
            }
            if values.is_empty() {
                return Err((422, "provide at least one value to create a record".into()));
            }
            validate_value_fields(values, &writable, &mut params, &live_columns)?;
            let columns = values.keys().map(|name| quote(name)).collect::<Vec<_>>();
            let bindings = values
                .keys()
                .enumerate()
                .map(|(index, _)| format!(":zelyra_field_{index}"))
                .collect::<Vec<_>>();
            format!(
                "INSERT INTO {} ({}) VALUES ({})",
                quote(table_name),
                columns.join(", "),
                bindings.join(", ")
            )
        }
        "update" => {
            let values = request
                .get("values")
                .and_then(Value::as_object)
                .ok_or((400, "values must be an object".into()))?;
            if values.is_empty() || values.len() > MAX_MUTATION_FIELDS {
                return Err((400, "update needs between 1 and 40 fields".into()));
            }
            let updateable = writable
                .iter()
                .copied()
                .filter(|column| !column.primary_key)
                .collect::<Vec<_>>();
            validate_value_fields(values, &updateable, &mut params, &live_columns)?;
            let key = request
                .get("key")
                .and_then(Value::as_object)
                .ok_or((400, "key must contain the primary key values".into()))?;
            let where_sql = append_key_predicates(key, &primary, &quote, &mut params)?;
            let assignments = values
                .keys()
                .enumerate()
                .map(|(index, name)| format!("{} = :zelyra_field_{index}", quote(name)))
                .collect::<Vec<_>>();
            format!(
                "UPDATE {} SET {} WHERE {}",
                quote(table_name),
                assignments.join(", "),
                where_sql
            )
        }
        "delete" => {
            let key = request
                .get("key")
                .and_then(Value::as_object)
                .ok_or((400, "key must contain the primary key values".into()))?;
            let where_sql = append_key_predicates(key, &primary, &quote, &mut params)?;
            format!("DELETE FROM {} WHERE {}", quote(table_name), where_sql)
        }
        _ => unreachable!(),
    };

    zelyra_database::execute_transactional_query(&database_url, &sql, params).map_err(|_| {
        (
            502,
            "the database rejected the change; refresh the table and check its constraints".into(),
        )
    })?;
    let response = json!({
        "success": true,
        "action": action,
        "table": table_name,
        "message": match action { "create" => "Record created.", "update" => "Record updated.", _ => "Record deleted." },
    });
    Ok((200, "application/json; charset=utf-8", response.to_string()))
}

fn input_query_value(value: &Value, column: &Column) -> Option<QueryValue> {
    if value.is_null() {
        return Some(QueryValue::Null);
    }
    let sql_type = column.sql_type.to_ascii_lowercase();
    if sql_type.contains("json") {
        return serde_json::to_string(value).ok().map(QueryValue::Json);
    }
    if value.is_object() || value.is_array() {
        return None;
    }
    if sql_type.contains("bool") {
        return match value {
            Value::Bool(value) => Some(QueryValue::Bool(*value)),
            Value::String(value) if matches!(value.as_str(), "1" | "true" | "TRUE") => {
                Some(QueryValue::Bool(true))
            }
            Value::String(value) if matches!(value.as_str(), "0" | "false" | "FALSE") => {
                Some(QueryValue::Bool(false))
            }
            _ => None,
        };
    }
    if sql_type == "date" || sql_type.starts_with("date ") {
        return match value {
            Value::String(value) => Some(QueryValue::Date(value.clone())),
            _ => None,
        };
    }
    if sql_type.starts_with("time") && !sql_type.contains("zone") {
        return match value {
            Value::String(value) => Some(QueryValue::Time(value.clone())),
            _ => None,
        };
    }
    if sql_type.contains("timestamp") && sql_type.contains("time zone") {
        return match value {
            Value::String(value) => chrono::DateTime::parse_from_rfc3339(value)
                .ok()
                .map(|value| QueryValue::Timestamp(value.timestamp_millis())),
            _ => None,
        };
    }
    if sql_type.contains("int") || sql_type.contains("serial") {
        return match value {
            Value::Number(value) => value
                .as_i64()
                .map(QueryValue::Int)
                .or_else(|| value.as_u64().map(QueryValue::UInt)),
            Value::String(value) => value
                .parse::<i64>()
                .map(QueryValue::Int)
                .ok()
                .or_else(|| value.parse::<u64>().ok().map(QueryValue::UInt)),
            _ => None,
        };
    }
    if sql_type.contains("decimal") || sql_type.contains("numeric") {
        return match value {
            Value::Number(value) => Some(QueryValue::Decimal(value.to_string())),
            Value::String(value) if !value.trim().is_empty() => {
                Some(QueryValue::Decimal(value.clone()))
            }
            _ => None,
        };
    }
    if sql_type.contains("float") || sql_type.contains("double") || sql_type == "real" {
        return match value {
            Value::Number(value) => value
                .as_f64()
                .filter(|value| value.is_finite())
                .map(QueryValue::Float),
            Value::String(value) => value
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(QueryValue::Float),
            _ => None,
        };
    }
    match value {
        Value::Bool(value) => Some(QueryValue::Bool(*value)),
        Value::Number(value) => value
            .as_i64()
            .map(QueryValue::Int)
            .or_else(|| value.as_u64().map(QueryValue::UInt))
            .or_else(|| {
                value
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .map(QueryValue::Float)
            }),
        Value::String(value) => Some(QueryValue::String(value.clone())),
        _ => None,
    }
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, (u16, String)> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or((400, format!("{key} is required")))
}

fn validate_value_fields(
    values: &Map<String, Value>,
    writable: &[&Column],
    params: &mut Vec<(String, QueryValue)>,
    live_columns: &std::collections::HashMap<&str, &Column>,
) -> Result<(), (u16, String)> {
    for (index, (name, value)) in values.iter().enumerate() {
        if !writable.iter().any(|column| column.name == *name) {
            return Err((
                403,
                format!("field `{name}` is not a writable declared column"),
            ));
        }
        let column = live_columns
            .get(name.as_str())
            .ok_or((409, format!("field `{name}` is not live")))?;
        if value.is_null() && !column.nullable {
            return Err((422, format!("field `{name}` does not allow NULL")));
        }
        if (value.is_object() || value.is_array())
            && !column.sql_type.to_ascii_lowercase().contains("json")
        {
            return Err((
                400,
                format!("field `{name}` must be a scalar value or NULL"),
            ));
        }
        let raw = input_query_value(value, column)
            .ok_or((400, format!("field `{name}` has an unsupported value")))?;
        params.push((format!("zelyra_field_{index}"), raw));
    }
    Ok(())
}

fn append_key_predicates(
    key: &Map<String, Value>,
    primary: &[&Column],
    quote: &impl Fn(&str) -> String,
    params: &mut Vec<(String, QueryValue)>,
) -> Result<String, (u16, String)> {
    if key.len() != primary.len() {
        return Err((400, "key must exactly match the table primary key".into()));
    }
    let mut predicates = Vec::new();
    for (index, column) in primary.iter().enumerate() {
        let value = key
            .get(&column.name)
            .ok_or((400, "key is missing a primary-key field".into()))?;
        if value.is_null() || value.is_object() || value.is_array() {
            return Err((400, "primary-key values must be non-null scalars".into()));
        }
        let value = input_query_value(value, column)
            .filter(|value| !matches!(value, QueryValue::Null | QueryValue::Json(_)))
            .ok_or((400, "primary-key value is unsupported".into()))?;
        let parameter = format!("zelyra_key_{index}");
        params.push((parameter.clone(), value));
        predicates.push(format!("{} = :{parameter}", quote(&column.name)));
    }
    Ok(predicates.join(" AND "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_key_predicate_binds_each_composite_key_field() {
        let columns = [
            Column {
                name: "tenant_id".into(),
                sql_type: "BIGINT".into(),
                nullable: false,
                primary_key: true,
                auto: false,
                unique: false,
                default: None,
            },
            Column {
                name: "customer_id".into(),
                sql_type: "BIGINT".into(),
                nullable: false,
                primary_key: true,
                auto: false,
                unique: false,
                default: None,
            },
        ];
        let primary = columns.iter().collect::<Vec<_>>();
        let key = serde_json::from_value::<Map<String, Value>>(
            json!({"tenant_id": 8, "customer_id": 12}),
        )
        .unwrap();
        let mut params = Vec::new();
        let where_sql =
            append_key_predicates(&key, &primary, &|name| format!("`{name}`"), &mut params)
                .unwrap();
        assert_eq!(
            where_sql,
            "`tenant_id` = :zelyra_key_0 AND `customer_id` = :zelyra_key_1"
        );
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn primary_key_predicate_rejects_incomplete_or_null_keys() {
        let column = Column {
            name: "id".into(),
            sql_type: "BIGINT".into(),
            nullable: false,
            primary_key: true,
            auto: true,
            unique: false,
            default: None,
        };
        let primary = [&column];
        let incomplete = serde_json::from_value::<Map<String, Value>>(json!({})).unwrap();
        assert!(
            append_key_predicates(&incomplete, &primary, &|name| name.into(), &mut Vec::new())
                .is_err()
        );
        let null_key = serde_json::from_value::<Map<String, Value>>(json!({"id": null})).unwrap();
        assert!(
            append_key_predicates(&null_key, &primary, &|name| name.into(), &mut Vec::new())
                .is_err()
        );
    }

    #[test]
    fn json_columns_bind_structured_json_and_other_columns_reject_it() {
        let json_column = Column {
            name: "metadata".into(),
            sql_type: "JSONB".into(),
            nullable: true,
            primary_key: false,
            auto: false,
            unique: false,
            default: None,
        };
        let text_column = Column {
            sql_type: "TEXT".into(),
            ..json_column.clone()
        };
        let object = json!({"active": true, "count": 2});
        let Some(QueryValue::Json(object_json)) = input_query_value(&object, &json_column) else {
            panic!("structured JSON should bind as JSON");
        };
        assert_eq!(object_json, r#"{"active":true,"count":2}"#);
        let Some(QueryValue::Json(string_json)) = input_query_value(&json!("label"), &json_column)
        else {
            panic!("JSON strings should retain JSON encoding");
        };
        assert_eq!(string_json, r#""label""#);
        assert!(input_query_value(&object, &text_column).is_none());
    }
}
