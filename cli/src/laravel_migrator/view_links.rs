//! Portable link/layout inventory and bounded public-asset copying for Blade views.
//!
//! Host filesystem paths are never emitted as application URLs. Only referenced
//! regular files under `public/` are copied, with symlink and size checks;
//! dynamic Laravel expressions remain explicit mapping work.

use super::{compiled_regex, PhpSourceMap, SourceSpan};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Component, Path},
};

const MAX_ASSET_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TOTAL_ASSET_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Default)]
pub(super) struct ViewLinkAnalyzer {
    entries: Vec<Value>,
    composition: Vec<Value>,
    views: BTreeSet<String>,
}

impl ViewLinkAnalyzer {
    pub(super) fn scan(&mut self, file: &str, source: &str) {
        if !file.starts_with("resources/views/") || !file.ends_with(".blade.php") {
            return;
        }
        self.views.insert(file.to_owned());
        let source_map = PhpSourceMap::new(source);
        let composition = compiled_regex(
            r#"@(?i:extends|include|includeIf|includeWhen|component|each|section|yield)\s*\(\s*(?:'([^']+)'|"([^"]+)")"#,
            "Blade view composition directive regex",
        );
        for capture in composition.captures_iter(source) {
            let directive = capture[0]
                .split('(')
                .next()
                .unwrap_or_default()
                .trim_start_matches('@')
                .to_ascii_lowercase();
            let target = capture
                .get(1)
                .or_else(|| capture.get(2))
                .expect("Blade directive literal");
            let (relationship, zelyra_construct) = match directive.as_str() {
                "extends" => ("extends_layout", "shared_named_view_with_layout_slots"),
                "include" | "includeif" | "includewhen" | "each" => {
                    ("includes_view_fragment", "shared_view_or_component")
                }
                "component" => ("uses_blade_component", "typed_zelyra_component"),
                "section" => ("defines_layout_section", "named_view_slot_content"),
                "yield" => ("requests_layout_section", "named_view_slot"),
                _ => ("unknown_composition", "manual_mapping"),
            };
            self.composition.push(json!({
                "file": file,
                "directive": directive,
                "target_or_slot": target.as_str(),
                "relationship": relationship,
                "zelyra_mapping": zelyra_construct,
                "source_span": local_span_json(source_map.span(source, capture.get(0).unwrap().range()))
            }));
        }
        let mut seen = BTreeSet::new();
        let attributes = compiled_regex(
            r#"(?is)\b(href|src|action|poster|data-[a-z0-9_-]*url)\s*=\s*(?:"([^"]*)"|'([^']*)')"#,
            "Blade URL attribute regex",
        );
        for capture in attributes.captures_iter(source) {
            let value_match = capture
                .get(2)
                .or_else(|| capture.get(3))
                .expect("URL attribute value");
            let value = value_match.as_str().trim();
            if value.is_empty() || value.starts_with("{{") || value.starts_with("@") {
                // The expression-specific pass below understands common helpers.
                continue;
            }
            let key = (value_match.start(), value_match.end());
            if !seen.insert(key) {
                continue;
            }
            self.entries.push(classify(
                file,
                &source_map,
                source,
                value_match.range(),
                value,
            ));
        }

        let helpers = compiled_regex(
            r#"(?is)\b(route|asset|secure_asset|url)\s*\(\s*(?:'([^']*)'|"([^"]*)")"#,
            "Blade URL helper regex",
        );
        for capture in helpers.captures_iter(source) {
            let value_match = capture
                .get(2)
                .or_else(|| capture.get(3))
                .expect("URL helper argument");
            let key = (value_match.start(), value_match.end());
            if !seen.insert(key) {
                continue;
            }
            let helper = capture[1].to_ascii_lowercase();
            let value = value_match.as_str();
            let mut item = classify(file, &source_map, source, value_match.range(), value);
            item["source_expression"] = json!(format!("{helper}('{value}')"));
            item["classification"] = match helper.as_str() {
                "route" => json!("named_route_reference"),
                "asset" | "secure_asset" => json!("project_asset_reference"),
                _ => item["classification"].clone(),
            };
            if matches!(helper.as_str(), "asset" | "secure_asset") {
                if let Some(relative) = safe_project_relative_path(value) {
                    item["project_file"] = json!(format!("public/{relative}"));
                    item["portable_url"] = json!(format!("/{relative}"));
                    item["migration_action"] =
                        json!("copy_asset_to_project_public_and_use_origin_relative_url");
                } else {
                    item["classification"] = json!("unsafe_or_dynamic_asset_path");
                    item["migration_action"] = json!("review_and_map_without_host_absolute_paths");
                }
            } else if helper == "route" {
                item["route_name"] = json!(value);
                let mut tail = source[value_match.end()..].trim_start();
                if tail.starts_with('\'') || tail.starts_with('"') {
                    tail = tail[1..].trim_start();
                }
                item["route_arguments_present"] = json!(tail.starts_with(','));
                item["migration_action"] = json!("resolve_route_name_against_static_route_graph");
            }
            self.entries.push(item);
        }
    }

    pub(super) fn finish(&self, graph_report: &Value) -> Value {
        let mut entries = self.entries.clone();
        for entry in &mut entries {
            if entry["classification"] != "named_route_reference" {
                continue;
            }
            let route_name = entry["route_name"].as_str().unwrap_or_default();
            let matches = graph_report["nodes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|node| {
                    node["kind"] == "route"
                        && node["route_context"]["route_name_complete"] == true
                        && node["route_context"]["effective_route_name"].as_str()
                            == Some(route_name)
                })
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [route] => {
                    let path = route["route_context"]["effective_path"]
                        .as_str()
                        .unwrap_or_default();
                    entry["resolved_route_path"] = json!(path);
                    let has_placeholders = path.contains('{') || path.contains('<');
                    if entry["route_arguments_present"] == false
                        && !has_placeholders
                        && safe_web_path(path)
                    {
                        entry["portable_url"] = json!(path);
                        entry["migration_action"] =
                            json!("map_to_zelyra_route_using_origin_relative_path");
                    } else {
                        entry["migration_action"] =
                            json!("map_to_zelyra_route_and_bind_dynamic_parameters_explicitly");
                    }
                }
                [] => {
                    entry["route_resolution"] = json!("unresolved");
                    entry["migration_action"] =
                        json!("resolve_route_or_mark_external_before_translation");
                }
                _ => {
                    entry["route_resolution"] = json!("ambiguous");
                    entry["migration_action"] =
                        json!("resolve_duplicate_route_names_before_translation");
                }
            }
        }
        entries.sort_by(|left, right| {
            (
                left["file"].as_str(),
                left["source_span"]["start"]["byte_offset"].as_u64(),
            )
                .cmp(&(
                    right["file"].as_str(),
                    right["source_span"]["start"]["byte_offset"].as_u64(),
                ))
        });
        let mut view_files = entries
            .iter()
            .filter_map(|entry| entry["file"].as_str())
            .collect::<BTreeSet<_>>();
        view_files.extend(
            self.composition
                .iter()
                .filter_map(|entry| entry["file"].as_str()),
        );
        let translated_link_count = entries
            .iter()
            .filter(|entry| {
                entry["classification"] == "origin_relative_url"
                    || entry["project_file"].is_string()
            })
            .count();
        let mut composition = self.composition.clone();
        composition.sort_by(|left, right| {
            (
                left["file"].as_str(),
                left["source_span"]["start"]["byte_offset"].as_u64(),
            )
                .cmp(&(
                    right["file"].as_str(),
                    right["source_span"]["start"]["byte_offset"].as_u64(),
                ))
        });
        json!({
            "schema_version": "1",
            "view_count": view_files.len(),
            "reference_count": entries.len(),
            "portable_mapping_count": translated_link_count,
            "host_absolute_paths_emitted": false,
            "composition_reference_count": composition.len(),
            "views": self.views,
            "shared_layouts": summarize_layouts(&composition),
            "view_layout_bindings": summarize_view_layout_bindings(&composition),
            "composition": composition,
            "entries": entries
        })
    }
}

