use super::{PhpSourceMap, SourceSpan};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone)]
struct Node {
    id: String,
    kind: &'static str,
    name: String,
    file: String,
    span: SourceSpan,
    route_context: Option<Value>,
    validation_rules: Option<Value>,
}

impl Node {
    fn new(kind: &'static str, name: &str, file: &str, span: SourceSpan) -> Self {
        Self {
            id: format!("{kind}:{file}:{}", span.start_byte),
            kind,
            name: name.into(),
            file: file.into(),
            span,
            route_context: None,
            validation_rules: None,
        }
    }

    fn json(&self) -> Value {
        let mut value = json!({
            "id": self.id,
            "kind": self.kind,
            "name": self.name,
            "file": self.file,
            "source_span": span_json(self.span)
        });
        if let Some(route_context) = &self.route_context {
            value["route_context"] = route_context.clone();
        }
        if let Some(validation_rules) = &self.validation_rules {
            value["validation_rules"] = validation_rules.clone();
        }
        value
    }
}

struct PendingEdge {
    kind: &'static str,
    from: Node,
    target_kind: &'static str,
    target_name: String,
    target_method: Option<String>,
    resolved_target: Option<Node>,
    unresolved_reason: &'static str,
}

#[derive(Default)]
pub(super) struct GraphBuilder {
    nodes: Vec<Node>,
    edges: Vec<PendingEdge>,
    controllers: BTreeMap<(String, String), Vec<Node>>,
    views: BTreeMap<String, Vec<Node>>,
    models: BTreeMap<String, Vec<Node>>,
    application_types: BTreeMap<String, Vec<Node>>,
    application_methods: BTreeMap<(String, String), Vec<Node>>,
    tables: BTreeMap<String, Vec<Node>>,
    table_creates: BTreeMap<String, Vec<Node>>,
}

impl GraphBuilder {
    pub(super) fn scan(&mut self, file: &str, source: &str) {
        let source_map = PhpSourceMap::new(source);
        self.scan_views(file, source, &source_map);
        self.scan_controllers(file, source, &source_map);
        self.scan_models(file, source, &source_map);
        self.scan_application_methods(file, source, &source_map);
        self.scan_migrations(file, source, &source_map);
        self.scan_routes(file, source, &source_map);
        self.scan_relationships(file, source, &source_map);
    }

    pub(super) fn finish(self) -> Value {
        let mut edges = Vec::new();
        for edge in self.edges {
            let direct = edge.resolved_target.clone().into_iter().collect::<Vec<_>>();
            let candidates = if direct.is_empty() {
                match edge.target_kind {
                    "controller_action" => self
                        .controllers
                        .get(&(
                            edge.target_name.clone(),
                            edge.target_method.clone().unwrap_or_default(),
                        ))
                        .cloned()
                        .unwrap_or_default(),
                    "model" => self
                        .models
                        .get(&edge.target_name)
                        .cloned()
                        .unwrap_or_default(),
                    "application_method" => self
                        .application_methods
                        .get(&(
                            edge.target_name.clone(),
                            edge.target_method.clone().unwrap_or_default(),
                        ))
                        .cloned()
                        .unwrap_or_default(),
                    "application_type" => self
                        .application_types
                        .get(&edge.target_name)
                        .cloned()
                        .unwrap_or_default(),
                    "view" => self
                        .views
                        .get(&edge.target_name)
                        .cloned()
                        .unwrap_or_default(),
                    "table" if edge.kind == "model_table" => self
                        .table_creates
                        .get(&edge.target_name)
                        .cloned()
                        .unwrap_or_default(),
                    "table" => self
                        .tables
                        .get(&edge.target_name)
                        .cloned()
                        .unwrap_or_default(),
                    _ => Vec::new(),
                }
            } else {
                direct
            };
            let resolved = candidates.len() == 1;
            let target = resolved.then(|| candidates[0].json());
            let unresolved_reason = if resolved {
                None
            } else if edge.target_kind == "controller_action" {
                let controller_exists = self
                    .controllers
                    .keys()
                    .any(|(controller, _)| controller == &edge.target_name);
                Some(if candidates.len() > 1 {
                    "multiple scanned controller declarations match this class and method"
                } else if controller_exists {
                    "controller class was scanned, but this method was not; inherited, trait-provided, and dynamic methods are not resolved"
                } else {
                    "controller class was not found among scanned source files"
                })
            } else if edge.target_kind == "application_method" {
                let application_type_exists =
                    self.application_types.contains_key(&edge.target_name)
                        || self
                            .application_methods
                            .keys()
                            .any(|(application_type, _)| application_type == &edge.target_name);
                Some(if candidates.len() > 1 {
                    "multiple scanned application declarations match this class and method"
                } else if application_type_exists {
                    "application class or contract was scanned, but this method was not declared; inherited, trait-provided, magic, and dynamic methods are not resolved"
                } else {
                    "application class or contract was not found among scanned source files"
                })
            } else {
                Some(edge.unresolved_reason)
            };
            let scanned_direct_methods = match edge.target_kind {
                "controller_action" if !resolved => self
                    .controllers
                    .keys()
                    .filter(|(controller, _)| controller == &edge.target_name)
                    .map(|(_, method)| method.as_str())
                    .collect::<Vec<_>>(),
                "application_method" if !resolved => self
                    .application_methods
                    .keys()
                    .filter(|(application_type, _)| application_type == &edge.target_name)
                    .map(|(_, method)| method.as_str())
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            };
            edges.push(json!({
                "kind": edge.kind,
                "from": edge.from.json(),
                "to": target,
                "resolution_candidate_count": candidates.len(),
                "scanned_direct_methods": scanned_direct_methods,
                "target": {
                    "kind": edge.target_kind,
                    "name": edge.target_name,
                    "method": edge.target_method
                },
                "status": if resolved { "resolved" } else { "unresolved" },
                "reason": unresolved_reason.map_or(Value::Null, |reason| json!(reason))
            }));
        }
        let mut nodes = self.nodes;
        nodes.sort_by(|left, right| left.id.cmp(&right.id));
        edges.sort_by(|left, right| {
            left["from"]["id"]
                .as_str()
                .cmp(&right["from"]["id"].as_str())
                .then_with(|| left["kind"].as_str().cmp(&right["kind"].as_str()))
        });
        let resolved_edges = edges
            .iter()
            .filter(|edge| edge["status"] == "resolved")
            .count();
        let unresolved_edges = edges.len().saturating_sub(resolved_edges);
        json!({
            "schema_version": "2",
            "scope": ["direct_controller_callback", "controller_group_action", "static_route_group_context", "resource_action_set", "inline_route_closure", "static_controller_view_literal", "static_inline_route_view_literal", "static_controller_validation_literals", "controller_injected_application_method", "controller_app_helper_method", "controller_static_eloquent_model_use", "controller_job_dispatch", "application_dependency_method", "application_static_eloquent_model_use", "application_job_dispatch", "application_interface_contracts", "model_relationship", "explicit_model_table_migration"],
            "resolution_semantics": "source_target_resolution_only_not_behavioral_equivalence",
            "nodes": nodes.iter().map(Node::json).collect::<Vec<_>>(),
            "edges": edges,
            "summary": {
                "nodes": nodes.len(),
                "edges": resolved_edges + unresolved_edges,
                "resolved_edges": resolved_edges,
                "unresolved_edges": unresolved_edges
            }
        })
    }

    fn scan_views(&mut self, file: &str, source: &str, source_map: &PhpSourceMap) {
        let Some(view_name) = blade_view_name(file) else {
            return;
        };
        let node = Node::new(
            "view",
            &view_name,
            file,
            source_map.span(source, 0..source.len()),
        );
        self.views.entry(view_name).or_default().push(node.clone());
        self.nodes.push(node);
    }

    fn scan_controllers(&mut self, file: &str, source: &str, source_map: &PhpSourceMap) {
        let class = super::compiled_regex(
            r"(?m)\bclass\s+(\w*Controller)\b[^\{]*\{",
            "controller class regex",
        );
        let method = super::compiled_regex(
            r"(?m)\bfunction\s+(&?\s*)([A-Za-z_]\w*)\s*\(",
            "controller method regex",
        );
        for capture in class.captures_iter(source) {
            let matched = capture.get(0).expect("class match");
            if !source_map.is_code(matched.start()) {
                continue;
            }
            let class_name = capture.get(1).expect("class name").as_str();
            let namespace = namespace_at(source, source_map, matched.start())
                .unwrap_or_else(|| format!("@unresolved:{}", file));
            let identity = qualified_name(&namespace, class_name);
            let open_brace = matched.end().saturating_sub(1);
            let Some(close_brace) = matching_brace(source.as_bytes(), source_map, open_brace)
            else {
                continue;
            };
            for action in method.captures_iter(&source[open_brace + 1..close_brace]) {
                let action_match = action.get(0).expect("method match");
                let start = open_brace + 1 + action_match.start();
                if !source_map.is_code(start) {
                    continue;
                }
                let method_name = action.get(2).expect("method name").as_str();
                let span = source_map.span(source, start..open_brace + 1 + action_match.end());
                let node = Node::new(
                    "controller_action",
                    &format!("{class_name}@{method_name}"),
                    file,
                    span,
                );
                self.controllers
                    .entry((identity.clone(), method_name.into()))
                    .or_default()
                    .push(node.clone());
                self.scan_controller_views(
                    source,
                    source_map,
                    &node,
                    action_match.start() + open_brace + 1,
                    action_match.end() + open_brace + 1,
                    close_brace,
                );
                self.scan_controller_validations(
                    source,
                    source_map,
                    &node,
                    start,
                    open_brace + 1 + action_match.end(),
                    close_brace,
                );
                self.scan_application_dependencies(
                    source,
                    source_map,
                    &node,
                    open_brace + 1 + action_match.start(),
                    open_brace + 1 + action_match.end(),
                    close_brace,
                    open_brace,
                );
                self.nodes.push(node);
            }
        }
    }

