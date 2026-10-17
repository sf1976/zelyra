//! Shared type relations, alias expansion, generic inference, and type validation.
//!
//! These pure compiler helpers are consumed by the type checker and by other
//! compiler/runtime services through the stable internal re-exports.
use super::*;

pub(crate) fn compatible(expected: &Type, actual: &Type) -> bool {
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
            expected_params.len() == actual_params.len()
                && expected_params
                    .iter()
                    .zip(actual_params)
                    .all(|(expected, actual)| compatible(expected, actual))
                && compatible(expected_return, actual_return)
                && actual_capabilities
                    .iter()
                    .all(|capability| expected_capabilities.contains(capability))
        }
        (Type::HttpResult(_), Type::Named(actual)) if actual == "HttpResult" => true,
        _ => expected == actual,
    }
}

pub(crate) fn infer_generic_types(
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

pub(crate) fn expand_type_aliases(
    ty: &Type,
    aliases: &HashMap<String, Type>,
) -> Result<Type, &'static str> {
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

pub(crate) fn substitute_generic_type(ty: &Type, substitutions: &HashMap<String, Type>) -> Type {
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

pub(crate) fn is_numeric(ty: &Type) -> bool {
    matches!(ty, Type::Int | Type::UInt | Type::Float | Type::Decimal)
}

pub(crate) fn is_map_key_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Int | Type::UInt | Type::String | Type::Bool | Type::Char | Type::Timestamp
    )
}

pub(crate) fn validate_type(
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

pub(crate) fn singular_table_type(name: &str) -> Option<String> {
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
