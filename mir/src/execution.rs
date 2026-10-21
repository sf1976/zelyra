//! Experimental interpreter for checked MIR.
//!
//! This experimental interpreter executes verified control-flow graphs and a deliberately
//! bounded language subset. Unsupported expressions fail with their source span;
//! callers must keep the AST interpreter as the default
//! backend until the supported language surface is at parity. It is not
//! bytecode and is not a VM.

use crate::{
    verify, ExpressionId, IteratorId, LoopInvariantPhase, MatchId, MirExpressionKind,
    MirInstruction, MirIteratorBinding, MirIteratorSource, MirProgram, MirTerminator,
};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use zelyra_ast::{ArrayIndexAssignOp, BinaryOp, Span, Type, UnaryOp};
use zelyra_hir::{FunctionId, LocalId};
use zelyra_runtime::{
    deliver_outbox_message_with_host, effective_function_call_depth,
    execute_http_request_with_host, inspect_filesystem_path_with_host, normalize_map_entries,
    random_int_with_host_services, runtime_stack_size_for_depth, uuid_v4_with_host_services,
    FileSystemPolicy, FunctionValue, HostHttpResponse, HostServices, MailPolicy, NetworkPolicy,
    PackageSettings, ProcessPolicy, RuntimeError, RuntimePolicy, StepBudget, StepBudgetError,
    SystemHostServices, Value,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionLimits {
    pub max_steps: usize,
    pub max_stack_depth: usize,
}

pub(crate) fn bounded_execution_limits(limits: ExecutionLimits) -> ExecutionLimits {
    ExecutionLimits {
        max_stack_depth: effective_function_call_depth(Some(limits.max_stack_depth)),
        ..limits
    }
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_steps: 100_000,
            max_stack_depth: effective_function_call_depth(None),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionResult {
    pub value: Value,
    pub steps: usize,
}

/// Function output retained even when execution fails after producing it.
#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionOutcome {
    pub output: Vec<String>,
    pub value: Option<Value>,
    pub steps: usize,
    pub error: Option<RuntimeError>,
}

impl ExecutionOutcome {
    fn into_result(self) -> Result<ExecutionResult, RuntimeError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        Ok(ExecutionResult {
            value: self.value.unwrap_or(Value::Unit),
            steps: self.steps,
        })
    }
}

/// Database connection inputs for the experimental MIR interpreter.
pub struct DatabaseExecution<'a> {
    pub url: &'a str,
    pub host_services: Arc<dyn HostServices>,
}

struct ExecutionHost {
    database_url: Option<String>,
    host_services: Arc<dyn HostServices>,
}

/// Execute one function from verified MIR.
///
/// The experimental interpreter executes only the host effects explicitly wired through the
/// shared host-service contract. SQL without an explicit database context fails closed.
pub fn execute_function(
    program: &MirProgram,
    name: &str,
    arguments: Vec<Value>,
    limits: ExecutionLimits,
) -> Result<ExecutionResult, RuntimeError> {
    execute_function_with_host_services(
        program,
        name,
        arguments,
        limits,
        None,
        Arc::new(SystemHostServices),
    )
}

/// Execute verified MIR with injected host services and optional project grants.
///
/// The same per-function capability checks as the AST runtime are enforced
/// before clock, environment, or random providers are called.
pub fn execute_function_with_host_services(
    program: &MirProgram,
    name: &str,
    arguments: Vec<Value>,
    limits: ExecutionLimits,
    grants: Option<&HashSet<String>>,
    host_services: Arc<dyn HostServices>,
) -> Result<ExecutionResult, RuntimeError> {
    execute_function_with_host_services_and_policy(
        program,
        name,
        arguments,
        limits,
        grants,
        None,
        host_services,
    )
}

/// Execute verified MIR with injected host services, grants, and runtime policy.
///
/// Process execution remains disabled unless an explicit `ProcessPolicy` is
/// supplied. The configured command allowlist and output/time bounds are
/// checked before the injected host is called. SQL uses the separate database
/// execution entry point.
pub fn execute_function_with_host_services_and_policy(
    program: &MirProgram,
    name: &str,
    arguments: Vec<Value>,
    limits: ExecutionLimits,
    grants: Option<&HashSet<String>>,
    runtime_policy: Option<&RuntimePolicy>,
    host_services: Arc<dyn HostServices>,
) -> Result<ExecutionResult, RuntimeError> {
    execute_function_captured_with_host_services_and_policy(
        program,
        name,
        arguments,
        limits,
        grants,
        runtime_policy,
        host_services,
    )
    .into_result()
}

/// Captured-output MIR execution with injected host services and runtime policy.
pub fn execute_function_captured_with_host_services_and_policy(
    program: &MirProgram,
    name: &str,
    arguments: Vec<Value>,
    limits: ExecutionLimits,
    grants: Option<&HashSet<String>>,
    runtime_policy: Option<&RuntimePolicy>,
    host_services: Arc<dyn HostServices>,
) -> ExecutionOutcome {
    execute_function_captured_with_host_services_and_database_url(
        program,
        name,
        arguments,
        limits,
        grants,
        runtime_policy,
        ExecutionHost {
            database_url: None,
            host_services,
        },
    )
}

/// Execute verified MIR with an explicitly configured database runtime.
///
/// SQL still requires both a function capability and an optional project grant;
/// storage validation and query execution use the injected HostServices.
pub fn execute_function_with_database_and_host_services_and_policy(
    program: &MirProgram,
    name: &str,
    arguments: Vec<Value>,
    limits: ExecutionLimits,
    grants: Option<&HashSet<String>>,
    runtime_policy: Option<&RuntimePolicy>,
    database: DatabaseExecution<'_>,
) -> Result<ExecutionResult, RuntimeError> {
    execute_function_captured_with_database_and_host_services_and_policy(
        program,
        name,
        arguments,
        limits,
        grants,
        runtime_policy,
        database,
    )
    .into_result()
}

/// Captured-output MIR execution with an explicitly configured database.
pub fn execute_function_captured_with_database_and_host_services_and_policy(
    program: &MirProgram,
    name: &str,
    arguments: Vec<Value>,
    limits: ExecutionLimits,
    grants: Option<&HashSet<String>>,
    runtime_policy: Option<&RuntimePolicy>,
    database: DatabaseExecution<'_>,
) -> ExecutionOutcome {
    execute_function_captured_with_host_services_and_database_url(
        program,
        name,
        arguments,
        limits,
        grants,
        runtime_policy,
        ExecutionHost {
            database_url: Some(database.url.to_owned()),
            host_services: database.host_services,
        },
    )
}

fn execute_function_captured_with_host_services_and_database_url(
    program: &MirProgram,
    name: &str,
    arguments: Vec<Value>,
    limits: ExecutionLimits,
    grants: Option<&HashSet<String>>,
    runtime_policy: Option<&RuntimePolicy>,
    execution_host: ExecutionHost,
) -> ExecutionOutcome {
    let limits = bounded_execution_limits(limits);
    if let Err(errors) = verify(program) {
        let error = errors.into_iter().next();
        return failed_outcome(error.map_or_else(
            || RuntimeError {
                message: "MIR verification failed".into(),
                span: Span::default(),
            },
            |error| RuntimeError {
                message: format!("{}: {}", error.code, error.message),
                span: error.span,
            },
        ));
    }
    let function = program
        .functions
        .iter()
        .find(|function| function.name == name)
        .ok_or_else(|| RuntimeError {
            message: format!("MIR function `{name}` was not found"),
            span: Span::default(),
        });
    let function = match function {
        Ok(function) => function,
        Err(error) => return failed_outcome(error),
    };
    let stack_size = runtime_stack_size_for_depth(limits.max_stack_depth);
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .name("zelyra-mir-interpreter".into())
            .stack_size(stack_size)
            .spawn_scoped(scope, move || {
                let mut machine = Machine {
                    program,
                    functions: program
                        .functions
                        .iter()
                        .map(|function| (function.id, function))
                        .collect(),
                    match_values: HashMap::new(),
                    iterators: HashMap::new(),
                    closures: Arc::new(Mutex::new(HashMap::new())),
                    next_closure_id: Arc::new(AtomicUsize::new(0)),
                    step_budget: StepBudget::new(Some(limits.max_steps)),
                    limits,
                    active_capabilities: Vec::new(),
                    database_url: execution_host.database_url,
                    storage_schema: program.database_schema.clone(),
                    storage_checked: false,
                    grants: grants.cloned(),
                    process_policy: runtime_policy.and_then(|policy| policy.process.clone()),
                    filesystem_policy: runtime_policy.and_then(|policy| policy.filesystem.clone()),
                    network_policy: runtime_policy.and_then(|policy| policy.network.clone()),
                    mail_policy: runtime_policy.and_then(|policy| policy.mail.clone()),
                    package_settings: runtime_policy
                        .and_then(|policy| policy.package_settings.clone()),
                    host_services: execution_host.host_services,
                    output: Arc::new(Mutex::new(Vec::new())),
                };
                match machine.call(function.id, arguments, 1, function.span) {
                    Ok(value) => ExecutionOutcome {
                        output: machine.captured_output(),
                        value: Some(value),
                        steps: machine.step_budget.consumed(),
                        error: None,
                    },
                    Err(error) => ExecutionOutcome {
                        output: machine.captured_output(),
                        value: None,
                        steps: machine.step_budget.consumed(),
                        error: Some(error),
                    },
                }
            })
            .map_err(|_| RuntimeError {
                message: "MIR interpreter thread could not be started".into(),
                span: Span::default(),
            });
        match handle {
            Ok(handle) => handle.join().unwrap_or_else(|_| {
                failed_outcome(RuntimeError {
                    message: "MIR interpreter thread panicked".into(),
                    span: Span::default(),
                })
            }),
            Err(error) => failed_outcome(error),
        }
    })
}

fn failed_outcome(error: RuntimeError) -> ExecutionOutcome {
    ExecutionOutcome {
        output: Vec::new(),
        value: None,
        steps: 0,
        error: Some(error),
    }
}

struct Machine<'program> {
    program: &'program MirProgram,
    functions: HashMap<FunctionId, &'program crate::MirFunction>,
    match_values: HashMap<MatchId, Value>,
    iterators: HashMap<IteratorId, IteratorState>,
    closures: Arc<Mutex<HashMap<String, MirCallback>>>,
    next_closure_id: Arc<AtomicUsize>,
    step_budget: StepBudget,
    limits: ExecutionLimits,
    active_capabilities: Vec<HashSet<String>>,
    database_url: Option<String>,
    storage_schema: Result<zelyra_database::Schema, Vec<zelyra_database::SchemaError>>,
    storage_checked: bool,
    grants: Option<HashSet<String>>,
    process_policy: Option<ProcessPolicy>,
    filesystem_policy: Option<FileSystemPolicy>,
    network_policy: Option<NetworkPolicy>,
    mail_policy: Option<MailPolicy>,
    package_settings: Option<PackageSettings>,
    host_services: Arc<dyn HostServices>,
    output: Arc<Mutex<Vec<String>>>,
}

enum IteratorState {
    Values(std::vec::IntoIter<Value>),
    Range {
        current: Option<i64>,
        end: i64,
        step: i64,
    },
}

#[derive(Clone)]
enum MirCallback {
    Lambda {
        params: Vec<(LocalId, String, Type)>,
        body: ExpressionId,
        captures: HashMap<LocalId, Value>,
        return_type: Type,
        capabilities: Vec<String>,
    },
    Function(FunctionId),
}

impl IteratorState {
    fn next(&mut self) -> Option<Value> {
        match self {
            Self::Values(values) => values.next(),
            Self::Range { current, end, step } => {
                let value = current.take()?;
                let in_range = |value: i64| {
                    if *step > 0 {
                        value < *end
                    } else {
                        value > *end
                    }
                };
                if !in_range(value) {
                    return None;
                }
                *current = value.checked_add(*step).filter(|next| in_range(*next));
                Some(Value::Int(value))
            }
        }
    }
}

fn capture_locals(
    program: &MirProgram,
    root: ExpressionId,
    visible: &HashMap<LocalId, Value>,
) -> HashMap<LocalId, Value> {
    fn visit(
        program: &MirProgram,
        id: ExpressionId,
        visited: &mut HashSet<ExpressionId>,
        referenced: &mut HashSet<LocalId>,
    ) {
        if !visited.insert(id) {
            return;
        }
        let Some(expression) = program.expressions.get(id.0) else {
            return;
        };
        let mut children = Vec::new();
        match &expression.kind {
            MirExpressionKind::Local(local) => {
                referenced.insert(*local);
            }
            MirExpressionKind::Array(items) => children.extend(items.iter().copied()),
            MirExpressionKind::Map(entries) => {
                children.extend(entries.iter().flat_map(|(key, value)| [*key, *value]));
            }
            MirExpressionKind::Record { fields, .. } => {
                children.extend(fields.iter().map(|(_, value)| *value));
            }
            MirExpressionKind::Lambda { body, .. } => children.push(*body),
            MirExpressionKind::Index { target, index } => children.extend([*target, *index]),
            MirExpressionKind::Field { target, .. } => children.push(*target),
            MirExpressionKind::Call {
                callee_local, args, ..
            } => {
                referenced.extend(callee_local);
                children.extend(args.iter().copied());
            }
            MirExpressionKind::Unary { expr, .. } | MirExpressionKind::Await(expr) => {
                children.push(*expr);
            }
            MirExpressionKind::Binary { left, right, .. } => children.extend([*left, *right]),
            MirExpressionKind::BinaryChain { first, operations } => {
                children.push(*first);
                children.extend(operations.iter().map(|(_, value, _)| *value));
            }
            MirExpressionKind::Match { value, arms } => {
                children.push(*value);
                for arm in arms {
                    children.extend(arm.guard);
                    children.push(arm.value);
                }
            }
            MirExpressionKind::Sql { parameters, .. } => {
                referenced.extend(parameters.iter().map(|(_, local)| *local));
            }
            MirExpressionKind::Uninitialized
            | MirExpressionKind::Int(_)
            | MirExpressionKind::UInt(_)
            | MirExpressionKind::Float(_)
            | MirExpressionKind::Bool(_)
            | MirExpressionKind::String(_)
            | MirExpressionKind::Char(_)
            | MirExpressionKind::EnumVariant { .. }
            | MirExpressionKind::FunctionValue { .. } => {}
        }
        for child in children {
            visit(program, child, visited, referenced);
        }
    }

    let mut referenced = HashSet::new();
    visit(program, root, &mut HashSet::new(), &mut referenced);
    referenced
        .into_iter()
        .filter_map(|local| visible.get(&local).cloned().map(|value| (local, value)))
        .collect()
}

fn bind_iterator_value(
    locals: &mut HashMap<LocalId, Value>,
    bindings: &[MirIteratorBinding],
    value: Value,
    span: Span,
) -> Result<(), RuntimeError> {
    let values = if bindings.len() == 1 {
        vec![value]
    } else {
        let Value::Tuple(values) = value else {
            return Err(runtime_error(
                span,
                "MIR tuple iterator pattern received a non-tuple value",
            ));
        };
        values
    };
    if values.len() != bindings.len() {
        return Err(runtime_error(
            span,
            format!(
                "MIR iterator pattern expects {} values, received {}",
                bindings.len(),
                values.len()
            ),
        ));
    }
    locals.extend(
        bindings
            .iter()
            .zip(values)
            .map(|(binding, value)| (binding.local, value)),
    );
    Ok(())
}

fn instruction_span(program: &MirProgram, instruction: &MirInstruction) -> Span {
    match instruction {
        MirInstruction::IteratorInit { span, .. }
        | MirInstruction::SourceStep { span }
        | MirInstruction::MatchInit { span, .. }
        | MirInstruction::Let { span, .. }
        | MirInstruction::Assign { span, .. }
        | MirInstruction::ArrayIndexAssign { span, .. }
        | MirInstruction::TupleBind { span, .. }
        | MirInstruction::Transaction { span, .. }
        | MirInstruction::Parallel { span, .. }
        | MirInstruction::LoopInvariantCheck { span, .. } => *span,
        MirInstruction::Evaluate(expression) => program
            .expressions
            .get(expression.0)
            .map_or(Span::default(), |expression| expression.span),
    }
}

