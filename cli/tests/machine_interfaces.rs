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

#[test]
fn valid_check_json_is_a_stable_machine_document() {
    let path = example("fibonacci.zyl");
    let first = run(&["check", path.to_str().unwrap(), "--format=json"]);
    let second = run(&["check", path.to_str().unwrap(), "--format=json"]);
    assert!(first.status.success());
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
