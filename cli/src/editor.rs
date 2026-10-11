use super::*;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
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
const EDITOR_DATABASE_MAX_TABLES: usize = 500;
const EDITOR_DATABASE_MAX_COLUMNS: usize = 500;
const EDITOR_DATABASE_MAX_TOTAL_COLUMNS: usize = 10_000;
const EDITOR_DATABASE_MAX_FOREIGN_KEYS: usize = 5_000;
const EDITOR_TIMEOUT: Duration = Duration::from_secs(30);

struct EditorConnection(Arc<AtomicUsize>);

impl Drop for EditorConnection {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Release);
    }
}

fn acquire_editor_connection(active: &AtomicUsize) -> bool {
    let mut count = active.load(Ordering::Acquire);
    loop {
        if count >= EDITOR_MAX_CONNECTIONS {
            return false;
        }
        match active.compare_exchange_weak(count, count + 1, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return true,
            Err(current) => count = current,
        }
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
    let mut open_browser = true;
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
        } else if argument == "--no-open" {
            open_browser = false;
        } else if !argument.starts_with('-') && !directory_given {
            directory = PathBuf::from(argument);
            directory_given = true;
        } else {
            eprintln!(
                "error[E-EDITOR-001]: usage: zelyra editor [directory] [--port <port>] [--no-open]"
            );
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
    if open_browser && has_graphical_session() {
        if open_system_browser(&format!("http://{authority}{EDITOR_PATH}/#{token}")) {
            println!("Der Standardbrowser wird geöffnet. Mit Ctrl+C beendest du Studio.");
        } else {
            println!(
                "Kein Browser konnte automatisch geöffnet werden. Rufe die URL oben manuell auf."
            );
        }
    } else if open_browser {
        println!("Keine grafische Browsersitzung erkannt. Rufe die URL oben manuell auf. Mit Ctrl+C beendest du Studio.");
    } else {
        println!(
            "Browserstart übersprungen. Öffne die URL oben selbst. Mit Ctrl+C beendest du Studio."
        );
    }
    println!(
        "Studio akzeptiert ausschließlich Verbindungen von diesem Rechner. Es startet weder Datenbank noch Webanwendung."
    );
    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                let peer = stream.peer_addr().ok();
                if !peer.is_some_and(|address| address.ip().is_loopback()) {
                    continue;
                }
                if !acquire_editor_connection(&active) {
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

fn has_graphical_session() -> bool {
    if env::var_os("CI").is_some() {
        return false;
    }
    #[cfg(target_os = "linux")]
    {
        env::var_os("DISPLAY").is_some() || env::var_os("WAYLAND_DISPLAY").is_some()
    }
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        true
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        false
    }
}

fn open_system_browser(url: &str) -> bool {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd.exe");
        command.args(["/C", "start", "", url]);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(url);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };
    #[cfg(not(any(unix, target_os = "windows")))]
    let mut command = {
        let _ = url;
        return false;
    };

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
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
        ("GET", "search") => {
            let query = request
                .target
                .split_once('?')
                .map(|(_, query)| query)
                .unwrap_or_default();
            let values =
                zelyra_web::parse_urlencoded(query).map_err(|error| (400, error.message))?;
            search_editor_project(state, values.get("q").map(String::as_str).unwrap_or(""))
        }
        ("GET", "references") => {
            let query = request
                .target
                .split_once('?')
                .map(|(_, query)| query)
                .unwrap_or_default();
            let values =
                zelyra_web::parse_urlencoded(query).map_err(|error| (400, error.message))?;
            find_editor_identifier_occurrences(
                state,
                values.get("name").map(String::as_str).unwrap_or(""),
            )
        }
        ("GET", "project-map") => editor_project_map(state),
        ("GET", "database-schema") => database_schema_response(state),
        ("GET", "symbols") => {
            let query = request
                .target
                .split_once('?')
                .map(|(_, query)| query)
                .unwrap_or_default();
            let values =
                zelyra_web::parse_urlencoded(query).map_err(|error| (400, error.message))?;
            editor_symbols(
                state,
                values.get("path").map(String::as_str).unwrap_or("main.zyl"),
            )
        }
        ("POST", "locals") => local_editor_symbols(state, &parse_json_body(&request.body)?),
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
        ("POST", "file-rename") => rename_editor_file(state, parse_json_body(&request.body)?),
        ("DELETE", "file") => delete_editor_file(state, parse_json_body(&request.body)?),
        ("POST", "check") => check_project(state, &parse_json_body(&request.body)?),
        ("POST", "run") => run_editor_program(state, &parse_json_body(&request.body)?),
        ("POST", "format") => format_editor_file(state, &parse_json_body(&request.body)?),
        ("POST", "crud-preview") => preview_crud_resource(state, &parse_json_body(&request.body)?),
        ("POST", "crud-apply") => apply_crud_resource(state, &parse_json_body(&request.body)?),
        _ => Err((404, "not found".into())),
    }
}

fn editor_symbols(
    state: &EditorState,
    relative_entry: &str,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let entry = safe_editor_path(&state.root, "main.zyl", false)?;
    let requested = safe_editor_path(&state.root, relative_entry, false)?;
    let relative_path = requested
        .strip_prefix(&state.root)
        .map_err(|_| (400, "symbol path must stay inside the project".into()))?
        .to_string_lossy()
        .replace('\\', "/");
    let loaded = project::load(&entry.to_string_lossy()).map_err(|_| {
        (
            422,
            "project symbols are unavailable until the imported source parses".into(),
        )
    })?;
    let active_source = fs::read_to_string(&requested)
        .map_err(|_| (415, "active Zelyra source is not UTF-8 text".into()))?;
    let active_tokens = lex(&active_source).map_err(|_| {
        (
            422,
            "active source declarations are not parseable yet".into(),
        )
    })?;
    let active_program = parse(&active_tokens).map_err(|_| {
        (
            422,
            "active source declarations are not parseable yet".into(),
        )
    })?;
    let imported_paths = active_program
        .imports
        .iter()
        .map(|import| import.path.as_str())
        .collect::<BTreeSet<_>>();
    let mut project_sources = loaded.sources.clone();
    if !project_sources
        .iter()
        .any(|source| source.path == relative_path)
    {
        project_sources.push(project::ProjectSource {
            path: relative_path.clone(),
            text: active_source,
        });
    }
    let mut symbols = Vec::new();

    for source in project_sources.iter().filter(|source| {
        source.path == relative_path || imported_paths.contains(source.path.as_str())
    }) {
        let is_local = source.path == relative_path;
        let tokens = lex(&source.text)
            .map_err(|_| (422, "project declarations are not parseable yet".into()))?;
        let parsed = parse(&tokens)
            .map_err(|_| (422, "project declarations are not parseable yet".into()))?;
        for item in &parsed.functions {
            if is_local || item.is_public {
                push_editor_symbol(
                    &mut symbols,
                    &source.path,
                    &source.text,
                    &item.name,
                    "function",
                    "function",
                    item.span,
                );
            }
        }
        for item in &parsed.types {
            if is_local || item.is_public {
                push_editor_symbol(
                    &mut symbols,
                    &source.path,
                    &source.text,
                    &item.name,
                    "type",
                    "type",
                    item.span,
                );
            }
        }
        for item in &parsed.records {
            if is_local || item.is_public {
                push_editor_symbol(
                    &mut symbols,
                    &source.path,
                    &source.text,
                    &item.name,
                    "type",
                    "type",
                    item.span,
                );
            }
        }
        for item in &parsed.views {
            if is_local || item.is_public {
                push_editor_symbol(
                    &mut symbols,
                    &source.path,
                    &source.text,
                    &item.name,
                    "class",
                    "view",
                    item.span,
                );
            }
        }
        for item in &parsed.components {
            if is_local || item.is_public {
                push_editor_symbol(
                    &mut symbols,
                    &source.path,
                    &source.text,
                    &item.name,
                    "class",
                    "component",
                    item.span,
                );
            }
        }
        if is_local {
            symbols.extend(
                parsed
                    .forms
                    .iter()
                    .map(|item| json!({"label": item.name, "kind": "class", "detail": "form"})),
            );
            symbols.extend(
                parsed
                    .cruds
                    .iter()
                    .map(|item| json!({"label": item.name, "kind": "class", "detail": "crud"})),
            );
            symbols.extend(
                parsed
                    .pages
                    .iter()
                    .map(|item| json!({"label": item.path, "kind": "class", "detail": "page"})),
            );
            symbols.extend(
                parsed
                    .apis
                    .iter()
                    .map(|item| json!({"label": item.path, "kind": "class", "detail": "api"})),
            );
            symbols.extend(
                parsed.tableviews.iter().map(
                    |item| json!({"label": item.name, "kind": "class", "detail": "tableview"}),
                ),
            );
        }
    }
    symbols.extend(
        loaded
            .program
            .tables
            .iter()
            .map(|item| json!({"label": item.name, "kind": "class", "detail": "table"})),
    );
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"symbols": symbols}).to_string(),
    ))
}

