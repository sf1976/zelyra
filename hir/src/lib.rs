use std::collections::{HashMap, HashSet};
use std::fmt;
use zelyra_ast::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FunctionId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LocalId(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub struct ResolveError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Clone, Debug)]
pub struct HirProgram {
    pub types: Vec<TypeDef>,
    pub records: Vec<RecordDef>,
    pub enums: Vec<EnumDef>,
    pub apis: Vec<ApiDef>,
    pub functions: Vec<HirFunction>,
    pub functions_by_name: HashMap<String, FunctionId>,
}

#[derive(Clone, Debug)]
pub struct HirFunction {
    pub id: FunctionId,
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<HirParam>,
    pub return_type: Option<Type>,
    pub capabilities: Vec<String>,
    pub requires: Vec<HirExpr>,
    pub ensures: Vec<HirExpr>,
    /// Local used by `result` references in postconditions.
    pub contract_result_local: LocalId,
    pub body: HirBlock,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct HirParam {
    pub name: String,
    pub ty: Type,
    pub local: LocalId,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct HirBlock {
    pub statements: Vec<HirStmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum HirStmt {
    Let {
        name: String,
        local: LocalId,
        ty: Option<Type>,
        value: HirExpr,
        mutable: bool,
        span: Span,
    },
    Assign {
        name: String,
        local: LocalId,
        value: HirExpr,
        span: Span,
    },
    ArrayIndexAssign {
        name: String,
        local: LocalId,
        index: HirExpr,
        value: HirExpr,
        operation: ArrayIndexAssignOp,
        span: Span,
    },
    TupleBindOrAssign {
        bindings: Vec<HirTupleBinding>,
        value: HirExpr,
        span: Span,
    },
    Expr(HirExpr),
    Return {
        value: Option<HirExpr>,
        span: Span,
    },
    If {
        condition: HirExpr,
        then_block: HirBlock,
        else_block: Option<HirBlock>,
        span: Span,
    },
    While {
        condition: HirExpr,
        invariants: Vec<HirExpr>,
        body: HirBlock,
        span: Span,
    },
    For {
        name: String,
        local: LocalId,
        additional_names: Vec<String>,
        additional_locals: Vec<LocalId>,
        iterable: HirExpr,
        body: HirBlock,
        span: Span,
    },
    Loop {
        invariants: Vec<HirExpr>,
        body: HirBlock,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    Match {
        value: HirExpr,
        arms: Vec<HirMatchArm>,
        span: Span,
    },
    Transaction {
        body: HirBlock,
        span: Span,
    },
    Parallel {
        body: HirBlock,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub struct HirTupleBinding {
    pub name: String,
    pub local: LocalId,
    pub is_new: bool,
}

#[derive(Clone, Debug)]
pub struct HirMatchArm {
    pub pattern: HirPattern,
    pub guard: Option<HirExpr>,
    pub body: HirBlock,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct HirPattern {
    pub kind: HirPatternKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum HirPatternKind {
    Wildcard,
    Binding {
        name: String,
        local: LocalId,
    },
    Constructor {
        name: String,
        inner: Option<Box<HirPattern>>,
    },
    EnumVariant {
        enum_name: String,
        variant: String,
    },
    EnumVariantPayload {
        enum_name: String,
        variant: String,
        payloads: Vec<HirPattern>,
    },
    Record {
        type_name: String,
        fields: Vec<(String, HirPattern)>,
    },
    Int(i64),
    Bool(bool),
    String(String),
    Char(char),
}

#[derive(Clone, Debug)]
pub struct HirExpr {
    pub kind: HirExprKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum HirExprKind {
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
    Char(char),
    Array(Vec<HirExpr>),
    Map(Vec<(HirExpr, HirExpr)>),
    Record {
        type_name: String,
        type_args: Vec<Type>,
        fields: Vec<(String, HirExpr)>,
    },
    Lambda {
        params: Vec<HirLambdaParam>,
        body: Box<HirExpr>,
    },
    Local(LocalId),
    EnumVariant {
        enum_name: String,
        variant: String,
    },
    FunctionValue {
        name: String,
        function: Option<FunctionId>,
    },
    Index {
        target: Box<HirExpr>,
        index: Box<HirExpr>,
    },
    Field {
        target: Box<HirExpr>,
        field: String,
    },
    Call {
        name: String,
        function: Option<FunctionId>,
        callee_local: Option<LocalId>,
        type_args: Vec<Type>,
        args: Vec<HirExpr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<HirExpr>,
    },
    Binary {
        left: Box<HirExpr>,
        op: BinaryOp,
        right: Box<HirExpr>,
    },
    BinaryChain {
        first: Box<HirExpr>,
        operations: Vec<(BinaryOp, HirExpr, Span)>,
    },
    Sql {
        result_type: Type,
        query: String,
        parameters: Vec<(String, LocalId)>,
    },
    Await(Box<HirExpr>),
    Match {
        value: Box<HirExpr>,
        arms: Vec<HirMatchExprArm>,
    },
}

#[derive(Clone, Debug)]
pub struct HirLambdaParam {
    pub name: String,
    pub local: LocalId,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct HirMatchExprArm {
    pub pattern: HirPattern,
    pub guard: Option<HirExpr>,
    pub value: HirExpr,
    pub span: Span,
}

pub fn lower(program: &Program) -> Result<HirProgram, Vec<ResolveError>> {
    let mut functions_by_name = HashMap::new();
    let mut errors = Vec::new();
    for (index, function) in program.functions.iter().enumerate() {
        if functions_by_name
            .insert(function.name.clone(), FunctionId(index))
            .is_some()
        {
            errors.push(ResolveError {
                message: format!("duplicate function `{}`", function.name),
                span: function.span,
            });
        }
    }

    let mut functions = Vec::new();
    let enum_names = program
        .enums
        .iter()
        .map(|definition| definition.name.clone())
        .collect::<HashSet<_>>();
    for (index, function) in program.functions.iter().enumerate() {
        let mut resolver = Resolver {
            functions: &functions_by_name,
            enum_names: &enum_names,
            errors: Vec::new(),
            scopes: vec![HashMap::new()],
            next_local: 0,
        };
        let mut params = Vec::new();
        for parameter in &function.params {
            let local = resolver.bind(parameter.name.clone(), parameter.span);
            params.push(HirParam {
                name: parameter.name.clone(),
                ty: parameter.ty.clone(),
                local,
                span: parameter.span,
            });
        }
        let body = resolver.block(&function.body);
        let requires = function
            .requires
            .iter()
            .map(|contract| resolver.expr(contract))
            .collect();
        let contract_result_local = resolver.bind("result".into(), function.span);
        let ensures = function
            .ensures
            .iter()
            .map(|contract| resolver.expr(contract))
            .collect();
        errors.extend(resolver.errors);
        functions.push(HirFunction {
            id: FunctionId(index),
            name: function.name.clone(),
            type_params: function.type_params.clone(),
            params,
            return_type: function.return_type.clone(),
            capabilities: function.capabilities.clone(),
            requires,
            ensures,
            contract_result_local,
            body,
            span: function.span,
        });
    }

    if errors.is_empty() {
        Ok(HirProgram {
            types: program.types.clone(),
            records: program.records.clone(),
            enums: program.enums.clone(),
            apis: program.apis.clone(),
            functions,
            functions_by_name,
        })
    } else {
        Err(errors)
    }
}

struct Resolver<'a> {
    functions: &'a HashMap<String, FunctionId>,
    enum_names: &'a HashSet<String>,
    errors: Vec<ResolveError>,
    scopes: Vec<HashMap<String, LocalId>>,
    next_local: usize,
}

impl<'a> Resolver<'a> {
    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.errors.push(ResolveError {
            message: message.into(),
            span,
        });
    }
    fn bind(&mut self, name: String, span: Span) -> LocalId {
        let local = LocalId(self.next_local);
        self.next_local += 1;
        if self
            .scopes
            .last_mut()
            .unwrap()
            .insert(name.clone(), local)
            .is_some()
        {
            self.error(
                span,
                format!("name `{name}` is already declared in this scope"),
            );
        }
        local
    }
    fn lookup(&self, name: &str) -> Option<LocalId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }
    fn block(&mut self, block: &Block) -> HirBlock {
        self.scopes.push(HashMap::new());
        let statements = block
            .statements
            .iter()
            .map(|statement| self.statement(statement))
            .collect();
        self.scopes.pop();
        HirBlock {
            statements,
            span: block.span,
        }
    }
    fn statement(&mut self, statement: &Stmt) -> HirStmt {
        match statement {
            Stmt::Let {
                name,
                ty,
                value,
                mutable,
                span,
            } => {
                let value = self.expr(value);
                let local = self.bind(name.clone(), *span);
                HirStmt::Let {
                    name: name.clone(),
                    local,
                    ty: ty.clone(),
                    value,
                    mutable: *mutable,
                    span: *span,
                }
            }
            Stmt::BindOrAssign { name, value, span } => {
                let value = self.expr(value);
                if let Some(local) = self.lookup(name) {
                    HirStmt::Assign {
                        name: name.clone(),
                        local,
                        value,
                        span: *span,
                    }
                } else {
                    let local = self.bind(name.clone(), *span);
                    HirStmt::Let {
                        name: name.clone(),
                        local,
                        ty: None,
                        value,
                        mutable: false,
                        span: *span,
                    }
                }
            }
            Stmt::ArrayIndexAssign {
                name,
                index,
                value,
                operation,
                span,
            } => {
                let local = self
                    .lookup(name)
                    .unwrap_or_else(|| self.bind(name.clone(), *span));
                HirStmt::ArrayIndexAssign {
                    name: name.clone(),
                    local,
                    index: self.expr(index),
                    value: self.expr(value),
                    operation: *operation,
                    span: *span,
                }
            }
            Stmt::TupleBindOrAssign { names, value, span } => {
                let value = self.expr(value);
                let bindings = names
                    .iter()
                    .map(|name| {
                        if let Some(local) = self.lookup(name) {
                            HirTupleBinding {
                                name: name.clone(),
                                local,
                                is_new: false,
                            }
                        } else {
                            let local = self.bind(name.clone(), *span);
                            HirTupleBinding {
                                name: name.clone(),
                                local,
                                is_new: true,
                            }
                        }
                    })
                    .collect();
                HirStmt::TupleBindOrAssign {
                    bindings,
                    value,
                    span: *span,
                }
            }
            Stmt::Expr(expr) => HirStmt::Expr(self.expr(expr)),
            Stmt::Return { value, span } => HirStmt::Return {
                value: value.as_ref().map(|value| self.expr(value)),
                span: *span,
            },
            Stmt::If {
                condition,
                then_block,
                else_block,
                span,
            } => HirStmt::If {
                condition: self.expr(condition),
                then_block: self.block(then_block),
                else_block: else_block.as_ref().map(|block| self.block(block)),
                span: *span,
            },
            Stmt::While {
                condition,
                invariants,
                body,
                span,
            } => HirStmt::While {
                condition: self.expr(condition),
                invariants: invariants
                    .iter()
                    .map(|invariant| self.expr(invariant))
                    .collect(),
                body: self.block(body),
                span: *span,
            },
            Stmt::For {
                name,
                additional_names,
                iterable,
                body,
                span,
            } => {
                let iterable = self.expr(iterable);
                self.scopes.push(HashMap::new());
                let local = self.bind(name.clone(), *span);
                let additional_locals = additional_names
                    .iter()
                    .map(|name| self.bind(name.clone(), *span))
                    .collect();
                let body = self.block(body);
                self.scopes.pop();
                HirStmt::For {
                    name: name.clone(),
                    local,
                    additional_names: additional_names.clone(),
                    additional_locals,
                    iterable,
                    body,
                    span: *span,
                }
            }
            Stmt::Loop {
                invariants,
                body,
                span,
            } => HirStmt::Loop {
                invariants: invariants
                    .iter()
                    .map(|invariant| self.expr(invariant))
                    .collect(),
                body: self.block(body),
                span: *span,
            },
            Stmt::Break { span } => HirStmt::Break { span: *span },
            Stmt::Continue { span } => HirStmt::Continue { span: *span },
            Stmt::Match { value, arms, span } => HirStmt::Match {
                value: self.expr(value),
                arms: arms
                    .iter()
                    .map(|arm| {
                        self.scopes.push(HashMap::new());
                        let pattern = self.pattern(&arm.pattern);
                        let guard = arm.guard.as_ref().map(|guard| self.expr(guard));
                        let body = self.block(&arm.body);
                        self.scopes.pop();
                        HirMatchArm {
                            pattern,
                            guard,
                            body,
                            span: arm.span,
                        }
                    })
                    .collect(),
                span: *span,
            },
            Stmt::Transaction { body, span } => HirStmt::Transaction {
                body: self.block(body),
                span: *span,
            },
            Stmt::Parallel { body, span } => {
                let statements = body
                    .statements
                    .iter()
                    .map(|statement| match statement {
                        Stmt::BindOrAssign { name, value, span } => {
                            let value = self.expr(value);
                            if let Some(local) = self.lookup(name) {
                                HirStmt::Assign {
                                    name: name.clone(),
                                    local,
                                    value,
                                    span: *span,
                                }
                            } else {
                                let local = self.bind(name.clone(), *span);
                                HirStmt::Let {
                                    name: name.clone(),
                                    local,
                                    ty: None,
                                    value,
                                    mutable: false,
                                    span: *span,
                                }
                            }
                        }
                        statement => self.statement(statement),
                    })
                    .collect();
                HirStmt::Parallel {
                    body: HirBlock {
                        statements,
                        span: body.span,
                    },
                    span: *span,
                }
            }
        }
    }
    fn pattern(&mut self, pattern: &Pattern) -> HirPattern {
        let kind = match &pattern.kind {
            PatternKind::Wildcard => HirPatternKind::Wildcard,
            PatternKind::Variable(name) => {
                let local = self.bind(name.clone(), pattern.span);
                HirPatternKind::Binding {
                    name: name.clone(),
                    local,
                }
            }
            PatternKind::Constructor { name, inner } => HirPatternKind::Constructor {
                name: name.clone(),
                inner: inner.as_ref().map(|inner| Box::new(self.pattern(inner))),
            },
            PatternKind::EnumVariant { enum_name, variant } => HirPatternKind::EnumVariant {
                enum_name: enum_name.clone(),
                variant: variant.clone(),
            },
            PatternKind::EnumVariantPayload {
                enum_name,
                variant,
                payloads,
            } => HirPatternKind::EnumVariantPayload {
                enum_name: enum_name.clone(),
                variant: variant.clone(),
                payloads: payloads
                    .iter()
                    .map(|payload| self.pattern(payload))
                    .collect(),
            },
            PatternKind::Record { type_name, fields } => HirPatternKind::Record {
                type_name: type_name.clone(),
                fields: fields
                    .iter()
                    .map(|(name, pattern)| (name.clone(), self.pattern(pattern)))
                    .collect(),
            },
            PatternKind::Int(value) => HirPatternKind::Int(*value),
            PatternKind::Bool(value) => HirPatternKind::Bool(*value),
            PatternKind::String(value) => HirPatternKind::String(value.clone()),
            PatternKind::Char(value) => HirPatternKind::Char(*value),
        };
        HirPattern {
            kind,
            span: pattern.span,
        }
    }
    fn expr(&mut self, expr: &Expr) -> HirExpr {
        if matches!(expr.kind, ExprKind::Binary { .. }) {
            return self.binary_expr_chain(expr);
        }
        let kind = match &expr.kind {
            ExprKind::Int(value) => HirExprKind::Int(*value),
            ExprKind::UInt(value) => HirExprKind::UInt(*value),
            ExprKind::Float(value) => HirExprKind::Float(*value),
            ExprKind::Bool(value) => HirExprKind::Bool(*value),
            ExprKind::String(value) => HirExprKind::String(value.clone()),
            ExprKind::Char(value) => HirExprKind::Char(*value),
            ExprKind::Array(values) => {
                HirExprKind::Array(values.iter().map(|value| self.expr(value)).collect())
            }
            ExprKind::Map(entries) => HirExprKind::Map(
                entries
                    .iter()
                    .map(|(key, value)| (self.expr(key), self.expr(value)))
                    .collect(),
            ),
            ExprKind::Record {
                type_name,
                type_args,
                fields,
            } => HirExprKind::Record {
                type_name: type_name.clone(),
                type_args: type_args.clone(),
                fields: fields
                    .iter()
                    .map(|(name, value)| (name.clone(), self.expr(value)))
                    .collect(),
            },
            ExprKind::Lambda { params, body } => {
                self.scopes.push(HashMap::new());
                let params = params
                    .iter()
                    .map(|param| HirLambdaParam {
                        name: param.name.clone(),
                        local: self.bind(param.name.clone(), param.span),
                        ty: param.ty.clone(),
                    })
                    .collect();
                let body = Box::new(self.expr(body));
                self.scopes.pop();
                HirExprKind::Lambda { params, body }
            }
            ExprKind::FunctionValue(name) => {
                let function = self.functions.get(name).copied();
                if function.is_none() {
                    self.error(expr.span, format!("unknown function `{name}`"));
                }
                HirExprKind::FunctionValue {
                    name: name.clone(),
                    function,
                }
            }
            ExprKind::Variable(name) => match self.lookup(name) {
                Some(local) => HirExprKind::Local(local),
                None if name.split_once("::").is_some() => {
                    let (enum_name, variant) = name.split_once("::").unwrap();
                    HirExprKind::EnumVariant {
                        enum_name: enum_name.to_owned(),
                        variant: variant.to_owned(),
                    }
                }
                None if name == "None" => HirExprKind::Call {
                    name: name.clone(),
                    function: None,
                    callee_local: None,
                    type_args: Vec::new(),
                    args: Vec::new(),
                },
                None => {
                    self.error(expr.span, format!("unknown variable `{name}`"));
                    HirExprKind::Local(LocalId(usize::MAX))
                }
            },
            ExprKind::Call {
                name,
                type_args,
                args,
            } => {
                let function = self.functions.get(name).copied();
                let callee_local = function.is_none().then(|| self.lookup(name)).flatten();
                if function.is_none()
                    && callee_local.is_none()
                    && !name
                        .rsplit_once("::")
                        .is_some_and(|(enum_name, _)| self.enum_names.contains(enum_name))
                    && !matches!(
                        name.as_str(),
                        "print"
                            | "assert"
                            | "len"
                            | "range"
                            | "array_replace_result"
                            | "object_set_field"
                            | "record_with"
                            | "record_update"
                            | "map"
                            | "filter"
                            | "find"
                            | "reduce"
                            | "append"
                            | "push_mut"
                            | "pop_mut"
                            | "tuple"
                            | "set"
                            | "set_add"
                            | "set_remove"
                            | "contains"
                            | "get"
                            | "put"
                            | "keys"
                            | "values"
                            | "first"
                            | "last"
                            | "Some"
                            | "Ok"
                            | "Err"
                            | "now"
                            | "date"
                            | "decimal"
                            | "time"
                            | "duration_ms"
                            | "bytes"
                            | "format_timestamp"
                            | "format_text"
                            | "env"
                            | "random_int"
                            | "read_console"
                            | "http_get"
                            | "http_request"
                            | "json_encode"
                            | "json_encode_tagged_result"
                            | "json_decode"
                            | "json_decode_result"
                            | "sql_decode"
                            | "json_decode_tagged_result"
                            | "module_setting"
                            | "csv_decode_result"
                            | "csv_encode"
                            | "text_contains"
                            | "text_trim"
                            | "text_chars"
                            | "abs"
                            | "min"
                            | "max"
                            | "clamp"
                            | "to_string"
                            | "parse_int"
                            | "parse_float"
                            | "sort"
                            | "sort_desc"
                            | "reverse"
                            | "slice"
                            | "concat"
                            | "join"
                            | "text_to_lower"
                            | "text_to_upper"
                            | "text_ucfirst"
                            | "text_starts_with"
                            | "text_ends_with"
                            | "text_replace"
                            | "text_split"
                            | "split"
                            | "text_pad_left"
                            | "text_pad_right"
                            | "get_or"
                            | "get_or_default"
                            | "map_increment"
                            | "has_key"
                            | "unwrap_or"
                            | "sqrt"
                            | "pow"
                            | "floor"
                            | "ceil"
                            | "round"
                            | "text_index_of"
                            | "text_repeat"
                            | "text_is_empty"
                            | "text_capitalize"
                            | "unique"
                            | "array_sum"
                            | "avg"
                            | "min_of"
                            | "max_of"
                            | "chunk"
                            | "flatten"
                            | "set_union"
                            | "set_intersection"
                            | "set_difference"
                            | "map_remove"
                            | "map_merge"
                            | "date_add_days"
                            | "date_diff_days"
                            | "is_ok"
                            | "is_err"
                            | "is_some"
                            | "is_none"
                            | "base64_encode"
                            | "base64_decode"
                            | "hash_sha256"
                            | "uuid_v4"
                            | "url_encode"
                            | "url_decode"
                            | "text_trim_start"
                            | "text_trim_end"
                            | "text_substring"
                            | "text_is_blank"
                            | "zip"
                            | "entries"
                            | "dedup"
                            | "to_int"
                            | "to_float"
                            | "parse_bool"
                            | "is_before"
                            | "is_after"
                            | "panic"
                            | "path_join"
                            | "path_filename"
                            | "path_extension"
                            | "path_dir"
                            | "file_exists"
                            | "dir_exists"
                            | "file_size"
                            | "text_replace_result"
                            | "text_split_result"
                            | "decimal_scaled_result"
                            | "format_scaled_result"
                            | "date_add_days_result"
                            | "date_days_between_result"
                            | "timestamp_add_result"
                            | "timestamp_difference_result"
                            | "decimal_quantize_result"
                            | "http_json"
                            | "http_result"
                            | "mail_queue"
                            | "render_pdf"
                            | "mail_deliver"
                            | "run_process"
                            | "read_text"
                            | "read_text_result"
                            | "write_text"
                            | "write_text_result"
                            | "write_text_new_result"
                            | "atomic_write_text_result"
                            | "append_text_result"
                            | "copy_file_new_result"
                            | "rename_file_result"
                            | "delete_file_result"
                            | "delete_file"
                            | "list_dir"
                    )
                {
                    self.error(expr.span, format!("unknown function `{name}`"));
                }
                HirExprKind::Call {
                    name: name.clone(),
                    function,
                    callee_local,
                    type_args: type_args.clone(),
                    args: args.iter().map(|arg| self.expr(arg)).collect(),
                }
            }
            ExprKind::Unary { op, expr: inner } => HirExprKind::Unary {
                op: *op,
                expr: Box::new(self.expr(inner)),
            },
            ExprKind::Binary { left, op, right } => HirExprKind::Binary {
                left: Box::new(self.expr(left)),
                op: *op,
                right: Box::new(self.expr(right)),
            },
            ExprKind::Index { target, index } => HirExprKind::Index {
                target: Box::new(self.expr(target)),
                index: Box::new(self.expr(index)),
            },
            ExprKind::Field { target, field } => HirExprKind::Field {
                target: Box::new(self.expr(target)),
                field: field.clone(),
            },
            ExprKind::Sql { result_type, query } => {
                let mut parameters = Vec::new();
                match zelyra_database::sql_parameter_names(query) {
                    Ok(names) => {
                        for name in names {
                            if parameters.iter().any(|(existing, _)| existing == &name) {
                                continue;
                            }
                            match self.lookup(&name) {
                                Some(local) => parameters.push((name.clone(), local)),
                                None => self.error(
                                    expr.span,
                                    format!("SQL parameter `:{name}` is not available"),
                                ),
                            }
                        }
                    }
                    Err(error) => {
                        self.error(expr.span, format!("could not scan SQL parameters: {error}"))
                    }
                }
                HirExprKind::Sql {
                    result_type: result_type.clone(),
                    query: query.clone(),
                    parameters,
                }
            }
            ExprKind::Await(inner) => HirExprKind::Await(Box::new(self.expr(inner))),
            ExprKind::Match { value, arms } => HirExprKind::Match {
                value: Box::new(self.expr(value)),
                arms: arms
                    .iter()
                    .map(|arm| {
                        self.scopes.push(HashMap::new());
                        let pattern = self.pattern(&arm.pattern);
                        let guard = arm.guard.as_ref().map(|guard| self.expr(guard));
                        let value = self.expr(&arm.value);
                        self.scopes.pop();
                        HirMatchExprArm {
                            pattern,
                            guard,
                            value,
                            span: arm.span,
                        }
                    })
                    .collect(),
            },
        };
        HirExpr {
            kind,
            span: expr.span,
        }
    }
    fn binary_expr_chain(&mut self, expr: &Expr) -> HirExpr {
        let mut cursor = expr;
        let mut operations = Vec::new();
        while let ExprKind::Binary { left, op, right } = &cursor.kind {
            operations.push((*op, right.as_ref(), cursor.span));
            cursor = left;
        }
        HirExpr {
            kind: HirExprKind::BinaryChain {
                first: Box::new(self.expr(cursor)),
                operations: operations
                    .into_iter()
                    .rev()
                    .map(|(op, right, span)| (op, self.expr(right), span))
                    .collect(),
            },
            span: expr.span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zelyra_lexer::lex;
    use zelyra_parser::parse;

    #[test]
    fn resolves_locals_and_function_calls() {
        let program = parse(
            &lex("fn add(value: Int) -> Int { return value + 1 } fn main() { number = add(2) print(number) }")
                .unwrap(),
        )
        .unwrap();
        let hir = lower(&program).unwrap();
        assert_eq!(hir.functions_by_name["add"], FunctionId(0));
        assert!(matches!(
            hir.functions[1].body.statements[0],
            HirStmt::Let { .. }
        ));
    }

    #[test]
    fn preserves_generic_function_parameters_in_hir() {
        let program = parse(
            &lex("pub fn identity<T, E>(value: Result<T, E>) -> Result<T, E> { return value } fn main() { }")
                .unwrap(),
        )
        .unwrap();
        let hir = lower(&program).unwrap();
        assert_eq!(hir.functions[0].type_params, ["T", "E"]);
        assert_eq!(
            hir.functions[0].params[0].ty,
            Type::Result(
                Box::new(Type::Named("T".into())),
                Box::new(Type::Named("E".into()))
            )
        );
    }

    #[test]
    fn lowers_payload_enum_constructors_and_pattern_bindings() {
        let program = parse(
            &lex("enum ResultState { Failed(String) Ready } fn main() { state = ResultState::Failed(\"no\") match state { ResultState::Failed(message) => { print(message) } ResultState::Ready => { print(\"ready\") } } }")
                .unwrap(),
        )
        .unwrap();
        let hir = lower(&program).unwrap();
        assert_eq!(hir.enums[0].variants, ["Failed", "Ready"]);
        assert!(matches!(
            &hir.functions[0].body.statements[0],
            HirStmt::Let {
                value: HirExpr {
                    kind: HirExprKind::Call { name, .. },
                    ..
                },
                ..
            } if name == "ResultState::Failed"
        ));
        let HirStmt::Match { arms, .. } = &hir.functions[0].body.statements[1] else {
            panic!("expected lowered payload enum match");
        };
        assert!(matches!(
            &arms[0].pattern.kind,
            HirPatternKind::EnumVariantPayload { payloads, .. }
                if matches!(payloads[0].kind, HirPatternKind::Binding { ref name, .. } if name == "message")
        ));
    }

    #[test]
    fn rejects_unresolved_names() {
        let program = parse(&lex("fn main() { print(missing) }").unwrap()).unwrap();
        let errors = lower(&program).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("unknown variable")));
    }

    #[test]
    fn lowers_records_arrays_and_array_builtins() {
        let program = parse(
            &lex("struct Address { city: String } fn main() { address = Address { city: \"Berlin\" } print(address.city) items = [1, 2] first = items[0] count = len(items) extended = append(items, 3) for item in extended { print(item) } print(first) print(count) print(extended) }").unwrap(),
        )
        .unwrap();
        let hir = lower(&program).unwrap();
        assert_eq!(hir.records[0].name, "Address");
        assert!(matches!(
            hir.functions[0].body.statements[0],
            HirStmt::Let {
                value: HirExpr {
                    kind: HirExprKind::Record { .. },
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            hir.functions[0].body.statements[1],
            HirStmt::Expr(HirExpr {
                kind: HirExprKind::Call { .. },
                ..
            })
        ));
        assert!(matches!(
            hir.functions[0].body.statements[4],
            HirStmt::Let {
                value: HirExpr {
                    kind: HirExprKind::Call { ref name, .. },
                    ..
                },
                ..
            } if name == "len"
        ));
        assert!(matches!(
            hir.functions[0].body.statements[6],
            HirStmt::For { ref name, .. } if name == "item"
        ));
    }

    #[test]
    fn lowers_explicit_scalar_text_formatting_builtin() {
        let program =
            parse(&lex("fn main() { formatted = format_text(7) print(formatted) }").unwrap())
                .unwrap();
        lower(&program).expect("format_text is a known language builtin");
    }

    #[test]
    fn lowers_map_literals_and_builtins() {
        let program = parse(
            &lex("fn main() { values: Map<String, Int> = Map { \"one\": 1 } result = get(values, \"one\") updated = put(values, \"two\", 2) } ").unwrap(),
        )
        .unwrap();
        let hir = lower(&program).unwrap();
        assert!(matches!(
            hir.functions[0].body.statements[0],
            HirStmt::Let {
                value: HirExpr {
                    kind: HirExprKind::Map(_),
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            hir.functions[0].body.statements[1],
            HirStmt::Let {
                value: HirExpr {
                    kind: HirExprKind::Call { ref name, .. },
                    ..
                },
                ..
            } if name == "get"
        ));
    }

    #[test]
    fn preserves_typed_json_call_arguments() {
        let program = parse(
            &lex("struct Customer { name: String } fn main() { customer = json_decode<Customer>(\"{}\") }").unwrap(),
        )
        .unwrap();
        let hir = lower(&program).unwrap();
        let HirStmt::Let {
            value:
                HirExpr {
                    kind:
                        HirExprKind::Call {
                            ref name,
                            ref type_args,
                            ..
                        },
                    ..
                },
            ..
        } = &hir.functions[0].body.statements[0]
        else {
            panic!("expected typed JSON call");
        };
        assert_eq!(name, "json_decode");
        assert_eq!(type_args, &[Type::Named("Customer".into())]);
    }

    #[test]
    fn lowers_result_filesystem_and_json_builtins() {
        let program = parse(
            &lex("fn main() uses FileSystem { decoded = json_decode_result<Int>(\"1\") content = read_text_result(\"state.json\") saved = write_text_result(\"state.json\", \"{}\") }").unwrap(),
        )
        .unwrap();
        let hir = lower(&program).unwrap();
        assert_eq!(hir.functions.len(), 1);
    }

    #[test]
    fn lowers_parallel_await_bindings() {
        let program = parse(
            &lex("fn load() -> Int { return 1 } fn main() { parallel { value = await load() } }")
                .unwrap(),
        )
        .unwrap();
        let hir = lower(&program).unwrap();
        assert!(matches!(
            hir.functions[1].body.statements[0],
            HirStmt::Parallel { .. }
        ));
    }

    #[test]
    fn resolves_clock_and_environment_builtins() {
        let program =
            parse(&lex("fn main() { timestamp = now() mode = env(\"ZELYRA_MODE\") }").unwrap())
                .unwrap();
        let hir = lower(&program).unwrap();
        assert!(matches!(
            hir.functions[0].body.statements[0],
            HirStmt::Let {
                value: HirExpr {
                    kind: HirExprKind::Call { ref name, .. },
                    ..
                },
                ..
            } if name == "now"
        ));
        assert!(matches!(
            hir.functions[0].body.statements[1],
            HirStmt::Let {
                value: HirExpr {
                    kind: HirExprKind::Call { ref name, .. },
                    ..
                },
                ..
            } if name == "env"
        ));
    }

    #[test]
    fn resolves_random_builtin() {
        let program = parse(&lex("fn main() { value = random_int(1, 6) }").unwrap()).unwrap();
        let hir = lower(&program).unwrap();
        assert!(matches!(
            hir.functions[0].body.statements[0],
            HirStmt::Let {
                value: HirExpr {
                    kind: HirExprKind::Call { ref name, .. },
                    ..
                },
                ..
            } if name == "random_int"
        ));
    }

    #[test]
    fn resolves_file_system_builtin() {
        let program =
            parse(&lex("fn main() { content = read_text(\"README.md\") }").unwrap()).unwrap();
        let hir = lower(&program).unwrap();
        assert!(matches!(
            hir.functions[0].body.statements[0],
            HirStmt::Let {
                value: HirExpr {
                    kind: HirExprKind::Call { ref name, .. },
                    ..
                },
                ..
            } if name == "read_text"
        ));
    }

    #[test]
    fn resolves_console_input_builtin() {
        let program =
            parse(&lex("fn main() { answer = read_console(\"Prompt: \") }").unwrap()).unwrap();
        let hir = lower(&program).unwrap();
        assert!(matches!(
            hir.functions[0].body.statements[0],
            HirStmt::Let {
                value: HirExpr {
                    kind: HirExprKind::Call { ref name, .. },
                    ..
                },
                ..
            } if name == "read_console"
        ));
    }

    #[test]
    fn lowers_function_values_and_resolves_indirect_calls() {
        let program = parse(
            &lex("fn increment(value: Int) -> Int { return value + 1 } fn apply(callback: Function<(Int) -> Int>, value: Int) -> Int { return callback(value) } fn main() { saved = &increment print(apply(saved, 4)) }")
                .unwrap(),
        )
        .unwrap();
        let hir = lower(&program).unwrap();
        assert!(matches!(
            hir.functions[2].body.statements[0],
            HirStmt::Let {
                value: HirExpr {
                    kind: HirExprKind::FunctionValue {
                        ref name,
                        function: Some(FunctionId(0)),
                    },
                    ..
                },
                ..
            } if name == "increment"
        ));
        assert!(matches!(
            hir.functions[1].body.statements[0],
            HirStmt::Return {
                value: Some(HirExpr {
                    kind: HirExprKind::Call { ref name, function: None, .. },
                    ..
                }),
                ..
            } if name == "callback"
        ));
    }
}
