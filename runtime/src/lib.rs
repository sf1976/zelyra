use std::collections::HashMap;
use std::fmt;
use zelyra_ast::*;

#[derive(Clone, Debug, PartialEq)]
pub struct TypeError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Clone)]
struct Variable {
    ty: Type,
    mutable: bool,
}

#[derive(Clone)]
struct FunctionSignature {
    params: Vec<Type>,
    return_type: Type,
}

struct Checker<'a> {
    functions: HashMap<String, FunctionSignature>,
    known_types: std::collections::HashSet<String>,
    errors: Vec<TypeError>,
    loop_depth: usize,
    _program: &'a Program,
}

pub fn check(program: &Program) -> Result<(), Vec<TypeError>> {
    let mut known_types = [
        "Id",
        "Int",
        "UInt",
        "Float",
        "Decimal",
        "Bool",
        "String",
        "Char",
        "Bytes",
        "Timestamp",
        "Date",
        "Time",
        "Duration",
        "Unit",
    ]
    .into_iter()
    .map(String::from)
    .collect::<std::collections::HashSet<_>>();
    let mut errors = Vec::new();
    for definition in &program.types {
        if !known_types.insert(definition.name.clone()) {
            errors.push(TypeError {
                message: format!("duplicate type `{}`", definition.name),
                span: definition.span,
            });
        }
    }
    for table in &program.tables {
        known_types.insert(table.name.clone());
        if let Some(singular) = singular_table_type(&table.name) {
            known_types.insert(singular);
        }
    }
    for definition in &program.types {
        validate_type(
            &definition.target,
            &known_types,
            &mut errors,
            definition.span,
        );
    }
    let mut functions = HashMap::new();
    for function in &program.functions {
        if functions.contains_key(&function.name) {
            errors.push(TypeError {
                message: format!("duplicate function `{}`", function.name),
                span: function.span,
            });
        } else {
            functions.insert(
                function.name.clone(),
                FunctionSignature {
                    params: function
                        .params
                        .iter()
                        .map(|param| param.ty.clone())
                        .collect(),
                    return_type: function.return_type.clone().unwrap_or(Type::Unknown),
                },
            );
        }
    }
    if !functions.contains_key("main") {
        errors.push(TypeError {
            message: "program must define `fn main()`".into(),
            span: Span::default(),
        });
    }
    let mut checker = Checker {
        functions,
        known_types,
        errors,
        loop_depth: 0,
        _program: program,
    };
    for function in &program.functions {
        for parameter in &function.params {
            checker.check_type(&parameter.ty, parameter.span);
        }
        if let Some(return_type) = &function.return_type {
            checker.check_type(return_type, function.span);
        }
    }
    for function in &program.functions {
        checker.check_function(function);
    }
    if checker.errors.is_empty() {
        Ok(())
    } else {
        Err(checker.errors)
    }
}

