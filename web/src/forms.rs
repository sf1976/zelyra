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
    let mut template_values = values.clone();
    for (name, value) in values {
        let safe = route
            .form
            .fields
            .iter()
            .find(|field| field.name == *name)
            .is_some_and(|field| {
                !audit_sensitive_field(name)
                    && input_type(route, field) != "password"
                    && input_type(route, field) != "file"
            });
        if safe && route.form_context_fields.iter().any(|field| field == name) {
            template_values.insert(format!("values.{name}"), value.clone());
        }
        if safe
            && route
                .record_context_fields
                .iter()
                .any(|field| field == name)
        {
            template_values.insert(format!("record.{name}"), value.clone());
        }
    }
    let cards = route.form_view.mode == CrudFormViewMode::Cards;
    if cards {
        html.push_str("<section class=\"z-generated-page\">");
    }
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
        html.push_str("<h1 class=\"z-page-header__title\">");
        html.push_str(&html_escape_preserving_locale_references(&title));
        html.push_str("</h1>");
    }
    if !errors.is_empty() {
        html.push_str("<div class=\"z-error-summary\" role=\"alert\" tabindex=\"-1\"><h2>");
        html.push_str(&html_escape(&tr(language, "validation.summary")));
        html.push_str("</h2><ul>");
        for error in errors {
            let message = html_escape_preserving_locale_references(&localized_validation_message(
                language,
                &error.message,
            ));
            if error.field.is_empty() {
                html.push_str("<li>");
                html.push_str(&message);
                html.push_str("</li>");
            } else {
                html.push_str("<li><a href=\"#");
                html.push_str(&html_escape(&error.field));
                html.push_str("\">");
                html.push_str(&message);
                html.push_str("</a></li>");
            }
        }
        html.push_str("</ul></div>");
    }
    if let Some(before) = &route.before_form_html {
        html.push_str(&render_interpolations(
            before,
            &HashMap::new(),
            &template_values,
        ));
    }
    if cards {
        html.push_str("<section class=\"zelyra-crud-form-card\">");
    }
    html.push_str("<form method=\"post\"");
    if route
        .form
        .fields
        .iter()
        .any(|field| field.file_upload.is_some())
    {
        html.push_str(" enctype=\"multipart/form-data\"");
    }
    html.push_str(" action=\"");
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
        // Sensitive values must never be reflected into HTML, including after
        // a validation error or on an edit form with a stored password hash.
        let value = if matches!(input_type(route, field), "password" | "file")
            || audit_sensitive_field(&field.name)
        {
            ""
        } else {
            values.get(&field.name).map(String::as_str).unwrap_or("")
        };
        let field_errors = errors
            .iter()
            .filter(|error| error.field == field.name)
            .collect::<Vec<_>>();
        let required = is_required(route, field);
        let max = field_max(route, field);
        let field_input_type = input_type(route, field);
        let is_checkbox =
            !relation_options.contains_key(&field.name) && field_input_type == "checkbox";
        html.push_str(if is_checkbox {
            "<div class=\"z-field zelyra-field z-field--checkbox\">"
        } else {
            "<div class=\"z-field zelyra-field\">"
        });
        html.push_str("<label for=\"");
        html.push_str(&html_escape(&field.name));
        html.push_str("\" class=\"z-field__label\">");
        html.push_str(&html_escape_preserving_locale_references(&label));
        html.push_str("</label>");
        if let Some(options) = relation_options.get(&field.name) {
            html.push_str("<select class=\"");
            html.push_str(&form_field_appearance_classes(field, "z-select"));
            html.push_str("\" id=\"");
            html.push_str(&html_escape(&field.name));
            html.push_str("\" name=\"");
            html.push_str(&html_escape(&field.name));
            html.push('"');
            if required {
                html.push_str(" required");
            }
            if !field_errors.is_empty() {
                html.push_str(" aria-invalid=\"true\" aria-describedby=\"");
                html.push_str(&format!("{}-error", html_escape(&field.name)));
                html.push('"');
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
        } else if !field.choices.is_empty() {
            html.push_str("<select class=\"");
            html.push_str(&form_field_appearance_classes(field, "z-select"));
            html.push_str("\" id=\"");
            html.push_str(&html_escape(&field.name));
            html.push_str("\" name=\"");
            html.push_str(&html_escape(&field.name));
            html.push('"');
            if required {
                html.push_str(" required");
            }
            if !field_errors.is_empty() {
                html.push_str(" aria-invalid=\"true\" aria-describedby=\"");
                html.push_str(&format!("{}-error", html_escape(&field.name)));
                html.push('"');
            }
            html.push('>');
            if !required {
                html.push_str(&format!(
                    "<option value=\"\">-- {} --</option>",
                    tr(language, "action.select")
                ));
            }
            for choice in &field.choices {
                html.push_str("<option value=\"");
                html.push_str(&html_escape(choice));
                html.push('"');
                if choice == value {
                    html.push_str(" selected");
                }
                html.push('>');
                html.push_str(&html_escape(choice));
                html.push_str("</option>");
            }
            html.push_str("</select>");
        } else {
            let input_type = field_input_type;
            html.push_str("<input class=\"");
            html.push_str(&form_field_appearance_classes(
                field,
                if input_type == "checkbox" {
                    "z-check"
                } else {
                    "z-input"
                },
            ));
            html.push_str("\" id=\"");
            html.push_str(&html_escape(&field.name));
            html.push_str("\" name=\"");
            html.push_str(&html_escape(&field.name));
            html.push_str("\" type=\"");
            html.push_str(input_type);
            if let Some(upload) = &field.file_upload {
                html.push_str("\" accept=\"");
                html.push_str(&html_escape(&upload.allowed_types.join(",")));
            }
            html.push('"');
            let ty = zelyra_forms::field_value_type(
                &route.form,
                route.table.as_ref(),
                route.schema.as_ref(),
                field,
            );
            let ty = ty.as_ref().map(|ty| match ty {
                Type::Option(inner) => inner.as_ref(),
                other => other,
            });
            if matches!(ty, Some(Type::Decimal))
                || matches!(ty, Some(Type::Named(name)) if name == "Money")
            {
                html.push_str(" inputmode=\"decimal\"");
            }
            if input_type == "number" && matches!(ty, Some(Type::Float | Type::Decimal)) {
                html.push_str(" step=\"any\"");
            }
            if input_type == "checkbox" {
                html.push_str(" value=\"true\"");
                if matches!(value, "true" | "1") {
                    html.push_str(" checked");
                }
            } else if input_type != "file" {
                html.push_str(" value=\"");
                html.push_str(&html_escape(value));
                html.push('"');
            }
            if required && !field.readonly {
                html.push_str(" required");
            }
            if let Some(max) = max {
                html.push_str(" maxlength=\"");
                html.push_str(&max.to_string());
                html.push('"');
            }
            if let Some(min_length) = field.min_length {
                html.push_str(" minlength=\"");
                html.push_str(&min_length.to_string());
                html.push('"');
            }
            if input_type == "number" {
                if let Some(min) = &field.min_value {
                    html.push_str(" min=\"");
                    html.push_str(&html_escape(min));
                    html.push('"');
                }
                if let Some(max) = &field.max_value {
                    html.push_str(" max=\"");
                    html.push_str(&html_escape(max));
                    html.push('"');
                }
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
            if !field_errors.is_empty() {
                html.push_str(" aria-invalid=\"true\" aria-describedby=\"");
                html.push_str(&format!("{}-error", html_escape(&field.name)));
                if input_type == "date" {
                    html.push(' ');
                    html.push_str(&format!("{}-hint", html_escape(&field.name)));
                }
                html.push('"');
            } else if input_type == "date" {
                html.push_str(" aria-describedby=\"");
                html.push_str(&format!("{}-hint\"", html_escape(&field.name)));
            }
            html.push('>');
            if input_type == "date" {
                let hint = if language == UiLanguage::German {
                    "Format: JJJJ-MM-TT"
                } else {
                    "Format: YYYY-MM-DD"
                };
                html.push_str("<small class=\"z-field__hint zelyra-field-hint\" id=\"");
                html.push_str(&format!("{}-hint\">", html_escape(&field.name)));
                html.push_str(hint);
                html.push_str("</small>");
            }
        }
        for (error_index, error) in field_errors.into_iter().enumerate() {
            html.push_str("<p class=\"z-field__error zelyra-error\" id=\"");
            html.push_str(&format!("{}-error", html_escape(&field.name)));
            if error_index > 0 {
                html.push('-');
                html.push_str(&error_index.to_string());
            }
            html.push_str("\">");
            html.push_str(&html_escape_preserving_locale_references(
                &localized_validation_message(language, &error.message),
            ));
            html.push_str("</p>");
        }
        if let Some(content) = route.field_content_html.get(&field.name) {
            html.push_str("<div class=\"z-field__supplemental\">");
            html.push_str(&render_interpolations(
                content,
                &HashMap::new(),
                &template_values,
            ));
            html.push_str("</div>");
        }
        html.push_str("</div>");
    }
    for child in &route.child_collections {
        render_child_collection(&mut html, child, values, errors, relation_options, language);
    }
    html.push_str("<button class=\"z-button z-button--primary\" type=\"submit\">");
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
    if let Some(after) = &route.after_form_html {
        html.push_str(&render_interpolations(
            after,
            &HashMap::new(),
            &template_values,
        ));
    }
    if cards {
        html.push_str("</section>");
        html.push_str("</section>");
    }
    html
}

