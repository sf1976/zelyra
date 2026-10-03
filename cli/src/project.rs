use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use zelyra_ast::{
    BinaryOp, Block, Expr, ExprKind, Function, ImportDef, Program, Span, Stmt, UnaryOp,
};
use zelyra_lexer::lex;
use zelyra_parser::parse;

#[derive(Clone, Debug)]
pub struct ProjectSource {
    pub path: String,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct LoadedProject {
    pub program: Program,
    pub sources: Vec<ProjectSource>,
}

#[derive(Clone, Debug)]
pub struct ProjectError {
    pub code: &'static str,
    pub message: String,
    pub path: String,
    pub span: Span,
    pub sources: Box<[ProjectSource]>,
}

struct Module {
    relative_path: String,
    source_id: u32,
    program: Program,
    imports: HashMap<String, PathBuf>,
}

struct Loader {
    root: PathBuf,
    sources: Vec<ProjectSource>,
    modules: HashMap<PathBuf, Module>,
    order: Vec<PathBuf>,
    visiting: Vec<PathBuf>,
}

pub fn load(entry: &str) -> Result<LoadedProject, ProjectError> {
    let entry_path = PathBuf::from(entry);
    let root = entry_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let root = fs::canonicalize(root).map_err(|error| ProjectError {
        code: "E-IO-001",
        message: format!("cannot resolve project root: {error}"),
        path: entry.to_owned(),
        span: Span::default(),
        sources: Box::default(),
    })?;
    let canonical_entry = fs::canonicalize(&entry_path).map_err(|error| ProjectError {
        code: "E-IO-001",
        message: format!("cannot read `{entry}`: {error}"),
        path: entry.to_owned(),
        span: Span::default(),
        sources: Box::default(),
    })?;
    if !canonical_entry.starts_with(&root) {
        return Err(ProjectError {
            code: "E-MOD-004",
            message: "entry source resolves outside the project root".into(),
            path: entry.to_owned(),
            span: Span::default(),
            sources: Box::default(),
        });
    }

    let mut loader = Loader {
        root,
        sources: Vec::new(),
        modules: HashMap::new(),
        order: Vec::new(),
        visiting: Vec::new(),
    };
    loader.visit(canonical_entry.clone(), None)?;
    let program =
        link_modules(&canonical_entry, &loader.modules, &loader.order).map_err(|mut error| {
            error.sources = loader.sources.clone().into_boxed_slice();
            error
        })?;
    Ok(LoadedProject {
        program,
        sources: loader.sources,
    })
}

impl Loader {
    fn visit(
        &mut self,
        path: PathBuf,
        import_site: Option<(String, Span)>,
    ) -> Result<(), ProjectError> {
        let canonical = fs::canonicalize(&path).map_err(|error| {
            let (display_path, span) = import_site
                .clone()
                .unwrap_or_else(|| (path.to_string_lossy().into_owned(), Span::default()));
            self.error(
                "E-MOD-002",
                format!(
                    "cannot resolve imported source `{}`: {error}",
                    path.display()
                ),
                display_path,
                span,
            )
        })?;
        if !canonical.starts_with(&self.root) {
            let (display_path, span) = import_site
                .unwrap_or_else(|| (canonical.to_string_lossy().into_owned(), Span::default()));
            return Err(self.error(
                "E-MOD-004",
                format!(
                    "module path `{}` resolves outside the project root",
                    path.display()
                ),
                display_path,
                span,
            ));
        }
        if let Some(cycle_start) = self.visiting.iter().position(|active| active == &canonical) {
            let mut cycle = self.visiting[cycle_start..]
                .iter()
                .map(|path| self.relative(path))
                .collect::<Vec<_>>();
            cycle.push(self.relative(&canonical));
            let (display_path, span) =
                import_site.unwrap_or_else(|| (self.relative(&canonical), Span::default()));
            return Err(self.error(
                "E-MOD-003",
                format!("module import cycle: {}", cycle.join(" -> ")),
                display_path,
                span,
            ));
        }
        if self.modules.contains_key(&canonical) {
            return Ok(());
        }

        let text = fs::read_to_string(&canonical).map_err(|error| {
            let (display_path, span) = import_site
                .clone()
                .unwrap_or_else(|| (self.relative(&canonical), Span::default()));
            self.error(
                "E-MOD-002",
                format!(
                    "cannot read module `{}`: {error}",
                    self.relative(&canonical)
                ),
                display_path,
                span,
            )
        })?;
        let source_id = u32::try_from(self.sources.len()).map_err(|_| {
            self.error(
                "E-MOD-010",
                "project contains too many source files".into(),
                self.relative(&canonical),
                Span::default(),
            )
        })?;
        let relative_path = self.relative(&canonical);
        let display_path = if self.sources.is_empty() {
            import_site
                .as_ref()
                .map_or_else(|| relative_path.clone(), |(path, _)| path.clone())
        } else {
            relative_path.clone()
        };
        self.sources.push(ProjectSource {
            path: display_path.clone(),
            text: text.clone(),
        });

        let mut tokens = match lex(&text) {
            Ok(tokens) => tokens,
            Err(error) => {
                let span = error.span.with_source_id(source_id);
                return Err(self.error("E-LEX-001", error.message, display_path, span));
            }
        };
        for token in &mut tokens {
            token.span = token.span.with_source_id(source_id);
        }
        let mut program = match parse(&tokens) {
            Ok(program) => program,
            Err(error) => {
                return Err(self.error("E-PARSE-001", error.message, display_path, error.span));
            }
        };

        let mut aliases = HashSet::new();
        for import in &program.imports {
            if !aliases.insert(import.alias.clone()) {
                return Err(self.error(
                    "E-MOD-005",
                    format!("module alias `{}` is declared more than once", import.alias),
                    display_path.clone(),
                    import.span,
                ));
            }
        }

        self.visiting.push(canonical.clone());
        let mut resolved_imports = HashMap::new();
        for import in &program.imports {
            let target = self.import_target(import, &display_path)?;
            self.visit(target.clone(), Some((display_path.clone(), import.span)))?;
            resolved_imports.insert(
                import.alias.clone(),
                fs::canonicalize(target).map_err(|error| {
                    self.error(
                        "E-MOD-002",
                        format!("cannot resolve imported module: {error}"),
                        display_path.clone(),
                        import.span,
                    )
                })?,
            );
        }
        self.visiting.pop();
        // Import statements are compiler metadata; the linked program contains
        // their resolved functions instead of unresolved source directives.
        program.imports.clear();
        self.modules.insert(
            canonical.clone(),
            Module {
                relative_path,
                source_id,
                program,
                imports: resolved_imports,
            },
        );
        self.order.push(canonical);
        Ok(())
    }

