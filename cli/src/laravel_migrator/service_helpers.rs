//! Conservative translation for pure, schema-backed Laravel service helpers.
//!
//! This module recognizes one deliberately narrow operation shape: joining a
//! static list of non-null String fields after PHP's default `trim` behavior.
//! PHP is never executed, and a candidate is never described as fully wired.

use super::{compiled_regex, graph, PhpSourceMap, SourceSpan};
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[derive(Clone)]
struct Candidate {
    kind: ServiceHelperKind,
    class_fqcn: String,
    method: String,
    function: String,
    delimiter: String,
    fields: Vec<String>,
    model_type: Option<String>,
    file: String,
    span: SourceSpan,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ServiceHelperKind {
    OrderAddressFromArray,
    TenantAddress,
}

#[derive(Default)]
pub(super) struct ServiceHelperAnalyzer {
    candidates: Vec<Candidate>,
    rejected: Vec<Value>,
}

impl ServiceHelperAnalyzer {
    pub(super) fn scan(&mut self, file: &str, source: &str) {
        let source_map = PhpSourceMap::new(source);
        let classes = compiled_regex(
            r"(?m)\bclass\s+([A-Za-z_][A-Za-z0-9_]*)[^\{]*\{",
            "Laravel service-helper class regex",
        );
        let methods = compiled_regex(
            r"(?m)\bpublic\s+static\s+function\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(\s*array\s+\$([A-Za-z_][A-Za-z0-9_]*)\s*\)\s*:\s*string\s*\{",
            "Laravel pure service-helper signature regex",
        );
        let tenant_address_method = compiled_regex(
            r"(?m)\bpublic\s+static\s+function\s+(address)\s*\(\s*(Tenant)\s+\$([A-Za-z_][A-Za-z0-9_]*)\s*\)\s*:\s*string\s*\{",
            "Laravel tenant-address helper signature regex",
        );
        let body_pattern = compiled_regex(
            r#"(?s)^\s*return\s+implode\s*\(\s*(?:'([^']*)'|"([^"]*)")\s*,\s*array_map\s*\(\s*fn\s*\(\s*\$([A-Za-z_][A-Za-z0-9_]*)\s*\)\s*=>\s*trim\s*\(\s*\(string\)\s*\(\s*\$([A-Za-z_][A-Za-z0-9_]*)\s*\[\s*\$([A-Za-z_][A-Za-z0-9_]*)\s*\]\s*\?\?\s*(?:''|"")\s*\)\s*\)\s*,\s*\[([\s\S]*?)\]\s*\)\s*\)\s*;\s*$"#,
            "Laravel static trim-and-join helper body regex",
        );
        let tenant_address_body = compiled_regex(
            r#"(?s)^\s*if\s*\(\s*blank\s*\(\s*\$([A-Za-z_][A-Za-z0-9_]*)\s*->\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)\s*\|\|\s*blank\s*\(\s*\$([A-Za-z_][A-Za-z0-9_]*)\s*->\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)\s*\|\|\s*blank\s*\(\s*\$([A-Za-z_][A-Za-z0-9_]*)\s*->\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)\s*\)\s*return\s*(?:''|\"\")\s*;\s*return\s*trim\s*\(\s*\$([A-Za-z_][A-Za-z0-9_]*)\s*->\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)\s*\.\s*['\"]([^'\"]*)['\"]\s*\.\s*trim\s*\(\s*\$([A-Za-z_][A-Za-z0-9_]*)\s*->\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)\s*\.\s*['\"]([^'\"]*)['\"]\s*\.\s*trim\s*\(\s*\$([A-Za-z_][A-Za-z0-9_]*)\s*->\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)\s*\.\s*['\"]([^'\"]*)['\"]\s*\.\s*\(\s*\$([A-Za-z_][A-Za-z0-9_]*)\s*->\s*([A-Za-z_][A-Za-z0-9_]*)\s*\?:\s*['\"]([^'\"]*)['\"]\s*\)\s*;\s*$"#,
            "Laravel pure tenant-address helper body regex",
        );

        let mut class_ranges = Vec::new();
        for capture in classes.captures_iter(source) {
            let matched = capture.get(0).expect("class declaration match");
            if !source_map.is_code(matched.start()) {
                continue;
            }
            let open = matched.end() - 1;
            if let Some(close) = graph::matching_brace(source.as_bytes(), &source_map, open) {
                class_ranges.push((open, close, capture[1].to_owned()));
            }
        }

        for capture in methods.captures_iter(source) {
            let signature = capture.get(0).expect("static method signature match");
            if !source_map.is_code(signature.start()) {
                continue;
            }
            let Some((class_open, class_close, class)) = class_ranges
                .iter()
                .filter(|(open, close, _)| *open < signature.start() && signature.start() < *close)
                .min_by_key(|(open, close, _)| close - open)
            else {
                continue;
            };
            let method = capture[1].to_owned();
            let argument = capture[2].to_owned();
            let body_open = signature.end() - 1;
            let Some(body_close) = graph::matching_brace(source.as_bytes(), &source_map, body_open)
            else {
                continue;
            };
            if body_close > *class_close {
                continue;
            }
            let body = &source[body_open + 1..body_close];
            let Some(shape) = body_pattern.captures(body) else {
                continue;
            };
            if !body_pattern
                .find(body)
                .is_some_and(|matched| matched.start() == 0 && matched.end() == body.len())
                || shape[3] != shape[5]
                || shape[4] != argument
            {
                continue;
            }
            let delimiter = shape
                .get(1)
                .or_else(|| shape.get(2))
                .expect("one static delimiter literal")
                .as_str()
                .to_owned();
            let fields_source = shape.get(6).expect("static field list").as_str();
            let Some(fields) = parse_static_string_list(fields_source) else {
                self.reject(
                    file,
                    &source_map,
                    source,
                    signature.range(),
                    &method,
                    "the joined array keys are not a unique list of static string fields",
                );
                continue;
            };
            if fields.is_empty() || fields.iter().collect::<BTreeSet<_>>().len() != fields.len() {
                self.reject(
                    file,
                    &source_map,
                    source,
                    signature.range(),
                    &method,
                    "the joined array keys are empty or duplicated",
                );
                continue;
            }
            let namespace = namespace_prefix(source, &source_map);
            let class_fqcn = namespace
                .map(|namespace| format!("{namespace}\\{class}"))
                .unwrap_or_else(|| class.clone());
            self.candidates.push(Candidate {
                kind: ServiceHelperKind::OrderAddressFromArray,
                function: format!(
                    "laravel_{}_{}",
                    to_snake_case(class),
                    to_snake_case(&method)
                ),
                class_fqcn,
                method,
                delimiter,
                fields,
                model_type: None,
                file: file.to_owned(),
                span: source_map.span(source, signature.start()..body_close + 1),
            });
            let _ = class_open;
        }

        for capture in tenant_address_method.captures_iter(source) {
            let signature = capture.get(0).expect("tenant-address signature match");
            if !source_map.is_code(signature.start()) {
                continue;
            }
            let Some((_, class_close, class)) = class_ranges
                .iter()
                .filter(|(open, close, _)| *open < signature.start() && signature.start() < *close)
                .min_by_key(|(open, close, _)| close - open)
            else {
                continue;
            };
            let model_type = capture[2].to_owned();
            let argument = capture[3].to_owned();
            let body_open = signature.end() - 1;
            let Some(body_close) = graph::matching_brace(source.as_bytes(), &source_map, body_open)
            else {
                continue;
            };
            if body_close > *class_close {
                continue;
            }
            let body = &source[body_open + 1..body_close];
            let Some(shape) = tenant_address_body.captures(body) else {
                continue;
            };
            if !tenant_address_body
                .find(body)
                .is_some_and(|matched| matched.start() == 0 && matched.end() == body.len())
            {
                continue;
            }
            let blank_fields = [&shape[2], &shape[4], &shape[6]];
            let trimmed_fields = [&shape[8], &shape[11], &shape[14]];
            let fallbacks = [&shape[18]];
            if shape[1] != argument
                || shape[3] != argument
                || shape[5] != argument
                || shape[7] != argument
                || shape[10] != argument
                || shape[13] != argument
                || shape[16] != argument
                || blank_fields != ["street", "postal_code", "city"]
                || trimmed_fields != ["street", "postal_code", "city"]
                || shape.get(17).map(|value| value.as_str()) != Some("country")
                || fallbacks != ["Deutschland"]
            {
                continue;
            }
            let delimiters = [
                shape.get(9).expect("literal delimiter").as_str(),
                shape.get(12).expect("literal delimiter").as_str(),
                shape.get(15).expect("literal delimiter").as_str(),
            ];
            if delimiters != [", ", " ", ", "] {
                continue;
            }
            let namespace = namespace_prefix(source, &source_map);
            let class_fqcn = namespace
                .map(|namespace| format!("{namespace}\\{class}"))
                .unwrap_or_else(|| class.clone());
            self.candidates.push(Candidate {
                kind: ServiceHelperKind::TenantAddress,
                function: format!("laravel_{}_address", to_snake_case(class)),
                class_fqcn,
                method: "address".to_owned(),
                delimiter: String::new(),
                fields: vec![
                    "street".into(),
                    "postal_code".into(),
                    "city".into(),
                    "country".into(),
                ],
                model_type: Some(model_type),
                file: file.to_owned(),
                span: source_map.span(source, signature.start()..body_close + 1),
            });
        }
    }

