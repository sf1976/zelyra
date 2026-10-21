//! Versioned, backend-neutral host services used by Zelyra runtimes.
//!
//! Implementations are trusted runtime embeddings. Language-level capability
//! checks remain authoritative and happen before these methods are called.

use rand_core::{OsRng, RngCore};
use std::{
    io::{self, BufRead, Read, Write},
    path::Path,
    process::{Command, Stdio},
    sync::Mutex,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// Semantic API version for the host-service contract (not a native ABI).
pub const HOST_SERVICES_API_VERSION: u32 = 8;

/// Metadata returned by an injected filesystem provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostFileMetadata {
    pub is_file: bool,
    pub is_directory: bool,
    pub length: u64,
}

/// Bounded response returned by an injected HTTP transport.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostHttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Bounded output from an explicitly authorized child process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostProcessResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Host-provided primitives shared by execution backends.
///
/// Implementations should be deterministic where appropriate and must not
/// broaden the capabilities granted by the embedding application.
pub trait HostServices: Send + Sync {
    /// Return Unix time in milliseconds.
    fn unix_time_millis(&self) -> Result<i64, String>;

    /// Read one UTF-8 environment variable. Missing keys return `None`.
    fn environment_variable(&self, name: &str) -> Result<Option<String>, String>;

    /// Fill a caller-owned buffer with cryptographically secure random bytes.
    fn fill_random(&self, output: &mut [u8]) -> Result<(), String>;

    /// Read one UTF-8 console line after displaying a prompt.
    fn read_console_line(&self, _prompt: &str) -> Result<Option<String>, String> {
        Err("console input is unavailable in this host".into())
    }

    /// Perform one HTTP request after runtime URL, capability, and policy checks.
    ///
    /// The runtime validates the response size and UTF-8 encoding again after
    /// return, so an embedding cannot bypass the language-level size contract.
    fn http_request(
        &self,
        _method: &str,
        _url: &str,
        _headers: &[(String, String)],
        _body: Option<&[u8]>,
        _timeout_ms: u64,
        _max_response_bytes: usize,
    ) -> Result<HostHttpResponse, String> {
        Err("network access is unavailable in this host".into())
    }

    /// Deliver one validated outbox event through the configured mail host.
    fn deliver_outbox_message(
        &self,
        _event_id: &str,
        _payload_json: &str,
        _mail: &super::MailPolicy,
        _network: &super::NetworkPolicy,
    ) -> Result<(), String> {
        Err("mail delivery is unavailable in this host".into())
    }

    /// Run one explicitly authorized process with bounded time and output.
    ///
    /// Runtime capability and allow-list checks happen before this method is
    /// called. Embedders that do not provide process execution inherit a clear
    /// rejection rather than accidentally falling back to the system shell.
    fn run_process(
        &self,
        _command: &str,
        _arguments: &[String],
        _timeout_ms: u64,
        _max_output_bytes: usize,
    ) -> Result<String, String> {
        Err("process execution is unavailable in this host".into())
    }

    /// Run an explicitly authorized process with bounded UTF-8 stdin/stdout/stderr.
    fn run_process_piped(
        &self,
        _command: &str,
        _arguments: &[String],
        _stdin: &str,
        _timeout_ms: u64,
        _max_output_bytes: usize,
    ) -> Result<HostProcessResult, String> {
        Err("piped process execution is unavailable in this host".into())
    }

    /// Canonicalize a candidate path before runtime root-containment checks.
    ///
    /// The default uses the system filesystem. Virtual filesystem hosts must
    /// override this method and preserve symlink/traversal safety.
    fn canonicalize_path(
        &self,
        path: &Path,
        allow_missing: bool,
    ) -> Result<std::path::PathBuf, String> {
        canonicalize_system_path(path, allow_missing)
    }

    /// Inspect metadata for a path after runtime root-containment checks.
    fn file_metadata(&self, _path: &Path) -> Result<HostFileMetadata, String> {
        Err("filesystem metadata is unavailable in this host".into())
    }

    /// Read UTF-8 text from a policy-checked path.
    fn read_text(&self, _path: &Path) -> Result<String, String> {
        Err("filesystem reads are unavailable in this host".into())
    }

    /// Write UTF-8 text to a policy-checked path.
    fn write_text(&self, _path: &Path, _content: &str) -> Result<(), String> {
        Err("filesystem writes are unavailable in this host".into())
    }

