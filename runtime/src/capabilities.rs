//! Shared static capability analysis and effect mappings.
//!
//! The AST runtime, MIR verifier, and CLI consume these same rules so effect
//! diagnostics and declared capability ceilings cannot drift by backend.
use super::*;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityError {
    pub message: String,
    pub span: Span,
}

/// A capability required by a concrete expression or resolved function call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityUse {
    pub function: String,
    pub capability: String,
    pub operation: String,
    pub span: Span,
    /// Literal paths can be checked against the runtime policy; None is dynamic.
    pub file_paths: Vec<FilePathUse>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePathUse {
    pub path: Option<String>,
    pub write: bool,
    pub allow_missing: bool,
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub fn check_capabilities(program: &Program) -> Result<(), Vec<CapabilityError>> {
    check_capabilities_with_grants(program, None)
}

pub fn check_capabilities_with_grants(
    program: &Program,
    grants: Option<&HashSet<String>>,
) -> Result<(), Vec<CapabilityError>> {
    let (errors, _) = analyze_capabilities(program, grants);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Return use sites from the same traversal that enforces capability checks.
/// Dynamic file paths and indirect calls still require runtime inspection.
pub fn inspect_capability_uses(program: &Program) -> Vec<CapabilityUse> {
    analyze_capabilities(program, None).1
}

fn analyze_capabilities(
    program: &Program,
    grants: Option<&HashSet<String>>,
) -> (Vec<CapabilityError>, Vec<CapabilityUse>) {
    let (validation_functions, _) = zelyra_ast::build_table_validation_functions(program);
    let all_functions = program
        .functions
        .iter()
        .chain(validation_functions.iter())
        .collect::<Vec<_>>();
    let functions = all_functions
        .iter()
        .copied()
        .map(|function| (function.name.as_str(), function))
        .collect::<HashMap<_, _>>();
    let known = KNOWN_CAPABILITIES.iter().copied().collect::<HashSet<_>>();
    let mut errors = Vec::new();
    let mut uses = Vec::new();

    for function in all_functions {
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
            } else if grants.is_some_and(|grants| !project_grants_capability(grants, capability)) {
                errors.push(CapabilityError {
                    message: format!(
                        "capability `{capability}` is not enabled by the project; set `{}` to true in zelyra.toml",
                        capability_config_key(capability)
                    ),
                    span: function.span,
                });
            }
        }
        check_capability_block(
            &function.body,
            function,
            &functions,
            &declared,
            &mut errors,
            &mut uses,
        );
        let mut function_values = HashMap::new();
        for parameter in &function.params {
            if let Type::Function { capabilities, .. } = &parameter.ty {
                function_values.insert(
                    parameter.name.clone(),
                    capabilities.iter().cloned().collect::<HashSet<_>>(),
                );
            }
        }
        check_indirect_capability_block(
            &function.body,
            function,
            &functions,
            &declared,
            &mut function_values,
            &mut errors,
            &mut uses,
        );
        for contract in function.requires.iter().chain(&function.ensures) {
            check_capability_expr(
                contract,
                function,
                &functions,
                &declared,
                &mut errors,
                &mut uses,
            );
        }
    }
    (errors, uses)
}

fn function_value_capabilities(
    expression: &Expr,
    values: &HashMap<String, HashSet<String>>,
    functions: &HashMap<&str, &Function>,
) -> Option<HashSet<String>> {
    match &expression.kind {
        ExprKind::FunctionValue(name) => functions
            .get(name.as_str())
            .map(|function| function.capabilities.iter().cloned().collect()),
        ExprKind::Variable(name) => values.get(name).cloned(),
        ExprKind::Call { name, .. } => functions
            .get(name.as_str())
            .and_then(|function| function.return_type.as_ref())
            .and_then(|ty| match ty {
                Type::Function { capabilities, .. } => Some(capabilities.iter().cloned().collect()),
                _ => None,
            }),
        ExprKind::Match { arms, .. } => {
            let mut found = false;
            let mut capabilities = HashSet::new();
            for arm in arms {
                if let Some(arm_capabilities) =
                    function_value_capabilities(&arm.value, values, functions)
                {
                    found = true;
                    capabilities.extend(arm_capabilities);
                }
            }
            found.then_some(capabilities)
        }
        _ => None,
    }
}

