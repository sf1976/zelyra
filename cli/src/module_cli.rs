use super::*;

fn context_span(fallback_source: &str, span: zelyra_ast::Span) -> Value {
    let source =
        PROJECT_SOURCES.with(|sources| sources.borrow().get(span.source_id as usize).cloned());
    let source_text = source
        .as_ref()
        .map_or(fallback_source, |source| source.text.as_str());
    let (end_line, end_column) = source_position(source_text, span.end);
    json!({
        "file": source.as_ref().map(|source| source.path.as_str()),
        "start": { "offset": span.start, "line": span.line, "column": span.column },
        "end": { "offset": span.end, "line": end_line, "column": end_column }
    })
}

fn default_value_json(value: &zelyra_ast::DefaultValue) -> Value {
    match value {
        zelyra_ast::DefaultValue::Int(value) => json!(value),
        zelyra_ast::DefaultValue::Bool(value) => json!(value),
        zelyra_ast::DefaultValue::String(value) => json!(value),
        zelyra_ast::DefaultValue::Ident(value) => json!(value),
    }
}

pub(super) fn project_name(path: &str) -> Option<String> {
    let config_path = project_config_path(path).ok().flatten()?;
    let contents = fs::read_to_string(config_path).ok()?;
    let mut in_project = false;
    for raw_line in contents.lines() {
        let line = raw_line.split('#').next()?.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_project = line == "[project]";
            continue;
        }
        if in_project {
            let (key, value) = line.split_once('=')?;
            if key.trim() == "name" {
                return value
                    .trim()
                    .strip_prefix('"')
                    .and_then(|value| value.strip_suffix('"'))
                    .map(str::to_owned);
            }
        }
    }
    None
}

pub(super) fn context_entry(path: &str) -> String {
    let source_path = fs::canonicalize(path).ok();
    let root = project_config_path(path)
        .ok()
        .flatten()
        .and_then(|path| path.parent().map(PathBuf::from));
    if let (Some(source_path), Some(root)) = (source_path, root) {
        if let Ok(relative) = source_path.strip_prefix(root) {
            return relative.to_string_lossy().replace('\\', "/");
        }
    }
    path.replace('\\', "/")
}

fn context_view_slots(html: &str) -> Vec<Value> {
    slot_invocations(html)
        .unwrap_or_default()
        .into_iter()
        .map(|slot| {
            json!({
                "name": slot.name.unwrap_or_else(|| "default".into()),
                "fallback": slot.body.is_some()
            })
        })
        .collect()
}