fn editor_project_map(state: &EditorState) -> Result<(u16, &'static str, String), (u16, String)> {
    let mut project_files = Vec::new();
    collect_editor_files(&state.root, &state.root, 0, &mut project_files)?;
    if project_files.len() >= EDITOR_MAX_FILES {
        return Err((413, "project map is limited to 2,000 project files".into()));
    }
    let mut total_source_bytes = 0_u64;
    for relative in project_files.iter().filter(|path| path.ends_with(".zyl")) {
        let path = safe_editor_path(&state.root, relative, false)?;
        let size = fs::metadata(path)
            .map_err(|_| (500, "could not inspect project source".into()))?
            .len();
        total_source_bytes = total_source_bytes.saturating_add(size);
        if size > EDITOR_MAX_FILE_BYTES as u64 || total_source_bytes > 32 * 1024 * 1024 {
            return Err((
                413,
                "project map source is limited to 1 MiB per file and 32 MiB total".into(),
            ));
        }
    }
    let entry = safe_editor_path(&state.root, "main.zyl", false)?;
    let loaded = project::load(&entry.to_string_lossy()).map_err(|_| {
        (
            422,
            "project map is unavailable until imported source files parse".into(),
        )
    })?;
    let mut nodes = BTreeMap::<String, Value>::new();
    let mut edges = BTreeSet::<(String, String, String)>::new();
    let mut incomplete = false;
    let node_limit = 500;
    for module in &loaded.modules {
        let id = format!("module:{}", module.path);
        if nodes.len() >= node_limit {
            incomplete = true;
            break;
        }
        nodes.insert(
            id.clone(),
            json!({"id": id, "kind": "module", "label": module.path, "path": module.path}),
        );
        for import in &module.imports {
            let target = format!("module:{}", import.path);
            if loaded
                .modules
                .iter()
                .any(|candidate| candidate.path == import.path)
            {
                edges.insert((id.clone(), target, "imports".into()));
            } else {
                incomplete = true;
            }
        }
    }

    let mut tables_by_name = BTreeMap::<String, Vec<String>>::new();
    let mut cruds = Vec::<(String, String)>::new();
    let mut api_handlers = Vec::<(String, String)>::new();
    let mut functions_by_name = BTreeMap::<String, Vec<String>>::new();
    let mut views_by_path_and_name = BTreeMap::<(String, String), String>::new();
    for source in &loaded.sources {
        let tokens =
            lex(&source.text).map_err(|_| (422, "project source is not parseable".into()))?;
        let program =
            parse(&tokens).map_err(|_| (422, "project source is not parseable".into()))?;
        let module_id = format!("module:{}", source.path);

        for table in &program.tables {
            let id = format!("table:{}:{}", source.path, table.name);
            insert_project_map_node(
                &mut nodes,
                &mut incomplete,
                node_limit,
                &id,
                "table",
                &table.name,
                &source.path,
                &source.text,
                table.span,
            );
            tables_by_name
                .entry(table.name.clone())
                .or_default()
                .push(id.clone());
            edges.insert((module_id.clone(), id, "declares".into()));
        }
        for view in &program.views {
            let id = format!("view:{}:{}", source.path, view.name);
            insert_project_map_node(
                &mut nodes,
                &mut incomplete,
                node_limit,
                &id,
                "view",
                &view.name,
                &source.path,
                &source.text,
                view.span,
            );
            views_by_path_and_name.insert((source.path.clone(), view.name.clone()), id.clone());
            edges.insert((module_id.clone(), id, "declares".into()));
        }
        for crud in &program.cruds {
            let id = format!("crud:{}:{}", source.path, crud.name);
            insert_project_map_node(
                &mut nodes,
                &mut incomplete,
                node_limit,
                &id,
                "crud",
                &crud.name,
                &source.path,
                &source.text,
                crud.span,
            );
            edges.insert((module_id.clone(), id.clone(), "declares".into()));
            cruds.push((id, crud.table.clone()));
        }
        for page in &program.pages {
            let id = format!("page:{}:{}", source.path, page.path);
            insert_project_map_node(
                &mut nodes,
                &mut incomplete,
                node_limit,
                &id,
                "page",
                &page.path,
                &source.path,
                &source.text,
                page.span,
            );
            edges.insert((module_id.clone(), id.clone(), "declares".into()));
            if let Some(view) = &page.view {
                if let Some(target) =
                    views_by_path_and_name.get(&(source.path.clone(), view.clone()))
                {
                    edges.insert((id, target.clone(), "uses view".into()));
                } else {
                    incomplete = true;
                }
            }
        }
        for api in &program.apis {
            let id = format!("api:{}:{}:{}", source.path, api.method, api.path);
            insert_project_map_node(
                &mut nodes,
                &mut incomplete,
                node_limit,
                &id,
                "api",
                &format!("{} {}", api.method, api.path),
                &source.path,
                &source.text,
                api.span,
            );
            edges.insert((module_id.clone(), id.clone(), "declares".into()));
            if let Some(handler) = &api.handler {
                api_handlers.push((id, handler.clone()));
            }
        }
        for function in &program.functions {
            let id = format!("function:{}:{}", source.path, function.name);
            insert_project_map_node(
                &mut nodes,
                &mut incomplete,
                node_limit,
                &id,
                "function",
                &function.name,
                &source.path,
                &source.text,
                function.span,
            );
            edges.insert((module_id.clone(), id.clone(), "declares".into()));
            functions_by_name
                .entry(function.name.clone())
                .or_default()
                .push(id);
        }
    }

    for (crud, table) in cruds {
        match tables_by_name.get(&table) {
            Some(targets) if targets.len() == 1 => {
                edges.insert((crud, targets[0].clone(), "uses table".into()));
            }
            _ => incomplete = true,
        }
    }
    for (route, handler) in api_handlers {
        match functions_by_name.get(&handler) {
            Some(targets) if targets.len() == 1 => {
                edges.insert((route, targets[0].clone(), "handled by".into()));
            }
            _ => incomplete = true,
        }
    }
    edges.retain(|(from, to, _)| {
        let keep = nodes.contains_key(from) && nodes.contains_key(to);
        incomplete |= !keep;
        keep
    });
    if edges.len() > 1_000 {
        incomplete = true;
        edges = edges.into_iter().take(1_000).collect();
    }
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({
            "nodes": nodes.into_values().collect::<Vec<_>>(),
            "edges": edges.into_iter().map(|(from, to, label)| json!({"from": from, "to": to, "label": label})).collect::<Vec<_>>(),
            "incomplete": incomplete,
        })
        .to_string(),
    ))
}

