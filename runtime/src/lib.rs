use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use zelyra_ast::temporal::{
    date_epoch_days, epoch_days_date, format_timestamp_utc, timestamp_bounds_millis, valid_date,
    valid_time,
};
use zelyra_ast::*;

pub use host::{
    HostFileMetadata, HostHttpResponse, HostProcessResult, HostServices, SystemHostServices,
    HOST_SERVICES_API_VERSION,
};

const BASE64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const MAX_HTTP_HEADER_BYTES: usize = 64 * 1024;
/// Maximum configured call depth shared by project settings and experimental backends.
pub const MAX_CONFIGURED_FUNCTION_CALL_DEPTH: usize = 500;
/// Maximum number of simultaneously spawned branches in a `parallel` block.
/// Each branch receives a bounded runtime thread stack, so this also bounds
/// the per-block stack reservation across AST, MIR, and bytecode execution.
pub const MAX_PARALLEL_BRANCHES: usize = 4;

fn base64_encode_bytes(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        out.push(BASE64_CHARS[(b0 >> 2) as usize] as char);
        out.push(BASE64_CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(BASE64_CHARS[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(BASE64_CHARS[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn base64_decode_bytes(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim_end_matches('=');
    let mut bytes = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for c in input.chars() {
        if c.is_ascii_whitespace() {
            continue;
        }
        let val = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 26,
            '0'..='9' => c as u32 - '0' as u32 + 52,
            '+' => 62,
            '/' => 63,
            _ => return Err(format!("invalid base64 character: `{c}`")),
        };
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(bytes)
}

fn base64_decode_str(input: &str) -> Result<String, String> {
    String::from_utf8(base64_decode_bytes(input)?)
        .map_err(|error| format!("invalid UTF-8 in base64 payload: {error}"))
}

fn url_encode_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.as_bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            _ => {
                use std::fmt::Write;
                let _ = write!(out, "%{:02X}", byte);
            }
        }
    }
    out
}

fn url_decode_str(s: &str) -> Result<String, String> {
    let mut bytes = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' {
            if i + 2 >= chars.len() {
                return Err("incomplete percent encoding".into());
            }
            let hex_str: String = chars[i + 1..=i + 2].iter().collect();
            let b = u8::from_str_radix(&hex_str, 16)
                .map_err(|_| format!("invalid hex in percent encoding: `%{hex_str}`"))?;
            bytes.push(b);
            i += 3;
        } else if chars[i] == '+' {
            bytes.push(b' ');
            i += 1;
        } else {
            let mut buf = [0u8; 4];
            let encoded = chars[i].encode_utf8(&mut buf);
            bytes.extend_from_slice(encoded.as_bytes());
            i += 1;
        }
    }
    String::from_utf8(bytes).map_err(|e| format!("invalid UTF-8 in url decoded content: {e}"))
}

const MAX_TEXT_BUILTIN_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

/// Generate an unbiased integer from an inclusive range using the shared host
/// random provider. Callers must perform the language capability check first.
pub fn random_int_with_host_services(
    host_services: &dyn HostServices,
    minimum: i64,
    maximum: i64,
) -> Result<i64, String> {
    if minimum > maximum {
        return Err("random_int requires minimum <= maximum".into());
    }
    let range = (i128::from(maximum) - i128::from(minimum) + 1) as u128;
    let sample_space = u128::from(u64::MAX) + 1;
    let cutoff = sample_space - (sample_space % range);
    loop {
        let mut bytes = [0_u8; 8];
        host_services.fill_random(&mut bytes)?;
        let candidate = u128::from(u64::from_ne_bytes(bytes));
        if candidate < cutoff {
            let value = i128::from(minimum) + (candidate % range) as i128;
            return Ok(value as i64);
        }
    }
}

/// Generate an RFC 4122 version 4 UUID from the shared host random provider.
/// Callers must perform the language capability check first.
pub fn uuid_v4_with_host_services(host_services: &dyn HostServices) -> Result<String, String> {
    let mut bytes = [0u8; 16];
    host_services.fill_random(&mut bytes)?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    ))
}

mod api_validation;
mod value;
pub(crate) use api_validation::contains_class_reference_type;
pub use api_validation::{check_apis, ApiDiagnostic};
pub(crate) use value::RuntimeClassObject;
pub use value::{normalize_map_entries, ClassObjectRef, FunctionValue, SqlRowsMetadata, Value};
mod verifier;
pub use verifier::{verify, ContractKind, VerificationResult, VerificationStatus};
mod type_checker;
pub use type_checker::{check, check_with_inferred_types};
pub(crate) use type_checker::{
    compatible, expand_type_aliases, infer_generic_types, singular_table_type,
    substitute_generic_type,
};
mod capabilities;
pub(crate) use capabilities::function_declares_capability;
pub use capabilities::{
    builtin_capability_requirements, check_capabilities, check_capabilities_with_grants,
    inspect_capability_uses, project_grants_capability, sql_required_capability, CapabilityError,
    CapabilityUse, FilePathUse,
};