impl<'a> Checker<'a> {
    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.errors.push(TypeError {
            message: message.into(),
            span,
        });
    }
    fn lookup<'b>(
        &self,
        scopes: &'b [HashMap<String, Variable>],
        name: &str,
    ) -> Option<&'b Variable> {
        scopes.iter().rev().find_map(|scope| scope.get(name))
    }
    fn check_function(&mut self, function: &Function) {
        let mut scopes = vec![HashMap::new()];
        for parameter in &function.params {
            if scopes[0]
                .insert(
                    parameter.name.clone(),
                    Variable {
                        ty: parameter.ty.clone(),
                        mutable: false,
                    },
                )
                .is_some()
            {
                self.error(
                    parameter.span,
                    format!("duplicate parameter `{}`", parameter.name),
                );
            }
        }
        let expected = function.return_type.clone().unwrap_or(Type::Unknown);
        self.check_block(&function.body, &mut scopes, &expected);
    }
    fn check_block(
        &mut self,
        block: &Block,
        scopes: &mut Vec<HashMap<String, Variable>>,
        expected: &Type,
    ) {
        scopes.push(HashMap::new());
        for statement in &block.statements {
            self.check_statement(statement, scopes, expected);
        }
        scopes.pop();
    }
    fn check_statement(
        &mut self,
        statement: &Stmt,
        scopes: &mut Vec<HashMap<String, Variable>>,
        expected: &Type,
    ) {
        match statement {
            Stmt::Let {
                name,
                ty,
                value,
                mutable,
                span,
            } => {
                let actual = self.check_expr(value, scopes);
                if let Some(declared) = ty {
                    self.expect_type(declared, &actual, value.span);
                }
                let final_ty = ty.clone().unwrap_or(actual);
                if scopes
                    .last_mut()
                    .unwrap()
                    .insert(
                        name.clone(),
                        Variable {
                            ty: final_ty,
                            mutable: *mutable,
                        },
                    )
                    .is_some()
                {
                    self.error(
                        *span,
                        format!("variable `{name}` is already declared in this scope"),
                    );
                }
            }
            Stmt::BindOrAssign { name, value, span } => {
                let actual = self.check_expr(value, scopes);
                if let Some(existing) = self.lookup(scopes, name).cloned() {
                    if !existing.mutable {
                        self.error(
                            *span,
                            format!("cannot assign to immutable variable `{name}`"),
                        );
                    }
                    self.expect_type(&existing.ty, &actual, value.span);
                } else {
                    scopes.last_mut().unwrap().insert(
                        name.clone(),
                        Variable {
                            ty: actual,
                            mutable: false,
                        },
                    );
                }
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr, scopes);
            }
            Stmt::Return { value, span } => {
                let actual = value
                    .as_ref()
                    .map(|expr| self.check_expr(expr, scopes))
                    .unwrap_or(Type::Unit);
                if *expected != Type::Unknown {
                    self.expect_type(expected, &actual, *span);
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let condition_type = self.check_expr(condition, scopes);
                self.expect_type(&Type::Bool, &condition_type, condition.span);
                self.check_block(then_block, scopes, expected);
                if let Some(block) = else_block {
                    self.check_block(block, scopes, expected);
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                let condition_type = self.check_expr(condition, scopes);
                self.expect_type(&Type::Bool, &condition_type, condition.span);
                self.loop_depth += 1;
                self.check_block(body, scopes, expected);
                self.loop_depth -= 1;
            }
            Stmt::Loop { body, .. } => {
                self.loop_depth += 1;
                self.check_block(body, scopes, expected);
                self.loop_depth -= 1;
            }
            Stmt::Break { span } => {
                if self.loop_depth == 0 {
                    self.error(*span, "`break` is only valid inside a loop");
                }
            }
            Stmt::Match { value, arms, span } => {
                let value_type = self.check_expr(value, scopes);
                let mut covered = std::collections::HashSet::new();
                for arm in arms {
                    let mut pattern_scope = HashMap::new();
                    self.check_pattern(
                        &arm.pattern,
                        &value_type,
                        &mut pattern_scope,
                        &mut covered,
                        true,
                    );
                    scopes.push(pattern_scope);
                    self.check_block(&arm.body, scopes, expected);
                    scopes.pop();
                }
                self.check_exhaustiveness(&value_type, &covered, *span);
            }
            Stmt::Transaction { body, .. } => self.check_block(body, scopes, expected),
        }
    }
    fn check_type(&mut self, ty: &Type, span: Span) {
        match ty {
            Type::Named(name) if !self.known_types.contains(name) => {
                self.error(span, format!("unknown type `{name}`"));
            }
            Type::Option(inner) => self.check_type(inner, span),
            Type::Result(ok, error) => {
                self.check_type(ok, span);
                self.check_type(error, span);
            }
            Type::Array(inner) => self.check_type(inner, span),
            _ => {}
        }
    }
    fn check_pattern(
        &mut self,
        pattern: &Pattern,
        expected: &Type,
        scope: &mut HashMap<String, Variable>,
        covered: &mut std::collections::HashSet<String>,
        top_level: bool,
    ) {
        match &pattern.kind {
            PatternKind::Wildcard => {
                if top_level {
                    covered.insert("_".into());
                }
            }
            PatternKind::Variable(name) => {
                if top_level {
                    covered.insert("_".into());
                }
                if scope
                    .insert(
                        name.clone(),
                        Variable {
                            ty: expected.clone(),
                            mutable: false,
                        },
                    )
                    .is_some()
                {
                    self.error(
                        pattern.span,
                        format!("pattern variable `{name}` is bound twice"),
                    );
                }
            }
            PatternKind::Constructor { name, inner } => {
                if top_level {
                    covered.insert(name.clone());
                }
                match (name.as_str(), expected) {
                    ("Some", Type::Option(inner_type)) => {
                        if let Some(inner) = inner {
                            self.check_pattern(inner, inner_type, scope, covered, false);
                        } else {
                            self.error(pattern.span, "`Some` requires an inner pattern");
                        }
                    }
                    ("None", Type::Option(_)) => {
                        if inner.is_some() {
                            self.error(pattern.span, "`None` does not accept an inner pattern");
                        }
                    }
                    ("Ok", Type::Result(ok_type, _)) => {
                        if let Some(inner) = inner {
                            self.check_pattern(inner, ok_type, scope, covered, false);
                        } else {
                            self.error(pattern.span, "`Ok` requires an inner pattern");
                        }
                    }
                    ("Err", Type::Result(_, error_type)) => {
                        if let Some(inner) = inner {
                            self.check_pattern(inner, error_type, scope, covered, false);
                        } else {
                            self.error(pattern.span, "`Err` requires an inner pattern");
                        }
                    }
                    ("Some" | "None" | "Ok" | "Err", Type::Unknown) => {}
                    _ => self.error(
                        pattern.span,
                        format!("constructor `{name}` does not match `{expected}`"),
                    ),
                }
            }
            PatternKind::Int(_) => self.expect_type(&Type::Int, expected, pattern.span),
            PatternKind::Bool(value) => {
                if top_level {
                    covered.insert(value.to_string());
                }
                self.expect_type(&Type::Bool, expected, pattern.span);
            }
            PatternKind::String(_) => self.expect_type(&Type::String, expected, pattern.span),
            PatternKind::Char(_) => self.expect_type(&Type::Char, expected, pattern.span),
        }
    }
    fn check_exhaustiveness(
        &mut self,
        ty: &Type,
        covered: &std::collections::HashSet<String>,
        span: Span,
    ) {
        if covered.contains("_") {
            return;
        }
        let missing = match ty {
            Type::Option(_) => {
                let mut missing = Vec::new();
                if !covered.contains("Some") {
                    missing.push("Some(value)");
                }
                if !covered.contains("None") {
                    missing.push("None");
                }
                missing.join(", ")
            }
            Type::Result(_, _) => {
                let mut missing = Vec::new();
                if !covered.contains("Ok") {
                    missing.push("Ok(value)");
                }
                if !covered.contains("Err") {
                    missing.push("Err(error)");
                }
                missing.join(", ")
            }
            Type::Bool => {
                let mut missing = Vec::new();
                if !covered.contains("true") {
                    missing.push("true");
                }
                if !covered.contains("false") {
                    missing.push("false");
                }
                missing.join(", ")
            }
            _ => String::new(),
        };
        if !missing.is_empty() {
            self.error(span, format!("non-exhaustive match; missing: {missing}"));
        }
    }
    fn expect_type(&mut self, expected: &Type, actual: &Type, span: Span) {
        if !compatible(expected, actual) {
            self.error(
                span,
                format!("type mismatch: expected `{expected}`, found `{actual}`"),
            );
        }
    }
    fn check_expr(&mut self, expr: &Expr, scopes: &[HashMap<String, Variable>]) -> Type {
        match &expr.kind {
            ExprKind::Int(_) => Type::Int,
            ExprKind::UInt(_) => Type::UInt,
            ExprKind::Float(_) => Type::Float,
            ExprKind::Bool(_) => Type::Bool,
            ExprKind::String(_) => Type::String,
            ExprKind::Char(_) => Type::Char,
            ExprKind::Variable(name) => self
                .lookup(scopes, name)
                .map(|v| v.ty.clone())
                .or_else(|| {
                    if name == "None" {
                        Some(Type::Option(Box::new(Type::Unknown)))
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    self.error(expr.span, format!("unknown variable `{name}`"));
                    Type::Unknown
                }),
            ExprKind::Call { name, args } => {
                if name == "print" {
                    if args.len() != 1 {
                        self.error(expr.span, "`print` expects exactly one argument");
                    }
                    for arg in args {
                        self.check_expr(arg, scopes);
                    }
                    Type::Unit
                } else if name == "Some" || name == "Ok" || name == "Err" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("`{name}` expects exactly one argument"));
                        return Type::Unknown;
                    }
                    let inner = self.check_expr(&args[0], scopes);
                    if name == "Some" {
                        Type::Option(Box::new(inner))
                    } else if name == "Ok" {
                        Type::Result(Box::new(inner), Box::new(Type::Unknown))
                    } else {
                        Type::Result(Box::new(Type::Unknown), Box::new(inner))
                    }
                } else if let Some(signature) = self.functions.get(name).cloned() {
                    if args.len() != signature.params.len() {
                        self.error(
                            expr.span,
                            format!(
                                "function `{name}` expects {} argument(s), found {}",
                                signature.params.len(),
                                args.len()
                            ),
                        );
                    }
                    for (index, arg) in args.iter().enumerate() {
                        let actual = self.check_expr(arg, scopes);
                        if let Some(expected) = signature.params.get(index) {
                            self.expect_type(expected, &actual, arg.span);
                        }
                    }
                    signature.return_type
                } else {
                    self.error(expr.span, format!("unknown function `{name}`"));
                    Type::Unknown
                }
            }
            ExprKind::Unary { op, expr: inner } => {
                let ty = self.check_expr(inner, scopes);
                match op {
                    UnaryOp::Negate => {
                        if !is_numeric(&ty) && ty != Type::Unknown {
                            self.error(expr.span, "unary `-` requires a numeric value");
                        }
                        ty
                    }
                    UnaryOp::Not => {
                        self.expect_type(&Type::Bool, &ty, inner.span);
                        Type::Bool
                    }
                }
            }
            ExprKind::Binary { left, op, right } => {
                let left_ty = self.check_expr(left, scopes);
                let right_ty = self.check_expr(right, scopes);
                match op {
                    BinaryOp::Add => {
                        if left_ty == Type::String && right_ty == Type::String {
                            Type::String
                        } else {
                            self.numeric_result(&left_ty, &right_ty, expr.span)
                        }
                    }
                    BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Remainder => self.numeric_result(&left_ty, &right_ty, expr.span),
                    BinaryOp::Equal | BinaryOp::NotEqual => {
                        self.expect_type(&left_ty, &right_ty, right.span);
                        Type::Bool
                    }
                    BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => {
                        self.numeric_result(&left_ty, &right_ty, expr.span);
                        Type::Bool
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        self.expect_type(&Type::Bool, &left_ty, left.span);
                        self.expect_type(&Type::Bool, &right_ty, right.span);
                        Type::Bool
                    }
                }
            }
            ExprKind::Sql { result_type, .. } => {
                self.check_type(result_type, expr.span);
                result_type.clone()
            }
        }
    }
    fn numeric_result(&mut self, left: &Type, right: &Type, span: Span) -> Type {
        if left == &Type::Unknown || right == &Type::Unknown {
            return Type::Unknown;
        }
        if is_numeric(left) && left == right {
            left.clone()
        } else {
            self.error(
                span,
                format!("numeric operands must have the same type, found `{left}` and `{right}`"),
            );
            Type::Unknown
        }
    }
}

