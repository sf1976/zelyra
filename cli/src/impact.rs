use std::collections::{HashMap, HashSet};

use crate::project::ProjectSource;
use serde_json::{json, Value};
use zelyra_ast::{Expr, ExprKind, Program, Span, Stmt, Type};

pub fn build_impact_with_sources(
    program: &Program,
    project_sources: &[ProjectSource],
    fallback_source: &str,
) -> Value {
    let sources = ImpactSources {
        project_sources,
        fallback_source,
    };
    let table_names = program
        .tables
        .iter()
        .map(|table| table.name.clone())
        .collect::<Vec<_>>();
    let sql = sql_entries(program, &table_names, &sources);
    let table_consumers = table_consumers(program, &table_names, &sql);
    let references = semantic_references(program, &table_names, &sql, &sources);
    let permissions = permissions(program);

    json!({
        "tables": program.tables.iter().map(|table| json!({
            "name": table.name,
            "columns": table.columns.iter().map(|column| column.name.clone()).collect::<Vec<_>>(),
            "consumers": table_consumers.get(&table.name).cloned().unwrap_or_default(),
            "span": span_value(table.span, &sources),
        })).collect::<Vec<_>>(),
        "sql": sql,
        "references": references,
        "forms": program.forms.iter().map(|form| json!({
            "name": form.name,
            "table": form.table,
            "fields": form.fields.iter().map(|field| field.name.clone()).collect::<Vec<_>>(),
            "actions": form.actions.iter().map(|action| action.name.clone()).collect::<Vec<_>>(),
            "span": span_value(form.span, &sources),
        })).collect::<Vec<_>>(),
        "crud": program.cruds.iter().map(|crud| json!({
            "name": crud.name,
            "table": crud.table,
            "layout": crud.layout,
            "permissions": crud_permissions(crud),
            "actions": crud.actions.iter().map(|action| action.name.clone()).collect::<Vec<_>>(),
            "span": span_value(crud.span, &sources),
        })).collect::<Vec<_>>(),
        "views": program.views.iter().map(|view| json!({
            "name": view.name,
            "kind": "named",
            "span": span_value(view.span, &sources),
        })).chain(program.components.iter().map(|component| json!({
            "name": component.name,
            "kind": "component",
            "props": component.props.iter().map(|prop| prop.name.clone()).collect::<Vec<_>>(),
            "span": span_value(component.span, &sources),
        }))).chain(program.pages.iter().map(|page| json!({
            "name": page.path,
            "kind": "page",
            "view": page.view,
            "permissions": page.permissions,
            "span": span_value(page.span, &sources),
        }))).collect::<Vec<_>>(),
        "apis": program.apis.iter().map(|api| json!({
            "method": api.method,
            "path": api.path,
            "handler": api.handler,
            "permissions": api.permissions,
            "span": span_value(api.span, &sources),
        })).collect::<Vec<_>>(),
        "permissions": permissions,
        "contracts": program.functions.iter()
            .filter(|function| !function.requires.is_empty() || !function.ensures.is_empty())
            .map(|function| json!({
                "function": function.name,
                "requires": function.requires.iter().map(|expression| span_value(expression.span, &sources)).collect::<Vec<_>>(),
                "ensures": function.ensures.iter().map(|expression| span_value(expression.span, &sources)).collect::<Vec<_>>(),
                "capabilities": function.capabilities,
                "span": span_value(function.span, &sources),
            })).collect::<Vec<_>>(),
        "emails": [],
        "jobs": [],
        "tests": [],
        "schema_changes": {
            "available": false,
            "reason": "impact is source-only and does not inspect a live database schema",
            "items": []
        }
    })
}

struct ImpactSources<'a> {
    project_sources: &'a [ProjectSource],
    fallback_source: &'a str,
}

impl ImpactSources<'_> {
    fn text(&self, span: Span) -> &str {
        self.project_sources
            .get(span.source_id as usize)
            .map_or(self.fallback_source, |source| source.text.as_str())
    }

    fn path(&self, span: Span) -> Option<&str> {
        self.project_sources
            .get(span.source_id as usize)
            .map(|source| source.path.as_str())
    }
}

pub fn focus_impact(impact: &Value, query: &str) -> Result<Value, String> {
    let Some((kind, name)) = query.split_once(':') else {
        return Err("impact symbol must use the form `<kind>:<name>`".into());
    };
    if kind.is_empty() || name.is_empty() {
        return Err("impact symbol must use a non-empty `<kind>:<name>` value".into());
    }

    let references = impact
        .get("references")
        .and_then(Value::as_array)
        .ok_or_else(|| "impact document has no semantic references".to_owned())?;
    let known_nodes = known_impact_nodes(impact, references);
    if !known_nodes.contains(query) {
        return Err(format!("no known impact node `{query}` exists"));
    }

    let focused_references = references
        .iter()
        .filter(|reference| {
            reference.get("from").and_then(Value::as_str) == Some(query)
                || reference.get("to").and_then(Value::as_str) == Some(query)
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut related = focused_references
        .iter()
        .flat_map(|reference| {
            [
                reference.get("from").and_then(Value::as_str),
                reference.get("to").and_then(Value::as_str),
            ]
        })
        .flatten()
        .filter(|node| *node != query)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    related.sort_unstable();
    related.dedup();

    Ok(json!({
        "focus": query,
        "references": focused_references,
        "related": related,
    }))
}

fn known_impact_nodes(impact: &Value, references: &[Value]) -> HashSet<String> {
    let mut nodes = HashSet::new();
    for table in impact
        .get("tables")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(name) = table.get("name").and_then(Value::as_str) {
            nodes.insert(format!("table:{name}"));
        }
    }
    for form in impact
        .get("forms")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(name) = form.get("name").and_then(Value::as_str) {
            nodes.insert(format!("form:{name}"));
        }
    }
    for crud in impact
        .get("crud")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(name) = crud.get("name").and_then(Value::as_str) {
            nodes.insert(format!("crud:{name}"));
        }
    }
    for view in impact
        .get("views")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let kind = view.get("kind").and_then(Value::as_str);
        let name = view.get("name").and_then(Value::as_str);
        if let (Some(kind), Some(name)) = (kind, name) {
            nodes.insert(format!("{kind}:{name}"));
        }
    }
    for api in impact
        .get("apis")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let (Some(method), Some(path)) = (
            api.get("method").and_then(Value::as_str),
            api.get("path").and_then(Value::as_str),
        ) {
            nodes.insert(format!("api:{method} {path}"));
        }
    }
    for reference in references {
        for key in ["from", "to"] {
            if let Some(node) = reference.get(key).and_then(Value::as_str) {
                nodes.insert(node.to_owned());
            }
        }
    }
    nodes
}

