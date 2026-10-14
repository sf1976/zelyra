#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

fn tutorial_with_compose(
    plugin_version: Option<&str>,
    standalone_version: &str,
) -> (String, String, String) {
    let directory = std::env::temp_dir().join(format!(
        "zelyra-compose-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let bin = directory.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let log = directory.join("commands.log");
    for (name, version, version_args) in [
        ("docker", plugin_version, "compose version --short"),
        (
            "docker-compose",
            Some(standalone_version),
            "version --short",
        ),
    ] {
        let version_command = version.map_or("exit 1".to_owned(), |value| {
            value.strip_prefix("stderr:").map_or_else(
                || format!("echo '{value}'\nexit 0"),
                |value| format!("echo '{value}' >&2\nexit 0"),
            )
        });
        let script = format!(
            "#!/bin/sh\nprintf '%s %s\\n' '{name}' \"$*\" >> \"$COMPOSE_TEST_LOG\"\nif [ \"$*\" = '{version_args}' ]; then\n{version_command}\nfi\necho 'selected {name} for startup' >&2\nexit 1\n"
        );
        let path = bin.join(name);
        fs::write(&path, script).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let project = directory.join("invoice");
    let output = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .args(["--tutorial", "invoice"])
        .arg(&project)
        .env("PATH", &bin)
        .env("COMPOSE_TEST_LOG", &log)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(project.join("main.zyl").is_file());
    assert!(project.join(".env").is_file());
    let compose = fs::read_to_string(project.join("docker-compose.mariadb.yml")).unwrap();
    let project_name = compose
        .lines()
        .next()
        .unwrap()
        .strip_prefix("name: zelyra-invoice-")
        .expect("invoice tutorial Compose project should have a unique name")
        .to_owned();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let calls = fs::read_to_string(log).unwrap();
    fs::remove_dir_all(directory).unwrap();
    (stderr, calls, project_name)
}

#[test]
fn python_compose_v1_is_rejected_before_starting_containers() {
    let (stderr, calls, _) = tutorial_with_compose(None, "1.29.2");
    assert!(stderr.contains("v2 or newer is required"), "{stderr}");
    assert!(stderr.contains("ContainerConfig"));
    assert!(!stderr.contains("usermod"));
    assert!(!calls.contains(" up "), "{calls}");
}

#[test]
fn plugin_is_preferred_over_python_compose() {
    let (stderr, calls, _) = tutorial_with_compose(Some("stderr:2.40"), "1.29.2");
    assert!(stderr.contains("selected docker for startup"), "{stderr}");
    assert!(
        !calls
            .lines()
            .any(|line| line.starts_with("docker-compose ")),
        "{calls}"
    );
}

#[test]
fn modern_standalone_compose_is_supported() {
    let (stderr, calls, _) = tutorial_with_compose(None, "v5.6.0");
    assert!(
        stderr.contains("selected docker-compose for startup"),
        "{stderr}"
    );
    assert!(calls.contains(" up -d mariadb"), "{calls}");
}

#[test]
fn unknown_plugin_version_does_not_bypass_version_requirement() {
    let (stderr, calls, _) = tutorial_with_compose(Some("unknown"), "1.29.2");
    assert!(stderr.contains("v2 or newer is required"), "{stderr}");
    assert!(!calls.contains(" up "), "{calls}");
}

#[test]
fn each_new_invoice_tutorial_gets_a_distinct_database_volume_namespace() {
    let (_, _, first) = tutorial_with_compose(None, "1.29.2");
    let (_, _, second) = tutorial_with_compose(None, "1.29.2");
    assert_ne!(first, second);
}
