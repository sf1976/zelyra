use super::*;

pub(super) fn load_schema(path: &str) -> Result<Schema, ()> {
    let project = load_project(path)?;
    match build_schema(&project.program) {
        Ok(schema) => Ok(schema),
        Err(errors) => {
            for error in errors {
                let source_path = PROJECT_SOURCES.with(|sources| {
                    sources
                        .borrow()
                        .get(error.span.source_id as usize)
                        .map_or_else(|| path.to_owned(), |source| source.path.clone())
                });
                diagnostic_with_span(&source_path, "E-DB-001", &error.message, error.span);
            }
            Err(())
        }
    }
}

fn print_plan(plan: &zelyra_database::SchemaPlan) {
    if plan.changes.is_empty() {
        println!("No schema changes.");
        return;
    }
    for check in &plan.nullability_preflights {
        println!(
            "[PREFLIGHT] verify `{}.{}` has no NULL values before applying any SQL",
            check.table, check.column
        );
    }
    for check in &plan.required_column_preflights {
        println!(
            "[PREFLIGHT] verify `{}` is empty before adding required column `{}` without a default",
            check.table, check.column
        );
    }
    for check in &plan.unique_index_preflights {
        println!(
            "[PREFLIGHT] verify `{}.{}` has no duplicate values before applying any SQL",
            check.table,
            check.columns.join(", ")
        );
    }
    for check in &plan.foreign_key_preflights {
        println!(
            "[PREFLIGHT] verify `{}.{}` references existing `{}.{}` values before applying any SQL",
            check.table, check.column, check.referenced_table, check.referenced_column
        );
    }
    for change in &plan.changes {
        let risk = match change.risk {
            Risk::Safe => "SAFE",
            Risk::RequiresApproval => "REVIEW",
            Risk::Destructive => "DESTRUCTIVE",
            Risk::Unsupported => "UNSUPPORTED",
        };
        println!("[{risk}] {}\n{}\n", change.description, change.sql);
    }
}

pub(super) fn schema_fingerprint(schema: &Schema) -> String {
    let mut tables = schema.tables.iter().collect::<Vec<_>>();
    tables.sort_by(|left, right| left.name.cmp(&right.name));
    let tables = tables
        .into_iter()
        .map(|table| {
            let mut columns = table.columns.iter().collect::<Vec<_>>();
            columns.sort_by(|left, right| left.name.cmp(&right.name));
            let columns = columns
                .into_iter()
                .map(|column| {
                    json!({
                        "name": column.name,
                        "sql_type": column.sql_type,
                        "nullable": column.nullable,
                        "primary_key": column.primary_key,
                        "auto_increment": column.auto,
                        "unique": column.unique,
                        "default": column.default,
                    })
                })
                .collect::<Vec<_>>();
            let mut foreign_keys = table.foreign_keys.iter().collect::<Vec<_>>();
            foreign_keys.sort_by_key(|key| {
                (
                    key.name.as_deref().unwrap_or_default(),
                    key.column.as_str(),
                    key.referenced_table.as_str(),
                    key.referenced_column.as_str(),
                )
            });
            let foreign_keys = foreign_keys
                .into_iter()
                .map(|key| {
                    json!({
                        "name": key.name,
                        "column": key.column,
                        "referenced_table": key.referenced_table,
                        "referenced_column": key.referenced_column,
                    })
                })
                .collect::<Vec<_>>();
            let mut indexes = table
                .indexes
                .iter()
                .chain(table.uniques.iter())
                .collect::<Vec<_>>();
            indexes.sort_by_key(|index| {
                (
                    index.name.as_str(),
                    index.unique,
                    index.columns.join("\0"),
                    index.constraint_owned,
                )
            });
            let indexes = indexes
                .into_iter()
                .map(|index| {
                    json!({
                        "name": index.name,
                        "columns": index.columns,
                        "unique": index.unique,
                        "constraint_owned": index.constraint_owned,
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "name": table.name,
                "columns": columns,
                "foreign_keys": foreign_keys,
                "indexes": indexes,
            })
        })
        .collect::<Vec<_>>();
    let canonical = json!({ "backend": schema.backend().name(), "tables": tables });
    let bytes = serde_json::to_vec(&canonical).expect("schema fingerprint JSON is serializable");
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn schema_plan_json(
    desired: &Schema,
    current: &Schema,
    plan: &zelyra_database::SchemaPlan,
) -> Value {
    let current_fingerprint = schema_fingerprint(current);
    let desired_fingerprint = schema_fingerprint(desired);
    let changes = schema_plan_changes_json(plan);
    let preflights = schema_plan_preflights_json(plan);
    let plan_id = schema_plan_identifier(
        desired.backend(),
        &current_fingerprint,
        &desired_fingerprint,
        &changes,
        &preflights,
    );

    // A reverse plan is derived from the same schema diff engine as a forward
    // plan. It is bound to the expected post-migration schema fingerprint and
    // remains an explicit operator action because reverse DDL can discard data
    // written after the migration, even when the old schema can be restored.
    let rollback_plan = zelyra_database::diff(current, desired);
    let rollback_changes = schema_plan_changes_json(&rollback_plan);
    let rollback_preflights = schema_plan_preflights_json(&rollback_plan);
    let rollback_id = schema_plan_identifier(
        current.backend(),
        &desired_fingerprint,
        &current_fingerprint,
        &rollback_changes,
        &rollback_preflights,
    );
    let rollback_generated =
        !plan.changes.is_empty() && !plan.has_unsupported() && !rollback_plan.has_unsupported();
    let identity = json!({
        "format": "zelyra.schema-plan/v1",
        "backend": desired.backend().name(),
        "current_schema_sha256": current_fingerprint,
        "desired_schema_sha256": desired_fingerprint,
        "changes": changes,
        "preflights": preflights,
    });
    debug_assert_eq!(
        plan_id,
        format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&identity).expect("plan identity serializes"))
        )
    );
    json!({
        "format": "zelyra.schema-plan/v1",
        "plan_id": plan_id,
        "backend": desired.backend().name(),
        "current_schema_sha256": current_fingerprint,
        "desired_schema_sha256": desired_fingerprint,
        "drift": if plan.changes.is_empty() { "none" } else { "present" },
        "changes": changes,
        "preflights": preflights,
        "requires_operator_approval": plan.requires_approval(),
        "has_unsupported_changes": plan.has_unsupported(),
        "rollback": {
            "generated": rollback_generated,
            "plan_id": if rollback_generated { json!(rollback_id) } else { Value::Null },
            "from_schema_sha256": desired_fingerprint,
            "to_schema_sha256": current_fingerprint,
            "changes": rollback_changes,
            "preflights": rollback_preflights,
            "requires_operator_approval": rollback_plan.requires_approval(),
            "requires_verified_backup": !plan.changes.is_empty(),
            "safe_to_apply_automatically": false,
            "hint": if plan.changes.is_empty() {
                "No schema changes; rollback is not applicable."
            } else if !rollback_generated {
                "A complete reverse plan is unavailable for unsupported changes. Restore from a verified operator-managed backup."
            } else {
                "Restore the pre-migration schema source, verify a backup, generate and review a fresh plan against the current schema, then apply it with its reviewed plan id and explicit approval when required. Reverse DDL can discard post-migration data."
            },
        },
        "automatic_retries": false,
    })
}

