use std::collections::{HashMap, HashSet};
use zelyra_ast::{Block, Expr, ExprKind, Span, Stmt, Type};

use super::{table_for_type, Schema};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for SqlError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SqlToken {
    Word(String),
    Parameter(String),
    String,
    Number,
    Star,
    Comma,
    Dot,
    OpenParen,
    CloseParen,
    Operator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SqlAccessMode {
    Read,
    Write,
    ReadWrite,
    Unknown,
}

impl SqlAccessMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::ReadWrite => "read_write",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlTableAccess {
    pub table: String,
    pub mode: SqlAccessMode,
}

/// Re-check a compiled SQL expression against the project schema and its
/// runtime parameter types. Alternate execution backends use this boundary
/// when accepting decoded bytecode artifacts.
pub fn check_sql_query(
    query: &str,
    result_type: &Type,
    schema: &Schema,
    parameters: &HashMap<String, Type>,
    span: Span,
) -> Result<(), Vec<SqlError>> {
    let errors = check_query(query, result_type, schema, parameters, span);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Reports conservative table access modes for the SQL statement forms
/// accepted by the current checker. Complex joined UPDATE/DELETE statements
/// are marked unknown because their write targets are dialect-sensitive.
pub fn analyze_table_access(
    query: &str,
    known_tables: &[String],
) -> (Option<&'static str>, Vec<SqlTableAccess>) {
    let tokens = tokenize(query);
    let Some(SqlToken::Word(command)) = tokens.first() else {
        return (None, Vec::new());
    };
    let command = command.to_ascii_uppercase();
    if !matches!(command.as_str(), "SELECT" | "INSERT" | "UPDATE" | "DELETE") {
        return (None, Vec::new());
    }

    let known = known_tables
        .iter()
        .map(|name| (name.to_ascii_lowercase(), name.as_str()))
        .collect::<HashMap<_, _>>();
    let mut modes = HashMap::<String, SqlAccessMode>::new();
    let has_join = tokens
        .iter()
        .any(|token| matches!(token, SqlToken::Word(word) if word.eq_ignore_ascii_case("JOIN")));
    let add_at = |index: usize, mode: SqlAccessMode, modes: &mut HashMap<String, SqlAccessMode>| {
        let candidate = match (
            tokens.get(index + 1),
            tokens.get(index + 2),
            tokens.get(index + 3),
        ) {
            (Some(SqlToken::Word(_schema)), Some(SqlToken::Dot), Some(SqlToken::Word(name))) => {
                name
            }
            (Some(SqlToken::Word(name)), _, _) => name,
            (Some(SqlToken::String), _, _) => {
                modes
                    .entry(String::new())
                    .and_modify(|existing| {
                        *existing = merge_access(*existing, SqlAccessMode::Unknown)
                    })
                    .or_insert(SqlAccessMode::Unknown);
                return;
            }
            _ => return,
        };
        let normalized = candidate.to_ascii_lowercase();
        let Some(canonical) = known.get(&normalized) else {
            return;
        };
        modes
            .entry((*canonical).to_owned())
            .and_modify(|existing| *existing = merge_access(*existing, mode))
            .or_insert(mode);
    };

    match command.as_str() {
        "SELECT" => {
            for (index, token) in tokens
                .iter()
                .enumerate()
                .take(tokens.len().saturating_sub(1))
            {
                if matches!(token, SqlToken::Word(word) if word.eq_ignore_ascii_case("FROM") || word.eq_ignore_ascii_case("JOIN"))
                {
                    add_at(index, SqlAccessMode::Read, &mut modes);
                }
            }
        }
        "INSERT" => {
            for (index, token) in tokens
                .iter()
                .enumerate()
                .take(tokens.len().saturating_sub(1))
            {
                if let SqlToken::Word(word) = token {
                    if word.eq_ignore_ascii_case("INTO") {
                        add_at(index, SqlAccessMode::Write, &mut modes);
                    } else if word.eq_ignore_ascii_case("FROM") || word.eq_ignore_ascii_case("JOIN")
                    {
                        add_at(index, SqlAccessMode::Read, &mut modes);
                    }
                }
            }
        }
        "UPDATE" => {
            add_at(
                0,
                if has_join {
                    SqlAccessMode::Unknown
                } else {
                    SqlAccessMode::Write
                },
                &mut modes,
            );
            for (index, token) in tokens
                .iter()
                .enumerate()
                .take(tokens.len().saturating_sub(1))
                .skip(1)
            {
                if matches!(token, SqlToken::Word(word) if word.eq_ignore_ascii_case("FROM") || word.eq_ignore_ascii_case("JOIN"))
                {
                    add_at(
                        index,
                        if has_join {
                            SqlAccessMode::Unknown
                        } else {
                            SqlAccessMode::Read
                        },
                        &mut modes,
                    );
                }
            }
        }
        "DELETE" => {
            let mut first_from = true;
            for (index, token) in tokens
                .iter()
                .enumerate()
                .take(tokens.len().saturating_sub(1))
                .skip(1)
            {
                let SqlToken::Word(word) = token else {
                    continue;
                };
                if word.eq_ignore_ascii_case("FROM") {
                    let mode = if has_join {
                        SqlAccessMode::Unknown
                    } else if first_from {
                        SqlAccessMode::Write
                    } else {
                        SqlAccessMode::Read
                    };
                    add_at(index, mode, &mut modes);
                    first_from = false;
                } else if word.eq_ignore_ascii_case("JOIN") {
                    add_at(index, SqlAccessMode::Unknown, &mut modes);
                }
            }
        }
        _ => unreachable!("SQL command was checked above"),
    }

    let mut accesses = modes
        .into_iter()
        .map(|(table, mode)| SqlTableAccess { table, mode })
        .collect::<Vec<_>>();
    accesses.sort_by(|left, right| left.table.cmp(&right.table));
    (
        Some(match command.as_str() {
            "SELECT" => "SELECT",
            "INSERT" => "INSERT",
            "UPDATE" => "UPDATE",
            "DELETE" => "DELETE",
            _ => unreachable!(),
        }),
        accesses,
    )
}

fn merge_access(existing: SqlAccessMode, incoming: SqlAccessMode) -> SqlAccessMode {
    match (existing, incoming) {
        (SqlAccessMode::Unknown, _) | (_, SqlAccessMode::Unknown) => SqlAccessMode::Unknown,
        (SqlAccessMode::Read, SqlAccessMode::Write)
        | (SqlAccessMode::Write, SqlAccessMode::Read)
        | (SqlAccessMode::ReadWrite, _)
        | (_, SqlAccessMode::ReadWrite) => SqlAccessMode::ReadWrite,
        (mode, _) => mode,
    }
}

pub fn check_program(program: &zelyra_ast::Program, schema: &Schema) -> Result<(), Vec<SqlError>> {
    let mut errors = Vec::new();
    for function in &program.functions {
        let mut environment = function
            .params
            .iter()
            .map(|parameter| (parameter.name.clone(), parameter.ty.clone()))
            .collect::<HashMap<_, _>>();
        check_block(
            &function.body,
            schema,
            &mut environment,
            &program.types,
            &mut errors,
        );
    }
    for form in &program.forms {
        let source_table = form.table.as_ref().and_then(|table_name| {
            program
                .tables
                .iter()
                .find(|table| table.name == *table_name)
        });
        for action in &form.actions {
            let mut environment = HashMap::new();
            if let Some(route) = form.path.as_deref() {
                for name in route.split('/').filter_map(|segment| {
                    segment
                        .strip_prefix('{')
                        .and_then(|name| name.strip_suffix('}'))
                }) {
                    environment.insert(name.to_owned(), Type::String);
                }
            }
            for field in &form.fields {
                let field_type = field.ty.clone().or_else(|| {
                    source_table.and_then(|table| {
                        table
                            .columns
                            .iter()
                            .find(|column| column.name == field.name)
                            .map(|column| column.ty.clone())
                    })
                });
                environment.insert(
                    field.name.clone(),
                    form_parameter_type(
                        &field_type.unwrap_or(Type::Unknown),
                        &program.types,
                        &mut HashSet::new(),
                    ),
                );
            }
            if action.requires_auth {
                environment.insert("zelyra_user_id".into(), Type::Int);
            }
            if program.auth.iter().any(|a| a.membership_table.is_some()) {
                environment.insert("zelyra_tenant_id".into(), Type::Int);
            }
            let block = Block {
                statements: action.statements.clone(),
                span: action.span,
            };
            check_block(
                &block,
                schema,
                &mut environment,
                &program.types,
                &mut errors,
            );
        }
    }
    for crud in &program.cruds {
        let source_table = program.tables.iter().find(|table| table.name == crud.table);
        for action in &crud.actions {
            let mut environment = HashMap::new();
            if let Some(id_column) = source_table
                .and_then(|table| table.columns.iter().find(|column| column.name == "id"))
            {
                environment.insert("id".into(), id_column.ty.clone());
            }
            if crud.requires_auth || action.requires_auth {
                environment.insert("zelyra_user_id".into(), Type::Int);
            }
            if crud.tenant_column.is_some()
                || program.auth.iter().any(|a| a.membership_table.is_some())
            {
                environment.insert("zelyra_tenant_id".into(), Type::Int);
            }
            for field in &action.fields {
                let field_type = field.ty.clone().or_else(|| {
                    source_table.and_then(|table| {
                        table
                            .columns
                            .iter()
                            .find(|column| column.name == field.name)
                            .map(|column| column.ty.clone())
                    })
                });
                environment.insert(field.name.clone(), field_type.unwrap_or(Type::Unknown));
            }
            let block = Block {
                statements: action.statements.clone(),
                span: action.span,
            };
            check_block(
                &block,
                schema,
                &mut environment,
                &program.types,
                &mut errors,
            );
        }
    }
    for page in &program.pages {
        let mut environment = page
            .path
            .split('/')
            .filter_map(|segment| {
                segment
                    .strip_prefix('{')
                    .and_then(|segment| segment.strip_suffix('}'))
                    .map(|name| (name.to_owned(), Type::String))
            })
            .collect::<HashMap<_, _>>();
        for input in &page.inputs {
            environment.insert(input.name.clone(), input.ty.clone());
        }
        for data in &page.data {
            if let Some(record_name) = result_record_name(&data.result_type) {
                if let Some(record) = program
                    .records
                    .iter()
                    .find(|record| record.name == record_name)
                {
                    errors.extend(check_tableview_query(
                        &data.query,
                        &data.result_type,
                        schema,
                        &environment,
                        &program.records,
                        TableViewProjection {
                            columns: &record
                                .fields
                                .iter()
                                .map(|field| field.name.clone())
                                .collect::<Vec<_>>(),
                            row_fields: &[],
                        },
                        data.span,
                    ));
                    continue;
                }
            }
            errors.extend(check_query(
                &data.query,
                &data.result_type,
                schema,
                &environment,
                data.span,
            ));
        }
    }
    for tableview in &program.tableviews {
        errors.extend(check_tableview_query(
            &tableview.source,
            &tableview.result_type,
            schema,
            &HashMap::new(),
            &program.records,
            TableViewProjection {
                columns: &tableview.columns,
                row_fields: &tableview.view.row_fields,
            },
            tableview.span,
        ));
    }
    check_computed_column_sql_writes(program, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn check_computed_column_sql_writes(program: &zelyra_ast::Program, errors: &mut Vec<SqlError>) {
    let known_tables = program
        .tables
        .iter()
        .map(|table| table.name.clone())
        .collect::<Vec<_>>();
    let computed_mappings = zelyra_ast::build_computed_column_functions(program).1;
    let dependencies = program
        .tables
        .iter()
        .filter_map(|table| {
            let mut affected = table
                .columns
                .iter()
                .filter(|column| column.computed.is_some())
                .map(|column| column.name.to_ascii_lowercase())
                .collect::<HashSet<_>>();
            for mapping in computed_mappings.iter().filter(|mapping| {
                program
                    .tables
                    .get(mapping.table_index)
                    .is_some_and(|item| item.name == table.name)
            }) {
                affected.extend(
                    mapping
                        .input_columns
                        .iter()
                        .map(|name| name.to_ascii_lowercase()),
                );
            }
            (!affected.is_empty()).then(|| (table.name.clone(), affected))
        })
        .collect::<HashMap<_, _>>();
    let computed_fields = program
        .tables
        .iter()
        .filter_map(|table| {
            let fields = table
                .columns
                .iter()
                .filter(|column| column.computed.is_some())
                .map(|column| column.name.to_ascii_lowercase())
                .collect::<HashSet<_>>();
            (!fields.is_empty()).then(|| (table.name.clone(), fields))
        })
        .collect::<HashMap<_, _>>();

    fn check_query_writes(
        query: &str,
        span: Span,
        known_tables: &[String],
        dependencies: &HashMap<String, HashSet<String>>,
        computed_fields: &HashMap<String, HashSet<String>>,
        api_insert_table: Option<&str>,
        errors: &mut Vec<SqlError>,
    ) {
        let (command, accesses) = analyze_table_access(query, known_tables);
        if !matches!(command, Some("INSERT" | "UPDATE")) {
            return;
        }
        let tokens = tokenize(query);
        for access in accesses.iter().filter(|access| {
            matches!(
                access.mode,
                SqlAccessMode::Write | SqlAccessMode::ReadWrite | SqlAccessMode::Unknown
            ) && dependencies.contains_key(&access.table)
        }) {
            let affected = dependencies.get(&access.table).expect("filtered above");
            let writes = if command == Some("INSERT") {
                None
            } else {
                update_assignment_columns(&tokens)
            };
            let safe_api_insert = command == Some("INSERT")
                && api_insert_table == Some(access.table.as_str())
                && computed_fields
                    .get(&access.table)
                    .is_some_and(|fields| insert_binds_computed_fields(&tokens, fields));
            let safe_api_update = command == Some("UPDATE")
                && api_insert_table == Some(access.table.as_str())
                && update_binds_computed_dependencies(&tokens, &access.table, affected);
            let reason = match writes {
                None if command == Some("INSERT") && safe_api_insert => None,
                None if command == Some("INSERT") => Some(
                    "INSERT cannot derive computed columns outside the checked CRUD recomputation path",
                ),
                None => Some("SQL update target is ambiguous for computed-column safety"),
                Some(columns)
                    if columns.iter().any(|column| affected.contains(column))
                        && safe_api_update =>
                {
                    None
                }
                Some(columns) if columns.iter().any(|column| affected.contains(column)) => Some(
                    "SQL cannot modify a computed column or one of its inputs; use the checked CRUD recomputation path",
                ),
                Some(_) => None,
            };
            if let Some(reason) = reason {
                errors.push(sql_error(
                    span,
                    format!("{reason} on table `{}`", access.table),
                ));
            }
        }
    }

    fn visit_expr(
        expression: &Expr,
        known_tables: &[String],
        dependencies: &HashMap<String, HashSet<String>>,
        computed_fields: &HashMap<String, HashSet<String>>,
        api_insert_table: Option<&str>,
        errors: &mut Vec<SqlError>,
    ) {
        match &expression.kind {
            ExprKind::Sql { query, .. } => check_query_writes(
                query,
                expression.span,
                known_tables,
                dependencies,
                computed_fields,
                api_insert_table,
                errors,
            ),
            ExprKind::Array(values) => {
                for value in values {
                    visit_expr(
                        value,
                        known_tables,
                        dependencies,
                        computed_fields,
                        api_insert_table,
                        errors,
                    );
                }
            }
            ExprKind::Map(entries) => {
                for (key, value) in entries {
                    visit_expr(
                        key,
                        known_tables,
                        dependencies,
                        computed_fields,
                        api_insert_table,
                        errors,
                    );
                    visit_expr(
                        value,
                        known_tables,
                        dependencies,
                        computed_fields,
                        api_insert_table,
                        errors,
                    );
                }
            }
            ExprKind::Record { fields, .. } => {
                for (_, value) in fields {
                    visit_expr(
                        value,
                        known_tables,
                        dependencies,
                        computed_fields,
                        api_insert_table,
                        errors,
                    );
                }
            }
            ExprKind::Lambda { body, .. } => visit_expr(
                body,
                known_tables,
                dependencies,
                computed_fields,
                api_insert_table,
                errors,
            ),
            ExprKind::Index { target, index } => {
                visit_expr(
                    target,
                    known_tables,
                    dependencies,
                    computed_fields,
                    api_insert_table,
                    errors,
                );
                visit_expr(
                    index,
                    known_tables,
                    dependencies,
                    computed_fields,
                    api_insert_table,
                    errors,
                );
            }
            ExprKind::Field { target, .. }
            | ExprKind::Unary { expr: target, .. }
            | ExprKind::Await(target) => visit_expr(
                target,
                known_tables,
                dependencies,
                computed_fields,
                api_insert_table,
                errors,
            ),
            ExprKind::Call { args, .. } => {
                for argument in args {
                    visit_expr(
                        argument,
                        known_tables,
                        dependencies,
                        computed_fields,
                        api_insert_table,
                        errors,
                    );
                }
            }
            ExprKind::Binary { left, right, .. } => {
                visit_expr(
                    left,
                    known_tables,
                    dependencies,
                    computed_fields,
                    api_insert_table,
                    errors,
                );
                visit_expr(
                    right,
                    known_tables,
                    dependencies,
                    computed_fields,
                    api_insert_table,
                    errors,
                );
            }
            ExprKind::Match { value, arms } => {
                visit_expr(
                    value,
                    known_tables,
                    dependencies,
                    computed_fields,
                    api_insert_table,
                    errors,
                );
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        visit_expr(
                            guard,
                            known_tables,
                            dependencies,
                            computed_fields,
                            api_insert_table,
                            errors,
                        );
                    }
                    visit_expr(
                        &arm.value,
                        known_tables,
                        dependencies,
                        computed_fields,
                        api_insert_table,
                        errors,
                    );
                }
            }
            ExprKind::Int(_)
            | ExprKind::UInt(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::String(_)
            | ExprKind::Char(_)
            | ExprKind::FunctionValue(_)
            | ExprKind::Variable(_) => {}
        }
    }

    fn visit_block(
        block: &Block,
        known_tables: &[String],
        dependencies: &HashMap<String, HashSet<String>>,
        computed_fields: &HashMap<String, HashSet<String>>,
        api_insert_table: Option<&str>,
        errors: &mut Vec<SqlError>,
    ) {
        for statement in &block.statements {
            match statement {
                Stmt::Let { value, .. }
                | Stmt::BindOrAssign { value, .. }
                | Stmt::TupleBindOrAssign { value, .. } => visit_expr(
                    value,
                    known_tables,
                    dependencies,
                    computed_fields,
                    api_insert_table,
                    errors,
                ),
                Stmt::ArrayIndexAssign { index, value, .. } => {
                    visit_expr(
                        index,
                        known_tables,
                        dependencies,
                        computed_fields,
                        api_insert_table,
                        errors,
                    );
                    visit_expr(
                        value,
                        known_tables,
                        dependencies,
                        computed_fields,
                        api_insert_table,
                        errors,
                    );
                }
                Stmt::Expr(expression) => visit_expr(
                    expression,
                    known_tables,
                    dependencies,
                    computed_fields,
                    api_insert_table,
                    errors,
                ),
                Stmt::Return {
                    value: Some(value), ..
                } => visit_expr(
                    value,
                    known_tables,
                    dependencies,
                    computed_fields,
                    api_insert_table,
                    errors,
                ),
                Stmt::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    visit_expr(
                        condition,
                        known_tables,
                        dependencies,
                        computed_fields,
                        api_insert_table,
                        errors,
                    );
                    visit_block(
                        then_block,
                        known_tables,
                        dependencies,
                        computed_fields,
                        api_insert_table,
                        errors,
                    );
                    if let Some(block) = else_block {
                        visit_block(
                            block,
                            known_tables,
                            dependencies,
                            computed_fields,
                            api_insert_table,
                            errors,
                        );
                    }
                }
                Stmt::While {
                    condition,
                    invariants,
                    body,
                    ..
                } => {
                    visit_expr(
                        condition,
                        known_tables,
                        dependencies,
                        computed_fields,
                        api_insert_table,
                        errors,
                    );
                    for invariant in invariants {
                        visit_expr(
                            invariant,
                            known_tables,
                            dependencies,
                            computed_fields,
                            api_insert_table,
                            errors,
                        );
                    }
                    visit_block(
                        body,
                        known_tables,
                        dependencies,
                        computed_fields,
                        api_insert_table,
                        errors,
                    );
                }
                Stmt::For { iterable, body, .. } => {
                    visit_expr(
                        iterable,
                        known_tables,
                        dependencies,
                        computed_fields,
                        api_insert_table,
                        errors,
                    );
                    visit_block(
                        body,
                        known_tables,
                        dependencies,
                        computed_fields,
                        api_insert_table,
                        errors,
                    );
                }
                Stmt::Loop {
                    invariants, body, ..
                } => {
                    for invariant in invariants {
                        visit_expr(
                            invariant,
                            known_tables,
                            dependencies,
                            computed_fields,
                            api_insert_table,
                            errors,
                        );
                    }
                    visit_block(
                        body,
                        known_tables,
                        dependencies,
                        computed_fields,
                        api_insert_table,
                        errors,
                    );
                }
                Stmt::Match { value, arms, .. } => {
                    visit_expr(
                        value,
                        known_tables,
                        dependencies,
                        computed_fields,
                        api_insert_table,
                        errors,
                    );
                    for arm in arms {
                        visit_block(
                            &arm.body,
                            known_tables,
                            dependencies,
                            computed_fields,
                            api_insert_table,
                            errors,
                        );
                    }
                }
                Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => visit_block(
                    body,
                    known_tables,
                    dependencies,
                    computed_fields,
                    api_insert_table,
                    errors,
                ),
                Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
            }
        }
    }

    for function in &program.functions {
        let api_insert_table = program.apis.iter().find_map(|api| {
            (api.handler.as_deref() == Some(function.name.as_str()))
                .then_some(api.validates_table.as_deref())
                .flatten()
        });
        visit_block(
            &function.body,
            &known_tables,
            &dependencies,
            &computed_fields,
            api_insert_table,
            errors,
        );
    }
    for form in &program.forms {
        for action in &form.actions {
            visit_block(
                &Block {
                    statements: action.statements.clone(),
                    span: action.span,
                },
                &known_tables,
                &dependencies,
                &computed_fields,
                None,
                errors,
            );
        }
    }
    for crud in &program.cruds {
        for action in &crud.actions {
            visit_block(
                &Block {
                    statements: action.statements.clone(),
                    span: action.span,
                },
                &known_tables,
                &dependencies,
                &computed_fields,
                None,
                errors,
            );
        }
    }
    for page in &program.pages {
        for data in &page.data {
            check_query_writes(
                &data.query,
                data.span,
                &known_tables,
                &dependencies,
                &computed_fields,
                None,
                errors,
            );
        }
    }
}

fn insert_binds_computed_fields(tokens: &[SqlToken], computed_fields: &HashSet<String>) -> bool {
    if !matches!(tokens.first(), Some(SqlToken::Word(word)) if word.eq_ignore_ascii_case("INSERT"))
        || computed_fields.is_empty()
    {
        return false;
    }
    let Some(into_index) = tokens.iter().position(
        |token| matches!(token, SqlToken::Word(word) if word.eq_ignore_ascii_case("INTO")),
    ) else {
        return false;
    };
    let Some(open_columns) = tokens[into_index + 1..]
        .iter()
        .position(|token| matches!(token, SqlToken::OpenParen))
        .map(|relative| into_index + 1 + relative)
    else {
        return false;
    };
    let Some(close_columns) = tokens[open_columns + 1..]
        .iter()
        .position(|token| matches!(token, SqlToken::CloseParen))
        .map(|relative| open_columns + 1 + relative)
    else {
        return false;
    };
    let columns = tokens[open_columns + 1..close_columns]
        .iter()
        .filter_map(|token| match token {
            SqlToken::Word(name) => Some(name.to_ascii_lowercase()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if computed_fields
        .iter()
        .any(|field| !columns.iter().any(|column| column == field))
    {
        return false;
    }
    let Some(values_index) = tokens[close_columns + 1..]
        .iter()
        .position(
            |token| matches!(token, SqlToken::Word(word) if word.eq_ignore_ascii_case("VALUES")),
        )
        .map(|relative| close_columns + 1 + relative)
    else {
        return false;
    };
    let Some(SqlToken::OpenParen) = tokens.get(values_index + 1) else {
        return false;
    };
    let mut values = Vec::<Vec<SqlToken>>::new();
    let mut current = Vec::new();
    let mut depth = 0_i32;
    let mut closed = false;
    for token in &tokens[values_index + 2..] {
        match token {
            SqlToken::OpenParen => {
                depth += 1;
                current.push(token.clone());
            }
            SqlToken::CloseParen if depth == 0 => {
                values.push(std::mem::take(&mut current));
                closed = true;
                break;
            }
            SqlToken::CloseParen => {
                depth -= 1;
                current.push(token.clone());
            }
            SqlToken::Comma if depth == 0 => values.push(std::mem::take(&mut current)),
            _ => current.push(token.clone()),
        }
    }
    if !closed || values.len() != columns.len() {
        return false;
    }
    computed_fields.iter().all(|field| {
        let Some(index) = columns.iter().position(|column| column == field) else {
            return false;
        };
        matches!(values.get(index).map(Vec::as_slice), Some([SqlToken::Parameter(name)]) if name.eq_ignore_ascii_case(field))
    })
}

fn update_binds_computed_dependencies(
    tokens: &[SqlToken],
    expected_table: &str,
    affected_columns: &HashSet<String>,
) -> bool {
    if !matches!(tokens.first(), Some(SqlToken::Word(word)) if word.eq_ignore_ascii_case("UPDATE"))
        || !matches!(tokens.get(1), Some(SqlToken::Word(table)) if table.eq_ignore_ascii_case(expected_table))
        || affected_columns.is_empty()
    {
        return false;
    }
    let Some(set_index) = tokens.iter().position(
        |token| matches!(token, SqlToken::Word(word) if word.eq_ignore_ascii_case("SET")),
    ) else {
        return false;
    };
    let mut assignments = HashMap::<String, Vec<SqlToken>>::new();
    let mut segment = Vec::new();
    let mut depth = 0_i32;
    let mut segments = Vec::new();
    for token in &tokens[set_index + 1..] {
        match token {
            SqlToken::OpenParen => depth += 1,
            SqlToken::CloseParen => depth -= 1,
            SqlToken::Comma if depth == 0 => {
                segments.push(std::mem::take(&mut segment));
                continue;
            }
            SqlToken::Word(word)
                if depth == 0
                    && ["WHERE", "RETURNING", "ORDER", "LIMIT"]
                        .iter()
                        .any(|term| word.eq_ignore_ascii_case(term)) =>
            {
                break
            }
            _ => {}
        }
        segment.push(token.clone());
    }
    if !segment.is_empty() {
        segments.push(segment);
    }
    for segment in segments {
        let Some(equal) = segment
            .iter()
            .position(|token| matches!(token, SqlToken::Operator))
        else {
            return false;
        };
        let Some(SqlToken::Word(column)) = segment.get(equal.wrapping_sub(1)) else {
            return false;
        };
        let column = column.to_ascii_lowercase();
        if assignments
            .insert(column, segment[equal + 1..].to_vec())
            .is_some()
        {
            return false;
        }
    }
    affected_columns.iter().all(|column| {
        matches!(assignments.get(column).map(Vec::as_slice), Some([SqlToken::Parameter(name)]) if name.eq_ignore_ascii_case(column))
    })
}

fn update_assignment_columns(tokens: &[SqlToken]) -> Option<HashSet<String>> {
    tokens.first().and_then(|token| match token {
        SqlToken::Word(command) if command.eq_ignore_ascii_case("UPDATE") => Some(()),
        _ => None,
    })?;
    let set_index = tokens.iter().position(
        |token| matches!(token, SqlToken::Word(word) if word.eq_ignore_ascii_case("SET")),
    )?;
    let mut columns = HashSet::new();
    let mut assignment = Vec::new();
    let mut depth = 0_i32;
    let mut segments = Vec::new();
    for token in &tokens[set_index + 1..] {
        match token {
            SqlToken::OpenParen => depth += 1,
            SqlToken::CloseParen => depth -= 1,
            SqlToken::Comma if depth == 0 => {
                segments.push(std::mem::take(&mut assignment));
                continue;
            }
            SqlToken::Word(word)
                if depth == 0
                    && ["WHERE", "RETURNING", "ORDER", "LIMIT"]
                        .iter()
                        .any(|term| word.eq_ignore_ascii_case(term)) =>
            {
                break
            }
            _ => {}
        }
        assignment.push(token.clone());
    }
    if !assignment.is_empty() {
        segments.push(assignment);
    }
    for segment in segments {
        let equal = segment
            .iter()
            .position(|token| matches!(token, SqlToken::Operator))?;
        let target = &segment[..equal];
        let name = match target {
            [SqlToken::Word(column)] => column,
            [SqlToken::Word(_table), SqlToken::Dot, SqlToken::Word(column)] => column,
            _ => return None,
        };
        columns.insert(name.to_ascii_lowercase());
    }
    Some(columns)
}

fn check_block(
    block: &Block,
    schema: &Schema,
    environment: &mut HashMap<String, Type>,
    aliases: &[zelyra_ast::TypeDef],
    errors: &mut Vec<SqlError>,
) {
    for statement in &block.statements {
        for ty in environment.values_mut() {
            *ty = super::resolve_type(ty, aliases);
        }
        match statement {
            Stmt::Let {
                name, ty, value, ..
            } => {
                check_expr(value, schema, environment, aliases, errors);
                environment.insert(name.clone(), ty.clone().unwrap_or_else(|| expr_type(value)));
            }
            Stmt::BindOrAssign { name, value, .. } => {
                check_expr(value, schema, environment, aliases, errors);
                environment.insert(name.clone(), expr_type(value));
            }
            Stmt::ArrayIndexAssign { index, value, .. } => {
                check_expr(index, schema, environment, aliases, errors);
                check_expr(value, schema, environment, aliases, errors);
            }
            Stmt::TupleBindOrAssign { names, value, .. } => {
                check_expr(value, schema, environment, aliases, errors);
                if let Type::Tuple(types) = expr_type(value) {
                    for (name, ty) in names.iter().zip(types) {
                        environment.insert(name.clone(), ty);
                    }
                }
            }
            Stmt::Expr(expression) => check_expr(expression, schema, environment, aliases, errors),
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    check_expr(value, schema, environment, aliases, errors);
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                check_expr(condition, schema, environment, aliases, errors);
                check_block(then_block, schema, environment, aliases, errors);
                if let Some(else_block) = else_block {
                    check_block(else_block, schema, environment, aliases, errors);
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                check_expr(condition, schema, environment, aliases, errors);
                for invariant in invariants {
                    check_expr(invariant, schema, environment, aliases, errors);
                }
                check_block(body, schema, environment, aliases, errors);
            }
            Stmt::For { iterable, body, .. } => {
                check_expr(iterable, schema, environment, aliases, errors);
                check_block(body, schema, environment, aliases, errors);
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    check_expr(invariant, schema, environment, aliases, errors);
                }
                check_block(body, schema, environment, aliases, errors);
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::Match { value, arms, .. } => {
                check_expr(value, schema, environment, aliases, errors);
                for arm in arms {
                    check_block(&arm.body, schema, environment, aliases, errors);
                }
            }
            Stmt::Transaction { body, .. } => {
                check_block(body, schema, environment, aliases, errors);
            }
            Stmt::Parallel { body, .. } => {
                check_block(body, schema, environment, aliases, errors);
            }
        }
    }
}

fn check_expr(
    expression: &Expr,
    schema: &Schema,
    environment: &HashMap<String, Type>,
    aliases: &[zelyra_ast::TypeDef],
    errors: &mut Vec<SqlError>,
) {
    match &expression.kind {
        ExprKind::Sql {
            result_type, query, ..
        } => errors.extend(check_query(
            query,
            &super::resolve_type(result_type, aliases),
            schema,
            environment,
            expression.span,
        )),
        ExprKind::Call { args, .. } => {
            for argument in args {
                check_expr(argument, schema, environment, aliases, errors);
            }
        }
        ExprKind::Unary { expr, .. } => check_expr(expr, schema, environment, aliases, errors),
        ExprKind::Binary { left, right, .. } => {
            check_expr(left, schema, environment, aliases, errors);
            check_expr(right, schema, environment, aliases, errors);
        }
        ExprKind::Array(values) => {
            for value in values {
                check_expr(value, schema, environment, aliases, errors);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                check_expr(key, schema, environment, aliases, errors);
                check_expr(value, schema, environment, aliases, errors);
            }
        }
        ExprKind::Record { fields, .. } => {
            for (_, value) in fields {
                check_expr(value, schema, environment, aliases, errors);
            }
        }
        ExprKind::Index { target, index } => {
            check_expr(target, schema, environment, aliases, errors);
            check_expr(index, schema, environment, aliases, errors);
        }
        ExprKind::Field { target, .. } => check_expr(target, schema, environment, aliases, errors),
        ExprKind::Lambda { body, .. } => check_expr(body, schema, environment, aliases, errors),
        ExprKind::Await(inner) => check_expr(inner, schema, environment, aliases, errors),
        ExprKind::Match { value, arms } => {
            check_expr(value, schema, environment, aliases, errors);
            for arm in arms {
                check_expr(&arm.value, schema, environment, aliases, errors);
            }
        }
        ExprKind::Int(_)
        | ExprKind::UInt(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::String(_)
        | ExprKind::Char(_)
        | ExprKind::Variable(_)
        | ExprKind::FunctionValue(_) => {}
    }
}

fn expr_type(expression: &Expr) -> Type {
    match &expression.kind {
        ExprKind::Int(_) => Type::Int,
        ExprKind::UInt(_) => Type::UInt,
        ExprKind::Float(_) => Type::Float,
        ExprKind::Bool(_) => Type::Bool,
        ExprKind::String(_) => Type::String,
        ExprKind::Char(_) => Type::Char,
        ExprKind::Array(values) => Type::Array(Box::new(
            values.first().map(expr_type).unwrap_or(Type::Unknown),
        )),
        ExprKind::Map(entries) => Type::Map(
            Box::new(
                entries
                    .first()
                    .map(|(key, _)| expr_type(key))
                    .unwrap_or(Type::Unknown),
            ),
            Box::new(
                entries
                    .first()
                    .map(|(_, value)| expr_type(value))
                    .unwrap_or(Type::Unknown),
            ),
        ),
        ExprKind::Index { .. } => Type::Unknown,
        ExprKind::Record { type_name, .. } => Type::Named(type_name.clone()),
        ExprKind::Lambda { params, body } => Type::Function {
            params: params.iter().map(|param| param.ty.clone()).collect(),
            return_type: Box::new(expr_type(body)),
            capabilities: Vec::new(),
        },
        ExprKind::Field { .. } => Type::Unknown,
        ExprKind::Sql { result_type, .. } => result_type.clone(),
        ExprKind::Variable(_)
        | ExprKind::FunctionValue(_)
        | ExprKind::Call { .. }
        | ExprKind::Unary { .. }
        | ExprKind::Binary { .. } => Type::Unknown,
        ExprKind::Await(inner) => expr_type(inner),
        ExprKind::Match { arms, .. } => arms
            .first()
            .map(|arm| expr_type(&arm.value))
            .unwrap_or(Type::Unknown),
    }
}

fn check_query(
    query: &str,
    result_type: &Type,
    schema: &Schema,
    environment: &HashMap<String, Type>,
    span: Span,
) -> Vec<SqlError> {
    let tokens = tokenize(query);
    let mut errors = Vec::new();
    let Some(SqlToken::Word(command)) = tokens.first() else {
        return vec![sql_error(span, "SQL query must not be empty")];
    };
    let command = command.to_ascii_uppercase();
    if !matches!(command.as_str(), "SELECT" | "INSERT" | "UPDATE" | "DELETE") {
        errors.push(sql_error(
            span,
            format!("unsupported SQL statement `{command}`"),
        ));
        return errors;
    }

    let table_names = schema
        .tables
        .iter()
        .map(|table| table.name.clone())
        .collect::<HashSet<_>>();
    let tables = query_tables(&tokens, schema, &table_names, span, &mut errors);
    for token in &tokens {
        let SqlToken::Parameter(name) = token else {
            continue;
        };
        if !environment.contains_key(name) {
            errors.push(sql_error(
                span,
                format!("SQL parameter `:{name}` is not available in this scope"),
            ));
        }
    }
    for (index, token) in tokens.iter().enumerate() {
        let SqlToken::Parameter(name) = token else {
            continue;
        };
        let Some(actual) = environment.get(name) else {
            continue;
        };
        let Some((column_name, expected)) = parameter_column_type(&tokens, index, &tables, schema)
        else {
            continue;
        };
        if !sql_types_compatible(&expected, actual) {
            errors.push(sql_error(
                span,
                format!(
                    "SQL parameter `:{name}` has type `{actual}`, but column `{column_name}` expects `{expected}`"
                ),
            ));
        }
    }

    if command == "SELECT" {
        check_select_columns(&tokens, &tables, schema, span, &mut errors);
        check_result_mapping(result_type, &table_names, span, &mut errors);
    } else if !matches!(result_type, Type::Unit) {
        errors.push(sql_error(
            span,
            "non-SELECT SQL blocks must use the result type `Unit`",
        ));
    }
    errors
}

struct TableViewProjection<'a> {
    columns: &'a [String],
    row_fields: &'a [String],
}

fn check_tableview_query(
    query: &str,
    result_type: &Type,
    schema: &Schema,
    environment: &HashMap<String, Type>,
    records: &[zelyra_ast::RecordDef],
    projection: TableViewProjection<'_>,
    span: Span,
) -> Vec<SqlError> {
    let mut errors = check_query(query, result_type, schema, environment, span);
    if let Some(record_name) = result_record_name(result_type) {
        if records.iter().any(|record| record.name == record_name) {
            errors.retain(|error| {
                !error.message.contains(&format!(
                    "SQL result type `{record_name}` does not map to a table"
                ))
            });
            if let Some(record) = records.iter().find(|record| record.name == record_name) {
                errors.extend(check_record_projection(
                    query,
                    record,
                    projection.columns,
                    projection.row_fields,
                    schema,
                    span,
                ));
            }
        }
    }
    errors
}

fn result_record_name(result_type: &Type) -> Option<&str> {
    let result_type = match result_type {
        Type::Array(inner) | Type::Option(inner) => inner,
        _ => result_type,
    };
    match result_type {
        Type::Named(name) => Some(name),
        _ => None,
    }
}

fn check_record_projection(
    query: &str,
    record: &zelyra_ast::RecordDef,
    columns: &[String],
    row_fields: &[String],
    schema: &Schema,
    span: Span,
) -> Vec<SqlError> {
    let tokens = tokenize(query);
    let mut errors = Vec::new();
    let Some(projections) = select_projections(&tokens) else {
        errors.push(sql_error(
            span,
            "tableview struct results require an explicit SELECT projection",
        ));
        return errors;
    };
    let table_names = schema
        .tables
        .iter()
        .map(|table| table.name.clone())
        .collect::<HashSet<_>>();
    let mut table_errors = Vec::new();
    let tables = query_tables(&tokens, schema, &table_names, span, &mut table_errors);
    let mut names = HashSet::new();
    for projection in projections {
        let Some(name) = projection_name(&projection) else {
            errors.push(sql_error(
                span,
                "computed tableview result expressions require an explicit AS alias",
            ));
            continue;
        };
        let normalized = name.to_ascii_lowercase();
        if !names.insert(normalized.clone()) {
            errors.push(sql_error(
                span,
                format!("tableview result alias `{name}` is specified more than once"),
            ));
            continue;
        }
        let Some(field) = record
            .fields
            .iter()
            .find(|field| field.name.eq_ignore_ascii_case(&normalized))
        else {
            errors.push(sql_error(
                span,
                format!(
                    "tableview result alias `{name}` is not a field of `{}`",
                    record.name
                ),
            ));
            continue;
        };
        if !columns
            .iter()
            .chain(row_fields.iter())
            .any(|column| column.eq_ignore_ascii_case(&normalized))
        {
            errors.push(sql_error(
                span,
                format!("tableview result alias `{name}` is not declared as a view or row field"),
            ));
            continue;
        }
        let (actual_type, nullable) = projection_type(&projection, &tables, schema);
        if !matches!(actual_type, Type::Unknown)
            && !result_types_compatible(&field.ty, &actual_type)
        {
            errors.push(sql_error(
                span,
                format!(
                    "tableview result field `{name}` expects `{}`, but SQL produces `{actual_type}`",
                    field.ty
                ),
            ));
        }
        if nullable && !matches!(field.ty, Type::Option(_)) {
            errors.push(sql_error(
                span,
                format!("tableview result field `{name}` may be NULL but is not optional"),
            ));
        }
    }
    for column in columns {
        if !names.contains(&column.to_ascii_lowercase()) {
            errors.push(sql_error(
                span,
                format!("tableview column `{column}` is missing from the SQL projection"),
            ));
        }
    }
    for field in row_fields {
        if !names.contains(&field.to_ascii_lowercase()) {
            errors.push(sql_error(
                span,
                format!("tableview row field `{field}` is missing from the SQL projection"),
            ));
        }
    }
    errors
}

fn select_projections(tokens: &[SqlToken]) -> Option<Vec<Vec<SqlToken>>> {
    if !matches!(tokens.first(), Some(SqlToken::Word(word)) if word.eq_ignore_ascii_case("SELECT"))
    {
        return None;
    }
    let mut depth = 0_u32;
    let mut current = Vec::new();
    let mut projections = Vec::new();
    for token in tokens.iter().skip(1) {
        match token {
            SqlToken::OpenParen => depth += 1,
            SqlToken::CloseParen => depth = depth.saturating_sub(1),
            SqlToken::Word(word) if depth == 0 && word.eq_ignore_ascii_case("FROM") => {
                if !current.is_empty() {
                    projections.push(current);
                }
                return (!projections.is_empty()).then_some(projections);
            }
            SqlToken::Comma if depth == 0 => {
                if current.is_empty() {
                    return None;
                }
                projections.push(current);
                current = Vec::new();
                continue;
            }
            _ => {}
        }
        current.push(token.clone());
    }
    None
}

fn projection_name(projection: &[SqlToken]) -> Option<String> {
    let mut depth = 0_u32;
    for index in 0..projection.len() {
        match &projection[index] {
            SqlToken::OpenParen => depth += 1,
            SqlToken::CloseParen => depth = depth.saturating_sub(1),
            SqlToken::Word(word)
                if depth == 0
                    && word.eq_ignore_ascii_case("AS")
                    && matches!(projection.get(index + 1), Some(SqlToken::Word(_))) =>
            {
                if let Some(SqlToken::Word(alias)) = projection.get(index + 1) {
                    return Some(alias.clone());
                }
            }
            _ => {}
        }
    }
    match projection {
        [SqlToken::Word(name)] => Some(name.clone()),
        [SqlToken::Word(_), SqlToken::Dot, SqlToken::Word(name)] => Some(name.clone()),
        _ => None,
    }
}

fn projection_type(
    projection: &[SqlToken],
    tables: &HashMap<String, String>,
    schema: &Schema,
) -> (Type, bool) {
    let expression_end = projection
        .iter()
        .position(|token| matches!(token, SqlToken::Word(word) if word.eq_ignore_ascii_case("AS")))
        .unwrap_or(projection.len());
    let expression = &projection[..expression_end];
    if let Some(SqlToken::Word(function)) = expression.first() {
        if matches!(expression.get(1), Some(SqlToken::OpenParen)) {
            return match function.to_ascii_uppercase().as_str() {
                "COUNT" => (Type::Int, false),
                "ROUND" => {
                    let inner = expression.get(2..expression.len().saturating_sub(1));
                    let (_, nullable) = inner
                        .and_then(|inner| {
                            inner.split(|token| matches!(token, SqlToken::Comma)).next()
                        })
                        .map(|amount| projection_type(amount, tables, schema))
                        .unwrap_or((Type::Unknown, true));
                    (Type::Decimal, nullable)
                }
                "SUM" | "AVG" | "MIN" | "MAX" => {
                    let inner = expression.get(2..expression.len().saturating_sub(1));
                    let (ty, _) = inner
                        .map(|inner| projection_type(inner, tables, schema))
                        .unwrap_or((Type::Unknown, true));
                    (ty, true)
                }
                "COALESCE" | "IFNULL" => {
                    let inner = expression.get(2..expression.len().saturating_sub(1));
                    let (ty, _) = inner
                        .and_then(|inner| {
                            inner
                                .split(|token| matches!(token, SqlToken::Comma))
                                .next()
                                .map(|part| projection_type(part, tables, schema))
                        })
                        .unwrap_or((Type::Unknown, false));
                    (ty, false)
                }
                _ => (Type::Unknown, true),
            };
        }
    }
    if let Some(column) = projection_source_column(expression, tables, schema) {
        return (sql_type_to_type(&column.sql_type), column.nullable);
    }
    if expression
        .iter()
        .any(|token| matches!(token, SqlToken::Operator | SqlToken::Star))
    {
        let mut inferred = Type::Unknown;
        for pair in expression.windows(3) {
            let [SqlToken::Word(table), SqlToken::Dot, SqlToken::Word(_)] = pair else {
                continue;
            };
            let Some(table_name) = tables.get(&table.to_ascii_lowercase()) else {
                continue;
            };
            let Some(SqlToken::Word(column_name)) = pair.get(2) else {
                continue;
            };
            if let Some(column) = schema
                .tables
                .iter()
                .find(|candidate| candidate.name == *table_name)
                .and_then(|candidate| {
                    candidate
                        .columns
                        .iter()
                        .find(|candidate| candidate.name.eq_ignore_ascii_case(column_name))
                })
            {
                let column_type = sql_type_to_type(&column.sql_type);
                if matches!(column_type, Type::Decimal | Type::Float) {
                    inferred = column_type;
                    if matches!(inferred, Type::Decimal) {
                        break;
                    }
                } else if matches!(column_type, Type::Int) && matches!(inferred, Type::Unknown) {
                    inferred = Type::Int;
                }
            }
        }
        if !matches!(inferred, Type::Unknown) {
            return (inferred, false);
        }
    }
    match expression.first() {
        Some(SqlToken::Number) => (Type::Int, false),
        Some(SqlToken::String) => (Type::String, false),
        _ => (Type::Unknown, true),
    }
}

fn projection_source_column<'a>(
    expression: &[SqlToken],
    tables: &HashMap<String, String>,
    schema: &'a Schema,
) -> Option<&'a super::Column> {
    let (table_name, column_name) = match expression {
        [SqlToken::Word(table), SqlToken::Dot, SqlToken::Word(column)] => {
            (tables.get(&table.to_ascii_lowercase())?, column)
        }
        [SqlToken::Word(column)] => (tables.values().next()?, column),
        _ => return None,
    };
    schema
        .tables
        .iter()
        .find(|table| table.name == *table_name)
        .and_then(|table| {
            table
                .columns
                .iter()
                .find(|column| column.name.eq_ignore_ascii_case(column_name))
        })
}

fn result_types_compatible(expected: &Type, actual: &Type) -> bool {
    if matches!(actual, Type::Unknown) || expected == actual {
        return true;
    }
    match expected {
        Type::Named(name) if actual == &Type::Int => name.to_ascii_lowercase().ends_with("id"),
        Type::Named(name) if actual == &Type::String => {
            matches!(name.as_str(), "Email" | "Url" | "Uuid" | "FileRef")
        }
        Type::Option(inner) => result_types_compatible(inner, actual),
        _ => false,
    }
}

fn query_tables(
    tokens: &[SqlToken],
    schema: &Schema,
    table_names: &HashSet<String>,
    span: Span,
    errors: &mut Vec<SqlError>,
) -> HashMap<String, String> {
    let mut tables = HashMap::new();
    for index in 0..tokens.len() {
        let SqlToken::Word(keyword) = &tokens[index] else {
            continue;
        };
        if !matches!(
            keyword.to_ascii_uppercase().as_str(),
            "FROM" | "JOIN" | "INTO" | "UPDATE"
        ) {
            continue;
        }
        let Some(SqlToken::Word(table_name)) = tokens.get(index + 1) else {
            continue;
        };
        let normalized = table_name.to_ascii_lowercase();
        if !table_names.contains(&normalized) {
            errors.push(sql_error(
                span,
                format!("table `{table_name}` does not exist in the Zelyra schema"),
            ));
            continue;
        }
        tables.insert(normalized.clone(), normalized.clone());
        if let Some(SqlToken::Word(alias)) = tokens.get(index + 2) {
            if !sql_keyword(alias) {
                tables.insert(alias.to_ascii_lowercase(), normalized);
            }
        }
    }
    if tables.is_empty() && !schema.tables.is_empty() {
        errors.push(sql_error(span, "SQL query does not reference a table"));
    }
    tables
}

fn check_select_columns(
    tokens: &[SqlToken],
    tables: &HashMap<String, String>,
    schema: &Schema,
    span: Span,
    errors: &mut Vec<SqlError>,
) {
    for index in 0..tokens.len() {
        let SqlToken::Word(word) = &tokens[index] else {
            continue;
        };
        let lower = word.to_ascii_lowercase();
        if sql_keyword(word) || tables.contains_key(&lower) {
            continue;
        }
        if matches!(tokens.get(index + 1), Some(SqlToken::OpenParen)) {
            continue;
        }
        if matches!(tokens.get(index + 1), Some(SqlToken::Dot)) {
            continue;
        }
        let qualified_table = if index >= 2 && matches!(tokens.get(index - 1), Some(SqlToken::Dot))
        {
            match tokens.get(index - 2) {
                Some(SqlToken::Word(alias)) => tables.get(&alias.to_ascii_lowercase()),
                _ => None,
            }
        } else {
            None
        };
        if matches!(tokens.get(index - 1), Some(SqlToken::Dot)) && qualified_table.is_none() {
            errors.push(sql_error(
                span,
                format!("table alias for column `{word}` does not exist"),
            ));
            continue;
        }
        let exists = if let Some(table_name) = qualified_table {
            schema
                .tables
                .iter()
                .find(|table| table.name == *table_name)
                .is_some_and(|table| table.columns.iter().any(|column| column.name == lower))
        } else {
            tables.values().any(|table_name| {
                schema
                    .tables
                    .iter()
                    .find(|table| table.name == *table_name)
                    .is_some_and(|table| table.columns.iter().any(|column| column.name == lower))
            })
        };
        if !exists && !is_sql_alias(tokens, index) {
            errors.push(sql_error(
                span,
                format!("column `{word}` does not exist in the referenced tables"),
            ));
        }
    }
}

fn check_result_mapping(
    result_type: &Type,
    table_names: &HashSet<String>,
    span: Span,
    errors: &mut Vec<SqlError>,
) {
    match result_type {
        Type::Array(inner) | Type::Option(inner) => {
            check_result_mapping(inner, table_names, span, errors)
        }
        Type::Named(name) => {
            if table_for_type(name, table_names).is_none() {
                errors.push(sql_error(
                    span,
                    format!("SQL result type `{name}` does not map to a table"),
                ));
            }
        }
        Type::Unit => errors.push(sql_error(span, "SELECT SQL blocks require a result type")),
        _ => {}
    }
}

fn parameter_column_type(
    tokens: &[SqlToken],
    parameter_index: usize,
    tables: &HashMap<String, String>,
    schema: &Schema,
) -> Option<(String, Type)> {
    if parameter_index < 2 || !matches!(tokens.get(parameter_index - 1), Some(SqlToken::Operator)) {
        return None;
    }
    let column_index = parameter_index - 2;
    let SqlToken::Word(column_name) = tokens.get(column_index)? else {
        return None;
    };
    let lower_column = column_name.to_ascii_lowercase();
    let table = if column_index >= 2 && matches!(tokens.get(column_index - 1), Some(SqlToken::Dot))
    {
        match tokens.get(column_index - 2) {
            Some(SqlToken::Word(alias)) => tables.get(&alias.to_ascii_lowercase()),
            _ => None,
        }
    } else {
        None
    };
    let column = if let Some(table_name) = table {
        schema
            .tables
            .iter()
            .find(|table| table.name == *table_name)
            .and_then(|table| {
                table
                    .columns
                    .iter()
                    .find(|column| column.name == lower_column)
            })
    } else {
        tables.values().find_map(|table_name| {
            schema
                .tables
                .iter()
                .find(|table| table.name == *table_name)
                .and_then(|table| {
                    table
                        .columns
                        .iter()
                        .find(|column| column.name == lower_column)
                })
        })
    }?;
    Some((column.name.clone(), sql_type_to_type(&column.sql_type)))
}

fn sql_type_to_type(sql_type: &str) -> Type {
    let upper = sql_type.to_ascii_uppercase();
    if matches!(upper.as_str(), "JSON" | "JSONB") {
        Type::Json
    } else if upper.contains("CHAR") || upper.contains("TEXT") || upper.contains("CLOB") {
        Type::String
    } else if upper.contains("BOOL") {
        Type::Bool
    } else if upper.contains("REAL") || upper.contains("DOUBLE") || upper.contains("FLOAT") {
        Type::Float
    } else if matches!(upper.as_str(), "DECIMAL(20,0)" | "NUMERIC(20,0)") {
        Type::UInt
    } else if upper.contains("DECIMAL") || upper.contains("NUMERIC") {
        Type::Decimal
    } else if upper == "DATE" {
        Type::Date
    } else if upper == "TIME" || upper.starts_with("TIME(") {
        Type::Time
    } else if upper.contains("TIMESTAMP") || upper.contains("DATETIME") {
        Type::Timestamp
    } else {
        Type::Int
    }
}

fn form_parameter_type(
    ty: &Type,
    definitions: &[zelyra_ast::TypeDef],
    visiting: &mut HashSet<String>,
) -> Type {
    match ty {
        Type::Named(name) if name == "Money" => Type::Decimal,
        Type::Named(name) if visiting.insert(name.clone()) => {
            let resolved = definitions
                .iter()
                .find(|definition| definition.name == *name)
                .map(|definition| form_parameter_type(&definition.target, definitions, visiting))
                .unwrap_or_else(|| ty.clone());
            visiting.remove(name);
            resolved
        }
        Type::Option(inner) => {
            Type::Option(Box::new(form_parameter_type(inner, definitions, visiting)))
        }
        _ => ty.clone(),
    }
}

fn sql_types_compatible(expected: &Type, actual: &Type) -> bool {
    if matches!(actual, Type::Unknown) || expected == actual {
        return true;
    }
    match actual {
        Type::Named(name) if expected == &Type::Int => {
            name.to_ascii_lowercase().ends_with("id")
                || !matches!(
                    name.as_str(),
                    "Email" | "Url" | "Uuid" | "Money" | "FileRef"
                )
        }
        Type::Named(name) if expected == &Type::String => {
            matches!(name.as_str(), "Email" | "Url" | "Uuid" | "FileRef")
        }
        Type::Option(inner) => sql_types_compatible(expected, inner),
        _ => false,
    }
}

fn is_sql_alias(tokens: &[SqlToken], index: usize) -> bool {
    index > 0
        && matches!(tokens.get(index - 1), Some(SqlToken::Word(word)) if word.eq_ignore_ascii_case("AS"))
}

fn sql_keyword(value: &str) -> bool {
    matches!(
        value.to_ascii_uppercase().as_str(),
        "SELECT"
            | "FROM"
            | "WHERE"
            | "ORDER"
            | "BY"
            | "GROUP"
            | "HAVING"
            | "LIMIT"
            | "OFFSET"
            | "ASC"
            | "DESC"
            | "AS"
            | "AND"
            | "OR"
            | "NOT"
            | "NULL"
            | "IS"
            | "IN"
            | "LIKE"
            | "BETWEEN"
            | "JOIN"
            | "LEFT"
            | "RIGHT"
            | "INNER"
            | "OUTER"
            | "CROSS"
            | "ON"
            | "INSERT"
            | "INTO"
            | "VALUES"
            | "UPDATE"
            | "SET"
            | "DELETE"
            | "RETURNING"
            | "DISTINCT"
            | "TRUE"
            | "FALSE"
            | "CASE"
            | "WHEN"
            | "THEN"
            | "ELSE"
            | "END"
    )
}

fn tokenize(query: &str) -> Vec<SqlToken> {
    let bytes = query.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if byte == b'-' && bytes.get(index + 1) == Some(&b'-') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
            {
                index += 1;
            }
            tokens.push(SqlToken::Word(query[start..index].into()));
            continue;
        }
        if byte == b':' {
            index += 1;
            let start = index;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
            {
                index += 1;
            }
            if start < index {
                tokens.push(SqlToken::Parameter(query[start..index].into()));
            }
            continue;
        }
        if byte == b'`' || byte == b'[' {
            let closing_quote = if byte == b'`' { b'`' } else { b']' };
            let content_start = index + 1;
            index += 1;
            while index < bytes.len() && bytes[index] != closing_quote {
                index += 1;
            }
            let content_end = index;
            if index < bytes.len() {
                index += 1;
            }
            tokens.push(SqlToken::Word(query[content_start..content_end].to_owned()));
            continue;
        }
        if byte.is_ascii_digit() {
            index += 1;
            while index < bytes.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'.') {
                index += 1;
            }
            tokens.push(SqlToken::Number);
            continue;
        }
        if byte == b'\'' || byte == b'"' {
            let quote = byte;
            index += 1;
            while index < bytes.len() {
                if bytes[index] == quote {
                    if bytes.get(index + 1) == Some(&quote) {
                        index += 2;
                    } else {
                        index += 1;
                        break;
                    }
                } else {
                    index += 1;
                }
            }
            tokens.push(SqlToken::String);
            continue;
        }
        let token = match byte {
            b'*' => SqlToken::Star,
            b',' => SqlToken::Comma,
            b'.' => SqlToken::Dot,
            b'(' => SqlToken::OpenParen,
            b')' => SqlToken::CloseParen,
            _ => SqlToken::Operator,
        };
        tokens.push(token);
        index += 1;
    }
    tokens
}

