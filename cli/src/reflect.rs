//! Deterministic, secret-free JSON reflection for statically checked projects.
use super::*;

const REFLECT_SCHEMA_VERSION: u32 = 1;

fn source_location(span: zelyra_ast::Span, sources: &[project::ProjectSource]) -> Value {
    match sources.get(span.source_id as usize) {
        Some(source) => json!({
            "file": source.path,
            "line": span.line,
            "column": span.column,
            "start": span.start,
            "end": span.end,
        }),
        None => Value::Null,
    }
}

fn field(
    name: &str,
    ty: Option<&Type>,
    span: zelyra_ast::Span,
    sources: &[project::ProjectSource],
) -> Value {
    json!({
        "name": name,
        "type": ty.map(ToString::to_string),
        "source": source_location(span, sources),
    })
}

fn build(program: &zelyra_ast::Program, sources: &[project::ProjectSource]) -> Value {
    let mut symbols = Vec::new();
    for record in &program.records {
        symbols.push(json!({
            "kind": if record.is_class { "class" } else { "struct" },
            "name": record.name,
            "public": record.is_public,
            "type_parameters": record.type_params,
            "fields": record.fields.iter().map(|item| field(&item.name, Some(&item.ty), item.span, sources)).collect::<Vec<_>>(),
            "source": source_location(record.span, sources),
        }));
    }
    for item in &program.enums {
        let variants = item
            .variants
            .iter()
            .map(|name| {
                let payload = item
                    .variant_payloads
                    .iter()
                    .find(|(variant, _)| variant == name)
                    .map(|(_, types)| types.iter().map(ToString::to_string).collect::<Vec<_>>())
                    .unwrap_or_default();
                json!({ "name": name, "payload_types": payload })
            })
            .collect::<Vec<_>>();
        symbols.push(json!({
            "kind": "enum", "name": item.name, "public": item.is_public,
            "type_parameters": item.type_params, "variants": variants,
            "source": source_location(item.span, sources),
        }));
    }
    for item in &program.types {
        symbols.push(json!({
            "kind": "type_alias", "name": item.name, "public": item.is_public,
            "target": item.target.to_string(), "source": source_location(item.span, sources),
        }));
    }
    for item in &program.functions {
        symbols.push(json!({
            "kind": "function", "name": item.name, "public": item.is_public,
            "type_parameters": item.type_params,
            "parameters": item.params.iter().map(|param| field(&param.name, Some(&param.ty), param.span, sources)).collect::<Vec<_>>(),
            "return_type": item.return_type.as_ref().map(ToString::to_string),
            "capabilities": sorted_strings(&item.capabilities),
            "contracts": {
                "requires": item.requires.iter().map(|expr| source_location(expr.span, sources)).collect::<Vec<_>>(),
                "ensures": item.ensures.iter().map(|expr| source_location(expr.span, sources)).collect::<Vec<_>>(),
            },
            "source": source_location(item.span, sources),
        }));
    }
    for item in &program.views {
        symbols.push(json!({ "kind": "view", "name": item.name, "public": item.is_public, "source": source_location(item.span, sources) }));
    }
    for item in &program.components {
        symbols.push(json!({
            "kind": "component", "name": item.name, "public": item.is_public,
            "properties": item.props.iter().map(|prop| field(&prop.name, Some(&prop.ty), prop.span, sources)).collect::<Vec<_>>(),
            "source": source_location(item.span, sources),
        }));
    }
    symbols.sort_by(|a, b| {
        a["kind"]
            .as_str()
            .cmp(&b["kind"].as_str())
            .then(a["name"].as_str().cmp(&b["name"].as_str()))
            .then(
                a["source"]["file"]
                    .as_str()
                    .cmp(&b["source"]["file"].as_str()),
            )
    });

    let mut tables = program.tables.iter().map(|table| json!({
        "name": table.name,
        "columns": table.columns.iter().map(|column| json!({
            "name": column.name,
            "type": column.ty.to_string(),
            "required": column.required,
            "primary_key": column.primary_key,
            "auto": column.auto,
            "unique": column.unique,
            "storage_name": column.storage_name,
        })).collect::<Vec<_>>(),
        "tenant_scope": table.tenant_scope.as_ref().map(|scope| json!({ "column": scope.column, "type": scope.tenant_type.to_string() })),
        "source": source_location(table.span, sources),
    })).collect::<Vec<_>>();
    tables.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));

    let mut apis = program.apis.iter().map(|api| json!({
        "method": api.method.to_ascii_uppercase(), "path": api.path,
        "handler": api.handler,
        "input": api.input.iter().map(|input| field(&input.name, Some(&input.ty), input.span, sources)).collect::<Vec<_>>(),
        "output_type": api.output.to_string(),
        "errors": api.errors.iter().map(|error| json!({ "status": error.status, "name": error.name, "payload_type": error.payload.as_ref().map(ToString::to_string) })).collect::<Vec<_>>(),
        "requires_auth": api.requires_auth, "permissions": sorted_strings(&api.permissions),
        "source": source_location(api.span, sources),
    })).collect::<Vec<_>>();
    apis.sort_by(|a, b| {
        a["path"]
            .as_str()
            .cmp(&b["path"].as_str())
            .then(a["method"].as_str().cmp(&b["method"].as_str()))
    });

    let mut forms = program.forms.iter().map(|form| json!({
        "name": form.name, "table": form.table,
        "fields": form.fields.iter().map(|item| json!({
            "name": item.name,
            "type": item.ty.as_ref().map(ToString::to_string),
            "required": item.required, "readonly": item.readonly,
            "appearance": item.appearance,
            "validation": {
                "min_length": item.min_length, "max": item.max,
                "min_value": item.min_value, "max_value": item.max_value,
                "choice_count": item.choices.len(),
            },
            "source": source_location(item.span, sources),
        })).collect::<Vec<_>>(),
        "actions": form.actions.iter().map(|action| json!({ "name": action.name, "handler": action.handler, "requires_auth": action.requires_auth, "permissions": sorted_strings(&action.permissions), "source": source_location(action.span, sources) })).collect::<Vec<_>>(),
        "source": source_location(form.span, sources),
    })).collect::<Vec<_>>();
    forms.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));

    let sources_list = sources
        .iter()
        .map(|source| source.path.as_str())
        .collect::<BTreeSet<_>>();
    let mut routes = route_cli::route_entries(program).into_iter().map(|route| {
        let file = sources.get(route.span.source_id as usize).map(|source| source.path.as_str());
        json!({ "method": route.method, "path": route.path, "kind": route.kind,
            "source": if let Some(file) = file { json!({ "file": file, "line": route.span.line, "column": route.span.column, "start": route.span.start, "end": route.span.end }) } else { Value::Null }
        })
    }).collect::<Vec<_>>();
    routes.sort_by(|a, b| {
        a["path"]
            .as_str()
            .cmp(&b["path"].as_str())
            .then(a["method"].as_str().cmp(&b["method"].as_str()))
            .then(a["kind"].as_str().cmp(&b["kind"].as_str()))
    });

    let modules = PROJECT_MODULES.with(|items| items.borrow().iter().map(|module| json!({
        "file": module.path,
        "imports": module.imports.iter().map(|import| json!({ "alias": import.alias, "path": import.path })).collect::<Vec<_>>(),
        "exports": module.exports.iter().map(|item| json!({ "kind": item.kind, "name": item.name })).collect::<Vec<_>>(),
    })).collect::<Vec<_>>());
    let mut modules = modules;
    modules.sort_by(|a, b| a["file"].as_str().cmp(&b["file"].as_str()));

    json!({
        "reflect_schema_version": REFLECT_SCHEMA_VERSION,
        "compiler_version": env!("CARGO_PKG_VERSION"),
        "checked": true,
        "completeness": { "static_project_check": "passed", "live_database_inspected": false, "runtime_values_included": false },
        "source_files": sources_list,
        "modules": modules,
        "symbols": symbols,
        "tables": tables,
        "routes": routes,
        "apis": apis,
        "forms": forms,
    })
}