fn schema_plan_changes_json(plan: &zelyra_database::SchemaPlan) -> Vec<Value> {
    plan.changes
        .iter()
        .map(|change| {
            let risk = match change.risk {
                Risk::Safe => "safe",
                Risk::RequiresApproval => "requires_approval",
                Risk::Destructive => "destructive",
                Risk::Unsupported => "unsupported",
            };
            json!({
                "description": change.description,
                "sql": change.sql,
                "risk": risk,
            })
        })
        .collect()
}

fn schema_plan_preflights_json(plan: &zelyra_database::SchemaPlan) -> Vec<Value> {
    plan.nullability_preflights
        .iter()
        .map(|check| {
            json!({
                "kind": "no_null_values",
                "table": check.table,
                "column": check.column,
            })
        })
        .chain(plan.required_column_preflights.iter().map(|check| {
            json!({
                "kind": "table_must_be_empty",
                "table": check.table,
                "column": check.column,
            })
        }))
        .chain(plan.unique_index_preflights.iter().map(|check| {
            json!({
                "kind": "values_must_be_unique",
                "table": check.table,
                "columns": check.columns,
            })
        }))
        .chain(plan.foreign_key_preflights.iter().map(|check| {
            json!({
                "kind": "foreign_key_values_must_exist",
                "table": check.table,
                "column": check.column,
                "referenced_table": check.referenced_table,
                "referenced_column": check.referenced_column,
                "referenced_table_exists_before_apply": check.referenced_table_exists,
            })
        }))
        .collect()
}

fn schema_plan_identifier(
    backend: Backend,
    current_fingerprint: &str,
    desired_fingerprint: &str,
    changes: &[Value],
    preflights: &[Value],
) -> String {
    let identity = json!({
        "format": "zelyra.schema-plan/v1",
        "backend": backend.name(),
        "current_schema_sha256": current_fingerprint,
        "desired_schema_sha256": desired_fingerprint,
        "changes": changes,
        "preflights": preflights,
    });
    let bytes = serde_json::to_vec(&identity).expect("schema plan identity is serializable");
    format!("sha256:{:x}", Sha256::digest(bytes))
}

const MARIADB_SCHEMA_HISTORY_TABLE: &str = "_zelyra_schema_history";

fn native_sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn mariadb_sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn ensure_mariadb_schema_history(database_url: &str) -> Result<(), DatabaseError> {
    apply_mariadb(
        database_url,
        &format!(
            "CREATE TABLE IF NOT EXISTS `{MARIADB_SCHEMA_HISTORY_TABLE}` (\
                plan_id VARCHAR(71) NOT NULL PRIMARY KEY, \
                before_schema_sha256 CHAR(64) NOT NULL, \
                desired_schema_sha256 CHAR(64) NOT NULL, \
                status VARCHAR(16) NOT NULL, \
                attempt_count INT UNSIGNED NOT NULL DEFAULT 1, \
                change_count INT UNSIGNED NOT NULL, \
                completed_changes INT UNSIGNED NOT NULL DEFAULT 0, \
                current_change_index INT UNSIGNED NULL, \
                current_change_description TEXT NULL, \
                started_at DATETIME(6) NOT NULL, \
                updated_at DATETIME(6) NOT NULL, \
                finished_at DATETIME(6) NULL\
            ) ENGINE=InnoDB"
        ),
    )
}

fn start_mariadb_migration(
    database_url: &str,
    plan_id: &str,
    before_fingerprint: &str,
    desired_fingerprint: &str,
    change_count: usize,
) -> Result<(), DatabaseError> {
    ensure_mariadb_schema_history(database_url)?;
    apply_mariadb(
        database_url,
        &format!(
            "INSERT INTO `{MARIADB_SCHEMA_HISTORY_TABLE}` \
                (plan_id, before_schema_sha256, desired_schema_sha256, status, attempt_count, change_count, completed_changes, current_change_index, current_change_description, started_at, updated_at, finished_at) \
             VALUES ({}, {}, {}, 'running', 1, {change_count}, 0, NULL, NULL, UTC_TIMESTAMP(6), UTC_TIMESTAMP(6), NULL) \
             ON DUPLICATE KEY UPDATE before_schema_sha256=VALUES(before_schema_sha256), desired_schema_sha256=VALUES(desired_schema_sha256), status='running', attempt_count=attempt_count+1, change_count=VALUES(change_count), completed_changes=0, current_change_index=NULL, current_change_description=NULL, started_at=UTC_TIMESTAMP(6), updated_at=UTC_TIMESTAMP(6), finished_at=NULL",
            mariadb_sql_literal(plan_id),
            mariadb_sql_literal(before_fingerprint),
            mariadb_sql_literal(desired_fingerprint),
        ),
    )
}