fn check_indirect_capability_block(
    block: &Block,
    function: &Function,
    functions: &HashMap<&str, &Function>,
    declared: &HashSet<&str>,
    values: &mut HashMap<String, HashSet<String>>,
    errors: &mut Vec<CapabilityError>,
    uses: &mut Vec<CapabilityUse>,
) {
    for statement in &block.statements {
        match statement {
            Stmt::Let {
                name, ty, value, ..
            } => {
                check_indirect_capability_expr(value, function, declared, values, errors, uses);
                let declared_effects = ty.as_ref().and_then(|ty| match ty {
                    Type::Function { capabilities, .. } => {
                        Some(capabilities.iter().cloned().collect())
                    }
                    _ => None,
                });
                if let Some(capabilities) = declared_effects
                    .or_else(|| function_value_capabilities(value, values, functions))
                {
                    values.insert(name.clone(), capabilities);
                }
            }
            Stmt::BindOrAssign { name, value, .. } => {
                check_indirect_capability_expr(value, function, declared, values, errors, uses);
                if let Some(capabilities) = function_value_capabilities(value, values, functions) {
                    values.entry(name.clone()).or_default().extend(capabilities);
                } else {
                    values.remove(name);
                }
            }
            Stmt::TupleBindOrAssign { names, value, .. } => {
                check_indirect_capability_expr(value, function, declared, values, errors, uses);
                for name in names {
                    values.remove(name);
                }
            }
            Stmt::ArrayIndexAssign { index, value, .. } => {
                check_indirect_capability_expr(index, function, declared, values, errors, uses);
                check_indirect_capability_expr(value, function, declared, values, errors, uses);
            }
            Stmt::Expr(expression)
            | Stmt::Return {
                value: Some(expression),
                ..
            } => {
                check_indirect_capability_expr(
                    expression, function, declared, values, errors, uses,
                );
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                check_indirect_capability_expr(condition, function, declared, values, errors, uses);
                let before = values.clone();
                let mut then_values = before.clone();
                check_indirect_capability_block(
                    then_block,
                    function,
                    functions,
                    declared,
                    &mut then_values,
                    errors,
                    uses,
                );
                let mut else_values = before.clone();
                if let Some(else_block) = else_block {
                    check_indirect_capability_block(
                        else_block,
                        function,
                        functions,
                        declared,
                        &mut else_values,
                        errors,
                        uses,
                    );
                }
                *values = merge_function_value_capabilities(&before, &[then_values, else_values]);
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                check_indirect_capability_expr(condition, function, declared, values, errors, uses);
                for invariant in invariants {
                    check_indirect_capability_expr(
                        invariant, function, declared, values, errors, uses,
                    );
                }
                let before = values.clone();
                let mut body_values = before.clone();
                check_indirect_capability_block(
                    body,
                    function,
                    functions,
                    declared,
                    &mut body_values,
                    errors,
                    uses,
                );
                *values = merge_function_value_capabilities(&before, &[body_values]);
            }
            Stmt::For { iterable, body, .. } => {
                check_indirect_capability_expr(iterable, function, declared, values, errors, uses);
                let before = values.clone();
                let mut body_values = before.clone();
                check_indirect_capability_block(
                    body,
                    function,
                    functions,
                    declared,
                    &mut body_values,
                    errors,
                    uses,
                );
                *values = merge_function_value_capabilities(&before, &[body_values]);
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    check_indirect_capability_expr(
                        invariant, function, declared, values, errors, uses,
                    );
                }
                let before = values.clone();
                let mut body_values = before.clone();
                check_indirect_capability_block(
                    body,
                    function,
                    functions,
                    declared,
                    &mut body_values,
                    errors,
                    uses,
                );
                *values = merge_function_value_capabilities(&before, &[body_values]);
            }
            Stmt::Match { value, arms, .. } => {
                check_indirect_capability_expr(value, function, declared, values, errors, uses);
                let before = values.clone();
                let mut branches = Vec::new();
                for arm in arms {
                    let mut branch = before.clone();
                    check_indirect_capability_block(
                        &arm.body,
                        function,
                        functions,
                        declared,
                        &mut branch,
                        errors,
                        uses,
                    );
                    branches.push(branch);
                }
                *values = merge_function_value_capabilities(&before, &branches);
            }
            Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
                let before = values.clone();
                let mut nested = before.clone();
                check_indirect_capability_block(
                    body,
                    function,
                    functions,
                    declared,
                    &mut nested,
                    errors,
                    uses,
                );
                *values = merge_function_value_capabilities(&before, &[nested]);
            }
            Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }
}