/// Resolve the conventional Blade view name to a project-relative template
/// path. This is an inventory hint only: it never reads a path outside the
/// scanned views tree.
fn blade_view_path(name: &str) -> Option<String> {
    let valid = name.split('.').all(|part| {
        !part.is_empty()
            && part.chars().all(|character| {
                character.is_ascii_alphanumeric() || character == '_' || character == '-'
            })
    });
    valid.then(|| format!("resources/views/{}.blade.php", name.replace('.', "/")))
}

fn summarize_layouts(composition: &[Value]) -> Vec<Value> {
    let mut layouts = BTreeSet::new();
    for entry in composition {
        if entry["directive"] == "extends" {
            if let Some(name) = entry["target_or_slot"].as_str() {
                layouts.insert(name.to_owned());
            }
        }
    }
    layouts
        .into_iter()
        .map(|name| {
            let path = blade_view_path(&name);
            let consumers = composition
                .iter()
                .filter(|entry| entry["directive"] == "extends" && entry["target_or_slot"] == name)
                .filter_map(|entry| entry["file"].as_str())
                .collect::<BTreeSet<_>>();
            let yielded_slots = path
                .as_ref()
                .map(|path| {
                    composition
                        .iter()
                        .filter(|entry| {
                            entry["file"].as_str() == Some(path.as_str())
                                && entry["directive"] == "yield"
                        })
                        .filter_map(|entry| entry["target_or_slot"].as_str())
                        .collect::<BTreeSet<_>>()
                })
                .unwrap_or_default();
            json!({
                "blade_layout": name,
                "source_file": path,
                "zelyra_construct": "named_view_with_default_and_named_slots",
                "yielded_slots": yielded_slots,
                "consumer_count": consumers.len(),
                "consumer_views": consumers,
                "translation_status": "layout_relationship_mapped_view_templates_not_translated"
            })
        })
        .collect()
}

fn summarize_view_layout_bindings(composition: &[Value]) -> Vec<Value> {
    let mut extends_by_file = std::collections::BTreeMap::<String, BTreeSet<String>>::new();
    let mut sections_by_file = std::collections::BTreeMap::<String, BTreeSet<String>>::new();
    let mut yields_by_file = std::collections::BTreeMap::<String, BTreeSet<String>>::new();
    for entry in composition {
        let (Some(file), Some(target)) = (entry["file"].as_str(), entry["target_or_slot"].as_str())
        else {
            continue;
        };
        match entry["directive"].as_str() {
            Some("extends") => {
                extends_by_file
                    .entry(file.to_owned())
                    .or_default()
                    .insert(target.to_owned());
            }
            Some("section") => {
                sections_by_file
                    .entry(file.to_owned())
                    .or_default()
                    .insert(target.to_owned());
            }
            Some("yield") => {
                yields_by_file
                    .entry(file.to_owned())
                    .or_default()
                    .insert(target.to_owned());
            }
            _ => {}
        }
    }
    let mut bindings = Vec::new();
    for (view_file, layouts) in extends_by_file {
        for layout in layouts {
            let layout_file = blade_view_path(&layout);
            let yielded = layout_file
                .as_ref()
                .and_then(|file| yields_by_file.get(file))
                .cloned()
                .unwrap_or_default();
            let sections = sections_by_file
                .get(&view_file)
                .cloned()
                .unwrap_or_default();
            // Zelyra pages provide their body through the view's default slot.
            // Laravel conventionally calls this section `content`, but that
            // name must not turn into `<slot name="content">` in Zelyra.
            let default_content_section =
                sections.contains("content") && yielded.contains("content");
            let mapped = sections
                .intersection(&yielded)
                .filter(|section| section.as_str() != "content")
                .cloned()
                .collect::<Vec<_>>();
            let unmapped = sections.difference(&yielded).cloned().collect::<Vec<_>>();
            bindings.push(json!({
                "view_file": view_file,
                "blade_layout": layout,
                "layout_source_file": layout_file,
                "zelyra_declaration": "page ... { view: ImportedAppShell ... }",
                "section_to_default_slot": if default_content_section { Some("content") } else { None },
                "section_to_named_slot": mapped,
                "sections_without_matching_layout_yield": unmapped,
                "translation_status": "binding_plan_only_view_templates_not_translated"
            }));
        }
    }
    bindings.sort_by(|left, right| {
        (left["view_file"].as_str(), left["blade_layout"].as_str())
            .cmp(&(right["view_file"].as_str(), right["blade_layout"].as_str()))
    });
    bindings
}