fn update_mariadb_migration(
    database_url: &str,
    plan_id: &str,
    status: &str,
    completed_changes: usize,
    current_change: Option<(usize, &str)>,
    finished: bool,
) -> Result<(), DatabaseError> {
    let (current_index, description) = current_change.map_or(
        ("NULL".to_owned(), "NULL".to_owned()),
        |(index, description)| (index.to_string(), mariadb_sql_literal(description)),
    );
    let finished_at = if finished { "UTC_TIMESTAMP(6)" } else { "NULL" };
    apply_mariadb(
        database_url,
        &format!(
            "UPDATE `{MARIADB_SCHEMA_HISTORY_TABLE}` SET status={}, completed_changes={completed_changes}, current_change_index={current_index}, current_change_description={description}, updated_at=UTC_TIMESTAMP(6), finished_at={finished_at} WHERE plan_id={}",
            mariadb_sql_literal(status),
            mariadb_sql_literal(plan_id),
        ),
    )
}

fn read_mariadb_migration_history(database_url: &str) -> Result<Value, DatabaseError> {
    let table_exists = query_mariadb(
        database_url,
        &format!(
            "SELECT COUNT(*) FROM information_schema.TABLES WHERE TABLE_SCHEMA=DATABASE() AND TABLE_NAME='{}'",
            MARIADB_SCHEMA_HISTORY_TABLE
        ),
    )?;
    match table_exists.trim() {
        "0" => return Ok(json!([])),
        "1" => {}
        _ => {
            return Err(DatabaseError {
                message: "MariaDB returned an invalid migration history table status".into(),
            });
        }
    }
    let output = query_mariadb(
        database_url,
        &format!(
            "SELECT COALESCE(JSON_ARRAYAGG(JSON_OBJECT(\
                'plan_id', plan_id, \
                'before_schema_sha256', before_schema_sha256, \
                'desired_schema_sha256', desired_schema_sha256, \
                'status', status, \
                'attempt_count', attempt_count, \
                'change_count', change_count, \
                'completed_changes', completed_changes, \
                'current_change_index', current_change_index, \
                'current_change_description', current_change_description, \
                'started_at', DATE_FORMAT(started_at, '%Y-%m-%dT%H:%i:%s.%fZ'), \
                'updated_at', DATE_FORMAT(updated_at, '%Y-%m-%dT%H:%i:%s.%fZ'), \
                'finished_at', IF(finished_at IS NULL, NULL, DATE_FORMAT(finished_at, '%Y-%m-%dT%H:%i:%s.%fZ'))\
            ) ORDER BY started_at DESC, plan_id DESC), JSON_ARRAY()) FROM `{MARIADB_SCHEMA_HISTORY_TABLE}`"
        ),
    )?;
    serde_json::from_str(output.trim()).map_err(|error| DatabaseError {
        message: format!("MariaDB returned invalid migration history JSON: {error}"),
    })
}

fn apply_mariadb_plan(
    database_url: &str,
    desired: &Schema,
    planned: &zelyra_database::SchemaPlan,
    expected_plan_id: &str,
) -> Result<(), DatabaseError> {
    with_mariadb_schema_lock(database_url, || {
        let current = inspect_mariadb(database_url)?;
        let fresh_plan = diff(desired, &current);
        let fresh_json = schema_plan_json(desired, &current, &fresh_plan);
        let fresh_plan_id = fresh_json["plan_id"].as_str().unwrap_or_default();
        if fresh_plan_id != expected_plan_id {
            return Err(DatabaseError {
                message: "E-DB-007: the database schema changed before the migration lock was acquired; no SQL was applied. Run `zelyra db plan --format=json` again and review the new plan id".into(),
            });
        }
        let before_fingerprint = fresh_json["current_schema_sha256"]
            .as_str()
            .expect("schema plan JSON contains current schema fingerprint");
        let desired_fingerprint = fresh_json["desired_schema_sha256"]
            .as_str()
            .expect("schema plan JSON contains desired schema fingerprint");
        start_mariadb_migration(
            database_url,
            expected_plan_id,
            before_fingerprint,
            desired_fingerprint,
            planned.changes.len(),
        )?;

        let strict_mode = !planned.nullability_preflights.is_empty()
            || !planned.required_column_preflights.is_empty();
        for (index, change) in planned.changes.iter().enumerate() {
            let step = index + 1;
            update_mariadb_migration(
                database_url,
                expected_plan_id,
                "running",
                index,
                Some((step, &change.description)),
                false,
            )?;
            let sql = if strict_mode {
                format!(
                    "SET SESSION sql_mode = CONCAT_WS(',', NULLIF(@@SESSION.sql_mode, ''), 'STRICT_ALL_TABLES');\n{}",
                    change.sql
                )
            } else {
                change.sql.clone()
            };
            if let Err(error) = apply_mariadb(database_url, &sql) {
                let _ = update_mariadb_migration(
                    database_url,
                    expected_plan_id,
                    "failed",
                    index,
                    Some((step, &change.description)),
                    true,
                );
                return Err(error);
            }
            update_mariadb_migration(database_url, expected_plan_id, "running", step, None, false)?;
        }
        update_mariadb_migration(
            database_url,
            expected_plan_id,
            "applied",
            planned.changes.len(),
            None,
            true,
        )
    })
}

