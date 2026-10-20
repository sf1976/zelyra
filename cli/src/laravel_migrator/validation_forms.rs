//! Generate reviewable standalone Zelyra forms from a strict subset of static
//! Laravel validation arrays. This never claims endpoint, authorization, or
//! Laravel validator parity.

use serde_json::{json, Value};
use std::collections::BTreeSet;

pub(super) struct ValidationForms {
    pub source: String,
    pub mappings: Vec<Value>,
    pub rejected: Vec<Value>,
}

pub(super) fn generate(report: &Value, excluded_controller_file: Option<&str>) -> ValidationForms {
    let mut source = String::new();
    let mut mappings = Vec::new();
    let mut rejected = Vec::new();
    let mut names = BTreeSet::new();

    for node in report["graph"]["nodes"].as_array().into_iter().flatten() {
        if node["kind"] != "validation" {
            continue;
        }
        let file = node["file"].as_str().unwrap_or("unknown");
        if excluded_controller_file == Some(file) {
            rejected.push(json!({
                "node_id": node["id"],
                "file": file,
                "line": node["source_span"]["start"]["line"],
                "reason": "standalone validation form omitted because this controller's login flow is replaced by the generated native Zelyra auth module"
            }));
            continue;
        }
        let validation = &node["validation_rules"];
        if validation["parse_mode"] != "static_array_literal_subset" {
            rejected.push(json!({
                "node_id": node["id"],
                "file": file,
                "line": node["source_span"]["start"]["line"],
                "reason": "validation rules are not a statically enumerated literal array"
            }));
            continue;
        }

        let base_name = form_base_name(node["name"].as_str().unwrap_or("Validation"));
        let line = node["source_span"]["start"]["line"]
            .as_u64()
            .unwrap_or_default();
        let mut form_name = format!("Imported{base_name}Line{line}");
        if !names.insert(form_name.clone()) {
            let offset = node["source_span"]["start"]["byte_offset"]
                .as_u64()
                .unwrap_or_default();
            form_name.push_str(&format!("Offset{offset}"));
            names.insert(form_name.clone());
        }

        let mut generated_fields = Vec::new();
        let mut omitted_fields = Vec::new();
        for field in validation["fields"].as_array().into_iter().flatten() {
            let field_name = field["field"].as_str().unwrap_or("");
            match field_mapping(field) {
                Ok(mapped) => generated_fields.push((field_name.to_owned(), mapped)),
                Err(reason) => omitted_fields.push(json!({
                    "field": field["field"],
                    "field_span": field["field_span"],
                    "rules": field["rules"],
                    "reason": reason
                })),
            }
        }

        if generated_fields.is_empty() {
            rejected.push(json!({
                "node_id": node["id"],
                "name": form_name,
                "file": file,
                "line": line,
                "omitted_fields": omitted_fields,
                "reason": "no field has a complete, supported standalone-form rule subset"
            }));
            continue;
        }

        source.push_str(&format!("form {form_name} {{\n"));
        for (field_name, mapped) in &generated_fields {
            source.push_str(&format!("    field {field_name}: {} {{\n", mapped.ty));
            if mapped.required {
                source.push_str("        required\n");
            }
            if let Some(min) = mapped.min_length {
                source.push_str(&format!("        min_length: {min}\n"));
            }
            if let Some(max) = mapped.max {
                source.push_str(&format!("        max: {max}\n"));
            }
            if let Some(min) = &mapped.min_value {
                source.push_str(&format!("        min_value: \"{min}\"\n"));
            }
            if let Some(max) = &mapped.max_value {
                source.push_str(&format!("        max_value: \"{max}\"\n"));
            }
            if !mapped.choices.is_empty() {
                source.push_str("        choices: [");
                source.push_str(
                    &mapped
                        .choices
                        .iter()
                        .map(|choice| format!("\"{choice}\""))
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                source.push_str("]\n");
            }
            source.push_str("    }\n");
        }
        source.push_str("}\n\n");
        mappings.push(json!({
            "node_id": node["id"],
            "form": form_name,
            "file": file,
            "line": line,
            "source_span": node["source_span"],
            "generated_fields": generated_fields.iter().map(|(name, field)| json!({
                "field": name,
                "type": field.ty,
                "required": field.required,
                "min_length": field.min_length,
                "max_length": field.max,
                "min_value": field.min_value,
                "max_value": field.max_value,
                "choices": field.choices
            })).collect::<Vec<_>>(),
            "omitted_fields": omitted_fields,
            "route_or_handler_wired": false,
            "persistence_action_generated": false
        }));
    }

    ValidationForms {
        source,
        mappings,
        rejected,
    }
}

struct FormFieldMapping {
    ty: &'static str,
    required: bool,
    max: Option<u32>,
    min_length: Option<u32>,
    min_value: Option<String>,
    max_value: Option<String>,
    choices: Vec<String>,
}

fn field_mapping(field: &Value) -> Result<FormFieldMapping, &'static str> {
    if field["complete"] != true {
        return Err("field rules contain dynamic or unparsed Laravel expressions");
    }
    let Some(name) = field["field"].as_str() else {
        return Err("field name is not a string literal");
    };
    if !valid_identifier(name) {
        return Err("nested, wildcard, or non-identifier field names need manual mapping");
    }
    let rules = field["rules"]
        .as_array()
        .ok_or("field rule list is not an array")?;
    let mut ty = None;
    let mut required = false;
    let mut max = None;
    let mut min = None;
    let mut choices = None;
    for rule in rules {
        let rule_name = rule["name"].as_str().unwrap_or("");
        match rule_name {
            "required" => required = true,
            "nullable" | "sometimes" => {}
            "string" => set_type(&mut ty, "String")?,
            "email" => set_type(&mut ty, "Email")?,
            "integer" => set_type(&mut ty, "Int")?,
            "numeric" => set_type(&mut ty, "Decimal")?,
            "boolean" => set_type(&mut ty, "Bool")?,
            "date" => set_type(&mut ty, "Date")?,
            "uuid" => set_type(&mut ty, "Uuid")?,
            "max" => {
                if max.is_some() {
                    return Err("duplicate Laravel max rules need manual review");
                }
                max = Some(
                    rule["parameter"]
                        .as_str()
                        .ok_or("max rule has no literal parameter")?
                        .to_owned(),
                );
            }
            "min" => {
                if min.is_some() {
                    return Err("duplicate Laravel min rules need manual review");
                }
                min = Some(
                    rule["parameter"]
                        .as_str()
                        .ok_or("min rule has no literal parameter")?
                        .to_owned(),
                );
            }
            "in" => {
                if choices.is_some() {
                    return Err("duplicate Laravel in rules need manual review");
                }
                let parameter = rule["parameter"]
                    .as_str()
                    .ok_or("in rule has no safe static parameter")?;
                choices = Some(parse_choices(parameter)?);
            }
            "accepted" | "active_url" | "after" | "after_or_equal" | "alpha" | "alpha_dash"
            | "alpha_num" | "array" | "before" | "before_or_equal" | "confirmed"
            | "date_equals" | "different" | "digits" | "digits_between" | "exists" | "file"
            | "image" | "ip" | "json" | "mimes" | "present" | "regex" | "required_if"
            | "required_unless" | "same" | "size" | "unique" | "url" => {
                return Err("Laravel rule is not represented by Zelyra FormField constraints")
            }
            _ => return Err("unknown Laravel rule needs manual mapping"),
        }
    }
    let ty = ty.ok_or("no supported field type rule was found")?;
    let choices = choices.unwrap_or_default();
    let (max, min_length, min_value, max_value) = match ty {
        "String" | "Email" | "Uuid" => {
            let min_length = min
                .map(|value| parse_positive_length(&value, "min"))
                .transpose()?;
            let max_length = max
                .map(|value| parse_positive_length(&value, "max"))
                .transpose()?;
            (max_length, min_length, None, None)
        }
        "Int" | "Decimal" => {
            let min_value = min
                .map(|value| validate_numeric_bound(&value, ty))
                .transpose()?;
            let max_value = max
                .map(|value| validate_numeric_bound(&value, ty))
                .transpose()?;
            if let (Some(min), Some(max)) = (&min_value, &max_value) {
                if compare_decimal(min, max).is_some_and(|ordering| ordering.is_gt()) {
                    return Err("Laravel min rule exceeds max rule");
                }
            }
            (None, None, min_value, max_value)
        }
        _ if min.is_some() || max.is_some() => {
            return Err("Laravel min/max constraints require text or numeric fields")
        }
        _ => (None, None, None, None),
    };
    if ty == "Int"
        && (min_value
            .as_deref()
            .is_some_and(|value| value.parse::<i64>().is_err())
            || max_value
                .as_deref()
                .is_some_and(|value| value.parse::<i64>().is_err()))
    {
        return Err("Laravel numeric bound does not fit the Zelyra Int range");
    }
    if !choices.is_empty()
        && !matches!(ty, "String" | "Email" | "Uuid" | "Int" | "Decimal" | "Bool")
    {
        return Err("Laravel in rule requires a supported scalar field type");
    }
    if ty == "Int" && choices.iter().any(|value| value.parse::<i64>().is_err()) {
        return Err("Laravel in value is not a valid Zelyra Int");
    }
    if ty == "Decimal"
        && choices
            .iter()
            .any(|value| zelyra_ast::decimal::parse_scaled(value, 9).is_none())
    {
        return Err("Laravel in value is not an exact Zelyra Decimal");
    }
    if ty == "Bool"
        && choices
            .iter()
            .any(|value| !matches!(value.as_str(), "true" | "false" | "1" | "0"))
    {
        return Err("Laravel in value is not a valid Zelyra Bool");
    }
    Ok(FormFieldMapping {
        ty,
        required,
        max,
        min_length,
        min_value,
        max_value,
        choices,
    })
}