fn compatible(expected: &Type, actual: &Type) -> bool {
    if expected == &Type::Unknown || actual == &Type::Unknown {
        return true;
    }
    match (expected, actual) {
        (Type::Option(expected), Type::Option(actual)) => compatible(expected, actual),
        (Type::Result(expected_ok, expected_error), Type::Result(actual_ok, actual_error)) => {
            compatible(expected_ok, actual_ok) && compatible(expected_error, actual_error)
        }
        (Type::Array(expected), Type::Array(actual)) => compatible(expected, actual),
        _ => expected == actual,
    }
}
fn is_numeric(ty: &Type) -> bool {
    matches!(ty, Type::Int | Type::UInt | Type::Float | Type::Decimal)
}

fn validate_type(
    ty: &Type,
    known_types: &std::collections::HashSet<String>,
    errors: &mut Vec<TypeError>,
    span: Span,
) {
    match ty {
        Type::Named(name) if !known_types.contains(name) => errors.push(TypeError {
            message: format!("unknown type `{name}`"),
            span,
        }),
        Type::Option(inner) => validate_type(inner, known_types, errors, span),
        Type::Result(ok, error) => {
            validate_type(ok, known_types, errors, span);
            validate_type(error, known_types, errors, span);
        }
        Type::Array(inner) => validate_type(inner, known_types, errors, span),
        _ => {}
    }
}

