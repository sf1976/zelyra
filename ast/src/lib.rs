use std::fmt;

pub mod decimal;
pub mod json;
pub mod temporal;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
    pub source_id: u32,
}

impl Span {
    pub const fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self {
            start,
            end,
            line,
            column,
            source_id: 0,
        }
    }

    pub const fn with_source_id(mut self, source_id: u32) -> Self {
        self.source_id = source_id;
        self
    }

    pub fn join(self, other: Span) -> Self {
        Self {
            start: self.start,
            end: other.end,
            line: self.line,
            column: self.column,
            source_id: self.source_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Int,
    UInt,
    Float,
    Decimal,
    Bool,
    String,
    Char,
    Bytes,
    Json,
    Timestamp,
    Date,
    Time,
    Duration,
    Unit,
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Array(Box<Type>),
    Tuple(Vec<Type>),
    Set(Box<Type>),
    Map(Box<Type>, Box<Type>),
    HttpResult(Box<Type>),
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
        capabilities: Vec<String>,
    },
    Applied {
        name: String,
        args: Vec<Type>,
    },
    Named(String),
    Unknown,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Type::Int => "Int",
            Type::UInt => "UInt",
            Type::Float => "Float",
            Type::Decimal => "Decimal",
            Type::Bool => "Bool",
            Type::String => "String",
            Type::Char => "Char",
            Type::Bytes => "Bytes",
            Type::Json => "Json",
            Type::Timestamp => "Timestamp",
            Type::Date => "Date",
            Type::Time => "Time",
            Type::Duration => "Duration",
            Type::Unit => "Unit",
            Type::Option(inner) => return write!(f, "Option<{inner}>"),
            Type::Result(ok, err) => return write!(f, "Result<{ok}, {err}>"),
            Type::Array(inner) => return write!(f, "{inner}[]"),
            Type::Tuple(items) => {
                return write!(
                    f,
                    "Tuple<{}>",
                    items
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Type::Set(inner) => return write!(f, "Set<{inner}>"),
            Type::Map(key, value) => return write!(f, "Map<{key}, {value}>"),
            Type::HttpResult(inner) => return write!(f, "HttpResult<{inner}>"),
            Type::Function {
                params,
                return_type,
                capabilities,
            } => {
                let params = params
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "Function<({params}) -> {return_type}>")?;
                if !capabilities.is_empty() {
                    write!(f, " uses {{{}}}", capabilities.join(", "))?;
                }
                return Ok(());
            }
            Type::Applied { name, args } => {
                write!(
                    f,
                    "{name}<{}>",
                    args.iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
                return Ok(());
            }
            Type::Named(name) => name,
            Type::Unknown => "unknown",
        };
        write!(f, "{name}")
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    pub imports: Vec<ImportDef>,
    pub databases: Vec<DatabaseDef>,
    pub tables: Vec<TableDef>,
    pub types: Vec<TypeDef>,
    pub records: Vec<RecordDef>,
    pub enums: Vec<EnumDef>,
    pub views: Vec<ViewDef>,
    pub components: Vec<ComponentDef>,
    pub pages: Vec<PageDef>,
    pub tableviews: Vec<TableViewDef>,
    pub forms: Vec<FormDef>,
    pub cruds: Vec<CrudDef>,
    pub auth: Vec<AuthDef>,
    pub apis: Vec<ApiDef>,
    pub commands: Vec<CommandDef>,
    pub schedules: Vec<ScheduleDef>,
    pub functions: Vec<Function>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableValidationFunction {
    pub table_index: usize,
    pub rule_index: usize,
    pub function_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComputedColumnFunction {
    pub table_index: usize,
    pub column_index: usize,
    pub function_name: String,
    pub input_columns: Vec<String>,
}

/// Expands declared computed fields into ordinary private functions so the
/// normal type and effect checker validates their expressions.
pub fn build_computed_column_functions(
    program: &Program,
) -> (Vec<Function>, Vec<ComputedColumnFunction>) {
    let mut names = program
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect::<std::collections::HashSet<_>>();
    let mut functions = Vec::new();
    let mut mappings = Vec::new();
    for (table_index, table) in program.tables.iter().enumerate() {
        for (column_index, column) in table.columns.iter().enumerate() {
            let Some(expression) = &column.computed else {
                continue;
            };
            let base_name = format!("__zelyra_computed_{table_index}_{column_index}");
            let mut function_name = base_name.clone();
            let mut suffix = 0usize;
            while !names.insert(function_name.clone()) {
                suffix += 1;
                function_name = format!("{base_name}_{suffix}");
            }
            let referenced_names = computed_expression_names(expression);
            let input_columns = table
                .columns
                .iter()
                .filter(|candidate| {
                    !candidate.auto && referenced_names.contains(candidate.name.as_str())
                })
                .map(|candidate| candidate.name.clone())
                .collect::<Vec<_>>();
            functions.push(Function {
                name: function_name.clone(),
                is_public: false,
                type_params: Vec::new(),
                params: table
                    .columns
                    .iter()
                    .filter(|candidate| input_columns.contains(&candidate.name))
                    .map(|candidate| Param {
                        name: candidate.name.clone(),
                        ty: candidate.ty.clone(),
                        span: candidate.span,
                    })
                    .collect(),
                return_type: Some(column.ty.clone()),
                capabilities: Vec::new(),
                requires: Vec::new(),
                ensures: Vec::new(),
                body: Block {
                    statements: vec![Stmt::Return {
                        value: Some(expression.clone()),
                        span: expression.span,
                    }],
                    span: expression.span,
                },
                span: expression.span,
            });
            mappings.push(ComputedColumnFunction {
                table_index,
                column_index,
                function_name,
                input_columns,
            });
        }
    }
    (functions, mappings)
}

/// Finds dependency cycles among computed columns. A computed column may
/// depend on ordinary inputs and on earlier computed columns, but not on
/// itself or a cycle of other computed columns.
pub fn computed_column_dependency_cycles(program: &Program) -> Vec<(Span, String)> {
    let (_, mappings) = build_computed_column_functions(program);
    let mut cycles = Vec::new();
    for (table_index, table) in program.tables.iter().enumerate() {
        let computed = mappings
            .iter()
            .filter(|mapping| mapping.table_index == table_index)
            .map(|mapping| (mapping.column_index, mapping))
            .collect::<std::collections::HashMap<_, _>>();
        let mut states = vec![0_u8; table.columns.len()];
        let mut stack = Vec::new();
        fn visit(
            index: usize,
            table: &TableDef,
            computed: &std::collections::HashMap<usize, &ComputedColumnFunction>,
            states: &mut [u8],
            stack: &mut Vec<usize>,
            cycles: &mut Vec<(Span, String)>,
        ) {
            if states[index] == 2 {
                return;
            }
            if states[index] == 1 {
                let start = stack
                    .iter()
                    .position(|candidate| *candidate == index)
                    .unwrap_or(0);
                let names = stack[start..]
                    .iter()
                    .filter_map(|column| table.columns.get(*column))
                    .map(|column| column.name.as_str())
                    .chain(table.columns.get(index).map(|column| column.name.as_str()))
                    .collect::<Vec<_>>()
                    .join(" -> ");
                cycles.push((table.columns[index].span, names));
                return;
            }
            states[index] = 1;
            stack.push(index);
            if let Some(mapping) = computed.get(&index) {
                for dependency in &mapping.input_columns {
                    if let Some(dependency_index) = table
                        .columns
                        .iter()
                        .position(|column| column.name == *dependency)
                        .filter(|dependency_index| computed.contains_key(dependency_index))
                    {
                        visit(dependency_index, table, computed, states, stack, cycles);
                    }
                }
            }
            stack.pop();
            states[index] = 2;
        }
        for index in computed.keys().copied() {
            visit(
                index,
                table,
                &computed,
                &mut states,
                &mut stack,
                &mut cycles,
            );
        }
    }
    cycles
}

fn computed_expression_names(expression: &Expr) -> std::collections::HashSet<&str> {
    fn visit<'a>(expression: &'a Expr, names: &mut std::collections::HashSet<&'a str>) {
        match &expression.kind {
            ExprKind::Variable(name) => {
                names.insert(name);
            }
            ExprKind::Array(items) => {
                for item in items {
                    visit(item, names);
                }
            }
            ExprKind::Map(entries) => {
                for (key, value) in entries {
                    visit(key, names);
                    visit(value, names);
                }
            }
            ExprKind::Record { fields, .. } => {
                for (_, value) in fields {
                    visit(value, names);
                }
            }
            ExprKind::Lambda { .. } | ExprKind::FunctionValue(_) => {}
            ExprKind::Index { target, index } => {
                visit(target, names);
                visit(index, names);
            }
            ExprKind::Field { target, .. } => visit(target, names),
            ExprKind::Call { args, .. } => {
                for argument in args {
                    visit(argument, names);
                }
            }
            ExprKind::Unary { expr, .. } | ExprKind::Await(expr) => visit(expr, names),
            ExprKind::Binary { left, right, .. } => {
                visit(left, names);
                visit(right, names);
            }
            ExprKind::Match { value, arms } => {
                visit(value, names);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        visit(guard, names);
                    }
                    visit(&arm.value, names);
                }
            }
            ExprKind::Sql { .. }
            | ExprKind::Int(_)
            | ExprKind::UInt(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::String(_)
            | ExprKind::Char(_) => {}
        }
    }
    let mut names = std::collections::HashSet::new();
    visit(expression, &mut names);
    names
}

/// Expands table validation expressions to ordinary private Boolean functions
/// so the compiler's normal scope, type, effect, and diagnostic checks remain
/// authoritative. Returns the lowered program and the stable mapping needed
/// by web runtimes to evaluate the checked rules.
pub fn lower_table_validation_functions(
    program: &Program,
) -> (Program, Vec<TableValidationFunction>) {
    let mut lowered = program.clone();
    let (functions, mappings) = build_table_validation_functions(program);
    lowered.functions.extend(functions);
    (lowered, mappings)
}

/// Ownership-taking counterpart for runtime preparation, which avoids cloning
/// the whole source program when it has already been transferred to the web
/// application.
pub fn lower_table_validation_functions_owned(
    mut program: Program,
) -> (Program, Vec<TableValidationFunction>) {
    let (functions, mappings) = build_table_validation_functions(&program);
    program.functions.extend(functions);
    (program, mappings)
}

/// Builds the checked Boolean functions for table invariants without cloning
/// unrelated program bodies. The type checker uses this to avoid deep-cloning
/// large ASTs merely to validate a table rule.
pub fn build_table_validation_functions(
    program: &Program,
) -> (Vec<Function>, Vec<TableValidationFunction>) {
    let mut names = program
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect::<std::collections::HashSet<_>>();
    let mut functions = Vec::new();
    let mut mappings = Vec::new();
    for (table_index, table) in program.tables.iter().enumerate() {
        for (rule_index, rule) in table.validations.iter().enumerate() {
            let base_name = format!("__zelyra_table_validation_{table_index}_{rule_index}");
            let mut function_name = base_name.clone();
            let mut suffix = 0usize;
            while !names.insert(function_name.clone()) {
                suffix += 1;
                function_name = format!("{base_name}_{suffix}");
            }
            functions.push(Function {
                name: function_name.clone(),
                is_public: false,
                type_params: Vec::new(),
                params: table
                    .columns
                    .iter()
                    .filter(|column| !column.auto)
                    .map(|column| Param {
                        name: column.name.clone(),
                        ty: table_validation_storage_type(program, &column.ty),
                        span: column.span,
                    })
                    .collect(),
                return_type: Some(Type::Bool),
                capabilities: Vec::new(),
                requires: Vec::new(),
                ensures: Vec::new(),
                body: Block {
                    statements: vec![Stmt::Return {
                        value: Some(rule.condition.clone()),
                        span: rule.span,
                    }],
                    span: rule.span,
                },
                span: rule.span,
            });
            mappings.push(TableValidationFunction {
                table_index,
                rule_index,
                function_name,
            });
        }
    }
    (functions, mappings)
}

/// Generated validation functions receive database storage values from the
/// web layer. Relation fields and `Id` aliases are stored as integer keys;
/// retain their source-level type checking on the original validation while
/// giving the private runtime bridge the matching storage type.
fn table_validation_storage_type(program: &Program, ty: &Type) -> Type {
    match ty {
        Type::Named(name) if table_identity_type(program, name) => Type::Int,
        Type::Option(inner) => {
            Type::Option(Box::new(table_validation_storage_type(program, inner)))
        }
        Type::Array(inner) => Type::Array(Box::new(table_validation_storage_type(program, inner))),
        other => other.clone(),
    }
}

fn table_identity_type(program: &Program, name: &str) -> bool {
    table_identity_type_inner(program, name, &mut std::collections::HashSet::new())
}

fn table_identity_type_inner(
    program: &Program,
    name: &str,
    visited: &mut std::collections::HashSet<String>,
) -> bool {
    let name = name.rsplit("::").next().unwrap_or(name);
    if name == "Id" {
        return true;
    }
    if !visited.insert(name.to_owned()) {
        return false;
    }
    if let Some(alias) = program.types.iter().find(|alias| alias.name == name) {
        return table_identity_type_for_type(program, &alias.target, visited);
    }
    let snake = name.to_ascii_lowercase();
    if program.tables.iter().any(|table| table.name == snake) {
        return true;
    }
    let plural = if snake.ends_with('y') {
        format!("{}ies", &snake[..snake.len() - 1])
    } else {
        format!("{snake}s")
    };
    program.tables.iter().any(|table| table.name == plural)
}

fn table_identity_type_for_type(
    program: &Program,
    ty: &Type,
    visited: &mut std::collections::HashSet<String>,
) -> bool {
    match ty {
        Type::Named(name) => table_identity_type_inner(program, name, visited),
        Type::Option(inner) | Type::Array(inner) => {
            table_identity_type_for_type(program, inner, visited)
        }
        _ => false,
    }
}

/// A named process entry that can be inspected and invoked by the Zelyra CLI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandDef {
    pub name: String,
    pub handler: String,
    pub description: Option<String>,
    pub span: Span,
}