fn insert_project_map_node(
    nodes: &mut BTreeMap<String, Value>,
    incomplete: &mut bool,
    limit: usize,
    id: &str,
    kind: &str,
    label: &str,
    path: &str,
    source: &str,
    span: zelyra_ast::Span,
) {
    if nodes.len() >= limit {
        *incomplete = true;
        return;
    }
    nodes.insert(
        id.into(),
        json!({
            "id": id,
            "kind": kind,
            "label": label,
            "path": path,
            "line": span.line,
            "position": byte_to_utf16(source, span.start),
        }),
    );
}

fn push_editor_symbol(
    symbols: &mut Vec<Value>,
    path: &str,
    source: &str,
    name: &str,
    kind: &str,
    detail: &str,
    span: zelyra_ast::Span,
) {
    let Some(declaration) = source.get(span.start..span.end.min(source.len())) else {
        return;
    };
    let Some(offset) = declaration.find(name) else {
        return;
    };
    let byte_position = span.start + offset;
    let Some(prefix) = source.get(..byte_position) else {
        return;
    };
    symbols.push(json!({
        "label": name,
        "kind": kind,
        "detail": detail,
        "path": path,
        "position": prefix.encode_utf16().count(),
    }));
}

fn local_editor_symbols(
    state: &EditorState,
    value: &Value,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let relative = value
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| path.ends_with(".zyl"))
        .ok_or((400, "choose a Zelyra source file".into()))?;
    let _path = safe_editor_path(&state.root, relative, false)?;
    let source = value
        .get("source")
        .and_then(Value::as_str)
        .ok_or((400, "source is required".into()))?;
    if source.len() > EDITOR_MAX_FILE_BYTES {
        return Err((413, "source exceeds the 1 MiB editor limit".into()));
    }
    let offset = value
        .get("offset")
        .and_then(Value::as_u64)
        .ok_or((400, "editor offset is required".into()))?;
    let byte_offset = utf16_to_byte(source, usize::try_from(offset).unwrap_or(usize::MAX));
    let tokens = lex(source).map_err(|_| (422, "source is not parseable yet".into()))?;
    let program = parse(&tokens).map_err(|_| (422, "source is not parseable yet".into()))?;
    let mut locals = Vec::new();
    if let Some(function) = program.functions.iter().find(|function| {
        function.body.span.start <= byte_offset && byte_offset <= function.body.span.end
    }) {
        for parameter in &function.params {
            locals.push(json!({
                "label": parameter.name,
                "kind": "variable",
                "detail": "parameter",
                "valueType": parameter.ty.to_string(),
            }));
        }
        collect_visible_bindings(&function.body, byte_offset, &mut locals);
    }
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"symbols": locals}).to_string(),
    ))
}

fn utf16_to_byte(source: &str, requested: usize) -> usize {
    let mut utf16_offset = 0;
    for (byte_offset, character) in source.char_indices() {
        if utf16_offset >= requested {
            return byte_offset;
        }
        utf16_offset += character.len_utf16();
        if utf16_offset > requested {
            return byte_offset;
        }
    }
    source.len()
}

fn collect_visible_bindings(block: &zelyra_ast::Block, offset: usize, output: &mut Vec<Value>) {
    if offset < block.span.start || offset > block.span.end {
        return;
    }
    for statement in &block.statements {
        match statement {
            zelyra_ast::Stmt::Let { name, ty, span, .. } if span.start < offset => {
                output.push(json!({
                    "label": name,
                    "kind": "variable",
                    "detail": "local value",
                    "valueType": ty.as_ref().map(ToString::to_string),
                }))
            }
            zelyra_ast::Stmt::If {
                then_block,
                else_block,
                span,
                ..
            } if span.start < offset => {
                collect_visible_bindings(then_block, offset, output);
                if let Some(block) = else_block {
                    collect_visible_bindings(block, offset, output);
                }
            }
            zelyra_ast::Stmt::While { body, span, .. }
            | zelyra_ast::Stmt::Loop { body, span, .. }
            | zelyra_ast::Stmt::Transaction { body, span }
            | zelyra_ast::Stmt::Parallel { body, span }
                if span.start < offset =>
            {
                collect_visible_bindings(body, offset, output);
            }
            zelyra_ast::Stmt::For {
                name, body, span, ..
            } if span.start < offset && body.span.start <= offset && offset <= body.span.end => {
                output.push(json!({
                    "label": name,
                    "kind": "variable",
                    "detail": "loop value",
                    "valueType": Value::Null,
                }));
                collect_visible_bindings(body, offset, output);
            }
            zelyra_ast::Stmt::Match { arms, span, .. } if span.start < offset => {
                for arm in arms
                    .iter()
                    .filter(|arm| arm.span.start <= offset && offset <= arm.span.end)
                {
                    collect_pattern_bindings(&arm.pattern, output);
                    collect_visible_bindings(&arm.body, offset, output);
                }
            }
            _ => {}
        }
    }
}

fn collect_pattern_bindings(pattern: &zelyra_ast::Pattern, output: &mut Vec<Value>) {
    match &pattern.kind {
        zelyra_ast::PatternKind::Variable(name) => output.push(json!({
            "label": name,
            "kind": "variable",
            "detail": "match value",
            "valueType": Value::Null,
        })),
        zelyra_ast::PatternKind::Constructor {
            inner: Some(inner), ..
        } => {
            collect_pattern_bindings(inner, output);
        }
        _ => {}
    }
}

