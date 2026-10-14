//! Stdio JSON-RPC language server. All compiler diagnostics come from the
//! shared source-overlay service used by Studio.

use super::*;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use url::Url;

const MAX_MESSAGE_BYTES: usize = 2 * 1_048_576;

#[derive(Clone)]
struct Document {
    uri: String,
    relative: String,
    text: String,
    version: i64,
}

struct Server {
    root: PathBuf,
    documents: HashMap<String, Document>,
    shutdown: bool,
}

pub fn command() -> ExitCode {
    let stdin = io::stdin();
    let stdout = io::stdout();
    match serve(BufReader::new(stdin.lock()), stdout.lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error[E-LSP-001]: {error}");
            ExitCode::from(1)
        }
    }
}

fn serve<R: BufRead, W: Write>(mut input: R, mut output: W) -> io::Result<()> {
    let mut server = Server {
        root: fs::canonicalize(".")?,
        documents: HashMap::new(),
        shutdown: false,
    };
    while let Some(message) = read_message(&mut input)? {
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            continue;
        };
        let id = message.get("id").cloned();
        let params = message.get("params").cloned().unwrap_or(Value::Null);
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
                                    "completionProvider": {"triggerCharacters": [".", ":"]}
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
                return if server.shutdown {
                    Ok(())
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "LSP exit before shutdown",
                    ))
                }
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
            "textDocument/documentSymbol" | "workspace/symbol" | "textDocument/completion" => {
                if let Some(id) = id {
                    let result = server.symbol_response(method, &params);
                    write_message(
                        &mut output,
                        &json!({"jsonrpc":"2.0","id":id,"result":result}),
                    )?;
                }
            }
            "$/cancelRequest" => {}
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
    Ok(())
}

impl Server {
    fn safe_relative(&self, uri: &str) -> Option<String> {
        let path = fs::canonicalize(uri_path(uri)?).ok()?;
        let relative = path.strip_prefix(&self.root).ok()?;
        if path.extension().and_then(|value| value.to_str()) != Some("zyl") || !path.is_file() {
            return None;
        }
        Some(relative.to_string_lossy().replace('\\', "/"))
    }

    fn analysis(&self) -> language_service::Analysis {
        let entry = self.root.join("main.zyl");
        let entry = if entry.is_file() {
            entry
        } else {
            self.documents
                .values()
                .min_by(|left, right| left.relative.cmp(&right.relative))
                .map_or(entry, |document| self.root.join(&document.relative))
        };
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
        let mut symbols = Vec::new();
        for function in &program.functions {
            if let Some(source) = analysis.sources.get(function.span.source_id as usize) {
                let plain_name = function.name.rsplit("::").next().unwrap_or(&function.name);
                let location = declaration_location(&self.root, source, function.span, plain_name);
                symbols.push((source.path.clone(), plain_name.to_owned(), 12, location));
            }
        }
        for record in &program.records {
            if let Some(source) = analysis.sources.get(record.span.source_id as usize) {
                let plain_name = record.name.rsplit("::").next().unwrap_or(&record.name);
                let location = declaration_location(&self.root, source, record.span, plain_name);
                symbols.push((source.path.clone(), plain_name.to_owned(), 23, location));
            }
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
                        "name":name,"kind":kind,"range":location["range"],"selectionRange":location["range"]
                    })).collect::<Vec<_>>())
            }
            _ => {
                let query = params
                    .get("query")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_lowercase();
                json!(symbols.iter().filter(|(_, name, _, _)| name.to_lowercase().contains(&query))
                    .map(|(_, name, kind, location)| json!({"name":name,"kind":kind,"location":location}))
                    .collect::<Vec<_>>())
            }
        }
    }
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
        serve(BufReader::new(input.as_slice()), &mut output).unwrap();
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
}
