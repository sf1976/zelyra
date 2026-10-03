use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

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
