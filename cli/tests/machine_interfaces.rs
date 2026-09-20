use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
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

fn temporary_project_source(name: &str, source: &str) -> (PathBuf, PathBuf) {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(format!(".zelyra-edit-test-{}-{}", std::process::id(), name));
    fs::create_dir_all(&directory).expect("temporary project should be created");
    fs::write(
        directory.join("zelyra.toml"),
        "[project]\nname = \"edit-test\"\nversion = \"0.1.39\"\nzelyra = \"0.1\"\n\n[capabilities]\ndatabase = false\nnetwork = false\n",
    )
    .expect("temporary project config should be written");
    let source_path = directory.join("main.zyl");
    fs::write(&source_path, source).expect("temporary project source should be written");
    (directory, source_path)
}

fn temporary_directory(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "zelyra-machine-interface-{}-{}-{}",
        std::process::id(),
        name,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn free_test_port() -> String {
    TcpListener::bind(("127.0.0.1", 0))
        .expect("a test port should be available")
        .local_addr()
        .expect("test listener should have an address")
        .port()
        .to_string()
}

#[test]
fn valid_check_json_is_a_stable_machine_document() {
    let path = example("fibonacci.zyl");
    let first = run(&["check", path.to_str().unwrap(), "--format=json"]);
    let second = run(&["check", path.to_str().unwrap(), "--format=json"]);
    assert!(
        first.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
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
fn new_mariadb_project_propagates_the_selected_web_port() {
    let directory = temporary_directory("new-web-port");
    let database_host_port = free_test_port();
    let web_host_port = free_test_port();
    let output = run(&[
        "new",
        directory.to_str().unwrap(),
        "--mariadb",
        "--web-port",
        "8080",
        "--host-port",
        &web_host_port,
        "--db-host-port",
        &database_host_port,
    ]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let env_example = fs::read_to_string(directory.join(".env.example")).unwrap();
    let env_file = fs::read_to_string(directory.join(".env")).unwrap();
    let compose = fs::read_to_string(directory.join("docker-compose.mariadb.yml")).unwrap();
    assert!(env_example.contains("ZELYRA_WEB_PORT=8080"));
    assert!(env_example.contains("ZELYRA_LANGUAGE=de"));
    assert!(env_example.contains("ZELYRA_LEVEL=learn"));
    assert!(compose.contains("0.0.0.0:${ZELYRA_WEB_PORT:-8080}"));
    assert!(compose.contains("ZELYRA_LANGUAGE: ${ZELYRA_LANGUAGE:-de}"));
    assert!(compose.contains("ZELYRA_LEVEL: ${ZELYRA_LEVEL:-learn}"));
    assert!(env_example.contains(&format!("ZELYRA_HOST_PORT={web_host_port}")));
    assert!(env_example.contains(&format!("ZELYRA_DB_HOST_PORT={database_host_port}")));
    assert!(env_file.contains("DATABASE_URL=mariadb://zelyra:"));
    assert!(env_file.contains("ZELYRA_LANGUAGE=de"));
    assert!(env_file.contains("ZELYRA_LEVEL=learn"));
    assert!(env_file.contains("# ZELYRA_WEB_PORT=8080"));
    assert!(env_file.contains(&format!("# ZELYRA_HOST_PORT={web_host_port}")));
    assert!(env_file.contains(&format!("ZELYRA_DB_HOST_PORT={database_host_port}")));
    assert!(!env_file.contains("change-me"));
    assert!(compose.contains(&format!(
        "127.0.0.1:${{ZELYRA_HOST_PORT:-{web_host_port}}}:${{ZELYRA_WEB_PORT:-8080}}"
    )));
    assert!(compose.contains(&format!(
        "127.0.0.1:${{ZELYRA_DB_HOST_PORT:-{database_host_port}}}:3306"
    )));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn serve_loads_project_theme_and_locale_catalogs() {
    let directory = temporary_directory("serve-theme");
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("main.zyl");
    fs::write(
        &source,
        r#"page "/" {
    html {
        <html><head></head><body><div class="zelyra-app"><main><h1 data-zelyra-i18n="app.home_title"></h1><p data-zelyra-i18n="project.greeting"></p></main></div></body></html>
    }
}
"#,
    )
    .unwrap();
    let theme = ":root { --zelyra-color-accent: #e04b67; }\n";
    fs::write(directory.join("zelyra.theme.css"), theme).unwrap();
    fs::create_dir(directory.join("locales")).unwrap();
    fs::write(
        directory.join("locales/en.json"),
        r#"{"app.home_title":"Custom project title","project.greeting":"Welcome to our workshop"}"#,
    )
    .unwrap();

    let port = free_test_port();
    let address = format!("127.0.0.1:{port}");
    let mut server = Command::new(binary())
        .args(["serve", source.to_str().unwrap(), &address])
        .env("ZELYRA_LANGUAGE", "en")
        .env("ZELYRA_LEVEL", "work")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("Zelyra server should start");

    let send_request = |path: &str| -> Result<String, String> {
        let socket_address: SocketAddr = address
            .parse()
            .map_err(|error| format!("invalid test server address: {error}"))?;
        for _ in 0..50 {
            if let Ok(mut stream) =
                TcpStream::connect_timeout(&socket_address, Duration::from_millis(100))
            {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .map_err(|error| error.to_string())?;
                stream
                    .write_all(format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes())
                    .map_err(|error| error.to_string())?;
                let mut response = String::new();
                stream
                    .read_to_string(&mut response)
                    .map_err(|error| error.to_string())?;
                if !response.is_empty() {
                    return Ok(response);
                }
            }
            std::thread::sleep(Duration::from_millis(40));
        }
        Err("Zelyra server did not answer before the test timeout".into())
    };

    let result = (|| {
        let page = send_request("/")?;
        let stylesheet = send_request("/__zelyra/theme.css")?;
        Ok::<_, String>((page, stylesheet))
    })();
    let _ = server.kill();
    let _ = server.wait();
    fs::remove_dir_all(&directory).unwrap();

    let (page, stylesheet) = result.unwrap_or_else(|error| panic!("{error}"));
    assert!(page.starts_with("HTTP/1.1 200 OK"));
    assert!(page.contains("Custom project title"));
    assert!(page.contains("Welcome to our workshop"));
    let built_in = page.find("data-zelyra-theme=\"default\"").unwrap();
    let project = page.find("href=\"/__zelyra/theme.css\"").unwrap();
    assert!(built_in < project);
    assert!(stylesheet.starts_with("HTTP/1.1 200 OK"));
    assert!(stylesheet.contains("Content-Type: text/css; charset=utf-8\r\n"));
    assert!(stylesheet.contains("X-Content-Type-Options: nosniff\r\n"));
    assert!(stylesheet.contains("Cache-Control: no-cache\r\n"));
    assert!(stylesheet.ends_with(theme));
}

#[test]
fn serve_rejects_routes_that_conflict_with_the_project_theme_asset() {
    let directory = temporary_directory("serve-theme-route-conflict");
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("main.zyl");
    fs::write(
        &source,
        r#"page "/__zelyra/theme.css" {
    html { <main>Conflicting route</main> }
}
"#,
    )
    .unwrap();
    fs::write(directory.join("zelyra.theme.css"), "/* project theme */\n").unwrap();

    let output = Command::new(binary())
        .args(["serve", source.to_str().unwrap(), "127.0.0.1:0"])
        .output()
        .expect("Zelyra CLI should run");
    fs::remove_dir_all(&directory).unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E-THEME-002"),
        "unexpected stderr: {stderr}"
    );
    assert!(stderr.contains("/__zelyra/theme.css"));
}

#[test]
fn serve_reports_invalid_project_locale_catalog_without_echoing_contents() {
    let directory = temporary_directory("serve-invalid-locale");
    fs::create_dir_all(directory.join("locales")).unwrap();
    let source = directory.join("main.zyl");
    fs::write(&source, "fn main() { print(\"ok\") }\n").unwrap();
    fs::write(
        directory.join("locales/en.json"),
        r#"{"custom.title":"PRIVATE_SENTINEL", bad-json}"#,
    )
    .unwrap();

    let output = run(&["serve", source.to_str().unwrap(), "127.0.0.1:0"]);
    fs::remove_dir_all(&directory).unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E-I18N-001"));
    assert!(stderr.contains("locales/en.json"));
    assert!(!stderr.contains("PRIVATE_SENTINEL"));
    assert!(output.stdout.is_empty());
}

#[test]
fn db_create_emits_checked_schema_ddl_without_connecting_to_a_database() {
    let output = run(&[
        "db",
        "create",
        example("machine_management_sqlite.zyl").to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CREATE TABLE"));
    assert!(stdout.contains("departments"));
    assert!(stdout.contains("machines"));
    assert!(output.stderr.is_empty());
}

#[test]
fn init_creates_a_ready_commented_mariadb_env() {
    let directory = temporary_directory("init-env-defaults");
    let database_host_port = free_test_port();
    let web_host_port = free_test_port();
    let output = run(&[
        "init",
        directory.to_str().unwrap(),
        "--mariadb",
        "--web-port",
        "8080",
        "--host-port",
        &web_host_port,
        "--db-host-port",
        &database_host_port,
    ]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let env_file = fs::read_to_string(directory.join(".env")).unwrap();
    assert!(env_file.contains("MARIADB_DATABASE=zelyra_app"));
    assert!(env_file.contains("MARIADB_USER=zelyra"));
    assert!(env_file.contains("MARIADB_PASSWORD="));
    assert!(env_file.contains("MARIADB_ROOT_PASSWORD="));
    assert!(env_file.contains("ZELYRA_LANGUAGE=de"));
    assert!(env_file.contains("ZELYRA_LEVEL=learn"));
    assert!(env_file.contains("# ZELYRA_FEATURE_API=true"));
    assert!(env_file.contains(&format!("ZELYRA_DB_HOST_PORT={database_host_port}")));
    assert!(
        env_file
            .find(&format!("ZELYRA_DB_HOST_PORT={database_host_port}"))
            .unwrap()
            < env_file.find("DATABASE_URL=mariadb://").unwrap()
    );
    assert!(!env_file
        .lines()
        .any(|line| line == "ZELYRA_FEATURE_API=true"));
    assert!(!env_file.contains("change-me"));
    assert!(directory.join(".env.example").is_file());
    assert!(directory.join("locales/de.json").is_file());
    assert!(directory.join("locales/en.json").is_file());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn default_mariadb_web_starter_is_catalog_localized_and_checkable() {
    let directory = temporary_directory("new-mariadb-starter-localized");
    let database_host_port = free_test_port();
    let output = run(&[
        "new",
        directory.to_str().unwrap(),
        "--mariadb",
        "--db-host-port",
        &database_host_port,
    ]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let source = fs::read_to_string(directory.join("main.zyl")).unwrap();
    assert!(source.contains("class=\"zelyra-app\""));
    assert!(source.contains("data-zelyra-language"));
    assert!(source.contains("data-zelyra-i18n=\"starter.workspace_title\""));
    assert!(!source.contains("A clear start for your next application."));

    let checked = run(&["check", directory.join("main.zyl").to_str().unwrap()]);
    assert!(
        checked.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&checked.stdout),
        String::from_utf8_lossy(&checked.stderr)
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn new_mariadb_crud_template_is_self_contained() {
    let directory = temporary_directory("new-mariadb-crud-template");
    let database_host_port = free_test_port();
    let web_host_port = free_test_port();
    let output = run(&[
        "new",
        directory.to_str().unwrap(),
        "--template",
        "mariadb-crud",
        "--web-port",
        "8080",
        "--host-port",
        &web_host_port,
        "--db-host-port",
        &database_host_port,
    ]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let source = fs::read_to_string(directory.join("main.zyl")).unwrap();
    let config = fs::read_to_string(directory.join("zelyra.toml")).unwrap();
    assert!(source.contains("table departments"));
    assert!(source.contains("form MachineCreate -> machines"));
    assert!(source.contains("crud Machine -> machines"));
    assert!(config.contains("engine = \"mariadb\""));
    assert!(directory.join("docker-compose.mariadb.yml").is_file());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn new_mariadb_auth_template_is_self_contained() {
    let directory = temporary_directory("new-mariadb-auth-template");
    let database_host_port = free_test_port();
    let web_host_port = free_test_port();
    let output = run(&[
        "new",
        directory.to_str().unwrap(),
        "--template",
        "mariadb-auth",
        "--web-port",
        "8080",
        "--host-port",
        &web_host_port,
        "--db-host-port",
        &database_host_port,
    ]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let source = fs::read_to_string(directory.join("main.zyl")).unwrap();
    let config = fs::read_to_string(directory.join("zelyra.toml")).unwrap();
    assert!(source.contains("auth users"));
    assert!(source.contains("table auth_sessions"));
    assert!(source.contains("requires auth"));
    assert!(source.contains("permits \"admin.view\""));
    assert!(config.contains("engine = \"mariadb\""));
    assert!(directory.join(".env.example").is_file());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn new_mariadb_business_template_is_self_contained() {
    let directory = temporary_directory("new-mariadb-business-template");
    let database_host_port = free_test_port();
    let web_host_port = free_test_port();
    let output = run(&[
        "new",
        directory.to_str().unwrap(),
        "--template",
        "mariadb-business",
        "--web-port",
        "8080",
        "--host-port",
        &web_host_port,
        "--db-host-port",
        &database_host_port,
    ]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let source = fs::read_to_string(directory.join("main.zyl")).unwrap();
    let config = fs::read_to_string(directory.join("zelyra.toml")).unwrap();
    assert!(source.contains("auth users"));
    assert!(source.contains("auth_audit_log"));
    assert!(source.contains("crud Customer -> customers"));
    assert!(source.contains("api GET \"/api/customers/{id}\""));
    assert!(source.contains("form CustomerQuickCreate -> customers"));
    assert!(config.contains("engine = \"mariadb\""));
    assert!(directory.join("docker-compose.mariadb.yml").is_file());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn new_rejects_an_invalid_web_port_before_creating_a_project() {
    let directory = temporary_directory("invalid-web-port");
    let output = run(&[
        "new",
        directory.to_str().unwrap(),
        "--mariadb",
        "--web-port",
        "65536",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("E-CLI-001"));
    assert!(!directory.exists());
}

#[test]
fn web_port_requires_the_mariadb_web_template() {
    let directory = temporary_directory("web-port-without-mariadb");
    let output = run(&["new", directory.to_str().unwrap(), "--web-port", "8080"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--mariadb"));
    assert!(!directory.exists());
}

#[test]
fn host_port_requires_the_mariadb_web_template() {
    let directory = temporary_directory("host-port-without-mariadb");
    let output = run(&["new", directory.to_str().unwrap(), "--host-port", "18080"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--mariadb"));
    assert!(!directory.exists());
}

#[test]
fn setup_creates_a_local_env_without_printing_or_overwriting_secrets() {
    let directory = temporary_directory("setup-env");
    let database_host_port = free_test_port();
    let web_host_port = free_test_port();
    let scaffold = run(&[
        "new",
        directory.to_str().unwrap(),
        "--mariadb",
        "--web-port",
        "8080",
        "--host-port",
        &web_host_port,
        "--db-host-port",
        &database_host_port,
    ]);
    assert!(scaffold.status.success());
    fs::remove_file(directory.join(".env")).unwrap();

    let setup = run(&["setup", directory.to_str().unwrap()]);
    assert!(
        setup.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&setup.stdout),
        String::from_utf8_lossy(&setup.stderr)
    );
    let stdout = String::from_utf8_lossy(&setup.stdout);
    assert!(!stdout.contains("MARIADB_PASSWORD"));
    assert!(!stdout.contains("change-me"));
    let env_file = directory.join(".env");
    let env_example = fs::read_to_string(directory.join(".env.example")).unwrap();
    let contents = fs::read_to_string(&env_file).unwrap();
    assert!(contents.contains("# ZELYRA_WEB_PORT=8080"));
    assert!(contents.contains(&format!("# ZELYRA_HOST_PORT={web_host_port}")));
    assert!(contents.contains(
        env_example
            .lines()
            .find(|line| line.starts_with("ZELYRA_DB_HOST_PORT="))
            .unwrap()
    ));
    assert!(!contents.contains("change-me"));
    assert!(contents.contains("DATABASE_URL=mariadb://zelyra:"));
    assert!(contents.contains("MARIADB_PASSWORD="));
    assert!(contents.contains("MARIADB_ROOT_PASSWORD="));

    let doctor = run(&[
        "doctor",
        directory.join("main.zyl").to_str().unwrap(),
        "--env-file",
        env_file.to_str().unwrap(),
        "--port",
        "18080",
        "--json",
    ]);
    let doctor_document: serde_json::Value = serde_json::from_slice(&doctor.stdout).unwrap();
    let doctor_checks = doctor_document["checks"].as_array().unwrap();
    assert!(doctor_checks
        .iter()
        .any(|check| check["name"] == "env_file"));
    assert!(doctor_checks
        .iter()
        .any(|check| check["name"] == "docker_compose"));
    assert!(!String::from_utf8_lossy(&doctor.stdout).contains("change-me"));
    let database_password = contents
        .lines()
        .find_map(|line| line.strip_prefix("MARIADB_PASSWORD="))
        .unwrap();
    assert!(!String::from_utf8_lossy(&doctor.stdout).contains(database_password));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&env_file).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    let original = contents;
    let second_setup = run(&["setup", directory.to_str().unwrap()]);
    assert!(second_setup.status.success());
    assert!(String::from_utf8_lossy(&second_setup.stdout).contains("kept existing"));
    assert_eq!(fs::read_to_string(env_file).unwrap(), original);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn setup_selects_free_ports_for_a_new_local_environment() {
    let directory = temporary_directory("setup-free-ports");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("zelyra.toml"),
        "[database.main]\nengine = \"mariadb\"\n",
    )
    .unwrap();
    let web_listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let requested_web_port = web_listener.local_addr().unwrap().port();
    let database_listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let requested_database_port = database_listener.local_addr().unwrap().port();
    fs::write(
        directory.join(".env.example"),
        format!(
            "ZELYRA_DB_HOST_PORT={requested_database_port}\nDATABASE_URL=mariadb://zelyra:change-me@127.0.0.1:${{ZELYRA_DB_HOST_PORT:-3306}}/zelyra_app\nMARIADB_DATABASE=zelyra_app\nMARIADB_USER=zelyra\nMARIADB_PASSWORD=change-me\nMARIADB_ROOT_PASSWORD=change-me-root\n# ZELYRA_HOST_PORT={requested_web_port}\n"
        ),
    )
    .unwrap();

    let output = run(&["setup", directory.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("selected free port"));
    let env_file = fs::read_to_string(directory.join(".env")).unwrap();
    assert!(env_file.contains("ZELYRA_HOST_PORT="));
    assert!(!env_file.contains(&format!("ZELYRA_HOST_PORT={requested_web_port}")));
    assert!(!env_file.contains(&format!("ZELYRA_DB_HOST_PORT={requested_database_port}")));
    assert!(env_file.contains("${ZELYRA_DB_HOST_PORT:-3306}"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn setup_rejects_web_server_port_without_the_web_assistant() {
    let directory = temporary_directory("setup-port-without-web");
    fs::create_dir_all(&directory).unwrap();
    let output = run(&["setup", directory.to_str().unwrap(), "--port", "3031"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--port requires --web"));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn setup_rejects_an_explicitly_occupied_host_port_without_creating_env() {
    let directory = temporary_directory("setup-explicit-port-conflict");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("zelyra.toml"),
        "[database.main]\nengine = \"mariadb\"\n",
    )
    .unwrap();
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let occupied_port = listener.local_addr().unwrap().port().to_string();
    let output = run(&[
        "setup",
        directory.to_str().unwrap(),
        "--host-port",
        &occupied_port,
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("already in use"));
    assert!(!directory.join(".env").exists());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn setup_refuses_to_change_a_static_database_url_port() {
    let directory = temporary_directory("setup-static-database-url");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("zelyra.toml"),
        "[database.main]\nengine = \"mariadb\"\n",
    )
    .unwrap();
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let occupied_port = listener.local_addr().unwrap().port();
    fs::write(
        directory.join(".env.example"),
        format!(
            "ZELYRA_DB_HOST_PORT={occupied_port}\nDATABASE_URL=mariadb://zelyra:change-me@127.0.0.1:{occupied_port}/zelyra_app\nMARIADB_PASSWORD=change-me\nMARIADB_ROOT_PASSWORD=change-me-root\n"
        ),
    )
    .unwrap();

    let output = run(&["setup", directory.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot safely select"));
    assert!(!directory.join(".env").exists());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn setup_rejects_a_directory_without_a_mariadb_scaffold() {
    let directory = temporary_directory("setup-missing-scaffold");
    fs::create_dir_all(&directory).unwrap();
    let output = run(&["setup", directory.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("E-SETUP-001"));
    fs::remove_dir_all(directory).unwrap();
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
fn config_reports_effective_optional_features_without_secret_values() {
    let (project_directory, source_path) = temporary_project_source("config", "fn main() {}\n");
    fs::write(
        project_directory.join("zelyra.toml"),
        "[project]\nname = \"config-test\"\nversion = \"0.1.39\"\nzelyra = \"0.1\"\n\n[features]\napi = false\ncrud = false\n",
    )
    .unwrap();
    fs::write(
        project_directory.join(".env"),
        "ZELYRA_FEATURE_CRUD=true\nDATABASE_URL=mariadb://user:super-secret@localhost/app\n",
    )
    .unwrap();

    let output = run(&["config", source_path.to_str().unwrap(), "--format=json"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["schema_version"], "1");
    assert_eq!(document["command"], "config");
    assert_eq!(document["features"]["api"]["enabled"], false);
    assert_eq!(document["features"]["api"]["source"], "zelyra.toml");
    assert_eq!(document["features"]["crud"]["enabled"], true);
    assert_eq!(document["features"]["crud"]["source"], ".env");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("super-secret"));
    assert_eq!(document["project"]["config_file"], "zelyra.toml");
    fs::remove_dir_all(project_directory).unwrap();
}

#[test]
fn disabled_api_feature_rejects_api_source() {
    let (project_directory, source_path) = temporary_project_source(
        "disabled-api",
        "api GET \"/echo\" { handler echo input { value: String } output String } fn echo(value: String) -> String { return value } fn main() { }",
    );
    fs::write(
        project_directory.join("zelyra.toml"),
        "[project]\nname = \"feature-test\"\nversion = \"0.1.39\"\nzelyra = \"0.1\"\n\n[features]\napi = false\n",
    )
    .unwrap();
    let output = run(&["check", source_path.to_str().unwrap(), "--format=json"]);
    assert_eq!(output.status.code(), Some(1));
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["diagnostics"][0]["code"], "E-FEATURE-001");
    fs::remove_dir_all(project_directory).unwrap();
}

#[test]
fn edit_json_is_preview_only_by_default_and_applies_explicitly() {
    let (project_directory, source_path) =
        temporary_project_source("edit-source", "fn greet() { greet() }\nfn main() {}\n");
    let request_path = source_path.with_file_name("change.json");
    let request = format!(
        "{{\"schema_version\":\"1\",\"entry\":{},\"expected_source_fingerprint\":\"fnv1a64:860a2a12aa4d2f8d\",\"operations\":[{{\"kind\":\"rename\",\"symbol\":\"function\",\"from\":\"greet\",\"to\":\"welcome\"}}]}}",
        serde_json::to_string(source_path.to_str().unwrap()).unwrap()
    );
    fs::write(&request_path, request).expect("edit request should be written");

    let first = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    let second = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert!(
        first.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(first.stdout, second.stdout);
    assert!(first.stderr.is_empty());
    assert_eq!(
        fs::read_to_string(&source_path).unwrap(),
        "fn greet() { greet() }\nfn main() {}\n"
    );
    let document: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(document["command"], "edit");
    assert_eq!(document["success"], true);
    assert_eq!(document["preview"]["applied"], false);
    assert_eq!(document["preview"]["changed_tokens"], 2);

    fs::write(&source_path, "fn changed() {}\nfn main() {}\n").expect("source should change");
    let stale = run(&[
        "edit",
        "--format=json",
        "--apply",
        request_path.to_str().unwrap(),
    ]);
    assert!(!stale.status.success());
    let stale_document: serde_json::Value = serde_json::from_slice(&stale.stdout).unwrap();
    assert_eq!(stale_document["diagnostics"][0]["code"], "E-EDIT-004");
    assert_eq!(
        fs::read_to_string(&source_path).unwrap(),
        "fn changed() {}\nfn main() {}\n"
    );

    fs::write(&source_path, "fn greet() { greet() }\nfn main() {}\n")
        .expect("source should be restored");
    let applied = run(&[
        "edit",
        "--format=json",
        "--apply",
        request_path.to_str().unwrap(),
    ]);
    assert!(applied.status.success());
    let applied_document: serde_json::Value = serde_json::from_slice(&applied.stdout).unwrap();
    assert_eq!(applied_document["preview"]["apply_requested"], true);
    assert_eq!(applied_document["preview"]["applied"], true);
    assert_eq!(
        fs::read_to_string(&source_path).unwrap(),
        "fn welcome() { welcome() }\nfn main() {}\n"
    );
    fs::remove_dir_all(project_directory).expect("temporary project should be removed");
}

#[test]
fn edit_requires_a_versioned_request_and_project_local_zelyra_source() {
    let (project_directory, source_path) =
        temporary_project_source("edit-boundary", "fn greet() {}\n");
    let request_path = project_directory.join("change.json");
    fs::write(
        &request_path,
        format!(
            "{{\"entry\":{}}}",
            serde_json::to_string(source_path.to_str().unwrap()).unwrap()
        ),
    )
    .expect("unversioned edit request should be written");
    let unversioned = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert_eq!(unversioned.status.code(), Some(1));
    let unversioned_document: serde_json::Value =
        serde_json::from_slice(&unversioned.stdout).unwrap();
    assert_eq!(unversioned_document["diagnostics"][0]["code"], "E-EDIT-001");

    fs::write(
        &request_path,
        format!(
            "{{\"schema_version\":\"999\",\"entry\":{}}}",
            serde_json::to_string(source_path.to_str().unwrap()).unwrap()
        ),
    )
    .expect("unsupported edit request should be written");
    let unsupported = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert_eq!(unsupported.status.code(), Some(1));
    let unsupported_document: serde_json::Value =
        serde_json::from_slice(&unsupported.stdout).unwrap();
    assert_eq!(unsupported_document["diagnostics"][0]["code"], "E-EDIT-001");

    fs::write(
        &request_path,
        "{\"schema_version\":\"1\",\"entry\":\"/tmp/not-a-zelyra-project.zyl\",\"operations\":[]}",
    )
    .expect("external edit request should be written");
    let external = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert_eq!(external.status.code(), Some(1));
    let external_document: serde_json::Value = serde_json::from_slice(&external.stdout).unwrap();
    assert_eq!(external_document["diagnostics"][0]["code"], "E-EDIT-005");
    assert_eq!(fs::read_to_string(source_path).unwrap(), "fn greet() {}\n");
    fs::remove_dir_all(project_directory).expect("temporary project should be removed");
}

#[test]
fn edit_rejects_a_semantically_invalid_baseline_or_result() {
    let (project_directory, source_path) =
        temporary_project_source("edit-semantic", "fn greet() {}\nfn main() {}\n");
    let request_path = project_directory.join("change.json");
    let request = format!(
        "{{\"schema_version\":\"1\",\"entry\":{},\"operations\":[{{\"kind\":\"rename\",\"symbol\":\"function\",\"from\":\"main\",\"to\":\"greet\"}}]}}",
        serde_json::to_string(source_path.to_str().unwrap()).unwrap()
    );
    fs::write(&request_path, request).expect("semantic edit request should be written");
    let output = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["success"], false);
    assert_eq!(document["preview"]["available"], false);
    assert_eq!(document["diagnostics"][0]["code"], "E-NAME-001");
    assert_eq!(
        fs::read_to_string(&source_path).unwrap(),
        "fn greet() {}\nfn main() {}\n"
    );
    fs::remove_dir_all(project_directory).expect("temporary project should be removed");
}

#[test]
fn edit_does_not_rename_a_shadowing_local_binding() {
    let (project_directory, source_path) = temporary_project_source(
        "edit-scope",
        "fn greet() { greet = 1\n print(greet) }\nfn main() { greet() }\n",
    );
    let request_path = project_directory.join("change.json");
    let request = format!(
        "{{\"schema_version\":\"1\",\"entry\":{},\"operations\":[{{\"kind\":\"rename\",\"symbol\":\"function\",\"from\":\"greet\",\"to\":\"welcome\"}}]}}",
        serde_json::to_string(source_path.to_str().unwrap()).unwrap()
    );
    fs::write(&request_path, request).expect("scoped edit request should be written");
    let output = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["preview"]["changed_tokens"], 2);
    assert_eq!(document["preview"]["changes"].as_array().unwrap().len(), 2);
    assert_eq!(
        fs::read_to_string(&source_path).unwrap(),
        "fn greet() { greet = 1\n print(greet) }\nfn main() { greet() }\n"
    );
    fs::remove_dir_all(project_directory).expect("temporary project should be removed");
}

#[test]
fn edit_renames_type_and_record_references_through_the_cli() {
    let (project_directory, source_path) = temporary_project_source(
        "edit-types",
        "type CustomerId = Id\nstruct Customer { id: CustomerId }\nfn load(id: CustomerId) -> CustomerId { return id }\nfn make(id: CustomerId) -> Customer { return Customer { id: id } }\nfn main() {}\n",
    );
    let request_path = project_directory.join("change.json");
    let request = format!(
        "{{\"schema_version\":\"1\",\"entry\":{},\"operations\":[{{\"kind\":\"rename\",\"symbol\":\"type\",\"from\":\"CustomerId\",\"to\":\"ClientId\"}},{{\"kind\":\"rename\",\"symbol\":\"record\",\"from\":\"Customer\",\"to\":\"Client\"}}]}}",
        serde_json::to_string(source_path.to_str().unwrap()).unwrap()
    );
    fs::write(&request_path, request).expect("type edit request should be written");
    let output = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["preview"]["changed_tokens"], 8);
    assert_eq!(
        fs::read_to_string(&source_path).unwrap(),
        "type CustomerId = Id\nstruct Customer { id: CustomerId }\nfn load(id: CustomerId) -> CustomerId { return id }\nfn make(id: CustomerId) -> Customer { return Customer { id: id } }\nfn main() {}\n"
    );
    fs::remove_dir_all(project_directory).expect("temporary project should be removed");
}

#[test]
fn edit_renames_tables_views_forms_and_cruds_through_the_cli() {
    let machine_source = fs::read_to_string(example("machine_form.zyl"))
        .expect("machine form example should be readable");
    let (machine_directory, machine_path) =
        temporary_project_source("edit-resources", &machine_source);
    let machine_request_path = machine_directory.join("change.json");
    let machine_request = format!(
        "{{\"schema_version\":\"1\",\"entry\":{},\"operations\":[{{\"kind\":\"rename\",\"symbol\":\"table\",\"from\":\"machines\",\"to\":\"equipment\"}},{{\"kind\":\"rename\",\"symbol\":\"form\",\"from\":\"MachineCreate\",\"to\":\"MachineEditor\"}},{{\"kind\":\"rename\",\"symbol\":\"crud\",\"from\":\"Machine\",\"to\":\"MachineAdmin\"}}]}}",
        serde_json::to_string(machine_path.to_str().unwrap()).unwrap()
    );
    fs::write(&machine_request_path, machine_request)
        .expect("resource edit request should be written");
    let machine_output = run(&[
        "edit",
        "--format=json",
        machine_request_path.to_str().unwrap(),
    ]);
    assert!(
        machine_output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&machine_output.stdout),
        String::from_utf8_lossy(&machine_output.stderr)
    );
    let machine_document: serde_json::Value =
        serde_json::from_slice(&machine_output.stdout).unwrap();
    assert_eq!(machine_document["preview"]["changed_tokens"], 8);
    assert_eq!(fs::read_to_string(&machine_path).unwrap(), machine_source);
    fs::remove_dir_all(machine_directory).expect("machine project should be removed");

    let view_source =
        fs::read_to_string(example("views.zyl")).expect("view example should be readable");
    let (view_directory, view_path) = temporary_project_source("edit-view", &view_source);
    let view_request_path = view_directory.join("change.json");
    let view_request = format!(
        "{{\"schema_version\":\"1\",\"entry\":{},\"operations\":[{{\"kind\":\"rename\",\"symbol\":\"view\",\"from\":\"SiteShell\",\"to\":\"AppShell\"}}]}}",
        serde_json::to_string(view_path.to_str().unwrap()).unwrap()
    );
    fs::write(&view_request_path, view_request).expect("view edit request should be written");
    let view_output = run(&["edit", "--format=json", view_request_path.to_str().unwrap()]);
    assert!(
        view_output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&view_output.stdout),
        String::from_utf8_lossy(&view_output.stderr)
    );
    let view_document: serde_json::Value = serde_json::from_slice(&view_output.stdout).unwrap();
    assert_eq!(view_document["preview"]["changed_tokens"], 2);
    assert_eq!(fs::read_to_string(&view_path).unwrap(), view_source);
    fs::remove_dir_all(view_directory).expect("view project should be removed");
}

#[test]
fn edit_renames_view_components_in_declarations_and_html() {
    let (project_directory, source_path) = temporary_project_source(
        "edit-components",
        "component Badge { html { <strong>Ready</strong> } }\nview Shell { html { <Badge /><slot /> } }\n",
    );
    let request_path = project_directory.join("change.json");
    let request = format!(
        "{{\"schema_version\":\"1\",\"entry\":{},\"operations\":[{{\"kind\":\"rename\",\"symbol\":\"component\",\"from\":\"Badge\",\"to\":\"StatusBadge\"}}]}}",
        serde_json::to_string(source_path.to_str().unwrap()).unwrap()
    );
    fs::write(&request_path, request).expect("component edit request should be written");
    let output = run(&["edit", "--format=json", request_path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["preview"]["changed_tokens"], 2);
    assert!(document["preview"]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|change| change["from"] == "Badge" && change["to"] == "StatusBadge"));
    assert_eq!(
        fs::read_to_string(&source_path).unwrap(),
        "component Badge { html { <strong>Ready</strong> } }\nview Shell { html { <Badge /><slot /> } }\n"
    );
    fs::remove_dir_all(project_directory).expect("temporary project should be removed");
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
fn reports_the_full_compiler_version() {
    for arguments in [["--version"], ["-V"], ["version"]] {
        let output = run(&arguments);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("zelyra {}\n", env!("CARGO_PKG_VERSION"))
        );
    }
}

#[test]
fn update_rejects_unknown_arguments_without_network_or_file_changes() {
    for arguments in [
        &["update", "--force"][..],
        &["update", "--check", "extra"][..],
    ] {
        let output = run(arguments);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("zelyra update [--check]"));
    }
}

#[test]
fn context_exposes_view_slot_structure_without_rendered_content() {
    let path = example("view_composition.zyl");
    let output = run(&["context", path.to_str().unwrap(), "--format=json"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let shell = document["declarations"]["views"]
        .as_array()
        .unwrap()
        .iter()
        .find(|view| view["name"] == "AppShell")
        .expect("AppShell should be present");
    assert_eq!(
        shell["slots"],
        serde_json::json!([
            { "name": "header", "fallback": true },
            { "name": "default", "fallback": false },
            { "name": "footer", "fallback": true }
        ])
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Describe intent"));
}

#[test]
fn context_exposes_crud_layout_slot_names_without_rendered_content() {
    let path = example("view_showcase.zyl");
    let output = run(&["context", path.to_str().unwrap(), "--format=json"]);
    let repeated = run(&["context", path.to_str().unwrap(), "--format=json"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, repeated.stdout);
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let customer_crud = document["declarations"]["cruds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|crud| crud["name"] == "Customer")
        .expect("Customer CRUD should be present");
    assert_eq!(customer_crud["layout"], "CustomerShell");
    assert_eq!(customer_crud["layout_slots"][0]["name"], "header");
    assert_eq!(customer_crud["layout_slots"][1]["name"], "intro");
    assert!(customer_crud["layout_slots"][0]["span"]["start"]["offset"]
        .as_u64()
        .is_some());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Manage customer records"));
}

#[test]
fn reports_unknown_crud_layout_slot_with_stable_json_diagnostic() {
    let source = r#"
        view Shell { html { <main><slot /></main> } }
        table customers { id: Id primary auto }
        crud Customer -> customers {
            layout: Shell
            slots { heading { html { <h1>Customers</h1> } } }
        }
    "#;
    let (project_directory, source_path) =
        temporary_project_source("invalid-crud-layout-slot", source);
    let output = run(&["check", source_path.to_str().unwrap(), "--format=json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["schema_version"], "1");
    assert_eq!(document["success"], false);
    assert_eq!(document["diagnostics"][0]["code"], "E-VIEW-031");
    fs::remove_dir_all(project_directory).expect("temporary project should be removed");
}

#[test]
fn impact_can_focus_on_a_known_node_with_versioned_json() {
    let path = example("auth_crud_api.zyl");
    let first = run(&[
        "impact",
        path.to_str().unwrap(),
        "--symbol",
        "table:customers",
        "--format=json",
    ]);
    let second = run(&[
        "impact",
        path.to_str().unwrap(),
        "--symbol",
        "table:customers",
        "--format=json",
    ]);
    assert!(first.status.success());
    assert!(first.stderr.is_empty());
    assert_eq!(first.stdout, second.stdout);
    let document: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(document["schema_version"], "1");
    assert_eq!(document["command"], "impact");
    assert_eq!(document["success"], true);
    assert_eq!(document["impact"]["focus"], "table:customers");
    assert!(document["impact"]["references"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference["to"] == "table:customers"));

    let unknown = run(&[
        "impact",
        path.to_str().unwrap(),
        "--symbol",
        "table:missing",
        "--format=json",
    ]);
    assert_eq!(unknown.status.code(), Some(1));
    assert!(unknown.stderr.is_empty());
    let unknown_document: serde_json::Value = serde_json::from_slice(&unknown.stdout).unwrap();
    assert_eq!(unknown_document["success"], false);
    assert_eq!(unknown_document["diagnostics"][0]["code"], "E-IMPACT-001");
}

#[test]
fn invalid_format_is_rejected_without_machine_output_claims() {
    let output = run(&["check", "../examples/fibonacci.zyl", "--format=xml"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("human` or `json"));
}

#[test]
fn typed_map_api_contract_drives_check_openapi_and_typescript() {
    let path = example("api_maps.zyl");
    let check = run(&["check", path.to_str().unwrap(), "--format=json"]);
    assert!(check.status.success());
    let check_document: serde_json::Value = serde_json::from_slice(&check.stdout).unwrap();
    assert_eq!(check_document["success"], true);

    let openapi = run(&["doc", path.to_str().unwrap(), "--openapi"]);
    assert!(openapi.status.success());
    let openapi_document: serde_json::Value = serde_json::from_slice(&openapi.stdout).unwrap();
    assert_eq!(
        openapi_document["paths"]["/settings"]["post"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["additionalProperties"]["type"],
        "integer"
    );

    let typescript = run(&["doc", path.to_str().unwrap(), "--typescript"]);
    assert!(typescript.status.success());
    let client = String::from_utf8(typescript.stdout).unwrap();
    assert!(client.contains("Record<string, number>"));
}

#[test]
fn typed_view_bindings_are_checked_as_machine_diagnostics() {
    let valid = run(&[
        "check",
        example("typed_views.zyl").to_str().unwrap(),
        "--format=json",
    ]);
    assert!(valid.status.success());
    let valid_document: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert_eq!(valid_document["success"], true);

    let invalid = run(&[
        "check",
        example("invalid_typed_views.zyl").to_str().unwrap(),
        "--format=json",
    ]);
    assert_eq!(invalid.status.code(), Some(1));
    let invalid_document: serde_json::Value = serde_json::from_slice(&invalid.stdout).unwrap();
    let codes = invalid_document["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"E-VIEW-010"));
    assert!(codes.contains(&"E-VIEW-015"));
}