/// Fail closed on SQL that could bypass a declared tenant boundary. The
/// caller supplies scope metadata from the checked project; unsupported SQL
/// forms and quoted table names are rejected whenever scoped tables exist.
pub fn check_scoped_sql(query: &str, scoped_tables: &[String]) -> Result<(), String> {
    if scoped_tables.is_empty() {
        return Ok(());
    }
    let (operation, accesses) = zelyra_database::sql::analyze_table_access(query, scoped_tables);
    if operation.is_none() {
        return Err("SQL form cannot be verified against declared tenant scopes".into());
    }
    if let Some(access) = accesses.into_iter().next() {
        if access.table.is_empty() {
            return Err(
                "quoted SQL table references cannot be verified against tenant scopes".into(),
            );
        }
        return Err(format!(
            "direct SQL access to tenant-scoped table `{}` requires a checked tenant query",
            access.table
        ));
    }
    Ok(())
}
mod ast_interpreter;
pub use ast_interpreter::execution_api::{
    execute, execute_captured_with_capabilities_and_policies,
    execute_captured_with_database_and_capabilities_and_policies,
    execute_captured_with_database_and_host_services, execute_captured_with_host_services,
    execute_function, execute_function_captured_with_capabilities_and_policies,
    execute_function_captured_with_host_services_and_capabilities_and_policies,
    execute_function_captured_with_mariadb_transaction,
    execute_function_captured_with_transactional_query, execute_function_with_capabilities,
    execute_function_with_capabilities_and_filesystem_policy,
    execute_function_with_capabilities_and_policies,
    execute_function_with_host_services_and_capabilities_and_policies, execute_with_capabilities,
    execute_with_capabilities_and_filesystem_policy, execute_with_capabilities_and_policies,
    execute_with_database, execute_with_database_and_capabilities,
    execute_with_database_and_capabilities_and_filesystem_policy,
    execute_with_database_and_capabilities_and_policies, execute_with_database_and_host_services,
    execute_with_host_services,
};
/// Decode native database rows using the same checked projection used by the
/// AST, MIR, and bytecode execution backends.
pub use ast_interpreter::sql_results::decode_native_sql_rows;
/// Return synthetic record metadata for table-derived SQL row projections.
pub use ast_interpreter::sql_results::projection_records as sql_projection_records;
use ast_interpreter::{AstInterpreter as Interpreter, Environment};
/// Default language-level function call depth for all execution backends.
pub const DEFAULT_FUNCTION_CALL_DEPTH: usize = 20;
/// Return the effective function-call limit, applying the shared hard ceiling.
pub fn effective_function_call_depth(configured_depth: Option<usize>) -> usize {
    configured_depth
        .unwrap_or(DEFAULT_FUNCTION_CALL_DEPTH)
        .min(MAX_CONFIGURED_FUNCTION_CALL_DEPTH)
}

/// Size a runtime worker stack for the bounded recursive AST/MIR execution.
pub fn runtime_stack_size_for_depth(depth: usize) -> usize {
    2 * 1024 * 1024 + depth.min(MAX_CONFIGURED_FUNCTION_CALL_DEPTH) * 192 * 1024
}

mod csv;
mod decimal_math;
mod filesystem;
mod host;
mod mail;
mod markdown;
mod operators;
mod package_settings;
mod policies;
mod repl;
pub use policies::{FileSystemPolicy, MailPolicy, NetworkPolicy, ProcessPolicy, RuntimePolicy};
mod pdf;
mod pdf_template;
mod pure_builtins;
mod step_budget;
pub use mail::{prepare_mail_queue_query, supports_mail_outbox_database};
pub use markdown::render_markdown;
pub use operators::{evaluate_binary_value, evaluate_unary_value};
pub use package_settings::PackageSettings;
use pdf_template::interpolate_pdf_data;
pub use pure_builtins::{execute_pure_builtin, is_pure_builtin};
pub use repl::{ReplError, ReplEvaluation, ReplExecutionContext, ReplSession};
pub use step_budget::{StepBudget, StepBudgetError};

/// Render a declared view with supplied record data without executing project code.
/// The same template and PDF pipeline is used by the `render_pdf` builtin.
pub fn render_pdf_view(view: &ViewDef, data: &Value) -> Result<Vec<u8>, &'static str> {
    let html = view.html.replace("<slot />", "").replace("<slot/>", "");
    let html = interpolate_pdf_data(&html, data)?;
    pdf::render_html_to_pdf(&html)
}
use ast_interpreter::sql_results;

#[derive(Clone, Debug, PartialEq)]
pub struct TypeError {
    pub code: &'static str,
    pub message: String,
    pub span: Span,
}

/// The inferred type for one expression in a successfully checked program.
///
/// Spans refer to the original source, so downstream representations can keep
/// precise diagnostics and editor navigation without re-running inference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferredExpressionType {
    pub span: Span,
    pub ty: Type,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub const KNOWN_CAPABILITIES: &[&str] = &[
    "Database",
    "Database(read)",
    "Database(write)",
    "Email",
    "Network",
    "FileSystem",
    "Environment",
    "Process",
    "Clock",
    "Random",
    "Console",
];

/// Convert an already resolved set of SQL placeholder locals into typed
/// database parameters using the AST runtime's scalar-value rules.
pub fn sql_parameters_from_named_values(
    query: &str,
    values: &HashMap<String, Value>,
    span: Span,
) -> Result<Vec<(String, zelyra_database::QueryValue)>, RuntimeError> {
    let names = zelyra_database::sql_parameter_names(query).map_err(|error| RuntimeError {
        message: format!("could not scan SQL parameters: {error}"),
        span,
    })?;
    let mut parameters = Vec::new();
    for name in names {
        if parameters.iter().any(|(existing, _)| existing == &name) {
            continue;
        }
        let value = values.get(&name).ok_or_else(|| RuntimeError {
            message: format!("SQL parameter `:{name}` is not available"),
            span,
        })?;
        parameters.push((name, value_to_query_value(value, span)?));
    }
    Ok(parameters)
}

/// Check declared storage compatibility through the configured host without
/// changing any database schema.
pub fn validate_database_storage_with_host(
    database_url: &str,
    schema: &Result<zelyra_database::Schema, Vec<zelyra_database::SchemaError>>,
    host: &dyn HostServices,
    span: Span,
) -> Result<(), RuntimeError> {
    let schema = schema.as_ref().map_err(|errors| RuntimeError {
        message: errors
            .first()
            .map_or("invalid database schema", |error| error.message.as_str())
            .to_owned(),
        span,
    })?;
    host.validate_database_storage(database_url, schema)
        .map_err(|error| RuntimeError {
            message: error,
            span,
        })
}

