use super::*;

impl<'program> Interpreter<'program> {
    pub(super) fn eval_filesystem_io_builtin(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Option<Result<Value, RuntimeError>> {
        if !matches!(
            name,
            "read_text"
                | "read_text_result"
                | "write_text"
                | "write_text_result"
                | "write_text_new_result"
                | "atomic_write_text_result"
                | "append_text_result"
                | "copy_file_new_result"
                | "rename_file_result"
                | "delete_file"
                | "delete_file_result"
                | "list_dir"
        ) {
            return None;
        }
        let result = (|| {
            if name == "read_text" {
                if args.len() != 1 {
                    return Err(
                        self.runtime_error(span, "read_text expects exactly one String path")
                    );
                }
                self.require_runtime_capability("FileSystem", "file-system read access", span)?;
                let path = self.eval(&args[0], env)?;
                let Value::String(path) = path else {
                    return Err(self.runtime_error(args[0].span, "read_text expects a String path"));
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
                    span,
                    false,
                )?;
                self.host_services
                    .read_text(&path)
                    .map_err(|error| {
                        self.runtime_error(span, format!("file-system read failed: {error}"))
                    })
                    .map(Value::String)
            } else if name == "read_text_result" {
                if args.len() != 1 {
                    return Err(self
                        .runtime_error(span, "read_text_result expects exactly one String path"));
                }
                self.require_runtime_capability("FileSystem", "file-system read access", span)?;
                let path_value = self.eval(&args[0], env)?;
                let outcome = match path_value {
                    Value::String(path) if !path.is_empty() => {
                        let roots = self
                            .filesystem_policy
                            .as_ref()
                            .map_or_else(Vec::new, |policy| policy.read_roots.clone());
                        match self.resolve_filesystem_path(&path, &roots, "read", span, false) {
                            Ok(path) => self
                                .host_services
                                .read_text(&path)
                                .map_err(|error| format!("file-system read failed: {error}")),
                            Err(error) => Err(error.message),
                        }
                    }
                    Value::String(_) => Err("read_text_result expects a non-empty path".into()),
                    _ => Err("read_text_result expects a String path".into()),
                };
                Ok(Value::Result(
                    outcome
                        .map(|value| Box::new(Value::String(value)))
                        .map_err(|error| Box::new(Value::String(error))),
                ))
            } else if name == "write_text" {
                if args.len() != 2 {
                    return Err(self.runtime_error(
                        span,
                        "write_text expects a String path and String content",
                    ));
                }
                self.require_runtime_capability("FileSystem", "file-system write access", span)?;
                let path = self.eval(&args[0], env)?;
                let content = self.eval(&args[1], env)?;
                let (Value::String(path), Value::String(content)) = (path, content) else {
                    return Err(self.runtime_error(span, "write_text expects String arguments"));
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
                let path = self.resolve_filesystem_path(&path, &roots, "write", span, true)?;
                self.host_services
                    .write_text(&path, &content)
                    .map_err(|error| {
                        self.runtime_error(span, format!("file-system write failed: {error}"))
                    })?;
                Ok(Value::Unit)
            } else if name == "write_text_result" {
                if args.len() != 2 {
                    return Err(self.runtime_error(
                        span,
                        "write_text_result expects a String path and String content",
                    ));
                }
                self.require_runtime_capability("FileSystem", "file-system write access", span)?;
                let path_value = self.eval(&args[0], env)?;
                let content_value = self.eval(&args[1], env)?;
                let outcome = match (path_value, content_value) {
                    (Value::String(path), Value::String(content)) if !path.is_empty() => {
                        let roots = self
                            .filesystem_policy
                            .as_ref()
                            .map_or_else(Vec::new, |policy| policy.write_roots.clone());
                        match self.resolve_filesystem_path(&path, &roots, "write", span, true) {
                            Ok(path) => self
                                .host_services
                                .write_text(&path, &content)
                                .map_err(|error| format!("file-system write failed: {error}")),
                            Err(error) => Err(error.message),
                        }
                    }
                    (Value::String(_), Value::String(_)) => {
                        Err("write_text_result expects a non-empty path".into())
                    }
                    _ => Err("write_text_result expects String arguments".into()),
                };
                Ok(Value::Result(
                    outcome
                        .map(|_| Box::new(Value::Unit))
                        .map_err(|error| Box::new(Value::String(error))),
                ))
            } else if matches!(
                name,
                "write_text_new_result" | "atomic_write_text_result" | "append_text_result"
            ) {
                if args.len() != 2 {
                    return Err(self.runtime_error(
                        span,
                        format!("{name} expects a String path and String content"),
                    ));
                }
                self.require_runtime_capability("FileSystem", "file-system write access", span)?;
                let path = self.eval(&args[0], env)?;
                let content = self.eval(&args[1], env)?;
                let outcome = match (path, content) {
                    (Value::String(path), Value::String(content)) if !path.is_empty() => self
                        .resolve_filesystem_path(
                            &path,
                            &self.filesystem_roots(true),
                            "write",
                            span,
                            true,
                        )
                        .map_err(|error| error.message)
                        .and_then(|target| {
                            let operation = match name {
                                "write_text_new_result" => {
                                    self.host_services.write_text_new(&target, &content)
                                }
                                "atomic_write_text_result" => {
                                    self.host_services.atomic_write_text(&target, &content)
                                }
                                _ => self.host_services.append_text(&target, &content),
                            };
                            operation.map_err(|error| format!("file-system {name} failed: {error}"))
                        }),
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
            } else if matches!(name, "copy_file_new_result" | "rename_file_result") {
                if args.len() != 2 {
                    return Err(self.runtime_error(
                        span,
                        format!("{name} expects source and destination String paths"),
                    ));
                }
                self.require_runtime_capability("FileSystem", "file-system write access", span)?;
                let source = self.eval(&args[0], env)?;
                let destination = self.eval(&args[1], env)?;
                let outcome = match (source, destination) {
                    (Value::String(source), Value::String(destination))
                        if !source.is_empty() && !destination.is_empty() =>
                    {
                        let source_roots = if name == "copy_file_new_result" {
                            self.filesystem_policy
                                .as_ref()
                                .map_or_else(Vec::new, |policy| policy.read_roots.clone())
                        } else {
                            self.filesystem_roots(true)
                        };
                        let source_path = self
                            .resolve_filesystem_path(
                                &source,
                                &source_roots,
                                "read or rename",
                                span,
                                false,
                            )
                            .map_err(|error| error.message);
                        source_path.and_then(|source_path| {
                            self.resolve_filesystem_path(
                                &destination,
                                &self.filesystem_roots(true),
                                "copy or rename",
                                span,
                                true,
                            )
                            .map_err(|error| error.message)
                            .and_then(|destination_path| {
                                let operation = if name == "copy_file_new_result" {
                                    self.host_services
                                        .copy_file_new(&source_path, &destination_path)
                                } else {
                                    self.host_services
                                        .rename_file_new(&source_path, &destination_path)
                                };
                                operation
                                    .map_err(|error| format!("file-system {name} failed: {error}"))
                            })
                        })
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
            } else if name == "delete_file" || name == "delete_file_result" {
                if args.len() != 1 {
                    return Err(self.runtime_error(span, "delete_file expects one String path"));
                }
                self.require_runtime_capability("FileSystem", "file-system delete access", span)?;
                let path_value = self.eval(&args[0], env)?;
                let operation = match path_value {
                    Value::String(path) if !path.is_empty() => {
                        let roots = self.filesystem_roots(true);
                        self.resolve_filesystem_path(&path, &roots, "delete", span, false)
                            .and_then(|path| {
                                self.host_services.remove_file(&path).map_err(|error| {
                                    self.runtime_error(
                                        span,
                                        format!("file-system delete failed: {error}"),
                                    )
                                })
                            })
                    }
                    Value::String(_) => {
                        Err(self.runtime_error(span, "delete_file expects a non-empty path"))
                    }
                    _ => Err(self.runtime_error(span, "delete_file expects a String path")),
                };
                if name == "delete_file_result" {
                    Ok(Value::Result(
                        operation
                            .map(|()| Box::new(Value::Unit))
                            .map_err(|error| Box::new(Value::String(error.message))),
                    ))
                } else {
                    operation.map(|()| Value::Unit)
                }
            } else if name == "list_dir" {
                if args.len() != 1 {
                    return Err(self.runtime_error(span, "list_dir expects one String path"));
                }
                self.require_runtime_capability("FileSystem", "file-system read access", span)?;
                let path = self.eval(&args[0], env)?;
                let Value::String(path) = path else {
                    return Err(self.runtime_error(args[0].span, "list_dir expects a String path"));
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
                let path = self.resolve_filesystem_path(&path, &roots, "list", span, false)?;
                let mut entries = self
                    .host_services
                    .list_directory(&path)
                    .map_err(|error| {
                        self.runtime_error(span, format!("file-system list failed: {error}"))
                    })?
                    .into_iter()
                    .map(Value::String)
                    .collect::<Vec<_>>();
                entries.sort_by_key(Value::output);
                Ok(Value::Array(entries))
            } else {
                unreachable!("filesystem I/O name was checked above")
            }
        })();
        Some(result)
    }
}