impl Machine<'_> {
    fn tick(&mut self, span: Span) -> Result<(), RuntimeError> {
        match self.step_budget.consume() {
            Ok(_) => Ok(()),
            Err(StepBudgetError::LimitExceeded { limit, .. }) => Err(runtime_error(
                span,
                format!("execution step budget exceeded (limit {limit})"),
            )),
            Err(StepBudgetError::CounterOverflow) => {
                Err(runtime_error(span, "execution step counter overflow"))
            }
        }
    }

    fn execute_parallel(
        &mut self,
        bindings: &[crate::MirParallelBinding],
        locals: &mut HashMap<LocalId, Value>,
        depth: usize,
        span: Span,
    ) -> Result<(), RuntimeError> {
        for binding in bindings {
            if !binding.is_new && !locals.contains_key(&binding.local) {
                return Err(runtime_error(
                    binding.span,
                    "MIR parallel assignment targets an uninitialized local",
                ));
            }
        }

        let stack_size = runtime_stack_size_for_depth(self.limits.max_stack_depth);
        let results = std::thread::scope(|scope| {
            let handles = bindings
                .iter()
                .map(|binding| {
                    let mut child = self.parallel_child();
                    let mut child_locals = locals.clone();
                    let body =
                        self.program
                            .expressions
                            .get(binding.value.0)
                            .and_then(|expression| match &expression.kind {
                                MirExpressionKind::Await(body) => Some(*body),
                                _ => None,
                            });
                    std::thread::Builder::new()
                        .name("zelyra-mir-parallel".into())
                        .stack_size(stack_size)
                        .spawn_scoped(scope, move || {
                            let result = body.map_or_else(
                                || {
                                    Err(runtime_error(
                                        binding.span,
                                        "MIR parallel binding lost its checked await expression",
                                    ))
                                },
                                |body| {
                                    child.expression(body, &mut child_locals, depth, binding.span)
                                },
                            );
                            result
                                .map(|value| (binding.local, value))
                                .map_err(|error| (binding.span, error))
                        })
                })
                .collect::<Vec<_>>();
            let mut results = Vec::with_capacity(handles.len());
            let mut first_error = None;
            for handle in handles {
                match handle {
                    Ok(handle) => match handle.join() {
                        Ok(Ok(result)) => results.push(result),
                        Ok(Err((branch_span, error))) => {
                            if first_error.is_none() {
                                first_error = Some(
                                    if error.message.contains("execution step budget exceeded") {
                                        runtime_error(span, error.message)
                                    } else {
                                        RuntimeError {
                                            span: if error.span == Span::default() {
                                                branch_span
                                            } else {
                                                error.span
                                            },
                                            ..error
                                        }
                                    },
                                );
                            }
                        }
                        Err(_) => {
                            if first_error.is_none() {
                                first_error =
                                    Some(runtime_error(span, "MIR parallel task panicked"));
                            }
                        }
                    },
                    Err(_) => {
                        if first_error.is_none() {
                            first_error = Some(runtime_error(
                                span,
                                "MIR parallel task thread could not be started",
                            ));
                        }
                    }
                }
            }
            first_error.map_or(Ok(results), Err)
        })?;

        for (local, value) in results {
            locals.insert(local, value);
        }
        Ok(())
    }

    fn parallel_child(&self) -> Machine<'_> {
        Machine {
            program: self.program,
            functions: self.functions.clone(),
            match_values: HashMap::new(),
            iterators: HashMap::new(),
            closures: Arc::clone(&self.closures),
            next_closure_id: Arc::clone(&self.next_closure_id),
            step_budget: self.step_budget.clone(),
            limits: self.limits,
            active_capabilities: self.active_capabilities.clone(),
            database_url: self.database_url.clone(),
            storage_schema: self.storage_schema.clone(),
            storage_checked: self.storage_checked,
            grants: self.grants.clone(),
            process_policy: self.process_policy.clone(),
            filesystem_policy: self.filesystem_policy.clone(),
            network_policy: self.network_policy.clone(),
            mail_policy: self.mail_policy.clone(),
            package_settings: self.package_settings.clone(),
            host_services: Arc::clone(&self.host_services),
            output: Arc::clone(&self.output),
        }
    }

    fn captured_output(&self) -> Vec<String> {
        self.output
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn check_loop_invariants(
        &mut self,
        expressions: &[ExpressionId],
        locals: &mut HashMap<LocalId, Value>,
        depth: usize,
        loop_span: Span,
        phase: LoopInvariantPhase,
    ) -> Result<(), RuntimeError> {
        let phase_name = match phase {
            LoopInvariantPhase::Before => "before",
            LoopInvariantPhase::After => "after",
        };
        for expression in expressions {
            let expression_span = self
                .program
                .expressions
                .get(expression.0)
                .map_or(loop_span, |item| item.span);
            let value = self.expression(*expression, locals, depth, loop_span)?;
            if !self.expect_bool(value, expression_span)? {
                return Err(runtime_error(
                    loop_span,
                    format!("loop invariant failed {phase_name} loop iteration"),
                ));
            }
        }
        Ok(())
    }

    fn call(
        &mut self,
        function_id: FunctionId,
        arguments: Vec<Value>,
        depth: usize,
        call_span: Span,
    ) -> Result<Value, RuntimeError> {
        let function = self
            .functions
            .get(&function_id)
            .copied()
            .ok_or_else(|| runtime_error(Span::default(), "MIR function id was not found"))?;
        if let Some(caller_capabilities) = self.active_capabilities.last() {
            if let Some(capability) = function
                .declared_capabilities
                .iter()
                .find(|capability| !has_capability(caller_capabilities, capability))
            {
                return Err(runtime_error(
                    function.span,
                    format!(
                        "runtime capability denied: calling `{}` requires `{capability}` in the caller",
                        function.name
                    ),
                ));
            }
        }
        self.active_capabilities
            .push(function.declared_capabilities.iter().cloned().collect());
        let result = self.call_frame(function_id, arguments, depth, call_span);
        self.active_capabilities.pop();
        result
    }

    fn call_frame(
        &mut self,
        function_id: FunctionId,
        arguments: Vec<Value>,
        depth: usize,
        call_span: Span,
    ) -> Result<Value, RuntimeError> {
        let function = self
            .functions
            .get(&function_id)
            .copied()
            .ok_or_else(|| RuntimeError {
                message: format!("MIR function id {} was not found", function_id.0),
                span: Span::default(),
            })?;
        if depth > self.limits.max_stack_depth {
            return Err(RuntimeError {
                message: "MIR maximum call stack depth exceeded".into(),
                span: call_span,
            });
        }
        if arguments.len() != function.parameters.len() {
            return Err(RuntimeError {
                message: format!(
                    "MIR function `{}` expects {} arguments but received {}",
                    function.name,
                    function.parameters.len(),
                    arguments.len()
                ),
                span: function.span,
            });
        }

        let mut locals = HashMap::<LocalId, Value>::new();
        for ((local, _, expected, span), value) in function.parameters.iter().zip(arguments) {
            if !runtime_type_matches(expected, &value.ty()) {
                return Err(RuntimeError {
                    message: format!(
                        "MIR parameter expects `{expected}` but received `{}`",
                        value.ty()
                    ),
                    span: *span,
                });
            }
            locals.insert(*local, value);
        }

        for contract in &function.requires {
            let value = self.expression(*contract, &mut locals, depth, function.span)?;
            if !self.expect_bool(value, self.program.expressions[contract.0].span)? {
                return Err(runtime_error(
                    self.program.expressions[contract.0].span,
                    format!("precondition failed for function `{}`", function.name),
                ));
            }
        }

        let result = (|| -> Result<Value, RuntimeError> {
            let mut block = function.entry;
            loop {
                let basic_block = function.blocks.get(block.0).ok_or_else(|| RuntimeError {
                    message: format!("MIR control flow reached missing block {}", block.0),
                    span: function.span,
                })?;
                if let Some(loop_span) = basic_block.loop_span {
                    self.tick(loop_span)?;
                }
                if !basic_block.loop_invariants.is_empty() {
                    self.check_loop_invariants(
                        &basic_block.loop_invariants,
                        &mut locals,
                        depth,
                        basic_block.loop_span.unwrap_or(basic_block.span),
                        LoopInvariantPhase::Before,
                    )?;
                }
                for instruction in &basic_block.instructions {
                    if !matches!(
                        instruction,
                        MirInstruction::SourceStep { .. }
                            | MirInstruction::LoopInvariantCheck { .. }
                    ) {
                        self.tick(instruction_span(self.program, instruction))?;
                    }
                    match instruction {
                        MirInstruction::SourceStep { span } => self.tick(*span)?,
                        MirInstruction::Let {
                            local, value, span, ..
                        } => {
                            let value = self.expression(*value, &mut locals, depth, *span)?;
                            locals.insert(*local, value);
                        }
                        MirInstruction::Assign {
                            local, value, span, ..
                        } => {
                            if !locals.contains_key(local) {
                                return Err(RuntimeError {
                                    message: "MIR assignment targets an uninitialized local".into(),
                                    span: *span,
                                });
                            }
                            let value = self.expression(*value, &mut locals, depth, *span)?;
                            locals.insert(*local, value);
                        }
                        MirInstruction::ArrayIndexAssign {
                            local,
                            index,
                            value,
                            operation,
                            span,
                            ..
                        } => {
                            let items_len = match locals.get(local) {
                                Some(Value::Array(items)) => items.len(),
                                Some(other) => {
                                    return Err(runtime_error(
                                        *span,
                                        format!(
                                            "indexed assignment requires Array, got `{}`",
                                            other.ty()
                                        ),
                                    ));
                                }
                                None => {
                                    return Err(runtime_error(
                                        *span,
                                        "MIR indexed assignment targets an uninitialized local",
                                    ));
                                }
                            };
                            let index = self.expression(*index, &mut locals, depth, *span)?;
                            let index = array_assignment_index(index, items_len, *span)?;
                            let current = if *operation == ArrayIndexAssignOp::Add {
                                let Some(Value::Array(items)) = locals.get(local) else {
                                    return Err(runtime_error(
                                        *span,
                                        "MIR indexed assignment target changed during evaluation",
                                    ));
                                };
                                Some(items[index].clone())
                            } else {
                                None
                            };
                            let value = self.expression(*value, &mut locals, depth, *span)?;
                            let value = match current {
                                None => value,
                                Some(current) => {
                                    evaluate_binary(current, BinaryOp::Add, value, *span)?
                                }
                            };
                            let Some(Value::Array(items)) = locals.get_mut(local) else {
                                return Err(runtime_error(
                                    *span,
                                    "MIR indexed assignment target changed during evaluation",
                                ));
                            };
                            let element = items.get_mut(index).ok_or_else(|| {
                                runtime_error(*span, "MIR mutable array index is out of bounds")
                            })?;
                            *element = value;
                        }
                        MirInstruction::MatchInit {
                            match_id,
                            value,
                            span,
                        } => {
                            let value = self.expression(*value, &mut locals, depth, *span)?;
                            self.match_values.insert(*match_id, value);
                        }
                        MirInstruction::IteratorInit {
                            iterator,
                            source,
                            span,
                        } => {
                            let state = match source {
                                MirIteratorSource::Collection(expression) => {
                                    let value =
                                        self.expression(*expression, &mut locals, depth, *span)?;
                                    match value {
                                        Value::Array(values) | Value::Set(values) => {
                                            IteratorState::Values(values.into_iter())
                                        }
                                        other => {
                                            return Err(runtime_error(
                                            *span,
                                            format!(
                                                "MIR collection iterator requires Array or Set, got `{}`",
                                                other.ty()
                                            ),
                                        ));
                                        }
                                    }
                                }
                                MirIteratorSource::IntegerRange { start, end, step } => {
                                    let start_value =
                                        self.expression(*start, &mut locals, depth, *span)?;
                                    let end_value =
                                        self.expression(*end, &mut locals, depth, *span)?;
                                    let (Value::Int(start), Value::Int(end)) =
                                        (start_value, end_value)
                                    else {
                                        return Err(runtime_error(
                                            *span,
                                            "MIR integer range requires Int start and end values",
                                        ));
                                    };
                                    let step = match step {
                                        Some(expression) => match self.expression(
                                            *expression,
                                            &mut locals,
                                            depth,
                                            *span,
                                        )? {
                                            Value::Int(step) => step,
                                            other => {
                                                return Err(runtime_error(
                                                    *span,
                                                    format!(
                                                    "MIR integer range step must be Int, got `{}`",
                                                    other.ty()
                                                ),
                                                ));
                                            }
                                        },
                                        None => 1,
                                    };
                                    if step == 0 {
                                        return Err(runtime_error(
                                            *span,
                                            "MIR integer range step must not be zero",
                                        ));
                                    }
                                    IteratorState::Range {
                                        current: Some(start),
                                        end,
                                        step,
                                    }
                                }
                            };
                            self.iterators.insert(*iterator, state);
                        }
                        MirInstruction::TupleBind {
                            bindings,
                            value,
                            span,
                        } => {
                            let value = self.expression(*value, &mut locals, depth, *span)?;
                            let Value::Tuple(values) = value else {
                                return Err(runtime_error(
                                    *span,
                                    "MIR tuple binding requires a Tuple value",
                                ));
                            };
                            if bindings.len() != values.len() {
                                return Err(runtime_error(
                                    *span,
                                    format!(
                                        "MIR tuple binding expects {} values, received {}",
                                        bindings.len(),
                                        values.len()
                                    ),
                                ));
                            }
                            if bindings.iter().any(|binding| {
                                !binding.is_new && !locals.contains_key(&binding.local)
                            }) {
                                return Err(runtime_error(
                                    *span,
                                    "MIR tuple assignment targets an uninitialized local",
                                ));
                            }
                            for (binding, value) in bindings.iter().zip(values) {
                                locals.insert(binding.local, value);
                            }
                        }
                        MirInstruction::Transaction { operations, span } => {
                            self.execute_transaction(operations, &mut locals, depth, *span)?;
                        }
                        MirInstruction::Parallel { bindings, span } => {
                            self.execute_parallel(bindings, &mut locals, depth, *span)?;
                        }
                        MirInstruction::LoopInvariantCheck {
                            expressions,
                            phase,
                            span,
                        } => self.check_loop_invariants(
                            expressions,
                            &mut locals,
                            depth,
                            *span,
                            *phase,
                        )?,
                        MirInstruction::Evaluate(expression) => {
                            let _ =
                                self.expression(*expression, &mut locals, depth, basic_block.span)?;
                        }
                    }
                }

                match basic_block
                    .terminator
                    .as_ref()
                    .expect("verified terminator")
                {
                    MirTerminator::Goto(target) => block = *target,
                    MirTerminator::Branch {
                        condition,
                        then_block,
                        else_block,
                    } => {
                        let condition =
                            self.expression(*condition, &mut locals, depth, basic_block.span)?;
                        block = if self.expect_bool(condition, basic_block.span)? {
                            *then_block
                        } else {
                            *else_block
                        };
                    }
                    MirTerminator::MatchTest {
                        match_id,
                        pattern,
                        matched_block,
                        unmatched_block,
                    } => {
                        let value = self.match_values.get(match_id).cloned().ok_or_else(|| {
                            runtime_error(basic_block.span, "MIR match value was not initialized")
                        })?;
                        let mut bindings = HashMap::new();
                        if match_pattern(pattern, &value, &mut bindings) {
                            locals.extend(bindings);
                            block = *matched_block;
                        } else {
                            block = *unmatched_block;
                        }
                    }
                    MirTerminator::MatchFailure { span } => {
                        return Err(runtime_error(*span, "MIR match had no matching arm"));
                    }
                    MirTerminator::IteratorNext {
                        iterator,
                        bindings,
                        body_block,
                        exit_block,
                    } => {
                        let value = self
                            .iterators
                            .get_mut(iterator)
                            .ok_or_else(|| {
                                runtime_error(basic_block.span, "MIR iterator was not initialized")
                            })?
                            .next();
                        if let Some(value) = value {
                            bind_iterator_value(&mut locals, bindings, value, basic_block.span)?;
                            block = *body_block;
                        } else {
                            block = *exit_block;
                        }
                    }
                    MirTerminator::Return(value) => {
                        return value.map_or(Ok(Value::Unit), |expression| {
                            self.expression(expression, &mut locals, depth, function.span)
                        });
                    }
                    MirTerminator::ReturnAfterLoopInvariants { value, invariants } => {
                        let result = value.map_or(Ok(Value::Unit), |expression| {
                            self.expression(expression, &mut locals, depth, function.span)
                        })?;
                        for invariant in invariants {
                            self.check_loop_invariants(
                                &[invariant.expression],
                                &mut locals,
                                depth,
                                invariant.loop_span,
                                LoopInvariantPhase::After,
                            )?;
                        }
                        return Ok(result);
                    }
                    MirTerminator::Unreachable => {
                        return Err(RuntimeError {
                            message: "MIR execution reached an unreachable block".into(),
                            span: basic_block.span,
                        });
                    }
                }
            }
        })()?;

        if !function.ensures.is_empty() {
            locals.insert(function.contract_result_local, result.clone());
            for contract in &function.ensures {
                let span = self.program.expressions[contract.0].span;
                let value = self.expression(*contract, &mut locals, depth, function.span)?;
                if !self.expect_bool(value, span)? {
                    return Err(runtime_error(
                        span,
                        format!("postcondition failed for function `{}`", function.name),
                    ));
                }
            }
        }
        Ok(result)
    }

    fn expression(
        &mut self,
        id: ExpressionId,
        locals: &mut HashMap<LocalId, Value>,
        depth: usize,
        fallback_span: Span,
    ) -> Result<Value, RuntimeError> {
        let expression = self
            .program
            .expressions
            .get(id.0)
            .ok_or_else(|| RuntimeError {
                message: "MIR expression id is outside the expression table".into(),
                span: fallback_span,
            })?;
        self.tick(expression.span)?;
        let span = expression.span;
        let value_type = expression.ty.clone();
        let kind = expression.kind.clone();
        match kind {
            MirExpressionKind::Int(value) => Ok(Value::Int(value)),
            MirExpressionKind::UInt(value) => Ok(Value::UInt(value)),
            MirExpressionKind::Float(value) => Ok(Value::Float(value)),
            MirExpressionKind::Bool(value) => Ok(Value::Bool(value)),
            MirExpressionKind::String(value) => Ok(Value::String(value)),
            MirExpressionKind::Char(value) => Ok(Value::Char(value)),
            MirExpressionKind::EnumVariant { enum_name, variant } => {
                let (type_name, type_args) = runtime_enum_type(&value_type, &enum_name);
                Ok(Value::Enum {
                    type_name,
                    type_args,
                    variant,
                    payloads: Vec::new(),
                })
            }
            MirExpressionKind::Local(local) => {
                locals.get(&local).cloned().ok_or_else(|| RuntimeError {
                    message: format!("MIR local {} is not initialized", local.0),
                    span,
                })
            }
            MirExpressionKind::FunctionValue {
                name,
                function: Some(function),
            } => {
                let definition = self.functions.get(&function).ok_or_else(|| {
                    runtime_error(span, format!("MIR function value `{name}` was not found"))
                })?;
                let Type::Function { return_type, .. } = value_type else {
                    return Err(runtime_error(
                        span,
                        "MIR function value does not have a function type",
                    ));
                };
                Ok(Value::Function(Box::new(FunctionValue {
                    name,
                    params: definition
                        .parameters
                        .iter()
                        .map(|(_, _, ty, _)| ty.clone())
                        .collect(),
                    return_type: *return_type,
                    capabilities: definition.declared_capabilities.clone(),
                    lambda_params: Vec::new(),
                    lambda_body: None,
                    captures: Vec::new(),
                    bytecode_target: None,
                    bytecode_captures: Vec::new(),
                })))
            }
            MirExpressionKind::Lambda { params, body } => {
                let Type::Function {
                    params: parameter_types,
                    return_type,
                    capabilities,
                } = value_type
                else {
                    return Err(runtime_error(
                        span,
                        "MIR lambda does not have a function type",
                    ));
                };
                if parameter_types.len() != params.len()
                    || params
                        .iter()
                        .zip(&parameter_types)
                        .any(|((_, _, actual), expected)| {
                            actual != &Type::Unknown && actual != expected
                        })
                {
                    return Err(runtime_error(
                        span,
                        "MIR lambda parameters do not match its checked function type",
                    ));
                }
                let id = self.next_closure_id.fetch_add(1, Ordering::Relaxed);
                let closure_name = format!("<zelyra-mir-closure:{id}>");
                let closure = MirCallback::Lambda {
                    params: params.clone(),
                    body,
                    captures: capture_locals(self.program, body, locals),
                    return_type: *return_type.clone(),
                    capabilities: capabilities.clone(),
                };
                self.closures
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(closure_name.clone(), closure);
                Ok(Value::Function(Box::new(FunctionValue {
                    name: closure_name,
                    params: parameter_types,
                    return_type: *return_type,
                    capabilities,
                    lambda_params: params.into_iter().map(|(_, name, _)| name).collect(),
                    lambda_body: None,
                    captures: Vec::new(),
                    bytecode_target: None,
                    bytecode_captures: Vec::new(),
                })))
            }
            MirExpressionKind::Array(items) => items
                .into_iter()
                .map(|item| self.expression(item, locals, depth, span))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            MirExpressionKind::Map(entries) => entries
                .into_iter()
                .map(|(key, value)| {
                    Ok((
                        self.expression(key, locals, depth, span)?,
                        self.expression(value, locals, depth, span)?,
                    ))
                })
                .collect::<Result<Vec<_>, RuntimeError>>()
                .map(normalize_map_entries)
                .map(Value::Map),
            MirExpressionKind::Record {
                type_name,
                type_args,
                fields,
            } => {
                let fields = fields
                    .into_iter()
                    .map(|(name, value)| Ok((name, self.expression(value, locals, depth, span)?)))
                    .collect::<Result<HashMap<_, _>, RuntimeError>>()?;
                Ok(Value::Object {
                    type_name,
                    type_args,
                    fields,
                })
            }
            MirExpressionKind::Index { target, index } => {
                let target = self.expression(target, locals, depth, span)?;
                let index = self.expression(index, locals, depth, span)?;
                index_value(target, index, span)
            }
            MirExpressionKind::Field { target, field } => {
                let target = self.expression(target, locals, depth, span)?;
                match target {
                    Value::Object { fields, .. } => fields.get(&field).cloned().ok_or_else(|| {
                        runtime_error(span, format!("record field `{field}` is missing"))
                    }),
                    Value::Json(value) => {
                        value.get(&field).cloned().map(Value::Json).ok_or_else(|| {
                            runtime_error(span, format!("JSON field `{field}` is missing"))
                        })
                    }
                    other => Err(runtime_error(
                        span,
                        format!("cannot read field `{field}` from `{}`", other.ty()),
                    )),
                }
            }
            MirExpressionKind::Match { value, arms } => {
                let value = self.expression(value, locals, depth, span)?;
                for arm in arms {
                    let mut bindings = HashMap::new();
                    if !match_pattern(&arm.pattern, &value, &mut bindings) {
                        continue;
                    }
                    let mut arm_locals = locals.clone();
                    arm_locals.extend(bindings);
                    if let Some(guard) = arm.guard {
                        let guard = self.expression(guard, &mut arm_locals, depth, arm.span)?;
                        if !self.expect_bool(guard, arm.span)? {
                            continue;
                        }
                    }
                    return self.expression(arm.value, &mut arm_locals, depth, arm.span);
                }
                Err(runtime_error(
                    span,
                    "MIR match expression had no matching arm",
                ))
            }
            MirExpressionKind::Sql {
                result_type,
                query,
                parameters,
                ..
            } => self.execute_sql(&query, &result_type, &parameters, locals, span),
            MirExpressionKind::Call {
                name,
                function: None,
                type_args,
                args,
                ..
            } if name == "record_update" => {
                if type_args.len() != 1 || args.is_empty() || args.len().is_multiple_of(2) {
                    return Err(runtime_error(
                        span,
                        "record update expects a base record followed by field/value pairs",
                    ));
                }
                let expected_type = type_args[0].clone();
                let base = self.expression(args[0], locals, depth, span)?;
                if !runtime_type_matches(&expected_type, &base.ty()) {
                    return Err(runtime_error(
                        span,
                        format!(
                            "record update expected `{expected_type}`, found `{}`",
                            base.ty()
                        ),
                    ));
                }
                let Value::Object {
                    type_name,
                    type_args: actual_type_args,
                    fields: mut record_fields,
                } = base
                else {
                    return Err(runtime_error(span, "record update requires a record value"));
                };
                let declaration = self
                    .program
                    .records
                    .iter()
                    .find(|record| record.name == type_name)
                    .ok_or_else(|| {
                        runtime_error(span, format!("unknown record type `{type_name}`"))
                    })?;
                if declaration.is_class || declaration.type_params.len() != actual_type_args.len() {
                    return Err(runtime_error(
                        span,
                        "record update cannot copy a class reference or malformed generic record",
                    ));
                }
                let substitutions = declaration
                    .type_params
                    .iter()
                    .cloned()
                    .zip(actual_type_args.iter().cloned())
                    .collect::<HashMap<_, _>>();
                let mut seen_fields = HashSet::new();
                for pair in args[1..].chunks(2) {
                    let field_value = self.expression(pair[0], locals, depth, span)?;
                    let Value::String(field_name) = field_value else {
                        return Err(runtime_error(
                            self.program.expressions[pair[0].0].span,
                            "record update field name must be a String literal",
                        ));
                    };
                    if !seen_fields.insert(field_name.clone()) {
                        return Err(runtime_error(
                            self.program.expressions[pair[0].0].span,
                            format!("record update repeats field `{field_name}`"),
                        ));
                    }
                    let field = declaration
                        .fields
                        .iter()
                        .find(|field| field.name == field_name)
                        .ok_or_else(|| {
                            runtime_error(
                                self.program.expressions[pair[0].0].span,
                                format!("record `{type_name}` has no field `{field_name}`"),
                            )
                        })?;
                    let value = self.expression(pair[1], locals, depth, span)?;
                    let expected =
                        super::bytecode::substitute_record_type(&field.ty, &substitutions);
                    if !runtime_type_matches(&expected, &value.ty()) {
                        return Err(runtime_error(
                            self.program.expressions[pair[1].0].span,
                            format!(
                                "record field `{field_name}` expects `{expected}`, found `{}`",
                                value.ty()
                            ),
                        ));
                    }
                    if !record_fields.contains_key(&field_name) {
                        return Err(runtime_error(
                            self.program.expressions[pair[0].0].span,
                            format!("record update field `{field_name}` is missing from the base"),
                        ));
                    }
                    record_fields.insert(field_name, value);
                }
                Ok(Value::Object {
                    type_name,
                    type_args: actual_type_args,
                    fields: record_fields,
                })
            }
            MirExpressionKind::Call {
                name,
                function: None,
                args,
                ..
            } if matches!(name.as_str(), "map" | "filter" | "find" | "reduce") => {
                self.higher_order_builtin(&name, &args, locals, depth, span)
            }
            MirExpressionKind::Call {
                name,
                function: None,
                args,
                ..
            } if name == "push_mut" || name == "pop_mut" => {
                let expected_arity = if name == "push_mut" { 2 } else { 1 };
                if args.len() != expected_arity {
                    return Err(runtime_error(
                        span,
                        format!("`{name}` expects {expected_arity} argument(s)"),
                    ));
                }
                let Some(MirExpressionKind::Local(local)) = self
                    .program
                    .expressions
                    .get(args[0].0)
                    .map(|expression| &expression.kind)
                else {
                    return Err(runtime_error(
                        span,
                        format!("`{name}` requires a mutable array binding"),
                    ));
                };
                let local = *local;
                let expected_element = self
                    .program
                    .expressions
                    .get(args[0].0)
                    .and_then(|expression| match &expression.ty {
                        Type::Array(element) => Some(element.as_ref().clone()),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        runtime_error(span, format!("`{name}` target is not an array"))
                    })?;
                let pushed_value = if name == "push_mut" {
                    Some(self.expression(args[1], locals, depth, span)?)
                } else {
                    None
                };
                let Some(Value::Array(values)) = locals.get_mut(&local) else {
                    return Err(runtime_error(
                        span,
                        format!("`{name}` target local is not an initialized array"),
                    ));
                };
                if let Some(value) = pushed_value {
                    if !runtime_type_matches(&expected_element, &value.ty()) {
                        return Err(runtime_error(
                            span,
                            format!(
                                "`push_mut` expected `{expected_element}`, found `{}`",
                                value.ty()
                            ),
                        ));
                    }
                    values.push(value);
                    Ok(Value::Unit)
                } else {
                    Ok(Value::Option(values.pop().map(Box::new)))
                }
            }
            MirExpressionKind::Call {
                name,
                function: None,
                type_args,
                args,
                ..
            } if name == "sql_decode" => {
                if type_args.len() != 1 || args.len() != 1 {
                    return Err(runtime_error(
                        span,
                        "sql_decode expects one type argument and one SQL expression",
                    ));
                }
                let query = match self
                    .program
                    .expressions
                    .get(args[0].0)
                    .map(|value| &value.kind)
                {
                    Some(MirExpressionKind::Sql { query, .. }) => query,
                    _ => {
                        return Err(runtime_error(
                            self.program.expressions[args[0].0].span,
                            "sql_decode requires a SELECT query",
                        ));
                    }
                };
                if zelyra_database::sql::analyze_table_access(query, &[]).0 != Some("SELECT") {
                    return Err(runtime_error(
                        self.program.expressions[args[0].0].span,
                        "sql_decode requires a SELECT query",
                    ));
                }
                let rows = self.expression(args[0], locals, depth, span)?;
                let records = self
                    .program
                    .records
                    .iter()
                    .chain(&self.program.sql_records)
                    .map(|record| (record.name.clone(), record.clone()))
                    .collect::<HashMap<_, _>>();
                let aliases = self
                    .program
                    .type_aliases
                    .iter()
                    .map(|alias| (alias.name.clone(), alias.target.clone()))
                    .collect::<HashMap<_, _>>();
                let enums = self
                    .program
                    .enums
                    .iter()
                    .map(|definition| definition.name.clone())
                    .collect::<HashSet<_>>();
                zelyra_runtime::decode_native_sql_rows(
                    rows,
                    &type_args[0],
                    &records,
                    &aliases,
                    &enums,
                    span,
                )
            }
            MirExpressionKind::Call {
                name,
                function: None,
                args,
                ..
            } if name == "print" => {
                if args.len() != 1 {
                    return Err(runtime_error(span, "`print` expects one argument"));
                }
                let value = self.expression(args[0], locals, depth, span)?;
                self.output
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(value.output());
                Ok(Value::Unit)
            }
            MirExpressionKind::Call {
                name,
                function: None,
                args,
                ..
            } if name == "tuple" => args
                .into_iter()
                .map(|argument| self.expression(argument, locals, depth, span))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Tuple),
            MirExpressionKind::Call {
                name,
                function: None,
                type_args,
                args,
                ..
            } if name == "render_pdf" => {
                if !type_args.is_empty() || args.len() != 2 {
                    return Err(runtime_error(
                        span,
                        "render_pdf expects a named View and one record value",
                    ));
                }
                let view_name = match self.expression(args[0], locals, depth, span)? {
                    Value::String(name) => name,
                    value => {
                        return Err(runtime_error(
                            span,
                            format!("render_pdf expects a View name, found {}", value.ty()),
                        ));
                    }
                };
                let view = self
                    .program
                    .views
                    .iter()
                    .find(|view| view.name == view_name)
                    .ok_or_else(|| {
                        runtime_error(span, format!("unknown PDF view `{view_name}`"))
                    })?;
                let data = self.expression(args[1], locals, depth, span)?;
                zelyra_runtime::render_pdf_view(view, &data)
                    .map(Value::Bytes)
                    .map_err(|error| runtime_error(span, error))
            }
            MirExpressionKind::Call {
                name,
                function: None,
                type_args,
                args,
                ..
            } if matches!(
                name.as_str(),
                "now"
                    | "env"
                    | "random_int"
                    | "uuid_v4"
                    | "read_console"
                    | "run_process"
                    | "read_text"
                    | "read_text_result"
                    | "file_exists"
                    | "dir_exists"
                    | "file_size"
                    | "list_dir"
                    | "write_text"
                    | "write_text_result"
                    | "write_text_new_result"
                    | "atomic_write_text_result"
                    | "append_text_result"
                    | "copy_file_new_result"
                    | "rename_file_result"
                    | "delete_file"
                    | "delete_file_result"
                    | "http_get"
                    | "http_request"
                    | "http_json"
                    | "http_result"
                    | "mail_deliver"
                    | "json_encode"
                    | "json_decode"
                    | "json_decode_result"
                    | "json_encode_tagged_result"
                    | "json_decode_tagged_result"
                    | "module_setting"
            ) =>
            {
                self.host_builtin(&name, &type_args, &args, locals, depth, span)
            }
            MirExpressionKind::Call {
                name,
                function: None,
                args,
                ..
            } if is_pure_builtin(&name) => {
                let values = args
                    .into_iter()
                    .map(|argument| self.expression(argument, locals, depth, span))
                    .collect::<Result<Vec<_>, _>>()?;
                execute_pure_builtin(&name, values, span)
            }
            MirExpressionKind::Call {
                name,
                function: None,
                args,
                ..
            } if name.rsplit_once("::").is_some() => {
                let (enum_name, variant) = name.rsplit_once("::").expect("guarded split");
                let (type_name, type_args) = runtime_enum_type(&value_type, enum_name);
                let payloads = args
                    .into_iter()
                    .map(|argument| self.expression(argument, locals, depth, span))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::Enum {
                    type_name,
                    type_args,
                    variant: variant.into(),
                    payloads,
                })
            }
            MirExpressionKind::Binary { left, op, right } => {
                let left_value = self.expression(left, locals, depth, span)?;
                if op == BinaryOp::And && left_value == Value::Bool(false) {
                    return Ok(Value::Bool(false));
                }
                if op == BinaryOp::Or && left_value == Value::Bool(true) {
                    return Ok(Value::Bool(true));
                }
                let right_value = self.expression(right, locals, depth, span)?;
                evaluate_binary(left_value, op, right_value, span)
            }
            MirExpressionKind::BinaryChain { first, operations } => {
                let mut value = self.expression(first, locals, depth, span)?;
                for (op, right, operation_span) in operations {
                    if op == BinaryOp::And && value == Value::Bool(false) {
                        return Ok(Value::Bool(false));
                    }
                    if op == BinaryOp::Or && value == Value::Bool(true) {
                        return Ok(Value::Bool(true));
                    }
                    let right = self.expression(right, locals, depth, span)?;
                    value = evaluate_binary(value, op, right, operation_span)?;
                }
                Ok(value)
            }
            MirExpressionKind::Unary { op, expr } => {
                let value = self.expression(expr, locals, depth, span)?;
                evaluate_unary(op, value, span)
            }
            MirExpressionKind::Await(expr) => self.expression(expr, locals, depth, span),
            MirExpressionKind::Call {
                function: Some(function),
                args,
                ..
            } => {
                let values = args
                    .into_iter()
                    .map(|argument| self.expression(argument, locals, depth, span))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call(function, values, depth + 1, span)
            }
            MirExpressionKind::Call {
                name,
                function: None,
                callee_local: Some(local),
                type_args,
                args,
            } => {
                if !type_args.is_empty() {
                    return Err(runtime_error(
                        span,
                        "generic function values cannot be called with explicit type arguments",
                    ));
                }
                let callable = locals.get(&local).cloned().ok_or_else(|| {
                    runtime_error(
                        span,
                        format!("MIR callee local {} is not initialized", local.0),
                    )
                })?;
                let Value::Function(callable) = callable else {
                    return Err(runtime_error(
                        span,
                        format!("MIR value `{name}` is not callable"),
                    ));
                };
                let closure = self
                    .closures
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get(&callable.name)
                    .cloned();
                if let Some(closure) = closure {
                    let values = args
                        .into_iter()
                        .map(|argument| self.expression(argument, locals, depth, span))
                        .collect::<Result<Vec<_>, _>>()?;
                    return self.invoke_mir_callback(&closure, values, depth, span);
                }
                let target = self
                    .functions
                    .values()
                    .find(|function| function.name == callable.name)
                    .map(|function| function.id)
                    .ok_or_else(|| {
                        runtime_error(span, "MIR function value target was not found")
                    })?;
                let values = args
                    .into_iter()
                    .map(|argument| self.expression(argument, locals, depth, span))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call(target, values, depth + 1, span)
            }
            unsupported => Err(runtime_error(
                span,
                format!(
                    "MIR interpreter does not support expression `{}` yet",
                    expression_name(&unsupported)
                ),
            )),
        }
    }

    fn expect_bool(&self, value: Value, span: Span) -> Result<bool, RuntimeError> {
        match value {
            Value::Bool(value) => Ok(value),
            other => Err(runtime_error(
                span,
                format!("MIR branch expected Bool but received `{}`", other.ty()),
            )),
        }
    }

    fn host_builtin(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[ExpressionId],
        locals: &mut HashMap<LocalId, Value>,
        depth: usize,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match name {
            "module_setting" => {
                if type_args.len() != 1 || args.len() != 1 {
                    return Err(runtime_error(
                        span,
                        "module_setting expects one type argument and one setting name",
                    ));
                }
                let setting_name = match self.expression(args[0], locals, depth, span)? {
                    Value::String(name) => name,
                    value => {
                        return Err(runtime_error(
                            self.program.expressions[args[0].0].span,
                            format!("module_setting expects a String name, found {}", value.ty()),
                        ));
                    }
                };
                let value = self
                    .package_settings
                    .as_ref()
                    .ok_or_else(|| runtime_error(span, "package settings are unavailable"))?
                    .get_value(span.source_id, &setting_name, &type_args[0])
                    .map_err(|error| runtime_error(span, error))?;
                if !runtime_type_matches(&type_args[0], &value.ty()) {
                    return Err(runtime_error(
                        span,
                        format!("module_setting result does not match `{}`", type_args[0]),
                    ));
                }
                Ok(value)
            }
            "now" => {
                if !args.is_empty() {
                    return Err(runtime_error(span, "now expects no arguments"));
                }
                self.require_host_capability("Clock", "clock access", span)?;
                let timestamp = self.host_services.unix_time_millis().map_err(|error| {
                    runtime_error(span, format!("clock access failed: {error}"))
                })?;
                if !zelyra_ast::temporal::valid_timestamp_millis(timestamp) {
                    return Err(runtime_error(
                        span,
                        "clock access failed: timestamp is outside years 0001 through 9999",
                    ));
                }
                Ok(Value::Timestamp(timestamp))
            }
            "env" => {
                if args.len() != 1 {
                    return Err(runtime_error(span, "env expects exactly one argument"));
                }
                self.require_host_capability("Environment", "environment access", span)?;
                let key = self.expression(args[0], locals, depth, span)?;
                let Value::String(key) = key else {
                    return Err(runtime_error(span, "env expects a String variable name"));
                };
                if key.is_empty() {
                    return Err(runtime_error(span, "env expects a non-empty variable name"));
                }
                let value = self
                    .host_services
                    .environment_variable(&key)
                    .map_err(|error| runtime_error(span, error))?;
                Ok(Value::Option(
                    value.map(|value| Box::new(Value::String(value))),
                ))
            }
            "random_int" => {
                if args.len() != 2 {
                    return Err(runtime_error(
                        span,
                        "random_int expects minimum and maximum Int arguments",
                    ));
                }
                self.require_host_capability("Random", "random access", span)?;
                let minimum = self.expression(args[0], locals, depth, span)?;
                let maximum = self.expression(args[1], locals, depth, span)?;
                let (Value::Int(minimum), Value::Int(maximum)) = (minimum, maximum) else {
                    return Err(runtime_error(span, "random_int expects Int arguments"));
                };
                if minimum > maximum {
                    return Err(runtime_error(
                        span,
                        "random_int requires minimum <= maximum",
                    ));
                }
                random_int_with_host_services(self.host_services.as_ref(), minimum, maximum)
                    .map(Value::Int)
                    .map_err(|error| runtime_error(span, error))
            }
            "uuid_v4" => {
                if !args.is_empty() {
                    return Err(runtime_error(span, "uuid_v4 expects no arguments"));
                }
                self.require_host_capability("Random", "random access", span)?;
                uuid_v4_with_host_services(self.host_services.as_ref())
                    .map(Value::String)
                    .map_err(|error| runtime_error(span, format!("random access failed: {error}")))
            }
            "read_console" => {
                if args.len() != 1 {
                    return Err(runtime_error(
                        span,
                        "read_console expects exactly one String prompt",
                    ));
                }
                self.require_host_capability("Console", "console input", span)?;
                let prompt = self.expression(args[0], locals, depth, span)?;
                let Value::String(prompt) = prompt else {
                    return Err(runtime_error(span, "read_console expects a String prompt"));
                };
                let value = self
                    .host_services
                    .read_console_line(&prompt)
                    .map_err(|error| {
                        runtime_error(span, format!("console input failed: {error}"))
                    })?;
                Ok(Value::Option(
                    value.map(|value| Box::new(Value::String(value))),
                ))
            }
            "mail_deliver" => {
                if args.len() != 2 {
                    return Err(runtime_error(
                        span,
                        "mail_deliver expects an outbox event ID and payload JSON",
                    ));
                }
                self.require_host_capability("Email", "email delivery", span)?;
                let event_id = self.expression(args[0], locals, depth, span)?;
                let payload_json = self.expression(args[1], locals, depth, span)?;
                let (Value::String(event_id), Value::String(payload_json)) =
                    (event_id, payload_json)
                else {
                    return Err(runtime_error(span, "mail_deliver expects String arguments"));
                };
                let result = deliver_outbox_message_with_host(
                    &event_id,
                    &payload_json,
                    self.mail_policy.as_ref(),
                    self.network_policy.as_ref(),
                    self.host_services.as_ref(),
                )
                .map(|()| Box::new(Value::Unit))
                .map_err(|error| Box::new(Value::String(error)));
                Ok(Value::Result(result))
            }
            "json_encode" | "json_encode_tagged_result" => {
                if !type_args.is_empty() || args.len() != 1 {
                    return Err(runtime_error(
                        span,
                        format!("{name} expects one value and no type arguments"),
                    ));
                }
                let value = self.expression(args[0], locals, depth, span)?;
                let encoded = if name == "json_encode" {
                    zelyra_runtime::encode_json_value(&value, span)
                } else {
                    zelyra_runtime::encode_tagged_result_json(&value, span)
                }
                .map_err(|error| runtime_error(span, error.message))?;
                Ok(Value::String(encoded))
            }
            "json_decode" | "json_decode_result" => {
                if type_args.len() != 1 || args.len() != 1 {
                    return Err(runtime_error(
                        span,
                        format!("{name} expects one type argument and one String value"),
                    ));
                }
                let source = self.expression(args[0], locals, depth, span)?;
                let Value::String(source) = source else {
                    return Err(runtime_error(
                        span,
                        format!("{name} expects a String value"),
                    ));
                };
                let decoded = zelyra_runtime::decode_json_value(
                    &source,
                    &type_args[0],
                    &self.program.records,
                    &self.program.type_aliases,
                    &self.program.enums,
                    span,
                );
                if name == "json_decode" {
                    decoded
                } else {
                    Ok(Value::Result(
                        decoded
                            .map(Box::new)
                            .map_err(|error| Box::new(Value::String(error.message))),
                    ))
                }
            }
            "json_decode_tagged_result" => {
                if type_args.len() != 2 || args.len() != 1 {
                    return Err(runtime_error(
                        span,
                        "json_decode_tagged_result expects Ok and Err type arguments and one String value",
                    ));
                }
                let source = self.expression(args[0], locals, depth, span)?;
                let Value::String(source) = source else {
                    return Err(runtime_error(
                        span,
                        "json_decode_tagged_result expects a String value",
                    ));
                };
                let decoded = zelyra_runtime::decode_tagged_result_json(
                    &source,
                    &type_args[0],
                    &type_args[1],
                    &self.program.records,
                    &self.program.type_aliases,
                    &self.program.enums,
                    span,
                );
                Ok(Value::Result(
                    decoded
                        .map(Box::new)
                        .map_err(|error| Box::new(Value::String(error))),
                ))
            }
            "http_json" => {
                let response =
                    self.http_typed_request(name, type_args, args, locals, depth, span)?;
                if !(200..300).contains(&response.status) {
                    return Err(runtime_error(
                        span,
                        format!("http_json received HTTP status {}", response.status),
                    ));
                }
                let body = String::from_utf8(response.body)
                    .map_err(|_| runtime_error(span, "network response is not valid UTF-8"))?;
                zelyra_runtime::decode_json_value(
                    &body,
                    &type_args[1],
                    &self.program.records,
                    &self.program.type_aliases,
                    &self.program.enums,
                    span,
                )
            }
            "http_result" => {
                let response =
                    self.http_typed_request(name, type_args, args, locals, depth, span)?;
                let status = i64::from(response.status);
                let headers = response
                    .headers
                    .into_iter()
                    .map(|(name, value)| Value::String(format!("{name}: {value}")))
                    .collect::<Vec<_>>();
                let body = String::from_utf8(response.body)
                    .map_err(|_| runtime_error(span, "network response is not valid UTF-8"))?;
                let successful = (200..300).contains(&status);
                let data = if successful {
                    let decoded = if type_args[1] == Type::Unit && body.trim().is_empty() {
                        Value::Unit
                    } else {
                        zelyra_runtime::decode_json_value(
                            &body,
                            &type_args[1],
                            &self.program.records,
                            &self.program.type_aliases,
                            &self.program.enums,
                            span,
                        )?
                    };
                    Value::Option(Some(Box::new(decoded)))
                } else {
                    Value::Option(None)
                };
                let error = if successful {
                    Value::Option(None)
                } else {
                    Value::Option(Some(Box::new(Value::Object {
                        type_name: "HttpError".into(),
                        type_args: Vec::new(),
                        fields: HashMap::from([
                            ("status".into(), Value::Int(status)),
                            ("headers".into(), Value::Array(headers.clone())),
                            ("body".into(), Value::String(body.clone())),
                            (
                                "message".into(),
                                Value::String(format!("HTTP status {status}")),
                            ),
                        ]),
                    })))
                };
                Ok(Value::Object {
                    type_name: "HttpResult".into(),
                    type_args: Vec::new(),
                    fields: HashMap::from([
                        ("status".into(), Value::Int(status)),
                        ("headers".into(), Value::Array(headers)),
                        ("body".into(), Value::String(body)),
                        ("data".into(), data),
                        ("error".into(), error),
                    ]),
                })
            }
            "http_get" => {
                if args.len() != 1 {
                    return Err(runtime_error(
                        span,
                        "http_get expects exactly one String URL",
                    ));
                }
                self.require_host_capability("Network", "network access", span)?;
                let url = self.expression(args[0], locals, depth, span)?;
                let Value::String(url) = url else {
                    return Err(runtime_error(span, "http_get expects a String URL"));
                };
                let response = execute_http_request_with_host(
                    "GET",
                    &url,
                    &[],
                    None,
                    self.network_policy.as_ref(),
                    self.host_services.as_ref(),
                )
                .map_err(|error| runtime_error(span, error))?;
                if !(200..300).contains(&response.status) {
                    return Err(runtime_error(
                        span,
                        format!("http_get received HTTP status {}", response.status),
                    ));
                }
                String::from_utf8(response.body)
                    .map(Value::String)
                    .map_err(|_| runtime_error(span, "network response is not valid UTF-8"))
            }
            "http_request" => {
                if args.len() != 4 {
                    return Err(runtime_error(
                        span,
                        "http_request expects String method, String URL, String[] headers, and String? body",
                    ));
                }
                self.require_host_capability("Network", "network access", span)?;
                let method = self.expression(args[0], locals, depth, span)?;
                let url = self.expression(args[1], locals, depth, span)?;
                let headers = self.expression(args[2], locals, depth, span)?;
                let body = self.expression(args[3], locals, depth, span)?;
                let Value::String(method) = method else {
                    return Err(runtime_error(span, "http_request expects a String method"));
                };
                let Value::String(url) = url else {
                    return Err(runtime_error(span, "http_request expects a String URL"));
                };
                let Value::Array(headers) = headers else {
                    return Err(runtime_error(span, "http_request expects String[] headers"));
                };
                let mut request_headers = Vec::with_capacity(headers.len());
                for header in headers {
                    let Value::String(header) = header else {
                        return Err(runtime_error(span, "http_request expects String[] headers"));
                    };
                    request_headers.push(header);
                }
                let body = match body {
                    Value::Option(Some(body)) => match *body {
                        Value::String(body) => Some(body),
                        _ => {
                            return Err(runtime_error(
                                span,
                                "http_request expects an optional String body",
                            ));
                        }
                    },
                    Value::Option(None) => None,
                    _ => {
                        return Err(runtime_error(
                            span,
                            "http_request expects an optional String body",
                        ));
                    }
                };
                let response = execute_http_request_with_host(
                    &method,
                    &url,
                    &request_headers,
                    body.as_deref(),
                    self.network_policy.as_ref(),
                    self.host_services.as_ref(),
                )
                .map_err(|error| runtime_error(span, error))?;
                http_response_value(response, span)
            }
            "read_text" => {
                if args.len() != 1 {
                    return Err(runtime_error(
                        span,
                        "read_text expects exactly one String path",
                    ));
                }
                self.require_host_capability("FileSystem", "file-system read access", span)?;
                let path = self.expression(args[0], locals, depth, span)?;
                let Value::String(path) = path else {
                    return Err(runtime_error(span, "read_text expects a String path"));
                };
                if path.is_empty() {
                    return Err(runtime_error(span, "read_text expects a non-empty path"));
                }
                let path = inspect_filesystem_path_with_host(
                    self.filesystem_policy.as_ref(),
                    &path,
                    false,
                    false,
                    self.host_services.as_ref(),
                )
                .map_err(|error| runtime_error(span, error))?;
                self.host_services
                    .read_text(&path)
                    .map(Value::String)
                    .map_err(|error| {
                        runtime_error(span, format!("file-system read failed: {error}"))
                    })
            }
            "read_text_result" => {
                if args.len() != 1 {
                    return Err(runtime_error(
                        span,
                        "read_text_result expects exactly one String path",
                    ));
                }
                self.require_host_capability("FileSystem", "file-system read access", span)?;
                let path = self.expression(args[0], locals, depth, span)?;
                let outcome = match path {
                    Value::String(path) if !path.is_empty() => inspect_filesystem_path_with_host(
                        self.filesystem_policy.as_ref(),
                        &path,
                        false,
                        false,
                        self.host_services.as_ref(),
                    )
                    .and_then(|path| {
                        self.host_services
                            .read_text(&path)
                            .map_err(|error| format!("file-system read failed: {error}"))
                    }),
                    Value::String(_) => Err("read_text_result expects a non-empty path".into()),
                    _ => Err("read_text_result expects a String path".into()),
                };
                Ok(Value::Result(
                    outcome
                        .map(|value| Box::new(Value::String(value)))
                        .map_err(|error| Box::new(Value::String(error))),
                ))
            }
            "file_exists" => {
                if args.len() != 1 {
                    return Err(runtime_error(span, "file_exists expects one String"));
                }
                self.require_host_capability("FileSystem", "file-system read access", span)?;
                let path = self.expression(args[0], locals, depth, span)?;
                let Value::String(path) = path else {
                    return Err(runtime_error(span, "file_exists expects a String"));
                };
                let exists = inspect_filesystem_path_with_host(
                    self.filesystem_policy.as_ref(),
                    &path,
                    false,
                    true,
                    self.host_services.as_ref(),
                )
                .ok()
                .is_some_and(|path| {
                    self.host_services
                        .file_metadata(&path)
                        .is_ok_and(|metadata| metadata.is_file)
                });
                Ok(Value::Bool(exists))
            }
            "dir_exists" => {
                if args.len() != 1 {
                    return Err(runtime_error(span, "dir_exists expects one String"));
                }
                self.require_host_capability("FileSystem", "file-system read access", span)?;
                let path = self.expression(args[0], locals, depth, span)?;
                let Value::String(path) = path else {
                    return Err(runtime_error(span, "dir_exists expects a String"));
                };
                let exists = inspect_filesystem_path_with_host(
                    self.filesystem_policy.as_ref(),
                    &path,
                    false,
                    true,
                    self.host_services.as_ref(),
                )
                .ok()
                .is_some_and(|path| {
                    self.host_services
                        .file_metadata(&path)
                        .is_ok_and(|metadata| metadata.is_directory)
                });
                Ok(Value::Bool(exists))
            }
            "file_size" => {
                if args.len() != 1 {
                    return Err(runtime_error(span, "file_size expects one String"));
                }
                self.require_host_capability("FileSystem", "file-system read access", span)?;
                let path = self.expression(args[0], locals, depth, span)?;
                let Value::String(path) = path else {
                    return Err(runtime_error(span, "file_size expects a String"));
                };
                let path = inspect_filesystem_path_with_host(
                    self.filesystem_policy.as_ref(),
                    &path,
                    false,
                    false,
                    self.host_services.as_ref(),
                )
                .map_err(|error| runtime_error(span, error))?;
                let outcome = self
                    .host_services
                    .file_metadata(&path)
                    .map(|metadata| Box::new(Value::Int(metadata.length as i64)))
                    .map_err(|error| Box::new(Value::String(error)));
                Ok(Value::Result(outcome))
            }
            "list_dir" => {
                if args.len() != 1 {
                    return Err(runtime_error(span, "list_dir expects one String path"));
                }
                self.require_host_capability("FileSystem", "file-system read access", span)?;
                let path = self.expression(args[0], locals, depth, span)?;
                let Value::String(path) = path else {
                    return Err(runtime_error(span, "list_dir expects a String path"));
                };
                if path.is_empty() {
                    return Err(runtime_error(span, "list_dir expects a non-empty path"));
                }
                let path = inspect_filesystem_path_with_host(
                    self.filesystem_policy.as_ref(),
                    &path,
                    false,
                    false,
                    self.host_services.as_ref(),
                )
                .map_err(|error| runtime_error(span, error))?;
                self.host_services
                    .list_directory(&path)
                    .map(|entries| Value::Array(entries.into_iter().map(Value::String).collect()))
                    .map_err(|error| {
                        runtime_error(span, format!("file-system list failed: {error}"))
                    })
            }
            "write_text" => {
                if args.len() != 2 {
                    return Err(runtime_error(
                        span,
                        "write_text expects a String path and String content",
                    ));
                }
                self.require_host_capability("FileSystem", "file-system write access", span)?;
                let path = self.expression(args[0], locals, depth, span)?;
                let content = self.expression(args[1], locals, depth, span)?;
                let (Value::String(path), Value::String(content)) = (path, content) else {
                    return Err(runtime_error(span, "write_text expects String arguments"));
                };
                if path.is_empty() {
                    return Err(runtime_error(span, "write_text expects a non-empty path"));
                }
                let path = inspect_filesystem_path_with_host(
                    self.filesystem_policy.as_ref(),
                    &path,
                    true,
                    true,
                    self.host_services.as_ref(),
                )
                .map_err(|error| runtime_error(span, error))?;
                self.host_services
                    .write_text(&path, &content)
                    .map_err(|error| {
                        runtime_error(span, format!("file-system write failed: {error}"))
                    })?;
                Ok(Value::Unit)
            }
            "write_text_result"
            | "write_text_new_result"
            | "atomic_write_text_result"
            | "append_text_result" => {
                if args.len() != 2 {
                    return Err(runtime_error(
                        span,
                        format!("{name} expects a String path and String content"),
                    ));
                }
                self.require_host_capability("FileSystem", "file-system write access", span)?;
                let path = self.expression(args[0], locals, depth, span)?;
                let content = self.expression(args[1], locals, depth, span)?;
                let outcome = match (path, content) {
                    (Value::String(path), Value::String(content)) if !path.is_empty() => {
                        match inspect_filesystem_path_with_host(
                            self.filesystem_policy.as_ref(),
                            &path,
                            true,
                            true,
                            self.host_services.as_ref(),
                        ) {
                            Err(error) => Err(error),
                            Ok(path) => {
                                let result = match name {
                                    "write_text_result" => {
                                        self.host_services.write_text(&path, &content)
                                    }
                                    "write_text_new_result" => {
                                        self.host_services.write_text_new(&path, &content)
                                    }
                                    "atomic_write_text_result" => {
                                        self.host_services.atomic_write_text(&path, &content)
                                    }
                                    _ => self.host_services.append_text(&path, &content),
                                };
                                result
                                    .map_err(|error| format!("file-system {name} failed: {error}"))
                            }
                        }
                    }
                    (Value::String(_), Value::String(_)) => {
                        Err(format!("{name} expects a non-empty path"))
                    }
                    _ => Err(format!("{name} expects String arguments")),
                };
                Ok(Value::Result(
                    outcome
                        .map(|()| Box::new(Value::Unit))
                        .map_err(|error| Box::new(Value::String(error))),
                ))
            }
            "copy_file_new_result" | "rename_file_result" => {
                if args.len() != 2 {
                    return Err(runtime_error(
                        span,
                        format!("{name} expects source and destination String paths"),
                    ));
                }
                self.require_host_capability("FileSystem", "file-system write access", span)?;
                let source = self.expression(args[0], locals, depth, span)?;
                let destination = self.expression(args[1], locals, depth, span)?;
                let outcome = match (source, destination) {
                    (Value::String(source), Value::String(destination))
                        if !source.is_empty() && !destination.is_empty() =>
                    {
                        let source_write = name == "rename_file_result";
                        match inspect_filesystem_path_with_host(
                            self.filesystem_policy.as_ref(),
                            &source,
                            source_write,
                            false,
                            self.host_services.as_ref(),
                        ) {
                            Err(error) => Err(error),
                            Ok(source) => match inspect_filesystem_path_with_host(
                                self.filesystem_policy.as_ref(),
                                &destination,
                                true,
                                true,
                                self.host_services.as_ref(),
                            ) {
                                Err(error) => Err(error),
                                Ok(destination) => {
                                    let result = if name == "copy_file_new_result" {
                                        self.host_services.copy_file_new(&source, &destination)
                                    } else {
                                        self.host_services.rename_file_new(&source, &destination)
                                    };
                                    result.map_err(|error| {
                                        format!("file-system {name} failed: {error}")
                                    })
                                }
                            },
                        }
                    }
                    (Value::String(_), Value::String(_)) => {
                        Err(format!("{name} expects non-empty paths"))
                    }
                    _ => Err(format!("{name} expects String paths")),
                };
                Ok(Value::Result(
                    outcome
                        .map(|()| Box::new(Value::Unit))
                        .map_err(|error| Box::new(Value::String(error))),
                ))
            }
            "delete_file" | "delete_file_result" => {
                if args.len() != 1 {
                    return Err(runtime_error(span, "delete_file expects one String path"));
                }
                self.require_host_capability("FileSystem", "file-system delete access", span)?;
                let path = self.expression(args[0], locals, depth, span)?;
                let outcome = match path {
                    Value::String(path) if !path.is_empty() => {
                        match inspect_filesystem_path_with_host(
                            self.filesystem_policy.as_ref(),
                            &path,
                            true,
                            false,
                            self.host_services.as_ref(),
                        ) {
                            Err(error) => Err(error),
                            Ok(path) => self
                                .host_services
                                .remove_file(&path)
                                .map_err(|error| format!("file-system delete failed: {error}")),
                        }
                    }
                    Value::String(_) => Err("delete_file expects a non-empty path".into()),
                    _ => Err("delete_file expects a String path".into()),
                };
                if name == "delete_file" {
                    outcome
                        .map(|()| Value::Unit)
                        .map_err(|error| runtime_error(span, error))
                } else {
                    Ok(Value::Result(
                        outcome
                            .map(|()| Box::new(Value::Unit))
                            .map_err(|error| Box::new(Value::String(error))),
                    ))
                }
            }
            "run_process" => {
                if args.len() != 2 {
                    return Err(runtime_error(
                        span,
                        "run_process expects a String command and String[] arguments",
                    ));
                }
                self.require_host_capability("Process", "process execution", span)?;
                let command = self.expression(args[0], locals, depth, span)?;
                let arguments = self.expression(args[1], locals, depth, span)?;
                let Value::String(command) = command else {
                    return Err(runtime_error(span, "run_process expects a String command"));
                };
                let Value::Array(arguments) = arguments else {
                    return Err(runtime_error(
                        span,
                        "run_process expects String[] arguments",
                    ));
                };
                let mut argument_strings = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    let Value::String(argument) = argument else {
                        return Err(runtime_error(
                            span,
                            "run_process expects String[] arguments",
                        ));
                    };
                    argument_strings.push(argument);
                }
                let Some(policy) = &self.process_policy else {
                    return Err(runtime_error(
                        span,
                        "process execution denied: no process policy is configured",
                    ));
                };
                if command.is_empty() || command.contains(['\r', '\n']) {
                    return Err(runtime_error(
                        span,
                        "run_process received an invalid command",
                    ));
                }
                if !policy
                    .allowed_commands
                    .iter()
                    .any(|allowed| allowed == &command)
                {
                    return Err(runtime_error(
                        span,
                        format!("process execution denied for command `{command}`"),
                    ));
                }
                if policy.timeout_ms == 0 || policy.max_output_bytes == 0 {
                    return Err(runtime_error(
                        span,
                        "process policy requires positive timeout and output limits",
                    ));
                }
                let output = self
                    .host_services
                    .run_process(
                        &command,
                        &argument_strings,
                        policy.timeout_ms,
                        policy.max_output_bytes,
                    )
                    .map_err(|error| runtime_error(span, error))?;
                Ok(Value::String(output))
            }
            _ => unreachable!("host builtin name was validated"),
        }
    }

    fn execute_sql(
        &mut self,
        query: &str,
        result_type: &Type,
        parameters: &[(String, LocalId)],
        locals: &HashMap<LocalId, Value>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        zelyra_runtime::check_scoped_sql(query, &self.program.scoped_tables)
            .map_err(|message| runtime_error(span, message))?;
        let capability = zelyra_runtime::sql_required_capability(query);
        self.require_host_capability(capability, "SQL access", span)?;
        let database_url = self.database_url.as_deref().ok_or_else(|| RuntimeError {
            message: "SQL execution requires a database runtime".into(),
            span,
        })?;
        let mut named_values = HashMap::new();
        for (name, local) in parameters {
            let value = locals.get(local).cloned().ok_or_else(|| RuntimeError {
                message: format!("SQL parameter `:{name}` is not available"),
                span,
            })?;
            named_values.insert(name.clone(), value);
        }
        let params = zelyra_runtime::sql_parameters_from_named_values(query, &named_values, span)?;
        if !self.storage_checked {
            zelyra_runtime::validate_database_storage_with_host(
                database_url,
                &self.storage_schema,
                self.host_services.as_ref(),
                span,
            )?;
            self.storage_checked = true;
        }
        let result = zelyra_runtime::execute_database_query_with_host(
            database_url,
            &zelyra_database::Query {
                sql: query.into(),
                params,
            },
            self.host_services.as_ref(),
            span,
        )?;
        Ok(Value::Rows {
            columns: result.columns,
            rows: result.rows,
            metadata: Box::new(zelyra_runtime::SqlRowsMetadata {
                nulls: result.nulls,
                column_types: result.column_types,
            }),
            result_type: Box::new(result_type.clone()),
        })
    }

    fn execute_transaction(
        &mut self,
        operation_ids: &[ExpressionId],
        locals: &mut HashMap<LocalId, Value>,
        depth: usize,
        span: Span,
    ) -> Result<(), RuntimeError> {
        let database_url = self.database_url.clone().ok_or_else(|| RuntimeError {
            message: "transaction execution requires a database runtime".into(),
            span,
        })?;
        let mut queries = Vec::with_capacity(operation_ids.len());
        for operation_id in operation_ids {
            let expression = self
                .program
                .expressions
                .get(operation_id.0)
                .ok_or_else(|| runtime_error(span, "MIR transaction references missing SQL"))?;
            match &expression.kind {
                MirExpressionKind::Sql {
                    query, parameters, ..
                } => {
                    zelyra_runtime::check_scoped_sql(query, &self.program.scoped_tables)
                        .map_err(|message| runtime_error(expression.span, message))?;
                    let capability = zelyra_runtime::sql_required_capability(query);
                    self.require_host_capability(capability, "SQL transaction", expression.span)?;
                    let mut named_values = HashMap::new();
                    for (name, local) in parameters {
                        let value = locals.get(local).cloned().ok_or_else(|| RuntimeError {
                            message: format!("SQL parameter `:{name}` is not available"),
                            span: expression.span,
                        })?;
                        named_values.insert(name.clone(), value);
                    }
                    let params = zelyra_runtime::sql_parameters_from_named_values(
                        query,
                        &named_values,
                        expression.span,
                    )?;
                    queries.push(zelyra_database::Query {
                        sql: query.clone(),
                        params,
                    });
                }
                MirExpressionKind::Call { name, args, .. } if name == "mail_queue" => {
                    self.require_host_capability("Email", "email outbox", expression.span)?;
                    self.require_host_capability(
                        "Database(write)",
                        "transactional mail queue write",
                        expression.span,
                    )?;
                    if !zelyra_runtime::supports_mail_outbox_database(&database_url) {
                        return Err(runtime_error(
                            expression.span,
                            "mail_queue currently requires MariaDB or MySQL; PostgreSQL outbox support is not available",
                        ));
                    }
                    let mut values = Vec::with_capacity(args.len());
                    let mut argument_spans = Vec::with_capacity(args.len());
                    for argument in args {
                        argument_spans.push(
                            self.program
                                .expressions
                                .get(argument.0)
                                .map(|argument| argument.span)
                                .unwrap_or(expression.span),
                        );
                        values.push(self.expression(*argument, locals, depth, expression.span)?);
                    }
                    queries.push(zelyra_runtime::prepare_mail_queue_query(
                        &values,
                        &argument_spans,
                        expression.span,
                        self.host_services.as_ref(),
                    )?);
                }
                _ => {
                    return Err(runtime_error(
                        expression.span,
                        "MIR transaction entries must be direct SQL expressions or standalone mail_queue calls",
                    ));
                }
            }
        }
        if !self.storage_checked {
            zelyra_runtime::validate_database_storage_with_host(
                &database_url,
                &self.storage_schema,
                self.host_services.as_ref(),
                span,
            )?;
            self.storage_checked = true;
        }
        self.host_services
            .execute_database_batch(&database_url, &queries, true)
            .map_err(|error| {
                runtime_error(span, format!("database transaction failed: {error}"))
            })?;
        Ok(())
    }

    fn http_typed_request(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[ExpressionId],
        locals: &mut HashMap<LocalId, Value>,
        depth: usize,
        span: Span,
    ) -> Result<HostHttpResponse, RuntimeError> {
        if type_args.len() != 2 || args.len() != 4 {
            return Err(runtime_error(
                span,
                format!(
                    "{name} expects request and response type arguments, String method, String URL, String[] headers, and Request? body"
                ),
            ));
        }
        self.require_host_capability("Network", "network access", span)?;
        let method = self.expression(args[0], locals, depth, span)?;
        let url = self.expression(args[1], locals, depth, span)?;
        let headers = self.expression(args[2], locals, depth, span)?;
        let body = self.expression(args[3], locals, depth, span)?;
        let Value::String(method) = method else {
            return Err(runtime_error(
                span,
                format!("{name} expects a String method"),
            ));
        };
        let Value::String(url) = url else {
            return Err(runtime_error(span, format!("{name} expects a String URL")));
        };
        let Value::Array(headers) = headers else {
            return Err(runtime_error(
                span,
                format!("{name} expects String[] headers"),
            ));
        };
        let mut request_headers = Vec::with_capacity(headers.len());
        for header in headers {
            let Value::String(header) = header else {
                return Err(runtime_error(
                    span,
                    format!("{name} expects String[] headers"),
                ));
            };
            request_headers.push(header);
        }
        let encoded_body = match body {
            Value::Option(Some(body)) => Some(
                zelyra_runtime::encode_json_value(&body, span)
                    .map_err(|error| runtime_error(span, error.message))?,
            ),
            Value::Option(None) => None,
            _ => {
                return Err(runtime_error(
                    span,
                    format!("{name} expects an optional request value"),
                ));
            }
        };
        execute_http_request_with_host(
            &method,
            &url,
            &request_headers,
            encoded_body.as_deref(),
            self.network_policy.as_ref(),
            self.host_services.as_ref(),
        )
        .map_err(|error| runtime_error(span, error))
    }

    fn require_host_capability(
        &self,
        capability: &str,
        operation: &str,
        span: Span,
    ) -> Result<(), RuntimeError> {
        let declared = self
            .active_capabilities
            .last()
            .is_some_and(|capabilities| has_capability(capabilities, capability));
        if !declared {
            return Err(runtime_error(
                span,
                format!(
                    "runtime capability denied: {operation} requires `{capability}` in the current function"
                ),
            ));
        }
        if self
            .grants
            .as_ref()
            .is_some_and(|grants| !has_capability(grants, capability))
        {
            return Err(runtime_error(
                span,
                format!("runtime capability denied: `{capability}` is not granted"),
            ));
        }
        Ok(())
    }

    fn higher_order_builtin(
        &mut self,
        name: &str,
        args: &[ExpressionId],
        locals: &HashMap<LocalId, Value>,
        depth: usize,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let expected_count = if name == "reduce" { 3 } else { 2 };
        if args.len() != expected_count {
            return Err(runtime_error(
                span,
                format!("`{name}` expects {expected_count} arguments"),
            ));
        }
        let collection = self.expression(args[0], &mut locals.clone(), depth, span)?;
        let Value::Array(values) = collection else {
            return Err(runtime_error(
                span,
                format!("`{name}` expects an array, found `{}`", collection.ty()),
            ));
        };

        let (initial, callback_index) = if name == "reduce" {
            (
                Some(self.expression(args[1], &mut locals.clone(), depth, span)?),
                2,
            )
        } else {
            (None, 1)
        };
        let callback_id = args[callback_index];
        let callback_expression = self
            .program
            .expressions
            .get(callback_id.0)
            .ok_or_else(|| runtime_error(span, "MIR callback expression is missing"))?;
        let callback_span = callback_expression.span;
        let callback_kind = callback_expression.kind.clone();
        let callback = match callback_kind {
            MirExpressionKind::Lambda { params, body } => {
                let Type::Function {
                    return_type,
                    capabilities,
                    ..
                } = callback_expression.ty.clone()
                else {
                    return Err(runtime_error(
                        callback_span,
                        "MIR lambda callback does not have a function type",
                    ));
                };
                MirCallback::Lambda {
                    params,
                    body,
                    captures: capture_locals(self.program, body, locals),
                    return_type: *return_type,
                    capabilities,
                }
            }
            MirExpressionKind::FunctionValue {
                function: Some(function),
                ..
            } => MirCallback::Function(function),
            MirExpressionKind::Local(_) => {
                match self.expression(callback_id, &mut locals.clone(), depth, callback_span)? {
                    Value::Function(function_value) => {
                        if let Some(function) = self.functions.iter().find_map(|(id, function)| {
                            (function.name == function_value.name).then_some(*id)
                        }) {
                            MirCallback::Function(function)
                        } else {
                            self.closures
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner)
                                .get(&function_value.name)
                                .cloned()
                                .ok_or_else(|| {
                                    runtime_error(
                                        callback_span,
                                        format!(
                                            "MIR callback function `{}` was not found",
                                            function_value.name
                                        ),
                                    )
                                })?
                        }
                    }
                    other => {
                        return Err(runtime_error(
                            callback_span,
                            format!(
                                "`{name}` expects a function or lambda callback, found `{}`",
                                other.ty()
                            ),
                        ));
                    }
                }
            }
            _ => {
                return Err(runtime_error(
                    callback_span,
                    format!("`{name}` expects a function or lambda callback"),
                ));
            }
        };

        match name {
            "map" => values
                .into_iter()
                .map(|value| self.invoke_mir_callback(&callback, vec![value], depth, span))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            "filter" => {
                let mut selected = Vec::new();
                for value in values {
                    match self.invoke_mir_callback(&callback, vec![value.clone()], depth, span)? {
                        Value::Bool(true) => selected.push(value),
                        Value::Bool(false) => {}
                        other => {
                            return Err(runtime_error(
                                span,
                                format!("filter callback must return Bool, found `{}`", other.ty()),
                            ));
                        }
                    }
                }
                Ok(Value::Array(selected))
            }
            "find" => {
                for value in values {
                    match self.invoke_mir_callback(&callback, vec![value.clone()], depth, span)? {
                        Value::Bool(true) => return Ok(Value::Option(Some(Box::new(value)))),
                        Value::Bool(false) => {}
                        other => {
                            return Err(runtime_error(
                                span,
                                format!("find callback must return Bool, found `{}`", other.ty()),
                            ));
                        }
                    }
                }
                Ok(Value::Option(None))
            }
            "reduce" => {
                let mut accumulator = initial.expect("reduce accumulator was evaluated above");
                for value in values {
                    accumulator =
                        self.invoke_mir_callback(&callback, vec![accumulator, value], depth, span)?;
                }
                Ok(accumulator)
            }
            _ => unreachable!("higher-order builtin name was validated"),
        }
    }

    fn invoke_mir_callback(
        &mut self,
        callback: &MirCallback,
        arguments: Vec<Value>,
        depth: usize,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let callback_depth = depth + 1;
        if callback_depth > self.limits.max_stack_depth {
            return Err(runtime_error(
                span,
                format!(
                    "function call depth limit exceeded (maximum {})",
                    self.limits.max_stack_depth
                ),
            ));
        }
        match callback {
            MirCallback::Function(function) => {
                self.call(*function, arguments, callback_depth, span)
            }
            MirCallback::Lambda {
                params,
                body,
                captures,
                return_type,
                capabilities,
            } => {
                if params.len() != arguments.len() {
                    return Err(runtime_error(
                        span,
                        format!(
                            "callback expects {} argument(s), found {}",
                            params.len(),
                            arguments.len()
                        ),
                    ));
                }
                for ((_, name, expected), value) in params.iter().zip(&arguments) {
                    if !runtime_type_matches(expected, &value.ty()) {
                        return Err(runtime_error(
                            span,
                            format!(
                                "lambda parameter `{name}` expects `{expected}`, found `{}`",
                                value.ty()
                            ),
                        ));
                    }
                }
                if let Some(active) = self.active_capabilities.last() {
                    if let Some(missing) = capabilities
                        .iter()
                        .find(|capability| !has_capability(active, capability))
                    {
                        return Err(runtime_error(
                            span,
                            format!(
                                "runtime capability denied: invoking lambda requires `{missing}` in the caller"
                            ),
                        ));
                    }
                }
                let mut locals = captures.clone();
                locals.extend(
                    params
                        .iter()
                        .zip(arguments)
                        .map(|((local, _, _), value)| (*local, value)),
                );
                let value = self.expression(*body, &mut locals, callback_depth, span)?;
                if !runtime_type_matches(return_type, &value.ty()) {
                    return Err(runtime_error(
                        span,
                        format!("lambda returns `{}`, expected `{return_type}`", value.ty()),
                    ));
                }
                Ok(value)
            }
        }
    }
}