/// Execute a pre-parsed, capability-checked SQL query through the same host
/// service boundary as the AST interpreter.
pub fn execute_database_query_with_host(
    database_url: &str,
    query: &zelyra_database::Query,
    host: &dyn HostServices,
    span: Span,
) -> Result<zelyra_database::QueryResult, RuntimeError> {
    host.execute_database_query(database_url, query)
        .map_err(|error| RuntimeError {
            message: format!("database query failed: {error}"),
            span,
        })
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeError {
    pub message: String,
    pub span: Span,
}

/// Inspect a literal path with the exact canonicalization and root rules used
/// by file-system builtins. Dynamic paths cannot be certified by this helper.
pub fn inspect_filesystem_path(
    policy: Option<&FileSystemPolicy>,
    path: &str,
    write: bool,
    allow_missing: bool,
) -> Result<PathBuf, String> {
    let roots = policy.map_or_else(Vec::new, |policy| {
        if write {
            policy.write_roots.clone()
        } else {
            policy.read_roots.clone()
        }
    });
    resolve_filesystem_path_rule(
        policy,
        path,
        &roots,
        if write { "write" } else { "read" },
        allow_missing,
        None,
    )
}

/// Inspect a file-system path using the same root and symlink checks as the
/// runtime while delegating canonicalization to the configured host. MIR and
/// other execution backends must use this helper instead of implementing their
/// own path-containment rules.
pub fn inspect_filesystem_path_with_host(
    policy: Option<&FileSystemPolicy>,
    path: &str,
    write: bool,
    allow_missing: bool,
    host_services: &dyn HostServices,
) -> Result<PathBuf, String> {
    let roots = policy.map_or_else(Vec::new, |policy| {
        if write {
            policy.write_roots.clone()
        } else {
            policy.read_roots.clone()
        }
    });
    resolve_filesystem_path_rule(
        policy,
        path,
        &roots,
        if write { "write" } else { "read" },
        allow_missing,
        Some(host_services),
    )
}

/// Inspect a directory path using the configured read roots and the same
/// canonicalization rules as file reads, while retaining directory-listing
/// diagnostics.
pub fn inspect_filesystem_directory_with_host(
    policy: Option<&FileSystemPolicy>,
    path: &str,
    host_services: &dyn HostServices,
) -> Result<PathBuf, String> {
    let roots = policy.map_or_else(Vec::new, |policy| policy.read_roots.clone());
    resolve_filesystem_path_rule(policy, path, &roots, "list", false, Some(host_services))
}

/// Inspect a file targeted by a deletion operation using configured write
/// roots and deletion-specific diagnostics.
pub fn inspect_filesystem_delete_path_with_host(
    policy: Option<&FileSystemPolicy>,
    path: &str,
    host_services: &dyn HostServices,
) -> Result<PathBuf, String> {
    let roots = policy.map_or_else(Vec::new, |policy| policy.write_roots.clone());
    resolve_filesystem_path_rule(policy, path, &roots, "delete", false, Some(host_services))
}

/// Inspect a file-copy source using read roots and the AST's shared operation
/// diagnostic.
pub fn inspect_filesystem_copy_source_with_host(
    policy: Option<&FileSystemPolicy>,
    path: &str,
    host_services: &dyn HostServices,
) -> Result<PathBuf, String> {
    inspect_filesystem_path_for_host(policy, path, false, "read or rename", false, host_services)
}

/// Inspect a rename source using write roots and the AST's shared operation
/// diagnostic.
pub fn inspect_filesystem_rename_source_with_host(
    policy: Option<&FileSystemPolicy>,
    path: &str,
    host_services: &dyn HostServices,
) -> Result<PathBuf, String> {
    inspect_filesystem_path_for_host(policy, path, true, "read or rename", false, host_services)
}

/// Inspect a copy or rename destination using write roots and allowing a new
/// path, matching the AST runtime's path-resolution rules.
pub fn inspect_filesystem_copy_or_rename_destination_with_host(
    policy: Option<&FileSystemPolicy>,
    path: &str,
    host_services: &dyn HostServices,
) -> Result<PathBuf, String> {
    inspect_filesystem_path_for_host(policy, path, true, "copy or rename", true, host_services)
}

fn inspect_filesystem_path_for_host(
    policy: Option<&FileSystemPolicy>,
    path: &str,
    write: bool,
    operation: &str,
    allow_missing: bool,
    host_services: &dyn HostServices,
) -> Result<PathBuf, String> {
    let roots = policy.map_or_else(Vec::new, |policy| {
        if write {
            policy.write_roots.clone()
        } else {
            policy.read_roots.clone()
        }
    });
    resolve_filesystem_path_rule(
        policy,
        path,
        &roots,
        operation,
        allow_missing,
        Some(host_services),
    )
}

fn resolve_filesystem_path_rule(
    policy: Option<&FileSystemPolicy>,
    path: &str,
    roots: &[PathBuf],
    operation: &str,
    allow_missing: bool,
    host_services: Option<&dyn HostServices>,
) -> Result<PathBuf, String> {
    let Some(policy) = policy else {
        return Ok(PathBuf::from(path));
    };
    let candidate = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        policy.base_dir.join(path)
    };
    let resolved = match host_services {
        Some(host) => host
            .canonicalize_path(&candidate, allow_missing)
            .map_err(|error| format!("file-system {operation} failed for {path}: {error}"))?,
        None if allow_missing && !candidate.exists() && !candidate.is_symlink() => {
            let Some(file_name) = candidate.file_name() else {
                return Err(format!("file-system {operation} requires a file path"));
            };
            let parent = candidate.parent().unwrap_or(Path::new("."));
            let parent = std::fs::canonicalize(parent)
                .map_err(|error| format!("file-system {operation} failed for {path}: {error}"))?;
            parent.join(file_name)
        }
        None => std::fs::canonicalize(&candidate)
            .map_err(|error| format!("file-system {operation} failed for {path}: {error}"))?,
    };
    if roots
        .iter()
        .any(|root| resolved == *root || resolved.starts_with(root))
    {
        Ok(resolved)
    } else {
        Err(format!(
            "file-system {operation} denied for {path}: path is outside the configured roots"
        ))
    }
}

