use super::*;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;
use zelyra_web::{parse_request, Request};

const EDITOR_HTML: &str = include_str!("../assets/editor.html");
const EDITOR_JS: &str = include_str!("../assets/editor.js");
const EDITOR_CSS: &str = include_str!("../assets/editor.css");
const EDITOR_PATH: &str = "/__zelyra/editor";
const EDITOR_MAX_CONNECTIONS: usize = 16;
const EDITOR_MAX_FILE_BYTES: usize = 1_048_576;
const EDITOR_MAX_FILES: usize = 2_000;
const EDITOR_MAX_DEPTH: usize = 8;
const EDITOR_TIMEOUT: Duration = Duration::from_secs(30);

struct EditorConnection(Arc<AtomicUsize>);

impl Drop for EditorConnection {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Release);
    }
}

#[derive(Clone)]
struct EditorState {
    root: PathBuf,
    token: String,
    authority: String,
}

pub(super) fn command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let mut directory = PathBuf::from(".");
    let mut directory_given = false;
    let mut port = 4177_u16;
    while let Some(argument) = arguments.next() {
        if argument == "--port" {
            let Some(value) = arguments.next() else {
                eprintln!("error[E-EDITOR-001]: --port requires a port number");
                return ExitCode::from(2);
            };
            port = match parse_port(&value, "editor") {
                Ok(port) => port,
                Err(error) => {
                    eprintln!("error[E-EDITOR-001]: {error}");
                    return ExitCode::from(2);
                }
            };
        } else if !argument.starts_with('-') && !directory_given {
            directory = PathBuf::from(argument);
            directory_given = true;
        } else {
            eprintln!("error[E-EDITOR-001]: usage: zelyra editor [directory] [--port <port>]");
            return ExitCode::from(2);
        }
    }
    let root = match fs::canonicalize(&directory) {
        Ok(root) if root.is_dir() => root,
        _ => {
            eprintln!(
                "error[E-EDITOR-002]: project directory `{}` does not exist",
                directory.display()
            );
            return ExitCode::from(1);
        }
    };
    if !root.join("zelyra.toml").is_file()
        && !root.join("main.zyl").is_file()
        && !root.join(".zelyra").is_dir()
    {
        eprintln!(
            "error[E-EDITOR-002]: `{}` is not a Zelyra project; expected zelyra.toml or main.zyl",
            root.display()
        );
        return ExitCode::from(1);
    }
    let authority = format!("127.0.0.1:{port}");
    let listener = match TcpListener::bind(&authority) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("error[E-EDITOR-003]: cannot start local editor on {authority}: {error}");
            return ExitCode::from(1);
        }
    };
    let mut token_bytes = [0_u8; 32];
    if OsRng.try_fill_bytes(&mut token_bytes).is_err() {
        eprintln!("error[E-EDITOR-004]: could not create an editor session token");
        return ExitCode::from(1);
    }
    let token = token_bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let state = Arc::new(EditorState {
        root,
        token: token.clone(),
        authority: authority.clone(),
    });
    let active = Arc::new(AtomicUsize::new(0));
    println!("Zelyra Studio ist lokal erreichbar: http://{authority}{EDITOR_PATH}/#{token}");
    println!("Projekt: {}", state.root.display());
    println!(
        "Der Editor akzeptiert ausschließlich Verbindungen von diesem Rechner. Beenden mit Ctrl+C."
    );
    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                let peer = stream.peer_addr().ok();
                if !peer.is_some_and(|address| address.ip().is_loopback()) {
                    continue;
                }
                if active
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                        (count < EDITOR_MAX_CONNECTIONS).then_some(count + 1)
                    })
                    .is_err()
                {
                    continue;
                }
                let state = Arc::clone(&state);
                let worker_active = Arc::clone(&active);
                if let Err(error) = thread::Builder::new()
                    .name("zelyra-editor-http".into())
                    .spawn(move || {
                        let _slot = EditorConnection(worker_active);
                        if let Err(error) = handle_editor_connection(stream, peer, &state) {
                            eprintln!("zelyra editor: request failed: {error}");
                        }
                    })
                {
                    active.fetch_sub(1, Ordering::Release);
                    eprintln!("zelyra editor: could not start request worker: {error}");
                }
            }
            Err(error) => eprintln!("zelyra editor: connection failed: {error}"),
        }
    }
    ExitCode::SUCCESS
}

