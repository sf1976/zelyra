use super::*;

fn auth_usage() {
    eprintln!(
        "Usage:\n  zelyra auth hash-password\n  zelyra auth hash-password --stdin\n  zelyra auth role grant <file.zyl> <user-id> <role>\n  zelyra auth role revoke <file.zyl> <user-id> <role>\n  zelyra auth role-permission grant <file.zyl> <role> <permission>\n  zelyra auth role-permission revoke <file.zyl> <role> <permission>\n\nRole commands use ZELYRA_DATABASE_<NAME>_URL (with DATABASE_URL as compatibility fallback) and the role tables declared in the first auth definition.\nThe interactive password form does not echo passwords. Use --stdin for automation."
    );
}

#[derive(Clone, Debug)]
struct AuthRoleTables {
    assignments: String,
    permissions: String,
    audit: Option<String>,
    audit_chain: bool,
}

fn auth_role_tables(path: &str) -> Result<AuthRoleTables, ExitCode> {
    let program = match validate(path) {
        Ok(program) => program,
        Err(()) => return Err(ExitCode::from(1)),
    };
    let Some(auth) = program.auth.first() else {
        eprintln!("error[E-AUTH-014]: role commands require an auth definition");
        return Err(ExitCode::from(1));
    };
    let (Some(assignments), Some(permissions)) = (
        auth.roles_table.clone(),
        auth.role_permissions_table.clone(),
    ) else {
        eprintln!(
            "error[E-AUTH-015]: role commands require roles and role_permissions in the auth definition"
        );
        return Err(ExitCode::from(1));
    };
    Ok(AuthRoleTables {
        assignments,
        permissions,
        audit: auth.audit_table.clone(),
        audit_chain: auth.audit_chain,
    })
}

fn auth_role_database_url(path: &str) -> Result<String, ExitCode> {
    let program = match validate(path) {
        Ok(program) => program,
        Err(()) => return Err(ExitCode::from(1)),
    };
    match database_url_from_program(&program) {
        Some(url) if url.starts_with("mariadb://") || url.starts_with("mysql://") => Ok(url),
        Some(_) => {
            eprintln!("error[E-AUTH-016]: role commands require a MariaDB database URL");
            Err(ExitCode::from(1))
        }
        None => {
            eprintln!("error[E-AUTH-017]: a database URL is required for role commands");
            Err(ExitCode::from(1))
        }
    }
}

fn execute_auth_role_mutation(
    database_url: &str,
    sql: String,
    params: Vec<(String, QueryValue)>,
    audit: Option<(&str, bool)>,
    event: &str,
    target_user_id: Option<i64>,
    details: String,
) -> Result<(), zelyra_database::DatabaseError> {
    let mut queries = vec![Query { sql, params }];
    if let Some((audit_table, audit_chain)) = audit {
        queries.extend(audit_insert_queries(
            audit_table,
            audit_chain,
            None,
            event,
            target_user_id,
            &details,
        ));
    }
    zelyra_database::execute_mariadb_queries(database_url, &queries, true).map(|_| ())
}

fn auth_role_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(operation) = args.next() else {
        auth_usage();
        return ExitCode::from(2);
    };
    let Some(path) = args.next() else {
        auth_usage();
        return ExitCode::from(2);
    };
    let Some(user_id) = args.next().and_then(|value| value.parse::<i64>().ok()) else {
        eprintln!("error[E-AUTH-018]: user-id must be an integer");
        return ExitCode::from(2);
    };
    let Some(role) = args.next() else {
        auth_usage();
        return ExitCode::from(2);
    };
    if args.next().is_some() || role.is_empty() || !matches!(operation.as_str(), "grant" | "revoke")
    {
        auth_usage();
        return ExitCode::from(2);
    }
    let tables = match auth_role_tables(&path) {
        Ok(tables) => tables,
        Err(code) => return code,
    };
    let database_url = match auth_role_database_url(&path) {
        Ok(url) => url,
        Err(code) => return code,
    };
    let (sql, message) = if operation == "grant" {
        (
            format!(
                "INSERT INTO {} (user_id, role) SELECT :user_id, :role FROM DUAL WHERE NOT EXISTS (SELECT 1 FROM {} WHERE user_id = :user_id AND role = :role)",
                quote_identifier(&tables.assignments),
                quote_identifier(&tables.assignments),
            ),
            "role granted",
        )
    } else {
        (
            format!(
                "DELETE FROM {} WHERE user_id = :user_id AND role = :role",
                quote_identifier(&tables.assignments),
            ),
            "role revoked",
        )
    };
    let event = format!("role.{operation}");
    let details = format!("source=cli;role={role}");
    match execute_auth_role_mutation(
        &database_url,
        sql,
        vec![
            ("user_id".into(), QueryValue::Int(user_id)),
            ("role".into(), QueryValue::String(role.clone())),
        ],
        tables
            .audit
            .as_deref()
            .map(|table| (table, tables.audit_chain)),
        &event,
        Some(user_id),
        details,
    ) {
        Ok(_) => {
            println!("{message}: user {user_id} -> {role}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error[E-AUTH-019]: cannot change role assignment: {error}");
            ExitCode::from(1)
        }
    }
}