fn print_mariadb_migration_history(
    database_url: &str,
    json_format: bool,
) -> Result<(), DatabaseError> {
    let mut history = read_mariadb_migration_history(database_url)?;
    let lock_held = mariadb_schema_migration_lock_is_held(database_url)?;
    if let Some(entries) = history.as_array_mut() {
        for entry in entries {
            if entry["status"] == "running" && !lock_held {
                entry["status"] = Value::String("interrupted".into());
            }
        }
    }
    if json_format {
        println!(
            "{}",
            serde_json::to_string_pretty(&history).expect("migration history JSON is serializable")
        );
    } else if let Some(entries) = history.as_array() {
        if entries.is_empty() {
            println!("No MariaDB schema migrations recorded.");
        }
        for entry in entries {
            println!(
                "{}  {}  {}/{} changes  {}",
                entry["status"].as_str().unwrap_or("unknown"),
                entry["plan_id"].as_str().unwrap_or("unknown plan"),
                entry["completed_changes"].as_u64().unwrap_or_default(),
                entry["change_count"].as_u64().unwrap_or_default(),
                entry["started_at"].as_str().unwrap_or("unknown time"),
            );
            if let Some(description) = entry["current_change_description"].as_str() {
                println!("  current change: {description}");
            }
        }
    }
    Ok(())
}

fn ensure_native_schema_history(backend: Backend, database_url: &str) -> Result<(), DatabaseError> {
    let create = format!(
        "CREATE TABLE IF NOT EXISTS {MARIADB_SCHEMA_HISTORY_TABLE} (\
            plan_id TEXT PRIMARY KEY, before_schema_sha256 TEXT NOT NULL, \
            desired_schema_sha256 TEXT NOT NULL, status TEXT NOT NULL, \
            attempt_count INTEGER NOT NULL DEFAULT 1, change_count INTEGER NOT NULL, \
            completed_changes INTEGER NOT NULL DEFAULT 0, current_change_index INTEGER, \
            current_change_description TEXT, started_at TEXT NOT NULL, updated_at TEXT NOT NULL, \
            finished_at TEXT)"
    );
    match backend {
        Backend::Sqlite => zelyra_database::query_sqlite(database_url, &create).map(|_| ()),
        Backend::Postgres => apply_postgres(database_url, &create),
        Backend::MariaDb => unreachable!("native history is only used for SQLite and PostgreSQL"),
    }
}

fn native_history_query(backend: Backend) -> &'static str {
    match backend {
        Backend::Sqlite => {
            "SELECT COALESCE(json_group_array(json_object(\
            'plan_id', plan_id, 'before_schema_sha256', before_schema_sha256, \
            'desired_schema_sha256', desired_schema_sha256, 'status', status, \
            'attempt_count', attempt_count, 'change_count', change_count, \
            'completed_changes', completed_changes, 'current_change_index', current_change_index, \
            'current_change_description', current_change_description, 'started_at', started_at, \
            'updated_at', updated_at, 'finished_at', finished_at)), '[]') \
            FROM (SELECT * FROM _zelyra_schema_history ORDER BY started_at DESC, plan_id DESC)"
        }
        Backend::Postgres => {
            "SELECT COALESCE(json_agg(json_build_object(\
            'plan_id', plan_id, 'before_schema_sha256', before_schema_sha256, \
            'desired_schema_sha256', desired_schema_sha256, 'status', status, \
            'attempt_count', attempt_count, 'change_count', change_count, \
            'completed_changes', completed_changes, 'current_change_index', current_change_index, \
            'current_change_description', current_change_description, 'started_at', started_at, \
            'updated_at', updated_at, 'finished_at', finished_at) \
            ORDER BY started_at DESC, plan_id DESC), '[]'::json)::text \
            FROM _zelyra_schema_history"
        }
        Backend::MariaDb => unreachable!("native history is only used for SQLite and PostgreSQL"),
    }
}

fn read_native_migration_history(
    backend: Backend,
    database_url: &str,
) -> Result<Value, DatabaseError> {
    let exists_query = match backend {
        Backend::Sqlite => "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_zelyra_schema_history'",
        Backend::Postgres => "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='public' AND table_name='_zelyra_schema_history'",
        Backend::MariaDb => unreachable!("native history is only used for SQLite and PostgreSQL"),
    };
    let exists = match backend {
        Backend::Sqlite => zelyra_database::query_sqlite(database_url, exists_query)?,
        Backend::Postgres => zelyra_database::query_postgres(database_url, exists_query)?,
        Backend::MariaDb => unreachable!("native history is only used for SQLite and PostgreSQL"),
    };
    if exists.trim() == "0" {
        return Ok(json!([]));
    }
    let result = match backend {
        Backend::Sqlite => zelyra_database::query_sqlite(
            database_url,
            &format!(
                "BEGIN IMMEDIATE; UPDATE _zelyra_schema_history SET status='interrupted', updated_at=CURRENT_TIMESTAMP, finished_at=CURRENT_TIMESTAMP WHERE status='running'; {}; COMMIT;",
                native_history_query(backend)
            ),
        )?,
        Backend::Postgres => zelyra_database::query_postgres(
            database_url,
            &format!(
                "BEGIN; SELECT pg_advisory_xact_lock(hashtext('zelyra-schema-migration'), hashtext(current_database())); UPDATE _zelyra_schema_history SET status='interrupted', updated_at=CURRENT_TIMESTAMP, finished_at=CURRENT_TIMESTAMP WHERE status='running'; {}; COMMIT;",
                native_history_query(backend)
            ),
        )?,
        Backend::MariaDb => unreachable!("native history is only used for SQLite and PostgreSQL"),
    };
    let json_line = result
        .lines()
        .find(|line| line.trim_start().starts_with('['))
        .unwrap_or(result.trim());
    serde_json::from_str(json_line).map_err(|error| DatabaseError {
        message: format!("database returned invalid migration history JSON: {error}"),
    })
}

