use super::*;

#[derive(Clone, Debug)]
struct RouteEntry {
    method: String,
    path: String,
    kind: String,
    span: zelyra_ast::Span,
}

fn route_entries(program: &zelyra_ast::Program) -> Vec<RouteEntry> {
    let mut entries = Vec::new();
    for page in &program.pages {
        entries.push(RouteEntry {
            method: "GET".into(),
            path: page.path.clone(),
            kind: "page".into(),
            span: page.span,
        });
    }
    for api in &program.apis {
        entries.push(RouteEntry {
            method: api.method.to_ascii_uppercase(),
            path: api.path.clone(),
            kind: "api".into(),
            span: api.span,
        });
    }
    for form in &program.forms {
        for method in ["GET", "POST"] {
            entries.push(RouteEntry {
                method: method.into(),
                path: format!("/forms/{}", form.name),
                kind: "form".into(),
                span: form.span,
            });
        }
    }
    for tableview in &program.tableviews {
        entries.push(RouteEntry {
            method: "GET".into(),
            path: format!("/views/{}", tableview.name.to_ascii_lowercase()),
            kind: "tableview".into(),
            span: tableview.span,
        });
    }
    for crud in &program.cruds {
        let base = format!("/{}", crud.table);
        let id = format!("{base}/{{id}}");
        let rows = [
            ("GET", base.clone(), "crud list"),
            ("GET", format!("{base}/new"), "crud create form"),
            ("POST", format!("{base}/new"), "crud create"),
            ("GET", id.clone(), "crud detail"),
            ("GET", format!("{id}/edit"), "crud edit form"),
            ("POST", format!("{id}/edit"), "crud edit"),
            ("POST", format!("{id}/delete"), "crud delete"),
            ("POST", format!("{id}/restore"), "crud restore"),
        ];
        entries.extend(rows.into_iter().map(|(method, path, kind)| RouteEntry {
            method: method.into(),
            path,
            kind: kind.into(),
            span: crud.span,
        }));
        for action in &crud.actions {
            entries.push(RouteEntry {
                method: "POST".into(),
                path: format!("{id}/{}", action.name),
                kind: format!("crud action {}", action.name),
                span: action.span,
            });
        }
    }
    for auth in &program.auth {
        const GET_POST: &[&str] = &["GET", "POST"];
        const POST: &[&str] = &["POST"];
        for (path, methods) in [
            ("/login", GET_POST),
            ("/logout", POST),
            ("/forgot-password", GET_POST),
            ("/reset-password", GET_POST),
        ] {
            for method in methods {
                entries.push(RouteEntry {
                    method: method.into(),
                    path: path.into(),
                    kind: "authentication".into(),
                    span: auth.span,
                });
            }
        }
        for method in ["GET", "POST"] {
            entries.push(RouteEntry {
                method: method.into(),
                path: "/account/sessions".into(),
                kind: "session management".into(),
                span: auth.span,
            });
        }
        if let Some(path) = &auth.admin_path {
            entries.push(RouteEntry {
                method: "GET".into(),
                path: path.clone(),
                kind: "authentication administration".into(),
                span: auth.span,
            });
        }
    }
    entries.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.method.cmp(&right.method))
            .then(left.kind.cmp(&right.kind))
            .then(left.span.source_id.cmp(&right.span.source_id))
            .then(left.span.line.cmp(&right.span.line))
    });
    entries
}

fn routes_overlap(left: &str, right: &str) -> bool {
    let left = left
        .trim_matches('/')
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let right = right
        .trim_matches('/')
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(left, right)| left == right || is_parameter(left) && is_parameter(right))
}

fn is_parameter(segment: &str) -> bool {
    segment.starts_with('{') && segment.ends_with('}')
}

fn route_collision_pairs(entries: &[RouteEntry]) -> Vec<(usize, usize)> {
    let mut collisions = Vec::new();
    for (current_index, current) in entries.iter().enumerate() {
        if let Some((previous_index, _)) =
            entries[..current_index]
                .iter()
                .enumerate()
                .find(|(_, previous)| {
                    previous.method == current.method
                        && routes_overlap(&previous.path, &current.path)
                })
        {
            collisions.push((previous_index, current_index));
        }
    }
    collisions
}