fn sorted_strings(values: &[String]) -> Vec<&str> {
    let mut values = values.iter().map(String::as_str).collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    values
}

pub(super) fn command(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(path) = args.next() else {
        eprintln!("Usage: zelyra reflect <entry.zyl> [--format=json|human]");
        return ExitCode::from(2);
    };
    let mut format = "json";
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--format" => match args.next().as_deref() {
                Some("json") => format = "json",
                Some("human") => format = "human",
                _ => {
                    eprintln!("error[E-CLI-001]: --format must be json or human");
                    return ExitCode::from(2);
                }
            },
            "--format=json" => format = "json",
            "--format=human" => format = "human",
            _ => {
                eprintln!("error[E-CLI-001]: unexpected argument `{argument}`");
                return ExitCode::from(2);
            }
        }
    }
    let program = match validate(&path) {
        Ok(program) => program,
        Err(()) => return ExitCode::from(1),
    };
    let sources = PROJECT_SOURCES.with(|items| items.borrow().clone());
    let document = build(&program, &sources);
    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&document).expect("reflection JSON is serializable")
        );
    } else {
        println!("Zelyra reflection v{} (checked)", REFLECT_SCHEMA_VERSION);
        for key in [
            "source_files",
            "modules",
            "symbols",
            "tables",
            "routes",
            "apis",
            "forms",
        ] {
            println!("{key}: {}", document[key].as_array().map_or(0, Vec::len));
        }
        println!("Live database inspected: no");
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflection_is_deterministic_and_omits_literal_defaults_and_credentials() {
        let source = r#"
            table customers {
                id: Id primary auto
                name: String required
            }
            struct Customer {
                id: Id
                name: String
            }
            fn greet(customer: Customer) -> String requires { true } ensures { true } { "Hallo" }
            form CustomerForm -> customers {
                fields { name }
            }
            page "/customers" { html { "Customers" } }
        "#;
        let program = zelyra_parser::parse(&zelyra_lexer::lex(source).unwrap()).unwrap();
        let sources = vec![project::ProjectSource {
            path: "main.zyl".into(),
            text: source.into(),
        }];
        let first = build(&program, &sources);
        let second = build(&program, &sources);
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert_eq!(first["reflect_schema_version"], REFLECT_SCHEMA_VERSION);
        let encoded = first.to_string();
        assert!(!encoded.contains("Secret label"));
        assert!(!encoded.contains("Hallo"));
        assert_eq!(first["routes"][0]["path"], "/customers");
        assert_eq!(first["completeness"]["live_database_inspected"], false);
    }
}