fn singular_table_type(name: &str) -> Option<String> {
    let singular = if let Some(stem) = name.strip_suffix("ies") {
        format!("{stem}y")
    } else if let Some(stem) = name.strip_suffix('s') {
        stem.to_owned()
    } else {
        name.to_owned()
    };
    let mut chars = singular.chars();
    let first = chars.next()?.to_ascii_uppercase();
    Some(std::iter::once(first).chain(chars).collect())
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
    Char(char),
    Option(Option<Box<Value>>),
    Result(Result<Box<Value>, Box<Value>>),
    Rows {
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Unit,
}

impl Value {
    pub fn ty(&self) -> Type {
        match self {
            Value::Int(_) => Type::Int,
            Value::UInt(_) => Type::UInt,
            Value::Float(_) => Type::Float,
            Value::Bool(_) => Type::Bool,
            Value::String(_) => Type::String,
            Value::Char(_) => Type::Char,
            Value::Option(Some(value)) => Type::Option(Box::new(value.ty())),
            Value::Option(None) => Type::Option(Box::new(Type::Unknown)),
            Value::Result(Ok(value)) => Type::Result(Box::new(value.ty()), Box::new(Type::Unknown)),
            Value::Result(Err(value)) => {
                Type::Result(Box::new(Type::Unknown), Box::new(value.ty()))
            }
            Value::Rows { .. } => Type::Array(Box::new(Type::Unknown)),
            Value::Unit => Type::Unit,
        }
    }
    pub fn output(&self) -> String {
        match self {
            Value::Int(v) => v.to_string(),
            Value::UInt(v) => v.to_string(),
            Value::Float(v) => v.to_string(),
            Value::Bool(v) => v.to_string(),
            Value::String(v) => v.clone(),
            Value::Char(v) => v.to_string(),
            Value::Option(Some(v)) => format!("Some({})", v.output()),
            Value::Option(None) => "None".into(),
            Value::Result(Ok(v)) => format!("Ok({})", v.output()),
            Value::Result(Err(v)) => format!("Err({})", v.output()),
            Value::Rows { columns, rows } => {
                let rendered_rows = rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .enumerate()
                            .map(|(index, value)| {
                                let column = columns.get(index).map(String::as_str).unwrap_or("?");
                                format!("{column}={value}")
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("[{rendered_rows}]")
            }
            Value::Unit => "()".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Clone)]
struct RuntimeVariable {
    value: Value,
    mutable: bool,
}

struct Environment {
    scopes: Vec<HashMap<String, RuntimeVariable>>,
}

impl Environment {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }
    fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }
    fn pop(&mut self) {
        self.scopes.pop();
    }
    fn declare(&mut self, name: String, value: Value, mutable: bool) {
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name, RuntimeVariable { value, mutable });
    }
    fn contains(&self, name: &str) -> bool {
        self.scopes
            .iter()
            .rev()
            .any(|scope| scope.contains_key(name))
    }
    fn get(&self, name: &str) -> Option<Value> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).map(|v| v.value.clone()))
    }
    fn assign(&mut self, name: &str, value: Value) -> Result<(), &'static str> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(variable) = scope.get_mut(name) {
                if !variable.mutable {
                    return Err("immutable");
                }
                variable.value = value;
                return Ok(());
            }
        }
        Err("missing")
    }
}