    fn reject(
        &mut self,
        file: &str,
        source_map: &PhpSourceMap,
        source: &str,
        range: std::ops::Range<usize>,
        method: &str,
        reason: &str,
    ) {
        self.rejected.push(json!({
            "file": file,
            "method": method,
            "source_span": span_json(source_map.span(source, range)),
            "reason": reason
        }));
    }

    pub(super) fn finish(&self, schema: &Value, graph_report: &Value) -> Value {
        let mut rejected = self.rejected.clone();
        let mut candidates = Vec::new();
        for candidate in &self.candidates {
            let resolved_schema = match candidate.kind {
                ServiceHelperKind::OrderAddressFromArray => {
                    resolve_string_schema_fields(schema, &candidate.fields)
                }
                ServiceHelperKind::TenantAddress => resolve_model_nullable_string_schema_fields(
                    schema,
                    graph_report,
                    candidate.model_type.as_deref().unwrap_or_default(),
                    &candidate.fields,
                ),
            };
            let Some((schema_table, schema_field_evidence)) = resolved_schema else {
                let reason = match candidate.kind {
                    ServiceHelperKind::OrderAddressFromArray => {
                        "the literal input fields do not resolve to one accepted schema table with non-null String columns"
                    }
                    ServiceHelperKind::TenantAddress => {
                        "the literal input fields do not resolve to the uniquely linked model table with String columns"
                    }
                };
                rejected.push(json!({
                    "file": candidate.file,
                    "method": format!("{}@{}", candidate.class_fqcn, candidate.method),
                    "source_span": span_json(candidate.span),
                    "reason": reason
                }));
                continue;
            };
            let method_node = graph_report["nodes"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|node| {
                    node["file"].as_str() == Some(candidate.file.as_str())
                        && node["name"].as_str().is_some_and(|name| {
                            name == format!("{}@{}", candidate.class_fqcn, candidate.method)
                        })
                });
            let method_id = method_node.and_then(|node| node["id"].as_str());
            let call_sites = graph_report["edges"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|edge| method_id.is_some_and(|method_id| edge["to"]["id"] == method_id))
                .map(|edge| {
                    json!({
                        "kind": edge["kind"],
                        "status": edge["status"],
                        "file": edge["from"]["file"],
                        "method": edge["from"]["name"],
                        "source_span": edge["from"]["source_span"]
                    })
                })
                .collect::<Vec<_>>();
            let function_source = render_candidate(candidate, &schema_field_evidence);
            let support_source = if candidate.kind == ServiceHelperKind::TenantAddress {
                render_service_support(true)
            } else {
                render_service_support(false)
            };
            let candidate_source = format!("{}\n{}", support_source, function_source);
            candidates.push(json!({
                "class": candidate.class_fqcn,
                "method": candidate.method,
                "function": candidate.function,
                "pattern": match candidate.kind {
                    ServiceHelperKind::OrderAddressFromArray => "static_array_order_address",
                    ServiceHelperKind::TenantAddress => "static_tenant_address_projection",
                },
                "model_type": candidate.model_type.as_deref(),
                "source_file": candidate.file,
                "source_span": span_json(candidate.span),
                "schema_table": schema_table,
                "fields": candidate.fields,
                "schema_field_evidence": schema_field_evidence,
                "static_call_sites": call_sites,
                "translation_status": "generated_helper_call_sites_not_wired",
                "candidate_source": candidate_source,
                "function_source": function_source
            }));
        }
        json!({
            "schema_version": "1",
            "generated_function_count": candidates.len(),
            "candidates": candidates,
            "rejected": rejected
        })
    }
}

fn parse_static_string_list(source: &str) -> Option<Vec<String>> {
    let literal = compiled_regex(
        r#"\s*(?:'([^']*)'|"([^"]*)")\s*(?:,|$)"#,
        "Laravel static string field literal regex",
    );
    let mut remaining = source;
    let mut fields = Vec::new();
    while !remaining.trim().is_empty() {
        let capture = literal.captures(remaining)?;
        let whole = capture.get(0)?;
        if whole.start() != 0 {
            return None;
        }
        let field = capture
            .get(1)
            .or_else(|| capture.get(2))?
            .as_str()
            .to_owned();
        if !field.chars().enumerate().all(|(index, character)| {
            character == '_'
                || character.is_ascii_alphanumeric() && (index > 0 || !character.is_ascii_digit())
        }) {
            return None;
        }
        fields.push(field);
        remaining = &remaining[whole.end()..];
    }
    Some(fields)
}

fn resolve_string_schema_fields(schema: &Value, fields: &[String]) -> Option<(String, Vec<Value>)> {
    let mut matches = Vec::new();
    for table in schema["candidates"].as_array()? {
        let table_name = table["table"].as_str()?;
        let columns = table["columns"].as_array()?;
        let mut evidence = Vec::with_capacity(fields.len());
        for field in fields {
            let matching = columns
                .iter()
                .filter(|column| column["source_column_name"].as_str() == Some(field.as_str()))
                .collect::<Vec<_>>();
            let [column] = matching.as_slice() else {
                evidence.clear();
                break;
            };
            if column["zelyra_type"] != "String" || column["nullable"] != false {
                evidence.clear();
                break;
            }
            evidence.push(json!({
                "source_column_name": field,
                "zelyra_type": column["zelyra_type"],
                "nullable": column["nullable"],
                "source_file": column["source_file"],
                "source_span": column["source_span"]
            }));
        }
        if evidence.len() == fields.len() {
            matches.push((table_name.to_owned(), evidence));
        }
    }
    match matches.as_slice() {
        [candidate] => Some(candidate.clone()),
        _ => None,
    }
}

pub(super) fn render_php_trim_helper() -> &'static str {
    "fn laravel_php_trim_default(value: String) -> String {\n    mutable result = \"\"\n    mutable pending = \"\"\n    for character in text_chars(value) {\n        if character == \" \" || character == \"\\n\" || character == \"\\r\" || character == \"\\t\" || character == \"\\v\" || character == \"\\0\" {\n            if result != \"\" { pending = pending + character }\n        } else {\n            result = result + pending + character\n            pending = \"\"\n        }\n    }\n    return result\n}\n"
}