fn apply_native_plan(
    backend: Backend,
    database_url: &str,
    sql: &str,
    plan_id: &str,
    before_fingerprint: &str,
    desired_fingerprint: &str,
    change_count: usize,
) -> Result<(), DatabaseError> {
    ensure_native_schema_history(backend, database_url)?;
    let now = "CURRENT_TIMESTAMP";
    let start = format!(
        "INSERT INTO _zelyra_schema_history \
         (plan_id, before_schema_sha256, desired_schema_sha256, status, attempt_count, change_count, completed_changes, started_at, updated_at) \
         VALUES ({}, {}, {}, 'running', 1, {change_count}, 0, {now}, {now}) \
         ON CONFLICT(plan_id) DO UPDATE SET before_schema_sha256=excluded.before_schema_sha256, \
         desired_schema_sha256=excluded.desired_schema_sha256, status='running', \
         attempt_count=_zelyra_schema_history.attempt_count+1, change_count=excluded.change_count, \
         completed_changes=0, current_change_index=NULL, current_change_description=NULL, \
         started_at={now}, updated_at={now}, finished_at=NULL",
        native_sql_literal(plan_id), native_sql_literal(before_fingerprint),
        native_sql_literal(desired_fingerprint),
    );
    match backend {
        Backend::Sqlite => {
            zelyra_database::query_sqlite(database_url, &start)?;
        }
        Backend::Postgres => {
            apply_postgres(database_url, &start)?;
        }
        Backend::MariaDb => unreachable!("native history is only used for SQLite and PostgreSQL"),
    }

    let complete = format!(
        "UPDATE _zelyra_schema_history SET status='applied', completed_changes={change_count}, \
         current_change_index=NULL, current_change_description=NULL, updated_at=CURRENT_TIMESTAMP, \
         finished_at=CURRENT_TIMESTAMP WHERE plan_id={}",
        native_sql_literal(plan_id)
    );
    let migration = match backend {
        Backend::Sqlite => format!("{sql}\n{complete};"),
        Backend::Postgres => format!(
            "SELECT pg_advisory_xact_lock(hashtext('zelyra-schema-migration'), hashtext(current_database()));\n{sql}\n{complete};"
        ),
        Backend::MariaDb => unreachable!("native history is only used for SQLite and PostgreSQL"),
    };
    let result = match backend {
        Backend::Sqlite => apply_sqlite(database_url, &migration),
        Backend::Postgres => apply_postgres(database_url, &migration),
        Backend::MariaDb => unreachable!("native history is only used for SQLite and PostgreSQL"),
    };
    if let Err(error) = result {
        let failed = format!(
            "UPDATE _zelyra_schema_history SET status='failed', updated_at=CURRENT_TIMESTAMP, \
             finished_at=CURRENT_TIMESTAMP WHERE plan_id={}",
            native_sql_literal(plan_id)
        );
        let _ = match backend {
            Backend::Sqlite => zelyra_database::query_sqlite(database_url, &failed).map(|_| ()),
            Backend::Postgres => apply_postgres(database_url, &failed),
            Backend::MariaDb => {
                unreachable!("native history is only used for SQLite and PostgreSQL")
            }
        };
        return Err(error);
    }
    Ok(())
}

fn print_native_migration_history(
    backend: Backend,
    database_url: &str,
    json_format: bool,
) -> Result<(), DatabaseError> {
    let history = read_native_migration_history(backend, database_url)?;
    if json_format {
        println!(
            "{}",
            serde_json::to_string_pretty(&history).expect("history JSON is serializable")
        );
    } else if let Some(entries) = history.as_array() {
        if entries.is_empty() {
            println!("No schema migrations recorded.");
        }
        for entry in entries {
            println!(
                "{}  {}  {}/{} changes  {}",
                entry["status"].as_str().unwrap_or("unknown"),
                entry["plan_id"].as_str().unwrap_or("unknown plan"),
                entry["completed_changes"].as_u64().unwrap_or_default(),
                entry["change_count"].as_u64().unwrap_or_default(),
                entry["started_at"].as_str().unwrap_or("unknown time")
            );
        }
    }
    Ok(())
}

fn database_map_column_json(column: &zelyra_database::Column) -> Value {
    json!({
        "name": column.name,
        "type": column.sql_type,
        "nullable": column.nullable,
        "primary_key": column.primary_key,
        "auto_increment": column.auto,
        "unique": column.unique,
    })
}

fn database_map_table_json(
    table: Option<&zelyra_database::Table>,
    live_table: Option<&zelyra_database::Table>,
    module: &str,
    status: &str,
) -> Value {
    let mut declared_columns = table
        .into_iter()
        .flat_map(|table| table.columns.iter())
        .collect::<Vec<_>>();
    declared_columns.sort_by(|left, right| left.name.cmp(&right.name));
    let mut live_columns = live_table
        .into_iter()
        .flat_map(|table| table.columns.iter())
        .collect::<Vec<_>>();
    live_columns.sort_by(|left, right| left.name.cmp(&right.name));
    let mut foreign_keys = table
        .into_iter()
        .flat_map(|table| table.foreign_keys.iter())
        .collect::<Vec<_>>();
    foreign_keys.sort_by_key(|key| {
        (
            key.column.as_str(),
            key.referenced_table.as_str(),
            key.referenced_column.as_str(),
        )
    });
    let mut live_foreign_keys = live_table
        .into_iter()
        .flat_map(|table| table.foreign_keys.iter())
        .collect::<Vec<_>>();
    live_foreign_keys.sort_by_key(|key| {
        (
            key.column.as_str(),
            key.referenced_table.as_str(),
            key.referenced_column.as_str(),
        )
    });
    json!({
        "name": table.map(|table| table.name.as_str()).or_else(|| live_table.map(|table| table.name.as_str())),
        "module": module,
        "status": status,
        "declared_columns": declared_columns.iter().map(|column| database_map_column_json(column)).collect::<Vec<_>>(),
        "live_columns": live_columns.iter().map(|column| database_map_column_json(column)).collect::<Vec<_>>(),
        "foreign_keys": foreign_keys.iter().map(|key| json!({
            "column": key.column,
            "references_table": key.referenced_table,
            "references_column": key.referenced_column,
        })).collect::<Vec<_>>(),
        "live_foreign_keys": live_foreign_keys.iter().map(|key| json!({
            "column": key.column,
            "references_table": key.referenced_table,
            "references_column": key.referenced_column,
        })).collect::<Vec<_>>(),
    })
}

