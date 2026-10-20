//! Incremental, compiler-checked AST REPL sessions.

use super::*;
use std::collections::BTreeMap;
use zelyra_lexer::lex;
use zelyra_parser::parse;

const REPL_FUNCTION: &str = "__zelyra_repl_entry";
const REPL_SESSION_TYPE: &str = "__ZelyraReplSessionContext";

#[derive(Clone, Debug, PartialEq)]
pub enum ReplError {
    Parse { message: String, span: Span },
    Type(TypeError),
    Runtime(RuntimeError),
    Session(String),
}

impl std::fmt::Display for ReplError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse { message, span } => write!(
                formatter,
                "error[E-PARSE-001]\n  --> {}:{}: {message}",
                span.line, span.column
            ),
            Self::Type(error) => write!(
                formatter,
                "error[{}]\n  --> {}:{}: {}",
                error.code, error.span.line, error.span.column, error.message
            ),
            Self::Runtime(error) => write!(
                formatter,
                "error[E-RUNTIME-001]\n  --> {}:{}: {}",
                error.span.line, error.span.column, error.message
            ),
            Self::Session(message) => write!(formatter, "error[E-REPL-001]: {message}"),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReplEvaluation {
    /// Value of the last expression, when the input ended with an expression.
    pub value: Option<Value>,
    /// Output produced by Zelyra print/output builtins during evaluation.
    pub output: Vec<String>,
}

#[derive(Clone, Debug)]
struct Binding {
    value: Value,
    ty: Type,
    mutable: bool,
}

/// One REPL session. Successful top-level bindings persist between calls;
/// each line is independently parsed, typechecked, and executed by the AST
/// reference interpreter. In sandbox mode the database connection is held
/// inside one MariaDB transaction and rolled back when the session is dropped.
pub struct ReplSession {
    program: Program,
    bindings: BTreeMap<String, Binding>,
    database_url: Option<String>,
    grants: Option<HashSet<String>>,
    policy: Option<RuntimePolicy>,
    transaction: Option<Arc<Mutex<zelyra_database::MariaDbInteractiveTransaction>>>,
    sandbox: bool,
    session_type_name: String,
    function_sequence: u64,
}

impl ReplSession {
    pub fn new(
        program: Program,
        database_url: Option<String>,
        grants: Option<HashSet<String>>,
        policy: Option<RuntimePolicy>,
        sandbox: bool,
    ) -> Result<Self, ReplError> {
        let session_type_name = unique_session_type_name(&program);
        let mut program = program;
        program.records.push(RecordDef {
            name: session_type_name.clone(),
            is_public: false,
            is_class: false,
            type_params: Vec::new(),
            fields: vec![RecordField {
                name: "tenant_id".into(),
                ty: Type::Option(Box::new(Type::Int)),
                span: Span::default(),
            }],
            span: Span::default(),
        });
        let transaction = if sandbox {
            match database_url.as_deref() {
                Some(url) if zelyra_database::is_postgres_url(url) => {
                    return Err(ReplError::Session(
                        "sandbox transactions currently require MariaDB/MySQL; PostgreSQL REPL transactions are not supported yet".into(),
                    ));
                }
                Some(url) => Some(Arc::new(Mutex::new(
                    zelyra_database::MariaDbInteractiveTransaction::begin(url).map_err(
                        |error| {
                            ReplError::Session(format!("cannot start sandbox transaction: {error}"))
                        },
                    )?,
                ))),
                None => None,
            }
        } else {
            None
        };
        Ok(Self {
            program,
            bindings: [(
                "session".into(),
                Binding {
                    value: Value::Object {
                        type_name: session_type_name.clone(),
                        type_args: Vec::new(),
                        fields: [("tenant_id".into(), Value::Option(None))]
                            .into_iter()
                            .collect(),
                    },
                    ty: Type::Named(session_type_name.clone()),
                    mutable: false,
                },
            )]
            .into_iter()
            .collect(),
            database_url,
            grants,
            policy,
            transaction,
            sandbox,
            session_type_name,
            function_sequence: 0,
        })
    }