    fn import_target(&self, import: &ImportDef, importer: &str) -> Result<PathBuf, ProjectError> {
        let relative = Path::new(&import.path);
        let valid_components = relative
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
        if !valid_components
            || relative.is_absolute()
            || relative.extension().is_none_or(|ext| ext != "zyl")
        {
            return Err(self.error(
                "E-MOD-001",
                format!(
                    "import path `{}` must be a project-relative `.zyl` path without parent or root components",
                    import.path
                ),
                importer.to_owned(),
                import.span,
            ));
        }
        Ok(self.root.join(relative))
    }

    fn relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }

    fn error(&self, code: &'static str, message: String, path: String, span: Span) -> ProjectError {
        ProjectError {
            code,
            message,
            path,
            span,
            sources: self.sources.clone().into_boxed_slice(),
        }
    }
}

fn link_modules(
    entry: &Path,
    modules: &HashMap<PathBuf, Module>,
    order: &[PathBuf],
) -> Result<Program, ProjectError> {
    let root = modules.get(entry).expect("entry module was loaded");
    let root_functions = function_visibility(&root.program);
    let module_names = modules
        .iter()
        .map(|(path, module)| (path.clone(), module.relative_path.clone()))
        .collect::<HashMap<_, _>>();
    let mut function_sets = HashMap::new();
    for (path, module) in modules {
        let functions = function_visibility(&module.program);
        if path != entry && has_non_function_declarations(&module.program) {
            let span = first_non_function_span(&module.program).unwrap_or_default();
            return Err(ProjectError {
                code: "E-MOD-008",
                message: "imported modules currently support function declarations only; keep tables, views, APIs, and other project declarations in the entry file".into(),
                path: module.relative_path.clone(),
                span,
                sources: Box::default(),
            });
        }
        function_sets.insert(path.clone(), functions);
    }

    let mut linked = root.program.clone();
    for path in order {
        if path == entry {
            continue;
        }
        let module = modules.get(path).expect("ordered module was loaded");
        let module_name = module_names.get(path).expect("module name exists");
        for mut function in module.program.functions.clone() {
            rewrite_function(
                &mut function,
                path,
                module_name,
                &module.imports,
                &function_sets,
                &module_names,
                &root_functions,
                &module.relative_path,
                false,
            )?;
            function.name = internal_name(module_name, &function.name);
            linked.functions.push(function);
        }
    }

    for function in &mut linked.functions {
        if function.span.source_id != root.source_id {
            continue;
        }
        rewrite_function(
            function,
            entry,
            &root.relative_path,
            &root.imports,
            &function_sets,
            &module_names,
            &root_functions,
            &root.relative_path,
            true,
        )?;
    }
    linked.imports.clear();
    Ok(linked)
}

