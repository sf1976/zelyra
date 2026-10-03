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
            "import \"src/money.zyl\" as money\npub fn total() -> Int { return money::amount() }\n",
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
            {"path":"main.zyl","imports":[{"alias":"invoice","path":"src/invoice.zyl"}]},
            {"path":"src/invoice.zyl","imports":[{"alias":"money","path":"src/money.zyl"}]},
            {"path":"src/money.zyl","imports":[]}
        ])
    );
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

#[test]
fn check_composes_a_project_database_from_an_imported_configuration_module() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/database.zyl\" as storage\nimport \"src/invoices.zyl\" as invoices\nfn main() uses Database { invoices::count() }\n",
        ),
        (
            "src/database.zyl",
            "database main { engine: mariadb database: \"invoices\" }\n",
        ),
        (
            "src/invoices.zyl",
            "table invoices { id: Id primary auto number: String(40) required total: Decimal required }\npub fn count() -> Int uses Database { rows = sql<Invoice[]> { SELECT id, number, total FROM invoices } return 0 }\n",
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
    assert_eq!(sql[0]["span"]["start"]["line"], 2);
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
    fs::remove_dir_all(directory).unwrap();
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
            "table machines { id: Id primary auto name: String(100) required department: Department required }\n",
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
