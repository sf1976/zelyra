use super::*;

#[derive(Clone)]
struct RuntimeVariable {
    value: Value,
    mutable: bool,
}

#[derive(Clone)]
pub(crate) struct Environment {
    scopes: Vec<HashMap<String, RuntimeVariable>>,
}

impl Environment {
    pub(crate) fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    pub(crate) fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(crate) fn pop(&mut self) {
        self.scopes.pop();
    }

    pub(crate) fn with_scope<T>(&mut self, action: impl FnOnce(&mut Environment) -> T) -> T {
        self.push();
        let result = action(self);
        self.pop();
        result
    }

    pub(crate) fn declare(&mut self, name: String, value: Value, mutable: bool) {
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name, RuntimeVariable { value, mutable });
    }

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.scopes
            .iter()
            .rev()
            .any(|scope| scope.contains_key(name))
    }

    pub(crate) fn is_mutable(&self, name: &str) -> Option<bool> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).map(|variable| variable.mutable))
    }

    pub(crate) fn visible_values(&self) -> Vec<(String, Value)> {
        let mut values = std::collections::BTreeMap::new();
        for scope in &self.scopes {
            for (name, variable) in scope {
                values.insert(name.clone(), variable.value.clone());
            }
        }
        values.into_iter().collect()
    }

    pub(crate) fn get(&self, name: &str) -> Option<Value> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).map(|variable| variable.value.clone()))
    }

    pub(crate) fn assign(&mut self, name: &str, value: Value) -> Result<(), &'static str> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(variable) = scope.get_mut(name) {
                if !variable.mutable {
                    return Err("immutable");
                }
                variable.value = value;
                return Ok(());
            }
        }
        Err("missing")
    }

    pub(crate) fn mutable_array_element(&self, name: &str, index: usize) -> Result<Value, String> {
        let variable = self
            .scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .ok_or("unknown array binding")?;
        if !variable.mutable {
            return Err("cannot update an immutable array binding".into());
        }
        let Value::Array(values) = &variable.value else {
            return Err("indexed update target is not an array".into());
        };
        values
            .get(index)
            .cloned()
            .ok_or_else(|| format!("array index {index} is out of bounds"))
    }

    pub(crate) fn assign_mutable_array_element(
        &mut self,
        name: &str,
        index: usize,
        value: Value,
    ) -> Result<(), String> {
        let variable = self
            .scopes
            .iter_mut()
            .rev()
            .find_map(|scope| scope.get_mut(name))
            .ok_or("unknown array binding")?;
        if !variable.mutable {
            return Err("cannot update an immutable array binding".into());
        }
        let Value::Array(values) = &mut variable.value else {
            return Err("indexed update target is not an array".into());
        };
        let element = values
            .get_mut(index)
            .ok_or_else(|| format!("array index {index} is out of bounds"))?;
        *element = value;
        Ok(())
    }

    pub(crate) fn push_mutable_array(
        &mut self,
        name: &str,
        value: Value,
    ) -> Result<(), &'static str> {
        let variable = self
            .scopes
            .iter_mut()
            .rev()
            .find_map(|scope| scope.get_mut(name))
            .ok_or("unknown array binding")?;
        if !variable.mutable {
            return Err("cannot update an immutable array binding");
        }
        let Value::Array(values) = &mut variable.value else {
            return Err("push_mut target is not an array");
        };
        values.push(value);
        Ok(())
    }

    pub(crate) fn pop_mutable_array(&mut self, name: &str) -> Result<Option<Value>, &'static str> {
        let variable = self
            .scopes
            .iter_mut()
            .rev()
            .find_map(|scope| scope.get_mut(name))
            .ok_or("unknown array binding")?;
        if !variable.mutable {
            return Err("cannot update an immutable array binding");
        }
        let Value::Array(values) = &mut variable.value else {
            return Err("pop_mut target is not an array");
        };
        Ok(values.pop())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_bindings_are_removed_when_action_returns_an_error() {
        let mut environment = Environment::new();
        environment.declare("outer".into(), Value::Int(1), false);

        let result: Result<(), &'static str> = environment.with_scope(|environment| {
            environment.declare("inner".into(), Value::Int(2), false);
            Err("scope action failed")
        });

        assert_eq!(result, Err("scope action failed"));
        assert_eq!(environment.get("outer"), Some(Value::Int(1)));
        assert_eq!(environment.get("inner"), None);
    }
}