fn context_declarations(program: &zelyra_ast::Program, source: &str) -> Value {
    let databases = program
        .databases
        .iter()
        .map(|database| {
            json!({
                "name": database.name,
                "engine": database.engine,
                "database": database.database,
                "span": context_span(source, database.span)
            })
        })
        .collect::<Vec<_>>();
    let tables = program
        .tables
        .iter()
        .map(|table| {
            let fields = table
                .columns
                .iter()
                .map(|column| {
                    json!({
                        "name": column.name,
                        "type": column.ty.to_string(),
                        "optional": !column.required,
                        "primary_key": column.primary_key,
                        "auto_increment": column.auto,
                        "unique": column.unique,
                        "default": column.default.as_ref().map(default_value_json),
                        "span": context_span(source, column.span)
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "name": table.name,
                "fields": fields,
                "span": context_span(source, table.span)
            })
        })
        .collect::<Vec<_>>();
    let cruds = program
        .cruds
        .iter()
        .map(|crud| {
            json!({
                "name": crud.name,
                "table": crud.table,
                "layout": crud.layout,
                "layout_slots": crud.layout_slots.iter().map(|slot| json!({
                    "name": slot.name,
                    "span": context_span(source, slot.span)
                })).collect::<Vec<_>>(),
                "view_fields": crud.view.fields,
                "span": context_span(source, crud.span)
            })
        })
        .collect::<Vec<_>>();
    let views = program
        .views
        .iter()
        .map(|view| {
            json!({
                "name": view.name,
                "input_type": Value::Null,
                "used_fields": Vec::<String>::new(),
                "slots": context_view_slots(&view.html),
                "span": context_span(source, view.span)
            })
        })
        .collect::<Vec<_>>();
    let components = program
        .components
        .iter()
        .map(|component| {
            json!({
                "name": component.name,
                "props": component.props.iter().map(|prop| json!({
                    "name": prop.name,
                    "type": prop.ty.to_string()
                })).collect::<Vec<_>>(),
                "slots": context_view_slots(&component.html),
                "span": context_span(source, component.span)
            })
        })
        .collect::<Vec<_>>();
    let pages = program
        .pages
        .iter()
        .map(|page| {
            let data = page
                .data
                .iter()
                .map(|binding| {
                    json!({
                        "name": binding.name,
                        "type": binding.result_type.to_string(),
                        "fields": page_data_fields(program, &binding.result_type),
                        "span": context_span(source, binding.span)
                    })
                })
                .collect::<Vec<_>>();
            let inputs = page
                .inputs
                .iter()
                .map(|input| {
                    json!({
                        "name": input.name,
                        "type": input.ty.to_string(),
                        "span": context_span(source, input.span)
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "path": page.path,
                "view": page.view,
                "inputs": inputs,
                "page_size": page.page_size,
                "sort": page.sort,
                "search": page.search,
                "filters": page.filters,
                "data": data,
                "span": context_span(source, page.span)
            })
        })
        .collect::<Vec<_>>();
    let tableviews = program
        .tableviews
        .iter()
        .map(|view| {
            json!({
                "name": view.name,
                "result_type": view.result_type.to_string(),
                "fields": view.columns,
                "span": context_span(source, view.span)
            })
        })
        .collect::<Vec<_>>();
    let forms = program
        .forms
        .iter()
        .map(|form| {
            json!({
                "name": form.name,
                "table": form.table,
                "fields": form.fields.iter().map(|field| field.name.clone()).collect::<Vec<_>>(),
                "span": context_span(source, form.span)
            })
        })
        .collect::<Vec<_>>();
    let apis = program
        .apis
        .iter()
        .map(|api| {
            json!({
                "method": api.method,
                "path": api.path,
                "input": api.input.iter().map(|field| json!({ "name": field.name, "type": field.ty.to_string() })).collect::<Vec<_>>(),
                "output": api.output.to_string(),
                "span": context_span(source, api.span)
            })
        })
        .collect::<Vec<_>>();
    let auth = program
        .auth
        .iter()
        .map(|auth| {
            json!({
                "name": auth.name,
                "table": auth.table,
                "audit_table": auth.audit_table,
                "audit_chain": auth.audit_chain,
                "span": context_span(source, auth.span)
            })
        })
        .collect::<Vec<_>>();
    json!({
        "databases": databases,
        "tables": tables,
        "cruds": cruds,
        "pages": pages,
        "views": views,
        "components": components,
        "tableviews": tableviews,
        "forms": forms,
        "apis": apis,
        "auth": auth
    })
}

fn empty_context_declarations() -> Value {
    json!({
        "databases": [],
        "tables": [],
        "cruds": [],
        "pages": [],
        "views": [],
        "components": [],
        "tableviews": [],
        "forms": [],
        "apis": [],
        "auth": []
    })
}

fn context_modules() -> Vec<project::ProjectModule> {
    PROJECT_MODULES.with(|modules| modules.borrow().clone())
}

fn context_modules_json(modules: &[project::ProjectModule]) -> Value {
    json!(modules
        .iter()
        .map(|module| json!({
            "path": module.path,
            "imports": module.imports.iter().map(|import| json!({
                "alias": import.alias,
                "path": import.path
            })).collect::<Vec<_>>(),
            "exports": module.exports.iter().map(|export| json!({
                "kind": export.kind,
                "name": export.name
            })).collect::<Vec<_>>()
        }))
        .collect::<Vec<_>>())
}

pub(super) fn module_declaration_owners(program: &zelyra_ast::Program) -> HashMap<String, String> {
    let source_paths = PROJECT_SOURCES.with(|sources| {
        sources
            .borrow()
            .iter()
            .map(|source| source.path.clone())
            .collect::<Vec<_>>()
    });
    let mut owners = HashMap::new();
    let mut insert = |name: String, span: zelyra_ast::Span| {
        if let Some(path) = source_paths.get(span.source_id as usize) {
            owners.insert(name, path.clone());
        }
    };
    for database in &program.databases {
        insert(format!("database:{}", database.name), database.span);
    }
    for table in &program.tables {
        insert(format!("table:{}", table.name), table.span);
    }
    for tableview in &program.tableviews {
        insert(format!("tableview:{}", tableview.name), tableview.span);
    }
    for definition in &program.types {
        insert(format!("type:{}", definition.name), definition.span);
    }
    for record in &program.records {
        insert(format!("record:{}", record.name), record.span);
    }
    for page in &program.pages {
        insert(format!("page:{}", page.path), page.span);
    }
    for view in &program.views {
        insert(format!("view:{}", view.name), view.span);
    }
    for component in &program.components {
        insert(format!("component:{}", component.name), component.span);
    }
    for function in &program.functions {
        insert(format!("function:{}", function.name), function.span);
    }
    for form in &program.forms {
        insert(format!("form:{}", form.name), form.span);
    }
    for crud in &program.cruds {
        insert(format!("crud:{}", crud.name), crud.span);
    }
    for api in &program.apis {
        insert(format!("api:{} {}", api.method, api.path), api.span);
    }
    for auth in &program.auth {
        insert(format!("auth:{}", auth.name), auth.span);
    }
    owners
}

fn module_runtime_effects(
    program: &zelyra_ast::Program,
    owners: &HashMap<String, String>,
    included: &BTreeSet<String>,
) -> (Vec<Value>, Vec<Value>) {
    let mut functions_by_capability = BTreeMap::<String, BTreeSet<String>>::new();
    for function in &program.functions {
        let declaration = format!("function:{}", function.name);
        if !owners
            .get(&declaration)
            .is_some_and(|module| included.contains(module))
        {
            continue;
        }
        for capability in &function.capabilities {
            functions_by_capability
                .entry(capability.clone())
                .or_default()
                .insert(declaration.clone());
        }
    }

    let effects = functions_by_capability
        .iter()
        .map(|(capability, functions)| {
            json!({
                "capability": capability,
                "declared_by": functions,
                "source_modules": functions.iter().filter_map(|function| {
                    owners.get(function).cloned()
                }).collect::<BTreeSet<_>>()
            })
        })
        .collect::<Vec<_>>();
    let blockers = functions_by_capability
        .iter()
        .filter_map(|(capability, functions)| {
            let reason = match capability.as_str() {
                "FileSystem" => Some("file_system_paths_are_not_declared_in_the_bundle_manifest"),
                "Network" => Some("outbound_network_service_contract_is_not_declared"),
                "Process" => Some("process_executable_and_runtime_dependencies_are_not_declared"),
                "Environment" => Some("environment_variable_names_are_not_declared"),
                _ => None,
            }?;
            Some(json!({
                "kind": "runtime_capability",
                "capability": capability,
                "reason": reason,
                "declared_by": functions
            }))
        })
        .collect::<Vec<_>>();
    (effects, blockers)
}

fn module_external_service_contracts(
    program: &zelyra_ast::Program,
    owners: &HashMap<String, String>,
    included: &BTreeSet<String>,
) -> Vec<Value> {
    program
        .auth
        .iter()
        .filter(|auth| auth.reset_tokens_table.is_some())
        .filter(|auth| {
            owners
                .get(&format!("auth:{}", auth.name))
                .is_some_and(|module| included.contains(module))
        })
        .map(|auth| {
            json!({
                "kind": "smtp",
                "used_by": format!("auth:{}", auth.name),
                "required_when": "password_reset_email_delivery_is_used",
                "environment_variables": [
                    "ZELYRA_SMTP_HOST",
                    "ZELYRA_SMTP_PORT",
                    "ZELYRA_SMTP_SECURITY",
                    "ZELYRA_SMTP_FROM",
                    "ZELYRA_SMTP_USERNAME",
                    "ZELYRA_SMTP_PASSWORD"
                ],
                "secret_environment_variables": [
                    "ZELYRA_SMTP_USERNAME",
                    "ZELYRA_SMTP_PASSWORD"
                ],
                "delivery_semantics": "at_least_once"
            })
        })
        .collect()
}

pub(super) fn module_uses_database(
    program: &zelyra_ast::Program,
    module_path: &str,
    owners: &HashMap<String, String>,
) -> bool {
    let owned = |node: &str| owners.get(node).is_some_and(|path| path == module_path);
    program
        .tableviews
        .iter()
        .any(|view| owned(&format!("tableview:{}", view.name)))
        || program
            .pages
            .iter()
            .any(|page| !page.data.is_empty() && owned(&format!("page:{}", page.path)))
        || program
            .forms
            .iter()
            .any(|form| owned(&format!("form:{}", form.name)))
        || program
            .cruds
            .iter()
            .any(|crud| owned(&format!("crud:{}", crud.name)))
        || program
            .auth
            .iter()
            .any(|auth| owned(&format!("auth:{}", auth.name)))
        || program.functions.iter().any(|function| {
            owned(&format!("function:{}", function.name))
                && function
                    .capabilities
                    .iter()
                    .any(|capability| capability.starts_with("Database"))
        })
}

fn is_application_resource_id(resource: &str) -> bool {
    matches!(
        resource.split_once(':').map(|(kind, _)| kind),
        Some("page" | "api" | "crud" | "form" | "tableview")
    )
}

fn qualify_impact_declaration(
    declaration: &str,
    owners: &HashMap<String, String>,
) -> Option<String> {
    let module_path = owners.get(declaration)?;
    let (kind, name) = declaration.split_once(':')?;
    let local_name = if matches!(kind, "function" | "record" | "type") {
        name.rsplit_once("::").map_or(name, |(_, name)| name)
    } else {
        name
    };
    Some(format!("{module_path}::{kind}:{local_name}"))
}

pub(super) fn module_command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    match arguments.next().as_deref() {
        Some("plan") => module_plan_command(arguments),
        Some("bundle") => module_bundle_command(arguments),
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}

fn module_plan_command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let (Some(entry), Some(selected)) = (arguments.next(), arguments.next()) else {
        usage();
        return ExitCode::from(2);
    };
    if arguments.next().is_some() {
        usage();
        return ExitCode::from(2);
    }

    let source = fs::read_to_string(&entry).unwrap_or_default();
    begin_json_diagnostics(&entry, &source);
    let validation = validate(&entry);
    let modules = PROJECT_MODULES.with(|modules| modules.borrow().clone());
    let mut plan = None;
    if let Ok(program) = validation {
        let selected = selected.replace('\\', "/");
        let by_path = modules
            .iter()
            .map(|module| (module.path.as_str(), module))
            .collect::<HashMap<_, _>>();
        let owners = module_declaration_owners(&program);
        let selected_resource = if by_path.contains_key(selected.as_str()) {
            None
        } else if is_application_resource_id(&selected) && owners.contains_key(&selected) {
            Some(selected.clone())
        } else {
            None
        };
        let selected_module = selected_resource
            .as_ref()
            .and_then(|resource| owners.get(resource).cloned())
            .or_else(|| {
                by_path
                    .contains_key(selected.as_str())
                    .then(|| selected.clone())
            });
        if let Some(selected_module) = selected_module {
            let project_sources = PROJECT_SOURCES.with(|sources| sources.borrow().clone());
            let impact = build_impact_with_modules(&program, &project_sources, &modules, &source);
            let references = impact
                .get("references")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut pending = vec![selected_module.clone()];
            let mut included = BTreeSet::new();
            let mut graph_is_complete = true;
            let mut unresolved_references = BTreeSet::new();
            let mut database_required = false;
            while let Some(path) = pending.pop() {
                if !included.insert(path.clone()) {
                    continue;
                }
                let Some(module) = by_path.get(path.as_str()) else {
                    diagnostic(
                        &entry,
                        "E-MOD-014",
                        &format!("module dependency `{path}` is missing from the loaded graph"),
                        1,
                        1,
                    );
                    graph_is_complete = false;
                    break;
                };
                pending.extend(module.imports.iter().map(|import| import.path.clone()));
                if module_uses_database(&program, &path, &owners) {
                    database_required = true;
                    for database in &program.databases {
                        if let Some(database_path) =
                            owners.get(&format!("database:{}", database.name))
                        {
                            pending.push(database_path.clone());
                        }
                    }
                }
                for reference in &references {
                    let Some(from) = reference.get("from").and_then(Value::as_str) else {
                        continue;
                    };
                    if owners.get(from).map(String::as_str) != Some(path.as_str()) {
                        continue;
                    }
                    let Some(to) = reference.get("to").and_then(Value::as_str) else {
                        continue;
                    };
                    if let Some(dependency_path) = owners.get(to) {
                        pending.push(dependency_path.clone());
                    } else {
                        unresolved_references.insert((
                            path.clone(),
                            from.to_owned(),
                            to.to_owned(),
                        ));
                    }
                }
            }
            if graph_is_complete {
                let closure = included
                    .iter()
                    .filter_map(|path| by_path.get(path.as_str()).copied())
                    .collect::<Vec<_>>();
                let declaration_roots = selected_resource
                    .as_ref()
                    .map(|resource| vec![format!("{selected_module}::{resource}")])
                    .unwrap_or_else(|| {
                        by_path
                            .get(selected_module.as_str())
                            .map(|module| {
                                module
                                    .declarations
                                    .iter()
                                    .map(|declaration| format!("{}::{declaration}", module.path))
                                    .collect()
                            })
                            .unwrap_or_default()
                    });
                let mut declaration_pending = declaration_roots.clone();
                let mut declaration_closure = BTreeSet::new();
                while let Some(current) = declaration_pending.pop() {
                    if !declaration_closure.insert(current.clone()) {
                        continue;
                    }
                    for reference in &references {
                        let Some(from) = reference.get("from").and_then(Value::as_str) else {
                            continue;
                        };
                        if qualify_impact_declaration(from, &owners).as_deref()
                            != Some(current.as_str())
                        {
                            continue;
                        }
                        let Some(to) = reference.get("to").and_then(Value::as_str) else {
                            continue;
                        };
                        if let Some(to) = qualify_impact_declaration(to, &owners) {
                            declaration_pending.push(to);
                        }
                    }
                }
                if database_required {
                    for database in &program.databases {
                        let database_id = format!("database:{}", database.name);
                        if let Some(module_path) = owners.get(&database_id) {
                            if included.contains(module_path) {
                                declaration_closure.insert(format!("{module_path}::{database_id}"));
                            }
                        }
                    }
                }
                let source_only_declarations = closure
                    .iter()
                    .flat_map(|module| {
                        module
                            .declarations
                            .iter()
                            .map(|declaration| format!("{}::{declaration}", module.path))
                    })
                    .filter(|declaration| !declaration_closure.contains(declaration))
                    .collect::<BTreeSet<_>>();
                let declaration_edges = references
                    .iter()
                    .filter(|reference| {
                        let from = reference
                            .get("from")
                            .and_then(Value::as_str)
                            .and_then(|from| qualify_impact_declaration(from, &owners));
                        let to = reference
                            .get("to")
                            .and_then(Value::as_str)
                            .and_then(|to| qualify_impact_declaration(to, &owners));
                        from.as_ref().is_some_and(|from| {
                            declaration_closure.contains(from)
                                && to
                                    .as_ref()
                                    .is_some_and(|to| declaration_closure.contains(to))
                        })
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let mut resource_dependencies = BTreeMap::new();
                for reference in &references {
                    let Some(from) = reference.get("from").and_then(Value::as_str) else {
                        continue;
                    };
                    let Some(from_path) = owners.get(from) else {
                        continue;
                    };
                    let Some(to) = reference.get("to").and_then(Value::as_str) else {
                        continue;
                    };
                    let Some(to_path) = owners.get(to) else {
                        continue;
                    };
                    if from_path == to_path
                        || !included.contains(from_path)
                        || !included.contains(to_path)
                    {
                        continue;
                    }
                    let kind = reference
                        .get("kind")
                        .and_then(Value::as_str)
                        .unwrap_or("reference");
                    let access = reference
                        .get("access")
                        .and_then(Value::as_str)
                        .unwrap_or("none");
                    let key = (
                        from_path.clone(),
                        from.to_owned(),
                        to_path.clone(),
                        to.to_owned(),
                        kind.to_owned(),
                        access.to_owned(),
                    );
                    resource_dependencies.insert(
                        key,
                        json!({
                            "from_module": from_path,
                            "from": from,
                            "to_module": to_path,
                            "to": to,
                            "kind": kind,
                            "access": reference.get("access").and_then(Value::as_str),
                            "span": reference.get("span")
                        }),
                    );
                }
                if database_required {
                    for module_path in &included {
                        if !module_uses_database(&program, module_path, &owners) {
                            continue;
                        }
                        for database in &program.databases {
                            let database_node = format!("database:{}", database.name);
                            let Some(database_path) = owners.get(&database_node) else {
                                continue;
                            };
                            if module_path == database_path {
                                continue;
                            }
                            let key = (
                                module_path.clone(),
                                "database:runtime".into(),
                                database_path.clone(),
                                database_node.clone(),
                                "database_configuration".into(),
                                "none".into(),
                            );
                            resource_dependencies.insert(
                                key,
                                json!({
                                    "from_module": module_path,
                                    "from": "database:runtime",
                                    "to_module": database_path,
                                    "to": database_node,
                                    "kind": "database_configuration"
                                }),
                            );
                        }
                    }
                }
                let database_configuration_sources = program
                    .databases
                    .iter()
                    .filter_map(|database| owners.get(&format!("database:{}", database.name)))
                    .filter(|path| included.contains(*path))
                    .cloned()
                    .collect::<BTreeSet<_>>();
                let database_configurations = program
                    .databases
                    .iter()
                    .filter_map(|database| {
                        let declaration = format!("database:{}", database.name);
                        let module_path = owners.get(&declaration)?;
                        included.contains(module_path).then(|| {
                            (
                                format!("{module_path}::{declaration}"),
                                json!({
                                    "declaration": declaration,
                                    "module": module_path,
                                    "engine": database.engine,
                                    "database": database.database,
                                    "connection_environment": database_url_environment_name(&database.name)
                                }),
                            )
                        })
                    })
                    .collect::<BTreeMap<_, _>>();
                let schema_ownership = program
                    .tables
                    .iter()
                    .filter_map(|table| {
                        let declaration = format!("table:{}", table.name);
                        let module_path = owners.get(&declaration)?;
                        included.contains(module_path).then(|| {
                            (
                                table.name.clone(),
                                json!({
                                "table": table.name,
                                "inferred_owner_module": module_path,
                                "ownership_enforced": false,
                                "cross_module_access_grants_enforced": true,
                                "access_grants": {
                                    "read": table.access.read,
                                    "write": table.access.write,
                                    "read_write": table.access.read_write
                                }
                                }),
                            )
                        })
                    })
                    .collect::<BTreeMap<_, _>>();
                let (runtime_effects, mut deployment_blockers) =
                    module_runtime_effects(&program, &owners, &included);
                let external_service_contracts =
                    module_external_service_contracts(&program, &owners, &included);
                if !unresolved_references.is_empty() {
                    deployment_blockers.push(json!({
                        "kind": "unresolved_reference",
                        "count": unresolved_references.len(),
                        "reason": "one_or_more_static_references_have_no_declaration_owner"
                    }));
                }
                plan = Some(json!({
                    "kind": "known-semantic-dependency-closure",
                    "closure_semantics": "explicit-imports-plus-statically-recognized-references",
                    "selected_module": selected_module,
                    "selected_resource": selected_resource,
                    "selection_kind": if selected_resource.is_some() { "resource" } else { "module" },
                    "entry": context_entry(&entry),
                    "source_files": included,
                    "declaration_closure": {
                        "semantics": "selected-root-plus-statically-recognized-impact-references-and-database-configuration",
                        "complete": false,
                        "roots": declaration_roots,
                        "declarations": declaration_closure,
                        "edges": declaration_edges,
                        "configuration_edges": resource_dependencies
                            .values()
                            .filter(|dependency| dependency["kind"] == "database_configuration")
                            .cloned()
                            .collect::<Vec<_>>(),
                        "additional_declarations_in_included_source_files": source_only_declarations
                    },
                    "modules": closure.iter().map(|module| json!({
                        "path": module.path,
                        "imports": module.imports.iter().map(|import| json!({
                            "alias": import.alias,
                            "path": import.path
                        })).collect::<Vec<_>>(),
                        "exports": module.exports.iter().map(|export| json!({
                            "kind": export.kind,
                            "name": export.name
                        })).collect::<Vec<_>>(),
                        "declarations": module.declarations
                    })).collect::<Vec<_>>(),
                    "resource_dependencies": resource_dependencies.values().cloned().collect::<Vec<_>>(),
                    "runtime_effects": {
                        "model": "capabilities_declared_by_functions_in_included_source_modules",
                        "effects": runtime_effects,
                        "explicit_function_capabilities_complete": true,
                        "implicit_resource_effects_complete": false
                    },
                    "external_service_contracts": external_service_contracts,
                    "deployment_readiness": {
                        "status": "incomplete",
                        "ready": false,
                        "blockers": deployment_blockers,
                        "unverified_requirements": [
                            "resource-level effects and service contracts are not fully modeled",
                            "runtime and asset dependencies are not proven complete"
                        ],
                        "note": "A complete source import closure does not prove that external runtime effects or service contracts are deployable."
                    },
                    "schema_ownership": {
                        "model": "inferred_from_table_declaration_source_module",
                        "enforced": false,
                        "cross_module_access_grants_enforced": true,
                        "schema_change_ownership_enforced": false,
                        "tables": schema_ownership.values().collect::<Vec<_>>()
                    },
                    "table_access_contract": {
                        "model": "table-owner-module-in-consumer-import-closure",
                        "dependency_enforced": true,
                        "module_access_grants_enforced": true,
                        "cross_module_mutation_policy": "explicit-table-access-grant",
                        "unknown_sql_access_requires_read_write_grant": true,
                        "entry_module_tables_project_visible": false,
                        "analysis_complete": false
                    },
                    "unresolved_references": unresolved_references.iter().map(|(module, from, to)| json!({
                        "from_module": module,
                        "from": from,
                        "to": to
                    })).collect::<Vec<_>>(),
                    "database": {
                        "required": database_required,
                        "configuration_sources": database_configuration_sources,
                        "configurations": database_configurations.values().collect::<Vec<_>>(),
                        "connection_model": "single-project-wide-connection",
                        "connection_environment": program.databases.first()
                            .map(|database| database_url_environment_name(&database.name))
                            .unwrap_or_else(|| "DATABASE_URL".to_owned()),
                        "supports_multiple_connections": false
                    },
                    "complete_deployment": false,
                    "limitations": [
                        "Only dependency kinds recognized by the current static impact graph are followed.",
                        "Project configuration, runtime adapters, assets, external service contracts, and Docker artifacts are not included."
                    ],
                    "note": "This read-only preview is not a complete deployment manifest, runnable application, or Docker export."
                }));
            }
        } else {
            diagnostic(
                &entry,
                "E-MOD-013",
                &format!(
                    "`{selected}` is neither a reachable module nor a supported application resource"
                ),
                1,
                1,
            );
        }
    }
    let diagnostics = finish_json_diagnostics();
    let success = plan.is_some() && diagnostics.is_empty();
    print_machine_document(&machine_document(
        "module plan",
        success,
        diagnostics,
        [("plan".into(), plan.unwrap_or(Value::Null))],
    ));
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

type ModuleBundleFiles = (Vec<String>, Vec<String>, Vec<String>);

pub(super) fn publish_directory_no_replace(
    staging: &std::path::Path,
    output: &std::path::Path,
) -> std::io::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        use std::os::unix::ffi::OsStrExt;

        let staging = std::ffi::CString::new(staging.as_os_str().as_bytes()).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "staging path contains a NUL byte",
            )
        })?;
        let output = std::ffi::CString::new(output.as_os_str().as_bytes()).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "output path contains a NUL byte",
            )
        })?;
        // renameat2 with RENAME_NOREPLACE is atomic and cannot replace a
        // destination created after the earlier user-facing existence check.
        let result = unsafe {
            libc::renameat2(
                libc::AT_FDCWD,
                staging.as_ptr(),
                libc::AT_FDCWD,
                output.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        return if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        };
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        use std::os::unix::ffi::OsStrExt;

        let staging = std::ffi::CString::new(staging.as_os_str().as_bytes()).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "staging path contains a NUL byte",
            )
        })?;
        let output = std::ffi::CString::new(output.as_os_str().as_bytes()).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "output path contains a NUL byte",
            )
        })?;
        // RENAME_EXCL gives renameatx_np the same no-replacement guarantee.
        let result = unsafe {
            libc::renameatx_np(
                libc::AT_FDCWD,
                staging.as_ptr(),
                libc::AT_FDCWD,
                output.as_ptr(),
                libc::RENAME_EXCL,
            )
        };
        return if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        };
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        #[link(name = "Kernel32")]
        extern "system" {
            fn MoveFileW(existing_file_name: *const u16, new_file_name: *const u16) -> i32;
        }

        let staging = staging
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let output = output
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        // MoveFileW fails when the destination already exists (unlike APIs
        // that request MOVEFILE_REPLACE_EXISTING).
        let result = unsafe { MoveFileW(staging.as_ptr(), output.as_ptr()) };
        return if result != 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        };
    }

    #[allow(unreachable_code)]
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "atomic no-replace directory publication is unsupported on this platform",
    ))
}

