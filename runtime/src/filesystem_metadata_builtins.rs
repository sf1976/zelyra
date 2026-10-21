use super::*;

impl AstInterpreter<'_, '_> {
    /// Evaluate filesystem metadata built-ins through the injected host.
    pub(super) fn eval_filesystem_metadata_builtin(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Option<Result<Value, RuntimeError>> {
        let result = match name {
            "file_exists" | "dir_exists" => {
                if args.len() != 1 {
                    return Some(Err(
                        self.runtime_error(span, format!("{name} expects one String"))
                    ));
                }
                if let Err(error) =
                    self.require_runtime_capability("FileSystem", "file-system read access", span)
                {
                    return Some(Err(error));
                }
                let path = match self.eval(&args[0], env) {
                    Ok(Value::String(path)) => path,
                    Ok(_) => {
                        return Some(Err(
                            self.runtime_error(span, format!("{name} expects a String"))
                        ))
                    }
                    Err(error) => return Some(Err(error)),
                };
                let roots = self
                    .execution_context
                    .filesystem_policy
                    .as_ref()
                    .map_or_else(Vec::new, |policy| policy.read_roots.clone());
                match self.resolve_filesystem_path(&path, &roots, "read", span, true) {
                    Ok(resolved) => {
                        let metadata = self
                            .execution_context
                            .host_services
                            .file_metadata(&resolved);
                        let exists = metadata.is_ok_and(|metadata| {
                            if name == "file_exists" {
                                metadata.is_file
                            } else {
                                metadata.is_directory
                            }
                        });
                        Ok(Value::Bool(exists))
                    }
                    Err(_) => Ok(Value::Bool(false)),
                }
            }
            "file_size" => {
                if args.len() != 1 {
                    return Some(Err(self.runtime_error(span, "file_size expects one String")));
                }
                if let Err(error) =
                    self.require_runtime_capability("FileSystem", "file-system read access", span)
                {
                    return Some(Err(error));
                }
                let path = match self.eval(&args[0], env) {
                    Ok(Value::String(path)) => path,
                    Ok(_) => {
                        return Some(Err(self.runtime_error(span, "file_size expects a String")))
                    }
                    Err(error) => return Some(Err(error)),
                };
                let roots = self
                    .execution_context
                    .filesystem_policy
                    .as_ref()
                    .map_or_else(Vec::new, |policy| policy.read_roots.clone());
                let resolved =
                    match self.resolve_filesystem_path(&path, &roots, "read", span, false) {
                        Ok(resolved) => resolved,
                        Err(error) => return Some(Err(error)),
                    };
                match self
                    .execution_context
                    .host_services
                    .file_metadata(&resolved)
                {
                    Ok(metadata) => Ok(Value::Result(Ok(Box::new(Value::Int(
                        metadata.length as i64,
                    ))))),
                    Err(error) => Ok(Value::Result(Err(Box::new(Value::String(error))))),
                }
            }
            _ => return None,
        };
        Some(result)
    }
}