fn auth_role_permission_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(operation) = args.next() else {
        auth_usage();
        return ExitCode::from(2);
    };
    let Some(path) = args.next() else {
        auth_usage();
        return ExitCode::from(2);
    };
    let Some(role) = args.next() else {
        auth_usage();
        return ExitCode::from(2);
    };
    let Some(permission) = args.next() else {
        auth_usage();
        return ExitCode::from(2);
    };
    if args.next().is_some()
        || role.is_empty()
        || permission.is_empty()
        || !matches!(operation.as_str(), "grant" | "revoke")
    {
        auth_usage();
        return ExitCode::from(2);
    }
    let tables = match auth_role_tables(&path) {
        Ok(tables) => tables,
        Err(code) => return code,
    };
    let database_url = match auth_role_database_url(&path) {
        Ok(url) => url,
        Err(code) => return code,
    };
    let (sql, message) = if operation == "grant" {
        (
            format!(
                "INSERT INTO {} (role, permission) SELECT :role, :permission FROM DUAL WHERE NOT EXISTS (SELECT 1 FROM {} WHERE role = :role AND permission = :permission)",
                quote_identifier(&tables.permissions),
                quote_identifier(&tables.permissions),
            ),
            "permission granted",
        )
    } else {
        (
            format!(
                "DELETE FROM {} WHERE role = :role AND permission = :permission",
                quote_identifier(&tables.permissions),
            ),
            "permission revoked",
        )
    };
    let event = format!("role_permission.{operation}");
    let details = format!("source=cli;role={role};permission={permission}");
    match execute_auth_role_mutation(
        &database_url,
        sql,
        vec![
            ("role".into(), QueryValue::String(role.clone())),
            ("permission".into(), QueryValue::String(permission.clone())),
        ],
        tables
            .audit
            .as_deref()
            .map(|table| (table, tables.audit_chain)),
        &event,
        None,
        details,
    ) {
        Ok(_) => {
            println!("{message}: {role} -> {permission}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error[E-AUTH-020]: cannot change role permission: {error}");
            ExitCode::from(1)
        }
    }
}

fn audit_usage() {
    eprintln!(
        "Usage:\n  zelyra audit inspect <file.zyl> [--limit <n>]\n  zelyra audit export <file.zyl> [--limit <n>] [--format json|csv]\n  zelyra audit verify <file.zyl>\n  zelyra audit prune <file.zyl> --before <timestamp> [--confirm]\n\nAudit commands use ZELYRA_DATABASE_<NAME>_URL (with DATABASE_URL as compatibility fallback) and the audit table declared in the first auth definition. The default limit is 100 and the maximum is 10,000. Prune never changes data without --confirm."
    );
}

fn audit_project(path: &str) -> Result<(String, String, bool), ExitCode> {
    let program = match validate(path) {
        Ok(program) => program,
        Err(()) => return Err(ExitCode::from(1)),
    };
    let Some(auth) = program.auth.first() else {
        eprintln!("error[E-AUDIT-001]: audit commands require an auth definition");
        return Err(ExitCode::from(1));
    };
    let Some(audit_table) = auth.audit_table.clone() else {
        eprintln!(
            "error[E-AUDIT-001]: audit commands require audit: <table> in the auth definition"
        );
        return Err(ExitCode::from(1));
    };
    let database_url = match database_url_from_program(&program) {
        Some(url) if url.starts_with("mariadb://") || url.starts_with("mysql://") => url,
        Some(_) => {
            eprintln!("error[E-AUDIT-002]: audit commands require a MariaDB database URL");
            return Err(ExitCode::from(1));
        }
        None => {
            eprintln!("error[E-AUDIT-003]: a database URL is required for audit commands");
            return Err(ExitCode::from(1));
        }
    };
    Ok((database_url, audit_table, auth.audit_chain))
}