fn semantic_references(
    program: &Program,
    table_names: &[String],
    sql: &[Value],
    sources: &ImpactSources<'_>,
) -> Vec<Value> {
    let mut references = Vec::new();

    for form in &program.forms {
        if let Some(table) = &form.table {
            add_reference(
                &mut references,
                format!("form:{}", form.name),
                format!("table:{table}"),
                "table",
                span_value(form.span, sources),
            );
        }
    }
    for crud in &program.cruds {
        add_reference(
            &mut references,
            format!("crud:{}", crud.name),
            format!("table:{}", crud.table),
            "table",
            span_value(crud.span, sources),
        );
        if let Some(layout) = &crud.layout {
            add_reference(
                &mut references,
                format!("crud:{}", crud.name),
                format!("view:{layout}"),
                "view",
                span_value(crud.span, sources),
            );
        }
    }
    for auth in &program.auth {
        let auth_node = format!("auth:{}", auth.name);
        let auth_span = span_value(auth.span, sources);
        for table in [
            Some(auth.table.as_str()),
            auth.session_table.as_deref(),
            auth.permissions_table.as_deref(),
            auth.roles_table.as_deref(),
            auth.role_permissions_table.as_deref(),
            auth.audit_table.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            add_reference(
                &mut references,
                auth_node.clone(),
                format!("table:{table}"),
                "auth_table",
                auth_span.clone(),
            );
        }
    }
    for tableview in &program.tableviews {
        let (_, accesses) =
            zelyra_database::sql::analyze_table_access(&tableview.source, table_names);
        for access in accesses {
            add_reference_with_access(
                &mut references,
                format!("tableview:{}", tableview.name),
                format!("table:{}", access.table),
                "table",
                span_value(tableview.span, sources),
                access.mode.as_str(),
            );
        }
    }
    for page in &program.pages {
        for data in &page.data {
            let (_, accesses) =
                zelyra_database::sql::analyze_table_access(&data.query, table_names);
            for access in accesses {
                add_reference_with_access(
                    &mut references,
                    format!("page:{}", page.path),
                    format!("table:{}", access.table),
                    "page_data_sql",
                    span_value(data.span, sources),
                    access.mode.as_str(),
                );
            }
        }
    }

    if let Some(auth) = program.auth.first() {
        let auth_node = format!("auth:{}", auth.name);
        for page in &program.pages {
            if page.requires_auth || !page.permissions.is_empty() {
                add_reference(
                    &mut references,
                    format!("page:{}", page.path),
                    auth_node.clone(),
                    "authentication",
                    span_value(page.span, sources),
                );
            }
        }
        for api in &program.apis {
            if api.requires_auth || !api.permissions.is_empty() {
                add_reference(
                    &mut references,
                    format!("api:{} {}", api.method, api.path),
                    auth_node.clone(),
                    "authentication",
                    span_value(api.span, sources),
                );
            }
        }
        for form in &program.forms {
            if form
                .actions
                .iter()
                .any(|action| action.requires_auth || !action.permissions.is_empty())
            {
                add_reference(
                    &mut references,
                    format!("form:{}", form.name),
                    auth_node.clone(),
                    "authentication",
                    span_value(form.span, sources),
                );
            }
        }
        for crud in &program.cruds {
            if crud.requires_auth
                || !crud.permissions.is_empty()
                || !crud.create_permissions.is_empty()
                || !crud.edit_permissions.is_empty()
                || !crud.delete_permissions.is_empty()
                || crud
                    .actions
                    .iter()
                    .any(|action| action.requires_auth || !action.permissions.is_empty())
            {
                add_reference(
                    &mut references,
                    format!("crud:{}", crud.name),
                    auth_node.clone(),
                    "authentication",
                    span_value(crud.span, sources),
                );
            }
        }
        for tableview in &program.tableviews {
            if tableview.requires_auth || !tableview.permissions.is_empty() {
                add_reference(
                    &mut references,
                    format!("tableview:{}", tableview.name),
                    auth_node.clone(),
                    "authentication",
                    span_value(tableview.span, sources),
                );
            }
        }
    }
    for table in &program.tables {
        for column in &table.columns {
            let zelyra_ast::Type::Named(name) = &column.ty else {
                continue;
            };
            if let Some(target) = relation_table(name, table_names) {
                add_reference(
                    &mut references,
                    format!("table:{}", table.name),
                    format!("table:{target}"),
                    "relation",
                    span_value(column.span, sources),
                );
            }
        }
    }
    for entry in sql {
        let Some(owner) = entry.get("owner").and_then(Value::as_str) else {
            continue;
        };
        let Some(accesses) = entry.get("table_accesses").and_then(Value::as_array) else {
            continue;
        };
        let source_owner = match entry.get("kind").and_then(Value::as_str) {
            Some("function") => format!("function:{owner}"),
            Some("form_action" | "crud_action") => owner
                .split_once(" action:")
                .map_or_else(|| owner.to_owned(), |(resource, _)| resource.to_owned()),
            _ => owner.to_owned(),
        };
        let span = entry.get("span").cloned().unwrap_or(Value::Null);
        for access in accesses {
            let (Some(table), Some(mode)) = (
                access.get("table").and_then(Value::as_str),
                access.get("mode").and_then(Value::as_str),
            ) else {
                continue;
            };
            add_reference_with_access(
                &mut references,
                source_owner.clone(),
                format!("table:{table}"),
                "sql_table",
                span.clone(),
                mode,
            );
        }
    }

    let view_names = program
        .views
        .iter()
        .map(|view| view.name.as_str())
        .collect::<HashSet<_>>();
    for page in &program.pages {
        if let Some(view) = &page.view {
            if view_names.contains(view.as_str()) {
                add_reference(
                    &mut references,
                    format!("page:{}", page.path),
                    format!("view:{view}"),
                    "view",
                    span_value(page.span, sources),
                );
            }
        }
    }

    let component_names = program
        .components
        .iter()
        .map(|component| component.name.as_str())
        .collect::<HashSet<_>>();
    for view in &program.views {
        for component in referenced_components(&view.html, &component_names) {
            add_reference(
                &mut references,
                format!("view:{}", view.name),
                format!("component:{component}"),
                "component",
                span_value(view.span, sources),
            );
        }
    }
    for component in &program.components {
        for nested in referenced_components(&component.html, &component_names) {
            add_reference(
                &mut references,
                format!("component:{}", component.name),
                format!("component:{nested}"),
                "component",
                span_value(component.span, sources),
            );
        }
    }
    for page in &program.pages {
        for component in referenced_components(&page.html, &component_names) {
            add_reference(
                &mut references,
                format!("page:{}", page.path),
                format!("component:{component}"),
                "component",
                span_value(page.span, sources),
            );
        }
    }

    let function_names = program
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect::<HashSet<_>>();
    for function in &program.functions {
        for call in called_functions(&function.body.statements, &function_names) {
            add_reference(
                &mut references,
                format!("function:{}", function.name),
                format!("function:{call}"),
                "call",
                span_value(function.span, sources),
            );
        }
        for expression in function.requires.iter().chain(function.ensures.iter()) {
            for call in called_functions_in_expression(expression, &function_names) {
                add_reference(
                    &mut references,
                    format!("function:{}", function.name),
                    format!("function:{call}"),
                    "call",
                    span_value(function.span, sources),
                );
            }
        }
    }

    for form in &program.forms {
        for action in &form.actions {
            for call in called_functions(&action.statements, &function_names) {
                add_reference(
                    &mut references,
                    format!("form:{}", form.name),
                    format!("function:{call}"),
                    "call",
                    span_value(action.span, sources),
                );
            }
        }
    }
    for crud in &program.cruds {
        for action in &crud.actions {
            for call in called_functions(&action.statements, &function_names) {
                add_reference(
                    &mut references,
                    format!("crud:{}", crud.name),
                    format!("function:{call}"),
                    "call",
                    span_value(action.span, sources),
                );
            }
        }
    }

    let handler_names = function_names;
    for api in &program.apis {
        if let Some(handler) = &api.handler {
            if handler_names.contains(handler.as_str()) {
                add_reference(
                    &mut references,
                    format!("api:{} {}", api.method, api.path),
                    format!("function:{handler}"),
                    "handler",
                    span_value(api.span, sources),
                );
            }
        }
    }

    let type_names = program
        .types
        .iter()
        .map(|definition| definition.name.as_str())
        .collect::<HashSet<_>>();
    let record_names = program
        .records
        .iter()
        .map(|record| record.name.as_str())
        .collect::<HashSet<_>>();
    for table in &program.tables {
        let owner = format!("table:{}", table.name);
        for column in &table.columns {
            add_type_references(
                &mut references,
                owner.clone(),
                &column.ty,
                &type_names,
                &record_names,
                column.span,
                sources,
            );
        }
    }
    for definition in &program.types {
        add_type_references(
            &mut references,
            format!("type:{}", definition.name),
            &definition.target,
            &type_names,
            &record_names,
            definition.span,
            sources,
        );
    }
    for record in &program.records {
        let owner = format!("record:{}", record.name);
        for field in &record.fields {
            add_type_references(
                &mut references,
                owner.clone(),
                &field.ty,
                &type_names,
                &record_names,
                field.span,
                sources,
            );
        }
    }
    for form in &program.forms {
        let owner = format!("form:{}", form.name);
        for field in form
            .fields
            .iter()
            .chain(form.actions.iter().flat_map(|action| action.fields.iter()))
        {
            if let Some(ty) = &field.ty {
                add_type_references(
                    &mut references,
                    owner.clone(),
                    ty,
                    &type_names,
                    &record_names,
                    field.span,
                    sources,
                );
            }
        }
        for action in &form.actions {
            add_statement_type_references(
                &mut references,
                &owner,
                &action.statements,
                &type_names,
                &record_names,
                sources,
            );
        }
    }
    for crud in &program.cruds {
        let owner = format!("crud:{}", crud.name);
        for field in crud.actions.iter().flat_map(|action| action.fields.iter()) {
            if let Some(ty) = &field.ty {
                add_type_references(
                    &mut references,
                    owner.clone(),
                    ty,
                    &type_names,
                    &record_names,
                    field.span,
                    sources,
                );
            }
        }
        for action in &crud.actions {
            add_statement_type_references(
                &mut references,
                &owner,
                &action.statements,
                &type_names,
                &record_names,
                sources,
            );
        }
    }
    for function in &program.functions {
        let owner = format!("function:{}", function.name);
        for parameter in &function.params {
            add_type_references(
                &mut references,
                owner.clone(),
                &parameter.ty,
                &type_names,
                &record_names,
                parameter.span,
                sources,
            );
        }
        if let Some(return_type) = &function.return_type {
            add_type_references(
                &mut references,
                owner.clone(),
                return_type,
                &type_names,
                &record_names,
                function.span,
                sources,
            );
        }
        add_statement_type_references(
            &mut references,
            &owner,
            &function.body.statements,
            &type_names,
            &record_names,
            sources,
        );
        for expression in function.requires.iter().chain(function.ensures.iter()) {
            add_expression_type_references(
                &mut references,
                &owner,
                expression,
                &type_names,
                &record_names,
                sources,
            );
        }
    }
    for page in &program.pages {
        let owner = format!("page:{}", page.path);
        for input in &page.inputs {
            add_type_references(
                &mut references,
                owner.clone(),
                &input.ty,
                &type_names,
                &record_names,
                input.span,
                sources,
            );
        }
        for data in &page.data {
            add_type_references(
                &mut references,
                owner.clone(),
                &data.result_type,
                &type_names,
                &record_names,
                data.span,
                sources,
            );
        }
    }
    for component in &program.components {
        let owner = format!("component:{}", component.name);
        for prop in &component.props {
            add_type_references(
                &mut references,
                owner.clone(),
                &prop.ty,
                &type_names,
                &record_names,
                prop.span,
                sources,
            );
        }
    }
    for tableview in &program.tableviews {
        add_type_references(
            &mut references,
            format!("tableview:{}", tableview.name),
            &tableview.result_type,
            &type_names,
            &record_names,
            tableview.span,
            sources,
        );
    }
    for api in &program.apis {
        let owner = format!("api:{} {}", api.method, api.path);
        for input in &api.input {
            add_type_references(
                &mut references,
                owner.clone(),
                &input.ty,
                &type_names,
                &record_names,
                input.span,
                sources,
            );
        }
        add_type_references(
            &mut references,
            owner.clone(),
            &api.output,
            &type_names,
            &record_names,
            api.span,
            sources,
        );
        for error in &api.errors {
            if let Some(payload) = &error.payload {
                add_type_references(
                    &mut references,
                    owner.clone(),
                    payload,
                    &type_names,
                    &record_names,
                    error.span,
                    sources,
                );
            }
        }
    }

    references.sort_by(|left, right| {
        let left_key = reference_sort_key(left);
        let right_key = reference_sort_key(right);
        left_key.cmp(&right_key)
    });
    references
}