pub(super) fn render_service_support(include_address_helpers: bool) -> String {
    let mut source = render_php_trim_helper().to_owned();
    if include_address_helpers {
        source.push_str(
            "\nfn laravel_php_trim_nullable(value: Option<String>) -> String {\n    return match value {\n        Some(text) => laravel_php_trim_default(text),\n        None => \"\"\n    }\n}\n\nfn laravel_php_blank(value: Option<String>) -> Bool {\n    return match value {\n        Some(text) => laravel_php_trim_default(text) == \"\",\n        None => true\n    }\n}\n\nfn laravel_php_country_or_default(value: Option<String>) -> String {\n    return match value {\n        Some(text) => if text == \"\" || text == \"0\" { \"Deutschland\" } else { text },\n        None => \"Deutschland\"\n    }\n}\n",
        );
    }
    source
}

fn resolve_model_nullable_string_schema_fields(
    schema: &Value,
    graph_report: &Value,
    model_type: &str,
    fields: &[String],
) -> Option<(String, Vec<Value>)> {
    let models = graph_report["nodes"]
        .as_array()?
        .iter()
        .filter(|node| {
            node["kind"] == "model"
                && node["name"].as_str().is_some_and(|name| {
                    name == model_type || name.rsplit('\\').next() == Some(model_type)
                })
        })
        .collect::<Vec<_>>();
    let [model] = models.as_slice() else {
        return None;
    };
    let model_id = model["id"].as_str()?;
    let explicit_edges = graph_report["edges"]
        .as_array()?
        .iter()
        .filter(|edge| edge["kind"] == "model_table" && edge["from"]["id"] == model_id)
        .collect::<Vec<_>>();
    let table_hint = match explicit_edges.as_slice() {
        [edge] if edge["status"] == "resolved" => edge["to"]["name"].as_str()?.to_owned(),
        // Infer only the regular Eloquent snake_case + "s" convention. Irregular
        // plurals and custom table names need an explicit model_table edge.
        [] => format!("{}s", to_snake_case(model_type)),
        _ => return None,
    };
    let mut matches = Vec::new();
    for table in schema["candidates"].as_array()? {
        let table_name = table["table"].as_str()?;
        if table_name != table_hint {
            continue;
        }
        let columns = table["columns"].as_array()?;
        let mut evidence = Vec::with_capacity(fields.len());
        for field in fields {
            let matching = columns
                .iter()
                .filter(|column| column["source_column_name"].as_str() == Some(field.as_str()))
                .collect::<Vec<_>>();
            let [column] = matching.as_slice() else {
                evidence.clear();
                break;
            };
            if column["zelyra_type"] != "String" {
                evidence.clear();
                break;
            }
            evidence.push(json!({
                "source_column_name": field,
                "zelyra_type": column["zelyra_type"],
                "nullable": column["nullable"],
                "source_file": column["source_file"],
                "source_span": column["source_span"]
            }));
        }
        if evidence.len() == fields.len() {
            matches.push((table_name.to_owned(), evidence));
        }
    }
    match matches.as_slice() {
        [candidate] => Some(candidate.clone()),
        _ => None,
    }
}

