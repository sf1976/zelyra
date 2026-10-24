use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use zelyra_ast::{Block, Expr, ExprKind, Program, Span, Stmt};
use zelyra_lexer::lex;
use zelyra_parser::parse;

#[derive(Debug, Clone, PartialEq, Eq)]
struct FilePlan {
    path: String,
    rule: &'static str,
    proposed_directory: String,
    status: &'static str,
    reason: String,
    span: Option<Span>,
    declarations: Vec<DeclarationPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeclarationPlan {
    kind: &'static str,
    name: String,
    proposed_directory: String,
    status: &'static str,
    rule: &'static str,
    reason: String,
    span: Span,
}

#[derive(Debug)]
struct MigrationPlan {
    schema_version: u32,
    mode: &'static str,
    files: Vec<FilePlan>,
    summary: Summary,
}

#[derive(Debug)]
struct Summary {
    classified: usize,
    manual_review: usize,
    unchanged_entrypoints: usize,
}

#[derive(Debug, Clone)]
struct RouteCall {
    name: String,
    args: Vec<Expr>,
    callback_targets: std::collections::HashMap<String, std::collections::HashSet<String>>,
    span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ArchitectureIssue {
    pub(super) path: String,
    pub(super) code: &'static str,
    pub(super) message: String,
    pub(super) span: Option<Span>,
}

pub(super) fn validate_project_modules(
    entry_path: &str,
    modules: &[crate::project::ProjectModule],
    program: &Program,
    sources: &[crate::project::ProjectSource],
) -> Result<(), Vec<ArchitectureIssue>> {
    let entry = Path::new(entry_path);
    let root = entry
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let strict = crate::project_layout::strict_architecture_enabled(root).map_err(|error| {
        vec![ArchitectureIssue {
            path: error.path,
            code: error.code,
            message: error.message,
            span: None,
        }]
    })?;
    if !strict {
        return Ok(());
    }

    let mut issues = Vec::new();
    for module in modules {
        let path = module.path.replace('\\', "/");
        if path == "main.zyl" || path.starts_with("<zelyra-") || path.starts_with("packages/") {
            continue;
        }
        let mut components = Path::new(&path).components();
        let Some(directory) = components.next().and_then(|part| part.as_os_str().to_str()) else {
            continue;
        };
        let Some(allowed) = architecture_declarations(directory) else {
            issues.push(ArchitectureIssue {
                path: module.path.clone(),
                code: "E-ARCH-001",
                message: "strict architecture requires project modules under `schema/`, `domain/`, or `routes/`".into(),
                span: None,
            });
            continue;
        };
        let declaration_spans = sources
            .iter()
            .find(|source| source.path.replace('\\', "/") == path)
            .and_then(|source| {
                declaration_spans(&source.text, source_id_for_path(sources, &path)?)
            });
        if path.starts_with("domain/") {
            let import_spans = sources
                .iter()
                .find(|source| source.path.replace('\\', "/") == path)
                .and_then(|source| {
                    source_import_spans(&source.text, source_id_for_path(sources, &path)?)
                });
            for import in module
                .imports
                .iter()
                .filter(|import| import.path.replace('\\', "/").starts_with("routes/"))
            {
                issues.push(ArchitectureIssue {
                    path: module.path.clone(),
                    code: "E-ARCH-004",
                    message: format!(
                        "domain module must not import route module `{}`; pass domain values to route handlers instead",
                        import.alias
                    ),
                    span: import_spans
                        .as_ref()
                        .and_then(|spans| spans.get(&import.alias).copied()),
                });
            }
        }
        for declaration in &module.declarations {
            let kind = declaration
                .split_once(':')
                .map_or(declaration.as_str(), |(kind, _)| kind);
            if !allowed.contains(&kind) {
                issues.push(ArchitectureIssue {
                    path: module.path.clone(),
                    code: "E-ARCH-001",
                    message: format!(
                        "`{kind}` declarations are not allowed under `{directory}/`; move this declaration to its matching project area"
                    ),
                    span: declaration_spans
                        .as_ref()
                        .and_then(|spans| spans.get(declaration).copied()),
                });
            }
        }
    }
    issues.extend(route_handler_sql_issues(program, sources));
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

fn source_id_for_path(sources: &[crate::project::ProjectSource], path: &str) -> Option<u32> {
    sources
        .iter()
        .position(|source| source.path.replace('\\', "/") == path)
        .and_then(|index| u32::try_from(index).ok())
}

fn declaration_spans(
    source: &str,
    source_id: u32,
) -> Option<std::collections::HashMap<String, Span>> {
    let tokens = lex(source).ok()?;
    let program = parse(&tokens).ok()?;
    let mut declarations = std::collections::HashMap::new();
    macro_rules! add {
        ($items:expr, $kind:literal) => {
            for item in $items {
                declarations.insert(
                    format!("{}:{}", $kind, item.name),
                    item.span.with_source_id(source_id),
                );
            }
        };
    }
    add!(program.databases, "database");
    add!(program.tables, "table");
    add!(program.tableviews, "tableview");
    add!(program.types, "type");
    add!(program.enums, "enum");
    add!(program.records, "record");
    add!(program.views, "view");
    add!(program.components, "component");
    for page in program.pages {
        declarations.insert(
            format!("page:{}", page.path),
            page.span.with_source_id(source_id),
        );
    }
    add!(program.forms, "form");
    add!(program.cruds, "crud");
    add!(program.auth, "auth");
    add!(program.commands, "command");
    add!(program.schedules, "schedule");
    add!(program.functions, "function");
    for api in program.apis {
        declarations.insert(
            format!("api:{} {}", api.method, api.path),
            api.span.with_source_id(source_id),
        );
    }
    Some(declarations)
}

fn source_import_spans(
    source: &str,
    source_id: u32,
) -> Option<std::collections::HashMap<String, Span>> {
    let tokens = lex(source).ok()?;
    let program = parse(&tokens).ok()?;
    Some(
        program
            .imports
            .into_iter()
            .map(|import| (import.alias, import.span.with_source_id(source_id)))
            .collect(),
    )
}

fn route_handler_sql_issues(
    program: &Program,
    sources: &[crate::project::ProjectSource],
) -> Vec<ArchitectureIssue> {
    let functions = program
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function))
        .collect::<std::collections::HashMap<_, _>>();
    let mut issues = Vec::new();

