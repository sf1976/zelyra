//! Static Studio canvas using the project compiler's real page composition.
//! No project function, query, handler, or JavaScript is executed here.
use super::*;
use std::collections::HashMap;
use zelyra_ast::Span;

type EditorReply = Result<(u16, &'static str, String), (u16, String)>;
const MAX_SAMPLE_BYTES: usize = 65_536;
const MAX_COLLECTION_ROWS: usize = 100;
const ZELYRA_UI_CSS: &str = include_str!("../../web/assets/zelyra-ui.v1.css");

struct SampleData {
    values: HashMap<String, String>,
    collections: HashMap<String, Vec<HashMap<String, String>>>,
}

fn analysis(
    state: &EditorState,
    request: &Value,
) -> Result<language_service::Analysis, (u16, String)> {
    let entry = state.root.join("main.zyl");
    if !entry.is_file() {
        return Err((422, "project entry file main.zyl is missing".into()));
    }
    let overlays = editor_analysis_overlays(state, request)?;
    Ok(language_service::analyze(&entry, &overlays))
}

fn source_position(analysis: &language_service::Analysis, span: Span) -> Value {
    let path = analysis
        .sources
        .get(span.source_id as usize)
        .map(|source| source.path.as_str());
    json!({"path":path,"line":span.line,"column":span.column})
}

pub(super) fn list(state: &EditorState, request: &Value) -> EditorReply {
    let analysis = analysis(state, request)?;
    let mut entries = Vec::new();
    if let Some(program) = &analysis.program {
        for page in &program.pages {
            entries.push(json!({"kind":"page","name":page.path,"source":source_position(&analysis,page.span)}));
        }
        for view in &program.views {
            entries.push(json!({"kind":"view","name":view.name,"source":source_position(&analysis,view.span)}));
        }
    }
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({
            "success":analysis.success,"entries":entries,"diagnostics":analysis.diagnostics,
            "version":request.get("version").and_then(Value::as_i64).unwrap_or(0),
        })
        .to_string(),
    ))
}

fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::String(text) if text.len() <= 4096 => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        _ => None,
    }
}

fn sample_data(value: &Value) -> Result<SampleData, (u16, String)> {
    if value.to_string().len() > MAX_SAMPLE_BYTES {
        return Err((413, "sample data exceeds 64 KiB".into()));
    }
    let object = value
        .as_object()
        .ok_or((400, "sample data must be a JSON object".into()))?;
    let mut values = HashMap::new();
    let mut collections = HashMap::new();
    for (key, value) in object {
        if key.is_empty() || key.len() > 120 {
            return Err((400, "sample data key is invalid".into()));
        }
        if let Some(text) = scalar(value) {
            values.insert(key.clone(), text);
        } else if let Some(fields) = value.as_object() {
            for (field, value) in fields {
                if field.len() > 120 {
                    return Err((400, "sample field name is too long".into()));
                }
                if let Some(text) = scalar(value) {
                    values.insert(format!("{key}.{field}"), text);
                }
            }
        } else if let Some(rows) = value.as_array() {
            if rows.len() > MAX_COLLECTION_ROWS {
                return Err((413, "sample collection exceeds 100 rows".into()));
            }
            let mut items = Vec::new();
            for row in rows {
                let fields = row
                    .as_object()
                    .ok_or((400, "sample collection rows must be objects".into()))?;
                let mut item = HashMap::new();
                for (field, value) in fields {
                    if field.len() > 120 {
                        return Err((400, "sample field name is too long".into()));
                    }
                    if let Some(text) = scalar(value) {
                        item.insert(field.clone(), text);
                    }
                }
                items.push(item);
            }
            collections.insert(key.clone(), items);
        } else if !value.is_null() {
            return Err((
                400,
                "sample data supports scalars, records, and arrays of records".into(),
            ));
        }
    }
    Ok(SampleData {
        values,
        collections,
    })
}

pub(super) fn render(state: &EditorState, request: &Value) -> EditorReply {
    let kind = request
        .get("kind")
        .and_then(Value::as_str)
        .filter(|kind| matches!(*kind, "page" | "view"))
        .ok_or((400, "choose a page or view".into()))?;
    let name = request
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty() && name.len() <= 256)
        .ok_or((400, "choose a declared page or view".into()))?;
    let sample = request
        .get("sample")
        .ok_or((400, "sample data is required".into()))?;
    let samples = sample_data(sample)?;
    let analysis = analysis(state, request)?;
    if !analysis.success {
        return Err((
            422,
            "project must pass compiler checks before canvas rendering".into(),
        ));
    }
    let program = analysis
        .program
        .as_ref()
        .ok_or((422, "project could not be loaded".into()))?;
    let (composed, span) = if kind == "page" {
        let page = program
            .pages
            .iter()
            .find(|page| page.path == name)
            .ok_or((404, "page is not declared".into()))?;
        (super::super::compose_page_view(program, page), page.span)
    } else {
        let view = program
            .views
            .iter()
            .find(|view| view.name == name)
            .ok_or((404, "view is not declared".into()))?;
        (
            super::super::compose_view_html(program, &view.name, ""),
            view.span,
        )
    };
    let html = zelyra_web::render_static_template_preview(
        &composed,
        &samples.values,
        &samples.collections,
    );
    let entry = state.root.join("main.zyl");
    let stylesheet = super::super::project_ui_stylesheet_css(&entry.to_string_lossy())
        .map_err(|error| (422, error))?
        .unwrap_or_else(|| ZELYRA_UI_CSS.to_owned());
    let theme = if let Some(source) = request.get("themeSource").and_then(Value::as_str) {
        if source.len() > 131_072 {
            return Err((413, "theme source exceeds 128 KiB".into()));
        }
        let _ = safe_editor_path(&state.root, "zelyra.theme.css", false)?;
        source.to_owned()
    } else {
        super::super::project_theme_css(&entry.to_string_lossy())
            .map_err(|error| (422, error))?
            .unwrap_or_default()
    };
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({
            "kind":kind,"name":name,"html":html,"stylesheet":stylesheet,"theme":theme,
            "source":source_position(&analysis,span),
            "version":request.get("version").and_then(Value::as_i64).unwrap_or(0),
            "sampleOnly":true,"scriptsExecuted":false,
        })
        .to_string(),
    ))
}