fn parse_choices(value: &str) -> Result<Vec<String>, &'static str> {
    let choices = value.split(',').map(str::to_owned).collect::<Vec<_>>();
    if choices.is_empty()
        || choices.len() > 64
        || choices.iter().any(|value| {
            value.is_empty()
                || value.len() > 64
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        })
    {
        return Err("Laravel in values are outside the supported static scalar subset");
    }
    if choices.iter().collect::<BTreeSet<_>>().len() != choices.len() {
        return Err("duplicate Laravel in values need manual review");
    }
    Ok(choices)
}

fn parse_positive_length(value: &str, rule: &str) -> Result<u32, &'static str> {
    value
        .parse::<u32>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or(if rule == "min" {
            "Laravel min length must be a positive integer"
        } else {
            "Laravel max length must be a positive integer"
        })
}

fn validate_numeric_bound(value: &str, ty: &str) -> Result<String, &'static str> {
    if ty == "Int" {
        value
            .parse::<i64>()
            .map_err(|_| "Laravel integer bound is outside the Zelyra Int range")?;
    } else if zelyra_ast::decimal::parse_scaled(value, 9).is_none() {
        return Err("Laravel numeric bound is not an exact Zelyra Decimal literal");
    }
    Ok(value.to_owned())
}

fn compare_decimal(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    let left = zelyra_ast::decimal::parse_scaled(left, 9)?;
    let right = zelyra_ast::decimal::parse_scaled(right, 9)?;
    Some(left.cmp(&right))
}

