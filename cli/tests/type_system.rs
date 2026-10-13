use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_zelyra"))
}

fn example() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples/type_system")
}

fn temporary_copy() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    let destination = std::env::temp_dir().join(format!(
        "zelyra-type-system-{}-{unique}",
        std::process::id()
    ));
    copy_directory(&example(), &destination);
    destination
}

fn copy_directory(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("temporary project directory should be created");
    for entry in fs::read_dir(source).expect("example directory should be readable") {
        let source_path = entry.expect("example entry should be readable").path();
        let destination_path = destination.join(source_path.file_name().unwrap());
        if source_path.is_dir() {
            copy_directory(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).expect("example file should be copied");
        }
    }
}

fn cli(project: &Path, arguments: &[&str]) -> std::process::Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(project)
        .output()
        .expect("Zelyra CLI should run")
}

#[test]
fn type_coverage_program_passes_supported_tests_and_reports_expected_failures() {
    let project = temporary_copy();
    let check = cli(&project, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "type coverage program did not compile: {}",
        String::from_utf8_lossy(&check.stderr)
    );

    let run = cli(&project, &["run", "main.zyl"]);
    assert!(
        run.status.success(),
        "type coverage program failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let output = String::from_utf8_lossy(&run.stdout);
    let generic_example = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples/generic_functions.zyl");
    for source in [
        project.join("main.zyl"),
        project.join("src/generics.zyl"),
        generic_example,
    ] {
        let result = Command::new(binary())
            .args(["fmt", "--check"])
            .arg(&source)
            .current_dir(&project)
            .output()
            .expect("Zelyra formatter should run");
        assert!(
            result.status.success(),
            "formatter rejected {}: {}",
            source.display(),
            String::from_utf8_lossy(&result.stderr)
        );
    }
    for expected in [
        "PASS Int/UInt ranges",
        "PASS Decimal exact base-10 arithmetic and Float binary precision",
        "PASS Bool, Char, String, Unicode, Timestamp, Date, Time, Duration and Bytes",
        "PASS Arrays, nested collections",
        "PASS List, Tuple and Set construction, typing, operations and JSON",
        "PASS records, nested fields",
        "PASS Unit type, function return",
        "PASS Option, Result, records",
        "PASS calendar arithmetic, Timestamp ordering, Decimal quantization, scalar text formatting, exclusive ranges",
        "PASS unit and payload enums, equality, exhaustive match and JSON round trip",
        "PASS named typed function values, argument checking and indirect calls",
        "PASS classes, shared references, identity equality and checked field updates",
        "Report: 13 positive type groups; 27 expected compiler errors and 21 expected runtime errors",
        "Type matrix: 19 supported entries, 6 partial entries, 0 unavailable entries",
        "Unexpected failures: 0 when the CLI integration test passes",
        "alle ausführbaren Typprüfungen bestanden",
    ] {
        assert!(
            output.contains(expected),
            "missing {expected:?} in {output}"
        );
    }

    for (fixture, expected) in [
        ("type_mismatch.zyl", "type mismatch"),
        ("mixed_array.zyl", "type mismatch"),
        ("non_exhaustive_match.zyl", "non-exhaustive match"),
        ("non_exhaustive_enum.zyl", "non-exhaustive match"),
        ("invalid_operation.zyl", "numeric operands"),
        (
            "unsupported_type.zyl",
            "Tuple requires at least two item types",
        ),
        ("unsupported_list.zyl", "type mismatch"),
        (
            "unsupported_set.zyl",
            "Set elements must be scalar comparable types",
        ),
        (
            "generic_explicit_mismatch.zyl",
            "type mismatch: expected `Int`, found `String`",
        ),
        ("generic_uninferred.zyl", "E-TYPE-GENERIC-001"),
        ("generic_wrong_arity.zyl", "E-TYPE-GENERIC-001"),
        (
            "enum_payload_type.zyl",
            "type mismatch: expected `Int`, found `String`",
        ),
        (
            "enum_payload_arity.zyl",
            "expects 2 payload value(s), found 1",
        ),
        ("enum_non_exhaustive_payload.zyl", "non-exhaustive match"),
        ("enum_unknown_payload_type.zyl", "unknown type `Missing`"),
        (
            "function_value_argument_type.zyl",
            "type mismatch: expected `Int`, found `String`",
        ),
        (
            "function_value_generic_reference.zyl",
            "cannot be stored as a function value yet",
        ),
        ("function_value_missing_capability.zyl", "Clock"),
        ("unsupported_format_text.zyl", "E-TYPE-FORMAT-001"),
        (
            "function_value_equality.zyl",
            "function values cannot be compared",
        ),
        (
            "class_field_type.zyl",
            "type mismatch: expected `Int`, found `String`",
        ),
        (
            "record_field_mutation.zyl",
            "field assignment requires a class instance",
        ),
        (
            "class_json_encode.zyl",
            "class references cannot be serialized directly",
        ),
        (
            "class_json_decode.zyl",
            "json_decode does not support class types",
        ),
        (
            "class_nested_json_encode.zyl",
            "class references cannot be serialized directly",
        ),
        (
            "class_unknown_field.zyl",
            "unknown field `missing` in class `Counter`",
        ),
        (
            "class_record_with.zyl",
            "record_with cannot copy or update a class reference",
        ),
    ] {
        let path = format!("expected/compiler/{fixture}");
        let result = cli(&project, &["check", &path]);
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(!result.status.success(), "{fixture} unexpectedly compiled");
        assert!(stderr.contains(expected), "{fixture}: {stderr}");
        if matches!(
            fixture,
            "class_field_type.zyl"
                | "class_unknown_field.zyl"
                | "record_field_mutation.zyl"
                | "class_record_with.zyl"
        ) {
            assert!(
                stderr.contains("E-TYPE-CLASS-001"),
                "{fixture} should use the stable class diagnostic: {stderr}"
            );
        }
    }

    for (fixture, expected) in [
        ("int_divide_by_zero.zyl", "division by zero"),
        ("int_overflow.zyl", "Int overflow"),
        ("int_negation_overflow.zyl", "Int overflow"),
        ("uint_overflow.zyl", "UInt overflow"),
        ("uint_underflow.zyl", "UInt underflow"),
        ("uint_divide_by_zero.zyl", "division by zero"),
        ("array_index.zyl", "out of bounds"),
        ("non_finite_json.zyl", "cannot serialize non-finite Float"),
        (
            "invalid_date_json.zyl",
            "expected Date in YYYY-MM-DD format",
        ),
        (
            "invalid_bytes_json.zyl",
            "each byte to be an integer from 0 to 255",
        ),
        ("invalid_time.zyl", "expected HH:MM:SS"),
        ("invalid_bytes.zyl", "values must be between 0 and 255"),
        ("decimal_precision.zyl", "at most 9 fractional digits"),
        (
            "decimal_exponent.zyl",
            "base-10 value with at most 9 fractional digits",
        ),
        ("decimal_json_number.zyl", "expected Decimal string"),
        ("invalid_tuple_json.zyl", "expected 2 tuple values, found 1"),
        (
            "non_string_map_json.zyl",
            "supports only Map<String, Value>",
        ),
        (
            "unsupported_result_json.zyl",
            "does not support target type `Result<Int, String>`",
        ),
        (
            "function_value_json.zyl",
            "cannot serialize a function value",
        ),
        (
            "class_mutation_in_parallel.zyl",
            "class field mutation is not allowed inside a `parallel` block",
        ),
        (
            "class_reference_cycle.zyl",
            "class field update would create a reference cycle",
        ),
    ] {
        let path = format!("expected/runtime/{fixture}");
        let result = cli(&project, &["run", &path]);
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(!result.status.success(), "{fixture} unexpectedly succeeded");
        assert!(stderr.contains(expected), "{fixture}: {stderr}");
    }

    fs::remove_dir_all(project).expect("temporary project should be removed");
}
