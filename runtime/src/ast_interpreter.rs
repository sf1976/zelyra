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

pub(super) struct AstInterpreter<'program> {
    pub(super) functions: HashMap<String, &'program Function>,
    pub(super) view_definitions: HashMap<String, ViewDef>,
    pub(super) output: Vec<String>,
    pub(super) next_class_object_id: Arc<AtomicU64>,
    pub(super) parallel_worker: bool,
    pub(super) step_budget: StepBudget,
    pub(super) call_depth_limit: usize,
    pub(super) call_depth: usize,
    pub(super) database_url: Option<String>,
    pub(super) database_transaction:
        Option<Arc<Mutex<zelyra_database::MariaDbInteractiveTransaction>>>,
    pub(super) storage_schema: Result<zelyra_database::Schema, Vec<zelyra_database::SchemaError>>,
    pub(super) storage_checked: bool,
    pub(super) scoped_tables: Vec<String>,
    pub(super) granted_capabilities: Option<HashSet<String>>,
    pub(super) filesystem_policy: Option<FileSystemPolicy>,
    pub(super) network_policy: Option<NetworkPolicy>,
    pub(super) process_policy: Option<ProcessPolicy>,
    pub(super) mail_policy: Option<MailPolicy>,
    pub(super) package_settings: Option<PackageSettings>,
    pub(super) host_services: Arc<dyn HostServices>,
    pub(super) record_definitions: HashMap<String, RecordDef>,
    pub(super) sql_record_definitions: HashMap<String, RecordDef>,
    pub(super) type_aliases: HashMap<String, Type>,
    pub(super) enum_definitions: HashMap<String, zelyra_ast::EnumDef>,
    pub(super) active_capabilities: Vec<HashSet<String>>,
    pub(super) active_type_substitutions: Vec<HashMap<String, Type>>,
}

impl<'program> AstInterpreter<'program> {
    pub(super) fn runtime_alias_type(&self, ty: &Type, span: Span) -> Result<Type, RuntimeError> {
        let mut aliases = HashMap::from([
            ("Id".into(), Type::Int),
            ("Money".into(), Type::Decimal),
            ("Email".into(), Type::String),
            ("Url".into(), Type::String),
            ("Uuid".into(), Type::String),
        ]);
        aliases.retain(|name, _| {
            !self.record_definitions.contains_key(name) && !self.enum_definitions.contains_key(name)
        });
        aliases.extend(self.type_aliases.clone());
        expand_type_aliases(ty, &aliases).map_err(|message| self.runtime_error(span, message))
    }

    pub(super) fn ensure_database_storage(&mut self, span: Span) -> Result<(), RuntimeError> {
        if !self.storage_checked {
            if let Some(url) = &self.database_url {
                validate_database_storage_with_host(
                    url,
                    &self.storage_schema,
                    self.host_services.as_ref(),
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
