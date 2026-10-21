use super::*;

impl<'program, 'services> Interpreter<'program, 'services> {
    pub(crate) fn json_encode(&self, value: &Value, span: Span) -> Result<String, RuntimeError> {
        serde_json::to_string(&self.json_value(value, span)?).map_err(|error| {
            self.runtime_error(
                span,
                format!("json_encode failed to serialize value: {error}"),
            )
        })
    }

    pub(crate) fn json_encode_tagged_result(
        &self,
        value: &Value,
        span: Span,
    ) -> Result<String, RuntimeError> {
        let tagged = match value {
            Value::Result(Ok(value)) => serde_json::json!({
                "$result": "ok",
                "value": self.json_value(value, span)?,
            }),
            Value::Result(Err(error)) => serde_json::json!({
                "$result": "err",
                "error": self.json_value(error, span)?,
            }),
            _ => {
                return Err(
                    self.runtime_error(span, "json_encode_tagged_result expects a Result value")
                );
            }
        };
        serde_json::to_string(&tagged).map_err(|error| {
            self.runtime_error(
                span,
                format!("json_encode_tagged_result failed to serialize value: {error}"),
            )
        })
    }

    pub(crate) fn json_decode_tagged_result(
        &self,
        source: &str,
        ok_type: &Type,
        error_type: &Type,
        span: Span,
    ) -> Result<Value, String> {
        let value: serde_json::Value = zelyra_ast::json::parse(source)
            .map_err(|error| format!("invalid tagged Result JSON: {error}"))?;
        let object = value
            .as_object()
            .ok_or_else(|| "tagged Result JSON must be an object".to_owned())?;
        match object.get("$result").and_then(serde_json::Value::as_str) {
            Some("ok") if object.len() == 2 => {
                let payload = object
                    .get("value")
                    .ok_or_else(|| "tagged Ok JSON is missing `value`".to_owned())?;
                let decoded = self
                    .json_decode_value(payload, ok_type, span)
                    .map_err(|error| error.message)?;
                Ok(Value::Result(Ok(Box::new(decoded))))
            }
            Some("err") if object.len() == 2 => {
                let payload = object
                    .get("error")
                    .ok_or_else(|| "tagged Err JSON is missing `error`".to_owned())?;
                let decoded = self
                    .json_decode_value(payload, error_type, span)
                    .map_err(|error| error.message)?;
                Ok(Value::Result(Err(Box::new(decoded))))
            }
            Some("ok") | Some("err") => {
                Err("tagged Result JSON has missing or unexpected fields".to_owned())
            }
            _ => Err("tagged Result JSON requires `$result` to be `ok` or `err`".to_owned()),
        }
    }

