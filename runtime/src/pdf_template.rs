use super::Value;

const MAX_PDF_TEMPLATE_BYTES: usize = 2 * 1024 * 1024;
pub(super) const MAX_PDF_TEMPLATE_ROWS: usize = 10_000;
pub(super) const PDF_TEMPLATE_LIMIT_ERROR: &str = "PDF view exceeds its rendering limit";

pub(super) fn interpolate_pdf_data(template: &str, data: &Value) -> Result<String, &'static str> {
    if template.len() > MAX_PDF_TEMPLATE_BYTES {
        return Err(PDF_TEMPLATE_LIMIT_ERROR);
    }
    let mut context = std::collections::BTreeMap::new();
    match data {
        Value::Object { fields, .. } => context.extend(fields.clone()),
        Value::Map(entries) => {
            for (key, value) in entries {
                if let Value::String(key) = key {
                    context.insert(key.clone(), value.clone());
                }
            }
        }
        _ => {}
    }
    let mut output = String::with_capacity(template.len());
    let mut row_budget = MAX_PDF_TEMPLATE_ROWS;
    render_pdf_template_fragment(template, &context, &mut output, &mut row_budget)?;
    Ok(output)
}

fn render_pdf_template_fragment(
    template: &str,
    context: &std::collections::BTreeMap<String, Value>,
    output: &mut String,
    row_budget: &mut usize,
) -> Result<(), &'static str> {
    if let Some(block) = next_pdf_template_for_block(template) {
        interpolate_pdf_values(&template[..block.start], context, output)?;
        if let Some(Value::Array(rows)) = context.get(&block.collection) {
            for row in rows {
                if *row_budget == 0 {
                    return Err("PDF view exceeds its 10000 row rendering limit");
                }
                *row_budget -= 1;
                let mut row_context = context.clone();
                row_context.insert(block.item.clone(), row.clone());
                render_pdf_template_fragment(&block.body, &row_context, output, row_budget)?;
            }
        }
        return render_pdf_template_fragment(&template[block.end..], context, output, row_budget);
    }
    interpolate_pdf_values(template, context, output)
}

struct PdfTemplateForBlock {
    start: usize,
    end: usize,
    item: String,
    collection: String,
    body: String,
}

fn next_pdf_template_for_block(template: &str) -> Option<PdfTemplateForBlock> {
    let mut search_from = 0;
    while let Some(relative_start) = template.get(search_from..)?.find("for ") {
        let start = search_from + relative_start;
        let line_start = template[..start].rfind('\n').map_or(0, |index| index + 1);
        if !template[line_start..start].trim().is_empty() {
            search_from = start + 4;
            continue;
        }
        let relative_open = template[start..].find('{')?;
        let open = start + relative_open;
        let header = template[start + 4..open].trim();
        let parts = header.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 3
            || parts[1] != "in"
            || !pdf_template_identifier(parts[0])
            || !pdf_template_identifier(parts[2])
        {
            search_from = open + 1;
            continue;
        }
        let mut depth = 1;
        let mut position = open + 1;
        while position < template.len() {
            match template.as_bytes()[position] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(PdfTemplateForBlock {
                            start,
                            end: position + 1,
                            item: parts[0].to_owned(),
                            collection: parts[2].to_owned(),
                            body: template[open + 1..position].to_owned(),
                        });
                    }
                }
                _ => {}
            }
            let character = template[position..].chars().next()?;
            position += character.len_utf8();
        }
        return None;
    }
    None
}

fn pdf_template_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.chars().enumerate().all(|(index, character)| {
            if index == 0 {
                character.is_ascii_alphabetic() || character == '_'
            } else {
                character.is_ascii_alphanumeric() || character == '_'
            }
        })
}

fn interpolate_pdf_values(
    template: &str,
    context: &std::collections::BTreeMap<String, Value>,
    output: &mut String,
) -> Result<(), &'static str> {
    let mut values = std::collections::BTreeMap::new();
    for (name, value) in context {
        collect_pdf_values(value, name, &mut values, 0)?;
    }
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        append_pdf_template(output, &rest[..open])?;
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            append_pdf_template(output, &rest[open..])?;
            return Ok(());
        };
        let name = &after_open[..close];
        if let Some(value) = values.get(name) {
            append_pdf_html_escaped(output, value)?;
        } else {
            append_pdf_template(output, "{")?;
            append_pdf_template(output, name)?;
            append_pdf_template(output, "}")?;
        }
        rest = &after_open[close + 1..];
    }
    append_pdf_template(output, rest)
}

fn append_pdf_template(output: &mut String, value: &str) -> Result<(), &'static str> {
    if output.len().saturating_add(value.len()) > MAX_PDF_TEMPLATE_BYTES {
        return Err(PDF_TEMPLATE_LIMIT_ERROR);
    }
    output.push_str(value);
    Ok(())
}

fn collect_pdf_values(
    value: &Value,
    prefix: &str,
    values: &mut std::collections::BTreeMap<String, String>,
    depth: usize,
) -> Result<(), &'static str> {
    if depth > 16 {
        return Ok(());
    }
    match value {
        Value::Object { fields, .. } => {
            let mut fields = fields.iter().collect::<Vec<_>>();
            fields.sort_by(|left, right| left.0.cmp(right.0));
            for (name, field) in fields {
                let key = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}.{name}")
                };
                collect_pdf_values(field, &key, values, depth + 1)?;
            }
        }
        Value::Map(entries) => {
            let mut entries = entries
                .iter()
                .filter_map(|(key, value)| match key {
                    Value::String(key) => Some((key, value)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            for (name, field) in entries {
                let key = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}.{name}")
                };
                collect_pdf_values(field, &key, values, depth + 1)?;
            }
        }
        Value::Option(Some(inner)) => collect_pdf_values(inner, prefix, values, depth + 1)?,
        Value::Option(None) => {
            if !prefix.is_empty() {
                values.insert(prefix.to_owned(), String::new());
            }
        }
        Value::String(text) if !prefix.is_empty() => {
            if text.len() > MAX_PDF_TEMPLATE_BYTES {
                return Err(PDF_TEMPLATE_LIMIT_ERROR);
            }
            values.insert(prefix.to_owned(), text.clone());
        }
        Value::Int(_)
        | Value::UInt(_)
        | Value::Float(_)
        | Value::Decimal(_)
        | Value::Bool(_)
        | Value::Char(_)
        | Value::Timestamp(_)
        | Value::Date(_)
        | Value::Time(_)
        | Value::Duration(_)
        | Value::Unit
            if !prefix.is_empty() =>
        {
            let rendered = value.output();
            if rendered.len() > MAX_PDF_TEMPLATE_BYTES {
                return Err(PDF_TEMPLATE_LIMIT_ERROR);
            }
            values.insert(prefix.to_owned(), rendered);
        }
        Value::Enum { variant, .. } if !prefix.is_empty() => {
            values.insert(prefix.to_owned(), variant.clone());
        }
        _ => {}
    }
    Ok(())
}

fn append_pdf_html_escaped(output: &mut String, value: &str) -> Result<(), &'static str> {
    for character in value.chars() {
        append_pdf_template(
            output,
            match character {
                '&' => "&amp;",
                '<' => "&lt;",
                '>' => "&gt;",
                '"' => "&quot;",
                '\'' => "&#39;",
                _ => {
                    let mut encoded = [0; 4];
                    append_pdf_template(output, character.encode_utf8(&mut encoded))?;
                    continue;
                }
            },
        )?;
    }
    Ok(())
}
