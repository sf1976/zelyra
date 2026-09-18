use std::collections::HashMap;

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
    let permissions = permissions(program);

    json!({
        "tables": program.tables.iter().map(|table| json!({
            "name": table.name,
            "columns": table.columns.iter().map(|column| column.name.clone()).collect::<Vec<_>>(),
            "consumers": table_consumers.get(&table.name).cloned().unwrap_or_default(),
            "span": span_value(table.span, source),
        })).collect::<Vec<_>>(),
        "sql": sql,
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
            fn load() uses Database {
                return sql<Customer> { SELECT id FROM customers }
            }
            api GET "/customers" { output String permits "customers.read" }
        "#;
        let program = parse(&lex(source).expect("source should lex")).expect("source should parse");
        let impact = build_impact(&program, source);
        assert_eq!(impact["tables"][0]["name"], "customers");
        assert_eq!(impact["sql"][0]["tables"][0], "customers");
        assert_eq!(impact["permissions"][0]["name"], "customers.read");
        assert_eq!(impact["schema_changes"]["available"], false);
    }
}
