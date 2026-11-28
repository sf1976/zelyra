use super::*;
use zelyra_runtime::{explain_decision, privacy::program_uses_privacy, DecisionTrace};

pub(super) fn command(arguments: &[String]) -> ExitCode {
    let (path, name, raw) = match arguments {
        [path] => (path, "main", None),
        [path, flag, name] if flag == "--entry" => (path, name.as_str(), None),
        [path, flag, name, args, raw] if flag == "--entry" && args == "--entry-args" => {
            (path, name.as_str(), Some(raw.as_str()))
        }
        _ => {
            usage();
            return ExitCode::from(2);
        }
    };
    match trace(path, name, raw) {
        Ok(DecisionTrace::Evaluated { completed, events }) => {
            let events = events
                .iter()
                .map(|event| {
                    json!({
                        "rule": event.rule.as_str(), "accepted": event.accepted,
                        "source_id": event.span.source_id,
                        "line": event.span.line, "column": event.span.column,
                    })
                })
                .collect::<Vec<_>>();
            print_machine_document(&json!({
                "schema_version": 1, "status": if completed { "completed" } else { "aborted" },
                "events": events,
            }));
            if completed {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Ok(DecisionTrace::Suppressed) => {
            print_machine_document(
                &json!({"schema_version": 1, "status": "suppressed", "events": []}),
            );
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("error[E-EXPLAIN-001]: {error}");
            ExitCode::FAILURE
        }
    }
}

fn trace(path: &str, name: &str, raw: Option<&str>) -> Result<DecisionTrace, &'static str> {
    // Loading failures can include source literals in diagnostics. Keep both
    // failure diagnostics and all outcomes of classified projects value-free.
    let loaded = project::load(path).map_err(|_| "could not load the project")?;
    if program_uses_privacy(&loaded.program) {
        return Ok(DecisionTrace::Suppressed);
    }
    let loaded = load_project_with(Ok(loaded)).map_err(|()| "could not load the project")?;
    let program =
        validate_loaded_project(path, loaded).map_err(|()| "project validation failed")?;
    let function = program
        .functions
        .iter()
        .find(|function| function.name == name)
        .filter(|function| function.type_params.is_empty())
        .ok_or("entry must name a non-generic function")?;
    let arguments = command_dispatch::parse_entry_arguments(name, &function.params, raw, &program)
        .map_err(|_| "public arguments do not match the entry signature")?;
    Ok(explain_decision(&program, name, arguments))
}