    fn scan_application_methods(&mut self, file: &str, source: &str, source_map: &PhpSourceMap) {
        if !file.starts_with("app/") || file.starts_with("app/Http/Controllers/") {
            return;
        }
        let class = super::compiled_regex(
            r"(?m)\b(class|interface|trait)\s+([A-Za-z_]\w*)\b[^\{]*\{",
            "application class regex",
        );
        let method = super::compiled_regex(
            r"(?m)\bfunction\s+(?:&\s*)?([A-Za-z_]\w*)\s*\(",
            "application method regex",
        );
        for capture in class.captures_iter(source) {
            let class_match = capture.get(0).expect("application class match");
            if !source_map.is_code(class_match.start()) {
                continue;
            }
            let declaration_kind = capture
                .get(1)
                .expect("application declaration kind")
                .as_str();
            let class_name = capture.get(2).expect("application class name").as_str();
            let namespace = namespace_at(source, source_map, class_match.start())
                .unwrap_or_else(|| format!("@unresolved:{file}"));
            let identity = qualified_name(&namespace, class_name);
            let open_brace = class_match.end().saturating_sub(1);
            let Some(close_brace) = matching_brace(source.as_bytes(), source_map, open_brace)
            else {
                continue;
            };
            if declaration_kind == "class" {
                let class_node = Node::new(
                    "application_class",
                    &identity,
                    file,
                    source_map.span(source, class_match.range()),
                );
                self.application_types
                    .entry(identity.clone())
                    .or_default()
                    .push(class_node.clone());
                self.nodes.push(class_node);
            }
            let body = &source[open_brace + 1..close_brace];
            for method_capture in method.captures_iter(body) {
                let method_match = method_capture.get(0).expect("application method match");
                let start = open_brace + 1 + method_match.start();
                if !source_map.is_code(start) {
                    continue;
                }
                let method_name = method_capture
                    .get(1)
                    .expect("application method name")
                    .as_str();
                let method_node = Node::new(
                    if declaration_kind == "interface" {
                        "application_method_contract"
                    } else {
                        "application_method"
                    },
                    &format!("{identity}@{method_name}"),
                    file,
                    source_map.span(source, start..open_brace + 1 + method_match.end()),
                );
                self.application_methods
                    .entry((identity.clone(), method_name.to_owned()))
                    .or_default()
                    .push(method_node.clone());
                self.scan_application_dependencies(
                    source,
                    source_map,
                    &method_node,
                    open_brace + 1 + method_match.start(),
                    open_brace + 1 + method_match.end(),
                    close_brace,
                    open_brace,
                );
                self.nodes.push(method_node);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn scan_application_dependencies(
        &mut self,
        source: &str,
        source_map: &PhpSourceMap,
        controller: &Node,
        method_start: usize,
        method_end: usize,
        class_end: usize,
        class_open: usize,
    ) {
        let Some((body_start, body_end)) =
            method_body_range(source, source_map, method_start, method_end, class_end)
        else {
            return;
        };
        let class_close =
            matching_brace(source.as_bytes(), source_map, class_open).unwrap_or(class_end);
        let property = super::compiled_regex(
            r"(?m)\b(?:public|protected|private)\s+(?:readonly\s+)?\??([A-Za-z_][A-Za-z0-9_\\]*)\s+\$(\w+)\b",
            "controller property type regex",
        );
        let mut property_types = HashMap::<String, String>::new();
        for capture in property.captures_iter(&source[class_open + 1..class_close]) {
            let matched = capture.get(0).expect("controller property match");
            let offset = class_open + 1 + matched.start();
            if !source_map.is_code(offset) {
                continue;
            }
            if let (Some(type_name), Some(property_name)) = (capture.get(1), capture.get(2)) {
                if let Some(resolved) =
                    resolve_class_reference(source, source_map, offset, type_name.as_str())
                {
                    property_types.insert(property_name.as_str().to_owned(), resolved);
                }
            }
        }
        let constructor = super::compiled_regex(
            r"(?s)function\s+__construct\s*\((.*?)\)",
            "controller constructor regex",
        );
        let parameter = super::compiled_regex(
            r"(?:^|,)\s*(?:\\?[A-Za-z_][A-Za-z0-9_\\]*\s+)\$([A-Za-z_]\w*)",
            "controller constructor dependency regex",
        );
        let assignment = super::compiled_regex(
            r"\$this\s*->\s*([A-Za-z_]\w*)\s*=\s*\$([A-Za-z_]\w*)",
            "constructor dependency assignment regex",
        );
        for capture in constructor.captures_iter(&source[class_open + 1..class_close]) {
            let matched = capture.get(0).expect("constructor match");
            let start = class_open + 1 + matched.start();
            if !source_map.is_code(start) {
                continue;
            }
            let Some(parameters) = capture.get(1) else {
                continue;
            };
            for dependency in parameter.captures_iter(parameters.as_str()) {
                let Some(variable) = dependency.get(1) else {
                    continue;
                };
                let type_text = parameters.as_str();
                let preceding = &type_text[..variable.start()];
                let type_name = preceding
                    .trim_end()
                    .trim_end_matches('$')
                    .rsplit(|character: char| character == ',' || character.is_whitespace())
                    .next()
                    .unwrap_or_default();
                if type_name.is_empty()
                    || type_name == "&"
                    || type_name == "..."
                    || type_name.contains('|')
                {
                    continue;
                }
                let parameter_offset = start + parameters.start() + variable.start();
                let Some(resolved) =
                    resolve_class_reference(source, source_map, parameter_offset, type_name)
                else {
                    continue;
                };
                let variable_name = variable.as_str();
                for assignment_capture in assignment.captures_iter(&source[start..class_close]) {
                    let assignment_match = assignment_capture.get(0).expect("assignment match");
                    if !source_map.is_code(start + assignment_match.start()) {
                        continue;
                    }
                    if assignment_capture
                        .get(2)
                        .is_some_and(|name| name.as_str() == variable_name)
                    {
                        if let Some(property_name) = assignment_capture.get(1) {
                            property_types
                                .insert(property_name.as_str().to_owned(), resolved.clone());
                        }
                    }
                }
            }
        }

        let body = &source[body_start..body_end];
        let property_call = super::compiled_regex(
            r"\$this\s*->\s*([A-Za-z_]\w*)\s*->\s*([A-Za-z_]\w*)\s*\(",
            "controller injected dependency call regex",
        );
        for capture in property_call.captures_iter(body) {
            let matched = capture.get(0).expect("dependency call match");
            let start = body_start + matched.start();
            if !source_map.is_code(start) {
                continue;
            }
            let Some(property_name) = capture.get(1).map(|value| value.as_str()) else {
                continue;
            };
            let Some(class_name) = property_types.get(property_name) else {
                continue;
            };
            let method_name = capture.get(2).expect("dependency method").as_str();
            self.edges.push(PendingEdge {
                kind: if controller.kind == "controller_action" {
                    "controller_dependency_call"
                } else {
                    "application_dependency_call"
                },
                from: controller.clone(),
                target_kind: "application_method",
                target_name: class_name.clone(),
                target_method: Some(method_name.into()),
                resolved_target: None,
                unresolved_reason: "injected dependency method did not resolve to exactly one scanned application method",
            });
        }

        let app_call = super::compiled_regex(
            r"\bapp\s*\(\s*(\\?[A-Za-z_][A-Za-z0-9_\\]*)\s*::\s*class\s*\)\s*->\s*([A-Za-z_]\w*)\s*\(",
            "controller app() dependency call regex",
        );
        for capture in app_call.captures_iter(body) {
            let matched = capture.get(0).expect("app dependency call match");
            let start = body_start + matched.start();
            if !source_map.is_code(start) {
                continue;
            }
            let class_reference = capture.get(1).expect("app dependency class");
            let Some(class_name) = resolve_class_reference(
                source,
                source_map,
                body_start + class_reference.start(),
                class_reference.as_str(),
            ) else {
                continue;
            };
            let method_name = capture.get(2).expect("app dependency method").as_str();
            self.edges.push(PendingEdge {
                kind: if controller.kind == "controller_action" {
                    "controller_dependency_call"
                } else {
                    "application_dependency_call"
                },
                from: controller.clone(),
                target_kind: "application_method",
                target_name: class_name,
                target_method: Some(method_name.into()),
                resolved_target: None,
                unresolved_reason: "app() dependency method did not resolve to exactly one scanned application method",
            });
        }

        let model_call = super::compiled_regex(
            r"([A-Za-z_\\][A-Za-z0-9_\\]*)\s*::\s*([A-Za-z_]\w*)\s*\(",
            "controller static model call regex",
        );
        for capture in model_call.captures_iter(body) {
            let matched = capture.get(0).expect("static class call match");
            let start = body_start + matched.start();
            if !source_map.is_code(start) {
                continue;
            }
            let reference = capture.get(1).expect("static class reference");
            if matches!(reference.as_str(), "self" | "static" | "parent") {
                continue;
            }
            let Some(class_name) = resolve_class_reference(
                source,
                source_map,
                body_start + reference.start(),
                reference.as_str(),
            ) else {
                continue;
            };
            let method_name = capture.get(2).expect("static method name").as_str();
            if class_name.starts_with("App\\Models\\") {
                self.edges.push(PendingEdge {
                    kind: if controller.kind == "controller_action" {
                        "controller_model_use"
                    } else {
                        "application_model_use"
                    },
                    from: controller.clone(),
                    target_kind: "model",
                    target_name: class_name,
                    target_method: None,
                    resolved_target: None,
                    unresolved_reason: "statically referenced Eloquent model did not resolve to exactly one scanned model",
                });
            } else if class_name.starts_with("App\\Jobs\\") && method_name == "dispatch" {
                self.edges.push(PendingEdge {
                    kind: if controller.kind == "controller_action" {
                        "controller_job_dispatch"
                    } else {
                        "application_job_dispatch"
                    },
                    from: controller.clone(),
                    target_kind: "application_type",
                    target_name: class_name,
                    target_method: Some(method_name.into()),
                    resolved_target: None,
                    unresolved_reason:
                        "queued job class did not resolve to exactly one scanned class",
                });
            } else if class_name.starts_with("App\\") {
                self.edges.push(PendingEdge {
                    kind: if controller.kind == "controller_action" {
                        "controller_dependency_call"
                    } else {
                        "application_dependency_call"
                    },
                    from: controller.clone(),
                    target_kind: "application_method",
                    target_name: class_name,
                    target_method: Some(method_name.into()),
                    resolved_target: None,
                    unresolved_reason: "statically referenced application method did not resolve to exactly one scanned method",
                });
            }
        }
    }

    fn scan_controller_views(
        &mut self,
        source: &str,
        source_map: &PhpSourceMap,
        controller: &Node,
        method_start: usize,
        method_end: usize,
        class_end: usize,
    ) {
        let Some((body_start, body_end)) =
            method_body_range(source, source_map, method_start, method_end, class_end)
        else {
            return;
        };
        let body = &source[body_start..body_end];
        let call = super::compiled_regex(
            r#"(?i)\b(?:view\s*\(|View\s*::\s*make\s*\()\s*(?:'([A-Za-z0-9_.-]+)'|"([A-Za-z0-9_.-]+)")"#,
            "static controller view reference regex",
        );
        for capture in call.captures_iter(body) {
            let matched = capture.get(0).expect("view call match");
            let start = body_start + matched.start();
            if !source_map.is_code(start) {
                continue;
            }
            let view_name = capture
                .get(1)
                .or_else(|| capture.get(2))
                .expect("view string literal")
                .as_str();
            self.edges.push(PendingEdge {
                kind: "controller_view",
                from: controller.clone(),
                target_kind: "view",
                target_name: view_name.into(),
                target_method: None,
                resolved_target: None,
                unresolved_reason:
                    "static controller view name did not resolve to exactly one Blade template",
            });
        }
    }

    fn scan_controller_validations(
        &mut self,
        source: &str,
        source_map: &PhpSourceMap,
        controller: &Node,
        method_start: usize,
        method_end: usize,
        class_end: usize,
    ) {
        let Some((body_start, body_end)) =
            method_body_range(source, source_map, method_start, method_end, class_end)
        else {
            return;
        };
        let body = &source[body_start..body_end];
        let call = super::compiled_regex(r"(?i)->\s*validate\s*\(", "validation call regex");
        for matched in call.find_iter(body) {
            let start = body_start + matched.start();
            if !source_map.is_code(start) {
                continue;
            }
            let Some(open_paren) = (start..body_start + matched.end())
                .find(|offset| source.as_bytes().get(*offset) == Some(&b'('))
            else {
                continue;
            };
            let Some(call_end) =
                matching_delimiter(source.as_bytes(), source_map, open_paren, b'(', b')')
            else {
                continue;
            };
            let array_start = skip_php_space(source, source_map, open_paren + 1, call_end);
            let (metadata, node_end) = if source.as_bytes().get(array_start) == Some(&b'[') {
                let Some(array_end) =
                    matching_delimiter(source.as_bytes(), source_map, array_start, b'[', b']')
                else {
                    continue;
                };
                (
                    parse_static_validation_rules(source, source_map, array_start, array_end),
                    array_end + 1,
                )
            } else {
                (
                    json!({
                        "parse_mode": "unsupported_expression",
                        "complete": false,
                        "fields": [],
                        "semantics": "validation rules were not statically enumerated"
                    }),
                    call_end + 1,
                )
            };
            let mut node = Node::new(
                "validation",
                &format!("{} validation", controller.name),
                &controller.file,
                source_map.span(source, start..node_end),
            );
            node.validation_rules = Some(metadata);
            self.nodes.push(node.clone());
            self.edges.push(PendingEdge {
                kind: "controller_validation",
                from: controller.clone(),
                target_kind: "validation",
                target_name: node.name.clone(),
                target_method: None,
                resolved_target: Some(node),
                unresolved_reason: "static validation metadata was not available",
            });
        }
    }

    fn scan_models(&mut self, file: &str, source: &str, source_map: &PhpSourceMap) {
        let model = super::compiled_regex(
            r"(?m)\bclass\s+([A-Za-z_]\w*)[^\{]*\bextends\s+(?:(?:\\?Illuminate\\Database\\Eloquent\\)?Model|(?:\\?Illuminate\\Foundation\\Auth\\)?Authenticatable)\b[^\{]*\{",
            "model class regex",
        );
        let table = super::compiled_regex(
            r#"\$table\s*=\s*['"]([A-Za-z_]\w*)['"]"#,
            "model table regex",
        );
        for capture in model.captures_iter(source) {
            let matched = capture.get(0).expect("model match");
            if !source_map.is_code(matched.start()) {
                continue;
            }
            let name = capture.get(1).expect("model name").as_str();
            let namespace = namespace_at(source, source_map, matched.start()).unwrap_or_default();
            let qualified_model_name = qualified_name(&namespace, name);
            let open_brace = matched.end().saturating_sub(1);
            let Some(close_brace) = matching_brace(source.as_bytes(), source_map, open_brace)
            else {
                continue;
            };
            let model_span = source_map.span(source, matched.start()..matched.end());
            let model_node = Node::new("model", name, file, model_span);
            self.models
                .entry(name.into())
                .or_default()
                .push(model_node.clone());
            if qualified_model_name != name {
                self.models
                    .entry(qualified_model_name)
                    .or_default()
                    .push(model_node.clone());
            }
            self.nodes.push(model_node.clone());
            let body = &source[open_brace + 1..close_brace];
            let explicit_table = table.captures_iter(body).find_map(|table_capture| {
                let table_match = table_capture.get(0)?;
                let start = open_brace + 1 + table_match.start();
                source_map
                    .is_code(start)
                    .then(|| table_capture.get(1).map(|value| value.as_str().to_owned()))
                    .flatten()
            });
            if let Some(table_name) = explicit_table {
                self.edges.push(PendingEdge {
                    kind: "model_table",
                    from: model_node,
                    target_kind: "table",
                    target_name: table_name,
                    target_method: None,
                    resolved_target: None,
                    unresolved_reason: "the explicit model table does not uniquely match a scanned schema migration",
                });
            }
        }
    }

    fn scan_migrations(&mut self, file: &str, source: &str, source_map: &PhpSourceMap) {
        let migration = super::compiled_regex(
            r#"Schema\s*::\s*(create|table)\s*\(\s*['"]([A-Za-z_]\w*)['"]"#,
            "migration table regex",
        );
        for capture in migration.captures_iter(source) {
            let matched = capture.get(0).expect("migration match");
            if !source_map.is_code(matched.start()) {
                continue;
            }
            let operation = capture.get(1).expect("migration operation").as_str();
            let table_name = capture.get(2).expect("table name").as_str();
            let node = Node::new(
                "table",
                table_name,
                file,
                source_map.span(source, matched.range()),
            );
            self.tables
                .entry(table_name.into())
                .or_default()
                .push(node.clone());
            if operation == "create" {
                self.table_creates
                    .entry(table_name.into())
                    .or_default()
                    .push(node.clone());
            }
            self.nodes.push(node);
        }
    }

    fn scan_routes(&mut self, file: &str, source: &str, source_map: &PhpSourceMap) {
        let route = super::compiled_regex(
            r#"Route\s*::\s*(get|post|put|patch|delete|options|any|resource|apiResource)\s*\(\s*['"]([^'"]+)['"]"#,
            "route graph regex",
        );
        let action = super::compiled_regex(
            r#"^\s*,\s*\[\s*([A-Za-z_\\][A-Za-z0-9_\\]*)\s*::\s*class\s*,\s*['"]([A-Za-z_]\w*)['"]"#,
            "route action regex",
        );
        let resource_action = super::compiled_regex(
            r"^\s*,\s*([A-Za-z_\\][A-Za-z0-9_\\]*)\s*::\s*class\b",
            "resource controller regex",
        );
        let closure = super::compiled_regex(r"^\s*,\s*(function|fn)\b", "route closure regex");
        let controller_group = super::compiled_regex(
            r"Route\s*::[^;{}]*?->\s*controller\s*\(\s*([A-Za-z_\\][A-Za-z0-9_\\]*)\s*::\s*class\s*\)\s*->\s*group\s*\(\s*function\s*\([^)]*\)\s*\{",
            "controller route group regex",
        );
        let code = code_only(source, source_map);
        let route_groups = route_group_prefixes(source, source_map, &code);
        let grouped_action = super::compiled_regex(
            r#"^\s*,\s*['"]([A-Za-z_]\w*)['"]"#,
            "controller-group action regex",
        );
        let mut controller_groups = Vec::new();
        for capture in controller_group.captures_iter(&code) {
            let matched = capture.get(0).expect("controller group match");
            if !source_map.is_code(matched.start()) {
                continue;
            }
            let controller = capture.get(1).expect("controller class").as_str();
            let controller = resolve_class_reference(
                source,
                source_map,
                matched.start() + capture.get(1).expect("controller class").start(),
                controller,
            )
            .unwrap_or_else(|| format!("@unresolved:{controller}"));
            let open_brace = matched.end().saturating_sub(1);
            let Some(close_brace) = matching_brace(source.as_bytes(), source_map, open_brace)
            else {
                continue;
            };
            controller_groups.push((open_brace, close_brace, controller));
        }
        for capture in route.captures_iter(source) {
            let matched = capture.get(0).expect("route match");
            if !source_map.is_code(matched.start()) {
                continue;
            }
            let method = capture.get(1).expect("HTTP method").as_str();
            let declared_path = capture.get(2).expect("route path").as_str();
            let effective_path =
                effective_route_path(declared_path, matched.start(), &route_groups);
            let path = effective_path.as_deref().unwrap_or(declared_path);
            let route_name =
                route_name_after_call(source, source_map, &code, matched.start(), matched.end());
            let route_middleware = route_middleware_after_call(
                source,
                source_map,
                &code,
                matched.start(),
                matched.end(),
            );
            let mut route_node = Node::new(
                "route",
                &format!("{method} {path}"),
                file,
                source_map.span(source, matched.range()),
            );
            route_node.route_context = Some(route_context_json(
                declared_path,
                effective_path.as_deref(),
                &route_name,
                &route_middleware,
                matched.start(),
                &route_groups,
            ));
            self.nodes.push(route_node.clone());
            let trailing = &source[matched.end()..];
            if matches!(method, "resource" | "apiResource") {
                if let Some((controller, class_offset)) =
                    resource_action.captures(trailing).and_then(|capture| {
                        let class = capture.get(1)?;
                        Some((class.as_str(), class.start()))
                    })
                {
                    let controller = resolve_class_reference(
                        source,
                        source_map,
                        matched.end() + class_offset,
                        controller,
                    )
                    .unwrap_or_else(|| format!("@unresolved:{controller}"));
                    let actions: &[&str] = if method == "resource" {
                        &[
                            "index", "create", "store", "show", "edit", "update", "destroy",
                        ]
                    } else {
                        &["index", "store", "show", "update", "destroy"]
                    };
                    for action_name in actions {
                        self.edges.push(PendingEdge {
                            kind: "route_controller_action",
                            from: route_node.clone(),
                            target_kind: "controller_action",
                            target_name: controller.clone(),
                            target_method: Some((*action_name).into()),
                            resolved_target: None,
                            unresolved_reason:
                                "resource controller action did not resolve to exactly one method",
                        });
                    }
                    continue;
                }
            }
            if let Some((_, _, controller)) = controller_groups
                .iter()
                .filter(|(start, end, _)| matched.start() > *start && matched.start() < *end)
                .min_by_key(|(start, end, _)| end - start)
            {
                if let Some(action_name) = grouped_action
                    .captures(trailing)
                    .and_then(|action| action.get(1))
                    .map(|action| action.as_str().to_owned())
                {
                    self.edges.push(PendingEdge {
                        kind: "route_controller_action",
                        from: route_node,
                        target_kind: "controller_action",
                        target_name: controller.clone(),
                        target_method: Some(action_name),
                        resolved_target: None,
                        unresolved_reason:
                            "controller-group route action did not resolve to exactly one method",
                    });
                    continue;
                }
            }
            let (controller, action_name) = action
                .captures(trailing)
                .and_then(|action| {
                    let class = action.get(1)?;
                    let method = action.get(2)?;
                    let qualified = resolve_class_reference(
                        source,
                        source_map,
                        matched.end() + class.start(),
                        class.as_str(),
                    )
                    .unwrap_or_else(|| format!("@unresolved:{}", class.as_str()));
                    Some((qualified, method.as_str().to_owned()))
                })
                .unwrap_or_default();
            let (target_kind, target, target_method, resolved_target, reason) = if controller
                .is_empty()
            {
                if let Some(handler) =
                    inline_handler(file, source, source_map, matched.end(), path, &closure)
                {
                    let name = handler.name.clone();
                    self.scan_inline_handler_views(source, source_map, &handler);
                    self.nodes.push(handler.clone());
                    ("inline_handler", name, None, Some(handler), "")
                } else {
                    (
                        "unsupported_handler",
                        format!("{method} {path}"),
                        None,
                        None,
                        "route action is not a statically recognized controller-class callback or inline closure",
                    )
                }
            } else {
                (
                    "controller_action",
                    controller,
                    Some(action_name),
                    None,
                    "controller action did not resolve uniquely",
                )
            };
            self.edges.push(PendingEdge {
                kind: "route_controller_action",
                from: route_node,
                target_kind,
                target_name: target,
                target_method,
                resolved_target,
                unresolved_reason: reason,
            });
        }
    }

    fn scan_inline_handler_views(
        &mut self,
        source: &str,
        source_map: &PhpSourceMap,
        handler: &Node,
    ) {
        let Some(body) = source.get(handler.span.start_byte..handler.span.end_byte) else {
            return;
        };
        let view_call = super::compiled_regex(
            r#"(?i)\b(?:view\s*\(|View\s*::\s*make\s*\()\s*(?:'([A-Za-z0-9_.-]+)'|"([A-Za-z0-9_.-]+)")"#,
            "static inline route view reference regex",
        );
        for capture in view_call.captures_iter(body) {
            let matched = capture.get(0).expect("inline view call match");
            let start = handler.span.start_byte + matched.start();
            if !source_map.is_code(start) {
                continue;
            }
            let view_name = capture
                .get(1)
                .or_else(|| capture.get(2))
                .expect("view string literal")
                .as_str();
            self.edges.push(PendingEdge {
                kind: "inline_handler_view",
                from: handler.clone(),
                target_kind: "view",
                target_name: view_name.into(),
                target_method: None,
                resolved_target: None,
                unresolved_reason:
                    "static inline-route view name did not resolve to exactly one Blade template",
            });
        }
    }

    fn scan_relationships(&mut self, file: &str, source: &str, source_map: &PhpSourceMap) {
        let model = super::compiled_regex(
            r"(?m)\bclass\s+([A-Za-z_]\w*)[^\{]*\bextends\s+(?:(?:\\?Illuminate\\Database\\Eloquent\\)?Model|(?:\\?Illuminate\\Foundation\\Auth\\)?Authenticatable)\b[^\{]*\{",
            "model class regex",
        );
        let relation = super::compiled_regex(
            r"(?s)function\s+([A-Za-z_]\w*)\s*\([^)]*\)\s*\{[^}]*?return\s+\$this\s*->\s*(belongsTo|hasOne|hasMany|belongsToMany|hasManyThrough)\s*\(\s*([A-Za-z_\\][A-Za-z0-9_\\]*)\s*::\s*class",
            "relationship regex",
        );
        for capture in model.captures_iter(source) {
            let class_match = capture.get(0).expect("model match");
            if !source_map.is_code(class_match.start()) {
                continue;
            }
            let source_model = capture.get(1).expect("model name").as_str();
            let open_brace = class_match.end().saturating_sub(1);
            let Some(close_brace) = matching_brace(source.as_bytes(), source_map, open_brace)
            else {
                continue;
            };
            for relation_capture in relation.captures_iter(&source[open_brace + 1..close_brace]) {
                let matched = relation_capture.get(0).expect("relationship match");
                let start = open_brace + 1 + matched.start();
                if !source_map.is_code(start) {
                    continue;
                }
                let relation_name = relation_capture.get(1).expect("relation name").as_str();
                let relation_type = relation_capture.get(2).expect("relation type").as_str();
                let target = relation_capture.get(3).expect("relation target").as_str();
                let target = target.rsplit('\\').next().unwrap_or(target);
                let from = Node::new(
                    "model",
                    source_model,
                    file,
                    source_map.span(source, class_match.range()),
                );
                self.edges.push(PendingEdge {
                    kind: "model_relationship",
                    from,
                    target_kind: "model",
                    target_name: target.into(),
                    target_method: None,
                    resolved_target: None,
                    unresolved_reason:
                        "relationship target did not resolve to exactly one scanned Eloquent model",
                });
                let relation_node = Node::new(
                    "relationship",
                    &format!("{source_model}.{relation_name}:{relation_type}"),
                    file,
                    source_map.span(source, start..start + matched.end()),
                );
                self.nodes.push(relation_node);
            }
        }
    }
}

fn skip_php_space(source: &str, source_map: &PhpSourceMap, mut offset: usize, end: usize) -> usize {
    let bytes = source.as_bytes();
    loop {
        while offset < end && bytes[offset].is_ascii_whitespace() {
            offset += 1;
        }
        if offset >= end {
            break;
        }
        let comment_start = !source_map.is_code(offset);
        if comment_start && bytes[offset] == b'#' {
            offset = (offset..end)
                .find(|index| matches!(bytes[*index], b'\n' | b'\r'))
                .unwrap_or(end);
        } else if comment_start && bytes.get(offset..offset.saturating_add(2)) == Some(b"//") {
            offset = (offset..end)
                .find(|index| matches!(bytes[*index], b'\n' | b'\r'))
                .unwrap_or(end);
        } else if comment_start && bytes.get(offset..offset.saturating_add(2)) == Some(b"/*") {
            let comment_end = (offset + 2..end.saturating_sub(1))
                .find(|index| bytes.get(*index..*index + 2) == Some(b"*/"));
            offset = comment_end.map_or(end, |index| index + 2);
        } else {
            break;
        }
    }
    offset
}

fn php_quoted_literal(source: &str, start: usize, end: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    let quote = *bytes.get(start)?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    let mut offset = start + 1;
    while offset < end {
        match bytes[offset] {
            b'\\' => offset = (offset + 2).min(end),
            byte if byte == quote => {
                return Some((source[start + 1..offset].to_owned(), offset + 1));
            }
            _ => offset += 1,
        }
    }
    None
}

fn validation_rule_names(value: &str) -> (Vec<Value>, bool) {
    const RULES: &[&str] = &[
        "accepted",
        "active_url",
        "after",
        "after_or_equal",
        "alpha",
        "alpha_dash",
        "alpha_num",
        "array",
        "before",
        "before_or_equal",
        "boolean",
        "confirmed",
        "date",
        "date_equals",
        "different",
        "digits",
        "digits_between",
        "email",
        "exists",
        "file",
        "image",
        "in",
        "integer",
        "ip",
        "json",
        "max",
        "mimes",
        "min",
        "nullable",
        "numeric",
        "present",
        "regex",
        "required",
        "required_if",
        "required_unless",
        "same",
        "size",
        "sometimes",
        "string",
        "unique",
        "url",
        "uuid",
    ];
    let mut rules = Vec::new();
    let mut complete = !value.is_empty();
    for part in split_validation_rule_parts(value) {
        let (name, parameter) = part
            .split_once(':')
            .map_or((part.as_str(), None), |(name, parameter)| {
                (name, Some(parameter.trim()))
            });
        let name = name.trim();
        if !name.is_empty() && RULES.contains(&name) {
            if parameter.is_some_and(str::is_empty) {
                complete = false;
            }
            let safe_parameter = parameter.filter(|value| safe_validation_parameter(name, value));
            let parameter_kind = safe_parameter.and_then(|_| {
                matches!(
                    name,
                    "after"
                        | "after_or_equal"
                        | "before"
                        | "before_or_equal"
                        | "date_equals"
                        | "same"
                        | "different"
                )
                .then_some("identifier_token_uninterpreted")
            });
            if parameter.is_some() && safe_parameter.is_none() {
                complete = false;
            }
            rules.push(json!({
                "name": name,
                "has_parameters": parameter.is_some(),
                "parameter": safe_parameter,
                "parameter_kind": parameter_kind,
                "parameter_redacted": parameter.is_some() && safe_parameter.is_none()
            }));
        } else {
            complete = false;
        }
    }
    (rules, complete)
}

fn safe_validation_parameter(rule: &str, value: &str) -> bool {
    if value.is_empty() || value.len() > 128 {
        return false;
    }
    match rule {
        "min" | "max" => safe_decimal_parameter(value),
        "in" => safe_choice_parameter(value),
        "size" | "digits" => value.bytes().all(|byte| byte.is_ascii_digit()),
        "digits_between" => {
            let mut bounds = value.split(',');
            matches!(
                (bounds.next(), bounds.next(), bounds.next()),
                (Some(min), Some(max), None)
                    if !min.is_empty()
                        && !max.is_empty()
                        && min.bytes().all(|byte| byte.is_ascii_digit())
                        && max.bytes().all(|byte| byte.is_ascii_digit())
            )
        }
        "exists" | "unique" => {
            let mut identifiers = value.split(',');
            matches!(
                (identifiers.next(), identifiers.next(), identifiers.next()),
                (Some(table), Some(column), None)
                    if safe_sql_identifier(table) && safe_sql_identifier(column)
            )
        }
        "after" | "after_or_equal" | "before" | "before_or_equal" | "date_equals" | "same"
        | "different" => {
            let mut bytes = value.bytes();
            bytes
                .next()
                .is_some_and(|first| first == b'_' || first.is_ascii_alphabetic())
                && bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
        }
        _ => false,
    }
}

fn safe_choice_parameter(value: &str) -> bool {
    let choices = value.split(',').collect::<Vec<_>>();
    (1..=64).contains(&choices.len())
        && choices.iter().all(|choice| {
            !choice.is_empty()
                && choice.len() <= 64
                && choice
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        })
}

fn safe_decimal_parameter(value: &str) -> bool {
    let unsigned = value
        .strip_prefix('-')
        .or_else(|| value.strip_prefix('+'))
        .unwrap_or(value);
    let mut parts = unsigned.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next();
    !whole.is_empty()
        && whole.bytes().all(|byte| byte.is_ascii_digit())
        && parts.next().is_none()
        && fraction.is_none_or(|fraction| {
            !fraction.is_empty() && fraction.bytes().all(|byte| byte.is_ascii_digit())
        })
}

fn safe_sql_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.split('.').all(|part| {
            let mut bytes = part.bytes();
            bytes
                .next()
                .is_some_and(|first| first == b'_' || first.is_ascii_alphabetic())
                && bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
        })
}

fn split_validation_rule_parts(value: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            current.push(character);
            escaped = false;
        } else if character == '\\' {
            current.push(character);
            escaped = true;
        } else if character == '|' {
            parts.push(std::mem::take(&mut current));
        } else {
            current.push(character);
        }
    }
    parts.push(current);
    parts
}