pub(super) const MAX_CRUD_CHILD_ROWS: usize = 50;

fn render_child_collection(
    html: &mut String,
    child: &CrudChildFormRoute,
    values: &HashMap<String, String>,
    errors: &[FieldError],
    relation_options: &HashMap<String, Vec<SelectOption>>,
    language: UiLanguage,
) {
    let count = values
        .get(&child.count_key)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
        .clamp(1, MAX_CRUD_CHILD_ROWS);
    html.push_str("<fieldset class=\"z-fieldset z-crud-child-collection\"><legend>");
    html.push_str(&html_escape(&localized_identifier_reference(
        language,
        &child.definition.name,
    )));
    html.push_str("</legend>");
    for row in 0..count {
        html.push_str("<fieldset class=\"z-crud-child-row\"><legend>");
        html.push_str(&html_escape(&format!(
            "{} {}",
            localized_identifier_reference(language, &child.definition.name),
            row + 1
        )));
        html.push_str("</legend>");
        for field in &child.fields {
            let key = crud_child_value_key(&child.definition.name, row, &field.name);
            let input_type = child_field_input_type(field);
            let value = if input_type == "password" {
                ""
            } else {
                values.get(&key).map(String::as_str).unwrap_or("")
            };
            let label = field
                .label
                .as_deref()
                .map(|label| localize_user_text(language, label))
                .unwrap_or_else(|| localized_identifier_reference(language, &field.name));
            let field_errors = errors
                .iter()
                .filter(|error| error.field == key)
                .collect::<Vec<_>>();
            html.push_str(
                "<div class=\"z-field zelyra-field\"><label class=\"z-field__label\" for=\"",
            );
            html.push_str(&html_escape(&key));
            html.push_str("\">");
            html.push_str(&html_escape_preserving_locale_references(&label));
            html.push_str("</label>");
            let child_relation_key = format!("{}.{}", child.definition.name, field.name);
            if let Some(options) = relation_options
                .get(&child_relation_key)
                .or_else(|| relation_options.get(&field.name))
            {
                html.push_str("<select class=\"z-select\" id=\"");
                html.push_str(&html_escape(&key));
                html.push_str("\" name=\"");
                html.push_str(&html_escape(&key));
                html.push('"');
                if field.required {
                    html.push_str(" required");
                }
                if !field_errors.is_empty() {
                    html.push_str(" aria-invalid=\"true\"");
                }
                html.push('>');
                if !field.required {
                    html.push_str("<option value=\"\">-- ");
                    html.push_str(&html_escape(&tr(language, "action.select")));
                    html.push_str(" --</option>");
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
                html.push_str("<input class=\"");
                html.push_str(if input_type == "checkbox" {
                    "z-check"
                } else {
                    "z-input"
                });
                html.push_str("\" id=\"");
                html.push_str(&html_escape(&key));
                html.push_str("\" name=\"");
                html.push_str(&html_escape(&key));
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
                if field.required {
                    html.push_str(" required");
                }
                if input_type == "number" && matches!(field.ty, Some(Type::Float | Type::Decimal)) {
                    html.push_str(" step=\"any\"");
                }
                if input_type == "number"
                    && (matches!(field.ty.as_ref(), Some(Type::Decimal))
                        || matches!(field.ty.as_ref(), Some(Type::Named(name)) if name == "Money"))
                {
                    html.push_str(" inputmode=\"decimal\"");
                }
                if !field_errors.is_empty() {
                    html.push_str(" aria-invalid=\"true\"");
                }
                html.push('>');
            }
            for error in field_errors {
                html.push_str("<p class=\"z-field__error zelyra-error\">");
                html.push_str(&html_escape_preserving_locale_references(
                    &localized_validation_message(language, &error.message),
                ));
                html.push_str("</p>");
            }
            html.push_str("</div>");
        }
        if row > 0 {
            html.push_str("<button class=\"z-button z-button--secondary\" type=\"submit\" name=\"_zelyra_child_action\" value=\"");
            if child.count_key != "_zelyra_child_count" {
                html.push_str(&html_escape(&child.definition.name));
                html.push(':');
            }
            html.push_str("remove:");
            html.push_str(&row.to_string());
            html.push_str("\"> ");
            html.push_str(&html_escape(if language == UiLanguage::German {
                "Position entfernen"
            } else {
                "Remove line"
            }));
            html.push_str("</button>");
        }
        html.push_str("</fieldset>");
    }
    html.push_str("<input type=\"hidden\" name=\"");
    html.push_str(&html_escape(&child.count_key));
    html.push_str("\" value=\"");
    html.push_str(&count.to_string());
    html.push_str("\"><button class=\"z-button z-button--secondary\" type=\"submit\" name=\"_zelyra_child_action\" value=\"");
    if child.count_key != "_zelyra_child_count" {
        html.push_str(&html_escape(&child.definition.name));
        html.push(':');
    }
    html.push_str("add\">");
    html.push_str(&html_escape(if language == UiLanguage::German {
        "Position hinzufügen"
    } else {
        "Add line"
    }));
    html.push_str("</button></fieldset>");
}

pub(super) fn crud_child_value_key(collection: &str, row: usize, field: &str) -> String {
    format!("{collection}.{row}.{field}")
}

fn child_field_input_type(field: &zelyra_ast::FormField) -> &'static str {
    match field.ty.as_ref().map(|ty| match ty {
        Type::Option(inner) => inner.as_ref(),
        other => other,
    }) {
        Some(Type::Int | Type::UInt | Type::Float | Type::Decimal) => "number",
        Some(Type::Bool) => "checkbox",
        Some(Type::Date) => "date",
        Some(Type::Time) => "time",
        Some(Type::Timestamp) => "datetime-local",
        Some(Type::Named(name)) if name == "Money" => "number",
        Some(Type::Named(name)) if name == "Email" => "email",
        Some(Type::Named(name)) if name == "Url" => "url",
        _ => "text",
    }
}

