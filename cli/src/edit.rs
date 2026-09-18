use std::{collections::HashSet, fs, io::Write, path::Path};

use serde_json::{json, Value};
use zelyra_ast::{Program, Span};
use zelyra_lexer::{Token, TokenKind};

pub struct EditPreview {
    pub source: String,
    pub operations: Value,
    pub changes: Value,
    pub changed_tokens: usize,
}

pub const EDIT_SCHEMA_VERSION: &str = "1";

pub fn source_fingerprint(source: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3_u64);
    }
    format!("fnv1a64:{hash:016x}")
}

pub fn apply_atomically(path: &str, source: &str) -> Result<(), String> {
    let target = Path::new(path);
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("cannot determine file name for `{path}`"))?;
    let temporary = parent.join(format!(
        ".{file_name}.zelyra-edit-{}.tmp",
        std::process::id()
    ));

    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("cannot create temporary edit file: {error}"))?;
        file.write_all(source.as_bytes())
            .map_err(|error| format!("cannot write temporary edit file: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("cannot flush temporary edit file: {error}"))?;
        let permissions = fs::metadata(target)
            .map_err(|error| format!("cannot inspect `{path}`: {error}"))?
            .permissions();
        fs::set_permissions(&temporary, permissions)
            .map_err(|error| format!("cannot preserve permissions for `{path}`: {error}"))?;
        fs::rename(&temporary, target)
            .map_err(|error| format!("cannot atomically replace `{path}`: {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub fn request_entry(request: &Value) -> Result<String, String> {
    request
        .get("entry")
        .and_then(Value::as_str)
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "request must contain a non-empty string `entry`".into())
}

pub fn validate_request(request: &Value) -> Result<(), String> {
    let schema_version = request
        .get("schema_version")
        .and_then(Value::as_str)
        .ok_or_else(|| "request must contain string `schema_version`".to_owned())?;
    if schema_version != EDIT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported edit request schema `{schema_version}`; expected `{EDIT_SCHEMA_VERSION}`"
        ));
    }
    Ok(())
}

pub fn resolve_entry(entry: &str) -> Result<(String, String), String> {
    let path = Path::new(entry);
    if path.extension().and_then(|extension| extension.to_str()) != Some("zyl") {
        return Err("edit entry must be a `.zyl` source file".into());
    }
    let canonical_entry = fs::canonicalize(path)
        .map_err(|error| format!("cannot resolve edit entry `{entry}`: {error}"))?;
    let mut directory = canonical_entry
        .parent()
        .ok_or_else(|| "edit entry has no parent directory".to_owned())?;
    let project_root = loop {
        if directory.join("zelyra.toml").is_file() {
            break directory;
        }
        let Some(parent) = directory.parent() else {
            return Err("edit entry is not inside a Zelyra project".into());
        };
        if parent == directory {
            return Err("edit entry is not inside a Zelyra project".into());
        }
        directory = parent;
    };
    if !canonical_entry.starts_with(project_root) {
        return Err("edit entry is outside the Zelyra project root".into());
    }
    let relative = canonical_entry
        .strip_prefix(project_root)
        .map_err(|_| "edit entry is outside the Zelyra project root".to_owned())?
        .to_string_lossy()
        .replace('\\', "/");
    Ok((canonical_entry.to_string_lossy().into_owned(), relative))
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
        renames.push((symbol.to_owned(), from.to_owned(), to.to_owned()));
    }

    let mut replacements = Vec::new();
    for (symbol, source_name, replacement) in &renames {
        let semantic_spans = match symbol.as_str() {
            "function" => Some(function_rename_spans(program, tokens, source_name)),
            "type" => Some(type_rename_spans(tokens, source_name)),
            "record" => Some(record_rename_spans(tokens, source_name)),
            "table" | "view" | "form" | "crud" | "tableview" => {
                Some(resource_rename_spans(tokens, symbol, source_name))
            }
            "component" => Some(component_rename_spans(tokens, source_name)),
            _ => None,
        };
        for token in tokens {
            let TokenKind::Ident(name) = &token.kind else {
                continue;
            };
            if name != source_name {
                continue;
            }
            if semantic_spans
                .as_ref()
                .is_some_and(|spans| spans.contains(&(token.span.start, token.span.end)))
                || semantic_spans.is_none()
            {
                replacements.push((token.span, replacement.clone(), name.clone()));
            }
        }
        if symbol == "component" {
            if let Some(spans) = semantic_spans.as_ref() {
                for (start, end) in spans {
                    if tokens
                        .iter()
                        .any(|token| token.span.start == *start && token.span.end == *end)
                    {
                        continue;
                    }
                    let (line, column) = source_position(source, *start);
                    replacements.push((
                        Span::new(*start, *end, line, column),
                        replacement.clone(),
                        source_name.clone(),
                    ));
                }
            }
        }
        if symbol == "table" {
            for token in tokens {
                let TokenKind::SqlBody(query) = &token.kind else {
                    continue;
                };
                for (start, end) in sql_table_reference_spans(query, source_name) {
                    let absolute_start = token.span.start + start;
                    let absolute_end = token.span.start + end;
                    let (line, column) = source_position(source, absolute_start);
                    replacements.push((
                        Span::new(absolute_start, absolute_end, line, column),
                        replacement.clone(),
                        source_name.clone(),
                    ));
                }
            }
        }
    }

    replacements.sort_by_key(|(span, _, _)| span.start);
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

