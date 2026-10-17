use super::*;

fn parse_entry_arguments(
    entry: &str,
    parameters: &[zelyra_ast::Param],
    raw_arguments: Option<&str>,
    program: &zelyra_ast::Program,
) -> Result<Vec<RuntimeValue>, String> {
    let json_arguments = match raw_arguments {
        Some(raw) => serde_json::from_str::<Vec<serde_json::Value>>(raw)
            .map_err(|error| format!("--entry-args must be a JSON array: {error}"))?,
        None if parameters.is_empty() => Vec::new(),
        None => {
            return Err(format!(
                "entry function `{entry}` expects {} argument(s); provide them with --entry-args '<JSON array>'",
                parameters.len()
            ));
        }
    };
    if json_arguments.len() != parameters.len() {
        return Err(format!(
            "entry function `{entry}` expects {} argument(s), received {}",
            parameters.len(),
            json_arguments.len()
        ));
    }
    parameters
        .iter()
        .zip(json_arguments.iter())
        .map(|(parameter, value)| {
            entry_argument_value(value, &parameter.ty, program, 0).map_err(|reason| {
                format!(
                    "entry argument `{}` does not match `{}`: {reason}",
                    parameter.name, parameter.ty
                )
            })
        })
        .collect()
}

fn entry_argument_value(
    value: &serde_json::Value,
    ty: &zelyra_ast::Type,
    program: &zelyra_ast::Program,
    depth: usize,
) -> Result<RuntimeValue, String> {
    use zelyra_ast::Type;

    if depth >= 64 {
        return Err("argument type nesting exceeds the supported limit".into());
    }
    match ty {
        Type::Int => value
            .as_i64()
            .map(RuntimeValue::Int)
            .ok_or_else(|| "expected a JSON integer in the signed 64-bit range".into()),
        Type::UInt => value.as_u64().map(RuntimeValue::UInt).ok_or_else(|| {
            "expected a non-negative JSON integer in the unsigned 64-bit range".into()
        }),
        Type::Float => value
            .as_f64()
            .filter(|number| number.is_finite())
            .map(RuntimeValue::Float)
            .ok_or_else(|| "expected a finite JSON number".into()),
        Type::Bool => value
            .as_bool()
            .map(RuntimeValue::Bool)
            .ok_or_else(|| "expected a JSON boolean".into()),
        Type::String => value
            .as_str()
            .map(|value| RuntimeValue::String(value.to_owned()))
            .ok_or_else(|| "expected a JSON string".into()),
        Type::Char => {
            let value = value
                .as_str()
                .ok_or_else(|| "expected a one-character JSON string".to_owned())?;
            let mut chars = value.chars();
            match (chars.next(), chars.next()) {
                (Some(character), None) => Ok(RuntimeValue::Char(character)),
                _ => Err("expected a one-character JSON string".into()),
            }
        }
        Type::Json => Ok(RuntimeValue::Json(value.clone())),
        Type::Option(inner) if value.is_null() => Ok(RuntimeValue::Option(None)),
        Type::Option(inner) => entry_argument_value(value, inner, program, depth + 1)
            .map(|value| RuntimeValue::Option(Some(Box::new(value)))),
        Type::Array(inner) => {
            let values = value
                .as_array()
                .ok_or_else(|| "expected a JSON array".to_owned())?;
            values
                .iter()
                .map(|value| entry_argument_value(value, inner, program, depth + 1))
                .collect::<Result<Vec<_>, _>>()
                .map(RuntimeValue::Array)
        }
        Type::Named(name) => {
            let alias = program
                .types
                .iter()
                .find(|definition| definition.name == *name)
                .ok_or_else(|| {
                    format!("named type `{name}` is not a supported JSON entry-argument type")
                })?;
            entry_argument_value(value, &alias.target, program, depth + 1)
        }
        Type::Decimal => Err(
            "Decimal arguments are not accepted as JSON numbers because that could lose precision"
                .into(),
        ),
        _ => Err("this parameter type is not supported by --entry-args".into()),
    }
}