fn form_field_appearance_classes(field: &zelyra_ast::FormField, control: &str) -> String {
    let mut classes = control.to_owned();
    match field.appearance.as_deref() {
        Some("success") => classes.push_str(&format!(" {control}--success")),
        Some("warning") => classes.push_str(&format!(" {control}--warning")),
        Some("danger") => classes.push_str(&format!(" {control}--danger")),
        _ => classes.push_str(&format!(" {control}--default")),
    }
    classes
}

pub(super) fn localized_validation_message(language: UiLanguage, message: &str) -> String {
    let key = match message {
        "unknown form field" => "validation.unknown_field",
        "read-only field cannot be submitted" => "validation.readonly",
        "value is required" => "validation.required",
        "value is below the minimum" => "validation.min_value",
        "value exceeds the maximum" => "validation.max_value",
        "value is not an allowed choice" => "validation.choice",
        "file inputs require an HTTP multipart upload route" => "validation.file_upload_unavailable",
        "must be an integer" => "validation.integer",
        "must be an unsigned integer" => "validation.unsigned_integer",
        "must be a number" => "validation.number",
        "must be an exact Decimal with at most 9 fractional digits" => "validation.decimal",
        "must be an exact Money amount with at most 15 integer and 4 fractional digits" => {
            "validation.money"
        }
        "must be true or false" => "validation.boolean",
        "must be a valid Date in YYYY-MM-DD format" => "validation.date",
        "must be a valid Time in HH:MM:SS format" => "validation.time",
        "must be a Timestamp in UTC ISO format or Unix milliseconds within years 0001 through 9999" => "validation.timestamp",
        "must be an integer duration in milliseconds" => "validation.duration",
        "must be a valid email address" => "validation.email",
        "must be a valid HTTP URL" => "validation.url",
        "must be an integer identifier" => "validation.id",
        "must be a UUID" => "validation.uuid",
        _ => {
            if let Some(maximum) = message.strip_prefix("value exceeds maximum length of ") {
                return i18n::parameterized_reference("validation.max_length", "max", maximum);
            }
            if let Some(minimum) = message.strip_prefix("value is shorter than minimum length of ") {
                return i18n::parameterized_reference("validation.min_length", "min", minimum);
            }
            return message.to_owned();
        }
    };
    tr(language, key).to_owned()
}

pub(super) fn input_type(route: &FormRoute, field: &zelyra_ast::FormField) -> &'static str {
    if field.file_upload.is_some() {
        return "file";
    }
    if let Some(widget) = field.widget.as_deref() {
        return match widget {
            "email" => "email",
            "number" => "number",
            "url" => "url",
            "checkbox" => "checkbox",
            "date" => "date",
            "time" => "time",
            "password" => "password",
            _ => "text",
        };
    }
    let ty = zelyra_forms::field_value_type(
        &route.form,
        route.table.as_ref(),
        route.schema.as_ref(),
        field,
    );
    let ty = ty.as_ref().map(|ty| match ty {
        Type::Option(inner) => inner.as_ref(),
        other => other,
    });
    match ty {
        Some(Type::Bool) => "checkbox",
        Some(Type::Date) => "date",
        Some(Type::Int | Type::UInt | Type::Float) => "number",
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