pub(super) fn render_layout_mapping_markdown(report: &Value) -> String {
    let mut output = String::from(
        "# Blade layout mapping\n\nEvery listed page inherits the named Blade layout. The scaffold emits a shared Zelyra starter view as `src/imported_layouts.zyl` (`ImportedAppShell`) and imports it from `main.zyl`. Each translated Zelyra `page` uses `view: ImportedAppShell`; Laravel's conventional `@section('content')` maps to the layout's default `<slot />`, while other matching sections map to named slots. This starter preserves a shared header/content/footer frame with local Zelyra UI classes. It does not copy Blade scripts, authorization state, dynamic navigation, or CSS; those behaviors and all page bodies still require translation and review.\n\n",
    );
    let layouts = report["shared_layouts"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    if layouts.is_empty() {
        output.push_str("No literal `@extends` layout was found.\n");
        return output;
    }
    for layout in layouts {
        let name = layout["blade_layout"].as_str().unwrap_or("(unknown)");
        let source = layout["source_file"].as_str().unwrap_or("unresolved");
        output.push_str(&format!("## `{name}`\n\n- Source: `{source}`\n- Consumers: {}\n- Layout sections (`@yield`) mapped to named slots: {}\n\n", layout["consumer_count"].as_u64().unwrap_or(0), layout["yielded_slots"].as_array().map(|slots| slots.iter().filter_map(Value::as_str).map(|slot| format!("`{slot}`")).collect::<Vec<_>>().join(", ")).filter(|slots| !slots.is_empty()).unwrap_or_else(|| "none detected".to_owned())));
    }
    output.push_str("## Per-view bindings\n\nEvery listed consumer should declare `view: ImportedAppShell` on its Zelyra `page`. The `content` section uses the default page slot; the other mapped section names use named slots. A minimal page follows this shape (replace the route and translated body with the actual route and content):\n\n```zelyra\npage \"/your-route\" {\n    view: ImportedAppShell\n    html {\n        <slot name=\"title\">Seitentitel</slot>\n        <slot name=\"header_title\">Seitentitel</slot>\n        <p>Übersetzter Seiteninhalt</p>\n    }\n}\n```\n\n| Blade view | Shared layout | Default slot section | Named slots | Unmapped sections |\n|---|---|---|---|---|\n");
    for binding in report["view_layout_bindings"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let list = |key: &str| {
            binding[key]
                .as_array()
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|value| format!("`{value}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "—".to_owned())
        };
        output.push_str(&format!(
            "| `{}` | `{}` | {} | {} | {} |\n",
            binding["view_file"].as_str().unwrap_or(""),
            binding["blade_layout"].as_str().unwrap_or(""),
            binding["section_to_default_slot"]
                .as_str()
                .map(|section| format!("`{section}`"))
                .unwrap_or_else(|| "—".to_owned()),
            list("section_to_named_slot"),
            list("sections_without_matching_layout_yield")
        ));
    }
    output
}

pub(super) fn render_asset_copy_markdown(report: &Value) -> String {
    let mut output = format!(
        "# Referenced public assets\n\nCopied {} referenced asset(s) ({} bytes). {} asset(s) need review. Per-file limit: {} bytes; total limit: {} bytes.\n\n| Source-relative path | Status | Bytes | Reason |\n|---|---|---:|---|\n",
        report["copied_count"].as_u64().unwrap_or(0),
        report["copied_bytes"].as_u64().unwrap_or(0),
        report["not_copied_count"].as_u64().unwrap_or(0),
        report["per_file_limit_bytes"].as_u64().unwrap_or(0),
        report["total_limit_bytes"].as_u64().unwrap_or(0)
    );
    for asset in report["assets"].as_array().into_iter().flatten() {
        let path = asset["project_file"].as_str().unwrap_or("");
        let status = asset["status"].as_str().unwrap_or("unknown");
        let bytes = asset["bytes"]
            .as_u64()
            .map_or_else(|| "—".to_owned(), |size| size.to_string());
        let reason = asset["reason"]
            .as_str()
            .unwrap_or("—")
            .replace('|', "&#124;");
        output.push_str(&format!("| `{path}` | `{status}` | {bytes} | {reason} |\n"));
    }
    output
}

pub(super) fn render_zelyra_layout_source(report: &Value) -> Option<String> {
    let primary = report["shared_layouts"]
        .as_array()?
        .iter()
        .max_by_key(|layout| layout["consumer_count"].as_u64().unwrap_or(0))?;
    let source_layout = primary["blade_layout"].as_str()?;
    let slots = primary["yielded_slots"].as_array()?;
    let has_slot = |name: &str| slots.iter().any(|slot| slot.as_str() == Some(name));
    let mut source = format!(
        "# Zelyra starter layout mapped from Blade `{source_layout}`.\n# Review navigation, authentication state, scripts, and styling from the original layout.\npub view ImportedAppShell {{\n    html {{\n        <html lang=\"de\" data-z-ui>\n            <head>\n                <meta charset=\"utf-8\" />\n                <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n                <title><slot name=\"title\">Business Portal</slot> | Zelyra</title>\n            </head>\n            <body>\n                <div class=\"z-app-shell zelyra-app\">\n                    <aside class=\"z-app-shell__sidebar zelyra-sidebar\">\n                        <a class=\"zelyra-brand\" href=\"/\">Zelyra</a>\n                        <nav class=\"z-app-shell__navigation zelyra-nav\" aria-label=\"Hauptnavigation\">\n                            <slot name=\"navigation\"><a href=\"/\">Startseite</a></slot>\n                        </nav>\n                    </aside>\n                    <section class=\"z-app-shell__main zelyra-generated-main\">\n                        <header class=\"z-page-header\">\n                            <div><h1><slot name=\"header_title\">Business Portal</slot></h1><slot name=\"header_badge\" /></div>\n                            <div class=\"z-toolbar\"><slot name=\"header_actions\" /></div>\n                        </header>\n                        <main class=\"zelyra-page-content\"><slot /></main>\n                    </section>\n                    <footer class=\"z-app-shell__footer zelyra-footer\"><slot name=\"footer\">Erstellt mit Zelyra</slot></footer>\n                </div>\n            </body>\n        </html>\n    }}\n}}\n"
    );
    let mapped = ["title", "header_title", "header_badge", "header_actions"]
        .into_iter()
        .filter(|name| has_slot(name))
        .collect::<BTreeSet<_>>();
    if mapped.is_empty() {
        source.push_str("\n# No matching standard page sections were detected; adapt named slots from VIEW_LAYOUTS.md.\n");
    }
    Some(source)
}

/// Translate only deliberately tiny standalone notice shapes used by the
/// reference application. Unsupported Blade and HTML stay inventory; this
/// must never guess at dynamic templates or route behavior.
pub(super) fn translate_static_confirmation_views(
    source_root: &Path,
    report: &Value,
    graph: &Value,
) -> (Option<String>, Value) {
    let mut translations = Vec::new();
    let mut component_names = BTreeSet::new();
    let mut declarations = Vec::new();
    let canonical_root = fs::canonicalize(source_root).ok();
    for file in report["views"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        let Some(root) = canonical_root.as_deref() else {
            break;
        };
        let Some(source) = read_safe_blade_view(root, file) else {
            continue;
        };
        let Some((component, markup, document_title, properties)) =
            standalone_notice_component(file, &source)
        else {
            continue;
        };
        if !component_names.insert(component.clone()) {
            translations.push(json!({
                "source_file": file,
                "status": "rejected",
                "reason": "generated component name collides with another translated view"
            }));
            continue;
        }
        translations.push(json!({
            "source_file": file,
            "status": "translated_static_presentation_component",
            "component": component,
            "document_title": document_title,
            "properties": properties,
            "route_wiring": "not_generated",
            "source_route_candidates": source_route_candidates(graph, file),
            "document_title_wiring": "not_generated_without_a_generated_page",
            "removed_external_script": "https://cdn.tailwindcss.com",
            "style_mapping": "Tailwind utility classes replaced with local Zelyra UI card classes"
        }));
        declarations.push(markup);
    }
    let source = (!declarations.is_empty()).then(|| {
        format!(
            "# Standalone static Blade views use a small local Zelyra UI shell. Route and handler behavior remain separate.\npub view ImportedPublicShell {{\n    html {{\n        <html lang=\"de\" data-z-ui>\n            <head>\n                <meta charset=\"utf-8\" />\n                <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n                <title><slot name=\"title\">Status</slot></title>\n            </head>\n            <body><main class=\"z-container\"><slot /></main></body>\n        </html>\n    }}\n}}\n\n{}\n",
            declarations.join("\n\n")
        )
    });
    let translated_count = translations
        .iter()
        .filter(|entry| entry["status"] == "translated_static_presentation_component")
        .count();
    let report = json!({
        "schema_version": "2",
        "translated_count": translated_count,
        "route_wiring_generated": false,
        "standalone_layout": if translated_count > 0 { Some("ImportedPublicShell") } else { None::<&str> },
        "translations": translations
    });
    (source, report)
}

/// Preserve only route/view provenance resolved through both graph edges.
/// These entries aid review and never imply that route behavior was migrated.
fn source_route_candidates(graph: &Value, view_file: &str) -> Vec<Value> {
    let edges = graph["edges"].as_array().cloned().unwrap_or_default();
    let view_edges = edges
        .iter()
        .filter(|edge| {
            matches!(
                edge["kind"].as_str(),
                Some("controller_view" | "inline_handler_view")
            ) && edge["status"] == "resolved"
                && edge["to"]["file"] == view_file
        })
        .collect::<Vec<_>>();
    let mut candidates = Vec::new();
    for view_edge in view_edges {
        let source_action = &view_edge["from"];
        let Some(source_action_id) = source_action["id"].as_str() else {
            continue;
        };
        for route_edge in edges.iter().filter(|edge| {
            edge["kind"] == "route_controller_action"
                && edge["status"] == "resolved"
                && edge["to"]["id"] == source_action_id
        }) {
            let route = &route_edge["from"];
            let route_context = &route["route_context"];
            let route_label = route["name"].as_str().unwrap_or_default();
            let (method, declared_path) = route_label
                .split_once(' ')
                .map(|(method, path)| (method.to_ascii_uppercase(), path))
                .unwrap_or_default();
            let path = route_context["effective_path"]
                .as_str()
                .unwrap_or(declared_path);
            candidates.push(json!({
                "route": route_label,
                "method": method,
                "path": path,
                "route_name": route_context["effective_route_name"],
                "inherited_middleware": route_context["inherited_middleware"],
                "route_middleware": route_context["route_middleware"],
                "route_middleware_seen": route_context["route_middleware_seen"],
                "middleware_complete": route_context["middleware_complete"],
                "route_middleware_complete": route_context["route_middleware_complete"],
                "middleware_groups": route_context["groups"],
                "handler": source_action["name"],
                "handler_file": source_action["file"],
                "handler_source_span": source_action["source_span"],
                "route_source_file": route["file"],
                "route_source_span": route["source_span"],
                "resolution": "static_source_link_only_handler_behavior_not_translated"
            }));
        }
    }
    candidates.sort_by(|left, right| {
        (
            left["route_source_file"].as_str(),
            left["route_source_span"]["start"]["byte_offset"].as_u64(),
            left["handler_file"].as_str(),
            left["handler_source_span"]["start"]["byte_offset"].as_u64(),
        )
            .cmp(&(
                right["route_source_file"].as_str(),
                right["route_source_span"]["start"]["byte_offset"].as_u64(),
                right["handler_file"].as_str(),
                right["handler_source_span"]["start"]["byte_offset"].as_u64(),
            ))
    });
    candidates.dedup_by(|left, right| {
        left["route_source_file"] == right["route_source_file"]
            && left["route_source_span"] == right["route_source_span"]
            && left["handler_file"] == right["handler_file"]
            && left["handler_source_span"] == right["handler_source_span"]
    });
    candidates
}

fn read_safe_blade_view(canonical_root: &Path, relative_file: &str) -> Option<String> {
    let relative = Path::new(relative_file);
    if relative
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
        || !relative_file.starts_with("resources/views/")
    {
        return None;
    }
    let mut source_path = canonical_root.to_path_buf();
    for part in relative.components() {
        let Component::Normal(part) = part else {
            return None;
        };
        source_path.push(part);
        if fs::symlink_metadata(&source_path)
            .ok()?
            .file_type()
            .is_symlink()
        {
            return None;
        }
    }
    let metadata = fs::symlink_metadata(&source_path).ok()?;
    if !metadata.is_file() || metadata.len() > 1024 * 1024 {
        return None;
    }
    let canonical_path = fs::canonicalize(&source_path).ok()?;
    let view_root = canonical_root.join("resources/views");
    if !fs::symlink_metadata(&view_root).ok()?.is_dir() {
        return None;
    }
    let canonical_view_root = fs::canonicalize(view_root).ok()?;
    if !canonical_path.starts_with(&canonical_view_root) {
        return None;
    }
    fs::read_to_string(canonical_path).ok()
}

fn standalone_notice_component(
    file: &str,
    source: &str,
) -> Option<(String, String, String, Vec<String>)> {
    if let Some((component, markup, title)) = static_confirmation_component(file, source) {
        return Some((component, markup, title, Vec::new()));
    }
    dynamic_module_locked_component(file, source)
}

fn static_confirmation_component(file: &str, source: &str) -> Option<(String, String, String)> {
    if source.contains("{{")
        || source.contains("{!!")
        || source.contains("<?")
        || source.contains('@')
        || source.matches("https://cdn.tailwindcss.com").count() != 1
    {
        return None;
    }
    let pattern = compiled_regex(
        r#"(?is)^\s*<!doctype html>\s*<html lang="de">\s*<head>\s*<meta charset="utf-8">\s*<meta name="viewport" content="width=device-width, initial-scale=1">\s*<title>([^<>]*)</title>\s*<script src="https://cdn\.tailwindcss\.com"></script>\s*</head>\s*<body class="[^"]*">\s*<main class="[^"]*">\s*<h1 class="[^"]*">([^<>]*)</h1>\s*<p class="[^"]*">([^<>]*)</p>\s*</main>\s*</body>\s*</html>\s*$"#,
        "static standalone confirmation view pattern",
    );
    let captures = pattern.captures(source)?;
    if captures.len() != 4 {
        return None;
    }
    let title = captures.get(1)?.as_str().trim();
    let heading = captures.get(2)?.as_str().trim();
    let paragraph = captures.get(3)?.as_str().trim();
    if [title, heading, paragraph].iter().any(|text| {
        text.is_empty() || text.contains('&') || text.contains('{') || text.contains('}')
    }) {
        return None;
    }
    let component = blade_component_name(file)?;
    let markup = format!(
        "pub component {component} {{\n    html {{\n        <section class=\"z-card\">\n            <header class=\"z-card__header\"><h1 class=\"z-card__title\">{}</h1></header>\n            <div class=\"z-card__body\"><p>{}</p></div>\n        </section>\n    }}\n}}",
        escape_static_html_text(heading),
        escape_static_html_text(paragraph)
    );
    Some((component, markup, title.to_owned()))
}

/// Preserve the one escaped Laravel interpolation in the reference project's
/// public module notice as a typed String property. Raw Blade output and every
/// other dynamic expression remain unsupported.
fn dynamic_module_locked_component(
    file: &str,
    source: &str,
) -> Option<(String, String, String, Vec<String>)> {
    if source.matches("{{").count() != 1
        || source.contains("{!!")
        || source.contains("<?")
        || source.contains('@')
        || source.matches("https://cdn.tailwindcss.com").count() != 1
    {
        return None;
    }
    let pattern = compiled_regex(
        r#"(?is)^\s*<!doctype html>\s*<html lang="de">\s*<head>\s*<meta charset="utf-8">\s*<meta name="viewport" content="width=device-width, initial-scale=1">\s*<title>([^<>]*)</title>\s*<script src="https://cdn\.tailwindcss\.com"></script>\s*</head>\s*<body class="[^"]*">\s*<main class="[^"]*">\s*<p class="[^"]*">([^<>]*)</p>\s*<h1 class="[^"]*">\s*\{\{\s*\$module\s*\}\}([^<>]*)</h1>\s*<p class="[^"]*">([^<>]*)</p>\s*</main>\s*</body>\s*</html>\s*$"#,
        "escaped standalone module notice pattern",
    );
    let captures = pattern.captures(source)?;
    if captures.len() != 5 {
        return None;
    }
    let title = captures.get(1)?.as_str().trim();
    let brand = captures.get(2)?.as_str().trim();
    let suffix = captures.get(3)?.as_str().trim();
    let paragraph = captures.get(4)?.as_str().trim();
    if [title, brand, suffix, paragraph].iter().any(|text| {
        text.is_empty() || text.contains('&') || text.contains('{') || text.contains('}')
    }) {
        return None;
    }
    let component = blade_component_name(file)?;
    let markup = format!(
        "pub component {component} {{\n    props {{ module: String }}\n    html {{\n        <section class=\"z-card\">\n            <header class=\"z-card__header\"><p class=\"z-badge z-badge--info\">{}</p><h1 class=\"z-card__title\">{{module}} {}</h1></header>\n            <div class=\"z-card__body\"><p>{}</p></div>\n        </section>\n    }}\n}}",
        escape_static_html_text(brand),
        escape_static_html_text(suffix),
        escape_static_html_text(paragraph)
    );
    Some((
        component,
        markup,
        title.to_owned(),
        vec!["module: String".to_owned()],
    ))
}

fn blade_component_name(file: &str) -> Option<String> {
    let relative = file
        .strip_prefix("resources/views/")?
        .strip_suffix(".blade.php")?;
    let mut name = String::new();
    for path_part in relative.split('/') {
        for part in path_part
            .split(['_', '-', '.'])
            .filter(|part| !part.is_empty())
        {
            let mut chars = part.chars();
            name.push(chars.next()?.to_ascii_uppercase());
            for character in chars {
                if !character.is_ascii_alphanumeric() {
                    return None;
                }
                name.push(character);
            }
        }
    }
    (!name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric()))
    .then_some(name)
}

/// Return a translated component only when the source view passed the strict
/// presentation translator. This lets route generators reuse the exact same
/// component rather than reconstructing its HTML independently.
pub(super) fn translated_component_name(report: &Value, source_file: &str) -> Option<String> {
    report["translations"]
        .as_array()?
        .iter()
        .find(|translation| {
            translation["source_file"].as_str() == Some(source_file)
                && translation["status"] == "translated_static_presentation_component"
        })?
        .get("component")?
        .as_str()
        .map(str::to_owned)
}

fn escape_static_html_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub(super) fn render_view_translation_markdown(report: &Value) -> String {
    let mut output = String::from(
        "# Bounded Blade presentation translations\n\nOnly two strict standalone notice shapes are translated: a fully static confirmation and a public module notice with exactly one escaped `{{ $module }}` interpolation mapped to a typed `String` property. Other dynamic Blade templates, handlers, routes, authorization, and persistence are not generated by this pass. Components use local Zelyra UI classes. The `route_wiring` column is populated only when a separately checked generated workflow explicitly reuses that translated component. Source route candidates below are static graph references and do not by themselves imply that handler behavior was migrated.\n\n| Source view | Status | Component | Source route candidates | Route wiring |\n|---|---|---|---:|---|\n",
    );
    for translation in report["translations"].as_array().into_iter().flatten() {
        output.push_str(&format!(
            "| `{}` | `{}` | {} | {} | `{}` |\n",
            translation["source_file"].as_str().unwrap_or(""),
            translation["status"].as_str().unwrap_or(""),
            translation["component"]
                .as_str()
                .map(|component| format!("`{component}`"))
                .unwrap_or_else(|| "—".to_owned()),
            translation["source_route_candidates"]
                .as_array()
                .map_or(0, Vec::len),
            translation["route_wiring"]
                .as_str()
                .unwrap_or("not generated")
        ));
    }
    if let Some(translation) = report["translations"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|translation| translation["status"] == "translated_static_presentation_component")
    {
        let component = translation["component"]
            .as_str()
            .unwrap_or("TranslatedView");
        let title = translation["document_title"].as_str().unwrap_or("Status");
        output.push_str(&format!(
            "\n## Wiring a translated presentation\n\nThe generator provides `pub view ImportedPublicShell` for these standalone pages. The static confirmation component takes no properties; the module notice takes `module: String`. Once route and handler behavior are migrated, a page can use the relevant component like this:\n\n```zelyra\npage \"/reviewed-route\" {{\n    view: ImportedPublicShell\n    html {{\n        <slot name=\"title\">{}</slot>\n        <{} />\n    }}\n}}\n```\n\nFor `PublicModuleLocked`, pass the required property as `<PublicModuleLocked module=\"{{module_name}}\" />`. The route above is an example placeholder and is not generated from the Laravel route graph.\n",
            escape_static_html_text(title), component
        ));
    }
    output
}

/// Copy only referenced, regular files beneath the Laravel public directory.
/// Source paths stay project-relative in reports; symlinks and oversized files
/// are recorded as unresolved rather than followed or copied.
pub(super) fn copy_referenced_assets(
    source_root: &Path,
    output_root: &Path,
    report: &mut Value,
) -> Value {
    let public_root = source_root.join("public");
    let public_metadata = fs::symlink_metadata(&public_root);
    let public_directory = match public_metadata {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::canonicalize(&public_root).ok()
        }
        _ => None,
    };
    let mut assets = Vec::new();
    let mut unique_paths = report["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry["project_file"].as_str())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let mut copied_bytes = 0_u64;
    let mut copied_paths = BTreeSet::new();
    for project_file in std::mem::take(&mut unique_paths) {
        let status = copy_one_asset(
            &public_root,
            public_directory.as_deref(),
            output_root,
            &project_file,
            &mut copied_bytes,
        );
        if status["status"] == "copied" {
            copied_paths.insert(project_file.clone());
        }
        assets.push(status);
    }
    if let Some(entries) = report["entries"].as_array_mut() {
        for entry in entries {
            let Some(path) = entry["project_file"].as_str().map(str::to_owned) else {
                continue;
            };
            entry["asset_copy_status"] = json!(if copied_paths.contains(&path) {
                "copied"
            } else {
                "not_copied_review_asset_copy"
            });
        }
    }
    json!({
        "copied_count": assets.iter().filter(|asset| asset["status"] == "copied").count(),
        "not_copied_count": assets.iter().filter(|asset| asset["status"] != "copied").count(),
        "copied_bytes": copied_bytes,
        "per_file_limit_bytes": MAX_ASSET_BYTES,
        "total_limit_bytes": MAX_TOTAL_ASSET_BYTES,
        "assets": assets
    })
}

fn copy_one_asset(
    public_root: &Path,
    canonical_public_root: Option<&Path>,
    output_root: &Path,
    project_file: &str,
    total_bytes: &mut u64,
) -> Value {
    let Some(relative) = project_file.strip_prefix("public/") else {
        return json!({"project_file": project_file, "status": "rejected", "reason": "asset path is outside public/"});
    };
    let Some(relative) = safe_project_relative_path(relative) else {
        return json!({"project_file": project_file, "status": "rejected", "reason": "asset path is not a safe project-relative path"});
    };
    let Some(canonical_public_root) = canonical_public_root else {
        return json!({"project_file": project_file, "status": "not_found", "reason": "public/ directory is missing, not a directory, or a symlink"});
    };
    let mut source_path = public_root.to_path_buf();
    for component in Path::new(&relative).components() {
        let Component::Normal(segment) = component else {
            return json!({"project_file": project_file, "status": "rejected", "reason": "asset path contains a non-normal path component"});
        };
        source_path.push(segment);
        match fs::symlink_metadata(&source_path) {
            Ok(metadata) if !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return json!({"project_file": project_file, "status": "rejected", "reason": "asset source contains a symlink"});
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return json!({"project_file": project_file, "status": "not_found", "reason": "referenced public asset does not exist"});
            }
            Err(_) => {
                return json!({"project_file": project_file, "status": "unreadable", "reason": "asset metadata could not be read"});
            }
        }
    }
    let Ok(canonical_source) = fs::canonicalize(&source_path) else {
        return json!({"project_file": project_file, "status": "unreadable", "reason": "asset source could not be canonicalized"});
    };
    if !canonical_source.starts_with(canonical_public_root) {
        return json!({"project_file": project_file, "status": "rejected", "reason": "asset source resolves outside public/"});
    }
    let Ok(metadata) = fs::metadata(&canonical_source) else {
        return json!({"project_file": project_file, "status": "unreadable", "reason": "asset metadata could not be read"});
    };
    if !metadata.is_file() {
        return json!({"project_file": project_file, "status": "rejected", "reason": "asset source is not a regular file"});
    }
    let size = metadata.len();
    if size > MAX_ASSET_BYTES || total_bytes.saturating_add(size) > MAX_TOTAL_ASSET_BYTES {
        return json!({"project_file": project_file, "status": "not_copied_size_limit", "bytes": size, "reason": "asset copy size limit exceeded"});
    }
    let target_path = output_root.join("public").join(&relative);
    let Some(parent) = target_path.parent() else {
        return json!({"project_file": project_file, "status": "rejected", "reason": "asset output path has no parent"});
    };
    if fs::create_dir_all(parent).is_err() {
        return json!({"project_file": project_file, "status": "write_error", "reason": "asset output directory could not be created"});
    }
    let Ok(source_file) = fs::File::open(&canonical_source) else {
        return json!({"project_file": project_file, "status": "unreadable", "reason": "asset source could not be read"});
    };
    let mut bytes = Vec::with_capacity(size as usize);
    if source_file
        .take(MAX_ASSET_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .is_err()
    {
        return json!({"project_file": project_file, "status": "unreadable", "reason": "asset source could not be read"});
    }
    if bytes.len() as u64 > MAX_ASSET_BYTES {
        return json!({"project_file": project_file, "status": "not_copied_size_limit", "reason": "asset exceeded the per-file limit while being copied"});
    }
    if bytes.len() as u64 != size {
        return json!({"project_file": project_file, "status": "changed_during_copy", "reason": "asset changed while it was being copied"});
    }
    let mut target = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target_path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return json!({"project_file": project_file, "status": "write_error", "reason": "asset output already exists"});
        }
        Err(_) => {
            return json!({"project_file": project_file, "status": "write_error", "reason": "asset output file could not be created"});
        }
    };
    if target.write_all(&bytes).is_err() {
        let _ = fs::remove_file(&target_path);
        return json!({"project_file": project_file, "status": "write_error", "reason": "asset bytes could not be written"});
    }
    *total_bytes += size;
    json!({"project_file": project_file, "status": "copied", "bytes": size})
}

