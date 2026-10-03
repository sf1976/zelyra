use serde_json::Value;
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
fn module_bundle_materializes_a_checked_source_closure_without_secrets() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/pages.zyl\" as pages\nimport \"src/ui.zyl\" as ui\nfn main() { print(\"original entry\") }\n",
        ),
        (
            "src/pages.zyl",
            "page \"/invoices\" { view: AppShell html { <h1>Bundled invoices</h1> } }\n",
        ),
        (
            "src/ui.zyl",
            "view AppShell { html { <html><body><slot /></body></html> } }\n",
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
    assert_eq!(manifest["kind"], "experimental-source-bundle");
    assert_eq!(manifest["source_closure_complete"], false);
    assert_eq!(manifest["complete_deployment"], false);
    let check = run(&bundle, &["check", "main.zyl", "--format=json"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stdout)
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
    let response = {
        let socket: std::net::SocketAddr = address.parse().unwrap();
        let mut response = String::new();
        for _ in 0..75 {
            if let Ok(mut stream) = TcpStream::connect_timeout(&socket, Duration::from_millis(100))
            {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .write_all(
                        b"GET /invoices HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                    )
                    .unwrap();
                let mut candidate = String::new();
                stream.read_to_string(&mut candidate).unwrap();
                if !candidate.is_empty() {
                    response = candidate;
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(40));
        }
        response
    };
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
    let env_example = fs::read_to_string(bundle.join(".env.example")).unwrap();
    assert!(env_example.contains("DATABASE_URL="));
    assert!(!env_example.contains("must-not-be-copied"));
    assert!(!env_example.contains("example-secret"));

    let manifest: Value =
        serde_json::from_slice(&fs::read(bundle.join("zelyra.bundle.json")).unwrap()).unwrap();
    assert_eq!(manifest["kind"], "experimental-docker-source-package");
    assert_eq!(manifest["docker"]["compiler_commit"], compiler_ref);
    assert_eq!(manifest["source_closure_complete"], false);
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
fn module_plan_adds_cross_module_views_components_and_database_tables() {
    let directory = project(&[
        (
            "main.zyl",
            "import \"src/pages.zyl\" as pages\nimport \"src/ui.zyl\" as ui\nimport \"src/models.zyl\" as models\nfn main() {}\n",
        ),
        (
            "src/pages.zyl",
            "page \"/invoices\" { view: InvoiceLayout load invoices = sql<Invoice[]> { SELECT id, customer_id FROM invoices } html { <InvoiceBadge label=\"Invoices\" /> } }\n",
        ),
        (
            "src/ui.zyl",
            "component InvoiceBadge { props { label: String } html { <strong>{label}</strong> } }\nview InvoiceLayout { html { <main><InvoiceBadge label=\"Title\" /><slot /></main> } }\n",
        ),
        (
            "src/models.zyl",
            "table customers { id: Id primary auto name: String(100) required }\ntable invoices { id: Id primary auto customer: Customer required }\n",
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
            "table invoices { id: Id primary auto }\npage \"/invoices\" { load invoices = sql<Invoice[]> { SELECT id FROM invoices } html { <p>Invoices</p> } }\n",
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
            "connection_environment": "DATABASE_URL"
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
            "import \"src/customer_schema.zyl\" as schema\npub fn listCustomers() uses Database { return sql<Customer[]> { SELECT id, name FROM customers } }\npub fn renameCustomer(id: Id, name: String) uses Database { sql { UPDATE customers SET name = :name WHERE id = :id } }\n",
        ),
        (
            "src/customer_schema.zyl",
            "table customers { id: Id primary auto name: String(100) required }\n",
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
        plan["plan"]["schema_ownership"]["tables"],
        serde_json::json!([{
            "table": "customers",
            "inferred_owner_module": "src/customer_schema.zyl",
            "ownership_enforced": false
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
        edge["from"] == "function:src/customer_service.zyl::renameCustomer"
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
    assert!(dependencies.iter().any(|dependency| {
        dependency["from_module"] == "src/customer_service.zyl"
            && dependency["to_module"] == "src/customer_schema.zyl"
            && dependency["to"] == "table:customers"
            && dependency["access"] == "write"
    }));
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
            "table customers { id: Id primary auto name: String(100) required }\n",
        ),
        (
            "src/audit.zyl",
            "table audit_events { id: Id primary auto message: String(200) required }\n",
        ),
        (
            "src/customers.zyl",
            "form CustomerCreate -> customers { fields { name } action save { let clean_name = normalize_name() sql { INSERT INTO customers (name) VALUES (:clean_name) } sql { INSERT INTO audit_events (message) VALUES (:clean_name) } } }\ncrud Customer -> customers { list { id name } action audit { sql { INSERT INTO audit_events (message) VALUES ('crud') } } }\nfn normalize_name() -> String { return \"New customer\" }\n",
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
        dependency["kind"] == "database_configuration"
            && dependency["to_module"] == "src/database.zyl"
    }));
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
            "import \"src/models.zyl\" as models\nauth users { table: users }\n",
        ),
        (
            "src/models.zyl",
            "table users { id: Id primary auto email: Email required password_hash: String(255) required }\n",
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
    let response = {
        let socket: std::net::SocketAddr = address.parse().unwrap();
        let mut response = String::new();
        for _ in 0..75 {
            if let Ok(mut stream) = TcpStream::connect_timeout(&socket, Duration::from_millis(100))
            {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .write_all(
                        b"GET /api/status HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                    )
                    .unwrap();
                let mut candidate = String::new();
                stream.read_to_string(&mut candidate).unwrap();
                if !candidate.is_empty() {
                    response = candidate;
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(40));
        }
        response
    };
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
    let response = {
        let socket: std::net::SocketAddr = address.parse().unwrap();
        let mut response = String::new();
        for _ in 0..75 {
            if let Ok(mut stream) = TcpStream::connect_timeout(&socket, Duration::from_millis(100))
            {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .write_all(
                        b"GET /views/invoiceoverview HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                    )
                    .unwrap();
                let mut candidate = String::new();
                stream.read_to_string(&mut candidate).unwrap();
                if !candidate.is_empty() {
                    response = candidate;
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(40));
        }
        response
    };
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
    }
}
fn main() {}
"#,
        ),
        (
            "src/shell.zyl",
            r#"component Brand {
    props {
        label: String
    }
    html {
        <strong>{label}</strong>
    }
}
view Shell {
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

    let response = {
        let socket: std::net::SocketAddr = address.parse().unwrap();
        let mut response = String::new();
        for _ in 0..75 {
            if let Ok(mut stream) = TcpStream::connect_timeout(&socket, Duration::from_millis(100))
            {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                    .unwrap();
                let mut candidate = String::new();
                stream.read_to_string(&mut candidate).unwrap();
                if !candidate.is_empty() {
                    response = candidate;
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(40));
        }
        response
    };
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
    let response = {
        let socket: std::net::SocketAddr = address.parse().unwrap();
        let mut response = String::new();
        for _ in 0..75 {
            if let Ok(mut stream) = TcpStream::connect_timeout(&socket, Duration::from_millis(100))
            {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .write_all(
                        b"GET /reports/42?term=7 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                    )
                    .unwrap();
                let mut candidate = String::new();
                stream.read_to_string(&mut candidate).unwrap();
                if !candidate.is_empty() {
                    response = candidate;
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(40));
        }
        response
    };
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
