use std::collections::HashMap;
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
    pub apis: Vec<ApiDef>,
    pub functions: Vec<HirFunction>,
    pub functions_by_name: HashMap<String, FunctionId>,
}

#[derive(Clone, Debug)]
pub struct HirFunction {
    pub id: FunctionId,
    pub name: String,
    pub params: Vec<HirParam>,
    pub return_type: Option<Type>,
    pub capabilities: Vec<String>,
    pub requires: Vec<HirExpr>,
    pub ensures: Vec<HirExpr>,
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
pub struct HirMatchArm {
    pub pattern: HirPattern,
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
    Record {
        type_name: String,
        fields: Vec<(String, HirExpr)>,
    },
    Local(LocalId),
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
    Sql {
        result_type: Type,
        query: String,
    },
    Await(Box<HirExpr>),
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
    for (index, function) in program.functions.iter().enumerate() {
        let mut resolver = Resolver {
            functions: &functions_by_name,
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
        resolver.bind("result".into(), function.span);
        let ensures = function
            .ensures
            .iter()
            .map(|contract| resolver.expr(contract))
            .collect();
        errors.extend(resolver.errors);
        functions.push(HirFunction {
            id: FunctionId(index),
            name: function.name.clone(),
            params,
            return_type: function.return_type.clone(),
            capabilities: function.capabilities.clone(),
            requires,
            ensures,
            body,
            span: function.span,
        });
    }

    if errors.is_empty() {
        Ok(HirProgram {
            types: program.types.clone(),
            records: program.records.clone(),
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
                iterable,
                body,
                span,
            } => {
                let iterable = self.expr(iterable);
                self.scopes.push(HashMap::new());
                let local = self.bind(name.clone(), *span);
                let body = self.block(body);
                self.scopes.pop();
                HirStmt::For {
                    name: name.clone(),
                    local,
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
                        let body = self.block(&arm.body);
                        self.scopes.pop();
                        HirMatchArm {
                            pattern,
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
            ExprKind::Record { type_name, fields } => HirExprKind::Record {
                type_name: type_name.clone(),
                fields: fields
                    .iter()
                    .map(|(name, value)| (name.clone(), self.expr(value)))
                    .collect(),
            },
            ExprKind::Variable(name) => match self.lookup(name) {
                Some(local) => HirExprKind::Local(local),
                None if name == "None" => HirExprKind::Call {
                    name: name.clone(),
                    function: None,
                    args: Vec::new(),
                },
                None => {
                    self.error(expr.span, format!("unknown variable `{name}`"));
                    HirExprKind::Local(LocalId(usize::MAX))
                }
            },
            ExprKind::Call { name, args } => {
                let function = self.functions.get(name).copied();
                if function.is_none()
                    && !matches!(
                        name.as_str(),
                        "print"
                            | "len"
                            | "append"
                            | "contains"
                            | "first"
                            | "last"
                            | "Some"
                            | "Ok"
                            | "Err"
                            | "now"
                            | "env"
                            | "random_int"
                    )
                {
                    self.error(expr.span, format!("unknown function `{name}`"));
                }
                HirExprKind::Call {
                    name: name.clone(),
                    function,
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
            ExprKind::Sql { result_type, query } => HirExprKind::Sql {
                result_type: result_type.clone(),
                query: query.clone(),
            },
            ExprKind::Await(inner) => HirExprKind::Await(Box::new(self.expr(inner))),
        };
        HirExpr {
            kind,
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
}
