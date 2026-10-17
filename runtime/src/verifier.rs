//! Contract and loop-invariant verifier for Zelyra programs.
//!
//! This analyzer reports only proofs or checks justified by its symbolic
//! model; unsupported constructs remain explicitly unproven.
use super::*;

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
        PatternKind::EnumVariant { .. } => {}
        PatternKind::EnumVariantPayload { .. } => {}
        PatternKind::Wildcard
        | PatternKind::Int(_)
        | PatternKind::Bool(_)
        | PatternKind::String(_)
        | PatternKind::Char(_)
        | PatternKind::Record { .. } => {}
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
            Stmt::ArrayIndexAssign { name, .. } => {
                names.insert(name.clone());
            }
            Stmt::TupleBindOrAssign {
                names: assigned, ..
            } => {
                names.extend(assigned.iter().cloned());
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
            | Stmt::ArrayIndexAssign { .. }
            | Stmt::TupleBindOrAssign { .. }
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
        ExprKind::Map(entries) => entries.iter().any(|(key, value)| {
            expression_contains_variable(key, name) || expression_contains_variable(value, name)
        }),
        ExprKind::Record { fields, .. } => fields
            .iter()
            .any(|(_, value)| expression_contains_variable(value, name)),
        ExprKind::Index { target, index } => {
            expression_contains_variable(target, name) || expression_contains_variable(index, name)
        }
        ExprKind::Field { target, .. } => expression_contains_variable(target, name),
        ExprKind::Await(inner) => expression_contains_variable(inner, name),
        ExprKind::Match { value, arms } => {
            expression_contains_variable(value, name)
                || arms
                    .iter()
                    .any(|arm| expression_contains_variable(&arm.value, name))
        }
        ExprKind::Lambda { params, body } => {
            !params.iter().any(|param| param.name == name)
                && expression_contains_variable(body, name)
        }
        ExprKind::Int(..)
        | ExprKind::UInt(..)
        | ExprKind::Float(..)
        | ExprKind::Bool(..)
        | ExprKind::String(..)
        | ExprKind::Char(..)
        | ExprKind::FunctionValue(..)
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
        PatternKind::String(_)
        | PatternKind::Char(_)
        | PatternKind::Record { .. }
        | PatternKind::EnumVariant { .. }
        | PatternKind::EnumVariantPayload { .. } => None,
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
        | ExprKind::Map(_)
        | ExprKind::Record { .. }
        | ExprKind::Lambda { .. }
        | ExprKind::Index { .. }
        | ExprKind::Field { .. }
        | ExprKind::Variable(_)
        | ExprKind::FunctionValue(_)
        | ExprKind::Call { .. }
        | ExprKind::Sql { .. }
        | ExprKind::Await(_)
        | ExprKind::Match { .. } => None,
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
