//! Value-free tracing for deterministic, capability-free AST executions.
use super::ast_interpreter::{
    execution_api::run_on_runtime_stack, AstInterpreter, RuntimeServices, SqlTenantContext,
};
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionRule {
    Requires,
    Ensures,
    If,
    While,
    InvariantBefore,
    InvariantAfter,
    MatchGuard,
    MatchArm,
}

impl DecisionRule {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Requires => "requires",
            Self::Ensures => "ensures",
            Self::If => "if",
            Self::While => "while",
            Self::InvariantBefore => "invariant_before",
            Self::InvariantAfter => "invariant_after",
            Self::MatchGuard => "match_guard",
            Self::MatchArm => "match_arm",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionEvent {
    pub rule: DecisionRule,
    pub span: Span,
    pub accepted: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DecisionTrace {
    Suppressed,
    Evaluated {
        completed: bool,
        events: Vec<DecisionEvent>,
    },
}

struct TraceHost;

impl HostServices for TraceHost {
    fn unix_time_millis(&self) -> Result<i64, String> {
        Err("time is unavailable in decision tracing".into())
    }

    fn environment_variable(&self, _name: &str) -> Result<Option<String>, String> {
        Err("environment access is unavailable in decision tracing".into())
    }

    fn fill_random(&self, _output: &mut [u8]) -> Result<(), String> {
        Err("randomness is unavailable in decision tracing".into())
    }

    fn canonicalize_path(
        &self,
        _path: &std::path::Path,
        _allow_missing: bool,
    ) -> Result<PathBuf, String> {
        Err("filesystem access is unavailable in decision tracing".into())
    }
}

/// Even control-flow outcomes can reveal classified input. Suppress the
/// entire execution when the composed program contains a privacy annotation.
pub fn explain_decision(program: &Program, name: &str, args: Vec<Value>) -> DecisionTrace {
    if privacy::program_uses_privacy(program) {
        return DecisionTrace::Suppressed;
    }
    let policy = RuntimePolicy {
        step_budget: Some(10_000),
        ..RuntimePolicy::default()
    };
    let result = run_on_runtime_stack(
        runtime_stack_size_for_depth(DEFAULT_FUNCTION_CALL_DEPTH),
        || {
            let mut interpreter = AstInterpreter::new(
                program,
                None,
                Some(HashSet::new()),
                Some(policy),
                RuntimeServices {
                    host_services: Arc::new(TraceHost),
                    sql_tenant: SqlTenantContext::Caller,
                    database_transaction: None,
                    transactional_query: None,
                },
            );
            interpreter.decision_trace = Some(Vec::new());
            let completed = interpreter.call(name, args, Span::default()).is_ok();
            let events = interpreter.decision_trace.unwrap_or_default();
            Ok(DecisionTrace::Evaluated { completed, events })
        },
    );
    result.unwrap_or(DecisionTrace::Evaluated {
        completed: false,
        events: Vec::new(),
    })
}

impl AstInterpreter<'_, '_> {
    pub(super) fn trace_rule(&mut self, rule: DecisionRule, span: Span, accepted: bool) {
        if let Some(trace) = &mut self.decision_trace {
            trace.push(DecisionEvent {
                rule,
                span,
                accepted,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traces_contract_and_branch_outcomes_and_denies_host_effects() {
        let program = zelyra_parser::parse(
            &zelyra_lexer::lex(
                r#"fn decide(n: Int) -> Int requires { n > 0 } ensures { result > 0 } {
                if n > 10 { return 20 }
                return 1
            }
            fn external() uses Clock { print(now()) }"#,
            )
            .unwrap(),
        )
        .unwrap();
        let first = explain_decision(&program, "decide", vec![Value::Int(11)]);
        let DecisionTrace::Evaluated { completed, events } = &first else {
            panic!("trace suppressed")
        };
        assert!(*completed);
        assert_eq!(
            events
                .iter()
                .map(|event| (event.rule, event.accepted))
                .collect::<Vec<_>>(),
            [
                (DecisionRule::Requires, true),
                (DecisionRule::If, true),
                (DecisionRule::Ensures, true)
            ]
        );
        assert_eq!(
            first,
            explain_decision(&program, "decide", vec![Value::Int(11)])
        );
        assert!(
            matches!(explain_decision(&program, "decide", vec![Value::Int(0)]),
            DecisionTrace::Evaluated { completed: false, events } if events.len() == 1 && !events[0].accepted)
        );
        assert_eq!(
            explain_decision(&program, "external", Vec::new()),
            DecisionTrace::Evaluated {
                completed: false,
                events: Vec::new()
            }
        );
    }
}