fn merge_function_value_capabilities(
    before: &HashMap<String, HashSet<String>>,
    branches: &[HashMap<String, HashSet<String>>],
) -> HashMap<String, HashSet<String>> {
    let mut merged = before.clone();
    for name in before.keys() {
        for branch in branches {
            if let Some(capabilities) = branch.get(name) {
                merged
                    .entry(name.clone())
                    .or_default()
                    .extend(capabilities.iter().cloned());
            }
        }
    }
    merged
}

fn check_indirect_capability_expr(
    expression: &Expr,
    function: &Function,
    declared: &HashSet<&str>,
    values: &HashMap<String, HashSet<String>>,
    errors: &mut Vec<CapabilityError>,
    uses: &mut Vec<CapabilityUse>,
) {
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match &expression.kind {
            ExprKind::Call { name, args, .. } => {
                if let Some(capabilities) = values.get(name) {
                    for capability in capabilities {
                        require_capability(
                            capability,
                            &format!("indirect call through `{name}`"),
                            expression.span,
                            function,
                            declared,
                            errors,
                            uses,
                        );
                    }
                }
                pending.extend(args);
            }
            ExprKind::Unary { expr, .. } | ExprKind::Await(expr) => pending.push(expr),
            ExprKind::Binary { left, right, .. } => {
                pending.push(left);
                pending.push(right);
            }
            ExprKind::Array(values) => pending.extend(values),
            ExprKind::Map(entries) => {
                for (key, value) in entries {
                    pending.push(key);
                    pending.push(value);
                }
            }
            ExprKind::Record { fields, .. } => {
                pending.extend(fields.iter().map(|(_, value)| value));
            }
            ExprKind::Index { target, index } => {
                pending.push(target);
                pending.push(index);
            }
            ExprKind::Field { target, .. } => pending.push(target),
            ExprKind::Lambda { body, .. } => pending.push(body),
            ExprKind::Match { value, arms } => {
                pending.push(value);
                pending.extend(arms.iter().map(|arm| &arm.value));
            }
            ExprKind::Sql { .. }
            | ExprKind::Int(_)
            | ExprKind::UInt(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::String(_)
            | ExprKind::Char(_)
            | ExprKind::FunctionValue(_)
            | ExprKind::Variable(_) => {}
        }
    }
}

