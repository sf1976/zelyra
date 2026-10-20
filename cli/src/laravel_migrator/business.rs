//! Conservative translation of pure Laravel model predicates.
//!
//! This module accepts only a small, auditable PHP grammar. It never evaluates
//! PHP and never resolves call sites. Field types must come from one unique
//! generated schema candidate or one unique static migration field declaration.
use super::{graph::matching_brace, PhpSourceMap, SourceSpan};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
struct Candidate {
    class: String,
    method: String,
    function: String,
    predicate: Predicate,
    return_type: &'static str,
    inferred_table: Option<String>,
    parameters: Vec<Parameter>,
    schema_evidence_source: Option<&'static str>,
    schema_field_evidence: Vec<Value>,
    supporting_source_spans: Vec<SourceSpan>,
    file: String,
    span: SourceSpan,
}

#[derive(Clone)]
enum Predicate {
    Membership {
        field: String,
        values: Vec<String>,
    },
    NonEmpty {
        fields: Vec<String>,
    },
    StaticBooleanLookup {
        field: String,
        true_values: Vec<String>,
    },
    CustomDisplayTimes {
        start_field: String,
        end_field: String,
    },
    ActiveUntilFuture {
        status_field: String,
        active_status: String,
        timestamp_field: String,
    },
    StaticStringMatch {
        field: String,
        key_domain: MatchKeyDomain,
        cases: Vec<(String, String)>,
        default: StringMatchFallback,
    },
    RoundedIntegerDivision {
        field: String,
        divisor: u32,
        decimals: u32,
    },
}

#[derive(Clone)]
struct Parameter {
    source_field: String,
    name: String,
    ty: String,
}

struct StaticStringMatch {
    field: String,
    key_domain: MatchKeyDomain,
    cases: Vec<(String, String)>,
    default: StringMatchFallback,
    supporting_source_span: Option<SourceSpan>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchKeyDomain {
    String,
    Integer,
}

#[derive(Clone)]
enum StringMatchFallback {
    Literal(String),
    Identity,
    PhpTruthyLiteral(String),
    PrefixField(String),
    UppercaseFirst,
    UppercaseFirstAfterUnderscoreReplacement,
}

#[derive(Default)]
pub(super) struct BusinessAnalyzer {
    candidates: Vec<Candidate>,
    rejected: Vec<Value>,
    php_ucfirst_ascii_semantics: bool,
}

impl BusinessAnalyzer {
    pub(super) fn with_php_ucfirst_ascii_semantics(enabled: bool) -> Self {
        Self {
            php_ucfirst_ascii_semantics: enabled,
            ..Self::default()
        }
    }

    pub(super) fn scan(&mut self, file: &str, source: &str) {
        let source_map = PhpSourceMap::new(source);
        let classes = super::compiled_regex(
            r"(?m)\bclass\s+([A-Za-z_][A-Za-z0-9_]*)[^\{]*\{",
            "Laravel business class regex",
        );
        let model_parent = super::compiled_regex(
            r"(?i)\bextends\s+(?:[A-Za-z_][A-Za-z0-9_]*\\)*(?:Model|Authenticatable)\b",
            "Laravel Eloquent parent regex",
        );
        let method = super::compiled_regex(
            r"(?m)\b(public)\s+function\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(\s*\)\s*:\s*(bool|string|float)\s*\{",
            "Laravel pure predicate or static string accessor method regex",
        );
        let body = super::compiled_regex(
            r"(?s)^\s*return\s+in_array\s*\(\s*\$this->([A-Za-z_]\w*)\s*,\s*\[([\s\S]*)\]\s*\)\s*;\s*$",
            "Laravel in_array predicate body regex",
        );
        let table_property =
            super::compiled_regex(r"\$table\b", "Laravel explicit model table property regex");
        let single_quoted_table = super::compiled_regex(
            r"(?m)\b(?:protected|public|private)\s+\$table\s*=\s*'([A-Za-z_][A-Za-z0-9_]*)'\s*;",
            "Laravel single-quoted table property regex",
        );
        let double_quoted_table = super::compiled_regex(
            r#"(?m)\b(?:protected|public|private)\s+\$table\s*=\s*"([A-Za-z_][A-Za-z0-9_]*)"\s*;"#,
            "Laravel double-quoted table property regex",
        );
        let mut class_ranges = Vec::new();
        for capture in classes.captures_iter(source) {
            let matched = capture.get(0).expect("class declaration match");
            if !source_map.is_code(matched.start()) {
                continue;
            }
            let open = matched.end() - 1;
            if !model_parent.is_match(&source[matched.start()..open]) {
                continue;
            }
            if let Some(close) = matching_brace(source.as_bytes(), &source_map, open) {
                let class = capture[1].to_owned();
                let class_body = open + 1..close;
                let has_table_property = table_property
                    .find_iter(&source[class_body.clone()])
                    .any(|matched| source_map.is_code(class_body.start + matched.start()));
                let explicit_tables = single_quoted_table
                    .captures_iter(&source[class_body.clone()])
                    .filter(|capture| {
                        source_map.is_code(class_body.start + capture.get(0).unwrap().start())
                    })
                    .map(|capture| capture[1].to_owned())
                    .chain(
                        double_quoted_table
                            .captures_iter(&source[class_body.clone()])
                            .filter(|capture| {
                                source_map
                                    .is_code(class_body.start + capture.get(0).unwrap().start())
                            })
                            .map(|capture| capture[1].to_owned()),
                    )
                    .collect::<Vec<_>>();
                let inferred_table = match explicit_tables.as_slice() {
                    [table] => Some(table.clone()),
                    [] if !has_table_property => default_model_table(&class),
                    _ => None,
                };
                class_ranges.push((open, close, class, inferred_table));
            }
        }
        for capture in method.captures_iter(source) {
            let matched = capture.get(0).expect("predicate method match");
            if !source_map.is_code(matched.start()) {
                continue;
            }
            let Some((class_start, class_end, class, inferred_table)) = class_ranges
                .iter()
                .filter(|(start, end, _, _)| *start < matched.start() && matched.start() < *end)
                .min_by_key(|(start, end, _, _)| end - start)
            else {
                continue;
            };
            let method_name = capture[2].to_owned();
            let return_type = capture[3].to_owned();
            let open = matched.end() - 1;
            let Some(close) = matching_brace(source.as_bytes(), &source_map, open) else {
                continue;
            };
            if close > *class_end {
                continue;
            }
            let function_body = &source[open + 1..close];
            let mut supporting_source_spans = Vec::new();
            let predicate = if return_type == "string" {
                if let Some(parsed) =
                    parse_static_string_match(function_body, self.php_ucfirst_ascii_semantics)
                {
                    supporting_source_spans.extend(parsed.supporting_source_span);
                    Predicate::StaticStringMatch {
                        field: parsed.field,
                        key_domain: parsed.key_domain,
                        cases: parsed.cases,
                        default: parsed.default,
                    }
                } else if let Some(parsed) = parse_static_string_map_lookup(
                    function_body,
                    source,
                    class_start + 1..*class_end,
                    &source_map,
                ) {
                    supporting_source_spans.extend(parsed.supporting_source_span);
                    Predicate::StaticStringMatch {
                        field: parsed.field,
                        key_domain: parsed.key_domain,
                        cases: parsed.cases,
                        default: parsed.default,
                    }
                } else if let Some(parsed) = parse_static_nested_string_lookup(
                    function_body,
                    source,
                    class_start + 1..*class_end,
                    &source_map,
                ) {
                    supporting_source_spans.extend(parsed.supporting_source_span);
                    Predicate::StaticStringMatch {
                        field: parsed.field,
                        key_domain: parsed.key_domain,
                        cases: parsed.cases,
                        default: parsed.default,
                    }
                } else {
                    self.reject(
                        file,
                        &source_map,
                        source,
                        matched.range(),
                        &method_name,
                        "string accessor is outside the static literal match subset",
                    );
                    continue;
                }
            } else if return_type == "float" {
                if let Some((field, divisor, decimals)) =
                    parse_rounded_integer_division(function_body)
                {
                    Predicate::RoundedIntegerDivision {
                        field,
                        divisor,
                        decimals,
                    }
                } else {
                    self.reject(
                        file,
                        &source_map,
                        source,
                        matched.range(),
                        &method_name,
                        "float accessor is outside the static rounded integer-division subset",
                    );
                    continue;
                }
            } else if let Some(parsed) = body.captures(function_body) {
                let field = parsed[1].to_owned();
                let Some(values) = parse_literal_list(&parsed[2]) else {
                    self.reject(
                        file,
                        &source_map,
                        source,
                        matched.range(),
                        &method_name,
                        "predicate values must be a nonempty list of simple quoted ASCII strings",
                    );
                    continue;
                };
                Predicate::Membership { field, values }
            } else if let Some(fields) = parse_nonempty_conjunction(function_body) {
                Predicate::NonEmpty { fields }
            } else if let Some((field, true_values, definition_span)) = parse_static_boolean_lookup(
                function_body,
                source,
                class_start + 1..*class_end,
                &source_map,
            ) {
                supporting_source_spans.push(definition_span);
                Predicate::StaticBooleanLookup { field, true_values }
            } else if let Some((start_field, end_field)) = parse_custom_display_times(function_body)
            {
                Predicate::CustomDisplayTimes {
                    start_field,
                    end_field,
                }
            } else if let Some((status_field, timestamp_field)) =
                parse_active_until_future(function_body)
            {
                Predicate::ActiveUntilFuture {
                    status_field,
                    active_status: "active".into(),
                    timestamp_field,
                }
            } else {
                self.reject(
                    file,
                    &source_map,
                    source,
                    matched.range(),
                    &method_name,
            "predicate body is outside the supported literal in_array, !empty conjunction, static boolean lookup, or active-until-future subsets",
                );
                continue;
            };
            let class_snake = to_snake_case(class);
            let method_snake = to_snake_case(&method_name);
            self.candidates.push(Candidate {
                class: class.clone(),
                method: method_name,
                function: format!("laravel_{class_snake}_{method_snake}"),
                predicate,
                return_type: if return_type == "string" {
                    "String"
                } else if return_type == "float" {
                    "Float"
                } else {
                    "Bool"
                },
                inferred_table: inferred_table.clone(),
                parameters: Vec::new(),
                schema_evidence_source: None,
                schema_field_evidence: Vec::new(),
                supporting_source_spans,
                file: file.into(),
                span: source_map.span(source, matched.range().start..close + 1),
            });
        }
    }

    fn reject(
        &mut self,
        file: &str,
        source_map: &PhpSourceMap,
        source: &str,
        range: std::ops::Range<usize>,
        name: &str,
        reason: &str,
    ) {
        let span = source_map.span(source, range);
        self.rejected.push(json!({
            "name": name,
            "file": file,
            "source_span": span_json(span),
            "reason": reason,
            "classification": "manual"
        }));
    }