enum Flow {
    Continue,
    Return(Value),
    Break,
}

pub fn execute(program: &Program) -> Result<Vec<String>, RuntimeError> {
    execute_internal(program, None)
}

pub fn execute_with_database(
    program: &Program,
    database_url: &str,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal(program, Some(database_url.to_owned()))
}

fn execute_internal(
    program: &Program,
    database_url: Option<String>,
) -> Result<Vec<String>, RuntimeError> {
    let mut interpreter = Interpreter {
        functions: program
            .functions
            .iter()
            .map(|f| (f.name.clone(), f.clone()))
            .collect(),
        output: Vec::new(),
        steps: 0,
        database_url,
    };
    interpreter.call("main", Vec::new(), Span::default())?;
    Ok(interpreter.output)
}

struct Interpreter {
    functions: HashMap<String, Function>,
    output: Vec<String>,
    steps: usize,
    database_url: Option<String>,
}

impl Interpreter {
    fn runtime_error(&self, span: Span, message: impl Into<String>) -> RuntimeError {
        RuntimeError {
            message: message.into(),
            span,
        }
    }
    fn call(&mut self, name: &str, args: Vec<Value>, span: Span) -> Result<Value, RuntimeError> {
        let function = self
            .functions
            .get(name)
            .cloned()
            .ok_or_else(|| self.runtime_error(span, format!("unknown function `{name}`")))?;
        if function.params.len() != args.len() {
            return Err(self.runtime_error(
                span,
                format!(
                    "function `{name}` expects {} argument(s)",
                    function.params.len()
                ),
            ));
        }
        let mut env = Environment::new();
        for (param, value) in function.params.iter().zip(args) {
            env.declare(param.name.clone(), value, false);
        }
        match self.exec_block(&function.body, &mut env)? {
            Flow::Return(value) => Ok(value),
            Flow::Continue | Flow::Break => Ok(Value::Unit),
        }
    }
    fn exec_block(&mut self, block: &Block, env: &mut Environment) -> Result<Flow, RuntimeError> {
        env.push();
        for statement in &block.statements {
            match self.exec_statement(statement, env)? {
                Flow::Continue => {}
                flow => {
                    env.pop();
                    return Ok(flow);
                }
            }
        }
        env.pop();
        Ok(Flow::Continue)
    }
    fn exec_statement(
        &mut self,
        statement: &Stmt,
        env: &mut Environment,
    ) -> Result<Flow, RuntimeError> {
        self.steps += 1;
        if self.steps > 10_000_000 {
            return Err(self.runtime_error(Span::default(), "execution step limit exceeded"));
        }
        match statement {
            Stmt::Let {
                name,
                value,
                mutable,
                ..
            } => {
                let value = self.eval(value, env)?;
                env.declare(name.clone(), value, *mutable);
                Ok(Flow::Continue)
            }
            Stmt::BindOrAssign { name, value, span } => {
                let value = self.eval(value, env)?;
                if env.contains(name) {
                    env.assign(name, value).map_err(|reason| {
                        self.runtime_error(
                            *span,
                            if reason == "immutable" {
                                format!("cannot assign to immutable variable `{name}`")
                            } else {
                                format!("unknown variable `{name}`")
                            },
                        )
                    })?;
                } else {
                    env.declare(name.clone(), value, false);
                }
                Ok(Flow::Continue)
            }
            Stmt::Expr(expr) => {
                self.eval(expr, env)?;
                Ok(Flow::Continue)
            }
            Stmt::Return { value, .. } => Ok(Flow::Return(
                value
                    .as_ref()
                    .map(|expr| self.eval(expr, env))
                    .transpose()?
                    .unwrap_or(Value::Unit),
            )),
            Stmt::If {
                condition,
                then_block,
                else_block,
                span,
            } => {
                let condition_value = self.eval(condition, env)?;
                if self.expect_bool(condition_value, *span)? {
                    self.exec_block(then_block, env)
                } else if let Some(block) = else_block {
                    self.exec_block(block, env)
                } else {
                    Ok(Flow::Continue)
                }
            }
            Stmt::While {
                condition,
                body,
                span,
            } => {
                loop {
                    let condition_value = self.eval(condition, env)?;
                    if !self.expect_bool(condition_value, *span)? {
                        break;
                    }
                    match self.exec_block(body, env)? {
                        Flow::Continue => {}
                        Flow::Break => break,
                        flow @ Flow::Return(_) => return Ok(flow),
                    }
                }
                Ok(Flow::Continue)
            }
            Stmt::Loop { body, .. } => {
                loop {
                    match self.exec_block(body, env)? {
                        Flow::Continue => {}
                        Flow::Break => break,
                        flow @ Flow::Return(_) => return Ok(flow),
                    }
                }
                Ok(Flow::Continue)
            }
            Stmt::Break { .. } => Ok(Flow::Break),
            Stmt::Match { value, arms, span } => {
                let scrutinee = self.eval(value, env)?;
                for arm in arms {
                    if let Some(bindings) = match_pattern(&scrutinee, &arm.pattern) {
                        env.push();
                        for (name, value) in bindings {
                            env.declare(name, value, false);
                        }
                        let flow = self.exec_block(&arm.body, env)?;
                        env.pop();
                        return Ok(flow);
                    }
                }
                Err(self.runtime_error(*span, "non-exhaustive match at runtime"))
            }
            Stmt::Transaction { span, .. } => {
                let Some(database_url) = self.database_url.clone() else {
                    return Err(self.runtime_error(
                        *span,
                        "transaction execution requires a database runtime",
                    ));
                };
                let Stmt::Transaction { body, .. } = statement else {
                    unreachable!();
                };
                let queries = transaction_queries(body, env, *span)?;
                zelyra_database::execute_mariadb_queries(&database_url, &queries, true).map_err(
                    |error| {
                        self.runtime_error(*span, format!("MariaDB transaction failed: {error}"))
                    },
                )?;
                Ok(Flow::Continue)
            }
        }
    }
    fn expect_bool(&self, value: Value, span: Span) -> Result<bool, RuntimeError> {
        match value {
            Value::Bool(v) => Ok(v),
            other => Err(self.runtime_error(span, format!("expected Bool, found {}", other.ty()))),
        }
    }
    fn eval(&mut self, expr: &Expr, env: &mut Environment) -> Result<Value, RuntimeError> {
        match &expr.kind {
            ExprKind::Int(v) => Ok(Value::Int(*v)),
            ExprKind::UInt(v) => Ok(Value::UInt(*v)),
            ExprKind::Float(v) => Ok(Value::Float(*v)),
            ExprKind::Bool(v) => Ok(Value::Bool(*v)),
            ExprKind::String(v) => Ok(Value::String(v.clone())),
            ExprKind::Char(v) => Ok(Value::Char(*v)),
            ExprKind::Variable(name) => env
                .get(name)
                .or_else(|| {
                    if name == "None" {
                        Some(Value::Option(None))
                    } else {
                        None
                    }
                })
                .ok_or_else(|| self.runtime_error(expr.span, format!("unknown variable `{name}`"))),
            ExprKind::Call { name, args } => {
                if name == "print" {
                    let value = self.eval(
                        args.first().ok_or_else(|| {
                            self.runtime_error(expr.span, "`print` expects one argument")
                        })?,
                        env,
                    )?;
                    self.output.push(value.output());
                    Ok(Value::Unit)
                } else if name == "Some" || name == "Ok" || name == "Err" {
                    let argument = args.first().ok_or_else(|| {
                        self.runtime_error(expr.span, format!("`{name}` expects one argument"))
                    })?;
                    if args.len() != 1 {
                        return Err(self.runtime_error(
                            expr.span,
                            format!("`{name}` expects exactly one argument"),
                        ));
                    }
                    let value = Box::new(self.eval(argument, env)?);
                    match name.as_str() {
                        "Some" => Ok(Value::Option(Some(value))),
                        "Ok" => Ok(Value::Result(Ok(value))),
                        _ => Ok(Value::Result(Err(value))),
                    }
                } else {
                    let values = args
                        .iter()
                        .map(|arg| self.eval(arg, env))
                        .collect::<Result<Vec<_>, _>>()?;
                    self.call(name, values, expr.span)
                }
            }
            ExprKind::Unary { op, expr: inner } => {
                let value = self.eval(inner, env)?;
                match (op, value) {
                    (UnaryOp::Negate, Value::Int(v)) => Ok(Value::Int(-v)),
                    (UnaryOp::Negate, Value::UInt(_)) => {
                        Err(self.runtime_error(expr.span, "cannot negate UInt"))
                    }
                    (UnaryOp::Negate, Value::Float(v)) => Ok(Value::Float(-v)),
                    (UnaryOp::Not, Value::Bool(v)) => Ok(Value::Bool(!v)),
                    (_, value) => Err(self.runtime_error(
                        expr.span,
                        format!("invalid unary operand of type {}", value.ty()),
                    )),
                }
            }
            ExprKind::Binary { left, op, right } => {
                let l = self.eval(left, env)?;
                if *op == BinaryOp::And && matches!(l, Value::Bool(false)) {
                    return Ok(Value::Bool(false));
                }
                if *op == BinaryOp::Or && matches!(l, Value::Bool(true)) {
                    return Ok(Value::Bool(true));
                }
                let r = self.eval(right, env)?;
                self.binary(l, *op, r, expr.span)
            }
            ExprKind::Sql { .. } => {
                let ExprKind::Sql { query, .. } = &expr.kind else {
                    unreachable!();
                };
                let Some(database_url) = self.database_url.clone() else {
                    return Err(
                        self.runtime_error(expr.span, "SQL execution requires a database runtime")
                    );
                };
                let params = query_parameters(query, env, expr.span)?;
                let result = zelyra_database::execute_mariadb_query(&database_url, query, params)
                    .map_err(|error| {
                    self.runtime_error(expr.span, format!("MariaDB query failed: {error}"))
                })?;
                Ok(Value::Rows {
                    columns: result.columns,
                    rows: result.rows,
                })
            }
        }
    }
    fn binary(
        &self,
        left: Value,
        op: BinaryOp,
        right: Value,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        use BinaryOp::*;
        match (left, op, right) {
            (Value::Int(a), Add, Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Int(a), Subtract, Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Int(a), Multiply, Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Int(_), Divide, Value::Int(0)) | (Value::Int(_), Remainder, Value::Int(0)) => {
                Err(self.runtime_error(span, "division by zero"))
            }
            (Value::Int(a), Divide, Value::Int(b)) => Ok(Value::Int(a / b)),
            (Value::Int(a), Remainder, Value::Int(b)) => Ok(Value::Int(a % b)),
            (Value::Float(a), Add, Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Float(a), Subtract, Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Float(a), Multiply, Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Float(a), Divide, Value::Float(b)) => Ok(Value::Float(a / b)),
            (Value::Float(a), Remainder, Value::Float(b)) => Ok(Value::Float(a % b)),
            (Value::String(a), Add, Value::String(b)) => Ok(Value::String(a + &b)),
            (a, Equal, b) => Ok(Value::Bool(a == b)),
            (a, NotEqual, b) => Ok(Value::Bool(a != b)),
            (Value::Int(a), Less, Value::Int(b)) => Ok(Value::Bool(a < b)),
            (Value::Int(a), LessEqual, Value::Int(b)) => Ok(Value::Bool(a <= b)),
            (Value::Int(a), Greater, Value::Int(b)) => Ok(Value::Bool(a > b)),
            (Value::Int(a), GreaterEqual, Value::Int(b)) => Ok(Value::Bool(a >= b)),
            (Value::Float(a), Less, Value::Float(b)) => Ok(Value::Bool(a < b)),
            (Value::Float(a), LessEqual, Value::Float(b)) => Ok(Value::Bool(a <= b)),
            (Value::Float(a), Greater, Value::Float(b)) => Ok(Value::Bool(a > b)),
            (Value::Float(a), GreaterEqual, Value::Float(b)) => Ok(Value::Bool(a >= b)),
            (Value::Bool(a), And, Value::Bool(b)) => Ok(Value::Bool(a && b)),
            (Value::Bool(a), Or, Value::Bool(b)) => Ok(Value::Bool(a || b)),
            (left, op, right) => Err(self.runtime_error(
                span,
                format!(
                    "invalid operation {op:?} for {} and {}",
                    left.ty(),
                    right.ty()
                ),
            )),
        }
    }
}

