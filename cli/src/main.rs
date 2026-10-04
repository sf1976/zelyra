use rand_core::{OsRng, RngCore};
use serde_json::{json, Map, Value};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    env,
    fmt::Write as _,
    fs,
    io::Read,
    net::TcpListener,
    path::PathBuf,
    process::Command,
    process::ExitCode,
    process::Stdio,
    sync::{Arc, Mutex},
};
use zelyra_ast::Type;
use zelyra_database::{
    apply_mariadb, apply_postgres, apply_sqlite, build_schema, count_null_values,
    create_mariadb_database, diff, inspect_mariadb, inspect_postgres, inspect_sqlite,
    sql::check_program as check_sql_program, table_has_rows, Backend, Query, QueryResult,
    QueryValue, Risk, Schema,
};
use zelyra_forms::{check_program as check_form_program, validate as validate_form};
use zelyra_hir::lower;
use zelyra_lexer::lex;
use zelyra_parser::parse;
use zelyra_runtime::{
    check, check_apis, check_capabilities_with_grants,
    execute_function_with_capabilities_and_policies, execute_with_capabilities_and_policies,
    execute_with_database_and_capabilities_and_policies, verify as verify_program,
    FileSystemPolicy, NetworkPolicy, ProcessPolicy, RuntimePolicy, Value as RuntimeValue,
    VerificationResult, VerificationStatus, KNOWN_CAPABILITIES,
};
use zelyra_web::{
    audit_insert_queries, html_escape, parse_urlencoded, serve_app, ApiRoute, AuthRoute,
    CorsPolicy, CrudActionRoute, CrudRoute, CsrfProtection, FormRoute, ProjectUiCatalogs, Response,
    Route, RouteData, RouteQuery, TableViewFilter, TableViewFilterKind, TableViewRoute, UiLanguage,
    UiLevel, WebApp, PROJECT_THEME_CSS_PATH,
};

mod edit;
mod formatter;
mod holes;
mod impact;
mod project;
mod updater;
use formatter::format_source;
use holes::collect_typed_holes;
use impact::{build_impact_with_sources, focus_impact};

const MARIADB_CRUD_TEMPLATE: &str = include_str!("../../examples/machine_form.zyl");
const MACHINE_MANAGEMENT_DEMO_DATA: &str =
    include_str!("../../examples/machine_management_demo.sql");
const MARIADB_MINIMAL_TEMPLATE: &str = include_str!("../../examples/mariadb_starter.zyl");
const MARIADB_AUTH_TEMPLATE: &str = include_str!("../../examples/auth.zyl");
const MARIADB_BUSINESS_TEMPLATE: &str = include_str!("../../examples/auth_crud_api.zyl");
const MARIADB_DATABASE_DECLARATION: &str = "database main {\n    engine: mariadb\n}\n\n";
const MARIADB_DATABASE_MODULE: &str = "database main {\n    engine: mariadb\n}\n";
const PROJECT_THEME_TEMPLATE: &str = r#"/*
Optional project-local overrides for the built-in Zelyra web design.
Uncomment a token below and change its value. This file is sent to browsers;
never put passwords, API keys, or private data here.

Token reference: https://github.com/sf1976/zelyra/blob/main/docs/env.md
*/
:root {
    /* --zelyra-color-accent: #7557f6; */
    /* --zelyra-color-accent-strong: #665ce9; */
    /* --zelyra-color-accent-text: #634ce0; */
    /* --zelyra-color-accent-soft: #f8f6ff; */
    /* --zelyra-color-ink: #172033; */
    /* --zelyra-color-muted: #738097; */
    /* --zelyra-color-border: #e8edf4; */
    /* --zelyra-color-canvas: #f5f7fb; */
    /* --zelyra-color-surface: #ffffff; */
    /* --zelyra-color-surface-subtle: #f9faff; */
    /* --zelyra-color-sidebar-start: #171c32; */
    /* --zelyra-color-sidebar-middle: #202743; */
    /* --zelyra-color-sidebar-end: #263958; */
    /* --zelyra-color-sidebar-foreground: #f6f7ff; */
    /* --zelyra-color-sidebar-muted: #bac4d8; */
    /* --zelyra-color-hero-start: #262f52; */
    /* --zelyra-color-hero-middle: #3e4381; */
    /* --zelyra-color-hero-end: #6258bb; */
    /* --zelyra-color-success-background: #effbf7; */
    /* --zelyra-color-success-border: #bcebdd; */
    /* --zelyra-color-success-ink: #17654f; */
    /* --zelyra-color-danger-background: #fff5f5; */
    /* --zelyra-color-danger-border: #f2c8cc; */
    /* --zelyra-color-danger-ink: #8b303c; */
    /* --zelyra-color-focus: #8f7aff; */
    /* --zelyra-font-body: Inter, system-ui, sans-serif; */
    /* --zelyra-radius-card: 16px; */
    /* --zelyra-radius-control: 10px; */
    /* --zelyra-content-max-width: 1180px; */
}
"#;

fn usage() {
    eprintln!("  impact focus: use `--symbol <kind:name>` to inspect one known node");
    eprintln!("  module plan: `zelyra module plan <entry.zyl> <module.zyl|resource-id>` previews known dependencies");
    eprintln!("  module bundle: `zelyra module bundle <entry.zyl> <module.zyl|resource-id> --output <dir> [--dry-run] [--docker --compiler-ref <40-char-commit>]` plans or writes a checked experimental bundle");
    eprintln!("  module bundle --dry-run emits a machine-readable file plan without publishing the bundle");
    eprintln!("  doctor supports `--env-file <path>` for generated MariaDB projects");
    eprintln!("  setup supports `--database`, `--schema`, `--all`, `--host-port`, `--db-host-port`, and `--web [--port <port>]`");
    eprintln!("Zelyra {}\n\nUsage:\n  zelyra --version\n  zelyra version\n  zelyra update [--check]\n  zelyra new <directory> [--mariadb] [--template minimal|mariadb-crud|mariadb-auth|mariadb-business] [--web-port <port>] [--host-port <port>] [--db-host-port <port>]\n  zelyra init [directory] [--mariadb] [--template minimal|mariadb-crud|mariadb-auth|mariadb-business] [--web-port <port>] [--host-port <port>] [--db-host-port <port>]\n  zelyra setup [directory] [--database|--schema|--all] [--host-port <port>] [--db-host-port <port>]\n  zelyra setup [directory] --web [--port <port>]\n  zelyra check <file.zyl> [--format human|json]\n  zelyra fmt <file.zyl> [--check]\n  zelyra impact <file.zyl> [--format human|json]\n  zelyra edit --format=json [--apply] <change.json>\n  zelyra context <file.zyl> [--format human|json]\n  zelyra config <file.zyl> [--format human|json]\n  zelyra build <file.zyl>\n  zelyra run <file.zyl>\n  zelyra serve <file.zyl> [address]\n  zelyra module plan <entry.zyl> <module.zyl|resource-id>\n  zelyra module bundle <entry.zyl> <module.zyl|resource-id> --output <dir> [--docker --compiler-ref <40-character-commit>]\n  zelyra doctor [file.zyl] [--port <port>] [--json]\n  zelyra verify <file.zyl> [--json]\n  zelyra doc <file.zyl> [--openapi|--typescript]\n  zelyra auth hash-password [--stdin]\n  zelyra auth role <grant|revoke> <file.zyl> <user-id> <role>\n  zelyra auth role-permission <grant|revoke> <file.zyl> <role> <permission>\n  zelyra audit inspect <file.zyl> [--limit <n>]\n  zelyra audit export <file.zyl> [--limit <n>] [--format json|csv]\n  zelyra audit verify <file.zyl>\n  zelyra audit prune <file.zyl> --before <timestamp> [--confirm]\n  zelyra form validate <file.zyl> <FormName> [field=value ...]\n  zelyra db <create|setup|bootstrap|inspect|plan|apply> <file.zyl>", env!("CARGO_PKG_VERSION"));
}

fn version_command() -> ExitCode {
    println!("zelyra {}", env!("CARGO_PKG_VERSION"));
    ExitCode::SUCCESS
}

const MACHINE_SCHEMA_VERSION: &str = "1";

struct JsonDiagnosticCollector {
    path: String,
    source: String,
    diagnostics: Vec<Value>,
}

thread_local! {
    static JSON_DIAGNOSTICS: RefCell<Option<JsonDiagnosticCollector>> = const { RefCell::new(None) };
    static PROJECT_SOURCES: RefCell<Vec<project::ProjectSource>> = const { RefCell::new(Vec::new()) };
    static PROJECT_MODULES: RefCell<Vec<project::ProjectModule>> = const { RefCell::new(Vec::new()) };
}

fn database_usage() {
    eprintln!(
        "Usage:\n  zelyra db create <file.zyl>\n  zelyra db setup <file.zyl>\n  zelyra db bootstrap <file.zyl>\n  zelyra db inspect <file.zyl>\n  zelyra db plan <file.zyl>\n  zelyra db apply <file.zyl> [--allow-risky]\n\n--allow-destructive remains available for DESTRUCTIVE plans only.\nA project database uses ZELYRA_DATABASE_<NAME>_URL (for example ZELYRA_DATABASE_MAIN_URL); DATABASE_URL remains a compatibility fallback."
    );
}

const DEFAULT_WEB_PORT: u16 = 3000;
const DEFAULT_DATABASE_HOST_PORT: u16 = 3306;
const DEFAULT_SETUP_WEB_PORT: u16 = 3030;
const PROJECT_THEME_CSS_FILE: &str = "zelyra.theme.css";
const PROJECT_THEME_CSS_MAX_BYTES: u64 = 128 * 1024;
const PROJECT_LOCALE_DIRECTORY: &str = "locales";
const PROJECT_LOCALE_MAX_BYTES: u64 = 256 * 1024;

struct ProjectOptions {
    allow_current_directory: bool,
    with_mariadb: bool,
    crud_template: bool,
    auth_template: bool,
    business_template: bool,
    web_port: u16,
    host_port: u16,
    database_host_port: u16,
    host_port_given: bool,
    database_host_port_given: bool,
}

#[derive(Default)]
struct SetupOptions {
    host_port: Option<u16>,
    database_host_port: Option<u16>,
}

struct LocalEnvSetup {
    created: bool,
    port_notes: Vec<String>,
}

fn parse_web_port(value: &str) -> Result<u16, String> {
    parse_port(value, "web")
}

fn parse_database_host_port(value: &str) -> Result<u16, String> {
    parse_port(value, "database host")
}

fn parse_port(value: &str, label: &str) -> Result<u16, String> {
    let port = value
        .parse::<u16>()
        .map_err(|_| format!("{label} port `{value}` must be an integer between 1 and 65535"))?;
    if port == 0 {
        return Err(format!("{label} port must be between 1 and 65535"));
    }
    Ok(port)
}

fn port_is_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

fn find_free_port(start: u16, reserved: &[u16]) -> Option<u16> {
    (start..=u16::MAX).find(|port| !reserved.contains(port) && port_is_available(*port))
}

fn resolve_host_port(
    requested: u16,
    explicitly_given: bool,
    label: &str,
    auto_select: bool,
    reserved: &[u16],
) -> Result<(u16, Option<String>), String> {
    if !explicitly_given && !auto_select {
        return Ok((requested, None));
    }
    if port_is_available(requested) && !reserved.contains(&requested) {
        return Ok((requested, None));
    }
    if explicitly_given {
        return Err(format!(
            "{label} port {requested} is already in use; choose a different port"
        ));
    }
    let selected = find_free_port(requested.saturating_add(1), reserved).ok_or_else(|| {
        format!("could not find a free {label} port after {requested}; choose a port explicitly")
    })?;
    Ok((
        selected,
        Some(format!(
            "{label} port {requested} is unavailable; selected free port {selected}"
        )),
    ))
}

fn resolve_project_host_ports(
    path: &str,
    options: &ProjectOptions,
) -> Result<(u16, u16, Vec<String>), String> {
    if !options.with_mariadb {
        return Ok((options.host_port, options.database_host_port, Vec::new()));
    }
    let env_exists = std::path::Path::new(path).join(".env").is_file();
    let auto_select = !env_exists;
    let (host_port, host_note) = resolve_host_port(
        options.host_port,
        options.host_port_given,
        "web host",
        auto_select,
        &[],
    )?;
    let (database_host_port, database_note) = resolve_host_port(
        options.database_host_port,
        options.database_host_port_given,
        "MariaDB host",
        auto_select,
        &[host_port],
    )?;
    let notes = [host_note, database_note].into_iter().flatten().collect();
    Ok((host_port, database_host_port, notes))
}

fn generate_local_secret() -> Result<String, String> {
    let mut bytes = [0_u8; 24];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|error| format!("cannot generate a local secret: {error}"))?;
    let mut secret = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut secret, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(secret)
}

fn mariadb_env_template(web_port: u16, host_port: u16, database_host_port: u16) -> String {
    format!(
        r#"# Zelyra local MariaDB configuration.
# This file is safe to edit locally but must never be committed.
#
# The active values below configure the generated MariaDB Compose project and
# the default German, guided Zelyra experience. Change language to `en` or
# level to `work` for the concise English work interface.
# The database host port is active so the CLI and Compose always use the
# selected port together.
ZELYRA_DB_HOST_PORT={database_host_port}
ZELYRA_LANGUAGE=de
ZELYRA_LEVEL=learn
ZELYRA_ALLOWED_HOSTS=localhost,127.0.0.1,[::1]
ZELYRA_DB_CONNECT_TIMEOUT_SECS=10
ZELYRA_DB_QUERY_TIMEOUT_SECS=30
ZELYRA_DB_POOL_MAX_SIZE=8
ZELYRA_DB_POOL_WAIT_TIMEOUT_SECS=10
# Local Docker MariaDB uses an isolated bridge network. For a remote database,
# use `required` and configure a trusted CA if it is not in the built-in roots.
ZELYRA_DB_TLS_MODE=disabled
# Optional absolute CA path, readable inside the Zelyra container/process.
# ZELYRA_DB_TLS_CA_CERT_FILE=
ZELYRA_DATABASE_MAIN_URL=mariadb://zelyra:change-me@127.0.0.1:${{ZELYRA_DB_HOST_PORT:-3306}}/zelyra_app
MARIADB_DATABASE=zelyra_app
MARIADB_USER=zelyra
MARIADB_PASSWORD=change-me
MARIADB_ROOT_PASSWORD=change-me-root

# Optional web port overrides. The generated Compose file already contains
# the selected defaults below, so these lines can remain commented out.
# ZELYRA_WEB_PORT={web_port}
# ZELYRA_HOST_PORT={host_port}

# Optional public hostnames or IP addresses, comma-separated. Add the host
# used in your browser when serving through a LAN address or reverse proxy.
# ZELYRA_ALLOWED_HOSTS=localhost,127.0.0.1,[::1],app.example.com

# Optional feature switches. They default to true and are normally not needed.
# ZELYRA_FEATURE_WEB=true
# ZELYRA_FEATURE_API=true
# ZELYRA_FEATURE_CRUD=true
# ZELYRA_FEATURE_AUTH=true
# ZELYRA_FEATURE_AUDIT=true

# Optional local request protection and application settings.
# ZELYRA_AUTH_TOKEN=replace-with-a-local-token
# ZELYRA_AUTH_PERMISSIONS=customers.view,customers.edit
# ZELYRA_MODE=development

# Optional CORS settings belong in zelyra.toml and should only be enabled with
# an explicit project decision. Do not put production secrets in this file.
"#
    )
}

fn template_port(template: &str, key: &str, fallback: u16) -> u16 {
    template
        .lines()
        .map(str::trim)
        .map(|line| line.strip_prefix('#').unwrap_or(line).trim())
        .find_map(|line| {
            line.strip_prefix(&format!("{key}="))
                .and_then(|value| parse_port(value.trim(), key).ok())
        })
        .unwrap_or(fallback)
}

fn replace_template_env_assignment(template: &str, key: &str, value: u16, active: bool) -> String {
    let mut replaced = false;
    let mut lines = template
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            let setting = trimmed.strip_prefix('#').unwrap_or(trimmed).trim_start();
            if setting.starts_with(&format!("{key}=")) {
                replaced = true;
                let indentation = &line[..line.len() - trimmed.len()];
                if active {
                    format!("{indentation}{key}={value}")
                } else {
                    format!("{indentation}# {key}={value}")
                }
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>();
    if !replaced {
        lines.push(if active {
            format!("{key}={value}")
        } else {
            format!("# {key}={value}")
        });
    }
    lines.join("\n") + "\n"
}

fn prepared_local_env_template(
    directory: &std::path::Path,
    options: &SetupOptions,
) -> Result<(String, Vec<String>), String> {
    let mut template = local_mariadb_template(directory)?;
    let requested_host_port = options
        .host_port
        .unwrap_or_else(|| template_port(&template, "ZELYRA_HOST_PORT", DEFAULT_WEB_PORT));
    let template_database_host_port =
        template_port(&template, "ZELYRA_DB_HOST_PORT", DEFAULT_DATABASE_HOST_PORT);
    let requested_database_host_port = options
        .database_host_port
        .unwrap_or(template_database_host_port);
    let (host_port, host_note) = resolve_host_port(
        requested_host_port,
        options.host_port.is_some(),
        "web host",
        true,
        &[],
    )?;
    let (database_host_port, database_note) = resolve_host_port(
        requested_database_host_port,
        options.database_host_port.is_some(),
        "MariaDB host",
        true,
        &[host_port],
    )?;
    if database_host_port != template_database_host_port
        && !template.contains("${ZELYRA_DB_HOST_PORT")
    {
        return Err(
            "cannot safely select a MariaDB port because the database URL does not use ${ZELYRA_DB_HOST_PORT:-...}; update the template explicitly or choose a matching free port"
                .into(),
        );
    }
    if options.host_port.is_some() || host_port != requested_host_port {
        template = replace_template_env_assignment(&template, "ZELYRA_HOST_PORT", host_port, true);
    }
    template =
        replace_template_env_assignment(&template, "ZELYRA_DB_HOST_PORT", database_host_port, true);
    Ok((
        template,
        [host_note, database_note].into_iter().flatten().collect(),
    ))
}

fn render_local_env(template: &str) -> Result<String, String> {
    let database_password = generate_local_secret()?;
    let root_password = generate_local_secret()?;
    Ok(template
        .replace(
            "ZELYRA_DATABASE_MAIN_URL=mariadb://zelyra:change-me@127.0.0.1:${ZELYRA_DB_HOST_PORT:-3306}/zelyra_app",
            &format!(
                "ZELYRA_DATABASE_MAIN_URL=mariadb://zelyra:{database_password}@127.0.0.1:${{ZELYRA_DB_HOST_PORT:-3306}}/zelyra_app"
            ),
        )
        .replace(
            "DATABASE_URL=mariadb://zelyra:change-me@127.0.0.1:${ZELYRA_DB_HOST_PORT:-3306}/zelyra_app",
            &format!(
                "DATABASE_URL=mariadb://zelyra:{database_password}@127.0.0.1:${{ZELYRA_DB_HOST_PORT:-3306}}/zelyra_app"
            ),
        )
        .replace(
            "MARIADB_PASSWORD=change-me",
            &format!("MARIADB_PASSWORD={database_password}"),
        )
        .replace(
            "MARIADB_ROOT_PASSWORD=change-me-root",
            &format!("MARIADB_ROOT_PASSWORD={root_password}"),
        ))
}

fn protect_env_file(path: &std::path::Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)
            .map_err(|error| format!("cannot inspect `{}`: {error}", path.display()))?
            .permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(path, permissions)
            .map_err(|error| format!("cannot protect `{}`: {error}", path.display()))?;
    }
    Ok(())
}

fn write_local_env_file(path: &std::path::Path, template: &str) -> Result<bool, String> {
    if path.exists() {
        return Ok(false);
    }
    let contents = render_local_env(template)?;
    fs::write(path, contents)
        .map_err(|error| format!("cannot write `{}`: {error}", path.display()))?;
    protect_env_file(path)?;
    Ok(true)
}

fn local_mariadb_template(directory: &std::path::Path) -> Result<String, String> {
    let example_file = directory.join(".env.example");
    if example_file.is_file() {
        return fs::read_to_string(&example_file)
            .map_err(|error| format!("cannot read `{}`: {error}", example_file.display()));
    }
    let config_file = directory.join("zelyra.toml");
    let is_mariadb_project = fs::read_to_string(&config_file)
        .ok()
        .is_some_and(|config| config.contains("engine = \"mariadb\""));
    if !is_mariadb_project {
        return Err(format!(
            "`{}` has no `.env.example` and no MariaDB project configuration; run `zelyra new <directory> --mariadb` first",
            directory.display()
        ));
    }
    Ok(mariadb_env_template(
        DEFAULT_WEB_PORT,
        DEFAULT_WEB_PORT,
        DEFAULT_DATABASE_HOST_PORT,
    ))
}

fn ensure_local_env_file(
    directory: &std::path::Path,
    options: &SetupOptions,
) -> Result<LocalEnvSetup, String> {
    let env_file = directory.join(".env");
    if env_file.exists() {
        if options.host_port.is_some() || options.database_host_port.is_some() {
            return Err(
                "an existing .env is never changed; update its ZELYRA_HOST_PORT or ZELYRA_DB_HOST_PORT manually"
                    .into(),
            );
        }
        return Ok(LocalEnvSetup {
            created: false,
            port_notes: Vec::new(),
        });
    }
    let (template, port_notes) = prepared_local_env_template(directory, options)?;
    let created = write_local_env_file(&env_file, &template)?;
    Ok(LocalEnvSetup {
        created,
        port_notes,
    })
}

fn setup_project(path: &str, options: &SetupOptions) -> ExitCode {
    let directory = std::path::Path::new(path);
    if !directory.is_dir() {
        eprintln!("error[E-SETUP-001]: project directory `{path}` does not exist");
        return ExitCode::from(1);
    }
    let env_file = directory.join(".env");
    let existed = env_file.exists();
    if !existed && !directory.join(".env.example").is_file() {
        println!("`.env.example` not found; using the safe built-in MariaDB defaults for `{path}`");
    }
    let setup = match ensure_local_env_file(directory, options) {
        Ok(setup) => setup,
        Err(error) => {
            eprintln!("error[E-SETUP-001]: {error}");
            return ExitCode::from(1);
        }
    };
    for note in setup.port_notes {
        println!("note: {note}");
    }
    if existed {
        println!("kept existing {}", env_file.display());
        println!("no credentials were changed or printed");
        return ExitCode::SUCCESS;
    }
    if !setup.created {
        eprintln!(
            "error[E-SETUP-002]: could not create {}",
            env_file.display()
        );
        return ExitCode::from(1);
    }
    println!(
        "created {} with local MariaDB credentials",
        env_file.display()
    );
    println!("credentials were generated locally and are not shown");
    print_compose_start_hint();
    if cfg!(windows) {
        println!("then load .env in your shell and run: zelyra db setup main.zyl");
    } else {
        println!("then run: set -a; . ./.env; set +a; zelyra db setup main.zyl");
    }
    ExitCode::SUCCESS
}

fn create_project(path: &str, mut options: ProjectOptions) -> ExitCode {
    let directory = std::path::Path::new(path);
    if directory.exists() && !options.allow_current_directory {
        eprintln!("error[E-INIT-001]: directory `{path}` already exists");
        return ExitCode::from(1);
    }
    let (host_port, database_host_port, port_notes) =
        match resolve_project_host_ports(path, &options) {
            Ok(ports) => ports,
            Err(error) => {
                eprintln!("error[E-INIT-005]: {error}");
                return ExitCode::from(2);
            }
        };
    options.host_port = host_port;
    options.database_host_port = database_host_port;
    for note in port_notes {
        println!("note: {note}");
    }
    if let Err(error) = fs::create_dir_all(directory) {
        eprintln!("error[E-INIT-002]: cannot create `{path}`: {error}");
        return ExitCode::from(1);
    }
    let database_section = if options.with_mariadb {
        "[database.main]\nengine = \"mariadb\"\n"
    } else {
        ""
    };
    let project_config = format!(
        r#"[project]
name = "zelyra-app"
version = "{version}"
zelyra = "0.1"

{database_section}

[capabilities]
database = true
network = false
console = false
"#,
        version = env!("CARGO_PKG_VERSION"),
        database_section = database_section
    );
    let main_template = if options.business_template {
        MARIADB_BUSINESS_TEMPLATE
    } else if options.crud_template {
        MARIADB_CRUD_TEMPLATE
    } else if options.auth_template {
        MARIADB_AUTH_TEMPLATE
    } else if options.with_mariadb {
        MARIADB_MINIMAL_TEMPLATE
    } else {
        r#"fn main() {
    print("Hello from Zelyra")
}
"#
    };
    let preserve_existing_main =
        options.allow_current_directory && directory.join("main.zyl").is_file();
    let main_source = if options.with_mariadb {
        let source = main_template
            .strip_prefix(MARIADB_DATABASE_DECLARATION)
            .unwrap_or(main_template);
        format!("import \"src/database.zyl\" as storage\n\n{source}")
    } else {
        main_template.to_owned()
    };
    let mut files = vec![
        ("zelyra.toml", project_config.to_owned()),
        ("main.zyl", main_source),
        // Keep the conventional module source directory present so the
        // generated Dockerfile can copy it even for a fresh single-file app.
        ("src/.keep", String::new()),
        (PROJECT_THEME_CSS_FILE, PROJECT_THEME_TEMPLATE.to_owned()),
        ("locales/de.json", "{}\n".to_owned()),
        ("locales/en.json", "{}\n".to_owned()),
    ];
    if options.with_mariadb && !preserve_existing_main {
        files.push(("src/database.zyl", MARIADB_DATABASE_MODULE.to_owned()));
    }
    if options.crud_template {
        files.push((
            "machine-management-demo.sql",
            MACHINE_MANAGEMENT_DEMO_DATA.to_owned(),
        ));
    }
    if options.with_mariadb {
        let env_value = |name: &str| format!("{}{{{name}}}", '$');
        let web_port_value = format!("{}{{ZELYRA_WEB_PORT:-{}}}", '$', options.web_port);
        let host_port_value = format!("{}{{ZELYRA_HOST_PORT:-{}}}", '$', options.host_port);
        let database_host_port_value = format!(
            "{}{{ZELYRA_DB_HOST_PORT:-{}}}",
            '$', options.database_host_port
        );
        files.extend([
            (
                ".env.example",
                mariadb_env_template(
                    options.web_port,
                    options.host_port,
                    options.database_host_port,
                ),
            ),
            (
                "docker-compose.mariadb.yml",
                r#"services:
  mariadb:
    image: mariadb:11
    restart: unless-stopped
    environment:
      MARIADB_DATABASE: __MARIADB_DATABASE__
      MARIADB_USER: __MARIADB_USER__
      MARIADB_PASSWORD: __MARIADB_PASSWORD__
      MARIADB_ROOT_PASSWORD: __MARIADB_ROOT_PASSWORD__
    ports:
      - "127.0.0.1:__DATABASE_HOST_PORT__:3306"
    volumes:
      - zelyra_mariadb_data:/var/lib/mysql
    healthcheck:
      test: ["CMD", "healthcheck.sh", "--connect", "--innodb_initialized"]
      interval: 5s
      timeout: 5s
      retries: 20

  web:
    build: .
    command: ["zelyra", "serve", "main.zyl", "0.0.0.0:__WEB_PORT__"]
    environment:
      ZELYRA_DATABASE_MAIN_URL: mariadb://__MARIADB_USER__:__MARIADB_PASSWORD__@mariadb:3306/__MARIADB_DATABASE__
      DATABASE_URL: mariadb://__MARIADB_USER__:__MARIADB_PASSWORD__@mariadb:3306/__MARIADB_DATABASE__
      ZELYRA_LANGUAGE: __ZELYRA_LANGUAGE__
      ZELYRA_LEVEL: __ZELYRA_LEVEL__
      ZELYRA_ALLOWED_HOSTS: "__ZELYRA_ALLOWED_HOSTS__"
      ZELYRA_DB_CONNECT_TIMEOUT_SECS: ${ZELYRA_DB_CONNECT_TIMEOUT_SECS:-10}
      ZELYRA_DB_QUERY_TIMEOUT_SECS: ${ZELYRA_DB_QUERY_TIMEOUT_SECS:-30}
      ZELYRA_DB_POOL_MAX_SIZE: ${ZELYRA_DB_POOL_MAX_SIZE:-8}
      ZELYRA_DB_POOL_WAIT_TIMEOUT_SECS: ${ZELYRA_DB_POOL_WAIT_TIMEOUT_SECS:-10}
      ZELYRA_DB_TLS_MODE: ${ZELYRA_DB_TLS_MODE:-disabled}
      ZELYRA_DB_TLS_CA_CERT_FILE: ${ZELYRA_DB_TLS_CA_CERT_FILE:-}
    depends_on:
      mariadb:
        condition: service_healthy
    ports:
      - "127.0.0.1:__HOST_PORT__:__WEB_PORT__"

volumes:
  zelyra_mariadb_data:
"#
                .replace("__MARIADB_DATABASE__", &env_value("MARIADB_DATABASE"))
                .replace("__MARIADB_USER__", &env_value("MARIADB_USER"))
                .replace("__MARIADB_PASSWORD__", &env_value("MARIADB_PASSWORD"))
                .replace("__MARIADB_ROOT_PASSWORD__", &env_value("MARIADB_ROOT_PASSWORD"))
                .replace(
                    "__ZELYRA_LANGUAGE__",
                    &format!("{}{{ZELYRA_LANGUAGE:-de}}", '$'),
                )
                .replace("__ZELYRA_LEVEL__", &format!("{}{{ZELYRA_LEVEL:-learn}}", '$'))
                .replace(
                    "__ZELYRA_ALLOWED_HOSTS__",
                    &format!(
                        "{}{{ZELYRA_ALLOWED_HOSTS:-localhost,127.0.0.1,[::1]}}",
                        '$'
                    ),
                )
                .replace("__WEB_PORT__", &web_port_value)
                .replace("__HOST_PORT__", &host_port_value)
                .replace("__DATABASE_HOST_PORT__", &database_host_port_value),
            ),
            (
                "Dockerfile",
                r#"FROM rust:1-bookworm AS build
ARG ZELYRA_REF=__ZELYRA_DEFAULT_REF__
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/*
RUN git clone --depth 1 --branch __ZELYRA_REF__ https://github.com/sf1976/zelyra.git /zelyra
RUN cargo install --locked --path /zelyra/cli --root /out

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates mariadb-client \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /out/bin/zelyra /usr/local/bin/zelyra
COPY main.zyl zelyra.toml zelyra.theme.css ./
COPY src ./src
COPY locales ./locales
EXPOSE __WEB_PORT__
CMD ["zelyra", "serve", "main.zyl", "0.0.0.0:__WEB_PORT__"]
"#
                .replace("__ZELYRA_REF__", &env_value("ZELYRA_REF"))
                .replace(
                    "__ZELYRA_DEFAULT_REF__",
                    &format!("v{}", env!("CARGO_PKG_VERSION")),
                )
                .replace("__WEB_PORT__", &options.web_port.to_string()),
            ),
            (
                ".dockerignore",
                ".git\ntarget\n.env\n.env.*\n*.sqlite3\n*.db\n*.pem\n*.key\n*.p12\n*.pfx\n".to_owned(),
            ),
            (".gitignore", ".env\ntarget/\n".to_owned()),
        ]);
    }
    for (name, contents) in files {
        let file = directory.join(name);
        if file.exists() && options.allow_current_directory {
            continue;
        }
        if let Some(parent) = file.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                eprintln!(
                    "error[E-INIT-003]: cannot create `{}`: {error}",
                    parent.display()
                );
                return ExitCode::from(1);
            }
        }
        let contents = contents.to_owned();
        if let Err(error) = fs::write(&file, contents) {
            eprintln!(
                "error[E-INIT-003]: cannot write `{}`: {error}",
                file.display()
            );
            return ExitCode::from(1);
        }
    }
    println!("created Zelyra project in {}", directory.display());
    if options.with_mariadb {
        let env_file = directory.join(".env");
        let env_example = directory.join(".env.example");
        match write_local_env_file(
            &env_file,
            &mariadb_env_template(
                options.web_port,
                options.host_port,
                options.database_host_port,
            ),
        ) {
            Ok(true) => println!("created protected {}", env_file.display()),
            Ok(false) => println!("kept existing {}", env_file.display()),
            Err(error) => {
                eprintln!("error[E-INIT-004]: {error}");
                return ExitCode::from(1);
            }
        }
        println!("reference template: {}", env_example.display());
        println!("then start MariaDB with:");
        print_compose_start_hint();
        if cfg!(windows) {
            println!("then load .env in your shell and run: zelyra db setup main.zyl");
        } else {
            println!("then run: set -a; . ./.env; set +a; zelyra db setup main.zyl");
        }
    } else {
        println!("next: cd {} && zelyra run main.zyl", path);
    }
    ExitCode::SUCCESS
}

fn begin_json_diagnostics(path: &str, source: &str) {
    JSON_DIAGNOSTICS.with(|collector| {
        *collector.borrow_mut() = Some(JsonDiagnosticCollector {
            path: context_entry(path),
            source: source.into(),
            diagnostics: Vec::new(),
        });
    });
}

fn finish_json_diagnostics() -> Vec<Value> {
    JSON_DIAGNOSTICS.with(|collector| {
        collector
            .borrow_mut()
            .take()
            .map_or_else(Vec::new, |collector| collector.diagnostics)
    })
}

fn source_offset(source: &str, line: usize, column: usize) -> usize {
    if line == 0 {
        return 0;
    }
    let mut current_line = 1;
    let mut offset = 0;
    for line_text in source.split_inclusive('\n') {
        if current_line == line {
            return offset
                + column
                    .saturating_sub(1)
                    .min(line_text.trim_end_matches('\n').len());
        }
        offset += line_text.len();
        current_line += 1;
    }
    if current_line == line {
        offset
            + column
                .saturating_sub(1)
                .min(source.len().saturating_sub(offset))
    } else {
        source.len()
    }
}

fn point_span(source: &str, line: usize, column: usize) -> zelyra_ast::Span {
    let start_offset = source_offset(source, line, column);
    let end = if start_offset < source.len() {
        start_offset
            + source[start_offset..]
                .chars()
                .next()
                .map_or(1, char::len_utf8)
    } else {
        start_offset
    };
    zelyra_ast::Span::new(start_offset, end, line, column)
}

fn span_value(source: &str, span: zelyra_ast::Span) -> Value {
    let (end_line, end_column) = source_position(source, span.end);
    json!({
        "start": { "offset": span.start, "line": span.line, "column": span.column },
        "end": { "offset": span.end, "line": end_line, "column": end_column }
    })
}

fn diagnostic_with_span(path: &str, code: &str, message: &str, span: zelyra_ast::Span) {
    let source =
        PROJECT_SOURCES.with(|sources| sources.borrow().get(span.source_id as usize).cloned());
    let captured = JSON_DIAGNOSTICS.with(|collector| {
        let mut collector = collector.borrow_mut();
        let Some(collector) = collector.as_mut() else {
            return false;
        };
        let file = if span.source_id == 0 && !collector.path.is_empty() {
            collector.path.clone()
        } else {
            source
                .as_ref()
                .map_or_else(|| path.to_owned(), |source| source.path.clone())
        };
        let source_text = source
            .as_ref()
            .map_or(collector.source.as_str(), |source| source.text.as_str());
        collector.diagnostics.push(json!({
            "code": code,
            "severity": "error",
            "message": message,
            "file": file,
            "span": span_value(source_text, span)
        }));
        true
    });
    if !captured {
        eprintln!(
            "error[{code}]: {message}\n\n --> {}:{}:{}",
            source.as_ref().map_or(path, |source| source.path.as_str()),
            span.line,
            span.column
        );
    }
}

fn diagnostic(path: &str, code: &str, message: &str, line: usize, column: usize) {
    let span = JSON_DIAGNOSTICS.with(|collector| {
        collector
            .borrow()
            .as_ref()
            .map(|collector| point_span(&collector.source, line, column))
    });
    diagnostic_with_span(
        path,
        code,
        message,
        span.unwrap_or_else(|| zelyra_ast::Span::new(0, 0, line, column)),
    );
}

fn machine_document(
    command: &str,
    success: bool,
    diagnostics: Vec<Value>,
    fields: impl IntoIterator<Item = (String, Value)>,
) -> Value {
    let mut document = Map::new();
    document.insert(
        "schema_version".into(),
        Value::String(MACHINE_SCHEMA_VERSION.into()),
    );
    document.insert("command".into(), Value::String(command.into()));
    document.insert("success".into(), Value::Bool(success));
    document.insert("diagnostics".into(), Value::Array(diagnostics));
    for (key, value) in fields {
        document.insert(key, value);
    }
    Value::Object(document)
}

fn print_machine_document(document: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(document).expect("machine document must be serializable")
    );
}

fn load(path: &str) -> Result<zelyra_ast::Program, ()> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            diagnostic(
                path,
                "E-IO-001",
                &format!("cannot read `{path}`: {error}"),
                1,
                1,
            );
            return Err(());
        }
    };
    parse_source(path, &source)
}

fn parse_source(path: &str, source: &str) -> Result<zelyra_ast::Program, ()> {
    let tokens = match lex(source) {
        Ok(tokens) => tokens,
        Err(error) => {
            diagnostic_with_span(path, "E-LEX-001", &error.message, error.span);
            return Err(());
        }
    };
    match parse(&tokens) {
        Ok(program) => Ok(program),
        Err(error) => {
            diagnostic_with_span(path, "E-PARSE-001", &error.message, error.span);
            Err(())
        }
    }
}

fn load_project(path: &str) -> Result<project::LoadedProject, ()> {
    PROJECT_SOURCES.with(|sources| sources.borrow_mut().clear());
    PROJECT_MODULES.with(|modules| modules.borrow_mut().clear());
    let loaded = match project::load(path) {
        Ok(loaded) => loaded,
        Err(error) => {
            PROJECT_SOURCES.with(|sources| *sources.borrow_mut() = error.sources.to_vec());
            diagnostic_with_span(&error.path, error.code, &error.message, error.span);
            return Err(());
        }
    };
    PROJECT_SOURCES.with(|sources| *sources.borrow_mut() = loaded.sources.clone());
    PROJECT_MODULES.with(|modules| *modules.borrow_mut() = loaded.modules.clone());
    Ok(loaded)
}

fn validate(path: &str) -> Result<zelyra_ast::Program, ()> {
    let loaded = load_project(path)?;
    if !validate_module_table_dependencies(path, &loaded) {
        return Err(());
    }
    let source = loaded
        .sources
        .first()
        .map_or_else(String::new, |source| source.text.clone());
    validate_program(path, &source, loaded.program)
}

fn validate_module_table_dependencies(path: &str, loaded: &project::LoadedProject) -> bool {
    let module_imports = loaded
        .modules
        .iter()
        .map(|module| {
            (
                module.path.as_str(),
                module
                    .imports
                    .iter()
                    .map(|import| import.path.as_str())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<HashMap<_, _>>();
    let source_paths = loaded
        .sources
        .iter()
        .enumerate()
        .map(|(id, source)| (source.path.as_str(), id as u32))
        .collect::<HashMap<_, _>>();

    let mut table_owners = HashMap::new();
    let mut duplicate_tables = HashSet::new();
    for table in &loaded.program.tables {
        let Some(owner) = loaded
            .sources
            .get(table.span.source_id as usize)
            .map(|source| source.path.as_str())
        else {
            continue;
        };
        if table_owners.insert(table.name.as_str(), owner).is_some() {
            duplicate_tables.insert(table.name.as_str());
        }
    }

    let impact = build_impact_with_sources(&loaded.program, &loaded.sources, "");
    let mut violations = BTreeMap::<(String, String), (zelyra_ast::Span, BTreeSet<String>)>::new();
    for reference in impact
        .get("references")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(table_name) = reference
            .get("to")
            .and_then(Value::as_str)
            .and_then(|target| target.strip_prefix("table:"))
        else {
            continue;
        };
        if duplicate_tables.contains(table_name) {
            continue;
        }
        let Some(span_value) = reference.get("span") else {
            continue;
        };
        let Some(source_path) = span_value.get("file").and_then(Value::as_str) else {
            continue;
        };
        let Some(owner_path) = table_owners.get(table_name).copied() else {
            continue;
        };
        if source_path == owner_path {
            continue;
        }
        let mut pending = vec![source_path];
        let mut visited = HashSet::new();
        let mut reaches_owner = false;
        while let Some(module_path) = pending.pop() {
            if !visited.insert(module_path) {
                continue;
            }
            if module_path == owner_path {
                reaches_owner = true;
                break;
            }
            pending.extend(
                module_imports
                    .get(module_path)
                    .into_iter()
                    .flatten()
                    .copied(),
            );
        }
        if reaches_owner {
            continue;
        }

        let Some(source_id) = source_paths.get(source_path).copied() else {
            continue;
        };
        let start = span_value
            .pointer("/start/offset")
            .and_then(Value::as_u64)
            .unwrap_or_default() as usize;
        let end = span_value
            .pointer("/end/offset")
            .and_then(Value::as_u64)
            .unwrap_or(start as u64) as usize;
        let line = span_value
            .pointer("/start/line")
            .and_then(Value::as_u64)
            .unwrap_or(1) as usize;
        let column = span_value
            .pointer("/start/column")
            .and_then(Value::as_u64)
            .unwrap_or(1) as usize;
        let span = zelyra_ast::Span::new(start, end, line, column).with_source_id(source_id);
        let key = (source_path.to_owned(), table_name.to_owned());
        let entry = violations
            .entry(key)
            .or_insert_with(|| (span, BTreeSet::new()));
        if let Some(access) = reference.get("access").and_then(Value::as_str) {
            entry.1.insert(access.to_owned());
        }
    }

    for ((source_path, table_name), (span, accesses)) in &violations {
        let owner_path = table_owners[table_name.as_str()];
        let access = if accesses.is_empty() {
            String::new()
        } else {
            format!(
                " ({})",
                accesses.iter().cloned().collect::<Vec<_>>().join(", ")
            )
        };
        diagnostic_with_span(
            path,
            "E-MOD-019",
            &format!(
                "module `{source_path}` references table `{table_name}`{access}, owned by `{owner_path}`, but that module is not in its import dependency graph; import the table-owning module explicitly"
            ),
            *span,
        );
    }
    violations.is_empty()
}

fn validate_program(
    path: &str,
    source: &str,
    program: zelyra_ast::Program,
) -> Result<zelyra_ast::Program, ()> {
    if !reject_typed_holes(source, path, &program) {
        return Err(());
    }
    validate_project_features(path, &program)?;
    if let Err(errors) = lower(&program) {
        for error in errors {
            diagnostic_with_span(path, "E-NAME-001", &error.message, error.span);
        }
        return Err(());
    }
    if !program.functions.is_empty() {
        if let Err(errors) = check(&program) {
            for error in errors {
                diagnostic_with_span(path, "E-TYPE-001", &error.message, error.span);
            }
            return Err(());
        }
    }
    if validate_capabilities(path, &program).is_err() {
        return Err(());
    }
    if let Err(error) = project_filesystem_policy(path) {
        diagnostic(path, "E-FS-002", &error, 1, 1);
        return Err(());
    }
    if let Err(error) = project_network_policy(path) {
        diagnostic(path, "E-NET-002", &error, 1, 1);
        return Err(());
    }
    if let Err(error) = project_process_policy(path) {
        diagnostic(path, "E-PROC-002", &error, 1, 1);
        return Err(());
    }
    if let Err(error) = project_cors_policy(path) {
        diagnostic(path, "E-WEB-004", &error, 1, 1);
        return Err(());
    }
    if let Err(errors) = check_apis(&program) {
        for error in errors {
            diagnostic_with_span(path, "E-API-001", &error.message, error.span);
        }
        return Err(());
    }
    if !validate_views(path, &program) {
        return Err(());
    }
    if !validate_page_inputs(path, &program) {
        return Err(());
    }
    if !validate_page_data(path, &program) {
        return Err(());
    }
    if !validate_components(path, &program) {
        return Err(());
    }
    let schema = match build_schema(&program) {
        Ok(schema) => schema,
        Err(errors) => {
            for error in errors {
                diagnostic_with_span(path, "E-DB-001", &error.message, error.span);
            }
            return Err(());
        }
    };
    {
        if !validate_auth(path, &program, &schema) {
            return Err(());
        }
        if !validate_cruds(path, &program, &schema) {
            return Err(());
        }
        if !validate_tableviews(path, &program, &schema) {
            return Err(());
        }
        if let Err(errors) = check_sql_program(&program, &schema) {
            for error in errors {
                diagnostic_with_span(path, "E-SQL-004", &error.message, error.span);
            }
            return Err(());
        }
        if let Err(errors) = check_form_program(&program, &schema) {
            for error in errors {
                diagnostic_with_span(path, "E-FORM-001", &error.message, error.span);
            }
            return Err(());
        }
    }
    Ok(program)
}

fn reject_typed_holes(source: &str, path: &str, program: &zelyra_ast::Program) -> bool {
    let holes = collect_typed_holes(program);
    for hole in &holes {
        let expected = hole
            .expected_type
            .as_ref()
            .map_or_else(|| "unknown".to_owned(), ToString::to_string);
        let values = if hole.visible_values.is_empty() {
            "none".to_owned()
        } else {
            hole.visible_values.join(", ")
        };
        let functions = if hole.visible_functions.is_empty() {
            "none".to_owned()
        } else {
            hole.visible_functions.join(", ")
        };
        let capabilities = if hole.capabilities.is_empty() {
            "none".to_owned()
        } else {
            hole.capabilities.join(", ")
        };
        let contracts = if hole.contract_spans.is_empty() {
            "none".to_owned()
        } else {
            hole.contract_spans
                .iter()
                .filter_map(|span| {
                    source_text_for_span(source, *span)
                        .get(span.start..span.end)
                        .map(str::to_owned)
                })
                .map(|contract| contract.replace(['\n', '\r'], " "))
                .collect::<Vec<_>>()
                .join("; ")
        };
        let message = format!(
            "typed hole `_` is incomplete and cannot be built; expected type: {expected}; visible values: {values}; visible functions: {functions}; capabilities: {capabilities}; contract obligations: {contracts}"
        );
        diagnostic_with_span(path, "E-HOLE-001", &message, hole.span);
    }
    holes.is_empty()
}

fn source_text_for_span(fallback: &str, span: zelyra_ast::Span) -> String {
    PROJECT_SOURCES.with(|sources| {
        sources
            .borrow()
            .get(span.source_id as usize)
            .map_or_else(|| fallback.to_owned(), |source| source.text.clone())
    })
}

fn validate_views(path: &str, program: &zelyra_ast::Program) -> bool {
    let mut valid = true;
    let mut names = HashSet::new();
    for view in &program.views {
        if !names.insert(view.name.as_str()) {
            diagnostic_with_span(
                path,
                "E-VIEW-001",
                &format!("duplicate view definition `{}`", view.name),
                view.span,
            );
            valid = false;
        }
        let slots = match slot_invocations(&view.html) {
            Ok(slots) => slots,
            Err(message) => {
                diagnostic_with_span(
                    path,
                    "E-VIEW-028",
                    &format!("view `{}` has invalid slots: {message}", view.name),
                    view.span,
                );
                valid = false;
                Vec::new()
            }
        };
        let default_slots = slots.iter().filter(|slot| slot.name.is_none()).count();
        if default_slots != 1 {
            diagnostic_with_span(
                path,
                "E-VIEW-002",
                &format!(
                    "view `{}` must contain exactly one default `<slot />` content slot (found {default_slots})",
                    view.name,
                ),
                view.span,
            );
            valid = false;
        }
        let mut named_slots = HashSet::new();
        for slot in slots.iter().filter_map(|slot| slot.name.as_deref()) {
            if !named_slots.insert(slot) {
                diagnostic_with_span(
                    path,
                    "E-VIEW-028",
                    &format!(
                        "view `{}` declares named slot `{slot}` more than once",
                        view.name
                    ),
                    view.span,
                );
                valid = false;
            }
        }
    }
    for page in &program.pages {
        if let Some(view_name) = &page.view {
            let Some(view) = program.views.iter().find(|view| view.name == *view_name) else {
                diagnostic(
                    path,
                    "E-VIEW-003",
                    &format!("page `{}` refers to unknown view `{view_name}`", page.path),
                    page.span.line,
                    page.span.column,
                );
                valid = false;
                continue;
            };
            if let Err(message) = validate_view_content_slots(view, &page.html) {
                diagnostic(
                    path,
                    "E-VIEW-029",
                    &format!(
                        "page `{}` has invalid slots for view `{view_name}`: {message}",
                        page.path
                    ),
                    page.span.line,
                    page.span.column,
                );
                valid = false;
            }
        } else {
            match page_layout_slot_invocations(&page.html) {
                Ok(slots) if !slots.is_empty() => {
                    diagnostic(
                        path,
                        "E-VIEW-029",
                        "page content slots require a `view: ...` layout",
                        page.span.line,
                        page.span.column,
                    );
                    valid = false;
                }
                Ok(_) => {}
                Err(message) => {
                    diagnostic(
                        path,
                        "E-VIEW-029",
                        &format!("page `{}` has invalid slots: {message}", page.path),
                        page.span.line,
                        page.span.column,
                    );
                    valid = false;
                }
            }
        }
    }
    for crud in &program.cruds {
        if let Some(layout) = &crud.layout {
            if let Some(view) = program.views.iter().find(|view| view.name == *layout) {
                if let Err((slot_index, message)) =
                    validate_crud_layout_slots(view, &crud.layout_slots)
                {
                    let span = crud
                        .layout_slots
                        .get(slot_index)
                        .map(|slot| slot.span)
                        .unwrap_or(crud.span);
                    diagnostic(
                        path,
                        "E-VIEW-031",
                        &format!(
                            "CRUD `{}` has invalid content for layout `{layout}`: {message}",
                            crud.name
                        ),
                        span.line,
                        span.column,
                    );
                    valid = false;
                }
            } else {
                diagnostic(
                    path,
                    "E-VIEW-030",
                    &format!(
                        "CRUD `{}` refers to unknown view layout `{layout}`",
                        crud.name
                    ),
                    crud.span.line,
                    crud.span.column,
                );
                valid = false;
            }
        } else if !crud.layout_slots.is_empty() {
            let slot = &crud.layout_slots[0];
            diagnostic(
                path,
                "E-VIEW-031",
                &format!(
                    "CRUD `{}` supplies layout slots but has no `layout: ...` reference",
                    crud.name
                ),
                slot.span.line,
                slot.span.column,
            );
            valid = false;
        }
    }
    valid
}

fn validate_page_data(path: &str, program: &zelyra_ast::Program) -> bool {
    let mut valid = true;
    for page in &program.pages {
        if page.page_size.is_some()
            && !page
                .data
                .iter()
                .any(|data| matches!(data.result_type, Type::Array(_)))
        {
            diagnostic(
                path,
                "E-VIEW-021",
                "`paginated` requires at least one page collection loaded with an array result type",
                page.span.line,
                page.span.column,
            );
            valid = false;
        }
        let route_names = page_template_bindings(
            &page.path,
            &[],
            &page.inputs,
            page.page_size,
            !page.sort.is_empty(),
            !page.search.is_empty(),
            &page.filters,
        );
        let mut names = HashSet::new();
        for data in &page.data {
            if !names.insert(data.name.as_str()) {
                diagnostic(
                    path,
                    "E-VIEW-016",
                    &format!("page data `{}` is declared more than once", data.name),
                    data.span.line,
                    data.span.column,
                );
                valid = false;
            }
            if route_names.contains_key(&data.name) {
                diagnostic(
                    path,
                    "E-VIEW-016",
                    &format!(
                        "page data `{}` conflicts with a route parameter or page input",
                        data.name
                    ),
                    data.span.line,
                    data.span.column,
                );
                valid = false;
            }
            if !page_data_type_supported(program, &data.result_type) {
                diagnostic(
                    path,
                    "E-VIEW-017",
                    &format!(
                        "page data `{}` must load a named record or table value, found `{}`",
                        data.name, data.result_type
                    ),
                    data.span.line,
                    data.span.column,
                );
                valid = false;
            }
        }
        if !page.sort.is_empty() {
            let collection_data = page
                .data
                .iter()
                .filter(|data| matches!(data.result_type, Type::Array(_)))
                .collect::<Vec<_>>();
            if collection_data.is_empty() {
                diagnostic(
                    path,
                    "E-VIEW-022",
                    "page `sort` requires at least one collection loaded with an array result type",
                    page.span.line,
                    page.span.column,
                );
                valid = false;
            } else {
                let mut sort_fields = HashSet::new();
                for field in &page.sort {
                    if !sort_fields.insert(field.as_str()) {
                        diagnostic(
                            path,
                            "E-VIEW-022",
                            &format!("page sort field `{field}` is declared more than once"),
                            page.span.line,
                            page.span.column,
                        );
                        valid = false;
                    }
                    if !collection_data.iter().all(|data| {
                        page_collection_fields(program, &data.result_type)
                            .is_some_and(|fields| fields.iter().any(|candidate| candidate == field))
                    }) {
                        diagnostic(
                            path,
                            "E-VIEW-023",
                            &format!(
                                "page sort field `{field}` does not exist in every collection result type"
                            ),
                            page.span.line,
                            page.span.column,
                        );
                        valid = false;
                    }
                }
            }
        }
        if !page.search.is_empty() {
            let collection_data = page
                .data
                .iter()
                .filter(|data| matches!(data.result_type, Type::Array(_)))
                .collect::<Vec<_>>();
            if collection_data.is_empty() {
                diagnostic(
                    path,
                    "E-VIEW-024",
                    "page `search` requires at least one collection loaded with an array result type",
                    page.span.line,
                    page.span.column,
                );
                valid = false;
            } else {
                let mut search_fields = HashSet::new();
                for field in &page.search {
                    if !search_fields.insert(field.as_str()) {
                        diagnostic(
                            path,
                            "E-VIEW-024",
                            &format!("page search field `{field}` is declared more than once"),
                            page.span.line,
                            page.span.column,
                        );
                        valid = false;
                    }
                    if !collection_data.iter().all(|data| {
                        page_collection_fields(program, &data.result_type)
                            .is_some_and(|fields| fields.iter().any(|candidate| candidate == field))
                    }) {
                        diagnostic(
                            path,
                            "E-VIEW-025",
                            &format!(
                                "page search field `{field}` does not exist in every collection result type"
                            ),
                            page.span.line,
                            page.span.column,
                        );
                        valid = false;
                    }
                }
            }
        }
        if !page.filters.is_empty() {
            let collection_data = page
                .data
                .iter()
                .filter(|data| matches!(data.result_type, Type::Array(_)))
                .collect::<Vec<_>>();
            if collection_data.is_empty() {
                diagnostic(
                    path,
                    "E-VIEW-026",
                    "page `filter` requires at least one collection loaded with an array result type",
                    page.span.line,
                    page.span.column,
                );
                valid = false;
            } else {
                let mut filter_fields = HashSet::new();
                for field in &page.filters {
                    if !filter_fields.insert(field.as_str()) {
                        diagnostic(
                            path,
                            "E-VIEW-026",
                            &format!("page filter field `{field}` is declared more than once"),
                            page.span.line,
                            page.span.column,
                        );
                        valid = false;
                    }
                    if !collection_data.iter().all(|data| {
                        page_collection_fields(program, &data.result_type)
                            .is_some_and(|fields| fields.iter().any(|candidate| candidate == field))
                    }) {
                        diagnostic(
                            path,
                            "E-VIEW-027",
                            &format!(
                                "page filter field `{field}` does not exist in every collection result type"
                            ),
                            page.span.line,
                            page.span.column,
                        );
                        valid = false;
                    }
                }
            }
        }
    }
    valid
}

fn validate_page_inputs(path: &str, program: &zelyra_ast::Program) -> bool {
    let mut valid = true;
    for page in &program.pages {
        let route_names =
            page_template_bindings(&page.path, &[], &[], page.page_size, false, false, &[]);
        let mut names = HashSet::new();
        for input in &page.inputs {
            if !names.insert(input.name.as_str()) {
                diagnostic(
                    path,
                    "E-VIEW-019",
                    &format!("page input `{}` is declared more than once", input.name),
                    input.span.line,
                    input.span.column,
                );
                valid = false;
            }
            if route_names.contains_key(&input.name) {
                diagnostic(
                    path,
                    "E-VIEW-019",
                    &format!(
                        "page input `{}` conflicts with a route parameter",
                        input.name
                    ),
                    input.span.line,
                    input.span.column,
                );
                valid = false;
            }
            if page.page_size.is_some() && input.name == "page" {
                diagnostic(
                    path,
                    "E-VIEW-019",
                    "page input `page` is reserved by `paginated`",
                    input.span.line,
                    input.span.column,
                );
                valid = false;
            }
            if page.page_size.is_some()
                && matches!(
                    input.name.as_str(),
                    "zelyra_page_limit" | "zelyra_page_offset"
                )
            {
                diagnostic(
                    path,
                    "E-VIEW-019",
                    &format!(
                        "page input `{}` is reserved for pagination internals",
                        input.name
                    ),
                    input.span.line,
                    input.span.column,
                );
                valid = false;
            }
            if page.page_size.is_some() && matches!(input.name.as_str(), "total" | "pages") {
                diagnostic(
                    path,
                    "E-VIEW-019",
                    &format!("page input `{}` is reserved by `paginated`", input.name),
                    input.span.line,
                    input.span.column,
                );
                valid = false;
            }
            if !page.sort.is_empty() && matches!(input.name.as_str(), "sort" | "order") {
                diagnostic(
                    path,
                    "E-VIEW-019",
                    &format!("page input `{}` is reserved by `sort`", input.name),
                    input.span.line,
                    input.span.column,
                );
                valid = false;
            }
            if !page.search.is_empty() && input.name == "search" {
                diagnostic(
                    path,
                    "E-VIEW-019",
                    "page input `search` is reserved by `search`",
                    input.span.line,
                    input.span.column,
                );
                valid = false;
            }
            if !page.search.is_empty() && input.name == "zelyra_page_search" {
                diagnostic(
                    path,
                    "E-VIEW-019",
                    "page input `zelyra_page_search` is reserved for search internals",
                    input.span.line,
                    input.span.column,
                );
                valid = false;
            }
            if !page.filters.is_empty()
                && page.filters.iter().any(|field| {
                    input.name == format!("filter_{field}")
                        || input.name == format!("filter_{field}__operator")
                })
            {
                diagnostic(
                    path,
                    "E-VIEW-019",
                    &format!("page input `{}` is reserved by `filter`", input.name),
                    input.span.line,
                    input.span.column,
                );
                valid = false;
            }
            if !page_input_type_supported(&input.ty) {
                diagnostic(
                    path,
                    "E-VIEW-020",
                    &format!(
                        "page input `{}` must use a scalar or optional scalar type, found `{}`",
                        input.name, input.ty
                    ),
                    input.span.line,
                    input.span.column,
                );
                valid = false;
            }
        }
    }
    valid
}

fn page_input_type_supported(ty: &Type) -> bool {
    let ty = match ty {
        Type::Option(inner) => inner.as_ref(),
        other => other,
    };
    matches!(
        ty,
        Type::Int
            | Type::UInt
            | Type::Float
            | Type::Decimal
            | Type::Bool
            | Type::String
            | Type::Char
            | Type::Bytes
            | Type::Timestamp
            | Type::Date
            | Type::Time
            | Type::Duration
            | Type::Named(_)
    )
}

fn page_data_type_supported(program: &zelyra_ast::Program, ty: &Type) -> bool {
    let ty = match ty {
        Type::Array(inner) => inner.as_ref(),
        other => other,
    };
    let Type::Named(name) = ty else {
        return false;
    };
    program.records.iter().any(|record| record.name == *name)
        || program.tables.iter().any(|table| {
            table.name == *name || singular_type_name(&table.name).as_deref() == Some(name)
        })
}

fn page_collection_fields(program: &zelyra_ast::Program, ty: &Type) -> Option<Vec<String>> {
    let Type::Array(inner) = ty else {
        return None;
    };
    let Type::Named(name) = inner.as_ref() else {
        return None;
    };
    if let Some(record) = program.records.iter().find(|record| record.name == *name) {
        return Some(
            record
                .fields
                .iter()
                .map(|field| field.name.clone())
                .collect(),
        );
    }
    program
        .tables
        .iter()
        .find(|table| {
            table.name == *name || singular_type_name(&table.name).as_deref() == Some(name)
        })
        .map(|table| {
            table
                .columns
                .iter()
                .map(|column| column.name.clone())
                .collect()
        })
}

fn page_collection_field_type(
    program: &zelyra_ast::Program,
    ty: &Type,
    field_name: &str,
) -> Option<Type> {
    let Type::Array(inner) = ty else {
        return None;
    };
    let Type::Named(name) = inner.as_ref() else {
        return None;
    };
    if let Some(record) = program.records.iter().find(|record| record.name == *name) {
        return record
            .fields
            .iter()
            .find(|field| field.name == field_name)
            .map(|field| field.ty.clone());
    }
    program
        .tables
        .iter()
        .find(|table| {
            table.name == *name || singular_type_name(&table.name).as_deref() == Some(name)
        })
        .and_then(|table| {
            table
                .columns
                .iter()
                .find(|column| column.name == field_name)
                .map(|column| column.ty.clone())
        })
}

fn page_filter_kind(
    program: &zelyra_ast::Program,
    result_type: &Type,
    field_name: &str,
) -> TableViewFilterKind {
    page_collection_field_type(program, result_type, field_name)
        .map(|ty| tableview_type_filter_kind(&ty))
        .unwrap_or(TableViewFilterKind::Other)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputFormat {
    Human,
    Json,
}

fn parse_output_format(value: &str) -> Option<OutputFormat> {
    match value {
        "human" => Some(OutputFormat::Human),
        "json" => Some(OutputFormat::Json),
        _ => None,
    }
}

fn check_command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let Some(path) = arguments.next() else {
        usage();
        return ExitCode::from(2);
    };
    let mut format = OutputFormat::Human;
    while let Some(argument) = arguments.next() {
        if argument == "--format=json" {
            format = OutputFormat::Json;
        } else if argument == "--format=human" {
            format = OutputFormat::Human;
        } else if argument.starts_with("--format=") {
            eprintln!("error[E-CLI-001]: format must be `human` or `json`");
            return ExitCode::from(2);
        } else if argument == "--format" {
            format = match arguments.next().as_deref().and_then(parse_output_format) {
                Some(format) => format,
                None => {
                    eprintln!("error[E-CLI-001]: format must be `human` or `json`");
                    return ExitCode::from(2);
                }
            };
        } else {
            eprintln!("error[E-CLI-001]: unknown check option `{argument}`");
            return ExitCode::from(2);
        }
    }
    if format == OutputFormat::Human {
        return if validate(&path).is_ok() {
            println!("ok: {path}");
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }
    let source = fs::read_to_string(&path).unwrap_or_default();
    begin_json_diagnostics(&path, &source);
    let success = validate(&path).is_ok();
    let diagnostics = finish_json_diagnostics();
    print_machine_document(&machine_document(
        "check",
        success,
        diagnostics,
        std::iter::empty(),
    ));
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn fmt_command(arguments: impl Iterator<Item = String>) -> ExitCode {
    let mut path = None;
    let mut check_only = false;
    for argument in arguments {
        if argument == "--check" && !check_only {
            check_only = true;
        } else if !argument.starts_with('-') && path.is_none() {
            path = Some(argument);
        } else {
            usage();
            return ExitCode::from(2);
        }
    }
    let Some(path) = path else {
        usage();
        return ExitCode::from(2);
    };
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            diagnostic(
                &path,
                "E-IO-001",
                &format!("cannot read `{path}`: {error}"),
                1,
                1,
            );
            return ExitCode::from(1);
        }
    };
    let tokens = match lex(&source) {
        Ok(tokens) => tokens,
        Err(error) => {
            diagnostic_with_span(&path, "E-LEX-001", &error.message, error.span);
            return ExitCode::from(1);
        }
    };
    if let Err(error) = parse(&tokens) {
        diagnostic_with_span(&path, "E-PARSE-001", &error.message, error.span);
        return ExitCode::from(1);
    }
    let formatted = format_source(&source, &tokens);
    if check_only {
        if source == formatted {
            println!("ok: {path}");
            ExitCode::SUCCESS
        } else {
            eprintln!("would reformat: {path}");
            ExitCode::from(1)
        }
    } else if source == formatted {
        println!("already formatted: {path}");
        ExitCode::SUCCESS
    } else {
        match fs::write(&path, formatted) {
            Ok(()) => {
                println!("formatted: {path}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error[E-FMT-001]: cannot write `{path}`: {error}");
                ExitCode::from(1)
            }
        }
    }
}

fn impact_command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let Some(path) = arguments.next() else {
        usage();
        return ExitCode::from(2);
    };
    let mut format = OutputFormat::Human;
    let mut focus = None;
    while let Some(argument) = arguments.next() {
        if argument == "--format=json" {
            format = OutputFormat::Json;
        } else if argument == "--format=human" {
            format = OutputFormat::Human;
        } else if let Some(value) = argument.strip_prefix("--symbol=") {
            if value.is_empty() || focus.replace(value.to_owned()).is_some() {
                eprintln!("error[E-CLI-001]: impact accepts one non-empty `--symbol` value");
                return ExitCode::from(2);
            }
        } else if argument == "--symbol" {
            let Some(value) = arguments
                .next()
                .filter(|value| !value.is_empty() && !value.starts_with('-'))
            else {
                eprintln!("error[E-CLI-001]: `--symbol` requires a non-empty value");
                return ExitCode::from(2);
            };
            if focus.replace(value).is_some() {
                eprintln!("error[E-CLI-001]: impact accepts one `--symbol` value");
                return ExitCode::from(2);
            }
        } else if argument == "--format" {
            format = match arguments.next().as_deref().and_then(parse_output_format) {
                Some(format) => format,
                None => {
                    eprintln!("error[E-CLI-001]: format must be `human` or `json`");
                    return ExitCode::from(2);
                }
            };
        } else {
            eprintln!("error[E-CLI-001]: unknown impact option `{argument}`");
            return ExitCode::from(2);
        }
    }
    if format == OutputFormat::Json {
        let source = fs::read_to_string(&path).unwrap_or_default();
        begin_json_diagnostics(&path, &source);
        let project = load_project(&path);
        let mut success = project.is_ok();
        let impact = if let Ok(project) = project.as_ref() {
            let fallback_source = project
                .sources
                .first()
                .map_or(source.as_str(), |source| source.text.as_str());
            let full_impact =
                build_impact_with_sources(&project.program, &project.sources, fallback_source);
            match focus.as_deref() {
                Some(query) => match focus_impact(&full_impact, query) {
                    Ok(focused) => focused,
                    Err(error) => {
                        diagnostic(&path, "E-IMPACT-001", &error, 1, 1);
                        success = false;
                        json!({})
                    }
                },
                None => full_impact,
            }
        } else {
            json!({})
        };
        let diagnostics = finish_json_diagnostics();
        print_machine_document(&machine_document(
            "impact",
            success,
            diagnostics,
            [("entry".to_owned(), Value::String(context_entry(&path)))]
                .into_iter()
                .chain([("impact".to_owned(), impact)]),
        ));
        return if success {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }
    let project = match load_project(&path) {
        Ok(project) => project,
        Err(()) => return ExitCode::from(1),
    };
    let source = fs::read_to_string(&path).unwrap_or_default();
    let fallback_source = project
        .sources
        .first()
        .map_or(source.as_str(), |source| source.text.as_str());
    let impact = build_impact_with_sources(&project.program, &project.sources, fallback_source);
    let impact = match focus.as_deref() {
        Some(query) => match focus_impact(&impact, query) {
            Ok(focused) => focused,
            Err(error) => {
                eprintln!("error[E-IMPACT-001]: {error}");
                return ExitCode::from(1);
            }
        },
        None => impact,
    };
    println!("impact: {path}");
    if let Some(query) = focus {
        println!("  focus: {query}");
        println!(
            "  references: {}",
            impact["references"].as_array().map_or(0, Vec::len)
        );
        println!(
            "  related: {}",
            impact["related"].as_array().map_or(0, Vec::len)
        );
        return ExitCode::SUCCESS;
    }
    for category in [
        "tables",
        "sql",
        "references",
        "forms",
        "crud",
        "views",
        "apis",
        "permissions",
        "contracts",
        "emails",
        "jobs",
        "tests",
    ] {
        let count = impact[category].as_array().map_or(0, Vec::len);
        println!("  {category}: {count}");
    }
    println!("  schema_changes: source-only");
    ExitCode::SUCCESS
}

fn edit_command(arguments: impl Iterator<Item = String>) -> ExitCode {
    let mut request_path = None;
    let mut json_format = false;
    let mut apply_requested = false;
    for argument in arguments {
        if argument == "--format=json" {
            json_format = true;
        } else if argument == "--apply" {
            apply_requested = true;
        } else if argument == "--format" {
            eprintln!("error[E-CLI-001]: edit requires `--format=json`");
            return ExitCode::from(2);
        } else if argument.starts_with('-') {
            eprintln!("error[E-CLI-001]: unknown edit option `{argument}`");
            return ExitCode::from(2);
        } else if request_path.replace(argument).is_some() {
            eprintln!("error[E-CLI-001]: edit accepts one change request");
            return ExitCode::from(2);
        }
    }
    let Some(request_path) = request_path else {
        usage();
        return ExitCode::from(2);
    };
    if !json_format {
        eprintln!("error[E-CLI-001]: edit requires `--format=json`");
        return ExitCode::from(2);
    }

    let request_source = match fs::read_to_string(&request_path) {
        Ok(source) => source,
        Err(error) => {
            return edit_error_document(
                &request_path,
                "E-IO-001",
                &format!("cannot read `{request_path}`: {error}"),
            )
        }
    };
    let request: Value = match serde_json::from_str(&request_source) {
        Ok(request) => request,
        Err(error) => {
            return edit_error_document(
                &request_path,
                "E-EDIT-001",
                &format!("invalid edit request JSON: {error}"),
            )
        }
    };
    if let Err(error) = edit::validate_request(&request) {
        return edit_error_document(&request_path, "E-EDIT-001", &error);
    }
    let entry = match edit::request_entry(&request) {
        Ok(entry) => entry,
        Err(error) => return edit_error_document(&request_path, "E-EDIT-001", &error),
    };
    let (entry, entry_display) = match edit::resolve_entry(&entry) {
        Ok(entry) => entry,
        Err(error) => return edit_error_document(&entry, "E-EDIT-005", &error),
    };
    let source = match fs::read_to_string(&entry) {
        Ok(source) => source,
        Err(error) => {
            return edit_error_document(
                &entry,
                "E-IO-001",
                &format!("cannot read `{entry}`: {error}"),
            )
        }
    };

    begin_json_diagnostics(&entry, &source);
    let mut success = false;
    let mut preview = json!({"available": false});
    let current_fingerprint = edit::source_fingerprint(&source);
    let expected_fingerprint = request
        .get("expected_source_fingerprint")
        .and_then(Value::as_str);
    if apply_requested && expected_fingerprint != Some(current_fingerprint.as_str()) {
        diagnostic(
            &entry,
            "E-EDIT-004",
            "--apply requires a matching `expected_source_fingerprint`; run a preview first",
            1,
            1,
        );
    } else {
        match lex(&source) {
            Ok(tokens) => match parse(&tokens) {
                Ok(program) => {
                    if validate_program(&entry, &source, program.clone()).is_ok() {
                        match edit::preview(&program, &source, &tokens, &request) {
                            Ok(result) => {
                                let candidate_source = result.source.clone();
                                let _ = finish_json_diagnostics();
                                begin_json_diagnostics(&entry, &candidate_source);
                                if let Ok(proposed_program) =
                                    parse_source(&entry, &candidate_source)
                                {
                                    if validate_program(&entry, &candidate_source, proposed_program)
                                        .is_ok()
                                    {
                                        let applied = if apply_requested {
                                            match edit::apply_atomically(&entry, &result.source) {
                                                Ok(()) => true,
                                                Err(error) => {
                                                    diagnostic(&entry, "E-EDIT-003", &error, 1, 1);
                                                    false
                                                }
                                            }
                                        } else {
                                            false
                                        };
                                        success = !apply_requested || applied;
                                        preview = json!({
                                            "available": true,
                                            "apply_requested": apply_requested,
                                            "applied": applied,
                                            "entry": entry_display.clone(),
                                            "source_fingerprint": current_fingerprint,
                                            "operations": result.operations,
                                            "changes": result.changes,
                                            "changed_tokens": result.changed_tokens,
                                            "before_bytes": source.len(),
                                            "after_bytes": result.source.len()
                                        });
                                    }
                                }
                            }
                            Err(error) => diagnostic(&entry, "E-EDIT-001", &error, 1, 1),
                        }
                    }
                }
                Err(error) => {
                    diagnostic_with_span(&entry, "E-PARSE-001", &error.message, error.span)
                }
            },
            Err(error) => diagnostic_with_span(&entry, "E-LEX-001", &error.message, error.span),
        }
    }
    let diagnostics = finish_json_diagnostics();
    print_machine_document(&machine_document(
        "edit",
        success,
        diagnostics,
        [
            ("request".into(), Value::String(request_path)),
            ("entry".into(), Value::String(entry_display)),
            ("preview".into(), preview),
        ],
    ));
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn edit_error_document(path: &str, code: &str, message: &str) -> ExitCode {
    begin_json_diagnostics(path, "");
    diagnostic(path, code, message, 1, 1);
    let diagnostics = finish_json_diagnostics();
    print_machine_document(&machine_document(
        "edit",
        false,
        diagnostics,
        [("request".into(), Value::String(path.into()))],
    ));
    ExitCode::from(1)
}

fn context_span(fallback_source: &str, span: zelyra_ast::Span) -> Value {
    let source =
        PROJECT_SOURCES.with(|sources| sources.borrow().get(span.source_id as usize).cloned());
    let source_text = source
        .as_ref()
        .map_or(fallback_source, |source| source.text.as_str());
    let (end_line, end_column) = source_position(source_text, span.end);
    json!({
        "file": source.as_ref().map(|source| source.path.as_str()),
        "start": { "offset": span.start, "line": span.line, "column": span.column },
        "end": { "offset": span.end, "line": end_line, "column": end_column }
    })
}

fn default_value_json(value: &zelyra_ast::DefaultValue) -> Value {
    match value {
        zelyra_ast::DefaultValue::Int(value) => json!(value),
        zelyra_ast::DefaultValue::Bool(value) => json!(value),
        zelyra_ast::DefaultValue::String(value) => json!(value),
        zelyra_ast::DefaultValue::Ident(value) => json!(value),
    }
}

fn project_name(path: &str) -> Option<String> {
    let config_path = project_config_path(path).ok().flatten()?;
    let contents = fs::read_to_string(config_path).ok()?;
    let mut in_project = false;
    for raw_line in contents.lines() {
        let line = raw_line.split('#').next()?.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_project = line == "[project]";
            continue;
        }
        if in_project {
            let (key, value) = line.split_once('=')?;
            if key.trim() == "name" {
                return value
                    .trim()
                    .strip_prefix('"')
                    .and_then(|value| value.strip_suffix('"'))
                    .map(str::to_owned);
            }
        }
    }
    None
}

fn context_entry(path: &str) -> String {
    let source_path = fs::canonicalize(path).ok();
    let root = project_config_path(path)
        .ok()
        .flatten()
        .and_then(|path| path.parent().map(PathBuf::from));
    if let (Some(source_path), Some(root)) = (source_path, root) {
        if let Ok(relative) = source_path.strip_prefix(root) {
            return relative.to_string_lossy().replace('\\', "/");
        }
    }
    path.replace('\\', "/")
}

fn context_view_slots(html: &str) -> Vec<Value> {
    slot_invocations(html)
        .unwrap_or_default()
        .into_iter()
        .map(|slot| {
            json!({
                "name": slot.name.unwrap_or_else(|| "default".into()),
                "fallback": slot.body.is_some()
            })
        })
        .collect()
}

fn context_declarations(program: &zelyra_ast::Program, source: &str) -> Value {
    let databases = program
        .databases
        .iter()
        .map(|database| {
            json!({
                "name": database.name,
                "engine": database.engine,
                "database": database.database,
                "span": context_span(source, database.span)
            })
        })
        .collect::<Vec<_>>();
    let tables = program
        .tables
        .iter()
        .map(|table| {
            let fields = table
                .columns
                .iter()
                .map(|column| {
                    json!({
                        "name": column.name,
                        "type": column.ty.to_string(),
                        "optional": !column.required,
                        "primary_key": column.primary_key,
                        "auto_increment": column.auto,
                        "unique": column.unique,
                        "default": column.default.as_ref().map(default_value_json),
                        "span": context_span(source, column.span)
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "name": table.name,
                "fields": fields,
                "span": context_span(source, table.span)
            })
        })
        .collect::<Vec<_>>();
    let cruds = program
        .cruds
        .iter()
        .map(|crud| {
            json!({
                "name": crud.name,
                "table": crud.table,
                "layout": crud.layout,
                "layout_slots": crud.layout_slots.iter().map(|slot| json!({
                    "name": slot.name,
                    "span": context_span(source, slot.span)
                })).collect::<Vec<_>>(),
                "view_fields": crud.view.fields,
                "span": context_span(source, crud.span)
            })
        })
        .collect::<Vec<_>>();
    let views = program
        .views
        .iter()
        .map(|view| {
            json!({
                "name": view.name,
                "input_type": Value::Null,
                "used_fields": Vec::<String>::new(),
                "slots": context_view_slots(&view.html),
                "span": context_span(source, view.span)
            })
        })
        .collect::<Vec<_>>();
    let components = program
        .components
        .iter()
        .map(|component| {
            json!({
                "name": component.name,
                "props": component.props.iter().map(|prop| json!({
                    "name": prop.name,
                    "type": prop.ty.to_string()
                })).collect::<Vec<_>>(),
                "slots": context_view_slots(&component.html),
                "span": context_span(source, component.span)
            })
        })
        .collect::<Vec<_>>();
    let pages = program
        .pages
        .iter()
        .map(|page| {
            let data = page
                .data
                .iter()
                .map(|binding| {
                    json!({
                        "name": binding.name,
                        "type": binding.result_type.to_string(),
                        "fields": page_data_fields(program, &binding.result_type),
                        "span": context_span(source, binding.span)
                    })
                })
                .collect::<Vec<_>>();
            let inputs = page
                .inputs
                .iter()
                .map(|input| {
                    json!({
                        "name": input.name,
                        "type": input.ty.to_string(),
                        "span": context_span(source, input.span)
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "path": page.path,
                "view": page.view,
                "inputs": inputs,
                "page_size": page.page_size,
                "sort": page.sort,
                "search": page.search,
                "filters": page.filters,
                "data": data,
                "span": context_span(source, page.span)
            })
        })
        .collect::<Vec<_>>();
    let tableviews = program
        .tableviews
        .iter()
        .map(|view| {
            json!({
                "name": view.name,
                "result_type": view.result_type.to_string(),
                "fields": view.columns,
                "span": context_span(source, view.span)
            })
        })
        .collect::<Vec<_>>();
    let forms = program
        .forms
        .iter()
        .map(|form| {
            json!({
                "name": form.name,
                "table": form.table,
                "fields": form.fields.iter().map(|field| field.name.clone()).collect::<Vec<_>>(),
                "span": context_span(source, form.span)
            })
        })
        .collect::<Vec<_>>();
    let apis = program
        .apis
        .iter()
        .map(|api| {
            json!({
                "method": api.method,
                "path": api.path,
                "input": api.input.iter().map(|field| json!({ "name": field.name, "type": field.ty.to_string() })).collect::<Vec<_>>(),
                "output": api.output.to_string(),
                "span": context_span(source, api.span)
            })
        })
        .collect::<Vec<_>>();
    let auth = program
        .auth
        .iter()
        .map(|auth| {
            json!({
                "name": auth.name,
                "table": auth.table,
                "audit_table": auth.audit_table,
                "audit_chain": auth.audit_chain,
                "span": context_span(source, auth.span)
            })
        })
        .collect::<Vec<_>>();
    json!({
        "databases": databases,
        "tables": tables,
        "cruds": cruds,
        "pages": pages,
        "views": views,
        "components": components,
        "tableviews": tableviews,
        "forms": forms,
        "apis": apis,
        "auth": auth
    })
}

fn empty_context_declarations() -> Value {
    json!({
        "databases": [],
        "tables": [],
        "cruds": [],
        "pages": [],
        "views": [],
        "components": [],
        "tableviews": [],
        "forms": [],
        "apis": [],
        "auth": []
    })
}

fn context_modules() -> Value {
    PROJECT_MODULES.with(|modules| {
        json!(modules
            .borrow()
            .iter()
            .map(|module| json!({
                "path": module.path,
                "imports": module.imports.iter().map(|import| json!({
                    "alias": import.alias,
                    "path": import.path
                })).collect::<Vec<_>>(),
                "exports": module.exports.iter().map(|export| json!({
                    "kind": export.kind,
                    "name": export.name
                })).collect::<Vec<_>>()
            }))
            .collect::<Vec<_>>())
    })
}

fn module_declaration_owners(program: &zelyra_ast::Program) -> HashMap<String, String> {
    let source_paths = PROJECT_SOURCES.with(|sources| {
        sources
            .borrow()
            .iter()
            .map(|source| source.path.clone())
            .collect::<Vec<_>>()
    });
    let mut owners = HashMap::new();
    let mut insert = |name: String, span: zelyra_ast::Span| {
        if let Some(path) = source_paths.get(span.source_id as usize) {
            owners.insert(name, path.clone());
        }
    };
    for database in &program.databases {
        insert(format!("database:{}", database.name), database.span);
    }
    for table in &program.tables {
        insert(format!("table:{}", table.name), table.span);
    }
    for tableview in &program.tableviews {
        insert(format!("tableview:{}", tableview.name), tableview.span);
    }
    for definition in &program.types {
        insert(format!("type:{}", definition.name), definition.span);
    }
    for record in &program.records {
        insert(format!("record:{}", record.name), record.span);
    }
    for page in &program.pages {
        insert(format!("page:{}", page.path), page.span);
    }
    for view in &program.views {
        insert(format!("view:{}", view.name), view.span);
    }
    for component in &program.components {
        insert(format!("component:{}", component.name), component.span);
    }
    for function in &program.functions {
        insert(format!("function:{}", function.name), function.span);
    }
    for form in &program.forms {
        insert(format!("form:{}", form.name), form.span);
    }
    for crud in &program.cruds {
        insert(format!("crud:{}", crud.name), crud.span);
    }
    for api in &program.apis {
        insert(format!("api:{} {}", api.method, api.path), api.span);
    }
    for auth in &program.auth {
        insert(format!("auth:{}", auth.name), auth.span);
    }
    owners
}

fn module_uses_database(
    program: &zelyra_ast::Program,
    module_path: &str,
    owners: &HashMap<String, String>,
) -> bool {
    let owned = |node: &str| owners.get(node).is_some_and(|path| path == module_path);
    program
        .tables
        .iter()
        .any(|table| owned(&format!("table:{}", table.name)))
        || program
            .tableviews
            .iter()
            .any(|view| owned(&format!("tableview:{}", view.name)))
        || program
            .pages
            .iter()
            .any(|page| !page.data.is_empty() && owned(&format!("page:{}", page.path)))
        || program
            .forms
            .iter()
            .any(|form| owned(&format!("form:{}", form.name)))
        || program
            .cruds
            .iter()
            .any(|crud| owned(&format!("crud:{}", crud.name)))
        || program
            .auth
            .iter()
            .any(|auth| owned(&format!("auth:{}", auth.name)))
        || program.functions.iter().any(|function| {
            owned(&format!("function:{}", function.name))
                && function
                    .capabilities
                    .iter()
                    .any(|capability| capability.starts_with("Database"))
        })
}

fn is_application_resource_id(resource: &str) -> bool {
    matches!(
        resource.split_once(':').map(|(kind, _)| kind),
        Some("page" | "api" | "crud" | "form" | "tableview")
    )
}

fn qualify_impact_declaration(
    declaration: &str,
    owners: &HashMap<String, String>,
) -> Option<String> {
    let module_path = owners.get(declaration)?;
    let (kind, name) = declaration.split_once(':')?;
    let local_name = if matches!(kind, "function" | "record" | "type") {
        name.rsplit_once("::").map_or(name, |(_, name)| name)
    } else {
        name
    };
    Some(format!("{module_path}::{kind}:{local_name}"))
}

fn module_command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    match arguments.next().as_deref() {
        Some("plan") => module_plan_command(arguments),
        Some("bundle") => module_bundle_command(arguments),
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}

fn module_plan_command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let (Some(entry), Some(selected)) = (arguments.next(), arguments.next()) else {
        usage();
        return ExitCode::from(2);
    };
    if arguments.next().is_some() {
        usage();
        return ExitCode::from(2);
    }

    let source = fs::read_to_string(&entry).unwrap_or_default();
    begin_json_diagnostics(&entry, &source);
    let validation = validate(&entry);
    let modules = PROJECT_MODULES.with(|modules| modules.borrow().clone());
    let mut plan = None;
    if let Ok(program) = validation {
        let selected = selected.replace('\\', "/");
        let by_path = modules
            .iter()
            .map(|module| (module.path.as_str(), module))
            .collect::<HashMap<_, _>>();
        let owners = module_declaration_owners(&program);
        let selected_resource = if by_path.contains_key(selected.as_str()) {
            None
        } else if is_application_resource_id(&selected) && owners.contains_key(&selected) {
            Some(selected.clone())
        } else {
            None
        };
        let selected_module = selected_resource
            .as_ref()
            .and_then(|resource| owners.get(resource).cloned())
            .or_else(|| {
                by_path
                    .contains_key(selected.as_str())
                    .then(|| selected.clone())
            });
        if let Some(selected_module) = selected_module {
            let project_sources = PROJECT_SOURCES.with(|sources| sources.borrow().clone());
            let impact = build_impact_with_sources(&program, &project_sources, &source);
            let references = impact
                .get("references")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut pending = vec![selected_module.clone()];
            let mut included = BTreeSet::new();
            let mut graph_is_complete = true;
            let mut unresolved_references = BTreeSet::new();
            let mut database_required = false;
            while let Some(path) = pending.pop() {
                if !included.insert(path.clone()) {
                    continue;
                }
                let Some(module) = by_path.get(path.as_str()) else {
                    diagnostic(
                        &entry,
                        "E-MOD-014",
                        &format!("module dependency `{path}` is missing from the loaded graph"),
                        1,
                        1,
                    );
                    graph_is_complete = false;
                    break;
                };
                pending.extend(module.imports.iter().map(|import| import.path.clone()));
                if module_uses_database(&program, &path, &owners) {
                    database_required = true;
                    for database in &program.databases {
                        if let Some(database_path) =
                            owners.get(&format!("database:{}", database.name))
                        {
                            pending.push(database_path.clone());
                        }
                    }
                }
                for reference in &references {
                    let Some(from) = reference.get("from").and_then(Value::as_str) else {
                        continue;
                    };
                    if owners.get(from).map(String::as_str) != Some(path.as_str()) {
                        continue;
                    }
                    let Some(to) = reference.get("to").and_then(Value::as_str) else {
                        continue;
                    };
                    if let Some(dependency_path) = owners.get(to) {
                        pending.push(dependency_path.clone());
                    } else {
                        unresolved_references.insert((
                            path.clone(),
                            from.to_owned(),
                            to.to_owned(),
                        ));
                    }
                }
            }
            if graph_is_complete {
                let closure = included
                    .iter()
                    .filter_map(|path| by_path.get(path.as_str()).copied())
                    .collect::<Vec<_>>();
                let declaration_roots = selected_resource
                    .as_ref()
                    .map(|resource| vec![format!("{selected_module}::{resource}")])
                    .unwrap_or_else(|| {
                        by_path
                            .get(selected_module.as_str())
                            .map(|module| {
                                module
                                    .declarations
                                    .iter()
                                    .map(|declaration| format!("{}::{declaration}", module.path))
                                    .collect()
                            })
                            .unwrap_or_default()
                    });
                let mut declaration_pending = declaration_roots.clone();
                let mut declaration_closure = BTreeSet::new();
                while let Some(current) = declaration_pending.pop() {
                    if !declaration_closure.insert(current.clone()) {
                        continue;
                    }
                    for reference in &references {
                        let Some(from) = reference.get("from").and_then(Value::as_str) else {
                            continue;
                        };
                        if qualify_impact_declaration(from, &owners).as_deref()
                            != Some(current.as_str())
                        {
                            continue;
                        }
                        let Some(to) = reference.get("to").and_then(Value::as_str) else {
                            continue;
                        };
                        if let Some(to) = qualify_impact_declaration(to, &owners) {
                            declaration_pending.push(to);
                        }
                    }
                }
                if database_required {
                    for database in &program.databases {
                        let database_id = format!("database:{}", database.name);
                        if let Some(module_path) = owners.get(&database_id) {
                            if included.contains(module_path) {
                                declaration_closure.insert(format!("{module_path}::{database_id}"));
                            }
                        }
                    }
                }
                let source_only_declarations = closure
                    .iter()
                    .flat_map(|module| {
                        module
                            .declarations
                            .iter()
                            .map(|declaration| format!("{}::{declaration}", module.path))
                    })
                    .filter(|declaration| !declaration_closure.contains(declaration))
                    .collect::<BTreeSet<_>>();
                let declaration_edges = references
                    .iter()
                    .filter(|reference| {
                        let from = reference
                            .get("from")
                            .and_then(Value::as_str)
                            .and_then(|from| qualify_impact_declaration(from, &owners));
                        let to = reference
                            .get("to")
                            .and_then(Value::as_str)
                            .and_then(|to| qualify_impact_declaration(to, &owners));
                        from.as_ref().is_some_and(|from| {
                            declaration_closure.contains(from)
                                && to
                                    .as_ref()
                                    .is_some_and(|to| declaration_closure.contains(to))
                        })
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let mut resource_dependencies = BTreeMap::new();
                for reference in &references {
                    let Some(from) = reference.get("from").and_then(Value::as_str) else {
                        continue;
                    };
                    let Some(from_path) = owners.get(from) else {
                        continue;
                    };
                    let Some(to) = reference.get("to").and_then(Value::as_str) else {
                        continue;
                    };
                    let Some(to_path) = owners.get(to) else {
                        continue;
                    };
                    if from_path == to_path
                        || !included.contains(from_path)
                        || !included.contains(to_path)
                    {
                        continue;
                    }
                    let kind = reference
                        .get("kind")
                        .and_then(Value::as_str)
                        .unwrap_or("reference");
                    let access = reference
                        .get("access")
                        .and_then(Value::as_str)
                        .unwrap_or("none");
                    let key = (
                        from_path.clone(),
                        from.to_owned(),
                        to_path.clone(),
                        to.to_owned(),
                        kind.to_owned(),
                        access.to_owned(),
                    );
                    resource_dependencies.insert(
                        key,
                        json!({
                            "from_module": from_path,
                            "from": from,
                            "to_module": to_path,
                            "to": to,
                            "kind": kind,
                            "access": reference.get("access").and_then(Value::as_str),
                            "span": reference.get("span")
                        }),
                    );
                }
                if database_required {
                    for module_path in &included {
                        if !module_uses_database(&program, module_path, &owners) {
                            continue;
                        }
                        for database in &program.databases {
                            let database_node = format!("database:{}", database.name);
                            let Some(database_path) = owners.get(&database_node) else {
                                continue;
                            };
                            if module_path == database_path {
                                continue;
                            }
                            let key = (
                                module_path.clone(),
                                "database:runtime".into(),
                                database_path.clone(),
                                database_node.clone(),
                                "database_configuration".into(),
                                "none".into(),
                            );
                            resource_dependencies.insert(
                                key,
                                json!({
                                    "from_module": module_path,
                                    "from": "database:runtime",
                                    "to_module": database_path,
                                    "to": database_node,
                                    "kind": "database_configuration"
                                }),
                            );
                        }
                    }
                }
                let database_configuration_sources = program
                    .databases
                    .iter()
                    .filter_map(|database| owners.get(&format!("database:{}", database.name)))
                    .filter(|path| included.contains(*path))
                    .cloned()
                    .collect::<BTreeSet<_>>();
                let database_configurations = program
                    .databases
                    .iter()
                    .filter_map(|database| {
                        let declaration = format!("database:{}", database.name);
                        let module_path = owners.get(&declaration)?;
                        included.contains(module_path).then(|| {
                            (
                                format!("{module_path}::{declaration}"),
                                json!({
                                    "declaration": declaration,
                                    "module": module_path,
                                    "engine": database.engine,
                                    "database": database.database,
                                    "connection_environment": database_url_environment_name(&database.name)
                                }),
                            )
                        })
                    })
                    .collect::<BTreeMap<_, _>>();
                let schema_ownership = program
                    .tables
                    .iter()
                    .filter_map(|table| {
                        let declaration = format!("table:{}", table.name);
                        let module_path = owners.get(&declaration)?;
                        included.contains(module_path).then(|| {
                            (
                                table.name.clone(),
                                json!({
                                    "table": table.name,
                                    "inferred_owner_module": module_path,
                                    "ownership_enforced": false
                                }),
                            )
                        })
                    })
                    .collect::<BTreeMap<_, _>>();
                plan = Some(json!({
                    "kind": "known-semantic-dependency-closure",
                    "closure_semantics": "explicit-imports-plus-statically-recognized-references",
                    "selected_module": selected_module,
                    "selected_resource": selected_resource,
                    "selection_kind": if selected_resource.is_some() { "resource" } else { "module" },
                    "entry": context_entry(&entry),
                    "source_files": included,
                    "declaration_closure": {
                        "semantics": "selected-root-plus-statically-recognized-impact-references-and-database-configuration",
                        "complete": false,
                        "roots": declaration_roots,
                        "declarations": declaration_closure,
                        "edges": declaration_edges,
                        "configuration_edges": resource_dependencies
                            .values()
                            .filter(|dependency| dependency["kind"] == "database_configuration")
                            .cloned()
                            .collect::<Vec<_>>(),
                        "additional_declarations_in_included_source_files": source_only_declarations
                    },
                    "modules": closure.iter().map(|module| json!({
                        "path": module.path,
                        "imports": module.imports.iter().map(|import| json!({
                            "alias": import.alias,
                            "path": import.path
                        })).collect::<Vec<_>>(),
                        "exports": module.exports.iter().map(|export| json!({
                            "kind": export.kind,
                            "name": export.name
                        })).collect::<Vec<_>>(),
                        "declarations": module.declarations
                    })).collect::<Vec<_>>(),
                    "resource_dependencies": resource_dependencies.values().cloned().collect::<Vec<_>>(),
                    "schema_ownership": {
                        "model": "inferred_from_table_declaration_source_module",
                        "enforced": false,
                        "tables": schema_ownership.values().collect::<Vec<_>>()
                    },
                    "table_access_contract": {
                        "model": "table-owner-module-in-consumer-import-closure",
                        "dependency_enforced": true,
                        "read_write_permissions_enforced": false,
                        "entry_module_tables_project_visible": false,
                        "analysis_complete": false
                    },
                    "unresolved_references": unresolved_references.iter().map(|(module, from, to)| json!({
                        "from_module": module,
                        "from": from,
                        "to": to
                    })).collect::<Vec<_>>(),
                    "database": {
                        "required": database_required,
                        "configuration_sources": database_configuration_sources,
                        "configurations": database_configurations.values().collect::<Vec<_>>(),
                        "connection_model": "single-project-wide-connection",
                        "connection_environment": program.databases.first()
                            .map(|database| database_url_environment_name(&database.name))
                            .unwrap_or_else(|| "DATABASE_URL".to_owned()),
                        "supports_multiple_connections": false
                    },
                    "complete_deployment": false,
                    "limitations": [
                        "Only dependency kinds recognized by the current static impact graph are followed.",
                        "Project configuration, runtime adapters, assets, external service contracts, and Docker artifacts are not included."
                    ],
                    "note": "This read-only preview is not a complete deployment manifest, runnable application, or Docker export."
                }));
            }
        } else {
            diagnostic(
                &entry,
                "E-MOD-013",
                &format!(
                    "`{selected}` is neither a reachable module nor a supported application resource"
                ),
                1,
                1,
            );
        }
    }
    let diagnostics = finish_json_diagnostics();
    let success = plan.is_some() && diagnostics.is_empty();
    print_machine_document(&machine_document(
        "module plan",
        success,
        diagnostics,
        [("plan".into(), plan.unwrap_or(Value::Null))],
    ));
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

type ModuleBundleFiles = (Vec<String>, Vec<String>, Vec<String>);

fn module_bundle_command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let (Some(entry), Some(selected)) = (arguments.next(), arguments.next()) else {
        usage();
        return ExitCode::from(2);
    };
    let mut output = None;
    let mut docker = false;
    let mut dry_run = false;
    let mut compiler_ref = None;
    while let Some(argument) = arguments.next() {
        if argument == "--output" && output.is_none() {
            output = arguments.next().map(PathBuf::from);
        } else if argument == "--dry-run" && !dry_run {
            dry_run = true;
        } else if argument == "--docker" && !docker {
            docker = true;
        } else if argument == "--compiler-ref" && compiler_ref.is_none() {
            compiler_ref = arguments.next();
        } else {
            eprintln!("error[E-CLI-001]: expected `--output <directory>` and optional `--docker --compiler-ref <40-character-commit>`");
            return ExitCode::from(2);
        }
    }
    let Some(output) = output else {
        eprintln!("error[E-CLI-001]: `module bundle` requires `--output <directory>`");
        return ExitCode::from(2);
    };
    if docker && !compiler_ref.as_deref().is_some_and(is_full_git_commit) {
        eprintln!("error[E-CLI-001]: `--docker` requires `--compiler-ref` with a full 40-character hexadecimal Git commit");
        return ExitCode::from(2);
    }
    if !docker && compiler_ref.is_some() {
        eprintln!("error[E-CLI-001]: `--compiler-ref` can only be used together with `--docker`");
        return ExitCode::from(2);
    }

    let executable = match env::current_exe() {
        Ok(executable) => executable,
        Err(error) => {
            eprintln!("error[E-MOD-018]: cannot locate the Zelyra executable: {error}");
            return ExitCode::from(1);
        }
    };
    let plan_output = match Command::new(&executable)
        .args(["module", "plan", &entry, &selected])
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            eprintln!("error[E-MOD-018]: cannot run the module dependency planner: {error}");
            return ExitCode::from(1);
        }
    };
    if !plan_output.status.success() {
        eprint!("{}", String::from_utf8_lossy(&plan_output.stdout));
        return ExitCode::from(1);
    }
    let plan_document: Value = match serde_json::from_slice(&plan_output.stdout) {
        Ok(document) => document,
        Err(error) => {
            eprintln!("error[E-MOD-018]: dependency planner returned invalid JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let Some(plan) = plan_document.get("plan") else {
        eprintln!("error[E-MOD-018]: dependency planner returned no plan");
        return ExitCode::from(1);
    };
    let Some(source_files) = plan.get("source_files").and_then(Value::as_array) else {
        eprintln!("error[E-MOD-018]: dependency plan has no source-file inventory");
        return ExitCode::from(1);
    };
    let source_files = source_files
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if source_files.is_empty() {
        eprintln!("error[E-MOD-018]: dependency plan contains no source files");
        return ExitCode::from(1);
    }
    if plan
        .get("unresolved_references")
        .and_then(Value::as_array)
        .is_none_or(|references| !references.is_empty())
    {
        eprintln!("error[E-MOD-018]: refusing to bundle a plan with unresolved references");
        return ExitCode::from(1);
    }

    let entry_path = match fs::canonicalize(&entry) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("error[E-MOD-018]: cannot resolve entry `{entry}`: {error}");
            return ExitCode::from(1);
        }
    };
    let Some(project_root) = entry_path.parent() else {
        eprintln!("error[E-MOD-018]: entry has no project directory");
        return ExitCode::from(1);
    };
    let entry_relative = match entry_path.strip_prefix(project_root) {
        Ok(path) => path.to_string_lossy().replace('\\', "/"),
        Err(error) => {
            eprintln!("error[E-MOD-018]: cannot make entry project-relative: {error}");
            return ExitCode::from(1);
        }
    };
    let selected_module = plan
        .get("selected_module")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let selected_resource = plan
        .get("selected_resource")
        .and_then(Value::as_str)
        .is_some();
    let full_project = selected_module == entry_relative && !selected_resource;
    if selected_module == entry_relative && selected_resource {
        eprintln!("error[E-MOD-018]: move the selected resource into its own source module before bundling; the project entry file cannot be isolated safely");
        return ExitCode::from(1);
    }
    if !full_project && source_files.contains(&entry_relative) {
        eprintln!("error[E-MOD-018]: selected module depends on the original project entry; split the entry from reusable application modules before bundling");
        return ExitCode::from(1);
    }

    let output_name = output.file_name().filter(|name| !name.is_empty());
    let Some(output_name) = output_name else {
        eprintln!("error[E-MOD-018]: output must name a new directory");
        return ExitCode::from(1);
    };
    let output_parent = output.parent().unwrap_or_else(|| std::path::Path::new("."));
    let output_parent = match fs::canonicalize(output_parent) {
        Ok(path) if path.is_dir() => path,
        Ok(_) => {
            eprintln!("error[E-MOD-018]: output parent is not a directory");
            return ExitCode::from(1);
        }
        Err(error) => {
            eprintln!("error[E-MOD-018]: output parent must already exist: {error}");
            return ExitCode::from(1);
        }
    };
    let output = output_parent.join(output_name);
    match fs::symlink_metadata(&output) {
        Ok(_) => {
            eprintln!(
                "error[E-MOD-018]: output `{}` already exists; no files were changed",
                output.display()
            );
            return ExitCode::from(1);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            eprintln!("error[E-MOD-018]: cannot inspect output directory: {error}");
            return ExitCode::from(1);
        }
    }
    if output.starts_with(project_root) {
        eprintln!("error[E-MOD-018]: choose an output directory outside the source project");
        return ExitCode::from(1);
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let mut staging = None;
    for attempt in 0..8 {
        let candidate = output_parent.join(format!(
            ".{}.zelyra-bundle-{}-{timestamp}-{attempt}",
            output_name.to_string_lossy(),
            std::process::id()
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => {
                staging = Some(candidate);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                eprintln!("error[E-MOD-018]: cannot create staging directory: {error}");
                return ExitCode::from(1);
            }
        }
    }
    let Some(staging) = staging else {
        eprintln!("error[E-MOD-018]: cannot allocate a unique staging directory");
        return ExitCode::from(1);
    };

    let build_result = (|| -> Result<ModuleBundleFiles, String> {
        let mut copied = Vec::new();
        let mut docker_files = Vec::new();
        if full_project {
            copy_bundle_source(project_root, &staging, &entry_relative, "main.zyl")?;
            copied.push("main.zyl".to_owned());
        }
        for relative in &source_files {
            if full_project && relative == &entry_relative {
                continue;
            }
            copy_bundle_source(project_root, &staging, relative, relative)?;
            copied.push(relative.clone());
        }
        if !full_project {
            let imports = source_files
                .iter()
                .enumerate()
                .map(|(index, path)| {
                    let escaped = path.replace('\\', "\\\\").replace('"', "\\\"");
                    format!("import \"{escaped}\" as bundle_{index}")
                })
                .collect::<Vec<_>>();
            fs::write(
                staging.join("main.zyl"),
                format!("{}\n", imports.join("\n")),
            )
            .map_err(|error| format!("cannot write generated entry: {error}"))?;
            copied.push("main.zyl".to_owned());
        }

        let mut support_files = Vec::new();
        for relative in ["zelyra.toml", PROJECT_THEME_CSS_FILE] {
            if project_root.join(relative).exists() {
                copy_bundle_source(project_root, &staging, relative, relative)?;
                support_files.push(relative.to_owned());
            }
        }
        let locales = project_root.join("locales");
        match fs::symlink_metadata(&locales) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err("refusing to copy a symlinked locales directory".into());
            }
            Ok(metadata) if metadata.is_dir() => {
                for item in fs::read_dir(&locales)
                    .map_err(|error| format!("cannot read locales directory: {error}"))?
                {
                    let item =
                        item.map_err(|error| format!("cannot read locale entry: {error}"))?;
                    let name = item.file_name().to_string_lossy().into_owned();
                    if !name.ends_with(".json") {
                        continue;
                    }
                    copy_bundle_source(
                        project_root,
                        &staging,
                        &format!("locales/{name}"),
                        &format!("locales/{name}"),
                    )?;
                    support_files.push(format!("locales/{name}"));
                }
            }
            Ok(_) => return Err("locales path is not a directory".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("cannot inspect locales directory: {error}")),
        }

        let database = plan.get("database").cloned().unwrap_or(Value::Null);
        let database_is_required = database["required"].as_bool().unwrap_or(false);
        let configured_backends = database["configurations"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let needs_mariadb_client = database_is_required
            && (configured_backends.is_empty()
                || configured_backends.iter().any(|configuration| {
                    configuration["engine"]
                        .as_str()
                        .is_some_and(|engine| engine.eq_ignore_ascii_case("mariadb"))
                }));
        let runtime_packages = if needs_mariadb_client {
            "ca-certificates mariadb-client"
        } else {
            "ca-certificates"
        };
        let runtime_package_manifest = if needs_mariadb_client {
            vec!["ca-certificates", "mariadb-client"]
        } else {
            vec!["ca-certificates"]
        };
        let connection_environment = configured_backends
            .first()
            .and_then(|configuration| configuration["connection_environment"].as_str())
            .unwrap_or("DATABASE_URL");
        if docker {
            let compiler_ref = compiler_ref.as_deref().expect("validated compiler ref");
            let dockerfile = format!(
                r#"FROM rust:1-bookworm AS build
ARG ZELYRA_REF={compiler_ref}
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/*
RUN git init /zelyra \
    && git -C /zelyra remote add origin https://github.com/sf1976/zelyra.git \
    && git -C /zelyra fetch --depth=1 origin "$ZELYRA_REF" \
    && git -C /zelyra checkout --detach FETCH_HEAD
RUN cargo install --locked --path /zelyra/cli --root /out

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends {runtime_packages} \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /out/bin/zelyra /usr/local/bin/zelyra
WORKDIR /app
COPY . ./
EXPOSE 8080
CMD ["zelyra", "serve", "main.zyl", "0.0.0.0:8080"]
"#
            );
            let compose = r#"services:
  app:
    build:
      context: .
    restart: unless-stopped
    env_file:
      - .env
    extra_hosts:
      - "host.docker.internal:host-gateway"
    ports:
      - "127.0.0.1:${ZELYRA_HOST_PORT:-18080}:8080"
"#;
            let env_example = format!(
                r#"# Copy this file to .env and configure values for this deployment.
# Never commit .env or put production credentials in this example.
ZELYRA_HOST_PORT=18080
# Required only when the selected application accesses a database.
# Set {connection_environment} to your MariaDB connection; never commit its real password.
# {connection_environment}=mariadb://USER:PASSWORD@host.docker.internal:3306/DATABASE
{connection_environment}=
ZELYRA_DB_CONNECT_TIMEOUT_SECS=10
ZELYRA_DB_QUERY_TIMEOUT_SECS=30
ZELYRA_DB_POOL_MAX_SIZE=8
ZELYRA_DB_POOL_WAIT_TIMEOUT_SECS=10
# Remote database connections use verified TLS automatically.
ZELYRA_DB_TLS_MODE=auto
# ZELYRA_DB_TLS_CA_CERT_FILE=/absolute/path/to/your/database-ca.pem
"#
            );
            let dockerignore = ".git\n.env\n.env.*\ntarget/\nbuild/\ndist/\n*.log\n*.sqlite*\n*.db\n*.pem\n*.key\n*.p12\n*.pfx\n";
            for (name, contents) in [
                ("Dockerfile", dockerfile),
                ("docker-compose.yml", compose.to_owned()),
                (".env.example", env_example.to_owned()),
                (".dockerignore", dockerignore.to_owned()),
            ] {
                fs::write(staging.join(name), contents)
                    .map_err(|error| format!("cannot write {name}: {error}"))?;
                docker_files.push(name.to_owned());
            }
        }

        let manifest = json!({
            "format_version": 1,
            "kind": if docker { "experimental-docker-source-package" } else { "experimental-source-bundle" },
            "selected": selected,
            "selected_module": selected_module,
            "selected_resource": plan.get("selected_resource"),
            "source_files": copied,
            "support_files": support_files,
            "database": database,
            "source_closure_complete": false,
            "complete_deployment": false,
            "docker": if docker {
                json!({
                    "compiler_repository": "https://github.com/sf1976/zelyra",
                    "compiler_commit": compiler_ref,
                    "files": docker_files,
                    "runtime_packages": runtime_package_manifest,
                    "database_connection": format!("external, configured per exported Compose project via {}", database["connection_environment"].as_str().unwrap_or("DATABASE_URL")),
                    "database_connection_scope": "per_exported_compose_project",
                    "supports_multiple_connections_per_process": false
                })
            } else {
                Value::Null
            },
            "limitations": [
                "The static dependency graph is incomplete; this bundle is not a deployment manifest.",
                if docker { "The Docker package builds the compiler from the pinned source commit; runtime deployment completeness is not yet proven." } else { "No Dockerfile, Compose stack, runtime binary, database service, or .env file is included." },
                "Database credentials and other runtime secrets are intentionally not copied.",
                "The selected source file is included in full; resource selection does not remove co-located declarations."
            ]
        });
        let manifest_text = serde_json::to_string_pretty(&manifest)
            .map_err(|error| format!("cannot serialize bundle manifest: {error}"))?;
        fs::write(
            staging.join("zelyra.bundle.json"),
            format!("{manifest_text}\n"),
        )
        .map_err(|error| format!("cannot write bundle manifest: {error}"))?;
        let readme = if docker {
            format!("# Experimental Zelyra Docker package\n\nThis package contains the selected known source closure and a Docker Compose app service. It is experimental, not a verified complete deployment; inspect `zelyra.bundle.json` (`complete_deployment: false`). The Dockerfile builds Zelyra from the exact compiler commit recorded in that manifest.\n\nCopy `.env.example` to `.env`, set a private `{connection_environment}` if needed, then run `docker compose up --build`. The MariaDB service is external and is not created by this package. The default `host.docker.internal` address is for a database on the Docker host; adjust it for your network. Remote database connections use verified TLS by default. For a private CA, mount its file into the app container and set `ZELYRA_DB_TLS_CA_CERT_FILE` to that in-container path. Disable TLS only for an isolated local network. Never commit `.env`.\n")
        } else {
            "# Experimental Zelyra source bundle\n\nThis directory contains the selected project module and source files in the dependency preview. It is not a Docker export or a complete deployment. Review `zelyra.bundle.json`; its `complete_deployment` value is `false`.\n\nRun `zelyra check main.zyl` with a compiler build that supports project imports. Configure any required external database and runtime settings separately. No `.env` file or credentials were copied.\n".to_owned()
        };
        fs::write(staging.join("README.md"), readme)
            .map_err(|error| format!("cannot write bundle README: {error}"))?;

        let check = Command::new(&executable)
            .current_dir(&staging)
            .args(["check", "main.zyl", "--format=json"])
            .output()
            .map_err(|error| format!("cannot validate generated bundle: {error}"))?;
        if !check.status.success() {
            return Err(format!(
                "generated bundle failed `zelyra check`; staged files were discarded:\n{}",
                String::from_utf8_lossy(&check.stdout)
            ));
        }
        Ok((copied, support_files, docker_files))
    })();

    match build_result {
        Ok((source_files, support_files, docker_files)) => {
            if dry_run {
                let mut files = source_files
                    .iter()
                    .chain(support_files.iter())
                    .chain(docker_files.iter())
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>();
                files.insert("README.md");
                files.insert("zelyra.bundle.json");
                let planned_files = files
                    .into_iter()
                    .map(|relative_path| {
                        json!({
                            "relative_path": relative_path,
                            "destination": output.join(relative_path).to_string_lossy()
                        })
                    })
                    .collect::<Vec<_>>();
                let document = json!({
                    "schema_version": "1",
                    "command": "module bundle",
                    "success": true,
                    "diagnostics": [],
                    "plan": {
                        "kind": "experimental-bundle-write-plan",
                        "selected": selected,
                        "selected_module": selected_module,
                        "output_directory": output,
                        "docker": docker,
                        "compiler_commit": compiler_ref,
                        "files": planned_files,
                        "secrets_included": false,
                        "source_closure_complete": false,
                        "complete_deployment": false,
                        "writes_performed": false
                    }
                });
                if let Err(error) = fs::remove_dir_all(&staging) {
                    eprintln!("error[E-MOD-018]: cannot remove temporary plan files: {error}");
                    return ExitCode::from(1);
                }
                println!("{}", serde_json::to_string_pretty(&document).unwrap());
                return ExitCode::SUCCESS;
            }
            if fs::symlink_metadata(&output).is_ok() {
                let _ = fs::remove_dir_all(&staging);
                eprintln!(
                    "error[E-MOD-018]: output `{}` appeared during bundle creation; it was not changed",
                    output.display()
                );
                return ExitCode::from(1);
            }
            if let Err(error) = fs::rename(&staging, &output) {
                let _ = fs::remove_dir_all(&staging);
                eprintln!("error[E-MOD-018]: cannot publish source bundle: {error}");
                return ExitCode::from(1);
            }
            println!(
                "created experimental {} bundle: {}",
                if docker { "Docker source" } else { "source" },
                output.display()
            );
            println!(
                "{} Zelyra source files; {} support files; {} Docker files",
                source_files.len(),
                support_files.len(),
                docker_files.len()
            );
            println!("the generated bundle passed `zelyra check`");
            println!("the static dependency graph is incomplete; complete_deployment remains false in zelyra.bundle.json");
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            eprintln!("error[E-MOD-018]: {error}");
            ExitCode::from(1)
        }
    }
}

fn copy_bundle_source(
    source_root: &std::path::Path,
    bundle_root: &std::path::Path,
    source_relative: &str,
    bundle_relative: &str,
) -> Result<(), String> {
    let source = source_root.join(source_relative);
    let metadata = fs::symlink_metadata(&source)
        .map_err(|error| format!("cannot inspect `{source_relative}`: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "refusing to copy non-regular source `{source_relative}`"
        ));
    }
    let destination = bundle_root.join(bundle_relative);
    let parent = destination
        .parent()
        .ok_or_else(|| format!("bundle path `{bundle_relative}` has no parent"))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create bundle directory: {error}"))?;
    fs::copy(&source, &destination)
        .map_err(|error| format!("cannot copy `{source_relative}`: {error}"))?;
    Ok(())
}

fn is_full_git_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn context_command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let Some(path) = arguments.next() else {
        usage();
        return ExitCode::from(2);
    };
    let mut format = OutputFormat::Human;
    while let Some(argument) = arguments.next() {
        if argument == "--format=json" {
            format = OutputFormat::Json;
        } else if argument == "--format=human" {
            format = OutputFormat::Human;
        } else if argument.starts_with("--format=") {
            eprintln!("error[E-CLI-001]: format must be `human` or `json`");
            return ExitCode::from(2);
        } else if argument == "--format" {
            format = match arguments.next().as_deref().and_then(parse_output_format) {
                Some(format) => format,
                None => {
                    eprintln!("error[E-CLI-001]: format must be `human` or `json`");
                    return ExitCode::from(2);
                }
            };
        } else {
            eprintln!("error[E-CLI-001]: unknown context option `{argument}`");
            return ExitCode::from(2);
        }
    }
    if format == OutputFormat::Human {
        return if validate(&path).is_ok() {
            println!("context: {}", context_entry(&path));
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }
    let source = fs::read_to_string(&path).unwrap_or_default();
    begin_json_diagnostics(&path, &source);
    let program = validate(&path);
    let diagnostics = finish_json_diagnostics();
    let success = program.is_ok();
    let declarations = program.as_ref().map_or_else(
        |_| empty_context_declarations(),
        |program| context_declarations(program, &source),
    );
    let fields = [
        (
            "project".into(),
            json!({
                "name": project_name(&path),
                "entry": context_entry(&path)
            }),
        ),
        ("declarations".into(), declarations),
        ("modules".into(), context_modules()),
    ];
    print_machine_document(&machine_document("context", success, diagnostics, fields));
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn feature_settings_json(features: &ProjectFeatures) -> Value {
    Value::Object(
        features
            .iter()
            .map(|(name, setting)| {
                (
                    name.clone(),
                    json!({
                        "enabled": setting.enabled,
                        "source": setting.source
                    }),
                )
            })
            .collect(),
    )
}

fn config_command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let Some(path) = arguments.next() else {
        usage();
        return ExitCode::from(2);
    };
    let mut format = OutputFormat::Human;
    while let Some(argument) = arguments.next() {
        if argument == "--format=json" {
            format = OutputFormat::Json;
        } else if argument == "--format=human" {
            format = OutputFormat::Human;
        } else if argument.starts_with("--format=") {
            eprintln!("error[E-CLI-001]: format must be `human` or `json`");
            return ExitCode::from(2);
        } else if argument == "--format" {
            format = match arguments.next().as_deref().and_then(parse_output_format) {
                Some(format) => format,
                None => {
                    eprintln!("error[E-CLI-001]: format must be `human` or `json`");
                    return ExitCode::from(2);
                }
            };
        } else {
            eprintln!("error[E-CLI-001]: unknown config option `{argument}`");
            return ExitCode::from(2);
        }
    }

    let config_path = project_config_path(&path).ok().flatten();
    let env_file = config_path
        .as_ref()
        .and_then(|path| path.parent())
        .map(|path| path.join(".env"))
        .filter(|path| path.is_file())
        .is_some();
    let result = project_features(&path);
    if format == OutputFormat::Human {
        let features = match result {
            Ok(features) => features,
            Err(error) => {
                eprintln!("error[E-FEATURE-002]: {error}");
                return ExitCode::from(1);
            }
        };
        println!(
            "configuration: {}",
            if config_path.is_some() {
                "zelyra.toml"
            } else {
                "defaults"
            }
        );
        println!(
            "environment file: {}",
            if env_file {
                "loaded (feature flags only)"
            } else {
                "not present"
            }
        );
        for (name, setting) in features {
            println!(
                "  {name}: {} ({})",
                if setting.enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                setting.source
            );
        }
        return ExitCode::SUCCESS;
    }

    begin_json_diagnostics(&path, "");
    let (success, features) = match result {
        Ok(features) => (true, feature_settings_json(&features)),
        Err(error) => {
            diagnostic(&path, "E-FEATURE-002", &error, 1, 1);
            (false, Value::Object(Map::new()))
        }
    };
    let diagnostics = finish_json_diagnostics();
    print_machine_document(&machine_document(
        "config",
        success,
        diagnostics,
        [
            (
                "project".into(),
                json!({
                    "name": project_name(&path),
                    "config_file": config_path.as_ref().map(|_| "zelyra.toml"),
                    "env_file_present": env_file,
                    "secrets": "not displayed"
                }),
            ),
            ("features".into(), features),
        ],
    ));
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

struct ComponentInvocation {
    name: String,
    attributes: String,
    body: Option<String>,
    start: usize,
    end: usize,
}

fn component_invocations(html: &str) -> Result<Vec<ComponentInvocation>, String> {
    let mut invocations = Vec::new();
    let mut search_from = 0;
    while let Some(relative_start) = html[search_from..].find('<') {
        let start = search_from + relative_start;
        let after_open = &html[start + 1..];
        let Some(first) = after_open.chars().next() else {
            break;
        };
        if !first.is_ascii_uppercase() {
            search_from = start + 1;
            continue;
        }
        let name_length = after_open
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .map(char::len_utf8)
            .sum::<usize>();
        let name = after_open[..name_length].to_owned();
        let after_name = start + 1 + name_length;
        let Some(relative_tag_end) = html[after_name..].find('>') else {
            return Err(format!(
                "component `<{name}>` has an unterminated opening tag"
            ));
        };
        let tag_end = after_name + relative_tag_end;
        let tag_content = &html[after_name..tag_end];
        if tag_content.trim_end().ends_with('/') {
            let attributes = tag_content.trim_end().trim_end_matches('/').trim_end();
            let end = tag_end + 1;
            invocations.push(ComponentInvocation {
                name,
                attributes: attributes.to_owned(),
                body: None,
                start,
                end,
            });
            search_from = end;
        } else {
            let closing = format!("</{name}>");
            let body_start = tag_end + 1;
            let Some(relative_closing_start) = html[body_start..].find(&closing) else {
                return Err(format!("component `<{name}>` is missing `{closing}`"));
            };
            let closing_start = body_start + relative_closing_start;
            let end = closing_start + closing.len();
            invocations.push(ComponentInvocation {
                name,
                attributes: tag_content.to_owned(),
                body: Some(html[body_start..closing_start].to_owned()),
                start,
                end,
            });
            search_from = end;
        }
    }
    Ok(invocations)
}

fn component_attributes(attributes: &str) -> Result<HashMap<String, String>, String> {
    let mut values = HashMap::new();
    let bytes = attributes.as_bytes();
    let mut position = 0;
    while position < bytes.len() {
        while bytes.get(position).is_some_and(u8::is_ascii_whitespace) {
            position += 1;
        }
        if position == bytes.len() {
            break;
        }
        let name_start = position;
        while bytes
            .get(position)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            position += 1;
        }
        if name_start == position {
            return Err("component properties require a name".into());
        }
        let name = attributes[name_start..position].to_owned();
        while bytes.get(position).is_some_and(u8::is_ascii_whitespace) {
            position += 1;
        }
        if bytes.get(position) != Some(&b'=') {
            return Err(format!("component property `{name}` requires `=`"));
        }
        position += 1;
        while bytes.get(position).is_some_and(u8::is_ascii_whitespace) {
            position += 1;
        }
        if bytes.get(position) != Some(&b'"') {
            return Err(format!(
                "component property `{name}` must use a quoted value"
            ));
        }
        position += 1;
        let value_start = position;
        while bytes.get(position).is_some_and(|byte| *byte != b'"') {
            position += 1;
        }
        if position == bytes.len() {
            return Err(format!(
                "component property `{name}` has an unterminated value"
            ));
        }
        let value = attributes[value_start..position].to_owned();
        position += 1;
        if values.insert(name.clone(), value).is_some() {
            return Err(format!(
                "component property `{name}` is specified more than once"
            ));
        }
    }
    Ok(values)
}

struct SlotInvocation {
    name: Option<String>,
    body: Option<String>,
    start: usize,
    end: usize,
}

fn slot_invocations(html: &str) -> Result<Vec<SlotInvocation>, String> {
    let mut slots = Vec::new();
    let mut search_from = 0;
    while let Some(relative_start) = html[search_from..].find("<slot") {
        let start = search_from + relative_start;
        let after_name = start + "<slot".len();
        if html
            .as_bytes()
            .get(after_name)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            search_from = after_name;
            continue;
        }
        let Some(relative_tag_end) = html[after_name..].find('>') else {
            return Err("slot has an unterminated opening tag".into());
        };
        let tag_end = after_name + relative_tag_end;
        let tag_content = &html[after_name..tag_end];
        let self_closing = tag_content.trim_end().ends_with('/');
        let attributes = if self_closing {
            tag_content.trim_end().trim_end_matches('/').trim_end()
        } else {
            tag_content
        };
        let attributes = component_attributes(attributes)
            .map_err(|error| format!("invalid slot declaration: {error}"))?;
        if attributes.keys().any(|name| name != "name") {
            return Err("slot supports only the `name` attribute".into());
        }
        let name = attributes.get("name").cloned();
        if self_closing {
            let end = tag_end + 1;
            slots.push(SlotInvocation {
                name,
                body: None,
                start,
                end,
            });
            search_from = end;
        } else {
            let Some(name) = name else {
                return Err("content slot blocks require a `name` attribute".into());
            };
            let body_start = tag_end + 1;
            let closing = "</slot>";
            let Some(relative_closing_start) = html[body_start..].find(closing) else {
                return Err("slot is missing `</slot>`".into());
            };
            let closing_start = body_start + relative_closing_start;
            let end = closing_start + closing.len();
            slots.push(SlotInvocation {
                name: Some(name),
                body: Some(html[body_start..closing_start].to_owned()),
                start,
                end,
            });
            search_from = end;
        }
    }
    Ok(slots)
}

fn declared_component_slots(
    component: &zelyra_ast::ComponentDef,
) -> Result<(bool, HashSet<String>), String> {
    let mut has_default = false;
    let mut named = HashSet::new();
    for slot in slot_invocations(&component.html)? {
        if let Some(name) = slot.name {
            if !named.insert(name) {
                return Err("component declares the same named slot more than once".into());
            }
        } else if has_default {
            return Err("component declares the default slot more than once".into());
        } else {
            has_default = true;
        }
    }
    Ok((has_default, named))
}

fn split_component_body(body: &str) -> Result<(String, HashMap<String, String>), String> {
    let slots = slot_invocations(body)?;
    let mut named = HashMap::new();
    let mut default_body = body.to_owned();
    for slot in slots.into_iter().rev() {
        let Some(slot_body) = slot.body else {
            return Err("component content slots must use opening and closing tags".into());
        };
        let Some(name) = slot.name else {
            return Err("component content slots require a `name` attribute".into());
        };
        if named.insert(name, slot_body).is_some() {
            return Err("the same named slot is provided more than once".into());
        }
        default_body.replace_range(slot.start..slot.end, "");
    }
    Ok((default_body, named))
}

fn declared_view_slots(view: &zelyra_ast::ViewDef) -> Result<(bool, HashSet<String>), String> {
    let mut has_default = false;
    let mut named = HashSet::new();
    for slot in slot_invocations(&view.html)? {
        if let Some(name) = slot.name {
            if !named.insert(name) {
                return Err("view declares the same named slot more than once".into());
            }
        } else if has_default {
            return Err("view declares the default slot more than once".into());
        } else {
            has_default = true;
        }
    }
    Ok((has_default, named))
}

fn split_view_content(body: &str) -> Result<(String, HashMap<String, String>), String> {
    let slots = slot_invocations(body)?;
    let mut named = HashMap::new();
    let mut default_body = body.to_owned();
    for slot in slots.into_iter().rev() {
        let Some(slot_body) = slot.body else {
            return Err("view content slots must use opening and closing tags".into());
        };
        let Some(name) = slot.name else {
            return Err("view content slots require a `name` attribute".into());
        };
        if named.insert(name, slot_body).is_some() {
            return Err("the same named view slot is provided more than once".into());
        }
        default_body.replace_range(slot.start..slot.end, "");
    }
    Ok((default_body, named))
}

fn page_layout_slot_invocations(html: &str) -> Result<Vec<SlotInvocation>, String> {
    let component_ranges = component_invocations(html)?
        .into_iter()
        .map(|component| component.start..component.end)
        .collect::<Vec<_>>();
    Ok(slot_invocations(html)?
        .into_iter()
        .filter(|slot| {
            !component_ranges
                .iter()
                .any(|range| range.start <= slot.start && slot.end <= range.end)
        })
        .collect())
}

fn validate_view_content_slots(view: &zelyra_ast::ViewDef, body: &str) -> Result<(), String> {
    let (_, declared_named) = declared_view_slots(view)?;
    let (_, supplied_named) = split_view_content(body)?;
    for name in supplied_named.keys() {
        if !declared_named.contains(name) {
            return Err(format!("view has no named slot `{name}`"));
        }
    }
    Ok(())
}

fn validate_crud_layout_slots(
    view: &zelyra_ast::ViewDef,
    supplied_slots: &[zelyra_ast::CrudLayoutSlotDef],
) -> Result<(), (usize, String)> {
    let (_, declared_slots) = declared_view_slots(view).map_err(|message| (0, message))?;
    let mut supplied_names = HashSet::new();
    for (index, slot) in supplied_slots.iter().enumerate() {
        if !declared_slots.contains(&slot.name) {
            return Err((index, format!("view has no named slot `{}`", slot.name)));
        }
        if !supplied_names.insert(slot.name.as_str()) {
            return Err((
                index,
                format!("named slot `{}` is supplied more than once", slot.name),
            ));
        }
    }
    Ok(())
}

fn component_prop_accepts(prop: &zelyra_ast::ComponentProp, value: &str) -> bool {
    if value.starts_with('{') && value.ends_with('}') {
        return value.len() > 2;
    }
    match &prop.ty {
        Type::Option(inner) => component_prop_accepts(
            &zelyra_ast::ComponentProp {
                name: prop.name.clone(),
                ty: (**inner).clone(),
                span: prop.span,
            },
            value,
        ),
        Type::Int | Type::UInt => value.parse::<i64>().is_ok(),
        Type::Float | Type::Decimal => value.parse::<f64>().is_ok(),
        Type::Bool => matches!(value, "true" | "false"),
        _ => true,
    }
}

fn template_expressions(html: &str) -> Result<Vec<String>, String> {
    let mut expressions = Vec::new();
    let mut rest = html;
    while let Some(open) = rest.find('{') {
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            return Err("view interpolation has an unterminated `{`".into());
        };
        let expression = after_open[..close].trim();
        if expression.is_empty() {
            return Err("view interpolation cannot be empty".into());
        }
        expressions.push(expression.to_owned());
        rest = &after_open[close + 1..];
    }
    Ok(expressions)
}

fn is_template_identifier(expression: &str) -> bool {
    !expression.is_empty()
        && expression.chars().enumerate().all(|(index, character)| {
            if index == 0 {
                character.is_ascii_alphabetic() || character == '_'
            } else {
                character.is_ascii_alphanumeric() || character == '_'
            }
        })
}

fn is_template_expression(expression: &str) -> bool {
    expression.split('.').all(is_template_identifier)
}

struct TemplateForBlock {
    start: usize,
    end: usize,
    item: String,
    collection: String,
    body: String,
}

fn next_template_for_block(template: &str) -> Option<Result<TemplateForBlock, String>> {
    let mut search_from = 0;
    while let Some(relative_start) = template[search_from..].find("for ") {
        let start = search_from + relative_start;
        let line_start = template[..start].rfind('\n').map_or(0, |index| index + 1);
        if !template[line_start..start].trim().is_empty() {
            search_from = start + 4;
            continue;
        }
        let Some(relative_open) = template[start..].find('{') else {
            return Some(Err("view `for` block is missing `{`".into()));
        };
        let open = start + relative_open;
        let header = template[start + 4..open].trim();
        let parts = header.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 3
            || parts[1] != "in"
            || !is_template_identifier(parts[0])
            || !is_template_identifier(parts[2])
        {
            return Some(Err(
                "view `for` block must use `for item in collection { ... }`".into(),
            ));
        }
        let mut depth = 1;
        let mut position = open + 1;
        while position < template.len() {
            match template.as_bytes()[position] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(Ok(TemplateForBlock {
                            start,
                            end: position + 1,
                            item: parts[0].into(),
                            collection: parts[2].into(),
                            body: template[open + 1..position].into(),
                        }));
                    }
                }
                _ => {}
            }
            position += 1;
        }
        return Some(Err("view `for` block is unterminated".into()));
    }
    None
}

fn resolve_template_type(
    expression: &str,
    bindings: &HashMap<String, Type>,
    program: &zelyra_ast::Program,
) -> Result<Type, String> {
    let mut parts = expression.split('.');
    let root = parts.next().unwrap_or_default();
    let Some(mut ty) = bindings.get(root).cloned() else {
        return Err(format!("unknown view value `{root}`"));
    };
    for field in parts {
        ty = template_field_type(program, &ty, field)?;
    }
    Ok(ty)
}

fn template_field_type(
    program: &zelyra_ast::Program,
    ty: &Type,
    field: &str,
) -> Result<Type, String> {
    let ty = match ty {
        Type::Option(_) => {
            return Err(format!(
                "field `{field}` requires explicit handling of an optional value"
            ));
        }
        Type::Named(name) => {
            if let Some(definition) = program
                .types
                .iter()
                .find(|definition| definition.name == *name)
            {
                return template_field_type(program, &definition.target, field);
            }
            ty
        }
        _ => ty,
    };
    if let Type::Named(name) = ty {
        if let Some(record) = program.records.iter().find(|record| record.name == *name) {
            return record
                .fields
                .iter()
                .find(|candidate| candidate.name == field)
                .map(|candidate| candidate.ty.clone())
                .ok_or_else(|| format!("field `{field}` does not exist on `{name}`"));
        }
        if let Some(table) = program.tables.iter().find(|table| {
            table.name == *name || singular_type_name(&table.name).as_deref() == Some(name)
        }) {
            return table
                .columns
                .iter()
                .find(|candidate| candidate.name == field)
                .map(|candidate| candidate.ty.clone())
                .ok_or_else(|| format!("field `{field}` does not exist on `{name}`"));
        }
    }
    Err(format!("type `{ty}` has no field `{field}`"))
}

fn template_type_compatible(expected: &Type, actual: &Type) -> bool {
    expected == actual
        || matches!(expected, Type::Option(inner) if template_type_compatible(inner, actual))
}

fn validate_template_expressions(
    path: &str,
    html: &str,
    program: &zelyra_ast::Program,
    bindings: &HashMap<String, Type>,
    line: usize,
    column: usize,
) -> bool {
    if let Some(block) = next_template_for_block(html) {
        let block = match block {
            Ok(block) => block,
            Err(message) => {
                diagnostic(path, "E-VIEW-018", &message, line, column);
                return false;
            }
        };
        let mut valid = validate_template_interpolations(
            path,
            &html[..block.start],
            program,
            bindings,
            line,
            column,
        );
        let Some(collection_type) = bindings.get(&block.collection) else {
            diagnostic(
                path,
                "E-VIEW-018",
                &format!("unknown view collection `{}`", block.collection),
                line,
                column,
            );
            return false;
        };
        let Type::Array(item_type) = collection_type else {
            diagnostic(
                path,
                "E-VIEW-018",
                &format!(
                    "view loop source `{}` must have an array type",
                    block.collection
                ),
                line,
                column,
            );
            return false;
        };
        if bindings.contains_key(&block.item) {
            diagnostic(
                path,
                "E-VIEW-018",
                &format!(
                    "view loop variable `{}` conflicts with an existing value",
                    block.item
                ),
                line,
                column,
            );
            return false;
        }
        let mut loop_bindings = bindings.clone();
        loop_bindings.insert(block.item, (**item_type).clone());
        valid &=
            validate_template_expressions(path, &block.body, program, &loop_bindings, line, column);
        valid &= validate_template_expressions(
            path,
            &html[block.end..],
            program,
            bindings,
            line,
            column,
        );
        return valid;
    }
    validate_template_interpolations(path, html, program, bindings, line, column)
}

fn validate_template_interpolations(
    path: &str,
    html: &str,
    program: &zelyra_ast::Program,
    bindings: &HashMap<String, Type>,
    line: usize,
    column: usize,
) -> bool {
    let expressions = match template_expressions(html) {
        Ok(expressions) => expressions,
        Err(message) => {
            diagnostic(path, "E-VIEW-013", &message, line, column);
            return false;
        }
    };
    let mut valid = true;
    for expression in expressions {
        if !is_template_expression(&expression) {
            diagnostic(
                path,
                "E-VIEW-014",
                &format!(
                    "view expression must contain identifiers separated by `.`, found `{{{expression}}}`"
                ),
                line,
                column,
            );
            valid = false;
        } else if let Err(message) = resolve_template_type(&expression, bindings, program) {
            let code = if expression.contains('.') {
                "E-VIEW-016"
            } else {
                "E-VIEW-015"
            };
            diagnostic(path, code, &message, line, column);
            valid = false;
        }
    }
    valid
}

fn validate_component_template(
    path: &str,
    program: &zelyra_ast::Program,
    html: &str,
    line: usize,
    column: usize,
    bindings: &HashMap<String, Type>,
) -> bool {
    let mut valid = validate_template_expressions(path, html, program, bindings, line, column);
    let invocations = match component_invocations(html) {
        Ok(invocations) => invocations,
        Err(message) => {
            diagnostic(path, "E-VIEW-006", &message, line, column);
            return false;
        }
    };
    for invocation in invocations {
        let ComponentInvocation {
            name,
            attributes,
            body,
            ..
        } = invocation;
        let Some(component) = program
            .components
            .iter()
            .find(|component| component.name == name)
        else {
            diagnostic(
                path,
                "E-VIEW-007",
                &format!("unknown view component `{name}`"),
                line,
                column,
            );
            valid = false;
            continue;
        };
        let attributes = match component_attributes(&attributes) {
            Ok(attributes) => attributes,
            Err(message) => {
                diagnostic(path, "E-VIEW-006", &message, line, column);
                valid = false;
                continue;
            }
        };
        let (has_default_slot, named_slots) = match declared_component_slots(component) {
            Ok(slots) => slots,
            Err(message) => {
                diagnostic(path, "E-VIEW-011", &message, line, column);
                valid = false;
                (false, HashSet::new())
            }
        };
        if let Some(body) = body.as_deref() {
            let (default_body, supplied_named_slots) = match split_component_body(body) {
                Ok(slots) => slots,
                Err(message) => {
                    diagnostic(path, "E-VIEW-012", &message, line, column);
                    valid = false;
                    (body.to_owned(), HashMap::new())
                }
            };
            if !default_body.trim().is_empty() && !has_default_slot {
                diagnostic(
                    path,
                    "E-VIEW-011",
                    &format!("component `{name}` receives content but has no `<slot />`"),
                    line,
                    column,
                );
                valid = false;
            }
            for slot_name in supplied_named_slots.keys() {
                if !named_slots.contains(slot_name) {
                    diagnostic(
                        path,
                        "E-VIEW-012",
                        &format!("component `{name}` has no named slot `{slot_name}`"),
                        line,
                        column,
                    );
                    valid = false;
                }
            }
            valid &= validate_component_template(path, program, body, line, column, bindings);
        }
        for attribute in attributes.keys() {
            if !component.props.iter().any(|prop| prop.name == *attribute) {
                diagnostic(
                    path,
                    "E-VIEW-008",
                    &format!("component `{name}` has no property `{attribute}`"),
                    line,
                    column,
                );
                valid = false;
            }
        }
        for prop in &component.props {
            let Some(value) = attributes.get(&prop.name) else {
                if !matches!(prop.ty, Type::Option(_)) {
                    diagnostic(
                        path,
                        "E-VIEW-009",
                        &format!(
                            "component `{name}` is missing required property `{}`",
                            prop.name
                        ),
                        line,
                        column,
                    );
                    valid = false;
                }
                continue;
            };
            let dynamic_type_error = value
                .strip_prefix('{')
                .and_then(|value| value.strip_suffix('}'))
                .map(str::trim)
                .and_then(|expression| resolve_template_type(expression, bindings, program).ok())
                .filter(|actual| !template_type_compatible(&prop.ty, actual));
            if dynamic_type_error.is_some() || !component_prop_accepts(prop, value) {
                diagnostic(
                    path,
                    "E-VIEW-010",
                    &format!(
                        "value `{value}` is incompatible with component property `{}` of type {}",
                        prop.name, prop.ty
                    ),
                    line,
                    column,
                );
                valid = false;
            }
        }
    }
    valid
}

fn validate_components(path: &str, program: &zelyra_ast::Program) -> bool {
    let mut valid = true;
    let mut names = HashSet::new();
    for component in &program.components {
        if !component
            .name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
        {
            diagnostic_with_span(
                path,
                "E-VIEW-004",
                &format!(
                    "view component `{}` must start with an uppercase letter",
                    component.name
                ),
                component.span,
            );
            valid = false;
        }
        if !names.insert(component.name.as_str()) {
            diagnostic_with_span(
                path,
                "E-VIEW-005",
                &format!("duplicate view component `{}`", component.name),
                component.span,
            );
            valid = false;
        }
        if let Err(message) = declared_component_slots(component) {
            diagnostic_with_span(
                path,
                "E-VIEW-011",
                &format!("component `{}`: {message}", component.name),
                component.span,
            );
            valid = false;
        }
        let mut props = HashSet::new();
        for prop in &component.props {
            if !props.insert(prop.name.as_str()) {
                diagnostic_with_span(
                    path,
                    "E-VIEW-005",
                    &format!(
                        "duplicate property `{}` in component `{}`",
                        prop.name, component.name
                    ),
                    prop.span,
                );
                valid = false;
            }
        }
        valid &= validate_component_template(
            path,
            program,
            &component.html,
            component.span.line,
            component.span.column,
            &component
                .props
                .iter()
                .map(|prop| (prop.name.clone(), prop.ty.clone()))
                .collect(),
        );
    }
    for page in &program.pages {
        let bindings = page_template_bindings(
            &page.path,
            &page.data,
            &page.inputs,
            page.page_size,
            !page.sort.is_empty(),
            !page.search.is_empty(),
            &page.filters,
        );
        valid &= validate_component_template(
            path,
            program,
            &page.html,
            page.span.line,
            page.span.column,
            &bindings,
        );
        if let Some(view_name) = &page.view {
            if let Some(view) = program.views.iter().find(|view| view.name == *view_name) {
                valid &= validate_component_template(
                    path,
                    program,
                    &view.html,
                    view.span.line,
                    view.span.column,
                    &bindings,
                );
            }
        }
    }
    for crud in &program.cruds {
        if let Some(layout_name) = &crud.layout {
            if let Some(layout) = program.views.iter().find(|view| view.name == *layout_name) {
                valid &= validate_component_template(
                    path,
                    program,
                    &layout.html,
                    layout.span.line,
                    layout.span.column,
                    &HashMap::new(),
                );
            }
            for slot in &crud.layout_slots {
                valid &= validate_component_template(
                    path,
                    program,
                    &slot.html,
                    slot.span.line,
                    slot.span.column,
                    &HashMap::new(),
                );
            }
        }
    }
    valid
}

fn page_template_bindings(
    path: &str,
    data: &[zelyra_ast::PageDataDef],
    inputs: &[zelyra_ast::PageInputDef],
    page_size: Option<u32>,
    sort_enabled: bool,
    search_enabled: bool,
    filter_names: &[String],
) -> HashMap<String, Type> {
    let mut bindings = path
        .split('/')
        .filter_map(|segment| {
            segment
                .strip_prefix('{')
                .and_then(|segment| segment.strip_suffix('}'))
                .filter(|name| is_template_identifier(name))
                .map(|name| (name.to_owned(), Type::String))
        })
        .collect::<HashMap<_, _>>();
    for input in inputs {
        bindings.insert(input.name.clone(), input.ty.clone());
    }
    if page_size.is_some() {
        bindings.insert("page".into(), Type::UInt);
        bindings.insert("total".into(), Type::UInt);
        bindings.insert("pages".into(), Type::UInt);
    }
    if sort_enabled {
        bindings.insert("sort".into(), Type::String);
        bindings.insert("order".into(), Type::String);
    }
    if search_enabled {
        bindings.insert("search".into(), Type::String);
    }
    for filter_name in filter_names {
        bindings.insert(format!("filter_{filter_name}"), Type::String);
        bindings.insert(format!("filter_{filter_name}__operator"), Type::String);
    }
    for data in data {
        bindings.insert(data.name.clone(), data.result_type.clone());
    }
    bindings
}

fn verify_command(path: &str, json: bool) -> ExitCode {
    let program = match validate(path) {
        Ok(program) => program,
        Err(()) => return ExitCode::from(1),
    };
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error[E-IO-001]: cannot read `{path}`: {error}");
            return ExitCode::from(1);
        }
    };
    let results = verify_program(&program);
    let failed = results
        .iter()
        .any(|result| result.status == VerificationStatus::Failed);
    if json {
        println!("{}", format_verification_json(path, &source, &results));
    } else {
        for result in &results {
            println!("{}", format_verification_result(path, &source, result));
        }
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

struct DoctorCheck {
    name: &'static str,
    category: &'static str,
    status: &'static str,
    message: String,
}

fn read_env_value(path: &str, key: &str) -> Result<Option<String>, String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("cannot read `{path}`: {error}"))?;
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim() == key {
            return Ok(Some(
                value.trim().trim_matches('"').trim_matches('\'').to_owned(),
            ));
        }
    }
    Ok(None)
}

fn database_url_environment_name(database_name: &str) -> String {
    let normalized = database_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("ZELYRA_DATABASE_{normalized}_URL")
}

fn database_url_from_environment(database_name: Option<&str>) -> Option<String> {
    database_name
        .map(database_url_environment_name)
        .and_then(|name| env::var(name).ok())
        .or_else(|| env::var("DATABASE_URL").ok())
}

fn database_url_from_program(program: &zelyra_ast::Program) -> Option<String> {
    database_url_from_environment(
        program
            .databases
            .first()
            .map(|database| database.name.as_str()),
    )
}

fn database_url_from_schema(schema: &Schema) -> Option<String> {
    database_url_from_environment(
        schema
            .database
            .as_ref()
            .map(|database| database.name.as_str()),
    )
}

fn database_url_from_env_file(
    path: &str,
    database_name: Option<&str>,
) -> Result<Option<(String, String)>, String> {
    if let Some(database_name) = database_name {
        let environment_name = database_url_environment_name(database_name);
        if let Some(url) = read_env_value(path, &environment_name)? {
            return Ok(Some((url, environment_name)));
        }
    }
    Ok(read_env_value(path, "DATABASE_URL")?.map(|url| (url, "DATABASE_URL".to_owned())))
}

fn project_ui_setting(path: &str, key: &str, default: &str) -> Result<String, String> {
    if let Ok(value) = env::var(key) {
        return Ok(value);
    }
    let source_path = std::path::Path::new(path);
    let project_directory = source_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let env_path = project_directory.join(".env");
    if !env_path.is_file() {
        return Ok(default.to_owned());
    }
    let env_path_string = env_path.to_string_lossy();
    Ok(read_env_value(&env_path_string, key)?.unwrap_or_else(|| default.to_owned()))
}

fn project_ui_settings(path: &str) -> Result<(UiLanguage, UiLevel), String> {
    let language = project_ui_setting(path, "ZELYRA_LANGUAGE", "en")?;
    let language = UiLanguage::parse(&language.to_ascii_lowercase())
        .ok_or_else(|| "ZELYRA_LANGUAGE must be `en` or `de`".to_owned())?;
    let level = project_ui_setting(path, "ZELYRA_LEVEL", "work")?;
    let level = UiLevel::parse(&level.to_ascii_lowercase())
        .ok_or_else(|| "ZELYRA_LEVEL must be `learn` or `work`".to_owned())?;
    Ok((language, level))
}

fn project_allowed_hosts(path: &str) -> Result<Vec<String>, String> {
    let configured = project_ui_setting(path, "ZELYRA_ALLOWED_HOSTS", "localhost,127.0.0.1,[::1]")?;
    let hosts = configured
        .split(',')
        .map(str::trim)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if hosts.is_empty() || hosts.iter().any(String::is_empty) {
        return Err(
            "ZELYRA_ALLOWED_HOSTS must contain comma-separated, non-empty hostnames or IP addresses".into(),
        );
    }
    Ok(hosts)
}

fn project_theme_css(path: &str) -> Result<Option<String>, String> {
    let source_path = std::path::Path::new(path);
    let project_directory = source_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let theme_path = project_directory.join(PROJECT_THEME_CSS_FILE);
    let metadata = match fs::symlink_metadata(&theme_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(format!("cannot inspect `{PROJECT_THEME_CSS_FILE}`")),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "`{PROJECT_THEME_CSS_FILE}` must be a regular project file, not a symbolic link"
        ));
    }
    if metadata.len() > PROJECT_THEME_CSS_MAX_BYTES {
        return Err(format!(
            "`{PROJECT_THEME_CSS_FILE}` exceeds the 128 KiB size limit"
        ));
    }

    let file = fs::File::open(&theme_path)
        .map_err(|_| format!("cannot read `{PROJECT_THEME_CSS_FILE}`"))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(PROJECT_THEME_CSS_MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| format!("cannot read `{PROJECT_THEME_CSS_FILE}`"))?;
    if bytes.len() as u64 > PROJECT_THEME_CSS_MAX_BYTES {
        return Err(format!(
            "`{PROJECT_THEME_CSS_FILE}` exceeds the 128 KiB size limit"
        ));
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| format!("`{PROJECT_THEME_CSS_FILE}` must contain UTF-8 text"))
}

fn project_ui_catalogs(path: &str) -> Result<ProjectUiCatalogs, String> {
    let source_path = std::path::Path::new(path);
    let project_directory = source_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let locale_directory = project_directory.join(PROJECT_LOCALE_DIRECTORY);
    let directory_metadata = match fs::symlink_metadata(&locale_directory) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ProjectUiCatalogs::default());
        }
        Err(_) => return Err("cannot inspect project locale directory".to_owned()),
    };
    if directory_metadata.file_type().is_symlink() || !directory_metadata.is_dir() {
        return Err("project `locales` must be a regular directory and not a symbolic link".into());
    }

    let mut catalogs = ProjectUiCatalogs::default();
    for language in [UiLanguage::German, UiLanguage::English] {
        let file_name = format!("{}.json", language.code());
        let display_name = format!("{PROJECT_LOCALE_DIRECTORY}/{file_name}");
        let catalog_path = locale_directory.join(&file_name);
        let metadata = match fs::symlink_metadata(&catalog_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err(format!("cannot inspect `{display_name}`")),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(format!(
                "`{display_name}` must be a regular file, not a symbolic link"
            ));
        }
        if metadata.len() > PROJECT_LOCALE_MAX_BYTES {
            return Err(format!("`{display_name}` exceeds the 256 KiB size limit"));
        }

        let file =
            fs::File::open(&catalog_path).map_err(|_| format!("cannot read `{display_name}`"))?;
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        file.take(PROJECT_LOCALE_MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| format!("cannot read `{display_name}`"))?;
        if bytes.len() as u64 > PROJECT_LOCALE_MAX_BYTES {
            return Err(format!("`{display_name}` exceeds the 256 KiB size limit"));
        }
        let source = String::from_utf8(bytes)
            .map_err(|_| format!("`{display_name}` must contain UTF-8 text"))?;
        catalogs
            .set_json(language, &source)
            .map_err(|error| format!("invalid `{display_name}`: {error}"))?;
    }
    Ok(catalogs)
}

fn project_uses_reserved_theme_route(program: &zelyra_ast::Program) -> bool {
    program
        .pages
        .iter()
        .any(|page| zelyra_web::route_pattern_matches_path(&page.path, PROJECT_THEME_CSS_PATH))
        || program
            .apis
            .iter()
            .any(|api| zelyra_web::route_pattern_matches_path(&api.path, PROJECT_THEME_CSS_PATH))
        || program.auth.iter().any(|auth| {
            auth.admin_path.as_deref().is_some_and(|path| {
                zelyra_web::route_pattern_matches_path(path, PROJECT_THEME_CSS_PATH)
            })
        })
}

fn docker_compose_check() -> DoctorCheck {
    if let Some(command) = detect_docker_compose() {
        return DoctorCheck {
            name: "docker_compose",
            category: "tooling",
            status: "pass",
            message: format!("{} is available", command.label()),
        };
    }
    DoctorCheck {
        name: "docker_compose",
        category: "tooling",
        status: "warn",
        message: format!(
            "{} The generated MariaDB stack cannot be started until Docker Compose is available.",
            docker_compose_install_hint()
        ),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DockerComposeCommand {
    Plugin,
    Legacy,
}

impl DockerComposeCommand {
    fn label(self) -> &'static str {
        match self {
            Self::Plugin => "docker compose",
            Self::Legacy => "docker-compose",
        }
    }
}

fn detect_docker_compose() -> Option<DockerComposeCommand> {
    let plugin = Command::new("docker")
        .args(["compose", "version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .is_some_and(|status| status.success());
    if plugin {
        return Some(DockerComposeCommand::Plugin);
    }
    let legacy = Command::new("docker-compose")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .is_some_and(|status| status.success());
    legacy.then_some(DockerComposeCommand::Legacy)
}

fn docker_compose_install_hint() -> String {
    let (platform, url) = if cfg!(target_os = "linux") {
        ("Linux", "https://docs.docker.com/engine/install/")
    } else if cfg!(target_os = "windows") {
        (
            "Windows",
            "https://docs.docker.com/desktop/setup/install/windows-install/",
        )
    } else if cfg!(target_os = "macos") {
        (
            "macOS",
            "https://docs.docker.com/desktop/setup/install/mac-install/",
        )
    } else {
        (
            "your operating system",
            "https://docs.docker.com/engine/install/",
        )
    };
    format!(
        "Docker Compose is unavailable. Install Docker for {platform} from:\n{url}\nAfter installation, verify with `docker compose version`, then run this step again."
    )
}

fn print_compose_start_hint() {
    match detect_docker_compose() {
        Some(command) => println!(
            "  {} --env-file .env -f docker-compose.mariadb.yml up -d --build",
            command.label()
        ),
        None => println!("{}", docker_compose_install_hint()),
    }
}

fn compose_command(directory: &std::path::Path, command: DockerComposeCommand) -> Command {
    let mut process = match command {
        DockerComposeCommand::Plugin => {
            let mut process = Command::new("docker");
            process.arg("compose");
            process
        }
        DockerComposeCommand::Legacy => Command::new("docker-compose"),
    };
    process
        .current_dir(directory)
        .args(["--env-file", ".env", "-f", "docker-compose.mariadb.yml"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    process
}

fn compose_start_failure_message(command: DockerComposeCommand, details: &str) -> String {
    let details = details.to_ascii_lowercase();
    if details.contains("permission denied")
        && (details.contains("docker.sock") || details.contains("docker"))
    {
        "Docker access was denied. On Linux, add the current user to the `docker` group with `sudo usermod -aG docker $USER`, then either fully sign out and sign in again or run these commands in the current terminal:\n  newgrp docker\n  id -nG\n  docker ps\nRetry setup when `docker` appears in the group list and `docker ps` succeeds. Opening another terminal window alone may not refresh group membership. Alternatively follow your distribution's Docker setup instructions.".into()
    } else if details.contains("address already in use")
        || details.contains("port is already allocated")
        || details.contains("failed to bind")
    {
        "a published web or MariaDB port is already in use. For a newly created .env, run `zelyra setup` to select free defaults; for an existing .env, choose free ZELYRA_HOST_PORT and ZELYRA_DB_HOST_PORT values, then retry.".into()
    } else {
        format!(
            "{} could not start the generated MariaDB application. Inspect the stack with `{} --env-file .env -f docker-compose.mariadb.yml logs`.",
            command.label(),
            command.label()
        )
    }
}

fn start_mariadb_compose(directory: &std::path::Path) -> Result<String, String> {
    if !directory.join("docker-compose.mariadb.yml").is_file() {
        return Err(
            "docker-compose.mariadb.yml is missing; use `zelyra init --mariadb` first".into(),
        );
    }
    let Some(command) = detect_docker_compose() else {
        return Err(docker_compose_install_hint());
    };
    let output = compose_command(directory, command)
        .args(["up", "-d", "--build"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("could not start {}: {error}", command.label()))?;
    if output.status.success() {
        Ok(format!(
            "MariaDB and the application were started with {}",
            command.label()
        ))
    } else {
        let details = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Err(compose_start_failure_message(command, &details))
    }
}

fn expanded_database_url(directory: &std::path::Path) -> Result<String, String> {
    let env_path = directory.join(".env");
    let database_name = directory
        .join("main.zyl")
        .to_str()
        .and_then(|path| load_project(path).ok())
        .and_then(|project| {
            project
                .program
                .databases
                .first()
                .map(|database| database.name.clone())
        });
    let url = database_url_from_env_file(
        env_path.to_str().unwrap_or(".env"),
        database_name.as_deref(),
    )?
    .map(|(url, _)| url)
    .ok_or_else(|| "`.env` does not define the project database URL or DATABASE_URL".to_owned())?;
    let port = read_env_value(env_path.to_str().unwrap_or(".env"), "ZELYRA_DB_HOST_PORT")?
        .unwrap_or_else(|| DEFAULT_DATABASE_HOST_PORT.to_string());
    Ok(url.replace("${ZELYRA_DB_HOST_PORT:-3306}", &port))
}

fn project_web_url(directory: &std::path::Path) -> Result<String, String> {
    let env_path = directory.join(".env");
    let env_path_string = env_path.to_string_lossy();
    project_web_url_with_host_port(
        directory,
        env::var("ZELYRA_HOST_PORT").ok().as_deref(),
        &env_path_string,
    )
}

fn project_web_url_with_host_port(
    directory: &std::path::Path,
    host_port_override: Option<&str>,
    env_path: &str,
) -> Result<String, String> {
    let port = match host_port_override.map(str::to_owned) {
        Some(value) => Some(value),
        None => read_env_value(env_path, "ZELYRA_HOST_PORT")?,
    };
    let port = match port {
        Some(value) => parse_web_port(&value)?,
        None => {
            let template = local_mariadb_template(directory)?;
            template_port(&template, "ZELYRA_HOST_PORT", DEFAULT_WEB_PORT)
        }
    };
    Ok(format!("http://127.0.0.1:{port}"))
}

fn run_local_schema_setup(directory: &std::path::Path) -> Result<(), String> {
    let database_url = expanded_database_url(directory)?;
    let executable =
        env::current_exe().map_err(|error| format!("cannot locate zelyra: {error}"))?;
    let status = Command::new(executable)
        .current_dir(directory)
        .args(["db", "setup", "main.zyl"])
        .env("DATABASE_URL", database_url)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| format!("could not run schema setup: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("schema setup failed; verify that MariaDB is reachable and the configured credentials are correct".into())
    }
}

fn run_container_schema_setup(directory: &std::path::Path) -> Result<(), String> {
    let Some(command) = detect_docker_compose() else {
        return Err(docker_compose_install_hint());
    };
    for _attempt in 0..20 {
        let status = compose_command(directory, command)
            .args(["exec", "-T", "web", "zelyra", "db", "apply", "main.zyl"])
            .status()
            .map_err(|error| {
                format!("could not run schema setup in the application container: {error}")
            })?;
        if status.success() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    Err("schema setup did not succeed after waiting for MariaDB; inspect the Compose logs and retry".into())
}

fn setup_action(path: &str, action: &str, options: &SetupOptions) -> Result<String, String> {
    let directory = std::path::Path::new(path);
    if !directory.is_dir() {
        return Err(format!("project directory `{path}` does not exist"));
    }
    let setup = ensure_local_env_file(directory, options)?;
    let mut messages = setup
        .port_notes
        .into_iter()
        .map(|note| format!("note: {note}"))
        .collect::<Vec<_>>();
    messages.push(if setup.created {
        "created protected .env".to_owned()
    } else {
        "kept existing .env; credentials were not changed".to_owned()
    });
    if matches!(action, "database" | "schema" | "all") {
        let web_url = project_web_url(directory)?;
        messages.push(start_mariadb_compose(directory)?);
        messages.push(format!("open: {web_url}"));
    }
    if matches!(action, "schema" | "all") {
        let schema_result = if directory.join("docker-compose.mariadb.yml").is_file() {
            run_container_schema_setup(directory)
        } else {
            run_local_schema_setup(directory)
        };
        schema_result?;
        messages.push("database schema setup completed".into());
    }
    Ok(messages.join("\n"))
}

struct SetupWebState {
    directory: PathBuf,
    token: String,
    message: String,
}

fn setup_web_token(
    request: &zelyra_web::Request,
    input: Option<&HashMap<String, String>>,
) -> Option<String> {
    if let Some(input) = input.and_then(|values| values.get("token")) {
        return Some(input.clone());
    }
    request
        .target
        .split_once('?')
        .and_then(|(_, query)| parse_urlencoded(query).ok())
        .and_then(|values| values.get("token").cloned())
}

fn setup_web_html(state: &SetupWebState) -> String {
    let directory = &state.directory;
    let env_status = if directory.join(".env").is_file() {
        "bereit"
    } else {
        "nicht angelegt"
    };
    let compose_status = if directory.join("docker-compose.mariadb.yml").is_file() {
        match detect_docker_compose() {
            Some(command) => format!("{} available", command.label()),
            None => docker_compose_install_hint(),
        }
    } else {
        "kein MariaDB-Compose-Projekt erkannt".into()
    };
    let project_status = if directory.join("main.zyl").is_file() {
        "main.zyl gefunden"
    } else {
        "main.zyl fehlt"
    };
    let message = if state.message.is_empty() {
        String::new()
    } else {
        format!(
            "<section><strong>Status</strong><pre>{}</pre></section>",
            html_escape(&state.message)
        )
    };
    format!(
        "<!doctype html><html lang=\"de\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Zelyra Setup</title><style>body{{font-family:system-ui,sans-serif;max-width:800px;margin:3rem auto;padding:0 1rem;color:#17202a;background:#f6f8fa}}main{{background:white;padding:2rem;border-radius:12px;box-shadow:0 4px 24px #0001}}button{{margin:.4rem .4rem .4rem 0;padding:.7rem 1rem;border:0;border-radius:7px;background:#1769aa;color:white;cursor:pointer}}section{{margin:1rem 0;padding:1rem;background:#eef6ff;border-left:4px solid #2675d8}}pre{{white-space:pre-wrap}}</style></head><body><main><h1>Zelyra Setup</h1><p>Lokaler Installationsassistent für <code>{directory}</code>.</p><p><small>Der Server bindet nur an 127.0.0.1. Docker selbst wird nicht mit Root-Rechten installiert.</small></p>{message}<h2>Prüfung</h2><ul><li>{project_status}</li><li>.env: {env_status}</li><li>Compose: {compose_status}</li></ul><h2>Aktionen</h2><form method=\"post\" action=\"/\"><input type=\"hidden\" name=\"token\" value=\"{token}\"><button name=\"action\" value=\"prepare\">Konfiguration vorbereiten</button><button name=\"action\" value=\"database\">MariaDB und Anwendung starten</button><button name=\"action\" value=\"schema\">Datenbankschema einrichten</button><button name=\"action\" value=\"all\">Alles ausführen</button></form><p><small>Fehlendes Docker wird mit einem Installationshinweis gemeldet. Zugangsdaten werden niemals angezeigt.</small></p></main></body></html>",
        directory = html_escape(&directory.display().to_string()),
        project_status = html_escape(project_status),
        env_status = html_escape(env_status),
        compose_status = html_escape(&compose_status),
        message = message,
        token = html_escape(&state.token),
    )
}

fn setup_web_response(
    state: &Arc<Mutex<SetupWebState>>,
    request: &zelyra_web::Request,
) -> Response {
    let input = if request.method == "POST" {
        match parse_urlencoded(&request.body) {
            Ok(values) => Some(values),
            Err(error) => {
                return Response::html(
                    400,
                    format!(
                        "<h1>400 Bad Request</h1><p>{}</p>",
                        html_escape(&error.message)
                    ),
                )
            }
        }
    } else {
        None
    };
    let Ok(mut state) = state.lock() else {
        return Response::html(500, "<h1>500 Internal Server Error</h1>");
    };
    if setup_web_token(request, input.as_ref()) != Some(state.token.clone()) {
        return Response::html(403, "<h1>403 Forbidden</h1><p>Invalid setup token.</p>");
    }
    if request.method == "POST" {
        let action = input
            .as_ref()
            .and_then(|values| values.get("action"))
            .map(String::as_str)
            .unwrap_or("prepare");
        state.message = match action {
            "prepare" | "database" | "schema" | "all" => setup_action(
                state.directory.to_str().unwrap_or("."),
                action,
                &SetupOptions::default(),
            )
            .unwrap_or_else(|error| format!("Fehler: {error}")),
            _ => "Fehler: unbekannte Setup-Aktion".into(),
        };
    }
    Response::html(200, setup_web_html(&state))
}

fn setup_web_command(path: &str, port: u16, port_given: bool) -> ExitCode {
    let directory = std::path::Path::new(path);
    if !directory.is_dir() {
        eprintln!("error[E-SETUP-001]: project directory `{path}` does not exist");
        return ExitCode::from(1);
    }
    let (port, port_note) = match resolve_host_port(port, port_given, "setup web", true, &[]) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("error[E-SETUP-WEB-001]: {error}");
            return ExitCode::from(2);
        }
    };
    if let Some(note) = port_note {
        println!("note: {note}");
    }
    let Ok(token) = generate_local_secret() else {
        eprintln!("error[E-SETUP-WEB-001]: cannot create a secure setup token");
        return ExitCode::from(1);
    };
    let address = format!("127.0.0.1:{port}");
    let state = Arc::new(Mutex::new(SetupWebState {
        directory: directory.to_path_buf(),
        token: token.clone(),
        message: String::new(),
    }));
    let get_state = Arc::clone(&state);
    let post_state = Arc::clone(&state);
    let routes = vec![
        ApiRoute::new("GET", "/", move |request, _| {
            setup_web_response(&get_state, request)
        }),
        ApiRoute::new("POST", "/", move |request, _| {
            setup_web_response(&post_state, request)
        }),
    ];
    println!("Zelyra setup web is running on http://{address}/");
    println!("open: http://{address}/?token={token}");
    println!("stop with Ctrl+C");
    match serve_app(
        WebApp::new(Vec::new(), Vec::new()).with_apis(routes),
        &address,
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "error[E-SETUP-WEB-002]: cannot start setup web server on {address}: {error}"
            );
            ExitCode::from(1)
        }
    }
}

fn format_doctor_json(path: &str, checks: &[DoctorCheck]) -> String {
    let failed = checks.iter().any(|check| check.status == "fail");
    let warnings = checks.iter().filter(|check| check.status == "warn").count();
    let checks = checks
        .iter()
        .map(|check| {
            serde_json::json!({
                "name": check.name,
                "category": check.category,
                "status": check.status,
                "message": check.message,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "project": path,
        "status": if failed { "failed" } else { "ready" },
        "warnings": warnings,
        "checks": checks,
    })
    .to_string()
}

fn doctor_database_category(message: &str) -> &'static str {
    let message = message.to_ascii_lowercase();
    if message.contains("access denied")
        || message.contains("authentication failed")
        || message.contains("rejected authentication")
        || message.contains("password authentication failed")
        || message.contains("no pg_hba.conf entry")
    {
        "authentication"
    } else if message.contains("timed out")
        || message.contains("timeout")
        || message.contains("time out")
        || message.contains("query execution was interrupted")
        || message.contains("max_statement_time")
    {
        "timeout"
    } else if message.contains("database_url is not set")
        || message.contains("must use mariadb://")
        || message.contains("must include a database name")
        || message.contains("must include user and host")
        || message.contains("contains an empty user")
        || message.contains("unknown database")
        || (message.contains("database ") && message.contains(" does not exist"))
        || message.contains("configuration is invalid")
    {
        "configuration"
    } else if message.contains("could not start mariadb")
        || message.contains("could not start psql")
        || message.contains("could not start sqlite")
        || message.contains("no such file or directory")
        || message.contains("client is unavailable")
    {
        "tooling"
    } else if message.contains("could not connect")
        || message.contains("can't connect")
        || message.contains("cannot connect")
        || message.contains("connection refused")
        || message.contains("connection reset")
        || message.contains("server has gone away")
        || message.contains("certificate")
        || message.contains("tls")
        || message.contains("ssl")
    {
        "connectivity"
    } else {
        "schema"
    }
}

fn doctor_database_failure_message(category: &str) -> &'static str {
    match category {
        "authentication" => "database rejected authentication; check the configured user and grants",
        "timeout" => "database check timed out; check server responsiveness and configured timeouts",
        "configuration" => "database configuration is invalid or the configured database is unavailable",
        "tooling" => "database client is unavailable; install the client required by this backend",
        "connectivity" => "could not connect securely to the database; check host, port, TLS, and server status",
        _ => "could not inspect the database schema; check schema compatibility and metadata permissions",
    }
}

fn doctor_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let mut path = "main.zyl".to_owned();
    let mut path_given = false;
    let mut port = DEFAULT_WEB_PORT;
    let mut env_file = None;
    let mut json = false;
    while let Some(argument) = args.next() {
        if argument == "--json" {
            json = true;
        } else if argument == "--port" {
            let Some(value) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            port = match parse_web_port(&value) {
                Ok(port) => port,
                Err(error) => {
                    eprintln!("error[E-DOCTOR-001]: {error}");
                    return ExitCode::from(2);
                }
            };
        } else if argument == "--env-file" {
            let Some(value) = args.next() else {
                usage();
                return ExitCode::from(2);
            };
            env_file = Some(value);
        } else if argument.starts_with('-') || path_given {
            usage();
            return ExitCode::from(2);
        } else {
            path = argument;
            path_given = true;
        }
    }

    let mut checks = Vec::new();
    let program = if fs::metadata(&path).is_ok() {
        checks.push(DoctorCheck {
            name: "project_file",
            category: "configuration",
            status: "pass",
            message: format!("{path} exists"),
        });
        match validate(&path) {
            Ok(program) => {
                checks.push(DoctorCheck {
                    name: "static_checks",
                    category: "project",
                    status: "pass",
                    message: "source, types, APIs, SQL, and forms are valid".into(),
                });
                Some(program)
            }
            Err(()) => {
                checks.push(DoctorCheck {
                    name: "static_checks",
                    category: "project",
                    status: "fail",
                    message: "see diagnostics above".into(),
                });
                None
            }
        }
    } else {
        checks.push(DoctorCheck {
            name: "project_file",
            category: "configuration",
            status: "fail",
            message: format!("`{path}` does not exist"),
        });
        None
    };

    let file_database_url = if let Some(env_file) = env_file.as_deref() {
        let database_name = program
            .as_ref()
            .and_then(|program| program.databases.first())
            .map(|database| database.name.as_str());
        match database_url_from_env_file(env_file, database_name) {
            Ok(Some((url, key))) => {
                checks.push(DoctorCheck {
                    name: "env_file",
                    category: "configuration",
                    status: "pass",
                    message: format!("loaded {key} from {env_file} without exposing credentials"),
                });
                Some(url)
            }
            Ok(None) => {
                let expected_key = database_name
                    .map(database_url_environment_name)
                    .unwrap_or_else(|| "DATABASE_URL".to_owned());
                checks.push(DoctorCheck {
                    name: "env_file",
                    category: "configuration",
                    status: "warn",
                    message: format!("{env_file} does not define {expected_key} or DATABASE_URL"),
                });
                None
            }
            Err(error) => {
                checks.push(DoctorCheck {
                    name: "env_file",
                    category: "configuration",
                    status: "fail",
                    message: error,
                });
                None
            }
        }
    } else {
        None
    };

    match Command::new("cargo").arg("--version").output() {
        Ok(output) if output.status.success() => checks.push(DoctorCheck {
            name: "rust_toolchain",
            category: "tooling",
            status: "pass",
            message: String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        }),
        _ => checks.push(DoctorCheck {
            name: "rust_toolchain",
            category: "tooling",
            status: "warn",
            message: "cargo is unavailable".into(),
        }),
    }

    if let Some(program) = &program {
        match build_schema(program) {
            Ok(schema) => {
                let backend = schema.backend();
                match file_database_url.or_else(|| database_url_from_program(program)) {
                    Some(url) => match inspect_for_backend(backend, &url) {
                        Ok(current) => checks.push(DoctorCheck {
                            name: "database",
                            category: "schema",
                            status: "pass",
                            message: format!(
                                "{}: {}",
                                backend.name(),
                                current.summary().replace('\n', ", ")
                            ),
                        }),
                        Err(error) => {
                            let category = doctor_database_category(&error.message);
                            checks.push(DoctorCheck {
                                name: "database",
                                category,
                                status: "fail",
                                message: format!(
                                    "{}: {}",
                                    backend.name(),
                                    doctor_database_failure_message(category)
                                ),
                            });
                        }
                    },
                    None => checks.push(DoctorCheck {
                        name: "database",
                        category: "configuration",
                        status: "warn",
                        message: format!("{}: DATABASE_URL is not set", backend.name()),
                    }),
                }
            }
            Err(errors) => checks.push(DoctorCheck {
                name: "schema",
                category: "schema",
                status: "fail",
                message: errors
                    .iter()
                    .map(|error| error.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; "),
            }),
        }
    }

    checks.push(docker_compose_check());

    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(listener) => {
            let actual_port = listener.local_addr().map_or(port, |address| address.port());
            checks.push(DoctorCheck {
                name: "web_port",
                category: "connectivity",
                status: "pass",
                message: format!("127.0.0.1:{actual_port} is available"),
            });
        }
        Err(error) => checks.push(DoctorCheck {
            name: "web_port",
            category: "connectivity",
            status: "fail",
            message: format!("127.0.0.1:{port} is unavailable ({error})"),
        }),
    }

    let failed = checks.iter().any(|check| check.status == "fail");
    if json {
        println!("{}", format_doctor_json(&path, &checks));
    } else {
        println!("Zelyra doctor {}", env!("CARGO_PKG_VERSION"));
        for check in &checks {
            let label = match check.status {
                "pass" => "PASS",
                "warn" => "WARN",
                _ => "FAIL",
            };
            println!("  [{label}] {}: {}", check.name, check.message);
        }
        let warnings = checks.iter().filter(|check| check.status == "warn").count();
        println!(
            "Doctor result: {} ({warnings} warning(s))",
            if failed { "failed" } else { "ready" }
        );
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn doc_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(path) = args.next() else {
        usage();
        return ExitCode::from(2);
    };
    let format = match args.next() {
        None => "openapi",
        Some(flag) if flag == "--openapi" => "openapi",
        Some(flag) if flag == "--typescript" => "typescript",
        Some(_) => {
            usage();
            return ExitCode::from(2);
        }
    };
    if args.next().is_some() {
        usage();
        return ExitCode::from(2);
    }
    let program = match load_project(&path) {
        Ok(project) => project.program,
        Err(()) => return ExitCode::from(1),
    };
    if format == "typescript" {
        println!("{}", format_typescript_client(&program));
    } else {
        println!("{}", format_openapi(&program));
    }
    ExitCode::SUCCESS
}

fn format_openapi(program: &zelyra_ast::Program) -> String {
    let paths = program
        .apis
        .iter()
        .map(|api| {
            let method = api.method.to_ascii_lowercase();
            let path_parameters = api
                .input
                .iter()
                .filter(|field| api.path.contains(&format!("{{{}}}", field.name)))
                .map(|field| {
                    format!(
                        "{{\"name\":\"{}\",\"in\":\"path\",\"required\":true,\"schema\":{}}}",
                        json_escape(&field.name),
                        openapi_schema(&field.ty)
                    )
                })
                .collect::<Vec<_>>();
            let query_parameters = if matches!(api.method.as_str(), "GET" | "DELETE") {
                api.input
                    .iter()
                    .filter(|field| !api.path.contains(&format!("{{{}}}", field.name)))
                    .map(|field| {
                        format!(
                            "{{\"name\":\"{}\",\"in\":\"query\",\"required\":{},\"schema\":{}}}",
                            json_escape(&field.name),
                            !matches!(field.ty, Type::Option(_)),
                            openapi_schema(&field.ty)
                        )
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let request_body = if matches!(api.method.as_str(), "GET" | "DELETE") {
                String::new()
            } else {
                let properties = api
                    .input
                    .iter()
                    .filter(|field| !api.path.contains(&format!("{{{}}}", field.name)))
                    .map(|field| {
                        format!(
                            "\"{}\":{}",
                            json_escape(&field.name),
                            openapi_schema(&field.ty)
                        )
                    })
                    .collect::<Vec<_>>();
                let required = api
                    .input
                    .iter()
                    .filter(|field| {
                        !api.path.contains(&format!("{{{}}}", field.name))
                            && !matches!(field.ty, Type::Option(_))
                    })
                    .map(|field| format!("\"{}\"", json_escape(&field.name)))
                    .collect::<Vec<_>>();
                if properties.is_empty() {
                    String::new()
                } else {
                    format!(
                        "\"requestBody\":{{\"required\":true,\"content\":{{\"application/json\":{{\"schema\":{{\"type\":\"object\",\"properties\":{{{}}},\"required\":[{}]}}}}}}}},",
                        properties.join(","),
                        required.join(",")
                    )
                }
            };
            let mut parameters = path_parameters;
            parameters.extend(query_parameters);
            let security = if api.requires_auth || !api.permissions.is_empty() {
                format!(
                    ",\"x-zelyra-requires-auth\":{},\"x-zelyra-permissions\":[{}]",
                    api.requires_auth,
                    api.permissions
                        .iter()
                        .map(|permission| format!("\"{}\"", json_escape(permission)))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            } else {
                String::new()
            };
            let responses = std::iter::once(format!(
                "\"200\":{{\"description\":\"Successful response\",\"content\":{{\"application/json\":{{\"schema\":{}}}}}}}",
                openapi_schema(&api.output)
            ))
            .chain(api.errors.iter().map(|error| {
                let response = if let Some(payload) = &error.payload {
                    format!(
                        "{{\"description\":\"{}\",\"content\":{{\"application/json\":{{\"schema\":{}}}}}}}",
                        json_escape(&error.name),
                        openapi_error_schema(payload)
                    )
                } else {
                    format!("{{\"description\":\"{}\"}}", json_escape(&error.name))
                };
                format!("\"{}\":{}", error.status, response)
            }))
            .collect::<Vec<_>>()
            .join(",");
            let operation_id = format!(
                "{}_{}",
                method,
                api.path
                    .trim_matches('/')
                    .replace(['{', '}'], "")
                    .replace('/', "_")
            );
            format!(
                "\"{}\":{{\"{}\":{{\"operationId\":\"{}\",\"parameters\":[{}],{}\"responses\":{{{}}}{}}}}}",
                json_escape(&api.path),
                method,
                json_escape(&operation_id),
                parameters.join(","),
                request_body,
                responses,
                security
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let components = format_openapi_components(program);
    let compiler_version = env!("CARGO_PKG_VERSION");
    format!(
        "{{\"openapi\":\"3.0.3\",\"info\":{{\"title\":\"Zelyra API\",\"version\":\"{compiler_version}\"}},\"paths\":{{{paths}}},\"components\":{{\"schemas\":{{{components}}}}}}}"
    )
}

fn format_openapi_components(program: &zelyra_ast::Program) -> String {
    let mut components = Vec::new();
    for definition in &program.types {
        components.push(format!(
            "\"{}\":{}",
            json_escape(&definition.name),
            openapi_schema(&definition.target)
        ));
    }
    for record in &program.records {
        let properties = record
            .fields
            .iter()
            .map(|field| {
                format!(
                    "\"{}\":{}",
                    json_escape(&field.name),
                    openapi_schema(&field.ty)
                )
            })
            .collect::<Vec<_>>();
        let required = record
            .fields
            .iter()
            .filter(|field| !matches!(field.ty, Type::Option(_)))
            .map(|field| format!("\"{}\"", json_escape(&field.name)))
            .collect::<Vec<_>>();
        components.push(format!(
            "\"{}\":{{\"type\":\"object\",\"properties\":{{{}}},\"required\":[{}]}}",
            json_escape(&record.name),
            properties.join(","),
            required.join(",")
        ));
    }
    for table in &program.tables {
        let properties = table
            .columns
            .iter()
            .map(|column| {
                format!(
                    "\"{}\":{}",
                    json_escape(&column.name),
                    openapi_schema(&column.ty)
                )
            })
            .collect::<Vec<_>>();
        let required = table
            .columns
            .iter()
            .filter(|column| column.required && !matches!(column.ty, Type::Option(_)))
            .map(|column| format!("\"{}\"", json_escape(&column.name)))
            .collect::<Vec<_>>();
        let schema = format!(
            "{{\"type\":\"object\",\"properties\":{{{}}},\"required\":[{}]}}",
            properties.join(","),
            required.join(",")
        );
        components.push(format!(
            "\"{}\":{}",
            json_escape(&table.name),
            schema.clone()
        ));
        if let Some(singular) = singular_type_name(&table.name) {
            components.push(format!("\"{}\":{}", json_escape(&singular), schema));
        }
    }
    components.join(",")
}

fn format_typescript_client(program: &zelyra_ast::Program) -> String {
    let mut output = String::new();
    output.push_str("// Generated by Zelyra. Do not edit by hand.\n\n");
    output.push_str("export type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };\n\n");

    let mut error_codes = program
        .apis
        .iter()
        .flat_map(|api| api.errors.iter().map(|error| error.name.clone()))
        .collect::<Vec<_>>();
    error_codes.sort();
    error_codes.dedup();
    if error_codes.is_empty() {
        output.push_str("export type ZelyraApiErrorCode = string;\n\n");
    } else {
        let codes = error_codes
            .iter()
            .map(|code| format!("\"{}\"", json_escape(code)))
            .collect::<Vec<_>>()
            .join(" | ");
        writeln!(
            output,
            "export type ZelyraApiErrorCode = {} | (string & {{}});\n",
            codes
        )
        .expect("writing to a String cannot fail");
    }

    let mut emitted = HashSet::new();
    for definition in &program.types {
        if emitted.insert(definition.name.clone()) {
            writeln!(
                output,
                "export type {} = {};",
                definition.name,
                typescript_type(&definition.target)
            )
            .expect("writing to a String cannot fail");
        }
    }
    for record in &program.records {
        if emitted.insert(record.name.clone()) {
            writeln!(output, "export interface {} {{", record.name)
                .expect("writing to a String cannot fail");
            for field in &record.fields {
                let optional = matches!(field.ty, Type::Option(_));
                writeln!(
                    output,
                    "  {}{}: {};",
                    field.name,
                    if optional { "?" } else { "" },
                    typescript_type(&field.ty)
                )
                .expect("writing to a String cannot fail");
            }
            output.push_str("}\n\n");
        }
    }
    for table in &program.tables {
        let names = std::iter::once(table.name.clone()).chain(
            singular_type_name(&table.name)
                .into_iter()
                .filter(|name| name != &table.name),
        );
        for name in names {
            if emitted.insert(name.clone()) {
                writeln!(output, "export interface {} {{", name)
                    .expect("writing to a String cannot fail");
                for column in &table.columns {
                    let optional = !column.required || matches!(column.ty, Type::Option(_));
                    writeln!(
                        output,
                        "  {}{}: {};",
                        column.name,
                        if optional { "?" } else { "" },
                        typescript_type(&column.ty)
                    )
                    .expect("writing to a String cannot fail");
                }
                output.push_str("}\n\n");
            }
        }
    }

    let payload_types = program
        .apis
        .iter()
        .flat_map(|api| {
            api.errors.iter().filter_map(|error| {
                error
                    .payload
                    .as_ref()
                    .map(|payload| (error.name.clone(), typescript_type(payload)))
            })
        })
        .collect::<HashMap<_, _>>();
    if payload_types.is_empty() {
        output.push_str("export type ZelyraApiErrorPayload = JsonValue;\n\n");
    } else {
        output.push_str("export interface ZelyraApiErrorPayloads {\n");
        let mut payload_types = payload_types.into_iter().collect::<Vec<_>>();
        payload_types.sort_by(|left, right| left.0.cmp(&right.0));
        for (name, ty) in payload_types {
            writeln!(output, "  \"{name}\": {ty};").expect("writing to a String cannot fail");
        }
        output.push_str("}\n\nexport type ZelyraApiErrorPayload = ZelyraApiErrorPayloads[keyof ZelyraApiErrorPayloads];\n\n");
    }

    output.push_str(
        "export interface ZelyraClientOptions {\n  baseUrl: string;\n  fetch?: typeof fetch;\n  token?: string;\n}\n\n",
    );
    output.push_str(
        "export class ZelyraApiError extends Error {\n  constructor(\n    public readonly status: number,\n    public readonly code: ZelyraApiErrorCode | undefined,\n    public readonly body: string,\n    message: string,\n  ) {\n    super(message);\n  }\n\n  static async fromResponse(response: Response): Promise<ZelyraApiError> {\n    const body = await response.text();\n    let code: ZelyraApiErrorCode | undefined;\n    let message = `Zelyra API request failed (${response.status})`;\n    try {\n      const payload = JSON.parse(body) as { error?: { code?: unknown; message?: unknown } };\n      if (payload.error && typeof payload.error === \"object\") {\n        if (typeof payload.error.code === \"string\") code = payload.error.code;\n        if (typeof payload.error.message === \"string\") message = payload.error.message;\n      }\n    } catch {\n      // Keep the original response body when the server did not return JSON.
    }\n    return new ZelyraApiError(response.status, code, body, message);\n  }\n}\n\n",
    );
    output.push_str(
        "export class ZelyraClient {\n  private readonly baseUrl: string;\n  private readonly fetchImpl: typeof fetch;\n  private readonly token?: string;\n\n  constructor(options: ZelyraClientOptions) {\n    this.baseUrl = options.baseUrl.replace(/\\/$/, \"\");\n    this.fetchImpl = options.fetch ?? fetch;\n    this.token = options.token;\n  }\n\n",
    );

    for api in &program.apis {
        format_typescript_operation(&mut output, api);
    }
    output = output
        .replace(
            "export class ZelyraApiError extends Error {",
            "export class ZelyraApiError<Details = ZelyraApiErrorPayload> extends Error {",
        )
        .replace(
            "public readonly code: ZelyraApiErrorCode | undefined,\n    public readonly body: string,",
            "public readonly code: ZelyraApiErrorCode | undefined,\n    public readonly details: Details | undefined,\n    public readonly body: string,",
        )
        .replace(
            "static async fromResponse(response: Response): Promise<ZelyraApiError> {",
            "static async fromResponse<Details = ZelyraApiErrorPayload>(response: Response): Promise<ZelyraApiError<Details>> {",
        )
        .replace(
            "let code: ZelyraApiErrorCode | undefined;\n    let message",
            "let code: ZelyraApiErrorCode | undefined;\n    let details: Details | undefined;\n    let message",
        )
        .replace(
            "{ error?: { code?: unknown; message?: unknown } }",
            "{ error?: { code?: unknown; message?: unknown; details?: unknown } }",
        )
        .replace(
            "if (typeof payload.error.message === \"string\") message = payload.error.message;",
            "if (typeof payload.error.message === \"string\") message = payload.error.message;\n        details = payload.error.details as Details | undefined;",
        )
        .replace(
            "new ZelyraApiError(response.status, code, body, message)",
            "new ZelyraApiError(response.status, code, details, body, message)",
        );
    output.push_str("}\n");
    output
}

fn format_typescript_operation(output: &mut String, api: &zelyra_ast::ApiDef) {
    let method = api.method.to_ascii_uppercase();
    let operation = typescript_operation_name(api);
    let query_fields = if matches!(method.as_str(), "GET" | "DELETE") {
        api.input
            .iter()
            .filter(|field| !api.path.contains(&format!("{{{}}}", field.name)))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let body_fields = if matches!(method.as_str(), "GET" | "DELETE") {
        Vec::new()
    } else {
        api.input
            .iter()
            .filter(|field| !api.path.contains(&format!("{{{}}}", field.name)))
            .collect::<Vec<_>>()
    };
    let has_params = !api.input.is_empty();
    write!(output, "  async {}(", operation).expect("writing to a String cannot fail");
    if has_params {
        write!(output, "params: {{ ").expect("writing to a String cannot fail");
        for (index, field) in api.input.iter().enumerate() {
            if index > 0 {
                output.push(' ');
            }
            let optional = matches!(field.ty, Type::Option(_));
            write!(
                output,
                "{}{}: {};",
                field.name,
                if optional { "?" } else { "" },
                typescript_type(&field.ty)
            )
            .expect("writing to a String cannot fail");
        }
        output.push_str(" }");
    }
    writeln!(output, "): Promise<{}> {{", typescript_type(&api.output))
        .expect("writing to a String cannot fail");
    let path = typescript_path_template(&api.path);
    writeln!(output, "    let url = this.baseUrl + `{}`;", path)
        .expect("writing to a String cannot fail");
    if !query_fields.is_empty() {
        output.push_str("    const query = new URLSearchParams();\n");
        for field in query_fields {
            writeln!(
                output,
                "    if (params.{} !== undefined && params.{} !== null) query.set(\"{}\", String(params.{}));",
                field.name, field.name, field.name, field.name
            )
            .expect("writing to a String cannot fail");
        }
        output.push_str("    const queryString = query.toString();\n    if (queryString) url += `?${queryString}`;\n");
    }
    output.push_str("    const headers: Record<string, string> = {};\n    if (this.token) headers.Authorization = `Bearer ${this.token}`;\n");
    if !body_fields.is_empty() {
        output.push_str("    headers[\"Content-Type\"] = \"application/json\";\n");
    }
    writeln!(
        output,
        "    const response = await this.fetchImpl(url, {{ method: \"{}\", headers{} }});",
        method,
        if body_fields.is_empty() {
            String::new()
        } else {
            format!(
                ", body: JSON.stringify({{{}}})",
                body_fields
                    .iter()
                    .map(|field| format!("{}: params.{}", field.name, field.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    )
    .expect("writing to a String cannot fail");
    output.push_str("    if (!response.ok) throw await ZelyraApiError.fromResponse(response);\n");
    output.push_str("    return await response.json() as ");
    output.push_str(&typescript_type(&api.output));
    output.push_str(";\n  }\n\n");
}

fn typescript_type(ty: &Type) -> String {
    match ty {
        Type::Int | Type::UInt | Type::Float | Type::Decimal => "number".into(),
        Type::Bool => "boolean".into(),
        Type::String
        | Type::Char
        | Type::Bytes
        | Type::Timestamp
        | Type::Date
        | Type::Time
        | Type::Duration => "string".into(),
        Type::Unit => "void".into(),
        Type::Option(inner) => format!("{} | null", typescript_type(inner)),
        Type::Result(ok, _) => typescript_type(ok),
        Type::Array(inner) => format!("Array<{}>", typescript_type(inner)),
        Type::Map(_, value) => format!("Record<string, {}>", typescript_type(value)),
        Type::HttpResult(inner) => format!("HttpResult<{}>", typescript_type(inner)),
        Type::Named(name) => match name.as_str() {
            "Id" => "number".into(),
            "Email" | "Url" | "Uuid" | "Money" => "string".into(),
            "Unit" => "void".into(),
            _ => name.clone(),
        },
        Type::Unknown => "unknown".into(),
    }
}

fn typescript_operation_name(api: &zelyra_ast::ApiDef) -> String {
    let mut name = format!(
        "{}_{}",
        api.method.to_ascii_lowercase(),
        api.path.trim_matches('/').replace(['{', '}'], "")
    );
    name = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect();
    if name.ends_with('_') {
        name.pop();
    }
    if name == api.method.to_ascii_lowercase() {
        name.push_str("_root");
    }
    name
}

fn typescript_path_template(path: &str) -> String {
    let mut result = String::new();
    let mut rest = path;
    while let Some(start) = rest.find('{') {
        let (literal, after_start) = rest.split_at(start);
        result.push_str(&escape_typescript_template(literal));
        let Some(end) = after_start.find('}') else {
            result.push_str(&escape_typescript_template(after_start));
            return result;
        };
        let name = &after_start[1..end];
        result.push_str("${encodeURIComponent(String(params.");
        result.push_str(name);
        result.push_str("))}");
        rest = &after_start[end + 1..];
    }
    result.push_str(&escape_typescript_template(rest));
    result
}

fn escape_typescript_template(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${")
}

fn singular_type_name(table: &str) -> Option<String> {
    let singular = if let Some(stem) = table.strip_suffix("ies") {
        format!("{stem}y")
    } else if let Some(stem) = table.strip_suffix('s') {
        stem.to_owned()
    } else {
        table.to_owned()
    };
    let mut chars = singular.chars();
    let first = chars.next()?.to_ascii_uppercase();
    Some(std::iter::once(first).chain(chars).collect())
}

fn openapi_schema(ty: &Type) -> String {
    match ty {
        Type::Int | Type::UInt => "{\"type\":\"integer\"}".into(),
        Type::Float | Type::Decimal => "{\"type\":\"number\"}".into(),
        Type::Bool => "{\"type\":\"boolean\"}".into(),
        Type::Array(inner) => format!("{{\"type\":\"array\",\"items\":{}}}", openapi_schema(inner)),
        Type::Map(_, value) => format!(
            "{{\"type\":\"object\",\"additionalProperties\":{}}}",
            openapi_schema(value)
        ),
        Type::Option(inner) => openapi_schema(inner),
        Type::Result(ok, _) => openapi_schema(ok),
        Type::HttpResult(_) => "{\"type\":\"object\"}".into(),
        Type::Named(name) => match name.as_str() {
            "Id" => "{\"type\":\"integer\",\"format\":\"int64\"}".into(),
            "Email" => "{\"type\":\"string\",\"format\":\"email\"}".into(),
            "Url" => "{\"type\":\"string\",\"format\":\"uri\"}".into(),
            "Uuid" => "{\"type\":\"string\",\"format\":\"uuid\"}".into(),
            _ => format!(
                "{{\"$ref\":\"#/components/schemas/{}\"}}",
                json_escape(name)
            ),
        },
        _ => "{\"type\":\"string\"}".into(),
    }
}

fn openapi_error_schema(payload: &Type) -> String {
    let details = serde_json::from_str(&openapi_schema(payload))
        .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));
    serde_json::json!({
        "type": "object",
        "required": ["error"],
        "properties": {
            "error": {
                "type": "object",
                "required": ["code", "message", "details"],
                "properties": {
                    "code": {"type": "string"},
                    "message": {"type": "string"},
                    "details": details,
                }
            }
        }
    })
    .to_string()
}

fn format_verification_result(path: &str, source: &str, result: &VerificationResult) -> String {
    let (end_line, end_column) = source_position(source, result.span.end);
    let header = format!(
        "{} [{}]: {}.{}[{}] ({}:{}:{}-{}:{})\n  = {}",
        result.status,
        result.status.code(),
        result.function,
        result.kind,
        result.index,
        path,
        result.span.line,
        result.span.column,
        end_line,
        end_column,
        result.message
    );
    match format_source_excerpt(source, result.span) {
        Some(excerpt) => {
            let counterexample = result
                .counterexample
                .as_ref()
                .map(|values| format!("\n  = Counterexample: {}", format_counterexample(values)))
                .unwrap_or_default();
            format!("{header}{counterexample}\n{excerpt}")
        }
        None => match &result.counterexample {
            Some(values) => format!(
                "{header}\n  = Counterexample: {}",
                format_counterexample(values)
            ),
            None => header,
        },
    }
}

fn format_verification_json(path: &str, source: &str, results: &[VerificationResult]) -> String {
    let entries = results
        .iter()
        .map(|result| {
            let (end_line, end_column) = source_position(source, result.span.end);
            let counterexample = result
                .counterexample
                .as_ref()
                .map(|values| format_counterexample_json(values))
                .unwrap_or_else(|| "null".into());
            format!(
                "{{\"status\":\"{}\",\"code\":\"{}\",\"message\":\"{}\",\"function\":\"{}\",\"kind\":\"{}\",\"index\":{},\"counterexample\":{},\"location\":{{\"file\":\"{}\",\"start\":{{\"line\":{},\"column\":{}}},\"end\":{{\"line\":{},\"column\":{}}}}}}}",
                result.status,
                result.status.code(),
                json_escape(&result.message),
                json_escape(&result.function),
                result.kind,
                result.index,
                counterexample,
                json_escape(path),
                result.span.line,
                result.span.column,
                end_line,
                end_column
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", entries.join(","))
}

fn format_counterexample(values: &[(String, i64)]) -> String {
    values
        .iter()
        .map(|(name, value)| format!("{name} = {value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_counterexample_json(values: &[(String, i64)]) -> String {
    let entries = values
        .iter()
        .map(|(name, value)| format!("\"{}\":{}", json_escape(name), value))
        .collect::<Vec<_>>();
    format!("{{{}}}", entries.join(","))
}

fn json_escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            character if character.is_control() => {
                format!("\\u{:04x}", character as u32).chars().collect()
            }
            character => vec![character],
        })
        .collect()
}

fn format_source_excerpt(source: &str, span: zelyra_ast::Span) -> Option<String> {
    let source_line = source
        .split('\n')
        .nth(span.line.checked_sub(1)?)?
        .trim_end_matches('\r');
    let start = span.column.checked_sub(1)?.min(source_line.len());
    let (end_line, end_column) = source_position(source, span.end);
    let width = if end_line == span.line {
        end_column
            .saturating_sub(span.column)
            .max(1)
            .min(source_line.len().saturating_sub(start).max(1))
    } else {
        source_line.len().saturating_sub(start).max(1)
    };
    let line_number = span.line.to_string();
    let padding = " ".repeat(line_number.len());
    let marker = format!("{}{}", " ".repeat(start), "^".repeat(width));
    Some(format!(
        "  {padding} |\n  {line_number} | {source_line}\n  {padding} | {marker}"
    ))
}

fn source_position(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
    let end = offset.min(source.len());
    for byte in &source.as_bytes()[..end] {
        if *byte == b'\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

fn validate_capabilities(path: &str, program: &zelyra_ast::Program) -> Result<(), ()> {
    let grants = match project_capability_grants(path) {
        Ok(grants) => grants,
        Err(error) => {
            diagnostic(path, "E-CAP-002", &error, 1, 1);
            return Err(());
        }
    };
    if let Err(errors) = check_capabilities_with_grants(program, grants.as_ref()) {
        for error in errors {
            diagnostic_with_span(path, "E-CAP-001", &error.message, error.span);
        }
        return Err(());
    }
    if grants
        .as_ref()
        .is_some_and(|grants| !grants.contains("Database"))
        && program.pages.iter().any(|page| !page.data.is_empty())
    {
        if let Some(data) = program.pages.iter().flat_map(|page| &page.data).next() {
            diagnostic(
                path,
                "E-CAP-001",
                "page data loading requires the `Database` capability",
                data.span.line,
                data.span.column,
            );
        }
        return Err(());
    }
    Ok(())
}

fn project_config_path(path: &str) -> Result<Option<PathBuf>, String> {
    let source_path =
        fs::canonicalize(path).map_err(|error| format!("cannot locate source: {error}"))?;
    let mut directory = source_path
        .parent()
        .ok_or_else(|| "source has no parent directory".to_owned())?;
    Ok(loop {
        let candidate = directory.join("zelyra.toml");
        if candidate.is_file() {
            break Some(candidate);
        }
        let Some(parent) = directory.parent() else {
            break None;
        };
        if parent == directory {
            break None;
        }
        directory = parent;
    })
}

const PROJECT_FEATURES: [&str; 5] = ["web", "api", "crud", "auth", "audit"];

#[derive(Clone, Debug, PartialEq, Eq)]
struct FeatureSetting {
    enabled: bool,
    source: String,
}

type ProjectFeatures = BTreeMap<String, FeatureSetting>;

fn parse_bool_setting(value: &str, setting: &str) -> Result<bool, String> {
    match value.trim().trim_matches('"') {
        "true" => Ok(true),
        "false" => Ok(false),
        value => Err(format!(
            "feature setting `{setting}` must be true or false, found `{value}`"
        )),
    }
}

fn parse_feature_section(contents: &str) -> Result<BTreeMap<String, bool>, String> {
    let mut values = BTreeMap::new();
    let mut in_features = false;
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_features = line == "[features]";
            continue;
        }
        if !in_features {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(format!(
                "invalid feature setting on line {}",
                line_index + 1
            ));
        };
        let key = raw_key.trim().to_ascii_lowercase();
        if !PROJECT_FEATURES.contains(&key.as_str()) {
            return Err(format!("unknown feature setting `{key}`"));
        }
        if values.contains_key(&key) {
            return Err(format!(
                "feature setting `{key}` is configured more than once"
            ));
        }
        values.insert(key.clone(), parse_bool_setting(raw_value, &key)?);
    }
    Ok(values)
}

fn parse_env_feature_overrides(contents: &str) -> Result<BTreeMap<String, bool>, String> {
    let mut values = BTreeMap::new();
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = raw_key.trim();
        let Some(feature) = key.strip_prefix("ZELYRA_FEATURE_") else {
            continue;
        };
        let feature = feature.to_ascii_lowercase();
        if !PROJECT_FEATURES.contains(&feature.as_str()) {
            return Err(format!(
                "unknown ZELYRA_FEATURE_ setting `{}` on line {}",
                feature,
                line_index + 1
            ));
        }
        if values.contains_key(&feature) {
            return Err(format!(
                "environment feature `{feature}` is configured more than once"
            ));
        }
        values.insert(
            feature.clone(),
            parse_bool_setting(
                raw_value.split('#').next().unwrap_or(raw_value),
                &format!("ZELYRA_FEATURE_{}", feature.to_ascii_uppercase()),
            )?,
        );
    }
    Ok(values)
}

fn feature_defaults() -> ProjectFeatures {
    PROJECT_FEATURES
        .into_iter()
        .map(|feature| {
            (
                feature.to_owned(),
                FeatureSetting {
                    enabled: true,
                    source: "default".into(),
                },
            )
        })
        .collect()
}

fn apply_feature_values(
    features: &mut ProjectFeatures,
    values: BTreeMap<String, bool>,
    source: &str,
) {
    for (feature, enabled) in values {
        if let Some(setting) = features.get_mut(&feature) {
            setting.enabled = enabled;
            setting.source = source.to_owned();
        }
    }
}

fn project_features(path: &str) -> Result<ProjectFeatures, String> {
    let mut features = feature_defaults();
    let config_path = project_config_path(path)?;
    if let Some(config_path) = &config_path {
        let contents = fs::read_to_string(config_path)
            .map_err(|error| format!("cannot read {}: {error}", config_path.display()))?;
        apply_feature_values(
            &mut features,
            parse_feature_section(&contents)?,
            "zelyra.toml",
        );

        let env_path = config_path
            .parent()
            .ok_or_else(|| "project configuration has no parent directory".to_owned())?
            .join(".env");
        if env_path.is_file() {
            let env_contents = fs::read_to_string(&env_path)
                .map_err(|error| format!("cannot read {}: {error}", env_path.display()))?;
            apply_feature_values(
                &mut features,
                parse_env_feature_overrides(&env_contents)?,
                ".env",
            );
        }
    }
    for feature in PROJECT_FEATURES {
        let variable = format!("ZELYRA_FEATURE_{}", feature.to_ascii_uppercase());
        if let Ok(value) = env::var(&variable) {
            let mut override_value = BTreeMap::new();
            override_value.insert(feature.to_owned(), parse_bool_setting(&value, &variable)?);
            apply_feature_values(&mut features, override_value, "environment");
        }
    }
    Ok(features)
}

fn feature_enabled(features: &ProjectFeatures, feature: &str) -> bool {
    features.get(feature).is_none_or(|setting| setting.enabled)
}

fn validate_project_features(path: &str, program: &zelyra_ast::Program) -> Result<(), ()> {
    let features = match project_features(path) {
        Ok(features) => features,
        Err(error) => {
            diagnostic(path, "E-FEATURE-002", &error, 1, 1);
            return Err(());
        }
    };
    let web_used = !program.pages.is_empty()
        || !program.forms.is_empty()
        || !program.tableviews.is_empty()
        || !program.cruds.is_empty()
        || !program.apis.is_empty()
        || !program.auth.is_empty();
    if !feature_enabled(&features, "web") && web_used {
        diagnostic(
            path,
            "E-FEATURE-001",
            "the `web` feature is disabled, but this program declares web resources",
            1,
            1,
        );
        return Err(());
    }
    if !feature_enabled(&features, "api") && !program.apis.is_empty() {
        diagnostic(
            path,
            "E-FEATURE-001",
            "the `api` feature is disabled, but this program declares an API",
            1,
            1,
        );
        return Err(());
    }
    if !feature_enabled(&features, "crud") && !program.cruds.is_empty() {
        diagnostic(
            path,
            "E-FEATURE-001",
            "the `crud` feature is disabled, but this program declares CRUD resources",
            1,
            1,
        );
        return Err(());
    }
    if !feature_enabled(&features, "auth") && !program.auth.is_empty() {
        diagnostic(
            path,
            "E-FEATURE-001",
            "the `auth` feature is disabled, but this program declares authentication",
            1,
            1,
        );
        return Err(());
    }
    if !feature_enabled(&features, "audit")
        && program
            .auth
            .iter()
            .any(|auth| auth.audit_table.is_some() || auth.audit_chain)
    {
        diagnostic(
            path,
            "E-FEATURE-001",
            "the `audit` feature is disabled, but audit logging is configured",
            1,
            1,
        );
        return Err(());
    }
    Ok(())
}

fn project_capability_grants(path: &str) -> Result<Option<HashSet<String>>, String> {
    let Some(config_path) = project_config_path(path)? else {
        return Ok(None);
    };
    let contents = fs::read_to_string(&config_path)
        .map_err(|error| format!("cannot read {}: {error}", config_path.display()))?;
    let mut grants = HashSet::new();
    let mut seen = HashSet::new();
    let mut in_capabilities = false;
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_capabilities = line == "[capabilities]";
            continue;
        }
        if !in_capabilities {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(format!(
                "invalid capability setting on line {}",
                line_index + 1
            ));
        };
        let key = raw_key.trim().to_ascii_lowercase();
        let capability = KNOWN_CAPABILITIES
            .iter()
            .copied()
            .find(|capability| capability.to_ascii_lowercase() == key)
            .ok_or_else(|| format!("unknown capability setting `{key}`"))?;
        if !seen.insert(capability) {
            return Err(format!("capability `{key}` is configured more than once"));
        }
        match raw_value.trim() {
            "true" => {
                grants.insert(capability.to_owned());
            }
            "false" => {}
            value => {
                return Err(format!(
                    "capability `{key}` must be true or false, found `{value}`"
                ));
            }
        }
    }
    Ok(Some(grants))
}

fn parse_string_array(value: &str) -> Result<Vec<String>, String> {
    let value = value.trim();
    if !value.starts_with('[') || !value.ends_with(']') {
        return Err("value must be a TOML string array".into());
    }
    let inner = value[1..value.len() - 1].trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(',')
        .map(|item| {
            let item = item.trim();
            let Some(item) = item
                .strip_prefix('"')
                .and_then(|item| item.strip_suffix('"'))
            else {
                return Err("string arrays must contain quoted strings".into());
            };
            Ok(item.replace("\\\\", "\\").replace("\\\"", "\""))
        })
        .collect()
}

fn project_filesystem_policy(path: &str) -> Result<Option<FileSystemPolicy>, String> {
    let Some(config_path) = project_config_path(path)? else {
        return Ok(None);
    };
    let contents = fs::read_to_string(&config_path)
        .map_err(|error| format!("cannot read {}: {error}", config_path.display()))?;
    let mut read_roots = None;
    let mut write_roots = None;
    let mut seen = HashSet::new();
    let mut in_filesystem = false;
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_filesystem = line == "[filesystem]";
            continue;
        }
        if !in_filesystem {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(format!(
                "invalid filesystem setting on line {}",
                line_index + 1
            ));
        };
        let key = raw_key.trim();
        if !seen.insert(key) {
            return Err(format!(
                "filesystem setting {key} is configured more than once"
            ));
        }
        match key {
            "read_roots" => read_roots = Some(parse_string_array(raw_value)?),
            "write_roots" => write_roots = Some(parse_string_array(raw_value)?),
            _ => return Err(format!("unknown filesystem setting {key}")),
        }
    }
    let base_dir = config_path
        .parent()
        .ok_or_else(|| "project configuration has no parent directory".to_owned())?
        .to_path_buf();
    let canonical_root = |root: &str| {
        let candidate = if PathBuf::from(root).is_absolute() {
            PathBuf::from(root)
        } else {
            base_dir.join(root)
        };
        let canonical = fs::canonicalize(&candidate).map_err(|error| {
            format!(
                "filesystem root {} is not accessible: {error}",
                candidate.display()
            )
        })?;
        if !canonical.is_dir() {
            return Err(format!(
                "filesystem root {} is not a directory",
                candidate.display()
            ));
        }
        Ok(canonical)
    };
    let read_roots = read_roots
        .unwrap_or_else(|| vec![".".into()])
        .iter()
        .map(String::as_str)
        .map(canonical_root)
        .collect::<Result<Vec<_>, _>>()?;
    let write_roots = write_roots
        .unwrap_or_default()
        .iter()
        .map(String::as_str)
        .map(canonical_root)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(FileSystemPolicy {
        base_dir,
        read_roots,
        write_roots,
    }))
}

fn project_network_policy(path: &str) -> Result<Option<NetworkPolicy>, String> {
    let Some(config_path) = project_config_path(path)? else {
        return Ok(None);
    };
    let contents = fs::read_to_string(&config_path)
        .map_err(|error| format!("cannot read {}: {error}", config_path.display()))?;
    let mut allowed_hosts = None;
    let mut timeout_ms = 5_000;
    let mut max_response_bytes = 1_048_576;
    let mut seen = HashSet::new();
    let mut in_network = false;
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_network = line == "[network]";
            continue;
        }
        if !in_network {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(format!(
                "invalid network setting on line {}",
                line_index + 1
            ));
        };
        let key = raw_key.trim();
        if !seen.insert(key) {
            return Err(format!(
                "network setting {key} is configured more than once"
            ));
        }
        match key {
            "allowed_hosts" => allowed_hosts = Some(parse_string_array(raw_value)?),
            "timeout_ms" => {
                timeout_ms = raw_value.trim().parse().map_err(|_| {
                    "network setting timeout_ms must be a positive integer".to_owned()
                })?;
                if timeout_ms == 0 {
                    return Err("network setting timeout_ms must be positive".into());
                }
            }
            "max_response_bytes" => {
                max_response_bytes = raw_value.trim().parse().map_err(|_| {
                    "network setting max_response_bytes must be a positive integer".to_owned()
                })?;
                if max_response_bytes == 0 {
                    return Err("network setting max_response_bytes must be positive".into());
                }
            }
            _ => return Err(format!("unknown network setting {key}")),
        }
    }
    Ok(Some(NetworkPolicy {
        allowed_hosts: allowed_hosts.unwrap_or_default(),
        timeout_ms,
        max_response_bytes,
    }))
}

fn project_runtime_policy(path: &str) -> Result<Option<RuntimePolicy>, String> {
    let filesystem = project_filesystem_policy(path)?;
    let network = project_network_policy(path)?;
    let process = project_process_policy(path)?;
    if filesystem.is_none() && network.is_none() && process.is_none() {
        Ok(None)
    } else {
        Ok(Some(RuntimePolicy {
            filesystem,
            network,
            process,
        }))
    }
}

fn project_cors_policy(path: &str) -> Result<Option<CorsPolicy>, String> {
    let Some(config_path) = project_config_path(path)? else {
        return Ok(None);
    };
    let contents = fs::read_to_string(&config_path)
        .map_err(|error| format!("cannot read {}: {error}", config_path.display()))?;
    let mut allowed_origins = None;
    let mut allow_credentials = false;
    let mut seen = HashSet::new();
    let mut in_web = false;
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_web = line == "[web]";
            continue;
        }
        if !in_web {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(format!("invalid web setting on line {}", line_index + 1));
        };
        let key = raw_key.trim();
        if !seen.insert(key) {
            return Err(format!("web setting {key} is configured more than once"));
        }
        match key {
            "allowed_origins" => allowed_origins = Some(parse_string_array(raw_value)?),
            "allow_credentials" => {
                allow_credentials = match raw_value.trim() {
                    "true" => true,
                    "false" => false,
                    value => {
                        return Err(format!(
                            "web setting allow_credentials must be true or false, found `{value}`"
                        ));
                    }
                };
            }
            _ => return Err(format!("unknown web setting {key}")),
        }
    }
    let Some(allowed_origins) = allowed_origins else {
        return Ok(None);
    };
    CorsPolicy::new(allowed_origins, allow_credentials)
        .map(Some)
        .map_err(|error| error.message)
}

fn project_process_policy(path: &str) -> Result<Option<ProcessPolicy>, String> {
    let Some(config_path) = project_config_path(path)? else {
        return Ok(None);
    };
    let contents = fs::read_to_string(&config_path)
        .map_err(|error| format!("cannot read {}: {error}", config_path.display()))?;
    let mut allowed_commands = None;
    let mut timeout_ms = 5_000;
    let mut max_output_bytes = 1_048_576;
    let mut seen = HashSet::new();
    let mut in_process = false;
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_process = line == "[process]";
            continue;
        }
        if !in_process {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(format!(
                "invalid process setting on line {}",
                line_index + 1
            ));
        };
        let key = raw_key.trim();
        if !seen.insert(key) {
            return Err(format!(
                "process setting {key} is configured more than once"
            ));
        }
        match key {
            "allowed_commands" => allowed_commands = Some(parse_string_array(raw_value)?),
            "timeout_ms" => {
                timeout_ms = raw_value.trim().parse().map_err(|_| {
                    "process setting timeout_ms must be a positive integer".to_owned()
                })?;
                if timeout_ms == 0 {
                    return Err("process setting timeout_ms must be positive".into());
                }
            }
            "max_output_bytes" => {
                max_output_bytes = raw_value.trim().parse().map_err(|_| {
                    "process setting max_output_bytes must be a positive integer".to_owned()
                })?;
                if max_output_bytes == 0 {
                    return Err("process setting max_output_bytes must be positive".into());
                }
            }
            _ => return Err(format!("unknown process setting {key}")),
        }
    }
    Ok(Some(ProcessPolicy {
        allowed_commands: allowed_commands.unwrap_or_default(),
        timeout_ms,
        max_output_bytes,
    }))
}

fn validate_auth(path: &str, program: &zelyra_ast::Program, schema: &Schema) -> bool {
    let mut valid = true;
    let mut names = HashSet::new();
    for auth in &program.auth {
        if !names.insert(auth.name.clone()) {
            diagnostic(
                path,
                "E-AUTH-002",
                &format!("duplicate authentication definition {}", auth.name),
                auth.span.line,
                auth.span.column,
            );
            valid = false;
        }
        if !schema.tables.iter().any(|table| table.name == auth.table) {
            diagnostic(
                path,
                "E-AUTH-001",
                &format!("authentication refers to unknown user table {}", auth.table),
                auth.span.line,
                auth.span.column,
            );
            valid = false;
            continue;
        }
        let Some(table) = schema.tables.iter().find(|table| table.name == auth.table) else {
            continue;
        };
        for required_column in ["id", "email", "password_hash"] {
            if !table
                .columns
                .iter()
                .any(|column| column.name == required_column)
            {
                diagnostic(
                    path,
                    "E-AUTH-004",
                    &format!(
                        "authentication table {} requires column {}",
                        auth.table, required_column
                    ),
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
            }
        }
        if let Some(session_table_name) = &auth.session_table {
            let Some(session_table) = schema
                .tables
                .iter()
                .find(|candidate| candidate.name == *session_table_name)
            else {
                diagnostic(
                    path,
                    "E-AUTH-005",
                    &format!(
                        "authentication refers to unknown session table {}",
                        session_table_name
                    ),
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
                continue;
            };
            for required_column in ["user_id", "token_hash", "expires_at"] {
                if !session_table
                    .columns
                    .iter()
                    .any(|column| column.name == required_column)
                {
                    diagnostic(
                        path,
                        "E-AUTH-006",
                        &format!(
                            "authentication session table {} requires column {}",
                            session_table_name, required_column
                        ),
                        auth.span.line,
                        auth.span.column,
                    );
                    valid = false;
                }
            }
        }
        if let Some(permissions_table_name) = &auth.permissions_table {
            let Some(permissions_table) = schema
                .tables
                .iter()
                .find(|candidate| candidate.name == *permissions_table_name)
            else {
                diagnostic(
                    path,
                    "E-AUTH-007",
                    &format!(
                        "authentication refers to unknown permissions table {}",
                        permissions_table_name
                    ),
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
                continue;
            };
            for required_column in ["user_id", "permission"] {
                if !permissions_table
                    .columns
                    .iter()
                    .any(|column| column.name == required_column)
                {
                    diagnostic(
                        path,
                        "E-AUTH-008",
                        &format!(
                            "authentication permissions table {} requires column {}",
                            permissions_table_name, required_column
                        ),
                        auth.span.line,
                        auth.span.column,
                    );
                    valid = false;
                }
            }
        }
        if auth.roles_table.is_some() != auth.role_permissions_table.is_some() {
            diagnostic(
                path,
                "E-AUTH-009",
                "authentication roles require both roles and role_permissions options",
                auth.span.line,
                auth.span.column,
            );
            valid = false;
        }
        if let Some(roles_table_name) = &auth.roles_table {
            let Some(roles_table) = schema
                .tables
                .iter()
                .find(|candidate| candidate.name == *roles_table_name)
            else {
                diagnostic(
                    path,
                    "E-AUTH-010",
                    &format!(
                        "authentication refers to unknown roles table {}",
                        roles_table_name
                    ),
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
                continue;
            };
            for required_column in ["user_id", "role"] {
                if !roles_table
                    .columns
                    .iter()
                    .any(|column| column.name == required_column)
                {
                    diagnostic(
                        path,
                        "E-AUTH-012",
                        &format!(
                            "authentication roles table {} requires column {}",
                            roles_table_name, required_column
                        ),
                        auth.span.line,
                        auth.span.column,
                    );
                    valid = false;
                }
            }
        }
        if let Some(role_permissions_table_name) = &auth.role_permissions_table {
            let Some(role_permissions_table) = schema
                .tables
                .iter()
                .find(|candidate| candidate.name == *role_permissions_table_name)
            else {
                diagnostic(
                    path,
                    "E-AUTH-011",
                    &format!(
                        "authentication refers to unknown role permissions table {}",
                        role_permissions_table_name
                    ),
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
                continue;
            };
            for required_column in ["role", "permission"] {
                if !role_permissions_table
                    .columns
                    .iter()
                    .any(|column| column.name == required_column)
                {
                    diagnostic(
                        path,
                        "E-AUTH-013",
                        &format!(
                            "authentication role permissions table {} requires column {}",
                            role_permissions_table_name, required_column
                        ),
                        auth.span.line,
                        auth.span.column,
                    );
                    valid = false;
                }
            }
        }
        if let Some(audit_table_name) = &auth.audit_table {
            let Some(audit_table) = schema
                .tables
                .iter()
                .find(|candidate| candidate.name == *audit_table_name)
            else {
                diagnostic(
                    path,
                    "E-AUTH-025",
                    &format!(
                        "authentication refers to unknown audit table {}",
                        audit_table_name
                    ),
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
                continue;
            };
            for required_column in [
                "actor_user_id",
                "event",
                "target_user_id",
                "details",
                "created_at",
            ] {
                if !audit_table
                    .columns
                    .iter()
                    .any(|column| column.name == required_column)
                {
                    diagnostic(
                        path,
                        "E-AUTH-026",
                        &format!(
                            "authentication audit table {} requires column {}",
                            audit_table_name, required_column
                        ),
                        auth.span.line,
                        auth.span.column,
                    );
                    valid = false;
                }
            }
            if auth.audit_chain {
                for required_column in ["id", "previous_hash", "entry_hash"] {
                    if !audit_table
                        .columns
                        .iter()
                        .any(|column| column.name == required_column)
                    {
                        diagnostic(
                            path,
                            "E-AUTH-027",
                            &format!(
                                "chained authentication audit table {} requires column {}",
                                audit_table_name, required_column
                            ),
                            auth.span.line,
                            auth.span.column,
                        );
                        valid = false;
                    }
                }
            }
        } else if auth.audit_chain {
            diagnostic(
                path,
                "E-AUTH-028",
                "audit_chain requires an audit: <table> option",
                auth.span.line,
                auth.span.column,
            );
            valid = false;
        }
        let admin_options = [
            auth.admin_path.is_some(),
            auth.admin_permission.is_some(),
            auth.admin_role.is_some(),
        ];
        if admin_options.iter().any(|configured| *configured)
            && !admin_options.iter().all(|configured| *configured)
        {
            diagnostic(
                path,
                "E-AUTH-021",
                "authentication administration requires admin_path, admin_permission, and admin_role",
                auth.span.line,
                auth.span.column,
            );
            valid = false;
        }
        if let Some(admin_path) = &auth.admin_path {
            if !admin_path.starts_with('/') || admin_path == "/login" || admin_path == "/logout" {
                diagnostic(
                    path,
                    "E-AUTH-022",
                    "authentication admin_path must be an application path other than /login or /logout",
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
            }
        }
        if let Some(admin_permission) = &auth.admin_permission {
            if admin_permission.is_empty() {
                diagnostic(
                    path,
                    "E-AUTH-023",
                    "authentication admin_permission must not be empty",
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
            }
        }
        if let Some(admin_role) = &auth.admin_role {
            if admin_role.is_empty() {
                diagnostic(
                    path,
                    "E-AUTH-024",
                    "authentication admin_role must not be empty",
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
            }
        }
    }
    let protected = program
        .pages
        .iter()
        .any(|page| page.requires_auth || !page.permissions.is_empty())
        || program.forms.iter().any(|form| {
            form.actions
                .iter()
                .any(|action| action.requires_auth || !action.permissions.is_empty())
        })
        || program.cruds.iter().any(|crud| {
            crud.requires_auth
                || !crud.permissions.is_empty()
                || !crud.create_permissions.is_empty()
                || !crud.edit_permissions.is_empty()
                || !crud.delete_permissions.is_empty()
        })
        || program
            .tableviews
            .iter()
            .any(|tableview| tableview.requires_auth || !tableview.permissions.is_empty())
        || program
            .apis
            .iter()
            .any(|api| api.requires_auth || !api.permissions.is_empty());
    if protected && program.auth.is_empty() {
        diagnostic(
            path,
            "E-AUTH-003",
            "protected routes require an auth definition",
            1,
            1,
        );
        valid = false;
    }
    valid
}

fn validate_cruds(path: &str, program: &zelyra_ast::Program, schema: &Schema) -> bool {
    let mut valid = true;
    let mut names = HashSet::new();
    let mut tables = HashSet::new();
    for crud in &program.cruds {
        if !names.insert(crud.name.clone()) {
            diagnostic(
                path,
                "E-CRUD-002",
                &format!("duplicate CRUD resource `{}`", crud.name),
                crud.span.line,
                crud.span.column,
            );
            valid = false;
        }
        if !tables.insert(crud.table.clone()) {
            diagnostic(
                path,
                "E-CRUD-003",
                &format!("table `{}` already has a CRUD resource", crud.table),
                crud.span.line,
                crud.span.column,
            );
            valid = false;
        }
        if !schema.tables.iter().any(|table| table.name == crud.table) {
            diagnostic(
                path,
                "E-CRUD-001",
                &format!("CRUD resource refers to unknown table `{}`", crud.table),
                crud.span.line,
                crud.span.column,
            );
            valid = false;
            continue;
        }
        let configured_columns = crud
            .view
            .fields
            .iter()
            .chain(&crud.list)
            .chain(&crud.search)
            .chain(&crud.filters);
        for column in configured_columns {
            if !crud_column_exists(program, schema, crud, column) {
                diagnostic(
                    path,
                    "E-CRUD-004",
                    &format!(
                        "CRUD column {column} does not exist in table {}",
                        crud.table
                    ),
                    crud.span.line,
                    crud.span.column,
                );
                valid = false;
            }
        }
        if let Some(soft_delete) = &crud.soft_delete {
            let Some(column) = schema
                .tables
                .iter()
                .find(|table| table.name == crud.table)
                .and_then(|table| {
                    table
                        .columns
                        .iter()
                        .find(|column| column.name == soft_delete.column)
                })
            else {
                diagnostic(
                    path,
                    "E-CRUD-005",
                    &format!(
                        "soft_delete column `{}` does not exist in table {}",
                        soft_delete.column, crud.table
                    ),
                    crud.span.line,
                    crud.span.column,
                );
                valid = false;
                continue;
            };
            let sql_type = column.sql_type.to_ascii_uppercase();
            if !sql_type.contains("TIMESTAMP") && !sql_type.contains("DATETIME") {
                diagnostic(
                    path,
                    "E-CRUD-006",
                    &format!(
                        "soft_delete column `{}` must use a timestamp-compatible type",
                        soft_delete.column
                    ),
                    crud.span.line,
                    crud.span.column,
                );
                valid = false;
            }
        }
    }
    valid
}

fn crud_column_exists(
    program: &zelyra_ast::Program,
    schema: &Schema,
    crud: &zelyra_ast::CrudDef,
    column: &str,
) -> bool {
    let Some(table) = program.tables.iter().find(|table| table.name == crud.table) else {
        return false;
    };
    let logical_exists = table
        .columns
        .iter()
        .any(|candidate| candidate.name == column);
    if !logical_exists
        && !schema.tables.iter().any(|table| {
            table.name == crud.table
                && table
                    .columns
                    .iter()
                    .any(|candidate| candidate.name == column)
        })
    {
        return false;
    }
    let storage = storage_column_name(schema, &crud.table, column);
    schema
        .tables
        .iter()
        .find(|table| table.name == crud.table)
        .is_some_and(|table| {
            table
                .columns
                .iter()
                .any(|candidate| candidate.name == storage)
        })
}

fn validate_tableviews(path: &str, program: &zelyra_ast::Program, schema: &Schema) -> bool {
    let mut valid = true;
    let mut names = HashSet::new();
    for tableview in &program.tableviews {
        if !names.insert(tableview.name.clone()) {
            diagnostic(
                path,
                "E-VIEW-004",
                &format!("duplicate tableview {}", tableview.name),
                tableview.span.line,
                tableview.span.column,
            );
            valid = false;
        }
        let Some(result_fields) =
            tableview_result_fields(&tableview.result_type, schema, &program.records)
        else {
            diagnostic(
                path,
                "E-VIEW-005",
                &format!(
                    "tableview {} requires a result type that maps to a declared table",
                    tableview.name
                ),
                tableview.span.line,
                tableview.span.column,
            );
            valid = false;
            continue;
        };
        let mut columns = HashSet::new();
        for column in &tableview.columns {
            if !columns.insert(column.as_str()) {
                diagnostic(
                    path,
                    "E-VIEW-006",
                    &format!(
                        "tableview {} contains column {} more than once",
                        tableview.name, column
                    ),
                    tableview.span.line,
                    tableview.span.column,
                );
                valid = false;
            } else if !result_fields.iter().any(|candidate| *candidate == column) {
                diagnostic(
                    path,
                    "E-VIEW-007",
                    &format!(
                        "tableview column {} does not exist in its result type",
                        column
                    ),
                    tableview.span.line,
                    tableview.span.column,
                );
                valid = false;
            }
        }
        let mut filters = HashSet::new();
        for column in &tableview.filters {
            if !filters.insert(column.as_str()) {
                diagnostic(
                    path,
                    "E-VIEW-008",
                    &format!(
                        "tableview {} contains filter column {} more than once",
                        tableview.name, column
                    ),
                    tableview.span.line,
                    tableview.span.column,
                );
                valid = false;
            } else if !result_fields.iter().any(|candidate| *candidate == column) {
                diagnostic(
                    path,
                    "E-VIEW-009",
                    &format!(
                        "tableview filter column {} does not exist in its result type",
                        column
                    ),
                    tableview.span.line,
                    tableview.span.column,
                );
                valid = false;
            }
        }
    }
    valid
}

fn tableview_result_fields<'a>(
    result_type: &zelyra_ast::Type,
    schema: &'a Schema,
    records: &'a [zelyra_ast::RecordDef],
) -> Option<Vec<&'a str>> {
    let result_type = match result_type {
        zelyra_ast::Type::Array(inner) | zelyra_ast::Type::Option(inner) => inner,
        _ => result_type,
    };
    let zelyra_ast::Type::Named(name) = result_type else {
        return None;
    };
    let snake = name.to_ascii_lowercase();
    if let Some(table) = schema
        .tables
        .iter()
        .find(|table| table.name == snake)
        .or_else(|| {
            let plural = if snake.ends_with('y') {
                format!("{}ies", &snake[..snake.len() - 1])
            } else {
                format!("{snake}s")
            };
            schema.tables.iter().find(|table| table.name == plural)
        })
    {
        return Some(
            table
                .columns
                .iter()
                .map(|column| column.name.as_str())
                .collect(),
        );
    }
    records
        .iter()
        .find(|record| record.name == *name)
        .map(|record| {
            record
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect()
        })
}

fn tableview_filter_kind(
    result_type: &Type,
    column: &str,
    schema: &Schema,
    records: &[zelyra_ast::RecordDef],
) -> TableViewFilterKind {
    let result_type = match result_type {
        Type::Array(inner) | Type::Option(inner) => inner,
        _ => result_type,
    };
    let Type::Named(name) = result_type else {
        return TableViewFilterKind::Other;
    };
    let snake = name.to_ascii_lowercase();
    let table = schema
        .tables
        .iter()
        .find(|table| table.name == snake)
        .or_else(|| {
            let plural = if snake.ends_with('y') {
                format!("{}ies", &snake[..snake.len() - 1])
            } else {
                format!("{snake}s")
            };
            schema.tables.iter().find(|table| table.name == plural)
        });
    if let Some(table) = table {
        if let Some(schema_column) = table
            .columns
            .iter()
            .find(|candidate| candidate.name == column)
        {
            return tableview_sql_filter_kind(&schema_column.sql_type);
        }
    }
    records
        .iter()
        .find(|record| record.name == *name)
        .and_then(|record| record.fields.iter().find(|field| field.name == column))
        .map(|field| tableview_type_filter_kind(&field.ty))
        .unwrap_or(TableViewFilterKind::Other)
}

fn tableview_type_filter_kind(ty: &Type) -> TableViewFilterKind {
    let ty = match ty {
        Type::Option(inner) => inner.as_ref(),
        _ => ty,
    };
    match ty {
        Type::Int | Type::UInt | Type::Float | Type::Decimal => TableViewFilterKind::Numeric,
        Type::Bool => TableViewFilterKind::Bool,
        Type::String | Type::Char => TableViewFilterKind::Text,
        Type::Named(name) if matches!(name.as_str(), "Email" | "Url" | "Uuid") => {
            TableViewFilterKind::Text
        }
        _ => TableViewFilterKind::Other,
    }
}

fn tableview_sql_filter_kind(sql_type: &str) -> TableViewFilterKind {
    let sql_type = sql_type.to_ascii_uppercase();
    if sql_type.contains("BOOL") {
        TableViewFilterKind::Bool
    } else if sql_type.contains("CHAR") || sql_type.contains("TEXT") {
        TableViewFilterKind::Text
    } else if sql_type.contains("INT")
        || sql_type.contains("DECIMAL")
        || sql_type.contains("NUMERIC")
        || sql_type.contains("DOUBLE")
        || sql_type.contains("FLOAT")
        || sql_type.contains("REAL")
    {
        TableViewFilterKind::Numeric
    } else {
        TableViewFilterKind::Other
    }
}

fn configured_crud_columns(
    program: &zelyra_ast::Program,
    schema: &Schema,
    crud: &zelyra_ast::CrudDef,
    configured: &[String],
    default: impl FnOnce(&zelyra_database::Table) -> Vec<String>,
) -> Vec<String> {
    let table = schema
        .tables
        .iter()
        .find(|table| table.name == crud.table)
        .expect("CRUD table was validated before route generation");
    if configured.is_empty() {
        return default(table);
    }
    configured
        .iter()
        .map(|column| storage_column_name(schema, &crud.table, column))
        .filter(|column| crud_column_exists(program, schema, crud, column))
        .collect()
}

fn configured_crud_filter_columns(
    program: &zelyra_ast::Program,
    schema: &Schema,
    crud: &zelyra_ast::CrudDef,
    configured: &[String],
    default: impl FnOnce(&zelyra_database::Table) -> Vec<String>,
) -> Vec<String> {
    let table = schema
        .tables
        .iter()
        .find(|table| table.name == crud.table)
        .expect("CRUD table was validated before route generation");
    if configured.is_empty() {
        return default(table);
    }
    configured
        .iter()
        .filter(|column| crud_column_exists(program, schema, crud, column))
        .cloned()
        .collect()
}

fn load_schema(path: &str) -> Result<Schema, ()> {
    let project = load_project(path)?;
    match build_schema(&project.program) {
        Ok(schema) => Ok(schema),
        Err(errors) => {
            for error in errors {
                let source_path = PROJECT_SOURCES.with(|sources| {
                    sources
                        .borrow()
                        .get(error.span.source_id as usize)
                        .map_or_else(|| path.to_owned(), |source| source.path.clone())
                });
                diagnostic(
                    &source_path,
                    "E-DB-001",
                    &error.message,
                    error.span.line,
                    error.span.column,
                );
            }
            Err(())
        }
    }
}

fn print_plan(plan: &zelyra_database::SchemaPlan) {
    if plan.changes.is_empty() {
        println!("No schema changes.");
        return;
    }
    for check in &plan.nullability_preflights {
        println!(
            "[PREFLIGHT] verify `{}.{}` has no NULL values before applying any SQL",
            check.table, check.column
        );
    }
    for check in &plan.required_column_preflights {
        println!(
            "[PREFLIGHT] verify `{}` is empty before adding required column `{}` without a default",
            check.table, check.column
        );
    }
    for change in &plan.changes {
        let risk = match change.risk {
            Risk::Safe => "SAFE",
            Risk::RequiresApproval => "REVIEW",
            Risk::Destructive => "DESTRUCTIVE",
            Risk::Unsupported => "UNSUPPORTED",
        };
        println!("[{risk}] {}\n{}\n", change.description, change.sql);
    }
}

fn database_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(subcommand) = args.next() else {
        database_usage();
        return ExitCode::from(2);
    };
    let path = args.next().unwrap_or_else(|| "main.zyl".into());
    let remaining_args = args.collect::<Vec<_>>();
    let allow_risky = remaining_args.iter().any(|arg| arg == "--allow-risky");
    let allow_destructive = remaining_args
        .iter()
        .any(|arg| arg == "--allow-destructive");
    let schema = match load_schema(&path) {
        Ok(schema) => schema,
        Err(()) => return ExitCode::from(1),
    };
    match subcommand.as_str() {
        "create" => {
            println!("{}", schema.create_sql());
            ExitCode::SUCCESS
        }
        "setup" | "bootstrap" => {
            let Some(url) = database_url_from_schema(&schema) else {
                eprintln!(
                    "error[E-DB-003]: DATABASE_URL is required for db {}",
                    subcommand
                );
                if subcommand == "setup" {
                    eprintln!("hint: set a MariaDB URL without committing it to source control");
                    eprintln!(
                        "  export DATABASE_URL='mariadb://user:<password>@127.0.0.1:3306/my_app'"
                    );
                    eprintln!("  # PowerShell: $env:DATABASE_URL = 'mariadb://user:<password>@127.0.0.1:3306/my_app'");
                }
                return ExitCode::from(1);
            };
            let result = match schema.backend() {
                Backend::MariaDb => match inspect_mariadb(&url) {
                    Ok(_) => apply_mariadb(&url, &schema.create_sql()),
                    Err(_) => create_mariadb_database(&url)
                        .and_then(|()| apply_mariadb(&url, &schema.create_sql())),
                },
                Backend::Sqlite => apply_sqlite(&url, &schema.create_sql()),
                Backend::Postgres => Err(zelyra_database::DatabaseError {
                    message: format!(
                        "db {} currently supports mariadb and sqlite; use db apply for postgres",
                        subcommand
                    ),
                }),
            };
            match result {
                Ok(()) => {
                    println!("database {} completed successfully", subcommand);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error[E-DB-005]: {error}");
                    ExitCode::from(1)
                }
            }
        }
        "inspect" => match database_url_from_schema(&schema) {
            Some(url) => match inspect_for_backend(schema.backend(), &url) {
                Ok(current) => {
                    println!("{}", current.summary());
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error[E-DB-002]: {error}");
                    ExitCode::from(1)
                }
            },
            None => {
                eprintln!("error[E-DB-003]: DATABASE_URL is required for db inspect");
                ExitCode::from(1)
            }
        },
        "plan" => {
            let current = match database_url_from_schema(&schema) {
                Some(url) => match inspect_for_backend(schema.backend(), &url) {
                    Ok(current) => current,
                    Err(error) => {
                        eprintln!("error[E-DB-002]: {error}");
                        return ExitCode::from(1);
                    }
                },
                None => {
                    eprintln!("note: DATABASE_URL is not set; planning against an empty database");
                    Schema {
                        database: None,
                        tables: Vec::new(),
                    }
                }
            };
            print_plan(&diff(&schema, &current));
            ExitCode::SUCCESS
        }
        "apply" => {
            let Some(url) = database_url_from_schema(&schema) else {
                eprintln!("error[E-DB-003]: DATABASE_URL is required for db apply");
                return ExitCode::from(1);
            };
            let current = match inspect_for_backend(schema.backend(), &url) {
                Ok(current) => current,
                Err(error) => {
                    eprintln!("error[E-DB-002]: {error}");
                    return ExitCode::from(1);
                }
            };
            let plan = diff(&schema, &current);
            print_plan(&plan);
            if plan.has_unsupported() {
                eprintln!(
                    "error[E-DB-006]: schema plan contains unsupported changes; no SQL was applied"
                );
                return ExitCode::from(1);
            }
            let has_review_changes = plan
                .changes
                .iter()
                .any(|change| change.risk == Risk::RequiresApproval);
            let legacy_approval_is_sufficient = allow_destructive && !has_review_changes;
            if plan.requires_approval() && !allow_risky && !legacy_approval_is_sufficient {
                eprintln!("error[E-DB-004]: schema changes requiring review were refused; review the plan and use --allow-risky to approve it");
                return ExitCode::from(1);
            }
            if plan.changes.is_empty() {
                return ExitCode::SUCCESS;
            }
            for check in &plan.nullability_preflights {
                match count_null_values(&url, schema.backend(), &check.table, &check.column) {
                    Ok(0) => {}
                    Ok(_) => {
                        eprintln!(
                            "error[E-DB-005]: cannot require `{}.{}` because existing rows contain NULL values; no schema SQL was applied",
                            check.table, check.column
                        );
                        return ExitCode::from(1);
                    }
                    Err(_) => {
                        eprintln!(
                            "error[E-DB-005]: could not verify that `{}.{}` contains no NULL values; no schema SQL was applied",
                            check.table, check.column
                        );
                        return ExitCode::from(1);
                    }
                }
            }
            let mut table_row_presence = HashMap::new();
            for check in &plan.required_column_preflights {
                let has_rows = *table_row_presence
                    .entry(check.table.as_str())
                    .or_insert_with(|| {
                        table_has_rows(&url, schema.backend(), &check.table).map_err(|_| ())
                    });
                match has_rows {
                    Ok(false) => {}
                    Ok(true) => {
                        eprintln!(
                            "error[E-DB-005]: cannot add required column `{}.{}` without a default because the table contains existing rows; no schema SQL was applied. Add a default or stage the change: add it as nullable, backfill the rows, then require it",
                            check.table, check.column
                        );
                        return ExitCode::from(1);
                    }
                    Err(_) => {
                        eprintln!(
                            "error[E-DB-005]: could not verify that table `{}` is empty before adding required column `{}.{}`; no schema SQL was applied",
                            check.table, check.table, check.column
                        );
                        return ExitCode::from(1);
                    }
                }
            }
            let mut sql = plan.sql();
            if schema.backend() == Backend::MariaDb
                && (!plan.nullability_preflights.is_empty()
                    || !plan.required_column_preflights.is_empty())
            {
                sql = format!(
                    "SET SESSION sql_mode = CONCAT_WS(',', NULLIF(@@SESSION.sql_mode, ''), 'STRICT_ALL_TABLES');\n{sql}"
                );
            }
            let result = match schema.backend() {
                Backend::Postgres => apply_postgres(&url, &sql),
                Backend::MariaDb => apply_mariadb(&url, &sql),
                Backend::Sqlite => apply_sqlite(&url, &sql),
            };
            match result {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("error[E-DB-005]: {error}");
                    ExitCode::from(1)
                }
            }
        }
        _ => {
            database_usage();
            ExitCode::from(2)
        }
    }
}

fn inspect_for_backend(
    backend: Backend,
    database_url: &str,
) -> Result<Schema, zelyra_database::DatabaseError> {
    match backend {
        Backend::Postgres => inspect_postgres(database_url),
        Backend::MariaDb => inspect_mariadb(database_url),
        Backend::Sqlite => inspect_sqlite(database_url),
    }
}

fn serve_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(path) = args.next() else {
        usage();
        return ExitCode::from(2);
    };
    let address = args.next().unwrap_or_else(|| "127.0.0.1:3000".into());
    if args.next().is_some() {
        usage();
        return ExitCode::from(2);
    }
    let (ui_language, ui_level) = match project_ui_settings(&path) {
        Ok(settings) => settings,
        Err(error) => {
            diagnostic(&path, "E-ENV-001", &error, 1, 1);
            return ExitCode::from(1);
        }
    };
    let allowed_hosts = match project_allowed_hosts(&path) {
        Ok(hosts) => hosts,
        Err(error) => {
            diagnostic(&path, "E-ENV-001", &error, 1, 1);
            return ExitCode::from(1);
        }
    };
    let theme_css = match project_theme_css(&path) {
        Ok(theme_css) => theme_css,
        Err(error) => {
            diagnostic(&path, "E-THEME-001", &error, 1, 1);
            return ExitCode::from(1);
        }
    };
    let ui_catalogs = match project_ui_catalogs(&path) {
        Ok(catalogs) => catalogs,
        Err(error) => {
            diagnostic(&path, "E-I18N-001", &error, 1, 1);
            return ExitCode::from(1);
        }
    };
    let program = match load_project(&path) {
        Ok(project) => project.program,
        Err(()) => return ExitCode::from(1),
    };
    if theme_css.is_some() && project_uses_reserved_theme_route(&program) {
        diagnostic(
            &path,
            "E-THEME-002",
            &format!(
                "route `{PROJECT_THEME_CSS_PATH}` is reserved for the project theme stylesheet"
            ),
            1,
            1,
        );
        return ExitCode::from(1);
    }
    let source = fs::read_to_string(&path).unwrap_or_default();
    if !reject_typed_holes(&source, &path, &program) {
        return ExitCode::from(1);
    }
    if validate_capabilities(&path, &program).is_err() {
        return ExitCode::from(1);
    }
    let capability_grants = match project_capability_grants(&path) {
        Ok(grants) => grants,
        Err(error) => {
            diagnostic(&path, "E-CAP-002", &error, 1, 1);
            return ExitCode::from(1);
        }
    };
    let runtime_policy = match project_runtime_policy(&path) {
        Ok(policy) => policy,
        Err(error) => {
            diagnostic(&path, "E-POLICY-002", &error, 1, 1);
            return ExitCode::from(1);
        }
    };
    let cors_policy = match project_cors_policy(&path) {
        Ok(policy) => policy,
        Err(error) => {
            diagnostic(&path, "E-WEB-004", &error, 1, 1);
            return ExitCode::from(1);
        }
    };
    if program.pages.is_empty()
        && program.forms.is_empty()
        && program.cruds.is_empty()
        && program.tableviews.is_empty()
        && program.apis.is_empty()
    {
        eprintln!("error[E-WEB-001]: {path} does not define a page, form, CRUD resource, or API");
        return ExitCode::from(1);
    }
    if !validate_views(&path, &program)
        || !validate_page_inputs(&path, &program)
        || !validate_page_data(&path, &program)
        || !validate_components(&path, &program)
    {
        return ExitCode::from(1);
    }
    let routes = program
        .pages
        .iter()
        .map(|page| Route {
            path: page.path.clone(),
            html: compose_page_view(&program, page),
            query: page
                .inputs
                .iter()
                .map(|input| RouteQuery {
                    name: input.name.clone(),
                    ty: input.ty.clone(),
                })
                .collect(),
            page_size: page.page_size,
            sort_columns: page.sort.clone(),
            search_columns: page.search.clone(),
            filters: page
                .data
                .iter()
                .find(|data| matches!(data.result_type, Type::Array(_)))
                .map(|data| {
                    page.filters
                        .iter()
                        .map(|name| TableViewFilter {
                            name: name.clone(),
                            kind: page_filter_kind(&program, &data.result_type, name),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            data: page
                .data
                .iter()
                .map(|data| RouteData {
                    name: data.name.clone(),
                    query: data.query.clone(),
                    fields: page_data_fields(&program, &data.result_type),
                    collection: matches!(data.result_type, Type::Array(_)),
                    optional: matches!(data.result_type, Type::Option(_)),
                })
                .collect(),
            requires_auth: page.requires_auth,
            permissions: page.permissions.clone(),
        })
        .collect();
    let schema = match build_schema(&program) {
        Ok(schema) => schema,
        Err(errors) => {
            for error in errors {
                diagnostic(
                    &path,
                    "E-DB-001",
                    &error.message,
                    error.span.line,
                    error.span.column,
                );
            }
            return ExitCode::from(1);
        }
    };
    if !validate_auth(&path, &program, &schema) {
        return ExitCode::from(1);
    }
    if !validate_cruds(&path, &program, &schema) {
        return ExitCode::from(1);
    }
    if !validate_tableviews(&path, &program, &schema) {
        return ExitCode::from(1);
    }
    if let Err(errors) = check_form_program(&program, &schema) {
        for error in errors {
            diagnostic(
                &path,
                "E-FORM-001",
                &error.message,
                error.span.line,
                error.span.column,
            );
        }
        return ExitCode::from(1);
    }
    let mut form_routes = Vec::new();
    let auth_route = if let Some(auth) = program.auth.first() {
        let Some(csrf) = CsrfProtection::generate().ok() else {
            eprintln!("error[E-WEB-003]: cannot create a secure CSRF token");
            return ExitCode::from(1);
        };
        Some(AuthRoute {
            table: auth.table.clone(),
            session_table: auth.session_table.clone(),
            permissions_table: auth.permissions_table.clone(),
            roles_table: auth.roles_table.clone(),
            role_permissions_table: auth.role_permissions_table.clone(),
            audit_table: auth.audit_table.clone(),
            audit_chain: auth.audit_chain,
            admin_path: auth.admin_path.clone(),
            admin_permission: auth.admin_permission.clone(),
            admin_role: auth.admin_role.clone(),
            schema: schema.clone(),
            csrf,
        })
    } else {
        None
    };
    for form in &program.forms {
        let Some(csrf) = CsrfProtection::generate().ok() else {
            eprintln!("error[E-WEB-003]: cannot create a secure CSRF token");
            return ExitCode::from(1);
        };
        let table = form.table.as_deref().and_then(|table_name| {
            program
                .tables
                .iter()
                .find(|table| table.name == table_name)
                .cloned()
        });
        form_routes.push(FormRoute {
            path: format!("/forms/{}", form.name),
            action: format!("/forms/{}", form.name),
            form: form.clone(),
            table,
            schema: Some(schema.clone()),
            requires_auth: false,
            permissions: Vec::new(),
            csrf,
            form_view: zelyra_ast::CrudFormViewDef::default(),
            post_only: false,
            audit_table: None,
            audit_event: None,
            audit_chain: false,
            layout_html: None,
        });
    }
    for crud in &program.cruds {
        let Some(table) = program.tables.iter().find(|table| table.name == crud.table) else {
            continue;
        };
        for edit in [false, true] {
            let Some(csrf) = CsrfProtection::generate().ok() else {
                eprintln!("error[E-WEB-003]: cannot create a secure CSRF token");
                return ExitCode::from(1);
            };
            form_routes.push(generated_crud_form(
                crud,
                table,
                &schema,
                edit,
                CrudGenerationContext {
                    layout_html: crud_layout_html(&program, crud),
                    csrf,
                    audit_table: auth_route
                        .as_ref()
                        .and_then(|auth| auth.audit_table.clone()),
                    audit_chain: auth_route.as_ref().is_some_and(|auth| auth.audit_chain),
                },
            ));
        }
    }
    let mut crud_routes = Vec::new();
    for crud in &program.cruds {
        let Some(table) = program.tables.iter().find(|table| table.name == crud.table) else {
            continue;
        };
        let Some(csrf) = CsrfProtection::generate().ok() else {
            eprintln!("error[E-WEB-003]: cannot create a secure CSRF token");
            return ExitCode::from(1);
        };
        let soft_delete_column = crud
            .soft_delete
            .as_ref()
            .map(|definition| definition.column.as_str());
        let view_fields = if crud.list.is_empty() {
            &crud.view.fields
        } else {
            &crud.list
        };
        let list_columns = configured_crud_columns(&program, &schema, crud, view_fields, |table| {
            table
                .columns
                .iter()
                .filter(|column| Some(column.name.as_str()) != soft_delete_column)
                .map(|column| column.name.clone())
                .collect()
        });
        let search_columns =
            configured_crud_columns(&program, &schema, crud, &crud.search, |table| {
                table
                    .columns
                    .iter()
                    .filter(|column| {
                        let sql_type = column.sql_type.to_ascii_uppercase();
                        sql_type.contains("CHAR") || sql_type.contains("TEXT")
                    })
                    .map(|column| column.name.clone())
                    .collect()
            });
        let filter_columns =
            configured_crud_filter_columns(&program, &schema, crud, &crud.filters, |table| {
                table
                    .columns
                    .iter()
                    .filter(|column| column.name != "id")
                    .filter(|column| Some(column.name.as_str()) != soft_delete_column)
                    .map(|column| column.name.clone())
                    .collect()
            });
        let actions = crud
            .actions
            .iter()
            .map(|action| {
                generated_crud_action(
                    crud,
                    table,
                    &schema,
                    action,
                    CrudGenerationContext {
                        layout_html: crud_layout_html(&program, crud),
                        csrf: csrf.clone(),
                        audit_table: auth_route
                            .as_ref()
                            .and_then(|auth| auth.audit_table.clone()),
                        audit_chain: auth_route.as_ref().is_some_and(|auth| auth.audit_chain),
                    },
                )
            })
            .collect();
        crud_routes.push(CrudRoute {
            path: format!("/{}", crud.table),
            title: crud
                .title
                .clone()
                .unwrap_or_else(|| zelyra_web::localized_identifier(ui_language, &crud.name)),
            table: crud.table.clone(),
            list_columns,
            search_columns,
            filter_columns,
            list_view: crud.view.list.clone(),
            detail_view: crud.view.detail.clone(),
            delete_view: crud.view.delete.clone(),
            loading_view: crud.view.loading.clone(),
            error_view: crud.view.error.clone(),
            layout_html: crud_layout_html(&program, crud),
            soft_delete: crud.soft_delete.clone(),
            actions,
            requires_auth: crud.requires_auth,
            permissions: crud.permissions.clone(),
            create_permissions: effective_crud_permissions(
                &crud.permissions,
                &crud.create_permissions,
            ),
            edit_permissions: effective_crud_permissions(&crud.permissions, &crud.edit_permissions),
            delete_permissions: effective_crud_permissions(
                &crud.permissions,
                &crud.delete_permissions,
            ),
            schema: schema.clone(),
            csrf,
        });
    }
    let tableview_routes = program
        .tableviews
        .iter()
        .map(|tableview| TableViewRoute {
            path: format!("/views/{}", tableview.name.to_ascii_lowercase()),
            title: zelyra_web::localized_identifier(ui_language, &tableview.name),
            source: tableview.source.clone(),
            columns: tableview.columns.clone(),
            filters: tableview
                .filters
                .iter()
                .map(|name| TableViewFilter {
                    name: name.clone(),
                    kind: tableview_filter_kind(
                        &tableview.result_type,
                        name,
                        &schema,
                        &program.records,
                    ),
                })
                .collect(),
            searchable: tableview.searchable,
            sortable: tableview.sortable,
            page_size: tableview.page_size,
            requires_auth: tableview.requires_auth,
            permissions: tableview.permissions.clone(),
        })
        .collect();
    let database_capability_granted = capability_grants
        .as_ref()
        .is_none_or(|grants| grants.contains("Database"));
    let api_routes = generated_api_routes(
        &program,
        capability_grants.as_ref(),
        runtime_policy.as_ref(),
    );
    eprintln!("Zelyra server listening on http://{address}");
    let app = WebApp::with_database_url(routes, form_routes, database_url_from_program(&program))
        .with_ui_settings(ui_language, ui_level)
        .with_project_theme_css(theme_css)
        .with_project_ui_catalogs(ui_catalogs)
        .with_database_capability(database_capability_granted)
        .with_apis(api_routes)
        .with_auth(
            env::var("ZELYRA_AUTH_TOKEN").ok(),
            env::var("ZELYRA_AUTH_PERMISSIONS")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|permission| !permission.is_empty())
                .map(str::to_owned)
                .collect(),
        )
        .with_cruds(crud_routes)
        .with_tableviews(tableview_routes);
    let app = match app.with_allowed_hosts(allowed_hosts) {
        Ok(app) => app,
        Err(error) => {
            diagnostic(&path, "E-ENV-001", &error.message, 1, 1);
            return ExitCode::from(1);
        }
    };
    let app = if let Some(cors_policy) = cors_policy {
        app.with_cors(cors_policy)
    } else {
        app
    };
    let app = if let Some(auth_route) = auth_route {
        app.with_auth_route(auth_route)
    } else {
        app
    };
    match serve_app(app, &address) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error[E-WEB-002]: cannot start server on {address}: {error}");
            ExitCode::from(1)
        }
    }
}

struct CrudGenerationContext {
    layout_html: Option<String>,
    csrf: CsrfProtection,
    audit_table: Option<String>,
    audit_chain: bool,
}

fn generated_crud_form(
    crud: &zelyra_ast::CrudDef,
    table: &zelyra_ast::TableDef,
    schema: &Schema,
    edit: bool,
    context: CrudGenerationContext,
) -> FormRoute {
    let permissions = if edit {
        effective_crud_permissions(&crud.permissions, &crud.edit_permissions)
    } else {
        effective_crud_permissions(&crud.permissions, &crud.create_permissions)
    };
    let configured_fields = (!crud.view.fields.is_empty()).then_some(&crud.view.fields);
    let fields = table
        .columns
        .iter()
        .filter(|column| !column.primary_key && !column.auto)
        .filter(|column| {
            configured_fields.is_none_or(|fields| fields.iter().any(|name| name == &column.name))
        })
        .filter(|column| {
            crud.soft_delete
                .as_ref()
                .is_none_or(|definition| definition.column != column.name)
        })
        .map(|column| zelyra_ast::FormField {
            name: column.name.clone(),
            ty: None,
            label: None,
            placeholder: None,
            required: false,
            max: None,
            widget: None,
            readonly: false,
            span: column.span,
        })
        .collect::<Vec<_>>();
    let storage_columns = fields
        .iter()
        .map(|field| storage_column_name(schema, &crud.table, &field.name))
        .collect::<Vec<_>>();
    let query = if edit {
        let assignments = storage_columns
            .iter()
            .zip(&fields)
            .map(|(column, field)| format!("{} = :{}", quote_identifier(column), field.name))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "UPDATE {} SET {} WHERE {} = :id",
            quote_identifier(&crud.table),
            assignments,
            quote_identifier("id")
        )
    } else {
        format!(
            "INSERT INTO {} ({}) VALUES ({})",
            quote_identifier(&crud.table),
            storage_columns
                .iter()
                .map(|column| quote_identifier(column))
                .collect::<Vec<_>>()
                .join(", "),
            fields
                .iter()
                .map(|field| format!(":{}", field.name))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let path = if edit {
        format!("/{}/{{id}}/edit", crud.table)
    } else {
        format!("/{}/new", crud.table)
    };
    FormRoute {
        path: path.clone(),
        action: path,
        form: zelyra_ast::FormDef {
            name: format!("{}{}", crud.name, if edit { "Edit" } else { "Create" }),
            table: Some(table.name.clone()),
            fields,
            actions: vec![zelyra_ast::FormAction {
                name: "save".into(),
                label: None,
                icon: None,
                confirm: None,
                confirm_page: None,
                success_page: None,
                error_page: None,
                fields: Vec::new(),
                requires_auth: false,
                permissions: Vec::new(),
                statements: vec![zelyra_ast::Stmt::Expr(zelyra_ast::Expr {
                    kind: zelyra_ast::ExprKind::Sql {
                        result_type: zelyra_ast::Type::Unit,
                        query,
                    },
                    span: table.span,
                })],
                success: Some("@i18n:form.saved".into()),
                redirect: Some(format!("/{}", crud.table)),
                span: table.span,
            }],
            span: table.span,
        },
        table: Some(table.clone()),
        schema: Some(schema.clone()),
        requires_auth: crud.requires_auth,
        permissions,
        csrf: context.csrf,
        form_view: crud.view.form.clone(),
        post_only: false,
        audit_table: context.audit_table,
        audit_event: Some(if edit {
            "crud.update".into()
        } else {
            "crud.create".into()
        }),
        audit_chain: context.audit_chain,
        layout_html: context.layout_html,
    }
}

fn generated_crud_action(
    crud: &zelyra_ast::CrudDef,
    table: &zelyra_ast::TableDef,
    schema: &Schema,
    action: &zelyra_ast::FormAction,
    context: CrudGenerationContext,
) -> CrudActionRoute {
    let mut permissions = crud.permissions.clone();
    permissions.extend(action.permissions.clone());
    permissions.sort();
    permissions.dedup();
    let path = format!("/{}/{{id}}/{}", crud.table, action.name);
    CrudActionRoute {
        name: action.name.clone(),
        label: action.label.clone().unwrap_or_else(|| action.name.clone()),
        icon: action.icon.clone(),
        confirm: action.confirm.clone(),
        confirm_page: action.confirm_page.clone(),
        form: FormRoute {
            path: path.clone(),
            action: path,
            form: zelyra_ast::FormDef {
                name: format!("{}{}", crud.name, action.name),
                table: Some(table.name.clone()),
                fields: action.fields.clone(),
                actions: vec![action.clone()],
                span: action.span,
            },
            table: Some(table.clone()),
            schema: Some(schema.clone()),
            requires_auth: crud.requires_auth || action.requires_auth,
            permissions,
            csrf: context.csrf,
            form_view: zelyra_ast::CrudFormViewDef {
                submit: action.label.clone(),
                ..zelyra_ast::CrudFormViewDef::default()
            },
            post_only: true,
            audit_table: context.audit_table,
            audit_event: Some(format!("crud.action.{}", action.name)),
            audit_chain: context.audit_chain,
            layout_html: context.layout_html,
        },
    }
}

fn effective_crud_permissions(default: &[String], scoped: &[String]) -> Vec<String> {
    if scoped.is_empty() {
        default.to_vec()
    } else {
        scoped.to_vec()
    }
}

fn generated_api_routes(
    program: &zelyra_ast::Program,
    capability_grants: Option<&HashSet<String>>,
    runtime_policy: Option<&RuntimePolicy>,
) -> Vec<ApiRoute> {
    let database_url = database_url_from_program(program);
    program
        .apis
        .iter()
        .filter_map(|api| {
            let handler = api.handler.as_ref()?.clone();
            let api = api.clone();
            let program = program.clone();
            let database_url = database_url.clone();
            let capability_grants = capability_grants.cloned();
            let runtime_policy = runtime_policy.cloned();
            let requires_auth = api.requires_auth;
            let permissions = api.permissions.clone();
            Some(
                ApiRoute::new(
                    api.method.clone(),
                    api.path.clone(),
                    move |request, path_params| {
                        dispatch_api_with_capabilities(
                            &program,
                            &api,
                            &handler,
                            request,
                            path_params,
                            ApiRuntimeContext {
                                database_url: database_url.as_deref(),
                                capability_grants: capability_grants.as_ref(),
                                runtime_policy: runtime_policy.as_ref(),
                            },
                        )
                    },
                )
                .with_auth(requires_auth, permissions),
            )
        })
        .collect()
}

#[cfg(test)]
fn dispatch_api(
    program: &zelyra_ast::Program,
    api: &zelyra_ast::ApiDef,
    handler: &str,
    request: &zelyra_web::Request,
    path_params: &HashMap<String, String>,
    database_url: Option<&str>,
) -> Response {
    dispatch_api_with_capabilities(
        program,
        api,
        handler,
        request,
        path_params,
        ApiRuntimeContext {
            database_url,
            capability_grants: None,
            runtime_policy: None,
        },
    )
}

#[derive(Clone, Copy)]
struct ApiRuntimeContext<'a> {
    database_url: Option<&'a str>,
    capability_grants: Option<&'a HashSet<String>>,
    runtime_policy: Option<&'a RuntimePolicy>,
}

fn dispatch_api_with_capabilities(
    program: &zelyra_ast::Program,
    api: &zelyra_ast::ApiDef,
    handler: &str,
    request: &zelyra_web::Request,
    path_params: &HashMap<String, String>,
    context: ApiRuntimeContext<'_>,
) -> Response {
    if !matches!(api.method.as_str(), "GET" | "DELETE") && !request.body.trim().is_empty() {
        if let Some(content_type) = request.headers.get("content-type") {
            let media_type = content_type
                .split(';')
                .next()
                .map(str::trim)
                .unwrap_or_default()
                .to_ascii_lowercase();
            if media_type != "application/json" && media_type != "application/x-www-form-urlencoded"
            {
                return api_error_response(
                    415,
                    "UnsupportedMediaType",
                    "API request bodies must use application/json or application/x-www-form-urlencoded",
                );
            }
        }
    }
    let values = if matches!(api.method.as_str(), "GET" | "DELETE") {
        request.target.split_once('?').map_or_else(
            || Ok(HashMap::new()),
            |(_, query)| parse_api_url_values(query),
        )
    } else if request
        .headers
        .get("content-type")
        .is_some_and(|content_type| content_type.starts_with("application/json"))
    {
        parse_api_json_object(&request.body).map_err(|message| zelyra_web::HttpError { message })
    } else {
        parse_api_url_values(&request.body)
    };
    let mut values = match values {
        Ok(values) => values,
        Err(error) => return api_error_response(400, "BadRequest", &error.to_string()),
    };
    for (name, value) in path_params {
        values.insert(name.clone(), serde_json::Value::String(value.clone()));
    }
    let mut arguments = Vec::new();
    for field in &api.input {
        let Some(value) = values.get(&field.name) else {
            if matches!(field.ty, Type::Option(_)) {
                arguments.push(RuntimeValue::Option(None));
                continue;
            }
            return api_error_response(
                400,
                "BadRequest",
                &format!("missing API input `{}`", field.name),
            );
        };
        match api_value_json(value, &field.ty, program) {
            Ok(value) => arguments.push(value),
            Err(error) => return api_error_response(400, "BadRequest", &error),
        }
    }
    match execute_function_with_capabilities_and_policies(
        program,
        handler,
        arguments,
        context.database_url,
        context.capability_grants,
        context.runtime_policy,
    ) {
        Ok(value) => api_result_response(api, &value),
        Err(error) => api_error_response(500, "InternalServerError", &error.message),
    }
}

fn api_result_response(api: &zelyra_ast::ApiDef, value: &RuntimeValue) -> Response {
    if let RuntimeValue::Result(Err(error)) = value {
        let error_name = match &**error {
            RuntimeValue::Object { type_name, .. } => type_name.clone(),
            _ => error.output(),
        };
        if let Some(declaration) = api.errors.iter().find(|declaration| {
            declaration.name == error_name
                || declaration.payload.as_ref().is_some_and(|payload| {
                    payload == &error.ty()
                        || matches!(payload, Type::Named(name) if name == &error_name)
                })
        }) {
            return api_error_response_with_details(
                declaration.status,
                &declaration.name,
                &format!("API handler returned {}", declaration.name),
                declaration.payload.as_ref().map(|_| &**error),
            );
        }
        return api_error_response(
            500,
            "InternalServerError",
            &format!("unmapped API error `{error_name}`"),
        );
    }
    Response::json(200, api_json_value(value))
}

fn api_value_json(
    value: &serde_json::Value,
    ty: &Type,
    program: &zelyra_ast::Program,
) -> Result<RuntimeValue, String> {
    if let Type::Option(inner) = ty {
        if value.is_null() {
            return Ok(RuntimeValue::Option(None));
        }
        return Ok(RuntimeValue::Option(Some(Box::new(api_value_json(
            value, inner, program,
        )?))));
    }
    if let Type::Named(name) = ty {
        if let Some(definition) = program
            .types
            .iter()
            .find(|definition| definition.name == *name)
        {
            return api_value_json(value, &definition.target, program);
        }
        if let Some(record) = program.records.iter().find(|record| record.name == *name) {
            return api_record_value(value, record, program);
        }
    }
    match ty {
        Type::Array(inner) => {
            let Some(values) = value.as_array() else {
                return Err("expected a JSON array".into());
            };
            values
                .iter()
                .map(|value| api_value_json(value, inner, program))
                .collect::<Result<Vec<_>, _>>()
                .map(RuntimeValue::Array)
        }
        Type::Map(key, value_type) => {
            if **key != Type::String {
                return Err("API JSON maps require String keys".into());
            }
            let Some(object) = value.as_object() else {
                return Err("expected a JSON object for Map<String, Value>".into());
            };
            object
                .iter()
                .map(|(key, value)| {
                    api_value_json(value, value_type, program)
                        .map(|value| (RuntimeValue::String(key.clone()), value))
                })
                .collect::<Result<Vec<_>, _>>()
                .map(RuntimeValue::Map)
        }
        Type::Int => value
            .as_i64()
            .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
            .map(RuntimeValue::Int)
            .ok_or_else(|| format!("invalid Int JSON value `{value}`")),
        Type::UInt => value
            .as_u64()
            .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
            .map(RuntimeValue::UInt)
            .ok_or_else(|| format!("invalid UInt JSON value `{value}`")),
        Type::Float | Type::Decimal => value
            .as_f64()
            .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
            .map(RuntimeValue::Float)
            .ok_or_else(|| format!("invalid numeric JSON value `{value}`")),
        Type::Bool => value
            .as_bool()
            .or_else(|| {
                value.as_str().and_then(|value| match value {
                    "true" | "1" => Some(true),
                    "false" | "0" => Some(false),
                    _ => None,
                })
            })
            .map(RuntimeValue::Bool)
            .ok_or_else(|| format!("invalid Bool JSON value `{value}`")),
        _ => value
            .as_str()
            .map(|value| RuntimeValue::String(value.to_owned()))
            .or_else(|| {
                if value.is_number() || value.is_boolean() {
                    Some(RuntimeValue::String(value.to_string()))
                } else {
                    None
                }
            })
            .ok_or_else(|| format!("expected a scalar JSON value, found `{value}`")),
    }
}

fn api_record_value(
    value: &serde_json::Value,
    record: &zelyra_ast::RecordDef,
    program: &zelyra_ast::Program,
) -> Result<RuntimeValue, String> {
    let Some(object) = value.as_object() else {
        return Err(format!("expected JSON object for record `{}`", record.name));
    };
    for field in object.keys() {
        if !record
            .fields
            .iter()
            .any(|candidate| candidate.name == *field)
        {
            return Err(format!(
                "unknown field `{field}` in record `{}`",
                record.name
            ));
        }
    }
    let mut fields = HashMap::new();
    for field in &record.fields {
        let Some(value) = object.get(&field.name) else {
            if matches!(field.ty, Type::Option(_)) {
                fields.insert(field.name.clone(), RuntimeValue::Option(None));
                continue;
            }
            return Err(format!(
                "missing field `{}` in record `{}`",
                field.name, record.name
            ));
        };
        fields.insert(
            field.name.clone(),
            api_value_json(value, &field.ty, program)?,
        );
    }
    Ok(RuntimeValue::Object {
        type_name: record.name.clone(),
        fields,
    })
}

fn api_json_value(value: &RuntimeValue) -> String {
    api_json_value_node(value).to_string()
}

fn api_json_value_node(value: &RuntimeValue) -> serde_json::Value {
    match value {
        RuntimeValue::Int(value) => serde_json::Value::from(*value),
        RuntimeValue::UInt(value) => serde_json::Value::from(*value),
        RuntimeValue::Float(value) => serde_json::Number::from_f64(*value)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        RuntimeValue::Bool(value) => serde_json::Value::from(*value),
        RuntimeValue::String(value) => serde_json::Value::String(value.clone()),
        RuntimeValue::Char(value) => serde_json::Value::String(value.to_string()),
        RuntimeValue::Timestamp(value) => serde_json::Value::from(*value),
        RuntimeValue::Array(values) => {
            serde_json::Value::Array(values.iter().map(api_json_value_node).collect())
        }
        RuntimeValue::Map(entries) => {
            let object = entries
                .iter()
                .map(|(key, value)| (key.output(), api_json_value_node(value)))
                .collect();
            serde_json::Value::Object(object)
        }
        RuntimeValue::Object { fields, .. } => {
            let object = fields
                .iter()
                .map(|(name, value)| (name.clone(), api_json_value_node(value)))
                .collect();
            serde_json::Value::Object(object)
        }
        RuntimeValue::Option(Some(value)) => api_json_value_node(value),
        RuntimeValue::Option(None) => serde_json::Value::Null,
        RuntimeValue::Result(Ok(value)) => api_json_value_node(value),
        RuntimeValue::Result(Err(value)) => {
            let mut object = serde_json::Map::new();
            object.insert("error".into(), api_json_value_node(value));
            serde_json::Value::Object(object)
        }
        RuntimeValue::Rows { columns, rows } => serde_json::Value::Array(
            rows.iter()
                .map(|row| {
                    let object = columns
                        .iter()
                        .zip(row)
                        .map(|(column, value)| {
                            (column.clone(), serde_json::Value::String(value.clone()))
                        })
                        .collect();
                    serde_json::Value::Object(object)
                })
                .collect(),
        ),
        RuntimeValue::Unit => serde_json::Value::Null,
    }
}

fn api_error_response(status: u16, code: &str, message: &str) -> Response {
    api_error_response_with_details(status, code, message, None)
}

fn api_error_response_with_details(
    status: u16,
    code: &str,
    message: &str,
    details: Option<&RuntimeValue>,
) -> Response {
    let mut error = serde_json::Map::new();
    error.insert("code".into(), serde_json::Value::String(code.into()));
    error.insert("message".into(), serde_json::Value::String(message.into()));
    if let Some(details) = details {
        error.insert("details".into(), api_json_value_node(details));
    }
    let mut response = serde_json::Map::new();
    response.insert("error".into(), serde_json::Value::Object(error));
    Response::json(status, serde_json::Value::Object(response).to_string())
}

fn parse_api_json_object(source: &str) -> Result<HashMap<String, serde_json::Value>, String> {
    let value: serde_json::Value = serde_json::from_str(source)
        .map_err(|error| format!("invalid JSON request body: {error}"))?;
    let serde_json::Value::Object(object) = value else {
        return Err("JSON request body must be an object".into());
    };
    object.into_iter().map(Ok).collect()
}

fn parse_api_url_values(
    source: &str,
) -> Result<HashMap<String, serde_json::Value>, zelyra_web::HttpError> {
    parse_urlencoded(source).map(|values| {
        values
            .into_iter()
            .map(|(name, value)| (name, serde_json::Value::String(value)))
            .collect()
    })
}

fn storage_column_name(schema: &Schema, table: &str, field: &str) -> String {
    schema
        .tables
        .iter()
        .find(|candidate| candidate.name == table)
        .and_then(|candidate| {
            candidate
                .columns
                .iter()
                .find(|column| column.name == field || column.name == format!("{field}_id"))
        })
        .map(|column| column.name.clone())
        .unwrap_or_else(|| field.into())
}

const CRUD_LAYOUT_CONTENT_MARKER: &str = "\u{0}ZELYRA_CRUD_CONTENT\u{0}";

fn compose_view_html(program: &zelyra_ast::Program, view_name: &str, content: &str) -> String {
    let (default_body, named_slots) =
        split_view_content(content).expect("page view slots are validated before route generation");
    compose_view_parts(program, view_name, &default_body, &named_slots)
}

fn compose_view_parts(
    program: &zelyra_ast::Program,
    view_name: &str,
    default_body: &str,
    named_slots: &HashMap<String, String>,
) -> String {
    let view = program
        .views
        .iter()
        .find(|view| view.name == view_name)
        .expect("page and CRUD views are validated before route generation");
    let slots = slot_invocations(&view.html).expect("view slots are validated");
    let mut composed = view.html.clone();
    for slot in slots.into_iter().rev() {
        let replacement = slot
            .name
            .as_deref()
            .and_then(|name| named_slots.get(name))
            .map_or_else(
                || match slot.name {
                    Some(_) => slot.body.as_deref().unwrap_or(""),
                    None => default_body,
                },
                String::as_str,
            );
        composed.replace_range(slot.start..slot.end, replacement);
    }
    expand_view_components(program, composed)
}

fn compose_page_view(program: &zelyra_ast::Program, page: &zelyra_ast::PageDef) -> String {
    if let Some(view_name) = page.view.as_deref() {
        compose_view_html(program, view_name, &page.html)
    } else {
        expand_view_components(program, page.html.clone())
    }
}

fn crud_layout_html(program: &zelyra_ast::Program, crud: &zelyra_ast::CrudDef) -> Option<String> {
    crud.layout.as_deref().map(|layout| {
        let supplied_slots = crud
            .layout_slots
            .iter()
            .map(|slot| (slot.name.clone(), slot.html.clone()))
            .collect::<HashMap<_, _>>();
        compose_view_parts(program, layout, CRUD_LAYOUT_CONTENT_MARKER, &supplied_slots)
    })
}

fn page_data_fields(program: &zelyra_ast::Program, ty: &Type) -> Vec<String> {
    let ty = match ty {
        Type::Array(inner) | Type::Option(inner) => inner.as_ref(),
        other => other,
    };
    let Type::Named(name) = ty else {
        return Vec::new();
    };
    if let Some(record) = program.records.iter().find(|record| record.name == *name) {
        return record
            .fields
            .iter()
            .map(|field| field.name.clone())
            .collect();
    }
    program
        .tables
        .iter()
        .find(|table| {
            table.name == *name || singular_type_name(&table.name).as_deref() == Some(name)
        })
        .map(|table| {
            table
                .columns
                .iter()
                .map(|column| column.name.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn render_view_component(
    component: &zelyra_ast::ComponentDef,
    attributes: &str,
    body: Option<&str>,
) -> String {
    let attributes = component_attributes(attributes).expect("view components are validated");
    let (default_body, named_slots) = body
        .map(split_component_body)
        .transpose()
        .expect("view component slots are validated")
        .unwrap_or_default();
    let mut template = component.html.clone();
    let slots = slot_invocations(&template).expect("view component slots are validated");
    for (index, slot) in slots.into_iter().enumerate().rev() {
        let replacement = format!("\u{0}ZELYRA_SLOT_{index}\u{0}");
        template.replace_range(slot.start..slot.end, &replacement);
    }
    let mut rendered = component.props.iter().fold(template, |html, prop| {
        let value = attributes.get(&prop.name).map_or("", String::as_str);
        let replacement = if value.starts_with('{') && value.ends_with('}') {
            value.to_owned()
        } else {
            html_escape(value)
        };
        html.replace(&format!("{{{}}}", prop.name), &replacement)
    });
    let slots = slot_invocations(&component.html).expect("view component slots are validated");
    for (index, slot) in slots.into_iter().enumerate() {
        let marker = format!("\u{0}ZELYRA_SLOT_{index}\u{0}");
        let replacement = slot
            .name
            .as_deref()
            .and_then(|name| named_slots.get(name))
            .map_or_else(
                || match slot.name {
                    Some(_) => slot.body.as_deref().unwrap_or(""),
                    None => default_body.as_str(),
                },
                String::as_str,
            );
        rendered = rendered.replace(&marker, replacement);
    }
    rendered
}

fn expand_view_components(program: &zelyra_ast::Program, mut html: String) -> String {
    for _ in 0..16 {
        let Ok(invocations) = component_invocations(&html) else {
            break;
        };
        let Some(invocation) = invocations.into_iter().next() else {
            break;
        };
        let Some(component) = program
            .components
            .iter()
            .find(|component| component.name == invocation.name)
        else {
            break;
        };
        let rendered = render_view_component(
            component,
            &invocation.attributes,
            invocation.body.as_deref(),
        );
        html.replace_range(invocation.start..invocation.end, &rendered);
    }
    html
}

fn quote_identifier(identifier: &str) -> String {
    format!("`{}`", identifier.replace('`', "``"))
}

fn form_usage() {
    eprintln!("Usage: zelyra form validate <file.zyl> <FormName> [field=value ...]");
}

fn form_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    if args.next().as_deref() != Some("validate") {
        form_usage();
        return ExitCode::from(2);
    }
    let Some(path) = args.next() else {
        form_usage();
        return ExitCode::from(2);
    };
    let Some(form_name) = args.next() else {
        form_usage();
        return ExitCode::from(2);
    };
    let mut input = std::collections::HashMap::new();
    for argument in args {
        let Some((field, value)) = argument.split_once('=') else {
            eprintln!("error[E-FORM-002]: expected field=value, found `{argument}`");
            return ExitCode::from(2);
        };
        if field.is_empty() {
            eprintln!("error[E-FORM-002]: field name must not be empty");
            return ExitCode::from(2);
        }
        input.insert(field.to_owned(), value.to_owned());
    }
    let program = match load(&path) {
        Ok(program) => program,
        Err(()) => return ExitCode::from(1),
    };
    let schema = match build_schema(&program) {
        Ok(schema) => schema,
        Err(errors) => {
            for error in errors {
                diagnostic(
                    &path,
                    "E-DB-001",
                    &error.message,
                    error.span.line,
                    error.span.column,
                );
            }
            return ExitCode::from(1);
        }
    };
    if let Err(errors) = check_form_program(&program, &schema) {
        for error in errors {
            diagnostic(
                &path,
                "E-FORM-001",
                &error.message,
                error.span.line,
                error.span.column,
            );
        }
        return ExitCode::from(1);
    }
    let Some(form) = program.forms.iter().find(|form| form.name == form_name) else {
        eprintln!("error[E-FORM-003]: form `{form_name}` was not found in `{path}`");
        return ExitCode::from(1);
    };
    let table_definition = form
        .table
        .as_deref()
        .and_then(|table_name| program.tables.iter().find(|table| table.name == table_name));
    let result = validate_form(form, table_definition, Some(&schema), &input);
    if result.is_valid() {
        println!("valid: {form_name}");
        ExitCode::SUCCESS
    } else {
        for error in result.errors {
            eprintln!("error[E-FORM-004]: {}: {}", error.field, error.message);
        }
        ExitCode::from(1)
    }
}

fn auth_usage() {
    eprintln!(
        "Usage:\n  zelyra auth hash-password\n  zelyra auth hash-password --stdin\n  zelyra auth role grant <file.zyl> <user-id> <role>\n  zelyra auth role revoke <file.zyl> <user-id> <role>\n  zelyra auth role-permission grant <file.zyl> <role> <permission>\n  zelyra auth role-permission revoke <file.zyl> <role> <permission>\n\nRole commands use ZELYRA_DATABASE_<NAME>_URL (with DATABASE_URL as compatibility fallback) and the role tables declared in the first auth definition.\nThe interactive password form does not echo passwords. Use --stdin for automation."
    );
}

#[derive(Clone, Debug)]
struct AuthRoleTables {
    assignments: String,
    permissions: String,
    audit: Option<String>,
    audit_chain: bool,
}

fn auth_role_tables(path: &str) -> Result<AuthRoleTables, ExitCode> {
    let program = match validate(path) {
        Ok(program) => program,
        Err(()) => return Err(ExitCode::from(1)),
    };
    let Some(auth) = program.auth.first() else {
        eprintln!("error[E-AUTH-014]: role commands require an auth definition");
        return Err(ExitCode::from(1));
    };
    let (Some(assignments), Some(permissions)) = (
        auth.roles_table.clone(),
        auth.role_permissions_table.clone(),
    ) else {
        eprintln!(
            "error[E-AUTH-015]: role commands require roles and role_permissions in the auth definition"
        );
        return Err(ExitCode::from(1));
    };
    Ok(AuthRoleTables {
        assignments,
        permissions,
        audit: auth.audit_table.clone(),
        audit_chain: auth.audit_chain,
    })
}

fn auth_role_database_url(path: &str) -> Result<String, ExitCode> {
    let program = match validate(path) {
        Ok(program) => program,
        Err(()) => return Err(ExitCode::from(1)),
    };
    match database_url_from_program(&program) {
        Some(url) if url.starts_with("mariadb://") || url.starts_with("mysql://") => Ok(url),
        Some(_) => {
            eprintln!("error[E-AUTH-016]: role commands require a MariaDB database URL");
            Err(ExitCode::from(1))
        }
        None => {
            eprintln!("error[E-AUTH-017]: a database URL is required for role commands");
            Err(ExitCode::from(1))
        }
    }
}

fn execute_auth_role_mutation(
    database_url: &str,
    sql: String,
    params: Vec<(String, QueryValue)>,
    audit: Option<(&str, bool)>,
    event: &str,
    target_user_id: Option<i64>,
    details: String,
) -> Result<(), zelyra_database::DatabaseError> {
    let mut queries = vec![Query { sql, params }];
    if let Some((audit_table, audit_chain)) = audit {
        queries.extend(audit_insert_queries(
            audit_table,
            audit_chain,
            None,
            event,
            target_user_id,
            &details,
        ));
    }
    zelyra_database::execute_mariadb_queries(database_url, &queries, true).map(|_| ())
}

fn auth_role_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(operation) = args.next() else {
        auth_usage();
        return ExitCode::from(2);
    };
    let Some(path) = args.next() else {
        auth_usage();
        return ExitCode::from(2);
    };
    let Some(user_id) = args.next().and_then(|value| value.parse::<i64>().ok()) else {
        eprintln!("error[E-AUTH-018]: user-id must be an integer");
        return ExitCode::from(2);
    };
    let Some(role) = args.next() else {
        auth_usage();
        return ExitCode::from(2);
    };
    if args.next().is_some() || role.is_empty() || !matches!(operation.as_str(), "grant" | "revoke")
    {
        auth_usage();
        return ExitCode::from(2);
    }
    let tables = match auth_role_tables(&path) {
        Ok(tables) => tables,
        Err(code) => return code,
    };
    let database_url = match auth_role_database_url(&path) {
        Ok(url) => url,
        Err(code) => return code,
    };
    let (sql, message) = if operation == "grant" {
        (
            format!(
                "INSERT INTO {} (user_id, role) SELECT :user_id, :role FROM DUAL WHERE NOT EXISTS (SELECT 1 FROM {} WHERE user_id = :user_id AND role = :role)",
                quote_identifier(&tables.assignments),
                quote_identifier(&tables.assignments),
            ),
            "role granted",
        )
    } else {
        (
            format!(
                "DELETE FROM {} WHERE user_id = :user_id AND role = :role",
                quote_identifier(&tables.assignments),
            ),
            "role revoked",
        )
    };
    let event = format!("role.{operation}");
    let details = format!("source=cli;role={role}");
    match execute_auth_role_mutation(
        &database_url,
        sql,
        vec![
            ("user_id".into(), QueryValue::Int(user_id)),
            ("role".into(), QueryValue::String(role.clone())),
        ],
        tables
            .audit
            .as_deref()
            .map(|table| (table, tables.audit_chain)),
        &event,
        Some(user_id),
        details,
    ) {
        Ok(_) => {
            println!("{message}: user {user_id} -> {role}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error[E-AUTH-019]: cannot change role assignment: {error}");
            ExitCode::from(1)
        }
    }
}

fn auth_role_permission_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(operation) = args.next() else {
        auth_usage();
        return ExitCode::from(2);
    };
    let Some(path) = args.next() else {
        auth_usage();
        return ExitCode::from(2);
    };
    let Some(role) = args.next() else {
        auth_usage();
        return ExitCode::from(2);
    };
    let Some(permission) = args.next() else {
        auth_usage();
        return ExitCode::from(2);
    };
    if args.next().is_some()
        || role.is_empty()
        || permission.is_empty()
        || !matches!(operation.as_str(), "grant" | "revoke")
    {
        auth_usage();
        return ExitCode::from(2);
    }
    let tables = match auth_role_tables(&path) {
        Ok(tables) => tables,
        Err(code) => return code,
    };
    let database_url = match auth_role_database_url(&path) {
        Ok(url) => url,
        Err(code) => return code,
    };
    let (sql, message) = if operation == "grant" {
        (
            format!(
                "INSERT INTO {} (role, permission) SELECT :role, :permission FROM DUAL WHERE NOT EXISTS (SELECT 1 FROM {} WHERE role = :role AND permission = :permission)",
                quote_identifier(&tables.permissions),
                quote_identifier(&tables.permissions),
            ),
            "permission granted",
        )
    } else {
        (
            format!(
                "DELETE FROM {} WHERE role = :role AND permission = :permission",
                quote_identifier(&tables.permissions),
            ),
            "permission revoked",
        )
    };
    let event = format!("role_permission.{operation}");
    let details = format!("source=cli;role={role};permission={permission}");
    match execute_auth_role_mutation(
        &database_url,
        sql,
        vec![
            ("role".into(), QueryValue::String(role.clone())),
            ("permission".into(), QueryValue::String(permission.clone())),
        ],
        tables
            .audit
            .as_deref()
            .map(|table| (table, tables.audit_chain)),
        &event,
        None,
        details,
    ) {
        Ok(_) => {
            println!("{message}: {role} -> {permission}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error[E-AUTH-020]: cannot change role permission: {error}");
            ExitCode::from(1)
        }
    }
}

fn audit_usage() {
    eprintln!(
        "Usage:\n  zelyra audit inspect <file.zyl> [--limit <n>]\n  zelyra audit export <file.zyl> [--limit <n>] [--format json|csv]\n  zelyra audit verify <file.zyl>\n  zelyra audit prune <file.zyl> --before <timestamp> [--confirm]\n\nAudit commands use ZELYRA_DATABASE_<NAME>_URL (with DATABASE_URL as compatibility fallback) and the audit table declared in the first auth definition. The default limit is 100 and the maximum is 10,000. Prune never changes data without --confirm."
    );
}

fn audit_project(path: &str) -> Result<(String, String, bool), ExitCode> {
    let program = match validate(path) {
        Ok(program) => program,
        Err(()) => return Err(ExitCode::from(1)),
    };
    let Some(auth) = program.auth.first() else {
        eprintln!("error[E-AUDIT-001]: audit commands require an auth definition");
        return Err(ExitCode::from(1));
    };
    let Some(audit_table) = auth.audit_table.clone() else {
        eprintln!(
            "error[E-AUDIT-001]: audit commands require audit: <table> in the auth definition"
        );
        return Err(ExitCode::from(1));
    };
    let database_url = match database_url_from_program(&program) {
        Some(url) if url.starts_with("mariadb://") || url.starts_with("mysql://") => url,
        Some(_) => {
            eprintln!("error[E-AUDIT-002]: audit commands require a MariaDB database URL");
            return Err(ExitCode::from(1));
        }
        None => {
            eprintln!("error[E-AUDIT-003]: a database URL is required for audit commands");
            return Err(ExitCode::from(1));
        }
    };
    Ok((database_url, audit_table, auth.audit_chain))
}

fn audit_limit(value: &str) -> Result<usize, ExitCode> {
    match value.parse::<usize>() {
        Ok(limit) if (1..=10_000).contains(&limit) => Ok(limit),
        _ => {
            eprintln!("error[E-AUDIT-004]: limit must be an integer between 1 and 10000");
            Err(ExitCode::from(2))
        }
    }
}

fn audit_rows(
    database_url: &str,
    audit_table: &str,
    limit: usize,
) -> Result<QueryResult, zelyra_database::DatabaseError> {
    zelyra_database::execute_mariadb_query(
        database_url,
        &format!(
            "SELECT actor_user_id, event, target_user_id, details, created_at FROM {} ORDER BY created_at DESC LIMIT {limit}",
            quote_identifier(audit_table)
        ),
        Vec::new(),
    )
}

fn audit_integrity(
    database_url: &str,
    audit_table: &str,
    chain: bool,
) -> Result<(u64, u64), zelyra_database::DatabaseError> {
    let query = if chain {
        format!(
            "SELECT COUNT(*), COALESCE(SUM(CASE WHEN event IS NULL OR event = '' OR details IS NULL OR created_at IS NULL OR previous_hash IS NULL OR entry_hash IS NULL OR previous_hash <> COALESCE(expected_previous_hash, '') OR entry_hash <> SHA2(CONCAT(COALESCE(previous_hash, ''), '|', COALESCE(actor_user_id, 'NULL'), '|', event, '|', COALESCE(target_user_id, 'NULL'), '|', details, '|', DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s')), 256) THEN 1 ELSE 0 END), 0) FROM (SELECT id, actor_user_id, event, target_user_id, details, created_at, previous_hash, entry_hash, LAG(entry_hash) OVER (ORDER BY id ASC) AS expected_previous_hash FROM {}) AS audit_rows",
            quote_identifier(audit_table)
        )
    } else {
        format!(
            "SELECT COUNT(*), COALESCE(SUM(CASE WHEN event IS NULL OR event = '' OR details IS NULL OR created_at IS NULL THEN 1 ELSE 0 END), 0) FROM {}",
            quote_identifier(audit_table)
        )
    };
    let result = zelyra_database::execute_mariadb_query(database_url, &query, Vec::new())?;
    let row = result.rows.first().cloned().unwrap_or_default();
    let total = row
        .first()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let invalid = row.get(1).and_then(|value| value.parse().ok()).unwrap_or(0);
    Ok((total, invalid))
}

fn audit_prune_count(
    database_url: &str,
    audit_table: &str,
    before: &str,
) -> Result<u64, zelyra_database::DatabaseError> {
    let result = zelyra_database::execute_mariadb_query(
        database_url,
        &format!(
            "SELECT COUNT(*) FROM {} WHERE created_at < :before",
            quote_identifier(audit_table)
        ),
        vec![("before".into(), QueryValue::String(before.into()))],
    )?;
    Ok(result
        .rows
        .first()
        .and_then(|row| row.first())
        .and_then(|value| value.parse().ok())
        .unwrap_or(0))
}

fn audit_prune(
    database_url: &str,
    audit_table: &str,
    before: &str,
) -> Result<(), zelyra_database::DatabaseError> {
    let details = format!("source=cli;before={before}");
    let queries = vec![
        Query {
            sql: format!(
                "DELETE FROM {} WHERE created_at < :before",
                quote_identifier(audit_table)
            ),
            params: vec![("before".into(), QueryValue::String(before.into()))],
        },
        Query {
            sql: format!(
                "INSERT INTO {} (actor_user_id, event, target_user_id, details) VALUES (:actor_user_id, :event, :target_user_id, :details)",
                quote_identifier(audit_table)
            ),
            params: vec![
                ("actor_user_id".into(), QueryValue::Null),
                ("event".into(), QueryValue::String("audit.prune".into())),
                ("target_user_id".into(), QueryValue::Null),
                (
                    "details".into(),
                    QueryValue::String(details.chars().take(1000).collect()),
                ),
            ],
        },
    ];
    zelyra_database::execute_mariadb_queries(database_url, &queries, true).map(|_| ())
}

fn audit_optional_value(row: &[String], index: usize) -> Option<&str> {
    row.get(index)
        .map(String::as_str)
        .filter(|value| *value != "NULL")
}

fn audit_csv_value(value: Option<&str>) -> String {
    let value = value.unwrap_or_default().replace('"', "\"\"");
    format!("\"{value}\"")
}

fn audit_rows_csv(result: &QueryResult) -> String {
    let mut output = String::from("actor_user_id,event,target_user_id,details,created_at\n");
    for row in &result.rows {
        let fields = (0..5)
            .map(|index| audit_csv_value(audit_optional_value(row, index)))
            .collect::<Vec<_>>();
        let _ = writeln!(output, "{}", fields.join(","));
    }
    output
}

fn audit_json_number(value: Option<&str>) -> String {
    value
        .and_then(|value| value.parse::<i64>().ok())
        .map_or_else(|| "null".into(), |value| value.to_string())
}

fn audit_rows_json(result: &QueryResult) -> String {
    let rows = result
        .rows
        .iter()
        .map(|row| {
            format!(
                "{{\"actor_user_id\":{},\"event\":{},\"target_user_id\":{},\"details\":{},\"created_at\":{}}}",
                audit_json_number(audit_optional_value(row, 0)),
                audit_optional_value(row, 1).map_or_else(|| "null".into(), |value| format!("\"{}\"", json_escape(value))),
                audit_json_number(audit_optional_value(row, 2)),
                audit_optional_value(row, 3).map_or_else(|| "null".into(), |value| format!("\"{}\"", json_escape(value))),
                audit_optional_value(row, 4).map_or_else(|| "null".into(), |value| format!("\"{}\"", json_escape(value))),
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", rows.join(","))
}

fn audit_rows_inspect(result: &QueryResult) -> String {
    let mut output = format!(
        "Audit log: {} entr{}\n",
        result.rows.len(),
        if result.rows.len() == 1 { "y" } else { "ies" }
    );
    output.push_str("actor_user_id | event | target_user_id | details | created_at\n");
    for row in &result.rows {
        let fields = (0..5)
            .map(|index| {
                audit_optional_value(row, index)
                    .unwrap_or("-")
                    .replace(['\n', '\r', '\t'], " ")
            })
            .collect::<Vec<_>>();
        let _ = writeln!(output, "{}", fields.join(" | "));
    }
    output
}

fn audit_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(operation) = args.next() else {
        audit_usage();
        return ExitCode::from(2);
    };
    let Some(path) = args.next() else {
        audit_usage();
        return ExitCode::from(2);
    };
    let mut limit = 100usize;
    let mut limit_given = false;
    let mut format = "inspect";
    let mut before = None;
    let mut confirm = false;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--limit" => {
                let Some(value) = args.next() else {
                    audit_usage();
                    return ExitCode::from(2);
                };
                limit = match audit_limit(&value) {
                    Ok(limit) => limit,
                    Err(code) => return code,
                };
                limit_given = true;
            }
            "--format" if operation == "export" => {
                let Some(value) = args.next() else {
                    audit_usage();
                    return ExitCode::from(2);
                };
                if !matches!(value.as_str(), "json" | "csv") {
                    eprintln!("error[E-AUDIT-005]: format must be json or csv");
                    return ExitCode::from(2);
                }
                format = if value == "json" { "json" } else { "csv" };
            }
            "--before" if operation == "prune" => {
                let Some(value) = args.next() else {
                    audit_usage();
                    return ExitCode::from(2);
                };
                if value.is_empty() {
                    eprintln!("error[E-AUDIT-007]: before timestamp must not be empty");
                    return ExitCode::from(2);
                }
                before = Some(value);
            }
            "--confirm" if operation == "prune" => {
                confirm = true;
            }
            _ => {
                audit_usage();
                return ExitCode::from(2);
            }
        }
    }
    if !matches!(
        operation.as_str(),
        "inspect" | "export" | "verify" | "prune"
    ) {
        audit_usage();
        return ExitCode::from(2);
    }
    if operation == "inspect" && format != "inspect" {
        audit_usage();
        return ExitCode::from(2);
    }
    if operation == "verify" && (format != "inspect" || before.is_some() || confirm || limit_given)
    {
        audit_usage();
        return ExitCode::from(2);
    }
    if operation == "prune" && (before.is_none() || format != "inspect" || limit_given) {
        audit_usage();
        return ExitCode::from(2);
    }
    if operation != "prune" && (before.is_some() || confirm) {
        audit_usage();
        return ExitCode::from(2);
    }
    let (database_url, audit_table, audit_chain) = match audit_project(&path) {
        Ok(project) => project,
        Err(code) => return code,
    };
    if operation == "verify" {
        let (total, invalid) = match audit_integrity(&database_url, &audit_table, audit_chain) {
            Ok(result) => result,
            Err(error) => {
                eprintln!("error[E-AUDIT-006]: cannot verify audit log: {error}");
                return ExitCode::from(1);
            }
        };
        if invalid == 0 {
            println!("Audit log verified: {total} entries, no invalid rows.");
            return ExitCode::SUCCESS;
        }
        eprintln!("error[E-AUDIT-008]: audit log contains {invalid} invalid rows out of {total}");
        return ExitCode::from(1);
    }
    if operation == "prune" {
        if audit_chain {
            eprintln!(
                "error[E-AUDIT-010]: audit prune is disabled for chained audit logs because deleting entries would break the hash chain"
            );
            return ExitCode::from(1);
        }
        let before = before
            .as_deref()
            .expect("prune requires a before timestamp");
        let count = match audit_prune_count(&database_url, &audit_table, before) {
            Ok(count) => count,
            Err(error) => {
                eprintln!("error[E-AUDIT-006]: cannot plan audit prune: {error}");
                return ExitCode::from(1);
            }
        };
        if !confirm {
            println!("Audit prune plan: {count} entries older than {before} would be removed.");
            println!("No changes applied. Re-run with --confirm to apply this plan.");
            return ExitCode::from(2);
        }
        if let Err(error) = audit_prune(&database_url, &audit_table, before) {
            eprintln!("error[E-AUDIT-009]: cannot apply audit prune: {error}");
            return ExitCode::from(1);
        }
        println!("Audit prune applied: {count} entries older than {before} removed.");
        return ExitCode::SUCCESS;
    }
    let result = match audit_rows(&database_url, &audit_table, limit) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("error[E-AUDIT-006]: cannot read audit log: {error}");
            return ExitCode::from(1);
        }
    };
    match operation.as_str() {
        "inspect" => print!("{}", audit_rows_inspect(&result)),
        "export" if format == "json" => println!("{}", audit_rows_json(&result)),
        "export" => print!("{}", audit_rows_csv(&result)),
        _ => unreachable!(),
    }
    ExitCode::SUCCESS
}

fn password_from_stdin() -> Result<String, String> {
    let mut password = String::new();
    std::io::stdin()
        .read_line(&mut password)
        .map_err(|error| format!("cannot read password from stdin: {error}"))?;
    Ok(password.trim_end_matches(['\r', '\n']).to_owned())
}

fn auth_hash_password_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let use_stdin = match args.next().as_deref() {
        None => false,
        Some("--stdin") => true,
        Some(_) => {
            auth_usage();
            return ExitCode::from(2);
        }
    };
    if args.next().is_some() {
        auth_usage();
        return ExitCode::from(2);
    }
    let password = if use_stdin {
        match password_from_stdin() {
            Ok(password) => password,
            Err(error) => {
                eprintln!("error[E-AUTH-001]: {error}");
                return ExitCode::from(1);
            }
        }
    } else {
        let password = match rpassword::prompt_password("Password: ") {
            Ok(password) => password,
            Err(error) => {
                eprintln!("error[E-AUTH-001]: cannot read password: {error}");
                return ExitCode::from(1);
            }
        };
        let confirmation = match rpassword::prompt_password("Confirm password: ") {
            Ok(password) => password,
            Err(error) => {
                eprintln!("error[E-AUTH-001]: cannot read password confirmation: {error}");
                return ExitCode::from(1);
            }
        };
        if password != confirmation {
            eprintln!("error[E-AUTH-002]: passwords do not match");
            return ExitCode::from(1);
        }
        password
    };
    match zelyra_web::hash_password(&password) {
        Ok(hash) => {
            println!("{hash}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error[E-AUTH-003]: {error}");
            ExitCode::from(1)
        }
    }
}

fn auth_command(mut args: impl Iterator<Item = String>) -> ExitCode {
    match args.next().as_deref() {
        Some("hash-password") => auth_hash_password_command(args),
        Some("role") => auth_role_command(args),
        Some("role-permission") => auth_role_permission_command(args),
        _ => {
            auth_usage();
            ExitCode::from(2)
        }
    }
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        usage();
        return ExitCode::from(2);
    };
    if command == "--help" || command == "-h" {
        usage();
        return ExitCode::SUCCESS;
    }
    if command == "--version" || command == "-V" || command == "version" {
        if args.next().is_some() {
            usage();
            return ExitCode::from(2);
        }
        return version_command();
    }
    if command == "update" {
        let check_only = match (args.next(), args.next()) {
            (None, None) => false,
            (Some(flag), None) if flag == "--check" => true,
            _ => {
                usage();
                return ExitCode::from(2);
            }
        };
        return updater::command(check_only);
    }
    if command == "db" {
        return database_command(args);
    }
    if command == "form" {
        return form_command(args);
    }
    if command == "auth" {
        return auth_command(args);
    }
    if command == "audit" {
        return audit_command(args);
    }
    if command == "setup" {
        let mut path = ".".to_owned();
        let mut path_given = false;
        let mut web = false;
        let mut action = "prepare";
        let mut web_port = DEFAULT_SETUP_WEB_PORT;
        let mut web_port_given = false;
        let mut setup_options = SetupOptions::default();
        let mut arguments = args;
        while let Some(argument) = arguments.next() {
            if argument == "--web" {
                web = true;
            } else if argument == "--database" {
                action = "database";
            } else if argument == "--schema" {
                action = "schema";
            } else if argument == "--all" {
                action = "all";
            } else if argument == "--port" {
                let Some(value) = arguments.next() else {
                    usage();
                    return ExitCode::from(2);
                };
                web_port_given = true;
                web_port = match parse_web_port(&value) {
                    Ok(port) => port,
                    Err(error) => {
                        eprintln!("error[E-SETUP-WEB-001]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else if argument == "--host-port" {
                let Some(value) = arguments.next() else {
                    usage();
                    return ExitCode::from(2);
                };
                setup_options.host_port = match parse_web_port(&value) {
                    Ok(port) => Some(port),
                    Err(error) => {
                        eprintln!("error[E-SETUP-002]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else if argument == "--db-host-port" {
                let Some(value) = arguments.next() else {
                    usage();
                    return ExitCode::from(2);
                };
                setup_options.database_host_port = match parse_database_host_port(&value) {
                    Ok(port) => Some(port),
                    Err(error) => {
                        eprintln!("error[E-SETUP-002]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else if !argument.starts_with('-') && !path_given {
                path = argument;
                path_given = true;
            } else {
                usage();
                return ExitCode::from(2);
            }
        }
        if web {
            if setup_options.host_port.is_some() || setup_options.database_host_port.is_some() {
                eprintln!(
                    "error[E-SETUP-002]: --host-port and --db-host-port cannot be used with --web"
                );
                return ExitCode::from(2);
            }
            return setup_web_command(&path, web_port, web_port_given);
        }
        if web_port_given {
            eprintln!("error[E-SETUP-002]: --port requires --web");
            return ExitCode::from(2);
        }
        if action == "prepare" {
            return setup_project(&path, &setup_options);
        }
        return match setup_action(&path, action, &setup_options) {
            Ok(message) => {
                println!("{message}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error[E-SETUP-006]: {error}");
                ExitCode::from(1)
            }
        };
    }
    if command == "new" {
        let Some(path) = args.next() else {
            usage();
            return ExitCode::from(2);
        };
        let mut with_mariadb = false;
        let mut crud_template = false;
        let mut auth_template = false;
        let mut business_template = false;
        let mut web_port = DEFAULT_WEB_PORT;
        let mut web_port_given = false;
        let mut host_port = DEFAULT_WEB_PORT;
        let mut host_port_given = false;
        let mut database_host_port = DEFAULT_DATABASE_HOST_PORT;
        let mut database_host_port_given = false;
        let mut arguments = args;
        while let Some(argument) = arguments.next() {
            if argument == "--mariadb" && !with_mariadb {
                with_mariadb = true;
            } else if argument == "--template" {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --template requires a value");
                    return ExitCode::from(2);
                };
                match value.as_str() {
                    "minimal" => {
                        crud_template = false;
                        auth_template = false;
                        business_template = false;
                    }
                    "mariadb-crud" => {
                        with_mariadb = true;
                        crud_template = true;
                        auth_template = false;
                        business_template = false;
                    }
                    "mariadb-auth" => {
                        with_mariadb = true;
                        crud_template = false;
                        auth_template = true;
                        business_template = false;
                    }
                    "mariadb-business" => {
                        with_mariadb = true;
                        crud_template = false;
                        auth_template = false;
                        business_template = true;
                    }
                    _ => {
                        eprintln!(
                            "error[E-CLI-001]: unknown template `{value}`; expected `minimal`, `mariadb-crud`, `mariadb-auth`, or `mariadb-business`"
                        );
                        return ExitCode::from(2);
                    }
                }
            } else if argument == "--web-port" {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --web-port requires a value");
                    return ExitCode::from(2);
                };
                web_port_given = true;
                web_port = match parse_web_port(&value) {
                    Ok(port) => port,
                    Err(error) => {
                        eprintln!("error[E-CLI-001]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else if argument == "--host-port" {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --host-port requires a value");
                    return ExitCode::from(2);
                };
                host_port_given = true;
                host_port = match parse_web_port(&value) {
                    Ok(port) => port,
                    Err(error) => {
                        eprintln!("error[E-CLI-001]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else if argument == "--db-host-port" {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --db-host-port requires a value");
                    return ExitCode::from(2);
                };
                database_host_port_given = true;
                database_host_port = match parse_database_host_port(&value) {
                    Ok(port) => port,
                    Err(error) => {
                        eprintln!("error[E-CLI-001]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else {
                usage();
                return ExitCode::from(2);
            }
        }
        if (web_port_given || host_port_given || database_host_port_given) && !with_mariadb {
            eprintln!(
                "error[E-CLI-001]: --web-port, --host-port, and --db-host-port require --mariadb"
            );
            return ExitCode::from(2);
        }
        return create_project(
            &path,
            ProjectOptions {
                allow_current_directory: false,
                with_mariadb,
                crud_template,
                auth_template,
                business_template,
                web_port,
                host_port,
                database_host_port,
                host_port_given,
                database_host_port_given,
            },
        );
    }
    if command == "init" {
        let mut path = ".".to_owned();
        let mut path_given = false;
        let mut with_mariadb = false;
        let mut crud_template = false;
        let mut auth_template = false;
        let mut business_template = false;
        let mut web_port = DEFAULT_WEB_PORT;
        let mut web_port_given = false;
        let mut host_port = DEFAULT_WEB_PORT;
        let mut host_port_given = false;
        let mut database_host_port = DEFAULT_DATABASE_HOST_PORT;
        let mut database_host_port_given = false;
        let mut arguments = args;
        while let Some(argument) = arguments.next() {
            if argument == "--mariadb" && !with_mariadb {
                with_mariadb = true;
            } else if argument == "--template" {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --template requires a value");
                    return ExitCode::from(2);
                };
                match value.as_str() {
                    "minimal" => {
                        crud_template = false;
                        auth_template = false;
                        business_template = false;
                    }
                    "mariadb-crud" => {
                        with_mariadb = true;
                        crud_template = true;
                        auth_template = false;
                        business_template = false;
                    }
                    "mariadb-auth" => {
                        with_mariadb = true;
                        crud_template = false;
                        auth_template = true;
                        business_template = false;
                    }
                    "mariadb-business" => {
                        with_mariadb = true;
                        crud_template = false;
                        auth_template = false;
                        business_template = true;
                    }
                    _ => {
                        eprintln!(
                            "error[E-CLI-001]: unknown template `{value}`; expected `minimal`, `mariadb-crud`, `mariadb-auth`, or `mariadb-business`"
                        );
                        return ExitCode::from(2);
                    }
                }
            } else if argument == "--web-port" {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --web-port requires a value");
                    return ExitCode::from(2);
                };
                web_port_given = true;
                web_port = match parse_web_port(&value) {
                    Ok(port) => port,
                    Err(error) => {
                        eprintln!("error[E-CLI-001]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else if argument == "--host-port" {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --host-port requires a value");
                    return ExitCode::from(2);
                };
                host_port_given = true;
                host_port = match parse_web_port(&value) {
                    Ok(port) => port,
                    Err(error) => {
                        eprintln!("error[E-CLI-001]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else if argument == "--db-host-port" {
                let Some(value) = arguments.next() else {
                    eprintln!("error[E-CLI-001]: --db-host-port requires a value");
                    return ExitCode::from(2);
                };
                database_host_port_given = true;
                database_host_port = match parse_database_host_port(&value) {
                    Ok(port) => port,
                    Err(error) => {
                        eprintln!("error[E-CLI-001]: {error}");
                        return ExitCode::from(2);
                    }
                };
            } else if !argument.starts_with('-') && !path_given {
                path = argument;
                path_given = true;
            } else {
                usage();
                return ExitCode::from(2);
            }
        }
        if (web_port_given || host_port_given || database_host_port_given) && !with_mariadb {
            eprintln!(
                "error[E-CLI-001]: --web-port, --host-port, and --db-host-port require --mariadb"
            );
            return ExitCode::from(2);
        }
        return create_project(
            &path,
            ProjectOptions {
                allow_current_directory: true,
                with_mariadb,
                crud_template,
                auth_template,
                business_template,
                web_port,
                host_port,
                database_host_port,
                host_port_given,
                database_host_port_given,
            },
        );
    }
    if command == "serve" {
        return serve_command(args);
    }
    if command == "doctor" {
        return doctor_command(args);
    }
    if command == "doc" {
        return doc_command(args);
    }
    if command == "check" {
        return check_command(args);
    }
    if command == "fmt" {
        return fmt_command(args);
    }
    if command == "impact" {
        return impact_command(args);
    }
    if command == "edit" {
        return edit_command(args);
    }
    if command == "context" {
        return context_command(args);
    }
    if command == "module" {
        return module_command(args);
    }
    if command == "config" {
        return config_command(args);
    }
    if command == "verify" {
        let Some(path) = args.next() else {
            usage();
            return ExitCode::from(2);
        };
        let json = match args.next() {
            None => false,
            Some(flag) if flag == "--json" => true,
            Some(_) => {
                usage();
                return ExitCode::from(2);
            }
        };
        if args.next().is_some() {
            usage();
            return ExitCode::from(2);
        }
        return verify_command(&path, json);
    }
    let Some(path) = args.next() else {
        usage();
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        usage();
        return ExitCode::from(2);
    }
    match command.as_str() {
        "check" | "build" => {
            if validate(&path).is_ok() {
                println!("ok: {path}");
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        "run" => match validate(&path).and_then(|program| {
            let grants = match project_capability_grants(&path) {
                Ok(grants) => grants,
                Err(error) => {
                    diagnostic(&path, "E-CAP-002", &error, 1, 1);
                    return Err(());
                }
            };
            let runtime_policy = match project_runtime_policy(&path) {
                Ok(policy) => policy,
                Err(error) => {
                    diagnostic(&path, "E-POLICY-002", &error, 1, 1);
                    return Err(());
                }
            };
            let result = match database_url_from_program(&program) {
                Some(database_url) => execute_with_database_and_capabilities_and_policies(
                    &program,
                    &database_url,
                    grants.as_ref(),
                    runtime_policy.as_ref(),
                ),
                None => execute_with_capabilities_and_policies(
                    &program,
                    grants.as_ref(),
                    runtime_policy.as_ref(),
                ),
            };
            result.map_err(|error| {
                diagnostic(
                    &path,
                    "E-RUNTIME-001",
                    &error.message,
                    error.span.line,
                    error.span.column,
                );
            })
        }) {
            Ok(output) => {
                for line in output {
                    println!("{line}");
                }
                ExitCode::SUCCESS
            }
            Err(()) => ExitCode::from(1),
        },
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_environment_names_are_derived_from_database_declarations() {
        assert_eq!(
            database_url_environment_name("main"),
            "ZELYRA_DATABASE_MAIN_URL"
        );
        assert_eq!(
            database_url_environment_name("sales_2"),
            "ZELYRA_DATABASE_SALES_2_URL"
        );
        assert_eq!(
            database_url_environment_name("sales-west"),
            "ZELYRA_DATABASE_SALES_WEST_URL"
        );
    }

    #[test]
    fn named_database_url_in_env_file_precedes_legacy_url_and_falls_back() {
        let directory = std::env::temp_dir().join(format!(
            "zelyra-database-env-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let env_file = directory.join(".env");
        fs::write(
            &env_file,
            "DATABASE_URL=mariadb://legacy.invalid/app\nZELYRA_DATABASE_MAIN_URL=mariadb://named.invalid/app\n",
        )
        .unwrap();

        assert_eq!(
            database_url_from_env_file(env_file.to_str().unwrap(), Some("main")).unwrap(),
            Some((
                "mariadb://named.invalid/app".to_owned(),
                "ZELYRA_DATABASE_MAIN_URL".to_owned()
            ))
        );

        fs::write(&env_file, "DATABASE_URL=mariadb://legacy.invalid/app\n").unwrap();
        assert_eq!(
            database_url_from_env_file(env_file.to_str().unwrap(), Some("main")).unwrap(),
            Some((
                "mariadb://legacy.invalid/app".to_owned(),
                "DATABASE_URL".to_owned()
            ))
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn feature_defaults_are_simple_and_enabled() {
        let features = feature_defaults();
        assert_eq!(features.len(), PROJECT_FEATURES.len());
        assert!(features.values().all(|setting| setting.enabled));
        assert!(features.values().all(|setting| setting.source == "default"));
    }

    #[test]
    fn feature_settings_accept_known_manifest_values() {
        let values = parse_feature_section(
            "[project]\nname = \"demo\"\n\n[features]\napi = false\ncrud = true\n",
        )
        .expect("feature settings should parse");
        assert_eq!(values.get("api"), Some(&false));
        assert_eq!(values.get("crud"), Some(&true));
        assert!(!values.contains_key("web"));
    }

    #[test]
    fn project_environment_settings_use_process_then_dotenv_then_fallback() {
        let directory = std::env::temp_dir().join(format!(
            "zelyra-ui-settings-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let source_path = directory.join("main.zyl");
        fs::write(&source_path, "").unwrap();
        fs::write(directory.join(".env"), "ZELYRA_TEST_UI_LOCALE=de\n").unwrap();

        let key = "ZELYRA_TEST_UI_LOCALE";
        let previous = env::var_os(key);
        env::remove_var(key);
        assert_eq!(
            project_ui_setting(source_path.to_str().unwrap(), key, "en").unwrap(),
            "de"
        );
        env::set_var(key, "en");
        assert_eq!(
            project_ui_setting(source_path.to_str().unwrap(), key, "fallback").unwrap(),
            "en"
        );
        env::remove_var(key);
        assert_eq!(
            project_ui_setting(
                source_path.to_str().unwrap(),
                "ZELYRA_TEST_UI_MISSING",
                "work"
            )
            .unwrap(),
            "work"
        );
        if let Some(previous) = previous {
            env::set_var(key, previous);
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn allowed_hosts_use_process_then_project_env_then_loopback_fallback() {
        let directory = std::env::temp_dir().join(format!(
            "zelyra-allowed-hosts-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let source_path = directory.join("main.zyl");
        fs::write(&source_path, "").unwrap();
        fs::write(
            directory.join(".env"),
            "ZELYRA_ALLOWED_HOSTS=localhost,app.example\n",
        )
        .unwrap();

        let previous = env::var_os("ZELYRA_ALLOWED_HOSTS");
        env::remove_var("ZELYRA_ALLOWED_HOSTS");
        assert_eq!(
            project_allowed_hosts(source_path.to_str().unwrap()).unwrap(),
            ["localhost", "app.example"]
        );
        fs::remove_file(directory.join(".env")).unwrap();
        assert_eq!(
            project_allowed_hosts(source_path.to_str().unwrap()).unwrap(),
            ["localhost", "127.0.0.1", "[::1]"]
        );
        fs::write(
            directory.join(".env"),
            "ZELYRA_ALLOWED_HOSTS=localhost,app.example\n",
        )
        .unwrap();
        env::set_var("ZELYRA_ALLOWED_HOSTS", "override.example, localhost");
        assert_eq!(
            project_allowed_hosts(source_path.to_str().unwrap()).unwrap(),
            ["override.example", "localhost"]
        );
        env::set_var("ZELYRA_ALLOWED_HOSTS", " , ");
        assert!(project_allowed_hosts(source_path.to_str().unwrap()).is_err());
        env::set_var("ZELYRA_ALLOWED_HOSTS", "localhost, ");
        assert!(project_allowed_hosts(source_path.to_str().unwrap()).is_err());
        if let Some(previous) = previous {
            env::set_var("ZELYRA_ALLOWED_HOSTS", previous);
        } else {
            env::remove_var("ZELYRA_ALLOWED_HOSTS");
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn project_theme_css_is_optional_utf8_and_size_limited() {
        let directory = env::temp_dir().join(format!(
            "zelyra-project-theme-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let source_path = directory.join("main.zyl");
        fs::write(&source_path, "").unwrap();
        assert_eq!(
            project_theme_css(source_path.to_str().unwrap()).unwrap(),
            None
        );

        let theme_path = directory.join(PROJECT_THEME_CSS_FILE);
        let theme = ":root { --zelyra-color-accent: #e04b67; }";
        fs::write(&theme_path, theme).unwrap();
        assert_eq!(
            project_theme_css(source_path.to_str().unwrap()).unwrap(),
            Some(theme.into())
        );

        fs::write(
            &theme_path,
            vec![b'x'; PROJECT_THEME_CSS_MAX_BYTES as usize + 1],
        )
        .unwrap();
        assert!(project_theme_css(source_path.to_str().unwrap())
            .unwrap_err()
            .contains("128 KiB size limit"));

        fs::write(&theme_path, [0xff, 0xfe]).unwrap();
        assert!(project_theme_css(source_path.to_str().unwrap())
            .unwrap_err()
            .contains("UTF-8"));

        fs::remove_file(&theme_path).unwrap();
        fs::create_dir(&theme_path).unwrap();
        assert!(project_theme_css(source_path.to_str().unwrap())
            .unwrap_err()
            .contains("regular project file"));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn project_theme_css_does_not_follow_symbolic_links() {
        use std::os::unix::fs::symlink;

        let directory = env::temp_dir().join(format!(
            "zelyra-project-theme-link-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let source_path = directory.join("main.zyl");
        let outside_file = directory.join("private.css");
        fs::write(&source_path, "").unwrap();
        fs::write(&outside_file, "private content").unwrap();
        symlink(&outside_file, directory.join(PROJECT_THEME_CSS_FILE)).unwrap();

        let error = project_theme_css(source_path.to_str().unwrap()).unwrap_err();
        assert!(error.contains("not a symbolic link"));
        assert!(!error.contains("private content"));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn project_locale_catalogs_are_optional_validated_and_size_limited() {
        let directory = env::temp_dir().join(format!(
            "zelyra-project-locales-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let source_path = directory.join("main.zyl");
        fs::write(&source_path, "").unwrap();
        assert!(project_ui_catalogs(source_path.to_str().unwrap()).is_ok());

        let locale_directory = directory.join(PROJECT_LOCALE_DIRECTORY);
        fs::create_dir(&locale_directory).unwrap();
        fs::write(
            locale_directory.join("de.json"),
            r#"{"custom.title":"Titel"}"#,
        )
        .unwrap();
        fs::write(
            locale_directory.join("en.json"),
            r#"{"custom.title":"Title"}"#,
        )
        .unwrap();
        assert!(project_ui_catalogs(source_path.to_str().unwrap()).is_ok());

        fs::write(locale_directory.join("de.json"), r#"{"custom.title":true}"#).unwrap();
        let error = project_ui_catalogs(source_path.to_str().unwrap()).unwrap_err();
        assert!(error.contains("locales/de.json"));
        assert!(error.contains("string values"));
        assert!(!error.contains("true"));

        fs::write(
            locale_directory.join("de.json"),
            vec![b'x'; PROJECT_LOCALE_MAX_BYTES as usize + 1],
        )
        .unwrap();
        assert!(project_ui_catalogs(source_path.to_str().unwrap())
            .unwrap_err()
            .contains("256 KiB size limit"));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn project_locale_catalog_loader_does_not_follow_symbolic_links() {
        use std::os::unix::fs::symlink;

        let directory = env::temp_dir().join(format!(
            "zelyra-project-locales-link-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let source_path = directory.join("main.zyl");
        fs::write(&source_path, "").unwrap();
        let outside_file = directory.join("outside.json");
        fs::write(&outside_file, r#"{"title":"private"}"#).unwrap();
        let locale_directory = directory.join(PROJECT_LOCALE_DIRECTORY);
        fs::create_dir(&locale_directory).unwrap();
        symlink(&outside_file, locale_directory.join("de.json")).unwrap();

        let error = project_ui_catalogs(source_path.to_str().unwrap()).unwrap_err();
        assert!(error.contains("locales/de.json"));
        assert!(error.contains("symbolic link"));
        assert!(!error.contains("private"));

        fs::remove_file(locale_directory.join("de.json")).unwrap();
        fs::remove_dir(&locale_directory).unwrap();
        let outside_directory = directory.join("outside-locales");
        fs::create_dir(&outside_directory).unwrap();
        symlink(&outside_directory, &locale_directory).unwrap();
        let error = project_ui_catalogs(source_path.to_str().unwrap()).unwrap_err();
        assert!(error.contains("locales"));
        assert!(error.contains("symbolic link"));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn project_theme_stylesheet_route_is_reserved_only_by_theme_projects() {
        let themed_program =
            parse(&lex("page \"/__zelyra/theme.css\" { html { <main>Theme</main> } }").unwrap())
                .unwrap();
        let parameter_program =
            parse(&lex("page \"/{namespace}/{asset}\" { html { <main>Asset</main> } }").unwrap())
                .unwrap();
        let ordinary_program = parse(&lex("fn main() { }").unwrap()).unwrap();

        assert!(project_uses_reserved_theme_route(&themed_program));
        assert!(project_uses_reserved_theme_route(&parameter_program));
        assert!(!project_uses_reserved_theme_route(&ordinary_program));
    }

    #[test]
    fn feature_settings_accept_only_known_env_overrides() {
        let values = parse_env_feature_overrides(
            "# optional\nZELYRA_FEATURE_API=false\nZELYRA_WEB_PORT=3000\n",
        )
        .expect("feature environment settings should parse");
        assert_eq!(values.get("api"), Some(&false));
        assert_eq!(values.len(), 1);
    }

    #[test]
    fn feature_settings_reject_unknown_values() {
        let error = parse_feature_section("[features]\nmagic = true\n")
            .expect_err("unknown features must not be silently accepted");
        assert!(error.contains("unknown feature setting"));
        let error = parse_env_feature_overrides("ZELYRA_FEATURE_MAGIC=true\n")
            .expect_err("unknown environment features must not be silently accepted");
        assert!(error.contains("unknown ZELYRA_FEATURE_"));
    }

    #[test]
    fn formats_verification_results_with_source_location() {
        let result = VerificationResult {
            function: "reduce".into(),
            kind: zelyra_runtime::ContractKind::LoopInvariant,
            index: 0,
            status: VerificationStatus::Proven,
            span: zelyra_ast::Span::new(6, 12, 2, 1),
            message: "The verifier proved this condition for all analyzed paths.".into(),
            counterexample: None,
        };
        assert_eq!(
            format_verification_result("src/reduce.zyl", "first\nsecond value\n", &result),
            "PROVEN [V-001]: reduce.invariant[0] (src/reduce.zyl:2:1-2:7)\n  = The verifier proved this condition for all analyzed paths.\n    |\n  2 | second value\n    | ^^^^^^"
        );
    }

    #[test]
    fn doctor_rejects_invalid_port() {
        let arguments = ["--port".to_owned(), "not-a-port".to_owned()];
        assert_eq!(doctor_command(arguments.into_iter()), ExitCode::from(2));
        let arguments = ["--port".to_owned(), "0".to_owned()];
        assert_eq!(doctor_command(arguments.into_iter()), ExitCode::from(2));
    }

    #[test]
    fn reads_database_url_from_an_env_file_without_normalizing_secrets() {
        let path = env::temp_dir().join(format!(
            "zelyra-doctor-env-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(
            &path,
            "# local\nexport DATABASE_URL='mariadb://user:secret@127.0.0.1:3306/app'\n",
        )
        .unwrap();
        assert_eq!(
            read_env_value(path.to_str().unwrap(), "DATABASE_URL").unwrap(),
            Some("mariadb://user:secret@127.0.0.1:3306/app".into())
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn formats_doctor_json_without_database_credentials() {
        let checks = vec![
            DoctorCheck {
                name: "project_file",
                category: "configuration",
                status: "pass",
                message: "app.zyl exists".into(),
            },
            DoctorCheck {
                name: "static_checks",
                category: "project",
                status: "pass",
                message: "source is valid".into(),
            },
            DoctorCheck {
                name: "database",
                category: "configuration",
                status: "warn",
                message: "mariadb: DATABASE_URL is not set".into(),
            },
        ];
        let document: serde_json::Value =
            serde_json::from_str(&format_doctor_json("app.zyl", &checks)).unwrap();
        assert_eq!(document["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(document["status"], "ready");
        assert_eq!(document["warnings"], 1);
        assert_eq!(document["checks"][1]["category"], "project");
        assert_eq!(document["checks"][2]["status"], "warn");
        assert_eq!(document["checks"][2]["category"], "configuration");
        assert!(!format_doctor_json("app.zyl", &checks).contains("password"));
    }

    #[test]
    fn doctor_classifies_database_failures_without_returning_backend_details() {
        let cases = [
            ("Access denied for user `app`", "authentication"),
            ("connection timed out", "timeout"),
            (
                "query execution was interrupted (max_statement_time exceeded)",
                "timeout",
            ),
            ("Unknown database 'private_name'", "configuration"),
            ("could not start mariadb: executable missing", "tooling"),
            ("Can't connect to server on 'db.example'", "connectivity"),
            ("unexpected column metadata", "schema"),
        ];
        for (detail, expected) in cases {
            let category = doctor_database_category(detail);
            assert_eq!(category, expected, "wrong category for {detail}");
            let safe_message = doctor_database_failure_message(category);
            assert!(!safe_message.contains(detail));
            assert!(!safe_message.contains("private_name"));
        }
    }

    #[test]
    fn formats_verification_results_as_json() {
        let result = VerificationResult {
            function: "say\"hello".into(),
            kind: zelyra_runtime::ContractKind::Ensures,
            index: 1,
            status: VerificationStatus::RuntimeCheck,
            span: zelyra_ast::Span::new(6, 12, 2, 1),
            message: "This postcondition needs a runtime check because not all return paths are symbolically modeled.".into(),
            counterexample: Some(vec![("value".into(), 0)]),
        };
        assert_eq!(
            format_verification_json("src/file.zyl", "first\nsecond value\n", &[result]),
            r#"[{"status":"RUNTIME_CHECK","code":"V-002","message":"This postcondition needs a runtime check because not all return paths are symbolically modeled.","function":"say\"hello","kind":"ensures","index":1,"counterexample":{"value":0},"location":{"file":"src/file.zyl","start":{"line":2,"column":1},"end":{"line":2,"column":7}}}]"#
        );
    }

    #[test]
    fn formats_audit_rows_as_json_and_csv_without_losing_nulls() {
        let result = QueryResult {
            columns: vec![
                "actor_user_id".into(),
                "event".into(),
                "target_user_id".into(),
                "details".into(),
                "created_at".into(),
            ],
            rows: vec![vec![
                "NULL".into(),
                "auth.login_failed".into(),
                "NULL".into(),
                "email=anna@example.test;note=\"unknown\"".into(),
                "2026-09-17 12:00:00".into(),
            ]],
        };
        let json: serde_json::Value = serde_json::from_str(&audit_rows_json(&result)).unwrap();
        assert_eq!(json[0]["actor_user_id"], serde_json::Value::Null);
        assert_eq!(json[0]["target_user_id"], serde_json::Value::Null);
        assert_eq!(json[0]["event"], "auth.login_failed");
        assert!(
            audit_rows_csv(&result).contains("\"email=anna@example.test;note=\"\"unknown\"\"\"")
        );
    }

    #[test]
    fn composes_a_page_inside_its_named_view() {
        let source = r#"
            view Shell {
                html { <body><slot /></body> }
            }
            page "/hello/{name}" {
                view: Shell
                html { <h1>Hello, {name}!</h1> }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(validate_views("views.zyl", &program));
        let html = compose_page_view(&program, &program.pages[0]);
        assert!(html.contains("<body>"));
        assert!(html.contains("<h1>Hello, {name}!</h1>"));
        assert!(!html.contains("<slot />"));
    }

    #[test]
    fn composes_default_component_slots_and_nested_components() {
        let source = r#"
            component Panel {
                html { <section class="panel"><slot /></section> }
            }
            component Badge {
                props { text: String }
                html { <strong>{text}</strong> }
            }
            page "/status" {
                html {
                    <Panel><Badge text="Ready" /></Panel>
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(validate_components("components.zyl", &program));
        let html = compose_page_view(&program, &program.pages[0]);
        assert!(html.contains("<section class=\"panel\">"));
        assert!(html.contains("<strong>Ready</strong>"));
        assert!(!html.contains("<slot />"));
    }

    #[test]
    fn validates_route_values_used_by_typed_view_components() {
        let source = r#"
            component Greeting {
                props { text: String }
                html { <strong>{text}</strong> }
            }
            page "/hello/{name}" {
                html { <Greeting text="{name}" /> }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(validate_components("views.zyl", &program));
    }

    #[test]
    fn validates_typed_page_data_field_bindings() {
        let source = r#"
            table customers { id: Id primary auto name: String(100) }
            page "/customers/{name}" {
                load customer = sql<Customer> {
                    SELECT id, name FROM customers WHERE name = :name
                }
                html { <h1>{customer.name}</h1> }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(validate_page_data("views.zyl", &program));
        assert!(validate_components("views.zyl", &program));
    }

    #[test]
    fn rejects_unknown_typed_page_data_field() {
        let source = r#"
            table customers { id: Id primary auto name: String(100) }
            page "/customers/{name}" {
                load customer = sql<Customer> {
                    SELECT id, name FROM customers WHERE name = :name
                }
                html { <h1>{customer.email}</h1> }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(validate_page_data("views.zyl", &program));
        assert!(!validate_components("views.zyl", &program));
    }

    #[test]
    fn validates_typed_page_collection_loops() {
        let source = r#"
            table customers { id: Id primary auto name: String(100) }
            page "/customers" {
                load customers = sql<Customer[]> {
                    SELECT id, name FROM customers
                }
                html {
                    <ul>
                        for customer in customers {
                            <li>{customer.name}</li>
                        }
                    </ul>
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(validate_page_data("views.zyl", &program));
        assert!(validate_components("views.zyl", &program));
    }

    #[test]
    fn validates_typed_page_filters_and_rejects_unknown_fields() {
        let valid_source = r#"
            table customers {
                id: Id primary auto
                name: String(100) required
                active: Bool default true
            }
            page "/customers" {
                filter { name active }
                load customers = sql<Customer[]> {
                    SELECT id, name, active FROM customers
                }
                html { <p>{filter_name}</p> }
            }
        "#;
        let valid_program = parse(&lex(valid_source).unwrap()).unwrap();
        assert!(validate_page_data("filters.zyl", &valid_program));
        assert!(validate_components("filters.zyl", &valid_program));

        let invalid_source = valid_source.replace("name active", "username active");
        let invalid_program = parse(&lex(&invalid_source).unwrap()).unwrap();
        assert!(!validate_page_data("filters.zyl", &invalid_program));
    }

    #[test]
    fn rejects_non_array_page_collection_loops() {
        let source = r#"
            table customers { id: Id primary auto name: String(100) }
            page "/customers" {
                load customer = sql<Customer> {
                    SELECT id, name FROM customers
                }
                html {
                    for customer in customer {
                        <p>{customer.name}</p>
                    }
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(validate_page_data("views.zyl", &program));
        assert!(!validate_components("views.zyl", &program));
    }

    #[test]
    fn rejects_unknown_view_values() {
        let source = r#"
            page "/hello" {
                html { <p>{missing}</p> }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(!validate_components("views.zyl", &program));
    }

    #[test]
    fn rejects_route_values_with_an_incompatible_component_property_type() {
        let source = r#"
            component Counter {
                props { count: Int }
                html { <strong>{count}</strong> }
            }
            page "/hello/{name}" {
                html { <Counter count="{name}" /> }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(!validate_components("views.zyl", &program));
    }

    #[test]
    fn rejects_component_content_without_a_default_slot() {
        let source = r#"
            component Panel {
                html { <section /> }
            }
            page "/status" {
                html { <Panel><p>Unexpected content</p></Panel> }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(!validate_components("components.zyl", &program));
    }

    #[test]
    fn composes_named_component_slots() {
        let source = r#"
            component Layout {
                html {
                    <header><slot name="header" /></header>
                    <main><slot /></main>
                }
            }
            page "/dashboard" {
                html {
                    <Layout>
                        <slot name="header"><h1>Dashboard</h1></slot>
                        <p>Content</p>
                    </Layout>
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(validate_components("components.zyl", &program));
        let html = compose_page_view(&program, &program.pages[0]);
        assert!(html.contains("<header><h1>Dashboard</h1></header>"));
        assert!(html.contains("<main>"));
        assert!(html.contains("<p>Content</p>"));
        assert!(!html.contains("<slot"));
    }

    #[test]
    fn allows_component_slots_without_a_page_view_layout() {
        let source = r#"
            component DashboardPanel {
                html {
                    <section>
                        <header><slot name="header"><h1>Dashboard</h1></slot></header>
                        <main><slot /></main>
                    </section>
                }
            }
            page "/dashboard" {
                html {
                    <DashboardPanel>
                        <slot name="header"><h1>Custom dashboard</h1></slot>
                        <p>Reusable content.</p>
                    </DashboardPanel>
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(validate_views("components.zyl", &program));
        assert!(validate_components("components.zyl", &program));
        let html = compose_page_view(&program, &program.pages[0]);
        assert!(html.contains("Custom dashboard"));
        assert!(html.contains("Reusable content."));
    }

    #[test]
    fn uses_named_component_slot_fallbacks_when_not_overridden() {
        let source = r#"
            component Layout {
                html {
                    <header><slot name="header"><h1>Default heading</h1></slot></header>
                    <main><slot /></main>
                }
            }
            page "/dashboard" {
                html {
                    <Layout><p>Content</p></Layout>
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(validate_components("components.zyl", &program));
        let html = compose_page_view(&program, &program.pages[0]);
        assert!(html.contains("<header><h1>Default heading</h1></header>"));
        assert!(html.contains("<main><p>Content</p></main>"));
        assert!(!html.contains("<slot"));
    }

    #[test]
    fn supplied_named_component_slot_replaces_its_fallback() {
        let source = r#"
            component Layout {
                html { <header><slot name="header"><h1>Default heading</h1></slot></header> }
            }
            page "/dashboard" {
                html {
                    <Layout><slot name="header"><h1>Custom heading</h1></slot></Layout>
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(validate_components("components.zyl", &program));
        let html = compose_page_view(&program, &program.pages[0]);
        assert!(html.contains("<header><h1>Custom heading</h1></header>"));
        assert!(!html.contains("Default heading"));
    }

    #[test]
    fn rejects_unknown_named_component_slots() {
        let source = r#"
            component Layout {
                html { <main><slot name="content" /></main> }
            }
            page "/dashboard" {
                html {
                    <Layout><slot name="footer"><p>Footer</p></slot></Layout>
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(!validate_components("components.zyl", &program));
    }

    #[test]
    fn composes_named_view_slots_and_fallbacks() {
        let source = r#"
            view Shell {
                html {
                    <html>
                        <body>
                            <header><slot name="header"><h1>Default heading</h1></slot></header>
                            <main><slot /></main>
                            <footer><slot name="footer">Default footer</slot></footer>
                        </body>
                    </html>
                }
            }
            page "/dashboard" {
                view: Shell
                html {
                    <slot name="header"><h1>Custom heading</h1></slot>
                    <p>Dashboard content</p>
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(validate_views("views.zyl", &program));
        assert!(validate_components("views.zyl", &program));
        let html = compose_page_view(&program, &program.pages[0]);
        assert!(html.contains("<header><h1>Custom heading</h1></header>"));
        assert!(html.contains("<main>"));
        assert!(html.contains("<p>Dashboard content</p>"));
        assert!(html.contains("<footer>Default footer</footer>"));
        assert!(!html.contains("<slot"));
    }

    #[test]
    fn rejects_unknown_named_view_slots() {
        let source = r#"
            view Shell {
                html {
                    <body>
                        <header><slot name="header" /></header>
                        <main><slot /></main>
                    </body>
                }
            }
            page "/dashboard" {
                view: Shell
                html {
                    <slot name="footer"><p>Footer</p></slot>
                    <p>Dashboard content</p>
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(!validate_views("views.zyl", &program));
    }

    #[test]
    fn composes_crud_layout_with_generated_content_marker() {
        let source = r#"
            component Badge {
                props { label: String }
                html { <strong>{label}</strong> }
            }
            view Shell {
                html {
                    <html><body>
                        <header><slot name="heading"><h1>Customers</h1></slot></header>
                        <main><slot /></main>
                        <aside><slot name="help"><p>Default help</p></slot></aside>
                        <footer><slot name="footer"><p>Default footer</p></slot></footer>
                    </body></html>
                }
            }
            table customers { id: Id primary auto name: String(100) }
            crud Customer -> customers {
                layout: Shell
                slots {
                    heading { html { <Badge label="Machine register" /> } }
                    help { html { <p>Choose a machine to see its details.</p> } }
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(validate_views("views.zyl", &program));
        assert!(validate_components("views.zyl", &program));
        let layout = crud_layout_html(&program, &program.cruds[0]).unwrap();
        assert!(layout.contains("<strong>Machine register</strong>"));
        assert!(layout.contains(CRUD_LAYOUT_CONTENT_MARKER));
        assert!(layout.contains("<p>Choose a machine to see its details.</p>"));
        assert!(!layout.contains("Default help"));
        assert!(layout.contains("Default footer"));
        assert!(!layout.contains("<slot"));
    }

    #[test]
    fn rejects_crud_layout_slots_without_a_matching_layout() {
        let source = r#"
            view Shell {
                html { <header><slot name="heading" /></header><main><slot /></main> }
            }
            table customers { id: Id primary auto }
            crud Customer -> customers {
                layout: Shell
                slots { missing { html { <p>Custom content</p> } } }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(!validate_views("views.zyl", &program));

        let source_without_layout = r#"
            table customers { id: Id primary auto }
            crud Customer -> customers {
                slots { heading { html { <h1>Customers</h1> } } }
            }
        "#;
        let program = parse(&lex(source_without_layout).unwrap()).unwrap();
        assert!(!validate_views("views.zyl", &program));

        let duplicate_slots = r#"
            view Shell {
                html {
                    <header><slot name="heading" /></header><main><slot /></main>
                }
            }
            table customers { id: Id primary auto }
            crud Customer -> customers {
                layout: Shell
                slots {
                    heading { html { <h1>First heading</h1> } }
                    heading { html { <h1>Second heading</h1> } }
                }
            }
        "#;
        let program = parse(&lex(duplicate_slots).unwrap()).unwrap();
        assert!(!validate_views("views.zyl", &program));
    }

    #[test]
    fn crud_layout_slot_content_cannot_read_crud_record_values() {
        let source = r#"
            view Shell {
                html { <header><slot name="heading" /></header><main><slot /></main> }
            }
            table customers { id: Id primary auto name: String(100) }
            crud Customer -> customers {
                layout: Shell
                slots { heading { html { <h1>{customer.name}</h1> } } }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(validate_views("views.zyl", &program));
        assert!(!validate_components("views.zyl", &program));
    }

    #[test]
    fn crud_layout_components_are_checked_even_without_a_page_using_the_view() {
        let source = r#"
            component BrandMark {
                props { label: String }
                html { <strong>{label}</strong> }
            }
            view Shell {
                html {
                    <header><slot name="heading" /><BrandMark /></header><main><slot /></main>
                }
            }
            table customers { id: Id primary auto }
            crud Customer -> customers {
                layout: Shell
                slots { heading { html { <h1>Customers</h1> } } }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(validate_views("views.zyl", &program));
        assert!(!validate_components("views.zyl", &program));
    }

    #[test]
    fn rejects_unknown_crud_layout_view() {
        let source = r#"
            table customers { id: Id primary auto }
            crud Customer -> customers { layout: MissingShell }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(!validate_views("views.zyl", &program));
    }

    #[test]
    fn rejects_duplicate_named_view_slots() {
        let source = r#"
            view Shell {
                html {
                    <header><slot name="header" /></header>
                    <main><slot /></main>
                }
            }
            page "/dashboard" {
                view: Shell
                html {
                    <slot name="header"><h1>First</h1></slot>
                    <slot name="header"><h1>Second</h1></slot>
                    <p>Dashboard content</p>
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(!validate_views("views.zyl", &program));
    }

    #[test]
    fn rejects_invalid_named_view_composition() {
        let source = r#"
            view Shell {
                html { <body>No content slot</body> }
            }
            page "/customers" {
                view: MissingShell
                html { <h1>Customers</h1> }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        assert!(!validate_views("views.zyl", &program));
    }

    #[test]
    fn formats_api_declarations_as_openapi() {
        let program = parse(
            &lex(
                "type CustomerId = Id table customers { id: CustomerId primary auto name: String(100) required } api GET \"/customers/{id}\" { input { id: CustomerId } output Customer errors { 404 NotFound } } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let document = format_openapi(&program);
        assert!(document.contains("\"openapi\":\"3.0.3\""));
        assert!(document.contains("\"/customers/{id}\""));
        assert!(document.contains("\"404\":{\"description\":\"NotFound\"}"));
        assert!(document.contains("#/components/schemas/Customer"));
    }

    #[test]
    fn formats_typed_typescript_client_from_api_and_records() {
        let program = parse(
            &lex(
                "struct Address { city: String } api GET \"/customers/{id}\" { input { id: Id, term: String? } output Address errors { 404 NotFound } } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let client = format_typescript_client(&program);
        assert!(client.contains("export interface Address"));
        assert!(client.contains("async get_customers_id"));
        assert!(client.contains("encodeURIComponent(String(params.id))"));
        assert!(client.contains("Promise<Address>"));
        assert!(client.contains("new URLSearchParams()"));
        assert!(client.contains("ZelyraApiError"));
        assert!(client.contains("ZelyraApiErrorCode = \"NotFound\" | (string & {})"));
        assert!(client.contains("fromResponse(response)"));
        assert!(client.contains("payload.error.message"));
    }

    #[test]
    fn formats_typed_api_error_payloads_for_openapi_and_typescript() {
        let program = parse(
            &lex(
                "struct Problem { message: String } api GET \"/fail\" { output Result<String, Problem> errors { 422 Validation: Problem } } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let openapi = format_openapi(&program);
        assert!(openapi.contains("\"details\""));
        assert!(openapi.contains("#/components/schemas/Problem"));
        let client = format_typescript_client(&program);
        assert!(client.contains("ZelyraApiErrorPayloads"));
        assert!(client.contains("\"Validation\": Problem"));
        assert!(client.contains("details: Details | undefined"));
    }

    #[test]
    fn dispatches_json_api_input_to_a_typed_handler() {
        let program = parse(
            &lex(
                "api POST \"/echo\" { handler echo input { value: Int } output Int } fn echo(value: Int) -> Int { return value } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        assert!(check_apis(&program).is_ok());
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\r\n{\"value\":42}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "application/json; charset=utf-8");
        assert_eq!(response.body, "42");
    }

    #[test]
    fn dispatches_string_keyed_map_api_input_and_output() {
        let program = parse(
            &lex(
                "api POST \"/settings\" { handler echo input { settings: Map<String, Int> } output Map<String, Int> } fn echo(settings: Map<String, Int>) -> Map<String, Int> { return settings } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        assert!(check_apis(&program).is_ok());
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /settings HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"settings\":{\"standard\":10,\"premium\":20}}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 200);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body, serde_json::json!({"premium": 20, "standard": 10}));
    }

    #[test]
    fn returns_structured_json_for_invalid_api_input() {
        let program = parse(
            &lex(
                "api POST \"/echo\" { handler echo input { value: Int } output Int } fn echo(value: Int) -> Int { return value } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /echo HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 400);
        assert_eq!(response.content_type, "application/json; charset=utf-8");
        assert!(response.body.contains("\"code\":\"BadRequest\""));
        assert!(response.body.contains("missing API input"));
    }

    #[test]
    fn rejects_unsupported_api_request_media_types() {
        let program = parse(
            &lex(
                "api POST \"/echo\" { handler echo input { value: String } output String } fn echo(value: String) -> String { return value } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /echo HTTP/1.1\r\nContent-Type: text/plain\r\n\r\nhello",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 415);
        assert!(response.body.contains("UnsupportedMediaType"));
    }

    #[test]
    fn decodes_json_strings_with_commas_colons_and_escapes() {
        let program = parse(
            &lex(
                "api POST \"/echo\" { handler echo input { value: String } output String } fn echo(value: String) -> String { return value } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /echo HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"value\":\"a,b: \\\"quoted\\\"\"}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "\"a,b: \\\"quoted\\\"\"");
    }

    #[test]
    fn decodes_json_booleans_and_null_options() {
        let program = parse(
            &lex(
                "api POST \"/echo\" { handler echo input { active: Bool? } output Bool? } fn echo(active: Bool?) -> Bool? { return active } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /echo HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"active\":null}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "null");
    }

    #[test]
    fn decodes_and_serializes_typed_json_arrays() {
        let program = parse(
            &lex(
                "api POST \"/echo\" { handler echo input { values: Int[] } output Int[] } fn echo(values: Int[]) -> Int[] { return values } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        assert!(check_apis(&program).is_ok());
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /echo HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"values\":[1,2,3]}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "[1,2,3]");
    }

    #[test]
    fn decodes_and_serializes_nested_structured_api_objects() {
        let program = parse(
            &lex(
                "struct Address { city: String } struct CustomerInput { name: String address: Address } api POST \"/customers\" { handler echo input { customer: CustomerInput } output CustomerInput } fn echo(customer: CustomerInput) -> CustomerInput { return customer } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        assert!(check_apis(&program).is_ok());
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /customers HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"customer\":{\"name\":\"Anna\",\"address\":{\"city\":\"Berlin\"}}}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 200);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["name"], "Anna");
        assert_eq!(body["address"]["city"], "Berlin");
    }

    #[test]
    fn rejects_unknown_and_missing_record_fields() {
        let program = parse(
            &lex(
                "struct CustomerInput { name: String email: Email? } api POST \"/customers\" { handler echo input { customer: CustomerInput } output CustomerInput } fn echo(customer: CustomerInput) -> CustomerInput { return customer } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        let unknown = zelyra_web::parse_request(
            "POST /customers HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"customer\":{\"name\":\"Anna\",\"unknown\":true}}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &unknown, &HashMap::new(), None);
        assert_eq!(response.status, 400);
        assert!(response.body.contains("unknown field"));

        let missing = zelyra_web::parse_request(
            "POST /customers HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"customer\":{}}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &missing, &HashMap::new(), None);
        assert_eq!(response.status, 400);
        assert!(response.body.contains("missing field"));
    }

    #[test]
    fn maps_declared_result_errors_to_http_responses() {
        let program = parse(
            &lex(
                "api GET \"/customers\" { handler find input { } output Result<String, String> errors { 404 NotFound } } fn find() -> Result<String, String> { return Err(\"NotFound\") } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        let request = zelyra_web::parse_request("GET /customers HTTP/1.1\r\n\r\n").unwrap();
        let response = dispatch_api(&program, api, "find", &request, &HashMap::new(), None);
        assert_eq!(response.status, 404);
        assert!(response.body.contains("\"code\":\"NotFound\""));
    }

    #[test]
    fn returns_internal_error_for_undeclared_result_errors() {
        let program = parse(
            &lex(
                "api GET \"/customers\" { handler find input { } output Result<String, String> errors { 404 NotFound } } fn find() -> Result<String, String> { return Err(\"Other\") } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        let request = zelyra_web::parse_request("GET /customers HTTP/1.1\r\n\r\n").unwrap();
        let response = dispatch_api(&program, api, "find", &request, &HashMap::new(), None);
        assert_eq!(response.status, 500);
        assert!(response.body.contains("InternalServerError"));
        assert!(response.body.contains("unmapped API error"));
    }

    #[test]
    fn returns_typed_details_for_declared_api_errors() {
        let program = parse(
            &lex(
                "struct Problem { message: String } api GET \"/fail\" { handler fail input { } output Result<String, Problem> errors { 422 Validation: Problem } } fn fail() -> Result<String, Problem> { return Err(Problem { message: \"invalid customer\" }) } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        assert!(check_apis(&program).is_ok());
        let api = &program.apis[0];
        let request = zelyra_web::parse_request("GET /fail HTTP/1.1\r\n\r\n").unwrap();
        let response = dispatch_api(&program, api, "fail", &request, &HashMap::new(), None);
        assert_eq!(response.status, 422);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["error"]["code"], "Validation");
        assert_eq!(body["error"]["details"]["message"], "invalid customer");
    }

    #[test]
    fn rejects_api_error_payload_that_does_not_match_result_error_type() {
        let program = parse(
            &lex(
                "struct Problem { message: String } struct Other { code: Int } api GET \"/fail\" { handler fail input { } output Result<String, Problem> errors { 422 Validation: Other } } fn fail() -> Result<String, Problem> { return Err(Problem { message: \"invalid\" }) } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let errors = check_apis(&program).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("does not match handler error type")));
    }

    #[test]
    fn rejects_object_json_for_scalar_input() {
        let program = parse(
            &lex(
                "api POST \"/echo\" { handler echo input { value: String } output String } fn echo(value: String) -> String { return value } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        let api = &program.apis[0];
        let request = zelyra_web::parse_request(
            "POST /echo HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{\"value\":{\"nested\":true}}",
        )
        .unwrap();
        let response = dispatch_api(&program, api, "echo", &request, &HashMap::new(), None);
        assert_eq!(response.status, 400);
        assert!(response.body.contains("scalar JSON value"));
    }

    #[test]
    fn rejects_unknown_configured_crud_columns() {
        let source = r#"
            table machines {
                id: Id primary auto
                name: String(100) required
            }

            crud Machine -> machines {
                view { fields { missing } }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(!validate_cruds("test.zyl", &program, &schema));
    }

    #[test]
    fn shared_crud_view_fields_drive_list_and_form_defaults() {
        let source = r#"
            table customers {
                id: Id primary auto
                name: String(100) required
                email: Email?
                active: Bool default true
            }

            crud Customer -> customers {
                view { fields { name email active } }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(validate_cruds("test.zyl", &program, &schema));
        let crud = &program.cruds[0];
        let table = program
            .tables
            .iter()
            .find(|table| table.name == "customers")
            .unwrap();
        let form = generated_crud_form(
            crud,
            table,
            &schema,
            false,
            CrudGenerationContext {
                layout_html: None,
                csrf: CsrfProtection::new("test-csrf"),
                audit_table: None,
                audit_chain: false,
            },
        );
        assert_eq!(
            form.form
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            ["name", "email", "active"]
        );
        let list = configured_crud_columns(&program, &schema, crud, &crud.view.fields, |table| {
            table
                .columns
                .iter()
                .map(|column| column.name.clone())
                .collect()
        });
        assert_eq!(list, ["name", "email", "active"]);
    }

    #[test]
    fn rejects_invalid_soft_delete_columns() {
        let missing_source = r#"
            table machines {
                id: Id primary auto
                name: String(100) required
            }

            crud Machine -> machines {
                soft_delete { column: deleted_at }
            }
        "#;
        let missing_program = parse(&lex(missing_source).unwrap()).unwrap();
        let missing_schema = build_schema(&missing_program).unwrap();
        assert!(!validate_cruds(
            "test.zyl",
            &missing_program,
            &missing_schema
        ));

        let wrong_type_source = r#"
            table machines {
                id: Id primary auto
                name: String(100) required
                deleted_at: String?
            }

            crud Machine -> machines {
                soft_delete { column: deleted_at }
            }
        "#;
        let wrong_type_program = parse(&lex(wrong_type_source).unwrap()).unwrap();
        let wrong_type_schema = build_schema(&wrong_type_program).unwrap();
        assert!(!validate_cruds(
            "test.zyl",
            &wrong_type_program,
            &wrong_type_schema
        ));
    }

    #[test]
    fn rejects_protected_routes_without_auth_definition() {
        let source = r#"
            page "/admin" {
                requires auth
                html {
                    <h1>Admin</h1>
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(!validate_auth("test.zyl", &program, &schema));
    }

    #[test]
    fn rejects_scoped_crud_permissions_without_auth_definition() {
        let source = r#"
            table customers {
                id: Id primary auto
                name: String(100) required
            }

            crud Customer -> customers {
                permits create "customers.create"
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(!validate_auth("test.zyl", &program, &schema));
    }

    #[test]
    fn rejects_protected_form_action_without_auth_definition() {
        let source = r#"
            form CustomerForm {
                field name: String {
                    required
                }
                action save {
                    requires auth
                    permits "customers.save"
                }
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(!validate_auth("test.zyl", &program, &schema));
    }

    #[test]
    fn rejects_protected_api_without_auth_definition() {
        let source = r#"
            api GET "/admin" {
                requires auth
                output String
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(!validate_auth("test.zyl", &program, &schema));
    }

    #[test]
    fn accepts_persistent_auth_tables() {
        let source = r#"
            auth users {
                table: users
                sessions: auth_sessions
                permissions: user_permissions
                roles: user_roles
                role_permissions: role_permissions
            }

            table users {
                id: Id primary auto
                email: Email required
                password_hash: String(255) required
            }

            table auth_sessions {
                id: Id primary auto
                user: User required
                token_hash: String(64) required
                expires_at: Timestamp required
            }

            table user_permissions {
                id: Id primary auto
                user: User required
                permission: String(100) required
            }

            table user_roles {
                id: Id primary auto
                user: User required
                role: String(100) required
            }

            table role_permissions {
                id: Id primary auto
                role: String(100) required
                permission: String(100) required
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(validate_auth("test.zyl", &program, &schema));
    }

    #[test]
    fn rejects_partial_auth_role_configuration() {
        let source = r#"
            auth users {
                table: users
                roles: user_roles
            }

            table users {
                id: Id primary auto
                email: Email required
                password_hash: String(255) required
            }

            table user_roles {
                id: Id primary auto
                user: User required
                role: String(100) required
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(!validate_auth("test.zyl", &program, &schema));
    }

    #[test]
    fn rejects_chained_audit_without_hash_columns() {
        let source = r#"
            auth users {
                table: users
                audit: audit_log
                audit_chain: true
            }

            table users {
                id: Id primary auto
                email: Email required
                password_hash: String(255) required
            }

            table audit_log {
                id: Id primary auto
                actor_user_id: Int?
                event: String(100) required
                target_user_id: Int?
                details: String(1000) required
                created_at: Timestamp default now
            }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(!validate_auth("test.zyl", &program, &schema));
    }

    #[test]
    fn reads_project_capability_grants() {
        let grants = project_capability_grants("../examples/capabilities.zyl")
            .unwrap()
            .unwrap();
        assert!(grants.contains("Database"));
        assert!(grants.contains("Network"));
        assert!(grants.contains("Console"));
    }

    #[test]
    fn defaults_project_file_system_policy_to_project_root() {
        let policy = project_filesystem_policy("../examples/filesystem_api.zyl")
            .unwrap()
            .unwrap();
        let project_root = fs::canonicalize("..").unwrap();
        assert!(policy.read_roots.iter().any(|root| root == &project_root));
        assert!(policy.write_roots.is_empty());
    }

    #[test]
    fn defaults_project_network_policy_to_no_allowed_hosts() {
        let policy = project_network_policy("../examples/filesystem_api.zyl")
            .unwrap()
            .unwrap();
        assert!(policy.allowed_hosts.is_empty());
        assert_eq!(policy.timeout_ms, 5_000);
        assert_eq!(policy.max_response_bytes, 1_048_576);
    }

    #[test]
    fn defaults_project_process_policy_to_no_allowed_commands() {
        let policy = project_process_policy("../examples/filesystem_api.zyl")
            .unwrap()
            .unwrap();
        assert!(policy.allowed_commands.is_empty());
        assert_eq!(policy.timeout_ms, 5_000);
        assert_eq!(policy.max_output_bytes, 1_048_576);
    }

    #[test]
    fn generates_project_with_target_release_version() {
        let path = env::temp_dir().join(format!(
            "zelyra-cli-template-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let status = create_project(
            path.to_str().unwrap(),
            ProjectOptions {
                allow_current_directory: false,
                with_mariadb: true,
                crud_template: false,
                auth_template: false,
                business_template: false,
                web_port: DEFAULT_WEB_PORT,
                host_port: DEFAULT_WEB_PORT,
                database_host_port: DEFAULT_DATABASE_HOST_PORT,
                host_port_given: false,
                database_host_port_given: false,
            },
        );
        assert_eq!(status, ExitCode::SUCCESS);

        let dockerfile = fs::read_to_string(path.join("Dockerfile")).unwrap();
        let project_config = fs::read_to_string(path.join("zelyra.toml")).unwrap();
        let project_theme = fs::read_to_string(path.join(PROJECT_THEME_CSS_FILE)).unwrap();
        assert!(dockerfile.contains(&format!("ARG ZELYRA_REF=v{}", env!("CARGO_PKG_VERSION"))));
        assert!(project_config.contains(&format!("version = \"{}\"", env!("CARGO_PKG_VERSION"))));
        assert!(project_config.contains("console = false"));
        assert!(dockerfile.contains("COPY main.zyl zelyra.toml zelyra.theme.css ./"));
        assert!(dockerfile.contains("COPY src ./src"));
        assert!(path.join("src/.keep").is_file());
        assert!(dockerfile.contains("COPY locales ./locales"));
        assert!(path.join("locales/de.json").is_file());
        assert!(path.join("locales/en.json").is_file());
        for token in [
            "--zelyra-color-accent",
            "--zelyra-color-accent-strong",
            "--zelyra-color-accent-text",
            "--zelyra-color-accent-soft",
            "--zelyra-color-ink",
            "--zelyra-color-muted",
            "--zelyra-color-border",
            "--zelyra-color-canvas",
            "--zelyra-color-surface",
            "--zelyra-color-surface-subtle",
            "--zelyra-color-sidebar-start",
            "--zelyra-color-sidebar-middle",
            "--zelyra-color-sidebar-end",
            "--zelyra-color-sidebar-foreground",
            "--zelyra-color-sidebar-muted",
            "--zelyra-color-hero-start",
            "--zelyra-color-hero-middle",
            "--zelyra-color-hero-end",
            "--zelyra-color-success-background",
            "--zelyra-color-success-border",
            "--zelyra-color-success-ink",
            "--zelyra-color-danger-background",
            "--zelyra-color-danger-border",
            "--zelyra-color-danger-ink",
            "--zelyra-color-focus",
            "--zelyra-font-body",
            "--zelyra-radius-card",
            "--zelyra-radius-control",
            "--zelyra-content-max-width",
        ] {
            assert!(
                project_theme.contains(token),
                "generated theme misses {token}"
            );
        }

        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn mariadb_crud_scaffold_includes_the_fictional_demo_fixture() {
        let path = env::temp_dir().join(format!(
            "zelyra-cli-crud-demo-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let status = create_project(
            path.to_str().unwrap(),
            ProjectOptions {
                allow_current_directory: false,
                with_mariadb: true,
                crud_template: true,
                auth_template: false,
                business_template: false,
                web_port: DEFAULT_WEB_PORT,
                host_port: DEFAULT_WEB_PORT,
                database_host_port: DEFAULT_DATABASE_HOST_PORT,
                host_port_given: false,
                database_host_port_given: false,
            },
        );
        assert_eq!(status, ExitCode::SUCCESS);

        let fixture = fs::read_to_string(path.join("machine-management-demo.sql")).unwrap();
        assert!(fixture.contains("ZLY-DEMO-030"));
        assert!(fixture.contains("INSERT IGNORE INTO machines"));
        let source = fs::read_to_string(path.join("main.zyl")).unwrap();
        assert!(source.contains("machines.resources.title"));
        assert!(source.contains("mode: cards"));

        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn generated_mariadb_template_uses_selected_web_port() {
        let path = env::temp_dir().join(format!(
            "zelyra-cli-port-template-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut selected_ports = None;
        for _ in 0..8 {
            let database_host_port = find_free_port(34_000, &[]).unwrap();
            let host_port = find_free_port(35_000, &[database_host_port]).unwrap();
            let status = create_project(
                path.to_str().unwrap(),
                ProjectOptions {
                    allow_current_directory: false,
                    with_mariadb: true,
                    crud_template: false,
                    auth_template: false,
                    business_template: false,
                    web_port: 8080,
                    host_port,
                    database_host_port,
                    host_port_given: true,
                    database_host_port_given: true,
                },
            );
            if status == ExitCode::SUCCESS {
                selected_ports = Some((host_port, database_host_port));
                break;
            }
        }
        let (host_port, database_host_port) =
            selected_ports.expect("template test should acquire two free host ports");

        let env_example = fs::read_to_string(path.join(".env.example")).unwrap();
        let compose = fs::read_to_string(path.join("docker-compose.mariadb.yml")).unwrap();
        let dockerfile = fs::read_to_string(path.join("Dockerfile")).unwrap();
        let dockerignore = fs::read_to_string(path.join(".dockerignore")).unwrap();
        let gitignore = fs::read_to_string(path.join(".gitignore")).unwrap();
        assert!(env_example.contains("ZELYRA_WEB_PORT=8080"));
        assert!(env_example.contains(&format!("ZELYRA_HOST_PORT={host_port}")));
        assert!(env_example.contains(&format!("ZELYRA_DB_HOST_PORT={database_host_port}")));
        assert!(env_example.contains("ZELYRA_DB_CONNECT_TIMEOUT_SECS=10"));
        assert!(env_example.contains("ZELYRA_DB_QUERY_TIMEOUT_SECS=30"));
        assert!(env_example.contains("ZELYRA_DB_POOL_MAX_SIZE=8"));
        assert!(env_example.contains("ZELYRA_DB_POOL_WAIT_TIMEOUT_SECS=10"));
        assert!(env_example.contains("ZELYRA_DB_TLS_MODE=disabled"));
        assert!(compose
            .contains("ZELYRA_DB_CONNECT_TIMEOUT_SECS: ${ZELYRA_DB_CONNECT_TIMEOUT_SECS:-10}"));
        assert!(
            compose.contains("ZELYRA_DB_QUERY_TIMEOUT_SECS: ${ZELYRA_DB_QUERY_TIMEOUT_SECS:-30}")
        );
        assert!(compose.contains("ZELYRA_DB_POOL_MAX_SIZE: ${ZELYRA_DB_POOL_MAX_SIZE:-8}"));
        assert!(compose
            .contains("ZELYRA_DB_POOL_WAIT_TIMEOUT_SECS: ${ZELYRA_DB_POOL_WAIT_TIMEOUT_SECS:-10}"));
        assert!(compose.contains("ZELYRA_DB_TLS_MODE: ${ZELYRA_DB_TLS_MODE:-disabled}"));
        assert!(compose.contains("ZELYRA_DB_TLS_CA_CERT_FILE: ${ZELYRA_DB_TLS_CA_CERT_FILE:-}"));
        assert!(compose.contains("0.0.0.0:${ZELYRA_WEB_PORT:-8080}"));
        assert!(compose.contains(&format!(
            "127.0.0.1:${{ZELYRA_HOST_PORT:-{host_port}}}:${{ZELYRA_WEB_PORT:-8080}}"
        )));
        assert!(compose.contains(&format!(
            "127.0.0.1:${{ZELYRA_DB_HOST_PORT:-{database_host_port}}}:3306"
        )));
        assert!(compose.contains("0.0.0.0:${ZELYRA_WEB_PORT:-8080}"));
        assert!(dockerfile.contains("EXPOSE 8080"));
        assert!(dockerfile.contains("0.0.0.0:8080"));
        assert!(dockerignore.contains(".env"));
        assert!(dockerignore.contains(".env.*"));
        assert!(gitignore.contains(".env"));

        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn setup_accepts_mariadb_project_without_env_example() {
        let path = env::temp_dir().join(format!(
            "zelyra-cli-setup-config-only-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("zelyra.toml"),
            "[database.main]\nengine = \"mariadb\"\n",
        )
        .unwrap();

        assert_eq!(
            setup_project(path.to_str().unwrap(), &SetupOptions::default()),
            ExitCode::SUCCESS
        );
        let env_file = fs::read_to_string(path.join(".env")).unwrap();
        assert!(env_file.contains("ZELYRA_DATABASE_MAIN_URL=mariadb://zelyra:"));
        assert!(env_file.contains("MARIADB_ROOT_PASSWORD="));

        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn setup_action_prepare_is_idempotent_and_does_not_replace_credentials() {
        let path = env::temp_dir().join(format!(
            "zelyra-cli-setup-action-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("zelyra.toml"),
            "[database.main]\nengine = \"mariadb\"\n",
        )
        .unwrap();

        let first =
            setup_action(path.to_str().unwrap(), "prepare", &SetupOptions::default()).unwrap();
        let contents = fs::read_to_string(path.join(".env")).unwrap();
        let second =
            setup_action(path.to_str().unwrap(), "prepare", &SetupOptions::default()).unwrap();
        assert!(first.contains("created protected .env"));
        assert!(second.contains("kept existing .env"));
        assert_eq!(contents, fs::read_to_string(path.join(".env")).unwrap());

        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn setup_web_url_uses_the_effective_host_port() {
        let path = env::temp_dir().join(format!(
            "zelyra-cli-setup-web-url-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("zelyra.toml"),
            "[database.main]\nengine = \"mariadb\"\n",
        )
        .unwrap();
        fs::write(
            path.join(".env.example"),
            "# ZELYRA_HOST_PORT=18080\nZELYRA_DB_HOST_PORT=3308\n",
        )
        .unwrap();
        fs::write(path.join(".env"), "ZELYRA_HOST_PORT=18443\n").unwrap();

        let env_path = path.join(".env");
        let env_path = env_path.to_string_lossy();
        assert_eq!(
            project_web_url_with_host_port(&path, None, &env_path).unwrap(),
            "http://127.0.0.1:18443"
        );

        fs::write(path.join(".env"), "ZELYRA_DB_HOST_PORT=3308\n").unwrap();
        assert_eq!(
            project_web_url_with_host_port(&path, None, &env_path).unwrap(),
            "http://127.0.0.1:18080"
        );

        assert_eq!(
            project_web_url_with_host_port(&path, Some("18444"), "unused.env").unwrap(),
            "http://127.0.0.1:18444"
        );

        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn setup_web_requires_the_token_and_renders_the_local_actions() {
        let path = env::temp_dir().join(format!(
            "zelyra-cli-setup-web-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        let state = Arc::new(Mutex::new(SetupWebState {
            directory: path.clone(),
            token: "test-token".into(),
            message: String::new(),
        }));
        let request = |target: &str| zelyra_web::Request {
            method: "GET".into(),
            target: target.into(),
            path: "/".into(),
            headers: HashMap::new(),
            body: String::new(),
        };

        let forbidden = setup_web_response(&state, &request("/?token=wrong"));
        assert_eq!(forbidden.status, 403);
        let page = setup_web_response(&state, &request("/?token=test-token"));
        assert_eq!(page.status, 200);
        assert!(page.body.contains("Konfiguration vorbereiten"));
        assert!(page.body.contains("MariaDB und Anwendung starten"));

        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn web_port_validation_rejects_zero_and_non_numeric_values() {
        assert_eq!(parse_web_port("1"), Ok(1));
        assert_eq!(parse_web_port("65535"), Ok(65535));
        assert!(parse_web_port("0").is_err());
        assert!(parse_web_port("65536").is_err());
        assert!(parse_web_port("web").is_err());
        assert_eq!(parse_database_host_port("3308"), Ok(3308));
        assert!(parse_database_host_port("0").is_err());
        assert!(parse_database_host_port("database").is_err());
    }

    #[test]
    fn docker_compose_hint_is_actionable_and_platform_specific() {
        let hint = docker_compose_install_hint();
        assert!(hint.contains("Docker Compose is unavailable"));
        assert!(hint.contains("https://docs.docker.com/"));
        assert!(hint.contains("docker compose version"));
    }

    #[test]
    fn compose_start_errors_are_actionable_without_exposing_output() {
        let secret = "mariadb://zelyra:secret-value@127.0.0.1:3306/zelyra_app";
        let permission = compose_start_failure_message(
            DockerComposeCommand::Legacy,
            &format!("permission denied while connecting to docker.sock: {secret}"),
        );
        assert!(permission.contains("usermod -aG docker"));
        assert!(permission.contains("\n  newgrp docker\n  id -nG\n  docker ps\n"));
        assert!(permission.contains("Opening another terminal window alone"));
        assert!(!permission.contains(secret));

        let conflict = compose_start_failure_message(
            DockerComposeCommand::Plugin,
            "failed to bind host port: address already in use",
        );
        assert!(conflict.contains("port is already in use"));
        assert!(!conflict.contains("failed to bind"));
    }

    #[test]
    fn free_port_selection_skips_a_bound_port() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let occupied = listener.local_addr().unwrap().port();
        let selected = find_free_port(occupied, &[]).unwrap();
        assert_ne!(selected, occupied);
    }

    #[test]
    fn explicit_port_conflicts_are_rejected() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let occupied = listener.local_addr().unwrap().port();
        let error = resolve_host_port(occupied, true, "web host", true, &[])
            .expect_err("an explicitly occupied port must be rejected");
        assert!(error.contains("already in use"));
        assert!(error.contains("choose a different port"));
    }
}
