//! Shared compiler-backed source analysis for Studio and the stdio LSP.
//!
//! Source overlays are immutable inputs to one analysis. The project loader
//! keeps their source IDs and import graph; diagnostics therefore refer to the
//! exact unsaved text rather than to a copied temporary project.

use super::*;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct SourceOverlay {
    pub path: String,
    pub text: String,
    pub version: i64,
}

pub struct Analysis {
    pub success: bool,
    pub diagnostics: Vec<Value>,
    pub program: Option<zelyra_ast::Program>,
    pub sources: Vec<project::ProjectSource>,
    pub modules: Vec<project::ProjectModule>,
    pub versions: HashMap<String, i64>,
}

pub fn analyze(entry: &Path, overlays: &[SourceOverlay]) -> Analysis {
    let entry_path = entry.to_string_lossy();
    let entry_name = entry
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let entry_source = overlays
        .iter()
        .find(|overlay| overlay.path == entry_name)
        .map(|overlay| overlay.text.clone())
        .or_else(|| fs::read_to_string(entry).ok())
        .unwrap_or_default();
    begin_json_diagnostics(&entry_path, &entry_source);
    let inputs = overlays
        .iter()
        .map(|overlay| (overlay.path.clone(), overlay.text.clone()))
        .collect::<Vec<_>>();
    let (success, program, sources, modules) =
        match load_project_with(project::load_with_source_overlays(&entry_path, &inputs)) {
            Ok(loaded) => {
                let sources = loaded.sources.clone();
                let modules = loaded.modules.clone();
                let program = loaded.program.clone();
                (
                    validate_loaded_project(&entry_path, loaded).is_ok(),
                    Some(program),
                    sources,
                    modules,
                )
            }
            Err(()) => (
                false,
                None,
                PROJECT_SOURCES.with(|project_sources| project_sources.borrow().clone()),
                Vec::new(),
            ),
        };
    let root = entry.parent().unwrap_or_else(|| Path::new("."));
    let diagnostics = finish_json_diagnostics()
        .into_iter()
        .map(|mut diagnostic| {
            if let Some(file) = diagnostic.get("file").and_then(Value::as_str) {
                let relative = Path::new(file)
                    .strip_prefix(root)
                    .map(|path| path.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_else(|_| file.to_owned());
                if let Some(source) = sources
                    .iter()
                    .find(|source| source.path == relative)
                    .map(|source| source.text.as_str())
                    .or_else(|| {
                        overlays
                            .iter()
                            .find(|item| item.path == relative)
                            .map(|item| item.text.as_str())
                    })
                {
                    let start = diagnostic
                        .pointer("/span/start/offset")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize;
                    let end = diagnostic
                        .pointer("/span/end/offset")
                        .and_then(Value::as_u64)
                        .unwrap_or(start as u64) as usize;
                    diagnostic["editorSpan"] = json!({
                        "start": utf16_offset(source, start),
                        "end": utf16_offset(source, end),
                    });
                }
                diagnostic["file"] = Value::String(relative);
            }
            diagnostic
        })
        .collect();
    Analysis {
        success,
        diagnostics,
        program,
        sources,
        modules,
        versions: overlays
            .iter()
            .map(|overlay| (overlay.path.clone(), overlay.version))
            .collect(),
    }
}

fn utf16_offset(source: &str, byte_offset: usize) -> usize {
    let mut safe_offset = byte_offset.min(source.len());
    while !source.is_char_boundary(safe_offset) {
        safe_offset -= 1;
    }
    source[..safe_offset].encode_utf16().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_unsaved_error_keeps_file_span_and_document_version() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-language-service-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("main.zyl"),
            "import \"src/math.zyl\" as math\nfn main() { print(math::answer()) }\n",
        )
        .unwrap();
        fs::write(
            root.join("src/math.zyl"),
            "pub fn answer() -> Int { return 1 }\n",
        )
        .unwrap();
        let analysis = analyze(
            &root.join("main.zyl"),
            &[SourceOverlay {
                path: "src/math.zyl".into(),
                text: "pub fn answer() -> Int { return \"wrong\" }\n".into(),
                version: 7,
            }],
        );
        assert!(!analysis.success);
        assert_eq!(analysis.versions["src/math.zyl"], 7);
        assert!(
            analysis.diagnostics.iter().any(|diagnostic| {
                diagnostic["file"] == "src/math.zyl"
                    && diagnostic["code"] == "E-TYPE-001"
                    && diagnostic["editorSpan"]["end"].as_u64().unwrap_or(0)
                        > diagnostic["editorSpan"]["start"].as_u64().unwrap_or(0)
            }),
            "diagnostics: {:?}",
            analysis.diagnostics
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn utf16_offset_counts_surrogate_pairs_and_handles_midpoint() {
        assert_eq!(utf16_offset("a😀b", 1), 1);
        assert_eq!(utf16_offset("a😀b", 3), 1);
        assert_eq!(utf16_offset("a😀b", 5), 3);
    }
}
