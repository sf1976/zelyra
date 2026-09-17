use rand_core::{OsRng, RngCore};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
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

#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityError {
    pub message: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApiDiagnostic {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for ApiDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub const KNOWN_CAPABILITIES: &[&str] = &[
    "Database",
    "Network",
    "FileSystem",
    "Environment",
    "Process",
    "Clock",
    "Random",
];

pub fn check_apis(program: &Program) -> Result<(), Vec<ApiDiagnostic>> {
    let mut errors = Vec::new();
    let mut known_types = [
        "Id",
        "Email",
        "Url",
        "Uuid",
        "Money",
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
        "HttpResponse",
        "HttpError",
        "HttpResult",
    ]
    .into_iter()
    .map(String::from)
    .collect::<HashSet<_>>();
    for definition in &program.types {
        known_types.insert(definition.name.clone());
    }
    for record in &program.records {
        known_types.insert(record.name.clone());
    }
    for table in &program.tables {
        known_types.insert(table.name.clone());
        if let Some(singular) = singular_table_type(&table.name) {
            known_types.insert(singular);
        }
    }

    let mut routes = HashSet::new();
    let functions = program
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function))
        .collect::<HashMap<_, _>>();
    for api in &program.apis {
        if !routes.insert((api.method.clone(), api.path.clone())) {
            errors.push(ApiDiagnostic {
                message: format!("duplicate API route `{}` `{}`", api.method, api.path),
                span: api.span,
            });
        }
        let parameters = match api_path_parameters(&api.path) {
            Ok(parameters) => parameters,
            Err(message) => {
                errors.push(ApiDiagnostic {
                    message,
                    span: api.span,
                });
                Vec::new()
            }
        };
        let mut fields = HashSet::new();
        for field in &api.input {
            if !fields.insert(field.name.clone()) {
                errors.push(ApiDiagnostic {
                    message: format!("duplicate API input field `{}`", field.name),
                    span: field.span,
                });
            }
            if !api_type_known(&field.ty, &known_types) {
                errors.push(ApiDiagnostic {
                    message: format!("unknown API input type `{}`", field.ty),
                    span: field.span,
                });
            }
        }
        for parameter in parameters {
            if !fields.contains(&parameter) {
                errors.push(ApiDiagnostic {
                    message: format!(
                        "path parameter `{parameter}` must be declared in the API input"
                    ),
                    span: api.span,
                });
            }
        }
        if !api_type_known(&api.output, &known_types) {
            errors.push(ApiDiagnostic {
                message: format!("unknown API output type `{}`", api.output),
                span: api.span,
            });
        }
        if let Some(handler) = &api.handler {
            let Some(function) = functions.get(handler.as_str()) else {
                errors.push(ApiDiagnostic {
                    message: format!("API handler function `{handler}` does not exist"),
                    span: api.span,
                });
                continue;
            };
            if function.params.len() != api.input.len() {
                errors.push(ApiDiagnostic {
                    message: format!(
                        "API handler `{handler}` expects {} input field(s), but the API declares {}",
                        function.params.len(),
                        api.input.len()
                    ),
                    span: api.span,
                });
            }
            for (parameter, field) in function.params.iter().zip(&api.input) {
                if parameter.name != field.name || parameter.ty != field.ty {
                    errors.push(ApiDiagnostic {
                        message: format!(
                            "API handler parameter `{}` must match input field `{}` with type `{}`",
                            parameter.name, field.name, field.ty
                        ),
                        span: field.span,
                    });
                }
            }
            if function.return_type.as_ref() != Some(&api.output) {
                errors.push(ApiDiagnostic {
                    message: format!(
                        "API handler `{handler}` must return `{}`, found `{}`",
                        api.output,
                        function
                            .return_type
                            .as_ref()
                            .map_or_else(|| "Unit".into(), ToString::to_string)
                    ),
                    span: api.span,
                });
            }
        }
        let mut statuses = HashSet::new();
        for error in &api.errors {
            if !statuses.insert(error.status) {
                errors.push(ApiDiagnostic {
                    message: format!("duplicate API error status `{}`", error.status),
                    span: error.span,
                });
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn api_type_known(ty: &Type, known_types: &HashSet<String>) -> bool {
    match ty {
        Type::Named(name) => known_types.contains(name),
        Type::Option(inner) | Type::Array(inner) | Type::HttpResult(inner) => {
            api_type_known(inner, known_types)
        }
        Type::Result(ok, error) => {
            api_type_known(ok, known_types) && api_type_known(error, known_types)
        }
        _ => true,
    }
}

fn api_path_parameters(path: &str) -> Result<Vec<String>, String> {
    if !path.starts_with('/') || path.contains(['?', '#']) {
        return Err("API path must start with `/` and must not contain a query or fragment".into());
    }
    let mut parameters = Vec::new();
    for segment in path.split('/').skip(1) {
        if segment.is_empty() {
            continue;
        }
        if let Some(name) = segment
            .strip_prefix('{')
            .and_then(|segment| segment.strip_suffix('}'))
        {
            if name.is_empty()
                || !name.chars().enumerate().all(|(index, character)| {
                    if index == 0 {
                        character == '_' || character.is_ascii_alphabetic()
                    } else {
                        character == '_' || character.is_ascii_alphanumeric()
                    }
                })
            {
                return Err(format!("invalid API path parameter `{segment}`"));
            }
            if parameters.iter().any(|existing| existing == name) {
                return Err(format!("duplicate API path parameter `{name}`"));
            }
            parameters.push(name.to_owned());
        } else if segment.contains(['{', '}']) {
            return Err(format!("invalid API path segment `{segment}`"));
        }
    }
    Ok(parameters)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractKind {
    Requires,
    Ensures,
    LoopInvariant,
}

impl fmt::Display for ContractKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Requires => write!(f, "requires"),
            Self::Ensures => write!(f, "ensures"),
            Self::LoopInvariant => write!(f, "invariant"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationStatus {
    Proven,
    RuntimeCheck,
    Unproven,
    Failed,
}

impl fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = match self {
            Self::Proven => "PROVEN",
            Self::RuntimeCheck => "RUNTIME_CHECK",
            Self::Unproven => "UNPROVEN",
            Self::Failed => "FAILED",
        };
        write!(f, "{status}")
    }
}

impl VerificationStatus {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Proven => "V-001",
            Self::RuntimeCheck => "V-002",
            Self::Unproven => "V-003",
            Self::Failed => "V-004",
        }
    }

    pub const fn explanation(self) -> &'static str {
        match self {
            Self::Proven => "The verifier proved this condition for all analyzed paths.",
            Self::RuntimeCheck => {
                "The verifier could not complete a symbolic proof; runtime checking is required."
            }
            Self::Unproven => "No proof is available for this condition.",
            Self::Failed => "The condition is false on a feasible analyzed path.",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationResult {
    pub function: String,
    pub kind: ContractKind,
    pub index: usize,
    pub status: VerificationStatus,
    pub span: Span,
    pub message: String,
    pub counterexample: Option<Vec<(String, i64)>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VerificationEvidence {
    status: VerificationStatus,
    counterexample: Option<Vec<(String, i64)>>,
}

pub fn verify(program: &Program) -> Vec<VerificationResult> {
    let functions = program
        .functions
        .iter()
        .map(|function| (function.name.clone(), function))
        .collect::<HashMap<_, _>>();
    let mut results = Vec::new();
    for function in &program.functions {
        for (index, contract) in function.requires.iter().enumerate() {
            let evidence = verify_contract(contract, None, &functions);
            results.push(VerificationResult {
                function: function.name.clone(),
                kind: ContractKind::Requires,
                index,
                message: verification_message(
                    ContractKind::Requires,
                    evidence.status,
                    Some(contract),
                ),
                status: evidence.status,
                span: contract.span,
                counterexample: evidence.counterexample,
            });
        }
        let mut invariant_diagnostics = HashMap::new();
        let return_paths =
            symbolic_return_paths(function, Some(&functions), &mut invariant_diagnostics);
        for (index, contract) in function.ensures.iter().enumerate() {
            let evidence = verify_postcondition(
                contract,
                &function.requires,
                return_paths.as_deref(),
                &functions,
            );
            results.push(VerificationResult {
                function: function.name.clone(),
                kind: ContractKind::Ensures,
                index,
                message: verification_message(
                    ContractKind::Ensures,
                    evidence.status,
                    Some(contract),
                ),
                status: evidence.status,
                span: contract.span,
                counterexample: evidence.counterexample,
            });
        }
        let mut loop_invariants = Vec::new();
        collect_loop_invariants(&function.body, &mut loop_invariants);
        for (loop_id, invariants) in loop_invariants {
            for (index, invariant) in invariants.iter().enumerate() {
                let evidence = invariant_diagnostics
                    .get(&(loop_id, index))
                    .cloned()
                    .unwrap_or_else(|| verify_contract(invariant, None, &functions));
                results.push(VerificationResult {
                    function: function.name.clone(),
                    kind: ContractKind::LoopInvariant,
                    index,
                    message: verification_message(
                        ContractKind::LoopInvariant,
                        evidence.status,
                        Some(invariant),
                    ),
                    status: evidence.status,
                    span: invariant.span,
                    counterexample: evidence.counterexample,
                });
            }
        }
        if function.requires.is_empty() && function.ensures.is_empty() {
            results.push(VerificationResult {
                function: function.name.clone(),
                kind: ContractKind::Requires,
                index: 0,
                status: VerificationStatus::Unproven,
                span: function.span,
                message: verification_message(
                    ContractKind::Requires,
                    VerificationStatus::Unproven,
                    None,
                ),
                counterexample: None,
            });
        }
    }
    results
}

fn verification_message(
    kind: ContractKind,
    status: VerificationStatus,
    expression: Option<&Expr>,
) -> String {
    if status == VerificationStatus::Failed
        && expression.is_some_and(|expression| {
            matches!(constant_value(expression), Some(ConstantValue::Bool(false)))
        })
    {
        return "Constant contradiction: this condition evaluates to false for every input.".into();
    }
    match (kind, status, expression.is_some()) {
        (_, VerificationStatus::Proven, _) => status.explanation().into(),
        (ContractKind::Requires, VerificationStatus::RuntimeCheck, _) => {
            "This precondition needs a runtime check because the symbolic proof is incomplete."
                .into()
        }
        (ContractKind::Ensures, VerificationStatus::RuntimeCheck, _) => {
            "This postcondition needs a runtime check because not all return paths are symbolically modeled."
                .into()
        }
        (ContractKind::LoopInvariant, VerificationStatus::RuntimeCheck, _) => {
            "This loop invariant needs a runtime check because entry or preservation could not be proved symbolically."
                .into()
        }
        (ContractKind::Requires, VerificationStatus::Failed, _) => {
            "The precondition is false for a feasible checked model.".into()
        }
        (ContractKind::Ensures, VerificationStatus::Failed, _) => {
            "The postcondition is false on a feasible return path.".into()
        }
        (ContractKind::LoopInvariant, VerificationStatus::Failed, _) => {
            "The loop invariant is false on a feasible path or is not preserved by the loop body."
                .into()
        }
        (_, VerificationStatus::Unproven, false) => {
            "No proof target was declared for this function.".into()
        }
        (_, VerificationStatus::Unproven, true) => {
            "No proof is available for this condition.".into()
        }
    }
}

fn verify_contract(
    contract: &Expr,
    return_expression: Option<&Expr>,
    functions: &HashMap<String, &Function>,
) -> VerificationEvidence {
    let status =
        match constant_value(contract) {
            Some(ConstantValue::Bool(true)) => VerificationStatus::Proven,
            Some(ConstantValue::Bool(false)) => VerificationStatus::Failed,
            Some(_) => VerificationStatus::Unproven,
            None => symbolic_bool(contract, return_expression, None, None, Some(functions), 0)
                .map_or(VerificationStatus::RuntimeCheck, |value| {
                    if value {
                        VerificationStatus::Proven
                    } else {
                        VerificationStatus::Failed
                    }
                }),
        };
    let counterexample = if status == VerificationStatus::Failed
        && !matches!(constant_value(contract), Some(ConstantValue::Bool(false)))
    {
        find_symbolic_counterexample(
            contract,
            return_expression,
            &[],
            &HashMap::new(),
            &HashMap::new(),
            Some(functions),
        )
    } else {
        None
    };
    VerificationEvidence {
        status,
        counterexample,
    }
}

fn verify_postcondition(
    contract: &Expr,
    preconditions: &[Expr],
    return_paths: Option<&[ReturnPath<'_>]>,
    functions: &HashMap<String, &Function>,
) -> VerificationEvidence {
    match constant_value(contract) {
        Some(ConstantValue::Bool(true)) => VerificationEvidence {
            status: VerificationStatus::Proven,
            counterexample: None,
        },
        Some(ConstantValue::Bool(false)) => VerificationEvidence {
            status: VerificationStatus::Failed,
            counterexample: None,
        },
        Some(_) => VerificationEvidence {
            status: VerificationStatus::Unproven,
            counterexample: None,
        },
        None => {
            let Some(return_paths) = return_paths else {
                return VerificationEvidence {
                    status: VerificationStatus::RuntimeCheck,
                    counterexample: None,
                };
            };
            let mut saw_feasible_path = false;
            let mut saw_unknown_path = false;
            for path in return_paths {
                let Some(expression) = path.expression else {
                    saw_unknown_path = true;
                    continue;
                };
                let mut alternatives = vec![Vec::new()];
                for precondition in preconditions {
                    let guard = SymbolicGuard::Condition {
                        expression: precondition,
                        expected: true,
                    };
                    let Some(precondition_alternatives) = constraints_for_guard(
                        &guard,
                        Some(expression),
                        Some(&path.bindings),
                        Some(&path.substitutions),
                        Some(functions),
                        0,
                    ) else {
                        saw_unknown_path = true;
                        alternatives.clear();
                        break;
                    };
                    alternatives = combine_alternatives(alternatives, precondition_alternatives);
                }
                if alternatives.is_empty() {
                    continue;
                }
                for guard in &path.guards {
                    let Some(guard_alternatives) = constraints_for_guard(
                        guard,
                        Some(expression),
                        Some(&path.bindings),
                        Some(&path.substitutions),
                        Some(functions),
                        0,
                    ) else {
                        saw_unknown_path = true;
                        alternatives.clear();
                        break;
                    };
                    alternatives = combine_alternatives(alternatives, guard_alternatives);
                }
                for guards in alternatives {
                    if constraints_satisfiable(guards.clone()) == Some(false) {
                        continue;
                    }
                    saw_feasible_path = true;
                    let context = SymbolicContext {
                        return_expression: Some(expression),
                        bindings: Some(&path.bindings),
                        substitutions: Some(&path.substitutions),
                        functions: Some(functions),
                        depth: 0,
                    };
                    if call_preconditions_hold(expression, &guards, context) != Some(true)
                        || call_preconditions_hold(contract, &guards, context) != Some(true)
                    {
                        saw_unknown_path = true;
                        continue;
                    }
                    match symbolic_bool_with_constraints(
                        contract,
                        expression,
                        &guards,
                        Some(&path.bindings),
                        Some(&path.substitutions),
                        Some(functions),
                        0,
                    ) {
                        Some(true) => {}
                        Some(false) => {
                            return VerificationEvidence {
                                status: VerificationStatus::Failed,
                                counterexample: find_symbolic_counterexample(
                                    contract,
                                    Some(expression),
                                    &guards,
                                    &path.bindings,
                                    &path.substitutions,
                                    Some(functions),
                                ),
                            }
                        }
                        None => saw_unknown_path = true,
                    }
                }
            }
            if saw_feasible_path && !saw_unknown_path {
                VerificationEvidence {
                    status: VerificationStatus::Proven,
                    counterexample: None,
                }
            } else {
                VerificationEvidence {
                    status: VerificationStatus::RuntimeCheck,
                    counterexample: None,
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
enum SymbolicGuard<'a> {
    Condition {
        expression: &'a Expr,
        expected: bool,
    },
    ConditionSnapshot {
        expression: &'a Expr,
        expected: bool,
        substitutions: HashMap<String, LinearValue>,
    },
    Match {
        value: &'a Expr,
        pattern: &'a Pattern,
        matched: bool,
    },
}

#[derive(Clone, Debug)]
struct ReturnPath<'a> {
    guards: Vec<SymbolicGuard<'a>>,
    expression: Option<&'a Expr>,
    bindings: HashMap<String, &'a Expr>,
    substitutions: HashMap<String, LinearValue>,
}

type InvariantDiagnostics = HashMap<(usize, usize), VerificationEvidence>;

struct LoopSymbolicContext<'a> {
    parameters: &'a HashSet<String>,
    functions: Option<&'a HashMap<String, &'a Function>>,
    diagnostics: &'a mut InvariantDiagnostics,
}

#[derive(Clone, Debug)]
enum SymbolicState<'a> {
    Continue {
        guards: Vec<SymbolicGuard<'a>>,
        bindings: HashMap<String, &'a Expr>,
        substitutions: HashMap<String, LinearValue>,
    },
    Break {
        guards: Vec<SymbolicGuard<'a>>,
        bindings: HashMap<String, &'a Expr>,
        substitutions: HashMap<String, LinearValue>,
    },
    LoopContinue {
        guards: Vec<SymbolicGuard<'a>>,
        bindings: HashMap<String, &'a Expr>,
        substitutions: HashMap<String, LinearValue>,
    },
    Return {
        guards: Vec<SymbolicGuard<'a>>,
        expression: Option<&'a Expr>,
        bindings: HashMap<String, &'a Expr>,
        substitutions: HashMap<String, LinearValue>,
    },
}

fn symbolic_return_paths<'a>(
    function: &'a Function,
    functions: Option<&HashMap<String, &Function>>,
    diagnostics: &mut InvariantDiagnostics,
) -> Option<Vec<ReturnPath<'a>>> {
    let parameters = function
        .params
        .iter()
        .map(|parameter| parameter.name.clone())
        .collect::<HashSet<_>>();
    let initial_guards = function
        .requires
        .iter()
        .map(|expression| SymbolicGuard::Condition {
            expression,
            expected: true,
        })
        .collect();
    let states = symbolic_states(
        &function.body,
        initial_guards,
        HashMap::new(),
        HashMap::new(),
        &parameters,
        functions,
        diagnostics,
    )?;
    let mut paths = Vec::new();
    for state in states {
        let SymbolicState::Return {
            guards,
            expression,
            bindings,
            substitutions,
        } = state
        else {
            return None;
        };
        paths.push(ReturnPath {
            guards,
            expression,
            bindings,
            substitutions,
        });
    }
    Some(paths)
}

fn direct_return_expression(function: &Function) -> Option<&Expr> {
    let [Stmt::Return {
        value: Some(expression),
        ..
    }] = function.body.statements.as_slice()
    else {
        return None;
    };
    Some(expression)
}

fn add_pattern_bindings<'a>(
    value: &'a Expr,
    pattern: &'a Pattern,
    bindings: &mut HashMap<String, &'a Expr>,
) {
    match &pattern.kind {
        PatternKind::Variable(name) => {
            if !matches!(&value.kind, ExprKind::Variable(existing) if existing == name) {
                bindings.insert(name.clone(), value);
            }
        }
        PatternKind::Constructor { name, inner } => {
            let Some(inner) = inner else {
                return;
            };
            let ExprKind::Call {
                name: value_name,
                args,
                ..
            } = &value.kind
            else {
                return;
            };
            if value_name == name && args.len() == 1 {
                add_pattern_bindings(&args[0], inner, bindings);
            }
        }
        PatternKind::Wildcard
        | PatternKind::Int(_)
        | PatternKind::Bool(_)
        | PatternKind::String(_)
        | PatternKind::Char(_) => {}
    }
}

fn symbolic_states<'a>(
    block: &'a Block,
    incoming: Vec<SymbolicGuard<'a>>,
    bindings: HashMap<String, &'a Expr>,
    substitutions: HashMap<String, LinearValue>,
    parameters: &HashSet<String>,
    functions: Option<&HashMap<String, &Function>>,
    diagnostics: &mut InvariantDiagnostics,
) -> Option<Vec<SymbolicState<'a>>> {
    let mut states = vec![SymbolicState::Continue {
        guards: incoming,
        bindings,
        substitutions,
    }];
    for statement in &block.statements {
        let mut next = Vec::new();
        for state in states {
            match state {
                SymbolicState::Return { .. }
                | SymbolicState::Break { .. }
                | SymbolicState::LoopContinue { .. } => next.push(state),
                SymbolicState::Continue {
                    guards,
                    bindings,
                    substitutions,
                } => match statement {
                    Stmt::Let {
                        name,
                        value,
                        mutable: false,
                        ..
                    } if !expression_contains_variable(value, name) => {
                        let mut bindings = bindings;
                        bindings.insert(name.clone(), value);
                        next.push(SymbolicState::Continue {
                            guards,
                            bindings,
                            substitutions,
                        });
                    }
                    Stmt::Let {
                        name,
                        value,
                        mutable: true,
                        ..
                    } => {
                        let value = linear_value(
                            value,
                            None,
                            Some(&bindings),
                            Some(&substitutions),
                            functions,
                            0,
                        )?;
                        let mut substitutions = substitutions;
                        substitutions.insert(name.clone(), value);
                        next.push(SymbolicState::Continue {
                            guards,
                            bindings,
                            substitutions,
                        });
                    }
                    Stmt::BindOrAssign { name, value, .. } if substitutions.contains_key(name) => {
                        let value = linear_value(
                            value,
                            None,
                            Some(&bindings),
                            Some(&substitutions),
                            functions,
                            0,
                        )?;
                        let mut substitutions = substitutions;
                        substitutions.insert(name.clone(), value);
                        next.push(SymbolicState::Continue {
                            guards,
                            bindings,
                            substitutions,
                        });
                    }
                    Stmt::BindOrAssign { name, value, .. }
                        if !parameters.contains(name)
                            && !bindings.contains_key(name)
                            && !substitutions.contains_key(name)
                            && !expression_contains_variable(value, name) =>
                    {
                        let mut bindings = bindings;
                        bindings.insert(name.clone(), value);
                        next.push(SymbolicState::Continue {
                            guards,
                            bindings,
                            substitutions,
                        });
                    }
                    Stmt::Return { value, .. } => next.push(SymbolicState::Return {
                        guards,
                        expression: value.as_ref(),
                        bindings,
                        substitutions,
                    }),
                    Stmt::Break { .. } => next.push(SymbolicState::Break {
                        guards,
                        bindings,
                        substitutions,
                    }),
                    Stmt::Continue { .. } => next.push(SymbolicState::LoopContinue {
                        guards,
                        bindings,
                        substitutions,
                    }),
                    Stmt::If {
                        condition,
                        then_block,
                        else_block,
                        ..
                    } => {
                        let mut then_guards = guards.clone();
                        then_guards.push(SymbolicGuard::ConditionSnapshot {
                            expression: condition,
                            expected: true,
                            substitutions: substitutions.clone(),
                        });
                        next.extend(symbolic_states(
                            then_block,
                            then_guards,
                            bindings.clone(),
                            substitutions.clone(),
                            parameters,
                            functions,
                            diagnostics,
                        )?);

                        if let Some(else_block) = else_block {
                            let mut else_guards = guards;
                            else_guards.push(SymbolicGuard::ConditionSnapshot {
                                expression: condition,
                                expected: false,
                                substitutions: substitutions.clone(),
                            });
                            next.extend(symbolic_states(
                                else_block,
                                else_guards,
                                bindings.clone(),
                                substitutions.clone(),
                                parameters,
                                functions,
                                diagnostics,
                            )?);
                        } else {
                            let mut guards = guards;
                            guards.push(SymbolicGuard::ConditionSnapshot {
                                expression: condition,
                                expected: false,
                                substitutions: substitutions.clone(),
                            });
                            next.push(SymbolicState::Continue {
                                guards,
                                bindings,
                                substitutions,
                            });
                        }
                    }
                    Stmt::Match { value, arms, .. } => {
                        for (index, arm) in arms.iter().enumerate() {
                            let mut arm_guards = guards.clone();
                            for previous in arms.iter().take(index) {
                                arm_guards.push(SymbolicGuard::Match {
                                    value,
                                    pattern: &previous.pattern,
                                    matched: false,
                                });
                            }
                            arm_guards.push(SymbolicGuard::Match {
                                value,
                                pattern: &arm.pattern,
                                matched: true,
                            });
                            let mut arm_bindings = bindings.clone();
                            add_pattern_bindings(value, &arm.pattern, &mut arm_bindings);
                            next.extend(symbolic_states(
                                &arm.body,
                                arm_guards,
                                arm_bindings,
                                substitutions.clone(),
                                parameters,
                                functions,
                                diagnostics,
                            )?);
                        }
                    }
                    Stmt::While {
                        condition,
                        invariants,
                        body,
                        span,
                    } => {
                        let mut context = LoopSymbolicContext {
                            parameters,
                            functions,
                            diagnostics,
                        };
                        next.extend(symbolic_loop_states(
                            condition,
                            invariants,
                            span.start,
                            body,
                            SymbolicState::Continue {
                                guards,
                                bindings,
                                substitutions,
                            },
                            &mut context,
                            0,
                        )?);
                    }
                    Stmt::Loop {
                        invariants,
                        body,
                        span,
                    } => {
                        let mut context = LoopSymbolicContext {
                            parameters,
                            functions,
                            diagnostics,
                        };
                        next.extend(symbolic_unconditional_loop_states(
                            invariants,
                            span.start,
                            body,
                            SymbolicState::Continue {
                                guards,
                                bindings,
                                substitutions,
                            },
                            &mut context,
                            0,
                        )?);
                    }
                    _ => return None,
                },
            }
        }
        states = next;
    }
    Some(states)
}

fn symbolic_loop_states<'a>(
    condition: &'a Expr,
    invariants: &'a [Expr],
    loop_id: usize,
    body: &'a Block,
    state: SymbolicState<'a>,
    context: &mut LoopSymbolicContext<'_>,
    iterations: usize,
) -> Option<Vec<SymbolicState<'a>>> {
    let SymbolicState::Continue {
        guards,
        bindings,
        substitutions,
    } = state
    else {
        return None;
    };
    for (index, invariant) in invariants.iter().enumerate() {
        let evidence = invariant_status(
            invariant,
            &guards,
            &bindings,
            &substitutions,
            context.functions,
        );
        record_invariant_status(context.diagnostics, loop_id, index, evidence.clone());
        if evidence.status != VerificationStatus::Proven {
            return None;
        }
    }
    let true_constraints = constraints_for_bool(
        condition,
        true,
        None,
        Some(&bindings),
        Some(&substitutions),
        context.functions,
        0,
    )?;
    let mut exit_guards = guards.clone();
    for invariant in invariants {
        exit_guards.push(SymbolicGuard::ConditionSnapshot {
            expression: invariant,
            expected: true,
            substitutions: substitutions.clone(),
        });
    }
    exit_guards.push(SymbolicGuard::ConditionSnapshot {
        expression: condition,
        expected: false,
        substitutions: substitutions.clone(),
    });
    let mut states = Vec::new();
    let true_possible = true_constraints
        .iter()
        .any(|constraints| constraints_satisfiable(constraints.clone()) != Some(false));
    if !true_possible {
        states.push(SymbolicState::Continue {
            guards: exit_guards,
            bindings,
            substitutions,
        });
        return Some(states);
    }
    if iterations >= 32 {
        if !invariants.is_empty() {
            let mut abstract_substitutions = substitutions.clone();
            let mut modified = HashSet::new();
            collect_loop_modified_names(body, &mut modified);
            for name in modified {
                abstract_substitutions.remove(&name);
            }
            let mut abstract_exit_guards = guards;
            for invariant in invariants {
                abstract_exit_guards.push(SymbolicGuard::ConditionSnapshot {
                    expression: invariant,
                    expected: true,
                    substitutions: abstract_substitutions.clone(),
                });
            }
            abstract_exit_guards.push(SymbolicGuard::ConditionSnapshot {
                expression: condition,
                expected: false,
                substitutions: abstract_substitutions.clone(),
            });
            states.push(SymbolicState::Continue {
                guards: abstract_exit_guards,
                bindings,
                substitutions: abstract_substitutions,
            });
            return Some(states);
        }
        return None;
    }

    let mut body_guards = guards;
    for invariant in invariants {
        body_guards.push(SymbolicGuard::ConditionSnapshot {
            expression: invariant,
            expected: true,
            substitutions: substitutions.clone(),
        });
    }
    body_guards.push(SymbolicGuard::ConditionSnapshot {
        expression: condition,
        expected: true,
        substitutions: substitutions.clone(),
    });
    for state in symbolic_states(
        body,
        body_guards,
        bindings,
        substitutions,
        context.parameters,
        context.functions,
        context.diagnostics,
    )? {
        match state {
            SymbolicState::Return { .. } => states.push(state),
            SymbolicState::Break {
                guards,
                bindings,
                substitutions,
            } => {
                for (index, invariant) in invariants.iter().enumerate() {
                    let evidence = invariant_status(
                        invariant,
                        &guards,
                        &bindings,
                        &substitutions,
                        context.functions,
                    );
                    record_invariant_status(context.diagnostics, loop_id, index, evidence.clone());
                    if evidence.status != VerificationStatus::Proven {
                        return None;
                    }
                }
                states.push(SymbolicState::Continue {
                    guards,
                    bindings,
                    substitutions,
                })
            }
            SymbolicState::LoopContinue {
                guards,
                bindings,
                substitutions,
            } => states.extend(symbolic_loop_states(
                condition,
                invariants,
                loop_id,
                body,
                SymbolicState::Continue {
                    guards,
                    bindings,
                    substitutions,
                },
                context,
                iterations + 1,
            )?),
            SymbolicState::Continue {
                guards,
                bindings,
                substitutions,
            } => {
                for (index, invariant) in invariants.iter().enumerate() {
                    let evidence = invariant_status(
                        invariant,
                        &guards,
                        &bindings,
                        &substitutions,
                        context.functions,
                    );
                    record_invariant_status(context.diagnostics, loop_id, index, evidence.clone());
                    if evidence.status != VerificationStatus::Proven {
                        return None;
                    }
                }
                states.extend(symbolic_loop_states(
                    condition,
                    invariants,
                    loop_id,
                    body,
                    SymbolicState::Continue {
                        guards,
                        bindings,
                        substitutions,
                    },
                    context,
                    iterations + 1,
                )?)
            }
        }
    }
    Some(states)
}

