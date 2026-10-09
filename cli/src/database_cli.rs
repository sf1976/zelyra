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
                diagnostic(
                    &source_path,
                    "E-DB-001",
                    &error.message,
                    error.span.line,
                    error.span.column,
                );
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
    let changes = plan
        .changes
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
        .collect::<Vec<_>>();
    let preflights = plan
        .nullability_preflights
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
        .collect::<Vec<_>>();
    let identity = json!({
        "format": "zelyra.schema-plan/v1",
        "backend": desired.backend().name(),
        "current_schema_sha256": current_fingerprint,
        "desired_schema_sha256": desired_fingerprint,
        "changes": changes,
        "preflights": preflights,
    });
    let identity_bytes =
        serde_json::to_vec(&identity).expect("schema plan identity is serializable");
    let plan_id = format!("sha256:{:x}", Sha256::digest(identity_bytes));
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
            "generated": false,
            "safe_to_apply_automatically": false,
            "hint": if plan.changes.is_empty() {
                "No schema changes; rollback is not applicable."
            } else {
                "No data-safe reverse plan is available. Take and verify an operator-managed backup before applying changes."
            },
        },
        "automatic_retries": false,
    })
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
            let mut sql = plan.sql();
            if schema.backend() == Backend::MariaDb
                && (!plan.nullability_preflights.is_empty()
                    || !plan.required_column_preflights.is_empty())
            {
                sql = format!(
                    "SET SESSION sql_mode = CONCAT_WS(',', NULLIF(@@SESSION.sql_mode, ''), 'STRICT_ALL_TABLES');\n{sql}"
                );
            }
            let result = match schema.backend() {
                Backend::Postgres => apply_postgres(&url, &sql),
                Backend::MariaDb => apply_mariadb(&url, &sql),
                Backend::Sqlite => apply_sqlite(&url, &sql),
            };
            match result {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("error[E-DB-005]: {error}");
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
