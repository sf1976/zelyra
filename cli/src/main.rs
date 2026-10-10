use rand_core::{OsRng, RngCore};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
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
    apply_mariadb, apply_postgres, apply_sqlite, build_schema, count_duplicate_value_groups,
    count_foreign_key_orphans, count_null_values, create_mariadb_database, diff, inspect_mariadb,
    inspect_postgres, inspect_sqlite, mariadb_schema_migration_lock_is_held, query_mariadb,
    sql::check_program as check_sql_program, table_has_rows, with_mariadb_schema_lock, Backend,
    DatabaseError, Query, QueryResult, QueryValue, Risk, Schema,
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

mod account_cli;
mod command_dispatch;
mod database_cli;
mod docs;
mod edit;
mod editor;
mod formatter;
mod holes;
mod impact;
mod module_cli;
mod project;
mod route_cli;
#[cfg(test)]
mod tests;
mod updater;
use account_cli::{audit_command, auth_command};
#[cfg(test)]
use account_cli::{audit_rows_csv, audit_rows_json};
use database_cli::{database_command, inspect_for_backend};
#[cfg(test)]
use database_cli::{schema_fingerprint, schema_plan_json};
use formatter::format_source;
use holes::collect_typed_holes;
use impact::{build_impact_with_modules, build_impact_with_sources, focus_impact};
#[cfg(test)]
use module_cli::publish_directory_no_replace;
use module_cli::{
    context_command, context_entry, module_command, module_declaration_owners,
    module_uses_database, project_name,
};

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
    eprintln!("  zelyra editor [directory] [--port <port>] starts the local browser editor");
    eprintln!("  routes: `zelyra routes <entry.zyl> [--format human|json]` lists declared routes and generated resource routes");
    eprintln!("  impact focus: use `--symbol <kind:name>` to inspect one known node");
    eprintln!("  module plan: `zelyra module plan <entry.zyl> <module.zyl|resource-id>` previews known dependencies");
    eprintln!("  module bundle: `zelyra module bundle <entry.zyl> <module.zyl|resource-id> --output <dir> [--dry-run] [--docker --compiler-ref <40-char-commit>]` plans or writes a checked experimental bundle");
    eprintln!("  module bundle --dry-run emits a machine-readable file plan without publishing the bundle");
    eprintln!("  doctor supports `--env-file <path>` for generated MariaDB projects");
    eprintln!("  setup supports `--database`, `--schema`, `--all`, `--host-port`, `--db-host-port`, and `--web [--port <port>]`");
    eprintln!("Zelyra {}\n\nUsage:\n  zelyra --version\n  zelyra version\n  zelyra update [--check]\n  zelyra new <directory> [--mariadb] [--template minimal|mariadb-crud|mariadb-auth|mariadb-business] [--web-port <port>] [--host-port <port>] [--db-host-port <port>]\n  zelyra init [directory] [--mariadb] [--template minimal|mariadb-crud|mariadb-auth|mariadb-business] [--web-port <port>] [--host-port <port>] [--db-host-port <port>]\n  zelyra setup [directory] [--database|--schema|--all] [--host-port <port>] [--db-host-port <port>]\n  zelyra setup [directory] --web [--port <port>]\n  zelyra check <file.zyl> [--format human|json]\n  zelyra fmt <file.zyl> [--check]\n  zelyra impact <file.zyl> [--format human|json]\n  zelyra edit --format=json [--apply] <change.json>\n  zelyra context <file.zyl> [--format human|json]\n  zelyra config <file.zyl> [--format human|json]\n  zelyra build <file.zyl>\n  zelyra run <file.zyl>\n  zelyra serve <file.zyl> [address]\n  zelyra module plan <entry.zyl> <module.zyl|resource-id>\n  zelyra module bundle <entry.zyl> <module.zyl|resource-id> --output <dir> [--docker --compiler-ref <40-character-commit>]\n  zelyra doctor [file.zyl] [--port <port>] [--json]\n  zelyra verify <file.zyl> [--json]\n  zelyra doc <file.zyl> [--openapi|--typescript]\n  zelyra auth hash-password [--stdin]\n  zelyra auth role <grant|revoke> <file.zyl> <user-id> <role>\n  zelyra auth role-permission <grant|revoke> <file.zyl> <role> <permission>\n  zelyra audit inspect <file.zyl> [--limit <n>]\n  zelyra audit export <file.zyl> [--limit <n>] [--format json|csv]\n  zelyra audit verify <file.zyl>\n  zelyra audit prune <file.zyl> --before <timestamp> [--confirm]\n  zelyra form validate <file.zyl> <FormName> [field=value ...]\n  zelyra db <create|setup|bootstrap|inspect|map|plan|apply> <file.zyl>", env!("CARGO_PKG_VERSION"));
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
        "Usage:\n  zelyra db create <file.zyl>\n  zelyra db setup <file.zyl>\n  zelyra db bootstrap <file.zyl>\n  zelyra db inspect <file.zyl>\n  zelyra db map <file.zyl> [--format=text|json]\n  zelyra db plan <file.zyl> [--format=text|json]\n  zelyra db apply <file.zyl> [--plan-id <sha256:...>] [--allow-risky]\n  zelyra db history <file.zyl> [--format=text|json]\n\nUse a plan id from `db plan --format=json` to refuse applying a plan if the database schema changed after review.\nThe read-only database map groups declared tables by source module and reports live tables without a declaration; module ownership is advisory.\nMariaDB migrations are journaled; use `db history` to inspect progress and interrupted runs.\n--allow-destructive remains available for DESTRUCTIVE plans only.\nA project database uses ZELYRA_DATABASE_<NAME>_URL (for example ZELYRA_DATABASE_MAIN_URL); DATABASE_URL remains a compatibility fallback."
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
# Optional password recovery mail delivery. Configure these values only when
# auth declares a reset_tokens table; credentials remain local secrets.
# ZELYRA_PUBLIC_BASE_URL=https://app.example.test
# ZELYRA_SMTP_HOST=mail.example.test
# ZELYRA_SMTP_PORT=465
# ZELYRA_SMTP_SECURITY=implicit_tls
# ZELYRA_SMTP_FROM=Zelyra <no-reply@example.test>
# ZELYRA_SMTP_USERNAME=
# ZELYRA_SMTP_PASSWORD=
# ZELYRA_RESET_DELIVERY_KEY=replace-with-64-hex-characters
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
            "`{}` is not configured for MariaDB. A project created with `zelyra init` runs without setup; to use MariaDB, create a project with `zelyra init <directory> --mariadb` or `zelyra new <directory> --mariadb`",
            directory.display()
        ));
    }
    Ok(mariadb_env_template(
        DEFAULT_WEB_PORT,
        DEFAULT_WEB_PORT,
        DEFAULT_DATABASE_HOST_PORT,
    ))
}