    for api in &program.apis {
        let Some(route_source) = sources.get(api.span.source_id as usize) else {
            continue;
        };
        if !route_source.path.replace('\\', "/").starts_with("routes/") {
            continue;
        }
        let Some(handler) = api.handler.as_deref() else {
            continue;
        };
        issues.extend(reachable_route_sql_issues(
            &functions,
            sources,
            vec![RouteCall {
                name: handler.to_owned(),
                args: Vec::new(),
                callback_targets: std::collections::HashMap::new(),
                span: api.span,
            }],
            Vec::new(),
            route_source.path.clone(),
            format!("API handler `{handler}`"),
        ));
    }

    for form in &program.forms {
        let Some(source) = sources.get(form.span.source_id as usize) else {
            continue;
        };
        if !source.path.replace('\\', "/").starts_with("routes/") {
            continue;
        }
        for action in &form.actions {
            let (calls, sql) = inspect_action(action, &functions);
            issues.extend(reachable_route_sql_issues(
                &functions,
                sources,
                calls,
                sql,
                source.path.clone(),
                format!("form action `{}.{}`", form.name, action.name),
            ));
        }
    }

    for crud in &program.cruds {
        let Some(source) = sources.get(crud.span.source_id as usize) else {
            continue;
        };
        if !source.path.replace('\\', "/").starts_with("routes/") {
            continue;
        }
        for action in &crud.actions {
            let (calls, sql) = inspect_action(action, &functions);
            issues.extend(reachable_route_sql_issues(
                &functions,
                sources,
                calls,
                sql,
                source.path.clone(),
                format!("CRUD action `{}.{}`", crud.name, action.name),
            ));
        }
    }
    issues.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| {
                left.span
                    .map(|span| span.start)
                    .cmp(&right.span.map(|span| span.start))
            })
            .then_with(|| left.code.cmp(right.code))
    });
    issues.dedup_by(|left, right| {
        left.path == right.path && left.code == right.code && left.span == right.span
    });
    issues
}

fn inspect_action(
    action: &zelyra_ast::FormAction,
    functions: &std::collections::HashMap<&str, &zelyra_ast::Function>,
) -> (Vec<RouteCall>, Vec<(String, Span)>) {
    let mut calls = Vec::new();
    let mut sql = Vec::new();
    let block = Block {
        statements: action.statements.clone(),
        span: action.span,
    };
    inspect_block(&block, &mut calls, &mut sql);
    if let Some(body) = &action.execute_body {
        inspect_block(body, &mut calls, &mut sql);
    }
    if let Some(guard) = &action.guard {
        inspect_expr(&guard.condition, &mut calls, &mut sql);
    }
    let mut aliases = std::collections::HashMap::new();
    collect_function_aliases(&block, &mut aliases);
    if let Some(body) = &action.execute_body {
        collect_function_aliases(body, &mut aliases);
    }
    (expand_route_calls(calls, &aliases, functions), sql)
}

fn reachable_route_sql_issues(
    functions: &std::collections::HashMap<&str, &zelyra_ast::Function>,
    sources: &[crate::project::ProjectSource],
    initial_calls: Vec<RouteCall>,
    initial_sql: Vec<(String, Span)>,
    initial_path: String,
    entry_label: String,
) -> Vec<ArchitectureIssue> {
    let mut issues = Vec::new();
    append_sql_issues(&mut issues, &initial_path, &entry_label, initial_sql);
    let mut pending = initial_calls;
    let mut visited = std::collections::HashSet::new();
    while let Some(call) = pending.pop() {
        let visit_key = (
            call.name.clone(),
            callback_targets_key(&call.callback_targets),
        );
        if !visited.insert(visit_key) {
            continue;
        }
        let Some(function) = functions.get(call.name.as_str()).copied() else {
            continue;
        };
        let Some(function_source) = sources.get(function.span.source_id as usize) else {
            continue;
        };
        if function_source
            .path
            .replace('\\', "/")
            .starts_with("domain/")
        {
            // Business writes in domain modules are the intended separation.
            continue;
        }

        let mut calls = Vec::new();
        let mut sql = Vec::new();
        let mut aliases = call.callback_targets;
        collect_function_aliases(&function.body, &mut aliases);
        let mut function_valued_locals = std::collections::HashSet::new();
        collect_typed_function_locals(&function.body, &mut function_valued_locals);
        inspect_block(&function.body, &mut calls, &mut sql);
        for next_call in calls {
            if let Some(targets) = aliases.get(&next_call.name) {
                for target in targets {
                    if let Some(callee) = functions.get(target.as_str()) {
                        pending.push(RouteCall {
                            name: target.clone(),
                            callback_targets: bind_callback_arguments(&next_call, callee, &aliases),
                            args: next_call.args.clone(),
                            span: next_call.span,
                        });
                    }
                }
                continue;
            }
            if function.params.iter().any(|parameter| {
                parameter.name == next_call.name && may_write_database(&parameter.ty)
            }) || function_valued_locals.contains(&next_call.name)
            {
                issues.push(ArchitectureIssue {
                    path: function_source.path.clone(),
                    code: "E-ARCH-005",
                    message: format!(
                        "{entry_label} reaches an indirect function call `{}` whose target cannot be resolved; pass a statically named function or move the call into a `domain/` function",
                        next_call.name
                    ),
                    span: Some(next_call.span),
                });
                continue;
            }
            if let Some(callee) = functions.get(next_call.name.as_str()) {
                let callback_targets = bind_callback_arguments(&next_call, callee, &aliases);
                pending.push(RouteCall {
                    name: next_call.name,
                    args: next_call.args,
                    callback_targets,
                    span: next_call.span,
                });
            }
        }
        append_sql_issues(&mut issues, &function_source.path, &entry_label, sql);
    }
    issues
}

impl RouteCall {
    fn new(name: String, args: Vec<Expr>, span: Span) -> Self {
        Self {
            name,
            args,
            callback_targets: std::collections::HashMap::new(),
            span,
        }
    }
}

