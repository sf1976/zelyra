use std::collections::{HashMap, HashSet};
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

#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityError {
    pub message: String,
    pub span: Span,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractKind {
    Requires,
    Ensures,
}

impl fmt::Display for ContractKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Requires => write!(f, "requires"),
            Self::Ensures => write!(f, "ensures"),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationResult {
    pub function: String,
    pub kind: ContractKind,
    pub index: usize,
    pub status: VerificationStatus,
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
            results.push(VerificationResult {
                function: function.name.clone(),
                kind: ContractKind::Requires,
                index,
                status: verify_contract(contract, None, &functions),
            });
        }
        let return_paths = symbolic_return_paths(function);
        for (index, contract) in function.ensures.iter().enumerate() {
            results.push(VerificationResult {
                function: function.name.clone(),
                kind: ContractKind::Ensures,
                index,
                status: verify_postcondition(
                    contract,
                    &function.requires,
                    return_paths.as_deref(),
                    &functions,
                ),
            });
        }
        if function.requires.is_empty() && function.ensures.is_empty() {
            results.push(VerificationResult {
                function: function.name.clone(),
                kind: ContractKind::Requires,
                index: 0,
                status: VerificationStatus::Unproven,
            });
        }
    }
    results
}

fn verify_contract(
    contract: &Expr,
    return_expression: Option<&Expr>,
    functions: &HashMap<String, &Function>,
) -> VerificationStatus {
    match constant_value(contract) {
        Some(ConstantValue::Bool(true)) => VerificationStatus::Proven,
        Some(ConstantValue::Bool(false)) => VerificationStatus::Failed,
        Some(_) => VerificationStatus::Unproven,
        None => symbolic_bool(contract, return_expression, None, None, Some(functions), 0).map_or(
            VerificationStatus::RuntimeCheck,
            |value| {
                if value {
                    VerificationStatus::Proven
                } else {
                    VerificationStatus::Failed
                }
            },
        ),
    }
}

