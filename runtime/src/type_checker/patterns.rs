//! Pattern binding and exhaustiveness checks for the shared type checker.

use super::*;

impl Checker<'_> {
    pub(super) fn check_pattern(
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
    pub(super) fn check_exhaustiveness(
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
}
