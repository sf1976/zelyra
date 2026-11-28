use super::*;

impl<'program, 'services> Interpreter<'program, 'services> {
    pub(crate) fn prepare_template_mail(
        &mut self,
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Result<zelyra_database::Query, RuntimeError> {
        let [template, recipient, subject, data] = args else {
            return Err(self.runtime_error(
                span,
                "mail_queue_template expects a view name, recipient, subject, and record",
            ));
        };
        let ExprKind::String(template_name) = &template.kind else {
            return Err(self.runtime_error(
                template.span,
                "mail_queue_template requires a literal view name",
            ));
        };
        let view = self
            .program_context
            .view_definitions
            .get(template_name)
            .copied()
            .ok_or_else(|| {
                self.runtime_error(template.span, "mail_queue_template names an unknown view")
            })?;
        self.require_runtime_capability(
            &format!("Email(templates: [{template_name}])"),
            "mail template",
            template.span,
        )?;
        self.require_runtime_capability(
            "Database(write: [_zelyra_outbox])",
            "transactional mail queue write",
            span,
        )?;
        if !self
            .database_url
            .as_deref()
            .is_some_and(supports_mail_outbox_database)
        {
            return Err(self.runtime_error(
                span,
                "mail_queue_template requires a MariaDB or MySQL outbox",
            ));
        }
        if ![recipient, subject, data]
            .into_iter()
            .all(is_pure_mail_expression)
        {
            return Err(self.runtime_error(
                span,
                "mail_queue_template arguments must be values or pure expressions",
            ));
        }
        let recipient_value = self.eval(recipient, env)?;
        let subject_value = self.eval(subject, env)?;
        let data_value = self.eval(data, env)?;
        if !matches!(&data_value, Value::Object { .. }) {
            return Err(self.runtime_error(
                data.span,
                "mail_queue_template expects a record for the view",
            ));
        }
        let html = interpolate_pdf_data(&view.html, &data_value)
            .map_err(|_| self.runtime_error(template.span, "could not render the mail template"))?;
        prepare_mail_queue_query(
            &[
                recipient_value,
                subject_value.clone(),
                subject_value,
                Value::String(html),
            ],
            &[recipient.span, subject.span, subject.span, template.span],
            span,
            self.execution_context.host_services.as_ref(),
        )
    }

    pub(crate) fn require_runtime_capability(
        &self,
        capability: &str,
        operation: &str,
        span: Span,
    ) -> Result<(), RuntimeError> {
        let declared = self.active_capabilities.last().is_some_and(|capabilities| {
            crate::capabilities::capability_set_satisfies_primitive(
                capabilities.iter().map(String::as_str),
                capability,
            )
        });
        if !declared {
            return Err(self.runtime_error(
                span,
                format!(
                    "runtime capability denied: {operation} requires `{capability}` in the current function"
                ),
            ));
        }
        if self
            .execution_context
            .granted_capabilities
            .as_ref()
            .is_some_and(|grants| !project_grants_capability(grants, capability))
        {
            return Err(self.runtime_error(
                span,
                format!("runtime capability denied: `{capability}` is not granted"),
            ));
        }
        Ok(())
    }

    pub(crate) fn require_runtime_sql_table_scope(
        &self,
        query: &str,
        span: Span,
    ) -> Result<(), RuntimeError> {
        let Some(capabilities) = self.active_capabilities.last() else {
            return Ok(());
        };
        let declared = capabilities.iter().map(String::as_str).collect::<Vec<_>>();
        let known_tables: Vec<String> = self.program_context.storage_schema.as_ref().map_or_else(
            |_| Vec::new(),
            |schema| schema.tables.iter().map(|t| t.name.clone()).collect(),
        );
        crate::capabilities::check_sql_table_scope(query, &known_tables, &declared).map_err(
            |reason| self.runtime_error(span, format!("runtime capability denied: {reason}")),
        )
    }

    pub(crate) fn filesystem_roots(&self, write: bool) -> Vec<PathBuf> {
        self.execution_context
            .filesystem_policy
            .as_ref()
            .map_or_else(Vec::new, |policy| {
                if write {
                    policy.write_roots.clone()
                } else {
                    policy.read_roots.clone()
                }
            })
    }

    pub(crate) fn resolve_filesystem_path(
        &self,
        path: &str,
        roots: &[PathBuf],
        operation: &str,
        span: Span,
        allow_missing: bool,
    ) -> Result<PathBuf, RuntimeError> {
        let declared = self
            .active_capabilities
            .last()
            .map_or_else(Vec::new, |capabilities| {
                capabilities.iter().map(String::as_str).collect::<Vec<_>>()
            });
        let scoped = crate::capabilities::filesystem_path_is_scoped(&declared);
        if scoped {
            crate::capabilities::check_filesystem_path_scope(path, &declared)
                .map_err(|reason| self.runtime_error(span, reason))?;
        }
        let policy = self.execution_context.filesystem_policy.as_ref();
        let host = self.execution_context.host_services.as_ref();
        let resolved =
            resolve_filesystem_path_rule(policy, path, roots, operation, allow_missing, Some(host))
                .map_err(|message| self.runtime_error(span, message))?;
        if !scoped {
            return Ok(resolved);
        }

        let base = policy.map_or(Path::new("."), |policy| policy.base_dir.as_path());
        let base = host
            .canonicalize_path(base, false)
            .map_err(|error| self.runtime_error(span, error))?;
        let resolved = if policy.is_some() {
            resolved
        } else {
            host.canonicalize_path(&resolved, allow_missing)
                .map_err(|error| self.runtime_error(span, error))?
        };
        let relative = resolved
            .strip_prefix(&base)
            .ok()
            .and_then(Path::to_str)
            .ok_or_else(|| {
                self.runtime_error(
                    span,
                    "resolved path is outside the declared `FileSystem(scope: ...)`",
                )
            })?;
        let relative = relative.replace(std::path::MAIN_SEPARATOR, "/");
        crate::capabilities::check_filesystem_path_scope(&relative, &declared)
            .map_err(|reason| self.runtime_error(span, reason))?;
        Ok(resolved)
    }

    pub(crate) fn http_request(
        &self,
        method: &str,
        url: &str,
        request_headers: &[String],
        request_body: Option<&str>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if let Some(capabilities) = self.active_capabilities.last() {
            let declared = capabilities.iter().map(String::as_str).collect::<Vec<_>>();
            crate::capabilities::check_network_url_scope(url, &declared).map_err(|reason| {
                self.runtime_error(span, format!("runtime capability denied: {reason}"))
            })?;
        }
        let response = execute_http_request_with_host(
            method,
            url,
            request_headers,
            request_body,
            self.execution_context.network_policy.as_ref(),
            self.execution_context.host_services.as_ref(),
        )
        .map_err(|error| self.runtime_error(span, error))?;
        http_response_value_from_host(response).map_err(|error| self.runtime_error(span, error))
    }

    pub(crate) fn http_json(
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

    pub(crate) fn http_result(
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

    pub(crate) fn http_get(&self, url: &str, span: Span) -> Result<String, RuntimeError> {
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

    pub(crate) fn run_process(
        &self,
        command: &str,
        arguments: &[String],
        span: Span,
    ) -> Result<String, RuntimeError> {
        run_process_with_host_services(
            command,
            arguments,
            self.execution_context.process_policy.as_ref(),
            self.execution_context.host_services.as_ref(),
        )
        .map_err(|error| self.runtime_error(span, error))
    }

    pub(crate) fn run_process_piped(
        &self,
        command: &str,
        arguments: &[String],
        stdin: &str,
        timeout_ms: u64,
        span: Span,
    ) -> Result<HostProcessResult, RuntimeError> {
        run_process_piped_with_host_services(
            command,
            arguments,
            stdin,
            timeout_ms,
            self.execution_context.process_policy.as_ref(),
            self.execution_context.host_services.as_ref(),
        )
        .map_err(|error| self.runtime_error(span, error))
    }
}
