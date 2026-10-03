use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use zelyra_ast::{
    BinaryOp, Block, Expr, ExprKind, FormAction, Function, ImportDef, Program, Span, Stmt, Type,
    UnaryOp,
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
    pub modules: Vec<ProjectModule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectModule {
    pub path: String,
    pub imports: Vec<ProjectImport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectImport {
    pub alias: String,
    pub path: String,
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
    let module_paths = loader
        .modules
        .iter()
        .map(|(path, module)| (path.clone(), module.relative_path.clone()))
        .collect::<HashMap<_, _>>();
    let mut modules = loader
        .modules
        .values()
        .map(|module| {
            let mut imports = module
                .imports
                .iter()
                .filter_map(|(alias, path)| {
                    module_paths.get(path).map(|path| ProjectImport {
                        alias: alias.clone(),
                        path: path.clone(),
                    })
                })
                .collect::<Vec<_>>();
            imports.sort_by(|left, right| {
                left.alias
                    .cmp(&right.alias)
                    .then_with(|| left.path.cmp(&right.path))
            });
            ProjectModule {
                path: module.relative_path.clone(),
                imports,
            }
        })
        .collect::<Vec<_>>();
    modules.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(LoadedProject {
        program,
        sources: loader.sources,
        modules,
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
    let mut type_sets = HashMap::new();
    for (path, module) in modules {
        let functions = function_visibility(&module.program);
        if path != entry && has_unsupported_import_declarations(&module.program) {
            let span = first_unsupported_import_span(&module.program).unwrap_or_default();
            return Err(ProjectError {
                code: "E-MOD-008",
                message: "imported modules support functions, type aliases, records, tables, tableviews, pages, views, and components; forms, CRUD, APIs, and authentication resources must remain in the entry file".into(),
                path: module.relative_path.clone(),
                span,
                sources: Box::default(),
            });
        }
        function_sets.insert(path.clone(), functions);
        type_sets.insert(path.clone(), type_visibility(&module.program));
    }

    let mut linked = root.program.clone();
    for path in order {
        if path == entry {
            continue;
        }
        let module = modules.get(path).expect("ordered module was loaded");
        let module_name = module_names.get(path).expect("module name exists");
        // A database declaration is project-wide connection configuration.
        // Tables also contribute to the application's shared physical schema;
        // their SQL names are global rather than module-qualified.
        linked.databases.extend(module.program.databases.clone());
        let linker = TypeLinker {
            current_path: path,
            current_name: module_name,
            imports: &module.imports,
            type_sets: &type_sets,
            module_names: &module_names,
            display_path: &module.relative_path,
            is_root: false,
        };
        for mut page in module.program.pages.clone() {
            if let Some(existing) = linked
                .pages
                .iter()
                .find(|existing| page_routes_overlap(&existing.path, &page.path))
            {
                return Err(ProjectError {
                    code: "E-MOD-012",
                    message: format!(
                        "page route `{}` overlaps existing project page `{}`",
                        page.path, existing.path
                    ),
                    path: module.relative_path.clone(),
                    span: page.span,
                    sources: Box::default(),
                });
            }
            for input in &mut page.inputs {
                linker.rewrite_type(&mut input.ty, input.span)?;
            }
            for data in &mut page.data {
                linker.rewrite_type(&mut data.result_type, data.span)?;
            }
            linked.pages.push(page);
        }
        for mut table in module.program.tables.clone() {
            for column in &mut table.columns {
                linker.rewrite_type(&mut column.ty, column.span)?;
            }
            linked.tables.push(table);
        }
        for mut definition in module.program.types.clone() {
            if definition.is_public {
                linker.ensure_exported_type_is_public(&definition.target, definition.span)?;
            }
            linker.rewrite_type(&mut definition.target, definition.span)?;
            definition.name = internal_type_name(module_name, &definition.name);
            linked.types.push(definition);
        }
        for mut record in module.program.records.clone() {
            for field in &mut record.fields {
                if record.is_public {
                    linker.ensure_exported_type_is_public(&field.ty, field.span)?;
                }
                linker.rewrite_type(&mut field.ty, field.span)?;
            }
            record.name = internal_type_name(module_name, &record.name);
            linked.records.push(record);
        }
        for mut function in module.program.functions.clone() {
            linker.rewrite_function_types(&mut function)?;
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
        for mut tableview in module.program.tableviews.clone() {
            if linked
                .tableviews
                .iter()
                .any(|existing| existing.name == tableview.name)
            {
                return Err(ProjectError {
                    code: "E-MOD-011",
                    message: format!(
                        "duplicate tableview `{}` across imported project modules",
                        tableview.name
                    ),
                    path: module.relative_path.clone(),
                    span: tableview.span,
                    sources: Box::default(),
                });
            }
            linker.rewrite_type(&mut tableview.result_type, tableview.span)?;
            linked.tableviews.push(tableview);
        }
        // Views and components are reusable project resources. They are
        // intentionally available by their declared names after import; the
        // current language does not yet define visibility modifiers for them.
        linked.views.extend(module.program.views.clone());
        for mut component in module.program.components.clone() {
            for prop in &mut component.props {
                linker.rewrite_type(&mut prop.ty, prop.span)?;
            }
            linked.components.push(component);
        }
    }

    let root_linker = TypeLinker {
        current_path: entry,
        current_name: &root.relative_path,
        imports: &root.imports,
        type_sets: &type_sets,
        module_names: &module_names,
        display_path: &root.relative_path,
        is_root: true,
    };
    root_linker.rewrite_root_declaration_types(&mut linked, root.source_id)?;
    for function in &mut linked.functions {
        if function.span.source_id != root.source_id {
            continue;
        }
        root_linker.rewrite_function_types(function)?;
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

fn type_visibility(program: &Program) -> HashMap<String, bool> {
    program
        .types
        .iter()
        .map(|definition| (definition.name.clone(), definition.is_public))
        .chain(
            program
                .records
                .iter()
                .map(|record| (record.name.clone(), record.is_public)),
        )
        .collect()
}

fn has_unsupported_import_declarations(program: &Program) -> bool {
    !program.forms.is_empty()
        || !program.cruds.is_empty()
        || !program.auth.is_empty()
        || !program.apis.is_empty()
}

fn page_routes_overlap(left: &str, right: &str) -> bool {
    let left_parts = left
        .trim_matches('/')
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let right_parts = right
        .trim_matches('/')
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    left_parts.len() == right_parts.len()
        && left_parts.iter().zip(right_parts).all(|(left, right)| {
            left == &right
                || left.starts_with('{') && left.ends_with('}')
                || right.starts_with('{') && right.ends_with('}')
        })
}

fn first_unsupported_import_span(program: &Program) -> Option<Span> {
    program
        .forms
        .first()
        .map(|item| item.span)
        .or_else(|| program.cruds.first().map(|item| item.span))
        .or_else(|| program.auth.first().map(|item| item.span))
        .or_else(|| program.apis.first().map(|item| item.span))
}

struct TypeLinker<'a> {
    current_path: &'a Path,
    current_name: &'a str,
    imports: &'a HashMap<String, PathBuf>,
    type_sets: &'a HashMap<PathBuf, HashMap<String, bool>>,
    module_names: &'a HashMap<PathBuf, String>,
    display_path: &'a str,
    is_root: bool,
}

impl TypeLinker<'_> {
    fn rewrite_root_declaration_types(
        &self,
        program: &mut Program,
        source_id: u32,
    ) -> Result<(), ProjectError> {
        for definition in &mut program.types {
            if definition.span.source_id == source_id {
                self.rewrite_type(&mut definition.target, definition.span)?;
            }
        }
        for record in &mut program.records {
            if record.span.source_id == source_id {
                for field in &mut record.fields {
                    self.rewrite_type(&mut field.ty, field.span)?;
                }
            }
        }
        for table in &mut program.tables {
            if table.span.source_id == source_id {
                for column in &mut table.columns {
                    self.rewrite_type(&mut column.ty, column.span)?;
                }
            }
        }
        for page in &mut program.pages {
            if page.span.source_id == source_id {
                for input in &mut page.inputs {
                    self.rewrite_type(&mut input.ty, input.span)?;
                }
                for data in &mut page.data {
                    self.rewrite_type(&mut data.result_type, data.span)?;
                }
            }
        }
        for component in &mut program.components {
            if component.span.source_id == source_id {
                for prop in &mut component.props {
                    self.rewrite_type(&mut prop.ty, prop.span)?;
                }
            }
        }
        for view in &mut program.tableviews {
            if view.span.source_id == source_id {
                self.rewrite_type(&mut view.result_type, view.span)?;
            }
        }
        for form in &mut program.forms {
            for field in &mut form.fields {
                if let Some(ty) = &mut field.ty {
                    self.rewrite_type(ty, field.span)?;
                }
            }
            for action in &mut form.actions {
                self.rewrite_action_types(action)?;
            }
        }
        for crud in &mut program.cruds {
            for action in &mut crud.actions {
                self.rewrite_action_types(action)?;
            }
        }
        for api in &mut program.apis {
            for field in &mut api.input {
                self.rewrite_type(&mut field.ty, field.span)?;
            }
            self.rewrite_type(&mut api.output, api.span)?;
            for error in &mut api.errors {
                if let Some(payload) = &mut error.payload {
                    self.rewrite_type(payload, error.span)?;
                }
            }
        }
        Ok(())
    }

    fn rewrite_action_types(&self, action: &mut FormAction) -> Result<(), ProjectError> {
        for field in &mut action.fields {
            if let Some(ty) = &mut field.ty {
                self.rewrite_type(ty, field.span)?;
            }
        }
        self.rewrite_statements_types(&mut action.statements)
    }

    fn rewrite_statements_types(&self, statements: &mut [Stmt]) -> Result<(), ProjectError> {
        for statement in statements {
            match statement {
                Stmt::Let {
                    ty, value, span, ..
                } => {
                    if let Some(ty) = ty {
                        self.rewrite_type(ty, *span)?;
                    }
                    self.rewrite_expression_types(value)?;
                }
                Stmt::BindOrAssign { value, .. } | Stmt::Expr(value) => {
                    self.rewrite_expression_types(value)?;
                }
                Stmt::Return {
                    value: Some(value), ..
                } => self.rewrite_expression_types(value)?,
                Stmt::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    self.rewrite_expression_types(condition)?;
                    self.rewrite_block_types(then_block)?;
                    if let Some(else_block) = else_block {
                        self.rewrite_block_types(else_block)?;
                    }
                }
                Stmt::While {
                    condition,
                    invariants,
                    body,
                    ..
                } => {
                    self.rewrite_expression_types(condition)?;
                    for invariant in invariants {
                        self.rewrite_expression_types(invariant)?;
                    }
                    self.rewrite_block_types(body)?;
                }
                Stmt::For { iterable, body, .. } => {
                    self.rewrite_expression_types(iterable)?;
                    self.rewrite_block_types(body)?;
                }
                Stmt::Loop {
                    invariants, body, ..
                } => {
                    for invariant in invariants {
                        self.rewrite_expression_types(invariant)?;
                    }
                    self.rewrite_block_types(body)?;
                }
                Stmt::Match { value, arms, .. } => {
                    self.rewrite_expression_types(value)?;
                    for arm in arms {
                        self.rewrite_block_types(&mut arm.body)?;
                    }
                }
                Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
                    self.rewrite_block_types(body)?;
                }
                Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
            }
        }
        Ok(())
    }

    fn ensure_exported_type_is_public(&self, ty: &Type, span: Span) -> Result<(), ProjectError> {
        match ty {
            Type::Named(name) if !name.contains("::") => {
                if self
                    .type_sets
                    .get(self.current_path)
                    .and_then(|types| types.get(name))
                    == Some(&false)
                {
                    return Err(project_error(
                        "E-MOD-007",
                        format!(
                            "public declaration exposes private type or record `{name}`; make the type public or keep it out of the public signature"
                        ),
                        self.display_path,
                        span,
                    ));
                }
            }
            Type::Option(inner) | Type::Array(inner) | Type::HttpResult(inner) => {
                self.ensure_exported_type_is_public(inner, span)?;
            }
            Type::Result(ok, error) | Type::Map(ok, error) => {
                self.ensure_exported_type_is_public(ok, span)?;
                self.ensure_exported_type_is_public(error, span)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn rewrite_type(&self, ty: &mut Type, span: Span) -> Result<(), ProjectError> {
        match ty {
            Type::Named(name) => {
                if let Some((alias, imported_name)) = name.split_once("::") {
                    if imported_name.contains("::") {
                        return Err(project_error(
                            "E-MOD-009",
                            "qualified type references currently support one imported namespace",
                            self.display_path,
                            span,
                        ));
                    }
                    let target = self.imports.get(alias).ok_or_else(|| {
                        project_error(
                            "E-MOD-009",
                            format!("unknown imported module alias `{alias}` in type reference"),
                            self.display_path,
                            span,
                        )
                    })?;
                    let visibility = self
                        .type_sets
                        .get(target)
                        .and_then(|types| types.get(imported_name))
                        .copied();
                    match visibility {
                        None => {
                            return Err(project_error(
                                "E-MOD-009",
                                format!("module `{alias}` has no type or record `{imported_name}`"),
                                self.display_path,
                                span,
                            ));
                        }
                        Some(false) => {
                            return Err(project_error(
                                "E-MOD-007",
                                format!("type or record `{imported_name}` in module `{alias}` is private; declare it with `pub` to use it from another module"),
                                self.display_path,
                                span,
                            ));
                        }
                        Some(true) => {}
                    }
                    let module_name = self
                        .module_names
                        .get(target)
                        .expect("resolved module has a canonical name");
                    *name = internal_type_name(module_name, imported_name);
                } else if !self.is_root
                    && self
                        .type_sets
                        .get(self.current_path)
                        .is_some_and(|types| types.contains_key(name))
                {
                    *name = internal_type_name(self.current_name, name);
                }
            }
            Type::Option(inner) | Type::Array(inner) | Type::HttpResult(inner) => {
                self.rewrite_type(inner, span)?;
            }
            Type::Result(ok, error) | Type::Map(ok, error) => {
                self.rewrite_type(ok, span)?;
                self.rewrite_type(error, span)?;
            }
            Type::Int
            | Type::UInt
            | Type::Float
            | Type::Decimal
            | Type::Bool
            | Type::String
            | Type::Char
            | Type::Bytes
            | Type::Timestamp
            | Type::Date
            | Type::Time
            | Type::Duration
            | Type::Unit
            | Type::Unknown => {}
        }
        Ok(())
    }

    fn rewrite_function_types(&self, function: &mut Function) -> Result<(), ProjectError> {
        for parameter in &mut function.params {
            if function.is_public {
                self.ensure_exported_type_is_public(&parameter.ty, parameter.span)?;
            }
            self.rewrite_type(&mut parameter.ty, parameter.span)?;
        }
        if let Some(return_type) = &mut function.return_type {
            if function.is_public {
                self.ensure_exported_type_is_public(return_type, function.span)?;
            }
            self.rewrite_type(return_type, function.span)?;
        }
        for expression in function
            .requires
            .iter_mut()
            .chain(function.ensures.iter_mut())
        {
            self.rewrite_expression_types(expression)?;
        }
        self.rewrite_block_types(&mut function.body)
    }

    fn rewrite_block_types(&self, block: &mut Block) -> Result<(), ProjectError> {
        self.rewrite_statements_types(&mut block.statements)
    }

    fn rewrite_expression_types(&self, expression: &mut Expr) -> Result<(), ProjectError> {
        match &mut expression.kind {
            ExprKind::Array(values) => {
                for value in values {
                    self.rewrite_expression_types(value)?;
                }
            }
            ExprKind::Map(entries) => {
                for (key, value) in entries {
                    self.rewrite_expression_types(key)?;
                    self.rewrite_expression_types(value)?;
                }
            }
            ExprKind::Record { type_name, fields } => {
                let mut ty = Type::Named(type_name.clone());
                self.rewrite_type(&mut ty, expression.span)?;
                let Type::Named(canonical_name) = ty else {
                    unreachable!("named record type remains a named type")
                };
                *type_name = canonical_name;
                for (_, value) in fields {
                    self.rewrite_expression_types(value)?;
                }
            }
            ExprKind::Index { target, index } => {
                self.rewrite_expression_types(target)?;
                self.rewrite_expression_types(index)?;
            }
            ExprKind::Field { target, .. } | ExprKind::Await(target) => {
                self.rewrite_expression_types(target)?;
            }
            ExprKind::Call {
                type_args, args, ..
            } => {
                for ty in type_args {
                    self.rewrite_type(ty, expression.span)?;
                }
                for argument in args {
                    self.rewrite_expression_types(argument)?;
                }
            }
            ExprKind::Unary { expr, .. } => self.rewrite_expression_types(expr)?,
            ExprKind::Binary { left, right, .. } => {
                self.rewrite_expression_types(left)?;
                self.rewrite_expression_types(right)?;
            }
            ExprKind::Sql { result_type, .. } => {
                self.rewrite_type(result_type, expression.span)?;
            }
            ExprKind::Int(_)
            | ExprKind::UInt(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::String(_)
            | ExprKind::Char(_)
            | ExprKind::Variable(_) => {}
        }
        Ok(())
    }
}

fn internal_type_name(module_path: &str, name: &str) -> String {
    format!("{module_path}::{name}")
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
    use zelyra_database::build_schema;
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
        assert_eq!(
            loaded.modules,
            [
                ProjectModule {
                    path: "main.zyl".into(),
                    imports: vec![ProjectImport {
                        alias: "math".into(),
                        path: "src/math.zyl".into()
                    }]
                },
                ProjectModule {
                    path: "src/math.zyl".into(),
                    imports: vec![]
                }
            ]
        );
        cleanup(&directory);
    }

    #[test]
    fn reports_transitive_module_graph_in_stable_path_order() {
        let directory = project(&[
            (
                "main.zyl",
                "import \"src/invoice.zyl\" as invoice\nfn main() { print(invoice::total()) }\n",
            ),
            (
                "src/invoice.zyl",
                "import \"src/money.zyl\" as money\npub fn total() -> Int { return money::amount() }\n",
            ),
            (
                "src/money.zyl",
                "pub fn amount() -> Int { return 25 }\n",
            ),
        ]);
        let entry = directory.join("main.zyl");
        let loaded = load(entry.to_str().unwrap()).unwrap();
        let repeated = load(entry.to_str().unwrap()).unwrap();
        assert_eq!(loaded.modules, repeated.modules);
        assert_eq!(
            loaded
                .modules
                .iter()
                .map(|module| module.path.as_str())
                .collect::<Vec<_>>(),
            ["main.zyl", "src/invoice.zyl", "src/money.zyl"]
        );
        assert_eq!(loaded.modules[1].imports[0].alias, "money");
        assert_eq!(loaded.modules[1].imports[0].path, "src/money.zyl");
        cleanup(&directory);
    }

    #[test]
    fn imports_public_records_and_aliases_across_transitive_modules() {
        let directory = project(&[
            (
                "main.zyl",
                "import \"src/invoice.zyl\" as invoice\nimport \"src/money.zyl\" as money\ntable invoices { id: money::InvoiceId primary auto }\nfn main() { print(invoice::amount().value) }\n",
            ),
            (
                "src/invoice.zyl",
                "import \"src/money.zyl\" as money\npub fn amount() -> money::Amount { return money::Amount { value: 25 } }\n",
            ),
            (
                "src/money.zyl",
                "pub type InvoiceId = Id\npub struct Amount { value: Int }\n",
            ),
        ]);
        let loaded = load(directory.join("main.zyl").to_str().unwrap()).unwrap();
        assert!(
            check(&loaded.program).is_ok(),
            "{:?}",
            check(&loaded.program)
        );
        assert!(build_schema(&loaded.program).is_ok());
        assert_eq!(execute(&loaded.program).unwrap(), ["25"]);
        assert!(loaded
            .program
            .records
            .iter()
            .any(|record| record.name == "src/money.zyl::Amount"));
        cleanup(&directory);
    }

    #[test]
    fn rejects_private_types_and_public_signatures_that_leak_them() {
        let private_use = project(&[
            (
                "main.zyl",
                "import \"src/models.zyl\" as models\nfn accept(value: models::Secret) {}\nfn main() {}\n",
            ),
            (
                "src/models.zyl",
                "struct Secret { value: Int }\npub struct Public { value: Int }\n",
            ),
        ]);
        let error = load(private_use.join("main.zyl").to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E-MOD-007");
        assert!(error.message.contains("private"));
        cleanup(&private_use);

        let leaked = project(&[
            (
                "main.zyl",
                "import \"src/models.zyl\" as models\nfn main() { models::create() }\n",
            ),
            (
                "src/models.zyl",
                "struct Secret { value: Int }\npub fn create() -> Secret { return Secret { value: 1 } }\n",
            ),
        ]);
        let error = load(leaked.join("main.zyl").to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E-MOD-007");
        assert!(error.message.contains("exposes private type"));
        cleanup(&leaked);

        let leaked_field = project(&[
            (
                "main.zyl",
                "import \"src/models.zyl\" as models\nfn main() { models::empty() }\n",
            ),
            (
                "src/models.zyl",
                "struct Secret { value: Int }\npub struct Public { secret: Secret }\npub fn empty() {}\n",
            ),
        ]);
        let error = load(leaked_field.join("main.zyl").to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E-MOD-007");
        assert!(error.message.contains("exposes private type"));
        cleanup(&leaked_field);
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
    fn imported_files_still_reject_forms_and_other_unsupported_resources() {
        let directory = project(&[
            (
                "main.zyl",
                "import \"src/domain.zyl\" as domain\nfn main() {}\n",
            ),
            (
                "src/domain.zyl",
                "form Contact -> customers { fields { name } }\n",
            ),
        ]);
        let error = load(directory.join("main.zyl").to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E-MOD-008");
        assert!(error.message.contains("forms"));
        cleanup(&directory);
    }

    #[test]
    fn duplicate_page_routes_across_modules_report_the_imported_source() {
        let directory = project(&[
            (
                "main.zyl",
                "import \"src/reports.zyl\" as reports\npage \"/reports/{id}\" { html { <h1>Root</h1> } }\nfn main() {}\n",
            ),
            (
                "src/reports.zyl",
                "page \"/reports/{slug}\" { html { <h1>Imported</h1> } }\n",
            ),
        ]);
        let error = load(directory.join("main.zyl").to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "E-MOD-012");
        assert_eq!(error.path, "src/reports.zyl");
        assert!(error.message.contains("overlaps existing project page"));
        cleanup(&directory);
    }

    #[test]
    fn imported_database_definition_configures_the_composed_project() {
        let directory = project(&[
            (
                "main.zyl",
                "import \"src/database.zyl\" as storage\nfn main() {}\n",
            ),
            (
                "src/database.zyl",
                "database main { engine: mariadb database: \"invoices\" }\n",
            ),
        ]);
        let loaded = load(directory.join("main.zyl").to_str().unwrap()).unwrap();
        let schema = build_schema(&loaded.program).unwrap();
        assert_eq!(schema.database.as_ref().unwrap().name, "main");
        assert_eq!(
            schema.database.as_ref().unwrap().database.as_deref(),
            Some("invoices")
        );
        cleanup(&directory);
    }

    #[test]
    fn imported_tables_join_the_project_schema_and_resolve_local_aliases() {
        let directory = project(&[
            (
                "main.zyl",
                "import \"src/invoices.zyl\" as invoices\nfn main() {}\n",
            ),
            (
                "src/invoices.zyl",
                "type Cents = Int\ntable invoices { id: Id primary auto total: Cents required }\n",
            ),
        ]);
        let loaded = load(directory.join("main.zyl").to_str().unwrap()).unwrap();
        let schema = build_schema(&loaded.program).unwrap();
        assert_eq!(schema.tables.len(), 1);
        assert_eq!(schema.tables[0].name, "invoices");
        assert_eq!(schema.tables[0].columns[1].sql_type, "BIGINT");
        cleanup(&directory);
    }

    #[test]
    fn imported_table_name_collisions_are_reported() {
        let directory = project(&[
            (
                "main.zyl",
                "import \"src/invoices.zyl\" as invoices\ntable invoices { id: Id primary auto }\nfn main() {}\n",
            ),
            (
                "src/invoices.zyl",
                "table invoices { id: Id primary auto }\n",
            ),
        ]);
        let loaded = load(directory.join("main.zyl").to_str().unwrap()).unwrap();
        let errors = build_schema(&loaded.program).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message == "duplicate table definition invoices"));
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