fn add_type_references(
    references: &mut Vec<Value>,
    owner: String,
    ty: &Type,
    type_names: &HashSet<&str>,
    record_names: &HashSet<&str>,
    span: Span,
    sources: &ImpactSources<'_>,
) {
    let node = match ty {
        Type::Option(inner) | Type::Array(inner) | Type::HttpResult(inner) => {
            add_type_references(
                references,
                owner,
                inner,
                type_names,
                record_names,
                span,
                sources,
            );
            return;
        }
        Type::Result(ok, error) | Type::Map(ok, error) => {
            add_type_references(
                references,
                owner.clone(),
                ok,
                type_names,
                record_names,
                span,
                sources,
            );
            add_type_references(
                references,
                owner,
                error,
                type_names,
                record_names,
                span,
                sources,
            );
            return;
        }
        Type::Named(name) if record_names.contains(name.as_str()) => format!("record:{name}"),
        Type::Named(name) if type_names.contains(name.as_str()) => format!("type:{name}"),
        _ => return,
    };
    add_reference(references, owner, node, "type", span_value(span, sources));
}

fn add_statement_type_references(
    references: &mut Vec<Value>,
    owner: &str,
    statements: &[Stmt],
    type_names: &HashSet<&str>,
    record_names: &HashSet<&str>,
    sources: &ImpactSources<'_>,
) {
    for statement in statements {
        match statement {
            Stmt::Let {
                ty, value, span, ..
            } => {
                if let Some(ty) = ty {
                    add_type_references(
                        references,
                        owner.to_owned(),
                        ty,
                        type_names,
                        record_names,
                        *span,
                        sources,
                    );
                }
                add_expression_type_references(
                    references,
                    owner,
                    value,
                    type_names,
                    record_names,
                    sources,
                );
            }
            Stmt::BindOrAssign { value, .. } | Stmt::Expr(value) => {
                add_expression_type_references(
                    references,
                    owner,
                    value,
                    type_names,
                    record_names,
                    sources,
                );
            }
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    add_expression_type_references(
                        references,
                        owner,
                        value,
                        type_names,
                        record_names,
                        sources,
                    );
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                add_expression_type_references(
                    references,
                    owner,
                    condition,
                    type_names,
                    record_names,
                    sources,
                );
                add_statement_type_references(
                    references,
                    owner,
                    &then_block.statements,
                    type_names,
                    record_names,
                    sources,
                );
                if let Some(else_block) = else_block {
                    add_statement_type_references(
                        references,
                        owner,
                        &else_block.statements,
                        type_names,
                        record_names,
                        sources,
                    );
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                add_expression_type_references(
                    references,
                    owner,
                    condition,
                    type_names,
                    record_names,
                    sources,
                );
                for invariant in invariants {
                    add_expression_type_references(
                        references,
                        owner,
                        invariant,
                        type_names,
                        record_names,
                        sources,
                    );
                }
                add_statement_type_references(
                    references,
                    owner,
                    &body.statements,
                    type_names,
                    record_names,
                    sources,
                );
            }
            Stmt::For { iterable, body, .. } => {
                add_expression_type_references(
                    references,
                    owner,
                    iterable,
                    type_names,
                    record_names,
                    sources,
                );
                add_statement_type_references(
                    references,
                    owner,
                    &body.statements,
                    type_names,
                    record_names,
                    sources,
                );
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    add_expression_type_references(
                        references,
                        owner,
                        invariant,
                        type_names,
                        record_names,
                        sources,
                    );
                }
                add_statement_type_references(
                    references,
                    owner,
                    &body.statements,
                    type_names,
                    record_names,
                    sources,
                );
            }
            Stmt::Match { value, arms, .. } => {
                add_expression_type_references(
                    references,
                    owner,
                    value,
                    type_names,
                    record_names,
                    sources,
                );
                for arm in arms {
                    add_statement_type_references(
                        references,
                        owner,
                        &arm.body.statements,
                        type_names,
                        record_names,
                        sources,
                    );
                }
            }
            Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
                add_statement_type_references(
                    references,
                    owner,
                    &body.statements,
                    type_names,
                    record_names,
                    sources,
                );
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }
}

