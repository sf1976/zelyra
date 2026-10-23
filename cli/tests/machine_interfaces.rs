use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::Duration,
};

fn binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_zelyra"))
}

fn run(arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .output()
        .expect("zelyra binary should run")
}

#[test]
fn reflect_exports_checked_project_metadata_deterministically_without_source_literals() {
    let source = r#"
table customers {
    id: Id primary auto
    name: String required
}

struct Customer {
    id: Id
    name: String
}

form CustomerCreate -> customers {
    fields { name }
}
page "/customers" { html { <p>Private display copy</p> } }
fn main() {}
"#;
    let (directory, path) = temporary_project_source("reflect", source);
    let run_reflect = || {
        Command::new(binary())
            .args(["reflect", path.to_str().unwrap(), "--format=json"])
            .output()
            .expect("reflect command should run")
    };
    let first = run_reflect();
    let second = run_reflect();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(first.stdout, second.stdout);
    let json: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(json["reflect_schema_version"], 1);
    assert_eq!(json["checked"], true);
    assert_eq!(json["tables"][0]["name"], "customers");
    assert!(json["routes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|route| route["path"] == "/customers"));
    assert!(!String::from_utf8_lossy(&first.stdout).contains("Private display copy"));
    assert!(!String::from_utf8_lossy(&first.stdout).contains(directory.to_str().unwrap()));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn reflect_rejects_unchecked_projects_without_emitting_a_success_document() {
    let (directory, path) = temporary_project_source("reflect-invalid", "fn main( {");
    let output = Command::new(binary())
        .args(["reflect", path.to_str().unwrap(), "--format=json"])
        .output()
        .expect("reflect command should run");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E-PARSE-001"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn nested_cli_help_describes_database_creation_and_setup_accurately() {
    let run_german_help = |arguments: &[&str]| {
        Command::new(binary())
            .args(arguments)
            .env("ZELYRA_LANGUAGE", "de")
            .output()
            .expect("zelyra help should run")
    };
    let create = run_german_help(&["db", "create", "--help"]);
    assert!(create.status.success());
    let create_help = String::from_utf8_lossy(&create.stdout);
    assert!(create_help.contains("Gibt das geprüfte CREATE-SQL"));
    assert!(create_help.contains("verbindet sich nicht mit einer Datenbank"));
    assert!(create_help.contains("zelyra db setup main.zyl"));

    let db_setup = run_german_help(&["db", "setup", "--help"]);
    assert!(db_setup.status.success());
    assert!(String::from_utf8_lossy(&db_setup.stdout).contains("MariaDB und MySQL"));
    let db_history = run_german_help(&["db", "history", "--help"]);
    assert!(db_history.status.success());
    assert!(String::from_utf8_lossy(&db_history.stdout).contains("MySQL-Schemamigrationen"));

    let setup = run_german_help(&["setup", "--help"]);
    assert!(setup.status.success());
    let setup_help = String::from_utf8_lossy(&setup.stdout);
    assert!(setup_help.contains("--all"));
    assert!(setup_help.contains("Webadresse"));
}

#[test]
fn mysql_db_setup_hint_uses_the_declared_connection_scheme() {
    let source = r#"
database main {
    engine: mysql
    database: "zelyra_ci"
}
fn main() {}
"#;
    let (directory, path) = temporary_project_source("mysql-setup-hint", source);
    let output = Command::new(binary())
        .args(["db", "setup", path.to_str().unwrap()])
        .env_remove("DATABASE_URL")
        .env_remove("ZELYRA_DATABASE_MAIN_URL")
        .output()
        .expect("zelyra should report a missing MySQL connection");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("DATABASE_URL is required"));
    assert!(stderr.contains("mysql://user:<password>"));
    assert!(!stderr.contains("mariadb://"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn long_binary_chain_survives_cli_project_validation_and_execution() {
    let expression = std::iter::repeat_n("1", 5_000)
        .collect::<Vec<_>>()
        .join(" + ");
    let source = format!("fn main() {{ print({expression}) }}");
    let (directory, path) = temporary_project_source("long-binary-chain", &source);
    let check = run(&["check", path.to_str().unwrap()]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let execution = run(&["run", path.to_str().unwrap()]);
    assert!(
        execution.status.success(),
        "{}",
        String::from_utf8_lossy(&execution.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&execution.stdout).trim(), "5000");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn run_step_budget_is_opt_in_and_reports_a_controlled_runtime_error() {
    let (directory, path) = temporary_project_source("step-budget", "fn main() { loop { } }");
    let output = run(&["run", path.to_str().unwrap(), "--max-steps", "30"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E-RUNTIME-001"));
    assert!(stderr.contains("step budget exceeded"));
    assert!(stderr.contains("main.zyl:"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn project_runtime_step_budget_limits_all_run_backends_without_cli_override() {
    let (directory, path) = temporary_project_source(
        "project-step-budget",
        "fn main() -> Int { mutable i: Int = 0 while i < 1000 { i = i + 1 } return i }",
    );
    fs::write(
        directory.join("zelyra.toml"),
        "[project]\nname = \"step-budget\"\n[runtime]\nmax_steps = 30\n",
    )
    .unwrap();

    for backend in ["ast", "mir", "bytecode"] {
        let output = run(&["run", path.to_str().unwrap(), "--backend", backend]);
        assert!(!output.status.success(), "{backend} unexpectedly succeeded");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("E-RUNTIME-001"), "{backend}: {stderr}");
        assert!(
            stderr.contains("execution step budget exceeded (limit 30)"),
            "{backend}: {stderr}"
        );
    }

    let overridden = run(&["run", path.to_str().unwrap(), "--max-steps", "7"]);
    assert!(!overridden.status.success());
    let stderr = String::from_utf8_lossy(&overridden.stderr);
    assert!(
        stderr.contains("execution step budget exceeded (limit 7)"),
        "{stderr}"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn run_step_budget_keeps_output_written_before_runtime_error() {
    let source = "fn main() { print(\"visible before failure\") loop { } }";
    let (directory, path) = temporary_project_source("step-budget-output", source);
    let output = run(&["run", path.to_str().unwrap(), "--max-steps", "30"]);
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "visible before failure"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E-RUNTIME-001"));
    assert!(stderr.contains("step budget exceeded"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn all_execution_backends_report_the_configured_step_budget_consistently() {
    let source = "fn main() -> Int { mutable i: Int = 0 while i < 100 { i = i + 1 } return i }";
    let (directory, path) = temporary_project_source("shared-step-budget", source);
    let mut failures = Vec::new();
    for backend in ["ast", "mir", "bytecode"] {
        let output = run(&[
            "run",
            path.to_str().unwrap(),
            "--backend",
            backend,
            "--max-steps",
            "20",
        ]);
        assert!(
            !output.status.success(),
            "backend {backend} unexpectedly succeeded"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("execution step budget exceeded (limit 20)"),
            "backend {backend}: {stderr}"
        );
        failures.push(stderr.into_owned());
    }
    assert!(failures
        .iter()
        .all(|failure| { failure.contains("execution step budget exceeded (limit 20)") }));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn reusable_csv_export_package_runs_with_matching_backend_output() {
    let entry = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/business_modules/main.zyl");
    let entry = entry.to_str().unwrap();
    let checked = run(&["check", entry]);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    let mut outputs = Vec::new();
    for backend in ["ast", "mir", "bytecode"] {
        let output = run(&["run", entry, "--entry", "csv_demo", "--backend", backend]);
        assert!(
            output.status.success(),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        outputs.push(output.stdout);
    }
    assert_eq!(
        outputs[0],
        b"number,customer,note\nINV-1,\"Acme, Inc.\",\"Paid\nby transfer\"\n"
    );
    assert!(outputs.iter().all(|output| output == &outputs[0]));
}

#[test]
fn reusable_csv_import_package_checks_rows_on_all_execution_backends() {
    let entry = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/business_modules/main.zyl");
    let entry = entry.to_str().unwrap();
    let checked = run(&["check", entry]);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    let expected = b"2\nAcme GmbH\n2\ncustomer_name\nAcme GmbH\nINV-1\nCSV preview row limit must be between 0 and 100\nCSV source column is missing: missing\nCSV source column is ambiguous: number\nCSV target column names must be unique\nCSV row 2 has a different number of columns than the header\nCSV header column 2 is empty\n";
    for backend in ["ast", "mir", "bytecode"] {
        let output = run(&[
            "run",
            entry,
            "--entry",
            "csv_import_demo",
            "--backend",
            backend,
        ]);
        assert!(
            output.status.success(),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, expected, "backend {backend}");
    }
}

#[test]
fn reusable_csv_import_package_performs_explicit_typed_conversion_on_all_backends() {
    let entry = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/business_modules/main.zyl");
    let entry = entry.to_str().unwrap();
    let checked = run(&["check", entry]);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    let expected = b"invoice_number\nINV-1\n2\ntrue\nempty note length 0\n12.34\nCSV row 2 column quantity: invalid integer\nCSV row 2 column amount: invalid decimal\n";
    for backend in ["ast", "mir", "bytecode"] {
        let output = run(&[
            "run",
            entry,
            "--entry",
            "csv_convert_demo",
            "--backend",
            backend,
        ]);
        assert!(
            output.status.success(),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, expected, "backend {backend}");
    }
}

#[test]
fn reusable_csv_import_package_performs_explicit_date_time_conversion_on_all_backends() {
    let entry = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/business_modules/main.zyl");
    let entry = entry.to_str().unwrap();
    let expected = b"2024-02-29\n09:15:30\nCSV row 2 column invoice_date: invalid date (expected YYYY-MM-DD)\nCSV row 2 column sent_at: invalid time (expected HH:MM:SS)\n";
    for backend in ["ast", "mir", "bytecode"] {
        let output = run(&[
            "run",
            entry,
            "--entry",
            "csv_temporal_demo",
            "--backend",
            backend,
        ]);
        assert!(
            output.status.success(),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, expected, "backend {backend}");
    }
}

#[test]
fn reusable_billing_package_calculates_line_amounts_on_all_backends() {
    let entry = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/business_modules/main.zyl");
    let entry = entry.to_str().unwrap();
    let checked = run(&["check", entry]);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    let expected = b"net=0.15 tax=0.03 gross=0.18\nquote=net:50.00 tax:10.00 gross:60.00\nrounding mode must be toward_zero, floor, ceiling, half_up, or half_even\n";
    for backend in ["ast", "mir", "bytecode"] {
        let output = run(&[
            "run",
            entry,
            "--entry",
            "billing_calculation_demo",
            "--backend",
            backend,
        ]);
        assert!(
            output.status.success(),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, expected, "backend {backend}");
    }
}

#[test]
fn run_can_invoke_an_explicit_parameterless_entry_without_running_main() {
    let source = r#"
fn main() { print("main entry") }
fn scheduled_invoice() { print("invoice task") }
"#;
    let (directory, path) = temporary_project_source("named-entry", source);
    let output = run(&[
        "run",
        path.to_str().unwrap(),
        "--entry",
        "scheduled_invoice",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "invoice task"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn run_rejects_missing_entries_and_requires_arguments_for_parameterized_entries() {
    let source = r#"
fn main() {}
fn takes_argument(value: Int) { print(value) }
fn generic_entry<T>() {}
"#;
    let (directory, path) = temporary_project_source("invalid-named-entry", source);
    let missing = run(&["run", path.to_str().unwrap(), "--entry", "missing"]);
    assert!(!missing.status.success());
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("entry function `missing` was not found")
    );

    let parameterized = run(&["run", path.to_str().unwrap(), "--entry", "takes_argument"]);
    assert!(!parameterized.status.success());
    assert!(String::from_utf8_lossy(&parameterized.stderr).contains("expects 1 argument(s)"));

    let generic = run(&["run", path.to_str().unwrap(), "--entry", "generic_entry"]);
    assert!(!generic.status.success());
    assert!(String::from_utf8_lossy(&generic.stderr).contains("generic type parameters"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn named_entries_accept_type_checked_json_arguments_on_each_backend() {
    let source = r#"
fn main() { print("wrong entry") }
fn scheduled_invoice(invoice_id: Int, attempt: UInt, dry_run: Bool, recipient: String) -> Int {
    print(invoice_id)
    print(attempt)
    print(dry_run)
    print(recipient)
    return 0
}
"#;
    let (directory, path) = temporary_project_source("named-entry-arguments", source);
    let args = r#"[501,2,false,"billing@example.test"]"#;
    let mut outputs = Vec::new();
    for backend in ["ast", "mir", "bytecode"] {
        let output = run(&[
            "run",
            path.to_str().unwrap(),
            "--entry",
            "scheduled_invoice",
            "--entry-args",
            args,
            "--backend",
            backend,
        ]);
        assert!(
            output.status.success(),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        outputs.push(output.stdout);
    }
    assert_eq!(outputs[0], b"501\n2\nfalse\nbilling@example.test\n");
    assert_eq!(outputs[1], outputs[0]);
    assert_eq!(outputs[2], outputs[0]);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn named_entry_arguments_reject_shape_and_type_errors_before_execution() {
    let source = r#"
fn main() {}
fn scheduled_invoice(invoice_id: Int) { print("must not run") }
"#;
    let (directory, path) = temporary_project_source("named-entry-argument-errors", source);
    for args in ["[\"501\"]", "[]", "[501,502]", "not-json"] {
        let output = run(&[
            "run",
            path.to_str().unwrap(),
            "--entry",
            "scheduled_invoice",
            "--entry-args",
            args,
        ]);
        assert!(!output.status.success(), "unexpected success for {args}");
        assert!(
            output.stdout.is_empty(),
            "entry ran for invalid arguments {args}"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("E-CLI-001"));
    }
    let no_entry = run(&["run", path.to_str().unwrap(), "--entry-args", "[501]"]);
    assert!(!no_entry.status.success());
    assert!(String::from_utf8_lossy(&no_entry.stderr).contains("requires --entry"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn named_entry_arguments_support_aliases_options_arrays_chars_and_json() {
    let source = r#"
type BatchCount = Int
fn main() {}
fn inspect(batch: BatchCount, note: Option<String>, values: Int[], marker: Char, metadata: Json) {
    print(batch)
    match note {
        Some(text) => { print(text) }
        None => { print("no note") }
    }
    print(values[1])
    print(marker)
}
"#;
    let (directory, path) = temporary_project_source("named-entry-argument-shapes", source);
    let output = run(&[
        "run",
        path.to_str().unwrap(),
        "--entry",
        "inspect",
        "--entry-args",
        r#"[8,"ready",[3,9],"✓",{"source":"scheduler"}]"#,
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "8\nready\n9\n✓\n");
    let no_note = run(&[
        "run",
        path.to_str().unwrap(),
        "--entry",
        "inspect",
        "--entry-args",
        r#"[8,null,[3,9],"✓",{"source":"scheduler"}]"#,
    ]);
    assert!(
        no_note.status.success(),
        "{}",
        String::from_utf8_lossy(&no_note.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&no_note.stdout),
        "8\nno note\n9\n✓\n"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn named_entry_arguments_do_not_erase_nominal_database_id_types() {
    let source = r#"
type OrderId = Id
fn main() {}
fn inspect(order_id: OrderId) { print("must not run") }
"#;
    let (directory, path) = temporary_project_source("named-entry-nominal-id", source);
    let output = run(&[
        "run",
        path.to_str().unwrap(),
        "--entry",
        "inspect",
        "--entry-args",
        "[501]",
    ]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E-CLI-001"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Id"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn named_entry_preserves_output_and_honors_step_budget() {
    let source = r#"
fn main() { print("wrong entry") }
fn limited_job() {
    print("job started")
    loop { }
}
"#;
    let (directory, path) = temporary_project_source("named-entry-budget", source);
    let output = run(&[
        "run",
        path.to_str().unwrap(),
        "--entry",
        "limited_job",
        "--max-steps",
        "30",
    ]);
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "job started"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E-RUNTIME-001"));
    assert!(stderr.contains("step budget exceeded"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn named_entry_maps_result_errors_and_integer_exit_codes_to_process_status() {
    let source = r#"
fn main() {}
fn failed_job() -> Result<Int, String> { return Err("invoice job failed") }
fn exit_job() -> Int { return 7 }
"#;
    let (directory, path) = temporary_project_source("named-entry-status", source);
    let failed = run(&["run", path.to_str().unwrap(), "--entry", "failed_job"]);
    assert_eq!(failed.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&failed.stderr).contains("entry function returned Err"));
    assert!(String::from_utf8_lossy(&failed.stderr).contains("returned Err"));
    assert!(String::from_utf8_lossy(&failed.stderr).contains("invoice job failed"));

    let failed_mir = run(&[
        "run",
        path.to_str().unwrap(),
        "--entry",
        "failed_job",
        "--backend",
        "mir",
    ]);
    assert_eq!(failed_mir.status.code(), Some(1));
    let failed_mir_stderr = String::from_utf8_lossy(&failed_mir.stderr);
    assert!(failed_mir_stderr.contains("returned Err"));
    assert!(failed_mir_stderr.contains("invoice job failed"));

    let exit = run(&["run", path.to_str().unwrap(), "--entry", "exit_job"]);
    assert_eq!(exit.status.code(), Some(7));
    let exit_mir = run(&[
        "run",
        path.to_str().unwrap(),
        "--entry",
        "exit_job",
        "--backend",
        "mir",
    ]);
    assert_eq!(exit_mir.status.code(), exit.status.code());
    let exit_bytecode = run(&[
        "run",
        path.to_str().unwrap(),
        "--entry",
        "exit_job",
        "--backend",
        "bytecode",
    ]);
    assert_eq!(exit_bytecode.status.code(), exit.status.code());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn run_mir_backend_is_opt_in_and_matches_ast_exit_code_for_supported_programs() {
    let source = "fn main() -> Int { return 7 }";
    let (directory, path) = temporary_project_source("mir-backend", source);
    let ast = run(&["run", path.to_str().unwrap(), "--backend", "ast"]);
    let mir = run(&["run", path.to_str().unwrap(), "--backend", "mir"]);
    assert_eq!(ast.status.code(), Some(7));
    assert_eq!(mir.status.code(), ast.status.code());
    assert!(mir.stdout.is_empty());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn run_mir_backend_honors_the_configured_step_budget() {
    let source = "fn main() { print(\"before budget\") loop { } }";
    let (directory, path) = temporary_project_source("mir-budget", source);
    let output = run(&[
        "run",
        path.to_str().unwrap(),
        "--backend",
        "mir",
        "--max-steps",
        "30",
    ]);
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "before budget"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("step budget exceeded"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn run_mir_backend_print_output_matches_ast() {
    let source = "fn main() { print(42) }";
    let (directory, path) = temporary_project_source("print-output", source);
    let ast = run(&["run", path.to_str().unwrap(), "--backend", "ast"]);
    assert!(ast.status.success());

    let mir = run(&["run", path.to_str().unwrap(), "--backend", "mir"]);
    assert!(mir.status.success());
    assert_eq!(
        String::from_utf8_lossy(&mir.stdout),
        String::from_utf8_lossy(&ast.stdout)
    );
    let bytecode = run(&["run", path.to_str().unwrap(), "--backend", "bytecode"]);
    assert!(
        bytecode.status.success(),
        "{}",
        String::from_utf8_lossy(&bytecode.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&bytecode.stdout),
        String::from_utf8_lossy(&ast.stdout)
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn run_mir_constant_folding_is_explicit_and_matches_ast_output() {
    let source = "fn main() { print((2 + 3) * 4) }";
    let (directory, path) = temporary_project_source("mir-constant-folding", source);
    let ast = run(&["run", path.to_str().unwrap(), "--backend", "ast"]);
    let optimized = run(&[
        "run",
        path.to_str().unwrap(),
        "--backend",
        "mir",
        "--optimize-mir",
    ]);
    assert!(ast.status.success());
    assert!(
        optimized.status.success(),
        "{}",
        String::from_utf8_lossy(&optimized.stderr)
    );
    assert_eq!(optimized.stdout, ast.stdout);
    assert_eq!(String::from_utf8_lossy(&optimized.stdout).trim(), "20");

    let without_mir = run(&["run", path.to_str().unwrap(), "--optimize-mir"]);
    assert_eq!(without_mir.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&without_mir.stderr).contains("requires --backend mir"));

    let bytecode = run(&[
        "run",
        path.to_str().unwrap(),
        "--backend",
        "bytecode",
        "--optimize-mir",
    ]);
    assert_eq!(bytecode.status.code(), Some(2));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn run_bytecode_backend_matches_ast_exit_code_for_supported_programs() {
    let source = "fn main() -> Int { return 7 }";
    let (directory, path) = temporary_project_source("bytecode-backend", source);
    let ast = run(&["run", path.to_str().unwrap(), "--backend", "ast"]);
    let bytecode = run(&["run", path.to_str().unwrap(), "--backend", "bytecode"]);
    assert_eq!(ast.status.code(), Some(7));
    assert_eq!(bytecode.status.code(), ast.status.code());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn run_bytecode_backend_matches_ast_for_stored_capturing_closures() {
    let source = r#"
fn make_adder(base: Int) -> Function<(Int) -> Int> {
    mutable offset = base
    callback: Function<(Int) -> Int> = |value| value + offset
    offset = offset + 100
    return callback
}
fn apply(values: Int[], callback: Function<(Int) -> Int>) -> Int[] {
    return values |> map(callback)
}
fn main() {
    add = make_adder(20)
    values = apply([2, 3], add)
    print(values[0] + values[1])
}
"#;
    let (directory, path) = temporary_project_source("bytecode-stored-closure", source);
    let ast = run(&["run", path.to_str().unwrap(), "--backend", "ast"]);
    let bytecode = run(&["run", path.to_str().unwrap(), "--backend", "bytecode"]);
    assert!(
        ast.status.success(),
        "{}",
        String::from_utf8_lossy(&ast.stderr)
    );
    assert!(
        bytecode.status.success(),
        "{}",
        String::from_utf8_lossy(&bytecode.stderr)
    );
    assert_eq!(bytecode.stdout, ast.stdout);
    assert_eq!(String::from_utf8_lossy(&bytecode.stdout).trim(), "45");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn run_bytecode_parallel_await_matches_ast_values_and_output_order() {
    let source = r#"
fn load(value: Int) -> Int {
    print(value)
    return value * 2
}
fn main() -> Int {
    mutable existing = 5
    parallel {
        first = await load(existing)
        existing = await load(3)
    }
    print(first + existing)
    return 0
}
"#;
    let (directory, path) = temporary_project_source("bytecode-parallel", source);
    let ast = run(&["run", path.to_str().unwrap(), "--backend", "ast"]);
    assert!(
        ast.status.success(),
        "{}",
        String::from_utf8_lossy(&ast.stderr)
    );
    let bytecode = run(&["run", path.to_str().unwrap(), "--backend", "bytecode"]);
    assert!(
        bytecode.status.success(),
        "{}",
        String::from_utf8_lossy(&bytecode.stderr)
    );
    assert_eq!(bytecode.stdout, ast.stdout);
    assert_eq!(String::from_utf8_lossy(&bytecode.stdout), "5\n3\n16\n");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn run_bytecode_backend_honors_the_configured_step_budget() {
    let source = "fn main() -> Int { print(\"before budget\") mutable value: Int = 0 while value < 1000 { value = value + 1 } return value }";
    let (directory, path) = temporary_project_source("bytecode-budget", source);
    let output = run(&[
        "run",
        path.to_str().unwrap(),
        "--backend",
        "bytecode",
        "--max-steps",
        "30",
    ]);
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "before budget"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("step budget exceeded"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn run_bytecode_console_preflight_requires_project_grant_before_prompt() {
    let directory = temporary_directory("bytecode-console-grant");
    fs::create_dir_all(&directory).unwrap();
    let project = directory.join("zelyra.toml");
    let source = directory.join("main.zyl");
    fs::write(
        &project,
        "[project]\nname = \"bytecode-console-grant\"\nversion = \"0.1.0\"\nzelyra = \"0.1\"\n\n[capabilities]\nconsole = true\ndatabase = false\nnetwork = false\n",
    )
    .unwrap();
    fs::write(
        &source,
        "fn main() uses Console { answer = read_console(\"Input? \") }\n",
    )
    .unwrap();

    let granted = Command::new(binary())
        .args(["run", source.to_str().unwrap(), "--backend", "bytecode"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        granted.status.success(),
        "{}",
        String::from_utf8_lossy(&granted.stderr)
    );
    assert_eq!(granted.stdout, b"Input? ");

    fs::write(
        &project,
        "[project]\nname = \"bytecode-console-grant\"\nversion = \"0.1.0\"\nzelyra = \"0.1\"\n\n[capabilities]\nconsole = false\ndatabase = false\nnetwork = false\n",
    )
    .unwrap();
    let denied = run(&["run", source.to_str().unwrap(), "--backend", "bytecode"]);
    assert!(!denied.status.success());
    assert!(denied.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&denied.stderr);
    assert!(stderr.contains("Console"), "{stderr}");
    assert!(stderr.contains("E-CAP-001"), "{stderr}");
    assert!(stderr.contains("not enabled by the project"), "{stderr}");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn run_bytecode_file_reads_use_project_roots_and_fail_closed_outside_them() {
    let directory = temporary_directory("bytecode-file-read-roots");
    let data = directory.join("data");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("input.txt"), "approved input\n").unwrap();
    fs::write(directory.join("outside.txt"), "private input\n").unwrap();
    fs::write(
        directory.join("zelyra.toml"),
        "[project]\nname = \"bytecode-file-read\"\nversion = \"0.1.0\"\nzelyra = \"0.1\"\n\n[capabilities]\nfilesystem = true\ndatabase = false\nnetwork = false\n\n[filesystem]\nread_roots = [\"data\"]\nwrite_roots = [\"data\"]\n",
    )
    .unwrap();
    let source = directory.join("main.zyl");
    fs::write(
        &source,
        "fn main() uses FileSystem { print(read_text(\"data/input.txt\")) }\n",
    )
    .unwrap();
    let allowed = run(&["run", source.to_str().unwrap(), "--backend", "bytecode"]);
    assert!(
        allowed.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&allowed.stdout),
        String::from_utf8_lossy(&allowed.stderr)
    );
    assert_eq!(allowed.stdout, b"approved input\n\n");

    fs::write(
        &source,
        "fn main() uses FileSystem { write_text(\"data/output.txt\", \"written\"); print(read_text(\"data/output.txt\")) }\n",
    )
    .unwrap();
    let write_allowed = run(&["run", source.to_str().unwrap(), "--backend", "bytecode"]);
    assert!(
        write_allowed.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&write_allowed.stdout),
        String::from_utf8_lossy(&write_allowed.stderr)
    );
    assert_eq!(write_allowed.stdout, b"written\n");
    assert_eq!(
        fs::read_to_string(data.join("output.txt")).unwrap(),
        "written"
    );

    fs::write(
        &source,
        "fn main() uses FileSystem { print(read_text(\"outside.txt\")) }\n",
    )
    .unwrap();
    let denied = run(&["run", source.to_str().unwrap(), "--backend", "bytecode"]);
    assert!(!denied.status.success());
    assert!(denied.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&denied.stderr);
    assert!(stderr.contains("E-RUNTIME-001"), "{stderr}");
    assert!(stderr.contains("outside the configured roots"), "{stderr}");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn run_mir_backend_retains_output_from_parallel_branches() {
    let source = r#"
fn fetch_left() -> Int { print("first") return 1 }
fn fetch_right() -> Int { print("second") return 2 }
fn main() -> Int {
    parallel {
        left = await fetch_left()
        right = await fetch_right()
    }
    return left + right
}
"#;
    let (directory, path) = temporary_project_source("mir-parallel-output", source);
    let output = run(&["run", path.to_str().unwrap(), "--backend", "mir"]);
    assert_eq!(output.status.code(), Some(3));
    let mut lines = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    lines.sort();
    assert_eq!(lines, ["first", "second"]);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn run_rejects_unknown_or_repeated_backend_selection() {
    let (directory, path) = temporary_project_source("backend-option", "fn main() {}");
    let unknown = run(&["run", path.to_str().unwrap(), "--backend", "native"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&unknown.stderr).contains("requires `ast`, `mir`, or `bytecode`")
    );

    let duplicate = run(&[
        "run",
        path.to_str().unwrap(),
        "--backend",
        "mir",
        "--backend",
        "ast",
    ]);
    assert_eq!(duplicate.status.code(), Some(2));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn run_help_documents_backend_default_and_mir_limit() {
    let output = run(&["run", "--help"]);
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("--backend ast|mir|bytecode"));
    assert!(help.contains("100.000 Schritte"));
    assert!(help.contains("AST bleibt Standard"));
    assert!(help.contains("--optimize-mir"));
}

#[test]
fn form_validate_covers_valid_and_invalid_inputs_without_database_access() {
    let source = example("customer_form.zyl");
    let valid = run(&[
        "form",
        "validate",
        source.to_str().unwrap(),
        "CustomerCreate",
        "name=Anna",
        "email=anna@example.org",
    ]);
    assert!(valid.status.success());
    assert!(String::from_utf8_lossy(&valid.stdout).contains("valid: CustomerCreate"));

    let invalid = run(&[
        "form",
        "validate",
        source.to_str().unwrap(),
        "CustomerCreate",
        "name=Anna",
        "email=not-an-email",
    ]);
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("E-FORM-004"));
}

#[test]
fn check_rejects_external_or_header_injecting_form_redirects() {
    let (directory, path) = temporary_project_source(
        "form-redirect-local",
        "form Contact { field email: Email {} action send { redirect \"/done?ok=1\" } } fn main() {}",
    );
    let output = Command::new(binary())
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("check command should run");
    assert!(
        output.status.success(),
        "local redirect should be accepted: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_dir_all(directory).unwrap();

    for (name, target) in [
        ("external", "https://outside.example/path"),
        ("protocol-relative", "//outside.example/path"),
        ("backslash", "/\\outside.example/path"),
        (
            "header-injection",
            "/done\r\nLocation: https://outside.example",
        ),
    ] {
        let source = format!(
            "form Contact {{ field email: Email {{}} action send {{ redirect {target:?} }} }} fn main() {{}}"
        );
        let (directory, path) = temporary_project_source(&format!("form-redirect-{name}"), &source);
        let output = Command::new(binary())
            .args(["check", path.to_str().unwrap(), "--format", "json"])
            .output()
            .expect("check command should run");
        assert!(!output.status.success(), "accepted redirect {target:?}");
        assert!(output.stderr.is_empty());
        let diagnostics: serde_json::Value = serde_json::from_slice(&output.stdout)
            .expect("check should return structured diagnostics");
        assert_eq!(diagnostics["success"], false);
        assert!(
            diagnostics["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["code"] == "E-FORM-007"),
            "missing redirect diagnostic for {target:?}: {diagnostics}"
        );
        fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn every_documented_cli_command_has_specific_nested_help() {
    let overview = run(&["--help"]);
    assert!(overview.status.success());
    let overview = String::from_utf8_lossy(&overview.stderr);
    for command in [
        "zelyra routes",
        "zelyra editor",
        "zelyra text edit",
        "zelyra outbox",
        "zelyra package lock",
        "zelyra db <create|setup|bootstrap|inspect|map|plan|apply|history>",
    ] {
        assert!(
            overview.contains(command),
            "top-level help omitted {command}"
        );
    }
    let command_help = [
        &["--tutorial", "invoice", "--help"][..],
        &["update", "--help"],
        &["new", "--help"],
        &["init", "--help"],
        &["setup", "--help"],
        &["setup", "--web", "--help"],
        &["check", "--help"],
        &["fmt", "--help"],
        &["impact", "--help"],
        &["edit", "--help"],
        &["text", "--help"],
        &["text", "edit", "--help"],
        &["context", "--help"],
        &["config", "--help"],
        &["build", "--help"],
        &["run", "--help"],
        &["serve", "--help"],
        &["routes", "--help"],
        &["editor", "--help"],
        &["module", "plan", "--help"],
        &["module", "bundle", "--help"],
        &["package", "--help"],
        &["package", "lock", "--help"],
        &["doctor", "--help"],
        &["verify", "--help"],
        &["doc", "--help"],
        &["auth", "hash-password", "--help"],
        &["auth", "role", "--help"],
        &["auth", "role-permission", "--help"],
        &["audit", "inspect", "--help"],
        &["audit", "export", "--help"],
        &["audit", "verify", "--help"],
        &["audit", "prune", "--help"],
        &["form", "validate", "--help"],
        &["db", "create", "--help"],
        &["db", "setup", "--help"],
        &["db", "bootstrap", "--help"],
        &["db", "inspect", "--help"],
        &["db", "map", "--help"],
        &["db", "plan", "--help"],
        &["db", "apply", "--help"],
        &["db", "history", "--help"],
        &["outbox", "setup", "--help"],
        &["outbox", "list", "--help"],
        &["outbox", "requeue", "--help"],
        &["outbox", "run", "--help"],
    ];
    for language in ["de", "en"] {
        for arguments in command_help {
            let output = Command::new(binary())
                .args(arguments)
                .env("ZELYRA_LANGUAGE", language)
                .output()
                .expect("zelyra help should run");
            assert!(
                output.status.success(),
                "{language} help failed for {arguments:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let help = String::from_utf8_lossy(&output.stdout);
            assert!(
                !help.contains("Detailed help is not available"),
                "missing specific English help for {arguments:?}"
            );
            assert!(
                !help.contains("Detaillierte Hilfe ist für diesen Befehl noch nicht verfügbar"),
                "missing specific German help for {arguments:?}"
            );
        }
    }
}

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples")
        .join(name)
}

fn temporary_source(name: &str, source: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "zelyra-machine-interface-{}-{}",
        std::process::id(),
        name
    ));
    fs::create_dir_all(&directory).expect("temporary directory should be created");
    let path = directory.join("main.zyl");
    fs::write(&path, source).expect("temporary source should be written");
    path
}

fn temporary_project_source(name: &str, source: &str) -> (PathBuf, PathBuf) {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(format!(".zelyra-edit-test-{}-{}", std::process::id(), name));
    fs::create_dir_all(&directory).expect("temporary project should be created");
    fs::write(
        directory.join("zelyra.toml"),
        "[project]\nname = \"edit-test\"\nversion = \"0.1.39\"\nzelyra = \"0.1\"\n\n[capabilities]\ndatabase = false\nnetwork = false\n",
    )
    .expect("temporary project config should be written");
    let source_path = directory.join("main.zyl");
    fs::write(&source_path, source).expect("temporary project source should be written");
    (directory, source_path)
}

#[test]
fn native_commands_and_schedules_are_listed_validated_run_and_exported_as_preview() {
    let source = r#"
command report_status {
    handler: report_status
    description: "Print the current business status"
}
schedule morning_status {
    command: report_status
    cron: "0 8 * * 1-5"
    timezone: "Europe/Berlin"
}
fn report_status() { print("ready") }
fn main() { }
"#;
    let (directory, path) = temporary_project_source("native-tasks", source);

    let commands = run(&["task", "list", path.to_str().unwrap(), "--json"]);
    assert!(
        commands.status.success(),
        "{}",
        String::from_utf8_lossy(&commands.stderr)
    );
    let command_json: serde_json::Value = serde_json::from_slice(&commands.stdout).unwrap();
    assert_eq!(command_json["commands"][0]["name"], "report_status");
    assert_eq!(command_json["commands"][0]["handler"], "report_status");

    let schedules = run(&["schedule", "list", path.to_str().unwrap(), "--json"]);
    assert!(
        schedules.status.success(),
        "{}",
        String::from_utf8_lossy(&schedules.stderr)
    );
    let schedule_json: serde_json::Value = serde_json::from_slice(&schedules.stdout).unwrap();
    assert_eq!(schedule_json["schedules"][0]["cron"], "0 8 * * 1-5");

    let config_home = directory.join("isolated-config");
    fs::create_dir_all(&config_home).unwrap();
    let status = Command::new(binary())
        .args(["schedule", "status", path.to_str().unwrap(), "--json"])
        .env("XDG_CONFIG_HOME", &config_home)
        .output()
        .expect("schedule status command should run");
    assert!(
        status.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&status.stdout),
        String::from_utf8_lossy(&status.stderr)
    );
    let status_json: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status_json["schema_version"], 1);
    assert_eq!(status_json["declared_schedule_count"], 1);
    assert_eq!(status_json["service_installed"], false);
    assert_eq!(status_json["service_active"], false);
    assert_eq!(status_json["scheduler_active"], false);
    assert_eq!(status_json["schedules"][0]["name"], "morning_status");

    let execution = run(&["task", "run", path.to_str().unwrap(), "report_status"]);
    assert!(
        execution.status.success(),
        "{}",
        String::from_utf8_lossy(&execution.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&execution.stdout).trim(), "ready");

    let preview = run(&[
        "schedule",
        "export",
        path.to_str().unwrap(),
        "--format",
        "cron",
    ]);
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let cron = String::from_utf8_lossy(&preview.stdout);
    assert!(cron.contains("CRON_TZ=Europe/Berlin\n0 8 * * 1-5"));
    assert!(cron.contains("schedule trigger"));
    assert!(cron.contains("--name 'morning_status'"));
    assert!(String::from_utf8_lossy(&preview.stderr).contains("did not install or enable"));

    let deploy_dir = directory.join("deploy");
    fs::create_dir(&deploy_dir).unwrap();
    let exported = run(&[
        "schedule",
        "export",
        path.to_str().unwrap(),
        "--output",
        "deploy/zelyra.cron",
    ]);
    assert!(
        exported.status.success(),
        "{}",
        String::from_utf8_lossy(&exported.stderr)
    );
    assert!(exported.stdout.is_empty());
    let cron_file = fs::read_to_string(deploy_dir.join("zelyra.cron")).unwrap();
    assert!(cron_file.contains("CRON_TZ=Europe/Berlin\n0 8 * * 1-5"));
    assert!(String::from_utf8_lossy(&exported.stderr).contains("did not install or enable"));

    let overwrite = run(&[
        "schedule",
        "export",
        path.to_str().unwrap(),
        "--output",
        "deploy/zelyra.cron",
    ]);
    assert!(!overwrite.status.success());
    assert_eq!(
        fs::read_to_string(deploy_dir.join("zelyra.cron")).unwrap(),
        cron_file
    );
    let escape = run(&[
        "schedule",
        "export",
        path.to_str().unwrap(),
        "--output",
        "../outside.cron",
    ]);
    assert!(!escape.status.success());
    assert!(String::from_utf8_lossy(&escape.stderr).contains("E-TASK-004"));

    fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn schedule_export_rejects_symlinked_output_directories_outside_project() {
    let (directory, path) = temporary_project_source(
        "schedule-output-symlink",
        "command c { handler: run_c }\nschedule s { command: c cron: \"0 0 * * *\" }\nfn run_c() {}\nfn main() {}\n",
    );
    let outside = temporary_directory("schedule-export-outside");
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, directory.join("linked"))
        .expect("test should create an output-directory symlink");
    let result = run(&[
        "schedule",
        "export",
        path.to_str().unwrap(),
        "--output",
        "linked/schedules.cron",
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("outside the project"));
    assert!(!outside.join("schedules.cron").exists());
    fs::remove_dir_all(directory).unwrap();
    fs::remove_dir_all(outside).unwrap();
}

fn temporary_directory(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "zelyra-machine-interface-{}-{}-{}",
        std::process::id(),
        name,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn assert_generated_database_module(directory: &Path) {
    let entry = directory.join("main.zyl");
    let source = fs::read_to_string(&entry).unwrap();
    assert!(source.starts_with("import \"src/database.zyl\" as storage\n"));
    assert!(!source.contains("database main"));
    assert_eq!(
        fs::read_to_string(directory.join("src/database.zyl"))
            .unwrap()
            .trim(),
        "database main {\n    engine: mariadb\n}"
    );
    let check = run(&["check", entry.to_str().unwrap()]);
    assert!(
        check.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}

fn free_test_port() -> String {
    TcpListener::bind(("127.0.0.1", 0))
        .expect("a test port should be available")
        .local_addr()
        .expect("test listener should have an address")
        .port()
        .to_string()
}

#[test]
fn run_reads_console_input_and_handles_eof_and_capability_denial() {
    let directory = temporary_directory("console-input");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("zelyra.toml"),
        "[project]\nname = \"console-input\"\nversion = \"0.2.0\"\nzelyra = \"0.1\"\n\n[capabilities]\nconsole = true\ndatabase = false\nnetwork = false\n",
    )
    .unwrap();
    let source = directory.join("main.zyl");
    fs::copy(example("console_input.zyl"), &source).unwrap();

    let run_with_input = |input: Option<&[u8]>| {
        let mut child = Command::new(binary())
            .args(["run", source.to_str().unwrap()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Zelyra process should start");
        if let Some(input) = input {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(input)
                .expect("test input should be written");
        } else {
            drop(child.stdin.take());
        }
        child.wait_with_output().unwrap()
    };

    let output = run_with_input(Some(b"2026-09-20\r\n"));
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"Datum: Eingegeben: 2026-09-20\n");
    assert!(output.stderr.is_empty());

    let empty_line = run_with_input(Some(b"\n"));
    assert!(empty_line.status.success());
    assert_eq!(empty_line.stdout, b"Datum: Eingegeben: \n");
    assert!(empty_line.stderr.is_empty());

    let eof = run_with_input(None);
    assert!(eof.status.success());
    assert_eq!(eof.stdout, b"Datum: Keine Eingabe.\n");
    assert!(eof.stderr.is_empty());

    fs::write(
        directory.join("zelyra.toml"),
        "[project]\nname = \"console-input\"\nversion = \"0.2.0\"\nzelyra = \"0.1\"\n\n[capabilities]\nconsole = false\ndatabase = false\nnetwork = false\n",
    )
    .unwrap();
    let denied = run_with_input(None);
    assert!(!denied.status.success());
    assert!(denied.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&denied.stderr);
    assert!(stderr.contains("E-CAP-001"), "unexpected stderr: {stderr}");
    assert!(stderr.contains("Console"), "unexpected stderr: {stderr}");

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn laravel_generation_requires_an_explicit_supported_selection() {
    let directory = temporary_directory("laravel-selection-required");
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("source-is-not-read-before-selection");
    let output_directory = directory.join("generated");
    let output = Command::new(binary())
        .args([
            "import",
            "laravel",
            "generate",
            source.to_str().unwrap(),
            "--output",
            output_directory.to_str().unwrap(),
        ])
        .output()
        .expect("Laravel importer should report the missing selection");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("requires an explicit selection") && stderr.contains("--all-supported"),
        "unexpected stderr: {stderr}"
    );
    assert!(!output_directory.exists());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn generated_laravel_identity_package_is_reusable_from_a_separate_project() {
    let directory = temporary_directory("laravel-identity-package-reuse");
    let source = directory.join("laravel-source");
    let generated = directory.join("generated");
    let consumer = directory.join("consumer");
    fs::create_dir_all(source.join("routes")).unwrap();
    fs::create_dir_all(source.join("app/Http/Controllers")).unwrap();
    fs::create_dir_all(source.join("database/migrations")).unwrap();
    fs::create_dir_all(source.join("resources/views/auth")).unwrap();
    fs::write(
        source.join("routes/web.php"),
        "<?php\nRoute::get('/login', [AuthController::class, 'showLogin']);\nRoute::post('/login', [AuthController::class, 'login']);\n",
    )
    .unwrap();
    fs::write(
        source.join("app/Http/Controllers/AuthController.php"),
        "<?php\nclass AuthController { public function showLogin() { return view('auth.login'); } public function login() { return null; } }\n",
    )
    .unwrap();
    fs::write(
        source.join("resources/views/auth/login.blade.php"),
        "<form method=\"post\"><input name=\"email\"><input name=\"password\"></form>",
    )
    .unwrap();
    fs::write(
        source.join("database/migrations/2026_01_01_000000_create_users_table.php"),
        "<?php\nSchema::create('users', function (Blueprint $table) { $table->id(); $table->string('name'); $table->string('email')->unique(); $table->string('password'); $table->rememberToken(); $table->timestamps(); });\n",
    )
    .unwrap();

    let generated = Command::new(binary())
        .args([
            "import",
            "laravel",
            "generate",
            source.to_str().unwrap(),
            "--output",
            generated.to_str().unwrap(),
            "--all-supported",
        ])
        .output()
        .expect("Laravel import should generate a native project scaffold");
    assert!(
        generated.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );
    assert!(directory
        .join("generated/packages/imported_identity/zelyra.toml")
        .is_file());

    fs::create_dir_all(consumer.join("packages")).unwrap();
    copy_directory(
        &directory.join("generated/packages/imported_identity"),
        &consumer.join("packages/imported_identity"),
    );
    fs::write(
        consumer.join("main.zyl"),
        "import \"pkg:identity\" as identity\n\nfn main() {}\n",
    )
    .unwrap();
    fs::write(
        consumer.join("zelyra.toml"),
        "[project]\nname = \"identity_consumer\"\nlanguage = \"0.1\"\n\n[package]\nname = \"identity_consumer\"\nversion = \"0.1.0\"\nentry = \"main.zyl\"\n\n[dependencies.identity]\npath = \"packages/imported_identity\"\nversion = \"^0.1.0\"\n",
    )
    .unwrap();

    for (arguments, label) in [
        (
            vec!["package", "lock", "main.zyl", "--update"],
            "package lock",
        ),
        (vec!["check", "main.zyl"], "consumer compiler check"),
    ] {
        let result = Command::new(binary())
            .current_dir(&consumer)
            .args(arguments)
            .output()
            .unwrap_or_else(|_| panic!("{label} should execute"));
        assert!(
            result.status.success(),
            "{label} failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let routes = Command::new(binary())
        .current_dir(&consumer)
        .args(["routes", "main.zyl", "--format=json"])
        .output()
        .expect("consumer routes should be listed");
    assert!(
        routes.status.success(),
        "{}",
        String::from_utf8_lossy(&routes.stderr)
    );
    let routes: serde_json::Value = serde_json::from_slice(&routes.stdout).unwrap();
    let routes = routes["routes"].as_array().unwrap();
    assert_eq!(routes.len(), 5);
    assert!(routes.iter().any(|route| route["path"] == "/login"));
    assert!(routes
        .iter()
        .any(|route| route["path"] == "/account/sessions"));

    fs::remove_dir_all(directory).unwrap();
}

fn copy_directory(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_directory(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn language_console_keeps_bindings_and_recovers_after_runtime_errors_in_piped_input() {
    let (directory, source) = temporary_project_source("language-console-repl", "fn main() {}\n");
    let mut child = Command::new(binary())
        .args(["console", source.to_str().unwrap(), "--sandbox"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("language console should start");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"x = 10\ny = 20\nx + y\n1 / 0\n\"weiter\"\n")
        .expect("REPL inputs should be written");
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "=> 30\n=> \"weiter\"\n"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("division by zero"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn language_console_bytecode_backend_keeps_bindings_and_recovers_after_errors() {
    let (directory, source) =
        temporary_project_source("language-console-backend", "fn main() {}\n");
    let mut child = Command::new(binary())
        .args(["console", source.to_str().unwrap(), "--backend", "bytecode"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("language console should start the bytecode backend");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"x = 10\ny = 20\nx + y\n1 / 0\n\"weiter\"\n")
        .expect("REPL inputs should be written");
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "=> 30\n=> \"weiter\"\n"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("division by zero"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn standard_layout_namespaces_resolve_and_remain_opt_in() {
    let directory = temporary_directory("standard-layout-namespaces");
    for path in ["schema", "domain/sales", "routes", "public", "assets"] {
        fs::create_dir_all(directory.join(path)).unwrap();
    }
    fs::write(
        directory.join("zelyra.toml"),
        "[project]\nname = \"namespace-test\"\nlayout = \"standard\"\n",
    )
    .unwrap();
    fs::write(
        directory.join("main.zyl"),
        "import \"domain::sales::actions\" as sales\nfn main() { print(sales::greeting()) }\n",
    )
    .unwrap();
    fs::write(
        directory.join("domain/sales/actions.zyl"),
        "pub fn greeting() -> String { return \"namespaces work\" }\n",
    )
    .unwrap();
    let entry = directory.join("main.zyl");
    let checked = Command::new(binary())
        .args(["check", entry.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    let run = Command::new(binary())
        .args(["run", entry.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        "namespaces work"
    );

    fs::write(
        &entry,
        "import \"domain::..::outside\" as outside\nfn main() {}\n",
    )
    .unwrap();
    let traversal = Command::new(binary())
        .args(["check", entry.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!traversal.status.success());
    assert!(String::from_utf8_lossy(&traversal.stderr)
        .contains("invalid standard-layout namespace import"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let outside = temporary_directory("standard-layout-namespace-outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(
            outside.join("actions.zyl"),
            "pub fn greeting() -> String { return \"outside\" }\n",
        )
        .unwrap();
        symlink(&outside, directory.join("domain/external")).unwrap();
        fs::write(
            &entry,
            "import \"domain::external::actions\" as external\nfn main() {}\n",
        )
        .unwrap();
        let escaped = Command::new(binary())
            .args(["check", entry.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(!escaped.status.success());
        assert!(String::from_utf8_lossy(&escaped.stderr).contains("E-MOD-004"));
        fs::remove_dir_all(outside).unwrap();
    }

    let legacy = temporary_directory("legacy-layout-namespace");
    fs::create_dir_all(legacy.join("domain/sales")).unwrap();
    fs::write(legacy.join("zelyra.toml"), "[project]\nname = \"legacy\"\n").unwrap();
    fs::write(
        legacy.join("main.zyl"),
        "import \"domain::sales::actions\" as sales\nfn main() {}\n",
    )
    .unwrap();
    fs::write(
        legacy.join("domain/sales/actions.zyl"),
        "pub fn greeting() -> String { return \"unreachable\" }\n",
    )
    .unwrap();
    let legacy_entry = legacy.join("main.zyl");
    let rejected = Command::new(binary())
        .args(["check", legacy_entry.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr)
        .contains("namespace imports require `[project].layout = \"standard\"`"));
    fs::remove_dir_all(directory).unwrap();
    fs::remove_dir_all(legacy).unwrap();
}

#[test]
fn digits_in_words_example_handles_signed_ints_and_unicode_output() {
    let directory = temporary_directory("digits-in-words");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("zelyra.toml"),
        "[project]\nname = \"digits-in-words\"\nversion = \"0.2.0\"\nzelyra = \"0.1\"\n\n[capabilities]\nconsole = true\ndatabase = false\nnetwork = false\n",
    )
    .unwrap();
    let source = directory.join("main.zyl");
    fs::copy(example("digits_in_words.zyl"), &source).unwrap();

    let run_with_input = |input: &[u8]| {
        let mut child = Command::new(binary())
            .args(["run", source.to_str().unwrap()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Zelyra process should start");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input)
            .expect("test input should be written");
        child.wait_with_output().unwrap()
    };

    for (input, expected) in [
        (b"124\n".as_slice(), "Zahl: Eins-Zwei-Vier\n"),
        (b"0\n".as_slice(), "Zahl: Null\n"),
        (b"-124\n".as_slice(), "Zahl: Minus-Eins-Zwei-Vier\n"),
        (b"5\n".as_slice(), "Zahl: Fünf\n"),
        (
            b"-9223372036854775808\n".as_slice(),
            "Zahl: Minus-Neun-Zwei-Zwei-Drei-Drei-Sieben-Zwei-Null-Drei-Sechs-Acht-Fünf-Vier-Sieben-Sieben-Fünf-Acht-Null-Acht\n",
        ),
        (
            b"9223372036854775807\n".as_slice(),
            "Zahl: Neun-Zwei-Zwei-Drei-Drei-Sieben-Zwei-Null-Drei-Sechs-Acht-Fünf-Vier-Sieben-Sieben-Fünf-Acht-Null-Sieben\n",
        ),
    ] {
        let output = run_with_input(input);
        assert!(
            output.status.success(),
            "input {input:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
        assert!(output.stderr.is_empty());
    }

    let invalid = run_with_input(b"not a number\n");
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("json_decode received invalid JSON"));

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn valid_check_json_is_a_stable_machine_document() {
    let path = example("fibonacci.zyl");
    let first = run(&["check", path.to_str().unwrap(), "--format=json"]);
    let second = run(&["check", path.to_str().unwrap(), "--format=json"]);
    assert!(
        first.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(second.status.success());
    assert_eq!(first.stderr, second.stderr);
    assert_eq!(first.stdout, second.stdout);
    assert!(first.stderr.is_empty());
    let document: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(document["schema_version"], "1");
    assert_eq!(document["command"], "check");
    assert_eq!(document["success"], true);
    assert_eq!(document["diagnostics"].as_array().unwrap().len(), 0);
}

#[test]
fn customer_order_acceptance_example_is_valid() {
    let output = run(&["check", example("customer_orders.zyl").to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn server_pdf_mail_outbox_example_checks_with_project_grants() {
    let path = example("pdf_mail_outbox/main.zyl");
    let output = run(&["check", path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("ok:"));
}

#[test]
fn language_107_reference_example_checks_and_runs() {
    let path = example("language_1_0_7/main.zyl");
    let check = run(&["check", path.to_str().unwrap()]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let execution = run(&["run", path.to_str().unwrap()]);
    assert!(
        execution.status.success(),
        "{}",
        String::from_utf8_lossy(&execution.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&execution.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["Paid", "1", "sale", "2", "6", "52", "Berlin", "Paid order"]
    );
}

#[test]
fn mutable_array_example_checks_and_runs() {
    let path = example("mutable_arrays/main.zyl");
    let check = run(&["check", path.to_str().unwrap()]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );

    let execution = run(&["run", path.to_str().unwrap()]);
    assert!(
        execution.status.success(),
        "{}",
        String::from_utf8_lossy(&execution.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&execution.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["5", "9", "2", "Some(12)"]
    );
}

#[test]
fn assertion_builtin_has_the_same_result_on_ast_mir_and_bytecode() {
    let source = r#"
fn main() {
    assert(true, "successful assertion")
    print("PASS assertion")
}
"#;
    let (directory, path) = temporary_project_source("assertion-backends", source);
    for backend in ["ast", "mir", "bytecode"] {
        let output = run(&["run", path.to_str().unwrap(), "--backend", backend]);
        assert!(
            output.status.success(),
            "backend {backend} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "PASS assertion"
        );
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn algorithm_workbench_examples_run_on_all_backends_with_expected_outputs() {
    let path = example("algorithms/main.zyl");
    let check = run(&["check", path.to_str().unwrap()]);
    assert!(
        check.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    let expected = [
        "PASS insertion sort: empty, single item, duplicates and negative values",
        "PASS linear search: empty, first, duplicate and missing values",
        "PASS array-backed stack: LIFO order and empty pop",
        "PASS bounded recursion: factorial base cases, valid input and rejected bounds",
        "PASS bottom-up Fibonacci: base cases, bounded work and invalid inputs",
        "PASS all algorithm examples",
    ];
    for backend in ["ast", "mir", "bytecode"] {
        let execution = run(&["run", path.to_str().unwrap(), "--backend", backend]);
        assert!(
            execution.status.success(),
            "backend {backend} failed; stdout: {}\nstderr: {}",
            String::from_utf8_lossy(&execution.stdout),
            String::from_utf8_lossy(&execution.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&execution.stdout)
                .lines()
                .collect::<Vec<_>>(),
            expected,
            "backend {backend} output differs"
        );
        assert!(
            execution.stderr.is_empty(),
            "backend {backend} wrote stderr"
        );
    }
}

#[test]
fn help_lists_the_route_inventory_command_and_syntax() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stderr);
    assert!(help.contains("zelyra routes <entry.zyl> [--format human|json]"));
    assert!(help.contains("lists declared pages, APIs, and generated resource routes"));
}

#[test]
fn command_help_explains_options_without_running_the_command() {
    let fmt = run(&["fmt", "--help"]);
    assert!(fmt.status.success());
    let fmt_help = String::from_utf8_lossy(&fmt.stdout);
    assert!(fmt_help.contains("zelyra fmt <datei.zyl> [--check]"));
    assert!(fmt_help.contains("schreibt nichts"));

    let init = run(&["init", "--help"]);
    assert!(init.status.success());
    let init_help = String::from_utf8_lossy(&init.stdout);
    assert!(init_help.contains("zelyra init [ordner] [Optionen]"));
    assert!(init_help.contains("--template minimal|mariadb-crud|mariadb-auth|mariadb-business"));

    let database = run(&["db", "plan", "--help"]);
    assert!(database.status.success());
    assert!(String::from_utf8_lossy(&database.stdout).contains("zelyra db plan <datei.zyl>"));

    let database_group = run(&["db", "--help"]);
    assert!(database_group.status.success());
    let database_help = String::from_utf8_lossy(&database_group.stdout);
    assert!(database_help.contains("ZELYRA_DATABASE_<NAME>_URL"));
    assert!(database_help.contains("DATABASE_URL"));

    let package = run(&["package", "--help"]);
    assert!(package.status.success());
    let package_help = String::from_utf8_lossy(&package.stdout);
    assert!(package_help.contains("cache clean [--apply]"));
    assert!(package_help.contains("update --plan <datei.toml>"));
    assert!(package_help.contains("Statischen Index"));
}

#[test]
fn command_help_is_available_in_english() {
    let output = Command::new(binary())
        .args(["fmt", "--help"])
        .env("ZELYRA_LANGUAGE", "en")
        .output()
        .expect("zelyra binary should run");
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("Command: zelyra fmt <file.zyl> [--check]"));
    assert!(help.contains("do not write changes"));

    let database = Command::new(binary())
        .args(["db", "--help"])
        .env("ZELYRA_LANGUAGE", "en")
        .output()
        .expect("database help command should run");
    assert!(database.status.success());
    let database_help = String::from_utf8_lossy(&database.stdout);
    assert!(database_help.contains("ZELYRA_DATABASE_<NAME>_URL"));
    assert!(database_help.contains("DATABASE_URL"));

    let package = Command::new(binary())
        .args(["package", "--help"])
        .env("ZELYRA_LANGUAGE", "en")
        .output()
        .expect("package help command should run");
    assert!(package.status.success());
    let package_help = String::from_utf8_lossy(&package.stdout);
    assert!(package_help.contains("cache clean [--apply]"));
    assert!(package_help.contains("update --plan <file.toml>"));
    assert!(package_help.contains("Build a static index"));
}

#[test]
fn invoice_tutorial_creates_a_checkable_project_without_overwriting() {
    let directory = temporary_directory("invoice-tutorial");
    let tutorial = run(&[
        "--tutorial",
        "invoice",
        directory.to_str().unwrap(),
        "--no-start",
    ]);
    assert!(
        tutorial.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&tutorial.stdout),
        String::from_utf8_lossy(&tutorial.stderr)
    );
    let source = fs::read_to_string(directory.join("main.zyl")).unwrap();
    assert!(source.contains("import \"src/customers.zyl\" as customers"));
    assert!(source.contains("import \"src/invoices.zyl\" as invoices"));
    let feature_sources = ["customers", "items", "invoices", "dashboard"]
        .map(|name| fs::read_to_string(directory.join(format!("src/{name}.zyl"))).unwrap())
        .join("\n");
    for section in [
        "table customers",
        "table items",
        "table invoices",
        "table invoice_lines",
        "tableview InvoiceDashboard",
    ] {
        assert!(
            feature_sources.contains(section),
            "tutorial source omitted {section}"
        );
    }
    assert!(directory.join("src/database.zyl").is_file());
    assert!(directory.join("src/shell.zyl").is_file());
    assert_eq!(
        fs::read_to_string(directory.join("zelyra.tutorial")).unwrap(),
        "invoice-v1\n"
    );
    let seed = fs::read_to_string(directory.join("invoice-tutorial.seed.sql")).unwrap();
    assert!(seed.contains("RE-2026-001"));
    assert!(seed.contains("WHERE NOT EXISTS"));
    let local_env = fs::read_to_string(directory.join(".env")).unwrap();
    assert!(local_env.contains("ZELYRA_DATABASE_MAIN_URL=mariadb://zelyra:"));
    assert!(local_env.contains("DATABASE_URL=mariadb://zelyra:"));
    assert!(!String::from_utf8_lossy(&tutorial.stdout).contains("MARIADB_PASSWORD="));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::metadata(directory.join(".env")).unwrap().permissions();
        assert_eq!(permissions.mode() & 0o777, 0o600);
    }
    assert!(feature_sources.contains("customers::Customer"));
    assert!(String::from_utf8_lossy(&tutorial.stdout).contains("zelyra setup --all"));
    assert!(
        String::from_utf8_lossy(&tutorial.stdout)
            .contains("application URL after startup: http://127.0.0.1:"),
        "tutorial output omitted the URL: {}",
        String::from_utf8_lossy(&tutorial.stdout)
    );
    let shell = fs::read_to_string(directory.join("src/shell.zyl")).unwrap();
    assert!(shell.contains("<html data-z-ui"));
    assert!(shell.contains("<AppShell"));

    let overwrite = run(&["--tutorial", "invoice", directory.to_str().unwrap()]);
    assert!(!overwrite.status.success());
    assert!(directory.join(".env").is_file());

    let check = run(&["check", directory.join("main.zyl").to_str().unwrap()]);
    assert!(
        check.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn mysql_schema_create_emits_checked_ddl_without_connecting_to_a_database() {
    let source = r#"
database main {
    engine: mysql
    database: "zelyra_ci"
}

table customers {
    id: Id primary auto
    email: String(100) required unique
}

fn main() {}
"#;
    let (directory, path) = temporary_project_source("mysql-schema-create", source);
    let check = run(&["check", path.to_str().unwrap()]);
    assert!(
        check.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );

    let schema = run(&["db", "create", path.to_str().unwrap()]);
    assert!(
        schema.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&schema.stdout),
        String::from_utf8_lossy(&schema.stderr)
    );
    assert!(String::from_utf8_lossy(&schema.stdout).contains("CREATE TABLE"));
    assert!(schema.stderr.is_empty());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn tenant_projects_reject_unscoped_tableviews_of_tenant_owned_tables() {
    let directory = temporary_directory("tenant-unscoped-tableview");
    fs::create_dir_all(&directory).unwrap();
    let mut source = fs::read_to_string(example("tenant_crud.zyl")).unwrap();
    source.push_str(
        r#"

struct InvoiceSummary {
    id: Id
    title: String
}

tableview InvoiceSummaryView {
    source sql<InvoiceSummary[]> {
        SELECT id, title FROM invoices
    }
    columns { id title }
}
"#,
    );
    let path = directory.join("main.zyl");
    fs::write(&path, &source).unwrap();

    let check = run(&["check", path.to_str().unwrap()]);
    assert!(!check.status.success());
    assert!(
        String::from_utf8_lossy(&check.stderr).contains("E-TENANT-003"),
        "stderr: {}",
        String::from_utf8_lossy(&check.stderr)
    );
    assert!(String::from_utf8_lossy(&check.stderr).contains("invoices"));

    let qualified = source.replace("FROM invoices", "FROM billing.invoices");
    fs::write(&path, qualified).unwrap();
    let check = run(&["check", path.to_str().unwrap()]);
    assert!(!check.status.success());
    assert!(String::from_utf8_lossy(&check.stderr).contains("E-TENANT-003"));

    let mut quoted = fs::read_to_string(example("tenant_crud.zyl")).unwrap();
    quoted.push_str(
        r#"

struct InvoiceSummary {
    id: Id
    title: String
}

fn load_invoices() uses Database {
    rows = sql<InvoiceSummary[]> { SELECT id, title FROM "invoices" }
}
"#,
    );
    fs::write(&path, quoted).unwrap();
    let check = run(&["check", path.to_str().unwrap()]);
    assert!(!check.status.success());
    assert!(
        String::from_utf8_lossy(&check.stderr).contains("E-TENANT-003"),
        "stderr: {}",
        String::from_utf8_lossy(&check.stderr)
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn language_compatibility_line_is_enforced_and_legacy_projects_stay_supported() {
    let directory = temporary_directory("language-compatibility-line");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("main.zyl"),
        "fn main() { print(\"compatible\") }\n",
    )
    .unwrap();
    let config = directory.join("zelyra.toml");
    fs::write(
        &config,
        "[project]\nname = \"compatibility-fixture\"\nversion = \"0.1.0\"\nzelyra = \"0.1\"\n",
    )
    .unwrap();
    let supported = run(&["check", directory.join("main.zyl").to_str().unwrap()]);
    assert!(
        supported.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&supported.stdout),
        String::from_utf8_lossy(&supported.stderr)
    );

    fs::write(
        &config,
        "[project]\nname = \"compatibility-fixture\"\nversion = \"0.1.0\"\nzelyra = \"0.1\" # supported line\n",
    )
    .unwrap();
    let commented = run(&["check", directory.join("main.zyl").to_str().unwrap()]);
    assert!(commented.status.success());

    fs::write(
        &config,
        "[project]\nname = \"compatibility-fixture\"\nversion = \"0.1.0\"\nzelyra = \"0.1#unsupported\"\n",
    )
    .unwrap();
    let hash_in_value = run(&["check", directory.join("main.zyl").to_str().unwrap()]);
    assert!(!hash_in_value.status.success());
    assert!(String::from_utf8_lossy(&hash_in_value.stderr).contains("E-LANG-001"));

    fs::write(
        &config,
        "[project]\nname = \"compatibility-fixture\"\nversion = \"0.1.0\"\nzelyra = \"0.2\"\n",
    )
    .unwrap();
    let unsupported = run(&["check", directory.join("main.zyl").to_str().unwrap()]);
    assert!(!unsupported.status.success());
    assert!(String::from_utf8_lossy(&unsupported.stderr).contains("E-LANG-001"));

    fs::write(
        &config,
        "[project]\nname = \"legacy-fixture\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let legacy = run(&["check", directory.join("main.zyl").to_str().unwrap()]);
    assert!(legacy.status.success());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn new_mariadb_project_propagates_the_selected_web_port() {
    let directory = temporary_directory("new-web-port");
    let web_host_port = free_test_port();
    let database_host_port = loop {
        let port = free_test_port();
        if port != web_host_port {
            break port;
        }
    };
    let output = run(&[
        "new",
        directory.to_str().unwrap(),
        "--mariadb",
        "--web-port",
        "8080",
        "--host-port",
        &web_host_port,
        "--db-host-port",
        &database_host_port,
    ]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_generated_database_module(&directory);
    let env_example = fs::read_to_string(directory.join(".env.example")).unwrap();
    let env_file = fs::read_to_string(directory.join(".env")).unwrap();
    let compose = fs::read_to_string(directory.join("docker-compose.mariadb.yml")).unwrap();
    assert!(env_example.contains("ZELYRA_WEB_PORT=8080"));
    assert!(env_example.contains("ZELYRA_LANGUAGE=de"));
    assert!(env_example.contains("ZELYRA_LEVEL=learn"));
    assert!(env_example.contains("ZELYRA_ALLOWED_HOSTS=localhost,127.0.0.1,[::1]"));
    assert!(compose.contains("0.0.0.0:${ZELYRA_WEB_PORT:-8080}"));
    assert!(compose.contains("ZELYRA_LANGUAGE: ${ZELYRA_LANGUAGE:-de}"));
    assert!(compose.contains("ZELYRA_LEVEL: ${ZELYRA_LEVEL:-learn}"));
    assert!(compose.contains("${ZELYRA_ALLOWED_HOSTS:-localhost,127.0.0.1,[::1]}"));
    assert!(env_example.contains(&format!("ZELYRA_HOST_PORT={web_host_port}")));
    assert!(env_example.contains(&format!("ZELYRA_DB_HOST_PORT={database_host_port}")));
    assert!(env_file.contains("ZELYRA_DATABASE_MAIN_URL=mariadb://zelyra:"));
    assert!(env_file.contains("ZELYRA_LANGUAGE=de"));
    assert!(env_file.contains("ZELYRA_LEVEL=learn"));
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains(&format!("http://127.0.0.1:{web_host_port}")));
    assert!(env_file.contains("ZELYRA_ALLOWED_HOSTS=localhost,127.0.0.1,[::1]"));
    assert!(env_file.contains("# ZELYRA_WEB_PORT=8080"));
    assert!(env_file.contains(&format!("# ZELYRA_HOST_PORT={web_host_port}")));
    assert!(env_file.contains("# ZELYRA_FEATURE_API=true"));
    assert!(env_file.contains("# ZELYRA_MFA_ENCRYPTION_KEY="));
    assert!(env_file.contains("# ZELYRA_SMTP_PASSWORD="));
    assert!(env_file
        .contains("# zelyra schedule export main.zyl --format cron --output deploy/zelyra.cron"));
    assert!(env_file.contains(
        "# Review deploy/zelyra.cron, then add its appropriate lines with `crontab -e`."
    ));
    assert!(!env_file.contains("\nZELYRA_FEATURE_API=true"));
    assert!(!env_file.contains("\nZELYRA_MFA_ENCRYPTION_KEY="));
    assert!(env_file.contains(&format!("ZELYRA_DB_HOST_PORT={database_host_port}")));
    assert!(!env_file.contains("change-me"));
    assert!(compose.contains(&format!(
        "127.0.0.1:${{ZELYRA_HOST_PORT:-{web_host_port}}}:${{ZELYRA_WEB_PORT:-8080}}"
    )));
    assert!(compose.contains(&format!(
        "127.0.0.1:${{ZELYRA_DB_HOST_PORT:-{database_host_port}}}:3306"
    )));
    for setting in [
        "ZELYRA_FEATURE_API: ${ZELYRA_FEATURE_API:-true}",
        "ZELYRA_AUTH_TOKEN: ${ZELYRA_AUTH_TOKEN:-}",
        "ZELYRA_MFA_ENCRYPTION_KEY: ${ZELYRA_MFA_ENCRYPTION_KEY:-}",
    ] {
        assert!(
            compose.contains(setting),
            "missing Compose forwarding: {setting}"
        );
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn new_standard_layout_generates_a_checkable_project_tree() {
    let directory = temporary_directory("new-standard-layout");
    let created = Command::new(binary())
        .args(["new", directory.to_str().unwrap(), "--layout", "standard"])
        .output()
        .unwrap();
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    let expected_files = [
        "schema/tables.zyl",
        "domain/dashboard/actions.zyl",
        "routes/web.zyl",
        "assets/.keep",
        "public/.keep",
    ];
    for file in expected_files {
        assert!(directory.join(file).is_file(), "missing {file}");
    }
    let manifest = fs::read_to_string(directory.join("zelyra.toml")).unwrap();
    assert!(manifest.contains("layout = \"standard\""));

    let entry = directory.join("main.zyl");
    let generated_main = fs::read_to_string(&entry).unwrap();
    assert!(generated_main.contains("import \"domain::dashboard::actions\" as dashboard"));
    let checked = Command::new(binary())
        .args(["check", entry.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    let executed = Command::new(binary())
        .args(["run", entry.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        executed.status.success(),
        "{}",
        String::from_utf8_lossy(&executed.stderr)
    );
    assert!(String::from_utf8_lossy(&executed.stdout).contains("Welcome to your Zelyra project"));
    let migration = Command::new(binary())
        .args([
            "migrate-layout",
            "--check",
            entry.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(
        migration.status.success(),
        "{}",
        String::from_utf8_lossy(&migration.stderr)
    );
    let migration_report: serde_json::Value = serde_json::from_slice(&migration.stdout).unwrap();
    assert_eq!(migration_report["schema_version"], 2);
    assert_eq!(migration_report["mode"], "read-only-check");
    assert_eq!(migration_report["summary"]["manual_review"], 0);
    let existing_domain = migration_report["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| file["path"] == "domain/dashboard/actions.zyl")
        .unwrap();
    assert_eq!(existing_domain["rule"], "LAYOUT-PLACEMENT-001");
    assert_eq!(existing_domain["span"]["line"], 1);
    assert_eq!(existing_domain["declarations"][0]["kind"], "function");
    assert_eq!(
        existing_domain["declarations"][0]["name"],
        "welcome_message"
    );
    assert_eq!(existing_domain["declarations"][0]["span"]["line"], 1);
    fs::remove_dir_all(directory.join("assets")).unwrap();
    let invalid = Command::new(binary())
        .args(["check", entry.to_str().unwrap(), "--format=json"])
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    let diagnostics: serde_json::Value = serde_json::from_slice(&invalid.stdout).unwrap();
    assert_eq!(diagnostics["diagnostics"][0]["code"], "E-LAYOUT-002");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn strict_architecture_rejects_route_declarations_inside_domain_modules() {
    let directory = temporary_directory("strict-architecture");
    for path in ["schema", "domain", "routes", "public", "assets"] {
        fs::create_dir_all(directory.join(path)).unwrap();
    }
    fs::write(
        directory.join("zelyra.toml"),
        "[project]\nname = \"strict-test\"\nlayout = \"standard\"\nstrict_architecture = true\n",
    )
    .unwrap();
    fs::write(
        directory.join("main.zyl"),
        "import \"domain::bad\" as bad\nfn main() {}\n",
    )
    .unwrap();
    fs::write(
        directory.join("domain/bad.zyl"),
        "page \"/misplaced\" { html { <p>misplaced</p> } }\n",
    )
    .unwrap();
    let checked = Command::new(binary())
        .args([
            "check",
            directory.join("main.zyl").to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert!(!checked.status.success());
    let report: serde_json::Value = serde_json::from_slice(&checked.stdout).unwrap();
    assert_eq!(report["diagnostics"][0]["code"], "E-ARCH-001");
    assert_eq!(report["diagnostics"][0]["file"], "domain/bad.zyl");
    assert_eq!(report["diagnostics"][0]["span"]["start"]["line"], 1);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn strict_architecture_checks_reachable_route_sql_and_preserves_domain_writes() {
    fn check_project(name: &str, route: &str, domain: Option<&str>) -> serde_json::Value {
        let directory = temporary_directory(name);
        for path in ["schema", "domain", "routes", "public", "assets"] {
            fs::create_dir_all(directory.join(path)).unwrap();
        }
        fs::write(
            directory.join("zelyra.toml"),
            "[project]\nname = \"strict-route-sql\"\nlayout = \"standard\"\nstrict_architecture = true\n\n[capabilities]\ndatabase = true\n",
        )
        .unwrap();
        fs::write(
            directory.join("main.zyl"),
            "import \"schema::db\" as db\nimport \"routes::customers\" as routes\nfn main() {}\n",
        )
        .unwrap();
        fs::write(
            directory.join("schema/db.zyl"),
            "database Main { engine: mariadb }\ntable customers { id: Id primary auto name: String required access { read: [\"routes/customers.zyl\"] write: [\"domain/actions.zyl\"] } }\n",
        )
        .unwrap();
        fs::write(
            directory.join("routes/customers.zyl"),
            format!("import \"schema::db\" as db\n{route}"),
        )
        .unwrap();
        if let Some(domain) = domain {
            fs::write(
                directory.join("domain/actions.zyl"),
                format!("import \"schema::db\" as db\n{domain}"),
            )
            .unwrap();
        }
        let output = Command::new(binary())
            .args([
                "check",
                directory.join("main.zyl").to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        fs::remove_dir_all(directory).unwrap();
        report
    }

    let direct = check_project(
        "strict-route-sql-direct",
        "fn create_customer() -> Int uses Database {\n    sql { INSERT INTO customers (name) VALUES ('Ada') }\n    return 1\n}\napi POST \"/customers\" { handler create_customer output Int }\n",
        None,
    );
    let diagnostic = &direct["diagnostics"][0];
    assert_eq!(diagnostic["code"], "E-ARCH-002");
    assert_eq!(diagnostic["file"], "routes/customers.zyl");
    assert_eq!(diagnostic["span"]["start"]["line"], 3);

    let through_local_helper = check_project(
        "strict-route-sql-helper",
        "fn write_customer() -> Int uses Database {\n    sql { INSERT INTO customers (name) VALUES ('Ada') }\n    return 1\n}\nfn create_customer() -> Int { return write_customer() }\napi POST \"/customers\" { handler create_customer output Int }\n",
        None,
    );
    assert_eq!(through_local_helper["diagnostics"][0]["code"], "E-ARCH-002");

    let through_domain = check_project(
        "strict-route-sql-domain",
        "import \"domain::actions\" as actions\nfn create_customer() -> Int uses Database { return actions::create_customer() }\napi POST \"/customers\" { handler create_customer output Int }\n",
        Some("pub fn create_customer() -> Int uses Database {\n    sql { INSERT INTO customers (name) VALUES ('Ada') }\n    return 1\n}\n"),
    );
    assert!(
        through_domain["diagnostics"].as_array().unwrap().is_empty(),
        "unexpected diagnostics: {through_domain}"
    );

    let read_query = check_project(
        "strict-route-sql-read",
        "fn get_count() -> Int uses Database(read) { return sql<Int> { SELECT 1 FROM customers LIMIT 1 } }\napi GET \"/customers/count\" { handler get_count output Int }\n",
        None,
    );
    assert!(
        read_query["diagnostics"].as_array().unwrap().is_empty(),
        "unexpected diagnostics: {read_query}"
    );

    let unknown_query = check_project(
        "strict-route-sql-unknown",
        "fn get_value() -> Int uses Database { return sql<Int> { SELECT SLEEP(1) FROM customers } }\napi GET \"/customers/value\" { handler get_value output Int }\n",
        None,
    );
    assert_eq!(unknown_query["diagnostics"][0]["code"], "E-ARCH-003");

    let form_action = check_project(
        "strict-route-sql-form-action",
        "form CustomerCreate -> customers { fields { name } action save { sql { INSERT INTO customers (name) VALUES ('Ada') } } }\n",
        None,
    );
    assert_eq!(form_action["diagnostics"][0]["code"], "E-ARCH-002");

    let crud_action = check_project(
        "strict-route-sql-crud-action",
        "crud Customer -> customers { action archive { sql { UPDATE customers SET name = 'Archived' } } }\n",
        None,
    );
    assert_eq!(crud_action["diagnostics"][0]["code"], "E-ARCH-002");
}

#[test]
fn strict_architecture_keeps_domain_modules_independent_from_routes() {
    let directory = temporary_directory("strict-domain-route-import");
    for path in ["schema", "domain", "routes", "public", "assets"] {
        fs::create_dir_all(directory.join(path)).unwrap();
    }
    fs::write(
        directory.join("zelyra.toml"),
        "[project]\nname = \"strict-domain-route-import\"\nlayout = \"standard\"\nstrict_architecture = true\n",
    )
    .unwrap();
    fs::write(
        directory.join("main.zyl"),
        "import \"routes::web\" as web\nimport \"domain::actions\" as actions\nfn main() {}\n",
    )
    .unwrap();
    fs::write(
        directory.join("routes/web.zyl"),
        "pub fn route_data() -> Int { return 1 }\napi GET \"/value\" { handler route_data output Int }\n",
    )
    .unwrap();
    fs::write(
        directory.join("domain/actions.zyl"),
        "import \"routes::web\" as web\npub fn calculate() -> Int { return web::route_data() }\n",
    )
    .unwrap();
    let checked = Command::new(binary())
        .args([
            "check",
            directory.join("main.zyl").to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&checked.stdout).unwrap();
    assert_eq!(report["diagnostics"][0]["code"], "E-ARCH-004");
    assert_eq!(report["diagnostics"][0]["file"], "domain/actions.zyl");
    assert_eq!(report["diagnostics"][0]["span"]["start"]["line"], 1);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn serve_loads_project_theme_and_locale_catalogs() {
    let directory = temporary_directory("serve-theme");
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("main.zyl");
    fs::write(
        &source,
        r#"page "/" {
    html {
        <html><head></head><body><div class="zelyra-app"><main><h1 data-zelyra-i18n="app.home_title"></h1><p data-zelyra-i18n="project.greeting"></p></main></div></body></html>
    }
}

"#,
    )
    .unwrap();
    let theme = ":root { --zelyra-color-accent: #e04b67; }\n";
    fs::write(directory.join("zelyra.theme.css"), theme).unwrap();
    fs::create_dir(directory.join("locales")).unwrap();
    fs::write(
        directory.join("locales/en.json"),
        r#"{"app.home_title":"Custom project title","project.greeting":"Welcome to our workshop"}"#,
    )
    .unwrap();

    let port = free_test_port();
    let address = format!("127.0.0.1:{port}");
    let mut server = Command::new(binary())
        .args(["serve", source.to_str().unwrap(), &address])
        .env("ZELYRA_LANGUAGE", "en")
        .env("ZELYRA_LEVEL", "work")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("Zelyra server should start");

    let send_request = |path: &str| -> Result<String, String> {
        let socket_address: SocketAddr = address
            .parse()
            .map_err(|error| format!("invalid test server address: {error}"))?;
        for _ in 0..50 {
            if let Ok(mut stream) =
                TcpStream::connect_timeout(&socket_address, Duration::from_millis(100))
            {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .map_err(|error| error.to_string())?;
                stream
                    .write_all(format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes())
                    .map_err(|error| error.to_string())?;
                let mut response = String::new();
                stream
                    .read_to_string(&mut response)
                    .map_err(|error| error.to_string())?;
                if !response.is_empty() {
                    return Ok(response);
                }
            }
            std::thread::sleep(Duration::from_millis(40));
        }
        Err("Zelyra server did not answer before the test timeout".into())
    };

    let result = (|| {
        let page = send_request("/")?;
        let stylesheet = send_request("/__zelyra/theme.css")?;
        Ok::<_, String>((page, stylesheet))
    })();
    let _ = server.kill();
    let _ = server.wait();
    fs::remove_dir_all(&directory).unwrap();

    let (page, stylesheet) = result.unwrap_or_else(|error| panic!("{error}"));
    assert!(page.starts_with("HTTP/1.1 200 OK"));
    assert!(page.contains("Custom project title"));
    assert!(page.contains("Welcome to our workshop"));
    let built_in = page.find("data-z-ui-stylesheet").unwrap();
    let project = page.find("data-zelyra-theme=\"project\"").unwrap();
    assert!(built_in < project);
    assert!(stylesheet.starts_with("HTTP/1.1 200 OK"));
    assert!(stylesheet.contains("Content-Type: text/css; charset=utf-8\r\n"));
    assert!(stylesheet.contains("X-Content-Type-Options: nosniff\r\n"));
    assert!(stylesheet.contains("Cache-Control: no-cache\r\n"));
    assert!(stylesheet.ends_with(theme));
}
#[test]
fn serve_mounts_authenticated_package_settings_page_with_the_real_store() {
    let directory = temporary_directory("serve-module-settings");
    let packages = directory.join("packages/settings_demo");
    fs::create_dir_all(packages.join("src")).unwrap();
    fs::write(
        directory.join("zelyra.toml"),
        "[package]\nname = \"settings_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[capabilities]\ndatabase = true\n\n[dependencies.settings_demo]\npath = \"packages/settings_demo\"\nversion = \"^1.0\"\n",
    )
    .unwrap();
    fs::write(
        packages.join("zelyra.toml"),
        "[package]\nname = \"settings_demo\"\nversion = \"1.0.0\"\nentry = \"src/lib.zyl\"\n\n[settings.currency]\ntype = \"String\"\nrequired = false\ndefault = \"EUR\"\n",
    )
    .unwrap();
    fs::write(
        packages.join("src/lib.zyl"),
        "pub fn currency() -> String { module_setting<String>(\"currency\") }\n",
    )
    .unwrap();
    let auth_source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/auth_crud_api.zyl"),
    )
    .unwrap();
    let source = directory.join("main.zyl");
    fs::write(
        &source,
        format!("import \"pkg:settings_demo\" as settings\n{auth_source}"),
    )
    .unwrap();
    let lock = Command::new(binary())
        .args(["package", "lock", source.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );

    let address = format!("127.0.0.1:{}", free_test_port());
    let mut server = Command::new(binary())
        .args(["serve", source.to_str().unwrap(), &address])
        .env("ZELYRA_AUTH_TOKEN", "module-settings-test-token")
        .env("ZELYRA_AUTH_PERMISSIONS", "settings.manage,auth.manage")
        .env("ZELYRA_ALLOWED_HOSTS", "localhost")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Zelyra server should start");

    let send_request = |authorization: Option<&str>| -> Result<String, String> {
        let socket_address: SocketAddr = address
            .parse()
            .map_err(|error| format!("invalid test server address: {error}"))?;
        for _ in 0..50 {
            if let Ok(mut stream) =
                TcpStream::connect_timeout(&socket_address, Duration::from_millis(100))
            {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .map_err(|error| error.to_string())?;
                let authorization = authorization
                    .map(|token| format!("Authorization: Bearer {token}\r\n"))
                    .unwrap_or_default();
                stream
                    .write_all(
                        format!(
                            "GET /__zelyra/module-settings HTTP/1.1\r\nHost: localhost\r\n{authorization}Connection: close\r\n\r\n"
                        )
                        .as_bytes(),
                    )
                    .map_err(|error| error.to_string())?;
                let mut response = String::new();
                stream
                    .read_to_string(&mut response)
                    .map_err(|error| error.to_string())?;
                if !response.is_empty() {
                    return Ok(response);
                }
            }
            std::thread::sleep(Duration::from_millis(40));
        }
        Err("Zelyra server did not answer before the test timeout".into())
    };

    let result = (|| {
        let denied = send_request(None)?;
        if !denied.starts_with("HTTP/1.1 401") {
            return Err(format!(
                "unauthenticated settings request was not rejected: {denied}"
            ));
        }
        let authorized = send_request(Some("module-settings-test-token"))?;
        if !authorized.starts_with("HTTP/1.1 200")
            || !authorized.contains("Module settings")
            || !authorized.contains("value=\"EUR\"")
        {
            return Err(format!(
                "authenticated settings page was not rendered: {authorized}"
            ));
        }
        let page_body = authorized
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .ok_or_else(|| "settings response had no body separator".to_owned())?;
        let input_value = |name: &str| -> Result<&str, String> {
            let marker = format!("name=\"{name}\" value=\"");
            page_body
                .split_once(&marker)
                .and_then(|(_, value)| value.split_once('"'))
                .map(|(value, _)| value)
                .ok_or_else(|| format!("settings page omitted hidden field {name}"))
        };
        let csrf = input_value("_zelyra_csrf")?;
        let revision = input_value("revision")?;
        let post_body = format!(
            "_zelyra_csrf={csrf}&revision={revision}&package_path=packages%2Fsettings_demo&name=currency&value=USD"
        );
        let socket_address: SocketAddr = address
            .parse()
            .map_err(|error| format!("invalid test server address: {error}"))?;
        let mut stream = TcpStream::connect_timeout(&socket_address, Duration::from_secs(1))
            .map_err(|error| error.to_string())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|error| error.to_string())?;
        stream
            .write_all(
                format!(
                    "POST /__zelyra/module-settings HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\nAuthorization: Bearer module-settings-test-token\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{post_body}",
                    post_body.len()
                )
                .as_bytes(),
            )
            .map_err(|error| error.to_string())?;
        let mut saved = String::new();
        stream
            .read_to_string(&mut saved)
            .map_err(|error| error.to_string())?;
        if !saved.starts_with("HTTP/1.1 303") {
            return Err(format!("authorized settings update failed: {saved}"));
        }
        let reloaded = send_request(Some("module-settings-test-token"))?;
        if !reloaded.contains("value=\"USD\"") {
            return Err(format!("saved setting was not reloaded: {reloaded}"));
        }
        Ok::<_, String>(())
    })();
    let _ = server.kill();
    let _ = server.wait();
    let mut server_stderr = String::new();
    if let Some(mut stderr) = server.stderr.take() {
        let _ = stderr.read_to_string(&mut server_stderr);
    }
    fs::remove_dir_all(directory).unwrap();
    result.unwrap_or_else(|error| panic!("{error}\nserver stderr: {server_stderr}"));
}

#[test]
fn serve_generated_crud_forms_use_project_catalogs_in_german_and_english() {
    let directory = temporary_directory("serve-crud-project-catalogs");
    fs::create_dir_all(directory.join("locales")).unwrap();
    let source = directory.join("main.zyl");
    fs::write(
        &source,
        r#"database main {
    engine: mariadb
}

table customers {
    id: Id primary auto
    name: String(100) required
}

crud Customer -> customers {
    list {
        name
    }
}
"#,
    )
    .unwrap();
    fs::write(
        directory.join("locales/en.json"),
        r#"{
            "form.create_title": "Add {field} to the directory",
            "form.create_submit": "Save English customer",
            "identifier.name": "English display name"
        }"#,
    )
    .unwrap();
    fs::write(
        directory.join("locales/de.json"),
        r#"{
            "form.create_title": "{field} zum Verzeichnis hinzufügen",
            "form.create_submit": "Kunden speichern",
            "identifier.name": "Deutscher Anzeigename"
        }"#,
    )
    .unwrap();

    let render_form = |language: &str| {
        let port = free_test_port();
        let address = format!("127.0.0.1:{port}");
        let mut server = Command::new(binary())
            .args(["serve", source.to_str().unwrap(), &address])
            .env("ZELYRA_LANGUAGE", language)
            .env("ZELYRA_LEVEL", "work")
            .env_remove("DATABASE_URL")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Zelyra server should start");

        let response = (|| {
            let socket_address: SocketAddr = address
                .parse()
                .map_err(|error| format!("invalid test server address: {error}"))?;
            for _ in 0..50 {
                if let Ok(mut stream) =
                    TcpStream::connect_timeout(&socket_address, Duration::from_millis(100))
                {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .map_err(|error| error.to_string())?;
                    stream
                        .write_all(b"GET /customers/new HTTP/1.1\r\nHost: localhost\r\n\r\n")
                        .map_err(|error| error.to_string())?;
                    let mut response = String::new();
                    stream
                        .read_to_string(&mut response)
                        .map_err(|error| error.to_string())?;
                    if !response.is_empty() {
                        return Ok(response);
                    }
                }
                std::thread::sleep(Duration::from_millis(40));
            }
            Err("Zelyra server did not answer before the test timeout".to_owned())
        })();
        let _ = server.kill();
        let _ = server.wait();
        response.unwrap_or_else(|error| panic!("{error}"))
    };

    let english = render_form("en");
    let german = render_form("de");
    fs::remove_dir_all(&directory).unwrap();

    assert!(english.starts_with("HTTP/1.1 200 OK"), "{english}");
    assert!(
        english.contains("Add Customer to the directory"),
        "{english}"
    );
    assert!(english.contains("English display name"), "{english}");
    assert!(english.contains("Save English customer"), "{english}");
    assert!(german.starts_with("HTTP/1.1 200 OK"), "{german}");
    assert!(
        german.contains("Kunde zum Verzeichnis hinzufügen"),
        "{german}"
    );
    assert!(german.contains("Deutscher Anzeigename"), "{german}");
    assert!(german.contains("Kunden speichern"), "{german}");
}

#[test]
fn generated_mariadb_business_starter_localizes_and_protects_crud_without_database() {
    const TEST_TOKEN: &str = "zelyra-business-e2e-test-token";

    let directory = temporary_directory("business-starter-localized-auth");
    let scaffold = run(&[
        "new",
        directory.to_str().unwrap(),
        "--template",
        "mariadb-business",
    ]);
    assert!(
        scaffold.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&scaffold.stdout),
        String::from_utf8_lossy(&scaffold.stderr)
    );
    let source = directory.join("main.zyl");
    // `serve` now honors project database settings from `.env`. This test is
    // specifically about static/authenticated routes without a configured DB.
    fs::write(
        directory.join(".env"),
        "# database intentionally unavailable\n",
    )
    .unwrap();
    let check = run(&["check", source.to_str().unwrap()]);
    assert!(
        check.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );

    fs::create_dir_all(directory.join("locales")).unwrap();
    fs::write(
        directory.join("locales/en.json"),
        r#"{
            "form.create_title": "Add {field} to the workspace",
            "form.create_submit": "Save customer profile",
            "identifier.name": "Customer display label",
            "error.authentication_required": "English access check required"
        }"#,
    )
    .unwrap();
    fs::write(
        directory.join("locales/de.json"),
        r#"{
            "form.create_title": "{field} zum Arbeitsbereich hinzufügen",
            "form.create_submit": "Kundenprofil speichern",
            "identifier.name": "Anzeigename des Kunden",
            "error.authentication_required": "Deutsche Anmeldung erforderlich"
        }"#,
    )
    .unwrap();

    let run_requests = |language: &str, requests: &[(&str, Option<&str>)]| {
        let port = free_test_port();
        let address = format!("127.0.0.1:{port}");
        let mut server = Command::new(binary())
            .args(["serve", source.to_str().unwrap(), &address])
            .env("ZELYRA_LANGUAGE", language)
            .env("ZELYRA_LEVEL", "work")
            .env("ZELYRA_AUTH_TOKEN", TEST_TOKEN)
            .env("ZELYRA_AUTH_PERMISSIONS", "customers.create")
            .env_remove("DATABASE_URL")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Zelyra business server should start");

        let responses = (|| {
            let socket_address: SocketAddr = address
                .parse()
                .map_err(|error| format!("invalid test server address: {error}"))?;
            let mut responses = Vec::with_capacity(requests.len());
            for (path, token) in requests {
                let authorization = token
                    .map(|token| format!("Authorization: Bearer {token}\r\n"))
                    .unwrap_or_default();
                let mut response = None;
                for _ in 0..50 {
                    if let Ok(mut stream) =
                        TcpStream::connect_timeout(&socket_address, Duration::from_millis(100))
                    {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .map_err(|error| error.to_string())?;
                        let request = format!(
                            "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n{authorization}\r\n"
                        );
                        stream
                            .write_all(request.as_bytes())
                            .map_err(|error| error.to_string())?;
                        let mut response_text = String::new();
                        stream
                            .read_to_string(&mut response_text)
                            .map_err(|error| error.to_string())?;
                        if !response_text.is_empty() {
                            response = Some(response_text);
                            break;
                        }
                    }
                    std::thread::sleep(Duration::from_millis(40));
                }
                responses.push(response.ok_or_else(|| {
                    "Zelyra business server did not answer before the test timeout".to_owned()
                })?);
            }
            Ok::<_, String>(responses)
        })();
        let _ = server.kill();
        let output = server
            .wait_with_output()
            .expect("collect business server output");
        responses.unwrap_or_else(|error| {
            panic!(
                "{error}; server exit: {}; stderr: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            )
        })
    };

    let english = run_requests(
        "en",
        &[
            ("/customers/new", None),
            ("/customers/new", Some(TEST_TOKEN)),
            ("/api/customers/1", Some(TEST_TOKEN)),
        ],
    );
    let german = run_requests(
        "de",
        &[
            ("/customers/new", None),
            ("/customers/new", Some(TEST_TOKEN)),
            ("/api/customers/1", Some(TEST_TOKEN)),
        ],
    );
    fs::remove_dir_all(&directory).unwrap();

    assert!(english[0].starts_with("HTTP/1.1 401 "), "{}", english[0]);
    assert!(english[0].contains("English access check required"));
    assert!(english[1].starts_with("HTTP/1.1 200 "));
    assert!(
        english[1].contains("Add Customer to the workspace"),
        "{}",
        english[1]
    );
    assert!(english[1].contains("Customer display label"));
    assert!(english[1].contains("Save customer profile"));
    assert!(english[2].starts_with("HTTP/1.1 403 "));

    assert!(german[0].starts_with("HTTP/1.1 401 "));
    assert!(german[0].contains("Deutsche Anmeldung erforderlich"));
    assert!(german[1].starts_with("HTTP/1.1 200 "));
    assert!(
        german[1].contains("Kunde zum Arbeitsbereich hinzufügen"),
        "{}",
        german[1]
    );
    assert!(german[1].contains("Anzeigename des Kunden"));
    assert!(german[1].contains("Kundenprofil speichern"));
    assert!(german[2].starts_with("HTTP/1.1 403 "));
    for response in english.iter().chain(&german) {
        assert!(!response.contains(TEST_TOKEN));
    }
}

#[test]
fn generated_api_metadata_and_rate_limit_reach_http_clients() {
    let directory = temporary_directory("api-metadata-rate-limit");
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("main.zyl");
    fs::write(
        &source,
        r#"
            api GET "/health" {
                version "v1"
                deprecated
                rate_limit 2 per 60
                handler health
                output String
            }

            fn health() -> String { return "ok" }
            fn main() { }
        "#,
    )
    .unwrap();

    let port = free_test_port();
    let address = format!("127.0.0.1:{port}");
    let mut server = Command::new(binary())
        .args(["serve", source.to_str().unwrap(), &address])
        .env("ZELYRA_LEVEL", "work")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Zelyra API server should start");

    let responses = (|| {
        let socket_address: SocketAddr = address
            .parse()
            .map_err(|error| format!("invalid test server address: {error}"))?;
        let mut responses = Vec::new();
        for index in 1..=3 {
            let mut result = None;
            for _ in 0..50 {
                let attempt = (|| {
                    let mut stream =
                        TcpStream::connect_timeout(&socket_address, Duration::from_millis(100))?;
                    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
                    stream
                        .write_all(
                            format!(
                                "GET /health HTTP/1.1\r\nHost: localhost\r\nX-Request-ID: test-{index}\r\n\r\n"
                            )
                            .as_bytes(),
                        )?;
                    let mut response = String::new();
                    stream.read_to_string(&mut response)?;
                    Ok::<_, std::io::Error>(response)
                })();
                match attempt {
                    Ok(response) if !response.is_empty() => {
                        result = Some(response);
                        break;
                    }
                    Ok(_) => {}
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::ConnectionRefused
                                | std::io::ErrorKind::ConnectionReset
                                | std::io::ErrorKind::ConnectionAborted
                                | std::io::ErrorKind::TimedOut
                                | std::io::ErrorKind::WouldBlock
                                | std::io::ErrorKind::UnexpectedEof
                                | std::io::ErrorKind::BrokenPipe
                        ) => {}
                    Err(error) => return Err(error.to_string()),
                }
                std::thread::sleep(Duration::from_millis(40));
            }
            responses.push(result.ok_or_else(|| {
                "Zelyra API server did not answer before the test timeout".to_owned()
            })?);
        }
        Ok::<_, String>(responses)
    })();
    let _ = server.kill();
    let _ = server.wait();
    fs::remove_dir_all(&directory).unwrap();

    let responses = responses.unwrap_or_else(|error| panic!("{error}"));
    assert!(
        responses[0].starts_with("HTTP/1.1 200 OK"),
        "{}",
        responses[0]
    );
    assert!(
        responses[1].starts_with("HTTP/1.1 200 OK"),
        "{}",
        responses[1]
    );
    assert!(
        responses[2].starts_with("HTTP/1.1 429 Too Many Requests"),
        "{}",
        responses[2]
    );
    for (index, response) in responses.iter().enumerate() {
        assert!(response.contains("X-Zelyra-API-Version: v1\r\n"));
        assert!(response.contains("X-Zelyra-API-Deprecated: true\r\n"));
        assert!(response.contains(&format!("X-Request-ID: test-{}\r\n", index + 1)));
    }
    assert!(responses[2].contains("Retry-After: "));
}

#[test]
fn serve_rejects_routes_that_conflict_with_the_project_theme_asset() {
    let directory = temporary_directory("serve-theme-route-conflict");
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("main.zyl");
    fs::write(
        &source,
        r#"page "/__zelyra/theme.css" {
    html { <main>Conflicting route</main> }
}
"#,
    )
    .unwrap();
    fs::write(directory.join("zelyra.theme.css"), "/* project theme */\n").unwrap();

    let output = Command::new(binary())
        .args(["serve", source.to_str().unwrap(), "127.0.0.1:0"])
        .output()
        .expect("Zelyra CLI should run");
    fs::remove_dir_all(&directory).unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E-THEME-002"),
        "unexpected stderr: {stderr}"
    );
    assert!(stderr.contains("/__zelyra/theme.css"));
}

#[test]
fn check_rejects_routes_that_shadow_the_builtin_liveness_endpoint() {
    let source = temporary_source(
        "health-route-conflict",
        r#"page "/__zelyra/health/live" {
    html { <main>Conflicting route</main> }
}
"#,
    );

    let output = run(&["check", source.to_str().unwrap()]);
    let _ = fs::remove_file(&source);
    let _ = fs::remove_dir(source.parent().unwrap());

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E-WEB-005"), "unexpected stderr: {stderr}");
    assert!(stderr.contains("/__zelyra/health/live"));
}

#[test]
fn serve_reports_invalid_project_locale_catalog_without_echoing_contents() {
    let directory = temporary_directory("serve-invalid-locale");
    fs::create_dir_all(directory.join("locales")).unwrap();
    let source = directory.join("main.zyl");
    fs::write(&source, "fn main() { print(\"ok\") }\n").unwrap();
    fs::write(
        directory.join("locales/en.json"),
        r#"{"custom.title":"PRIVATE_SENTINEL", bad-json}"#,
    )
    .unwrap();

    let output = run(&["serve", source.to_str().unwrap(), "127.0.0.1:0"]);
    fs::remove_dir_all(&directory).unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E-I18N-001"));
    assert!(stderr.contains("locales/en.json"));
    assert!(!stderr.contains("PRIVATE_SENTINEL"));
    assert!(output.stdout.is_empty());
}

#[test]
fn db_create_emits_checked_schema_ddl_without_connecting_to_a_database() {
    let output = run(&[
        "db",
        "create",
        example("machine_management_sqlite.zyl").to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CREATE TABLE"));
    assert!(stdout.contains("departments"));
    assert!(stdout.contains("machines"));
    assert!(output.stderr.is_empty());
}

#[test]
fn db_schema_errors_keep_the_imported_source_file_and_span() {
    let (directory, entry) = temporary_project_source(
        "schema-error-source",
        "import \"src/schema.zyl\" as schema\nfn main() {}\n",
    );
    fs::create_dir_all(directory.join("src")).unwrap();
    fs::write(
        directory.join("src/schema.zyl"),
        "table broken {\n    id: Id primary auto\n    id: Id primary auto\n}\n",
    )
    .unwrap();

    let output = run(&["db", "create", entry.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E-DB-001"), "{stderr}");
    assert!(stderr.contains("src/schema.zyl"), "{stderr}");
    assert!(stderr.contains(":3:"), "{stderr}");
    assert!(!stderr.contains("main.zyl"), "{stderr}");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn db_plan_json_emits_a_stable_versioned_fingerprint_and_approval_gate() {
    let directory = temporary_directory("migration-plan-json");
    fs::create_dir_all(&directory).unwrap();
    let database_path = directory.join("current.sqlite");
    let database_url = format!("sqlite://{}", database_path.display());
    zelyra_database::apply_sqlite(
        &database_url,
        "CREATE TABLE customers (id INTEGER PRIMARY KEY AUTOINCREMENT);",
    )
    .unwrap();
    let source = temporary_source(
        "migration-plan-json",
        "database main { engine: sqlite database: \"zelyra_test\" }\ntable customers { id: Id primary auto name: String(30) required }\n",
    );
    let run_plan = || {
        Command::new(binary())
            .args(["db", "plan", source.to_str().unwrap(), "--format=json"])
            .env("DATABASE_URL", &database_url)
            .env_remove("ZELYRA_DATABASE_MAIN_URL")
            .output()
            .unwrap()
    };
    let first = run_plan();
    let second = run_plan();
    assert!(
        first.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(first.stdout, second.stdout);
    assert!(!String::from_utf8_lossy(&first.stdout).contains(database_path.to_str().unwrap()));
    let document: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(document["format"], "zelyra.schema-plan/v1");
    assert_eq!(document["backend"], "sqlite");
    assert_eq!(document["drift"], "present");
    assert!(document["requires_operator_approval"].as_bool().unwrap());
    assert_eq!(document["rollback"]["generated"], true);
    assert!(document["rollback"]["plan_id"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert_eq!(document["rollback"]["requires_verified_backup"], true);
    assert_eq!(document["rollback"]["safe_to_apply_automatically"], false);
    assert_eq!(document["rollback"]["changes"][0]["risk"], "destructive");
    assert_eq!(document["automatic_retries"], false);
    assert!(document["plan_id"].as_str().unwrap().starts_with("sha256:"));
    assert_eq!(
        document["current_schema_sha256"].as_str().unwrap().len(),
        64
    );
    assert_eq!(
        document["desired_schema_sha256"].as_str().unwrap().len(),
        64
    );
    fs::remove_file(&source).unwrap();
    fs::remove_dir_all(source.parent().unwrap()).unwrap();
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn db_plan_reverse_id_applies_only_after_restoring_source_and_reviewing_it() {
    let directory = temporary_directory("reviewed-reverse-plan");
    fs::create_dir_all(&directory).unwrap();
    let database_path = directory.join("current.sqlite");
    let database_url = format!("sqlite://{}", database_path.display());
    zelyra_database::apply_sqlite(
        &database_url,
        "CREATE TABLE customers (id INTEGER PRIMARY KEY AUTOINCREMENT);",
    )
    .unwrap();
    let old_source = temporary_source(
        "reviewed-reverse-old",
        "database main { engine: sqlite database: \"zelyra_test\" }\ntable customers { id: Id primary auto }\n",
    );
    let new_source = temporary_source(
        "reviewed-reverse-new",
        "database main { engine: sqlite database: \"zelyra_test\" }\ntable customers { id: Id primary auto name: String(30) required }\n",
    );
    let run_plan = |source: &std::path::Path| {
        Command::new(binary())
            .args(["db", "plan"])
            .arg(source)
            .args(["--format=json"])
            .env("DATABASE_URL", &database_url)
            .env_remove("ZELYRA_DATABASE_MAIN_URL")
            .output()
            .unwrap()
    };
    let forward_plan = run_plan(&new_source);
    assert!(forward_plan.status.success());
    let forward_json: serde_json::Value = serde_json::from_slice(&forward_plan.stdout).unwrap();
    let forward_id = forward_json["plan_id"].as_str().unwrap();
    let reverse_id = forward_json["rollback"]["plan_id"].as_str().unwrap();

    let apply_forward = Command::new(binary())
        .args(["db", "apply"])
        .arg(&new_source)
        .args(["--allow-risky", "--plan-id", forward_id])
        .env("DATABASE_URL", &database_url)
        .env_remove("ZELYRA_DATABASE_MAIN_URL")
        .output()
        .unwrap();
    assert!(
        apply_forward.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&apply_forward.stdout),
        String::from_utf8_lossy(&apply_forward.stderr)
    );
    assert!(zelyra_database::inspect_sqlite(&database_url)
        .unwrap()
        .tables[0]
        .columns
        .iter()
        .any(|column| column.name == "name"));

    let reviewed_reverse = run_plan(&old_source);
    assert!(reviewed_reverse.status.success());
    let reverse_json: serde_json::Value = serde_json::from_slice(&reviewed_reverse.stdout).unwrap();
    assert_eq!(reverse_json["plan_id"], reverse_id);
    let apply_reverse = Command::new(binary())
        .args(["db", "apply"])
        .arg(&old_source)
        .args(["--allow-risky", "--plan-id", reverse_id])
        .env("DATABASE_URL", &database_url)
        .env_remove("ZELYRA_DATABASE_MAIN_URL")
        .output()
        .unwrap();
    assert!(
        apply_reverse.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&apply_reverse.stdout),
        String::from_utf8_lossy(&apply_reverse.stderr)
    );
    let restored = zelyra_database::inspect_sqlite(&database_url).unwrap();
    let customers = restored
        .tables
        .iter()
        .find(|table| table.name == "customers")
        .unwrap();
    assert!(!customers.columns.iter().any(|column| column.name == "name"));

    fs::remove_file(&old_source).unwrap();
    fs::remove_dir_all(old_source.parent().unwrap()).unwrap();
    fs::remove_file(&new_source).unwrap();
    fs::remove_dir_all(new_source.parent().unwrap()).unwrap();
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn db_apply_rejects_a_stale_reviewed_plan_before_applying_sql() {
    let directory = temporary_directory("stale-migration-plan");
    fs::create_dir_all(&directory).unwrap();
    let database_path = directory.join("current.sqlite");
    let database_url = format!("sqlite://{}", database_path.display());
    zelyra_database::apply_sqlite(
        &database_url,
        "CREATE TABLE customers (id INTEGER PRIMARY KEY AUTOINCREMENT);",
    )
    .unwrap();
    let source = temporary_source(
        "stale-migration-plan",
        "database main { engine: sqlite database: \"zelyra_test\" }\ntable customers { id: Id primary auto name: String(30) required }\n",
    );
    let plan = Command::new(binary())
        .args(["db", "plan", source.to_str().unwrap(), "--format=json"])
        .env("DATABASE_URL", &database_url)
        .env_remove("ZELYRA_DATABASE_MAIN_URL")
        .output()
        .unwrap();
    assert!(
        plan.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&plan.stdout),
        String::from_utf8_lossy(&plan.stderr)
    );
    let plan_json: serde_json::Value = serde_json::from_slice(&plan.stdout).unwrap();
    let old_plan_id = plan_json["plan_id"].as_str().unwrap();

    // An unrelated table changes the reviewed schema fingerprint without
    // changing the pending customers-column migration.
    zelyra_database::apply_sqlite(
        &database_url,
        "CREATE TABLE unrelated (id INTEGER PRIMARY KEY);",
    )
    .unwrap();
    let stale_apply = Command::new(binary())
        .args(["db", "apply"])
        .arg(&source)
        .args(["--allow-risky", "--plan-id", old_plan_id])
        .env("DATABASE_URL", &database_url)
        .env_remove("ZELYRA_DATABASE_MAIN_URL")
        .output()
        .unwrap();
    assert_eq!(stale_apply.status.code(), Some(1));
    let stale_stderr = String::from_utf8_lossy(&stale_apply.stderr);
    assert!(stale_stderr.contains("E-DB-007"), "{stale_stderr}");
    assert!(
        stale_stderr.contains("no SQL was applied"),
        "{stale_stderr}"
    );
    let empty_history = Command::new(binary())
        .args(["db", "history", source.to_str().unwrap(), "--format=json"])
        .env("DATABASE_URL", &database_url)
        .env_remove("ZELYRA_DATABASE_MAIN_URL")
        .output()
        .unwrap();
    assert!(empty_history.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&empty_history.stdout).unwrap(),
        serde_json::json!([])
    );
    assert_eq!(
        zelyra_database::query_sqlite(
            &database_url,
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_zelyra_schema_history'"
        )
        .unwrap()
        .trim(),
        "0"
    );
    let after_stale_apply = zelyra_database::inspect_sqlite(&database_url).unwrap();
    let customers = after_stale_apply
        .tables
        .iter()
        .find(|table| table.name == "customers")
        .unwrap();
    assert!(!customers.columns.iter().any(|column| column.name == "name"));
    assert!(after_stale_apply
        .tables
        .iter()
        .any(|table| table.name == "unrelated"));

    let fresh_plan = Command::new(binary())
        .args(["db", "plan", source.to_str().unwrap(), "--format=json"])
        .env("DATABASE_URL", &database_url)
        .env_remove("ZELYRA_DATABASE_MAIN_URL")
        .output()
        .unwrap();
    assert!(fresh_plan.status.success());
    let fresh_plan_json: serde_json::Value = serde_json::from_slice(&fresh_plan.stdout).unwrap();
    let fresh_plan_id = fresh_plan_json["plan_id"].as_str().unwrap();
    assert_ne!(old_plan_id, fresh_plan_id);
    let apply = Command::new(binary())
        .args(["db", "apply"])
        .arg(&source)
        .args(["--allow-risky", "--plan-id", fresh_plan_id])
        .env("DATABASE_URL", &database_url)
        .env_remove("ZELYRA_DATABASE_MAIN_URL")
        .output()
        .unwrap();
    assert!(
        apply.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&apply.stdout),
        String::from_utf8_lossy(&apply.stderr)
    );
    let after_apply = zelyra_database::inspect_sqlite(&database_url).unwrap();
    let customers = after_apply
        .tables
        .iter()
        .find(|table| table.name == "customers")
        .unwrap();
    assert!(customers.columns.iter().any(|column| column.name == "name"));
    assert!(!after_apply
        .tables
        .iter()
        .any(|table| table.name == "_zelyra_schema_history"));

    let history = Command::new(binary())
        .args(["db", "history", source.to_str().unwrap(), "--format=json"])
        .env("DATABASE_URL", &database_url)
        .env_remove("ZELYRA_DATABASE_MAIN_URL")
        .output()
        .unwrap();
    assert!(
        history.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&history.stdout),
        String::from_utf8_lossy(&history.stderr)
    );
    let history_json: serde_json::Value = serde_json::from_slice(&history.stdout).unwrap();
    assert_eq!(history_json.as_array().unwrap().len(), 1);
    assert_eq!(history_json[0]["status"], "applied");
    assert_eq!(history_json[0]["plan_id"], fresh_plan_id);
    assert_eq!(
        history_json[0]["completed_changes"],
        history_json[0]["change_count"]
    );
    zelyra_database::query_sqlite(
        &database_url,
        "UPDATE _zelyra_schema_history SET status='running'",
    )
    .unwrap();
    let interrupted = Command::new(binary())
        .args(["db", "history", source.to_str().unwrap(), "--format=json"])
        .env("DATABASE_URL", &database_url)
        .env_remove("ZELYRA_DATABASE_MAIN_URL")
        .output()
        .unwrap();
    assert!(interrupted.status.success());
    let interrupted_json: serde_json::Value = serde_json::from_slice(&interrupted.stdout).unwrap();
    assert_eq!(interrupted_json[0]["status"], "interrupted");
    assert_eq!(
        zelyra_database::query_sqlite(&database_url, "SELECT status FROM _zelyra_schema_history",)
            .unwrap()
            .trim(),
        "interrupted"
    );

    fs::remove_file(&source).unwrap();
    fs::remove_dir_all(source.parent().unwrap()).unwrap();
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn init_creates_a_ready_commented_mariadb_env() {
    let directory = temporary_directory("init-env-defaults");
    let database_host_port = free_test_port();
    let web_host_port = free_test_port();
    let output = run(&[
        "init",
        directory.to_str().unwrap(),
        "--mariadb",
        "--web-port",
        "8080",
        "--host-port",
        &web_host_port,
        "--db-host-port",
        &database_host_port,
    ]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let env_file = fs::read_to_string(directory.join(".env")).unwrap();
    assert!(env_file.contains("MARIADB_DATABASE=zelyra_app"));
    assert!(env_file.contains("MARIADB_USER=zelyra"));
    assert!(env_file.contains("MARIADB_PASSWORD="));
    assert!(env_file.contains("MARIADB_ROOT_PASSWORD="));
    assert!(env_file.contains("ZELYRA_LANGUAGE=de"));
    assert!(env_file.contains("ZELYRA_LEVEL=learn"));
    assert!(env_file.contains("# ZELYRA_FEATURE_API=true"));
    assert!(env_file.contains(&format!("ZELYRA_DB_HOST_PORT={database_host_port}")));
    assert!(
        env_file
            .find(&format!("ZELYRA_DB_HOST_PORT={database_host_port}"))
            .unwrap()
            < env_file
                .find("ZELYRA_DATABASE_MAIN_URL=mariadb://")
                .unwrap()
    );
    assert!(!env_file
        .lines()
        .any(|line| line == "ZELYRA_FEATURE_API=true"));
    assert!(!env_file.contains("change-me"));
    assert!(directory.join(".env.example").is_file());
    assert!(directory.join("locales/de.json").is_file());
    assert!(directory.join("locales/en.json").is_file());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn init_with_mariadb_preserves_an_existing_main_without_adding_an_unused_database_module() {
    let directory = temporary_directory("init-existing-main-mariadb");
    fs::create_dir_all(&directory).unwrap();
    let main_source = "fn main() {\n    print(\"existing application\")\n}\n";
    fs::write(directory.join("main.zyl"), main_source).unwrap();

    let web_host_port = free_test_port();
    let database_host_port = free_test_port();
    let output = run(&[
        "init",
        directory.to_str().unwrap(),
        "--mariadb",
        "--host-port",
        &web_host_port,
        "--db-host-port",
        &database_host_port,
    ]);

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(directory.join("main.zyl")).unwrap(),
        main_source
    );
    assert!(!directory.join("src/database.zyl").exists());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn default_mariadb_web_starter_is_catalog_localized_and_checkable() {
    let directory = temporary_directory("new-mariadb-starter-localized");
    let database_host_port = free_test_port();
    let output = run(&[
        "new",
        directory.to_str().unwrap(),
        "--mariadb",
        "--db-host-port",
        &database_host_port,
    ]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_generated_database_module(&directory);
    let source = fs::read_to_string(directory.join("main.zyl")).unwrap();
    assert!(source.contains("<html data-z-ui data-zelyra-language>"));
    assert!(source.contains("<AppShell id=\"starter\""));
    assert!(source.contains("<Card title=\"@i18n:starter.home_title\">"));
    assert!(source.contains("data-zelyra-language"));
    assert!(source.contains("title=\"@i18n:starter.workspace_title\""));
    assert!(!source.contains("A clear start for your next application."));

    let checked = run(&["check", directory.join("main.zyl").to_str().unwrap()]);
    assert!(
        checked.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&checked.stdout),
        String::from_utf8_lossy(&checked.stderr)
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn new_mariadb_crud_template_is_self_contained() {
    let directory = temporary_directory("new-mariadb-crud-template");
    let database_host_port = free_test_port();
    let web_host_port = free_test_port();
    let output = run(&[
        "new",
        directory.to_str().unwrap(),
        "--template",
        "mariadb-crud",
        "--web-port",
        "8080",
        "--host-port",
        &web_host_port,
        "--db-host-port",
        &database_host_port,
    ]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_generated_database_module(&directory);
    let source = fs::read_to_string(directory.join("main.zyl")).unwrap();
    let config = fs::read_to_string(directory.join("zelyra.toml")).unwrap();
    assert!(source.contains("table departments"));
    assert!(source.contains("form MachineCreate -> machines"));
    assert!(source.contains("crud Machine -> machines"));
    assert!(source.contains("import \"zelyra/ui\" as ui"));
    assert!(source.contains("<AppShell id=\"machines\""));
    assert!(config.contains("engine = \"mariadb\""));
    assert!(directory.join("docker-compose.mariadb.yml").is_file());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn new_mariadb_auth_template_is_self_contained() {
    let directory = temporary_directory("new-mariadb-auth-template");
    let database_host_port = free_test_port();
    let web_host_port = free_test_port();
    let output = run(&[
        "new",
        directory.to_str().unwrap(),
        "--template",
        "mariadb-auth",
        "--web-port",
        "8080",
        "--host-port",
        &web_host_port,
        "--db-host-port",
        &database_host_port,
    ]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_generated_database_module(&directory);
    let source = fs::read_to_string(directory.join("main.zyl")).unwrap();
    let config = fs::read_to_string(directory.join("zelyra.toml")).unwrap();
    assert!(source.contains("auth users"));
    assert!(source.contains("table auth_sessions"));
    assert!(source.contains("requires auth"));
    assert!(source.contains("permits \"admin.view\""));
    assert!(source.contains("import \"zelyra/ui\" as ui"));
    assert!(source.contains("<AppShell id=\"admin\""));
    assert!(config.contains("engine = \"mariadb\""));
    assert!(directory.join(".env.example").is_file());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn new_mariadb_business_template_is_self_contained() {
    let directory = temporary_directory("new-mariadb-business-template");
    let database_host_port = free_test_port();
    let web_host_port = free_test_port();
    let output = run(&[
        "new",
        directory.to_str().unwrap(),
        "--template",
        "mariadb-business",
        "--web-port",
        "8080",
        "--host-port",
        &web_host_port,
        "--db-host-port",
        &database_host_port,
    ]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_generated_database_module(&directory);
    let source = fs::read_to_string(directory.join("main.zyl")).unwrap();
    let config = fs::read_to_string(directory.join("zelyra.toml")).unwrap();
    assert!(source.contains("auth users"));
    assert!(source.contains("auth_audit_log"));
    assert!(source.contains("crud Customer -> customers"));
    assert!(source.contains("api GET \"/api/customers/{id}\""));
    assert!(source.contains("form CustomerQuickCreate -> customers"));
    assert!(config.contains("engine = \"mariadb\""));
    assert!(directory.join("docker-compose.mariadb.yml").is_file());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn new_rejects_an_invalid_web_port_before_creating_a_project() {
    let directory = temporary_directory("invalid-web-port");
    let output = run(&[
        "new",
        directory.to_str().unwrap(),
        "--mariadb",
        "--web-port",
        "65536",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("E-CLI-001"));
    assert!(!directory.exists());
}

#[test]
fn web_port_requires_the_mariadb_web_template() {
    let directory = temporary_directory("web-port-without-mariadb");
    let output = run(&["new", directory.to_str().unwrap(), "--web-port", "8080"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--mariadb"));
    assert!(!directory.exists());
}

#[test]
fn host_port_requires_the_mariadb_web_template() {
    let directory = temporary_directory("host-port-without-mariadb");
    let output = run(&["new", directory.to_str().unwrap(), "--host-port", "18080"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--mariadb"));
    assert!(!directory.exists());
}

#[test]
fn setup_creates_a_local_env_without_printing_or_overwriting_secrets() {
    let directory = temporary_directory("setup-env");
    let database_host_port = free_test_port();
    let web_host_port = free_test_port();
    let scaffold = run(&[
        "new",
        directory.to_str().unwrap(),
        "--mariadb",
        "--web-port",
        "8080",
        "--host-port",
        &web_host_port,
        "--db-host-port",
        &database_host_port,
    ]);
    assert!(
        scaffold.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&scaffold.stdout),
        String::from_utf8_lossy(&scaffold.stderr)
    );
    fs::remove_file(directory.join(".env")).unwrap();

    let setup = run(&["setup", directory.to_str().unwrap()]);
    assert!(
        setup.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&setup.stdout),
        String::from_utf8_lossy(&setup.stderr)
    );
    let stdout = String::from_utf8_lossy(&setup.stdout);
    assert!(!stdout.contains("MARIADB_PASSWORD"));
    assert!(!stdout.contains("change-me"));
    let env_file = directory.join(".env");
    let env_example = fs::read_to_string(directory.join(".env.example")).unwrap();
    let contents = fs::read_to_string(&env_file).unwrap();
    assert!(contents.contains("# ZELYRA_WEB_PORT=8080"));
    assert!(contents.contains(&format!("# ZELYRA_HOST_PORT={web_host_port}")));
    assert!(contents.contains(
        env_example
            .lines()
            .find(|line| line.starts_with("ZELYRA_DB_HOST_PORT="))
            .unwrap()
    ));
    assert!(!contents.contains("change-me"));
    assert!(contents.contains("ZELYRA_DATABASE_MAIN_URL=mariadb://zelyra:"));
    assert!(contents.contains("ZELYRA_DB_CONNECT_TIMEOUT_SECS=10"));
    assert!(contents.contains("ZELYRA_DB_QUERY_TIMEOUT_SECS=30"));
    assert!(contents.contains("ZELYRA_DB_POOL_MAX_SIZE=8"));
    assert!(contents.contains("ZELYRA_DB_POOL_WAIT_TIMEOUT_SECS=10"));
    assert!(contents.contains("MARIADB_PASSWORD="));
    assert!(contents.contains("MARIADB_ROOT_PASSWORD="));

    let doctor = run(&[
        "doctor",
        directory.join("main.zyl").to_str().unwrap(),
        "--env-file",
        env_file.to_str().unwrap(),
        "--port",
        "18080",
        "--json",
    ]);
    let doctor_document: serde_json::Value = serde_json::from_slice(&doctor.stdout).unwrap();
    let doctor_checks = doctor_document["checks"].as_array().unwrap();
    assert!(doctor_checks
        .iter()
        .any(|check| check["name"] == "env_file"));
    assert!(doctor_checks
        .iter()
        .any(|check| check["name"] == "docker_compose"));
    assert!(!String::from_utf8_lossy(&doctor.stdout).contains("change-me"));
    let database_password = contents
        .lines()
        .find_map(|line| line.strip_prefix("MARIADB_PASSWORD="))
        .unwrap();
    assert!(!String::from_utf8_lossy(&doctor.stdout).contains(database_password));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&env_file).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    let original = contents;
    let second_setup = run(&["setup", directory.to_str().unwrap()]);
    assert!(second_setup.status.success());
    assert!(String::from_utf8_lossy(&second_setup.stdout).contains("kept existing"));
    assert_eq!(fs::read_to_string(env_file).unwrap(), original);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn doctor_reports_unreachable_database_without_exposing_credentials() {
    let directory = temporary_directory("doctor-unreachable-database");
    fs::create_dir_all(&directory).unwrap();
    let secret = "doctor-secret-redaction";
    let env_file = directory.join(".env");
    fs::write(
        &env_file,
        format!("DATABASE_URL=mariadb://doctor:{secret}@127.0.0.1:1/doctor_db\n"),
    )
    .unwrap();

    let output = run(&[
        "doctor",
        example("machine_form.zyl").to_str().unwrap(),
        "--env-file",
        env_file.to_str().unwrap(),
        "--port",
        &free_test_port(),
        "--json",
    ]);
    assert!(
        !output.status.success(),
        "doctor unexpectedly accepted an unavailable database"
    );
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["status"], "failed");
    let database_check = document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == "database")
        .expect("doctor should report a database check");
    assert_eq!(database_check["status"], "fail");
    assert_eq!(database_check["category"], "connectivity");
    assert!(database_check["message"]
        .as_str()
        .unwrap()
        .contains("mariadb"));
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!combined.contains(secret));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn setup_selects_free_ports_for_a_new_local_environment() {
    let directory = temporary_directory("setup-free-ports");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("zelyra.toml"),
        "[database.main]\nengine = \"mariadb\"\n",
    )
    .unwrap();
    let web_listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let requested_web_port = web_listener.local_addr().unwrap().port();
    let database_listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let requested_database_port = database_listener.local_addr().unwrap().port();
    fs::write(
        directory.join(".env.example"),
        format!(
            "ZELYRA_DB_HOST_PORT={requested_database_port}\nZELYRA_DATABASE_MAIN_URL=mariadb://zelyra:change-me@127.0.0.1:${{ZELYRA_DB_HOST_PORT:-3306}}/zelyra_app\nMARIADB_DATABASE=zelyra_app\nMARIADB_USER=zelyra\nMARIADB_PASSWORD=change-me\nMARIADB_ROOT_PASSWORD=change-me-root\n# ZELYRA_HOST_PORT={requested_web_port}\n"
        ),
    )
    .unwrap();

    let output = run(&["setup", directory.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("selected free port"));
    let env_file = fs::read_to_string(directory.join(".env")).unwrap();
    assert!(env_file.contains("ZELYRA_HOST_PORT="));
    assert!(!env_file.contains(&format!("ZELYRA_HOST_PORT={requested_web_port}")));
    assert!(!env_file.contains(&format!("ZELYRA_DB_HOST_PORT={requested_database_port}")));
    assert!(env_file.contains("${ZELYRA_DB_HOST_PORT:-3306}"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn setup_rejects_web_server_port_without_the_web_assistant() {
    let directory = temporary_directory("setup-port-without-web");
    fs::create_dir_all(&directory).unwrap();
    let output = run(&["setup", directory.to_str().unwrap(), "--port", "3031"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--port requires --web"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn setup_rejects_an_explicitly_occupied_host_port_without_creating_env() {
    let directory = temporary_directory("setup-explicit-port-conflict");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("zelyra.toml"),
        "[database.main]\nengine = \"mariadb\"\n",
    )
    .unwrap();
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let occupied_port = listener.local_addr().unwrap().port().to_string();
    let output = run(&[
        "setup",
        directory.to_str().unwrap(),
        "--host-port",
        &occupied_port,
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("already in use"));
    assert!(!directory.join(".env").exists());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn setup_refuses_to_change_a_static_database_url_port() {
    let directory = temporary_directory("setup-static-database-url");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("zelyra.toml"),
        "[database.main]\nengine = \"mariadb\"\n",
    )
    .unwrap();
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let occupied_port = listener.local_addr().unwrap().port();
    fs::write(
        directory.join(".env.example"),
        format!(
            "ZELYRA_DB_HOST_PORT={occupied_port}\nDATABASE_URL=mariadb://zelyra:change-me@127.0.0.1:{occupied_port}/zelyra_app\nMARIADB_PASSWORD=change-me\nMARIADB_ROOT_PASSWORD=change-me-root\n"
        ),
    )
    .unwrap();

    let output = run(&["setup", directory.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot safely select"));
    assert!(!directory.join(".env").exists());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn setup_rejects_a_directory_without_a_mariadb_scaffold() {
    let directory = temporary_directory("setup-missing-scaffold");
    fs::create_dir_all(&directory).unwrap();
    let output = run(&["setup", directory.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("E-SETUP-001"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn setup_guides_the_reported_absolute_path_typo_and_minimal_init_flow() {
    let working_directory = std::env::temp_dir();
    let directory = temporary_directory("setup-human-onboarding");
    let relative_directory = format!("./{}", directory.file_name().unwrap().to_string_lossy());
    let project = directory.join("test_a");
    fs::create_dir_all(&directory).unwrap();
    fs::create_dir_all(&project).unwrap();
    let run_from = |working_directory: &Path, arguments: &[&str]| {
        Command::new(binary())
            .current_dir(working_directory)
            .args(arguments)
            .output()
            .expect("zelyra binary should run")
    };

    let mistaken_absolute = format!(
        "/{}/test_a",
        directory.file_name().unwrap().to_string_lossy()
    );
    let relative_project = format!("{relative_directory}/test_a");
    let setup_typo = run_from(&working_directory, &["setup", &mistaken_absolute]);
    assert_eq!(setup_typo.status.code(), Some(1));
    let typo_error = String::from_utf8_lossy(&setup_typo.stderr);
    assert!(typo_error.contains("does not exist"), "{typo_error}");
    assert!(typo_error.contains("zelyra setup ./"), "{typo_error}");

    let init = run_from(&working_directory, &["init", &relative_project]);
    assert!(
        init.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );

    let setup_typo_after_init = run_from(&working_directory, &["setup", &mistaken_absolute]);
    assert_eq!(setup_typo_after_init.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&setup_typo_after_init.stderr).contains("zelyra setup ./"));

    let setup_minimal_project = run_from(&project, &["setup"]);
    assert_eq!(setup_minimal_project.status.code(), Some(1));
    let setup_error = String::from_utf8_lossy(&setup_minimal_project.stderr);
    assert!(setup_error.contains("runs without setup"), "{setup_error}");
    assert!(
        setup_error.contains("zelyra init <directory> --mariadb"),
        "{setup_error}"
    );
    assert!(!project.join(".env").exists());

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn invalid_check_json_keeps_human_logs_off_stdout() {
    let invalid_sql = example("invalid_sql.zyl");
    let output = run(&["check", invalid_sql.to_str().unwrap(), "--format", "json"]);
    assert!(!output.status.success());
    assert!(output.stderr.is_empty());
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["schema_version"], "1");
    assert_eq!(document["success"], false);
    let diagnostics = document["diagnostics"].as_array().unwrap();
    assert!(!diagnostics.is_empty());
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic["code"] == "E-NAME-001"));
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic["code"].as_str().is_some()
            && diagnostic["severity"] == "error"
            && diagnostic["file"].as_str().is_some()
            && diagnostic["span"]["start"]["offset"].is_number()
            && diagnostic["span"]["end"]["offset"].is_number()
    }));
}

#[test]
fn machine_diagnostics_use_project_relative_paths() {
    let path = example("invalid_sql.zyl").canonicalize().unwrap();
    let output = run(&["check", path.to_str().unwrap(), "--format=json"]);
    assert_eq!(output.status.code(), Some(1));
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        document["diagnostics"][0]["file"],
        "examples/invalid_sql.zyl"
    );
}

#[test]
fn machine_impact_uses_project_relative_entry_paths() {
    let path = example("auth_crud_api.zyl").canonicalize().unwrap();
    let output = run(&["impact", path.to_str().unwrap(), "--format=json"]);
    assert!(output.status.success());
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["entry"], "examples/auth_crud_api.zyl");
}

#[test]
fn lexer_and_parser_errors_have_versioned_json_diagnostics() {
    let lexer_path = temporary_source("lexer-error", "fn main() { @ }");
    let lexer_output = run(&["check", lexer_path.to_str().unwrap(), "--format=json"]);
    assert_eq!(lexer_output.status.code(), Some(1));
    let lexer_document: serde_json::Value = serde_json::from_slice(&lexer_output.stdout).unwrap();
    assert_eq!(lexer_document["diagnostics"][0]["code"], "E-LEX-001");

    let parser_path = temporary_source("parser-error", "fn main( { }");
    let parser_output = run(&["check", parser_path.to_str().unwrap(), "--format=json"]);
    assert_eq!(parser_output.status.code(), Some(1));
    let parser_document: serde_json::Value = serde_json::from_slice(&parser_output.stdout).unwrap();
    assert_eq!(parser_document["diagnostics"][0]["code"], "E-PARSE-001");
}

#[test]
fn unicode_positions_use_documented_utf8_byte_offsets() {
    let source = "fn main() { print(\"ä\") @ }";
    let path = temporary_source("unicode-error", source);
    let output = run(&["check", path.to_str().unwrap(), "--format=json"]);
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let offset = document["diagnostics"][0]["span"]["start"]["offset"]
        .as_u64()
        .unwrap() as usize;
    assert_eq!(offset, source.find('@').unwrap());
}

#[test]
fn empty_source_is_a_valid_deterministic_context() {
    let path = temporary_source("empty", "");
    let first = run(&["context", path.to_str().unwrap(), "--format=json"]);
    let second = run(&["context", path.to_str().unwrap(), "--format=json"]);
    assert!(first.status.success());
    assert_eq!(first.stdout, second.stdout);
    let document: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(document["command"], "context");
    assert_eq!(document["success"], true);
    assert!(document["declarations"]["tables"].is_array());
}

#[test]
fn config_reports_effective_optional_features_without_secret_values() {
    let (project_directory, source_path) = temporary_project_source("config", "fn main() {}\n");
    fs::write(
        project_directory.join("zelyra.toml"),
        "[project]\nname = \"config-test\"\nversion = \"0.1.39\"\nzelyra = \"0.1\"\n\n[features]\napi = false\ncrud = false\n",
    )
    .unwrap();
    fs::write(
        project_directory.join(".env"),
        "ZELYRA_FEATURE_CRUD=true\nDATABASE_URL=mariadb://user:super-secret@localhost/app\n",
    )
    .unwrap();

    let output = run(&["config", source_path.to_str().unwrap(), "--format=json"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["schema_version"], "1");
    assert_eq!(document["command"], "config");
    assert_eq!(document["features"]["api"]["enabled"], false);
    assert_eq!(document["features"]["api"]["source"], "zelyra.toml");
    assert_eq!(document["features"]["crud"]["enabled"], true);
    assert_eq!(document["features"]["crud"]["source"], ".env");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("super-secret"));
    assert_eq!(document["project"]["config_file"], "zelyra.toml");
    fs::remove_dir_all(project_directory).unwrap();
}

#[test]
fn disabled_api_feature_rejects_api_source() {
    let (project_directory, source_path) = temporary_project_source(
        "disabled-api",
        "api GET \"/echo\" { handler echo input { value: String } output String } fn echo(value: String) -> String { return value } fn main() { }",
    );
    fs::write(
        project_directory.join("zelyra.toml"),
        "[project]\nname = \"feature-test\"\nversion = \"0.1.39\"\nzelyra = \"0.1\"\n\n[features]\napi = false\n",
    )
    .unwrap();
    let output = run(&["check", source_path.to_str().unwrap(), "--format=json"]);
    assert_eq!(output.status.code(), Some(1));
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["diagnostics"][0]["code"], "E-FEATURE-001");
    fs::remove_dir_all(project_directory).unwrap();
}

#[test]
fn edit_json_is_preview_only_by_default_and_applies_explicitly() {
    let (project_directory, source_path) =
        temporary_project_source("edit-source", "fn greet() { greet() }\nfn main() {}\n");
    let request_path = source_path.with_file_name("change.json");
    let request = format!(
        "{{\"schema_version\":\"1\",\"entry\":{},\"expected_source_fingerprint\":\"fnv1a64:860a2a12aa4d2f8d\",\"operations\":[{{\"kind\":\"rename\",\"symbol\":\"function\",\"from\":\"greet\",\"to\":\"welcome\"}}]}}",
        serde_json::to_string(source_path.to_str().unwrap()).unwrap()
    );
    fs::write(&request_path, request).expect("edit request should be written");

    let first = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    let second = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert!(
        first.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(first.stdout, second.stdout);
    assert!(first.stderr.is_empty());
    assert_eq!(
        fs::read_to_string(&source_path).unwrap(),
        "fn greet() { greet() }\nfn main() {}\n"
    );
    let document: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(document["command"], "edit");
    assert_eq!(document["success"], true);
    assert_eq!(document["preview"]["applied"], false);
    assert_eq!(
        document["preview"]["affected_files"],
        serde_json::json!(["main.zyl"])
    );
    assert_eq!(document["preview"]["changed_tokens"], 2);

    fs::write(&source_path, "fn changed() {}\nfn main() {}\n").expect("source should change");
    let stale = run(&[
        "edit",
        "--format=json",
        "--apply",
        request_path.to_str().unwrap(),
    ]);
    assert!(!stale.status.success());
    let stale_document: serde_json::Value = serde_json::from_slice(&stale.stdout).unwrap();
    assert_eq!(stale_document["diagnostics"][0]["code"], "E-EDIT-004");
    assert_eq!(
        fs::read_to_string(&source_path).unwrap(),
        "fn changed() {}\nfn main() {}\n"
    );

    fs::write(&source_path, "fn greet() { greet() }\nfn main() {}\n")
        .expect("source should be restored");
    let applied = run(&[
        "edit",
        "--format=json",
        "--apply",
        request_path.to_str().unwrap(),
    ]);
    assert!(applied.status.success());
    let applied_document: serde_json::Value = serde_json::from_slice(&applied.stdout).unwrap();
    assert_eq!(applied_document["preview"]["apply_requested"], true);
    assert_eq!(applied_document["preview"]["applied"], true);
    assert_eq!(
        fs::read_to_string(&source_path).unwrap(),
        "fn welcome() { welcome() }\nfn main() {}\n"
    );
    fs::remove_dir_all(project_directory).expect("temporary project should be removed");
}

#[test]
fn edit_preview_reports_capability_effects_of_renamed_functions_and_callers() {
    let source = "fn load_data() -> Int uses Database(read) { return sql<Int> { SELECT 1 } }\nfn page_data() -> Int uses Database(read) { return load_data() }\nfn main() {}\n";
    let (project_directory, source_path) = temporary_project_source("edit-effects", source);
    fs::write(
        project_directory.join("zelyra.toml"),
        "[project]\nname = \"edit-test\"\nversion = \"0.1.39\"\nzelyra = \"0.1\"\n\n[capabilities]\ndatabase_read = true\ndatabase_write = false\n",
    )
    .expect("project config should grant read-only SQL");
    let request_path = source_path.with_file_name("change.json");
    fs::write(
        &request_path,
        format!(
            "{{\"schema_version\":\"1\",\"entry\":{},\"operations\":[{{\"kind\":\"rename\",\"symbol\":\"function\",\"from\":\"load_data\",\"to\":\"read_data\"}}]}}",
            serde_json::to_string(source_path.to_str().unwrap()).unwrap()
        ),
    )
    .expect("edit request should be written");

    let output = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let effects = document["preview"]["affected_effects"].as_array().unwrap();
    assert!(effects.iter().any(|effect| {
        effect["function"] == "read_data"
            && effect["relation"] == "renamed_function"
            && effect["declared_capabilities"] == serde_json::json!(["Database(read)"])
    }));
    assert!(effects.iter().any(|effect| {
        effect["function"] == "page_data"
            && effect["relation"] == "caller_of_renamed_function"
            && effect["declared_capabilities"] == serde_json::json!(["Database(read)"])
    }));
    assert_eq!(fs::read_to_string(&source_path).unwrap(), source);
    fs::remove_dir_all(project_directory).expect("temporary project should be removed");
}

#[test]
fn edit_requires_a_versioned_request_and_project_local_zelyra_source() {
    let (project_directory, source_path) =
        temporary_project_source("edit-boundary", "fn greet() {}\n");
    let request_path = project_directory.join("change.json");
    fs::write(
        &request_path,
        format!(
            "{{\"entry\":{}}}",
            serde_json::to_string(source_path.to_str().unwrap()).unwrap()
        ),
    )
    .expect("unversioned edit request should be written");
    let unversioned = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert_eq!(unversioned.status.code(), Some(1));
    let unversioned_document: serde_json::Value =
        serde_json::from_slice(&unversioned.stdout).unwrap();
    assert_eq!(unversioned_document["diagnostics"][0]["code"], "E-EDIT-001");

    fs::write(
        &request_path,
        format!(
            "{{\"schema_version\":\"999\",\"entry\":{}}}",
            serde_json::to_string(source_path.to_str().unwrap()).unwrap()
        ),
    )
    .expect("unsupported edit request should be written");
    let unsupported = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert_eq!(unsupported.status.code(), Some(1));
    let unsupported_document: serde_json::Value =
        serde_json::from_slice(&unsupported.stdout).unwrap();
    assert_eq!(unsupported_document["diagnostics"][0]["code"], "E-EDIT-001");

    fs::write(
        &request_path,
        "{\"schema_version\":\"1\",\"entry\":\"/tmp/not-a-zelyra-project.zyl\",\"operations\":[]}",
    )
    .expect("external edit request should be written");
    let external = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert_eq!(external.status.code(), Some(1));
    let external_document: serde_json::Value = serde_json::from_slice(&external.stdout).unwrap();
    assert_eq!(external_document["diagnostics"][0]["code"], "E-EDIT-005");
    assert_eq!(fs::read_to_string(source_path).unwrap(), "fn greet() {}\n");
    fs::remove_dir_all(project_directory).expect("temporary project should be removed");
}

#[test]
fn edit_rejects_a_semantically_invalid_baseline_or_result() {
    let (project_directory, source_path) =
        temporary_project_source("edit-semantic", "fn greet() {}\nfn main() {}\n");
    let request_path = project_directory.join("change.json");
    let request = format!(
        "{{\"schema_version\":\"1\",\"entry\":{},\"operations\":[{{\"kind\":\"rename\",\"symbol\":\"function\",\"from\":\"main\",\"to\":\"greet\"}}]}}",
        serde_json::to_string(source_path.to_str().unwrap()).unwrap()
    );
    fs::write(&request_path, request).expect("semantic edit request should be written");
    let output = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["success"], false);
    assert_eq!(document["preview"]["available"], false);
    assert_eq!(document["diagnostics"][0]["code"], "E-NAME-001");
    assert_eq!(
        fs::read_to_string(&source_path).unwrap(),
        "fn greet() {}\nfn main() {}\n"
    );
    fs::remove_dir_all(project_directory).expect("temporary project should be removed");
}

#[test]
fn edit_does_not_rename_a_shadowing_local_binding() {
    let (project_directory, source_path) = temporary_project_source(
        "edit-scope",
        "fn greet() { greet = 1\n print(greet) }\nfn main() { greet() }\n",
    );
    let request_path = project_directory.join("change.json");
    let request = format!(
        "{{\"schema_version\":\"1\",\"entry\":{},\"operations\":[{{\"kind\":\"rename\",\"symbol\":\"function\",\"from\":\"greet\",\"to\":\"welcome\"}}]}}",
        serde_json::to_string(source_path.to_str().unwrap()).unwrap()
    );
    fs::write(&request_path, request).expect("scoped edit request should be written");
    let output = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["preview"]["changed_tokens"], 2);
    assert_eq!(document["preview"]["changes"].as_array().unwrap().len(), 2);
    assert_eq!(
        fs::read_to_string(&source_path).unwrap(),
        "fn greet() { greet = 1\n print(greet) }\nfn main() { greet() }\n"
    );
    fs::remove_dir_all(project_directory).expect("temporary project should be removed");
}

#[test]
fn edit_renames_type_and_record_references_through_the_cli() {
    let (project_directory, source_path) = temporary_project_source(
        "edit-types",
        "type CustomerId = Id\nstruct Customer { id: CustomerId }\nfn load(id: CustomerId) -> CustomerId { return id }\nfn make(id: CustomerId) -> Customer { return Customer { id: id } }\nfn main() {}\n",
    );
    let request_path = project_directory.join("change.json");
    let request = format!(
        "{{\"schema_version\":\"1\",\"entry\":{},\"operations\":[{{\"kind\":\"rename\",\"symbol\":\"type\",\"from\":\"CustomerId\",\"to\":\"ClientId\"}},{{\"kind\":\"rename\",\"symbol\":\"record\",\"from\":\"Customer\",\"to\":\"Client\"}}]}}",
        serde_json::to_string(source_path.to_str().unwrap()).unwrap()
    );
    fs::write(&request_path, request).expect("type edit request should be written");
    let output = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["preview"]["changed_tokens"], 8);
    assert_eq!(
        fs::read_to_string(&source_path).unwrap(),
        "type CustomerId = Id\nstruct Customer { id: CustomerId }\nfn load(id: CustomerId) -> CustomerId { return id }\nfn make(id: CustomerId) -> Customer { return Customer { id: id } }\nfn main() {}\n"
    );
    fs::remove_dir_all(project_directory).expect("temporary project should be removed");
}

#[test]
fn edit_renames_tables_views_forms_and_cruds_through_the_cli() {
    let machine_source = fs::read_to_string(example("machine_form.zyl"))
        .expect("machine form example should be readable");
    let (machine_directory, machine_path) =
        temporary_project_source("edit-resources", &machine_source);
    let machine_request_path = machine_directory.join("change.json");
    let machine_request = format!(
        "{{\"schema_version\":\"1\",\"entry\":{},\"operations\":[{{\"kind\":\"rename\",\"symbol\":\"table\",\"from\":\"machines\",\"to\":\"equipment\"}},{{\"kind\":\"rename\",\"symbol\":\"form\",\"from\":\"MachineCreate\",\"to\":\"MachineEditor\"}},{{\"kind\":\"rename\",\"symbol\":\"crud\",\"from\":\"Machine\",\"to\":\"MachineAdmin\"}}]}}",
        serde_json::to_string(machine_path.to_str().unwrap()).unwrap()
    );
    fs::write(&machine_request_path, machine_request)
        .expect("resource edit request should be written");
    let machine_output = run(&[
        "edit",
        "--format=json",
        machine_request_path.to_str().unwrap(),
    ]);
    assert!(
        machine_output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&machine_output.stdout),
        String::from_utf8_lossy(&machine_output.stderr)
    );
    let machine_document: serde_json::Value =
        serde_json::from_slice(&machine_output.stdout).unwrap();
    assert_eq!(machine_document["preview"]["changed_tokens"], 8);
    assert_eq!(fs::read_to_string(&machine_path).unwrap(), machine_source);
    fs::remove_dir_all(machine_directory).expect("machine project should be removed");

    let view_source =
        fs::read_to_string(example("views.zyl")).expect("view example should be readable");
    let (view_directory, view_path) = temporary_project_source("edit-view", &view_source);
    let view_request_path = view_directory.join("change.json");
    let view_request = format!(
        "{{\"schema_version\":\"1\",\"entry\":{},\"operations\":[{{\"kind\":\"rename\",\"symbol\":\"view\",\"from\":\"SiteShell\",\"to\":\"AppShell\"}}]}}",
        serde_json::to_string(view_path.to_str().unwrap()).unwrap()
    );
    fs::write(&view_request_path, view_request).expect("view edit request should be written");
    let view_output = run(&["edit", "--format=json", view_request_path.to_str().unwrap()]);
    assert!(
        view_output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&view_output.stdout),
        String::from_utf8_lossy(&view_output.stderr)
    );
    let view_document: serde_json::Value = serde_json::from_slice(&view_output.stdout).unwrap();
    assert_eq!(view_document["preview"]["changed_tokens"], 2);
    assert_eq!(fs::read_to_string(&view_path).unwrap(), view_source);
    fs::remove_dir_all(view_directory).expect("view project should be removed");
}

#[test]
fn edit_renames_view_components_in_declarations_and_html() {
    let (project_directory, source_path) = temporary_project_source(
        "edit-components",
        "component Badge { html { <strong>Ready</strong> } }\nview Shell { html { <Badge /><slot /> } }\n",
    );
    let request_path = project_directory.join("change.json");
    let request = format!(
        "{{\"schema_version\":\"1\",\"entry\":{},\"operations\":[{{\"kind\":\"rename\",\"symbol\":\"component\",\"from\":\"Badge\",\"to\":\"StatusBadge\"}}]}}",
        serde_json::to_string(source_path.to_str().unwrap()).unwrap()
    );
    fs::write(&request_path, request).expect("component edit request should be written");
    let output = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["preview"]["changed_tokens"], 2);
    assert!(document["preview"]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|change| change["from"] == "Badge" && change["to"] == "StatusBadge"));
    assert_eq!(
        fs::read_to_string(&source_path).unwrap(),
        "component Badge { html { <strong>Ready</strong> } }\nview Shell { html { <Badge /><slot /> } }\n"
    );
    fs::remove_dir_all(project_directory).expect("temporary project should be removed");
}

#[test]
fn context_exposes_safe_structural_project_information() {
    let auth_example = example("auth_crud_api.zyl");
    let output = run(&["context", auth_example.to_str().unwrap(), "--format=json"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["schema_version"], "1");
    assert_eq!(document["project"]["entry"], "examples/auth_crud_api.zyl");
    assert!(document["declarations"]["tables"]
        .as_array()
        .unwrap()
        .iter()
        .any(|table| table["name"] == "customers"));
    assert!(document["declarations"]["cruds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|crud| crud["table"] == "customers"));
    assert_eq!(
        document["database"]["connection_model"],
        "single-project-wide-connection"
    );
    assert_eq!(
        document["database"]["consumers"][0]["connection_environment"],
        "DATABASE_URL"
    );
    assert_eq!(document["database"]["credentials_included"], false);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("mariadb://"));
}

#[test]
fn reports_the_full_compiler_version() {
    for arguments in [["--version"], ["-V"], ["version"]] {
        let output = run(&arguments);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("zelyra {}\n", env!("CARGO_PKG_VERSION"))
        );
    }
}

#[test]
fn update_rejects_unknown_arguments_without_network_or_file_changes() {
    for arguments in [
        &["update", "--force"][..],
        &["update", "--check", "extra"][..],
    ] {
        let output = run(arguments);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("zelyra update [--check]"));
    }
}

#[test]
fn context_exposes_view_slot_structure_without_rendered_content() {
    let path = example("view_composition.zyl");
    let output = run(&["context", path.to_str().unwrap(), "--format=json"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let shell = document["declarations"]["views"]
        .as_array()
        .unwrap()
        .iter()
        .find(|view| view["name"] == "AppShell")
        .expect("AppShell should be present");
    assert_eq!(
        shell["slots"],
        serde_json::json!([
            { "name": "header", "fallback": true },
            { "name": "default", "fallback": false },
            { "name": "footer", "fallback": true }
        ])
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Describe intent"));
}

#[test]
fn context_exposes_crud_layout_slot_names_without_rendered_content() {
    let path = example("view_showcase.zyl");
    let output = run(&["context", path.to_str().unwrap(), "--format=json"]);
    let repeated = run(&["context", path.to_str().unwrap(), "--format=json"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, repeated.stdout);
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let customer_crud = document["declarations"]["cruds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|crud| crud["name"] == "Customer")
        .expect("Customer CRUD should be present");
    assert_eq!(customer_crud["layout"], "CustomerShell");
    assert_eq!(customer_crud["layout_slots"][0]["name"], "header");
    assert_eq!(customer_crud["layout_slots"][1]["name"], "intro");
    assert!(customer_crud["layout_slots"][0]["span"]["start"]["offset"]
        .as_u64()
        .is_some());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Manage customer records"));
}

#[test]
fn reports_unknown_crud_layout_slot_with_stable_json_diagnostic() {
    let source = r#"
        view Shell { html { <main><slot /></main> } }
        table customers { id: Id primary auto }
        crud Customer -> customers {
            layout: Shell
            slots { heading { html { <h1>Customers</h1> } } }
        }
    "#;
    let (project_directory, source_path) =
        temporary_project_source("invalid-crud-layout-slot", source);
    let output = run(&["check", source_path.to_str().unwrap(), "--format=json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["schema_version"], "1");
    assert_eq!(document["success"], false);
    assert_eq!(document["diagnostics"][0]["code"], "E-VIEW-031");
    fs::remove_dir_all(project_directory).expect("temporary project should be removed");
}

#[test]
fn impact_can_focus_on_a_known_node_with_versioned_json() {
    let path = example("auth_crud_api.zyl");
    let first = run(&[
        "impact",
        path.to_str().unwrap(),
        "--symbol",
        "table:customers",
        "--format=json",
    ]);
    let second = run(&[
        "impact",
        path.to_str().unwrap(),
        "--symbol",
        "table:customers",
        "--format=json",
    ]);
    assert!(first.status.success());
    assert!(first.stderr.is_empty());
    assert_eq!(first.stdout, second.stdout);
    let document: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(document["schema_version"], "1");
    assert_eq!(document["command"], "impact");
    assert_eq!(document["success"], true);
    assert_eq!(document["impact"]["focus"], "table:customers");
    assert!(document["impact"]["references"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference["to"] == "table:customers"));

    let unknown = run(&[
        "impact",
        path.to_str().unwrap(),
        "--symbol",
        "table:missing",
        "--format=json",
    ]);
    assert_eq!(unknown.status.code(), Some(1));
    assert!(unknown.stderr.is_empty());
    let unknown_document: serde_json::Value = serde_json::from_slice(&unknown.stdout).unwrap();
    assert_eq!(unknown_document["success"], false);
    assert_eq!(unknown_document["diagnostics"][0]["code"], "E-IMPACT-001");
}

#[test]
fn invalid_format_is_rejected_without_machine_output_claims() {
    let output = run(&["check", "../examples/fibonacci.zyl", "--format=xml"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("human` or `json"));
}

#[test]
fn typed_map_api_contract_drives_check_openapi_and_typescript() {
    let path = example("api_maps.zyl");
    let check = run(&["check", path.to_str().unwrap(), "--format=json"]);
    assert!(check.status.success());
    let check_document: serde_json::Value = serde_json::from_slice(&check.stdout).unwrap();
    assert_eq!(check_document["success"], true);

    let openapi = run(&["doc", path.to_str().unwrap(), "--openapi"]);
    assert!(openapi.status.success());
    let openapi_document: serde_json::Value = serde_json::from_slice(&openapi.stdout).unwrap();
    assert_eq!(
        openapi_document["paths"]["/settings"]["post"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["additionalProperties"]["type"],
        "integer"
    );

    let typescript = run(&["doc", path.to_str().unwrap(), "--typescript"]);
    assert!(typescript.status.success());
    let client = String::from_utf8(typescript.stdout).unwrap();
    assert!(client.contains("Record<string, number>"));
}

#[test]
fn typed_view_bindings_are_checked_as_machine_diagnostics() {
    let valid = run(&[
        "check",
        example("typed_views.zyl").to_str().unwrap(),
        "--format=json",
    ]);
    assert!(valid.status.success());
    let valid_document: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert_eq!(valid_document["success"], true);

    let invalid = run(&[
        "check",
        example("invalid_typed_views.zyl").to_str().unwrap(),
        "--format=json",
    ]);
    assert_eq!(invalid.status.code(), Some(1));
    let invalid_document: serde_json::Value = serde_json::from_slice(&invalid.stdout).unwrap();
    let codes = invalid_document["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"E-VIEW-010"));
    assert!(codes.contains(&"E-VIEW-015"));
}