fn classify(
    file: &str,
    source_map: &PhpSourceMap,
    source: &str,
    range: std::ops::Range<usize>,
    value: &str,
) -> Value {
    let trimmed = value.trim();
    let (classification, action) = if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("mailto:")
        || trimmed.starts_with("tel:")
        || trimmed.starts_with("data:")
    {
        (
            "external_or_protocol_url",
            "preserve_only_after_external_link_review",
        )
    } else if trimmed.starts_with('#') {
        ("same_document_fragment", "preserve_fragment")
    } else if looks_like_host_path(trimmed) {
        (
            "host_absolute_filesystem_path",
            "reject_and_replace_with_project_relative_asset_or_route",
        )
    } else if trimmed.starts_with('/') {
        if safe_web_path(trimmed) {
            (
                "origin_relative_url",
                "preserve_as_origin_relative_route_or_asset",
            )
        } else {
            ("unsafe_web_path", "normalize_or_reject_path_traversal")
        }
    } else if trimmed.contains("{{") || trimmed.contains("}}") || trimmed.contains('$') {
        (
            "dynamic_url",
            "resolve_expression_and_verify_route_or_project_asset",
        )
    } else if safe_project_relative_path(trimmed).is_some() {
        (
            "relative_url_or_asset",
            "resolve_against_view_or_public_root_and_keep_project_relative",
        )
    } else if trimmed.starts_with("../") || trimmed.split('/').any(|part| part == "..") {
        (
            "unsafe_relative_path",
            "resolve_inside_project_and_remove_parent_traversal",
        )
    } else {
        ("unresolved_url", "review_before_translation")
    };
    json!({
        "file": file,
        "source_value": trimmed,
        "source_span": local_span_json(source_map.span(source, range)),
        "classification": classification,
        "migration_action": action,
        "host_absolute_path_allowed": false
    })
}