fn handle_editor_connection(
    mut stream: TcpStream,
    peer: Option<SocketAddr>,
    state: &EditorState,
) -> io::Result<()> {
    stream.set_read_timeout(Some(EDITOR_TIMEOUT))?;
    stream.set_write_timeout(Some(EDITOR_TIMEOUT))?;
    let raw = read_editor_request(&mut stream)?;
    let request = match parse_request(&raw) {
        Ok(request) => request,
        Err(error) => {
            return write_editor_response(
                &mut stream,
                400,
                "application/json; charset=utf-8",
                &json!({"error": error.message}).to_string(),
            )
        }
    };
    let host = request
        .headers
        .get("host")
        .map(String::as_str)
        .unwrap_or_default();
    if host != state.authority || !peer.is_some_and(|address| address.ip().is_loopback()) {
        return write_editor_response(
            &mut stream,
            403,
            "application/json; charset=utf-8",
            r#"{"error":"editor is local-only"}"#,
        );
    }
    if request.path.starts_with("/__zelyra/") && !request.path.starts_with(EDITOR_PATH) {
        return write_editor_response(
            &mut stream,
            404,
            "application/json; charset=utf-8",
            r#"{"error":"not found"}"#,
        );
    }
    let (status, content_type, body) = match editor_response(&request, state) {
        Ok(response) => response,
        Err((status, message)) => (
            status,
            "application/json; charset=utf-8",
            json!({"error": message}).to_string(),
        ),
    };
    write_editor_response(&mut stream, status, content_type, &body)
}

fn editor_response(
    request: &Request,
    state: &EditorState,
) -> Result<(u16, &'static str, String), (u16, String)> {
    if request.path == EDITOR_PATH || request.path == format!("{EDITOR_PATH}/") {
        if request.method != "GET" {
            return Err((405, "method not allowed".into()));
        }
        let project = state
            .root
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Zelyra");
        let html = EDITOR_HTML.replace(
            "data-project=\"\"",
            &format!("data-project=\"{}\"", html_escape(project)),
        );
        return Ok((200, "text/html; charset=utf-8", html));
    }
    if request.path == format!("{EDITOR_PATH}/assets/editor.js") && request.method == "GET" {
        return Ok((200, "text/javascript; charset=utf-8", EDITOR_JS.into()));
    }
    if request.path == format!("{EDITOR_PATH}/assets/editor.css") && request.method == "GET" {
        return Ok((200, "text/css; charset=utf-8", EDITOR_CSS.into()));
    }
    let api_prefix = format!("{EDITOR_PATH}/api/");
    let Some(operation) = request.path.strip_prefix(&api_prefix) else {
        return Err((404, "not found".into()));
    };
    if request
        .headers
        .get("x-zelyra-editor-token")
        .map(String::as_str)
        != Some(state.token.as_str())
    {
        return Err((403, "invalid editor session".into()));
    }
    if request.method != "GET" {
        let origin = request
            .headers
            .get("origin")
            .map(String::as_str)
            .unwrap_or_default();
        let expected_origin = format!("http://{}", state.authority);
        if origin != expected_origin {
            return Err((
                403,
                "state-changing requests must come from the editor origin".into(),
            ));
        }
    }
    match (request.method.as_str(), operation) {
        ("GET", "tree") => tree_response(state),
        ("GET", "file") => {
            let query = request
                .target
                .split_once('?')
                .map(|(_, query)| query)
                .unwrap_or_default();
            let values =
                zelyra_web::parse_urlencoded(query).map_err(|error| (400, error.message))?;
            let path = values.get("path").ok_or((400, "missing path".into()))?;
            read_editor_file(state, path)
        }
        ("PUT", "file") => write_existing_file(state, parse_json_body(&request.body)?, false),
        ("POST", "file") => write_existing_file(state, parse_json_body(&request.body)?, true),
        ("POST", "check") => check_project(state, &parse_json_body(&request.body)?),
        ("POST", "format") => format_editor_file(state, &parse_json_body(&request.body)?),
        _ => Err((404, "not found".into())),
    }
}