fn check_capability_block(
    block: &Block,
    function: &Function,
    functions: &HashMap<&str, &Function>,
    declared: &HashSet<&str>,
    errors: &mut Vec<CapabilityError>,
    uses: &mut Vec<CapabilityUse>,
) {
    for statement in &block.statements {
        match statement {
            Stmt::Let { value, .. }
            | Stmt::BindOrAssign { value, .. }
            | Stmt::TupleBindOrAssign { value, .. }
            | Stmt::Expr(value)
            | Stmt::Return {
                value: Some(value), ..
            }
            | Stmt::Match { value, .. } => {
                check_capability_expr(value, function, functions, declared, errors, uses);
                if let Stmt::Match { arms, .. } = statement {
                    for arm in arms {
                        check_capability_block(
                            &arm.body, function, functions, declared, errors, uses,
                        );
                    }
                }
            }
            Stmt::ArrayIndexAssign { index, value, .. } => {
                check_capability_expr(index, function, functions, declared, errors, uses);
                check_capability_expr(value, function, functions, declared, errors, uses);
            }
            Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                check_capability_expr(condition, function, functions, declared, errors, uses);
                check_capability_block(then_block, function, functions, declared, errors, uses);
                if let Some(else_block) = else_block {
                    check_capability_block(else_block, function, functions, declared, errors, uses);
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                check_capability_expr(condition, function, functions, declared, errors, uses);
                for invariant in invariants {
                    check_capability_expr(invariant, function, functions, declared, errors, uses);
                }
                check_capability_block(body, function, functions, declared, errors, uses);
            }
            Stmt::For { iterable, body, .. } => {
                check_capability_expr(iterable, function, functions, declared, errors, uses);
                check_capability_block(body, function, functions, declared, errors, uses);
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    check_capability_expr(invariant, function, functions, declared, errors, uses);
                }
                check_capability_block(body, function, functions, declared, errors, uses);
            }
            Stmt::Transaction { body, .. } => {
                check_capability_block(body, function, functions, declared, errors, uses);
            }
            Stmt::Parallel { body, .. } => {
                check_capability_block(body, function, functions, declared, errors, uses);
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
    uses: &mut Vec<CapabilityUse>,
) {
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match &expression.kind {
            ExprKind::Sql { query, .. } => {
                let capability = sql_required_capability(query);
                require_capability(
                    capability,
                    "SQL access",
                    expression.span,
                    function,
                    declared,
                    errors,
                    uses,
                );
            }
            ExprKind::Call { name, args, .. } => {
                for (capability, operation) in builtin_capability_requirements(name) {
                    require_capability(
                        capability,
                        operation,
                        expression.span,
                        function,
                        declared,
                        errors,
                        uses,
                    );
                }
                if let Some(usage) = uses.last_mut() {
                    if usage.span == expression.span && usage.capability == "FileSystem" {
                        usage.file_paths = file_path_uses(name, args);
                    }
                }
                if let Some(callee) = functions.get(name.as_str()) {
                    for capability in &callee.capabilities {
                        if KNOWN_CAPABILITIES.contains(&capability.as_str()) {
                            uses.push(CapabilityUse {
                                function: function.name.clone(),
                                capability: capability.clone(),
                                operation: format!("call to `{name}`"),
                                span: expression.span,
                                file_paths: Vec::new(),
                            });
                        }
                        if KNOWN_CAPABILITIES.contains(&capability.as_str())
                            && !function_declares_capability(declared, capability)
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
                    pending.push(argument);
                }
            }
            ExprKind::Unary { expr, .. } => {
                pending.push(expr);
            }
            ExprKind::Binary { left, right, .. } => {
                pending.push(left);
                pending.push(right);
            }
            ExprKind::Array(values) => {
                for value in values {
                    pending.push(value);
                }
            }
            ExprKind::Map(entries) => {
                for (key, value) in entries {
                    pending.push(key);
                    pending.push(value);
                }
            }
            ExprKind::Record { fields, .. } => {
                for (_, value) in fields {
                    pending.push(value);
                }
            }
            ExprKind::Index { target, index } => {
                pending.push(target);
                pending.push(index);
            }
            ExprKind::Field { target, .. } => {
                pending.push(target);
            }
            ExprKind::Lambda { body, .. } => pending.push(body),
            ExprKind::Await(inner) => {
                pending.push(inner);
            }
            ExprKind::Match { value, arms } => {
                pending.push(value);
                for arm in arms {
                    pending.push(&arm.value);
                }
            }
            ExprKind::Int(..)
            | ExprKind::UInt(..)
            | ExprKind::Float(..)
            | ExprKind::Bool(..)
            | ExprKind::String(..)
            | ExprKind::Char(..)
            | ExprKind::Variable(..)
            | ExprKind::FunctionValue(..) => {}
        }
    }
}

/// Return the statically required capabilities and their diagnostic operation
/// labels for a built-in call. Source checking, MIR verification, and other
/// backends use this shared mapping so security effects do not drift by backend.
pub fn builtin_capability_requirements(name: &str) -> &'static [(&'static str, &'static str)] {
    match name {
        "read_console" => &[("Console", "console input")],
        "now" => &[("Clock", "clock access")],
        "env" => &[("Environment", "environment access")],
        "random_int" | "uuid_v4" => &[("Random", "random access")],
        "http_get" | "http_request" | "http_json" | "http_result" => {
            &[("Network", "network access")]
        }
        "mail_queue" => &[
            ("Email", "email delivery"),
            ("Database(write)", "transactional mail queue write"),
        ],
        "mail_deliver" => &[("Email", "email delivery")],
        "run_process" => &[("Process", "process execution")],
        "read_text" | "read_text_result" | "file_exists" | "dir_exists" | "file_size"
        | "list_dir" => &[("FileSystem", "file-system read access")],
        "write_text"
        | "write_text_result"
        | "write_text_new_result"
        | "atomic_write_text_result"
        | "append_text_result"
        | "copy_file_new_result"
        | "rename_file_result" => &[("FileSystem", "file-system write access")],
        "delete_file" | "delete_file_result" => &[("FileSystem", "file-system delete access")],
        _ => &[],
    }
}

fn require_capability(
    capability: &str,
    operation: &str,
    span: Span,
    function: &Function,
    declared: &HashSet<&str>,
    errors: &mut Vec<CapabilityError>,
    uses: &mut Vec<CapabilityUse>,
) {
    uses.push(CapabilityUse {
        function: function.name.clone(),
        capability: capability.into(),
        operation: operation.into(),
        span,
        file_paths: Vec::new(),
    });
    if !function_declares_capability(declared, capability) {
        errors.push(CapabilityError {
            message: format!(
                "function `{}` uses {operation} but does not declare capability `{capability}`; add `uses {capability}`",
                function.name
            ),
            span,
        });
    }
}

fn file_path_uses(name: &str, args: &[Expr]) -> Vec<FilePathUse> {
    let (modes, allow_missing): (&[bool], bool) = match name {
        "read_text" | "read_text_result" | "file_size" | "list_dir" => (&[false], false),
        "file_exists" | "dir_exists" => (&[false], true),
        "write_text"
        | "write_text_result"
        | "write_text_new_result"
        | "atomic_write_text_result"
        | "append_text_result" => (&[true], true),
        "delete_file" | "delete_file_result" => (&[true], false),
        "copy_file_new_result" => (&[false, true], false),
        "rename_file_result" => (&[true, true], false),
        _ => return Vec::new(),
    };
    modes
        .iter()
        .enumerate()
        .map(|(index, write)| FilePathUse {
            path: args.get(index).and_then(|arg| match &arg.kind {
                ExprKind::String(path) => Some(path.clone()),
                _ => None,
            }),
            write: *write,
            allow_missing: allow_missing || (index == 1 && modes.len() == 2),
        })
        .collect()
}

pub(super) fn function_declares_capability(declared: &HashSet<&str>, capability: &str) -> bool {
    declared.contains(capability)
        || (capability.starts_with("Database(") && declared.contains("Database"))
}

pub fn project_grants_capability(grants: &HashSet<String>, capability: &str) -> bool {
    if grants.contains(capability)
        || (capability.starts_with("Database(") && grants.contains("Database"))
    {
        return true;
    }
    capability == "Database"
        && grants.contains("Database(read)")
        && grants.contains("Database(write)")
}

fn capability_config_key(capability: &str) -> String {
    match capability {
        "Database(read)" => "database_read".into(),
        "Database(write)" => "database_write".into(),
        _ => capability.to_ascii_lowercase(),
    }
}

pub fn sql_required_capability(query: &str) -> &'static str {
    let command = zelyra_database::sql::analyze_table_access(query, &[]).0;
    match command {
        Some("SELECT") => {
            let upper = query.to_ascii_uppercase();
            if upper.contains("FOR UPDATE")
                || upper.contains("INTO OUTFILE")
                || upper.contains("INTO DUMPFILE")
            {
                "Database(write)"
            } else {
                "Database(read)"
            }
        }
        Some("INSERT" | "UPDATE" | "DELETE") => "Database(write)",
        _ => "Database",
    }
}