fn symbolic_unconditional_loop_states<'a>(
    invariants: &'a [Expr],
    loop_id: usize,
    body: &'a Block,
    state: SymbolicState<'a>,
    context: &mut LoopSymbolicContext<'_>,
    iterations: usize,
) -> Option<Vec<SymbolicState<'a>>> {
    let SymbolicState::Continue {
        guards,
        bindings,
        substitutions,
    } = state
    else {
        return None;
    };
    for (index, invariant) in invariants.iter().enumerate() {
        let evidence = invariant_status(
            invariant,
            &guards,
            &bindings,
            &substitutions,
            context.functions,
        );
        record_invariant_status(context.diagnostics, loop_id, index, evidence.clone());
        if evidence.status != VerificationStatus::Proven {
            return None;
        }
    }

    let mut body_guards = guards;
    for invariant in invariants {
        body_guards.push(SymbolicGuard::ConditionSnapshot {
            expression: invariant,
            expected: true,
            substitutions: substitutions.clone(),
        });
    }
    let mut states = Vec::new();
    for state in symbolic_states(
        body,
        body_guards,
        bindings,
        substitutions,
        context.parameters,
        context.functions,
        context.diagnostics,
    )? {
        match state {
            SymbolicState::Return { .. } => states.push(state),
            SymbolicState::Break {
                guards,
                bindings,
                substitutions,
            } => {
                for (index, invariant) in invariants.iter().enumerate() {
                    let evidence = invariant_status(
                        invariant,
                        &guards,
                        &bindings,
                        &substitutions,
                        context.functions,
                    );
                    record_invariant_status(context.diagnostics, loop_id, index, evidence.clone());
                    if evidence.status != VerificationStatus::Proven {
                        return None;
                    }
                }
                states.push(SymbolicState::Continue {
                    guards,
                    bindings,
                    substitutions,
                });
            }
            SymbolicState::LoopContinue {
                guards,
                bindings,
                substitutions,
            }
            | SymbolicState::Continue {
                guards,
                bindings,
                substitutions,
            } => {
                if iterations >= 32 {
                    return None;
                }
                states.extend(symbolic_unconditional_loop_states(
                    invariants,
                    loop_id,
                    body,
                    SymbolicState::Continue {
                        guards,
                        bindings,
                        substitutions,
                    },
                    context,
                    iterations + 1,
                )?);
            }
        }
    }
    Some(states)
}

fn collect_loop_modified_names(block: &Block, names: &mut HashSet<String>) {
    for statement in &block.statements {
        match statement {
            Stmt::Let {
                name,
                mutable: true,
                ..
            }
            | Stmt::BindOrAssign { name, .. } => {
                names.insert(name.clone());
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_loop_modified_names(then_block, names);
                if let Some(else_block) = else_block {
                    collect_loop_modified_names(else_block, names);
                }
            }
            Stmt::While { body, .. }
            | Stmt::For { body, .. }
            | Stmt::Loop { body, .. }
            | Stmt::Transaction { body, .. }
            | Stmt::Parallel { body, .. } => collect_loop_modified_names(body, names),
            Stmt::Match { arms, .. } => {
                for arm in arms {
                    collect_loop_modified_names(&arm.body, names);
                }
            }
            Stmt::Let { .. }
            | Stmt::Expr(_)
            | Stmt::Return { .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. } => {}
        }
    }
}

fn collect_loop_invariants<'a>(block: &'a Block, loops: &mut Vec<(usize, &'a [Expr])>) {
    for statement in &block.statements {
        match statement {
            Stmt::While {
                invariants,
                body,
                span,
                ..
            }
            | Stmt::Loop {
                invariants,
                body,
                span,
            } => {
                loops.push((span.start, invariants));
                collect_loop_invariants(body, loops);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_loop_invariants(then_block, loops);
                if let Some(else_block) = else_block {
                    collect_loop_invariants(else_block, loops);
                }
            }
            Stmt::Match { arms, .. } => {
                for arm in arms {
                    collect_loop_invariants(&arm.body, loops);
                }
            }
            Stmt::For { body, .. } => collect_loop_invariants(body, loops),
            Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
                collect_loop_invariants(body, loops)
            }
            Stmt::Let { .. }
            | Stmt::BindOrAssign { .. }
            | Stmt::Expr(_)
            | Stmt::Return { .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. } => {}
        }
    }
}