fn audit_limit(value: &str) -> Result<usize, ExitCode> {
    match value.parse::<usize>() {
        Ok(limit) if (1..=10_000).contains(&limit) => Ok(limit),
        _ => {
            eprintln!("error[E-AUDIT-004]: limit must be an integer between 1 and 10000");
            Err(ExitCode::from(2))
        }
    }
}

fn audit_rows(
    database_url: &str,
    audit_table: &str,
    limit: usize,
) -> Result<QueryResult, zelyra_database::DatabaseError> {
    zelyra_database::execute_mariadb_query(
        database_url,
        &format!(
            "SELECT actor_user_id, event, target_user_id, details, created_at FROM {} ORDER BY created_at DESC LIMIT {limit}",
            quote_identifier(audit_table)
        ),
        Vec::new(),
    )
}

fn audit_integrity(
    database_url: &str,
    audit_table: &str,
    chain: bool,
) -> Result<(u64, u64), zelyra_database::DatabaseError> {
    let query = if chain {
        format!(
            "SELECT COUNT(*), COALESCE(SUM(CASE WHEN event IS NULL OR event = '' OR details IS NULL OR created_at IS NULL OR previous_hash IS NULL OR entry_hash IS NULL OR previous_hash <> COALESCE(expected_previous_hash, '') OR entry_hash <> SHA2(CONCAT(COALESCE(previous_hash, ''), '|', COALESCE(actor_user_id, 'NULL'), '|', event, '|', COALESCE(target_user_id, 'NULL'), '|', details, '|', DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s')), 256) THEN 1 ELSE 0 END), 0) FROM (SELECT id, actor_user_id, event, target_user_id, details, created_at, previous_hash, entry_hash, LAG(entry_hash) OVER (ORDER BY id ASC) AS expected_previous_hash FROM {}) AS audit_rows",
            quote_identifier(audit_table)
        )
    } else {
        format!(
            "SELECT COUNT(*), COALESCE(SUM(CASE WHEN event IS NULL OR event = '' OR details IS NULL OR created_at IS NULL THEN 1 ELSE 0 END), 0) FROM {}",
            quote_identifier(audit_table)
        )
    };
    let result = zelyra_database::execute_mariadb_query(database_url, &query, Vec::new())?;
    let row = result.rows.first().cloned().unwrap_or_default();
    let total = row
        .first()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let invalid = row.get(1).and_then(|value| value.parse().ok()).unwrap_or(0);
    Ok((total, invalid))
}

fn audit_prune_count(
    database_url: &str,
    audit_table: &str,
    before: &str,
) -> Result<u64, zelyra_database::DatabaseError> {
    let result = zelyra_database::execute_mariadb_query(
        database_url,
        &format!(
            "SELECT COUNT(*) FROM {} WHERE created_at < :before",
            quote_identifier(audit_table)
        ),
        vec![("before".into(), QueryValue::String(before.into()))],
    )?;
    Ok(result
        .rows
        .first()
        .and_then(|row| row.first())
        .and_then(|value| value.parse().ok())
        .unwrap_or(0))
}

fn audit_prune(
    database_url: &str,
    audit_table: &str,
    before: &str,
) -> Result<(), zelyra_database::DatabaseError> {
    let details = format!("source=cli;before={before}");
    let queries = vec![
        Query {
            sql: format!(
                "DELETE FROM {} WHERE created_at < :before",
                quote_identifier(audit_table)
            ),
            params: vec![("before".into(), QueryValue::String(before.into()))],
        },
        Query {
            sql: format!(
                "INSERT INTO {} (actor_user_id, event, target_user_id, details) VALUES (:actor_user_id, :event, :target_user_id, :details)",
                quote_identifier(audit_table)
            ),
            params: vec![
                ("actor_user_id".into(), QueryValue::Null),
                ("event".into(), QueryValue::String("audit.prune".into())),
                ("target_user_id".into(), QueryValue::Null),
                (
                    "details".into(),
                    QueryValue::String(details.chars().take(1000).collect()),
                ),
            ],
        },
    ];
    zelyra_database::execute_mariadb_queries(database_url, &queries, true).map(|_| ())
}

