//! Narrow, source-evidenced route workflow translations.
//!
//! This module deliberately recognizes only the complete public token
//! unsubscribe pattern. Unsupported controllers remain analysis findings.

use super::{compiled_regex, is_safe_table_identifier, PhpSourceMap};
use serde_json::{json, Value};
use std::{fs, path::Path};

pub(super) struct GeneratedWorkflow {
    pub(super) source: String,
    pub(super) report: Value,
}

pub(super) fn grant_schema_access(source: &str, table: &str, module: &str) -> Option<String> {
    if !is_safe_table_identifier(table)
        || !module
            .bytes()
            .all(|byte| is_ident_byte(byte) || byte == b'/' || byte == b'.')
    {
        return None;
    }
    let pattern = format!(r"(?s)(table\s+{}\s*\{{.*?)(\n\}})", regex::escape(table));
    let regex = regex::Regex::new(&pattern).ok()?;
    if regex
        .captures(source)
        .is_some_and(|captures| captures[1].contains("access {"))
    {
        return None;
    }
    let updated = regex.replace(source, |captures: &regex::Captures<'_>| {
        let declaration = &captures[1];
        format!(
            "{}\n    access {{ read_write: [{:?}] }}{}",
            declaration, module, &captures[2]
        )
    });
    (updated.as_ref() != source).then(|| updated.into_owned())
}

pub(super) fn generate(
    root: &Path,
    route_candidates: &Value,
    schema_candidates: &[Value],
) -> Option<GeneratedWorkflow> {
    let routes = route_candidates["model_consumer_routes"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(route_candidates["routes"].as_array().into_iter().flatten())
        .collect::<Vec<_>>();
    let hash_column = compiled_regex(
        r"(?m)^\s*token_hash\s*:\s*String\(64\)",
        "unsubscribe token hash schema column pattern",
    );
    let timestamp_column = compiled_regex(
        r"(?m)^\s*unsubscribed_at\s*:\s*Timestamp\?",
        "unsubscribe timestamp schema column pattern",
    );

    for get in &routes {
        let route = get["route"].as_str()?;
        let (method, path) = route.split_once(' ')?;
        if !method.eq_ignore_ascii_case("get") {
            continue;
        }
        let action = get["controller_action"].as_str()?;
        let (controller, method_name) = action.split_once('@')?;
        if method_name != "show" {
            continue;
        }
        let table = get["table"].as_str()?;
        if !is_safe_table_identifier(table) {
            continue;
        }
        let schema = schema_candidates
            .iter()
            .find(|candidate| candidate["table"].as_str() == Some(table))?
            .get("candidate_source")?
            .as_str()?;
        let has_hash_column = hash_column.is_match(schema);
        let has_timestamp_column = timestamp_column.is_match(schema);
        if !has_hash_column || !has_timestamp_column {
            continue;
        }
        let token_name = match path
            .strip_suffix('}')
            .and_then(|path| path.rsplit_once("/{"))
        {
            Some((_, name)) if !name.is_empty() && name.bytes().all(is_ident_byte) => name,
            _ => continue,
        };
        let controller_file = get["controller_source_file"].as_str()?;
        if routes.iter().any(|post| {
            post["route"]
                .as_str()
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(&format!("post {path}")))
                && post["table"].as_str() == Some(table)
                && post["controller_action"].as_str().is_some_and(|candidate| {
                    candidate.starts_with(&format!("{controller}@unsubscribe"))
                })
                && post["controller_source_file"].as_str() == Some(controller_file)
        }) {
            let source_path = match safe_source_path(root, controller_file) {
                Some(path) => path,
                None => continue,
            };
            let Ok(source) = fs::read_to_string(&source_path) else {
                continue;
            };
            if !recognized_controller(&source, token_name) {
                continue;
            }
            let Some((confirm_view, success_view)) = controller_views(&source) else {
                continue;
            };
            let Some(confirm_content) = static_notice(root, &confirm_view) else {
                continue;
            };
            let Some(success_content) = static_notice(root, &success_view) else {
                continue;
            };
            let form_name = "ImportedPublicUnsubscribe";
            let success_path = unique_success_path(path, token_name);
            let generated = format!(
                "import \"src/imported_schema.zyl\" as imported_schema\n\n\
                 // Generated from a statically verified Laravel token-unsubscribe workflow. Review the source mapping report before deployment.\n\
                 form {form_name} -> {table} {{\n\
                     route: {path_literal}\n\
                     verify sql<Int[]> {{\n\
                         SELECT id FROM {table} WHERE token_hash = SHA2(:{token_name}, 256)\n\
                     }}\n\
                     action unsubscribe {{\n\
                         require_match\n\
                         label: {submit_literal}\n\
                         confirm_page {{\n\
                             title: {confirm_title}\n\
                             message: {confirm_message}\n\
                             submit: {submit_literal}\n\
                         }}\n\
                         success {success_message}\n\
                         redirect {success_path_literal}\n\
                         sql<Int[]> {{\n\
                             SELECT id FROM {table} WHERE token_hash = SHA2(:{token_name}, 256)\n\
                         }}\n\
                         sql {{\n\
                             UPDATE {table} SET unsubscribed_at = COALESCE(unsubscribed_at, CURRENT_TIMESTAMP) WHERE token_hash = SHA2(:{token_name}, 256)\n\
                         }}\n\
                     }}\n\
                 }}\n\
                 page {success_path_literal} {{\n\
                     html {{\n\
                         <main class=\"z-container\"><h1>{success_heading_html}</h1><p>{success_body_html}</p></main>\n\
                     }}\n\
                 }}\n",
                path_literal = zelyra_string(path),
                submit_literal = zelyra_string(&confirm_content.button),
                confirm_title = zelyra_string(&confirm_content.heading),
                confirm_message = zelyra_string(&confirm_content.body),
                success_message = zelyra_string(&success_content.body),
                success_path_literal = zelyra_string(&success_path),
                success_heading_html = html_escape(&success_content.heading),
                success_body_html = html_escape(&success_content.body),
            );
            let report = json!({
                "status": "generated_checked_token_unsubscribe_module",
                "module": "src/imported_public_unsubscribe.zyl",
                "source_controller": controller_file,
                "source_table": table,
                "source_route": path,
                "route_parameter": token_name,
                "verification": "sha256-token-hash-read-before-render-and-write",
                "write_semantics": "idempotent-unsubscribed-at-coalesce-current-timestamp",
                "csrf": "native-zelyra-form-csrf",
                "source_views": [confirm_view, success_view],
                "runtime_copied": false
            });
            return Some(GeneratedWorkflow {
                source: generated,
                report,
            });
        }
    }
    None
}

