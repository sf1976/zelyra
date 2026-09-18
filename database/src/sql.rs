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

pub fn check_program(program: &zelyra_ast::Program, schema: &Schema) -> Result<(), Vec<SqlError>> {
    let mut errors = Vec::new();
    for function in &program.functions {
        let mut environment = function
            .params
            .iter()
            .map(|parameter| (parameter.name.clone(), parameter.ty.clone()))
            .collect::<HashMap<_, _>>();
        check_block(&function.body, schema, &mut environment, &mut errors);
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
                environment.insert(field.name.clone(), field_type.unwrap_or(Type::Unknown));
            }
            let block = Block {
                statements: action.statements.clone(),
                span: action.span,
            };
            check_block(&block, schema, &mut environment, &mut errors);
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
            check_block(&block, schema, &mut environment, &mut errors);
        }
    }
    for page in &program.pages {
        let environment = page
            .path
            .split('/')
            .filter_map(|segment| {
                segment
                    .strip_prefix('{')
                    .and_then(|segment| segment.strip_suffix('}'))
                    .map(|name| (name.to_owned(), Type::String))
            })
            .collect::<HashMap<_, _>>();
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
                        &record
                            .fields
                            .iter()
                            .map(|field| field.name.clone())
                            .collect::<Vec<_>>(),
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
            &tableview.columns,
            tableview.span,
        ));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn check_block(
    block: &Block,
    schema: &Schema,
    environment: &mut HashMap<String, Type>,
    errors: &mut Vec<SqlError>,
) {
    for statement in &block.statements {
        match statement {
            Stmt::Let {
                name, ty, value, ..
            } => {
                check_expr(value, schema, environment, errors);
                environment.insert(name.clone(), ty.clone().unwrap_or_else(|| expr_type(value)));
            }
            Stmt::BindOrAssign { name, value, .. } => {
                check_expr(value, schema, environment, errors);
                environment.insert(name.clone(), expr_type(value));
            }
            Stmt::Expr(expression) => check_expr(expression, schema, environment, errors),
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    check_expr(value, schema, environment, errors);
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                check_expr(condition, schema, environment, errors);
                check_block(then_block, schema, environment, errors);
                if let Some(else_block) = else_block {
                    check_block(else_block, schema, environment, errors);
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                check_expr(condition, schema, environment, errors);
                for invariant in invariants {
                    check_expr(invariant, schema, environment, errors);
                }
                check_block(body, schema, environment, errors);
            }
            Stmt::For { iterable, body, .. } => {
                check_expr(iterable, schema, environment, errors);
                check_block(body, schema, environment, errors);
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    check_expr(invariant, schema, environment, errors);
                }
                check_block(body, schema, environment, errors);
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::Match { value, arms, .. } => {
                check_expr(value, schema, environment, errors);
                for arm in arms {
                    check_block(&arm.body, schema, environment, errors);
                }
            }
            Stmt::Transaction { body, .. } => {
                check_block(body, schema, environment, errors);
            }
            Stmt::Parallel { body, .. } => {
                check_block(body, schema, environment, errors);
            }
        }
    }
}

fn check_expr(
    expression: &Expr,
    schema: &Schema,
    environment: &HashMap<String, Type>,
    errors: &mut Vec<SqlError>,
) {
    match &expression.kind {
        ExprKind::Sql {
            result_type, query, ..
        } => errors.extend(check_query(
            query,
            result_type,
            schema,
            environment,
            expression.span,
        )),
        ExprKind::Call { args, .. } => {
            for argument in args {
                check_expr(argument, schema, environment, errors);
            }
        }
        ExprKind::Unary { expr, .. } => check_expr(expr, schema, environment, errors),
        ExprKind::Binary { left, right, .. } => {
            check_expr(left, schema, environment, errors);
            check_expr(right, schema, environment, errors);
        }
        ExprKind::Array(values) => {
            for value in values {
                check_expr(value, schema, environment, errors);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                check_expr(key, schema, environment, errors);
                check_expr(value, schema, environment, errors);
            }
        }
        ExprKind::Record { fields, .. } => {
            for (_, value) in fields {
                check_expr(value, schema, environment, errors);
            }
        }
        ExprKind::Index { target, index } => {
            check_expr(target, schema, environment, errors);
            check_expr(index, schema, environment, errors);
        }
        ExprKind::Field { target, .. } => check_expr(target, schema, environment, errors),
        ExprKind::Await(inner) => check_expr(inner, schema, environment, errors),
        ExprKind::Int(_)
        | ExprKind::UInt(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::String(_)
        | ExprKind::Char(_)
        | ExprKind::Variable(_) => {}
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
        ExprKind::Field { .. } => Type::Unknown,
        ExprKind::Sql { result_type, .. } => result_type.clone(),
        ExprKind::Variable(_)
        | ExprKind::Call { .. }
        | ExprKind::Unary { .. }
        | ExprKind::Binary { .. } => Type::Unknown,
        ExprKind::Await(inner) => expr_type(inner),
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

fn check_tableview_query(
    query: &str,
    result_type: &Type,
    schema: &Schema,
    environment: &HashMap<String, Type>,
    records: &[zelyra_ast::RecordDef],
    columns: &[String],
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
                    query, record, columns, schema, span,
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
            .any(|column| column.eq_ignore_ascii_case(&normalized))
        {
            errors.push(sql_error(
                span,
                format!("tableview result alias `{name}` is not declared as a view column"),
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
                "SUM" | "AVG" | "MIN" | "MAX" => {
                    let inner = expression.get(2..expression.len().saturating_sub(1));
                    let (ty, _) = inner
                        .map(|inner| projection_type(inner, tables, schema))
                        .unwrap_or((Type::Unknown, true));
                    (ty, true)
                }
                "COALESCE" | "IFNULL" => {
                    let inner = expression.get(2..expression.len().saturating_sub(1));
                    inner
                        .and_then(|inner| {
                            inner
                                .split(|token| matches!(token, SqlToken::Comma))
                                .next()
                                .map(|part| projection_type(part, tables, schema))
                        })
                        .unwrap_or((Type::Unknown, false))
                }
                _ => (Type::Unknown, true),
            };
        }
    }
    if let Some(column) = projection_source_column(expression, tables, schema) {
        return (sql_type_to_type(&column.sql_type), column.nullable);
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
            matches!(name.as_str(), "Email" | "Url" | "Uuid")
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
    if upper.contains("CHAR") || upper.contains("TEXT") || upper.contains("CLOB") {
        Type::String
    } else if upper.contains("BOOL") {
        Type::Bool
    } else if upper.contains("REAL") || upper.contains("DOUBLE") || upper.contains("FLOAT") {
        Type::Float
    } else if upper.contains("DECIMAL") || upper.contains("NUMERIC") {
        Type::Decimal
    } else if upper.contains("DATE") || upper.contains("TIME") {
        Type::Timestamp
    } else {
        Type::Int
    }
}

fn sql_types_compatible(expected: &Type, actual: &Type) -> bool {
    if matches!(actual, Type::Unknown) || expected == actual {
        return true;
    }
    match actual {
        Type::Named(name) if expected == &Type::Int => {
            name.to_ascii_lowercase().ends_with("id")
                || !matches!(name.as_str(), "Email" | "Url" | "Uuid" | "Money")
        }
        Type::Named(name) if expected == &Type::String => {
            matches!(name.as_str(), "Email" | "Url" | "Uuid")
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