fn parse_json_body(body: &str) -> Result<Value, (u16, String)> {
    serde_json::from_str(body).map_err(|_| (400, "request body must be valid JSON".into()))
}

fn tree_response(state: &EditorState) -> Result<(u16, &'static str, String), (u16, String)> {
    let mut files = Vec::new();
    collect_editor_files(&state.root, &state.root, 0, &mut files)?;
    files.sort();
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"files": files}).to_string(),
    ))
}

fn collect_editor_files(
    root: &Path,
    directory: &Path,
    depth: usize,
    files: &mut Vec<String>,
) -> Result<(), (u16, String)> {
    if depth > EDITOR_MAX_DEPTH {
        return Ok(());
    }
    let entries = fs::read_dir(directory)
        .map_err(|error| (500, format!("could not read project directory: {error}")))?;
    let mut entries = entries
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| (500, format!("could not read project entry: {error}")))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if files.len() >= EDITOR_MAX_FILES {
            return Ok(());
        }
        let name = entry.file_name();
        let Some(name_text) = name.to_str() else {
            continue;
        };
        if name_text.starts_with('.')
            || matches!(
                name_text,
                "target" | "node_modules" | "vendor" | "dist" | "build"
            )
        {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| (500, format!("could not inspect project entry: {error}")))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            collect_editor_files(root, &entry.path(), depth + 1, files)?;
        } else if metadata.is_file()
            && metadata.len() <= EDITOR_MAX_FILE_BYTES as u64
            && editor_file_allowed(&entry.path())
        {
            let entry_path = entry.path();
            let relative = entry_path
                .strip_prefix(root)
                .map_err(|_| (403, "file is outside the project".into()))?;
            files.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

fn editor_file_allowed(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    if name.starts_with('.')
        || name.ends_with("~")
        || name.ends_with(".pem")
        || name.ends_with(".key")
    {
        return false;
    }
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("zyl" | "toml" | "css" | "json" | "md" | "html" | "sql" | "txt" | "yaml" | "yml")
    ) || matches!(name, "Dockerfile" | "Makefile" | "README" | "LICENSE")
}

fn safe_editor_path(
    root: &Path,
    relative: &str,
    allow_missing_leaf: bool,
) -> Result<PathBuf, (u16, String)> {
    if relative.is_empty() || relative.len() > 512 || relative.contains(['\\', '\0']) {
        return Err((400, "invalid project-relative path".into()));
    }
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err((
            400,
            "path must stay inside the project and cannot use hidden or parent components".into(),
        ));
    }
    if !editor_file_allowed(relative_path) {
        return Err((
            403,
            "this file type is not editable in the web editor".into(),
        ));
    }
    let mut current = root.to_path_buf();
    let components = relative_path.components().collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(part) = component else {
            return Err((400, "invalid project path".into()));
        };
        let text = part
            .to_str()
            .ok_or((400, "project path must be valid UTF-8".into()))?;
        if text.starts_with('.')
            || matches!(
                text,
                "target" | "node_modules" | "vendor" | "dist" | "build"
            )
        {
            return Err((403, "hidden and generated paths are not editable".into()));
        }
        current.push(part);
        let is_leaf = index + 1 == components.len();
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err((
                    403,
                    "symbolic links are not editable in the web editor".into(),
                ))
            }
            Ok(metadata) if !is_leaf && !metadata.is_dir() => {
                return Err((400, "parent path is not a directory".into()))
            }
            Ok(metadata) if is_leaf && !metadata.is_file() => {
                return Err((400, "path is not a regular file".into()))
            }
            Ok(_) => {}
            Err(error)
                if error.kind() == io::ErrorKind::NotFound && is_leaf && allow_missing_leaf => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err((404, "file not found".into()))
            }
            Err(_) => return Err((500, "could not inspect project path".into())),
        }
    }
    let parent = current
        .parent()
        .ok_or((400, "invalid file path".into()))?
        .canonicalize()
        .map_err(|_| (404, "parent directory not found".into()))?;
    if !parent.starts_with(root) {
        return Err((403, "file is outside the project".into()));
    }
    if current.exists() {
        let canonical = current
            .canonicalize()
            .map_err(|_| (404, "file not found".into()))?;
        if !canonical.starts_with(root) {
            return Err((403, "file is outside the project".into()));
        }
    }
    Ok(current)
}

