use super::*;

impl<'program> Interpreter<'program> {
    pub(crate) fn call(
        &mut self,
        name: &str,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        self.call_with_type_args(name, &[], args, span)
    }

    pub(crate) fn call_with_type_args(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let function = self
            .functions
            .get(name)
            .copied()
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
        let resolved_type_args = type_args
            .iter()
            .map(|ty| {
                self.active_type_substitutions
                    .last()
                    .map_or_else(|| ty.clone(), |active| substitute_generic_type(ty, active))
            })
            .collect::<Vec<_>>();
        let type_args = resolved_type_args.as_slice();
        let substitutions = if function.type_params.is_empty() {
            if !type_args.is_empty() {
                return Err(self.runtime_error(
                    span,
                    format!("function `{name}` does not accept type arguments"),
                ));
            }
            HashMap::new()
        } else {
            if !type_args.is_empty() && type_args.len() != function.type_params.len() {
                return Err(self.runtime_error(
                    span,
                    format!(
                        "generic function `{name}` expects {} type argument(s), found {}",
                        function.type_params.len(),
                        type_args.len()
                    ),
                ));
            }
            let parameters = function.type_params.iter().cloned().collect::<HashSet<_>>();
            let mut substitutions = function
                .type_params
                .iter()
                .zip(type_args)
                .map(|(parameter, ty)| (parameter.clone(), ty.clone()))
                .collect::<HashMap<_, _>>();
            if type_args.is_empty() {
                for (parameter, value) in function.params.iter().zip(&args) {
                    infer_generic_types(
                        &parameter.ty,
                        &value.ty(),
                        &parameters,
                        &mut substitutions,
                    );
                }
            }
            if let Some(missing) = function
                .type_params
                .iter()
                .find(|parameter| !substitutions.contains_key(*parameter))
            {
                return Err(self.runtime_error(
                    span,
                    format!(
                        "cannot infer type parameter `{missing}` for `{name}`; provide explicit type arguments"
                    ),
                ));
            }
            substitutions
        };
        let expected_return = function
            .return_type
            .as_ref()
            .map(|ty| self.runtime_alias_type(&substitute_generic_type(ty, &substitutions), span))
            .transpose()?;
        if let Some(grants) = &self.granted_capabilities {
            if let Some(capability) = function
                .capabilities
                .iter()
                .find(|capability| !project_grants_capability(grants, capability))
            {
                return Err(self.runtime_error(
                    function.span,
                    format!("runtime capability denied: function `{name}` requires `{capability}`"),
                ));
            }
        }
        if self.call_depth >= self.call_depth_limit {
            return Err(self.runtime_error(
                span,
                format!(
                    "function call depth limit exceeded (maximum {})",
                    self.call_depth_limit
                ),
            ));
        }
        self.call_depth += 1;
        self.active_capabilities
            .push(function.capabilities.iter().cloned().collect());
        self.active_type_substitutions.push(substitutions.clone());
        let result = (|| {
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
            let flow = self.exec_block(&function.body, &mut env)?;
            let returned = matches!(&flow, Flow::Return(_));
            let result = match flow {
                Flow::Return(value) => value,
                Flow::Continue | Flow::LoopContinue | Flow::Break => Value::Unit,
            };
            if let Some(expected) = expected_return.as_ref() {
                if *expected != Type::Unit && !returned {
                    return Err(self.runtime_error(
                        function.body.span,
                        format!("function `{name}` reached its end without returning `{expected}`"),
                    ));
                }
                if returned && !compatible(expected, &self.runtime_alias_type(&result.ty(), span)?)
                {
                    return Err(self.runtime_error(
                        function.body.span,
                        format!(
                            "function `{name}` returned `{}`, expected `{expected}`",
                            result.ty()
                        ),
                    ));
                }
            }
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
        })();
        self.active_type_substitutions.pop();
        self.active_capabilities.pop();
        self.call_depth -= 1;
        result
    }
}