fn add_expression_type_references(
    references: &mut Vec<Value>,
    owner: &str,
    expression: &Expr,
    type_names: &HashSet<&str>,
    record_names: &HashSet<&str>,
    sources: &ImpactSources<'_>,
) {
    match &expression.kind {
        ExprKind::Array(values) => {
            for value in values {
                add_expression_type_references(
                    references,
                    owner,
                    value,
                    type_names,
                    record_names,
                    sources,
                );
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                add_expression_type_references(
                    references,
                    owner,
                    key,
                    type_names,
                    record_names,
                    sources,
                );
                add_expression_type_references(
                    references,
                    owner,
                    value,
                    type_names,
                    record_names,
                    sources,
                );
            }
        }
        ExprKind::Record { type_name, fields } => {
            let ty = Type::Named(type_name.clone());
            add_type_references(
                references,
                owner.to_owned(),
                &ty,
                type_names,
                record_names,
                expression.span,
                sources,
            );
            for (_, value) in fields {
                add_expression_type_references(
                    references,
                    owner,
                    value,
                    type_names,
                    record_names,
                    sources,
                );
            }
        }
        ExprKind::Index { target, index } => {
            add_expression_type_references(
                references,
                owner,
                target,
                type_names,
                record_names,
                sources,
            );
            add_expression_type_references(
                references,
                owner,
                index,
                type_names,
                record_names,
                sources,
            );
        }
        ExprKind::Field { target, .. } | ExprKind::Await(target) => {
            add_expression_type_references(
                references,
                owner,
                target,
                type_names,
                record_names,
                sources,
            );
        }
        ExprKind::Call {
            type_args, args, ..
        } => {
            for ty in type_args {
                add_type_references(
                    references,
                    owner.to_owned(),
                    ty,
                    type_names,
                    record_names,
                    expression.span,
                    sources,
                );
            }
            for argument in args {
                add_expression_type_references(
                    references,
                    owner,
                    argument,
                    type_names,
                    record_names,
                    sources,
                );
            }
        }
        ExprKind::Unary { expr, .. } => add_expression_type_references(
            references,
            owner,
            expr,
            type_names,
            record_names,
            sources,
        ),
        ExprKind::Binary { left, right, .. } => {
            add_expression_type_references(
                references,
                owner,
                left,
                type_names,
                record_names,
                sources,
            );
            add_expression_type_references(
                references,
                owner,
                right,
                type_names,
                record_names,
                sources,
            );
        }
        ExprKind::Sql { result_type, .. } => add_type_references(
            references,
            owner.to_owned(),
            result_type,
            type_names,
            record_names,
            expression.span,
            sources,
        ),
        ExprKind::Int(_)
        | ExprKind::UInt(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::String(_)
        | ExprKind::Char(_)
        | ExprKind::Variable(_) => {}
    }
}

fn add_reference(references: &mut Vec<Value>, from: String, to: String, kind: &str, span: Value) {
    if references.iter().any(|reference| {
        reference.get("from").and_then(Value::as_str) == Some(from.as_str())
            && reference.get("to").and_then(Value::as_str) == Some(to.as_str())
            && reference.get("kind").and_then(Value::as_str) == Some(kind)
    }) {
        return;
    }
    references.push(json!({
        "from": from,
        "to": to,
        "kind": kind,
        "span": span,
    }));
}

fn add_reference_with_access(
    references: &mut Vec<Value>,
    from: String,
    to: String,
    kind: &str,
    span: Value,
    access: &str,
) {
    if references.iter().any(|reference| {
        reference.get("from").and_then(Value::as_str) == Some(from.as_str())
            && reference.get("to").and_then(Value::as_str) == Some(to.as_str())
            && reference.get("kind").and_then(Value::as_str) == Some(kind)
            && reference.get("access").and_then(Value::as_str) == Some(access)
    }) {
        return;
    }
    references.push(json!({
        "from": from,
        "to": to,
        "kind": kind,
        "access": access,
        "span": span,
    }));
}

fn referenced_components(html: &str, component_names: &HashSet<&str>) -> Vec<String> {
    let bytes = html.as_bytes();
    let mut names = HashSet::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'<' {
            index += 1;
            continue;
        }
        let name_start = if bytes.get(index + 1) == Some(&b'/') {
            index + 2
        } else {
            index + 1
        };
        let mut name_end = name_start;
        while name_end < bytes.len()
            && (bytes[name_end].is_ascii_alphanumeric() || bytes[name_end] == b'_')
        {
            name_end += 1;
        }
        if name_end > name_start
            && bytes[name_start].is_ascii_uppercase()
            && component_names.contains(&html[name_start..name_end])
        {
            names.insert(html[name_start..name_end].to_owned());
        }
        index = name_end.max(index + 1);
    }
    let mut names = names.into_iter().collect::<Vec<_>>();
    names.sort_unstable();
    names
}

