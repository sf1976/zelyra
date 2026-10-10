use super::*;

const DEFAULT_OUTBOX_LIMIT: usize = 50;
const DEFAULT_OUTBOX_LEASE_SECONDS: u64 = 300;

pub(super) fn outbox_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(action) = args.next() else {
        outbox_usage();
        return ExitCode::from(2);
    };

    let mut event_id = None;
    if action == "requeue" {
        event_id = args.next();
        if event_id.is_none() {
            outbox_usage();
            return ExitCode::from(2);
        }
    }
    let mut path = "main.zyl".to_owned();
    let mut path_given = false;
    let mut limit = DEFAULT_OUTBOX_LIMIT;
    let mut format = "text";
    let mut handler_specs = Vec::new();
    let mut once = false;
    let mut lease_seconds = DEFAULT_OUTBOX_LEASE_SECONDS;
    while let Some(argument) = args.next() {
        if argument == "--limit" && action == "list" {
            let Some(value) = args.next() else {
                outbox_usage();
                return ExitCode::from(2);
            };
            limit = match value.parse::<usize>() {
                Ok(value)
                    if (1..=zelyra_database::outbox::MAX_OUTBOX_INSPECTION_LIMIT)
                        .contains(&value) =>
                {
                    value
                }
                _ => {
                    eprintln!(
                        "error[E-OUTBOX-001]: --limit must be between 1 and {}",
                        zelyra_database::outbox::MAX_OUTBOX_INSPECTION_LIMIT
                    );
                    return ExitCode::from(2);
                }
            };
        } else if argument == "--format" && action == "list" {
            let Some(value) = args.next() else {
                outbox_usage();
                return ExitCode::from(2);
            };
            if value != "text" && value != "json" {
                eprintln!("error[E-OUTBOX-001]: --format must be `text` or `json`");
                return ExitCode::from(2);
            }
            format = if value == "json" { "json" } else { "text" };
        } else if argument == "--handler" && action == "run" {
            let Some(value) = args.next() else {
                outbox_usage();
                return ExitCode::from(2);
            };
            let Some((event_type, function_name)) = value.split_once('=') else {
                eprintln!("error[E-OUTBOX-001]: --handler must use event.type=function_name");
                return ExitCode::from(2);
            };
            if event_type.is_empty() || function_name.is_empty() || function_name.contains('=') {
                eprintln!("error[E-OUTBOX-001]: --handler must use event.type=function_name");
                return ExitCode::from(2);
            }
            handler_specs.push((event_type.to_owned(), function_name.to_owned()));
        } else if argument == "--once" && action == "run" {
            once = true;
        } else if argument == "--lease-seconds" && action == "run" {
            let Some(value) = args.next() else {
                outbox_usage();
                return ExitCode::from(2);
            };
            lease_seconds = match value.parse::<u64>() {
                Ok(seconds) if (5..=3600).contains(&seconds) => seconds,
                _ => {
                    eprintln!("error[E-OUTBOX-001]: --lease-seconds must be between 5 and 3600");
                    return ExitCode::from(2);
                }
            };
        } else if !argument.starts_with('-') && !path_given {
            path = argument;
            path_given = true;
        } else {
            outbox_usage();
            return ExitCode::from(2);
        }
    }

    if !matches!(action.as_str(), "setup" | "list" | "requeue" | "run") {
        eprintln!("error[E-OUTBOX-001]: unknown outbox action `{action}`");
        outbox_usage();
        return ExitCode::from(2);
    }
    if action == "run" && handler_specs.is_empty() {
        eprintln!("error[E-OUTBOX-001]: outbox run requires at least one --handler mapping");
        outbox_usage();
        return ExitCode::from(2);
    }
    let schema = match database_cli::load_schema(&path) {
        Ok(schema) => schema,
        Err(()) => return ExitCode::from(1),
    };
    if schema.backend() != Backend::MariaDb {
        eprintln!("error[E-OUTBOX-002]: the local outbox currently supports MariaDB projects only");
        return ExitCode::from(1);
    }
    let Some(database_url) = database_url_from_schema(&schema) else {
        eprintln!("error[E-OUTBOX-003]: a MariaDB database URL is required for outbox {action}");
        eprintln!(
            "hint: set the project database URL without committing credentials to source control"
        );
        return ExitCode::from(1);
    };

    match action.as_str() {
        "setup" => match zelyra_database::outbox::ensure_mariadb_outbox(&database_url) {
            Ok(()) => {
                println!("MariaDB outbox is ready");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error[E-OUTBOX-004]: {error}");
                ExitCode::from(1)
            }
        },
        "list" => match zelyra_database::outbox::inspect_mariadb_outbox(&database_url, limit) {
            Ok(events) => {
                print_events(&events, format == "json");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error[E-OUTBOX-005]: {error}");
                ExitCode::from(1)
            }
        },
        "run" => run_outbox_worker(&path, &database_url, &handler_specs, once, lease_seconds),
        "requeue" => match zelyra_database::outbox::requeue_exhausted_mariadb_event(
            &database_url,
            event_id.as_deref().expect("requeue event id was parsed"),
        ) {
            Ok(true) => {
                println!("outbox event was released for another delivery attempt");
                ExitCode::SUCCESS
            }
            Ok(false) => {
                eprintln!("error[E-OUTBOX-006]: event was not found or is not exhausted");
                ExitCode::from(1)
            }
            Err(error) => {
                eprintln!("error[E-OUTBOX-007]: {error}");
                ExitCode::from(1)
            }
        },
        _ => unreachable!("action was validated above"),
    }
}