fn query_parameters(
    query: &str,
    env: &Environment,
    span: Span,
) -> Result<Vec<(String, zelyra_database::QueryValue)>, RuntimeError> {
    let bytes = query.as_bytes();
    let mut parameters = Vec::new();
    let mut index = 0;
    let mut quote = None;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(active_quote) = quote {
            if byte == active_quote {
                if bytes.get(index + 1) == Some(&active_quote) {
                    index += 2;
                    continue;
                }
                quote = None;
            }
            index += 1;
            continue;
        }
        if byte == b'\'' || byte == b'"' {
            quote = Some(byte);
            index += 1;
            continue;
        }
        if byte == b':' {
            let start = index + 1;
            let mut end = start;
            while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                end += 1;
            }
            if end > start {
                let name = query[start..end].to_owned();
                if !parameters.iter().any(|(existing, _)| existing == &name) {
                    let value = env.get(&name).ok_or_else(|| RuntimeError {
                        message: format!("SQL parameter `:{name}` is not available"),
                        span,
                    })?;
                    parameters.push((name, value_to_query_value(&value, span)?));
                }
                index = end;
                continue;
            }
        }
        index += 1;
    }
    Ok(parameters)
}

fn value_to_query_value(
    value: &Value,
    span: Span,
) -> Result<zelyra_database::QueryValue, RuntimeError> {
    use zelyra_database::QueryValue;
    match value {
        Value::Int(value) => Ok(QueryValue::Int(*value)),
        Value::UInt(value) => Ok(QueryValue::UInt(*value)),
        Value::Float(value) => Ok(QueryValue::Float(*value)),
        Value::Bool(value) => Ok(QueryValue::Bool(*value)),
        Value::String(value) => Ok(QueryValue::String(value.clone())),
        Value::Char(value) => Ok(QueryValue::String(value.to_string())),
        Value::Option(None) | Value::Unit => Ok(QueryValue::Null),
        Value::Option(Some(value)) => value_to_query_value(value, span),
        Value::Rows { .. } | Value::Result(_) => Err(RuntimeError {
            message: "SQL parameters must be scalar values".into(),
            span,
        }),
    }
}

