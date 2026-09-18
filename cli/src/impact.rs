use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};
use zelyra_ast::{Expr, ExprKind, Program, Span, Stmt};

pub fn build_impact(program: &Program, source: &str) -> Value {
    let table_names = program
        .tables
        .iter()
        .map(|table| table.name.clone())
        .collect::<Vec<_>>();
    let sql = sql_entries(program, &table_names, source);
    let table_consumers = table_consumers(program, &table_names, &sql);
    let references = semantic_references(program, &table_names, &sql, source);
    let permissions = permissions(program);

    json!({
        "tables": program.tables.iter().map(|table| json!({
            "name": table.name,
            "columns": table.columns.iter().map(|column| column.name.clone()).collect::<Vec<_>>(),
            "consumers": table_consumers.get(&table.name).cloned().unwrap_or_default(),
            "span": span_value(table.span, source),
        })).collect::<Vec<_>>(),
        "sql": sql,
        "references": references,
        "forms": program.forms.iter().map(|form| json!({
            "name": form.name,
            "table": form.table,
            "fields": form.fields.iter().map(|field| field.name.clone()).collect::<Vec<_>>(),
            "actions": form.actions.iter().map(|action| action.name.clone()).collect::<Vec<_>>(),
            "span": span_value(form.span, source),
        })).collect::<Vec<_>>(),
        "crud": program.cruds.iter().map(|crud| json!({
            "name": crud.name,
            "table": crud.table,
            "permissions": crud_permissions(crud),
            "actions": crud.actions.iter().map(|action| action.name.clone()).collect::<Vec<_>>(),
            "span": span_value(crud.span, source),
        })).collect::<Vec<_>>(),
        "views": program.views.iter().map(|view| json!({
            "name": view.name,
            "kind": "named",
            "span": span_value(view.span, source),
        })).chain(program.components.iter().map(|component| json!({
            "name": component.name,
            "kind": "component",
            "props": component.props.iter().map(|prop| prop.name.clone()).collect::<Vec<_>>(),
            "span": span_value(component.span, source),
        }))).chain(program.pages.iter().map(|page| json!({
            "name": page.path,
            "kind": "page",
            "view": page.view,
            "permissions": page.permissions,
            "span": span_value(page.span, source),
        }))).collect::<Vec<_>>(),
        "apis": program.apis.iter().map(|api| json!({
            "method": api.method,
            "path": api.path,
            "handler": api.handler,
            "permissions": api.permissions,
            "span": span_value(api.span, source),
        })).collect::<Vec<_>>(),
        "permissions": permissions,
        "contracts": program.functions.iter()
            .filter(|function| !function.requires.is_empty() || !function.ensures.is_empty())
            .map(|function| json!({
                "function": function.name,
                "requires": function.requires.iter().map(|expression| span_value(expression.span, source)).collect::<Vec<_>>(),
                "ensures": function.ensures.iter().map(|expression| span_value(expression.span, source)).collect::<Vec<_>>(),
                "capabilities": function.capabilities,
                "span": span_value(function.span, source),
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

fn semantic_references(
    program: &Program,
    table_names: &[String],
    sql: &[Value],
    source: &str,
) -> Vec<Value> {
    let mut references = Vec::new();

    for form in &program.forms {
        if let Some(table) = &form.table {
            add_reference(
                &mut references,
                format!("form:{}", form.name),
                format!("table:{table}"),
                "table",
                span_value(form.span, source),
            );
        }
    }
    for crud in &program.cruds {
        add_reference(
            &mut references,
            format!("crud:{}", crud.name),
            format!("table:{}", crud.table),
            "table",
            span_value(crud.span, source),
        );
    }
    for tableview in &program.tableviews {
        for table in referenced_tables(&tableview.source, table_names) {
            add_reference(
                &mut references,
                format!("tableview:{}", tableview.name),
                format!("table:{table}"),
                "table",
                span_value(tableview.span, source),
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
        let source_owner = if entry.get("kind").and_then(Value::as_str) == Some("function") {
            format!("function:{owner}")
        } else {
            owner.to_owned()
        };
        let span = entry.get("span").cloned().unwrap_or(Value::Null);
        for table in tables.iter().filter_map(Value::as_str) {
            add_reference(
                &mut references,
                source_owner.clone(),
                format!("table:{table}"),
                "sql_table",
                span.clone(),
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
                    span_value(page.span, source),
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
                span_value(view.span, source),
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
                span_value(component.span, source),
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
                span_value(page.span, source),
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
                span_value(function.span, source),
            );
        }
        for expression in function.requires.iter().chain(function.ensures.iter()) {
            for call in called_functions_in_expression(expression, &function_names) {
                add_reference(
                    &mut references,
                    format!("function:{}", function.name),
                    format!("function:{call}"),
                    "call",
                    span_value(function.span, source),
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
                    span_value(api.span, source),
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

fn span_value(span: Span, source: &str) -> Value {
    let (end_line, end_column) = source_position(source, span.end);
    json!({
        "start": { "offset": span.start, "line": span.line, "column": span.column },
        "end": { "offset": span.end, "line": end_line, "column": end_column }
    })
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
        for table in referenced_tables(&tableview.source, table_names) {
            add_consumer(
                &mut consumers,
                &table,
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

fn sql_entries(program: &Program, table_names: &[String], source: &str) -> Vec<Value> {
    let mut entries = Vec::new();
    for function in &program.functions {
        collect_statement_sql(
            &function.body.statements,
            &function.name,
            "function",
            table_names,
            source,
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
                source,
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
                source,
                &mut entries,
            );
        }
    }
    for tableview in &program.tableviews {
        entries.push(json!({
            "owner": format!("tableview:{}", tableview.name),
            "kind": "tableview",
            "tables": referenced_tables(&tableview.source, table_names),
            "span": span_value(tableview.span, source),
        }));
    }
    entries
}

fn collect_statement_sql(
    statements: &[Stmt],
    owner: &str,
    kind: &str,
    table_names: &[String],
    source: &str,
    entries: &mut Vec<Value>,
) {
    for statement in statements {
        collect_statement_sql_inner(statement, owner, kind, table_names, source, entries);
    }
}

fn collect_statement_sql_inner(
    statement: &Stmt,
    owner: &str,
    kind: &str,
    table_names: &[String],
    source: &str,
    entries: &mut Vec<Value>,
) {
    match statement {
        Stmt::Let { value, .. } | Stmt::BindOrAssign { value, .. } | Stmt::Expr(value) => {
            collect_expr_sql(value, owner, kind, table_names, source, entries)
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                collect_expr_sql(value, owner, kind, table_names, source, entries);
            }
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            collect_expr_sql(condition, owner, kind, table_names, source, entries);
            collect_statement_sql(
                &then_block.statements,
                owner,
                kind,
                table_names,
                source,
                entries,
            );
            if let Some(else_block) = else_block {
                collect_statement_sql(
                    &else_block.statements,
                    owner,
                    kind,
                    table_names,
                    source,
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
            collect_expr_sql(condition, owner, kind, table_names, source, entries);
            for invariant in invariants {
                collect_expr_sql(invariant, owner, kind, table_names, source, entries);
            }
            collect_statement_sql(&body.statements, owner, kind, table_names, source, entries);
        }
        Stmt::For { iterable, body, .. } => {
            collect_expr_sql(iterable, owner, kind, table_names, source, entries);
            collect_statement_sql(&body.statements, owner, kind, table_names, source, entries);
        }
        Stmt::Loop {
            invariants, body, ..
        } => {
            for invariant in invariants {
                collect_expr_sql(invariant, owner, kind, table_names, source, entries);
            }
            collect_statement_sql(&body.statements, owner, kind, table_names, source, entries);
        }
        Stmt::Match { value, arms, .. } => {
            collect_expr_sql(value, owner, kind, table_names, source, entries);
            for arm in arms {
                collect_statement_sql(
                    &arm.body.statements,
                    owner,
                    kind,
                    table_names,
                    source,
                    entries,
                );
            }
        }
        Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
            collect_statement_sql(&body.statements, owner, kind, table_names, source, entries);
        }
        Stmt::Break { .. } | Stmt::Continue { .. } => {}
    }
}

fn collect_expr_sql(
    expression: &Expr,
    owner: &str,
    kind: &str,
    table_names: &[String],
    source: &str,
    entries: &mut Vec<Value>,
) {
    match &expression.kind {
        ExprKind::Sql { query, .. } => entries.push(json!({
            "owner": owner,
            "kind": kind,
            "tables": referenced_tables(query, table_names),
            "span": span_value(expression.span, source),
        })),
        ExprKind::Array(values) => values
            .iter()
            .for_each(|value| collect_expr_sql(value, owner, kind, table_names, source, entries)),
        ExprKind::Record { fields, .. } => fields.iter().for_each(|(_, value)| {
            collect_expr_sql(value, owner, kind, table_names, source, entries)
        }),
        ExprKind::Index { target, index } => {
            collect_expr_sql(target, owner, kind, table_names, source, entries);
            collect_expr_sql(index, owner, kind, table_names, source, entries);
        }
        ExprKind::Field { target, .. } | ExprKind::Await(target) => {
            collect_expr_sql(target, owner, kind, table_names, source, entries)
        }
        ExprKind::Call { args, .. } => args.iter().for_each(|argument| {
            collect_expr_sql(argument, owner, kind, table_names, source, entries)
        }),
        ExprKind::Unary { expr, .. } => {
            collect_expr_sql(expr, owner, kind, table_names, source, entries)
        }
        ExprKind::Binary { left, right, .. } => {
            collect_expr_sql(left, owner, kind, table_names, source, entries);
            collect_expr_sql(right, owner, kind, table_names, source, entries);
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

fn referenced_tables(query: &str, table_names: &[String]) -> Vec<String> {
    table_names
        .iter()
        .filter(|table| contains_identifier(query, table))
        .cloned()
        .collect()
}

fn contains_identifier(source: &str, needle: &str) -> bool {
    let needle = needle.to_ascii_lowercase();
    source
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .any(|word| word.eq_ignore_ascii_case(&needle))
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
            component Badge { html { <strong>Ready</strong> } }
            view Shell { html { <Badge /><slot /> } }
            page "/customers" {
                view: Shell
                html { <Badge /> }
            }
            form CustomerForm -> customers { fields { id } }
            crud Customer -> customers
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
        let impact = build_impact(&program, source);
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
        assert_eq!(impact, build_impact(&program, source));
    }
}
