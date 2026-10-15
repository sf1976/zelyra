//! Stdio JSON-RPC language server. All compiler diagnostics come from the
//! shared source-overlay service used by Studio.

use super::*;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use url::Url;

const MAX_MESSAGE_BYTES: usize = 2 * 1_048_576;

#[derive(Clone)]
struct Document {
    uri: String,
    relative: String,
    text: String,
    version: i64,
}

#[derive(Clone)]
struct Server {
    root: PathBuf,
    documents: HashMap<String, Document>,
    shutdown: bool,
}

struct PendingRequest {
    cancelled: Arc<AtomicBool>,
    document: Option<(String, i64)>,
}

pub fn command() -> ExitCode {
    let stdin = io::stdin();
    let stdout = io::stdout();
    match serve(BufReader::new(stdin), stdout.lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error[E-LSP-001]: {error}");
            ExitCode::from(1)
        }
    }
}

fn serve<R: BufRead + Send + 'static, W: Write>(mut input: R, mut output: W) -> io::Result<()> {
    let mut server = Server {
        root: fs::canonicalize(".")?,
        documents: HashMap::new(),
        shutdown: false,
    };
    let (incoming_tx, incoming_rx) = mpsc::sync_channel(32);
    std::thread::spawn(move || loop {
        match read_message(&mut input) {
            Ok(Some(message)) => {
                if incoming_tx.send(Ok(message)).is_err() {
                    break;
                }
            }
            Ok(None) => break,
            Err(error) => {
                let _ = incoming_tx.send(Err(error));
                break;
            }
        }
    });
    let (completed_tx, completed_rx) = mpsc::channel::<(String, Value)>();
    let mut pending = HashMap::<String, PendingRequest>::new();
    let mut workers: Vec<std::thread::JoinHandle<()>> = Vec::new();
    let mut input_closed = false;
    while !input_closed || !pending.is_empty() {
        while let Ok((key, response)) = completed_rx.try_recv() {
            if let Some(request) = pending.remove(&key) {
                if !request.cancelled.load(Ordering::Acquire) {
                    let stale = request.document.as_ref().is_some_and(|(uri, version)| {
                        server
                            .documents
                            .get(uri)
                            .is_none_or(|document| document.version != *version)
                    });
                    let response = if stale {
                        json!({
                            "jsonrpc":"2.0",
                            "id":response["id"],
                            "error":{"code":-32801,"message":"document changed while the request was running"}
                        })
                    } else {
                        response
                    };
                    write_message(&mut output, &response)?;
                }
            }
        }
        let mut running = Vec::new();
        for worker in workers.drain(..) {
            if worker.is_finished() {
                let _ = worker.join();
            } else {
                running.push(worker);
            }
        }
        workers = running;
        if input_closed && pending.is_empty() {
            break;
        }
        let message = match incoming_rx.recv_timeout(Duration::from_millis(10)) {
            Ok(Ok(message)) => message,
            Ok(Err(error)) => return Err(error),
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                input_closed = true;
                continue;
            }
        };
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            continue;
        };
        let id = message.get("id").cloned();
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        if matches!(
            method,
            "textDocument/documentSymbol"
                | "workspace/symbol"
                | "textDocument/completion"
                | "textDocument/hover"
                | "textDocument/definition"
                | "textDocument/references"
                | "textDocument/prepareRename"
                | "textDocument/rename"
        ) {
            if let Some(id) = id {
                if workers.len() >= 4 {
                    write_message(
                        &mut output,
                        &json!({"jsonrpc":"2.0","id":id,"error":{"code":-32002,"message":"too many language requests are already running"}}),
                    )?;
                    continue;
                }
                let key = id.to_string();
                let document = params
                    .pointer("/textDocument/uri")
                    .and_then(Value::as_str)
                    .and_then(|uri| {
                        server
                            .documents
                            .get(uri)
                            .map(|doc| (uri.to_owned(), doc.version))
                    });
                let cancelled = Arc::new(AtomicBool::new(false));
                pending.insert(
                    key.clone(),
                    PendingRequest {
                        cancelled: cancelled.clone(),
                        document,
                    },
                );
                let snapshot = server.clone();
                let method = method.to_owned();
                let sender = completed_tx.clone();
                workers.push(std::thread::spawn(move || {
                    let response = handle_request(&snapshot, &method, &params, &id);
                    if !cancelled.load(Ordering::Acquire) {
                        let _ = sender.send((key, response));
                    }
                }));
            }
            continue;
        }
        match method {
            "initialize" => {
                if let Some(root) = params
                    .get("rootUri")
                    .and_then(Value::as_str)
                    .and_then(uri_path)
                    .and_then(|path| fs::canonicalize(path).ok())
                {
                    server.root = root;
                }
                if let Some(id) = id {
                    write_message(
                        &mut output,
                        &json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "capabilities": {
                                    "positionEncoding": "utf-16",
                                    "textDocumentSync": {"openClose": true, "change": 2},
                                    "documentSymbolProvider": true,
                                    "workspaceSymbolProvider": true,
                                "completionProvider": {"triggerCharacters": [".", ":"]},
                                "hoverProvider": true,
                                "definitionProvider": true,
                                "referencesProvider": true,
                                "renameProvider": {"prepareProvider": true}
                                },
                                "serverInfo": {"name": "zelyra", "version": env!("CARGO_PKG_VERSION")}
                            }
                        }),
                    )?;
                }
            }
            "initialized" => {}
            "shutdown" => {
                server.shutdown = true;
                if let Some(id) = id {
                    write_message(&mut output, &json!({"jsonrpc":"2.0","id":id,"result":null}))?;
                }
            }
            "exit" => {
                if !server.shutdown {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "LSP exit before shutdown",
                    ));
                }
                for request in pending.values() {
                    request.cancelled.store(true, Ordering::Release);
                }
                pending.clear();
                input_closed = true;
            }
            "textDocument/didOpen" => {
                if let Some(document) = params.get("textDocument") {
                    if let (Some(uri), Some(text), Some(version)) = (
                        document.get("uri").and_then(Value::as_str),
                        document.get("text").and_then(Value::as_str),
                        document.get("version").and_then(Value::as_i64),
                    ) {
                        if let Some(relative) = server.safe_relative(uri) {
                            let other_bytes = server
                                .documents
                                .iter()
                                .filter(|(key, _)| key.as_str() != uri)
                                .map(|(_, document)| document.text.len())
                                .sum::<usize>();
                            if text.len() <= 1_048_576
                                && other_bytes.saturating_add(text.len()) <= 16 * 1_048_576
                                && (server.documents.contains_key(uri)
                                    || server.documents.len() < 128)
                            {
                                server.documents.insert(
                                    uri.into(),
                                    Document {
                                        uri: uri.into(),
                                        relative,
                                        text: text.into(),
                                        version,
                                    },
                                );
                                server.publish_diagnostics(&mut output)?;
                            }
                        }
                    }
                }
            }
            "textDocument/didChange" => {
                let uri = params.pointer("/textDocument/uri").and_then(Value::as_str);
                let version = params
                    .pointer("/textDocument/version")
                    .and_then(Value::as_i64);
                if let (Some(uri), Some(version), Some(changes)) = (
                    uri,
                    version,
                    params.get("contentChanges").and_then(Value::as_array),
                ) {
                    let total_bytes = server
                        .documents
                        .values()
                        .map(|item| item.text.len())
                        .sum::<usize>();
                    if let Some(document) = server.documents.get_mut(uri) {
                        if version > document.version {
                            let mut text = document.text.clone();
                            let mut valid = true;
                            for change in changes {
                                let Some(replacement) = change.get("text").and_then(Value::as_str)
                                else {
                                    valid = false;
                                    break;
                                };
                                if let Some(range) = change.get("range") {
                                    let Some(start) = range
                                        .get("start")
                                        .and_then(|value| position_offset(&text, value))
                                    else {
                                        valid = false;
                                        break;
                                    };
                                    let Some(end) = range
                                        .get("end")
                                        .and_then(|value| position_offset(&text, value))
                                    else {
                                        valid = false;
                                        break;
                                    };
                                    if start > end {
                                        valid = false;
                                        break;
                                    }
                                    text.replace_range(start..end, replacement);
                                } else {
                                    text = replacement.into();
                                }
                            }
                            let other_bytes = total_bytes.saturating_sub(document.text.len());
                            if valid
                                && text.len() <= 1_048_576
                                && other_bytes.saturating_add(text.len()) <= 16 * 1_048_576
                            {
                                document.text = text;
                                document.version = version;
                                server.publish_diagnostics(&mut output)?;
                            }
                        }
                    }
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) = params.pointer("/textDocument/uri").and_then(Value::as_str) {
                    if server.documents.remove(uri).is_some() {
                        write_message(
                            &mut output,
                            &json!({
                                "jsonrpc":"2.0","method":"textDocument/publishDiagnostics",
                                "params":{"uri":uri,"diagnostics":[]}
                            }),
                        )?;
                        server.publish_diagnostics(&mut output)?;
                    }
                }
            }
            "textDocument/documentSymbol"
            | "workspace/symbol"
            | "textDocument/completion"
            | "textDocument/hover"
            | "textDocument/definition"
            | "textDocument/references"
            | "textDocument/prepareRename"
            | "textDocument/rename" => {}
            "$/cancelRequest" => {
                if let Some(cancelled_id) = params.get("id") {
                    let key = cancelled_id.to_string();
                    if let Some(request) = pending.remove(&key) {
                        request.cancelled.store(true, Ordering::Release);
                        write_message(
                            &mut output,
                            &json!({
                                "jsonrpc":"2.0","id":cancelled_id,
                                "error":{"code":-32800,"message":"request cancelled"}
                            }),
                        )?;
                    }
                }
            }
            _ => {
                if let Some(id) = id {
                    write_message(
                        &mut output,
                        &json!({
                            "jsonrpc":"2.0","id":id,
                            "error":{"code":-32601,"message":format!("method `{method}` is not implemented")}
                        }),
                    )?;
                }
            }
        }
    }
    for worker in workers {
        let _ = worker.join();
    }
    Ok(())
}

