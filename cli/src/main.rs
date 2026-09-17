use std::{collections::HashSet, env, fs, process::ExitCode};
use zelyra_database::{
    apply_mariadb, apply_postgres, apply_sqlite, build_schema, create_mariadb_database, diff,
    inspect_mariadb, inspect_postgres, inspect_sqlite, sql::check_program as check_sql_program,
    Backend, Risk, Schema,
};
use zelyra_forms::{check_program as check_form_program, validate as validate_form};
use zelyra_hir::lower;
use zelyra_lexer::lex;
use zelyra_parser::parse;
use zelyra_runtime::{
    check, check_capabilities_with_grants, execute, execute_with_database,
    verify as verify_program, VerificationResult, VerificationStatus, KNOWN_CAPABILITIES,
};
use zelyra_web::{serve_app, AuthRoute, CrudRoute, CsrfProtection, FormRoute, Route, WebApp};

fn usage() {
    eprintln!("Zelyra 0.1\n\nUsage:\n  zelyra new <directory>\n  zelyra init [directory]\n  zelyra check <file.zyl>\n  zelyra build <file.zyl>\n  zelyra run <file.zyl>\n  zelyra serve <file.zyl> [address]\n  zelyra verify <file.zyl> [--json]\n  zelyra form validate <file.zyl> <FormName> [field=value ...]\n  zelyra db <create|bootstrap|inspect|plan|apply> <file.zyl>");
}

fn database_usage() {
    eprintln!(
        "Usage:\n  zelyra db create <file.zyl>\n  zelyra db bootstrap <file.zyl>\n  zelyra db inspect <file.zyl>\n  zelyra db plan <file.zyl>\n  zelyra db apply <file.zyl> [--allow-destructive]\n\nDATABASE_URL is used by bootstrap, inspect, plan, and apply."
    );
}

fn create_project(path: &str, allow_current_directory: bool) -> ExitCode {
    let directory = std::path::Path::new(path);
    if directory.exists() && !allow_current_directory {
        eprintln!("error[E-INIT-001]: directory `{path}` already exists");
        return ExitCode::from(1);
    }
    if let Err(error) = fs::create_dir_all(directory) {
        eprintln!("error[E-INIT-002]: cannot create `{path}`: {error}");
        return ExitCode::from(1);
    }
    let files = [
        (
            "zelyra.toml",
            "[project]\nname = \"zelyra-app\"\nversion = \"0.1.0\"\nzelyra = \"0.1\"\n\n[capabilities]\ndatabase = true\nnetwork = false\n",
        ),
        (
            "main.zyl",
            "fn main() {\n    print(\"Hello from Zelyra\")\n}\n",
        ),
    ];
    for (name, contents) in files {
        let file = directory.join(name);
        if file.exists() && allow_current_directory {
            continue;
        }
        if let Err(error) = fs::write(&file, contents) {
            eprintln!(
                "error[E-INIT-003]: cannot write `{}`: {error}",
                file.display()
            );
            return ExitCode::from(1);
        }
    }
    println!("created Zelyra project in {}", directory.display());
    println!("next: cd {} && zelyra run main.zyl", path);
    ExitCode::SUCCESS
}

fn diagnostic(path: &str, code: &str, message: &str, line: usize, column: usize) {
    eprintln!("error[{code}]: {message}\n\n --> {path}:{line}:{column}");
}

fn load(path: &str) -> Result<zelyra_ast::Program, ()> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error[E-IO-001]: cannot read `{path}`: {error}");
            return Err(());
        }
    };
    let tokens = match lex(&source) {
        Ok(tokens) => tokens,
        Err(error) => {
            diagnostic(
                path,
                "E-LEX-001",
                &error.message,
                error.span.line,
                error.span.column,
            );
            return Err(());
        }
    };
    match parse(&tokens) {
        Ok(program) => Ok(program),
        Err(error) => {
            diagnostic(
                path,
                "E-PARSE-001",
                &error.message,
                error.span.line,
                error.span.column,
            );
            Err(())
        }
    }
}

