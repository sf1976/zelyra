use super::*;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
enum TestOutputFormat {
    Human,
    Json,
}

struct TestRunEntry {
    name: String,
    file: String,
    line: usize,
    column: usize,
    passed: bool,
    duration_ms: u128,
    rollback_sandbox: bool,
    output: Vec<String>,
    error: Option<String>,
}

pub(super) fn command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let mut target_path: Option<String> = None;
    let mut filter: Option<String> = None;
    let mut format = TestOutputFormat::Human;

    while let Some(argument) = args.next() {
        if argument == "--filter" {
            let Some(value) = args.next() else {
                eprintln!("error[E-TEST-CLI-001]: `--filter` expects a substring argument");
                return ExitCode::from(2);
            };
            filter = Some(value);
        } else if let Some(value) = argument.strip_prefix("--filter=") {
            filter = Some(value.to_owned());
        } else if argument == "--format" {
            let Some(value) = args.next() else {
                eprintln!("error[E-TEST-CLI-001]: `--format` expects `human` or `json`");
                return ExitCode::from(2);
            };
            match parse_format(&value) {
                Ok(parsed) => format = parsed,
                Err(error) => {
                    eprintln!("error[E-TEST-CLI-001]: {error}");
                    return ExitCode::from(2);
                }
            }
        } else if let Some(value) = argument.strip_prefix("--format=") {
            match parse_format(value) {
                Ok(parsed) => format = parsed,
                Err(error) => {
                    eprintln!("error[E-TEST-CLI-001]: {error}");
                    return ExitCode::from(2);
                }
            }
        } else if argument == "--json" {
            format = TestOutputFormat::Json;
        } else if !argument.starts_with('-') && target_path.is_none() {
            target_path = Some(argument);
        } else {
            eprintln!("error[E-TEST-CLI-001]: unexpected argument `{argument}` for `zelyra test`");
            return ExitCode::from(2);
        }
    }

    let path = PathBuf::from(target_path.as_deref().unwrap_or("."));
    let targets = match discover_test_targets(&path) {
        Ok(targets) => targets,
        Err(error) => {
            report_setup_error(format, &error);
            return ExitCode::from(1);
        }
    };

    let suite_start = Instant::now();
    let mut results = Vec::new();
    let mut filtered_out = 0usize;

    for target in &targets {
        let target_str = target.to_string_lossy().into_owned();
        let loaded = match load_project(&target_str) {
            Ok(loaded) => loaded,
            Err(()) => {
                report_setup_error(format, &format!("failed to load `{}`", target.display()));
                return ExitCode::from(1);
            }
        };
        let grants = match project_capability_grants(&target_str) {
            Ok(grants) => grants,
            Err(error) => {
                report_setup_error(format, &error);
                return ExitCode::from(1);
            }
        };
        let policy = match project_runtime_policy(&target_str) {
            Ok(policy) => policy,
            Err(error) => {
                report_setup_error(format, &error);
                return ExitCode::from(1);
            }
        };
        if let Err(error) = validate_loaded_test_project(&loaded, grants.as_ref()) {
            report_setup_error(format, &error);
            return ExitCode::from(1);
        }

        let (test_functions, mappings) = zelyra_ast::build_test_functions(&loaded.program);
        let mut runnable_program = loaded.program.clone();
        runnable_program.functions.extend(test_functions);

        let project_directory = target
            .parent()
            .filter(|directory| !directory.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let database_name = loaded
            .program
            .databases
            .first()
            .map(|database| database.name.as_str());
        let database_url = match database_url_for_project(project_directory, database_name) {
            Ok(url) => url,
            Err(error) => {
                report_setup_error(format, &error);
                return ExitCode::from(1);
            }
        };

        for mapping in mappings {
            if let Some(needle) = &filter {
                if !mapping.test_name.contains(needle.as_str()) {
                    filtered_out += 1;
                    continue;
                }
            }
            let test_def = &loaded.program.tests[mapping.test_index];
            let source_file = loaded
                .sources
                .get(test_def.span.source_id as usize)
                .map(|source| source.path.clone())
                .unwrap_or_else(|| target_str.clone());
            let uses_database = test_def
                .capabilities
                .iter()
                .any(|cap| cap == "Database" || cap.starts_with("Database("));

            let test_start = Instant::now();
            let (outcome, rollback_sandbox) = if uses_database {
                if let Some(url) = database_url
                    .as_deref()
                    .filter(|url| !url.trim().is_empty() && !is_sqlite_url(url))
                {
                    match zelyra_database::with_rollback_transaction(url, |tx| {
                        let mut bridge = |query: &zelyra_database::Query| {
                            tx.execute(query).map_err(|error| error.message)
                        };
                        Ok(
                            zelyra_runtime::execute_function_captured_with_transactional_query(
                                &runnable_program,
                                &mapping.function_name,
                                Vec::new(),
                                url,
                                grants.as_ref(),
                                policy.as_ref(),
                                &mut bridge,
                            ),
                        )
                    }) {
                        Ok(outcome) => (outcome, true),
                        Err(error) => (
                            zelyra_runtime::FunctionExecutionOutcome {
                                output: Vec::new(),
                                value: None,
                                error: Some(zelyra_runtime::RuntimeError {
                                    message: error.message,
                                    span: test_def.span,
                                }),
                            },
                            true,
                        ),
                    }
                } else {
                    (
                        execute_function_captured_with_capabilities_and_policies(
                            &runnable_program,
                            &mapping.function_name,
                            Vec::new(),
                            database_url.as_deref(),
                            grants.as_ref(),
                            policy.as_ref(),
                        ),
                        false,
                    )
                }
            } else {
                (
                    execute_function_captured_with_capabilities_and_policies(
                        &runnable_program,
                        &mapping.function_name,
                        Vec::new(),
                        database_url.as_deref(),
                        grants.as_ref(),
                        policy.as_ref(),
                    ),
                    false,
                )
            };

            let duration_ms = test_start.elapsed().as_millis();
            let (passed, error_message) = match outcome.error {
                None => (true, None),
                Some(error) => {
                    let error_file = loaded
                        .sources
                        .get(error.span.source_id as usize)
                        .map(|source| source.path.as_str())
                        .unwrap_or(&source_file);
                    let formatted = if error.span.line > 0 {
                        format!(
                            "{error_file}:{}:{}: {}",
                            error.span.line, error.span.column, error.message
                        )
                    } else {
                        error.message
                    };
                    (false, Some(formatted))
                }
            };

            results.push(TestRunEntry {
                name: mapping.test_name,
                file: source_file,
                line: test_def.span.line,
                column: test_def.span.column,
                passed,
                duration_ms,
                rollback_sandbox,
                output: outcome.output,
                error: error_message,
            });
        }
    }

    let total_duration_ms = suite_start.elapsed().as_millis();
    let passed = results.iter().filter(|entry| entry.passed).count();
    let failed = results.len() - passed;

    match format {
        TestOutputFormat::Human => {
            println!("running {} test(s)", results.len());
            for entry in &results {
                let sandbox = if entry.rollback_sandbox {
                    ", rollback sandbox"
                } else {
                    ""
                };
                if entry.passed {
                    println!("  PASS {} ({}ms{sandbox})", entry.name, entry.duration_ms);
                } else {
                    println!("  FAIL {} ({}ms{sandbox})", entry.name, entry.duration_ms);
                    if let Some(error) = &entry.error {
                        println!("       {error}");
                    }
                }
            }
            let status = if failed == 0 { "ok" } else { "FAILED" };
            println!(
                "\ntest result: {status}. {passed} passed; {failed} failed; {filtered_out} filtered out; finished in {total_duration_ms}ms"
            );
        }
        TestOutputFormat::Json => {
            let payload = json!({
                "schema_version": "1",
                "status": if failed == 0 { "passed" } else { "failed" },
                "summary": {
                    "total": results.len(),
                    "passed": passed,
                    "failed": failed,
                    "filtered_out": filtered_out,
                    "duration_ms": total_duration_ms,
                },
                "tests": results.iter().map(|entry| json!({
                    "name": entry.name,
                    "file": entry.file,
                    "line": entry.line,
                    "column": entry.column,
                    "status": if entry.passed { "passed" } else { "failed" },
                    "duration_ms": entry.duration_ms,
                    "rollback_sandbox": entry.rollback_sandbox,
                    "output": entry.output,
                    "error": entry.error,
                })).collect::<Vec<_>>(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&payload).expect("test JSON serializes")
            );
        }
    }

    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn parse_format(value: &str) -> Result<TestOutputFormat, String> {
    match value {
        "human" => Ok(TestOutputFormat::Human),
        "json" => Ok(TestOutputFormat::Json),
        other => Err(format!(
            "unknown `--format` value `{other}`; expected `human` or `json`"
        )),
    }
}

fn is_sqlite_url(url: &str) -> bool {
    url.starts_with("sqlite:") || url.ends_with(".sqlite") || url.ends_with(".db")
}

fn discover_test_targets(path: &Path) -> Result<Vec<PathBuf>, String> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    if !path.is_dir() {
        return Err(format!("test path `{}` does not exist", path.display()));
    }
    let mut targets = Vec::new();
    let tests_dir = path.join("tests");
    if tests_dir.is_dir() {
        collect_zyl_files(&tests_dir, &mut targets)?;
    }
    let tests_file = path.join("tests.zyl");
    if tests_file.is_file() {
        targets.push(tests_file);
    }
    if targets.is_empty() {
        let main_file = path.join("main.zyl");
        if main_file.is_file() {
            targets.push(main_file);
        } else {
            return Err(format!(
                "no Zelyra test files found in `{}` (expected `tests/*.zyl`, `tests.zyl`, or `main.zyl`)",
                path.display()
            ));
        }
    }
    targets.sort();
    Ok(targets)
}

fn collect_zyl_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|error| format!("cannot read `{}`: {error}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot read directory entry: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_zyl_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "zyl") {
            out.push(path);
        }
    }
    Ok(())
}