fn editor_database_schemas(
    state: &EditorState,
) -> Result<(zelyra_database::Schema, zelyra_database::Schema, Value), (u16, String)> {
    let entry = state.root.join("main.zyl");
    if !entry.is_file() {
        return Err((
            422,
            "database tools need a project entry file named main.zyl".into(),
        ));
    }
    let entry_text = entry.to_string_lossy().into_owned();
    let declared = database_cli::load_schema(&entry_text).map_err(|_| {
        (
            422,
            "the project schema could not be read; fix project diagnostics first".into(),
        )
    })?;
    if declared
        .database
        .as_ref()
        .is_some_and(|database| database.engine.eq_ignore_ascii_case("mysql"))
    {
        return Err((501, "MySQL schema introspection is not supported yet; the current MySQL runtime is limited to typed parameterized queries".into()));
    }
    let database_name = declared
        .database
        .as_ref()
        .map(|database| database.name.as_str());
    let environment_file = state.root.join(".env");
    let from_environment = super::database_url_from_schema(&declared);
    let from_file = if from_environment.is_none() && environment_file.is_file() {
        super::database_url_from_env_file(&environment_file.to_string_lossy(), database_name)
            .map_err(|_| (422, "the project environment file could not be read".into()))?
            .map(|(url, _)| url)
    } else {
        None
    };
    let database_url = from_environment.or(from_file).ok_or((
        422,
        "database connection is not configured; set DATABASE_URL in the environment or project .env file".into(),
    ))?;
    let live =
        database_cli::inspect_for_backend(declared.backend(), &database_url).map_err(|_| {
            (
                502,
                "database metadata could not be loaded; check the local database connection".into(),
            )
        })?;
    let total_columns = declared
        .tables
        .iter()
        .chain(&live.tables)
        .map(|table| table.columns.len())
        .sum::<usize>();
    let total_foreign_keys = declared
        .tables
        .iter()
        .chain(&live.tables)
        .map(|table| table.foreign_keys.len())
        .sum::<usize>();
    if declared.tables.len() > EDITOR_DATABASE_MAX_TABLES
        || live.tables.len() > EDITOR_DATABASE_MAX_TABLES
        || total_columns > EDITOR_DATABASE_MAX_TOTAL_COLUMNS
        || total_foreign_keys > EDITOR_DATABASE_MAX_FOREIGN_KEYS
        || declared
            .tables
            .iter()
            .chain(&live.tables)
            .any(|table| table.columns.len() > EDITOR_DATABASE_MAX_COLUMNS)
    {
        return Err((
            413,
            "database schema exceeds the Studio display limit of 500 tables, 500 columns per table, 10,000 columns, or 5,000 relationships"
                .into(),
        ));
    }
    let project = load_project(&entry_text)
        .map_err(|_| (422, "the project modules could not be loaded".into()))?;
    let owners = module_declaration_owners(&project.program);
    let mut map = database_cli::database_map_json(&declared, &live, &owners);
    map["crud_resources"] = json!(project
        .program
        .cruds
        .iter()
        .map(|crud| json!({"table": crud.table, "name": crud.name}))
        .collect::<Vec<_>>());
    let mut crud_targets = vec!["main.zyl".to_owned()];
    crud_targets.extend(project.modules.iter().map(|module| module.path.clone()));
    crud_targets.sort();
    crud_targets.dedup();
    map["crud_targets"] = json!(crud_targets);
    Ok((declared, live, map))
}

fn database_schema_response(
    state: &EditorState,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let (_declared, _live, map) = editor_database_schemas(state)?;
    Ok((200, "application/json; charset=utf-8", map.to_string()))
}

fn preview_crud_resource(
    state: &EditorState,
    value: &Value,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let preview = build_crud_preview(state, value)?;
    Ok((200, "application/json; charset=utf-8", preview.to_string()))
}

fn apply_crud_resource(
    state: &EditorState,
    value: &Value,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let preview = build_crud_preview(state, value)?;
    if !preview["success"].as_bool().unwrap_or(false) {
        return Err((
            422,
            "generated CRUD source did not pass the project compiler".into(),
        ));
    }
    let path = preview["path"]
        .as_str()
        .ok_or((500, "generated preview is missing its target path".into()))?;
    let candidate = preview["candidate"]
        .as_str()
        .ok_or((500, "generated preview is missing its source".into()))?;
    let base_hash = preview["baseHash"]
        .as_str()
        .ok_or((500, "generated preview is missing its file hash".into()))?;
    let saved = write_existing_file(
        state,
        json!({"path": path, "content": candidate, "baseHash": base_hash}),
        false,
    )?;
    let mut response: Value = serde_json::from_str(&saved.2)
        .map_err(|_| (500, "saved editor response could not be encoded".into()))?;
    response["applied"] = Value::Bool(true);
    Ok((200, "application/json; charset=utf-8", response.to_string()))
}