    pub(crate) fn json_value(
        &self,
        value: &Value,
        span: Span,
    ) -> Result<serde_json::Value, RuntimeError> {
        match value {
            Value::Json(value) => Ok(value.clone()),
            Value::Int(value) => Ok(serde_json::Value::from(*value)),
            Value::UInt(value) => Ok(serde_json::Value::from(*value)),
            Value::Float(value) => serde_json::Number::from_f64(*value)
                .map(serde_json::Value::Number)
                .ok_or_else(|| {
                    self.runtime_error(span, "json_encode cannot serialize non-finite Float")
                }),
            Value::Decimal(value) => Ok(serde_json::Value::String(format_decimal(*value))),
            Value::Bool(value) => Ok(serde_json::Value::from(*value)),
            Value::String(value) => Ok(serde_json::Value::String(value.clone())),
            Value::Char(value) => Ok(serde_json::Value::String(value.to_string())),
            Value::Enum {
                type_name,
                variant,
                payloads,
                ..
            } => {
                let mut encoded = serde_json::json!({
                    "$enum": type_name,
                    "variant": variant,
                });
                if !payloads.is_empty() {
                    let encoded_payloads = payloads
                        .iter()
                        .map(|payload| self.json_value(payload, span))
                        .collect::<Result<Vec<_>, _>>()?;
                    encoded
                        .as_object_mut()
                        .expect("enum JSON is an object")
                        .insert("payload".into(), serde_json::Value::Array(encoded_payloads));
                }
                Ok(encoded)
            }
            Value::Timestamp(value) => Ok(serde_json::Value::from(*value)),
            Value::Date(value) | Value::Time(value) => Ok(serde_json::Value::String(value.clone())),
            Value::Duration(value) => Ok(serde_json::Value::from(*value)),
            Value::Bytes(values) => Ok(serde_json::Value::Array(
                values
                    .iter()
                    .copied()
                    .map(serde_json::Value::from)
                    .collect(),
            )),
            Value::Array(values) => values
                .iter()
                .map(|value| self.json_value(value, span))
                .collect::<Result<Vec<_>, _>>()
                .map(serde_json::Value::Array),
            Value::Tuple(values) | Value::Set(values) => values
                .iter()
                .map(|value| self.json_value(value, span))
                .collect::<Result<Vec<_>, _>>()
                .map(serde_json::Value::Array),
            Value::Map(entries) => {
                let mut object = serde_json::Map::new();
                for (key, value) in entries {
                    let Value::String(key) = key else {
                        return Err(self.runtime_error(
                            span,
                            "json_encode supports only Map<String, Value> objects",
                        ));
                    };
                    object.insert(key.clone(), self.json_value(value, span)?);
                }
                Ok(serde_json::Value::Object(object))
            }
            Value::Object { fields, .. } => fields
                .iter()
                .map(|(name, value)| {
                    self.json_value(value, span)
                        .map(|value| (name.clone(), value))
                })
                .collect::<Result<serde_json::Map<_, _>, _>>()
                .map(serde_json::Value::Object),
            Value::ClassRef { .. } => Err(self.runtime_error(
                span,
                "class references cannot be serialized directly; convert them to a record",
            )),
            Value::Function(_) => {
                Err(self.runtime_error(span, "json_encode cannot serialize a function value"))
            }
            Value::Option(Some(value)) => self.json_value(value, span),
            Value::Option(None) | Value::Unit => Ok(serde_json::Value::Null),
            Value::Result(Ok(value)) => self.json_value(value, span),
            Value::Result(Err(value)) => {
                let mut object = serde_json::Map::new();
                object.insert("error".into(), self.json_value(value, span)?);
                Ok(serde_json::Value::Object(object))
            }
            Value::Rows { columns, rows, .. } => rows
                .iter()
                .map(|row| {
                    columns
                        .iter()
                        .zip(row)
                        .map(|(column, value)| {
                            Ok((column.clone(), serde_json::Value::String(value.clone())))
                        })
                        .collect::<Result<serde_json::Map<_, _>, RuntimeError>>()
                        .map(serde_json::Value::Object)
                })
                .collect::<Result<Vec<_>, _>>()
                .map(serde_json::Value::Array),
        }
    }

    pub(crate) fn json_decode(
        &self,
        source: &str,
        target: &Type,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let value: serde_json::Value = zelyra_ast::json::parse(source).map_err(|error| {
            self.runtime_error(span, format!("json_decode received invalid JSON: {error}"))
        })?;
        self.json_decode_value(&value, target, span)
    }