fn called_functions(statements: &[Stmt], function_names: &HashSet<&str>) -> Vec<String> {
    let mut calls = Vec::new();
    for statement in statements {
        collect_statement_calls(statement, function_names, &mut calls);
    }
    calls
}

fn collect_statement_calls(
    statement: &Stmt,
    function_names: &HashSet<&str>,
    calls: &mut Vec<String>,
) {
    match statement {
        Stmt::Let { value, .. } | Stmt::BindOrAssign { value, .. } | Stmt::Expr(value) => {
            collect_expression_calls(value, function_names, calls)
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                collect_expression_calls(value, function_names, calls);
            }
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            collect_expression_calls(condition, function_names, calls);
            for statement in &then_block.statements {
                collect_statement_calls(statement, function_names, calls);
            }
            if let Some(else_block) = else_block {
                for statement in &else_block.statements {
                    collect_statement_calls(statement, function_names, calls);
                }
            }
        }
        Stmt::While {
            condition,
            invariants,
            body,
            ..
        } => {
            collect_expression_calls(condition, function_names, calls);
            for invariant in invariants {
                collect_expression_calls(invariant, function_names, calls);
            }
            for statement in &body.statements {
                collect_statement_calls(statement, function_names, calls);
            }
        }
        Stmt::For { iterable, body, .. } => {
            collect_expression_calls(iterable, function_names, calls);
            for statement in &body.statements {
                collect_statement_calls(statement, function_names, calls);
            }
        }
        Stmt::Loop {
            invariants, body, ..
        } => {
            for invariant in invariants {
                collect_expression_calls(invariant, function_names, calls);
            }
            for statement in &body.statements {
                collect_statement_calls(statement, function_names, calls);
            }
        }
        Stmt::Match { value, arms, .. } => {
            collect_expression_calls(value, function_names, calls);
            for arm in arms {
                for statement in &arm.body.statements {
                    collect_statement_calls(statement, function_names, calls);
                }
            }
        }
        Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
            for statement in &body.statements {
                collect_statement_calls(statement, function_names, calls);
            }
        }
        Stmt::Break { .. } | Stmt::Continue { .. } => {}
    }
}