fn render_candidate(candidate: &Candidate, evidence: &[Value]) -> String {
    if candidate.kind == ServiceHelperKind::TenantAddress {
        return render_tenant_address_candidate(candidate, evidence);
    }
    let parameters = candidate
        .fields
        .iter()
        .map(|field| format!("{field}: String"))
        .collect::<Vec<_>>()
        .join(", ");
    let pieces = candidate
        .fields
        .iter()
        .map(|field| format!("laravel_php_trim_default({field})"))
        .collect::<Vec<_>>();
    let expression = pieces.join(&format!(
        " + {} + ",
        zelyra_string_literal(&candidate.delimiter)
    ));
    format!(
        "pub fn {}({parameters}) -> String {{\n    return {expression}\n}}\n",
        candidate.function
    )
}

fn render_tenant_address_candidate(candidate: &Candidate, evidence: &[Value]) -> String {
    let fields = ["street", "postal_code", "city", "country"];
    let parameter = |field: &str| {
        let nullable = evidence
            .iter()
            .find(|item| item["source_column_name"] == field)
            .is_some_and(|item| item["nullable"] == true);
        format!(
            "{field}: {}",
            if nullable { "Option<String>" } else { "String" }
        )
    };
    let optional_value = |field: &str| {
        let nullable = evidence
            .iter()
            .find(|item| item["source_column_name"] == field)
            .is_some_and(|item| item["nullable"] == true);
        if nullable {
            field.to_owned()
        } else {
            format!("Some({field})")
        }
    };
    let parameters = fields
        .iter()
        .map(|field| parameter(field))
        .collect::<Vec<_>>()
        .join(", ");
    let street = optional_value("street");
    let postal = optional_value("postal_code");
    let city = optional_value("city");
    let country = optional_value("country");
    format!(
        "pub fn {}({parameters}) -> String {{\n    if laravel_php_blank({street}) || laravel_php_blank({postal}) || laravel_php_blank({city}) {{\n        return \"\"\n    }}\n    return laravel_php_trim_nullable({street}) + \", \" + laravel_php_trim_nullable({postal}) + \" \" + laravel_php_trim_nullable({city}) + \", \" + laravel_php_country_or_default({country})\n}}\n",
        candidate.function
    )
}