fn has_capability(capabilities: &HashSet<String>, required: &str) -> bool {
    capabilities.contains(required)
        || (required.starts_with("Database(") && capabilities.contains("Database"))
        || (required == "Database"
            && capabilities.contains("Database(read)")
            && capabilities.contains("Database(write)"))
}

pub(crate) fn runtime_type_matches(expected: &Type, actual: &Type) -> bool {
    if expected == actual || matches!(expected, Type::Unknown) || matches!(actual, Type::Unknown) {
        return true;
    }
    match (expected, actual) {
        (Type::Option(expected), Type::Option(actual))
        | (Type::Array(expected), Type::Array(actual))
        | (Type::Set(expected), Type::Set(actual)) => runtime_type_matches(expected, actual),
        (Type::Result(expected_ok, expected_err), Type::Result(actual_ok, actual_err)) => {
            runtime_type_matches(expected_ok, actual_ok)
                && runtime_type_matches(expected_err, actual_err)
        }
        (Type::Tuple(expected), Type::Tuple(actual)) => {
            expected.len() == actual.len()
                && expected
                    .iter()
                    .zip(actual)
                    .all(|(expected, actual)| runtime_type_matches(expected, actual))
        }
        (Type::Map(expected_key, expected_value), Type::Map(actual_key, actual_value)) => {
            runtime_type_matches(expected_key, actual_key)
                && runtime_type_matches(expected_value, actual_value)
        }
        (
            Type::Applied {
                name: expected_name,
                args: expected_args,
            },
            Type::Applied {
                name: actual_name,
                args: actual_args,
            },
        ) => {
            expected_name == actual_name
                && expected_args.len() == actual_args.len()
                && expected_args
                    .iter()
                    .zip(actual_args)
                    .all(|(expected, actual)| runtime_type_matches(expected, actual))
        }
        (Type::HttpResult(_), Type::Named(actual)) if actual == "HttpResult" => true,
        _ => false,
    }
}

