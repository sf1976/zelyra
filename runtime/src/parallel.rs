//! Parallel execution kept outside the recursive statement dispatch frame.
use super::*;

impl Interpreter<'_, '_> {
    pub(super) fn exec_parallel(
        &mut self,
        body: &Block,
        span: Span,
        env: &mut Environment,
    ) -> Result<Flow, RuntimeError> {
        let mut branches = Vec::new();
        let mut names = HashSet::new();
        for statement in &body.statements {
            let Stmt::BindOrAssign { name, value, .. } = statement else {
                return Err(self.runtime_error(
                    span,
                    "parallel blocks may contain only `name = await expression` bindings",
                ));
            };
            let ExprKind::Await(inner) = &value.kind else {
                return Err(
                    self.runtime_error(value.span, "parallel bindings must await their expression")
                );
            };
            if !names.insert(name.clone()) {
                return Err(self.runtime_error(
                    value.span,
                    format!("parallel binding `{name}` is declared twice"),
                ));
            }
            branches.push((name.as_str(), inner.as_ref(), value.span));
        }
        if branches.len() > MAX_PARALLEL_BRANCHES {
            return Err(self.runtime_error(
                span,
                format!("parallel block exceeds the maximum of {MAX_PARALLEL_BRANCHES} branches"),
            ));
        }
        let mut results = Vec::new();
        let mut first_error = None;
        std::thread::scope(|scope| {
            let handles = branches
                .iter()
                .map(|(name, expression, branch_span)| {
                    let mut child = Interpreter {
                        functions: self.functions.clone(),
                        view_definitions: self.view_definitions.clone(),
                        output: Vec::new(),
                        next_class_object_id: Arc::clone(&self.next_class_object_id),
                        parallel_worker: true,
                        step_budget: self.step_budget.clone(),
                        call_depth_limit: self.call_depth_limit,
                        call_depth: 0,
                        database_url: self.database_url.clone(),
                        database_transaction: self.database_transaction.clone(),
                        transactional_query: None,
                        storage_schema: self.storage_schema.clone(),
                        storage_checked: self.storage_checked,
                        scoped_tables: self.scoped_tables.clone(),
                        granted_capabilities: self.granted_capabilities.clone(),
                        filesystem_policy: self.filesystem_policy.clone(),
                        network_policy: self.network_policy.clone(),
                        process_policy: self.process_policy.clone(),
                        mail_policy: self.mail_policy.clone(),
                        package_settings: self.package_settings.clone(),
                        host_services: Arc::clone(&self.host_services),
                        record_definitions: self.record_definitions.clone(),
                        sql_record_definitions: self.sql_record_definitions.clone(),
                        type_aliases: self.type_aliases.clone(),
                        enum_definitions: self.enum_definitions.clone(),
                        active_capabilities: self.active_capabilities.clone(),
                        active_type_substitutions: self.active_type_substitutions.clone(),
                    };
                    let mut child_env = env.clone();
                    let stack_size = runtime_stack_size_for_depth(child.call_depth_limit);
                    std::thread::Builder::new()
                        .name("zelyra-parallel".into())
                        .stack_size(stack_size)
                        .spawn_scoped(scope, move || {
                            child
                                .eval(expression, &mut child_env)
                                .map(|value| ((*name).to_owned(), value, child.output))
                                .map_err(|error| (*branch_span, error))
                        })
                })
                .collect::<Vec<_>>();
            for handle in handles {
                match handle {
                    Ok(handle) => match handle.join() {
                        Ok(Ok(result)) => results.push(result),
                        Ok(Err((_, error))) => {
                            if first_error.is_none() {
                                first_error = Some(
                                    if error.message.starts_with("execution step budget exceeded") {
                                        // Branch scheduling is concurrent, so report a shared-budget
                                        // failure at the stable parallel statement span.
                                        self.runtime_error(span, error.message)
                                    } else {
                                        error
                                    },
                                );
                            }
                        }
                        Err(_) => {
                            if first_error.is_none() {
                                first_error =
                                    Some(self.runtime_error(span, "parallel task panicked"));
                            }
                        }
                    },
                    Err(_) => {
                        if first_error.is_none() {
                            first_error =
                                Some(self.runtime_error(
                                    span,
                                    "parallel task thread could not be started",
                                ));
                        }
                    }
                }
            }
        });
        if let Some(error) = first_error {
            return Err(error);
        }
        for (name, value, output) in results {
            self.output.extend(output);
            if env.contains(&name) {
                env.assign(&name, value).map_err(|reason| {
                    self.runtime_error(
                        span,
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