fn validate(path: &str) -> Result<zelyra_ast::Program, ()> {
    let program = load(path)?;
    if let Err(errors) = lower(&program) {
        for error in errors {
            diagnostic(
                path,
                "E-NAME-001",
                &error.message,
                error.span.line,
                error.span.column,
            );
        }
        return Err(());
    }
    if !program.functions.is_empty() {
        if let Err(errors) = check(&program) {
            for error in errors {
                diagnostic(
                    path,
                    "E-TYPE-001",
                    &error.message,
                    error.span.line,
                    error.span.column,
                );
            }
            return Err(());
        }
    }
    if validate_capabilities(path, &program).is_err() {
        return Err(());
    }
    if let Ok(schema) = build_schema(&program) {
        if !validate_auth(path, &program, &schema) {
            return Err(());
        }
        if !validate_cruds(path, &program, &schema) {
            return Err(());
        }
        if let Err(errors) = check_sql_program(&program, &schema) {
            for error in errors {
                diagnostic(
                    path,
                    "E-SQL-004",
                    &error.message,
                    error.span.line,
                    error.span.column,
                );
            }
            return Err(());
        }
        if let Err(errors) = check_form_program(&program, &schema) {
            for error in errors {
                diagnostic(
                    path,
                    "E-FORM-001",
                    &error.message,
                    error.span.line,
                    error.span.column,
                );
            }
            return Err(());
        }
    }
    Ok(program)
}