fn runtime_enum_type(expression_type: &Type, fallback: &str) -> (String, Vec<Type>) {
    match expression_type {
        Type::Named(name) => (name.clone(), Vec::new()),
        Type::Applied { name, args } => (name.clone(), args.clone()),
        _ => (fallback.into(), Vec::new()),
    }
}

pub(crate) fn match_pattern(
    pattern: &crate::MirPattern,
    value: &Value,
    bindings: &mut HashMap<LocalId, Value>,
) -> bool {
    use crate::MirPatternKind;
    match &pattern.kind {
        MirPatternKind::Wildcard => true,
        MirPatternKind::Binding { local, .. } => {
            bindings.insert(*local, value.clone());
            true
        }
        MirPatternKind::Int(expected) => matches!(value, Value::Int(actual) if actual == expected),
        MirPatternKind::Bool(expected) => {
            matches!(value, Value::Bool(actual) if actual == expected)
        }
        MirPatternKind::String(expected) => {
            matches!(value, Value::String(actual) if actual == expected)
        }
        MirPatternKind::Char(expected) => {
            matches!(value, Value::Char(actual) if actual == expected)
        }
        MirPatternKind::Constructor { name, inner } => {
            let payload = match (name.as_str(), value) {
                ("Some", Value::Option(Some(value))) => Some(value.as_ref()),
                ("None", Value::Option(None)) => return inner.is_none(),
                ("Ok", Value::Result(Ok(value))) => Some(value.as_ref()),
                ("Err", Value::Result(Err(value))) => Some(value.as_ref()),
                _ => None,
            };
            match (inner, payload) {
                (None, None) => true,
                (Some(inner), Some(payload)) => match_pattern(inner, payload, bindings),
                _ => false,
            }
        }
        MirPatternKind::EnumVariant { enum_name, variant } => matches!(
            value,
            Value::Enum {
                type_name,
                variant: actual_variant,
                payloads,
                ..
            } if type_name == enum_name && actual_variant == variant && payloads.is_empty()
        ),
        MirPatternKind::EnumVariantPayload {
            enum_name,
            variant,
            payloads,
        } => {
            let Value::Enum {
                type_name,
                variant: actual_variant,
                payloads: actual_payloads,
                ..
            } = value
            else {
                return false;
            };
            type_name == enum_name
                && actual_variant == variant
                && actual_payloads.len() == payloads.len()
                && payloads
                    .iter()
                    .zip(actual_payloads)
                    .all(|(pattern, value)| match_pattern(pattern, value, bindings))
        }
        MirPatternKind::Record { type_name, fields } => {
            let Value::Object {
                type_name: actual_type,
                fields: actual_fields,
                ..
            } = value
            else {
                return false;
            };
            actual_type == type_name
                && fields.iter().all(|(name, pattern)| {
                    actual_fields
                        .get(name)
                        .is_some_and(|value| match_pattern(pattern, value, bindings))
                })
        }
    }
}

pub(crate) fn is_pure_builtin(name: &str) -> bool {
    zelyra_runtime::is_pure_builtin(name)
}

pub(crate) fn execute_pure_builtin(
    name: &str,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, RuntimeError> {
    zelyra_runtime::execute_pure_builtin(name, arguments, span)
}

pub(crate) fn array_assignment_index(
    index: Value,
    length: usize,
    span: Span,
) -> Result<usize, RuntimeError> {
    let index = match index {
        Value::Int(index) => usize::try_from(index)
            .map_err(|_| runtime_error(span, "array index must not be negative"))?,
        Value::UInt(index) => {
            usize::try_from(index).map_err(|_| runtime_error(span, "array index is too large"))?
        }
        other => {
            return Err(runtime_error(
                span,
                format!("array index must be Int or UInt, got `{}`", other.ty()),
            ));
        }
    };
    if index >= length {
        return Err(runtime_error(
            span,
            format!("array index {index} is out of bounds"),
        ));
    }
    Ok(index)
}

pub(crate) fn index_value(target: Value, index: Value, span: Span) -> Result<Value, RuntimeError> {
    match (target, index) {
        (Value::Array(items), Value::Int(index)) | (Value::Tuple(items), Value::Int(index)) => {
            let index = usize::try_from(index)
                .map_err(|_| runtime_error(span, "collection index must not be negative"))?;
            items.get(index).cloned().ok_or_else(|| {
                runtime_error(span, format!("collection index {index} is out of bounds"))
            })
        }
        (Value::Array(items), Value::UInt(index)) | (Value::Tuple(items), Value::UInt(index)) => {
            let index = usize::try_from(index)
                .map_err(|_| runtime_error(span, "collection index is too large"))?;
            items.get(index).cloned().ok_or_else(|| {
                runtime_error(span, format!("collection index {index} is out of bounds"))
            })
        }
        (Value::String(value), Value::Int(index)) => {
            let index = usize::try_from(index)
                .map_err(|_| runtime_error(span, "string index must not be negative"))?;
            value.chars().nth(index).map(Value::Char).ok_or_else(|| {
                runtime_error(span, format!("string index {index} is out of bounds"))
            })
        }
        (Value::Map(entries), key) => entries
            .into_iter()
            .find_map(|(candidate, value)| (candidate == key).then_some(value))
            .ok_or_else(|| runtime_error(span, "map key was not found")),
        (Value::Json(value), Value::String(key)) => value
            .get(&key)
            .cloned()
            .map(Value::Json)
            .ok_or_else(|| runtime_error(span, format!("JSON key `{key}` was not found"))),
        (Value::Json(value), Value::Int(index)) => {
            let index = usize::try_from(index)
                .map_err(|_| runtime_error(span, "JSON array index must not be negative"))?;
            value.get(index).cloned().map(Value::Json).ok_or_else(|| {
                runtime_error(span, format!("JSON array index {index} is out of bounds"))
            })
        }
        (target, index) => Err(runtime_error(
            span,
            format!("cannot index `{}` with `{}`", target.ty(), index.ty()),
        )),
    }
}

pub(crate) fn evaluate_binary(
    left: Value,
    op: BinaryOp,
    right: Value,
    span: Span,
) -> Result<Value, RuntimeError> {
    zelyra_runtime::evaluate_binary_value(left, op, right, span)
}

pub(crate) fn evaluate_unary(
    operator: UnaryOp,
    value: Value,
    span: Span,
) -> Result<Value, RuntimeError> {
    zelyra_runtime::evaluate_unary_value(operator, value, span)
}

fn expression_name(expression: &MirExpressionKind) -> &'static str {
    match expression {
        MirExpressionKind::Uninitialized => "uninitialized",
        MirExpressionKind::Int(_) => "Int literal",
        MirExpressionKind::UInt(_) => "UInt literal",
        MirExpressionKind::Float(_) => "Float literal",
        MirExpressionKind::Bool(_) => "Bool literal",
        MirExpressionKind::String(_) => "String literal",
        MirExpressionKind::Char(_) => "Char literal",
        MirExpressionKind::Array(_) => "array",
        MirExpressionKind::Map(_) => "map",
        MirExpressionKind::Record { .. } => "record",
        MirExpressionKind::Lambda { .. } => "lambda",
        MirExpressionKind::Local(_) => "local",
        MirExpressionKind::EnumVariant { .. } => "enum variant",
        MirExpressionKind::FunctionValue { .. } => "function value",
        MirExpressionKind::Index { .. } => "index",
        MirExpressionKind::Field { .. } => "field",
        MirExpressionKind::Call { .. } => "call",
        MirExpressionKind::Unary { .. } => "unary",
        MirExpressionKind::Binary { .. } => "binary",
        MirExpressionKind::BinaryChain { .. } => "binary chain",
        MirExpressionKind::Sql { .. } => "SQL",
        MirExpressionKind::Await(_) => "await",
        MirExpressionKind::Match { .. } => "match expression",
    }
}

pub(crate) fn runtime_error(span: Span, message: impl Into<String>) -> RuntimeError {
    RuntimeError {
        message: message.into(),
        span,
    }
}