fn validate_loaded_test_project(
    loaded: &project::LoadedProject,
    grants: Option<&HashSet<String>>,
) -> Result<(), String> {
    if let Err(errors) = check(&loaded.program) {
        let details = errors
            .iter()
            .map(|error| format!("{} [{}]", error.message, error.code))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("type check failed: {details}"));
    }
    let schema = build_schema(&loaded.program).map_err(|errors| {
        let details = errors
            .iter()
            .map(|error| error.message.clone())
            .collect::<Vec<_>>()
            .join("; ");
        format!("schema check failed: {details}")
    })?;
    if let Err(errors) = check_sql_program(&loaded.program, &schema) {
        let details = errors
            .iter()
            .map(|error| error.message.clone())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("SQL check failed: {details}"));
    }
    if let Err(errors) = check_form_program(&loaded.program, &schema) {
        let details = errors
            .iter()
            .map(|error| error.message.clone())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("form check failed: {details}"));
    }
    if let Err(errors) = check_apis(&loaded.program) {
        let details = errors
            .iter()
            .map(|error| error.message.clone())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("API check failed: {details}"));
    }
    if let Err(errors) = check_capabilities_with_grants(&loaded.program, grants) {
        let details = errors
            .iter()
            .map(|error| error.message.clone())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("capability check failed: {details}"));
    }
    Ok(())
}

fn report_setup_error(format: TestOutputFormat, error: &str) {
    match format {
        TestOutputFormat::Human => {
            eprintln!("error[E-TEST-001]: {error}");
        }
        TestOutputFormat::Json => {
            let payload = json!({
                "schema_version": "1",
                "status": "error",
                "error": error,
                "summary": {
                    "total": 0,
                    "passed": 0,
                    "failed": 0,
                    "filtered_out": 0,
                    "duration_ms": 0,
                },
                "tests": [],
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&payload).expect("test error JSON serializes")
            );
        }
    }
}