/// Execute one HTTP request using the same URL, host allow-list, header, timeout,
/// response-size, and UTF-8 checks as the AST runtime. Backends must perform
/// their language-level capability check before calling this helper.
pub fn execute_http_request_with_host(
    method: &str,
    url: &str,
    request_headers: &[String],
    request_body: Option<&str>,
    network_policy: Option<&NetworkPolicy>,
    host_services: &dyn HostServices,
) -> Result<HostHttpResponse, String> {
    if !matches!(method, "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD") {
        return Err(format!("http_request does not support method `{method}`"));
    }
    if matches!(method, "GET" | "HEAD") && request_body.is_some() {
        return Err(format!("http_request does not allow a body with {method}"));
    }
    let Some((scheme, authority_and_path)) = url.split_once("://") else {
        return Err("http_request supports only http:// and https:// URLs".into());
    };
    if !matches!(scheme, "http" | "https") {
        return Err("http_request supports only http:// and https:// URLs".into());
    }
    let authority_end = authority_and_path
        .find(['/', '?', '#'])
        .unwrap_or(authority_and_path.len());
    let authority = &authority_and_path[..authority_end];
    if authority.is_empty() || authority.contains('@') || authority.contains(['\r', '\n']) {
        return Err("http_request received an invalid URL".into());
    }
    let host = if let Some((host, port)) = authority.rsplit_once(':') {
        if port.parse::<u16>().is_err() {
            return Err("http_request received an invalid port".into());
        }
        host
    } else {
        authority
    };
    if host.is_empty() || host.contains(['/', '?', '#', '[', ']', '\\']) {
        return Err("http_request received an invalid host".into());
    }
    if let Some(policy) = network_policy {
        let authority_allowed = policy
            .allowed_hosts
            .iter()
            .any(|allowed| allowed == authority || allowed == host);
        if !authority_allowed {
            return Err(format!("network access denied for host `{authority}`"));
        }
    }
    let timeout_ms = network_policy.map_or(5_000, |policy| policy.timeout_ms);
    let maximum = network_policy.map_or(1_048_576, |policy| policy.max_response_bytes);
    if timeout_ms == 0 {
        return Err("http_request requires a positive timeout".into());
    }
    if maximum == 0 {
        return Err("http_request requires a positive maximum response size".into());
    }
    let mut headers = Vec::with_capacity(request_headers.len());
    let mut request_header_bytes = 0_usize;
    for header in request_headers {
        let Some((name, value)) = header.split_once(':') else {
            return Err("http_request headers must use the `Name: value` format".into());
        };
        let name = name.trim();
        let value = value.trim();
        if name.is_empty() || value.contains(['\r', '\n']) {
            return Err("http_request received an invalid header".into());
        }
        let name = ureq::http::HeaderName::try_from(name)
            .map_err(|_| "http_request received an invalid header".to_string())?;
        let value = ureq::http::HeaderValue::try_from(value)
            .map_err(|_| "http_request received an invalid header".to_string())?;
        let value = value
            .to_str()
            .map_err(|_| "http_request received an invalid header".to_string())?;
        request_header_bytes = request_header_bytes
            .saturating_add(name.as_str().len())
            .saturating_add(value.len())
            .saturating_add(4);
        if request_header_bytes > MAX_HTTP_HEADER_BYTES {
            return Err("http_request headers exceed the 65536 byte limit".into());
        }
        headers.push((name.to_string(), value.to_owned()));
    }
    let response = host_services.http_request(
        method,
        url,
        &headers,
        request_body.map(str::as_bytes),
        timeout_ms,
        maximum,
    )?;
    if !(100..=599).contains(&response.status) {
        return Err("http_request received an invalid status".into());
    }
    let response_header_bytes = response
        .headers
        .iter()
        .fold(0_usize, |total, (name, value)| {
            total
                .saturating_add(name.len())
                .saturating_add(value.len())
                .saturating_add(4)
        });
    if response_header_bytes > MAX_HTTP_HEADER_BYTES {
        return Err("network response headers exceed the 65536 byte limit".into());
    }
    let content_length = response
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<usize>().ok());
    if content_length.is_some_and(|content_length| content_length > maximum)
        || response.body.len() > maximum
    {
        return Err(format!("network response exceeds the {maximum} byte limit"));
    }
    std::str::from_utf8(&response.body)
        .map_err(|_| "network response is not valid UTF-8".to_string())?;
    Ok(response)
}

/// Convert a validated injected HTTP response into Zelyra's public
/// `HttpResponse` value shape. AST and alternative backends share this mapping.
pub fn http_response_value_from_host(response: HostHttpResponse) -> Result<Value, String> {
    let headers = response
        .headers
        .into_iter()
        .map(|(name, value)| format!("{name}: {value}"))
        .map(Value::String)
        .collect();
    let body = String::from_utf8(response.body)
        .map_err(|_| "network response is not valid UTF-8".to_owned())?;
    Ok(Value::Object {
        type_name: "HttpResponse".into(),
        type_args: Vec::new(),
        fields: HashMap::from([
            ("status".into(), Value::Int(i64::from(response.status))),
            ("headers".into(), Value::Array(headers)),
            ("body".into(), Value::String(body)),
        ]),
    })
}

/// Build the typed response envelope used by API handlers. Header names and
/// values are checked here so AST, MIR, and Bytecode reject injection and
/// hop-by-hop headers consistently before a response reaches the web server.
pub fn api_response_value(status: i64, headers: Vec<String>, body: Value) -> Result<Value, String> {
    if !(200..=299).contains(&status) {
        return Err("api_response status must be between 200 and 299; use the API `errors` contract for failures".into());
    }
    let mut checked_headers = Vec::with_capacity(headers.len());
    let mut content_types = 0usize;
    for header in headers {
        let Some((name, value)) = header.split_once(':') else {
            return Err("api_response header must use `Name: value` format".into());
        };
        let name = name.trim();
        let value = value.trim();
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
            || value
                .bytes()
                .any(|byte| byte.is_ascii_control() && byte != b'\t')
        {
            return Err("api_response contains an invalid header name or value".into());
        }
        if [
            "content-length",
            "transfer-encoding",
            "connection",
            "keep-alive",
            "trailer",
            "upgrade",
            "set-cookie",
        ]
        .iter()
        .any(|forbidden| name.eq_ignore_ascii_case(forbidden))
        {
            return Err(
                "api_response cannot set a hop-by-hop, content-length, or cookie header".into(),
            );
        }
        if !name.eq_ignore_ascii_case("content-type")
            && ![
                "cache-control",
                "content-disposition",
                "content-language",
                "etag",
                "last-modified",
                "link",
                "location",
                "retry-after",
                "vary",
                "x-request-id",
            ]
            .iter()
            .any(|allowed| name.eq_ignore_ascii_case(allowed))
        {
            return Err(format!(
                "api_response header `{name}` is not in the supported response header allowlist"
            ));
        }
        if name.eq_ignore_ascii_case("location")
            && (!value.starts_with('/')
                || value.starts_with("//")
                || value.contains('\\')
                || value.chars().any(char::is_control))
        {
            return Err("api_response Location must be a same-origin absolute path".into());
        }
        if name.eq_ignore_ascii_case("content-type") {
            content_types += 1;
            if content_types > 1 {
                return Err("api_response may set Content-Type only once".into());
            }
        }
        checked_headers.push(Value::String(format!("{name}: {value}")));
    }
    Ok(Value::Object {
        type_name: "ApiResponse".into(),
        type_args: vec![body.ty()],
        fields: HashMap::from([
            ("status".into(), Value::Int(status)),
            ("headers".into(), Value::Array(checked_headers)),
            ("body".into(), body),
        ]),
    })
}

