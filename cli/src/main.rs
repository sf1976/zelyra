use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::Write as _,
    fs,
    net::TcpListener,
    path::PathBuf,
    process::Command,
    process::ExitCode,
};
use zelyra_ast::Type;
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
    check, check_apis, check_capabilities_with_grants,
    execute_function_with_capabilities_and_policies, execute_with_capabilities_and_policies,
    execute_with_database_and_capabilities_and_policies, verify as verify_program,
    FileSystemPolicy, NetworkPolicy, ProcessPolicy, RuntimePolicy, Value, VerificationResult,
    VerificationStatus, KNOWN_CAPABILITIES,
};
use zelyra_web::{
    parse_urlencoded, serve_app, ApiRoute, AuthRoute, CorsPolicy, CrudRoute, CsrfProtection,
    FormRoute, Response, Route, WebApp,
};

fn usage() {
    eprintln!("Zelyra 0.1\n\nUsage:\n  zelyra new <directory> [--mariadb]\n  zelyra init [directory] [--mariadb]\n  zelyra check <file.zyl>\n  zelyra build <file.zyl>\n  zelyra run <file.zyl>\n  zelyra serve <file.zyl> [address]\n  zelyra doctor [file.zyl] [--port <port>] [--json]\n  zelyra verify <file.zyl> [--json]\n  zelyra doc <file.zyl> [--openapi|--typescript]\n  zelyra auth hash-password [--stdin]\n  zelyra form validate <file.zyl> <FormName> [field=value ...]\n  zelyra db <create|setup|bootstrap|inspect|plan|apply> <file.zyl>");
}

fn database_usage() {
    eprintln!(
        "Usage:\n  zelyra db create <file.zyl>\n  zelyra db setup <file.zyl>\n  zelyra db bootstrap <file.zyl>\n  zelyra db inspect <file.zyl>\n  zelyra db plan <file.zyl>\n  zelyra db apply <file.zyl> [--allow-destructive]\n\nDATABASE_URL is used by setup, bootstrap, inspect, plan, and apply."
    );
}