fn skip_php_expression(
    source: &str,
    source_map: &PhpSourceMap,
    mut offset: usize,
    end: usize,
) -> usize {
    let bytes = source.as_bytes();
    let mut depth = [0_usize; 3];
    while offset < end {
        if matches!(bytes[offset], b'\'' | b'"') {
            if let Some((_, after)) = php_quoted_literal(source, offset, end) {
                offset = after;
                continue;
            }
        }
        if !source_map.is_code(offset) {
            offset += 1;
            continue;
        }
        match bytes[offset] {
            b'(' => depth[0] += 1,
            b')' if depth[0] > 0 => depth[0] -= 1,
            b'[' => depth[1] += 1,
            b']' if depth[1] > 0 => depth[1] -= 1,
            b'{' => depth[2] += 1,
            b'}' if depth[2] > 0 => depth[2] -= 1,
            b',' if depth == [0, 0, 0] => break,
            _ => {}
        }
        offset += 1;
    }
    offset
}

fn parse_static_validation_rules(
    source: &str,
    source_map: &PhpSourceMap,
    array_start: usize,
    array_end: usize,
) -> Value {
    let bytes = source.as_bytes();
    let mut offset = array_start + 1;
    let mut fields = Vec::new();
    let mut complete = true;
    let mut unsupported_entries = 0_usize;
    while offset < array_end {
        offset = skip_php_space(source, source_map, offset, array_end);
        if offset >= array_end {
            break;
        }
        if bytes[offset] == b',' {
            offset += 1;
            continue;
        }
        let key_start = offset;
        let Some((field_name, key_end)) = php_quoted_literal(source, offset, array_end) else {
            complete = false;
            unsupported_entries += 1;
            offset = skip_php_expression(source, source_map, offset, array_end);
            if bytes.get(offset) == Some(&b',') {
                offset += 1;
            }
            continue;
        };
        offset = skip_php_space(source, source_map, key_end, array_end);
        if bytes.get(offset..offset.saturating_add(2)) != Some(b"=>") {
            complete = false;
            unsupported_entries += 1;
            offset = skip_php_expression(source, source_map, key_start, array_end);
            if bytes.get(offset) == Some(&b',') {
                offset += 1;
            }
            continue;
        }
        offset += 2;
        offset = skip_php_space(source, source_map, offset, array_end);
        let mut field_complete = true;
        let mut rules = Vec::new();
        match bytes.get(offset) {
            Some(b'\'' | b'"') => {
                if let Some((rule_text, after)) = php_quoted_literal(source, offset, array_end) {
                    let (parsed, rules_complete) = validation_rule_names(&rule_text);
                    rules = parsed;
                    field_complete &= rules_complete;
                    offset = after;
                } else {
                    field_complete = false;
                }
            }
            Some(b'[') => {
                if let Some(rules_end) = matching_delimiter(bytes, source_map, offset, b'[', b']') {
                    let mut rule_offset = offset + 1;
                    while rule_offset < rules_end {
                        rule_offset = skip_php_space(source, source_map, rule_offset, rules_end);
                        if rule_offset >= rules_end {
                            break;
                        }
                        if bytes[rule_offset] == b',' {
                            rule_offset += 1;
                            continue;
                        }
                        if let Some((rule_text, after)) =
                            php_quoted_literal(source, rule_offset, rules_end)
                        {
                            let (parsed, rules_complete) = validation_rule_names(&rule_text);
                            rules.extend(parsed);
                            field_complete &= rules_complete;
                            rule_offset = after;
                        } else {
                            field_complete = false;
                            let next =
                                skip_php_expression(source, source_map, rule_offset, rules_end);
                            if next <= rule_offset {
                                rule_offset += 1;
                            } else {
                                rule_offset = next;
                            }
                        }
                        if bytes.get(rule_offset) == Some(&b',') {
                            rule_offset += 1;
                        }
                    }
                    offset = rules_end + 1;
                } else {
                    field_complete = false;
                    complete = false;
                    unsupported_entries += 1;
                    offset = array_end;
                }
            }
            _ => {
                field_complete = false;
                offset = skip_php_expression(source, source_map, offset, array_end);
            }
        }
        let safe_field_name = !field_name.is_empty()
            && field_name
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "_.*-".contains(character))
            && !field_name.contains('\\');
        if !safe_field_name {
            field_complete = false;
        }
        complete &= field_complete && safe_field_name;
        if safe_field_name {
            fields.push(json!({
                "field": field_name,
                "field_span": span_json(source_map.span(source, key_start..key_end)),
                "rules": rules,
                "complete": field_complete
            }));
        }
        offset = skip_php_space(source, source_map, offset, array_end);
        if bytes.get(offset) == Some(&b',') {
            offset += 1;
        } else if offset < array_end {
            complete = false;
            unsupported_entries += 1;
            offset = skip_php_expression(source, source_map, offset, array_end);
            if bytes.get(offset) == Some(&b',') {
                offset += 1;
            }
        }
    }
    json!({
        "parse_mode": "static_array_literal_subset",
        "complete": complete,
        "fields": fields,
        "unsupported_entry_count": unsupported_entries,
        "semantics": "literal rule names and allowlisted parameters only (numeric bounds, choices, schema identifiers, and uninterpreted comparison-rule identifier tokens); other parameters are redacted; conditional rules, custom rules, authorization, and Laravel runtime behavior are not evaluated"
    })
}

