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

pub fn verification_entries(analysis: &Analysis) -> Vec<Value> {
    if !analysis.success {
        return Vec::new();
    }
    let Some(program) = analysis.program.as_ref() else {
        return Vec::new();
    };
    verify_program(program).into_iter().filter_map(|result| {
        let source = analysis.sources.get(result.span.source_id as usize)?;
        let counterexample = result.counterexample.map(|values| {
            values.into_iter().map(|(name, value)| (name, json!(value))).collect::<Map<String, Value>>()
        });
        Some(json!({
            "file":source.path,
            "function":result.function,
            "kind":result.kind.to_string(),
            "index":result.index,
            "status":result.status.to_string(),
            "code":result.status.code(),
            "message":result.message,
            "counterexample":counterexample,
            "span":span_value(&source.text, result.span),
            "editorSpan": {"start":utf16_offset(&source.text,result.span.start),"end":utf16_offset(&source.text,result.span.end)},
            "version":analysis.versions.get(&source.path),
        }))
    }).collect()
}

/// Return a single expression hole with conservative, type- and capability-
/// compatible replacements. Unknown inferred types are not advertised as
/// safe replacements. `_` in a pattern is never an expression hole.
pub fn typed_hole_entry(analysis: &Analysis, path: &str, byte_offset: usize) -> Option<Value> {
    let program = analysis.program.as_ref()?;
    let holes = collect_typed_holes(program);
    let hole = holes.into_iter().find(|hole| {
        analysis
            .sources
            .get(hole.span.source_id as usize)
            .is_some_and(|source| source.path == path)
            && hole.span.start <= byte_offset
            && byte_offset <= hole.span.end
    })?;
    let source = analysis.sources.get(hole.span.source_id as usize)?;
    let expected = hole.expected_type.as_ref();
    let mut suggestions = Vec::new();
    if let Some(expected) = expected {
        for (name, ty) in &hole.typed_values {
            if ty == expected {
                suggestions.push(json!({
                    "expression":name,"kind":"value","type":ty.to_string(),
                    "contractReview":false
                }));
            }
        }
        for function in &program.functions {
            if !function.params.is_empty()
                || function.return_type.as_ref() != Some(expected)
                || !function
                    .capabilities
                    .iter()
                    .all(|capability| hole.capabilities.contains(capability))
            {
                continue;
            }
            let Some(owner) = analysis.sources.get(function.span.source_id as usize) else {
                continue;
            };
            let plain = function.name.rsplit("::").next().unwrap_or(&function.name);
            let expression = if owner.path == path {
                format!("{plain}()")
            } else {
                if !function.is_public {
                    continue;
                }
                let Some(alias) = analysis
                    .modules
                    .iter()
                    .find(|module| module.path == path)
                    .and_then(|module| {
                        module
                            .imports
                            .iter()
                            .find(|import| import.path == owner.path)
                    })
                    .map(|import| import.alias.as_str())
                else {
                    continue;
                };
                format!("{alias}::{plain}()")
            };
            suggestions.push(json!({
                "expression":expression,"kind":"function","type":expected.to_string(),
                "contractReview":!function.requires.is_empty() || !function.ensures.is_empty(),
                "capabilities":function.capabilities,
            }));
        }
        let literals: &[&str] = match expected {
            Type::Bool => &["true", "false"],
            Type::Int => &["0"],
            Type::Float => &["0.0"],
            Type::String => &["\"\""],
            Type::Option(_) => &["None"],
            _ => &[],
        };
        for expression in literals {
            suggestions.push(json!({
                "expression":expression,"kind":"literal","type":expected.to_string(),
                "contractReview":false
            }));
        }
    }
    suggestions.sort_by(|left, right| {
        left["expression"]
            .as_str()
            .cmp(&right["expression"].as_str())
    });
    suggestions.dedup_by(|left, right| left["expression"] == right["expression"]);
    Some(json!({
        "found":true,
        "file":path,
        "expectedType":expected.map(ToString::to_string),
        "editorSpan":{"start":utf16_offset(&source.text,hole.span.start),"end":utf16_offset(&source.text,hole.span.end)},
        "visibleValues":hole.typed_values.iter().map(|(name,ty)| json!({"name":name,"type":ty.to_string()})).collect::<Vec<_>>(),
        "capabilities":hole.capabilities,
        "contracts":hole.contract_spans.iter().filter_map(|span| {
            analysis.sources.get(span.source_id as usize).map(|source| json!({
                "file":source.path,"span":span_value(&source.text,*span)
            }))
        }).collect::<Vec<_>>(),
        "suggestions":suggestions,
        "version":analysis.versions.get(path),
    }))
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

    #[test]
    fn verifier_entries_keep_proof_statuses_and_imported_source_positions() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-verify-overlay-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("main.zyl"),
            "import \"src/contracts.zyl\" as contracts\nfn main() {}\n",
        )
        .unwrap();
        fs::write(root.join("src/contracts.zyl"), "pub fn check() {}\n").unwrap();
        let source = "pub fn proven() requires { 1 < 2 } { }\npub fn dynamic(value: Int) requires { value >= 0 } { }\npub fn failed() ensures { 1 > 2 } { }\n";
        let analysis = analyze(
            &root.join("main.zyl"),
            &[SourceOverlay {
                path: "src/contracts.zyl".into(),
                text: source.into(),
                version: 9,
            }],
        );
        assert!(analysis.success, "{:?}", analysis.diagnostics);
        let entries = verification_entries(&analysis);
        assert!(entries
            .iter()
            .any(|item| item["status"] == "PROVEN" && item["code"] == "V-001"));
        assert!(entries
            .iter()
            .any(|item| item["status"] == "RUNTIME_CHECK" && item["code"] == "V-002"));
        assert!(entries
            .iter()
            .any(|item| item["status"] == "FAILED" && item["code"] == "V-004"));
        assert!(entries.iter().any(|item| item["file"] == "main.zyl"
            && item["status"] == "UNPROVEN"
            && item["code"] == "V-003"));
        assert!(entries
            .iter()
            .filter(|item| item["file"] == "src/contracts.zyl")
            .all(|item| item["version"] == 9));
        assert_eq!(
            entries
                .iter()
                .find(|item| item["status"] == "PROVEN")
                .unwrap()["span"]["start"]["line"],
            1
        );
        assert_eq!(
            fs::read_to_string(root.join("src/contracts.zyl")).unwrap(),
            "pub fn check() {}\n"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn syntax_errors_never_become_proven_verifier_results() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-verify-error-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("main.zyl"), "fn main() {}\n").unwrap();
        let analysis = analyze(
            &root.join("main.zyl"),
            &[SourceOverlay {
                path: "main.zyl".into(),
                text: "fn main( {\n".into(),
                version: 2,
            }],
        );
        assert!(!analysis.success);
        assert!(verification_entries(&analysis).is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hole_suggestions_respect_scope_exact_type_and_capability() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-hole-hud-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        let source = "fn pure() -> String { return \"x\" }\nfn effect() -> String uses Console { return \"y\" }\nfn target(name: String, count: Int) -> String { message: String = _ return message }\nfn main() {}\n";
        fs::write(root.join("main.zyl"), source).unwrap();
        let analysis = analyze(
            &root.join("main.zyl"),
            &[SourceOverlay {
                path: "main.zyl".into(),
                text: source.into(),
                version: 4,
            }],
        );
        let hole_offset = source.find("= _").unwrap() + 2;
        let hole = typed_hole_entry(&analysis, "main.zyl", hole_offset).unwrap();
        assert_eq!(hole["expectedType"], "String");
        assert_eq!(hole["version"], 4);
        let suggestions = hole["suggestions"].as_array().unwrap();
        let expressions = suggestions
            .iter()
            .filter_map(|item| item["expression"].as_str())
            .collect::<Vec<_>>();
        assert!(expressions.contains(&"name"), "{expressions:?}");
        assert!(expressions.contains(&"pure()"), "{expressions:?}");
        assert!(!expressions.contains(&"count"), "{expressions:?}");
        assert!(!expressions.contains(&"effect()"), "{expressions:?}");
        assert!(analysis
            .diagnostics
            .iter()
            .any(|item| item["code"] == "E-HOLE-001"));
        let candidate = source.replacen("= _", "= pure()", 1);
        let checked = analyze(
            &root.join("main.zyl"),
            &[SourceOverlay {
                path: "main.zyl".into(),
                text: candidate,
                version: 5,
            }],
        );
        assert!(checked.success, "{:?}", checked.diagnostics);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn match_wildcards_are_not_expression_holes() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-hole-wildcard-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        let source = "fn main() { match true { _ => { } } }\n";
        fs::write(root.join("main.zyl"), source).unwrap();
        let analysis = analyze(&root.join("main.zyl"), &[]);
        assert!(analysis.success, "{:?}", analysis.diagnostics);
        assert!(typed_hole_entry(&analysis, "main.zyl", source.find('_').unwrap()).is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn imported_hole_suggestion_uses_declared_alias_and_flags_contracts() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-hole-import-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        let source = "import \"src/text.zyl\" as text\nfn target() -> String { value: String = _ return value }\nfn main() {}\n";
        fs::write(root.join("main.zyl"), source).unwrap();
        fs::write(
            root.join("src/text.zyl"),
            "pub fn make() -> String requires { 1 < 2 } { return \"hello\" }\n",
        )
        .unwrap();
        let analysis = analyze(&root.join("main.zyl"), &[]);
        let hole =
            typed_hole_entry(&analysis, "main.zyl", source.find("= _").unwrap() + 2).unwrap();
        let suggestion = hole["suggestions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["expression"] == "text::make()")
            .unwrap();
        assert_eq!(suggestion["contractReview"], true);
        let candidate = source.replacen("= _", "= text::make()", 1);
        let checked = analyze(
            &root.join("main.zyl"),
            &[SourceOverlay {
                path: "main.zyl".into(),
                text: candidate,
                version: 2,
            }],
        );
        assert!(checked.success, "{:?}", checked.diagnostics);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unimported_new_overlay_is_parsed_and_type_checked_without_disk_writes() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-overlay-new-module-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("main.zyl"), "fn main() {}\n").unwrap();
        let new_source = "pub fn invalid() -> Int { return \"not an integer\" }\n";
        let analysis = analyze(
            &root.join("main.zyl"),
            &[SourceOverlay {
                path: "src/new.zyl".into(),
                text: new_source.into(),
                version: 7,
            }],
        );
        assert!(!analysis.success);
        assert!(analysis
            .sources
            .iter()
            .any(|source| source.path == "src/new.zyl" && source.text == new_source));
        assert!(analysis.diagnostics.iter().any(|diagnostic| {
            diagnostic["file"] == "src/new.zyl" && diagnostic["code"] == "E-TYPE-001"
        }));
        assert!(!root.join("src/new.zyl").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
