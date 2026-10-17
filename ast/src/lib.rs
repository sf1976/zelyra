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
    pub filters: Vec<String>,
    pub searchable: bool,
    pub sortable: bool,
    pub page_size: Option<u32>,
    pub requires_auth: bool,
    pub permissions: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct FormDef {
    pub name: String,
    pub table: Option<String>,
    pub fields: Vec<FormField>,
    pub actions: Vec<FormAction>,
    pub span: Span,
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
    pub statements: Vec<Stmt>,
    pub success: Option<String>,
    pub redirect: Option<String>,
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
    pub actions: Vec<FormAction>,
    pub span: Span,
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
    pub delete: CrudDeleteViewDef,
    pub loading: CrudLoadingViewDef,
    pub error: CrudErrorViewDef,
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
    /// Optional table with active `(user_id, tenant_id, active)` membership rows.
    pub membership_table: Option<String>,
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
    pub output: Type,
    pub errors: Vec<ApiError>,
    pub version: Option<String>,
    pub deprecated: bool,
    pub rate_limit: Option<ApiRateLimit>,
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
    pub indexes: Vec<IndexDef>,
    pub uniques: Vec<IndexDef>,
    pub access: TableAccessDef,
    /// Declares a nominal tenant identity column. Until checked tenant-aware
    /// query expressions are available, raw SQL access to this table is denied.
    pub tenant_scope: Option<TenantScopeDef>,
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
    /// Optional explicit database column name; useful when a relation column
    /// does not follow Zelyra's default `<field>_id` naming convention.
    pub storage_name: Option<String>,
    /// Optional total precision and fractional scale for fixed-point database columns.
    pub decimal_precision: Option<(u32, u32)>,
    pub required: bool,
    pub primary_key: bool,
    pub auto: bool,
    pub unique: bool,
    pub default: Option<DefaultValue>,
    pub on_delete: Option<ReferentialAction>,
    pub span: Span,
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