    pub(super) fn finish(mut self, schema_generation: &Value) -> Value {
        let mut counts = BTreeMap::<String, usize>::new();
        for candidate in &self.candidates {
            *counts.entry(candidate.function.clone()).or_default() += 1;
        }
        let mut unique = Vec::new();
        for mut candidate in self.candidates.drain(..) {
            if counts.get(&candidate.function) == Some(&1) {
                let inferred = match &candidate.predicate {
                    Predicate::Membership { field, .. } => Some((
                        vec![Parameter {
                            source_field: field.clone(),
                            name: "field_value".into(),
                            ty: "String".into(),
                        }],
                        Vec::new(),
                        "not_applicable",
                    )),
                    Predicate::NonEmpty { fields } => {
                        let Some(table) = candidate.inferred_table.as_deref() else {
                            self.rejected.push(rejected_candidate(
                                &candidate,
                                "no supported Laravel default table name or static table mapping is available for this model",
                            ));
                            continue;
                        };
                        match infer_nonempty_parameters(
                            table,
                            fields,
                            &["String", "Timestamp"],
                            schema_generation,
                        ) {
                            Ok((parameters, evidence, source)) => {
                                Some((parameters, evidence, source))
                            }
                            Err(reason) => {
                                self.rejected.push(rejected_candidate(&candidate, &reason));
                                continue;
                            }
                        }
                    }
                    Predicate::StaticBooleanLookup { field, .. } => Some((
                        vec![Parameter {
                            source_field: field.clone(),
                            name: "field_value".into(),
                            ty: "String".into(),
                        }],
                        Vec::new(),
                        "static_literal_lookup",
                    )),
                    Predicate::StaticStringMatch {
                        field,
                        key_domain,
                        cases,
                        default,
                    } => {
                        let Some(table) = candidate.inferred_table.as_deref() else {
                            self.rejected.push(rejected_candidate(
                                &candidate,
                                "no supported Laravel default table name or static table mapping is available for this model",
                            ));
                            continue;
                        };
                        let expected_type = match key_domain {
                            MatchKeyDomain::String => {
                                "String (nullable fields need a statically known null fallback)"
                            }
                            MatchKeyDomain::Integer => "Int|UInt",
                        };
                        match infer_nonempty_parameters(
                            table,
                            std::slice::from_ref(field),
                            match key_domain {
                                MatchKeyDomain::String => &["String"],
                                MatchKeyDomain::Integer => &["Int", "UInt"],
                            },
                            schema_generation,
                        ) {
                            Ok((parameters, evidence, source))
                                if match key_domain {
                                    MatchKeyDomain::String => {
                                        parameters[0].ty == "String"
                                            || (parameters[0].ty == "String?"
                                                && nullable_string_match_default(default, cases)
                                                    .is_some())
                                    }
                                    MatchKeyDomain::Integer => {
                                        matches!(parameters[0].ty.as_str(), "Int" | "UInt")
                                            && !(parameters[0].ty == "UInt"
                                                && cases
                                                    .iter()
                                                    .any(|(key, _)| key.starts_with('-')))
                                    }
                                } =>
                            {
                                Some((parameters, evidence, source))
                            }
                            Ok(_) => {
                                self.rejected.push(rejected_candidate(
                                    &candidate,
                                    &format!("static string match keys require a non-null {expected_type} schema field"),
                                ));
                                continue;
                            }
                            Err(reason) => {
                                self.rejected.push(rejected_candidate(&candidate, &reason));
                                continue;
                            }
                        }
                    }
                    Predicate::CustomDisplayTimes {
                        start_field,
                        end_field,
                    } => {
                        match infer_unique_nullable_time_fields(
                            &[start_field.clone(), end_field.clone()],
                            schema_generation,
                        ) {
                            Ok((mut parameters, evidence, source)) => {
                                parameters.insert(
                                    0,
                                    Parameter {
                                        source_field: "capacity_override.exists".into(),
                                        name: "override_exists".into(),
                                        ty: "Bool".into(),
                                    },
                                );
                                Some((parameters, evidence, source))
                            }
                            Err(reason) => {
                                self.rejected.push(rejected_candidate(&candidate, &reason));
                                continue;
                            }
                        }
                    }
                    Predicate::ActiveUntilFuture {
                        status_field,
                        timestamp_field,
                        ..
                    } => {
                        let fields = vec![status_field.clone(), timestamp_field.clone()];
                        let Some(table) = candidate.inferred_table.as_deref() else {
                            self.rejected.push(rejected_candidate(
                                &candidate,
                                "no supported Laravel default table name or static table mapping is available for this model",
                            ));
                            continue;
                        };
                        match infer_nonempty_parameters(
                            table,
                            &fields,
                            &["String", "Timestamp"],
                            schema_generation,
                        ) {
                            Ok((mut parameters, evidence, source)) => {
                                if parameters[0].ty != "String" || parameters[1].ty != "Timestamp?"
                                {
                                    self.rejected.push(rejected_candidate(
                                        &candidate,
                                        "active-until-future requires a non-null String status and nullable Timestamp field",
                                    ));
                                    continue;
                                }
                                parameters.push(Parameter {
                                    source_field: "reference_time".into(),
                                    name: "reference_now".into(),
                                    ty: "Timestamp".into(),
                                });
                                Some((parameters, evidence, source))
                            }
                            Err(reason) => {
                                self.rejected.push(rejected_candidate(&candidate, &reason));
                                continue;
                            }
                        }
                    }
                    Predicate::RoundedIntegerDivision { field, .. } => {
                        let Some(table) = candidate.inferred_table.as_deref() else {
                            self.rejected.push(rejected_candidate(
                                &candidate,
                                "no supported Laravel default table name or static table mapping is available for this model",
                            ));
                            continue;
                        };
                        match infer_nonempty_parameters(
                            table,
                            std::slice::from_ref(field),
                            &["Int", "UInt"],
                            schema_generation,
                        ) {
                            Ok((parameters, evidence, source)) => {
                                if parameters[0].ty.ends_with('?') {
                                    self.rejected.push(rejected_candidate(
                                        &candidate,
                                        "rounded integer division requires a non-null schema field",
                                    ));
                                    continue;
                                }
                                Some((parameters, evidence, source))
                            }
                            Err(reason) => {
                                self.rejected.push(rejected_candidate(&candidate, &reason));
                                continue;
                            }
                        }
                    }
                };
                let (parameters, evidence, source) =
                    inferred.expect("supported predicate parameters");
                candidate.parameters = parameters;
                candidate.schema_field_evidence = evidence;
                candidate.schema_evidence_source = Some(source);
                unique.push(candidate);
            } else {
                self.rejected.push(json!({
                    "name": candidate.method,
                    "file": candidate.file,
                    "source_span": span_json(candidate.span),
                    "reason": format!("generated function name `{}` collides with another source declaration; neither candidate was emitted", candidate.function),
                    "classification": "manual"
                }));
            }
        }
        self.candidates = unique;
        let functions = self
            .candidates
            .iter()
            .map(render_candidate)
            .collect::<Vec<_>>()
            .join("\n\n");
        let accepted = self
            .candidates
            .iter()
            .map(|candidate| {
                let mut assumptions = vec![
                    "caller must pass the original model field explicitly",
                    "only this predicate body is transferred; call sites and surrounding workflow are not",
                    "PHP model-cast and coercion behavior is not transferred",
                    "field parameter types come from unique static schema evidence; review every source span and the model-to-table mapping",
                ];
                if matches!(&candidate.predicate, Predicate::ActiveUntilFuture { .. }) {
                    assumptions.push("reference_now must use the same instant and timezone basis as the Laravel Carbon comparison");
                }
                if matches!(
                    &candidate.predicate,
                    Predicate::RoundedIntegerDivision { .. }
                ) {
                    assumptions.push("PHP integer division is explicitly converted to Float; confirm the model field is integer-cast as declared by Laravel and recheck behavior for the target PHP/runtime versions");
                    assumptions.push("PHP and Zelyra round halfway cases away from zero; callers must pass the exact non-null integer field value from the reviewed schema");
                }
                if matches!(&candidate.predicate, Predicate::CustomDisplayTimes { .. }) {
                    assumptions.push("caller must resolve the same capacity-override row and pass its existence plus nullable display-time fields; the relationship query is not transferred");
                    assumptions.push("nullable SQL TIME values preserve the Laravel truth test for valid TIME values; review backend decoding and model casts");
                }
                if matches!(
                    &candidate.predicate,
                    Predicate::StaticStringMatch {
                        default: StringMatchFallback::UppercaseFirst,
                        ..
                    }
                ) {
                    assumptions.push("PHP ucfirst mapping is enabled only when composer.json guarantees PHP 8.2 or newer; PHP 8.2+ ucfirst uppercases only an initial ASCII letter");
                }
                json!({
                "class": candidate.class,
                "method": candidate.method,
                "function": candidate.function,
                "schema_table": candidate.inferred_table.as_deref(),
                "schema_evidence_source": candidate.schema_evidence_source,
                "schema_field_evidence": candidate.schema_field_evidence,
                "supporting_source_spans": candidate.supporting_source_spans.iter().map(|span| span_json(*span)).collect::<Vec<_>>(),
                "candidate_source": render_candidate(candidate),
                "parameters": candidate.parameters.iter().map(|parameter| json!({
                    "source_field": parameter.source_field,
                    "function_parameter": parameter.name,
                    "zelyra_type": parameter.ty
                })).collect::<Vec<_>>(),
                "file": candidate.file,
                "source_span": span_json(candidate.span),
                "classification": "assisted",
                "assumptions": assumptions
            })})
            .collect::<Vec<_>>();
        json!({
            "schema_version": "1",
                "scope": "public zero-argument bool Eloquent model methods with a static literal in_array return, a conjunction of !empty($this->field) terms whose schema fields have supported types, a static boolean lookup over a literal PHP array, a capacity-override display-time truth check with unique nullable TIME schema fields, or a status guard followed by a nullable timestamp isFuture check; and string-returning methods containing a static literal match with unique String/Int/UInt schema evidence, homogeneous literal keys, literal values, and a literal default or same-field prefix fallback. Nullable String fields require a statically known null fallback; float-returning methods are accepted only for rounded division of one non-null Int/UInt schema field by a positive literal with at most 15 decimals",
            "generated_function_count": self.candidates.len(),
            "candidate_source": functions,
            "candidates": accepted,
            "rejected": self.rejected
        })
    }
}

fn rejected_candidate(candidate: &Candidate, reason: &str) -> Value {
    json!({
        "name": candidate.method,
        "file": candidate.file,
        "source_span": span_json(candidate.span),
        "reason": reason,
        "classification": "manual"
    })
}

fn render_candidate(candidate: &Candidate) -> String {
    let parameters = candidate
        .parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, parameter.ty))
        .collect::<Vec<_>>()
        .join(", ");
    let evidence_note = if candidate.schema_evidence_source == Some("standalone_static_field") {
        "; parameter type from a standalone static schema field"
    } else if matches!(&candidate.predicate, Predicate::ActiveUntilFuture { .. }) {
        "; `reference_now` is explicit so time is not read implicitly"
    } else {
        ""
    };
    format!(
        "# Imported pure projection from {}:{} (reviewed static subset{}).\npub fn {}({}) -> {} {{\n    return {}\n}}",
        candidate.file,
        candidate.span.start_line,
        evidence_note,
        candidate.function,
        parameters,
        candidate.return_type,
        predicate_expression(candidate)
    )
}

fn infer_nonempty_parameters(
    table: &str,
    fields: &[String],
    allowed_types: &[&str],
    schema_generation: &Value,
) -> Result<(Vec<Parameter>, Vec<Value>, &'static str), String> {
    let tables = schema_generation["candidates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|candidate| candidate["table"].as_str() == Some(table))
        .collect::<Vec<_>>();
    let (evidence_source, allowed_origins): (&str, &[&str]) = match tables.as_slice() {
        [_] => (
            "generated_table_candidate",
            &["create_candidate", "table_alteration"],
        ),
        [] => (
            "standalone_static_field",
            &["create_candidate", "table_alteration"],
        ),
        _ => {
            return Err(format!(
                "model table `{table}` has no unique generated static schema candidate"
            ))
        }
    };
    let mut evidence = schema_generation["field_evidence"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|entry| {
            entry["table"].as_str() == Some(table)
                && allowed_origins.contains(&entry["origin"].as_str().unwrap_or_default())
                && fields
                    .iter()
                    .any(|field| entry["source_column_name"].as_str() == Some(field.as_str()))
        })
        .cloned()
        .collect::<Vec<_>>();
    if evidence.is_empty() {
        if let [table_candidate] = tables.as_slice() {
            let columns = table_candidate["columns"].as_array().ok_or_else(|| {
                format!("generated schema candidate `{table}` has no column list")
            })?;
            evidence = fields
                .iter()
                .filter_map(|field| {
                    columns
                        .iter()
                        .find(|column| column["source_column_name"].as_str() == Some(field))
                        .map(|column| {
                            json!({
                                "table": table,
                                "source_column_name": column["source_column_name"],
                                "zelyra_type": column["zelyra_type"],
                                "nullable": column["nullable"],
                                "source_file": table_candidate["source_file"],
                                "source_span": table_candidate["source_span"],
                                "origin": "generated_table_candidate"
                            })
                        })
                })
                .collect();
        }
    }
    if tables.is_empty() && evidence.is_empty() {
        return Err(format!(
            "model table `{table}` has no standalone static column evidence for the requested fields"
        ));
    }
    let mut parameters = Vec::with_capacity(fields.len());
    let mut selected_evidence = Vec::with_capacity(fields.len());
    for (index, field) in fields.iter().enumerate() {
        let matches = evidence
            .iter()
            .filter(|entry| entry["source_column_name"].as_str() == Some(field))
            .collect::<Vec<_>>();
        let [column] = matches.as_slice() else {
            return Err(format!(
                "field `{field}` does not have one unique static source column in `{table}`"
            ));
        };
        let base_type = column["zelyra_type"].as_str().ok_or_else(|| {
            format!("field `{field}` in `{table}` has no static Zelyra type evidence")
        })?;
        let nullable = column["nullable"].as_bool().unwrap_or(false);
        if !allowed_types.contains(&base_type) {
            let kind = if allowed_types == ["String", "Timestamp"] {
                "unsupported `!empty` type"
            } else {
                "unsupported type"
            };
            return Err(format!(
                "field `{field}` in `{table}` has {kind} `{base_type}`"
            ));
        }
        parameters.push(Parameter {
            source_field: field.clone(),
            name: format!("field_value_{index}"),
            ty: if nullable {
                format!("{base_type}?")
            } else {
                base_type.into()
            },
        });
        selected_evidence.push((*column).clone());
    }
    Ok((parameters, selected_evidence, evidence_source))
}

fn predicate_expression(candidate: &Candidate) -> String {
    match &candidate.predicate {
        Predicate::Membership { values, .. } => values
            .iter()
            .map(|value| format!("field_value == {}", zelyra_string(value)))
            .collect::<Vec<_>>()
            .join(" || "),
        Predicate::NonEmpty { .. } => candidate
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| nonempty_expression(&parameter.name, &parameter.ty, index))
            .collect::<Vec<_>>()
            .join(" && "),
        Predicate::StaticBooleanLookup { true_values, .. } => {
            if true_values.is_empty() {
                "false".into()
            } else {
                true_values
                    .iter()
                    .map(|value| format!("field_value == {}", zelyra_string(value)))
                    .collect::<Vec<_>>()
                    .join(" || ")
            }
        }
        Predicate::StaticStringMatch {
            key_domain,
            cases,
            default,
            ..
        } => match key_domain {
            MatchKeyDomain::String => {
                let arms = cases
                    .iter()
                    .map(|(key, value)| {
                        format!("{} => {}, ", zelyra_string(key), zelyra_string(value))
                    })
                    .collect::<String>();
                if candidate
                    .parameters
                    .first()
                    .is_some_and(|parameter| parameter.ty == "String?")
                {
                    let null_value = nullable_string_match_default(default, cases)
                        .expect("nullable string fields are validated before rendering");
                    format!(
                        "match field_value_0 {{ Some(value_0) => match value_0 {{ {arms}_ => {} }}, None => {} }}",
                        string_match_fallback(default, "value_0"),
                        zelyra_string(&null_value)
                    )
                } else {
                    format!(
                        "match field_value_0 {{ {arms}_ => {} }}",
                        string_match_fallback(default, "field_value_0")
                    )
                }
            }
            MatchKeyDomain::Integer => {
                let key_type = candidate
                    .parameters
                    .first()
                    .map(|parameter| parameter.ty.as_str())
                    .unwrap_or("Int");
                let conditions = cases
                    .iter()
                    .map(|(key, _)| {
                        format!("field_value_0 == {}", integer_match_key(key, key_type))
                    })
                    .collect::<Vec<_>>();
                let mut fallback = string_match_fallback(default, "field_value_0");
                for ((_, value), condition) in cases.iter().zip(conditions).rev() {
                    fallback = format!(
                        "if {condition} {{ {} }} else {{ {fallback} }}",
                        zelyra_string(value)
                    );
                }
                fallback
            }
        },
        Predicate::CustomDisplayTimes { .. } => {
            "override_exists && (is_some(display_start_time) || is_some(display_end_time))".into()
        }
        Predicate::ActiveUntilFuture {
            active_status, ..
        } => format!(
            "field_value_0 == {} && match field_value_1 {{ Some(deadline) => deadline > reference_now, None => true }}",
            zelyra_string(active_status)
        ),
        Predicate::RoundedIntegerDivision {
            divisor, decimals, ..
        } => format!(
            "round(to_float(field_value_0) / {divisor}.0, {decimals})"
        ),
    }
}

