use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
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
        "[project]\nname = \"edit-test\"\nversion = \"0.1.38\"\nzelyra = \"0.1\"\n\n[capabilities]\ndatabase = false\nnetwork = false\n",
    )
    .expect("temporary project config should be written");
    let source_path = directory.join("main.zyl");
    fs::write(&source_path, source).expect("temporary project source should be written");
    (directory, source_path)
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
fn invalid_check_json_keeps_human_logs_off_stdout() {
    let invalid_sql = example("invalid_sql.zyl");
    let output = run(&["check", invalid_sql.to_str().unwrap(), "--format", "json"]);
    assert!(!output.status.success());
    assert!(output.stderr.is_empty());
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["schema_version"], "1");
    assert_eq!(document["success"], false);
    let diagnostics = document["diagnostics"].as_array().unwrap();
    assert!(diagnostics.len() >= 2);
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic["code"].as_str().is_some()
            && diagnostic["severity"] == "error"
            && diagnostic["file"].as_str().is_some()
            && diagnostic["span"]["start"]["offset"].is_number()
            && diagnostic["span"]["end"]["offset"].is_number()
    }));
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
    assert!(!String::from_utf8_lossy(&output.stdout).contains("DATABASE_URL"));
}

#[test]
fn invalid_format_is_rejected_without_machine_output_claims() {
    let output = run(&["check", "../examples/fibonacci.zyl", "--format=xml"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("human` or `json"));
}