fn audit_optional_value(row: &[String], index: usize) -> Option<&str> {
    row.get(index)
        .map(String::as_str)
        .filter(|value| *value != "NULL")
}

fn audit_csv_value(value: Option<&str>) -> String {
    let value = value.unwrap_or_default().replace('"', "\"\"");
    format!("\"{value}\"")
}

pub(super) fn audit_rows_csv(result: &QueryResult) -> String {
    let mut output = String::from("actor_user_id,event,target_user_id,details,created_at\n");
    for row in &result.rows {
        let fields = (0..5)
            .map(|index| audit_csv_value(audit_optional_value(row, index)))
            .collect::<Vec<_>>();
        let _ = writeln!(output, "{}", fields.join(","));
    }
    output
}

fn audit_json_number(value: Option<&str>) -> String {
    value
        .and_then(|value| value.parse::<i64>().ok())
        .map_or_else(|| "null".into(), |value| value.to_string())
}

pub(super) fn audit_rows_json(result: &QueryResult) -> String {
    let rows = result
        .rows
        .iter()
        .map(|row| {
            format!(
                "{{\"actor_user_id\":{},\"event\":{},\"target_user_id\":{},\"details\":{},\"created_at\":{}}}",
                audit_json_number(audit_optional_value(row, 0)),
                audit_optional_value(row, 1).map_or_else(|| "null".into(), |value| format!("\"{}\"", json_escape(value))),
                audit_json_number(audit_optional_value(row, 2)),
                audit_optional_value(row, 3).map_or_else(|| "null".into(), |value| format!("\"{}\"", json_escape(value))),
                audit_optional_value(row, 4).map_or_else(|| "null".into(), |value| format!("\"{}\"", json_escape(value))),
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", rows.join(","))
}

fn audit_rows_inspect(result: &QueryResult) -> String {
    let mut output = format!(
        "Audit log: {} entr{}\n",
        result.rows.len(),
        if result.rows.len() == 1 { "y" } else { "ies" }
    );
    output.push_str("actor_user_id | event | target_user_id | details | created_at\n");
    for row in &result.rows {
        let fields = (0..5)
            .map(|index| {
                audit_optional_value(row, index)
                    .unwrap_or("-")
                    .replace(['\n', '\r', '\t'], " ")
            })
            .collect::<Vec<_>>();
        let _ = writeln!(output, "{}", fields.join(" | "));
    }
    output
}

pub(super) fn audit_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(operation) = args.next() else {
        audit_usage();
        return ExitCode::from(2);
    };
    let Some(path) = args.next() else {
        audit_usage();
        return ExitCode::from(2);
    };
    let mut limit = 100usize;
    let mut limit_given = false;
    let mut format = "inspect";
    let mut before = None;
    let mut confirm = false;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--limit" => {
                let Some(value) = args.next() else {
                    audit_usage();
                    return ExitCode::from(2);
                };
                limit = match audit_limit(&value) {
                    Ok(limit) => limit,
                    Err(code) => return code,
                };
                limit_given = true;
            }
            "--format" if operation == "export" => {
                let Some(value) = args.next() else {
                    audit_usage();
                    return ExitCode::from(2);
                };
                if !matches!(value.as_str(), "json" | "csv") {
                    eprintln!("error[E-AUDIT-005]: format must be json or csv");
                    return ExitCode::from(2);
                }
                format = if value == "json" { "json" } else { "csv" };
            }
            "--before" if operation == "prune" => {
                let Some(value) = args.next() else {
                    audit_usage();
                    return ExitCode::from(2);
                };
                if value.is_empty() {
                    eprintln!("error[E-AUDIT-007]: before timestamp must not be empty");
                    return ExitCode::from(2);
                }
                before = Some(value);
            }
            "--confirm" if operation == "prune" => {
                confirm = true;
            }
            _ => {
                audit_usage();
                return ExitCode::from(2);
            }
        }
    }
    if !matches!(
        operation.as_str(),
        "inspect" | "export" | "verify" | "prune"
    ) {
        audit_usage();
        return ExitCode::from(2);
    }
    if operation == "inspect" && format != "inspect" {
        audit_usage();
        return ExitCode::from(2);
    }
    if operation == "verify" && (format != "inspect" || before.is_some() || confirm || limit_given)
    {
        audit_usage();
        return ExitCode::from(2);
    }
    if operation == "prune" && (before.is_none() || format != "inspect" || limit_given) {
        audit_usage();
        return ExitCode::from(2);
    }
    if operation != "prune" && (before.is_some() || confirm) {
        audit_usage();
        return ExitCode::from(2);
    }
    let (database_url, audit_table, audit_chain) = match audit_project(&path) {
        Ok(project) => project,
        Err(code) => return code,
    };
    if operation == "verify" {
        let (total, invalid) = match audit_integrity(&database_url, &audit_table, audit_chain) {
            Ok(result) => result,
            Err(error) => {
                eprintln!("error[E-AUDIT-006]: cannot verify audit log: {error}");
                return ExitCode::from(1);
            }
        };
        if invalid == 0 {
            println!("Audit log verified: {total} entries, no invalid rows.");
            return ExitCode::SUCCESS;
        }
        eprintln!("error[E-AUDIT-008]: audit log contains {invalid} invalid rows out of {total}");
        return ExitCode::from(1);
    }
    if operation == "prune" {
        if audit_chain {
            eprintln!(
                "error[E-AUDIT-010]: audit prune is disabled for chained audit logs because deleting entries would break the hash chain"
            );
            return ExitCode::from(1);
        }
        let before = before
            .as_deref()
            .expect("prune requires a before timestamp");
        let count = match audit_prune_count(&database_url, &audit_table, before) {
            Ok(count) => count,
            Err(error) => {
                eprintln!("error[E-AUDIT-006]: cannot plan audit prune: {error}");
                return ExitCode::from(1);
            }
        };
        if !confirm {
            println!("Audit prune plan: {count} entries older than {before} would be removed.");
            println!("No changes applied. Re-run with --confirm to apply this plan.");
            return ExitCode::from(2);
        }
        if let Err(error) = audit_prune(&database_url, &audit_table, before) {
            eprintln!("error[E-AUDIT-009]: cannot apply audit prune: {error}");
            return ExitCode::from(1);
        }
        println!("Audit prune applied: {count} entries older than {before} removed.");
        return ExitCode::SUCCESS;
    }
    let result = match audit_rows(&database_url, &audit_table, limit) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("error[E-AUDIT-006]: cannot read audit log: {error}");
            return ExitCode::from(1);
        }
    };
    match operation.as_str() {
        "inspect" => print!("{}", audit_rows_inspect(&result)),
        "export" if format == "json" => println!("{}", audit_rows_json(&result)),
        "export" => print!("{}", audit_rows_csv(&result)),
        _ => unreachable!(),
    }
    ExitCode::SUCCESS
}