/// Deliver one already-created outbox event through the configured host.
///
/// This shared policy boundary keeps AST and MIR execution consistent. The
/// caller must enforce the language-level `Email` capability before invoking
/// this helper. Errors from the transport are deliberately redacted.
pub fn deliver_outbox_message_with_host(
    event_id: &str,
    payload_json: &str,
    mail_policy: Option<&MailPolicy>,
    network_policy: Option<&NetworkPolicy>,
    host_services: &dyn HostServices,
) -> Result<(), String> {
    mail::validate_mail_network_policy(mail_policy, network_policy)?;
    let mail = mail_policy.ok_or_else(|| "SMTP configuration is incomplete".to_string())?;
    let network = network_policy
        .ok_or_else(|| "SMTP host is not allowed by the project network policy".to_string())?;
    host_services
        .deliver_outbox_message(event_id, payload_json, mail, network)
        .map_err(|_| "SMTP delivery failed".to_string())
}

/// Run an explicitly allowlisted process through the configured host service.
/// This shared check is used by the AST interpreter and alternate backends so
/// command validation, timeout, and output limits cannot drift by backend.
pub fn run_process_with_host_services(
    command: &str,
    arguments: &[String],
    policy: Option<&ProcessPolicy>,
    host_services: &dyn HostServices,
) -> Result<String, String> {
    let policy = policy
        .ok_or_else(|| "process execution denied: no process policy is configured".to_owned())?;
    if command.is_empty() || command.contains(['\r', '\n']) {
        return Err("run_process received an invalid command".into());
    }
    if !policy
        .allowed_commands
        .iter()
        .any(|allowed| allowed == command)
    {
        return Err(format!("process execution denied for command `{command}`"));
    }
    if policy.timeout_ms == 0 || policy.max_output_bytes == 0 {
        return Err("process policy requires positive timeout and output limits".into());
    }
    host_services.run_process(
        command,
        arguments,
        policy.timeout_ms,
        policy.max_output_bytes,
    )
}

/// Run one explicitly allowlisted process with bounded stdin and output.
pub fn run_process_piped_with_host_services(
    command: &str,
    arguments: &[String],
    stdin: &str,
    timeout_ms: u64,
    policy: Option<&ProcessPolicy>,
    host_services: &dyn HostServices,
) -> Result<HostProcessResult, String> {
    let policy = policy
        .ok_or_else(|| "process execution denied: no process policy is configured".to_owned())?;
    if command.is_empty() || command.contains(['\r', '\n']) {
        return Err("run_process_piped received an invalid command".into());
    }
    if !policy
        .allowed_commands
        .iter()
        .any(|allowed| allowed == command)
    {
        return Err(format!("process execution denied for command `{command}`"));
    }
    if timeout_ms == 0 || policy.timeout_ms == 0 || policy.max_output_bytes == 0 {
        return Err("process policy requires positive timeout and output limits".into());
    }
    if stdin.len() > policy.max_output_bytes {
        return Err(format!(
            "process stdin exceeds the {} byte input limit",
            policy.max_output_bytes
        ));
    }
    host_services.run_process_piped(
        command,
        arguments,
        stdin,
        timeout_ms.min(policy.timeout_ms),
        policy.max_output_bytes,
    )
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Output produced before a runtime failure, together with the failure itself.
///
/// This is useful for command-line and editor runs, where already printed output
/// should remain visible even if a later expression fails.
#[derive(Debug)]
pub struct ExecutionOutcome {
    pub output: Vec<String>,
    pub error: Option<RuntimeError>,
}

impl ExecutionOutcome {
    fn into_result(self) -> Result<Vec<String>, RuntimeError> {
        match self.error {
            Some(error) => Err(error),
            None => Ok(self.output),
        }
    }
}

/// Captured output and return value from one explicitly named function call.
#[derive(Debug)]
pub struct FunctionExecutionOutcome {
    pub output: Vec<String>,
    pub value: Option<Value>,
    pub error: Option<RuntimeError>,
}

pub fn encode_json_value(value: &Value, span: Span) -> Result<String, RuntimeError> {
    json_codec_interpreter(&[], &[], &[]).json_encode(value, span)
}

/// Encode a runtime value using checked project definitions. This is used by
/// execution backends that serialize typed records and enum payloads without
/// constructing an AST interpreter instance.
pub fn encode_json_value_with_definitions(
    value: &Value,
    records: &[RecordDef],
    aliases: &[TypeDef],
    enums: &[zelyra_ast::EnumDef],
    span: Span,
) -> Result<String, RuntimeError> {
    json_codec_interpreter(records, aliases, enums).json_encode(value, span)
}

/// Encode `Result` as Zelyra's explicitly tagged JSON representation.
pub fn encode_tagged_result_json(value: &Value, span: Span) -> Result<String, RuntimeError> {
    json_codec_interpreter(&[], &[], &[]).json_encode_tagged_result(value, span)
}

/// Decode JSON using the same type-aware rules as the AST interpreter.
///
/// Callers pass checked project definitions from their typed program so that
/// aliases, records, enums, and generic payloads retain their semantics.
pub fn decode_json_value(
    source: &str,
    target: &Type,
    records: &[RecordDef],
    aliases: &[TypeDef],
    enums: &[zelyra_ast::EnumDef],
    span: Span,
) -> Result<Value, RuntimeError> {
    json_codec_interpreter(records, aliases, enums).json_decode(source, target, span)
}

/// Decode Zelyra's explicitly tagged JSON representation of `Result<T, E>`.
pub fn decode_tagged_result_json(
    source: &str,
    ok_type: &Type,
    error_type: &Type,
    records: &[RecordDef],
    aliases: &[TypeDef],
    enums: &[zelyra_ast::EnumDef],
    span: Span,
) -> Result<Value, String> {
    json_codec_interpreter(records, aliases, enums)
        .json_decode_tagged_result(source, ok_type, error_type, span)
}

fn json_codec_interpreter<'program>(
    records: &'program [RecordDef],
    aliases: &[TypeDef],
    enums: &'program [zelyra_ast::EnumDef],
) -> Interpreter<'program, 'static> {
    Interpreter {
        program_context: Arc::new(ast_interpreter::ProgramContext::from_parts(
            records, aliases, enums,
        )),
        output: Vec::new(),
        next_class_object_id: Arc::new(AtomicU64::new(1)),
        parallel_worker: false,
        step_budget: StepBudget::new(None),
        call_depth_limit: DEFAULT_FUNCTION_CALL_DEPTH,
        call_depth: 0,
        database_url: None,
        database_transaction: None,
        transactional_query: None,
        storage_checked: false,
        execution_context: ast_interpreter::ExecutionContext {
            granted_capabilities: None,
            filesystem_policy: None,
            network_policy: None,
            process_policy: None,
            mail_policy: None,
            package_settings: None,
            host_services: Arc::new(SystemHostServices),
        },
        active_capabilities: Vec::new(),
        active_type_substitutions: Vec::new(),
    }
}