fn function_rename_spans(
    program: &Program,
    tokens: &[Token],
    name: &str,
) -> HashSet<(usize, usize)> {
    let mut spans = HashSet::new();
    for function in &program.functions {
        if function.name == name {
            if let Some(span) = declaration_name_span(tokens, function.span, TokenKind::Fn, name) {
                spans.insert((span.start, span.end));
            }
        }
        collect_function_block(&function.body, name, &mut spans);
        for expression in &function.requires {
            collect_function_expression(expression, name, &mut spans);
        }
        for expression in &function.ensures {
            collect_function_expression(expression, name, &mut spans);
        }
    }
    spans
}

fn type_rename_spans(tokens: &[Token], name: &str) -> HashSet<(usize, usize)> {
    let mut spans = type_reference_spans(tokens, name);
    if let Some(span) = any_declaration_name_span(tokens, TokenKind::Type, name) {
        spans.insert((span.start, span.end));
    }
    spans
}

fn record_rename_spans(tokens: &[Token], name: &str) -> HashSet<(usize, usize)> {
    let mut spans = type_reference_spans(tokens, name);
    if let Some(span) = any_declaration_name_span(tokens, TokenKind::Struct, name) {
        spans.insert((span.start, span.end));
    }
    spans.extend(record_literal_spans(tokens, name));
    spans
}

fn resource_rename_spans(tokens: &[Token], symbol: &str, name: &str) -> HashSet<(usize, usize)> {
    let keyword = match symbol {
        "table" => TokenKind::Table,
        "view" => TokenKind::View,
        "form" => TokenKind::Form,
        "crud" => TokenKind::Crud,
        "tableview" => TokenKind::TableView,
        _ => return HashSet::new(),
    };
    let mut spans = HashSet::new();
    if let Some(span) = any_declaration_name_span(tokens, keyword, name) {
        spans.insert((span.start, span.end));
    }
    match symbol {
        "table" => {
            spans.extend(header_arrow_reference_spans(
                tokens,
                &[TokenKind::Form, TokenKind::Crud],
                name,
            ));
            spans.extend(property_reference_spans(tokens, TokenKind::Table, name));
        }
        "view" => {
            spans.extend(property_reference_spans(tokens, TokenKind::View, name));
        }
        _ => {}
    }
    spans
}

fn component_rename_spans(tokens: &[Token], name: &str) -> HashSet<(usize, usize)> {
    let mut spans = HashSet::new();
    if let Some(span) = any_declaration_name_span(tokens, TokenKind::Component, name) {
        spans.insert((span.start, span.end));
    }
    for token in tokens {
        let TokenKind::HtmlBody(body) = &token.kind else {
            continue;
        };
        for (start, end) in html_component_name_spans(body, name) {
            spans.insert((token.span.start + start, token.span.start + end));
        }
    }
    spans
}