fn build_crud_preview(state: &EditorState, value: &Value) -> Result<Value, (u16, String)> {
    let target = value
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("main.zyl");
    if !target.ends_with(".zyl") {
        return Err((400, "CRUD source must be added to a .zyl file".into()));
    }
    let target_path = safe_editor_path(&state.root, target, false)?;
    let current = fs::read_to_string(&target_path)
        .map_err(|_| (415, "target source is not UTF-8 text".into()))?;
    let base_hash = content_hash(current.as_bytes());
    if value.get("baseHash").and_then(Value::as_str) != Some(base_hash.as_str()) {
        return Err((
            409,
            "target file changed; reload it before creating the preview".into(),
        ));
    }
    let table_name = value
        .get("table")
        .and_then(Value::as_str)
        .ok_or((400, "table is required".into()))?;
    let fields = value
        .get("fields")
        .and_then(Value::as_array)
        .ok_or((400, "choose at least one field".into()))?
        .iter()
        .map(|field| {
            field
                .as_str()
                .map(str::to_owned)
                .ok_or((400, "field names must be text".into()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if fields.is_empty() || fields.len() > EDITOR_DATABASE_MAX_COLUMNS {
        return Err((400, "choose between 1 and 500 fields".into()));
    }
    let (declared, live, _) = editor_database_schemas(state)?;
    let table = declared
        .tables
        .iter()
        .find(|table| table.name == table_name)
        .ok_or((404, "table is not declared in the project".into()))?;
    let live_table = live
        .tables
        .iter()
        .find(|table| table.name == table_name)
        .ok_or((
            409,
            "table is declared in source but missing from the live database".into(),
        ))?;
    let live_columns = live_table
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect::<HashSet<_>>();
    let mut unique_fields = HashSet::new();
    for field in &fields {
        if !unique_fields.insert(field.as_str())
            || !table.columns.iter().any(|column| column.name == *field)
            || !live_columns.contains(field.as_str())
            || !is_zelyra_identifier(field)
        {
            return Err((
                400,
                format!("field `{field}` is not a unique, declared live column"),
            ));
        }
    }
    let project = load_project(&state.root.join("main.zyl").to_string_lossy())
        .map_err(|_| (422, "project modules could not be loaded".into()))?;
    if target != "main.zyl" && !project.modules.iter().any(|module| module.path == target) {
        return Err((
            400,
            "choose a Zelyra source file imported by main.zyl".into(),
        ));
    }
    if project
        .program
        .cruds
        .iter()
        .any(|crud| crud.table == table_name)
    {
        return Err((
            409,
            "this table already has a generated CRUD resource".into(),
        ));
    }
    let default_name = resource_name_from_table(table_name);
    let resource_name = value
        .get("resourceName")
        .and_then(Value::as_str)
        .unwrap_or(&default_name);
    if !is_zelyra_identifier(resource_name)
        || project
            .program
            .cruds
            .iter()
            .any(|crud| crud.name == resource_name)
    {
        return Err((
            400,
            "resource name must be a unique Zelyra identifier".into(),
        ));
    }
    let title = value
        .get("title")
        .and_then(Value::as_str)
        .filter(|title| !title.trim().is_empty())
        .unwrap_or(resource_name);
    if title.len() > 120 || title.chars().any(char::is_control) {
        return Err((400, "title must be at most 120 printable characters".into()));
    }
    let snippet = generated_crud_source(resource_name, table_name, title, &fields);
    let candidate = format!(
        "{}{}{}",
        current,
        if current.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        },
        snippet
    );
    let (success, diagnostics) = validate_editor_candidate(state, target, &candidate)?;
    let diff = snippet
        .lines()
        .map(|line| format!("+ {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let incoming_relationships = declared
        .tables
        .iter()
        .flat_map(|source| {
            source
                .foreign_keys
                .iter()
                .filter(move |key| key.referenced_table == table_name)
                .map(move |key| {
                    json!({
                        "fromTable": source.name,
                        "fromColumn": key.column,
                        "toTable": key.referenced_table,
                        "toColumn": key.referenced_column,
                    })
                })
        })
        .collect::<Vec<_>>();
    let relationships = table
        .foreign_keys
        .iter()
        .map(|key| {
            json!({
                "fromTable": table_name,
                "fromColumn": key.column,
                "toTable": key.referenced_table,
                "toColumn": key.referenced_column,
            })
        })
        .chain(incoming_relationships)
        .collect::<Vec<_>>();
    let base_route = format!("/{table_name}");
    let id_route = format!("{base_route}/{{id}}");
    let affected_routes = [
        ("GET", base_route.clone()),
        ("GET", format!("{base_route}/new")),
        ("POST", format!("{base_route}/new")),
        ("GET", id_route.clone()),
        ("GET", format!("{id_route}/edit")),
        ("POST", format!("{id_route}/edit")),
        ("POST", format!("{id_route}/delete")),
        ("POST", format!("{id_route}/restore")),
    ];
    Ok(json!({
        "success": success,
        "path": target,
        "baseHash": base_hash,
        "snippet": snippet,
        "candidate": candidate,
        "diff": diff,
        "diagnostics": diagnostics,
        "table": table_name,
        "resourceName": resource_name,
        "fields": fields,
        "impact": {
            "files": [target],
            "tables": [table_name],
            "relationships": relationships,
            "routes": affected_routes.iter().map(|(method, path)| json!({"method": method, "path": path})).collect::<Vec<_>>(),
        },
    }))
}

fn validate_editor_candidate(
    state: &EditorState,
    target: &str,
    candidate: &str,
) -> Result<(bool, Vec<Value>), (u16, String)> {
    let cleanup = create_editor_preview_directory()?;
    let temporary_root = cleanup.0.clone();
    let mut file_count = 0;
    copy_editor_sources(
        &state.root,
        &state.root,
        &temporary_root,
        0,
        &mut file_count,
    )?;
    let relative = Path::new(target);
    let candidate_path = temporary_root.join(relative);
    let parent = candidate_path
        .parent()
        .ok_or((400, "invalid preview path".into()))?;
    fs::create_dir_all(parent)
        .map_err(|_| (500, "could not prepare the compiler preview".into()))?;
    fs::write(&candidate_path, candidate)
        .map_err(|_| (500, "could not write the private compiler preview".into()))?;
    let entry = temporary_root.join("main.zyl");
    if !entry.is_file() {
        return Err((422, "project entry file main.zyl is missing".into()));
    }
    let entry_source = fs::read_to_string(&entry)
        .map_err(|_| (415, "project entry file is not UTF-8 text".into()))?;
    begin_json_diagnostics(&entry.to_string_lossy(), &entry_source);
    let success = validate(&entry.to_string_lossy()).is_ok();
    let diagnostics = finish_json_diagnostics()
        .into_iter()
        .map(|mut item| {
            if let Some(file) = item.get("file").and_then(Value::as_str) {
                let relative = Path::new(file)
                    .strip_prefix(&temporary_root)
                    .map(|path| path.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_else(|_| "main.zyl".into());
                item["file"] = Value::String(relative.clone());
                let diagnostic_source = if relative == target {
                    Some(candidate.to_owned())
                } else {
                    safe_editor_path(&state.root, &relative, false)
                        .ok()
                        .and_then(|path| fs::read_to_string(path).ok())
                };
                if let Some(diagnostic_source) = diagnostic_source {
                    let start = item
                        .pointer("/span/start/offset")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize;
                    let end = item
                        .pointer("/span/end/offset")
                        .and_then(Value::as_u64)
                        .unwrap_or(start as u64) as usize;
                    item["editorSpan"] = json!({
                        "start": byte_to_utf16(&diagnostic_source, start),
                        "end": byte_to_utf16(&diagnostic_source, end),
                    });
                }
            }
            item
        })
        .collect();
    drop(cleanup);
    Ok((success, diagnostics))
}

fn copy_editor_sources(
    root: &Path,
    directory: &Path,
    destination: &Path,
    depth: usize,
    file_count: &mut usize,
) -> Result<(), (u16, String)> {
    if depth > EDITOR_MAX_DEPTH {
        return Err((
            413,
            "project exceeds the compiler preview depth limit".into(),
        ));
    }
    let entries = fs::read_dir(directory).map_err(|_| {
        (
            500,
            "could not read project sources for compiler preview".into(),
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|_| (500, "could not read project source entry".into()))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.starts_with('.')
            || matches!(
                name,
                "target" | "node_modules" | "vendor" | "dist" | "build"
            )
        {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|_| (500, "could not inspect project source entry".into()))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            copy_editor_sources(root, &entry.path(), destination, depth + 1, file_count)?;
            continue;
        }
        if !metadata.is_file()
            || entry.path().extension().and_then(|value| value.to_str()) != Some("zyl")
        {
            continue;
        }
        if metadata.len() > EDITOR_MAX_FILE_BYTES as u64 {
            return Err((
                413,
                "a source file exceeds the 1 MiB compiler preview limit".into(),
            ));
        }
        *file_count += 1;
        if *file_count > EDITOR_MAX_FILES {
            return Err((
                413,
                "project exceeds the compiler preview file limit".into(),
            ));
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| (403, "project source resolves outside the project".into()))?
            .to_path_buf();
        let target = destination.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|_| (500, "could not prepare compiler preview directories".into()))?;
        }
        fs::copy(entry.path(), target).map_err(|_| {
            (
                500,
                "could not copy project source into compiler preview".into(),
            )
        })?;
    }
    Ok(())
}

struct EditorPreviewDirectory(PathBuf);

fn create_editor_preview_directory() -> Result<EditorPreviewDirectory, (u16, String)> {
    for _ in 0..8 {
        let mut random = [0_u8; 16];
        OsRng
            .try_fill_bytes(&mut random)
            .map_err(|_| (500, "could not create a private compiler preview".into()))?;
        let name = random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let path = std::env::temp_dir().join(format!(
            "zelyra-studio-preview-{}-{name}",
            std::process::id()
        ));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&path) {
            Ok(()) => return Ok(EditorPreviewDirectory(path)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err((500, "could not create a private compiler preview".into())),
        }
    }
    Err((500, "could not create a private compiler preview".into()))
}

impl Drop for EditorPreviewDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn generated_crud_source(resource: &str, table: &str, title: &str, fields: &[String]) -> String {
    let fields = fields
        .iter()
        .map(|field| format!("            {field}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "crud {resource} -> {table} {{\n    title: \"{}\"\n\n    view {{\n        fields {{\n{fields}\n        }}\n\n        list {{\n            mode: cards\n        }}\n\n        detail {{\n            mode: cards\n        }}\n\n        form {{\n            mode: cards\n        }}\n    }}\n}}",
        title.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

fn is_zelyra_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn resource_name_from_table(table: &str) -> String {
    table
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            characters
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + characters.as_str())
                .unwrap_or_default()
        })
        .collect()
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

fn search_editor_project(
    state: &EditorState,
    query: &str,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let query = query.trim();
    if query.is_empty() || query.chars().count() > 120 {
        return Err((400, "search text must contain 1 to 120 characters".into()));
    }
    let mut files = Vec::new();
    collect_editor_files(&state.root, &state.root, 0, &mut files)?;
    files.sort();
    let mut matches = Vec::new();
    let mut truncated = files.len() >= EDITOR_MAX_FILES;
    'files: for relative in files.iter().filter(|path| path.ends_with(".zyl")) {
        let path = safe_editor_path(&state.root, relative, false)?;
        let source = match fs::read_to_string(path) {
            Ok(source) => source,
            Err(_) => continue,
        };
        for (line_index, line) in source.lines().enumerate() {
            for (byte_column, _) in line.match_indices(query) {
                if matches.len() == 500 {
                    truncated = true;
                    break 'files;
                }
                let prefix = &line[..byte_column];
                let mut snippet_start = byte_column.saturating_sub(100);
                while !line.is_char_boundary(snippet_start) {
                    snippet_start += 1;
                }
                let mut snippet_end = byte_column
                    .saturating_add(query.len())
                    .saturating_add(140)
                    .min(line.len());
                while !line.is_char_boundary(snippet_end) {
                    snippet_end -= 1;
                }
                let snippet = format!(
                    "{}{}{}",
                    if snippet_start > 0 { "…" } else { "" },
                    &line[snippet_start..snippet_end],
                    if snippet_end < line.len() { "…" } else { "" }
                );
                matches.push(json!({
                    "path": relative,
                    "line": line_index + 1,
                    "column": prefix.encode_utf16().count() + 1,
                    "snippet": snippet,
                }));
            }
        }
    }
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"matches": matches, "truncated": truncated}).to_string(),
    ))
}

fn find_editor_identifier_occurrences(
    state: &EditorState,
    name: &str,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let mut chars = name.chars();
    let valid_start = chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_alphabetic());
    if !valid_start
        || chars.clone().any(|ch| ch != '_' && !ch.is_alphanumeric())
        || name.len() > 120
    {
        return Err((400, "choose a valid Zelyra identifier to find".into()));
    }

    let mut files = Vec::new();
    collect_editor_files(&state.root, &state.root, 0, &mut files)?;
    files.sort();
    let mut matches = Vec::new();
    let mut truncated = files.len() >= EDITOR_MAX_FILES;
    let mut scanned_bytes = 0_usize;
    const MAX_REFERENCE_SCAN_BYTES: usize = 32 * 1024 * 1024;

    'files: for relative in files.iter().filter(|path| path.ends_with(".zyl")) {
        let path = safe_editor_path(&state.root, relative, false)?;
        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => {
                truncated = true;
                continue;
            }
        };
        let Ok(file_bytes) = usize::try_from(metadata.len()) else {
            truncated = true;
            break;
        };
        if file_bytes > EDITOR_MAX_FILE_BYTES
            || scanned_bytes.saturating_add(file_bytes) > MAX_REFERENCE_SCAN_BYTES
        {
            truncated = true;
            break;
        }
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(_) => {
                truncated = true;
                continue;
            }
        };
        scanned_bytes = scanned_bytes.saturating_add(source.len());
        let tokens = match lex(&source) {
            Ok(tokens) => tokens,
            Err(_) => {
                truncated = true;
                continue;
            }
        };
        for token in tokens {
            if !matches!(&token.kind, TokenKind::Ident(identifier) if identifier == name) {
                continue;
            }
            if matches.len() == 500 {
                truncated = true;
                break 'files;
            }
            let line_number = token.span.line.max(1);
            let Some(line) = source.lines().nth(line_number - 1) else {
                continue;
            };
            let column = source
                .get(..token.span.start.min(source.len()))
                .and_then(|prefix| {
                    prefix
                        .rsplit_once('\n')
                        .map(|(_, tail)| tail)
                        .or(Some(prefix))
                })
                .map(|prefix| prefix.encode_utf16().count() + 1)
                .unwrap_or(1);
            let snippet = if line.encode_utf16().count() <= 240 {
                line.to_owned()
            } else {
                let start = token.span.start.min(source.len());
                let line_start = source[..start].rfind('\n').map_or(0, |index| index + 1);
                let local_start = start.saturating_sub(line_start);
                let mut from = local_start.saturating_sub(100);
                while !line.is_char_boundary(from) {
                    from = from.saturating_sub(1);
                }
                let mut to = local_start.saturating_add(140).min(line.len());
                while !line.is_char_boundary(to) {
                    to -= 1;
                }
                format!(
                    "{}{}{}",
                    if from > 0 { "…" } else { "" },
                    &line[from..to],
                    if to < line.len() { "…" } else { "" }
                )
            };
            matches.push(json!({
                "path": relative,
                "line": line_number,
                "column": column,
                "snippet": snippet,
            }));
        }
    }
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"matches": matches, "truncated": truncated, "mode": "lexical-identifiers"})
            .to_string(),
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
        if file.write_all(content.as_bytes()).is_err() {
            drop(file);
            let _ = fs::remove_file(&path);
            return Err((500, "could not write file".into()));
        }
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
        super::edit::apply_atomically(&path.to_string_lossy(), content)
            .map_err(|_| (500, "could not atomically replace file".into()))?;
    }
    let hash = content_hash(content.as_bytes());
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"path": relative, "content": content, "hash": hash}).to_string(),
    ))
}

