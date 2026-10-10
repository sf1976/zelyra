use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

fn binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_zelyra"))
}

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests/language-compatibility/0.1")
}

fn temporary_project(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "zelyra-language-compatibility-{}-{name}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).expect("temporary project should be created");
    fs::write(
        directory.join("zelyra.toml"),
        "[project]\nname = \"compatibility-fixture\"\nversion = \"0.1.0\"\nzelyra = \"0.1\"\n\n[capabilities]\nconsole = true\ndatabase = false\nnetwork = false\n",
    )
    .expect("fixture project configuration should be written");
    directory
}

fn run(arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .output()
        .expect("Zelyra CLI should run")
}

fn source_files(directory: &Path) -> Vec<PathBuf> {
    let mut files: Vec<_> = fs::read_dir(directory)
        .expect("corpus directory should be readable")
        .map(|entry| entry.expect("corpus entry should be readable").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "zyl"))
        .collect();
    files.sort();
    files
}

fn assert_required_fixtures(category: &str, required: &[&str]) {
    let directory = corpus().join(category);
    let available: Vec<_> = source_files(&directory)
        .into_iter()
        .filter_map(|path| path.file_name()?.to_str().map(str::to_owned))
        .collect();
    for fixture in required {
        assert!(
            available.iter().any(|name| name == fixture),
            "frozen 0.1 compatibility fixture {category}/{fixture} is missing"
        );
    }
}

#[test]
fn accepted_line_0_1_sources_check_and_format_idempotently() {
    assert_required_fixtures(
        "accepted",
        &[
            "basic.zyl",
            "typed_function.zyl",
            "unicode_source.zyl",
            "nested_records_arrays.zyl",
            "typed_maps.zyl",
        ],
    );
    for fixture in source_files(&corpus().join("accepted")) {
        let project = temporary_project(fixture.file_stem().unwrap().to_str().unwrap());
        let source = project.join("main.zyl");
        fs::copy(&fixture, &source).expect("accepted fixture should be copied");
        let source_arg = source.to_str().unwrap();

        let check = run(&["check", source_arg]);
        assert!(
            check.status.success(),
            "{} failed check: {}",
            fixture.display(),
            String::from_utf8_lossy(&check.stderr)
        );

        let first_format = run(&["fmt", source_arg]);
        assert!(
            first_format.status.success(),
            "{} failed formatting: {}",
            fixture.display(),
            String::from_utf8_lossy(&first_format.stderr)
        );
        let first_result = fs::read(&source).expect("formatted source should be readable");
        let second_format = run(&["fmt", source_arg]);
        assert!(second_format.status.success());
        assert_eq!(
            fs::read(&source).expect("second formatted source should be readable"),
            first_result,
            "formatter output for {} should be idempotent",
            fixture.display()
        );
        let format_check = run(&["fmt", source_arg, "--check"]);
        assert!(
            format_check.status.success(),
            "{} failed fmt --check: {}",
            fixture.display(),
            String::from_utf8_lossy(&format_check.stderr)
        );
        fs::remove_dir_all(project).expect("temporary project should be removed");
    }
}

#[test]
fn rejected_line_0_1_sources_keep_their_diagnostic_codes() {
    assert_required_fixtures(
        "rejected",
        &[
            "disabled_network_capability.zyl",
            "incompatible_operands.zyl",
            "incomplete_function.zyl",
            "unknown_variable.zyl",
        ],
    );
    for fixture in source_files(&corpus().join("rejected")) {
        let project = temporary_project(fixture.file_stem().unwrap().to_str().unwrap());
        let source = project.join("main.zyl");
        fs::copy(&fixture, &source).expect("rejected fixture should be copied");
        let expected_code = fs::read_to_string(fixture.with_extension("code"))
            .expect("rejected fixture should have a diagnostic code");
        let output = run(&["check", source.to_str().unwrap()]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "{} unexpectedly passed",
            fixture.display()
        );
        assert!(
            stderr.contains(expected_code.trim()),
            "{} expected {}, got: {stderr}",
            fixture.display(),
            expected_code.trim()
        );
        fs::remove_dir_all(project).expect("temporary project should be removed");
    }
}

#[test]
fn line_0_1_runtime_fixtures_preserve_their_output() {
    assert_required_fixtures(
        "runtime",
        &[
            "arithmetic.zyl",
            "print_value.zyl",
            "unicode_output.zyl",
            "nested_records_arrays.zyl",
            "typed_maps.zyl",
        ],
    );
    for fixture in source_files(&corpus().join("runtime")) {
        let project = temporary_project(fixture.file_stem().unwrap().to_str().unwrap());
        let source = project.join("main.zyl");
        fs::copy(&fixture, &source).expect("runtime fixture should be copied");
        let output = run(&["run", source.to_str().unwrap()]);
        let expected = fs::read(fixture.with_extension("out"))
            .expect("runtime fixture should have expected output");
        assert!(
            output.status.success(),
            "{} failed to run: {}",
            fixture.display(),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout,
            expected,
            "runtime output for {}",
            fixture.display()
        );
        fs::remove_dir_all(project).expect("temporary project should be removed");
    }
}
