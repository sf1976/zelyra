use super::*;

pub(super) fn field_completion(
    state: &EditorState,
    request: &Value,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let relative = request
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| path.ends_with(".zyl"))
        .ok_or((400, "choose a Zelyra source file".into()))?;
    let _path = safe_editor_path(&state.root, relative, false)?;
    let source = request
        .get("source")
        .and_then(Value::as_str)
        .ok_or((400, "source is required".into()))?;
    if source.len() > EDITOR_MAX_FILE_BYTES {
        return Err((413, "source exceeds the 1 MiB editor limit".into()));
    }
    let dot_end = request
        .get("dotEnd")
        .and_then(Value::as_u64)
        .ok_or((400, "field access position is required".into()))?;
    let cursor = request
        .get("cursor")
        .and_then(Value::as_u64)
        .ok_or((400, "cursor position is required".into()))?;
    let dot_end_byte = utf16_to_byte(source, usize::try_from(dot_end).unwrap_or(usize::MAX));
    let cursor_byte = utf16_to_byte(source, usize::try_from(cursor).unwrap_or(usize::MAX));
    if dot_end_byte == 0
        || dot_end_byte > cursor_byte
        || source.get(dot_end_byte - 1..dot_end_byte) != Some(".")
    {
        return Err((400, "field access position is invalid".into()));
    }

    let mut overlays = editor_analysis_overlays(state, request)?;
    let current = overlays
        .iter_mut()
        .find(|overlay| overlay.path == relative)
        .ok_or((400, "source overlay is missing".into()))?;
    const PLACEHOLDER: &str = "__zelyra_completion_hole";
    current
        .text
        .replace_range(dot_end_byte..cursor_byte, PLACEHOLDER);

    let entry = state.root.join("main.zyl");
    if !entry.is_file() {
        return Err((422, "project entry file main.zyl is missing".into()));
    }
    let analysis = language_service::analyze(&entry, &overlays);
    let Some(program) = analysis.program.as_ref() else {
        return Ok((
            200,
            "application/json; charset=utf-8",
            json!({ "found": false }).to_string(),
        ));
    };
    let Some(source_id) = analysis
        .sources
        .iter()
        .position(|source| source.path == relative)
    else {
        return Ok((
            200,
            "application/json; charset=utf-8",
            json!({ "found": false }).to_string(),
        ));
    };
    let (inferred, _) = zelyra_runtime::inspect_expression_types(program);
    let receiver = inferred
        .iter()
        .filter(|item| {
            item.span.source_id as usize == source_id && item.span.end == dot_end_byte - 1
        })
        .max_by_key(|item| item.span.start);
    let Some(receiver) = receiver else {
        return Ok((
            200,
            "application/json; charset=utf-8",
            json!({ "found": false }).to_string(),
        ));
    };
    let options = record_completion_fields(program, &receiver.ty, source_id)
        .into_iter()
        .map(|(label, detail)| json!({ "label": label, "type": "property", "detail": detail }))
        .collect::<Vec<_>>();
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({ "found": !options.is_empty(), "from": dot_end, "to": cursor, "options": options })
            .to_string(),
    ))
}

fn record_completion_fields(
    program: &zelyra_ast::Program,
    ty: &zelyra_ast::Type,
    current_source_id: usize,
) -> Vec<(String, String)> {
    use zelyra_ast::Type;

    let mut name = match ty {
        Type::Named(name) | Type::Applied { name, .. } => name.as_str(),
        _ => return Vec::new(),
    };
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(name.to_owned()) {
            return Vec::new();
        }
        let records = program
            .records
            .iter()
            .filter(|record| record.name == name || record.name.rsplit("::").next() == Some(name));
        let records = records.collect::<Vec<_>>();
        if records.len() == 1 {
            let record = records[0];
            if record.span.source_id as usize != current_source_id && !record.is_public {
                return Vec::new();
            }
            return record
                .fields
                .iter()
                .map(|field| (field.name.clone(), field.ty.to_string()))
                .collect();
        }
        if !records.is_empty() {
            return Vec::new();
        }
        let aliases = program
            .types
            .iter()
            .filter(|alias| alias.name == name || alias.name.rsplit("::").next() == Some(name))
            .collect::<Vec<_>>();
        if aliases.len() != 1 {
            return Vec::new();
        }
        let alias = aliases[0];
        if alias.span.source_id as usize != current_source_id && !alias.is_public {
            return Vec::new();
        }
        let Type::Named(target) = &alias.target else {
            return Vec::new();
        };
        name = target;
    }
}
