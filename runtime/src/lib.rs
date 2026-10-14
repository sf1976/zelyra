use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use zelyra_ast::temporal::{
    date_epoch_days, epoch_days_date, format_timestamp_utc, timestamp_bounds_millis, valid_date,
    valid_time,
};
use zelyra_ast::*;

const BASE64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode_bytes(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        out.push(BASE64_CHARS[(b0 >> 2) as usize] as char);
        out.push(BASE64_CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(BASE64_CHARS[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(BASE64_CHARS[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn base64_decode_bytes(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim_end_matches('=');
    let mut bytes = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for c in input.chars() {
        if c.is_ascii_whitespace() {
            continue;
        }
        let val = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 26,
            '0'..='9' => c as u32 - '0' as u32 + 52,
            '+' => 62,
            '/' => 63,
            _ => return Err(format!("invalid base64 character: `{c}`")),
        };
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(bytes)
}

fn base64_decode_str(input: &str) -> Result<String, String> {
    String::from_utf8(base64_decode_bytes(input)?)
        .map_err(|error| format!("invalid UTF-8 in base64 payload: {error}"))
}

fn url_encode_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.as_bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            _ => {
                use std::fmt::Write;
                let _ = write!(out, "%{:02X}", byte);
            }
        }
    }
    out
}

fn url_decode_str(s: &str) -> Result<String, String> {
    let mut bytes = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' {
            if i + 2 >= chars.len() {
                return Err("incomplete percent encoding".into());
            }
            let hex_str: String = chars[i + 1..=i + 2].iter().collect();
            let b = u8::from_str_radix(&hex_str, 16)
                .map_err(|_| format!("invalid hex in percent encoding: `%{hex_str}`"))?;
            bytes.push(b);
            i += 3;
        } else if chars[i] == '+' {
            bytes.push(b' ');
            i += 1;
        } else {
            let mut buf = [0u8; 4];
            let encoded = chars[i].encode_utf8(&mut buf);
            bytes.extend_from_slice(encoded.as_bytes());
            i += 1;
        }
    }
    String::from_utf8(bytes).map_err(|e| format!("invalid UTF-8 in url decoded content: {e}"))
}

const MAX_TEXT_BUILTIN_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

fn pad_text_to_codepoints(
    text: &str,
    target_len: i64,
    pad: &str,
    left: bool,
) -> Result<String, String> {
    let target_len = usize::try_from(target_len)
        .map_err(|_| "padding target length must be non-negative".to_owned())?;
    if target_len > MAX_TEXT_BUILTIN_OUTPUT_BYTES {
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
    if output_bytes > MAX_TEXT_BUILTIN_OUTPUT_BYTES {
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

fn generate_uuid_v4() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    let mut rng = OsRng;
    rng.try_fill_bytes(&mut bytes)
        .map_err(|e| format!("random access failed: {e}"))?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    ))
}

mod csv;
mod decimal_math;
mod filesystem;
mod mail;
mod parallel;
mod pdf;
mod pdf_template;
use pdf_template::interpolate_pdf_data;
mod records;
mod sql_results;

#[derive(Clone, Debug, PartialEq)]
pub struct TypeError {
    pub code: &'static str,
    pub message: String,
    pub span: Span,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityError {
    pub message: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApiDiagnostic {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for ApiDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub const KNOWN_CAPABILITIES: &[&str] = &[
    "Database",
    "Database(read)",
    "Database(write)",
    "Email",
    "Network",
    "FileSystem",
    "Environment",
    "Process",
    "Clock",
    "Random",
    "Console",
];

pub fn check_apis(program: &Program) -> Result<(), Vec<ApiDiagnostic>> {
    let mut errors = Vec::new();
    let mut known_types = [
        "Id",
        "Email",
        "Url",
        "Uuid",
        "Money",
        "Int",
        "UInt",
        "Float",
        "Decimal",
        "Bool",
        "String",
        "Char",
        "Bytes",
        "Timestamp",
        "Date",
        "Time",
        "Duration",
        "Unit",
        "HttpResponse",
        "HttpError",
        "HttpResult",
    ]
    .into_iter()
    .map(String::from)
    .collect::<HashSet<_>>();
    for definition in &program.types {
        known_types.insert(definition.name.clone());
    }
    for record in &program.records {
        known_types.insert(record.name.clone());
    }
    for definition in &program.enums {
        known_types.insert(definition.name.clone());
    }
    for table in &program.tables {
        known_types.insert(table.name.clone());
        if let Some(singular) = singular_table_type(&table.name) {
            known_types.insert(singular);
        }
    }
    let generic_arities = program
        .records
        .iter()
        .map(|record| (record.name.clone(), record.type_params.len()))
        .chain(
            program
                .enums
                .iter()
                .map(|definition| (definition.name.clone(), definition.type_params.len())),
        )
        .collect::<HashMap<_, _>>();

    let mut routes = HashSet::new();
    let functions = program
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function))
        .collect::<HashMap<_, _>>();
    for api in &program.apis {
        if !routes.insert((api.method.clone(), api.path.clone())) {
            errors.push(ApiDiagnostic {
                message: format!("duplicate API route `{}` `{}`", api.method, api.path),
                span: api.span,
            });
        }
        let parameters = match api_path_parameters(&api.path) {
            Ok(parameters) => parameters,
            Err(message) => {
                errors.push(ApiDiagnostic {
                    message,
                    span: api.span,
                });
                Vec::new()
            }
        };
        let mut fields = HashSet::new();
        for field in &api.input {
            if !fields.insert(field.name.clone()) {
                errors.push(ApiDiagnostic {
                    message: format!("duplicate API input field `{}`", field.name),
                    span: field.span,
                });
            }
            if !api_type_known(&field.ty, &known_types, &generic_arities) {
                errors.push(ApiDiagnostic {
                    message: format!("unknown API input type `{}`", field.ty),
                    span: field.span,
                });
            } else if let Some(message) =
                api_json_type_error(&field.ty, program, &mut HashSet::new())
            {
                errors.push(ApiDiagnostic {
                    message: format!("API input field `{}`: {message}", field.name),
                    span: field.span,
                });
            }
        }
        for parameter in parameters {
            if !fields.contains(&parameter) {
                errors.push(ApiDiagnostic {
                    message: format!(
                        "path parameter `{parameter}` must be declared in the API input"
                    ),
                    span: api.span,
                });
            }
        }
        if !api_type_known(&api.output, &known_types, &generic_arities) {
            errors.push(ApiDiagnostic {
                message: format!("unknown API output type `{}`", api.output),
                span: api.span,
            });
        } else if let Some(message) = api_json_type_error(&api.output, program, &mut HashSet::new())
        {
            errors.push(ApiDiagnostic {
                message: format!("API output: {message}"),
                span: api.span,
            });
        }
        if let Some(handler) = &api.handler {
            let Some(function) = functions.get(handler.as_str()) else {
                errors.push(ApiDiagnostic {
                    message: format!("API handler function `{handler}` does not exist"),
                    span: api.span,
                });
                continue;
            };
            if !function.type_params.is_empty() {
                errors.push(ApiDiagnostic {
                    message: format!(
                        "API handler `{handler}` must be non-generic because API routes cannot supply type arguments"
                    ),
                    span: function.span,
                });
                continue;
            }
            if function.params.len() != api.input.len() {
                errors.push(ApiDiagnostic {
                    message: format!(
                        "API handler `{handler}` expects {} input field(s), but the API declares {}",
                        function.params.len(),
                        api.input.len()
                    ),
                    span: api.span,
                });
            }
            for (parameter, field) in function.params.iter().zip(&api.input) {
                if parameter.name != field.name || parameter.ty != field.ty {
                    errors.push(ApiDiagnostic {
                        message: format!(
                            "API handler parameter `{}` must match input field `{}` with type `{}`",
                            parameter.name, field.name, field.ty
                        ),
                        span: field.span,
                    });
                }
            }
            if function.return_type.as_ref() != Some(&api.output) {
                errors.push(ApiDiagnostic {
                    message: format!(
                        "API handler `{handler}` must return `{}`, found `{}`",
                        api.output,
                        function
                            .return_type
                            .as_ref()
                            .map_or_else(|| "Unit".into(), ToString::to_string)
                    ),
                    span: api.span,
                });
            }
        }
        let mut statuses = HashSet::new();
        for error in &api.errors {
            if !statuses.insert(error.status) {
                errors.push(ApiDiagnostic {
                    message: format!("duplicate API error status `{}`", error.status),
                    span: error.span,
                });
            }
            if let Some(payload) = &error.payload {
                if !api_type_known(payload, &known_types, &generic_arities) {
                    errors.push(ApiDiagnostic {
                        message: format!(
                            "API error `{}` refers to unknown payload type `{payload}`",
                            error.name
                        ),
                        span: error.span,
                    });
                } else if let Some(message) =
                    api_json_type_error(payload, program, &mut HashSet::new())
                {
                    errors.push(ApiDiagnostic {
                        message: format!("API error `{}`: {message}", error.name),
                        span: error.span,
                    });
                }
                match &api.output {
                    Type::Result(_, error_type) if compatible(error_type, payload) => {}
                    Type::Result(_, error_type) => errors.push(ApiDiagnostic {
                        message: format!(
                            "API error `{}` payload type `{payload}` does not match handler error type `{error_type}`",
                            error.name
                        ),
                        span: error.span,
                    }),
                    _ => errors.push(ApiDiagnostic {
                        message: format!(
                            "API error `{}` declares a payload but API output is not a Result",
                            error.name
                        ),
                        span: error.span,
                    }),
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn api_type_known(
    ty: &Type,
    known_types: &HashSet<String>,
    generic_arities: &HashMap<String, usize>,
) -> bool {
    match ty {
        Type::Named(name) => {
            known_types.contains(name) && generic_arities.get(name).is_none_or(|arity| *arity == 0)
        }
        Type::Applied { name, args } => {
            known_types.contains(name)
                && generic_arities
                    .get(name)
                    .is_some_and(|arity| *arity == args.len())
                && args
                    .iter()
                    .all(|argument| api_type_known(argument, known_types, generic_arities))
        }
        Type::Option(inner) | Type::Array(inner) | Type::Set(inner) | Type::HttpResult(inner) => {
            api_type_known(inner, known_types, generic_arities)
        }
        Type::Tuple(items) => items
            .iter()
            .all(|item| api_type_known(item, known_types, generic_arities)),
        Type::Map(key, value) => {
            api_type_known(key, known_types, generic_arities)
                && api_type_known(value, known_types, generic_arities)
        }
        Type::Result(ok, error) => {
            api_type_known(ok, known_types, generic_arities)
                && api_type_known(error, known_types, generic_arities)
        }
        _ => true,
    }
}

/// Returns an error for values that cannot cross the JSON API boundary without
/// losing their declared shape. JSON objects have string keys, so a Zelyra map
/// exposed through an API must use `String` keys. The traversal also checks
/// aliases and nested records so the guarantee is enforced at the boundary,
/// rather than only for the top-level API type.
fn api_json_type_error(
    ty: &Type,
    program: &Program,
    visiting: &mut HashSet<String>,
) -> Option<String> {
    match ty {
        Type::Option(inner) | Type::Array(inner) | Type::Set(inner) | Type::HttpResult(inner) => {
            api_json_type_error(inner, program, visiting)
        }
        Type::Tuple(items) => items
            .iter()
            .find_map(|item| api_json_type_error(item, program, visiting)),
        Type::Map(key, value) => {
            if **key != Type::String {
                return Some(format!(
                    "JSON API maps require `String` keys, found `{}`",
                    key
                ));
            }
            api_json_type_error(value, program, visiting)
        }
        Type::Result(ok, error) => api_json_type_error(ok, program, visiting)
            .or_else(|| api_json_type_error(error, program, visiting)),
        Type::Function { .. } => Some("function values cannot cross an API JSON boundary".into()),
        Type::Applied { name, args } => {
            if program
                .records
                .iter()
                .any(|record| record.name == *name && record.is_class)
            {
                return Some(format!(
                    "class references of type `{ty}` cannot cross an API JSON boundary"
                ));
            }
            if let Some(error) = args
                .iter()
                .find_map(|argument| api_json_type_error(argument, program, visiting))
            {
                return Some(error);
            }
            let key = ty.to_string();
            if !visiting.insert(key.clone()) {
                return None;
            }
            let substitutions = program
                .records
                .iter()
                .find(|record| record.name == *name)
                .map(|record| {
                    record
                        .type_params
                        .iter()
                        .zip(args)
                        .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                        .collect::<HashMap<_, _>>()
                })
                .or_else(|| {
                    program
                        .enums
                        .iter()
                        .find(|definition| definition.name == *name)
                        .map(|definition| {
                            definition
                                .type_params
                                .iter()
                                .zip(args)
                                .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                                .collect::<HashMap<_, _>>()
                        })
                });
            let result = if let Some(substitutions) = substitutions {
                if let Some(record) = program.records.iter().find(|record| record.name == *name) {
                    record.fields.iter().find_map(|field| {
                        let field_type = substitute_generic_type(&field.ty, &substitutions);
                        api_json_type_error(&field_type, program, visiting)
                    })
                } else if let Some(definition) = program
                    .enums
                    .iter()
                    .find(|definition| definition.name == *name)
                {
                    definition
                        .variant_payloads
                        .iter()
                        .flat_map(|(_, payloads)| payloads)
                        .find_map(|payload| {
                            let payload_type = substitute_generic_type(payload, &substitutions);
                            api_json_type_error(&payload_type, program, visiting)
                        })
                } else {
                    None
                }
            } else {
                None
            };
            visiting.remove(&key);
            result
        }
        Type::Named(name) => {
            if program
                .records
                .iter()
                .any(|record| record.name == *name && record.is_class)
            {
                return Some(format!(
                    "class references of type `{name}` cannot cross an API JSON boundary"
                ));
            }
            if !visiting.insert(name.clone()) {
                return None;
            }
            let result = program
                .types
                .iter()
                .find(|definition| definition.name == *name)
                .and_then(|definition| api_json_type_error(&definition.target, program, visiting))
                .or_else(|| {
                    program
                        .records
                        .iter()
                        .find(|record| record.name == *name)
                        .and_then(|record| {
                            record
                                .fields
                                .iter()
                                .find_map(|field| api_json_type_error(&field.ty, program, visiting))
                        })
                })
                .or_else(|| {
                    program
                        .tables
                        .iter()
                        .find(|table| {
                            table.name == *name
                                || singular_table_type(&table.name).as_deref() == Some(name)
                        })
                        .and_then(|table| {
                            table.columns.iter().find_map(|column| {
                                api_json_type_error(&column.ty, program, visiting)
                            })
                        })
                });
            visiting.remove(name);
            result
        }
        _ => None,
    }
}

fn contains_class_reference_type(ty: &Type, program: &Program) -> bool {
    fn contains(ty: &Type, program: &Program, visiting: &mut HashSet<String>) -> bool {
        let record = |name: &str| program.records.iter().find(|item| item.name == name);
        match ty {
            Type::Named(name) => {
                if record(name).is_some_and(|definition| definition.is_class) {
                    return true;
                }
                if !visiting.insert(ty.to_string()) {
                    return false;
                }
                let found = program
                    .types
                    .iter()
                    .find(|alias| alias.name == *name)
                    .is_some_and(|alias| contains(&alias.target, program, visiting))
                    || record(name).is_some_and(|definition| {
                        definition
                            .fields
                            .iter()
                            .any(|field| contains(&field.ty, program, visiting))
                    })
                    || program
                        .enums
                        .iter()
                        .find(|definition| definition.name == *name)
                        .is_some_and(|definition| {
                            definition
                                .variant_payloads
                                .iter()
                                .flat_map(|(_, payloads)| payloads)
                                .any(|payload| contains(payload, program, visiting))
                        });
                visiting.remove(&ty.to_string());
                found
            }
            Type::Applied { name, args } => {
                if record(name).is_some_and(|definition| definition.is_class)
                    || args
                        .iter()
                        .any(|argument| contains(argument, program, visiting))
                {
                    return true;
                }
                let key = ty.to_string();
                if !visiting.insert(key.clone()) {
                    return false;
                }
                let substitutions = record(name)
                    .map(|definition| {
                        definition
                            .type_params
                            .iter()
                            .zip(args)
                            .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                            .collect::<HashMap<_, _>>()
                    })
                    .or_else(|| {
                        program
                            .enums
                            .iter()
                            .find(|definition| definition.name == *name)
                            .map(|definition| {
                                definition
                                    .type_params
                                    .iter()
                                    .zip(args)
                                    .map(|(parameter, argument)| {
                                        (parameter.clone(), argument.clone())
                                    })
                                    .collect::<HashMap<_, _>>()
                            })
                    });
                let found = if let Some(substitutions) = substitutions {
                    record(name)
                        .map(|definition| {
                            definition.fields.iter().any(|field| {
                                let field_type = substitute_generic_type(&field.ty, &substitutions);
                                contains(&field_type, program, visiting)
                            })
                        })
                        .or_else(|| {
                            program
                                .enums
                                .iter()
                                .find(|definition| definition.name == *name)
                                .map(|definition| {
                                    definition
                                        .variant_payloads
                                        .iter()
                                        .flat_map(|(_, payloads)| payloads)
                                        .any(|payload| {
                                            let payload_type =
                                                substitute_generic_type(payload, &substitutions);
                                            contains(&payload_type, program, visiting)
                                        })
                                })
                        })
                        .unwrap_or(false)
                } else {
                    false
                };
                visiting.remove(&key);
                found
            }
            Type::Option(inner)
            | Type::Array(inner)
            | Type::Set(inner)
            | Type::HttpResult(inner) => contains(inner, program, visiting),
            Type::Result(ok, error) => {
                contains(ok, program, visiting) || contains(error, program, visiting)
            }
            Type::Tuple(items) => items.iter().any(|item| contains(item, program, visiting)),
            Type::Map(key, value) => {
                contains(key, program, visiting) || contains(value, program, visiting)
            }
            Type::Function {
                params,
                return_type,
                ..
            } => {
                params
                    .iter()
                    .any(|parameter| contains(parameter, program, visiting))
                    || contains(return_type, program, visiting)
            }
            _ => false,
        }
    }

    contains(ty, program, &mut HashSet::new())
}

fn api_path_parameters(path: &str) -> Result<Vec<String>, String> {
    if !path.starts_with('/') || path.contains(['?', '#']) {
        return Err("API path must start with `/` and must not contain a query or fragment".into());
    }
    let mut parameters = Vec::new();
    for segment in path.split('/').skip(1) {
        if segment.is_empty() {
            continue;
        }
        if let Some(name) = segment
            .strip_prefix('{')
            .and_then(|segment| segment.strip_suffix('}'))
        {
            if name.is_empty()
                || !name.chars().enumerate().all(|(index, character)| {
                    if index == 0 {
                        character == '_' || character.is_ascii_alphabetic()
                    } else {
                        character == '_' || character.is_ascii_alphanumeric()
                    }
                })
            {
                return Err(format!("invalid API path parameter `{segment}`"));
            }
            if parameters.iter().any(|existing| existing == name) {
                return Err(format!("duplicate API path parameter `{name}`"));
            }
            parameters.push(name.to_owned());
        } else if segment.contains(['{', '}']) {
            return Err(format!("invalid API path segment `{segment}`"));
        }
    }
    Ok(parameters)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractKind {
    Requires,
    Ensures,
    LoopInvariant,
}

impl fmt::Display for ContractKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Requires => write!(f, "requires"),
            Self::Ensures => write!(f, "ensures"),
            Self::LoopInvariant => write!(f, "invariant"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationStatus {
    Proven,
    RuntimeCheck,
    Unproven,
    Failed,
}

impl fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = match self {
            Self::Proven => "PROVEN",
            Self::RuntimeCheck => "RUNTIME_CHECK",
            Self::Unproven => "UNPROVEN",
            Self::Failed => "FAILED",
        };
        write!(f, "{status}")
    }
}

impl VerificationStatus {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Proven => "V-001",
            Self::RuntimeCheck => "V-002",
            Self::Unproven => "V-003",
            Self::Failed => "V-004",
        }
    }

    pub const fn explanation(self) -> &'static str {
        match self {
            Self::Proven => "The verifier proved this condition for all analyzed paths.",
            Self::RuntimeCheck => {
                "The verifier could not complete a symbolic proof; runtime checking is required."
            }
            Self::Unproven => "No proof is available for this condition.",
            Self::Failed => "The condition is false on a feasible analyzed path.",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationResult {
    pub function: String,
    pub kind: ContractKind,
    pub index: usize,
    pub status: VerificationStatus,
    pub span: Span,
    pub message: String,
    pub counterexample: Option<Vec<(String, i64)>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VerificationEvidence {
    status: VerificationStatus,
    counterexample: Option<Vec<(String, i64)>>,
}

pub fn verify(program: &Program) -> Vec<VerificationResult> {
    let functions = program
        .functions
        .iter()
        .map(|function| (function.name.clone(), function))
        .collect::<HashMap<_, _>>();
    let mut results = Vec::new();
    for function in &program.functions {
        for (index, contract) in function.requires.iter().enumerate() {
            let evidence = verify_contract(contract, None, &functions);
            results.push(VerificationResult {
                function: function.name.clone(),
                kind: ContractKind::Requires,
                index,
                message: verification_message(
                    ContractKind::Requires,
                    evidence.status,
                    Some(contract),
                ),
                status: evidence.status,
                span: contract.span,
                counterexample: evidence.counterexample,
            });
        }
        let mut invariant_diagnostics = HashMap::new();
        let return_paths =
            symbolic_return_paths(function, Some(&functions), &mut invariant_diagnostics);
        for (index, contract) in function.ensures.iter().enumerate() {
            let evidence = verify_postcondition(
                contract,
                &function.requires,
                return_paths.as_deref(),
                &functions,
            );
            results.push(VerificationResult {
                function: function.name.clone(),
                kind: ContractKind::Ensures,
                index,
                message: verification_message(
                    ContractKind::Ensures,
                    evidence.status,
                    Some(contract),
                ),
                status: evidence.status,
                span: contract.span,
                counterexample: evidence.counterexample,
            });
        }
        let mut loop_invariants = Vec::new();
        collect_loop_invariants(&function.body, &mut loop_invariants);
        for (loop_id, invariants) in loop_invariants {
            for (index, invariant) in invariants.iter().enumerate() {
                let evidence = invariant_diagnostics
                    .get(&(loop_id, index))
                    .cloned()
                    .unwrap_or_else(|| verify_contract(invariant, None, &functions));
                results.push(VerificationResult {
                    function: function.name.clone(),
                    kind: ContractKind::LoopInvariant,
                    index,
                    message: verification_message(
                        ContractKind::LoopInvariant,
                        evidence.status,
                        Some(invariant),
                    ),
                    status: evidence.status,
                    span: invariant.span,
                    counterexample: evidence.counterexample,
                });
            }
        }
        if function.requires.is_empty() && function.ensures.is_empty() {
            results.push(VerificationResult {
                function: function.name.clone(),
                kind: ContractKind::Requires,
                index: 0,
                status: VerificationStatus::Unproven,
                span: function.span,
                message: verification_message(
                    ContractKind::Requires,
                    VerificationStatus::Unproven,
                    None,
                ),
                counterexample: None,
            });
        }
    }
    results
}

fn verification_message(
    kind: ContractKind,
    status: VerificationStatus,
    expression: Option<&Expr>,
) -> String {
    if status == VerificationStatus::Failed
        && expression.is_some_and(|expression| {
            matches!(constant_value(expression), Some(ConstantValue::Bool(false)))
        })
    {
        return "Constant contradiction: this condition evaluates to false for every input.".into();
    }
    match (kind, status, expression.is_some()) {
        (_, VerificationStatus::Proven, _) => status.explanation().into(),
        (ContractKind::Requires, VerificationStatus::RuntimeCheck, _) => {
            "This precondition needs a runtime check because the symbolic proof is incomplete."
                .into()
        }
        (ContractKind::Ensures, VerificationStatus::RuntimeCheck, _) => {
            "This postcondition needs a runtime check because not all return paths are symbolically modeled."
                .into()
        }
        (ContractKind::LoopInvariant, VerificationStatus::RuntimeCheck, _) => {
            "This loop invariant needs a runtime check because entry or preservation could not be proved symbolically."
                .into()
        }
        (ContractKind::Requires, VerificationStatus::Failed, _) => {
            "The precondition is false for a feasible checked model.".into()
        }
        (ContractKind::Ensures, VerificationStatus::Failed, _) => {
            "The postcondition is false on a feasible return path.".into()
        }
        (ContractKind::LoopInvariant, VerificationStatus::Failed, _) => {
            "The loop invariant is false on a feasible path or is not preserved by the loop body."
                .into()
        }
        (_, VerificationStatus::Unproven, false) => {
            "No proof target was declared for this function.".into()
        }
        (_, VerificationStatus::Unproven, true) => {
            "No proof is available for this condition.".into()
        }
    }
}

fn verify_contract(
    contract: &Expr,
    return_expression: Option<&Expr>,
    functions: &HashMap<String, &Function>,
) -> VerificationEvidence {
    let status =
        match constant_value(contract) {
            Some(ConstantValue::Bool(true)) => VerificationStatus::Proven,
            Some(ConstantValue::Bool(false)) => VerificationStatus::Failed,
            Some(_) => VerificationStatus::Unproven,
            None => symbolic_bool(contract, return_expression, None, None, Some(functions), 0)
                .map_or(VerificationStatus::RuntimeCheck, |value| {
                    if value {
                        VerificationStatus::Proven
                    } else {
                        VerificationStatus::Failed
                    }
                }),
        };
    let counterexample = if status == VerificationStatus::Failed
        && !matches!(constant_value(contract), Some(ConstantValue::Bool(false)))
    {
        find_symbolic_counterexample(
            contract,
            return_expression,
            &[],
            &HashMap::new(),
            &HashMap::new(),
            Some(functions),
        )
    } else {
        None
    };
    VerificationEvidence {
        status,
        counterexample,
    }
}

fn verify_postcondition(
    contract: &Expr,
    preconditions: &[Expr],
    return_paths: Option<&[ReturnPath<'_>]>,
    functions: &HashMap<String, &Function>,
) -> VerificationEvidence {
    match constant_value(contract) {
        Some(ConstantValue::Bool(true)) => VerificationEvidence {
            status: VerificationStatus::Proven,
            counterexample: None,
        },
        Some(ConstantValue::Bool(false)) => VerificationEvidence {
            status: VerificationStatus::Failed,
            counterexample: None,
        },
        Some(_) => VerificationEvidence {
            status: VerificationStatus::Unproven,
            counterexample: None,
        },
        None => {
            let Some(return_paths) = return_paths else {
                return VerificationEvidence {
                    status: VerificationStatus::RuntimeCheck,
                    counterexample: None,
                };
            };
            let mut saw_feasible_path = false;
            let mut saw_unknown_path = false;
            for path in return_paths {
                let Some(expression) = path.expression else {
                    saw_unknown_path = true;
                    continue;
                };
                let mut alternatives = vec![Vec::new()];
                for precondition in preconditions {
                    let guard = SymbolicGuard::Condition {
                        expression: precondition,
                        expected: true,
                    };
                    let Some(precondition_alternatives) = constraints_for_guard(
                        &guard,
                        Some(expression),
                        Some(&path.bindings),
                        Some(&path.substitutions),
                        Some(functions),
                        0,
                    ) else {
                        saw_unknown_path = true;
                        alternatives.clear();
                        break;
                    };
                    alternatives = combine_alternatives(alternatives, precondition_alternatives);
                }
                if alternatives.is_empty() {
                    continue;
                }
                for guard in &path.guards {
                    let Some(guard_alternatives) = constraints_for_guard(
                        guard,
                        Some(expression),
                        Some(&path.bindings),
                        Some(&path.substitutions),
                        Some(functions),
                        0,
                    ) else {
                        saw_unknown_path = true;
                        alternatives.clear();
                        break;
                    };
                    alternatives = combine_alternatives(alternatives, guard_alternatives);
                }
                for guards in alternatives {
                    if constraints_satisfiable(guards.clone()) == Some(false) {
                        continue;
                    }
                    saw_feasible_path = true;
                    let context = SymbolicContext {
                        return_expression: Some(expression),
                        bindings: Some(&path.bindings),
                        substitutions: Some(&path.substitutions),
                        functions: Some(functions),
                        depth: 0,
                    };
                    if call_preconditions_hold(expression, &guards, context) != Some(true)
                        || call_preconditions_hold(contract, &guards, context) != Some(true)
                    {
                        saw_unknown_path = true;
                        continue;
                    }
                    match symbolic_bool_with_constraints(
                        contract,
                        expression,
                        &guards,
                        Some(&path.bindings),
                        Some(&path.substitutions),
                        Some(functions),
                        0,
                    ) {
                        Some(true) => {}
                        Some(false) => {
                            return VerificationEvidence {
                                status: VerificationStatus::Failed,
                                counterexample: find_symbolic_counterexample(
                                    contract,
                                    Some(expression),
                                    &guards,
                                    &path.bindings,
                                    &path.substitutions,
                                    Some(functions),
                                ),
                            }
                        }
                        None => saw_unknown_path = true,
                    }
                }
            }
            if saw_feasible_path && !saw_unknown_path {
                VerificationEvidence {
                    status: VerificationStatus::Proven,
                    counterexample: None,
                }
            } else {
                VerificationEvidence {
                    status: VerificationStatus::RuntimeCheck,
                    counterexample: None,
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
enum SymbolicGuard<'a> {
    Condition {
        expression: &'a Expr,
        expected: bool,
    },
    ConditionSnapshot {
        expression: &'a Expr,
        expected: bool,
        substitutions: HashMap<String, LinearValue>,
    },
    Match {
        value: &'a Expr,
        pattern: &'a Pattern,
        matched: bool,
    },
}

#[derive(Clone, Debug)]
struct ReturnPath<'a> {
    guards: Vec<SymbolicGuard<'a>>,
    expression: Option<&'a Expr>,
    bindings: HashMap<String, &'a Expr>,
    substitutions: HashMap<String, LinearValue>,
}

type InvariantDiagnostics = HashMap<(usize, usize), VerificationEvidence>;

struct LoopSymbolicContext<'a> {
    parameters: &'a HashSet<String>,
    functions: Option<&'a HashMap<String, &'a Function>>,
    diagnostics: &'a mut InvariantDiagnostics,
}

#[derive(Clone, Debug)]
enum SymbolicState<'a> {
    Continue {
        guards: Vec<SymbolicGuard<'a>>,
        bindings: HashMap<String, &'a Expr>,
        substitutions: HashMap<String, LinearValue>,
    },
    Break {
        guards: Vec<SymbolicGuard<'a>>,
        bindings: HashMap<String, &'a Expr>,
        substitutions: HashMap<String, LinearValue>,
    },
    LoopContinue {
        guards: Vec<SymbolicGuard<'a>>,
        bindings: HashMap<String, &'a Expr>,
        substitutions: HashMap<String, LinearValue>,
    },
    Return {
        guards: Vec<SymbolicGuard<'a>>,
        expression: Option<&'a Expr>,
        bindings: HashMap<String, &'a Expr>,
        substitutions: HashMap<String, LinearValue>,
    },
}

fn symbolic_return_paths<'a>(
    function: &'a Function,
    functions: Option<&HashMap<String, &Function>>,
    diagnostics: &mut InvariantDiagnostics,
) -> Option<Vec<ReturnPath<'a>>> {
    let parameters = function
        .params
        .iter()
        .map(|parameter| parameter.name.clone())
        .collect::<HashSet<_>>();
    let initial_guards = function
        .requires
        .iter()
        .map(|expression| SymbolicGuard::Condition {
            expression,
            expected: true,
        })
        .collect();
    let states = symbolic_states(
        &function.body,
        initial_guards,
        HashMap::new(),
        HashMap::new(),
        &parameters,
        functions,
        diagnostics,
    )?;
    let mut paths = Vec::new();
    for state in states {
        let SymbolicState::Return {
            guards,
            expression,
            bindings,
            substitutions,
        } = state
        else {
            return None;
        };
        paths.push(ReturnPath {
            guards,
            expression,
            bindings,
            substitutions,
        });
    }
    Some(paths)
}

fn direct_return_expression(function: &Function) -> Option<&Expr> {
    let [Stmt::Return {
        value: Some(expression),
        ..
    }] = function.body.statements.as_slice()
    else {
        return None;
    };
    Some(expression)
}

fn add_pattern_bindings<'a>(
    value: &'a Expr,
    pattern: &'a Pattern,
    bindings: &mut HashMap<String, &'a Expr>,
) {
    match &pattern.kind {
        PatternKind::Variable(name) => {
            if !matches!(&value.kind, ExprKind::Variable(existing) if existing == name) {
                bindings.insert(name.clone(), value);
            }
        }
        PatternKind::Constructor { name, inner } => {
            let Some(inner) = inner else {
                return;
            };
            let ExprKind::Call {
                name: value_name,
                args,
                ..
            } = &value.kind
            else {
                return;
            };
            if value_name == name && args.len() == 1 {
                add_pattern_bindings(&args[0], inner, bindings);
            }
        }
        PatternKind::EnumVariant { .. } => {}
        PatternKind::EnumVariantPayload { .. } => {}
        PatternKind::Wildcard
        | PatternKind::Int(_)
        | PatternKind::Bool(_)
        | PatternKind::String(_)
        | PatternKind::Char(_)
        | PatternKind::Record { .. } => {}
    }
}

fn symbolic_states<'a>(
    block: &'a Block,
    incoming: Vec<SymbolicGuard<'a>>,
    bindings: HashMap<String, &'a Expr>,
    substitutions: HashMap<String, LinearValue>,
    parameters: &HashSet<String>,
    functions: Option<&HashMap<String, &Function>>,
    diagnostics: &mut InvariantDiagnostics,
) -> Option<Vec<SymbolicState<'a>>> {
    let mut states = vec![SymbolicState::Continue {
        guards: incoming,
        bindings,
        substitutions,
    }];
    for statement in &block.statements {
        let mut next = Vec::new();
        for state in states {
            match state {
                SymbolicState::Return { .. }
                | SymbolicState::Break { .. }
                | SymbolicState::LoopContinue { .. } => next.push(state),
                SymbolicState::Continue {
                    guards,
                    bindings,
                    substitutions,
                } => match statement {
                    Stmt::Let {
                        name,
                        value,
                        mutable: false,
                        ..
                    } if !expression_contains_variable(value, name) => {
                        let mut bindings = bindings;
                        bindings.insert(name.clone(), value);
                        next.push(SymbolicState::Continue {
                            guards,
                            bindings,
                            substitutions,
                        });
                    }
                    Stmt::Let {
                        name,
                        value,
                        mutable: true,
                        ..
                    } => {
                        let value = linear_value(
                            value,
                            None,
                            Some(&bindings),
                            Some(&substitutions),
                            functions,
                            0,
                        )?;
                        let mut substitutions = substitutions;
                        substitutions.insert(name.clone(), value);
                        next.push(SymbolicState::Continue {
                            guards,
                            bindings,
                            substitutions,
                        });
                    }
                    Stmt::BindOrAssign { name, value, .. } if substitutions.contains_key(name) => {
                        let value = linear_value(
                            value,
                            None,
                            Some(&bindings),
                            Some(&substitutions),
                            functions,
                            0,
                        )?;
                        let mut substitutions = substitutions;
                        substitutions.insert(name.clone(), value);
                        next.push(SymbolicState::Continue {
                            guards,
                            bindings,
                            substitutions,
                        });
                    }
                    Stmt::BindOrAssign { name, value, .. }
                        if !parameters.contains(name)
                            && !bindings.contains_key(name)
                            && !substitutions.contains_key(name)
                            && !expression_contains_variable(value, name) =>
                    {
                        let mut bindings = bindings;
                        bindings.insert(name.clone(), value);
                        next.push(SymbolicState::Continue {
                            guards,
                            bindings,
                            substitutions,
                        });
                    }
                    Stmt::Return { value, .. } => next.push(SymbolicState::Return {
                        guards,
                        expression: value.as_ref(),
                        bindings,
                        substitutions,
                    }),
                    Stmt::Break { .. } => next.push(SymbolicState::Break {
                        guards,
                        bindings,
                        substitutions,
                    }),
                    Stmt::Continue { .. } => next.push(SymbolicState::LoopContinue {
                        guards,
                        bindings,
                        substitutions,
                    }),
                    Stmt::If {
                        condition,
                        then_block,
                        else_block,
                        ..
                    } => {
                        let mut then_guards = guards.clone();
                        then_guards.push(SymbolicGuard::ConditionSnapshot {
                            expression: condition,
                            expected: true,
                            substitutions: substitutions.clone(),
                        });
                        next.extend(symbolic_states(
                            then_block,
                            then_guards,
                            bindings.clone(),
                            substitutions.clone(),
                            parameters,
                            functions,
                            diagnostics,
                        )?);

                        if let Some(else_block) = else_block {
                            let mut else_guards = guards;
                            else_guards.push(SymbolicGuard::ConditionSnapshot {
                                expression: condition,
                                expected: false,
                                substitutions: substitutions.clone(),
                            });
                            next.extend(symbolic_states(
                                else_block,
                                else_guards,
                                bindings.clone(),
                                substitutions.clone(),
                                parameters,
                                functions,
                                diagnostics,
                            )?);
                        } else {
                            let mut guards = guards;
                            guards.push(SymbolicGuard::ConditionSnapshot {
                                expression: condition,
                                expected: false,
                                substitutions: substitutions.clone(),
                            });
                            next.push(SymbolicState::Continue {
                                guards,
                                bindings,
                                substitutions,
                            });
                        }
                    }
                    Stmt::Match { value, arms, .. } => {
                        for (index, arm) in arms.iter().enumerate() {
                            let mut arm_guards = guards.clone();
                            for previous in arms.iter().take(index) {
                                arm_guards.push(SymbolicGuard::Match {
                                    value,
                                    pattern: &previous.pattern,
                                    matched: false,
                                });
                            }
                            arm_guards.push(SymbolicGuard::Match {
                                value,
                                pattern: &arm.pattern,
                                matched: true,
                            });
                            let mut arm_bindings = bindings.clone();
                            add_pattern_bindings(value, &arm.pattern, &mut arm_bindings);
                            next.extend(symbolic_states(
                                &arm.body,
                                arm_guards,
                                arm_bindings,
                                substitutions.clone(),
                                parameters,
                                functions,
                                diagnostics,
                            )?);
                        }
                    }
                    Stmt::While {
                        condition,
                        invariants,
                        body,
                        span,
                    } => {
                        let mut context = LoopSymbolicContext {
                            parameters,
                            functions,
                            diagnostics,
                        };
                        next.extend(symbolic_loop_states(
                            condition,
                            invariants,
                            span.start,
                            body,
                            SymbolicState::Continue {
                                guards,
                                bindings,
                                substitutions,
                            },
                            &mut context,
                            0,
                        )?);
                    }
                    Stmt::Loop {
                        invariants,
                        body,
                        span,
                    } => {
                        let mut context = LoopSymbolicContext {
                            parameters,
                            functions,
                            diagnostics,
                        };
                        next.extend(symbolic_unconditional_loop_states(
                            invariants,
                            span.start,
                            body,
                            SymbolicState::Continue {
                                guards,
                                bindings,
                                substitutions,
                            },
                            &mut context,
                            0,
                        )?);
                    }
                    _ => return None,
                },
            }
        }
        states = next;
    }
    Some(states)
}

fn symbolic_loop_states<'a>(
    condition: &'a Expr,
    invariants: &'a [Expr],
    loop_id: usize,
    body: &'a Block,
    state: SymbolicState<'a>,
    context: &mut LoopSymbolicContext<'_>,
    iterations: usize,
) -> Option<Vec<SymbolicState<'a>>> {
    let SymbolicState::Continue {
        guards,
        bindings,
        substitutions,
    } = state
    else {
        return None;
    };
    for (index, invariant) in invariants.iter().enumerate() {
        let evidence = invariant_status(
            invariant,
            &guards,
            &bindings,
            &substitutions,
            context.functions,
        );
        record_invariant_status(context.diagnostics, loop_id, index, evidence.clone());
        if evidence.status != VerificationStatus::Proven {
            return None;
        }
    }
    let true_constraints = constraints_for_bool(
        condition,
        true,
        None,
        Some(&bindings),
        Some(&substitutions),
        context.functions,
        0,
    )?;
    let mut exit_guards = guards.clone();
    for invariant in invariants {
        exit_guards.push(SymbolicGuard::ConditionSnapshot {
            expression: invariant,
            expected: true,
            substitutions: substitutions.clone(),
        });
    }
    exit_guards.push(SymbolicGuard::ConditionSnapshot {
        expression: condition,
        expected: false,
        substitutions: substitutions.clone(),
    });
    let mut states = Vec::new();
    let true_possible = true_constraints
        .iter()
        .any(|constraints| constraints_satisfiable(constraints.clone()) != Some(false));
    if !true_possible {
        states.push(SymbolicState::Continue {
            guards: exit_guards,
            bindings,
            substitutions,
        });
        return Some(states);
    }
    if iterations >= 32 {
        if !invariants.is_empty() {
            let mut abstract_substitutions = substitutions.clone();
            let mut modified = HashSet::new();
            collect_loop_modified_names(body, &mut modified);
            for name in modified {
                abstract_substitutions.remove(&name);
            }
            let mut abstract_exit_guards = guards;
            for invariant in invariants {
                abstract_exit_guards.push(SymbolicGuard::ConditionSnapshot {
                    expression: invariant,
                    expected: true,
                    substitutions: abstract_substitutions.clone(),
                });
            }
            abstract_exit_guards.push(SymbolicGuard::ConditionSnapshot {
                expression: condition,
                expected: false,
                substitutions: abstract_substitutions.clone(),
            });
            states.push(SymbolicState::Continue {
                guards: abstract_exit_guards,
                bindings,
                substitutions: abstract_substitutions,
            });
            return Some(states);
        }
        return None;
    }

    let mut body_guards = guards;
    for invariant in invariants {
        body_guards.push(SymbolicGuard::ConditionSnapshot {
            expression: invariant,
            expected: true,
            substitutions: substitutions.clone(),
        });
    }
    body_guards.push(SymbolicGuard::ConditionSnapshot {
        expression: condition,
        expected: true,
        substitutions: substitutions.clone(),
    });
    for state in symbolic_states(
        body,
        body_guards,
        bindings,
        substitutions,
        context.parameters,
        context.functions,
        context.diagnostics,
    )? {
        match state {
            SymbolicState::Return { .. } => states.push(state),
            SymbolicState::Break {
                guards,
                bindings,
                substitutions,
            } => {
                for (index, invariant) in invariants.iter().enumerate() {
                    let evidence = invariant_status(
                        invariant,
                        &guards,
                        &bindings,
                        &substitutions,
                        context.functions,
                    );
                    record_invariant_status(context.diagnostics, loop_id, index, evidence.clone());
                    if evidence.status != VerificationStatus::Proven {
                        return None;
                    }
                }
                states.push(SymbolicState::Continue {
                    guards,
                    bindings,
                    substitutions,
                })
            }
            SymbolicState::LoopContinue {
                guards,
                bindings,
                substitutions,
            } => states.extend(symbolic_loop_states(
                condition,
                invariants,
                loop_id,
                body,
                SymbolicState::Continue {
                    guards,
                    bindings,
                    substitutions,
                },
                context,
                iterations + 1,
            )?),
            SymbolicState::Continue {
                guards,
                bindings,
                substitutions,
            } => {
                for (index, invariant) in invariants.iter().enumerate() {
                    let evidence = invariant_status(
                        invariant,
                        &guards,
                        &bindings,
                        &substitutions,
                        context.functions,
                    );
                    record_invariant_status(context.diagnostics, loop_id, index, evidence.clone());
                    if evidence.status != VerificationStatus::Proven {
                        return None;
                    }
                }
                states.extend(symbolic_loop_states(
                    condition,
                    invariants,
                    loop_id,
                    body,
                    SymbolicState::Continue {
                        guards,
                        bindings,
                        substitutions,
                    },
                    context,
                    iterations + 1,
                )?)
            }
        }
    }
    Some(states)
}

fn symbolic_unconditional_loop_states<'a>(
    invariants: &'a [Expr],
    loop_id: usize,
    body: &'a Block,
    state: SymbolicState<'a>,
    context: &mut LoopSymbolicContext<'_>,
    iterations: usize,
) -> Option<Vec<SymbolicState<'a>>> {
    let SymbolicState::Continue {
        guards,
        bindings,
        substitutions,
    } = state
    else {
        return None;
    };
    for (index, invariant) in invariants.iter().enumerate() {
        let evidence = invariant_status(
            invariant,
            &guards,
            &bindings,
            &substitutions,
            context.functions,
        );
        record_invariant_status(context.diagnostics, loop_id, index, evidence.clone());
        if evidence.status != VerificationStatus::Proven {
            return None;
        }
    }

    let mut body_guards = guards;
    for invariant in invariants {
        body_guards.push(SymbolicGuard::ConditionSnapshot {
            expression: invariant,
            expected: true,
            substitutions: substitutions.clone(),
        });
    }
    let mut states = Vec::new();
    for state in symbolic_states(
        body,
        body_guards,
        bindings,
        substitutions,
        context.parameters,
        context.functions,
        context.diagnostics,
    )? {
        match state {
            SymbolicState::Return { .. } => states.push(state),
            SymbolicState::Break {
                guards,
                bindings,
                substitutions,
            } => {
                for (index, invariant) in invariants.iter().enumerate() {
                    let evidence = invariant_status(
                        invariant,
                        &guards,
                        &bindings,
                        &substitutions,
                        context.functions,
                    );
                    record_invariant_status(context.diagnostics, loop_id, index, evidence.clone());
                    if evidence.status != VerificationStatus::Proven {
                        return None;
                    }
                }
                states.push(SymbolicState::Continue {
                    guards,
                    bindings,
                    substitutions,
                });
            }
            SymbolicState::LoopContinue {
                guards,
                bindings,
                substitutions,
            }
            | SymbolicState::Continue {
                guards,
                bindings,
                substitutions,
            } => {
                if iterations >= 32 {
                    return None;
                }
                states.extend(symbolic_unconditional_loop_states(
                    invariants,
                    loop_id,
                    body,
                    SymbolicState::Continue {
                        guards,
                        bindings,
                        substitutions,
                    },
                    context,
                    iterations + 1,
                )?);
            }
        }
    }
    Some(states)
}

fn collect_loop_modified_names(block: &Block, names: &mut HashSet<String>) {
    for statement in &block.statements {
        match statement {
            Stmt::Let {
                name,
                mutable: true,
                ..
            }
            | Stmt::BindOrAssign { name, .. } => {
                names.insert(name.clone());
            }
            Stmt::TupleBindOrAssign {
                names: assigned, ..
            } => {
                names.extend(assigned.iter().cloned());
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_loop_modified_names(then_block, names);
                if let Some(else_block) = else_block {
                    collect_loop_modified_names(else_block, names);
                }
            }
            Stmt::While { body, .. }
            | Stmt::For { body, .. }
            | Stmt::Loop { body, .. }
            | Stmt::Transaction { body, .. }
            | Stmt::Parallel { body, .. } => collect_loop_modified_names(body, names),
            Stmt::Match { arms, .. } => {
                for arm in arms {
                    collect_loop_modified_names(&arm.body, names);
                }
            }
            Stmt::Let { .. }
            | Stmt::Expr(_)
            | Stmt::Return { .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. } => {}
        }
    }
}

fn collect_loop_invariants<'a>(block: &'a Block, loops: &mut Vec<(usize, &'a [Expr])>) {
    for statement in &block.statements {
        match statement {
            Stmt::While {
                invariants,
                body,
                span,
                ..
            }
            | Stmt::Loop {
                invariants,
                body,
                span,
            } => {
                loops.push((span.start, invariants));
                collect_loop_invariants(body, loops);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_loop_invariants(then_block, loops);
                if let Some(else_block) = else_block {
                    collect_loop_invariants(else_block, loops);
                }
            }
            Stmt::Match { arms, .. } => {
                for arm in arms {
                    collect_loop_invariants(&arm.body, loops);
                }
            }
            Stmt::For { body, .. } => collect_loop_invariants(body, loops),
            Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
                collect_loop_invariants(body, loops)
            }
            Stmt::Let { .. }
            | Stmt::BindOrAssign { .. }
            | Stmt::TupleBindOrAssign { .. }
            | Stmt::Expr(_)
            | Stmt::Return { .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. } => {}
        }
    }
}

fn expression_contains_variable(expression: &Expr, name: &str) -> bool {
    match &expression.kind {
        ExprKind::Variable(variable) => variable == name,
        ExprKind::Call { args, .. } => args
            .iter()
            .any(|argument| expression_contains_variable(argument, name)),
        ExprKind::Unary { expr, .. } => expression_contains_variable(expr, name),
        ExprKind::Binary { left, right, .. } => {
            expression_contains_variable(left, name) || expression_contains_variable(right, name)
        }
        ExprKind::Array(values) => values
            .iter()
            .any(|value| expression_contains_variable(value, name)),
        ExprKind::Map(entries) => entries.iter().any(|(key, value)| {
            expression_contains_variable(key, name) || expression_contains_variable(value, name)
        }),
        ExprKind::Record { fields, .. } => fields
            .iter()
            .any(|(_, value)| expression_contains_variable(value, name)),
        ExprKind::Index { target, index } => {
            expression_contains_variable(target, name) || expression_contains_variable(index, name)
        }
        ExprKind::Field { target, .. } => expression_contains_variable(target, name),
        ExprKind::Await(inner) => expression_contains_variable(inner, name),
        ExprKind::Match { value, arms } => {
            expression_contains_variable(value, name)
                || arms
                    .iter()
                    .any(|arm| expression_contains_variable(&arm.value, name))
        }
        ExprKind::Lambda { params, body } => {
            !params.iter().any(|param| param.name == name)
                && expression_contains_variable(body, name)
        }
        ExprKind::Int(..)
        | ExprKind::UInt(..)
        | ExprKind::Float(..)
        | ExprKind::Bool(..)
        | ExprKind::String(..)
        | ExprKind::Char(..)
        | ExprKind::FunctionValue(..)
        | ExprKind::Sql { .. } => false,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct LinearValue {
    coefficients: HashMap<String, i64>,
    constant: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct LinearConstraint {
    coefficients: HashMap<String, i128>,
    constant: i128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PathConstraint {
    Linear(LinearConstraint),
    Constructor {
        expression: usize,
        name: String,
        equal: bool,
    },
}

impl LinearConstraint {
    fn from_value(value: LinearValue) -> Self {
        Self {
            coefficients: value
                .coefficients
                .into_iter()
                .map(|(name, coefficient)| (name, i128::from(coefficient)))
                .collect(),
            constant: i128::from(value.constant),
        }
    }

    fn negate(mut self) -> Option<Self> {
        self.constant = self.constant.checked_neg()?;
        for coefficient in self.coefficients.values_mut() {
            *coefficient = coefficient.checked_neg()?;
        }
        Some(self)
    }

    fn shift(mut self, amount: i128) -> Option<Self> {
        self.constant = self.constant.checked_sub(amount)?;
        Some(self)
    }

    fn combine(self, other: Self, self_factor: i128, other_factor: i128) -> Option<Self> {
        let mut combined = Self {
            coefficients: HashMap::new(),
            constant: self
                .constant
                .checked_mul(self_factor)?
                .checked_add(other.constant.checked_mul(other_factor)?)?,
        };
        for (name, coefficient) in self.coefficients {
            let value = coefficient.checked_mul(self_factor)?;
            if value != 0 {
                combined.coefficients.insert(name, value);
            }
        }
        for (name, coefficient) in other.coefficients {
            let value = coefficient.checked_mul(other_factor)?;
            let sum = combined
                .coefficients
                .get(&name)
                .copied()
                .unwrap_or_default()
                .checked_add(value)?;
            if sum == 0 {
                combined.coefficients.remove(&name);
            } else {
                combined.coefficients.insert(name, sum);
            }
        }
        Some(combined)
    }
}

fn combine_alternatives(
    left: Vec<Vec<PathConstraint>>,
    right: Vec<Vec<PathConstraint>>,
) -> Vec<Vec<PathConstraint>> {
    left.into_iter()
        .flat_map(|left_constraints| {
            right.iter().map(move |right_constraints| {
                left_constraints
                    .iter()
                    .cloned()
                    .chain(right_constraints.iter().cloned())
                    .collect()
            })
        })
        .collect()
}

#[derive(Clone, Copy)]
struct SymbolicContext<'a> {
    return_expression: Option<&'a Expr>,
    bindings: Option<&'a HashMap<String, &'a Expr>>,
    substitutions: Option<&'a HashMap<String, LinearValue>>,
    functions: Option<&'a HashMap<String, &'a Function>>,
    depth: usize,
}

fn constraints_for_guard(
    guard: &SymbolicGuard<'_>,
    return_expression: Option<&Expr>,
    bindings: Option<&HashMap<String, &Expr>>,
    substitutions: Option<&HashMap<String, LinearValue>>,
    functions: Option<&HashMap<String, &Function>>,
    depth: usize,
) -> Option<Vec<Vec<PathConstraint>>> {
    match guard {
        SymbolicGuard::Condition {
            expression,
            expected,
        } => constraints_for_bool(
            expression,
            *expected,
            return_expression,
            bindings,
            substitutions,
            functions,
            depth,
        ),
        SymbolicGuard::ConditionSnapshot {
            expression,
            expected,
            substitutions,
        } => constraints_for_bool(
            expression,
            *expected,
            return_expression,
            bindings,
            Some(substitutions),
            functions,
            depth,
        ),
        SymbolicGuard::Match {
            value,
            pattern,
            matched,
        } => constraints_for_pattern(
            value,
            pattern,
            *matched,
            SymbolicContext {
                return_expression,
                bindings,
                substitutions,
                functions,
                depth,
            },
        ),
    }
}

fn constraints_for_pattern(
    value: &Expr,
    pattern: &Pattern,
    matched: bool,
    context: SymbolicContext<'_>,
) -> Option<Vec<Vec<PathConstraint>>> {
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Variable(_) => {
            if matched {
                Some(vec![Vec::new()])
            } else {
                Some(Vec::new())
            }
        }
        PatternKind::Int(expected) => {
            let left = linear_value_alternatives(
                value,
                context.return_expression,
                context.bindings,
                context.substitutions,
                context.functions,
                context.depth,
            )?;
            comparison_constraint_alternatives(
                left,
                vec![(
                    LinearValue {
                        constant: *expected,
                        ..LinearValue::default()
                    },
                    Vec::new(),
                )],
                BinaryOp::Equal,
                matched,
            )
        }
        PatternKind::Bool(expected) => {
            let left = linear_value_alternatives(
                value,
                context.return_expression,
                context.bindings,
                context.substitutions,
                context.functions,
                context.depth,
            )?;
            let right = LinearConstraint {
                coefficients: HashMap::new(),
                constant: i128::from(*expected as u8),
            };
            let mut alternatives = Vec::new();
            for (left_value, left_constraints) in left {
                let left = LinearConstraint::from_value(left_value);
                let domain = [
                    left.clone(),
                    LinearConstraint {
                        coefficients: HashMap::new(),
                        constant: 1,
                    }
                    .combine(left.clone(), 1, -1)?,
                ];
                for comparison in
                    comparison_constraints(left.clone(), BinaryOp::Equal, right.clone(), matched)?
                {
                    alternatives.push(
                        left_constraints
                            .iter()
                            .cloned()
                            .chain(comparison)
                            .chain(domain.iter().cloned().map(PathConstraint::Linear))
                            .collect(),
                    );
                }
            }
            Some(alternatives)
        }
        PatternKind::String(_)
        | PatternKind::Char(_)
        | PatternKind::Record { .. }
        | PatternKind::EnumVariant { .. }
        | PatternKind::EnumVariantPayload { .. } => None,
        PatternKind::Constructor { name, .. } => {
            let expression = value as *const Expr as usize;
            let mut constraints = vec![PathConstraint::Constructor {
                expression,
                name: name.clone(),
                equal: matched,
            }];
            if let ExprKind::Call {
                name: known_name,
                args,
                ..
            } = &value.kind
            {
                if args.len() == 1 && matches!(known_name.as_str(), "Some" | "None" | "Ok" | "Err")
                {
                    constraints.push(PathConstraint::Constructor {
                        expression,
                        name: known_name.clone(),
                        equal: true,
                    });
                }
            }
            Some(vec![constraints])
        }
    }
}

fn comparison_constraint_alternatives(
    left: Vec<(LinearValue, Vec<PathConstraint>)>,
    right: Vec<(LinearValue, Vec<PathConstraint>)>,
    operator: BinaryOp,
    expected: bool,
) -> Option<Vec<Vec<PathConstraint>>> {
    let mut alternatives = Vec::new();
    for (left, left_constraints) in left {
        for (right, right_constraints) in &right {
            for comparison in comparison_constraints(
                LinearConstraint::from_value(left.clone()),
                operator,
                LinearConstraint::from_value(right.clone()),
                expected,
            )? {
                alternatives.push(
                    left_constraints
                        .iter()
                        .cloned()
                        .chain(right_constraints.iter().cloned())
                        .chain(comparison)
                        .collect(),
                );
            }
        }
    }
    Some(alternatives)
}

fn constraints_for_bool(
    expression: &Expr,
    expected: bool,
    return_expression: Option<&Expr>,
    bindings: Option<&HashMap<String, &Expr>>,
    substitutions: Option<&HashMap<String, LinearValue>>,
    functions: Option<&HashMap<String, &Function>>,
    depth: usize,
) -> Option<Vec<Vec<PathConstraint>>> {
    if let Some(ConstantValue::Bool(value)) = constant_value(expression) {
        return if value == expected {
            Some(vec![Vec::new()])
        } else {
            Some(Vec::new())
        };
    }
    match &expression.kind {
        ExprKind::Unary {
            op: UnaryOp::Not,
            expr,
        } => constraints_for_bool(
            expr,
            !expected,
            return_expression,
            bindings,
            substitutions,
            functions,
            depth,
        ),
        ExprKind::Binary { left, op, right } => match op {
            BinaryOp::And if expected => Some(combine_alternatives(
                constraints_for_bool(
                    left,
                    true,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?,
                constraints_for_bool(
                    right,
                    true,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?,
            )),
            BinaryOp::Or if !expected => Some(combine_alternatives(
                constraints_for_bool(
                    left,
                    false,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?,
                constraints_for_bool(
                    right,
                    false,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?,
            )),
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual => {
                let left = linear_value_alternatives(
                    left,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?;
                let right = linear_value_alternatives(
                    right,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?;
                comparison_constraint_alternatives(left, right, *op, expected)
            }
            _ => None,
        },
        _ => None,
    }
}

fn comparison_constraints(
    left: LinearConstraint,
    operator: BinaryOp,
    right: LinearConstraint,
    expected: bool,
) -> Option<Vec<Vec<PathConstraint>>> {
    let operator = if expected {
        operator
    } else {
        match operator {
            BinaryOp::Equal => BinaryOp::NotEqual,
            BinaryOp::NotEqual => BinaryOp::Equal,
            BinaryOp::Less => BinaryOp::GreaterEqual,
            BinaryOp::LessEqual => BinaryOp::Greater,
            BinaryOp::Greater => BinaryOp::LessEqual,
            BinaryOp::GreaterEqual => BinaryOp::Less,
            _ => return None,
        }
    };
    let difference = left.clone().combine(right.clone(), 1, -1)?;
    match operator {
        BinaryOp::Equal => Some(vec![vec![
            PathConstraint::Linear(difference.clone()),
            PathConstraint::Linear(difference.negate()?),
        ]]),
        BinaryOp::NotEqual => Some(vec![
            vec![PathConstraint::Linear(
                right.clone().combine(left.clone(), 1, -1)?.shift(1)?,
            )],
            vec![PathConstraint::Linear(difference.shift(1)?)],
        ]),
        BinaryOp::Less => Some(vec![vec![PathConstraint::Linear(
            right.combine(left, 1, -1)?.shift(1)?,
        )]]),
        BinaryOp::LessEqual => Some(vec![vec![PathConstraint::Linear(
            right.combine(left, 1, -1)?,
        )]]),
        BinaryOp::Greater => Some(vec![vec![PathConstraint::Linear(difference.shift(1)?)]]),
        BinaryOp::GreaterEqual => Some(vec![vec![PathConstraint::Linear(difference)]]),
        _ => None,
    }
}

fn constraints_satisfiable(constraints: Vec<PathConstraint>) -> Option<bool> {
    let mut constructor_equals = HashMap::<usize, String>::new();
    let mut constructor_not_equals = HashMap::<usize, HashSet<String>>::new();
    let mut linear_constraints = Vec::new();
    for constraint in constraints {
        match constraint {
            PathConstraint::Linear(constraint) => linear_constraints.push(constraint),
            PathConstraint::Constructor {
                expression,
                name,
                equal,
            } => {
                if equal {
                    if constructor_equals
                        .get(&expression)
                        .is_some_and(|existing| existing != &name)
                        || constructor_not_equals
                            .get(&expression)
                            .is_some_and(|names| names.contains(&name))
                    {
                        return Some(false);
                    }
                    constructor_equals.insert(expression, name);
                } else if constructor_equals
                    .get(&expression)
                    .is_some_and(|existing| existing == &name)
                {
                    return Some(false);
                } else {
                    constructor_not_equals
                        .entry(expression)
                        .or_default()
                        .insert(name);
                }
            }
        }
    }

    let mut variables = linear_constraints
        .iter()
        .flat_map(|constraint| constraint.coefficients.keys().cloned())
        .collect::<Vec<_>>();
    variables.sort();
    variables.dedup();

    for variable in variables {
        let mut positive = Vec::new();
        let mut negative = Vec::new();
        let mut zero = Vec::new();
        for mut constraint in linear_constraints {
            match constraint
                .coefficients
                .remove(&variable)
                .unwrap_or_default()
            {
                coefficient if coefficient > 0 => positive.push((coefficient, constraint)),
                coefficient if coefficient < 0 => negative.push((coefficient, constraint)),
                _ => zero.push(constraint),
            }
        }
        let mut reduced = zero;
        for (positive_coefficient, lower) in &positive {
            for (negative_coefficient, upper) in &negative {
                reduced.push(lower.clone().combine(
                    upper.clone(),
                    negative_coefficient.checked_neg()?,
                    *positive_coefficient,
                )?);
            }
        }
        linear_constraints = reduced;
    }
    Some(
        linear_constraints
            .iter()
            .all(|constraint| constraint.constant >= 0),
    )
}

fn find_symbolic_counterexample(
    predicate: &Expr,
    return_expression: Option<&Expr>,
    guards: &[PathConstraint],
    bindings: &HashMap<String, &Expr>,
    substitutions: &HashMap<String, LinearValue>,
    functions: Option<&HashMap<String, &Function>>,
) -> Option<Vec<(String, i64)>> {
    let violations = constraints_for_bool(
        predicate,
        false,
        return_expression,
        Some(bindings),
        Some(substitutions),
        functions,
        0,
    )?;
    let alternatives = combine_alternatives(vec![guards.to_vec()], violations);
    for constraints in alternatives {
        if constraints_satisfiable(constraints.clone()) == Some(false) {
            continue;
        }
        if let Some(model) = find_linear_model(&constraints) {
            return Some(model);
        }
    }
    None
}

fn find_linear_model(constraints: &[PathConstraint]) -> Option<Vec<(String, i64)>> {
    const COUNTEREXAMPLE_BOUND: i64 = 32;
    const MAX_COUNTEREXAMPLE_VARIABLES: usize = 3;

    let mut linear_constraints = Vec::new();
    for constraint in constraints {
        match constraint {
            PathConstraint::Linear(constraint) => linear_constraints.push(constraint),
            PathConstraint::Constructor { .. } => return None,
        }
    }
    let mut variables = linear_constraints
        .iter()
        .flat_map(|constraint| constraint.coefficients.keys().cloned())
        .collect::<Vec<_>>();
    variables.sort();
    variables.dedup();
    if variables.len() > MAX_COUNTEREXAMPLE_VARIABLES {
        return None;
    }

    let mut model = HashMap::new();
    find_linear_model_values(
        &linear_constraints,
        &variables,
        0,
        &mut model,
        COUNTEREXAMPLE_BOUND,
    )
}

fn find_linear_model_values(
    constraints: &[&LinearConstraint],
    variables: &[String],
    index: usize,
    model: &mut HashMap<String, i64>,
    bound: i64,
) -> Option<Vec<(String, i64)>> {
    if index == variables.len() {
        if !linear_model_satisfies(constraints, model) {
            return None;
        }
        return Some(
            variables
                .iter()
                .map(|name| (name.clone(), *model.get(name).expect("model value")))
                .collect(),
        );
    }

    let name = &variables[index];
    for distance in 0..=bound {
        for value in [distance, -distance] {
            model.insert(name.clone(), value);
            if let Some(result) =
                find_linear_model_values(constraints, variables, index + 1, model, bound)
            {
                return Some(result);
            }
        }
    }
    model.remove(name);
    None
}

fn linear_model_satisfies(constraints: &[&LinearConstraint], model: &HashMap<String, i64>) -> bool {
    constraints.iter().all(|constraint| {
        let Some(value) = constraint.coefficients.iter().try_fold(
            constraint.constant,
            |total, (name, coefficient)| {
                total.checked_add(coefficient.checked_mul(i128::from(*model.get(name)?))?)
            },
        ) else {
            return false;
        };
        value >= 0
    })
}

fn prove_symbolic_predicate(
    predicate: &Expr,
    guards: &[SymbolicGuard<'_>],
    bindings: &HashMap<String, &Expr>,
    substitutions: &HashMap<String, LinearValue>,
    functions: Option<&HashMap<String, &Function>>,
) -> Option<bool> {
    let alternatives =
        symbolic_predicate_alternatives(predicate, guards, bindings, substitutions, functions)?;
    for constraints in alternatives {
        match constraints_satisfiable(constraints) {
            Some(false) => {}
            Some(true) => return Some(false),
            None => return None,
        }
    }
    Some(true)
}

fn symbolic_predicate_alternatives(
    predicate: &Expr,
    guards: &[SymbolicGuard<'_>],
    bindings: &HashMap<String, &Expr>,
    substitutions: &HashMap<String, LinearValue>,
    functions: Option<&HashMap<String, &Function>>,
) -> Option<Vec<Vec<PathConstraint>>> {
    let mut guard_alternatives = vec![Vec::new()];
    for guard in guards {
        let alternatives = constraints_for_guard(
            guard,
            None,
            Some(bindings),
            Some(substitutions),
            functions,
            0,
        )?;
        guard_alternatives = combine_alternatives(guard_alternatives, alternatives);
    }
    let violations = constraints_for_bool(
        predicate,
        false,
        None,
        Some(bindings),
        Some(substitutions),
        functions,
        0,
    )?;
    let alternatives = combine_alternatives(guard_alternatives, violations);
    Some(alternatives)
}

fn find_symbolic_predicate_counterexample(
    predicate: &Expr,
    guards: &[SymbolicGuard<'_>],
    bindings: &HashMap<String, &Expr>,
    substitutions: &HashMap<String, LinearValue>,
    functions: Option<&HashMap<String, &Function>>,
) -> Option<Vec<(String, i64)>> {
    for constraints in
        symbolic_predicate_alternatives(predicate, guards, bindings, substitutions, functions)?
    {
        if constraints_satisfiable(constraints.clone()) == Some(false) {
            continue;
        }
        if let Some(model) = find_linear_model(&constraints) {
            return Some(model);
        }
    }
    None
}

fn invariant_status(
    predicate: &Expr,
    guards: &[SymbolicGuard<'_>],
    bindings: &HashMap<String, &Expr>,
    substitutions: &HashMap<String, LinearValue>,
    functions: Option<&HashMap<String, &Function>>,
) -> VerificationEvidence {
    let status =
        match prove_symbolic_predicate(predicate, guards, bindings, substitutions, functions) {
            Some(true) => VerificationStatus::Proven,
            Some(false) => VerificationStatus::Failed,
            None => VerificationStatus::RuntimeCheck,
        };
    let counterexample = (status == VerificationStatus::Failed)
        .then(|| {
            find_symbolic_predicate_counterexample(
                predicate,
                guards,
                bindings,
                substitutions,
                functions,
            )
        })
        .flatten();
    VerificationEvidence {
        status,
        counterexample,
    }
}

fn record_invariant_status(
    diagnostics: &mut InvariantDiagnostics,
    loop_id: usize,
    index: usize,
    evidence: VerificationEvidence,
) {
    diagnostics
        .entry((loop_id, index))
        .and_modify(|existing| {
            let existing_priority = verification_status_priority(existing.status);
            let new_priority = verification_status_priority(evidence.status);
            if new_priority > existing_priority {
                *existing = evidence.clone();
            } else if existing.counterexample.is_none() {
                existing.counterexample = evidence.counterexample.clone();
            }
        })
        .or_insert(evidence);
}

fn verification_status_priority(status: VerificationStatus) -> u8 {
    match status {
        VerificationStatus::Proven => 0,
        VerificationStatus::Unproven => 1,
        VerificationStatus::RuntimeCheck => 2,
        VerificationStatus::Failed => 3,
    }
}

fn symbolic_bool_with_constraints(
    expression: &Expr,
    return_expression: &Expr,
    guards: &[PathConstraint],
    bindings: Option<&HashMap<String, &Expr>>,
    substitutions: Option<&HashMap<String, LinearValue>>,
    functions: Option<&HashMap<String, &Function>>,
    depth: usize,
) -> Option<bool> {
    if let Some(value) = symbolic_bool(
        expression,
        Some(return_expression),
        bindings,
        substitutions,
        functions,
        depth,
    ) {
        return Some(value);
    }
    let true_constraints = constraints_for_bool(
        expression,
        true,
        Some(return_expression),
        bindings,
        substitutions,
        functions,
        depth,
    )?;
    let false_constraints = constraints_for_bool(
        expression,
        false,
        Some(return_expression),
        bindings,
        substitutions,
        functions,
        depth,
    )?;

    let true_possible = true_constraints.into_iter().any(|constraints| {
        let mut true_path = guards.to_owned();
        true_path.extend(constraints);
        constraints_satisfiable(true_path) != Some(false)
    });
    if !true_possible {
        return Some(false);
    }
    let false_possible = false_constraints.into_iter().any(|constraints| {
        let mut false_path = guards.to_owned();
        false_path.extend(constraints);
        constraints_satisfiable(false_path) != Some(false)
    });
    if !false_possible {
        return Some(true);
    }
    None
}

fn call_preconditions_hold(
    expression: &Expr,
    guards: &[PathConstraint],
    context: SymbolicContext<'_>,
) -> Option<bool> {
    if context.depth >= 32 {
        return None;
    }
    match &expression.kind {
        ExprKind::Unary { expr, .. } => call_preconditions_hold(expr, guards, context),
        ExprKind::Binary { left, right, .. } => {
            if call_preconditions_hold(left, guards, context)?
                && call_preconditions_hold(right, guards, context)?
            {
                Some(true)
            } else {
                Some(false)
            }
        }
        ExprKind::Call { name, args, .. } => {
            for argument in args {
                match call_preconditions_hold(argument, guards, context) {
                    Some(true) => {}
                    result => return result,
                }
            }
            let Some(functions) = context.functions else {
                return Some(true);
            };
            let Some(function) = functions.get(name) else {
                return Some(true);
            };
            if args.len() != function.params.len() {
                return None;
            }
            let mut argument_values = HashMap::new();
            for (parameter, argument) in function.params.iter().zip(args) {
                let value = linear_value(
                    argument,
                    context.return_expression,
                    context.bindings,
                    context.substitutions,
                    context.functions,
                    context.depth,
                )?;
                argument_values.insert(parameter.name.clone(), value);
            }
            for requirement in &function.requires {
                let true_constraints = constraints_for_bool(
                    requirement,
                    true,
                    None,
                    None,
                    Some(&argument_values),
                    context.functions,
                    context.depth + 1,
                )?;
                let false_constraints = constraints_for_bool(
                    requirement,
                    false,
                    None,
                    None,
                    Some(&argument_values),
                    context.functions,
                    context.depth + 1,
                )?;
                let true_possible = true_constraints.into_iter().any(|constraints| {
                    let mut path = guards.to_owned();
                    path.extend(constraints);
                    constraints_satisfiable(path) != Some(false)
                });
                if !true_possible {
                    return Some(false);
                }
                let false_possible = false_constraints.into_iter().any(|constraints| {
                    let mut path = guards.to_owned();
                    path.extend(constraints);
                    constraints_satisfiable(path) != Some(false)
                });
                if false_possible {
                    return None;
                }
            }
            let Some(return_expression) = direct_return_expression(function) else {
                return Some(true);
            };
            call_preconditions_hold(
                return_expression,
                guards,
                SymbolicContext {
                    return_expression: None,
                    bindings: None,
                    substitutions: Some(&argument_values),
                    functions: context.functions,
                    depth: context.depth + 1,
                },
            )
        }
        _ => Some(true),
    }
}

impl LinearValue {
    fn add(mut self, other: Self) -> Option<Self> {
        self.constant = self.constant.checked_add(other.constant)?;
        for (name, coefficient) in other.coefficients {
            let value = self
                .coefficients
                .get(&name)
                .copied()
                .unwrap_or_default()
                .checked_add(coefficient)?;
            if value == 0 {
                self.coefficients.remove(&name);
            } else {
                self.coefficients.insert(name, value);
            }
        }
        Some(self)
    }

    fn negate(mut self) -> Option<Self> {
        self.constant = self.constant.checked_neg()?;
        for coefficient in self.coefficients.values_mut() {
            *coefficient = coefficient.checked_neg()?;
        }
        Some(self)
    }

    fn scale(mut self, factor: i64) -> Option<Self> {
        self.constant = self.constant.checked_mul(factor)?;
        for coefficient in self.coefficients.values_mut() {
            *coefficient = coefficient.checked_mul(factor)?;
        }
        Some(self)
    }

    fn subtract(self, other: Self) -> Option<Self> {
        self.add(other.negate()?)
    }
}

fn substitute_linear_value(
    value: &LinearValue,
    substitutions: &HashMap<String, LinearValue>,
) -> Option<LinearValue> {
    let mut result = LinearValue {
        constant: value.constant,
        ..LinearValue::default()
    };
    for (name, coefficient) in &value.coefficients {
        let term = substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| LinearValue {
                coefficients: HashMap::from([(name.clone(), 1)]),
                constant: 0,
            })
            .scale(*coefficient)?;
        result = result.add(term)?;
    }
    Some(result)
}

fn linear_value(
    expression: &Expr,
    return_expression: Option<&Expr>,
    bindings: Option<&HashMap<String, &Expr>>,
    substitutions: Option<&HashMap<String, LinearValue>>,
    functions: Option<&HashMap<String, &Function>>,
    depth: usize,
) -> Option<LinearValue> {
    let mut alternatives = linear_value_alternatives(
        expression,
        return_expression,
        bindings,
        substitutions,
        functions,
        depth,
    )?;
    if alternatives.len() != 1 || !alternatives[0].1.is_empty() {
        return None;
    }
    Some(alternatives.remove(0).0)
}

fn linear_value_alternatives(
    expression: &Expr,
    return_expression: Option<&Expr>,
    bindings: Option<&HashMap<String, &Expr>>,
    substitutions: Option<&HashMap<String, LinearValue>>,
    functions: Option<&HashMap<String, &Function>>,
    depth: usize,
) -> Option<Vec<(LinearValue, Vec<PathConstraint>)>> {
    match &expression.kind {
        ExprKind::Int(value) => Some(vec![(
            (LinearValue {
                constant: *value,
                ..LinearValue::default()
            }),
            Vec::new(),
        )]),
        ExprKind::Variable(name) if name == "result" => return_expression.and_then(|expression| {
            linear_value_alternatives(
                expression,
                return_expression,
                bindings,
                substitutions,
                functions,
                depth,
            )
        }),
        ExprKind::Variable(name)
            if substitutions
                .and_then(|substitutions| substitutions.get(name))
                .is_some() =>
        {
            Some(vec![(substitutions?.get(name).cloned()?, Vec::new())])
        }
        ExprKind::Variable(name) if bindings.and_then(|bindings| bindings.get(name)).is_some() => {
            let bound = bindings?.get(name)?;
            linear_value_alternatives(
                bound,
                return_expression,
                bindings,
                substitutions,
                functions,
                depth,
            )
        }
        ExprKind::Variable(name) => Some(vec![(
            LinearValue {
                coefficients: HashMap::from([(name.clone(), 1)]),
                constant: 0,
            },
            Vec::new(),
        )]),
        ExprKind::Unary {
            op: UnaryOp::Negate,
            expr,
        } => linear_value_alternatives(
            expr,
            return_expression,
            bindings,
            substitutions,
            functions,
            depth,
        )?
        .into_iter()
        .map(|(value, constraints)| Some((value.negate()?, constraints)))
        .collect(),
        ExprKind::Binary { left, op, right } => {
            let left = linear_value_alternatives(
                left,
                return_expression,
                bindings,
                substitutions,
                functions,
                depth,
            )?;
            let right = linear_value_alternatives(
                right,
                return_expression,
                bindings,
                substitutions,
                functions,
                depth,
            )?;
            let mut alternatives = Vec::new();
            for (left, left_constraints) in &left {
                for (right, right_constraints) in &right {
                    let value = match op {
                        BinaryOp::Add => left.clone().add(right.clone()),
                        BinaryOp::Subtract => left.clone().subtract(right.clone()),
                        BinaryOp::Multiply if right.coefficients.is_empty() => {
                            left.clone().scale(right.constant)
                        }
                        BinaryOp::Multiply if left.coefficients.is_empty() => {
                            right.clone().scale(left.constant)
                        }
                        _ => None,
                    }?;
                    alternatives.push((
                        value,
                        left_constraints
                            .iter()
                            .cloned()
                            .chain(right_constraints.iter().cloned())
                            .collect(),
                    ));
                }
            }
            Some(alternatives)
        }
        ExprKind::Call { name, args, .. } => {
            if depth >= 32 {
                return None;
            }
            let function = functions?.get(name)?;
            if args.len() != function.params.len() {
                return None;
            }
            let mut argument_alternatives = vec![(Vec::new(), Vec::new())];
            for argument in args {
                let values = linear_value_alternatives(
                    argument,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth + 1,
                )?;
                argument_alternatives = argument_alternatives
                    .into_iter()
                    .flat_map(|(arguments, constraints)| {
                        values.iter().map(move |(value, value_constraints)| {
                            let mut next_arguments = arguments.clone();
                            next_arguments.push(value.clone());
                            (
                                next_arguments,
                                constraints
                                    .iter()
                                    .cloned()
                                    .chain(value_constraints.iter().cloned())
                                    .collect(),
                            )
                        })
                    })
                    .collect();
            }
            let mut alternatives = Vec::new();
            for (arguments, argument_constraints) in argument_alternatives {
                let argument_values = function
                    .params
                    .iter()
                    .zip(arguments)
                    .map(|(parameter, value)| (parameter.name.clone(), value))
                    .collect::<HashMap<_, _>>();
                let mut invariant_diagnostics = HashMap::new();
                for path in symbolic_return_paths(function, functions, &mut invariant_diagnostics)?
                {
                    let Some(return_expression) = path.expression else {
                        continue;
                    };
                    let mut callee_substitutions = argument_values.clone();
                    for (name, value) in &path.substitutions {
                        callee_substitutions.insert(
                            name.clone(),
                            substitute_linear_value(value, &argument_values)?,
                        );
                    }
                    let mut path_alternatives = vec![argument_constraints.clone()];
                    for guard in &path.guards {
                        let guard_alternatives = constraints_for_guard(
                            guard,
                            Some(return_expression),
                            Some(&path.bindings),
                            Some(&callee_substitutions),
                            functions,
                            depth + 1,
                        )?;
                        path_alternatives =
                            combine_alternatives(path_alternatives, guard_alternatives);
                    }
                    let values = linear_value_alternatives(
                        return_expression,
                        None,
                        Some(&path.bindings),
                        Some(&callee_substitutions),
                        functions,
                        depth + 1,
                    )?;
                    for (value, value_constraints) in values {
                        for constraints in &path_alternatives {
                            alternatives.push((
                                value.clone(),
                                constraints
                                    .iter()
                                    .cloned()
                                    .chain(value_constraints.iter().cloned())
                                    .collect(),
                            ));
                        }
                    }
                }
            }
            if alternatives.is_empty() {
                None
            } else {
                Some(alternatives)
            }
        }
        _ => None,
    }
}

fn symbolic_bool(
    expression: &Expr,
    return_expression: Option<&Expr>,
    bindings: Option<&HashMap<String, &Expr>>,
    substitutions: Option<&HashMap<String, LinearValue>>,
    functions: Option<&HashMap<String, &Function>>,
    depth: usize,
) -> Option<bool> {
    match &expression.kind {
        ExprKind::Unary {
            op: UnaryOp::Not,
            expr,
        } => symbolic_bool(
            expr,
            return_expression,
            bindings,
            substitutions,
            functions,
            depth,
        )
        .map(|value| !value),
        ExprKind::Binary { left, op, right } => match op {
            BinaryOp::And => match (
                symbolic_bool(
                    left,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                ),
                symbolic_bool(
                    right,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                ),
            ) {
                (Some(false), _) | (_, Some(false)) => Some(false),
                (Some(true), Some(true)) => Some(true),
                _ => None,
            },
            BinaryOp::Or => match (
                symbolic_bool(
                    left,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                ),
                symbolic_bool(
                    right,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                ),
            ) {
                (Some(true), _) | (_, Some(true)) => Some(true),
                (Some(false), Some(false)) => Some(false),
                _ => None,
            },
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual => {
                let left = linear_value(
                    left,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?;
                let right = linear_value(
                    right,
                    return_expression,
                    bindings,
                    substitutions,
                    functions,
                    depth,
                )?;
                let difference = left.subtract(right)?;
                if !difference.coefficients.is_empty() {
                    return None;
                }
                Some(match op {
                    BinaryOp::Equal => difference.constant == 0,
                    BinaryOp::NotEqual => difference.constant != 0,
                    BinaryOp::Less => difference.constant < 0,
                    BinaryOp::LessEqual => difference.constant <= 0,
                    BinaryOp::Greater => difference.constant > 0,
                    BinaryOp::GreaterEqual => difference.constant >= 0,
                    _ => unreachable!(),
                })
            }
            _ => None,
        },
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ConstantValue {
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
    Char(char),
}

fn constant_value(expression: &Expr) -> Option<ConstantValue> {
    match &expression.kind {
        ExprKind::Int(value) => Some(ConstantValue::Int(*value)),
        ExprKind::UInt(value) => Some(ConstantValue::UInt(*value)),
        ExprKind::Float(value) => Some(ConstantValue::Float(*value)),
        ExprKind::Bool(value) => Some(ConstantValue::Bool(*value)),
        ExprKind::String(value) => Some(ConstantValue::String(value.clone())),
        ExprKind::Char(value) => Some(ConstantValue::Char(*value)),
        ExprKind::Unary { op, expr } => {
            let value = constant_value(expr)?;
            match (op, value) {
                (UnaryOp::Negate, ConstantValue::Int(value)) => {
                    value.checked_neg().map(ConstantValue::Int)
                }
                (UnaryOp::Negate, ConstantValue::Float(value)) => {
                    Some(ConstantValue::Float(-value))
                }
                (UnaryOp::Not, ConstantValue::Bool(value)) => Some(ConstantValue::Bool(!value)),
                _ => None,
            }
        }
        ExprKind::Binary { left, op, right } => {
            let left = constant_value(left)?;
            if *op == BinaryOp::And && matches!(left, ConstantValue::Bool(false)) {
                return Some(ConstantValue::Bool(false));
            }
            if *op == BinaryOp::Or && matches!(left, ConstantValue::Bool(true)) {
                return Some(ConstantValue::Bool(true));
            }
            let right = constant_value(right)?;
            constant_binary(left, *op, right)
        }
        ExprKind::Array(_)
        | ExprKind::Map(_)
        | ExprKind::Record { .. }
        | ExprKind::Lambda { .. }
        | ExprKind::Index { .. }
        | ExprKind::Field { .. }
        | ExprKind::Variable(_)
        | ExprKind::FunctionValue(_)
        | ExprKind::Call { .. }
        | ExprKind::Sql { .. }
        | ExprKind::Await(_)
        | ExprKind::Match { .. } => None,
    }
}

fn constant_binary(
    left: ConstantValue,
    op: BinaryOp,
    right: ConstantValue,
) -> Option<ConstantValue> {
    use BinaryOp::*;
    match (left, op, right) {
        (ConstantValue::Int(left), Add, ConstantValue::Int(right)) => {
            left.checked_add(right).map(ConstantValue::Int)
        }
        (ConstantValue::Int(left), Subtract, ConstantValue::Int(right)) => {
            left.checked_sub(right).map(ConstantValue::Int)
        }
        (ConstantValue::Int(left), Multiply, ConstantValue::Int(right)) => {
            left.checked_mul(right).map(ConstantValue::Int)
        }
        (ConstantValue::Int(left), Divide, ConstantValue::Int(right)) if right != 0 => {
            left.checked_div(right).map(ConstantValue::Int)
        }
        (ConstantValue::Int(left), Remainder, ConstantValue::Int(right)) if right != 0 => {
            left.checked_rem(right).map(ConstantValue::Int)
        }
        (ConstantValue::Float(left), Add, ConstantValue::Float(right)) => {
            Some(ConstantValue::Float(left + right))
        }
        (ConstantValue::Float(left), Subtract, ConstantValue::Float(right)) => {
            Some(ConstantValue::Float(left - right))
        }
        (ConstantValue::Float(left), Multiply, ConstantValue::Float(right)) => {
            Some(ConstantValue::Float(left * right))
        }
        (ConstantValue::Float(left), Divide, ConstantValue::Float(right)) if right != 0.0 => {
            Some(ConstantValue::Float(left / right))
        }
        (ConstantValue::String(left), Add, ConstantValue::String(right)) => {
            Some(ConstantValue::String(left + &right))
        }
        (left, Equal, right) => Some(ConstantValue::Bool(left == right)),
        (left, NotEqual, right) => Some(ConstantValue::Bool(left != right)),
        (ConstantValue::Int(left), Less, ConstantValue::Int(right)) => {
            Some(ConstantValue::Bool(left < right))
        }
        (ConstantValue::Int(left), LessEqual, ConstantValue::Int(right)) => {
            Some(ConstantValue::Bool(left <= right))
        }
        (ConstantValue::Int(left), Greater, ConstantValue::Int(right)) => {
            Some(ConstantValue::Bool(left > right))
        }
        (ConstantValue::Int(left), GreaterEqual, ConstantValue::Int(right)) => {
            Some(ConstantValue::Bool(left >= right))
        }
        (ConstantValue::Float(left), Less, ConstantValue::Float(right)) => {
            Some(ConstantValue::Bool(left < right))
        }
        (ConstantValue::Float(left), LessEqual, ConstantValue::Float(right)) => {
            Some(ConstantValue::Bool(left <= right))
        }
        (ConstantValue::Float(left), Greater, ConstantValue::Float(right)) => {
            Some(ConstantValue::Bool(left > right))
        }
        (ConstantValue::Float(left), GreaterEqual, ConstantValue::Float(right)) => {
            Some(ConstantValue::Bool(left >= right))
        }
        (ConstantValue::Bool(left), And, ConstantValue::Bool(right)) => {
            Some(ConstantValue::Bool(left && right))
        }
        (ConstantValue::Bool(left), Or, ConstantValue::Bool(right)) => {
            Some(ConstantValue::Bool(left || right))
        }
        _ => None,
    }
}

pub fn check_capabilities(program: &Program) -> Result<(), Vec<CapabilityError>> {
    check_capabilities_with_grants(program, None)
}

pub fn check_capabilities_with_grants(
    program: &Program,
    grants: Option<&HashSet<String>>,
) -> Result<(), Vec<CapabilityError>> {
    let functions = program
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function))
        .collect::<HashMap<_, _>>();
    let known = KNOWN_CAPABILITIES.iter().copied().collect::<HashSet<_>>();
    let mut errors = Vec::new();

    for function in &program.functions {
        let mut declared = HashSet::new();
        for capability in &function.capabilities {
            if !known.contains(capability.as_str()) {
                errors.push(CapabilityError {
                    message: format!(
                        "unknown capability `{capability}`; expected one of {}",
                        KNOWN_CAPABILITIES.join(", ")
                    ),
                    span: function.span,
                });
            } else if !declared.insert(capability.as_str()) {
                errors.push(CapabilityError {
                    message: format!("capability `{capability}` is declared more than once"),
                    span: function.span,
                });
            } else if grants.is_some_and(|grants| !project_grants_capability(grants, capability)) {
                errors.push(CapabilityError {
                    message: format!(
                        "capability `{capability}` is not enabled by the project; set `{}` to true in zelyra.toml",
                        capability_config_key(capability)
                    ),
                    span: function.span,
                });
            }
        }
        check_capability_block(&function.body, function, &functions, &declared, &mut errors);
        let mut function_values = HashMap::new();
        for parameter in &function.params {
            if let Type::Function { capabilities, .. } = &parameter.ty {
                function_values.insert(
                    parameter.name.clone(),
                    capabilities.iter().cloned().collect::<HashSet<_>>(),
                );
            }
        }
        check_indirect_capability_block(
            &function.body,
            function,
            &functions,
            &declared,
            &mut function_values,
            &mut errors,
        );
        for contract in function.requires.iter().chain(&function.ensures) {
            check_capability_expr(contract, function, &functions, &declared, &mut errors);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn function_value_capabilities(
    expression: &Expr,
    values: &HashMap<String, HashSet<String>>,
    functions: &HashMap<&str, &Function>,
) -> Option<HashSet<String>> {
    match &expression.kind {
        ExprKind::FunctionValue(name) => functions
            .get(name.as_str())
            .map(|function| function.capabilities.iter().cloned().collect()),
        ExprKind::Variable(name) => values.get(name).cloned(),
        ExprKind::Call { name, .. } => functions
            .get(name.as_str())
            .and_then(|function| function.return_type.as_ref())
            .and_then(|ty| match ty {
                Type::Function { capabilities, .. } => Some(capabilities.iter().cloned().collect()),
                _ => None,
            }),
        ExprKind::Match { arms, .. } => {
            let mut found = false;
            let mut capabilities = HashSet::new();
            for arm in arms {
                if let Some(arm_capabilities) =
                    function_value_capabilities(&arm.value, values, functions)
                {
                    found = true;
                    capabilities.extend(arm_capabilities);
                }
            }
            found.then_some(capabilities)
        }
        _ => None,
    }
}

fn check_indirect_capability_block(
    block: &Block,
    function: &Function,
    functions: &HashMap<&str, &Function>,
    declared: &HashSet<&str>,
    values: &mut HashMap<String, HashSet<String>>,
    errors: &mut Vec<CapabilityError>,
) {
    for statement in &block.statements {
        match statement {
            Stmt::Let {
                name, ty, value, ..
            } => {
                check_indirect_capability_expr(value, function, declared, values, errors);
                let declared_effects = ty.as_ref().and_then(|ty| match ty {
                    Type::Function { capabilities, .. } => {
                        Some(capabilities.iter().cloned().collect())
                    }
                    _ => None,
                });
                if let Some(capabilities) = declared_effects
                    .or_else(|| function_value_capabilities(value, values, functions))
                {
                    values.insert(name.clone(), capabilities);
                }
            }
            Stmt::BindOrAssign { name, value, .. } => {
                check_indirect_capability_expr(value, function, declared, values, errors);
                if let Some(capabilities) = function_value_capabilities(value, values, functions) {
                    values.entry(name.clone()).or_default().extend(capabilities);
                } else {
                    values.remove(name);
                }
            }
            Stmt::TupleBindOrAssign { names, value, .. } => {
                check_indirect_capability_expr(value, function, declared, values, errors);
                for name in names {
                    values.remove(name);
                }
            }
            Stmt::Expr(expression)
            | Stmt::Return {
                value: Some(expression),
                ..
            } => {
                check_indirect_capability_expr(expression, function, declared, values, errors);
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                check_indirect_capability_expr(condition, function, declared, values, errors);
                let before = values.clone();
                let mut then_values = before.clone();
                check_indirect_capability_block(
                    then_block,
                    function,
                    functions,
                    declared,
                    &mut then_values,
                    errors,
                );
                let mut else_values = before.clone();
                if let Some(else_block) = else_block {
                    check_indirect_capability_block(
                        else_block,
                        function,
                        functions,
                        declared,
                        &mut else_values,
                        errors,
                    );
                }
                *values = merge_function_value_capabilities(&before, &[then_values, else_values]);
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                check_indirect_capability_expr(condition, function, declared, values, errors);
                for invariant in invariants {
                    check_indirect_capability_expr(invariant, function, declared, values, errors);
                }
                let before = values.clone();
                let mut body_values = before.clone();
                check_indirect_capability_block(
                    body,
                    function,
                    functions,
                    declared,
                    &mut body_values,
                    errors,
                );
                *values = merge_function_value_capabilities(&before, &[body_values]);
            }
            Stmt::For { iterable, body, .. } => {
                check_indirect_capability_expr(iterable, function, declared, values, errors);
                let before = values.clone();
                let mut body_values = before.clone();
                check_indirect_capability_block(
                    body,
                    function,
                    functions,
                    declared,
                    &mut body_values,
                    errors,
                );
                *values = merge_function_value_capabilities(&before, &[body_values]);
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    check_indirect_capability_expr(invariant, function, declared, values, errors);
                }
                let before = values.clone();
                let mut body_values = before.clone();
                check_indirect_capability_block(
                    body,
                    function,
                    functions,
                    declared,
                    &mut body_values,
                    errors,
                );
                *values = merge_function_value_capabilities(&before, &[body_values]);
            }
            Stmt::Match { value, arms, .. } => {
                check_indirect_capability_expr(value, function, declared, values, errors);
                let before = values.clone();
                let mut branches = Vec::new();
                for arm in arms {
                    let mut branch = before.clone();
                    check_indirect_capability_block(
                        &arm.body,
                        function,
                        functions,
                        declared,
                        &mut branch,
                        errors,
                    );
                    branches.push(branch);
                }
                *values = merge_function_value_capabilities(&before, &branches);
            }
            Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
                let before = values.clone();
                let mut nested = before.clone();
                check_indirect_capability_block(
                    body,
                    function,
                    functions,
                    declared,
                    &mut nested,
                    errors,
                );
                *values = merge_function_value_capabilities(&before, &[nested]);
            }
            Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }
}

fn merge_function_value_capabilities(
    before: &HashMap<String, HashSet<String>>,
    branches: &[HashMap<String, HashSet<String>>],
) -> HashMap<String, HashSet<String>> {
    let mut merged = before.clone();
    for name in before.keys() {
        for branch in branches {
            if let Some(capabilities) = branch.get(name) {
                merged
                    .entry(name.clone())
                    .or_default()
                    .extend(capabilities.iter().cloned());
            }
        }
    }
    merged
}

fn check_indirect_capability_expr(
    expression: &Expr,
    function: &Function,
    declared: &HashSet<&str>,
    values: &HashMap<String, HashSet<String>>,
    errors: &mut Vec<CapabilityError>,
) {
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match &expression.kind {
            ExprKind::Call { name, args, .. } => {
                if let Some(capabilities) = values.get(name) {
                    for capability in capabilities {
                        require_capability(
                            capability,
                            &format!("indirect call through `{name}`"),
                            expression.span,
                            function,
                            declared,
                            errors,
                        );
                    }
                }
                pending.extend(args);
            }
            ExprKind::Unary { expr, .. } | ExprKind::Await(expr) => pending.push(expr),
            ExprKind::Binary { left, right, .. } => {
                pending.push(left);
                pending.push(right);
            }
            ExprKind::Array(values) => pending.extend(values),
            ExprKind::Map(entries) => {
                for (key, value) in entries {
                    pending.push(key);
                    pending.push(value);
                }
            }
            ExprKind::Record { fields, .. } => {
                pending.extend(fields.iter().map(|(_, value)| value));
            }
            ExprKind::Index { target, index } => {
                pending.push(target);
                pending.push(index);
            }
            ExprKind::Field { target, .. } => pending.push(target),
            ExprKind::Lambda { body, .. } => pending.push(body),
            ExprKind::Match { value, arms } => {
                pending.push(value);
                pending.extend(arms.iter().map(|arm| &arm.value));
            }
            ExprKind::Sql { .. }
            | ExprKind::Int(_)
            | ExprKind::UInt(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::String(_)
            | ExprKind::Char(_)
            | ExprKind::FunctionValue(_)
            | ExprKind::Variable(_) => {}
        }
    }
}

fn check_capability_block(
    block: &Block,
    function: &Function,
    functions: &HashMap<&str, &Function>,
    declared: &HashSet<&str>,
    errors: &mut Vec<CapabilityError>,
) {
    for statement in &block.statements {
        match statement {
            Stmt::Let { value, .. }
            | Stmt::BindOrAssign { value, .. }
            | Stmt::TupleBindOrAssign { value, .. }
            | Stmt::Expr(value)
            | Stmt::Return {
                value: Some(value), ..
            }
            | Stmt::Match { value, .. } => {
                check_capability_expr(value, function, functions, declared, errors);
                if let Stmt::Match { arms, .. } = statement {
                    for arm in arms {
                        check_capability_block(&arm.body, function, functions, declared, errors);
                    }
                }
            }
            Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                check_capability_expr(condition, function, functions, declared, errors);
                check_capability_block(then_block, function, functions, declared, errors);
                if let Some(else_block) = else_block {
                    check_capability_block(else_block, function, functions, declared, errors);
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                check_capability_expr(condition, function, functions, declared, errors);
                for invariant in invariants {
                    check_capability_expr(invariant, function, functions, declared, errors);
                }
                check_capability_block(body, function, functions, declared, errors);
            }
            Stmt::For { iterable, body, .. } => {
                check_capability_expr(iterable, function, functions, declared, errors);
                check_capability_block(body, function, functions, declared, errors);
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    check_capability_expr(invariant, function, functions, declared, errors);
                }
                check_capability_block(body, function, functions, declared, errors);
            }
            Stmt::Transaction { body, .. } => {
                check_capability_block(body, function, functions, declared, errors);
            }
            Stmt::Parallel { body, .. } => {
                check_capability_block(body, function, functions, declared, errors);
            }
        }
    }
}

fn check_capability_expr(
    expression: &Expr,
    function: &Function,
    functions: &HashMap<&str, &Function>,
    declared: &HashSet<&str>,
    errors: &mut Vec<CapabilityError>,
) {
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match &expression.kind {
            ExprKind::Sql { query, .. } => {
                let capability = sql_required_capability(query);
                require_capability(
                    capability,
                    "SQL access",
                    expression.span,
                    function,
                    declared,
                    errors,
                );
            }
            ExprKind::Call { name, args, .. } => {
                if name == "read_console" {
                    require_capability(
                        "Console",
                        "console input",
                        expression.span,
                        function,
                        declared,
                        errors,
                    );
                } else if name == "now" {
                    require_capability(
                        "Clock",
                        "clock access",
                        expression.span,
                        function,
                        declared,
                        errors,
                    );
                } else if name == "env" {
                    require_capability(
                        "Environment",
                        "environment access",
                        expression.span,
                        function,
                        declared,
                        errors,
                    );
                } else if name == "random_int" || name == "uuid_v4" {
                    require_capability(
                        "Random",
                        "random access",
                        expression.span,
                        function,
                        declared,
                        errors,
                    );
                } else if matches!(
                    name.as_str(),
                    "http_get" | "http_request" | "http_json" | "http_result"
                ) {
                    require_capability(
                        "Network",
                        "network access",
                        expression.span,
                        function,
                        declared,
                        errors,
                    );
                } else if name == "mail_queue" {
                    require_capability(
                        "Email",
                        "email delivery",
                        expression.span,
                        function,
                        declared,
                        errors,
                    );
                    require_capability(
                        "Database(write)",
                        "transactional mail queue write",
                        expression.span,
                        function,
                        declared,
                        errors,
                    );
                } else if name == "mail_deliver" {
                    require_capability(
                        "Email",
                        "email delivery",
                        expression.span,
                        function,
                        declared,
                        errors,
                    );
                } else if name == "run_process" {
                    require_capability(
                        "Process",
                        "process execution",
                        expression.span,
                        function,
                        declared,
                        errors,
                    );
                } else if matches!(
                    name.as_str(),
                    "read_text" | "read_text_result" | "file_exists" | "dir_exists" | "file_size"
                ) {
                    require_capability(
                        "FileSystem",
                        "file-system read access",
                        expression.span,
                        function,
                        declared,
                        errors,
                    );
                } else if matches!(
                    name.as_str(),
                    "write_text"
                        | "write_text_result"
                        | "write_text_new_result"
                        | "atomic_write_text_result"
                        | "append_text_result"
                        | "copy_file_new_result"
                        | "rename_file_result"
                ) {
                    require_capability(
                        "FileSystem",
                        "file-system write access",
                        expression.span,
                        function,
                        declared,
                        errors,
                    );
                } else if name == "delete_file" || name == "delete_file_result" {
                    require_capability(
                        "FileSystem",
                        "file-system delete access",
                        expression.span,
                        function,
                        declared,
                        errors,
                    );
                } else if name == "list_dir" {
                    require_capability(
                        "FileSystem",
                        "file-system read access",
                        expression.span,
                        function,
                        declared,
                        errors,
                    );
                }
                if let Some(callee) = functions.get(name.as_str()) {
                    for capability in &callee.capabilities {
                        if KNOWN_CAPABILITIES.contains(&capability.as_str())
                            && !function_declares_capability(declared, capability)
                        {
                            errors.push(CapabilityError {
                            message: format!(
                                "function `{}` calls `{name}`, which requires capability `{capability}`; declare `uses {capability}`",
                                function.name
                            ),
                            span: expression.span,
                        });
                        }
                    }
                }
                for argument in args {
                    pending.push(argument);
                }
            }
            ExprKind::Unary { expr, .. } => {
                pending.push(expr);
            }
            ExprKind::Binary { left, right, .. } => {
                pending.push(left);
                pending.push(right);
            }
            ExprKind::Array(values) => {
                for value in values {
                    pending.push(value);
                }
            }
            ExprKind::Map(entries) => {
                for (key, value) in entries {
                    pending.push(key);
                    pending.push(value);
                }
            }
            ExprKind::Record { fields, .. } => {
                for (_, value) in fields {
                    pending.push(value);
                }
            }
            ExprKind::Index { target, index } => {
                pending.push(target);
                pending.push(index);
            }
            ExprKind::Field { target, .. } => {
                pending.push(target);
            }
            ExprKind::Lambda { body, .. } => pending.push(body),
            ExprKind::Await(inner) => {
                pending.push(inner);
            }
            ExprKind::Match { value, arms } => {
                pending.push(value);
                for arm in arms {
                    pending.push(&arm.value);
                }
            }
            ExprKind::Int(..)
            | ExprKind::UInt(..)
            | ExprKind::Float(..)
            | ExprKind::Bool(..)
            | ExprKind::String(..)
            | ExprKind::Char(..)
            | ExprKind::Variable(..)
            | ExprKind::FunctionValue(..) => {}
        }
    }
}

fn require_capability(
    capability: &str,
    operation: &str,
    span: Span,
    function: &Function,
    declared: &HashSet<&str>,
    errors: &mut Vec<CapabilityError>,
) {
    if !function_declares_capability(declared, capability) {
        errors.push(CapabilityError {
            message: format!(
                "function `{}` uses {operation} but does not declare capability `{capability}`; add `uses {capability}`",
                function.name
            ),
            span,
        });
    }
}

fn function_declares_capability(declared: &HashSet<&str>, capability: &str) -> bool {
    declared.contains(capability)
        || (capability.starts_with("Database(") && declared.contains("Database"))
}

fn project_grants_capability(grants: &HashSet<String>, capability: &str) -> bool {
    if grants.contains(capability)
        || (capability.starts_with("Database(") && grants.contains("Database"))
    {
        return true;
    }
    capability == "Database"
        && grants.contains("Database(read)")
        && grants.contains("Database(write)")
}

fn capability_config_key(capability: &str) -> String {
    match capability {
        "Database(read)" => "database_read".into(),
        "Database(write)" => "database_write".into(),
        _ => capability.to_ascii_lowercase(),
    }
}

fn sql_required_capability(query: &str) -> &'static str {
    let command = zelyra_database::sql::analyze_table_access(query, &[]).0;
    match command {
        Some("SELECT") => {
            let upper = query.to_ascii_uppercase();
            if upper.contains("FOR UPDATE")
                || upper.contains("INTO OUTFILE")
                || upper.contains("INTO DUMPFILE")
            {
                "Database(write)"
            } else {
                "Database(read)"
            }
        }
        Some("INSERT" | "UPDATE" | "DELETE") => "Database(write)",
        _ => "Database",
    }
}

#[derive(Clone)]
struct Variable {
    ty: Type,
    mutable: bool,
}

#[derive(Clone)]
struct FunctionSignature {
    type_params: Vec<String>,
    params: Vec<Type>,
    return_type: Type,
    capabilities: Vec<String>,
}

struct Checker<'a> {
    type_aliases: HashMap<String, Type>,
    functions: HashMap<String, FunctionSignature>,
    known_types: std::collections::HashSet<String>,
    generic_arities: HashMap<String, usize>,
    active_type_parameters: HashSet<String>,
    errors: Vec<TypeError>,
    loop_depth: usize,
    parallel_depth: usize,
    transaction_depth: usize,
    _program: &'a Program,
}

pub fn check(program: &Program) -> Result<(), Vec<TypeError>> {
    let mut known_types = [
        "Id",
        "Int",
        "UInt",
        "Float",
        "Decimal",
        "Bool",
        "String",
        "Char",
        "Bytes",
        "Timestamp",
        "Date",
        "Time",
        "Duration",
        "Unit",
        "HttpResponse",
        "HttpError",
        "HttpResult",
    ]
    .into_iter()
    .map(String::from)
    .collect::<std::collections::HashSet<_>>();
    let mut errors = Vec::new();
    for definition in &program.types {
        if !known_types.insert(definition.name.clone()) {
            errors.push(TypeError {
                code: "E-TYPE-001",
                message: format!("duplicate type `{}`", definition.name),
                span: definition.span,
            });
        }
    }
    for record in &program.records {
        if !known_types.insert(record.name.clone()) {
            errors.push(TypeError {
                code: "E-TYPE-001",
                message: format!("duplicate type `{}`", record.name),
                span: record.span,
            });
        }
    }
    for definition in &program.enums {
        if !known_types.insert(definition.name.clone()) {
            errors.push(TypeError {
                code: "E-TYPE-001",
                message: format!("duplicate type `{}`", definition.name),
                span: definition.span,
            });
        }
    }
    // Schema scalar domains also work in function/decoder annotations. Keep
    // existing explicit definitions authoritative rather than reserving names.
    known_types.extend(["Money", "Email", "Url", "Uuid"].map(String::from));
    let generic_arities = program
        .records
        .iter()
        .map(|record| (record.name.clone(), record.type_params.len()))
        .chain(
            program
                .enums
                .iter()
                .map(|definition| (definition.name.clone(), definition.type_params.len())),
        )
        .collect::<HashMap<_, _>>();
    for table in &program.tables {
        known_types.insert(table.name.clone());
        if let Some(singular) = singular_table_type(&table.name) {
            known_types.insert(singular);
        }
    }
    let aliases: HashMap<_, _> = program
        .types
        .iter()
        .map(|definition| (definition.name.clone(), definition.target.clone()))
        .collect();
    for definition in &program.types {
        if let Err(message) = expand_type_aliases(&Type::Named(definition.name.clone()), &aliases) {
            errors.push(TypeError {
                code: "E-TYPE-ALIAS-001",
                message: message.into(),
                span: definition.span,
            });
        }
    }
    for definition in &program.types {
        validate_type(
            &definition.target,
            &known_types,
            &generic_arities,
            &mut errors,
            definition.span,
        );
    }
    for record in &program.records {
        let mut fields = HashSet::new();
        let mut record_types = known_types.clone();
        record_types.extend(record.type_params.iter().cloned());
        for parameter in &record.type_params {
            if known_types.contains(parameter) {
                errors.push(TypeError {
                    code: "E-TYPE-GENERIC-001",
                    message: format!(
                        "generic type parameter `{parameter}` shadows a declared type"
                    ),
                    span: record.span,
                });
            }
        }
        for field in &record.fields {
            if !fields.insert(field.name.clone()) {
                errors.push(TypeError {
                    code: "E-TYPE-001",
                    message: format!(
                        "duplicate field `{}` in record `{}`",
                        field.name, record.name
                    ),
                    span: field.span,
                });
            }
            validate_type(
                &field.ty,
                &record_types,
                &generic_arities,
                &mut errors,
                field.span,
            );
        }
    }
    for definition in &program.enums {
        let mut enum_types = known_types.clone();
        enum_types.extend(definition.type_params.iter().cloned());
        for parameter in &definition.type_params {
            if known_types.contains(parameter) {
                errors.push(TypeError {
                    code: "E-TYPE-GENERIC-001",
                    message: format!(
                        "generic type parameter `{parameter}` shadows a declared type"
                    ),
                    span: definition.span,
                });
            }
        }
        for (_, payload_types) in &definition.variant_payloads {
            for ty in payload_types {
                validate_type(
                    ty,
                    &enum_types,
                    &generic_arities,
                    &mut errors,
                    definition.span,
                );
            }
        }
    }
    let mut functions = HashMap::new();
    for function in &program.functions {
        if functions.contains_key(&function.name) {
            errors.push(TypeError {
                code: "E-TYPE-001",
                message: format!("duplicate function `{}`", function.name),
                span: function.span,
            });
        } else {
            functions.insert(
                function.name.clone(),
                FunctionSignature {
                    type_params: function.type_params.clone(),
                    params: function
                        .params
                        .iter()
                        .map(|param| param.ty.clone())
                        .collect(),
                    return_type: function.return_type.clone().unwrap_or(Type::Unknown),
                    capabilities: function.capabilities.clone(),
                },
            );
        }
    }
    if !functions.contains_key("main") {
        errors.push(TypeError {
            code: "E-TYPE-001",
            message: "program must define `fn main()`".into(),
            span: Span::default(),
        });
    }
    let mut checker = Checker {
        type_aliases: program
            .types
            .iter()
            .map(|definition| (definition.name.clone(), definition.target.clone()))
            .collect(),
        functions,
        known_types,
        generic_arities,
        active_type_parameters: HashSet::new(),
        errors,
        loop_depth: 0,
        parallel_depth: 0,
        transaction_depth: 0,
        _program: program,
    };
    for function in &program.functions {
        checker.check_function(function);
    }
    if checker.errors.is_empty() {
        Ok(())
    } else {
        Err(checker.errors)
    }
}

const FLOW_FALLTHROUGH: u8 = 1;
const FLOW_RETURN: u8 = 2;
const FLOW_BREAK: u8 = 4;
const FLOW_CONTINUE: u8 = 8;

fn flow_outcomes(block: &Block, program: &Program) -> u8 {
    let mut outcomes = FLOW_FALLTHROUGH;
    for statement in &block.statements {
        if outcomes & FLOW_FALLTHROUGH == 0 {
            break;
        }
        outcomes = (outcomes & !FLOW_FALLTHROUGH) | statement_flow(statement, program);
    }
    outcomes
}

fn statement_flow(statement: &Stmt, program: &Program) -> u8 {
    match statement {
        Stmt::Return { .. } => FLOW_RETURN,
        Stmt::Break { .. } => FLOW_BREAK,
        Stmt::Continue { .. } => FLOW_CONTINUE,
        Stmt::If {
            then_block,
            else_block,
            ..
        } => {
            flow_outcomes(then_block, program)
                | else_block
                    .as_ref()
                    .map(|block| flow_outcomes(block, program))
                    .unwrap_or(FLOW_FALLTHROUGH)
        }
        Stmt::Match { arms, .. } => {
            let mut outcomes = arms
                .iter()
                .map(|arm| flow_outcomes(&arm.body, program))
                .fold(0, |all, arm| all | arm);
            if !match_arms_are_exhaustive(arms, program) {
                outcomes |= FLOW_FALLTHROUGH;
            }
            outcomes
        }
        Stmt::Loop { body, .. } => {
            let body_outcomes = flow_outcomes(body, program);
            let mut outcomes = body_outcomes & FLOW_RETURN;
            if body_outcomes & FLOW_BREAK != 0 {
                outcomes |= FLOW_FALLTHROUGH;
            }
            outcomes
        }
        Stmt::While {
            condition, body, ..
        } => {
            let body_outcomes = flow_outcomes(body, program);
            let mut outcomes = body_outcomes & FLOW_RETURN;
            if !matches!(condition.kind, ExprKind::Bool(true)) || body_outcomes & FLOW_BREAK != 0 {
                outcomes |= FLOW_FALLTHROUGH;
            }
            outcomes
        }
        Stmt::For { body, .. } => FLOW_FALLTHROUGH | (flow_outcomes(body, program) & FLOW_RETURN),
        Stmt::Transaction { body, .. } => flow_outcomes(body, program),
        Stmt::Let { .. }
        | Stmt::BindOrAssign { .. }
        | Stmt::TupleBindOrAssign { .. }
        | Stmt::Expr(_)
        | Stmt::Parallel { .. } => FLOW_FALLTHROUGH,
    }
}

fn match_arms_are_exhaustive(arms: &[MatchArm], program: &Program) -> bool {
    let mut covered = HashSet::new();
    let mut enum_covered = HashSet::new();
    for arm in arms {
        if arm.guard.is_some() {
            continue;
        }
        match &arm.pattern.kind {
            PatternKind::Wildcard | PatternKind::Variable(_) => return true,
            PatternKind::Bool(value) => {
                covered.insert(if *value { "true" } else { "false" });
            }
            PatternKind::Constructor { name, inner } => {
                let irrefutable_inner = inner.as_deref().is_none_or(|inner| {
                    matches!(inner.kind, PatternKind::Wildcard | PatternKind::Variable(_))
                });
                if irrefutable_inner {
                    covered.insert(name.as_str());
                }
            }
            PatternKind::EnumVariant { enum_name, variant } => {
                enum_covered.insert(format!("{enum_name}::{variant}"));
            }
            PatternKind::EnumVariantPayload {
                enum_name,
                variant,
                payloads,
            } => {
                if payloads.iter().all(|payload| {
                    matches!(
                        payload.kind,
                        PatternKind::Wildcard | PatternKind::Variable(_)
                    )
                }) {
                    enum_covered.insert(format!("{enum_name}::{variant}"));
                }
            }
            PatternKind::Int(_)
            | PatternKind::String(_)
            | PatternKind::Char(_)
            | PatternKind::Record { .. } => {}
        }
    }
    (covered.contains("true") && covered.contains("false"))
        || (covered.contains("Some") && covered.contains("None"))
        || (covered.contains("Ok") && covered.contains("Err"))
        || program.enums.iter().any(|definition| {
            definition
                .variants
                .iter()
                .all(|variant| enum_covered.contains(&format!("{}::{variant}", definition.name)))
        })
}

fn instantiated_enum_payload_types(
    definition: &zelyra_ast::EnumDef,
    type_args: &[Type],
    variant: &str,
) -> Option<Vec<Type>> {
    let substitutions = definition
        .type_params
        .iter()
        .zip(type_args)
        .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
        .collect::<HashMap<_, _>>();
    definition
        .variant_payloads
        .iter()
        .find(|(name, _)| name == variant)
        .map(|(_, payloads)| {
            payloads
                .iter()
                .map(|ty| substitute_generic_type(ty, &substitutions))
                .collect()
        })
}

impl<'a> Checker<'a> {
    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.error_with_code("E-TYPE-001", span, message);
    }
    fn error_with_code(&mut self, code: &'static str, span: Span, message: impl Into<String>) {
        self.errors.push(TypeError {
            code,
            message: message.into(),
            span,
        });
    }

    fn check_enum_constructor(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[Expr],
        span: Span,
        scopes: &[HashMap<String, Variable>],
    ) -> Option<Type> {
        let (enum_name, variant) = name.rsplit_once("::")?;
        let definition = self
            ._program
            .enums
            .iter()
            .find(|definition| definition.name == enum_name)?;
        let payload_types = definition
            .variant_payloads
            .iter()
            .find(|(name, _)| name == variant)
            .map(|(_, types)| types.clone());
        let Some(payload_types) = payload_types else {
            for argument in args {
                self.check_expr(argument, scopes);
            }
            self.error_with_code(
                "E-TYPE-ENUM-001",
                span,
                format!("unknown enum variant `{enum_name}::{variant}`"),
            );
            return Some(Type::Unknown);
        };
        if definition.type_params.is_empty() && !type_args.is_empty() {
            self.error_with_code(
                "E-TYPE-GENERIC-001",
                span,
                format!("enum `{enum_name}` does not accept type arguments"),
            );
        }
        if !definition.type_params.is_empty() && !type_args.is_empty() {
            if type_args.len() != definition.type_params.len() {
                self.error_with_code(
                    "E-TYPE-GENERIC-001",
                    span,
                    format!(
                        "enum `{enum_name}` expects {} type argument(s), found {}",
                        definition.type_params.len(),
                        type_args.len()
                    ),
                );
            }
            for argument in type_args {
                self.check_type(argument, span);
            }
        }
        if payload_types.is_empty() && definition.type_params.is_empty() {
            self.error_with_code(
                "E-TYPE-ENUM-001",
                span,
                format!(
                    "unit enum variant `{enum_name}::{variant}` is written without parentheses"
                ),
            );
        } else if args.len() != payload_types.len() {
            self.error_with_code(
                "E-TYPE-ENUM-001",
                span,
                format!(
                    "enum variant `{enum_name}::{variant}` expects {} payload value(s), found {}",
                    payload_types.len(),
                    args.len()
                ),
            );
        }
        let actual_types = args
            .iter()
            .map(|argument| self.check_expr(argument, scopes))
            .collect::<Vec<_>>();
        let generic_parameters = definition
            .type_params
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        let mut substitutions = definition
            .type_params
            .iter()
            .zip(type_args)
            .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
            .collect::<HashMap<_, _>>();
        if type_args.is_empty() {
            for (expected, actual) in payload_types.iter().zip(&actual_types) {
                infer_generic_types(expected, actual, &generic_parameters, &mut substitutions);
            }
        }
        for parameter in &definition.type_params {
            if !substitutions.contains_key(parameter) {
                self.error_with_code(
                    "E-TYPE-GENERIC-001",
                    span,
                    format!(
                        "cannot infer type parameter `{parameter}` for enum `{enum_name}`; provide explicit type arguments"
                    ),
                );
                substitutions.insert(parameter.clone(), Type::Unknown);
            }
        }
        for ((argument, actual), expected) in
            args.iter().zip(actual_types).zip(payload_types.iter())
        {
            let expected = substitute_generic_type(expected, &substitutions);
            self.expect_type(&expected, &actual, argument.span);
        }
        if definition.type_params.is_empty() {
            Some(Type::Named(enum_name.to_owned()))
        } else {
            Some(Type::Applied {
                name: enum_name.to_owned(),
                args: definition
                    .type_params
                    .iter()
                    .map(|parameter| {
                        substitutions
                            .get(parameter)
                            .cloned()
                            .unwrap_or(Type::Unknown)
                    })
                    .collect(),
            })
        }
    }
    fn lookup<'b>(
        &self,
        scopes: &'b [HashMap<String, Variable>],
        name: &str,
    ) -> Option<&'b Variable> {
        scopes.iter().rev().find_map(|scope| scope.get(name))
    }
    fn check_function(&mut self, function: &Function) {
        let previous_type_parameters = std::mem::replace(
            &mut self.active_type_parameters,
            function.type_params.iter().cloned().collect(),
        );
        for parameter in &function.type_params {
            if self.known_types.contains(parameter) {
                self.error_with_code(
                    "E-TYPE-GENERIC-001",
                    function.span,
                    format!("generic type parameter `{parameter}` shadows a declared type"),
                );
            }
        }
        for parameter in &function.params {
            self.check_type(&parameter.ty, parameter.span);
        }
        if let Some(return_type) = &function.return_type {
            self.check_type(return_type, function.span);
        }
        let mut scopes = vec![HashMap::new()];
        for parameter in &function.params {
            if scopes[0]
                .insert(
                    parameter.name.clone(),
                    Variable {
                        ty: parameter.ty.clone(),
                        mutable: false,
                    },
                )
                .is_some()
            {
                self.error(
                    parameter.span,
                    format!("duplicate parameter `{}`", parameter.name),
                );
            }
        }
        let expected = function.return_type.clone().unwrap_or(Type::Unknown);
        for contract in &function.requires {
            let actual = self.check_expr(contract, &scopes);
            self.expect_type(&Type::Bool, &actual, contract.span);
        }
        self.check_block(&function.body, &mut scopes, &expected);
        if !function.ensures.is_empty() {
            scopes[0].insert(
                "result".into(),
                Variable {
                    ty: function.return_type.clone().unwrap_or(Type::Unit),
                    mutable: false,
                },
            );
            for contract in &function.ensures {
                let actual = self.check_expr(contract, &scopes);
                self.expect_type(&Type::Bool, &actual, contract.span);
            }
        }
        if function
            .return_type
            .as_ref()
            .is_some_and(|return_type| *return_type != Type::Unit)
            && flow_outcomes(&function.body, self._program) & FLOW_FALLTHROUGH != 0
        {
            let return_type = function.return_type.as_ref().expect("checked above");
            self.error_with_code(
                "E-TYPE-RETURN-001",
                function.body.span,
                format!(
                    "function `{}` can reach its end without returning `{return_type}`",
                    function.name
                ),
            );
        }
        self.active_type_parameters = previous_type_parameters;
    }
    fn check_block(
        &mut self,
        block: &Block,
        scopes: &mut Vec<HashMap<String, Variable>>,
        expected: &Type,
    ) {
        scopes.push(HashMap::new());
        for statement in &block.statements {
            self.check_statement(statement, scopes, expected);
        }
        scopes.pop();
    }
    fn check_statement(
        &mut self,
        statement: &Stmt,
        scopes: &mut Vec<HashMap<String, Variable>>,
        expected: &Type,
    ) {
        match statement {
            Stmt::Let {
                name,
                ty,
                value,
                mutable,
                span,
            } => {
                let actual = self.check_expr(value, scopes);
                if let Some(declared) = ty {
                    self.check_type(declared, *span);
                    self.expect_type(declared, &actual, value.span);
                }
                let final_ty = ty.clone().unwrap_or(actual);
                if scopes
                    .last_mut()
                    .unwrap()
                    .insert(
                        name.clone(),
                        Variable {
                            ty: final_ty,
                            mutable: *mutable,
                        },
                    )
                    .is_some()
                {
                    self.error(
                        *span,
                        format!("variable `{name}` is already declared in this scope"),
                    );
                }
            }
            Stmt::BindOrAssign { name, value, span } => {
                let actual = self.check_expr(value, scopes);
                if let Some(existing) = self.lookup(scopes, name).cloned() {
                    if !existing.mutable {
                        self.error(
                            *span,
                            format!("cannot assign to immutable variable `{name}`"),
                        );
                    }
                    self.expect_type(&existing.ty, &actual, value.span);
                } else {
                    scopes.last_mut().unwrap().insert(
                        name.clone(),
                        Variable {
                            ty: actual,
                            mutable: false,
                        },
                    );
                }
            }
            Stmt::TupleBindOrAssign { names, value, span } => {
                let actual = self.check_expr(value, scopes);
                let item_types = match actual {
                    Type::Tuple(items) if items.len() == names.len() => items,
                    Type::Tuple(items) => {
                        self.error_with_code(
                            "E-TYPE-TUPLE-001",
                            value.span,
                            format!(
                                "tuple assignment has {} variables but the value has {} items",
                                names.len(),
                                items.len()
                            ),
                        );
                        vec![Type::Unknown; names.len()]
                    }
                    Type::Unknown => vec![Type::Unknown; names.len()],
                    other => {
                        self.error_with_code(
                            "E-TYPE-TUPLE-001",
                            value.span,
                            format!("tuple assignment expects a tuple, found `{other}`"),
                        );
                        vec![Type::Unknown; names.len()]
                    }
                };
                for (name, item_type) in names.iter().zip(item_types) {
                    if let Some(existing) = self.lookup(scopes, name).cloned() {
                        if !existing.mutable {
                            self.error(
                                *span,
                                format!("cannot assign to immutable variable `{name}`"),
                            );
                        }
                        self.expect_type(&existing.ty, &item_type, value.span);
                    } else {
                        scopes.last_mut().unwrap().insert(
                            name.clone(),
                            Variable {
                                ty: item_type,
                                mutable: false,
                            },
                        );
                    }
                }
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr, scopes);
            }
            Stmt::Return { value, span } => {
                let actual = value
                    .as_ref()
                    .map(|expr| self.check_expr(expr, scopes))
                    .unwrap_or(Type::Unit);
                if *expected != Type::Unknown {
                    self.expect_type(expected, &actual, *span);
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let condition_type = self.check_expr(condition, scopes);
                self.expect_type(&Type::Bool, &condition_type, condition.span);
                self.check_block(then_block, scopes, expected);
                if let Some(block) = else_block {
                    self.check_block(block, scopes, expected);
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                let condition_type = self.check_expr(condition, scopes);
                self.expect_type(&Type::Bool, &condition_type, condition.span);
                for invariant in invariants {
                    let invariant_type = self.check_expr(invariant, scopes);
                    self.expect_type(&Type::Bool, &invariant_type, invariant.span);
                }
                self.loop_depth += 1;
                self.check_block(body, scopes, expected);
                self.loop_depth -= 1;
            }
            Stmt::For {
                name,
                additional_names,
                iterable,
                body,
                ..
            } => {
                let iterable_type = self.check_expr(iterable, scopes);
                let element_type = match iterable_type {
                    Type::Array(inner) => *inner,
                    Type::Set(inner) => *inner,
                    Type::Unknown => Type::Unknown,
                    other => {
                        self.error(
                            iterable.span,
                            format!("`for ... in` expects an array or Set, found `{other}`"),
                        );
                        Type::Unknown
                    }
                };
                let binding_count = additional_names.len() + 1;
                let binding_types = if binding_count == 1 {
                    vec![element_type]
                } else {
                    match element_type {
                        Type::Tuple(items) if items.len() == binding_count => items,
                        Type::Tuple(items) => {
                            self.error(
                                iterable.span,
                                format!(
                                    "tuple loop pattern has {binding_count} variables but the value has {} elements",
                                    items.len()
                                ),
                            );
                            vec![Type::Unknown; binding_count]
                        }
                        Type::Unknown => vec![Type::Unknown; binding_count],
                        other => {
                            self.error(
                                iterable.span,
                                format!("tuple loop pattern expects tuple values, found `{other}`"),
                            );
                            vec![Type::Unknown; binding_count]
                        }
                    }
                };
                scopes.push(HashMap::new());
                scopes.last_mut().unwrap().insert(
                    name.clone(),
                    Variable {
                        ty: binding_types[0].clone(),
                        mutable: false,
                    },
                );
                for (name, ty) in additional_names
                    .iter()
                    .zip(binding_types.into_iter().skip(1))
                {
                    scopes
                        .last_mut()
                        .unwrap()
                        .insert(name.clone(), Variable { ty, mutable: false });
                }
                self.loop_depth += 1;
                self.check_block(body, scopes, expected);
                self.loop_depth -= 1;
                scopes.pop();
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    let invariant_type = self.check_expr(invariant, scopes);
                    self.expect_type(&Type::Bool, &invariant_type, invariant.span);
                }
                self.loop_depth += 1;
                self.check_block(body, scopes, expected);
                self.loop_depth -= 1;
            }
            Stmt::Break { span } => {
                if self.loop_depth == 0 {
                    self.error(*span, "`break` is only valid inside a loop");
                }
            }
            Stmt::Continue { span } => {
                if self.loop_depth == 0 {
                    self.error(*span, "`continue` is only valid inside a loop");
                }
            }
            Stmt::Match { value, arms, span } => {
                let value_type = self.check_expr(value, scopes);
                let mut covered = std::collections::HashSet::new();
                for arm in arms {
                    let mut pattern_scope = HashMap::new();
                    self.check_pattern(
                        &arm.pattern,
                        &value_type,
                        &mut pattern_scope,
                        &mut covered,
                        arm.guard.is_none(),
                    );
                    scopes.push(pattern_scope);
                    if let Some(guard) = &arm.guard {
                        let guard_type = self.check_expr(guard, scopes);
                        self.expect_type(&Type::Bool, &guard_type, guard.span);
                    }
                    self.check_block(&arm.body, scopes, expected);
                    scopes.pop();
                }
                self.check_exhaustiveness(&value_type, &covered, *span);
            }
            Stmt::Transaction { body, .. } => {
                self.transaction_depth += 1;
                self.check_block(body, scopes, expected);
                self.transaction_depth -= 1;
            }
            Stmt::Parallel { body, span } => {
                let mut names = HashSet::new();
                let mut new_bindings = Vec::new();
                self.parallel_depth += 1;
                for statement in &body.statements {
                    let Stmt::BindOrAssign { name, value, span } = statement else {
                        self.error(
                            *span,
                            "parallel blocks may contain only `name = await expression` bindings",
                        );
                        continue;
                    };
                    if !matches!(&value.kind, ExprKind::Await(_)) {
                        self.error(value.span, "parallel bindings must await their expression");
                    }
                    if !names.insert(name.clone()) {
                        self.error(
                            *span,
                            format!("parallel binding `{name}` is declared twice"),
                        );
                    }
                    let actual = self.check_expr(value, scopes);
                    if let Some(existing) = self.lookup(scopes, name).cloned() {
                        if !existing.mutable {
                            self.error(
                                *span,
                                format!("cannot assign to immutable variable `{name}`"),
                            );
                        }
                        self.expect_type(&existing.ty, &actual, value.span);
                    } else {
                        new_bindings.push((name.clone(), actual));
                    }
                }
                self.parallel_depth -= 1;
                for (name, ty) in new_bindings {
                    scopes
                        .last_mut()
                        .unwrap()
                        .insert(name, Variable { ty, mutable: false });
                }
            }
        }
    }
    fn check_type(&mut self, ty: &Type, span: Span) {
        match ty {
            Type::Named(name)
                if !self.known_types.contains(name)
                    && !self.active_type_parameters.contains(name) =>
            {
                self.error(span, format!("unknown type `{name}`"));
            }
            Type::Named(name)
                if self
                    .generic_arities
                    .get(name)
                    .is_some_and(|arity| *arity > 0) =>
            {
                let arity = self.generic_arities[name];
                self.error_with_code(
                    "E-TYPE-GENERIC-001",
                    span,
                    format!("generic type `{name}` requires {arity} type argument(s)"),
                );
            }
            Type::Applied { name, args } => {
                if !self.known_types.contains(name) {
                    self.error(span, format!("unknown type `{name}`"));
                }
                if let Some(arity) = self.generic_arities.get(name) {
                    if *arity != args.len() {
                        self.error_with_code(
                            "E-TYPE-GENERIC-001",
                            span,
                            format!(
                                "generic type `{name}` expects {arity} type argument(s), found {}",
                                args.len()
                            ),
                        );
                    }
                } else {
                    self.error_with_code(
                        "E-TYPE-GENERIC-001",
                        span,
                        format!("type `{name}` is not generic and does not accept type arguments"),
                    );
                }
                for argument in args {
                    self.check_type(argument, span);
                }
            }
            Type::Option(inner) => self.check_type(inner, span),
            Type::Result(ok, error) => {
                self.check_type(ok, span);
                self.check_type(error, span);
            }
            Type::Array(inner) => self.check_type(inner, span),
            Type::Set(inner) => {
                self.check_type(inner, span);
                if !matches!(inner.as_ref(), Type::Unknown) && !is_map_key_type(inner) {
                    self.error(
                        span,
                        format!("Set elements must be scalar comparable types; found `{inner}`"),
                    );
                }
            }
            Type::Tuple(items) => {
                for item in items {
                    self.check_type(item, span);
                }
            }
            Type::Map(key, value) => {
                self.check_type(key, span);
                self.check_type(value, span);
                if !matches!(key.as_ref(), Type::Unknown) && !is_map_key_type(key) {
                    self.error(
                        span,
                        format!(
                            "Map keys must be Int, UInt, String, Bool, Char, or Timestamp; found `{key}`"
                        ),
                    );
                }
            }
            Type::HttpResult(inner) => self.check_type(inner, span),
            Type::Function {
                params,
                return_type,
                capabilities,
            } => {
                for parameter in params {
                    self.check_type(parameter, span);
                }
                self.check_type(return_type, span);
                for capability in capabilities {
                    if !KNOWN_CAPABILITIES.contains(&capability.as_str()) {
                        self.error_with_code(
                            "E-TYPE-FUNCTION-001",
                            span,
                            format!("unknown capability `{capability}` in function type"),
                        );
                    }
                }
            }
            _ => {}
        }
    }
    fn check_pattern(
        &mut self,
        pattern: &Pattern,
        expected: &Type,
        scope: &mut HashMap<String, Variable>,
        covered: &mut std::collections::HashSet<String>,
        top_level: bool,
    ) {
        match &pattern.kind {
            PatternKind::Wildcard => {
                if top_level {
                    covered.insert("_".into());
                }
            }
            PatternKind::Variable(name) => {
                if top_level {
                    covered.insert("_".into());
                }
                if scope
                    .insert(
                        name.clone(),
                        Variable {
                            ty: expected.clone(),
                            mutable: false,
                        },
                    )
                    .is_some()
                {
                    self.error(
                        pattern.span,
                        format!("pattern variable `{name}` is bound twice"),
                    );
                }
            }
            PatternKind::Constructor { name, inner } => {
                if top_level {
                    covered.insert(name.clone());
                }
                match (name.as_str(), expected) {
                    ("Some", Type::Option(inner_type)) => {
                        if let Some(inner) = inner {
                            self.check_pattern(inner, inner_type, scope, covered, false);
                        } else {
                            self.error(pattern.span, "`Some` requires an inner pattern");
                        }
                    }
                    ("None", Type::Option(_)) => {
                        if inner.is_some() {
                            self.error(pattern.span, "`None` does not accept an inner pattern");
                        }
                    }
                    ("Ok", Type::Result(ok_type, _)) => {
                        if let Some(inner) = inner {
                            self.check_pattern(inner, ok_type, scope, covered, false);
                        } else {
                            self.error(pattern.span, "`Ok` requires an inner pattern");
                        }
                    }
                    ("Err", Type::Result(_, error_type)) => {
                        if let Some(inner) = inner {
                            self.check_pattern(inner, error_type, scope, covered, false);
                        } else {
                            self.error(pattern.span, "`Err` requires an inner pattern");
                        }
                    }
                    ("Some" | "None" | "Ok" | "Err", Type::Unknown) => {}
                    _ => self.error(
                        pattern.span,
                        format!("constructor `{name}` does not match `{expected}`"),
                    ),
                }
            }
            PatternKind::EnumVariant { enum_name, variant } => match expected {
                Type::Named(expected_name)
                | Type::Applied {
                    name: expected_name,
                    ..
                } if expected_name == enum_name => {
                    let type_args = match expected {
                        Type::Applied { args, .. } => args.as_slice(),
                        _ => &[],
                    };
                    let payload_types = self
                        ._program
                        .enums
                        .iter()
                        .find(|definition| definition.name == *enum_name)
                        .and_then(|definition| {
                            instantiated_enum_payload_types(definition, type_args, variant)
                        });
                    if let Some(payload_types) = payload_types {
                        if !payload_types.is_empty() {
                            self.error_with_code(
                                "E-TYPE-ENUM-001",
                                pattern.span,
                                format!(
                                    "enum variant `{enum_name}::{variant}` requires {} payload pattern(s)",
                                    payload_types.len()
                                ),
                            );
                        }
                        if top_level {
                            covered.insert(format!("{enum_name}::{variant}"));
                        }
                    } else {
                        self.error_with_code(
                            "E-TYPE-ENUM-001",
                            pattern.span,
                            format!("unknown enum variant `{enum_name}::{variant}`"),
                        );
                    }
                }
                Type::Unknown => {}
                other => self.error(
                    pattern.span,
                    format!("enum variant `{enum_name}::{variant}` does not match `{other}`"),
                ),
            },
            PatternKind::EnumVariantPayload {
                enum_name,
                variant,
                payloads,
            } => match expected {
                Type::Named(expected_name)
                | Type::Applied {
                    name: expected_name,
                    ..
                } if expected_name == enum_name => {
                    let type_args = match expected {
                        Type::Applied { args, .. } => args.as_slice(),
                        _ => &[],
                    };
                    let payload_types = self
                        ._program
                        .enums
                        .iter()
                        .find(|definition| definition.name == *enum_name)
                        .and_then(|definition| {
                            instantiated_enum_payload_types(definition, type_args, variant)
                        });
                    let Some(payload_types) = payload_types else {
                        self.error_with_code(
                            "E-TYPE-ENUM-001",
                            pattern.span,
                            format!("unknown enum variant `{enum_name}::{variant}`"),
                        );
                        return;
                    };
                    if payload_types.is_empty() {
                        self.error_with_code(
                            "E-TYPE-ENUM-001",
                            pattern.span,
                            format!(
                                "unit enum variant `{enum_name}::{variant}` is written without parentheses"
                            ),
                        );
                    }
                    if payloads.len() != payload_types.len() {
                        self.error_with_code(
                            "E-TYPE-ENUM-001",
                            pattern.span,
                            format!(
                                "enum variant pattern `{enum_name}::{variant}` expects {} payload pattern(s), found {}",
                                payload_types.len(),
                                payloads.len()
                            ),
                        );
                    }
                    for (payload, payload_type) in payloads.iter().zip(&payload_types) {
                        self.check_pattern(payload, payload_type, scope, covered, false);
                    }
                    if top_level
                        && payloads.iter().all(|payload| {
                            matches!(
                                payload.kind,
                                PatternKind::Wildcard | PatternKind::Variable(_)
                            )
                        })
                    {
                        covered.insert(format!("{enum_name}::{variant}"));
                    }
                }
                Type::Unknown => {}
                other => self.error(
                    pattern.span,
                    format!("enum variant `{enum_name}::{variant}` does not match `{other}`"),
                ),
            },
            PatternKind::Record { type_name, fields } => {
                let (expected_name, type_args) = match expected {
                    Type::Named(name) => (name.as_str(), &[][..]),
                    Type::Applied { name, args } => (name.as_str(), args.as_slice()),
                    Type::Unknown => (type_name.as_str(), &[][..]),
                    other => {
                        self.error(
                            pattern.span,
                            format!("record pattern {type_name} does not match {other}"),
                        );
                        return;
                    }
                };
                if expected_name != type_name {
                    self.error(
                        pattern.span,
                        format!("record pattern {type_name} does not match {expected_name}"),
                    );
                    return;
                }
                let Some(record) = self
                    ._program
                    .records
                    .iter()
                    .find(|record| record.name == *type_name)
                else {
                    self.error(pattern.span, format!("unknown record type {type_name}"));
                    return;
                };
                let substitutions = record
                    .type_params
                    .iter()
                    .zip(type_args)
                    .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                    .collect::<HashMap<_, _>>();
                let mut seen = HashSet::new();
                for (field_name, field_pattern) in fields {
                    if !seen.insert(field_name) {
                        self.error(
                            field_pattern.span,
                            format!("duplicate field {field_name} in record pattern"),
                        );
                        continue;
                    }
                    let Some(field) = record.fields.iter().find(|field| field.name == *field_name)
                    else {
                        self.error(
                            field_pattern.span,
                            format!("record {type_name} has no field {field_name}"),
                        );
                        continue;
                    };
                    let field_type = substitute_generic_type(&field.ty, &substitutions);
                    self.check_pattern(field_pattern, &field_type, scope, covered, false);
                }
            }
            PatternKind::Int(_) => self.expect_type(&Type::Int, expected, pattern.span),
            PatternKind::Bool(value) => {
                if top_level {
                    covered.insert(value.to_string());
                }
                self.expect_type(&Type::Bool, expected, pattern.span);
            }
            PatternKind::String(_) => self.expect_type(&Type::String, expected, pattern.span),
            PatternKind::Char(_) => self.expect_type(&Type::Char, expected, pattern.span),
        }
    }
    fn check_exhaustiveness(
        &mut self,
        ty: &Type,
        covered: &std::collections::HashSet<String>,
        span: Span,
    ) {
        if covered.contains("_") {
            return;
        }
        let missing = match ty {
            Type::Option(_) => {
                let mut missing = Vec::new();
                if !covered.contains("Some") {
                    missing.push("Some(value)");
                }
                if !covered.contains("None") {
                    missing.push("None");
                }
                missing.join(", ")
            }
            Type::Result(_, _) => {
                let mut missing = Vec::new();
                if !covered.contains("Ok") {
                    missing.push("Ok(value)");
                }
                if !covered.contains("Err") {
                    missing.push("Err(error)");
                }
                missing.join(", ")
            }
            Type::Bool => {
                let mut missing = Vec::new();
                if !covered.contains("true") {
                    missing.push("true");
                }
                if !covered.contains("false") {
                    missing.push("false");
                }
                missing.join(", ")
            }
            Type::Named(name) | Type::Applied { name, .. } => self
                ._program
                .enums
                .iter()
                .find(|definition| definition.name == *name)
                .map(|definition| {
                    definition
                        .variants
                        .iter()
                        .filter(|variant| !covered.contains(&format!("{name}::{variant}")))
                        .map(|variant| format!("{name}::{variant}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default(),
            _ => String::new(),
        };
        if !missing.is_empty() {
            self.error_with_code(
                "E-TYPE-MATCH-001",
                span,
                format!("non-exhaustive match; missing: {missing}"),
            );
        }
    }
    fn normalize_alias(&mut self, ty: &Type, span: Span) -> Type {
        match expand_type_aliases(ty, &self.type_aliases) {
            Ok(ty) => ty,
            Err(message) => {
                self.error_with_code("E-TYPE-ALIAS-001", span, message);
                Type::Unknown
            }
        }
    }

    fn expect_type(&mut self, expected: &Type, actual: &Type, span: Span) {
        let resolved_expected = self.normalize_alias(expected, span);
        let resolved_actual = self.normalize_alias(actual, span);
        if !compatible(&resolved_expected, &resolved_actual) {
            self.error(
                span,
                format!("type mismatch: expected `{expected}`, found `{actual}`"),
            );
        }
    }
    fn check_expr(&mut self, expr: &Expr, scopes: &[HashMap<String, Variable>]) -> Type {
        let ty = self.check_expr_inner(expr, scopes);
        self.normalize_alias(&ty, expr.span)
    }
    fn check_callable(
        &mut self,
        expression: &Expr,
        expected_params: &[Type],
        scopes: &[HashMap<String, Variable>],
    ) -> Type {
        if let ExprKind::Lambda { params, body } = &expression.kind {
            if params.len() != expected_params.len() {
                self.error(
                    expression.span,
                    format!(
                        "lambda expects {} parameter(s) in this operation, found {}",
                        expected_params.len(),
                        params.len()
                    ),
                );
            }
            let mut parameter_scope = HashMap::new();
            let mut parameter_types = Vec::new();
            for (index, parameter) in params.iter().enumerate() {
                let expected = expected_params.get(index).cloned().unwrap_or(Type::Unknown);
                let ty = if parameter.ty == Type::Unknown {
                    expected
                } else {
                    self.expect_type(&expected, &parameter.ty, parameter.span);
                    parameter.ty.clone()
                };
                parameter_types.push(ty.clone());
                parameter_scope.insert(parameter.name.clone(), Variable { ty, mutable: false });
            }
            let mut lambda_scopes = scopes.to_vec();
            lambda_scopes.push(parameter_scope);
            let return_type = self.check_expr(body, &lambda_scopes);
            return Type::Function {
                params: parameter_types,
                return_type: Box::new(return_type),
                capabilities: Vec::new(),
            };
        }
        let callable = self.check_expr(expression, scopes);
        let Type::Function {
            params,
            return_type,
            ..
        } = callable
        else {
            self.error(
                expression.span,
                "higher-order operation expects a function or lambda",
            );
            return Type::Function {
                params: expected_params.to_vec(),
                return_type: Box::new(Type::Unknown),
                capabilities: Vec::new(),
            };
        };
        if params.len() != expected_params.len() {
            self.error(
                expression.span,
                format!(
                    "callback expects {} parameter(s), expected {}",
                    params.len(),
                    expected_params.len()
                ),
            );
        }
        for (expected, actual) in expected_params.iter().zip(&params) {
            self.expect_type(expected, actual, expression.span);
        }
        Type::Function {
            params,
            return_type,
            capabilities: Vec::new(),
        }
    }

    fn check_higher_order_call(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[Expr],
        span: Span,
        scopes: &[HashMap<String, Variable>],
    ) -> Type {
        if !type_args.is_empty() {
            self.error(
                span,
                format!("`{name}` does not accept explicit type arguments"),
            );
        }
        let (expected_count, callback_index) = match name {
            "map" | "filter" | "find" => (2, 1),
            "reduce" => (3, 2),
            _ => unreachable!(),
        };
        if args.len() != expected_count {
            self.error(
                span,
                format!("`{name}` expects {expected_count} argument(s)"),
            );
            return Type::Unknown;
        }
        let (items, item_type) = match self.check_expr(&args[0], scopes) {
            Type::Array(item) => (true, *item),
            Type::Unknown => (true, Type::Unknown),
            other => {
                self.error(
                    args[0].span,
                    format!("`{name}` expects an array, found `{other}`"),
                );
                (false, Type::Unknown)
            }
        };
        if !items {
            return Type::Unknown;
        }
        let callback_params = if name == "reduce" {
            let accumulator_type = self.check_expr(&args[1], scopes);
            vec![accumulator_type.clone(), item_type.clone()]
        } else {
            vec![item_type.clone()]
        };
        let callback = self.check_callable(&args[callback_index], &callback_params, scopes);
        let Type::Function { return_type, .. } = callback else {
            unreachable!()
        };
        match name {
            "map" => Type::Array(return_type),
            "filter" => {
                self.expect_type(&Type::Bool, &return_type, args[callback_index].span);
                Type::Array(Box::new(item_type))
            }
            "find" => {
                self.expect_type(&Type::Bool, &return_type, args[callback_index].span);
                Type::Option(Box::new(item_type))
            }
            "reduce" => {
                let accumulator_type = self.check_expr(&args[1], scopes);
                self.expect_type(&accumulator_type, &return_type, args[callback_index].span);
                accumulator_type
            }
            _ => Type::Unknown,
        }
    }
    fn check_expr_inner(&mut self, expr: &Expr, scopes: &[HashMap<String, Variable>]) -> Type {
        match &expr.kind {
            ExprKind::Int(_) => Type::Int,
            ExprKind::UInt(_) => Type::UInt,
            ExprKind::Float(_) => Type::Float,
            ExprKind::Bool(_) => Type::Bool,
            ExprKind::String(_) => Type::String,
            ExprKind::Char(_) => Type::Char,
            ExprKind::FunctionValue(name) => {
                let Some(signature) = self.functions.get(name).cloned() else {
                    self.error_with_code(
                        "E-TYPE-FUNCTION-001",
                        expr.span,
                        format!("unknown function value `{name}`"),
                    );
                    return Type::Unknown;
                };
                if !signature.type_params.is_empty() {
                    self.error_with_code(
                        "E-TYPE-FUNCTION-001",
                        expr.span,
                        format!(
                            "generic function `{name}` cannot be stored as a function value yet"
                        ),
                    );
                    return Type::Unknown;
                }
                Type::Function {
                    params: signature.params,
                    return_type: Box::new(signature.return_type),
                    capabilities: signature.capabilities,
                }
            }
            ExprKind::Lambda { params, body } => {
                let mut lambda_scopes = scopes.to_vec();
                let mut parameter_types = Vec::with_capacity(params.len());
                let mut parameter_scope = HashMap::new();
                for parameter in params {
                    parameter_types.push(parameter.ty.clone());
                    parameter_scope.insert(
                        parameter.name.clone(),
                        Variable {
                            ty: parameter.ty.clone(),
                            mutable: false,
                        },
                    );
                }
                lambda_scopes.push(parameter_scope);
                let return_type = self.check_expr(body, &lambda_scopes);
                Type::Function {
                    params: parameter_types,
                    return_type: Box::new(return_type),
                    capabilities: Vec::new(),
                }
            }
            ExprKind::Array(values) => {
                let mut element = Type::Unknown;
                for value in values {
                    let value_type = self.check_expr(value, scopes);
                    if element == Type::Unknown {
                        element = value_type;
                    } else {
                        self.expect_type(&element, &value_type, value.span);
                    }
                }
                Type::Array(Box::new(element))
            }
            ExprKind::Map(entries) => {
                let mut key_type = Type::Unknown;
                let mut value_type = Type::Unknown;
                for (key, value) in entries {
                    let actual_key = self.check_expr(key, scopes);
                    let actual_value = self.check_expr(value, scopes);
                    if key_type == Type::Unknown {
                        key_type = actual_key;
                    } else {
                        self.expect_type(&key_type, &actual_key, key.span);
                    }
                    if value_type == Type::Unknown {
                        value_type = actual_value;
                    } else {
                        self.expect_type(&value_type, &actual_value, value.span);
                    }
                }
                if key_type != Type::Unknown && !is_map_key_type(&key_type) {
                    self.error(
                        expr.span,
                        format!(
                            "Map keys must be Int, UInt, String, Bool, Char, or Timestamp; found `{key_type}`"
                        ),
                    );
                }
                Type::Map(Box::new(key_type), Box::new(value_type))
            }
            ExprKind::Record {
                type_name,
                type_args,
                fields,
            } => {
                let Some(record) = self
                    ._program
                    .records
                    .iter()
                    .find(|record| record.name == *type_name)
                    .cloned()
                else {
                    for (_, value) in fields {
                        self.check_expr(value, scopes);
                    }
                    self.error(expr.span, format!("unknown record type `{type_name}`"));
                    return Type::Unknown;
                };
                let generic_parameters = record.type_params.iter().cloned().collect::<HashSet<_>>();
                if type_args.len() != record.type_params.len() {
                    if record.type_params.is_empty() && !type_args.is_empty() {
                        self.error_with_code(
                            "E-TYPE-GENERIC-001",
                            expr.span,
                            format!("record `{type_name}` does not accept type arguments"),
                        );
                    } else if !type_args.is_empty() {
                        self.error_with_code(
                            "E-TYPE-GENERIC-001",
                            expr.span,
                            format!(
                                "record `{type_name}` expects {} type argument(s), found {}",
                                record.type_params.len(),
                                type_args.len()
                            ),
                        );
                    }
                }
                for argument in type_args {
                    self.check_type(argument, expr.span);
                }
                let mut substitutions = record
                    .type_params
                    .iter()
                    .zip(type_args)
                    .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                    .collect::<HashMap<_, _>>();
                let mut provided = HashSet::new();
                let mut checked_fields = Vec::new();
                for (field_name, value) in fields {
                    let actual = self.check_expr(value, scopes);
                    if !provided.insert(field_name) {
                        self.error(
                            value.span,
                            format!("record field `{field_name}` is specified more than once"),
                        );
                        continue;
                    }
                    let Some(field) = record.fields.iter().find(|field| field.name == *field_name)
                    else {
                        self.error(
                            value.span,
                            format!("unknown field `{field_name}` in record `{type_name}`"),
                        );
                        continue;
                    };
                    infer_generic_types(
                        &field.ty,
                        &actual,
                        &generic_parameters,
                        &mut substitutions,
                    );
                    checked_fields.push((field, value, actual));
                }
                for (field, value, actual) in checked_fields {
                    let expected = substitute_generic_type(&field.ty, &substitutions);
                    self.expect_type(&expected, &actual, value.span);
                }
                for field in &record.fields {
                    if !provided.contains(&field.name) && !matches!(field.ty, Type::Option(_)) {
                        self.error(
                            expr.span,
                            format!("missing field `{}` in record `{type_name}`", field.name),
                        );
                    }
                }
                let applied_args = record
                    .type_params
                    .iter()
                    .map(|parameter| {
                        substitutions
                            .get(parameter)
                            .cloned()
                            .unwrap_or(Type::Unknown)
                    })
                    .collect::<Vec<_>>();
                if let Some(missing) = record
                    .type_params
                    .iter()
                    .find(|parameter| !substitutions.contains_key(*parameter))
                {
                    self.error_with_code(
                        "E-TYPE-GENERIC-001",
                        expr.span,
                        format!(
                            "cannot infer type parameter `{missing}` for record `{type_name}`; use `{type_name}<...>`"
                        ),
                    );
                }
                if record.type_params.is_empty() {
                    Type::Named(type_name.clone())
                } else {
                    Type::Applied {
                        name: type_name.clone(),
                        args: applied_args,
                    }
                }
            }
            ExprKind::Variable(name) => {
                if let Some((enum_name, variant)) = name.rsplit_once("::") {
                    if let Some(definition) = self
                        ._program
                        .enums
                        .iter()
                        .find(|definition| definition.name == enum_name)
                        .filter(|definition| definition.variants.iter().any(|item| item == variant))
                    {
                        if definition
                            .variant_payloads
                            .iter()
                            .find(|(name, _)| name == variant)
                            .is_some_and(|(_, payloads)| !payloads.is_empty())
                        {
                            self.error_with_code(
                                "E-TYPE-ENUM-001",
                                expr.span,
                                format!(
                                    "enum variant `{enum_name}::{variant}` requires payload values"
                                ),
                            );
                            return Type::Unknown;
                        }
                        if !definition.type_params.is_empty() {
                            self.error_with_code(
                                "E-TYPE-GENERIC-001",
                                expr.span,
                                format!(
                                    "generic enum variant `{name}` requires explicit type arguments; use `{enum_name}::{variant}<...>()`"
                                ),
                            );
                            Type::Unknown
                        } else {
                            Type::Named(enum_name.to_owned())
                        }
                    } else {
                        self.error_with_code(
                            "E-TYPE-ENUM-001",
                            expr.span,
                            format!("unknown enum variant `{name}`"),
                        );
                        Type::Unknown
                    }
                } else {
                    self.lookup(scopes, name)
                        .map(|v| v.ty.clone())
                        .or_else(|| {
                            if name == "None" {
                                Some(Type::Option(Box::new(Type::Unknown)))
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| {
                            self.error(expr.span, format!("unknown variable `{name}`"));
                            Type::Unknown
                        })
                }
            }
            ExprKind::Call {
                name,
                type_args,
                args,
            } => {
                if matches!(name.as_str(), "map" | "filter" | "find" | "reduce") {
                    return self.check_higher_order_call(name, type_args, args, expr.span, scopes);
                }
                if name == "render_pdf" {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            "render_pdf expects a named View and one record value",
                        );
                        return Type::Unknown;
                    }
                    let view_type = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &view_type, args[0].span);
                    if let ExprKind::String(view_name) = &args[0].kind {
                        if !self
                            ._program
                            .views
                            .iter()
                            .any(|view| view.name == *view_name)
                        {
                            self.error(args[0].span, format!("unknown PDF view `{view_name}`"));
                        }
                    }
                    match self.check_expr(&args[1], scopes) {
                        Type::Named(name)
                            if self
                                ._program
                                .records
                                .iter()
                                .any(|record| record.name == name) => {}
                        Type::Map(key, _) if *key == Type::String => {}
                        Type::Unknown => {}
                        found => self.error(
                            args[1].span,
                            format!(
                                "render_pdf expects a record or Map<String, T>, found `{found}`"
                            ),
                        ),
                    }
                    return Type::Bytes;
                } else if let Some(ty) =
                    self.check_enum_constructor(name, type_args, args, expr.span, scopes)
                {
                    return ty;
                }
                if name == "read_console" {
                    if args.len() != 1 {
                        self.error(expr.span, "read_console expects exactly one String prompt");
                        return Type::Unknown;
                    }
                    let prompt = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &prompt, args[0].span);
                    Type::Option(Box::new(Type::String))
                } else if name == "print" {
                    if args.len() != 1 {
                        self.error(expr.span, "`print` expects exactly one argument");
                    }
                    for arg in args {
                        self.check_expr(arg, scopes);
                    }
                    Type::Unit
                } else if name == "assert" {
                    if !type_args.is_empty() {
                        self.error(expr.span, "`assert` does not accept type arguments");
                    }
                    if !(1..=2).contains(&args.len()) {
                        self.error(
                            expr.span,
                            "`assert` expects a Bool condition and an optional String message",
                        );
                        return Type::Unknown;
                    }
                    let condition = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::Bool, &condition, args[0].span);
                    if let Some(message) = args.get(1) {
                        let message_type = self.check_expr(message, scopes);
                        self.expect_type(&Type::String, &message_type, message.span);
                    }
                    Type::Unit
                } else if name == "range" {
                    if !type_args.is_empty() || !(2..=3).contains(&args.len()) {
                        self.error(
                            expr.span,
                            "range expects two Int bounds, an optional Int step, and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let start = self.check_expr(&args[0], scopes);
                    let end = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Int, &start, args[0].span);
                    self.expect_type(&Type::Int, &end, args[1].span);
                    if let Some(step) = args.get(2) {
                        let step_type = self.check_expr(step, scopes);
                        self.expect_type(&Type::Int, &step_type, step.span);
                    }
                    Type::Array(Box::new(Type::Int))
                } else if name == "array_replace_result" {
                    if !type_args.is_empty() || args.len() != 3 {
                        self.error(
                            expr.span,
                            "array_replace_result expects an array, Int index, and value",
                        );
                        return Type::Unknown;
                    }
                    let array = self.check_expr(&args[0], scopes);
                    let index = self.check_expr(&args[1], scopes);
                    let value = self.check_expr(&args[2], scopes);
                    self.expect_type(&Type::Int, &index, args[1].span);
                    match array {
                        Type::Array(element) => {
                            self.expect_type(&element, &value, args[2].span);
                            Type::Result(Box::new(Type::Array(element)), Box::new(Type::String))
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("array_replace_result expects an array, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "object_set_field" {
                    if !type_args.is_empty() || args.len() != 3 {
                        self.error_with_code(
                            "E-TYPE-CLASS-001",
                            expr.span,
                            "class field update expects an object, literal field name, and value",
                        );
                        return Type::Unknown;
                    }
                    let object_type = self.check_expr(&args[0], scopes);
                    let field_name = match &args[1].kind {
                        ExprKind::String(name) => Some(name.as_str()),
                        _ => {
                            self.error_with_code(
                                "E-TYPE-CLASS-001",
                                args[1].span,
                                "class field name must be a String literal",
                            );
                            None
                        }
                    };
                    let value_type = self.check_expr(&args[2], scopes);
                    let (type_name, type_arguments) = match &object_type {
                        Type::Named(type_name) => (Some(type_name.as_str()), &[][..]),
                        Type::Applied { name, args } => (Some(name.as_str()), args.as_slice()),
                        _ => (None, &[][..]),
                    };
                    if let (Some(type_name), Some(field_name)) = (type_name, field_name) {
                        match self
                            ._program
                            .records
                            .iter()
                            .find(|record| record.name == type_name)
                        {
                            Some(class) if class.is_class => {
                                if let Some(field) =
                                    class.fields.iter().find(|field| field.name == field_name)
                                {
                                    let substitutions = class
                                        .type_params
                                        .iter()
                                        .zip(type_arguments)
                                        .map(|(parameter, argument)| {
                                            (parameter.clone(), argument.clone())
                                        })
                                        .collect::<HashMap<_, _>>();
                                    let field_type =
                                        substitute_generic_type(&field.ty, &substitutions);
                                    let field_type =
                                        self.normalize_alias(&field_type, args[2].span);
                                    if !compatible(&field_type, &value_type) {
                                        self.error_with_code(
                                            "E-TYPE-CLASS-001",
                                            args[2].span,
                                            format!(
                                                "type mismatch: expected `{field_type}`, found `{value_type}`"
                                            ),
                                        );
                                    }
                                } else {
                                    self.error_with_code(
                                        "E-TYPE-CLASS-001",
                                        args[1].span,
                                        format!(
                                            "unknown field `{field_name}` in class `{type_name}`"
                                        ),
                                    );
                                }
                            }
                            _ => self.error_with_code(
                                "E-TYPE-CLASS-001",
                                args[0].span,
                                "field assignment requires a class instance",
                            ),
                        }
                    }
                    Type::Unit
                } else if name == "record_update" {
                    if type_args.len() != 1 || args.is_empty() || args.len() % 2 == 0 {
                        self.error(
                            expr.span,
                            "record update expects a base record followed by field/value pairs",
                        );
                        return Type::Unknown;
                    }
                    self.check_type(&type_args[0], expr.span);
                    let expected_record_type = self.normalize_alias(&type_args[0], expr.span);
                    let actual_record_type = self.check_expr(&args[0], scopes);
                    self.expect_type(&expected_record_type, &actual_record_type, args[0].span);
                    let (record_name, type_arguments) = match &expected_record_type {
                        Type::Named(name) => (Some(name.as_str()), &[][..]),
                        Type::Applied { name, args } => (Some(name.as_str()), args.as_slice()),
                        _ => (None, &[][..]),
                    };
                    let Some(record_name) = record_name else {
                        self.error(
                            args[0].span,
                            "record update base must have a declared record type",
                        );
                        return Type::Unknown;
                    };
                    let Some(record) = self
                        ._program
                        .records
                        .iter()
                        .find(|record| record.name == record_name)
                        .cloned()
                    else {
                        self.error(
                            args[0].span,
                            format!("unknown record type `{record_name}` in record update"),
                        );
                        return Type::Unknown;
                    };
                    if record.is_class {
                        self.error_with_code(
                            "E-TYPE-CLASS-001",
                            args[0].span,
                            "record update cannot copy or update a class reference",
                        );
                        return Type::Unknown;
                    }
                    let substitutions = record
                        .type_params
                        .iter()
                        .zip(type_arguments)
                        .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                        .collect::<HashMap<_, _>>();
                    for pair in args[1..].chunks(2) {
                        let field_name = match &pair[0].kind {
                            ExprKind::String(name) => Some(name.as_str()),
                            _ => {
                                self.error(
                                    pair[0].span,
                                    "record update field name must be a literal field name",
                                );
                                None
                            }
                        };
                        let value_type = self.check_expr(&pair[1], scopes);
                        if let Some(field_name) = field_name {
                            if let Some(field) =
                                record.fields.iter().find(|field| field.name == field_name)
                            {
                                let field_type = substitute_generic_type(&field.ty, &substitutions);
                                self.expect_type(&field_type, &value_type, pair[1].span);
                            } else {
                                self.error(
                                    pair[0].span,
                                    format!(
                                        "unknown field `{field_name}` in record `{record_name}`"
                                    ),
                                );
                            }
                        }
                    }
                    expected_record_type
                } else if name == "record_with" {
                    if !type_args.is_empty() || args.len() != 3 {
                        self.error(
                            expr.span,
                            "record_with expects a record, literal field name, and value",
                        );
                        return Type::Unknown;
                    }
                    let record_type = self.check_expr(&args[0], scopes);
                    let field_name = match &args[1].kind {
                        ExprKind::String(name) => Some(name.as_str()),
                        _ => {
                            self.error(
                                args[1].span,
                                "record_with field name must be a String literal",
                            );
                            None
                        }
                    };
                    let value = self.check_expr(&args[2], scopes);
                    let (type_name, type_arguments) = match &record_type {
                        Type::Named(type_name) => (Some(type_name.as_str()), &[][..]),
                        Type::Applied { name, args } => (Some(name.as_str()), args.as_slice()),
                        _ => (None, &[][..]),
                    };
                    if let (Some(type_name), Some(field_name)) = (type_name, field_name) {
                        if let Some(record) = self
                            ._program
                            .records
                            .iter()
                            .find(|record| record.name == type_name)
                        {
                            if record.is_class {
                                self.error_with_code(
                                    "E-TYPE-CLASS-001",
                                    args[0].span,
                                    "record_with cannot copy or update a class reference",
                                );
                                return Type::Unknown;
                            }
                            if let Some(field) =
                                record.fields.iter().find(|field| field.name == field_name)
                            {
                                let substitutions = record
                                    .type_params
                                    .iter()
                                    .zip(type_arguments)
                                    .map(|(parameter, argument)| {
                                        (parameter.clone(), argument.clone())
                                    })
                                    .collect::<HashMap<_, _>>();
                                let field_type = substitute_generic_type(&field.ty, &substitutions);
                                self.expect_type(&field_type, &value, args[2].span);
                            } else {
                                self.error(
                                    args[1].span,
                                    format!("unknown field `{field_name}` in record `{type_name}`"),
                                );
                            }
                        } else {
                            self.error(
                                args[0].span,
                                format!("record_with expects a record, found `{record_type}`"),
                            );
                        }
                    }
                    record_type
                } else if name == "len" {
                    if args.len() != 1 {
                        self.error(expr.span, "`len` expects exactly one argument");
                        return Type::Unknown;
                    }
                    let argument = self.check_expr(&args[0], scopes);
                    if !matches!(
                        argument,
                        Type::Array(_)
                            | Type::Tuple(_)
                            | Type::Set(_)
                            | Type::Map(_, _)
                            | Type::String
                            | Type::Bytes
                    ) && argument != Type::Unknown
                    {
                        self.error(
                            args[0].span,
                            "`len` expects an array, Tuple, Set, Map, String or Bytes",
                        );
                    }
                    Type::Int
                } else if name == "append" {
                    if args.len() != 2 {
                        self.error(expr.span, "`append` expects an array and one value");
                        return Type::Unknown;
                    }
                    let array_type = self.check_expr(&args[0], scopes);
                    let value_type = self.check_expr(&args[1], scopes);
                    match array_type {
                        Type::Array(inner) => {
                            self.expect_type(&inner, &value_type, args[1].span);
                            Type::Array(inner)
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("`append` expects an array, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "contains" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "`contains` expects an array, Set or Map and one value",
                        );
                        return Type::Unknown;
                    }
                    let array_type = self.check_expr(&args[0], scopes);
                    let value_type = self.check_expr(&args[1], scopes);
                    match array_type {
                        Type::Array(inner) => {
                            self.expect_type(&inner, &value_type, args[1].span);
                            Type::Bool
                        }
                        Type::Map(key, _) => {
                            self.expect_type(&key, &value_type, args[1].span);
                            Type::Bool
                        }
                        Type::Set(inner) => {
                            self.expect_type(&inner, &value_type, args[1].span);
                            Type::Bool
                        }
                        Type::Unknown => Type::Bool,
                        other => {
                            self.error(
                                args[0].span,
                                format!(
                                    "`contains` expects an array, Set, or Map, found `{other}`"
                                ),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "tuple" {
                    if args.len() < 2 {
                        self.error(expr.span, "tuple expects at least two values");
                        return Type::Unknown;
                    }
                    Type::Tuple(
                        args.iter()
                            .map(|arg| self.check_expr(arg, scopes))
                            .collect(),
                    )
                } else if name == "set" {
                    if args.len() != 1 {
                        self.error(expr.span, "set expects one array");
                        return Type::Unknown;
                    }
                    match self.check_expr(&args[0], scopes) {
                        Type::Array(inner) => {
                            if !matches!(inner.as_ref(), Type::Unknown) && !is_map_key_type(&inner)
                            {
                                self.error(args[0].span, format!("Set elements must be scalar comparable types; found `{inner}`"));
                            }
                            Type::Set(inner)
                        }
                        Type::Unknown => Type::Set(Box::new(Type::Unknown)),
                        other => {
                            self.error(
                                args[0].span,
                                format!("set expects an array, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "set_add" || name == "set_remove" {
                    if args.len() != 2 {
                        self.error(expr.span, format!("{name} expects a Set and one element"));
                        return Type::Unknown;
                    }
                    let collection = self.check_expr(&args[0], scopes);
                    let element = self.check_expr(&args[1], scopes);
                    match collection {
                        Type::Set(inner) => {
                            self.expect_type(&inner, &element, args[1].span);
                            Type::Set(inner)
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("{name} expects a Set, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "text_contains" {
                    if args.len() != 2 {
                        self.error(expr.span, "text_contains expects a text and search text");
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    let search = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    self.expect_type(&Type::String, &search, args[1].span);
                    Type::Bool
                } else if name == "text_trim" {
                    if args.len() != 1 {
                        self.error(expr.span, "text_trim expects exactly one String");
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    Type::String
                } else if name == "text_chars" {
                    if args.len() != 1 {
                        self.error(expr.span, "text_chars expects exactly one String");
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    Type::Array(Box::new(Type::String))
                } else if name == "abs" {
                    if args.len() != 1 {
                        self.error(expr.span, "abs expects exactly one numeric argument");
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    if matches!(ty, Type::Int | Type::Float | Type::Decimal) {
                        ty
                    } else if ty == Type::Unknown {
                        Type::Unknown
                    } else {
                        self.error(
                            args[0].span,
                            format!("abs expects Int, Float, or Decimal, found {ty}"),
                        );
                        Type::Unknown
                    }
                } else if name == "min" || name == "max" {
                    if args.len() != 2 {
                        self.error(expr.span, format!("{name} expects exactly two arguments"));
                        return Type::Unknown;
                    }
                    let a = self.check_expr(&args[0], scopes);
                    let b = self.check_expr(&args[1], scopes);
                    self.expect_type(&a, &b, args[1].span);
                    if !matches!(
                        &a,
                        Type::Int
                            | Type::UInt
                            | Type::Float
                            | Type::Decimal
                            | Type::String
                            | Type::Unknown
                    ) {
                        self.error(
                            args[0].span,
                            format!(
                                "{name} expects matching numeric or String arguments, found {a}"
                            ),
                        );
                        return Type::Unknown;
                    }
                    a
                } else if name == "clamp" {
                    if args.len() != 3 {
                        self.error(expr.span, "clamp expects val, min, and max");
                        return Type::Unknown;
                    }
                    let val = self.check_expr(&args[0], scopes);
                    let min_val = self.check_expr(&args[1], scopes);
                    let max_val = self.check_expr(&args[2], scopes);
                    self.expect_type(&val, &min_val, args[1].span);
                    self.expect_type(&val, &max_val, args[2].span);
                    if !matches!(
                        &val,
                        Type::Int | Type::UInt | Type::Float | Type::Decimal | Type::Unknown
                    ) {
                        self.error(
                            args[0].span,
                            format!("clamp expects matching numeric arguments, found {val}"),
                        );
                        return Type::Unknown;
                    }
                    val
                } else if name == "to_string" {
                    if args.len() != 1 {
                        self.error(expr.span, "to_string expects exactly one argument");
                        return Type::Unknown;
                    }
                    let _ = self.check_expr(&args[0], scopes);
                    Type::String
                } else if name == "parse_int" {
                    if args.len() != 1 {
                        self.error(expr.span, "parse_int expects exactly one String");
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &ty, args[0].span);
                    Type::Result(Box::new(Type::Int), Box::new(Type::String))
                } else if name == "parse_float" {
                    if args.len() != 1 {
                        self.error(expr.span, "parse_float expects exactly one String");
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &ty, args[0].span);
                    Type::Result(Box::new(Type::Float), Box::new(Type::String))
                } else if name == "sort" || name == "sort_desc" || name == "reverse" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects exactly one array"));
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    match ty {
                        Type::Array(inner) => Type::Array(inner),
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("{name} expects an array, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "slice" {
                    if args.len() != 3 {
                        self.error(expr.span, "slice expects array, start index, and length");
                        return Type::Unknown;
                    }
                    let arr_ty = self.check_expr(&args[0], scopes);
                    let start_ty = self.check_expr(&args[1], scopes);
                    let len_ty = self.check_expr(&args[2], scopes);
                    self.expect_type(&Type::Int, &start_ty, args[1].span);
                    self.expect_type(&Type::Int, &len_ty, args[2].span);
                    match arr_ty {
                        Type::Array(inner) => Type::Array(inner),
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("slice expects an array, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "concat" {
                    if args.len() != 2 {
                        self.error(expr.span, "concat expects two arrays of the same type");
                        return Type::Unknown;
                    }
                    let a_ty = self.check_expr(&args[0], scopes);
                    let b_ty = self.check_expr(&args[1], scopes);
                    self.expect_type(&a_ty, &b_ty, args[1].span);
                    a_ty
                } else if name == "join" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "join expects an array of Strings and a delimiter",
                        );
                        return Type::Unknown;
                    }
                    let arr_ty = self.check_expr(&args[0], scopes);
                    let sep_ty = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Array(Box::new(Type::String)), &arr_ty, args[0].span);
                    self.expect_type(&Type::String, &sep_ty, args[1].span);
                    Type::String
                } else if name == "text_to_lower" || name == "text_to_upper" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects exactly one String"));
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &ty, args[0].span);
                    Type::String
                } else if name == "text_starts_with" || name == "text_ends_with" {
                    if args.len() != 2 {
                        self.error(expr.span, format!("{name} expects text and pattern"));
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    let pat = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    self.expect_type(&Type::String, &pat, args[1].span);
                    Type::Bool
                } else if name == "text_split" || name == "split" {
                    if args.len() != 2 {
                        self.error(expr.span, format!("{name} expects text and delimiter"));
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    let sep = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    self.expect_type(&Type::String, &sep, args[1].span);
                    Type::Array(Box::new(Type::String))
                } else if name == "text_replace" {
                    if args.len() != 3 {
                        self.error(
                            expr.span,
                            "text_replace expects text, search, and replacement",
                        );
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    let old = self.check_expr(&args[1], scopes);
                    let new = self.check_expr(&args[2], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    self.expect_type(&Type::String, &old, args[1].span);
                    self.expect_type(&Type::String, &new, args[2].span);
                    Type::String
                } else if name == "text_pad_left" || name == "text_pad_right" {
                    if args.len() != 3 {
                        self.error(
                            expr.span,
                            format!("{name} expects text, target length, and pad char"),
                        );
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    let target_len = self.check_expr(&args[1], scopes);
                    let pad_char = self.check_expr(&args[2], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    self.expect_type(&Type::Int, &target_len, args[1].span);
                    self.expect_type(&Type::String, &pad_char, args[2].span);
                    Type::String
                } else if name == "get_or" || name == "get_or_default" {
                    if args.len() != 3 {
                        self.error(
                            expr.span,
                            format!("{name} expects a Map, key, and default fallback value"),
                        );
                        return Type::Unknown;
                    }
                    let map_ty = self.check_expr(&args[0], scopes);
                    let key_ty = self.check_expr(&args[1], scopes);
                    let fallback_ty = self.check_expr(&args[2], scopes);
                    match map_ty {
                        Type::Map(k, v) => {
                            self.expect_type(&k, &key_ty, args[1].span);
                            self.expect_type(&v, &fallback_ty, args[2].span);
                            *v
                        }
                        Type::Unknown => fallback_ty,
                        other => {
                            self.error(
                                args[0].span,
                                format!("{name} expects a Map, found {other}"),
                            );
                            fallback_ty
                        }
                    }
                } else if name == "map_increment" {
                    if args.len() != 2 {
                        self.error(expr.span, "map_increment expects a Map<Int> and key");
                        return Type::Unknown;
                    }
                    let map_ty = self.check_expr(&args[0], scopes);
                    let key_ty = self.check_expr(&args[1], scopes);
                    match map_ty {
                        Type::Map(key, value) => {
                            self.expect_type(&key, &key_ty, args[1].span);
                            self.expect_type(&Type::Int, &value, args[0].span);
                            Type::Result(
                                Box::new(Type::Map(key, Box::new(Type::Int))),
                                Box::new(Type::String),
                            )
                        }
                        Type::Unknown => Type::Result(
                            Box::new(Type::Map(Box::new(Type::Unknown), Box::new(Type::Int))),
                            Box::new(Type::String),
                        ),
                        other => {
                            self.error(
                                args[0].span,
                                format!("map_increment expects a Map, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "has_key" {
                    if args.len() != 2 {
                        self.error(expr.span, "has_key expects a Map and key");
                        return Type::Unknown;
                    }
                    let map_ty = self.check_expr(&args[0], scopes);
                    let key_ty = self.check_expr(&args[1], scopes);
                    match map_ty {
                        Type::Map(k, _) => {
                            self.expect_type(&k, &key_ty, args[1].span);
                            Type::Bool
                        }
                        Type::Unknown => Type::Bool,
                        other => {
                            self.error(
                                args[0].span,
                                format!("has_key expects a Map, found {other}"),
                            );
                            Type::Bool
                        }
                    }
                } else if name == "sqrt" || name == "floor" || name == "ceil" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects exactly one Float"));
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::Float, &ty, args[0].span);
                    Type::Float
                } else if name == "pow" {
                    if args.len() != 2 {
                        self.error(expr.span, "pow expects base (Float) and exponent (Float)");
                        return Type::Unknown;
                    }
                    let base = self.check_expr(&args[0], scopes);
                    let exp = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Float, &base, args[0].span);
                    self.expect_type(&Type::Float, &exp, args[1].span);
                    Type::Float
                } else if name == "round" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "round expects number (Float) and decimal places (Int)",
                        );
                        return Type::Unknown;
                    }
                    let num = self.check_expr(&args[0], scopes);
                    let decimals = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Float, &num, args[0].span);
                    self.expect_type(&Type::Int, &decimals, args[1].span);
                    Type::Float
                } else if name == "text_index_of" {
                    if args.len() != 2 {
                        self.error(expr.span, "text_index_of expects text and substring");
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    let sub = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    self.expect_type(&Type::String, &sub, args[1].span);
                    Type::Option(Box::new(Type::Int))
                } else if name == "text_repeat" {
                    if args.len() != 2 {
                        self.error(expr.span, "text_repeat expects text and count (Int)");
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    let count = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    self.expect_type(&Type::Int, &count, args[1].span);
                    Type::String
                } else if name == "text_is_empty" {
                    if args.len() != 1 {
                        self.error(expr.span, "text_is_empty expects exactly one String");
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    Type::Bool
                } else if name == "text_capitalize" {
                    if args.len() != 1 {
                        self.error(expr.span, "text_capitalize expects exactly one String");
                        return Type::Unknown;
                    }
                    let text = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &text, args[0].span);
                    Type::String
                } else if name == "unique" {
                    if args.len() != 1 {
                        self.error(expr.span, "unique expects an array");
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    match ty {
                        Type::Array(inner) => Type::Array(inner),
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("unique expects an array, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "array_sum" {
                    if args.len() != 1 {
                        self.error(expr.span, "sum expects an array of numbers");
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    match ty {
                        Type::Array(inner)
                            if matches!(*inner, Type::Int | Type::Float | Type::Decimal) =>
                        {
                            *inner
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!(
                                    "sum expects array of Int, Float, or Decimal, found {other}"
                                ),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "avg" {
                    if args.len() != 1 {
                        self.error(expr.span, "avg expects an array of numbers");
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    match ty {
                        Type::Array(inner) if matches!(*inner, Type::Int | Type::Float) => {
                            Type::Float
                        }
                        Type::Array(inner) if matches!(*inner, Type::Decimal) => Type::Decimal,
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("avg expects array of numbers, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "min_of" || name == "max_of" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects an array"));
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    match ty {
                        Type::Array(inner)
                            if matches!(
                                *inner,
                                Type::Int | Type::UInt | Type::Float | Type::Decimal | Type::String
                            ) =>
                        {
                            Type::Option(inner)
                        }
                        Type::Array(inner) if inner.as_ref() == &Type::Unknown => {
                            Type::Option(inner)
                        }
                        Type::Array(inner) => {
                            self.error(
                                args[0].span,
                                format!(
                                    "{name} expects an array of numbers or String, found {}",
                                    inner
                                ),
                            );
                            Type::Option(inner)
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("{name} expects an array, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "chunk" {
                    if args.len() != 2 {
                        self.error(expr.span, "chunk expects an array and chunk size (Int)");
                        return Type::Unknown;
                    }
                    let arr_ty = self.check_expr(&args[0], scopes);
                    let size_ty = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Int, &size_ty, args[1].span);
                    match arr_ty {
                        Type::Array(inner) => Type::Array(Box::new(Type::Array(inner))),
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("chunk expects an array, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "flatten" {
                    if args.len() != 1 {
                        self.error(expr.span, "flatten expects a nested array");
                        return Type::Unknown;
                    }
                    let arr_ty = self.check_expr(&args[0], scopes);
                    match arr_ty {
                        Type::Array(inner) => {
                            match *inner {
                                Type::Array(element) => Type::Array(element),
                                other => {
                                    self.error(args[0].span, format!("flatten expects array of arrays, found array of {other}"));
                                    Type::Unknown
                                }
                            }
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("flatten expects an array, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "set_union"
                    || name == "set_intersection"
                    || name == "set_difference"
                {
                    if args.len() != 2 {
                        self.error(expr.span, format!("{name} expects two Sets"));
                        return Type::Unknown;
                    }
                    let a_ty = self.check_expr(&args[0], scopes);
                    let b_ty = self.check_expr(&args[1], scopes);
                    self.expect_type(&a_ty, &b_ty, args[1].span);
                    match a_ty {
                        Type::Set(inner) => Type::Set(inner),
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(args[0].span, format!("{name} expects Sets, found {other}"));
                            Type::Unknown
                        }
                    }
                } else if name == "map_remove" {
                    if args.len() != 2 {
                        self.error(expr.span, "map_remove expects a Map and key");
                        return Type::Unknown;
                    }
                    let map_ty = self.check_expr(&args[0], scopes);
                    let key_ty = self.check_expr(&args[1], scopes);
                    match map_ty {
                        Type::Map(k, v) => {
                            self.expect_type(&k, &key_ty, args[1].span);
                            Type::Map(k, v)
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("map_remove expects a Map, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "map_merge" {
                    if args.len() != 2 {
                        self.error(expr.span, "map_merge expects two Maps of the same type");
                        return Type::Unknown;
                    }
                    let a_ty = self.check_expr(&args[0], scopes);
                    let b_ty = self.check_expr(&args[1], scopes);
                    self.expect_type(&a_ty, &b_ty, args[1].span);
                    a_ty
                } else if name == "unwrap_or" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "unwrap_or expects an Option and fallback default",
                        );
                        return Type::Unknown;
                    }
                    let opt_ty = self.check_expr(&args[0], scopes);
                    let fallback_ty = self.check_expr(&args[1], scopes);
                    match opt_ty {
                        Type::Option(inner) => {
                            self.expect_type(&inner, &fallback_ty, args[1].span);
                            *inner
                        }
                        Type::Unknown => fallback_ty,
                        other => {
                            self.error(
                                args[0].span,
                                format!("unwrap_or expects Option, found {other}"),
                            );
                            fallback_ty
                        }
                    }
                } else if name == "text_replace_result" {
                    if args.len() != 3 {
                        self.error(
                            expr.span,
                            "text_replace_result expects text, search, and replacement",
                        );
                        return Type::Unknown;
                    }
                    for arg in args {
                        let value = self.check_expr(arg, scopes);
                        self.expect_type(&Type::String, &value, arg.span);
                    }
                    Type::Result(Box::new(Type::String), Box::new(Type::String))
                } else if name == "text_split_result" {
                    if args.len() != 2 {
                        self.error(expr.span, "text_split_result expects text and separator");
                        return Type::Unknown;
                    }
                    for arg in args {
                        let value = self.check_expr(arg, scopes);
                        self.expect_type(&Type::String, &value, arg.span);
                    }
                    Type::Result(
                        Box::new(Type::Array(Box::new(Type::String))),
                        Box::new(Type::String),
                    )
                } else if name == "get" {
                    if args.len() != 2 {
                        self.error(expr.span, "`get` expects a Map and one key");
                        return Type::Unknown;
                    }
                    let map_type = self.check_expr(&args[0], scopes);
                    let key_type = self.check_expr(&args[1], scopes);
                    match map_type {
                        Type::Map(expected_key, value) => {
                            self.expect_type(&expected_key, &key_type, args[1].span);
                            Type::Option(value)
                        }
                        Type::Unknown => Type::Option(Box::new(Type::Unknown)),
                        other => {
                            self.error(
                                args[0].span,
                                format!("`get` expects a Map, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "put" {
                    if args.len() != 3 {
                        self.error(expr.span, "`put` expects a Map, a key, and a value");
                        return Type::Unknown;
                    }
                    let map_type = self.check_expr(&args[0], scopes);
                    let key_type = self.check_expr(&args[1], scopes);
                    let value_type = self.check_expr(&args[2], scopes);
                    match map_type {
                        Type::Map(expected_key, expected_value) => {
                            self.expect_type(&expected_key, &key_type, args[1].span);
                            self.expect_type(&expected_value, &value_type, args[2].span);
                            Type::Map(expected_key, expected_value)
                        }
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("`put` expects a Map, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "keys" || name == "values" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("`{name}` expects exactly one Map"));
                        return Type::Unknown;
                    }
                    let map_type = self.check_expr(&args[0], scopes);
                    match map_type {
                        Type::Map(key, _) if name == "keys" => Type::Array(key),
                        Type::Map(_, value) => Type::Array(value),
                        Type::Unknown => Type::Array(Box::new(Type::Unknown)),
                        other => {
                            self.error(
                                args[0].span,
                                format!("`{name}` expects a Map, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "first" || name == "last" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("`{name}` expects exactly one argument"));
                        return Type::Unknown;
                    }
                    let argument = self.check_expr(&args[0], scopes);
                    match argument {
                        Type::Array(inner) => Type::Option(inner),
                        Type::Unknown => Type::Option(Box::new(Type::Unknown)),
                        other => {
                            self.error(
                                args[0].span,
                                format!("`{name}` expects an array, found `{other}`"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "Some" || name == "Ok" || name == "Err" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("`{name}` expects exactly one argument"));
                        return Type::Unknown;
                    }
                    let inner = self.check_expr(&args[0], scopes);
                    if name == "Some" {
                        Type::Option(Box::new(inner))
                    } else if name == "Ok" {
                        Type::Result(Box::new(inner), Box::new(Type::Unknown))
                    } else {
                        Type::Result(Box::new(Type::Unknown), Box::new(inner))
                    }
                } else if name == "now" {
                    if !args.is_empty() {
                        self.error(expr.span, "now expects no arguments");
                    }
                    Type::Timestamp
                } else if name == "date_add_days" {
                    if args.len() != 2 {
                        self.error(expr.span, "date_add_days expects Date and Int days");
                        return Type::Unknown;
                    }
                    let d = self.check_expr(&args[0], scopes);
                    let days = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Date, &d, args[0].span);
                    self.expect_type(&Type::Int, &days, args[1].span);
                    Type::Date
                } else if name == "date_diff_days" {
                    if args.len() != 2 {
                        self.error(expr.span, "date_diff_days expects two Dates");
                        return Type::Unknown;
                    }
                    let a = self.check_expr(&args[0], scopes);
                    let b = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Date, &a, args[0].span);
                    self.expect_type(&Type::Date, &b, args[1].span);
                    Type::Int
                } else if name == "is_ok" || name == "is_err" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects one Result"));
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    if !matches!(ty, Type::Result(_, _)) && ty != Type::Unknown {
                        self.error(args[0].span, format!("{name} expects Result, found {ty}"));
                    }
                    Type::Bool
                } else if name == "is_some" || name == "is_none" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects one Option"));
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    if !matches!(ty, Type::Option(_)) && ty != Type::Unknown {
                        self.error(args[0].span, format!("{name} expects Option, found {ty}"));
                    }
                    Type::Bool
                } else if name == "is_before" || name == "is_after" {
                    if args.len() != 2 {
                        self.error(expr.span, format!("{name} expects two Dates"));
                        return Type::Unknown;
                    }
                    let a = self.check_expr(&args[0], scopes);
                    let b = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Date, &a, args[0].span);
                    self.expect_type(&Type::Date, &b, args[1].span);
                    Type::Bool
                } else if name == "base64_encode" || name == "hash_sha256" || name == "url_encode" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects one String"));
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    Type::String
                } else if name == "base64_decode" || name == "url_decode" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects one String"));
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    Type::Result(Box::new(Type::String), Box::new(Type::String))
                } else if name == "uuid_v4" {
                    if !args.is_empty() {
                        self.error(expr.span, "uuid_v4 expects no arguments");
                        return Type::Unknown;
                    }
                    Type::String
                } else if name == "text_trim_start" || name == "text_trim_end" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects one String"));
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    Type::String
                } else if name == "text_is_blank" {
                    if args.len() != 1 {
                        self.error(expr.span, "text_is_blank expects one String");
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    Type::Bool
                } else if name == "text_substring" {
                    if args.len() != 3 {
                        self.error(
                            expr.span,
                            "text_substring expects String, start Int, and length Int",
                        );
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    let start = self.check_expr(&args[1], scopes);
                    let len = self.check_expr(&args[2], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    self.expect_type(&Type::Int, &start, args[1].span);
                    self.expect_type(&Type::Int, &len, args[2].span);
                    Type::String
                } else if name == "to_int" {
                    if args.len() != 1 {
                        self.error(expr.span, "to_int expects one argument");
                        return Type::Unknown;
                    }
                    let _ = self.check_expr(&args[0], scopes);
                    Type::Int
                } else if name == "to_float" {
                    if args.len() != 1 {
                        self.error(expr.span, "to_float expects one argument");
                        return Type::Unknown;
                    }
                    let _ = self.check_expr(&args[0], scopes);
                    Type::Float
                } else if name == "parse_bool" {
                    if args.len() != 1 {
                        self.error(expr.span, "parse_bool expects one String");
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    Type::Result(Box::new(Type::Bool), Box::new(Type::String))
                } else if name == "panic" {
                    if args.len() != 1 {
                        self.error(expr.span, "panic expects one String message");
                        return Type::Unknown;
                    }
                    let _ = self.check_expr(&args[0], scopes);
                    Type::Unknown
                } else if name == "entries" {
                    if args.len() != 1 {
                        self.error(expr.span, "entries expects a Map");
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    match ty {
                        Type::Map(k, v) => Type::Array(Box::new(Type::Tuple(vec![*k, *v]))),
                        Type::Unknown => Type::Array(Box::new(Type::Unknown)),
                        other => {
                            self.error(
                                args[0].span,
                                format!("entries expects a Map, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "zip" {
                    if args.len() != 2 {
                        self.error(expr.span, "zip expects two Arrays");
                        return Type::Unknown;
                    }
                    let a = self.check_expr(&args[0], scopes);
                    let b = self.check_expr(&args[1], scopes);
                    match (a, b) {
                        (Type::Array(ta), Type::Array(tb)) => {
                            Type::Array(Box::new(Type::Tuple(vec![*ta, *tb])))
                        }
                        _ => Type::Array(Box::new(Type::Unknown)),
                    }
                } else if name == "dedup" {
                    if args.len() != 1 {
                        self.error(expr.span, "dedup expects an Array");
                        return Type::Unknown;
                    }
                    let ty = self.check_expr(&args[0], scopes);
                    match ty {
                        Type::Array(inner) => Type::Array(inner),
                        Type::Unknown => Type::Unknown,
                        other => {
                            self.error(
                                args[0].span,
                                format!("dedup expects an Array, found {other}"),
                            );
                            Type::Unknown
                        }
                    }
                } else if name == "path_join" {
                    if args.len() != 2 {
                        self.error(expr.span, "path_join expects two String path segments");
                        return Type::Unknown;
                    }
                    let a = self.check_expr(&args[0], scopes);
                    let b = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &a, args[0].span);
                    self.expect_type(&Type::String, &b, args[1].span);
                    Type::String
                } else if matches!(
                    name.as_str(),
                    "path_filename" | "path_extension" | "path_dir"
                ) {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects one String path"));
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    Type::String
                } else if name == "file_exists" || name == "dir_exists" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects one String path"));
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    Type::Bool
                } else if name == "file_size" {
                    if args.len() != 1 {
                        self.error(expr.span, "file_size expects one String path");
                        return Type::Unknown;
                    }
                    let s = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &s, args[0].span);
                    Type::Result(Box::new(Type::Int), Box::new(Type::String))
                } else if name == "date" {
                    if args.len() != 1 {
                        self.error(expr.span, "date expects one ISO date string (YYYY-MM-DD)");
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &value, args[0].span);
                    Type::Date
                } else if name == "decimal" {
                    if args.len() != 1 {
                        self.error(expr.span, "decimal expects one decimal string");
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &value, args[0].span);
                    Type::Decimal
                } else if name == "time" {
                    if args.len() != 1 {
                        self.error(expr.span, "time expects one ISO time string (HH:MM:SS)");
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &value, args[0].span);
                    Type::Time
                } else if name == "duration_ms" {
                    if args.len() != 1 {
                        self.error(expr.span, "duration_ms expects one Int");
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::Int, &value, args[0].span);
                    Type::Duration
                } else if name == "bytes" {
                    if args.len() != 1 {
                        self.error(
                            expr.span,
                            "bytes expects one Int array with values from 0 to 255",
                        );
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::Array(Box::new(Type::Int)), &value, args[0].span);
                    Type::Bytes
                } else if name == "format_timestamp" {
                    if args.len() != 1 {
                        self.error(expr.span, "format_timestamp expects exactly one Timestamp");
                        return Type::Unknown;
                    }
                    let timestamp = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::Timestamp, &timestamp, args[0].span);
                    Type::String
                } else if name == "format_text" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error_with_code(
                            "E-TYPE-FORMAT-001",
                            expr.span,
                            "format_text expects one scalar value and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    if !matches!(
                        value,
                        Type::Int
                            | Type::UInt
                            | Type::Float
                            | Type::Decimal
                            | Type::Bool
                            | Type::Char
                            | Type::String
                            | Type::Timestamp
                            | Type::Date
                            | Type::Time
                            | Type::Duration
                    ) {
                        self.error_with_code(
                            "E-TYPE-FORMAT-001",
                            args[0].span,
                            format!("format_text does not support `{value}`"),
                        );
                    }
                    Type::String
                } else if name == "env" {
                    if args.len() != 1 {
                        self.error(expr.span, "env expects exactly one argument");
                        return Type::Unknown;
                    }
                    let argument = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &argument, args[0].span);
                    Type::Option(Box::new(Type::String))
                } else if name == "random_int" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "random_int expects minimum and maximum Int arguments",
                        );
                        return Type::Unknown;
                    }
                    let minimum = self.check_expr(&args[0], scopes);
                    let maximum = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Int, &minimum, args[0].span);
                    self.expect_type(&Type::Int, &maximum, args[1].span);
                    Type::Int
                } else if name == "http_get" {
                    if args.len() != 1 {
                        self.error(expr.span, "http_get expects exactly one String URL");
                        return Type::Unknown;
                    }
                    let url = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &url, args[0].span);
                    Type::String
                } else if name == "json_encode" {
                    if !type_args.is_empty() {
                        self.error(expr.span, "json_encode does not accept type arguments");
                    }
                    if args.len() != 1 {
                        self.error(expr.span, "json_encode expects exactly one value");
                        return Type::Unknown;
                    }
                    let value_type = self.check_expr(&args[0], scopes);
                    if contains_class_reference_type(&value_type, self._program) {
                        self.error_with_code(
                            "E-TYPE-CLASS-001",
                            args[0].span,
                            "class references cannot be serialized directly; convert them to a record",
                        );
                    }
                    Type::String
                } else if name == "json_encode_tagged_result" {
                    if !type_args.is_empty() || args.len() != 1 {
                        self.error(
                            expr.span,
                            "json_encode_tagged_result expects one Result value and no type arguments",
                        );
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    if !matches!(value, Type::Result(_, _)) {
                        self.error(
                            args[0].span,
                            "json_encode_tagged_result expects a Result value",
                        );
                    }
                    if contains_class_reference_type(&value, self._program) {
                        self.error_with_code(
                            "E-TYPE-CLASS-001",
                            args[0].span,
                            "class references cannot be serialized directly; convert them to a record",
                        );
                    }
                    Type::String
                } else if name == "json_decode_tagged_result" {
                    if type_args.len() != 2 {
                        self.error(
                            expr.span,
                            "json_decode_tagged_result expects Ok and Err type arguments",
                        );
                    } else {
                        self.check_type(&type_args[0], expr.span);
                        self.check_type(&type_args[1], expr.span);
                    }
                    if args.len() != 1 {
                        self.error(
                            expr.span,
                            "json_decode_tagged_result expects one String value",
                        );
                        return Type::Unknown;
                    }
                    let source = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &source, args[0].span);
                    Type::Result(
                        Box::new(Type::Result(
                            Box::new(type_args.first().cloned().unwrap_or(Type::Unknown)),
                            Box::new(type_args.get(1).cloned().unwrap_or(Type::Unknown)),
                        )),
                        Box::new(Type::String),
                    )
                } else if name == "sql_decode" {
                    if type_args.len() != 1 || args.len() != 1 {
                        self.error_with_code(
                            "E-TYPE-SQL-DECODE",
                            expr.span,
                            "sql_decode expects one type argument and one direct SQL expression",
                        );
                        for argument in args {
                            self.check_expr(argument, scopes);
                        }
                        return Type::Unknown;
                    }
                    self.check_type(&type_args[0], expr.span);
                    self.check_expr(&args[0], scopes);
                    if let ExprKind::Sql { query, .. } = &args[0].kind {
                        if zelyra_database::sql::analyze_table_access(query, &[]).0
                            != Some("SELECT")
                        {
                            self.error_with_code(
                                "E-TYPE-SQL-DECODE",
                                args[0].span,
                                "sql_decode requires a SELECT query",
                            );
                        }
                    }
                    if !matches!(args[0].kind, ExprKind::Sql { .. }) {
                        self.error_with_code(
                            "E-TYPE-SQL-DECODE",
                            args[0].span,
                            "sql_decode requires a direct SQL expression",
                        );
                    }
                    if let Err(message) = sql_results::validate_target(&type_args[0], self._program)
                    {
                        self.error_with_code("E-TYPE-SQL-DECODE", expr.span, message);
                    }
                    type_args[0].clone()
                } else if name == "json_decode" {
                    if type_args.len() != 1 {
                        self.error(expr.span, "json_decode expects exactly one type argument");
                    } else {
                        self.check_type(&type_args[0], expr.span);
                    }
                    if args.len() != 1 {
                        self.error(expr.span, "json_decode expects exactly one String value");
                        return Type::Unknown;
                    }
                    let source = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &source, args[0].span);
                    let target = type_args.first().cloned().unwrap_or(Type::Unknown);
                    if contains_class_reference_type(&target, self._program) {
                        self.error_with_code(
                            "E-TYPE-CLASS-001",
                            expr.span,
                            "json_decode does not support class types because it cannot restore object identity",
                        );
                    }
                    target
                } else if name == "json_decode_result" {
                    if type_args.len() != 1 {
                        self.error(
                            expr.span,
                            "json_decode_result expects exactly one type argument",
                        );
                    } else {
                        self.check_type(&type_args[0], expr.span);
                    }
                    if args.len() != 1 {
                        self.error(
                            expr.span,
                            "json_decode_result expects exactly one String value",
                        );
                        return Type::Unknown;
                    }
                    let source = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &source, args[0].span);
                    Type::Result(
                        Box::new(type_args.first().cloned().unwrap_or(Type::Unknown)),
                        Box::new(Type::String),
                    )
                } else if name == "csv_decode_result" {
                    if !type_args.is_empty() {
                        self.error(
                            expr.span,
                            "csv_decode_result does not accept type arguments",
                        );
                    }
                    if args.len() != 2 {
                        self.error(expr.span, "csv_decode_result expects a String and Char");
                        return Type::Unknown;
                    }
                    let source = self.check_expr(&args[0], scopes);
                    let delimiter = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &source, args[0].span);
                    self.expect_type(&Type::Char, &delimiter, args[1].span);
                    Type::Result(
                        Box::new(Type::Array(Box::new(Type::Array(Box::new(Type::String))))),
                        Box::new(Type::String),
                    )
                } else if name == "csv_encode" {
                    if !type_args.is_empty() {
                        self.error(expr.span, "csv_encode does not accept type arguments");
                    }
                    if args.len() != 2 {
                        self.error(expr.span, "csv_encode expects String[][] rows and a Char");
                        return Type::Unknown;
                    }
                    let rows = self.check_expr(&args[0], scopes);
                    let delimiter = self.check_expr(&args[1], scopes);
                    self.expect_type(
                        &Type::Array(Box::new(Type::Array(Box::new(Type::String)))),
                        &rows,
                        args[0].span,
                    );
                    self.expect_type(&Type::Char, &delimiter, args[1].span);
                    Type::Result(Box::new(Type::String), Box::new(Type::String))
                } else if name == "decimal_scaled_result" {
                    if !type_args.is_empty() {
                        self.error(
                            expr.span,
                            "decimal_scaled_result does not accept type arguments",
                        );
                    }
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "decimal_scaled_result expects a String and scale Int",
                        );
                        return Type::Unknown;
                    }
                    let source = self.check_expr(&args[0], scopes);
                    let scale = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &source, args[0].span);
                    self.expect_type(&Type::Int, &scale, args[1].span);
                    Type::Result(Box::new(Type::Int), Box::new(Type::String))
                } else if name == "format_scaled_result" {
                    if !type_args.is_empty() {
                        self.error(
                            expr.span,
                            "format_scaled_result does not accept type arguments",
                        );
                    }
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "format_scaled_result expects an Int and scale Int",
                        );
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    let scale = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Int, &value, args[0].span);
                    self.expect_type(&Type::Int, &scale, args[1].span);
                    Type::Result(Box::new(Type::String), Box::new(Type::String))
                } else if matches!(
                    name.as_str(),
                    "date_add_days_result" | "date_days_between_result"
                ) {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            format!("{name} expects two arguments and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let left = self.check_expr(&args[0], scopes);
                    let right = self.check_expr(&args[1], scopes);
                    if name == "date_add_days_result" {
                        self.expect_type(&Type::Date, &left, args[0].span);
                        self.expect_type(&Type::Int, &right, args[1].span);
                        Type::Result(Box::new(Type::Date), Box::new(Type::String))
                    } else {
                        self.expect_type(&Type::Date, &left, args[0].span);
                        self.expect_type(&Type::Date, &right, args[1].span);
                        Type::Result(Box::new(Type::Int), Box::new(Type::String))
                    }
                } else if matches!(
                    name.as_str(),
                    "timestamp_add_result" | "timestamp_difference_result"
                ) {
                    if !type_args.is_empty() || args.len() != 2 {
                        self.error(
                            expr.span,
                            format!("{name} expects two arguments and no type arguments"),
                        );
                        return Type::Unknown;
                    }
                    let left = self.check_expr(&args[0], scopes);
                    let right = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::Timestamp, &left, args[0].span);
                    if name == "timestamp_add_result" {
                        self.expect_type(&Type::Duration, &right, args[1].span);
                        Type::Result(Box::new(Type::Timestamp), Box::new(Type::String))
                    } else {
                        self.expect_type(&Type::Timestamp, &right, args[1].span);
                        Type::Result(Box::new(Type::Duration), Box::new(Type::String))
                    }
                } else if name == "decimal_quantize_result" {
                    if !type_args.is_empty() || args.len() != 3 {
                        self.error(
                            expr.span,
                            "decimal_quantize_result expects Decimal, scale Int, and mode String",
                        );
                        return Type::Unknown;
                    }
                    let value = self.check_expr(&args[0], scopes);
                    let scale = self.check_expr(&args[1], scopes);
                    let mode = self.check_expr(&args[2], scopes);
                    self.expect_type(&Type::Decimal, &value, args[0].span);
                    self.expect_type(&Type::Int, &scale, args[1].span);
                    self.expect_type(&Type::String, &mode, args[2].span);
                    Type::Result(Box::new(Type::Decimal), Box::new(Type::String))
                } else if name == "http_json" {
                    if type_args.len() != 2 {
                        self.error(
                            expr.span,
                            "http_json expects request and response type arguments",
                        );
                    } else {
                        self.check_type(&type_args[0], expr.span);
                        self.check_type(&type_args[1], expr.span);
                    }
                    if args.len() != 4 {
                        self.error(
                            expr.span,
                            "http_json expects String method, String URL, String[] headers, and Request? body",
                        );
                        return Type::Unknown;
                    }
                    let method = self.check_expr(&args[0], scopes);
                    let url = self.check_expr(&args[1], scopes);
                    let headers = self.check_expr(&args[2], scopes);
                    let body = self.check_expr(&args[3], scopes);
                    self.expect_type(&Type::String, &method, args[0].span);
                    self.expect_type(&Type::String, &url, args[1].span);
                    self.expect_type(&Type::Array(Box::new(Type::String)), &headers, args[2].span);
                    if let Some(request_type) = type_args.first() {
                        self.expect_type(
                            &Type::Option(Box::new(request_type.clone())),
                            &body,
                            args[3].span,
                        );
                    }
                    type_args.get(1).cloned().unwrap_or(Type::Unknown)
                } else if name == "http_result" {
                    if type_args.len() != 2 {
                        self.error(
                            expr.span,
                            "http_result expects request and response type arguments",
                        );
                    } else {
                        self.check_type(&type_args[0], expr.span);
                        self.check_type(&type_args[1], expr.span);
                    }
                    if args.len() != 4 {
                        self.error(
                            expr.span,
                            "http_result expects String method, String URL, String[] headers, and Request? body",
                        );
                        return Type::Unknown;
                    }
                    let method = self.check_expr(&args[0], scopes);
                    let url = self.check_expr(&args[1], scopes);
                    let headers = self.check_expr(&args[2], scopes);
                    let body = self.check_expr(&args[3], scopes);
                    self.expect_type(&Type::String, &method, args[0].span);
                    self.expect_type(&Type::String, &url, args[1].span);
                    self.expect_type(&Type::Array(Box::new(Type::String)), &headers, args[2].span);
                    if let Some(request_type) = type_args.first() {
                        self.expect_type(
                            &Type::Option(Box::new(request_type.clone())),
                            &body,
                            args[3].span,
                        );
                    }
                    Type::HttpResult(Box::new(type_args.get(1).cloned().unwrap_or(Type::Unknown)))
                } else if name == "http_request" {
                    if args.len() != 4 {
                        self.error(
                            expr.span,
                            "http_request expects String method, String URL, String[] headers, and String? body",
                        );
                        return Type::Unknown;
                    }
                    let method = self.check_expr(&args[0], scopes);
                    let url = self.check_expr(&args[1], scopes);
                    let headers = self.check_expr(&args[2], scopes);
                    let body = self.check_expr(&args[3], scopes);
                    self.expect_type(&Type::String, &method, args[0].span);
                    self.expect_type(&Type::String, &url, args[1].span);
                    self.expect_type(&Type::Array(Box::new(Type::String)), &headers, args[2].span);
                    self.expect_type(&Type::Option(Box::new(Type::String)), &body, args[3].span);
                    Type::Named("HttpResponse".into())
                } else if name == "mail_queue" {
                    if !type_args.is_empty() {
                        self.error(expr.span, "mail_queue does not accept type arguments");
                    }
                    if self.transaction_depth == 0 {
                        self.error(expr.span, "mail_queue is only allowed inside a transaction");
                    }
                    if !matches!(args.len(), 3..=6) {
                        self.error(
                            expr.span,
                            "mail_queue expects recipient, subject, text body, optional HTML body, and optional PDF filename and Bytes",
                        );
                        return Type::Unknown;
                    }
                    for (index, argument) in args.iter().enumerate() {
                        let is_pdf_argument = matches!((args.len(), index), (5, 4) | (6, 5));
                        if is_pdf_argument
                            && !matches!(
                                &argument.kind,
                                ExprKind::Call { name, .. } if name == "render_pdf"
                            )
                        {
                            self.error(
                                argument.span,
                                "mail_queue PDF attachments must be produced directly by render_pdf",
                            );
                        }
                        if !is_pure_mail_expression(argument) {
                            self.error(
                                argument.span,
                                "mail_queue arguments must be values or pure expressions; PDF attachments must use render_pdf",
                            );
                        }
                        let actual = self.check_expr(argument, scopes);
                        let expected = match (args.len(), index) {
                            (5, 4) | (6, 5) => Type::Bytes,
                            _ => Type::String,
                        };
                        self.expect_type(&expected, &actual, argument.span);
                    }
                    Type::Unit
                } else if name == "mail_deliver" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "mail_deliver expects an outbox event ID and payload JSON",
                        );
                        return Type::Unknown;
                    }
                    for argument in args {
                        let actual = self.check_expr(argument, scopes);
                        self.expect_type(&Type::String, &actual, argument.span);
                    }
                    Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                } else if name == "run_process" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "run_process expects a String command and String[] arguments",
                        );
                        return Type::Unknown;
                    }
                    let command = self.check_expr(&args[0], scopes);
                    let arguments = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &command, args[0].span);
                    self.expect_type(
                        &Type::Array(Box::new(Type::String)),
                        &arguments,
                        args[1].span,
                    );
                    Type::String
                } else if name == "read_text" {
                    if args.len() != 1 {
                        self.error(expr.span, "read_text expects exactly one String path");
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    Type::String
                } else if name == "read_text_result" {
                    if args.len() != 1 {
                        self.error(
                            expr.span,
                            "read_text_result expects exactly one String path",
                        );
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    Type::Result(Box::new(Type::String), Box::new(Type::String))
                } else if name == "write_text" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "write_text expects a String path and String content",
                        );
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    let content = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    self.expect_type(&Type::String, &content, args[1].span);
                    Type::Unit
                } else if name == "write_text_result" {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            "write_text_result expects a String path and String content",
                        );
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    let content = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    self.expect_type(&Type::String, &content, args[1].span);
                    Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                } else if matches!(
                    name.as_str(),
                    "write_text_new_result" | "atomic_write_text_result" | "append_text_result"
                ) {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            format!("{name} expects a String path and String content"),
                        );
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    let content = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    self.expect_type(&Type::String, &content, args[1].span);
                    Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                } else if matches!(name.as_str(), "copy_file_new_result" | "rename_file_result") {
                    if args.len() != 2 {
                        self.error(
                            expr.span,
                            format!("{name} expects source and destination String paths"),
                        );
                        return Type::Unknown;
                    }
                    let source = self.check_expr(&args[0], scopes);
                    let destination = self.check_expr(&args[1], scopes);
                    self.expect_type(&Type::String, &source, args[0].span);
                    self.expect_type(&Type::String, &destination, args[1].span);
                    Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                } else if name == "delete_file" || name == "delete_file_result" {
                    if args.len() != 1 {
                        self.error(expr.span, format!("{name} expects one String path"));
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    if name == "delete_file_result" {
                        Type::Result(Box::new(Type::Unit), Box::new(Type::String))
                    } else {
                        Type::Unit
                    }
                } else if name == "list_dir" {
                    if args.len() != 1 {
                        self.error(expr.span, "list_dir expects one String path");
                        return Type::Unknown;
                    }
                    let path = self.check_expr(&args[0], scopes);
                    self.expect_type(&Type::String, &path, args[0].span);
                    Type::Array(Box::new(Type::String))
                } else if let Some(variable) = self.lookup(scopes, name).cloned() {
                    let Type::Function {
                        params,
                        return_type,
                        ..
                    } = variable.ty
                    else {
                        self.error(expr.span, format!("value `{name}` is not callable"));
                        return Type::Unknown;
                    };
                    if !type_args.is_empty() {
                        self.error_with_code(
                            "E-TYPE-FUNCTION-001",
                            expr.span,
                            "indirect function calls do not accept explicit type arguments",
                        );
                    }
                    if args.len() != params.len() {
                        self.error(
                            expr.span,
                            format!(
                                "function value `{name}` expects {} argument(s), found {}",
                                params.len(),
                                args.len()
                            ),
                        );
                    }
                    for (index, argument) in args.iter().enumerate() {
                        let actual = self.check_expr(argument, scopes);
                        if let Some(expected) = params.get(index) {
                            self.expect_type(expected, &actual, argument.span);
                        }
                    }
                    *return_type
                } else if let Some(signature) = self.functions.get(name).cloned() {
                    if args.len() != signature.params.len() {
                        self.error(
                            expr.span,
                            format!(
                                "function `{name}` expects {} argument(s), found {}",
                                signature.params.len(),
                                args.len()
                            ),
                        );
                    }
                    let actual_types = args
                        .iter()
                        .map(|argument| self.check_expr(argument, scopes))
                        .collect::<Vec<_>>();
                    if signature.type_params.is_empty() {
                        if !type_args.is_empty() {
                            self.error(
                                expr.span,
                                format!("function `{name}` does not accept type arguments"),
                            );
                        }
                        for (index, actual) in actual_types.iter().enumerate() {
                            if let Some(expected) = signature.params.get(index) {
                                self.expect_type(expected, actual, args[index].span);
                            }
                        }
                        signature.return_type
                    } else {
                        let parameters = signature
                            .type_params
                            .iter()
                            .cloned()
                            .collect::<HashSet<_>>();
                        let mut substitutions = HashMap::new();
                        if !type_args.is_empty() {
                            if type_args.len() != signature.type_params.len() {
                                self.error_with_code(
                                    "E-TYPE-GENERIC-001",
                                    expr.span,
                                    format!(
                                        "generic function `{name}` expects {} type argument(s), found {}",
                                        signature.type_params.len(),
                                        type_args.len()
                                    ),
                                );
                            }
                            for argument in type_args {
                                self.check_type(argument, expr.span);
                            }
                            for (parameter, argument) in signature.type_params.iter().zip(type_args)
                            {
                                substitutions.insert(parameter.clone(), argument.clone());
                            }
                        }
                        for (index, actual) in actual_types.iter().enumerate() {
                            if let Some(expected) = signature.params.get(index) {
                                if type_args.is_empty() {
                                    infer_generic_types(
                                        expected,
                                        actual,
                                        &parameters,
                                        &mut substitutions,
                                    );
                                }
                            }
                        }
                        for parameter in &signature.type_params {
                            if !substitutions.contains_key(parameter) {
                                self.error_with_code(
                                    "E-TYPE-GENERIC-001",
                                    expr.span,
                                    format!(
                                        "cannot infer type parameter `{parameter}` for `{name}`; provide explicit type arguments"
                                    ),
                                );
                                substitutions.insert(parameter.clone(), Type::Unknown);
                            }
                        }
                        for (index, actual) in actual_types.iter().enumerate() {
                            if let Some(expected) = signature.params.get(index) {
                                let expected = substitute_generic_type(expected, &substitutions);
                                self.expect_type(&expected, actual, args[index].span);
                            }
                        }
                        substitute_generic_type(&signature.return_type, &substitutions)
                    }
                } else {
                    self.error(expr.span, format!("unknown function `{name}`"));
                    Type::Unknown
                }
            }
            ExprKind::Unary { op, expr: inner } => {
                let ty = self.check_expr(inner, scopes);
                match op {
                    UnaryOp::Negate => {
                        if !is_numeric(&ty) && ty != Type::Unknown {
                            self.error(expr.span, "unary `-` requires a numeric value");
                        }
                        ty
                    }
                    UnaryOp::Not => {
                        self.expect_type(&Type::Bool, &ty, inner.span);
                        Type::Bool
                    }
                }
            }
            ExprKind::Index { target, index } => {
                let target_type = self.check_expr(target, scopes);
                let index_type = self.check_expr(index, scopes);
                if !matches!(index_type, Type::Int | Type::UInt | Type::Unknown) {
                    self.error(
                        index.span,
                        format!("array index must be an integer, found `{index_type}`"),
                    );
                }
                match target_type {
                    Type::Array(inner) => *inner,
                    Type::Tuple(items) => match index.kind {
                        ExprKind::Int(value) if value >= 0 => {
                            items.get(value as usize).cloned().unwrap_or_else(|| {
                                self.error(
                                    index.span,
                                    format!("tuple index {value} is out of bounds"),
                                );
                                Type::Unknown
                            })
                        }
                        ExprKind::UInt(value) => {
                            items.get(value as usize).cloned().unwrap_or_else(|| {
                                self.error(
                                    index.span,
                                    format!("tuple index {value} is out of bounds"),
                                );
                                Type::Unknown
                            })
                        }
                        _ => {
                            self.error(
                                index.span,
                                "tuple index must be a non-negative integer literal",
                            );
                            Type::Unknown
                        }
                    },
                    Type::Unknown => Type::Unknown,
                    other => {
                        self.error(
                            target.span,
                            format!("indexing requires an array or Tuple, found `{other}`"),
                        );
                        Type::Unknown
                    }
                }
            }
            ExprKind::Field { target, field } => {
                let target_type = self.check_expr(target, scopes);
                match self.record_field_type(&target_type, field) {
                    Some(field_type) => field_type,
                    None if target_type == Type::Unknown => Type::Unknown,
                    None => {
                        self.error(
                            expr.span,
                            format!("unknown field `{field}` on `{target_type}`"),
                        );
                        Type::Unknown
                    }
                }
            }
            ExprKind::Binary { left, op, right } => {
                let _ = (left, op, right);
                self.check_binary_chain(expr, scopes)
            }
            ExprKind::Sql { result_type, .. } => {
                self.check_type(result_type, expr.span);
                result_type.clone()
            }
            ExprKind::Await(inner) => {
                if self.parallel_depth == 0 {
                    self.error(expr.span, "`await` is only valid inside a `parallel` block");
                }
                self.check_expr(inner, scopes)
            }
            ExprKind::Match { value, arms } => {
                let value_type = self.check_expr(value, scopes);
                let mut covered = std::collections::HashSet::new();
                let mut result_type = Type::Unknown;
                for arm in arms {
                    let mut pattern_scope = HashMap::new();
                    self.check_pattern(
                        &arm.pattern,
                        &value_type,
                        &mut pattern_scope,
                        &mut covered,
                        arm.guard.is_none(),
                    );
                    let mut arm_scopes = scopes.to_vec();
                    arm_scopes.push(pattern_scope);
                    if let Some(guard) = &arm.guard {
                        let guard_type = self.check_expr(guard, &arm_scopes);
                        self.expect_type(&Type::Bool, &guard_type, guard.span);
                    }
                    let arm_type = self.check_expr(&arm.value, &arm_scopes);
                    if result_type == Type::Unknown {
                        result_type = arm_type;
                    } else {
                        self.expect_type(&result_type, &arm_type, arm.value.span);
                    }
                }
                self.check_exhaustiveness(&value_type, &covered, expr.span);
                result_type
            }
        }
    }
    fn check_binary_chain(&mut self, expr: &Expr, scopes: &[HashMap<String, Variable>]) -> Type {
        let mut cursor = expr;
        let mut operations = Vec::new();
        while let ExprKind::Binary { left, op, right } = &cursor.kind {
            operations.push((*op, right.as_ref(), cursor.span, left.span));
            cursor = left;
        }
        let mut left_type = self.check_expr(cursor, scopes);
        for (op, right, span, left_span) in operations.into_iter().rev() {
            let right_type = self.check_expr(right, scopes);
            left_type = match op {
                BinaryOp::Add => {
                    if left_type == Type::String && right_type == Type::String {
                        Type::String
                    } else if left_type == Type::Duration && right_type == Type::Duration {
                        Type::Duration
                    } else if let (Type::Array(left), Type::Array(right)) =
                        (&left_type, &right_type)
                    {
                        self.expect_type(left, right, span);
                        Type::Array(left.clone())
                    } else {
                        self.numeric_result(&left_type, &right_type, span)
                    }
                }
                BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Remainder => {
                    if op == BinaryOp::Subtract
                        && left_type == Type::Duration
                        && right_type == Type::Duration
                    {
                        Type::Duration
                    } else {
                        self.numeric_result(&left_type, &right_type, span)
                    }
                }
                BinaryOp::Equal | BinaryOp::NotEqual => {
                    if matches!(&left_type, Type::Function { .. })
                        || matches!(&right_type, Type::Function { .. })
                    {
                        self.error_with_code(
                            "E-TYPE-FUNCTION-001",
                            span,
                            "function values cannot be compared",
                        );
                    }
                    self.expect_type(&left_type, &right_type, right.span);
                    Type::Bool
                }
                BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual => {
                    if left_type == right_type
                        && matches!(
                            left_type,
                            Type::String
                                | Type::Date
                                | Type::Time
                                | Type::Duration
                                | Type::Timestamp
                        )
                    {
                        Type::Bool
                    } else {
                        self.numeric_result(&left_type, &right_type, span);
                        Type::Bool
                    }
                }
                BinaryOp::And | BinaryOp::Or => {
                    self.expect_type(&Type::Bool, &left_type, left_span);
                    self.expect_type(&Type::Bool, &right_type, right.span);
                    Type::Bool
                }
            };
        }
        left_type
    }
    fn numeric_result(&mut self, left: &Type, right: &Type, span: Span) -> Type {
        if left == &Type::Unknown || right == &Type::Unknown {
            return Type::Unknown;
        }
        if is_numeric(left) && left == right {
            left.clone()
        } else {
            self.error(
                span,
                format!("numeric operands must have the same type, found `{left}` and `{right}`"),
            );
            Type::Unknown
        }
    }

    fn record_field_type(&self, ty: &Type, field: &str) -> Option<Type> {
        if let Type::HttpResult(response) = ty {
            return match field {
                "status" => Some(Type::Int),
                "headers" => Some(Type::Array(Box::new(Type::String))),
                "body" => Some(Type::String),
                "data" => Some(Type::Option(response.clone())),
                "error" => Some(Type::Option(Box::new(Type::Named("HttpError".into())))),
                _ => None,
            };
        }
        let (initial_name, substitutions) = match ty {
            Type::Named(name) => (name.as_str(), HashMap::new()),
            Type::Applied { name, args } => {
                let record = self
                    ._program
                    .records
                    .iter()
                    .find(|record| record.name == *name)?;
                let substitutions = record
                    .type_params
                    .iter()
                    .zip(args)
                    .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                    .collect::<HashMap<_, _>>();
                let field_type = record
                    .fields
                    .iter()
                    .find(|candidate| candidate.name == field)?
                    .ty
                    .clone();
                return Some(substitute_generic_type(&field_type, &substitutions));
            }
            _ => return None,
        };
        if initial_name == "HttpError" {
            return match field {
                "status" => Some(Type::Int),
                "headers" => Some(Type::Array(Box::new(Type::String))),
                "body" | "message" => Some(Type::String),
                _ => None,
            };
        }
        let mut name = initial_name;
        if name == "HttpResponse" {
            return match field {
                "status" => Some(Type::Int),
                "headers" => Some(Type::Array(Box::new(Type::String))),
                "body" => Some(Type::String),
                _ => None,
            };
        }
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(name) {
                return None;
            }
            if let Some(record) = self
                ._program
                .records
                .iter()
                .find(|record| record.name == name)
            {
                return record
                    .fields
                    .iter()
                    .find(|candidate| candidate.name == field)
                    .map(|candidate| substitute_generic_type(&candidate.ty, &substitutions));
            }
            if let Some(record) = sql_results::definitions(self._program).get(name) {
                return record
                    .fields
                    .iter()
                    .find(|candidate| candidate.name == field)
                    .map(|candidate| candidate.ty.clone());
            }
            let alias = self
                ._program
                .types
                .iter()
                .find(|definition| definition.name == name)?;
            let Type::Named(next_name) = &alias.target else {
                return None;
            };
            name = next_name;
        }
    }
}

fn compatible(expected: &Type, actual: &Type) -> bool {
    if expected == &Type::Unknown || actual == &Type::Unknown {
        return true;
    }
    match (expected, actual) {
        (Type::Option(expected), Type::Option(actual)) => compatible(expected, actual),
        (Type::Result(expected_ok, expected_error), Type::Result(actual_ok, actual_error)) => {
            compatible(expected_ok, actual_ok) && compatible(expected_error, actual_error)
        }
        (Type::Array(expected), Type::Array(actual)) => compatible(expected, actual),
        (
            Type::Applied {
                name: expected_name,
                args: expected_args,
            },
            Type::Applied {
                name: actual_name,
                args: actual_args,
            },
        ) => {
            expected_name == actual_name
                && expected_args.len() == actual_args.len()
                && expected_args
                    .iter()
                    .zip(actual_args)
                    .all(|(expected, actual)| compatible(expected, actual))
        }
        (Type::Map(expected_key, expected_value), Type::Map(actual_key, actual_value)) => {
            compatible(expected_key, actual_key) && compatible(expected_value, actual_value)
        }
        (Type::HttpResult(expected), Type::HttpResult(actual)) => compatible(expected, actual),
        (
            Type::Function {
                params: expected_params,
                return_type: expected_return,
                capabilities: expected_capabilities,
            },
            Type::Function {
                params: actual_params,
                return_type: actual_return,
                capabilities: actual_capabilities,
            },
        ) => {
            expected_params == actual_params
                && expected_return == actual_return
                && actual_capabilities
                    .iter()
                    .all(|capability| expected_capabilities.contains(capability))
        }
        (Type::HttpResult(_), Type::Named(actual)) if actual == "HttpResult" => true,
        _ => expected == actual,
    }
}

fn infer_generic_types(
    expected: &Type,
    actual: &Type,
    parameters: &HashSet<String>,
    substitutions: &mut HashMap<String, Type>,
) {
    if *actual == Type::Unknown {
        return;
    }
    match expected {
        Type::Named(name) if parameters.contains(name) => {
            substitutions
                .entry(name.clone())
                .or_insert_with(|| actual.clone());
        }
        Type::Option(expected) => {
            if let Type::Option(actual) = actual {
                infer_generic_types(expected, actual, parameters, substitutions);
            }
        }
        Type::Result(expected_ok, expected_error) => {
            if let Type::Result(actual_ok, actual_error) = actual {
                infer_generic_types(expected_ok, actual_ok, parameters, substitutions);
                infer_generic_types(expected_error, actual_error, parameters, substitutions);
            }
        }
        Type::Array(expected) | Type::Set(expected) => match actual {
            Type::Array(actual) | Type::Set(actual) => {
                infer_generic_types(expected, actual, parameters, substitutions);
            }
            _ => {}
        },
        Type::Tuple(expected) => {
            if let Type::Tuple(actual) = actual {
                if expected.len() == actual.len() {
                    for (expected, actual) in expected.iter().zip(actual) {
                        infer_generic_types(expected, actual, parameters, substitutions);
                    }
                }
            }
        }
        Type::Map(expected_key, expected_value) => {
            if let Type::Map(actual_key, actual_value) = actual {
                infer_generic_types(expected_key, actual_key, parameters, substitutions);
                infer_generic_types(expected_value, actual_value, parameters, substitutions);
            }
        }
        Type::HttpResult(expected) => {
            if let Type::HttpResult(actual) = actual {
                infer_generic_types(expected, actual, parameters, substitutions);
            }
        }
        Type::Applied {
            name: expected_name,
            args: expected_args,
        } => {
            if let Type::Applied {
                name: actual_name,
                args: actual_args,
            } = actual
            {
                if expected_name == actual_name && expected_args.len() == actual_args.len() {
                    for (expected, actual) in expected_args.iter().zip(actual_args) {
                        infer_generic_types(expected, actual, parameters, substitutions);
                    }
                }
            }
        }
        Type::Function {
            params: expected_params,
            return_type: expected_return,
            ..
        } => {
            if let Type::Function {
                params: actual_params,
                return_type: actual_return,
                ..
            } = actual
            {
                if expected_params.len() == actual_params.len() {
                    for (expected, actual) in expected_params.iter().zip(actual_params) {
                        infer_generic_types(expected, actual, parameters, substitutions);
                    }
                    infer_generic_types(expected_return, actual_return, parameters, substitutions);
                }
            }
        }
        _ => {}
    }
}

fn alias_shape_has_names(ty: &Type, aliases: &HashMap<String, Type>) -> Result<bool, &'static str> {
    let mut stack = vec![(ty, 0)];
    let mut nodes = 0;
    let mut has_alias = false;
    while let Some((ty, depth)) = stack.pop() {
        nodes += 1;
        if nodes > 4096 || depth > 64 {
            return Err("type alias expansion exceeds 64 levels or 4096 type nodes");
        }
        match ty {
            Type::Named(name) => has_alias |= aliases.contains_key(name),
            Type::Option(inner)
            | Type::Array(inner)
            | Type::Set(inner)
            | Type::HttpResult(inner) => stack.push((inner, depth + 1)),
            Type::Result(left, right) | Type::Map(left, right) => {
                stack.push((left, depth + 1));
                stack.push((right, depth + 1));
            }
            Type::Tuple(items) | Type::Applied { args: items, .. } => {
                stack.extend(items.iter().map(|item| (item, depth + 1)))
            }
            Type::Function {
                params,
                return_type,
                ..
            } => {
                stack.extend(params.iter().map(|item| (item, depth + 1)));
                stack.push((return_type, depth + 1));
            }
            _ => {}
        }
    }
    Ok(has_alias)
}

fn expand_type_aliases(ty: &Type, aliases: &HashMap<String, Type>) -> Result<Type, &'static str> {
    let mut resolved = ty.clone();
    for _ in 0..=aliases.len().min(64) {
        if !alias_shape_has_names(&resolved, aliases)? {
            return Ok(resolved);
        }
        let next = substitute_generic_type(&resolved, aliases);
        if next == resolved {
            return Err("cyclic type alias");
        }
        resolved = next;
    }
    Err("cyclic or excessively deep type alias")
}

fn substitute_generic_type(ty: &Type, substitutions: &HashMap<String, Type>) -> Type {
    match ty {
        Type::Named(name) => substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        Type::Applied { name, args } => Type::Applied {
            name: name.clone(),
            args: args
                .iter()
                .map(|argument| substitute_generic_type(argument, substitutions))
                .collect(),
        },
        Type::Option(inner) => {
            Type::Option(Box::new(substitute_generic_type(inner, substitutions)))
        }
        Type::Result(ok, error) => Type::Result(
            Box::new(substitute_generic_type(ok, substitutions)),
            Box::new(substitute_generic_type(error, substitutions)),
        ),
        Type::Array(inner) => Type::Array(Box::new(substitute_generic_type(inner, substitutions))),
        Type::Tuple(items) => Type::Tuple(
            items
                .iter()
                .map(|item| substitute_generic_type(item, substitutions))
                .collect(),
        ),
        Type::Set(inner) => Type::Set(Box::new(substitute_generic_type(inner, substitutions))),
        Type::Map(key, value) => Type::Map(
            Box::new(substitute_generic_type(key, substitutions)),
            Box::new(substitute_generic_type(value, substitutions)),
        ),
        Type::HttpResult(inner) => {
            Type::HttpResult(Box::new(substitute_generic_type(inner, substitutions)))
        }
        Type::Function {
            params,
            return_type,
            capabilities,
        } => Type::Function {
            params: params
                .iter()
                .map(|parameter| substitute_generic_type(parameter, substitutions))
                .collect(),
            return_type: Box::new(substitute_generic_type(return_type, substitutions)),
            capabilities: capabilities.clone(),
        },
        _ => ty.clone(),
    }
}

fn is_numeric(ty: &Type) -> bool {
    matches!(ty, Type::Int | Type::UInt | Type::Float | Type::Decimal)
}

fn is_map_key_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Int | Type::UInt | Type::String | Type::Bool | Type::Char | Type::Timestamp
    )
}

fn validate_type(
    ty: &Type,
    known_types: &std::collections::HashSet<String>,
    generic_arities: &HashMap<String, usize>,
    errors: &mut Vec<TypeError>,
    span: Span,
) {
    match ty {
        Type::Named(name) if !known_types.contains(name) => errors.push(TypeError {
            code: "E-TYPE-001",
            message: format!("unknown type `{name}`"),
            span,
        }),
        Type::Named(name) if generic_arities.get(name).is_some_and(|arity| *arity > 0) => {
            errors.push(TypeError {
                code: "E-TYPE-GENERIC-001",
                message: format!("generic type `{name}` requires type arguments"),
                span,
            });
        }
        Type::Applied { name, args } => {
            if !known_types.contains(name) {
                errors.push(TypeError {
                    code: "E-TYPE-001",
                    message: format!("unknown type `{name}`"),
                    span,
                });
            }
            if let Some(arity) = generic_arities.get(name) {
                if *arity != args.len() {
                    errors.push(TypeError {
                        code: "E-TYPE-GENERIC-001",
                        message: format!(
                            "generic type `{name}` expects {arity} type argument(s), found {}",
                            args.len()
                        ),
                        span,
                    });
                }
            } else if known_types.contains(name) {
                errors.push(TypeError {
                    code: "E-TYPE-GENERIC-001",
                    message: format!(
                        "type `{name}` is not generic and does not accept type arguments"
                    ),
                    span,
                });
            }
            for argument in args {
                validate_type(argument, known_types, generic_arities, errors, span);
            }
        }
        Type::Option(inner) => validate_type(inner, known_types, generic_arities, errors, span),
        Type::Result(ok, error) => {
            validate_type(ok, known_types, generic_arities, errors, span);
            validate_type(error, known_types, generic_arities, errors, span);
        }
        Type::Array(inner) | Type::Set(inner) => {
            validate_type(inner, known_types, generic_arities, errors, span)
        }
        Type::Tuple(items) => {
            for item in items {
                validate_type(item, known_types, generic_arities, errors, span);
            }
        }
        Type::Map(key, value) => {
            validate_type(key, known_types, generic_arities, errors, span);
            validate_type(value, known_types, generic_arities, errors, span);
            if !matches!(key.as_ref(), Type::Unknown) && !is_map_key_type(key) {
                errors.push(TypeError {
                    code: "E-TYPE-001",
                    message: format!(
                        "Map keys must be Int, UInt, String, Bool, Char, or Timestamp; found `{key}`"
                    ),
                    span,
                });
            }
        }
        Type::HttpResult(inner) => validate_type(inner, known_types, generic_arities, errors, span),
        Type::Function {
            params,
            return_type,
            capabilities,
        } => {
            for parameter in params {
                validate_type(parameter, known_types, generic_arities, errors, span);
            }
            validate_type(return_type, known_types, generic_arities, errors, span);
            for capability in capabilities {
                if !KNOWN_CAPABILITIES.contains(&capability.as_str()) {
                    errors.push(TypeError {
                        code: "E-TYPE-FUNCTION-001",
                        message: format!("unknown capability `{capability}` in function type"),
                        span,
                    });
                }
            }
        }
        _ => {}
    }
}

fn singular_table_type(name: &str) -> Option<String> {
    let singular = if let Some(stem) = name.strip_suffix("ies") {
        format!("{stem}y")
    } else if let Some(stem) = name.strip_suffix('s') {
        stem.to_owned()
    } else {
        name.to_owned()
    };
    let mut chars = singular.chars();
    let first = chars.next()?.to_ascii_uppercase();
    Some(std::iter::once(first).chain(chars).collect())
}

#[derive(Clone, Debug, PartialEq)]
pub struct SqlRowsMetadata {
    pub nulls: Vec<Vec<bool>>,
    pub column_types: Vec<zelyra_database::QueryColumnType>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    UInt(u64),
    Float(f64),
    Decimal(i128),
    Bool(bool),
    String(String),
    Char(char),
    Enum {
        type_name: String,
        type_args: Vec<Type>,
        variant: String,
        payloads: Vec<Value>,
    },
    Timestamp(i64),
    Date(String),
    Time(String),
    Duration(i64),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Tuple(Vec<Value>),
    Set(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Function(Box<FunctionValue>),
    Object {
        type_name: String,
        type_args: Vec<Type>,
        fields: HashMap<String, Value>,
    },
    ClassRef {
        reference: ClassObjectRef,
        type_name: String,
        type_args: Vec<Type>,
    },
    Option(Option<Box<Value>>),
    Result(Result<Box<Value>, Box<Value>>),
    Rows {
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
        metadata: Box<SqlRowsMetadata>,
        result_type: Box<Type>,
    },
    Unit,
}

#[derive(Clone, Debug)]
pub struct FunctionValue {
    pub name: String,
    pub params: Vec<Type>,
    pub return_type: Type,
    pub capabilities: Vec<String>,
    pub lambda_params: Vec<String>,
    pub lambda_body: Option<Box<Expr>>,
    pub captures: Vec<(String, Value)>,
}

impl PartialEq for FunctionValue {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.params == other.params
            && self.return_type == other.return_type
            && self.capabilities == other.capabilities
            && self.lambda_params == other.lambda_params
            && self.captures == other.captures
            && self.lambda_body.is_some() == other.lambda_body.is_some()
    }
}

impl Value {
    pub fn ty(&self) -> Type {
        match self {
            Value::Int(_) => Type::Int,
            Value::UInt(_) => Type::UInt,
            Value::Float(_) => Type::Float,
            Value::Decimal(_) => Type::Decimal,
            Value::Bool(_) => Type::Bool,
            Value::String(_) => Type::String,
            Value::Char(_) => Type::Char,
            Value::Enum {
                type_name,
                type_args,
                ..
            } => {
                if type_args.is_empty() {
                    Type::Named(type_name.clone())
                } else {
                    Type::Applied {
                        name: type_name.clone(),
                        args: type_args.clone(),
                    }
                }
            }
            Value::Function(function) => Type::Function {
                params: function.params.clone(),
                return_type: Box::new(function.return_type.clone()),
                capabilities: function.capabilities.clone(),
            },
            Value::Timestamp(_) => Type::Timestamp,
            Value::Date(_) => Type::Date,
            Value::Time(_) => Type::Time,
            Value::Duration(_) => Type::Duration,
            Value::Bytes(_) => Type::Bytes,
            Value::Array(values) => Type::Array(Box::new(
                values.first().map(Value::ty).unwrap_or(Type::Unknown),
            )),
            Value::Tuple(values) => Type::Tuple(values.iter().map(Value::ty).collect()),
            Value::Set(values) => Type::Set(Box::new(
                values.first().map(Value::ty).unwrap_or(Type::Unknown),
            )),
            Value::Map(entries) => Type::Map(
                Box::new(
                    entries
                        .first()
                        .map(|(key, _)| key.ty())
                        .unwrap_or(Type::Unknown),
                ),
                Box::new(
                    entries
                        .first()
                        .map(|(_, value)| value.ty())
                        .unwrap_or(Type::Unknown),
                ),
            ),
            Value::Object {
                type_name,
                type_args,
                ..
            } => {
                if type_args.is_empty() {
                    Type::Named(type_name.clone())
                } else {
                    Type::Applied {
                        name: type_name.clone(),
                        args: type_args.clone(),
                    }
                }
            }
            Value::ClassRef {
                type_name,
                type_args,
                ..
            } => {
                if type_args.is_empty() {
                    Type::Named(type_name.clone())
                } else {
                    Type::Applied {
                        name: type_name.clone(),
                        args: type_args.clone(),
                    }
                }
            }
            Value::Option(Some(value)) => Type::Option(Box::new(value.ty())),
            Value::Option(None) => Type::Option(Box::new(Type::Unknown)),
            Value::Result(Ok(value)) => Type::Result(Box::new(value.ty()), Box::new(Type::Unknown)),
            Value::Result(Err(value)) => {
                Type::Result(Box::new(Type::Unknown), Box::new(value.ty()))
            }
            Value::Rows { result_type, .. } => result_type.as_ref().clone(),
            Value::Unit => Type::Unit,
        }
    }
    pub fn output(&self) -> String {
        match self {
            Value::Int(v) => v.to_string(),
            Value::UInt(v) => v.to_string(),
            Value::Float(v) => v.to_string(),
            Value::Decimal(v) => format_decimal(*v),
            Value::Bool(v) => v.to_string(),
            Value::String(v) => v.clone(),
            Value::Char(v) => v.to_string(),
            Value::Enum {
                type_name,
                type_args,
                variant,
                payloads,
            } => {
                let type_name = if type_args.is_empty() {
                    type_name.clone()
                } else {
                    format!(
                        "{type_name}<{}>",
                        type_args
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                if payloads.is_empty() {
                    format!("{type_name}::{variant}")
                } else {
                    format!(
                        "{type_name}::{variant}({})",
                        payloads
                            .iter()
                            .map(Value::output)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            Value::Timestamp(v) => v.to_string(),
            Value::Date(v) | Value::Time(v) => v.clone(),
            Value::Duration(v) => format!("{v}ms"),
            Value::Bytes(values) => format!(
                "Bytes[{}]",
                values
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::ClassRef {
                reference,
                type_name,
                type_args,
            } => format!(
                "<{}#{}>",
                if type_args.is_empty() {
                    type_name.clone()
                } else {
                    format!(
                        "{type_name}<{}>",
                        type_args
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                },
                reference.object_id
            ),
            Value::Array(values) => format!(
                "[{}]",
                values
                    .iter()
                    .map(Value::output)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Tuple(values) => format!(
                "({})",
                values
                    .iter()
                    .map(Value::output)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Set(values) => format!(
                "Set{{{}}}",
                values
                    .iter()
                    .map(Value::output)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Map(entries) => format!(
                "Map{{{}}}",
                entries
                    .iter()
                    .map(|(key, value)| format!("{}={}", key.output(), value.output()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Function(function) => format!("<function {}>", function.name),
            Value::Object {
                type_name,
                type_args,
                fields,
            } => format!(
                "{}{{{}}}",
                if type_args.is_empty() {
                    type_name.clone()
                } else {
                    format!(
                        "{type_name}<{}>",
                        type_args
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                },
                fields
                    .iter()
                    .map(|(name, value)| format!("{name}={}", value.output()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Option(Some(v)) => format!("Some({})", v.output()),
            Value::Option(None) => "None".into(),
            Value::Result(Ok(v)) => format!("Ok({})", v.output()),
            Value::Result(Err(v)) => format!("Err({})", v.output()),
            Value::Rows { columns, rows, .. } => {
                let rendered_rows = rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .enumerate()
                            .map(|(index, value)| {
                                let column = columns.get(index).map(String::as_str).unwrap_or("?");
                                format!("{column}={value}")
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("[{rendered_rows}]")
            }
            Value::Unit => "()".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeError {
    pub message: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileSystemPolicy {
    pub base_dir: PathBuf,
    pub read_roots: Vec<PathBuf>,
    pub write_roots: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkPolicy {
    pub allowed_hosts: Vec<String>,
    pub timeout_ms: u64,
    pub max_response_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessPolicy {
    pub allowed_commands: Vec<String>,
    pub timeout_ms: u64,
    pub max_output_bytes: usize,
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct MailPolicy {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub security: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub ca_certificate_pem: Option<Vec<u8>>,
    pub from: Option<String>,
}

impl fmt::Debug for MailPolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MailPolicy")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("security", &self.security)
            .field("username", &self.username.as_ref().map(|_| "<redacted>"))
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .field(
                "ca_certificate_pem",
                &self.ca_certificate_pem.as_ref().map(|_| "<configured>"),
            )
            .field("from", &self.from)
            .finish()
    }
}

impl Drop for MailPolicy {
    fn drop(&mut self) {
        if let Some(password) = &mut self.password {
            zeroize::Zeroize::zeroize(password);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RuntimePolicy {
    pub filesystem: Option<FileSystemPolicy>,
    pub network: Option<NetworkPolicy>,
    pub process: Option<ProcessPolicy>,
    pub mail: Option<MailPolicy>,
    /// Optional statement and expression evaluation budget for CLI, sandbox,
    /// and untrusted-code runs.
    /// `None` leaves long-running server handlers unrestricted.
    pub step_budget: Option<usize>,
    /// Maximum language function-call depth. Project settings are capped at 500.
    pub max_stack_depth: Option<usize>,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Clone)]
struct RuntimeVariable {
    value: Value,
    mutable: bool,
}

#[derive(Clone)]
struct Environment {
    scopes: Vec<HashMap<String, RuntimeVariable>>,
}

impl Environment {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }
    fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }
    fn pop(&mut self) {
        self.scopes.pop();
    }
    fn declare(&mut self, name: String, value: Value, mutable: bool) {
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name, RuntimeVariable { value, mutable });
    }
    fn contains(&self, name: &str) -> bool {
        self.scopes
            .iter()
            .rev()
            .any(|scope| scope.contains_key(name))
    }
    fn is_mutable(&self, name: &str) -> Option<bool> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).map(|variable| variable.mutable))
    }
    fn visible_values(&self) -> Vec<(String, Value)> {
        let mut values = std::collections::BTreeMap::new();
        for scope in &self.scopes {
            for (name, variable) in scope {
                values.insert(name.clone(), variable.value.clone());
            }
        }
        values.into_iter().collect()
    }
    fn get(&self, name: &str) -> Option<Value> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).map(|v| v.value.clone()))
    }
    fn assign(&mut self, name: &str, value: Value) -> Result<(), &'static str> {
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
}

enum Flow {
    Continue,
    LoopContinue,
    Return(Value),
    Break,
}

pub fn execute(program: &Program) -> Result<Vec<String>, RuntimeError> {
    execute_internal(program, None, None, None)
}

pub fn execute_with_database(
    program: &Program,
    database_url: &str,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal(program, Some(database_url.to_owned()), None, None)
}

pub fn execute_with_capabilities(
    program: &Program,
    grants: Option<&HashSet<String>>,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal(program, None, grants.cloned(), None)
}

pub fn execute_with_database_and_capabilities(
    program: &Program,
    database_url: &str,
    grants: Option<&HashSet<String>>,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal(
        program,
        Some(database_url.to_owned()),
        grants.cloned(),
        None,
    )
}

pub fn execute_with_capabilities_and_filesystem_policy(
    program: &Program,
    grants: Option<&HashSet<String>>,
    filesystem_policy: Option<&FileSystemPolicy>,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal(
        program,
        None,
        grants.cloned(),
        Some(RuntimePolicy {
            filesystem: filesystem_policy.cloned(),
            network: None,
            process: None,
            mail: None,
            step_budget: None,
            max_stack_depth: None,
        }),
    )
}

pub fn execute_with_capabilities_and_policies(
    program: &Program,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal(program, None, grants.cloned(), policy.cloned())
}

pub fn execute_with_database_and_capabilities_and_filesystem_policy(
    program: &Program,
    database_url: &str,
    grants: Option<&HashSet<String>>,
    filesystem_policy: Option<&FileSystemPolicy>,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal(
        program,
        Some(database_url.to_owned()),
        grants.cloned(),
        Some(RuntimePolicy {
            filesystem: filesystem_policy.cloned(),
            network: None,
            process: None,
            mail: None,
            step_budget: None,
            max_stack_depth: None,
        }),
    )
}

pub fn execute_with_database_and_capabilities_and_policies(
    program: &Program,
    database_url: &str,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
) -> Result<Vec<String>, RuntimeError> {
    execute_internal(
        program,
        Some(database_url.to_owned()),
        grants.cloned(),
        policy.cloned(),
    )
}

pub fn execute_function(
    program: &Program,
    name: &str,
    args: Vec<Value>,
    database_url: Option<&str>,
) -> Result<Value, RuntimeError> {
    execute_function_with_capabilities_and_filesystem_policy(
        program,
        name,
        args,
        database_url,
        None,
        None,
    )
}

pub fn execute_function_with_capabilities_and_policies(
    program: &Program,
    name: &str,
    args: Vec<Value>,
    database_url: Option<&str>,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
) -> Result<Value, RuntimeError> {
    let depth = policy
        .and_then(|policy| policy.max_stack_depth)
        .unwrap_or(MAX_FUNCTION_CALL_DEPTH);
    run_on_runtime_stack(runtime_stack_size(depth), || {
        execute_function_on_runtime_stack(program, name, args, database_url, grants, policy)
    })
}

fn execute_function_on_runtime_stack(
    program: &Program,
    name: &str,
    args: Vec<Value>,
    database_url: Option<&str>,
    grants: Option<&HashSet<String>>,
    policy: Option<&RuntimePolicy>,
) -> Result<Value, RuntimeError> {
    let mut interpreter = Interpreter {
        functions: program
            .functions
            .iter()
            .map(|function| (function.name.clone(), function))
            .collect(),
        view_definitions: program
            .views
            .iter()
            .map(|view| (view.name.clone(), view.clone()))
            .collect(),
        output: Vec::new(),
        next_class_object_id: Arc::new(AtomicU64::new(1)),
        parallel_worker: false,
        steps: Arc::new(AtomicUsize::new(0)),
        step_budget: policy.as_ref().and_then(|policy| policy.step_budget),
        call_depth_limit: policy
            .as_ref()
            .and_then(|policy| policy.max_stack_depth)
            .unwrap_or(MAX_FUNCTION_CALL_DEPTH),
        call_depth: 0,
        database_url: database_url.map(str::to_owned),
        storage_schema: zelyra_database::build_schema(program),
        storage_checked: false,
        granted_capabilities: grants.cloned(),
        filesystem_policy: policy.and_then(|policy| policy.filesystem.clone()),
        network_policy: policy.and_then(|policy| policy.network.clone()),
        process_policy: policy.and_then(|policy| policy.process.clone()),
        mail_policy: policy.and_then(|policy| policy.mail.clone()),
        sql_record_definitions: sql_results::definitions(program),
        record_definitions: program
            .records
            .iter()
            .map(|record| (record.name.clone(), record.clone()))
            .collect(),
        type_aliases: program
            .types
            .iter()
            .map(|definition| (definition.name.clone(), definition.target.clone()))
            .collect(),
        enum_definitions: program
            .enums
            .iter()
            .map(|definition| (definition.name.clone(), definition.clone()))
            .collect(),
        active_capabilities: Vec::new(),
        active_type_substitutions: Vec::new(),
    };
    interpreter.call(name, args, Span::default())
}

pub fn execute_function_with_capabilities(
    program: &Program,
    name: &str,
    args: Vec<Value>,
    database_url: Option<&str>,
    grants: Option<&HashSet<String>>,
) -> Result<Value, RuntimeError> {
    execute_function_with_capabilities_and_filesystem_policy(
        program,
        name,
        args,
        database_url,
        grants,
        None,
    )
}

pub fn execute_function_with_capabilities_and_filesystem_policy(
    program: &Program,
    name: &str,
    args: Vec<Value>,
    database_url: Option<&str>,
    grants: Option<&HashSet<String>>,
    filesystem_policy: Option<&FileSystemPolicy>,
) -> Result<Value, RuntimeError> {
    execute_function_with_capabilities_and_policies(
        program,
        name,
        args,
        database_url,
        grants,
        Some(&RuntimePolicy {
            filesystem: filesystem_policy.cloned(),
            network: None,
            process: None,
            mail: None,
            step_budget: None,
            max_stack_depth: None,
        }),
    )
}

fn execute_internal(
    program: &Program,
    database_url: Option<String>,
    granted_capabilities: Option<HashSet<String>>,
    policy: Option<RuntimePolicy>,
) -> Result<Vec<String>, RuntimeError> {
    let depth = policy
        .as_ref()
        .and_then(|policy| policy.max_stack_depth)
        .unwrap_or(MAX_FUNCTION_CALL_DEPTH);
    run_on_runtime_stack(runtime_stack_size(depth), || {
        execute_internal_on_runtime_stack(program, database_url, granted_capabilities, policy)
    })
}

fn execute_internal_on_runtime_stack(
    program: &Program,
    database_url: Option<String>,
    granted_capabilities: Option<HashSet<String>>,
    policy: Option<RuntimePolicy>,
) -> Result<Vec<String>, RuntimeError> {
    let mut interpreter = Interpreter {
        functions: program
            .functions
            .iter()
            .map(|f| (f.name.clone(), f))
            .collect(),
        view_definitions: program
            .views
            .iter()
            .map(|view| (view.name.clone(), view.clone()))
            .collect(),
        output: Vec::new(),
        next_class_object_id: Arc::new(AtomicU64::new(1)),
        parallel_worker: false,
        steps: Arc::new(AtomicUsize::new(0)),
        step_budget: policy.as_ref().and_then(|policy| policy.step_budget),
        call_depth_limit: policy
            .as_ref()
            .and_then(|policy| policy.max_stack_depth)
            .unwrap_or(MAX_FUNCTION_CALL_DEPTH),
        call_depth: 0,
        database_url,
        storage_schema: zelyra_database::build_schema(program),
        storage_checked: false,
        granted_capabilities,
        filesystem_policy: policy.as_ref().and_then(|policy| policy.filesystem.clone()),
        network_policy: policy.as_ref().and_then(|policy| policy.network.clone()),
        process_policy: policy.as_ref().and_then(|policy| policy.process.clone()),
        mail_policy: policy.as_ref().and_then(|policy| policy.mail.clone()),
        sql_record_definitions: sql_results::definitions(program),
        record_definitions: program
            .records
            .iter()
            .map(|record| (record.name.clone(), record.clone()))
            .collect(),
        type_aliases: program
            .types
            .iter()
            .map(|definition| (definition.name.clone(), definition.target.clone()))
            .collect(),
        enum_definitions: program
            .enums
            .iter()
            .map(|definition| (definition.name.clone(), definition.clone()))
            .collect(),
        active_capabilities: Vec::new(),
        active_type_substitutions: Vec::new(),
    };
    interpreter.call("main", Vec::new(), Span::default())?;
    Ok(interpreter.output)
}

const MAX_FUNCTION_CALL_DEPTH: usize = 20;
const MAX_CONFIGURED_FUNCTION_CALL_DEPTH: usize = 500;

fn runtime_stack_size(depth: usize) -> usize {
    2 * 1024 * 1024 + depth.min(MAX_CONFIGURED_FUNCTION_CALL_DEPTH) * 192 * 1024
}

fn run_on_runtime_stack<T: Send>(
    stack_size: usize,
    action: impl FnOnce() -> Result<T, RuntimeError> + Send,
) -> Result<T, RuntimeError> {
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .name("zelyra-runtime".into())
            .stack_size(stack_size)
            .spawn_scoped(scope, action)
            .map_err(|_| RuntimeError {
                message: "runtime execution thread could not be started".into(),
                span: Span::default(),
            })?;
        handle.join().unwrap_or_else(|_| {
            Err(RuntimeError {
                message: "runtime execution thread panicked".into(),
                span: Span::default(),
            })
        })
    })
}

struct Interpreter<'program> {
    functions: HashMap<String, &'program Function>,
    view_definitions: HashMap<String, ViewDef>,
    output: Vec<String>,
    next_class_object_id: Arc<AtomicU64>,
    parallel_worker: bool,
    steps: Arc<AtomicUsize>,
    step_budget: Option<usize>,
    call_depth_limit: usize,
    call_depth: usize,
    database_url: Option<String>,
    storage_schema: Result<zelyra_database::Schema, Vec<zelyra_database::SchemaError>>,
    storage_checked: bool,
    granted_capabilities: Option<HashSet<String>>,
    filesystem_policy: Option<FileSystemPolicy>,
    network_policy: Option<NetworkPolicy>,
    process_policy: Option<ProcessPolicy>,
    mail_policy: Option<MailPolicy>,
    record_definitions: HashMap<String, RecordDef>,
    sql_record_definitions: HashMap<String, RecordDef>,
    type_aliases: HashMap<String, Type>,
    enum_definitions: HashMap<String, zelyra_ast::EnumDef>,
    active_capabilities: Vec<HashSet<String>>,
    active_type_substitutions: Vec<HashMap<String, Type>>,
}

fn integer_range(start: i64, end: i64, step: i64) -> impl Iterator<Item = i64> {
    let ascending = step > 0;
    let first = if (ascending && start < end) || (!ascending && start > end) {
        Some(start)
    } else {
        None
    };
    std::iter::successors(first, move |current| {
        let next = current.checked_add(step)?;
        let before_end = if ascending { next < end } else { next > end };
        before_end.then_some(next)
    })
}

struct RuntimeClassObject {
    fields: HashMap<String, Value>,
}

#[derive(Clone)]
pub struct ClassObjectRef {
    object_id: u64,
    object: Arc<Mutex<RuntimeClassObject>>,
}

impl fmt::Debug for ClassObjectRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ClassObjectRef({})", self.object_id)
    }
}

impl PartialEq for ClassObjectRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.object, &other.object)
    }
}

fn class_reference_would_cycle(value: &Value, target: &ClassObjectRef) -> Result<bool, String> {
    let mut pending = vec![value.clone()];
    let mut visited = HashSet::new();
    while let Some(value) = pending.pop() {
        match value {
            Value::ClassRef { reference, .. } => {
                if Arc::ptr_eq(&reference.object, &target.object) {
                    return Ok(true);
                }
                let address = Arc::as_ptr(&reference.object) as usize;
                if visited.insert(address) {
                    let object = reference
                        .object
                        .lock()
                        .map_err(|_| "class object is unavailable".to_owned())?;
                    pending.extend(object.fields.values().cloned());
                }
            }
            Value::Array(values) | Value::Tuple(values) | Value::Set(values) => {
                pending.extend(values);
            }
            Value::Map(entries) => {
                pending.extend(entries.into_iter().flat_map(|(key, value)| [key, value]));
            }
            Value::Object { fields, .. } => pending.extend(fields.into_values()),
            Value::Enum { payloads, .. } => pending.extend(payloads),
            Value::Option(Some(value)) => pending.push(*value),
            Value::Result(Ok(value)) | Value::Result(Err(value)) => pending.push(*value),
            Value::Int(_)
            | Value::UInt(_)
            | Value::Float(_)
            | Value::Decimal(_)
            | Value::Bool(_)
            | Value::String(_)
            | Value::Char(_)
            | Value::Timestamp(_)
            | Value::Date(_)
            | Value::Time(_)
            | Value::Duration(_)
            | Value::Bytes(_)
            | Value::Function(_)
            | Value::Option(None)
            | Value::Rows { .. }
            | Value::Unit => {}
        }
    }
    Ok(false)
}

fn statement_span(statement: &Stmt) -> Span {
    match statement {
        Stmt::Let { span, .. }
        | Stmt::BindOrAssign { span, .. }
        | Stmt::TupleBindOrAssign { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::If { span, .. }
        | Stmt::While { span, .. }
        | Stmt::For { span, .. }
        | Stmt::Loop { span, .. }
        | Stmt::Break { span }
        | Stmt::Continue { span }
        | Stmt::Match { span, .. }
        | Stmt::Transaction { span, .. }
        | Stmt::Parallel { span, .. } => *span,
        Stmt::Expr(expression) => expression.span,
    }
}

fn read_console_line(prompt: &str) -> io::Result<Option<String>> {
    static CONSOLE_INPUT_LOCK: Mutex<()> = Mutex::new(());
    let _guard = CONSOLE_INPUT_LOCK
        .lock()
        .map_err(|_| io::Error::other("console input lock is unavailable"))?;

    let mut stdout = io::stdout().lock();
    stdout.write_all(prompt.as_bytes())?;
    stdout.flush()?;

    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input)? == 0 {
        return Ok(None);
    }
    if input.ends_with('\n') {
        input.pop();
        if input.ends_with('\r') {
            input.pop();
        }
    }
    Ok(Some(input))
}

fn is_runtime_builtin(name: &str) -> bool {
    matches!(
        name,
        "print"
            | "assert"
            | "render_pdf"
            | "len"
            | "range"
            | "array_replace_result"
            | "object_set_field"
            | "record_with"
            | "record_update"
            | "map"
            | "filter"
            | "find"
            | "reduce"
            | "append"
            | "tuple"
            | "set"
            | "set_add"
            | "set_remove"
            | "contains"
            | "get"
            | "put"
            | "keys"
            | "values"
            | "first"
            | "last"
            | "Some"
            | "Ok"
            | "Err"
            | "now"
            | "date"
            | "decimal"
            | "time"
            | "duration_ms"
            | "bytes"
            | "format_timestamp"
            | "format_text"
            | "env"
            | "random_int"
            | "read_console"
            | "http_get"
            | "http_request"
            | "mail_queue"
            | "mail_deliver"
            | "json_encode"
            | "json_encode_tagged_result"
            | "json_decode"
            | "json_decode_result"
            | "sql_decode"
            | "json_decode_tagged_result"
            | "csv_decode_result"
            | "csv_encode"
            | "text_contains"
            | "text_trim"
            | "text_chars"
            | "abs"
            | "min"
            | "max"
            | "clamp"
            | "to_string"
            | "parse_int"
            | "parse_float"
            | "sort"
            | "sort_desc"
            | "reverse"
            | "slice"
            | "concat"
            | "join"
            | "text_to_lower"
            | "text_to_upper"
            | "text_starts_with"
            | "text_ends_with"
            | "text_replace"
            | "text_split"
            | "split"
            | "text_pad_left"
            | "text_pad_right"
            | "get_or"
            | "get_or_default"
            | "map_increment"
            | "has_key"
            | "unwrap_or"
            | "sqrt"
            | "pow"
            | "floor"
            | "ceil"
            | "round"
            | "text_index_of"
            | "text_repeat"
            | "text_is_empty"
            | "text_capitalize"
            | "unique"
            | "array_sum"
            | "avg"
            | "min_of"
            | "max_of"
            | "chunk"
            | "flatten"
            | "set_union"
            | "set_intersection"
            | "set_difference"
            | "map_remove"
            | "map_merge"
            | "date_add_days"
            | "date_diff_days"
            | "is_ok"
            | "is_err"
            | "is_some"
            | "is_none"
            | "is_before"
            | "is_after"
            | "base64_encode"
            | "base64_decode"
            | "hash_sha256"
            | "uuid_v4"
            | "url_encode"
            | "url_decode"
            | "text_trim_start"
            | "text_trim_end"
            | "text_substring"
            | "text_is_blank"
            | "zip"
            | "entries"
            | "dedup"
            | "to_int"
            | "to_float"
            | "parse_bool"
            | "panic"
            | "path_join"
            | "path_filename"
            | "path_extension"
            | "path_dir"
            | "file_exists"
            | "dir_exists"
            | "file_size"
            | "text_replace_result"
            | "text_split_result"
            | "decimal_scaled_result"
            | "format_scaled_result"
            | "date_add_days_result"
            | "date_days_between_result"
            | "timestamp_add_result"
            | "timestamp_difference_result"
            | "decimal_quantize_result"
            | "http_json"
            | "http_result"
            | "run_process"
            | "read_text"
            | "read_text_result"
            | "write_text"
            | "write_text_result"
            | "write_text_new_result"
            | "atomic_write_text_result"
            | "append_text_result"
            | "copy_file_new_result"
            | "rename_file_result"
            | "delete_file_result"
            | "delete_file"
            | "list_dir"
    )
}

impl<'program> Interpreter<'program> {
    fn runtime_alias_type(&self, ty: &Type, span: Span) -> Result<Type, RuntimeError> {
        let mut aliases = HashMap::from([
            ("Id".into(), Type::Int),
            ("Money".into(), Type::Decimal),
            ("Email".into(), Type::String),
            ("Url".into(), Type::String),
            ("Uuid".into(), Type::String),
        ]);
        aliases.retain(|name, _| {
            !self.record_definitions.contains_key(name) && !self.enum_definitions.contains_key(name)
        });
        aliases.extend(self.type_aliases.clone());
        expand_type_aliases(ty, &aliases).map_err(|message| self.runtime_error(span, message))
    }

    fn ensure_decimal_storage(&mut self, span: Span) -> Result<(), RuntimeError> {
        if !self.storage_checked {
            if let Some(url) = &self.database_url {
                let schema = self.storage_schema.as_ref().map_err(|errors| {
                    self.runtime_error(
                        span,
                        errors
                            .first()
                            .map_or("invalid database schema", |error| error.message.as_str()),
                    )
                })?;
                zelyra_database::validate_decimal_storage(url, schema)
                    .map_err(|error| self.runtime_error(span, error.to_string()))?;
                zelyra_database::validate_temporal_storage(url, schema)
                    .map_err(|error| self.runtime_error(span, error.to_string()))?;
            }
            self.storage_checked = true;
        }
        Ok(())
    }

    fn consume_step(&mut self, span: Span) -> Result<(), RuntimeError> {
        let mut previous = self.steps.load(Ordering::Relaxed);
        let steps = loop {
            let next = previous.saturating_add(1);
            match self.steps.compare_exchange_weak(
                previous,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break next,
                Err(actual) => previous = actual,
            }
        };
        if self.step_budget.is_some_and(|budget| steps > budget) {
            return Err(self.runtime_error(
                span,
                format!(
                    "execution step budget exceeded (limit {})",
                    self.step_budget.unwrap()
                ),
            ));
        }
        Ok(())
    }
    fn runtime_error(&self, span: Span, message: impl Into<String>) -> RuntimeError {
        RuntimeError {
            message: message.into(),
            span,
        }
    }
    fn require_runtime_capability(
        &self,
        capability: &str,
        operation: &str,
        span: Span,
    ) -> Result<(), RuntimeError> {
        let declared = self.active_capabilities.last().is_some_and(|capabilities| {
            capabilities.contains(capability)
                || (capability.starts_with("Database(") && capabilities.contains("Database"))
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

    fn filesystem_roots(&self, write: bool) -> Vec<PathBuf> {
        self.filesystem_policy
            .as_ref()
            .map_or_else(Vec::new, |policy| {
                if write {
                    policy.write_roots.clone()
                } else {
                    policy.read_roots.clone()
                }
            })
    }

    fn resolve_filesystem_path(
        &self,
        path: &str,
        roots: &[PathBuf],
        operation: &str,
        span: Span,
        allow_missing: bool,
    ) -> Result<PathBuf, RuntimeError> {
        let Some(policy) = &self.filesystem_policy else {
            return Ok(PathBuf::from(path));
        };
        let candidate = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            policy.base_dir.join(path)
        };
        let resolved = if allow_missing && !candidate.exists() && !candidate.is_symlink() {
            let Some(file_name) = candidate.file_name() else {
                return Err(self.runtime_error(
                    span,
                    format!("file-system {operation} requires a file path"),
                ));
            };
            let parent = candidate.parent().unwrap_or(Path::new("."));
            let parent = std::fs::canonicalize(parent).map_err(|error| {
                self.runtime_error(
                    span,
                    format!("file-system {operation} failed for {path}: {error}"),
                )
            })?;
            parent.join(file_name)
        } else {
            std::fs::canonicalize(&candidate).map_err(|error| {
                self.runtime_error(
                    span,
                    format!("file-system {operation} failed for {path}: {error}"),
                )
            })?
        };
        if roots
            .iter()
            .any(|root| resolved == *root || resolved.starts_with(root))
        {
            Ok(resolved)
        } else {
            Err(self.runtime_error(
                span,
                format!("file-system {operation} denied for {path}: path is outside the configured roots"),
            ))
        }
    }

    fn http_request(
        &self,
        method: &str,
        url: &str,
        request_headers: &[String],
        request_body: Option<&str>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if !matches!(method, "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD") {
            return Err(self.runtime_error(
                span,
                format!("http_request does not support method `{method}`"),
            ));
        }
        if matches!(method, "GET" | "HEAD") && request_body.is_some() {
            return Err(self.runtime_error(
                span,
                format!("http_request does not allow a body with {method}"),
            ));
        }
        let Some((scheme, authority_and_path)) = url.split_once("://") else {
            return Err(
                self.runtime_error(span, "http_request supports only http:// and https:// URLs")
            );
        };
        if !matches!(scheme, "http" | "https") {
            return Err(
                self.runtime_error(span, "http_request supports only http:// and https:// URLs")
            );
        }
        let authority_end = authority_and_path
            .find(['/', '?', '#'])
            .unwrap_or(authority_and_path.len());
        let authority = &authority_and_path[..authority_end];
        if authority.is_empty() || authority.contains('@') || authority.contains(['\r', '\n']) {
            return Err(self.runtime_error(span, "http_request received an invalid URL"));
        }
        let host = if let Some((host, port)) = authority.rsplit_once(':') {
            if port.parse::<u16>().is_err() {
                return Err(self.runtime_error(span, "http_request received an invalid port"));
            }
            host
        } else {
            authority
        };
        if host.is_empty() || host.contains(['/', '?', '#', '[', ']', '\\']) {
            return Err(self.runtime_error(span, "http_request received an invalid host"));
        }
        if let Some(policy) = &self.network_policy {
            let authority_allowed = policy
                .allowed_hosts
                .iter()
                .any(|allowed| allowed == authority || allowed == host);
            if !authority_allowed {
                return Err(self.runtime_error(
                    span,
                    format!("network access denied for host `{authority}`"),
                ));
            }
        }
        let policy = self.network_policy.as_ref();
        let timeout = Duration::from_millis(policy.map_or(5_000, |policy| policy.timeout_ms));
        let maximum = policy.map_or(1_048_576, |policy| policy.max_response_bytes);
        if maximum == 0 {
            return Err(self.runtime_error(
                span,
                "http_request requires a positive maximum response size",
            ));
        }
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .into();
        let mut builder = ureq::http::Request::builder().method(method).uri(url);
        for header in request_headers {
            let Some((name, value)) = header.split_once(':') else {
                return Err(self.runtime_error(
                    span,
                    "http_request headers must use the `Name: value` format",
                ));
            };
            let name = name.trim();
            let value = value.trim();
            if name.is_empty() || value.contains(['\r', '\n']) {
                return Err(self.runtime_error(span, "http_request received an invalid header"));
            }
            builder = builder.header(name, value);
        }
        let mut response = if let Some(body) = request_body {
            let request = builder.body(body.to_owned()).map_err(|error| {
                self.runtime_error(
                    span,
                    format!("http_request could not build request: {error}"),
                )
            })?;
            agent.run(request)
        } else {
            let request = builder.body(()).map_err(|error| {
                self.runtime_error(
                    span,
                    format!("http_request could not build request: {error}"),
                )
            })?;
            agent.run(request)
        }
        .map_err(|error| self.runtime_error(span, format!("network request failed: {error}")))?;
        let status = response.status().as_u16();
        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<usize>().ok());
        if content_length.is_some_and(|content_length| content_length > maximum) {
            return Err(self.runtime_error(
                span,
                format!("network response exceeds the {maximum} byte limit"),
            ));
        }
        let body = response
            .body_mut()
            .with_config()
            .limit(u64::try_from(maximum).unwrap_or(u64::MAX).saturating_add(1))
            .read_to_vec()
            .map_err(|error| {
                self.runtime_error(span, format!("network response failed: {error}"))
            })?;
        if body.len() > maximum {
            return Err(self.runtime_error(
                span,
                format!("network response exceeds the {maximum} byte limit"),
            ));
        }
        let body = String::from_utf8(body)
            .map_err(|_| self.runtime_error(span, "network response is not valid UTF-8"))?;
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                format!(
                    "{}: {}",
                    name,
                    value.to_str().unwrap_or("<non-utf8-header>")
                )
            })
            .map(Value::String)
            .collect();
        Ok(Value::Object {
            type_name: "HttpResponse".into(),
            type_args: Vec::new(),
            fields: HashMap::from([
                ("status".into(), Value::Int(i64::from(status))),
                ("headers".into(), Value::Array(headers)),
                ("body".into(), Value::String(body)),
            ]),
        })
    }

    fn http_json(
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

    fn http_result(
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

    fn http_get(&self, url: &str, span: Span) -> Result<String, RuntimeError> {
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

    fn json_encode(&self, value: &Value, span: Span) -> Result<String, RuntimeError> {
        serde_json::to_string(&self.json_value(value, span)?).map_err(|error| {
            self.runtime_error(
                span,
                format!("json_encode failed to serialize value: {error}"),
            )
        })
    }

    fn json_encode_tagged_result(&self, value: &Value, span: Span) -> Result<String, RuntimeError> {
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

    fn json_decode_tagged_result(
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

    fn json_value(&self, value: &Value, span: Span) -> Result<serde_json::Value, RuntimeError> {
        match value {
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

    fn json_decode(&self, source: &str, target: &Type, span: Span) -> Result<Value, RuntimeError> {
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
            if let Some(alias) = self.type_aliases.get(name) {
                return self.json_decode_value(value, alias, span);
            }
            if let Some(definition) = self._enum_definition(name) {
                return self.json_decode_enum(value, definition, &[], span);
            }
            if let Some(record) = self.record_definitions.get(name) {
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
            if let Some(record) = self.record_definitions.get(name) {
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
        self.enum_definitions.get(name)
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

    fn run_process(
        &self,
        command: &str,
        arguments: &[String],
        span: Span,
    ) -> Result<String, RuntimeError> {
        let Some(policy) = &self.process_policy else {
            return Err(self.runtime_error(
                span,
                "process execution denied: no process policy is configured",
            ));
        };
        if command.is_empty() || command.contains(['\r', '\n']) {
            return Err(self.runtime_error(span, "run_process received an invalid command"));
        }
        if !policy
            .allowed_commands
            .iter()
            .any(|allowed| allowed == command)
        {
            return Err(self.runtime_error(
                span,
                format!("process execution denied for command `{command}`"),
            ));
        }
        if policy.timeout_ms == 0 || policy.max_output_bytes == 0 {
            return Err(self.runtime_error(
                span,
                "process policy requires positive timeout and output limits",
            ));
        }
        let mut child = Command::new(command)
            .args(arguments)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                self.runtime_error(
                    span,
                    format!("process start failed for `{command}`: {error}"),
                )
            })?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| self.runtime_error(span, "process stdout pipe was unavailable"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| self.runtime_error(span, "process stderr pipe was unavailable"))?;
        let output_limit = u64::try_from(policy.max_output_bytes)
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        let stdout_reader = std::thread::spawn(move || {
            let mut output = Vec::new();
            stdout
                .take(output_limit)
                .read_to_end(&mut output)
                .map(|_| output)
        });
        let stderr_reader = std::thread::spawn(move || {
            let mut output = Vec::new();
            stderr
                .take(output_limit)
                .read_to_end(&mut output)
                .map(|_| output)
        });
        let timeout = Duration::from_millis(policy.timeout_ms);
        let deadline = Instant::now() + timeout;
        let mut timed_out = false;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() >= deadline => {
                    timed_out = true;
                    let _ = child.kill();
                    break child.wait().map_err(|error| {
                        self.runtime_error(span, format!("process termination failed: {error}"))
                    })?;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(5)),
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(self.runtime_error(span, format!("process status failed: {error}")));
                }
            }
        };
        let stdout = stdout_reader
            .join()
            .map_err(|_| self.runtime_error(span, "process stdout reader terminated unexpectedly"))?
            .map_err(|error| {
                self.runtime_error(span, format!("process stdout read failed: {error}"))
            })?;
        let stderr = stderr_reader
            .join()
            .map_err(|_| self.runtime_error(span, "process stderr reader terminated unexpectedly"))?
            .map_err(|error| {
                self.runtime_error(span, format!("process stderr read failed: {error}"))
            })?;
        if timed_out {
            return Err(self.runtime_error(
                span,
                format!(
                    "process `{command}` exceeded the {} ms timeout",
                    policy.timeout_ms
                ),
            ));
        }
        if stdout.len() > policy.max_output_bytes || stderr.len() > policy.max_output_bytes {
            return Err(self.runtime_error(
                span,
                format!(
                    "process `{command}` exceeded the {} byte output limit",
                    policy.max_output_bytes
                ),
            ));
        }
        if !status.success() {
            let error_output = String::from_utf8_lossy(&stderr);
            return Err(self.runtime_error(
                span,
                format!(
                    "process `{command}` exited unsuccessfully ({}): {}",
                    status,
                    error_output.trim()
                ),
            ));
        }
        String::from_utf8(stdout)
            .map_err(|_| self.runtime_error(span, "process stdout is not valid UTF-8"))
    }

    fn call(&mut self, name: &str, args: Vec<Value>, span: Span) -> Result<Value, RuntimeError> {
        self.call_with_type_args(name, &[], args, span)
    }

    fn call_with_type_args(
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
    fn check_loop_invariants(
        &mut self,
        invariants: &[Expr],
        env: &mut Environment,
        span: Span,
        phase: &str,
    ) -> Result<(), RuntimeError> {
        for invariant in invariants {
            let value = self.eval(invariant, env)?;
            if !self.expect_bool(value, invariant.span)? {
                return Err(self.runtime_error(
                    span,
                    format!("loop invariant failed {phase} loop iteration"),
                ));
            }
        }
        Ok(())
    }
    fn exec_block(&mut self, block: &Block, env: &mut Environment) -> Result<Flow, RuntimeError> {
        env.push();
        for statement in &block.statements {
            match self.exec_statement(statement, env)? {
                Flow::Continue => {}
                flow => {
                    env.pop();
                    return Ok(flow);
                }
            }
        }
        env.pop();
        Ok(Flow::Continue)
    }
    fn exec_statement(
        &mut self,
        statement: &Stmt,
        env: &mut Environment,
    ) -> Result<Flow, RuntimeError> {
        self.consume_step(statement_span(statement))?;
        match statement {
            Stmt::Let {
                name,
                value,
                mutable,
                ..
            } => {
                let value = self.eval(value, env)?;
                env.declare(name.clone(), value, *mutable);
                Ok(Flow::Continue)
            }
            Stmt::BindOrAssign { name, value, span } => {
                let value = self.eval(value, env)?;
                if env.contains(name) {
                    env.assign(name, value).map_err(|reason| {
                        self.runtime_error(
                            *span,
                            if reason == "immutable" {
                                format!("cannot assign to immutable variable `{name}`")
                            } else {
                                format!("unknown variable `{name}`")
                            },
                        )
                    })?;
                } else {
                    env.declare(name.clone(), value, false);
                }
                Ok(Flow::Continue)
            }
            Stmt::TupleBindOrAssign { names, value, span } => {
                let value = self.eval(value, env)?;
                let Value::Tuple(values) = value else {
                    return Err(self.runtime_error(
                        *span,
                        format!("tuple assignment expects a tuple, found `{}`", value.ty()),
                    ));
                };
                if values.len() != names.len() {
                    return Err(self.runtime_error(
                        *span,
                        format!(
                            "tuple assignment has {} variables but the value has {} items",
                            names.len(),
                            values.len()
                        ),
                    ));
                }
                for name in names {
                    if env.is_mutable(name) == Some(false) {
                        return Err(self.runtime_error(
                            *span,
                            format!("cannot assign to immutable variable `{name}`"),
                        ));
                    }
                }
                for (name, value) in names.iter().zip(values) {
                    if env.contains(name) {
                        env.assign(name, value).map_err(|_| {
                            self.runtime_error(
                                *span,
                                format!("cannot assign to immutable variable `{name}`"),
                            )
                        })?;
                    } else {
                        env.declare(name.clone(), value, false);
                    }
                }
                Ok(Flow::Continue)
            }
            Stmt::Expr(expr) => {
                self.eval(expr, env)?;
                Ok(Flow::Continue)
            }
            Stmt::Return { value, .. } => Ok(Flow::Return(
                value
                    .as_ref()
                    .map(|expr| self.eval(expr, env))
                    .transpose()?
                    .unwrap_or(Value::Unit),
            )),
            Stmt::If {
                condition,
                then_block,
                else_block,
                span,
            } => {
                let condition_value = self.eval(condition, env)?;
                if self.expect_bool(condition_value, *span)? {
                    self.exec_block(then_block, env)
                } else if let Some(block) = else_block {
                    self.exec_block(block, env)
                } else {
                    Ok(Flow::Continue)
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                span,
            } => {
                loop {
                    self.consume_step(*span)?;
                    self.check_loop_invariants(invariants, env, *span, "before")?;
                    let condition_value = self.eval(condition, env)?;
                    if !self.expect_bool(condition_value, *span)? {
                        break;
                    }
                    let flow = self.exec_block(body, env)?;
                    self.check_loop_invariants(invariants, env, *span, "after")?;
                    match flow {
                        Flow::Continue => {}
                        Flow::LoopContinue => continue,
                        Flow::Break => break,
                        flow @ Flow::Return(_) => return Ok(flow),
                    }
                }
                Ok(Flow::Continue)
            }
            Stmt::For {
                name,
                additional_names,
                iterable,
                body,
                span,
            } => {
                let values: Box<dyn Iterator<Item = Value>> = match &iterable.kind {
                    ExprKind::Call {
                        name: call,
                        type_args,
                        args,
                    } if call == "range" => {
                        if !type_args.is_empty() || !(2..=3).contains(&args.len()) {
                            return Err(self.runtime_error(
                                *span,
                                "range expects two Int bounds and an optional Int step",
                            ));
                        }
                        let start = self.eval(&args[0], env)?;
                        let end = self.eval(&args[1], env)?;
                        let (Value::Int(start), Value::Int(end)) = (start, end) else {
                            return Err(self.runtime_error(
                                *span,
                                "range expects two Int bounds and an optional Int step",
                            ));
                        };
                        let step = match args.get(2) {
                            Some(expression) => match self.eval(expression, env)? {
                                Value::Int(step) => step,
                                _ => {
                                    return Err(self.runtime_error(
                                        expression.span,
                                        "range step must be an Int",
                                    ));
                                }
                            },
                            None => 1,
                        };
                        if step == 0 {
                            return Err(self.runtime_error(
                                args.get(2).map_or(*span, |step| step.span),
                                "range step must not be zero",
                            ));
                        }
                        Box::new(integer_range(start, end, step).map(Value::Int))
                    }
                    _ => {
                        let iterable_value = self.eval(iterable, env)?;
                        match iterable_value {
                            Value::Array(values) | Value::Set(values) => {
                                Box::new(values.into_iter())
                            }
                            value => {
                                return Err(self.runtime_error(
                                    *span,
                                    format!(
                                        "`for ... in` expects an array or Set, found {}",
                                        value.ty()
                                    ),
                                ));
                            }
                        }
                    }
                };
                for value in values {
                    self.consume_step(*span)?;
                    env.push();
                    if additional_names.is_empty() {
                        env.declare(name.clone(), value, false);
                    } else {
                        let Value::Tuple(values) = value else {
                            env.pop();
                            return Err(self.runtime_error(
                                *span,
                                "tuple loop pattern received a non-tuple value",
                            ));
                        };
                        if values.len() != additional_names.len() + 1 {
                            env.pop();
                            return Err(self.runtime_error(
                                *span,
                                format!(
                                    "tuple loop pattern expects {} values, found {}",
                                    additional_names.len() + 1,
                                    values.len()
                                ),
                            ));
                        }
                        let mut values = values.into_iter();
                        env.declare(
                            name.clone(),
                            values.next().expect("tuple arity checked"),
                            false,
                        );
                        for name in additional_names {
                            env.declare(
                                name.clone(),
                                values.next().expect("tuple arity checked"),
                                false,
                            );
                        }
                    }
                    let flow = self.exec_block(body, env)?;
                    env.pop();
                    match flow {
                        Flow::Continue | Flow::LoopContinue => {}
                        Flow::Break => break,
                        flow @ Flow::Return(_) => return Ok(flow),
                    }
                }
                Ok(Flow::Continue)
            }
            Stmt::Loop {
                invariants,
                body,
                span,
            } => {
                loop {
                    self.consume_step(*span)?;
                    self.check_loop_invariants(invariants, env, *span, "before")?;
                    let flow = self.exec_block(body, env)?;
                    self.check_loop_invariants(invariants, env, *span, "after")?;
                    match flow {
                        Flow::Continue => {}
                        Flow::LoopContinue => continue,
                        Flow::Break => break,
                        flow @ Flow::Return(_) => return Ok(flow),
                    }
                }
                Ok(Flow::Continue)
            }
            Stmt::Break { .. } => Ok(Flow::Break),
            Stmt::Continue { .. } => Ok(Flow::LoopContinue),
            Stmt::Match { value, arms, span } => {
                let scrutinee = self.eval(value, env)?;
                for arm in arms {
                    if let Some(bindings) = match_pattern(&scrutinee, &arm.pattern) {
                        env.push();
                        for (name, value) in bindings {
                            env.declare(name, value, false);
                        }
                        if let Some(guard) = &arm.guard {
                            let guard_value = self.eval(guard, env);
                            let guard_matches = match guard_value {
                                Ok(value) => self.expect_bool(value, guard.span),
                                Err(error) => Err(error),
                            };
                            let guard_matches = match guard_matches {
                                Ok(value) => value,
                                Err(error) => {
                                    env.pop();
                                    return Err(error);
                                }
                            };
                            if !guard_matches {
                                env.pop();
                                continue;
                            }
                        }
                        let flow = self.exec_block(&arm.body, env)?;
                        env.pop();
                        return Ok(flow);
                    }
                }
                Err(self.runtime_error(*span, "non-exhaustive match at runtime"))
            }
            Stmt::Transaction { span, .. } => {
                let Some(database_url) = self.database_url.clone() else {
                    return Err(self.runtime_error(
                        *span,
                        "transaction execution requires a database runtime",
                    ));
                };
                let Stmt::Transaction { body, .. } = statement else {
                    unreachable!();
                };
                let queries = transaction_queries(self, body, env, *span)?;
                self.ensure_decimal_storage(*span)?;
                zelyra_database::execute_queries(&database_url, &queries, true).map_err(
                    |error| {
                        self.runtime_error(*span, format!("database transaction failed: {error}"))
                    },
                )?;
                Ok(Flow::Continue)
            }
            Stmt::Parallel { body, span } => self.exec_parallel(body, *span, env),
        }
    }
    fn expect_bool(&self, value: Value, span: Span) -> Result<bool, RuntimeError> {
        match value {
            Value::Bool(v) => Ok(v),
            other => Err(self.runtime_error(span, format!("expected Bool, found {}", other.ty()))),
        }
    }
    fn eval_scalar_constructor(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if args.len() != 1 {
            return Err(self.runtime_error(span, format!("{name} expects exactly one argument")));
        }
        let argument = self.eval(&args[0], env)?;
        match (name, argument) {
            ("decimal", Value::String(source)) => {
                parse_decimal(&source).map(Value::Decimal).ok_or_else(|| {
                    self.runtime_error(
                        args[0].span,
                        "decimal expects a base-10 value with at most 9 fractional digits",
                    )
                })
            }
            ("decimal", value) => Err(self.runtime_error(
                args[0].span,
                format!("decimal expects String, found {}", value.ty()),
            )),
            ("date", Value::String(value)) if valid_date(&value) => Ok(Value::Date(value)),
            ("time", Value::String(value)) if valid_time(&value) => Ok(Value::Time(value)),
            ("date" | "time", Value::String(value)) => Err(self.runtime_error(
                args[0].span,
                format!(
                    "{name} received invalid value `{value}`; expected {}",
                    if name == "date" {
                        "YYYY-MM-DD"
                    } else {
                        "HH:MM:SS"
                    }
                ),
            )),
            ("date" | "time", value) => Err(self.runtime_error(
                args[0].span,
                format!("{name} expects String, found {}", value.ty()),
            )),
            ("duration_ms", Value::Int(value)) => Ok(Value::Duration(value)),
            ("duration_ms", value) => Err(self.runtime_error(
                args[0].span,
                format!("duration_ms expects Int, found {}", value.ty()),
            )),
            ("bytes", Value::Array(values)) => values
                .into_iter()
                .map(|value| match value {
                    Value::Int(value) => u8::try_from(value).map_err(|_| {
                        self.runtime_error(args[0].span, "bytes values must be between 0 and 255")
                    }),
                    other => Err(self.runtime_error(
                        args[0].span,
                        format!("bytes expects Int values, found {}", other.ty()),
                    )),
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Bytes),
            ("bytes", value) => Err(self.runtime_error(
                args[0].span,
                format!("bytes expects an Int array, found {}", value.ty()),
            )),
            _ => Err(self.runtime_error(span, format!("unknown scalar constructor `{name}`"))),
        }
    }
    fn eval_language_value_builtin(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match name {
            "decimal" | "date" | "time" | "duration_ms" | "bytes" => {
                self.eval_scalar_constructor(name, args, env, span)
            }
            _ => self.eval_collection_builtin(name, args, env, span),
        }
    }
    fn eval_collection_builtin(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match name {
            "map" | "filter" | "find" | "reduce" => {
                self.eval_higher_order_builtin(name, args, env, span)
            }
            "tuple" => {
                if args.len() < 2 {
                    return Err(self.runtime_error(span, "tuple expects at least two values"));
                }
                args.iter()
                    .map(|arg| self.eval(arg, env))
                    .collect::<Result<Vec<_>, _>>()
                    .map(Value::Tuple)
            }
            "set" => {
                if args.len() != 1 {
                    return Err(
                        self.runtime_error(span, format!("{name} expects exactly one array"))
                    );
                }
                match self.eval(&args[0], env)? {
                    Value::Array(values) => Ok(Value::Set(canonical_set(values))),
                    value => Err(self.runtime_error(
                        args[0].span,
                        format!("{name} expects an array, found {}", value.ty()),
                    )),
                }
            }
            "set_add" | "set_remove" => {
                if args.len() != 2 {
                    return Err(
                        self.runtime_error(span, format!("{name} expects a Set and one element"))
                    );
                }
                let collection = self.eval(&args[0], env)?;
                let element = self.eval(&args[1], env)?;
                let Value::Set(mut values) = collection else {
                    return Err(self.runtime_error(
                        args[0].span,
                        format!("{name} expects a Set, found {}", collection.ty()),
                    ));
                };
                if name == "set_add" {
                    if !values.contains(&element) {
                        values.push(element);
                    }
                } else {
                    values.retain(|value| value != &element);
                }
                Ok(Value::Set(canonical_set(values)))
            }
            _ => Err(self.runtime_error(span, format!("unknown collection builtin `{name}`"))),
        }
    }

    fn eval_higher_order_builtin(
        &mut self,
        name: &str,
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let expected_count = if name == "reduce" { 3 } else { 2 };
        if args.len() != expected_count {
            return Err(
                self.runtime_error(span, format!("`{name}` expects {expected_count} arguments"))
            );
        }
        let values = self.eval(&args[0], env)?;
        let Value::Array(values) = values else {
            return Err(self.runtime_error(
                args[0].span,
                format!("`{name}` expects an array, found `{}`", values.ty()),
            ));
        };
        let mut callback_index = 1;
        let mut accumulator = if name == "reduce" {
            callback_index = 2;
            Some(self.eval(&args[1], env)?)
        } else {
            None
        };
        let callback = self.eval(&args[callback_index], env)?;
        let Value::Function(callback) = callback else {
            return Err(self.runtime_error(
                args[callback_index].span,
                format!("`{name}` expects a function or lambda callback"),
            ));
        };
        match name {
            "map" => values
                .into_iter()
                .map(|value| self.invoke_callback(&callback, vec![value], span))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            "filter" => {
                let mut selected = Vec::new();
                for value in values {
                    let keep = self.invoke_callback(&callback, vec![value.clone()], span)?;
                    match keep {
                        Value::Bool(true) => selected.push(value),
                        Value::Bool(false) => {}
                        other => {
                            return Err(self.runtime_error(
                                span,
                                format!("filter callback must return Bool, found `{}`", other.ty()),
                            ));
                        }
                    }
                }
                Ok(Value::Array(selected))
            }
            "find" => {
                for value in values {
                    match self.invoke_callback(&callback, vec![value.clone()], span)? {
                        Value::Bool(true) => return Ok(Value::Option(Some(Box::new(value)))),
                        Value::Bool(false) => {}
                        other => {
                            return Err(self.runtime_error(
                                span,
                                format!("find callback must return Bool, found `{}`", other.ty()),
                            ));
                        }
                    }
                }
                Ok(Value::Option(None))
            }
            "reduce" => {
                let mut current = accumulator
                    .take()
                    .expect("reduce accumulator was evaluated above");
                for value in values {
                    current = self.invoke_callback(&callback, vec![current, value], span)?;
                }
                Ok(current)
            }
            _ => unreachable!(),
        }
    }

    fn invoke_callback(
        &mut self,
        callback: &FunctionValue,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if callback.params.len() != args.len() {
            return Err(self.runtime_error(
                span,
                format!(
                    "callback expects {} argument(s), found {}",
                    callback.params.len(),
                    args.len()
                ),
            ));
        }
        if let Some(caller_capabilities) = self.active_capabilities.last() {
            let declared = caller_capabilities
                .iter()
                .map(String::as_str)
                .collect::<HashSet<_>>();
            if let Some(capability) = callback
                .capabilities
                .iter()
                .find(|capability| !function_declares_capability(&declared, capability))
            {
                return Err(self.runtime_error(
                    span,
                    format!("runtime capability denied: callback requires `{capability}`"),
                ));
            }
        }
        if callback.lambda_body.is_none() {
            return self.call_with_type_args(&callback.name, &[], args, span);
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
        let mut lambda_env = Environment::new();
        for (name, value) in &callback.captures {
            lambda_env.declare(name.clone(), value.clone(), false);
        }
        lambda_env.push();
        for (name, value) in callback.lambda_params.iter().zip(args) {
            lambda_env.declare(name.clone(), value, false);
        }
        let result = self.eval(
            callback.lambda_body.as_deref().expect("lambda body exists"),
            &mut lambda_env,
        );
        self.call_depth -= 1;
        result
    }
    fn eval_call_expr(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if let Some(Value::Function(function)) = env.get(name) {
            if !type_args.is_empty() {
                return Err(self.runtime_error(
                    span,
                    "indirect function calls do not accept explicit type arguments",
                ));
            }
            if let Some(caller_capabilities) = self.active_capabilities.last() {
                let declared = caller_capabilities
                    .iter()
                    .map(String::as_str)
                    .collect::<HashSet<_>>();
                if let Some(capability) = function
                    .capabilities
                    .iter()
                    .find(|capability| !function_declares_capability(&declared, capability))
                {
                    return Err(self.runtime_error(
                        span,
                        format!(
                            "runtime capability denied: current function does not declare `{capability}` required by indirect call through `{name}`"
                        ),
                    ));
                }
            }
            let values = args
                .iter()
                .map(|argument| self.eval(argument, env))
                .collect::<Result<Vec<_>, _>>()?;
            return self.invoke_callback(&function, values, span);
        }
        if let Some((enum_name, variant)) = name.rsplit_once("::") {
            if let Some(definition) = self.enum_definitions.get(enum_name).cloned() {
                let payload_types = definition
                    .variant_payloads
                    .iter()
                    .find(|(name, _)| name == variant)
                    .map(|(_, types)| types.clone())
                    .ok_or_else(|| {
                        self.runtime_error(
                            span,
                            format!("unknown enum variant `{enum_name}::{variant}`"),
                        )
                    })?;
                if definition.type_params.is_empty() && !type_args.is_empty() {
                    return Err(self.runtime_error(
                        span,
                        format!("enum `{enum_name}` does not accept type arguments"),
                    ));
                }
                if payload_types.is_empty() && definition.type_params.is_empty() {
                    return Err(self.runtime_error(
                        span,
                        format!(
                            "unit enum variant `{enum_name}::{variant}` is written without parentheses"
                        ),
                    ));
                }
                if args.len() != payload_types.len() {
                    return Err(self.runtime_error(
                        span,
                        format!(
                            "enum variant `{enum_name}::{variant}` expects {} payload value(s), found {}",
                            payload_types.len(),
                            args.len()
                        ),
                    ));
                }
                let payloads = args
                    .iter()
                    .map(|argument| self.eval(argument, env))
                    .collect::<Result<Vec<_>, _>>()?;
                if !type_args.is_empty() && type_args.len() != definition.type_params.len() {
                    return Err(self.runtime_error(
                        span,
                        format!(
                            "enum `{enum_name}` expects {} type argument(s), found {}",
                            definition.type_params.len(),
                            type_args.len()
                        ),
                    ));
                }
                let parameters = definition
                    .type_params
                    .iter()
                    .cloned()
                    .collect::<HashSet<_>>();
                let mut substitutions = definition
                    .type_params
                    .iter()
                    .zip(type_args)
                    .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                    .collect::<HashMap<_, _>>();
                if type_args.is_empty() {
                    for (expected, payload) in payload_types.iter().zip(&payloads) {
                        infer_generic_types(
                            expected,
                            &payload.ty(),
                            &parameters,
                            &mut substitutions,
                        );
                    }
                }
                let concrete_args = definition
                    .type_params
                    .iter()
                    .map(|parameter| substitutions.get(parameter).cloned())
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| {
                        self.runtime_error(
                            span,
                            format!("cannot infer generic arguments for enum `{enum_name}`"),
                        )
                    })?;
                let instantiated_payloads = payload_types
                    .iter()
                    .map(|expected| substitute_generic_type(expected, &substitutions))
                    .collect::<Vec<_>>();
                for ((argument, payload), expected) in
                    args.iter().zip(&payloads).zip(&instantiated_payloads)
                {
                    if !compatible(
                        &self.runtime_alias_type(expected, span)?,
                        &self.runtime_alias_type(&payload.ty(), span)?,
                    ) {
                        return Err(self.runtime_error(
                            argument.span,
                            format!(
                                "enum variant `{enum_name}::{variant}` payload expects `{expected}`, found `{}`",
                                payload.ty()
                            ),
                        ));
                    }
                }
                return Ok(Value::Enum {
                    type_name: enum_name.to_owned(),
                    type_args: concrete_args,
                    variant: variant.to_owned(),
                    payloads,
                });
            }
        }
        if !is_runtime_builtin(name) && self.functions.contains_key(name) {
            let values = args
                .iter()
                .map(|arg| self.eval(arg, env))
                .collect::<Result<Vec<_>, _>>()?;
            return self.call_with_type_args(name, type_args, values, span);
        }
        self.eval_builtin_call(name, type_args, args, env, span)
    }

    fn eval_builtin_call(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[Expr],
        env: &mut Environment,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if matches!(
            name,
            "map"
                | "filter"
                | "find"
                | "reduce"
                | "tuple"
                | "set"
                | "set_add"
                | "set_remove"
                | "decimal"
                | "date"
                | "time"
                | "duration_ms"
                | "bytes"
        ) {
            return self.eval_language_value_builtin(name, args, env, span);
        }
        if name == "render_pdf" {
            if args.len() != 2 || !type_args.is_empty() {
                return Err(self
                    .runtime_error(span, "render_pdf expects a named View and one record value"));
            }
            let view_name = match self.eval(&args[0], env)? {
                Value::String(name) => name,
                value => {
                    return Err(self.runtime_error(
                        args[0].span,
                        format!("render_pdf expects a View name, found {}", value.ty()),
                    ));
                }
            };
            let mut html = self
                .view_definitions
                .get(&view_name)
                .map(|view| view.html.clone())
                .ok_or_else(|| {
                    self.runtime_error(args[0].span, format!("unknown PDF view `{view_name}`"))
                })?;
            let data = self.eval(&args[1], env)?;
            html = html.replace("<slot />", "").replace("<slot/>", "");
            html = interpolate_pdf_data(&html, &data)
                .map_err(|message| self.runtime_error(span, message))?;
            pdf::render_html_to_pdf(&html)
                .map(Value::Bytes)
                .map_err(|error| self.runtime_error(span, error))
        } else if name == "read_console" {
            if args.len() != 1 {
                return Err(
                    self.runtime_error(span, "read_console expects exactly one String prompt")
                );
            }
            self.require_runtime_capability("Console", "console input", span)?;
            let prompt = self.eval(&args[0], env)?;
            let Value::String(prompt) = prompt else {
                return Err(
                    self.runtime_error(args[0].span, "read_console expects a String prompt")
                );
            };
            read_console_line(&prompt)
                .map(|value| Value::Option(value.map(|value| Box::new(Value::String(value)))))
                .map_err(|error| self.runtime_error(span, format!("console input failed: {error}")))
        } else if name == "print" {
            let value = self.eval(
                args.first()
                    .ok_or_else(|| self.runtime_error(span, "`print` expects one argument"))?,
                env,
            )?;
            self.output.push(value.output());
            Ok(Value::Unit)
        } else if name == "assert" {
            if !(1..=2).contains(&args.len()) {
                return Err(self.runtime_error(
                    span,
                    "`assert` expects a Bool condition and an optional String message",
                ));
            }
            let condition = self.eval(&args[0], env)?;
            let Value::Bool(condition) = condition else {
                return Err(self.runtime_error(args[0].span, "`assert` expects a Bool condition"));
            };
            let message = if let Some(message_expr) = args.get(1) {
                let value = self.eval(message_expr, env)?;
                let Value::String(message) = value else {
                    return Err(
                        self.runtime_error(message_expr.span, "`assert` message must be a String")
                    );
                };
                Some(message)
            } else {
                None
            };
            if condition {
                Ok(Value::Unit)
            } else {
                Err(self.runtime_error(span, message.unwrap_or_else(|| "assertion failed".into())))
            }
        } else if name == "array_replace_result" {
            if !type_args.is_empty() || args.len() != 3 {
                return Err(self.runtime_error(
                    span,
                    "array_replace_result expects an array, Int index, and value",
                ));
            }
            let array = self.eval(&args[0], env)?;
            let index = self.eval(&args[1], env)?;
            let value = self.eval(&args[2], env)?;
            let (Value::Array(mut values), Value::Int(index)) = (array, index) else {
                return Err(
                    self.runtime_error(span, "array_replace_result expects an array and Int index")
                );
            };
            let result = if let Ok(index) = usize::try_from(index) {
                if let Some(slot) = values.get_mut(index) {
                    *slot = value;
                    Ok(Value::Array(values))
                } else {
                    Err("array_replace_result index is out of bounds".to_owned())
                }
            } else {
                Err("array_replace_result index is out of bounds".to_owned())
            };
            Ok(Value::Result(
                result
                    .map(Box::new)
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        } else if name == "object_set_field" {
            if self.parallel_worker {
                return Err(self.runtime_error(
                    span,
                    "class field mutation is not allowed inside a `parallel` block",
                ));
            }
            if !type_args.is_empty() || args.len() != 3 {
                return Err(self.runtime_error(
                    span,
                    "class field update expects an object, literal field name, and value",
                ));
            }
            let object_value = self.eval(&args[0], env)?;
            let field_name = self.eval(&args[1], env)?;
            let value = self.eval(&args[2], env)?;
            let (reference, type_name, type_args, field_name) = match (object_value, field_name) {
                (
                    Value::ClassRef {
                        reference,
                        type_name,
                        type_args,
                    },
                    Value::String(field_name),
                ) => (reference, type_name, type_args, field_name),
                _ => {
                    return Err(self.runtime_error(
                        span,
                        "field assignment requires a class instance and literal field name",
                    ))
                }
            };
            let definition = self.record_definitions.get(&type_name).ok_or_else(|| {
                self.runtime_error(span, format!("unknown class type `{type_name}`"))
            })?;
            if !definition.is_class {
                return Err(self.runtime_error(span, "field assignment requires a class instance"));
            }
            let field = definition
                .fields
                .iter()
                .find(|field| field.name == field_name)
                .ok_or_else(|| {
                    self.runtime_error(
                        args[1].span,
                        format!("unknown field `{field_name}` in class `{type_name}`"),
                    )
                })?;
            let substitutions = definition
                .type_params
                .iter()
                .zip(&type_args)
                .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                .collect::<HashMap<_, _>>();
            let expected = substitute_generic_type(&field.ty, &substitutions);
            if !compatible(
                &self.runtime_alias_type(&expected, span)?,
                &self.runtime_alias_type(&value.ty(), span)?,
            ) {
                return Err(self.runtime_error(
                    args[2].span,
                    format!(
                        "class field `{field_name}` expects `{expected}`, found `{}`",
                        value.ty()
                    ),
                ));
            }
            if class_reference_would_cycle(&value, &reference)
                .map_err(|message| self.runtime_error(span, message))?
            {
                return Err(self.runtime_error(
                    args[2].span,
                    "class field update would create a reference cycle",
                ));
            }
            let mut object = reference
                .object
                .lock()
                .map_err(|_| self.runtime_error(span, "class object is unavailable"))?;
            object.fields.insert(field_name, value);
            Ok(Value::Unit)
        } else if name == "record_update" {
            if type_args.len() != 1 || args.is_empty() || args.len().is_multiple_of(2) {
                return Err(self.runtime_error(
                    span,
                    "record update expects a base record followed by field/value pairs",
                ));
            }
            let expected = self.runtime_alias_type(&type_args[0], span)?;
            let base = self.eval(&args[0], env)?;
            let (type_name, type_arguments, mut fields) = match base {
                Value::Object {
                    type_name,
                    type_args,
                    fields,
                } => (type_name, type_args, fields),
                value => {
                    return Err(self.runtime_error(
                        args[0].span,
                        format!("record update expects a record, found `{}`", value.ty()),
                    ))
                }
            };
            let actual = if type_arguments.is_empty() {
                Type::Named(type_name.clone())
            } else {
                Type::Applied {
                    name: type_name.clone(),
                    args: type_arguments.clone(),
                }
            };
            if !compatible(&expected, &actual) {
                return Err(self.runtime_error(
                    args[0].span,
                    format!("record update expects `{expected}`, found `{actual}`"),
                ));
            }
            let definition = self
                .record_definitions
                .get(&type_name)
                .cloned()
                .ok_or_else(|| {
                    self.runtime_error(span, format!("unknown record type `{type_name}`"))
                })?;
            if definition.is_class {
                return Err(self.runtime_error(
                    args[0].span,
                    "record update cannot copy or update a class reference",
                ));
            }
            let substitutions = definition
                .type_params
                .iter()
                .zip(&type_arguments)
                .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                .collect::<HashMap<_, _>>();
            for pair in args[1..].chunks(2) {
                let Value::String(field_name) = self.eval(&pair[0], env)? else {
                    return Err(self.runtime_error(
                        pair[0].span,
                        "record update field name must be a String literal",
                    ));
                };
                let field = definition
                    .fields
                    .iter()
                    .find(|field| field.name == field_name)
                    .ok_or_else(|| {
                        self.runtime_error(
                            pair[0].span,
                            format!("unknown field `{field_name}` in record `{type_name}`"),
                        )
                    })?;
                let value = self.eval(&pair[1], env)?;
                let field_type = substitute_generic_type(&field.ty, &substitutions);
                if !compatible(
                    &self.runtime_alias_type(&field_type, pair[1].span)?,
                    &self.runtime_alias_type(&value.ty(), pair[1].span)?,
                ) {
                    return Err(self.runtime_error(
                        pair[1].span,
                        format!(
                            "record field `{field_name}` expects `{field_type}`, found `{}`",
                            value.ty()
                        ),
                    ));
                }
                fields.insert(field_name, value);
            }
            Ok(Value::Object {
                type_name,
                type_args: type_arguments,
                fields,
            })
        } else if name == "record_with" {
            if !type_args.is_empty() || args.len() != 3 {
                return Err(self.runtime_error(
                    span,
                    "record_with expects a record, literal field name, and value",
                ));
            }
            let record = self.eval(&args[0], env)?;
            let field_name = self.eval(&args[1], env)?;
            let value = self.eval(&args[2], env)?;
            let (
                Value::Object {
                    type_name,
                    type_args,
                    mut fields,
                },
                Value::String(field_name),
            ) = (record, field_name)
            else {
                return Err(
                    self.runtime_error(span, "record_with expects a record and String field name")
                );
            };
            let definition = self.record_definitions.get(&type_name).ok_or_else(|| {
                self.runtime_error(span, format!("unknown record type `{type_name}`"))
            })?;
            let field = definition
                .fields
                .iter()
                .find(|field| field.name == field_name)
                .ok_or_else(|| {
                    self.runtime_error(
                        args[1].span,
                        format!("unknown field `{field_name}` in record `{type_name}`"),
                    )
                })?;
            let substitutions = definition
                .type_params
                .iter()
                .zip(&type_args)
                .map(|(parameter, argument)| (parameter.clone(), argument.clone()))
                .collect::<HashMap<_, _>>();
            let field_type = substitute_generic_type(&field.ty, &substitutions);
            if !compatible(
                &self.runtime_alias_type(&field_type, span)?,
                &self.runtime_alias_type(&value.ty(), span)?,
            ) {
                return Err(self.runtime_error(
                    args[2].span,
                    format!(
                        "record field `{field_name}` expects `{}`, found `{}`",
                        field_type,
                        value.ty()
                    ),
                ));
            }
            fields.insert(field_name, value);
            Ok(Value::Object {
                type_name,
                type_args,
                fields,
            })
        } else if name == "range" {
            if !type_args.is_empty() || !(2..=3).contains(&args.len()) {
                return Err(self.runtime_error(
                    span,
                    "range expects two Int bounds and an optional Int step",
                ));
            }
            let start = self.eval(&args[0], env)?;
            let end = self.eval(&args[1], env)?;
            let (Value::Int(start), Value::Int(end)) = (start, end) else {
                return Err(self.runtime_error(
                    span,
                    "range expects two Int bounds and an optional Int step",
                ));
            };
            let step = match args.get(2) {
                Some(expression) => match self.eval(expression, env)? {
                    Value::Int(step) => step,
                    _ => {
                        return Err(self.runtime_error(expression.span, "range step must be an Int"))
                    }
                },
                None => 1,
            };
            if step == 0 {
                return Err(self.runtime_error(
                    args.get(2).map_or(span, |step| step.span),
                    "range step must not be zero",
                ));
            }
            let mut values = Vec::new();
            for value in integer_range(start, end, step) {
                if values.len() == 1_000_000 {
                    return Err(
                        self.runtime_error(span, "range array exceeds the 1,000,000 element limit")
                    );
                }
                values.push(Value::Int(value));
            }
            Ok(Value::Array(values))
        } else if name == "len" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "`len` expects exactly one argument"));
            }
            match self.eval(&args[0], env)? {
                Value::Array(values) | Value::Tuple(values) | Value::Set(values) => {
                    Ok(Value::Int(values.len() as i64))
                }
                Value::Map(entries) => Ok(Value::Int(entries.len() as i64)),
                Value::String(s) => Ok(Value::Int(s.chars().count() as i64)),
                Value::Bytes(b) => Ok(Value::Int(b.len() as i64)),
                value => Err(self.runtime_error(
                    span,
                    format!(
                        "`len` expects an array, Tuple, Set, Map, String or Bytes, found {}",
                        value.ty()
                    ),
                )),
            }
        } else if name == "append" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "`append` expects an array and one value"));
            }
            let array = self.eval(&args[0], env)?;
            let value = self.eval(&args[1], env)?;
            match array {
                Value::Array(mut values) => {
                    values.push(value);
                    Ok(Value::Array(values))
                }
                value => Err(self.runtime_error(
                    span,
                    format!("`append` expects an array, found {}", value.ty()),
                )),
            }
        } else if name == "contains" {
            if args.len() != 2 {
                return Err(self.runtime_error(
                    span,
                    "`contains` expects an array, Set or Map and one value",
                ));
            }
            let collection = self.eval(&args[0], env)?;
            let value = self.eval(&args[1], env)?;
            match collection {
                Value::Array(values) => Ok(Value::Bool(values.iter().any(|item| item == &value))),
                Value::Set(values) => Ok(Value::Bool(values.contains(&value))),
                Value::Map(entries) => {
                    Ok(Value::Bool(entries.iter().any(|(key, _)| key == &value)))
                }
                value => Err(self.runtime_error(
                    span,
                    format!(
                        "`contains` expects an array, Set or Map, found {}",
                        value.ty()
                    ),
                )),
            }
        } else if name == "text_contains" {
            if args.len() != 2 {
                return Err(
                    self.runtime_error(span, "text_contains expects a text and search text")
                );
            }
            let text = self.eval(&args[0], env)?;
            let search = self.eval(&args[1], env)?;
            match (text, search) {
                (Value::String(text), Value::String(search)) => {
                    Ok(Value::Bool(text.contains(&search)))
                }
                _ => Err(self.runtime_error(span, "text_contains expects String arguments")),
            }
        } else if name == "text_trim" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "text_trim expects exactly one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(text) => Ok(Value::String(text.trim().to_owned())),
                _ => Err(self.runtime_error(args[0].span, "text_trim expects a String")),
            }
        } else if name == "text_chars" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "text_chars expects exactly one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(text) => Ok(Value::Array(
                    text.chars()
                        .map(|character| Value::String(character.to_string()))
                        .collect(),
                )),
                _ => Err(self.runtime_error(args[0].span, "text_chars expects a String")),
            }
        } else if name == "abs" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "abs expects exactly one numeric argument"));
            }
            match self.eval(&args[0], env)? {
                Value::Int(val) => val
                    .checked_abs()
                    .map(Value::Int)
                    .ok_or_else(|| self.runtime_error(args[0].span, "Int overflow in abs")),
                Value::Float(val) if val.is_finite() => Ok(Value::Float(val.abs())),
                Value::Float(_) => {
                    Err(self.runtime_error(args[0].span, "abs requires a finite Float"))
                }
                Value::Decimal(val) => val
                    .checked_abs()
                    .map(Value::Decimal)
                    .ok_or_else(|| self.runtime_error(args[0].span, "Decimal overflow in abs")),
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("abs expects numeric value, found {}", other.ty()),
                )),
            }
        } else if name == "min" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "min expects exactly two arguments"));
            }
            let a = self.eval(&args[0], env)?;
            let b = self.eval(&args[1], env)?;
            match (&a, &b) {
                (Value::Int(va), Value::Int(vb)) => Ok(Value::Int(*va.min(vb))),
                (Value::UInt(va), Value::UInt(vb)) => Ok(Value::UInt(*va.min(vb))),
                (Value::Float(va), Value::Float(vb)) if va.is_finite() && vb.is_finite() => {
                    Ok(Value::Float(va.min(*vb)))
                }
                (Value::Float(_), Value::Float(_)) => {
                    Err(self.runtime_error(span, "min requires finite Float arguments"))
                }
                (Value::Decimal(va), Value::Decimal(vb)) => Ok(Value::Decimal(*va.min(vb))),
                (Value::String(va), Value::String(vb)) => {
                    if va <= vb {
                        Ok(a)
                    } else {
                        Ok(b)
                    }
                }
                _ => {
                    Err(self
                        .runtime_error(span, "min expects matching numeric or String arguments"))
                }
            }
        } else if name == "max" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "max expects exactly two arguments"));
            }
            let a = self.eval(&args[0], env)?;
            let b = self.eval(&args[1], env)?;
            match (&a, &b) {
                (Value::Int(va), Value::Int(vb)) => Ok(Value::Int(*va.max(vb))),
                (Value::UInt(va), Value::UInt(vb)) => Ok(Value::UInt(*va.max(vb))),
                (Value::Float(va), Value::Float(vb)) if va.is_finite() && vb.is_finite() => {
                    Ok(Value::Float(va.max(*vb)))
                }
                (Value::Float(_), Value::Float(_)) => {
                    Err(self.runtime_error(span, "max requires finite Float arguments"))
                }
                (Value::Decimal(va), Value::Decimal(vb)) => Ok(Value::Decimal(*va.max(vb))),
                (Value::String(va), Value::String(vb)) => {
                    if va >= vb {
                        Ok(a)
                    } else {
                        Ok(b)
                    }
                }
                _ => {
                    Err(self
                        .runtime_error(span, "max expects matching numeric or String arguments"))
                }
            }
        } else if name == "clamp" {
            if args.len() != 3 {
                return Err(self.runtime_error(span, "clamp expects val, min, and max"));
            }
            let val = self.eval(&args[0], env)?;
            let min_val = self.eval(&args[1], env)?;
            let max_val = self.eval(&args[2], env)?;
            match (&val, &min_val, &max_val) {
                (Value::Int(v), Value::Int(mn), Value::Int(mx)) => {
                    if mn > mx {
                        return Err(
                            self.runtime_error(span, "clamp minimum must not exceed maximum")
                        );
                    }
                    Ok(Value::Int(*v.max(mn).min(mx)))
                }
                (Value::UInt(v), Value::UInt(mn), Value::UInt(mx)) => {
                    if mn > mx {
                        return Err(
                            self.runtime_error(span, "clamp minimum must not exceed maximum")
                        );
                    }
                    Ok(Value::UInt(*v.max(mn).min(mx)))
                }
                (Value::Float(v), Value::Float(mn), Value::Float(mx))
                    if v.is_finite() && mn.is_finite() && mx.is_finite() =>
                {
                    if mn > mx {
                        return Err(
                            self.runtime_error(span, "clamp minimum must not exceed maximum")
                        );
                    }
                    Ok(Value::Float(v.max(*mn).min(*mx)))
                }
                (Value::Float(_), Value::Float(_), Value::Float(_)) => {
                    Err(self.runtime_error(span, "clamp requires finite Float arguments"))
                }
                (Value::Decimal(v), Value::Decimal(mn), Value::Decimal(mx)) => {
                    if mn > mx {
                        return Err(
                            self.runtime_error(span, "clamp minimum must not exceed maximum")
                        );
                    }
                    Ok(Value::Decimal(*v.max(mn).min(mx)))
                }
                _ => Err(self.runtime_error(span, "clamp expects matching numeric arguments")),
            }
        } else if name == "to_string" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "to_string expects exactly one argument"));
            }
            let val = self.eval(&args[0], env)?;
            Ok(Value::String(val.output()))
        } else if name == "parse_int" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "parse_int expects exactly one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(text) => match text.trim().parse::<i64>() {
                    Ok(parsed) => Ok(Value::Result(Ok(Box::new(Value::Int(parsed))))),
                    Err(_) => Ok(Value::Result(Err(Box::new(Value::String(format!(
                        "invalid integer: {text}"
                    )))))),
                },
                _ => Err(self.runtime_error(args[0].span, "parse_int expects a String")),
            }
        } else if name == "parse_float" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "parse_float expects exactly one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(text) => match text.trim().parse::<f64>() {
                    Ok(parsed) if parsed.is_finite() => {
                        Ok(Value::Result(Ok(Box::new(Value::Float(parsed)))))
                    }
                    Err(_) => Ok(Value::Result(Err(Box::new(Value::String(format!(
                        "invalid float: {text}"
                    )))))),
                    Ok(_) => Ok(Value::Result(Err(Box::new(Value::String(format!(
                        "invalid float: {text}"
                    )))))),
                },
                _ => Err(self.runtime_error(args[0].span, "parse_float expects a String")),
            }
        } else if name == "sort" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "sort expects exactly one array"));
            }
            match self.eval(&args[0], env)? {
                Value::Array(mut items) => {
                    items.sort_by(|a, b| match (a, b) {
                        (Value::Int(x), Value::Int(y)) => x.cmp(y),
                        (Value::UInt(x), Value::UInt(y)) => x.cmp(y),
                        (Value::Float(x), Value::Float(y)) => {
                            x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        (Value::Decimal(x), Value::Decimal(y)) => x.cmp(y),
                        (Value::String(x), Value::String(y)) => x.cmp(y),
                        _ => std::cmp::Ordering::Equal,
                    });
                    Ok(Value::Array(items))
                }
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("sort expects array, found {}", other.ty()),
                )),
            }
        } else if name == "sort_desc" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "sort_desc expects exactly one array"));
            }
            match self.eval(&args[0], env)? {
                Value::Array(mut items) => {
                    items.sort_by(|a, b| match (a, b) {
                        (Value::Int(x), Value::Int(y)) => y.cmp(x),
                        (Value::UInt(x), Value::UInt(y)) => y.cmp(x),
                        (Value::Float(x), Value::Float(y)) => {
                            y.partial_cmp(x).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        (Value::Decimal(x), Value::Decimal(y)) => y.cmp(x),
                        (Value::String(x), Value::String(y)) => y.cmp(x),
                        _ => std::cmp::Ordering::Equal,
                    });
                    Ok(Value::Array(items))
                }
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("sort_desc expects array, found {}", other.ty()),
                )),
            }
        } else if name == "reverse" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "reverse expects exactly one array"));
            }
            match self.eval(&args[0], env)? {
                Value::Array(mut items) => {
                    items.reverse();
                    Ok(Value::Array(items))
                }
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("reverse expects array, found {}", other.ty()),
                )),
            }
        } else if name == "slice" {
            if args.len() != 3 {
                return Err(
                    self.runtime_error(span, "slice expects array, start index, and length")
                );
            }
            let arr = self.eval(&args[0], env)?;
            let start = self.eval(&args[1], env)?;
            let len_val = self.eval(&args[2], env)?;
            match (arr, start, len_val) {
                (Value::Array(items), Value::Int(start_idx), Value::Int(length)) => {
                    if start_idx < 0 || length < 0 {
                        return Err(self.runtime_error(span, "slice indices must be non-negative"));
                    }
                    let start_usize = start_idx as usize;
                    let len_usize = length as usize;
                    if start_usize > items.len() {
                        return Ok(Value::Array(Vec::new()));
                    }
                    let end_usize = (start_usize + len_usize).min(items.len());
                    Ok(Value::Array(items[start_usize..end_usize].to_vec()))
                }
                _ => {
                    Err(self.runtime_error(span, "slice expects array, Int start, and Int length"))
                }
            }
        } else if name == "concat" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "concat expects two arrays"));
            }
            let a = self.eval(&args[0], env)?;
            let b = self.eval(&args[1], env)?;
            match (a, b) {
                (Value::Array(mut a_items), Value::Array(b_items)) => {
                    a_items.extend(b_items);
                    Ok(Value::Array(a_items))
                }
                _ => Err(self.runtime_error(span, "concat expects two arrays")),
            }
        } else if name == "join" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "join expects array of Strings and delimiter"));
            }
            let arr = self.eval(&args[0], env)?;
            let sep = self.eval(&args[1], env)?;
            match (arr, sep) {
                (Value::Array(items), Value::String(delimiter)) => {
                    let mut pieces = Vec::new();
                    for item in items {
                        pieces.push(item.output());
                    }
                    Ok(Value::String(pieces.join(&delimiter)))
                }
                _ => Err(self.runtime_error(span, "join expects array and String delimiter")),
            }
        } else if name == "text_to_lower" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "text_to_lower expects exactly one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(text) => Ok(Value::String(text.to_lowercase())),
                _ => Err(self.runtime_error(args[0].span, "text_to_lower expects a String")),
            }
        } else if name == "text_to_upper" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "text_to_upper expects exactly one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(text) => Ok(Value::String(text.to_uppercase())),
                _ => Err(self.runtime_error(args[0].span, "text_to_upper expects a String")),
            }
        } else if name == "text_starts_with" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "text_starts_with expects text and prefix"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::String(text), Value::String(prefix)) => {
                    Ok(Value::Bool(text.starts_with(&prefix)))
                }
                _ => Err(self.runtime_error(span, "text_starts_with expects two Strings")),
            }
        } else if name == "text_ends_with" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "text_ends_with expects text and suffix"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::String(text), Value::String(suffix)) => {
                    Ok(Value::Bool(text.ends_with(&suffix)))
                }
                _ => Err(self.runtime_error(span, "text_ends_with expects two Strings")),
            }
        } else if name == "text_split" || name == "split" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, format!("{name} expects text and delimiter")));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::String(text), Value::String(sep)) => {
                    let parts = text
                        .split(&sep)
                        .map(|s| Value::String(s.to_string()))
                        .collect();
                    Ok(Value::Array(parts))
                }
                _ => Err(self.runtime_error(span, format!("{name} expects two Strings"))),
            }
        } else if name == "text_replace" {
            if args.len() != 3 {
                return Err(
                    self.runtime_error(span, "text_replace expects text, search, and replacement")
                );
            }
            match (
                self.eval(&args[0], env)?,
                self.eval(&args[1], env)?,
                self.eval(&args[2], env)?,
            ) {
                (Value::String(text), Value::String(search), Value::String(replacement)) => {
                    Ok(Value::String(text.replace(&search, &replacement)))
                }
                _ => Err(self.runtime_error(span, "text_replace expects three Strings")),
            }
        } else if name == "text_pad_left" {
            if args.len() != 3 {
                return Err(self.runtime_error(
                    span,
                    "text_pad_left expects text, target length, and pad char",
                ));
            }
            match (
                self.eval(&args[0], env)?,
                self.eval(&args[1], env)?,
                self.eval(&args[2], env)?,
            ) {
                (Value::String(text), Value::Int(target_len), Value::String(pad)) => {
                    pad_text_to_codepoints(&text, target_len, &pad, true)
                        .map(Value::String)
                        .map_err(|message| self.runtime_error(args[1].span, message))
                }
                _ => Err(self.runtime_error(span, "text_pad_left expects String, Int, and String")),
            }
        } else if name == "text_pad_right" {
            if args.len() != 3 {
                return Err(self.runtime_error(
                    span,
                    "text_pad_right expects text, target length, and pad char",
                ));
            }
            match (
                self.eval(&args[0], env)?,
                self.eval(&args[1], env)?,
                self.eval(&args[2], env)?,
            ) {
                (Value::String(text), Value::Int(target_len), Value::String(pad)) => {
                    pad_text_to_codepoints(&text, target_len, &pad, false)
                        .map(Value::String)
                        .map_err(|message| self.runtime_error(args[1].span, message))
                }
                _ => {
                    Err(self.runtime_error(span, "text_pad_right expects String, Int, and String"))
                }
            }
        } else if name == "get_or" || name == "get_or_default" {
            if args.len() != 3 {
                return Err(self.runtime_error(
                    span,
                    format!("{name} expects a Map, key, and fallback default"),
                ));
            }
            let map_val = self.eval(&args[0], env)?;
            let key_val = self.eval(&args[1], env)?;
            let default_val = self.eval(&args[2], env)?;
            match map_val {
                Value::Map(entries) => {
                    for (k, v) in entries {
                        if k == key_val {
                            return Ok(v);
                        }
                    }
                    Ok(default_val)
                }
                _ => Err(self.runtime_error(args[0].span, format!("{name} expects a Map"))),
            }
        } else if name == "map_increment" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "map_increment expects a Map<Int> and key"));
            }
            let map = self.eval(&args[0], env)?;
            let key = self.eval(&args[1], env)?;
            let Value::Map(mut entries) = map else {
                return Err(self.runtime_error(args[0].span, "map_increment expects a Map"));
            };
            let current = entries.iter().position(|(existing, _)| existing == &key);
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
                        return Err(self.runtime_error(
                            args[0].span,
                            format!("map_increment expects Int values, found {}", other.ty()),
                        ));
                    }
                },
                None => 1,
            };
            if let Some(index) = current {
                entries[index].1 = Value::Int(next);
            } else {
                entries.push((key, Value::Int(next)));
            }
            Ok(Value::Result(Ok(Box::new(Value::Map(entries)))))
        } else if name == "has_key" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "has_key expects a Map and key"));
            }
            let map_val = self.eval(&args[0], env)?;
            let key_val = self.eval(&args[1], env)?;
            match map_val {
                Value::Map(entries) => {
                    let exists = entries.iter().any(|(k, _)| k == &key_val);
                    Ok(Value::Bool(exists))
                }
                _ => Err(self.runtime_error(args[0].span, "has_key expects a Map")),
            }
        } else if name == "sqrt" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "sqrt expects one Float"));
            }
            match self.eval(&args[0], env)? {
                Value::Float(f) => {
                    if !f.is_finite() || f < 0.0 {
                        return Err(
                            self.runtime_error(span, "sqrt requires a finite non-negative Float")
                        );
                    }
                    Ok(Value::Float(f.sqrt()))
                }
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("sqrt expects Float, found {}", other.ty()),
                )),
            }
        } else if name == "pow" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "pow expects base and exponent"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::Float(b), Value::Float(e)) => {
                    let result = b.powf(e);
                    if !b.is_finite() || !e.is_finite() || !result.is_finite() {
                        return Err(self.runtime_error(
                            span,
                            "pow is undefined or outside the finite Float range",
                        ));
                    }
                    Ok(Value::Float(result))
                }
                _ => Err(self.runtime_error(span, "pow expects Float arguments")),
            }
        } else if name == "floor" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "floor expects one Float"));
            }
            match self.eval(&args[0], env)? {
                Value::Float(f) if f.is_finite() => Ok(Value::Float(f.floor())),
                Value::Float(_) => Err(self.runtime_error(span, "floor requires a finite Float")),
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("floor expects Float, found {}", other.ty()),
                )),
            }
        } else if name == "ceil" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "ceil expects one Float"));
            }
            match self.eval(&args[0], env)? {
                Value::Float(f) if f.is_finite() => Ok(Value::Float(f.ceil())),
                Value::Float(_) => Err(self.runtime_error(span, "ceil requires a finite Float")),
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("ceil expects Float, found {}", other.ty()),
                )),
            }
        } else if name == "round" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "round expects number and decimal places"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::Float(f), Value::Int(d)) => {
                    if !f.is_finite() {
                        return Err(
                            self.runtime_error(args[0].span, "round requires a finite Float")
                        );
                    }
                    if !(-15..=15).contains(&d) {
                        return Err(self.runtime_error(
                            args[1].span,
                            "round decimal places must be between -15 and 15",
                        ));
                    }
                    let factor = (10.0_f64).powi(d as i32);
                    let rounded = (f * factor).round() / factor;
                    if !rounded.is_finite() {
                        return Err(self.runtime_error(
                            span,
                            "round result is outside the finite Float range",
                        ));
                    }
                    Ok(Value::Float(rounded))
                }
                _ => Err(self.runtime_error(span, "round expects Float and Int")),
            }
        } else if name == "text_index_of" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "text_index_of expects text and substring"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::String(text), Value::String(sub)) => match text.find(&sub) {
                    Some(byte_pos) => {
                        let char_pos = text[..byte_pos].chars().count();
                        Ok(Value::Option(Some(Box::new(Value::Int(char_pos as i64)))))
                    }
                    None => Ok(Value::Option(None)),
                },
                _ => Err(self.runtime_error(span, "text_index_of expects two Strings")),
            }
        } else if name == "text_repeat" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "text_repeat expects text and count"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::String(text), Value::Int(count)) => {
                    if count < 0 {
                        return Err(
                            self.runtime_error(span, "text_repeat count must be non-negative")
                        );
                    }
                    let output_bytes = text.len().checked_mul(count as usize);
                    if output_bytes.is_none_or(|bytes| bytes > MAX_TEXT_BUILTIN_OUTPUT_BYTES) {
                        return Err(self.runtime_error(
                            span,
                            "text_repeat output exceeds the 16 MiB text limit",
                        ));
                    }
                    Ok(Value::String(text.repeat(count as usize)))
                }
                _ => Err(self.runtime_error(span, "text_repeat expects String and Int")),
            }
        } else if name == "text_is_empty" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "text_is_empty expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(text) => Ok(Value::Bool(text.is_empty())),
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("text_is_empty expects String, found {}", other.ty()),
                )),
            }
        } else if name == "text_capitalize" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "text_capitalize expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(text) => {
                    let mut c = text.chars();
                    let res = match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    };
                    Ok(Value::String(res))
                }
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("text_capitalize expects String, found {}", other.ty()),
                )),
            }
        } else if name == "unique" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "unique expects one array"));
            }
            match self.eval(&args[0], env)? {
                Value::Array(items) => {
                    let mut out: Vec<Value> = Vec::new();
                    for item in items {
                        if !out.iter().any(|existing| existing == &item) {
                            out.push(item);
                        }
                    }
                    Ok(Value::Array(out))
                }
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("unique expects array, found {}", other.ty()),
                )),
            }
        } else if name == "array_sum" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "sum expects one array"));
            }
            match self.eval(&args[0], env)? {
                Value::Array(items) => {
                    if items.is_empty() {
                        return Ok(Value::Int(0));
                    }
                    match &items[0] {
                        Value::Int(_) => {
                            let mut s: i64 = 0;
                            for it in items {
                                if let Value::Int(n) = it {
                                    s = s.checked_add(n).ok_or_else(|| {
                                        self.runtime_error(span, "sum integer overflow")
                                    })?;
                                }
                            }
                            Ok(Value::Int(s))
                        }
                        Value::Float(_) => {
                            let mut s: f64 = 0.0;
                            for it in items {
                                if let Value::Float(n) = it {
                                    if !n.is_finite() {
                                        return Err(self.runtime_error(
                                            span,
                                            "array_sum requires finite Float elements",
                                        ));
                                    }
                                    s += n;
                                    if !s.is_finite() {
                                        return Err(
                                            self.runtime_error(span, "array_sum Float overflow")
                                        );
                                    }
                                }
                            }
                            Ok(Value::Float(s))
                        }
                        Value::Decimal(_) => {
                            let mut s: i128 = 0;
                            for it in items {
                                if let Value::Decimal(n) = it {
                                    s = s.checked_add(n).ok_or_else(|| {
                                        self.runtime_error(span, "sum decimal overflow")
                                    })?;
                                }
                            }
                            Ok(Value::Decimal(s))
                        }
                        other => Err(self.runtime_error(
                            span,
                            format!("sum expects numbers, found {}", other.ty()),
                        )),
                    }
                }
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("sum expects array, found {}", other.ty()),
                )),
            }
        } else if name == "avg" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "avg expects one array"));
            }
            match self.eval(&args[0], env)? {
                Value::Array(items) => {
                    if items.is_empty() {
                        return Err(self.runtime_error(span, "avg of empty array is undefined"));
                    }
                    let count = items.len() as f64;
                    match &items[0] {
                        Value::Int(_) => {
                            let mut s: f64 = 0.0;
                            for it in items {
                                if let Value::Int(n) = it {
                                    s += n as f64;
                                }
                            }
                            Ok(Value::Float(s / count))
                        }
                        Value::Float(_) => {
                            let mut s: f64 = 0.0;
                            for it in items {
                                if let Value::Float(n) = it {
                                    if !n.is_finite() {
                                        return Err(self.runtime_error(
                                            span,
                                            "avg requires finite Float elements",
                                        ));
                                    }
                                    s += n;
                                    if !s.is_finite() {
                                        return Err(self.runtime_error(span, "avg Float overflow"));
                                    }
                                }
                            }
                            Ok(Value::Float(s / count))
                        }
                        Value::Decimal(_) => {
                            let mut s: i128 = 0;
                            let n_items = items.len() as i128;
                            for it in items {
                                if let Value::Decimal(n) = it {
                                    s = s.checked_add(n).ok_or_else(|| {
                                        self.runtime_error(span, "avg decimal overflow")
                                    })?;
                                }
                            }
                            Ok(Value::Decimal(s / n_items))
                        }
                        other => Err(self.runtime_error(
                            span,
                            format!("avg expects numbers, found {}", other.ty()),
                        )),
                    }
                }
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("avg expects array, found {}", other.ty()),
                )),
            }
        } else if name == "min_of" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "min_of expects one array"));
            }
            match self.eval(&args[0], env)? {
                Value::Array(items) => {
                    if items.is_empty() {
                        Ok(Value::Option(None))
                    } else {
                        if items
                            .iter()
                            .any(|item| matches!(item, Value::Float(value) if !value.is_finite()))
                        {
                            return Err(
                                self.runtime_error(span, "min_of requires finite Float elements")
                            );
                        }
                        let mut m = items[0].clone();
                        for item in &items[1..] {
                            match (&m, item) {
                                (Value::Int(a), Value::Int(b)) if b < a => m = item.clone(),
                                (Value::UInt(a), Value::UInt(b)) if b < a => m = item.clone(),
                                (Value::Float(a), Value::Float(b))
                                    if a.is_finite() && b.is_finite() && b < a =>
                                {
                                    m = item.clone()
                                }
                                (Value::Float(a), Value::Float(b))
                                    if !a.is_finite() || !b.is_finite() =>
                                {
                                    return Err(self.runtime_error(
                                        span,
                                        "min_of requires finite Float elements",
                                    ));
                                }
                                (Value::Decimal(a), Value::Decimal(b)) if b < a => m = item.clone(),
                                (Value::String(a), Value::String(b)) if b < a => m = item.clone(),
                                _ => {}
                            }
                        }
                        Ok(Value::Option(Some(Box::new(m))))
                    }
                }
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("min_of expects array, found {}", other.ty()),
                )),
            }
        } else if name == "max_of" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "max_of expects one array"));
            }
            match self.eval(&args[0], env)? {
                Value::Array(items) => {
                    if items.is_empty() {
                        Ok(Value::Option(None))
                    } else {
                        if items
                            .iter()
                            .any(|item| matches!(item, Value::Float(value) if !value.is_finite()))
                        {
                            return Err(
                                self.runtime_error(span, "max_of requires finite Float elements")
                            );
                        }
                        let mut m = items[0].clone();
                        for item in &items[1..] {
                            match (&m, item) {
                                (Value::Int(a), Value::Int(b)) if b > a => m = item.clone(),
                                (Value::UInt(a), Value::UInt(b)) if b > a => m = item.clone(),
                                (Value::Float(a), Value::Float(b))
                                    if a.is_finite() && b.is_finite() && b > a =>
                                {
                                    m = item.clone()
                                }
                                (Value::Float(a), Value::Float(b))
                                    if !a.is_finite() || !b.is_finite() =>
                                {
                                    return Err(self.runtime_error(
                                        span,
                                        "max_of requires finite Float elements",
                                    ));
                                }
                                (Value::Decimal(a), Value::Decimal(b)) if b > a => m = item.clone(),
                                (Value::String(a), Value::String(b)) if b > a => m = item.clone(),
                                _ => {}
                            }
                        }
                        Ok(Value::Option(Some(Box::new(m))))
                    }
                }
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("max_of expects array, found {}", other.ty()),
                )),
            }
        } else if name == "chunk" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "chunk expects array and size"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::Array(items), Value::Int(size)) => {
                    if size <= 0 {
                        return Err(
                            self.runtime_error(span, "chunk size must be greater than zero")
                        );
                    }
                    let sz = size as usize;
                    let chunks: Vec<Value> =
                        items.chunks(sz).map(|c| Value::Array(c.to_vec())).collect();
                    Ok(Value::Array(chunks))
                }
                _ => Err(self.runtime_error(span, "chunk expects array and Int size")),
            }
        } else if name == "flatten" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "flatten expects one array"));
            }
            match self.eval(&args[0], env)? {
                Value::Array(items) => {
                    let mut out = Vec::new();
                    for item in items {
                        match item {
                            Value::Array(sub) => out.extend(sub),
                            other => {
                                return Err(self.runtime_error(
                                    span,
                                    format!(
                                        "flatten expects array of arrays, found element of type {}",
                                        other.ty()
                                    ),
                                ))
                            }
                        }
                    }
                    Ok(Value::Array(out))
                }
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("flatten expects array, found {}", other.ty()),
                )),
            }
        } else if name == "set_union" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "set_union expects two Sets"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::Set(mut a), Value::Set(b)) => {
                    for item in b {
                        if !a.contains(&item) {
                            a.push(item);
                        }
                    }
                    Ok(Value::Set(a))
                }
                _ => Err(self.runtime_error(span, "set_union expects two Sets")),
            }
        } else if name == "set_intersection" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "set_intersection expects two Sets"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::Set(a), Value::Set(b)) => {
                    let mut out = Vec::new();
                    for item in a {
                        if b.contains(&item) {
                            out.push(item);
                        }
                    }
                    Ok(Value::Set(out))
                }
                _ => Err(self.runtime_error(span, "set_intersection expects two Sets")),
            }
        } else if name == "set_difference" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "set_difference expects two Sets"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::Set(a), Value::Set(b)) => {
                    let mut out = Vec::new();
                    for item in a {
                        if !b.contains(&item) {
                            out.push(item);
                        }
                    }
                    Ok(Value::Set(out))
                }
                _ => Err(self.runtime_error(span, "set_difference expects two Sets")),
            }
        } else if name == "map_remove" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "map_remove expects Map and key"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::Map(entries), key) => {
                    let filtered: Vec<(Value, Value)> =
                        entries.into_iter().filter(|(k, _)| k != &key).collect();
                    Ok(Value::Map(filtered))
                }
                _ => Err(self.runtime_error(span, "map_remove expects a Map")),
            }
        } else if name == "date_add_days" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "date_add_days expects Date and Int days"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::Date(date), Value::Int(days)) => {
                    let res = date_epoch_days(&date)
                        .and_then(|start| start.checked_add(days))
                        .and_then(epoch_days_date)
                        .map(Value::Date)
                        .ok_or_else(|| {
                            self.runtime_error(
                                span,
                                "date arithmetic is outside years 0001 through 9999",
                            )
                        })?;
                    Ok(res)
                }
                _ => Err(self.runtime_error(span, "date_add_days expects Date and Int")),
            }
        } else if name == "date_diff_days" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "date_diff_days expects two Dates"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::Date(start), Value::Date(end)) => {
                    let diff = date_epoch_days(&end)
                        .and_then(|end_days| {
                            date_epoch_days(&start)
                                .and_then(|start_days| start_days.checked_sub(end_days))
                        })
                        .map(Value::Int)
                        .ok_or_else(|| self.runtime_error(span, "date difference overflow"))?;
                    Ok(diff)
                }
                _ => Err(self.runtime_error(span, "date_diff_days expects two Dates")),
            }
        } else if name == "is_ok" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "is_ok expects one Result"));
            }
            match self.eval(&args[0], env)? {
                Value::Result(res) => Ok(Value::Bool(res.is_ok())),
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("is_ok expects Result, found {}", other.ty()),
                )),
            }
        } else if name == "is_err" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "is_err expects one Result"));
            }
            match self.eval(&args[0], env)? {
                Value::Result(res) => Ok(Value::Bool(res.is_err())),
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("is_err expects Result, found {}", other.ty()),
                )),
            }
        } else if name == "is_some" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "is_some expects one Option"));
            }
            match self.eval(&args[0], env)? {
                Value::Option(opt) => Ok(Value::Bool(opt.is_some())),
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("is_some expects Option, found {}", other.ty()),
                )),
            }
        } else if name == "is_none" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "is_none expects one Option"));
            }
            match self.eval(&args[0], env)? {
                Value::Option(opt) => Ok(Value::Bool(opt.is_none())),
                other => Err(self.runtime_error(
                    args[0].span,
                    format!("is_none expects Option, found {}", other.ty()),
                )),
            }
        } else if name == "is_before" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "is_before expects two Dates"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::Date(a), Value::Date(b)) => {
                    let ea = date_epoch_days(&a)
                        .ok_or_else(|| self.runtime_error(span, "invalid Date"))?;
                    let eb = date_epoch_days(&b)
                        .ok_or_else(|| self.runtime_error(span, "invalid Date"))?;
                    Ok(Value::Bool(ea < eb))
                }
                _ => Err(self.runtime_error(span, "is_before expects two Dates")),
            }
        } else if name == "is_after" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "is_after expects two Dates"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::Date(a), Value::Date(b)) => {
                    let ea = date_epoch_days(&a)
                        .ok_or_else(|| self.runtime_error(span, "invalid Date"))?;
                    let eb = date_epoch_days(&b)
                        .ok_or_else(|| self.runtime_error(span, "invalid Date"))?;
                    Ok(Value::Bool(ea > eb))
                }
                _ => Err(self.runtime_error(span, "is_after expects two Dates")),
            }
        } else if name == "hash_sha256" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "hash_sha256 expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(s) => {
                    let mut hasher = Sha256::new();
                    hasher.update(s.as_bytes());
                    let result = hasher.finalize();
                    Ok(Value::String(format!("{:x}", result)))
                }
                _ => Err(self.runtime_error(span, "hash_sha256 expects a String")),
            }
        } else if name == "base64_encode" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "base64_encode expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(s) => Ok(Value::String(base64_encode_bytes(s.as_bytes()))),
                _ => Err(self.runtime_error(span, "base64_encode expects a String")),
            }
        } else if name == "base64_decode" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "base64_decode expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(s) => match base64_decode_str(&s) {
                    Ok(decoded) => Ok(Value::Result(Ok(Box::new(Value::String(decoded))))),
                    Err(e) => Ok(Value::Result(Err(Box::new(Value::String(e))))),
                },
                _ => Err(self.runtime_error(span, "base64_decode expects a String")),
            }
        } else if name == "url_encode" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "url_encode expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(s) => Ok(Value::String(url_encode_str(&s))),
                _ => Err(self.runtime_error(span, "url_encode expects a String")),
            }
        } else if name == "url_decode" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "url_decode expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(s) => match url_decode_str(&s) {
                    Ok(decoded) => Ok(Value::Result(Ok(Box::new(Value::String(decoded))))),
                    Err(e) => Ok(Value::Result(Err(Box::new(Value::String(e))))),
                },
                _ => Err(self.runtime_error(span, "url_decode expects a String")),
            }
        } else if name == "uuid_v4" {
            if !args.is_empty() {
                return Err(self.runtime_error(span, "uuid_v4 expects no arguments"));
            }
            self.require_runtime_capability("Random", "random access", span)?;
            match generate_uuid_v4() {
                Ok(uuid) => Ok(Value::String(uuid)),
                Err(e) => Err(self.runtime_error(span, e)),
            }
        } else if name == "text_trim_start" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "text_trim_start expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(s) => Ok(Value::String(s.trim_start().to_string())),
                _ => Err(self.runtime_error(span, "text_trim_start expects a String")),
            }
        } else if name == "text_trim_end" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "text_trim_end expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(s) => Ok(Value::String(s.trim_end().to_string())),
                _ => Err(self.runtime_error(span, "text_trim_end expects a String")),
            }
        } else if name == "text_is_blank" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "text_is_blank expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(s) => Ok(Value::Bool(s.trim().is_empty())),
                _ => Err(self.runtime_error(span, "text_is_blank expects a String")),
            }
        } else if name == "text_substring" {
            if args.len() != 3 {
                return Err(
                    self.runtime_error(span, "text_substring expects String, start, and length")
                );
            }
            match (
                self.eval(&args[0], env)?,
                self.eval(&args[1], env)?,
                self.eval(&args[2], env)?,
            ) {
                (Value::String(s), Value::Int(start), Value::Int(length)) => {
                    let chars: Vec<char> = s.chars().collect();
                    let start = (start.max(0) as usize).min(chars.len());
                    let length = (length.max(0) as usize).min(chars.len() - start);
                    Ok(Value::String(chars[start..start + length].iter().collect()))
                }
                _ => Err(self.runtime_error(span, "text_substring expects String, Int, and Int")),
            }
        } else if name == "to_int" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "to_int expects one argument"));
            }
            match self.eval(&args[0], env)? {
                Value::Int(i) => Ok(Value::Int(i)),
                Value::UInt(u) => i64::try_from(u).map(Value::Int).map_err(|_| {
                    self.runtime_error(span, format!("to_int cannot convert `{u}` to Int"))
                }),
                Value::Float(f)
                    if f.is_finite() && f >= i64::MIN as f64 && f < -(i64::MIN as f64) =>
                {
                    Ok(Value::Int(f as i64))
                }
                Value::Float(_) => Err(self
                    .runtime_error(span, "to_int Float must be finite and within the Int range")),
                Value::Bool(b) => Ok(Value::Int(if b { 1 } else { 0 })),
                Value::String(s) => match s.trim().parse::<i64>() {
                    Ok(i) => Ok(Value::Int(i)),
                    Err(_) => {
                        Err(self.runtime_error(span, format!("to_int cannot convert `{s}` to Int")))
                    }
                },
                other => Err(self
                    .runtime_error(span, format!("to_int cannot convert {} to Int", other.ty()))),
            }
        } else if name == "to_float" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "to_float expects one argument"));
            }
            match self.eval(&args[0], env)? {
                Value::Float(f) if f.is_finite() => Ok(Value::Float(f)),
                Value::Float(_) => {
                    Err(self.runtime_error(span, "to_float requires a finite value"))
                }
                Value::Int(i) => Ok(Value::Float(i as f64)),
                Value::UInt(u) => Ok(Value::Float(u as f64)),
                Value::String(s) => match s.trim().parse::<f64>() {
                    Ok(f) if f.is_finite() => Ok(Value::Float(f)),
                    Err(_) => {
                        Err(self
                            .runtime_error(span, format!("to_float cannot convert `{s}` to Float")))
                    }
                    Ok(_) => Err(self.runtime_error(
                        span,
                        format!("to_float cannot convert `{s}` to a finite Float"),
                    )),
                },
                other => Err(self.runtime_error(
                    span,
                    format!("to_float cannot convert {} to Float", other.ty()),
                )),
            }
        } else if name == "parse_bool" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "parse_bool expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
                    "true" | "1" => Ok(Value::Result(Ok(Box::new(Value::Bool(true))))),
                    "false" | "0" => Ok(Value::Result(Ok(Box::new(Value::Bool(false))))),
                    _ => Ok(Value::Result(Err(Box::new(Value::String(format!(
                        "invalid boolean string: {s}"
                    )))))),
                },
                _ => Err(self.runtime_error(span, "parse_bool expects a String")),
            }
        } else if name == "panic" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "panic expects one String message"));
            }
            let msg = self.eval(&args[0], env)?;
            Err(self.runtime_error(span, format!("panic: {}", msg.output())))
        } else if name == "entries" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "entries expects a Map"));
            }
            match self.eval(&args[0], env)? {
                Value::Map(entries) => {
                    let pairs = entries
                        .into_iter()
                        .map(|(k, v)| Value::Tuple(vec![k, v]))
                        .collect();
                    Ok(Value::Array(pairs))
                }
                _ => Err(self.runtime_error(span, "entries expects a Map")),
            }
        } else if name == "zip" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "zip expects two Arrays"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::Array(a1), Value::Array(a2)) => {
                    let pairs = a1
                        .into_iter()
                        .zip(a2)
                        .map(|(x, y)| Value::Tuple(vec![x, y]))
                        .collect();
                    Ok(Value::Array(pairs))
                }
                _ => Err(self.runtime_error(span, "zip expects two Arrays")),
            }
        } else if name == "dedup" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "dedup expects an Array"));
            }
            match self.eval(&args[0], env)? {
                Value::Array(items) => {
                    let mut unique_items = Vec::new();
                    for item in items {
                        if !unique_items.contains(&item) {
                            unique_items.push(item);
                        }
                    }
                    Ok(Value::Array(unique_items))
                }
                _ => Err(self.runtime_error(span, "dedup expects an Array")),
            }
        } else if name == "path_join" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "path_join expects two String arguments"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::String(a), Value::String(b)) => {
                    let p = Path::new(&a).join(Path::new(&b));
                    Ok(Value::String(p.to_string_lossy().to_string()))
                }
                _ => Err(self.runtime_error(span, "path_join expects two Strings")),
            }
        } else if name == "path_filename" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "path_filename expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(s) => {
                    let p = Path::new(&s);
                    Ok(Value::String(
                        p.file_name()
                            .and_then(|f| f.to_str())
                            .unwrap_or("")
                            .to_string(),
                    ))
                }
                _ => Err(self.runtime_error(span, "path_filename expects a String")),
            }
        } else if name == "path_extension" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "path_extension expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(s) => {
                    let p = Path::new(&s);
                    Ok(Value::String(
                        p.extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap_or("")
                            .to_string(),
                    ))
                }
                _ => Err(self.runtime_error(span, "path_extension expects a String")),
            }
        } else if name == "path_dir" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "path_dir expects one String"));
            }
            match self.eval(&args[0], env)? {
                Value::String(s) => {
                    let p = Path::new(&s);
                    Ok(Value::String(
                        p.parent()
                            .and_then(|parent| parent.to_str())
                            .unwrap_or("")
                            .to_string(),
                    ))
                }
                _ => Err(self.runtime_error(span, "path_dir expects a String")),
            }
        } else if name == "file_exists" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "file_exists expects one String"));
            }
            self.require_runtime_capability("FileSystem", "file-system read access", span)?;
            match self.eval(&args[0], env)? {
                Value::String(path) => {
                    let roots = self
                        .filesystem_policy
                        .as_ref()
                        .map_or_else(Vec::new, |p| p.read_roots.clone());
                    match self.resolve_filesystem_path(&path, &roots, "read", span, true) {
                        Ok(resolved) => Ok(Value::Bool(resolved.is_file())),
                        Err(_) => Ok(Value::Bool(false)),
                    }
                }
                _ => Err(self.runtime_error(span, "file_exists expects a String")),
            }
        } else if name == "dir_exists" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "dir_exists expects one String"));
            }
            self.require_runtime_capability("FileSystem", "file-system read access", span)?;
            match self.eval(&args[0], env)? {
                Value::String(path) => {
                    let roots = self
                        .filesystem_policy
                        .as_ref()
                        .map_or_else(Vec::new, |p| p.read_roots.clone());
                    match self.resolve_filesystem_path(&path, &roots, "read", span, true) {
                        Ok(resolved) => Ok(Value::Bool(resolved.is_dir())),
                        Err(_) => Ok(Value::Bool(false)),
                    }
                }
                _ => Err(self.runtime_error(span, "dir_exists expects a String")),
            }
        } else if name == "file_size" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "file_size expects one String"));
            }
            self.require_runtime_capability("FileSystem", "file-system read access", span)?;
            match self.eval(&args[0], env)? {
                Value::String(path) => {
                    let roots = self
                        .filesystem_policy
                        .as_ref()
                        .map_or_else(Vec::new, |p| p.read_roots.clone());
                    let resolved =
                        self.resolve_filesystem_path(&path, &roots, "read", span, false)?;
                    match std::fs::metadata(&resolved) {
                        Ok(meta) => Ok(Value::Result(Ok(Box::new(Value::Int(meta.len() as i64))))),
                        Err(e) => Ok(Value::Result(Err(Box::new(Value::String(e.to_string()))))),
                    }
                }
                _ => Err(self.runtime_error(span, "file_size expects a String")),
            }
        } else if name == "map_merge" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "map_merge expects two Maps"));
            }
            match (self.eval(&args[0], env)?, self.eval(&args[1], env)?) {
                (Value::Map(mut a), Value::Map(b)) => {
                    for (bk, bv) in b {
                        if let Some(pos) = a.iter().position(|(k, _)| k == &bk) {
                            a[pos] = (bk, bv);
                        } else {
                            a.push((bk, bv));
                        }
                    }
                    Ok(Value::Map(a))
                }
                _ => Err(self.runtime_error(span, "map_merge expects two Maps")),
            }
        } else if name == "unwrap_or" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "unwrap_or expects Option and default"));
            }
            let opt_val = self.eval(&args[0], env)?;
            let default_val = self.eval(&args[1], env)?;
            match opt_val {
                Value::Option(Some(inner)) => Ok(*inner),
                Value::Option(None) => Ok(default_val),
                _ => Err(self.runtime_error(args[0].span, "unwrap_or expects an Option")),
            }
        } else if name == "text_replace_result" {
            if args.len() != 3 {
                return Err(self.runtime_error(
                    span,
                    "text_replace_result expects text, search, and replacement",
                ));
            }
            let text = self.eval(&args[0], env)?;
            let search = self.eval(&args[1], env)?;
            let replacement = self.eval(&args[2], env)?;
            let outcome = match (text, search, replacement) {
                (Value::String(text), Value::String(search), Value::String(replacement))
                    if !search.is_empty() =>
                {
                    Ok(text.replace(&search, &replacement))
                }
                (Value::String(_), Value::String(_), Value::String(_)) => {
                    Err("text_replace_result requires a non-empty search text".to_string())
                }
                _ => Err("text_replace_result expects String arguments".to_string()),
            };
            Ok(Value::Result(
                outcome
                    .map(|value| Box::new(Value::String(value)))
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        } else if name == "text_split_result" {
            if args.len() != 2 {
                return Err(
                    self.runtime_error(span, "text_split_result expects text and separator")
                );
            }
            let text = self.eval(&args[0], env)?;
            let separator = self.eval(&args[1], env)?;
            let outcome = match (text, separator) {
                (Value::String(text), Value::String(separator)) if !separator.is_empty() => {
                    Ok(text
                        .split(&separator)
                        .map(|part| Value::String(part.to_string()))
                        .collect())
                }
                (Value::String(_), Value::String(_)) => {
                    Err("text_split_result requires a non-empty separator".to_string())
                }
                _ => Err("text_split_result expects String arguments".to_string()),
            };
            Ok(Value::Result(
                outcome
                    .map(|values| Box::new(Value::Array(values)))
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        } else if name == "get" {
            if args.len() != 2 {
                return Err(self.runtime_error(span, "`get` expects a Map and one key"));
            }
            let map = self.eval(&args[0], env)?;
            let key = self.eval(&args[1], env)?;
            match map {
                Value::Map(entries) => Ok(Value::Option(
                    entries
                        .iter()
                        .find(|(existing, _)| existing == &key)
                        .map(|(_, value)| Box::new(value.clone())),
                )),
                value => {
                    Err(self
                        .runtime_error(span, format!("`get` expects a Map, found {}", value.ty())))
                }
            }
        } else if name == "put" {
            if args.len() != 3 {
                return Err(self.runtime_error(span, "`put` expects a Map, a key, and a value"));
            }
            let map = self.eval(&args[0], env)?;
            let key = self.eval(&args[1], env)?;
            let value = self.eval(&args[2], env)?;
            match map {
                Value::Map(mut entries) => {
                    if let Some((_, existing_value)) =
                        entries.iter_mut().find(|(existing, _)| existing == &key)
                    {
                        *existing_value = value;
                    } else {
                        entries.push((key, value));
                    }
                    Ok(Value::Map(entries))
                }
                value => {
                    Err(self
                        .runtime_error(span, format!("`put` expects a Map, found {}", value.ty())))
                }
            }
        } else if name == "keys" || name == "values" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, format!("`{name}` expects exactly one Map")));
            }
            match self.eval(&args[0], env)? {
                Value::Map(entries) if name == "keys" => Ok(Value::Array(
                    entries.into_iter().map(|(key, _)| key).collect(),
                )),
                Value::Map(entries) => Ok(Value::Array(
                    entries.into_iter().map(|(_, value)| value).collect(),
                )),
                value => Err(self.runtime_error(
                    span,
                    format!("`{name}` expects a Map, found {}", value.ty()),
                )),
            }
        } else if name == "first" || name == "last" {
            if args.len() != 1 {
                return Err(
                    self.runtime_error(span, format!("`{name}` expects exactly one argument"))
                );
            }
            match self.eval(&args[0], env)? {
                Value::Array(values) => {
                    let value = if name == "first" {
                        values.first()
                    } else {
                        values.last()
                    };
                    Ok(Value::Option(value.cloned().map(Box::new)))
                }
                value => Err(self.runtime_error(
                    span,
                    format!("`{name}` expects an array, found {}", value.ty()),
                )),
            }
        } else if name == "Some" || name == "Ok" || name == "Err" {
            let argument = args.first().ok_or_else(|| {
                self.runtime_error(span, format!("`{name}` expects one argument"))
            })?;
            if args.len() != 1 {
                return Err(
                    self.runtime_error(span, format!("`{name}` expects exactly one argument"))
                );
            }
            let value = Box::new(self.eval(argument, env)?);
            match name {
                "Some" => Ok(Value::Option(Some(value))),
                "Ok" => Ok(Value::Result(Ok(value))),
                _ => Ok(Value::Result(Err(value))),
            }
        } else if matches!(name, "decimal" | "date" | "time" | "duration_ms" | "bytes") {
            self.eval_language_value_builtin(name, args, env, span)
        } else if name == "now" {
            if !args.is_empty() {
                return Err(self.runtime_error(span, "now expects no arguments"));
            }
            self.require_runtime_capability("Clock", "clock access", span)?;
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| {
                    self.runtime_error(
                        span,
                        format!("clock access failed: system time is before Unix epoch: {error}"),
                    )
                })?
                .as_millis();
            let timestamp = i64::try_from(timestamp)
                .map_err(|_| self.runtime_error(span, "clock access failed: timestamp overflow"))?;
            Ok(Value::Timestamp(timestamp))
        } else if name == "format_timestamp" {
            if args.len() != 1 {
                return Err(
                    self.runtime_error(span, "format_timestamp expects exactly one Timestamp")
                );
            }
            match self.eval(&args[0], env)? {
                Value::Timestamp(timestamp)
                    if zelyra_ast::temporal::valid_timestamp_millis(timestamp) =>
                {
                    Ok(Value::String(format_timestamp_utc(timestamp)))
                }
                Value::Timestamp(_) => {
                    Err(self.runtime_error(span, "Timestamp is outside years 0001 through 9999"))
                }
                value => Err(self.runtime_error(
                    args[0].span,
                    format!("format_timestamp expects a Timestamp, found {}", value.ty()),
                )),
            }
        } else if name == "format_text" {
            if args.len() != 1 {
                return Err(self.runtime_error(
                    span,
                    "format_text expects one scalar value and no type arguments",
                ));
            }
            let value = self.eval(&args[0], env)?;
            if matches!(
                value,
                Value::Int(_)
                    | Value::UInt(_)
                    | Value::Float(_)
                    | Value::Decimal(_)
                    | Value::Bool(_)
                    | Value::Char(_)
                    | Value::String(_)
                    | Value::Timestamp(_)
                    | Value::Date(_)
                    | Value::Time(_)
                    | Value::Duration(_)
            ) {
                Ok(Value::String(value.output()))
            } else {
                Err(self.runtime_error(
                    args[0].span,
                    format!("format_text does not support `{}`", value.ty()),
                ))
            }
        } else if name == "env" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "env expects exactly one argument"));
            }
            self.require_runtime_capability("Environment", "environment access", span)?;
            let key = self.eval(&args[0], env)?;
            let Value::String(key) = key else {
                return Err(self.runtime_error(args[0].span, "env expects a String variable name"));
            };
            if key.is_empty() {
                return Err(
                    self.runtime_error(args[0].span, "env expects a non-empty variable name")
                );
            }
            let value = std::env::var_os(&key)
                .map(|value| {
                    value.into_string().map_err(|_| {
                        self.runtime_error(
                            args[0].span,
                            format!("environment variable {key} is not valid UTF-8"),
                        )
                    })
                })
                .transpose()?;
            Ok(Value::Option(
                value.map(|value| Box::new(Value::String(value))),
            ))
        } else if name == "random_int" {
            if args.len() != 2 {
                return Err(self
                    .runtime_error(span, "random_int expects minimum and maximum Int arguments"));
            }
            self.require_runtime_capability("Random", "random access", span)?;
            let minimum = self.eval(&args[0], env)?;
            let maximum = self.eval(&args[1], env)?;
            let (Value::Int(minimum), Value::Int(maximum)) = (minimum, maximum) else {
                return Err(self.runtime_error(span, "random_int expects Int arguments"));
            };
            if minimum > maximum {
                return Err(self.runtime_error(span, "random_int requires minimum <= maximum"));
            }
            let range = (i128::from(maximum) - i128::from(minimum) + 1) as u128;
            let limit = u128::from(u64::MAX) + 1;
            let cutoff = limit - (limit % range);
            let mut random = OsRng;
            loop {
                let mut bytes = [0_u8; 8];
                random.try_fill_bytes(&mut bytes).map_err(|error| {
                    self.runtime_error(span, format!("random access failed: {error}"))
                })?;
                let candidate = u128::from(u64::from_ne_bytes(bytes));
                if candidate < cutoff {
                    let value = i128::from(minimum) + (candidate % range) as i128;
                    return Ok(Value::Int(value as i64));
                }
            }
        } else if name == "http_get" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "http_get expects exactly one String URL"));
            }
            self.require_runtime_capability("Network", "network access", span)?;
            let url = self.eval(&args[0], env)?;
            let Value::String(url) = url else {
                return Err(self.runtime_error(args[0].span, "http_get expects a String URL"));
            };
            self.http_get(&url, span).map(Value::String)
        } else if name == "json_encode" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "json_encode expects exactly one value"));
            }
            let value = self.eval(&args[0], env)?;
            self.json_encode(&value, span).map(Value::String)
        } else if name == "json_encode_tagged_result" {
            if !type_args.is_empty() || args.len() != 1 {
                return Err(self.runtime_error(
                    span,
                    "json_encode_tagged_result expects one Result value and no type arguments",
                ));
            }
            let value = self.eval(&args[0], env)?;
            self.json_encode_tagged_result(&value, span)
                .map(Value::String)
        } else if name == "sql_decode" {
            self.eval_sql_decode(type_args, args, env, span)
        } else if name == "json_decode" {
            if type_args.len() != 1 || args.len() != 1 {
                return Err(self.runtime_error(
                    span,
                    "json_decode expects one type argument and one String value",
                ));
            }
            let value = self.eval(&args[0], env)?;
            let Value::String(source) = value else {
                return Err(self.runtime_error(args[0].span, "json_decode expects a String value"));
            };
            self.json_decode(&source, &type_args[0], span)
        } else if name == "json_decode_result" {
            if type_args.len() != 1 || args.len() != 1 {
                return Err(self.runtime_error(
                    span,
                    "json_decode_result expects one type argument and one String value",
                ));
            }
            let value = self.eval(&args[0], env)?;
            let Value::String(source) = value else {
                return Err(
                    self.runtime_error(args[0].span, "json_decode_result expects a String value")
                );
            };
            Ok(Value::Result(
                match self.json_decode(&source, &type_args[0], span) {
                    Ok(value) => Ok(Box::new(value)),
                    Err(error) => Err(Box::new(Value::String(error.message))),
                },
            ))
        } else if name == "json_decode_tagged_result" {
            if type_args.len() != 2 || args.len() != 1 {
                return Err(self.runtime_error(
                    span,
                    "json_decode_tagged_result expects Ok and Err type arguments and one String value",
                ));
            }
            let value = self.eval(&args[0], env)?;
            let Value::String(source) = value else {
                return Err(self.runtime_error(
                    args[0].span,
                    "json_decode_tagged_result expects a String value",
                ));
            };
            let decoded =
                self.json_decode_tagged_result(&source, &type_args[0], &type_args[1], span);
            Ok(Value::Result(
                decoded
                    .map(Box::new)
                    .map_err(|error| Box::new(Value::String(error))),
            ))
        } else if name == "csv_decode_result" {
            if !type_args.is_empty() || args.len() != 2 {
                return Err(self.runtime_error(span, "csv_decode_result expects a String and Char"));
            }
            let source = self.eval(&args[0], env)?;
            let delimiter = self.eval(&args[1], env)?;
            let (Value::String(source), Value::Char(delimiter)) = (source, delimiter) else {
                return Err(self.runtime_error(span, "csv_decode_result expects a String and Char"));
            };
            let decoded = csv::decode(&source, delimiter).map(|rows| {
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
        } else if name == "csv_encode" {
            if !type_args.is_empty() || args.len() != 2 {
                return Err(
                    self.runtime_error(span, "csv_encode expects String[][] rows and a Char")
                );
            }
            let rows = self.eval(&args[0], env)?;
            let delimiter = self.eval(&args[1], env)?;
            let (Value::Array(rows), Value::Char(delimiter)) = (rows, delimiter) else {
                return Err(
                    self.runtime_error(span, "csv_encode expects String[][] rows and a Char")
                );
            };
            let rows = rows
                .into_iter()
                .map(|row| match row {
                    Value::Array(fields) => fields
                        .into_iter()
                        .map(|field| match field {
                            Value::String(value) => Ok(value),
                            _ => Err("csv_encode expects String[][] rows".to_string()),
                        })
                        .collect::<Result<Vec<_>, _>>(),
                    _ => Err("csv_encode expects String[][] rows".to_string()),
                })
                .collect::<Result<Vec<_>, _>>();
            let encoded = rows
                .and_then(|rows| csv::encode(&rows, delimiter))
                .map(Value::String);
            Ok(Value::Result(
                encoded
                    .map(Box::new)
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        } else if name == "decimal_scaled_result" {
            if !type_args.is_empty() || args.len() != 2 {
                return Err(self
                    .runtime_error(span, "decimal_scaled_result expects a String and scale Int"));
            }
            let source = self.eval(&args[0], env)?;
            let scale = self.eval(&args[1], env)?;
            let (Value::String(source), Value::Int(scale)) = (source, scale) else {
                return Err(self
                    .runtime_error(span, "decimal_scaled_result expects a String and scale Int"));
            };
            Ok(Value::Result(
                csv::decimal_to_scaled(&source, scale)
                    .map(|value| Box::new(Value::Int(value)))
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        } else if name == "format_scaled_result" {
            if !type_args.is_empty() || args.len() != 2 {
                return Err(
                    self.runtime_error(span, "format_scaled_result expects an Int and scale Int")
                );
            }
            let value = self.eval(&args[0], env)?;
            let scale = self.eval(&args[1], env)?;
            let (Value::Int(value), Value::Int(scale)) = (value, scale) else {
                return Err(self.runtime_error(span, "format_scaled_result expects Int arguments"));
            };
            Ok(Value::Result(
                csv::format_scaled(value, scale)
                    .map(|value| Box::new(Value::String(value)))
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        } else if matches!(name, "date_add_days_result" | "date_days_between_result") {
            if !type_args.is_empty() || args.len() != 2 {
                return Err(self.runtime_error(
                    span,
                    format!("{name} expects two arguments and no type arguments"),
                ));
            }
            let left = self.eval(&args[0], env)?;
            let right = self.eval(&args[1], env)?;
            let result = if name == "date_add_days_result" {
                match (left, right) {
                    (Value::Date(date), Value::Int(days)) => date_epoch_days(&date)
                        .and_then(|start| start.checked_add(days))
                        .and_then(epoch_days_date)
                        .map(Value::Date)
                        .ok_or_else(|| {
                            "date arithmetic is outside years 0001 through 9999".to_owned()
                        }),
                    _ => Err("date_add_days_result expects Date and Int".to_owned()),
                }
            } else {
                match (left, right) {
                    (Value::Date(start), Value::Date(end)) => date_epoch_days(&end)
                        .and_then(|end| {
                            date_epoch_days(&start).and_then(|start| end.checked_sub(start))
                        })
                        .map(Value::Int)
                        .ok_or_else(|| "date difference overflow".to_owned()),
                    _ => Err("date_days_between_result expects two Dates".to_owned()),
                }
            };
            Ok(Value::Result(
                result
                    .map(Box::new)
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        } else if matches!(name, "timestamp_add_result" | "timestamp_difference_result") {
            if !type_args.is_empty() || args.len() != 2 {
                return Err(self.runtime_error(
                    span,
                    format!("{name} expects two arguments and no type arguments"),
                ));
            }
            let left = self.eval(&args[0], env)?;
            let right = self.eval(&args[1], env)?;
            let result = if name == "timestamp_add_result" {
                match (left, right) {
                    (Value::Timestamp(timestamp), Value::Duration(duration)) => timestamp
                        .checked_add(duration)
                        .filter(|value| {
                            timestamp_bounds_millis()
                                .is_some_and(|(min, max)| (min..=max).contains(value))
                        })
                        .map(Value::Timestamp)
                        .ok_or_else(|| {
                            "timestamp arithmetic is outside years 0001 through 9999".to_owned()
                        }),
                    _ => Err("timestamp_add_result expects Timestamp and Duration".to_owned()),
                }
            } else {
                match (left, right) {
                    (Value::Timestamp(end), Value::Timestamp(start)) => end
                        .checked_sub(start)
                        .map(Value::Duration)
                        .ok_or_else(|| "timestamp difference overflow".to_owned()),
                    _ => Err("timestamp_difference_result expects two Timestamps".to_owned()),
                }
            };
            Ok(Value::Result(
                result
                    .map(Box::new)
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        } else if name == "decimal_quantize_result" {
            if !type_args.is_empty() || args.len() != 3 {
                return Err(self.runtime_error(
                    span,
                    "decimal_quantize_result expects Decimal, scale Int, and mode String",
                ));
            }
            let value = self.eval(&args[0], env)?;
            let scale = self.eval(&args[1], env)?;
            let mode = self.eval(&args[2], env)?;
            let result = match (value, scale, mode) {
                (Value::Decimal(value), Value::Int(scale), Value::String(mode)) => {
                    u32::try_from(scale)
                        .map_err(|_| "decimal scale must be between 0 and 9".to_owned())
                        .and_then(|scale| quantize_decimal(value, scale, &mode))
                        .map(Value::Decimal)
                }
                _ => Err("decimal_quantize_result expects Decimal, Int, and String".to_owned()),
            };
            Ok(Value::Result(
                result
                    .map(Box::new)
                    .map_err(|message| Box::new(Value::String(message))),
            ))
        } else if name == "http_json" {
            if type_args.len() != 2 || args.len() != 4 {
                return Err(self.runtime_error(
                    span,
                    "http_json expects request and response type arguments plus four values",
                ));
            }
            self.require_runtime_capability("Network", "network access", span)?;
            let method = self.eval(&args[0], env)?;
            let url = self.eval(&args[1], env)?;
            let headers = self.eval(&args[2], env)?;
            let body = self.eval(&args[3], env)?;
            let Value::String(method) = method else {
                return Err(self.runtime_error(args[0].span, "http_json expects a String method"));
            };
            let Value::String(url) = url else {
                return Err(self.runtime_error(args[1].span, "http_json expects a String URL"));
            };
            let Value::Array(headers) = headers else {
                return Err(self.runtime_error(args[2].span, "http_json expects String[] headers"));
            };
            let mut header_strings = Vec::with_capacity(headers.len() + 1);
            let mut has_content_type = false;
            for header in headers {
                let Value::String(header) = header else {
                    return Err(
                        self.runtime_error(args[2].span, "http_json expects String[] headers")
                    );
                };
                if header
                    .split_once(':')
                    .is_some_and(|(name, _)| name.trim().eq_ignore_ascii_case("content-type"))
                {
                    has_content_type = true;
                }
                header_strings.push(header);
            }
            if !has_content_type {
                header_strings.push("Content-Type: application/json".into());
            }
            let body = match body {
                Value::Option(Some(body)) => Some(body),
                Value::Option(None) => None,
                _ => {
                    return Err(
                        self.runtime_error(args[3].span, "http_json expects a Request? body")
                    );
                }
            };
            self.http_json(
                &method,
                &url,
                &header_strings,
                body.as_deref(),
                &type_args[1],
                span,
            )
        } else if name == "http_result" {
            if type_args.len() != 2 || args.len() != 4 {
                return Err(self.runtime_error(
                    span,
                    "http_result expects request and response type arguments plus four values",
                ));
            }
            self.require_runtime_capability("Network", "network access", span)?;
            let method = self.eval(&args[0], env)?;
            let url = self.eval(&args[1], env)?;
            let headers = self.eval(&args[2], env)?;
            let body = self.eval(&args[3], env)?;
            let Value::String(method) = method else {
                return Err(self.runtime_error(args[0].span, "http_result expects a String method"));
            };
            let Value::String(url) = url else {
                return Err(self.runtime_error(args[1].span, "http_result expects a String URL"));
            };
            let Value::Array(headers) = headers else {
                return Err(
                    self.runtime_error(args[2].span, "http_result expects String[] headers")
                );
            };
            let mut header_strings = Vec::with_capacity(headers.len() + 1);
            let mut has_content_type = false;
            for header in headers {
                let Value::String(header) = header else {
                    return Err(
                        self.runtime_error(args[2].span, "http_result expects String[] headers")
                    );
                };
                if header
                    .split_once(':')
                    .is_some_and(|(name, _)| name.trim().eq_ignore_ascii_case("content-type"))
                {
                    has_content_type = true;
                }
                header_strings.push(header);
            }
            if !has_content_type {
                header_strings.push("Content-Type: application/json".into());
            }
            let body = match body {
                Value::Option(Some(body)) => Some(body),
                Value::Option(None) => None,
                _ => {
                    return Err(
                        self.runtime_error(args[3].span, "http_result expects a Request? body")
                    );
                }
            };
            self.http_result(
                &method,
                &url,
                &header_strings,
                body.as_deref(),
                &type_args[1],
                span,
            )
        } else if name == "http_request" {
            if args.len() != 4 {
                return Err(self.runtime_error(
                            span,
                            "http_request expects String method, String URL, String[] headers, and String? body",
                        ));
            }
            self.require_runtime_capability("Network", "network access", span)?;
            let method = self.eval(&args[0], env)?;
            let url = self.eval(&args[1], env)?;
            let headers = self.eval(&args[2], env)?;
            let body = self.eval(&args[3], env)?;
            let Value::String(method) = method else {
                return Err(
                    self.runtime_error(args[0].span, "http_request expects a String method")
                );
            };
            let Value::String(url) = url else {
                return Err(self.runtime_error(args[1].span, "http_request expects a String URL"));
            };
            let Value::Array(headers) = headers else {
                return Err(
                    self.runtime_error(args[2].span, "http_request expects String[] headers")
                );
            };
            let mut header_strings = Vec::with_capacity(headers.len());
            for header in headers {
                let Value::String(header) = header else {
                    return Err(
                        self.runtime_error(args[2].span, "http_request expects String[] headers")
                    );
                };
                header_strings.push(header);
            }
            let body = match body {
                Value::Option(Some(body)) => match *body {
                    Value::String(body) => Some(body),
                    _ => {
                        return Err(
                            self.runtime_error(args[3].span, "http_request expects a String? body")
                        );
                    }
                },
                Value::Option(None) => None,
                _ => {
                    return Err(
                        self.runtime_error(args[3].span, "http_request expects a String? body")
                    );
                }
            };
            self.http_request(&method, &url, &header_strings, body.as_deref(), span)
        } else if name == "mail_queue" {
            Err(self.runtime_error(span, "mail_queue is only allowed inside a transaction"))
        } else if name == "mail_deliver" {
            if args.len() != 2 {
                return Err(self.runtime_error(
                    span,
                    "mail_deliver expects an outbox event ID and payload JSON",
                ));
            }
            self.require_runtime_capability("Email", "email delivery", span)?;
            let event_id = self.eval(&args[0], env)?;
            let payload_json = self.eval(&args[1], env)?;
            let (Value::String(event_id), Value::String(payload_json)) = (event_id, payload_json)
            else {
                return Err(self.runtime_error(span, "mail_deliver expects String arguments"));
            };
            let result = mail::deliver_outbox_message(
                &event_id,
                &payload_json,
                self.mail_policy.as_ref(),
                self.network_policy.as_ref(),
            )
            .map(|()| Box::new(Value::Unit))
            .map_err(|error| Box::new(Value::String(error.to_owned())));
            Ok(Value::Result(result))
        } else if name == "run_process" {
            if args.len() != 2 {
                return Err(self.runtime_error(
                    span,
                    "run_process expects a String command and String[] arguments",
                ));
            }
            self.require_runtime_capability("Process", "process execution", span)?;
            let command = self.eval(&args[0], env)?;
            let arguments = self.eval(&args[1], env)?;
            let Value::String(command) = command else {
                return Err(
                    self.runtime_error(args[0].span, "run_process expects a String command")
                );
            };
            let Value::Array(arguments) = arguments else {
                return Err(
                    self.runtime_error(args[1].span, "run_process expects String[] arguments")
                );
            };
            let mut argument_strings = Vec::with_capacity(arguments.len());
            for argument in arguments {
                let Value::String(argument) = argument else {
                    return Err(
                        self.runtime_error(args[1].span, "run_process expects String[] arguments")
                    );
                };
                argument_strings.push(argument);
            }
            self.run_process(&command, &argument_strings, span)
                .map(Value::String)
        } else if name == "read_text" {
            if args.len() != 1 {
                return Err(self.runtime_error(span, "read_text expects exactly one String path"));
            }
            self.require_runtime_capability("FileSystem", "file-system read access", span)?;
            let path = self.eval(&args[0], env)?;
            let Value::String(path) = path else {
                return Err(self.runtime_error(args[0].span, "read_text expects a String path"));
            };
            if path.is_empty() {
                return Err(self.runtime_error(args[0].span, "read_text expects a non-empty path"));
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
            std::fs::read_to_string(&path)
                .map_err(|error| {
                    self.runtime_error(span, format!("file-system read failed: {error}"))
                })
                .map(Value::String)
        } else if name == "read_text_result" {
            if args.len() != 1 {
                return Err(
                    self.runtime_error(span, "read_text_result expects exactly one String path")
                );
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
                        Ok(path) => std::fs::read_to_string(path)
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
                return Err(
                    self.runtime_error(span, "write_text expects a String path and String content")
                );
            }
            self.require_runtime_capability("FileSystem", "file-system write access", span)?;
            let path = self.eval(&args[0], env)?;
            let content = self.eval(&args[1], env)?;
            let (Value::String(path), Value::String(content)) = (path, content) else {
                return Err(self.runtime_error(span, "write_text expects String arguments"));
            };
            if path.is_empty() {
                return Err(self.runtime_error(args[0].span, "write_text expects a non-empty path"));
            }
            let roots = self
                .filesystem_policy
                .as_ref()
                .map_or_else(Vec::new, |policy| policy.write_roots.clone());
            let path = self.resolve_filesystem_path(&path, &roots, "write", span, true)?;
            std::fs::write(&path, content).map_err(|error| {
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
                        Ok(path) => std::fs::write(path, content)
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
                                filesystem::write_text_new(&target, &content)
                            }
                            "atomic_write_text_result" => {
                                filesystem::atomic_write_text(&target, &content)
                            }
                            _ => filesystem::append_text(&target, &content),
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
                                filesystem::copy_file_new(&source_path, &destination_path)
                            } else {
                                filesystem::rename_file_new(&source_path, &destination_path)
                            };
                            operation.map_err(|error| format!("file-system {name} failed: {error}"))
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
                            std::fs::remove_file(path).map_err(|error| {
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
                return Err(self.runtime_error(args[0].span, "list_dir expects a non-empty path"));
            }
            let roots = self
                .filesystem_policy
                .as_ref()
                .map_or_else(Vec::new, |policy| policy.read_roots.clone());
            let path = self.resolve_filesystem_path(&path, &roots, "list", span, false)?;
            let mut entries = std::fs::read_dir(&path)
                .map_err(|error| {
                    self.runtime_error(span, format!("file-system list failed: {error}"))
                })?
                .map(|entry| {
                    entry
                        .map(|entry| {
                            Value::String(entry.file_name().to_string_lossy().into_owned())
                        })
                        .map_err(|error| {
                            self.runtime_error(span, format!("file-system list failed: {error}"))
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            entries.sort_by_key(Value::output);
            Ok(Value::Array(entries))
        } else {
            Err(self.runtime_error(span, format!("unknown function `{name}`")))
        }
    }
    fn eval(&mut self, expr: &Expr, env: &mut Environment) -> Result<Value, RuntimeError> {
        self.consume_step(expr.span)?;
        match &expr.kind {
            ExprKind::Int(v) => Ok(Value::Int(*v)),
            ExprKind::UInt(v) => Ok(Value::UInt(*v)),
            ExprKind::Float(v) => Ok(Value::Float(*v)),
            ExprKind::Bool(v) => Ok(Value::Bool(*v)),
            ExprKind::String(v) => Ok(Value::String(v.clone())),
            ExprKind::Char(v) => Ok(Value::Char(*v)),
            ExprKind::FunctionValue(name) => {
                let function = self.functions.get(name).copied().ok_or_else(|| {
                    self.runtime_error(expr.span, format!("unknown function value `{name}`"))
                })?;
                if !function.type_params.is_empty() {
                    return Err(self.runtime_error(
                        expr.span,
                        format!(
                            "generic function `{name}` cannot be stored as a function value yet"
                        ),
                    ));
                }
                Ok(Value::Function(Box::new(FunctionValue {
                    name: name.clone(),
                    params: function
                        .params
                        .iter()
                        .map(|param| param.ty.clone())
                        .collect(),
                    return_type: function.return_type.clone().unwrap_or(Type::Unit),
                    capabilities: function.capabilities.clone(),
                    lambda_params: Vec::new(),
                    lambda_body: None,
                    captures: Vec::new(),
                })))
            }
            ExprKind::Lambda { params, body } => Ok(Value::Function(Box::new(FunctionValue {
                name: "<lambda>".into(),
                params: params
                    .iter()
                    .map(|parameter| parameter.ty.clone())
                    .collect(),
                return_type: Type::Unknown,
                capabilities: Vec::new(),
                lambda_params: params
                    .iter()
                    .map(|parameter| parameter.name.clone())
                    .collect(),
                lambda_body: Some(body.clone()),
                captures: env.visible_values(),
            }))),
            ExprKind::Array(values) => values
                .iter()
                .map(|value| self.eval(value, env))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            ExprKind::Map(entries) => {
                let mut values = Vec::with_capacity(entries.len());
                for (key, value) in entries {
                    let key = self.eval(key, env)?;
                    let value = self.eval(value, env)?;
                    if let Some((_, existing_value)) =
                        values.iter_mut().find(|(existing, _)| existing == &key)
                    {
                        *existing_value = value;
                    } else {
                        values.push((key, value));
                    }
                }
                Ok(Value::Map(values))
            }
            ExprKind::Record {
                type_name,
                type_args,
                fields,
            } => self.eval_record_value(type_name, type_args, fields, expr.span, env),
            ExprKind::Variable(name) => {
                if let Some(value) = env.get(name) {
                    return Ok(value);
                }
                if name == "None" {
                    return Ok(Value::Option(None));
                }
                if let Some((type_name, variant)) = name.rsplit_once("::") {
                    if let Some(definition) = self.enum_definitions.get(type_name) {
                        let payload_types = definition
                            .variant_payloads
                            .iter()
                            .find(|(candidate, _)| candidate == variant)
                            .map(|(_, types)| types.as_slice())
                            .ok_or_else(|| {
                                self.runtime_error(
                                    expr.span,
                                    format!("unknown enum variant `{type_name}::{variant}`"),
                                )
                            })?;
                        if !payload_types.is_empty() {
                            return Err(self.runtime_error(
                                expr.span,
                                format!(
                                    "enum variant `{type_name}::{variant}` requires payload values"
                                ),
                            ));
                        }
                        return Ok(Value::Enum {
                            type_name: type_name.to_owned(),
                            type_args: Vec::new(),
                            variant: variant.to_owned(),
                            payloads: Vec::new(),
                        });
                    }
                }
                Err(self.runtime_error(expr.span, format!("unknown variable `{name}`")))
            }
            ExprKind::Call {
                name,
                type_args,
                args,
            } => self.eval_call_expr(name, type_args, args, env, expr.span),
            ExprKind::Unary { op, expr: inner } => {
                let value = self.eval(inner, env)?;
                match (op, value) {
                    (UnaryOp::Negate, Value::Int(v)) => v
                        .checked_neg()
                        .map(Value::Int)
                        .ok_or_else(|| self.runtime_error(expr.span, "Int overflow")),
                    (UnaryOp::Negate, Value::UInt(_)) => {
                        Err(self.runtime_error(expr.span, "cannot negate UInt"))
                    }
                    (UnaryOp::Negate, Value::Float(v)) => Ok(Value::Float(-v)),
                    (UnaryOp::Negate, Value::Decimal(v)) => v
                        .checked_neg()
                        .map(Value::Decimal)
                        .ok_or_else(|| self.runtime_error(expr.span, "Decimal overflow")),
                    (UnaryOp::Not, Value::Bool(v)) => Ok(Value::Bool(!v)),
                    (_, value) => Err(self.runtime_error(
                        expr.span,
                        format!("invalid unary operand of type {}", value.ty()),
                    )),
                }
            }
            ExprKind::Index { target, index } => {
                let target = self.eval(target, env)?;
                let index = self.eval(index, env)?;
                let index = match index {
                    Value::Int(value) if value >= 0 => value as usize,
                    Value::UInt(value) => value as usize,
                    value => {
                        return Err(self.runtime_error(
                            expr.span,
                            format!("array index must be Int or UInt, found {}", value.ty()),
                        ));
                    }
                };
                match target {
                    Value::Array(values) => values.get(index).cloned().ok_or_else(|| {
                        self.runtime_error(
                            expr.span,
                            format!("array index {index} is out of bounds"),
                        )
                    }),
                    Value::Tuple(values) => values.get(index).cloned().ok_or_else(|| {
                        self.runtime_error(
                            expr.span,
                            format!("tuple index {index} is out of bounds"),
                        )
                    }),
                    value => Err(self.runtime_error(
                        expr.span,
                        format!("indexing requires an array or Tuple, found {}", value.ty()),
                    )),
                }
            }
            ExprKind::Field { target, field } => {
                let value = self.eval(target, env)?;
                match value {
                    Value::Object { fields, .. } => fields.get(field).cloned().ok_or_else(|| {
                        self.runtime_error(expr.span, format!("record field `{field}` is missing"))
                    }),
                    Value::ClassRef { reference, .. } => {
                        let object = reference.object.lock().map_err(|_| {
                            self.runtime_error(expr.span, "class object is unavailable")
                        })?;
                        object.fields.get(field).cloned().ok_or_else(|| {
                            self.runtime_error(
                                expr.span,
                                format!("class field `{field}` is missing"),
                            )
                        })
                    }
                    value => Err(self.runtime_error(
                        expr.span,
                        format!(
                            "field access requires a record or class, found {}",
                            value.ty()
                        ),
                    )),
                }
            }
            ExprKind::Binary { .. } => {
                let mut cursor = expr;
                let mut operations = Vec::new();
                while let ExprKind::Binary { left, op, right } = &cursor.kind {
                    operations.push((*op, right.as_ref(), cursor.span));
                    cursor = left;
                }
                let mut value = self.eval(cursor, env)?;
                for (op, right, span) in operations.into_iter().rev() {
                    if op == BinaryOp::And && matches!(&value, Value::Bool(false)) {
                        value = Value::Bool(false);
                        continue;
                    }
                    if op == BinaryOp::Or && matches!(&value, Value::Bool(true)) {
                        value = Value::Bool(true);
                        continue;
                    }
                    let right = self.eval(right, env)?;
                    value = self.binary(value, op, right, span)?;
                }
                Ok(value)
            }
            ExprKind::Sql { query, result_type } => {
                let capability = sql_required_capability(query);
                self.require_runtime_capability(capability, "SQL access", expr.span)?;
                let Some(database_url) = self.database_url.clone() else {
                    return Err(
                        self.runtime_error(expr.span, "SQL execution requires a database runtime")
                    );
                };
                let params = query_parameters(query, env, expr.span)?;
                self.ensure_decimal_storage(expr.span)?;
                let result = zelyra_database::execute_query(&database_url, query, params).map_err(
                    |error| {
                        self.runtime_error(expr.span, format!("database query failed: {error}"))
                    },
                )?;
                Ok(Value::Rows {
                    columns: result.columns,
                    rows: result.rows,
                    metadata: Box::new(SqlRowsMetadata {
                        nulls: result.nulls,
                        column_types: result.column_types,
                    }),
                    result_type: Box::new(result_type.clone()),
                })
            }
            ExprKind::Await(inner) => self.eval(inner, env),
            ExprKind::Match { value, arms } => {
                let scrutinee = self.eval(value, env)?;
                for arm in arms {
                    if let Some(bindings) = match_pattern(&scrutinee, &arm.pattern) {
                        env.push();
                        for (name, value) in bindings {
                            env.declare(name, value, false);
                        }
                        if let Some(guard) = &arm.guard {
                            let guard_value = self.eval(guard, env);
                            let guard_matches = match guard_value {
                                Ok(value) => self.expect_bool(value, guard.span),
                                Err(error) => Err(error),
                            };
                            let guard_matches = match guard_matches {
                                Ok(value) => value,
                                Err(error) => {
                                    env.pop();
                                    return Err(error);
                                }
                            };
                            if !guard_matches {
                                env.pop();
                                continue;
                            }
                        }
                        let result = self.eval(&arm.value, env);
                        env.pop();
                        return result;
                    }
                }
                Err(self.runtime_error(expr.span, "non-exhaustive match at runtime"))
            }
        }
    }
    fn binary(
        &self,
        left: Value,
        op: BinaryOp,
        right: Value,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        use BinaryOp::*;
        match (left, op, right) {
            (Value::Int(a), Add, Value::Int(b)) => a
                .checked_add(b)
                .map(Value::Int)
                .ok_or_else(|| self.runtime_error(span, "Int overflow")),
            (Value::Int(a), Subtract, Value::Int(b)) => a
                .checked_sub(b)
                .map(Value::Int)
                .ok_or_else(|| self.runtime_error(span, "Int overflow")),
            (Value::Int(a), Multiply, Value::Int(b)) => a
                .checked_mul(b)
                .map(Value::Int)
                .ok_or_else(|| self.runtime_error(span, "Int overflow")),
            (Value::Int(_), Divide, Value::Int(0)) | (Value::Int(_), Remainder, Value::Int(0)) => {
                Err(self.runtime_error(span, "division by zero"))
            }
            (Value::Int(i64::MIN), Divide, Value::Int(-1)) => {
                Err(self.runtime_error(span, "Int overflow"))
            }
            (Value::Int(i64::MIN), Remainder, Value::Int(-1)) => {
                Err(self.runtime_error(span, "Int overflow"))
            }
            (Value::Int(a), Divide, Value::Int(b)) => a
                .checked_div(b)
                .map(Value::Int)
                .ok_or_else(|| self.runtime_error(span, "Int overflow")),
            (Value::Int(a), Remainder, Value::Int(b)) => a
                .checked_rem(b)
                .map(Value::Int)
                .ok_or_else(|| self.runtime_error(span, "Int overflow")),
            (Value::UInt(a), Add, Value::UInt(b)) => a
                .checked_add(b)
                .map(Value::UInt)
                .ok_or_else(|| self.runtime_error(span, "UInt overflow")),
            (Value::UInt(a), Subtract, Value::UInt(b)) => a
                .checked_sub(b)
                .map(Value::UInt)
                .ok_or_else(|| self.runtime_error(span, "UInt underflow")),
            (Value::UInt(a), Multiply, Value::UInt(b)) => a
                .checked_mul(b)
                .map(Value::UInt)
                .ok_or_else(|| self.runtime_error(span, "UInt overflow")),
            (Value::UInt(_), Divide, Value::UInt(0))
            | (Value::UInt(_), Remainder, Value::UInt(0)) => {
                Err(self.runtime_error(span, "division by zero"))
            }
            (Value::UInt(a), Divide, Value::UInt(b)) => Ok(Value::UInt(a / b)),
            (Value::UInt(a), Remainder, Value::UInt(b)) => Ok(Value::UInt(a % b)),
            (Value::Float(a), Add, Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Float(a), Subtract, Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Float(a), Multiply, Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Float(a), Divide, Value::Float(b)) => Ok(Value::Float(a / b)),
            (Value::Float(a), Remainder, Value::Float(b)) => Ok(Value::Float(a % b)),
            (Value::Decimal(a), Add, Value::Decimal(b)) => a
                .checked_add(b)
                .map(Value::Decimal)
                .ok_or_else(|| self.runtime_error(span, "Decimal overflow")),
            (Value::Decimal(a), Subtract, Value::Decimal(b)) => a
                .checked_sub(b)
                .map(Value::Decimal)
                .ok_or_else(|| self.runtime_error(span, "Decimal overflow")),
            (Value::Decimal(a), Multiply, Value::Decimal(b)) => {
                decimal_math::scaled_product(a, b, DECIMAL_SCALE)
                    .map(Value::Decimal)
                    .ok_or_else(|| self.runtime_error(span, "Decimal overflow"))
            }
            (Value::Decimal(_), Divide, Value::Decimal(0)) => {
                Err(self.runtime_error(span, "division by zero"))
            }
            (Value::Decimal(a), Divide, Value::Decimal(b)) => {
                decimal_math::scaled_product(a, DECIMAL_SCALE, b)
                    .map(Value::Decimal)
                    .ok_or_else(|| self.runtime_error(span, "Decimal overflow"))
            }
            (Value::Decimal(_), Remainder, Value::Decimal(0)) => {
                Err(self.runtime_error(span, "division by zero"))
            }
            (Value::Decimal(a), Remainder, Value::Decimal(b)) => {
                // i128::MIN % -1 has a representable zero remainder, but the
                // machine division used for `%` traps before returning it.
                Ok(Value::Decimal(if b == -1 { 0 } else { a % b }))
            }
            (Value::String(a), Add, Value::String(b)) => Ok(Value::String(a + &b)),
            (Value::Duration(a), Add, Value::Duration(b)) => a
                .checked_add(b)
                .map(Value::Duration)
                .ok_or_else(|| self.runtime_error(span, "Duration overflow")),
            (Value::Duration(a), Subtract, Value::Duration(b)) => a
                .checked_sub(b)
                .map(Value::Duration)
                .ok_or_else(|| self.runtime_error(span, "Duration overflow")),
            (Value::Array(mut a), Add, Value::Array(b)) => {
                a.extend(b);
                Ok(Value::Array(a))
            }
            (Value::Function(_), Equal | NotEqual, _)
            | (_, Equal | NotEqual, Value::Function(_)) => {
                Err(self.runtime_error(span, "function values cannot be compared"))
            }
            (a, Equal, b) => Ok(Value::Bool(a == b)),
            (a, NotEqual, b) => Ok(Value::Bool(a != b)),
            (Value::Int(a), Less, Value::Int(b)) => Ok(Value::Bool(a < b)),
            (Value::Int(a), LessEqual, Value::Int(b)) => Ok(Value::Bool(a <= b)),
            (Value::Int(a), Greater, Value::Int(b)) => Ok(Value::Bool(a > b)),
            (Value::Int(a), GreaterEqual, Value::Int(b)) => Ok(Value::Bool(a >= b)),
            (Value::UInt(a), Less, Value::UInt(b)) => Ok(Value::Bool(a < b)),
            (Value::UInt(a), LessEqual, Value::UInt(b)) => Ok(Value::Bool(a <= b)),
            (Value::UInt(a), Greater, Value::UInt(b)) => Ok(Value::Bool(a > b)),
            (Value::UInt(a), GreaterEqual, Value::UInt(b)) => Ok(Value::Bool(a >= b)),
            (Value::Float(a), Less, Value::Float(b)) => Ok(Value::Bool(a < b)),
            (Value::Float(a), LessEqual, Value::Float(b)) => Ok(Value::Bool(a <= b)),
            (Value::Float(a), Greater, Value::Float(b)) => Ok(Value::Bool(a > b)),
            (Value::Float(a), GreaterEqual, Value::Float(b)) => Ok(Value::Bool(a >= b)),
            (Value::Decimal(a), Less, Value::Decimal(b)) => Ok(Value::Bool(a < b)),
            (Value::Decimal(a), LessEqual, Value::Decimal(b)) => Ok(Value::Bool(a <= b)),
            (Value::Decimal(a), Greater, Value::Decimal(b)) => Ok(Value::Bool(a > b)),
            (Value::Decimal(a), GreaterEqual, Value::Decimal(b)) => Ok(Value::Bool(a >= b)),
            (Value::String(a), Less, Value::String(b)) => Ok(Value::Bool(a < b)),
            (Value::String(a), LessEqual, Value::String(b)) => Ok(Value::Bool(a <= b)),
            (Value::String(a), Greater, Value::String(b)) => Ok(Value::Bool(a > b)),
            (Value::String(a), GreaterEqual, Value::String(b)) => Ok(Value::Bool(a >= b)),
            (Value::Date(a), Less, Value::Date(b)) | (Value::Time(a), Less, Value::Time(b)) => {
                Ok(Value::Bool(a < b))
            }
            (Value::Date(a), LessEqual, Value::Date(b))
            | (Value::Time(a), LessEqual, Value::Time(b)) => Ok(Value::Bool(a <= b)),
            (Value::Date(a), Greater, Value::Date(b))
            | (Value::Time(a), Greater, Value::Time(b)) => Ok(Value::Bool(a > b)),
            (Value::Date(a), GreaterEqual, Value::Date(b))
            | (Value::Time(a), GreaterEqual, Value::Time(b)) => Ok(Value::Bool(a >= b)),
            (Value::Timestamp(a), Less, Value::Timestamp(b)) => Ok(Value::Bool(a < b)),
            (Value::Timestamp(a), LessEqual, Value::Timestamp(b)) => Ok(Value::Bool(a <= b)),
            (Value::Timestamp(a), Greater, Value::Timestamp(b)) => Ok(Value::Bool(a > b)),
            (Value::Timestamp(a), GreaterEqual, Value::Timestamp(b)) => Ok(Value::Bool(a >= b)),
            (Value::Duration(a), Less, Value::Duration(b)) => Ok(Value::Bool(a < b)),
            (Value::Duration(a), LessEqual, Value::Duration(b)) => Ok(Value::Bool(a <= b)),
            (Value::Duration(a), Greater, Value::Duration(b)) => Ok(Value::Bool(a > b)),
            (Value::Duration(a), GreaterEqual, Value::Duration(b)) => Ok(Value::Bool(a >= b)),
            (Value::Bool(a), And, Value::Bool(b)) => Ok(Value::Bool(a && b)),
            (Value::Bool(a), Or, Value::Bool(b)) => Ok(Value::Bool(a || b)),
            (left, op, right) => Err(self.runtime_error(
                span,
                format!(
                    "invalid operation {op:?} for {} and {}",
                    left.ty(),
                    right.ty()
                ),
            )),
        }
    }
}

const DECIMAL_SCALE: i128 = 1_000_000_000;

fn quantize_decimal(value: i128, scale: u32, mode: &str) -> Result<i128, String> {
    if scale > 9 {
        return Err("decimal scale must be between 0 and 9".into());
    }
    if !matches!(
        mode,
        "toward_zero" | "floor" | "ceiling" | "half_up" | "half_even"
    ) {
        return Err(
            "rounding mode must be toward_zero, floor, ceiling, half_up, or half_even".into(),
        );
    }
    let factor = 10_i128.pow(9 - scale);
    let quotient = value / factor;
    let remainder = value % factor;
    if remainder == 0 || mode == "toward_zero" {
        return quotient
            .checked_mul(factor)
            .ok_or_else(|| "Decimal overflow while quantizing".into());
    }
    let negative = value < 0;
    let increment = match mode {
        "floor" => negative,
        "ceiling" => !negative,
        "half_up" => remainder.unsigned_abs() * 2 >= factor as u128,
        "half_even" => {
            let doubled = remainder.unsigned_abs() * 2;
            doubled > factor as u128 || (doubled == factor as u128 && quotient % 2 != 0)
        }
        _ => false,
    };
    let rounded = if increment {
        quotient.checked_add(if negative { -1 } else { 1 })
    } else {
        Some(quotient)
    }
    .ok_or_else(|| "Decimal overflow while quantizing".to_owned())?;
    rounded
        .checked_mul(factor)
        .ok_or_else(|| "Decimal overflow while quantizing".into())
}

fn canonical_set(mut values: Vec<Value>) -> Vec<Value> {
    values.sort_by_key(Value::output);
    values.dedup();
    values
}

fn parse_decimal(value: &str) -> Option<i128> {
    zelyra_ast::decimal::parse_scaled(value, zelyra_ast::decimal::SCALE_DIGITS)
}

fn format_decimal(value: i128) -> String {
    let magnitude = value.unsigned_abs();
    let whole = magnitude / DECIMAL_SCALE as u128;
    let fraction = magnitude % DECIMAL_SCALE as u128;
    if fraction == 0 {
        return format!("{}{whole}", if value < 0 { "-" } else { "" });
    }
    let fraction = format!("{fraction:09}").trim_end_matches('0').to_owned();
    format!("{}{whole}.{fraction}", if value < 0 { "-" } else { "" })
}

fn query_parameters(
    query: &str,
    env: &Environment,
    span: Span,
) -> Result<Vec<(String, zelyra_database::QueryValue)>, RuntimeError> {
    let mut parameters = Vec::new();
    let names = zelyra_database::sql_parameter_names(query).map_err(|error| RuntimeError {
        message: format!("could not scan SQL parameters: {error}"),
        span,
    })?;
    for name in names {
        if !parameters.iter().any(|(existing, _)| existing == &name) {
            let value = env.get(&name).ok_or_else(|| RuntimeError {
                message: format!("SQL parameter `:{name}` is not available"),
                span,
            })?;
            parameters.push((name, value_to_query_value(&value, span)?));
        }
    }
    Ok(parameters)
}

fn is_pure_mail_expression(expression: &Expr) -> bool {
    match &expression.kind {
        ExprKind::String(_) | ExprKind::Variable(_) => true,
        ExprKind::Field { target, .. } => is_pure_mail_expression(target),
        ExprKind::Record { fields, .. } => fields
            .iter()
            .all(|(_, value)| is_pure_mail_expression(value)),
        ExprKind::Map(entries) => entries
            .iter()
            .all(|(key, value)| is_pure_mail_expression(key) && is_pure_mail_expression(value)),
        ExprKind::Call { name, args, .. }
            if name == "render_pdf"
                && args.len() == 2
                && matches!(args[0].kind, ExprKind::String(_)) =>
        {
            is_pure_mail_expression(&args[1])
        }
        ExprKind::Binary {
            left,
            op: BinaryOp::Add,
            right,
        } => is_pure_mail_expression(left) && is_pure_mail_expression(right),
        _ => false,
    }
}

fn value_to_query_value(
    value: &Value,
    span: Span,
) -> Result<zelyra_database::QueryValue, RuntimeError> {
    use zelyra_database::QueryValue;
    match value {
        Value::Int(value) => Ok(QueryValue::Int(*value)),
        Value::UInt(value) => Ok(QueryValue::UInt(*value)),
        Value::Float(value) => Ok(QueryValue::Float(*value)),
        Value::Decimal(value) => Ok(QueryValue::Decimal(format_decimal(*value))),
        Value::Bool(value) => Ok(QueryValue::Bool(*value)),
        Value::String(value) => Ok(QueryValue::String(value.clone())),
        Value::Char(value) => Ok(QueryValue::String(value.to_string())),
        Value::Timestamp(value) => Ok(QueryValue::Timestamp(*value)),
        Value::Date(value) => Ok(QueryValue::Date(value.clone())),
        Value::Time(value) => Ok(QueryValue::Time(value.clone())),
        Value::Duration(value) => Ok(QueryValue::Int(*value)),
        Value::Option(None) | Value::Unit => Ok(QueryValue::Null),
        Value::Option(Some(value)) => value_to_query_value(value, span),
        Value::Array(_)
        | Value::Tuple(_)
        | Value::Set(_)
        | Value::Map(_)
        | Value::Object { .. }
        | Value::ClassRef { .. }
        | Value::Enum { .. }
        | Value::Function(_)
        | Value::Rows { .. }
        | Value::Bytes(_)
        | Value::Result(_) => Err(RuntimeError {
            message: "SQL parameters must be scalar values".into(),
            span,
        }),
    }
}

fn transaction_queries(
    interpreter: &mut Interpreter,
    block: &Block,
    env: &mut Environment,
    span: Span,
) -> Result<Vec<zelyra_database::Query>, RuntimeError> {
    let mut queries = Vec::new();
    for statement in &block.statements {
        let (expression, allow_mail_queue) = match statement {
            Stmt::Expr(expression) => (expression, true),
            Stmt::Let { value, .. } => (value, false),
            _ => {
                return Err(RuntimeError {
                    message: "transactions may contain only SQL statements".into(),
                    span,
                })
            }
        };
        match &expression.kind {
            ExprKind::Sql { query, .. } => queries.push(zelyra_database::Query {
                sql: query.clone(),
                params: query_parameters(query, env, expression.span)?,
            }),
            ExprKind::Call { name, args, .. } if name == "mail_queue" => {
                if !allow_mail_queue {
                    return Err(interpreter.runtime_error(
                        expression.span,
                        "mail_queue must be a standalone statement inside a transaction",
                    ));
                }
                if !matches!(args.len(), 3..=6) {
                    return Err(interpreter.runtime_error(
                        expression.span,
                        "mail_queue expects recipient, subject, text body, optional HTML body, and optional PDF filename and Bytes",
                    ));
                }
                if args
                    .iter()
                    .any(|argument| !is_pure_mail_expression(argument))
                {
                    return Err(interpreter.runtime_error(
                        expression.span,
                        "mail_queue arguments must be values or pure expressions; PDF attachments must use render_pdf",
                    ));
                }
                interpreter.require_runtime_capability(
                    "Email",
                    "email delivery",
                    expression.span,
                )?;
                interpreter.require_runtime_capability(
                    "Database(write)",
                    "transactional mail queue write",
                    expression.span,
                )?;
                let database_url = interpreter.database_url.as_deref().ok_or_else(|| {
                    interpreter.runtime_error(
                        expression.span,
                        "mail_queue requires a MariaDB or MySQL database runtime",
                    )
                })?;
                if !supports_mariadb_outbox(database_url) {
                    return Err(interpreter.runtime_error(
                        expression.span,
                        "mail_queue currently requires MariaDB or MySQL; PostgreSQL outbox support is not available",
                    ));
                }
                let mut evaluated = Vec::with_capacity(args.len());
                for argument in args {
                    evaluated.push(interpreter.eval(argument, env)?);
                }
                let string_argument = |index: usize| -> Result<String, RuntimeError> {
                    match evaluated.get(index) {
                        Some(Value::String(value)) => Ok(value.clone()),
                        Some(value) => Err(interpreter.runtime_error(
                            args[index].span,
                            format!("mail_queue expects String argument, found {}", value.ty()),
                        )),
                        None => Err(interpreter.runtime_error(
                            expression.span,
                            "mail_queue is missing a required argument",
                        )),
                    }
                };
                let to = string_argument(0)?;
                let subject = string_argument(1)?;
                let text_body = string_argument(2)?;
                let html = match args.len() {
                    4 | 6 => Some(string_argument(3)?),
                    _ => None,
                };
                let attachment_start = match args.len() {
                    5 => Some(3),
                    6 => Some(4),
                    _ => None,
                };
                let attachment_storage = if let Some(start) = attachment_start {
                    let filename = string_argument(start)?;
                    let bytes = match evaluated.get(start + 1) {
                        Some(Value::Bytes(bytes)) => bytes.clone(),
                        Some(value) => {
                            return Err(interpreter.runtime_error(
                                args[start + 1].span,
                                format!(
                                    "mail_queue PDF attachment expects Bytes, found {}",
                                    value.ty()
                                ),
                            ));
                        }
                        None => {
                            return Err(interpreter.runtime_error(
                                expression.span,
                                "mail_queue is missing PDF attachment bytes",
                            ));
                        }
                    };
                    Some((filename, bytes))
                } else {
                    None
                };
                let attachment_refs = attachment_storage
                    .as_ref()
                    .map(|(filename, bytes)| {
                        vec![zelyra_mail::MailAttachment {
                            filename,
                            content_type: "application/pdf",
                            bytes,
                        }]
                    })
                    .unwrap_or_default();
                let mail_message = zelyra_mail::MailMessage {
                    to: &to,
                    subject: &subject,
                    text_body: &text_body,
                    html_body: html.as_deref(),
                    message_id: None,
                    attachments: &attachment_refs,
                };
                zelyra_mail::validate_message(&mail_message).map_err(|_| {
                    interpreter
                        .runtime_error(expression.span, "mail_queue received an invalid message")
                })?;
                let attachments_json = attachment_storage
                    .as_ref()
                    .map(|(filename, bytes)| {
                        vec![serde_json::json!({
                            "filename": filename,
                            "content_type": "application/pdf",
                            "content": base64_encode_bytes(bytes),
                        })]
                    })
                    .unwrap_or_default();
                let payload = serde_json::json!({
                    "to": &to,
                    "subject": &subject,
                    "text": &text_body,
                    "html": html,
                    "attachments": attachments_json,
                })
                .to_string();
                if payload.len() > zelyra_database::outbox::MAX_EVENT_PAYLOAD_BYTES {
                    return Err(interpreter.runtime_error(
                        expression.span,
                        "mail_queue payload exceeds the outbox size limit",
                    ));
                }
                let mut random = [0_u8; 16];
                OsRng.try_fill_bytes(&mut random).map_err(|_| {
                    interpreter.runtime_error(
                        expression.span,
                        "mail_queue could not create a unique event ID",
                    )
                })?;
                let event_id = random
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                queries.push(zelyra_database::Query {
                    sql: "INSERT INTO `_zelyra_outbox` (`event_id`, `event_type`, `payload`) VALUES (:event_id, :event_type, :payload)".into(),
                    params: vec![
                        ("event_id".into(), zelyra_database::QueryValue::String(event_id)),
                        ("event_type".into(), zelyra_database::QueryValue::String("zelyra.mail.send".into())),
                        ("payload".into(), zelyra_database::QueryValue::String(payload)),
                    ],
                });
            }
            _ => return Err(RuntimeError {
                message:
                    "transactions may contain only SQL statements and standalone mail_queue calls"
                        .into(),
                span: expression.span,
            }),
        }
    }
    if queries.is_empty() {
        return Err(RuntimeError {
            message: "transaction must contain at least one SQL statement".into(),
            span,
        });
    }
    Ok(queries)
}

fn supports_mariadb_outbox(database_url: &str) -> bool {
    database_url.starts_with("mariadb://") || database_url.starts_with("mysql://")
}

fn match_pattern(value: &Value, pattern: &Pattern) -> Option<Vec<(String, Value)>> {
    match &pattern.kind {
        PatternKind::Wildcard => Some(Vec::new()),
        PatternKind::Variable(name) => Some(vec![(name.clone(), value.clone())]),
        PatternKind::Int(expected) => match value {
            Value::Int(actual) if actual == expected => Some(Vec::new()),
            _ => None,
        },
        PatternKind::Bool(expected) => match value {
            Value::Bool(actual) if actual == expected => Some(Vec::new()),
            _ => None,
        },
        PatternKind::String(expected) => match value {
            Value::String(actual) if actual == expected => Some(Vec::new()),
            _ => None,
        },
        PatternKind::Char(expected) => match value {
            Value::Char(actual) if actual == expected => Some(Vec::new()),
            _ => None,
        },
        PatternKind::EnumVariant { enum_name, variant } => match value {
            Value::Enum {
                type_name,
                variant: actual,
                payloads,
                ..
            } if type_name == enum_name && actual == variant && payloads.is_empty() => {
                Some(Vec::new())
            }
            _ => None,
        },
        PatternKind::EnumVariantPayload {
            enum_name,
            variant,
            payloads: patterns,
        } => match value {
            Value::Enum {
                type_name,
                variant: actual,
                payloads,
                ..
            } if type_name == enum_name
                && actual == variant
                && payloads.len() == patterns.len() =>
            {
                let mut bindings = Vec::new();
                for (value, pattern) in payloads.iter().zip(patterns) {
                    bindings.extend(match_pattern(value, pattern)?);
                }
                Some(bindings)
            }
            _ => None,
        },
        PatternKind::Record {
            type_name,
            fields: patterns,
        } => match value {
            Value::Object {
                type_name: actual,
                fields,
                ..
            } if actual == type_name => {
                let mut bindings = Vec::new();
                for (name, pattern) in patterns {
                    bindings.extend(match_pattern(fields.get(name)?, pattern)?);
                }
                Some(bindings)
            }
            _ => None,
        },
        PatternKind::Constructor { name, inner } => match (name.as_str(), value) {
            ("Some", Value::Option(Some(value))) | ("Ok", Value::Result(Ok(value))) => inner
                .as_ref()
                .and_then(|pattern| match_pattern(value, pattern)),
            ("None", Value::Option(None)) => {
                if inner.is_none() {
                    Some(Vec::new())
                } else {
                    None
                }
            }
            ("Err", Value::Result(Err(_))) if inner.is_none() => Some(Vec::new()),
            ("Err", Value::Result(Err(value))) => inner
                .as_ref()
                .and_then(|pattern| match_pattern(value, pattern)),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests;