fn transaction_queries(
    block: &Block,
    env: &Environment,
    span: Span,
) -> Result<Vec<zelyra_database::Query>, RuntimeError> {
    let mut queries = Vec::new();
    for statement in &block.statements {
        let expression = match statement {
            Stmt::Expr(expression) => expression,
            Stmt::Let { value, .. } => value,
            _ => {
                return Err(RuntimeError {
                    message: "transactions may contain only SQL statements".into(),
                    span,
                })
            }
        };
        let ExprKind::Sql { query, .. } = &expression.kind else {
            return Err(RuntimeError {
                message: "transactions may contain only SQL statements".into(),
                span: expression.span,
            });
        };
        queries.push(zelyra_database::Query {
            sql: query.clone(),
            params: query_parameters(query, env, expression.span)?,
        });
    }
    if queries.is_empty() {
        return Err(RuntimeError {
            message: "transaction must contain at least one SQL statement".into(),
            span,
        });
    }
    Ok(queries)
}

fn match_pattern(value: &Value, pattern: &Pattern) -> Option<Vec<(String, Value)>> {
    match &pattern.kind {
        PatternKind::Wildcard => Some(Vec::new()),
        PatternKind::Variable(name) => Some(vec![(name.clone(), value.clone())]),
        PatternKind::Int(expected) => match value {
            Value::Int(actual) if actual == expected => Some(Vec::new()),
            _ => None,
        },
        PatternKind::Bool(expected) => match value {
            Value::Bool(actual) if actual == expected => Some(Vec::new()),
            _ => None,
        },
        PatternKind::String(expected) => match value {
            Value::String(actual) if actual == expected => Some(Vec::new()),
            _ => None,
        },
        PatternKind::Char(expected) => match value {
            Value::Char(actual) if actual == expected => Some(Vec::new()),
            _ => None,
        },
        PatternKind::Constructor { name, inner } => match (name.as_str(), value) {
            ("Some", Value::Option(Some(value))) | ("Ok", Value::Result(Ok(value))) => inner
                .as_ref()
                .and_then(|pattern| match_pattern(value, pattern)),
            ("None", Value::Option(None)) => {
                if inner.is_none() {
                    Some(Vec::new())
                } else {
                    None
                }
            }
            ("Err", Value::Result(Err(_))) if inner.is_none() => Some(Vec::new()),
            ("Err", Value::Result(Err(value))) => inner
                .as_ref()
                .and_then(|pattern| match_pattern(value, pattern)),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zelyra_lexer::lex;
    use zelyra_parser::parse;

    fn run(source: &str) -> Vec<String> {
        let program = parse(&lex(source).unwrap()).unwrap();
        check(&program).unwrap();
        execute(&program).unwrap()
    }

    #[test]
    fn fibonacci_acceptance_test() {
        let output = run("fn fibonacci(n: Int) -> Int { if n <= 1 { return n } return fibonacci(n - 1) + fibonacci(n - 2) } fn main() { print(fibonacci(10)) }");
        assert_eq!(output, ["55"]);
    }

    #[test]
    fn mutable_while_loop_works() {
        let output = run("fn main() { mutable i = 0 while i < 3 { print(i) i = i + 1 } }");
        assert_eq!(output, ["0", "1", "2"]);
    }

    #[test]
    fn supports_float_comparisons_and_inferred_returns() {
        let output = run(
            "fn twice(value: Float) { return value * 2.0 } fn main() { if twice(1.5) < 4.0 { print(twice(1.5)) } }",
        );
        assert_eq!(output, ["3"]);
    }

    #[test]
    fn rejects_immutable_assignment() {
        let program = parse(&lex("fn main() { value = 1 value = 2 }").unwrap()).unwrap();
        let errors = check(&program).unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("immutable")));
    }
}
