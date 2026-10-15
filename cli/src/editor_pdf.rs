//! Explicit, data-only Studio PDF preview. Project functions are never run.
use super::*;
use zelyra_runtime::Value as RuntimeValue;

const MAX_RECORD_NODES: usize = 10_000;

pub(super) fn views(state: &EditorState) -> Result<(u16, &'static str, String), (u16, String)> {
    let entry = state.root.join("main.zyl");
    let loaded = project::load(&entry.to_string_lossy())
        .map_err(|error| (422, format!("{}: {}", error.code, error.message)))?;
    let names = loaded
        .program
        .views
        .iter()
        .map(|view| view.name.as_str())
        .collect::<Vec<_>>();
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"views":names}).to_string(),
    ))
}

pub(super) fn render(
    state: &EditorState,
    request: &Value,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let view_name = request
        .get("view")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty() && name.len() <= 128)
        .ok_or((400, "choose a declared view".into()))?;
    let record = request
        .get("record")
        .filter(|record| record.is_object())
        .ok_or((400, "PDF example data must be a JSON object".into()))?;
    let mut nodes = 0;
    let data = example_value(record, 0, &mut nodes)?;

    let entry = state.root.join("main.zyl");
    if !entry.is_file() {
        return Err((422, "project entry file main.zyl is missing".into()));
    }
    let loaded = if let (Some(path), Some(source)) = (
        request.get("path").and_then(Value::as_str),
        request.get("source").and_then(Value::as_str),
    ) {
        let _ = safe_editor_path(&state.root, path, false)?;
        project::load_with_source_overlays(
            &entry.to_string_lossy(),
            &[(path.to_owned(), source.to_owned())],
        )
    } else {
        project::load(&entry.to_string_lossy())
    }
    .map_err(|error| (422, format!("{}: {}", error.code, error.message)))?;
    let view = loaded
        .program
        .views
        .iter()
        .find(|view| view.name == view_name)
        .ok_or((404, format!("view `{view_name}` is not declared")))?;
    zelyra_runtime::check(&loaded.program).map_err(|_| {
        (
            422,
            "project must typecheck before rendering a PDF preview".into(),
        )
    })?;
    let bytes =
        zelyra_runtime::render_pdf_view(view, &data).map_err(|error| (422, error.to_owned()))?;
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({
            "view":view_name,
            "mimeType":"application/pdf",
            "bytes":bytes.len(),
            "base64":base64_encode(&bytes),
        })
        .to_string(),
    ))
}

fn example_value(
    value: &Value,
    depth: usize,
    nodes: &mut usize,
) -> Result<RuntimeValue, (u16, String)> {
    *nodes += 1;
    if depth > 16 || *nodes > MAX_RECORD_NODES {
        return Err((
            413,
            "PDF example data exceeds nesting or item limits".into(),
        ));
    }
    match value {
        Value::Null => Ok(RuntimeValue::Option(None)),
        Value::Bool(boolean) => Ok(RuntimeValue::Bool(*boolean)),
        Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                Ok(RuntimeValue::Int(integer))
            } else if let Some(integer) = number.as_u64() {
                Ok(RuntimeValue::UInt(integer))
            } else {
                number
                    .as_f64()
                    .filter(|float| float.is_finite())
                    .map(RuntimeValue::Float)
                    .ok_or((400, "PDF example number is invalid".into()))
            }
        }
        Value::String(string) => Ok(RuntimeValue::String(string.clone())),
        Value::Array(items) => items
            .iter()
            .map(|item| example_value(item, depth + 1, nodes))
            .collect::<Result<Vec<_>, _>>()
            .map(RuntimeValue::Array),
        Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| {
                Ok((
                    RuntimeValue::String(key.clone()),
                    example_value(value, depth + 1, nodes)?,
                ))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(RuntimeValue::Map),
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        output.push(ALPHABET[(first >> 2) as usize] as char);
        output.push(ALPHABET[(((first & 3) << 4) | (second >> 4)) as usize] as char);
        output.push(if chunk.len() > 1 {
            ALPHABET[(((second & 15) << 2) | (third >> 6)) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            ALPHABET[(third & 63) as usize] as char
        } else {
            '='
        });
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_data_keeps_nested_records_and_rejects_large_trees() {
        let value = json!({"number":"R-1","lines":[{"price":12}],"comment":null});
        let result = example_value(&value, 0, &mut 0).unwrap();
        assert!(matches!(result, RuntimeValue::Map(_)));
        assert!(example_value(&value, 17, &mut 0).is_err());
    }

    #[test]
    fn base64_encoder_matches_pdf_header() {
        assert_eq!(base64_encode(b"%PDF-1.4"), "JVBERi0xLjQ=");
    }
}