/// A host-triggered schedule for a declared Zelyra command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleDef {
    pub name: String,
    pub command: String,
    pub cron: String,
    pub timezone: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct EnumDef {
    pub name: String,
    pub is_public: bool,
    pub type_params: Vec<String>,
    pub variants: Vec<String>,
    /// Payload field types keyed by variant name; variants absent from this
    /// list are unit variants.
    pub variant_payloads: Vec<(String, Vec<Type>)>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportDef {
    /// Project-relative `.zyl` path or a local package specifier such as `pkg:math`.
    pub path: String,
    /// The local namespace used to qualify public declarations.
    pub alias: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageDef {
    pub path: String,
    pub html: String,
    pub view: Option<String>,
    /// Literal or template-expression arguments supplied to the page layout.
    pub view_props: Vec<PageViewProp>,
    pub content_type: Option<String>,
    pub inputs: Vec<PageInputDef>,
    pub page_size: Option<u32>,
    pub sort: Vec<String>,
    pub search: Vec<String>,
    pub filters: Vec<String>,
    pub data: Vec<PageDataDef>,
    pub requires_auth: bool,
    pub permissions: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageViewProp {
    pub name: String,
    pub value: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageInputDef {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageDataDef {
    pub name: String,
    pub result_type: Type,
    pub query: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewDef {
    pub name: String,
    pub is_public: bool,
    pub props: Vec<ComponentProp>,
    pub html: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentDef {
    pub name: String,
    pub is_public: bool,
    pub props: Vec<ComponentProp>,
    pub html: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentProp {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TableViewDef {
    pub name: String,
    pub layout: Option<String>,
    pub result_type: Type,
    pub source: String,
    pub columns: Vec<String>,
    /// Optional safe content rendered around the generated query/table view.
    pub view: TableViewViewDef,
    pub filters: Vec<String>,
    pub searchable: bool,
    pub sortable: bool,
    pub page_size: Option<u32>,
    pub requires_auth: bool,
    pub permissions: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TableViewViewDef {
    pub before_content: Option<String>,
    pub after_content: Option<String>,
    /// Explicit result fields available only to the row templates below.
    pub row_fields: Vec<String>,
    pub column_content: Vec<TableViewColumnContentDef>,
    pub row_actions: Option<TableViewRowActionsDef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableViewColumnContentDef {
    /// Name of a declared display column whose cell receives this content.
    pub column: String,
    /// Lexical result-row binding available only inside this fragment.
    pub item: String,
    pub html: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableViewRowActionsDef {
    /// Lexical row binding available only inside the action fragment.
    pub item: String,
    /// CRUD model whose per-record edit/delete policies govern these slots.
    pub crud_model: Option<String>,
    /// Legacy unclassified action content, retained for compatibility.
    pub html: String,
    /// Content rendered only when the linked CRUD edit policy allows the row.
    pub edit_html: Option<String>,
    /// Content rendered only when the linked CRUD delete policy allows the row.
    pub delete_html: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct FormDef {
    pub name: String,
    pub table: Option<String>,
    /// Optional user-facing title rendered by the native form page header.
    pub title: Option<String>,
    /// Optional short description rendered below the form title.
    pub description: Option<String>,
    /// Optional explicit browser route for a standalone or table-backed form.
    /// When absent, the CLI keeps the conventional `/forms/<name>` route.
    pub path: Option<String>,
    /// Render only on POST so a checked page can own the GET side of the same
    /// path. This form still provides the CSRF-protected mutation endpoint.
    pub post_only: bool,
    /// Optional read query checked on GET before any form is rendered.
    pub verify: Option<Expr>,
    /// Explicit bearer-token form for a single tenant-owned row. The runtime
    /// derives the row and tenant scope from a globally unique token hash.
    pub public_token: Option<PublicTokenFormDef>,
    pub fields: Vec<FormField>,
    pub actions: Vec<FormAction>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct PublicTokenFormDef {
    /// Globally unique SHA-256 hex digest column stored by the target table.
    pub hash_field: String,
    /// Nullable timestamp field set once by the generated action.
    pub timestamp_field: String,
    pub span: Span,
}

/// Compiler-checked metadata for a single uploaded file input. The client
/// filename and MIME header are untrusted and are not represented here as
/// storage paths or trusted content types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileUploadOptions {
    pub max_size_bytes: u64,
    pub allowed_types: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormField {
    pub name: String,
    pub ty: Option<Type>,
    pub label: Option<String>,
    pub placeholder: Option<String>,
    pub required: bool,
    /// Minimum character count for String and Email inputs.
    pub min_length: Option<u32>,
    pub max: Option<u32>,
    /// Inclusive numeric bounds for numeric inputs, written as decimal text
    /// so exact Decimal values are not rounded through binary floats.
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    /// Explicit allowed values for a static select; values are validated server-side.
    pub choices: Vec<String>,
    pub widget: Option<String>,
    /// Optional visual variant; it does not alter validation state.
    pub appearance: Option<String>,
    /// Present only when `ty` is the transient `File` form-input type.
    pub file_upload: Option<FileUploadOptions>,
    pub readonly: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct FormAction {
    pub name: String,
    pub label: Option<String>,
    pub icon: Option<String>,
    pub confirm: Option<String>,
    pub confirm_page: Option<CrudConfirmViewDef>,
    pub success_page: Option<CrudActionNoticeDef>,
    pub error_page: Option<CrudActionNoticeDef>,
    pub fields: Vec<FormField>,
    pub requires_auth: bool,
    pub permissions: Vec<String>,
    /// Typed Zelyra function used by a standalone form action.
    pub handler: Option<String>,
    /// Explicit language capabilities required by the typed execution block.
    pub capabilities: Vec<String>,
    /// Require the first SQL statement to be a SELECT and to return a row in
    /// the same transaction before the remaining action statements run.
    pub require_match: bool,
    /// Legacy action statements remain accepted for source compatibility.
    pub statements: Vec<Stmt>,
    /// Explicit, compiler-checked execution body for domain actions.
    pub execute_body: Option<Block>,
    /// Optional precondition evaluated against the locked target row.
    pub guard: Option<CrudActionGuardDef>,
    pub success: Option<String>,
    pub redirect: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct CrudActionGuardDef {
    pub condition: Expr,
    pub message: String,
    pub span: Span,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrudConfirmViewDef {
    pub title: Option<String>,
    pub message: Option<String>,
    pub submit: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrudActionNoticeDef {
    pub title: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CrudDef {
    pub name: String,
    pub table: String,
    /// Enables request-scoped tenant isolation on this generated CRUD resource.
    pub tenant_column: Option<String>,
    /// Restricts generated CRUD to the authenticated user's records.
    pub recipient_column: Option<String>,
    /// Nullable timestamp that marks an item as read; generated lists show an unread count.
    pub unread_column: Option<String>,
    /// Optional expiry timestamp; expired rows are hidden from generated lists.
    pub expires_column: Option<String>,
    pub title: Option<String>,
    /// Optional named view used as the outer layout for generated CRUD pages.
    pub layout: Option<String>,
    /// Static/custom content supplied to named slots of the outer CRUD layout.
    pub layout_slots: Vec<CrudLayoutSlotDef>,
    pub list: Vec<String>,
    pub search: Vec<String>,
    pub filters: Vec<String>,
    pub view: CrudViewDef,
    pub soft_delete: Option<CrudSoftDeleteDef>,
    pub requires_auth: bool,
    /// Legacy/default permission(s), used for list/detail and as the fallback
    /// for actions without an explicit permission scope.
    pub permissions: Vec<String>,
    pub create_permissions: Vec<String>,
    pub edit_permissions: Vec<String>,
    pub delete_permissions: Vec<String>,
    /// Per-record edit/delete permissions that additionally require a pure,
    /// type-checked Boolean condition over the table row.
    pub conditional_permissions: Vec<CrudConditionalPermissionDef>,
    /// Lifecycle conditions that prevent edits or deletes when true.
    pub guards: Vec<CrudGuardDef>,
    /// Typed direct child collections rendered and saved by generated CRUD.
    pub children: Vec<CrudChildRelationDef>,
    pub actions: Vec<FormAction>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct CrudChildRelationDef {
    /// Collection name used for labels and form grouping.
    pub name: String,
    /// Child model/table type as written in the declaration.
    pub model: String,
    /// Child-side foreign-key field referencing the CRUD parent.
    pub foreign_key: String,
    /// Optional stable ordering field and direction.
    pub order_by: Option<String>,
    pub descending: bool,
    /// Optional assertion that must agree with the schema foreign-key action.
    pub on_delete: Option<ReferentialAction>,
    /// Parent columns recalculated from child rows after save.
    pub aggregates: Vec<CrudAggregateDef>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct CrudAggregateDef {
    pub target: String,
    /// Restricted to pure arithmetic over child fields by CRUD validation.
    pub expression: Expr,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrudRecordAction {
    Edit,
    Delete,
}

#[derive(Clone, Debug)]
pub struct CrudConditionalPermissionDef {
    pub action: CrudRecordAction,
    pub permission: String,
    pub condition: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct CrudGuardDef {
    pub action: CrudRecordAction,
    pub condition: Expr,
    pub message: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrudPolicyFunction {
    pub crud_index: usize,
    pub policy_index: usize,
    pub is_guard: bool,
    pub function_name: String,
}

/// Names and synthetic row/session types used to typecheck and execute one
/// typed CRUD action. Both the CLI validator and web runtime use this mapping
/// so their private wrapper names and signatures cannot drift apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrudActionFunction {
    pub crud_index: usize,
    pub action_index: usize,
    pub row_type_name: String,
    pub session_type_name: String,
    pub execute_function: Option<String>,
    pub guard_function: Option<String>,
}

/// Builds compiler-checkable wrappers for typed CRUD action bodies.
/// `row` has exactly the columns in the CRUD table. Session identity fields
/// are nominal IDs, optional when no corresponding context is available.
pub fn build_crud_action_functions(program: &Program) -> (Vec<Function>, Vec<CrudActionFunction>) {
    let mut names = program
        .functions
        .iter()
        .map(|function| function.name.clone())
        .chain(program.records.iter().map(|record| record.name.clone()))
        .collect::<std::collections::HashSet<_>>();
    let mut functions = Vec::new();
    let mut mappings = Vec::new();

    for (crud_index, crud) in program.cruds.iter().enumerate() {
        if !program.tables.iter().any(|table| table.name == crud.table) {
            continue;
        }
        for (action_index, action) in crud.actions.iter().enumerate() {
            if action.execute_body.is_none() && action.guard.is_none() {
                continue;
            }
            let row_type_name = unique_crud_internal_name(
                format!("__ZelyraCrudActionRow_{crud_index}_{action_index}"),
                &mut names,
            );
            let session_type_name = unique_crud_internal_name(
                format!("__ZelyraCrudActionSession_{crud_index}_{action_index}"),
                &mut names,
            );
            let row_type = Type::Named(row_type_name.clone());
            let session_type = Type::Named(session_type_name.clone());
            let params = vec![
                Param {
                    name: "row".into(),
                    ty: row_type,
                    span: action.span,
                },
                Param {
                    name: "session".into(),
                    ty: session_type,
                    span: action.span,
                },
            ];
            let execute_function = action.execute_body.as_ref().map(|body| {
                let name = unique_crud_internal_name(
                    format!("__zelyra_crud_action_{crud_index}_{action_index}"),
                    &mut names,
                );
                functions.push(Function {
                    name: name.clone(),
                    is_public: false,
                    type_params: Vec::new(),
                    params: params.clone(),
                    return_type: Some(Type::Unit),
                    capabilities: action.capabilities.clone(),
                    requires: Vec::new(),
                    ensures: Vec::new(),
                    body: body.clone(),
                    span: action.span,
                });
                name
            });
            let guard_function = action.guard.as_ref().map(|guard| {
                let name = unique_crud_internal_name(
                    format!("__zelyra_crud_action_guard_{crud_index}_{action_index}"),
                    &mut names,
                );
                functions.push(Function {
                    name: name.clone(),
                    is_public: false,
                    type_params: Vec::new(),
                    params,
                    return_type: Some(Type::Bool),
                    capabilities: Vec::new(),
                    requires: Vec::new(),
                    ensures: Vec::new(),
                    body: Block {
                        statements: vec![Stmt::Return {
                            value: Some(guard.condition.clone()),
                            span: guard.span,
                        }],
                        span: guard.span,
                    },
                    span: guard.span,
                });
                name
            });
            mappings.push(CrudActionFunction {
                crud_index,
                action_index,
                row_type_name,
                session_type_name,
                execute_function,
                guard_function,
            });
        }
    }
    (functions, mappings)
}

fn unique_crud_internal_name(
    base: String,
    names: &mut std::collections::HashSet<String>,
) -> String {
    let mut name = base.clone();
    let mut suffix = 0usize;
    while !names.insert(name.clone()) {
        suffix += 1;
        name = format!("{base}_{suffix}");
    }
    name
}

/// Lowers typed CRUD action signatures into ordinary private language
/// declarations. These declarations are compiler/runtime scaffolding only.
pub fn lower_crud_action_functions_owned(
    mut program: Program,
) -> (Program, Vec<CrudActionFunction>) {
    let (functions, mappings) = build_crud_action_functions(&program);
    for mapping in &mappings {
        let crud = &program.cruds[mapping.crud_index];
        let table = program
            .tables
            .iter()
            .find(|table| table.name == crud.table)
            .unwrap();
        program.records.push(RecordDef {
            name: mapping.row_type_name.clone(),
            is_public: false,
            is_class: false,
            type_params: Vec::new(),
            fields: table
                .columns
                .iter()
                .map(|column| RecordField {
                    name: column.name.clone(),
                    ty: column.ty.clone(),
                    span: column.span,
                })
                .collect(),
            span: crud.actions[mapping.action_index].span,
        });
        let action = &crud.actions[mapping.action_index];
        let tenant_type = table
            .tenant_scope
            .as_ref()
            .map(|scope| scope.tenant_type.clone())
            .unwrap_or_else(|| Type::Named("Id".into()));
        program.records.push(RecordDef {
            name: mapping.session_type_name.clone(),
            is_public: false,
            is_class: false,
            type_params: Vec::new(),
            fields: vec![
                RecordField {
                    name: "tenant_id".into(),
                    ty: Type::Option(Box::new(tenant_type)),
                    span: action.span,
                },
                RecordField {
                    name: "user_id".into(),
                    ty: Type::Option(Box::new(Type::Named("Id".into()))),
                    span: action.span,
                },
            ],
            span: action.span,
        });
    }
    program.functions.extend(functions);
    (program, mappings)
}

/// Builds ordinary private predicates for row-conditioned CRUD policies.
/// Normal compiler scope, type, and effect checking remains authoritative.
pub fn build_crud_policy_functions(program: &Program) -> (Vec<Function>, Vec<CrudPolicyFunction>) {
    let mut names = program
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect::<std::collections::HashSet<_>>();
    let mut functions = Vec::new();
    let mut mappings = Vec::new();
    for (crud_index, crud) in program.cruds.iter().enumerate() {
        let Some(table) = program.tables.iter().find(|table| table.name == crud.table) else {
            continue;
        };
        for (is_guard, conditions) in [
            (
                true,
                crud.guards
                    .iter()
                    .map(|rule| &rule.condition)
                    .collect::<Vec<_>>(),
            ),
            (
                false,
                crud.conditional_permissions
                    .iter()
                    .map(|rule| &rule.condition)
                    .collect::<Vec<_>>(),
            ),
        ] {
            for (policy_index, condition) in conditions.into_iter().enumerate() {
                let base_name =
                    format!("__zelyra_crud_policy_{crud_index}_{is_guard}_{policy_index}");
                let mut function_name = base_name.clone();
                let mut suffix = 0usize;
                while !names.insert(function_name.clone()) {
                    suffix += 1;
                    function_name = format!("{base_name}_{suffix}");
                }
                functions.push(Function {
                    name: function_name.clone(),
                    is_public: false,
                    type_params: Vec::new(),
                    params: table
                        .columns
                        .iter()
                        .map(|column| Param {
                            name: column.name.clone(),
                            ty: column.ty.clone(),
                            span: column.span,
                        })
                        .collect(),
                    return_type: Some(Type::Bool),
                    capabilities: Vec::new(),
                    requires: Vec::new(),
                    ensures: Vec::new(),
                    body: Block {
                        statements: vec![Stmt::Return {
                            value: Some(condition.clone()),
                            span: condition.span,
                        }],
                        span: condition.span,
                    },
                    span: condition.span,
                });
                mappings.push(CrudPolicyFunction {
                    crud_index,
                    policy_index,
                    is_guard,
                    function_name,
                });
            }
        }
    }
    (functions, mappings)
}

pub fn lower_crud_policy_functions_owned(
    mut program: Program,
) -> (Program, Vec<CrudPolicyFunction>) {
    let (functions, mappings) = build_crud_policy_functions(&program);
    program.functions.extend(functions);
    (program, mappings)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrudLayoutSlotDef {
    pub name: String,
    pub html: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrudSoftDeleteDef {
    pub column: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrudViewDef {
    /// Primary fields shared by generated list, detail, and CRUD form views.
    /// Explicit CRUD list fields remain an override for the list/detail view.
    pub fields: Vec<String>,
    pub list: CrudListViewDef,
    pub detail: CrudDetailViewDef,
    pub form: CrudFormViewDef,
    /// Per-route create and edit overrides. Each retains the shared form defaults.
    pub create: CrudFormPageDef,
    pub edit: CrudFormPageDef,
    pub delete: CrudDeleteViewDef,
    pub loading: CrudLoadingViewDef,
    pub error: CrudErrorViewDef,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrudFormPageDef {
    pub title: Option<String>,
    pub submit: Option<String>,
    pub before_form: Option<String>,
    pub after_form: Option<String>,
    /// Per-field help or auxiliary content rendered inside the matching field wrapper.
    pub field_content: Vec<CrudFieldContentDef>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrudFieldContentDef {
    pub field: String,
    pub html: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrudListViewDef {
    pub mode: CrudListViewMode,
    pub empty: Option<String>,
}

impl Default for CrudListViewDef {
    fn default() -> Self {
        Self {
            mode: CrudListViewMode::Table,
            empty: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CrudListViewMode {
    Table,
    Cards,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrudDetailViewDef {
    pub mode: CrudDetailViewMode,
    pub title: Option<String>,
}

impl Default for CrudDetailViewDef {
    fn default() -> Self {
        Self {
            mode: CrudDetailViewMode::Standard,
            title: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CrudDetailViewMode {
    Standard,
    Cards,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrudFormViewDef {
    pub mode: CrudFormViewMode,
    pub title: Option<String>,
    pub submit: Option<String>,
}

impl Default for CrudFormViewDef {
    fn default() -> Self {
        Self {
            mode: CrudFormViewMode::Standard,
            title: None,
            submit: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CrudFormViewMode {
    Standard,
    Cards,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrudDeleteViewDef {
    pub title: Option<String>,
    pub message: Option<String>,
    pub submit: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrudLoadingViewDef {
    pub message: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrudErrorViewDef {
    pub title: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthDef {
    pub name: String,
    pub table: String,
    /// Optional required boolean user column. When configured, only users
    /// whose value is true may complete password authentication.
    pub login_enabled_column: Option<String>,
    /// Optional table with `(user_id, tenant_id)` membership rows and an
    /// optional `active` Boolean column (when present, only active rows count).
    pub membership_table: Option<String>,
    /// Optional tenant table and owner foreign-key column, written in source
    /// as `tenant_owner: tenants.user_id`. This is an additional verified
    /// access path for users who own a tenant directly.
    pub tenant_owner_table: Option<String>,
    pub tenant_owner_column: Option<String>,
    pub session_table: Option<String>,
    pub permissions_table: Option<String>,
    pub roles_table: Option<String>,
    pub role_permissions_table: Option<String>,
    pub audit_table: Option<String>,
    pub audit_chain: bool,
    pub admin_path: Option<String>,
    pub admin_permission: Option<String>,
    pub admin_role: Option<String>,
    pub login_rate_limit: Option<ApiRateLimit>,
    pub login_block_seconds: Option<u32>,
    pub reset_tokens_table: Option<String>,
    /// MariaDB table storing keyed, persistent password-reset rate-limit buckets.
    pub reset_rate_limits_table: Option<String>,
    pub reset_rate_limit: Option<ApiRateLimit>,
    pub reset_block_seconds: Option<u32>,
    /// Optional table for encrypted local TOTP factors.
    pub mfa_table: Option<String>,
    /// Optional table for hashed, single-use MFA recovery codes.
    pub mfa_recovery_table: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ApiDef {
    pub method: String,
    pub path: String,
    pub handler: Option<String>,
    pub requires_auth: bool,
    pub permissions: Vec<String>,
    pub input: Vec<ApiField>,
    /// Applies the named table's cross-field validation rules to the typed
    /// request values before invoking the API handler.
    pub validates_table: Option<String>,
    pub output: Type,
    /// Explicit successful status and media-type pairs for `ApiResponse<T>` handlers.
    pub responses: Vec<ApiSuccessResponse>,
    pub errors: Vec<ApiError>,
    pub version: Option<String>,
    pub deprecated: bool,
    pub rate_limit: Option<ApiRateLimit>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiSuccessResponse {
    pub status: u16,
    pub content_type: String,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiRateLimit {
    pub requests: u32,
    pub window_seconds: u32,
}

#[derive(Clone, Debug)]
pub struct ApiField {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ApiError {
    pub status: u16,
    pub name: String,
    pub payload: Option<Type>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct DatabaseDef {
    pub name: String,
    pub engine: String,
    pub database: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TableDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub validations: Vec<TableValidationDef>,
    pub indexes: Vec<IndexDef>,
    pub uniques: Vec<IndexDef>,
    pub access: TableAccessDef,
    /// Declares a nominal tenant identity column. Until checked tenant-aware
    /// query expressions are available, raw SQL access to this table is denied.
    pub tenant_scope: Option<TenantScopeDef>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TableValidationDef {
    /// A pure Boolean expression evaluated against the submitted table fields.
    pub condition: Expr,
    /// Static user-facing message shown when the condition is false.
    pub message: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TenantScopeDef {
    pub column: String,
    pub tenant_type: Type,
    pub span: Span,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TableAccessDef {
    pub read: Vec<String>,
    pub write: Vec<String>,
    pub read_write: Vec<String>,
    pub span: Option<Span>,
}

#[derive(Clone, Debug)]
pub struct ColumnDef {
    pub name: String,
    pub ty: Type,
    pub length: Option<u32>,
    /// Optional text storage capacity override. This affects physical schema
    /// storage only; the source-language value remains String.
    pub text_storage: Option<TextStorage>,
    /// Optional physical width for an unsigned integer or ID relation.
    pub unsigned_storage: Option<UnsignedStorage>,
    /// Optional explicit database column name; useful when a relation column
    /// does not follow Zelyra's default `<field>_id` naming convention.
    pub storage_name: Option<String>,
    /// Optional total precision and fractional scale for fixed-point database columns.
    pub decimal_precision: Option<(u32, u32)>,
    /// Declarative business number format. `{0001}` is a zero-padded,
    /// transactionally allocated counter and `{year}` is the UTC year.
    /// Allocated values are monotonic but intentionally not gapless.
    pub auto_sequence: Option<String>,
    /// Candidate expression for a server-derived column. The compiler checks
    /// its field scope and result type; mutation-path evaluation is separate.
    pub computed: Option<Expr>,
    /// Upload constraints for an opaque FileRef column exposed by generated CRUD.
    pub attachment: Option<FileUploadOptions>,
    pub required: bool,
    pub primary_key: bool,
    pub auto: bool,
    pub unique: bool,
    /// Adds a composite unique key over the table's tenant scope and this
    /// column. Valid only on a table declared with `scoped by`.
    pub unique_within_tenant: bool,
    pub default: Option<DefaultValue>,
    /// Explicit database foreign-key target for a scalar ID column. Unlike a
    /// relation-typed column, this preserves the scalar's nominal domain type.
    pub references: Option<String>,
    pub on_delete: Option<ReferentialAction>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextStorage {
    Text,
    MediumText,
    LongText,
}

/// Physical width for an unsigned integer column. The source-language value
/// retains its full UInt/ID type; native database limits apply where the
/// backend offers an unsigned integer type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsignedStorage {
    TinyInt,
    SmallInt,
    MediumInt,
    Int,
    BigInt,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReferentialAction {
    #[default]
    NoAction,
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
}

impl fmt::Display for ReferentialAction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NoAction => "no_action",
            Self::Restrict => "restrict",
            Self::Cascade => "cascade",
            Self::SetNull => "set_null",
            Self::SetDefault => "set_default",
        })
    }
}

#[derive(Clone, Debug)]
pub enum DefaultValue {
    Int(i64),
    Bool(bool),
    String(String),
    Ident(String),
}

#[derive(Clone, Debug)]
pub struct IndexDef {
    pub columns: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TypeDef {
    pub name: String,
    pub is_public: bool,
    pub target: Type,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct RecordDef {
    pub name: String,
    pub is_public: bool,
    pub is_class: bool,
    pub type_params: Vec<String>,
    pub fields: Vec<RecordField>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct RecordField {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub is_public: bool,
    /// Type parameters declared after the function name, for example `fn first<T>`.
    pub type_params: Vec<String>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub capabilities: Vec<String>,
    pub requires: Vec<Expr>,
    pub ensures: Vec<Expr>,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Let {
        name: String,
        ty: Option<Type>,
        value: Expr,
        mutable: bool,
        span: Span,
    },
    BindOrAssign {
        name: String,
        value: Expr,
        span: Span,
    },
    ArrayIndexAssign {
        name: String,
        index: Expr,
        value: Expr,
        operation: ArrayIndexAssignOp,
        span: Span,
    },
    TupleBindOrAssign {
        names: Vec<String>,
        value: Expr,
        span: Span,
    },
    Expr(Expr),
    Return {
        value: Option<Expr>,
        span: Span,
    },
    If {
        condition: Expr,
        then_block: Block,
        else_block: Option<Block>,
        span: Span,
    },
    While {
        condition: Expr,
        invariants: Vec<Expr>,
        body: Block,
        span: Span,
    },
    For {
        name: String,
        additional_names: Vec<String>,
        iterable: Expr,
        body: Block,
        span: Span,
    },
    Loop {
        invariants: Vec<Expr>,
        body: Block,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    Match {
        value: Expr,
        arms: Vec<MatchArm>,
        span: Span,
    },
    Transaction {
        body: Block,
        span: Span,
    },
    Parallel {
        body: Block,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct MatchExprArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum PatternKind {
    Wildcard,
    Variable(String),
    Constructor {
        name: String,
        inner: Option<Box<Pattern>>,
    },
    EnumVariant {
        enum_name: String,
        variant: String,
    },
    EnumVariantPayload {
        enum_name: String,
        variant: String,
        payloads: Vec<Pattern>,
    },
    Record {
        type_name: String,
        fields: Vec<(String, Pattern)>,
    },
    Int(i64),
    Bool(bool),
    String(String),
    Char(char),
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
    Char(char),
    Array(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Record {
        type_name: String,
        type_args: Vec<Type>,
        fields: Vec<(String, Expr)>,
    },
    Lambda {
        params: Vec<Param>,
        body: Box<Expr>,
    },
    FunctionValue(String),
    Variable(String),
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    Field {
        target: Box<Expr>,
        field: String,
    },
    Call {
        name: String,
        type_args: Vec<Type>,
        args: Vec<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Sql {
        result_type: Type,
        query: String,
    },
    Await(Box<Expr>),
    Match {
        value: Box<Expr>,
        arms: Vec<MatchExprArm>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrayIndexAssignOp {
    Set,
    Add,
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. } => *span,
            Stmt::BindOrAssign { span, .. } => *span,
            Stmt::ArrayIndexAssign { span, .. } => *span,
            Stmt::TupleBindOrAssign { span, .. } => *span,
            Stmt::Expr(expr) => expr.span,
            Stmt::Return { span, .. } => *span,
            Stmt::If { span, .. } => *span,
            Stmt::While { span, .. } => *span,
            Stmt::For { span, .. } => *span,
            Stmt::Break { span } => *span,
            Stmt::Continue { span } => *span,
            Stmt::Transaction { span, .. } => *span,
            Stmt::Parallel { span, .. } => *span,
            Stmt::Loop { span, .. } => *span,
            Stmt::Match { span, .. } => *span,
        }
    }
}