fn html_component_name_spans(html: &str, name: &str) -> Vec<(usize, usize)> {
    let bytes = html.as_bytes();
    let name_bytes = name.as_bytes();
    let mut spans = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'<' {
            index += 1;
            continue;
        }
        let name_start = if bytes.get(index + 1) == Some(&b'/') {
            index + 2
        } else {
            index + 1
        };
        let name_end = name_start.saturating_add(name_bytes.len());
        let boundary = bytes.get(name_end).copied();
        if name_end <= bytes.len()
            && &bytes[name_start..name_end] == name_bytes
            && boundary
                .is_some_and(|byte| byte.is_ascii_whitespace() || byte == b'/' || byte == b'>')
        {
            spans.push((name_start, name_end));
            index = name_end;
        } else {
            index += 1;
        }
    }
    spans
}

fn header_arrow_reference_spans(
    tokens: &[Token],
    headers: &[TokenKind],
    name: &str,
) -> HashSet<(usize, usize)> {
    let mut spans = HashSet::new();
    for (index, token) in tokens.iter().enumerate() {
        let TokenKind::Ident(candidate) = &token.kind else {
            continue;
        };
        if candidate != name {
            continue;
        }
        let Some(arrow_index) = previous_token_index(tokens, index) else {
            continue;
        };
        if tokens[arrow_index].kind != TokenKind::Arrow {
            continue;
        }
        let Some(owner_index) = previous_token_index(tokens, arrow_index) else {
            continue;
        };
        let Some(header_index) = previous_token_index(tokens, owner_index) else {
            continue;
        };
        if headers
            .iter()
            .any(|header| tokens[header_index].kind == *header)
        {
            spans.insert((token.span.start, token.span.end));
        }
    }
    spans
}

fn property_reference_spans(
    tokens: &[Token],
    property: TokenKind,
    name: &str,
) -> HashSet<(usize, usize)> {
    let mut spans = HashSet::new();
    for (index, token) in tokens.iter().enumerate() {
        let TokenKind::Ident(candidate) = &token.kind else {
            continue;
        };
        if candidate != name {
            continue;
        }
        let Some(colon_index) = previous_token_index(tokens, index) else {
            continue;
        };
        if tokens[colon_index].kind != TokenKind::Colon {
            continue;
        }
        let Some(property_index) = previous_token_index(tokens, colon_index) else {
            continue;
        };
        if tokens[property_index].kind == property {
            spans.insert((token.span.start, token.span.end));
        }
    }
    spans
}

fn sql_table_reference_spans(query: &str, table_name: &str) -> Vec<(usize, usize)> {
    let bytes = query.as_bytes();
    let mut index = 0;
    let mut expects_table = false;
    let mut spans = Vec::new();
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if bytes[index] == b'-' && bytes.get(index + 1) == Some(&b'-') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                index += 1;
            }
            index = (index + 2).min(bytes.len());
            continue;
        }
        if bytes[index] == b'\'' || bytes[index] == b'"' {
            let quote = bytes[index];
            index += 1;
            while index < bytes.len() {
                if bytes[index] == quote {
                    if bytes.get(index + 1) == Some(&quote) {
                        index += 2;
                    } else {
                        index += 1;
                        break;
                    }
                } else {
                    index += 1;
                }
            }
            continue;
        }
        if is_sql_identifier_start(bytes[index]) {
            let start = index;
            index += 1;
            while index < bytes.len() && is_sql_identifier_continue(bytes[index]) {
                index += 1;
            }
            let word = &query[start..index];
            if expects_table && word.eq_ignore_ascii_case(table_name) {
                spans.push((start, index));
                expects_table = false;
            } else {
                expects_table = matches!(
                    word.to_ascii_uppercase().as_str(),
                    "FROM" | "JOIN" | "INTO" | "UPDATE"
                );
            }
            continue;
        }
        if !matches!(bytes[index], b'.' | b'(' | b')') {
            expects_table = false;
        }
        index += 1;
    }
    spans
}

fn is_sql_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_sql_identifier_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn any_declaration_name_span(tokens: &[Token], keyword: TokenKind, name: &str) -> Option<Span> {
    for (index, token) in tokens.iter().enumerate() {
        if token.kind != keyword {
            continue;
        }
        if let Some(Token {
            kind: TokenKind::Ident(candidate),
            span,
        }) = tokens.get(index + 1)
        {
            if candidate == name {
                return Some(*span);
            }
        }
    }
    None
}