fn blade_view_name(file: &str) -> Option<String> {
    let relative = file.strip_prefix("resources/views/")?;
    let relative = relative.strip_suffix(".blade.php")?;
    if relative.is_empty()
        || relative.split('/').any(|part| {
            part.is_empty()
                || !part.chars().all(|character| {
                    character.is_ascii_alphanumeric() || character == '_' || character == '-'
                })
        })
    {
        return None;
    }
    Some(relative.replace('/', "."))
}

fn method_body_range(
    source: &str,
    source_map: &PhpSourceMap,
    method_start: usize,
    method_end: usize,
    enclosing_end: usize,
) -> Option<(usize, usize)> {
    let open_paren = method_end.checked_sub(1)?;
    let parameter_end = matching_delimiter(source.as_bytes(), source_map, open_paren, b'(', b')')?;
    let body_start = (parameter_end + 1..enclosing_end).find(|offset| {
        source_map.is_code(*offset) && matches!(source.as_bytes().get(*offset), Some(b'{' | b';'))
    })?;
    if source.as_bytes().get(body_start) != Some(&b'{') {
        return None;
    }
    let body_end = matching_brace(source.as_bytes(), source_map, body_start)?;
    (method_start < body_start && body_end <= enclosing_end).then_some((body_start + 1, body_end))
}

#[derive(Clone, Debug)]
struct RouteGroupPrefix {
    body_start: usize,
    body_end: usize,
    source_span: SourceSpan,
    prefix: Option<String>,
    name_prefix: Option<String>,
    middleware: Vec<String>,
    middleware_complete: bool,
}

struct RouteNameContext {
    declared: Option<String>,
    seen: bool,
    complete: bool,
}

struct RouteMiddlewareContext {
    declared: Vec<String>,
    seen: bool,
    complete: bool,
}