fn expression_contains_variable(expression: &Expr, name: &str) -> bool {
    match &expression.kind {
        ExprKind::Variable(variable) => variable == name,
        ExprKind::Call { args, .. } => args
            .iter()
            .any(|argument| expression_contains_variable(argument, name)),
        ExprKind::Unary { expr, .. } => expression_contains_variable(expr, name),
        ExprKind::Binary { left, right, .. } => {
            expression_contains_variable(left, name) || expression_contains_variable(right, name)
        }
        ExprKind::Array(values) => values
            .iter()
            .any(|value| expression_contains_variable(value, name)),
        ExprKind::Record { fields, .. } => fields
            .iter()
            .any(|(_, value)| expression_contains_variable(value, name)),
        ExprKind::Index { target, index } => {
            expression_contains_variable(target, name) || expression_contains_variable(index, name)
        }
        ExprKind::Field { target, .. } => expression_contains_variable(target, name),
        ExprKind::Await(inner) => expression_contains_variable(inner, name),
        ExprKind::Int(..)
        | ExprKind::UInt(..)
        | ExprKind::Float(..)
        | ExprKind::Bool(..)
        | ExprKind::String(..)
        | ExprKind::Char(..)
        | ExprKind::Sql { .. } => false,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct LinearValue {
    coefficients: HashMap<String, i64>,
    constant: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct LinearConstraint {
    coefficients: HashMap<String, i128>,
    constant: i128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PathConstraint {
    Linear(LinearConstraint),
    Constructor {
        expression: usize,
        name: String,
        equal: bool,
    },
}

impl LinearConstraint {
    fn from_value(value: LinearValue) -> Self {
        Self {
            coefficients: value
                .coefficients
                .into_iter()
                .map(|(name, coefficient)| (name, i128::from(coefficient)))
                .collect(),
            constant: i128::from(value.constant),
        }
    }

    fn negate(mut self) -> Option<Self> {
        self.constant = self.constant.checked_neg()?;
        for coefficient in self.coefficients.values_mut() {
            *coefficient = coefficient.checked_neg()?;
        }
        Some(self)
    }

    fn shift(mut self, amount: i128) -> Option<Self> {
        self.constant = self.constant.checked_sub(amount)?;
        Some(self)
    }

    fn combine(self, other: Self, self_factor: i128, other_factor: i128) -> Option<Self> {
        let mut combined = Self {
            coefficients: HashMap::new(),
            constant: self
                .constant
                .checked_mul(self_factor)?
                .checked_add(other.constant.checked_mul(other_factor)?)?,
        };
        for (name, coefficient) in self.coefficients {
            let value = coefficient.checked_mul(self_factor)?;
            if value != 0 {
                combined.coefficients.insert(name, value);
            }
        }
        for (name, coefficient) in other.coefficients {
            let value = coefficient.checked_mul(other_factor)?;
            let sum = combined
                .coefficients
                .get(&name)
                .copied()
                .unwrap_or_default()
                .checked_add(value)?;
            if sum == 0 {
                combined.coefficients.remove(&name);
            } else {
                combined.coefficients.insert(name, sum);
            }
        }
        Some(combined)
    }
}

fn combine_alternatives(
    left: Vec<Vec<PathConstraint>>,
    right: Vec<Vec<PathConstraint>>,
) -> Vec<Vec<PathConstraint>> {
    left.into_iter()
        .flat_map(|left_constraints| {
            right.iter().map(move |right_constraints| {
                left_constraints
                    .iter()
                    .cloned()
                    .chain(right_constraints.iter().cloned())
                    .collect()
            })
        })
        .collect()
}

#[derive(Clone, Copy)]
struct SymbolicContext<'a> {
    return_expression: Option<&'a Expr>,
    bindings: Option<&'a HashMap<String, &'a Expr>>,
    substitutions: Option<&'a HashMap<String, LinearValue>>,
    functions: Option<&'a HashMap<String, &'a Function>>,
    depth: usize,
}

fn constraints_for_guard(
    guard: &SymbolicGuard<'_>,
    return_expression: Option<&Expr>,
    bindings: Option<&HashMap<String, &Expr>>,
    substitutions: Option<&HashMap<String, LinearValue>>,
    functions: Option<&HashMap<String, &Function>>,
    depth: usize,
) -> Option<Vec<Vec<PathConstraint>>> {
    match guard {
        SymbolicGuard::Condition {
            expression,
            expected,
        } => constraints_for_bool(
            expression,
            *expected,
            return_expression,
            bindings,
            substitutions,
            functions,
            depth,
        ),
        SymbolicGuard::ConditionSnapshot {
            expression,
            expected,
            substitutions,
        } => constraints_for_bool(
            expression,
            *expected,
            return_expression,
            bindings,
            Some(substitutions),
            functions,
            depth,
        ),
        SymbolicGuard::Match {
            value,
            pattern,
            matched,
        } => constraints_for_pattern(
            value,
            pattern,
            *matched,
            SymbolicContext {
                return_expression,
                bindings,
                substitutions,
                functions,
                depth,
            },
        ),
    }
}

fn constraints_for_pattern(
    value: &Expr,
    pattern: &Pattern,
    matched: bool,
    context: SymbolicContext<'_>,
) -> Option<Vec<Vec<PathConstraint>>> {
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Variable(_) => {
            if matched {
                Some(vec![Vec::new()])
            } else {
                Some(Vec::new())
            }
        }
        PatternKind::Int(expected) => {
            let left = linear_value_alternatives(
                value,
                context.return_expression,
                context.bindings,
                context.substitutions,
                context.functions,
                context.depth,
            )?;
            comparison_constraint_alternatives(
                left,
                vec![(
                    LinearValue {
                        constant: *expected,
                        ..LinearValue::default()
                    },
                    Vec::new(),
                )],
                BinaryOp::Equal,
                matched,
            )
        }
        PatternKind::Bool(expected) => {
            let left = linear_value_alternatives(
                value,
                context.return_expression,
                context.bindings,
                context.substitutions,
                context.functions,
                context.depth,
            )?;
            let right = LinearConstraint {
                coefficients: HashMap::new(),
                constant: i128::from(*expected as u8),
            };
            let mut alternatives = Vec::new();
            for (left_value, left_constraints) in left {
                let left = LinearConstraint::from_value(left_value);
                let domain = [
                    left.clone(),
                    LinearConstraint {
                        coefficients: HashMap::new(),
                        constant: 1,
                    }
                    .combine(left.clone(), 1, -1)?,
                ];
                for comparison in
                    comparison_constraints(left.clone(), BinaryOp::Equal, right.clone(), matched)?
                {
                    alternatives.push(
                        left_constraints
                            .iter()
                            .cloned()
                            .chain(comparison)
                            .chain(domain.iter().cloned().map(PathConstraint::Linear))
                            .collect(),
                    );
                }
            }
            Some(alternatives)
        }
        PatternKind::String(_) | PatternKind::Char(_) => None,
        PatternKind::Constructor { name, .. } => {
            let expression = value as *const Expr as usize;
            let mut constraints = vec![PathConstraint::Constructor {
                expression,
                name: name.clone(),
                equal: matched,
            }];
            if let ExprKind::Call {
                name: known_name,
                args,
                ..
            } = &value.kind
            {
                if args.len() == 1 && matches!(known_name.as_str(), "Some" | "None" | "Ok" | "Err")
                {
                    constraints.push(PathConstraint::Constructor {
                        expression,
                        name: known_name.clone(),
                        equal: true,
                    });
                }
            }
            Some(vec![constraints])
        }
    }
}

fn comparison_constraint_alternatives(
    left: Vec<(LinearValue, Vec<PathConstraint>)>,
    right: Vec<(LinearValue, Vec<PathConstraint>)>,
    operator: BinaryOp,
    expected: bool,
) -> Option<Vec<Vec<PathConstraint>>> {
    let mut alternatives = Vec::new();
    for (left, left_constraints) in left {
        for (right, right_constraints) in &right {
            for comparison in comparison_constraints(
                LinearConstraint::from_value(left.clone()),
                operator,
                LinearConstraint::from_value(right.clone()),
                expected,
            )? {
                alternatives.push(
                    left_constraints
                        .iter()
                        .cloned()
                        .chain(right_constraints.iter().cloned())
                        .chain(comparison)
                        .collect(),
                );
            }
        }
    }
    Some(alternatives)
}

fn constraints_for_bool(
    expression: &Expr,
    expected: bool,
    return_expression: Option<&Expr>,
    bindings: Option<&HashMap<String, &Expr>>,
    substitutions: Option<&HashMap<String, LinearValue>>,
    functions: Option<&HashMap<String, &Function>>,
    depth: usize,
) -> Option<Vec<Vec<PathConstraint>>> {
    if let Some(ConstantValue::Bool(value)) = constant_value(expression) {
        return if value == expected {
            Some(vec![Vec::new()])
        } else {
            Some(Vec::new())
        };
    }
    match &expression.kind {
        ExprKind::Unary {
            op: UnaryOp::Not,
            expr,
        } => constraints_for_bool(
            expr,
            !expected,
            return_expression,
            bindings,
            substitutions,
            functions,
            depth,
        ),
        ExprKind::Binary { left, op, right } => match op {
            BinaryOp::And if expected => Some(combine_alternatives(
                constraints_for_bool(
                    left,
                    true,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?,
                constraints_for_bool(
                    right,
                    true,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?,
            )),
            BinaryOp::Or if !expected => Some(combine_alternatives(
                constraints_for_bool(
                    left,
                    false,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?,
                constraints_for_bool(
                    right,
                    false,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?,
            )),
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual => {
                let left = linear_value_alternatives(
                    left,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?;
                let right = linear_value_alternatives(
                    right,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?;
                comparison_constraint_alternatives(left, right, *op, expected)
            }
            _ => None,
        },
        _ => None,
    }
}

fn comparison_constraints(
    left: LinearConstraint,
    operator: BinaryOp,
    right: LinearConstraint,
    expected: bool,
) -> Option<Vec<Vec<PathConstraint>>> {
    let operator = if expected {
        operator
    } else {
        match operator {
            BinaryOp::Equal => BinaryOp::NotEqual,
            BinaryOp::NotEqual => BinaryOp::Equal,
            BinaryOp::Less => BinaryOp::GreaterEqual,
            BinaryOp::LessEqual => BinaryOp::Greater,
            BinaryOp::Greater => BinaryOp::LessEqual,
            BinaryOp::GreaterEqual => BinaryOp::Less,
            _ => return None,
        }
    };
    let difference = left.clone().combine(right.clone(), 1, -1)?;
    match operator {
        BinaryOp::Equal => Some(vec![vec![
            PathConstraint::Linear(difference.clone()),
            PathConstraint::Linear(difference.negate()?),
        ]]),
        BinaryOp::NotEqual => Some(vec![
            vec![PathConstraint::Linear(
                right.clone().combine(left.clone(), 1, -1)?.shift(1)?,
            )],
            vec![PathConstraint::Linear(difference.shift(1)?)],
        ]),
        BinaryOp::Less => Some(vec![vec![PathConstraint::Linear(
            right.combine(left, 1, -1)?.shift(1)?,
        )]]),
        BinaryOp::LessEqual => Some(vec![vec![PathConstraint::Linear(
            right.combine(left, 1, -1)?,
        )]]),
        BinaryOp::Greater => Some(vec![vec![PathConstraint::Linear(difference.shift(1)?)]]),
        BinaryOp::GreaterEqual => Some(vec![vec![PathConstraint::Linear(difference)]]),
        _ => None,
    }
}

fn constraints_satisfiable(constraints: Vec<PathConstraint>) -> Option<bool> {
    let mut constructor_equals = HashMap::<usize, String>::new();
    let mut constructor_not_equals = HashMap::<usize, HashSet<String>>::new();
    let mut linear_constraints = Vec::new();
    for constraint in constraints {
        match constraint {
            PathConstraint::Linear(constraint) => linear_constraints.push(constraint),
            PathConstraint::Constructor {
                expression,
                name,
                equal,
            } => {
                if equal {
                    if constructor_equals
                        .get(&expression)
                        .is_some_and(|existing| existing != &name)
                        || constructor_not_equals
                            .get(&expression)
                            .is_some_and(|names| names.contains(&name))
                    {
                        return Some(false);
                    }
                    constructor_equals.insert(expression, name);
                } else if constructor_equals
                    .get(&expression)
                    .is_some_and(|existing| existing == &name)
                {
                    return Some(false);
                } else {
                    constructor_not_equals
                        .entry(expression)
                        .or_default()
                        .insert(name);
                }
            }
        }
    }

    let mut variables = linear_constraints
        .iter()
        .flat_map(|constraint| constraint.coefficients.keys().cloned())
        .collect::<Vec<_>>();
    variables.sort();
    variables.dedup();

    for variable in variables {
        let mut positive = Vec::new();
        let mut negative = Vec::new();
        let mut zero = Vec::new();
        for mut constraint in linear_constraints {
            match constraint
                .coefficients
                .remove(&variable)
                .unwrap_or_default()
            {
                coefficient if coefficient > 0 => positive.push((coefficient, constraint)),
                coefficient if coefficient < 0 => negative.push((coefficient, constraint)),
                _ => zero.push(constraint),
            }
        }
        let mut reduced = zero;
        for (positive_coefficient, lower) in &positive {
            for (negative_coefficient, upper) in &negative {
                reduced.push(lower.clone().combine(
                    upper.clone(),
                    negative_coefficient.checked_neg()?,
                    *positive_coefficient,
                )?);
            }
        }
        linear_constraints = reduced;
    }
    Some(
        linear_constraints
            .iter()
            .all(|constraint| constraint.constant >= 0),
    )
}

fn find_symbolic_counterexample(
    predicate: &Expr,
    return_expression: Option<&Expr>,
    guards: &[PathConstraint],
    bindings: &HashMap<String, &Expr>,
    substitutions: &HashMap<String, LinearValue>,
    functions: Option<&HashMap<String, &Function>>,
) -> Option<Vec<(String, i64)>> {
    let violations = constraints_for_bool(
        predicate,
        false,
        return_expression,
        Some(bindings),
        Some(substitutions),
        functions,
        0,
    )?;
    let alternatives = combine_alternatives(vec![guards.to_vec()], violations);
    for constraints in alternatives {
        if constraints_satisfiable(constraints.clone()) == Some(false) {
            continue;
        }
        if let Some(model) = find_linear_model(&constraints) {
            return Some(model);
        }
    }
    None
}

fn find_linear_model(constraints: &[PathConstraint]) -> Option<Vec<(String, i64)>> {
    const COUNTEREXAMPLE_BOUND: i64 = 32;
    const MAX_COUNTEREXAMPLE_VARIABLES: usize = 3;

    let mut linear_constraints = Vec::new();
    for constraint in constraints {
        match constraint {
            PathConstraint::Linear(constraint) => linear_constraints.push(constraint),
            PathConstraint::Constructor { .. } => return None,
        }
    }
    let mut variables = linear_constraints
        .iter()
        .flat_map(|constraint| constraint.coefficients.keys().cloned())
        .collect::<Vec<_>>();
    variables.sort();
    variables.dedup();
    if variables.len() > MAX_COUNTEREXAMPLE_VARIABLES {
        return None;
    }

    let mut model = HashMap::new();
    find_linear_model_values(
        &linear_constraints,
        &variables,
        0,
        &mut model,
        COUNTEREXAMPLE_BOUND,
    )
}

fn find_linear_model_values(
    constraints: &[&LinearConstraint],
    variables: &[String],
    index: usize,
    model: &mut HashMap<String, i64>,
    bound: i64,
) -> Option<Vec<(String, i64)>> {
    if index == variables.len() {
        if !linear_model_satisfies(constraints, model) {
            return None;
        }
        return Some(
            variables
                .iter()
                .map(|name| (name.clone(), *model.get(name).expect("model value")))
                .collect(),
        );
    }

    let name = &variables[index];
    for distance in 0..=bound {
        for value in [distance, -distance] {
            model.insert(name.clone(), value);
            if let Some(result) =
                find_linear_model_values(constraints, variables, index + 1, model, bound)
            {
                return Some(result);
            }
        }
    }
    model.remove(name);
    None
}

fn linear_model_satisfies(constraints: &[&LinearConstraint], model: &HashMap<String, i64>) -> bool {
    constraints.iter().all(|constraint| {
        let Some(value) = constraint.coefficients.iter().try_fold(
            constraint.constant,
            |total, (name, coefficient)| {
                total.checked_add(coefficient.checked_mul(i128::from(*model.get(name)?))?)
            },
        ) else {
            return false;
        };
        value >= 0
    })
}

fn prove_symbolic_predicate(
    predicate: &Expr,
    guards: &[SymbolicGuard<'_>],
    bindings: &HashMap<String, &Expr>,
    substitutions: &HashMap<String, LinearValue>,
    functions: Option<&HashMap<String, &Function>>,
) -> Option<bool> {
    let alternatives =
        symbolic_predicate_alternatives(predicate, guards, bindings, substitutions, functions)?;
    for constraints in alternatives {
        match constraints_satisfiable(constraints) {
            Some(false) => {}
            Some(true) => return Some(false),
            None => return None,
        }
    }
    Some(true)
}

fn symbolic_predicate_alternatives(
    predicate: &Expr,
    guards: &[SymbolicGuard<'_>],
    bindings: &HashMap<String, &Expr>,
    substitutions: &HashMap<String, LinearValue>,
    functions: Option<&HashMap<String, &Function>>,
) -> Option<Vec<Vec<PathConstraint>>> {
    let mut guard_alternatives = vec![Vec::new()];
    for guard in guards {
        let alternatives = constraints_for_guard(
            guard,
            None,
            Some(bindings),
            Some(substitutions),
            functions,
            0,
        )?;
        guard_alternatives = combine_alternatives(guard_alternatives, alternatives);
    }
    let violations = constraints_for_bool(
        predicate,
        false,
        None,
        Some(bindings),
        Some(substitutions),
        functions,
        0,
    )?;
    let alternatives = combine_alternatives(guard_alternatives, violations);
    Some(alternatives)
}

fn find_symbolic_predicate_counterexample(
    predicate: &Expr,
    guards: &[SymbolicGuard<'_>],
    bindings: &HashMap<String, &Expr>,
    substitutions: &HashMap<String, LinearValue>,
    functions: Option<&HashMap<String, &Function>>,
) -> Option<Vec<(String, i64)>> {
    for constraints in
        symbolic_predicate_alternatives(predicate, guards, bindings, substitutions, functions)?
    {
        if constraints_satisfiable(constraints.clone()) == Some(false) {
            continue;
        }
        if let Some(model) = find_linear_model(&constraints) {
            return Some(model);
        }
    }
    None
}

fn invariant_status(
    predicate: &Expr,
    guards: &[SymbolicGuard<'_>],
    bindings: &HashMap<String, &Expr>,
    substitutions: &HashMap<String, LinearValue>,
    functions: Option<&HashMap<String, &Function>>,
) -> VerificationEvidence {
    let status =
        match prove_symbolic_predicate(predicate, guards, bindings, substitutions, functions) {
            Some(true) => VerificationStatus::Proven,
            Some(false) => VerificationStatus::Failed,
            None => VerificationStatus::RuntimeCheck,
        };
    let counterexample = (status == VerificationStatus::Failed)
        .then(|| {
            find_symbolic_predicate_counterexample(
                predicate,
                guards,
                bindings,
                substitutions,
                functions,
            )
        })
        .flatten();
    VerificationEvidence {
        status,
        counterexample,
    }
}

fn record_invariant_status(
    diagnostics: &mut InvariantDiagnostics,
    loop_id: usize,
    index: usize,
    evidence: VerificationEvidence,
) {
    diagnostics
        .entry((loop_id, index))
        .and_modify(|existing| {
            let existing_priority = verification_status_priority(existing.status);
            let new_priority = verification_status_priority(evidence.status);
            if new_priority > existing_priority {
                *existing = evidence.clone();
            } else if existing.counterexample.is_none() {
                existing.counterexample = evidence.counterexample.clone();
            }
        })
        .or_insert(evidence);
}

fn verification_status_priority(status: VerificationStatus) -> u8 {
    match status {
        VerificationStatus::Proven => 0,
        VerificationStatus::Unproven => 1,
        VerificationStatus::RuntimeCheck => 2,
        VerificationStatus::Failed => 3,
    }
}

fn symbolic_bool_with_constraints(
    expression: &Expr,
    return_expression: &Expr,
    guards: &[PathConstraint],
    bindings: Option<&HashMap<String, &Expr>>,
    substitutions: Option<&HashMap<String, LinearValue>>,
    functions: Option<&HashMap<String, &Function>>,
    depth: usize,
) -> Option<bool> {
    if let Some(value) = symbolic_bool(
        expression,
        Some(return_expression),
        bindings,
        substitutions,
        functions,
        depth,
    ) {
        return Some(value);
    }
    let true_constraints = constraints_for_bool(
        expression,
        true,
        Some(return_expression),
        bindings,
        substitutions,
        functions,
        depth,
    )?;
    let false_constraints = constraints_for_bool(
        expression,
        false,
        Some(return_expression),
        bindings,
        substitutions,
        functions,
        depth,
    )?;

    let true_possible = true_constraints.into_iter().any(|constraints| {
        let mut true_path = guards.to_owned();
        true_path.extend(constraints);
        constraints_satisfiable(true_path) != Some(false)
    });
    if !true_possible {
        return Some(false);
    }
    let false_possible = false_constraints.into_iter().any(|constraints| {
        let mut false_path = guards.to_owned();
        false_path.extend(constraints);
        constraints_satisfiable(false_path) != Some(false)
    });
    if !false_possible {
        return Some(true);
    }
    None
}

fn call_preconditions_hold(
    expression: &Expr,
    guards: &[PathConstraint],
    context: SymbolicContext<'_>,
) -> Option<bool> {
    if context.depth >= 32 {
        return None;
    }
    match &expression.kind {
        ExprKind::Unary { expr, .. } => call_preconditions_hold(expr, guards, context),
        ExprKind::Binary { left, right, .. } => {
            if call_preconditions_hold(left, guards, context)?
                && call_preconditions_hold(right, guards, context)?
            {
                Some(true)
            } else {
                Some(false)
            }
        }
        ExprKind::Call { name, args, .. } => {
            for argument in args {
                match call_preconditions_hold(argument, guards, context) {
                    Some(true) => {}
                    result => return result,
                }
            }
            let Some(functions) = context.functions else {
                return Some(true);
            };
            let Some(function) = functions.get(name) else {
                return Some(true);
            };
            if args.len() != function.params.len() {
                return None;
            }
            let mut argument_values = HashMap::new();
            for (parameter, argument) in function.params.iter().zip(args) {
                let value = linear_value(
                    argument,
                    context.return_expression,
                    context.bindings,
                    context.substitutions,
                    context.functions,
                    context.depth,
                )?;
                argument_values.insert(parameter.name.clone(), value);
            }
            for requirement in &function.requires {
                let true_constraints = constraints_for_bool(
                    requirement,
                    true,
                    None,
                    None,
                    Some(&argument_values),
                    context.functions,
                    context.depth + 1,
                )?;
                let false_constraints = constraints_for_bool(
                    requirement,
                    false,
                    None,
                    None,
                    Some(&argument_values),
                    context.functions,
                    context.depth + 1,
                )?;
                let true_possible = true_constraints.into_iter().any(|constraints| {
                    let mut path = guards.to_owned();
                    path.extend(constraints);
                    constraints_satisfiable(path) != Some(false)
                });
                if !true_possible {
                    return Some(false);
                }
                let false_possible = false_constraints.into_iter().any(|constraints| {
                    let mut path = guards.to_owned();
                    path.extend(constraints);
                    constraints_satisfiable(path) != Some(false)
                });
                if false_possible {
                    return None;
                }
            }
            let Some(return_expression) = direct_return_expression(function) else {
                return Some(true);
            };
            call_preconditions_hold(
                return_expression,
                guards,
                SymbolicContext {
                    return_expression: None,
                    bindings: None,
                    substitutions: Some(&argument_values),
                    functions: context.functions,
                    depth: context.depth + 1,
                },
            )
        }
        _ => Some(true),
    }
}

impl LinearValue {
    fn add(mut self, other: Self) -> Option<Self> {
        self.constant = self.constant.checked_add(other.constant)?;
        for (name, coefficient) in other.coefficients {
            let value = self
                .coefficients
                .get(&name)
                .copied()
                .unwrap_or_default()
                .checked_add(coefficient)?;
            if value == 0 {
                self.coefficients.remove(&name);
            } else {
                self.coefficients.insert(name, value);
            }
        }
        Some(self)
    }

    fn negate(mut self) -> Option<Self> {
        self.constant = self.constant.checked_neg()?;
        for coefficient in self.coefficients.values_mut() {
            *coefficient = coefficient.checked_neg()?;
        }
        Some(self)
    }

    fn scale(mut self, factor: i64) -> Option<Self> {
        self.constant = self.constant.checked_mul(factor)?;
        for coefficient in self.coefficients.values_mut() {
            *coefficient = coefficient.checked_mul(factor)?;
        }
        Some(self)
    }

    fn subtract(self, other: Self) -> Option<Self> {
        self.add(other.negate()?)
    }
}

fn substitute_linear_value(
    value: &LinearValue,
    substitutions: &HashMap<String, LinearValue>,
) -> Option<LinearValue> {
    let mut result = LinearValue {
        constant: value.constant,
        ..LinearValue::default()
    };
    for (name, coefficient) in &value.coefficients {
        let term = substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| LinearValue {
                coefficients: HashMap::from([(name.clone(), 1)]),
                constant: 0,
            })
            .scale(*coefficient)?;
        result = result.add(term)?;
    }
    Some(result)
}

fn linear_value(
    expression: &Expr,
    return_expression: Option<&Expr>,
    bindings: Option<&HashMap<String, &Expr>>,
    substitutions: Option<&HashMap<String, LinearValue>>,
    functions: Option<&HashMap<String, &Function>>,
    depth: usize,
) -> Option<LinearValue> {
    let mut alternatives = linear_value_alternatives(
        expression,
        return_expression,
        bindings,
        substitutions,
        functions,
        depth,
    )?;
    if alternatives.len() != 1 || !alternatives[0].1.is_empty() {
        return None;
    }
    Some(alternatives.remove(0).0)
}

fn linear_value_alternatives(
    expression: &Expr,
    return_expression: Option<&Expr>,
    bindings: Option<&HashMap<String, &Expr>>,
    substitutions: Option<&HashMap<String, LinearValue>>,
    functions: Option<&HashMap<String, &Function>>,
    depth: usize,
) -> Option<Vec<(LinearValue, Vec<PathConstraint>)>> {
    match &expression.kind {
        ExprKind::Int(value) => Some(vec![(
            (LinearValue {
                constant: *value,
                ..LinearValue::default()
            }),
            Vec::new(),
        )]),
        ExprKind::Variable(name) if name == "result" => return_expression.and_then(|expression| {
            linear_value_alternatives(
                expression,
                return_expression,
                bindings,
                substitutions,
                functions,
                depth,
            )
        }),
        ExprKind::Variable(name)
            if substitutions
                .and_then(|substitutions| substitutions.get(name))
                .is_some() =>
        {
            Some(vec![(substitutions?.get(name).cloned()?, Vec::new())])
        }
        ExprKind::Variable(name) if bindings.and_then(|bindings| bindings.get(name)).is_some() => {
            let bound = bindings?.get(name)?;
            linear_value_alternatives(
                bound,
                return_expression,
                bindings,
                substitutions,
                functions,
                depth,
            )
        }
        ExprKind::Variable(name) => Some(vec![(
            LinearValue {
                coefficients: HashMap::from([(name.clone(), 1)]),
                constant: 0,
            },
            Vec::new(),
        )]),
        ExprKind::Unary {
            op: UnaryOp::Negate,
            expr,
        } => linear_value_alternatives(
            expr,
            return_expression,
            bindings,
            substitutions,
            functions,
            depth,
        )?
        .into_iter()
        .map(|(value, constraints)| Some((value.negate()?, constraints)))
        .collect(),
        ExprKind::Binary { left, op, right } => {
            let left = linear_value_alternatives(
                left,
                return_expression,
                bindings,
                substitutions,
                functions,
                depth,
            )?;
            let right = linear_value_alternatives(
                right,
                return_expression,
                bindings,
                substitutions,
                functions,
                depth,
            )?;
            let mut alternatives = Vec::new();
            for (left, left_constraints) in &left {
                for (right, right_constraints) in &right {
                    let value = match op {
                        BinaryOp::Add => left.clone().add(right.clone()),
                        BinaryOp::Subtract => left.clone().subtract(right.clone()),
                        BinaryOp::Multiply if right.coefficients.is_empty() => {
                            left.clone().scale(right.constant)
                        }
                        BinaryOp::Multiply if left.coefficients.is_empty() => {
                            right.clone().scale(left.constant)
                        }
                        _ => None,
                    }?;
                    alternatives.push((
                        value,
                        left_constraints
                            .iter()
                            .cloned()
                            .chain(right_constraints.iter().cloned())
                            .collect(),
                    ));
                }
            }
            Some(alternatives)
        }
        ExprKind::Call { name, args, .. } => {
            if depth >= 32 {
                return None;
            }
            let function = functions?.get(name)?;
            if args.len() != function.params.len() {
                return None;
            }
            let mut argument_alternatives = vec![(Vec::new(), Vec::new())];
            for argument in args {
                let values = linear_value_alternatives(
                    argument,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth + 1,
                )?;
                argument_alternatives = argument_alternatives
                    .into_iter()
                    .flat_map(|(arguments, constraints)| {
                        values.iter().map(move |(value, value_constraints)| {
                            let mut next_arguments = arguments.clone();
                            next_arguments.push(value.clone());
                            (
                                next_arguments,
                                constraints
                                    .iter()
                                    .cloned()
                                    .chain(value_constraints.iter().cloned())
                                    .collect(),
                            )
                        })
                    })
                    .collect();
            }
            let mut alternatives = Vec::new();
            for (arguments, argument_constraints) in argument_alternatives {
                let argument_values = function
                    .params
                    .iter()
                    .zip(arguments)
                    .map(|(parameter, value)| (parameter.name.clone(), value))
                    .collect::<HashMap<_, _>>();
                let mut invariant_diagnostics = HashMap::new();
                for path in symbolic_return_paths(function, functions, &mut invariant_diagnostics)?
                {
                    let Some(return_expression) = path.expression else {
                        continue;
                    };
                    let mut callee_substitutions = argument_values.clone();
                    for (name, value) in &path.substitutions {
                        callee_substitutions.insert(
                            name.clone(),
                            substitute_linear_value(value, &argument_values)?,
                        );
                    }
                    let mut path_alternatives = vec![argument_constraints.clone()];
                    for guard in &path.guards {
                        let guard_alternatives = constraints_for_guard(
                            guard,
                            Some(return_expression),
                            Some(&path.bindings),
                            Some(&callee_substitutions),
                            functions,
                            depth + 1,
                        )?;
                        path_alternatives =
                            combine_alternatives(path_alternatives, guard_alternatives);
                    }
                    let values = linear_value_alternatives(
                        return_expression,
                        None,
                        Some(&path.bindings),
                        Some(&callee_substitutions),
                        functions,
                        depth + 1,
                    )?;
                    for (value, value_constraints) in values {
                        for constraints in &path_alternatives {
                            alternatives.push((
                                value.clone(),
                                constraints
                                    .iter()
                                    .cloned()
                                    .chain(value_constraints.iter().cloned())
                                    .collect(),
                            ));
                        }
                    }
                }
            }
            if alternatives.is_empty() {
                None
            } else {
                Some(alternatives)
            }
        }
        _ => None,
    }
}

fn symbolic_bool(
    expression: &Expr,
    return_expression: Option<&Expr>,
    bindings: Option<&HashMap<String, &Expr>>,
    substitutions: Option<&HashMap<String, LinearValue>>,
    functions: Option<&HashMap<String, &Function>>,
    depth: usize,
) -> Option<bool> {
    match &expression.kind {
        ExprKind::Unary {
            op: UnaryOp::Not,
            expr,
        } => symbolic_bool(
            expr,
            return_expression,
            bindings,
            substitutions,
            functions,
            depth,
        )
        .map(|value| !value),
        ExprKind::Binary { left, op, right } => match op {
            BinaryOp::And => match (
                symbolic_bool(
                    left,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                ),
                symbolic_bool(
                    right,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                ),
            ) {
                (Some(false), _) | (_, Some(false)) => Some(false),
                (Some(true), Some(true)) => Some(true),
                _ => None,
            },
            BinaryOp::Or => match (
                symbolic_bool(
                    left,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                ),
                symbolic_bool(
                    right,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                ),
            ) {
                (Some(true), _) | (_, Some(true)) => Some(true),
                (Some(false), Some(false)) => Some(false),
                _ => None,
            },
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual => {
                let left = linear_value(
                    left,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?;
                let right = linear_value(
                    right,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?;
                let difference = left.subtract(right)?;
                if !difference.coefficients.is_empty() {
                    return None;
                }
                Some(match op {
                    BinaryOp::Equal => difference.constant == 0,
                    BinaryOp::NotEqual => difference.constant != 0,
                    BinaryOp::Less => difference.constant < 0,
                    BinaryOp::LessEqual => difference.constant <= 0,
                    BinaryOp::Greater => difference.constant > 0,
                    BinaryOp::GreaterEqual => difference.constant >= 0,
                    _ => unreachable!(),
                })
            }
            _ => None,
        },
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ConstantValue {
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
    Char(char),
}

fn constant_value(expression: &Expr) -> Option<ConstantValue> {
    match &expression.kind {
        ExprKind::Int(value) => Some(ConstantValue::Int(*value)),
        ExprKind::UInt(value) => Some(ConstantValue::UInt(*value)),
        ExprKind::Float(value) => Some(ConstantValue::Float(*value)),
        ExprKind::Bool(value) => Some(ConstantValue::Bool(*value)),
        ExprKind::String(value) => Some(ConstantValue::String(value.clone())),
        ExprKind::Char(value) => Some(ConstantValue::Char(*value)),
        ExprKind::Unary { op, expr } => {
            let value = constant_value(expr)?;
            match (op, value) {
                (UnaryOp::Negate, ConstantValue::Int(value)) => {
                    value.checked_neg().map(ConstantValue::Int)
                }
                (UnaryOp::Negate, ConstantValue::Float(value)) => {
                    Some(ConstantValue::Float(-value))
                }
                (UnaryOp::Not, ConstantValue::Bool(value)) => Some(ConstantValue::Bool(!value)),
                _ => None,
            }
        }
        ExprKind::Binary { left, op, right } => {
            let left = constant_value(left)?;
            if *op == BinaryOp::And && matches!(left, ConstantValue::Bool(false)) {
                return Some(ConstantValue::Bool(false));
            }
            if *op == BinaryOp::Or && matches!(left, ConstantValue::Bool(true)) {
                return Some(ConstantValue::Bool(true));
            }
            let right = constant_value(right)?;
            constant_binary(left, *op, right)
        }
        ExprKind::Array(_)
        | ExprKind::Record { .. }
        | ExprKind::Index { .. }
        | ExprKind::Field { .. }
        | ExprKind::Variable(_)
        | ExprKind::Call { .. }
        | ExprKind::Sql { .. }
        | ExprKind::Await(_) => None,
    }
}

fn constant_binary(
    left: ConstantValue,
    op: BinaryOp,
    right: ConstantValue,
) -> Option<ConstantValue> {
    use BinaryOp::*;
    match (left, op, right) {
        (ConstantValue::Int(left), Add, ConstantValue::Int(right)) => {
            left.checked_add(right).map(ConstantValue::Int)
        }
        (ConstantValue::Int(left), Subtract, ConstantValue::Int(right)) => {
            left.checked_sub(right).map(ConstantValue::Int)
        }
        (ConstantValue::Int(left), Multiply, ConstantValue::Int(right)) => {
            left.checked_mul(right).map(ConstantValue::Int)
        }
        (ConstantValue::Int(left), Divide, ConstantValue::Int(right)) if right != 0 => {
            left.checked_div(right).map(ConstantValue::Int)
        }
        (ConstantValue::Int(left), Remainder, ConstantValue::Int(right)) if right != 0 => {
            left.checked_rem(right).map(ConstantValue::Int)
        }
        (ConstantValue::Float(left), Add, ConstantValue::Float(right)) => {
            Some(ConstantValue::Float(left + right))
        }
        (ConstantValue::Float(left), Subtract, ConstantValue::Float(right)) => {
            Some(ConstantValue::Float(left - right))
        }
        (ConstantValue::Float(left), Multiply, ConstantValue::Float(right)) => {
            Some(ConstantValue::Float(left * right))
        }
        (ConstantValue::Float(left), Divide, ConstantValue::Float(right)) if right != 0.0 => {
            Some(ConstantValue::Float(left / right))
        }
        (ConstantValue::String(left), Add, ConstantValue::String(right)) => {
            Some(ConstantValue::String(left + &right))
        }
        (left, Equal, right) => Some(ConstantValue::Bool(left == right)),
        (left, NotEqual, right) => Some(ConstantValue::Bool(left != right)),
        (ConstantValue::Int(left), Less, ConstantValue::Int(right)) => {
            Some(ConstantValue::Bool(left < right))
        }
        (ConstantValue::Int(left), LessEqual, ConstantValue::Int(right)) => {
            Some(ConstantValue::Bool(left <= right))
        }
        (ConstantValue::Int(left), Greater, ConstantValue::Int(right)) => {
            Some(ConstantValue::Bool(left > right))
        }
        (ConstantValue::Int(left), GreaterEqual, ConstantValue::Int(right)) => {
            Some(ConstantValue::Bool(left >= right))
        }
        (ConstantValue::Float(left), Less, ConstantValue::Float(right)) => {
            Some(ConstantValue::Bool(left < right))
        }
        (ConstantValue::Float(left), LessEqual, ConstantValue::Float(right)) => {
            Some(ConstantValue::Bool(left <= right))
        }
        (ConstantValue::Float(left), Greater, ConstantValue::Float(right)) => {
            Some(ConstantValue::Bool(left > right))
        }
        (ConstantValue::Float(left), GreaterEqual, ConstantValue::Float(right)) => {
            Some(ConstantValue::Bool(left >= right))
        }
        (ConstantValue::Bool(left), And, ConstantValue::Bool(right)) => {
            Some(ConstantValue::Bool(left && right))
        }
        (ConstantValue::Bool(left), Or, ConstantValue::Bool(right)) => {
            Some(ConstantValue::Bool(left || right))
        }
        _ => None,
    }
}

pub fn check_capabilities(program: &Program) -> Result<(), Vec<CapabilityError>> {
    check_capabilities_with_grants(program, None)
}

pub fn check_capabilities_with_grants(
    program: &Program,
    grants: Option<&HashSet<String>>,
) -> Result<(), Vec<CapabilityError>> {
    let functions = program
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function))
        .collect::<HashMap<_, _>>();
    let known = KNOWN_CAPABILITIES.iter().copied().collect::<HashSet<_>>();
    let mut errors = Vec::new();

    for function in &program.functions {
        let mut declared = HashSet::new();
        for capability in &function.capabilities {
            if !known.contains(capability.as_str()) {
                errors.push(CapabilityError {
                    message: format!(
                        "unknown capability `{capability}`; expected one of {}",
                        KNOWN_CAPABILITIES.join(", ")
                    ),
                    span: function.span,
                });
            } else if !declared.insert(capability.as_str()) {
                errors.push(CapabilityError {
                    message: format!("capability `{capability}` is declared more than once"),
                    span: function.span,
                });
            } else if grants.is_some_and(|grants| !grants.contains(capability)) {
                errors.push(CapabilityError {
                    message: format!(
                        "capability `{capability}` is not enabled by the project; set `{}` to true in zelyra.toml",
                        capability.to_ascii_lowercase()
                    ),
                    span: function.span,
                });
            }
        }
        check_capability_block(&function.body, function, &functions, &declared, &mut errors);
        for contract in function.requires.iter().chain(&function.ensures) {
            check_capability_expr(contract, function, &functions, &declared, &mut errors);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn check_capability_block(
    block: &Block,
    function: &Function,
    functions: &HashMap<&str, &Function>,
    declared: &HashSet<&str>,
    errors: &mut Vec<CapabilityError>,
) {
    for statement in &block.statements {
        match statement {
            Stmt::Let { value, .. }
            | Stmt::BindOrAssign { value, .. }
            | Stmt::Expr(value)
            | Stmt::Return {
                value: Some(value), ..
            }
            | Stmt::Match { value, .. } => {
                check_capability_expr(value, function, functions, declared, errors);
                if let Stmt::Match { arms, .. } = statement {
                    for arm in arms {
                        check_capability_block(&arm.body, function, functions, declared, errors);
                    }
                }
            }
            Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                check_capability_expr(condition, function, functions, declared, errors);
                check_capability_block(then_block, function, functions, declared, errors);
                if let Some(else_block) = else_block {
                    check_capability_block(else_block, function, functions, declared, errors);
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                check_capability_expr(condition, function, functions, declared, errors);
                for invariant in invariants {
                    check_capability_expr(invariant, function, functions, declared, errors);
                }
                check_capability_block(body, function, functions, declared, errors);
            }
            Stmt::For { iterable, body, .. } => {
                check_capability_expr(iterable, function, functions, declared, errors);
                check_capability_block(body, function, functions, declared, errors);
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    check_capability_expr(invariant, function, functions, declared, errors);
                }
                check_capability_block(body, function, functions, declared, errors);
            }
            Stmt::Transaction { body, .. } => {
                check_capability_block(body, function, functions, declared, errors);
            }
            Stmt::Parallel { body, .. } => {
                check_capability_block(body, function, functions, declared, errors);
            }
        }
    }
}

fn check_capability_expr(
    expression: &Expr,
    function: &Function,
    functions: &HashMap<&str, &Function>,
    declared: &HashSet<&str>,
    errors: &mut Vec<CapabilityError>,
) {
    match &expression.kind {
        ExprKind::Sql { .. } => require_capability(
            "Database",
            "SQL access",
            expression.span,
            function,
            declared,
            errors,
        ),
        ExprKind::Call { name, args, .. } => {
            if name == "now" {
                require_capability(
                    "Clock",
                    "clock access",
                    expression.span,
                    function,
                    declared,
                    errors,
                );
            } else if name == "env" {
                require_capability(
                    "Environment",
                    "environment access",
                    expression.span,
                    function,
                    declared,
                    errors,
                );
            } else if name == "random_int" {
                require_capability(
                    "Random",
                    "random access",
                    expression.span,
                    function,
                    declared,
                    errors,
                );
            } else if matches!(
                name.as_str(),
                "http_get" | "http_request" | "http_json" | "http_result"
            ) {
                require_capability(
                    "Network",
                    "network access",
                    expression.span,
                    function,
                    declared,
                    errors,
                );
            } else if name == "run_process" {
                require_capability(
                    "Process",
                    "process execution",
                    expression.span,
                    function,
                    declared,
                    errors,
                );
            } else if name == "read_text" {
                require_capability(
                    "FileSystem",
                    "file-system read access",
                    expression.span,
                    function,
                    declared,
                    errors,
                );
            } else if name == "write_text" {
                require_capability(
                    "FileSystem",
                    "file-system write access",
                    expression.span,
                    function,
                    declared,
                    errors,
                );
            } else if name == "delete_file" {
                require_capability(
                    "FileSystem",
                    "file-system delete access",
                    expression.span,
                    function,
                    declared,
                    errors,
                );
            } else if name == "list_dir" {
                require_capability(
                    "FileSystem",
                    "file-system read access",
                    expression.span,
                    function,
                    declared,
                    errors,
                );
            }
            if let Some(callee) = functions.get(name.as_str()) {
                for capability in &callee.capabilities {
                    if KNOWN_CAPABILITIES.contains(&capability.as_str())
                        && !declared.contains(capability.as_str())
                    {
                        errors.push(CapabilityError {
                            message: format!(
                                "function `{}` calls `{name}`, which requires capability `{capability}`; declare `uses {capability}`",
                                function.name
                            ),
                            span: expression.span,
                        });
                    }
                }
            }
            for argument in args {
                check_capability_expr(argument, function, functions, declared, errors);
            }
        }
        ExprKind::Unary { expr, .. } => {
            check_capability_expr(expr, function, functions, declared, errors);
        }
        ExprKind::Binary { left, right, .. } => {
            check_capability_expr(left, function, functions, declared, errors);
            check_capability_expr(right, function, functions, declared, errors);
        }
        ExprKind::Array(values) => {
            for value in values {
                check_capability_expr(value, function, functions, declared, errors);
            }
        }
        ExprKind::Record { fields, .. } => {
            for (_, value) in fields {
                check_capability_expr(value, function, functions, declared, errors);
            }
        }
        ExprKind::Index { target, index } => {
            check_capability_expr(target, function, functions, declared, errors);
            check_capability_expr(index, function, functions, declared, errors);
        }
        ExprKind::Field { target, .. } => {
            check_capability_expr(target, function, functions, declared, errors);
        }
        ExprKind::Await(inner) => {
            check_capability_expr(inner, function, functions, declared, errors);
        }
        ExprKind::Int(..)
        | ExprKind::UInt(..)
        | ExprKind::Float(..)
        | ExprKind::Bool(..)
        | ExprKind::String(..)
        | ExprKind::Char(..)
        | ExprKind::Variable(..) => {}
    }
}

fn require_capability(
    capability: &str,
    operation: &str,
    span: Span,
    function: &Function,
    declared: &HashSet<&str>,
    errors: &mut Vec<CapabilityError>,
) {
    if !declared.contains(capability) {
        errors.push(CapabilityError {
            message: format!(
                "function `{}` uses {operation} but does not declare capability `{capability}`; add `uses {capability}`",
                function.name
            ),
            span,
        });
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
    parallel_depth: usize,
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
        "HttpResponse",
        "HttpError",
        "HttpResult",
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
    for record in &program.records {
        if !known_types.insert(record.name.clone()) {
            errors.push(TypeError {
                message: format!("duplicate type `{}`", record.name),
                span: record.span,
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
    for record in &program.records {
        let mut fields = HashSet::new();
        for field in &record.fields {
            if !fields.insert(field.name.clone()) {
                errors.push(TypeError {
                    message: format!(
                        "duplicate field `{}` in record `{}`",
                        field.name, record.name
                    ),
                    span: field.span,
                });
            }
            validate_type(&field.ty, &known_types, &mut errors, field.span);
        }
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
        parallel_depth: 0,
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
        for contract in &function.requires {
            let actual = self.check_expr(contract, &scopes);
            self.expect_type(&Type::Bool, &actual, contract.span);
        }
        self.check_block(&function.body, &mut scopes, &expected);
        if !function.ensures.is_empty() {
            scopes[0].insert(
                "result".into(),
                Variable {
                    ty: function.return_type.clone().unwrap_or(Type::Unit),
                    mutable: false,
                },
            );
            for contract in &function.ensures {
                let actual = self.check_expr(contract, &scopes);
                self.expect_type(&Type::Bool, &actual, contract.span);
            }
        }
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
                condition,
                invariants,
                body,
                ..
            } => {
                let condition_type = self.check_expr(condition, scopes);
                self.expect_type(&Type::Bool, &condition_type, condition.span);
                for invariant in invariants {
                    let invariant_type = self.check_expr(invariant, scopes);
                    self.expect_type(&Type::Bool, &invariant_type, invariant.span);
                }
                self.loop_depth += 1;
                self.check_block(body, scopes, expected);
                self.loop_depth -= 1;
            }
            Stmt::For {
                name,
                iterable,
                body,
                ..
            } => {
                let iterable_type = self.check_expr(iterable, scopes);
                let element_type = match iterable_type {
                    Type::Array(inner) => *inner,
                    Type::Unknown => Type::Unknown,
                    other => {
                        self.error(
                            iterable.span,
                            format!("`for ... in` expects an array, found `{other}`"),
                        );
                        Type::Unknown
                    }
                };
                scopes.push(HashMap::new());
                scopes.last_mut().unwrap().insert(
                    name.clone(),
                    Variable {
                        ty: element_type,
                        mutable: false,
                    },
                );
                self.loop_depth += 1;
                self.check_block(body, scopes, expected);
                self.loop_depth -= 1;
                scopes.pop();
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    let invariant_type = self.check_expr(invariant, scopes);
                    self.expect_type(&Type::Bool, &invariant_type, invariant.span);
                }
                self.loop_depth += 1;
                self.check_block(body, scopes, expected);
                self.loop_depth -= 1;
            }
            Stmt::Break { span } => {
                if self.loop_depth == 0 {
                    self.error(*span, "`break` is only valid inside a loop");
                }
            }
            Stmt::Continue { span } => {
                if self.loop_depth == 0 {
                    self.error(*span, "`continue` is only valid inside a loop");
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
            Stmt::Parallel { body, span } => {
                let mut names = HashSet::new();
                let mut new_bindings = Vec::new();
                self.parallel_depth += 1;
                for statement in &body.statements {
                    let Stmt::BindOrAssign { name, value, span } = statement else {
                        self.error(
                            *span,
                            "parallel blocks may contain only `name = await expression` bindings",
                        );
                        continue;
                    };
                    if !matches!(&value.kind, ExprKind::Await(_)) {
                        self.error(value.span, "parallel bindings must await their expression");
                    }
                    if !names.insert(name.clone()) {
                        self.error(
                            *span,
                            format!("parallel binding `{name}` is declared twice"),
                        );
                    }
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
                        new_bindings.push((name.clone(), actual));
                    }
                }
                self.parallel_depth -= 1;
                for (name, ty) in new_bindings {
                    scopes
                        .last_mut()
                        .unwrap()
                        .insert(name, Variable { ty, mutable: false });
                }
            }
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
            Type::HttpResult(inner) => self.check_type(inner, span),
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
            ExprKind::Array(values) => {
                let mut element = Type::Unknown;
                for value in values {
                    let value_type = self.check_expr(value, scopes);
                    if element == Type::Unknown {
                        element = value_type;
                    } else {
                        self.expect_type(&element, &value_type, value.span);
                    }
                }
                Type::Array(Box::new(element))
            }
            ExprKind::Record { type_name, fields } => {
                let Some(record) = self
                    ._program
                    .records
                    .iter()
                    .find(|record| record.name == *type_name)
                    .cloned()
                else {
                    for (_, value) in fields {
                        self.check_expr(value, scopes);
                    }
                    self.error(expr.span, format!("unknown record type `{type_name}`"));
                    return Type::Unknown;
                };
                let mut provided = HashSet::new();
                for (field_name, value) in fields {
                    let actual = self.check_expr(value, scopes);
                    if !provided.insert(field_name) {
                        self.error(
                            value.span,
                            format!("record field `{field_name}` is specified more than once"),
                        );
                        continue;
                    }
                    let Some(field) = record.fields.iter().find(|field| field.name == *field_name)
                    else {
                        self.error(
                            value.span,
                            format!("unknown field `{field_name}` in record `{type_name}`"),
                        );
                        continue;
                    };
                    self.expect_type(&field.ty, &actual, value.span);
                }
                for field in &record.fields {
                    if !provided.contains(&field.name) && !matches!(field.ty, Type::Option(_)) {
                        self.error(
                            expr.span,
                            format!("missing field `{}` in record `{type_name}`", field.name),
                        );
                    }
                }
                Type::Named(type_name.clone())
            }
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
            ExprKind::Call {
                name,
                type_args,
                args,
            } => {
                if name == "print" {
                    if args.len() != 1 {
                        self.error(expr.span, "`print` expects exactly one argument");
                    }
                    for arg in args {
                        self.check_expr(arg, scopes);
                    }
                    Type::Unit
                } else if name == "len" {
                    if args.len() != 1 {
                        self.error(expr.span, "`len` expects exactly one argument");
                        return Type::Unknown;
                    }
                    let argument = self.check_expr(&args[0], scopes);
                    if !matches!(argument, Type::Array(_)) && argument != Type::Unknown {
                        self.error(args[0].span, "`len` expects an array");
                    }
                    Type::Int
                } else if name == "append" {
                    if args.len() != 2 {
                        self.error(expr.span, "`append` expects an array and one value");
                        return Type::Unknown;
                    }
                    let array_type = self.check_expr(&args[0], scopes);
                    let value_type = self.check_expr(&args[1], scopes);
                    match array_type {
                        Type::Array(inner) => {
                            self.expect_type(&inner, &value_type, args[1].span);
                            Type::Array(inner)
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("`append` expects an array, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "contains" {
                    if args.len() != 2 {
                        self.error(expr.span, "`contains` expects an array and one value");
                        return Type::Unknown;
                    }
                    let array_type = self.check_expr(&args[0], scopes);
                    let value_type = self.check_expr(&args[1], scopes);
                    match array_type {
                        Type::Array(inner) => {
                            self.expect_type(&inner, &value_type, args[1].span);
                            Type::Bool
                        }
                        Type::Unknown => Type::Bool,
                        other => {
                            self.error(
                                args[0].span,
                                format!("`contains` expects an array, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "first" || name == "last" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("`{name}` expects exactly one argument"));
                        return Type::Unknown;
                    }
                    let argument = self.check_expr(&args[0], scopes);
                    match argument {
                        Type::Array(inner) => Type::Option(inner),
                        Type::Unknown => Type::Option(Box::new(Type::Unknown)),
                        other => {
                            self.error(
                                args[0].span,
                                format!("`{name}` expects an array, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
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
                } else if name == "now" {
                    if !args.is_empty() {
                        self.error(expr.span, "now expects no arguments");
                    }
                    Type::Timestamp
                } else if name == "env" {
                    if args.len() != 1 {
                        self.error(expr.span, "env expects exactly one argument");
                        return Type::Unknown;
                    }
                    let argument = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &argument, args[0].span);
                    Type::Option(Box::new(Type::String))
                } else if name == "random_int" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "random_int expects minimum and maximum Int arguments",
                        );
                        return Type::Unknown;
                    }
                    let minimum = self.check_expr(&args[0], scopes);
                    let maximum = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Int, &minimum, args[0].span);
                    self.expect_type(&Type::Int, &maximum, args[1].span);
                    Type::Int
                } else if name == "http_get" {
                    if args.len() != 1 {
                        self.error(expr.span, "http_get expects exactly one String URL");
                        return Type::Unknown;
                    }
                    let url = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &url, args[0].span);
                    Type::String
                } else if name == "json_encode" {
                    if !type_args.is_empty() {
                        self.error(expr.span, "json_encode does not accept type arguments");
                    }
                    if args.len() != 1 {
                        self.error(expr.span, "json_encode expects exactly one value");
                        return Type::Unknown;
                    }
                    self.check_expr(&args[0], scopes);
                    Type::String
                } else if name == "json_decode" {
                    if type_args.len() != 1 {
                        self.error(expr.span, "json_decode expects exactly one type argument");
                    } else {
                        self.check_type(&type_args[0], expr.span);
                    }
                    if args.len() != 1 {
                        self.error(expr.span, "json_decode expects exactly one String value");
                        return Type::Unknown;
                    }
                    let source = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &source, args[0].span);
                    type_args.first().cloned().unwrap_or(Type::Unknown)
                } else if name == "http_json" {
                    if type_args.len() != 2 {
                        self.error(
                            expr.span,
                            "http_json expects request and response type arguments",
                        );
                    } else {
                        self.check_type(&type_args[0], expr.span);
                        self.check_type(&type_args[1], expr.span);
                    }
                    if args.len() != 4 {
                        self.error(
                            expr.span,
                            "http_json expects String method, String URL, String[] headers, and Request? body",
                        );
                        return Type::Unknown;
                    }
                    let method = self.check_expr(&args[0], scopes);
                    let url = self.check_expr(&args[1], scopes);
                    let headers = self.check_expr(&args[2], scopes);
                    let body = self.check_expr(&args[3], scopes);
                    self.expect_type(&Type::String, &method, args[0].span);
                    self.expect_type(&Type::String, &url, args[1].span);
                    self.expect_type(&Type::Array(Box::new(Type::String)), &headers, args[2].span);
                    if let Some(request_type) = type_args.first() {
                        self.expect_type(
                            &Type::Option(Box::new(request_type.clone())),
                            &body,
                            args[3].span,
                        );
                    }
                    type_args.get(1).cloned().unwrap_or(Type::Unknown)
                } else if name == "http_result" {
                    if type_args.len() != 2 {
                        self.error(
                            expr.span,
                            "http_result expects request and response type arguments",
                        );
                    } else {
                        self.check_type(&type_args[0], expr.span);
                        self.check_type(&type_args[1], expr.span);
                    }
                    if args.len() != 4 {
                        self.error(
                            expr.span,
                            "http_result expects String method, String URL, String[] headers, and Request? body",
                        );
                        return Type::Unknown;
                    }
                    let method = self.check_expr(&args[0], scopes);
                    let url = self.check_expr(&args[1], scopes);
                    let headers = self.check_expr(&args[2], scopes);
                    let body = self.check_expr(&args[3], scopes);
                    self.expect_type(&Type::String, &method, args[0].span);
                    self.expect_type(&Type::String, &url, args[1].span);
                    self.expect_type(&Type::Array(Box::new(Type::String)), &headers, args[2].span);
                    if let Some(request_type) = type_args.first() {
                        self.expect_type(
                            &Type::Option(Box::new(request_type.clone())),
                            &body,
                            args[3].span,
                        );
                    }
                    Type::HttpResult(Box::new(type_args.get(1).cloned().unwrap_or(Type::Unknown)))
                } else if name == "http_request" {
                    if args.len() != 4 {
                        self.error(
                            expr.span,
                            "http_request expects String method, String URL, String[] headers, and String? body",
                        );
                        return Type::Unknown;
                    }
                    let method = self.check_expr(&args[0], scopes);
                    let url = self.check_expr(&args[1], scopes);
                    let headers = self.check_expr(&args[2], scopes);
                    let body = self.check_expr(&args[3], scopes);
                    self.expect_type(&Type::String, &method, args[0].span);
                    self.expect_type(&Type::String, &url, args[1].span);
                    self.expect_type(&Type::Array(Box::new(Type::String)), &headers, args[2].span);
                    self.expect_type(&Type::Option(Box::new(Type::String)), &body, args[3].span);
                    Type::Named("HttpResponse".into())
                } else if name == "run_process" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "run_process expects a String command and String[] arguments",
                        );
                        return Type::Unknown;
                    }
                    let command = self.check_expr(&args[0], scopes);
                    let arguments = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &command, args[0].span);
                    self.expect_type(
                        &Type::Array(Box::new(Type::String)),
                        &arguments,
                        args[1].span,
                    );
                    Type::String
                } else if name == "read_text" {
                    if args.len() != 1 {
                        self.error(expr.span, "read_text expects exactly one String path");
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    Type::String
                } else if name == "write_text" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "write_text expects a String path and String content",
                        );
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    let content = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    self.expect_type(&Type::String, &content, args[1].span);
                    Type::Unit
                } else if name == "delete_file" {
                    if args.len() != 1 {
                        self.error(expr.span, "delete_file expects one String path");
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    Type::Unit
                } else if name == "list_dir" {
                    if args.len() != 1 {
                        self.error(expr.span, "list_dir expects one String path");
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    Type::Array(Box::new(Type::String))
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
            ExprKind::Index { target, index } => {
                let target_type = self.check_expr(target, scopes);
                let index_type = self.check_expr(index, scopes);
                if !matches!(index_type, Type::Int | Type::UInt | Type::Unknown) {
                    self.error(
                        index.span,
                        format!("array index must be an integer, found `{index_type}`"),
                    );
                }
                match target_type {
                    Type::Array(inner) => *inner,
                    Type::Unknown => Type::Unknown,
                    other => {
                        self.error(
                            target.span,
                            format!("array indexing requires an array, found `{other}`"),
                        );
                        Type::Unknown
                    }
                }
            }
            ExprKind::Field { target, field } => {
                let target_type = self.check_expr(target, scopes);
                match self.record_field_type(&target_type, field) {
                    Some(field_type) => field_type,
                    None if target_type == Type::Unknown => Type::Unknown,
                    None => {
                        self.error(
                            expr.span,
                            format!("unknown field `{field}` on `{target_type}`"),
                        );
                        Type::Unknown
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
                        } else if let (Type::Array(left), Type::Array(right)) =
                            (&left_ty, &right_ty)
                        {
                            self.expect_type(left, right, expr.span);
                            Type::Array(left.clone())
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
            ExprKind::Await(inner) => {
                if self.parallel_depth == 0 {
                    self.error(expr.span, "`await` is only valid inside a `parallel` block");
                }
                self.check_expr(inner, scopes)
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

    fn record_field_type(&self, ty: &Type, field: &str) -> Option<Type> {
        if let Type::HttpResult(response) = ty {
            return match field {
                "status" => Some(Type::Int),
                "headers" => Some(Type::Array(Box::new(Type::String))),
                "body" => Some(Type::String),
                "data" => Some(Type::Option(response.clone())),
                "error" => Some(Type::Option(Box::new(Type::Named("HttpError".into())))),
                _ => None,
            };
        }
        let Type::Named(initial_name) = ty else {
            return None;
        };
        if initial_name == "HttpError" {
            return match field {
                "status" => Some(Type::Int),
                "headers" => Some(Type::Array(Box::new(Type::String))),
                "body" | "message" => Some(Type::String),
                _ => None,
            };
        }
        let mut name = initial_name.as_str();
        if name == "HttpResponse" {
            return match field {
                "status" => Some(Type::Int),
                "headers" => Some(Type::Array(Box::new(Type::String))),
                "body" => Some(Type::String),
                _ => None,
            };
        }
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(name) {
                return None;
            }
            if let Some(record) = self
                ._program
                .records
                .iter()
                .find(|record| record.name == name)
            {
                return record
                    .fields
                    .iter()
                    .find(|candidate| candidate.name == field)
                    .map(|candidate| candidate.ty.clone());
            }
            let alias = self
                ._program
                .types
                .iter()
                .find(|definition| definition.name == name)?;
            let Type::Named(next_name) = &alias.target else {
                return None;
            };
            name = next_name;
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
        (Type::HttpResult(expected), Type::HttpResult(actual)) => compatible(expected, actual),
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
        Type::HttpResult(inner) => validate_type(inner, known_types, errors, span),
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
    Timestamp(i64),
    Array(Vec<Value>),
    Object {
        type_name: String,
        fields: HashMap<String, Value>,
    },
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
            Value::Timestamp(_) => Type::Timestamp,
            Value::Array(values) => Type::Array(Box::new(
                values.first().map(Value::ty).unwrap_or(Type::Unknown),
            )),
            Value::Object { type_name, .. } => Type::Named(type_name.clone()),
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
            Value::Timestamp(v) => v.to_string(),
            Value::Array(values) => format!(
                "[{}]",
                values
                    .iter()
                    .map(Value::output)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Object { type_name, fields } => format!(
                "{}{{{}}}",
                type_name,
                fields
                    .iter()
                    .map(|(name, value)| format!("{name}={}", value.output()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileSystemPolicy {
    pub base_dir: PathBuf,
    pub read_roots: Vec<PathBuf>,
    pub write_roots: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkPolicy {
    pub allowed_hosts: Vec<String>,
    pub timeout_ms: u64,
    pub max_response_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessPolicy {
    pub allowed_commands: Vec<String>,
    pub timeout_ms: u64,
    pub max_output_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RuntimePolicy {
    pub filesystem: Option<FileSystemPolicy>,
    pub network: Option<NetworkPolicy>,
    pub process: Option<ProcessPolicy>,
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

#[derive(Clone)]
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
    LoopContinue,
    Return(Value),
    Break,
}

pub fn execute(program: &Program) -> Result<Vec<String>, RuntimeError> {
    execute_internal(program, None, None, None)
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
    let mut interpreter = Interpreter {
        functions: program
            .functions
            .iter()
            .map(|function| (function.name.clone(), function.clone()))
            .collect(),
        output: Vec::new(),
        steps: 0,
        database_url: database_url.map(str::to_owned),
        granted_capabilities: grants.cloned(),
        filesystem_policy: policy.and_then(|policy| policy.filesystem.clone()),
        network_policy: policy.and_then(|policy| policy.network.clone()),
        process_policy: policy.and_then(|policy| policy.process.clone()),
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
        active_capabilities: Vec::new(),
    };
    interpreter.call(name, args, Span::default())
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
        }),
    )
}

fn execute_internal(
    program: &Program,
    database_url: Option<String>,
    granted_capabilities: Option<HashSet<String>>,
    policy: Option<RuntimePolicy>,
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
        granted_capabilities,
        filesystem_policy: policy.as_ref().and_then(|policy| policy.filesystem.clone()),
        network_policy: policy.as_ref().and_then(|policy| policy.network.clone()),
        process_policy: policy.and_then(|policy| policy.process),
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
        active_capabilities: Vec::new(),
    };
    interpreter.call("main", Vec::new(), Span::default())?;
    Ok(interpreter.output)
}

struct Interpreter {
    functions: HashMap<String, Function>,
    output: Vec<String>,
    steps: usize,
    database_url: Option<String>,
    granted_capabilities: Option<HashSet<String>>,
    filesystem_policy: Option<FileSystemPolicy>,
    network_policy: Option<NetworkPolicy>,
    process_policy: Option<ProcessPolicy>,
    record_definitions: HashMap<String, RecordDef>,
    type_aliases: HashMap<String, Type>,
    active_capabilities: Vec<HashSet<String>>,
}

impl Interpreter {
    fn runtime_error(&self, span: Span, message: impl Into<String>) -> RuntimeError {
        RuntimeError {
            message: message.into(),
            span,
        }
    }
    fn require_runtime_capability(
        &self,
        capability: &str,
        operation: &str,
        span: Span,
    ) -> Result<(), RuntimeError> {
        let declared = self
            .active_capabilities
            .last()
            .is_some_and(|capabilities| capabilities.contains(capability));
        if !declared {
            return Err(self.runtime_error(
                span,
                format!(
                    "runtime capability denied: {operation} requires `{capability}` in the current function"
                ),
            ));
        }
        if self
            .granted_capabilities
            .as_ref()
            .is_some_and(|grants| !grants.contains(capability))
        {
            return Err(self.runtime_error(
                span,
                format!("runtime capability denied: `{capability}` is not granted"),
            ));
        }
        Ok(())
    }

    fn resolve_filesystem_path(
        &self,
        path: &str,
        roots: &[PathBuf],
        operation: &str,
        span: Span,
        allow_missing: bool,
    ) -> Result<PathBuf, RuntimeError> {
        let Some(policy) = &self.filesystem_policy else {
            return Ok(PathBuf::from(path));
        };
        let candidate = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            policy.base_dir.join(path)
        };
        let resolved = if allow_missing && !candidate.exists() && !candidate.is_symlink() {
            let Some(file_name) = candidate.file_name() else {
                return Err(self.runtime_error(
                    span,
                    format!("file-system {operation} requires a file path"),
                ));
            };
            let parent = candidate.parent().unwrap_or(Path::new("."));
            let parent = std::fs::canonicalize(parent).map_err(|error| {
                self.runtime_error(
                    span,
                    format!("file-system {operation} failed for {path}: {error}"),
                )
            })?;
            parent.join(file_name)
        } else {
            std::fs::canonicalize(&candidate).map_err(|error| {
                self.runtime_error(
                    span,
                    format!("file-system {operation} failed for {path}: {error}"),
                )
            })?
        };
        if roots
            .iter()
            .any(|root| resolved == *root || resolved.starts_with(root))
        {
            Ok(resolved)
        } else {
            Err(self.runtime_error(
                span,
                format!("file-system {operation} denied for {path}: path is outside the configured roots"),
            ))
        }
    }

    fn http_request(
        &self,
        method: &str,
        url: &str,
        request_headers: &[String],
        request_body: Option<&str>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if !matches!(method, "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD") {
            return Err(self.runtime_error(
                span,
                format!("http_request does not support method `{method}`"),
            ));
        }
        if matches!(method, "GET" | "HEAD") && request_body.is_some() {
            return Err(self.runtime_error(
                span,
                format!("http_request does not allow a body with {method}"),
            ));
        }
        let Some((scheme, authority_and_path)) = url.split_once("://") else {
            return Err(
                self.runtime_error(span, "http_request supports only http:// and https:// URLs")
            );
        };
        if !matches!(scheme, "http" | "https") {
            return Err(
                self.runtime_error(span, "http_request supports only http:// and https:// URLs")
            );
        }
        let authority_end = authority_and_path
            .find(['/', '?', '#'])
            .unwrap_or(authority_and_path.len());
        let authority = &authority_and_path[..authority_end];
        if authority.is_empty() || authority.contains('@') || authority.contains(['\r', '\n']) {
            return Err(self.runtime_error(span, "http_request received an invalid URL"));
        }
        let host = if let Some((host, port)) = authority.rsplit_once(':') {
            if port.parse::<u16>().is_err() {
                return Err(self.runtime_error(span, "http_request received an invalid port"));
            }
            host
        } else {
            authority
        };
        if host.is_empty() || host.contains(['/', '?', '#', '[', ']', '\\']) {
            return Err(self.runtime_error(span, "http_request received an invalid host"));
        }
        if let Some(policy) = &self.network_policy {
            let authority_allowed = policy
                .allowed_hosts
                .iter()
                .any(|allowed| allowed == authority || allowed == host);
            if !authority_allowed {
                return Err(self.runtime_error(
                    span,
                    format!("network access denied for host `{authority}`"),
                ));
            }
        }
        let policy = self.network_policy.as_ref();
        let timeout = Duration::from_millis(policy.map_or(5_000, |policy| policy.timeout_ms));
        let maximum = policy.map_or(1_048_576, |policy| policy.max_response_bytes);
        if maximum == 0 {
            return Err(self.runtime_error(
                span,
                "http_request requires a positive maximum response size",
            ));
        }
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .into();
        let mut builder = ureq::http::Request::builder().method(method).uri(url);
        for header in request_headers {
            let Some((name, value)) = header.split_once(':') else {
                return Err(self.runtime_error(
                    span,
                    "http_request headers must use the `Name: value` format",
                ));
            };
            let name = name.trim();
            let value = value.trim();
            if name.is_empty() || value.contains(['\r', '\n']) {
                return Err(self.runtime_error(span, "http_request received an invalid header"));
            }
            builder = builder.header(name, value);
        }
        let mut response = if let Some(body) = request_body {
            let request = builder.body(body.to_owned()).map_err(|error| {
                self.runtime_error(
                    span,
                    format!("http_request could not build request: {error}"),
                )
            })?;
            agent.run(request)
        } else {
            let request = builder.body(()).map_err(|error| {
                self.runtime_error(
                    span,
                    format!("http_request could not build request: {error}"),
                )
            })?;
            agent.run(request)
        }
        .map_err(|error| self.runtime_error(span, format!("network request failed: {error}")))?;
        let status = response.status().as_u16();
        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<usize>().ok());
        if content_length.is_some_and(|content_length| content_length > maximum) {
            return Err(self.runtime_error(
                span,
                format!("network response exceeds the {maximum} byte limit"),
            ));
        }
        let body = response
            .body_mut()
            .with_config()
            .limit(u64::try_from(maximum).unwrap_or(u64::MAX).saturating_add(1))
            .read_to_vec()
            .map_err(|error| {
                self.runtime_error(span, format!("network response failed: {error}"))
            })?;
        if body.len() > maximum {
            return Err(self.runtime_error(
                span,
                format!("network response exceeds the {maximum} byte limit"),
            ));
        }
        let body = String::from_utf8(body)
            .map_err(|_| self.runtime_error(span, "network response is not valid UTF-8"))?;
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                format!(
                    "{}: {}",
                    name,
                    value.to_str().unwrap_or("<non-utf8-header>")
                )
            })
            .map(Value::String)
            .collect();
        Ok(Value::Object {
            type_name: "HttpResponse".into(),
            fields: HashMap::from([
                ("status".into(), Value::Int(i64::from(status))),
                ("headers".into(), Value::Array(headers)),
                ("body".into(), Value::String(body)),
            ]),
        })
    }

    fn http_json(
        &self,
        method: &str,
        url: &str,
        request_headers: &[String],
        request_body: Option<&Value>,
        response_type: &Type,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let encoded_body = request_body
            .map(|value| self.json_encode(value, span))
            .transpose()?;
        let response =
            self.http_request(method, url, request_headers, encoded_body.as_deref(), span)?;
        let Value::Object { fields, .. } = response else {
            return Err(self.runtime_error(span, "http_json received an invalid response"));
        };
        let status = match fields.get("status") {
            Some(Value::Int(status)) => *status,
            _ => return Err(self.runtime_error(span, "http_json received an invalid status")),
        };
        if !(200..300).contains(&status) {
            return Err(
                self.runtime_error(span, format!("http_json received HTTP status {status}"))
            );
        }
        let Some(Value::String(body)) = fields.get("body") else {
            return Err(self.runtime_error(span, "http_json received an invalid body"));
        };
        self.json_decode(body, response_type, span)
    }

    fn http_result(
        &self,
        method: &str,
        url: &str,
        request_headers: &[String],
        request_body: Option<&Value>,
        response_type: &Type,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let encoded_body = request_body
            .map(|value| self.json_encode(value, span))
            .transpose()?;
        let response =
            self.http_request(method, url, request_headers, encoded_body.as_deref(), span)?;
        let Value::Object { fields, .. } = response else {
            return Err(self.runtime_error(span, "http_result received an invalid response"));
        };
        let status = match fields.get("status") {
            Some(Value::Int(status)) => *status,
            _ => return Err(self.runtime_error(span, "http_result received an invalid status")),
        };
        let headers = match fields.get("headers") {
            Some(Value::Array(headers)) => headers.clone(),
            _ => return Err(self.runtime_error(span, "http_result received invalid headers")),
        };
        let body = match fields.get("body") {
            Some(Value::String(body)) => body.clone(),
            _ => return Err(self.runtime_error(span, "http_result received an invalid body")),
        };
        let successful = (200..300).contains(&status);
        let data = if successful {
            let decoded = if matches!(response_type, Type::Unit) && body.trim().is_empty() {
                Value::Unit
            } else {
                self.json_decode(&body, response_type, span)?
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
            fields: HashMap::from([
                ("status".into(), Value::Int(status)),
                ("headers".into(), Value::Array(headers)),
                ("body".into(), Value::String(body)),
                ("data".into(), data),
                ("error".into(), error),
            ]),
        })
    }

    fn http_get(&self, url: &str, span: Span) -> Result<String, RuntimeError> {
        let response = self.http_request("GET", url, &[], None, span)?;
        let Value::Object { fields, .. } = response else {
            return Err(self.runtime_error(span, "http_get received an invalid response"));
        };
        let status = match fields.get("status") {
            Some(Value::Int(status)) => *status,
            _ => return Err(self.runtime_error(span, "http_get received an invalid status")),
        };
        if !(200..300).contains(&status) {
            return Err(self.runtime_error(span, format!("http_get received HTTP status {status}")));
        }
        match fields.get("body") {
            Some(Value::String(body)) => Ok(body.clone()),
            _ => Err(self.runtime_error(span, "http_get received an invalid body")),
        }
    }

    fn json_encode(&self, value: &Value, span: Span) -> Result<String, RuntimeError> {
        serde_json::to_string(&self.json_value(value, span)?).map_err(|error| {
            self.runtime_error(
                span,
                format!("json_encode failed to serialize value: {error}"),
            )
        })
    }

    fn json_value(&self, value: &Value, span: Span) -> Result<serde_json::Value, RuntimeError> {
        match value {
            Value::Int(value) => Ok(serde_json::Value::from(*value)),
            Value::UInt(value) => Ok(serde_json::Value::from(*value)),
            Value::Float(value) => serde_json::Number::from_f64(*value)
                .map(serde_json::Value::Number)
                .ok_or_else(|| {
                    self.runtime_error(span, "json_encode cannot serialize non-finite Float")
                }),
            Value::Bool(value) => Ok(serde_json::Value::from(*value)),
            Value::String(value) => Ok(serde_json::Value::String(value.clone())),
            Value::Char(value) => Ok(serde_json::Value::String(value.to_string())),
            Value::Timestamp(value) => Ok(serde_json::Value::from(*value)),
            Value::Array(values) => values
                .iter()
                .map(|value| self.json_value(value, span))
                .collect::<Result<Vec<_>, _>>()
                .map(serde_json::Value::Array),
            Value::Object { fields, .. } => fields
                .iter()
                .map(|(name, value)| {
                    self.json_value(value, span)
                        .map(|value| (name.clone(), value))
                })
                .collect::<Result<serde_json::Map<_, _>, _>>()
                .map(serde_json::Value::Object),
            Value::Option(Some(value)) => self.json_value(value, span),
            Value::Option(None) | Value::Unit => Ok(serde_json::Value::Null),
            Value::Result(Ok(value)) => self.json_value(value, span),
            Value::Result(Err(value)) => {
                let mut object = serde_json::Map::new();
                object.insert("error".into(), self.json_value(value, span)?);
                Ok(serde_json::Value::Object(object))
            }
            Value::Rows { columns, rows } => rows
                .iter()
                .map(|row| {
                    columns
                        .iter()
                        .zip(row)
                        .map(|(column, value)| {
                            Ok((column.clone(), serde_json::Value::String(value.clone())))
                        })
                        .collect::<Result<serde_json::Map<_, _>, RuntimeError>>()
                        .map(serde_json::Value::Object)
                })
                .collect::<Result<Vec<_>, _>>()
                .map(serde_json::Value::Array),
        }
    }

    fn json_decode(&self, source: &str, target: &Type, span: Span) -> Result<Value, RuntimeError> {
        let value: serde_json::Value = serde_json::from_str(source).map_err(|error| {
            self.runtime_error(span, format!("json_decode received invalid JSON: {error}"))
        })?;
        self.json_decode_value(&value, target, span)
    }

    fn json_decode_value(
        &self,
        value: &serde_json::Value,
        target: &Type,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if let Type::Option(inner) = target {
            if value.is_null() {
                return Ok(Value::Option(None));
            }
            return Ok(Value::Option(Some(Box::new(
                self.json_decode_value(value, inner, span)?,
            ))));
        }
        if let Type::Named(name) = target {
            if let Some(alias) = self.type_aliases.get(name) {
                return self.json_decode_value(value, alias, span);
            }
            if let Some(record) = self.record_definitions.get(name) {
                return self.json_decode_record(value, record, span);
            }
            if matches!(name.as_str(), "Id" | "Email" | "Url" | "Uuid" | "Money") {
                return self.json_decode_string(value, target, span);
            }
        }
        match target {
            Type::Int => value
                .as_i64()
                .map(Value::Int)
                .ok_or_else(|| self.json_type_error("Int", value, span)),
            Type::UInt => value
                .as_u64()
                .map(Value::UInt)
                .ok_or_else(|| self.json_type_error("UInt", value, span)),
            Type::Float | Type::Decimal => value
                .as_f64()
                .map(Value::Float)
                .ok_or_else(|| self.json_type_error("number", value, span)),
            Type::Bool => value
                .as_bool()
                .map(Value::Bool)
                .ok_or_else(|| self.json_type_error("Bool", value, span)),
            Type::String => self.json_decode_string(value, target, span),
            Type::Char => {
                let Value::String(string) = self.json_decode_string(value, target, span)? else {
                    return Err(self.runtime_error(span, "json_decode expected a String value"));
                };
                let mut chars = string.chars();
                let Some(character) = chars.next() else {
                    return Err(self.runtime_error(span, "json_decode expected one character"));
                };
                if chars.next().is_some() {
                    return Err(self.runtime_error(span, "json_decode expected one character"));
                }
                Ok(Value::Char(character))
            }
            Type::Timestamp => value
                .as_i64()
                .map(Value::Timestamp)
                .ok_or_else(|| self.json_type_error("Timestamp", value, span)),
            Type::Array(inner) => {
                let Some(values) = value.as_array() else {
                    return Err(self.json_type_error("array", value, span));
                };
                values
                    .iter()
                    .map(|value| self.json_decode_value(value, inner, span))
                    .collect::<Result<Vec<_>, _>>()
                    .map(Value::Array)
            }
            Type::Unit if value.is_null() => Ok(Value::Unit),
            Type::Unit => Err(self.json_type_error("null", value, span)),
            Type::Named(name) => Err(self.runtime_error(
                span,
                format!("json_decode does not support target type `{name}`"),
            )),
            Type::Result(_, _) | Type::Unknown | Type::Option(_) | Type::HttpResult(_) => Err(self
                .runtime_error(
                    span,
                    format!("json_decode does not support target type `{target}`"),
                )),
            Type::Bytes | Type::Date | Type::Time | Type::Duration => Err(self.runtime_error(
                span,
                format!("json_decode does not support target type `{target}`"),
            )),
        }
    }

    fn json_decode_string(
        &self,
        value: &serde_json::Value,
        target: &Type,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        value
            .as_str()
            .map(|value| Value::String(value.to_owned()))
            .ok_or_else(|| self.json_type_error(&target.to_string(), value, span))
    }

    fn json_decode_record(
        &self,
        value: &serde_json::Value,
        record: &RecordDef,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let Some(object) = value.as_object() else {
            return Err(self.json_type_error(&format!("{} object", record.name), value, span));
        };
        for field in object.keys() {
            if !record
                .fields
                .iter()
                .any(|candidate| candidate.name == *field)
            {
                return Err(self.runtime_error(
                    span,
                    format!(
                        "json_decode found unknown field `{field}` in `{}`",
                        record.name
                    ),
                ));
            }
        }
        let mut fields = HashMap::new();
        for field in &record.fields {
            let Some(value) = object.get(&field.name) else {
                if matches!(field.ty, Type::Option(_)) {
                    fields.insert(field.name.clone(), Value::Option(None));
                    continue;
                }
                return Err(self.runtime_error(
                    span,
                    format!(
                        "json_decode is missing field `{}` in `{}`",
                        field.name, record.name
                    ),
                ));
            };
            fields.insert(
                field.name.clone(),
                self.json_decode_value(value, &field.ty, span)?,
            );
        }
        Ok(Value::Object {
            type_name: record.name.clone(),
            fields,
        })
    }

    fn json_type_error(
        &self,
        expected: &str,
        value: &serde_json::Value,
        span: Span,
    ) -> RuntimeError {
        self.runtime_error(
            span,
            format!("json_decode expected {expected}, found JSON value `{value}`"),
        )
    }

    fn run_process(
        &self,
        command: &str,
        arguments: &[String],
        span: Span,
    ) -> Result<String, RuntimeError> {
        let Some(policy) = &self.process_policy else {
            return Err(self.runtime_error(
                span,
                "process execution denied: no process policy is configured",
            ));
        };
        if command.is_empty() || command.contains(['\r', '\n']) {
            return Err(self.runtime_error(span, "run_process received an invalid command"));
        }
        if !policy
            .allowed_commands
            .iter()
            .any(|allowed| allowed == command)
        {
            return Err(self.runtime_error(
                span,
                format!("process execution denied for command `{command}`"),
            ));
        }
        if policy.timeout_ms == 0 || policy.max_output_bytes == 0 {
            return Err(self.runtime_error(
                span,
                "process policy requires positive timeout and output limits",
            ));
        }
        let mut child = Command::new(command)
            .args(arguments)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                self.runtime_error(
                    span,
                    format!("process start failed for `{command}`: {error}"),
                )
            })?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| self.runtime_error(span, "process stdout pipe was unavailable"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| self.runtime_error(span, "process stderr pipe was unavailable"))?;
        let output_limit = u64::try_from(policy.max_output_bytes)
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        let stdout_reader = std::thread::spawn(move || {
            let mut output = Vec::new();
            stdout
                .take(output_limit)
                .read_to_end(&mut output)
                .map(|_| output)
        });
        let stderr_reader = std::thread::spawn(move || {
            let mut output = Vec::new();
            stderr
                .take(output_limit)
                .read_to_end(&mut output)
                .map(|_| output)
        });
        let timeout = Duration::from_millis(policy.timeout_ms);
        let deadline = Instant::now() + timeout;
        let mut timed_out = false;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() >= deadline => {
                    timed_out = true;
                    let _ = child.kill();
                    break child.wait().map_err(|error| {
                        self.runtime_error(span, format!("process termination failed: {error}"))
                    })?;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(5)),
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(self.runtime_error(span, format!("process status failed: {error}")));
                }
            }
        };
        let stdout = stdout_reader
            .join()
            .map_err(|_| self.runtime_error(span, "process stdout reader terminated unexpectedly"))?
            .map_err(|error| {
                self.runtime_error(span, format!("process stdout read failed: {error}"))
            })?;
        let stderr = stderr_reader
            .join()
            .map_err(|_| self.runtime_error(span, "process stderr reader terminated unexpectedly"))?
            .map_err(|error| {
                self.runtime_error(span, format!("process stderr read failed: {error}"))
            })?;
        if timed_out {
            return Err(self.runtime_error(
                span,
                format!(
                    "process `{command}` exceeded the {} ms timeout",
                    policy.timeout_ms
                ),
            ));
        }
        if stdout.len() > policy.max_output_bytes || stderr.len() > policy.max_output_bytes {
            return Err(self.runtime_error(
                span,
                format!(
                    "process `{command}` exceeded the {} byte output limit",
                    policy.max_output_bytes
                ),
            ));
        }
        if !status.success() {
            let error_output = String::from_utf8_lossy(&stderr);
            return Err(self.runtime_error(
                span,
                format!(
                    "process `{command}` exited unsuccessfully ({}): {}",
                    status,
                    error_output.trim()
                ),
            ));
        }
        String::from_utf8(stdout)
            .map_err(|_| self.runtime_error(span, "process stdout is not valid UTF-8"))
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
        if let Some(grants) = &self.granted_capabilities {
            if let Some(capability) = function
                .capabilities
                .iter()
                .find(|capability| !grants.contains(*capability))
            {
                return Err(self.runtime_error(
                    function.span,
                    format!("runtime capability denied: function `{name}` requires `{capability}`"),
                ));
            }
        }
        self.active_capabilities
            .push(function.capabilities.iter().cloned().collect());
        let result = (|| {
            let mut env = Environment::new();
            for (param, value) in function.params.iter().zip(args) {
                env.declare(param.name.clone(), value, false);
            }
            for contract in &function.requires {
                let value = self.eval(contract, &mut env)?;
                let condition = self.expect_bool(value, contract.span)?;
                if !condition {
                    return Err(self.runtime_error(
                        contract.span,
                        format!("precondition failed for function `{name}`"),
                    ));
                }
            }
            let result = match self.exec_block(&function.body, &mut env)? {
                Flow::Return(value) => value,
                Flow::Continue | Flow::LoopContinue | Flow::Break => Value::Unit,
            };
            if !function.ensures.is_empty() {
                env.declare("result".into(), result.clone(), false);
                for contract in &function.ensures {
                    let value = self.eval(contract, &mut env)?;
                    let condition = self.expect_bool(value, contract.span)?;
                    if !condition {
                        return Err(self.runtime_error(
                            contract.span,
                            format!("postcondition failed for function `{name}`"),
                        ));
                    }
                }
            }
            Ok(result)
        })();
        self.active_capabilities.pop();
        result
    }
    fn check_loop_invariants(
        &mut self,
        invariants: &[Expr],
        env: &mut Environment,
        span: Span,
        phase: &str,
    ) -> Result<(), RuntimeError> {
        for invariant in invariants {
            let value = self.eval(invariant, env)?;
            if !self.expect_bool(value, invariant.span)? {
                return Err(self.runtime_error(
                    span,
                    format!("loop invariant failed {phase} loop iteration"),
                ));
            }
        }
        Ok(())
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
                invariants,
                body,
                span,
            } => {
                loop {
                    self.check_loop_invariants(invariants, env, *span, "before")?;
                    let condition_value = self.eval(condition, env)?;
                    if !self.expect_bool(condition_value, *span)? {
                        break;
                    }
                    let flow = self.exec_block(body, env)?;
                    self.check_loop_invariants(invariants, env, *span, "after")?;
                    match flow {
                        Flow::Continue => {}
                        Flow::LoopContinue => continue,
                        Flow::Break => break,
                        flow @ Flow::Return(_) => return Ok(flow),
                    }
                }
                Ok(Flow::Continue)
            }
            Stmt::For {
                name,
                iterable,
                body,
                span,
            } => {
                let iterable = self.eval(iterable, env)?;
                let values = match iterable {
                    Value::Array(values) => values,
                    value => {
                        return Err(self.runtime_error(
                            *span,
                            format!("`for ... in` expects an array, found {}", value.ty()),
                        ));
                    }
                };
                for value in values {
                    env.push();
                    env.declare(name.clone(), value, false);
                    let flow = self.exec_block(body, env)?;
                    env.pop();
                    match flow {
                        Flow::Continue | Flow::LoopContinue => {}
                        Flow::Break => break,
                        flow @ Flow::Return(_) => return Ok(flow),
                    }
                }
                Ok(Flow::Continue)
            }
            Stmt::Loop {
                invariants,
                body,
                span,
            } => {
                loop {
                    self.check_loop_invariants(invariants, env, *span, "before")?;
                    let flow = self.exec_block(body, env)?;
                    self.check_loop_invariants(invariants, env, *span, "after")?;
                    match flow {
                        Flow::Continue => {}
                        Flow::LoopContinue => continue,
                        Flow::Break => break,
                        flow @ Flow::Return(_) => return Ok(flow),
                    }
                }
                Ok(Flow::Continue)
            }
            Stmt::Break { .. } => Ok(Flow::Break),
            Stmt::Continue { .. } => Ok(Flow::LoopContinue),
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
            Stmt::Parallel { body, span } => {
                let mut branches = Vec::new();
                let mut names = HashSet::new();
                for statement in &body.statements {
                    let Stmt::BindOrAssign { name, value, .. } = statement else {
                        return Err(self.runtime_error(
                            *span,
                            "parallel blocks may contain only `name = await expression` bindings",
                        ));
                    };
                    let ExprKind::Await(inner) = &value.kind else {
                        return Err(self.runtime_error(
                            value.span,
                            "parallel bindings must await their expression",
                        ));
                    };
                    if !names.insert(name.clone()) {
                        return Err(self.runtime_error(
                            value.span,
                            format!("parallel binding `{name}` is declared twice"),
                        ));
                    }
                    branches.push((name.clone(), (**inner).clone(), value.span));
                }
                let handles = branches
                    .into_iter()
                    .map(|(name, expression, span)| {
                        let mut child = Interpreter {
                            functions: self.functions.clone(),
                            output: Vec::new(),
                            steps: 0,
                            database_url: self.database_url.clone(),
                            granted_capabilities: self.granted_capabilities.clone(),
                            filesystem_policy: self.filesystem_policy.clone(),
                            network_policy: self.network_policy.clone(),
                            process_policy: self.process_policy.clone(),
                            record_definitions: self.record_definitions.clone(),
                            type_aliases: self.type_aliases.clone(),
                            active_capabilities: self.active_capabilities.clone(),
                        };
                        let mut child_env = env.clone();
                        std::thread::spawn(move || {
                            child
                                .eval(&expression, &mut child_env)
                                .map(|value| (name, value, child.output))
                                .map_err(|error| (span, error))
                        })
                    })
                    .collect::<Vec<_>>();
                let mut results = Vec::new();
                let mut first_error = None;
                for handle in handles {
                    match handle.join() {
                        Ok(Ok(result)) => results.push(result),
                        Ok(Err((_, error))) => {
                            if first_error.is_none() {
                                first_error = Some(error);
                            }
                        }
                        Err(_) => {
                            if first_error.is_none() {
                                first_error =
                                    Some(self.runtime_error(*span, "parallel task panicked"));
                            }
                        }
                    }
                }
                if let Some(error) = first_error {
                    return Err(error);
                }
                for (name, value, output) in results {
                    self.output.extend(output);
                    if env.contains(&name) {
                        env.assign(&name, value).map_err(|reason| {
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
                        env.declare(name, value, false);
                    }
                }
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
            ExprKind::Array(values) => values
                .iter()
                .map(|value| self.eval(value, env))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            ExprKind::Record { type_name, fields } => {
                let mut values = HashMap::new();
                for (field, value) in fields {
                    values.insert(field.clone(), self.eval(value, env)?);
                }
                Ok(Value::Object {
                    type_name: type_name.clone(),
                    fields: values,
                })
            }
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
            ExprKind::Call {
                name,
                type_args,
                args,
            } => {
                if name == "print" {
                    let value = self.eval(
                        args.first().ok_or_else(|| {
                            self.runtime_error(expr.span, "`print` expects one argument")
                        })?,
                        env,
                    )?;
                    self.output.push(value.output());
                    Ok(Value::Unit)
                } else if name == "len" {
                    if args.len() != 1 {
                        return Err(
                            self.runtime_error(expr.span, "`len` expects exactly one argument")
                        );
                    }
                    match self.eval(&args[0], env)? {
                        Value::Array(values) => Ok(Value::Int(values.len() as i64)),
                        value => Err(self.runtime_error(
                            expr.span,
                            format!("`len` expects an array, found {}", value.ty()),
                        )),
                    }
                } else if name == "append" {
                    if args.len() != 2 {
                        return Err(self
                            .runtime_error(expr.span, "`append` expects an array and one value"));
                    }
                    let array = self.eval(&args[0], env)?;
                    let value = self.eval(&args[1], env)?;
                    match array {
                        Value::Array(mut values) => {
                            values.push(value);
                            Ok(Value::Array(values))
                        }
                        value => Err(self.runtime_error(
                            expr.span,
                            format!("`append` expects an array, found {}", value.ty()),
                        )),
                    }
                } else if name == "contains" {
                    if args.len() != 2 {
                        return Err(self.runtime_error(
                            expr.span,
                            "`contains` expects an array and one value",
                        ));
                    }
                    let array = self.eval(&args[0], env)?;
                    let value = self.eval(&args[1], env)?;
                    match array {
                        Value::Array(values) => {
                            Ok(Value::Bool(values.iter().any(|item| item == &value)))
                        }
                        value => Err(self.runtime_error(
                            expr.span,
                            format!("`contains` expects an array, found {}", value.ty()),
                        )),
                    }
                } else if name == "first" || name == "last" {
                    if args.len() != 1 {
                        return Err(self.runtime_error(
                            expr.span,
                            format!("`{name}` expects exactly one argument"),
                        ));
                    }
                    match self.eval(&args[0], env)? {
                        Value::Array(values) => {
                            let value = if name == "first" {
                                values.first()
                            } else {
                                values.last()
                            };
                            Ok(Value::Option(value.cloned().map(Box::new)))
                        }
                        value => Err(self.runtime_error(
                            expr.span,
                            format!("`{name}` expects an array, found {}", value.ty()),
                        )),
                    }
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
                } else if name == "now" {
                    if !args.is_empty() {
                        return Err(self.runtime_error(expr.span, "now expects no arguments"));
                    }
                    self.require_runtime_capability("Clock", "clock access", expr.span)?;
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_err(|error| {
                            self.runtime_error(
                                expr.span,
                                format!("clock access failed: system time is before Unix epoch: {error}"),
                            )
                        })?
                        .as_millis();
                    let timestamp = i64::try_from(timestamp).map_err(|_| {
                        self.runtime_error(expr.span, "clock access failed: timestamp overflow")
                    })?;
                    Ok(Value::Timestamp(timestamp))
                } else if name == "env" {
                    if args.len() != 1 {
                        return Err(
                            self.runtime_error(expr.span, "env expects exactly one argument")
                        );
                    }
                    self.require_runtime_capability(
                        "Environment",
                        "environment access",
                        expr.span,
                    )?;
                    let key = self.eval(&args[0], env)?;
                    let Value::String(key) = key else {
                        return Err(
                            self.runtime_error(args[0].span, "env expects a String variable name")
                        );
                    };
                    if key.is_empty() {
                        return Err(self
                            .runtime_error(args[0].span, "env expects a non-empty variable name"));
                    }
                    let value = std::env::var_os(&key)
                        .map(|value| {
                            value.into_string().map_err(|_| {
                                self.runtime_error(
                                    args[0].span,
                                    format!("environment variable {key} is not valid UTF-8"),
                                )
                            })
                        })
                        .transpose()?;
                    Ok(Value::Option(
                        value.map(|value| Box::new(Value::String(value))),
                    ))
                } else if name == "random_int" {
                    if args.len() != 2 {
                        return Err(self.runtime_error(
                            expr.span,
                            "random_int expects minimum and maximum Int arguments",
                        ));
                    }
                    self.require_runtime_capability("Random", "random access", expr.span)?;
                    let minimum = self.eval(&args[0], env)?;
                    let maximum = self.eval(&args[1], env)?;
                    let (Value::Int(minimum), Value::Int(maximum)) = (minimum, maximum) else {
                        return Err(
                            self.runtime_error(expr.span, "random_int expects Int arguments")
                        );
                    };
                    if minimum > maximum {
                        return Err(
                            self.runtime_error(expr.span, "random_int requires minimum <= maximum")
                        );
                    }
                    let range = (i128::from(maximum) - i128::from(minimum) + 1) as u128;
                    let limit = u128::from(u64::MAX) + 1;
                    let cutoff = limit - (limit % range);
                    let mut random = OsRng;
                    loop {
                        let mut bytes = [0_u8; 8];
                        random.try_fill_bytes(&mut bytes).map_err(|error| {
                            self.runtime_error(expr.span, format!("random access failed: {error}"))
                        })?;
                        let candidate = u128::from(u64::from_ne_bytes(bytes));
                        if candidate < cutoff {
                            let value = i128::from(minimum) + (candidate % range) as i128;
                            return Ok(Value::Int(value as i64));
                        }
                    }
                } else if name == "http_get" {
                    if args.len() != 1 {
                        return Err(self
                            .runtime_error(expr.span, "http_get expects exactly one String URL"));
                    }
                    self.require_runtime_capability("Network", "network access", expr.span)?;
                    let url = self.eval(&args[0], env)?;
                    let Value::String(url) = url else {
                        return Err(
                            self.runtime_error(args[0].span, "http_get expects a String URL")
                        );
                    };
                    self.http_get(&url, expr.span).map(Value::String)
                } else if name == "json_encode" {
                    if args.len() != 1 {
                        return Err(
                            self.runtime_error(expr.span, "json_encode expects exactly one value")
                        );
                    }
                    let value = self.eval(&args[0], env)?;
                    self.json_encode(&value, expr.span).map(Value::String)
                } else if name == "json_decode" {
                    if type_args.len() != 1 || args.len() != 1 {
                        return Err(self.runtime_error(
                            expr.span,
                            "json_decode expects one type argument and one String value",
                        ));
                    }
                    let value = self.eval(&args[0], env)?;
                    let Value::String(source) = value else {
                        return Err(
                            self.runtime_error(args[0].span, "json_decode expects a String value")
                        );
                    };
                    self.json_decode(&source, &type_args[0], expr.span)
                } else if name == "http_json" {
                    if type_args.len() != 2 || args.len() != 4 {
                        return Err(self.runtime_error(
                            expr.span,
                            "http_json expects request and response type arguments plus four values",
                        ));
                    }
                    self.require_runtime_capability("Network", "network access", expr.span)?;
                    let method = self.eval(&args[0], env)?;
                    let url = self.eval(&args[1], env)?;
                    let headers = self.eval(&args[2], env)?;
                    let body = self.eval(&args[3], env)?;
                    let Value::String(method) = method else {
                        return Err(
                            self.runtime_error(args[0].span, "http_json expects a String method")
                        );
                    };
                    let Value::String(url) = url else {
                        return Err(
                            self.runtime_error(args[1].span, "http_json expects a String URL")
                        );
                    };
                    let Value::Array(headers) = headers else {
                        return Err(
                            self.runtime_error(args[2].span, "http_json expects String[] headers")
                        );
                    };
                    let mut header_strings = Vec::with_capacity(headers.len() + 1);
                    let mut has_content_type = false;
                    for header in headers {
                        let Value::String(header) = header else {
                            return Err(self.runtime_error(
                                args[2].span,
                                "http_json expects String[] headers",
                            ));
                        };
                        if header.split_once(':').is_some_and(|(name, _)| {
                            name.trim().eq_ignore_ascii_case("content-type")
                        }) {
                            has_content_type = true;
                        }
                        header_strings.push(header);
                    }
                    if !has_content_type {
                        header_strings.push("Content-Type: application/json".into());
                    }
                    let body = match body {
                        Value::Option(Some(body)) => Some(body),
                        Value::Option(None) => None,
                        _ => {
                            return Err(self
                                .runtime_error(args[3].span, "http_json expects a Request? body"));
                        }
                    };
                    self.http_json(
                        &method,
                        &url,
                        &header_strings,
                        body.as_deref(),
                        &type_args[1],
                        expr.span,
                    )
                } else if name == "http_result" {
                    if type_args.len() != 2 || args.len() != 4 {
                        return Err(self.runtime_error(
                            expr.span,
                            "http_result expects request and response type arguments plus four values",
                        ));
                    }
                    self.require_runtime_capability("Network", "network access", expr.span)?;
                    let method = self.eval(&args[0], env)?;
                    let url = self.eval(&args[1], env)?;
                    let headers = self.eval(&args[2], env)?;
                    let body = self.eval(&args[3], env)?;
                    let Value::String(method) = method else {
                        return Err(
                            self.runtime_error(args[0].span, "http_result expects a String method")
                        );
                    };
                    let Value::String(url) = url else {
                        return Err(
                            self.runtime_error(args[1].span, "http_result expects a String URL")
                        );
                    };
                    let Value::Array(headers) = headers else {
                        return Err(self
                            .runtime_error(args[2].span, "http_result expects String[] headers"));
                    };
                    let mut header_strings = Vec::with_capacity(headers.len() + 1);
                    let mut has_content_type = false;
                    for header in headers {
                        let Value::String(header) = header else {
                            return Err(self.runtime_error(
                                args[2].span,
                                "http_result expects String[] headers",
                            ));
                        };
                        if header.split_once(':').is_some_and(|(name, _)| {
                            name.trim().eq_ignore_ascii_case("content-type")
                        }) {
                            has_content_type = true;
                        }
                        header_strings.push(header);
                    }
                    if !has_content_type {
                        header_strings.push("Content-Type: application/json".into());
                    }
                    let body = match body {
                        Value::Option(Some(body)) => Some(body),
                        Value::Option(None) => None,
                        _ => {
                            return Err(self.runtime_error(
                                args[3].span,
                                "http_result expects a Request? body",
                            ));
                        }
                    };
                    self.http_result(
                        &method,
                        &url,
                        &header_strings,
                        body.as_deref(),
                        &type_args[1],
                        expr.span,
                    )
                } else if name == "http_request" {
                    if args.len() != 4 {
                        return Err(self.runtime_error(
                            expr.span,
                            "http_request expects String method, String URL, String[] headers, and String? body",
                        ));
                    }
                    self.require_runtime_capability("Network", "network access", expr.span)?;
                    let method = self.eval(&args[0], env)?;
                    let url = self.eval(&args[1], env)?;
                    let headers = self.eval(&args[2], env)?;
                    let body = self.eval(&args[3], env)?;
                    let Value::String(method) = method else {
                        return Err(self
                            .runtime_error(args[0].span, "http_request expects a String method"));
                    };
                    let Value::String(url) = url else {
                        return Err(
                            self.runtime_error(args[1].span, "http_request expects a String URL")
                        );
                    };
                    let Value::Array(headers) = headers else {
                        return Err(self
                            .runtime_error(args[2].span, "http_request expects String[] headers"));
                    };
                    let mut header_strings = Vec::with_capacity(headers.len());
                    for header in headers {
                        let Value::String(header) = header else {
                            return Err(self.runtime_error(
                                args[2].span,
                                "http_request expects String[] headers",
                            ));
                        };
                        header_strings.push(header);
                    }
                    let body = match body {
                        Value::Option(Some(body)) => match *body {
                            Value::String(body) => Some(body),
                            _ => {
                                return Err(self.runtime_error(
                                    args[3].span,
                                    "http_request expects a String? body",
                                ));
                            }
                        },
                        Value::Option(None) => None,
                        _ => {
                            return Err(self.runtime_error(
                                args[3].span,
                                "http_request expects a String? body",
                            ));
                        }
                    };
                    self.http_request(&method, &url, &header_strings, body.as_deref(), expr.span)
                } else if name == "run_process" {
                    if args.len() != 2 {
                        return Err(self.runtime_error(
                            expr.span,
                            "run_process expects a String command and String[] arguments",
                        ));
                    }
                    self.require_runtime_capability("Process", "process execution", expr.span)?;
                    let command = self.eval(&args[0], env)?;
                    let arguments = self.eval(&args[1], env)?;
                    let Value::String(command) = command else {
                        return Err(self
                            .runtime_error(args[0].span, "run_process expects a String command"));
                    };
                    let Value::Array(arguments) = arguments else {
                        return Err(self.runtime_error(
                            args[1].span,
                            "run_process expects String[] arguments",
                        ));
                    };
                    let mut argument_strings = Vec::with_capacity(arguments.len());
                    for argument in arguments {
                        let Value::String(argument) = argument else {
                            return Err(self.runtime_error(
                                args[1].span,
                                "run_process expects String[] arguments",
                            ));
                        };
                        argument_strings.push(argument);
                    }
                    self.run_process(&command, &argument_strings, expr.span)
                        .map(Value::String)
                } else if name == "read_text" {
                    if args.len() != 1 {
                        return Err(self.runtime_error(
                            expr.span,
                            "read_text expects exactly one String path",
                        ));
                    }
                    self.require_runtime_capability(
                        "FileSystem",
                        "file-system read access",
                        expr.span,
                    )?;
                    let path = self.eval(&args[0], env)?;
                    let Value::String(path) = path else {
                        return Err(
                            self.runtime_error(args[0].span, "read_text expects a String path")
                        );
                    };
                    if path.is_empty() {
                        return Err(
                            self.runtime_error(args[0].span, "read_text expects a non-empty path")
                        );
                    }
                    let path = self.resolve_filesystem_path(
                        &path,
                        &self
                            .filesystem_policy
                            .as_ref()
                            .map_or_else(Vec::new, |policy| policy.read_roots.clone()),
                        "read",
                        expr.span,
                        false,
                    )?;
                    std::fs::read_to_string(&path)
                        .map_err(|error| {
                            self.runtime_error(
                                expr.span,
                                format!("file-system read failed: {error}"),
                            )
                        })
                        .map(Value::String)
                } else if name == "write_text" {
                    if args.len() != 2 {
                        return Err(self.runtime_error(
                            expr.span,
                            "write_text expects a String path and String content",
                        ));
                    }
                    self.require_runtime_capability(
                        "FileSystem",
                        "file-system write access",
                        expr.span,
                    )?;
                    let path = self.eval(&args[0], env)?;
                    let content = self.eval(&args[1], env)?;
                    let (Value::String(path), Value::String(content)) = (path, content) else {
                        return Err(
                            self.runtime_error(expr.span, "write_text expects String arguments")
                        );
                    };
                    if path.is_empty() {
                        return Err(
                            self.runtime_error(args[0].span, "write_text expects a non-empty path")
                        );
                    }
                    let roots = self
                        .filesystem_policy
                        .as_ref()
                        .map_or_else(Vec::new, |policy| policy.write_roots.clone());
                    let path =
                        self.resolve_filesystem_path(&path, &roots, "write", expr.span, true)?;
                    std::fs::write(&path, content).map_err(|error| {
                        self.runtime_error(expr.span, format!("file-system write failed: {error}"))
                    })?;
                    Ok(Value::Unit)
                } else if name == "delete_file" {
                    if args.len() != 1 {
                        return Err(
                            self.runtime_error(expr.span, "delete_file expects one String path")
                        );
                    }
                    self.require_runtime_capability(
                        "FileSystem",
                        "file-system delete access",
                        expr.span,
                    )?;
                    let path = self.eval(&args[0], env)?;
                    let Value::String(path) = path else {
                        return Err(
                            self.runtime_error(args[0].span, "delete_file expects a String path")
                        );
                    };
                    if path.is_empty() {
                        return Err(self
                            .runtime_error(args[0].span, "delete_file expects a non-empty path"));
                    }
                    let roots = self
                        .filesystem_policy
                        .as_ref()
                        .map_or_else(Vec::new, |policy| policy.write_roots.clone());
                    let path =
                        self.resolve_filesystem_path(&path, &roots, "delete", expr.span, false)?;
                    std::fs::remove_file(&path).map_err(|error| {
                        self.runtime_error(expr.span, format!("file-system delete failed: {error}"))
                    })?;
                    Ok(Value::Unit)
                } else if name == "list_dir" {
                    if args.len() != 1 {
                        return Err(
                            self.runtime_error(expr.span, "list_dir expects one String path")
                        );
                    }
                    self.require_runtime_capability(
                        "FileSystem",
                        "file-system read access",
                        expr.span,
                    )?;
                    let path = self.eval(&args[0], env)?;
                    let Value::String(path) = path else {
                        return Err(
                            self.runtime_error(args[0].span, "list_dir expects a String path")
                        );
                    };
                    if path.is_empty() {
                        return Err(
                            self.runtime_error(args[0].span, "list_dir expects a non-empty path")
                        );
                    }
                    let roots = self
                        .filesystem_policy
                        .as_ref()
                        .map_or_else(Vec::new, |policy| policy.read_roots.clone());
                    let path =
                        self.resolve_filesystem_path(&path, &roots, "list", expr.span, false)?;
                    let mut entries = std::fs::read_dir(&path)
                        .map_err(|error| {
                            self.runtime_error(
                                expr.span,
                                format!("file-system list failed: {error}"),
                            )
                        })?
                        .map(|entry| {
                            entry
                                .map(|entry| {
                                    Value::String(entry.file_name().to_string_lossy().into_owned())
                                })
                                .map_err(|error| {
                                    self.runtime_error(
                                        expr.span,
                                        format!("file-system list failed: {error}"),
                                    )
                                })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    entries.sort_by_key(Value::output);
                    Ok(Value::Array(entries))
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
            ExprKind::Index { target, index } => {
                let target = self.eval(target, env)?;
                let index = self.eval(index, env)?;
                let index = match index {
                    Value::Int(value) if value >= 0 => value as usize,
                    Value::UInt(value) => value as usize,
                    value => {
                        return Err(self.runtime_error(
                            expr.span,
                            format!("array index must be Int or UInt, found {}", value.ty()),
                        ));
                    }
                };
                match target {
                    Value::Array(values) => values.get(index).cloned().ok_or_else(|| {
                        self.runtime_error(
                            expr.span,
                            format!("array index {index} is out of bounds"),
                        )
                    }),
                    value => Err(self.runtime_error(
                        expr.span,
                        format!("array indexing requires an array, found {}", value.ty()),
                    )),
                }
            }
            ExprKind::Field { target, field } => {
                let value = self.eval(target, env)?;
                match value {
                    Value::Object { fields, .. } => fields.get(field).cloned().ok_or_else(|| {
                        self.runtime_error(expr.span, format!("record field `{field}` is missing"))
                    }),
                    value => Err(self.runtime_error(
                        expr.span,
                        format!("field access requires a record, found {}", value.ty()),
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
                self.require_runtime_capability("Database", "SQL access", expr.span)?;
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
            ExprKind::Await(inner) => self.eval(inner, env),
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
            (Value::Array(mut a), Add, Value::Array(b)) => {
                a.extend(b);
                Ok(Value::Array(a))
            }
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
        Value::Timestamp(value) => Ok(QueryValue::Int(*value)),
        Value::Option(None) | Value::Unit => Ok(QueryValue::Null),
        Value::Option(Some(value)) => value_to_query_value(value, span),
        Value::Array(_) | Value::Object { .. } | Value::Rows { .. } | Value::Result(_) => {
            Err(RuntimeError {
                message: "SQL parameters must be scalar values".into(),
                span,
            })
        }
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
    use std::net::TcpListener;
    use std::thread;
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
    fn supports_array_literals_indexing_length_append_and_concatenation() {
        let output = run(
            "fn main() { values = [1, 2] extended = append(values, 3) combined = extended + [4] print(combined[2]) print(len(combined)) }",
        );
        assert_eq!(output, ["3", "4"]);
    }

    #[test]
    fn supports_array_for_iteration_and_loop_control() {
        let output = run(
            "fn main() { mutable total = 0 for value in [1, 2, 3, 4] { if value == 2 { continue } if value == 4 { break } total = total + value } print(total) }",
        );
        assert_eq!(output, ["4"]);
    }

    #[test]
    fn executes_parallel_await_bindings_and_merges_results() {
        let output = run(
            "fn load_customer() -> String { return \"customer\" } fn load_orders() -> Int { return 3 } fn main() { parallel { customer = await load_customer() orders = await load_orders() } print(customer) print(orders) }",
        );
        assert_eq!(output, ["customer", "3"]);
    }

    #[test]
    fn rejects_await_outside_parallel_block() {
        let program = parse(
            &lex("fn load() -> Int { return 1 } fn main() { value = await load() }").unwrap(),
        )
        .unwrap();
        let errors = check(&program).unwrap_err();
        assert!(errors.iter().any(|error| error
            .message
            .contains("only valid inside a `parallel` block")));
    }

    #[test]
    fn rejects_parallel_bindings_without_await() {
        let program = parse(
            &lex("fn load() -> Int { return 1 } fn main() { parallel { value = load() } }")
                .unwrap(),
        )
        .unwrap();
        let errors = check(&program).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("must await their expression")));
    }

    #[test]
    fn rejects_non_array_for_iteration() {
        let program =
            parse(&lex("fn main() { for value in 1 { print(value) } }").unwrap()).unwrap();
        let errors = check(&program).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("expects an array")));
    }

    #[test]
    fn supports_record_literals_field_access_and_array_queries() {
        let output = run(
            "struct Address { city: String } struct Customer { name: String address: Address } fn main() { customer = Customer { name: \"Anna\", address: Address { city: \"Berlin\" } } print(customer.address.city) print(contains([1, 2, 3], 2)) print(first([4, 5])) print(last([4, 5])) }",
        );
        assert_eq!(output, ["Berlin", "true", "Some(4)", "Some(5)"]);
    }

    #[test]
    fn rejects_unknown_record_fields() {
        let program = parse(
            &lex("struct Address { city: String } fn main() { address = Address { country: \"DE\" } }").unwrap(),
        )
        .unwrap();
        let errors = check(&program).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("unknown field `country`")));
    }

    #[test]
    fn rejects_array_index_out_of_bounds_at_runtime() {
        let program = parse(&lex("fn main() { values = [1] print(values[1]) }").unwrap()).unwrap();
        check(&program).unwrap();
        let error = execute(&program).unwrap_err();
        assert!(error.message.contains("out of bounds"));
    }

    #[test]
    fn loop_continue_skips_remaining_body() {
        let output = run(
            "fn main() { mutable i = 0 mutable sum = 0 while i < 5 { i = i + 1 if i == 3 { continue } sum = sum + i } print(sum) }",
        );
        assert_eq!(output, ["12"]);
    }

    #[test]
    fn loop_break_exits_the_current_loop() {
        let output = run("fn main() { mutable i = 0 while i < 5 { i = i + 1 break } print(i) }");
        assert_eq!(output, ["1"]);
    }

    #[test]
    fn rejects_loop_continue_outside_a_loop() {
        let program = parse(&lex("fn main() { continue }").unwrap()).unwrap();
        let errors = check(&program).unwrap_err();
        assert!(errors.iter().any(|error| error
            .message
            .contains("`continue` is only valid inside a loop")));
    }

    #[test]
    fn checks_loop_invariants_at_runtime() {
        let output = run(
            "fn main() { mutable i = 0 while i < 2 invariant { i >= 0 } { i = i + 1 } print(i) }",
        );
        assert_eq!(output, ["2"]);

        let program = parse(
            &lex("fn main() { mutable i = 0 while i < 1 invariant { i > 0 } { i = i + 1 } }")
                .unwrap(),
        )
        .unwrap();
        check(&program).unwrap();
        let error = execute(&program).unwrap_err();
        assert!(error.message.contains("loop invariant failed"));
    }

    #[test]
    fn checks_invariants_on_unconditional_loops() {
        let output = run(
            "fn main() { mutable i = 0 loop invariant { i >= 0 } { i = i + 1 if i == 2 { break } } print(i) }",
        );
        assert_eq!(output, ["2"]);
    }

    #[test]
    fn rejects_non_boolean_loop_invariants() {
        let program =
            parse(&lex("fn main() { while true invariant { 1 } { break } }").unwrap()).unwrap();
        let errors = check(&program).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("expected `Bool`")));
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

    #[test]
    fn accepts_declared_database_capability_for_sql() {
        let program = parse(
            &lex("fn load() uses Database { rows = sql<Int> { SELECT 1 } } fn main() uses Database { load() }").unwrap(),
        )
        .unwrap();
        check_capabilities(&program).unwrap();
    }

    #[test]
    fn rejects_sql_without_database_capability() {
        let program = parse(&lex("fn main() { rows = sql<Int> { SELECT 1 } }").unwrap()).unwrap();
        let errors = check_capabilities(&program).unwrap_err();
        assert!(errors.iter().any(|error| {
            error
                .message
                .contains("does not declare capability `Database`")
        }));
    }

    #[test]
    fn propagates_called_function_capabilities() {
        let program =
            parse(&lex("fn send() uses Network { print(1) } fn main() { send() }").unwrap())
                .unwrap();
        let errors = check_capabilities(&program).unwrap_err();
        assert!(errors.iter().any(|error| {
            error
                .message
                .contains("calls `send`, which requires capability `Network`")
        }));
    }

    #[test]
    fn rejects_unknown_and_duplicate_capabilities() {
        let program =
            parse(&lex("fn main() uses Network, Network, Telepathy { print(1) }").unwrap())
                .unwrap();
        let errors = check_capabilities(&program).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("declared more than once")));
        assert!(errors
            .iter()
            .any(|error| error.message.contains("unknown capability `Telepathy`")));
    }

    #[test]
    fn rejects_capability_not_granted_by_project() {
        let program = parse(&lex("fn main() uses Network { print(1) }").unwrap()).unwrap();
        let grants = HashSet::new();
        let errors = check_capabilities_with_grants(&program, Some(&grants)).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("not enabled by the project")));
    }

    #[test]
    fn enforces_runtime_capability_for_called_function() {
        let program =
            parse(&lex("fn send() uses Network { print(1) } fn main() { send() }").unwrap())
                .unwrap();
        let grants = HashSet::new();
        let error = execute_with_capabilities(&program, Some(&grants)).unwrap_err();
        assert!(error.message.contains("function `send` requires `Network`"));
    }

    #[test]
    fn enforces_runtime_database_capability_before_connecting() {
        let program = parse(&lex("fn main() { rows = sql<Int> { SELECT 1 } }").unwrap()).unwrap();
        let grants = HashSet::new();
        let error = execute_with_database_and_capabilities(
            &program,
            "mariadb://invalid:invalid@127.0.0.1:1/invalid",
            Some(&grants),
        )
        .unwrap_err();
        assert!(error
            .message
            .contains("SQL access requires `Database` in the current function"));
    }

    #[test]
    fn accepts_runtime_capability_grant() {
        let program = parse(&lex("fn main() uses Network { print(1) }").unwrap()).unwrap();
        let grants = HashSet::from([String::from("Network")]);
        assert_eq!(
            execute_with_capabilities(&program, Some(&grants)).unwrap(),
            ["1"]
        );
    }

    #[test]
    fn requires_network_capability_for_http_get() {
        let program =
            parse(&lex("fn main() { print(http_get(\"http://example.test\")) }").unwrap()).unwrap();
        let errors = check_capabilities(&program).unwrap_err();
        assert!(errors.iter().any(|error| error
            .message
            .contains("does not declare capability `Network`")));
    }

    #[test]
    fn requires_network_capability_for_http_json() {
        let source = "struct Customer { name: String } fn main() { customer = http_json<Customer, Customer>(\"POST\", \"http://example.test\", [], None) }";
        let program = parse(&lex(source).unwrap()).unwrap();
        check(&program).unwrap();
        let errors = check_capabilities(&program).unwrap_err();
        assert!(errors.iter().any(|error| error
            .message
            .contains("does not declare capability `Network`")));
    }

    #[test]
    fn performs_http_get_with_network_policy() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).unwrap();
            std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
            )
            .unwrap();
        });
        let source =
            "fn fetch(url: String) -> String uses Network { return http_get(url) } fn main() { }";
        let program = parse(&lex(source).unwrap()).unwrap();
        check(&program).unwrap();
        check_capabilities(&program).unwrap();
        let grants = HashSet::from([String::from("Network")]);
        let policy = RuntimePolicy {
            filesystem: None,
            network: Some(NetworkPolicy {
                allowed_hosts: vec![format!("127.0.0.1:{port}")],
                timeout_ms: 1_000,
                max_response_bytes: 100,
            }),
            process: None,
        };
        let body = execute_function_with_capabilities_and_policies(
            &program,
            "fetch",
            vec![Value::String(format!("http://127.0.0.1:{port}/health"))],
            None,
            Some(&grants),
            Some(&policy),
        )
        .unwrap();
        assert_eq!(body, Value::String("hello".into()));
        server.join().unwrap();
    }

    #[test]
    fn rejects_http_get_outside_network_allowlist() {
        let source =
            "fn fetch() -> String uses Network { return http_get(\"http://example.test\") } fn main() { }";
        let program = parse(&lex(source).unwrap()).unwrap();
        check(&program).unwrap();
        let grants = HashSet::from([String::from("Network")]);
        let policy = RuntimePolicy {
            filesystem: None,
            network: Some(NetworkPolicy {
                allowed_hosts: Vec::new(),
                timeout_ms: 100,
                max_response_bytes: 100,
            }),
            process: None,
        };
        let error = execute_function_with_capabilities_and_policies(
            &program,
            "fetch",
            Vec::new(),
            None,
            Some(&grants),
            Some(&policy),
        )
        .unwrap_err();
        assert!(error.message.contains("network access denied"));
    }

    #[test]
    fn sends_typed_http_request_and_returns_structured_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut chunk = [0_u8; 512];
            while !request
                .windows(b"{\"name\":\"Anna\"}".len())
                .any(|window| window == b"{\"name\":\"Anna\"}")
            {
                let size = stream.read(&mut chunk).unwrap();
                if size == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..size]);
            }
            let request = String::from_utf8_lossy(&request);
            let request_lower = request.to_ascii_lowercase();
            assert!(request.starts_with("POST /customers HTTP/1.1"));
            assert!(request_lower.contains("x-request-id: test-1"));
            assert!(request.contains("{\"name\":\"Anna\"}"));
            std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 201 Created\r\nX-Request-ID: test-1\r\nContent-Length: 7\r\nConnection: close\r\n\r\ncreated",
            )
            .unwrap();
        });
        let source =
            "fn request(url: String) -> HttpResponse uses Network { return http_request(\"POST\", url, [\"X-Request-ID: test-1\", \"Content-Type: application/json\"], Some(\"{\\\"name\\\":\\\"Anna\\\"}\")) } fn status(url: String) -> Int uses Network { response = request(url) return response.status } fn main() { }";
        let program = parse(&lex(source).unwrap()).unwrap();
        check(&program).unwrap();
        check_capabilities(&program).unwrap();
        let grants = HashSet::from([String::from("Network")]);
        let policy = RuntimePolicy {
            filesystem: None,
            network: Some(NetworkPolicy {
                allowed_hosts: vec![format!("127.0.0.1:{port}")],
                timeout_ms: 1_000,
                max_response_bytes: 100,
            }),
            process: None,
        };
        let response = execute_function_with_capabilities_and_policies(
            &program,
            "request",
            vec![Value::String(format!("http://127.0.0.1:{port}/customers"))],
            None,
            Some(&grants),
            Some(&policy),
        )
        .unwrap();
        let Value::Object { fields, .. } = response else {
            panic!("expected HttpResponse object");
        };
        assert_eq!(fields.get("status"), Some(&Value::Int(201)));
        assert_eq!(fields.get("body"), Some(&Value::String("created".into())));
        assert!(
            matches!(fields.get("headers"), Some(Value::Array(headers)) if headers.iter().any(|header| header == &Value::String("x-request-id: test-1".into())))
        );
        server.join().unwrap();
    }

    #[test]
    fn sends_and_decodes_typed_json_http_request() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut chunk = [0_u8; 512];
            while !request
                .windows(b"{\"name\":\"Anna\"}".len())
                .any(|window| window == b"{\"name\":\"Anna\"}")
            {
                let size = stream.read(&mut chunk).unwrap();
                if size == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..size]);
            }
            let request = String::from_utf8_lossy(&request);
            let request_lower = request.to_ascii_lowercase();
            assert!(request.starts_with("POST /customers HTTP/1.1"));
            assert!(request_lower.contains("content-type: application/json"));
            assert!(request.contains("{\"name\":\"Anna\"}"));
            let body = b"{\"id\":42,\"name\":\"Anna\"}";
            let response = format!(
                "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                String::from_utf8_lossy(body)
            );
            std::io::Write::write_all(&mut stream, response.as_bytes()).unwrap();
        });
        let source = r#"
            struct CustomerCreate { name: String }
            struct Customer { id: Int name: String }
            fn create(url: String, payload: CustomerCreate) -> Customer uses Network {
                return http_json<CustomerCreate, Customer>("POST", url, [], Some(payload))
            }
            fn main() { }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        check(&program).unwrap();
        check_capabilities(&program).unwrap();
        let grants = HashSet::from([String::from("Network")]);
        let policy = RuntimePolicy {
            filesystem: None,
            network: Some(NetworkPolicy {
                allowed_hosts: vec![format!("127.0.0.1:{port}")],
                timeout_ms: 1_000,
                max_response_bytes: 100,
            }),
            process: None,
        };
        let customer = execute_function_with_capabilities_and_policies(
            &program,
            "create",
            vec![
                Value::String(format!("http://127.0.0.1:{port}/customers")),
                Value::Object {
                    type_name: "CustomerCreate".into(),
                    fields: HashMap::from([(String::from("name"), Value::String("Anna".into()))]),
                },
            ],
            None,
            Some(&grants),
            Some(&policy),
        )
        .unwrap();
        let Value::Object { fields, .. } = customer else {
            panic!("expected decoded Customer object");
        };
        assert_eq!(fields.get("id"), Some(&Value::Int(42)));
        assert_eq!(fields.get("name"), Some(&Value::String("Anna".into())));
        server.join().unwrap();
    }

    #[test]
    fn returns_structured_http_result_for_http_errors() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 512];
            let _ = stream.read(&mut request).unwrap();
            std::io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 422 Unprocessable Entity\r\nX-Reason: invalid\r\nContent-Length: 21\r\nConnection: close\r\n\r\n{\"message\":\"invalid\"}",
            )
            .unwrap();
        });
        let source = r#"
            struct Customer { id: Int name: String }
            fn submit(url: String) -> HttpResult<Customer> uses Network {
                return http_result<Customer, Customer>("POST", url, [], None)
            }
            fn main() { }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        check(&program).unwrap();
        check_capabilities(&program).unwrap();
        let grants = HashSet::from([String::from("Network")]);
        let policy = RuntimePolicy {
            filesystem: None,
            network: Some(NetworkPolicy {
                allowed_hosts: vec![format!("127.0.0.1:{port}")],
                timeout_ms: 1_000,
                max_response_bytes: 100,
            }),
            process: None,
        };
        let result = execute_function_with_capabilities_and_policies(
            &program,
            "submit",
            vec![Value::String(format!("http://127.0.0.1:{port}/customers"))],
            None,
            Some(&grants),
            Some(&policy),
        )
        .unwrap();
        let Value::Object { fields, .. } = result else {
            panic!("expected HttpResult object");
        };
        assert_eq!(fields.get("status"), Some(&Value::Int(422)));
        assert_eq!(fields.get("data"), Some(&Value::Option(None)));
        let Value::Option(Some(error)) = fields.get("error").unwrap() else {
            panic!("expected structured HttpError");
        };
        let Value::Object { fields, .. } = &**error else {
            panic!("expected HttpError object");
        };
        assert_eq!(fields.get("status"), Some(&Value::Int(422)));
        assert_eq!(
            fields.get("body"),
            Some(&Value::String(r#"{"message":"invalid"}"#.into()))
        );
        server.join().unwrap();
    }

    #[test]
    fn rejects_http_request_body_for_get() {
        let source = "fn request() -> HttpResponse uses Network { return http_request(\"GET\", \"http://example.test\", [], Some(\"body\")) } fn main() { }";
        let program = parse(&lex(source).unwrap()).unwrap();
        check(&program).unwrap();
        check_capabilities(&program).unwrap();
        let grants = HashSet::from([String::from("Network")]);
        let policy = RuntimePolicy {
            filesystem: None,
            network: Some(NetworkPolicy {
                allowed_hosts: vec![String::from("example.test")],
                timeout_ms: 100,
                max_response_bytes: 100,
            }),
            process: None,
        };
        let error = execute_function_with_capabilities_and_policies(
            &program,
            "request",
            Vec::new(),
            None,
            Some(&grants),
            Some(&policy),
        )
        .unwrap_err();
        assert!(error.message.contains("does not allow a body with GET"));
    }

    #[test]
    fn decodes_and_encodes_typed_json_records() {
        let source = r#"
            struct Address { city: String }
            struct Customer {
                name: String
                address: Address
                tags: String[]
                nickname: String?
            }
            fn decode(body: String) -> Customer {
                return json_decode<Customer>(body)
            }
            fn encode(customer: Customer) -> String {
                return json_encode(customer)
            }
            fn main() { }
        "#;
        let program = parse(&lex(source).unwrap()).unwrap();
        check(&program).unwrap();
        let decoded = execute_function(
            &program,
            "decode",
            vec![Value::String(
                r#"{"name":"Anna","address":{"city":"Berlin"},"tags":["vip","de"]}"#.into(),
            )],
            None,
        )
        .unwrap();
        let Value::Object { fields, .. } = &decoded else {
            panic!("expected decoded Customer object");
        };
        assert_eq!(fields.get("name"), Some(&Value::String("Anna".into())));
        assert_eq!(fields.get("nickname"), Some(&Value::Option(None)));
        let Value::Object {
            fields: address, ..
        } = fields.get("address").unwrap()
        else {
            panic!("expected decoded Address object");
        };
        assert_eq!(address.get("city"), Some(&Value::String("Berlin".into())));

        let encoded = execute_function(&program, "encode", vec![decoded], None).unwrap();
        let Value::String(encoded) = encoded else {
            panic!("expected JSON string");
        };
        let json: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(json["name"], "Anna");
        assert_eq!(json["address"]["city"], "Berlin");
        assert_eq!(json["tags"][0], "vip");
        assert!(json["nickname"].is_null());
    }

    #[test]
    fn rejects_json_decode_unknown_record_field() {
        let source = "struct Customer { name: String } fn decode(body: String) -> Customer { return json_decode<Customer>(body) } fn main() { }";
        let program = parse(&lex(source).unwrap()).unwrap();
        check(&program).unwrap();
        let error = execute_function(
            &program,
            "decode",
            vec![Value::String(r#"{"name":"Anna","admin":true}"#.into())],
            None,
        )
        .unwrap_err();
        assert!(error.message.contains("unknown field `admin`"));
    }

    #[test]
    fn rejects_json_decode_without_target_type() {
        let source = "fn decode(body: String) -> String { return json_decode(body) } fn main() { }";
        let program = parse(&lex(source).unwrap()).unwrap();
        let errors = check(&program).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("exactly one type argument")));
    }

    #[test]
    fn requires_process_capability_for_run_process() {
        let program = parse(
            &lex("fn main() { print(run_process(\"/usr/bin/printf\", [\"hello\"])) }").unwrap(),
        )
        .unwrap();
        let errors = check_capabilities(&program).unwrap_err();
        assert!(errors.iter().any(|error| error
            .message
            .contains("does not declare capability `Process`")));
    }

    #[test]
    fn runs_allowlisted_process_without_a_shell() {
        let source =
            "fn run() -> String uses Process { return run_process(\"/usr/bin/printf\", [\"%s\", \"hello\"]) } fn main() { }";
        let program = parse(&lex(source).unwrap()).unwrap();
        check(&program).unwrap();
        check_capabilities(&program).unwrap();
        let grants = HashSet::from([String::from("Process")]);
        let policy = RuntimePolicy {
            filesystem: None,
            network: None,
            process: Some(ProcessPolicy {
                allowed_commands: vec![String::from("/usr/bin/printf")],
                timeout_ms: 1_000,
                max_output_bytes: 100,
            }),
        };
        let output = execute_function_with_capabilities_and_policies(
            &program,
            "run",
            Vec::new(),
            None,
            Some(&grants),
            Some(&policy),
        )
        .unwrap();
        assert_eq!(output, Value::String("hello".into()));
    }

    #[test]
    fn rejects_processes_outside_allowlist_and_output_limit() {
        let source =
            "fn run(command: String) -> String uses Process { return run_process(command, [\"%s\", \"hello\"]) } fn main() { }";
        let program = parse(&lex(source).unwrap()).unwrap();
        check(&program).unwrap();
        let grants = HashSet::from([String::from("Process")]);
        let error = execute_function_with_capabilities_and_policies(
            &program,
            "run",
            vec![Value::String(String::from("/usr/bin/printf"))],
            None,
            Some(&grants),
            None,
        )
        .unwrap_err();
        assert!(error.message.contains("no process policy"));

        let policy = RuntimePolicy {
            filesystem: None,
            network: None,
            process: Some(ProcessPolicy {
                allowed_commands: vec![String::from("/usr/bin/printf")],
                timeout_ms: 1_000,
                max_output_bytes: 3,
            }),
        };
        let error = execute_function_with_capabilities_and_policies(
            &program,
            "run",
            vec![Value::String(String::from("/usr/bin/echo"))],
            None,
            Some(&grants),
            Some(&policy),
        )
        .unwrap_err();
        assert!(error.message.contains("process execution denied"));

        let error = execute_function_with_capabilities_and_policies(
            &program,
            "run",
            vec![Value::String(String::from("/usr/bin/printf"))],
            None,
            Some(&grants),
            Some(&policy),
        )
        .unwrap_err();
        assert!(error.message.contains("output limit"));
    }

    #[test]
    fn accepts_clock_and_environment_host_apis() {
        let program = parse(
            &lex(
                "fn current_time() -> Timestamp uses Clock { return now() } fn read_env(name: String) -> String? uses Environment { return env(name) } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        check(&program).unwrap();
        check_capabilities(&program).unwrap();

        let grants = HashSet::from([String::from("Clock"), String::from("Environment")]);
        let timestamp = execute_function_with_capabilities(
            &program,
            "current_time",
            Vec::new(),
            None,
            Some(&grants),
        )
        .unwrap();
        assert!(matches!(timestamp, Value::Timestamp(value) if value > 0));

        let environment = execute_function_with_capabilities(
            &program,
            "read_env",
            vec![Value::String(String::from("PATH"))],
            None,
            Some(&grants),
        )
        .unwrap();
        assert!(matches!(
            environment,
            Value::Option(Some(value)) if matches!(*value, Value::String(_))
        ));
    }

    #[test]
    fn requires_clock_and_environment_capabilities_for_host_apis() {
        let program =
            parse(&lex("fn main() { print(now()) print(env(\"PATH\")) }").unwrap()).unwrap();
        let errors = check_capabilities(&program).unwrap_err();
        assert!(errors.iter().any(|error| {
            error.message.contains("does not declare capability") && error.message.contains("Clock")
        }));
        assert!(errors.iter().any(|error| {
            error.message.contains("does not declare capability")
                && error.message.contains("Environment")
        }));
    }

    #[test]
    fn enforces_runtime_host_api_capabilities() {
        let clock_program = parse(&lex("fn main() { print(now()) }").unwrap()).unwrap();
        let clock_error = execute(&clock_program).unwrap_err();
        assert!(clock_error.message.contains("clock access requires"));
        assert!(clock_error.message.contains("Clock"));

        let environment_program =
            parse(&lex("fn main() uses Environment { print(env(\"PATH\")) }").unwrap()).unwrap();
        let grants = HashSet::new();
        let environment_error =
            execute_with_capabilities(&environment_program, Some(&grants)).unwrap_err();
        assert!(environment_error.message.contains("function"));
        assert!(environment_error.message.contains("Environment"));
    }

    #[test]
    fn accepts_secure_random_int_with_random_capability() {
        let program = parse(
            &lex("fn roll() -> Int uses Random { return random_int(1, 6) } fn main() { }").unwrap(),
        )
        .unwrap();
        check(&program).unwrap();
        check_capabilities(&program).unwrap();

        let grants = HashSet::from([String::from("Random")]);
        let value =
            execute_function_with_capabilities(&program, "roll", Vec::new(), None, Some(&grants))
                .unwrap();
        assert!(matches!(value, Value::Int(value) if (1..=6).contains(&value)));
    }

    #[test]
    fn rejects_invalid_random_range() {
        let program =
            parse(&lex("fn main() uses Random { print(random_int(6, 1)) }").unwrap()).unwrap();
        let grants = HashSet::from([String::from("Random")]);
        let error = execute_with_capabilities(&program, Some(&grants)).unwrap_err();
        assert!(error.message.contains("minimum <= maximum"));
    }

    #[test]
    fn requires_random_capability_for_random_int() {
        let program = parse(&lex("fn main() { print(random_int(1, 6)) }").unwrap()).unwrap();
        let errors = check_capabilities(&program).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("does not declare capability")));
        assert!(errors.iter().any(|error| error.message.contains("Random")));

        let grants = HashSet::new();
        let error = execute_with_capabilities(&program, Some(&grants)).unwrap_err();
        assert!(error.message.contains("random access requires"));
    }

    #[test]
    fn reads_utf8_text_with_file_system_capability() {
        let program = parse(
            &lex("fn read(path: String) -> String uses FileSystem { return read_text(path) } fn main() { }")
                .unwrap(),
        )
        .unwrap();
        check(&program).unwrap();
        check_capabilities(&program).unwrap();

        let grants = HashSet::from([String::from("FileSystem")]);
        let path = format!("{}/../examples/fibonacci.zyl", env!("CARGO_MANIFEST_DIR"));
        let value = execute_function_with_capabilities(
            &program,
            "read",
            vec![Value::String(path)],
            None,
            Some(&grants),
        )
        .unwrap();
        assert!(matches!(value, Value::String(value) if value.contains("fibonacci")));
    }

    #[test]
    fn enforces_file_system_capability_and_read_errors() {
        let program =
            parse(&lex("fn main() { print(read_text(\"README.md\")) }").unwrap()).unwrap();
        let errors = check_capabilities(&program).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("FileSystem")));

        let grants = HashSet::new();
        let capability_error = execute_with_capabilities(&program, Some(&grants)).unwrap_err();
        assert!(capability_error
            .message
            .contains("file-system read access requires"));

        let missing = parse(
            &lex("fn main() uses FileSystem { print(read_text(\"/zelyra/path-that-does-not-exist\")) }")
                .unwrap(),
        )
        .unwrap();
        let grants = HashSet::from([String::from("FileSystem")]);
        let read_error = execute_with_capabilities(&missing, Some(&grants)).unwrap_err();
        assert!(read_error.message.contains("file-system read failed"));
    }

    #[test]
    fn writes_lists_and_deletes_with_a_file_system_policy() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-filesystem-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let program = parse(
            &lex(
                "fn write_file(path: String) uses FileSystem { write_text(path, \"hello\") } fn read_file(path: String) -> String uses FileSystem { return read_text(path) } fn list_entries(path: String) -> String[] uses FileSystem { return list_dir(path) } fn remove_file(path: String) uses FileSystem { delete_file(path) } fn main() { }",
            )
            .unwrap(),
        )
        .unwrap();
        check(&program).unwrap();
        check_capabilities(&program).unwrap();
        let policy = FileSystemPolicy {
            base_dir: root.clone(),
            read_roots: vec![root.clone()],
            write_roots: vec![root.clone()],
        };
        let grants = HashSet::from([String::from("FileSystem")]);
        let path = Value::String(String::from("note.txt"));
        execute_function_with_capabilities_and_filesystem_policy(
            &program,
            "write_file",
            vec![path.clone()],
            None,
            Some(&grants),
            Some(&policy),
        )
        .unwrap();
        let content = execute_function_with_capabilities_and_filesystem_policy(
            &program,
            "read_file",
            vec![path.clone()],
            None,
            Some(&grants),
            Some(&policy),
        )
        .unwrap();
        assert_eq!(content, Value::String(String::from("hello")));
        let entries = execute_function_with_capabilities_and_filesystem_policy(
            &program,
            "list_entries",
            vec![Value::String(String::from("."))],
            None,
            Some(&grants),
            Some(&policy),
        )
        .unwrap();
        assert_eq!(
            entries,
            Value::Array(vec![Value::String(String::from("note.txt"))])
        );
        execute_function_with_capabilities_and_filesystem_policy(
            &program,
            "remove_file",
            vec![path],
            None,
            Some(&grants),
            Some(&policy),
        )
        .unwrap();
        assert!(!root.join("note.txt").exists());
        std::fs::remove_dir(&root).unwrap();
    }

    #[test]
    fn rejects_file_system_writes_outside_configured_roots() {
        let program = parse(
            &lex("fn main() uses FileSystem { write_text(\"blocked.txt\", \"no\") }").unwrap(),
        )
        .unwrap();
        let root = std::env::current_dir().unwrap();
        let policy = FileSystemPolicy {
            base_dir: root.clone(),
            read_roots: vec![root.clone()],
            write_roots: Vec::new(),
        };
        let grants = HashSet::from([String::from("FileSystem")]);
        let error =
            execute_with_capabilities_and_filesystem_policy(&program, Some(&grants), Some(&policy))
                .unwrap_err();
        assert!(error.message.contains("outside the configured roots"));
    }

    #[test]
    fn enforces_preconditions_and_postconditions() {
        let source = "fn increment(value: Int) -> Int requires { value >= 0 } ensures { result > value } { return value + 1 } fn main() { print(increment(1)) }";
        let program = parse(&lex(source).unwrap()).unwrap();
        check(&program).unwrap();
        assert_eq!(execute(&program).unwrap(), ["2"]);

        let failing_precondition =
            parse(&lex("fn increment(value: Int) -> Int requires { value >= 0 } { return value + 1 } fn main() { increment(-1) }").unwrap()).unwrap();
        check(&failing_precondition).unwrap();
        let error = execute(&failing_precondition).unwrap_err();
        assert!(error.message.contains("precondition failed"));

        let failing_postcondition =
            parse(&lex("fn broken(value: Int) -> Int ensures { result > value } { return value } fn main() { broken(1) }").unwrap()).unwrap();
        check(&failing_postcondition).unwrap();
        let error = execute(&failing_postcondition).unwrap_err();
        assert!(error.message.contains("postcondition failed"));
    }

    #[test]
    fn requires_and_ensures_must_be_bool() {
        let program =
            parse(&lex("fn main() requires { 1 } ensures { \"done\" } { print(1) }").unwrap())
                .unwrap();
        let errors = check(&program).unwrap_err();
        assert_eq!(errors.len(), 2);
        assert!(errors
            .iter()
            .all(|error| error.message.contains("expected `Bool`")));
    }

    #[test]
    fn classifies_verification_results_without_overclaiming() {
        let program = parse(
            &lex("fn proven() requires { 1 < 2 } { } fn dynamic(value: Int) requires { value >= 0 } { } fn failed() ensures { 1 > 2 } { } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::Proven);
        assert_eq!(results[1].status, VerificationStatus::RuntimeCheck);
        assert_eq!(results[2].status, VerificationStatus::Failed);
        assert_eq!(
            results[2].message,
            "Constant contradiction: this condition evaluates to false for every input."
        );
        assert_eq!(results[3].status, VerificationStatus::Unproven);
    }

    #[test]
    fn reports_a_small_linear_counterexample_for_a_failed_postcondition() {
        let program = parse(
            &lex("fn bad(value: Int) -> Int requires { value >= 0 } ensures { result > value } { return value } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[1].status, VerificationStatus::Failed);
        assert_eq!(results[1].counterexample, Some(vec![("value".into(), 0)]));
    }

    #[test]
    fn reports_a_small_linear_counterexample_with_three_variables() {
        let program = parse(
            &lex("fn bad(a: Int, b: Int, c: Int) -> Int requires { a >= 0 && c <= b } ensures { result > b } { return c } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[1].status, VerificationStatus::Failed);
        assert_eq!(
            results[1].counterexample,
            Some(vec![("a".into(), 0), ("b".into(), 0), ("c".into(), 0)])
        );
    }

    #[test]
    fn proves_simple_integer_postconditions_from_direct_returns() {
        let program = parse(
            &lex("fn increment(value: Int) -> Int ensures { result > value } { return value + 1 } fn unchanged(value: Int) -> Int ensures { result > value } { return value } fn absolute(value: Int) -> Int ensures { result >= 0 } { if value >= 0 { return value } else { return -value } } fn classify(value: Int) -> Int ensures { result >= 0 } { match value { 0 => { return 0 } _ => { return 1 } } } fn bool_value(value: Bool) -> Int ensures { result >= 0 } { match value { true => { return 1 } false => { return 0 } } } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::Proven);
        assert_eq!(results[1].status, VerificationStatus::Failed);
        assert_eq!(results[2].status, VerificationStatus::Proven);
        assert_eq!(results[3].status, VerificationStatus::Proven);
        assert_eq!(results[4].status, VerificationStatus::Proven);
        assert_eq!(results[5].status, VerificationStatus::Unproven);
    }

    #[test]
    fn proves_immutable_local_bindings_in_contracts() {
        let program = parse(
            &lex("fn increment_local(value: Int) -> Int ensures { result > value } { next: Int = value + 1 return next } fn caller(value: Int) -> Int ensures { result > value } { next = value + 1 return next } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::Proven);
        assert_eq!(results[1].status, VerificationStatus::Proven);
        assert_eq!(results[2].status, VerificationStatus::Unproven);
    }

    #[test]
    fn proves_mutable_local_initialization() {
        let program = parse(
            &lex("fn increment_local(value: Int) -> Int ensures { result > value } { mutable next = value + 1 return next } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::Proven);
        assert_eq!(results[1].status, VerificationStatus::Unproven);
    }

    #[test]
    fn proves_linear_mutable_assignments() {
        let program = parse(
            &lex("fn increment_mutable(value: Int) -> Int ensures { result > value } { mutable next = value next = next + 1 return next } fn caller(value: Int) -> Int ensures { result > value } { mutable next = value next = next + 1 return next } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::Proven);
        assert_eq!(results[1].status, VerificationStatus::Proven);
        assert_eq!(results[2].status, VerificationStatus::Unproven);
    }

    #[test]
    fn keeps_non_linear_mutable_assignments_unproven() {
        let program = parse(
            &lex("fn multiply_mutable(value: Int) -> Int ensures { result > value } { mutable next = value next = next * value return next } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::RuntimeCheck);
        assert_eq!(results[1].status, VerificationStatus::Unproven);
    }

    #[test]
    fn proves_bounded_linear_while_loops() {
        let program = parse(
            &lex("fn add_three(value: Int) -> Int ensures { result >= value + 3 } { mutable total = value mutable count = 0 while count < 3 { total = total + 1 count = count + 1 } return total } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::Proven);
        assert_eq!(results[1].status, VerificationStatus::Unproven);
    }

    #[test]
    fn preserves_mutable_condition_snapshots() {
        let program = parse(
            &lex("fn avoid_zero(value: Int) -> Int ensures { result != 0 } { mutable current = value if current >= 0 { current = current + 1 } else { current = current - 1 } return current } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::Proven);
        assert_eq!(results[1].status, VerificationStatus::Unproven);
    }

    #[test]
    fn keeps_unbounded_while_loops_unproven() {
        let program = parse(
            &lex("fn reduce(value: Int) -> Int ensures { result == 0 } { mutable current = value while current > 0 { current = current - 1 } return current } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::RuntimeCheck);
        assert_eq!(results[1].status, VerificationStatus::Unproven);
    }

    #[test]
    fn proves_unbounded_linear_loops_with_invariants() {
        let program = parse(
            &lex("fn reduce(value: Int) -> Int requires { value >= 0 } ensures { result == 0 } { mutable current = value while current > 0 invariant { current >= 0 } { current = current - 1 } return current } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::RuntimeCheck);
        assert_eq!(results[1].status, VerificationStatus::Proven);
        assert_eq!(results[2].kind, ContractKind::LoopInvariant);
        assert_eq!(results[2].status, VerificationStatus::Proven);
        assert_eq!(results[3].status, VerificationStatus::Unproven);
    }

    #[test]
    fn proves_invariant_controlled_unconditional_loops() {
        let program = parse(
            &lex("fn stop_after_one() -> Int ensures { result == 1 } { mutable i = 0 loop invariant { i >= 0 } { i = i + 1 break } return i } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::Proven);
        assert_eq!(results[1].kind, ContractKind::LoopInvariant);
        assert_eq!(results[1].status, VerificationStatus::Proven);
        assert_eq!(results[2].status, VerificationStatus::Unproven);
    }

    #[test]
    fn does_not_prove_a_non_preserved_loop_invariant() {
        let program = parse(
            &lex("fn reduce(value: Int) -> Int requires { value > 0 } ensures { result == 0 } { mutable current = value while current > 0 invariant { current == value } { current = current - 1 } return current } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::RuntimeCheck);
        assert_eq!(results[1].status, VerificationStatus::RuntimeCheck);
        assert_eq!(results[2].kind, ContractKind::LoopInvariant);
        assert_eq!(results[2].status, VerificationStatus::Failed);
        assert!(results[2].message.contains("not preserved"));
        assert!(results[2].counterexample.is_some());
        assert_eq!(results[3].status, VerificationStatus::Unproven);
    }

    #[test]
    fn reports_each_loop_invariant_status() {
        let program = parse(
            &lex("fn reduce(value: Int) -> Int requires { value >= 0 } ensures { result == 0 } { mutable current = value while current > 0 invariant { current >= 0 } invariant { current == value } { current = current - 1 } return current } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[2].kind, ContractKind::LoopInvariant);
        assert_eq!(results[2].index, 0);
        assert_eq!(results[2].status, VerificationStatus::Proven);
        assert_eq!(results[3].kind, ContractKind::LoopInvariant);
        assert_eq!(results[3].index, 1);
        assert_eq!(results[3].status, VerificationStatus::Failed);
    }

    #[test]
    fn proves_break_and_continue_paths_in_bounded_loops() {
        let program = parse(
            &lex("fn stop_after_one() -> Int ensures { result == 1 } { mutable i = 0 while i < 5 { i = i + 1 break } return i } fn skip_one() -> Int ensures { result == 2 } { mutable i = 0 mutable total = 0 while i < 3 { i = i + 1 if i == 2 { continue } total = total + 1 } return total } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::Proven);
        assert_eq!(results[1].status, VerificationStatus::Proven);
        assert_eq!(results[2].status, VerificationStatus::Unproven);
    }

    #[test]
    fn collects_option_and_result_constructor_paths() {
        let program = parse(
            &lex("fn option_flag(value: Int?) -> Int ensures { result >= 0 } { match value { Some(number) => { return 1 } None => { return 0 } } } fn result_flag(value: Result<Int, String>) -> Int ensures { result >= 0 } { match value { Ok(number) => { return 1 } Err(message) => { return 0 } } } fn known_option() -> Int ensures { result == 4 } { match Some(4) { Some(number) => { return number } None => { return 0 } } } fn known_result() -> Int ensures { result == 7 } { match Ok(7) { Ok(number) => { return number } Err(message) => { return 0 } } } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::Proven);
        assert_eq!(results[1].status, VerificationStatus::Proven);
        assert_eq!(results[2].status, VerificationStatus::Proven);
        assert_eq!(results[3].status, VerificationStatus::Proven);
        assert_eq!(results[4].status, VerificationStatus::Unproven);
    }

    #[test]
    fn proves_direct_function_call_summaries() {
        let program = parse(
            &lex("fn increment(value: Int) -> Int { return value + 1 } fn twice(value: Int) -> Int ensures { result > value } { return increment(increment(value)) } fn known() -> Int ensures { result == 5 } { return increment(4) } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::Unproven);
        assert_eq!(results[1].status, VerificationStatus::Proven);
        assert_eq!(results[2].status, VerificationStatus::Proven);
        assert_eq!(results[3].status, VerificationStatus::Unproven);
    }

    #[test]
    fn proves_path_sensitive_function_call_summaries() {
        let program = parse(
            &lex("fn absolute(value: Int) -> Int ensures { result >= 0 } { if value >= 0 { return value } else { return -value } } fn caller(value: Int) -> Int ensures { result >= 0 } { return absolute(value) } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::Proven);
        assert_eq!(results[1].status, VerificationStatus::Proven);
        assert_eq!(results[2].status, VerificationStatus::Unproven);
    }

    #[test]
    fn rejects_impossible_postcondition_for_path_sensitive_call() {
        let program = parse(
            &lex("fn absolute(value: Int) -> Int ensures { result >= 0 } { if value >= 0 { return value } else { return -value } } fn caller(value: Int) -> Int ensures { result < 0 } { return absolute(value) } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::Proven);
        assert_eq!(results[1].status, VerificationStatus::Failed);
        assert_eq!(results[2].status, VerificationStatus::Unproven);
    }

    #[test]
    fn uses_function_preconditions_as_postcondition_assumptions() {
        let program = parse(
            &lex("fn non_negative(value: Int) -> Int requires { value >= 0 } ensures { result >= 0 } { return value } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::RuntimeCheck);
        assert_eq!(results[1].status, VerificationStatus::Proven);
        assert_eq!(results[2].status, VerificationStatus::Unproven);
    }

    #[test]
    fn checks_called_function_preconditions_under_caller_assumptions() {
        let program = parse(
            &lex("fn non_negative(value: Int) -> Int requires { value >= 0 } { return value } fn safe_call(value: Int) -> Int requires { value >= 0 } ensures { result >= 0 } { return non_negative(value) } fn unchecked_call(value: Int) -> Int ensures { result >= 0 } { return non_negative(value) } fn main() { }").unwrap(),
        )
        .unwrap();
        let results = verify(&program);
        assert_eq!(results[0].status, VerificationStatus::RuntimeCheck);
        assert_eq!(results[1].status, VerificationStatus::RuntimeCheck);
        assert_eq!(results[2].status, VerificationStatus::Proven);
        assert_eq!(results[3].status, VerificationStatus::RuntimeCheck);
        assert_eq!(results[4].status, VerificationStatus::Unproven);
    }

    #[test]
    fn validates_api_types_and_path_parameters() {
        let program = parse(
            &lex("type CustomerId = Id table customers { id: Id primary auto } api GET \"/customers/{id}\" { input { id: CustomerId } output Customer errors { 404 NotFound } } fn main() { }").unwrap(),
        )
        .unwrap();
        assert!(check_apis(&program).is_ok());
    }

    #[test]
    fn validates_api_handler_signature() {
        let program = parse(
            &lex("type CustomerId = Id table customers { id: Id primary auto } fn get_customer(id: CustomerId) -> Customer { } api GET \"/customers/{id}\" { handler get_customer input { id: CustomerId } output Customer } fn main() { }").unwrap(),
        )
        .unwrap();
        assert!(check_apis(&program).is_ok());
    }

    #[test]
    fn rejects_api_paths_without_declared_inputs() {
        let program =
            parse(&lex("api GET \"/customers/{id}\" { output String } fn main() { }").unwrap())
                .unwrap();
        let errors = check_apis(&program).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("path parameter `id`")));
    }
}