fn type_reference_spans(tokens: &[Token], name: &str) -> HashSet<(usize, usize)> {
    let record_literal_ranges = record_literal_ranges(tokens);
    let mut spans = HashSet::new();
    let mut in_type = false;
    let mut generic_depth = 0_usize;

    for (index, token) in tokens.iter().enumerate() {
        if matches!(token.kind, TokenKind::Eof) {
            break;
        }
        if in_type {
            match &token.kind {
                TokenKind::Ident(candidate) if candidate == name => {
                    spans.insert((token.span.start, token.span.end));
                }
                TokenKind::Less => generic_depth += 1,
                TokenKind::Greater if generic_depth > 0 => generic_depth -= 1,
                TokenKind::Greater | TokenKind::Comma if generic_depth == 0 => in_type = false,
                TokenKind::Newline | TokenKind::RParen | TokenKind::RBrace | TokenKind::Equal => {
                    in_type = false
                }
                TokenKind::Colon if generic_depth == 0 => in_type = false,
                _ if is_type_terminator(&token.kind) => in_type = false,
                _ => {}
            }
            if in_type {
                continue;
            }
        }

        let previous = previous_token(tokens, index);
        let starts_after_colon =
            matches!(previous.map(|token| &token.kind), Some(TokenKind::Colon))
                && !record_literal_ranges
                    .iter()
                    .any(|(start, end)| *start <= index && index <= *end);
        let starts_after_arrow =
            matches!(previous.map(|token| &token.kind), Some(TokenKind::Arrow));
        let starts_after_output =
            matches!(previous.map(|token| &token.kind), Some(TokenKind::Output));
        let starts_after_type_alias = matches!(
            (
                previous_token(tokens, index),
                previous_token_before(tokens, index)
            ),
            (
                Some(Token {
                    kind: TokenKind::Ident(_),
                    ..
                }),
                Some(Token {
                    kind: TokenKind::Type,
                    ..
                })
            )
        ) && token.kind == TokenKind::Equal;
        let starts_after_generic_header =
            matches!(token.kind, TokenKind::Less) && is_generic_type_header(previous);

        if starts_after_colon
            || starts_after_arrow
            || starts_after_output
            || starts_after_type_alias
        {
            in_type = true;
            if let TokenKind::Ident(candidate) = &token.kind {
                if candidate == name {
                    spans.insert((token.span.start, token.span.end));
                }
            }
            continue;
        } else if starts_after_generic_header {
            in_type = true;
            generic_depth = 1;
            continue;
        }
    }
    spans
}

fn is_type_terminator(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Required
            | TokenKind::Primary
            | TokenKind::Auto
            | TokenKind::Unique
            | TokenKind::Default
            | TokenKind::LBrace
            | TokenKind::LParen
            | TokenKind::RBracket
    )
}

fn is_generic_type_header(token: Option<&Token>) -> bool {
    match token.map(|token| &token.kind) {
        Some(TokenKind::Sql) => true,
        Some(TokenKind::Ident(name)) => matches!(
            name.as_str(),
            "Option" | "Result" | "HttpResult" | "json_decode" | "http_json" | "http_result"
        ),
        _ => false,
    }
}

fn record_literal_spans(tokens: &[Token], name: &str) -> HashSet<(usize, usize)> {
    let mut spans = HashSet::new();
    for index in 0..tokens.len() {
        let Some(Token {
            kind: TokenKind::Ident(candidate),
            span,
        }) = tokens.get(index)
        else {
            continue;
        };
        if candidate != name
            || !matches!(
                tokens.get(index + 1).map(|token| &token.kind),
                Some(TokenKind::LBrace)
            )
        {
            continue;
        }
        if matches!(
            previous_token(tokens, index).map(|token| &token.kind),
            Some(
                TokenKind::Struct
                    | TokenKind::Table
                    | TokenKind::Form
                    | TokenKind::Crud
                    | TokenKind::View
                    | TokenKind::Component
                    | TokenKind::TableView
            )
        ) {
            continue;
        }
        if matches!(
            (
                tokens.get(index + 2).map(|token| &token.kind),
                tokens.get(index + 3).map(|token| &token.kind)
            ),
            (Some(TokenKind::Ident(_)), Some(TokenKind::Colon))
        ) {
            spans.insert((span.start, span.end));
        }
    }
    spans
}