fn http_response_value(response: HostHttpResponse, span: Span) -> Result<Value, RuntimeError> {
    let body = String::from_utf8(response.body)
        .map_err(|_| runtime_error(span, "network response is not valid UTF-8"))?;
    let headers = response
        .headers
        .into_iter()
        .map(|(name, value)| Value::String(format!("{name}: {value}")))
        .collect();
    Ok(Value::Object {
        type_name: "HttpResponse".into(),
        type_args: Vec::new(),
        fields: HashMap::from([
            ("status".into(), Value::Int(i64::from(response.status))),
            ("headers".into(), Value::Array(headers)),
            ("body".into(), Value::String(body)),
        ]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode;
    use crate::{lower, optimize_scalar_constants, MirExpressionKind};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use zelyra_lexer::lex;
    use zelyra_parser::parse;
    use zelyra_runtime::MAX_CONFIGURED_FUNCTION_CALL_DEPTH;

    #[test]
    fn execution_limits_cannot_raise_native_call_depth_above_the_shared_cap() {
        let limits = bounded_execution_limits(ExecutionLimits {
            max_steps: 17,
            max_stack_depth: usize::MAX,
        });
        assert_eq!(limits.max_steps, 17);
        assert_eq!(limits.max_stack_depth, MAX_CONFIGURED_FUNCTION_CALL_DEPTH);
        assert_eq!(
            ExecutionLimits::default().max_stack_depth,
            effective_function_call_depth(None)
        );
    }

    #[test]
    fn mir_and_bytecode_enforce_the_shared_call_depth_cap_for_embedding_limits() {
        let source = "fn dive(depth: Int) -> Int { if depth == 0 { return 0 } return dive(depth - 1) + 1 } fn main() -> Int { return dive(600) }";
        let (_, mir) = program(source);
        let limits = ExecutionLimits {
            max_steps: 100_000,
            max_stack_depth: usize::MAX,
        };
        let mir_error = execute_function(&mir, "main", vec![], limits)
            .expect_err("MIR enforces a hard stack bound for embedding callers");
        assert!(mir_error
            .message
            .contains("MIR maximum call stack depth exceeded"));

        let bytecode = crate::bytecode::compile(&mir, "main")
            .expect("recursive program compiles to the supported bytecode slice");
        let bytecode_error = crate::bytecode::execute(&bytecode, vec![], limits)
            .expect_err("bytecode applies the same configured call-depth ceiling");
        assert!(bytecode_error
            .message
            .contains("bytecode maximum call stack depth exceeded"));
    }

    #[test]
    fn scalar_constant_optimization_preserves_ast_results_errors_and_effects() {
        let source = r#"
fn calculate() -> Int { return (2 + 3) * 4 }
fn add() -> Int { return 2 + 3 }
fn negate() -> Int { return -(2 + 3) }
fn invert() -> Bool { return !false }
fn divide_by_zero() -> Int { return 1 / 0 }
fn effect() -> Int { print("effect ran"); return 3 }
fn with_effect() -> Int { return 4 + effect() }
fn main() {}
"#;
        let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
        let mut optimized = lower(&ast).expect("source type-checks and lowers");
        let unoptimized = optimized.clone();

        let report =
            optimize_scalar_constants(&mut optimized).expect("optimization preserves verified MIR");
        assert!(report.folded_scalar_expressions >= 3);
        assert!(optimized
            .expressions
            .iter()
            .any(|expression| matches!(expression.kind, MirExpressionKind::Call { ref name, .. } if name == "effect")));

        let expected = zelyra_runtime::execute_function(&ast, "calculate", vec![], None).unwrap();
        let actual =
            execute_function(&optimized, "calculate", vec![], ExecutionLimits::default()).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(20));
        let unoptimized_result = execute_function(
            &unoptimized,
            "calculate",
            vec![],
            ExecutionLimits::default(),
        )
        .unwrap();
        assert!(actual.steps < unoptimized_result.steps);

        for (name, expected_value) in [
            ("add", Value::Int(5)),
            ("negate", Value::Int(-5)),
            ("invert", Value::Bool(true)),
        ] {
            let expected = zelyra_runtime::execute_function(&ast, name, vec![], None).unwrap();
            let actual =
                execute_function(&optimized, name, vec![], ExecutionLimits::default()).unwrap();
            assert_eq!(expected, expected_value, "AST {name}");
            assert_eq!(actual.value, expected_value, "optimized MIR {name}");
        }

        let expected_error =
            zelyra_runtime::execute_function(&ast, "divide_by_zero", vec![], None).unwrap_err();
        let actual_error = execute_function(
            &optimized,
            "divide_by_zero",
            vec![],
            ExecutionLimits::default(),
        )
        .unwrap_err();
        assert_eq!(actual_error.message, expected_error.message);
        assert_eq!(actual_error.span, expected_error.span);

        let expected_effect =
            zelyra_runtime::execute_function_captured_with_capabilities_and_policies(
                &ast,
                "with_effect",
                vec![],
                None,
                None,
                None,
            );
        assert_eq!(expected_effect.error, None);
        let actual_effect = execute_function_captured_with_host_services_and_policy(
            &optimized,
            "with_effect",
            vec![],
            ExecutionLimits::default(),
            None,
            None,
            Arc::new(SystemHostServices),
        );
        assert_eq!(actual_effect.error, None);
        assert_eq!(actual_effect.output, expected_effect.output);
        assert_eq!(actual_effect.output, vec!["effect ran"]);
        assert_eq!(actual_effect.value, expected_effect.value);
        assert_eq!(actual_effect.value, Some(Value::Int(7)));
    }

    #[test]
    fn loop_invariants_match_ast_for_normal_iterations_and_failures() {
        let source = "fn count() -> Int { mutable value: Int = 0 while value < 1 invariant { value >= 0 } { value = value + 1 } return value } fn main() {}";
        let ast = parse(&lex(source).expect("source lexes")).expect("source parses");
        let mir = lower(&ast).expect("source type-checks and lowers");
        let expected = zelyra_runtime::execute_function(&ast, "count", vec![], None).unwrap();
        let actual = execute_function(&mir, "count", vec![], ExecutionLimits::default()).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(1));

        for (name, source) in [
            (
                "break_bad",
                "fn break_bad() -> Int { mutable value: Int = 0 loop invariant { value == 0 } { value = 1; break } return value } fn main() {}",
            ),
            (
                "return_bad",
                "fn return_bad() -> Int { mutable value: Int = 0 loop invariant { value == 0 } { value = 1; return value } return 0 } fn main() {}",
            ),
            (
                "continue_bad",
                "fn continue_bad() -> Int { mutable value: Int = 0 while value < 2 invariant { value < 1 } { value = value + 1; continue } return value } fn main() {}",
            ),
            (
                "nested_return_bad",
                "fn nested_return_bad() -> Int { mutable value: Int = 0 loop invariant { value == 0 } { loop invariant { value == 0 } { value = 1; return value } } return 0 } fn main() {}",
            ),
        ] {
            let ast = parse(&lex(source).unwrap()).unwrap();
            let mir = lower(&ast).unwrap();
            let ast_error = zelyra_runtime::execute_function(&ast, name, vec![], None).unwrap_err();
            let mir_error = execute_function(&mir, name, vec![], ExecutionLimits::default())
                .expect_err("violated loop invariant must fail in MIR");
            assert_eq!(mir_error.message, ast_error.message, "{name}");
            assert_eq!(mir_error.span, ast_error.span, "{name}");
        }
    }

    struct ClockHost {
        timestamp: i64,
        environment: Option<String>,
        calls: AtomicUsize,
    }

    struct ConsoleHost {
        line: Option<String>,
        prompt: Mutex<Option<String>>,
        calls: AtomicUsize,
    }

    type ProcessInvocation = (String, Vec<String>, u64, usize);
    type HttpRequest = (
        String,
        String,
        Vec<(String, String)>,
        Option<Vec<u8>>,
        u64,
        usize,
    );
    type DatabaseQueryCall = (String, String, Vec<(String, zelyra_database::QueryValue)>);

    struct ProcessHost {
        invocation: Mutex<Option<ProcessInvocation>>,
        calls: AtomicUsize,
    }

    struct HttpHost {
        request: Mutex<Option<HttpRequest>>,
        response: HostHttpResponse,
        calls: AtomicUsize,
    }

    struct ConcurrentHttpHost {
        arrivals: std::sync::Mutex<usize>,
        ready: std::sync::Condvar,
        calls: AtomicUsize,
    }

    struct MailHost {
        delivery: Mutex<Option<(String, String, String)>>,
        calls: AtomicUsize,
    }

    struct DatabaseHost {
        query: Mutex<Option<DatabaseQueryCall>>,
        result: zelyra_database::QueryResult,
        validations: AtomicUsize,
        queries: AtomicUsize,
    }

    struct TransactionHost {
        batches: Mutex<Vec<(String, Vec<zelyra_database::Query>, bool)>>,
        validations: AtomicUsize,
        failure: Option<String>,
    }

    impl HostServices for ConsoleHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(1_700_000_000_000)
        }

        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }

        fn read_console_line(&self, prompt: &str) -> Result<Option<String>, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            *self.prompt.lock().unwrap() = Some(prompt.to_owned());
            Ok(self.line.clone())
        }
    }

    impl HostServices for ProcessHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(1_700_000_000_000)
        }

        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }

        fn run_process(
            &self,
            command: &str,
            arguments: &[String],
            timeout_ms: u64,
            max_output_bytes: usize,
        ) -> Result<String, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            *self.invocation.lock().unwrap() = Some((
                command.to_owned(),
                arguments.to_vec(),
                timeout_ms,
                max_output_bytes,
            ));
            Ok("safe output".into())
        }
    }

    impl HostServices for HttpHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(1_700_000_000_000)
        }

        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }

        fn http_request(
            &self,
            method: &str,
            url: &str,
            headers: &[(String, String)],
            body: Option<&[u8]>,
            timeout_ms: u64,
            max_response_bytes: usize,
        ) -> Result<HostHttpResponse, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            *self.request.lock().unwrap() = Some((
                method.to_owned(),
                url.to_owned(),
                headers.to_vec(),
                body.map(<[u8]>::to_vec),
                timeout_ms,
                max_response_bytes,
            ));
            Ok(self.response.clone())
        }
    }

    impl HostServices for ConcurrentHttpHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(1_700_000_000_000)
        }

        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }

        fn http_request(
            &self,
            _method: &str,
            _url: &str,
            _headers: &[(String, String)],
            _body: Option<&[u8]>,
            _timeout_ms: u64,
            _max_response_bytes: usize,
        ) -> Result<HostHttpResponse, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let mut arrivals = self.arrivals.lock().map_err(|error| error.to_string())?;
            *arrivals += 1;
            if *arrivals >= 2 {
                self.ready.notify_all();
            } else {
                let (next, timeout) = self
                    .ready
                    .wait_timeout_while(arrivals, std::time::Duration::from_secs(2), |count| {
                        *count < 2
                    })
                    .map_err(|error| error.to_string())?;
                arrivals = next;
                if timeout.timed_out() && *arrivals < 2 {
                    return Err("parallel HTTP requests did not overlap".into());
                }
            }
            Ok(HostHttpResponse {
                status: 200,
                headers: Vec::new(),
                body: b"ok".to_vec(),
            })
        }
    }

    impl HostServices for MailHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(1_700_000_000_000)
        }

        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }

        fn deliver_outbox_message(
            &self,
            event_id: &str,
            payload_json: &str,
            mail: &MailPolicy,
            network: &NetworkPolicy,
        ) -> Result<(), String> {
            let host = mail
                .host
                .as_deref()
                .ok_or_else(|| "missing test SMTP host".to_string())?;
            if !network.allowed_hosts.iter().any(|allowed| allowed == host) {
                return Err("test SMTP host is not allowed".into());
            }
            self.calls.fetch_add(1, Ordering::SeqCst);
            *self.delivery.lock().unwrap() = Some((
                event_id.to_owned(),
                payload_json.to_owned(),
                host.to_owned(),
            ));
            Ok(())
        }
    }

    impl HostServices for DatabaseHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(1_700_000_000_000)
        }

        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }

        fn validate_database_storage(
            &self,
            _database_url: &str,
            _schema: &zelyra_database::Schema,
        ) -> Result<(), String> {
            self.validations.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn execute_database_query(
            &self,
            database_url: &str,
            query: &zelyra_database::Query,
        ) -> Result<zelyra_database::QueryResult, String> {
            self.queries.fetch_add(1, Ordering::SeqCst);
            *self.query.lock().unwrap() = Some((
                database_url.to_owned(),
                query.sql.clone(),
                query.params.clone(),
            ));
            Ok(self.result.clone())
        }
    }

    impl HostServices for TransactionHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            Ok(1_700_000_000_000)
        }

        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }

        fn validate_database_storage(
            &self,
            _database_url: &str,
            _schema: &zelyra_database::Schema,
        ) -> Result<(), String> {
            self.validations.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn execute_database_batch(
            &self,
            database_url: &str,
            queries: &[zelyra_database::Query],
            transaction: bool,
        ) -> Result<Vec<zelyra_database::QueryResult>, String> {
            self.batches.lock().unwrap().push((
                database_url.to_owned(),
                queries.to_vec(),
                transaction,
            ));
            if let Some(error) = &self.failure {
                return Err(error.clone());
            }
            Ok(vec![zelyra_database::QueryResult::default(); queries.len()])
        }
    }

    impl HostServices for ClockHost {
        fn unix_time_millis(&self) -> Result<i64, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.timestamp)
        }

        fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(self.environment.clone())
        }

        fn fill_random(&self, output: &mut [u8]) -> Result<(), String> {
            output.fill(0);
            Ok(())
        }
    }

    fn program(source: &str) -> (zelyra_ast::Program, MirProgram) {
        let source = parse(&lex(source).unwrap()).unwrap();
        let mir = lower(&source).unwrap();
        (source, mir)
    }

    #[test]
    fn function_preconditions_and_postconditions_match_ast_execution() {
        let source = "fn increment(value: Int) -> Int requires { value >= 0 } ensures { result > value } { return value + 1 } fn broken(value: Int) -> Int ensures { result > value } { return value } fn main() {}";
        let (ast, mir) = program(source);

        let actual = execute_function(
            &mir,
            "increment",
            vec![Value::Int(4)],
            ExecutionLimits::default(),
        )
        .unwrap();
        let expected =
            zelyra_runtime::execute_function(&ast, "increment", vec![Value::Int(4)], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(5));

        let mir_precondition = execute_function(
            &mir,
            "increment",
            vec![Value::Int(-1)],
            ExecutionLimits::default(),
        )
        .unwrap_err();
        let ast_precondition =
            zelyra_runtime::execute_function(&ast, "increment", vec![Value::Int(-1)], None)
                .unwrap_err();
        assert!(mir_precondition.message.contains("precondition failed"));
        assert_eq!(mir_precondition.message, ast_precondition.message);

        let mir_postcondition = execute_function(
            &mir,
            "broken",
            vec![Value::Int(4)],
            ExecutionLimits::default(),
        )
        .unwrap_err();
        let ast_postcondition =
            zelyra_runtime::execute_function(&ast, "broken", vec![Value::Int(4)], None)
                .unwrap_err();
        assert!(mir_postcondition.message.contains("postcondition failed"));
        assert_eq!(mir_postcondition.message, ast_postcondition.message);
    }

    #[test]
    fn executes_checked_functions_and_matches_the_ast_reference() {
        let (ast, mir) = program(
            "fn add(left: Int, right: Int) -> Int { return left + right } fn main() -> Int { total = add(2, 3) if total > 4 { return total } else { return 0 } }",
        );
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(5));
        assert!(actual.steps > 0);
    }

    #[test]
    fn indirect_named_function_calls_match_ast_execution_and_preserve_effect_checks() {
        let source = r#"
            fn increment(value: Int) -> Int { return value + 1 }
            fn apply(callback: Function<(Int) -> Int>, value: Int) -> Int {
                return callback(value)
            }
            fn main() -> Int {
                incrementer = &increment
                return apply(incrementer, 41)
            }
        "#;
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(42));
        let bytecode = crate::bytecode::compile(&mir, "main").unwrap();
        let bytecode =
            crate::bytecode::decode(&crate::bytecode::encode(&bytecode).unwrap()).unwrap();
        let bytecode_result =
            crate::bytecode::execute(&bytecode, vec![], ExecutionLimits::default()).unwrap();
        assert_eq!(bytecode_result.value, actual.value);

        let mut invalid_target = mir.clone();
        let indirect_call = invalid_target
            .expressions
            .iter_mut()
            .find_map(|expression| match &mut expression.kind {
                MirExpressionKind::Call {
                    callee_local: Some(local),
                    ..
                } => Some(local),
                _ => None,
            })
            .expect("indirect call retains its resolved local");
        *indirect_call = LocalId(usize::MAX);
        let errors = super::verify(&invalid_target)
            .expect_err("verifier must reject unresolved indirect-call locals");
        assert!(errors.iter().any(|error| error.code == "E-MIR-VERIFY-001"));

        let mut invalid_argument = mir.clone();
        let argument_id = invalid_argument
            .expressions
            .iter()
            .find_map(|expression| match &expression.kind {
                MirExpressionKind::Call {
                    callee_local: Some(_),
                    args,
                    ..
                } => args.first().copied(),
                _ => None,
            })
            .expect("indirect call has one argument");
        invalid_argument.expressions[argument_id.0].ty = Type::String;
        let errors = super::verify(&invalid_argument)
            .expect_err("verifier must reject indirect argument type mismatches");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-002" && error.message.contains("expected `Int`")
        }));

        let effectful = r#"
            fn read_clock() -> Timestamp uses Clock { return now() }
            fn invoke(callback: Function<() -> Timestamp> uses {Clock}) -> Timestamp uses Clock {
                return callback()
            }
            fn main() -> Timestamp uses Clock {
                reader = &read_clock
                return invoke(reader)
            }
        "#;
        let (effect_ast, effect_mir) = program(effectful);
        let host = Arc::new(ClockHost {
            timestamp: 1_700_000_000_000,
            environment: None,
            calls: AtomicUsize::new(0),
        });
        let grants = HashSet::from(["Clock".to_owned()]);
        let actual = execute_function_with_host_services(
            &effect_mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            Some(&grants),
            host.clone(),
        )
        .unwrap();
        let expected =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &effect_ast,
                "main",
                vec![],
                None,
                Some(&grants),
                None,
                Arc::new(ClockHost {
                    timestamp: host.timestamp,
                    environment: None,
                    calls: AtomicUsize::new(0),
                }),
            )
            .unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Timestamp(host.timestamp));
        assert_eq!(host.calls.load(Ordering::SeqCst), 1);

        let effect_bytecode = crate::bytecode::compile(&effect_mir, "main").unwrap();
        let effect_bytecode =
            crate::bytecode::decode(&crate::bytecode::encode(&effect_bytecode).unwrap()).unwrap();
        let bytecode_host = Arc::new(ClockHost {
            timestamp: host.timestamp,
            environment: None,
            calls: AtomicUsize::new(0),
        });
        let bytecode_result = crate::bytecode::execute_with_host_services(
            &effect_bytecode,
            vec![],
            ExecutionLimits::default(),
            Some(&grants),
            bytecode_host.clone(),
        )
        .unwrap();
        assert_eq!(bytecode_result.value, actual.value);
        assert_eq!(bytecode_host.calls.load(Ordering::SeqCst), 1);

        let denied_bytecode_host = Arc::new(ClockHost {
            timestamp: host.timestamp,
            environment: None,
            calls: AtomicUsize::new(0),
        });
        let denied = crate::bytecode::execute_with_host_services(
            &effect_bytecode,
            vec![],
            ExecutionLimits::default(),
            Some(&HashSet::new()),
            denied_bytecode_host.clone(),
        )
        .expect_err("indirect Clock call must require an explicit project grant");
        assert!(denied.message.contains("Clock"));
        assert_eq!(denied_bytecode_host.calls.load(Ordering::SeqCst), 0);

        let mut effect_mir = effect_mir;
        let invoke = effect_mir
            .functions
            .iter_mut()
            .find(|function| function.name == "invoke")
            .unwrap();
        invoke.declared_capabilities.clear();
        let errors = super::verify(&effect_mir)
            .expect_err("verifier must reject an indirect capability escalation");
        assert!(errors
            .iter()
            .any(|error| { error.code == "E-MIR-EFFECT-001" && error.message.contains("Clock") }));
    }

    #[test]
    fn stored_capturing_lambdas_execute_and_match_ast_reference() {
        let source = r#"
            fn make_adder(base: Int) -> Function<(Int) -> Int> {
                return |value| value + base
            }
            fn invoke(callback: Function<(Int) -> Int>, value: Int) -> Int {
                return callback(value)
            }
            fn main() -> Int {
                add = make_adder(40)
                return invoke(add, 2) + invoke(add, 3)
            }
        "#;
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default())
            .expect("MIR invokes the stored capturing lambda");
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None)
            .expect("AST invokes the same stored capturing lambda");
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(85));

        let mut malformed_lambda = mir.clone();
        let lambda = malformed_lambda
            .expressions
            .iter_mut()
            .find(|expression| matches!(expression.kind, MirExpressionKind::Lambda { .. }))
            .expect("factory contains one lambda");
        let Type::Function { return_type, .. } = &mut lambda.ty else {
            panic!("lambda has a function type");
        };
        **return_type = Type::String;
        let errors = super::verify(&malformed_lambda)
            .expect_err("MIR verifier must reject a lambda signature/body mismatch");
        assert!(errors.iter().any(|error| {
            error.code == "E-MIR-VERIFY-002" && error.message.contains("lambda body")
        }));
    }

    #[test]
    fn stored_lambda_calls_keep_declared_capabilities_and_runtime_grants() {
        let source = r#"
            fn main() -> Timestamp uses Clock {
                read_clock = |unused: Int| now()
                return read_clock(0)
            }
        "#;
        let (ast, mir) = program(source);
        let host = Arc::new(ClockHost {
            timestamp: 1_700_000_000_000,
            environment: None,
            calls: AtomicUsize::new(0),
        });
        let grants = HashSet::from(["Clock".to_owned()]);
        let actual = execute_function_with_host_services(
            &mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            Some(&grants),
            host.clone(),
        )
        .unwrap();
        let expected =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                "main",
                vec![],
                None,
                Some(&grants),
                None,
                Arc::new(ClockHost {
                    timestamp: host.timestamp,
                    environment: None,
                    calls: AtomicUsize::new(0),
                }),
            )
            .unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Timestamp(host.timestamp));
        assert_eq!(host.calls.load(Ordering::SeqCst), 1);

        let denied_host = Arc::new(ClockHost {
            timestamp: host.timestamp,
            environment: None,
            calls: AtomicUsize::new(0),
        });
        let denied = execute_function_with_host_services(
            &mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            Some(&HashSet::new()),
            denied_host.clone(),
        )
        .unwrap_err();
        assert!(denied.message.contains("Clock"));
        assert_eq!(denied_host.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn parallel_await_bindings_execute_and_match_the_ast_reference() {
        let source = r#"
            fn add_one(value: Int) -> Int { return value + 1 }
            fn double(value: Int) -> Int { return value * 2 }
            fn main() -> Int {
                mutable existing = 10
                parallel {
                    first = await add_one(existing)
                    existing = await double(existing)
                }
                return first + existing
            }
        "#;
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(31));
    }

    #[test]
    fn parallel_await_branches_share_the_step_budget() {
        let source = r#"
            fn spend() -> Int {
                mutable value = 0
                while value < 100 { value = value + 1 }
                return value
            }
            fn main() {
                parallel {
                    left = await spend()
                    right = await spend()
                }
            }
        "#;
        let (_, mir) = program(source);
        let error = execute_function(
            &mir,
            "main",
            vec![],
            ExecutionLimits {
                max_steps: 30,
                max_stack_depth: 20,
            },
        )
        .expect_err("all parallel branches share one bounded step counter");
        assert!(error.message.contains("step budget exceeded"));
    }

    #[test]
    fn parallel_await_branches_keep_network_capability_and_host_policy_checks() {
        let source = r#"
            fn main() -> String uses Network {
                parallel {
                    first = await http_get("https://api.example.test/first")
                    second = await http_get("https://api.example.test/second")
                }
                return first + second
            }
        "#;
        let (_, mir) = program(source);
        let policy = RuntimePolicy {
            network: Some(NetworkPolicy {
                allowed_hosts: vec!["api.example.test".into()],
                timeout_ms: 1000,
                max_response_bytes: 1024,
            }),
            ..RuntimePolicy::default()
        };
        let make_host = || {
            Arc::new(HttpHost {
                request: Mutex::new(None),
                response: HostHttpResponse {
                    status: 200,
                    headers: vec![],
                    body: b"ok".to_vec(),
                },
                calls: AtomicUsize::new(0),
            })
        };
        let allowed_host = make_host();
        let allowed = execute_function_with_host_services_and_policy(
            &mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            Some(&HashSet::from(["Network".into()])),
            Some(&policy),
            allowed_host.clone(),
        )
        .unwrap();
        assert_eq!(allowed.value, Value::String("okok".into()));
        assert_eq!(allowed_host.calls.load(Ordering::SeqCst), 2);

        let denied_host = make_host();
        let denied = execute_function_with_host_services_and_policy(
            &mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            Some(&HashSet::new()),
            Some(&policy),
            denied_host.clone(),
        )
        .unwrap_err();
        assert!(denied.message.contains("`Network` is not granted"));
        assert_eq!(denied_host.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn parallel_await_branches_overlap_host_requests() {
        let source = r#"
            fn main() -> String uses Network {
                parallel {
                    first = await http_get("https://api.example.test/first")
                    second = await http_get("https://api.example.test/second")
                }
                return first + second
            }
        "#;
        let (_, mir) = program(source);
        let host = Arc::new(ConcurrentHttpHost {
            arrivals: std::sync::Mutex::new(0),
            ready: std::sync::Condvar::new(),
            calls: AtomicUsize::new(0),
        });
        let result = execute_function_with_host_services_and_policy(
            &mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            Some(&HashSet::from(["Network".into()])),
            Some(&RuntimePolicy {
                network: Some(NetworkPolicy {
                    allowed_hosts: vec!["api.example.test".into()],
                    timeout_ms: 1000,
                    max_response_bytes: 1024,
                }),
                ..RuntimePolicy::default()
            }),
            host.clone(),
        )
        .unwrap();
        assert_eq!(result.value, Value::String("okok".into()));
        assert_eq!(host.calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn executes_record_fields_and_array_indexing_differentially() {
        let source = "struct Customer { name: String age: Int } fn main() -> String { customer = Customer { name: \"Ada\", age: 37 } names = [\"Ada\", \"Lin\"] return customer.name + names[1] }";
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::String("AdaLin".into()));
    }

    #[test]
    fn executes_tuple_returns_and_indexing_differentially() {
        let source = "fn coordinates() -> Tuple<Int, String> { return (52, \"Berlin\") } fn main() -> String { point = coordinates() return point[1] }";
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::String("Berlin".into()));
    }

    #[test]
    fn executes_tuple_destructuring_assignments_differentially() {
        let source = "fn coordinates() -> Tuple<Int, Int> { return (4, 9) } fn main() -> Int { mutable left = 0 mutable right = 0 (left, right) = coordinates() return left + right }";
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(13));
    }

    #[test]
    fn executes_option_and_result_constructors_differentially() {
        let source = "fn option_value(value: Option<Int>) -> Int { return match value { Some(found) => found None => 0 } } fn result_value(value: Result<Int, String>) -> Int { return match value { Ok(found) => found Err(_) => 0 } } fn direct_none() -> Option<Int> { return None } fn direct_error() -> Result<Int, String> { return Err(\"invalid\") } fn main() -> Int { return option_value(Some(7)) + option_value(None) + result_value(Ok(5)) + result_value(Err(\"invalid\")) + option_value(direct_none()) + result_value(direct_error()) }";
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(12));
    }

    #[test]
    fn executes_core_array_builtins_differentially() {
        let source = "fn main() -> Int { values = append([2, 4], 6) if contains(values, 4) { return len(values) + range(1, 5, 2)[1] } else { return 0 } }";
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(6));
    }

    #[test]
    fn executes_csv_encode_and_decode_with_shared_ast_semantics() {
        let source = r#"
fn main() -> String {
    rows = [["number", "customer", "note"], ["INV-1", "Acme, Inc.", "Paid\nby transfer"]]
    encoded = match csv_encode(rows, ',') {
        Ok(text) => text
        Err(message) => message
    }
    return match csv_decode_result(encoded, ',') {
        Ok(decoded) => decoded[1][2]
        Err(message) => message
    }
}
"#;
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::String("Paid\nby transfer".into()));
    }

    #[test]
    fn executes_fixed_point_money_builtins_with_shared_ast_semantics() {
        let source = r#"
fn money_text(input_text: String, scale: Int) -> String {
    return match decimal_scaled_result(input_text, scale) {
        Ok(value) => match format_scaled_result(value, scale) {
            Ok(text) => text
            Err(message) => message
        }
        Err(message) => message
    }
}
fn rounded(amount: Decimal, mode: String) -> Decimal {
    return match decimal_quantize_result(amount, 2, mode) {
        Ok(value) => value
        Err(_) => decimal("0")
    }
}
fn main() {}
"#;
        let (ast, mir) = program(source);
        for (source, scale, expected_value) in [
            ("119.00", 2, Value::String("119.00".into())),
            ("1,2", 2, Value::String("1.20".into())),
        ] {
            let arguments = vec![Value::String(source.into()), Value::Int(scale)];
            let actual = execute_function(
                &mir,
                "money_text",
                arguments.clone(),
                ExecutionLimits::default(),
            )
            .unwrap();
            let expected =
                zelyra_runtime::execute_function(&ast, "money_text", arguments, None).unwrap();
            assert_eq!(actual.value, expected);
            assert_eq!(actual.value, expected_value);
        }
        let arguments = vec![
            Value::Decimal(1_245_000_000),
            Value::String("half_even".into()),
        ];
        let actual = execute_function(
            &mir,
            "rounded",
            arguments.clone(),
            ExecutionLimits::default(),
        )
        .unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "rounded", arguments, None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Decimal(1_240_000_000));
    }

    #[test]
    fn executes_set_builtins_differentially() {
        let source = "fn build() -> Set<Int> { return set_add(set([3, 1, 3]), 2) } fn main() -> Int { values = set_remove(build(), 3) if contains(values, 2) { return len(values) } else { return 0 } }";
        let (ast, mir) = program(source);
        for (name, expected_value) in [
            (
                "build",
                Value::Set(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
            ),
            ("main", Value::Int(2)),
        ] {
            let actual = execute_function(&mir, name, vec![], ExecutionLimits::default()).unwrap();
            let expected = zelyra_runtime::execute_function(&ast, name, vec![], None).unwrap();
            assert_eq!(actual.value, expected);
            assert_eq!(actual.value, expected_value);
        }
    }

    #[test]
    fn executes_pure_numeric_selection_builtins_differentially() {
        let source = r#"
            fn main() -> String {
                return to_string(abs(-4))
                    + "|" + to_string(abs(decimal("-1.2")))
                    + "|" + to_string(min(2, 3))
                    + "|" + to_string(max(2.5, 3.0))
                    + "|" + min("alpha", "beta")
            }
        "#;
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::String("4|1.2|2|3|alpha".into()));
    }

    #[test]
    fn scalar_operator_matrix_matches_ast_for_uint_float_and_string() {
        let source = r#"
            fn uint_math(left: UInt, right: UInt) -> UInt { return (left + right) * right }
            fn uint_order(left: UInt, right: UInt) -> Bool { return left >= right }
            fn float_order(left: Float, right: Float) -> Bool { return left < right }
            fn string_order(left: String, right: String) -> Bool { return left < right }
            fn main() {}
        "#;
        let (ast, mir) = program(source);
        for (name, arguments) in [
            ("uint_math", vec![Value::UInt(3), Value::UInt(4)]),
            ("uint_order", vec![Value::UInt(4), Value::UInt(4)]),
            ("float_order", vec![Value::Float(1.25), Value::Float(1.5)]),
            (
                "string_order",
                vec![Value::String("alpha".into()), Value::String("beta".into())],
            ),
        ] {
            let actual =
                execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                    .unwrap();
            let expected = zelyra_runtime::execute_function(&ast, name, arguments, None).unwrap();
            assert_eq!(actual.value, expected, "function {name}");
        }
        let underflow = execute_function(
            &mir,
            "uint_math",
            vec![Value::UInt(0), Value::UInt(u64::MAX)],
            ExecutionLimits::default(),
        )
        .unwrap_err();
        let expected = zelyra_runtime::execute_function(
            &ast,
            "uint_math",
            vec![Value::UInt(0), Value::UInt(u64::MAX)],
            None,
        )
        .unwrap_err();
        assert_eq!(underflow.message, expected.message);
    }

    #[test]
    fn extended_operator_semantics_match_ast_for_decimal_duration_and_arrays() {
        let source = r#"
            fn decimal_math(left: Decimal, right: Decimal) -> Decimal {
                return (left + right) * right
            }
            fn negate_decimal(value: Decimal) -> Decimal { return -value }
            fn duration_math(left: Duration, right: Duration) -> Duration {
                return left + right
            }
            fn join_arrays(left: Int[], right: Int[]) -> Int[] { return left + right }
            fn date_order(left: Date, right: Date) -> Bool { return left <= right }
            fn main() {}
        "#;
        let (ast, mir) = program(source);
        let cases = [
            (
                "decimal_math",
                vec![Value::Decimal(1_500_000_000), Value::Decimal(2_000_000_000)],
            ),
            ("negate_decimal", vec![Value::Decimal(1_250_000_000)]),
            (
                "duration_math",
                vec![Value::Duration(3), Value::Duration(4)],
            ),
            (
                "join_arrays",
                vec![
                    Value::Array(vec![Value::Int(1)]),
                    Value::Array(vec![Value::Int(2), Value::Int(3)]),
                ],
            ),
            (
                "date_order",
                vec![
                    Value::Date("2024-02-28".into()),
                    Value::Date("2024-02-29".into()),
                ],
            ),
        ];
        for (name, arguments) in cases {
            let actual =
                execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                    .unwrap()
                    .value;
            let expected = zelyra_runtime::execute_function(&ast, name, arguments, None).unwrap();
            assert_eq!(actual, expected, "function {name}");
        }
    }

    #[test]
    fn pure_numeric_builtin_errors_match_the_ast_runtime() {
        let source = r#"
            fn decimal_value(value: String) -> Decimal { return decimal(value) }
            fn absolute(value: Int) -> Int { return abs(value) }
            fn main() {}
        "#;
        let (ast, mir) = program(source);
        for (name, arguments) in [
            ("decimal_value", vec![Value::String("1.0000000001".into())]),
            ("absolute", vec![Value::Int(i64::MIN)]),
        ] {
            let actual =
                execute_function(&mir, name, arguments.clone(), ExecutionLimits::default())
                    .unwrap_err();
            let expected =
                zelyra_runtime::execute_function(&ast, name, arguments, None).unwrap_err();
            assert_eq!(actual.message, expected.message);
        }
    }

    #[test]
    fn executes_float_math_and_clamp_builtins_differentially() {
        let source = r#"
            fn main() -> String {
                return to_string(clamp(9, 1, 5))
                    + "|" + to_string(sqrt(9.0))
                    + "|" + to_string(pow(2.0, 3.0))
                    + "|" + to_string(floor(2.8))
                    + "|" + to_string(ceil(2.1))
                    + "|" + to_string(round(1.236, 2))
            }
        "#;
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::String("5|3|8|2|3|1.24".into()));
    }

    #[test]
    fn to_float_conversion_matches_ast_for_the_laravel_projection_shape() {
        let source = r#"
fn useful_life(months: UInt) -> Float {
    return round(to_float(months) / 12.0, 2)
}
fn invalid_float() -> Float { return to_float("inf") }
fn main() {}
"#;
        let (ast, mir) = program(source);
        let arguments = vec![Value::UInt(60)];
        let actual = execute_function(
            &mir,
            "useful_life",
            arguments.clone(),
            ExecutionLimits::default(),
        )
        .expect("MIR to_float conversion");
        let expected = zelyra_runtime::execute_function(&ast, "useful_life", arguments, None)
            .expect("AST to_float conversion");
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Float(5.0));

        let actual_error =
            execute_function(&mir, "invalid_float", vec![], ExecutionLimits::default())
                .expect_err("MIR must reject non-finite parsed floats");
        let expected_error = zelyra_runtime::execute_function(&ast, "invalid_float", vec![], None)
            .expect_err("AST must reject non-finite parsed floats");
        assert_eq!(actual_error.message, expected_error.message);
        assert!(actual_error
            .message
            .contains("to_float cannot convert `inf` to a finite Float"));
    }

    #[test]
    fn executes_array_ordering_and_slicing_builtins_differentially() {
        let source = r#"
            fn main() -> String {
                values = concat(reverse(["c", "b"]), ["a"])
                sorted = sort(values)
                descending = sort_desc(values)
                selected = slice(sorted, 1, 2)
                numbers = sort([3, 1, 2])
                return join(selected, ",")
                    + "|" + join(descending, "")
                    + "|" + to_string(numbers[1])
            }
        "#;
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::String("b,c|cba|2".into()));
    }

    #[test]
    fn executes_higher_order_collection_builtins_with_captures_differentially() {
        let source = r#"
            fn double(value: Int) -> Int { return value * 2 }

            fn main() -> String {
                offset = 2
                mapped = map([1, 2, 3], |value| value + offset)
                filtered = filter(mapped, |value| value > 3)
                found = find(filtered, |value| value == 5)
                total = reduce(filtered, 0, |sum, value| sum + value)
                double_function = &double
                named = map([2, 3], double_function)
                first_found = match found {
                    Some(value) => to_string(value)
                    None => "missing"
                }
                return to_string(total) + "|" + first_found + "|" + to_string(named[0])
            }
        "#;
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::String("9|5|4".into()));
    }

    #[test]
    fn executes_pure_text_and_parse_builtins_differentially() {
        let source = r#"
            fn text_ops() -> String {
                upper = text_to_upper(text_trim("  grüß "))
                lower = text_to_lower(upper)
                count = to_string(len(text_chars("é🙂")))
                part = text_split("first,second", ",")[1]
                if text_contains(upper, "Ü") && text_starts_with(upper, "GR") && text_ends_with(upper, "SS") {
                    return text_replace(upper, "Ü", "UE") + "|" + lower + "|" + count + "|" + part
                } else {
                    return "invalid"
                }
            }

            fn parse_ops() -> String {
                integer = parse_int(" 42 ")
                float = parse_float("2.5")
                invalid = parse_int("no")
                return match integer { Ok(value) => to_string(value) Err(_) => "invalid integer" }
                    + "|"
                    + match float { Ok(value) => to_string(value) Err(_) => "invalid float" }
                    + "|"
                    + match invalid { Ok(_) => "unexpected success" Err(reason) => reason }
            }

            fn ucfirst_ops() -> String {
                return text_ucfirst("hello") + "|" + text_ucfirst("äbc")
            }

            fn main() -> String { return text_ops() }
        "#;
        let (ast, mir) = program(source);
        for (name, expected_value) in [
            ("text_ops", "GRUESS|grüss|2|second"),
            ("parse_ops", "42|2.5|invalid integer: no"),
            ("ucfirst_ops", "Hello|äbc"),
        ] {
            let actual = execute_function(&mir, name, vec![], ExecutionLimits::default()).unwrap();
            let expected = zelyra_runtime::execute_function(&ast, name, vec![], None).unwrap();
            assert_eq!(actual.value, expected);
            assert_eq!(actual.value, Value::String(expected_value.into()));
        }
    }

    #[test]
    fn executes_opt_in_mutable_array_index_updates_differentially() {
        let source = "fn main() -> Int { mutable totals: Int[] = [4, 7] totals[0] += 1 totals[1] = 9 return totals[0] + totals[1] }";
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(14));
    }

    #[test]
    fn executes_mutable_array_push_and_pop_differentially() {
        let source = r#"
fn main() -> Int {
    mutable values: Int[] = []
    push_mut(values, 4)
    push_mut(values, 8)
    last = pop_mut(values)
    first = pop_mut(values)
    empty = pop_mut(values)
    last_value = match last { Some(value) => value None => 0 }
    first_value = match first { Some(value) => value None => 0 }
    if is_none(empty) { return last_value * 10 + first_value }
    return 0
}
"#;
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(84));
    }

    #[test]
    fn executes_enum_payload_match_expressions_with_guards_differentially() {
        let source = "enum Outcome { Draft Paid(Int) } fn main() -> Int { outcome = Outcome::Paid(7) return match outcome { Outcome::Draft => 0 Outcome::Paid(value) if value > 4 => value Outcome::Paid(_) => 1 } }";
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(7));
    }

    #[test]
    fn executes_enum_payload_match_statements_with_guards_differentially() {
        let source = "enum Outcome { Draft Paid(Int) } fn main() -> Int { outcome = Outcome::Paid(7) match outcome { Outcome::Draft => { return 0 } Outcome::Paid(value) if value > 4 => { return value } Outcome::Paid(_) => { return 1 } } }";
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(7));
    }

    #[test]
    fn executes_collection_and_stepped_range_loops_differentially() {
        let source = "fn main() -> Int { mutable total = 0 for value in [1, 2, 3] { total = total + value } for value in range(1, 6, 2) { total = total + value } return total }";
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(15));
    }

    #[test]
    fn executes_while_and_loop_control_differentially() {
        let source = "fn main() -> Int { mutable value = 0 mutable total = 0 while value < 4 { value = value + 1 if value == 2 { continue } total = total + value } loop { value = value + 1 if value >= 6 { break } total = total + 10 } return total }";
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(18));
    }

    #[test]
    fn executes_tuple_destructuring_collection_loops_differentially() {
        let source = "fn main() -> Int { mutable total = 0 for (left, right) in [(1, 2), (3, 4)] { total = total + left + right } return total }";
        let (ast, mir) = program(source);
        let actual = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        let expected = zelyra_runtime::execute_function(&ast, "main", vec![], None).unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(10));
    }

    #[test]
    fn enforces_step_budget_and_call_depth() {
        let (_, mir) = program(
            "fn descend(value: Int) -> Int { if value == 0 { return 0 } else { return descend(value - 1) } } fn main() {}",
        );
        let step_error = execute_function(
            &mir,
            "descend",
            vec![Value::Int(20)],
            ExecutionLimits {
                max_steps: 8,
                max_stack_depth: 100,
            },
        )
        .unwrap_err();
        assert!(step_error.message.contains("step budget exceeded"));

        let stack_error = execute_function(
            &mir,
            "descend",
            vec![Value::Int(20)],
            ExecutionLimits {
                max_steps: 10_000,
                max_stack_depth: 4,
            },
        )
        .unwrap_err();
        assert!(stack_error.message.contains("stack depth exceeded"));
    }

    #[test]
    fn assertion_builtin_uses_shared_success_and_failure_semantics() {
        let (_, mir) = program("fn main() { assert(true, \"ok\") }");
        let success = execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap();
        assert_eq!(success.value, Value::Unit);

        let (_, mir) = program("fn main() { assert(false, \"inventory mismatch\") }");
        let failure =
            execute_function(&mir, "main", vec![], ExecutionLimits::default()).unwrap_err();
        assert_eq!(failure.message, "inventory mismatch");
    }

    #[test]
    fn clock_host_service_uses_declared_capability_and_project_grant() {
        let (ast, mir) = program("fn main() -> Timestamp uses Clock { return now() }");
        let host = Arc::new(ClockHost {
            timestamp: 1_700_000_000_000,
            environment: None,
            calls: AtomicUsize::new(0),
        });
        let grants = HashSet::from(["Clock".to_owned()]);
        let result = execute_function_with_host_services(
            &mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            Some(&grants),
            host.clone(),
        )
        .unwrap();
        let expected =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                "main",
                vec![],
                None,
                Some(&grants),
                None,
                Arc::new(ClockHost {
                    timestamp: host.timestamp,
                    environment: None,
                    calls: AtomicUsize::new(0),
                }),
            )
            .unwrap();
        assert_eq!(result.value, expected);
        assert_eq!(result.value, Value::Timestamp(host.timestamp));
        assert_eq!(host.calls.load(Ordering::SeqCst), 1);

        let denied_host = Arc::new(ClockHost {
            timestamp: 1_700_000_000_000,
            environment: None,
            calls: AtomicUsize::new(0),
        });
        let denied = execute_function_with_host_services(
            &mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            Some(&HashSet::new()),
            denied_host.clone(),
        )
        .unwrap_err();
        assert!(denied.message.contains("`Clock` is not granted"));
        assert_eq!(denied_host.calls.load(Ordering::SeqCst), 0);

        let (_, mut undeclared_mir) = program("fn main() -> Timestamp uses Clock { return now() }");
        undeclared_mir
            .functions
            .iter_mut()
            .find(|function| function.name == "main")
            .unwrap()
            .declared_capabilities
            .clear();
        let undeclared_host = Arc::new(ClockHost {
            timestamp: 1_700_000_000_000,
            environment: None,
            calls: AtomicUsize::new(0),
        });
        let undeclared = execute_function_with_host_services(
            &undeclared_mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            None,
            undeclared_host.clone(),
        )
        .unwrap_err();
        assert!(undeclared
            .message
            .contains("uses clock access requires capability `Clock`"));
        assert_eq!(undeclared_host.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn called_functions_cannot_escalate_the_callers_declared_capabilities() {
        let (_, mut mir) = program(
            "fn read_clock() -> Timestamp uses Clock { return now() } fn main() -> Timestamp uses Clock { return read_clock() }",
        );
        mir.functions
            .iter_mut()
            .find(|function| function.name == "main")
            .unwrap()
            .declared_capabilities
            .clear();
        let host = Arc::new(ClockHost {
            timestamp: 1_700_000_000_000,
            environment: None,
            calls: AtomicUsize::new(0),
        });
        let error = execute_function_with_host_services(
            &mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            None,
            host.clone(),
        )
        .unwrap_err();
        assert!(error
            .message
            .contains("calls `read_clock` requires capability `Clock`"));
        assert_eq!(host.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn http_request_uses_shared_policy_and_matches_the_ast_runtime() {
        let source = "fn request() -> HttpResponse uses Network { return http_request(\"POST\", \"https://api.example.test/customers\", [\"X-Request-ID: test-7\"], Some(\"payload\")) } fn main() {}";
        let (ast, mir) = program(source);
        let grants = HashSet::from(["Network".to_owned()]);
        let policy = RuntimePolicy {
            network: Some(NetworkPolicy {
                allowed_hosts: vec!["api.example.test".into()],
                timeout_ms: 1234,
                max_response_bytes: 4096,
            }),
            ..RuntimePolicy::default()
        };
        let host = || {
            Arc::new(HttpHost {
                request: Mutex::new(None),
                response: HostHttpResponse {
                    status: 201,
                    headers: vec![("content-type".into(), "text/plain".into())],
                    body: b"created".to_vec(),
                },
                calls: AtomicUsize::new(0),
            })
        };
        let mir_host = host();
        let mir_value = execute_function_with_host_services_and_policy(
            &mir,
            "request",
            vec![],
            ExecutionLimits::default(),
            Some(&grants),
            Some(&policy),
            mir_host.clone(),
        )
        .unwrap()
        .value;
        let ast_host = host();
        let ast_value =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                "request",
                vec![],
                None,
                Some(&grants),
                Some(&policy),
                ast_host.clone(),
            )
            .unwrap();
        assert_eq!(mir_value, ast_value);
        assert_eq!(mir_host.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            *mir_host.request.lock().unwrap(),
            Some((
                "POST".into(),
                "https://api.example.test/customers".into(),
                vec![("x-request-id".into(), "test-7".into())],
                Some(b"payload".to_vec()),
                1234,
                4096,
            ))
        );
    }

    #[test]
    fn typed_http_json_and_result_match_the_ast_runtime() {
        let source = r#"
            struct CustomerCreate { name: String }
            struct Customer { id: Int name: String }
            fn create(payload: CustomerCreate) -> Customer uses Network {
                return http_json<CustomerCreate, Customer>("POST", "https://api.example.test/customers", ["Content-Type: application/json"], Some(payload))
            }
            fn submit() -> HttpResult<Customer> uses Network {
                return http_result<Customer, Customer>("POST", "https://api.example.test/customers", [], None)
            }
            fn main() {}
        "#;
        let (ast, mir) = program(source);
        let grants = HashSet::from(["Network".to_owned()]);
        let policy = RuntimePolicy {
            network: Some(NetworkPolicy {
                allowed_hosts: vec!["api.example.test".into()],
                timeout_ms: 900,
                max_response_bytes: 4096,
            }),
            ..RuntimePolicy::default()
        };
        let payload = Value::Object {
            type_name: "CustomerCreate".into(),
            type_args: vec![],
            fields: HashMap::from([("name".into(), Value::String("Ada".into()))]),
        };
        let host = |status: u16, body: &[u8]| {
            Arc::new(HttpHost {
                request: Mutex::new(None),
                response: HostHttpResponse {
                    status,
                    headers: vec![("content-type".into(), "application/json".into())],
                    body: body.to_vec(),
                },
                calls: AtomicUsize::new(0),
            })
        };

        let mir_json_host = host(201, br#"{"id":42,"name":"Ada"}"#);
        let mir_json = execute_function_with_host_services_and_policy(
            &mir,
            "create",
            vec![payload.clone()],
            ExecutionLimits::default(),
            Some(&grants),
            Some(&policy),
            mir_json_host.clone(),
        )
        .unwrap()
        .value;
        let ast_json_host = host(201, br#"{"id":42,"name":"Ada"}"#);
        let ast_json =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                "create",
                vec![payload],
                None,
                Some(&grants),
                Some(&policy),
                ast_json_host,
            )
            .unwrap();
        assert_eq!(mir_json, ast_json);
        assert_eq!(mir_json_host.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            mir_json_host.request.lock().unwrap().as_ref().unwrap().3,
            Some(br#"{"name":"Ada"}"#.to_vec())
        );

        let response_body = br#"{"message":"invalid"}"#;
        let mir_result_host = host(422, response_body);
        let mir_result = execute_function_with_host_services_and_policy(
            &mir,
            "submit",
            vec![],
            ExecutionLimits::default(),
            Some(&grants),
            Some(&policy),
            mir_result_host.clone(),
        )
        .unwrap()
        .value;
        let ast_result_host = host(422, response_body);
        let ast_result =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                "submit",
                vec![],
                None,
                Some(&grants),
                Some(&policy),
                ast_result_host,
            )
            .unwrap();
        assert_eq!(mir_result, ast_result);
        assert_eq!(mir_result_host.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn shared_json_codecs_keep_mir_records_and_results_in_ast_parity() {
        let source = r#"
            struct Customer { id: Int name: String }
            fn encode(customer: Customer) -> String { return json_encode(customer) }
            fn decode(raw: String) -> Customer { return json_decode<Customer>(raw) }
            fn decode_result(raw: String) -> Result<Customer, String> {
                return json_decode_result<Customer>(raw)
            }
            fn encode_tagged(value: Result<Customer, String>) -> String {
                return json_encode_tagged_result(value)
            }
            fn decode_tagged(raw: String) -> Result<Result<Customer, String>, String> {
                return json_decode_tagged_result<Customer, String>(raw)
            }
            fn main() {}
        "#;
        let (ast, mir) = program(source);
        let customer = Value::Object {
            type_name: "Customer".into(),
            type_args: vec![],
            fields: HashMap::from([
                ("id".into(), Value::Int(42)),
                ("name".into(), Value::String("Ada".into())),
            ]),
        };
        let cases = [
            ("encode", vec![customer.clone()]),
            (
                "decode",
                vec![Value::String(r#"{"id":42,"name":"Ada"}"#.into())],
            ),
            (
                "decode_result",
                vec![Value::String(r#"{"id":42,"name":"Ada"}"#.into())],
            ),
            ("decode_result", vec![Value::String("{invalid".into())]),
            (
                "encode_tagged",
                vec![Value::Result(Ok(Box::new(customer.clone())))],
            ),
            (
                "decode_tagged",
                vec![Value::String(
                    r#"{"$result":"ok","value":{"id":42,"name":"Ada"}}"#.into(),
                )],
            ),
            (
                "decode_tagged",
                vec![Value::String(r#"{"$result":"err","error":"bad"}"#.into())],
            ),
        ];
        for (function, arguments) in cases {
            let actual = execute_function(
                &mir,
                function,
                arguments.clone(),
                ExecutionLimits::default(),
            )
            .unwrap()
            .value;
            let expected =
                zelyra_runtime::execute_function(&ast, function, arguments, None).unwrap();
            assert_eq!(actual, expected, "JSON operation `{function}` differs");
        }
    }

    #[test]
    fn date_arithmetic_and_result_builtins_match_the_ast_runtime() {
        let source = r#"
            fn plus(value: Date, days: Int) -> Date { return date_add_days(value, days) }
            fn difference(start: Date, end: Date) -> Int {
                return date_diff_days(start, end)
            }
            fn plus_result(value: Date, days: Int) -> Result<Date, String> {
                return date_add_days_result(value, days)
            }
            fn between_result(start: Date, end: Date) -> Result<Int, String> {
                return date_days_between_result(start, end)
            }
            fn main() {}
        "#;
        let (ast, mir) = program(source);
        let cases = [
            (
                "plus",
                vec![Value::Date("2024-02-28".into()), Value::Int(1)],
            ),
            (
                "difference",
                vec![
                    Value::Date("2024-03-01".into()),
                    Value::Date("2024-02-28".into()),
                ],
            ),
            (
                "plus_result",
                vec![Value::Date("9999-12-31".into()), Value::Int(1)],
            ),
            (
                "between_result",
                vec![
                    Value::Date("2024-02-28".into()),
                    Value::Date("2024-03-01".into()),
                ],
            ),
        ];
        for (function, arguments) in cases {
            let actual = execute_function(
                &mir,
                function,
                arguments.clone(),
                ExecutionLimits::default(),
            )
            .unwrap()
            .value;
            let expected =
                zelyra_runtime::execute_function(&ast, function, arguments, None).unwrap();
            assert_eq!(actual, expected, "date operation `{function}` differs");
        }
        let overflow = execute_function(
            &mir,
            "plus",
            vec![Value::Date("9999-12-31".into()), Value::Int(1)],
            ExecutionLimits::default(),
        )
        .unwrap_err();
        assert!(overflow.message.contains("outside years 0001 through 9999"));
    }

    #[test]
    fn option_result_and_date_predicates_match_the_ast_runtime() {
        let source = r#"
            fn result_ok(value: Result<Int, String>) -> Bool { return is_ok(value) }
            fn result_err(value: Result<Int, String>) -> Bool { return is_err(value) }
            fn option_some(value: Int?) -> Bool { return is_some(value) }
            fn option_none(value: Int?) -> Bool { return is_none(value) }
            fn before(left: Date, right: Date) -> Bool { return is_before(left, right) }
            fn after(left: Date, right: Date) -> Bool { return is_after(left, right) }
            fn main() {}
        "#;
        let (ast, mir) = program(source);
        let cases = [
            (
                "result_ok",
                vec![Value::Result(Ok(Box::new(Value::Int(3))))],
            ),
            (
                "result_err",
                vec![Value::Result(Err(Box::new(Value::String("bad".into()))))],
            ),
            (
                "option_some",
                vec![Value::Option(Some(Box::new(Value::Int(2))))],
            ),
            ("option_none", vec![Value::Option(None)]),
            (
                "before",
                vec![
                    Value::Date("2024-01-01".into()),
                    Value::Date("2024-02-01".into()),
                ],
            ),
            (
                "after",
                vec![
                    Value::Date("2024-02-01".into()),
                    Value::Date("2024-01-01".into()),
                ],
            ),
        ];
        for (function, arguments) in cases {
            let actual = execute_function(
                &mir,
                function,
                arguments.clone(),
                ExecutionLimits::default(),
            )
            .unwrap()
            .value;
            let expected =
                zelyra_runtime::execute_function(&ast, function, arguments, None).unwrap();
            assert_eq!(actual, expected, "predicate `{function}` differs");
        }
    }

    #[test]
    fn http_requests_are_denied_before_transport_without_grants_or_allowlist() {
        let (_, mir) = program(
            "fn request() -> HttpResponse uses Network { return http_request(\"GET\", \"https://api.example.test/\", [], None) } fn main() {}",
        );
        let host = Arc::new(HttpHost {
            request: Mutex::new(None),
            response: HostHttpResponse {
                status: 200,
                headers: vec![],
                body: b"ok".to_vec(),
            },
            calls: AtomicUsize::new(0),
        });
        let no_grant = execute_function_with_host_services(
            &mir,
            "request",
            vec![],
            ExecutionLimits::default(),
            Some(&HashSet::new()),
            host.clone(),
        )
        .unwrap_err();
        assert!(no_grant.message.contains("`Network` is not granted"));
        assert_eq!(host.calls.load(Ordering::SeqCst), 0);

        let grants = HashSet::from(["Network".to_owned()]);
        let policy = RuntimePolicy {
            network: Some(NetworkPolicy {
                allowed_hosts: vec!["other.example.test".into()],
                timeout_ms: 1000,
                max_response_bytes: 4096,
            }),
            ..RuntimePolicy::default()
        };
        let denied_host = Arc::new(HttpHost {
            request: Mutex::new(None),
            response: HostHttpResponse {
                status: 200,
                headers: vec![],
                body: b"ok".to_vec(),
            },
            calls: AtomicUsize::new(0),
        });
        let denied_host_error = execute_function_with_host_services_and_policy(
            &mir,
            "request",
            vec![],
            ExecutionLimits::default(),
            Some(&grants),
            Some(&policy),
            denied_host.clone(),
        )
        .unwrap_err();
        assert!(denied_host_error
            .message
            .contains("network access denied for host"));
        assert_eq!(denied_host.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn mail_delivery_uses_shared_policy_and_matches_the_ast_runtime() {
        let source = "fn deliver(id: String, payload: String) -> Result<Unit, String> uses Email { return mail_deliver(id, payload) } fn main() {}";
        let (ast, mir) = program(source);
        let grants = HashSet::from(["Email".to_owned()]);
        let mut mail = MailPolicy::default();
        mail.host = Some("smtp.example.test".into());
        mail.from = Some("billing@example.test".into());
        let policy = RuntimePolicy {
            network: Some(NetworkPolicy {
                allowed_hosts: vec!["smtp.example.test".into()],
                timeout_ms: 1000,
                max_response_bytes: 1024,
            }),
            mail: Some(mail),
            ..RuntimePolicy::default()
        };
        let host = || {
            Arc::new(MailHost {
                delivery: Mutex::new(None),
                calls: AtomicUsize::new(0),
            })
        };
        let arguments = || {
            vec![
                Value::String("event-01".into()),
                Value::String(r#"{"to":"a@example.test","subject":"Hi","text":"Body"}"#.into()),
            ]
        };
        let mir_host = host();
        let mir_result = execute_function_with_host_services_and_policy(
            &mir,
            "deliver",
            arguments(),
            ExecutionLimits::default(),
            Some(&grants),
            Some(&policy),
            mir_host.clone(),
        )
        .unwrap()
        .value;
        let ast_host = host();
        let ast_result =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                "deliver",
                arguments(),
                None,
                Some(&grants),
                Some(&policy),
                ast_host,
            )
            .unwrap();
        assert_eq!(mir_result, ast_result);
        assert_eq!(
            *mir_host.delivery.lock().unwrap(),
            Some((
                "event-01".into(),
                r#"{"to":"a@example.test","subject":"Hi","text":"Body"}"#.into(),
                "smtp.example.test".into(),
            ))
        );
        assert_eq!(mir_host.calls.load(Ordering::SeqCst), 1);

        let denied_host = host();
        let no_grant = execute_function_with_host_services(
            &mir,
            "deliver",
            arguments(),
            ExecutionLimits::default(),
            Some(&HashSet::new()),
            denied_host.clone(),
        )
        .unwrap_err();
        assert!(no_grant.message.contains("`Email` is not granted"));
        assert_eq!(denied_host.calls.load(Ordering::SeqCst), 0);

        let denied_policy = RuntimePolicy {
            network: Some(NetworkPolicy {
                allowed_hosts: vec!["other.example.test".into()],
                timeout_ms: 1000,
                max_response_bytes: 1024,
            }),
            ..policy
        };
        let smtp_denied_host = host();
        let smtp_denied = execute_function_with_host_services_and_policy(
            &mir,
            "deliver",
            arguments(),
            ExecutionLimits::default(),
            Some(&grants),
            Some(&denied_policy),
            smtp_denied_host.clone(),
        )
        .unwrap()
        .value;
        assert!(matches!(
            smtp_denied,
            Value::Result(Err(message)) if matches!(*message, Value::String(ref message) if message.contains("not allowed"))
        ));
        assert_eq!(smtp_denied_host.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn sql_queries_use_resolved_locals_capabilities_and_shared_storage_checks() {
        let source = r#"
            database main { engine: mariadb database: "mir_read_only_test" }
            fn lookup(expected: Int) -> Int[] uses Database(read) {
                return sql<Int[]> { SELECT :expected AS value }
            }
            fn main() {}
        "#;
        let (ast, mir) = program(source);
        let grants = HashSet::from(["Database(read)".to_owned()]);
        let database_url = "mariadb://user:secret@127.0.0.1:3306/mir_test";
        let result = zelyra_database::QueryResult {
            columns: vec!["value".into()],
            rows: vec![vec!["42".into()]],
            nulls: vec![vec![false]],
            column_types: vec![zelyra_database::QueryColumnType::Integer],
        };
        let host = || {
            Arc::new(DatabaseHost {
                query: Mutex::new(None),
                result: result.clone(),
                validations: AtomicUsize::new(0),
                queries: AtomicUsize::new(0),
            })
        };
        let mir_host = host();
        let mir_result = execute_function_with_database_and_host_services_and_policy(
            &mir,
            "lookup",
            vec![Value::Int(42)],
            ExecutionLimits::default(),
            Some(&grants),
            None,
            DatabaseExecution {
                url: database_url,
                host_services: mir_host.clone(),
            },
        )
        .unwrap()
        .value;
        let ast_host = host();
        let ast_result =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                "lookup",
                vec![Value::Int(42)],
                Some(database_url),
                Some(&grants),
                None,
                ast_host,
            )
            .unwrap();
        assert_eq!(mir_result, ast_result);
        assert_eq!(mir_host.validations.load(Ordering::SeqCst), 1);
        assert_eq!(mir_host.queries.load(Ordering::SeqCst), 1);
        let captured = mir_host.query.lock().unwrap();
        let (captured_url, captured_sql, captured_params) = captured.as_ref().unwrap();
        assert_eq!(captured_url, database_url);
        assert_eq!(captured_sql, " SELECT :expected AS value ");
        assert_eq!(captured_params.len(), 1);
        assert_eq!(captured_params[0].0, "expected");
        assert!(matches!(
            captured_params[0].1,
            zelyra_database::QueryValue::Int(42)
        ));

        let denied_host = host();
        let denied = execute_function_with_database_and_host_services_and_policy(
            &mir,
            "lookup",
            vec![Value::Int(42)],
            ExecutionLimits::default(),
            Some(&HashSet::new()),
            None,
            DatabaseExecution {
                url: database_url,
                host_services: denied_host.clone(),
            },
        )
        .unwrap_err();
        assert!(denied.message.contains("`Database(read)` is not granted"));
        assert_eq!(denied_host.validations.load(Ordering::SeqCst), 0);
        assert_eq!(denied_host.queries.load(Ordering::SeqCst), 0);

        let missing_database = execute_function_with_host_services(
            &mir,
            "lookup",
            vec![Value::Int(42)],
            ExecutionLimits::default(),
            Some(&grants),
            host(),
        )
        .unwrap_err();
        assert!(missing_database
            .message
            .contains("SQL execution requires a database runtime"));
    }

    #[test]
    fn sql_decode_uses_the_shared_native_row_decoder_in_ast_and_mir() {
        let source = r#"
            database main { engine: mariadb database: "mir_decode_test" }
            fn lookup(expected: Int) -> Int[] uses Database(read) {
                return sql_decode<Int[]>(sql<Int[]> { SELECT :expected AS value })
            }
            fn main() {}
        "#;
        let (ast, mir) = program(source);
        let bytecode = bytecode::compile(&mir, "lookup").expect("SQL decoder lowers to bytecode");
        let bytecode = bytecode::decode(&bytecode::encode(&bytecode).unwrap()).unwrap();
        let grants = HashSet::from(["Database".to_owned()]);
        let database_url = "mariadb://user:secret@127.0.0.1:3306/mir_decode_test";
        let result = zelyra_database::QueryResult {
            columns: vec!["value".into()],
            rows: vec![vec!["42".into()], vec!["43".into()]],
            nulls: vec![vec![false], vec![false]],
            column_types: vec![zelyra_database::QueryColumnType::Integer],
        };
        let host = || {
            Arc::new(DatabaseHost {
                query: Mutex::new(None),
                result: result.clone(),
                validations: AtomicUsize::new(0),
                queries: AtomicUsize::new(0),
            })
        };
        let mir_value = execute_function_with_database_and_host_services_and_policy(
            &mir,
            "lookup",
            vec![Value::Int(42)],
            ExecutionLimits::default(),
            Some(&grants),
            None,
            DatabaseExecution {
                url: database_url,
                host_services: host(),
            },
        )
        .unwrap()
        .value;
        let ast_value =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                "lookup",
                vec![Value::Int(42)],
                Some(database_url),
                Some(&grants),
                None,
                host(),
            )
            .unwrap();
        let bytecode_outcome =
            bytecode::execute_with_database_and_host_services_and_policy_captured(
                &bytecode,
                vec![Value::Int(42)],
                ExecutionLimits::default(),
                Some(&grants),
                None,
                bytecode::BytecodeDatabaseExecution {
                    url: database_url,
                    schema: &mir.database_schema,
                },
                host(),
            );

        assert_eq!(mir_value, ast_value);
        assert_eq!(
            mir_value,
            Value::Array(vec![Value::Int(42), Value::Int(43)])
        );
        assert_eq!(bytecode_outcome.error, None);
        assert_eq!(bytecode_outcome.value, Some(mir_value));
    }

    #[test]
    fn sql_decode_materializes_table_derived_records_across_backends() {
        let source = r#"
            database main { engine: mariadb database: "mir_record_decode_test" }
            table invoices {
                id: Id primary auto
                customer: String(80) required
            }
            fn invoices() -> Invoice[] uses Database(read) {
                return sql_decode<Invoice[]>(sql<Invoice[]> {
                    SELECT id, customer FROM invoices
                })
            }
            fn main() {}
        "#;
        let (ast, mir) = program(source);
        assert!(mir
            .sql_records
            .iter()
            .any(|record| record.name == "Invoice"));
        let bytecode = bytecode::compile(&mir, "invoices").expect("table projections lower");
        let bytecode = bytecode::decode(&bytecode::encode(&bytecode).unwrap()).unwrap();
        let grants = HashSet::from(["Database".to_owned()]);
        let database_url = "mariadb://user:secret@127.0.0.1:3306/mir_record_decode_test";
        let result = zelyra_database::QueryResult {
            columns: vec!["id".into(), "customer".into()],
            rows: vec![vec!["17".into(), "Northwind GmbH".into()]],
            nulls: vec![vec![false, false]],
            column_types: vec![
                zelyra_database::QueryColumnType::Integer,
                zelyra_database::QueryColumnType::Text,
            ],
        };
        let host = || {
            Arc::new(DatabaseHost {
                query: Mutex::new(None),
                result: result.clone(),
                validations: AtomicUsize::new(0),
                queries: AtomicUsize::new(0),
            })
        };
        let expected = Value::Array(vec![Value::Object {
            type_name: "Invoice".into(),
            type_args: Vec::new(),
            fields: HashMap::from([
                ("id".into(), Value::Int(17)),
                ("customer".into(), Value::String("Northwind GmbH".into())),
            ]),
        }]);
        let mir_value = execute_function_with_database_and_host_services_and_policy(
            &mir,
            "invoices",
            Vec::new(),
            ExecutionLimits::default(),
            Some(&grants),
            None,
            DatabaseExecution {
                url: database_url,
                host_services: host(),
            },
        )
        .unwrap()
        .value;
        let ast_value =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                "invoices",
                Vec::new(),
                Some(database_url),
                Some(&grants),
                None,
                host(),
            )
            .unwrap();
        let bytecode_outcome =
            bytecode::execute_with_database_and_host_services_and_policy_captured(
                &bytecode,
                Vec::new(),
                ExecutionLimits::default(),
                Some(&grants),
                None,
                bytecode::BytecodeDatabaseExecution {
                    url: database_url,
                    schema: &mir.database_schema,
                },
                host(),
            );

        assert_eq!(ast_value, expected);
        assert_eq!(mir_value, expected);
        assert_eq!(bytecode_outcome.error, None);
        assert_eq!(bytecode_outcome.value, Some(expected));
    }

    #[test]
    fn sql_transactions_use_one_atomic_host_batch_and_match_ast() {
        let source = r#"
            database main { engine: mariadb database: "transaction_test" }
            table accounts { id: Id primary auto balance: Int required }
            fn transfer(from_id: Int, to_id: Int, amount: Int) uses Database(write) {
                transaction {
                    sql { UPDATE accounts SET balance = balance - :amount WHERE id = :from_id }
                    sql { UPDATE accounts SET balance = balance + :amount WHERE id = :to_id }
                    saved_row: Unit = sql { UPDATE accounts SET balance = balance WHERE id = :to_id }
                }
            }
            fn main() {}
        "#;
        let (ast, mir) = program(source);
        let grants = HashSet::from(["Database(write)".to_owned()]);
        let database_url = "mariadb://user:secret@127.0.0.1:3306/mir_transaction_test";
        let make_host = || {
            Arc::new(TransactionHost {
                batches: Mutex::new(Vec::new()),
                validations: AtomicUsize::new(0),
                failure: None,
            })
        };
        let arguments = || vec![Value::Int(10), Value::Int(20), Value::Int(75)];

        let mir_host = make_host();
        let mir_result = execute_function_with_database_and_host_services_and_policy(
            &mir,
            "transfer",
            arguments(),
            ExecutionLimits::default(),
            Some(&grants),
            None,
            DatabaseExecution {
                url: database_url,
                host_services: mir_host.clone(),
            },
        )
        .unwrap()
        .value;
        let ast_host = make_host();
        let ast_result =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                "transfer",
                arguments(),
                Some(database_url),
                Some(&grants),
                None,
                ast_host.clone(),
            )
            .unwrap();

        assert_eq!(mir_result, ast_result);
        assert_eq!(mir_host.validations.load(Ordering::SeqCst), 1);
        let mir_batches = mir_host.batches.lock().unwrap();
        let ast_batches = ast_host.batches.lock().unwrap();
        assert_eq!(mir_batches.len(), 1);
        assert_eq!(ast_batches.len(), 1);
        for batches in [&*mir_batches, &*ast_batches] {
            let (url, queries, transaction) = &batches[0];
            assert_eq!(url, database_url);
            assert!(*transaction);
            assert_eq!(queries.len(), 3);
            assert!(queries[0].sql.contains("balance = balance - :amount"));
            assert!(queries[1].sql.contains("balance = balance + :amount"));
            assert!(queries[2]
                .sql
                .contains("balance = balance WHERE id = :to_id"));
            assert!(matches!(
                queries[0].params.as_slice(),
                [(name1, zelyra_database::QueryValue::Int(75)), (name2, zelyra_database::QueryValue::Int(10))]
                    if name1 == "amount" && name2 == "from_id"
            ));
            assert!(matches!(
                queries[1].params.as_slice(),
                [(name1, zelyra_database::QueryValue::Int(75)), (name2, zelyra_database::QueryValue::Int(20))]
                    if name1 == "amount" && name2 == "to_id"
            ));
            assert!(matches!(
                queries[2].params.as_slice(),
                [(name, zelyra_database::QueryValue::Int(20))] if name == "to_id"
            ));
        }
    }

    #[test]
    fn sql_transaction_failures_and_missing_capabilities_fail_closed() {
        let source = r#"
            database main { engine: mariadb database: "transaction_test" }
            table accounts { id: Id primary auto balance: Int required }
            fn transfer(account_id: Int, amount: Int) uses Database(write) {
                transaction {
                    sql { UPDATE accounts SET balance = balance - :amount WHERE id = :account_id }
                }
            }
            fn main() {}
        "#;
        let (ast, mir) = program(source);
        let grants = HashSet::from(["Database(write)".to_owned()]);
        let database_url = "mariadb://user:secret@127.0.0.1:3306/mir_transaction_test";
        let make_host = |failure: Option<&str>| {
            Arc::new(TransactionHost {
                batches: Mutex::new(Vec::new()),
                validations: AtomicUsize::new(0),
                failure: failure.map(str::to_owned),
            })
        };
        let arguments = || vec![Value::Int(10), Value::Int(75)];

        let failing_mir_host = make_host(Some("transaction rejected"));
        let mir_error = execute_function_with_database_and_host_services_and_policy(
            &mir,
            "transfer",
            arguments(),
            ExecutionLimits::default(),
            Some(&grants),
            None,
            DatabaseExecution {
                url: database_url,
                host_services: failing_mir_host.clone(),
            },
        )
        .unwrap_err();
        let failing_ast_host = make_host(Some("transaction rejected"));
        let ast_error =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                "transfer",
                arguments(),
                Some(database_url),
                Some(&grants),
                None,
                failing_ast_host,
            )
            .unwrap_err();
        assert_eq!(mir_error.message, ast_error.message);
        assert!(mir_error.message.contains("database transaction failed"));

        let denied_host = make_host(None);
        let denied = execute_function_with_database_and_host_services_and_policy(
            &mir,
            "transfer",
            arguments(),
            ExecutionLimits::default(),
            Some(&HashSet::new()),
            None,
            DatabaseExecution {
                url: database_url,
                host_services: denied_host.clone(),
            },
        )
        .unwrap_err();
        assert!(denied.message.contains("`Database(write)` is not granted"));
        assert_eq!(denied_host.validations.load(Ordering::SeqCst), 0);
        assert!(denied_host.batches.lock().unwrap().is_empty());

        let no_database_host = make_host(None);
        let missing_database = execute_function_with_host_services(
            &mir,
            "transfer",
            arguments(),
            ExecutionLimits::default(),
            Some(&grants),
            no_database_host.clone(),
        )
        .unwrap_err();
        assert!(missing_database
            .message
            .contains("transaction execution requires a database runtime"));
        assert!(no_database_host.batches.lock().unwrap().is_empty());
    }

    #[test]
    fn mail_queue_is_batched_with_sql_and_uses_the_shared_outbox_contract() {
        let source = r#"
            database main { engine: mariadb database: "outbox_test" }
            table accounts { id: Id primary auto balance: Int required }
            fn enqueue(to: String, subject: String, body: String) uses Email, Database(write) {
                transaction {
                    mail_queue(to, subject, body)
                    sql { UPDATE accounts SET balance = 1 WHERE id = 1 }
                }
            }
            fn main() {}
        "#;
        let (ast, mir) = program(source);
        let grants = HashSet::from(["Email".to_owned(), "Database(write)".to_owned()]);
        let database_url = "mariadb://user:secret@127.0.0.1:3306/mir_outbox_test";
        let host = || {
            Arc::new(TransactionHost {
                batches: Mutex::new(Vec::new()),
                validations: AtomicUsize::new(0),
                failure: None,
            })
        };
        let arguments = || {
            vec![
                Value::String("customer@example.test".into()),
                Value::String("Invoice ready".into()),
                Value::String("Please find your invoice attached.".into()),
            ]
        };
        let mir_host = host();
        let mir_result = execute_function_with_database_and_host_services_and_policy(
            &mir,
            "enqueue",
            arguments(),
            ExecutionLimits::default(),
            Some(&grants),
            None,
            DatabaseExecution {
                url: database_url,
                host_services: mir_host.clone(),
            },
        )
        .unwrap()
        .value;
        let ast_host = host();
        let ast_result =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                "enqueue",
                arguments(),
                Some(database_url),
                Some(&grants),
                None,
                ast_host.clone(),
            )
            .unwrap();
        assert_eq!(mir_result, ast_result);
        assert_eq!(mir_host.validations.load(Ordering::SeqCst), 1);
        for batches in [
            mir_host.batches.lock().unwrap(),
            ast_host.batches.lock().unwrap(),
        ] {
            assert_eq!(batches.len(), 1);
            let (url, queries, transaction) = &batches[0];
            assert_eq!(url, database_url);
            assert!(*transaction);
            assert_eq!(queries.len(), 2);
            assert!(queries[0].sql.contains("INSERT INTO `_zelyra_outbox`"));
            assert_eq!(queries[0].params.len(), 3);
            assert!(matches!(
                queries[0].params[0].1,
                zelyra_database::QueryValue::String(ref event_id)
                    if event_id == "00000000000000000000000000000000"
            ));
            assert!(matches!(
                queries[0].params[1].1,
                zelyra_database::QueryValue::String(ref event_type)
                    if event_type == "zelyra.mail.send"
            ));
            assert!(matches!(
                queries[0].params[2].1,
                zelyra_database::QueryValue::String(ref payload)
                    if payload.contains("customer@example.test")
                        && payload.contains("Invoice ready")
            ));
            assert!(queries[1].sql.contains("UPDATE accounts"));
        }

        let denied_host = host();
        let denied = execute_function_with_database_and_host_services_and_policy(
            &mir,
            "enqueue",
            arguments(),
            ExecutionLimits::default(),
            Some(&HashSet::from(["Database(write)".to_owned()])),
            None,
            DatabaseExecution {
                url: database_url,
                host_services: denied_host.clone(),
            },
        )
        .unwrap_err();
        assert!(denied.message.contains("`Email` is not granted"));
        assert!(denied_host.batches.lock().unwrap().is_empty());

        let pdf_attachment = r#"
            database main { engine: mariadb database: "outbox_test" }
            struct Invoice { number: String }
            view InvoicePrint { html { <p>{number}</p> } }
            fn enqueue(invoice: Invoice) uses Email, Database(write) {
                transaction { mail_queue("customer@example.test", "Invoice", "Attached", "invoice.pdf", render_pdf("InvoicePrint", invoice)) }
            }
            fn main() {}
        "#;
        let (ast, mir) = program(pdf_attachment);
        let invoice = || Value::Object {
            type_name: "Invoice".into(),
            type_args: vec![],
            fields: HashMap::from([("number".into(), Value::String("INV-1".into()))]),
        };
        let grants = HashSet::from(["Email".to_owned(), "Database(write)".to_owned()]);
        let database_url = "mariadb://user:secret@127.0.0.1:3306/outbox_test";
        let make_transaction_host = || {
            Arc::new(TransactionHost {
                batches: Mutex::new(Vec::new()),
                validations: AtomicUsize::new(0),
                failure: None,
            })
        };
        let mir_host = make_transaction_host();
        execute_function_with_database_and_host_services_and_policy(
            &mir,
            "enqueue",
            vec![invoice()],
            ExecutionLimits::default(),
            Some(&grants),
            None,
            DatabaseExecution {
                url: database_url,
                host_services: mir_host.clone(),
            },
        )
        .expect("MIR renders PDF attachments into the transactional outbox");
        let ast_host = make_transaction_host();
        zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
            &ast,
            "enqueue",
            vec![invoice()],
            Some(database_url),
            Some(&grants),
            None,
            ast_host.clone(),
        )
        .unwrap();
        let mir_batches = mir_host.batches.lock().unwrap();
        let ast_batches = ast_host.batches.lock().unwrap();
        assert_eq!(mir_batches.len(), 1);
        assert_eq!(ast_batches.len(), 1);
        let (mir_url, mir_queries, mir_transaction) = &mir_batches[0];
        let (ast_url, ast_queries, ast_transaction) = &ast_batches[0];
        assert_eq!(mir_url, ast_url);
        assert_eq!(mir_transaction, ast_transaction);
        assert_eq!(mir_queries.len(), 1);
        assert_eq!(ast_queries.len(), 1);
        assert_eq!(mir_queries[0].sql, ast_queries[0].sql);
        assert_eq!(
            format!("{:?}", mir_queries[0].params),
            format!("{:?}", ast_queries[0].params)
        );
        let outbox_payload = match &mir_batches[0].1[0].params[2].1 {
            zelyra_database::QueryValue::String(payload) => payload,
            value => panic!("expected serialized outbox payload, got {value:?}"),
        };
        assert!(outbox_payload.contains("application/pdf"));
        assert!(
            outbox_payload.contains("JVBER"),
            "PDF bytes are base64 encoded"
        );
    }

    #[test]
    fn environment_and_random_host_services_respect_runtime_grants() {
        let (environment_ast, environment_mir) =
            program("fn main() -> String? uses Environment { return env(\"APP_MODE\") }");
        let environment_host = Arc::new(ClockHost {
            timestamp: 1_700_000_000_000,
            environment: Some("test".into()),
            calls: AtomicUsize::new(0),
        });
        let environment_grants = HashSet::from(["Environment".to_owned()]);
        let actual = execute_function_with_host_services(
            &environment_mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            Some(&environment_grants),
            environment_host,
        )
        .unwrap();
        let expected =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &environment_ast,
                "main",
                vec![],
                None,
                Some(&environment_grants),
                None,
                Arc::new(ClockHost {
                    timestamp: 1_700_000_000_000,
                    environment: Some("test".into()),
                    calls: AtomicUsize::new(0),
                }),
            )
            .unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(
            actual.value,
            Value::Option(Some(Box::new(Value::String("test".into()))))
        );

        let (random_ast, random_mir) =
            program("fn main() -> Int uses Random { return random_int(2, 5) }");
        let random_host = Arc::new(ClockHost {
            timestamp: 1_700_000_000_000,
            environment: None,
            calls: AtomicUsize::new(0),
        });
        let random_grants = HashSet::from(["Random".to_owned()]);
        let actual = execute_function_with_host_services(
            &random_mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            Some(&random_grants),
            random_host,
        )
        .unwrap();
        let expected =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &random_ast,
                "main",
                vec![],
                None,
                Some(&random_grants),
                None,
                Arc::new(ClockHost {
                    timestamp: 1_700_000_000_000,
                    environment: None,
                    calls: AtomicUsize::new(0),
                }),
            )
            .unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::Int(2));

        let (uuid_ast, uuid_mir) = program("fn main() -> String uses Random { return uuid_v4() }");
        let uuid_host = Arc::new(ClockHost {
            timestamp: 1_700_000_000_000,
            environment: None,
            calls: AtomicUsize::new(0),
        });
        let actual = execute_function_with_host_services(
            &uuid_mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            Some(&random_grants),
            uuid_host,
        )
        .unwrap();
        let expected =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &uuid_ast,
                "main",
                vec![],
                None,
                Some(&random_grants),
                None,
                Arc::new(ClockHost {
                    timestamp: 1_700_000_000_000,
                    environment: None,
                    calls: AtomicUsize::new(0),
                }),
            )
            .unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(
            actual.value,
            Value::String("00000000-0000-4000-8000-000000000000".into())
        );
    }

    #[test]
    fn console_host_service_uses_declared_capability_and_project_grant() {
        let (ast, mir) =
            program("fn main() -> String? uses Console { return read_console(\"Name? \") }");
        let grants = HashSet::from(["Console".to_owned()]);
        let host = Arc::new(ConsoleHost {
            line: Some("Ada".into()),
            prompt: Mutex::new(None),
            calls: AtomicUsize::new(0),
        });
        let actual = execute_function_with_host_services(
            &mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            Some(&grants),
            host.clone(),
        )
        .unwrap();
        let expected =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                "main",
                vec![],
                None,
                Some(&grants),
                None,
                Arc::new(ConsoleHost {
                    line: Some("Ada".into()),
                    prompt: Mutex::new(None),
                    calls: AtomicUsize::new(0),
                }),
            )
            .unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(
            actual.value,
            Value::Option(Some(Box::new(Value::String("Ada".into()))))
        );
        assert_eq!(*host.prompt.lock().unwrap(), Some("Name? ".into()));
        assert_eq!(host.calls.load(Ordering::SeqCst), 1);

        let denied_host = Arc::new(ConsoleHost {
            line: Some("Ada".into()),
            prompt: Mutex::new(None),
            calls: AtomicUsize::new(0),
        });
        let denied = execute_function_with_host_services(
            &mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            Some(&HashSet::new()),
            denied_host.clone(),
        )
        .unwrap_err();
        assert!(denied.message.contains("`Console` is not granted"));
        assert_eq!(denied_host.calls.load(Ordering::SeqCst), 0);
        assert_eq!(*denied_host.prompt.lock().unwrap(), None);
    }

    #[test]
    fn process_host_service_obeys_capability_allowlist_and_resource_bounds() {
        let source = "fn main() -> String uses Process { return run_process(\"fake-tool\", [\"--safe\", \"value\"]) }";
        let (ast, mir) = program(source);
        let grants = HashSet::from(["Process".to_owned()]);
        let policy = RuntimePolicy {
            process: Some(ProcessPolicy {
                allowed_commands: vec!["fake-tool".into()],
                timeout_ms: 250,
                max_output_bytes: 4096,
            }),
            ..RuntimePolicy::default()
        };
        let host = Arc::new(ProcessHost {
            invocation: Mutex::new(None),
            calls: AtomicUsize::new(0),
        });
        let actual = execute_function_with_host_services_and_policy(
            &mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            Some(&grants),
            Some(&policy),
            host.clone(),
        )
        .unwrap();
        let expected =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                "main",
                vec![],
                None,
                Some(&grants),
                Some(&policy),
                Arc::new(ProcessHost {
                    invocation: Mutex::new(None),
                    calls: AtomicUsize::new(0),
                }),
            )
            .unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::String("safe output".into()));
        assert_eq!(host.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            *host.invocation.lock().unwrap(),
            Some((
                "fake-tool".into(),
                vec!["--safe".into(), "value".into()],
                250,
                4096,
            ))
        );

        let denied_host = Arc::new(ProcessHost {
            invocation: Mutex::new(None),
            calls: AtomicUsize::new(0),
        });
        let denied_policy = RuntimePolicy {
            process: Some(ProcessPolicy {
                allowed_commands: vec!["another-tool".into()],
                timeout_ms: 250,
                max_output_bytes: 4096,
            }),
            ..RuntimePolicy::default()
        };
        let denied = execute_function_with_host_services_and_policy(
            &mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            Some(&grants),
            Some(&denied_policy),
            denied_host.clone(),
        )
        .unwrap_err();
        assert!(denied.message.contains("denied for command `fake-tool`"));
        assert_eq!(denied_host.calls.load(Ordering::SeqCst), 0);

        let no_policy_host = Arc::new(ProcessHost {
            invocation: Mutex::new(None),
            calls: AtomicUsize::new(0),
        });
        let denied = execute_function_with_host_services(
            &mir,
            "main",
            vec![],
            ExecutionLimits::default(),
            Some(&grants),
            no_policy_host.clone(),
        )
        .unwrap_err();
        assert!(denied.message.contains("no process policy is configured"));
        assert_eq!(no_policy_host.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn filesystem_host_services_share_capability_and_root_checks_with_ast_runtime() {
        let source = "fn main(path: String) -> String uses FileSystem { if file_exists(path) { return read_text(path) } else { return \"missing\" } } fn has_file(path: String) -> Bool uses FileSystem { return file_exists(path) } fn has_dir(path: String) -> Bool uses FileSystem { return dir_exists(\".\") } fn file_len(path: String) -> Result<Int, String> uses FileSystem { return file_size(path) } fn files() -> String[] uses FileSystem { return list_dir(\".\") } fn read_result(path: String) -> Result<String, String> uses FileSystem { return read_text_result(path) }";
        let (ast, mir) = program(source);
        let directory = std::env::temp_dir().join(format!(
            "zelyra-mir-filesystem-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("invoice.txt"), "invoice data\n").unwrap();
        let outside_file = directory.with_extension("outside");
        std::fs::write(&outside_file, "outside\n").unwrap();

        let grants = HashSet::from(["FileSystem".to_owned()]);
        let policy = RuntimePolicy {
            filesystem: Some(FileSystemPolicy {
                base_dir: directory.clone(),
                read_roots: vec![directory.clone()],
                write_roots: Vec::new(),
            }),
            ..RuntimePolicy::default()
        };
        let actual = execute_function_with_host_services_and_policy(
            &mir,
            "main",
            vec![Value::String("invoice.txt".into())],
            ExecutionLimits::default(),
            Some(&grants),
            Some(&policy),
            Arc::new(SystemHostServices),
        )
        .unwrap();
        let expected =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                "main",
                vec![Value::String("invoice.txt".into())],
                None,
                Some(&grants),
                Some(&policy),
                Arc::new(SystemHostServices),
            )
            .unwrap();
        assert_eq!(actual.value, expected);
        assert_eq!(actual.value, Value::String("invoice data\n".into()));

        for (name, arguments, expected_value) in [
            (
                "has_file",
                vec![Value::String("invoice.txt".into())],
                Value::Bool(true),
            ),
            (
                "has_dir",
                vec![Value::String("invoice.txt".into())],
                Value::Bool(true),
            ),
            (
                "file_len",
                vec![Value::String("invoice.txt".into())],
                Value::Result(Ok(Box::new(Value::Int(13)))),
            ),
            (
                "files",
                vec![],
                Value::Array(vec![Value::String("invoice.txt".into())]),
            ),
            (
                "read_result",
                vec![Value::String("invoice.txt".into())],
                Value::Result(Ok(Box::new(Value::String("invoice data\n".into())))),
            ),
        ] {
            let actual = execute_function_with_host_services_and_policy(
                &mir,
                name,
                arguments.clone(),
                ExecutionLimits::default(),
                Some(&grants),
                Some(&policy),
                Arc::new(SystemHostServices),
            )
            .unwrap();
            let expected =
                zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                    &ast,
                    name,
                    arguments,
                    None,
                    Some(&grants),
                    Some(&policy),
                    Arc::new(SystemHostServices),
                )
                .unwrap();
            assert_eq!(actual.value, expected, "{name} differential result");
            assert_eq!(actual.value, expected_value, "{name} expected output");
        }

        let denied = execute_function_with_host_services_and_policy(
            &mir,
            "main",
            vec![Value::String("invoice.txt".into())],
            ExecutionLimits::default(),
            Some(&HashSet::new()),
            Some(&policy),
            Arc::new(SystemHostServices),
        )
        .unwrap_err();
        assert!(denied.message.contains("`FileSystem` is not granted"));

        let (outside_ast, outside_mir) =
            program("fn main(path: String) -> String uses FileSystem { return read_text(path) }");
        let outside_relative =
            format!("../{}", outside_file.file_name().unwrap().to_string_lossy());
        let denied_outside_root = execute_function_with_host_services_and_policy(
            &outside_mir,
            "main",
            vec![Value::String(outside_relative.clone())],
            ExecutionLimits::default(),
            Some(&grants),
            Some(&policy),
            Arc::new(SystemHostServices),
        )
        .unwrap_err();
        assert!(
            denied_outside_root
                .message
                .contains("outside the configured"),
            "{}",
            denied_outside_root.message
        );

        let outside_reference =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &outside_ast,
                "main",
                vec![Value::String(outside_relative)],
                None,
                Some(&grants),
                Some(&policy),
                Arc::new(SystemHostServices),
            )
            .unwrap_err();
        assert_eq!(denied_outside_root.message, outside_reference.message);
        std::fs::remove_dir_all(directory).unwrap();
        std::fs::remove_file(outside_file).unwrap();
    }

    #[test]
    fn filesystem_write_results_use_host_services_and_configured_write_roots() {
        let source = "fn main() {} fn overwrite(path: String, content: String) -> Result<Unit, String> uses FileSystem { return write_text_result(path, content) } fn create(path: String, content: String) -> Result<Unit, String> uses FileSystem { return write_text_new_result(path, content) } fn replace(path: String, content: String) -> Result<Unit, String> uses FileSystem { return atomic_write_text_result(path, content) } fn append(path: String, content: String) -> Result<Unit, String> uses FileSystem { return append_text_result(path, content) } fn copy_file(source_path: String, destination: String) -> Result<Unit, String> uses FileSystem { return copy_file_new_result(source_path, destination) } fn rename_file(source_path: String, destination: String) -> Result<Unit, String> uses FileSystem { return rename_file_result(source_path, destination) } fn delete(path: String) -> Result<Unit, String> uses FileSystem { return delete_file_result(path) } fn direct_write(path: String, content: String) -> Unit uses FileSystem { write_text(path, content) } fn direct_delete(path: String) -> Unit uses FileSystem { delete_file(path) }";
        let (ast, mir) = program(source);
        let base = std::env::temp_dir().join(format!(
            "zelyra-mir-filesystem-write-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let ast_root = base.join("ast");
        let mir_root = base.join("mir");
        std::fs::create_dir_all(&ast_root).unwrap();
        std::fs::create_dir_all(&mir_root).unwrap();
        std::fs::write(ast_root.join("invoice.txt"), "before").unwrap();
        std::fs::write(mir_root.join("invoice.txt"), "before").unwrap();
        for root in [&ast_root, &mir_root] {
            std::fs::write(root.join("copy-source.txt"), "copied").unwrap();
            std::fs::write(root.join("move-source.txt"), "renamed").unwrap();
            std::fs::write(root.join("delete-me.txt"), "deleted").unwrap();
            std::fs::write(root.join("delete-direct.txt"), "deleted").unwrap();
        }

        let grants = HashSet::from(["FileSystem".to_owned()]);
        for (name, path, content) in [
            ("overwrite", "new.txt", "first"),
            ("create", "created.txt", "created"),
            ("replace", "invoice.txt", "atomic"),
            ("append", "invoice.txt", "+tail"),
        ] {
            let ast_policy = RuntimePolicy {
                filesystem: Some(FileSystemPolicy {
                    base_dir: ast_root.clone(),
                    read_roots: vec![ast_root.clone()],
                    write_roots: vec![ast_root.clone()],
                }),
                ..RuntimePolicy::default()
            };
            let mir_policy = RuntimePolicy {
                filesystem: Some(FileSystemPolicy {
                    base_dir: mir_root.clone(),
                    read_roots: vec![mir_root.clone()],
                    write_roots: vec![mir_root.clone()],
                }),
                ..RuntimePolicy::default()
            };
            let arguments = vec![Value::String(path.into()), Value::String(content.into())];
            let expected =
                zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                    &ast,
                    name,
                    arguments.clone(),
                    None,
                    Some(&grants),
                    Some(&ast_policy),
                    Arc::new(SystemHostServices),
                )
                .unwrap();
            let actual = execute_function_with_host_services_and_policy(
                &mir,
                name,
                arguments,
                ExecutionLimits::default(),
                Some(&grants),
                Some(&mir_policy),
                Arc::new(SystemHostServices),
            )
            .unwrap();
            assert_eq!(actual.value, expected, "{name} differential result");
            assert_eq!(actual.value, Value::Result(Ok(Box::new(Value::Unit))));
        }
        for (name, source, destination) in [
            ("copy_file", "copy-source.txt", "copy-destination.txt"),
            ("rename_file", "move-source.txt", "move-destination.txt"),
        ] {
            let ast_policy = RuntimePolicy {
                filesystem: Some(FileSystemPolicy {
                    base_dir: ast_root.clone(),
                    read_roots: vec![ast_root.clone()],
                    write_roots: vec![ast_root.clone()],
                }),
                ..RuntimePolicy::default()
            };
            let mir_policy = RuntimePolicy {
                filesystem: Some(FileSystemPolicy {
                    base_dir: mir_root.clone(),
                    read_roots: vec![mir_root.clone()],
                    write_roots: vec![mir_root.clone()],
                }),
                ..RuntimePolicy::default()
            };
            let arguments = vec![
                Value::String(source.into()),
                Value::String(destination.into()),
            ];
            let expected =
                zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                    &ast,
                    name,
                    arguments.clone(),
                    None,
                    Some(&grants),
                    Some(&ast_policy),
                    Arc::new(SystemHostServices),
                )
                .unwrap();
            let actual = execute_function_with_host_services_and_policy(
                &mir,
                name,
                arguments,
                ExecutionLimits::default(),
                Some(&grants),
                Some(&mir_policy),
                Arc::new(SystemHostServices),
            )
            .unwrap();
            assert_eq!(actual.value, expected, "{name} differential result");
            assert_eq!(actual.value, Value::Result(Ok(Box::new(Value::Unit))));
        }
        let (name, path) = ("delete", "delete-me.txt");
        let ast_policy = RuntimePolicy {
            filesystem: Some(FileSystemPolicy {
                base_dir: ast_root.clone(),
                read_roots: vec![ast_root.clone()],
                write_roots: vec![ast_root.clone()],
            }),
            ..RuntimePolicy::default()
        };
        let mir_policy = RuntimePolicy {
            filesystem: Some(FileSystemPolicy {
                base_dir: mir_root.clone(),
                read_roots: vec![mir_root.clone()],
                write_roots: vec![mir_root.clone()],
            }),
            ..RuntimePolicy::default()
        };
        let arguments = vec![Value::String(path.into())];
        let expected =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                name,
                arguments.clone(),
                None,
                Some(&grants),
                Some(&ast_policy),
                Arc::new(SystemHostServices),
            )
            .unwrap();
        let actual = execute_function_with_host_services_and_policy(
            &mir,
            name,
            arguments,
            ExecutionLimits::default(),
            Some(&grants),
            Some(&mir_policy),
            Arc::new(SystemHostServices),
        )
        .unwrap();
        assert_eq!(actual.value, expected, "{name} differential result");
        assert_eq!(actual.value, Value::Result(Ok(Box::new(Value::Unit))));
        for (name, arguments) in [
            (
                "direct_write",
                vec![
                    Value::String("direct.txt".into()),
                    Value::String("direct".into()),
                ],
            ),
            (
                "direct_delete",
                vec![Value::String("delete-direct.txt".into())],
            ),
        ] {
            let ast_policy = RuntimePolicy {
                filesystem: Some(FileSystemPolicy {
                    base_dir: ast_root.clone(),
                    read_roots: vec![ast_root.clone()],
                    write_roots: vec![ast_root.clone()],
                }),
                ..RuntimePolicy::default()
            };
            let mir_policy = RuntimePolicy {
                filesystem: Some(FileSystemPolicy {
                    base_dir: mir_root.clone(),
                    read_roots: vec![mir_root.clone()],
                    write_roots: vec![mir_root.clone()],
                }),
                ..RuntimePolicy::default()
            };
            let expected =
                zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                    &ast,
                    name,
                    arguments.clone(),
                    None,
                    Some(&grants),
                    Some(&ast_policy),
                    Arc::new(SystemHostServices),
                )
                .unwrap();
            let actual = execute_function_with_host_services_and_policy(
                &mir,
                name,
                arguments,
                ExecutionLimits::default(),
                Some(&grants),
                Some(&mir_policy),
                Arc::new(SystemHostServices),
            )
            .unwrap();
            assert_eq!(actual.value, expected, "{name} differential result");
            assert_eq!(actual.value, Value::Unit);
        }
        for path in ["new.txt", "created.txt", "invoice.txt"] {
            assert_eq!(
                std::fs::read(ast_root.join(path)).unwrap(),
                std::fs::read(mir_root.join(path)).unwrap(),
                "final file content for {path}"
            );
        }
        assert_eq!(
            std::fs::read(mir_root.join("invoice.txt")).unwrap(),
            b"atomic+tail"
        );
        assert_eq!(
            std::fs::read(mir_root.join("copy-destination.txt")).unwrap(),
            b"copied"
        );
        assert_eq!(
            std::fs::read(mir_root.join("direct.txt")).unwrap(),
            b"direct"
        );
        assert_eq!(
            std::fs::read(mir_root.join("move-destination.txt")).unwrap(),
            b"renamed"
        );
        assert!(!mir_root.join("move-source.txt").exists());
        assert!(!mir_root.join("delete-me.txt").exists());
        assert!(!mir_root.join("delete-direct.txt").exists());

        let ast_policy = RuntimePolicy {
            filesystem: Some(FileSystemPolicy {
                base_dir: ast_root.clone(),
                read_roots: vec![ast_root.clone()],
                write_roots: vec![ast_root.clone()],
            }),
            ..RuntimePolicy::default()
        };
        let mir_policy = RuntimePolicy {
            filesystem: Some(FileSystemPolicy {
                base_dir: mir_root.clone(),
                read_roots: vec![mir_root.clone()],
                write_roots: vec![mir_root.clone()],
            }),
            ..RuntimePolicy::default()
        };
        let arguments = vec![
            Value::String("../escape.txt".into()),
            Value::String("blocked".into()),
        ];
        let expected =
            zelyra_runtime::execute_function_with_host_services_and_capabilities_and_policies(
                &ast,
                "overwrite",
                arguments.clone(),
                None,
                Some(&grants),
                Some(&ast_policy),
                Arc::new(SystemHostServices),
            )
            .unwrap();
        let actual = execute_function_with_host_services_and_policy(
            &mir,
            "overwrite",
            arguments,
            ExecutionLimits::default(),
            Some(&grants),
            Some(&mir_policy),
            Arc::new(SystemHostServices),
        )
        .unwrap();
        assert_eq!(actual.value, expected);
        assert!(matches!(actual.value, Value::Result(Err(_))));
        assert!(!base.join("escape.txt").exists());
        std::fs::remove_dir_all(base).unwrap();
    }
}
