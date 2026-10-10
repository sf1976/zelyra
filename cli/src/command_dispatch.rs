use super::*;

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
    if command == "editor" {
        return editor::command(args);
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
    if command == "fmt" {
        return fmt_command(args);
    }
    if command == "impact" {
        return impact_command(args);
    }
    if command == "edit" {
        return edit_command(args);
    }
    if command == "context" {
        return context_command(args);
    }
    if command == "module" {
        return module_command(args);
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
            let result = match database_url_from_program(&program) {
                Some(database_url) => execute_with_database_and_capabilities_and_policies(
                    &program,
                    &database_url,
                    grants.as_ref(),
                    runtime_policy.as_ref(),
                ),
                None => execute_with_capabilities_and_policies(
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