fn editor_file_hash(
    state: &EditorState,
    relative: &str,
    field: &str,
    value: &Value,
) -> Result<PathBuf, (u16, String)> {
    let path = safe_editor_path(&state.root, relative, false)?;
    let expected = value
        .get(field)
        .and_then(Value::as_str)
        .ok_or((400, format!("{field} is required")))?;
    let current = fs::read(&path).map_err(|_| (404, "file not found".into()))?;
    if content_hash(&current) != expected {
        return Err((
            409,
            "file changed on disk; reload it before changing its path".into(),
        ));
    }
    Ok(path)
}

fn protected_editor_file(relative: &str) -> bool {
    matches!(relative, "main.zyl" | "zelyra.toml")
}

fn rename_editor_file(
    state: &EditorState,
    value: Value,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let relative = value
        .get("path")
        .and_then(Value::as_str)
        .ok_or((400, "path is required".into()))?;
    let new_relative = value
        .get("newPath")
        .and_then(Value::as_str)
        .ok_or((400, "newPath is required".into()))?;
    if protected_editor_file(relative) || protected_editor_file(new_relative) {
        return Err((
            403,
            "the project entry file and project configuration cannot be renamed".into(),
        ));
    }
    let source = editor_file_hash(state, relative, "baseHash", &value)?;
    let target = safe_editor_path(&state.root, new_relative, true)?;
    if source == target {
        return Err((400, "newPath must differ from path".into()));
    }
    // hard_link creates the destination atomically only when it does not exist,
    // so a concurrent file cannot be silently overwritten by a rename.
    fs::hard_link(&source, &target).map_err(|error| {
        if error.kind() == io::ErrorKind::AlreadyExists {
            (409, "destination file already exists".into())
        } else {
            (500, "could not create renamed file; source and destination must be on the same filesystem".into())
        }
    })?;
    let renamed =
        fs::read(&target).map_err(|_| (500, "renamed file could not be verified".into()))?;
    if content_hash(&renamed) != value["baseHash"].as_str().unwrap_or_default() {
        let _ = fs::remove_file(&target);
        return Err((
            409,
            "file changed during rename; reload it before retrying".into(),
        ));
    }
    if fs::remove_file(&source).is_err() {
        let _ = fs::remove_file(&target);
        return Err((500, "could not finish file rename".into()));
    }
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"path": relative, "newPath": new_relative, "hash": content_hash(&renamed)})
            .to_string(),
    ))
}