fn read_editor_file(
    state: &EditorState,
    relative: &str,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let path = safe_editor_path(&state.root, relative, false)?;
    let metadata = fs::metadata(&path).map_err(|_| (404, "file not found".into()))?;
    if metadata.len() > EDITOR_MAX_FILE_BYTES as u64 {
        return Err((413, "file exceeds the 1 MiB editor limit".into()));
    }
    let content =
        fs::read_to_string(&path).map_err(|_| (415, "file is not valid UTF-8 text".into()))?;
    let hash = content_hash(content.as_bytes());
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"path": relative, "content": content, "hash": hash}).to_string(),
    ))
}

fn write_existing_file(
    state: &EditorState,
    value: Value,
    create: bool,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let relative = value
        .get("path")
        .and_then(Value::as_str)
        .ok_or((400, "path is required".into()))?;
    let content = value
        .get("content")
        .and_then(Value::as_str)
        .ok_or((400, "content must be text".into()))?;
    if content.len() > EDITOR_MAX_FILE_BYTES {
        return Err((413, "file exceeds the 1 MiB editor limit".into()));
    }
    let path = safe_editor_path(&state.root, relative, create)?;
    if create {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                if error.kind() == io::ErrorKind::AlreadyExists {
                    (409, "file already exists".into())
                } else {
                    (500, "could not create file".into())
                }
            })?;
        file.write_all(content.as_bytes())
            .map_err(|_| (500, "could not write file".into()))?;
    } else {
        let base_hash = value
            .get("baseHash")
            .and_then(Value::as_str)
            .ok_or((400, "baseHash is required".into()))?;
        let current = fs::read(&path).map_err(|_| (404, "file not found".into()))?;
        if content_hash(&current) != base_hash {
            return Err((
                409,
                "file changed on disk since it was opened; reload it before saving".into(),
            ));
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&path)
            .map_err(|_| (500, "could not open file for writing".into()))?;
        file.write_all(content.as_bytes())
            .map_err(|_| (500, "could not write file".into()))?;
    }
    let hash = content_hash(content.as_bytes());
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"path": relative, "content": content, "hash": hash}).to_string(),
    ))
}

fn check_project(
    state: &EditorState,
    value: &Value,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let entry = value
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| path.ends_with(".zyl"))
        .or_else(|| state.root.join("main.zyl").is_file().then_some("main.zyl"))
        .ok_or((400, "choose a Zelyra entry file to check".into()))?;
    let path = safe_editor_path(&state.root, entry, false)?;
    let source =
        fs::read_to_string(&path).map_err(|_| (415, "entry file is not UTF-8 text".into()))?;
    begin_json_diagnostics(&path.to_string_lossy(), &source);
    let success = validate(&path.to_string_lossy()).is_ok();
    let diagnostics = finish_json_diagnostics()
        .into_iter()
        .map(|mut item| {
            if let Some(file) = item.get("file").and_then(Value::as_str) {
                let file_path = Path::new(file);
                let relative = file_path
                    .strip_prefix(&state.root)
                    .map(|path| path.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_else(|_| file.to_owned());
                item["file"] = Value::String(relative.clone());
                if let Ok(source_path) = safe_editor_path(&state.root, &relative, false) {
                    if let Ok(source) = fs::read_to_string(source_path) {
                        let start = item
                            .pointer("/span/start/offset")
                            .and_then(Value::as_u64)
                            .unwrap_or(0) as usize;
                        let end = item
                            .pointer("/span/end/offset")
                            .and_then(Value::as_u64)
                            .unwrap_or(start as u64) as usize;
                        item["editorSpan"] = json!({
                            "start": byte_to_utf16(&source, start),
                            "end": byte_to_utf16(&source, end),
                        });
                    }
                }
            }
            item
        })
        .collect::<Vec<_>>();
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"success": success, "diagnostics": diagnostics}).to_string(),
    ))
}