fn integer_range(start: i64, end: i64, step: i64) -> impl Iterator<Item = i64> {
    let ascending = step > 0;
    let first = if (ascending && start < end) || (!ascending && start > end) {
        Some(start)
    } else {
        None
    };
    std::iter::successors(first, move |current| {
        let next = current.checked_add(step)?;
        let before_end = if ascending { next < end } else { next > end };
        before_end.then_some(next)
    })
}

fn class_reference_would_cycle(value: &Value, target: &ClassObjectRef) -> Result<bool, String> {
    let mut pending = vec![value.clone()];
    let mut visited = HashSet::new();
    while let Some(value) = pending.pop() {
        match value {
            Value::ClassRef { reference, .. } => {
                if Arc::ptr_eq(&reference.object, &target.object) {
                    return Ok(true);
                }
                let address = Arc::as_ptr(&reference.object) as usize;
                if visited.insert(address) {
                    let object = reference
                        .object
                        .lock()
                        .map_err(|_| "class object is unavailable".to_owned())?;
                    pending.extend(object.fields.values().cloned());
                }
            }
            Value::Array(values) | Value::Tuple(values) | Value::Set(values) => {
                pending.extend(values);
            }
            Value::Map(entries) => {
                pending.extend(entries.into_iter().flat_map(|(key, value)| [key, value]));
            }
            Value::Object { fields, .. } => pending.extend(fields.into_values()),
            Value::Enum { payloads, .. } => pending.extend(payloads),
            Value::Option(Some(value)) => pending.push(*value),
            Value::Result(Ok(value)) | Value::Result(Err(value)) => pending.push(*value),
            Value::Int(_)
            | Value::UInt(_)
            | Value::Float(_)
            | Value::Decimal(_)
            | Value::Bool(_)
            | Value::String(_)
            | Value::Char(_)
            | Value::Timestamp(_)
            | Value::Date(_)
            | Value::Time(_)
            | Value::Duration(_)
            | Value::Bytes(_)
            | Value::Json(_)
            | Value::Function(_)
            | Value::Option(None)
            | Value::Rows { .. }
            | Value::Unit => {}
        }
    }
    Ok(false)
}