fn record_literal_ranges(tokens: &[Token]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    for index in 0..tokens.len() {
        let is_literal = matches!(
            (
                tokens.get(index).map(|token| &token.kind),
                tokens.get(index + 1).map(|token| &token.kind),
                tokens.get(index + 2).map(|token| &token.kind),
                tokens.get(index + 3).map(|token| &token.kind)
            ),
            (
                Some(TokenKind::Ident(_)),
                Some(TokenKind::LBrace),
                Some(TokenKind::Ident(_)),
                Some(TokenKind::Colon)
            )
        ) && !matches!(
            previous_token(tokens, index).map(|token| &token.kind),
            Some(
                TokenKind::Struct
                    | TokenKind::Table
                    | TokenKind::Form
                    | TokenKind::Crud
                    | TokenKind::View
                    | TokenKind::Component
                    | TokenKind::TableView
            )
        );
        if !is_literal {
            continue;
        }
        let mut depth = 0_usize;
        for (end, token) in tokens.iter().enumerate().skip(index + 1) {
            match token.kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        ranges.push((index + 1, end));
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    ranges
}

fn previous_token(tokens: &[Token], index: usize) -> Option<&Token> {
    previous_token_index(tokens, index).and_then(|index| tokens.get(index))
}

fn previous_token_before(tokens: &[Token], index: usize) -> Option<&Token> {
    let previous_index = previous_token_index(tokens, index)?;
    previous_token(tokens, previous_index)
}

fn previous_token_index(tokens: &[Token], index: usize) -> Option<usize> {
    tokens[..index]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, token)| !matches!(token.kind, TokenKind::Newline))
        .map(|(index, _)| index)
}

fn declaration_name_span(
    tokens: &[Token],
    owner: Span,
    keyword: TokenKind,
    name: &str,
) -> Option<Span> {
    let mut keyword_found = false;
    for token in tokens
        .iter()
        .filter(|token| token.span.start >= owner.start && token.span.end <= owner.end)
    {
        if !keyword_found {
            if token.kind == keyword {
                keyword_found = true;
            }
            continue;
        }
        if let TokenKind::Ident(candidate) = &token.kind {
            return (candidate == name).then_some(token.span);
        }
    }
    None
}

fn collect_function_block(
    block: &zelyra_ast::Block,
    name: &str,
    spans: &mut HashSet<(usize, usize)>,
) {
    for statement in &block.statements {
        match statement {
            zelyra_ast::Stmt::Let { value, .. } | zelyra_ast::Stmt::BindOrAssign { value, .. } => {
                collect_function_expression(value, name, spans);
            }
            zelyra_ast::Stmt::Expr(expression) => {
                collect_function_expression(expression, name, spans);
            }
            zelyra_ast::Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    collect_function_expression(value, name, spans);
                }
            }
            zelyra_ast::Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                collect_function_expression(condition, name, spans);
                collect_function_block(then_block, name, spans);
                if let Some(else_block) = else_block {
                    collect_function_block(else_block, name, spans);
                }
            }
            zelyra_ast::Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                collect_function_expression(condition, name, spans);
                for invariant in invariants {
                    collect_function_expression(invariant, name, spans);
                }
                collect_function_block(body, name, spans);
            }
            zelyra_ast::Stmt::For { iterable, body, .. } => {
                collect_function_expression(iterable, name, spans);
                collect_function_block(body, name, spans);
            }
            zelyra_ast::Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    collect_function_expression(invariant, name, spans);
                }
                collect_function_block(body, name, spans);
            }
            zelyra_ast::Stmt::Match { value, arms, .. } => {
                collect_function_expression(value, name, spans);
                for arm in arms {
                    collect_function_block(&arm.body, name, spans);
                }
            }
            zelyra_ast::Stmt::Transaction { body, .. }
            | zelyra_ast::Stmt::Parallel { body, .. } => {
                collect_function_block(body, name, spans);
            }
            zelyra_ast::Stmt::Break { .. } | zelyra_ast::Stmt::Continue { .. } => {}
        }
    }
}