fn create_project(path: &str, allow_current_directory: bool, with_mariadb: bool) -> ExitCode {
    let directory = std::path::Path::new(path);
    if directory.exists() && !allow_current_directory {
        eprintln!("error[E-INIT-001]: directory `{path}` already exists");
        return ExitCode::from(1);
    }
    if let Err(error) = fs::create_dir_all(directory) {
        eprintln!("error[E-INIT-002]: cannot create `{path}`: {error}");
        return ExitCode::from(1);
    }
    let project_config = if with_mariadb {
        "[project]\nname = \"zelyra-app\"\nversion = \"0.1.37\"\nzelyra = \"0.1\"\n\n[database.main]\nengine = \"mariadb\"\n\n[capabilities]\ndatabase = true\nnetwork = false\n"
    } else {
        "[project]\nname = \"zelyra-app\"\nversion = \"0.1.37\"\nzelyra = \"0.1\"\n\n[capabilities]\ndatabase = true\nnetwork = false\n"
    };
    let main_source = if with_mariadb {
        "database main {\n    engine: mariadb\n}\n\npage \"/\" {\n    html {\n        <h1>Welcome to Zelyra</h1>\n        <p>Your MariaDB-ready application is running.</p>\n    }\n}\n\nfn main() {\n    print(\"Hello from Zelyra\")\n}\n"
    } else {
        "fn main() {\n    print(\"Hello from Zelyra\")\n}\n"
    };
    let mut files = vec![("zelyra.toml", project_config), ("main.zyl", main_source)];
    if with_mariadb {
        files.extend([
            (
                ".env.example",
                "# Copy this file to .env. Never commit .env or real credentials.\n# ZELYRA_WEB_PORT is the internal and host port of the web server.\nZELYRA_WEB_PORT=3000\nDATABASE_URL=mariadb://zelyra:change-me@127.0.0.1:3306/zelyra_app\nMARIADB_DATABASE=zelyra_app\nMARIADB_USER=zelyra\nMARIADB_PASSWORD=change-me\nMARIADB_ROOT_PASSWORD=change-me-root\n",
            ),
            (
                "docker-compose.mariadb.yml",
                "services:\n  mariadb:\n    image: mariadb:11\n    restart: unless-stopped\n    environment:\n      MARIADB_DATABASE: ${MARIADB_DATABASE}\n      MARIADB_USER: ${MARIADB_USER}\n      MARIADB_PASSWORD: ${MARIADB_PASSWORD}\n      MARIADB_ROOT_PASSWORD: ${MARIADB_ROOT_PASSWORD}\n    ports:\n      - \"127.0.0.1:3306:3306\"\n    volumes:\n      - zelyra_mariadb_data:/var/lib/mysql\n    healthcheck:\n      test: [\"CMD\", \"healthcheck.sh\", \"--connect\", \"--innodb_initialized\"]\n      interval: 5s\n      timeout: 5s\n      retries: 20\n\n  web:\n    build: .\n    command: [\"zelyra\", \"serve\", \"main.zyl\", \"0.0.0.0:${ZELYRA_WEB_PORT:-3000}\"]\n    environment:\n      DATABASE_URL: mariadb://${MARIADB_USER}:${MARIADB_PASSWORD}@mariadb:3306/${MARIADB_DATABASE}\n    depends_on:\n      mariadb:\n        condition: service_healthy\n    ports:\n      - \"127.0.0.1:${ZELYRA_WEB_PORT:-3000}:${ZELYRA_WEB_PORT:-3000}\"\n\nvolumes:\n  zelyra_mariadb_data:\n",
            ),
            (
                "Dockerfile",
                "FROM rust:1-bookworm AS build\nARG ZELYRA_REF=v0.1.37-alpha.1\nRUN apt-get update \\\n    && apt-get install -y --no-install-recommends ca-certificates git \\\n    && rm -rf /var/lib/apt/lists/*\nRUN git clone --depth 1 --branch ${ZELYRA_REF} https://github.com/sf1976/zelyra.git /zelyra\nRUN cargo install --path /zelyra/cli --root /out\n\nFROM debian:bookworm-slim\nRUN apt-get update \\\n    && apt-get install -y --no-install-recommends ca-certificates mariadb-client \\\n    && rm -rf /var/lib/apt/lists/*\nCOPY --from=build /out/bin/zelyra /usr/local/bin/zelyra\nCOPY main.zyl zelyra.toml ./\nEXPOSE 3000\nCMD [\"zelyra\", \"serve\", \"main.zyl\", \"0.0.0.0:3000\"]\n",
            ),
            (
                ".dockerignore",
                ".git\ntarget\n.env\n*.sqlite3\n",
            ),
        ]);
    }
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
    if with_mariadb {
        if cfg!(windows) {
            println!("next: cd {} && copy .env.example .env", path);
        } else {
            println!("next: cd {} && cp .env.example .env", path);
        }
        println!("then start MariaDB with docker compose -f docker-compose.mariadb.yml up -d");
        println!("then run: zelyra db setup main.zyl");
    } else {
        println!("next: cd {} && zelyra run main.zyl", path);
    }
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
    if let Err(error) = project_filesystem_policy(path) {
        diagnostic(path, "E-FS-002", &error, 1, 1);
        return Err(());
    }
    if let Err(error) = project_network_policy(path) {
        diagnostic(path, "E-NET-002", &error, 1, 1);
        return Err(());
    }
    if let Err(error) = project_process_policy(path) {
        diagnostic(path, "E-PROC-002", &error, 1, 1);
        return Err(());
    }
    if let Err(error) = project_cors_policy(path) {
        diagnostic(path, "E-WEB-004", &error, 1, 1);
        return Err(());
    }
    if let Err(errors) = check_apis(&program) {
        for error in errors {
            diagnostic(
                path,
                "E-API-001",
                &error.message,
                error.span.line,
                error.span.column,
            );
        }
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

struct DoctorCheck {
    name: &'static str,
    status: &'static str,
    message: String,
}

fn format_doctor_json(path: &str, checks: &[DoctorCheck]) -> String {
    let failed = checks.iter().any(|check| check.status == "fail");
    let warnings = checks.iter().filter(|check| check.status == "warn").count();
    let checks = checks
        .iter()
        .map(|check| {
            serde_json::json!({
                "name": check.name,
                "status": check.status,
                "message": check.message,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "project": path,
        "status": if failed { "failed" } else { "ready" },
        "warnings": warnings,
        "checks": checks,
    })
    .to_string()
}

fn doctor_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let mut path = "main.zyl".to_owned();
    let mut path_given = false;
    let mut port = 3000u16;
    let mut json = false;
    while let Some(argument) = args.next() {
        if argument == "--json" {
            json = true;
        } else if argument == "--port" {
            let Some(value) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            port = match value.parse() {
                Ok(port) => port,
                Err(_) => {
                    eprintln!("error[E-DOCTOR-001]: invalid TCP port `{value}`");
                    return ExitCode::from(2);
                }
            };
        } else if argument.starts_with('-') || path_given {
            usage();
            return ExitCode::from(2);
        } else {
            path = argument;
            path_given = true;
        }
    }

    let mut checks = Vec::new();
    let program = if fs::metadata(&path).is_ok() {
        checks.push(DoctorCheck {
            name: "project_file",
            status: "pass",
            message: format!("{path} exists"),
        });
        match validate(&path) {
            Ok(program) => {
                checks.push(DoctorCheck {
                    name: "static_checks",
                    status: "pass",
                    message: "source, types, APIs, SQL, and forms are valid".into(),
                });
                Some(program)
            }
            Err(()) => {
                checks.push(DoctorCheck {
                    name: "static_checks",
                    status: "fail",
                    message: "see diagnostics above".into(),
                });
                None
            }
        }
    } else {
        checks.push(DoctorCheck {
            name: "project_file",
            status: "fail",
            message: format!("`{path}` does not exist"),
        });
        None
    };

    match Command::new("cargo").arg("--version").output() {
        Ok(output) if output.status.success() => checks.push(DoctorCheck {
            name: "rust_toolchain",
            status: "pass",
            message: String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        }),
        _ => checks.push(DoctorCheck {
            name: "rust_toolchain",
            status: "warn",
            message: "cargo is unavailable".into(),
        }),
    }

    if let Some(program) = &program {
        match build_schema(program) {
            Ok(schema) => {
                let backend = schema.backend();
                match env::var("DATABASE_URL") {
                    Ok(url) => match inspect_for_backend(backend, &url) {
                        Ok(current) => checks.push(DoctorCheck {
                            name: "database",
                            status: "pass",
                            message: format!(
                                "{}: {}",
                                backend.name(),
                                current.summary().replace('\n', ", ")
                            ),
                        }),
                        Err(error) => checks.push(DoctorCheck {
                            name: "database",
                            status: "fail",
                            message: format!("{}: {error}", backend.name()),
                        }),
                    },
                    Err(_) => checks.push(DoctorCheck {
                        name: "database",
                        status: "warn",
                        message: format!("{}: DATABASE_URL is not set", backend.name()),
                    }),
                }
            }
            Err(errors) => checks.push(DoctorCheck {
                name: "schema",
                status: "fail",
                message: errors
                    .iter()
                    .map(|error| error.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; "),
            }),
        }
    }

    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(listener) => {
            let actual_port = listener.local_addr().map_or(port, |address| address.port());
            checks.push(DoctorCheck {
                name: "web_port",
                status: "pass",
                message: format!("127.0.0.1:{actual_port} is available"),
            });
        }
        Err(error) => checks.push(DoctorCheck {
            name: "web_port",
            status: "fail",
            message: format!("127.0.0.1:{port} is unavailable ({error})"),
        }),
    }

    let failed = checks.iter().any(|check| check.status == "fail");
    if json {
        println!("{}", format_doctor_json(&path, &checks));
    } else {
        println!("Zelyra doctor {}", env!("CARGO_PKG_VERSION"));
        for check in &checks {
            let label = match check.status {
                "pass" => "PASS",
                "warn" => "WARN",
                _ => "FAIL",
            };
            println!("  [{label}] {}: {}", check.name, check.message);
        }
        let warnings = checks.iter().filter(|check| check.status == "warn").count();
        println!(
            "Doctor result: {} ({warnings} warning(s))",
            if failed { "failed" } else { "ready" }
        );
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn doc_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(path) = args.next() else {
        usage();
        return ExitCode::from(2);
    };
    let format = match args.next() {
        None => "openapi",
        Some(flag) if flag == "--openapi" => "openapi",
        Some(flag) if flag == "--typescript" => "typescript",
        Some(_) => {
            usage();
            return ExitCode::from(2);
        }
    };
    if args.next().is_some() {
        usage();
        return ExitCode::from(2);
    }
    let program = match validate(&path) {
        Ok(program) => program,
        Err(()) => return ExitCode::from(1),
    };
    if format == "typescript" {
        println!("{}", format_typescript_client(&program));
    } else {
        println!("{}", format_openapi(&program));
    }
    ExitCode::SUCCESS
}

fn format_openapi(program: &zelyra_ast::Program) -> String {
    let paths = program
        .apis
        .iter()
        .map(|api| {
            let method = api.method.to_ascii_lowercase();
            let path_parameters = api
                .input
                .iter()
                .filter(|field| api.path.contains(&format!("{{{}}}", field.name)))
                .map(|field| {
                    format!(
                        "{{\"name\":\"{}\",\"in\":\"path\",\"required\":true,\"schema\":{}}}",
                        json_escape(&field.name),
                        openapi_schema(&field.ty)
                    )
                })
                .collect::<Vec<_>>();
            let query_parameters = if matches!(api.method.as_str(), "GET" | "DELETE") {
                api.input
                    .iter()
                    .filter(|field| !api.path.contains(&format!("{{{}}}", field.name)))
                    .map(|field| {
                        format!(
                            "{{\"name\":\"{}\",\"in\":\"query\",\"required\":{},\"schema\":{}}}",
                            json_escape(&field.name),
                            !matches!(field.ty, Type::Option(_)),
                            openapi_schema(&field.ty)
                        )
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let request_body = if matches!(api.method.as_str(), "GET" | "DELETE") {
                String::new()
            } else {
                let properties = api
                    .input
                    .iter()
                    .filter(|field| !api.path.contains(&format!("{{{}}}", field.name)))
                    .map(|field| {
                        format!(
                            "\"{}\":{}",
                            json_escape(&field.name),
                            openapi_schema(&field.ty)
                        )
                    })
                    .collect::<Vec<_>>();
                let required = api
                    .input
                    .iter()
                    .filter(|field| {
                        !api.path.contains(&format!("{{{}}}", field.name))
                            && !matches!(field.ty, Type::Option(_))
                    })
                    .map(|field| format!("\"{}\"", json_escape(&field.name)))
                    .collect::<Vec<_>>();
                if properties.is_empty() {
                    String::new()
                } else {
                    format!(
                        "\"requestBody\":{{\"required\":true,\"content\":{{\"application/json\":{{\"schema\":{{\"type\":\"object\",\"properties\":{{{}}},\"required\":[{}]}}}}}}}},",
                        properties.join(","),
                        required.join(",")
                    )
                }
            };
            let mut parameters = path_parameters;
            parameters.extend(query_parameters);
            let security = if api.requires_auth || !api.permissions.is_empty() {
                format!(
                    ",\"x-zelyra-requires-auth\":{},\"x-zelyra-permissions\":[{}]",
                    api.requires_auth,
                    api.permissions
                        .iter()
                        .map(|permission| format!("\"{}\"", json_escape(permission)))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            } else {
                String::new()
            };
            let responses = std::iter::once(format!(
                "\"200\":{{\"description\":\"Successful response\",\"content\":{{\"application/json\":{{\"schema\":{}}}}}}}",
                openapi_schema(&api.output)
            ))
            .chain(api.errors.iter().map(|error| {
                let response = if let Some(payload) = &error.payload {
                    format!(
                        "{{\"description\":\"{}\",\"content\":{{\"application/json\":{{\"schema\":{}}}}}}}",
                        json_escape(&error.name),
                        openapi_error_schema(payload)
                    )
                } else {
                    format!("{{\"description\":\"{}\"}}", json_escape(&error.name))
                };
                format!("\"{}\":{}", error.status, response)
            }))
            .collect::<Vec<_>>()
            .join(",");
            let operation_id = format!(
                "{}_{}",
                method,
                api.path
                    .trim_matches('/')
                    .replace(['{', '}'], "")
                    .replace('/', "_")
            );
            format!(
                "\"{}\":{{\"{}\":{{\"operationId\":\"{}\",\"parameters\":[{}],{}\"responses\":{{{}}}{}}}}}",
                json_escape(&api.path),
                method,
                json_escape(&operation_id),
                parameters.join(","),
                request_body,
                responses,
                security
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let components = format_openapi_components(program);
    let compiler_version = env!("CARGO_PKG_VERSION");
    format!(
        "{{\"openapi\":\"3.0.3\",\"info\":{{\"title\":\"Zelyra API\",\"version\":\"{compiler_version}\"}},\"paths\":{{{paths}}},\"components\":{{\"schemas\":{{{components}}}}}}}"
    )
}

fn format_openapi_components(program: &zelyra_ast::Program) -> String {
    let mut components = Vec::new();
    for definition in &program.types {
        components.push(format!(
            "\"{}\":{}",
            json_escape(&definition.name),
            openapi_schema(&definition.target)
        ));
    }
    for record in &program.records {
        let properties = record
            .fields
            .iter()
            .map(|field| {
                format!(
                    "\"{}\":{}",
                    json_escape(&field.name),
                    openapi_schema(&field.ty)
                )
            })
            .collect::<Vec<_>>();
        let required = record
            .fields
            .iter()
            .filter(|field| !matches!(field.ty, Type::Option(_)))
            .map(|field| format!("\"{}\"", json_escape(&field.name)))
            .collect::<Vec<_>>();
        components.push(format!(
            "\"{}\":{{\"type\":\"object\",\"properties\":{{{}}},\"required\":[{}]}}",
            json_escape(&record.name),
            properties.join(","),
            required.join(",")
        ));
    }
    for table in &program.tables {
        let properties = table
            .columns
            .iter()
            .map(|column| {
                format!(
                    "\"{}\":{}",
                    json_escape(&column.name),
                    openapi_schema(&column.ty)
                )
            })
            .collect::<Vec<_>>();
        let required = table
            .columns
            .iter()
            .filter(|column| column.required && !matches!(column.ty, Type::Option(_)))
            .map(|column| format!("\"{}\"", json_escape(&column.name)))
            .collect::<Vec<_>>();
        let schema = format!(
            "{{\"type\":\"object\",\"properties\":{{{}}},\"required\":[{}]}}",
            properties.join(","),
            required.join(",")
        );
        components.push(format!(
            "\"{}\":{}",
            json_escape(&table.name),
            schema.clone()
        ));
        if let Some(singular) = singular_type_name(&table.name) {
            components.push(format!("\"{}\":{}", json_escape(&singular), schema));
        }
    }
    components.join(",")
}

fn format_typescript_client(program: &zelyra_ast::Program) -> String {
    let mut output = String::new();
    output.push_str("// Generated by Zelyra. Do not edit by hand.\n\n");
    output.push_str("export type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };\n\n");

    let mut error_codes = program
        .apis
        .iter()
        .flat_map(|api| api.errors.iter().map(|error| error.name.clone()))
        .collect::<Vec<_>>();
    error_codes.sort();
    error_codes.dedup();
    if error_codes.is_empty() {
        output.push_str("export type ZelyraApiErrorCode = string;\n\n");
    } else {
        let codes = error_codes
            .iter()
            .map(|code| format!("\"{}\"", json_escape(code)))
            .collect::<Vec<_>>()
            .join(" | ");
        writeln!(
            output,
            "export type ZelyraApiErrorCode = {} | (string & {{}});\n",
            codes
        )
        .expect("writing to a String cannot fail");
    }

    let mut emitted = HashSet::new();
    for definition in &program.types {
        if emitted.insert(definition.name.clone()) {
            writeln!(
                output,
                "export type {} = {};",
                definition.name,
                typescript_type(&definition.target)
            )
            .expect("writing to a String cannot fail");
        }
    }
    for record in &program.records {
        if emitted.insert(record.name.clone()) {
            writeln!(output, "export interface {} {{", record.name)
                .expect("writing to a String cannot fail");
            for field in &record.fields {
                let optional = matches!(field.ty, Type::Option(_));
                writeln!(
                    output,
                    "  {}{}: {};",
                    field.name,
                    if optional { "?" } else { "" },
                    typescript_type(&field.ty)
                )
                .expect("writing to a String cannot fail");
            }
            output.push_str("}\n\n");
        }
    }
    for table in &program.tables {
        let names = std::iter::once(table.name.clone()).chain(
            singular_type_name(&table.name)
                .into_iter()
                .filter(|name| name != &table.name),
        );
        for name in names {
            if emitted.insert(name.clone()) {
                writeln!(output, "export interface {} {{", name)
                    .expect("writing to a String cannot fail");
                for column in &table.columns {
                    let optional = !column.required || matches!(column.ty, Type::Option(_));
                    writeln!(
                        output,
                        "  {}{}: {};",
                        column.name,
                        if optional { "?" } else { "" },
                        typescript_type(&column.ty)
                    )
                    .expect("writing to a String cannot fail");
                }
                output.push_str("}\n\n");
            }
        }
    }

    let payload_types = program
        .apis
        .iter()
        .flat_map(|api| {
            api.errors.iter().filter_map(|error| {
                error
                    .payload
                    .as_ref()
                    .map(|payload| (error.name.clone(), typescript_type(payload)))
            })
        })
        .collect::<HashMap<_, _>>();
    if payload_types.is_empty() {
        output.push_str("export type ZelyraApiErrorPayload = JsonValue;\n\n");
    } else {
        output.push_str("export interface ZelyraApiErrorPayloads {\n");
        let mut payload_types = payload_types.into_iter().collect::<Vec<_>>();
        payload_types.sort_by(|left, right| left.0.cmp(&right.0));
        for (name, ty) in payload_types {
            writeln!(output, "  \"{name}\": {ty};").expect("writing to a String cannot fail");
        }
        output.push_str("}\n\nexport type ZelyraApiErrorPayload = ZelyraApiErrorPayloads[keyof ZelyraApiErrorPayloads];\n\n");
    }

    output.push_str(
        "export interface ZelyraClientOptions {\n  baseUrl: string;\n  fetch?: typeof fetch;\n  token?: string;\n}\n\n",
    );
    output.push_str(
        "export class ZelyraApiError extends Error {\n  constructor(\n    public readonly status: number,\n    public readonly code: ZelyraApiErrorCode | undefined,\n    public readonly body: string,\n    message: string,\n  ) {\n    super(message);\n  }\n\n  static async fromResponse(response: Response): Promise<ZelyraApiError> {\n    const body = await response.text();\n    let code: ZelyraApiErrorCode | undefined;\n    let message = `Zelyra API request failed (${response.status})`;\n    try {\n      const payload = JSON.parse(body) as { error?: { code?: unknown; message?: unknown } };\n      if (payload.error && typeof payload.error === \"object\") {\n        if (typeof payload.error.code === \"string\") code = payload.error.code;\n        if (typeof payload.error.message === \"string\") message = payload.error.message;\n      }\n    } catch {\n      // Keep the original response body when the server did not return JSON.
    }\n    return new ZelyraApiError(response.status, code, body, message);\n  }\n}\n\n",
    );
    output.push_str(
        "export class ZelyraClient {\n  private readonly baseUrl: string;\n  private readonly fetchImpl: typeof fetch;\n  private readonly token?: string;\n\n  constructor(options: ZelyraClientOptions) {\n    this.baseUrl = options.baseUrl.replace(/\\/$/, \"\");\n    this.fetchImpl = options.fetch ?? fetch;\n    this.token = options.token;\n  }\n\n",
    );

    for api in &program.apis {
        format_typescript_operation(&mut output, api);
    }
    output = output
        .replace(
            "export class ZelyraApiError extends Error {",
            "export class ZelyraApiError<Details = ZelyraApiErrorPayload> extends Error {",
        )
        .replace(
            "public readonly code: ZelyraApiErrorCode | undefined,\n    public readonly body: string,",
            "public readonly code: ZelyraApiErrorCode | undefined,\n    public readonly details: Details | undefined,\n    public readonly body: string,",
        )
        .replace(
            "static async fromResponse(response: Response): Promise<ZelyraApiError> {",
            "static async fromResponse<Details = ZelyraApiErrorPayload>(response: Response): Promise<ZelyraApiError<Details>> {",
        )
        .replace(
            "let code: ZelyraApiErrorCode | undefined;\n    let message",
            "let code: ZelyraApiErrorCode | undefined;\n    let details: Details | undefined;\n    let message",
        )
        .replace(
            "{ error?: { code?: unknown; message?: unknown } }",
            "{ error?: { code?: unknown; message?: unknown; details?: unknown } }",
        )
        .replace(
            "if (typeof payload.error.message === \"string\") message = payload.error.message;",
            "if (typeof payload.error.message === \"string\") message = payload.error.message;\n        details = payload.error.details as Details | undefined;",
        )
        .replace(
            "new ZelyraApiError(response.status, code, body, message)",
            "new ZelyraApiError(response.status, code, details, body, message)",
        );
    output.push_str("}\n");
    output
}

fn format_typescript_operation(output: &mut String, api: &zelyra_ast::ApiDef) {
    let method = api.method.to_ascii_uppercase();
    let operation = typescript_operation_name(api);
    let query_fields = if matches!(method.as_str(), "GET" | "DELETE") {
        api.input
            .iter()
            .filter(|field| !api.path.contains(&format!("{{{}}}", field.name)))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let body_fields = if matches!(method.as_str(), "GET" | "DELETE") {
        Vec::new()
    } else {
        api.input
            .iter()
            .filter(|field| !api.path.contains(&format!("{{{}}}", field.name)))
            .collect::<Vec<_>>()
    };
    let has_params = !api.input.is_empty();
    write!(output, "  async {}(", operation).expect("writing to a String cannot fail");
    if has_params {
        write!(output, "params: {{ ").expect("writing to a String cannot fail");
        for (index, field) in api.input.iter().enumerate() {
            if index > 0 {
                output.push(' ');
            }
            let optional = matches!(field.ty, Type::Option(_));
            write!(
                output,
                "{}{}: {};",
                field.name,
                if optional { "?" } else { "" },
                typescript_type(&field.ty)
            )
            .expect("writing to a String cannot fail");
        }
        output.push_str(" }");
    }
    writeln!(output, "): Promise<{}> {{", typescript_type(&api.output))
        .expect("writing to a String cannot fail");
    let path = typescript_path_template(&api.path);
    writeln!(output, "    let url = this.baseUrl + `{}`;", path)
        .expect("writing to a String cannot fail");
    if !query_fields.is_empty() {
        output.push_str("    const query = new URLSearchParams();\n");
        for field in query_fields {
            writeln!(
                output,
                "    if (params.{} !== undefined && params.{} !== null) query.set(\"{}\", String(params.{}));",
                field.name, field.name, field.name, field.name
            )
            .expect("writing to a String cannot fail");
        }
        output.push_str("    const queryString = query.toString();\n    if (queryString) url += `?${queryString}`;\n");
    }
    output.push_str("    const headers: Record<string, string> = {};\n    if (this.token) headers.Authorization = `Bearer ${this.token}`;\n");
    if !body_fields.is_empty() {
        output.push_str("    headers[\"Content-Type\"] = \"application/json\";\n");
    }
    writeln!(
        output,
        "    const response = await this.fetchImpl(url, {{ method: \"{}\", headers{} }});",
        method,
        if body_fields.is_empty() {
            String::new()
        } else {
            format!(
                ", body: JSON.stringify({{{}}})",
                body_fields
                    .iter()
                    .map(|field| format!("{}: params.{}", field.name, field.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    )
    .expect("writing to a String cannot fail");
    output.push_str("    if (!response.ok) throw await ZelyraApiError.fromResponse(response);\n");
    output.push_str("    return await response.json() as ");
    output.push_str(&typescript_type(&api.output));
    output.push_str(";\n  }\n\n");
}

fn typescript_type(ty: &Type) -> String {
    match ty {
        Type::Int | Type::UInt | Type::Float | Type::Decimal => "number".into(),
        Type::Bool => "boolean".into(),
        Type::String
        | Type::Char
        | Type::Bytes
        | Type::Timestamp
        | Type::Date
        | Type::Time
        | Type::Duration => "string".into(),
        Type::Unit => "void".into(),
        Type::Option(inner) => format!("{} | null", typescript_type(inner)),
        Type::Result(ok, _) => typescript_type(ok),
        Type::Array(inner) => format!("Array<{}>", typescript_type(inner)),
        Type::HttpResult(inner) => format!("HttpResult<{}>", typescript_type(inner)),
        Type::Named(name) => match name.as_str() {
            "Id" => "number".into(),
            "Email" | "Url" | "Uuid" | "Money" => "string".into(),
            "Unit" => "void".into(),
            _ => name.clone(),
        },
        Type::Unknown => "unknown".into(),
    }
}

fn typescript_operation_name(api: &zelyra_ast::ApiDef) -> String {
    let mut name = format!(
        "{}_{}",
        api.method.to_ascii_lowercase(),
        api.path.trim_matches('/').replace(['{', '}'], "")
    );
    name = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect();
    if name.ends_with('_') {
        name.pop();
    }
    if name == api.method.to_ascii_lowercase() {
        name.push_str("_root");
    }
    name
}

fn typescript_path_template(path: &str) -> String {
    let mut result = String::new();
    let mut rest = path;
    while let Some(start) = rest.find('{') {
        let (literal, after_start) = rest.split_at(start);
        result.push_str(&escape_typescript_template(literal));
        let Some(end) = after_start.find('}') else {
            result.push_str(&escape_typescript_template(after_start));
            return result;
        };
        let name = &after_start[1..end];
        result.push_str("${encodeURIComponent(String(params.");
        result.push_str(name);
        result.push_str("))}");
        rest = &after_start[end + 1..];
    }
    result.push_str(&escape_typescript_template(rest));
    result
}

fn escape_typescript_template(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${")
}

fn singular_type_name(table: &str) -> Option<String> {
    let singular = if let Some(stem) = table.strip_suffix("ies") {
        format!("{stem}y")
    } else if let Some(stem) = table.strip_suffix('s') {
        stem.to_owned()
    } else {
        table.to_owned()
    };
    let mut chars = singular.chars();
    let first = chars.next()?.to_ascii_uppercase();
    Some(std::iter::once(first).chain(chars).collect())
}

fn openapi_schema(ty: &Type) -> String {
    match ty {
        Type::Int | Type::UInt => "{\"type\":\"integer\"}".into(),
        Type::Float | Type::Decimal => "{\"type\":\"number\"}".into(),
        Type::Bool => "{\"type\":\"boolean\"}".into(),
        Type::Array(inner) => format!("{{\"type\":\"array\",\"items\":{}}}", openapi_schema(inner)),
        Type::Option(inner) => openapi_schema(inner),
        Type::Result(ok, _) => openapi_schema(ok),
        Type::HttpResult(_) => "{\"type\":\"object\"}".into(),
        Type::Named(name) => match name.as_str() {
            "Id" => "{\"type\":\"integer\",\"format\":\"int64\"}".into(),
            "Email" => "{\"type\":\"string\",\"format\":\"email\"}".into(),
            "Url" => "{\"type\":\"string\",\"format\":\"uri\"}".into(),
            "Uuid" => "{\"type\":\"string\",\"format\":\"uuid\"}".into(),
            _ => format!(
                "{{\"$ref\":\"#/components/schemas/{}\"}}",
                json_escape(name)
            ),
        },
        _ => "{\"type\":\"string\"}".into(),
    }
}

fn openapi_error_schema(payload: &Type) -> String {
    let details = serde_json::from_str(&openapi_schema(payload))
        .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));
    serde_json::json!({
        "type": "object",
        "required": ["error"],
        "properties": {
            "error": {
                "type": "object",
                "required": ["code", "message", "details"],
                "properties": {
                    "code": {"type": "string"},
                    "message": {"type": "string"},
                    "details": details,
                }
            }
        }
    })
    .to_string()
}

fn format_verification_result(path: &str, source: &str, result: &VerificationResult) -> String {
    let (end_line, end_column) = source_position(source, result.span.end);
    let header = format!(
        "{} [{}]: {}.{}[{}] ({}:{}:{}-{}:{})\n  = {}",
        result.status,
        result.status.code(),
        result.function,
        result.kind,
        result.index,
        path,
        result.span.line,
        result.span.column,
        end_line,
        end_column,
        result.message
    );
    match format_source_excerpt(source, result.span) {
        Some(excerpt) => {
            let counterexample = result
                .counterexample
                .as_ref()
                .map(|values| format!("\n  = Counterexample: {}", format_counterexample(values)))
                .unwrap_or_default();
            format!("{header}{counterexample}\n{excerpt}")
        }
        None => match &result.counterexample {
            Some(values) => format!(
                "{header}\n  = Counterexample: {}",
                format_counterexample(values)
            ),
            None => header,
        },
    }
}

fn format_verification_json(path: &str, source: &str, results: &[VerificationResult]) -> String {
    let entries = results
        .iter()
        .map(|result| {
            let (end_line, end_column) = source_position(source, result.span.end);
            let counterexample = result
                .counterexample
                .as_ref()
                .map(|values| format_counterexample_json(values))
                .unwrap_or_else(|| "null".into());
            format!(
                "{{\"status\":\"{}\",\"code\":\"{}\",\"message\":\"{}\",\"function\":\"{}\",\"kind\":\"{}\",\"index\":{},\"counterexample\":{},\"location\":{{\"file\":\"{}\",\"start\":{{\"line\":{},\"column\":{}}},\"end\":{{\"line\":{},\"column\":{}}}}}}}",
                result.status,
                result.status.code(),
                json_escape(&result.message),
                json_escape(&result.function),
                result.kind,
                result.index,
                counterexample,
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

fn format_counterexample(values: &[(String, i64)]) -> String {
    values
        .iter()
        .map(|(name, value)| format!("{name} = {value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_counterexample_json(values: &[(String, i64)]) -> String {
    let entries = values
        .iter()
        .map(|(name, value)| format!("\"{}\":{}", json_escape(name), value))
        .collect::<Vec<_>>();
    format!("{{{}}}", entries.join(","))
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

fn format_source_excerpt(source: &str, span: zelyra_ast::Span) -> Option<String> {
    let source_line = source
        .split('\n')
        .nth(span.line.checked_sub(1)?)?
        .trim_end_matches('\r');
    let start = span.column.checked_sub(1)?.min(source_line.len());
    let (end_line, end_column) = source_position(source, span.end);
    let width = if end_line == span.line {
        end_column
            .saturating_sub(span.column)
            .max(1)
            .min(source_line.len().saturating_sub(start).max(1))
    } else {
        source_line.len().saturating_sub(start).max(1)
    };
    let line_number = span.line.to_string();
    let padding = " ".repeat(line_number.len());
    let marker = format!("{}{}", " ".repeat(start), "^".repeat(width));
    Some(format!(
        "  {padding} |\n  {line_number} | {source_line}\n  {padding} | {marker}"
    ))
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

fn project_config_path(path: &str) -> Result<Option<PathBuf>, String> {
    let source_path =
        fs::canonicalize(path).map_err(|error| format!("cannot locate source: {error}"))?;
    let mut directory = source_path
        .parent()
        .ok_or_else(|| "source has no parent directory".to_owned())?;
    Ok(loop {
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
    })
}

fn project_capability_grants(path: &str) -> Result<Option<HashSet<String>>, String> {
    let Some(config_path) = project_config_path(path)? else {
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

fn parse_string_array(value: &str) -> Result<Vec<String>, String> {
    let value = value.trim();
    if !value.starts_with('[') || !value.ends_with(']') {
        return Err("value must be a TOML string array".into());
    }
    let inner = value[1..value.len() - 1].trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(',')
        .map(|item| {
            let item = item.trim();
            let Some(item) = item
                .strip_prefix('"')
                .and_then(|item| item.strip_suffix('"'))
            else {
                return Err("string arrays must contain quoted strings".into());
            };
            Ok(item.replace("\\\\", "\\").replace("\\\"", "\""))
        })
        .collect()
}

fn project_filesystem_policy(path: &str) -> Result<Option<FileSystemPolicy>, String> {
    let Some(config_path) = project_config_path(path)? else {
        return Ok(None);
    };
    let contents = fs::read_to_string(&config_path)
        .map_err(|error| format!("cannot read {}: {error}", config_path.display()))?;
    let mut read_roots = None;
    let mut write_roots = None;
    let mut seen = HashSet::new();
    let mut in_filesystem = false;
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_filesystem = line == "[filesystem]";
            continue;
        }
        if !in_filesystem {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(format!(
                "invalid filesystem setting on line {}",
                line_index + 1
            ));
        };
        let key = raw_key.trim();
        if !seen.insert(key) {
            return Err(format!(
                "filesystem setting {key} is configured more than once"
            ));
        }
        match key {
            "read_roots" => read_roots = Some(parse_string_array(raw_value)?),
            "write_roots" => write_roots = Some(parse_string_array(raw_value)?),
            _ => return Err(format!("unknown filesystem setting {key}")),
        }
    }
    let base_dir = config_path
        .parent()
        .ok_or_else(|| "project configuration has no parent directory".to_owned())?
        .to_path_buf();
    let canonical_root = |root: &str| {
        let candidate = if PathBuf::from(root).is_absolute() {
            PathBuf::from(root)
        } else {
            base_dir.join(root)
        };
        let canonical = fs::canonicalize(&candidate).map_err(|error| {
            format!(
                "filesystem root {} is not accessible: {error}",
                candidate.display()
            )
        })?;
        if !canonical.is_dir() {
            return Err(format!(
                "filesystem root {} is not a directory",
                candidate.display()
            ));
        }
        Ok(canonical)
    };
    let read_roots = read_roots
        .unwrap_or_else(|| vec![".".into()])
        .iter()
        .map(String::as_str)
        .map(canonical_root)
        .collect::<Result<Vec<_>, _>>()?;
    let write_roots = write_roots
        .unwrap_or_default()
        .iter()
        .map(String::as_str)
        .map(canonical_root)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(FileSystemPolicy {
        base_dir,
        read_roots,
        write_roots,
    }))
}

fn project_network_policy(path: &str) -> Result<Option<NetworkPolicy>, String> {
    let Some(config_path) = project_config_path(path)? else {
        return Ok(None);
    };
    let contents = fs::read_to_string(&config_path)
        .map_err(|error| format!("cannot read {}: {error}", config_path.display()))?;
    let mut allowed_hosts = None;
    let mut timeout_ms = 5_000;
    let mut max_response_bytes = 1_048_576;
    let mut seen = HashSet::new();
    let mut in_network = false;
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_network = line == "[network]";
            continue;
        }
        if !in_network {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(format!(
                "invalid network setting on line {}",
                line_index + 1
            ));
        };
        let key = raw_key.trim();
        if !seen.insert(key) {
            return Err(format!(
                "network setting {key} is configured more than once"
            ));
        }
        match key {
            "allowed_hosts" => allowed_hosts = Some(parse_string_array(raw_value)?),
            "timeout_ms" => {
                timeout_ms = raw_value.trim().parse().map_err(|_| {
                    "network setting timeout_ms must be a positive integer".to_owned()
                })?;
                if timeout_ms == 0 {
                    return Err("network setting timeout_ms must be positive".into());
                }
            }
            "max_response_bytes" => {
                max_response_bytes = raw_value.trim().parse().map_err(|_| {
                    "network setting max_response_bytes must be a positive integer".to_owned()
                })?;
                if max_response_bytes == 0 {
                    return Err("network setting max_response_bytes must be positive".into());
                }
            }
            _ => return Err(format!("unknown network setting {key}")),
        }
    }
    Ok(Some(NetworkPolicy {
        allowed_hosts: allowed_hosts.unwrap_or_default(),
        timeout_ms,
        max_response_bytes,
    }))
}

fn project_runtime_policy(path: &str) -> Result<Option<RuntimePolicy>, String> {
    let filesystem = project_filesystem_policy(path)?;
    let network = project_network_policy(path)?;
    let process = project_process_policy(path)?;
    if filesystem.is_none() && network.is_none() && process.is_none() {
        Ok(None)
    } else {
        Ok(Some(RuntimePolicy {
            filesystem,
            network,
            process,
        }))
    }
}

fn project_cors_policy(path: &str) -> Result<Option<CorsPolicy>, String> {
    let Some(config_path) = project_config_path(path)? else {
        return Ok(None);
    };
    let contents = fs::read_to_string(&config_path)
        .map_err(|error| format!("cannot read {}: {error}", config_path.display()))?;
    let mut allowed_origins = None;
    let mut allow_credentials = false;
    let mut seen = HashSet::new();
    let mut in_web = false;
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_web = line == "[web]";
            continue;
        }
        if !in_web {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(format!("invalid web setting on line {}", line_index + 1));
        };
        let key = raw_key.trim();
        if !seen.insert(key) {
            return Err(format!("web setting {key} is configured more than once"));
        }
        match key {
            "allowed_origins" => allowed_origins = Some(parse_string_array(raw_value)?),
            "allow_credentials" => {
                allow_credentials = match raw_value.trim() {
                    "true" => true,
                    "false" => false,
                    value => {
                        return Err(format!(
                            "web setting allow_credentials must be true or false, found `{value}`"
                        ));
                    }
                };
            }
            _ => return Err(format!("unknown web setting {key}")),
        }
    }
    let Some(allowed_origins) = allowed_origins else {
        return Ok(None);
    };
    CorsPolicy::new(allowed_origins, allow_credentials)
        .map(Some)
        .map_err(|error| error.message)
}

fn project_process_policy(path: &str) -> Result<Option<ProcessPolicy>, String> {
    let Some(config_path) = project_config_path(path)? else {
        return Ok(None);
    };
    let contents = fs::read_to_string(&config_path)
        .map_err(|error| format!("cannot read {}: {error}", config_path.display()))?;
    let mut allowed_commands = None;
    let mut timeout_ms = 5_000;
    let mut max_output_bytes = 1_048_576;
    let mut seen = HashSet::new();
    let mut in_process = false;
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_process = line == "[process]";
            continue;
        }
        if !in_process {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(format!(
                "invalid process setting on line {}",
                line_index + 1
            ));
        };
        let key = raw_key.trim();
        if !seen.insert(key) {
            return Err(format!(
                "process setting {key} is configured more than once"
            ));
        }
        match key {
            "allowed_commands" => allowed_commands = Some(parse_string_array(raw_value)?),
            "timeout_ms" => {
                timeout_ms = raw_value.trim().parse().map_err(|_| {
                    "process setting timeout_ms must be a positive integer".to_owned()
                })?;
                if timeout_ms == 0 {
                    return Err("process setting timeout_ms must be positive".into());
                }
            }
            "max_output_bytes" => {
                max_output_bytes = raw_value.trim().parse().map_err(|_| {
                    "process setting max_output_bytes must be a positive integer".to_owned()
                })?;
                if max_output_bytes == 0 {
                    return Err("process setting max_output_bytes must be positive".into());
                }
            }
            _ => return Err(format!("unknown process setting {key}")),
        }
    }
    Ok(Some(ProcessPolicy {
        allowed_commands: allowed_commands.unwrap_or_default(),
        timeout_ms,
        max_output_bytes,
    }))
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
            .any(|crud| crud.requires_auth || !crud.permissions.is_empty())
        || program
            .apis
            .iter()
            .any(|api| api.requires_auth || !api.permissions.is_empty());
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
        "setup" | "bootstrap" => {
            let Ok(url) = env::var("DATABASE_URL") else {
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
                Backend::MariaDb => create_mariadb_database(&url)
                    .and_then(|()| apply_mariadb(&url, &schema.create_sql())),
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
    let capability_grants = match project_capability_grants(&path) {
        Ok(grants) => grants,
        Err(error) => {
            diagnostic(&path, "E-CAP-002", &error, 1, 1);
            return ExitCode::from(1);
        }
    };
    let runtime_policy = match project_runtime_policy(&path) {
        Ok(policy) => policy,
        Err(error) => {
            diagnostic(&path, "E-POLICY-002", &error, 1, 1);
            return ExitCode::from(1);
        }
    };
    let cors_policy = match project_cors_policy(&path) {
        Ok(policy) => policy,
        Err(error) => {
            diagnostic(&path, "E-WEB-004", &error, 1, 1);
            return ExitCode::from(1);
        }
    };
    if program.pages.is_empty()
        && program.forms.is_empty()
        && program.cruds.is_empty()
        && program.apis.is_empty()
    {
        eprintln!("error[E-WEB-001]: {path} does not define a page, form, CRUD resource, or API");
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
    let database_capability_granted = capability_grants
        .as_ref()
        .is_none_or(|grants| grants.contains("Database"));
    let api_routes = generated_api_routes(
        &program,
        capability_grants.as_ref(),
        runtime_policy.as_ref(),
    );
    eprintln!("Zelyra server listening on http://{address}");
    let app = WebApp::with_database_url(routes, form_routes, env::var("DATABASE_URL").ok())
        .with_database_capability(database_capability_granted)
        .with_apis(api_routes)
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
    let app = if let Some(cors_policy) = cors_policy {
        app.with_cors(cors_policy)
    } else {
        app
    };
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

fn generated_api_routes(
    program: &zelyra_ast::Program,
    capability_grants: Option<&HashSet<String>>,
    runtime_policy: Option<&RuntimePolicy>,
) -> Vec<ApiRoute> {
    let database_url = env::var("DATABASE_URL").ok();
    program
        .apis
        .iter()
        .filter_map(|api| {
            let handler = api.handler.as_ref()?.clone();
            let api = api.clone();
            let program = program.clone();
            let database_url = database_url.clone();
            let capability_grants = capability_grants.cloned();
            let runtime_policy = runtime_policy.cloned();
            let requires_auth = api.requires_auth;
            let permissions = api.permissions.clone();
            Some(
                ApiRoute::new(
                    api.method.clone(),
                    api.path.clone(),
                    move |request, path_params| {
                        dispatch_api_with_capabilities(
                            &program,
                            &api,
                            &handler,
                            request,
                            path_params,
                            ApiRuntimeContext {
                                database_url: database_url.as_deref(),
                                capability_grants: capability_grants.as_ref(),
                                runtime_policy: runtime_policy.as_ref(),
                            },
                        )
                    },
                )
                .with_auth(requires_auth, permissions),
            )
        })
        .collect()
}

#[cfg(test)]
fn dispatch_api(
    program: &zelyra_ast::Program,
    api: &zelyra_ast::ApiDef,
    handler: &str,
    request: &zelyra_web::Request,
    path_params: &HashMap<String, String>,
    database_url: Option<&str>,
) -> Response {
    dispatch_api_with_capabilities(
        program,
        api,
        handler,
        request,
        path_params,
        ApiRuntimeContext {
            database_url,
            capability_grants: None,
            runtime_policy: None,
        },
    )
}

#[derive(Clone, Copy)]
struct ApiRuntimeContext<'a> {
    database_url: Option<&'a str>,
    capability_grants: Option<&'a HashSet<String>>,
    runtime_policy: Option<&'a RuntimePolicy>,
}

fn dispatch_api_with_capabilities(
    program: &zelyra_ast::Program,
    api: &zelyra_ast::ApiDef,
    handler: &str,
    request: &zelyra_web::Request,
    path_params: &HashMap<String, String>,
    context: ApiRuntimeContext<'_>,
) -> Response {
    if !matches!(api.method.as_str(), "GET" | "DELETE") && !request.body.trim().is_empty() {
        if let Some(content_type) = request.headers.get("content-type") {
            let media_type = content_type
                .split(';')
                .next()
                .map(str::trim)
                .unwrap_or_default()
                .to_ascii_lowercase();
            if media_type != "application/json" && media_type != "application/x-www-form-urlencoded"
            {
                return api_error_response(
                    415,
                    "UnsupportedMediaType",
                    "API request bodies must use application/json or application/x-www-form-urlencoded",
                );
            }
        }
    }
    let values = if matches!(api.method.as_str(), "GET" | "DELETE") {
        request.target.split_once('?').map_or_else(
            || Ok(HashMap::new()),
            |(_, query)| parse_api_url_values(query),
        )
    } else if request
        .headers
        .get("content-type")
        .is_some_and(|content_type| content_type.starts_with("application/json"))
    {
        parse_api_json_object(&request.body).map_err(|message| zelyra_web::HttpError { message })
    } else {
        parse_api_url_values(&request.body)
    };
    let mut values = match values {
        Ok(values) => values,
        Err(error) => return api_error_response(400, "BadRequest", &error.to_string()),
    };
    for (name, value) in path_params {
        values.insert(name.clone(), serde_json::Value::String(value.clone()));
    }
    let mut arguments = Vec::new();
    for field in &api.input {
        let Some(value) = values.get(&field.name) else {
            if matches!(field.ty, Type::Option(_)) {
                arguments.push(Value::Option(None));
                continue;
            }
            return api_error_response(
                400,
                "BadRequest",
                &format!("missing API input `{}`", field.name),
            );
        };
        match api_value_json(value, &field.ty, program) {
            Ok(value) => arguments.push(value),
            Err(error) => return api_error_response(400, "BadRequest", &error),
        }
    }
    match execute_function_with_capabilities_and_policies(
        program,
        handler,
        arguments,
        context.database_url,
        context.capability_grants,
        context.runtime_policy,
    ) {
        Ok(value) => api_result_response(api, &value),
        Err(error) => api_error_response(500, "InternalServerError", &error.message),
    }
}

fn api_result_response(api: &zelyra_ast::ApiDef, value: &Value) -> Response {
    if let Value::Result(Err(error)) = value {
        let error_name = match &**error {
            Value::Object { type_name, .. } => type_name.clone(),
            _ => error.output(),
        };
        if let Some(declaration) = api.errors.iter().find(|declaration| {
            declaration.name == error_name
                || declaration.payload.as_ref().is_some_and(|payload| {
                    payload == &error.ty()
                        || matches!(payload, Type::Named(name) if name == &error_name)
                })
        }) {
            return api_error_response_with_details(
                declaration.status,
                &declaration.name,
                &format!("API handler returned {}", declaration.name),
                declaration.payload.as_ref().map(|_| &**error),
            );
        }
        return api_error_response(
            500,
            "InternalServerError",
            &format!("unmapped API error `{error_name}`"),
        );
    }
    Response::json(200, api_json_value(value))
}

fn api_value_json(
    value: &serde_json::Value,
    ty: &Type,
    program: &zelyra_ast::Program,
) -> Result<Value, String> {
    if let Type::Option(inner) = ty {
        if value.is_null() {
            return Ok(Value::Option(None));
        }
        return Ok(Value::Option(Some(Box::new(api_value_json(
            value, inner, program,
        )?))));
    }
    if let Type::Named(name) = ty {
        if let Some(definition) = program
            .types
            .iter()
            .find(|definition| definition.name == *name)
        {
            return api_value_json(value, &definition.target, program);
        }
        if let Some(record) = program.records.iter().find(|record| record.name == *name) {
            return api_record_value(value, record, program);
        }
    }
    match ty {
        Type::Array(inner) => {
            let Some(values) = value.as_array() else {
                return Err("expected a JSON array".into());
            };
            values
                .iter()
                .map(|value| api_value_json(value, inner, program))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
        }
        Type::Int => value
            .as_i64()
            .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
            .map(Value::Int)
            .ok_or_else(|| format!("invalid Int JSON value `{value}`")),
        Type::UInt => value
            .as_u64()
            .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
            .map(Value::UInt)
            .ok_or_else(|| format!("invalid UInt JSON value `{value}`")),
        Type::Float | Type::Decimal => value
            .as_f64()
            .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
            .map(Value::Float)
            .ok_or_else(|| format!("invalid numeric JSON value `{value}`")),
        Type::Bool => value
            .as_bool()
            .or_else(|| {
                value.as_str().and_then(|value| match value {
                    "true" | "1" => Some(true),
                    "false" | "0" => Some(false),
                    _ => None,
                })
            })
            .map(Value::Bool)
            .ok_or_else(|| format!("invalid Bool JSON value `{value}`")),
        _ => value
            .as_str()
            .map(|value| Value::String(value.to_owned()))
            .or_else(|| {
                if value.is_number() || value.is_boolean() {
                    Some(Value::String(value.to_string()))
                } else {
                    None
                }
            })
            .ok_or_else(|| format!("expected a scalar JSON value, found `{value}`")),
    }
}

fn api_record_value(
    value: &serde_json::Value,
    record: &zelyra_ast::RecordDef,
    program: &zelyra_ast::Program,
) -> Result<Value, String> {
    let Some(object) = value.as_object() else {
        return Err(format!("expected JSON object for record `{}`", record.name));
    };
    for field in object.keys() {
        if !record
            .fields
            .iter()
            .any(|candidate| candidate.name == *field)
        {
            return Err(format!(
                "unknown field `{field}` in record `{}`",
                record.name
            ));
        }
    }
    let mut fields = HashMap::new();
    for field in &record.fields {
        let Some(value) = object.get(&field.name) else {
            if matches!(field.ty, Type::Option(_)) {
                fields.insert(field.name.clone(), Value::Option(None));
                continue;
            }
            return Err(format!(
                "missing field `{}` in record `{}`",
                field.name, record.name
            ));
        };
        fields.insert(
            field.name.clone(),
            api_value_json(value, &field.ty, program)?,
        );
    }
    Ok(Value::Object {
        type_name: record.name.clone(),
        fields,
    })
}

fn api_json_value(value: &Value) -> String {
    api_json_value_node(value).to_string()
}

fn api_json_value_node(value: &Value) -> serde_json::Value {
    match value {
        Value::Int(value) => serde_json::Value::from(*value),
        Value::UInt(value) => serde_json::Value::from(*value),
        Value::Float(value) => serde_json::Number::from_f64(*value)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::Bool(value) => serde_json::Value::from(*value),
        Value::String(value) => serde_json::Value::String(value.clone()),
        Value::Char(value) => serde_json::Value::String(value.to_string()),
        Value::Timestamp(value) => serde_json::Value::from(*value),
        Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(api_json_value_node).collect())
        }
        Value::Object { fields, .. } => {
            let object = fields
                .iter()
                .map(|(name, value)| (name.clone(), api_json_value_node(value)))
                .collect();
            serde_json::Value::Object(object)
        }
        Value::Option(Some(value)) => api_json_value_node(value),
        Value::Option(None) => serde_json::Value::Null,
        Value::Result(Ok(value)) => api_json_value_node(value),
        Value::Result(Err(value)) => {
            let mut object = serde_json::Map::new();
            object.insert("error".into(), api_json_value_node(value));
            serde_json::Value::Object(object)
        }
        Value::Rows { columns, rows } => serde_json::Value::Array(
            rows.iter()
                .map(|row| {
                    let object = columns
                        .iter()
                        .zip(row)
                        .map(|(column, value)| {
                            (column.clone(), serde_json::Value::String(value.clone()))
                        })
                        .collect();
                    serde_json::Value::Object(object)
                })
                .collect(),
        ),
        Value::Unit => serde_json::Value::Null,
    }
}

fn api_error_response(status: u16, code: &str, message: &str) -> Response {
    api_error_response_with_details(status, code, message, None)
}

fn api_error_response_with_details(
    status: u16,
    code: &str,
    message: &str,
    details: Option<&Value>,
) -> Response {
    let mut error = serde_json::Map::new();
    error.insert("code".into(), serde_json::Value::String(code.into()));
    error.insert("message".into(), serde_json::Value::String(message.into()));
    if let Some(details) = details {
        error.insert("details".into(), api_json_value_node(details));
    }
    let mut response = serde_json::Map::new();
    response.insert("error".into(), serde_json::Value::Object(error));
    Response::json(status, serde_json::Value::Object(response).to_string())
}

fn parse_api_json_object(source: &str) -> Result<HashMap<String, serde_json::Value>, String> {
    let value: serde_json::Value = serde_json::from_str(source)
        .map_err(|error| format!("invalid JSON request body: {error}"))?;
    let serde_json::Value::Object(object) = value else {
        return Err("JSON request body must be an object".into());
    };
    object.into_iter().map(Ok).collect()
}

fn parse_api_url_values(
    source: &str,
) -> Result<HashMap<String, serde_json::Value>, zelyra_web::HttpError> {
    parse_urlencoded(source).map(|values| {
        values
            .into_iter()
            .map(|(name, value)| (name, serde_json::Value::String(value)))
            .collect()
    })
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

fn auth_usage() {
    eprintln!(
        "Usage:\n  zelyra auth hash-password\n  zelyra auth hash-password --stdin\n\nThe interactive form does not echo passwords. Use --stdin for automation."
    );
}

fn password_from_stdin() -> Result<String, String> {
    let mut password = String::new();
    std::io::stdin()
        .read_line(&mut password)
        .map_err(|error| format!("cannot read password from stdin: {error}"))?;
    Ok(password.trim_end_matches(['\r', '\n']).to_owned())
}

fn auth_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    if args.next().as_deref() != Some("hash-password") {
        auth_usage();
        return ExitCode::from(2);
    }
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
    if command == "auth" {
        return auth_command(args);
    }
    if command == "new" {
        let Some(path) = args.next() else {
            usage();
            return ExitCode::from(2);
        };
        let mut with_mariadb = false;
        for argument in args {
            if argument == "--mariadb" && !with_mariadb {
                with_mariadb = true;
            } else {
                usage();
                return ExitCode::from(2);
            }
        }
        return create_project(&path, false, with_mariadb);
    }
    if command == "init" {
        let mut path = ".".to_owned();
        let mut path_given = false;
        let mut with_mariadb = false;
        for argument in args {
            if argument == "--mariadb" && !with_mariadb {
                with_mariadb = true;
            } else if !argument.starts_with('-') && !path_given {
                path = argument;
                path_given = true;
            } else {
                usage();
                return ExitCode::from(2);
            }
        }
        return create_project(&path, true, with_mariadb);
    }
    if command == "serve" {
        return serve_command(args);
    }
    if command == "doctor" {
        return doctor_command(args);
    }
    if command == "doc" {
        return doc_command(args);
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
            let grants = match project_capability_grants(&path) {
                Ok(grants) => grants,
                Err(error) => {
                    diagnostic(&path, "E-CAP-002", &error, 1, 1);
                    return Err(());
                }
            };
            let runtime_policy = match project_runtime_policy(&path) {
                Ok(policy) => policy,
                Err(error) => {
                    diagnostic(&path, "E-POLICY-002", &error, 1, 1);
                    return Err(());
                }
            };
            let result = match env::var("DATABASE_URL") {
                Ok(database_url) => execute_with_database_and_capabilities_and_policies(
                    &program,
                    &database_url,
                    grants.as_ref(),
                    runtime_policy.as_ref(),
                ),
                Err(_) => execute_with_capabilities_and_policies(
                    &program,
                    grants.as_ref(),
                    runtime_policy.as_ref(),
                ),
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
            message: "The verifier proved this condition for all analyzed paths.".into(),
            counterexample: None,
        };
        assert_eq!(
            format_verification_result("src/reduce.zyl", "first\nsecond value\n", &result),
            "PROVEN [V-001]: reduce.invariant[0] (src/reduce.zyl:2:1-2:7)\n  = The verifier proved this condition for all analyzed paths.\n    |\n  2 | second value\n    | ^^^^^^"
        );
    }

    #[test]
    fn doctor_rejects_invalid_port() {
        let arguments = ["--port".to_owned(), "not-a-port".to_owned()];
        assert_eq!(doctor_command(arguments.into_iter()), ExitCode::from(2));
    }

    #[test]
    fn formats_doctor_json_without_database_credentials() {
        let checks = vec![
            DoctorCheck {
                name: "project_file",
                status: "pass",
                message: "app.zyl exists".into(),
            },
            DoctorCheck {
                name: "database",
                status: "warn",
                message: "mariadb: DATABASE_URL is not set".into(),
            },
        ];
        let document: serde_json::Value =
            serde_json::from_str(&format_doctor_json("app.zyl", &checks)).unwrap();
        assert_eq!(document["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(document["status"], "ready");
        assert_eq!(document["warnings"], 1);
        assert_eq!(document["checks"][1]["status"], "warn");
        assert!(!format_doctor_json("app.zyl", &checks).contains("password"));
    }

    #[test]
    fn formats_verification_results_as_json() {
        let result = VerificationResult {
            function: "say\"hello".into(),
            kind: zelyra_runtime::ContractKind::Ensures,
            index: 1,
            status: VerificationStatus::RuntimeCheck,
            span: zelyra_ast::Span::new(6, 12, 2, 1),
            message: "This postcondition needs a runtime check because not all return paths are symbolically modeled.".into(),
            counterexample: Some(vec![("value".into(), 0)]),
        };
        assert_eq!(
            format_verification_json("src/file.zyl", "first\nsecond value\n", &[result]),
            r#"[{"status":"RUNTIME_CHECK","code":"V-002","message":"This postcondition needs a runtime check because not all return paths are symbolically modeled.","function":"say\"hello","kind":"ensures","index":1,"counterexample":{"value":0},"location":{"file":"src/file.zyl","start":{"line":2,"column":1},"end":{"line":2,"column":7}}}]"#
        );
    }

    #[test]
    fn formats_api_declarations_as_openapi() {
        let program = parse(
            &lex(
                "type CustomerId = Id table customers { id: CustomerId primary auto name: String(100) required } api GET \"/customers/{id}\" { input { id: CustomerId } output Customer errors { 404 NotFound } } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let document = format_openapi(&program);
        assert!(document.contains("\"openapi\":\"3.0.3\""));
        assert!(document.contains("\"/customers/{id}\""));
        assert!(document.contains("\"404\":{\"description\":\"NotFound\"}"));
        assert!(document.contains("#/components/schemas/Customer"));
    }

    #[test]
    fn formats_typed_typescript_client_from_api_and_records() {
        let program = parse(
            &lex(
                "struct Address { city: String } api GET \"/customers/{id}\" { input { id: Id, term: String? } output Address errors { 404 NotFound } } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let client = format_typescript_client(&program);
        assert!(client.contains("export interface Address"));
        assert!(client.contains("async get_customers_id"));
        assert!(client.contains("encodeURIComponent(String(params.id))"));
        assert!(client.contains("Promise<Address>"));
        assert!(client.contains("new URLSearchParams()"));
        assert!(client.contains("ZelyraApiError"));
        assert!(client.contains("ZelyraApiErrorCode = \"NotFound\" | (string & {})"));
        assert!(client.contains("fromResponse(response)"));
        assert!(client.contains("payload.error.message"));
    }

    #[test]
    fn formats_typed_api_error_payloads_for_openapi_and_typescript() {
        let program = parse(
            &lex(
                "struct Problem { message: String } api GET \"/fail\" { output Result<String, Problem> errors { 422 Validation: Problem } } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let openapi = format_openapi(&program);
        assert!(openapi.contains("\"details\""));
        assert!(openapi.contains("#/components/schemas/Problem"));
        let client = format_typescript_client(&program);
        assert!(client.contains("ZelyraApiErrorPayloads"));
        assert!(client.contains("\"Validation\": Problem"));
        assert!(client.contains("details: Details | undefined"));
    }

    #[test]
    fn dispatches_json_api_input_to_a_typed_handler() {
        let program = parse(
            &lex(
                "api POST \"/echo\" { handler echo input { value: Int } output Int } fn echo(value: Int) -> Int { return value } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        assert!(check_apis(&program).is_ok());
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\r\n{\"value\":42}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "application/json; charset=utf-8");
        assert_eq!(response.body, "42");
    }

    #[test]
    fn returns_structured_json_for_invalid_api_input() {
        let program = parse(
            &lex(
                "api POST \"/echo\" { handler echo input { value: Int } output Int } fn echo(value: Int) -> Int { return value } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /echo HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 400);
        assert_eq!(response.content_type, "application/json; charset=utf-8");
        assert!(response.body.contains("\"code\":\"BadRequest\""));
        assert!(response.body.contains("missing API input"));
    }

    #[test]
    fn rejects_unsupported_api_request_media_types() {
        let program = parse(
            &lex(
                "api POST \"/echo\" { handler echo input { value: String } output String } fn echo(value: String) -> String { return value } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /echo HTTP/1.1\r\nContent-Type: text/plain\r\n\r\nhello",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 415);
        assert!(response.body.contains("UnsupportedMediaType"));
    }

    #[test]
    fn decodes_json_strings_with_commas_colons_and_escapes() {
        let program = parse(
            &lex(
                "api POST \"/echo\" { handler echo input { value: String } output String } fn echo(value: String) -> String { return value } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /echo HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"value\":\"a,b: \\\"quoted\\\"\"}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "\"a,b: \\\"quoted\\\"\"");
    }

    #[test]
    fn decodes_json_booleans_and_null_options() {
        let program = parse(
            &lex(
                "api POST \"/echo\" { handler echo input { active: Bool? } output Bool? } fn echo(active: Bool?) -> Bool? { return active } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /echo HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"active\":null}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "null");
    }

    #[test]
    fn decodes_and_serializes_typed_json_arrays() {
        let program = parse(
            &lex(
                "api POST \"/echo\" { handler echo input { values: Int[] } output Int[] } fn echo(values: Int[]) -> Int[] { return values } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        assert!(check_apis(&program).is_ok());
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /echo HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"values\":[1,2,3]}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "[1,2,3]");
    }

    #[test]
    fn decodes_and_serializes_nested_structured_api_objects() {
        let program = parse(
            &lex(
                "struct Address { city: String } struct CustomerInput { name: String address: Address } api POST \"/customers\" { handler echo input { customer: CustomerInput } output CustomerInput } fn echo(customer: CustomerInput) -> CustomerInput { return customer } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        assert!(check_apis(&program).is_ok());
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /customers HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"customer\":{\"name\":\"Anna\",\"address\":{\"city\":\"Berlin\"}}}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 200);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["name"], "Anna");
        assert_eq!(body["address"]["city"], "Berlin");
    }

    #[test]
    fn rejects_unknown_and_missing_record_fields() {
        let program = parse(
            &lex(
                "struct CustomerInput { name: String email: Email? } api POST \"/customers\" { handler echo input { customer: CustomerInput } output CustomerInput } fn echo(customer: CustomerInput) -> CustomerInput { return customer } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        let unknown = zelyra_web::parse_request(
            "POST /customers HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"customer\":{\"name\":\"Anna\",\"unknown\":true}}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &unknown, &HashMap::new(), None);
        assert_eq!(response.status, 400);
        assert!(response.body.contains("unknown field"));

        let missing = zelyra_web::parse_request(
            "POST /customers HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"customer\":{}}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &missing, &HashMap::new(), None);
        assert_eq!(response.status, 400);
        assert!(response.body.contains("missing field"));
    }

    #[test]
    fn maps_declared_result_errors_to_http_responses() {
        let program = parse(
            &lex(
                "api GET \"/customers\" { handler find input { } output Result<String, String> errors { 404 NotFound } } fn find() -> Result<String, String> { return Err(\"NotFound\") } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        let request = zelyra_web::parse_request("GET /customers HTTP/1.1\r\n\r\n").unwrap();
        let response = dispatch_api(&program, api, "find", &request, &HashMap::new(), None);
        assert_eq!(response.status, 404);
        assert!(response.body.contains("\"code\":\"NotFound\""));
    }

    #[test]
    fn returns_internal_error_for_undeclared_result_errors() {
        let program = parse(
            &lex(
                "api GET \"/customers\" { handler find input { } output Result<String, String> errors { 404 NotFound } } fn find() -> Result<String, String> { return Err(\"Other\") } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        let request = zelyra_web::parse_request("GET /customers HTTP/1.1\r\n\r\n").unwrap();
        let response = dispatch_api(&program, api, "find", &request, &HashMap::new(), None);
        assert_eq!(response.status, 500);
        assert!(response.body.contains("InternalServerError"));
        assert!(response.body.contains("unmapped API error"));
    }

    #[test]
    fn returns_typed_details_for_declared_api_errors() {
        let program = parse(
            &lex(
                "struct Problem { message: String } api GET \"/fail\" { handler fail input { } output Result<String, Problem> errors { 422 Validation: Problem } } fn fail() -> Result<String, Problem> { return Err(Problem { message: \"invalid customer\" }) } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        assert!(check_apis(&program).is_ok());
        let api = &program.apis[0];
        let request = zelyra_web::parse_request("GET /fail HTTP/1.1\r\n\r\n").unwrap();
        let response = dispatch_api(&program, api, "fail", &request, &HashMap::new(), None);
        assert_eq!(response.status, 422);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["error"]["code"], "Validation");
        assert_eq!(body["error"]["details"]["message"], "invalid customer");
    }

    #[test]
    fn rejects_api_error_payload_that_does_not_match_result_error_type() {
        let program = parse(
            &lex(
                "struct Problem { message: String } struct Other { code: Int } api GET \"/fail\" { handler fail input { } output Result<String, Problem> errors { 422 Validation: Other } } fn fail() -> Result<String, Problem> { return Err(Problem { message: \"invalid\" }) } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let errors = check_apis(&program).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("does not match handler error type")));
    }

    #[test]
    fn rejects_object_json_for_scalar_input() {
        let program = parse(
            &lex(
                "api POST \"/echo\" { handler echo input { value: String } output String } fn echo(value: String) -> String { return value } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /echo HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"value\":{\"nested\":true}}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 400);
        assert!(response.body.contains("scalar JSON value"));
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
    fn rejects_protected_api_without_auth_definition() {
        let source = r#"
            api GET "/admin" {
                requires auth
                output String
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

    #[test]
    fn defaults_project_file_system_policy_to_project_root() {
        let policy = project_filesystem_policy("../examples/filesystem_api.zyl")
            .unwrap()
            .unwrap();
        assert!(policy
            .read_roots
            .iter()
            .any(|root| root.ends_with("zelyra")));
        assert!(policy.write_roots.is_empty());
    }

    #[test]
    fn defaults_project_network_policy_to_no_allowed_hosts() {
        let policy = project_network_policy("../examples/filesystem_api.zyl")
            .unwrap()
            .unwrap();
        assert!(policy.allowed_hosts.is_empty());
        assert_eq!(policy.timeout_ms, 5_000);
        assert_eq!(policy.max_response_bytes, 1_048_576);
    }

    #[test]
    fn defaults_project_process_policy_to_no_allowed_commands() {
        let policy = project_process_policy("../examples/filesystem_api.zyl")
            .unwrap()
            .unwrap();
        assert!(policy.allowed_commands.is_empty());
        assert_eq!(policy.timeout_ms, 5_000);
        assert_eq!(policy.max_output_bytes, 1_048_576);
    }

    #[test]
    fn generates_dockerfile_with_published_release_ref() {
        let path = env::temp_dir().join(format!(
            "zelyra-cli-template-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let status = create_project(path.to_str().unwrap(), false, true);
        assert_eq!(status, ExitCode::SUCCESS);

        let dockerfile = fs::read_to_string(path.join("Dockerfile")).unwrap();
        assert!(dockerfile.contains("ARG ZELYRA_REF=v0.1.37-alpha.1"));

        fs::remove_dir_all(path).unwrap();
    }
}
