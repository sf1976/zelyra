use super::*;

pub fn render_form(
    route: &FormRoute,
    values: &HashMap<String, String>,
    errors: &[FieldError],
    notice: Option<&str>,
) -> String {
    render_form_with_options(route, values, errors, notice, &HashMap::new())
}

pub(super) fn render_form_with_options(
    route: &FormRoute,
    values: &HashMap<String, String>,
    errors: &[FieldError],
    notice: Option<&str>,
    relation_options: &HashMap<String, Vec<SelectOption>>,
) -> String {
    render_form_with_language(
        route,
        values,
        errors,
        notice,
        relation_options,
        UiLanguage::English,
    )
}

pub(super) fn render_form_with_language(
    route: &FormRoute,
    values: &HashMap<String, String>,
    errors: &[FieldError],
    notice: Option<&str>,
    relation_options: &HashMap<String, Vec<SelectOption>>,
    language: UiLanguage,
) -> String {
    let mut html = String::new();
    let form_kind = if route.form.name.ends_with("Create") {
        Some(("form.create_title", "form.create_submit"))
    } else if route.form.name.ends_with("Edit") {
        Some(("form.edit_title", "form.save_submit"))
    } else {
        None
    };
    let generated_resource = form_kind.map(|_| {
        route
            .form
            .name
            .strip_suffix("Create")
            .or_else(|| route.form.name.strip_suffix("Edit"))
            .unwrap_or_else(|| route.form.table.as_deref().unwrap_or("record"))
    });
    let generated_title = form_kind
        .zip(generated_resource)
        .map(|((title_key, _), resource)| {
            field_text(
                language,
                title_key,
                &localized_identifier_reference(language, resource),
            )
        });
    if let Some(title) = route
        .form_view
        .title
        .as_ref()
        .map(|title| localize_user_text(language, title))
        .or(generated_title)
    {
        html.push_str("<h1>");
        html.push_str(&html_escape_preserving_locale_references(&title));
        html.push_str("</h1>");
    }
    let cards = route.form_view.mode == CrudFormViewMode::Cards;
    if cards {
        html.push_str("<section class=\"zelyra-crud-form-card\">");
    }
    html.push_str("<form method=\"post\" action=\"");
    html.push_str(&html_escape(&route.action));
    html.push_str("\">");
    html.push_str("<input type=\"hidden\" name=\"_zelyra_csrf\" value=\"");
    html.push_str(&html_escape(route.csrf.token()));
    html.push_str("\">");
    if let Some(notice) = notice {
        html.push_str("<p class=\"zelyra-notice\">");
        html.push_str(&html_escape_preserving_locale_references(
            &localize_user_text(language, notice),
        ));
        html.push_str("</p>");
    }
    for field in &route.form.fields {
        let label = field
            .label
            .as_deref()
            .map(|label| localize_user_text(language, label))
            .unwrap_or_else(|| localized_identifier_reference(language, &field.name));
        let value = values.get(&field.name).map(String::as_str).unwrap_or("");
        let field_errors = errors
            .iter()
            .filter(|error| error.field == field.name)
            .collect::<Vec<_>>();
        let required = is_required(route, field);
        let max = field_max(route, field);
        html.push_str("<div class=\"zelyra-field\">");
        html.push_str("<label for=\"");
        html.push_str(&html_escape(&field.name));
        html.push_str("\">");
        html.push_str(&html_escape_preserving_locale_references(&label));
        html.push_str("</label>");
        if let Some(options) = relation_options.get(&field.name) {
            html.push_str("<select id=\"");
            html.push_str(&html_escape(&field.name));
            html.push_str("\" name=\"");
            html.push_str(&html_escape(&field.name));
            html.push('"');
            if required {
                html.push_str(" required");
            }
            html.push('>');
            if !required {
                html.push_str(&format!(
                    "<option value=\"\">-- {} --</option>",
                    tr(language, "action.select")
                ));
            }
            for option in options {
                html.push_str("<option value=\"");
                html.push_str(&html_escape(&option.value));
                html.push('"');
                if option.value == value {
                    html.push_str(" selected");
                }
                html.push('>');
                html.push_str(&html_escape(&option.label));
                html.push_str("</option>");
            }
            html.push_str("</select>");
        } else {
            let input_type = input_type(route, field);
            html.push_str("<input id=\"");
            html.push_str(&html_escape(&field.name));
            html.push_str("\" name=\"");
            html.push_str(&html_escape(&field.name));
            html.push_str("\" type=\"");
            html.push_str(input_type);
            html.push('"');
            if input_type == "checkbox" {
                html.push_str(" value=\"true\"");
                if matches!(value, "true" | "1") {
                    html.push_str(" checked");
                }
            } else {
                html.push_str(" value=\"");
                html.push_str(&html_escape(value));
                html.push('"');
            }
            if required {
                html.push_str(" required");
            }
            if let Some(max) = max {
                html.push_str(" maxlength=\"");
                html.push_str(&max.to_string());
                html.push('"');
            }
            if let Some(placeholder) = &field.placeholder {
                html.push_str(" placeholder=\"");
                html.push_str(&html_escape_preserving_locale_references(
                    &localize_user_text(language, placeholder),
                ));
                html.push('"');
            }
            if field.readonly {
                html.push_str(" readonly");
            }
            html.push('>');
        }
        for error in field_errors {
            html.push_str("<p class=\"zelyra-error\">");
            html.push_str(&html_escape_preserving_locale_references(
                &localized_validation_message(language, &error.message),
            ));
            html.push_str("</p>");
        }
        html.push_str("</div>");
    }
    html.push_str("<button type=\"submit\">");
    let default_submit = form_kind
        .map(|(_, submit_key)| tr(language, submit_key))
        .unwrap_or(tr(language, "form.submit"));
    let submit = route
        .form_view
        .submit
        .as_deref()
        .map(|submit| localize_user_text(language, submit))
        .unwrap_or(default_submit);
    html.push_str(&html_escape_preserving_locale_references(&submit));
    html.push_str("</button></form>");
    if cards {
        html.push_str("</section>");
    }
    html
}

