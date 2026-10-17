use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

fn project(files: &[(&str, &str)]) -> PathBuf {
    let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "zelyra-cli-module-test-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).unwrap();
    for (relative, source) in files {
        let path = directory.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }
    directory
}

fn run(directory: &Path, arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .current_dir(directory)
        .args(arguments)
        .output()
        .unwrap()
}

fn run_with_environment(
    directory: &Path,
    arguments: &[&str],
    key: &str,
    value: Option<&str>,
) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_zelyra"));
    command
        .current_dir(directory)
        .args(arguments)
        .env_remove(key);
    if let Some(value) = value {
        command.env(key, value);
    }
    command.output().unwrap()
}

#[test]
fn verify_json_uses_the_imported_module_source_for_contract_locations() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/contracts.zyl\" as contracts\nfn main() {}\n",
        ),
        (
            "src/contracts.zyl",
            "\n\npub fn proven() requires { 1 < 2 } {}\n",
        ),
    ]);
    let output = run(&directory, &["verify", "main.zyl", "--json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let entries: Value = serde_json::from_slice(&output.stdout).unwrap();
    let proven = entries
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["status"] == "PROVEN")
        .unwrap();
    assert_eq!(proven["location"]["file"], "src/contracts.zyl");
    assert_eq!(proven["location"]["start"]["line"], 3);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn local_packages_are_version_checked_locked_and_importable_offline() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"pkg:math\" as math\nfn main() { print(math::add(2, 3)) }\n",
        ),
        (
            "zelyra.toml",
            "[package]\nname = \"invoice_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.math]\npath = \"packages/math\"\nversion = \"^1.2\"\n",
        ),
        (
            "packages/math/zelyra.toml",
            "[package]\nname = \"math\"\nversion = \"1.2.3\"\nentry = \"src/lib.zyl\"\n",
        ),
        (
            "packages/math/src/lib.zyl",
            "import \"src/arithmetic.zyl\" as arithmetic\npub fn add(left: Int, right: Int) -> Int { return arithmetic::sum(left, right) }\n",
        ),
        (
            "packages/math/src/arithmetic.zyl",
            "pub fn sum(left: Int, right: Int) -> Int { return left + right }\n",
        ),
    ]);

    let lock = run(&directory, &["package", "lock", "main.zyl"]);
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );
    let lock_contents = fs::read_to_string(directory.join("zelyra.lock")).unwrap();
    assert!(lock_contents.contains("name = \"math\""));
    assert!(lock_contents.contains("checksum = \"sha256:"));

    let check = run(&directory, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let executed = run(&directory, &["run", "main.zyl"]);
    assert!(
        executed.status.success(),
        "{}",
        String::from_utf8_lossy(&executed.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&executed.stdout).trim(), "5");
    let bytecode = run(&directory, &["run", "main.zyl", "--backend", "bytecode"]);
    assert!(
        bytecode.status.success(),
        "{}",
        String::from_utf8_lossy(&bytecode.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&bytecode.stdout).trim(), "5");

    fs::write(
        directory.join("packages/math/src/arithmetic.zyl"),
        "pub fn sum(left: Int, right: Int) -> Int { return left + right + 2 }\n",
    )
    .unwrap();
    let stale = run(&directory, &["check", "main.zyl"]);
    assert!(!stale.status.success());
    assert!(String::from_utf8_lossy(&stale.stderr).contains("E-PKG-002"));
    let stale_run = run(&directory, &["run", "main.zyl"]);
    assert!(!stale_run.status.success());
    assert!(String::from_utf8_lossy(&stale_run.stderr).contains("E-PKG-002"));
    let updated = run(&directory, &["package", "lock", "main.zyl", "--update"]);
    assert!(
        updated.status.success(),
        "{}",
        String::from_utf8_lossy(&updated.stderr)
    );
    let checked_lock = run(&directory, &["package", "lock", "main.zyl", "--check"]);
    assert!(checked_lock.status.success());
    let refreshed_check = run(&directory, &["check", "main.zyl"]);
    assert!(
        refreshed_check.status.success(),
        "{}",
        String::from_utf8_lossy(&refreshed_check.stderr)
    );
    let refreshed_run = run(&directory, &["run", "main.zyl"]);
    assert!(
        refreshed_run.status.success(),
        "{}",
        String::from_utf8_lossy(&refreshed_run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&refreshed_run.stdout).trim(), "7");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn named_function_values_cross_package_boundaries_on_ast_and_bytecode_backends() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"pkg:math\" as math\nfn double(value: Int) -> Int { return value * 2 }\nfn main() { print(math::sum_mapped([1, 2, 3], &double)) }\n",
        ),
        (
            "zelyra.toml",
            "[package]\nname = \"callback_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.math]\npath = \"packages/math\"\nversion = \"^1.2\"\n",
        ),
        (
            "packages/math/zelyra.toml",
            "[package]\nname = \"math\"\nversion = \"1.2.3\"\nentry = \"src/lib.zyl\"\n",
        ),
        (
            "packages/math/src/lib.zyl",
            "pub fn sum_mapped(values: Int[], callback: Function<(Int) -> Int>) -> Int {\n    mapped = values |> map(callback)\n    return mapped |> reduce(0, |sum, value| sum + value)\n}\n",
        ),
    ]);

    let locked = run(&directory, &["package", "lock", "main.zyl"]);
    assert!(
        locked.status.success(),
        "{}",
        String::from_utf8_lossy(&locked.stderr)
    );
    let checked = run(&directory, &["check", "main.zyl"]);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    for args in [
        vec!["run", "main.zyl"],
        vec!["run", "main.zyl", "--backend", "bytecode"],
    ] {
        let output = run(&directory, &args);
        assert!(
            output.status.success(),
            "command {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "12");
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn imported_modules_contribute_namespaced_commands_and_schedules() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/tasks.zyl\" as tasks\nfn main() { }\n",
        ),
        (
            "src/tasks.zyl",
            "command refresh { handler: refresh_data }\nschedule refresh_hourly { command: refresh cron: \"0 * * * *\" }\nfn refresh_data() { print(\"refreshed\") }\n",
        ),
    ]);
    let check = run(&directory, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let commands = run(&directory, &["task", "list", "main.zyl", "--json"]);
    assert!(
        commands.status.success(),
        "{}",
        String::from_utf8_lossy(&commands.stderr)
    );
    let command_json: Value = serde_json::from_slice(&commands.stdout).unwrap();
    assert_eq!(command_json["commands"].as_array().unwrap().len(), 1);
    let namespaced_command = command_json["commands"][0]["name"].as_str().unwrap();
    assert!(namespaced_command.ends_with("::refresh"));
    let task_run = run(&directory, &["task", "run", "main.zyl", namespaced_command]);
    assert!(
        task_run.status.success(),
        "{}",
        String::from_utf8_lossy(&task_run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&task_run.stdout).trim(),
        "refreshed"
    );
    let schedules = run(&directory, &["schedule", "list", "main.zyl", "--json"]);
    assert!(
        schedules.status.success(),
        "{}",
        String::from_utf8_lossy(&schedules.stderr)
    );
    let schedule_json: Value = serde_json::from_slice(&schedules.stdout).unwrap();
    assert_eq!(schedule_json["schedules"].as_array().unwrap().len(), 1);
    assert!(schedule_json["schedules"][0]["command"]
        .as_str()
        .unwrap()
        .ends_with("::refresh"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn task_arguments_are_checked_and_schedule_export_is_reviewable() {
    let directory = project(&[(
        "main.zyl",
        r#"command greet {
    handler: greet
    description: "Greet a user"
}
command heartbeat { handler: heartbeat }
schedule hourly_heartbeat {
    command: heartbeat
    cron: "0 * * * *"
    timezone: "Europe/Berlin"
}
fn greet(name: String) { print(name) }
fn heartbeat() { print("alive") }
fn main() { }
"#,
    )]);

    let listed = run(&directory, &["task", "list", "main.zyl", "--json"]);
    assert!(
        listed.status.success(),
        "{}",
        String::from_utf8_lossy(&listed.stderr)
    );
    let listed: Value = serde_json::from_slice(&listed.stdout).unwrap();
    let greet = listed["commands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|command| command["name"] == "greet")
        .unwrap();
    assert_eq!(greet["description"], "Greet a user");
    assert_eq!(greet["parameters"][0]["name"], "name");
    assert_eq!(greet["parameters"][0]["type"], "String");

    let executed = run(
        &directory,
        &["task", "run", "main.zyl", "greet", "--args", r#"["Ada"]"#],
    );
    assert!(
        executed.status.success(),
        "{}",
        String::from_utf8_lossy(&executed.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&executed.stdout).trim(), "Ada");

    let wrong_type = run(
        &directory,
        &["task", "run", "main.zyl", "greet", "--args", "[42]"],
    );
    assert!(!wrong_type.status.success());
    assert!(String::from_utf8_lossy(&wrong_type.stderr).contains("E-CLI-001"));

    let preview = run(
        &directory,
        &["schedule", "export", "main.zyl", "--format", "cron"],
    );
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let preview = String::from_utf8_lossy(&preview.stdout);
    assert!(preview.contains("CRON_TZ=Europe/Berlin"));
    assert!(preview.contains("0 * * * *"));
    assert!(preview.contains("--entry 'heartbeat'"));

    fs::create_dir_all(directory.join("deploy")).unwrap();
    let written = run(
        &directory,
        &[
            "schedule",
            "export",
            "main.zyl",
            "--format",
            "cron",
            "--output",
            "deploy/zelyra.cron",
        ],
    );
    assert!(
        written.status.success(),
        "{}",
        String::from_utf8_lossy(&written.stderr)
    );
    assert!(directory.join("deploy/zelyra.cron").is_file());

    let traversal = run(
        &directory,
        &[
            "schedule",
            "export",
            "main.zyl",
            "--format",
            "cron",
            "--output",
            "../outside.cron",
        ],
    );
    assert!(!traversal.status.success());
    assert!(String::from_utf8_lossy(&traversal.stderr).contains("E-TASK-004"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn local_package_graph_supports_two_versions_of_one_package_identity() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"pkg:legacy_api\" as legacy_api\nimport \"pkg:modern_api\" as modern_api\nfn main() { print(legacy_api::value() + modern_api::value()) }\n",
        ),
        (
            "zelyra.toml",
            "[package]\nname = \"multi_version_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.legacy_api]\npath = \"packages/api-v1\"\nversion = \"^1.0\"\n\n[dependencies.modern_api]\npath = \"packages/api-v2\"\nversion = \"^2.0\"\n",
        ),
        (
            "packages/api-v1/zelyra.toml",
            "[package]\nname = \"legacy_api\"\nversion = \"1.0.0\"\nentry = \"src/lib.zyl\"\n\n[dependencies.math]\npath = \"../math-v1\"\nversion = \"^1.0\"\n",
        ),
        (
            "packages/api-v1/src/lib.zyl",
            "import \"pkg:math\" as math\npub fn value() -> Int { return math::value() }\n",
        ),
        (
            "packages/api-v2/zelyra.toml",
            "[package]\nname = \"modern_api\"\nversion = \"2.0.0\"\nentry = \"src/lib.zyl\"\n\n[dependencies.math]\npath = \"../math-v2\"\nversion = \"^2.0\"\n",
        ),
        (
            "packages/api-v2/src/lib.zyl",
            "import \"pkg:math\" as math\npub fn value() -> Int { return math::value() }\n",
        ),
        (
            "packages/math-v1/zelyra.toml",
            "[package]\nname = \"math\"\nversion = \"1.4.2\"\nentry = \"src/lib.zyl\"\n",
        ),
        (
            "packages/math-v1/src/lib.zyl",
            "pub fn value() -> Int { return 20 }\n",
        ),
        (
            "packages/math-v2/zelyra.toml",
            "[package]\nname = \"math\"\nversion = \"2.1.0\"\nentry = \"src/lib.zyl\"\n",
        ),
        (
            "packages/math-v2/src/lib.zyl",
            "pub fn value() -> Int { return 22 }\n",
        ),
    ]);

    let lock = run(&directory, &["package", "lock", "main.zyl"]);
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );
    let lock_contents = fs::read_to_string(directory.join("zelyra.lock")).unwrap();
    assert!(
        lock_contents.contains("name = \"math\"\nversion = \"1.4.2\"\npath = \"packages/math-v1\"")
    );
    assert!(
        lock_contents.contains("name = \"math\"\nversion = \"2.1.0\"\npath = \"packages/math-v2\"")
    );

    let check = run(&directory, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    for backend_args in [
        vec!["run", "main.zyl"],
        vec!["run", "main.zyl", "--backend", "bytecode"],
    ] {
        let execution = run(&directory, &backend_args);
        assert!(
            execution.status.success(),
            "{}",
            String::from_utf8_lossy(&execution.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&execution.stdout).trim(), "42");
    }

    let bundle = directory.with_extension("multi-version-bundle");
    let bundle_arg = bundle.to_string_lossy().into_owned();
    let bundled = run(
        &directory,
        &[
            "module",
            "bundle",
            "main.zyl",
            "main.zyl",
            "--output",
            &bundle_arg,
        ],
    );
    assert!(
        bundled.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&bundled.stdout),
        String::from_utf8_lossy(&bundled.stderr)
    );
    let bundled_lock = fs::read_to_string(bundle.join("zelyra.lock")).unwrap();
    assert!(bundled_lock.contains("version = \"1.4.2\""));
    assert!(bundled_lock.contains("version = \"2.1.0\""));
    let bundled_check = run(&bundle, &["check", "main.zyl"]);
    assert!(
        bundled_check.status.success(),
        "{}",
        String::from_utf8_lossy(&bundled_check.stderr)
    );
    let bundled_execution = run(&bundle, &["run", "main.zyl"]);
    assert!(bundled_execution.status.success());
    assert_eq!(
        String::from_utf8_lossy(&bundled_execution.stdout).trim(),
        "42"
    );

    fs::remove_dir_all(directory).unwrap();
    fs::remove_dir_all(bundle).unwrap();
}

#[test]
fn package_effect_ceiling_is_locked_and_does_not_grant_runtime_capabilities() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"pkg:network_client\" as client\nfn main() uses Network { client::fetch() }\n",
        ),
        (
            "zelyra.toml",
            "[package]\nname = \"effect_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.network_client]\npath = \"packages/client\"\nversion = \"^1.0\"\n\n[capabilities]\nnetwork = true\n",
        ),
        (
            "packages/client/zelyra.toml",
            "[package]\nname = \"network_client\"\nversion = \"1.0.0\"\nentry = \"src/lib.zyl\"\neffects = [\"Network\"]\n",
        ),
        (
            "packages/client/src/lib.zyl",
            "pub fn fetch() uses Network {}\n",
        ),
    ]);

    let lock = run(&directory, &["package", "lock", "main.zyl"]);
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );
    let allowed = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(
        allowed.status.success(),
        "{}",
        String::from_utf8_lossy(&allowed.stdout)
    );

    let package_manifest = directory.join("packages/client/zelyra.toml");
    let declared = fs::read_to_string(&package_manifest).unwrap();
    fs::write(
        &package_manifest,
        declared.replace("effects = [\"Network\"]", "effects = []"),
    )
    .unwrap();
    let stale = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(!stale.status.success());
    assert!(String::from_utf8_lossy(&stale.stdout).contains("E-PKG-002"));

    let update = run(&directory, &["package", "lock", "main.zyl", "--update"]);
    assert!(update.status.success());
    let denied_by_package = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(!denied_by_package.status.success());
    let diagnostics: Value = serde_json::from_slice(&denied_by_package.stdout).unwrap();
    assert!(diagnostics["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item["code"] == "E-PKG-003"
                && item["file"] == "packages/client/src/lib.zyl"
                && item["message"].as_str().is_some_and(|message| {
                    message.contains("Network") && message.contains("package.effects")
                })
        }));

    fs::write(&package_manifest, declared).unwrap();
    let update = run(&directory, &["package", "lock", "main.zyl", "--update"]);
    assert!(update.status.success());
    fs::write(
        directory.join("zelyra.toml"),
        "[package]\nname = \"effect_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.network_client]\npath = \"packages/client\"\nversion = \"^1.0\"\n\n[capabilities]\nnetwork = false\n",
    )
    .unwrap();
    let update = run(&directory, &["package", "lock", "main.zyl", "--update"]);
    assert!(update.status.success());
    let denied_by_project = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(!denied_by_project.status.success());
    let diagnostics: Value = serde_json::from_slice(&denied_by_project.stdout).unwrap();
    assert!(diagnostics["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item["code"] == "E-CAP-001" && item["file"] == "packages/client/src/lib.zyl"
        }));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn package_dependency_alias_can_differ_from_package_identity() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"pkg:money_core\" as money\nfn main() { print(money::add(12, 50)) }\n",
        ),
        (
            "zelyra.toml",
            "[package]\nname = \"invoice_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.money_core]\npath = \"packages/money\"\nversion = \"^2.1\"\n",
        ),
        (
            "packages/money/zelyra.toml",
            "[package]\nname = \"money\"\nversion = \"2.1.4\"\nentry = \"src/lib.zyl\"\n",
        ),
        (
            "packages/money/src/lib.zyl",
            "pub fn add(euros: Int, fraction: Int) -> Int { return euros * 100 + fraction }\n",
        ),
    ]);

    let lock = run(&directory, &["package", "lock", "main.zyl"]);
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );
    let lock_contents = fs::read_to_string(directory.join("zelyra.lock")).unwrap();
    assert!(lock_contents.contains(r#"dependencies = ["money_core=packages/money"]"#));

    let check = run(&directory, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let executed = run(&directory, &["run", "main.zyl"]);
    assert!(
        executed.status.success(),
        "{}",
        String::from_utf8_lossy(&executed.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&executed.stdout).trim(), "1250");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn package_lock_rejects_an_incompatible_zelyra_requirement_before_writing() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"pkg:math\" as math\nfn main() { print(math::value()) }\n",
        ),
        (
            "zelyra.toml",
            "[package]\nname = \"compatibility_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.math]\npath = \"packages/math\"\nversion = \"^1.0\"\n",
        ),
        (
            "packages/math/zelyra.toml",
            "[package]\nname = \"math\"\nversion = \"1.0.0\"\nentry = \"src/lib.zyl\"\nzelyra = \">=99.0.0\"\n",
        ),
        (
            "packages/math/src/lib.zyl",
            "pub fn value() -> Int { return 42 }\n",
        ),
    ]);
    let result = run(&directory, &["package", "lock", "main.zyl"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("E-PKG-001"), "{stderr}");
    assert!(stderr.contains("requires Zelyra `>=99.0.0`"), "{stderr}");
    assert!(!directory.join("zelyra.lock").exists());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_init_creates_a_checkable_local_package_and_never_overwrites() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"pkg:small_tools\" as tools\nfn main() { print(tools::module_name()) }\n",
        ),
        (
            "zelyra.toml",
            "[package]\nname = \"module_init_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.small_tools]\npath = \"packages/small_tools\"\nversion = \"^0.4.2\"\n",
        ),
    ]);
    let created = run(
        &directory,
        &[
            "module",
            "init",
            "packages/small_tools",
            "--name",
            "small_tools",
            "--version",
            "0.4.2",
        ],
    );
    assert!(
        created.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&created.stdout),
        String::from_utf8_lossy(&created.stderr)
    );
    let package_manifest =
        fs::read_to_string(directory.join("packages/small_tools/zelyra.toml")).unwrap();
    assert!(package_manifest.contains("zelyra = \">=1.1.0, <1.2.0\""));
    assert!(package_manifest.contains("effects = []"));
    assert!(directory.join("packages/small_tools/README.md").is_file());

    let lock = run(&directory, &["package", "lock", "."]);
    assert!(
        lock.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&lock.stdout),
        String::from_utf8_lossy(&lock.stderr)
    );
    let checked = run(&directory, &["check", "main.zyl"]);
    assert!(
        checked.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&checked.stdout),
        String::from_utf8_lossy(&checked.stderr)
    );
    let executed = run(&directory, &["run", "main.zyl"]);
    assert!(
        executed.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&executed.stdout),
        String::from_utf8_lossy(&executed.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&executed.stdout).trim(),
        "small_tools"
    );

    let source_before =
        fs::read_to_string(directory.join("packages/small_tools/src/lib.zyl")).unwrap();
    let duplicate = run(
        &directory,
        &[
            "module",
            "init",
            "packages/small_tools",
            "--name",
            "replacement",
        ],
    );
    assert!(!duplicate.status.success());
    assert!(String::from_utf8_lossy(&duplicate.stderr).contains("already exists"));
    assert_eq!(
        fs::read_to_string(directory.join("packages/small_tools/src/lib.zyl")).unwrap(),
        source_before
    );

    let invalid = run(
        &directory,
        &["module", "init", "packages/bad_name", "--name", "BadName"],
    );
    assert!(!invalid.status.success());
    assert!(!directory.join("packages/bad_name").exists());

    let duplicate_version = run(
        &directory,
        &[
            "module",
            "init",
            "packages/duplicate_version",
            "--name",
            "duplicate_version",
            "--version",
            "0.1.0",
            "--version",
            "0.2.0",
        ],
    );
    assert!(!duplicate_version.status.success());
    assert!(String::from_utf8_lossy(&duplicate_version.stderr).contains("at most once"));
    assert!(!directory.join("packages/duplicate_version").exists());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn package_imports_cannot_reach_sources_excluded_from_lock_checksums() {
    for excluded_dir in [".git", "target", ".zelyra"] {
        let directory = project(&[
            (
                "main.zyl",
                "import \"pkg:math\" as math\nfn main() { print(math::value()) }\n",
            ),
            (
                "zelyra.toml",
                "[package]\nname = \"invoice_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.math]\npath = \"packages/math\"\nversion = \"1.0.0\"\n",
            ),
            (
                "packages/math/zelyra.toml",
                "[package]\nname = \"math\"\nversion = \"1.0.0\"\nentry = \"src/lib.zyl\"\n",
            ),
            (
                "packages/math/src/lib.zyl",
                &format!(
                    "import \"{excluded_dir}/hidden.zyl\" as hidden\npub fn value() -> Int {{ return hidden::value() }}\n"
                ),
            ),
            (
                &format!("packages/math/{excluded_dir}/hidden.zyl"),
                "pub fn value() -> Int { return 42 }\n",
            ),
        ]);

        let lock = run(&directory, &["package", "lock", "main.zyl"]);
        assert!(
            lock.status.success(),
            "{}",
            String::from_utf8_lossy(&lock.stderr)
        );

        let check = run(&directory, &["check", "main.zyl"]);
        assert!(
            !check.status.success(),
            "{excluded_dir} import was accepted"
        );
        let diagnostics = String::from_utf8_lossy(&check.stderr);
        assert!(diagnostics.contains("E-MOD-001"), "{diagnostics}");
        assert!(
            diagnostics.contains("excluded from package checksums"),
            "{diagnostics}"
        );

        fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn package_entry_cannot_come_from_a_directory_excluded_from_lock_checksums() {
    for excluded_dir in [".git", "target", ".zelyra"] {
        let directory = project(&[
            (
                "main.zyl",
                "import \"pkg:math\" as math\nfn main() { print(math::add(2, 3)) }\n",
            ),
            (
                "zelyra.toml",
                "[package]\nname = \"invoice_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.math]\npath = \"packages/math\"\nversion = \"1.0.0\"\n",
            ),
            (
                "packages/math/zelyra.toml",
                &format!(
                    "[package]\nname = \"math\"\nversion = \"1.0.0\"\nentry = \"{excluded_dir}/lib.zyl\"\n"
                ),
            ),
            (
                &format!("packages/math/{excluded_dir}/lib.zyl"),
                "pub fn add(left: Int, right: Int) -> Int { return left + right }\n",
            ),
        ]);

        let lock = run(&directory, &["package", "lock", "main.zyl"]);
        assert!(!lock.status.success(), "{excluded_dir} entry was accepted");
        let diagnostics = String::from_utf8_lossy(&lock.stderr);
        assert!(
            diagnostics.contains("excluded from package checksums"),
            "{diagnostics}"
        );

        fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn local_packages_can_depend_on_a_sibling_package_without_leaving_the_workspace() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"pkg:math\" as math\nfn main() { print(math::double(21)) }\n",
        ),
        (
            "zelyra.toml",
            "[package]\nname = \"sibling_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.math]\npath = \"packages/math\"\nversion = \"^1.0\"\n",
        ),
        (
            "packages/math/zelyra.toml",
            "[package]\nname = \"math\"\nversion = \"1.1.0\"\nentry = \"src/lib.zyl\"\n\n[dependencies.base]\npath = \"../base\"\nversion = \"^2.0\"\n",
        ),
        (
            "packages/math/src/lib.zyl",
            "import \"pkg:base\" as base\npub fn double(value: Int) -> Int { return base::twice(value) }\n",
        ),
        (
            "packages/base/zelyra.toml",
            "[package]\nname = \"base\"\nversion = \"2.3.0\"\nentry = \"src/lib.zyl\"\n",
        ),
        (
            "packages/base/src/lib.zyl",
            "pub fn twice(value: Int) -> Int { return value * 2 }\n",
        ),
    ]);

    let lock = run(&directory, &["package", "lock", "main.zyl"]);
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );
    let lock_contents = fs::read_to_string(directory.join("zelyra.lock")).unwrap();
    assert!(lock_contents.contains(r#"path = "packages/base""#));
    assert!(lock_contents.contains(r#"dependencies = ["base=packages/base"]"#));
    let check = run(&directory, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let executed = run(&directory, &["run", "main.zyl"]);
    assert!(
        executed.status.success(),
        "{}",
        String::from_utf8_lossy(&executed.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&executed.stdout).trim(), "42");
    fs::remove_dir_all(directory).unwrap();
}

fn directory_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn collect(root: &Path, directory: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                collect(root, &path, files);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(relative, fs::read(path).unwrap());
            }
        }
    }

    let mut files = BTreeMap::new();
    collect(root, root, &mut files);
    files
}

fn get_until_response(address: &str, request: &[u8]) -> String {
    let socket: std::net::SocketAddr = address.parse().unwrap();
    for _ in 0..75 {
        if let Ok(mut stream) = TcpStream::connect_timeout(&socket, Duration::from_millis(100)) {
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            if stream.write_all(request).is_ok() {
                let mut response = String::new();
                match stream.read_to_string(&mut response) {
                    Ok(_) if !response.is_empty() => return response,
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {}
                    Err(error) => panic!("failed to read HTTP response: {error}"),
                }
            }
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    String::new()
}

fn run_test_database_sql(database_url: &str, sql: &str) -> std::process::Output {
    let rest = database_url
        .strip_prefix("mariadb://")
        .or_else(|| database_url.strip_prefix("mysql://"))
        .expect("test database URL must be MariaDB");
    let (authority, database) = rest.split_once('/').unwrap();
    let (credentials, host_port) = authority.split_once('@').unwrap();
    let (user, password) = credentials.split_once(':').unwrap_or((credentials, ""));
    let (host, port) = host_port.split_once(':').unwrap_or((host_port, "3306"));
    Command::new("mariadb")
        .args([
            "--protocol=tcp",
            "--host",
            host,
            "--port",
            port,
            "--user",
            user,
            "--database",
            database,
            "--execute",
            sql,
        ])
        .env("MYSQL_PWD", password)
        .output()
        .expect("MariaDB client is required for the module integration test")
}

fn drop_test_database(database_url: &str) {
    let rest = database_url
        .strip_prefix("mariadb://")
        .or_else(|| database_url.strip_prefix("mysql://"))
        .expect("test database URL must be MariaDB");
    let (_, database) = rest.split_once('/').unwrap();
    let output = run_test_database_sql(
        database_url,
        &format!("DROP DATABASE IF EXISTS `{database}`"),
    );
    assert!(
        output.status.success(),
        "could not clean up the integration database: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn check_and_run_compile_imported_public_functions() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/math.zyl\" as math\nfn main() { print(math::add(2, 3)) }\n",
        ),
        (
            "src/math.zyl",
            "fn double(n: Int) -> Int { return n * 2 }\npub fn add(a: Int, b: Int) -> Int { return double(a + b) }\n",
        ),
    ]);
    let check = run(&directory, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let check_again = run(&directory, &["check", "main.zyl"]);
    assert_eq!(check.stdout, check_again.stdout);
    let build = run(&directory, &["build", "main.zyl"]);
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert_eq!(check.stdout, build.stdout);
    let run_result = run(&directory, &["run", "main.zyl"]);
    assert!(
        run_result.status.success(),
        "{}",
        String::from_utf8_lossy(&run_result.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run_result.stdout).trim(), "10");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn checked_in_multifile_example_checks_and_runs_without_external_services() {
    let example = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples/modules");

    let check = run(&example, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );

    let run_result = run(&example, &["run", "main.zyl"]);
    assert!(
        run_result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&run_result.stdout),
        String::from_utf8_lossy(&run_result.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run_result.stdout).trim(), "25");
}

#[test]
fn checked_in_customer_orders_module_example_checks_and_plans_its_boundaries() {
    let example = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples/customer_orders_modules");

    let check = run(&example, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );

    let plan = run(
        &example,
        &["module", "plan", "main.zyl", "src/reporting.zyl"],
    );
    assert!(
        plan.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&plan.stdout),
        String::from_utf8_lossy(&plan.stderr)
    );
    let document: Value = serde_json::from_slice(&plan.stdout).unwrap();
    let source_files = document["plan"]["source_files"].as_array().unwrap();
    for source in [
        "src/customers.zyl",
        "src/database.zyl",
        "src/orders.zyl",
        "src/reporting.zyl",
    ] {
        assert!(source_files.iter().any(|path| path == source), "{source}");
    }
    assert_eq!(document["plan"]["complete_deployment"], false);
    assert_eq!(document["plan"]["deployment_readiness"]["ready"], false);
    assert!(document["plan"]["unresolved_references"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn checked_in_business_package_example_locks_checks_and_exports_database_provider() {
    let example = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples/business_modules");
    let lock = run(&example, &["package", "lock", "main.zyl", "--check"]);
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );
    let check = run(&example, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );

    let output = std::env::temp_dir().join(format!(
        "zelyra-checked-business-package-{}-{}",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    let output_arg = output.to_string_lossy().into_owned();
    let bundled = run(
        &example,
        &[
            "module",
            "bundle",
            "main.zyl",
            "pkg:billing",
            "--output",
            &output_arg,
            "--docker",
            "--compiler-ref",
            &"a".repeat(40),
        ],
    );
    assert!(
        bundled.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&bundled.stdout),
        String::from_utf8_lossy(&bundled.stderr)
    );
    for path in [
        "Dockerfile",
        "docker-compose.yml",
        "packages/billing/src/lib.zyl",
        "packages/storage/src/lib.zyl",
    ] {
        assert!(output.join(path).is_file(), "missing bundled {path}");
    }
    let bundled_billing_manifest =
        fs::read_to_string(output.join("packages/billing/zelyra.toml")).unwrap();
    assert!(bundled_billing_manifest.contains("zelyra = \">=1.1.0, <1.2.0\""));
    let bundled_lock = run(&output, &["package", "lock", "main.zyl", "--check"]);
    assert!(
        bundled_lock.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&bundled_lock.stdout),
        String::from_utf8_lossy(&bundled_lock.stderr)
    );
    let bundled_check = run(&output, &["check", "main.zyl"]);
    assert!(
        bundled_check.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&bundled_check.stdout),
        String::from_utf8_lossy(&bundled_check.stderr)
    );
    let manifest: Value =
        serde_json::from_slice(&fs::read(output.join("zelyra.bundle.json")).unwrap()).unwrap();
    assert_eq!(
        manifest["database"]["connection_environment"],
        "ZELYRA_DATABASE_MAIN_URL"
    );
    assert_eq!(manifest["complete_deployment"], false);
    fs::remove_dir_all(output).unwrap();
}

#[test]
fn module_bundle_materializes_a_checked_source_closure_without_secrets() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/pages.zyl\" as pages\nimport \"src/ui.zyl\" as ui\nfn main() { print(\"original entry\") }\n",
        ),
        (
            "src/pages.zyl",
            "import \"src/ui.zyl\" as ui\npage \"/invoices\" { view: AppShell html { <h1>Bundled invoices</h1> } }\n",
        ),
        (
            "src/ui.zyl",
            "pub view AppShell { html { <html><body><slot /></body></html> } }\n",
        ),
        ("zelyra.toml", "[project]\nname = \"module-bundle-test\"\n"),
        ("zelyra.theme.css", ":root { --test-color: blue; }\n"),
        ("locales/en.json", "{}\n"),
        ("locales/de.json", "{}\n"),
        (".env", "DATABASE_URL=mariadb://must-not-be-copied\n"),
        (".env.example", "DATABASE_URL=mariadb://replace-me\n"),
    ]);
    let bundle = directory.with_extension("invoice-bundle");
    let bundle_arg = bundle.to_string_lossy().into_owned();
    let result = run(
        &directory,
        &[
            "module",
            "bundle",
            "main.zyl",
            "page:/invoices",
            "--output",
            &bundle_arg,
        ],
    );
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(bundle.join("src/pages.zyl").is_file());
    assert!(bundle.join("src/ui.zyl").is_file());
    assert!(bundle.join("zelyra.toml").is_file());
    assert!(bundle.join("zelyra.theme.css").is_file());
    assert!(bundle.join("locales/en.json").is_file());
    assert!(bundle.join("locales/de.json").is_file());
    assert!(!bundle.join(".env").exists());
    assert!(!bundle.join(".env.example").exists());
    assert!(!bundle.join("Dockerfile").exists());

    let manifest: Value =
        serde_json::from_slice(&fs::read(bundle.join("zelyra.bundle.json")).unwrap()).unwrap();
    assert_eq!(manifest["format_version"], 1);
    assert_eq!(manifest["kind"], "experimental-source-bundle");
    assert_eq!(manifest["source_closure_complete"], true);
    assert_eq!(
        manifest["source_closure_evidence"]["unresolved_references"],
        0
    );
    assert_eq!(
        manifest["source_closure_evidence"]["compiler_check"],
        "passed_before_publication"
    );
    assert_eq!(manifest["complete_deployment"], false);
    let check = run(&bundle, &["check", "main.zyl", "--format=json"]);
    assert!(
        check.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );

    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let address = format!("127.0.0.1:{port}");
    let mut server = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .current_dir(&bundle)
        .args(["serve", "main.zyl", &address])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let response = get_until_response(
        &address,
        b"GET /invoices HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    let _ = server.kill();
    let server_output = server.wait_with_output().unwrap();
    assert!(
        response.contains("200 OK") && response.contains("Bundled invoices"),
        "{response}\nserver stderr: {}",
        String::from_utf8_lossy(&server_output.stderr)
    );

    let original_readme = fs::read(bundle.join("README.md")).unwrap();
    let repeat = run(
        &directory,
        &[
            "module",
            "bundle",
            "main.zyl",
            "page:/invoices",
            "--output",
            &bundle_arg,
        ],
    );
    assert!(!repeat.status.success());
    assert_eq!(fs::read(bundle.join("README.md")).unwrap(), original_readme);
    let nested_output = directory.join("nested-bundle");
    let nested_output_arg = nested_output.to_string_lossy().into_owned();
    let nested = run(
        &directory,
        &[
            "module",
            "bundle",
            "main.zyl",
            "page:/invoices",
            "--output",
            &nested_output_arg,
        ],
    );
    assert!(!nested.status.success());
    assert!(!nested_output.exists());
    let _ = fs::remove_dir_all(bundle);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_bundle_rejects_multiple_database_definitions_before_writing() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/invoices.zyl\" as invoices\nimport \"src/db_primary.zyl\" as storage\nimport \"src/db_reporting.zyl\" as analytics\nfn main() {}\n",
        ),
        (
            "src/invoices.zyl",
            "table invoices { id: Id primary auto number: String(30) required }\ncrud Invoice -> invoices\n",
        ),
        (
            "src/db_primary.zyl",
            "database main { engine: mariadb database: \"billing\" }\n",
        ),
        (
            "src/db_reporting.zyl",
            "database reporting { engine: mariadb database: \"reports\" }\n",
        ),
    ]);
    let bundle = directory.with_extension("multiple-database-bundle");
    let bundle_arg = bundle.to_string_lossy().into_owned();
    let result = run(
        &directory,
        &[
            "module",
            "bundle",
            "main.zyl",
            "crud:Invoice",
            "--output",
            &bundle_arg,
        ],
    );
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr)
        .contains("single project-wide DATABASE_URL connection"));
    assert!(!bundle.exists());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_bundle_can_generate_a_pinned_experimental_docker_package() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/pages.zyl\" as pages\nfn main() {}\n",
        ),
        (
            "src/pages.zyl",
            "page \"/invoices\" { html { <h1>Bundled invoices</h1> } }\n",
        ),
        (
            ".env",
            "DATABASE_URL=mariadb://must-not-be-copied\nPRIVATE_TOKEN=do-not-copy\n",
        ),
        (
            ".env.example",
            "DATABASE_URL=mariadb://example-secret-must-not-be-copied\n",
        ),
    ]);
    let bundle = directory.with_extension("docker-bundle");
    let bundle_arg = bundle.to_string_lossy().into_owned();
    let compiler_ref = "a".repeat(40);
    let result = run(
        &directory,
        &[
            "module",
            "bundle",
            "main.zyl",
            "src/pages.zyl",
            "--output",
            &bundle_arg,
            "--docker",
            "--compiler-ref",
            &compiler_ref,
        ],
    );
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    for file in [
        "Dockerfile",
        "docker-compose.yml",
        ".dockerignore",
        ".env.example",
    ] {
        assert!(bundle.join(file).is_file(), "missing generated {file}");
    }
    assert!(!bundle.join(".env").exists());
    let dockerfile = fs::read_to_string(bundle.join("Dockerfile")).unwrap();
    assert!(dockerfile.contains(&format!("ARG ZELYRA_REF={compiler_ref}")));
    assert!(dockerfile.contains("git -C /zelyra fetch --depth=1 origin \"$ZELYRA_REF\""));
    assert!(dockerfile.contains("RUN /out/bin/zelyra check main.zyl"));
    assert!(dockerfile.contains("COPY . ./"));
    let bundle_readme = fs::read_to_string(bundle.join("README.md")).unwrap();
    assert!(bundle_readme.contains("incompatible pins fail the image build"));
    let env_example = fs::read_to_string(bundle.join(".env.example")).unwrap();
    assert!(env_example.contains("DATABASE_URL="));
    assert!(env_example.contains("ZELYRA_DB_CONNECT_TIMEOUT_SECS=10"));
    assert!(env_example.contains("ZELYRA_DB_QUERY_TIMEOUT_SECS=30"));
    assert!(env_example.contains("ZELYRA_DB_POOL_MAX_SIZE=8"));
    assert!(env_example.contains("ZELYRA_DB_POOL_WAIT_TIMEOUT_SECS=10"));
    assert!(!env_example.contains("must-not-be-copied"));
    assert!(!env_example.contains("example-secret"));
    assert!(!env_example.contains("ZELYRA_SMTP_"));
    assert!(!env_example.contains("ZELYRA_RESET_DELIVERY_KEY"));

    let manifest: Value =
        serde_json::from_slice(&fs::read(bundle.join("zelyra.bundle.json")).unwrap()).unwrap();
    assert_eq!(manifest["format_version"], 1);
    assert_eq!(manifest["kind"], "experimental-docker-source-package");
    assert_eq!(manifest["docker"]["compiler_commit"], compiler_ref);
    assert_eq!(manifest["source_closure_complete"], true);
    assert_eq!(
        manifest["source_closure_evidence"]["unresolved_references"],
        0
    );
    assert_eq!(
        manifest["source_closure_evidence"]["compiler_check"],
        "passed_before_publication"
    );
    assert_eq!(manifest["complete_deployment"], false);
    let check = run(&bundle, &["check", "main.zyl", "--format=json"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stdout)
    );

    let invalid_bundle = bundle.with_extension("invalid-docker-bundle");
    let invalid_arg = invalid_bundle.to_string_lossy().into_owned();
    let invalid = run(
        &directory,
        &[
            "module",
            "bundle",
            "main.zyl",
            "src/pages.zyl",
            "--output",
            &invalid_arg,
            "--docker",
            "--compiler-ref",
            "main; touch /tmp/unsafe",
        ],
    );
    assert!(!invalid.status.success());
    assert!(!invalid_bundle.exists());
    fs::remove_dir_all(bundle).unwrap();
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_bundle_dry_run_lists_every_output_without_creating_the_target() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/invoices.zyl\" as invoices\nfn main() {}\n",
        ),
        (
            "src/invoices.zyl",
            "table invoices { id: Id primary auto number: String(30) required }\ncrud Invoice -> invoices\n",
        ),
    ]);
    let bundle = directory.with_extension("bundle-plan");
    let bundle_arg = bundle.to_string_lossy().into_owned();
    let arguments = [
        "module",
        "bundle",
        "main.zyl",
        "src/invoices.zyl",
        "--output",
        &bundle_arg,
        "--dry-run",
    ];
    let first = run(&directory, &arguments);
    let second = run(&directory, &arguments);
    assert!(
        first.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(first.stdout, second.stdout);
    assert!(!bundle.exists());
    let document: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(document["command"], "module bundle");
    assert_eq!(document["success"], true);
    assert_eq!(document["diagnostics"], serde_json::json!([]));
    assert_eq!(document["plan"]["writes_performed"], false);
    assert_eq!(document["plan"]["secrets_included"], false);
    assert_eq!(document["plan"]["source_closure_complete"], true);
    assert_eq!(
        document["plan"]["source_closure_evidence"]["unresolved_references"],
        0
    );
    assert_eq!(
        document["plan"]["source_closure_evidence"]["compiler_check"],
        "passed_in_staging"
    );
    assert_eq!(document["plan"]["complete_deployment"], false);
    let files = document["plan"]["files"].as_array().unwrap();
    for expected in [
        "main.zyl",
        "src/invoices.zyl",
        "README.md",
        "zelyra.bundle.json",
    ] {
        assert!(
            files.iter().any(|file| file["relative_path"] == expected),
            "planned files omitted {expected}: {files:#?}"
        );
    }
    assert!(files.iter().all(|file| {
        file["destination"]
            .as_str()
            .is_some_and(|path| path.starts_with(bundle_arg.as_str()))
    }));

    let docker_bundle = directory.with_extension("docker-bundle-plan");
    let docker_bundle_arg = docker_bundle.to_string_lossy().into_owned();
    let compiler_ref = "b".repeat(40);
    let docker_plan = run(
        &directory,
        &[
            "module",
            "bundle",
            "main.zyl",
            "src/invoices.zyl",
            "--output",
            &docker_bundle_arg,
            "--docker",
            "--compiler-ref",
            &compiler_ref,
            "--dry-run",
        ],
    );
    assert!(
        docker_plan.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&docker_plan.stdout),
        String::from_utf8_lossy(&docker_plan.stderr)
    );
    assert!(!docker_bundle.exists());
    let docker_document: Value = serde_json::from_slice(&docker_plan.stdout).unwrap();
    assert_eq!(docker_document["plan"]["docker"], true);
    assert_eq!(docker_document["plan"]["compiler_commit"], compiler_ref);
    let docker_files = docker_document["plan"]["files"].as_array().unwrap();
    for expected in [
        "Dockerfile",
        "docker-compose.yml",
        ".env.example",
        ".dockerignore",
    ] {
        assert!(
            docker_files
                .iter()
                .any(|file| file["relative_path"] == expected),
            "Docker plan omitted {expected}: {docker_files:#?}"
        );
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn repeated_module_bundles_are_byte_identical_with_locale_files() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/pages.zyl\" as pages\nfn main() {}\n",
        ),
        (
            "src/pages.zyl",
            "page \"/invoices\" { html { <h1>Invoices</h1> } }\n",
        ),
        ("locales/zz.json", "{\"title\":\"Last\"}\n"),
        ("locales/en.json", "{\"title\":\"Invoices\"}\n"),
        ("locales/de.json", "{\"title\":\"Rechnungen\"}\n"),
    ]);
    let first_bundle = directory.with_extension("deterministic-bundle-a");
    let second_bundle = directory.with_extension("deterministic-bundle-b");
    let first_path = first_bundle.to_string_lossy().into_owned();
    let second_path = second_bundle.to_string_lossy().into_owned();

    for output in [&first_path, &second_path] {
        let result = run(
            &directory,
            &[
                "module",
                "bundle",
                "main.zyl",
                "src/pages.zyl",
                "--output",
                output,
            ],
        );
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }

    let first = directory_snapshot(&first_bundle);
    let second = directory_snapshot(&second_bundle);
    assert_eq!(first, second);
    let manifest: Value = serde_json::from_slice(&first["zelyra.bundle.json"]).unwrap();
    assert_eq!(
        manifest["support_files"],
        serde_json::json!(["locales/de.json", "locales/en.json", "locales/zz.json"])
    );

    fs::remove_dir_all(first_bundle).unwrap();
    fs::remove_dir_all(second_bundle).unwrap();
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn independent_database_using_bundles_get_separate_connection_configuration() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/database.zyl\" as storage\nimport \"src/invoices.zyl\" as invoices\nimport \"src/inventory.zyl\" as inventory\nfn main() {}\n",
        ),
        (
            "src/database.zyl",
            "database main { engine: mariadb database: \"business\" }\n",
        ),
        (
            "src/invoices.zyl",
            "import \"src/database.zyl\" as storage\ntable invoices { id: Id primary auto number: String(30) required }\ncrud Invoice -> invoices\n",
        ),
        (
            "src/inventory.zyl",
            "import \"src/database.zyl\" as storage\ntable inventory { id: Id primary auto sku: String(30) required }\ncrud Inventory -> inventory\n",
        ),
    ]);

    let compiler_ref = "a".repeat(40);
    let deployments = [
        (
            "crud:Invoice",
            "invoices",
            "src/invoices.zyl",
            "src/inventory.zyl",
        ),
        (
            "crud:Inventory",
            "inventory",
            "src/inventory.zyl",
            "src/invoices.zyl",
        ),
    ];
    for (resource, name, included_module, excluded_module) in deployments {
        let bundle = directory.with_extension(format!("{name}-docker-bundle"));
        let bundle_arg = bundle.to_string_lossy().into_owned();
        let result = run(
            &directory,
            &[
                "module",
                "bundle",
                "main.zyl",
                resource,
                "--output",
                &bundle_arg,
                "--docker",
                "--compiler-ref",
                &compiler_ref,
            ],
        );
        assert!(
            result.status.success(),
            "{resource}: {}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(bundle.join("src/database.zyl").is_file());
        assert!(bundle.join(included_module).is_file());
        assert!(!bundle.join(excluded_module).exists());
        assert!(!bundle.join(".env").exists());

        let env_example = fs::read_to_string(bundle.join(".env.example")).unwrap();
        assert!(env_example.contains("ZELYRA_DATABASE_MAIN_URL="));
        assert!(env_example
            .lines()
            .any(|line| line == "ZELYRA_DATABASE_MAIN_URL="));
        let dockerfile = fs::read_to_string(bundle.join("Dockerfile")).unwrap();
        assert!(dockerfile.contains("ca-certificates mariadb-client"));
        let manifest: Value =
            serde_json::from_slice(&fs::read(bundle.join("zelyra.bundle.json")).unwrap()).unwrap();
        assert_eq!(manifest["database"]["required"], true);
        assert_eq!(
            manifest["database"]["connection_environment"],
            "ZELYRA_DATABASE_MAIN_URL"
        );
        assert_eq!(
            manifest["database"]["connection_model"],
            "single-project-wide-connection"
        );
        assert_eq!(
            manifest["docker"]["database_connection_scope"],
            "per_exported_compose_project"
        );
        assert_eq!(
            manifest["docker"]["supports_multiple_connections_per_process"],
            false
        );
        assert_eq!(manifest["docker"]["runtime_packages"][1], "mariadb-client");
        assert_eq!(manifest["source_closure_complete"], true);
        assert_eq!(
            manifest["source_closure_evidence"]["unresolved_references"],
            0
        );
        assert_eq!(
            manifest["source_closure_evidence"]["compiler_check"],
            "passed_before_publication"
        );
        assert_eq!(manifest["complete_deployment"], false);

        fs::remove_dir_all(bundle).unwrap();
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn check_and_run_support_public_records_and_qualified_types() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/invoice.zyl\" as invoice\nfn main() { print(invoice::total().cents) }\n",
        ),
        (
            "src/invoice.zyl",
            "import \"src/money.zyl\" as money\npub fn total() -> money::Money { return money::Money { cents: 25 } }\n",
        ),
        (
            "src/money.zyl",
            "pub struct Money { cents: Int }\n",
        ),
    ]);
    let check = run(&directory, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let result = run(&directory, &["run", "main.zyl"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), "25");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn imported_type_diagnostic_names_the_imported_source() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/math.zyl\" as math\nfn main() { print(math::wrong()) }\n",
        ),
        (
            "src/math.zyl",
            "pub fn wrong() -> Int { return \"not an integer\" }\n",
        ),
    ]);
    let result = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(!result.status.success());
    let document: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(document["success"], false);
    let diagnostics = document["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics[0]["file"], "src/math.zyl");
    assert_eq!(diagnostics[0]["code"], "E-TYPE-001");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn context_exposes_the_transitive_module_graph_deterministically() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/invoice.zyl\" as invoice\nfn main() { print(invoice::total()) }\n",
        ),
        (
            "src/invoice.zyl",
            "import \"src/money.zyl\" as money\ntype LocalCache = Int\nfn helper() -> Int { return 0 }\npub type InvoiceId = Int\npub struct Invoice { id: InvoiceId }\npub fn total() -> Int { return money::amount() }\n",
        ),
        ("src/money.zyl", "pub fn amount() -> Int { return 25 }\n"),
    ]);
    let context = run(&directory, &["context", "main.zyl", "--format=json"]);
    let repeated = run(&directory, &["context", "main.zyl", "--format=json"]);
    assert!(
        context.status.success(),
        "{}",
        String::from_utf8_lossy(&context.stderr)
    );
    assert_eq!(context.stdout, repeated.stdout);
    let document: Value = serde_json::from_slice(&context.stdout).unwrap();
    assert_eq!(
        document["modules"],
        serde_json::json!([
            {"path":"main.zyl","imports":[{"alias":"invoice","path":"src/invoice.zyl"}],"exports":[]},
            {"path":"src/invoice.zyl","imports":[{"alias":"money","path":"src/money.zyl"}],"exports":[{"kind":"function","name":"total"},{"kind":"record","name":"Invoice"},{"kind":"type","name":"InvoiceId"}]},
            {"path":"src/money.zyl","imports":[],"exports":[{"kind":"function","name":"amount"}]}
        ])
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn context_reports_database_provider_bindings_without_exposing_credentials() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/billing.zyl\" as billing\nimport \"src/report.zyl\" as report\nfn main() {}\n",
        ),
        (
            "src/billing.zyl",
            "import \"src/storage.zyl\" as storage\ntable invoices { id: Id primary auto }\npub fn count() -> Int uses Database(read) { rows = sql<Int[]> { SELECT id FROM invoices } return 0 }\n",
        ),
        (
            "src/storage.zyl",
            "import \"src/database.zyl\" as dbconfig\n",
        ),
        (
            "src/report.zyl",
            "import \"src/database.zyl\" as dbconfig\ntable reports { id: Id primary auto }\npub fn report_count() -> Int uses Database(read) { rows = sql<Int[]> { SELECT id FROM reports } return 0 }\n",
        ),
        (
            "src/database.zyl",
            "database main { engine: mariadb database: \"accounting\" }\n",
        ),
        (
            "zelyra.toml",
            "[project]\nname = \"context-database\"\nversion = \"0.4.0-dev\"\nzelyra = \"0.1\"\n\n[capabilities]\ndatabase = true\n",
        ),
    ]);

    let context = run(&directory, &["context", "main.zyl", "--format=json"]);
    let repeated = run(&directory, &["context", "main.zyl", "--format=json"]);
    assert!(
        context.status.success(),
        "{}",
        String::from_utf8_lossy(&context.stdout)
    );
    assert_eq!(context.stdout, repeated.stdout);
    let document: Value = serde_json::from_slice(&context.stdout).unwrap();
    assert_eq!(
        document["database"]["configuration"],
        serde_json::json!({
            "declaration": "database:main",
            "module": "src/database.zyl",
            "name": "main",
            "engine": "mariadb",
            "database": "accounting",
            "connection_environment": "ZELYRA_DATABASE_MAIN_URL",
            "fallback_connection_environment": "DATABASE_URL"
        })
    );
    assert_eq!(
        document["database"]["consumers"],
        serde_json::json!([
            {
                "module": "src/billing.zyl",
                "provider_module": "src/database.zyl",
                "connection_environment": "ZELYRA_DATABASE_MAIN_URL",
                "fallback_connection_environment": "DATABASE_URL",
                "resolution": "transitive_import"
            },
            {
                "module": "src/report.zyl",
                "provider_module": "src/database.zyl",
                "connection_environment": "ZELYRA_DATABASE_MAIN_URL",
                "fallback_connection_environment": "DATABASE_URL",
                "resolution": "direct_import"
            }
        ])
    );
    assert_eq!(document["database"]["supports_multiple_connections"], false);
    assert_eq!(document["database"]["credentials_included"], false);
    assert_eq!(
        document["database"]["fallback_connection_environment"],
        "DATABASE_URL"
    );
    assert!(!String::from_utf8_lossy(&context.stdout).contains("PASSWORD"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn context_marks_legacy_database_url_when_no_provider_is_declared() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/reports.zyl\" as reports\nfn main() {}\n",
        ),
        (
            "src/reports.zyl",
            "table reports { id: Id primary auto }\npub fn report_count() -> Int uses Database(read) { rows = sql<Int[]> { SELECT id FROM reports } return 0 }\n",
        ),
        (
            "zelyra.toml",
            "[project]\nname = \"context-legacy-database\"\nversion = \"0.3.0\"\nzelyra = \"0.1\"\n\n[capabilities]\ndatabase = true\n",
        ),
    ]);

    let output = run(&directory, &["context", "main.zyl", "--format=json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["database"]["required"], true);
    assert_eq!(document["database"]["configuration"], Value::Null);
    assert_eq!(
        document["database"]["consumers"],
        serde_json::json!([{
            "module": "src/reports.zyl",
            "provider_module": null,
            "connection_environment": "DATABASE_URL",
            "fallback_connection_environment": null,
            "resolution": "legacy_project_environment"
        }])
    );
    assert_eq!(document["database"]["credentials_included"], false);
    assert_eq!(
        document["database"]["fallback_connection_environment"],
        Value::Null
    );

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_plan_lists_only_the_selected_modules_source_dependency_closure() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/invoices.zyl\" as invoices\nimport \"src/reports.zyl\" as reports\nfn main() {}\n",
        ),
        (
            "src/invoices.zyl",
            "import \"src/money.zyl\" as money\npub fn total() -> Int { return money::amount() }\n",
        ),
        ("src/money.zyl", "pub fn amount() -> Int { return 42 }\n"),
        ("src/reports.zyl", "pub fn count() -> Int { return 0 }\n"),
    ]);
    let result = run(
        &directory,
        &["module", "plan", "main.zyl", "src/invoices.zyl"],
    );
    let repeated = run(
        &directory,
        &["module", "plan", "main.zyl", "src/invoices.zyl"],
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
    assert_eq!(result.stdout, repeated.stdout);
    let document: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(document["schema_version"], "1");
    assert_eq!(document["command"], "module plan");
    assert_eq!(document["success"], true);
    assert_eq!(
        document["plan"]["kind"],
        "known-semantic-dependency-closure"
    );
    assert_eq!(
        document["plan"]["closure_semantics"],
        "explicit-imports-plus-statically-recognized-references"
    );
    assert_eq!(document["plan"]["database"]["required"], false);
    assert_eq!(
        document["plan"]["source_files"],
        serde_json::json!(["src/invoices.zyl", "src/money.zyl"])
    );
    assert_eq!(
        document["plan"]["modules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|module| module["path"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["src/invoices.zyl", "src/money.zyl"]
    );
    assert_eq!(document["plan"]["complete_deployment"], false);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_list_is_deterministic_and_shows_exports_package_lock_and_effect_ceiling() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"pkg:ledger\" as ledger\nfn main() { print(ledger::balance()) }\n",
        ),
        (".env", "ZELYRA_MODULE_LEDGER_API_TOKEN=sentinel-private-value\n"),
        (
            "zelyra.toml",
            "[package]\nname = \"business_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.ledger]\npath = \"packages/ledger\"\nversion = \"^2.0\"\n",
        ),
        (
            "packages/ledger/zelyra.toml",
            "[package]\nname = \"ledger\"\nversion = \"2.1.0\"\nentry = \"src/lib.zyl\"\nzelyra = \">=1.0.0\"\neffects = [\"Database(read)\"]\n\n[settings.api_timeout]\ntype = \"Int\"\nrequired = false\ndefault = 15\ndescription = \"Request timeout in seconds\"\n\n[settings.api_token]\ntype = \"String\"\nrequired = true\nsecret = true\ndescription = \"Private API credential\"\n",
        ),
        (
            "packages/ledger/src/lib.zyl",
            "pub fn balance() -> Int { return 0 }\n",
        ),
    ]);
    let lock = run(&directory, &["package", "lock", "main.zyl"]);
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );

    let first = run(&directory, &["module", "list", "main.zyl", "--format=json"]);
    let second = run(&directory, &["module", "list", "main.zyl", "--format=json"]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(first.stdout, second.stdout);
    assert!(!String::from_utf8_lossy(&first.stdout).contains("sentinel-private-value"));
    let document: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(document["command"], "module list");
    assert_eq!(document["success"], true);
    assert_eq!(document["package_lock"]["status"], "current");
    assert_eq!(document["capabilities_granted"], false);
    assert_eq!(document["secrets_included"], false);
    assert!(document["modules"]
        .as_array()
        .unwrap()
        .iter()
        .any(|module| {
            module["path"] == "packages/ledger/src/lib.zyl"
                && module["exports"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|export| export["kind"] == "function" && export["name"] == "balance")
        }));
    assert!(document["packages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|package| {
            package["name"] == "ledger"
                && package["version"] == "2.1.0"
                && package["zelyra"] == ">=1.0.0"
                && package["declared_effect_ceiling"] == serde_json::json!(["Database(read)"])
                && package["settings_schema"]
                    == serde_json::json!([
                        {
                            "name": "api_timeout",
                            "type": "Int",
                            "required": false,
                            "secret": false,
                            "default": 15,
                            "description": "Request timeout in seconds"
                        },
                        {
                            "name": "api_token",
                            "type": "String",
                            "required": true,
                            "secret": true,
                            "default": null,
                            "description": "Private API credential"
                        }
                    ])
                && package["dependencies"].as_array().unwrap().is_empty()
        }));
    let human = run(&directory, &["module", "list", "main.zyl"]);
    assert!(human.status.success());
    assert!(String::from_utf8_lossy(&human.stdout).contains("export function balance"));
    assert!(String::from_utf8_lossy(&human.stdout).contains("zelyra compatibility: >=1.0.0"));
    assert!(String::from_utf8_lossy(&human.stdout).contains("effect ceiling: Database(read)"));
    assert!(String::from_utf8_lossy(&human.stdout)
        .contains("setting api_timeout: Int (optional, not secret)"));
    assert!(String::from_utf8_lossy(&human.stdout).contains("default: 15"));
    assert!(String::from_utf8_lossy(&human.stdout)
        .contains("setting api_token: String (required, secret)"));
    let help = run(&directory, &["module", "list", "--help"]);
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("Effektobergrenze"));

    fs::write(
        directory.join("packages/ledger/src/lib.zyl"),
        "pub fn balance() -> Int { return 1 }\n",
    )
    .unwrap();
    let stale = run(&directory, &["module", "list", "main.zyl", "--format=json"]);
    assert!(!stale.status.success());
    let stale_document: Value = serde_json::from_slice(&stale.stdout).unwrap();
    assert_eq!(stale_document["package_lock"]["status"], "not_verified");
    assert!(stale_document["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|diagnostic| diagnostic["code"] == "E-PKG-002"));
}

#[test]
fn module_settings_check_validates_values_and_never_emits_secret_data() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"pkg:ledger\" as ledger\nfn main() { print(ledger::timeout()) print(ledger::token_configured()) }\n",
        ),
        (
            "zelyra.toml",
            "[package]\nname = \"business_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.ledger]\npath = \"packages/ledger\"\nversion = \"^2.0\"\n",
        ),
        (
            "packages/ledger/zelyra.toml",
            "[package]\nname = \"ledger\"\nversion = \"2.1.0\"\nentry = \"src/lib.zyl\"\n\n[settings.api_timeout]\ntype = \"Int\"\nrequired = false\ndefault = 15\n\n[settings.api_token]\ntype = \"String\"\nrequired = true\nsecret = true\n",
        ),
        (
            "packages/ledger/src/lib.zyl",
            "pub fn balance() -> Int { return 0 }\npub fn timeout() -> Int { return module_setting<Int>(\"api_timeout\") }\npub fn token_configured() -> Bool { return module_setting<String>(\"api_token\") != \"\" }\n",
        ),
        (
            "zelyra.settings.toml",
            "[package_settings.\"packages/ledger\"]\napi_token = { env = \"LEDGER_TOKEN\" }\n",
        ),
    ]);
    let lock = run(&directory, &["package", "lock", "main.zyl"]);
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );

    let secret = "never-print-this-token";
    let execution_missing =
        run_with_environment(&directory, &["run", "main.zyl"], "LEDGER_TOKEN", None);
    assert!(!execution_missing.status.success());
    assert!(
        String::from_utf8_lossy(&execution_missing.stderr).contains("error[E-PKG-SETTINGS-002]"),
        "{}",
        String::from_utf8_lossy(&execution_missing.stderr)
    );
    assert!(!String::from_utf8_lossy(&execution_missing.stderr).contains("LEDGER_TOKEN"));
    let serve_missing = run_with_environment(
        &directory,
        &["serve", "main.zyl", "127.0.0.1:0"],
        "LEDGER_TOKEN",
        None,
    );
    assert!(!serve_missing.status.success());
    assert!(String::from_utf8_lossy(&serve_missing.stderr).contains("error[E-PKG-SETTINGS-002]"));
    assert!(!String::from_utf8_lossy(&serve_missing.stderr).contains("LEDGER_TOKEN"));
    for backend in ["ast", "mir", "bytecode"] {
        let execution_ready = run_with_environment(
            &directory,
            &["run", "main.zyl", "--backend", backend],
            "LEDGER_TOKEN",
            Some(secret),
        );
        assert!(
            execution_ready.status.success(),
            "backend {backend}: {}",
            String::from_utf8_lossy(&execution_ready.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&execution_ready.stdout).trim(),
            "15\ntrue",
            "backend {backend}"
        );
        assert!(!String::from_utf8_lossy(&execution_ready.stdout).contains(secret));
    }
    let configured = run_with_environment(
        &directory,
        &["module", "settings", "check", "main.zyl", "--format=json"],
        "LEDGER_TOKEN",
        Some(secret),
    );
    assert!(
        configured.status.success(),
        "{}",
        String::from_utf8_lossy(&configured.stderr)
    );
    let document: Value = serde_json::from_slice(&configured.stdout).unwrap();
    assert_eq!(document["command"], "module settings check");
    assert_eq!(document["success"], true);
    assert_eq!(document["secret_values_included"], false);
    assert!(configured
        .stdout
        .windows(secret.len())
        .all(|window| window != secret.as_bytes()));
    let settings = document["settings"][0]["values"].as_array().unwrap();
    assert!(settings.iter().any(|setting| {
        setting["name"] == "api_token" && setting["status"] == "secret_available"
    }));
    assert!(settings.iter().any(|setting| {
        setting["name"] == "api_timeout"
            && setting["status"] == "defaulted"
            && setting["required"] == false
    }));
    assert!(settings
        .iter()
        .any(|setting| { setting["name"] == "api_token" && setting["required"] == true }));

    let missing = run_with_environment(
        &directory,
        &["module", "settings", "check", "main.zyl", "--format=json"],
        "LEDGER_TOKEN",
        None,
    );
    assert!(!missing.status.success());
    let document: Value = serde_json::from_slice(&missing.stdout).unwrap();
    assert_eq!(document["success"], false);
    assert!(document["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item["code"] == "E-PKG-SETTINGS-002"
                && item["message"].as_str().unwrap().contains("api_token")
        }));
    assert!(!String::from_utf8_lossy(&missing.stdout).contains("LEDGER_TOKEN"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn typed_secret_package_settings_are_checked_and_shared_by_all_backends() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"pkg:limits\" as limits\nfn main() { print(limits::max_connections()) }\n",
        ),
        (
            "zelyra.toml",
            "[package]\nname = \"business_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.limits]\npath = \"packages/limits\"\nversion = \"^1.0\"\n",
        ),
        (
            "packages/limits/zelyra.toml",
            "[package]\nname = \"limits\"\nversion = \"1.0.0\"\nentry = \"src/lib.zyl\"\n\n[settings.max_connections]\ntype = \"Int\"\nrequired = true\nsecret = true\n",
        ),
        (
            "packages/limits/src/lib.zyl",
            "pub fn max_connections() -> Int { return module_setting<Int>(\"max_connections\") }\n",
        ),
        (
            "zelyra.settings.toml",
            "[package_settings.\"packages/limits\"]\nmax_connections = { env = \"MAX_CONNECTIONS\" }\n",
        ),
    ]);
    let lock = run(&directory, &["package", "lock", "main.zyl"]);
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );

    for backend in ["ast", "mir", "bytecode"] {
        let execution = run_with_environment(
            &directory,
            &["run", "main.zyl", "--backend", backend],
            "MAX_CONNECTIONS",
            Some("64"),
        );
        assert!(
            execution.status.success(),
            "backend {backend}: {}",
            String::from_utf8_lossy(&execution.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&execution.stdout).trim(), "64");
    }

    let invalid = run_with_environment(
        &directory,
        &["module", "settings", "check", "main.zyl", "--format=json"],
        "MAX_CONNECTIONS",
        Some("64-connections-private"),
    );
    assert!(!invalid.status.success());
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&invalid.stdout),
        String::from_utf8_lossy(&invalid.stderr)
    );
    assert!(output.contains("does not match type `Int`"), "{output}");
    assert!(!output.contains("64-connections-private"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_setting_requires_a_declared_setting_and_exact_schema_type() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"pkg:config\" as config\nfn main() { print(config::timeout()) }\n",
        ),
        (
            "zelyra.toml",
            "[package]\nname = \"app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.config]\npath = \"packages/config\"\nversion = \"^1.0\"\n",
        ),
        (
            "packages/config/zelyra.toml",
            "[package]\nname = \"config\"\nversion = \"1.0.0\"\nentry = \"src/lib.zyl\"\n\n[settings.timeout]\ntype = \"Int\"\nrequired = false\ndefault = 30\n",
        ),
        (
            "packages/config/src/lib.zyl",
            "pub fn timeout() -> String { return module_setting<String>(\"timeout\") }\n",
        ),
    ]);
    let lock = run(&directory, &["package", "lock", "main.zyl"]);
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );

    let invalid_type = run(&directory, &["check", "main.zyl"]);
    assert!(!invalid_type.status.success());
    let type_errors = String::from_utf8_lossy(&invalid_type.stderr);
    assert!(
        type_errors.contains("error[E-PKG-SETTINGS-004]"),
        "{type_errors}"
    );

    fs::write(
        directory.join("packages/config/src/lib.zyl"),
        "pub fn timeout() -> Int { return module_setting<Int>(\"timeout\") }\n",
    )
    .unwrap();
    fs::write(
        directory.join("main.zyl"),
        "import \"pkg:config\" as config\nfn main() { print(module_setting<Int>(\"timeout\")) }\n",
    )
    .unwrap();
    let refreshed = run(&directory, &["package", "lock", "main.zyl", "--update"]);
    assert!(
        refreshed.status.success(),
        "{}",
        String::from_utf8_lossy(&refreshed.stderr)
    );
    let cross_package = run(&directory, &["check", "main.zyl"]);
    assert!(!cross_package.status.success());
    let errors = String::from_utf8_lossy(&cross_package.stderr);
    assert!(errors.contains("error[E-PKG-SETTINGS-003]"), "{errors}");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn reusable_csv_package_writes_escaped_output_through_file_capability() {
    const CSV_PACKAGE_SOURCE: &str =
        include_str!("../../examples/business_modules/packages/csv_export/src/lib.zyl");
    let directory = project(&[
        (
            "main.zyl",
            r#"
                import "pkg:csv_export" as csv_export
                fn main() uses FileSystem {
                    rows = [["name", "note"], ["A", "quoted, value"], ["B", "line\nbreak"]]
                    match csv_export::write_csv("data/export.csv", rows) {
                        Ok(_) => { print("CSV exported") }
                        Err(message) => { print(message) }
                    }
                }
            "#,
        ),
        (
            "zelyra.toml",
            "[package]\nname = \"csv_demo\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[capabilities]\nfilesystem = true\ndatabase = false\nnetwork = false\n\n[filesystem]\nread_roots = [\"data\"]\nwrite_roots = [\"data\"]\n\n[dependencies.csv_export]\npath = \"packages/csv_export\"\nversion = \"^1.0\"\n",
        ),
        (
            "packages/csv_export/zelyra.toml",
            "[package]\nname = \"csv_export\"\nversion = \"1.0.0\"\nentry = \"src/lib.zyl\"\neffects = [\"FileSystem\"]\n",
        ),
        ("packages/csv_export/src/lib.zyl", CSV_PACKAGE_SOURCE),
    ]);
    fs::create_dir_all(directory.join("data")).unwrap();
    let locked = run(&directory, &["package", "lock", "main.zyl"]);
    assert!(
        locked.status.success(),
        "{}",
        String::from_utf8_lossy(&locked.stderr)
    );
    let output = run(&directory, &["run", "main.zyl"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("CSV exported"));
    assert_eq!(
        fs::read_to_string(directory.join("data/export.csv")).unwrap(),
        "name,note\nA,\"quoted, value\"\nB,\"line\nbreak\""
    );

    let manifest_path = directory.join("zelyra.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    fs::write(
        &manifest_path,
        manifest.replace("filesystem = true", "filesystem = false"),
    )
    .unwrap();
    let relocked = run(&directory, &["package", "lock", "main.zyl", "--update"]);
    assert!(
        relocked.status.success(),
        "{}",
        String::from_utf8_lossy(&relocked.stderr)
    );
    let denied = run(&directory, &["run", "main.zyl"]);
    assert!(!denied.status.success());
    assert!(
        String::from_utf8_lossy(&denied.stderr).contains("FileSystem"),
        "{}",
        String::from_utf8_lossy(&denied.stderr)
    );
    assert_eq!(
        fs::read_to_string(directory.join("data/export.csv")).unwrap(),
        "name,note\nA,\"quoted, value\"\nB,\"line\nbreak\""
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_plan_and_docker_bundle_export_password_reset_smtp_contracts() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/auth.zyl\" as account_data\nfn main() {}\n",
        ),
        (
            "src/auth.zyl",
            "import \"src/reset_limits.zyl\" as reset_storage\nauth users { table: users sessions: auth_sessions reset_tokens: password_resets reset_rate_limits: password_reset_limits audit: auth_audit_log }\ntable users { id: Id primary auto email: Email required unique password_hash: String(255) required }\ntable auth_sessions { id: Id primary auto user: User required token_hash: String(64) required unique expires_at: Timestamp required }\ntable password_resets { id: Id primary auto user: User required token_hash: String(64) required unique expires_at: Timestamp required consumed_at: Timestamp? delivery_payload: String(2048) delivery_retry_at: Timestamp? }\ntable auth_audit_log { id: Id primary auto actor_user_id: Int? event: String(100) required target_user_id: Int? details: String(1000) required created_at: Timestamp default now }\n",
        ),
        ("src/reset_limits.zyl", "table password_reset_limits { bucket_key: String(64) primary window_started: Timestamp required request_count: Int required blocked_until: Timestamp? index { window_started } access { read_write: [\"src/auth.zyl\"] } }\n"),
    ]);
    let plan = run(&directory, &["module", "plan", "main.zyl", "src/auth.zyl"]);
    assert!(
        plan.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&plan.stdout),
        String::from_utf8_lossy(&plan.stderr)
    );
    let document: Value = serde_json::from_slice(&plan.stdout).unwrap();
    assert!(document["plan"]["modules"]
        .as_array()
        .unwrap()
        .iter()
        .any(|module| module["path"] == "src/reset_limits.zyl"));
    assert_eq!(
        document["plan"]["external_service_contracts"][0]["kind"],
        "smtp"
    );
    assert_eq!(
        document["plan"]["external_service_contracts"][0]["used_by"],
        "auth:users"
    );
    assert_eq!(
        document["plan"]["external_service_contracts"][0]["secret_environment_variables"],
        serde_json::json!([
            "ZELYRA_SMTP_USERNAME",
            "ZELYRA_SMTP_PASSWORD",
            "ZELYRA_RESET_DELIVERY_KEY"
        ])
    );
    assert_eq!(
        document["plan"]["external_service_contracts"][0]["required_when"],
        "auth_reset_tokens_enabled"
    );
    assert_eq!(
        document["plan"]["external_service_contracts"][0]["contract_version"],
        1
    );
    assert_eq!(
        document["plan"]["external_service_contracts"][0]["retry_delay_seconds"],
        30
    );
    assert_eq!(
        document["plan"]["external_service_contracts"][0]["duplicate_delivery_possible"],
        true
    );

    let bundle = directory.with_extension("smtp-docker-bundle");
    let bundle_arg = bundle.to_string_lossy().into_owned();
    let compiler_ref = "a".repeat(40);
    let result = run(
        &directory,
        &[
            "module",
            "bundle",
            "main.zyl",
            "src/auth.zyl",
            "--output",
            &bundle_arg,
            "--docker",
            "--compiler-ref",
            &compiler_ref,
        ],
    );
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let env_example = fs::read_to_string(bundle.join(".env.example")).unwrap();
    for variable in [
        "ZELYRA_PUBLIC_BASE_URL=https://app.example.test",
        "ZELYRA_SMTP_HOST=",
        "ZELYRA_SMTP_PORT=465",
        "ZELYRA_SMTP_SECURITY=implicit_tls",
        "ZELYRA_SMTP_FROM=",
        "ZELYRA_SMTP_USERNAME=",
        "ZELYRA_SMTP_PASSWORD=",
        "ZELYRA_RESET_DELIVERY_KEY=",
    ] {
        assert!(
            env_example.lines().any(|line| line == variable),
            "{variable}"
        );
    }
    let manifest: Value =
        serde_json::from_slice(&fs::read(bundle.join("zelyra.bundle.json")).unwrap()).unwrap();
    assert_eq!(manifest["external_service_contracts"][0]["kind"], "smtp");
    assert_eq!(
        manifest["external_service_contracts"][0]["delivery_semantics"],
        "at_least_once"
    );
    assert_eq!(
        manifest["external_service_contracts"][0]["contract_version"],
        1
    );
    assert!(!env_example.contains("must-not-be-copied"));
    let check = run(&bundle, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let context = run(&directory, &["context", "main.zyl", "--format=json"]);
    assert!(context.status.success());
    let context: Value = serde_json::from_slice(&context.stdout).unwrap();
    assert_eq!(
        context["declarations"]["auth"][0]["reset_rate_limits_table"],
        "password_reset_limits"
    );
    let rate_path = directory.join("src/reset_limits.zyl");
    let rate_source = fs::read_to_string(&rate_path).unwrap();
    fs::write(
        &rate_path,
        rate_source.replace("index { window_started }", ""),
    )
    .unwrap();
    let invalid = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(!invalid.status.success());
    let invalid: Value = serde_json::from_slice(&invalid.stdout).unwrap();
    assert!(invalid["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|diagnostic| {
            diagnostic["code"] == "E-AUTH-041" && diagnostic["file"] == "src/auth.zyl"
        }));
    fs::write(&rate_path, rate_source.replace("read_write:", "read:")).unwrap();
    let denied = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(!denied.status.success());
    let denied: Value = serde_json::from_slice(&denied.stdout).unwrap();
    assert!(denied["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|diagnostic| {
            diagnostic["code"] == "E-MOD-021" && diagnostic["file"] == "src/auth.zyl"
        }));
    fs::remove_dir_all(bundle).unwrap();
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_plan_reports_runtime_effects_in_its_source_closure() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/storage.zyl\" as storage\nfn main() {}\n",
        ),
        (
            "src/storage.zyl",
            "pub fn read_customer_file(path: String) -> String uses FileSystem { return read_text(path) }\n",
        ),
    ]);
    let result = run(
        &directory,
        &["module", "plan", "main.zyl", "src/storage.zyl"],
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
    let document: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        document["plan"]["runtime_effects"]["effects"],
        serde_json::json!([{
            "capability": "FileSystem",
            "declared_by": ["function:src/storage.zyl::read_customer_file"],
            "source_modules": ["src/storage.zyl"]
        }])
    );
    assert_eq!(
        document["plan"]["runtime_effects"]["explicit_function_capabilities_complete"],
        true
    );
    assert_eq!(
        document["plan"]["runtime_effects"]["implicit_resource_effects_complete"],
        false
    );
    assert_eq!(document["plan"]["deployment_readiness"]["ready"], false);
    assert_eq!(
        document["plan"]["deployment_readiness"]["status"],
        "incomplete"
    );
    assert_eq!(
        document["plan"]["deployment_readiness"]["unverified_requirements"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        document["plan"]["deployment_readiness"]["blockers"][0]["reason"],
        "file_system_paths_are_not_declared_in_the_bundle_manifest"
    );
    let bundle = directory.with_extension("effect-bundle");
    let bundle_arg = bundle.to_string_lossy().into_owned();
    let bundled = run(
        &directory,
        &[
            "module",
            "bundle",
            "main.zyl",
            "src/storage.zyl",
            "--output",
            &bundle_arg,
        ],
    );
    assert!(
        bundled.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&bundled.stdout),
        String::from_utf8_lossy(&bundled.stderr)
    );
    let manifest: Value =
        serde_json::from_slice(&fs::read(bundle.join("zelyra.bundle.json")).unwrap()).unwrap();
    assert_eq!(manifest["complete_deployment"], false);
    assert_eq!(manifest["deployment_readiness"]["ready"], false);
    assert_eq!(
        manifest["deployment_readiness"]["blockers"][0]["reason"],
        "file_system_paths_are_not_declared_in_the_bundle_manifest"
    );
    assert_eq!(
        manifest["runtime_effects"]["effects"][0]["capability"],
        "FileSystem"
    );
    fs::remove_dir_all(bundle).unwrap();
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_plan_blocks_effects_without_declared_runtime_contracts() {
    for (capability, reason) in [
        (
            "Network",
            "outbound_network_service_contract_is_not_declared",
        ),
        (
            "Process",
            "process_executable_and_runtime_dependencies_are_not_declared",
        ),
        ("Environment", "environment_variable_names_are_not_declared"),
    ] {
        let source =
            format!("pub fn external_effect() uses {capability} {{ print(\"effect\") }}\n");
        let directory = project(&[
            (
                "main.zyl",
                "import \"src/effects.zyl\" as effects\nfn main() {}\n",
            ),
            ("src/effects.zyl", &source),
        ]);
        let result = run(
            &directory,
            &["module", "plan", "main.zyl", "src/effects.zyl"],
        );
        assert!(
            result.status.success(),
            "{capability}: {}",
            String::from_utf8_lossy(&result.stdout)
        );
        let document: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(
            document["plan"]["runtime_effects"]["effects"][0]["capability"],
            capability
        );
        assert_eq!(
            document["plan"]["deployment_readiness"]["blockers"][0]["reason"],
            reason
        );
        fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn module_plan_adds_cross_module_views_components_and_database_tables() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/pages.zyl\" as pages\nimport \"src/ui.zyl\" as ui\nimport \"src/models.zyl\" as models\nfn main() {}\n",
        ),
        (
            "src/pages.zyl",
            "import \"src/models.zyl\" as models\nimport \"src/ui.zyl\" as ui\npage \"/invoices\" { view: InvoiceLayout load invoices = sql<Invoice[]> { SELECT id, customer_id FROM invoices } html { <InvoiceBadge label=\"Invoices\" /> } }\n",
        ),
        (
            "src/ui.zyl",
            "pub component InvoiceBadge { props { label: String } html { <strong>{label}</strong> } }\npub view InvoiceLayout { html { <main><InvoiceBadge label=\"Title\" /><slot /></main> } }\n",
        ),
        (
            "src/models.zyl",
            "table customers { id: Id primary auto name: String(100) required }\ntable invoices { id: Id primary auto customer: Customer required access { read: [\"src/pages.zyl\"] } }\n",
        ),
    ]);
    let result = run(&directory, &["module", "plan", "main.zyl", "src/pages.zyl"]);
    let repeated = run(&directory, &["module", "plan", "main.zyl", "src/pages.zyl"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
    assert_eq!(result.stdout, repeated.stdout);
    let document: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        document["plan"]["source_files"],
        serde_json::json!(["src/models.zyl", "src/pages.zyl", "src/ui.zyl"])
    );
    assert_eq!(document["plan"]["database"]["required"], true);
    assert_eq!(
        document["plan"]["database"]["configuration_sources"],
        serde_json::json!([])
    );
    assert_eq!(
        document["plan"]["database"]["configurations"],
        serde_json::json!([])
    );
    assert_eq!(
        document["plan"]["database"]["connection_model"],
        "single-project-wide-connection"
    );
    assert_eq!(
        document["plan"]["database"]["connection_environment"],
        "DATABASE_URL"
    );
    assert_eq!(
        document["plan"]["database"]["supports_multiple_connections"],
        false
    );
    let dependencies = document["plan"]["resource_dependencies"]
        .as_array()
        .unwrap();
    assert!(dependencies.iter().any(|dependency| {
        dependency["from"] == "page:/invoices"
            && dependency["to"] == "view:InvoiceLayout"
            && dependency["kind"] == "view"
    }));
    assert!(dependencies.iter().any(|dependency| {
        dependency["from"] == "page:/invoices"
            && dependency["to"] == "component:InvoiceBadge"
            && dependency["kind"] == "component"
    }));
    assert!(dependencies.iter().any(|dependency| {
        dependency["from"] == "page:/invoices"
            && dependency["to"] == "table:invoices"
            && dependency["kind"] == "page_data_sql"
    }));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_plan_includes_separate_database_configuration_source() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/pages.zyl\" as pages\nimport \"src/database.zyl\" as storage\nfn main() {}\n",
        ),
        (
            "src/pages.zyl",
            "import \"src/database.zyl\" as storage\ntable invoices { id: Id primary auto }\npage \"/invoices\" { load invoices = sql<Invoice[]> { SELECT id FROM invoices } html { <p>Invoices</p> } }\n",
        ),
        (
            "src/database.zyl",
            "database main { engine: mariadb database: \"invoices\" }\n",
        ),
    ]);
    let result = run(&directory, &["module", "plan", "main.zyl", "src/pages.zyl"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
    let document: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        document["plan"]["source_files"],
        serde_json::json!(["src/database.zyl", "src/pages.zyl"])
    );
    assert_eq!(document["plan"]["database"]["required"], true);
    assert_eq!(
        document["plan"]["database"]["configuration_sources"],
        serde_json::json!(["src/database.zyl"])
    );
    assert_eq!(
        document["plan"]["database"]["configurations"],
        serde_json::json!([{
            "declaration": "database:main",
            "module": "src/database.zyl",
            "engine": "mariadb",
            "database": "invoices",
            "connection_environment": "ZELYRA_DATABASE_MAIN_URL"
        }])
    );
    assert_eq!(
        document["plan"]["database"]["supports_multiple_connections"],
        false
    );
    assert!(document["plan"]["resource_dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|dependency| {
            dependency["from_module"] == "src/pages.zyl"
                && dependency["to_module"] == "src/database.zyl"
                && dependency["kind"] == "database_configuration"
        }));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_plan_reports_inferred_table_owner_and_sql_access_modes() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/customer_service.zyl\" as customers\nimport \"src/database.zyl\" as storage\nfn main() {}\n",
        ),
        (
            "src/customer_service.zyl",
            "import \"src/customer_schema.zyl\" as schema\npub fn listCustomers() uses Database(read) { return sql<Customer[]> { SELECT id, name FROM customers } }\npub fn renameCustomer(id: Id, name: String) uses Database(write) { schema::renameCustomer(id, name) }\n",
        ),
        (
            "src/customer_schema.zyl",
            "import \"src/database.zyl\" as storage\ntable customers { id: Id primary auto name: String(100) required access { read: [\"src/customer_service.zyl\"] } }\npub fn renameCustomer(id: Id, name: String) uses Database(write) { sql { UPDATE customers SET name = :name WHERE id = :id } }\n",
        ),
        (
            "src/database.zyl",
            "database main { engine: mariadb database: \"customers\" }\n",
        ),
    ]);
    let plan = run(
        &directory,
        &["module", "plan", "main.zyl", "src/customer_service.zyl"],
    );
    assert!(
        plan.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&plan.stdout),
        String::from_utf8_lossy(&plan.stderr)
    );
    let plan: Value = serde_json::from_slice(&plan.stdout).unwrap();
    assert_eq!(
        plan["plan"]["schema_ownership"]["model"],
        "inferred_from_table_declaration_source_module"
    );
    assert_eq!(plan["plan"]["schema_ownership"]["enforced"], false);
    assert_eq!(
        plan["plan"]["table_access_contract"]["dependency_enforced"],
        true
    );
    assert_eq!(
        plan["plan"]["table_access_contract"]["cross_module_mutation_policy"],
        "explicit-table-access-grant"
    );
    assert_eq!(
        plan["plan"]["table_access_contract"]["module_access_grants_enforced"],
        true
    );
    assert_eq!(
        plan["plan"]["table_access_contract"]["entry_module_tables_project_visible"],
        false
    );
    assert_eq!(
        plan["plan"]["schema_ownership"]["tables"],
        serde_json::json!([{
            "table": "customers",
            "inferred_owner_module": "src/customer_schema.zyl",
            "ownership_enforced": false,
            "cross_module_access_grants_enforced": true,
            "access_grants": {
                "read": ["src/customer_service.zyl"],
                "write": [],
                "read_write": []
            }
        }])
    );
    let edges = plan["plan"]["declaration_closure"]["edges"]
        .as_array()
        .unwrap();
    assert!(
        edges.iter().any(|edge| {
            edge["from"] == "function:src/customer_service.zyl::listCustomers"
                && edge["to"] == "table:customers"
                && edge["kind"] == "sql_table"
                && edge["access"] == "read"
        }),
        "{edges:#?}"
    );
    assert!(edges.iter().any(|edge| {
        edge["from"] == "function:src/customer_schema.zyl::renameCustomer"
            && edge["to"] == "table:customers"
            && edge["kind"] == "sql_table"
            && edge["access"] == "write"
    }));
    let dependencies = plan["plan"]["resource_dependencies"].as_array().unwrap();
    assert!(
        dependencies.iter().any(|dependency| {
            dependency["from_module"] == "src/customer_service.zyl"
                && dependency["to_module"] == "src/customer_schema.zyl"
                && dependency["to"] == "table:customers"
                && dependency["access"] == "read"
        }),
        "{dependencies:#?}"
    );
    assert!(!dependencies.iter().any(|dependency| {
        dependency["from_module"] == "src/customer_service.zyl"
            && dependency["to_module"] == "src/customer_schema.zyl"
            && dependency["to"] == "table:customers"
            && dependency["access"] == "write"
    }));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn cross_module_table_access_requires_the_owner_in_the_import_graph() {
    let source = "pub fn listCustomers() uses Database(read) { return sql<Customer[]> { SELECT id, name FROM customers } }\npub fn renameCustomer(id: Id, name: String) uses Database(write) { sql { UPDATE customers SET name = :name WHERE id = :id } }\n";
    let allowed = project(&[
        (
            "main.zyl",
            "import \"src/customer_service.zyl\" as customers\nfn main() {}\n",
        ),
        (
            "src/customer_service.zyl",
            "import \"src/customer_schema.zyl\" as schema\npub fn listCustomers() uses Database(read) { return schema::listCustomers() }\npub fn renameCustomer(id: Id, name: String) uses Database(write) { schema::renameCustomer(id, name) }\n",
        ),
        (
            "src/customer_schema.zyl",
            "import \"src/database.zyl\" as storage\ntable customers { id: Id primary auto name: String(100) required access { read: [\"src/customer_service.zyl\"] } }\npub fn listCustomers() uses Database(read) { return sql<Customer[]> { SELECT id, name FROM customers } }\npub fn renameCustomer(id: Id, name: String) uses Database(write) { sql { UPDATE customers SET name = :name WHERE id = :id } }\n",
        ),
        (
            "src/database.zyl",
            "database main { engine: mariadb database: \"customers\" }\n",
        ),
    ]);
    let allowed_check = run(&allowed, &["check", "main.zyl", "--format=json"]);
    assert!(
        allowed_check.status.success(),
        "{}",
        String::from_utf8_lossy(&allowed_check.stdout)
    );
    fs::remove_dir_all(allowed).unwrap();

    let denied = project(&[
        (
            "main.zyl",
            "import \"src/customer_service.zyl\" as customers\nimport \"src/customer_schema.zyl\" as schema\nimport \"src/database.zyl\" as storage\nfn main() {}\n",
        ),
        (
            "src/customer_service.zyl",
            &format!("import \"src/customer_schema.zyl\" as schema\n{source}"),
        ),
        (
            "src/customer_schema.zyl",
            "import \"src/database.zyl\" as storage\ntable customers { id: Id primary auto name: String(100) required }\n",
        ),
        (
            "src/database.zyl",
            "database main { engine: mariadb database: \"customers\" }\n",
        ),
    ]);
    let denied_check = run(&denied, &["check", "main.zyl", "--format=json"]);
    assert!(!denied_check.status.success());
    let diagnostics: Value = serde_json::from_slice(&denied_check.stdout).unwrap();
    let boundary = diagnostics["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "E-MOD-021")
        .expect("non-owner table writes must be rejected");
    assert_eq!(boundary["file"], "src/customer_service.zyl");
    assert!(boundary["message"].as_str().unwrap().contains("write"));
    assert!(boundary["message"]
        .as_str()
        .unwrap()
        .contains("src/customer_schema.zyl"));
    assert!(boundary["message"]
        .as_str()
        .unwrap()
        .contains("matching table access grant"));
    fs::remove_dir_all(denied).unwrap();
}

#[test]
fn generated_crud_writes_require_the_table_owning_module() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/customer_ui.zyl\" as customers\nimport \"src/customer_schema.zyl\" as schema\nimport \"src/database.zyl\" as storage\nfn main() {}\n",
        ),
        (
            "src/customer_ui.zyl",
            "import \"src/customer_schema.zyl\" as schema\ncrud Customer -> customers\n",
        ),
        (
            "src/customer_schema.zyl",
            "import \"src/database.zyl\" as storage\ntable customers { id: Id primary auto name: String(100) required }\n",
        ),
        (
            "src/database.zyl",
            "database main { engine: mariadb database: \"customers\" }\n",
        ),
    ]);

    let check = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(!check.status.success());
    let diagnostics: Value = serde_json::from_slice(&check.stdout).unwrap();
    let boundary = diagnostics["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "E-MOD-021")
        .expect("generated CRUD must not silently write another module's table");
    assert_eq!(boundary["file"], "src/customer_ui.zyl");
    assert!(boundary["message"].as_str().unwrap().contains("read_write"));

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn an_unmatched_table_access_grant_does_not_authorize_a_consumer() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/report.zyl\" as report\nimport \"src/schema.zyl\" as schema\nfn main() {}\n",
        ),
        (
            "src/report.zyl",
            "import \"src/schema.zyl\" as schema\nfn load() uses Database(read) { return sql<Int[]> { SELECT id FROM customers } }\n",
        ),
        (
            "src/schema.zyl",
            "table customers { id: Id primary auto access { read: [\"src/missing.zyl\"] } }\n",
        ),
    ]);

    let check = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(!check.status.success());
    let diagnostics: Value = serde_json::from_slice(&check.stdout).unwrap();
    let invalid_grant = diagnostics["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "E-MOD-021")
        .expect("a grant for another module must not authorize the consumer");
    assert_eq!(invalid_grant["file"], "src/report.zyl");
    assert!(invalid_grant["message"]
        .as_str()
        .unwrap()
        .contains("matching table access grant"));

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn imported_modules_must_import_schema_instead_of_inheriting_entry_tables() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/customer_service.zyl\" as customers\ntable customers { id: Id primary auto name: String(100) required }\nfn main() {}\n",
        ),
        (
            "src/customer_service.zyl",
            "pub fn listCustomers() -> Customer[] uses Database(read) { return sql<Customer[]> { SELECT id, name FROM customers } }\n",
        ),
    ]);

    let check = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(!check.status.success());
    let diagnostics: Value = serde_json::from_slice(&check.stdout).unwrap();
    let boundary = diagnostics["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "E-MOD-019")
        .expect("entry-module tables must not be implicitly visible to children");
    assert_eq!(boundary["file"], "src/customer_service.zyl");
    assert!(boundary["message"].as_str().unwrap().contains("main.zyl"));

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_plan_composes_imported_forms_and_crud_with_database_dependencies() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/models.zyl\" as models\nimport \"src/customers.zyl\" as customers\nimport \"src/database.zyl\" as storage\nimport \"src/audit.zyl\" as audit\nfn main() {}\n",
        ),
        (
            "src/models.zyl",
            "table customers { id: Id primary auto name: String(100) required access { read_write: [\"src/customers.zyl\"] } }\n",
        ),
        (
            "src/audit.zyl",
            "table audit_events { id: Id primary auto message: String(200) required access { write: [\"src/customers.zyl\"] } }\n",
        ),
        (
            "src/customers.zyl",
            "import \"src/database.zyl\" as storage\nimport \"src/models.zyl\" as models\nimport \"src/audit.zyl\" as audit\nimport \"src/helpers.zyl\" as helpers\nform CustomerCreate -> customers { fields { name } action save { let clean_name = helpers::normalize_name() sql { INSERT INTO customers (name) VALUES (:clean_name) } sql { INSERT INTO audit_events (message) VALUES (:clean_name) } } }\ncrud Customer -> customers { list { id name } action audit { let message = helpers::audit_message() sql { INSERT INTO audit_events (message) VALUES ('crud') } } }\n",
        ),
        (
            "src/helpers.zyl",
            "pub fn normalize_name() -> String { return \"New customer\" }\npub fn audit_message() -> String { return \"crud\" }\n",
        ),
        (
            "src/database.zyl",
            "database main { engine: mariadb database: \"customers\" }\n",
        ),
    ]);
    let check = run(&directory, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let context = run(&directory, &["context", "main.zyl", "--format=json"]);
    assert!(
        context.status.success(),
        "{}",
        String::from_utf8_lossy(&context.stderr)
    );
    let context: Value = serde_json::from_slice(&context.stdout).unwrap();
    assert_eq!(
        context["declarations"]["forms"][0]["span"]["file"],
        "src/customers.zyl"
    );
    assert_eq!(
        context["declarations"]["cruds"][0]["span"]["file"],
        "src/customers.zyl"
    );
    let result = run(
        &directory,
        &["module", "plan", "main.zyl", "src/customers.zyl"],
    );
    let repeated = run(
        &directory,
        &["module", "plan", "main.zyl", "src/customers.zyl"],
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
    assert_eq!(result.stdout, repeated.stdout);
    let document: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        document["plan"]["source_files"],
        serde_json::json!([
            "src/audit.zyl",
            "src/customers.zyl",
            "src/database.zyl",
            "src/helpers.zyl",
            "src/models.zyl"
        ])
    );
    assert_eq!(document["plan"]["database"]["required"], true);
    let dependencies = document["plan"]["resource_dependencies"]
        .as_array()
        .unwrap();
    assert!(dependencies.iter().any(|dependency| {
        dependency["from"] == "form:CustomerCreate"
            && dependency["to"] == "table:customers"
            && dependency["kind"] == "table"
    }));
    assert!(dependencies.iter().any(|dependency| {
        dependency["from"] == "form:CustomerCreate"
            && dependency["to"] == "table:audit_events"
            && dependency["kind"] == "sql_table"
            && dependency["to_module"] == "src/audit.zyl"
    }));
    assert!(dependencies.iter().any(|dependency| {
        dependency["from"] == "crud:Customer"
            && dependency["to"] == "table:audit_events"
            && dependency["kind"] == "sql_table"
            && dependency["to_module"] == "src/audit.zyl"
    }));
    assert!(dependencies.iter().any(|dependency| {
        dependency["from"] == "crud:Customer"
            && dependency["to"] == "table:customers"
            && dependency["kind"] == "table"
    }));
    assert!(dependencies.iter().any(|dependency| {
        dependency["from"] == "form:CustomerCreate"
            && dependency["to"] == "function:src/helpers.zyl::normalize_name"
            && dependency["kind"] == "call"
            && dependency["to_module"] == "src/helpers.zyl"
    }));
    assert!(dependencies.iter().any(|dependency| {
        dependency["from"] == "crud:Customer"
            && dependency["to"] == "function:src/helpers.zyl::audit_message"
            && dependency["kind"] == "call"
            && dependency["to_module"] == "src/helpers.zyl"
    }));
    assert!(dependencies.iter().any(|dependency| {
        dependency["kind"] == "database_configuration"
            && dependency["to_module"] == "src/database.zyl"
    }));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_plan_tracks_types_used_by_table_columns() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/schema.zyl\" as schema\nfn main() {}\n",
        ),
        (
            "src/schema.zyl",
            "import \"src/types.zyl\" as types\ntable customers { id: Id primary auto code: types::CustomerCode required }\n",
        ),
        (
            "src/types.zyl",
            "pub type CustomerCode = Int\n",
        ),
    ]);

    let check = run(&directory, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stdout)
    );
    let output = run(
        &directory,
        &["module", "plan", "main.zyl", "src/schema.zyl"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    let dependency = document["plan"]["resource_dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|dependency| {
            dependency["from"] == "table:customers"
                && dependency["to"] == "type:src/types.zyl::CustomerCode"
                && dependency["kind"] == "type"
        })
        .expect("table column type must be a semantic dependency");
    assert_eq!(dependency["from_module"], "src/schema.zyl");
    assert_eq!(dependency["to_module"], "src/types.zyl");
    assert!(document["plan"]["declaration_closure"]["declarations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|declaration| declaration == "src/types.zyl::type:CustomerCode"));

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_plan_tracks_types_used_by_form_and_crud_fields() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/customer_admin.zyl\" as admin\nfn main() {}\n",
        ),
        (
            "src/customer_admin.zyl",
            "import \"src/types.zyl\" as types\nform CustomerSearch { field code: types::SearchCode { label: \"Code\" } action refine { field minimum: types::MinimumCode { label: \"Minimum\" } } }\ntable customers { id: Id primary auto code: Int }\ncrud Customer -> customers { action reset_code { field code: types::ResetCode { label: \"Code\" } sql { UPDATE customers SET code = :code } } }\n",
        ),
        (
            "src/types.zyl",
            "pub type SearchCode = Int\npub type MinimumCode = Int\npub type ResetCode = Int\n",
        ),
    ]);

    let check = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(
        check.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );

    for (resource, aliases) in [
        (
            "form:CustomerSearch",
            ["SearchCode", "MinimumCode"].as_slice(),
        ),
        ("crud:Customer", ["ResetCode"].as_slice()),
    ] {
        let output = run(&directory, &["module", "plan", "main.zyl", resource]);
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let document: Value = serde_json::from_slice(&output.stdout).unwrap();
        let dependencies = document["plan"]["resource_dependencies"]
            .as_array()
            .unwrap();
        let declarations = document["plan"]["declaration_closure"]["declarations"]
            .as_array()
            .unwrap();
        for alias in aliases {
            let target = format!("type:src/types.zyl::{alias}");
            let dependency = dependencies
                .iter()
                .find(|dependency| {
                    dependency["from"] == resource
                        && dependency["to"] == target
                        && dependency["kind"] == "type"
                })
                .unwrap_or_else(|| panic!("{resource} must depend on field type {alias}"));
            assert_eq!(dependency["from_module"], "src/customer_admin.zyl");
            assert_eq!(dependency["to_module"], "src/types.zyl");
            let expected_declaration = format!("src/types.zyl::type:{alias}");
            assert!(declarations
                .iter()
                .any(|declaration| declaration.as_str() == Some(expected_declaration.as_str())));
        }
    }

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_plan_tracks_types_used_inside_function_bodies() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/logic.zyl\" as logic\nfn main() {}\n",
        ),
        (
            "src/logic.zyl",
            "import \"src/database.zyl\" as storage\nimport \"src/schema.zyl\" as schema\nimport \"src/models.zyl\" as models\npub fn load() -> Int {\n id: models::CustomerId? = None\n customer = models::Customer { name: \"Example\" }\n return 0\n}\npub fn count() -> Int uses Database(read) {\n customers = sql<models::Customer[]> { SELECT name FROM customers }\n return 0\n}\n",
        ),
        (
            "src/schema.zyl",
            "import \"src/database.zyl\" as storage\ntable customers { id: Id primary auto name: String(100) required access { read: [\"src/logic.zyl\"] } }\n",
        ),
        (
            "src/models.zyl",
            "pub type CustomerId = Int\npub struct Customer { name: String }\n",
        ),
        (
            "src/database.zyl",
            "database main { engine: mariadb database: \"customers\" }\n",
        ),
        (
            "zelyra.toml",
            "[project]\nname = \"module-type-closure\"\nversion = \"0.4.0-dev\"\nzelyra = \"0.1\"\n\n[capabilities]\ndatabase = true\n",
        ),
    ]);

    let check = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(
        check.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    let output = run(&directory, &["module", "plan", "main.zyl", "src/logic.zyl"]);
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    let dependencies = document["plan"]["resource_dependencies"]
        .as_array()
        .unwrap();
    for (function, target) in [
        (
            "function:src/logic.zyl::load",
            "type:src/models.zyl::CustomerId",
        ),
        (
            "function:src/logic.zyl::load",
            "record:src/models.zyl::Customer",
        ),
        (
            "function:src/logic.zyl::count",
            "record:src/models.zyl::Customer",
        ),
    ] {
        assert!(
            dependencies.iter().any(|dependency| {
                dependency["from"] == function
                    && dependency["to"] == target
                    && dependency["kind"] == "type"
                    && dependency["to_module"] == "src/models.zyl"
            }),
            "missing function type dependency {function} -> {target}"
        );
    }
    for declaration in [
        "src/models.zyl::type:CustomerId",
        "src/models.zyl::record:Customer",
    ] {
        assert!(document["plan"]["declaration_closure"]["declarations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| candidate == declaration));
    }

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn check_composes_imported_api_routes_and_authentication_configuration() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/security.zyl\" as security\nimport \"src/status_api.zyl\" as status_api\nimport \"src/database.zyl\" as storage\nfn main() {}\n",
        ),
        (
            "src/security.zyl",
            "import \"src/database.zyl\" as storage\nimport \"src/models.zyl\" as models\nauth users { table: users }\n",
        ),
        (
            "src/models.zyl",
            "table users { id: Id primary auto email: Email required password_hash: String(255) required access { read_write: [\"src/security.zyl\"] } }\n",
        ),
        (
            "src/database.zyl",
            "database main { engine: mariadb database: \"app\" }\n",
        ),
        (
            "src/status_api.zyl",
            "pub type StatusCode = Int\npub struct StatusPayload { status: String }\nstruct UnusedPayload { note: String }\nfn status(payload: StatusPayload) -> String { return \"ok\" }\nfn unusedHelper() -> String { return \"co-located\" }\napi GET \"/api/status\" { handler status requires auth permits \"status.read\" input { payload: StatusPayload } output String }\n",
        ),
    ]);
    let check = run(&directory, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let context = run(&directory, &["context", "main.zyl", "--format=json"]);
    assert!(
        context.status.success(),
        "{}",
        String::from_utf8_lossy(&context.stderr)
    );
    let context: Value = serde_json::from_slice(&context.stdout).unwrap();
    assert_eq!(
        context["declarations"]["apis"][0]["span"]["file"],
        "src/status_api.zyl"
    );
    assert_eq!(
        context["declarations"]["auth"][0]["span"]["file"],
        "src/security.zyl"
    );
    let plan = run(
        &directory,
        &["module", "plan", "main.zyl", "src/security.zyl"],
    );
    assert!(
        plan.status.success(),
        "{}",
        String::from_utf8_lossy(&plan.stderr)
    );
    let plan: Value = serde_json::from_slice(&plan.stdout).unwrap();
    assert_eq!(
        plan["plan"]["source_files"],
        serde_json::json!(["src/database.zyl", "src/models.zyl", "src/security.zyl"])
    );
    assert!(plan["plan"]["resource_dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|dependency| {
            dependency["from"] == "auth:users"
                && dependency["to"] == "table:users"
                && dependency["kind"] == "auth_table"
        }));
    let plan = run(
        &directory,
        &["module", "plan", "main.zyl", "src/status_api.zyl"],
    );
    assert!(
        plan.status.success(),
        "{}",
        String::from_utf8_lossy(&plan.stderr)
    );
    let plan: Value = serde_json::from_slice(&plan.stdout).unwrap();
    assert_eq!(
        plan["plan"]["source_files"],
        serde_json::json!([
            "src/database.zyl",
            "src/models.zyl",
            "src/security.zyl",
            "src/status_api.zyl"
        ])
    );
    let dependencies = plan["plan"]["resource_dependencies"].as_array().unwrap();
    assert!(dependencies.iter().any(|dependency| {
        dependency["from"] == "api:GET /api/status"
            && dependency["to"] == "auth:users"
            && dependency["kind"] == "authentication"
    }));
    assert!(dependencies.iter().any(|dependency| {
        dependency["from"] == "auth:users"
            && dependency["to"] == "table:users"
            && dependency["kind"] == "auth_table"
    }));
    let plan = run(
        &directory,
        &["module", "plan", "main.zyl", "api:GET /api/status"],
    );
    let repeated_plan = run(
        &directory,
        &["module", "plan", "main.zyl", "api:GET /api/status"],
    );
    assert!(
        plan.status.success(),
        "{}",
        String::from_utf8_lossy(&plan.stderr)
    );
    assert_eq!(plan.stdout, repeated_plan.stdout);
    let plan: Value = serde_json::from_slice(&plan.stdout).unwrap();
    assert_eq!(plan["plan"]["selection_kind"], "resource");
    assert_eq!(plan["plan"]["selected_resource"], "api:GET /api/status");
    assert_eq!(plan["plan"]["selected_module"], "src/status_api.zyl");
    let api_module = plan["plan"]["modules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|module| module["path"] == "src/status_api.zyl")
        .unwrap();
    assert_eq!(
        api_module["declarations"],
        serde_json::json!([
            "api:GET /api/status",
            "function:status",
            "function:unusedHelper",
            "record:StatusPayload",
            "record:UnusedPayload",
            "type:StatusCode"
        ])
    );
    assert_eq!(
        plan["plan"]["declaration_closure"]["declarations"],
        serde_json::json!([
            "src/database.zyl::database:main",
            "src/models.zyl::table:users",
            "src/security.zyl::auth:users",
            "src/status_api.zyl::api:GET /api/status",
            "src/status_api.zyl::function:status",
            "src/status_api.zyl::record:StatusPayload"
        ])
    );
    assert_eq!(
        plan["plan"]["declaration_closure"]["additional_declarations_in_included_source_files"],
        serde_json::json!([
            "src/status_api.zyl::function:unusedHelper",
            "src/status_api.zyl::record:UnusedPayload",
            "src/status_api.zyl::type:StatusCode"
        ])
    );
    assert_eq!(plan["plan"]["declaration_closure"]["complete"], false);
    assert!(plan["plan"]["declaration_closure"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|edge| {
            edge["from"] == "api:GET /api/status"
                && edge["to"] == "auth:users"
                && edge["kind"] == "authentication"
        }));
    assert!(plan["plan"]["declaration_closure"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|edge| {
            edge["from"] == "api:GET /api/status"
                && edge["to"] == "record:src/status_api.zyl::StatusPayload"
                && edge["kind"] == "type"
        }));
    assert!(plan["plan"]["declaration_closure"]["configuration_edges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|edge| { edge["kind"] == "database_configuration" && edge["to"] == "database:main" }));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn serve_dispatches_an_imported_api_to_its_module_handler() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/status_api.zyl\" as status_api\nfn main() {}\n",
        ),
        (
            "src/status_api.zyl",
            "fn status() -> String { return \"imported api works\" }\napi GET \"/api/status\" { handler status output String }\n",
        ),
    ]);
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let address = format!("127.0.0.1:{port}");
    let mut server = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .current_dir(&directory)
        .args(["serve", "main.zyl", &address])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let response = get_until_response(
        &address,
        b"GET /api/status HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    let _ = server.kill();
    let server_output = server.wait_with_output().unwrap();
    assert!(
        response.contains("200 OK"),
        "{response}\nserver stderr: {}",
        String::from_utf8_lossy(&server_output.stderr)
    );
    assert!(response.contains("imported api works"), "{response}");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn module_plan_rejects_modules_outside_the_reachable_project_graph() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/invoices.zyl\" as invoices\nfn main() {}\n",
        ),
        ("src/invoices.zyl", "pub fn count() -> Int { return 0 }\n"),
        ("src/unrelated.zyl", "pub fn other() -> Int { return 1 }\n"),
    ]);
    let result = run(
        &directory,
        &["module", "plan", "main.zyl", "src/unrelated.zyl"],
    );
    assert!(!result.status.success());
    let document: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(document["success"], false);
    assert!(document["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|diagnostic| diagnostic["code"] == "E-MOD-013"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn context_reports_imported_table_spans_against_their_own_source_files() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/database.zyl\" as storage\nimport \"src/invoices.zyl\" as invoices\ntable customers { id: Id primary auto name: String(100) required }\nfn main() {}\n",
        ),
        (
            "src/database.zyl",
            "database main { engine: mariadb database: \"context-test\" }\n",
        ),
        (
            "src/invoices.zyl",
            "table invoices { id: Id primary auto number: String(40) required }\n",
        ),
    ]);
    let output = run(&directory, &["context", "main.zyl", "--format=json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    let tables = document["declarations"]["tables"].as_array().unwrap();
    let imported = tables
        .iter()
        .find(|table| table["name"] == "invoices")
        .unwrap();
    assert_eq!(imported["span"]["file"], "src/invoices.zyl");
    assert_eq!(imported["span"]["start"]["line"], 1);
    assert_eq!(imported["fields"][1]["span"]["file"], "src/invoices.zyl");
    assert_eq!(
        document["declarations"]["databases"][0]["span"]["file"],
        "src/database.zyl"
    );
    let entry = tables
        .iter()
        .find(|table| table["name"] == "customers")
        .unwrap();
    assert_eq!(entry["span"]["file"], "main.zyl");
    assert_eq!(entry["span"]["start"]["line"], 3);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn imported_tableviews_are_type_checked_and_served_from_the_composed_schema() {
    let Ok(base_database_url) = std::env::var("ZELYRA_MODULE_IMPORTS_DATABASE_URL") else {
        eprintln!("skipping imported tableview serving test: MariaDB test URL is not configured");
        return;
    };
    let database_root = base_database_url
        .rsplit_once('/')
        .map(|(root, _)| root)
        .expect("configured MariaDB URL must include a database");
    let database_name = format!(
        "zelyra_module_imports_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let database_url = format!("{database_root}/{database_name}");
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/invoices.zyl\" as invoices\nfn main() {}\n",
        ),
        (
            "src/invoices.zyl",
            r#"database main {
    engine: mariadb
}

table invoices {
    id: Id primary auto
    number: String(40) required
}

struct InvoiceRow {
    id: Id
    number: String
}

tableview InvoiceOverview {
    source sql<InvoiceRow[]> {
        SELECT id, number FROM invoices
    }
    columns { id number }
}
"#,
        ),
    ]);
    let bootstrap = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .current_dir(&directory)
        .env("DATABASE_URL", &database_url)
        .args(["db", "bootstrap", "main.zyl"])
        .output()
        .unwrap();
    assert!(
        bootstrap.status.success(),
        "{}",
        String::from_utf8_lossy(&bootstrap.stderr)
    );
    let inserted = run_test_database_sql(
        &database_url,
        "INSERT INTO invoices (number) VALUES ('INV-IMPORT-001')",
    );
    assert!(
        inserted.status.success(),
        "{}",
        String::from_utf8_lossy(&inserted.stderr)
    );

    let context = run(&directory, &["context", "main.zyl", "--format=json"]);
    assert!(
        context.status.success(),
        "{}",
        String::from_utf8_lossy(&context.stderr)
    );
    let document: Value = serde_json::from_slice(&context.stdout).unwrap();
    let tableview = document["declarations"]["tableviews"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tableview| tableview["name"] == "InvoiceOverview")
        .unwrap();
    assert_eq!(tableview["span"]["file"], "src/invoices.zyl");

    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let address = format!("127.0.0.1:{port}");
    let mut server = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .current_dir(&directory)
        .env("DATABASE_URL", &database_url)
        .args(["serve", "main.zyl", &address])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let response = get_until_response(
        &address,
        b"GET /views/invoiceoverview HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    let _ = server.kill();
    let server_output = server.wait_with_output().unwrap();
    assert!(
        response.contains("200 OK"),
        "{response}\nserver stderr: {}",
        String::from_utf8_lossy(&server_output.stderr)
    );
    assert!(response.contains("Number"), "{response}");
    assert!(response.contains("INV-IMPORT-001"), "{response}");
    drop_test_database(&database_url);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn duplicate_imported_tableviews_report_the_duplicate_module_source() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/a.zyl\" as a\nimport \"src/b.zyl\" as b\nfn main() {}\n",
        ),
        (
            "src/a.zyl",
            "tableview Shared { source sql<Int[]> { SELECT 1 AS value } columns { value } }\n",
        ),
        (
            "src/b.zyl",
            "tableview Shared { source sql<Int[]> { SELECT 2 AS value } columns { value } }\n",
        ),
    ]);
    let result = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(!result.status.success());
    let document: Value = serde_json::from_slice(&result.stdout).unwrap();
    let diagnostics = document["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic["code"] == "E-MOD-011" && diagnostic["file"] == "src/b.zyl"
        }),
        "{document}"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn check_rejects_import_cycles_and_private_symbols_before_execution() {
    let cycle = project(&[
        ("main.zyl", "import \"src/a.zyl\" as a\nfn main() {}\n"),
        (
            "src/a.zyl",
            "import \"main.zyl\" as app\npub fn value() -> Int { return 1 }\n",
        ),
    ]);
    let cycle_result = run(&cycle, &["check", "main.zyl"]);
    assert!(!cycle_result.status.success());
    assert!(String::from_utf8_lossy(&cycle_result.stderr).contains("E-MOD-003"));
    fs::remove_dir_all(cycle).unwrap();

    let private = project(&[
        (
            "main.zyl",
            "import \"src/a.zyl\" as a\nfn main() { print(a::secret()) }\n",
        ),
        ("src/a.zyl", "fn secret() -> Int { return 1 }\n"),
    ]);
    let private_result = run(&private, &["check", "main.zyl"]);
    assert!(!private_result.status.success());
    assert!(String::from_utf8_lossy(&private_result.stderr).contains("E-MOD-007"));
    fs::remove_dir_all(private).unwrap();
}

