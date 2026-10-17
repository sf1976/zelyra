use super::*;

impl<'program> Interpreter<'program> {
    pub(super) fn eval_path_builtin(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Option<Result<Value, RuntimeError>> {
        if !matches!(
            name,
            "path_join" | "path_filename" | "path_extension" | "path_dir"
        ) {
            return None;
        }
        let result = (|| match name {
            "path_join" => {
                if args.len() != 2 {
                    return Err(self.runtime_error(span, "path_join expects two String arguments"));
                }
                match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                    (Value::String(a), Value::String(b)) => {
                        let path = Path::new(&a).join(Path::new(&b));
                        Ok(Value::String(path.to_string_lossy().to_string()))
                    }
                    _ => Err(self.runtime_error(span, "path_join expects two Strings")),
                }
            }
            "path_filename" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(span, "path_filename expects one String"));
                }
                match self.eval(&args[0], env)? {
                    Value::String(value) => Ok(Value::String(
                        Path::new(&value)
                            .file_name()
                            .and_then(|file| file.to_str())
                            .unwrap_or("")
                            .to_string(),
                    )),
                    _ => Err(self.runtime_error(span, "path_filename expects a String")),
                }
            }
            "path_extension" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(span, "path_extension expects one String"));
                }
                match self.eval(&args[0], env)? {
                    Value::String(value) => Ok(Value::String(
                        Path::new(&value)
                            .extension()
                            .and_then(|extension| extension.to_str())
                            .unwrap_or("")
                            .to_string(),
                    )),
                    _ => Err(self.runtime_error(span, "path_extension expects a String")),
                }
            }
            "path_dir" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(span, "path_dir expects one String"));
                }
                match self.eval(&args[0], env)? {
                    Value::String(value) => Ok(Value::String(
                        Path::new(&value)
                            .parent()
                            .and_then(|parent| parent.to_str())
                            .unwrap_or("")
                            .to_string(),
                    )),
                    _ => Err(self.runtime_error(span, "path_dir expects a String")),
                }
            }
            _ => unreachable!("path builtin name was checked above"),
        })();
        Some(result)
    }
}