/// Laravel converts a null string-array key to the empty-string key. If there
/// is no such entry, null is accepted only when the fallback has a known string
/// result; otherwise this helper would guess at PHP coercion behavior.
fn nullable_string_match_default(
    default: &StringMatchFallback,
    cases: &[(String, String)],
) -> Option<String> {
    if let Some((_, value)) = cases.iter().find(|(key, _)| key.is_empty()) {
        return Some(value.clone());
    }
    match default {
        StringMatchFallback::Literal(value) | StringMatchFallback::PhpTruthyLiteral(value) => {
            Some(value.clone())
        }
        StringMatchFallback::PrefixField(prefix) => Some(prefix.clone()),
        StringMatchFallback::Identity
        | StringMatchFallback::UppercaseFirst
        | StringMatchFallback::UppercaseFirstAfterUnderscoreReplacement => None,
    }
}

fn string_match_fallback(fallback: &StringMatchFallback, field: &str) -> String {
    match fallback {
        StringMatchFallback::Literal(value) => zelyra_string(value),
        StringMatchFallback::Identity => field.into(),
        StringMatchFallback::PhpTruthyLiteral(value) => format!(
            "if {field} != \"\" && {field} != \"0\" {{ {field} }} else {{ {} }}",
            zelyra_string(value)
        ),
        StringMatchFallback::PrefixField(prefix) => {
            format!("{} + to_string({field})", zelyra_string(prefix))
        }
        StringMatchFallback::UppercaseFirst => format!("text_ucfirst({field})"),
        StringMatchFallback::UppercaseFirstAfterUnderscoreReplacement => {
            format!("text_ucfirst(text_replace({field}, \"_\", \" \"))")
        }
    }
}

fn integer_match_key(key: &str, field_type: &str) -> String {
    if field_type == "UInt" {
        format!("json_decode<UInt>({})", zelyra_string(key))
    } else {
        key.into()
    }
}

fn parse_custom_display_times(body: &str) -> Option<(String, String)> {
    let pattern = super::compiled_regex(
        r"(?s)^\s*\$([A-Za-z_]\w*)\s*=\s*\$this->getCapacityOverride\s*\(\s*\)\s*;\s*return\s+\(bool\)\s*\(\s*\$([A-Za-z_]\w*)\s*&&\s*\(\s*\$([A-Za-z_]\w*)->([A-Za-z_]\w*)\s*\|\|\s*\$([A-Za-z_]\w*)->([A-Za-z_]\w*)\s*\)\s*\)\s*;\s*$",
        "Laravel capacity override display-time accessor regex",
    );
    let captures = pattern.captures(body)?;
    let override_variable = captures.get(1)?.as_str();
    if captures.get(2)?.as_str() != override_variable
        || captures.get(3)?.as_str() != override_variable
        || captures.get(5)?.as_str() != override_variable
    {
        return None;
    }
    let start_field = captures.get(4)?.as_str();
    let end_field = captures.get(6)?.as_str();
    (is_ascii_identifier(start_field) && is_ascii_identifier(end_field) && start_field != end_field)
        .then(|| (start_field.to_owned(), end_field.to_owned()))
}

fn parse_static_string_match(
    body: &str,
    allow_php_ucfirst_ascii: bool,
) -> Option<StaticStringMatch> {
    let match_body = super::compiled_regex(
        r"(?s)^\s*return\s+match\s*\(\s*\$this->([A-Za-z_]\w*)\s*\)\s*\{(.*?)\}\s*;\s*$",
        "Laravel static string match body regex",
    );
    let captures = match_body.captures(body)?;
    let field = captures.get(1)?.as_str();
    if !is_ascii_identifier(field) {
        return None;
    }
    let arm_source = captures.get(2)?.as_str();
    let arms = super::compiled_regex(
        r#"(?s)(?P<key>default|(?:'[^'\\]*'|"[^"\\]*"|-?[0-9]+)(?:\s*,\s*(?:'[^'\\]*'|"[^"\\]*"|-?[0-9]+))*)\s*=>\s*(?P<value>ucfirst\s*\(\s*str_replace\s*\(\s*['"]_['"]\s*,\s*['"] ['"]\s*,\s*\$this->[A-Za-z_]\w*\s*\)\s*\)|ucfirst\s*\(\s*\$this->[A-Za-z_]\w*\s*\)|'[^'\\]*'\s*\.\s*\$this->[A-Za-z_]\w*|"[^"\\]*"\s*\.\s*\$this->[A-Za-z_]\w*|'[^'\\]*'|"[^"\\]*"|\$this->[A-Za-z_]\w*)\s*,?"#,
        "Laravel static string match arm regex",
    );
    let mut cursor = 0;
    let mut cases = Vec::new();
    let mut default = None;
    let mut key_domain = None;
    for arm in arms.captures_iter(arm_source) {
        let whole = arm.get(0)?;
        if !arm_source[cursor..whole.start()]
            .chars()
            .all(|character| character.is_whitespace())
        {
            return None;
        }
        cursor = whole.end();
        let key = arm.name("key")?.as_str();
        let raw_value = arm.name("value")?.as_str().trim();
        let (value, literal_value) =
            parse_static_match_value(raw_value, field, allow_php_ucfirst_ascii)?;
        if key == "default" {
            if default.replace(value).is_some() {
                return None;
            }
        } else {
            let (domain, keys) = parse_static_match_keys(key)?;
            if key_domain.is_some_and(|existing| existing != domain) || !literal_value {
                return None;
            }
            key_domain = Some(domain);
            let StringMatchFallback::Literal(value) = value else {
                return None;
            };
            if keys
                .iter()
                .any(|key| cases.iter().any(|(existing, _)| existing == key))
            {
                return None;
            }
            cases.extend(keys.into_iter().map(|key| (key, value.clone())));
        }
    }
    if !arm_source[cursor..]
        .chars()
        .all(|character| character.is_whitespace())
        || cases.is_empty()
    {
        return None;
    }
    let key_domain = key_domain?;
    let default = default?;
    if key_domain == MatchKeyDomain::Integer
        && matches!(
            default,
            StringMatchFallback::Identity
                | StringMatchFallback::UppercaseFirst
                | StringMatchFallback::UppercaseFirstAfterUnderscoreReplacement
        )
    {
        return None;
    }
    Some(StaticStringMatch {
        field: field.to_owned(),
        key_domain,
        cases,
        default,
        supporting_source_span: None,
    })
}

fn parse_static_match_keys(source: &str) -> Option<(MatchKeyDomain, Vec<String>)> {
    let bytes = source.as_bytes();
    let mut index = 0;
    let mut domain = None;
    let mut keys = Vec::new();
    loop {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index == bytes.len() {
            break;
        }
        let (this_domain, key) = if matches!(bytes[index], b'\'' | b'"') {
            let quote = bytes[index];
            let start = index;
            index += 1;
            while index < bytes.len() && bytes[index] != quote {
                if bytes[index] == b'\\' {
                    return None;
                }
                index += 1;
            }
            if index == bytes.len() {
                return None;
            }
            index += 1;
            (
                MatchKeyDomain::String,
                unquote_simple_php_string(&source[start..index])?,
            )
        } else {
            let start = index;
            if bytes[index] == b'-' {
                index += 1;
            }
            let digits = index;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            if digits == index {
                return None;
            }
            let raw = &source[start..index];
            let normalized = if raw.starts_with('-') {
                raw.parse::<i64>().ok()?.to_string()
            } else {
                raw.parse::<u64>().ok()?.to_string()
            };
            if normalized != raw {
                return None;
            }
            (MatchKeyDomain::Integer, normalized)
        };
        if domain.is_some_and(|existing| existing != this_domain) {
            return None;
        }
        domain = Some(this_domain);
        if keys.contains(&key) {
            return None;
        }
        keys.push(key);
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index == bytes.len() {
            break;
        }
        if bytes[index] != b',' {
            return None;
        }
        index += 1;
    }
    Some((domain?, keys))
}

fn parse_static_match_value(
    value: &str,
    match_field: &str,
    allow_php_ucfirst_ascii: bool,
) -> Option<(StringMatchFallback, bool)> {
    if let Some(literal) = unquote_simple_php_string(value) {
        return Some((StringMatchFallback::Literal(literal), true));
    }
    let ucfirst = super::compiled_regex(
        r"^ucfirst\s*\(\s*\$this->([A-Za-z_]\w*)\s*\)$",
        "Laravel ucfirst accessor regex",
    );
    if let Some(captures) = ucfirst.captures(value) {
        let field = captures.get(1)?.as_str();
        if !allow_php_ucfirst_ascii || field != match_field || !is_ascii_identifier(field) {
            return None;
        }
        return Some((StringMatchFallback::UppercaseFirst, false));
    }
    let ucfirst_underscore_words = super::compiled_regex(
        r#"^ucfirst\s*\(\s*str_replace\s*\(\s*['"]_['"]\s*,\s*['"] ['"]\s*,\s*\$this->([A-Za-z_]\w*)\s*\)\s*\)$"#,
        "Laravel ucfirst/str_replace accessor regex",
    );
    if let Some(captures) = ucfirst_underscore_words.captures(value) {
        let field = captures.get(1)?.as_str();
        if !allow_php_ucfirst_ascii || field != match_field || !is_ascii_identifier(field) {
            return None;
        }
        return Some((
            StringMatchFallback::UppercaseFirstAfterUnderscoreReplacement,
            false,
        ));
    }
    if let Some((prefix, field)) = value.split_once('.') {
        let prefix = unquote_simple_php_string(prefix.trim())?;
        let field = field.trim().strip_prefix("$this->")?;
        if field != match_field || !is_ascii_identifier(field) {
            return None;
        }
        return Some((StringMatchFallback::PrefixField(prefix), false));
    }
    let field = value.trim().strip_prefix("$this->")?;
    if field == match_field && is_ascii_identifier(field) {
        Some((StringMatchFallback::Identity, false))
    } else {
        None
    }
}