#[test]
fn imported_views_and_components_are_composed_and_served() {
    let directory = project(&[
        (
            "main.zyl",
            r#"import "src/shell.zyl" as shell
page "/" {
    view: Shell
    html {
        <p>Entry page content</p>
        <Brand label="Used by entry" />
    }
}
fn main() {}
"#,
        ),
        (
            "src/shell.zyl",
            r#"pub component Brand {
    props {
        label: String
    }
    html {
        <strong>{label}</strong>
    }
}
pub view Shell {
    html {
        <html><body><header><Brand label="Shared module" /></header><main><slot /></main></body></html>
    }
}
"#,
        ),
    ]);

    let check = run(&directory, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let context = run(&directory, &["context", "main.zyl", "--format=json"]);
    assert!(
        context.status.success(),
        "{}",
        String::from_utf8_lossy(&context.stderr)
    );
    let document: Value = serde_json::from_slice(&context.stdout).unwrap();
    let shell = document["declarations"]["views"]
        .as_array()
        .unwrap()
        .iter()
        .find(|view| view["name"] == "Shell")
        .unwrap();
    let module = document["modules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|module| module["path"] == "src/shell.zyl")
        .unwrap();
    assert!(module["exports"]
        .as_array()
        .unwrap()
        .iter()
        .any(|export| { export["kind"] == "view" && export["name"] == "Shell" }));
    assert!(module["exports"]
        .as_array()
        .unwrap()
        .iter()
        .any(|export| { export["kind"] == "component" && export["name"] == "Brand" }));
    assert_eq!(shell["span"]["file"], "src/shell.zyl");
    let brand = document["declarations"]["components"]
        .as_array()
        .unwrap()
        .iter()
        .find(|component| component["name"] == "Brand")
        .unwrap();
    assert_eq!(brand["span"]["file"], "src/shell.zyl");

    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let address = format!("127.0.0.1:{port}");
    let mut server = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .current_dir(&directory)
        .args(["serve", "main.zyl", &address])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    let response = get_until_response(
        &address,
        b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    let _ = server.kill();
    let _ = server.wait();

    assert!(response.contains("200 OK"), "{response}");
    assert!(
        response.contains("<strong>Shared module</strong>"),
        "{response}"
    );
    assert!(response.contains("Entry page content"), "{response}");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn imported_ui_resources_must_be_public_and_explicitly_imported() {
    let private_view = project(&[
        (
            "main.zyl",
            "import \"src/ui.zyl\" as ui\npage \"/\" { view: Shell html { <p>Home</p> } }\nfn main() {}\n",
        ),
        (
            "src/ui.zyl",
            "view Shell { html { <main><slot /></main> } }\n",
        ),
    ]);
    let private_result = run(&private_view, &["check", "main.zyl"]);
    assert!(!private_result.status.success());
    assert!(String::from_utf8_lossy(&private_result.stderr).contains("E-MOD-007"));
    assert!(String::from_utf8_lossy(&private_result.stderr).contains("pub view"));
    fs::remove_dir_all(private_view).unwrap();

    let private_component = project(&[
        (
            "main.zyl",
            "import \"src/ui.zyl\" as ui\npage \"/\" { view: Shell html { <Badge /> } }\nfn main() {}\n",
        ),
        (
            "src/ui.zyl",
            "component Badge { html { <strong>Badge</strong> } }\npub view Shell { html { <main><slot /></main> } }\n",
        ),
    ]);
    let component_result = run(&private_component, &["check", "main.zyl"]);
    assert!(!component_result.status.success());
    assert!(String::from_utf8_lossy(&component_result.stderr).contains("E-MOD-007"));
    assert!(String::from_utf8_lossy(&component_result.stderr).contains("pub component"));
    fs::remove_dir_all(private_component).unwrap();

    let undeclared_dependency = project(&[
        (
            "main.zyl",
            "import \"src/pages.zyl\" as pages\nimport \"src/ui.zyl\" as ui\nfn main() {}\n",
        ),
        (
            "src/pages.zyl",
            "page \"/\" { view: Shell html { <p>Home</p> } }\n",
        ),
        (
            "src/ui.zyl",
            "pub view Shell { html { <main><slot /></main> } }\n",
        ),
    ]);
    let dependency_result = run(&undeclared_dependency, &["check", "main.zyl"]);
    assert!(!dependency_result.status.success());
    assert!(String::from_utf8_lossy(&dependency_result.stderr).contains("E-MOD-020"));
    fs::remove_dir_all(undeclared_dependency).unwrap();

    let private_crud_layout = project(&[
        (
            "main.zyl",
            "import \"src/admin.zyl\" as admin\nimport \"src/ui.zyl\" as ui\nfn main() {}\n",
        ),
        (
            "src/admin.zyl",
            "import \"src/ui.zyl\" as ui\ncrud Customer -> customers { layout: Shell }\n",
        ),
        (
            "src/ui.zyl",
            "view Shell { html { <main><slot /></main> } }\n",
        ),
    ]);
    let crud_result = run(&private_crud_layout, &["check", "main.zyl"]);
    assert!(!crud_result.status.success());
    assert!(String::from_utf8_lossy(&crud_result.stderr).contains("E-MOD-007"));
    assert!(String::from_utf8_lossy(&crud_result.stderr).contains("pub view"));
    fs::remove_dir_all(private_crud_layout).unwrap();

    let private_crud_slot_component = project(&[
        (
            "main.zyl",
            "import \"src/admin.zyl\" as admin\nimport \"src/ui.zyl\" as ui\nfn main() {}\n",
        ),
        (
            "src/admin.zyl",
            r#"import "src/ui.zyl" as ui
crud Customer -> customers {
    layout: Shell
    slots {
        header {
            html { <Badge /> }
        }
    }
}
"#,
        ),
        (
            "src/ui.zyl",
            "pub view Shell { html { <main><slot name=\"header\" /></main> } }\ncomponent Badge { html { <strong>Badge</strong> } }\n",
        ),
    ]);
    let slot_result = run(&private_crud_slot_component, &["check", "main.zyl"]);
    assert!(!slot_result.status.success());
    assert!(String::from_utf8_lossy(&slot_result.stderr).contains("E-MOD-007"));
    assert!(String::from_utf8_lossy(&slot_result.stderr).contains("pub component"));
    fs::remove_dir_all(private_crud_slot_component).unwrap();

    let deterministic_component_diagnostic = project(&[
        (
            "main.zyl",
            "import \"src/ui.zyl\" as ui\npage \"/\" { html { <Zulu /><Alpha /> } }\nfn main() {}\n",
        ),
        (
            "src/ui.zyl",
            "component Zulu { html { <strong>Z</strong> } }\ncomponent Alpha { html { <strong>A</strong> } }\n",
        ),
    ]);
    let deterministic_result = run(&deterministic_component_diagnostic, &["check", "main.zyl"]);
    assert!(!deterministic_result.status.success());
    let stderr = String::from_utf8_lossy(&deterministic_result.stderr);
    assert!(stderr.contains("E-MOD-007"));
    assert!(stderr.contains("component `Alpha`"), "{stderr}");
    fs::remove_dir_all(deterministic_component_diagnostic).unwrap();
}

#[test]
fn imported_crud_can_use_public_layout_and_component_from_transitive_imports() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/admin.zyl\" as admin\nfn main() {}\n",
        ),
        (
            "src/admin.zyl",
            r#"import "src/models.zyl" as models
import "src/shells.zyl" as shells
crud Customer -> customers {
    layout: CustomerShell
    slots {
        header {
            html { <CustomerBadge label="Customers" /> }
        }
    }
}
"#,
        ),
        (
            "src/models.zyl",
            "table customers { id: Id primary auto name: String(100) required access { read_write: [\"src/admin.zyl\"] } }\n",
        ),
        (
            "src/shells.zyl",
            "import \"src/ui.zyl\" as ui\npub view CustomerShell { html { <main><slot name=\"header\" /><slot /></main> } }\n",
        ),
        (
            "src/ui.zyl",
            "pub component CustomerBadge { props { label: String } html { <strong>{label}</strong> } }\n",
        ),
    ]);
    let result = run(&directory, &["check", "main.zyl"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn imported_pages_are_type_checked_and_served_as_application_routes() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/reports.zyl\" as reports\nfn main() {}\n",
        ),
        (
            "src/reports.zyl",
            r#"type ReportId = Int

page "/reports/{id}" {
    input {
        term: ReportId
    }
    html {
        <h1>Imported report {term}</h1>
    }
}
"#,
        ),
    ]);
    let check = run(&directory, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let context = run(&directory, &["context", "main.zyl", "--format=json"]);
    assert!(context.status.success());
    let document: Value = serde_json::from_slice(&context.stdout).unwrap();
    let page = document["declarations"]["pages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|page| page["path"] == "/reports/{id}")
        .unwrap();
    assert_eq!(page["span"]["file"], "src/reports.zyl");
    assert_eq!(page["inputs"][0]["name"], "term");
    assert_eq!(page["inputs"][0]["type"], "src/reports.zyl::ReportId");

    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let address = format!("127.0.0.1:{port}");
    let mut server = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .current_dir(&directory)
        .args(["serve", "main.zyl", &address])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let response = get_until_response(
        &address,
        b"GET /reports/42?term=7 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    let _ = server.kill();
    let server_output = server.wait_with_output().unwrap();
    assert!(
        response.contains("200 OK"),
        "{response}\nserver stderr: {}",
        String::from_utf8_lossy(&server_output.stderr)
    );
    assert!(response.contains("Imported report 7"), "{response}");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn duplicate_imported_components_report_their_own_source_file() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/a.zyl\" as a\nimport \"src/b.zyl\" as b\nfn main() {}\n",
        ),
        (
            "src/a.zyl",
            "component Badge { html { <strong>A</strong> } }\n",
        ),
        (
            "src/b.zyl",
            "component Badge { html { <strong>B</strong> } }\n",
        ),
    ]);
    let result = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(!result.status.success());
    let document: Value = serde_json::from_slice(&result.stdout).unwrap();
    let diagnostics = document["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|item| { item["code"] == "E-VIEW-005" && item["file"] == "src/b.zyl" }),
        "{document}"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn duplicate_imported_views_report_their_own_source_file() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/a.zyl\" as a\nimport \"src/b.zyl\" as b\nfn main() {}\n",
        ),
        (
            "src/a.zyl",
            "view Shell { html { <main><slot /></main> } }\n",
        ),
        (
            "src/b.zyl",
            "view Shell { html { <main><slot /></main> } }\n",
        ),
    ]);
    let result = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(!result.status.success());
    let document: Value = serde_json::from_slice(&result.stdout).unwrap();
    let diagnostics = document["diagnostics"].as_array().unwrap();
    assert!(diagnostics
        .iter()
        .any(|item| { item["code"] == "E-VIEW-001" && item["file"] == "src/b.zyl" }));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn imported_capabilities_remain_subject_to_project_grants() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/console.zyl\" as console\nfn main() uses Console { console::prompt() }\n",
        ),
        (
            "src/console.zyl",
            "pub fn prompt() uses Console { read_console(\"Name: \") }\n",
        ),
        (
            "zelyra.toml",
            "[project]\nname = \"module-test\"\nversion = \"0.1.0\"\nzelyra = \"0.1\"\n\n[capabilities]\nconsole = false\n",
        ),
    ]);
    let result = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(!result.status.success());
    let document: Value = serde_json::from_slice(&result.stdout).unwrap();
    let diagnostics = document["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic["code"] == "E-CAP-001" && diagnostic["file"] == "src/console.zyl"
    }));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn scoped_database_effects_propagate_across_imported_modules() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/reader.zyl\" as reader\nfn main() uses Database(read) { reader::readiness() }\n",
        ),
        (
            "src/reader.zyl",
            "pub fn readiness() -> Int uses Database(read) { return sql<Int> { SELECT 1 } }\n",
        ),
        (
            "zelyra.toml",
            "[project]\nname = \"module-test\"\nversion = \"0.4.0-dev\"\nzelyra = \"0.1\"\n\n[capabilities]\ndatabase_read = true\ndatabase_write = false\n",
        ),
    ]);

    let allowed = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(
        allowed.status.success(),
        "{}",
        String::from_utf8_lossy(&allowed.stdout)
    );

    fs::write(
        directory.join("zelyra.toml"),
        "[project]\nname = \"module-test\"\nversion = \"0.4.0-dev\"\nzelyra = \"0.1\"\n\n[capabilities]\ndatabase_read = false\ndatabase_write = true\n",
    )
    .unwrap();
    let denied = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(!denied.status.success());
    let document: Value = serde_json::from_slice(&denied.stdout).unwrap();
    assert!(document["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|diagnostic| diagnostic["code"] == "E-CAP-001"
            && diagnostic["message"]
                .as_str()
                .is_some_and(|message| message.contains("database_read"))));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn check_composes_a_project_database_from_an_imported_configuration_module() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/database.zyl\" as storage\nimport \"src/invoices.zyl\" as invoices\nfn main() uses Database(read) { invoices::count() }\n",
        ),
        (
            "src/database.zyl",
            "database main { engine: mariadb database: \"invoices\" }\n",
        ),
        (
            "src/invoices.zyl",
            "import \"src/database.zyl\" as storage\ntable invoices { id: Id primary auto number: String(40) required total: Decimal required }\npub fn count() -> Int uses Database(read) { rows = sql<Invoice[]> { SELECT id, number, total FROM invoices } return 0 }\n",
        ),
        (
            "zelyra.toml",
            "[project]\nname = \"module-test\"\nversion = \"0.4.0-dev\"\nzelyra = \"0.1\"\n\n[capabilities]\ndatabase = true\n",
        ),
    ]);
    let check = run(&directory, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let impact = run(&directory, &["impact", "main.zyl", "--format=json"]);
    let impact_again = run(&directory, &["impact", "main.zyl", "--format=json"]);
    assert!(
        impact.status.success(),
        "{}",
        String::from_utf8_lossy(&impact.stderr)
    );
    assert_eq!(impact.stdout, impact_again.stdout);
    let document: Value = serde_json::from_slice(&impact.stdout).unwrap();
    let modules = document["impact"]["modules"].as_array().unwrap();
    assert_eq!(modules.len(), 3);
    assert!(modules.iter().any(|module| {
        module["path"] == "src/invoices.zyl"
            && module["imports"]
                .as_array()
                .unwrap()
                .iter()
                .any(|import| import["path"] == "src/database.zyl")
    }));
    assert!(document["impact"]["references"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| {
            reference["from"] == "module:src/invoices.zyl"
                && reference["to"] == "module:src/database.zyl"
                && reference["kind"] == "module_import"
        }));
    let tables = document["impact"]["tables"].as_array().unwrap();
    let invoice_table = tables
        .iter()
        .find(|table| table["name"] == "invoices")
        .unwrap();
    assert_eq!(invoice_table["span"]["file"], "src/invoices.zyl");
    let sql = document["impact"]["sql"].as_array().unwrap();
    assert_eq!(sql.len(), 1);
    assert_eq!(sql[0]["owner"], "src/invoices.zyl::count");
    assert_eq!(sql[0]["span"]["file"], "src/invoices.zyl");
    assert_eq!(sql[0]["span"]["start"]["line"], 3);
    assert_eq!(sql[0]["tables"][0], "invoices");
    let focused = run(
        &directory,
        &[
            "impact",
            "main.zyl",
            "--symbol",
            "table:invoices",
            "--format=json",
        ],
    );
    assert!(focused.status.success());
    let focused: Value = serde_json::from_slice(&focused.stdout).unwrap();
    assert_eq!(focused["impact"]["references"].as_array().unwrap().len(), 1);
    let focused_module = run(
        &directory,
        &[
            "impact",
            "main.zyl",
            "--symbol",
            "module:src/invoices.zyl",
            "--format=json",
        ],
    );
    assert!(focused_module.status.success());
    let focused_module: Value = serde_json::from_slice(&focused_module.stdout).unwrap();
    assert_eq!(focused_module["impact"]["focus"], "module:src/invoices.zyl");
    assert!(focused_module["impact"]["references"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference["kind"] == "module_import"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn database_consumers_must_import_the_provider_module_through_their_own_graph() {
    let allowed = project(&[
        (
            "main.zyl",
            "import \"src/service.zyl\" as service\nfn main() uses Database(read) { service::count() }\n",
        ),
        (
            "src/service.zyl",
            "import \"src/storage_facade.zyl\" as storage\ntable things { id: Id primary auto }\npub fn count() -> Int uses Database(read) { rows = sql<Int[]> { SELECT id FROM things } return 0 }\n",
        ),
        (
            "src/storage_facade.zyl",
            "import \"src/database.zyl\" as db\n",
        ),
        (
            "src/database.zyl",
            "database main { engine: mariadb database: \"things\" }\n",
        ),
        (
            "zelyra.toml",
            "[project]\nname = \"database-boundary\"\nversion = \"0.4.0-dev\"\nzelyra = \"0.1\"\n\n[capabilities]\ndatabase = true\n",
        ),
    ]);
    let allowed_check = run(&allowed, &["check", "main.zyl", "--format=json"]);
    assert!(
        allowed_check.status.success(),
        "{}",
        String::from_utf8_lossy(&allowed_check.stdout)
    );
    fs::remove_dir_all(allowed).unwrap();

    let denied = project(&[
        (
            "main.zyl",
            "import \"src/service.zyl\" as service\nimport \"src/database.zyl\" as db\nfn main() uses Database(read) { service::count() }\n",
        ),
        (
            "src/service.zyl",
            "table things { id: Id primary auto }\npub fn count() -> Int uses Database(read) { rows = sql<Int[]> { SELECT id FROM things } return 0 }\n",
        ),
        (
            "src/database.zyl",
            "database main { engine: mariadb database: \"things\" }\n",
        ),
        (
            "zelyra.toml",
            "[project]\nname = \"database-boundary\"\nversion = \"0.4.0-dev\"\nzelyra = \"0.1\"\n\n[capabilities]\ndatabase = true\n",
        ),
    ]);
    let denied_check = run(&denied, &["check", "main.zyl", "--format=json"]);
    assert!(!denied_check.status.success());
    let diagnostics: Value = serde_json::from_slice(&denied_check.stdout).unwrap();
    let boundary = diagnostics["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "E-MOD-022")
        .expect("a main-module import must not silently configure an unrelated consumer");
    assert_eq!(boundary["file"], "src/service.zyl");
    assert!(boundary["message"]
        .as_str()
        .unwrap()
        .contains("src/database.zyl"));
    fs::remove_dir_all(denied).unwrap();

    let misplaced = project(&[
        (
            "main.zyl",
            "import \"src/service.zyl\" as service\ndatabase main { engine: mariadb database: \"things\" }\nfn main() uses Database(read) { service::count() }\n",
        ),
        (
            "src/service.zyl",
            "table things { id: Id primary auto }\npub fn count() -> Int uses Database(read) { rows = sql<Int[]> { SELECT id FROM things } return 0 }\n",
        ),
        (
            "zelyra.toml",
            "[project]\nname = \"database-boundary\"\nversion = \"0.4.0-dev\"\nzelyra = \"0.1\"\n\n[capabilities]\ndatabase = true\n",
        ),
    ]);
    let misplaced_check = run(&misplaced, &["check", "main.zyl", "--format=json"]);
    assert!(!misplaced_check.status.success());
    let diagnostics: Value = serde_json::from_slice(&misplaced_check.stdout).unwrap();
    let boundary = diagnostics["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "E-MOD-022")
        .expect("an entry-local database declaration cannot serve imported modules");
    assert!(boundary["message"]
        .as_str()
        .unwrap()
        .contains("move `database main` from the entry module"));
    fs::remove_dir_all(misplaced).unwrap();
}

#[test]
fn check_rejects_more_than_one_database_across_the_project_graph() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/database.zyl\" as storage\ndatabase main { engine: mariadb database: \"invoices\" }\nfn main() {}\n",
        ),
        (
            "src/database.zyl",
            "database reports { engine: mariadb database: \"reports\" }\n",
        ),
    ]);
    let check = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(!check.status.success());
    let document: Value = serde_json::from_slice(&check.stdout).unwrap();
    assert_eq!(document["success"], false);
    assert!(document["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item["code"] == "E-DB-001"
                && item["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("only one database definition")
        }));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn check_resolves_relations_between_tables_in_imported_modules() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/departments.zyl\" as departments\nimport \"src/machines.zyl\" as machines\nfn main() {}\n",
        ),
        (
            "src/departments.zyl",
            "table departments { id: Id primary auto name: String(100) required }\n",
        ),
        (
            "src/machines.zyl",
            "import \"src/departments.zyl\" as departments\ntable machines { id: Id primary auto name: String(100) required department: Department required }\n",
        ),
    ]);
    let check = run(&directory, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn deeply_nested_module_calls_stop_at_the_controlled_runtime_limit() {
    const MODULES: usize = 40;
    let mut files = vec![(
        "main.zyl".to_owned(),
        "import \"src/m0.zyl\" as m0\nfn main() { print(m0::call()) }".to_owned(),
    )];
    for index in 0..MODULES {
        let body = if index + 1 == MODULES {
            "pub fn call() -> Int { return 42 }".to_owned()
        } else {
            format!(
                "import \"src/m{}.zyl\" as next\npub fn call() -> Int {{ return next::call() }}",
                index + 1
            )
        };
        files.push((format!("src/m{index}.zyl"), body));
    }
    let borrowed = files
        .iter()
        .map(|(path, source)| (path.as_str(), source.as_str()))
        .collect::<Vec<_>>();
    let directory = project(&borrowed);

    let check = run(&directory, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "module chain should type-check: {}",
        String::from_utf8_lossy(&check.stderr)
    );
    let execution = run(&directory, &["run", "main.zyl"]);
    let stderr = String::from_utf8_lossy(&execution.stderr);
    assert!(!execution.status.success(), "deep module chain should stop");
    assert!(
        stderr.contains("function call depth limit exceeded"),
        "{stderr}"
    );
    assert!(
        stderr.contains("main.zyl"),
        "diagnostic should include a source location: {stderr}"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn package_dependencies_can_be_planned_and_bundled_with_a_filtered_locked_graph() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"pkg:money_core\" as cash\nfn main() { print(cash::amount()) }\n",
        ),
        (
            "zelyra.toml",
            "[package]\nname = \"business_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.money_core]\npath = \"packages/money\"\nversion = \"^1.2\"\n\n[dependencies.unused]\npath = \"packages/unused\"\nversion = \"^1.0\"\n",
        ),
        (
            "packages/money/zelyra.toml",
            "[package]\nname = \"money\"\nversion = \"1.2.3\"\nentry = \"src/lib.zyl\"\n\n[dependencies.currency]\npath = \"../currency\"\nversion = \"^1.0\"\n",
        ),
        (
            "packages/money/src/lib.zyl",
            "import \"pkg:currency\" as currency\nimport \"src/math.zyl\" as math\npub fn amount() -> Int { return math::plus(currency::cents()) }\n",
        ),
        (
            "packages/money/src/math.zyl",
            "pub fn plus(value: Int) -> Int { return value + 100 }\n",
        ),
        (
            "packages/money/src/unreferenced.zyl",
            "pub fn extra() -> Int { return 7 }\n",
        ),
        (
            "packages/currency/zelyra.toml",
            "[package]\nname = \"currency\"\nversion = \"1.0.2\"\nentry = \"src/lib.zyl\"\n",
        ),
        (
            "packages/currency/src/lib.zyl",
            "pub fn cents() -> Int { return 25 }\n",
        ),
        (
            "packages/unused/zelyra.toml",
            "[package]\nname = \"unused\"\nversion = \"1.0.1\"\nentry = \"src/lib.zyl\"\n",
        ),
        (
            "packages/unused/src/lib.zyl",
            "pub fn unused() -> Int { return 0 }\n",
        ),
    ]);
    let lock = run(&directory, &["package", "lock", "main.zyl"]);
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );

    let plan = run(
        &directory,
        &["module", "plan", "main.zyl", "pkg:money_core"],
    );
    assert!(
        plan.status.success(),
        "package selection should resolve to its checked entry: {}",
        String::from_utf8_lossy(&plan.stdout)
    );
    let plan: Value = serde_json::from_slice(&plan.stdout).unwrap();
    assert_eq!(plan["plan"]["selection_kind"], "package");
    assert_eq!(
        plan["plan"]["selected_module"],
        "packages/money/src/lib.zyl"
    );
    assert_eq!(
        plan["plan"]["source_files"],
        serde_json::json!([
            "packages/currency/src/lib.zyl",
            "packages/money/src/lib.zyl",
            "packages/money/src/math.zyl"
        ])
    );

    let output_parent = std::env::temp_dir().join(format!(
        "zelyra-cli-package-bundle-{}-{}",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&output_parent).unwrap();
    let output = output_parent.join("bundle");
    let bundle = run(
        &directory,
        &[
            "module",
            "bundle",
            "main.zyl",
            "pkg:money_core",
            "--output",
            output.to_str().unwrap(),
        ],
    );
    assert!(
        bundle.status.success(),
        "{}",
        String::from_utf8_lossy(&bundle.stdout)
    );
    let bundled_manifest = fs::read_to_string(output.join("zelyra.toml")).unwrap();
    assert!(bundled_manifest.contains("[dependencies.money_core]"));
    assert!(!bundled_manifest.contains("[dependencies.money]\n"));
    assert!(bundled_manifest.contains("path = \"packages/money\""));
    assert!(!bundled_manifest.contains("unused"));
    assert!(output.join("zelyra.lock").is_file());
    assert!(output.join("packages/money/zelyra.toml").is_file());
    let bundled_package_manifest =
        fs::read_to_string(output.join("packages/money/zelyra.toml")).unwrap();
    assert!(bundled_package_manifest.contains("path = \"../currency\""));
    assert!(output.join("packages/money/src/unreferenced.zyl").is_file());
    assert!(!output.join("packages/unused").exists());

    let bundled_lock = run(&output, &["package", "lock", "main.zyl", "--check"]);
    assert!(
        bundled_lock.status.success(),
        "bundled dependency lock should be current: {}",
        String::from_utf8_lossy(&bundled_lock.stderr)
    );
    let check = run(&output, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "package-backed bundle should compile: {}",
        String::from_utf8_lossy(&check.stderr)
    );

    fs::remove_dir_all(directory).unwrap();
    fs::remove_dir_all(output_parent).unwrap();
}