fn function_visibility(program: &Program) -> HashMap<String, bool> {
    program
        .functions
        .iter()
        .map(|function| (function.name.clone(), function.is_public))
        .collect()
}

fn has_non_function_declarations(program: &Program) -> bool {
    !program.databases.is_empty()
        || !program.tables.is_empty()
        || !program.types.is_empty()
        || !program.records.is_empty()
        || !program.views.is_empty()
        || !program.components.is_empty()
        || !program.pages.is_empty()
        || !program.tableviews.is_empty()
        || !program.forms.is_empty()
        || !program.cruds.is_empty()
        || !program.auth.is_empty()
        || !program.apis.is_empty()
}

fn first_non_function_span(program: &Program) -> Option<Span> {
    program
        .databases
        .first()
        .map(|item| item.span)
        .or_else(|| program.tables.first().map(|item| item.span))
        .or_else(|| program.types.first().map(|item| item.span))
        .or_else(|| program.records.first().map(|item| item.span))
        .or_else(|| program.views.first().map(|item| item.span))
        .or_else(|| program.components.first().map(|item| item.span))
        .or_else(|| program.pages.first().map(|item| item.span))
        .or_else(|| program.tableviews.first().map(|item| item.span))
        .or_else(|| program.forms.first().map(|item| item.span))
        .or_else(|| program.cruds.first().map(|item| item.span))
        .or_else(|| program.auth.first().map(|item| item.span))
        .or_else(|| program.apis.first().map(|item| item.span))
}

fn internal_name(module_path: &str, function: &str) -> String {
    format!("{module_path}::{function}")
}

#[allow(clippy::too_many_arguments)]
fn rewrite_function(
    function: &mut Function,
    module_path: &Path,
    module_name: &str,
    imports: &HashMap<String, PathBuf>,
    function_sets: &HashMap<PathBuf, HashMap<String, bool>>,
    module_names: &HashMap<PathBuf, String>,
    root_functions: &HashMap<String, bool>,
    display_path: &str,
    is_root: bool,
) -> Result<(), ProjectError> {
    let local_functions = function_sets
        .get(module_path)
        .expect("current module functions are indexed");
    for contract in function.requires.iter_mut().chain(&mut function.ensures) {
        rewrite_expr(
            contract,
            module_path,
            module_name,
            imports,
            local_functions,
            function_sets,
            module_names,
            root_functions,
            display_path,
            is_root,
        )?;
    }
    rewrite_block(
        &mut function.body,
        module_path,
        module_name,
        imports,
        local_functions,
        function_sets,
        module_names,
        root_functions,
        display_path,
        is_root,
    )
}

