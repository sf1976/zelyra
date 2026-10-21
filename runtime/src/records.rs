//! Record/class construction kept outside the recursive expression dispatch frame.
use super::*;
use std::sync::atomic::Ordering;

impl Interpreter<'_, '_> {
    pub(crate) fn eval_record_value(
        &mut self,
        type_name: &str,
        type_args: &[Type],
        fields: &[(String, Expr)],
        span: Span,
        env: &mut Environment,
    ) -> Result<Value, RuntimeError> {
        let mut values = HashMap::new();
        for (field, value) in fields {
            values.insert(field.clone(), self.eval(value, env)?);
        }
        let definition = self
            .program_context
            .record_definitions
            .get(type_name)
            .ok_or_else(|| {
                self.runtime_error(span, format!("unknown record type `{type_name}`"))
            })?;
        for field in &definition.fields {
            if !values.contains_key(&field.name) && matches!(&field.ty, Type::Option(_)) {
                values.insert(field.name.clone(), Value::Option(None));
            }
        }
        let concrete_args = if !type_args.is_empty() {
            type_args.to_vec()
        } else if definition.type_params.is_empty() {
            Vec::new()
        } else {
            let parameters = definition
                .type_params
                .iter()
                .cloned()
                .collect::<HashSet<_>>();
            let mut substitutions = HashMap::new();
            for field in &definition.fields {
                if let Some(actual) = values.get(&field.name) {
                    infer_generic_types(&field.ty, &actual.ty(), &parameters, &mut substitutions);
                }
            }
            definition
                .type_params
                .iter()
                .map(|parameter| substitutions.get(parameter).cloned())
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    self.runtime_error(
                        span,
                        format!("cannot infer generic arguments for `{type_name}`"),
                    )
                })?
        };
        if definition.is_class {
            let object_id = loop {
                let current = self.next_class_object_id.load(Ordering::Relaxed);
                let next = current.checked_add(1).ok_or_else(|| {
                    self.runtime_error(span, "class object identity space exhausted")
                })?;
                if self
                    .next_class_object_id
                    .compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    break current;
                }
            };
            let reference = ClassObjectRef {
                object_id,
                object: Arc::new(Mutex::new(RuntimeClassObject { fields: values })),
            };
            return Ok(Value::ClassRef {
                reference,
                type_name: type_name.to_owned(),
                type_args: concrete_args,
            });
        }
        Ok(Value::Object {
            type_name: type_name.to_owned(),
            type_args: concrete_args,
            fields: values,
        })
    }
}