fn verify_command(path: &str, json: bool) -> ExitCode {
    let program = match validate(path) {
        Ok(program) => program,
        Err(()) => return ExitCode::from(1),
    };
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error[E-IO-001]: cannot read `{path}`: {error}");
            return ExitCode::from(1);
        }
    };
    let results = verify_program(&program);
    let failed = results
        .iter()
        .any(|result| result.status == VerificationStatus::Failed);
    if json {
        println!("{}", format_verification_json(path, &source, &results));
    } else {
        for result in &results {
            println!("{}", format_verification_result(path, &source, result));
        }
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn format_verification_result(path: &str, source: &str, result: &VerificationResult) -> String {
    let (end_line, end_column) = source_position(source, result.span.end);
    format!(
        "{} [{}]: {}.{}[{}] ({}:{}:{}-{}:{})",
        result.status,
        result.status.code(),
        result.function,
        result.kind,
        result.index,
        path,
        result.span.line,
        result.span.column,
        end_line,
        end_column
    )
}

fn format_verification_json(path: &str, source: &str, results: &[VerificationResult]) -> String {
    let entries = results
        .iter()
        .map(|result| {
            let (end_line, end_column) = source_position(source, result.span.end);
            format!(
                "{{\"status\":\"{}\",\"code\":\"{}\",\"function\":\"{}\",\"kind\":\"{}\",\"index\":{},\"location\":{{\"file\":\"{}\",\"start\":{{\"line\":{},\"column\":{}}},\"end\":{{\"line\":{},\"column\":{}}}}}}}",
                result.status,
                result.status.code(),
                json_escape(&result.function),
                result.kind,
                result.index,
                json_escape(path),
                result.span.line,
                result.span.column,
                end_line,
                end_column
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", entries.join(","))
}

fn json_escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            character if character.is_control() => {
                format!("\\u{:04x}", character as u32).chars().collect()
            }
            character => vec![character],
        })
        .collect()
}

fn source_position(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
    let end = offset.min(source.len());
    for byte in &source.as_bytes()[..end] {
        if *byte == b'\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

fn validate_capabilities(path: &str, program: &zelyra_ast::Program) -> Result<(), ()> {
    let grants = match project_capability_grants(path) {
        Ok(grants) => grants,
        Err(error) => {
            diagnostic(path, "E-CAP-002", &error, 1, 1);
            return Err(());
        }
    };
    if let Err(errors) = check_capabilities_with_grants(program, grants.as_ref()) {
        for error in errors {
            diagnostic(
                path,
                "E-CAP-001",
                &error.message,
                error.span.line,
                error.span.column,
            );
        }
        return Err(());
    }
    Ok(())
}

fn project_capability_grants(path: &str) -> Result<Option<HashSet<String>>, String> {
    let source_path =
        fs::canonicalize(path).map_err(|error| format!("cannot locate source: {error}"))?;
    let mut directory = source_path
        .parent()
        .ok_or_else(|| "source has no parent directory".to_owned())?;
    let Some(config_path) = (loop {
        let candidate = directory.join("zelyra.toml");
        if candidate.is_file() {
            break Some(candidate);
        }
        let Some(parent) = directory.parent() else {
            break None;
        };
        if parent == directory {
            break None;
        }
        directory = parent;
    }) else {
        return Ok(None);
    };
    let contents = fs::read_to_string(&config_path)
        .map_err(|error| format!("cannot read {}: {error}", config_path.display()))?;
    let mut grants = HashSet::new();
    let mut seen = HashSet::new();
    let mut in_capabilities = false;
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_capabilities = line == "[capabilities]";
            continue;
        }
        if !in_capabilities {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(format!(
                "invalid capability setting on line {}",
                line_index + 1
            ));
        };
        let key = raw_key.trim().to_ascii_lowercase();
        let capability = KNOWN_CAPABILITIES
            .iter()
            .copied()
            .find(|capability| capability.to_ascii_lowercase() == key)
            .ok_or_else(|| format!("unknown capability setting `{key}`"))?;
        if !seen.insert(capability) {
            return Err(format!("capability `{key}` is configured more than once"));
        }
        match raw_value.trim() {
            "true" => {
                grants.insert(capability.to_owned());
            }
            "false" => {}
            value => {
                return Err(format!(
                    "capability `{key}` must be true or false, found `{value}`"
                ));
            }
        }
    }
    Ok(Some(grants))
}

fn validate_auth(path: &str, program: &zelyra_ast::Program, schema: &Schema) -> bool {
    let mut valid = true;
    let mut names = HashSet::new();
    for auth in &program.auth {
        if !names.insert(auth.name.clone()) {
            diagnostic(
                path,
                "E-AUTH-002",
                &format!("duplicate authentication definition {}", auth.name),
                auth.span.line,
                auth.span.column,
            );
            valid = false;
        }
        if !schema.tables.iter().any(|table| table.name == auth.table) {
            diagnostic(
                path,
                "E-AUTH-001",
                &format!("authentication refers to unknown user table {}", auth.table),
                auth.span.line,
                auth.span.column,
            );
            valid = false;
            continue;
        }
        let Some(table) = schema.tables.iter().find(|table| table.name == auth.table) else {
            continue;
        };
        for required_column in ["id", "email", "password_hash"] {
            if !table
                .columns
                .iter()
                .any(|column| column.name == required_column)
            {
                diagnostic(
                    path,
                    "E-AUTH-004",
                    &format!(
                        "authentication table {} requires column {}",
                        auth.table, required_column
                    ),
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
            }
        }
        if let Some(session_table_name) = &auth.session_table {
            let Some(session_table) = schema
                .tables
                .iter()
                .find(|candidate| candidate.name == *session_table_name)
            else {
                diagnostic(
                    path,
                    "E-AUTH-005",
                    &format!(
                        "authentication refers to unknown session table {}",
                        session_table_name
                    ),
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
                continue;
            };
            for required_column in ["user_id", "token_hash", "expires_at"] {
                if !session_table
                    .columns
                    .iter()
                    .any(|column| column.name == required_column)
                {
                    diagnostic(
                        path,
                        "E-AUTH-006",
                        &format!(
                            "authentication session table {} requires column {}",
                            session_table_name, required_column
                        ),
                        auth.span.line,
                        auth.span.column,
                    );
                    valid = false;
                }
            }
        }
        if let Some(permissions_table_name) = &auth.permissions_table {
            let Some(permissions_table) = schema
                .tables
                .iter()
                .find(|candidate| candidate.name == *permissions_table_name)
            else {
                diagnostic(
                    path,
                    "E-AUTH-007",
                    &format!(
                        "authentication refers to unknown permissions table {}",
                        permissions_table_name
                    ),
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
                continue;
            };
            for required_column in ["user_id", "permission"] {
                if !permissions_table
                    .columns
                    .iter()
                    .any(|column| column.name == required_column)
                {
                    diagnostic(
                        path,
                        "E-AUTH-008",
                        &format!(
                            "authentication permissions table {} requires column {}",
                            permissions_table_name, required_column
                        ),
                        auth.span.line,
                        auth.span.column,
                    );
                    valid = false;
                }
            }
        }
    }
    let protected = program
        .pages
        .iter()
        .any(|page| page.requires_auth || !page.permissions.is_empty())
        || program
            .cruds
            .iter()
            .any(|crud| crud.requires_auth || !crud.permissions.is_empty());
    if protected && program.auth.is_empty() {
        diagnostic(
            path,
            "E-AUTH-003",
            "protected routes require an auth definition",
            1,
            1,
        );
        valid = false;
    }
    valid
}

fn validate_cruds(path: &str, program: &zelyra_ast::Program, schema: &Schema) -> bool {
    let mut valid = true;
    let mut names = HashSet::new();
    let mut tables = HashSet::new();
    for crud in &program.cruds {
        if !names.insert(crud.name.clone()) {
            diagnostic(
                path,
                "E-CRUD-002",
                &format!("duplicate CRUD resource `{}`", crud.name),
                crud.span.line,
                crud.span.column,
            );
            valid = false;
        }
        if !tables.insert(crud.table.clone()) {
            diagnostic(
                path,
                "E-CRUD-003",
                &format!("table `{}` already has a CRUD resource", crud.table),
                crud.span.line,
                crud.span.column,
            );
            valid = false;
        }
        if !schema.tables.iter().any(|table| table.name == crud.table) {
            diagnostic(
                path,
                "E-CRUD-001",
                &format!("CRUD resource refers to unknown table `{}`", crud.table),
                crud.span.line,
                crud.span.column,
            );
            valid = false;
            continue;
        }
        let configured_columns = crud.list.iter().chain(&crud.search).chain(&crud.filters);
        for column in configured_columns {
            if !crud_column_exists(program, schema, crud, column) {
                diagnostic(
                    path,
                    "E-CRUD-004",
                    &format!(
                        "CRUD column {column} does not exist in table {}",
                        crud.table
                    ),
                    crud.span.line,
                    crud.span.column,
                );
                valid = false;
            }
        }
    }
    valid
}

fn crud_column_exists(
    program: &zelyra_ast::Program,
    schema: &Schema,
    crud: &zelyra_ast::CrudDef,
    column: &str,
) -> bool {
    let Some(table) = program.tables.iter().find(|table| table.name == crud.table) else {
        return false;
    };
    let logical_exists = table
        .columns
        .iter()
        .any(|candidate| candidate.name == column);
    if !logical_exists
        && !schema.tables.iter().any(|table| {
            table.name == crud.table
                && table
                    .columns
                    .iter()
                    .any(|candidate| candidate.name == column)
        })
    {
        return false;
    }
    let storage = storage_column_name(schema, &crud.table, column);
    schema
        .tables
        .iter()
        .find(|table| table.name == crud.table)
        .is_some_and(|table| {
            table
                .columns
                .iter()
                .any(|candidate| candidate.name == storage)
        })
}

fn configured_crud_columns(
    program: &zelyra_ast::Program,
    schema: &Schema,
    crud: &zelyra_ast::CrudDef,
    configured: &[String],
    default: impl FnOnce(&zelyra_database::Table) -> Vec<String>,
) -> Vec<String> {
    let table = schema
        .tables
        .iter()
        .find(|table| table.name == crud.table)
        .expect("CRUD table was validated before route generation");
    if configured.is_empty() {
        return default(table);
    }
    configured
        .iter()
        .map(|column| storage_column_name(schema, &crud.table, column))
        .filter(|column| crud_column_exists(program, schema, crud, column))
        .collect()
}

fn load_schema(path: &str) -> Result<Schema, ()> {
    let program = load(path)?;
    match build_schema(&program) {
        Ok(schema) => Ok(schema),
        Err(errors) => {
            for error in errors {
                diagnostic(
                    path,
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
    for change in &plan.changes {
        let risk = match change.risk {
            Risk::Safe => "SAFE",
            Risk::Destructive => "DESTRUCTIVE",
        };
        println!("[{risk}] {}\n{}\n", change.description, change.sql);
    }
}

fn database_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(subcommand) = args.next() else {
        database_usage();
        return ExitCode::from(2);
    };
    let path = args.next().unwrap_or_else(|| "main.zyl".into());
    let allow_destructive = args.any(|arg| arg == "--allow-destructive");
    let schema = match load_schema(&path) {
        Ok(schema) => schema,
        Err(()) => return ExitCode::from(1),
    };
    match subcommand.as_str() {
        "create" => {
            println!("{}", schema.create_sql());
            ExitCode::SUCCESS
        }
        "bootstrap" => {
            let Ok(url) = env::var("DATABASE_URL") else {
                eprintln!("error[E-DB-003]: DATABASE_URL is required for db bootstrap");
                return ExitCode::from(1);
            };
            let result = match schema.backend() {
                Backend::MariaDb => create_mariadb_database(&url)
                    .and_then(|()| apply_mariadb(&url, &schema.create_sql())),
                Backend::Sqlite => apply_sqlite(&url, &schema.create_sql()),
                Backend::Postgres => Err(zelyra_database::DatabaseError {
                    message: "db bootstrap currently supports mariadb and sqlite; use db apply for postgres".into(),
                }),
            };
            match result {
                Ok(()) => {
                    println!("database bootstrapped successfully");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error[E-DB-005]: {error}");
                    ExitCode::from(1)
                }
            }
        }
        "inspect" => match env::var("DATABASE_URL") {
            Ok(url) => match inspect_for_backend(schema.backend(), &url) {
                Ok(current) => {
                    println!("{}", current.summary());
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error[E-DB-002]: {error}");
                    ExitCode::from(1)
                }
            },
            Err(_) => {
                eprintln!("error[E-DB-003]: DATABASE_URL is required for db inspect");
                ExitCode::from(1)
            }
        },
        "plan" => {
            let current = match env::var("DATABASE_URL") {
                Ok(url) => match inspect_for_backend(schema.backend(), &url) {
                    Ok(current) => current,
                    Err(error) => {
                        eprintln!("error[E-DB-002]: {error}");
                        return ExitCode::from(1);
                    }
                },
                Err(_) => {
                    eprintln!("note: DATABASE_URL is not set; planning against an empty database");
                    Schema {
                        database: None,
                        tables: Vec::new(),
                    }
                }
            };
            print_plan(&diff(&schema, &current));
            ExitCode::SUCCESS
        }
        "apply" => {
            let Ok(url) = env::var("DATABASE_URL") else {
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
            if plan.is_destructive() && !allow_destructive {
                eprintln!("error[E-DB-004]: destructive changes refused; use --allow-destructive after review");
                return ExitCode::from(1);
            }
            if plan.changes.is_empty() {
                return ExitCode::SUCCESS;
            }
            let result = match schema.backend() {
                Backend::Postgres => apply_postgres(&url, &plan.sql()),
                Backend::MariaDb => apply_mariadb(&url, &plan.sql()),
                Backend::Sqlite => apply_sqlite(&url, &plan.sql()),
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

fn inspect_for_backend(
    backend: Backend,
    database_url: &str,
) -> Result<Schema, zelyra_database::DatabaseError> {
    match backend {
        Backend::Postgres => inspect_postgres(database_url),
        Backend::MariaDb => inspect_mariadb(database_url),
        Backend::Sqlite => inspect_sqlite(database_url),
    }
}

fn serve_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(path) = args.next() else {
        usage();
        return ExitCode::from(2);
    };
    let address = args.next().unwrap_or_else(|| "127.0.0.1:3000".into());
    if args.next().is_some() {
        usage();
        return ExitCode::from(2);
    }
    let program = match load(&path) {
        Ok(program) => program,
        Err(()) => return ExitCode::from(1),
    };
    if validate_capabilities(&path, &program).is_err() {
        return ExitCode::from(1);
    }
    if program.pages.is_empty() && program.forms.is_empty() && program.cruds.is_empty() {
        eprintln!("error[E-WEB-001]: {path} does not define a page, form, or CRUD resource");
        return ExitCode::from(1);
    }
    let routes = program
        .pages
        .iter()
        .map(|page| Route {
            path: page.path.clone(),
            html: page.html.clone(),
            requires_auth: page.requires_auth,
            permissions: page.permissions.clone(),
        })
        .collect();
    let schema = match build_schema(&program) {
        Ok(schema) => schema,
        Err(errors) => {
            for error in errors {
                diagnostic(
                    &path,
                    "E-DB-001",
                    &error.message,
                    error.span.line,
                    error.span.column,
                );
            }
            return ExitCode::from(1);
        }
    };
    if !validate_auth(&path, &program, &schema) {
        return ExitCode::from(1);
    }
    if !validate_cruds(&path, &program, &schema) {
        return ExitCode::from(1);
    }
    if let Err(errors) = check_form_program(&program, &schema) {
        for error in errors {
            diagnostic(
                &path,
                "E-FORM-001",
                &error.message,
                error.span.line,
                error.span.column,
            );
        }
        return ExitCode::from(1);
    }
    let mut form_routes = Vec::new();
    let auth_route = if let Some(auth) = program.auth.first() {
        let Some(csrf) = CsrfProtection::generate().ok() else {
            eprintln!("error[E-WEB-003]: cannot create a secure CSRF token");
            return ExitCode::from(1);
        };
        Some(AuthRoute {
            table: auth.table.clone(),
            session_table: auth.session_table.clone(),
            permissions_table: auth.permissions_table.clone(),
            schema: schema.clone(),
            csrf,
        })
    } else {
        None
    };
    for form in &program.forms {
        let Some(csrf) = CsrfProtection::generate().ok() else {
            eprintln!("error[E-WEB-003]: cannot create a secure CSRF token");
            return ExitCode::from(1);
        };
        let table = form.table.as_deref().and_then(|table_name| {
            program
                .tables
                .iter()
                .find(|table| table.name == table_name)
                .cloned()
        });
        form_routes.push(FormRoute {
            path: format!("/forms/{}", form.name),
            action: format!("/forms/{}", form.name),
            form: form.clone(),
            table,
            schema: Some(schema.clone()),
            csrf,
        });
    }
    for crud in &program.cruds {
        let Some(table) = program.tables.iter().find(|table| table.name == crud.table) else {
            continue;
        };
        for edit in [false, true] {
            let Some(csrf) = CsrfProtection::generate().ok() else {
                eprintln!("error[E-WEB-003]: cannot create a secure CSRF token");
                return ExitCode::from(1);
            };
            form_routes.push(generated_crud_form(crud, table, &schema, edit, csrf));
        }
    }
    let mut crud_routes = Vec::new();
    for crud in &program.cruds {
        let Some(csrf) = CsrfProtection::generate().ok() else {
            eprintln!("error[E-WEB-003]: cannot create a secure CSRF token");
            return ExitCode::from(1);
        };
        let list_columns = configured_crud_columns(&program, &schema, crud, &crud.list, |table| {
            table
                .columns
                .iter()
                .map(|column| column.name.clone())
                .collect()
        });
        let search_columns =
            configured_crud_columns(&program, &schema, crud, &crud.search, |table| {
                table
                    .columns
                    .iter()
                    .filter(|column| {
                        let sql_type = column.sql_type.to_ascii_uppercase();
                        sql_type.contains("CHAR") || sql_type.contains("TEXT")
                    })
                    .map(|column| column.name.clone())
                    .collect()
            });
        let filter_columns =
            configured_crud_columns(&program, &schema, crud, &crud.filters, |table| {
                table
                    .columns
                    .iter()
                    .filter(|column| column.name != "id")
                    .map(|column| column.name.clone())
                    .collect()
            });
        crud_routes.push(CrudRoute {
            path: format!("/{}", crud.table),
            title: crud.title.clone().unwrap_or_else(|| crud.name.clone()),
            table: crud.table.clone(),
            list_columns,
            search_columns,
            filter_columns,
            requires_auth: crud.requires_auth,
            permissions: crud.permissions.clone(),
            schema: schema.clone(),
            csrf,
        });
    }
    eprintln!("Zelyra server listening on http://{address}");
    let app = WebApp::with_database_url(routes, form_routes, env::var("DATABASE_URL").ok())
        .with_auth(
            env::var("ZELYRA_AUTH_TOKEN").ok(),
            env::var("ZELYRA_AUTH_PERMISSIONS")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|permission| !permission.is_empty())
                .map(str::to_owned)
                .collect(),
        )
        .with_cruds(crud_routes);
    let app = if let Some(auth_route) = auth_route {
        app.with_auth_route(auth_route)
    } else {
        app
    };
    match serve_app(app, &address) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error[E-WEB-002]: cannot start server on {address}: {error}");
            ExitCode::from(1)
        }
    }
}

fn generated_crud_form(
    crud: &zelyra_ast::CrudDef,
    table: &zelyra_ast::TableDef,
    schema: &Schema,
    edit: bool,
    csrf: CsrfProtection,
) -> FormRoute {
    let fields = table
        .columns
        .iter()
        .filter(|column| !column.primary_key && !column.auto)
        .map(|column| zelyra_ast::FormField {
            name: column.name.clone(),
            ty: None,
            label: None,
            placeholder: None,
            required: false,
            max: None,
            widget: None,
            readonly: false,
            span: column.span,
        })
        .collect::<Vec<_>>();
    let storage_columns = fields
        .iter()
        .map(|field| storage_column_name(schema, &crud.table, &field.name))
        .collect::<Vec<_>>();
    let query = if edit {
        let assignments = storage_columns
            .iter()
            .zip(&fields)
            .map(|(column, field)| format!("{} = :{}", quote_identifier(column), field.name))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "UPDATE {} SET {} WHERE {} = :id",
            quote_identifier(&crud.table),
            assignments,
            quote_identifier("id")
        )
    } else {
        format!(
            "INSERT INTO {} ({}) VALUES ({})",
            quote_identifier(&crud.table),
            storage_columns
                .iter()
                .map(|column| quote_identifier(column))
                .collect::<Vec<_>>()
                .join(", "),
            fields
                .iter()
                .map(|field| format!(":{}", field.name))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let path = if edit {
        format!("/{}/{{id}}/edit", crud.table)
    } else {
        format!("/{}/new", crud.table)
    };
    FormRoute {
        path: path.clone(),
        action: path,
        form: zelyra_ast::FormDef {
            name: format!("{}{}", crud.name, if edit { "Edit" } else { "Create" }),
            table: Some(table.name.clone()),
            fields,
            actions: vec![zelyra_ast::FormAction {
                name: "save".into(),
                statements: vec![zelyra_ast::Stmt::Expr(zelyra_ast::Expr {
                    kind: zelyra_ast::ExprKind::Sql {
                        result_type: zelyra_ast::Type::Unit,
                        query,
                    },
                    span: table.span,
                })],
                success: Some("Saved.".into()),
                redirect: Some(format!("/{}", crud.table)),
                span: table.span,
            }],
            span: table.span,
        },
        table: Some(table.clone()),
        schema: Some(schema.clone()),
        csrf,
    }
}

fn storage_column_name(schema: &Schema, table: &str, field: &str) -> String {
    schema
        .tables
        .iter()
        .find(|candidate| candidate.name == table)
        .and_then(|candidate| {
            candidate
                .columns
                .iter()
                .find(|column| column.name == field || column.name == format!("{field}_id"))
        })
        .map(|column| column.name.clone())
        .unwrap_or_else(|| field.into())
}

fn quote_identifier(identifier: &str) -> String {
    format!("`{}`", identifier.replace('`', "``"))
}

fn form_usage() {
    eprintln!("Usage: zelyra form validate <file.zyl> <FormName> [field=value ...]");
}

fn form_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    if args.next().as_deref() != Some("validate") {
        form_usage();
        return ExitCode::from(2);
    }
    let Some(path) = args.next() else {
        form_usage();
        return ExitCode::from(2);
    };
    let Some(form_name) = args.next() else {
        form_usage();
        return ExitCode::from(2);
    };
    let mut input = std::collections::HashMap::new();
    for argument in args {
        let Some((field, value)) = argument.split_once('=') else {
            eprintln!("error[E-FORM-002]: expected field=value, found `{argument}`");
            return ExitCode::from(2);
        };
        if field.is_empty() {
            eprintln!("error[E-FORM-002]: field name must not be empty");
            return ExitCode::from(2);
        }
        input.insert(field.to_owned(), value.to_owned());
    }
    let program = match load(&path) {
        Ok(program) => program,
        Err(()) => return ExitCode::from(1),
    };
    let schema = match build_schema(&program) {
        Ok(schema) => schema,
        Err(errors) => {
            for error in errors {
                diagnostic(
                    &path,
                    "E-DB-001",
                    &error.message,
                    error.span.line,
                    error.span.column,
                );
            }
            return ExitCode::from(1);
        }
    };
    if let Err(errors) = check_form_program(&program, &schema) {
        for error in errors {
            diagnostic(
                &path,
                "E-FORM-001",
                &error.message,
                error.span.line,
                error.span.column,
            );
        }
        return ExitCode::from(1);
    }
    let Some(form) = program.forms.iter().find(|form| form.name == form_name) else {
        eprintln!("error[E-FORM-003]: form `{form_name}` was not found in `{path}`");
        return ExitCode::from(1);
    };
    let table_definition = form
        .table
        .as_deref()
        .and_then(|table_name| program.tables.iter().find(|table| table.name == table_name));
    let result = validate_form(form, table_definition, Some(&schema), &input);
    if result.is_valid() {
        println!("valid: {form_name}");
        ExitCode::SUCCESS
    } else {
        for error in result.errors {
            eprintln!("error[E-FORM-004]: {}: {}", error.field, error.message);
        }
        ExitCode::from(1)
    }
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        usage();
        return ExitCode::from(2);
    };
    if command == "--help" || command == "-h" {
        usage();
        return ExitCode::SUCCESS;
    }
    if command == "db" {
        return database_command(args);
    }
    if command == "form" {
        return form_command(args);
    }
    if command == "new" {
        let Some(path) = args.next() else {
            usage();
            return ExitCode::from(2);
        };
        if args.next().is_some() {
            usage();
            return ExitCode::from(2);
        }
        return create_project(&path, false);
    }
    if command == "init" {
        let path = args.next().unwrap_or_else(|| ".".into());
        if args.next().is_some() {
            usage();
            return ExitCode::from(2);
        }
        return create_project(&path, true);
    }
    if command == "serve" {
        return serve_command(args);
    }
    if command == "verify" {
        let Some(path) = args.next() else {
            usage();
            return ExitCode::from(2);
        };
        let json = match args.next() {
            None => false,
            Some(flag) if flag == "--json" => true,
            Some(_) => {
                usage();
                return ExitCode::from(2);
            }
        };
        if args.next().is_some() {
            usage();
            return ExitCode::from(2);
        }
        return verify_command(&path, json);
    }
    let Some(path) = args.next() else {
        usage();
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        usage();
        return ExitCode::from(2);
    }
    match command.as_str() {
        "check" | "build" => {
            if validate(&path).is_ok() {
                println!("ok: {path}");
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        "run" => match validate(&path).and_then(|program| {
            let result = match env::var("DATABASE_URL") {
                Ok(database_url) => execute_with_database(&program, &database_url),
                Err(_) => execute(&program),
            };
            result.map_err(|error| {
                diagnostic(
                    &path,
                    "E-RUNTIME-001",
                    &error.message,
                    error.span.line,
                    error.span.column,
                );
            })
        }) {
            Ok(output) => {
                for line in output {
                    println!("{line}");
                }
                ExitCode::SUCCESS
            }
            Err(()) => ExitCode::from(1),
        },
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_verification_results_with_source_location() {
        let result = VerificationResult {
            function: "reduce".into(),
            kind: zelyra_runtime::ContractKind::LoopInvariant,
            index: 0,
            status: VerificationStatus::Proven,
            span: zelyra_ast::Span::new(6, 12, 2, 1),
        };
        assert_eq!(
            format_verification_result("src/reduce.zyl", "first\nsecond value\n", &result),
            "PROVEN [V-001]: reduce.invariant[0] (src/reduce.zyl:2:1-2:7)"
        );
    }

    #[test]
    fn formats_verification_results_as_json() {
        let result = VerificationResult {
            function: "say\"hello".into(),
            kind: zelyra_runtime::ContractKind::Ensures,
            index: 1,
            status: VerificationStatus::RuntimeCheck,
            span: zelyra_ast::Span::new(6, 12, 2, 1),
        };
        assert_eq!(
            format_verification_json("src/file.zyl", "first\nsecond value\n", &[result]),
            r#"[{"status":"RUNTIME_CHECK","code":"V-002","function":"say\"hello","kind":"ensures","index":1,"location":{"file":"src/file.zyl","start":{"line":2,"column":1},"end":{"line":2,"column":7}}}]"#
        );
    }

    #[test]
    fn rejects_unknown_configured_crud_columns() {
        let source = r#"
            table machines {
                id: Id primary auto
                name: String(100) required
            }

            crud Machine -> machines {
                list { missing }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(!validate_cruds("test.zyl", &program, &schema));
    }

    #[test]
    fn rejects_protected_routes_without_auth_definition() {
        let source = r#"
            page "/admin" {
                requires auth
                html {
                    <h1>Admin</h1>
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(!validate_auth("test.zyl", &program, &schema));
    }

    #[test]
    fn accepts_persistent_auth_tables() {
        let source = r#"
            auth users {
                table: users
                sessions: auth_sessions
                permissions: user_permissions
            }

            table users {
                id: Id primary auto
                email: Email required
                password_hash: String(255) required
            }

            table auth_sessions {
                id: Id primary auto
                user: User required
                token_hash: String(64) required
                expires_at: Timestamp required
            }

            table user_permissions {
                id: Id primary auto
                user: User required
                permission: String(100) required
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(validate_auth("test.zyl", &program, &schema));
    }

    #[test]
    fn reads_project_capability_grants() {
        let grants = project_capability_grants("../examples/capabilities.zyl")
            .unwrap()
            .unwrap();
        assert!(grants.contains("Database"));
        assert!(grants.contains("Network"));
    }
}
