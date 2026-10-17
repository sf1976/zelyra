//! Public execution entry points for the AST reference backend.
//!
//! This module owns interpreter construction, policy wiring, and bounded
//! runtime-thread dispatch while preserving the crate's existing public API.
use super::*;

pub fn execute(program: &Program) -> Result<Vec<String>, RuntimeError> {
    execute_internal(program, None, None, None)
}

pub fn execute_captured_with_capabilities_and_policies(
    program: &Program,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
) -> ExecutionOutcome {
    execute_internal_captured(program, None, grants.cloned(), policy.cloned())
}

/// Execute with an embedding-provided host-service implementation.
/// Capability checks, runtime policies, and step limits remain in force.
pub fn execute_with_host_services(
    program: &Program,
    host_services: Arc<dyn HostServices>,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal_with_host_services(
        program,
        None,
        grants.cloned(),
        policy.cloned(),
        host_services,
    )
}

/// Captured-output variant of [`execute_with_host_services`].
pub fn execute_captured_with_host_services(
    program: &Program,
    host_services: Arc<dyn HostServices>,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
) -> ExecutionOutcome {
    execute_internal_captured_with_host_services(
        program,
        None,
        grants.cloned(),
        policy.cloned(),
        host_services,
    )
}

/// Captured-output host-service execution with an explicit database runtime.
pub fn execute_captured_with_database_and_host_services(
    program: &Program,
    database_url: &str,
    host_services: Arc<dyn HostServices>,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
) -> ExecutionOutcome {
    execute_internal_captured_with_host_services(
        program,
        Some(database_url.to_owned()),
        grants.cloned(),
        policy.cloned(),
        host_services,
    )
}

/// Execute with host services and an explicit database runtime.
pub fn execute_with_database_and_host_services(
    program: &Program,
    database_url: &str,
    host_services: Arc<dyn HostServices>,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal_with_host_services(
        program,
        Some(database_url.to_owned()),
        grants.cloned(),
        policy.cloned(),
        host_services,
    )
}

pub fn execute_captured_with_database_and_capabilities_and_policies(
    program: &Program,
    database_url: &str,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
) -> ExecutionOutcome {
    execute_internal_captured(
        program,
        Some(database_url.to_owned()),
        grants.cloned(),
        policy.cloned(),
    )
}

pub fn execute_with_database(
    program: &Program,
    database_url: &str,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal(program, Some(database_url.to_owned()), None, None)
}

pub fn execute_with_capabilities(
    program: &Program,
    grants: Option<&HashSet<String>>,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal(program, None, grants.cloned(), None)
}

pub fn execute_with_database_and_capabilities(
    program: &Program,
    database_url: &str,
    grants: Option<&HashSet<String>>,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal(
        program,
        Some(database_url.to_owned()),
        grants.cloned(),
        None,
    )
}

pub fn execute_with_capabilities_and_filesystem_policy(
    program: &Program,
    grants: Option<&HashSet<String>>,
    filesystem_policy: Option<&FileSystemPolicy>,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal(
        program,
        None,
        grants.cloned(),
        Some(RuntimePolicy {
            filesystem: filesystem_policy.cloned(),
            network: None,
            process: None,
            mail: None,
            step_budget: None,
            max_stack_depth: None,
            package_settings: None,
        }),
    )
}

pub fn execute_with_capabilities_and_policies(
    program: &Program,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal(program, None, grants.cloned(), policy.cloned())
}

pub fn execute_with_database_and_capabilities_and_filesystem_policy(
    program: &Program,
    database_url: &str,
    grants: Option<&HashSet<String>>,
    filesystem_policy: Option<&FileSystemPolicy>,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal(
        program,
        Some(database_url.to_owned()),
        grants.cloned(),
        Some(RuntimePolicy {
            filesystem: filesystem_policy.cloned(),
            network: None,
            process: None,
            mail: None,
            step_budget: None,
            max_stack_depth: None,
            package_settings: None,
        }),
    )
}

pub fn execute_with_database_and_capabilities_and_policies(
    program: &Program,
    database_url: &str,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal(
        program,
        Some(database_url.to_owned()),
        grants.cloned(),
        policy.cloned(),
    )
}

pub fn execute_function(
    program: &Program,
    name: &str,
    args: Vec<Value>,
    database_url: Option<&str>,
) -> Result<Value, RuntimeError> {
    execute_function_with_capabilities_and_filesystem_policy(
        program,
        name,
        args,
        database_url,
        None,
        None,
    )
}

pub fn execute_function_with_capabilities_and_policies(
    program: &Program,
    name: &str,
    args: Vec<Value>,
    database_url: Option<&str>,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
) -> Result<Value, RuntimeError> {
    execute_function_with_host_services_and_capabilities_and_policies(
        program,
        name,
        args,
        database_url,
        grants,
        policy,
        Arc::new(SystemHostServices),
    )
}