fn called_functions_in_expression(
    expression: &Expr,
    function_names: &HashSet<&str>,
) -> Vec<String> {
    let mut calls = Vec::new();
    collect_expression_calls(expression, function_names, &mut calls);
    calls
}

fn collect_expression_calls(
    expression: &Expr,
    function_names: &HashSet<&str>,
    calls: &mut Vec<String>,
) {
    match &expression.kind {
        ExprKind::Array(values) => {
            for value in values {
                collect_expression_calls(value, function_names, calls);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                collect_expression_calls(key, function_names, calls);
                collect_expression_calls(value, function_names, calls);
            }
        }
        ExprKind::Record { fields, .. } => {
            for (_, value) in fields {
                collect_expression_calls(value, function_names, calls);
            }
        }
        ExprKind::Index { target, index } => {
            collect_expression_calls(target, function_names, calls);
            collect_expression_calls(index, function_names, calls);
        }
        ExprKind::Field { target, .. } | ExprKind::Await(target) => {
            collect_expression_calls(target, function_names, calls);
        }
        ExprKind::Call { name, args, .. } => {
            if function_names.contains(name.as_str()) {
                calls.push(name.clone());
            }
            for argument in args {
                collect_expression_calls(argument, function_names, calls);
            }
        }
        ExprKind::Unary { expr, .. } => {
            collect_expression_calls(expr, function_names, calls);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_expression_calls(left, function_names, calls);
            collect_expression_calls(right, function_names, calls);
        }
        ExprKind::Int(_)
        | ExprKind::UInt(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::String(_)
        | ExprKind::Char(_)
        | ExprKind::Variable(_)
        | ExprKind::Sql { .. } => {}
    }
}

fn reference_sort_key(reference: &Value) -> (String, String, String, usize) {
    (
        reference
            .get("from")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        reference
            .get("to")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        reference
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        reference
            .get("span")
            .and_then(|span| span.get("start"))
            .and_then(|start| start.get("offset"))
            .and_then(Value::as_u64)
            .unwrap_or_default() as usize,
    )
}

fn span_value(span: Span, sources: &ImpactSources<'_>) -> Value {
    let (end_line, end_column) = source_position(sources.text(span), span.end);
    let mut value = json!({
        "start": { "offset": span.start, "line": span.line, "column": span.column },
        "end": { "offset": span.end, "line": end_line, "column": end_column }
    });
    if let Some(path) = sources.path(span) {
        value["file"] = json!(path);
    }
    value
}

fn source_position(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
    for character in source[..offset.min(source.len())].chars() {
        if character == '\n' {
            line += 1;
            column = 1;
        } else {
            column += character.len_utf8();
        }
    }
    (line, column)
}

fn table_consumers(
    program: &Program,
    table_names: &[String],
    sql: &[Value],
) -> HashMap<String, Vec<String>> {
    let mut consumers = table_names
        .iter()
        .map(|table| (table.clone(), Vec::new()))
        .collect::<HashMap<_, _>>();
    for form in &program.forms {
        if let Some(table) = &form.table {
            add_consumer(&mut consumers, table, format!("form:{}", form.name));
        }
    }
    for crud in &program.cruds {
        add_consumer(&mut consumers, &crud.table, format!("crud:{}", crud.name));
    }
    for tableview in &program.tableviews {
        let (_, accesses) =
            zelyra_database::sql::analyze_table_access(&tableview.source, table_names);
        for access in accesses {
            add_consumer(
                &mut consumers,
                &access.table,
                format!("tableview:{}", tableview.name),
            );
        }
    }
    for entry in sql {
        let Some(owner) = entry.get("owner").and_then(Value::as_str) else {
            continue;
        };
        let Some(tables) = entry.get("tables").and_then(Value::as_array) else {
            continue;
        };
        for table in tables.iter().filter_map(Value::as_str) {
            add_consumer(&mut consumers, table, owner.to_owned());
        }
    }
    consumers
        .values_mut()
        .for_each(|names| names.sort_unstable());
    consumers
}

fn add_consumer(consumers: &mut HashMap<String, Vec<String>>, table: &str, consumer: String) {
    if let Some(names) = consumers.get_mut(table) {
        if !names.contains(&consumer) {
            names.push(consumer);
        }
    }
}

fn sql_entries(
    program: &Program,
    table_names: &[String],
    sources: &ImpactSources<'_>,
) -> Vec<Value> {
    let mut entries = Vec::new();
    for function in &program.functions {
        collect_statement_sql(
            &function.body.statements,
            &function.name,
            "function",
            table_names,
            sources,
            &mut entries,
        );
    }
    for form in &program.forms {
        for action in &form.actions {
            collect_statement_sql(
                &action.statements,
                &format!("form:{} action:{}", form.name, action.name),
                "form_action",
                table_names,
                sources,
                &mut entries,
            );
        }
    }
    for crud in &program.cruds {
        for action in &crud.actions {
            collect_statement_sql(
                &action.statements,
                &format!("crud:{} action:{}", crud.name, action.name),
                "crud_action",
                table_names,
                sources,
                &mut entries,
            );
        }
    }
    for tableview in &program.tableviews {
        let (operation, tables, table_accesses) =
            analyzed_sql_accesses(&tableview.source, table_names);
        entries.push(json!({
            "owner": format!("tableview:{}", tableview.name),
            "kind": "tableview",
            "operation": operation,
            "tables": tables,
            "table_accesses": table_accesses,
            "span": span_value(tableview.span, sources),
        }));
    }
    entries
}

fn collect_statement_sql(
    statements: &[Stmt],
    owner: &str,
    kind: &str,
    table_names: &[String],
    sources: &ImpactSources<'_>,
    entries: &mut Vec<Value>,
) {
    for statement in statements {
        collect_statement_sql_inner(statement, owner, kind, table_names, sources, entries);
    }
}

fn collect_statement_sql_inner(
    statement: &Stmt,
    owner: &str,
    kind: &str,
    table_names: &[String],
    sources: &ImpactSources<'_>,
    entries: &mut Vec<Value>,
) {
    match statement {
        Stmt::Let { value, .. } | Stmt::BindOrAssign { value, .. } | Stmt::Expr(value) => {
            collect_expr_sql(value, owner, kind, table_names, sources, entries)
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                collect_expr_sql(value, owner, kind, table_names, sources, entries);
            }
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            collect_expr_sql(condition, owner, kind, table_names, sources, entries);
            collect_statement_sql(
                &then_block.statements,
                owner,
                kind,
                table_names,
                sources,
                entries,
            );
            if let Some(else_block) = else_block {
                collect_statement_sql(
                    &else_block.statements,
                    owner,
                    kind,
                    table_names,
                    sources,
                    entries,
                );
            }
        }
        Stmt::While {
            condition,
            invariants,
            body,
            ..
        } => {
            collect_expr_sql(condition, owner, kind, table_names, sources, entries);
            for invariant in invariants {
                collect_expr_sql(invariant, owner, kind, table_names, sources, entries);
            }
            collect_statement_sql(&body.statements, owner, kind, table_names, sources, entries);
        }
        Stmt::For { iterable, body, .. } => {
            collect_expr_sql(iterable, owner, kind, table_names, sources, entries);
            collect_statement_sql(&body.statements, owner, kind, table_names, sources, entries);
        }
        Stmt::Loop {
            invariants, body, ..
        } => {
            for invariant in invariants {
                collect_expr_sql(invariant, owner, kind, table_names, sources, entries);
            }
            collect_statement_sql(&body.statements, owner, kind, table_names, sources, entries);
        }
        Stmt::Match { value, arms, .. } => {
            collect_expr_sql(value, owner, kind, table_names, sources, entries);
            for arm in arms {
                collect_statement_sql(
                    &arm.body.statements,
                    owner,
                    kind,
                    table_names,
                    sources,
                    entries,
                );
            }
        }
        Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
            collect_statement_sql(&body.statements, owner, kind, table_names, sources, entries);
        }
        Stmt::Break { .. } | Stmt::Continue { .. } => {}
    }
}