fn byte_to_utf16(source: &str, byte_offset: usize) -> usize {
    let mut boundary = byte_offset.min(source.len());
    while !source.is_char_boundary(boundary) {
        boundary = boundary.saturating_sub(1);
    }
    source[..boundary].encode_utf16().count()
}

fn format_editor_file(
    state: &EditorState,
    value: &Value,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let relative = value
        .get("path")
        .and_then(Value::as_str)
        .ok_or((400, "path is required".into()))?;
    if !relative.ends_with(".zyl") {
        return Err((400, "only .zyl files can be formatted".into()));
    }
    let path = safe_editor_path(&state.root, relative, false)?;
    let source = fs::read_to_string(&path).map_err(|_| (415, "file is not UTF-8 text".into()))?;
    let base_hash = value
        .get("baseHash")
        .and_then(Value::as_str)
        .ok_or((400, "baseHash is required".into()))?;
    if content_hash(source.as_bytes()) != base_hash {
        return Err((
            409,
            "file changed on disk since it was opened; reload it before formatting".into(),
        ));
    }
    let tokens = lex(&source).map_err(|error| (422, format!("E-LEX-001: {}", error.message)))?;
    parse(&tokens).map_err(|error| (422, format!("E-PARSE-001: {}", error.message)))?;
    let formatted = format_source(&source, &tokens);
    if formatted != source {
        fs::write(&path, formatted.as_bytes())
            .map_err(|_| (500, "could not write formatted file".into()))?;
    }
    let hash = content_hash(formatted.as_bytes());
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"path": relative, "content": formatted, "hash": hash}).to_string(),
    ))
}

fn content_hash(contents: &[u8]) -> String {
    let digest = Sha256::digest(contents);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn read_editor_request(stream: &mut TcpStream) -> io::Result<String> {
    const HEADER_LIMIT: usize = 64 * 1024;
    const BODY_LIMIT: usize = 1024 * 1024;
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 8192];
    let (header_end, body_len) = loop {
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "incomplete HTTP request",
            ));
        }
        bytes.extend_from_slice(&chunk[..count]);
        let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
            if bytes.len() > HEADER_LIMIT {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "HTTP headers exceed the size limit",
                ));
            }
            continue;
        };
        if end > HEADER_LIMIT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "HTTP headers exceed the size limit",
            ));
        }
        let headers = std::str::from_utf8(&bytes[..end]).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "HTTP headers are not UTF-8")
        })?;
        let mut body_len = 0_usize;
        let mut transfer_encoding = false;
        for line in headers.split("\r\n").skip(1) {
            let Some((name, value)) = line.split_once(':') else {
                continue;
            };
            if name.eq_ignore_ascii_case("content-length") {
                body_len = value.trim().parse().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "invalid Content-Length")
                })?;
            }
            if name.eq_ignore_ascii_case("transfer-encoding") {
                transfer_encoding = true;
            }
        }
        if transfer_encoding {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Transfer-Encoding is unsupported",
            ));
        }
        if body_len > BODY_LIMIT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "request body exceeds the 1 MiB limit",
            ));
        }
        break (end + 4, body_len);
    };
    let expected = header_end + body_len;
    while bytes.len() < expected {
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "incomplete HTTP request body",
            ));
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    bytes.truncate(expected);
    String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "HTTP request is not UTF-8"))
}