pub(super) fn database_map_json(
    declared: &Schema,
    live: &Schema,
    owners: &HashMap<String, String>,
) -> Value {
    let live_tables = live
        .tables
        .iter()
        .map(|table| (table.name.as_str(), table))
        .collect::<HashMap<_, _>>();
    let declared_names = declared
        .tables
        .iter()
        .map(|table| table.name.as_str())
        .collect::<HashSet<_>>();
    let mut modules = BTreeMap::<String, Vec<Value>>::new();
    let mut declared_tables = declared.tables.iter().collect::<Vec<_>>();
    declared_tables.sort_by(|left, right| left.name.cmp(&right.name));
    for table in declared_tables {
        let module = owners
            .get(&format!("table:{}", table.name))
            .cloned()
            .unwrap_or_else(|| "(source module unknown)".into());
        let status = if live_tables.contains_key(table.name.as_str()) {
            "matched"
        } else {
            "declared_missing_live"
        };
        modules
            .entry(module.clone())
            .or_default()
            .push(database_map_table_json(
                Some(table),
                live_tables.get(table.name.as_str()).copied(),
                &module,
                status,
            ));
    }
    let mut live_only = live
        .tables
        .iter()
        .filter(|table| !declared_names.contains(table.name.as_str()))
        .collect::<Vec<_>>();
    live_only.sort_by(|left, right| left.name.cmp(&right.name));
    let unmapped_live_tables = live_only
        .iter()
        .map(|table| database_map_table_json(None, Some(table), "(unmapped)", "live_unmapped"))
        .collect::<Vec<_>>();
    json!({
        "schema_version": 1,
        "read_only": true,
        "ownership_enforced": false,
        "backend": declared.backend().name(),
        "database": declared.database.as_ref().and_then(|database| database.database.clone()),
        "modules": modules.into_iter().map(|(path, tables)| json!({
            "source_module": path,
            "tables": tables,
        })).collect::<Vec<_>>(),
        "unmapped_live_tables": unmapped_live_tables,
    })
}

fn print_database_map(declared: &Schema, live: &Schema, owners: &HashMap<String, String>) {
    println!("Database map (read-only; module ownership is advisory)");
    println!("Backend: {}", declared.backend().name());
    if let Some(database) = declared
        .database
        .as_ref()
        .and_then(|database| database.database.as_deref())
    {
        println!("Database: {database}");
    }
    let live_names = live
        .tables
        .iter()
        .map(|table| table.name.as_str())
        .collect::<HashSet<_>>();
    let declared_names = declared
        .tables
        .iter()
        .map(|table| table.name.as_str())
        .collect::<HashSet<_>>();
    let mut modules = BTreeMap::<String, Vec<&zelyra_database::Table>>::new();
    for table in &declared.tables {
        let module = owners
            .get(&format!("table:{}", table.name))
            .cloned()
            .unwrap_or_else(|| "(source module unknown)".into());
        modules.entry(module).or_default().push(table);
    }
    for (module, mut tables) in modules {
        tables.sort_by(|left, right| left.name.cmp(&right.name));
        println!("\nSource module: {module}");
        for table in tables {
            let status = if live_names.contains(table.name.as_str()) {
                "matched"
            } else {
                "declared, missing from live database"
            };
            println!("  {} [{status}]", table.name);
            let mut columns = table.columns.iter().collect::<Vec<_>>();
            columns.sort_by(|left, right| left.name.cmp(&right.name));
            for column in columns {
                println!("    {}: {}", column.name, column.sql_type);
            }
            for key in &table.foreign_keys {
                println!(
                    "    {} -> {}.{}",
                    key.column, key.referenced_table, key.referenced_column
                );
            }
        }
    }
    let mut unmapped = live
        .tables
        .iter()
        .filter(|table| !declared_names.contains(table.name.as_str()))
        .collect::<Vec<_>>();
    unmapped.sort_by(|left, right| left.name.cmp(&right.name));
    println!("\nLive tables without a source declaration:");
    if unmapped.is_empty() {
        println!("  (none)");
    } else {
        for table in unmapped {
            println!("  {}", table.name);
            let mut columns = table.columns.iter().collect::<Vec<_>>();
            columns.sort_by(|left, right| left.name.cmp(&right.name));
            for column in columns {
                println!("    {}: {}", column.name, column.sql_type);
            }
        }
    }
    println!("\nThis map does not enforce database ownership or change the schema.");
}

