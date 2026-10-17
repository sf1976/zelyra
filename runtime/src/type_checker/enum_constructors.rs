//! Type checking for enum payload constructors and generic enum inference.

use super::*;

pub(crate) fn instantiated_enum_payload_types(
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

impl Checker<'_> {
    pub(super) fn check_enum_constructor(
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
}