/// Resolve statically known local function values used as call names.
///
/// If a local is assigned more than one named function value in different
/// branches, every known target is checked. Unknown targets are rejected at
/// the callback call site while traversing the reachable function.
fn expand_route_calls(
    calls: Vec<RouteCall>,
    aliases: &std::collections::HashMap<String, std::collections::HashSet<String>>,
    functions: &std::collections::HashMap<&str, &zelyra_ast::Function>,
) -> Vec<RouteCall> {
    let mut expanded = Vec::new();
    for call in calls {
        let names = aliases
            .get(&call.name)
            .cloned()
            .unwrap_or_else(|| std::collections::HashSet::from([call.name.clone()]));
        for name in names {
            if let Some(callee) = functions.get(name.as_str()) {
                expanded.push(RouteCall {
                    callback_targets: bind_callback_arguments(&call, callee, aliases),
                    name,
                    args: call.args.clone(),
                    span: call.span,
                });
            }
        }
    }
    expanded
}

fn collect_typed_function_locals(block: &Block, names: &mut std::collections::HashSet<String>) {
    for statement in &block.statements {
        match statement {
            Stmt::Let {
                name, ty: Some(ty), ..
            } if may_write_database(ty) => {
                names.insert(name.clone());
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_typed_function_locals(then_block, names);
                if let Some(block) = else_block {
                    collect_typed_function_locals(block, names);
                }
            }
            Stmt::While { body, .. }
            | Stmt::For { body, .. }
            | Stmt::Loop { body, .. }
            | Stmt::Transaction { body, .. }
            | Stmt::Parallel { body, .. } => collect_typed_function_locals(body, names),
            Stmt::Match { arms, .. } => {
                for arm in arms {
                    collect_typed_function_locals(&arm.body, names);
                }
            }
            Stmt::Let { .. }
            | Stmt::BindOrAssign { .. }
            | Stmt::ArrayIndexAssign { .. }
            | Stmt::TupleBindOrAssign { .. }
            | Stmt::Expr(_)
            | Stmt::Return { .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. } => {}
        }
    }
}

fn may_write_database(ty: &zelyra_ast::Type) -> bool {
    matches!(
        ty,
        zelyra_ast::Type::Function { capabilities, .. }
            if capabilities.iter().any(|capability| capability == "Database")
    )
}

fn callback_targets_key(
    targets: &std::collections::HashMap<String, std::collections::HashSet<String>>,
) -> Vec<(String, Vec<String>)> {
    let mut key = targets
        .iter()
        .map(|(name, values)| {
            let mut values = values.iter().cloned().collect::<Vec<_>>();
            values.sort();
            (name.clone(), values)
        })
        .collect::<Vec<_>>();
    key.sort_by(|left, right| left.0.cmp(&right.0));
    key
}

fn bind_callback_arguments(
    call: &RouteCall,
    callee: &zelyra_ast::Function,
    aliases: &std::collections::HashMap<String, std::collections::HashSet<String>>,
) -> std::collections::HashMap<String, std::collections::HashSet<String>> {
    let mut targets = std::collections::HashMap::new();
    for (parameter, argument) in callee.params.iter().zip(&call.args) {
        if !matches!(parameter.ty, zelyra_ast::Type::Function { .. }) {
            continue;
        }
        match &argument.kind {
            ExprKind::FunctionValue(target) => {
                targets
                    .entry(parameter.name.clone())
                    .or_insert_with(std::collections::HashSet::new)
                    .insert(target.clone());
            }
            ExprKind::Variable(source) => {
                if let Some(values) = aliases.get(source) {
                    targets.insert(parameter.name.clone(), values.clone());
                }
            }
            _ => {}
        }
    }
    targets
}