pub(super) fn localized_validation_message(language: UiLanguage, message: &str) -> String {
    let key = match message {
        "unknown form field" => "validation.unknown_field",
        "read-only field cannot be submitted" => "validation.readonly",
        "value is required" => "validation.required",
        "must be an integer" => "validation.integer",
        "must be an unsigned integer" => "validation.unsigned_integer",
        "must be a number" => "validation.number",
        "must be true or false" => "validation.boolean",
        "must be a valid email address" => "validation.email",
        "must be a valid HTTP URL" => "validation.url",
        "must be an integer identifier" => "validation.id",
        "must be a UUID" => "validation.uuid",
        _ => {
            if let Some(maximum) = message.strip_prefix("value exceeds maximum length of ") {
                return i18n::parameterized_reference("validation.max_length", "max", maximum);
            }
            return message.to_owned();
        }
    };
    tr(language, key).to_owned()
}

pub(super) fn input_type(route: &FormRoute, field: &zelyra_ast::FormField) -> &'static str {
    if let Some(widget) = field.widget.as_deref() {
        return match widget {
            "email" => "email",
            "number" => "number",
            "url" => "url",
            "checkbox" => "checkbox",
            "date" => "date",
            "time" => "time",
            _ => "text",
        };
    }
    let ty = field.ty.as_ref().or_else(|| {
        route.table.as_ref().and_then(|table| {
            table
                .columns
                .iter()
                .find(|column| column.name == field.name)
                .map(|column| &column.ty)
        })
    });
    match ty {
        Some(Type::Bool) => "checkbox",
        Some(Type::Int | Type::UInt | Type::Float | Type::Decimal) => "number",
        Some(Type::Named(name)) if name == "Email" => "email",
        Some(Type::Named(name)) if name == "Url" => "url",
        _ => "text",
    }
}

pub(super) fn is_required(route: &FormRoute, field: &zelyra_ast::FormField) -> bool {
    if field.required {
        return true;
    }
    let table_required = route.table.as_ref().and_then(|table| {
        table
            .columns
            .iter()
            .find(|column| column.name == field.name)
            .map(|column| column.required || column.primary_key)
    });
    let schema_required = route.schema.as_ref().and_then(|schema| {
        route.form.table.as_deref().and_then(|table_name| {
            schema
                .tables
                .iter()
                .find(|table| table.name == table_name)
                .and_then(|table| {
                    table
                        .columns
                        .iter()
                        .find(|column| column.name == field.name)
                })
                .map(|column| !column.nullable || column.primary_key)
        })
    });
    table_required.unwrap_or(false) || schema_required.unwrap_or(false)
}

pub(super) fn field_max(route: &FormRoute, field: &zelyra_ast::FormField) -> Option<u32> {
    field.max.or_else(|| {
        route.table.as_ref().and_then(|table| {
            table
                .columns
                .iter()
                .find(|column| column.name == field.name)
                .and_then(|column| column.length)
        })
    })
}

pub(super) fn humanize(name: &str) -> String {
    let mut result = name.replace('_', " ");
    if let Some(first) = result.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    result
}

pub fn localized_identifier(language: UiLanguage, name: &str) -> String {
    if let Some(label) = locale_identifier(language, name) {
        return label.to_owned();
    }
    if language == UiLanguage::English {
        return humanize(name);
    }
    let words = name
        .to_ascii_lowercase()
        .split('_')
        .map(|word| locale_identifier(language, word).unwrap_or(word))
        .collect::<Vec<_>>()
        .join(" ");
    let mut characters = words.chars();
    characters.next().map_or(words.clone(), |first| {
        first.to_uppercase().collect::<String>() + characters.as_str()
    })
}

pub(super) fn localized_identifier_reference(language: UiLanguage, name: &str) -> String {
    let key = format!("identifier.{}", name.to_ascii_lowercase());
    if i18n::valid_catalog_key(&key) {
        i18n::reference(&key)
    } else {
        localized_identifier(language, name)
    }
}

pub(super) fn localized_identifier_marker(language: UiLanguage, name: &str) -> String {
    let key = format!("identifier.{}", name.to_ascii_lowercase());
    if i18n::valid_catalog_key(&key) {
        format!("@i18n:{key}")
    } else {
        localized_identifier(language, name)
    }
}