/// Compose only literal prefixes from statically delimited closure route groups.
/// If any containing group has a dynamic prefix, retain the declaration path
/// instead of manufacturing an effective route that might be wrong.
fn effective_route_path(
    declared_path: &str,
    route_offset: usize,
    groups: &[RouteGroupPrefix],
) -> Option<String> {
    let mut containing = groups
        .iter()
        .filter(|group| route_offset > group.body_start && route_offset < group.body_end)
        .collect::<Vec<_>>();
    containing.sort_by_key(|group| group.body_start);
    if containing.iter().any(|group| group.prefix.is_none()) {
        return None;
    }
    let prefix = containing
        .iter()
        .filter_map(|group| group.prefix.as_deref())
        .filter(|prefix| !prefix.is_empty())
        .fold(String::new(), |mut combined, next| {
            combined = join_route_path(&combined, next);
            combined
        });
    Some(join_route_path(&prefix, declared_path))
}

fn route_context_json(
    declared_path: &str,
    effective_path: Option<&str>,
    route_name: &RouteNameContext,
    route_middleware: &RouteMiddlewareContext,
    route_offset: usize,
    groups: &[RouteGroupPrefix],
) -> Value {
    let mut containing = groups
        .iter()
        .filter(|group| route_offset > group.body_start && route_offset < group.body_end)
        .collect::<Vec<_>>();
    containing.sort_by_key(|group| group.body_start);
    let name_prefix_complete = containing.iter().all(|group| group.name_prefix.is_some());
    let name_prefix = containing
        .iter()
        .map(|group| group.name_prefix.as_deref())
        .collect::<Option<Vec<_>>>()
        .map(|parts| parts.concat());
    let effective_route_name = if route_name.complete && name_prefix_complete {
        route_name
            .declared
            .as_deref()
            .map(|name| format!("{}{name}", name_prefix.as_deref().unwrap_or("")))
    } else {
        None
    };
    json!({
        "declared_path": declared_path,
        "effective_path": effective_path,
        "name_prefix": name_prefix,
        "name_prefix_complete": name_prefix_complete,
        "declared_route_name": route_name.declared,
        "effective_route_name": effective_route_name,
        "route_name_seen": route_name.seen,
        "route_name_complete": route_name.seen && route_name.complete && name_prefix_complete,
        "inherited_middleware": containing.iter().flat_map(|group| group.middleware.iter()).collect::<Vec<_>>(),
        "route_middleware": route_middleware.declared,
        "route_middleware_seen": route_middleware.seen,
        "route_middleware_complete": route_middleware.seen && route_middleware.complete,
        "middleware_complete": containing.iter().all(|group| group.middleware_complete)
            && (!route_middleware.seen || route_middleware.complete),
        "group_count": containing.len(),
        "groups": containing.iter().map(|group| json!({
            "source_span": span_json(group.source_span),
            "prefix": group.prefix,
            "name_prefix": group.name_prefix,
            "middleware": group.middleware,
            "middleware_complete": group.middleware_complete
        })).collect::<Vec<_>>(),
        "semantics": "static_declarations_only_runtime_behavior_not_proven"
    })
}

fn route_middleware_after_call(
    source: &str,
    source_map: &PhpSourceMap,
    code: &str,
    route_start: usize,
    route_match_end: usize,
) -> RouteMiddlewareContext {
    let Some(open_paren) = code[route_start..route_match_end]
        .bytes()
        .position(|byte| byte == b'(')
        .map(|offset| route_start + offset)
    else {
        return RouteMiddlewareContext {
            declared: Vec::new(),
            seen: false,
            complete: true,
        };
    };
    let Some(route_call_end) =
        matching_delimiter(source.as_bytes(), source_map, open_paren, b'(', b')')
    else {
        return RouteMiddlewareContext {
            declared: Vec::new(),
            seen: false,
            complete: false,
        };
    };
    let bytes = code.as_bytes();
    let mut position = route_call_end + 1;
    let mut middleware = Vec::new();
    let mut seen = false;
    let mut complete = true;
    loop {
        while bytes.get(position).is_some_and(u8::is_ascii_whitespace) {
            position += 1;
        }
        if bytes.get(position..position.saturating_add(2)) != Some(b"->") {
            break;
        }
        position += 2;
        while bytes.get(position).is_some_and(u8::is_ascii_whitespace) {
            position += 1;
        }
        let method_start = position;
        while bytes
            .get(position)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            position += 1;
        }
        if method_start == position {
            break;
        }
        let method = &code[method_start..position];
        while bytes.get(position).is_some_and(u8::is_ascii_whitespace) {
            position += 1;
        }
        if bytes.get(position) != Some(&b'(') {
            break;
        }
        let open = position;
        let Some(close) = matching_delimiter(source.as_bytes(), source_map, open, b'(', b')')
        else {
            if method == "middleware" {
                seen = true;
                complete = false;
            }
            break;
        };
        if method == "middleware" {
            seen = true;
            match php_middleware_list(&source[open + 1..close]) {
                Some(values) => middleware.extend(values),
                None => complete = false,
            }
        }
        position = close + 1;
    }
    RouteMiddlewareContext {
        declared: middleware,
        seen,
        complete,
    }
}

fn route_name_after_call(
    source: &str,
    source_map: &PhpSourceMap,
    code: &str,
    route_start: usize,
    route_match_end: usize,
) -> RouteNameContext {
    let Some(open_paren) = code[route_start..route_match_end]
        .bytes()
        .position(|byte| byte == b'(')
        .map(|offset| route_start + offset)
    else {
        return RouteNameContext {
            declared: None,
            seen: false,
            complete: true,
        };
    };
    let Some(route_call_end) =
        matching_delimiter(source.as_bytes(), source_map, open_paren, b'(', b')')
    else {
        return RouteNameContext {
            declared: None,
            seen: false,
            complete: false,
        };
    };
    let bytes = code.as_bytes();
    let mut position = route_call_end + 1;
    let mut name = None;
    let mut seen = false;
    let mut complete = true;
    loop {
        while bytes.get(position).is_some_and(u8::is_ascii_whitespace) {
            position += 1;
        }
        if bytes.get(position..position.saturating_add(2)) != Some(b"->") {
            break;
        }
        position += 2;
        while bytes.get(position).is_some_and(u8::is_ascii_whitespace) {
            position += 1;
        }
        let method_start = position;
        while bytes
            .get(position)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            position += 1;
        }
        if method_start == position {
            break;
        }
        let method = &code[method_start..position];
        while bytes.get(position).is_some_and(u8::is_ascii_whitespace) {
            position += 1;
        }
        if bytes.get(position) != Some(&b'(') {
            break;
        }
        let open = position;
        let Some(close) = matching_delimiter(source.as_bytes(), source_map, open, b'(', b')')
        else {
            if method == "name" {
                seen = true;
                complete = false;
            }
            break;
        };
        if method == "name" {
            if seen {
                complete = false;
                name = None;
            } else {
                seen = true;
                match php_string_literal(&source[open + 1..close]) {
                    Some(value) => name = Some(value),
                    None => complete = false,
                }
            }
        }
        position = close + 1;
    }
    RouteNameContext {
        declared: name,
        seen,
        complete,
    }
}

fn join_route_path(prefix: &str, path: &str) -> String {
    let prefix = prefix.trim_matches('/');
    let path = path.trim_matches('/');
    match (prefix.is_empty(), path.is_empty()) {
        (true, true) => "/".to_owned(),
        (true, false) => format!("/{path}"),
        (false, true) => format!("/{prefix}"),
        (false, false) => format!("/{prefix}/{path}"),
    }
}

fn route_group_prefixes(
    source: &str,
    source_map: &PhpSourceMap,
    code: &str,
) -> Vec<RouteGroupPrefix> {
    let group = super::compiled_regex(
        r"Route\s*::[^;{}]*?->\s*group\s*\(\s*function\s*\([^)]*\)\s*\{",
        "static route group regex",
    );
    let modifier_call = super::compiled_regex(
        r"(?:Route\s*::|->)\s*(prefix|name|middleware)\s*\(",
        "route group modifier regex",
    );
    let mut groups = Vec::new();
    for capture in group.captures_iter(code) {
        let matched = capture.get(0).expect("route group match");
        if !source_map.is_code(matched.start()) {
            continue;
        }
        let open_brace = matched.end().saturating_sub(1);
        let Some(close_brace) = matching_brace(source.as_bytes(), source_map, open_brace) else {
            continue;
        };
        let mut prefix = Some(String::new());
        let mut name_prefix = Some(String::new());
        let mut middleware = Vec::new();
        let mut middleware_complete = true;
        let header = &code[matched.start()..open_brace];
        for call_capture in modifier_call.captures_iter(header) {
            let call = call_capture.get(0).expect("route group modifier");
            let method = call_capture.get(1).expect("modifier name").as_str();
            let open_paren = matched.start() + call.end() - 1;
            let Some(close_paren) =
                matching_delimiter(source.as_bytes(), source_map, open_paren, b'(', b')')
            else {
                match method {
                    "prefix" => prefix = None,
                    "name" => name_prefix = None,
                    "middleware" => middleware_complete = false,
                    _ => unreachable!(),
                }
                continue;
            };
            let arguments = &source[open_paren + 1..close_paren];
            match method {
                "prefix" => match php_string_literal(arguments) {
                    Some(value) => {
                        if let Some(prefix) = prefix.as_mut() {
                            *prefix = join_route_path(prefix, &value);
                        }
                    }
                    None => prefix = None,
                },
                "name" => match php_string_literal(arguments) {
                    Some(value) => {
                        if let Some(name_prefix) = name_prefix.as_mut() {
                            name_prefix.push_str(&value);
                        }
                    }
                    None => name_prefix = None,
                },
                "middleware" => match php_middleware_list(arguments) {
                    Some(values) => middleware.extend(values),
                    None => middleware_complete = false,
                },
                _ => unreachable!(),
            }
        }
        groups.push(RouteGroupPrefix {
            body_start: open_brace,
            body_end: close_brace,
            source_span: source_map.span(source, matched.range()),
            prefix,
            name_prefix,
            middleware,
            middleware_complete,
        });
    }
    groups
}

fn php_string_literal(expression: &str) -> Option<String> {
    let expression = expression.trim();
    let quote = *expression.as_bytes().first()?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    let end = super::skip_quoted(expression.as_bytes(), 0, quote);
    if end != expression.len() || end < 2 {
        return None;
    }
    let value = &expression[1..end - 1];
    if value.contains(['\\', '$']) {
        return None;
    }
    Some(value.to_owned())
}

fn php_middleware_list(expression: &str) -> Option<Vec<String>> {
    let expression = expression.trim();
    if let Some(value) = php_string_literal(expression) {
        return Some(vec![value]);
    }
    let inner = expression.strip_prefix('[')?.strip_suffix(']')?;
    let mut values = Vec::new();
    for item in inner
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        if let Some(value) = php_string_literal(item) {
            values.push(value);
            continue;
        }
        let class_name = item.strip_suffix("::class")?;
        if !is_php_class_name(class_name) {
            return None;
        }
        values.push(format!("{class_name}::class"));
    }
    Some(values)
}

fn is_php_class_name(name: &str) -> bool {
    let name = name.strip_prefix('\\').unwrap_or(name);
    !name.is_empty()
        && name.split('\\').all(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
                && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        })
}