pub(super) fn run() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        usage();
        return ExitCode::from(2);
    };
    if command == "--help" || command == "-h" {
        usage();
        return ExitCode::SUCCESS;
    }
    if command == "--version" || command == "-V" || command == "version" {
        if args.next().is_some() {
            usage();
            return ExitCode::from(2);
        }
        return version_command();
    }
    let arguments = args.collect::<Vec<_>>();
    if help::print_if_requested(&command, &arguments) {
        return ExitCode::SUCCESS;
    }
    let mut args = arguments.into_iter();
    if command == "update" {
        let check_only = match (args.next(), args.next()) {
            (None, None) => false,
            (Some(flag), None) if flag == "--check" => true,
            _ => {
                usage();
                return ExitCode::from(2);
            }
        };
        return updater::command(check_only);
    }
    if command == "uninstall" {
        return uninstall::command(args);
    }
    if command == "--tutorial" {
        return tutorial::command(args);
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
    if command == "audit" {
        return audit_command(args);
    }
    if command == "outbox" {
        return outbox_cli::outbox_command(args);
    }
    if command == "task" {
        return task_cli::task_command(args);
    }
    if command == "schedule" {
        return task_cli::schedule_command(args);
    }
    if command == "editor" {
        return editor::command(args);
    }
    if command == "lsp" {
        if args.next().is_some() {
            usage();
            return ExitCode::from(2);
        }
        return lsp::command();
    }
    if command == "setup" {
        let mut path = ".".to_owned();
        let mut path_given = false;
        let mut web = false;
        let mut action = "prepare";
        let mut web_port = DEFAULT_SETUP_WEB_PORT;
        let mut web_port_given = false;
        let mut setup_options = SetupOptions::default();
        let mut arguments = args;
        while let Some(argument) = arguments.next() {
            if argument == "--web" {
                web = true;
            } else if argument == "--database" {
                action = "database";
            } else if argument == "--schema" {
                action = "schema";
            } else if argument == "--all" {
                action = "all";
            } else if argument == "--port" {
                let Some(value) = arguments.next() else {
                    usage();
                    return ExitCode::from(2);
                };
                web_port_given = true;
                web_port = match parse_web_port(&value) {
                    Ok(port) => port,
                    Err(error) => {
                        eprintln!("error[E-SETUP-WEB-001]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else if argument == "--host-port" {
                let Some(value) = arguments.next() else {
                    usage();
                    return ExitCode::from(2);
                };
                setup_options.host_port = match parse_web_port(&value) {
                    Ok(port) => Some(port),
                    Err(error) => {
                        eprintln!("error[E-SETUP-002]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else if argument == "--db-host-port" {
                let Some(value) = arguments.next() else {
                    usage();
                    return ExitCode::from(2);
                };
                setup_options.database_host_port = match parse_database_host_port(&value) {
                    Ok(port) => Some(port),
                    Err(error) => {
                        eprintln!("error[E-SETUP-002]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else if !argument.starts_with('-') && !path_given {
                path = argument;
                path_given = true;
            } else {
                usage();
                return ExitCode::from(2);
            }
        }
        if web {
            if setup_options.host_port.is_some() || setup_options.database_host_port.is_some() {
                eprintln!(
                    "error[E-SETUP-002]: --host-port and --db-host-port cannot be used with --web"
                );
                return ExitCode::from(2);
            }
            return setup_web_command(&path, web_port, web_port_given);
        }
        if web_port_given {
            eprintln!("error[E-SETUP-002]: --port requires --web");
            return ExitCode::from(2);
        }
        if action == "prepare" {
            return setup_project(&path, &setup_options);
        }
        let english = std::env::var("ZELYRA_LANGUAGE")
            .is_ok_and(|language| language.eq_ignore_ascii_case("en"));
        println!(
            "{}",
            match (english, action) {
                (true, "database") => "Starting the local MariaDB and web application...",
                (true, "schema") => "Starting the local stack and applying the project schema...",
                (true, _) => "Starting the local stack and applying the project schema...",
                (false, "database") => "Lokale MariaDB und Webanwendung werden gestartet...",
                (false, "schema") =>
                    "Lokaler Stack wird gestartet und das Projektschema wird eingerichtet...",
                (false, _) =>
                    "Lokaler Stack wird gestartet und das Projektschema wird eingerichtet...",
            }
        );
        return match setup_action(&path, action, &setup_options) {
            Ok(message) => {
                println!("{message}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error[E-SETUP-006]: {error}");
                ExitCode::from(1)
            }
        };
    }
    if command == "new" {
        let Some(path) = args.next() else {
            usage();
            return ExitCode::from(2);
        };
        let mut with_mariadb = false;
        let mut crud_template = false;
        let mut auth_template = false;
        let mut business_template = false;
        let mut web_port = DEFAULT_WEB_PORT;
        let mut web_port_given = false;
        let mut host_port = DEFAULT_WEB_PORT;
        let mut host_port_given = false;
        let mut database_host_port = DEFAULT_DATABASE_HOST_PORT;
        let mut database_host_port_given = false;
        let mut arguments = args;
        while let Some(argument) = arguments.next() {
            if argument == "--mariadb" && !with_mariadb {
                with_mariadb = true;
            } else if argument == "--template" {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --template requires a value");
                    return ExitCode::from(2);
                };
                match value.as_str() {
                    "minimal" => {
                        crud_template = false;
                        auth_template = false;
                        business_template = false;
                    }
                    "mariadb-crud" => {
                        with_mariadb = true;
                        crud_template = true;
                        auth_template = false;
                        business_template = false;
                    }
                    "mariadb-auth" => {
                        with_mariadb = true;
                        crud_template = false;
                        auth_template = true;
                        business_template = false;
                    }
                    "mariadb-business" => {
                        with_mariadb = true;
                        crud_template = false;
                        auth_template = false;
                        business_template = true;
                    }
                    _ => {
                        eprintln!(
                            "error[E-CLI-001]: unknown template `{value}`; expected `minimal`, `mariadb-crud`, `mariadb-auth`, or `mariadb-business`"
                        );
                        return ExitCode::from(2);
                    }
                }
            } else if argument == "--web-port" {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --web-port requires a value");
                    return ExitCode::from(2);
                };
                web_port_given = true;
                web_port = match parse_web_port(&value) {
                    Ok(port) => port,
                    Err(error) => {
                        eprintln!("error[E-CLI-001]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else if argument == "--host-port" {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --host-port requires a value");
                    return ExitCode::from(2);
                };
                host_port_given = true;
                host_port = match parse_web_port(&value) {
                    Ok(port) => port,
                    Err(error) => {
                        eprintln!("error[E-CLI-001]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else if argument == "--db-host-port" {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --db-host-port requires a value");
                    return ExitCode::from(2);
                };
                database_host_port_given = true;
                database_host_port = match parse_database_host_port(&value) {
                    Ok(port) => port,
                    Err(error) => {
                        eprintln!("error[E-CLI-001]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else {
                usage();
                return ExitCode::from(2);
            }
        }
        if (web_port_given || host_port_given || database_host_port_given) && !with_mariadb {
            eprintln!(
                "error[E-CLI-001]: --web-port, --host-port, and --db-host-port require --mariadb"
            );
            return ExitCode::from(2);
        }
        return create_project(
            &path,
            ProjectOptions {
                allow_current_directory: false,
                with_mariadb,
                crud_template,
                auth_template,
                business_template,
                web_port,
                host_port,
                database_host_port,
                host_port_given,
                database_host_port_given,
            },
        );
    }
    if command == "init" {
        let mut path = ".".to_owned();
        let mut path_given = false;
        let mut with_mariadb = false;
        let mut crud_template = false;
        let mut auth_template = false;
        let mut business_template = false;
        let mut web_port = DEFAULT_WEB_PORT;
        let mut web_port_given = false;
        let mut host_port = DEFAULT_WEB_PORT;
        let mut host_port_given = false;
        let mut database_host_port = DEFAULT_DATABASE_HOST_PORT;
        let mut database_host_port_given = false;
        let mut arguments = args;
        while let Some(argument) = arguments.next() {
            if argument == "--mariadb" && !with_mariadb {
                with_mariadb = true;
            } else if argument == "--template" {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --template requires a value");
                    return ExitCode::from(2);
                };
                match value.as_str() {
                    "minimal" => {
                        crud_template = false;
                        auth_template = false;
                        business_template = false;
                    }
                    "mariadb-crud" => {
                        with_mariadb = true;
                        crud_template = true;
                        auth_template = false;
                        business_template = false;
                    }
                    "mariadb-auth" => {
                        with_mariadb = true;
                        crud_template = false;
                        auth_template = true;
                        business_template = false;
                    }
                    "mariadb-business" => {
                        with_mariadb = true;
                        crud_template = false;
                        auth_template = false;
                        business_template = true;
                    }
                    _ => {
                        eprintln!(
                            "error[E-CLI-001]: unknown template `{value}`; expected `minimal`, `mariadb-crud`, `mariadb-auth`, or `mariadb-business`"
                        );
                        return ExitCode::from(2);
                    }
                }
            } else if argument == "--web-port" {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --web-port requires a value");
                    return ExitCode::from(2);
                };
                web_port_given = true;
                web_port = match parse_web_port(&value) {
                    Ok(port) => port,
                    Err(error) => {
                        eprintln!("error[E-CLI-001]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else if argument == "--host-port" {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --host-port requires a value");
                    return ExitCode::from(2);
                };
                host_port_given = true;
                host_port = match parse_web_port(&value) {
                    Ok(port) => port,
                    Err(error) => {
                        eprintln!("error[E-CLI-001]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else if argument == "--db-host-port" {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --db-host-port requires a value");
                    return ExitCode::from(2);
                };
                database_host_port_given = true;
                database_host_port = match parse_database_host_port(&value) {
                    Ok(port) => port,
                    Err(error) => {
                        eprintln!("error[E-CLI-001]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else if !argument.starts_with('-') && !path_given {
                path = argument;
                path_given = true;
            } else {
                usage();
                return ExitCode::from(2);
            }
        }
        if (web_port_given || host_port_given || database_host_port_given) && !with_mariadb {
            eprintln!(
                "error[E-CLI-001]: --web-port, --host-port, and --db-host-port require --mariadb"
            );
            return ExitCode::from(2);
        }
        return create_project(
            &path,
            ProjectOptions {
                allow_current_directory: true,
                with_mariadb,
                crud_template,
                auth_template,
                business_template,
                web_port,
                host_port,
                database_host_port,
                host_port_given,
                database_host_port_given,
            },
        );
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
    if command == "check" {
        return check_command(args);
    }
    if command == "routes" {
        return route_cli::command(args);
    }
    if command == "reflect" {
        return reflect::command(args);
    }
    if command == "fmt" {
        return fmt_command(args);
    }
    if command == "impact" {
        return impact_command(args);
    }
    if command == "edit" {
        return edit_command(args);
    }
    if command == "text" {
        return text_edit::command(args);
    }
    if command == "context" {
        return context_command(args);
    }
    if command == "module" {
        return module_command(args);
    }
    if command == "package" {
        return packages::command(args);
    }
    if command == "config" {
        return config_command(args);
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
    let mut step_budget = None;
    let mut entry_function = None;
    let mut entry_arguments_json = None;
    let mut run_backend = "ast";
    let mut backend_given = false;
    let mut optimize_mir = false;
    if command == "run" {
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--max-steps" if step_budget.is_none() => {
                    let Some(value) = args.next() else {
                        eprintln!("error[E-CLI-001]: --max-steps requires a positive integer");
                        return ExitCode::from(2);
                    };
                    step_budget = match value.parse::<usize>() {
                        Ok(value) if value > 0 => Some(value),
                        _ => {
                            eprintln!("error[E-CLI-001]: --max-steps requires a positive integer");
                            return ExitCode::from(2);
                        }
                    };
                }
                "--entry" if entry_function.is_none() => {
                    let Some(value) = args.next() else {
                        eprintln!("error[E-CLI-001]: --entry requires a function name");
                        return ExitCode::from(2);
                    };
                    if value.is_empty() || value.starts_with('-') {
                        eprintln!("error[E-CLI-001]: --entry requires a function name");
                        return ExitCode::from(2);
                    }
                    entry_function = Some(value);
                }
                "--entry-args" if entry_arguments_json.is_none() => {
                    let Some(value) = args.next() else {
                        eprintln!("error[E-CLI-001]: --entry-args requires a JSON array");
                        return ExitCode::from(2);
                    };
                    entry_arguments_json = Some(value);
                }
                "--backend" if !backend_given => {
                    let Some(value) = args.next() else {
                        eprintln!(
                            "error[E-CLI-001]: --backend requires `ast`, `mir`, or `bytecode`"
                        );
                        return ExitCode::from(2);
                    };
                    if value != "ast" && value != "mir" && value != "bytecode" {
                        eprintln!(
                            "error[E-CLI-001]: --backend requires `ast`, `mir`, or `bytecode`"
                        );
                        return ExitCode::from(2);
                    }
                    backend_given = true;
                    run_backend = match value.as_str() {
                        "mir" => "mir",
                        "bytecode" => "bytecode",
                        _ => "ast",
                    };
                }
                "--optimize-mir" if !optimize_mir => optimize_mir = true,
                _ => {
                    usage();
                    return ExitCode::from(2);
                }
            }
        }
        if entry_arguments_json.is_some() && entry_function.is_none() {
            eprintln!("error[E-CLI-001]: --entry-args requires --entry");
            return ExitCode::from(2);
        }
        if optimize_mir && run_backend != "mir" {
            eprintln!("error[E-CLI-001]: --optimize-mir requires --backend mir");
            return ExitCode::from(2);
        }
    } else if args.next().is_some() {
        usage();
        return ExitCode::from(2);
    }
    match command.as_str() {
        "check" | "build" => {
            if validate(&path).is_ok() {
                println!("{}", terminal::success(&format!("ok: {path}")));
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        "run" => match validate(&path).and_then(|program| {
            validate_execution_settings(&path)?;
            let entry_arguments = if let Some(entry) = entry_function.as_deref() {
                match program.functions.iter().find(|function| function.name == entry) {
                    Some(function) if !function.type_params.is_empty() => {
                        diagnostic(
                            &path,
                            "E-CLI-001",
                            &format!("entry function `{entry}` must not have generic type parameters"),
                            1,
                            1,
                        );
                        return Err(());
                    }
                    Some(function) => {
                        match parse_entry_arguments(
                            entry,
                            &function.params,
                            entry_arguments_json.as_deref(),
                            &program,
                        ) {
                            Ok(arguments) => arguments,
                            Err(error) => {
                                diagnostic(&path, "E-CLI-001", &error, 1, 1);
                                return Err(());
                            }
                        }
                    }
                    None => {
                        diagnostic(
                            &path,
                            "E-CLI-001",
                            &format!("entry function `{entry}` was not found"),
                            1,
                            1,
                        );
                        return Err(());
                    }
                }
            } else {
                Vec::new()
            };
            let grants = match project_capability_grants(&path) {
                Ok(grants) => grants,
                Err(error) => {
                    diagnostic(&path, "E-CAP-002", &error, 1, 1);
                    return Err(());
                }
            };
            let mut runtime_policy = match project_runtime_policy(&path) {
                Ok(policy) => policy,
                Err(error) => {
                    diagnostic(&path, "E-POLICY-002", &error, 1, 1);
                    return Err(());
                }
            };
            if let Some(step_budget) = step_budget {
                runtime_policy
                    .get_or_insert_with(RuntimePolicy::default)
                    .step_budget = Some(step_budget);
            }
            let database_url = database_url_from_program(&program);
            let function_name = entry_function.as_deref().unwrap_or("main");
            let (output, error, return_value) = match run_backend {
                "ast" => {
                    let outcome = execute_function_captured_with_capabilities_and_policies(
                        &program,
                        function_name,
                        entry_arguments.clone(),
                        database_url.as_deref(),
                        grants.as_ref(),
                        runtime_policy.as_ref(),
                    );
                    (outcome.output, outcome.error, outcome.value)
                }
                "mir" => {
                    let mut mir = match zelyra_mir::lower(&program) {
                        Ok(mir) => mir,
                        Err(errors) => {
                            for error in errors {
                                diagnostic(
                                    &path,
                                    error.code,
                                    &error.message,
                                    error.span.line,
                                    error.span.column,
                                );
                            }
                            return Err(());
                        }
                    };
                    if optimize_mir {
                        if let Err(errors) = zelyra_mir::optimize_scalar_constants(&mut mir) {
                            for error in errors {
                                diagnostic(
                                    &path,
                                    error.code,
                                    &error.message,
                                    error.span.line,
                                    error.span.column,
                                );
                            }
                            return Err(());
                        }
                    }
                    let limits = zelyra_mir::execution::ExecutionLimits {
                        max_steps: runtime_policy
                            .as_ref()
                            .and_then(|policy| policy.step_budget)
                            .unwrap_or(100_000),
                        max_stack_depth: runtime_policy
                            .as_ref()
                            .and_then(|policy| policy.max_stack_depth)
                            .unwrap_or(20),
                    };
                    let execution = match database_url.as_deref() {
                        Some(database_url) => {
                            zelyra_mir::execution::execute_function_captured_with_database_and_host_services_and_policy(
                                &mir,
                                function_name,
                                entry_arguments.clone(),
                                limits,
                                grants.as_ref(),
                                runtime_policy.as_ref(),
                                zelyra_mir::execution::DatabaseExecution {
                                    url: database_url,
                                    host_services: Arc::new(SystemHostServices),
                                },
                            )
                        }
                        None => zelyra_mir::execution::execute_function_captured_with_host_services_and_policy(
                            &mir,
                            function_name,
                            entry_arguments.clone(),
                            limits,
                            grants.as_ref(),
                            runtime_policy.as_ref(),
                            Arc::new(SystemHostServices),
                        ),
                    };
                    (execution.output, execution.error, execution.value)
                }
                "bytecode" => {
                    let mir = match zelyra_mir::lower(&program) {
                        Ok(mir) => mir,
                        Err(errors) => {
                            for error in errors {
                                diagnostic(
                                    &path,
                                    error.code,
                                    &error.message,
                                    error.span.line,
                                    error.span.column,
                                );
                            }
                            return Err(());
                        }
                    };
                    let bytecode = match zelyra_mir::bytecode::compile(&mir, function_name) {
                        Ok(bytecode) => bytecode,
                        Err(errors) => {
                            for error in errors {
                                diagnostic(
                                    &path,
                                    error.code,
                                    &error.message,
                                    error.span.line,
                                    error.span.column,
                                );
                            }
                            return Err(());
                        }
                    };
                    let limits = zelyra_mir::execution::ExecutionLimits {
                        max_steps: runtime_policy
                            .as_ref()
                            .and_then(|policy| policy.step_budget)
                            .unwrap_or(100_000),
                        max_stack_depth: runtime_policy
                            .as_ref()
                            .and_then(|policy| policy.max_stack_depth)
                            .unwrap_or(20),
                    };
                    let host = Arc::new(SystemHostServices);
                    let execution = match database_url.as_deref() {
                        Some(database_url) => zelyra_mir::bytecode::execute_with_database_and_host_services_and_policy_captured(
                            &bytecode,
                            entry_arguments.clone(),
                            limits,
                            grants.as_ref(),
                            runtime_policy.as_ref(),
                            zelyra_mir::bytecode::BytecodeDatabaseExecution {
                                url: database_url,
                                schema: &mir.database_schema,
                            },
                            host,
                        ),
                        None => zelyra_mir::bytecode::execute_with_host_services_and_policy_captured(
                            &bytecode,
                            entry_arguments.clone(),
                            limits,
                            grants.as_ref(),
                            runtime_policy.as_ref(),
                            host,
                        ),
                    };
                    (execution.output, execution.error, execution.value)
                }
                _ => unreachable!("run backend is validated while parsing options"),
            };
            for line in output {
                println!("{line}");
            }
            if let Some(error) = error {
                diagnostic(
                    &path,
                    "E-RUNTIME-001",
                    &error.message,
                    error.span.line,
                    error.span.column,
                );
                Err(())
            } else {
                match return_value {
                    None | Some(RuntimeValue::Unit) | Some(RuntimeValue::Result(Ok(_))) => Ok(0),
                    Some(RuntimeValue::Int(code)) if (0..=255).contains(&code) => {
                        Ok(code as u8)
                    }
                    Some(RuntimeValue::UInt(code)) if code <= 255 => Ok(code as u8),
                    Some(RuntimeValue::Result(Err(error))) => {
                        diagnostic(
                            &path,
                            "E-RUNTIME-001",
                            &format!("entry function returned Err: {}", error.output()),
                            1,
                            1,
                        );
                        Err(())
                    }
                    Some(value) => {
                        diagnostic(
                            &path,
                            "E-CLI-001",
                            &format!(
                                "entry function must return Unit, Result, or an exit code from 0 through 255; found {}",
                                value.ty()
                            ),
                            1,
                            1,
                        );
                        Err(())
                    }
                }
            }
        }) {
            Ok(code) => ExitCode::from(code),
            Err(()) => ExitCode::from(1),
        },
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}
