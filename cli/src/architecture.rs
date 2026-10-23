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
    proposed_directory: String,
    status: &'static str,
    reason: String,
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
                    span: None,
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
        let mut pending = vec![handler.to_owned()];
        let mut visited = std::collections::HashSet::new();
        while let Some(name) = pending.pop() {
            if !visited.insert(name.clone()) {
                continue;
            }
            let Some(function) = functions.get(name.as_str()).copied() else {
                continue;
            };
            let Some(function_source) = sources.get(function.span.source_id as usize) else {
                continue;
            };
            if !function_source
                .path
                .replace('\\', "/")
                .starts_with("routes/")
            {
                // Business writes in domain modules are the intended separation.
                continue;
            }

            let mut calls = Vec::new();
            let mut sql = Vec::new();
            inspect_block(&function.body, &mut calls, &mut sql);
            pending.extend(calls);
            for (query, span) in sql {
                match sql_effect(&query) {
                    SqlEffect::Write => issues.push(ArchitectureIssue {
                        path: function_source.path.clone(),
                        code: "E-ARCH-002",
                        message: format!(
                            "route handler `{name}` reaches a write SQL statement; move the business write into a `domain/` function"
                        ),
                        span: Some(span),
                    }),
                    SqlEffect::Unknown => issues.push(ArchitectureIssue {
                        path: function_source.path.clone(),
                        code: "E-ARCH-003",
                        message: format!(
                            "route handler `{name}` reaches SQL that cannot be classified safely; move it into a `domain/` function"
                        ),
                        span: Some(span),
                    }),
                    SqlEffect::Read => {}
                }
            }
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

fn inspect_block(block: &Block, calls: &mut Vec<String>, sql: &mut Vec<(String, Span)>) {
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

fn inspect_expr(expression: &Expr, calls: &mut Vec<String>, sql: &mut Vec<(String, Span)>) {
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
            calls.push(name.clone());
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
                        "proposed_directory": file.proposed_directory,
                        "status": file.status,
                        "reason": file.reason,
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
            .map_err(|error| error.message)
            .and_then(|tokens| parse(&tokens).map_err(|error| error.message));
        let program = match parsed {
            Ok(program) => program,
            Err(error) if path == canonical_entry => {
                return Err(format!("{display}: {error}"));
            }
            Err(error) => {
                manual_review += 1;
                files.push(FilePlan {
                    path: display,
                    proposed_directory: "(manual decision)".into(),
                    status: "needs-manual-review",
                    reason: format!("source does not parse: {error}"),
                });
                continue;
            }
        };
        if path == canonical_entry {
            unchanged_entrypoints += 1;
            files.push(FilePlan {
                path: display,
                proposed_directory: "(project root)".into(),
                status: "unchanged-entrypoint",
                reason: "the selected entry file remains at the project root".into(),
            });
            continue;
        }
        if let Some(directory) = standard_directory(relative) {
            files.push(FilePlan {
                path: display,
                proposed_directory: directory.into(),
                status: "already-standard",
                reason: format!("source is already inside the `{directory}/` project area"),
            });
            continue;
        }
        let destinations = destinations(&program);
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
            proposed_directory: directory,
            status,
            reason,
        });
    }
    Ok(MigrationPlan {
        schema_version: 1,
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
        println!(
            "{} -> {} [{}]: {}",
            file.path, file.proposed_directory, file.status, file.reason
        );
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
        assert_eq!(schema.proposed_directory, "schema");
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