fn password_from_stdin() -> Result<String, String> {
    let mut password = String::new();
    std::io::stdin()
        .read_line(&mut password)
        .map_err(|error| format!("cannot read password from stdin: {error}"))?;
    Ok(password.trim_end_matches(['\r', '\n']).to_owned())
}

fn auth_hash_password_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let use_stdin = match args.next().as_deref() {
        None => false,
        Some("--stdin") => true,
        Some(_) => {
            auth_usage();
            return ExitCode::from(2);
        }
    };
    if args.next().is_some() {
        auth_usage();
        return ExitCode::from(2);
    }
    let password = if use_stdin {
        match password_from_stdin() {
            Ok(password) => password,
            Err(error) => {
                eprintln!("error[E-AUTH-001]: {error}");
                return ExitCode::from(1);
            }
        }
    } else {
        let password = match rpassword::prompt_password("Password: ") {
            Ok(password) => password,
            Err(error) => {
                eprintln!("error[E-AUTH-001]: cannot read password: {error}");
                return ExitCode::from(1);
            }
        };
        let confirmation = match rpassword::prompt_password("Confirm password: ") {
            Ok(password) => password,
            Err(error) => {
                eprintln!("error[E-AUTH-001]: cannot read password confirmation: {error}");
                return ExitCode::from(1);
            }
        };
        if password != confirmation {
            eprintln!("error[E-AUTH-002]: passwords do not match");
            return ExitCode::from(1);
        }
        password
    };
    match zelyra_web::hash_password(&password) {
        Ok(hash) => {
            println!("{hash}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error[E-AUTH-003]: {error}");
            ExitCode::from(1)
        }
    }
}

pub(super) fn auth_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    match args.next().as_deref() {
        Some("hash-password") => auth_hash_password_command(args),
        Some("role") => auth_role_command(args),
        Some("role-permission") => auth_role_permission_command(args),
        _ => {
            auth_usage();
            ExitCode::from(2)
        }
    }
}
