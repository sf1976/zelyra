//! Compiler-backed capability inspection and explicitly reviewed project grants.
use super::*;
use toml_edit::{value as toml_value, DocumentMut, Item, Table};

type EditorReply = Result<(u16, &'static str, String), (u16, String)>;

fn entry(state: &EditorState) -> Result<PathBuf, (u16, String)> {
    let entry = state.root.join("main.zyl");
    if !entry.is_file() {
        return Err((422, "project entry file main.zyl is missing".into()));
    }
    Ok(entry)
}

pub(super) fn inspect(state: &EditorState, request: &Value) -> EditorReply {
    let relative = request
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| path.ends_with(".zyl"))
        .ok_or((400, "choose a Zelyra source file".into()))?;
    let _ = safe_editor_path(&state.root, relative, false)?;
    let source = request.get("source").and_then(Value::as_str);
    if source.is_some_and(|text| text.len() > EDITOR_MAX_FILE_BYTES) {
        return Err((413, "source exceeds the 1 MiB editor limit".into()));
    }
    let version = request.get("version").and_then(Value::as_i64).unwrap_or(0);
    let entry = entry(state)?;
    let overlays = source
        .map(|text| {
            vec![language_service::SourceOverlay {
                path: relative.into(),
                text: text.into(),
                version,
            }]
        })
        .unwrap_or_default();
    let analysis = language_service::analyze(&entry, &overlays);
    let grants =
        super::project_capability_grants(&entry.to_string_lossy()).map_err(|error| (422, error))?;
    let policy =
        super::project_filesystem_policy(&entry.to_string_lossy()).map_err(|error| (422, error))?;
    let mut declared = Vec::new();
    let mut needed = Vec::new();
    let mut file_paths = Vec::new();
    let mut project_capabilities = HashSet::new();
    if let Some(program) = &analysis.program {
        for function in &program.functions {
            project_capabilities.extend(function.capabilities.iter().cloned());
            if analysis
                .sources
                .get(function.span.source_id as usize)
                .is_some_and(|source| source.path == relative)
            {
                for capability in &function.capabilities {
                    declared.push(json!({"function":function.name,"capability":capability,
                        "line":function.span.line,
                        "granted":grants.as_ref().is_none_or(|grants| grants_capability(grants, capability))}));
                }
            }
        }
        for usage in zelyra_runtime::inspect_capability_uses(program) {
            project_capabilities.insert(usage.capability.clone());
            if !analysis
                .sources
                .get(usage.span.source_id as usize)
                .is_some_and(|source| source.path == relative)
            {
                continue;
            }
            let granted = grants
                .as_ref()
                .is_none_or(|grants| grants_capability(grants, &usage.capability));
            needed.push(
                json!({"function":usage.function,"capability":usage.capability,
                "operation":usage.operation,"line":usage.span.line,"granted":granted}),
            );
            for path in usage.file_paths {
                let (status, reason) = match path.path.as_deref() {
                    None => (
                        "unknown",
                        "dynamic path cannot be checked statically".to_owned(),
                    ),
                    Some("") => ("invalid", "empty file path".to_owned()),
                    Some(literal) => match zelyra_runtime::inspect_filesystem_path(
                        policy.as_ref(),
                        literal,
                        path.write,
                        path.allow_missing,
                    ) {
                        Ok(_) if policy.is_none() => (
                            "unrestricted",
                            "no project filesystem policy is configured".to_owned(),
                        ),
                        Ok(_) => (
                            "allowed",
                            "path is within the configured roots at inspection time".to_owned(),
                        ),
                        Err(error) if error.contains("outside the configured roots") => {
                            ("denied", error)
                        }
                        Err(error) => ("unknown", error),
                    },
                };
                file_paths.push(json!({"function":usage.function,"line":usage.span.line,
                    "path":path.path,"access":if path.write {"write"} else {"read"},
                    "status":status,"reason":reason}));
            }
        }
    }
    let grant_values = grants.as_ref().map(|values| {
        let mut values = values.iter().cloned().collect::<Vec<_>>();
        values.sort();
        values
    });
    let mut grants_without_function_use = grants
        .as_ref()
        .filter(|_| analysis.success)
        .map(|values| {
            values
                .iter()
                .filter(|grant| {
                    !project_capabilities.iter().any(|capability| {
                        grant.as_str() == capability
                            || (grant.as_str() == "Database" && capability.starts_with("Database("))
                            || (capability == "Database" && grant.starts_with("Database("))
                    })
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    grants_without_function_use.sort();
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({
            "path":relative,"version":version,"success":analysis.success,
            "declared":declared,"needed":needed,"filePaths":file_paths,
            "grants":grant_values,"configured":grants.is_some(),
        "grantsWithoutFunctionUse":grants_without_function_use,
            "readRoots":policy.as_ref().map(|policy| &policy.read_roots),
            "writeRoots":policy.as_ref().map(|policy| &policy.write_roots),
            "diagnostics":analysis.diagnostics,
        })
        .to_string(),
    ))
}

fn grants_capability(grants: &HashSet<String>, capability: &str) -> bool {
    grants.contains(capability)
        || (capability.starts_with("Database(") && grants.contains("Database"))
        || (capability == "Database"
            && grants.contains("Database(read)")
            && grants.contains("Database(write)"))
}

fn grant_key(capability: &str) -> Option<&'static str> {
    match capability {
        "Database" => Some("database"),
        "Database(read)" => Some("database_read"),
        "Database(write)" => Some("database_write"),
        "Email" => Some("email"),
        "Network" => Some("network"),
        "FileSystem" => Some("filesystem"),
        "Environment" => Some("environment"),
        "Process" => Some("process"),
        "Clock" => Some("clock"),
        "Random" => Some("random"),
        "Console" => Some("console"),
        _ => None,
    }
}

fn grant_candidate(state: &EditorState, request: &Value) -> Result<Value, (u16, String)> {
    let capability = request
        .get("capability")
        .and_then(Value::as_str)
        .ok_or((400, "capability is required".into()))?;
    let key = grant_key(capability).ok_or((400, "unknown capability".into()))?;
    let path = safe_editor_path(&state.root, "zelyra.toml", false)?;
    let before = fs::read_to_string(&path)
        .map_err(|_| (415, "project configuration must be UTF-8 text".into()))?;
    if before.len() > EDITOR_MAX_FILE_BYTES {
        return Err((413, "project configuration exceeds the editor limit".into()));
    }
    let mut document = before
        .parse::<DocumentMut>()
        .map_err(|error| (422, format!("invalid project TOML: {error}")))?;
    if !document.contains_key("capabilities") {
        document["capabilities"] = Item::Table(Table::new());
    }
    let table = document["capabilities"]
        .as_table_mut()
        .ok_or((422, "[capabilities] is not a TOML table".into()))?;
    table[key] = toml_value(true);
    let after = document.to_string();
    Ok(
        json!({"capability":capability,"key":key,"path":"zelyra.toml",
        "before":before,"after":after,"baseHash":content_hash(before.as_bytes())}),
    )
}

pub(super) fn preview_grant(state: &EditorState, request: &Value) -> EditorReply {
    let preview = grant_candidate(state, request)?;
    Ok((200, "application/json; charset=utf-8", preview.to_string()))
}

pub(super) fn apply_grant(state: &EditorState, request: &Value) -> EditorReply {
    if request.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err((400, "explicit confirmation is required".into()));
    }
    let preview = grant_candidate(state, request)?;
    if request.get("baseHash") != preview.get("baseHash")
        || request.get("after") != preview.get("after")
    {
        return Err((
            409,
            "project configuration changed; review the grant again".into(),
        ));
    }
    let saved = write_existing_file(
        state,
        json!({
            "path":"zelyra.toml","content":preview["after"],"baseHash":preview["baseHash"]
        }),
        false,
    )?;
    Ok(saved)
}