pub(super) fn database_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(subcommand) = args.next() else {
        database_usage();
        return ExitCode::from(2);
    };
    let path = args.next().unwrap_or_else(|| "main.zyl".into());
    let remaining_args = args.collect::<Vec<_>>();
    let allow_risky = remaining_args.iter().any(|arg| arg == "--allow-risky");
    let allow_destructive = remaining_args
        .iter()
        .any(|arg| arg == "--allow-destructive");
    let schema = match load_schema(&path) {
        Ok(schema) => schema,
        Err(()) => return ExitCode::from(1),
    };
    if schema.tables.iter().any(|table| {
        table
            .name
            .eq_ignore_ascii_case(MARIADB_SCHEMA_HISTORY_TABLE)
    }) {
        eprintln!(
            "error[E-DB-001]: `{MARIADB_SCHEMA_HISTORY_TABLE}` is reserved for Zelyra migration history"
        );
        return ExitCode::from(1);
    }
    match subcommand.as_str() {
        "create" => {
            println!("{}", schema.create_sql());
            ExitCode::SUCCESS
        }
        "setup" | "bootstrap" => {
            let Some(url) = database_url_from_schema(&schema) else {
                eprintln!(
                    "error[E-DB-003]: DATABASE_URL is required for db {}",
                    subcommand
                );
                if subcommand == "setup" {
                    eprintln!("hint: set a MariaDB URL without committing it to source control");
                    eprintln!(
                        "  export DATABASE_URL='mariadb://user:<password>@127.0.0.1:3306/my_app'"
                    );
                    eprintln!("  # PowerShell: $env:DATABASE_URL = 'mariadb://user:<password>@127.0.0.1:3306/my_app'");
                }
                return ExitCode::from(1);
            };
            let result = match schema.backend() {
                Backend::MariaDb => match inspect_mariadb(&url) {
                    Ok(_) => apply_mariadb(&url, &schema.create_sql()),
                    Err(_) => create_mariadb_database(&url)
                        .and_then(|()| apply_mariadb(&url, &schema.create_sql())),
                },
                Backend::Sqlite => apply_sqlite(&url, &schema.create_sql()),
                Backend::Postgres => Err(zelyra_database::DatabaseError {
                    message: format!(
                        "db {} currently supports mariadb and sqlite; use db apply for postgres",
                        subcommand
                    ),
                }),
            };
            match result {
                Ok(()) => {
                    println!("database {} completed successfully", subcommand);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error[E-DB-005]: {error}");
                    ExitCode::from(1)
                }
            }
        }
        "inspect" => match database_url_from_schema(&schema) {
            Some(url) => match inspect_for_backend(schema.backend(), &url) {
                Ok(current) => {
                    println!("{}", current.summary());
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error[E-DB-002]: {error}");
                    ExitCode::from(1)
                }
            },
            None => {
                eprintln!("error[E-DB-003]: DATABASE_URL is required for db inspect");
                ExitCode::from(1)
            }
        },
        "map" => {
            let json_format = match remaining_args.as_slice() {
                [] => false,
                [format] if format == "--format=text" => false,
                [format] if format == "--format=json" => true,
                _ => {
                    database_usage();
                    return ExitCode::from(2);
                }
            };
            let Some(url) = database_url_from_schema(&schema) else {
                eprintln!("error[E-DB-003]: DATABASE_URL is required for db map");
                return ExitCode::from(1);
            };
            let live = match inspect_for_backend(schema.backend(), &url) {
                Ok(current) => current,
                Err(error) => {
                    eprintln!("error[E-DB-002]: {error}");
                    return ExitCode::from(1);
                }
            };
            let project = match load_project(&path) {
                Ok(project) => project,
                Err(()) => return ExitCode::from(1),
            };
            let owners = module_declaration_owners(&project.program);
            if json_format {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&database_map_json(&schema, &live, &owners))
                        .expect("database map JSON is serializable")
                );
            } else {
                print_database_map(&schema, &live, &owners);
            }
            ExitCode::SUCCESS
        }
        "history" => {
            let json_format =
                if remaining_args.is_empty() || remaining_args.as_slice() == ["--format=text"] {
                    false
                } else if remaining_args.as_slice() == ["--format=json"] {
                    true
                } else {
                    database_usage();
                    return ExitCode::from(2);
                };
            let Some(url) = database_url_from_schema(&schema) else {
                eprintln!("error[E-DB-003]: DATABASE_URL is required for db history");
                return ExitCode::from(1);
            };
            let result = match schema.backend() {
                Backend::MariaDb => print_mariadb_migration_history(&url, json_format),
                backend => print_native_migration_history(backend, &url, json_format),
            };
            match result {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("error[E-DB-005]: {error}");
                    ExitCode::from(1)
                }
            }
        }
        "plan" => {
            let json_format = match remaining_args.as_slice() {
                [] => false,
                [format] if format == "--format=text" => false,
                [format] if format == "--format=json" => true,
                _ => {
                    database_usage();
                    return ExitCode::from(2);
                }
            };
            let current = match database_url_from_schema(&schema) {
                Some(url) => match inspect_for_backend(schema.backend(), &url) {
                    Ok(current) => current,
                    Err(error) => {
                        eprintln!("error[E-DB-002]: {error}");
                        return ExitCode::from(1);
                    }
                },
                None => {
                    eprintln!("note: DATABASE_URL is not set; planning against an empty database");
                    Schema {
                        database: None,
                        tables: Vec::new(),
                    }
                }
            };
            let plan = diff(&schema, &current);
            if json_format {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&schema_plan_json(&schema, &current, &plan))
                        .expect("schema plan JSON is serializable")
                );
            } else {
                print_plan(&plan);
            }
            ExitCode::SUCCESS
        }
        "apply" => {
            let mut expected_plan_id = None;
            let mut index = 0;
            while index < remaining_args.len() {
                match remaining_args[index].as_str() {
                    "--allow-risky" | "--allow-destructive" => index += 1,
                    "--plan-id" => {
                        let Some(value) = remaining_args.get(index + 1) else {
                            database_usage();
                            return ExitCode::from(2);
                        };
                        if expected_plan_id.replace(value.as_str()).is_some() {
                            database_usage();
                            return ExitCode::from(2);
                        }
                        index += 2;
                    }
                    value if value.starts_with("--plan-id=") => {
                        let value = &value["--plan-id=".len()..];
                        if value.is_empty() || expected_plan_id.replace(value).is_some() {
                            database_usage();
                            return ExitCode::from(2);
                        }
                        index += 1;
                    }
                    _ => {
                        database_usage();
                        return ExitCode::from(2);
                    }
                }
            }
            let Some(url) = database_url_from_schema(&schema) else {
                eprintln!("error[E-DB-003]: DATABASE_URL is required for db apply");
                return ExitCode::from(1);
            };
            let current = match inspect_for_backend(schema.backend(), &url) {
                Ok(current) => current,
                Err(error) => {
                    eprintln!("error[E-DB-002]: {error}");
                    return ExitCode::from(1);
                }
            };
            let plan = diff(&schema, &current);
            print_plan(&plan);
            if let Some(expected_plan_id) = expected_plan_id {
                let actual_plan_id = schema_plan_json(&schema, &current, &plan)["plan_id"]
                    .as_str()
                    .expect("schema plan JSON always contains a plan id")
                    .to_owned();
                if expected_plan_id != actual_plan_id {
                    eprintln!("error[E-DB-007]: the database schema changed after this plan was reviewed; no SQL was applied. Run `zelyra db plan --format=json` again and review the new plan id");
                    return ExitCode::from(1);
                }
            }
            if plan.has_unsupported() {
                eprintln!(
                    "error[E-DB-006]: schema plan contains unsupported changes; no SQL was applied"
                );
                return ExitCode::from(1);
            }
            let has_review_changes = plan
                .changes
                .iter()
                .any(|change| change.risk == Risk::RequiresApproval);
            let legacy_approval_is_sufficient = allow_destructive && !has_review_changes;
            if plan.requires_approval() && !allow_risky && !legacy_approval_is_sufficient {
                eprintln!("error[E-DB-004]: schema changes requiring review were refused; review the plan and use --allow-risky to approve it");
                return ExitCode::from(1);
            }
            if plan.changes.is_empty() {
                return ExitCode::SUCCESS;
            }
            for check in &plan.nullability_preflights {
                match count_null_values(&url, schema.backend(), &check.table, &check.column) {
                    Ok(0) => {}
                    Ok(_) => {
                        eprintln!(
                            "error[E-DB-005]: cannot require `{}.{}` because existing rows contain NULL values; no schema SQL was applied",
                            check.table, check.column
                        );
                        return ExitCode::from(1);
                    }
                    Err(_) => {
                        eprintln!(
                            "error[E-DB-005]: could not verify that `{}.{}` contains no NULL values; no schema SQL was applied",
                            check.table, check.column
                        );
                        return ExitCode::from(1);
                    }
                }
            }
            let mut table_row_presence = HashMap::new();
            for check in &plan.required_column_preflights {
                let has_rows = *table_row_presence
                    .entry(check.table.as_str())
                    .or_insert_with(|| {
                        table_has_rows(&url, schema.backend(), &check.table).map_err(|_| ())
                    });
                match has_rows {
                    Ok(false) => {}
                    Ok(true) => {
                        eprintln!(
                            "error[E-DB-005]: cannot add required column `{}.{}` without a default because the table contains existing rows; no schema SQL was applied. Add a default or stage the change: add it as nullable, backfill the rows, then require it",
                            check.table, check.column
                        );
                        return ExitCode::from(1);
                    }
                    Err(_) => {
                        eprintln!(
                            "error[E-DB-005]: could not verify that table `{}` is empty before adding required column `{}.{}`; no schema SQL was applied",
                            check.table, check.table, check.column
                        );
                        return ExitCode::from(1);
                    }
                }
            }
            for check in &plan.unique_index_preflights {
                match count_duplicate_value_groups(
                    &url,
                    schema.backend(),
                    &check.table,
                    &check.columns,
                ) {
                    Ok(0) => {}
                    Ok(count) => {
                        eprintln!(
                            "error[E-DB-005]: cannot add a unique index on `{}.{}` because {count} duplicate value group(s) exist; no schema SQL was applied",
                            check.table,
                            check.columns.join(", ")
                        );
                        return ExitCode::from(1);
                    }
                    Err(_) => {
                        eprintln!(
                            "error[E-DB-005]: could not verify uniqueness of `{}.{}`; no schema SQL was applied",
                            check.table,
                            check.columns.join(", ")
                        );
                        return ExitCode::from(1);
                    }
                }
            }
            for check in &plan.foreign_key_preflights {
                match count_foreign_key_orphans(&url, schema.backend(), check) {
                    Ok(0) => {}
                    Ok(count) => {
                        eprintln!(
                            "error[E-DB-005]: cannot add foreign key `{}.{}` because {count} existing row(s) have no matching `{}.{}` value; no schema SQL was applied",
                            check.table,
                            check.column,
                            check.referenced_table,
                            check.referenced_column
                        );
                        return ExitCode::from(1);
                    }
                    Err(_) => {
                        eprintln!(
                            "error[E-DB-005]: could not verify foreign-key values in `{}.{}`; no schema SQL was applied",
                            check.table, check.column
                        );
                        return ExitCode::from(1);
                    }
                }
            }
            let actual_plan_id = schema_plan_json(&schema, &current, &plan)["plan_id"]
                .as_str()
                .expect("schema plan JSON always contains a plan id")
                .to_owned();
            let mut sql = plan.sql();
            if schema.backend() == Backend::MariaDb
                && (!plan.nullability_preflights.is_empty()
                    || !plan.required_column_preflights.is_empty())
            {
                sql = format!(
                    "SET SESSION sql_mode = CONCAT_WS(',', NULLIF(@@SESSION.sql_mode, ''), 'STRICT_ALL_TABLES');\n{sql}"
                );
            }
            let plan_json = schema_plan_json(&schema, &current, &plan);
            let before_fingerprint = plan_json["current_schema_sha256"]
                .as_str()
                .expect("schema plan includes current fingerprint");
            let desired_fingerprint = plan_json["desired_schema_sha256"]
                .as_str()
                .expect("schema plan includes desired fingerprint");
            let result = match schema.backend() {
                Backend::Postgres | Backend::Sqlite => apply_native_plan(
                    schema.backend(),
                    &url,
                    &sql,
                    &actual_plan_id,
                    before_fingerprint,
                    desired_fingerprint,
                    plan.changes.len(),
                ),
                Backend::MariaDb => apply_mariadb_plan(&url, &schema, &plan, &actual_plan_id),
            };
            match result {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    if let Some(message) = error.message.strip_prefix("E-DB-007: ") {
                        eprintln!("error[E-DB-007]: {message}");
                    } else {
                        eprintln!("error[E-DB-005]: {error}");
                    }
                    ExitCode::from(1)
                }
            }
        }
        _ => {
            database_usage();
            ExitCode::from(2)
        }
    }
}

pub(super) fn inspect_for_backend(
    backend: Backend,
    database_url: &str,
) -> Result<Schema, zelyra_database::DatabaseError> {
    match backend {
        Backend::Postgres => inspect_postgres(database_url),
        Backend::MariaDb => inspect_mariadb(database_url),
        Backend::Sqlite => inspect_sqlite(database_url),
    }
}