fn statement_span(statement: &Stmt) -> Span {
    match statement {
        Stmt::Let { span, .. }
        | Stmt::BindOrAssign { span, .. }
        | Stmt::ArrayIndexAssign { span, .. }
        | Stmt::TupleBindOrAssign { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::If { span, .. }
        | Stmt::While { span, .. }
        | Stmt::For { span, .. }
        | Stmt::Loop { span, .. }
        | Stmt::Break { span }
        | Stmt::Continue { span }
        | Stmt::Match { span, .. }
        | Stmt::Transaction { span, .. }
        | Stmt::Parallel { span, .. } => *span,
        Stmt::Expr(expression) => expression.span,
    }
}

fn is_runtime_builtin(name: &str) -> bool {
    matches!(
        name,
        "print"
            | "assert"
            | "render_markdown"
            | "render_pdf"
            | "api_response"
            | "len"
            | "range"
            | "array_replace_result"
            | "object_set_field"
            | "record_with"
            | "record_update"
            | "map"
            | "filter"
            | "find"
            | "reduce"
            | "append"
            | "push_mut"
            | "pop_mut"
            | "tuple"
            | "set"
            | "set_add"
            | "set_remove"
            | "contains"
            | "get"
            | "put"
            | "keys"
            | "values"
            | "first"
            | "last"
            | "Some"
            | "Ok"
            | "Err"
            | "now"
            | "date"
            | "decimal"
            | "time"
            | "duration_ms"
            | "bytes"
            | "format_timestamp"
            | "format_text"
            | "env"
            | "random_int"
            | "read_console"
            | "http_get"
            | "http_request"
            | "mail_queue"
            | "mail_deliver"
            | "json_encode"
            | "json_encode_tagged_result"
            | "json_decode"
            | "json_decode_result"
            | "sql_decode"
            | "json_decode_tagged_result"
            | "csv_decode_result"
            | "csv_encode"
            | "text_contains"
            | "text_trim"
            | "text_chars"
            | "abs"
            | "min"
            | "max"
            | "clamp"
            | "to_string"
            | "parse_int"
            | "parse_float"
            | "sort"
            | "sort_desc"
            | "reverse"
            | "slice"
            | "concat"
            | "join"
            | "text_to_lower"
            | "text_to_upper"
            | "text_ucfirst"
            | "text_starts_with"
            | "text_ends_with"
            | "text_replace"
            | "text_split"
            | "split"
            | "text_pad_left"
            | "text_pad_right"
            | "get_or"
            | "get_or_default"
            | "map_increment"
            | "has_key"
            | "unwrap_or"
            | "sqrt"
            | "pow"
            | "floor"
            | "ceil"
            | "round"
            | "text_index_of"
            | "text_repeat"
            | "text_is_empty"
            | "text_capitalize"
            | "unique"
            | "array_sum"
            | "avg"
            | "min_of"
            | "max_of"
            | "chunk"
            | "flatten"
            | "set_union"
            | "set_intersection"
            | "set_difference"
            | "map_remove"
            | "map_merge"
            | "date_add_days"
            | "date_diff_days"
            | "is_ok"
            | "is_err"
            | "is_some"
            | "is_none"
            | "is_before"
            | "is_after"
            | "base64_encode"
            | "base64_decode"
            | "hash_sha256"
            | "uuid_v4"
            | "url_encode"
            | "url_decode"
            | "text_trim_start"
            | "text_trim_end"
            | "text_substring"
            | "text_is_blank"
            | "zip"
            | "entries"
            | "dedup"
            | "to_int"
            | "to_float"
            | "parse_bool"
            | "panic"
            | "path_join"
            | "path_filename"
            | "path_extension"
            | "path_dir"
            | "file_exists"
            | "dir_exists"
            | "file_size"
            | "text_replace_result"
            | "text_split_result"
            | "decimal_scaled_result"
            | "format_scaled_result"
            | "date_add_days_result"
            | "date_days_between_result"
            | "timestamp_add_result"
            | "timestamp_difference_result"
            | "decimal_quantize_result"
            | "http_json"
            | "http_result"
            | "run_process"
            | "run_process_piped"
            | "read_text"
            | "read_text_result"
            | "write_text"
            | "write_text_result"
            | "write_text_new_result"
            | "atomic_write_text_result"
            | "append_text_result"
            | "copy_file_new_result"
            | "rename_file_result"
            | "delete_file_result"
            | "delete_file"
            | "list_dir"
    )
}

const DECIMAL_SCALE: i128 = 1_000_000_000;

fn quantize_decimal(value: i128, scale: u32, mode: &str) -> Result<i128, String> {
    if scale > 9 {
        return Err("decimal scale must be between 0 and 9".into());
    }
    if !matches!(
        mode,
        "toward_zero" | "floor" | "ceiling" | "half_up" | "half_even"
    ) {
        return Err(
            "rounding mode must be toward_zero, floor, ceiling, half_up, or half_even".into(),
        );
    }
    let factor = 10_i128.pow(9 - scale);
    let quotient = value / factor;
    let remainder = value % factor;
    if remainder == 0 || mode == "toward_zero" {
        return quotient
            .checked_mul(factor)
            .ok_or_else(|| "Decimal overflow while quantizing".into());
    }
    let negative = value < 0;
    let increment = match mode {
        "floor" => negative,
        "ceiling" => !negative,
        "half_up" => remainder.unsigned_abs() * 2 >= factor as u128,
        "half_even" => {
            let doubled = remainder.unsigned_abs() * 2;
            doubled > factor as u128 || (doubled == factor as u128 && quotient % 2 != 0)
        }
        _ => false,
    };
    let rounded = if increment {
        quotient.checked_add(if negative { -1 } else { 1 })
    } else {
        Some(quotient)
    }
    .ok_or_else(|| "Decimal overflow while quantizing".to_owned())?;
    rounded
        .checked_mul(factor)
        .ok_or_else(|| "Decimal overflow while quantizing".into())
}

fn canonical_set(mut values: Vec<Value>) -> Vec<Value> {
    values.sort_by_key(Value::output);
    values.dedup();
    values
}

fn parse_decimal(value: &str) -> Option<i128> {
    zelyra_ast::decimal::parse_scaled(value, zelyra_ast::decimal::SCALE_DIGITS)
}

fn format_decimal(value: i128) -> String {
    let magnitude = value.unsigned_abs();
    let whole = magnitude / DECIMAL_SCALE as u128;
    let fraction = magnitude % DECIMAL_SCALE as u128;
    if fraction == 0 {
        return format!("{}{whole}", if value < 0 { "-" } else { "" });
    }
    let fraction = format!("{fraction:09}").trim_end_matches('0').to_owned();
    format!("{}{whole}.{fraction}", if value < 0 { "-" } else { "" })
}

fn query_parameters(
    query: &str,
    env: &Environment,
    span: Span,
) -> Result<Vec<(String, zelyra_database::QueryValue)>, RuntimeError> {
    let mut parameters = Vec::new();
    let names = zelyra_database::sql_parameter_names(query).map_err(|error| RuntimeError {
        message: format!("could not scan SQL parameters: {error}"),
        span,
    })?;
    for name in names {
        if !parameters.iter().any(|(existing, _)| existing == &name) {
            let value = env.get(&name).ok_or_else(|| RuntimeError {
                message: format!("SQL parameter `:{name}` is not available"),
                span,
            })?;
            parameters.push((name, value_to_query_value(&value, span)?));
        }
    }
    Ok(parameters)
}

fn is_pure_mail_expression(expression: &Expr) -> bool {
    match &expression.kind {
        ExprKind::String(_) | ExprKind::Variable(_) => true,
        ExprKind::Field { target, .. } => is_pure_mail_expression(target),
        ExprKind::Record { fields, .. } => fields
            .iter()
            .all(|(_, value)| is_pure_mail_expression(value)),
        ExprKind::Map(entries) => entries
            .iter()
            .all(|(key, value)| is_pure_mail_expression(key) && is_pure_mail_expression(value)),
        ExprKind::Call { name, args, .. }
            if name == "render_pdf"
                && args.len() == 2
                && matches!(args[0].kind, ExprKind::String(_)) =>
        {
            is_pure_mail_expression(&args[1])
        }
        ExprKind::Binary {
            left,
            op: BinaryOp::Add,
            right,
        } => is_pure_mail_expression(left) && is_pure_mail_expression(right),
        _ => false,
    }
}

fn value_to_query_value(
    value: &Value,
    span: Span,
) -> Result<zelyra_database::QueryValue, RuntimeError> {
    use zelyra_database::QueryValue;
    match value {
        Value::Int(value) => Ok(QueryValue::Int(*value)),
        Value::UInt(value) => Ok(QueryValue::UInt(*value)),
        Value::Float(value) => Ok(QueryValue::Float(*value)),
        Value::Decimal(value) => Ok(QueryValue::Decimal(format_decimal(*value))),
        Value::Bool(value) => Ok(QueryValue::Bool(*value)),
        Value::String(value) => Ok(QueryValue::String(value.clone())),
        Value::Char(value) => Ok(QueryValue::String(value.to_string())),
        Value::Json(value) => serde_json::to_string(value)
            .map(QueryValue::Json)
            .map_err(|error| RuntimeError {
                message: format!("could not encode JSON SQL parameter: {error}"),
                span,
            }),
        Value::Timestamp(value) => Ok(QueryValue::Timestamp(*value)),
        Value::Date(value) => Ok(QueryValue::Date(value.clone())),
        Value::Time(value) => Ok(QueryValue::Time(value.clone())),
        Value::Duration(value) => Ok(QueryValue::Int(*value)),
        Value::Option(None) | Value::Unit => Ok(QueryValue::Null),
        Value::Option(Some(value)) => value_to_query_value(value, span),
        Value::Array(_)
        | Value::Tuple(_)
        | Value::Set(_)
        | Value::Map(_)
        | Value::Object { .. }
        | Value::ClassRef { .. }
        | Value::Enum { .. }
        | Value::Function(_)
        | Value::Rows { .. }
        | Value::Bytes(_)
        | Value::Result(_) => Err(RuntimeError {
            message: "SQL parameters must be scalar values".into(),
            span,
        }),
    }
}

fn transaction_queries(
    interpreter: &mut Interpreter,
    block: &Block,
    env: &mut Environment,
    span: Span,
) -> Result<Vec<zelyra_database::Query>, RuntimeError> {
    let mut queries = Vec::new();
    for statement in &block.statements {
        let (expression, allow_mail_queue) = match statement {
            Stmt::Expr(expression) => (expression, true),
            Stmt::Let { value, .. } => (value, false),
            _ => {
                return Err(RuntimeError {
                    message: "transactions may contain only SQL statements".into(),
                    span,
                })
            }
        };
        match &expression.kind {
            ExprKind::Sql { query, .. } => {
                check_scoped_sql(query, &interpreter.program_context.scoped_tables).map_err(
                    |message| RuntimeError {
                        message,
                        span: expression.span,
                    },
                )?;
                let capability = sql_required_capability(query);
                interpreter.require_runtime_capability(
                    capability,
                    "SQL access",
                    expression.span,
                )?;
                queries.push(zelyra_database::Query {
                    sql: query.clone(),
                    params: query_parameters(query, env, expression.span)?,
                });
            }
            ExprKind::Call { name, args, .. } if name == "mail_queue" => {
                if !allow_mail_queue {
                    return Err(interpreter.runtime_error(
                        expression.span,
                        "mail_queue must be a standalone statement inside a transaction",
                    ));
                }
                if !matches!(args.len(), 3..=6) {
                    return Err(interpreter.runtime_error(
                        expression.span,
                        "mail_queue expects recipient, subject, text body, optional HTML body, and optional PDF filename and Bytes",
                    ));
                }
                if args
                    .iter()
                    .any(|argument| !is_pure_mail_expression(argument))
                {
                    return Err(interpreter.runtime_error(
                        expression.span,
                        "mail_queue arguments must be values or pure expressions; PDF attachments must use render_pdf",
                    ));
                }
                interpreter.require_runtime_capability(
                    "Email",
                    "email delivery",
                    expression.span,
                )?;
                interpreter.require_runtime_capability(
                    "Database(write)",
                    "transactional mail queue write",
                    expression.span,
                )?;
                let database_url = interpreter.database_url.as_deref().ok_or_else(|| {
                    interpreter.runtime_error(
                        expression.span,
                        "mail_queue requires a MariaDB or MySQL database runtime",
                    )
                })?;
                if !supports_mail_outbox_database(database_url) {
                    return Err(interpreter.runtime_error(
                        expression.span,
                        "mail_queue currently requires MariaDB or MySQL; PostgreSQL outbox support is not available",
                    ));
                }
                let mut evaluated = Vec::with_capacity(args.len());
                for argument in args {
                    evaluated.push(interpreter.eval(argument, env)?);
                }
                let argument_spans = args
                    .iter()
                    .map(|argument| argument.span)
                    .collect::<Vec<_>>();
                queries.push(prepare_mail_queue_query(
                    &evaluated,
                    &argument_spans,
                    expression.span,
                    interpreter.execution_context.host_services.as_ref(),
                )?);
            }
            _ => return Err(RuntimeError {
                message:
                    "transactions may contain only SQL statements and standalone mail_queue calls"
                        .into(),
                span: expression.span,
            }),
        }
    }
    if queries.is_empty() {
        return Err(RuntimeError {
            message: "transaction must contain at least one SQL statement".into(),
            span,
        });
    }
    Ok(queries)
}

fn match_pattern(value: &Value, pattern: &Pattern) -> Option<Vec<(String, Value)>> {
    match &pattern.kind {
        PatternKind::Wildcard => Some(Vec::new()),
        PatternKind::Variable(name) => Some(vec![(name.clone(), value.clone())]),
        PatternKind::Int(expected) => match value {
            Value::Int(actual) if actual == expected => Some(Vec::new()),
            _ => None,
        },
        PatternKind::Bool(expected) => match value {
            Value::Bool(actual) if actual == expected => Some(Vec::new()),
            _ => None,
        },
        PatternKind::String(expected) => match value {
            Value::String(actual) if actual == expected => Some(Vec::new()),
            _ => None,
        },
        PatternKind::Char(expected) => match value {
            Value::Char(actual) if actual == expected => Some(Vec::new()),
            _ => None,
        },
        PatternKind::EnumVariant { enum_name, variant } => match value {
            Value::Enum {
                type_name,
                variant: actual,
                payloads,
                ..
            } if type_name == enum_name && actual == variant && payloads.is_empty() => {
                Some(Vec::new())
            }
            _ => None,
        },
        PatternKind::EnumVariantPayload {
            enum_name,
            variant,
            payloads: patterns,
        } => match value {
            Value::Enum {
                type_name,
                variant: actual,
                payloads,
                ..
            } if type_name == enum_name
                && actual == variant
                && payloads.len() == patterns.len() =>
            {
                let mut bindings = Vec::new();
                for (value, pattern) in payloads.iter().zip(patterns) {
                    bindings.extend(match_pattern(value, pattern)?);
                }
                Some(bindings)
            }
            _ => None,
        },
        PatternKind::Record {
            type_name,
            fields: patterns,
        } => match value {
            Value::Object {
                type_name: actual,
                fields,
                ..
            } if actual == type_name => {
                let mut bindings = Vec::new();
                for (name, pattern) in patterns {
                    bindings.extend(match_pattern(fields.get(name)?, pattern)?);
                }
                Some(bindings)
            }
            _ => None,
        },
        PatternKind::Constructor { name, inner } => match (name.as_str(), value) {
            ("Some", Value::Option(Some(value))) | ("Ok", Value::Result(Ok(value))) => inner
                .as_ref()
                .and_then(|pattern| match_pattern(value, pattern)),
            ("None", Value::Option(None)) => {
                if inner.is_none() {
                    Some(Vec::new())
                } else {
                    None
                }
            }
            ("Err", Value::Result(Err(_))) if inner.is_none() => Some(Vec::new()),
            ("Err", Value::Result(Err(value))) => inner
                .as_ref()
                .and_then(|pattern| match_pattern(value, pattern)),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests;
