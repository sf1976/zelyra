//! Explicit, read-only Laravel database schema inspection.
//!
//! Only connection settings are read from the project's `.env`; the generated
//! evidence contains schema metadata and never connection credentials or rows.

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use url::Url;

const MAX_DOTENV_BYTES: u64 = 64 * 1024;

pub(super) fn inspect(project: &Path) -> Result<Value, String> {
    let root = fs::canonicalize(project)
        .map_err(|_| "Laravel project directory does not exist or is not readable".to_owned())?;
    let env_path = root.join(".env");
    let metadata = fs::symlink_metadata(&env_path)
        .map_err(|_| "`--inspect-database` requires a readable project `.env` file".to_owned())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("refusing to read a symlink or non-file Laravel `.env`".into());
    }
    if metadata.len() > MAX_DOTENV_BYTES {
        return Err("Laravel `.env` exceeds the 64 KiB import limit".into());
    }
    let canonical_env = fs::canonicalize(&env_path)
        .map_err(|_| "could not resolve Laravel `.env` safely".to_owned())?;
    if !canonical_env.starts_with(&root) {
        return Err("Laravel `.env` resolves outside the project directory".into());
    }
    let contents = fs::read_to_string(&canonical_env)
        .map_err(|_| "Laravel `.env` is not readable UTF-8".to_owned())?;
    let values = parse_connection_values(&contents)?;
    let connection_url = connection_url(&values)?;
    let scheme = Url::parse(&connection_url)
        .map_err(|_| "Laravel database connection settings are malformed".to_owned())?
        .scheme()
        .to_ascii_lowercase();

    let (schema, connection_resolution) = match scheme.as_str() {
        "mysql" | "mariadb" => match zelyra_database::inspect_mariadb(&connection_url) {
            Ok(schema) => (schema, "configured_host"),
            Err(error) if is_host_resolution_error(&error.to_string()) => {
                let resolved_url = docker_compose_database_url(&connection_url, &root)
                    .ok_or_else(|| classify_connection_error(&error.to_string()))?;
                let schema = zelyra_database::inspect_mariadb(&resolved_url)
                    .map_err(|error| classify_connection_error(&error.to_string()))?;
                (schema, "docker_compose_service_alias")
            }
            Err(error) => return Err(classify_connection_error(&error.to_string())),
        },
        // The current PostgreSQL inspector passes its connection URL to `psql`
        // as a process argument. Do not expose a Laravel password in argv.
        "postgres" | "postgresql" => {
            return Err("PostgreSQL live inspection is not enabled because the current client interface can expose credentials in process arguments".into());
        }
        "sqlite" => (
            zelyra_database::inspect_sqlite(&connection_url)
                .map_err(|error| classify_connection_error(&error.to_string()))?,
            "configured_host",
        ),
        _ => {
            return Err(format!(
                "Laravel database driver `{scheme}` is not supported for live schema inspection"
            ))
        }
    };

    let table_count = schema.tables.len();
    let column_count: usize = schema.tables.iter().map(|table| table.columns.len()).sum();
    let tables = schema
        .tables
        .iter()
        .map(|table| {
            json!({
                "name": table.name,
                "columns": table.columns.iter().map(|column| json!({
                    "name": column.name,
                    "sql_type": column.sql_type,
                    "nullable": column.nullable,
                    "primary_key": column.primary_key,
                    "auto_increment": column.auto,
                    "unique": column.unique,
                    "default_present": column.default.is_some()
                })).collect::<Vec<_>>(),
                "indexes": table.indexes.iter().map(|index| json!({
                    "name": index.name,
                    "columns": index.columns,
                    "unique": index.unique
                })).collect::<Vec<_>>(),
                "uniques": table.uniques.iter().map(|index| json!({
                    "name": index.name,
                    "columns": index.columns
                })).collect::<Vec<_>>(),
                "foreign_keys": table.foreign_keys.iter().map(|key| json!({
                    "column": key.column,
                    "columns": key.columns,
                    "referenced_table": key.referenced_table,
                    "referenced_column": key.referenced_column,
                    "referenced_columns": key.referenced_columns,
                    "on_delete": format!("{:?}", key.on_delete)
                })).collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "status": "connected_read_only",
        "driver": scheme,
        "connection_resolution": connection_resolution,
        "source": "Laravel .env connection settings; information_schema/SQLite metadata only",
        "application_rows_read": false,
        "credentials_recorded": false,
        "table_count": table_count,
        "column_count": column_count,
        "tables": tables
    }))
}