    pub fn eval_line(&mut self, input: &str) -> Result<ReplEvaluation, ReplError> {
        let parsed = parse_input(input)?;
        let parsed_function = parsed.functions.first().ok_or_else(|| {
            ReplError::Session("the REPL input did not produce a function body".into())
        })?;
        let mut statements = parsed_function.body.statements.clone();
        if statements.is_empty() {
            return Ok(ReplEvaluation::default());
        }
        if statements.iter().any(contains_return) {
            return Err(ReplError::Session(
                "return cannot leave an interactive input; evaluate an expression instead".into(),
            ));
        }

        let last_expression = match statements.last() {
            Some(Stmt::Expr(expression)) => {
                let expression = expression.clone();
                statements.pop();
                Some(expression)
            }
            _ => None,
        };

        let existing = self.bindings.iter().collect::<Vec<_>>();
        let mut params = Vec::with_capacity(existing.len());
        let mut prefix = Vec::with_capacity(existing.len());
        let mut arguments = Vec::with_capacity(existing.len());
        let span = Span::default();
        for (index, (name, binding)) in existing.iter().enumerate() {
            let argument = format!("__zelyra_repl_arg_{index}");
            params.push(Param {
                name: argument.clone(),
                ty: binding.ty.clone(),
                span,
            });
            prefix.push(Stmt::Let {
                name: (*name).clone(),
                ty: Some(binding.ty.clone()),
                value: Expr {
                    kind: ExprKind::Variable(argument),
                    span,
                },
                mutable: binding.mutable,
                span,
            });
            arguments.push(binding.value.clone());
        }

        let existing_names = self.bindings.keys().cloned().collect::<HashSet<_>>();
        let new_bindings = collect_top_level_bindings(&statements, &existing_names);
        let mut output_names = self.bindings.keys().cloned().collect::<Vec<_>>();
        for (name, _) in new_bindings {
            if !output_names.contains(&name) {
                output_names.push(name.clone());
            }
        }
        output_names.sort();

        prefix.append(&mut statements);
        let mut return_values = output_names
            .iter()
            .map(|name| Expr {
                kind: ExprKind::Variable(name.clone()),
                span,
            })
            .collect::<Vec<_>>();
        let has_result = last_expression.is_some();
        if let Some(expression) = last_expression {
            return_values.push(expression);
        }
        if return_values.len() == 1 {
            prefix.push(Stmt::Return {
                value: return_values.pop(),
                span,
            });
        } else if return_values.len() > 1 {
            prefix.push(Stmt::Return {
                value: Some(Expr {
                    kind: ExprKind::Call {
                        name: "tuple".into(),
                        type_args: Vec::new(),
                        args: return_values,
                    },
                    span,
                }),
                span,
            });
        }

        self.function_sequence = self.function_sequence.saturating_add(1);
        let mut function_name = format!("{REPL_FUNCTION}_{}", self.function_sequence);
        while self
            .program
            .functions
            .iter()
            .any(|function| function.name == function_name)
        {
            self.function_sequence = self.function_sequence.saturating_add(1);
            function_name = format!("{REPL_FUNCTION}_{}", self.function_sequence);
        }
        let function = Function {
            name: function_name.clone(),
            is_public: false,
            type_params: Vec::new(),
            params,
            return_type: None,
            // The wrapper may invoke any capability explicitly granted to the
            // project. Runtime policy and grant checks remain authoritative.
            capabilities: self
                .grants
                .as_ref()
                .map(|grants| grants.iter().cloned().collect())
                .unwrap_or_default(),
            requires: Vec::new(),
            ensures: Vec::new(),
            body: Block {
                statements: prefix,
                span,
            },
            span,
        };
        let mut program = self.program.clone();
        program
            .functions
            .retain(|existing| existing.name != function_name);
        program.functions.push(function);

        if let Err(mut errors) = check(&program) {
            let error = errors.drain(..).next().expect("non-empty type diagnostics");
            return Err(ReplError::Type(remap_input_type_error(error)));
        }

        if let Some(transaction) = &self.transaction {
            transaction
                .lock()
                .map_err(|_| ReplError::Session("sandbox transaction is unavailable".into()))?
                .begin_input()
                .map_err(|error| {
                    ReplError::Session(format!("cannot begin input transaction: {error}"))
                })?;
        }

        let outcome = if let (Some(url), Some(transaction)) =
            (self.database_url.as_deref(), self.transaction.clone())
        {
            execute_function_captured_with_mariadb_transaction(
                &program,
                &function_name,
                arguments,
                url,
                self.grants.as_ref(),
                self.policy.as_ref(),
                transaction.clone(),
            )
        } else {
            execute_function_captured_with_capabilities_and_policies(
                &program,
                &function_name,
                arguments,
                self.database_url.as_deref(),
                self.grants.as_ref(),
                self.policy.as_ref(),
            )
        };

        if let Some(error) = outcome.error {
            if let Some(transaction) = &self.transaction {
                let _ = transaction
                    .lock()
                    .map_err(|_| ReplError::Session("sandbox transaction is unavailable".into()))?
                    .rollback_input();
            }
            return Err(ReplError::Runtime(remap_input_runtime_error(error)));
        }
        if let Some(transaction) = &self.transaction {
            transaction
                .lock()
                .map_err(|_| ReplError::Session("sandbox transaction is unavailable".into()))?
                .finish_input()
                .map_err(|error| {
                    ReplError::Session(format!("cannot finish input transaction: {error}"))
                })?;
        }

        let tuple = match outcome.value.unwrap_or(Value::Unit) {
            Value::Tuple(values) => values,
            Value::Unit if output_names.is_empty() && !has_result => Vec::new(),
            value => vec![value],
        };
        if tuple.len() < output_names.len() {
            return Err(ReplError::Session(
                "interpreter returned an incomplete REPL scope".into(),
            ));
        }
        for (name, value) in output_names.iter().zip(tuple.iter()) {
            if value.ty() == Type::Unknown
                || matches!(value.ty(), Type::Array(element) if *element == Type::Unknown)
            {
                return Err(ReplError::Session(format!(
                    "binding `{name}` has an unknown runtime type and cannot persist in this REPL"
                )));
            }
            let mutable = self
                .bindings
                .get(name)
                .map(|binding| binding.mutable)
                .or_else(|| new_bindings_mutability(&parsed_function.body.statements, name))
                .unwrap_or(false);
            self.bindings.insert(
                name.clone(),
                Binding {
                    value: value.clone(),
                    ty: value.ty(),
                    mutable,
                },
            );
        }
        let value = if has_result {
            tuple.get(output_names.len()).cloned()
        } else {
            None
        };
        Ok(ReplEvaluation {
            value,
            output: outcome.output,
        })
    }