    fn json_decode_value(
        &self,
        value: &serde_json::Value,
        target: &Type,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if let Type::Option(inner) = target {
            if value.is_null() {
                return Ok(Value::Option(None));
            }
            return Ok(Value::Option(Some(Box::new(
                self.json_decode_value(value, inner, span)?,
            ))));
        }
        if let Type::Named(name) = target {
            if let Some(alias) = self.program_context.type_aliases.get(name) {
                return self.json_decode_value(value, alias, span);
            }
            if let Some(definition) = self._enum_definition(name) {
                return self.json_decode_enum(value, definition, &[], span);
            }
            if let Some(record) = self.program_context.record_definitions.get(name) {
                if record.is_class {
                    return Err(self.runtime_error(
                        span,
                        "json_decode does not support class types because it cannot restore object identity",
                    ));
                }
                return self.json_decode_record(value, record, &[], span);
            }
            if name == "Money" {
                return value.as_str().and_then(sql_results::parse_money).map(Value::Decimal).ok_or_else(|| self.runtime_error(span, "json_decode expected an exact Money string within DECIMAL(19,4) bounds"));
            }
            if name == "Id" && value.is_number() {
                return value.as_i64().map(Value::Int).ok_or_else(|| {
                    self.runtime_error(
                        span,
                        "json_decode expected an Id integer within the signed 64-bit range",
                    )
                });
            }
            // Preserve the legacy Id string input; materialized SQL Id values
            // additionally round-trip through their native JSON integer form.
            if matches!(name.as_str(), "Id" | "Email" | "Url" | "Uuid") {
                return self.json_decode_string(value, target, span);
            }
        }
        if let Type::Applied { name, args } = target {
            if let Some(definition) = self._enum_definition(name) {
                return self.json_decode_enum(value, definition, args, span);
            }
            if let Some(record) = self.program_context.record_definitions.get(name) {
                if record.is_class {
                    return Err(self.runtime_error(
                        span,
                        "json_decode does not support class types because it cannot restore object identity",
                    ));
                }
                return self.json_decode_record(value, record, args, span);
            }
        }
        match target {
            Type::Json => Ok(Value::Json(value.clone())),
            Type::Int => value
                .as_i64()
                .map(Value::Int)
                .ok_or_else(|| self.json_type_error("Int", value, span)),
            Type::UInt => value
                .as_u64()
                .map(Value::UInt)
                .ok_or_else(|| self.json_type_error("UInt", value, span)),
            Type::Float => value
                .as_f64()
                .map(Value::Float)
                .ok_or_else(|| self.json_type_error("number", value, span)),
            Type::Decimal => {
                let Some(source) = value.as_str() else {
                    return Err(self.json_type_error("Decimal string", value, span));
                };
                parse_decimal(source).map(Value::Decimal).ok_or_else(|| {
                    self.runtime_error(
                        span,
                        "json_decode expected a Decimal with at most 9 fractional digits",
                    )
                })
            }
            Type::Bool => value
                .as_bool()
                .map(Value::Bool)
                .ok_or_else(|| self.json_type_error("Bool", value, span)),
            Type::String => self.json_decode_string(value, target, span),
            Type::Char => {
                let Value::String(string) = self.json_decode_string(value, target, span)? else {
                    return Err(self.runtime_error(span, "json_decode expected a String value"));
                };
                let mut chars = string.chars();
                let Some(character) = chars.next() else {
                    return Err(self.runtime_error(span, "json_decode expected one character"));
                };
                if chars.next().is_some() {
                    return Err(self.runtime_error(span, "json_decode expected one character"));
                }
                Ok(Value::Char(character))
            }
            Type::Timestamp => value
                .as_i64()
                .filter(|value| zelyra_ast::temporal::valid_timestamp_millis(*value))
                .map(Value::Timestamp)
                .ok_or_else(|| self.json_type_error("Timestamp", value, span)),
            Type::Date => {
                let Some(text) = value.as_str() else {
                    return Err(self.json_type_error("Date string", value, span));
                };
                if !valid_date(text) {
                    return Err(
                        self.runtime_error(span, "json_decode expected Date in YYYY-MM-DD format")
                    );
                }
                Ok(Value::Date(text.to_owned()))
            }
            Type::Time => {
                let Some(text) = value.as_str() else {
                    return Err(self.json_type_error("Time string", value, span));
                };
                if !valid_time(text) {
                    return Err(
                        self.runtime_error(span, "json_decode expected Time in HH:MM:SS format")
                    );
                }
                Ok(Value::Time(text.to_owned()))
            }
            Type::Duration => value
                .as_i64()
                .map(Value::Duration)
                .ok_or_else(|| self.json_type_error("Duration milliseconds", value, span)),
            Type::Bytes => {
                let Some(values) = value.as_array() else {
                    return Err(self.json_type_error("byte array", value, span));
                };
                values
                    .iter()
                    .map(|value| {
                        value
                            .as_u64()
                            .and_then(|value| u8::try_from(value).ok())
                            .ok_or_else(|| {
                                self.runtime_error(
                                    span,
                                    "json_decode expected each byte to be an integer from 0 to 255",
                                )
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map(Value::Bytes)
            }
            Type::Array(inner) => {
                let Some(values) = value.as_array() else {
                    return Err(self.json_type_error("array", value, span));
                };
                values
                    .iter()
                    .map(|value| self.json_decode_value(value, inner, span))
                    .collect::<Result<Vec<_>, _>>()
                    .map(Value::Array)
            }
            Type::Tuple(types) => {
                let Some(values) = value.as_array() else {
                    return Err(self.json_type_error("array", value, span));
                };
                if values.len() != types.len() {
                    return Err(self.runtime_error(
                        span,
                        format!(
                            "json_decode expected {} tuple values, found {}",
                            types.len(),
                            values.len()
                        ),
                    ));
                }
                values
                    .iter()
                    .zip(types)
                    .map(|(value, ty)| self.json_decode_value(value, ty, span))
                    .collect::<Result<Vec<_>, _>>()
                    .map(Value::Tuple)
            }
            Type::Set(inner) => {
                let Some(values) = value.as_array() else {
                    return Err(self.json_type_error("array", value, span));
                };
                let values = values
                    .iter()
                    .map(|value| self.json_decode_value(value, inner, span))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::Set(canonical_set(values)))
            }
            Type::Map(key, value_type) => {
                if key.as_ref() != &Type::String {
                    return Err(self.runtime_error(
                        span,
                        "json_decode supports only Map<String, Value> objects",
                    ));
                }
                let Some(object) = value.as_object() else {
                    return Err(self.json_type_error("object", value, span));
                };
                object
                    .iter()
                    .map(|(key, value)| {
                        self.json_decode_value(value, value_type, span)
                            .map(|value| (Value::String(key.clone()), value))
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map(Value::Map)
            }
            Type::Unit if value.is_null() => Ok(Value::Unit),
            Type::Unit => Err(self.json_type_error("null", value, span)),
            Type::Named(name) => Err(self.runtime_error(
                span,
                format!("json_decode does not support target type `{name}`"),
            )),
            Type::Applied { name, .. } => Err(self.runtime_error(
                span,
                format!("json_decode does not support target type `{name}`"),
            )),
            Type::Result(_, _)
            | Type::Unknown
            | Type::Option(_)
            | Type::HttpResult(_)
            | Type::Function { .. } => Err(self.runtime_error(
                span,
                format!("json_decode does not support target type `{target}`"),
            )),
        }
    }

    fn _enum_definition(&self, name: &str) -> Option<&zelyra_ast::EnumDef> {
        // Enum definitions are attached to the source Program; Interpreter keeps
        // their variant names as a compact map to avoid cloning AST nodes.
        self.program_context.enum_definitions.get(name)
    }

    fn json_decode_string(
        &self,
        value: &serde_json::Value,
        target: &Type,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        value
            .as_str()
            .map(|value| Value::String(value.to_owned()))
            .ok_or_else(|| self.json_type_error(&target.to_string(), value, span))
    }

    fn json_decode_record(
        &self,
        value: &serde_json::Value,
        record: &RecordDef,
        type_args: &[Type],
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if record.type_params.len() != type_args.len() {
            return Err(self.runtime_error(
                span,
                format!(
                    "json_decode expects {} type argument(s) for `{}`, found {}",
                    record.type_params.len(),
                    record.name,
                    type_args.len()
                ),
            ));
        }
        let substitutions = record
            .type_params
            .iter()
            .zip(type_args)
            .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
            .collect::<HashMap<_, _>>();
        let Some(object) = value.as_object() else {
            return Err(self.json_type_error(&format!("{} object", record.name), value, span));
        };
        for field in object.keys() {
            if !record
                .fields
                .iter()
                .any(|candidate| candidate.name == *field)
            {
                return Err(self.runtime_error(
                    span,
                    format!(
                        "json_decode found unknown field `{field}` in `{}`",
                        record.name
                    ),
                ));
            }
        }
        let mut fields = HashMap::new();
        for field in &record.fields {
            let Some(value) = object.get(&field.name) else {
                if matches!(field.ty, Type::Option(_)) {
                    fields.insert(field.name.clone(), Value::Option(None));
                    continue;
                }
                return Err(self.runtime_error(
                    span,
                    format!(
                        "json_decode is missing field `{}` in `{}`",
                        field.name, record.name
                    ),
                ));
            };
            let field_type = substitute_generic_type(&field.ty, &substitutions);
            fields.insert(
                field.name.clone(),
                self.json_decode_value(value, &field_type, span)?,
            );
        }
        Ok(Value::Object {
            type_name: record.name.clone(),
            type_args: type_args.to_vec(),
            fields,
        })
    }

    fn json_decode_enum(
        &self,
        value: &serde_json::Value,
        definition: &zelyra_ast::EnumDef,
        type_args: &[Type],
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if definition.type_params.len() != type_args.len() {
            return Err(self.runtime_error(
                span,
                format!(
                    "json_decode expects {} type argument(s) for `{}`, found {}",
                    definition.type_params.len(),
                    definition.name,
                    type_args.len()
                ),
            ));
        }
        let Some(object) = value.as_object() else {
            return Err(self.json_type_error("enum object", value, span));
        };
        if object.get("$enum").and_then(serde_json::Value::as_str) != Some(definition.name.as_str())
        {
            return Err(self.runtime_error(
                span,
                format!("json_decode expected enum `{}`", definition.name),
            ));
        }
        let Some(variant) = object.get("variant").and_then(serde_json::Value::as_str) else {
            return Err(self.runtime_error(span, "json_decode expected enum variant String"));
        };
        if !definition.variants.iter().any(|item| item == variant) {
            return Err(self.runtime_error(
                span,
                format!("unknown `{}` variant `{variant}`", definition.name),
            ));
        }
        let payload_types = definition
            .variant_payloads
            .iter()
            .find(|(candidate, _)| candidate == variant)
            .map(|(_, types)| types.as_slice())
            .unwrap_or_default();
        let payloads = if payload_types.is_empty() {
            if object.len() != 2 || object.contains_key("payload") {
                return Err(self.runtime_error(
                    span,
                    format!(
                        "unit enum `{}` variant `{variant}` must not have a payload",
                        definition.name
                    ),
                ));
            }
            Vec::new()
        } else {
            if object.len() != 3 {
                return Err(self.runtime_error(
                    span,
                    format!(
                        "enum `{}` variant `{variant}` requires a payload array",
                        definition.name
                    ),
                ));
            }
            let Some(values) = object.get("payload").and_then(serde_json::Value::as_array) else {
                return Err(self.runtime_error(
                    span,
                    format!(
                        "enum `{}` variant `{variant}` requires a payload array",
                        definition.name
                    ),
                ));
            };
            if values.len() != payload_types.len() {
                return Err(self.runtime_error(
                    span,
                    format!(
                        "enum `{}` variant `{variant}` expects {} payload value(s), found {}",
                        definition.name,
                        payload_types.len(),
                        values.len()
                    ),
                ));
            }
            let substitutions = definition
                .type_params
                .iter()
                .zip(type_args)
                .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                .collect::<HashMap<_, _>>();
            values
                .iter()
                .zip(payload_types)
                .map(|(value, ty)| {
                    let ty = substitute_generic_type(ty, &substitutions);
                    self.json_decode_value(value, &ty, span)
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(Value::Enum {
            type_name: definition.name.clone(),
            type_args: type_args.to_vec(),
            variant: variant.to_owned(),
            payloads,
        })
    }

    fn json_type_error(
        &self,
        expected: &str,
        value: &serde_json::Value,
        span: Span,
    ) -> RuntimeError {
        self.runtime_error(
            span,
            format!("json_decode expected {expected}, found JSON value `{value}`"),
        )
    }
}