fn setup_directory(path: &str) -> Result<PathBuf, String> {
    let current = std::env::current_dir().ok();
    setup_directory_from(path, current.as_deref())
}

fn setup_directory_from(
    path: &str,
    current_directory: Option<&std::path::Path>,
) -> Result<PathBuf, String> {
    let directory = std::path::Path::new(path);
    if directory.is_dir() {
        return Ok(directory.to_path_buf());
    }

    // A leading slash is often added accidentally when a relative directory
    // name is copied into the command. Keep absolute-path semantics, but point
    // out the likely relative path when it exists from the current directory.
    if directory.is_absolute() {
        if let Ok(relative) = directory.strip_prefix(std::path::Path::new("/")) {
            let candidate = current_directory.map(|current| current.join(relative));
            if candidate.is_some_and(|candidate| candidate.is_dir()) {
                let suggestion = format!("./{}", relative.display());
                return Err(format!(
                    "project directory `{path}` does not exist; `{suggestion}` exists in the current directory, so use `zelyra setup {suggestion}` if that is the intended project"
                ));
            }
        }
    }

    Err(format!("project directory `{path}` does not exist"))
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
    let directory = match setup_directory(path) {
        Ok(directory) => directory,
        Err(error) => {
            eprintln!("error[E-SETUP-001]: {error}");
            return ExitCode::from(1);
        }
    };
    let env_file = directory.join(".env");
    let existed = env_file.exists();
    let setup = match ensure_local_env_file(&directory, options) {
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
      ZELYRA_PUBLIC_BASE_URL: ${ZELYRA_PUBLIC_BASE_URL:-}
      ZELYRA_SMTP_HOST: ${ZELYRA_SMTP_HOST:-}
      ZELYRA_SMTP_PORT: ${ZELYRA_SMTP_PORT:-}
      ZELYRA_SMTP_SECURITY: ${ZELYRA_SMTP_SECURITY:-implicit_tls}
      ZELYRA_SMTP_FROM: ${ZELYRA_SMTP_FROM:-}
      ZELYRA_SMTP_USERNAME: ${ZELYRA_SMTP_USERNAME:-}
      ZELYRA_SMTP_PASSWORD: ${ZELYRA_SMTP_PASSWORD:-}
      ZELYRA_RESET_DELIVERY_KEY: ${ZELYRA_RESET_DELIVERY_KEY:-}
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
RUN git init /zelyra \
    && git -C /zelyra remote add origin https://github.com/sf1976/zelyra.git \
    && git -C /zelyra fetch --depth=1 origin "$ZELYRA_REF" \
    && git -C /zelyra checkout --detach FETCH_HEAD
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
    let table_dependencies_valid = validate_module_table_dependencies(path, &loaded);
    let database_dependencies_valid = validate_module_database_dependencies(path, &loaded);
    if !table_dependencies_valid || !database_dependencies_valid {
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
    let mut table_definitions = HashMap::new();
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
        table_definitions.insert(table.name.as_str(), table);
    }

    let impact = build_impact_with_sources(&loaded.program, &loaded.sources, "");
    let mut violations = BTreeMap::<(String, String), (zelyra_ast::Span, BTreeSet<String>)>::new();
    let mut access_violations =
        BTreeMap::<(String, String), (zelyra_ast::Span, BTreeSet<String>)>::new();
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
        let kind = reference
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let from = reference
            .get("from")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let access_mode = match kind {
            "sql_table" | "page_data_sql" => Some(
                reference
                    .get("access")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
            ),
            "auth_table" => Some("read_write"),
            "table" if from.starts_with("crud:") || from.starts_with("form:") => Some("read_write"),
            "table" if from.starts_with("tableview:") => Some(
                reference
                    .get("access")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
            ),
            _ => None,
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
        if reaches_owner {
            if let Some(mode) = access_mode {
                let granted = table_definitions
                    .get(table_name)
                    .is_some_and(|table| table_access_granted(table, source_path, mode));
                if !granted {
                    let key = (source_path.to_owned(), table_name.to_owned());
                    let entry = access_violations
                        .entry(key)
                        .or_insert_with(|| (span, BTreeSet::new()));
                    entry.1.insert(mode.to_owned());
                }
            }
            continue;
        }

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
    for ((source_path, table_name), (span, accesses)) in &access_violations {
        let owner_path = table_owners[table_name.as_str()];
        let access = accesses.iter().cloned().collect::<Vec<_>>().join(", ");
        diagnostic_with_span(
            path,
            "E-MOD-021",
            &format!(
                "module `{source_path}` attempts {access} access to table `{table_name}`, owned by `{owner_path}`, without a matching table access grant; add the module path to the appropriate `access` list or expose a public operation from the owning module"
            ),
            *span,
        );
    }
    violations.is_empty() && access_violations.is_empty()
}

fn validate_module_database_dependencies(path: &str, loaded: &project::LoadedProject) -> bool {
    if loaded.program.databases.len() != 1 {
        return true;
    }
    let database = &loaded.program.databases[0];
    let owners = module_declaration_owners(&loaded.program);
    let database_node = format!("database:{}", database.name);
    let Some(database_module) = owners.get(&database_node) else {
        return true;
    };
    let import_graph = loaded
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
    let source_ids = loaded
        .sources
        .iter()
        .enumerate()
        .map(|(id, source)| (source.path.as_str(), id as u32))
        .collect::<HashMap<_, _>>();
    let mut valid = true;

    for module in &loaded.modules {
        if module.path == *database_module
            || !module_uses_database(&loaded.program, &module.path, &owners)
        {
            continue;
        }

        let mut pending = vec![module.path.as_str()];
        let mut visited = HashSet::new();
        let imports_database = loop {
            let Some(current) = pending.pop() else {
                break false;
            };
            if !visited.insert(current) {
                continue;
            }
            if current == database_module {
                break true;
            }
            pending.extend(import_graph.get(current).into_iter().flatten().copied());
        };
        if imports_database {
            continue;
        }

        valid = false;
        let span = source_ids
            .get(module.path.as_str())
            .copied()
            .map(|source_id| zelyra_ast::Span::new(0, 0, 1, 1).with_source_id(source_id))
            .unwrap_or_default();
        let entry_is_database_module = loaded
            .sources
            .first()
            .is_some_and(|entry| entry.path == *database_module);
        let hint = if entry_is_database_module {
            format!(
                "move `database {}` from the entry module into an importable source module",
                database.name
            )
        } else {
            format!("import `{database_module}` directly or through another module")
        };
        diagnostic_with_span(
            path,
            "E-MOD-022",
            &format!(
                "module `{}` uses the project database but its import graph does not include the database provider module `{database_module}`; {hint}",
                module.path
            ),
            span,
        );
    }

    valid
}

fn table_access_granted(table: &zelyra_ast::TableDef, module: &str, mode: &str) -> bool {
    let read = table.access.read.iter().any(|grant| grant == module);
    let write = table.access.write.iter().any(|grant| grant == module);
    let read_write = table.access.read_write.iter().any(|grant| grant == module);
    match mode {
        "read" => read || read_write,
        "write" => write || read_write,
        "read_write" => read_write || (read && write),
        // An unknown SQL access mode must never inherit separate read or write
        // grants; only the explicit combined grant is broad enough.
        "unknown" => read_write,
        _ => false,
    }
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
    if !route_cli::validate_no_collisions(path, &program) {
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
            let full_impact = build_impact_with_modules(
                &project.program,
                &project.sources,
                &project.modules,
                fallback_source,
            );
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
    let impact = build_impact_with_modules(
        &project.program,
        &project.sources,
        &project.modules,
        fallback_source,
    );
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
                                        let affected_effects = edit::affected_function_effects(
                                            &program,
                                            &result.operations,
                                            &result.changes,
                                        );
                                        preview = json!({
                                            "available": true,
                                            "apply_requested": apply_requested,
                                            "applied": applied,
                                            "entry": entry_display.clone(),
                                            "affected_files": [entry_display.clone()],
                                            "affected_effects": affected_effects,
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

fn project_uses_reserved_health_route(program: &zelyra_ast::Program) -> bool {
    let reserved_path = zelyra_web::HEALTH_LIVENESS_PATH;
    program
        .pages
        .iter()
        .any(|page| zelyra_web::route_pattern_matches_path(&page.path, reserved_path))
        || program
            .apis
            .iter()
            .any(|api| zelyra_web::route_pattern_matches_path(&api.path, reserved_path))
        || program.auth.iter().any(|auth| {
            auth.admin_path
                .as_deref()
                .is_some_and(|path| zelyra_web::route_pattern_matches_path(path, reserved_path))
        })
        || program.cruds.iter().any(|crud| {
            let base = format!("/{}", crud.table);
            [
                base.clone(),
                format!("{base}/new"),
                format!("{base}/{{id}}"),
                format!("{base}/{{id}}/edit"),
                format!("{base}/{{id}}/delete"),
                format!("{base}/{{id}}/restore"),
            ]
            .iter()
            .any(|pattern| zelyra_web::route_pattern_matches_path(pattern, reserved_path))
                || crud.actions.iter().any(|action| {
                    let pattern = format!("{base}/{{id}}/{}", action.name);
                    zelyra_web::route_pattern_matches_path(&pattern, reserved_path)
                })
        })
}

fn project_uses_reserved_account_sessions_route(program: &zelyra_ast::Program) -> bool {
    !program.auth.is_empty()
        && (program.pages.iter().any(|page| {
            zelyra_web::route_pattern_matches_path(&page.path, zelyra_web::ACCOUNT_SESSIONS_PATH)
        }) || program.apis.iter().any(|api| {
            zelyra_web::route_pattern_matches_path(&api.path, zelyra_web::ACCOUNT_SESSIONS_PATH)
        }) || program.cruds.iter().any(|crud| {
            let base = format!("/{}", crud.table);
            [
                base.clone(),
                format!("{base}/new"),
                format!("{base}/{{id}}"),
                format!("{base}/{{id}}/edit"),
                format!("{base}/{{id}}/delete"),
                format!("{base}/{{id}}/restore"),
            ]
            .iter()
            .any(|pattern| {
                zelyra_web::route_pattern_matches_path(pattern, zelyra_web::ACCOUNT_SESSIONS_PATH)
            }) || crud.actions.iter().any(|action| {
                let pattern = format!("{base}/{{id}}/{}", action.name);
                zelyra_web::route_pattern_matches_path(&pattern, zelyra_web::ACCOUNT_SESSIONS_PATH)
            })
        }) || program.auth.iter().any(|auth| {
            auth.admin_path.as_deref().is_some_and(|path| {
                zelyra_web::route_pattern_matches_path(path, zelyra_web::ACCOUNT_SESSIONS_PATH)
            })
        }))
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
    let directory = setup_directory(path)?;
    let setup = ensure_local_env_file(&directory, options)?;
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
        let web_url = project_web_url(&directory)?;
        messages.push(start_mariadb_compose(&directory)?);
        messages.push(format!("open: {web_url}"));
    }
    if matches!(action, "schema" | "all") {
        let schema_result = if directory.join("docker-compose.mariadb.yml").is_file() {
            run_container_schema_setup(&directory)
        } else {
            run_local_schema_setup(&directory)
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
    let directory = match setup_directory(path) {
        Ok(directory) => directory,
        Err(error) => {
            eprintln!("error[E-SETUP-001]: {error}");
            return ExitCode::from(1);
        }
    };
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
        println!("{}", docs::format_typescript_client(&program));
    } else {
        println!("{}", docs::format_openapi(&program));
    }
    ExitCode::SUCCESS
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
    if project_uses_reserved_health_route(program) {
        diagnostic(
            path,
            "E-WEB-005",
            &format!(
                "route `{}` is reserved for Zelyra liveness checks",
                zelyra_web::HEALTH_LIVENESS_PATH
            ),
            1,
            1,
        );
        return Err(());
    }
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
        let capability = match key.as_str() {
            "database_read" => "Database(read)",
            "database_write" => "Database(write)",
            _ => KNOWN_CAPABILITIES
                .iter()
                .copied()
                .find(|capability| capability.to_ascii_lowercase() == key)
                .ok_or_else(|| format!("unknown capability setting `{key}`"))?,
        };
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
        if let Some(membership_table_name) = &auth.membership_table {
            if membership_table_name == &auth.table
                || auth.session_table.as_ref() == Some(membership_table_name)
            {
                diagnostic_with_span(
                    path,
                    "E-TENANT-001",
                    "tenant memberships must use a dedicated table separate from users and sessions",
                    auth.span,
                );
                valid = false;
            }
            if schema.backend() != Backend::MariaDb {
                diagnostic_with_span(
                    path,
                    "E-TENANT-001",
                    "tenant memberships currently require the MariaDB runtime",
                    auth.span,
                );
                valid = false;
            }
            if auth.session_table.is_none() {
                diagnostic_with_span(
                    path,
                    "E-TENANT-001",
                    "tenant memberships require database-backed authentication sessions",
                    auth.span,
                );
                valid = false;
            }
            let Some(membership_table) = schema
                .tables
                .iter()
                .find(|candidate| candidate.name == *membership_table_name)
            else {
                diagnostic_with_span(
                    path,
                    "E-TENANT-001",
                    &format!("unknown tenant membership table `{membership_table_name}`"),
                    auth.span,
                );
                valid = false;
                continue;
            };
            for required_column in ["user_id", "tenant_id", "active"] {
                if !membership_table
                    .columns
                    .iter()
                    .any(|column| column.name == required_column)
                {
                    diagnostic_with_span(
                        path,
                        "E-TENANT-001",
                        &format!(
                            "tenant membership table `{membership_table_name}` requires column `{required_column}`"
                        ),
                        auth.span,
                    );
                    valid = false;
                }
            }
            for numeric_column in ["user_id", "tenant_id"] {
                if let Some(column) = membership_table
                    .columns
                    .iter()
                    .find(|column| column.name == numeric_column)
                {
                    if !tenant_sql_type_is_integer(&column.sql_type) || column.nullable {
                        diagnostic_with_span(
                            path,
                            "E-TENANT-001",
                            &format!(
                                "tenant membership column `{membership_table_name}.{numeric_column}` must be a non-null integer"
                            ),
                            auth.span,
                        );
                        valid = false;
                    }
                }
            }
            if let Some(column) = membership_table
                .columns
                .iter()
                .find(|column| column.name == "active")
            {
                let sql_type = column.sql_type.to_ascii_uppercase();
                if column.nullable || !(sql_type.contains("BOOL") || sql_type.contains("TINYINT")) {
                    diagnostic_with_span(
                        path,
                        "E-TENANT-001",
                        &format!(
                            "tenant membership column `{membership_table_name}.active` must be a non-null boolean"
                        ),
                        auth.span,
                    );
                    valid = false;
                }
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
        if let Some(reset_table_name) = &auth.reset_tokens_table {
            if schema.backend() != Backend::MariaDb {
                diagnostic(
                    path,
                    "E-AUTH-034",
                    "password reset currently requires the MariaDB runtime",
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
            }
            if auth.audit_table.is_none() {
                diagnostic(
                    path,
                    "E-AUTH-033",
                    "password reset requires an authentication audit table",
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
            }
            let Some(reset_table) = schema
                .tables
                .iter()
                .find(|candidate| candidate.name == *reset_table_name)
            else {
                diagnostic(
                    path,
                    "E-AUTH-030",
                    &format!(
                        "authentication refers to unknown password reset table {}",
                        reset_table_name
                    ),
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
                continue;
            };
            if reset_table_name == &auth.table {
                diagnostic(
                    path,
                    "E-AUTH-030",
                    "password reset tokens must use a separate table from user accounts",
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
            }
            for required_column in [
                "id",
                "user_id",
                "token_hash",
                "expires_at",
                "consumed_at",
                "delivery_payload",
                "delivery_retry_at",
            ] {
                if !reset_table
                    .columns
                    .iter()
                    .any(|column| column.name == required_column)
                {
                    diagnostic(
                        path,
                        "E-AUTH-031",
                        &format!(
                            "authentication password reset table {} requires column {}",
                            reset_table_name, required_column
                        ),
                        auth.span.line,
                        auth.span.column,
                    );
                    valid = false;
                }
            }
            let delivery_payload_is_compatible = reset_table
                .columns
                .iter()
                .find(|column| column.name == "delivery_payload")
                .is_some_and(|column| {
                    column.nullable && column.sql_type.eq_ignore_ascii_case("varchar(2048)")
                });
            if !delivery_payload_is_compatible {
                diagnostic(
                    path,
                    "E-AUTH-037",
                    &format!(
                        "authentication password reset table {} requires nullable delivery_payload VARCHAR(2048)",
                        reset_table_name
                    ),
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
            }
            let delivery_retry_at_is_compatible = reset_table
                .columns
                .iter()
                .find(|column| column.name == "delivery_retry_at")
                .is_some_and(|column| {
                    column.nullable && column.sql_type.eq_ignore_ascii_case("timestamp")
                });
            if !delivery_retry_at_is_compatible {
                diagnostic(
                    path,
                    "E-AUTH-038",
                    &format!(
                        "authentication password reset table {} requires nullable delivery_retry_at TIMESTAMP",
                        reset_table_name
                    ),
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
            }
            let token_hash_is_unique = reset_table
                .columns
                .iter()
                .any(|column| column.name == "token_hash" && column.unique)
                || reset_table
                    .indexes
                    .iter()
                    .chain(reset_table.uniques.iter())
                    .any(|index| index.unique && index.columns == ["token_hash"]);
            if !token_hash_is_unique {
                diagnostic(
                    path,
                    "E-AUTH-032",
                    &format!(
                        "authentication password reset table {} requires a unique token_hash",
                        reset_table_name
                    ),
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
            }
            if !reset_table.foreign_keys.iter().any(|foreign_key| {
                foreign_key.column == "user_id"
                    && foreign_key.referenced_table == auth.table
                    && foreign_key.referenced_column == "id"
            }) {
                diagnostic(
                    path,
                    "E-AUTH-036",
                    &format!(
                        "authentication password reset table {} requires user_id to reference {}.id",
                        reset_table_name, auth.table
                    ),
                    auth.span.line,
                    auth.span.column,
                );
                valid = false;
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

fn project_optional_setting(path: &str, key: &str) -> Result<Option<String>, String> {
    if let Ok(value) = env::var(key) {
        return Ok((!value.trim().is_empty()).then_some(value));
    }
    let source_path = std::path::Path::new(path);
    let project_directory = source_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let env_path = project_directory.join(".env");
    if !env_path.is_file() {
        return Ok(None);
    }
    let env_path_string = env_path.to_string_lossy();
    Ok(read_env_value(&env_path_string, key)?.filter(|value| !value.trim().is_empty()))
}

fn password_reset_mailer_from_environment(
    path: &str,
    database_url: &str,
    reset_table: &str,
) -> Result<zelyra_web::PasswordResetMailer, String> {
    let required = |name: &str| {
        project_optional_setting(path, name)?
            .ok_or_else(|| format!("{name} is required when password reset is enabled"))
    };
    let host = required("ZELYRA_SMTP_HOST")?;
    let security = project_optional_setting(path, "ZELYRA_SMTP_SECURITY")?
        .unwrap_or_else(|| "implicit_tls".into());
    let default_port = match security.as_str() {
        "implicit_tls" => 465,
        "starttls" => 587,
        "local_plaintext" => 25,
        _ => {
            return Err(
                "ZELYRA_SMTP_SECURITY must be implicit_tls, starttls, or local_plaintext".into(),
            )
        }
    };
    let port = project_optional_setting(path, "ZELYRA_SMTP_PORT")?
        .map(|value| {
            value
                .parse::<u16>()
                .ok()
                .filter(|port| *port > 0)
                .ok_or_else(|| "ZELYRA_SMTP_PORT must be between 1 and 65535".to_owned())
        })
        .transpose()?
        .unwrap_or(default_port);
    let username = project_optional_setting(path, "ZELYRA_SMTP_USERNAME")?;
    let password = project_optional_setting(path, "ZELYRA_SMTP_PASSWORD")?;
    let from = required("ZELYRA_SMTP_FROM")?;
    let base_url = required("ZELYRA_PUBLIC_BASE_URL")?;
    let delivery_key = required("ZELYRA_RESET_DELIVERY_KEY")?;
    zelyra_web::PasswordResetMailer::smtp_with_outbox(zelyra_web::PasswordResetSmtpConfig {
        host: &host,
        port,
        security: &security,
        username: username.as_deref(),
        password: password.as_deref(),
        from: &from,
        base_url: &base_url,
        database_url,
        reset_table,
        delivery_key: &delivery_key,
    })
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
        if let Some(tenant_column) = crud.tenant_column.as_deref() {
            let fail_tenant = |message: &str| {
                diagnostic_with_span(path, "E-TENANT-002", message, crud.span);
            };
            if !crud.requires_auth {
                fail_tenant("tenant-scoped CRUD must declare `requires auth`");
                valid = false;
            }
            if !program
                .auth
                .iter()
                .any(|auth| auth.membership_table.is_some() && auth.session_table.is_some())
            {
                fail_tenant(
                    "tenant-scoped CRUD requires auth with database sessions and `memberships`",
                );
                valid = false;
            }
            let table = schema
                .tables
                .iter()
                .find(|table| table.name == crud.table)
                .expect("CRUD table existence was checked above");
            match table
                .columns
                .iter()
                .find(|column| column.name == tenant_column)
            {
                Some(column)
                    if tenant_sql_type_is_integer(&column.sql_type) && !column.nullable => {}
                Some(_) => {
                    fail_tenant("tenant key column must be a non-null integer");
                    valid = false;
                }
                None => {
                    fail_tenant(&format!(
                        "tenant key column `{tenant_column}` does not exist in table `{}`",
                        crud.table
                    ));
                    valid = false;
                }
            }
            if !table.foreign_keys.is_empty() {
                fail_tenant(
                    "tenant-scoped CRUD does not yet support relations; remove foreign keys from this table",
                );
                valid = false;
            }
            if !crud.actions.is_empty() {
                fail_tenant(
                    "tenant-scoped CRUD does not support custom actions because their SQL is not tenant-scoped",
                );
                valid = false;
            }
            if crud
                .view
                .fields
                .iter()
                .chain(&crud.list)
                .chain(&crud.search)
                .chain(&crud.filters)
                .any(|column| column == tenant_column)
            {
                fail_tenant("tenant key must not be exposed as a CRUD field, list, search, or filter column");
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

fn tenant_sql_type_is_integer(sql_type: &str) -> bool {
    let sql_type = sql_type.to_ascii_uppercase();
    ["TINYINT", "SMALLINT", "MEDIUMINT", "INT", "BIGINT"]
        .iter()
        .any(|kind| sql_type.contains(kind))
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
    if !program.auth.is_empty() && project_uses_reserved_account_sessions_route(&program) {
        diagnostic(
            &path,
            "E-AUTH-029",
            &format!(
                "route `{}` is reserved for account session management",
                zelyra_web::ACCOUNT_SESSIONS_PATH
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
            membership_table: auth.membership_table.clone(),
            session_table: auth.session_table.clone(),
            permissions_table: auth.permissions_table.clone(),
            roles_table: auth.roles_table.clone(),
            role_permissions_table: auth.role_permissions_table.clone(),
            audit_table: auth.audit_table.clone(),
            audit_chain: auth.audit_chain,
            admin_path: auth.admin_path.clone(),
            admin_permission: auth.admin_permission.clone(),
            admin_role: auth.admin_role.clone(),
            login_rate_limit: auth.login_rate_limit.unwrap_or(zelyra_ast::ApiRateLimit {
                requests: 5,
                window_seconds: 15 * 60,
            }),
            login_block_seconds: auth.login_block_seconds.unwrap_or(60),
            reset_tokens_table: auth.reset_tokens_table.clone(),
            reset_rate_limit: auth
                .reset_rate_limit
                .unwrap_or(zelyra_web::DEFAULT_RESET_RATE_LIMIT),
            reset_block_seconds: auth
                .reset_block_seconds
                .unwrap_or(zelyra_web::DEFAULT_RESET_BLOCK_SECONDS),
            schema: schema.clone(),
            csrf,
        })
    } else {
        None
    };
    let reset_mailer = if auth_route
        .as_ref()
        .is_some_and(|auth| auth.reset_tokens_table.is_some())
    {
        let auth = auth_route.as_ref().expect("reset route was detected");
        let database_url = database_url_from_program(&program);
        let reset_table = auth
            .reset_tokens_table
            .as_deref()
            .expect("reset table was detected");
        match database_url
            .as_deref()
            .ok_or_else(|| "password reset requires a MariaDB database URL".to_owned())
            .and_then(|database_url| {
                password_reset_mailer_from_environment(&path, database_url, reset_table)
            }) {
            Ok(mailer) => Some(mailer),
            Err(error) => {
                eprintln!("error[E-AUTH-035]: {error}");
                return ExitCode::from(1);
            }
        }
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
            tenant_column: None,
            tenant_membership_table: None,
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
                    membership_table: auth_route
                        .as_ref()
                        .and_then(|auth| auth.membership_table.clone()),
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
                        membership_table: auth_route
                            .as_ref()
                            .and_then(|auth| auth.membership_table.clone()),
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
            tenant_column: crud
                .tenant_column
                .as_deref()
                .map(|column| storage_column_name(&schema, &crud.table, column)),
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
    let app = if let Some(reset_mailer) = reset_mailer {
        app.with_password_reset_mailer(reset_mailer)
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
    membership_table: Option<String>,
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
        .filter(|column| crud.tenant_column.as_deref() != Some(column.name.as_str()))
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
    let membership_guard = context.membership_table.as_deref().map(|membership_table| {
        format!(
            " AND EXISTS (SELECT 1 FROM {} AS zelyra_membership WHERE zelyra_membership.user_id = :zelyra_user_id AND zelyra_membership.tenant_id = :zelyra_tenant_id AND zelyra_membership.active = true)",
            quote_identifier(membership_table)
        )
    });
    let query = if edit {
        let assignments = storage_columns
            .iter()
            .zip(&fields)
            .map(|(column, field)| format!("{} = :{}", quote_identifier(column), field.name))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "UPDATE {} SET {} WHERE {} = :id{}{}",
            quote_identifier(&crud.table),
            assignments,
            quote_identifier("id"),
            crud.tenant_column
                .as_deref()
                .map_or_else(String::new, |column| {
                    format!(
                        " AND {} = :zelyra_tenant_id",
                        quote_identifier(&storage_column_name(schema, &crud.table, column))
                    )
                }),
            membership_guard.clone().unwrap_or_default()
        )
    } else {
        let mut insert_columns = storage_columns.clone();
        let mut insert_values = fields
            .iter()
            .map(|field| format!(":{}", field.name))
            .collect::<Vec<_>>();
        if let Some(tenant_column) = crud.tenant_column.as_deref() {
            insert_columns.push(storage_column_name(schema, &crud.table, tenant_column));
            insert_values.push(":zelyra_tenant_id".into());
        }
        let insert_select = if crud.tenant_column.is_some() {
            format!(
                " SELECT {} WHERE true{}",
                insert_values.join(", "),
                membership_guard.unwrap_or_default()
            )
        } else {
            format!(" VALUES ({})", insert_values.join(", "))
        };
        format!(
            "INSERT INTO {} ({}){}",
            quote_identifier(&crud.table),
            insert_columns
                .iter()
                .map(|column| quote_identifier(column))
                .collect::<Vec<_>>()
                .join(", "),
            insert_select
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
        tenant_column: crud
            .tenant_column
            .as_deref()
            .map(|column| storage_column_name(schema, &crud.table, column)),
        tenant_membership_table: context.membership_table,
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
            tenant_column: crud
                .tenant_column
                .as_deref()
                .map(|column| storage_column_name(schema, &crud.table, column)),
            tenant_membership_table: context.membership_table,
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
            let api_version = api.version.clone();
            let api_deprecated = api.deprecated;
            let api_rate_limit = api.rate_limit;
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
                .with_auth(requires_auth, permissions)
                .with_metadata(api_version, api_deprecated, api_rate_limit),
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

fn main() -> ExitCode {
    command_dispatch::run()
}