fn set_type(
    current: &mut Option<&'static str>,
    candidate: &'static str,
) -> Result<(), &'static str> {
    if let Some(existing) = *current {
        let compatible = match (existing, candidate) {
            ("String", "Email") | ("Email", "String") => Some("Email"),
            ("String", "Uuid") | ("Uuid", "String") => Some("Uuid"),
            ("Decimal", "Int") | ("Int", "Decimal") => Some("Int"),
            (left, right) if left == right => Some(left),
            _ => None,
        };
        *current = Some(compatible.ok_or(
            "conflicting Laravel type rules cannot be represented as one Zelyra field type",
        )?);
    } else {
        *current = Some(candidate);
    }
    Ok(())
}

fn valid_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn form_base_name(name: &str) -> String {
    let mut output = String::new();
    let mut capitalize = true;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if capitalize {
                output.push(character.to_ascii_uppercase());
            } else {
                output.push(character);
            }
            capitalize = false;
        } else {
            capitalize = true;
        }
    }
    if output.is_empty() {
        "Validation".into()
    } else {
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_laravel_text_length_and_exact_numeric_bounds() {
        let text = field_mapping(&json!({
            "field": "name", "complete": true,
            "rules": [
                {"name": "string"}, {"name": "min", "parameter": "2"},
                {"name": "max", "parameter": "80"}
            ]
        }))
        .unwrap();
        assert_eq!(text.min_length, Some(2));
        assert_eq!(text.max, Some(80));
        assert_eq!(text.min_value, None);

        let amount = field_mapping(&json!({
            "field": "amount", "complete": true,
            "rules": [
                {"name": "numeric"}, {"name": "min", "parameter": "0.01"},
                {"name": "max", "parameter": "9999.99"}
            ]
        }))
        .unwrap();
        assert_eq!(amount.min_value.as_deref(), Some("0.01"));
        assert_eq!(amount.max_value.as_deref(), Some("9999.99"));
    }

    #[test]
    fn rejects_misapplied_or_inverted_laravel_bounds() {
        let invalid = json!({
            "field": "flag", "complete": true,
            "rules": [{"name": "boolean"}, {"name": "min", "parameter": "2"}]
        });
        assert!(field_mapping(&invalid).is_err());

        let inverted = json!({
            "field": "amount", "complete": true,
            "rules": [
                {"name": "numeric"}, {"name": "min", "parameter": "10"},
                {"name": "max", "parameter": "2"}
            ]
        });
        assert!(matches!(
            field_mapping(&inverted),
            Err("Laravel min rule exceeds max rule")
        ));
    }

    #[test]
    fn imports_static_allowlists_and_rejects_unsafe_values() {
        let mapping = field_mapping(&json!({
            "field": "status", "complete": true,
            "rules": [
                {"name": "string"},
                {"name": "in", "parameter": "draft,issued,paid"}
            ]
        }))
        .unwrap();
        assert_eq!(mapping.choices, ["draft", "issued", "paid"]);

        let untrusted = json!({
            "field": "status", "complete": true,
            "rules": [
                {"name": "string"},
                {"name": "in", "parameter": "draft,paid\";print(1)"}
            ]
        });
        assert!(field_mapping(&untrusted).is_err());
    }

    #[test]
    fn imports_explicit_uuid_rules_as_uuid_fields() {
        let mapping = field_mapping(&json!({
            "field": "request_token", "complete": true,
            "rules": [
                {"name": "required"},
                {"name": "string"},
                {"name": "uuid"}
            ]
        }))
        .unwrap();
        assert_eq!(mapping.ty, "Uuid");
        assert!(mapping.required);

        let generated = generate(
            &report(
                "static_array_literal_subset",
                json!([{
                    "field": "request_token", "complete": true,
                    "rules": [
                        {"name": "required"},
                        {"name": "string"},
                        {"name": "uuid"}
                    ]
                }]),
            ),
            None,
        );
        assert!(generated
            .source
            .contains("field request_token: Uuid {\n        required"));
        assert_eq!(generated.mappings[0]["generated_fields"][0]["type"], "Uuid");
    }

    fn report(parse_mode: &str, fields: Value) -> Value {
        json!({
            "graph": { "nodes": [{
                "id": "validation:test",
                "kind": "validation",
                "name": "CustomerController@store validation",
                "file": "app/Http/Controllers/CustomerController.php",
                "source_span": { "start": { "line": 12, "byte_offset": 410 } },
                "validation_rules": { "parse_mode": parse_mode, "fields": fields }
            }] }
        })
    }

    #[test]
    fn generates_only_complete_standalone_fields_and_preserves_rejections() {
        let report = report(
            "static_array_literal_subset",
            json!([
                {"field":"email","complete":true,"rules":[
                    {"name":"required","parameter":null},
                    {"name":"string","parameter":null},
                    {"name":"email","parameter":null},
                    {"name":"max","parameter":"255"}
                ]},
                {"field":"customer_id","complete":true,"rules":[
                    {"name":"required","parameter":null},
                    {"name":"integer","parameter":null},
                    {"name":"numeric","parameter":null},
                    {"name":"exists","parameter":"customers,id"}
                ]}
            ]),
        );

        let generated = generate(&report, None);

        assert!(generated
            .source
            .contains("form ImportedCustomerControllerStoreValidationLine12 {"));
        assert!(generated
            .source
            .contains("field email: Email {\n        required\n        max: 255"));
        assert!(!generated.source.contains("customer_id"));
        assert_eq!(generated.mappings.len(), 1);
        assert_eq!(generated.mappings[0]["route_or_handler_wired"], false);
        assert_eq!(generated.mappings[0]["persistence_action_generated"], false);
        assert_eq!(
            generated.mappings[0]["omitted_fields"][0]["field"],
            "customer_id"
        );
        assert_eq!(generated.rejected.len(), 0);
    }

    #[test]
    fn rejects_dynamic_rule_arrays_without_inventing_forms() {
        let generated = generate(&report("dynamic_expression", json!([])), None);

        assert!(generated.source.is_empty());
        assert!(generated.mappings.is_empty());
        assert_eq!(generated.rejected.len(), 1);
        assert!(generated.rejected[0]["reason"]
            .as_str()
            .unwrap()
            .contains("statically enumerated"));
    }

    #[test]
    fn omits_login_controller_forms_when_native_auth_replaces_that_controller() {
        let source_file = "app/Http/Controllers/CustomerController.php";
        let generated = generate(
            &report(
                "static_array_literal_subset",
                json!([
                    {"field":"email","complete":true,"rules":[{"name":"required"},{"name":"email"}]}
                ]),
            ),
            Some(source_file),
        );

        assert!(generated.source.is_empty());
        assert!(generated.mappings.is_empty());
        assert_eq!(generated.rejected.len(), 1);
        assert!(generated.rejected[0]["reason"]
            .as_str()
            .unwrap()
            .contains("replaced by the generated native Zelyra auth module"));
    }
}