fn parse_static_string_map_lookup(
    accessor_body: &str,
    source: &str,
    class_body_range: std::ops::Range<usize>,
    source_map: &PhpSourceMap,
) -> Option<StaticStringMatch> {
    let class_body = source.get(class_body_range.clone())?;
    let accessor = super::compiled_regex(
        r#"(?s)^\s*return\s+self::([A-Z_][A-Z0-9_]*)\s*\[\s*\$this->([A-Za-z_]\w*)\s*\]\s*\?\?\s*(\$this->([A-Za-z_]\w*)|\(\s*\$this->([A-Za-z_]\w*)\s*\?:\s*('[^'\\]*'|\"[^\"\\]*\")\s*\))\s*;\s*$"#,
        "Laravel static string constant lookup accessor regex",
    );
    let captures = accessor.captures(accessor_body.trim())?;
    let constant_name = captures.get(1)?.as_str();
    let field = captures.get(2)?.as_str();
    if !is_ascii_identifier(field) {
        return None;
    }
    let fallback = if let Some(identity_field) = captures.get(4) {
        if identity_field.as_str() != field {
            return None;
        }
        StringMatchFallback::Identity
    } else {
        let truthy_field = captures.get(5)?.as_str();
        if truthy_field != field {
            return None;
        }
        StringMatchFallback::PhpTruthyLiteral(unquote_simple_php_string(captures.get(6)?.as_str())?)
    };
    let constant_regex = super::compiled_regex(
        r"(?s)\b(?:public\s+|protected\s+|private\s+)?const\s+([A-Z_][A-Z0-9_]*)\s*=\s*\[(.*?)\]\s*;",
        "Laravel static string constant array regex",
    );
    let definitions = constant_regex
        .captures_iter(class_body)
        .filter(|capture| {
            capture
                .get(1)
                .is_some_and(|name| name.as_str() == constant_name)
                && source_map.is_code(class_body_range.start + capture.get(0).unwrap().start())
        })
        .collect::<Vec<_>>();
    let [definition] = definitions.as_slice() else {
        return None;
    };
    let definition_match = definition.get(0)?;
    let entries = split_php_array_entries(definition.get(2)?.as_str())?;
    if entries.is_empty() {
        return None;
    }
    let entry_pattern = super::compiled_regex(
        r#"(?s)^\s*(?:'[^'\\]*'|"[^"\\]*")\s*=>\s*(?:'[^'\\]*'|"[^"\\]*")\s*$"#,
        "Laravel static string constant array entry regex",
    );
    let mut cases = Vec::with_capacity(entries.len());
    for entry in entries {
        if !entry_pattern.is_match(entry) {
            return None;
        }
        let (key, value) = entry.split_once("=>")?;
        let key = unquote_simple_php_string(key.trim())?;
        let value = unquote_simple_php_string(value.trim())?;
        if cases.iter().any(|(existing, _)| existing == &key) {
            return None;
        }
        cases.push((key, value));
    }
    Some(StaticStringMatch {
        field: field.to_owned(),
        key_domain: MatchKeyDomain::String,
        cases,
        default: fallback,
        supporting_source_span: Some(source_map.span(
            source,
            class_body_range.start + definition_match.start()
                ..class_body_range.start + definition_match.end(),
        )),
    })
}

/// Import `self::definitions()[$this->field]['property'] ?? fallback` only when
/// the static method returns a closed literal array of literal scalar values.
/// The method is parsed as data; PHP is never executed.
fn parse_static_nested_string_lookup(
    accessor_body: &str,
    source: &str,
    class_body_range: std::ops::Range<usize>,
    source_map: &PhpSourceMap,
) -> Option<StaticStringMatch> {
    let class_body = source.get(class_body_range.clone())?;
    let accessor = super::compiled_regex(
        r#"(?s)^\s*return\s+self::([A-Za-z_][A-Za-z0-9_]*)\s*\(\s*\)\s*\[\s*\$this->([A-Za-z_]\w*)\s*\]\s*\[\s*(?:'([A-Za-z_][A-Za-z0-9_]*)'|\"([A-Za-z_][A-Za-z0-9_]*)\")\s*\]\s*\?\?\s*(\$this->([A-Za-z_]\w*)|'[^'\\]*'|\"[^\"\\]*\"|\(\s*\$this->([A-Za-z_]\w*)\s*\?:\s*('[^'\\]*'|\"[^\"\\]*\")\s*\))\s*;\s*$"#,
        "Laravel nested static string lookup accessor regex",
    );
    let captures = accessor.captures(accessor_body.trim())?;
    let method_name = captures.get(1)?.as_str();
    let field = captures.get(2)?.as_str();
    let property = captures.get(3).or_else(|| captures.get(4))?.as_str();
    if !is_ascii_identifier(field) {
        return None;
    }
    let fallback = if let Some(fallback_field) = captures.get(6) {
        if fallback_field.as_str() != field {
            return None;
        }
        StringMatchFallback::Identity
    } else if let Some(fallback_field) = captures.get(7) {
        if fallback_field.as_str() != field {
            return None;
        }
        StringMatchFallback::PhpTruthyLiteral(unquote_simple_php_string(captures.get(8)?.as_str())?)
    } else {
        let fallback_source = captures.get(5)?.as_str();
        StringMatchFallback::Literal(unquote_simple_php_string(fallback_source)?)
    };

    let constant_regex = super::compiled_regex(
        r#"(?m)\bconst\s+([A-Z_][A-Z0-9_]*)\s*=\s*(?:'([A-Za-z0-9_-]+)'|\"([A-Za-z0-9_-]+)\")\s*;"#,
        "Laravel simple string class constant regex",
    );
    let mut constants = BTreeMap::new();
    for constant in constant_regex.captures_iter(class_body) {
        let whole = constant.get(0)?;
        if !source_map.is_code(class_body_range.start + whole.start()) {
            continue;
        }
        let value = constant.get(2).or_else(|| constant.get(3))?.as_str();
        if constants
            .insert(constant.get(1)?.as_str().to_owned(), value.to_owned())
            .is_some()
        {
            return None;
        }
    }
    if constants.is_empty() {
        return None;
    }

    let method_regex = super::compiled_regex(
        r"(?s)\bstatic\s+function\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(\s*\)\s*:\s*array\s*\{",
        "Laravel static literal array method regex",
    );
    let definitions = method_regex
        .captures_iter(class_body)
        .filter(|capture| {
            capture
                .get(1)
                .is_some_and(|name| name.as_str() == method_name)
                && source_map.is_code(class_body_range.start + capture.get(0).unwrap().start())
        })
        .collect::<Vec<_>>();
    let [definition] = definitions.as_slice() else {
        return None;
    };
    let method_match = definition.get(0)?;
    let open_brace = method_match.end() - 1;
    let open_brace_absolute = class_body_range.start + open_brace;
    let close_brace_absolute =
        super::graph::matching_brace(source.as_bytes(), source_map, open_brace_absolute)?;
    let method_body = source
        .get(open_brace_absolute + 1..close_brace_absolute)?
        .trim();
    let returned = method_body.strip_prefix("return")?.trim();
    let returned = returned.strip_suffix(';')?.trim();
    let array_content = returned.strip_prefix('[')?.strip_suffix(']')?;
    let entries = split_php_array_entries(array_content)?;
    if entries.is_empty() {
        return None;
    }

    let outer_entry = super::compiled_regex(
        r"(?s)^\s*self::([A-Z_][A-Z0-9_]*)\s*=>\s*\[(.*)\]\s*$",
        "Laravel nested static string map entry regex",
    );
    let inner_entry = super::compiled_regex(
        r#"(?s)^\s*(?:'([A-Za-z_][A-Za-z0-9_]*)'|\"([A-Za-z_][A-Za-z0-9_]*)\")\s*=>\s*(.*?)\s*$"#,
        "Laravel nested static string map property regex",
    );
    let mut cases = Vec::with_capacity(entries.len());
    for entry in entries {
        let outer = outer_entry.captures(entry)?;
        let key = constants.get(outer.get(1)?.as_str())?.clone();
        let nested_entries = split_php_array_entries(outer.get(2)?.as_str())?;
        let mut property_value = None;
        let mut seen_properties = BTreeSet::new();
        for nested in nested_entries {
            let parsed = inner_entry.captures(nested)?;
            let nested_key = parsed.get(1).or_else(|| parsed.get(2))?.as_str();
            if !seen_properties.insert(nested_key.to_owned()) {
                return None;
            }
            let scalar = parsed.get(3)?.as_str().trim();
            if !is_static_php_scalar(scalar) {
                return None;
            }
            if nested_key == property {
                property_value = Some(unquote_simple_php_string(scalar)?);
            }
        }
        let value = property_value?;
        if cases.iter().any(|(existing, _)| existing == &key) {
            return None;
        }
        cases.push((key, value));
    }
    Some(StaticStringMatch {
        field: field.to_owned(),
        key_domain: MatchKeyDomain::String,
        cases,
        default: fallback,
        supporting_source_span: Some(source_map.span(
            source,
            class_body_range.start + method_match.start()..close_brace_absolute + 1,
        )),
    })
}

fn unquote_simple_php_string(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    if bytes.len() < 2
        || !matches!(bytes.first(), Some(b'\'' | b'"'))
        || bytes.first() != bytes.last()
        || bytes[1..bytes.len() - 1].contains(&b'\\')
    {
        return None;
    }
    Some(value[1..value.len() - 1].to_owned())
}

fn infer_unique_nullable_time_fields(
    fields: &[String],
    schema_generation: &Value,
) -> Result<(Vec<Parameter>, Vec<Value>, &'static str), String> {
    let mut matches = Vec::new();
    for table in schema_generation["candidates"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let Some(columns) = table["columns"].as_array() else {
            continue;
        };
        let mut selected = Vec::new();
        let mut valid = true;
        for field in fields {
            let columns = columns
                .iter()
                .filter(|column| column["source_column_name"].as_str() == Some(field))
                .collect::<Vec<_>>();
            let [column] = columns.as_slice() else {
                valid = false;
                break;
            };
            if column["zelyra_type"].as_str() != Some("Time")
                || column["nullable"].as_bool() != Some(true)
            {
                valid = false;
                break;
            }
            selected.push((*column).clone());
        }
        if valid {
            matches.push((table, selected));
        }
    }
    let [(table, evidence)] = matches.as_slice() else {
        return Err(format!(
            "fields `{}` must resolve to one table with nullable Time columns",
            fields.join("`, `")
        ));
    };
    let table_name = table["table"].as_str().unwrap_or("related table");
    let source_file = table["source_file"].as_str().unwrap_or_default();
    let mut parameters = Vec::with_capacity(fields.len());
    for (index, field) in fields.iter().enumerate() {
        parameters.push(Parameter {
            source_field: format!("{table_name}.{field}"),
            name: if index == 0 {
                "display_start_time".into()
            } else {
                "display_end_time".into()
            },
            ty: "Time?".into(),
        });
    }
    let evidence = evidence
        .iter()
        .map(|column| {
            json!({
                "table": table_name,
                "source_column_name": column["source_column_name"],
                "zelyra_type": column["zelyra_type"],
                "nullable": column["nullable"],
                "source_file": source_file,
                "source_span": table["source_span"],
                "origin": "generated_table_candidate"
            })
        })
        .collect();
    Ok((parameters, evidence, "unique_related_schema_fields"))
}

fn parse_active_until_future(body: &str) -> Option<(String, String)> {
    let pattern = super::compiled_regex(
        r#"(?s)^\s*if\s*\(\s*\$this->([A-Za-z_]\w*)\s*!==\s*'active'\s*\)\s*return\s+false\s*;\s*if\s*\(\s*!\s*\$this->([A-Za-z_]\w*)\s*\)\s*return\s+true\s*;\s*return\s+\$this->([A-Za-z_]\w*)->isFuture\s*\(\s*\)\s*;\s*$"#,
        "Laravel active-until-future predicate regex",
    );
    let captures = pattern.captures(body)?;
    let status = captures.get(1)?.as_str();
    let nullable_until = captures.get(2)?.as_str();
    let compared_until = captures.get(3)?.as_str();
    if nullable_until != compared_until {
        return None;
    }
    let until = nullable_until;
    (is_ascii_identifier(status) && is_ascii_identifier(until))
        .then(|| (status.to_owned(), until.to_owned()))
}

fn nonempty_expression(parameter: &str, ty: &str, index: usize) -> String {
    match ty {
        "String" => format!("{parameter} != \"\" && {parameter} != \"0\""),
        "String?" => format!(
            "match {parameter} {{ Some(value_{index}) => value_{index} != \"\" && value_{index} != \"0\", None => false }}"
        ),
        "Timestamp" => "true".into(),
        "Timestamp?" => format!(
            "match {parameter} {{ Some(_) => true, None => false }}"
        ),
        _ => "false".into(),
    }
}

fn parse_nonempty_conjunction(body: &str) -> Option<Vec<String>> {
    let expression = body.trim().strip_prefix("return")?.trim();
    let expression = expression.strip_suffix(';')?.trim();
    let mut fields = Vec::new();
    for term in expression.split("&&") {
        let field = term
            .trim()
            .strip_prefix("!empty(")?
            .strip_suffix(')')?
            .trim()
            .strip_prefix("$this->")?;
        if !is_ascii_identifier(field) || fields.iter().any(|existing| existing == field) {
            return None;
        }
        fields.push(field.to_owned());
    }
    (!fields.is_empty()).then_some(fields)
}

/// Match a narrow, pure Laravel accessor such as
/// `return round($this->useful_life_months / 12, 2);`. Require literal,
/// positive bounds and an integer field; model accessors, casts, and all
/// surrounding calculations remain outside this parser.
fn parse_rounded_integer_division(body: &str) -> Option<(String, u32, u32)> {
    let regex = super::compiled_regex(
        r"(?s)^\s*return\s+round\s*\(\s*\$this->([A-Za-z_]\w*)\s*/\s*([0-9]+)\s*,\s*([0-9]+)\s*\)\s*;\s*$",
        "Laravel rounded integer-division accessor regex",
    );
    let captures = regex.captures(body)?;
    let field = captures.get(1)?.as_str();
    let divisor = captures.get(2)?.as_str().parse::<u32>().ok()?;
    let decimals = captures.get(3)?.as_str().parse::<u32>().ok()?;
    (is_ascii_identifier(field) && divisor > 0 && decimals <= 15)
        .then(|| (field.to_owned(), divisor, decimals))
}