fn verify_postcondition(
    contract: &Expr,
    preconditions: &[Expr],
    return_paths: Option<&[ReturnPath<'_>]>,
    functions: &HashMap<String, &Function>,
) -> VerificationStatus {
    match constant_value(contract) {
        Some(ConstantValue::Bool(true)) => VerificationStatus::Proven,
        Some(ConstantValue::Bool(false)) => VerificationStatus::Failed,
        Some(_) => VerificationStatus::Unproven,
        None => {
            let Some(return_paths) = return_paths else {
                return VerificationStatus::RuntimeCheck;
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
                        None,
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
                        None,
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
                        substitutions: None,
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
                        Some(functions),
                        0,
                    ) {
                        Some(true) => {}
                        Some(false) => return VerificationStatus::Failed,
                        None => saw_unknown_path = true,
                    }
                }
            }
            if saw_feasible_path && !saw_unknown_path {
                VerificationStatus::Proven
            } else {
                VerificationStatus::RuntimeCheck
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum SymbolicGuard<'a> {
    Condition {
        expression: &'a Expr,
        expected: bool,
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
}

#[derive(Clone, Debug)]
enum SymbolicState<'a> {
    Continue {
        guards: Vec<SymbolicGuard<'a>>,
        bindings: HashMap<String, &'a Expr>,
    },
    Return {
        guards: Vec<SymbolicGuard<'a>>,
        expression: Option<&'a Expr>,
        bindings: HashMap<String, &'a Expr>,
    },
}

fn symbolic_return_paths(function: &Function) -> Option<Vec<ReturnPath<'_>>> {
    let states = symbolic_states(&function.body, Vec::new(), HashMap::new())?;
    let mut paths = Vec::new();
    for state in states {
        let SymbolicState::Return {
            guards,
            expression,
            bindings,
        } = state
        else {
            return None;
        };
        paths.push(ReturnPath {
            guards,
            expression,
            bindings,
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
) -> Option<Vec<SymbolicState<'a>>> {
    let mut states = vec![SymbolicState::Continue {
        guards: incoming,
        bindings,
    }];
    for statement in &block.statements {
        let mut next = Vec::new();
        for state in states {
            match state {
                SymbolicState::Return { .. } => next.push(state),
                SymbolicState::Continue { guards, bindings } => match statement {
                    Stmt::Return { value, .. } => next.push(SymbolicState::Return {
                        guards,
                        expression: value.as_ref(),
                        bindings,
                    }),
                    Stmt::If {
                        condition,
                        then_block,
                        else_block,
                        ..
                    } => {
                        let mut then_guards = guards.clone();
                        then_guards.push(SymbolicGuard::Condition {
                            expression: condition,
                            expected: true,
                        });
                        next.extend(symbolic_states(then_block, then_guards, bindings.clone())?);

                        if let Some(else_block) = else_block {
                            let mut else_guards = guards;
                            else_guards.push(SymbolicGuard::Condition {
                                expression: condition,
                                expected: false,
                            });
                            next.extend(symbolic_states(
                                else_block,
                                else_guards,
                                bindings.clone(),
                            )?);
                        } else {
                            let mut guards = guards;
                            guards.push(SymbolicGuard::Condition {
                                expression: condition,
                                expected: false,
                            });
                            next.push(SymbolicState::Continue { guards, bindings });
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
                            next.extend(symbolic_states(&arm.body, arm_guards, arm_bindings)?);
                        }
                    }
                    _ => return None,
                },
            }
        }
        states = next;
    }
    Some(states)
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
            let left = LinearConstraint::from_value(linear_value(
                value,
                context.return_expression,
                context.bindings,
                context.substitutions,
                context.functions,
                context.depth,
            )?);
            let right = LinearConstraint {
                coefficients: HashMap::new(),
                constant: i128::from(*expected),
            };
            comparison_constraints(left, BinaryOp::Equal, right, matched)
        }
        PatternKind::Bool(expected) => {
            let left = LinearConstraint::from_value(linear_value(
                value,
                context.return_expression,
                context.bindings,
                context.substitutions,
                context.functions,
                context.depth,
            )?);
            let right = LinearConstraint {
                coefficients: HashMap::new(),
                constant: i128::from(*expected as u8),
            };
            let domain = [
                left.clone(),
                LinearConstraint {
                    coefficients: HashMap::new(),
                    constant: 1,
                }
                .combine(left.clone(), 1, -1)?,
            ];
            let mut alternatives = comparison_constraints(left, BinaryOp::Equal, right, matched)?;
            for alternative in &mut alternatives {
                alternative.extend(domain.iter().cloned().map(PathConstraint::Linear));
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
                let left = LinearConstraint::from_value(linear_value(
                    left,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?);
                let right = LinearConstraint::from_value(linear_value(
                    right,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?);
                comparison_constraints(left, *op, right, expected)
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

fn symbolic_bool_with_constraints(
    expression: &Expr,
    return_expression: &Expr,
    guards: &[PathConstraint],
    bindings: Option<&HashMap<String, &Expr>>,
    functions: Option<&HashMap<String, &Function>>,
    depth: usize,
) -> Option<bool> {
    if let Some(value) = symbolic_bool(
        expression,
        Some(return_expression),
        bindings,
        None,
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
        None,
        functions,
        depth,
    )?;
    let false_constraints = constraints_for_bool(
        expression,
        false,
        Some(return_expression),
        bindings,
        None,
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
        ExprKind::Call { name, args } => {
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

fn linear_value(
    expression: &Expr,
    return_expression: Option<&Expr>,
    bindings: Option<&HashMap<String, &Expr>>,
    substitutions: Option<&HashMap<String, LinearValue>>,
    functions: Option<&HashMap<String, &Function>>,
    depth: usize,
) -> Option<LinearValue> {
    match &expression.kind {
        ExprKind::Int(value) => Some(LinearValue {
            constant: *value,
            ..LinearValue::default()
        }),
        ExprKind::Variable(name) if name == "result" => return_expression.and_then(|expression| {
            linear_value(
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
            substitutions?.get(name).cloned()
        }
        ExprKind::Variable(name) if bindings.and_then(|bindings| bindings.get(name)).is_some() => {
            let bound = bindings?.get(name)?;
            linear_value(
                bound,
                return_expression,
                bindings,
                substitutions,
                functions,
                depth,
            )
        }
        ExprKind::Variable(name) => Some(LinearValue {
            coefficients: HashMap::from([(name.clone(), 1)]),
            constant: 0,
        }),
        ExprKind::Unary {
            op: UnaryOp::Negate,
            expr,
        } => linear_value(
            expr,
            return_expression,
            bindings,
            substitutions,
            functions,
            depth,
        )?
        .negate(),
        ExprKind::Binary { left, op, right } => {
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
            match op {
                BinaryOp::Add => left.add(right),
                BinaryOp::Subtract => left.subtract(right),
                BinaryOp::Multiply if right.coefficients.is_empty() => left.scale(right.constant),
                BinaryOp::Multiply if left.coefficients.is_empty() => right.scale(left.constant),
                _ => None,
            }
        }
        ExprKind::Call { name, args } => {
            if depth >= 32 {
                return None;
            }
            let function = functions?.get(name)?;
            let callee_return_expression = direct_return_expression(function)?;
            if args.len() != function.params.len() {
                return None;
            }
            let mut argument_values = HashMap::new();
            for (parameter, argument) in function.params.iter().zip(args) {
                let value = linear_value(
                    argument,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth + 1,
                )?;
                argument_values.insert(parameter.name.clone(), value);
            }
            linear_value(
                callee_return_expression,
                None,
                None,
                Some(&argument_values),
                functions,
                depth + 1,
            )
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
        ExprKind::Variable(_) | ExprKind::Call { .. } | ExprKind::Sql { .. } => None,
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
            Stmt::Return { value: None, .. } | Stmt::Break { .. } => {}
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
                condition, body, ..
            } => {
                check_capability_expr(condition, function, functions, declared, errors);
                check_capability_block(body, function, functions, declared, errors);
            }
            Stmt::Loop { body, .. } | Stmt::Transaction { body, .. } => {
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
        ExprKind::Call { name, args } => {
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
            Flow::Continue | Flow::Break => Value::Unit,
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
        assert_eq!(results[3].status, VerificationStatus::Unproven);
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
}
