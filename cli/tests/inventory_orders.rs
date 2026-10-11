use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

fn binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_zelyra"))
}

fn example() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples/inventory_orders")
}

fn temporary_copy() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    let destination = std::env::temp_dir().join(format!(
        "zelyra-inventory-orders-{}-{unique}",
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
fn inventory_orders_example_checks_tests_and_persists_cancelled_orders() {
    let project = temporary_copy();
    let check = cli(&project, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "project check failed: {}",
        String::from_utf8_lossy(&check.stderr)
    );

    let business_tests = cli(&project, &["run", "tests.zyl"]);
    assert!(
        business_tests.status.success(),
        "business tests failed: {}",
        String::from_utf8_lossy(&business_tests.stderr)
    );
    let test_output = String::from_utf8_lossy(&business_tests.stdout);
    assert!(test_output.contains("PASS Preisberechnung"));
    assert!(test_output.contains("PASS Bestandspruefung"));
    assert!(test_output.contains("PASS Stornierung"));
    assert!(test_output.contains("\n4\n0\n"));

    let mut application = Command::new(binary())
        .args(["run", "main.zyl"])
        .current_dir(&project)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("inventory application should start");
    application
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(b"1\n2\n101\nHammer\n1250\n5\nWerkzeug\n1\n6\n101\nHammer Pro\n1250\n5\nWerkzeug\n2\n2\nAnna Beispiel\nanna@example.test\n3\n1\n101\n2\n\n10\n1\n7\n101\n4\n2\n5\n0\n")
        .expect("console scenario should be sent");
    let output = application
        .wait_with_output()
        .expect("inventory application should finish");
    assert!(
        output.status.success(),
        "application run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output_text = String::from_utf8_lossy(&output.stdout);
    assert!(output_text.contains("Bestellung storniert; Waren wurden wieder eingelagert."));
    assert!(
        output_text.contains("Artikel ist in einer Bestellung referenziert und bleibt erhalten.")
    );
    assert!(output_text.contains('T') && output_text.contains('Z'));
    assert!(output_text.contains("Umsatz in Cent:\n0"));

    let state: serde_json::Value = serde_json::from_slice(
        &fs::read(project.join("data/state.json")).expect("state should be saved"),
    )
    .expect("saved state should be valid JSON");
    assert_eq!(state["articles"][0]["stock"], 5);
    assert_eq!(state["articles"][0]["name"], "Hammer Pro");
    assert_eq!(state["orders"][0]["status"], "cancelled");
    assert!(state["activities"].as_array().unwrap().len() >= 6);

    fs::remove_dir_all(project).expect("temporary project should be removed");
}

#[test]
fn inventory_orders_reports_invalid_input_and_preserves_corrupt_state() {
    let project = temporary_copy();
    let state_path = project.join("data/state.json");
    fs::write(&state_path, "not valid json").expect("corrupt state fixture should be written");
    let mut application = Command::new(binary())
        .args(["run", "main.zyl"])
        .current_dir(&project)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("inventory application should start");
    application
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(b"1\n2\nnot-a-number\nHammer\n1250\n5\nWerkzeug\n0\n")
        .expect("invalid-input scenario should be sent");
    let output = application
        .wait_with_output()
        .expect("inventory application should finish");
    assert!(
        output.status.success(),
        "application run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output_text = String::from_utf8_lossy(&output.stdout);
    assert!(output_text.contains("Die JSON-Datei ist ungueltig:"));
    assert!(output_text.contains("Artikelnummer muss eine Ganzzahl sein."));
    assert_eq!(
        fs::read_to_string(&state_path).expect("corrupt source file should remain readable"),
        "not valid json"
    );
    fs::remove_dir_all(project).expect("temporary project should be removed");
}