fn write_editor_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &str,
) -> io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Content",
        _ => "Internal Server Error",
    };
    let csp = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; font-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'";
    write!(stream, "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nCross-Origin-Resource-Policy: same-origin\r\nX-Frame-Options: DENY\r\nContent-Security-Policy: {csp}\r\n\r\n", body.len())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    fn project() -> EditorState {
        let directory = std::env::temp_dir().join(format!(
            "zelyra-editor-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).expect("create editor test project");
        fs::write(directory.join("zelyra.toml"), "name = \"editor-test\"\n")
            .expect("write project config");
        fs::write(directory.join("main.zyl"), "fn main() {}\n").expect("write entry source");
        EditorState {
            root: directory.canonicalize().expect("canonical project root"),
            token: "local-session-token".into(),
            authority: "127.0.0.1:4177".into(),
        }
    }

    fn request(method: &str, path: &str, headers: &[(&str, &str)]) -> Request {
        Request {
            method: method.into(),
            target: path.into(),
            path: path.split('?').next().unwrap_or(path).into(),
            headers: headers
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
            body: String::new(),
            remote_addr: None,
        }
    }

    #[test]
    fn editor_api_requires_session_token() {
        let state = project();
        let request = request("GET", "/__zelyra/editor/api/tree", &[]);
        let error = editor_response(&request, &state).expect_err("missing token must fail");
        assert_eq!(error.0, 403);
        let _ = fs::remove_dir_all(state.root);
    }

    #[test]
    fn editor_mutations_require_same_origin() {
        let state = project();
        let request = request(
            "POST",
            "/__zelyra/editor/api/file",
            &[
                ("x-zelyra-editor-token", "local-session-token"),
                ("origin", "https://attacker.invalid"),
            ],
        );
        let error = editor_response(&request, &state).expect_err("cross-origin mutation must fail");
        assert_eq!(error.0, 403);
        let _ = fs::remove_dir_all(state.root);
    }

    #[test]
    fn editor_paths_reject_traversal_and_secrets() {
        let state = project();
        for path in [
            "../outside.zyl",
            ".env",
            "nested/.env.local",
            "id_rsa.pem",
            "target/debug.zyl",
        ] {
            assert!(
                safe_editor_path(&state.root, path, true).is_err(),
                "path should be rejected: {path}"
            );
        }
        assert!(editor_file_allowed(Path::new("src/invoice.zyl")));
        assert!(!editor_file_allowed(Path::new(".env")));
        let _ = fs::remove_dir_all(state.root);
    }

    #[test]
    fn editor_save_uses_optimistic_hash_and_never_overwrites_external_edits() {
        let state = project();
        let path = state.root.join("main.zyl");
        let original = fs::read(&path).expect("read fixture");
        let stale = json!({"path":"main.zyl", "content":"fn main() { }\n", "baseHash":"stale"});
        assert_eq!(
            write_existing_file(&state, stale, false)
                .expect_err("stale save must fail")
                .0,
            409
        );
        assert_eq!(fs::read(&path).expect("read after conflict"), original);

        let updated = "fn main() { }\n";
        let request =
            json!({"path":"main.zyl", "content":updated, "baseHash":content_hash(&original)});
        write_existing_file(&state, request, false).expect("fresh save should succeed");
        assert_eq!(
            fs::read_to_string(&path).expect("read saved source"),
            updated
        );
        let _ = fs::remove_dir_all(state.root);
    }

    #[test]
    fn editor_can_create_only_new_project_files() {
        let state = project();
        let request = json!({"path":"src/new_file.zyl", "content":"fn main() {}\n"});
        assert_eq!(
            write_existing_file(&state, request.clone(), true)
                .expect_err("missing parent must fail")
                .0,
            404
        );
        fs::create_dir(state.root.join("src")).expect("create source directory");
        write_existing_file(&state, request.clone(), true).expect("create source file");
        assert_eq!(
            write_existing_file(&state, request, true)
                .expect_err("duplicate file must fail")
                .0,
            409
        );
        let _ = fs::remove_dir_all(state.root);
    }

    #[test]
    fn compiler_byte_offsets_are_converted_for_browser_utf16_positions() {
        let source = "aä🙂z";
        assert_eq!(byte_to_utf16(source, 0), 0);
        assert_eq!(byte_to_utf16(source, 1), 1);
        assert_eq!(byte_to_utf16(source, 3), 2);
        assert_eq!(byte_to_utf16(source, 7), 4);
        assert_eq!(byte_to_utf16(source, usize::MAX), 5);
    }
}