fn docker_compose_database_url(connection_url: &str, project_root: &Path) -> Option<String> {
    let url = Url::parse(connection_url).ok()?;
    let configured_host = url.host_str()?.to_owned();
    if configured_host.parse::<IpAddr>().is_ok() {
        return None;
    }
    let working_directory = compose_working_directory(project_root)?;
    let filter = format!(
        "label=com.docker.compose.project.working_dir={}",
        working_directory.display()
    );
    let container_output =
        docker_metadata_output(&["ps".into(), "--quiet".into(), "--filter".into(), filter])?;
    let ids = String::from_utf8(container_output).ok()?;
    let ids = ids
        .lines()
        .filter(|id| {
            (12..=64).contains(&id.len()) && id.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return None;
    }

    // Limit inspected fields to Compose's working directory, network aliases,
    // and assigned IPs. Container environment variables (which may contain
    // secrets) are never requested from Docker or copied to this process.
    const INSPECT_TEMPLATE: &str = "{{ $wd := index .Config.Labels \"com.docker.compose.project.working_dir\" }}{{range $networkName,$network := .NetworkSettings.Networks}}{{range $alias := $network.Aliases}}{{printf \"%s\\t%s\\t%s\\n\" $wd $alias $network.IPAddress}}{{end}}{{end}}";
    let mut inspect_args = vec![
        "inspect".to_owned(),
        "--format".to_owned(),
        INSPECT_TEMPLATE.to_owned(),
    ];
    inspect_args.extend(ids.into_iter().map(str::to_owned));
    let inspected = String::from_utf8(docker_metadata_output(&inspect_args)?).ok()?;
    let ip = compose_alias_ip(&inspected, &working_directory, &configured_host)?;
    replace_database_host(connection_url, ip)
}

fn docker_metadata_output(args: &[String]) -> Option<Vec<u8>> {
    const TIMEOUT: Duration = Duration::from_secs(5);
    let mut child = Command::new("docker")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child.wait_with_output().ok()?;
                return output.status.success().then_some(output.stdout);
            }
            Ok(None) if started.elapsed() < TIMEOUT => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

fn compose_working_directory(project_root: &Path) -> Option<PathBuf> {
    const COMPOSE_FILES: &[&str] = &[
        "compose.yaml",
        "compose.yml",
        "docker-compose.yaml",
        "docker-compose.yml",
    ];
    project_root.ancestors().find_map(|directory| {
        let contains_compose_file = COMPOSE_FILES.iter().any(|name| {
            fs::symlink_metadata(directory.join(name))
                .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        });
        contains_compose_file
            .then(|| fs::canonicalize(directory).ok())
            .flatten()
    })
}

fn compose_alias_ip(output: &str, working_directory: &Path, alias: &str) -> Option<IpAddr> {
    let expected_working_directory = working_directory.to_str()?;
    let mut addresses = BTreeSet::new();
    for line in output.lines() {
        let mut fields = line.split('\t');
        let (Some(directory), Some(candidate_alias), Some(address)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if directory != expected_working_directory || !candidate_alias.eq_ignore_ascii_case(alias) {
            continue;
        }
        if let Ok(address) = address.parse::<IpAddr>() {
            if !address.is_unspecified() && !address.is_loopback() {
                addresses.insert(address);
            }
        }
    }
    (addresses.len() == 1).then(|| *addresses.iter().next().expect("one address"))
}

fn is_host_resolution_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("unknown server host") || lower.contains("name or service not known")
}

fn replace_database_host(connection_url: &str, host: IpAddr) -> Option<String> {
    let mut url = Url::parse(connection_url).ok()?;
    url.set_host(Some(&host.to_string())).ok()?;
    Some(url.to_string())
}

/// Compare only metadata that both the migration projection and the live
/// inspector represent without backend-specific value normalization.
pub(super) fn compare_candidates(live: &Value, generation: &Value) -> Value {
    let live_tables = live["tables"].as_array().cloned().unwrap_or_default();
    let live_by_name = live_tables
        .iter()
        .filter_map(|table| Some((table["name"].as_str()?.to_owned(), table)))
        .collect::<BTreeMap<_, _>>();
    let candidates = generation["candidates"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut compared_tables = Vec::new();
    let mut differences = Vec::new();
    let mut unclassified_types = Vec::new();
    let maria_db_type_comparison = matches!(live["driver"].as_str(), Some("mysql" | "mariadb"));
    let mut candidate_names = BTreeSet::new();

    for candidate in &candidates {
        let Some(table_name) = candidate["table"].as_str() else {
            continue;
        };
        candidate_names.insert(table_name.to_owned());
        let Some(live_table) = live_by_name.get(table_name).copied() else {
            differences.push(json!({"table": table_name, "kind": "missing_live_table"}));
            compared_tables.push(json!({"table": table_name, "status": "missing_live_table"}));
            continue;
        };

        let actual_columns = live_table["columns"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|column| Some((column["name"].as_str()?.to_owned(), column)))
            .collect::<BTreeMap<_, _>>();
        let expected_columns = candidate["columns"].as_array().cloned().unwrap_or_default();
        let mut expected_physical_names = BTreeSet::new();
        for expected in &expected_columns {
            let logical_name = expected["name"].as_str().unwrap_or_default();
            let physical_name = physical_column_name(expected).unwrap_or(logical_name);
            expected_physical_names.insert(physical_name.to_owned());
            let Some(actual) = actual_columns.get(physical_name) else {
                differences.push(json!({
                    "table": table_name,
                    "column": physical_name,
                    "kind": "missing_live_column"
                }));
                continue;
            };
            compare_boolean(
                &mut differences,
                table_name,
                physical_name,
                "nullability_mismatch",
                expected["nullable"].as_bool(),
                actual["nullable"].as_bool(),
            );
            compare_boolean(
                &mut differences,
                table_name,
                physical_name,
                "primary_key_mismatch",
                expected["primary_key"].as_bool(),
                actual["primary_key"].as_bool(),
            );
            compare_boolean(
                &mut differences,
                table_name,
                physical_name,
                "default_presence_mismatch",
                Some(!expected["default"].is_null()),
                actual["default_present"].as_bool(),
            );
            if !maria_db_type_comparison {
                unclassified_types.push(json!({
                    "table": table_name,
                    "column": physical_name,
                    "zelyra_type": expected["zelyra_type"],
                    "reason": "live driver has no matching type-equivalence rules"
                }));
                continue;
            }
            match expected_mariadb_type(expected) {
                Some(expected_type) => {
                    if let Some(actual_type) = actual["sql_type"].as_str() {
                        if normalize_schema_type(actual_type)
                            != normalize_schema_type(&expected_type)
                        {
                            differences.push(json!({
                                "table": table_name,
                                "column": physical_name,
                                "kind": "sql_type_mismatch",
                                "expected_type": expected_type,
                                "actual_type": actual_type
                            }));
                        }
                    }
                }
                None => unclassified_types.push(json!({
                    "table": table_name,
                    "column": physical_name,
                    "zelyra_type": expected["zelyra_type"]
                })),
            }
        }

        for column_name in actual_columns.keys() {
            if !expected_physical_names.contains(column_name) {
                differences.push(json!({
                    "table": table_name,
                    "column": column_name,
                    "kind": "unmodeled_live_column"
                }));
            }
        }

        let physical_names = expected_columns
            .iter()
            .filter_map(|column| {
                Some((
                    column["name"].as_str()?.to_owned(),
                    physical_column_name(column)?.to_owned(),
                ))
            })
            .collect::<BTreeMap<_, _>>();
        compare_indexes(
            &mut differences,
            table_name,
            candidate["indexes"].as_array(),
            live_table["indexes"].as_array(),
            &physical_names,
            "missing_live_index",
            false,
        );
        compare_indexes(
            &mut differences,
            table_name,
            candidate["uniques"].as_array(),
            live_table["uniques"].as_array(),
            &physical_names,
            "missing_live_unique",
            true,
        );

        let actual_foreign_keys = live_table["foreign_keys"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for expected in &expected_columns {
            if expected["relation_status"] != "resolved" {
                continue;
            }
            let physical_name = physical_column_name(expected)
                .unwrap_or_else(|| expected["name"].as_str().unwrap_or_default());
            let relation_table = expected["relation_table"].as_str().unwrap_or_default();
            let relation_column = expected["relation_target_column"].as_str().unwrap_or("id");
            let delete_action = expected["on_delete"].as_str();
            let found = actual_foreign_keys.iter().any(|actual| {
                actual["column"].as_str() == Some(physical_name)
                    && actual["referenced_table"].as_str() == Some(relation_table)
                    && actual["referenced_column"].as_str() == Some(relation_column)
                    && delete_action.is_none_or(|expected_action| {
                        delete_action_matches(
                            expected_action,
                            actual["on_delete"].as_str().unwrap_or_default(),
                        )
                    })
            });
            if !found {
                differences.push(json!({
                    "table": table_name,
                    "column": physical_name,
                    "kind": "missing_or_different_live_foreign_key",
                    "expected_table": relation_table,
                    "expected_column": relation_column,
                    "expected_on_delete": delete_action
                }));
            }
        }
        compared_tables.push(json!({"table": table_name, "status": "compared"}));
    }

    let unmodeled_live_tables = live_by_name
        .keys()
        .filter(|name| !candidate_names.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    let compared_count = compared_tables
        .iter()
        .filter(|table| table["status"] == "compared")
        .count();
    json!({
        "status": "completed",
        "comparison_scope": [
            "table_presence", "column_presence", "nullability", "primary_key",
            "default_presence", "known_sql_type_equivalence", "indexes", "uniques",
            "foreign_key_source_column_target_table_target_column_delete_action"
        ],
        "not_compared": ["default_value_equivalence", "engine", "charset", "collation", "triggers"],
        "unclassified_types": unclassified_types,
        "candidate_table_count": candidates.len(),
        "compared_table_count": compared_count,
        "missing_live_table_count": compared_tables.len().saturating_sub(compared_count),
        "difference_count": differences.len(),
        "tables": compared_tables,
        "differences": differences,
        "unmodeled_live_tables": unmodeled_live_tables
    })
}

fn expected_mariadb_type(column: &Value) -> Option<String> {
    let type_name = column["zelyra_type"].as_str()?;
    if column["relation_status"] == "resolved" {
        return Some("BIGINT".into());
    }
    let length = column["length"].as_u64();
    let decimal = &column["decimal_precision"];
    Some(match type_name {
        "Id" | "Int" => "BIGINT".into(),
        "UInt" => "DECIMAL(20,0)".into(),
        "Float" => "DOUBLE".into(),
        "Decimal" => match (decimal["precision"].as_u64(), decimal["scale"].as_u64()) {
            (Some(precision), Some(scale)) => format!("DECIMAL({precision},{scale})"),
            _ if decimal.is_null() => "DECIMAL(39,9)".into(),
            _ => return None,
        },
        "Bool" => "BOOLEAN".into(),
        "String" => length.map_or_else(|| "TEXT".into(), |length| format!("VARCHAR({length})")),
        "Json" => "JSON".into(),
        "Char" => "CHAR(1)".into(),
        "Bytes" => "BLOB".into(),
        "Timestamp" => "DATETIME(3)".into(),
        "Date" => "DATE".into(),
        "Time" => "TIME".into(),
        "Duration" => "BIGINT".into(),
        "Email" => "VARCHAR(255)".into(),
        "Url" => "VARCHAR(2048)".into(),
        "Uuid" => "CHAR(36)".into(),
        _ => return None,
    })
}

fn normalize_schema_type(value: &str) -> String {
    let normalized = value
        .chars()
        .filter(|character| !character.is_ascii_whitespace() && *character != '`')
        .flat_map(char::to_uppercase)
        .collect::<String>();
    match normalized.as_str() {
        "NUMERIC(20,0)" => "DECIMAL(20,0)".into(),
        "NUMERIC(39,9)" => "DECIMAL(39,9)".into(),
        "NUMERIC" => "DECIMAL".into(),
        other => other.to_owned(),
    }
}

fn physical_column_name(column: &Value) -> Option<&str> {
    column["storage_name"]
        .as_str()
        .or_else(|| column["source_column_name"].as_str())
}

fn compare_boolean(
    differences: &mut Vec<Value>,
    table: &str,
    column: &str,
    kind: &str,
    expected: Option<bool>,
    actual: Option<bool>,
) {
    if let (Some(expected), Some(actual)) = (expected, actual) {
        if expected != actual {
            differences.push(json!({
                "table": table,
                "column": column,
                "kind": kind,
                "expected": expected,
                "actual": actual
            }));
        }
    }
}

fn compare_indexes(
    differences: &mut Vec<Value>,
    table: &str,
    expected: Option<&Vec<Value>>,
    actual: Option<&Vec<Value>>,
    physical_names: &BTreeMap<String, String>,
    kind: &str,
    unique: bool,
) {
    let actual_indexes = actual
        .into_iter()
        .flatten()
        .filter(|index| index["unique"].as_bool().unwrap_or(unique) == unique)
        .filter_map(|index| {
            index["columns"].as_array().map(|columns| {
                columns
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    for index in expected.into_iter().flatten() {
        let Some(columns) = index.as_array() else {
            continue;
        };
        let expected_columns = columns
            .iter()
            .filter_map(Value::as_str)
            .map(|name| {
                physical_names
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| name.to_owned())
            })
            .collect::<Vec<_>>();
        if !actual_indexes
            .iter()
            .any(|actual| actual == &expected_columns)
        {
            differences.push(json!({
                "table": table,
                "kind": kind,
                "columns": expected_columns
            }));
        }
    }
}

fn delete_action_matches(expected: &str, actual: &str) -> bool {
    let normalize = |value: &str| {
        let normalized = value
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>();
        if normalized.is_empty() {
            "noaction".to_owned()
        } else {
            normalized
        }
    };
    normalize(expected) == normalize(actual)
}

fn parse_connection_values(contents: &str) -> Result<Vec<(String, String)>, String> {
    let mut values = Vec::new();
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim_start();
        let Some((key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key != "DB_URL" && key != "DATABASE_URL" && !key.starts_with("DB_") {
            continue;
        }
        if !valid_env_key(key) {
            return Err(format!(
                "invalid database setting name on `.env` line {}",
                line_index + 1
            ));
        }
        if values.iter().any(|(existing, _)| existing == key) {
            return Err(format!("duplicate database setting `{key}` in `.env`"));
        }
        let value = parse_env_value(raw_value.trim()).map_err(|_| {
            format!(
                "unsupported quoting in database setting `{key}` on `.env` line {}",
                line_index + 1
            )
        })?;
        if value.contains('$') {
            return Err(format!("variable expansion in database setting `{key}` is unsupported; use a literal value in `.env`"));
        }
        values.push((key.to_owned(), value));
    }
    Ok(values)
}

fn valid_env_key(key: &str) -> bool {
    let mut bytes = key.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn parse_env_value(raw: &str) -> Result<String, ()> {
    if let Some(quoted) = raw.strip_prefix('\'') {
        let end = quoted.find('\'').ok_or(())?;
        if !quoted[end + 1..].trim().is_empty() {
            return Err(());
        }
        return Ok(quoted[..end].to_owned());
    }
    if let Some(quoted) = raw.strip_prefix('"') {
        let mut output = String::new();
        let mut escaped = false;
        let mut closed_end = None;
        for (index, character) in quoted.char_indices() {
            if escaped {
                output.push(match character {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '"' => '"',
                    '\\' => '\\',
                    other => other,
                });
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                closed_end = Some(index + character.len_utf8());
                break;
            } else {
                output.push(character);
            }
        }
        let Some(closed_end) = closed_end else {
            return Err(());
        };
        if escaped {
            return Err(());
        }
        let after = &quoted[closed_end..];
        if !after.trim().is_empty() && !after.trim_start().starts_with('#') {
            return Err(());
        }
        return Ok(output);
    }
    let value = raw
        .split_once(" #")
        .map_or(raw, |(value, _)| value)
        .trim_end();
    Ok(value.to_owned())
}

fn connection_url(values: &[(String, String)]) -> Result<String, String> {
    let get = |key: &str| {
        values
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
            .filter(|value| !value.is_empty())
    };
    if let Some(url) = get("DB_URL").or_else(|| get("DATABASE_URL")) {
        let parsed = Url::parse(url)
            .map_err(|_| "Laravel DB_URL is malformed (credential values suppressed)".to_owned())?;
        if !matches!(
            parsed.scheme(),
            "mysql" | "mariadb" | "postgres" | "postgresql" | "sqlite"
        ) {
            return Err(format!(
                "Laravel DB_URL scheme `{}` is unsupported",
                parsed.scheme()
            ));
        }
        return Ok(url.to_owned());
    }
    let driver = get("DB_CONNECTION").unwrap_or("mysql");
    let scheme = match driver {
        "mysql" | "mariadb" => driver,
        "pgsql" => "postgresql",
        "sqlite" => "sqlite",
        other => {
            return Err(format!(
                "Laravel DB_CONNECTION driver `{other}` is unsupported for live schema inspection"
            ))
        }
    };
    if scheme == "sqlite" {
        let database =
            get("DB_DATABASE").ok_or("Laravel `.env` is missing DB_DATABASE for SQLite")?;
        return Ok(format!("sqlite://{}", percent_encode(database.as_bytes())));
    }
    let host = get("DB_HOST").ok_or("Laravel `.env` is missing DB_HOST")?;
    let database = get("DB_DATABASE").ok_or("Laravel `.env` is missing DB_DATABASE")?;
    let username = get("DB_USERNAME").ok_or("Laravel `.env` is missing DB_USERNAME")?;
    let port = get("DB_PORT").unwrap_or(if scheme == "postgresql" {
        "5432"
    } else {
        "3306"
    });
    let password = get("DB_PASSWORD").unwrap_or("");
    Ok(format!(
        "{scheme}://{}:{}@{}:{}/{}",
        percent_encode(username.as_bytes()),
        percent_encode(password.as_bytes()),
        percent_encode(host.as_bytes()),
        port,
        percent_encode(database.as_bytes())
    ))
}

fn percent_encode(bytes: &[u8]) -> String {
    let mut encoded = String::new();
    for byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(*byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn classify_connection_error(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if lower.contains("unknown server host") || lower.contains("name or service not known") {
        "could not resolve Laravel database host; credentials and raw client output were suppressed"
            .into()
    } else if lower.contains("access denied") || lower.contains("password authentication failed") {
        "Laravel database rejected the configured credentials or metadata grants; raw client output was suppressed".into()
    } else if lower.contains("unknown database") || lower.contains("database does not exist") {
        "the configured Laravel database name was not found; credentials and raw client output were suppressed".into()
    } else if lower.contains("unknown system variable") || lower.contains("unknown variable") {
        "the database rejected a vendor-specific inspection setting; importer timeout selection may need an update".into()
    } else if lower.contains("certificate") || lower.contains("tls") || lower.contains("ssl") {
        "Laravel database TLS negotiation failed; check the database certificate and explicit Zelyra TLS settings".into()
    } else if lower.contains("unexpected mariadb")
        || lower.contains("invalid encoded schema metadata")
        || lower.contains("encoded schema metadata is not valid utf-8")
        || lower.contains("unexpected foreign-key inspection row")
        || lower.contains("unsupported maria")
    {
        "the database answered, but its schema metadata contains a shape not supported by the importer".into()
    } else if lower.contains("connection refused") || lower.contains("timed out") {
        "could not connect to the Laravel database host and port; credentials and raw client output were suppressed".into()
    } else if lower.contains("could not start") || lower.contains("no such file") {
        "required database inspection client is unavailable; install the matching database client"
            .into()
    } else {
        "could not inspect Laravel database schema; check reachability, grants, and driver support (raw client output suppressed)".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_database_dotenv_values_and_quotes() {
        let parsed = parse_connection_values(
            "APP_KEY=do-not-read\nDB_HOST=localhost # comment\nDB_DATABASE=app\nDB_USERNAME='a user'\nDB_PASSWORD=\"p\\\"ss\"\n",
        )
        .unwrap();
        assert_eq!(
            parsed,
            vec![
                ("DB_HOST".into(), "localhost".into()),
                ("DB_DATABASE".into(), "app".into()),
                ("DB_USERNAME".into(), "a user".into()),
                ("DB_PASSWORD".into(), "p\"ss".into())
            ]
        );
        let url = connection_url(&parsed).unwrap();
        assert!(url.contains("a%20user:p%22ss@"));
        assert!(!url.contains("APP_KEY"));
    }

    #[test]
    fn rejects_duplicate_or_expanded_database_settings() {
        assert!(parse_connection_values("DB_HOST=a\nDB_HOST=b\n").is_err());
        assert!(parse_connection_values("DB_PASSWORD=${PASSWORD}\n").is_err());
    }

    #[test]
    fn compose_alias_resolution_requires_one_matching_ip_from_the_same_project() {
        let output = concat!(
            "/opt/project\tmariadb\t172.18.0.2\n",
            "/opt/other\tmariadb\t172.18.0.3\n",
            "/opt/project\tweb\t172.18.0.4\n",
            "/opt/project\tmariadb\tinvalid\n"
        );
        assert_eq!(
            compose_alias_ip(output, Path::new("/opt/project"), "mariadb"),
            Some("172.18.0.2".parse().unwrap())
        );
        assert_eq!(
            compose_alias_ip(output, Path::new("/opt/project"), "missing"),
            None
        );
    }

    #[test]
    fn compose_alias_resolution_rejects_ambiguous_addresses() {
        let output = "/opt/project\tmariadb\t172.18.0.2\n/opt/project\tmariadb\t172.18.0.3\n";
        assert_eq!(
            compose_alias_ip(output, Path::new("/opt/project"), "mariadb"),
            None
        );
    }

    #[test]
    fn docker_host_resolution_fallback_is_limited_to_name_resolution_errors() {
        assert!(is_host_resolution_error(
            "ERROR 2005 (HY000): Unknown server host 'mariadb'"
        ));
        assert!(!is_host_resolution_error("Access denied for user"));
        assert!(!is_host_resolution_error("connection timed out"));
    }

    #[test]
    fn live_schema_errors_are_classified_without_echoing_raw_database_details() {
        assert!(classify_connection_error("Unknown database 'client-db'").contains("database name"));
        assert!(classify_connection_error("TLS certificate verification failed").contains("TLS"));
        assert!(
            classify_connection_error("unexpected MariaDB column metadata with 11 fields")
                .contains("schema metadata")
        );
        let rejected =
            classify_connection_error("Access denied for user importer with password p@ss");
        assert!(rejected.contains("credentials or metadata grants"));
        assert!(!rejected.contains("importer"));
        assert!(!rejected.contains("p@ss"));
        assert!(
            classify_connection_error("Unknown system variable max_execution_time")
                .contains("vendor-specific inspection setting")
        );
    }

    #[test]
    fn database_url_host_override_keeps_credentials_private_and_connection_settings() {
        let url = replace_database_host(
            "mariadb://importer:encoded%40password@mariadb:3307/siedelmann?ssl-mode=required",
            "172.18.0.2".parse().unwrap(),
        )
        .unwrap();
        let parsed = Url::parse(&url).unwrap();
        assert_eq!(parsed.host_str(), Some("172.18.0.2"));
        assert_eq!(parsed.port(), Some(3307));
        assert_eq!(parsed.path(), "/siedelmann");
        assert_eq!(parsed.query(), Some("ssl-mode=required"));
        assert_eq!(parsed.username(), "importer");
        assert_eq!(parsed.password(), Some("encoded%40password"));
    }

    #[test]
    fn unsupported_driver_does_not_include_credentials_in_error() {
        let values = vec![
            ("DB_CONNECTION".into(), "sqlsrv".into()),
            ("DB_PASSWORD".into(), "sensitive-secret".into()),
        ];
        let error = connection_url(&values).unwrap_err();
        assert!(error.contains("sqlsrv"));
        assert!(!error.contains("sensitive-secret"));
    }

    #[test]
    fn compares_live_metadata_to_supported_migration_projection() {
        let generation = json!({
            "candidates": [{
                "table": "orders",
                "columns": [
                    {"name":"id","source_column_name":"id","nullable":false,"primary_key":true,"default":null,"relation_status":"not_a_foreign_key"},
                    {"name":"customer","source_column_name":"customer_id","nullable":false,"primary_key":false,"default":null,"relation_status":"resolved","relation_table":"customers","on_delete":null},
                    {"name":"note","source_column_name":"note","storage_name":"note_text","nullable":true,"primary_key":false,"default":"default 'reviewed'","relation_status":"not_a_foreign_key"},
                    {"name":"email","source_column_name":"email","nullable":false,"primary_key":false,"default":null,"relation_status":"not_a_foreign_key"}
                ],
                "indexes": [["customer"]],
                "uniques": [["email"]]
            }, {"table":"missing_table","columns":[]}]
        });
        let live = json!({
            "tables": [{
                "name":"orders",
                "columns": [
                    {"name":"id","nullable":false,"primary_key":true,"default_present":false},
                    {"name":"customer_id","nullable":false,"primary_key":false,"default_present":false},
                    {"name":"note_text","nullable":true,"primary_key":false,"default_present":true},
                    {"name":"email","nullable":false,"primary_key":false,"default_present":false},
                    {"name":"legacy_flag","nullable":false,"primary_key":false,"default_present":true}
                ],
                "indexes": [{"columns":["customer_id"],"unique":false}],
                "uniques": [{"columns":["email"]}],
                "foreign_keys": [{"column":"customer_id","referenced_table":"customers","referenced_column":"id","on_delete":"Restrict"}]
            }, {"name":"laravel_sessions","columns":[],"indexes":[],"uniques":[],"foreign_keys":[]}]
        });

        let comparison = compare_candidates(&live, &generation);
        assert_eq!(comparison["status"], "completed");
        assert_eq!(comparison["candidate_table_count"], 2);
        assert_eq!(comparison["compared_table_count"], 1);
        assert_eq!(comparison["missing_live_table_count"], 1);
        let differences = comparison["differences"].as_array().unwrap();
        assert_eq!(differences.len(), 2);
        assert!(differences.iter().any(|difference| {
            difference["kind"] == "unmodeled_live_column" && difference["column"] == "legacy_flag"
        }));
        assert!(differences
            .iter()
            .any(|difference| difference["kind"] == "missing_live_table"));
        assert!(comparison["unmodeled_live_tables"]
            .as_array()
            .unwrap()
            .iter()
            .any(|table| table == "laravel_sessions"));
    }

    #[test]
    fn reports_live_column_and_constraint_differences_without_guessing_types() {
        let generation = json!({"candidates":[{
            "table":"orders",
            "columns":[
                {"name":"id","source_column_name":"id","nullable":false,"primary_key":true,"default":null,"relation_status":"not_a_foreign_key"},
                {"name":"customer","source_column_name":"customer_id","nullable":false,"primary_key":false,"default":null,"relation_status":"resolved","relation_table":"customers","on_delete":"cascade"}
            ],
            "indexes":[["customer"]],
            "uniques":[]
        }]});
        let live = json!({"tables":[{
            "name":"orders",
            "columns":[
                {"name":"id","nullable":false,"primary_key":false,"default_present":false},
                {"name":"customer_id","nullable":true,"primary_key":false,"default_present":true}
            ],
            "indexes":[],"uniques":[],"foreign_keys":[]
        }]});
        let comparison = compare_candidates(&live, &generation);
        let differences = comparison["differences"].as_array().unwrap();
        assert!(differences.iter().any(|difference| {
            difference["kind"] == "primary_key_mismatch" && difference["column"] == "id"
        }));
        assert!(differences.iter().any(|difference| {
            difference["kind"] == "nullability_mismatch" && difference["column"] == "customer_id"
        }));
        assert!(differences.iter().any(|difference| {
            difference["kind"] == "default_presence_mismatch"
                && difference["column"] == "customer_id"
        }));
        assert!(differences
            .iter()
            .any(|difference| difference["kind"] == "missing_live_index"));
        assert!(differences
            .iter()
            .any(|difference| { difference["kind"] == "missing_or_different_live_foreign_key" }));
        assert!(comparison["not_compared"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "default_value_equivalence"));
    }

    #[test]
    fn compares_known_mariadb_types_and_foreign_key_target_columns() {
        let generation = json!({"candidates":[{
            "table":"orders",
            "columns":[
                {"name":"id","source_column_name":"id","zelyra_type":"Id","nullable":false,"primary_key":true,"default":null,"relation_status":"not_a_foreign_key"},
                {"name":"amount","source_column_name":"amount","zelyra_type":"Decimal","decimal_precision":{"precision":12,"scale":2},"nullable":false,"primary_key":false,"default":null,"relation_status":"not_a_foreign_key"},
                {"name":"customer","source_column_name":"customer_id","zelyra_type":"Customer","nullable":false,"primary_key":false,"default":null,"relation_status":"resolved","relation_table":"customers","relation_target_column":"customer_key","on_delete":"Restrict"},
                {"name":"legacy","source_column_name":"legacy_id","zelyra_type":"LegacyIdentifier","nullable":true,"primary_key":false,"default":null,"relation_status":"not_a_foreign_key"}
            ],
            "indexes":[],"uniques":[]
        }]});
        let live = json!({"driver":"mariadb","tables":[{
            "name":"orders",
            "columns":[
                {"name":"id","sql_type":"BIGINT","nullable":false,"primary_key":true,"default_present":false},
                {"name":"amount","sql_type":"DECIMAL(12,2)","nullable":false,"primary_key":false,"default_present":false},
                {"name":"customer_id","sql_type":"BIGINT","nullable":false,"primary_key":false,"default_present":false},
                {"name":"legacy_id","sql_type":"VARCHAR(40)","nullable":true,"primary_key":false,"default_present":false}
            ],
            "indexes":[],"uniques":[],
            "foreign_keys":[{"column":"customer_id","referenced_table":"customers","referenced_column":"id","on_delete":"Restrict"}]
        }]});

        let comparison = compare_candidates(&live, &generation);
        let differences = comparison["differences"].as_array().unwrap();
        assert!(differences.iter().any(|difference| {
            difference["kind"] == "missing_or_different_live_foreign_key"
                && difference["expected_column"] == "customer_key"
        }));
        assert!(!differences.iter().any(|difference| {
            difference["kind"] == "sql_type_mismatch" && difference["column"] == "amount"
        }));
        assert_eq!(comparison["unclassified_types"][0]["column"], "legacy_id");
        assert!(comparison["comparison_scope"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "known_sql_type_equivalence"));
    }

    #[test]
    fn reports_known_sql_type_mismatches_without_normalizing_precision_away() {
        let generation = json!({"candidates":[{"table":"orders","columns":[{
            "name":"amount","source_column_name":"amount","zelyra_type":"Decimal",
            "decimal_precision":{"precision":12,"scale":2},"nullable":false,
            "primary_key":false,"default":null,"relation_status":"not_a_foreign_key"
        }],"indexes":[],"uniques":[]}]});
        let live = json!({"driver":"mysql","tables":[{"name":"orders","columns":[{
            "name":"amount","sql_type":"DECIMAL(12,4)","nullable":false,
            "primary_key":false,"default_present":false
        }],"indexes":[],"uniques":[],"foreign_keys":[]}]});

        let comparison = compare_candidates(&live, &generation);
        assert!(comparison["differences"]
            .as_array()
            .unwrap()
            .iter()
            .any(|difference| {
                difference["kind"] == "sql_type_mismatch"
                    && difference["expected_type"] == "DECIMAL(12,2)"
                    && difference["actual_type"] == "DECIMAL(12,4)"
            }));
    }

    #[test]
    fn does_not_apply_mariadb_type_rules_to_sqlite_schema_metadata() {
        let generation = json!({"candidates":[{"table":"orders","columns":[{
            "name":"id","source_column_name":"id","zelyra_type":"Id",
            "nullable":false,"primary_key":true,"default":null,
            "relation_status":"not_a_foreign_key"
        }],"indexes":[],"uniques":[]}]});
        let live = json!({"driver":"sqlite","tables":[{"name":"orders","columns":[{
            "name":"id","sql_type":"INTEGER","nullable":false,
            "primary_key":true,"default_present":false
        }],"indexes":[],"uniques":[],"foreign_keys":[]}]});

        let comparison = compare_candidates(&live, &generation);
        assert_eq!(comparison["difference_count"], 0);
        assert_eq!(comparison["unclassified_types"][0]["column"], "id");
        assert_eq!(
            comparison["unclassified_types"][0]["reason"],
            "live driver has no matching type-equivalence rules"
        );
    }

    #[test]
    fn connection_failures_are_redacted_and_actionable() {
        let error = classify_connection_error("ERROR 2005 (HY000): Unknown server host 'db'");
        assert!(error.contains("resolve"));
        assert!(!error.contains("'db'"));
    }
}