#[test]
fn package_database_provider_and_business_module_export_as_docker_app() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"pkg:billing\" as billing\nfn main() uses Database(read) { print(billing::invoice_count()) }\n",
        ),
        (
            "zelyra.toml",
            "[package]\nname = \"business_app\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[capabilities]\ndatabase = true\n\n[dependencies.billing]\npath = \"packages/billing\"\nversion = \"^1.0\"\n",
        ),
        (
            "packages/billing/zelyra.toml",
            "[package]\nname = \"billing\"\nversion = \"1.2.0\"\nentry = \"src/lib.zyl\"\neffects = [\"Database(read)\"]\n\n[dependencies.storage]\npath = \"../storage\"\nversion = \"^1.0\"\n",
        ),
        (
            "packages/billing/src/lib.zyl",
            "import \"pkg:storage\" as storage\npub fn invoice_count() -> Int uses Database(read) { return storage::invoice_count() }\n",
        ),
        (
            "packages/storage/zelyra.toml",
            "[package]\nname = \"storage\"\nversion = \"1.1.0\"\nentry = \"src/lib.zyl\"\neffects = [\"Database(read)\"]\n",
        ),
        (
            "packages/storage/src/lib.zyl",
            "database main { engine: mariadb database: \"business\" }\ntable invoices { id: Id primary auto customer: String(80) required access { read: [\"packages/storage/src/lib.zyl\"] } }\npub fn invoice_count() -> Int uses Database(read) { rows = sql_decode<Int[]>(sql<Int[]> { SELECT id FROM invoices }) return len(rows) }\n",
        ),
    ]);
    let lock = run(&directory, &["package", "lock", "main.zyl"]);
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );
    let check = run(&directory, &["check", "main.zyl"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );

    let output = directory.with_extension("package-business-docker");
    let output_arg = output.to_string_lossy().into_owned();
    let compiler_ref = "a".repeat(40);
    let bundle = run(
        &directory,
        &[
            "module",
            "bundle",
            "main.zyl",
            "pkg:billing",
            "--output",
            &output_arg,
            "--docker",
            "--compiler-ref",
            &compiler_ref,
        ],
    );
    assert!(
        bundle.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&bundle.stdout),
        String::from_utf8_lossy(&bundle.stderr)
    );

    for path in [
        "Dockerfile",
        "docker-compose.yml",
        ".dockerignore",
        ".env.example",
        "zelyra.lock",
        "packages/billing/src/lib.zyl",
        "packages/storage/src/lib.zyl",
    ] {
        assert!(output.join(path).is_file(), "missing bundled {path}");
    }
    let manifest: Value =
        serde_json::from_slice(&fs::read(output.join("zelyra.bundle.json")).unwrap()).unwrap();
    assert_eq!(manifest["source_closure_complete"], true);
    assert_eq!(manifest["complete_deployment"], false);
    assert_eq!(manifest["database"]["required"], true);
    assert_eq!(
        manifest["database"]["connection_environment"],
        "ZELYRA_DATABASE_MAIN_URL"
    );
    assert_eq!(
        manifest["database"]["configurations"][0]["module"],
        "packages/storage/src/lib.zyl"
    );
    let env_example = fs::read_to_string(output.join(".env.example")).unwrap();
    assert!(env_example
        .lines()
        .any(|line| line == "ZELYRA_DATABASE_MAIN_URL="));
    let bundled_lock = run(&output, &["package", "lock", "main.zyl", "--check"]);
    assert!(
        bundled_lock.status.success(),
        "{}",
        String::from_utf8_lossy(&bundled_lock.stderr)
    );
    let bundled_check = run(&output, &["check", "main.zyl"]);
    assert!(
        bundled_check.status.success(),
        "{}",
        String::from_utf8_lossy(&bundled_check.stderr)
    );

    if let Ok(base_database_url) = std::env::var("ZELYRA_MODULE_IMPORTS_DATABASE_URL") {
        let database_root = base_database_url
            .rsplit_once('/')
            .map(|(root, _)| root)
            .expect("configured MariaDB test URL must include a database");
        let database_name = format!(
            "zelyra_package_business_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let database_url = format!("{database_root}/{database_name}");
        let bootstrap = Command::new(env!("CARGO_BIN_EXE_zelyra"))
            .current_dir(&directory)
            .env("DATABASE_URL", &database_url)
            .args(["db", "bootstrap", "main.zyl"])
            .output()
            .unwrap();
        assert!(
            bootstrap.status.success(),
            "{}",
            String::from_utf8_lossy(&bootstrap.stderr)
        );
        let executed = Command::new(env!("CARGO_BIN_EXE_zelyra"))
            .current_dir(&directory)
            .env("DATABASE_URL", &database_url)
            .args(["run", "main.zyl"])
            .output()
            .unwrap();
        assert!(
            executed.status.success(),
            "{}",
            String::from_utf8_lossy(&executed.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&executed.stdout).trim(), "0");
        drop_test_database(&database_url);
    }

    // The provider is intentionally read-only. A consuming package may ask
    // for Database(write), but that must not widen the provider's ceiling.
    fs::write(
        directory.join("main.zyl"),
        "import \"pkg:billing\" as billing\nfn main() uses Database(write) { print(billing::invoice_count()) }\n",
    )
    .unwrap();
    fs::write(
        directory.join("packages/billing/zelyra.toml"),
        "[package]\nname = \"billing\"\nversion = \"1.2.0\"\nentry = \"src/lib.zyl\"\neffects = [\"Database(write)\"]\n\n[dependencies.storage]\npath = \"../storage\"\nversion = \"^1.0\"\n",
    )
    .unwrap();
    fs::write(
        directory.join("packages/billing/src/lib.zyl"),
        "import \"pkg:storage\" as storage\npub fn invoice_count() -> Int uses Database(write) { return storage::invoice_count() }\n",
    )
    .unwrap();
    fs::write(
        directory.join("packages/storage/src/lib.zyl"),
        "database main { engine: mariadb database: \"business\" }\ntable invoices { id: Id primary auto customer: String(80) required }\npub fn invoice_count() -> Int uses Database(write) { sql { UPDATE invoices SET customer = 'example' WHERE id = 1 } return 1 }\n",
    )
    .unwrap();
    let update_lock = run(&directory, &["package", "lock", "main.zyl", "--update"]);
    assert!(
        update_lock.status.success(),
        "{}",
        String::from_utf8_lossy(&update_lock.stderr)
    );
    let denied_write = run(&directory, &["check", "main.zyl", "--format=json"]);
    assert!(!denied_write.status.success());
    let diagnostics: Value = serde_json::from_slice(&denied_write.stdout).unwrap();
    assert!(diagnostics["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item["code"] == "E-PKG-003"
                && item["file"] == "packages/storage/src/lib.zyl"
                && item["message"].as_str().is_some_and(|message| {
                    message.contains("Database(write)") && message.contains("package.effects")
                })
        }));

    fs::remove_dir_all(directory).unwrap();
    fs::remove_dir_all(output).unwrap();
}