/// Accept one narrowly defined Laravel accessor backed by a side-effect-free
/// static array: `self::typeDefinitions()[$this->field]['flag'] ?? false`.
/// Every class constant and every nested array value must be a literal so this
/// importer never executes PHP or silently drops dynamic behavior.
fn parse_static_boolean_lookup(
    accessor_body: &str,
    source: &str,
    class_body_range: std::ops::Range<usize>,
    source_map: &PhpSourceMap,
) -> Option<(String, Vec<String>, SourceSpan)> {
    let class_body = source.get(class_body_range.clone())?;
    let accessor = super::compiled_regex(
        r#"(?s)^\s*return\s+self::typeDefinitions\s*\(\s*\)\s*\[\s*\$this->([A-Za-z_]\w*)\s*\]\s*\[\s*(?:'deducts_vacation'|"deducts_vacation")\s*\]\s*\?\?\s*false\s*;\s*$"#,
        "Laravel static boolean lookup accessor regex",
    );
    let captures = accessor.captures(accessor_body.trim())?;
    let field = captures.get(1)?.as_str();
    if !is_ascii_identifier(field) {
        return None;
    }

    let constant_regex = super::compiled_regex(
        r"(?m)\bconst\s+(TYPE_[A-Z0-9_]+)\s*=\s*'([A-Za-z0-9_-]+)'\s*;",
        "Laravel static type constant regex",
    );
    let mut constants = BTreeMap::new();
    for constant in constant_regex.captures_iter(class_body) {
        let whole = constant.get(0)?;
        if !source_map.is_code(class_body_range.start + whole.start()) {
            continue;
        }
        let key = constant.get(1)?.as_str().to_owned();
        let value = constant.get(2)?.as_str().to_owned();
        if constants.insert(key, value).is_some() {
            return None;
        }
    }
    if constants.is_empty() {
        return None;
    }

    let definition = super::compiled_regex(
        r#"(?s)\bstatic\s+function\s+typeDefinitions\s*\(\s*\)\s*:\s*array\s*\{"#,
        "Laravel literal typeDefinitions declaration regex",
    );
    let definition_match = definition
        .find_iter(class_body)
        .find(|matched| source_map.is_code(class_body_range.start + matched.start()))?;
    let open_brace = definition_match.start() + class_body[definition_match.range()].rfind('{')?;
    let open_brace_absolute = class_body_range.start + open_brace;
    let close_brace_absolute =
        super::graph::matching_brace(source.as_bytes(), source_map, open_brace_absolute)?;
    let method_body = source
        .get(open_brace_absolute + 1..close_brace_absolute)?
        .trim();
    let returned = method_body.strip_prefix("return")?.trim();
    let returned = returned.strip_suffix(';')?.trim();
    let array_content = returned.strip_prefix('[')?.strip_suffix(']')?;
    let entries = split_php_array_entries(array_content)?;
    if entries.is_empty() {
        return None;
    }

    let outer_entry = super::compiled_regex(
        r"(?s)^\s*self::(TYPE_[A-Z0-9_]+)\s*=>\s*\[(.*)\]\s*$",
        "Laravel static type map entry regex",
    );
    let inner_entry = super::compiled_regex(
        r#"(?s)^\s*(?:'(?P<single>[A-Za-z_][A-Za-z0-9_]*)'|"(?P<double>[A-Za-z_][A-Za-z0-9_]*)")\s*=>\s*(.*?)\s*$"#,
        "Laravel static type definition property regex",
    );
    let mut values = BTreeMap::<String, bool>::new();
    for entry in entries {
        let outer = outer_entry.captures(entry)?;
        let constant = outer.get(1)?.as_str();
        let value = constants.get(constant)?.clone();
        let nested = split_php_array_entries(outer.get(2)?.as_str())?;
        let mut deducts_vacation = false;
        let mut found_flag = false;
        for property in nested {
            let parsed = inner_entry.captures(property)?;
            let key = parsed
                .name("single")
                .or_else(|| parsed.name("double"))?
                .as_str();
            let scalar = parsed.get(3)?.as_str().trim();
            if key == "deducts_vacation" {
                if found_flag {
                    return None;
                }
                deducts_vacation = match scalar {
                    "true" => true,
                    "false" => false,
                    _ => return None,
                };
                found_flag = true;
            } else if !is_static_php_scalar(scalar) {
                return None;
            }
        }
        if values.insert(value, deducts_vacation).is_some() {
            return None;
        }
    }
    if values.is_empty() {
        return None;
    }
    Some((
        field.to_owned(),
        values
            .into_iter()
            .filter_map(|(value, enabled)| enabled.then_some(value))
            .collect(),
        source_map.span(
            source,
            class_body_range.start + definition_match.start()..close_brace_absolute + 1,
        ),
    ))
}

fn split_php_array_entries(source: &str) -> Option<Vec<&str>> {
    let bytes = source.as_bytes();
    let mut entries = Vec::new();
    let mut start = 0;
    let mut depth = 0_usize;
    let mut quote = None;
    let mut escaped = false;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == active_quote {
                quote = None;
            }
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = Some(byte),
            b'[' | b'{' | b'(' => depth += 1,
            b']' | b'}' | b')' => {
                depth = depth.checked_sub(1)?;
            }
            b',' if depth == 0 => {
                let entry = source[start..index].trim();
                if !entry.is_empty() {
                    entries.push(entry);
                }
                start = index + 1;
            }
            _ => {}
        }
    }
    if quote.is_some() || depth != 0 {
        return None;
    }
    let tail = source[start..].trim();
    if !tail.is_empty() {
        entries.push(tail);
    }
    Some(entries)
}

fn is_static_php_scalar(source: &str) -> bool {
    let source = source.trim();
    matches!(source, "true" | "false" | "null")
        || source.parse::<i64>().is_ok()
        || is_php_static_string(source)
}

fn is_php_static_string(source: &str) -> bool {
    let Some(quote) = source.as_bytes().first().copied() else {
        return false;
    };
    if !matches!(quote, b'\'' | b'"') || source.as_bytes().last() != Some(&quote) {
        return false;
    }
    let inner = &source[1..source.len() - 1];
    !inner.contains('$')
        && !inner.contains(quote as char)
        && !inner.contains('\\')
        && !inner
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
}

fn is_ascii_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn default_model_table(class: &str) -> Option<String> {
    let singular = to_snake_case(class);
    if singular == "user" {
        return Some("users".into());
    }
    if singular.is_empty()
        || !singular
            .chars()
            .all(|character| character.is_ascii_lowercase() || character == '_')
    {
        return None;
    }
    if let Some(stem) = singular.strip_suffix("_y") {
        return Some(format!("{stem}_ies"));
    }
    if let Some(stem) = singular.strip_suffix('y') {
        if !stem.ends_with(['a', 'e', 'i', 'o', 'u']) {
            return Some(format!("{stem}ies"));
        }
    }
    if ["s", "x", "z", "ch", "sh"]
        .iter()
        .any(|suffix| singular.ends_with(suffix))
    {
        Some(format!("{singular}es"))
    } else {
        Some(format!("{singular}s"))
    }
}

fn parse_literal_list(source: &str) -> Option<Vec<String>> {
    let bytes = source.as_bytes();
    let mut index = 0;
    let mut values = Vec::new();
    loop {
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        let quote = *bytes.get(index)?;
        if !matches!(quote, b'\'' | b'"') {
            return None;
        }
        index += 1;
        let start = index;
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            index += 1;
        }
        if start == index || bytes.get(index) != Some(&quote) {
            return None;
        }
        values.push(source[start..index].to_owned());
        index += 1;
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        match bytes.get(index) {
            None => break,
            Some(b',') => index += 1,
            _ => return None,
        }
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        if index == bytes.len() {
            break;
        }
    }
    (!values.is_empty()).then_some(values)
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
            output.push(character);
        }
    }
    output
}