fn sql_error(span: Span, message: impl Into<String>) -> SqlError {
    SqlError {
        message: message.into(),
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_schema;
    use zelyra_lexer::lex;
    use zelyra_parser::parse;

    fn source_schema() -> Schema {
        let program = parse(&lex(
            "database main { engine: mariadb } table customers { id: Id primary auto name: String(100) email: Email } fn main() {}",
        )
        .unwrap())
        .unwrap();
        build_schema(&program).unwrap()
    }

    #[test]
    fn computed_columns_reject_raw_inserts_and_updates_to_inputs_or_outputs() {
        let source = r#"
            table invoice_items {
                id: Id primary auto
                quantity: Int required
                unit_price: Int required
                total: Int required compute(quantity * unit_price)
                memo: String?
            }
            fn raw_insert() uses Database(write) {
                transaction { sql { INSERT INTO invoice_items (quantity, unit_price, total) VALUES (1, 20, 20) } }
            }
            fn raw_input_update() uses Database(write) {
                transaction { sql { UPDATE invoice_items SET quantity = 2 WHERE id = 1 } }
            }
            fn raw_output_update() uses Database(write) {
                transaction { sql { UPDATE `invoice_items` SET `total` = 20 WHERE id = 1 } }
            }
            fn safe_memo_update() uses Database(write) {
                transaction { sql { UPDATE invoice_items SET memo = 'ok' WHERE id = 1 } }
            }
            fn main() {}
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        let errors = check_program(&program, &schema)
            .expect_err("ordinary SQL must not bypass computed-field recalculation");
        assert_eq!(errors.len(), 3, "unexpected SQL diagnostics: {errors:?}");
        assert!(errors
            .iter()
            .all(|error| error.message.contains("computed")));
    }

    #[test]
    fn computed_columns_allow_only_validated_api_inserts_bound_to_server_fields() {
        let source = r#"
            table invoice_items {
                id: Id primary auto
                quantity: Int required
                unit_price: Int required
                total: Int required compute(quantity * unit_price)
                validate { total >= 0: "Total must be nonnegative" }
            }
            api POST "/invoice-items" {
                handler create_item
                input { quantity: Int unit_price: Int }
                validates invoice_items
                output Result<Int, String>
                errors { 422 ValidationError: String }
            }
            fn create_item(quantity: Int, unit_price: Int, total: Int) -> Result<Int, String> uses Database(write) {
                transaction {
                    sql { INSERT INTO invoice_items (quantity, unit_price, total) VALUES (:quantity, :unit_price, :total) }
                }
                return Ok(total)
            }
            fn main() {}
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        check_program(&program, &schema)
            .expect("API-validated SQL may persist its server-computed output parameter");

        let forged = source.replace(":total)", ":quantity)");
        let program = parse(&lex(&forged).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        let errors = check_program(&program, &schema)
            .expect_err("computed output must bind the matching server-computed parameter");
        assert!(errors
            .iter()
            .any(|error| error.message.contains("computed")));
    }

    #[test]
    fn computed_columns_allow_only_api_updates_with_all_inputs_and_server_output() {
        let source = r#"
            table invoice_items {
                id: Id primary auto
                quantity: Int required
                unit_price: Int required
                total: Int required compute(quantity * unit_price)
            }
            api PUT "/invoice-items/{id}" {
                handler update_item
                input { id: Id quantity: Int unit_price: Int }
                validates invoice_items
                output Result<Int, String>
            }
            fn update_item(id: Id, quantity: Int, unit_price: Int, total: Int) -> Result<Int, String> uses Database(write) {
                transaction {
                    sql { UPDATE invoice_items SET quantity = :quantity, unit_price = :unit_price, total = :total WHERE id = :id }
                }
                return Ok(total)
            }
            fn main() {}
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        check_program(&program, &schema)
            .expect("validated API updates may persist recomputed inputs and outputs");

        for unsafe_update in [
            "UPDATE invoice_items SET quantity = :quantity, unit_price = :unit_price, total = 999 WHERE id = :id",
            "UPDATE invoice_items SET quantity = :quantity, total = :total WHERE id = :id",
        ] {
            let unsafe_source = source.replace(
                "UPDATE invoice_items SET quantity = :quantity, unit_price = :unit_price, total = :total WHERE id = :id",
                unsafe_update,
            );
            let program = parse(&lex(&unsafe_source).unwrap()).unwrap();
            let schema = build_schema(&program).unwrap();
            let errors = check_program(&program, &schema)
                .expect_err("updates must bind every computed dependency and result");
            assert!(errors.iter().any(|error| error.message.contains("computed")));
        }
    }

    #[test]
    fn calendar_date_and_time_sql_parameters_use_their_declared_types() {
        let program = parse(
            &lex(
                r#"table dates { id: Id primary day: Date required clock: Time required }
            fn load(day: Date, clock: Time) uses Database {
                rows = sql<Date[]> { SELECT day FROM dates WHERE day = :day AND clock = :clock }
            } fn main() {}"#,
            )
            .unwrap(),
        )
        .unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(check_program(&program, &schema).is_ok());
        for wrong_type in ["String", "Int", "Timestamp", "Time"] {
            let source = format!("table dates {{ id: Id primary day: Date required }} fn load(day: {wrong_type}) uses Database {{ rows = sql<Date[]> {{ SELECT day FROM dates WHERE day = :day }} }} fn main() {{}}");
            let program = parse(&lex(&source).unwrap()).unwrap();
            let schema = build_schema(&program).unwrap();
            assert!(
                check_program(&program, &schema).is_err(),
                "accepted {wrong_type} as Date SQL parameter"
            );
        }
    }

    #[test]
    fn decimal_alias_sql_parameters_keep_their_underlying_numeric_type() {
        let program = parse(&lex(r#"type Amount = Decimal table charges { id: Id primary amount: Decimal required }
            fn load(amount: Amount) uses Database {
                rows = sql<Amount[]> { SELECT amount FROM charges WHERE amount = :amount }
                nested: Amount = amount
                if true { rows2 = sql<Decimal[]> { SELECT amount FROM charges WHERE amount = :nested } }
            } fn main() {}"#).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(check_program(&program, &schema).is_ok());
    }

    #[test]
    fn form_money_and_decimal_alias_parameters_match_exact_storage() {
        let program = parse(&lex(r#"type Amount = Decimal
            table charges { id: Id primary auto price: Money required precise: Amount required }
            form Charge -> charges { fields { price precise }
                action save { sql { UPDATE charges SET price = :price, precise = :precise WHERE price = :price } } }
            fn main() {}"#).unwrap()).unwrap();
        let schema = build_schema(&program).unwrap();
        assert!(check_program(&program, &schema).is_ok());
    }

    #[test]
    fn analyzes_sql_table_reads_and_writes_conservatively() {
        let tables = ["customers", "orders", "audit_events"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let (operation, accesses) = analyze_table_access(
            "-- UPDATE orders SET status = 'x'\nSELECT id FROM customers WHERE email = 'UPDATE orders'",
            &tables,
        );
        assert_eq!(operation, Some("SELECT"));
        assert_eq!(
            accesses,
            vec![SqlTableAccess {
                table: "customers".into(),
                mode: SqlAccessMode::Read
            }]
        );

        let (operation, accesses) = analyze_table_access(
            "INSERT INTO audit_events (id) SELECT id FROM customers",
            &tables,
        );
        assert_eq!(operation, Some("INSERT"));
        assert_eq!(
            accesses,
            vec![
                SqlTableAccess {
                    table: "audit_events".into(),
                    mode: SqlAccessMode::Write
                },
                SqlTableAccess {
                    table: "customers".into(),
                    mode: SqlAccessMode::Read
                }
            ]
        );

        let (operation, accesses) = analyze_table_access(
            "UPDATE customers SET email = 'x' WHERE id IN (SELECT id FROM orders)",
            &tables,
        );
        assert_eq!(operation, Some("UPDATE"));
        assert_eq!(accesses[0].mode, SqlAccessMode::Write);
        assert_eq!(accesses[1].mode, SqlAccessMode::Read);

        let (operation, accesses) = analyze_table_access(
            "DELETE FROM customers WHERE id IN (SELECT id FROM orders)",
            &tables,
        );
        assert_eq!(operation, Some("DELETE"));
        assert_eq!(
            accesses,
            vec![
                SqlTableAccess {
                    table: "customers".into(),
                    mode: SqlAccessMode::Write
                },
                SqlTableAccess {
                    table: "orders".into(),
                    mode: SqlAccessMode::Read
                }
            ]
        );

        let (_, accesses) = analyze_table_access(
            "UPDATE customers SET email = (SELECT email FROM customers)",
            &tables,
        );
        assert_eq!(accesses[0].mode, SqlAccessMode::ReadWrite);

        let (_, accesses) = analyze_table_access(
            "UPDATE customers JOIN orders ON customers.id = orders.id SET customers.email = 'x'",
            &tables,
        );
        assert!(accesses
            .iter()
            .all(|access| access.mode == SqlAccessMode::Unknown));

        let (_, accesses) = analyze_table_access(
            "DELETE customers FROM customers JOIN orders ON customers.id = orders.id",
            &tables,
        );
        assert!(accesses
            .iter()
            .all(|access| access.mode == SqlAccessMode::Unknown));

        let (_, accesses) =
            analyze_table_access("SELECT id FROM billing.invoices", &["invoices".to_owned()]);
        assert_eq!(
            accesses,
            vec![SqlTableAccess {
                table: "invoices".into(),
                mode: SqlAccessMode::Read
            }]
        );

        for (query, table, mode) in [
            ("SELECT id FROM \"invoices\"", "", SqlAccessMode::Unknown),
            ("SELECT id FROM `invoices`", "invoices", SqlAccessMode::Read),
            ("SELECT id FROM [invoices]", "invoices", SqlAccessMode::Read),
        ] {
            let (_, accesses) = analyze_table_access(query, &["invoices".to_owned()]);
            assert_eq!(
                accesses,
                vec![SqlTableAccess {
                    table: table.into(),
                    mode
                }],
                "query: {query}"
            );
        }
    }

    #[test]
    fn accepts_typed_select_and_parameters() {
        let program = parse(&lex(
            "fn load(id: Int) { customer = sql<Customer[]> { SELECT id, name, email FROM customers WHERE id = :id } }",
        )
        .unwrap())
        .unwrap();
        assert!(check_program(&program, &source_schema()).is_ok());
    }

    #[test]
    fn rejects_unknown_column_and_parameter() {
        let program = parse(&lex(
            "fn load() { customer = sql<Customer[]> { SELECT username FROM customers WHERE id = :id } }",
        )
        .unwrap())
        .unwrap();
        let errors = check_program(&program, &source_schema()).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("username")));
        assert!(errors.iter().any(|error| error.message.contains(":id")));
    }

    #[test]
    fn checks_form_action_sql_against_form_fields() {
        let program = parse(&lex(
            "table customers { id: Id primary auto name: String(100) } form CustomerCreate -> customers { fields { name } action save { sql { INSERT INTO customers (username) VALUES (:username) } } }",
        )
        .unwrap())
        .unwrap();
        let errors = check_program(&program, &source_schema()).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("username")));
        assert!(errors
            .iter()
            .any(|error| error.message.contains("not available in this scope")));
    }

    #[test]
    fn checks_crud_action_sql_against_typed_fields_and_route_id() {
        let program = parse(
            &lex("table customers { id: Id primary auto name: String(100) } crud Customer -> customers { action rename { field name: String { required } sql { UPDATE customers SET name = :name WHERE id = :id } } }")
                .unwrap(),
        )
        .unwrap();
        assert!(check_program(&program, &source_schema()).is_ok());

        let invalid_program = parse(
            &lex("table customers { id: Id primary auto name: String(100) } crud Customer -> customers { action rename { field name: String { required } sql { UPDATE customers SET name = :unknown WHERE id = :id } } }")
                .unwrap(),
        )
        .unwrap();
        let errors = check_program(&invalid_program, &source_schema()).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains(":unknown")));
    }

    #[test]
    fn checks_tableview_source_sql_against_schema() {
        let program = parse(&lex(
            "table customers { id: Id primary auto name: String(100) } tableview Customers { source sql<Customer[]> { SELECT id, name FROM customers } columns { id name } }",
        )
        .unwrap())
        .unwrap();
        assert!(check_program(&program, &source_schema()).is_ok());
    }

    #[test]
    fn checks_page_data_sql_against_schema_and_route_parameters() {
        let program = parse(&lex(
            "table customers { id: Id primary auto name: String(100) } page \"/customers/{name}\" { load customer = sql<Customer> { SELECT id, name FROM customers WHERE name = :name } html { <h1>{customer.name}</h1> } }",
        )
        .unwrap())
        .unwrap();
        assert!(check_program(&program, &source_schema()).is_ok());
    }

    #[test]
    fn checks_page_collection_sql_against_schema() {
        let program = parse(&lex(
            "table customers { id: Id primary auto name: String(100) } page \"/customers\" { load customers = sql<Customer[]> { SELECT id, name FROM customers } html { <h1>Customers</h1> } }",
        )
        .unwrap())
        .unwrap();
        assert!(check_program(&program, &source_schema()).is_ok());
    }

    #[test]
    fn checks_page_query_input_parameters_against_schema() {
        let program = parse(&lex(
            "table customers { id: Id primary auto name: String(100) } page \"/customers\" { input { search: String? } load customers = sql<Customer[]> { SELECT id, name FROM customers WHERE name LIKE :search } html { <h1>Customers</h1> } }",
        )
        .unwrap())
        .unwrap();
        assert!(check_program(&program, &source_schema()).is_ok());
    }

    #[test]
    fn rejects_unknown_page_data_column() {
        let program = parse(&lex(
            "table customers { id: Id primary auto name: String(100) } page \"/customers/{name}\" { load customer = sql<Customer> { SELECT username FROM customers WHERE name = :name } html { <h1>{customer.name}</h1> } }",
        )
        .unwrap())
        .unwrap();
        let errors = check_program(&program, &source_schema()).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("username")));
    }

    #[test]
    fn rejects_unknown_tableview_source_column() {
        let program = parse(&lex(
            "table customers { id: Id primary auto name: String(100) } tableview Customers { source sql<Customer[]> { SELECT username FROM customers } columns { id } }",
        )
        .unwrap())
        .unwrap();
        let errors = check_program(&program, &source_schema()).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("username")));
    }

    #[test]
    fn accepts_record_tableview_results() {
        let program = parse(&lex(
            "struct CustomerOverview { id: Id name: String? orders: Int turnover: Int? } tableview Customers { source sql<CustomerOverview[]> { SELECT id, name, COUNT(id) AS orders, SUM(id) AS turnover FROM customers GROUP BY id, name } columns { id name orders turnover } }",
        )
        .unwrap())
        .unwrap();
        let result = check_program(&program, &source_schema());
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn accepts_explicit_hidden_tableview_row_fields_and_requires_them_in_sql() {
        let program = parse(&lex(
            "struct CustomerOverview { id: Id name: String? email: Email? } tableview Customers { source sql<CustomerOverview[]> { SELECT id, name, email FROM customers } columns { name } view { row_fields { id email } } }",
        )
        .unwrap())
        .unwrap();
        let result = check_program(&program, &source_schema());
        assert!(result.is_ok(), "{result:?}");

        let missing = parse(&lex(
            "struct CustomerOverview { id: Id name: String? email: Email? } tableview Customers { source sql<CustomerOverview[]> { SELECT id, name FROM customers } columns { name } view { row_fields { id email } } }",
        )
        .unwrap())
        .unwrap();
        let errors = check_program(&missing, &source_schema()).unwrap_err();
        assert!(errors.iter().any(|error| {
            error.message.contains("row field `email`")
                && error.message.contains("missing from the SQL projection")
        }));
    }

    #[test]
    fn rejects_missing_record_tableview_projection_field() {
        let program = parse(&lex(
            "struct CustomerOverview { id: Id name: String? orders: Int } tableview Customers { source sql<CustomerOverview[]> { SELECT id, name FROM customers } columns { id name orders } }",
        )
        .unwrap())
        .unwrap();
        let errors = check_program(&program, &source_schema()).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("orders") && error.message.contains("missing")));
    }

    #[test]
    fn rejects_incompatible_record_tableview_projection_type() {
        let program = parse(&lex(
            "struct CustomerOverview { id: Id name: String? orders: String } tableview Customers { source sql<CustomerOverview[]> { SELECT id, name, COUNT(id) AS orders FROM customers } columns { id name orders } }",
        )
        .unwrap())
        .unwrap();
        let errors = check_program(&program, &source_schema()).unwrap_err();
        assert!(errors.iter().any(|error| error
            .message
            .contains("tableview result field `orders` expects `String`, but SQL produces `Int`")));
    }
}