fn collect_function_expression(
    expression: &zelyra_ast::Expr,
    name: &str,
    spans: &mut HashSet<(usize, usize)>,
) {
    match &expression.kind {
        zelyra_ast::ExprKind::Array(values) => {
            for value in values {
                collect_function_expression(value, name, spans);
            }
        }
        zelyra_ast::ExprKind::Record { fields, .. } => {
            for (_, value) in fields {
                collect_function_expression(value, name, spans);
            }
        }
        zelyra_ast::ExprKind::Index { target, index } => {
            collect_function_expression(target, name, spans);
            collect_function_expression(index, name, spans);
        }
        zelyra_ast::ExprKind::Field { target, .. } => {
            collect_function_expression(target, name, spans);
        }
        zelyra_ast::ExprKind::Call {
            name: called, args, ..
        } => {
            if called == name {
                spans.insert((expression.span.start, expression.span.start + name.len()));
            }
            for argument in args {
                collect_function_expression(argument, name, spans);
            }
        }
        zelyra_ast::ExprKind::Unary { expr, .. } | zelyra_ast::ExprKind::Await(expr) => {
            collect_function_expression(expr, name, spans);
        }
        zelyra_ast::ExprKind::Binary { left, right, .. } => {
            collect_function_expression(left, name, spans);
            collect_function_expression(right, name, spans);
        }
        zelyra_ast::ExprKind::Int(_)
        | zelyra_ast::ExprKind::UInt(_)
        | zelyra_ast::ExprKind::Float(_)
        | zelyra_ast::ExprKind::Bool(_)
        | zelyra_ast::ExprKind::String(_)
        | zelyra_ast::ExprKind::Char(_)
        | zelyra_ast::ExprKind::Variable(_)
        | zelyra_ast::ExprKind::Sql { .. } => {}
    }
}