pub(super) fn run_outbox_worker(
    path: &str,
    database_url: &str,
    handler_specs: &[(String, String)],
    once: bool,
    lease_seconds: u64,
) -> ExitCode {
    let program = match validate(path) {
        Ok(program) => Arc::new(program),
        Err(()) => return ExitCode::from(1),
    };
    let grants = match project_capability_grants(path) {
        Ok(grants) => grants,
        Err(error) => {
            eprintln!("error[E-OUTBOX-008]: {error}");
            return ExitCode::from(1);
        }
    };
    let policy = match project_runtime_policy(path) {
        Ok(policy) => policy,
        Err(error) => {
            eprintln!("error[E-OUTBOX-008]: {error}");
            return ExitCode::from(1);
        }
    };
    let mut handlers = zelyra_database::outbox::OutboxHandlerRegistry::default();
    for (event_type, function_name) in handler_specs {
        let Some(function) = program
            .functions
            .iter()
            .find(|function| function.name == *function_name && function.is_public)
        else {
            eprintln!(
                "error[E-OUTBOX-009]: `{function_name}` is not a public function in `{path}`"
            );
            return ExitCode::from(1);
        };
        if function.params.len() != 3
            || function
                .params
                .iter()
                .any(|parameter| parameter.ty != Type::String)
            || function.return_type.as_ref() != Some(&Type::Bool)
        {
            eprintln!("error[E-OUTBOX-009]: handler `{function_name}` must be `pub fn name(event_id: String, event_type: String, payload_json: String) -> Bool`");
            return ExitCode::from(1);
        }
        let program = Arc::clone(&program);
        let function_name = function_name.clone();
        let database_url = database_url.to_owned();
        let grants = grants.clone();
        let policy = policy.clone();
        if let Err(error) = handlers.register(event_type.clone(), move |event, _context| {
            let result = execute_function_with_capabilities_and_policies(
                &program,
                &function_name,
                vec![
                    RuntimeValue::String(event.id.clone()),
                    RuntimeValue::String(event.event_type.clone()),
                    RuntimeValue::String(event.payload_json.clone()),
                ],
                Some(&database_url),
                grants.as_ref(),
                policy.as_ref(),
            );
            if matches!(result, Ok(RuntimeValue::Bool(true))) {
                Ok(())
            } else {
                Err(zelyra_database::outbox::OutboxHandlerFailure)
            }
        }) {
            eprintln!("error[E-OUTBOX-010]: {error}");
            return ExitCode::from(2);
        }
    }

    let mut delivered = 0usize;
    let mut retried = 0usize;
    let mut exhausted = 0usize;
    let mut lease_lost = 0usize;
    loop {
        let mut random = [0u8; 32];
        OsRng.fill_bytes(&mut random);
        let lease_token = format!(
            "outbox-worker-{}",
            random
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        let result = match zelyra_database::outbox::run_mariadb_outbox_once(
            database_url,
            &handlers,
            &lease_token,
            lease_seconds,
        ) {
            Ok(result) => result,
            Err(error) => {
                eprintln!("error[E-OUTBOX-011]: {error}");
                return ExitCode::from(1);
            }
        };
        match result {
            zelyra_database::outbox::OutboxWorkerResult::Idle => break,
            zelyra_database::outbox::OutboxWorkerResult::Delivered { .. } => delivered += 1,
            zelyra_database::outbox::OutboxWorkerResult::RetryScheduled { .. } => retried += 1,
            zelyra_database::outbox::OutboxWorkerResult::Exhausted { .. } => exhausted += 1,
            zelyra_database::outbox::OutboxWorkerResult::LeaseLost { .. } => lease_lost += 1,
        }
        if once {
            break;
        }
    }
    println!(
        "outbox worker stopped: {delivered} delivered, {retried} retry scheduled, {exhausted} exhausted, {lease_lost} lease lost"
    );
    ExitCode::SUCCESS
}

fn print_events(events: &[zelyra_database::outbox::OutboxEventSummary], json_format: bool) {
    if json_format {
        let events = events
            .iter()
            .map(|event| {
                json!({
                    "id": event.id,
                    "event_type": event.event_type,
                    "attempts": event.attempts,
                    "status": event.status,
                    "created_at": event.created_at,
                    "available_at": event.available_at,
                    "lease_until": event.lease_until,
                    "last_error": event.last_error,
                })
            })
            .collect::<Vec<_>>();
        println!("{}", json!({"events": events}));
        return;
    }
    println!("id\ttype\tstatus\tattempts\tcreated_at\tavailable_at\tlease_until\terror");
    for event in events {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            event.id,
            event.event_type,
            event.status,
            event.attempts,
            event.created_at,
            event.available_at,
            event.lease_until.as_deref().unwrap_or("-"),
            event.last_error.as_deref().unwrap_or("-")
        );
    }
}

fn outbox_usage() {
    eprintln!(
        "Usage:\n  zelyra outbox setup [file.zyl]\n  zelyra outbox list [file.zyl] [--limit <1..200>] [--format text|json]\n  zelyra outbox requeue <event-id> [file.zyl]\n  zelyra outbox run [file.zyl] --handler <event.type=function_name> [--handler ...] [--once] [--lease-seconds <5..3600>]"
    );
}
