use std::collections::HashSet;

use serde_json::{json, Value};
use zelyra_ast::{Program, Span};
use zelyra_lexer::{Token, TokenKind};

pub struct EditPreview {
    pub source: String,
    pub operations: Value,
    pub changes: Value,
    pub changed_tokens: usize,
}

pub fn request_entry(request: &Value) -> Result<String, String> {
    request
        .get("entry")
        .and_then(Value::as_str)
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "request must contain a non-empty string `entry`".into())
}

pub fn preview(
    program: &Program,
    source: &str,
    tokens: &[Token],
    request: &Value,
) -> Result<EditPreview, String> {
    let operations = request
        .get("operations")
        .and_then(Value::as_array)
        .ok_or_else(|| "request must contain an `operations` array".to_owned())?;
    if operations.is_empty() {
        return Err("request must contain at least one operation".into());
    }

    let mut renames = Vec::with_capacity(operations.len());
    let mut sources = HashSet::new();
    for operation in operations {
        let kind = operation
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| "each operation needs a string `kind`".to_owned())?;
        if kind != "rename" {
            return Err(format!(
                "unsupported edit operation `{kind}`; only `rename` is available"
            ));
        }
        let symbol = operation
            .get("symbol")
            .and_then(Value::as_str)
            .ok_or_else(|| "rename operations need a string `symbol`".to_owned())?;
        let from = operation
            .get("from")
            .and_then(Value::as_str)
            .ok_or_else(|| "rename operations need a string `from`".to_owned())?;
        let to = operation
            .get("to")
            .and_then(Value::as_str)
            .ok_or_else(|| "rename operations need a string `to`".to_owned())?;
        if !valid_identifier(from) || !valid_identifier(to) {
            return Err("rename `from` and `to` must be valid Zelyra identifiers".into());
        }
        if from == to {
            return Err(format!("rename `{from}` to itself is not an edit"));
        }
        if !sources.insert(from.to_owned()) {
            return Err(format!("rename source `{from}` occurs more than once"));
        }
        if !declares_symbol(program, symbol, from) {
            return Err(format!("no declared {symbol} named `{from}` exists"));
        }
        renames.push((from.to_owned(), to.to_owned()));
    }

    let mut replacements = Vec::new();
    for token in tokens {
        let TokenKind::Ident(name) = &token.kind else {
            continue;
        };
        let Some((_, replacement)) = renames.iter().find(|(source_name, _)| source_name == name)
        else {
            continue;
        };
        replacements.push((token.span, replacement.clone(), name.clone()));
    }

    let mut updated_source = source.to_owned();
    for (span, replacement, _) in replacements.iter().rev() {
        updated_source.replace_range(span.start..span.end, replacement);
    }

    let changes = replacements
        .iter()
        .map(|(span, replacement, original)| {
            json!({
                "from": original,
                "to": replacement,
                "span": span_value(source, *span),
            })
        })
        .collect::<Vec<_>>();

    Ok(EditPreview {
        source: updated_source,
        operations: Value::Array(operations.clone()),
        changes: Value::Array(changes),
        changed_tokens: replacements.len(),
    })
}

fn declares_symbol(program: &Program, symbol: &str, name: &str) -> bool {
    match symbol {
        "function" => program.functions.iter().any(|item| item.name == name),
        "table" => program.tables.iter().any(|item| item.name == name),
        "form" => program.forms.iter().any(|item| item.name == name),
        "crud" => program.cruds.iter().any(|item| item.name == name),
        "view" => program.views.iter().any(|item| item.name == name),
        "component" => program.components.iter().any(|item| item.name == name),
        _ => false,
    }
}

fn valid_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn span_value(source: &str, span: Span) -> Value {
    let (end_line, end_column) = source_position(source, span.end);
    json!({
        "start": { "offset": span.start, "line": span.line, "column": span.column },
        "end": { "offset": span.end, "line": end_line, "column": end_column }
    })
}

fn source_position(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
    for character in source[..offset.min(source.len())].chars() {
        if character == '\n' {
            line += 1;
            column = 1;
        } else {
            column += character.len_utf8();
        }
    }
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zelyra_lexer::lex;
    use zelyra_parser::parse;

    #[test]
    fn previews_an_atomic_symbol_rename() {
        let source = "fn greet() { greet() }\n";
        let tokens = lex(source).expect("source should lex");
        let program = parse(&tokens).expect("source should parse");
        let request = json!({
            "entry": "main.zyl",
            "operations": [{
                "kind": "rename",
                "symbol": "function",
                "from": "greet",
                "to": "welcome"
            }]
        });
        let preview = preview(&program, source, &tokens, &request).expect("edit should preview");
        assert_eq!(preview.source, "fn welcome() { welcome() }\n");
        assert_eq!(preview.changed_tokens, 2);
        assert_eq!(preview.changes.as_array().unwrap().len(), 2);
    }
}