pub(super) fn validate_no_collisions(path: &str, program: &zelyra_ast::Program) -> bool {
    let entries = route_entries(program);
    let collisions = route_collision_pairs(&entries);
    for (previous_index, current_index) in &collisions {
        let previous = &entries[*previous_index];
        let current = &entries[*current_index];
        diagnostic_with_span(
            path,
            "E-ROUTE-001",
            &format!(
                "route `{} {}` from {} overlaps `{} {}` from {}",
                current.method,
                current.path,
                current.kind,
                previous.method,
                previous.path,
                previous.kind
            ),
            current.span,
        );
    }
    collisions.is_empty()
}

pub(super) fn command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(path) = args.next() else {
        usage();
        return ExitCode::from(2);
    };
    let format = match (args.next(), args.next(), args.next()) {
        (None, None, None) => "human",
        (Some(flag), Some(value), None)
            if flag == "--format" && matches!(value.as_str(), "human" | "json") =>
        {
            if value == "json" {
                "json"
            } else {
                "human"
            }
        }
        (Some(argument), None, None) if argument == "--format=human" => "human",
        (Some(argument), None, None) if argument == "--format=json" => "json",
        _ => {
            usage();
            return ExitCode::from(2);
        }
    };
    let Ok(program) = validate(&path) else {
        return ExitCode::from(1);
    };
    let sources = PROJECT_SOURCES.with(|sources| sources.borrow().clone());
    let entries = route_entries(&program);
    if format == "json" {
        let routes = entries
            .iter()
            .map(|entry| {
                let source = sources.get(entry.span.source_id as usize);
                json!({
                    "method": entry.method,
                    "path": entry.path,
                    "kind": entry.kind,
                    "file": source.map_or(path.as_str(), |source| source.path.as_str()),
                    "line": entry.span.line,
                })
            })
            .collect::<Vec<_>>();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "schema_version": MACHINE_SCHEMA_VERSION,
                "entry": path,
                "routes": routes,
            }))
            .expect("route inventory JSON is serializable")
        );
    } else if entries.is_empty() {
        println!("No routes declared in {path}");
    } else {
        for entry in entries {
            let source = sources.get(entry.span.source_id as usize);
            let file = source.map_or(path.as_str(), |source| source.path.as_str());
            println!(
                "{:<5} {:<32} {:<22} {}:{}",
                entry.method, entry.path, entry.kind, file, entry.span.line
            );
        }
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::{route_collision_pairs, route_entries, routes_overlap, RouteEntry};
    use zelyra_ast::Program;

    #[test]
    fn route_patterns_overlap_by_segment_and_parameter_name() {
        assert!(routes_overlap("/customers/{id}", "/customers/{slug}"));
        assert!(!routes_overlap("/customers/{id}", "/customers/new"));
        assert!(!routes_overlap("/customers/{id}", "/customers/{id}/edit"));
    }

    #[test]
    fn route_inventory_is_sorted_and_empty_for_a_minimal_program() {
        let entries = route_entries(&Program {
            pages: vec![],
            tableviews: vec![],
            forms: vec![],
            cruds: vec![],
            auth: vec![],
            apis: vec![],
            functions: vec![],
            imports: vec![],
            tables: vec![],
            records: vec![],
            types: vec![],
            components: vec![],
            views: vec![],
            databases: vec![],
        });
        assert!(entries.is_empty());
    }

    #[test]
    fn collisions_require_same_method_and_normalized_path_pattern() {
        let span = zelyra_ast::Span::default();
        let route = |method: &str, path: &str| RouteEntry {
            method: method.into(),
            path: path.into(),
            kind: "api".into(),
            span,
        };
        let entries = vec![
            route("GET", "/customers/{id}"),
            route("GET", "/customers/{slug}"),
            route("POST", "/customers/{customer_id}"),
        ];
        assert_eq!(route_collision_pairs(&entries), vec![(0, 1)]);
    }
}
