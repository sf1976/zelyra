//! AST reference interpreter execution state and shared execution policy.
//!
//! This module is the semantic reference backend. MIR and bytecode remain
//! separate experimental implementations.
use super::*;

// Keep AST-only evaluation code behind this backend boundary. The source files
// remain separate so each concern can evolve independently without expanding
// the crate-root runtime API.
#[path = "builtin_evaluation.rs"]
mod builtin_evaluation;
#[path = "call_evaluation.rs"]
mod call_evaluation;
#[path = "environment.rs"]
mod environment;
#[path = "expression_evaluation.rs"]
mod expression_evaluation;
pub(crate) use environment::Environment;
#[path = "filesystem_io_builtins.rs"]
mod filesystem_io_builtins;
#[path = "filesystem_metadata_builtins.rs"]
mod filesystem_metadata_builtins;
#[path = "function_execution.rs"]
mod function_execution;
#[path = "host_builtins.rs"]
mod host_builtins;
#[path = "host_effects.rs"]
mod host_effects;
#[path = "json_codec.rs"]
mod json_codec;
#[path = "parallel.rs"]
mod parallel;
#[path = "path_builtins.rs"]
mod path_builtins;
#[path = "records.rs"]
mod records;
#[path = "sql_results.rs"]
pub(super) mod sql_results;
#[path = "statement_execution.rs"]
mod statement_execution;

enum Flow {
    Continue,
    LoopContinue,
    Return(Value),
    Break,
}

pub(super) struct AstInterpreter<'program, 'services> {
    pub(super) program_context: Arc<ProgramContext<'program>>,
    pub(super) output: Vec<String>,
    pub(super) next_class_object_id: Arc<AtomicU64>,
    pub(super) parallel_worker: bool,
    pub(super) step_budget: StepBudget,
    pub(super) call_depth_limit: usize,
    pub(super) call_depth: usize,
    pub(super) database_url: Option<String>,
    pub(super) database_transaction:
        Option<Arc<Mutex<zelyra_database::MariaDbInteractiveTransaction>>>,
    /// A request-owned transaction query bridge. Its borrow is scoped to one
    /// synchronous execution and is never cloned into parallel workers.
    pub(super) transactional_query: Option<&'services mut TransactionalQuery<'services>>,
    pub(super) storage_checked: bool,
    pub(super) execution_context: ExecutionContext,
    pub(super) active_capabilities: Vec<HashSet<String>>,
    pub(super) active_type_substitutions: Vec<HashMap<String, Type>>,
}

pub(super) type TransactionalQuery<'services> = dyn FnMut(&zelyra_database::Query) -> Result<zelyra_database::QueryResult, String>
    + Send
    + 'services;

/// Immutable symbols and schema metadata derived from one checked program.
/// Parallel workers share this context instead of cloning all declarations.
pub(super) struct ProgramContext<'program> {
    pub(super) functions: HashMap<String, &'program Function>,
    pub(super) view_definitions: HashMap<String, ViewDef>,
    pub(super) storage_schema: Result<zelyra_database::Schema, Vec<zelyra_database::SchemaError>>,
    pub(super) scoped_tables: Vec<String>,
    pub(super) record_definitions: HashMap<String, RecordDef>,
    pub(super) sql_record_definitions: HashMap<String, RecordDef>,
    pub(super) type_aliases: HashMap<String, Type>,
    pub(super) enum_definitions: HashMap<String, zelyra_ast::EnumDef>,
}

impl<'program> ProgramContext<'program> {
    fn from_program(program: &'program Program) -> Self {
        Self {
            functions: program
                .functions
                .iter()
                .map(|function| (function.name.clone(), function))
                .collect(),
            view_definitions: program
                .views
                .iter()
                .map(|view| (view.name.clone(), view.clone()))
                .collect(),
            storage_schema: zelyra_database::build_schema(program),
            scoped_tables: program
                .tables
                .iter()
                .filter(|table| table.tenant_scope.is_some())
                .map(|table| table.name.clone())
                .collect(),
            sql_record_definitions: sql_results::definitions(program),
            record_definitions: program
                .records
                .iter()
                .map(|record| (record.name.clone(), record.clone()))
                .collect(),
            type_aliases: program
                .types
                .iter()
                .map(|definition| (definition.name.clone(), definition.target.clone()))
                .collect(),
            enum_definitions: program
                .enums
                .iter()
                .map(|definition| (definition.name.clone(), definition.clone()))
                .collect(),
        }
    }

    pub(super) fn from_parts(
        records: &[RecordDef],
        aliases: &[TypeDef],
        enums: &[zelyra_ast::EnumDef],
    ) -> Self {
        Self {
            functions: HashMap::new(),
            view_definitions: HashMap::new(),
            storage_schema: Ok(zelyra_database::Schema {
                database: None,
                tables: Vec::new(),
            }),
            scoped_tables: Vec::new(),
            record_definitions: records
                .iter()
                .cloned()
                .map(|record| (record.name.clone(), record))
                .collect(),
            sql_record_definitions: HashMap::new(),
            type_aliases: aliases
                .iter()
                .map(|definition| (definition.name.clone(), definition.target.clone()))
                .collect(),
            enum_definitions: enums
                .iter()
                .cloned()
                .map(|definition| (definition.name.clone(), definition))
                .collect(),
        }
    }
}

