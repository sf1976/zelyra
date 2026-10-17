//! Shared, deterministic standard-library operations used by AST and MIR runtimes.

use super::*;

fn builtin_error(span: Span, message: impl Into<String>) -> RuntimeError {
    RuntimeError {
        message: message.into(),
        span,
    }
}

pub fn is_pure_builtin(name: &str) -> bool {
    matches!(
        name,
        "Some"
            | "None"
            | "Ok"
            | "Err"
            | "len"
            | "csv_decode_result"
            | "csv_encode"
            | "decimal_scaled_result"
            | "format_scaled_result"
            | "decimal_quantize_result"
            | "append"
            | "array_replace_result"
            | "entries"
            | "get"
            | "contains"
            | "get_or"
            | "get_or_default"
            | "has_key"
            | "keys"
            | "values"
            | "map_remove"
            | "map_merge"
            | "map_increment"
            | "array_sum"
            | "avg"
            | "min_of"
            | "max_of"
            | "set"
            | "set_add"
            | "set_remove"
            | "set_union"
            | "set_intersection"
            | "set_difference"
            | "range"
            | "to_string"
            | "to_int"
            | "to_float"
            | "abs"
            | "min"
            | "max"
            | "clamp"
            | "sqrt"
            | "pow"
            | "floor"
            | "ceil"
            | "round"
            | "decimal"
            | "text_contains"
            | "text_trim"
            | "text_chars"
            | "text_to_lower"
            | "text_to_upper"
            | "text_ucfirst"
            | "text_starts_with"
            | "text_ends_with"
            | "text_replace"
            | "text_split"
            | "split"
            | "text_pad_left"
            | "text_pad_right"
            | "parse_bool"
            | "text_replace_result"
            | "text_split_result"
            | "text_trim_start"
            | "text_trim_end"
            | "text_is_blank"
            | "text_substring"
            | "text_index_of"
            | "text_repeat"
            | "text_is_empty"
            | "text_capitalize"
            | "parse_int"
            | "parse_float"
            | "sort"
            | "sort_desc"
            | "reverse"
            | "slice"
            | "unique"
            | "chunk"
            | "flatten"
            | "concat"
            | "join"
            | "zip"
            | "dedup"
            | "date_add_days"
            | "date_diff_days"
            | "date_add_days_result"
            | "date_days_between_result"
            | "hash_sha256"
            | "base64_encode"
            | "base64_decode"
            | "url_encode"
            | "url_decode"
            | "is_ok"
            | "is_err"
            | "is_some"
            | "is_none"
            | "is_before"
            | "is_after"
    )
}

/// PHP 8.2+ `ucfirst` compatibility: uppercase an initial ASCII `a` through
/// `z` byte only. Non-ASCII leading characters and the remainder are unchanged.
pub(crate) fn text_ucfirst(text: &str) -> String {
    let Some(first) = text.as_bytes().first().copied() else {
        return String::new();
    };
    if !first.is_ascii_lowercase() {
        return text.to_owned();
    }
    let mut result = text.to_owned();
    result.replace_range(0..1, &char::from(first.to_ascii_uppercase()).to_string());
    result
}