fn delete_editor_file(
    state: &EditorState,
    value: Value,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let relative = value
        .get("path")
        .and_then(Value::as_str)
        .ok_or((400, "path is required".into()))?;
    if protected_editor_file(relative) {
        return Err((
            403,
            "the project entry file and project configuration cannot be deleted".into(),
        ));
    }
    let path = editor_file_hash(state, relative, "baseHash", &value)?;
    fs::remove_file(&path).map_err(|_| (500, "could not delete file".into()))?;
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"path": relative, "deleted": true}).to_string(),
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
    let source = value
        .get("source")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .map(Ok)
        .unwrap_or_else(|| {
            fs::read_to_string(&path).map_err(|_| (415, "entry file is not UTF-8 text".into()))
        })?;
    if value.get("source").and_then(Value::as_str).is_some() {
        let (success, diagnostics) = validate_editor_candidate(state, entry, &source)?;
        return Ok((
            200,
            "application/json; charset=utf-8",
            json!({"success": success, "diagnostics": diagnostics}).to_string(),
        ));
    }
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

fn run_editor_program(
    state: &EditorState,
    value: &Value,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let relative = value
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| path.ends_with(".zyl"))
        .ok_or((400, "choose a Zelyra entry file to run".into()))?;
    let path = safe_editor_path(&state.root, relative, false)?;
    if fs::metadata(&path)
        .map_err(|_| (404, "entry file could not be read".into()))?
        .len()
        > EDITOR_MAX_FILE_BYTES as u64
    {
        return Err((413, "entry file exceeds the 1 MiB editor limit".into()));
    }
    let executable =
        env::current_exe().map_err(|_| (500, "could not locate the running Zelyra CLI".into()))?;
    let mut child = Command::new(executable)
        .arg("run")
        .arg(relative)
        .current_dir(&state.root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| (500, "could not start the Zelyra program".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or((500, "could not capture program output".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or((500, "could not capture program errors".into()))?;
    let stdout_reader = read_editor_output(stdout);
    let stderr_reader = read_editor_output(stderr);
    let started = std::time::Instant::now();
    let timeout = Duration::from_secs(15);
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| (500, "could not read program status".into()))?
        {
            break status;
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            let _ = child.kill();
            break child
                .wait()
                .map_err(|_| (500, "could not stop the timed-out program".into()))?;
        }
        thread::sleep(Duration::from_millis(25));
    };
    let (stdout, stdout_truncated) = collect_editor_output(stdout_reader);
    let (stderr, stderr_truncated) = collect_editor_output(stderr_reader);
    let secrets = editor_secret_values(state);
    let stdout = redact_editor_secrets(String::from_utf8_lossy(&stdout).into_owned(), &secrets);
    let stderr = redact_editor_secrets(String::from_utf8_lossy(&stderr).into_owned(), &secrets);
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({
            "success": status.success() && !timed_out,
            "exitCode": status.code(),
            "timedOut": timed_out,
            "durationMs": started.elapsed().as_millis(),
            "stdout": stdout,
            "stderr": stderr,
            "truncated": stdout_truncated || stderr_truncated,
        })
        .to_string(),
    ))
}

fn read_editor_output<R: Read + Send + 'static>(
    mut reader: R,
) -> std::sync::mpsc::Receiver<(Vec<u8>, bool)> {
    let (sender, receiver) = std::sync::mpsc::channel();
    thread::spawn(move || {
        const OUTPUT_LIMIT: usize = 65_536;
        let mut captured = Vec::new();
        let mut buffer = [0_u8; 8192];
        let mut truncated = false;
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    let remaining = OUTPUT_LIMIT.saturating_sub(captured.len());
                    let retained = count.min(remaining);
                    captured.extend_from_slice(&buffer[..retained]);
                    truncated |= retained < count;
                }
            }
        }
        let _ = sender.send((captured, truncated));
    });
    receiver
}

fn collect_editor_output(receiver: std::sync::mpsc::Receiver<(Vec<u8>, bool)>) -> (Vec<u8>, bool) {
    receiver
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_else(|_| (Vec::new(), true))
}

fn editor_secret_values(state: &EditorState) -> Vec<String> {
    let is_secret_key = |key: &str| {
        let key = key.to_ascii_uppercase();
        [
            "PASSWORD",
            "SECRET",
            "TOKEN",
            "API_KEY",
            "DATABASE_URL",
            "PRIVATE_KEY",
            "CREDENTIAL",
        ]
        .iter()
        .any(|marker| key.contains(marker))
    };
    let mut secrets = env::vars_os()
        .filter_map(|(key, value)| {
            let key = key.to_string_lossy();
            is_secret_key(&key).then(|| value.to_string_lossy().into_owned())
        })
        .filter(|value| value.len() >= 4)
        .collect::<Vec<_>>();
    if let Ok(contents) = fs::read_to_string(state.root.join(".env")) {
        if contents.len() <= 256 * 1024 {
            secrets.extend(contents.lines().filter_map(|line| {
                let (key, value) = line.split_once('=')?;
                if !is_secret_key(key.trim()) {
                    return None;
                }
                let value = value.trim().trim_matches(['"', '\'']).to_owned();
                (value.len() >= 4).then_some(value)
            }));
        }
    }
    secrets.sort_by_key(|value| std::cmp::Reverse(value.len()));
    secrets.dedup();
    secrets
}

