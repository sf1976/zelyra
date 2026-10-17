//! Redacted, source-scoped package settings passed into runtime execution.

use crate::Value;
use std::collections::BTreeMap;
use zelyra_ast::Type;
use zeroize::Zeroizing;

/// JSON scalar values keyed by the importing package source and setting name.
/// Values are kept as JSON so the interpreter can apply the same type-aware
/// decoder used by `json_decode<T>`.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct PackageSettings {
    values: BTreeMap<(u32, String), Zeroizing<String>>,
}

impl PackageSettings {
    /// Insert one serialized scalar setting for a source file.
    pub fn insert_json(
        &mut self,
        source_id: u32,
        name: impl Into<String>,
        serialized_value: String,
    ) -> Result<(), String> {
        let name = name.into();
        if name.is_empty() || name.len() > 64 || serialized_value.len() > 8192 {
            return Err("package setting exceeds the supported key or value size".into());
        }
        let value: serde_json::Value = serde_json::from_str(&serialized_value)
            .map_err(|_| "package setting is not valid JSON".to_owned())?;
        if !(value.is_string() || value.is_boolean() || value.is_i64() || value.is_f64()) {
            return Err("package setting must be a scalar value".into());
        }
        if value.as_f64().is_some_and(|number| !number.is_finite()) {
            return Err("package setting float must be finite".into());
        }
        match self.values.entry((source_id, name)) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(Zeroizing::new(serialized_value));
            }
            std::collections::btree_map::Entry::Occupied(_) => {
                return Err("package setting was supplied more than once".into());
            }
        }
        Ok(())
    }

    pub(crate) fn get_json(&self, source_id: u32, name: &str) -> Option<&str> {
        self.values
            .get(&(source_id, name.to_owned()))
            .map(|value| value.as_str())
    }

    /// Read one source-scoped scalar setting as its compiler-declared type.
    /// The serialized secret is never included in errors.
    pub fn get_value(&self, source_id: u32, name: &str, expected: &Type) -> Result<Value, String> {
        let serialized = self
            .get_json(source_id, name)
            .ok_or_else(|| format!("package setting `{name}` is unavailable for this source"))?;
        let value: serde_json::Value = serde_json::from_str(serialized)
            .map_err(|_| "package setting contains invalid serialized data".to_owned())?;
        match expected {
            Type::String => match value {
                serde_json::Value::String(value) => Ok(Value::String(value)),
                _ => Err(format!(
                    "package setting `{name}` does not match type `String`"
                )),
            },
            Type::Bool => value
                .as_bool()
                .map(Value::Bool)
                .ok_or_else(|| format!("package setting `{name}` does not match type `Bool`")),
            Type::Int => value
                .as_i64()
                .map(Value::Int)
                .ok_or_else(|| format!("package setting `{name}` does not match type `Int`")),
            Type::Float => value
                .as_f64()
                .filter(|value| value.is_finite())
                .map(Value::Float)
                .ok_or_else(|| format!("package setting `{name}` does not match type `Float`")),
            _ => Err(format!(
                "package setting `{name}` has unsupported requested type `{expected}`"
            )),
        }
    }

    /// Return the number of source-scoped settings without revealing keys.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl std::fmt::Debug for PackageSettings {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PackageSettings")
            .field("entry_count", &self.values.len())
            .field("values", &"<redacted>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_source_scoped_and_debug_redacted() {
        let secret = "do-not-print-this-secret";
        let mut settings = PackageSettings::default();
        settings
            .insert_json(3, "api_token", serde_json::to_string(secret).unwrap())
            .unwrap();
        assert_eq!(
            settings.get_json(3, "api_token"),
            Some("\"do-not-print-this-secret\"")
        );
        assert_eq!(settings.get_json(4, "api_token"), None);
        assert!(!format!("{settings:?}").contains(secret));
    }

    #[test]
    fn rejects_compound_values_and_duplicate_entries() {
        let mut settings = PackageSettings::default();
        assert!(settings.insert_json(0, "value", "[]".into()).is_err());
        settings.insert_json(0, "value", "1".into()).unwrap();
        assert!(settings.insert_json(0, "value", "2".into()).is_err());
        assert_eq!(settings.get_json(0, "value"), Some("1"));
    }

    #[test]
    fn typed_read_checks_schema_type_and_source_scope() {
        let mut settings = PackageSettings::default();
        settings
            .insert_json(7, "timeout", "30".into())
            .expect("valid integer setting");
        assert_eq!(
            settings.get_value(7, "timeout", &Type::Int),
            Ok(Value::Int(30))
        );
        assert!(settings
            .get_value(7, "timeout", &Type::String)
            .expect_err("wrong declared type")
            .contains("does not match type `String`"));
        assert!(settings
            .get_value(8, "timeout", &Type::Int)
            .expect_err("other source cannot read this setting")
            .contains("unavailable"));
    }

    #[test]
    fn typed_read_supports_each_declared_scalar_setting_type() {
        let mut settings = PackageSettings::default();
        settings
            .insert_json(0, "label", serde_json::to_string("invoice").unwrap())
            .unwrap();
        settings.insert_json(0, "enabled", "true".into()).unwrap();
        settings.insert_json(0, "ratio", "1.25".into()).unwrap();
        assert_eq!(
            settings.get_value(0, "label", &Type::String),
            Ok(Value::String("invoice".into()))
        );
        assert_eq!(
            settings.get_value(0, "enabled", &Type::Bool),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            settings.get_value(0, "ratio", &Type::Float),
            Ok(Value::Float(1.25))
        );
    }
}
