use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }

    pub fn join(self, other: Span) -> Self {
        Self {
            start: self.start,
            end: other.end,
            line: self.line,
            column: self.column,
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
    Timestamp,
    Date,
    Time,
    Duration,
    Unit,
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Array(Box<Type>),
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
            Type::Timestamp => "Timestamp",
            Type::Date => "Date",
            Type::Time => "Time",
            Type::Duration => "Duration",
            Type::Unit => "Unit",
            Type::Option(inner) => return write!(f, "Option<{inner}>"),
            Type::Result(ok, err) => return write!(f, "Result<{ok}, {err}>"),
            Type::Array(inner) => return write!(f, "{inner}[]"),
            Type::Named(name) => name,
            Type::Unknown => "unknown",
        };
        write!(f, "{name}")
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    pub databases: Vec<DatabaseDef>,
    pub tables: Vec<TableDef>,
    pub types: Vec<TypeDef>,
    pub pages: Vec<PageDef>,
    pub forms: Vec<FormDef>,
    pub cruds: Vec<CrudDef>,
    pub auth: Vec<AuthDef>,
    pub apis: Vec<ApiDef>,
    pub functions: Vec<Function>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageDef {
    pub path: String,
    pub html: String,
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
    pub max: Option<u32>,
    pub widget: Option<String>,
    pub readonly: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct FormAction {
    pub name: String,
    pub statements: Vec<Stmt>,
    pub success: Option<String>,
    pub redirect: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct CrudDef {
    pub name: String,
    pub table: String,
    pub title: Option<String>,
    pub list: Vec<String>,
    pub search: Vec<String>,
    pub filters: Vec<String>,
    pub requires_auth: bool,
    pub permissions: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthDef {
    pub name: String,
    pub table: String,
    pub session_table: Option<String>,
    pub permissions_table: Option<String>,
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
    pub span: Span,
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
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ColumnDef {
    pub name: String,
    pub ty: Type,
    pub length: Option<u32>,
    pub required: bool,
    pub primary_key: bool,
    pub auto: bool,
    pub unique: bool,
    pub default: Option<DefaultValue>,
    pub span: Span,
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
    pub target: Type,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
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
}

#[derive(Clone, Debug)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Block,
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
    Variable(String),
    Call {
        name: String,
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