pub fn execute_pure_builtin(
    name: &str,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, RuntimeError> {
    match name {
        "None" if arguments.is_empty() => Ok(Value::Option(None)),
        "Some" if arguments.len() == 1 => Ok(Value::Option(Some(Box::new(arguments[0].clone())))),
        "Ok" if arguments.len() == 1 => Ok(Value::Result(Ok(Box::new(arguments[0].clone())))),
        "Err" if arguments.len() == 1 => Ok(Value::Result(Err(Box::new(arguments[0].clone())))),
        "is_ok" | "is_err" if arguments.len() == 1 => match &arguments[0] {
            Value::Result(result) => Ok(Value::Bool(if name == "is_ok" {
                result.is_ok()
            } else {
                result.is_err()
            })),
            other => Err(builtin_error(
                span,
                format!("{name} expects Result, found {}", other.ty()),
            )),
        },
        "is_some" | "is_none" if arguments.len() == 1 => match &arguments[0] {
            Value::Option(option) => Ok(Value::Bool(if name == "is_some" {
                option.is_some()
            } else {
                option.is_none()
            })),
            other => Err(builtin_error(
                span,
                format!("{name} expects Option, found {}", other.ty()),
            )),
        },
        "is_before" | "is_after" if arguments.len() == 2 => match arguments.as_slice() {
            [Value::Date(left), Value::Date(right)] => {
                let left = zelyra_ast::temporal::date_epoch_days(left)
                    .ok_or_else(|| builtin_error(span, "invalid Date"))?;
                let right = zelyra_ast::temporal::date_epoch_days(right)
                    .ok_or_else(|| builtin_error(span, "invalid Date"))?;
                Ok(Value::Bool(if name == "is_before" {
                    left < right
                } else {
                    left > right
                }))
            }
            _ => Err(builtin_error(span, format!("{name} expects two Dates"))),
        },
        "date_add_days" if arguments.len() == 2 => match arguments.as_slice() {
            [Value::Date(date), Value::Int(days)] => zelyra_ast::temporal::date_epoch_days(date)
                .and_then(|start| start.checked_add(*days))
                .and_then(zelyra_ast::temporal::epoch_days_date)
                .map(Value::Date)
                .ok_or_else(|| {
                    builtin_error(span, "date arithmetic is outside years 0001 through 9999")
                }),
            _ => Err(builtin_error(span, "date_add_days expects Date and Int")),
        },
        "date_diff_days" if arguments.len() == 2 => match arguments.as_slice() {
            [Value::Date(start), Value::Date(end)] => zelyra_ast::temporal::date_epoch_days(end)
                .and_then(|end| {
                    zelyra_ast::temporal::date_epoch_days(start)
                        .and_then(|start| start.checked_sub(end))
                })
                .map(Value::Int)
                .ok_or_else(|| builtin_error(span, "date difference overflow")),
            _ => Err(builtin_error(span, "date_diff_days expects two Dates")),
        },
        "date_add_days_result" | "date_days_between_result" if arguments.len() == 2 => {
            let result = if name == "date_add_days_result" {
                match arguments.as_slice() {
                    [Value::Date(date), Value::Int(days)] => {
                        zelyra_ast::temporal::date_epoch_days(date)
                            .and_then(|start| start.checked_add(*days))
                            .and_then(zelyra_ast::temporal::epoch_days_date)
                            .map(Value::Date)
                            .ok_or_else(|| {
                                "date arithmetic is outside years 0001 through 9999".to_owned()
                            })
                    }
                    _ => Err("date_add_days_result expects Date and Int".to_owned()),
                }
            } else {
                match arguments.as_slice() {
                    [Value::Date(start), Value::Date(end)] => {
                        zelyra_ast::temporal::date_epoch_days(end)
                            .and_then(|end| {
                                zelyra_ast::temporal::date_epoch_days(start)
                                    .and_then(|start| end.checked_sub(start))
                            })
                            .map(Value::Int)
                            .ok_or_else(|| "date difference overflow".to_owned())
                    }
                    _ => Err("date_days_between_result expects two Dates".to_owned()),
                }
            };
            Ok(Value::Result(
                result
                    .map(Box::new)
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        }
        "hash_sha256" if arguments.len() == 1 => match &arguments[0] {
            Value::String(value) => {
                let mut hasher = Sha256::new();
                hasher.update(value.as_bytes());
                Ok(Value::String(format!("{:x}", hasher.finalize())))
            }
            _ => Err(builtin_error(span, "hash_sha256 expects a String")),
        },
        "base64_encode" if arguments.len() == 1 => match &arguments[0] {
            Value::String(value) => Ok(Value::String(base64_encode_bytes(value.as_bytes()))),
            _ => Err(builtin_error(span, "base64_encode expects a String")),
        },
        "base64_decode" if arguments.len() == 1 => match &arguments[0] {
            Value::String(value) => Ok(Value::Result(
                base64_decode_str(value)
                    .map(|decoded| Box::new(Value::String(decoded)))
                    .map_err(|error| Box::new(Value::String(error))),
            )),
            _ => Err(builtin_error(span, "base64_decode expects a String")),
        },
        "url_encode" if arguments.len() == 1 => match &arguments[0] {
            Value::String(value) => Ok(Value::String(url_encode_str(value))),
            _ => Err(builtin_error(span, "url_encode expects a String")),
        },
        "url_decode" if arguments.len() == 1 => match &arguments[0] {
            Value::String(value) => Ok(Value::Result(
                url_decode_str(value)
                    .map(|decoded| Box::new(Value::String(decoded)))
                    .map_err(|error| Box::new(Value::String(error))),
            )),
            _ => Err(builtin_error(span, "url_decode expects a String")),
        },
        "text_pad_left" | "text_pad_right" if arguments.len() == 3 => match arguments.as_slice() {
            [Value::String(text), Value::Int(target_len), Value::String(pad)] => {
                pad_text_to_codepoints(text, *target_len, pad, name == "text_pad_left")
                    .map(Value::String)
                    .map_err(|message| builtin_error(span, message))
            }
            _ => Err(builtin_error(
                span,
                format!("{name} expects String, Int, and String"),
            )),
        },
        "array_replace_result" => {
            if arguments.len() != 3 {
                return Err(builtin_error(
                    span,
                    "array_replace_result expects an array, Int index, and value",
                ));
            }
            let [Value::Array(values), Value::Int(index), replacement] = arguments.as_slice()
            else {
                return Err(builtin_error(
                    span,
                    "array_replace_result expects an array and Int index",
                ));
            };
            let result = match usize::try_from(*index) {
                Ok(index) => {
                    let mut updated = values.clone();
                    if let Some(slot) = updated.get_mut(index) {
                        *slot = replacement.clone();
                        Ok(Value::Array(updated))
                    } else {
                        Err("array_replace_result index is out of bounds".to_owned())
                    }
                }
                Err(_) => Err("array_replace_result index is out of bounds".to_owned()),
            };
            Ok(Value::Result(
                result
                    .map(Box::new)
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        }
        "entries" => {
            if arguments.len() != 1 {
                return Err(builtin_error(span, "entries expects a Map"));
            }
            match &arguments[0] {
                Value::Map(entries) => Ok(Value::Array(
                    entries
                        .iter()
                        .map(|(key, value)| Value::Tuple(vec![key.clone(), value.clone()]))
                        .collect(),
                )),
                _ => Err(builtin_error(span, "entries expects a Map")),
            }
        }
        "zip" => {
            if arguments.len() != 2 {
                return Err(builtin_error(span, "zip expects two Arrays"));
            }
            match arguments.as_slice() {
                [Value::Array(left), Value::Array(right)] => Ok(Value::Array(
                    left.iter()
                        .zip(right)
                        .map(|(left, right)| Value::Tuple(vec![left.clone(), right.clone()]))
                        .collect(),
                )),
                _ => Err(builtin_error(span, "zip expects two Arrays")),
            }
        }
        "dedup" => {
            if arguments.len() != 1 {
                return Err(builtin_error(span, "dedup expects an Array"));
            }
            match &arguments[0] {
                Value::Array(values) => {
                    let mut unique = Vec::new();
                    for value in values {
                        if !unique.contains(value) {
                            unique.push(value.clone());
                        }
                    }
                    Ok(Value::Array(unique))
                }
                _ => Err(builtin_error(span, "dedup expects an Array")),
            }
        }
        "parse_bool" => {
            if arguments.len() != 1 {
                return Err(builtin_error(span, "parse_bool expects one String"));
            }
            match &arguments[0] {
                Value::String(text) => {
                    let parsed = match text.trim().to_ascii_lowercase().as_str() {
                        "true" | "1" => Ok(Value::Bool(true)),
                        "false" | "0" => Ok(Value::Bool(false)),
                        _ => Err(Value::String(format!("invalid boolean string: {text}"))),
                    };
                    Ok(Value::Result(parsed.map(Box::new).map_err(Box::new)))
                }
                _ => Err(builtin_error(span, "parse_bool expects a String")),
            }
        }
        "text_replace_result" => {
            if arguments.len() != 3 {
                return Err(builtin_error(
                    span,
                    "text_replace_result expects text, search, and replacement",
                ));
            }
            let [Value::String(text), Value::String(search), Value::String(replacement)] =
                arguments.as_slice()
            else {
                return Ok(Value::Result(Err(Box::new(Value::String(
                    "text_replace_result expects String arguments".into(),
                )))));
            };
            let result = if search.is_empty() {
                Err(Value::String(
                    "text_replace_result requires a non-empty search text".into(),
                ))
            } else {
                Ok(Value::String(text.replace(search, replacement)))
            };
            Ok(Value::Result(result.map(Box::new).map_err(Box::new)))
        }
        "text_split_result" => {
            if arguments.len() != 2 {
                return Err(builtin_error(
                    span,
                    "text_split_result expects text and separator",
                ));
            }
            let [Value::String(text), Value::String(separator)] = arguments.as_slice() else {
                return Ok(Value::Result(Err(Box::new(Value::String(
                    "text_split_result expects String arguments".into(),
                )))));
            };
            let result = if separator.is_empty() {
                Err(Value::String(
                    "text_split_result requires a non-empty separator".into(),
                ))
            } else {
                Ok(Value::Array(
                    text.split(separator)
                        .map(|part| Value::String(part.to_owned()))
                        .collect(),
                ))
            };
            Ok(Value::Result(result.map(Box::new).map_err(Box::new)))
        }
        "text_trim_start" | "text_trim_end" | "text_is_blank" => {
            if arguments.len() != 1 {
                return Err(builtin_error(span, format!("{name} expects one String")));
            }
            let Value::String(text) = &arguments[0] else {
                return Err(builtin_error(span, format!("{name} expects a String")));
            };
            match name {
                "text_trim_start" => Ok(Value::String(text.trim_start().to_owned())),
                "text_trim_end" => Ok(Value::String(text.trim_end().to_owned())),
                _ => Ok(Value::Bool(text.trim().is_empty())),
            }
        }
        "text_substring" => {
            if arguments.len() != 3 {
                return Err(builtin_error(
                    span,
                    "text_substring expects String, start, and length",
                ));
            }
            let [Value::String(text), Value::Int(start), Value::Int(length)] = arguments.as_slice()
            else {
                return Err(builtin_error(
                    span,
                    "text_substring expects String, Int, and Int",
                ));
            };
            let characters = text.chars().collect::<Vec<_>>();
            let start = usize::try_from((*start).max(0))
                .unwrap_or(usize::MAX)
                .min(characters.len());
            let length = usize::try_from((*length).max(0))
                .unwrap_or(usize::MAX)
                .min(characters.len() - start);
            Ok(Value::String(
                characters[start..start + length].iter().collect(),
            ))
        }
        "text_index_of" => {
            if arguments.len() != 2 {
                return Err(builtin_error(
                    span,
                    "text_index_of expects text and substring",
                ));
            }
            let [Value::String(text), Value::String(needle)] = arguments.as_slice() else {
                return Err(builtin_error(span, "text_index_of expects two Strings"));
            };
            Ok(Value::Option(text.find(needle).map(|byte_pos| {
                Box::new(Value::Int(text[..byte_pos].chars().count() as i64))
            })))
        }
        "text_repeat" => {
            if arguments.len() != 2 {
                return Err(builtin_error(span, "text_repeat expects text and count"));
            }
            let [Value::String(text), Value::Int(count)] = arguments.as_slice() else {
                return Err(builtin_error(span, "text_repeat expects String and Int"));
            };
            if *count < 0 {
                return Err(builtin_error(
                    span,
                    "text_repeat count must be non-negative",
                ));
            }
            if text.is_empty() || *count == 0 {
                return Ok(Value::String(String::new()));
            }
            let count = usize::try_from(*count).map_err(|_| {
                builtin_error(span, "text_repeat output exceeds the 16 MiB text limit")
            })?;
            let output_bytes = text.len().checked_mul(count);
            if output_bytes.is_none_or(|bytes| bytes > MAX_TEXT_BUILTIN_OUTPUT_BYTES) {
                return Err(builtin_error(
                    span,
                    "text_repeat output exceeds the 16 MiB text limit",
                ));
            }
            Ok(Value::String(text.repeat(count)))
        }
        "text_is_empty" => {
            if arguments.len() != 1 {
                return Err(builtin_error(span, "text_is_empty expects one String"));
            }
            match &arguments[0] {
                Value::String(text) => Ok(Value::Bool(text.is_empty())),
                other => Err(builtin_error(
                    span,
                    format!("text_is_empty expects String, found {}", other.ty()),
                )),
            }
        }
        "text_capitalize" => {
            if arguments.len() != 1 {
                return Err(builtin_error(span, "text_capitalize expects one String"));
            }
            match &arguments[0] {
                Value::String(text) => {
                    let mut chars = text.chars();
                    let capitalized = match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    };
                    Ok(Value::String(capitalized))
                }
                other => Err(builtin_error(
                    span,
                    format!("text_capitalize expects String, found {}", other.ty()),
                )),
            }
        }
        "len" if arguments.len() == 1 => {
            let length = match &arguments[0] {
                Value::Array(values) | Value::Tuple(values) | Value::Set(values) => values.len(),
                Value::Map(entries) => entries.len(),
                Value::String(value) => value.chars().count(),
                Value::Bytes(value) => value.len(),
                other => {
                    return Err(builtin_error(
                        span,
                        format!("`len` does not support `{}`", other.ty()),
                    ));
                }
            };
            i64::try_from(length)
                .map(Value::Int)
                .map_err(|_| builtin_error(span, "collection length exceeds Int range"))
        }
        "csv_decode_result" if arguments.len() == 2 => {
            let [Value::String(source), Value::Char(delimiter)] = arguments.as_slice() else {
                return Err(builtin_error(
                    span,
                    "csv_decode_result expects a String and Char",
                ));
            };
            let decoded = super::csv::decode(source, *delimiter).map(|rows| {
                Value::Array(
                    rows.into_iter()
                        .map(|row| {
                            Value::Array(row.into_iter().map(Value::String).collect::<Vec<_>>())
                        })
                        .collect::<Vec<_>>(),
                )
            });
            Ok(Value::Result(
                decoded
                    .map(Box::new)
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        }
        "csv_encode" if arguments.len() == 2 => {
            let [Value::Array(rows), Value::Char(delimiter)] = arguments.as_slice() else {
                return Err(builtin_error(
                    span,
                    "csv_encode expects String[][] rows and a Char",
                ));
            };
            let rows = rows
                .iter()
                .map(|row| match row {
                    Value::Array(fields) => fields
                        .iter()
                        .map(|field| match field {
                            Value::String(value) => Ok(value.clone()),
                            _ => Err("csv_encode expects String[][] rows".to_owned()),
                        })
                        .collect::<Result<Vec<_>, _>>(),
                    _ => Err("csv_encode expects String[][] rows".to_owned()),
                })
                .collect::<Result<Vec<_>, _>>();
            let encoded = rows
                .and_then(|rows| super::csv::encode(&rows, *delimiter))
                .map(Value::String);
            Ok(Value::Result(
                encoded
                    .map(Box::new)
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        }
        "decimal_scaled_result" if arguments.len() == 2 => {
            let [Value::String(source), Value::Int(scale)] = arguments.as_slice() else {
                return Err(builtin_error(
                    span,
                    "decimal_scaled_result expects a String and scale Int",
                ));
            };
            Ok(Value::Result(
                super::csv::decimal_to_scaled(source, *scale)
                    .map(|value| Box::new(Value::Int(value)))
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        }
        "format_scaled_result" if arguments.len() == 2 => {
            let [Value::Int(value), Value::Int(scale)] = arguments.as_slice() else {
                return Err(builtin_error(
                    span,
                    "format_scaled_result expects Int arguments",
                ));
            };
            Ok(Value::Result(
                super::csv::format_scaled(*value, *scale)
                    .map(|text| Box::new(Value::String(text)))
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        }
        "decimal_quantize_result" if arguments.len() == 3 => {
            let [Value::Decimal(value), Value::Int(scale), Value::String(mode)] =
                arguments.as_slice()
            else {
                return Ok(Value::Result(Err(Box::new(Value::String(
                    "decimal_quantize_result expects Decimal, Int, and String".into(),
                )))));
            };
            let result = u32::try_from(*scale)
                .map_err(|_| "decimal scale must be between 0 and 9".to_owned())
                .and_then(|scale| super::quantize_decimal(*value, scale, mode));
            Ok(Value::Result(
                result
                    .map(|value| Box::new(Value::Decimal(value)))
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        }
        "append" if arguments.len() == 2 => {
            let mut arguments = arguments.into_iter();
            match (arguments.next(), arguments.next()) {
                (Some(Value::Array(mut items)), Some(value)) => {
                    items.push(value);
                    Ok(Value::Array(items))
                }
                (Some(other), Some(_)) => Err(builtin_error(
                    span,
                    format!("`append` expects Array, got `{}`", other.ty()),
                )),
                _ => unreachable!("arity checked by match guard"),
            }
        }
        "contains" if arguments.len() == 2 => {
            let [collection, searched] = arguments.as_slice() else {
                unreachable!("arity checked by match guard");
            };
            match collection {
                Value::Array(values) | Value::Set(values) => {
                    Ok(Value::Bool(values.contains(searched)))
                }
                Value::Map(entries) => {
                    Ok(Value::Bool(entries.iter().any(|(key, _)| key == searched)))
                }
                other => Err(builtin_error(
                    span,
                    format!("`contains` does not support `{}`", other.ty()),
                )),
            }
        }
        "get" if arguments.len() == 2 => {
            let [Value::Map(entries), key] = arguments.as_slice() else {
                return Err(builtin_error(span, "`get` expects a Map and one key"));
            };
            Ok(Value::Option(
                entries
                    .iter()
                    .find(|(existing, _)| existing == key)
                    .map(|(_, value)| Box::new(value.clone())),
            ))
        }
        "map_remove" if arguments.len() == 2 => match (&arguments[0], &arguments[1]) {
            (Value::Map(entries), key) => Ok(Value::Map(
                entries
                    .iter()
                    .filter(|(entry_key, _)| entry_key != key)
                    .cloned()
                    .collect(),
            )),
            (other, _) => Err(builtin_error(
                span,
                format!("map_remove expects a Map, found {}", other.ty()),
            )),
        },
        "get_or" | "get_or_default" if arguments.len() == 3 => {
            match (&arguments[0], &arguments[1], &arguments[2]) {
                (Value::Map(entries), key, fallback) => Ok(entries
                    .iter()
                    .find(|(entry_key, _)| entry_key == key)
                    .map(|(_, value)| value.clone())
                    .unwrap_or_else(|| fallback.clone())),
                (other, _, _) => Err(builtin_error(
                    span,
                    format!("{name} expects a Map, found {}", other.ty()),
                )),
            }
        }
        "has_key" if arguments.len() == 2 => match (&arguments[0], &arguments[1]) {
            (Value::Map(entries), key) => Ok(Value::Bool(
                entries.iter().any(|(entry_key, _)| entry_key == key),
            )),
            (other, _) => Err(builtin_error(
                span,
                format!("has_key expects a Map, found {}", other.ty()),
            )),
        },
        "keys" | "values" if arguments.len() == 1 => match &arguments[0] {
            Value::Map(entries) if name == "keys" => Ok(Value::Array(
                entries.iter().map(|(key, _)| key.clone()).collect(),
            )),
            Value::Map(entries) => Ok(Value::Array(
                entries.iter().map(|(_, value)| value.clone()).collect(),
            )),
            other => Err(builtin_error(
                span,
                format!("`{name}` expects a Map, found {}", other.ty()),
            )),
        },
        "map_merge" if arguments.len() == 2 => match arguments.as_slice() {
            [Value::Map(left), Value::Map(right)] => {
                let mut merged = left.clone();
                for (right_key, right_value) in right {
                    if let Some(position) = merged
                        .iter()
                        .position(|(left_key, _)| left_key == right_key)
                    {
                        merged[position] = (right_key.clone(), right_value.clone());
                    } else {
                        merged.push((right_key.clone(), right_value.clone()));
                    }
                }
                Ok(Value::Map(merged))
            }
            _ => Err(builtin_error(span, "map_merge expects two Maps")),
        },
        "map_increment" if arguments.len() == 2 => match arguments.as_slice() {
            [Value::Map(existing), key] => {
                let mut entries = existing.clone();
                let current = entries.iter().position(|(existing, _)| existing == key);
                let next = match current {
                    Some(index) => match &entries[index].1 {
                        Value::Int(value) => match value.checked_add(1) {
                            Some(next) => next,
                            None => {
                                return Ok(Value::Result(Err(Box::new(Value::String(
                                    "map counter overflow".into(),
                                )))));
                            }
                        },
                        other => {
                            return Err(builtin_error(
                                span,
                                format!("map_increment expects Int values, found {}", other.ty()),
                            ));
                        }
                    },
                    None => 1,
                };
                if let Some(index) = current {
                    entries[index].1 = Value::Int(next);
                } else {
                    entries.push((key.clone(), Value::Int(next)));
                }
                Ok(Value::Result(Ok(Box::new(Value::Map(entries)))))
            }
            _ => Err(builtin_error(span, "map_increment expects a Map and key")),
        },
        "array_sum" if arguments.len() == 1 => match &arguments[0] {
            Value::Array(items) if items.is_empty() => Ok(Value::Int(0)),
            Value::Array(items) => match &items[0] {
                Value::Int(_) => {
                    let mut sum = 0_i64;
                    for item in items {
                        if let Value::Int(value) = item {
                            sum = sum
                                .checked_add(*value)
                                .ok_or_else(|| builtin_error(span, "sum integer overflow"))?;
                        }
                    }
                    Ok(Value::Int(sum))
                }
                Value::Float(_) => {
                    let mut sum = 0.0_f64;
                    for item in items {
                        if let Value::Float(value) = item {
                            if !value.is_finite() {
                                return Err(builtin_error(
                                    span,
                                    "array_sum requires finite Float elements",
                                ));
                            }
                            sum += value;
                            if !sum.is_finite() {
                                return Err(builtin_error(span, "array_sum Float overflow"));
                            }
                        }
                    }
                    Ok(Value::Float(sum))
                }
                Value::Decimal(_) => {
                    let mut sum = 0_i128;
                    for item in items {
                        if let Value::Decimal(value) = item {
                            sum = sum
                                .checked_add(*value)
                                .ok_or_else(|| builtin_error(span, "sum decimal overflow"))?;
                        }
                    }
                    Ok(Value::Decimal(sum))
                }
                other => Err(builtin_error(
                    span,
                    format!("sum expects numbers, found {}", other.ty()),
                )),
            },
            other => Err(builtin_error(
                span,
                format!("sum expects array, found {}", other.ty()),
            )),
        },
        "avg" if arguments.len() == 1 => match &arguments[0] {
            Value::Array(items) if items.is_empty() => {
                Err(builtin_error(span, "avg of empty array is undefined"))
            }
            Value::Array(items) => {
                let count = items.len() as f64;
                match &items[0] {
                    Value::Int(_) => {
                        let mut sum = 0.0_f64;
                        for item in items {
                            if let Value::Int(value) = item {
                                sum += *value as f64;
                            }
                        }
                        Ok(Value::Float(sum / count))
                    }
                    Value::Float(_) => {
                        let mut sum = 0.0_f64;
                        for item in items {
                            if let Value::Float(value) = item {
                                if !value.is_finite() {
                                    return Err(builtin_error(
                                        span,
                                        "avg requires finite Float elements",
                                    ));
                                }
                                sum += value;
                                if !sum.is_finite() {
                                    return Err(builtin_error(span, "avg Float overflow"));
                                }
                            }
                        }
                        Ok(Value::Float(sum / count))
                    }
                    Value::Decimal(_) => {
                        let mut sum = 0_i128;
                        for item in items {
                            if let Value::Decimal(value) = item {
                                sum = sum
                                    .checked_add(*value)
                                    .ok_or_else(|| builtin_error(span, "avg decimal overflow"))?;
                            }
                        }
                        Ok(Value::Decimal(sum / items.len() as i128))
                    }
                    other => Err(builtin_error(
                        span,
                        format!("avg expects numbers, found {}", other.ty()),
                    )),
                }
            }
            other => Err(builtin_error(
                span,
                format!("avg expects array, found {}", other.ty()),
            )),
        },
        "min_of" | "max_of" if arguments.len() == 1 => match &arguments[0] {
            Value::Array(items) if items.is_empty() => Ok(Value::Option(None)),
            Value::Array(items) => {
                if items
                    .iter()
                    .any(|item| matches!(item, Value::Float(value) if !value.is_finite()))
                {
                    return Err(builtin_error(
                        span,
                        format!("{name} requires finite Float elements"),
                    ));
                }
                let mut best = items[0].clone();
                for item in &items[1..] {
                    let replace = match (&best, item) {
                        (Value::Int(left), Value::Int(right)) => {
                            if name == "min_of" {
                                right < left
                            } else {
                                right > left
                            }
                        }
                        (Value::UInt(left), Value::UInt(right)) => {
                            if name == "min_of" {
                                right < left
                            } else {
                                right > left
                            }
                        }
                        (Value::Float(left), Value::Float(right)) => {
                            if name == "min_of" {
                                right < left
                            } else {
                                right > left
                            }
                        }
                        (Value::Decimal(left), Value::Decimal(right)) => {
                            if name == "min_of" {
                                right < left
                            } else {
                                right > left
                            }
                        }
                        (Value::String(left), Value::String(right)) => {
                            if name == "min_of" {
                                right < left
                            } else {
                                right > left
                            }
                        }
                        _ => false,
                    };
                    if replace {
                        best = item.clone();
                    }
                }
                Ok(Value::Option(Some(Box::new(best))))
            }
            other => Err(builtin_error(
                span,
                format!("{name} expects array, found {}", other.ty()),
            )),
        },
        "set" if arguments.len() == 1 => match arguments.into_iter().next() {
            Some(Value::Array(mut values)) => {
                values.sort_by_key(Value::output);
                values.dedup();
                Ok(Value::Set(values))
            }
            Some(other) => Err(builtin_error(
                span,
                format!("set expects an array, found {}", other.ty()),
            )),
            None => unreachable!("arity checked by match guard"),
        },
        "set_add" | "set_remove" if arguments.len() == 2 => {
            let [collection, element] = arguments.as_slice() else {
                unreachable!("arity checked by match guard");
            };
            let Value::Set(existing) = collection else {
                return Err(builtin_error(
                    span,
                    format!("{name} expects a Set, found {}", collection.ty()),
                ));
            };
            let mut values = existing.clone();
            if name == "set_add" {
                if !values.contains(element) {
                    values.push(element.clone());
                }
            } else {
                values.retain(|value| value != element);
            }
            values.sort_by_key(Value::output);
            values.dedup();
            Ok(Value::Set(values))
        }
        "set_union" | "set_intersection" | "set_difference" if arguments.len() == 2 => {
            match arguments.as_slice() {
                [Value::Set(left), Value::Set(right)] => {
                    let result = match name {
                        "set_union" => {
                            let mut values = left.clone();
                            for item in right {
                                if !values.contains(item) {
                                    values.push(item.clone());
                                }
                            }
                            values
                        }
                        "set_intersection" => left
                            .iter()
                            .filter(|item| right.contains(item))
                            .cloned()
                            .collect(),
                        _ => left
                            .iter()
                            .filter(|item| !right.contains(item))
                            .cloned()
                            .collect(),
                    };
                    Ok(Value::Set(result))
                }
                _ => Err(builtin_error(span, format!("{name} expects two Sets"))),
            }
        }
        "range" if (2..=3).contains(&arguments.len()) => {
            let (Value::Int(start), Value::Int(end)) = (&arguments[0], &arguments[1]) else {
                return Err(builtin_error(span, "`range` expects Int bounds"));
            };
            let step = match arguments.get(2) {
                Some(Value::Int(step)) => *step,
                Some(other) => {
                    return Err(builtin_error(
                        span,
                        format!("`range` step must be Int, got `{}`", other.ty()),
                    ));
                }
                None => 1,
            };
            if step == 0 {
                return Err(builtin_error(span, "`range` step must not be zero"));
            }
            let mut values = Vec::new();
            let mut current = *start;
            let before_end = |value: i64| if step > 0 { value < *end } else { value > *end };
            while before_end(current) {
                if values.len() >= 1_000_000 {
                    return Err(builtin_error(
                        span,
                        "`range` exceeds the 1,000,000 item limit",
                    ));
                }
                values.push(Value::Int(current));
                let Some(next) = current.checked_add(step) else {
                    break;
                };
                current = next;
            }
            Ok(Value::Array(values))
        }
        "sort" | "sort_desc" if arguments.len() == 1 => match &arguments[0] {
            Value::Array(items) => {
                let mut items = items.clone();
                items.sort_by(|left, right| {
                    let ordering = match (left, right) {
                        (Value::Int(left), Value::Int(right)) => left.cmp(right),
                        (Value::UInt(left), Value::UInt(right)) => left.cmp(right),
                        (Value::Float(left), Value::Float(right)) => {
                            left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        (Value::Decimal(left), Value::Decimal(right)) => left.cmp(right),
                        (Value::String(left), Value::String(right)) => left.cmp(right),
                        _ => std::cmp::Ordering::Equal,
                    };
                    if name == "sort_desc" {
                        ordering.reverse()
                    } else {
                        ordering
                    }
                });
                Ok(Value::Array(items))
            }
            other => Err(builtin_error(
                span,
                format!("{name} expects array, found {}", other.ty()),
            )),
        },
        "reverse" if arguments.len() == 1 => match &arguments[0] {
            Value::Array(items) => {
                let mut items = items.clone();
                items.reverse();
                Ok(Value::Array(items))
            }
            other => Err(builtin_error(
                span,
                format!("reverse expects array, found {}", other.ty()),
            )),
        },
        "unique" if arguments.len() == 1 => match &arguments[0] {
            Value::Array(items) => {
                let mut unique = Vec::new();
                for item in items {
                    if !unique.iter().any(|existing| existing == item) {
                        unique.push(item.clone());
                    }
                }
                Ok(Value::Array(unique))
            }
            other => Err(builtin_error(
                span,
                format!("unique expects array, found {}", other.ty()),
            )),
        },
        "chunk" if arguments.len() == 2 => match arguments.as_slice() {
            [Value::Array(items), Value::Int(size)] if *size > 0 => {
                let size = usize::try_from(*size)
                    .map_err(|_| builtin_error(span, "chunk size exceeds platform limit"))?;
                Ok(Value::Array(
                    items
                        .chunks(size)
                        .map(|chunk| Value::Array(chunk.to_vec()))
                        .collect(),
                ))
            }
            [Value::Array(_), Value::Int(_)] => {
                Err(builtin_error(span, "chunk size must be greater than zero"))
            }
            _ => Err(builtin_error(span, "chunk expects array and Int size")),
        },
        "flatten" if arguments.len() == 1 => match &arguments[0] {
            Value::Array(items) => {
                let mut flattened = Vec::new();
                for item in items {
                    match item {
                        Value::Array(nested) => flattened.extend(nested.iter().cloned()),
                        other => {
                            return Err(builtin_error(
                                span,
                                format!(
                                    "flatten expects array of arrays, found element of type {}",
                                    other.ty()
                                ),
                            ));
                        }
                    }
                }
                Ok(Value::Array(flattened))
            }
            other => Err(builtin_error(
                span,
                format!("flatten expects array, found {}", other.ty()),
            )),
        },
        "slice" if arguments.len() == 3 => match arguments.as_slice() {
            [Value::Array(items), Value::Int(start), Value::Int(length)] => {
                if *start < 0 || *length < 0 {
                    return Err(builtin_error(span, "slice indices must be non-negative"));
                }
                let start = *start as usize;
                let length = *length as usize;
                if start > items.len() {
                    return Ok(Value::Array(Vec::new()));
                }
                let end = start.saturating_add(length).min(items.len());
                Ok(Value::Array(items[start..end].to_vec()))
            }
            _ => Err(builtin_error(
                span,
                "slice expects array, Int start, and Int length",
            )),
        },
        "concat" if arguments.len() == 2 => match arguments.as_slice() {
            [Value::Array(left), Value::Array(right)] => {
                let mut values = left.clone();
                values.extend(right.iter().cloned());
                Ok(Value::Array(values))
            }
            _ => Err(builtin_error(span, "concat expects two arrays")),
        },
        "join" if arguments.len() == 2 => match arguments.as_slice() {
            [Value::Array(items), Value::String(delimiter)] => Ok(Value::String(
                items
                    .iter()
                    .map(Value::output)
                    .collect::<Vec<_>>()
                    .join(delimiter),
            )),
            _ => Err(builtin_error(
                span,
                "join expects array and String delimiter",
            )),
        },
        "to_string" if arguments.len() == 1 => Ok(Value::String(arguments[0].output())),
        "abs" if arguments.len() == 1 => match &arguments[0] {
            Value::Int(value) => value
                .checked_abs()
                .map(Value::Int)
                .ok_or_else(|| builtin_error(span, "Int overflow in abs")),
            Value::Float(value) if value.is_finite() => Ok(Value::Float(value.abs())),
            Value::Float(_) => Err(builtin_error(span, "abs requires a finite Float")),
            Value::Decimal(value) => value
                .checked_abs()
                .map(Value::Decimal)
                .ok_or_else(|| builtin_error(span, "Decimal overflow in abs")),
            other => Err(builtin_error(
                span,
                format!("abs expects numeric value, found {}", other.ty()),
            )),
        },
        "clamp" if arguments.len() == 3 => {
            let [value, minimum, maximum] = arguments.as_slice() else {
                unreachable!("arity checked by match guard");
            };
            match (value, minimum, maximum) {
                (Value::Int(value), Value::Int(minimum), Value::Int(maximum)) => {
                    if minimum > maximum {
                        return Err(builtin_error(span, "clamp minimum must not exceed maximum"));
                    }
                    Ok(Value::Int(*value.max(minimum).min(maximum)))
                }
                (Value::UInt(value), Value::UInt(minimum), Value::UInt(maximum)) => {
                    if minimum > maximum {
                        return Err(builtin_error(span, "clamp minimum must not exceed maximum"));
                    }
                    Ok(Value::UInt(*value.max(minimum).min(maximum)))
                }
                (Value::Float(value), Value::Float(minimum), Value::Float(maximum))
                    if value.is_finite() && minimum.is_finite() && maximum.is_finite() =>
                {
                    if minimum > maximum {
                        return Err(builtin_error(span, "clamp minimum must not exceed maximum"));
                    }
                    Ok(Value::Float(value.max(*minimum).min(*maximum)))
                }
                (Value::Float(_), Value::Float(_), Value::Float(_)) => {
                    Err(builtin_error(span, "clamp requires finite Float arguments"))
                }
                (Value::Decimal(value), Value::Decimal(minimum), Value::Decimal(maximum)) => {
                    if minimum > maximum {
                        return Err(builtin_error(span, "clamp minimum must not exceed maximum"));
                    }
                    Ok(Value::Decimal(*value.max(minimum).min(maximum)))
                }
                _ => Err(builtin_error(
                    span,
                    "clamp expects matching numeric arguments",
                )),
            }
        }
        "min" | "max" if arguments.len() == 2 => {
            let [left, right] = arguments.as_slice() else {
                unreachable!("arity checked by match guard");
            };
            match (left, right) {
                (Value::Int(left), Value::Int(right)) => Ok(Value::Int(if name == "min" {
                    *left.min(right)
                } else {
                    *left.max(right)
                })),
                (Value::UInt(left), Value::UInt(right)) => Ok(Value::UInt(if name == "min" {
                    *left.min(right)
                } else {
                    *left.max(right)
                })),
                (Value::Float(left), Value::Float(right))
                    if left.is_finite() && right.is_finite() =>
                {
                    Ok(Value::Float(if name == "min" {
                        left.min(*right)
                    } else {
                        left.max(*right)
                    }))
                }
                (Value::Float(_), Value::Float(_)) => Err(builtin_error(
                    span,
                    format!("{name} requires finite Float arguments"),
                )),
                (Value::Decimal(left), Value::Decimal(right)) => {
                    Ok(Value::Decimal(if name == "min" {
                        *left.min(right)
                    } else {
                        *left.max(right)
                    }))
                }
                (Value::String(left), Value::String(right)) => {
                    let selected =
                        if (name == "min" && left <= right) || (name == "max" && left >= right) {
                            left.clone()
                        } else {
                            right.clone()
                        };
                    Ok(Value::String(selected))
                }
                _ => Err(builtin_error(
                    span,
                    format!("{name} expects matching numeric or String arguments"),
                )),
            }
        }
        "decimal" if arguments.len() == 1 => match &arguments[0] {
            Value::String(source) => {
                zelyra_ast::decimal::parse_scaled(source, zelyra_ast::decimal::SCALE_DIGITS)
                    .map(Value::Decimal)
                    .ok_or_else(|| {
                        builtin_error(
                            span,
                            "decimal expects a base-10 value with at most 9 fractional digits",
                        )
                    })
            }
            other => Err(builtin_error(
                span,
                format!("decimal expects String, found {}", other.ty()),
            )),
        },
        "sqrt" if arguments.len() == 1 => match &arguments[0] {
            Value::Float(value) if value.is_finite() && *value >= 0.0 => {
                Ok(Value::Float(value.sqrt()))
            }
            Value::Float(_) => Err(builtin_error(
                span,
                "sqrt requires a finite non-negative Float",
            )),
            other => Err(builtin_error(
                span,
                format!("sqrt expects Float, found {}", other.ty()),
            )),
        },
        "pow" if arguments.len() == 2 => match arguments.as_slice() {
            [Value::Float(base), Value::Float(exponent)] => {
                let result = base.powf(*exponent);
                if !base.is_finite() || !exponent.is_finite() || !result.is_finite() {
                    Err(builtin_error(
                        span,
                        "pow is undefined or outside the finite Float range",
                    ))
                } else {
                    Ok(Value::Float(result))
                }
            }
            _ => Err(builtin_error(span, "pow expects Float arguments")),
        },
        "floor" | "ceil" if arguments.len() == 1 => match &arguments[0] {
            Value::Float(value) if value.is_finite() => Ok(Value::Float(if name == "floor" {
                value.floor()
            } else {
                value.ceil()
            })),
            Value::Float(_) => Err(builtin_error(
                span,
                format!("{name} requires a finite Float"),
            )),
            other => Err(builtin_error(
                span,
                format!("{name} expects Float, found {}", other.ty()),
            )),
        },
        "round" if arguments.len() == 2 => match arguments.as_slice() {
            [Value::Float(value), Value::Int(places)] => {
                if !value.is_finite() {
                    return Err(builtin_error(span, "round requires a finite Float"));
                }
                if !(-15..=15).contains(places) {
                    return Err(builtin_error(
                        span,
                        "round decimal places must be between -15 and 15",
                    ));
                }
                let factor = (10.0_f64).powi(*places as i32);
                let rounded = (value * factor).round() / factor;
                if !rounded.is_finite() {
                    Err(builtin_error(
                        span,
                        "round result is outside the finite Float range",
                    ))
                } else {
                    Ok(Value::Float(rounded))
                }
            }
            _ => Err(builtin_error(span, "round expects Float and Int")),
        },
        "text_contains" if arguments.len() == 2 => match arguments.as_slice() {
            [Value::String(text), Value::String(search)] => Ok(Value::Bool(text.contains(search))),
            _ => Err(builtin_error(
                span,
                "text_contains expects String arguments",
            )),
        },
        "text_trim" if arguments.len() == 1 => match &arguments[0] {
            Value::String(text) => Ok(Value::String(text.trim().to_owned())),
            _ => Err(builtin_error(span, "text_trim expects a String")),
        },
        "text_chars" if arguments.len() == 1 => match &arguments[0] {
            Value::String(text) => Ok(Value::Array(
                text.chars()
                    .map(|character| Value::String(character.to_string()))
                    .collect(),
            )),
            _ => Err(builtin_error(span, "text_chars expects a String")),
        },
        "text_to_lower" if arguments.len() == 1 => match &arguments[0] {
            Value::String(text) => Ok(Value::String(text.to_lowercase())),
            _ => Err(builtin_error(span, "text_to_lower expects a String")),
        },
        "text_to_upper" if arguments.len() == 1 => match &arguments[0] {
            Value::String(text) => Ok(Value::String(text.to_uppercase())),
            _ => Err(builtin_error(span, "text_to_upper expects a String")),
        },
        "text_ucfirst" if arguments.len() == 1 => match &arguments[0] {
            Value::String(text) => Ok(Value::String(text_ucfirst(text))),
            _ => Err(builtin_error(span, "text_ucfirst expects a String")),
        },
        "text_starts_with" if arguments.len() == 2 => match arguments.as_slice() {
            [Value::String(text), Value::String(prefix)] => {
                Ok(Value::Bool(text.starts_with(prefix)))
            }
            _ => Err(builtin_error(span, "text_starts_with expects two Strings")),
        },
        "text_ends_with" if arguments.len() == 2 => match arguments.as_slice() {
            [Value::String(text), Value::String(suffix)] => Ok(Value::Bool(text.ends_with(suffix))),
            _ => Err(builtin_error(span, "text_ends_with expects two Strings")),
        },
        "text_replace" if arguments.len() == 3 => match arguments.as_slice() {
            [Value::String(text), Value::String(search), Value::String(replacement)] => {
                Ok(Value::String(text.replace(search, replacement)))
            }
            _ => Err(builtin_error(span, "text_replace expects three Strings")),
        },
        "text_split" | "split" if arguments.len() == 2 => match arguments.as_slice() {
            [Value::String(text), Value::String(delimiter)] => Ok(Value::Array(
                text.split(delimiter)
                    .map(|part| Value::String(part.to_owned()))
                    .collect(),
            )),
            _ => Err(builtin_error(span, format!("{name} expects two Strings"))),
        },
        "parse_int" if arguments.len() == 1 => match &arguments[0] {
            Value::String(text) => match text.trim().parse::<i64>() {
                Ok(parsed) => Ok(Value::Result(Ok(Box::new(Value::Int(parsed))))),
                Err(_) => Ok(Value::Result(Err(Box::new(Value::String(format!(
                    "invalid integer: {text}"
                )))))),
            },
            _ => Err(builtin_error(span, "parse_int expects a String")),
        },
        "to_float" if arguments.len() == 1 => match &arguments[0] {
            Value::Float(value) if value.is_finite() => Ok(Value::Float(*value)),
            Value::Float(_) => Err(builtin_error(span, "to_float requires a finite value")),
            Value::Int(value) => Ok(Value::Float(*value as f64)),
            Value::UInt(value) => Ok(Value::Float(*value as f64)),
            Value::String(value) => match value.trim().parse::<f64>() {
                Ok(parsed) if parsed.is_finite() => Ok(Value::Float(parsed)),
                Err(_) => Err(builtin_error(
                    span,
                    format!("to_float cannot convert `{value}` to Float"),
                )),
                Ok(_) => Err(builtin_error(
                    span,
                    format!("to_float cannot convert `{value}` to a finite Float"),
                )),
            },
            other => Err(builtin_error(
                span,
                format!("to_float cannot convert {} to Float", other.ty()),
            )),
        },
        "to_int" if arguments.len() == 1 => match &arguments[0] {
            Value::Int(value) => Ok(Value::Int(*value)),
            Value::UInt(value) => i64::try_from(*value).map(Value::Int).map_err(|_| {
                builtin_error(span, format!("to_int cannot convert `{value}` to Int"))
            }),
            Value::Float(value)
                if value.is_finite()
                    && *value >= i64::MIN as f64
                    && *value < -(i64::MIN as f64) =>
            {
                Ok(Value::Int(*value as i64))
            }
            Value::Float(_) => Err(builtin_error(
                span,
                "to_int Float must be finite and within the Int range",
            )),
            Value::Bool(value) => Ok(Value::Int(i64::from(*value))),
            Value::String(value) => match value.trim().parse::<i64>() {
                Ok(parsed) => Ok(Value::Int(parsed)),
                Err(_) => Err(builtin_error(
                    span,
                    format!("to_int cannot convert `{value}` to Int"),
                )),
            },
            other => Err(builtin_error(
                span,
                format!("to_int cannot convert {} to Int", other.ty()),
            )),
        },
        "parse_float" if arguments.len() == 1 => match &arguments[0] {
            Value::String(text) => match text.trim().parse::<f64>() {
                Ok(parsed) if parsed.is_finite() => {
                    Ok(Value::Result(Ok(Box::new(Value::Float(parsed)))))
                }
                _ => Ok(Value::Result(Err(Box::new(Value::String(format!(
                    "invalid float: {text}"
                )))))),
            },
            _ => Err(builtin_error(span, "parse_float expects a String")),
        },
        _ => Err(builtin_error(
            span,
            format!("`{name}` received an unsupported argument count"),
        )),
    }
}

fn pad_text_to_codepoints(
    text: &str,
    target_len: i64,
    pad: &str,
    left: bool,
) -> Result<String, String> {
    const MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;
    let target_len = usize::try_from(target_len)
        .map_err(|_| "padding target length must be non-negative".to_owned())?;
    if target_len > MAX_OUTPUT_BYTES {
        return Err("padding target length exceeds the 16 MiB text limit".into());
    }
    let text_len = text.chars().count();
    if text_len >= target_len {
        return Ok(text.to_owned());
    }

    let pad_chars = if pad.is_empty() {
        vec![' ']
    } else {
        pad.chars().collect::<Vec<_>>()
    };
    let needed = target_len - text_len;
    let fill_bytes = (0..needed).try_fold(0usize, |bytes, index| {
        bytes.checked_add(pad_chars[index % pad_chars.len()].len_utf8())
    });
    let Some(output_bytes) = fill_bytes.and_then(|fill| text.len().checked_add(fill)) else {
        return Err("padded text size overflow".into());
    };
    if output_bytes > MAX_OUTPUT_BYTES {
        return Err("padded text exceeds the 16 MiB text limit".into());
    }

    let fill = (0..needed)
        .map(|index| pad_chars[index % pad_chars.len()])
        .collect::<String>();
    Ok(if left {
        format!("{fill}{text}")
    } else {
        format!("{text}{fill}")
    })
}