    pub fn table_names(&self) -> Vec<String> {
        self.program
            .tables
            .iter()
            .map(|table| table.name.clone())
            .collect()
    }

    pub fn completion_candidates(&self) -> Vec<String> {
        let mut names = vec![
            "hash_sha256",
            "now",
            "print",
            "len",
            "to_string",
            "Some",
            "None",
            "Ok",
            "Err",
            "true",
            "false",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        names.extend(
            self.program
                .functions
                .iter()
                .map(|function| function.name.clone()),
        );
        names.extend(self.program.tables.iter().map(|table| table.name.clone()));
        names.extend(self.program.records.iter().flat_map(|structure| {
            std::iter::once(structure.name.clone())
                .chain(structure.fields.iter().map(|field| field.name.clone()))
        }));
        names.extend(self.bindings.keys().cloned());
        names.sort();
        names.dedup();
        names
    }

    pub fn schema_description(&self, table_name: &str) -> Option<String> {
        self.program
            .tables
            .iter()
            .find(|table| table.name.eq_ignore_ascii_case(table_name))
            .map(|table| {
                let mut lines = vec![format!("table {}", table.name)];
                if let Some(scope) = &table.tenant_scope {
                    lines.push(format!(
                        "  scoped by {}: {}",
                        scope.column, scope.tenant_type
                    ));
                }
                for column in &table.columns {
                    let mut flags = Vec::new();
                    if column.required {
                        flags.push("required");
                    }
                    if column.primary_key {
                        flags.push("primary");
                    }
                    if column.unique {
                        flags.push("unique");
                    }
                    lines.push(format!(
                        "  {}: {}{}",
                        column.name,
                        column.ty,
                        if flags.is_empty() {
                            String::new()
                        } else {
                            format!(" ({})", flags.join(", "))
                        }
                    ));
                }
                if !table.validations.is_empty() {
                    lines.push(format!(
                        "  invariants: {} declared",
                        table.validations.len()
                    ));
                }
                lines.join("\n")
            })
    }

    pub fn commit(&mut self) -> Result<(), ReplError> {
        let Some(transaction) = &self.transaction else {
            return Err(ReplError::Session(
                ":commit is available only in sandbox mode with a configured MariaDB database"
                    .into(),
            ));
        };
        transaction
            .lock()
            .map_err(|_| ReplError::Session("sandbox transaction is unavailable".into()))?
            .commit()
            .map_err(|error| ReplError::Session(format!("commit failed: {error}")))
    }

    pub fn rollback(&mut self) -> Result<(), ReplError> {
        let Some(transaction) = &self.transaction else {
            return Err(ReplError::Session(
                ":rollback is available only in sandbox mode with a configured MariaDB database"
                    .into(),
            ));
        };
        transaction
            .lock()
            .map_err(|_| ReplError::Session("sandbox transaction is unavailable".into()))?
            .rollback()
            .map_err(|error| ReplError::Session(format!("rollback failed: {error}")))
    }

    pub fn is_sandbox(&self) -> bool {
        self.sandbox
    }

    /// Set the session tenant id exposed to expressions and SQL parameters as `tenant_id`.
    pub fn set_tenant_id(&mut self, tenant_id: i64) {
        self.bindings.insert(
            "session".into(),
            Binding {
                value: Value::Object {
                    type_name: self.session_type_name.clone(),
                    type_args: Vec::new(),
                    fields: [(
                        "tenant_id".into(),
                        Value::Option(Some(Box::new(Value::Int(tenant_id)))),
                    )]
                    .into_iter()
                    .collect(),
                },
                ty: Type::Named(self.session_type_name.clone()),
                mutable: false,
            },
        );
        self.bindings.insert(
            "tenant_id".into(),
            Binding {
                value: Value::Int(tenant_id),
                ty: Type::Int,
                mutable: true,
            },
        );
    }
}

fn unique_session_type_name(program: &Program) -> String {
    let existing = program
        .records
        .iter()
        .map(|record| record.name.as_str())
        .collect::<HashSet<_>>();
    let mut candidate = REPL_SESSION_TYPE.to_owned();
    let mut suffix = 0_u64;
    while existing.contains(candidate.as_str()) {
        suffix = suffix.saturating_add(1);
        candidate = format!("{REPL_SESSION_TYPE}_{suffix}");
    }
    candidate
}

fn parse_input(input: &str) -> Result<Program, ReplError> {
    let source = format!("fn __zelyra_repl_parse() {{\n{input}\n}}\n");
    let tokens = lex(&source).map_err(|error| ReplError::Parse {
        message: error.message,
        span: input_span(error.span),
    })?;
    parse(&tokens).map_err(|error| ReplError::Parse {
        message: error.message,
        span: input_span(error.span),
    })
}

fn input_span(mut span: Span) -> Span {
    span.line = span.line.saturating_sub(1).max(1);
    span
}

fn remap_input_type_error(mut error: TypeError) -> TypeError {
    error.span = input_span(error.span);
    error
}

fn remap_input_runtime_error(mut error: RuntimeError) -> RuntimeError {
    error.span = input_span(error.span);
    error
}

fn collect_top_level_bindings(
    statements: &[Stmt],
    existing: &HashSet<String>,
) -> Vec<(String, bool)> {
    let mut bindings = BTreeMap::new();
    for statement in statements {
        match statement {
            Stmt::Let { name, mutable, .. } => {
                bindings.insert(name.clone(), *mutable);
            }
            Stmt::BindOrAssign { name, .. } if !existing.contains(name) => {
                bindings.insert(name.clone(), false);
            }
            Stmt::TupleBindOrAssign { names, .. } => {
                for name in names.iter().filter(|name| !existing.contains(*name)) {
                    bindings.insert(name.clone(), false);
                }
            }
            _ => {}
        }
    }

    bindings.into_iter().collect()
}

fn new_bindings_mutability(statements: &[Stmt], name: &str) -> Option<bool> {
    for statement in statements {
        match statement {
            Stmt::Let {
                name: binding,
                mutable,
                ..
            } if binding == name => return Some(*mutable),
            Stmt::BindOrAssign { name: binding, .. } if binding == name => return Some(false),
            Stmt::TupleBindOrAssign { names, .. }
                if names.iter().any(|binding| binding == name) =>
            {
                return Some(false);
            }
            _ => {}
        }
    }
    None
}

fn contains_return(statement: &Stmt) -> bool {
    match statement {
        Stmt::Return { .. } => true,
        Stmt::If {
            then_block,
            else_block,
            ..
        } => {
            then_block.statements.iter().any(contains_return)
                || else_block
                    .as_ref()
                    .is_some_and(|block| block.statements.iter().any(contains_return))
        }
        Stmt::While { body, .. } | Stmt::For { body, .. } | Stmt::Loop { body, .. } => {
            body.statements.iter().any(contains_return)
        }
        Stmt::Match { arms, .. } => arms
            .iter()
            .any(|arm| arm.body.statements.iter().any(contains_return)),
        Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
            body.statements.iter().any(contains_return)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> ReplSession {
        let program =
            parse(&lex("fn main() {}\n").expect("lex base program")).expect("parse base program");
        ReplSession::new(program, None, None, None, true).expect("start REPL")
    }

    #[test]
    fn diagnostics_put_source_location_and_message_on_a_second_line() {
        let error = ReplError::Parse {
            message: "unexpected token".into(),
            span: Span {
                line: 3,
                column: 5,
                start: 0,
                end: 0,
                source_id: 0,
            },
        };
        assert_eq!(
            error.to_string(),
            "error[E-PARSE-001]\n  --> 3:5: unexpected token"
        );
    }

    #[test]
    fn bindings_and_expressions_persist_across_inputs() {
        let mut session = session();
        session.eval_line("x = 10").expect("bind x");
        session.eval_line("y = 20").expect("bind y");
        let result = session.eval_line("x + y").expect("evaluate sum");
        assert_eq!(result.value, Some(Value::Int(30)));
    }

    #[test]
    fn tenant_is_available_through_typed_session_context_and_legacy_binding() {
        let mut session = session();
        session.set_tenant_id(42);
        assert_eq!(
            session.eval_line("session.tenant_id").unwrap().value,
            Some(Value::Option(Some(Box::new(Value::Int(42)))))
        );
        assert_eq!(
            session.eval_line("tenant_id").unwrap().value,
            Some(Value::Int(42))
        );
    }

    #[test]
    fn session_context_type_avoids_project_record_name_collisions() {
        let mut program = parse(&lex("fn main() {}\n").unwrap()).unwrap();
        program.records.push(RecordDef {
            name: REPL_SESSION_TYPE.into(),
            is_public: false,
            is_class: false,
            type_params: Vec::new(),
            fields: Vec::new(),
            span: Span::default(),
        });
        assert_eq!(
            unique_session_type_name(&program),
            format!("{REPL_SESSION_TYPE}_1")
        );
    }

    #[test]
    fn runtime_error_does_not_poison_the_next_input() {
        let mut session = session();
        assert!(session.eval_line("1 / 0").is_err());
        let result = session.eval_line("\"weiter\"").expect("evaluate next line");
        assert_eq!(result.value, Some(Value::String("weiter".into())));
    }

    #[test]
    fn syntax_and_type_errors_are_recoverable() {
        let mut session = session();
        assert!(session.eval_line("x = {").is_err());
        assert!(session.eval_line("x = true").is_ok());
        assert!(session.eval_line("x = 5").is_err());
        assert_eq!(
            session
                .eval_line("x")
                .expect("previous binding survives")
                .value,
            Some(Value::Bool(true))
        );
    }
}
