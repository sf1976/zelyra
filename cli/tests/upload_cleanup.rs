use std::process::Command;

fn zelyra(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .args(args)
        .output()
        .expect("run zelyra CLI")
}

#[test]
fn cleanup_requires_explicit_maintenance_acknowledgement_before_project_access() {
    let output = zelyra(&["files", "cleanup", "--apply"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--maintenance-window"));
}

#[test]
fn maintenance_acknowledgement_cannot_be_used_without_apply() {
    let output = zelyra(&["files", "cleanup", "--maintenance-window"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("only valid together with --apply"));
}

#[test]
fn cleanup_rejects_unbounded_or_zero_age_values() {
    for age in ["0", "3651", "unlimited"] {
        let output = zelyra(&["files", "cleanup", "--min-age-days", age]);
        assert_eq!(output.status.code(), Some(2), "age: {age}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("between 1 and 3650"));
    }
}

#[test]
fn cleanup_help_explains_preview_and_maintenance_window() {
    let output = zelyra(&["files", "cleanup", "--help"]);
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(
        help.contains("Preview only") || help.contains("standardmäßig nur"),
        "{help}"
    );
    assert!(help.contains("--maintenance-window"), "{help}");
}