fn zelyra_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character => escaped.push(character),
        }
    }
    format!("\"{escaped}\"")
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

    #[test]
    fn translates_only_literal_in_array_predicates_with_explicit_source_mapping() {
        let source = r#"<?php
class CashBookEntry extends Model {
    public function isIncome(): bool
    {
        return in_array($this->type, ['receipt', 'bank_withdrawal', 'transit_in']);
    }
}
"#;
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/CashBookEntry.php", source);
        let result = analyzer.finish(&json!({"candidates": []}));
        assert_eq!(result["generated_function_count"], 1);
        assert_eq!(
            result["candidates"][0]["function"],
            "laravel_cash_book_entry_is_income"
        );
        assert_eq!(result["candidates"][0]["source_span"]["start"]["line"], 3);
        assert!(result["candidate_source"]
            .as_str()
            .unwrap()
            .contains("field_value == \"receipt\" || field_value == \"bank_withdrawal\""));
    }

    #[test]
    fn imports_static_string_match_when_schema_field_is_unique_nonnullable_string() {
        let source = r#"<?php
class BankConnection extends Model {
    public function getStatusBadgeClassAttribute(): string
    {
        return match ($this->status) {
            'active' => 'bg-green text-green-800',
            'pending' => 'bg-orange text-orange-800',
            default => 'bg-gray text-gray-800',
        };
    }
}

"#;
        let schema = json!({"candidates":[{"table":"bank_connections","columns":[
            {"source_column_name":"status","zelyra_type":"String","nullable":false}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/BankConnection.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 1);
        assert_eq!(report["candidates"][0]["schema_table"], "bank_connections");
        assert_eq!(
            report["candidates"][0]["parameters"][0]["zelyra_type"],
            "String"
        );
        let generated = report["candidate_source"].as_str().unwrap();
        assert!(generated.contains("-> String {"));
        assert!(generated.contains("\"active\" => \"bg-green text-green-800\""));
        assert!(generated.contains("_ => \"bg-gray text-gray-800\""));
        let tokens = zelyra_lexer::lex(generated).expect("generated accessor lexes");
        let program = zelyra_parser::parse(&tokens).expect("generated accessor parses");
        for (status, expected) in [
            ("active", "bg-green text-green-800"),
            ("pending", "bg-orange text-orange-800"),
            ("unknown", "bg-gray text-gray-800"),
        ] {
            let actual = zelyra_runtime::execute_function(
                &program,
                "laravel_bank_connection_get_status_badge_class_attribute",
                vec![zelyra_runtime::Value::String(status.into())],
                None,
            )
            .unwrap();
            assert_eq!(actual, zelyra_runtime::Value::String(expected.into()));
        }
    }

    #[test]
    fn imports_nullable_static_string_lookup_with_php_null_and_truthiness_semantics() {
        let source = r#"<?php
class Contract extends Model {
    public const PAYMENT_METHODS = [
        'bank_transfer' => 'Banküberweisung',
        'credit_card' => 'Kreditkarte',
    ];
    public function getPaymentMethodLabelAttribute(): string
    {
        return self::PAYMENT_METHODS[$this->payment_method] ?? ($this->payment_method ?: '–');
    }
}
"#;
        let schema = json!({"candidates":[{"table":"contracts","columns":[
            {"source_column_name":"payment_method","zelyra_type":"String","nullable":true}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Contract.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 1);
        assert_eq!(
            report["candidates"][0]["parameters"][0]["zelyra_type"],
            "String?"
        );
        let generated = report["candidate_source"].as_str().unwrap();
        assert!(generated.contains("Some(value_0) => match value_0"));
        assert!(generated.contains("None => \"–\""));
        let tokens = zelyra_lexer::lex(generated).expect("generated accessor lexes");
        let program = zelyra_parser::parse(&tokens).expect("generated accessor parses");
        for (value, expected) in [
            (None, "–"),
            (Some(""), "–"),
            (Some("0"), "–"),
            (Some("bank_transfer"), "Banküberweisung"),
            (Some("unlisted"), "unlisted"),
        ] {
            let value = value.map_or(zelyra_runtime::Value::Option(None), |value| {
                zelyra_runtime::Value::Option(Some(Box::new(zelyra_runtime::Value::String(
                    value.into(),
                ))))
            });
            let actual = zelyra_runtime::execute_function(
                &program,
                "laravel_contract_get_payment_method_label_attribute",
                vec![value],
                None,
            )
            .expect("generated helper executes");
            assert_eq!(actual, zelyra_runtime::Value::String(expected.into()));
        }
    }

    #[test]
    fn nullable_static_string_lookup_preserves_the_empty_php_array_key() {
        let source = r#"<?php
class Contract extends Model {
    public const LABELS = ['' => 'Leerwert', 'active' => 'Aktiv'];
    public function getStatusLabelAttribute(): string
    {
        return self::LABELS[$this->status] ?? ($this->status ?: 'Unbekannt');
    }
}
"#;
        let schema = json!({"candidates":[{"table":"contracts","columns":[
            {"source_column_name":"status","zelyra_type":"String","nullable":true}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Contract.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 1);
        assert!(report["candidate_source"]
            .as_str()
            .unwrap()
            .contains("None => \"Leerwert\""));

        let unsafe_source = source
            .replace("'' => 'Leerwert', ", "")
            .replace("($this->status ?: 'Unbekannt')", "$this->status");
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Contract.php", &unsafe_source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 0);
        assert_eq!(report["rejected"][0]["name"], "getStatusLabelAttribute");
    }

    #[test]
    fn imports_php82_ucfirst_default_only_with_proven_runtime_semantics() {
        let source = r#"<?php
class Invoice extends Model {
    public function getStatusLabelAttribute(): string
    {
        return match ($this->status) {
            'paid' => 'Bezahlt',
            'draft' => 'Entwurf',
            default => ucfirst($this->status),
        };
    }
}
"#;
        let schema = json!({"candidates":[{"table":"invoices","columns":[
            {"source_column_name":"status","zelyra_type":"String","nullable":false}
        ]}]});

        let mut analyzer = BusinessAnalyzer::with_php_ucfirst_ascii_semantics(true);
        analyzer.scan("app/Models/Invoice.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 1);
        assert!(report["candidate_source"]
            .as_str()
            .unwrap()
            .contains("_ => text_ucfirst(field_value_0)"));
        assert!(report["candidates"][0]["assumptions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|assumption| assumption
                .as_str()
                .unwrap()
                .contains("composer.json guarantees PHP 8.2")));

        let generated = report["candidate_source"].as_str().unwrap();
        let tokens = zelyra_lexer::lex(generated).expect("generated accessor lexes");
        let program = zelyra_parser::parse(&tokens).expect("generated accessor parses");
        for (status, expected) in [
            ("paid", "Bezahlt"),
            ("draft", "Entwurf"),
            ("overdue", "Overdue"),
            ("äbgelaufen", "äbgelaufen"),
            ("7days", "7days"),
        ] {
            let actual = zelyra_runtime::execute_function(
                &program,
                "laravel_invoice_get_status_label_attribute",
                vec![zelyra_runtime::Value::String(status.into())],
                None,
            )
            .unwrap();
            assert_eq!(actual, zelyra_runtime::Value::String(expected.into()));
        }
    }

    #[test]
    fn imports_php_label_default_from_underscore_to_words_only_for_the_same_string_field() {
        let source = r#"<?php
class DocumentTemplate extends Model {
    public function getTypeLabelAttribute(): string
    {
        return match ($this->type) {
            'invoice' => 'Rechnung',
            'quote' => 'Angebot',
            default => ucfirst(str_replace('_', ' ', $this->type)),
        };
    }
}
"#;
        let schema = json!({"candidates":[{"table":"document_templates","columns":[
            {"source_column_name":"type","zelyra_type":"String","nullable":false}
        ]}]});

        let mut analyzer = BusinessAnalyzer::with_php_ucfirst_ascii_semantics(true);
        analyzer.scan("app/Models/DocumentTemplate.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 1);
        let generated = report["candidate_source"].as_str().unwrap();
        assert!(generated.contains("text_ucfirst(text_replace(field_value_0, \"_\", \" \"))"));
        let tokens = zelyra_lexer::lex(generated).expect("generated label lexes");
        let program = zelyra_parser::parse(&tokens).expect("generated label parses");
        for (field_value, expected) in [
            ("invoice", "Rechnung"),
            ("quote", "Angebot"),
            ("credit_note", "Credit note"),
            ("custom__type", "Custom  type"),
            ("__custom", "  custom"),
            ("äbc_label", "äbc label"),
        ] {
            let actual = zelyra_runtime::execute_function(
                &program,
                "laravel_document_template_get_type_label_attribute",
                vec![zelyra_runtime::Value::String(field_value.into())],
                None,
            )
            .unwrap();
            assert_eq!(actual, zelyra_runtime::Value::String(expected.into()));
        }
    }

    #[test]
    fn rejects_underscore_label_mapping_for_other_replacements_fields_or_unknown_php_rules() {
        let schema = json!({"candidates":[{"table":"document_templates","columns":[
            {"source_column_name":"type","zelyra_type":"String","nullable":false},
            {"source_column_name":"other","zelyra_type":"String","nullable":false}
        ]}]});
        for (source, php_semantics) in [
            (
                "<?php class DocumentTemplate extends Model { public function getTypeLabelAttribute(): string { return match ($this->type) { default => ucfirst(str_replace('-', ' ', $this->type)), }; } }",
                true,
            ),
            (
                "<?php class DocumentTemplate extends Model { public function getTypeLabelAttribute(): string { return match ($this->type) { default => ucfirst(str_replace('_', ' ', $this->other)), }; } }",
                true,
            ),
            (
                "<?php class DocumentTemplate extends Model { public function getTypeLabelAttribute(): string { return match ($this->type) { default => ucfirst(str_replace('_', ' ', $this->type)), }; } }",
                false,
            ),
        ] {
            let mut analyzer = if php_semantics {
                BusinessAnalyzer::with_php_ucfirst_ascii_semantics(true)
            } else {
                BusinessAnalyzer::default()
            };
            analyzer.scan("app/Models/DocumentTemplate.php", source);
            let report = analyzer.finish(&schema);
            assert_eq!(report["generated_function_count"], 0);
            assert!(
                !report["rejected"].as_array().unwrap().is_empty(),
                "unsupported form was not reported: {source}; report={report}"
            );
        }
    }

    #[test]
    fn rejects_php_ucfirst_import_when_composer_semantics_are_not_proven() {
        let source = r#"<?php
class Invoice extends Model {
    public function getStatusLabelAttribute(): string {
        return match ($this->status) {
            'paid' => 'Bezahlt',
            default => ucfirst($this->status),
        };
    }
}
"#;
        let schema = json!({"candidates":[{"table":"invoices","columns":[
            {"source_column_name":"status","zelyra_type":"String","nullable":false}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Invoice.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 0);
        assert!(report["rejected"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["reason"]
                .as_str()
                .unwrap()
                .contains("outside the static literal match subset")));
    }

    #[test]
    fn imports_comma_grouped_string_match_keys_with_schema_evidence() {
        let source = r#"<?php
class DocumentTemplate extends Model {
    public function getTypeBadgeClassAttribute(): string
    {
        return match ($this->type) {
            'invoice' => 'invoice-badge',
            'email_invoice', 'email_quote', 'email_reminder' => 'email-badge',
            default => 'other-badge',
        };
    }
}
"#;
        let schema = json!({"candidates":[{"table":"document_templates","columns":[
            {"source_column_name":"type","zelyra_type":"String","nullable":false}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/DocumentTemplate.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 1);
        let generated = report["candidate_source"].as_str().unwrap();
        assert!(generated.contains("\"email_invoice\" => \"email-badge\""));
        assert!(generated.contains("\"email_quote\" => \"email-badge\""));
        assert!(generated.contains("\"email_reminder\" => \"email-badge\""));
        let tokens = zelyra_lexer::lex(generated).expect("generated helper lexes");
        let program = zelyra_parser::parse(&tokens).expect("generated helper parses");
        for (value, expected) in [
            ("invoice", "invoice-badge"),
            ("email_invoice", "email-badge"),
            ("email_quote", "email-badge"),
            ("email_reminder", "email-badge"),
            ("unknown", "other-badge"),
        ] {
            let actual = zelyra_runtime::execute_function(
                &program,
                "laravel_document_template_get_type_badge_class_attribute",
                vec![zelyra_runtime::Value::String(value.into())],
                None,
            )
            .unwrap();
            assert_eq!(actual, zelyra_runtime::Value::String(expected.into()));
        }
    }

    #[test]
    fn rejects_duplicate_or_mixed_domain_grouped_match_keys() {
        for arms in [
            "'same', 'same' => 'duplicate', default => 'other',",
            "'text', 1 => 'mixed', default => 'other',",
        ] {
            let source = format!(
                "<?php class DocumentTemplate extends Model {{ public function getTypeBadgeClassAttribute(): string {{ return match ($this->type) {{ {arms} }}; }} }}"
            );
            let schema = json!({"candidates":[{"table":"document_templates","columns":[
                {"source_column_name":"type","zelyra_type":"String","nullable":false}
            ]}]});
            let mut analyzer = BusinessAnalyzer::default();
            analyzer.scan("app/Models/DocumentTemplate.php", &source);
            let report = analyzer.finish(&schema);
            assert_eq!(report["generated_function_count"], 0, "{arms}");
            assert_eq!(report["rejected"][0]["classification"], "manual");
        }
    }

    #[test]
    fn imports_integer_match_with_a_prefix_and_uint_schema_evidence() {
        let source = r#"<?php
class BookingAvailability extends Model {
    public function getDayNameAttribute(): string
    {
        return match ($this->day_of_week) {
            1 => 'Montag',
            2 => 'Dienstag',
            3 => 'Mittwoch',
            4 => 'Donnerstag',
            5 => 'Freitag',
            6 => 'Samstag',
            0 => 'Sonntag',
            default => 'Tag ' . $this->day_of_week,
        };
    }
}
"#;
        let schema = json!({"candidates":[{"table":"booking_availabilities","columns":[
            {"source_column_name":"day_of_week","zelyra_type":"UInt","nullable":false,
             "source_file":"database/migrations/create_booking_tables.php",
             "source_span":{"start":{"line":46}},"origin":"generated_table_candidate"}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/BookingAvailability.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 1);
        assert_eq!(
            report["candidates"][0]["schema_table"],
            "booking_availabilities"
        );
        assert_eq!(
            report["candidates"][0]["parameters"][0]["zelyra_type"],
            "UInt"
        );
        let generated = report["candidate_source"].as_str().unwrap();
        assert!(generated.contains("json_decode<UInt>(\"1\")"));
        assert!(generated.contains("\"Tag \" + to_string(field_value_0)"));
        let tokens = zelyra_lexer::lex(generated).expect("generated helper lexes");
        let program = zelyra_parser::parse(&tokens).expect("generated helper parses");
        for (day, expected) in [
            (0, "Sonntag"),
            (1, "Montag"),
            (6, "Samstag"),
            (7, "Tag 7"),
            (255, "Tag 255"),
        ] {
            let actual = zelyra_runtime::execute_function(
                &program,
                "laravel_booking_availability_get_day_name_attribute",
                vec![zelyra_runtime::Value::UInt(day)],
                None,
            )
            .unwrap();
            assert_eq!(actual, zelyra_runtime::Value::String(expected.into()));
        }
    }

    #[test]
    fn rejects_integer_match_when_schema_domain_cannot_preserve_keys() {
        let source = r#"<?php
class BookingAvailability extends Model {
    public function getDayNameAttribute(): string
    {
        return match ($this->day_of_week) {
            -1 => 'invalid',
            default => 'Tag ' . $this->day_of_week,
        };
    }
}
"#;
        let schema = json!({"candidates":[{"table":"booking_availabilities","columns":[
            {"source_column_name":"day_of_week","zelyra_type":"UInt","nullable":false}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/BookingAvailability.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 0);
        assert_eq!(report["rejected"][0]["classification"], "manual");
    }

    #[test]
    fn imports_string_match_from_create_field_evidence_when_table_candidate_is_withheld() {
        let source = r#"<?php
class Quote extends Model {
    protected $table = 'quotes';
    public function getStatusBadgeClassAttribute(): string
    {
        return match ($this->status) {
            'draft' => 'draft-badge',
            'sent' => 'sent-badge',
            default => 'other-badge',
        };
    }
}
"#;
        let schema = json!({
            "candidates": [],
            "field_evidence": [{
                "table": "quotes",
                "source_column_name": "status",
                "zelyra_type": "String",
                "nullable": false,
                "source_file": "database/migrations/create_quotes.php",
                "source_span": {"start": {"line": 14}},
                "origin": "create_candidate"
            }]
        });
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Quote.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 1);
        assert_eq!(
            report["candidates"][0]["schema_evidence_source"],
            "standalone_static_field"
        );
        assert_eq!(
            report["candidates"][0]["schema_field_evidence"][0]["origin"],
            "create_candidate"
        );
        let generated = report["candidate_source"].as_str().unwrap();
        let tokens = zelyra_lexer::lex(generated).expect("generated accessor lexes");
        let program = zelyra_parser::parse(&tokens).expect("generated accessor parses");
        for (status, expected) in [
            ("draft", "draft-badge"),
            ("sent", "sent-badge"),
            ("paid", "other-badge"),
        ] {
            let actual = zelyra_runtime::execute_function(
                &program,
                "laravel_quote_get_status_badge_class_attribute",
                vec![zelyra_runtime::Value::String(status.into())],
                None,
            )
            .unwrap();
            assert_eq!(actual, zelyra_runtime::Value::String(expected.into()));
        }
    }

    #[test]
    fn imports_literal_class_string_maps_with_identity_fallback_and_source_evidence() {
        let source = r#"<?php
class Contract extends Model {
    public const BILLING_CYCLES = [
        'monthly' => 'Monatlich',
        'quarterly' => 'Vierteljährlich',
    ];

    public function getBillingCycleLabelAttribute(): string
    {
        return self::BILLING_CYCLES[$this->billing_cycle] ?? $this->billing_cycle;
    }
}
"#;
        let schema = json!({"candidates":[{"table":"contracts","columns":[
            {"source_column_name":"billing_cycle","zelyra_type":"String","nullable":false}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Contract.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 1);
        assert_eq!(
            report["candidates"][0]["supporting_source_spans"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let generated = report["candidate_source"].as_str().unwrap();
        assert!(generated.contains("\"quarterly\" => \"Vierteljährlich\""));
        assert!(generated.contains("_ => field_value_0"));
        let tokens = zelyra_lexer::lex(generated).expect("generated accessor lexes");
        let program = zelyra_parser::parse(&tokens).expect("generated accessor parses");
        for (value, expected) in [("monthly", "Monatlich"), ("other", "other")] {
            let actual = zelyra_runtime::execute_function(
                &program,
                "laravel_contract_get_billing_cycle_label_attribute",
                vec![zelyra_runtime::Value::String(value.into())],
                None,
            )
            .unwrap();
            assert_eq!(actual, zelyra_runtime::Value::String(expected.into()));
        }
    }

    #[test]
    fn imports_php_string_truthy_fallbacks_for_static_maps() {
        let source = r#"<?php
class Contract extends Model {
    public const CATEGORIES = ['consulting' => 'Beratung'];
    public function getCategoryLabelAttribute(): string {
        return self::CATEGORIES[$this->category] ?? ($this->category ?: 'Sonstiges');
    }
}
"#;
        let schema = json!({"candidates":[{"table":"contracts","columns":[
            {"source_column_name":"category","zelyra_type":"String","nullable":false}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Contract.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 1);
        let generated = report["candidate_source"].as_str().unwrap();
        assert!(generated.contains("field_value_0 != \"\""));
        assert!(generated.contains("field_value_0 != \"0\""));
        let tokens = zelyra_lexer::lex(generated).expect("generated accessor lexes");
        let program = zelyra_parser::parse(&tokens).expect("generated accessor parses");
        for (value, expected) in [
            ("consulting", "Beratung"),
            ("other", "other"),
            ("", "Sonstiges"),
            ("0", "Sonstiges"),
            ("00", "00"),
        ] {
            let actual = zelyra_runtime::execute_function(
                &program,
                "laravel_contract_get_category_label_attribute",
                vec![zelyra_runtime::Value::String(value.into())],
                None,
            )
            .unwrap();
            assert_eq!(actual, zelyra_runtime::Value::String(expected.into()));
        }
    }

    #[test]
    fn imports_php_string_truthy_fallback_for_nested_static_definitions() {
        let source = r#"<?php
class Contract extends Model {
    public const STANDARD = 'standard';
    public static function definitions(): array {
        return [self::STANDARD => ['label' => 'Standard']];
    }
    public function getTypeLabelAttribute(): string {
        return self::definitions()[$this->type]['label'] ?? ($this->type ?: 'Sonstiges');
    }
}
"#;
        let schema = json!({"candidates":[{"table":"contracts","columns":[
            {"source_column_name":"type","zelyra_type":"String","nullable":false}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Contract.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 1);
        let generated = report["candidate_source"].as_str().unwrap();
        let tokens = zelyra_lexer::lex(generated).expect("generated accessor lexes");
        let program = zelyra_parser::parse(&tokens).expect("generated accessor parses");
        for (value, expected) in [
            ("standard", "Standard"),
            ("", "Sonstiges"),
            ("0", "Sonstiges"),
        ] {
            let actual = zelyra_runtime::execute_function(
                &program,
                "laravel_contract_get_type_label_attribute",
                vec![zelyra_runtime::Value::String(value.into())],
                None,
            )
            .unwrap();
            assert_eq!(actual, zelyra_runtime::Value::String(expected.into()));
        }
    }

    #[test]
    fn rejects_nullable_string_lookup_with_unknown_null_behavior() {
        let source = r#"<?php
class Contract extends Model {
    public const CATEGORIES = ['consulting' => 'Beratung'];
    public function getCategoryLabelAttribute(): string {
            return self::CATEGORIES[$this->category] ?? $this->category;
    }
}
"#;
        let schema = json!({"candidates":[{"table":"contracts","columns":[
            {"source_column_name":"category","zelyra_type":"String","nullable":true}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Contract.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 0);
        assert!(report["rejected"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("statically known null fallback"));
    }

    #[test]
    fn imports_nested_static_string_definition_lookups_with_explicit_fallbacks() {
        let source = r#"<?php
class VacationRequest extends Model {
    public const TYPE_VACATION = 'vacation';
    public const TYPE_SICK = 'sick';

    public static function typeDefinitions(): array {
        return [
            self::TYPE_VACATION => ['label' => 'Erholungsurlaub', 'icon' => '🏖️'],
            self::TYPE_SICK => ['label' => 'Krankheit', 'icon' => '🤒'],
        ];
    }

    public function getTypeLabelAttribute(): string {
        return self::typeDefinitions()[$this->type]['label'] ?? $this->type;
    }

    public function getTypeIconAttribute(): string {
        return self::typeDefinitions()[$this->type]['icon'] ?? '📅';
    }
}
"#;
        let schema = json!({"candidates":[{"table":"vacation_requests","columns":[
            {"source_column_name":"type","zelyra_type":"String","nullable":false}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/VacationRequest.php", source);
        let report = analyzer.finish(&schema);

        assert_eq!(report["generated_function_count"], 2);
        let generated = report["candidate_source"].as_str().unwrap();
        assert!(generated.contains("laravel_vacation_request_get_type_label_attribute"));
        assert!(generated.contains("laravel_vacation_request_get_type_icon_attribute"));
        let tokens = zelyra_lexer::lex(generated).expect("generated helpers lex");
        let program = zelyra_parser::parse(&tokens).expect("generated helpers parse");
        for (function, input, expected) in [
            (
                "laravel_vacation_request_get_type_label_attribute",
                "vacation",
                "Erholungsurlaub",
            ),
            (
                "laravel_vacation_request_get_type_label_attribute",
                "other",
                "other",
            ),
            (
                "laravel_vacation_request_get_type_icon_attribute",
                "sick",
                "🤒",
            ),
            (
                "laravel_vacation_request_get_type_icon_attribute",
                "other",
                "📅",
            ),
        ] {
            let actual = zelyra_runtime::execute_function(
                &program,
                function,
                vec![zelyra_runtime::Value::String(input.into())],
                None,
            )
            .unwrap();
            assert_eq!(actual, zelyra_runtime::Value::String(expected.into()));
        }
        assert!(report["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .all(|candidate| {
                candidate["schema_evidence_source"] == "generated_table_candidate"
                    && candidate["supporting_source_spans"]
                        .as_array()
                        .unwrap()
                        .len()
                        == 1
            }));
    }

    #[test]
    fn rejects_nested_static_lookup_when_a_map_value_is_dynamic() {
        let source = r#"<?php
class VacationRequest extends Model {
    public const TYPE_VACATION = 'vacation';
    public static function typeDefinitions(): array {
        return [self::TYPE_VACATION => ['label' => __('Vacation')]];
    }
    public function getTypeLabelAttribute(): string {
        return self::typeDefinitions()[$this->type]['label'] ?? $this->type;
    }
}
"#;
        let schema = json!({"candidates":[{"table":"vacation_requests","columns":[
            {"source_column_name":"type","zelyra_type":"String","nullable":false}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/VacationRequest.php", source);
        let report = analyzer.finish(&schema);

        assert_eq!(report["generated_function_count"], 0);
        assert_eq!(report["rejected"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn rejects_dynamic_or_duplicate_values_in_static_class_string_maps() {
        let source = r#"<?php
class Contract extends Model {
    public const LABELS = ['active' => 'Aktiv', 'active' => dynamic_label()];
    public function getStatusLabelAttribute(): string {
        return self::LABELS[$this->status] ?? $this->status;
    }
}
"#;
        let schema = json!({"candidates":[{"table":"contracts","columns":[
            {"source_column_name":"status","zelyra_type":"String","nullable":false}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Contract.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 0);
        assert!(!report["rejected"].as_array().unwrap().is_empty());

        let duplicate_literals = source.replace("dynamic_label()", "'Aktiv erneut'");
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Contract.php", &duplicate_literals);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 0);

        let changed_fallback = source.replace("?? $this->status", "?? $this->other");
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Contract.php", &changed_fallback);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 0);
    }

    #[test]
    fn imports_boolean_lookup_from_static_literal_php_map() {
        let source = r#"<?php
class VacationRequest extends Model {
    public const TYPE_VACATION = 'vacation';
    public const TYPE_SICK = 'sick';

    public function getDeductsVacationAttribute(): bool
    {
        return self::typeDefinitions()[$this->type]['deducts_vacation'] ?? false;
    }

    public static function typeDefinitions(): array
    {
        return [
            self::TYPE_VACATION => [
                'label' => 'Urlaub, Erholung',
                'icon' => '🏖️',
                'deducts_vacation' => true,
            ],
            self::TYPE_SICK => [
                'label' => 'Krankheit',
                'deducts_vacation' => false,
            ],
        ];
    }
}
"#;
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/VacationRequest.php", source);
        let result = analyzer.finish(&json!({"candidates": []}));
        assert_eq!(result["generated_function_count"], 1);
        assert_eq!(
            result["candidates"][0]["schema_evidence_source"],
            "static_literal_lookup"
        );
        assert_eq!(
            result["candidates"][0]["parameters"][0]["source_field"],
            "type"
        );
        assert_eq!(
            result["candidates"][0]["supporting_source_spans"][0]["start"]["line"],
            11
        );
        let generated = result["candidate_source"].as_str().unwrap();
        assert!(generated.contains("field_value == \"vacation\""));
        assert!(!generated.contains("field_value == \"sick\""));
    }

    #[test]
    fn imports_rounded_float_accessor_only_from_a_nonnullable_integer_field() {
        let source = r#"<?php
class Asset extends Model {
    public function getUsefulLifeYearsAttribute(): float
    {
        return round($this->useful_life_months / 12, 2);
    }
}
"#;
        let schema = json!({
            "candidates": [{
                "table": "assets",
                "columns": [{
                    "source_column_name": "useful_life_months",
                    "zelyra_type": "UInt",
                    "nullable": false
                }]
            }],
            "field_evidence": []
        });
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Asset.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 1);
        let candidate = &report["candidates"][0];
        assert_eq!(candidate["parameters"][0]["zelyra_type"], "UInt");
        let generated = report["candidate_source"].as_str().unwrap();
        assert!(generated.contains("-> Float"));
        assert!(generated.contains("round(to_float(field_value_0) / 12.0, 2)"));
        assert!(candidate["assumptions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item
                .as_str()
                .unwrap()
                .contains("model field is integer-cast")));

        let nullable_schema = json!({
            "candidates": [{
                "table": "assets",
                "columns": [{
                    "source_column_name": "useful_life_months",
                    "zelyra_type": "UInt",
                    "nullable": true
                }]
            }],
            "field_evidence": []
        });
        let mut nullable_analyzer = BusinessAnalyzer::default();
        nullable_analyzer.scan("app/Models/Asset.php", source);
        let nullable_report = nullable_analyzer.finish(&nullable_schema);
        assert_eq!(nullable_report["generated_function_count"], 0);
        assert!(nullable_report["rejected"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("non-null schema field"));
    }

    #[test]
    fn rounded_float_accessor_requires_a_positive_literal_divisor_and_bounded_precision() {
        for body in [
            "return round($this->months / 0, 2);",
            "return round($this->months / 12, 16);",
            "return round($this->months / $this->period, 2);",
            "return round($this->months / 12, 2) + 1;",
        ] {
            assert!(parse_rounded_integer_division(body).is_none(), "{body}");
        }
    }

    #[test]
    fn rejects_dynamic_values_in_static_lookup_map() {
        let source = r#"<?php
class VacationRequest extends Model {
    public const TYPE_VACATION = 'vacation';

    public function getDeductsVacationAttribute(): bool
    {
        return self::typeDefinitions()[$this->type]['deducts_vacation'] ?? false;
    }

    public static function typeDefinitions(): array
    {
        return [
            self::TYPE_VACATION => [
                'deducts_vacation' => self::isVacationEnabled(),
            ],
        ];
    }
}
"#;
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/VacationRequest.php", source);
        let result = analyzer.finish(&json!({"candidates": []}));
        assert_eq!(result["generated_function_count"], 0);
        assert_eq!(result["rejected"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn rejects_dynamic_values_and_ignores_comment_or_string_lookalikes() {
        let source = r#"<?php
// public function fake(): bool { return in_array($this->x, ['a']); }
class Ordinary {
    public function ignored(): bool { return in_array($this->status, ['a']); }
}
class Unsafe extends Model {
    public function dynamic(): bool { return in_array($this->status, [$value]); }
    public function escaped(): bool { return in_array($this->status, ['a\\b']); }
}
"#;
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Unsafe.php", source);
        let result = analyzer.finish(&json!({"candidates": []}));
        assert_eq!(result["generated_function_count"], 0);
        assert_eq!(result["rejected"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn imports_nonempty_conjunction_only_with_unique_typed_schema_fields() {
        let source = r#"<?php
class User extends Authenticatable {
    public function hasTwoFactorEnabled(): bool
    {
        return !empty($this->two_factor_secret) && !empty($this->two_factor_confirmed_at);
    }
}
"#;
        let schema = json!({"candidates":[{"table":"users","columns":[
            {"source_column_name":"two_factor_secret","zelyra_type":"String","nullable":true},
            {"source_column_name":"two_factor_confirmed_at","zelyra_type":"Timestamp","nullable":true}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/User.php", source);
        let result = analyzer.finish(&schema);
        assert_eq!(result["generated_function_count"], 1);
        assert_eq!(result["candidates"][0]["schema_table"], "users");
        assert_eq!(
            result["candidates"][0]["parameters"][0]["zelyra_type"],
            "String?"
        );
        assert_eq!(
            result["candidates"][0]["parameters"][1]["zelyra_type"],
            "Timestamp?"
        );
        let generated = result["candidate_source"].as_str().unwrap();
        assert!(generated.contains("match field_value_0 { Some(value_0) => value_0 != \"\" && value_0 != \"0\", None => false }"));
        assert!(generated.contains("match field_value_1 { Some(_) => true, None => false }"));
    }

    #[test]
    fn rejects_nonempty_import_when_schema_type_cannot_preserve_empty_semantics() {
        let source = r#"<?php
class User extends Authenticatable {
    public function hasTwoFactorEnabled(): bool
    {
        return !empty($this->two_factor_secret) && !empty($this->two_factor_confirmed_at);
    }
}
"#;
        let schema = json!({"candidates":[{"table":"users","columns":[
            {"source_column_name":"two_factor_secret","zelyra_type":"Json","nullable":true},
            {"source_column_name":"two_factor_confirmed_at","zelyra_type":"Timestamp","nullable":true}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/User.php", source);
        let result = analyzer.finish(&schema);
        assert_eq!(result["generated_function_count"], 0);
        assert!(result["rejected"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("unsupported `!empty` type `Json`"));
    }

    #[test]
    fn imports_a_field_from_a_standalone_migration_when_the_table_is_not_emitted() {
        let source = r#"<?php
class Invoice extends Model {
    protected $table = 'invoices';
    public function isLocked(): bool { return !empty($this->locked_at); }
}
"#;
        let schema = json!({
            "candidates": [],
            "field_evidence": [{
                "table": "invoices",
                "source_column_name": "locked_at",
                "zelyra_type": "Timestamp",
                "nullable": true,
                "source_file": "database/migrations/2026_01_01_add_locked_at.php",
                "source_span": {"start": {"line": 4}},
            "origin": "table_alteration"
            }]
        });
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Invoice.php", source);
        let result = analyzer.finish(&schema);
        assert_eq!(result["generated_function_count"], 1);
        assert_eq!(
            result["candidates"][0]["schema_evidence_source"],
            "standalone_static_field"
        );
        assert_eq!(
            result["candidates"][0]["parameters"][0]["zelyra_type"],
            "Timestamp?"
        );
        assert_eq!(
            result["candidates"][0]["schema_field_evidence"][0]["source_file"],
            "database/migrations/2026_01_01_add_locked_at.php"
        );
        assert!(result["candidate_source"]
            .as_str()
            .unwrap()
            .contains("pub fn laravel_invoice_is_locked(field_value_0: Timestamp?) -> Bool"));
    }

    #[test]
    fn imports_active_connection_future_check_with_explicit_reference_time() {
        let source = r#"<?php
class BankConnection extends Model {
    public function isAccessValid(): bool
    {
        if ($this->status !== 'active') return false;
        if (!$this->access_valid_until) return true;
        return $this->access_valid_until->isFuture();
    }
}
"#;
        let schema = json!({"candidates":[{"table":"bank_connections","columns":[
            {"source_column_name":"status","zelyra_type":"String","nullable":false},
            {"source_column_name":"access_valid_until","zelyra_type":"Timestamp","nullable":true}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/BankConnection.php", source);
        let result = analyzer.finish(&schema);
        assert_eq!(result["generated_function_count"], 1);
        assert_eq!(result["candidates"][0]["schema_table"], "bank_connections");
        assert_eq!(
            result["candidates"][0]["parameters"][2]["source_field"],
            "reference_time"
        );
        assert!(result["candidates"][0]["assumptions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str().unwrap().contains("timezone basis")));
        let generated = result["candidate_source"].as_str().unwrap();
        assert!(generated.contains(
            "field_value_0: String, field_value_1: Timestamp?, reference_now: Timestamp"
        ));
        assert!(generated.contains("field_value_0 == \"active\" && match field_value_1 { Some(deadline) => deadline > reference_now, None => true }"));
        let parsed = zelyra_lexer::lex(generated).expect("generated predicate lexes");
        zelyra_parser::parse(&parsed).expect("generated predicate parses");
    }

    #[test]
    fn imports_capacity_override_display_time_projection_with_related_schema_evidence() {
        let source = r#"<?php
class BookingAppointment extends Model {
    public function getHasCustomDisplayTimeAttribute(): bool
    {
        $override = $this->getCapacityOverride();
        return (bool) ($override && ($override->display_start_time || $override->display_end_time));
    }
}
"#;
        let schema = json!({"candidates":[
            {"table":"booking_appointments","source_file":"app/Migrations/appointments.php","columns":[]},
            {"table":"booking_exception_capacities","source_file":"app/Migrations/capacities.php","source_span":{"start":{"line":8}},"columns":[
                {"source_column_name":"display_start_time","zelyra_type":"Time","nullable":true},
                {"source_column_name":"display_end_time","zelyra_type":"Time","nullable":true}
            ]}
        ]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/BookingAppointment.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 1);
        let candidate = &report["candidates"][0];
        assert_eq!(candidate["schema_table"], "booking_appointments");
        assert_eq!(
            candidate["schema_evidence_source"],
            "unique_related_schema_fields"
        );
        assert_eq!(
            candidate["parameters"][0]["source_field"],
            "capacity_override.exists"
        );
        assert_eq!(candidate["parameters"][1]["zelyra_type"], "Time?");
        assert_eq!(
            candidate["schema_field_evidence"][0]["table"],
            "booking_exception_capacities"
        );
        let generated = report["candidate_source"].as_str().unwrap();
        assert!(generated.contains(
            "override_exists && (is_some(display_start_time) || is_some(display_end_time))"
        ));
        let tokens = zelyra_lexer::lex(generated).expect("generated accessor lexes");
        let program = zelyra_parser::parse(&tokens).expect("generated accessor parses");

        let no_override = zelyra_runtime::execute_function(
            &program,
            "laravel_booking_appointment_get_has_custom_display_time_attribute",
            vec![
                zelyra_runtime::Value::Bool(false),
                zelyra_runtime::Value::Option(None),
                zelyra_runtime::Value::Option(None),
            ],
            None,
        )
        .unwrap();
        let start_override = zelyra_runtime::execute_function(
            &program,
            "laravel_booking_appointment_get_has_custom_display_time_attribute",
            vec![
                zelyra_runtime::Value::Bool(true),
                zelyra_runtime::Value::Option(Some(Box::new(zelyra_runtime::Value::Time(
                    "00:00:00".into(),
                )))),
                zelyra_runtime::Value::Option(None),
            ],
            None,
        )
        .unwrap();
        assert_eq!(no_override, zelyra_runtime::Value::Bool(false));
        assert_eq!(start_override, zelyra_runtime::Value::Bool(true));
    }

    #[test]
    fn rejects_capacity_display_time_when_related_schema_evidence_is_ambiguous_or_unsafe() {
        let source = r#"<?php
class BookingAppointment extends Model {
    public function getHasCustomDisplayTimeAttribute(): bool
    {
        $override = $this->getCapacityOverride();
        return (bool) ($override && ($override->display_start_time || $override->display_end_time));
    }
}
"#;
        let table = |name: &str, nullable: bool| {
            json!({
                "table": name,
                "source_file": format!("{name}.php"),
                "columns": [
                    {"source_column_name":"display_start_time","zelyra_type":"Time","nullable":nullable},
                    {"source_column_name":"display_end_time","zelyra_type":"Time","nullable":true}
                ]
            })
        };
        for schema in [
            json!({"candidates":[table("first", true), table("second", true)]}),
            json!({"candidates":[table("wrong_nullability", false)]}),
        ] {
            let mut analyzer = BusinessAnalyzer::default();
            analyzer.scan("app/Models/BookingAppointment.php", source);
            let report = analyzer.finish(&schema);
            assert_eq!(report["generated_function_count"], 0);
            assert_eq!(report["rejected"].as_array().unwrap().len(), 1);
            assert!(report["rejected"][0]["reason"]
                .as_str()
                .unwrap()
                .contains("one table with nullable Time columns"));
        }

        let changed_body = source.replace(
            "return (bool) ($override && ($override->display_start_time || $override->display_end_time));",
            "return (bool) ($override && ($override->display_start_time || $other->display_end_time));",
        );
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/BookingAppointment.php", &changed_body);
        let report = analyzer.finish(&json!({"candidates":[]}));
        assert_eq!(report["generated_function_count"], 0);
    }

    #[test]
    fn rejects_changed_time_predicate_or_incompatible_schema_types() {
        let source = |tail: &str| {
            format!(
                "<?php\nclass BankConnection extends Model {{\n    public function isAccessValid(): bool {{\n        if ($this->status !== 'active') return false;\n        if (!$this->access_valid_until) return true;\n        return $this->access_valid_until->{tail};\n    }}\n}}"
            )
        };
        let schema = json!({"candidates":[{"table":"bank_connections","columns":[
            {"source_column_name":"status","zelyra_type":"String","nullable":false},
            {"source_column_name":"access_valid_until","zelyra_type":"Timestamp","nullable":true}
        ]}]});
        let mut changed = BusinessAnalyzer::default();
        changed.scan("app/Models/BankConnection.php", &source("isPast()"));
        let rejected = changed.finish(&schema);
        assert_eq!(rejected["generated_function_count"], 0);
        assert_eq!(rejected["rejected"].as_array().unwrap().len(), 1);

        let incompatible = json!({"candidates":[{"table":"bank_connections","columns":[
            {"source_column_name":"status","zelyra_type":"String?","nullable":true},
            {"source_column_name":"access_valid_until","zelyra_type":"Timestamp","nullable":true}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/BankConnection.php", &source("isFuture()"));
        let rejected = analyzer.finish(&incompatible);
        assert_eq!(rejected["generated_function_count"], 0);
        assert!(
            rejected["rejected"][0]["reason"]
                .as_str()
                .unwrap()
                .contains("String?"),
            "{}",
            rejected["rejected"][0]["reason"]
        );
    }

    #[test]
    fn rejects_ambiguous_standalone_migration_field_evidence() {
        let source = r#"<?php
class Invoice extends Model {
    protected $table = 'invoices';
    public function isLocked(): bool { return !empty($this->locked_at); }
}
"#;
        let evidence = |file: &str| {
            json!({
                "table": "invoices",
                "source_column_name": "locked_at",
                "zelyra_type": "Timestamp",
                "nullable": true,
                "source_file": file,
                "source_span": {"start": {"line": 4}},
                "origin": "table_alteration"
            })
        };
        let mut create_candidate = evidence("database/migrations/2025_01_01_create_invoices.php");
        create_candidate["origin"] = json!("create_candidate");
        let schema = json!({
            "candidates": [],
            "field_evidence": [
                evidence("database/migrations/2026_01_01_add_locked_at.php"),
                evidence("database/migrations/2026_01_02_add_locked_at_again.php"),
                create_candidate
            ]
        });
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Invoice.php", source);
        let result = analyzer.finish(&schema);
        assert_eq!(result["generated_function_count"], 0);
        assert!(result["rejected"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("does not have one unique static source column"));
    }

    #[test]
    fn imports_integer_match_with_prefix_and_int_schema_evidence() {
        let source = r#"<?php
class Invoice extends Model {
    public function getDunningLevelLabelAttribute(): string
    {
        return match ($this->dunning_level) {
            1 => 'Zahlungserinnerung (Stufe 1)',
            2 => '1. Mahnung (Stufe 2)',
            default => 'Keine Mahnung (Stufe 0)',
        };
    }
}
"#;
        let schema = json!({"candidates":[{"table":"invoices","columns":[
            {"source_column_name":"dunning_level","zelyra_type":"Int","nullable":false}
        ]}]});
        let mut analyzer = BusinessAnalyzer::default();
        analyzer.scan("app/Models/Invoice.php", source);
        let report = analyzer.finish(&schema);
        assert_eq!(report["generated_function_count"], 1);
        let generated = report["candidate_source"].as_str().unwrap();
        assert!(generated.contains("field_value_0 == 1"));
        assert!(generated.contains("\"Keine Mahnung (Stufe 0)\""));
        let tokens = zelyra_lexer::lex(generated).expect("generated helper lexes");
        let program = zelyra_parser::parse(&tokens).expect("generated helper parses");
        for (level, expected) in [
            (0, "Keine Mahnung (Stufe 0)"),
            (1, "Zahlungserinnerung (Stufe 1)"),
            (2, "1. Mahnung (Stufe 2)"),
            (9, "Keine Mahnung (Stufe 0)"),
        ] {
            let actual = zelyra_runtime::execute_function(
                &program,
                "laravel_invoice_get_dunning_level_label_attribute",
                vec![zelyra_runtime::Value::Int(level)],
                None,
            )
            .unwrap();
            assert_eq!(actual, zelyra_runtime::Value::String(expected.into()));
        }
    }
}