fn collect_expr_sql(
    expression: &Expr,
    owner: &str,
    kind: &str,
    table_names: &[String],
    sources: &ImpactSources<'_>,
    entries: &mut Vec<Value>,
) {
    match &expression.kind {
        ExprKind::Sql { query, .. } => {
            let (operation, tables, table_accesses) = analyzed_sql_accesses(query, table_names);
            entries.push(json!({
                "owner": owner,
                "kind": kind,
                "operation": operation,
                "tables": tables,
                "table_accesses": table_accesses,
                "span": span_value(expression.span, sources),
            }));
        }
        ExprKind::Array(values) => values
            .iter()
            .for_each(|value| collect_expr_sql(value, owner, kind, table_names, sources, entries)),
        ExprKind::Map(map_entries) => map_entries.iter().for_each(|(key, value)| {
            collect_expr_sql(key, owner, kind, table_names, sources, entries);
            collect_expr_sql(value, owner, kind, table_names, sources, entries);
        }),
        ExprKind::Record { fields, .. } => fields.iter().for_each(|(_, value)| {
            collect_expr_sql(value, owner, kind, table_names, sources, entries)
        }),
        ExprKind::Index { target, index } => {
            collect_expr_sql(target, owner, kind, table_names, sources, entries);
            collect_expr_sql(index, owner, kind, table_names, sources, entries);
        }
        ExprKind::Field { target, .. } | ExprKind::Await(target) => {
            collect_expr_sql(target, owner, kind, table_names, sources, entries)
        }
        ExprKind::Call { args, .. } => args.iter().for_each(|argument| {
            collect_expr_sql(argument, owner, kind, table_names, sources, entries)
        }),
        ExprKind::Unary { expr, .. } => {
            collect_expr_sql(expr, owner, kind, table_names, sources, entries)
        }
        ExprKind::Binary { left, right, .. } => {
            collect_expr_sql(left, owner, kind, table_names, sources, entries);
            collect_expr_sql(right, owner, kind, table_names, sources, entries);
        }
        ExprKind::Int(_)
        | ExprKind::UInt(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::String(_)
        | ExprKind::Char(_)
        | ExprKind::Variable(_) => {}
    }
}

fn analyzed_sql_accesses(
    query: &str,
    table_names: &[String],
) -> (Option<&'static str>, Vec<String>, Vec<Value>) {
    let (operation, accesses) = zelyra_database::sql::analyze_table_access(query, table_names);
    let tables = accesses.iter().map(|access| access.table.clone()).collect();
    let accesses = accesses
        .into_iter()
        .map(|access| json!({ "table": access.table, "mode": access.mode.as_str() }))
        .collect();
    (operation, tables, accesses)
}

fn relation_table(name: &str, table_names: &[String]) -> Option<String> {
    let normalized = name.to_ascii_lowercase();
    if let Some(table) = table_names
        .iter()
        .find(|table| table.eq_ignore_ascii_case(&normalized))
    {
        return Some(table.clone());
    }
    let plural = if normalized.ends_with('y') {
        format!("{}ies", &normalized[..normalized.len() - 1])
    } else {
        format!("{normalized}s")
    };
    table_names
        .iter()
        .find(|table| table.eq_ignore_ascii_case(&plural))
        .cloned()
}

fn crud_permissions(crud: &zelyra_ast::CrudDef) -> Vec<String> {
    let mut permissions = Vec::new();
    for permission in crud
        .permissions
        .iter()
        .chain(crud.create_permissions.iter())
        .chain(crud.edit_permissions.iter())
        .chain(crud.delete_permissions.iter())
        .chain(
            crud.actions
                .iter()
                .flat_map(|action| action.permissions.iter()),
        )
    {
        if !permissions.contains(permission) {
            permissions.push(permission.clone());
        }
    }
    permissions
}

fn permissions(program: &Program) -> Vec<Value> {
    let mut owners = HashMap::<String, Vec<String>>::new();
    for page in &program.pages {
        for permission in &page.permissions {
            add_permission(&mut owners, permission, format!("page:{}", page.path));
        }
    }
    for api in &program.apis {
        for permission in &api.permissions {
            add_permission(
                &mut owners,
                permission,
                format!("api:{} {}", api.method, api.path),
            );
        }
    }
    for crud in &program.cruds {
        for permission in crud_permissions(crud) {
            add_permission(&mut owners, &permission, format!("crud:{}", crud.name));
        }
    }
    for form in &program.forms {
        for action in &form.actions {
            for permission in &action.permissions {
                add_permission(
                    &mut owners,
                    permission,
                    format!("form:{} action:{}", form.name, action.name),
                );
            }
        }
    }
    if let Some(auth) = program.auth.first() {
        if let Some(permission) = &auth.admin_permission {
            add_permission(&mut owners, permission, format!("auth:{}", auth.name));
        }
    }
    let mut permissions = owners.into_iter().collect::<Vec<_>>();
    permissions.sort_by(|left, right| left.0.cmp(&right.0));
    permissions
        .into_iter()
        .map(|(name, mut consumers)| {
            consumers.sort_unstable();
            json!({ "name": name, "consumers": consumers })
        })
        .collect()
}

fn add_permission(owners: &mut HashMap<String, Vec<String>>, permission: &str, owner: String) {
    let consumers = owners.entry(permission.to_owned()).or_default();
    if !consumers.contains(&owner) {
        consumers.push(owner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zelyra_lexer::lex;
    use zelyra_parser::parse;

    #[test]
    fn builds_deterministic_source_impact() {
        let source = r#"
            table customers { id: Id }
            table audit_events { id: Id message: String(100) }
            table users { id: Id email: Email password_hash: String(255) }
            auth users { table: users }
            component Badge { html { <strong>Ready</strong> } }
            view Shell { html { <Badge /><slot /> } }
            page "/customers" {
                requires auth
                view: Shell
                load customers = sql<Customer[]> { SELECT id FROM customers }
                html { <Badge /> }
            }
            table invoices { id: Id customer: Customer }
            form CustomerForm -> customers {
                fields { id }
                action save {
                    requires auth
                    permits "customers.write"
                    sql { INSERT INTO audit_events (message) VALUES ('saved') }
                }
            }
            crud Customer -> customers { requires auth }
            tableview CustomerList {
                source sql<Int[]> { SELECT id FROM customers }
                columns { id }
                requires auth
            }
            fn load() uses Database {
                return sql<Customer> { SELECT id FROM customers }
            }
            fn main() { load() }
            api GET "/customers" {
                handler load
                output Customer
                permits "customers.read"
            }
        "#;
        let program = parse(&lex(source).expect("source should lex")).expect("source should parse");
        let impact = build_impact_with_sources(&program, &[], source);
        assert_eq!(impact["tables"][0]["name"], "customers");
        assert_eq!(impact["sql"][0]["tables"][0], "customers");
        assert_eq!(impact["permissions"][0]["name"], "customers.read");
        assert_eq!(impact["schema_changes"]["available"], false);
        let references = impact["references"].as_array().unwrap();
        assert!(references.iter().any(|reference| {
            reference["from"] == "form:CustomerForm"
                && reference["to"] == "table:customers"
                && reference["kind"] == "table"
        }));
        assert!(references.iter().any(|reference| {
            reference["from"] == "form:CustomerForm"
                && reference["to"] == "table:audit_events"
                && reference["kind"] == "sql_table"
        }));
        assert!(references.iter().any(|reference| {
            reference["from"] == "form:CustomerForm"
                && reference["to"] == "auth:users"
                && reference["kind"] == "authentication"
        }));
        assert!(references.iter().any(|reference| {
            reference["from"] == "page:/customers"
                && reference["to"] == "auth:users"
                && reference["kind"] == "authentication"
        }));
        assert!(references.iter().any(|reference| {
            reference["from"] == "auth:users"
                && reference["to"] == "table:users"
                && reference["kind"] == "auth_table"
        }));
        assert!(references.iter().any(|reference| {
            reference["from"] == "api:GET /customers"
                && reference["to"] == "auth:users"
                && reference["kind"] == "authentication"
        }));
        assert!(references.iter().any(|reference| {
            reference["from"] == "crud:Customer"
                && reference["to"] == "auth:users"
                && reference["kind"] == "authentication"
        }));
        assert!(references.iter().any(|reference| {
            reference["from"] == "tableview:CustomerList"
                && reference["to"] == "auth:users"
                && reference["kind"] == "authentication"
        }));
        assert!(references.iter().any(|reference| {
            reference["from"] == "function:load"
                && reference["to"] == "table:customers"
                && reference["kind"] == "sql_table"
        }));
        assert!(references.iter().any(|reference| {
            reference["from"] == "page:/customers"
                && reference["to"] == "view:Shell"
                && reference["kind"] == "view"
        }));
        assert!(references.iter().any(|reference| {
            reference["from"] == "page:/customers"
                && reference["to"] == "table:customers"
                && reference["kind"] == "page_data_sql"
        }));
        assert!(references.iter().any(|reference| {
            reference["from"] == "table:invoices"
                && reference["to"] == "table:customers"
                && reference["kind"] == "relation"
        }));
        assert!(references.iter().any(|reference| {
            reference["from"] == "view:Shell"
                && reference["to"] == "component:Badge"
                && reference["kind"] == "component"
        }));
        assert!(references.iter().any(|reference| {
            reference["from"] == "api:GET /customers"
                && reference["to"] == "function:load"
                && reference["kind"] == "handler"
        }));
        assert!(references.iter().any(|reference| {
            reference["from"] == "function:main"
                && reference["to"] == "function:load"
                && reference["kind"] == "call"
        }));
        assert_eq!(impact, build_impact_with_sources(&program, &[], source));
    }

    #[test]
    fn focuses_impact_on_a_known_node_and_rejects_unknown_nodes() {
        let source = r#"
            table customers { id: Id }
            form CustomerForm -> customers { fields { id } }
            crud Customer -> customers
        "#;
        let program = parse(&lex(source).expect("source should lex")).expect("source should parse");
        let impact = build_impact_with_sources(&program, &[], source);
        let focused = focus_impact(&impact, "table:customers").expect("table should be known");
        assert_eq!(focused["focus"], "table:customers");
        assert!(focused["references"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reference| {
                reference["from"] == "form:CustomerForm" && reference["to"] == "table:customers"
            }));
        assert!(focused["related"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node == "crud:Customer"));
        assert_eq!(
            focus_impact(&impact, "table:missing").unwrap_err(),
            "no known impact node `table:missing` exists"
        );
    }
}