#[allow(clippy::too_many_arguments)]
fn rewrite_block(
    block: &mut Block,
    module_path: &Path,
    module_name: &str,
    imports: &HashMap<String, PathBuf>,
    local_functions: &HashMap<String, bool>,
    function_sets: &HashMap<PathBuf, HashMap<String, bool>>,
    module_names: &HashMap<PathBuf, String>,
    root_functions: &HashMap<String, bool>,
    display_path: &str,
    is_root: bool,
) -> Result<(), ProjectError> {
    for statement in &mut block.statements {
        match statement {
            Stmt::Let { value, .. } | Stmt::BindOrAssign { value, .. } => rewrite_expr(
                value,
                module_path,
                module_name,
                imports,
                local_functions,
                function_sets,
                module_names,
                root_functions,
                display_path,
                is_root,
            )?,
            Stmt::Expr(expression) => rewrite_expr(
                expression,
                module_path,
                module_name,
                imports,
                local_functions,
                function_sets,
                module_names,
                root_functions,
                display_path,
                is_root,
            )?,
            Stmt::Return {
                value: Some(expression),
                ..
            } => rewrite_expr(
                expression,
                module_path,
                module_name,
                imports,
                local_functions,
                function_sets,
                module_names,
                root_functions,
                display_path,
                is_root,
            )?,
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                rewrite_expr(
                    condition,
                    module_path,
                    module_name,
                    imports,
                    local_functions,
                    function_sets,
                    module_names,
                    root_functions,
                    display_path,
                    is_root,
                )?;
                rewrite_block(
                    then_block,
                    module_path,
                    module_name,
                    imports,
                    local_functions,
                    function_sets,
                    module_names,
                    root_functions,
                    display_path,
                    is_root,
                )?;
                if let Some(block) = else_block {
                    rewrite_block(
                        block,
                        module_path,
                        module_name,
                        imports,
                        local_functions,
                        function_sets,
                        module_names,
                        root_functions,
                        display_path,
                        is_root,
                    )?;
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                rewrite_expr(
                    condition,
                    module_path,
                    module_name,
                    imports,
                    local_functions,
                    function_sets,
                    module_names,
                    root_functions,
                    display_path,
                    is_root,
                )?;
                for invariant in invariants {
                    rewrite_expr(
                        invariant,
                        module_path,
                        module_name,
                        imports,
                        local_functions,
                        function_sets,
                        module_names,
                        root_functions,
                        display_path,
                        is_root,
                    )?;
                }
                rewrite_block(
                    body,
                    module_path,
                    module_name,
                    imports,
                    local_functions,
                    function_sets,
                    module_names,
                    root_functions,
                    display_path,
                    is_root,
                )?;
            }
            Stmt::For { iterable, body, .. } => {
                rewrite_expr(
                    iterable,
                    module_path,
                    module_name,
                    imports,
                    local_functions,
                    function_sets,
                    module_names,
                    root_functions,
                    display_path,
                    is_root,
                )?;
                rewrite_block(
                    body,
                    module_path,
                    module_name,
                    imports,
                    local_functions,
                    function_sets,
                    module_names,
                    root_functions,
                    display_path,
                    is_root,
                )?;
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    rewrite_expr(
                        invariant,
                        module_path,
                        module_name,
                        imports,
                        local_functions,
                        function_sets,
                        module_names,
                        root_functions,
                        display_path,
                        is_root,
                    )?;
                }
                rewrite_block(
                    body,
                    module_path,
                    module_name,
                    imports,
                    local_functions,
                    function_sets,
                    module_names,
                    root_functions,
                    display_path,
                    is_root,
                )?;
            }
            Stmt::Match { value, arms, .. } => {
                rewrite_expr(
                    value,
                    module_path,
                    module_name,
                    imports,
                    local_functions,
                    function_sets,
                    module_names,
                    root_functions,
                    display_path,
                    is_root,
                )?;
                for arm in arms {
                    rewrite_block(
                        &mut arm.body,
                        module_path,
                        module_name,
                        imports,
                        local_functions,
                        function_sets,
                        module_names,
                        root_functions,
                        display_path,
                        is_root,
                    )?;
                }
            }
            Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => rewrite_block(
                body,
                module_path,
                module_name,
                imports,
                local_functions,
                function_sets,
                module_names,
                root_functions,
                display_path,
                is_root,
            )?,
            Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
fn rewrite_expr(
    expression: &mut Expr,
    module_path: &Path,
    module_name: &str,
    imports: &HashMap<String, PathBuf>,
    local_functions: &HashMap<String, bool>,
    function_sets: &HashMap<PathBuf, HashMap<String, bool>>,
    module_names: &HashMap<PathBuf, String>,
    root_functions: &HashMap<String, bool>,
    display_path: &str,
    is_root: bool,
) -> Result<(), ProjectError> {
    match &mut expression.kind {
        ExprKind::Array(values) => {
            for value in values {
                rewrite_expr(
                    value,
                    module_path,
                    module_name,
                    imports,
                    local_functions,
                    function_sets,
                    module_names,
                    root_functions,
                    display_path,
                    is_root,
                )?;
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                rewrite_expr(
                    key,
                    module_path,
                    module_name,
                    imports,
                    local_functions,
                    function_sets,
                    module_names,
                    root_functions,
                    display_path,
                    is_root,
                )?;
                rewrite_expr(
                    value,
                    module_path,
                    module_name,
                    imports,
                    local_functions,
                    function_sets,
                    module_names,
                    root_functions,
                    display_path,
                    is_root,
                )?;
            }
        }
        ExprKind::Record { fields, .. } => {
            for (_, value) in fields {
                rewrite_expr(
                    value,
                    module_path,
                    module_name,
                    imports,
                    local_functions,
                    function_sets,
                    module_names,
                    root_functions,
                    display_path,
                    is_root,
                )?;
            }
        }
        ExprKind::Index { target, index } => {
            rewrite_expr(
                target,
                module_path,
                module_name,
                imports,
                local_functions,
                function_sets,
                module_names,
                root_functions,
                display_path,
                is_root,
            )?;
            rewrite_expr(
                index,
                module_path,
                module_name,
                imports,
                local_functions,
                function_sets,
                module_names,
                root_functions,
                display_path,
                is_root,
            )?;
        }
        ExprKind::Field { target, .. } | ExprKind::Await(target) => rewrite_expr(
            target,
            module_path,
            module_name,
            imports,
            local_functions,
            function_sets,
            module_names,
            root_functions,
            display_path,
            is_root,
        )?,
        ExprKind::Unary {
            op: UnaryOp::Negate | UnaryOp::Not,
            expr,
        } => rewrite_expr(
            expr,
            module_path,
            module_name,
            imports,
            local_functions,
            function_sets,
            module_names,
            root_functions,
            display_path,
            is_root,
        )?,
        ExprKind::Binary {
            left,
            op:
                BinaryOp::Add
                | BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Remainder
                | BinaryOp::Equal
                | BinaryOp::NotEqual
                | BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual
                | BinaryOp::And
                | BinaryOp::Or,
            right,
        } => {
            rewrite_expr(
                left,
                module_path,
                module_name,
                imports,
                local_functions,
                function_sets,
                module_names,
                root_functions,
                display_path,
                is_root,
            )?;
            rewrite_expr(
                right,
                module_path,
                module_name,
                imports,
                local_functions,
                function_sets,
                module_names,
                root_functions,
                display_path,
                is_root,
            )?;
        }
        ExprKind::Call { name, args, .. } => {
            if let Some((alias, function_name)) = name.split_once("::") {
                if function_name.contains("::") {
                    return Err(project_error(
                        "E-MOD-006",
                        "qualified function calls currently support one imported namespace",
                        display_path,
                        expression.span,
                    ));
                }
                let target = imports.get(alias).ok_or_else(|| {
                    project_error(
                        "E-MOD-006",
                        format!("unknown imported module alias `{alias}`"),
                        display_path,
                        expression.span,
                    )
                })?;
                let visibility = function_sets
                    .get(target)
                    .and_then(|functions| functions.get(function_name))
                    .copied();
                match visibility {
                    None => return Err(project_error("E-MOD-006", format!("module `{alias}` has no function `{function_name}`"), display_path, expression.span)),
                    Some(false) => return Err(project_error("E-MOD-007", format!("function `{function_name}` in module `{alias}` is private; declare it with `pub fn` to call it from another module"), display_path, expression.span)),
                    Some(true) => {}
                }
                let target_module = function_sets
                    .get(target)
                    .expect("resolved module functions");
                let imported = target_module.get(function_name).is_some();
                if !imported {
                    unreachable!();
                }
                let target_module_name = module_names.get(target).expect("resolved module name");
                *name = internal_name(target_module_name, function_name);
            } else if let Some(public) = local_functions.get(name) {
                let _ = public;
                if !is_root {
                    *name = internal_name(module_name, name);
                }
            } else if !is_root && root_functions.contains_key(name) {
                return Err(project_error("E-MOD-007", format!("function `{name}` belongs to the entry module; call imported functions through an explicit namespace"), display_path, expression.span));
            }
            for argument in args {
                rewrite_expr(
                    argument,
                    module_path,
                    module_name,
                    imports,
                    local_functions,
                    function_sets,
                    module_names,
                    root_functions,
                    display_path,
                    is_root,
                )?;
            }
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
    Ok(())
}

fn project_error(
    code: &'static str,
    message: impl Into<String>,
    path: &str,
    span: Span,
) -> ProjectError {
    ProjectError {
        code,
        message: message.into(),
        path: path.to_owned(),
        span,
        sources: Box::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use zelyra_runtime::{check, execute};

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    fn project(files: &[(&str, &str)]) -> PathBuf {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let directory =
            std::env::temp_dir().join(format!("zelyra-module-test-{}-{id}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        for (relative, source) in files {
            let path = directory.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, source).unwrap();
        }
        directory
    }

    fn cleanup(directory: &Path) {
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn imports_public_functions_and_keeps_module_helpers_private() {
        let directory = project(&[
            (
                "main.zyl",
                "import \"src/math.zyl\" as math\nfn main() { print(math::add(2, 3)) }\n",
            ),
            (
                "src/math.zyl",
                "fn double(n: Int) -> Int { return n * 2 }\npub fn add(a: Int, b: Int) -> Int { return double(a + b) }\n",
            ),
        ]);
        let loaded = load(directory.join("main.zyl").to_str().unwrap()).unwrap();
        assert_eq!(execute(&loaded.program).unwrap(), ["10"]);
        assert!(check(&loaded.program).is_ok());
        assert_eq!(loaded.sources.len(), 2);
        cleanup(&directory);
    }

    #[test]
    fn imported_function_contracts_are_still_checked_at_runtime() {
        let directory = project(&[
            (
                "main.zyl",
                "import \"src/guards.zyl\" as guards\nfn main() { guards::positive(0) }\n",
            ),
            (
                "src/guards.zyl",
                "pub fn positive(value: Int) -> Int requires { value > 0 } { return value }\n",
            ),
        ]);
        let loaded = load(directory.join("main.zyl").to_str().unwrap()).unwrap();
        let error = execute(&loaded.program).unwrap_err();
        assert!(error.message.contains("precondition"));
        cleanup(&directory);
    }

    #[test]
    fn rejects_private_imported_functions() {
        let directory = project(&[
            (
                "main.zyl",
                "import \"src/math.zyl\" as math\nfn main() { print(math::secret()) }\n",
            ),
            ("src/math.zyl", "fn secret() -> Int { return 7 }\n"),
        ]);
        let error = load(directory.join("main.zyl").to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E-MOD-007");
        assert!(error.message.contains("private"));
        cleanup(&directory);
    }

    #[test]
    fn imported_files_reject_non_function_declarations_for_now() {
        let directory = project(&[
            (
                "main.zyl",
                "import \"src/domain.zyl\" as domain\nfn main() {}\n",
            ),
            (
                "src/domain.zyl",
                "table customers { id: Id primary auto }\npub fn count() -> Int { return 0 }\n",
            ),
        ]);
        let error = load(directory.join("main.zyl").to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E-MOD-008");
        assert!(error.message.contains("function declarations only"));
        cleanup(&directory);
    }

    #[test]
    fn rejects_cycles_and_parent_path_imports() {
        let cycle = project(&[
            ("main.zyl", "import \"src/a.zyl\" as a\nfn main() {}\n"),
            (
                "src/a.zyl",
                "import \"main.zyl\" as app\npub fn value() -> Int { return 1 }\n",
            ),
        ]);
        let error = load(cycle.join("main.zyl").to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E-MOD-003");
        assert!(error.message.contains("main.zyl"));
        cleanup(&cycle);

        let traversal = project(&[(
            "main.zyl",
            "import \"../outside.zyl\" as outside\nfn main() {}\n",
        )]);
        let error = load(traversal.join("main.zyl").to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E-MOD-001");
        cleanup(&traversal);
    }

    #[test]
    fn rejects_duplicate_import_aliases_and_missing_modules() {
        let duplicate = project(&[
            (
                "main.zyl",
                "import \"src/a.zyl\" as shared\nimport \"src/b.zyl\" as shared\nfn main() {}\n",
            ),
            ("src/a.zyl", "pub fn a() -> Int { return 1 }\n"),
            ("src/b.zyl", "pub fn b() -> Int { return 2 }\n"),
        ]);
        let error = load(duplicate.join("main.zyl").to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E-MOD-005");
        cleanup(&duplicate);

        let missing = project(&[(
            "main.zyl",
            "import \"src/missing.zyl\" as missing\nfn main() {}\n",
        )]);
        let error = load(missing.join("main.zyl").to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E-MOD-002");
        cleanup(&missing);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_modules_that_escape_the_project_root() {
        use std::os::unix::fs::symlink;

        let directory = project(&[(
            "main.zyl",
            "import \"src/external.zyl\" as external\nfn main() {}\n",
        )]);
        let outside = directory.with_extension("outside.zyl");
        fs::write(&outside, "pub fn value() -> Int { return 1 }\n").unwrap();
        fs::create_dir_all(directory.join("src")).unwrap();
        symlink(&outside, directory.join("src/external.zyl")).unwrap();
        let error = load(directory.join("main.zyl").to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E-MOD-004");
        cleanup(&directory);
        fs::remove_file(outside).unwrap();
    }

    #[test]
    fn imported_type_errors_retain_their_source_file_id() {
        let directory = project(&[
            (
                "main.zyl",
                "import \"src/math.zyl\" as math\nfn main() { print(math::wrong()) }\n",
            ),
            (
                "src/math.zyl",
                "pub fn wrong() -> Int { return \"not an integer\" }\n",
            ),
        ]);
        let loaded = load(directory.join("main.zyl").to_str().unwrap()).unwrap();
        let errors = check(&loaded.program).unwrap_err();
        assert!(errors.iter().any(|error| error.span.source_id == 1));
        cleanup(&directory);
    }
}
