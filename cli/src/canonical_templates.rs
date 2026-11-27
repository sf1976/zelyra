use std::collections::HashSet;
use zelyra_lexer::{lex, TokenKind};
use zelyra_parser::parse;

pub(crate) struct CanonicalTemplateModules {
    pub(crate) schema: String,
    pub(crate) routes: String,
    pub(crate) domain_actions: Option<String>,
}

fn template_camel_to_snake(name: &str) -> String {
    let mut output = String::with_capacity(name.len() + 4);
    for (index, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index > 0 {
                output.push('_');
            }
            output.push(ch.to_ascii_lowercase());
        } else {
            output.push(ch);
        }
    }
    output
}

fn format_template_type(ty: &zelyra_ast::Type) -> String {
    match ty {
        zelyra_ast::Type::Option(inner) => format!("{}?", format_template_type(inner)),
        other => other.to_string(),
    }
}

fn qualify_schema_types(
    module_source: &mut String,
    schema_type_names: &HashSet<String>,
) -> Result<(), String> {
    if schema_type_names.is_empty() {
        return Ok(());
    }
    let tokens = lex(module_source).map_err(|error| error.message)?;
    let mut edits = tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| {
            let TokenKind::Ident(name) = &token.kind else {
                return None;
            };
            if !schema_type_names.contains(name)
                || tokens
                    .get(index + 1)
                    .is_some_and(|next| next.kind == TokenKind::DoubleColon)
            {
                return None;
            }
            Some((token.span.start, token.span.end, format!("schema::{name}")))
        })
        .collect::<Vec<_>>();
    edits.sort_by_key(|edit| std::cmp::Reverse(edit.0));
    for (start, end, replacement) in edits {
        if start > end
            || end > module_source.len()
            || !module_source.is_char_boundary(start)
            || !module_source.is_char_boundary(end)
        {
            return Err("template parser returned an invalid type reference span".to_owned());
        }
        module_source.replace_range(start..end, &replacement);
    }
    Ok(())
}