fn handle_request(server: &Server, method: &str, params: &Value, id: &Value) -> Value {
    #[cfg(test)]
    if let Some(delay_ms) = params.get("__testDelayMs").and_then(Value::as_u64) {
        std::thread::sleep(Duration::from_millis(delay_ms.min(250)));
    }
    if matches!(
        method,
        "textDocument/documentSymbol" | "workspace/symbol" | "textDocument/completion"
    ) {
        return json!({"jsonrpc":"2.0","id":id,"result":server.symbol_response(method, params)});
    }
    match server.semantic_response(method, params) {
        Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
        Err(message) => json!({
            "jsonrpc":"2.0","id":id,"error":{"code":-32602,"message":message}
        }),
    }
}

impl Server {
    fn entry(&self) -> PathBuf {
        let entry = self.root.join("main.zyl");
        if entry.is_file() {
            entry
        } else {
            self.documents
                .values()
                .min_by(|left, right| left.relative.cmp(&right.relative))
                .map_or(entry, |document| self.root.join(&document.relative))
        }
    }

    fn safe_relative(&self, uri: &str) -> Option<String> {
        let requested = uri_path(uri)?;
        if requested.extension().and_then(|value| value.to_str()) != Some("zyl")
            || requested
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return None;
        }
        let path = match fs::symlink_metadata(&requested) {
            Ok(_) => {
                let canonical = fs::canonicalize(&requested).ok()?;
                if !canonical.is_file() {
                    return None;
                }
                canonical
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                canonicalize_missing_path(&requested)?
            }
            Err(_) => return None,
        };
        let relative = path.strip_prefix(&self.root).ok()?;
        Some(relative.to_string_lossy().replace('\\', "/"))
    }

    fn analysis(&self) -> language_service::Analysis {
        let entry = self.entry();
        let overlays = self
            .documents
            .values()
            .map(|document| language_service::SourceOverlay {
                path: document.relative.clone(),
                text: document.text.clone(),
                version: document.version,
            })
            .collect::<Vec<_>>();
        language_service::analyze(&entry, &overlays)
    }

    fn publish_diagnostics<W: Write>(&self, output: &mut W) -> io::Result<()> {
        let analysis = self.analysis();
        let mut documents = self.documents.values().collect::<Vec<_>>();
        documents.sort_by(|left, right| left.relative.cmp(&right.relative));
        for document in documents {
            let diagnostics = analysis.diagnostics.iter()
                .filter(|item| item.get("file").and_then(Value::as_str) == Some(document.relative.as_str()))
                .map(|item| {
                    let source = analysis.sources.iter().find(|source| source.path == document.relative)
                        .map_or(document.text.as_str(), |source| source.text.as_str());
                    let start = item.pointer("/span/start/offset").and_then(Value::as_u64).unwrap_or(0) as usize;
                    let end = item.pointer("/span/end/offset").and_then(Value::as_u64).unwrap_or(start as u64) as usize;
                    json!({
                        "range": {"start": offset_position(source, start), "end": offset_position(source, end)},
                        "severity": 1,
                        "code": item["code"],
                        "source": "zelyra",
                        "message": item["message"],
                    })
                }).collect::<Vec<_>>();
            write_message(
                output,
                &json!({
                    "jsonrpc":"2.0","method":"textDocument/publishDiagnostics",
                    "params":{"uri":document.uri,"version":analysis.versions.get(&document.relative),"diagnostics":diagnostics}
                }),
            )?;
        }
        Ok(())
    }

    fn symbol_response(&self, method: &str, params: &Value) -> Value {
        let analysis = self.analysis();
        let Some(program) = analysis.program.as_ref() else {
            return json!([]);
        };
        if method == "textDocument/completion" {
            let uri = params
                .pointer("/textDocument/uri")
                .and_then(Value::as_str)
                .unwrap_or("");
            if let Some(relative) = self.safe_relative(uri) {
                if let (Some(source), Some(position)) = (
                    analysis
                        .sources
                        .iter()
                        .find(|source| source.path == relative),
                    params.get("position"),
                ) {
                    if let Some(offset) = position_offset(&source.text, position) {
                        if let Some(hole) =
                            language_service::typed_hole_entry(&analysis, &relative, offset)
                        {
                            let start = utf16_absolute_offset(
                                &source.text,
                                hole["editorSpan"]["start"].as_u64().unwrap_or(0) as usize,
                            );
                            let end = utf16_absolute_offset(
                                &source.text,
                                hole["editorSpan"]["end"].as_u64().unwrap_or(0) as usize,
                            );
                            let range = json!({"start":offset_position(&source.text,start),"end":offset_position(&source.text,end)});
                            return json!(hole["suggestions"].as_array().map(|items| items.iter().map(|item| {
                                let expression = item["expression"].as_str().unwrap_or("");
                                let kind = match item["kind"].as_str() {
                                    Some("function") => 3,
                                    Some("value") => 6,
                                    _ => 1,
                                };
                                json!({
                                    "label":expression,"kind":kind,"detail":item["type"],
                                    "documentation":item["contractReview"].as_bool().unwrap_or(false).then_some("Contracts need review before use."),
                                    "textEdit":{"range":range,"newText":expression}
                                })
                            }).collect::<Vec<_>>()).unwrap_or_default());
                        }
                    }
                }
            }
        }
        let mut symbols = Vec::new();
        for function in &program.functions {
            push_outline_symbol(
                &mut symbols,
                &self.root,
                &analysis.sources,
                &function.name,
                function.span,
                12,
            );
        }
        for record in &program.records {
            push_outline_symbol(
                &mut symbols,
                &self.root,
                &analysis.sources,
                &record.name,
                record.span,
                23,
            );
        }
        for table in &program.tables {
            push_outline_symbol(
                &mut symbols,
                &self.root,
                &analysis.sources,
                &table.name,
                table.span,
                23,
            );
        }
        for type_def in &program.types {
            push_outline_symbol(
                &mut symbols,
                &self.root,
                &analysis.sources,
                &type_def.name,
                type_def.span,
                26,
            );
        }
        for enum_def in &program.enums {
            push_outline_symbol(
                &mut symbols,
                &self.root,
                &analysis.sources,
                &enum_def.name,
                enum_def.span,
                10,
            );
        }
        for view in &program.views {
            push_outline_symbol(
                &mut symbols,
                &self.root,
                &analysis.sources,
                &view.name,
                view.span,
                2,
            );
        }
        for component in &program.components {
            push_outline_symbol(
                &mut symbols,
                &self.root,
                &analysis.sources,
                &component.name,
                component.span,
                5,
            );
        }
        for page in &program.pages {
            push_outline_symbol(
                &mut symbols,
                &self.root,
                &analysis.sources,
                &page.path,
                page.span,
                2,
            );
        }
        for tableview in &program.tableviews {
            push_outline_symbol(
                &mut symbols,
                &self.root,
                &analysis.sources,
                &tableview.name,
                tableview.span,
                23,
            );
        }
        for form in &program.forms {
            push_outline_symbol(
                &mut symbols,
                &self.root,
                &analysis.sources,
                &form.name,
                form.span,
                23,
            );
        }
        for crud in &program.cruds {
            push_outline_symbol(
                &mut symbols,
                &self.root,
                &analysis.sources,
                &crud.name,
                crud.span,
                23,
            );
        }
        for auth in &program.auth {
            push_outline_symbol(
                &mut symbols,
                &self.root,
                &analysis.sources,
                &auth.name,
                auth.span,
                23,
            );
        }
        for api in &program.apis {
            let name = format!("{} {}", api.method, api.path);
            push_outline_symbol(
                &mut symbols,
                &self.root,
                &analysis.sources,
                &name,
                api.span,
                12,
            );
        }
        match method {
            "textDocument/completion" => {
                let uri = params
                    .pointer("/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let Some(relative) = self.safe_relative(uri) else {
                    return json!([]);
                };
                let imported = analysis
                    .modules
                    .iter()
                    .find(|module| module.path == relative)
                    .map(|module| {
                        module
                            .imports
                            .iter()
                            .map(|import| import.path.as_str())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                json!(
                    symbols
                        .iter()
                        .filter(|(path, _, _, _)| path == &relative
                            || imported.contains(&path.as_str()))
                        .map(|(_, name, kind, _)| json!({"label":name,"kind":kind}))
                        .collect::<Vec<_>>()
                )
            }
            "textDocument/documentSymbol" => {
                let uri = params
                    .pointer("/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let Some(relative) = self.safe_relative(uri) else {
                    return json!([]);
                };
                json!(symbols.iter().filter(|(path, _, _, _)| path == &relative)
                    .map(|(_, name, kind, location)| json!({
                        "name":name,"kind":kind,"range":location["range"],"selectionRange":location["selectionRange"]
                    })).collect::<Vec<_>>())
            }
            _ => {
                let query = params
                    .get("query")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_lowercase();
                json!(symbols
                    .iter()
                    .filter(|(_, name, _, _)| name.to_lowercase().contains(&query))
                    .map(|(_, name, kind, location)| json!({
                        "name":name,"kind":kind,
                        "location":{"uri":location["uri"],"range":location["range"]}
                    }))
                    .collect::<Vec<_>>())
            }
        }
    }

    fn semantic_response(&self, method: &str, params: &Value) -> Result<Value, String> {
        let uri = params
            .pointer("/textDocument/uri")
            .and_then(Value::as_str)
            .ok_or("textDocument.uri is required")?;
        let relative = self
            .safe_relative(uri)
            .ok_or("source is outside the Zelyra workspace")?;
        let analysis = self.analysis();
        let Some(source) = analysis
            .sources
            .iter()
            .find(|source| source.path == relative)
        else {
            return Ok(Value::Null);
        };
        let Some(position) = params.get("position") else {
            return Err("position is required".into());
        };
        let Some(offset) = position_offset(&source.text, position) else {
            return Err("invalid UTF-16 position".into());
        };
        let index = semantic_index::SemanticIndex::from_analysis(&analysis);
        let Some(occurrence) = index.at(&relative, offset) else {
            return Ok(Value::Null);
        };
        let Some(symbol) = index.symbol(&occurrence.key) else {
            return Ok(Value::Null);
        };
        let declaration = occurrence_location(&self.root, &symbol.declaration, &analysis.sources);
        match method {
            "textDocument/hover" => Ok(json!({
                "contents":{"kind":"plaintext","value":symbol.detail},
                "range": occurrence_range(occurrence, &source.text),
            })),
            "textDocument/definition" => Ok(declaration),
            "textDocument/references" => {
                let include_declaration = params
                    .pointer("/context/includeDeclaration")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                Ok(json!(index
                    .references(&symbol.key, include_declaration)
                    .iter()
                    .map(|reference| occurrence_location(&self.root, reference, &analysis.sources))
                    .collect::<Vec<_>>()))
            }
            "textDocument/prepareRename" => Ok(json!({
                "range": occurrence_range(occurrence, &source.text),
                "placeholder": symbol.name,
            })),
            "textDocument/rename" => self.rename_function(params, &analysis, &index, symbol),
            _ => Ok(Value::Null),
        }
    }

    fn rename_function(
        &self,
        params: &Value,
        analysis: &language_service::Analysis,
        index: &semantic_index::SemanticIndex,
        symbol: &semantic_index::Symbol,
    ) -> Result<Value, String> {
        let replacement = params
            .get("newName")
            .and_then(Value::as_str)
            .ok_or("newName is required")?;
        let overlays = self
            .documents
            .values()
            .map(|document| language_service::SourceOverlay {
                path: document.relative.clone(),
                text: document.text.clone(),
                version: document.version,
            })
            .collect::<Vec<_>>();
        let changes = semantic_index::plan_symbol_rename(
            &self.entry(),
            analysis,
            index,
            symbol,
            replacement,
            &overlays,
        )?;
        Ok(json!({"documentChanges":changes.iter().map(|change| {
            let uri = Url::from_file_path(self.root.join(&change.path)).map_or_else(|_| String::new(), |url| url.to_string());
            json!({
                "textDocument":{"uri":uri,"version":self.documents.values().find(|document| document.relative == change.path).map(|document| document.version)},
                "edits":change.edits.iter().map(|edit| json!({
                    "range":{"start":offset_position(&change.before, edit.start),
                             "end":offset_position(&change.before, edit.end)},
                    "newText":edit.replacement,
                })).collect::<Vec<_>>()
            })
        }).collect::<Vec<_>>()}))
    }
}

/// Resolve the existing portion of a new document path before accepting it.
/// This keeps unsaved documents workspace-scoped even through symlinked parents.
fn canonicalize_missing_path(path: &Path) -> Option<PathBuf> {
    let mut current = path;
    let mut missing = Vec::new();
    loop {
        match fs::canonicalize(current) {
            Ok(mut canonical) => {
                for component in missing.iter().rev() {
                    canonical.push(component);
                }
                return Some(canonical);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                missing.push(current.file_name()?.to_os_string());
                current = current.parent()?;
            }
            Err(_) => return None,
        }
    }
}

fn occurrence_range(occurrence: &semantic_index::Occurrence, source: &str) -> Value {
    json!({"start":offset_position(source, occurrence.start),"end":offset_position(source, occurrence.end)})
}

fn occurrence_location(
    root: &Path,
    occurrence: &semantic_index::Occurrence,
    sources: &[project::ProjectSource],
) -> Value {
    let source = sources.iter().find(|source| source.path == occurrence.path);
    let uri = Url::from_file_path(root.join(&occurrence.path))
        .map_or_else(|_| String::new(), |url| url.to_string());
    json!({
        "uri":uri,
        "range":source.map_or(Value::Null, |source| occurrence_range(occurrence, &source.text))
    })
}

fn declaration_location(
    root: &Path,
    source: &project::ProjectSource,
    span: zelyra_ast::Span,
    name: &str,
) -> Value {
    let start = source
        .text
        .get(span.start..span.end)
        .and_then(|body| body.find(name))
        .map_or(span.start, |offset| span.start + offset);
    let uri = Url::from_file_path(root.join(&source.path))
        .map_or_else(|_| String::new(), |url| url.to_string());
    json!({
        "uri": uri,
        "range": {"start":offset_position(&source.text,start),"end":offset_position(&source.text,start+name.len())}
    })
}

fn push_outline_symbol(
    symbols: &mut Vec<(String, String, u32, Value)>,
    root: &Path,
    sources: &[project::ProjectSource],
    qualified_name: &str,
    span: zelyra_ast::Span,
    kind: u32,
) {
    let Some(source) = sources.get(span.source_id as usize) else {
        return;
    };
    let name = qualified_name.rsplit("::").next().unwrap_or(qualified_name);
    let name_location = declaration_location(root, source, span, name);
    let uri = name_location["uri"].clone();
    let location = json!({
        "uri":uri,
        "range":{"start":offset_position(&source.text, span.start),"end":offset_position(&source.text, span.end)},
        "selectionRange":name_location["range"]
    });
    symbols.push((source.path.clone(), name.to_owned(), kind, location));
}

fn uri_path(uri: &str) -> Option<PathBuf> {
    Url::parse(uri).ok()?.to_file_path().ok()
}

fn offset_position(source: &str, requested: usize) -> Value {
    let mut offset = requested.min(source.len());
    while !source.is_char_boundary(offset) {
        offset -= 1;
    }
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let line_start = prefix.rfind('\n').map_or(0, |position| position + 1);
    json!({"line":line,"character":prefix[line_start..].encode_utf16().count()})
}

fn position_offset(source: &str, position: &Value) -> Option<usize> {
    let line = usize::try_from(position.get("line")?.as_u64()?).ok()?;
    let character = usize::try_from(position.get("character")?.as_u64()?).ok()?;
    let mut start = 0;
    for _ in 0..line {
        start += source.get(start..)?.find('\n')? + 1;
    }
    let end = start + source[start..].find('\n').unwrap_or(source.len() - start);
    let mut units = 0;
    for (relative, symbol) in source[start..end].char_indices() {
        if units == character {
            return Some(start + relative);
        }
        units += symbol.len_utf16();
        if units > character {
            return None;
        }
    }
    (units == character).then_some(end)
}

fn utf16_absolute_offset(source: &str, requested: usize) -> usize {
    let mut units = 0;
    for (byte_offset, symbol) in source.char_indices() {
        if units >= requested {
            return byte_offset;
        }
        units += symbol.len_utf16();
        if units > requested {
            return byte_offset;
        }
    }
    source.len()
}

fn read_message<R: BufRead>(input: &mut R) -> io::Result<Option<Value>> {
    let mut length = None;
    let mut header_bytes = 0;
    loop {
        let mut line = String::new();
        let read = input.read_line(&mut line)?;
        if read == 0 {
            return if header_bytes == 0 {
                Ok(None)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "truncated LSP header",
                ))
            };
        }
        header_bytes += read;
        if header_bytes > 8192 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "LSP header exceeds 8 KiB",
            ));
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            if key.eq_ignore_ascii_case("Content-Length") {
                length = value.trim().parse::<usize>().ok();
            }
        }
    }
    let length = length
        .filter(|length| *length <= MAX_MESSAGE_BYTES)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid LSP Content-Length"))?;
    let mut body = vec![0; length];
    input.read_exact(&mut body)?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn write_message<W: Write>(output: &mut W, value: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(value)?;
    write!(output, "Content-Length: {}\r\n\r\n", body.len())?;
    output.write_all(&body)?;
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framing_reads_unicode_and_multiple_messages() {
        let first = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"name":"Ä😀"}});
        let second = json!({"jsonrpc":"2.0","method":"exit"});
        let mut bytes = Vec::new();
        write_message(&mut bytes, &first).unwrap();
        write_message(&mut bytes, &second).unwrap();
        let mut reader = BufReader::new(bytes.as_slice());
        assert_eq!(read_message(&mut reader).unwrap(), Some(first));
        assert_eq!(read_message(&mut reader).unwrap(), Some(second));
        assert_eq!(read_message(&mut reader).unwrap(), None);
    }

    #[test]
    fn utf16_positions_respect_crlf_and_surrogate_pairs() {
        let source = "A\r\n😀B\n";
        assert_eq!(
            position_offset(source, &json!({"line":1,"character":2})),
            Some(7)
        );
        assert_eq!(offset_position(source, 7), json!({"line":1,"character":2}));
        assert_eq!(
            position_offset(source, &json!({"line":1,"character":1})),
            None
        );
    }

    #[test]
    fn lsp_accepts_new_workspace_paths_but_rejects_escape_and_symlinks() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-lsp-path-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let outside = root.with_extension("outside");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let server = Server {
            root: fs::canonicalize(&root).unwrap(),
            documents: HashMap::new(),
            shutdown: false,
        };
        let new_uri = Url::from_file_path(root.join("src/new.zyl"))
            .unwrap()
            .to_string();
        assert_eq!(
            server.safe_relative(&new_uri).as_deref(),
            Some("src/new.zyl")
        );
        let traversal = Url::from_file_path(root.join("src/../../escaped.zyl"))
            .unwrap()
            .to_string();
        assert_eq!(server.safe_relative(&traversal), None);
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, root.join("linked")).unwrap();
            let escaped = Url::from_file_path(root.join("linked/new.zyl"))
                .unwrap()
                .to_string();
            assert_eq!(server.safe_relative(&escaped), None);
        }
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn unsaved_document_diagnostics_are_versioned_and_stdout_is_only_json_rpc() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-lsp-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("main.zyl"), "fn main() {}\n").unwrap();
        let uri = Url::from_file_path(root.join("main.zyl"))
            .unwrap()
            .to_string();
        let root_uri = Url::from_file_path(&root).unwrap().to_string();
        let mut input = Vec::new();
        for request in [
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":root_uri}}),
            json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
            json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":"fn main() {}\n"}}}),
            json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":2},"contentChanges":[{"text":"fn main( {\n"}]}}),
            json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
            json!({"jsonrpc":"2.0","method":"exit"}),
        ] {
            write_message(&mut input, &request).unwrap();
        }
        let mut output = Vec::new();
        serve(BufReader::new(io::Cursor::new(input)), &mut output).unwrap();
        let mut reader = BufReader::new(output.as_slice());
        let mut messages = Vec::new();
        while let Some(message) = read_message(&mut reader).unwrap() {
            messages.push(message);
        }
        assert_eq!(
            messages[0]["result"]["capabilities"]["positionEncoding"],
            "utf-16"
        );
        assert!(
            messages.iter().any(|message| {
                message["method"] == "textDocument/publishDiagnostics"
                    && message["params"]["version"] == 2
                    && message["params"]["diagnostics"][0]["code"] == "E-PARSE-001"
            }),
            "messages: {messages:?}"
        );
        assert_eq!(
            fs::read_to_string(root.join("main.zyl")).unwrap(),
            "fn main() {}\n"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn new_unsaved_lsp_document_gets_compiler_diagnostics_without_disk_write() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-lsp-new-document-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("main.zyl"), "fn main() {}\n").unwrap();
        let uri = Url::from_file_path(root.join("src/new.zyl"))
            .unwrap()
            .to_string();
        let root_uri = Url::from_file_path(&root).unwrap().to_string();
        let mut input = Vec::new();
        for request in [
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":root_uri}}),
            json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":3,"text":"pub fn invalid() -> Int { return \"wrong\" }\n"}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
            json!({"jsonrpc":"2.0","method":"exit"}),
        ] {
            write_message(&mut input, &request).unwrap();
        }
        let mut output = Vec::new();
        serve(BufReader::new(io::Cursor::new(input)), &mut output).unwrap();
        let mut reader = BufReader::new(output.as_slice());
        let mut messages = Vec::new();
        while let Some(message) = read_message(&mut reader).unwrap() {
            messages.push(message);
        }
        assert!(
            messages.iter().any(|message| {
                message["method"] == "textDocument/publishDiagnostics"
                    && message["params"]["uri"] == uri
                    && message["params"]["version"] == 3
                    && message["params"]["diagnostics"][0]["code"] == "E-TYPE-001"
            }),
            "messages: {messages:?}"
        );
        assert!(!root.join("src/new.zyl").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lsp_cancellation_replies_without_publishing_the_cancelled_result() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-lsp-cancel-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("main.zyl"), "fn main() {}\n").unwrap();
        let uri = Url::from_file_path(root.join("main.zyl"))
            .unwrap()
            .to_string();
        let root_uri = Url::from_file_path(&root).unwrap().to_string();
        let mut input = Vec::new();
        for request in [
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":root_uri}}),
            json!({"jsonrpc":"2.0","id":"slow","method":"textDocument/documentSymbol","params":{"textDocument":{"uri":uri},"__testDelayMs":100}}),
            json!({"jsonrpc":"2.0","method":"$/cancelRequest","params":{"id":"slow"}}),
            json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
            json!({"jsonrpc":"2.0","method":"exit"}),
        ] {
            write_message(&mut input, &request).unwrap();
        }
        let mut output = Vec::new();
        serve(BufReader::new(io::Cursor::new(input)), &mut output).unwrap();
        let mut reader = BufReader::new(output.as_slice());
        let mut messages = Vec::new();
        while let Some(message) = read_message(&mut reader).unwrap() {
            messages.push(message);
        }
        assert!(messages
            .iter()
            .any(|message| { message["id"] == "slow" && message["error"]["code"] == -32800 }));
        assert!(!messages
            .iter()
            .any(|message| message["id"] == "slow" && message.get("result").is_some()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lsp_discards_results_for_documents_changed_during_a_request() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-lsp-stale-result-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("main.zyl"), "fn main() {}\n").unwrap();
        let uri = Url::from_file_path(root.join("main.zyl"))
            .unwrap()
            .to_string();
        let root_uri = Url::from_file_path(&root).unwrap().to_string();
        let mut input = Vec::new();
        for request in [
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":root_uri}}),
            json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":"fn main() {}\n"}}}),
            json!({"jsonrpc":"2.0","id":"slow","method":"textDocument/documentSymbol","params":{"textDocument":{"uri":uri},"__testDelayMs":80}}),
            json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":2},"contentChanges":[{"text":"fn main() { print(\"new\") }\n"}]}}),
        ] {
            write_message(&mut input, &request).unwrap();
        }
        let mut output = Vec::new();
        serve(BufReader::new(io::Cursor::new(input)), &mut output).unwrap();
        let mut reader = BufReader::new(output.as_slice());
        let mut messages = Vec::new();
        while let Some(message) = read_message(&mut reader).unwrap() {
            messages.push(message);
        }
        assert!(
            messages
                .iter()
                .any(|message| { message["id"] == "slow" && message["error"]["code"] == -32801 }),
            "messages: {messages:?}"
        );
        assert!(!messages
            .iter()
            .any(|message| message["id"] == "slow" && message.get("result").is_some()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn function_navigation_and_rename_follow_resolved_imports_only() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-lsp-symbol-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        let main = "import \"src/a.zyl\" as a\nimport \"src/b.zyl\" as b\nfn main() { print(a::answer()) print(b::answer()) print(\"answer\") }\n";
        fs::write(root.join("main.zyl"), main).unwrap();
        fs::write(
            root.join("src/a.zyl"),
            "pub fn answer() -> Int { return 1 }\n",
        )
        .unwrap();
        fs::write(
            root.join("src/b.zyl"),
            "pub fn answer() -> Int { return 2 }\n",
        )
        .unwrap();
        let uri = Url::from_file_path(root.join("main.zyl"))
            .unwrap()
            .to_string();
        let server = Server {
            root: root.clone(),
            documents: HashMap::new(),
            shutdown: false,
        };
        let call = main.find("a::answer()").unwrap() + 3;
        let params = json!({"textDocument":{"uri":uri},"position":offset_position(main,call)});
        let definition = server
            .semantic_response("textDocument/definition", &params)
            .unwrap();
        assert!(definition["uri"].as_str().unwrap().ends_with("/src/a.zyl"));
        let hover = server
            .semantic_response("textDocument/hover", &params)
            .unwrap();
        assert_eq!(hover["contents"]["value"], "fn answer() -> Int");
        let references = server
            .semantic_response("textDocument/references", &params)
            .unwrap();
        assert_eq!(references.as_array().unwrap().len(), 1);
        let rename = server.semantic_response("textDocument/rename", &json!({
            "textDocument":{"uri":uri},"position":offset_position(main,call),"newName":"result"
        })).unwrap();
        let changes = rename["documentChanges"].as_array().unwrap();
        assert_eq!(changes.len(), 2);
        assert_eq!(
            changes
                .iter()
                .map(|change| change["edits"].as_array().unwrap().len())
                .sum::<usize>(),
            2
        );
        assert!(changes.iter().all(|change| !change["textDocument"]["uri"]
            .as_str()
            .unwrap()
            .ends_with("/src/b.zyl")));
        assert!(server
            .semantic_response(
                "textDocument/rename",
                &json!({
                    "textDocument":{"uri":uri},"position":offset_position(main,call),"newName":"b"
                })
            )
            .is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn record_navigation_and_rename_follow_imported_types_and_constructors() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-lsp-record-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        let main = "import \"src/models.zyl\" as model\ntype CustomerAlias = model::Customer\nfn retain(customer: model::Customer) -> model::Customer { return customer }\nfn main() { customer: model::Customer = model::Customer { name: \"Ada\" } print(customer.name) }\n";
        fs::write(root.join("main.zyl"), main).unwrap();
        fs::write(
            root.join("src/models.zyl"),
            "pub struct Customer { name: String }\n",
        )
        .unwrap();
        let uri = Url::from_file_path(root.join("main.zyl"))
            .unwrap()
            .to_string();
        let server = Server {
            root: root.clone(),
            documents: HashMap::new(),
            shutdown: false,
        };
        let constructor = main.find("Customer {").unwrap() + 2;
        let params = json!({
            "textDocument":{"uri":uri},
            "position":offset_position(main, constructor)
        });
        let definition = server
            .semantic_response("textDocument/definition", &params)
            .unwrap();
        assert!(definition["uri"]
            .as_str()
            .unwrap()
            .ends_with("/src/models.zyl"));
        let references = server
            .semantic_response("textDocument/references", &params)
            .unwrap();
        assert_eq!(references.as_array().unwrap().len(), 5);
        let rename = server
            .semantic_response(
                "textDocument/rename",
                &json!({"textDocument":{"uri":uri},"position":offset_position(main,constructor),"newName":"Account"}),
            )
            .unwrap();
        let changes = rename["documentChanges"].as_array().unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().any(|change| change["textDocument"]["uri"]
            .as_str()
            .unwrap()
            .ends_with("/src/models.zyl")));
        let main_change = changes
            .iter()
            .find(|change| change["textDocument"]["uri"] == uri)
            .unwrap();
        assert_eq!(main_change["edits"].as_array().unwrap().len(), 5);
        assert!(main_change["edits"]
            .as_array()
            .unwrap()
            .iter()
            .all(|edit| edit["newText"] == "Account"));
        let field_offset = main.rfind("customer.name").unwrap() + "customer.".len();
        let field_params = json!({
            "textDocument":{"uri":uri},
            "position":offset_position(main, field_offset)
        });
        let field_definition = server
            .semantic_response("textDocument/definition", &field_params)
            .unwrap();
        assert!(field_definition["uri"]
            .as_str()
            .unwrap()
            .ends_with("/src/models.zyl"));
        let field_references = server
            .semantic_response("textDocument/references", &field_params)
            .unwrap();
        assert_eq!(
            field_references.as_array().unwrap().len(),
            2,
            "{field_references:?}"
        );
        let field_rename = server
            .semantic_response(
                "textDocument/rename",
                &json!({"textDocument":{"uri":uri},"position":offset_position(main,field_offset),"newName":"full_name"}),
            )
            .unwrap();
        let field_changes = field_rename["documentChanges"].as_array().unwrap();
        let changed_main = field_changes
            .iter()
            .find(|change| change["textDocument"]["uri"] == uri)
            .unwrap();
        assert_eq!(changed_main["edits"].as_array().unwrap().len(), 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lsp_outline_and_workspace_symbols_include_zelyra_resource_kinds() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-lsp-outline-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("main.zyl"),
            "enum OrderStatus { Draft Paid }\nstruct Customer { id: Id name: String }\ntype Quantity = Int\ntable customers { id: Id primary auto name: String(50) required }\nfn main() {}\n",
        )
        .unwrap();
        let uri = Url::from_file_path(root.join("main.zyl"))
            .unwrap()
            .to_string();
        let server = Server {
            root: root.clone(),
            documents: HashMap::new(),
            shutdown: false,
        };
        let outline = server.symbol_response(
            "textDocument/documentSymbol",
            &json!({"textDocument":{"uri":uri}}),
        );
        let names = outline
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|symbol| symbol.get("name").and_then(Value::as_str))
            .collect::<HashSet<_>>();
        assert!(names.contains("OrderStatus"), "outline: {outline:?}");
        assert!(names.contains("Customer"), "outline: {outline:?}");
        assert!(names.contains("Quantity"), "outline: {outline:?}");
        assert!(names.contains("customers"), "outline: {outline:?}");
        assert!(names.contains("main"), "outline: {outline:?}");
        let status = outline
            .as_array()
            .unwrap()
            .iter()
            .find(|symbol| symbol["name"] == "OrderStatus")
            .unwrap();
        assert_eq!(status["range"]["start"], json!({"line":0,"character":0}));
        assert_eq!(
            status["selectionRange"]["start"],
            json!({"line":0,"character":5})
        );
        assert_eq!(
            status["selectionRange"]["end"],
            json!({"line":0,"character":16})
        );
        assert!(status["range"]["end"]["character"].as_u64().unwrap() > 16);
        let search = server.symbol_response("workspace/symbol", &json!({"query":"OrderStatus"}));
        assert_eq!(search.as_array().unwrap().len(), 1);
        assert!(search[0]["location"].get("selectionRange").is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn table_navigation_and_rename_follow_same_module_form_references() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-lsp-table-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        let source = "table customers { id: Id primary auto name: String(100) required }\nform CustomerCreate -> customers { fields { name } }\nfn main() {}\n";
        fs::write(root.join("main.zyl"), source).unwrap();
        let uri = Url::from_file_path(root.join("main.zyl"))
            .unwrap()
            .to_string();
        let server = Server {
            root: root.clone(),
            documents: HashMap::new(),
            shutdown: false,
        };
        let offset = source.find("customers {").unwrap();
        let params = json!({
            "textDocument":{"uri":uri},
            "position":offset_position(source, offset)
        });
        let definition = server
            .semantic_response("textDocument/definition", &params)
            .unwrap();
        assert_eq!(definition["uri"], uri);
        assert_eq!(
            definition["range"]["start"],
            offset_position(source, offset)
        );
        let references = server
            .semantic_response("textDocument/references", &params)
            .unwrap();
        assert_eq!(references.as_array().unwrap().len(), 1);
        let rename = server
            .semantic_response(
                "textDocument/rename",
                &json!({"textDocument":{"uri":uri},"position":offset_position(source,offset),"newName":"clients"}),
            )
            .unwrap();
        let edits = rename["documentChanges"][0]["edits"].as_array().unwrap();
        assert_eq!(edits.len(), 2);
        assert!(edits.iter().all(|edit| edit["newText"] == "clients"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn table_navigation_and_rename_follow_unique_imported_resource_references() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-lsp-table-cross-file-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("zelyra.toml"),
            "[project]\nname = \"lsp-table-cross-file\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        let source = "import \"src/schema.zyl\" as schema\nform CustomerCreate -> customers { fields { name } }\nfn main() {}\n";
        let schema = "table customers { id: Id primary auto name: String(100) required access { read_write: [\"main.zyl\"] } }\n";
        fs::write(root.join("main.zyl"), source).unwrap();
        fs::write(root.join("src/schema.zyl"), schema).unwrap();
        let uri = Url::from_file_path(root.join("main.zyl"))
            .unwrap()
            .to_string();
        let schema_uri = Url::from_file_path(root.join("src/schema.zyl"))
            .unwrap()
            .to_string();
        let server = Server {
            root: root.clone(),
            documents: HashMap::new(),
            shutdown: false,
        };
        let offset = source.find("customers {").unwrap();
        let params = json!({
            "textDocument":{"uri":uri},
            "position":offset_position(source, offset)
        });
        let definition = server
            .semantic_response("textDocument/definition", &params)
            .unwrap();
        assert_eq!(definition["uri"], schema_uri);
        let references = server
            .semantic_response("textDocument/references", &params)
            .unwrap();
        assert_eq!(references.as_array().unwrap().len(), 1);
        let rename = server
            .semantic_response(
                "textDocument/rename",
                &json!({"textDocument":{"uri":uri},"position":offset_position(source,offset),"newName":"clients"}),
            )
            .unwrap();
        let changes = rename["documentChanges"].as_array().unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes
            .iter()
            .any(|change| change["textDocument"]["uri"] == schema_uri));
        assert!(changes
            .iter()
            .any(|change| change["textDocument"]["uri"] == uri));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn typed_hole_completion_uses_shared_scope_and_exact_edit_range() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-lsp-hole-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        let source = "fn pure() -> String { return \"x\" }\nfn target(name: String, count: Int) -> String { message: String = _ return message }\nfn main() {}\n";
        fs::write(root.join("main.zyl"), source).unwrap();
        let uri = Url::from_file_path(root.join("main.zyl"))
            .unwrap()
            .to_string();
        let server = Server {
            root: root.clone(),
            documents: HashMap::new(),
            shutdown: false,
        };
        let completion = server.symbol_response("textDocument/completion", &json!({
            "textDocument":{"uri":uri},"position":offset_position(source,source.find("= _").unwrap()+3)
        }));
        let items = completion.as_array().unwrap();
        let name = items.iter().find(|item| item["label"] == "name").unwrap();
        assert_eq!(name["textEdit"]["newText"], "name");
        assert_eq!(
            name["textEdit"]["range"]["start"],
            offset_position(source, source.find("= _").unwrap() + 2)
        );
        assert!(!items.iter().any(|item| item["label"] == "count"));
        fs::remove_dir_all(root).unwrap();
    }
}