fn collect_function_aliases(
    block: &Block,
    aliases: &mut std::collections::HashMap<String, std::collections::HashSet<String>>,
) {
    for statement in &block.statements {
        match statement {
            Stmt::Let { name, value, .. } | Stmt::BindOrAssign { name, value, .. } => {
                if let ExprKind::FunctionValue(target) = &value.kind {
                    aliases
                        .entry(name.clone())
                        .or_default()
                        .insert(target.clone());
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_function_aliases(then_block, aliases);
                if let Some(block) = else_block {
                    collect_function_aliases(block, aliases);
                }
            }
            Stmt::While { body, .. }
            | Stmt::For { body, .. }
            | Stmt::Loop { body, .. }
            | Stmt::Transaction { body, .. }
            | Stmt::Parallel { body, .. } => collect_function_aliases(body, aliases),
            Stmt::Match { arms, .. } => {
                for arm in arms {
                    collect_function_aliases(&arm.body, aliases);
                }
            }
            Stmt::ArrayIndexAssign { .. }
            | Stmt::TupleBindOrAssign { .. }
            | Stmt::Expr(_)
            | Stmt::Return { .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. } => {}
        }
    }
}

fn append_sql_issues(
    issues: &mut Vec<ArchitectureIssue>,
    path: &str,
    entry_label: &str,
    sql: Vec<(String, Span)>,
) {
    for (query, span) in sql {
        match sql_effect(&query) {
            SqlEffect::Write => issues.push(ArchitectureIssue {
                path: path.to_owned(),
                code: "E-ARCH-002",
                message: format!(
                    "{entry_label} reaches a write SQL statement; move the business write into a `domain/` function"
                ),
                span: Some(span),
            }),
            SqlEffect::Unknown => issues.push(ArchitectureIssue {
                path: path.to_owned(),
                code: "E-ARCH-003",
                message: format!(
                    "{entry_label} reaches SQL that cannot be classified safely; move it into a `domain/` function"
                ),
                span: Some(span),
            }),
            SqlEffect::Read => {}
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SqlEffect {
    Read,
    Write,
    Unknown,
}

fn sql_effect(query: &str) -> SqlEffect {
    let Some(keyword) = sql_main_keyword(query) else {
        return SqlEffect::Unknown;
    };
    match keyword.as_str() {
        "INSERT" | "UPDATE" | "DELETE" | "REPLACE" | "MERGE" | "CREATE" | "ALTER" | "DROP"
        | "TRUNCATE" | "GRANT" | "REVOKE" => SqlEffect::Write,
        "SELECT" | "WITH" | "SHOW" | "DESCRIBE" | "DESC" | "EXPLAIN" | "VALUES"
            if crate::database_cli::read_only_statement(query, false)
                && crate::database_cli::read_only_statement(query, true) =>
        {
            SqlEffect::Read
        }
        _ => SqlEffect::Unknown,
    }
}

fn sql_main_keyword(query: &str) -> Option<String> {
    let bytes = query.as_bytes();
    let mut index = 0;
    let mut depth = 0usize;
    let mut first = None;
    while index < bytes.len() {
        match bytes[index] {
            byte if byte.is_ascii_whitespace() => index += 1,
            b'-' if bytes.get(index + 1) == Some(&b'-') => {
                index += 2;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'#' => {
                index += 1;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                if bytes.get(index + 2) == Some(&b'!') {
                    return None;
                }
                index += 2;
                while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/')
                {
                    index += 1;
                }
                if index + 1 >= bytes.len() {
                    return None;
                }
                index += 2;
            }
            b'\'' | b'"' | b'`' => {
                let quote = bytes[index];
                index += 1;
                let mut closed = false;
                while index < bytes.len() {
                    if bytes[index] == quote {
                        if bytes.get(index + 1) == Some(&quote) {
                            index += 2;
                        } else {
                            index += 1;
                            closed = true;
                            break;
                        }
                    } else if bytes[index] == b'\\' && quote != b'`' {
                        index = (index + 2).min(bytes.len());
                    } else {
                        index += 1;
                    }
                }
                if !closed {
                    return None;
                }
            }
            b'(' => {
                depth += 1;
                index += 1;
            }
            b')' => {
                depth = depth.saturating_sub(1);
                index += 1;
            }
            byte if byte.is_ascii_alphabetic() || byte == b'_' => {
                let start = index;
                index += 1;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
                if depth == 0 {
                    let word = std::str::from_utf8(&bytes[start..index])
                        .ok()?
                        .to_ascii_uppercase();
                    if first.is_none() {
                        first = Some(word.clone());
                        if word != "WITH" {
                            return Some(word);
                        }
                    } else if matches!(
                        word.as_str(),
                        "SELECT" | "INSERT" | "UPDATE" | "DELETE" | "REPLACE" | "MERGE"
                    ) {
                        return Some(word);
                    }
                }
            }
            _ => index += 1,
        }
    }
    first.filter(|keyword| keyword != "WITH")
}

fn inspect_block(block: &Block, calls: &mut Vec<RouteCall>, sql: &mut Vec<(String, Span)>) {
    for statement in &block.statements {
        match statement {
            Stmt::Let { value, .. }
            | Stmt::BindOrAssign { value, .. }
            | Stmt::TupleBindOrAssign { value, .. }
            | Stmt::Expr(value)
            | Stmt::Return {
                value: Some(value), ..
            } => inspect_expr(value, calls, sql),
            Stmt::ArrayIndexAssign { index, value, .. } => {
                inspect_expr(index, calls, sql);
                inspect_expr(value, calls, sql);
            }
            Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                inspect_expr(condition, calls, sql);
                inspect_block(then_block, calls, sql);
                if let Some(block) = else_block {
                    inspect_block(block, calls, sql);
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                inspect_expr(condition, calls, sql);
                for invariant in invariants {
                    inspect_expr(invariant, calls, sql);
                }
                inspect_block(body, calls, sql);
            }
            Stmt::For { iterable, body, .. } => {
                inspect_expr(iterable, calls, sql);
                inspect_block(body, calls, sql);
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    inspect_expr(invariant, calls, sql);
                }
                inspect_block(body, calls, sql);
            }
            Stmt::Match { value, arms, .. } => {
                inspect_expr(value, calls, sql);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        inspect_expr(guard, calls, sql);
                    }
                    inspect_block(&arm.body, calls, sql);
                }
            }
            Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
                inspect_block(body, calls, sql)
            }
        }
    }
}

fn inspect_expr(expression: &Expr, calls: &mut Vec<RouteCall>, sql: &mut Vec<(String, Span)>) {
    match &expression.kind {
        ExprKind::Array(items) => {
            for item in items {
                inspect_expr(item, calls, sql);
            }
        }
        ExprKind::Map(items) => {
            for (key, value) in items {
                inspect_expr(key, calls, sql);
                inspect_expr(value, calls, sql);
            }
        }
        ExprKind::Record { fields, .. } => {
            for (_, value) in fields {
                inspect_expr(value, calls, sql);
            }
        }
        ExprKind::Lambda { body, .. } => inspect_expr(body, calls, sql),
        ExprKind::Index { target, index } => {
            inspect_expr(target, calls, sql);
            inspect_expr(index, calls, sql);
        }
        ExprKind::Field { target, .. }
        | ExprKind::Unary { expr: target, .. }
        | ExprKind::Await(target) => inspect_expr(target, calls, sql),
        ExprKind::Call { name, args, .. } => {
            calls.push(RouteCall::new(name.clone(), args.clone(), expression.span));
            for argument in args {
                inspect_expr(argument, calls, sql);
            }
        }
        ExprKind::Binary { left, right, .. } => {
            inspect_expr(left, calls, sql);
            inspect_expr(right, calls, sql);
        }
        ExprKind::Sql { query, .. } => sql.push((query.clone(), expression.span)),
        ExprKind::Match { value, arms } => {
            inspect_expr(value, calls, sql);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    inspect_expr(guard, calls, sql);
                }
                inspect_expr(&arm.value, calls, sql);
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

fn architecture_declarations(directory: &str) -> Option<&'static [&'static str]> {
    match directory {
        "schema" => Some(&["database", "table", "type", "record", "enum"]),
        "domain" => Some(&["type", "record", "enum", "function", "command", "schedule"]),
        "routes" => Some(&[
            "page",
            "api",
            "tableview",
            "form",
            "crud",
            "auth",
            "view",
            "component",
            "function",
        ]),
        _ => None,
    }
}

pub(super) fn migrate_layout(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    let Some(first) = arguments.next() else {
        eprintln!("usage: zelyra migrate-layout --check [entry.zyl] [--format human|json]");
        return ExitCode::from(2);
    };
    if first != "--check" {
        eprintln!("error[E-LAYOUT-MIGRATE-001]: only the read-only --check mode is supported");
        return ExitCode::from(2);
    }
    let mut entry = "main.zyl".to_owned();
    let mut format = "human";
    let mut entry_set = false;
    while let Some(argument) = arguments.next() {
        if argument == "--format" {
            let Some(value) = arguments.next() else {
                eprintln!("error[E-LAYOUT-MIGRATE-001]: --format requires human or json");
                return ExitCode::from(2);
            };
            if value != "human" && value != "json" {
                eprintln!("error[E-LAYOUT-MIGRATE-001]: --format must be human or json");
                return ExitCode::from(2);
            }
            format = if value == "json" { "json" } else { "human" };
        } else if !argument.starts_with('-') && !entry_set {
            entry = argument;
            entry_set = true;
        } else {
            eprintln!("error[E-LAYOUT-MIGRATE-001]: unexpected argument `{argument}`");
            return ExitCode::from(2);
        }
    }

    match build_plan(Path::new(&entry)) {
        Ok(plan) => {
            if format == "json" {
                let json = serde_json::json!({
                    "schema_version": plan.schema_version,
                    "mode": plan.mode,
                    "files": plan.files.iter().map(|file| serde_json::json!({
                        "path": file.path,
                        "rule": file.rule,
                        "proposed_directory": file.proposed_directory,
                        "status": file.status,
                        "reason": file.reason,
                        "span": file.span.map(|span| serde_json::json!({
                            "start": span.start,
                            "end": span.end,
                            "line": span.line,
                            "column": span.column,
                        })),
                        "declarations": file.declarations.iter().map(|declaration| serde_json::json!({
                            "kind": declaration.kind,
                            "name": declaration.name,
                            "proposed_directory": declaration.proposed_directory,
                            "status": declaration.status,
                            "rule": declaration.rule,
                            "reason": declaration.reason,
                            "span": {
                                "start": declaration.span.start,
                                "end": declaration.span.end,
                                "line": declaration.span.line,
                                "column": declaration.span.column,
                            },
                        })).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                    "summary": {
                        "classified": plan.summary.classified,
                        "manual_review": plan.summary.manual_review,
                        "unchanged_entrypoints": plan.summary.unchanged_entrypoints,
                    },
                });
                match serde_json::to_string_pretty(&json) {
                    Ok(json) => println!("{json}"),
                    Err(error) => {
                        eprintln!("error[E-LAYOUT-MIGRATE-002]: cannot encode report: {error}");
                        return ExitCode::from(1);
                    }
                }
            } else {
                print_human(&plan);
            }
            if plan.summary.manual_review > 0 {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("error[E-LAYOUT-MIGRATE-002]: {error}");
            ExitCode::from(1)
        }
    }
}

fn build_plan(entry: &Path) -> Result<MigrationPlan, String> {
    let canonical_entry = fs::canonicalize(entry)
        .map_err(|error| format!("cannot resolve entry `{}`: {error}", entry.display()))?;
    if !canonical_entry.is_file() || canonical_entry.extension().is_none_or(|ext| ext != "zyl") {
        return Err("entry must be an existing .zyl file".into());
    }
    let root = canonical_entry
        .parent()
        .ok_or_else(|| "entry has no project directory".to_owned())?;
    let mut paths = Vec::new();
    collect_sources(root, root, &mut paths)?;
    paths.sort_by_key(|path| path.strip_prefix(root).unwrap_or(path).to_path_buf());

    let mut files = Vec::new();
    let mut manual_review = 0;
    let mut unchanged_entrypoints = 0;
    for path in paths {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| "source path escaped the project root".to_owned())?;
        let display = relative.to_string_lossy().replace('\\', "/");
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?;
        let parsed = lex(&source)
            .map_err(|error| (error.message, error.span))
            .and_then(|tokens| parse(&tokens).map_err(|error| (error.message, error.span)));
        let program = match parsed {
            Ok(program) => program,
            Err((error, _span)) if path == canonical_entry => {
                return Err(format!("{display}: {error}"));
            }
            Err((error, span)) => {
                manual_review += 1;
                files.push(FilePlan {
                    path: display,
                    rule: "LAYOUT-PARSE-001",
                    proposed_directory: "(manual decision)".into(),
                    status: "needs-manual-review",
                    reason: format!("source does not parse: {error}"),
                    span: Some(span),
                    declarations: Vec::new(),
                });
                continue;
            }
        };
        if path == canonical_entry {
            unchanged_entrypoints += 1;
            files.push(FilePlan {
                path: display,
                rule: "LAYOUT-ENTRY-001",
                proposed_directory: "(project root)".into(),
                status: "unchanged-entrypoint",
                reason: "the selected entry file remains at the project root".into(),
                span: None,
                declarations: Vec::new(),
            });
            continue;
        }
        if let Some(directory) = standard_directory(relative) {
            let declarations = declaration_plans(&program, Some(directory));
            let mismatched = declarations
                .iter()
                .any(|declaration| declaration.status == "needs-manual-review");
            if mismatched {
                manual_review += 1;
            }
            files.push(FilePlan {
                path: display,
                rule: if mismatched {
                    "LAYOUT-PLACEMENT-002"
                } else {
                    "LAYOUT-PLACEMENT-001"
                },
                proposed_directory: directory.into(),
                status: if mismatched { "needs-manual-review" } else { "already-standard" },
                reason: if mismatched {
                    format!("one or more declarations do not belong in the current `{directory}/` project area")
                } else {
                    format!("source is already inside the `{directory}/` project area")
                },
                span: first_declaration_span(&program),
                declarations,
            });
            continue;
        }
        let destinations = destinations(&program);
        let span = first_declaration_span(&program);
        let declarations = declaration_plans(&program, None);
        let (directory, status, reason) = match destinations.as_slice() {
            [directory] => (
                directory.to_string(),
                "classified",
                format!("declarations map to the `{directory}/` project area"),
            ),
            [] => (
                "domain".to_owned(),
                "needs-manual-review",
                "no recognized top-level declarations; destination is only a suggestion".into(),
            ),
            _ => (
                "(manual decision)".to_owned(),
                "needs-manual-review",
                format!("mixed declarations map to: {}", destinations.join(", ")),
            ),
        };
        if status == "needs-manual-review" {
            manual_review += 1;
        }
        files.push(FilePlan {
            path: display,
            rule: if status == "classified" {
                "LAYOUT-CLASSIFY-001"
            } else {
                "LAYOUT-REVIEW-001"
            },
            proposed_directory: directory,
            status,
            reason,
            span,
            declarations,
        });
    }
    Ok(MigrationPlan {
        schema_version: 2,
        mode: "read-only-check",
        summary: Summary {
            classified: files
                .iter()
                .filter(|file| file.status == "classified")
                .count(),
            manual_review,
            unchanged_entrypoints,
        },
        files,
    })
}

fn declaration_plans(program: &Program, current_directory: Option<&str>) -> Vec<DeclarationPlan> {
    let mut declarations = Vec::new();
    macro_rules! add {
        ($items:expr, $kind:literal, $name:expr, $span:expr, $destination:literal) => {
            for item in &$items {
                let (status, rule, reason) = match current_directory {
                    Some(directory) if directory == $destination => (
                        "already-standard",
                        "LAYOUT-PLACEMENT-001",
                        format!("{} belongs in the current `{directory}/` area", $kind),
                    ),
                    Some(directory) => (
                        "needs-manual-review",
                        "LAYOUT-PLACEMENT-002",
                        format!(
                            "{} belongs in `{}/`, not the current `{directory}/` area",
                            $kind, $destination
                        ),
                    ),
                    None => (
                        "classified",
                        "LAYOUT-CLASSIFY-001",
                        format!("{} belongs in the `{}/` area", $kind, $destination),
                    ),
                };
                declarations.push(DeclarationPlan {
                    kind: $kind,
                    name: ($name)(item),
                    proposed_directory: $destination.into(),
                    status,
                    rule,
                    reason,
                    span: ($span)(item),
                });
            }
        };
    }
    add!(
        program.databases,
        "database",
        |item: &zelyra_ast::DatabaseDef| item.name.clone(),
        |item: &zelyra_ast::DatabaseDef| item.span,
        "schema"
    );
    add!(
        program.tables,
        "table",
        |item: &zelyra_ast::TableDef| item.name.clone(),
        |item: &zelyra_ast::TableDef| item.span,
        "schema"
    );
    add!(
        program.types,
        "type",
        |item: &zelyra_ast::TypeDef| item.name.clone(),
        |item: &zelyra_ast::TypeDef| item.span,
        "domain"
    );
    add!(
        program.records,
        "record",
        |item: &zelyra_ast::RecordDef| item.name.clone(),
        |item: &zelyra_ast::RecordDef| item.span,
        "domain"
    );
    add!(
        program.enums,
        "enum",
        |item: &zelyra_ast::EnumDef| item.name.clone(),
        |item: &zelyra_ast::EnumDef| item.span,
        "domain"
    );
    add!(
        program.views,
        "view",
        |item: &zelyra_ast::ViewDef| item.name.clone(),
        |item: &zelyra_ast::ViewDef| item.span,
        "routes"
    );
    add!(
        program.components,
        "component",
        |item: &zelyra_ast::ComponentDef| item.name.clone(),
        |item: &zelyra_ast::ComponentDef| item.span,
        "routes"
    );
    add!(
        program.pages,
        "page",
        |item: &zelyra_ast::PageDef| item.path.clone(),
        |item: &zelyra_ast::PageDef| item.span,
        "routes"
    );
    add!(
        program.tableviews,
        "tableview",
        |item: &zelyra_ast::TableViewDef| item.name.clone(),
        |item: &zelyra_ast::TableViewDef| item.span,
        "routes"
    );
    add!(
        program.forms,
        "form",
        |item: &zelyra_ast::FormDef| item.name.clone(),
        |item: &zelyra_ast::FormDef| item.span,
        "routes"
    );
    add!(
        program.cruds,
        "crud",
        |item: &zelyra_ast::CrudDef| item.name.clone(),
        |item: &zelyra_ast::CrudDef| item.span,
        "routes"
    );
    add!(
        program.auth,
        "auth",
        |item: &zelyra_ast::AuthDef| item.name.clone(),
        |item: &zelyra_ast::AuthDef| item.span,
        "routes"
    );
    add!(
        program.apis,
        "api",
        |item: &zelyra_ast::ApiDef| format!("{} {}", item.method, item.path),
        |item: &zelyra_ast::ApiDef| item.span,
        "routes"
    );
    add!(
        program.commands,
        "command",
        |item: &zelyra_ast::CommandDef| item.name.clone(),
        |item: &zelyra_ast::CommandDef| item.span,
        "domain"
    );
    add!(
        program.schedules,
        "schedule",
        |item: &zelyra_ast::ScheduleDef| item.name.clone(),
        |item: &zelyra_ast::ScheduleDef| item.span,
        "domain"
    );
    add!(
        program.functions,
        "function",
        |item: &zelyra_ast::Function| item.name.clone(),
        |item: &zelyra_ast::Function| item.span,
        "domain"
    );
    declarations.sort_by_key(|declaration| declaration.span.start);
    declarations
}

fn first_declaration_span(program: &Program) -> Option<Span> {
    program
        .databases
        .iter()
        .map(|item| item.span)
        .chain(program.tables.iter().map(|item| item.span))
        .chain(program.types.iter().map(|item| item.span))
        .chain(program.records.iter().map(|item| item.span))
        .chain(program.enums.iter().map(|item| item.span))
        .chain(program.views.iter().map(|item| item.span))
        .chain(program.components.iter().map(|item| item.span))
        .chain(program.pages.iter().map(|item| item.span))
        .chain(program.tableviews.iter().map(|item| item.span))
        .chain(program.forms.iter().map(|item| item.span))
        .chain(program.cruds.iter().map(|item| item.span))
        .chain(program.auth.iter().map(|item| item.span))
        .chain(program.apis.iter().map(|item| item.span))
        .chain(program.commands.iter().map(|item| item.span))
        .chain(program.schedules.iter().map(|item| item.span))
        .chain(program.functions.iter().map(|item| item.span))
        .min_by_key(|span| span.start)
}

fn collect_sources(root: &Path, directory: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    for item in fs::read_dir(directory)
        .map_err(|error| format!("cannot read `{}`: {error}", directory.display()))?
    {
        let item = item.map_err(|error| format!("cannot inspect project entry: {error}"))?;
        let path = item.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect `{}`: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            if path.extension().is_some_and(|extension| extension == "zyl") {
                return Err(format!(
                    "refusing to follow source symlink `{}`",
                    path.display()
                ));
            }
            continue;
        }
        if metadata.is_dir() {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "directory path escaped the project root".to_owned())?;
            if relative.components().count() == 1
                && matches!(
                    relative.to_string_lossy().as_ref(),
                    ".git" | "target" | ".zelyra" | "packages" | "vendor"
                )
            {
                continue;
            }
            collect_sources(root, &path, paths)?;
        } else if metadata.is_file() && path.extension().is_some_and(|extension| extension == "zyl")
        {
            paths.push(path);
        }
    }
    Ok(())
}

fn standard_directory(path: &Path) -> Option<&'static str> {
    let mut components = path.components();
    let first = components.next()?.as_os_str().to_str()?;
    components.next()?;
    ["schema", "domain", "routes"]
        .into_iter()
        .find(|directory| *directory == first)
}

fn destinations(program: &Program) -> Vec<&'static str> {
    let mut destinations = Vec::new();
    if !program.databases.is_empty() || !program.tables.is_empty() {
        destinations.push("schema");
    }
    if !program.pages.is_empty()
        || !program.apis.is_empty()
        || !program.forms.is_empty()
        || !program.cruds.is_empty()
        || !program.auth.is_empty()
        || !program.views.is_empty()
        || !program.components.is_empty()
        || !program.tableviews.is_empty()
    {
        destinations.push("routes");
    }
    if !program.functions.is_empty()
        || !program.types.is_empty()
        || !program.records.is_empty()
        || !program.enums.is_empty()
        || !program.commands.is_empty()
        || !program.schedules.is_empty()
    {
        destinations.push("domain");
    }
    destinations
}

fn print_human(plan: &MigrationPlan) {
    println!(
        "Read-only project layout check (schema {})",
        plan.schema_version
    );
    for file in &plan.files {
        let location = file.span.map_or_else(
            || "-".to_owned(),
            |span| format!("{}:{}", span.line, span.column),
        );
        println!(
            "{}:{} -> {} [{}] {}: {}",
            file.path, location, file.proposed_directory, file.status, file.rule, file.reason
        );
        for declaration in &file.declarations {
            println!(
                "  {}:{}:{} {} `{}` -> {} [{}] {}: {}",
                file.path,
                declaration.span.line,
                declaration.span.column,
                declaration.kind,
                declaration.name,
                declaration.proposed_directory,
                declaration.status,
                declaration.rule,
                declaration.reason
            );
        }
    }
    println!(
        "Summary: {} classified, {} need manual review, {} entrypoint(s) unchanged. No files were modified.",
        plan.summary.classified, plan.summary.manual_review, plan.summary.unchanged_entrypoints
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn empty_program() -> Program {
        Program {
            imports: Vec::new(),
            databases: Vec::new(),
            tables: Vec::new(),
            types: Vec::new(),
            records: Vec::new(),
            enums: Vec::new(),
            views: Vec::new(),
            components: Vec::new(),
            pages: Vec::new(),
            tableviews: Vec::new(),
            forms: Vec::new(),
            cruds: Vec::new(),
            auth: Vec::new(),
            apis: Vec::new(),
            commands: Vec::new(),
            schedules: Vec::new(),
            functions: Vec::new(),
        }
    }

    struct TempProject(PathBuf);

    impl TempProject {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("zelyra-layout-preview-{nonce}"));
            fs::create_dir_all(&root).unwrap();
            fs::write(root.join("main.zyl"), "fn main() {}\n").unwrap();
            Self(root)
        }
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn preview_classifies_sources_deterministically_without_writing() {
        let project = TempProject::new();
        fs::write(project.0.join("schema.zyl"), "table Customer { id: Id }\n").unwrap();
        fs::write(
            project.0.join("logic.zyl"),
            "fn total() -> Int { return 42 }\n",
        )
        .unwrap();
        fs::create_dir(project.0.join("domain")).unwrap();
        fs::write(project.0.join("domain/existing.zyl"), "fn saved() {}\n").unwrap();
        let before = fs::read_dir(&project.0).unwrap().count();
        let first = build_plan(&project.0.join("main.zyl")).unwrap();
        let second = build_plan(&project.0.join("main.zyl")).unwrap();
        assert_eq!(first.files, second.files);
        assert_eq!(before, fs::read_dir(&project.0).unwrap().count());
        assert_eq!(first.summary.classified, 2);
        let logic = first
            .files
            .iter()
            .find(|file| file.path == "logic.zyl")
            .unwrap();
        let schema = first
            .files
            .iter()
            .find(|file| file.path == "schema.zyl")
            .unwrap();
        let entry = first
            .files
            .iter()
            .find(|file| file.path == "main.zyl")
            .unwrap();
        assert_eq!(logic.proposed_directory, "domain");
        assert_eq!(logic.rule, "LAYOUT-CLASSIFY-001");
        assert_eq!(logic.span.map(|span| span.line), Some(1));
        assert_eq!(logic.declarations.len(), 1);
        assert_eq!(logic.declarations[0].kind, "function");
        assert_eq!(logic.declarations[0].name, "total");
        assert_eq!(logic.declarations[0].status, "classified");
        assert_eq!(schema.proposed_directory, "schema");
        assert_eq!(schema.rule, "LAYOUT-CLASSIFY-001");
        assert_eq!(schema.declarations[0].kind, "table");
        assert_eq!(schema.declarations[0].name, "Customer");
        assert_eq!(schema.span.map(|span| span.line), Some(1));
        assert_eq!(entry.status, "unchanged-entrypoint");
        assert!(first.files.iter().any(|file| {
            file.path == "domain/existing.zyl" && file.status == "already-standard"
        }));
    }

    #[test]
    fn mixed_and_empty_sources_require_manual_review() {
        let project = TempProject::new();
        fs::write(
            project.0.join("mixed.zyl"),
            "table Customer { id: Id }\nfn f() {}\n",
        )
        .unwrap();
        fs::write(project.0.join("empty.zyl"), "// no declarations\n").unwrap();
        let plan = build_plan(&project.0.join("main.zyl")).unwrap();
        assert_eq!(plan.summary.manual_review, 2);
        assert!(plan
            .files
            .iter()
            .all(|file| file.status != "classified" || file.path == "main.zyl"));
        let mixed = plan
            .files
            .iter()
            .find(|file| file.path == "mixed.zyl")
            .unwrap();
        assert_eq!(mixed.declarations.len(), 2);
        assert_eq!(mixed.declarations[0].proposed_directory, "schema");
        assert_eq!(mixed.declarations[1].proposed_directory, "domain");
        assert!(mixed.declarations[0].span.start < mixed.declarations[1].span.start);
    }

    #[test]
    fn declarations_in_the_wrong_standard_area_are_reported_individually() {
        let project = TempProject::new();
        fs::create_dir(project.0.join("domain")).unwrap();
        fs::write(
            project.0.join("domain/misplaced.zyl"),
            "fn calculate() -> Int { return 1 }\ntable Customer { id: Id }\n",
        )
        .unwrap();

        let plan = build_plan(&project.0.join("main.zyl")).unwrap();
        let file = plan
            .files
            .iter()
            .find(|file| file.path == "domain/misplaced.zyl")
            .unwrap();
        assert_eq!(file.status, "needs-manual-review");
        assert_eq!(file.rule, "LAYOUT-PLACEMENT-002");
        assert_eq!(file.declarations.len(), 2);
        assert_eq!(file.declarations[0].name, "calculate");
        assert_eq!(file.declarations[0].status, "already-standard");
        assert_eq!(file.declarations[1].name, "Customer");
        assert_eq!(file.declarations[1].proposed_directory, "schema");
        assert_eq!(file.declarations[1].status, "needs-manual-review");
        assert_eq!(plan.summary.manual_review, 1);
    }

    #[test]
    fn unparseable_sources_are_reported_without_aborting_the_inventory() {
        let project = TempProject::new();
        fs::write(project.0.join("invalid.zyl"), "fn broken( -> {\n").unwrap();
        fs::write(project.0.join("valid.zyl"), "fn good() {}\n").unwrap();
        let plan = build_plan(&project.0.join("main.zyl")).unwrap();
        assert_eq!(plan.summary.manual_review, 1);
        assert!(plan.files.iter().any(|file| {
            file.path == "invalid.zyl"
                && file.status == "needs-manual-review"
                && file.reason.contains("does not parse")
        }));
        assert!(plan
            .files
            .iter()
            .any(|file| file.path == "valid.zyl" && file.status == "classified"));
    }

    #[test]
    fn strict_architecture_enforces_declaration_areas_only_when_enabled() {
        let project = TempProject::new();
        for directory in ["schema", "domain", "routes", "public", "assets"] {
            fs::create_dir(project.0.join(directory)).unwrap();
        }
        fs::write(
            project.0.join("zelyra.toml"),
            "[project]\nlayout = \"standard\"\nstrict_architecture = true\n",
        )
        .unwrap();
        let module = |path: &str, declarations: &[&str]| crate::project::ProjectModule {
            path: path.into(),
            imports: Vec::new(),
            exports: Vec::new(),
            declarations: declarations.iter().map(|value| (*value).into()).collect(),
        };
        let valid = vec![
            module(
                "schema/customer.zyl",
                &["table:Customer", "record:Customer"],
            ),
            module("domain/customer.zyl", &["function:find_customer"]),
            module(
                "routes/customers.zyl",
                &["api:GET /customers", "function:list"],
            ),
        ];
        assert!(validate_project_modules(
            project.0.join("main.zyl").to_str().unwrap(),
            &valid,
            &empty_program(),
            &[],
        )
        .is_ok());
        let invalid = vec![module("domain/http.zyl", &["api:GET /remote"])];
        let issues = validate_project_modules(
            project.0.join("main.zyl").to_str().unwrap(),
            &invalid,
            &empty_program(),
            &[],
        )
        .unwrap_err();
        assert_eq!(issues[0].path, "domain/http.zyl");
        assert!(issues[0].message.contains("not allowed under `domain/`"));
    }

    #[test]
    fn route_sql_effect_classification_handles_comments_ctes_and_unknowns() {
        assert_eq!(
            sql_effect("INSERT INTO customers (name) VALUES ('Ada')"),
            SqlEffect::Write
        );
        assert_eq!(
            sql_effect("/* audit */ UPDATE customers SET name = 'Ada'"),
            SqlEffect::Write
        );
        assert_eq!(
            sql_effect("WITH candidates AS (SELECT id FROM customers) DELETE FROM customers WHERE id IN (SELECT id FROM candidates)"),
            SqlEffect::Write
        );
        assert_eq!(
            sql_effect("SELECT 'INSERT INTO customers' FROM customers"),
            SqlEffect::Read
        );
        assert_eq!(
            sql_effect("SELECT SLEEP(1) FROM customers"),
            SqlEffect::Unknown
        );
        assert_eq!(
            sql_effect("CALL refresh_customer_cache()"),
            SqlEffect::Unknown
        );
        assert_eq!(sql_effect("/* unterminated"), SqlEffect::Unknown);
    }

    #[test]
    fn route_sql_analysis_continues_through_non_domain_modules() {
        let source = "fn helper() -> Int { sql { INSERT INTO customers (name) VALUES ('Ada') } return 1 }\nfn serve_customer() -> Int { return helper() }\napi POST \"/customers\" { handler serve_customer output Int }\n";
        let mut program = parse(&lex(source).unwrap()).unwrap();
        program.functions[0].span = program.functions[0].span.with_source_id(1);
        program.functions[1].span = program.functions[1].span.with_source_id(0);
        let sources = vec![
            crate::project::ProjectSource {
                path: "routes/customers.zyl".into(),
                text: source.into(),
            },
            crate::project::ProjectSource {
                path: "main.zyl".into(),
                text: source.into(),
            },
        ];
        let issues = route_handler_sql_issues(&program, &sources);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].path, "main.zyl");
        assert_eq!(issues[0].code, "E-ARCH-002");
    }

    #[test]
    fn route_form_and_crud_actions_are_checked_for_sql_writes() {
        let source = "form CustomerCreate { action save { sql { INSERT INTO customers (name) VALUES ('Ada') } } }\ncrud Customer -> customers { action archive { sql { UPDATE customers SET active = false } } }\n";
        let program = parse(&lex(source).unwrap()).unwrap();
        let sources = vec![crate::project::ProjectSource {
            path: "routes/customers.zyl".into(),
            text: source.into(),
        }];
        let issues = route_handler_sql_issues(&program, &sources);
        assert_eq!(issues.len(), 2);
        assert!(issues.iter().all(|issue| issue.code == "E-ARCH-002"));
        assert!(issues[0].message.contains("form action"));
        assert!(issues[1].message.contains("CRUD action"));
    }

    #[cfg(unix)]
    #[test]
    fn source_symlinks_are_rejected_without_following_them() {
        use std::os::unix::fs::symlink;
        let project = TempProject::new();
        let external = project.0.parent().unwrap().join("outside.zyl");
        fs::write(&external, "fn outside() {}\n").unwrap();
        symlink(&external, project.0.join("linked.zyl")).unwrap();
        assert!(build_plan(&project.0.join("main.zyl"))
            .unwrap_err()
            .contains("symlink"));
        fs::remove_file(external).unwrap();
    }
}