/// Execute a function using caller-supplied host services.
pub fn execute_function_with_host_services_and_capabilities_and_policies(
    program: &Program,
    name: &str,
    args: Vec<Value>,
    database_url: Option<&str>,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
    host_services: Arc<dyn HostServices>,
) -> Result<Value, RuntimeError> {
    let depth = effective_call_depth(policy.and_then(|policy| policy.max_stack_depth));
    run_on_runtime_stack(runtime_stack_size(depth), || {
        let (result, _) = execute_function_on_runtime_stack(
            program,
            name,
            args,
            database_url,
            grants,
            policy,
            host_services,
        );
        result
    })
}

/// Execute a named function while preserving output produced before a runtime
/// failure. This is intended for command/task entry points that need the same
/// runtime policy and capability checks as `main`.
pub fn execute_function_captured_with_capabilities_and_policies(
    program: &Program,
    name: &str,
    args: Vec<Value>,
    database_url: Option<&str>,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
) -> FunctionExecutionOutcome {
    execute_function_captured_with_host_services_and_capabilities_and_policies(
        program,
        name,
        args,
        database_url,
        grants,
        policy,
        Arc::new(SystemHostServices),
    )
}

/// Captured-output variant of [`execute_function_with_host_services_and_capabilities_and_policies`].
pub fn execute_function_captured_with_host_services_and_capabilities_and_policies(
    program: &Program,
    name: &str,
    args: Vec<Value>,
    database_url: Option<&str>,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
    host_services: Arc<dyn HostServices>,
) -> FunctionExecutionOutcome {
    let depth = effective_call_depth(policy.and_then(|policy| policy.max_stack_depth));
    match run_on_runtime_stack(runtime_stack_size(depth), || {
        Ok(execute_function_on_runtime_stack(
            program,
            name,
            args,
            database_url,
            grants,
            policy,
            host_services,
        ))
    }) {
        Ok((Ok(value), output)) => FunctionExecutionOutcome {
            output,
            value: Some(value),
            error: None,
        },
        Ok((Err(error), output)) => FunctionExecutionOutcome {
            output,
            value: None,
            error: Some(error),
        },
        Err(error) => FunctionExecutionOutcome {
            output: Vec::new(),
            value: None,
            error: Some(error),
        },
    }
}

fn execute_function_on_runtime_stack(
    program: &Program,
    name: &str,
    args: Vec<Value>,
    database_url: Option<&str>,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
    host_services: Arc<dyn HostServices>,
) -> (Result<Value, RuntimeError>, Vec<String>) {
    let mut interpreter = Interpreter {
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
        output: Vec::new(),
        next_class_object_id: Arc::new(AtomicU64::new(1)),
        parallel_worker: false,
        step_budget: StepBudget::new(policy.as_ref().and_then(|policy| policy.step_budget)),
        call_depth_limit: effective_call_depth(
            policy.as_ref().and_then(|policy| policy.max_stack_depth),
        ),
        call_depth: 0,
        database_url: database_url.map(str::to_owned),
        storage_schema: zelyra_database::build_schema(program),
        storage_checked: false,
        scoped_tables: program
            .tables
            .iter()
            .filter(|table| table.tenant_scope.is_some())
            .map(|table| table.name.clone())
            .collect(),
        granted_capabilities: grants.cloned(),
        filesystem_policy: policy.and_then(|policy| policy.filesystem.clone()),
        network_policy: policy.and_then(|policy| policy.network.clone()),
        process_policy: policy.and_then(|policy| policy.process.clone()),
        mail_policy: policy.and_then(|policy| policy.mail.clone()),
        package_settings: policy.and_then(|policy| policy.package_settings.clone()),
        host_services,
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
        active_capabilities: Vec::new(),
        active_type_substitutions: Vec::new(),
    };
    let result = interpreter.call(name, args, Span::default());
    (result, interpreter.output)
}

pub fn execute_function_with_capabilities(
    program: &Program,
    name: &str,
    args: Vec<Value>,
    database_url: Option<&str>,
    grants: Option<&HashSet<String>>,
) -> Result<Value, RuntimeError> {
    execute_function_with_capabilities_and_filesystem_policy(
        program,
        name,
        args,
        database_url,
        grants,
        None,
    )
}

pub fn execute_function_with_capabilities_and_filesystem_policy(
    program: &Program,
    name: &str,
    args: Vec<Value>,
    database_url: Option<&str>,
    grants: Option<&HashSet<String>>,
    filesystem_policy: Option<&FileSystemPolicy>,
) -> Result<Value, RuntimeError> {
    execute_function_with_capabilities_and_policies(
        program,
        name,
        args,
        database_url,
        grants,
        Some(&RuntimePolicy {
            filesystem: filesystem_policy.cloned(),
            network: None,
            process: None,
            mail: None,
            step_budget: None,
            max_stack_depth: None,
            package_settings: None,
        }),
    )
}