fn looks_like_host_path(value: &str) -> bool {
    value.starts_with("/var/")
        || value.starts_with("/home/")
        || value.starts_with("/opt/")
        || value.starts_with("/tmp/")
        || value.starts_with("C:\\")
        || value.starts_with("C:/")
        || value.starts_with("\\\\")
}

fn safe_web_path(value: &str) -> bool {
    !value.contains('\\')
        && !value
            .split(['?', '#'])
            .next()
            .unwrap_or(value)
            .split('/')
            .any(|segment| segment == "..")
}

fn safe_project_relative_path(value: &str) -> Option<String> {
    if value.is_empty()
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.contains('\\')
        || value.contains(':')
        || value
            .chars()
            .any(|character| matches!(character, '?' | '#' | '{' | '}' | '$'))
    {
        return None;
    }
    let mut parts = Vec::new();
    for part in value.split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            _ => parts.push(part),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

fn local_span_json(span: SourceSpan) -> Value {
    json!({
        "start": {"byte_offset": span.start_byte, "line": span.start_line, "column": span.start_column},
        "end": {"byte_offset": span.end_byte, "line": span.end_line, "column": span.end_column},
        "end_exclusive": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_asset_fixture(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "zelyra-view-assets-{}-{name}-{nonce}",
            std::process::id()
        ));
        let output = root.join("output");
        fs::create_dir_all(root.join("public/images")).unwrap();
        fs::create_dir_all(&output).unwrap();
        (root, output)
    }

    #[test]
    fn translates_only_the_static_reference_confirmation_view() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-static-view-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let view_file = "resources/views/public/csv_mailer_unsubscribed.blade.php";
        let view_path = root.join(view_file);
        fs::create_dir_all(view_path.parent().unwrap()).unwrap();
        let source = r#"<!doctype html><html lang="de"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Abmeldung gespeichert</title><script src="https://cdn.tailwindcss.com"></script></head><body class="min-h-screen bg-slate-50 flex items-center justify-center p-6"><main class="max-w-lg rounded-2xl bg-white p-8 shadow"><h1 class="text-2xl font-bold text-slate-900">Abmeldung gespeichert</h1><p class="mt-3 text-slate-600">Diese E-Mail-Adresse erhält über diesen Verteiler keine weiteren Nachrichten.</p></main></body></html>"#;
        fs::write(&view_path, source).unwrap();
        let (module, report) =
            translate_static_confirmation_views(&root, &json!({"views": [view_file]}), &json!({}));
        let module = module.unwrap();
        assert!(module.contains("pub view ImportedPublicShell"));
        assert!(module.contains("pub component PublicCsvMailerUnsubscribed"));
        assert!(module.contains("class=\"z-card\""));
        assert!(module.contains(
            "Diese E-Mail-Adresse erhält über diesen Verteiler keine weiteren Nachrichten."
        ));
        assert!(!module.contains("tailwindcss"));
        assert_eq!(report["translated_count"], 1);
        assert_eq!(report["standalone_layout"], "ImportedPublicShell");
        assert_eq!(report["translations"][0]["route_wiring"], "not_generated");

        let dynamic_source = source.replace("Abmeldung gespeichert</h1>", "{{ $message }}</h1>");
        fs::write(view_path, dynamic_source).unwrap();
        let (module, report) =
            translate_static_confirmation_views(&root, &json!({"views": [view_file]}), &json!({}));
        assert!(module.is_none());
        assert_eq!(report["translated_count"], 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn translated_component_lookup_requires_an_exact_source_and_success_status() {
        let report = json!({"translations": [
            {
                "source_file": "resources/views/public/done.blade.php",
                "status": "translated_static_presentation_component",
                "component": "PublicDone"
            },
            {
                "source_file": "resources/views/public/unreviewed.blade.php",
                "status": "manual_review_required",
                "component": "Unreviewed"
            }
        ]});
        assert_eq!(
            translated_component_name(&report, "resources/views/public/done.blade.php").as_deref(),
            Some("PublicDone")
        );
        assert_eq!(
            translated_component_name(&report, "resources/views/public/unreviewed.blade.php"),
            None
        );
        assert_eq!(
            translated_component_name(&report, "resources/views/public/other.blade.php"),
            None
        );
    }

    #[test]
    fn translates_only_the_escaped_module_notice_interpolation() {
        let view_file = "resources/views/public/module-locked.blade.php";
        let source = r#"<!doctype html>
<html lang="de">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Modul nicht freigeschaltet | Siedelmann Business Suite</title>
    <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="min-h-screen bg-slate-100 px-5 py-12 text-slate-800">
    <main class="mx-auto max-w-xl rounded-2xl border border-slate-200 bg-white p-8 shadow-sm">
        <p class="text-xs font-bold uppercase tracking-widest text-teal-700">Siedelmann Business Suite</p>
        <h1 class="mt-3 text-2xl font-bold">{{ $module }} ist derzeit nicht verfügbar</h1>
        <p class="mt-3 leading-7 text-slate-600">Der zugehörige Unternehmensbereich hat dieses Modul momentan nicht freigeschaltet. Bitte wenden Sie sich direkt an das Unternehmen, das Ihnen diesen Link gesendet hat.</p>
    </main>
</body>
</html>"#;
        let (component, markup, title, properties) =
            dynamic_module_locked_component(view_file, source).unwrap();
        assert_eq!(component, "PublicModuleLocked");
        assert_eq!(
            title,
            "Modul nicht freigeschaltet | Siedelmann Business Suite"
        );
        assert_eq!(properties, ["module: String"]);
        assert!(markup.contains("props { module: String }"));
        assert!(markup
            .contains("<h1 class=\"z-card__title\">{module} ist derzeit nicht verfügbar</h1>"));
        assert!(!markup.contains("{{"));
        assert!(!markup.contains("tailwindcss"));

        let changed = source.replace("{{ $module }}", "{!! $module !!}");
        assert!(dynamic_module_locked_component(view_file, &changed).is_none());
        let extra_expression = source.replace(
            "Der zugehörige Unternehmensbereich",
            "{{ $user }} Der zugehörige Unternehmensbereich",
        );
        assert!(dynamic_module_locked_component(view_file, &extra_expression).is_none());
    }

    #[test]
    fn blade_links_are_classified_as_portable_routes_assets_and_files() {
        let source = r#"<a href="/orders">Orders</a><a href="../secret">bad</a><a href="{{ route('orders.show', $order) }}">show</a><a href="{{ route('orders.index') }}">index</a><img src="{{ asset('images/logo.svg') }}"><a href="/opt/app/storage/logo.png">bad host path</a>"#;
        let mut analyzer = ViewLinkAnalyzer::default();
        analyzer.scan("resources/views/orders/index.blade.php", source);
        let graph = json!({"nodes": [
            {
                "kind": "route",
                "route_context": {
                    "route_name_complete": true,
                    "effective_route_name": "orders.index",
                    "effective_path": "/orders"
                }
            },
            {
                "kind": "route",
                "route_context": {
                    "route_name_complete": true,
                    "effective_route_name": "orders.show",
                    "effective_path": "/orders/{order}"
                }
            }
        ]});
        let report = analyzer.finish(&graph);
        let entries = report["entries"].as_array().unwrap();
        assert!(entries
            .iter()
            .any(|entry| entry["classification"] == "origin_relative_url"
                && entry["source_value"] == "/orders"));
        assert!(entries
            .iter()
            .any(|entry| entry["classification"] == "unsafe_relative_path"
                || entry["classification"] == "unsafe_web_path"));
        assert!(entries
            .iter()
            .any(|entry| entry["classification"] == "named_route_reference"
                && entry["route_name"] == "orders.show"));
        assert!(entries
            .iter()
            .any(|entry| entry["route_name"] == "orders.index"
                && entry["resolved_route_path"] == "/orders"
                && entry["portable_url"] == "/orders"));
        assert!(entries
            .iter()
            .any(|entry| entry["route_name"] == "orders.show"
                && entry["resolved_route_path"] == "/orders/{order}"
                && entry["route_arguments_present"] == true
                && entry["portable_url"].is_null()));
        assert!(entries
            .iter()
            .any(|entry| entry["project_file"] == "public/images/logo.svg"
                && entry["portable_url"] == "/images/logo.svg"));
        assert!(entries
            .iter()
            .any(|entry| entry["classification"] == "host_absolute_filesystem_path"));
        assert_eq!(report["host_absolute_paths_emitted"], false);
    }

    #[test]
    fn project_asset_paths_reject_traversal_and_accept_nested_relative_paths() {
        assert_eq!(
            safe_project_relative_path("images/logo.svg").as_deref(),
            Some("images/logo.svg")
        );
        assert_eq!(safe_project_relative_path("../../etc/passwd"), None);
        assert_eq!(
            safe_project_relative_path("/opt/project/public/a.png"),
            None
        );
    }

    #[test]
    fn copies_only_referenced_public_files_and_records_missing_assets() {
        let (root, output) = temp_asset_fixture("copy");
        fs::write(root.join("public/images/logo.svg"), b"<svg/>").unwrap();
        let mut report = json!({"entries": [
            {"project_file":"public/images/logo.svg"},
            {"project_file":"public/images/missing.png"}
        ]});
        let copied = copy_referenced_assets(&root, &output, &mut report);
        assert_eq!(copied["copied_count"], 1);
        assert_eq!(copied["not_copied_count"], 1);
        assert_eq!(copied["copied_bytes"], 6);
        assert_eq!(
            fs::read(output.join("public/images/logo.svg")).unwrap(),
            b"<svg/>"
        );
        assert_eq!(report["entries"][0]["asset_copy_status"], "copied");
        assert_eq!(
            report["entries"][1]["asset_copy_status"],
            "not_copied_review_asset_copy"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_public_asset_sources() {
        use std::os::unix::fs::symlink;

        let (root, output) = temp_asset_fixture("symlink");
        let outside = root.join("private.txt");
        fs::write(&outside, b"private").unwrap();
        symlink(outside, root.join("public/images/link.txt")).unwrap();
        let mut report = json!({"entries": [{"project_file":"public/images/link.txt"}]});
        let copied = copy_referenced_assets(&root, &output, &mut report);
        assert_eq!(copied["copied_count"], 0);
        assert_eq!(copied["assets"][0]["status"], "rejected");
        assert_eq!(
            copied["assets"][0]["reason"],
            "asset source contains a symlink"
        );
        assert!(!output.join("public/images/link.txt").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn blade_layout_and_fragment_references_are_recorded_as_shared_zelyra_composition() {
        let view = "resources/views/orders/index.blade.php";
        let source = "@extends('layouts.app')\n@section('content') @include('shared.nav')";
        let mut analyzer = ViewLinkAnalyzer::default();
        analyzer.scan(view, source);
        analyzer.scan(
            "resources/views/layouts/app.blade.php",
            "@yield('title') @yield('content') @yield('footer')",
        );
        let composition = analyzer.finish(&json!({"nodes": []}))["composition"]
            .as_array()
            .unwrap()
            .clone();
        assert!(composition
            .iter()
            .any(|entry| entry["relationship"] == "extends_layout"
                && entry["target_or_slot"] == "layouts.app"
                && entry["zelyra_mapping"] == "shared_named_view_with_layout_slots"));
        assert!(composition
            .iter()
            .any(|entry| entry["relationship"] == "includes_view_fragment"
                && entry["target_or_slot"] == "shared.nav"));
        assert!(composition
            .iter()
            .any(|entry| entry["relationship"] == "defines_layout_section"
                && entry["target_or_slot"] == "content"));
        assert!(composition.iter().any(|entry| {
            entry["file"] == "resources/views/layouts/app.blade.php"
                && entry["relationship"] == "requests_layout_section"
                && entry["target_or_slot"] == "footer"
        }));
        let report = analyzer.finish(&json!({"nodes": []}));
        assert_eq!(report["view_count"], 2);
        assert_eq!(report["shared_layouts"][0]["blade_layout"], "layouts.app");
        assert_eq!(
            report["shared_layouts"][0]["yielded_slots"],
            json!(["content", "footer", "title"])
        );
        assert_eq!(report["view_layout_bindings"][0]["view_file"], view);
        assert_eq!(
            report["view_layout_bindings"][0]["section_to_default_slot"],
            json!("content")
        );
        assert_eq!(
            report["view_layout_bindings"][0]["section_to_named_slot"],
            json!([])
        );
        let markdown = render_layout_mapping_markdown(&report);
        assert!(markdown.contains("default `<slot />`"));
        assert!(markdown.contains("`content`"));
    }
}