/// Host access and the policies that constrain it for one AST execution.
/// Parallel workers clone this context as a unit so they cannot silently
/// diverge in capability grants or provider policy.
#[derive(Clone)]
pub(super) struct ExecutionContext {
    pub(super) granted_capabilities: Option<HashSet<String>>,
    pub(super) filesystem_policy: Option<FileSystemPolicy>,
    pub(super) network_policy: Option<NetworkPolicy>,
    pub(super) process_policy: Option<ProcessPolicy>,
    pub(super) mail_policy: Option<MailPolicy>,
    pub(super) package_settings: Option<PackageSettings>,
    pub(super) host_services: Arc<dyn HostServices>,
}

/// Per-execution host resources shared by every AST entry point.
pub(super) struct RuntimeServices<'services> {
    pub(super) host_services: Arc<dyn HostServices>,
    pub(super) database_transaction:
        Option<Arc<Mutex<zelyra_database::MariaDbInteractiveTransaction>>>,
    pub(super) transactional_query: Option<&'services mut TransactionalQuery<'services>>,
}

impl<'program, 'services> AstInterpreter<'program, 'services> {
    /// Construct the AST interpreter through one policy-wiring path for both
    /// program entry points and direct function execution.
    pub(super) fn new(
        program: &'program Program,
        database_url: Option<String>,
        granted_capabilities: Option<HashSet<String>>,
        policy: Option<RuntimePolicy>,
        services: RuntimeServices<'services>,
    ) -> Self {
        Self {
            program_context: Arc::new(ProgramContext::from_program(program)),
            output: Vec::new(),
            next_class_object_id: Arc::new(AtomicU64::new(1)),
            parallel_worker: false,
            step_budget: StepBudget::new(policy.as_ref().and_then(|policy| policy.step_budget)),
            call_depth_limit: effective_function_call_depth(
                policy.as_ref().and_then(|policy| policy.max_stack_depth),
            ),
            call_depth: 0,
            database_url,
            database_transaction: services.database_transaction,
            transactional_query: services.transactional_query,
            storage_checked: false,
            execution_context: ExecutionContext {
                granted_capabilities,
                filesystem_policy: policy.as_ref().and_then(|policy| policy.filesystem.clone()),
                network_policy: policy.as_ref().and_then(|policy| policy.network.clone()),
                process_policy: policy.as_ref().and_then(|policy| policy.process.clone()),
                mail_policy: policy.as_ref().and_then(|policy| policy.mail.clone()),
                package_settings: policy.and_then(|policy| policy.package_settings),
                host_services: services.host_services,
            },
            active_capabilities: Vec::new(),
            active_type_substitutions: Vec::new(),
        }
    }

    pub(super) fn runtime_alias_type(&self, ty: &Type, span: Span) -> Result<Type, RuntimeError> {
        let mut aliases = HashMap::from([
            ("Id".into(), Type::Int),
            ("Money".into(), Type::Decimal),
            ("Email".into(), Type::String),
            ("Url".into(), Type::String),
            ("Uuid".into(), Type::String),
            ("FileRef".into(), Type::String),
        ]);
        aliases.retain(|name, _| {
            !self.program_context.record_definitions.contains_key(name)
                && !self.program_context.enum_definitions.contains_key(name)
        });
        aliases.extend(self.program_context.type_aliases.clone());
        expand_type_aliases(ty, &aliases).map_err(|message| self.runtime_error(span, message))
    }

    pub(super) fn ensure_database_storage(&mut self, span: Span) -> Result<(), RuntimeError> {
        if !self.storage_checked {
            if let Some(url) = &self.database_url {
                validate_database_storage_with_host(
                    url,
                    &self.program_context.storage_schema,
                    self.execution_context.host_services.as_ref(),
                    span,
                )?;
            }
            self.storage_checked = true;
        }
        Ok(())
    }

    pub(super) fn consume_step(&mut self, span: Span) -> Result<(), RuntimeError> {
        match self.step_budget.consume() {
            Ok(_) => Ok(()),
            Err(StepBudgetError::LimitExceeded { limit, .. }) => Err(self.runtime_error(
                span,
                format!("execution step budget exceeded (limit {limit})"),
            )),
            Err(StepBudgetError::CounterOverflow) => {
                Err(self.runtime_error(span, "execution step counter overflow"))
            }
        }
    }
    pub(super) fn runtime_error(&self, span: Span, message: impl Into<String>) -> RuntimeError {
        RuntimeError {
            message: message.into(),
            span,
        }
    }
}

#[path = "execution_api.rs"]
pub(super) mod execution_api;