/// Split a bundled database template into canonical schema, domain, and route
/// modules using parser source spans. Direct-SQL write actions inside forms
/// and CRUD declarations are lifted into `domain/actions.zyl` and called from
/// `routes/web.zyl` via typed `execute { ... }` blocks so every canonical
/// starter satisfies `strict_architecture = true` out of the box.
pub(crate) fn canonical_template_modules(source: &str) -> Result<CanonicalTemplateModules, String> {
    let tokens = lex(source).map_err(|error| error.message)?;
    let program = parse(&tokens).map_err(|error| error.message)?;
    let needs_database = program.databases.is_empty();
    let known_tables = program
        .tables
        .iter()
        .map(|table| table.name.clone())
        .collect::<Vec<_>>();
    let schema_type_names = program
        .types
        .iter()
        .map(|definition| definition.name.clone())
        .chain(
            program
                .records
                .iter()
                .map(|definition| definition.name.clone()),
        )
        .chain(
            program
                .enums
                .iter()
                .map(|definition| definition.name.clone()),
        )
        .collect::<HashSet<_>>();
    let mut schema_spans = Vec::new();
    let mut route_spans = Vec::new();
    let mut table_starts = HashSet::new();
    let public_schema_starts = program
        .types
        .iter()
        .filter(|definition| !definition.is_public)
        .map(|definition| definition.span.start)
        .chain(
            program
                .records
                .iter()
                .filter(|definition| !definition.is_public)
                .map(|definition| definition.span.start),
        )
        .chain(
            program
                .enums
                .iter()
                .filter(|definition| !definition.is_public)
                .map(|definition| definition.span.start),
        )
        .collect::<HashSet<_>>();
    macro_rules! collect {
        ($items:expr, $schema:expr) => {
            for item in $items {
                if $schema {
                    schema_spans.push(item.span);
                } else {
                    route_spans.push(item.span);
                }
            }
        };
    }
    collect!(program.databases, true);
    for table in &program.tables {
        if table.access.span.is_some() {
            return Err(format!(
                "table `{}` already declares module access; canonical generation cannot safely broaden it",
                table.name
            ));
        }
        table_starts.insert(table.span.start);
        schema_spans.push(table.span);
    }
    collect!(program.types, true);
    collect!(program.records, true);
    collect!(program.enums, true);
    collect!(program.views, false);
    collect!(program.components, false);
    collect!(program.pages, false);
    collect!(program.tableviews, false);
    collect!(&program.forms, false);
    collect!(&program.cruds, false);
    collect!(program.auth, false);
    collect!(program.apis, false);
    collect!(program.commands, false);
    collect!(program.schedules, false);
    collect!(program.tests, false);
    collect!(program.functions, false);

    // Lift direct-SQL write actions from `form` and `crud` declarations into
    // `domain/actions.zyl` and rewrite the route action body to `execute { ... }`.
    let mut domain_functions = Vec::new();
    let mut route_rewrites: Vec<(usize, usize, String)> = Vec::new();
    let action_has_write_sql = |action: &zelyra_ast::FormAction| -> bool {
        if action.execute_body.is_some() || action.statements.is_empty() {
            return false;
        }
        action.statements.iter().any(|statement| {
            let zelyra_ast::Stmt::Expr(expression) = statement else {
                return false;
            };
            let zelyra_ast::ExprKind::Sql { query, .. } = &expression.kind else {
                return false;
            };
            let (command, _) = zelyra_database::sql::analyze_table_access(query, &known_tables);
            command.is_some() && command != Some("SELECT")
        })
    };
    let resolve_field_type =
        |table: Option<&zelyra_ast::TableDef>, field: &zelyra_ast::FormField| -> zelyra_ast::Type {
            field
                .ty
                .clone()
                .or_else(|| {
                    table.and_then(|table| {
                        table
                            .columns
                            .iter()
                            .find(|column| column.name == field.name)
                            .map(|column| column.ty.clone())
                    })
                })
                .unwrap_or(zelyra_ast::Type::String)
        };

    for form in &program.forms {
        let table = form
            .table
            .as_deref()
            .and_then(|name| program.tables.iter().find(|table| table.name == name));
        for action in &form.actions {
            if !action_has_write_sql(action) {
                continue;
            }
            let Some((first_start, last_end)) = action
                .statements
                .first()
                .zip(action.statements.last())
                .map(|(first, last)| (first.span().start, last.span().end))
            else {
                continue;
            };
            if first_start > last_end
                || last_end > source.len()
                || !source.is_char_boundary(first_start)
                || !source.is_char_boundary(last_end)
            {
                return Err("template parser returned an invalid form action span".to_owned());
            }
            let fn_name = format!(
                "{}_{}",
                template_camel_to_snake(&form.name),
                template_camel_to_snake(&action.name)
            );
            let mut param_defs = Vec::new();
            let mut call_args = Vec::new();
            let mut seen = HashSet::new();
            for field in form.fields.iter().chain(action.fields.iter()) {
                if !seen.insert(field.name.clone()) {
                    continue;
                }
                let ty = format_template_type(&resolve_field_type(table, field));
                param_defs.push(format!("{}: {ty}", field.name));
                call_args.push(field.name.clone());
            }
            let raw_body = source[first_start..last_end].trim();
            domain_functions.push(format!(
                "pub fn {fn_name}({}) uses Database(write) {{\n    {raw_body}\n}}\n",
                param_defs.join(", ")
            ));
            route_rewrites.push((
                first_start,
                last_end,
                format!(
                    "uses Database(write)\n\n        execute {{\n            actions::{fn_name}({})\n        }}",
                    call_args.join(", ")
                ),
            ));
        }
    }

    for crud in &program.cruds {
        let table = program.tables.iter().find(|table| table.name == crud.table);
        let id_type = table
            .and_then(|table| table.columns.iter().find(|column| column.name == "id"))
            .map(|column| format_template_type(&column.ty))
            .unwrap_or_else(|| "Id".into());
        for action in &crud.actions {
            if !action_has_write_sql(action) {
                continue;
            }
            let Some((first_start, last_end)) = action
                .statements
                .first()
                .zip(action.statements.last())
                .map(|(first, last)| (first.span().start, last.span().end))
            else {
                continue;
            };
            if first_start > last_end
                || last_end > source.len()
                || !source.is_char_boundary(first_start)
                || !source.is_char_boundary(last_end)
            {
                return Err("template parser returned an invalid CRUD action span".to_owned());
            }
            let fn_name = format!(
                "{}_{}",
                template_camel_to_snake(&crud.name),
                template_camel_to_snake(&action.name)
            );
            let mut param_defs = vec![format!("id: {id_type}")];
            let mut call_args = vec!["row.id".to_owned()];
            let mut seen = HashSet::from(["id".to_owned()]);
            for field in &action.fields {
                if !seen.insert(field.name.clone()) {
                    continue;
                }
                let ty = format_template_type(&resolve_field_type(table, field));
                param_defs.push(format!("{}: {ty}", field.name));
                call_args.push(field.name.clone());
            }
            let raw_body = source[first_start..last_end].trim();
            domain_functions.push(format!(
                "pub fn {fn_name}({}) uses Database(write) {{\n    {raw_body}\n}}\n",
                param_defs.join(", ")
            ));
            route_rewrites.push((
                first_start,
                last_end,
                format!(
                    "uses Database(write)\n\n        execute {{\n            actions::{fn_name}({})\n        }}",
                    call_args.join(", ")
                ),
            ));
        }
    }

    let has_domain_actions = !domain_functions.is_empty();

    fn emit(
        source: &str,
        spans: &mut [zelyra_ast::Span],
        table_starts: &HashSet<usize>,
        public_starts: &HashSet<usize>,
        has_domain_actions: bool,
        rewrites: &[(usize, usize, String)],
    ) -> Result<String, String> {
        spans.sort_by_key(|span| span.start);
        let mut output = String::new();
        for span in spans {
            let start = span.start;
            let end = span.end;
            if start > end
                || end > source.len()
                || !source.is_char_boundary(start)
                || !source.is_char_boundary(end)
            {
                return Err("template parser returned an invalid source span".to_owned());
            }
            let mut declaration = source[start..end].to_owned();
            let mut local_rewrites = rewrites
                .iter()
                .filter(|(r_start, r_end, _)| *r_start >= start && *r_end <= end)
                .map(|(r_start, r_end, text)| (r_start - start, r_end - start, text))
                .collect::<Vec<_>>();
            local_rewrites.sort_by_key(|(r_start, _, _)| std::cmp::Reverse(*r_start));
            for (r_start, r_end, text) in local_rewrites {
                declaration.replace_range(r_start..r_end, text);
            }
            let mut declaration = declaration.trim().to_owned();
            if public_starts.contains(&span.start) {
                declaration.insert_str(0, "pub ");
            }
            if table_starts.contains(&span.start) {
                let closing = declaration
                    .rfind('}')
                    .ok_or_else(|| "table declaration has no closing brace".to_owned())?;
                let access_modules = if has_domain_actions {
                    "\"domain/actions.zyl\", \"routes/web.zyl\""
                } else {
                    "\"routes/web.zyl\""
                };
                declaration.insert_str(
                    closing,
                    &format!("\n    access {{\n        read_write: [{access_modules}]\n    }}\n"),
                );
            }
            if !declaration.is_empty() {
                output.push_str(&declaration);
                output.push_str("\n\n");
            }
        }
        Ok(output)
    }

    let mut schema = emit(
        source,
        &mut schema_spans,
        &table_starts,
        &public_schema_starts,
        has_domain_actions,
        &[],
    )?;
    if needs_database {
        schema.insert_str(0, "database main {\n    engine: mariadb\n}\n\n");
    }
    if schema.is_empty() {
        schema.push_str("// No table declarations in this starter yet.\n");
    }
    let mut routes = String::from("import \"schema::tables\" as schema\n");
    if has_domain_actions {
        routes.push_str("import \"domain::actions\" as actions\n");
    }
    for import in program.imports {
        let start = import.span.start;
        let end = import.span.end;
        if start > end
            || end > source.len()
            || !source.is_char_boundary(start)
            || !source.is_char_boundary(end)
        {
            return Err("template parser returned an invalid import span".to_owned());
        }
        routes.push_str(source[start..end].trim());
        routes.push('\n');
    }
    if !routes.is_empty() {
        routes.push('\n');
    }
    let route_declarations = emit(
        source,
        &mut route_spans,
        &table_starts,
        &public_schema_starts,
        has_domain_actions,
        &route_rewrites,
    )?;
    if route_declarations.trim().is_empty() {
        return Err("template contains no route or application declarations".to_owned());
    }
    routes.push('\n');
    routes.push_str(&route_declarations);
    qualify_schema_types(&mut routes, &schema_type_names)?;

    let domain_actions = if has_domain_actions {
        let mut actions_source = String::from("import \"schema::tables\" as schema\n\n");
        actions_source.push_str(&domain_functions.join("\n"));
        qualify_schema_types(&mut actions_source, &schema_type_names)?;
        Some(actions_source)
    } else {
        None
    };

    Ok(CanonicalTemplateModules {
        schema,
        routes,
        domain_actions,
    })
}