fn execute_internal(
    program: &Program,
    database_url: Option<String>,
    granted_capabilities: Option<HashSet<String>>,
    policy: Option<RuntimePolicy>,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal_with_host_services(
        program,
        database_url,
        granted_capabilities,
        policy,
        Arc::new(SystemHostServices),
    )
}

fn execute_internal_with_host_services(
    program: &Program,
    database_url: Option<String>,
    granted_capabilities: Option<HashSet<String>>,
    policy: Option<RuntimePolicy>,
    host_services: Arc<dyn HostServices>,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal_captured_with_host_services(
        program,
        database_url,
        granted_capabilities,
        policy,
        host_services,
    )
    .into_result()
}

fn execute_internal_captured(
    program: &Program,
    database_url: Option<String>,
    granted_capabilities: Option<HashSet<String>>,
    policy: Option<RuntimePolicy>,
) -> ExecutionOutcome {
    execute_internal_captured_with_host_services(
        program,
        database_url,
        granted_capabilities,
        policy,
        Arc::new(SystemHostServices),
    )
}

fn execute_internal_captured_with_host_services(
    program: &Program,
    database_url: Option<String>,
    granted_capabilities: Option<HashSet<String>>,
    policy: Option<RuntimePolicy>,
    host_services: Arc<dyn HostServices>,
) -> ExecutionOutcome {
    let depth = effective_call_depth(policy.as_ref().and_then(|policy| policy.max_stack_depth));
    match run_on_runtime_stack(runtime_stack_size(depth), || {
        Ok(execute_internal_on_runtime_stack(
            program,
            database_url,
            granted_capabilities,
            policy,
            host_services,
        ))
    }) {
        Ok(outcome) => outcome,
        Err(error) => ExecutionOutcome {
            output: Vec::new(),
            error: Some(error),
        },
    }
}

fn execute_internal_on_runtime_stack(
    program: &Program,
    database_url: Option<String>,
    granted_capabilities: Option<HashSet<String>>,
    policy: Option<RuntimePolicy>,
    host_services: Arc<dyn HostServices>,
) -> ExecutionOutcome {
    let mut interpreter = Interpreter {
        functions: program
            .functions
            .iter()
            .map(|f| (f.name.clone(), f))
            .collect(),
        view_definitions: program
            .views
            .iter()
            .map(|view| (view.name.clone(), view.clone()))
            .collect(),
        output: Vec::new(),
        next_class_object_id: Arc::new(AtomicU64::new(1)),
        parallel_worker: false,
        step_budget: StepBudget::new(policy.as_ref().and_then(|policy| policy.step_budget)),
        call_depth_limit: effective_call_depth(
            policy.as_ref().and_then(|policy| policy.max_stack_depth),
        ),
        call_depth: 0,
        database_url,
        storage_schema: zelyra_database::build_schema(program),
        storage_checked: false,
        scoped_tables: program
            .tables
            .iter()
            .filter(|table| table.tenant_scope.is_some())
            .map(|table| table.name.clone())
            .collect(),
        granted_capabilities,
        filesystem_policy: policy.as_ref().and_then(|policy| policy.filesystem.clone()),
        network_policy: policy.as_ref().and_then(|policy| policy.network.clone()),
        process_policy: policy.as_ref().and_then(|policy| policy.process.clone()),
        mail_policy: policy.as_ref().and_then(|policy| policy.mail.clone()),
        package_settings: policy
            .as_ref()
            .and_then(|policy| policy.package_settings.clone()),
        host_services,
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
        active_capabilities: Vec::new(),
        active_type_substitutions: Vec::new(),
    };
    let error = interpreter.call("main", Vec::new(), Span::default()).err();
    ExecutionOutcome {
        output: interpreter.output,
        error,
    }
}

fn effective_call_depth(configured_depth: Option<usize>) -> usize {
    configured_depth
        .unwrap_or(MAX_FUNCTION_CALL_DEPTH)
        .min(MAX_CONFIGURED_FUNCTION_CALL_DEPTH)
}

pub(super) fn runtime_stack_size(depth: usize) -> usize {
    2 * 1024 * 1024 + depth.min(MAX_CONFIGURED_FUNCTION_CALL_DEPTH) * 192 * 1024
}

fn run_on_runtime_stack<T: Send>(
    stack_size: usize,
    action: impl FnOnce() -> Result<T, RuntimeError> + Send,
) -> Result<T, RuntimeError> {
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .name("zelyra-runtime".into())
            .stack_size(stack_size)
            .spawn_scoped(scope, action)
            .map_err(|_| RuntimeError {
                message: "runtime execution thread could not be started".into(),
                span: Span::default(),
            })?;
        handle.join().unwrap_or_else(|_| {
            Err(RuntimeError {
                message: "runtime execution thread panicked".into(),
                span: Span::default(),
            })
        })
    })
}
