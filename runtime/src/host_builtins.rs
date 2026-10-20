use super::*;

impl AstInterpreter<'_, '_> {
    /// Evaluate built-ins that directly consult injected host services.
    /// Returns `None` when the name belongs to another built-in family.
    pub(super) fn eval_host_builtin(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Option<Result<Value, RuntimeError>> {
        let result = match name {
            "read_console" => {
                if args.len() != 1 {
                    return Some(Err(self.runtime_error(
                        span,
                        "read_console expects exactly one String prompt",
                    )));
                }
                if let Err(error) =
                    self.require_runtime_capability("Console", "console input", span)
                {
                    return Some(Err(error));
                }
                let prompt = match self.eval(&args[0], env) {
                    Ok(Value::String(prompt)) => prompt,
                    Ok(_) => {
                        return Some(Err(self.runtime_error(
                            args[0].span,
                            "read_console expects a String prompt",
                        )))
                    }
                    Err(error) => return Some(Err(error)),
                };
                self.host_services
                    .read_console_line(&prompt)
                    .map(|value| Value::Option(value.map(|value| Box::new(Value::String(value)))))
                    .map_err(|error| {
                        self.runtime_error(span, format!("console input failed: {error}"))
                    })
            }
            "uuid_v4" => {
                if !args.is_empty() {
                    return Some(Err(self.runtime_error(span, "uuid_v4 expects no arguments")));
                }
                if let Err(error) = self.require_runtime_capability("Random", "random access", span)
                {
                    return Some(Err(error));
                }
                uuid_v4_with_host_services(self.host_services.as_ref())
                    .map(Value::String)
                    .map_err(|error| self.runtime_error(span, error))
            }
            "now" => {
                if !args.is_empty() {
                    return Some(Err(self.runtime_error(span, "now expects no arguments")));
                }
                if let Err(error) = self.require_runtime_capability("Clock", "clock access", span) {
                    return Some(Err(error));
                }
                match self.host_services.unix_time_millis() {
                    Ok(timestamp) if zelyra_ast::temporal::valid_timestamp_millis(timestamp) => {
                        Ok(Value::Timestamp(timestamp))
                    }
                    Ok(_) => Err(self.runtime_error(
                        span,
                        "clock access failed: timestamp is outside years 0001 through 9999",
                    )),
                    Err(error) => {
                        Err(self.runtime_error(span, format!("clock access failed: {error}")))
                    }
                }
            }
            "env" => {
                if args.len() != 1 {
                    return Some(Err(
                        self.runtime_error(span, "env expects exactly one argument")
                    ));
                }
                if let Err(error) =
                    self.require_runtime_capability("Environment", "environment access", span)
                {
                    return Some(Err(error));
                }
                let key = match self.eval(&args[0], env) {
                    Ok(Value::String(key)) => key,
                    Ok(_) => {
                        return Some(Err(
                            self.runtime_error(args[0].span, "env expects a String variable name")
                        ))
                    }
                    Err(error) => return Some(Err(error)),
                };
                if key.is_empty() {
                    return Some(Err(self.runtime_error(
                        args[0].span,
                        "env expects a non-empty variable name",
                    )));
                }
                match self.host_services.environment_variable(&key) {
                    Ok(value) => Ok(Value::Option(
                        value.map(|value| Box::new(Value::String(value))),
                    )),
                    Err(error) => Err(self.runtime_error(args[0].span, error)),
                }
            }
            "random_int" => {
                if args.len() != 2 {
                    return Some(Err(self.runtime_error(
                        span,
                        "random_int expects minimum and maximum Int arguments",
                    )));
                }
                if let Err(error) = self.require_runtime_capability("Random", "random access", span)
                {
                    return Some(Err(error));
                }
                let minimum = match self.eval(&args[0], env) {
                    Ok(value) => value,
                    Err(error) => return Some(Err(error)),
                };
                let maximum = match self.eval(&args[1], env) {
                    Ok(value) => value,
                    Err(error) => return Some(Err(error)),
                };
                match (minimum, maximum) {
                    (Value::Int(minimum), Value::Int(maximum)) => {
                        random_int_with_host_services(self.host_services.as_ref(), minimum, maximum)
                            .map(Value::Int)
                            .map_err(|error| self.runtime_error(span, error))
                    }
                    _ => Err(self.runtime_error(span, "random_int expects Int arguments")),
                }
            }
            _ => return None,
        };
        Some(result)
    }
}