struct Notice {
    heading: String,
    body: String,
    button: String,
}

fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn safe_source_path(root: &Path, relative: &str) -> Option<std::path::PathBuf> {
    let relative = Path::new(relative);
    if relative
        .components()
        .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return None;
    }
    let candidate = root.join(relative);
    let canonical_root = fs::canonicalize(root).ok()?;
    let canonical = fs::canonicalize(candidate).ok()?;
    canonical.starts_with(canonical_root).then_some(canonical)
}

fn method_declaration<'a>(source: &'a str, method: &str) -> Option<(&'a str, &'a str)> {
    let pattern = format!(
        r"(?s)\bpublic\s+function\s+{}\s*\([^)]*\)\s*(?::\s*[^{{}}]+)?\{{",
        regex::escape(method)
    );
    let regex = regex::Regex::new(&pattern).ok()?;
    let source_map = PhpSourceMap::new(source);
    for declaration in regex.find_iter(source) {
        if !source_map.is_code(declaration.start()) {
            continue;
        }
        let open_brace = declaration.end().checked_sub(1)?;
        let mut depth = 0_usize;
        for offset in open_brace..source.len() {
            if !source_map.is_code(offset) {
                continue;
            }
            match source.as_bytes()[offset] {
                b'{' => depth = depth.checked_add(1)?,
                b'}' => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some((
                            &source[declaration.start()..=open_brace],
                            &source[open_brace + 1..offset],
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn recognized_controller(source: &str, token: &str) -> bool {
    let Some((show_declaration, show)) = method_declaration(source, "show") else {
        return false;
    };
    let Some((unsubscribe_declaration, unsubscribe)) = method_declaration(source, "unsubscribe")
    else {
        return false;
    };
    let token = regex::escape(token);
    let token_parameter = |declaration: &str| {
        let Ok(regex) = regex::Regex::new(&format!(r"(?is)\([^)]*\${token}\b")) else {
            return false;
        };
        let map = PhpSourceMap::new(declaration);
        let found = regex
            .find_iter(declaration)
            .any(|matched| map.is_code(matched.start()));
        found
    };
    let contains_code_match = |body: &str, pattern: &'static str, description: &'static str| {
        let map = PhpSourceMap::new(body);
        let comment_free = map.without_comments(body);
        let regex = compiled_regex(pattern, description);
        let found = regex
            .find_iter(&comment_free)
            .any(|matched| map.is_code(matched.start()));
        found
    };
    let lookup = |body: &str| {
        contains_code_match(
            body,
            r#"(?is)\bwhere\s*\(\s*['"]token_hash['"]\s*,\s*hash\s*\(\s*['"]sha256['"]\s*,\s*\$token\s*\)\s*\)\s*->\s*first\s*\(\s*\)"#,
            "verified unsubscribe token lookup pattern",
        ) && contains_code_match(
            body,
            r"(?is)\babort_unless\s*\(\s*\$[a-z_][a-z0-9_]*\s*,\s*404\s*\)",
            "verified unsubscribe 404 guard pattern",
        )
    };
    let idempotent = compiled_regex(
        r#"(?is)\bforceFill\s*\(\s*\[.*?['\"]unsubscribed_at['\"]\s*=>\s*\$[a-z_][a-z0-9_]*->unsubscribed_at\s*\?:\s*now\s*\(\s*\).*?\]\s*\)\s*->\s*save\s*\(\s*\)"#,
        "idempotent unsubscribe update pattern",
    );
    let unsubscribe_map = PhpSourceMap::new(unsubscribe);
    let unsubscribe_code = unsubscribe_map.without_comments(unsubscribe);
    token_parameter(show_declaration)
        && token_parameter(unsubscribe_declaration)
        && lookup(show)
        && lookup(unsubscribe)
        && idempotent
            .find_iter(&unsubscribe_code)
            .any(|matched| unsubscribe_map.is_code(matched.start()))
}

fn controller_views(source: &str) -> Option<(String, String)> {
    let confirm = compiled_regex(
        r#"(?s)view\s*\(\s*['\"]([^'\"]+)['\"]"#,
        "unsubscribe confirmation view pattern",
    );
    let source_map = PhpSourceMap::new(source);
    let comment_free = source_map.without_comments(source);
    let matches = confirm
        .captures_iter(&comment_free)
        .filter_map(|capture| {
            let start = capture.get(0)?.start();
            source_map
                .is_code(start)
                .then(|| capture.get(1).map(|value| value.as_str().to_owned()))
                .flatten()
        })
        .collect::<Vec<_>>();
    (matches.len() == 2).then(|| (matches[0].clone(), matches[1].clone()))
}

fn static_notice(root: &Path, view: &str) -> Option<Notice> {
    if !view.bytes().all(|byte| is_ident_byte(byte) || byte == b'.')
        || view.starts_with('.')
        || view.ends_with('.')
        || view.contains("..")
    {
        return None;
    }
    let relative = format!("resources/views/{}.blade.php", view.replace('.', "/"));
    let source = fs::read_to_string(safe_source_path(root, &relative)?).ok()?;
    if source.contains("@include") || source.contains("@extends") {
        return None;
    }
    let extract = |tag: &str| {
        let regex = regex::Regex::new(&format!(r"(?is)<{tag}\b[^>]*>(.*?)</{tag}\s*>")).ok()?;
        let capture = regex.captures(&source)?.get(1)?.as_str();
        let strip_tags = compiled_regex(r"(?is)<[^>]*>", "static Blade text tag pattern");
        let text = strip_tags.replace_all(capture, " ");
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if text.contains("{{") || text.contains("@") {
            return None;
        }
        (!text.is_empty()).then_some(text)
    };
    let heading = extract("h1")?;
    let body = extract("p")?;
    let button = extract("button").unwrap_or_else(|| "Continue".into());
    Some(Notice {
        heading,
        body,
        button,
    })
}

fn unique_success_path(route: &str, token: &str) -> String {
    route.trim_end_matches(&format!("/{{{token}}}")).to_owned()
}

fn zelyra_string(value: &str) -> String {
    format!("{:?}", value)
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTROLLER: &str = r#"<?php
class PublicUnsubscribeController {
    public function show(string $token) {
        $record = Model::where('token_hash', hash('sha256', $token))->first();
        abort_unless($record, 404);
        return view('public.confirm', compact('token'));
    }
    public function unsubscribe(string $token) {
        $record = Model::where('token_hash', hash('sha256', $token))->first();
        abort_unless($record, 404);
        $record->forceFill(['unsubscribed_at' => $record->unsubscribed_at ?: now()])->save();
        return view('public.done');
    }
}"#;

    #[test]
    fn recognizes_only_matching_hashed_route_token_and_idempotent_write() {
        assert!(recognized_controller(CONTROLLER, "token"));
        assert!(!recognized_controller(CONTROLLER, "other"));
        assert!(!recognized_controller(
            &CONTROLLER.replace("?: now()", "=> now()"),
            "token"
        ));
    }

    #[test]
    fn ignores_method_lookalikes_in_comments_and_handles_nested_method_blocks() {
        let commented = format!(
            "<?php\n/* {} */\nclass EmptyController {{}}",
            CONTROLLER.replace("<?php", "")
        );
        assert!(!recognized_controller(&commented, "token"));

        let nested = CONTROLLER.replace(
            "abort_unless($record, 404);",
            "if ($record) {\n            abort_unless($record, 404);\n        } else {\n            abort_unless(false, 404);\n        }",
        );
        assert!(recognized_controller(&nested, "token"));
    }

    #[test]
    fn supports_typed_method_return_declarations() {
        let typed = CONTROLLER
            .replace(
                "function show(string $token) {",
                "function show(string $token): View {",
            )
            .replace(
                "function unsubscribe(string $token) {",
                "function unsubscribe(string $token): View {",
            );
        assert!(recognized_controller(&typed, "token"));
    }

    #[test]
    fn rejects_lookup_guards_and_idempotent_updates_that_exist_only_in_comments() {
        let commented_lookup = CONTROLLER.replace(
            "$record = Model::where('token_hash', hash('sha256', $token))->first();\n        abort_unless($record, 404);",
            "/* Model::where('token_hash', hash('sha256', $token))->first();\n        abort_unless($record, 404); */",
        );
        assert!(!recognized_controller(&commented_lookup, "token"));

        let commented_update = CONTROLLER.replace(
            "$record->forceFill(['unsubscribed_at' => $record->unsubscribed_at ?: now()])->save();",
            "$record->forceFill([]); /* $record->forceFill(['unsubscribed_at' => $record->unsubscribed_at ?: now()])->save(); */",
        );
        assert!(!recognized_controller(&commented_update, "token"));
    }

    #[test]
    fn ignores_view_calls_inside_comments_and_string_literals() {
        let source = format!(
            "<?php\n$example = \"view('fake.one')\";\n// view('fake.two')\n{}",
            CONTROLLER.replace("<?php", "")
        );
        assert_eq!(
            controller_views(&source),
            Some(("public.confirm".into(), "public.done".into()))
        );
    }

    #[test]
    fn success_route_replaces_the_named_capture() {
        assert_eq!(
            unique_success_path("/unsubscribe/{confirmation}", "confirmation"),
            "/unsubscribe"
        );
    }

    #[test]
    fn schema_grant_is_scoped_to_the_verified_workflow_module() {
        let schema = "table suppressions {\n    id: Id primary auto\n}\n";
        let generated = grant_schema_access(
            schema,
            "suppressions",
            "src/imported_public_unsubscribe.zyl",
        )
        .expect("the selected table receives the workflow grant");
        assert!(
            generated.contains("access { read_write: [\"src/imported_public_unsubscribe.zyl\"] }")
        );
        assert!(grant_schema_access(schema, "other_table", "src/workflow.zyl").is_none());
    }

    #[test]
    fn generates_module_only_from_complete_route_controller_schema_and_static_views() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-route-workflow-test-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = fs::remove_dir_all(&root);
        let controller_dir = root.join("app/Http/Controllers");
        let view_dir = root.join("resources/views/public");
        fs::create_dir_all(&controller_dir).unwrap();
        fs::create_dir_all(&view_dir).unwrap();
        fs::write(controller_dir.join("UnsubscribeController.php"), CONTROLLER).unwrap();
        fs::write(
            view_dir.join("confirm.blade.php"),
            "<h1>Confirm mailing preferences</h1><p>Stop these messages?</p><button>Unsubscribe</button><form>{{ route('unsubscribe') }}</form>",
        )
        .unwrap();
        fs::write(
            view_dir.join("done.blade.php"),
            "<h1>Preferences saved</h1><p>This address is unsubscribed.</p>",
        )
        .unwrap();
        let candidates = json!({
            "model_consumer_routes": [
                {
                    "route": "get /unsubscribe/{token}",
                    "controller_action": "UnsubscribeController@show",
                    "controller_source_file": "app/Http/Controllers/UnsubscribeController.php",
                    "table": "suppressions"
                },
                {
                    "route": "post /unsubscribe/{token}",
                    "controller_action": "UnsubscribeController@unsubscribe",
                    "controller_source_file": "app/Http/Controllers/UnsubscribeController.php",
                    "table": "suppressions"
                }
            ]
        });
        let schema = vec![json!({
            "table": "suppressions",
            "candidate_source": "table suppressions {\n    id: Id primary auto\n    token_hash: String(64) required\n    unsubscribed_at: Timestamp?\n}"
        })];
        let result = generate(&root, &candidates, &schema).expect("supported shape generated");
        assert!(result.source.contains("verify sql<Int[]>"));
        assert!(result.source.contains("require_match"));
        assert!(result
            .source
            .contains("COALESCE(unsubscribed_at, CURRENT_TIMESTAMP)"));
        assert_eq!(
            result.report["status"],
            "generated_checked_token_unsubscribe_module"
        );
        let incomplete_schema = vec![json!({
            "table": "suppressions",
            "candidate_source": "table suppressions { id: Id primary auto }"
        })];
        assert!(generate(&root, &candidates, &incomplete_schema).is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