fn module_bundle_command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let (Some(entry), Some(selected)) = (arguments.next(), arguments.next()) else {
        usage();
        return ExitCode::from(2);
    };
    let mut output = None;
    let mut docker = false;
    let mut dry_run = false;
    let mut compiler_ref = None;
    while let Some(argument) = arguments.next() {
        if argument == "--output" && output.is_none() {
            output = arguments.next().map(PathBuf::from);
        } else if argument == "--dry-run" && !dry_run {
            dry_run = true;
        } else if argument == "--docker" && !docker {
            docker = true;
        } else if argument == "--compiler-ref" && compiler_ref.is_none() {
            compiler_ref = arguments.next();
        } else {
            eprintln!("error[E-CLI-001]: expected `--output <directory>` and optional `--docker --compiler-ref <40-character-commit>`");
            return ExitCode::from(2);
        }
    }
    let Some(output) = output else {
        eprintln!("error[E-CLI-001]: `module bundle` requires `--output <directory>`");
        return ExitCode::from(2);
    };
    if docker && !compiler_ref.as_deref().is_some_and(is_full_git_commit) {
        eprintln!("error[E-CLI-001]: `--docker` requires `--compiler-ref` with a full 40-character hexadecimal Git commit");
        return ExitCode::from(2);
    }
    if !docker && compiler_ref.is_some() {
        eprintln!("error[E-CLI-001]: `--compiler-ref` can only be used together with `--docker`");
        return ExitCode::from(2);
    }

    let executable = match env::current_exe() {
        Ok(executable) => executable,
        Err(error) => {
            eprintln!("error[E-MOD-018]: cannot locate the Zelyra executable: {error}");
            return ExitCode::from(1);
        }
    };
    let plan_output = match Command::new(&executable)
        .args(["module", "plan", &entry, &selected])
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            eprintln!("error[E-MOD-018]: cannot run the module dependency planner: {error}");
            return ExitCode::from(1);
        }
    };
    if !plan_output.status.success() {
        eprint!("{}", String::from_utf8_lossy(&plan_output.stdout));
        return ExitCode::from(1);
    }
    let plan_document: Value = match serde_json::from_slice(&plan_output.stdout) {
        Ok(document) => document,
        Err(error) => {
            eprintln!("error[E-MOD-018]: dependency planner returned invalid JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let Some(plan) = plan_document.get("plan") else {
        eprintln!("error[E-MOD-018]: dependency planner returned no plan");
        return ExitCode::from(1);
    };
    let Some(source_files) = plan.get("source_files").and_then(Value::as_array) else {
        eprintln!("error[E-MOD-018]: dependency plan has no source-file inventory");
        return ExitCode::from(1);
    };
    let source_files = source_files
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if source_files.is_empty() {
        eprintln!("error[E-MOD-018]: dependency plan contains no source files");
        return ExitCode::from(1);
    }
    if plan
        .get("unresolved_references")
        .and_then(Value::as_array)
        .is_none_or(|references| !references.is_empty())
    {
        eprintln!("error[E-MOD-018]: refusing to bundle a plan with unresolved references");
        return ExitCode::from(1);
    }

    let entry_path = match fs::canonicalize(&entry) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("error[E-MOD-018]: cannot resolve entry `{entry}`: {error}");
            return ExitCode::from(1);
        }
    };
    let Some(project_root) = entry_path.parent() else {
        eprintln!("error[E-MOD-018]: entry has no project directory");
        return ExitCode::from(1);
    };
    let entry_relative = match entry_path.strip_prefix(project_root) {
        Ok(path) => path.to_string_lossy().replace('\\', "/"),
        Err(error) => {
            eprintln!("error[E-MOD-018]: cannot make entry project-relative: {error}");
            return ExitCode::from(1);
        }
    };
    let selected_module = plan
        .get("selected_module")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let selected_resource = plan
        .get("selected_resource")
        .and_then(Value::as_str)
        .is_some();
    let full_project = selected_module == entry_relative && !selected_resource;
    if selected_module == entry_relative && selected_resource {
        eprintln!("error[E-MOD-018]: move the selected resource into its own source module before bundling; the project entry file cannot be isolated safely");
        return ExitCode::from(1);
    }
    if !full_project && source_files.contains(&entry_relative) {
        eprintln!("error[E-MOD-018]: selected module depends on the original project entry; split the entry from reusable application modules before bundling");
        return ExitCode::from(1);
    }

    let output_name = output.file_name().filter(|name| !name.is_empty());
    let Some(output_name) = output_name else {
        eprintln!("error[E-MOD-018]: output must name a new directory");
        return ExitCode::from(1);
    };
    let output_parent = output.parent().unwrap_or_else(|| std::path::Path::new("."));
    let output_parent = match fs::canonicalize(output_parent) {
        Ok(path) if path.is_dir() => path,
        Ok(_) => {
            eprintln!("error[E-MOD-018]: output parent is not a directory");
            return ExitCode::from(1);
        }
        Err(error) => {
            eprintln!("error[E-MOD-018]: output parent must already exist: {error}");
            return ExitCode::from(1);
        }
    };
    let output = output_parent.join(output_name);
    match fs::symlink_metadata(&output) {
        Ok(_) => {
            eprintln!(
                "error[E-MOD-018]: output `{}` already exists; no files were changed",
                output.display()
            );
            return ExitCode::from(1);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            eprintln!("error[E-MOD-018]: cannot inspect output directory: {error}");
            return ExitCode::from(1);
        }
    }
    if output.starts_with(project_root) {
        eprintln!("error[E-MOD-018]: choose an output directory outside the source project");
        return ExitCode::from(1);
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let mut staging = None;
    for attempt in 0..8 {
        let candidate = output_parent.join(format!(
            ".{}.zelyra-bundle-{}-{timestamp}-{attempt}",
            output_name.to_string_lossy(),
            std::process::id()
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => {
                staging = Some(candidate);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                eprintln!("error[E-MOD-018]: cannot create staging directory: {error}");
                return ExitCode::from(1);
            }
        }
    }
    let Some(staging) = staging else {
        eprintln!("error[E-MOD-018]: cannot allocate a unique staging directory");
        return ExitCode::from(1);
    };

    let build_result = (|| -> Result<ModuleBundleFiles, String> {
        let mut copied = Vec::new();
        let mut docker_files = Vec::new();
        if full_project {
            copy_bundle_source(project_root, &staging, &entry_relative, "main.zyl")?;
            copied.push("main.zyl".to_owned());
        }
        for relative in &source_files {
            if full_project && relative == &entry_relative {
                continue;
            }
            copy_bundle_source(project_root, &staging, relative, relative)?;
            copied.push(relative.clone());
        }
        if !full_project {
            let imports = source_files
                .iter()
                .enumerate()
                .map(|(index, path)| {
                    let escaped = path.replace('\\', "\\\\").replace('"', "\\\"");
                    format!("import \"{escaped}\" as bundle_{index}")
                })
                .collect::<Vec<_>>();
            fs::write(
                staging.join("main.zyl"),
                format!("{}\n", imports.join("\n")),
            )
            .map_err(|error| format!("cannot write generated entry: {error}"))?;
            copied.push("main.zyl".to_owned());
        }

        let mut support_files = Vec::new();
        for relative in ["zelyra.toml", PROJECT_THEME_CSS_FILE] {
            if project_root.join(relative).exists() {
                copy_bundle_source(project_root, &staging, relative, relative)?;
                support_files.push(relative.to_owned());
            }
        }
        let locales = project_root.join("locales");
        match fs::symlink_metadata(&locales) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err("refusing to copy a symlinked locales directory".into());
            }
            Ok(metadata) if metadata.is_dir() => {
                let mut locale_files = fs::read_dir(&locales)
                    .map_err(|error| format!("cannot read locales directory: {error}"))?
                    .map(|item| {
                        let item =
                            item.map_err(|error| format!("cannot read locale entry: {error}"))?;
                        item.file_name()
                            .into_string()
                            .map_err(|_| "locale filenames must be valid UTF-8".to_owned())
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                locale_files.sort_unstable();
                for name in locale_files {
                    if !name.ends_with(".json") {
                        continue;
                    }
                    copy_bundle_source(
                        project_root,
                        &staging,
                        &format!("locales/{name}"),
                        &format!("locales/{name}"),
                    )?;
                    support_files.push(format!("locales/{name}"));
                }
            }
            Ok(_) => return Err("locales path is not a directory".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("cannot inspect locales directory: {error}")),
        }

        let database = plan.get("database").cloned().unwrap_or(Value::Null);
        let database_is_required = database["required"].as_bool().unwrap_or(false);
        let configured_backends = database["configurations"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let needs_mariadb_client = database_is_required
            && (configured_backends.is_empty()
                || configured_backends.iter().any(|configuration| {
                    configuration["engine"]
                        .as_str()
                        .is_some_and(|engine| engine.eq_ignore_ascii_case("mariadb"))
                }));
        let runtime_packages = if needs_mariadb_client {
            "ca-certificates mariadb-client"
        } else {
            "ca-certificates"
        };
        let runtime_package_manifest = if needs_mariadb_client {
            vec!["ca-certificates", "mariadb-client"]
        } else {
            vec!["ca-certificates"]
        };
        let connection_environment = configured_backends
            .first()
            .and_then(|configuration| configuration["connection_environment"].as_str())
            .unwrap_or("DATABASE_URL");
        let external_service_contracts = plan
            .get("external_service_contracts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let smtp_environment_example = if external_service_contracts
            .iter()
            .any(|contract| contract["kind"] == "smtp")
        {
            "\n# Configure SMTP only when password-reset email delivery is enabled.\nZELYRA_SMTP_HOST=\nZELYRA_SMTP_PORT=587\nZELYRA_SMTP_SECURITY=starttls\nZELYRA_SMTP_FROM=\nZELYRA_SMTP_USERNAME=\nZELYRA_SMTP_PASSWORD=\n"
        } else {
            ""
        };
        if docker {
            let compiler_ref = compiler_ref.as_deref().expect("validated compiler ref");
            let dockerfile = format!(
                r#"FROM rust:1-bookworm AS build
ARG ZELYRA_REF={compiler_ref}
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/*
RUN git init /zelyra \
    && git -C /zelyra remote add origin https://github.com/sf1976/zelyra.git \
    && git -C /zelyra fetch --depth=1 origin "$ZELYRA_REF" \
    && git -C /zelyra checkout --detach FETCH_HEAD
RUN cargo install --locked --path /zelyra/cli --root /out

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends {runtime_packages} \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /out/bin/zelyra /usr/local/bin/zelyra
WORKDIR /app
COPY . ./
EXPOSE 8080
CMD ["zelyra", "serve", "main.zyl", "0.0.0.0:8080"]
"#
            );
            let compose = r#"services:
  app:
    build:
      context: .
    restart: unless-stopped
    env_file:
      - .env
    extra_hosts:
      - "host.docker.internal:host-gateway"
    ports:
      - "127.0.0.1:${ZELYRA_HOST_PORT:-18080}:8080"
"#;
            let env_example = format!(
                r#"# Copy this file to .env and configure values for this deployment.
# Never commit .env or put production credentials in this example.
ZELYRA_HOST_PORT=18080
# Required only when the selected application accesses a database.
# Set {connection_environment} to your MariaDB connection; never commit its real password.
# {connection_environment}=mariadb://USER:PASSWORD@host.docker.internal:3306/DATABASE
{connection_environment}=
ZELYRA_DB_CONNECT_TIMEOUT_SECS=10
ZELYRA_DB_QUERY_TIMEOUT_SECS=30
ZELYRA_DB_POOL_MAX_SIZE=8
ZELYRA_DB_POOL_WAIT_TIMEOUT_SECS=10
# Remote database connections use verified TLS automatically.
ZELYRA_DB_TLS_MODE=auto
# ZELYRA_DB_TLS_CA_CERT_FILE=/absolute/path/to/your/database-ca.pem
{smtp_environment_example}
"#
            );
            let dockerignore = ".git\n.env\n.env.*\ntarget/\nbuild/\ndist/\n*.log\n*.sqlite*\n*.db\n*.pem\n*.key\n*.p12\n*.pfx\n";
            for (name, contents) in [
                ("Dockerfile", dockerfile),
                ("docker-compose.yml", compose.to_owned()),
                (".env.example", env_example.to_owned()),
                (".dockerignore", dockerignore.to_owned()),
            ] {
                fs::write(staging.join(name), contents)
                    .map_err(|error| format!("cannot write {name}: {error}"))?;
                docker_files.push(name.to_owned());
            }
        }

        let manifest = json!({
            "format_version": 1,
            "kind": if docker { "experimental-docker-source-package" } else { "experimental-source-bundle" },
            "selected": selected,
            "selected_module": selected_module,
            "selected_resource": plan.get("selected_resource"),
            "source_files": copied,
            "support_files": support_files,
            "database": database,
            "external_service_contracts": external_service_contracts,
            "source_closure_complete": false,
            "complete_deployment": false,
            "docker": if docker {
                json!({
                    "compiler_repository": "https://github.com/sf1976/zelyra",
                    "compiler_commit": compiler_ref,
                    "files": docker_files,
                    "runtime_packages": runtime_package_manifest,
                    "database_connection": format!("external, configured per exported Compose project via {}", database["connection_environment"].as_str().unwrap_or("DATABASE_URL")),
                    "database_connection_scope": "per_exported_compose_project",
                    "supports_multiple_connections_per_process": false
                })
            } else {
                Value::Null
            },
            "limitations": [
                "The static dependency graph is incomplete; this bundle is not a deployment manifest.",
                if docker { "The Docker package builds the compiler from the pinned source commit; runtime deployment completeness is not yet proven." } else { "No Dockerfile, Compose stack, runtime binary, database service, or .env file is included." },
                "Database credentials and other runtime secrets are intentionally not copied.",
                "The selected source file is included in full; resource selection does not remove co-located declarations."
            ]
        });
        let manifest_text = serde_json::to_string_pretty(&manifest)
            .map_err(|error| format!("cannot serialize bundle manifest: {error}"))?;
        fs::write(
            staging.join("zelyra.bundle.json"),
            format!("{manifest_text}\n"),
        )
        .map_err(|error| format!("cannot write bundle manifest: {error}"))?;
        let readme = if docker {
            format!("# Experimental Zelyra Docker package\n\nThis package contains the selected known source closure and a Docker Compose app service. It is experimental, not a verified complete deployment; inspect `zelyra.bundle.json` (`complete_deployment: false`). The Dockerfile builds Zelyra from the exact compiler commit recorded in that manifest.\n\nCopy `.env.example` to `.env`, set a private `{connection_environment}` if needed, then run `docker compose up --build`. The MariaDB service is external and is not created by this package. The default `host.docker.internal` address is for a database on the Docker host; adjust it for your network. Remote database connections use verified TLS by default. For a private CA, mount its file into the app container and set `ZELYRA_DB_TLS_CA_CERT_FILE` to that in-container path. Disable TLS only for an isolated local network. Never commit `.env`.\n")
        } else {
            "# Experimental Zelyra source bundle\n\nThis directory contains the selected project module and source files in the dependency preview. It is not a Docker export or a complete deployment. Review `zelyra.bundle.json`; its `complete_deployment` value is `false`.\n\nRun `zelyra check main.zyl` with a compiler build that supports project imports. Configure any required external database and runtime settings separately. No `.env` file or credentials were copied.\n".to_owned()
        };
        fs::write(staging.join("README.md"), readme)
            .map_err(|error| format!("cannot write bundle README: {error}"))?;

        let check = Command::new(&executable)
            .current_dir(&staging)
            .args(["check", "main.zyl", "--format=json"])
            .output()
            .map_err(|error| format!("cannot validate generated bundle: {error}"))?;
        if !check.status.success() {
            return Err(format!(
                "generated bundle failed `zelyra check`; staged files were discarded:\n{}",
                String::from_utf8_lossy(&check.stdout)
            ));
        }
        Ok((copied, support_files, docker_files))
    })();

    match build_result {
        Ok((source_files, support_files, docker_files)) => {
            if dry_run {
                let mut files = source_files
                    .iter()
                    .chain(support_files.iter())
                    .chain(docker_files.iter())
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>();
                files.insert("README.md");
                files.insert("zelyra.bundle.json");
                let planned_files = files
                    .into_iter()
                    .map(|relative_path| {
                        json!({
                            "relative_path": relative_path,
                            "destination": output.join(relative_path).to_string_lossy()
                        })
                    })
                    .collect::<Vec<_>>();
                let document = json!({
                    "schema_version": "1",
                    "command": "module bundle",
                    "success": true,
                    "diagnostics": [],
                    "plan": {
                        "kind": "experimental-bundle-write-plan",
                        "selected": selected,
                        "selected_module": selected_module,
                        "output_directory": output,
                        "docker": docker,
                        "compiler_commit": compiler_ref,
                        "files": planned_files,
                        "secrets_included": false,
                        "source_closure_complete": false,
                        "complete_deployment": false,
                        "writes_performed": false
                    }
                });
                if let Err(error) = fs::remove_dir_all(&staging) {
                    eprintln!("error[E-MOD-018]: cannot remove temporary plan files: {error}");
                    return ExitCode::from(1);
                }
                println!("{}", serde_json::to_string_pretty(&document).unwrap());
                return ExitCode::SUCCESS;
            }
            if fs::symlink_metadata(&output).is_ok() {
                let _ = fs::remove_dir_all(&staging);
                eprintln!(
                    "error[E-MOD-018]: output `{}` appeared during bundle creation; it was not changed",
                    output.display()
                );
                return ExitCode::from(1);
            }
            if let Err(error) = publish_directory_no_replace(&staging, &output) {
                let _ = fs::remove_dir_all(&staging);
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    eprintln!(
                        "error[E-MOD-018]: output `{}` appeared during bundle creation; it was not changed",
                        output.display()
                    );
                } else {
                    eprintln!("error[E-MOD-018]: cannot publish source bundle: {error}");
                }
                return ExitCode::from(1);
            }
            println!(
                "created experimental {} bundle: {}",
                if docker { "Docker source" } else { "source" },
                output.display()
            );
            println!(
                "{} Zelyra source files; {} support files; {} Docker files",
                source_files.len(),
                support_files.len(),
                docker_files.len()
            );
            println!("the generated bundle passed `zelyra check`");
            println!("the static dependency graph is incomplete; complete_deployment remains false in zelyra.bundle.json");
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            eprintln!("error[E-MOD-018]: {error}");
            ExitCode::from(1)
        }
    }
}

fn copy_bundle_source(
    source_root: &std::path::Path,
    bundle_root: &std::path::Path,
    source_relative: &str,
    bundle_relative: &str,
) -> Result<(), String> {
    let source = source_root.join(source_relative);
    let metadata = fs::symlink_metadata(&source)
        .map_err(|error| format!("cannot inspect `{source_relative}`: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "refusing to copy non-regular source `{source_relative}`"
        ));
    }
    let destination = bundle_root.join(bundle_relative);
    let parent = destination
        .parent()
        .ok_or_else(|| format!("bundle path `{bundle_relative}` has no parent"))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create bundle directory: {error}"))?;
    fs::copy(&source, &destination)
        .map_err(|error| format!("cannot copy `{source_relative}`: {error}"))?;
    Ok(())
}

fn is_full_git_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn context_command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let Some(path) = arguments.next() else {
        usage();
        return ExitCode::from(2);
    };
    let mut format = OutputFormat::Human;
    while let Some(argument) = arguments.next() {
        if argument == "--format=json" {
            format = OutputFormat::Json;
        } else if argument == "--format=human" {
            format = OutputFormat::Human;
        } else if argument.starts_with("--format=") {
            eprintln!("error[E-CLI-001]: format must be `human` or `json`");
            return ExitCode::from(2);
        } else if argument == "--format" {
            format = match arguments.next().as_deref().and_then(parse_output_format) {
                Some(format) => format,
                None => {
                    eprintln!("error[E-CLI-001]: format must be `human` or `json`");
                    return ExitCode::from(2);
                }
            };
        } else {
            eprintln!("error[E-CLI-001]: unknown context option `{argument}`");
            return ExitCode::from(2);
        }
    }
    if format == OutputFormat::Human {
        return if validate(&path).is_ok() {
            println!("context: {}", context_entry(&path));
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }
    let source = fs::read_to_string(&path).unwrap_or_default();
    begin_json_diagnostics(&path, &source);
    let program = validate(&path);
    let diagnostics = finish_json_diagnostics();
    let success = program.is_ok();
    let declarations = program.as_ref().map_or_else(
        |_| empty_context_declarations(),
        |program| context_declarations(program, &source),
    );
    let project_modules = context_modules();
    let modules = context_modules_json(&project_modules);
    let database = program.as_ref().map_or_else(
        |_| empty_database_context(),
        |program| database_context(program, &project_modules),
    );
    let fields = [
        (
            "project".into(),
            json!({
                "name": project_name(&path),
                "entry": context_entry(&path)
            }),
        ),
        ("declarations".into(), declarations),
        ("modules".into(), modules),
        ("database".into(), database),
    ];
    print_machine_document(&machine_document("context", success, diagnostics, fields));
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn empty_database_context() -> Value {
    json!({
        "required": false,
        "connection_model": "single-project-wide-connection",
        "supports_multiple_connections": false,
        "configuration": null,
        "consumers": [],
        "fallback_connection_environment": null,
        "credentials_included": false
    })
}

fn database_context(program: &zelyra_ast::Program, modules: &[project::ProjectModule]) -> Value {
    let owners = module_declaration_owners(program);
    let provider = program.databases.first().map(|database| {
        let declaration = format!("database:{}", database.name);
        json!({
            "declaration": declaration,
            "module": owners.get(&declaration),
            "name": database.name,
            "engine": database.engine,
            "database": database.database,
            "connection_environment": database_url_environment_name(&database.name),
            "fallback_connection_environment": "DATABASE_URL"
        })
    });
    let provider_module = program
        .databases
        .first()
        .and_then(|database| owners.get(&format!("database:{}", database.name)));
    let mut module_definitions = modules.to_vec();
    module_definitions.sort_by(|left, right| left.path.cmp(&right.path));
    let consumers = module_definitions
        .iter()
        .filter(|module| module_uses_database(program, &module.path, &owners))
        .map(|module| {
            let path = &module.path;
            let resolution = provider_module.map_or("legacy_project_environment", |provider| {
                if path == provider {
                    "provider_module"
                } else {
                    match module_import_distance(&module_definitions, path, provider) {
                        Some(1) => "direct_import",
                        Some(_) => "transitive_import",
                        None => "unresolved",
                    }
                }
            });
            json!({
                "module": path,
                "provider_module": provider_module,
                "connection_environment": program.databases.first()
                    .map(|database| database_url_environment_name(&database.name))
                    .unwrap_or_else(|| "DATABASE_URL".to_owned()),
                "fallback_connection_environment": program
                    .databases
                    .first()
                    .map(|_| "DATABASE_URL"),
                "resolution": resolution
            })
        })
        .collect::<Vec<_>>();

    json!({
        "required": !consumers.is_empty(),
        "connection_model": "single-project-wide-connection",
        "supports_multiple_connections": false,
        "configuration": provider,
        "consumers": consumers,
        "fallback_connection_environment": program
            .databases
            .first()
            .map(|_| "DATABASE_URL"),
        "credentials_included": false
    })
}

fn module_import_distance(
    modules: &[project::ProjectModule],
    start: &str,
    target: &str,
) -> Option<usize> {
    let mut pending = VecDeque::from([(start.to_owned(), 0_usize)]);
    let mut visited = HashSet::new();
    while let Some((path, distance)) = pending.pop_front() {
        if !visited.insert(path.clone()) {
            continue;
        }
        if path == target {
            return Some(distance);
        }
        if let Some(module) = modules.iter().find(|module| module.path == path) {
            pending.extend(
                module
                    .imports
                    .iter()
                    .map(|import| (import.path.clone(), distance + 1)),
            );
        }
    }
    None
}