    /// Create a UTF-8 text file only when its target does not exist.
    fn write_text_new(&self, _path: &Path, _content: &str) -> Result<(), String> {
        Err("exclusive filesystem writes are unavailable in this host".into())
    }

    /// Atomically replace a UTF-8 text file.
    fn atomic_write_text(&self, _path: &Path, _content: &str) -> Result<(), String> {
        Err("atomic filesystem writes are unavailable in this host".into())
    }

    /// Append UTF-8 text to a policy-checked path.
    fn append_text(&self, _path: &Path, _content: &str) -> Result<(), String> {
        Err("filesystem appends are unavailable in this host".into())
    }

    /// Copy a file without replacing an existing destination.
    fn copy_file_new(&self, _source: &Path, _destination: &Path) -> Result<(), String> {
        Err("filesystem copies are unavailable in this host".into())
    }

    /// Rename a file without replacing an existing destination.
    fn rename_file_new(&self, _source: &Path, _destination: &Path) -> Result<(), String> {
        Err("filesystem renames are unavailable in this host".into())
    }

    /// Remove a policy-checked file.
    fn remove_file(&self, _path: &Path) -> Result<(), String> {
        Err("filesystem deletes are unavailable in this host".into())
    }

    /// List child names in a policy-checked directory.
    fn list_directory(&self, _path: &Path) -> Result<Vec<String>, String> {
        Err("filesystem directory listing is unavailable in this host".into())
    }

    /// Execute one already parsed and capability-checked Zelyra SQL query.
    fn execute_database_query(
        &self,
        _database_url: &str,
        _query: &zelyra_database::Query,
    ) -> Result<zelyra_database::QueryResult, String> {
        Err("database queries are unavailable in this host".into())
    }

    /// Execute a SQL batch, optionally within one database transaction.
    fn execute_database_batch(
        &self,
        _database_url: &str,
        _queries: &[zelyra_database::Query],
        _transaction: bool,
    ) -> Result<Vec<zelyra_database::QueryResult>, String> {
        Err("database batches are unavailable in this host".into())
    }

    /// Check storage compatibility before queries use declared temporal or
    /// fixed-point columns. This operation must never migrate the schema.
    fn validate_database_storage(
        &self,
        database_url: &str,
        schema: &zelyra_database::Schema,
    ) -> Result<(), String> {
        if zelyra_database::schema_requires_storage_validation(
            schema,
            zelyra_database::is_postgres_url(database_url),
        ) {
            Err("database storage validation is unavailable in this host".into())
        } else {
            Ok(())
        }
    }
}

/// Host implementation used by existing Zelyra APIs.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemHostServices;

impl HostServices for SystemHostServices {
    fn unix_time_millis(&self) -> Result<i64, String> {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system time is before Unix epoch: {error}"))?
            .as_millis();
        i64::try_from(millis).map_err(|_| "timestamp overflow".into())
    }

    fn environment_variable(&self, name: &str) -> Result<Option<String>, String> {
        match std::env::var_os(name) {
            Some(value) => value
                .into_string()
                .map(Some)
                .map_err(|_| format!("environment variable {name} is not valid UTF-8")),
            None => Ok(None),
        }
    }

    fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
        OsRng
            .try_fill_bytes(output)
            .map_err(|error| format!("random access failed: {error}"))
    }

    fn read_console_line(&self, prompt: &str) -> Result<Option<String>, String> {
        static CONSOLE_INPUT_LOCK: Mutex<()> = Mutex::new(());
        let _guard = CONSOLE_INPUT_LOCK
            .lock()
            .map_err(|_| "console input lock is unavailable".to_owned())?;

        let mut stdout = io::stdout().lock();
        stdout
            .write_all(prompt.as_bytes())
            .and_then(|()| stdout.flush())
            .map_err(|error| error.to_string())?;

        let mut input = String::new();
        if io::stdin()
            .lock()
            .read_line(&mut input)
            .map_err(|error| error.to_string())?
            == 0
        {
            return Ok(None);
        }
        if input.ends_with('\n') {
            input.pop();
            if input.ends_with('\r') {
                input.pop();
            }
        }
        Ok(Some(input))
    }

    fn http_request(
        &self,
        method: &str,
        url: &str,
        headers: &[(String, String)],
        body: Option<&[u8]>,
        timeout_ms: u64,
        max_response_bytes: usize,
    ) -> Result<HostHttpResponse, String> {
        if timeout_ms == 0 || max_response_bytes == 0 {
            return Err("HTTP transport limits must be positive".into());
        }
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_millis(timeout_ms)))
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .into();
        let mut builder = ureq::http::Request::builder().method(method).uri(url);
        for (name, value) in headers {
            builder = builder.header(name, value);
        }
        let request = builder
            .body(body.unwrap_or_default().to_vec())
            .map_err(|error| format!("could not build HTTP request: {error}"))?;
        let mut response = agent
            .run(request)
            .map_err(|error| format!("network request failed: {error}"))?;
        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<usize>().ok());
        if content_length.is_some_and(|length| length > max_response_bytes) {
            return Err(format!(
                "network response exceeds the {max_response_bytes} byte limit"
            ));
        }
        let body = response
            .body_mut()
            .with_config()
            .limit(
                u64::try_from(max_response_bytes)
                    .unwrap_or(u64::MAX)
                    .saturating_add(1),
            )
            .read_to_vec()
            .map_err(|error| format!("network response failed: {error}"))?;
        if body.len() > max_response_bytes {
            return Err(format!(
                "network response exceeds the {max_response_bytes} byte limit"
            ));
        }
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.to_string(),
                    value.to_str().unwrap_or("<non-utf8-header>").to_owned(),
                )
            })
            .collect();
        Ok(HostHttpResponse {
            status: response.status().as_u16(),
            headers,
            body,
        })
    }

    fn deliver_outbox_message(
        &self,
        event_id: &str,
        payload_json: &str,
        mail: &super::MailPolicy,
        network: &super::NetworkPolicy,
    ) -> Result<(), String> {
        super::mail::deliver_outbox_message(event_id, payload_json, Some(mail), Some(network))
            .map_err(str::to_owned)
    }

    fn run_process(
        &self,
        command: &str,
        arguments: &[String],
        timeout_ms: u64,
        max_output_bytes: usize,
    ) -> Result<String, String> {
        run_system_process(command, arguments, timeout_ms, max_output_bytes)
    }

    fn run_process_piped(
        &self,
        command: &str,
        arguments: &[String],
        stdin: &str,
        timeout_ms: u64,
        max_output_bytes: usize,
    ) -> Result<HostProcessResult, String> {
        run_system_process_piped(command, arguments, stdin, timeout_ms, max_output_bytes)
    }

    fn canonicalize_path(
        &self,
        path: &Path,
        allow_missing: bool,
    ) -> Result<std::path::PathBuf, String> {
        canonicalize_system_path(path, allow_missing)
    }

    fn file_metadata(&self, path: &Path) -> Result<HostFileMetadata, String> {
        let metadata = std::fs::metadata(path).map_err(|error| error.to_string())?;
        Ok(HostFileMetadata {
            is_file: metadata.is_file(),
            is_directory: metadata.is_dir(),
            length: metadata.len(),
        })
    }

    fn read_text(&self, path: &Path) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|error| error.to_string())
    }

    fn write_text(&self, path: &Path, content: &str) -> Result<(), String> {
        std::fs::write(path, content).map_err(|error| error.to_string())
    }

    fn write_text_new(&self, path: &Path, content: &str) -> Result<(), String> {
        super::filesystem::write_text_new(path, content).map_err(|error| error.to_string())
    }

    fn atomic_write_text(&self, path: &Path, content: &str) -> Result<(), String> {
        super::filesystem::atomic_write_text(path, content).map_err(|error| error.to_string())
    }

    fn append_text(&self, path: &Path, content: &str) -> Result<(), String> {
        super::filesystem::append_text(path, content).map_err(|error| error.to_string())
    }

    fn copy_file_new(&self, source: &Path, destination: &Path) -> Result<(), String> {
        super::filesystem::copy_file_new(source, destination).map_err(|error| error.to_string())
    }

    fn rename_file_new(&self, source: &Path, destination: &Path) -> Result<(), String> {
        super::filesystem::rename_file_new(source, destination).map_err(|error| error.to_string())
    }

    fn remove_file(&self, path: &Path) -> Result<(), String> {
        std::fs::remove_file(path).map_err(|error| error.to_string())
    }

    fn list_directory(&self, path: &Path) -> Result<Vec<String>, String> {
        let entries = std::fs::read_dir(path).map_err(|error| error.to_string())?;
        let mut names = entries
            .map(|entry| {
                entry
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        names.sort();
        Ok(names)
    }

    fn execute_database_query(
        &self,
        database_url: &str,
        query: &zelyra_database::Query,
    ) -> Result<zelyra_database::QueryResult, String> {
        zelyra_database::execute_query(database_url, &query.sql, query.params.clone())
            .map_err(|error| error.to_string())
    }

    fn execute_database_batch(
        &self,
        database_url: &str,
        queries: &[zelyra_database::Query],
        transaction: bool,
    ) -> Result<Vec<zelyra_database::QueryResult>, String> {
        zelyra_database::execute_queries(database_url, queries, transaction)
            .map_err(|error| error.to_string())
    }

    fn validate_database_storage(
        &self,
        database_url: &str,
        schema: &zelyra_database::Schema,
    ) -> Result<(), String> {
        zelyra_database::validate_decimal_storage(database_url, schema)
            .and_then(|()| zelyra_database::validate_temporal_storage(database_url, schema))
            .map_err(|error| error.to_string())
    }
}

fn canonicalize_system_path(
    path: &Path,
    allow_missing: bool,
) -> Result<std::path::PathBuf, String> {
    if allow_missing && !path.exists() && !path.is_symlink() {
        let Some(file_name) = path.file_name() else {
            return Err("filesystem operation requires a file path".into());
        };
        let parent = path.parent().unwrap_or(Path::new("."));
        let parent = std::fs::canonicalize(parent).map_err(|error| error.to_string())?;
        Ok(parent.join(file_name))
    } else {
        std::fs::canonicalize(path).map_err(|error| error.to_string())
    }
}

fn run_system_process(
    command: &str,
    arguments: &[String],
    timeout_ms: u64,
    max_output_bytes: usize,
) -> Result<String, String> {
    if command.is_empty() || command.contains(['\r', '\n']) {
        return Err("run_process received an invalid command".into());
    }
    if timeout_ms == 0 || max_output_bytes == 0 {
        return Err("process policy requires positive timeout and output limits".into());
    }
    let mut child = Command::new(command)
        .args(arguments)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("process start failed for `{command}`: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "process stdout pipe was unavailable".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "process stderr pipe was unavailable".to_owned())?;
    let output_limit = u64::try_from(max_output_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let stdout_reader = thread::spawn(move || {
        let mut output = Vec::new();
        stdout
            .take(output_limit)
            .read_to_end(&mut output)
            .map(|_| output)
    });
    let stderr_reader = thread::spawn(move || {
        let mut output = Vec::new();
        stderr
            .take(output_limit)
            .read_to_end(&mut output)
            .map(|_| output)
    });
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                timed_out = true;
                let _ = child.kill();
                break child
                    .wait()
                    .map_err(|error| format!("process termination failed: {error}"))?;
            }
            Ok(None) => thread::sleep(Duration::from_millis(5)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("process status failed: {error}"));
            }
        }
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| "process stdout reader terminated unexpectedly".to_owned())?
        .map_err(|error| format!("process stdout read failed: {error}"))?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| "process stderr reader terminated unexpectedly".to_owned())?
        .map_err(|error| format!("process stderr read failed: {error}"))?;
    if timed_out {
        return Err(format!(
            "process `{command}` exceeded the {timeout_ms} ms timeout"
        ));
    }
    if stdout.len() > max_output_bytes || stderr.len() > max_output_bytes {
        return Err(format!(
            "process `{command}` exceeded the {max_output_bytes} byte output limit"
        ));
    }
    if !status.success() {
        return Err(format!(
            "process `{command}` exited unsuccessfully ({}): {}",
            status,
            String::from_utf8_lossy(&stderr).trim()
        ));
    }
    String::from_utf8(stdout).map_err(|_| "process stdout is not valid UTF-8".into())
}