fn matching_delimiter(
    source: &[u8],
    source_map: &PhpSourceMap,
    open: usize,
    opening: u8,
    closing: u8,
) -> Option<usize> {
    if source.get(open) != Some(&opening) || !source_map.is_code(open) {
        return None;
    }
    let mut depth = 0_usize;
    for (index, byte) in source.iter().enumerate().skip(open) {
        if !source_map.is_code(index) {
            continue;
        }
        match *byte {
            byte if byte == opening => depth = depth.checked_add(1)?,
            byte if byte == closing => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn namespace_at(source: &str, source_map: &PhpSourceMap, offset: usize) -> Option<String> {
    let declaration = super::compiled_regex(
        r"\bnamespace(?:\s+([A-Za-z_][A-Za-z0-9_\\]*))?\s*([;{])",
        "PHP namespace declaration regex",
    );
    let mut current = String::new();
    for capture in declaration.captures_iter(source) {
        let matched = capture.get(0)?;
        if matched.start() >= offset {
            break;
        }
        if !source_map.is_code(matched.start()) {
            continue;
        }
        let name = capture
            .get(1)
            .map_or_else(String::new, |value| value.as_str().to_owned());
        match capture.get(2)?.as_str() {
            ";" => current = name,
            "{" => {
                let open = matched.end().saturating_sub(1);
                let close = matching_brace(source.as_bytes(), source_map, open)?;
                if offset > open && offset < close {
                    return Some(name);
                }
            }
            _ => return None,
        }
    }
    Some(current)
}

fn qualified_name(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        name.to_owned()
    } else {
        format!("{namespace}\\{name}")
    }
}

fn resolve_class_reference(
    source: &str,
    source_map: &PhpSourceMap,
    offset: usize,
    reference: &str,
) -> Option<String> {
    if let Some(absolute) = reference.strip_prefix('\\') {
        return Some(absolute.to_owned());
    }
    let namespace = namespace_at(source, source_map, offset)?;
    let imports = super::compiled_regex(
        r"(?m)^\s*use\s+(?:(function|const)\s+)?(\\?[A-Za-z_][A-Za-z0-9_\\]*)(?:\s+as\s+([A-Za-z_]\w*))?\s*;",
        "PHP class import regex",
    );
    let grouped_import = super::compiled_regex(
        r"(?m)^\s*use\s+(?:(function|const)\s+)?([^;{}\r\n]+?)\\\{([^{}\r\n]*)\}\s*;",
        "PHP grouped import regex",
    );
    let grouped_item = super::compiled_regex(
        r"^(?:(function|const)\s+)?([A-Za-z_][A-Za-z0-9_\\]*)(?:\s+as\s+([A-Za-z_]\w*))?$",
        "PHP grouped import item regex",
    );
    let mut aliases = HashMap::new();
    let mut ambiguous_alias = false;
    let mut unsupported_grouped_import = false;
    for capture in imports.captures_iter(source) {
        let matched = capture.get(0)?;
        if matched.start() >= offset {
            break;
        }
        if !source_map.is_code(matched.start())
            || capture.get(1).is_some()
            || namespace_at(source, source_map, matched.start()).as_deref()
                != Some(namespace.as_str())
        {
            continue;
        }
        let import = capture.get(2)?.as_str().trim_start_matches('\\');
        let alias = capture.get(3).map_or_else(
            || import.rsplit('\\').next().unwrap_or(import),
            |value| value.as_str(),
        );
        if aliases
            .insert(alias.to_owned(), import.to_owned())
            .is_some_and(|previous| previous != import)
        {
            ambiguous_alias = true;
        }
    }
    for capture in grouped_import.captures_iter(source) {
        let matched = capture.get(0)?;
        if matched.start() >= offset {
            break;
        }
        if !source_map.is_code(matched.start())
            || namespace_at(source, source_map, matched.start()).as_deref()
                != Some(namespace.as_str())
            || capture.get(1).is_some()
        {
            continue;
        }
        let prefix = capture.get(2)?.as_str().trim().trim_start_matches('\\');
        let prefix_parts = prefix.split('\\').collect::<Vec<_>>();
        if prefix_parts.is_empty() || prefix_parts.iter().any(|part| !is_php_identifier(part)) {
            unsupported_grouped_import = true;
            continue;
        }
        for item in capture.get(3)?.as_str().split(',') {
            let Some(item_capture) = grouped_item.captures(item.trim()) else {
                unsupported_grouped_import = true;
                continue;
            };
            if item_capture.get(1).is_some() {
                continue;
            }
            let imported_name = item_capture.get(2)?.as_str();
            let import = format!("{prefix}\\{imported_name}");
            let alias = item_capture.get(3).map_or_else(
                || imported_name.rsplit('\\').next().unwrap_or(imported_name),
                |value| value.as_str(),
            );
            if aliases
                .insert(alias.to_owned(), import.clone())
                .is_some_and(|previous| previous != import)
            {
                ambiguous_alias = true;
            }
        }
    }
    if ambiguous_alias {
        return None;
    }
    if unsupported_grouped_import {
        return None;
    }
    let mut parts = reference.splitn(2, '\\');
    let first = parts.next()?;
    if let Some(import) = aliases.get(first) {
        return Some(match parts.next() {
            Some(suffix) => format!("{import}\\{suffix}"),
            None => import.clone(),
        });
    }
    Some(qualified_name(&namespace, reference))
}

fn is_php_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    matches!(bytes.next(), Some(b'A'..=b'Z' | b'a'..=b'z' | b'_'))
        && bytes.all(|byte| matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_'))
}

fn code_only(source: &str, source_map: &PhpSourceMap) -> String {
    let bytes = source
        .bytes()
        .enumerate()
        .map(|(offset, byte)| {
            if source_map.is_code(offset) || matches!(byte, b'\n' | b'\r') {
                byte
            } else {
                b' '
            }
        })
        .collect::<Vec<_>>();
    String::from_utf8(bytes).expect("lexical masking preserves valid UTF-8")
}

fn inline_handler(
    file: &str,
    source: &str,
    source_map: &PhpSourceMap,
    after_route_path: usize,
    route_path: &str,
    closure: &Regex,
) -> Option<Node> {
    let capture = closure.captures(source.get(after_route_path..)?)?;
    let kind = capture.get(1)?;
    let start = after_route_path + kind.start();
    if !source_map.is_code(start) {
        return None;
    }
    let mut end = start + kind.as_str().len();
    if kind.as_str() == "function" {
        let open_brace = (end..source.len()).find(|offset| {
            source.as_bytes().get(*offset) == Some(&b'{') && source_map.is_code(*offset)
        })?;
        end = matching_brace(source.as_bytes(), source_map, open_brace)? + 1;
    } else {
        end = arrow_closure_expression_end(source, source_map, end)?;
    }
    Some(Node::new(
        "inline_handler",
        &format!("closure for {route_path}"),
        file,
        source_map.span(source, start..end),
    ))
}

fn arrow_closure_expression_end(
    source: &str,
    source_map: &PhpSourceMap,
    mut offset: usize,
) -> Option<usize> {
    let bytes = source.as_bytes();
    offset = skip_php_space(source, source_map, offset, source.len());
    if bytes.get(offset) != Some(&b'(') {
        return None;
    }
    let parameters_end = matching_delimiter(bytes, source_map, offset, b'(', b')')?;
    offset = skip_php_space(source, source_map, parameters_end + 1, source.len());
    if bytes.get(offset..offset.saturating_add(2)) != Some(b"=>")
        || !source_map.is_code(offset)
        || !source_map.is_code(offset + 1)
    {
        return None;
    }
    let expression_start = skip_php_space(source, source_map, offset + 2, source.len());
    let mut parentheses = 0_usize;
    let mut brackets = 0_usize;
    let mut braces = 0_usize;
    let mut cursor = expression_start;
    while cursor < bytes.len() {
        if !source_map.is_code(cursor) {
            cursor += 1;
            continue;
        }
        match bytes[cursor] {
            b'(' => parentheses = parentheses.checked_add(1)?,
            b')' if parentheses > 0 => parentheses -= 1,
            b')' if brackets == 0 && braces == 0 => return Some(cursor),
            b'[' => brackets = brackets.checked_add(1)?,
            b']' if brackets > 0 => brackets -= 1,
            b'{' => braces = braces.checked_add(1)?,
            b'}' if braces > 0 => braces -= 1,
            b',' if parentheses == 0 && brackets == 0 && braces == 0 => return Some(cursor),
            _ => {}
        }
        cursor += 1;
    }
    (expression_start < cursor).then_some(cursor)
}

pub(super) fn matching_brace(
    source: &[u8],
    source_map: &PhpSourceMap,
    open: usize,
) -> Option<usize> {
    if source.get(open) != Some(&b'{') || !source_map.is_code(open) {
        return None;
    }
    let mut depth = 0_usize;
    for (index, byte) in source.iter().enumerate().skip(open) {
        if !source_map.is_code(index) {
            continue;
        }
        match *byte {
            b'{' => depth = depth.checked_add(1)?,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn span_json(span: SourceSpan) -> Value {
    json!({
        "start": {
            "byte_offset": span.start_byte,
            "line": span.start_line,
            "column": span.start_column
        },
        "end": {
            "byte_offset": span.end_byte,
            "line": span.end_line,
            "column": span.end_column
        },
        "end_exclusive": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_static_routes_models_relationships_and_explicit_tables() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "routes/web.php",
            "<?php\nRoute::get('/orders', [OrderController::class, 'index']);\n",
        );
        graph.scan(
            "app/Http/Controllers/OrderController.php",
            "<?php class OrderController { public function index() { return []; } }",
        );
        graph.scan(
            "app/Models/Order.php",
            "<?php class Order extends Model { protected $table = 'orders'; public function customer() { return $this->belongsTo(Customer::class); } }",
        );
        graph.scan(
            "app/Models/Customer.php",
            "<?php class Customer extends Model {}",
        );
        graph.scan(
            "database/migrations/2026_create_orders.php",
            "<?php Schema::create('orders', function ($table) {});",
        );

        let report = graph.finish();
        assert_eq!(report["summary"]["resolved_edges"], 3);
        assert_eq!(report["summary"]["unresolved_edges"], 0);
        let route = report["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["kind"] == "route" && node["name"] == "get /orders")
            .unwrap();
        assert_eq!(route["route_context"]["route_name_seen"], false);
        assert_eq!(route["route_context"]["route_name_complete"], false);
        assert!(report["edges"].as_array().unwrap().iter().any(|edge| {
            edge["kind"] == "route_controller_action"
                && edge["status"] == "resolved"
                && edge["to"]["name"] == "OrderController@index"
        }));
        assert!(report["edges"].as_array().unwrap().iter().any(|edge| {
            edge["kind"] == "model_table"
                && edge["status"] == "resolved"
                && edge["to"]["name"] == "orders"
        }));
        assert!(report["edges"].as_array().unwrap().iter().any(|edge| {
            edge["kind"] == "model_relationship"
                && edge["status"] == "resolved"
                && edge["to"]["name"] == "Customer"
        }));
    }

    #[test]
    fn links_only_static_controller_view_calls_to_blade_templates() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "app/Http/Controllers/InvoiceController.php",
            r#"<?php
class InvoiceController {
    public function index() {
        // return view('app.invoices.comment');
        $example = "view('app.invoices.string')";
        return view('app.invoices.index');
    }

    public function missing() { return view("app.invoices.not_found"); }
    public function dynamic() { return view($this->viewName()); }
}"#,
        );
        graph.scan(
            "resources/views/app/invoices/index.blade.php",
            "<h1>Invoices</h1>",
        );

        let report = graph.finish();
        let edges = report["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 2);
        assert!(edges.iter().any(|edge| {
            edge["kind"] == "controller_view"
                && edge["status"] == "resolved"
                && edge["target"]["name"] == "app.invoices.index"
                && edge["to"]["file"] == "resources/views/app/invoices/index.blade.php"
        }));
        assert!(edges.iter().any(|edge| {
            edge["kind"] == "controller_view"
                && edge["status"] == "unresolved"
                && edge["target"]["name"] == "app.invoices.not_found"
        }));
        assert!(!edges.iter().any(|edge| {
            edge["target"]["name"] == "app.invoices.comment"
                || edge["target"]["name"] == "app.invoices.string"
        }));
    }

    #[test]
    fn links_controller_actions_to_injected_and_resolved_application_methods() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "app/Http/Controllers/BookingController.php",
            r#"<?php
namespace App\Http\Controllers;
use App\Services\BookingCalculator;
use App\Services\CustomFieldService;
use App\Models\BookingAppointment;
use App\Services\Optimizer;
use App\Jobs\RebuildBooking;
class BookingController {
    private BookingCalculator $calculator;
    private Optimizer $optimizer;
    public function __construct(BookingCalculator $calculator, Optimizer $optimizer) {
        $this->calculator = $calculator;
        $this->optimizer = $optimizer;
    }
    public function show() {
        $this->calculator->availability();
        $this->optimizer->optimize();
        app(CustomFieldService::class)->forService();
        BookingAppointment::where('status', 'confirmed');
        RebuildBooking::dispatch();
        $ignored = "app(CustomFieldService::class)->fake()";
    }
}"#,
        );
        graph.scan(
            "app/Services/BookingCalculator.php",
            "<?php namespace App\\Services; class BookingCalculator { private CalendarService $calendar; public function __construct(CalendarService $calendar) { $this->calendar = $calendar; } public function availability() { $this->calendar->load(); PlanningRules::filter(); } }",
        );
        graph.scan(
            "app/Services/CustomFieldService.php",
            "<?php namespace App\\Services; class CustomFieldService { public function forService() {} public function fake() {} }",
        );
        graph.scan(
            "app/Models/BookingAppointment.php",
            "<?php namespace App\\Models; class BookingAppointment extends \\Illuminate\\Database\\Eloquent\\Model {}",
        );
        graph.scan(
            "app/Services/CalendarService.php",
            "<?php namespace App\\Services; class CalendarService { public function load() {} }",
        );
        graph.scan(
            "app/Services/PlanningRules.php",
            "<?php namespace App\\Services; class PlanningRules { public static function filter() {} }",
        );
        graph.scan(
            "app/Services/Optimizer.php",
            "<?php namespace App\\Services; interface Optimizer { public function optimize(): array; }",
        );
        graph.scan(
            "app/Jobs/RebuildBooking.php",
            "<?php namespace App\\Jobs; class RebuildBooking {}",
        );

        let report = graph.finish();
        let edges = report["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 7);
        assert!(edges.iter().any(|edge| {
            edge["kind"] == "controller_dependency_call"
                && edge["status"] == "resolved"
                && edge["target"]["name"] == "App\\Services\\BookingCalculator"
                && edge["target"]["method"] == "availability"
                && edge["to"]["kind"] == "application_method"
        }));
        assert!(edges.iter().any(|edge| {
            edge["kind"] == "controller_dependency_call"
                && edge["status"] == "resolved"
                && edge["target"]["name"] == "App\\Services\\CustomFieldService"
                && edge["target"]["method"] == "forService"
        }));
        assert!(edges.iter().any(|edge| {
            edge["kind"] == "controller_model_use"
                && edge["status"] == "resolved"
                && edge["target"]["name"] == "App\\Models\\BookingAppointment"
                && edge["to"]["name"] == "BookingAppointment"
        }));
        assert!(edges.iter().any(|edge| {
            edge["kind"] == "application_dependency_call"
                && edge["status"] == "resolved"
                && edge["from"]["name"] == "App\\Services\\BookingCalculator@availability"
                && edge["to"]["name"] == "App\\Services\\CalendarService@load"
        }));
        assert!(edges.iter().any(|edge| {
            edge["kind"] == "application_dependency_call"
                && edge["status"] == "resolved"
                && edge["to"]["name"] == "App\\Services\\PlanningRules@filter"
        }));
        assert!(edges.iter().any(|edge| {
            edge["kind"] == "controller_dependency_call"
                && edge["status"] == "resolved"
                && edge["target"]["name"] == "App\\Services\\Optimizer"
                && edge["target"]["method"] == "optimize"
                && edge["to"]["kind"] == "application_method_contract"
        }));
        assert!(edges.iter().any(|edge| {
            edge["kind"] == "controller_job_dispatch"
                && edge["status"] == "resolved"
                && edge["to"]["kind"] == "application_class"
                && edge["to"]["name"] == "App\\Jobs\\RebuildBooking"
        }));
    }

    #[test]
    fn extracts_literal_controller_validation_rules_without_evaluating_dynamic_rules() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "app/Http/Controllers/InvoiceController.php",
            r#"<?php
class InvoiceController {
    public function store(Request $request) {
        // $request->validate(['comment' => 'required']);
        $example = "$request->validate(['string' => 'required'])";
        $request->validate([
            'customer_id' => 'required|integer|exists:customers,id',
            'issued_at' => ['required', 'date'],
            'amount' => ['numeric', Rule::min(0)],
        ]);
    }
    public function dynamic(Request $request) {
        $request->validate($this->rulesFor($request));
    }
}"#,
        );

        let report = graph.finish();
        let nodes = report["nodes"].as_array().unwrap();
        let validations = nodes
            .iter()
            .filter(|node| node["kind"] == "validation")
            .collect::<Vec<_>>();
        assert_eq!(validations.len(), 2);

        let literal = validations
            .iter()
            .find(|node| node["validation_rules"]["parse_mode"] == "static_array_literal_subset")
            .unwrap();
        let rules = &literal["validation_rules"];
        assert_eq!(rules["complete"], false);
        assert_eq!(rules["fields"].as_array().unwrap().len(), 3);
        assert_eq!(rules["fields"][0]["field"], "customer_id");
        assert_eq!(rules["fields"][0]["rules"][0]["name"], "required");
        assert_eq!(rules["fields"][0]["rules"][1]["name"], "integer");
        assert_eq!(rules["fields"][0]["rules"][2]["name"], "exists");
        assert_eq!(rules["fields"][0]["rules"][2]["has_parameters"], true);
        assert_eq!(rules["fields"][0]["rules"][2]["parameter"], "customers,id");
        assert_eq!(rules["fields"][1]["complete"], true);
        assert_eq!(rules["fields"][2]["complete"], false);
        assert!(rules["semantics"]
            .as_str()
            .unwrap()
            .contains("parameters only"));

        let dynamic = validations
            .iter()
            .find(|node| node["validation_rules"]["parse_mode"] == "unsupported_expression")
            .unwrap();
        assert_eq!(dynamic["validation_rules"]["complete"], false);
        assert_eq!(
            dynamic["validation_rules"]["fields"]
                .as_array()
                .unwrap()
                .len(),
            0
        );

        let edges = report["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 2);
        assert!(edges.iter().all(|edge| {
            edge["kind"] == "controller_validation" && edge["status"] == "resolved"
        }));
    }

    #[test]
    fn redacts_unbounded_or_free_form_validation_parameters() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "app/Http/Controllers/CustomerController.php",
            r#"<?php
class CustomerController {
    public function store(Request $request) {
        $request->validate([
            'email' => 'required|email|max:255',
            'amount' => 'numeric|min:0.25|max:100.50',
            'status' => 'required|string|in:draft,issued,paid',
            'code' => 'regex:secret-pattern-value',
        ]);
    }
}"#,
        );
        let report = graph.finish();
        let node = report["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["kind"] == "validation")
            .unwrap();
        let rules = &node["validation_rules"];

        assert_eq!(rules["fields"][0]["rules"][2]["parameter"], "255");
        assert_eq!(rules["fields"][1]["rules"][1]["parameter"], "0.25");
        assert_eq!(rules["fields"][1]["rules"][2]["parameter"], "100.50");
        assert_eq!(
            rules["fields"][2]["rules"][2]["parameter"],
            "draft,issued,paid"
        );
        assert_eq!(rules["fields"][2]["complete"], true);
        assert_eq!(rules["fields"][3]["rules"][0]["parameter"], Value::Null);
        assert_eq!(rules["fields"][3]["rules"][0]["parameter_redacted"], true);
        assert_eq!(rules["fields"][3]["complete"], false);
        assert!(!serde_json::to_string(rules)
            .unwrap()
            .contains("secret-pattern-value"));
    }

    #[test]
    fn retains_only_identifier_tokens_for_comparison_rules_without_interpreting_them() {
        let (rules, complete) = validation_rule_names(
            "date|after_or_equal:start_date|before:today|same:confirmation_code",
        );
        assert!(complete);
        assert_eq!(rules[1]["parameter"], "start_date");
        assert_eq!(rules[1]["parameter_kind"], "identifier_token_uninterpreted");
        assert_eq!(rules[2]["parameter"], "today");
        assert_eq!(rules[2]["parameter_kind"], "identifier_token_uninterpreted");
        assert_eq!(rules[3]["parameter"], "confirmation_code");

        let (unsafe_rules, unsafe_complete) =
            validation_rule_names("date|after_or_equal:today + 1 day");
        assert!(!unsafe_complete);
        assert_eq!(unsafe_rules[1]["parameter"], Value::Null);
        assert_eq!(unsafe_rules[1]["parameter_redacted"], true);
        assert!(!serde_json::to_string(&unsafe_rules)
            .unwrap()
            .contains("today + 1 day"));
    }

    #[test]
    fn route_controller_resolution_uses_fully_qualified_namespaces() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "routes/web.php",
            "<?php\nRoute::get('/invoices', [\\App\\Billing\\InvoiceController::class, 'index']);\n",
        );
        graph.scan(
            "app/Other/InvoiceController.php",
            "<?php\nnamespace App\\Other;\nclass InvoiceController { public function index() {} }",
        );
        graph.scan(
            "app/Billing/InvoiceController.php",
            "<?php\nnamespace App\\Billing;\nclass InvoiceController { public function index() {} }",
        );

        let report = graph.finish();
        assert_eq!(report["summary"]["resolved_edges"], 1);
        assert_eq!(report["summary"]["unresolved_edges"], 0);
        let edge = report["edges"]
            .as_array()
            .unwrap()
            .iter()
            .find(|edge| edge["kind"] == "route_controller_action")
            .unwrap();
        assert_eq!(edge["to"]["file"], "app/Billing/InvoiceController.php");
    }

    #[test]
    fn route_controller_aliases_and_group_imports_resolve_conservatively() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "routes/aliased.php",
            "<?php\nuse App\\Billing\\InvoiceController as BillingInvoices;\nRoute::get('/invoices', [BillingInvoices::class, 'index']);\n",
        );
        graph.scan(
            "routes/grouped.php",
            "<?php\nuse App\\Billing\\{InvoiceController as GroupedInvoices};\nRoute::get('/grouped', [GroupedInvoices::class, 'index']);\n",
        );
        graph.scan(
            "routes/grouped-fallback.php",
            "<?php\nuse App\\Billing\\{InvoiceController as BillingInvoices};\nRoute::get('/fallback', [GlobalController::class, 'index']);\n",
        );
        graph.scan(
            "routes/grouped-ambiguous.php",
            "<?php\nuse App\\Billing\\{InvoiceController as Duplicate, OtherController as Duplicate};\nRoute::get('/ambiguous', [Duplicate::class, 'index']);\n",
        );
        graph.scan(
            "app/Billing/InvoiceController.php",
            "<?php\nnamespace App\\Billing;\nclass InvoiceController { public function index() {} }",
        );
        graph.scan(
            "app/Billing/OtherController.php",
            "<?php\nnamespace App\\Billing;\nclass OtherController { public function index() {} }",
        );
        graph.scan(
            "app/GlobalController.php",
            "<?php\nclass GlobalController { public function index() {} }",
        );
        graph.scan(
            "app/Global/InvoiceController.php",
            "<?php class InvoiceController { public function index() {} }",
        );

        let report = graph.finish();
        assert_eq!(report["summary"]["resolved_edges"], 3);
        assert_eq!(report["summary"]["unresolved_edges"], 1);
        let edges = report["edges"].as_array().unwrap();
        let aliased = edges
            .iter()
            .find(|edge| edge["from"]["name"] == "get /invoices")
            .unwrap();
        assert_eq!(aliased["to"]["file"], "app/Billing/InvoiceController.php");
        let grouped = edges
            .iter()
            .find(|edge| edge["from"]["name"] == "get /grouped")
            .unwrap();
        assert_eq!(grouped["to"]["file"], "app/Billing/InvoiceController.php");
        let fallback = edges
            .iter()
            .find(|edge| edge["from"]["name"] == "get /fallback")
            .unwrap();
        assert_eq!(fallback["to"]["file"], "app/GlobalController.php");
        let ambiguous = edges
            .iter()
            .find(|edge| edge["from"]["name"] == "get /ambiguous")
            .unwrap();
        assert_eq!(ambiguous["status"], "unresolved");
    }

    #[test]
    fn static_nested_route_group_prefixes_are_composed_and_dynamic_prefixes_fail_closed() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "routes/web.php",
            "<?php\nRoute::middleware(['auth', \\App\\Http\\Middleware\\CheckTenant::class])->prefix('app')->name('app.')->group(function () {\n    Route::prefix('inventory')->name('inventory.')->group(function () {\n        Route::get('/items', fn () => [])->name('items.index');\n    });\n});\nRoute::middleware($checks)->prefix($tenant)->name($name)->group(function () {\n    Route::get('/items', fn () => [])->name($routeName);\n});\n",
        );

        let report = graph.finish();
        let routes = report["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|node| node["kind"] == "route")
            .map(|node| node["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert!(routes.contains(&"get /app/inventory/items"));
        assert!(routes.contains(&"get /items"));
        let static_route = report["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["name"] == "get /app/inventory/items")
            .unwrap();
        let context = &static_route["route_context"];
        assert_eq!(context["declared_path"], "/items");
        assert_eq!(context["effective_path"], "/app/inventory/items");
        assert_eq!(context["name_prefix"], "app.inventory.");
        assert_eq!(context["name_prefix_complete"], true);
        assert_eq!(context["declared_route_name"], "items.index");
        assert_eq!(context["effective_route_name"], "app.inventory.items.index");
        assert_eq!(context["route_name_complete"], true);
        assert_eq!(context["inherited_middleware"][0], "auth");
        assert_eq!(
            context["inherited_middleware"][1],
            r"\App\Http\Middleware\CheckTenant::class"
        );
        assert_eq!(context["middleware_complete"], true);
        assert_eq!(context["groups"][0]["source_span"]["start"]["line"], 2);
        assert_eq!(context["groups"][1]["source_span"]["start"]["line"], 3);

        let dynamic_route = report["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["name"] == "get /items")
            .unwrap();
        assert_eq!(
            dynamic_route["route_context"]["effective_path"],
            Value::Null
        );
        assert_eq!(
            dynamic_route["route_context"]["name_prefix_complete"],
            false
        );
        assert_eq!(dynamic_route["route_context"]["route_name_seen"], true);
        assert_eq!(dynamic_route["route_context"]["route_name_complete"], false);
        assert_eq!(dynamic_route["route_context"]["middleware_complete"], false);
    }

    #[test]
    fn route_level_middleware_is_reported_as_static_declaration_or_incomplete() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "routes/web.php",
            "<?php\nRoute::get('/health', fn () => 'ok')->middleware(['auth', \\App\\Http\\Middleware\\CheckTenant::class])->name('health');\nRoute::get('/dynamic', fn () => 'ok')->middleware($middleware);\n",
        );

        let report = graph.finish();
        let routes = report["nodes"].as_array().unwrap();
        let static_route = routes
            .iter()
            .find(|node| node["name"] == "get /health")
            .unwrap();
        let context = &static_route["route_context"];
        assert_eq!(context["route_middleware_seen"], true);
        assert_eq!(context["route_middleware_complete"], true);
        assert_eq!(context["route_middleware"][0], "auth");
        assert_eq!(
            context["route_middleware"][1],
            r"\App\Http\Middleware\CheckTenant::class"
        );
        assert_eq!(context["middleware_complete"], true);

        let dynamic_route = routes
            .iter()
            .find(|node| node["name"] == "get /dynamic")
            .unwrap();
        let context = &dynamic_route["route_context"];
        assert_eq!(context["route_middleware_seen"], true);
        assert_eq!(context["route_middleware_complete"], false);
        assert_eq!(context["middleware_complete"], false);
    }

    #[test]
    fn bracketed_php_namespaces_are_scoped_when_linking_controller_actions() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "routes/web.php",
            "<?php\nRoute::get('/invoices', [\\App\\Billing\\InvoiceController::class, 'index']);\n",
        );
        graph.scan(
            "app/InvoiceController.php",
            "<?php\nnamespace App\\Billing { class InvoiceController { public function index() {} } }",
        );

        let report = graph.finish();
        assert_eq!(report["summary"]["resolved_edges"], 1);
        assert_eq!(report["summary"]["unresolved_edges"], 0);
    }

    #[test]
    fn explicit_model_table_links_to_create_migration_not_follow_up_table_changes() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "app/Models/GoogleCalendarOAuthSetting.php",
            "<?php class GoogleCalendarOAuthSetting extends Model { protected $table = 'google_calendar_oauth_settings'; }",
        );
        graph.scan(
            "database/migrations/2026_create_google_calendar_oauth_settings.php",
            "<?php Schema::create('google_calendar_oauth_settings', function ($table) {});",
        );
        graph.scan(
            "database/migrations/2026_scope_google_calendar_oauth_settings.php",
            "<?php Schema::table('google_calendar_oauth_settings', function ($table) {});",
        );

        let report = graph.finish();
        assert_eq!(report["summary"]["resolved_edges"], 1);
        assert_eq!(report["summary"]["unresolved_edges"], 0);
        let edge = report["edges"]
            .as_array()
            .unwrap()
            .iter()
            .find(|edge| edge["kind"] == "model_table")
            .unwrap();
        assert_eq!(edge["resolution_candidate_count"], 1);
        assert_eq!(
            edge["to"]["file"],
            "database/migrations/2026_create_google_calendar_oauth_settings.php"
        );
        assert_eq!(
            report["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|node| node["kind"] == "table")
                .count(),
            2,
            "follow-up schema operations remain visible as graph nodes"
        );
    }

    #[test]
    fn explicit_model_table_stays_unresolved_without_a_scanned_create_migration() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "app/Models/ExternalRecord.php",
            "<?php class ExternalRecord extends Model { protected $table = 'external_records'; }",
        );
        graph.scan(
            "database/migrations/2026_update_external_records.php",
            "<?php Schema::table('external_records', function ($table) {});",
        );

        let report = graph.finish();
        assert_eq!(report["summary"]["resolved_edges"], 0);
        assert_eq!(report["summary"]["unresolved_edges"], 1);
        assert_eq!(report["edges"][0]["resolution_candidate_count"], 0);
    }

    #[test]
    fn does_not_link_comment_or_string_examples_as_source_code() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "routes/web.php",
            "<?php\n// Route::get('/comment', [GhostController::class, 'run']);\n$example = \"Route::get('/string', [GhostController::class, 'run']);\";\n",
        );
        let report = graph.finish();
        assert_eq!(report["summary"]["nodes"], 0);
        assert_eq!(report["summary"]["edges"], 0);
    }

    #[test]
    fn preserves_unresolved_routes_and_ambiguous_controller_actions() {
        let mut graph = GraphBuilder::default();
        graph.scan("routes/web.php", "<?php Route::get('/dynamic', $handler); Route::get('/orders', [OrderController::class, 'index']);");
        graph.scan(
            "a.php",
            "<?php class OrderController { function index() {} }",
        );
        graph.scan(
            "b.php",
            "<?php class OrderController { function index() {} }",
        );

        let report = graph.finish();
        assert_eq!(report["summary"]["resolved_edges"], 0);
        assert_eq!(report["summary"]["unresolved_edges"], 2);
        assert!(report["edges"]
            .as_array()
            .unwrap()
            .iter()
            .all(|edge| { edge["status"] == "unresolved" && edge["reason"].is_string() }));
        let ambiguous = report["edges"]
            .as_array()
            .unwrap()
            .iter()
            .find(|edge| edge["target"]["kind"] == "controller_action")
            .unwrap();
        assert_eq!(ambiguous["resolution_candidate_count"], 2);
        assert!(ambiguous["reason"]
            .as_str()
            .unwrap()
            .contains("multiple scanned controller declarations"));
    }

    #[test]
    fn explains_when_a_scanned_controller_does_not_define_the_route_method() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "routes/web.php",
            "<?php Route::post('/banking/rules/toggle', [BankRuleController::class, 'toggleActive']);",
        );
        graph.scan(
            "app/Http/Controllers/BankRuleController.php",
            "<?php class BankRuleController { function toggle() {} }",
        );

        let report = graph.finish();
        let edge = &report["edges"][0];
        assert_eq!(edge["status"], "unresolved");
        assert_eq!(edge["resolution_candidate_count"], 0);
        assert!(edge["reason"]
            .as_str()
            .unwrap()
            .contains("this method was not"));
        assert_eq!(edge["scanned_direct_methods"], json!(["toggle"]));
    }

    #[test]
    fn explains_missing_application_methods_and_lists_scanned_methods() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "app/Services/PlanningService.php",
            r#"<?php namespace App\Services;