fn zelyra_string_literal(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
        .replace('\0', "\\0")
        .replace('\u{000b}', "\\v");
    format!("\"{escaped}\"")
}

fn to_snake_case(value: &str) -> String {
    let mut output = String::new();
    for (index, character) in value.chars().enumerate() {
        if character.is_ascii_uppercase() {
            if index > 0 {
                output.push('_');
            }
            output.push(character.to_ascii_lowercase());
        } else {
            output.push(character.to_ascii_lowercase());
        }
    }
    output
}

fn namespace_prefix(source: &str, source_map: &PhpSourceMap) -> Option<String> {
    let namespace = compiled_regex(
        r"(?m)\bnamespace\s+([A-Za-z_][A-Za-z0-9_\\]*)\s*;",
        "Laravel namespace declaration regex",
    );
    let result = namespace
        .captures_iter(source)
        .find(|capture| source_map.is_code(capture.get(0).expect("namespace match").start()))
        .map(|capture| capture[1].trim_matches('\\').to_owned());
    result
}

fn span_json(span: SourceSpan) -> Value {
    json!({
        "start": {"byte_offset": span.start_byte, "line": span.start_line, "column": span.start_column},
        "end": {"byte_offset": span.end_byte, "line": span.end_line, "column": span.end_column},
        "end_exclusive": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#"<?php
namespace App\Services;
class AddressFormatter {
    public static function orderAddress(array $order): string {
        return implode('|', array_map(fn ($key) => trim((string) ($order[$key] ?? '')), ['street', 'postal_code', 'city']));
    }
}
"#;

    fn schema(nullable: bool) -> Value {
        json!({"candidates": [{
            "table": "dispatch_orders",
            "columns": (["street", "postal_code", "city"]).iter().map(|field| json!({
                "source_column_name": field,
                "zelyra_type": "String",
                "nullable": nullable,
                "source_file": "database/migrations/create_dispatch_orders.php",
                "source_span": {"start": {"line": 12}}
            })).collect::<Vec<_>>()
        }]})
    }

    fn graph() -> Value {
        json!({
            "nodes": [{
                "id": "method:app/Services/AddressFormatter.php:orderAddress",
                "file": "app/Services/AddressFormatter.php",
                "name": "App\\Services\\AddressFormatter@orderAddress"
            }, {
                "id": "model:app/Models/Tenant.php:Tenant",
                "file": "app/Models/Tenant.php",
                "kind": "model",
                "name": "Tenant"
            }],
            "edges": [{
                "kind": "service_call",
                "status": "resolved",
                "from": {"file": "app/Services/PlanningService.php", "name": "App\\Services\\PlanningService@compute", "source_span": {"start": {"line": 31}}},
                "to": {"id": "method:app/Services/AddressFormatter.php:orderAddress"}
            }]
        })
    }

    #[test]
    fn translates_schema_backed_service_helper_and_records_resolved_call_sites() {
        let mut analyzer = ServiceHelperAnalyzer::default();
        analyzer.scan("app/Services/AddressFormatter.php", SOURCE);
        let report = analyzer.finish(&schema(false), &graph());
        let [candidate] = report["candidates"].as_array().unwrap().as_slice() else {
            panic!("expected exactly one service helper candidate: {report}");
        };
        assert_eq!(candidate["schema_table"], "dispatch_orders");
        assert_eq!(
            candidate["fields"],
            json!(["street", "postal_code", "city"])
        );
        assert_eq!(
            candidate["translation_status"],
            "generated_helper_call_sites_not_wired"
        );
        assert_eq!(
            candidate["static_call_sites"][0]["file"],
            "app/Services/PlanningService.php"
        );
        assert!(candidate["candidate_source"]
            .as_str()
            .unwrap()
            .contains("laravel_php_trim_default"));
        assert!(candidate["function_source"]
            .as_str()
            .unwrap()
            .contains("pub fn laravel_address_formatter_order_address"));
    }

    #[test]
    fn rejects_service_translation_when_schema_fields_are_nullable_or_ambiguous() {
        let mut analyzer = ServiceHelperAnalyzer::default();
        analyzer.scan("app/Services/AddressFormatter.php", SOURCE);
        let report = analyzer.finish(&schema(true), &graph());
        assert_eq!(report["generated_function_count"], 0);
        assert!(report["rejected"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("non-null String"));
    }

    #[test]
    fn resolves_tenant_address_fields_through_the_typed_model_and_generates_nullable_types() {
        let source = r#"<?php
namespace App\Services\Dispatch;
class ZoneClustering {
    public static function address(Tenant $tenant): string {
        if (blank($tenant->street) || blank($tenant->postal_code) || blank($tenant->city)) return '';
        return trim($tenant->street).', '.trim($tenant->postal_code).' '.trim($tenant->city).', '.($tenant->country ?: 'Deutschland');
    }
}
"#;
        let schema = json!({"candidates": [
            {"table": "customers", "columns": [
                {"source_column_name":"street", "zelyra_type":"String", "nullable":true},
                {"source_column_name":"postal_code", "zelyra_type":"String", "nullable":true},
                {"source_column_name":"city", "zelyra_type":"String", "nullable":true},
                {"source_column_name":"country", "zelyra_type":"String", "nullable":false}
            ]},
            {"table": "tenants", "columns": [
                {"source_column_name":"street", "zelyra_type":"String", "nullable":true},
                {"source_column_name":"postal_code", "zelyra_type":"String", "nullable":true},
                {"source_column_name":"city", "zelyra_type":"String", "nullable":true},
                {"source_column_name":"country", "zelyra_type":"String", "nullable":false}
            ]}
        ]});
        let mut analyzer = ServiceHelperAnalyzer::default();
        analyzer.scan("app/Services/Dispatch/ZoneClustering.php", source);
        let report = analyzer.finish(&schema, &graph());
        let [candidate] = report["candidates"].as_array().unwrap().as_slice() else {
            panic!("expected model-resolved service helper: {report}");
        };
        assert_eq!(candidate["schema_table"], "tenants");
        assert_eq!(candidate["pattern"], "static_tenant_address_projection");
        let function_source = candidate["function_source"].as_str().unwrap();
        assert!(function_source.contains("street: Option<String>"));
        assert!(function_source.contains("country: String"));
        assert!(function_source.contains("laravel_php_blank(street)"));
        assert!(candidate["candidate_source"]
            .as_str()
            .unwrap()
            .contains("laravel_php_country_or_default"));

        let mut irregular_schema = schema.clone();
        irregular_schema["candidates"][1]["table"] = json!("tenant_records");
        assert_eq!(
            analyzer.finish(&irregular_schema, &graph())["generated_function_count"],
            0,
            "an irregular or custom table needs an explicit model_table mapping"
        );
        let graph_without_model = json!({"nodes": [], "edges": []});
        assert_eq!(
            analyzer.finish(&schema, &graph_without_model)["generated_function_count"],
            0,
            "a matching field set alone must not choose among unrelated tables"
        );
    }
}