fn run_system_process_piped(
    command: &str,
    arguments: &[String],
    input: &str,
    timeout_ms: u64,
    max_output_bytes: usize,
) -> Result<HostProcessResult, String> {
    if command.is_empty() || command.contains(['\r', '\n']) {
        return Err("run_process_piped received an invalid command".into());
    }
    if timeout_ms == 0 || max_output_bytes == 0 {
        return Err("process policy requires positive timeout and output limits".into());
    }
    if input.len() > max_output_bytes {
        return Err(format!(
            "process stdin exceeds the {max_output_bytes} byte input limit"
        ));
    }
    let mut child = Command::new(command)
        .args(arguments)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("process start failed for `{command}`: {error}"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "process stdin pipe was unavailable".to_owned())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "process stdout pipe was unavailable".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "process stderr pipe was unavailable".to_owned())?;
    let output_limit = u64::try_from(max_output_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let input = input.as_bytes().to_vec();
    let stdin_writer = thread::spawn(move || {
        let mut stdin = stdin;
        stdin.write_all(&input).map_err(|error| error.to_string())
    });
    let stdout_reader = thread::spawn(move || {
        let mut output = Vec::new();
        stdout
            .take(output_limit)
            .read_to_end(&mut output)
            .map(|_| output)
    });
    let stderr_reader = thread::spawn(move || {
        let mut output = Vec::new();
        stderr
            .take(output_limit)
            .read_to_end(&mut output)
            .map(|_| output)
    });
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                timed_out = true;
                let _ = child.kill();
                break child
                    .wait()
                    .map_err(|error| format!("process termination failed: {error}"))?;
            }
            Ok(None) => thread::sleep(Duration::from_millis(5)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("process status failed: {error}"));
            }
        }
    };
    let _ = stdin_writer.join();
    let stdout = stdout_reader
        .join()
        .map_err(|_| "process stdout reader terminated unexpectedly".to_owned())?
        .map_err(|error| format!("process stdout read failed: {error}"))?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| "process stderr reader terminated unexpectedly".to_owned())?
        .map_err(|error| format!("process stderr read failed: {error}"))?;
    if timed_out {
        return Err(format!(
            "process `{command}` exceeded the {timeout_ms} ms timeout"
        ));
    }
    if stdout.len() > max_output_bytes || stderr.len() > max_output_bytes {
        return Err(format!(
            "process `{command}` exceeded the {max_output_bytes} byte output limit"
        ));
    }
    let stdout =
        String::from_utf8(stdout).map_err(|_| "process stdout is not valid UTF-8".to_owned())?;
    let stderr =
        String::from_utf8(stderr).map_err(|_| "process stderr is not valid UTF-8".to_owned())?;
    Ok(HostProcessResult {
        exit_code: status.code().unwrap_or(-1),
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_process_service_runs_without_a_shell_and_enforces_output_limit() {
        let output = SystemHostServices
            .run_process("/usr/bin/printf", &["%s".into(), "host".into()], 1_000, 16)
            .unwrap();
        assert_eq!(output, "host");
        let error = SystemHostServices
            .run_process("/usr/bin/printf", &["longer".into()], 1_000, 2)
            .unwrap_err();
        assert!(error.contains("output limit"));
    }

    #[test]
    fn system_piped_process_returns_stdin_stdout_and_exit_code() {
        let result = SystemHostServices
            .run_process_piped("/usr/bin/cat", &[], "stdin text", 1_000, 32)
            .unwrap();
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "stdin text");
        assert_eq!(result.stderr, "");
        let failed = SystemHostServices
            .run_process_piped("/usr/bin/false", &[], "", 1_000, 32)
            .unwrap();
        assert_ne!(failed.exit_code, 0);
        assert!(failed.stdout.is_empty());
        assert!(SystemHostServices
            .run_process_piped("/usr/bin/cat", &[], "too long", 1_000, 2)
            .unwrap_err()
            .contains("stdin"));
    }

    #[test]
    fn system_filesystem_service_performs_bounded_file_operations() {
        let directory = std::env::temp_dir().join(format!(
            "zelyra-host-files-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let file = directory.join("sample.txt");
        let host = SystemHostServices;
        host.write_text_new(&file, "first").unwrap();
        assert_eq!(host.read_text(&file).unwrap(), "first");
        assert_eq!(host.file_metadata(&file).unwrap().length, 5);
        host.append_text(&file, "-second").unwrap();
        assert_eq!(host.read_text(&file).unwrap(), "first-second");
        assert_eq!(host.list_directory(&directory).unwrap(), ["sample.txt"]);
        host.remove_file(&file).unwrap();
        assert!(host.file_metadata(&file).is_err());
        std::fs::remove_dir(directory).unwrap();
    }
}