fn declares_symbol(program: &Program, symbol: &str, name: &str) -> bool {
    match symbol {
        "function" => program.functions.iter().any(|item| item.name == name),
        "type" => program.types.iter().any(|item| item.name == name),
        "record" => program.records.iter().any(|item| item.name == name),
        "table" => program.tables.iter().any(|item| item.name == name),
        "tableview" => program.tableviews.iter().any(|item| item.name == name),
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
    use std::{fs, path::PathBuf};
    use zelyra_lexer::lex;
    use zelyra_parser::parse;

    #[test]
    fn previews_an_atomic_symbol_rename() {
        let source = "fn greet() { greet() }\n";
        let tokens = lex(source).expect("source should lex");
        let program = parse(&tokens).expect("source should parse");
        let request = json!({
            "schema_version": "1",
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

    #[test]
    fn function_rename_does_not_change_shadowing_locals() {
        let source = "fn greet() { greet = 1\n print(greet) }\nfn main() { greet() }\n";
        let tokens = lex(source).expect("source should lex");
        let program = parse(&tokens).expect("source should parse");
        let request = json!({
            "schema_version": "1",
            "entry": "main.zyl",
            "operations": [{
                "kind": "rename",
                "symbol": "function",
                "from": "greet",
                "to": "welcome"
            }]
        });
        let preview = preview(&program, source, &tokens, &request).expect("edit should preview");
        assert_eq!(
            preview.source,
            "fn welcome() { greet = 1\n print(greet) }\nfn main() { welcome() }\n"
        );
        assert_eq!(preview.changed_tokens, 2);
    }

    #[test]
    fn type_and_record_renames_follow_type_references_and_literals() {
        let source = "type CustomerId = Id\nstruct Customer { id: CustomerId }\nfn load(id: CustomerId) -> CustomerId { return id }\nfn make() -> Customer { return Customer { id: 1 } }\nfn main() { load(1) }\n";
        let tokens = lex(source).expect("source should lex");
        let program = parse(&tokens).expect("source should parse");
        let request = json!({
            "schema_version": "1",
            "entry": "main.zyl",
            "operations": [
                {"kind": "rename", "symbol": "type", "from": "CustomerId", "to": "ClientId"},
                {"kind": "rename", "symbol": "record", "from": "Customer", "to": "Client"}
            ]
        });
        let preview = preview(&program, source, &tokens, &request).expect("edit should preview");
        assert!(preview.source.contains("type ClientId = Id"));
        assert!(
            preview.source.contains("struct Client { id: ClientId }"),
            "{}",
            preview.source
        );
        assert!(preview.source.contains("fn load(id: ClientId) -> ClientId"));
        assert!(preview
            .source
            .contains("fn make() -> Client { return Client { id: 1 } }"));
        assert_eq!(preview.changed_tokens, 7);
    }

    #[test]
    fn resource_renames_follow_structured_references() {
        let source = r#"
            table customers { id: Id }
            view Shell {
                html { <slot /> }
            }
            page "/customers" {
                view: Shell
                html { <h1>Customers</h1> }
            }
            form CustomerForm -> customers {
                fields { id }
            }
            crud Customer -> customers
            fn load() {
                sql { SELECT id FROM customers WHERE note = 'customers' }
            }
            fn main() {}
        "#;
        let tokens = lex(source).expect("source should lex");
        let program = parse(&tokens).expect("source should parse");
        let request = json!({
            "schema_version": "1",
            "entry": "main.zyl",
            "operations": [
                {"kind": "rename", "symbol": "table", "from": "customers", "to": "clients"},
                {"kind": "rename", "symbol": "view", "from": "Shell", "to": "AppShell"},
                {"kind": "rename", "symbol": "form", "from": "CustomerForm", "to": "CustomerEditor"},
                {"kind": "rename", "symbol": "crud", "from": "Customer", "to": "CustomerAdmin"}
            ]
        });
        let preview = preview(&program, source, &tokens, &request).expect("edit should preview");
        assert!(preview.source.contains("table clients"));
        assert!(preview.source.contains("view AppShell"));
        assert!(preview.source.contains("view: AppShell"));
        assert!(preview.source.contains("form CustomerEditor -> clients"));
        assert!(preview.source.contains("crud CustomerAdmin -> clients"));
        assert!(preview
            .source
            .contains("SELECT id FROM clients WHERE note = 'customers'"));
        assert_eq!(preview.changed_tokens, 8);
    }

    #[test]
    fn previews_type_record_and_tableview_renames() {
        let source = r#"
            type CustomerId = Id
            struct CustomerInput { id: CustomerId }
            table customers { id: Id }
            tableview Customers {
                source sql<CustomerInput[]> { SELECT id FROM customers }
                columns { id }
            }
        "#;
        let tokens = lex(source).expect("source should lex");
        let program = parse(&tokens).expect("source should parse");
        let request = json!({
            "schema_version": "1",
            "entry": "main.zyl",
            "operations": [
                {"kind": "rename", "symbol": "type", "from": "CustomerId", "to": "ClientId"},
                {"kind": "rename", "symbol": "record", "from": "CustomerInput", "to": "ClientInput"},
                {"kind": "rename", "symbol": "tableview", "from": "Customers", "to": "Clients"}
            ]
        });
        let preview = preview(&program, source, &tokens, &request).expect("edit should preview");
        assert!(preview.source.contains("type ClientId = Id"));
        assert!(preview.source.contains("struct ClientInput"));
        assert!(preview.source.contains("tableview Clients"));
    }

    #[test]
    fn component_renames_follow_html_opening_and_closing_tags() {
        let source = r#"
            component Panel {
                html { <section><Badge /></section> }
            }
            component Badge {
                html { <strong>Ready</strong> }
            }
            view Shell {
                html { <Panel><Badge></Badge></Panel> }
            }
        "#;
        let tokens = lex(source).expect("source should lex");
        let program = parse(&tokens).expect("source should parse");
        let request = json!({
            "schema_version": "1",
            "entry": "main.zyl",
            "operations": [{
                "kind": "rename",
                "symbol": "component",
                "from": "Badge",
                "to": "StatusBadge"
            }]
        });
        let preview = preview(&program, source, &tokens, &request).expect("edit should preview");
        assert!(preview.source.contains("component StatusBadge"));
        assert!(preview.source.contains("<StatusBadge />"));
        assert!(preview.source.contains("<StatusBadge></StatusBadge>"));
        assert_eq!(preview.changed_tokens, 4);
    }

    #[test]
    fn applies_a_preview_atomically() {
        let directory =
            std::env::temp_dir().join(format!("zelyra-edit-atomic-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("temporary directory should exist");
        let path: PathBuf = directory.join("main.zyl");
        fs::write(&path, "fn greet() {}\n").expect("source should be written");
        apply_atomically(path.to_str().unwrap(), "fn welcome() {}\n").expect("edit should apply");
        assert_eq!(fs::read_to_string(path).unwrap(), "fn welcome() {}\n");
    }
}