class PlanningService {
    public function snapshot() {}
    public function compute() {}
}"#,
        );
        graph.scan(
            "app/Http/Controllers/DispatchController.php",
            r#"<?php namespace App\Http\Controllers;
class DispatchController {
    public function index() {
        app(\App\Services\PlanningService::class)->routingProvider();
        app(\App\Services\MissingService::class)->send();
    }
}"#,
        );

        let report = graph.finish();
        let edges = report["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 2);
        let existing_type = edges
            .iter()
            .find(|edge| edge["target"]["name"] == "App\\Services\\PlanningService")
            .unwrap();
        assert_eq!(existing_type["status"], "unresolved");
        assert!(existing_type["reason"]
            .as_str()
            .unwrap()
            .contains("this method was not declared"));
        assert_eq!(
            existing_type["scanned_direct_methods"],
            json!(["compute", "snapshot"])
        );

        let missing_type = edges
            .iter()
            .find(|edge| edge["target"]["name"] == "App\\Services\\MissingService")
            .unwrap();
        assert_eq!(missing_type["status"], "unresolved");
        assert!(missing_type["reason"]
            .as_str()
            .unwrap()
            .contains("was not found among scanned source files"));
        assert_eq!(missing_type["scanned_direct_methods"], json!([]));
    }

    #[test]
    fn links_inline_closure_routes_without_claiming_behavioral_equivalence() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "routes/web.php",
            "<?php Route::get('/inline', function () { return view('page'); }); Route::get('/arrow', fn () => View::make('arrow-page'));",
        );
        graph.scan("resources/views/page.blade.php", "<h1>Page</h1>");
        graph.scan("resources/views/arrow-page.blade.php", "<h1>Arrow</h1>");
        let report = graph.finish();
        assert_eq!(report["summary"]["resolved_edges"], 4);
        assert_eq!(report["summary"]["unresolved_edges"], 0);
        let edges = report["edges"].as_array().unwrap();
        assert_eq!(
            edges
                .iter()
                .filter(|edge| edge["kind"] == "route_controller_action")
                .count(),
            2
        );
        assert_eq!(
            edges
                .iter()
                .filter(|edge| edge["kind"] == "inline_handler_view")
                .count(),
            2
        );
        let arrow = report["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["kind"] == "inline_handler" && node["name"] == "closure for /arrow")
            .unwrap();
        assert_eq!(arrow["source_span"]["start"]["line"], 1);
        assert!(
            arrow["source_span"]["end"]["byte_offset"].as_u64().unwrap()
                > arrow["source_span"]["start"]["byte_offset"]
                    .as_u64()
                    .unwrap()
        );
    }

    #[test]
    fn inline_route_view_graph_ignores_dynamic_names_and_string_examples() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "routes/web.php",
            "<?php Route::get('/dynamic', fn () => view($template)); Route::get('/text', fn () => 'view(\\\"fake\\\")');",
        );
        let report = graph.finish();
        assert!(!report["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["kind"] == "inline_handler_view"));
    }

    #[test]
    fn expands_resource_routes_only_to_the_documented_controller_actions() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "routes/api.php",
            "<?php Route::apiResource('/orders', OrderController::class);",
        );
        graph.scan(
            "app/Http/Controllers/OrderController.php",
            "<?php class OrderController { function index() {} function store() {} function show() {} function update() {} function destroy() {} function create() {} }",
        );
        let report = graph.finish();
        assert_eq!(report["summary"]["resolved_edges"], 5);
        assert_eq!(report["summary"]["unresolved_edges"], 0);
        let names = report["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|edge| edge["to"]["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "OrderController@index",
                "OrderController@store",
                "OrderController@show",
                "OrderController@update",
                "OrderController@destroy",
            ]
        );
    }

    #[test]
    fn resolves_method_routes_inside_static_controller_groups() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "routes/web.php",
            "<?php Route::prefix('dispatch')->controller(DispatchController::class)->group(function () { Route::get('/orders', 'index'); });",
        );
        graph.scan(
            "app/Http/Controllers/DispatchController.php",
            "<?php class DispatchController { function index() {} }",
        );
        let report = graph.finish();
        assert_eq!(report["summary"]["resolved_edges"], 1);
        assert_eq!(report["summary"]["unresolved_edges"], 0);
        assert_eq!(report["edges"][0]["to"]["name"], "DispatchController@index");
    }

    #[test]
    fn resolves_fluent_controller_groups_with_braces_in_route_prefix_strings() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "routes/web.php",
            "<?php Route::prefix('einsatz/{token}')->name('dispatch.public.')->middleware(['throttle:30,1', \\App\\Http\\Middleware\\EnsureBusinessSuiteModule::class])->controller(\\App\\Http\\Controllers\\Dispatch\\PublicDispatchController::class)->group(function () { Route::get('/', 'show'); Route::get('/live', 'liveTracking'); Route::get('/ics', 'downloadIcs'); Route::post('/buchen', 'book'); Route::post('/warten', 'wait'); Route::post('/absagen', 'cancel'); });",
        );
        graph.scan(
            "app/Http/Controllers/Dispatch/PublicDispatchController.php",
            "<?php\nnamespace App\\Http\\Controllers\\Dispatch;\nclass PublicDispatchController { function show() {} function liveTracking() {} function downloadIcs() {} function book() {} function wait() {} function cancel() {} }",
        );

        let report = graph.finish();
        assert_eq!(report["summary"]["resolved_edges"], 6);
        assert_eq!(report["summary"]["unresolved_edges"], 0);
        assert!(report["edges"].as_array().unwrap().iter().any(|edge| {
            edge["kind"] == "route_controller_action"
                && edge["status"] == "resolved"
                && edge["to"]["name"] == "PublicDispatchController@liveTracking"
        }));
    }

    #[test]
    fn recognizes_authenticatable_models_as_eloquent_relation_targets() {
        let mut graph = GraphBuilder::default();
        graph.scan(
            "app/Models/Order.php",
            "<?php class Order extends Model { public function user() { return $this->belongsTo(User::class); } }",
        );
        graph.scan(
            "app/Models/User.php",
            "<?php class User extends Authenticatable {}",
        );
        let report = graph.finish();
        assert_eq!(report["summary"]["resolved_edges"], 1);
        assert_eq!(report["summary"]["unresolved_edges"], 0);
        assert_eq!(report["edges"][0]["to"]["name"], "User");
    }
}