fn redact_editor_secrets(mut output: String, secrets: &[String]) -> String {
    for secret in secrets {
        output = output.replace(secret, "[redacted]");
    }
    output
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
        501 => "Not Implemented",
        502 => "Bad Gateway",
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

    #[test]
    fn editor_connection_limit_uses_atomic_compare_exchange() {
        let active = AtomicUsize::new(EDITOR_MAX_CONNECTIONS - 1);
        assert!(acquire_editor_connection(&active));
        assert_eq!(active.load(Ordering::Acquire), EDITOR_MAX_CONNECTIONS);
        assert!(!acquire_editor_connection(&active));
        assert_eq!(active.load(Ordering::Acquire), EDITOR_MAX_CONNECTIONS);
    }

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
        for (method, path) in [
            ("POST", "/__zelyra/editor/api/file"),
            ("POST", "/__zelyra/editor/api/file-rename"),
            ("DELETE", "/__zelyra/editor/api/file"),
        ] {
            let request = request(
                method,
                path,
                &[
                    ("x-zelyra-editor-token", "local-session-token"),
                    ("origin", "https://attacker.invalid"),
                ],
            );
            let error =
                editor_response(&request, &state).expect_err("cross-origin mutation must fail");
            assert_eq!(error.0, 403, "{method} {path} must be protected");
        }
        let _ = fs::remove_dir_all(state.root);
    }

    #[test]
    fn database_endpoints_require_token_before_inspecting_project() {
        let state = project();
        for path in [
            "/__zelyra/editor/api/database-schema",
            "/__zelyra/editor/api/crud-preview",
            "/__zelyra/editor/api/crud-apply",
        ] {
            let request = request("GET", path, &[]);
            let error = editor_response(&request, &state).expect_err("API token is required");
            assert_eq!(error.0, 403, "endpoint {path} must be protected");
        }
        let _ = fs::remove_dir_all(state.root);
    }

    #[test]
    fn mysql_schema_introspection_is_reported_as_unsupported_without_secret_details() {
        let state = project();
        fs::write(
            state.root.join("main.zyl"),
            "database main {\n    engine: mysql\n}\n",
        )
        .expect("write MySQL project fixture");
        let request = request(
            "GET",
            "/__zelyra/editor/api/database-schema",
            &[("x-zelyra-editor-token", "local-session-token")],
        );
        let error = editor_response(&request, &state).expect_err("MySQL introspection is bounded");
        assert_eq!(error.0, 501);
        assert!(error
            .1
            .contains("MySQL schema introspection is not supported"));
        assert!(!error.1.contains("password") && !error.1.contains("DATABASE_URL"));
        let _ = fs::remove_dir_all(state.root);
    }

    #[test]
    fn crud_preview_rejects_stale_source_before_database_access() {
        let state = project();
        let error = build_crud_preview(&state, &json!({"path":"main.zyl", "baseHash":"stale"}))
            .expect_err("stale source must block preview");
        assert_eq!(error.0, 409);
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
    fn editor_rename_is_project_scoped_hash_checked_and_never_overwrites() {
        let state = project();
        fs::create_dir(state.root.join("src")).expect("create source directory");
        fs::write(state.root.join("src/old.zyl"), "module old\n").expect("write source");
        let source = state.root.join("src/old.zyl");
        let hash = content_hash(&fs::read(&source).expect("read source"));

        let error = rename_editor_file(
            &state,
            json!({"path":"src/old.zyl", "newPath":"../outside.zyl", "baseHash":hash}),
        )
        .expect_err("path traversal must fail");
        assert_eq!(error.0, 400);
        assert!(source.exists());

        fs::write(state.root.join("src/existing.zyl"), "keep me").expect("write destination");
        let error = rename_editor_file(
            &state,
            json!({"path":"src/old.zyl", "newPath":"src/existing.zyl", "baseHash":hash}),
        )
        .expect_err("existing file must not be overwritten");
        assert_eq!(error.0, 409);
        assert_eq!(
            fs::read_to_string(state.root.join("src/existing.zyl")).unwrap(),
            "keep me"
        );

        let response = rename_editor_file(
            &state,
            json!({"path":"src/old.zyl", "newPath":"src/new.zyl", "baseHash":hash}),
        )
        .expect("rename source");
        assert_eq!(response.0, 200);
        assert!(!source.exists());
        assert_eq!(
            fs::read_to_string(state.root.join("src/new.zyl")).unwrap(),
            "module old\n"
        );

        let _ = fs::remove_dir_all(state.root);
    }

    #[test]
    fn editor_delete_requires_current_hash_and_protects_project_files() {
        let state = project();
        fs::create_dir(state.root.join("src")).expect("create source directory");
        let file = state.root.join("src/remove.zyl");
        fs::write(&file, "original").expect("write source");
        let stale =
            delete_editor_file(&state, json!({"path":"src/remove.zyl", "baseHash":"stale"}))
                .expect_err("stale delete must fail");
        assert_eq!(stale.0, 409);
        assert!(file.exists());
        let protected =
            delete_editor_file(&state, json!({"path":"main.zyl", "baseHash":"anything"}))
                .expect_err("entry file must be protected");
        assert_eq!(protected.0, 403);
        let hash = content_hash(&fs::read(&file).expect("read file"));
        assert_eq!(
            delete_editor_file(&state, json!({"path":"src/remove.zyl", "baseHash":hash}))
                .unwrap()
                .0,
            200
        );
        assert!(!file.exists());
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

    #[test]
    fn crud_builder_uses_valid_identifiers_and_escapes_display_text() {
        assert!(is_zelyra_identifier("InvoiceLine"));
        assert!(is_zelyra_identifier("_private"));
        assert!(!is_zelyra_identifier("1Invoice"));
        assert!(!is_zelyra_identifier("Invoice Line"));
        assert_eq!(resource_name_from_table("invoice_lines"), "InvoiceLines");

        let source = generated_crud_source(
            "InvoiceLines",
            "invoice_lines",
            "Line \"one\"",
            &["description".into(), "quantity".into()],
        );
        assert!(source.contains("crud InvoiceLines -> invoice_lines"));
        assert!(source.contains("title: \"Line \\\"one\\\"\""));
        assert!(source.contains("            description\n            quantity"));
    }

    #[test]
    fn generated_crud_source_passes_the_project_compiler() {
        let state = project();
        let path = state.root.join("main.zyl");
        let base = "database main {\n    engine: mariadb\n}\n\ntable customers {\n    id: Id primary auto\n    name: String(100) required\n}\n";
        fs::write(&path, base).expect("write compiler fixture");
        let snippet = generated_crud_source("Customer", "customers", "Customers", &["name".into()]);
        let candidate = format!("{base}\n{snippet}\n");
        let (success, diagnostics) = validate_editor_candidate(&state, "main.zyl", &candidate)
            .expect("private compiler preview should run");
        assert!(success, "generated source diagnostics: {diagnostics:?}");
        assert!(diagnostics.is_empty(), "diagnostics: {diagnostics:?}");
        assert_eq!(
            fs::read_to_string(path).expect("original remains intact"),
            base,
            "preview must not modify the project source"
        );
        let _ = fs::remove_dir_all(state.root);
    }

    #[test]
    fn editor_live_check_validates_unsaved_source_without_writing_it() {
        let state = project();
        let path = state.root.join("main.zyl");
        let original = fs::read_to_string(&path).expect("read original source");
        let response = check_project(
            &state,
            &json!({
                "path": "main.zyl",
                "source": "fn main( {\n"
            }),
        )
        .expect("unsaved source can be checked");
        let payload: Value = serde_json::from_str(&response.2).expect("decode check result");
        assert_eq!(payload["success"], false);
        assert!(!payload["diagnostics"].as_array().unwrap().is_empty());
        assert_eq!(
            fs::read_to_string(path).expect("source remains untouched"),
            original
        );
        let _ = fs::remove_dir_all(state.root);
    }

    #[test]
    fn editor_symbols_come_from_linked_project_declarations() {
        let state = project();
        fs::write(
            state.root.join("main.zyl"),
            "fn main() {}\nfn calculate_total() -> Int { return 1 }\n",
        )
        .expect("write project declarations");
        let response = editor_symbols(&state, "main.zyl").expect("load project symbols");
        let payload: Value = serde_json::from_str(&response.2).expect("decode symbols");
        let names = payload["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|item| item["label"].as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"main"));
        assert!(names.contains(&"calculate_total"));
        let _ = fs::remove_dir_all(state.root);
    }

    #[cfg(unix)]
    #[test]
    fn compiler_preview_directory_is_private_and_removed_after_use() {
        use std::os::unix::fs::PermissionsExt;

        let preview = create_editor_preview_directory().expect("create private preview");
        let path = preview.0.clone();
        assert_eq!(
            fs::metadata(&path)
                .expect("inspect preview permissions")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        drop(preview);
        assert!(!path.exists(), "preview directory is cleaned up");
    }
}
