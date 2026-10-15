//! Resolved function and record occurrences from the compiler's linked AST.
//! The linker qualifies imported names, so identical spellings in different
//! modules have different identities. Lexing finds exact source token spans.

use super::*;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use zelyra_ast::{Block, Expr, ExprKind, Pattern, PatternKind, Span, Stmt, Type};
use zelyra_lexer::{Token, TokenKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Occurrence {
    pub key: String,
    pub path: String,
    pub start: usize,
    pub end: usize,
    pub declaration: bool,
}

#[derive(Clone, Debug)]
pub struct Symbol {
    pub key: String,
    pub name: String,
    pub path: String,
    pub detail: String,
    pub declaration: Occurrence,
}

#[derive(Default)]
pub struct SemanticIndex {
    pub symbols: Vec<Symbol>,
    pub occurrences: Vec<Occurrence>,
}

#[derive(Clone, Debug)]
pub struct SourceEdit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

#[derive(Clone, Debug)]
pub struct SourceChange {
    pub path: String,
    pub before: String,
    pub after: String,
    pub edits: Vec<SourceEdit>,
}

/// Compiler-resolved symbol rename shared by Studio and LSP. Only indexed
/// source spans are edited; the complete candidate project is rechecked.
pub fn plan_symbol_rename(
    entry: &Path,
    analysis: &language_service::Analysis,
    index: &SemanticIndex,
    symbol: &Symbol,
    replacement: &str,
    base_overlays: &[language_service::SourceOverlay],
) -> Result<Vec<SourceChange>, String> {
    if !analysis.success {
        return Err("fix project diagnostics before renaming".into());
    }
    let tokens = lex(replacement).map_err(|_| "newName must be a Zelyra identifier")?;
    if !matches!((tokens.first(), tokens.get(1)),
        (Some(zelyra_lexer::Token { kind: TokenKind::Ident(name), .. }), Some(zelyra_lexer::Token { kind: TokenKind::Eof, .. })) if name == replacement)
    {
        return Err("newName must be a single non-keyword identifier".into());
    }
    if replacement == symbol.name
        || index
            .symbols
            .iter()
            .any(|item| item.path == symbol.path && item.name == replacement)
    {
        return Err("rename conflicts with an existing symbol in the same module".into());
    }
    let mut edits = BTreeMap::<String, Vec<SourceEdit>>::new();
    for occurrence in index.references(&symbol.key, true) {
        edits
            .entry(occurrence.path.clone())
            .or_default()
            .push(SourceEdit {
                start: occurrence.start,
                end: occurrence.end,
                replacement: replacement.into(),
            });
    }
    let mut changes = Vec::new();
    for (path, mut edits) in edits {
        let source = analysis
            .sources
            .iter()
            .find(|source| source.path == path)
            .ok_or("reference source is unavailable")?;
        // This conservative rule protects local shadowing until the semantic
        // index covers local bindings and all symbol kinds.
        if lex(&source.text)
            .map_err(|_| "source is not lexable")?
            .iter()
            .any(|token| matches!(&token.kind, TokenKind::Ident(name) if name == replacement))
        {
            return Err(
                "new name already occurs in a changed file; inspect shadowing before renaming"
                    .into(),
            );
        }
        edits.sort_by_key(|edit| edit.start);
        if edits.windows(2).any(|pair| pair[0].end > pair[1].start) {
            return Err("rename occurrences overlap".into());
        }
        let mut after = source.text.clone();
        for edit in edits.iter().rev() {
            if !after.is_char_boundary(edit.start)
                || !after.is_char_boundary(edit.end)
                || after.get(edit.start..edit.end) != Some(symbol.name.as_str())
            {
                return Err("rename source span is stale".into());
            }
            after.replace_range(edit.start..edit.end, &edit.replacement);
        }
        changes.push(SourceChange {
            path,
            before: source.text.clone(),
            after,
            edits,
        });
    }
    let mut overlays = base_overlays.to_vec();
    for change in &changes {
        if let Some(overlay) = overlays.iter_mut().find(|item| item.path == change.path) {
            overlay.text = change.after.clone();
        } else {
            overlays.push(language_service::SourceOverlay {
                path: change.path.clone(),
                text: change.after.clone(),
                version: 0,
            });
        }
    }
    let checked = language_service::analyze(entry, &overlays);
    if !checked.success {
        return Err(format!(
            "renamed project does not compile: {}",
            checked
                .diagnostics
                .first()
                .and_then(|item| item.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("unknown compiler error")
        ));
    }
    Ok(changes)
}

impl SemanticIndex {
    pub fn from_analysis(analysis: &language_service::Analysis) -> Self {
        let Some(program) = analysis.program.as_ref() else {
            return Self::default();
        };
        let tokens = analysis
            .sources
            .iter()
            .enumerate()
            .filter_map(|(index, source)| {
                lex(&source.text).ok().map(|tokens| (index as u32, tokens))
            })
            .collect::<HashMap<_, _>>();
        let mut index = Self::default();
        let mut known_records = HashSet::new();
        for record in &program.records {
            let Some(source) = analysis.sources.get(record.span.source_id as usize) else {
                continue;
            };
            let Some(source_tokens) = tokens.get(&record.span.source_id) else {
                continue;
            };
            let name = record.name.rsplit("::").next().unwrap_or(&record.name);
            let Some(span) = record_declaration_name_span(source_tokens, record.span, name) else {
                continue;
            };
            let key = format!("record:{}", record.name);
            let declaration = Occurrence {
                key: key.clone(),
                path: source.path.clone(),
                start: span.start,
                end: span.end,
                declaration: true,
            };
            let fields = record
                .fields
                .iter()
                .map(|field| format!("{}: {}", field.name, field.ty))
                .collect::<Vec<_>>()
                .join(", ");
            index.occurrences.push(declaration.clone());
            index.symbols.push(Symbol {
                key: key.clone(),
                name: name.into(),
                path: source.path.clone(),
                detail: format!(
                    "{} {name} {{ {fields} }}",
                    if record.is_class { "class" } else { "struct" }
                ),
                declaration,
            });
            known_records.insert(key);
        }
        for function in &program.functions {
            let Some(source) = analysis.sources.get(function.span.source_id as usize) else {
                continue;
            };
            let Some(source_tokens) = tokens.get(&function.span.source_id) else {
                continue;
            };
            let name = function.name.rsplit("::").next().unwrap_or(&function.name);
            let Some(span) = declaration_name_span(source_tokens, function.span, name) else {
                continue;
            };
            let declaration = Occurrence {
                key: function.name.clone(),
                path: source.path.clone(),
                start: span.start,
                end: span.end,
                declaration: true,
            };
            let parameters = function
                .params
                .iter()
                .map(|parameter| format!("{}: {}", parameter.name, parameter.ty))
                .collect::<Vec<_>>()
                .join(", ");
            let returns = function
                .return_type
                .as_ref()
                .map_or_else(|| "Unit".into(), ToString::to_string);
            let mut detail = format!("fn {name}({parameters}) -> {returns}");
            if !function.capabilities.is_empty() {
                detail.push_str(&format!(" uses {{{}}}", function.capabilities.join(", ")));
            }
            index.occurrences.push(declaration.clone());
            index.symbols.push(Symbol {
                key: function.name.clone(),
                name: name.into(),
                path: source.path.clone(),
                detail,
                declaration,
            });
        }
        let known = index
            .symbols
            .iter()
            .map(|symbol| symbol.key.clone())
            .collect::<HashSet<_>>();
        for type_definition in &program.types {
            if let Some(span) = declaration_type_value_span(
                type_definition.span,
                tokens
                    .get(&type_definition.span.source_id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
            ) {
                push_type_references(
                    &type_definition.target,
                    span,
                    &known,
                    &analysis.sources,
                    &tokens,
                    &mut index.occurrences,
                );
            }
        }
        for table in &program.tables {
            for column in &table.columns {
                push_type_references(
                    &column.ty,
                    column.span,
                    &known,
                    &analysis.sources,
                    &tokens,
                    &mut index.occurrences,
                );
            }
        }
        for form in &program.forms {
            for field in form
                .fields
                .iter()
                .chain(form.actions.iter().flat_map(|action| &action.fields))
            {
                if let Some(ty) = &field.ty {
                    push_type_references(
                        ty,
                        field.span,
                        &known,
                        &analysis.sources,
                        &tokens,
                        &mut index.occurrences,
                    );
                }
            }
        }
        for api in &program.apis {
            for field in &api.input {
                push_type_references(
                    &field.ty,
                    field.span,
                    &known,
                    &analysis.sources,
                    &tokens,
                    &mut index.occurrences,
                );
            }
        }
        for page in &program.pages {
            for input in &page.inputs {
                push_type_references(
                    &input.ty,
                    input.span,
                    &known,
                    &analysis.sources,
                    &tokens,
                    &mut index.occurrences,
                );
            }
            for data in &page.data {
                push_type_references(
                    &data.result_type,
                    data.span,
                    &known,
                    &analysis.sources,
                    &tokens,
                    &mut index.occurrences,
                );
            }
        }
        for tableview in &program.tableviews {
            push_type_references(
                &tableview.result_type,
                tableview.span,
                &known,
                &analysis.sources,
                &tokens,
                &mut index.occurrences,
            );
        }
        for component in &program.components {
            for prop in &component.props {
                push_type_references(
                    &prop.ty,
                    prop.span,
                    &known,
                    &analysis.sources,
                    &tokens,
                    &mut index.occurrences,
                );
            }
        }
        for function in &program.functions {
            for parameter in &function.params {
                if let Some(span) = parameter_type_span(
                    parameter.span,
                    function.body.span.start,
                    tokens
                        .get(&function.span.source_id)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                ) {
                    push_type_references(
                        &parameter.ty,
                        span,
                        &known,
                        &analysis.sources,
                        &tokens,
                        &mut index.occurrences,
                    );
                }
            }
            if let Some(return_type) = &function.return_type {
                if let Some(span) = function_return_type_span(
                    function.span,
                    function.body.span.start,
                    tokens
                        .get(&function.span.source_id)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                ) {
                    push_type_references(
                        return_type,
                        span,
                        &known,
                        &analysis.sources,
                        &tokens,
                        &mut index.occurrences,
                    );
                }
            }
            for contract in function.requires.iter().chain(&function.ensures) {
                visit_expression(
                    contract,
                    &known,
                    &analysis.sources,
                    &tokens,
                    &mut index.occurrences,
                );
            }
            visit_block(
                &function.body,
                &known,
                &analysis.sources,
                &tokens,
                &mut index.occurrences,
            );
        }
        for record in &program.records {
            for field in &record.fields {
                push_type_references(
                    &field.ty,
                    field.span,
                    &known,
                    &analysis.sources,
                    &tokens,
                    &mut index.occurrences,
                );
            }
        }
        index.occurrences.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then(left.start.cmp(&right.start))
                .then(left.end.cmp(&right.end))
        });
        index.occurrences.dedup();
        index.symbols.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then(left.declaration.start.cmp(&right.declaration.start))
        });
        index
    }

    pub fn at(&self, path: &str, offset: usize) -> Option<&Occurrence> {
        self.occurrences
            .iter()
            .find(|item| item.path == path && item.start <= offset && offset < item.end)
    }

    pub fn symbol(&self, key: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|symbol| symbol.key == key)
    }

    pub fn references(&self, key: &str, include_declaration: bool) -> Vec<&Occurrence> {
        self.occurrences
            .iter()
            .filter(|item| item.key == key && (include_declaration || !item.declaration))
            .collect()
    }

    /// Detect possible recursion from resolved function references. Function
    /// values and contract calls also count as possible edges, so this is a
    /// risk view, not a proof of the maximum runtime stack depth.
    pub fn recursion_risks(&self, analysis: &language_service::Analysis) -> (Vec<Value>, bool) {
        let Some(program) = analysis.program.as_ref() else {
            return (Vec::new(), false);
        };
        const MAX_FUNCTIONS: usize = 500;
        if program.functions.len() > MAX_FUNCTIONS {
            return (Vec::new(), true);
        }
        let names = self
            .symbols
            .iter()
            .map(|symbol| symbol.key.as_str())
            .collect::<Vec<_>>();
        let by_name = names
            .iter()
            .enumerate()
            .map(|(index, name)| (*name, index))
            .collect::<HashMap<_, _>>();
        let mut edges = vec![Vec::new(); names.len()];
        for function in &program.functions {
            let Some(&owner) = by_name.get(function.name.as_str()) else {
                continue;
            };
            let Some(source) = analysis.sources.get(function.span.source_id as usize) else {
                continue;
            };
            for reference in self.occurrences.iter().filter(|item| {
                !item.declaration
                    && item.path == source.path
                    && function.span.start <= item.start
                    && item.end <= function.span.end
            }) {
                if let Some(&target) = by_name.get(reference.key.as_str()) {
                    edges[owner].push(target);
                }
            }
            edges[owner].sort_unstable();
            edges[owner].dedup();
        }
        let mut reverse = vec![Vec::new(); names.len()];
        for (owner, targets) in edges.iter().enumerate() {
            for &target in targets {
                reverse[target].push(owner);
            }
        }
        let mut visited = vec![false; names.len()];
        let mut order = Vec::with_capacity(names.len());
        for start in 0..names.len() {
            if visited[start] {
                continue;
            }
            visited[start] = true;
            let mut stack = vec![(start, 0)];
            while let Some((node, next)) = stack.last_mut() {
                if *next < edges[*node].len() {
                    let target = edges[*node][*next];
                    *next += 1;
                    if !visited[target] {
                        visited[target] = true;
                        stack.push((target, 0));
                    }
                } else {
                    order.push(*node);
                    stack.pop();
                }
            }
        }
        let mut grouped = vec![false; names.len()];
        let mut risks = Vec::new();
        for &index in order.iter().rev() {
            if grouped[index] {
                continue;
            }
            grouped[index] = true;
            let mut pending = vec![index];
            let mut members = Vec::new();
            while let Some(member) = pending.pop() {
                members.push(member);
                for &caller in &reverse[member] {
                    if !grouped[caller] {
                        grouped[caller] = true;
                        pending.push(caller);
                    }
                }
            }
            members.sort_unstable();
            if members.len() == 1 && !edges[index].contains(&index) {
                continue;
            }
            let functions = members
                .iter()
                .map(|&member| {
                    let symbol = &self.symbols[member];
                    let line = analysis
                        .sources
                        .iter()
                        .find(|source| source.path == symbol.path)
                        .map(|source| {
                            source.text[..symbol.declaration.start.min(source.text.len())]
                                .bytes()
                                .filter(|byte| *byte == b'\n')
                                .count()
                                + 1
                        })
                        .unwrap_or(1);
                    json!({"name":symbol.name,"path":symbol.path,"line":line})
                })
                .collect::<Vec<_>>();
            risks.push(json!({"kind":if members.len()>1 {"mutual"} else {"direct"},
                "functions":functions}));
        }
        (risks, false)
    }
}

fn declaration_name_span(tokens: &[Token], declaration: Span, name: &str) -> Option<Span> {
    let mut after_fn = false;
    for token in tokens
        .iter()
        .filter(|token| token.span.start >= declaration.start && token.span.end <= declaration.end)
    {
        match &token.kind {
            TokenKind::Fn => after_fn = true,
            TokenKind::Ident(found) if after_fn && found == name => return Some(token.span),
            _ => {}
        }
    }
    None
}

fn record_declaration_name_span(tokens: &[Token], declaration: Span, name: &str) -> Option<Span> {
    let mut tokens = tokens
        .iter()
        .filter(|token| token.span.start >= declaration.start && token.span.end <= declaration.end);
    tokens.next()?;
    tokens.find_map(|token| match &token.kind {
        TokenKind::Ident(found) if found == name => Some(token.span),
        _ => None,
    })
}

fn record_reference_span(tokens: &[Token], expression: Span, name: &str) -> Option<Span> {
    tokens
        .iter()
        .filter(|token| token.span.start >= expression.start && token.span.end <= expression.end)
        .find_map(|token| match &token.kind {
            TokenKind::Ident(found) if found == name => Some(token.span),
            _ => None,
        })
}

fn parameter_type_span(parameter: Span, body_start: usize, tokens: &[Token]) -> Option<Span> {
    let colon = tokens.iter().position(|token| {
        token.span.start >= parameter.end
            && token.span.start < body_start
            && token.kind == TokenKind::Colon
    })?;
    let mut type_tokens = Vec::new();
    let mut angle_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    for token in tokens.iter().skip(colon + 1) {
        if token.span.start >= body_start {
            break;
        }
        match &token.kind {
            TokenKind::Less => angle_depth += 1,
            TokenKind::Greater => angle_depth = angle_depth.saturating_sub(1),
            TokenKind::LParen => paren_depth += 1,
            TokenKind::RParen if paren_depth == 0 && angle_depth == 0 && bracket_depth == 0 => {
                break
            }
            TokenKind::RParen => paren_depth = paren_depth.saturating_sub(1),
            TokenKind::LBracket => bracket_depth += 1,
            TokenKind::RBracket => bracket_depth = bracket_depth.saturating_sub(1),
            TokenKind::Comma if angle_depth == 0 && paren_depth == 0 && bracket_depth == 0 => break,
            _ => {}
        }
        type_tokens.push(token);
    }
    let first = type_tokens.first()?.span;
    let last = type_tokens.last()?.span;
    Some(Span {
        start: first.start,
        end: last.end,
        line: first.line,
        column: first.column,
        source_id: parameter.source_id,
    })
}

fn function_return_type_span(function: Span, body_start: usize, tokens: &[Token]) -> Option<Span> {
    let arrow = tokens.iter().position(|token| {
        token.span.start >= function.start
            && token.span.start < body_start
            && token.kind == TokenKind::Arrow
    })?;
    let first = tokens.get(arrow + 1)?.span;
    let mut last = None;
    for token in tokens.iter().skip(arrow + 1) {
        if token.span.end > body_start {
            break;
        }
        last = Some(token.span);
    }
    let last = last?;
    Some(Span {
        start: first.start,
        end: last.end,
        line: first.line,
        column: first.column,
        source_id: function.source_id,
    })
}

fn declaration_type_value_span(declaration: Span, tokens: &[Token]) -> Option<Span> {
    let equal = tokens
        .iter()
        .position(|token| token.span.start >= declaration.end && token.kind == TokenKind::Equal)?;
    let mut type_tokens = Vec::new();
    for token in tokens.iter().skip(equal + 1) {
        if matches!(token.kind, TokenKind::Newline | TokenKind::Eof) {
            break;
        }
        type_tokens.push(token);
    }
    let first = type_tokens.first()?.span;
    let last = type_tokens.last()?.span;
    Some(Span {
        start: first.start,
        end: last.end,
        line: first.line,
        column: first.column,
        source_id: declaration.source_id,
    })
}

fn push_record_reference(
    type_name: &str,
    span: Span,
    known: &HashSet<String>,
    sources: &[project::ProjectSource],
    tokens: &HashMap<u32, Vec<Token>>,
    output: &mut Vec<Occurrence>,
) {
    let key = format!("record:{type_name}");
    if !known.contains(&key) {
        return;
    }
    let Some(source) = sources.get(span.source_id as usize) else {
        return;
    };
    let Some(source_tokens) = tokens.get(&span.source_id) else {
        return;
    };
    let name = type_name.rsplit("::").next().unwrap_or(type_name);
    let Some(name_span) = record_reference_span(source_tokens, span, name) else {
        return;
    };
    output.push(Occurrence {
        key,
        path: source.path.clone(),
        start: name_span.start,
        end: name_span.end,
        declaration: false,
    });
}

fn push_type_references(
    ty: &Type,
    span: Span,
    known: &HashSet<String>,
    sources: &[project::ProjectSource],
    tokens: &HashMap<u32, Vec<Token>>,
    output: &mut Vec<Occurrence>,
) {
    match ty {
        Type::Named(type_name) => {
            let key = format!("record:{type_name}");
            if !known.contains(&key) {
                return;
            }
            let Some(source) = sources.get(span.source_id as usize) else {
                return;
            };
            let Some(source_tokens) = tokens.get(&span.source_id) else {
                return;
            };
            let plain = type_name.rsplit("::").next().unwrap_or(type_name);
            let name_span = source_tokens
                .iter()
                .filter(|token| token.span.start >= span.start && token.span.end <= span.end)
                .filter_map(|token| match &token.kind {
                    TokenKind::Ident(found) if found == plain => Some(token.span),
                    _ => None,
                })
                .next_back();
            if let Some(name_span) = name_span {
                output.push(Occurrence {
                    key,
                    path: source.path.clone(),
                    start: name_span.start,
                    end: name_span.end,
                    declaration: false,
                });
            }
        }
        Type::Option(inner) | Type::Array(inner) | Type::Set(inner) | Type::HttpResult(inner) => {
            push_type_references(inner, span, known, sources, tokens, output)
        }
        Type::Result(ok, error) | Type::Map(ok, error) => {
            push_type_references(ok, span, known, sources, tokens, output);
            push_type_references(error, span, known, sources, tokens, output);
        }
        Type::Tuple(items) => {
            for item in items {
                push_type_references(item, span, known, sources, tokens, output);
            }
        }
        Type::Function {
            params,
            return_type,
            ..
        } => {
            for parameter in params {
                push_type_references(parameter, span, known, sources, tokens, output);
            }
            push_type_references(return_type, span, known, sources, tokens, output);
        }
        Type::Applied { name, args } => {
            push_type_references(
                &Type::Named(name.clone()),
                span,
                known,
                sources,
                tokens,
                output,
            );
            for argument in args {
                push_type_references(argument, span, known, sources, tokens, output);
            }
        }
        Type::Int
        | Type::UInt
        | Type::Float
        | Type::Decimal
        | Type::Bool
        | Type::String
        | Type::Char
        | Type::Bytes
        | Type::Timestamp
        | Type::Date
        | Type::Time
        | Type::Duration
        | Type::Unit
        | Type::Unknown => {}
    }
}

fn reference_name_span(tokens: &[Token], expression: Span, name: &str, call: bool) -> Option<Span> {
    let mut last_ident = None;
    for token in tokens
        .iter()
        .filter(|token| token.span.start >= expression.start && token.span.end <= expression.end)
    {
        if call && matches!(token.kind, TokenKind::LParen) {
            break;
        }
        if let TokenKind::Ident(found) = &token.kind {
            if found == name {
                last_ident = Some(token.span);
            }
        }
    }
    last_ident
}

fn push_reference(
    name: &str,
    span: Span,
    call: bool,
    known: &HashSet<String>,
    sources: &[project::ProjectSource],
    tokens: &HashMap<u32, Vec<Token>>,
    output: &mut Vec<Occurrence>,
) {
    if !known.contains(name) {
        return;
    }
    let Some(source) = sources.get(span.source_id as usize) else {
        return;
    };
    let Some(source_tokens) = tokens.get(&span.source_id) else {
        return;
    };
    let plain = name.rsplit("::").next().unwrap_or(name);
    let Some(name_span) = reference_name_span(source_tokens, span, plain, call) else {
        return;
    };
    output.push(Occurrence {
        key: name.into(),
        path: source.path.clone(),
        start: name_span.start,
        end: name_span.end,
        declaration: false,
    });
}

fn visit_block(
    block: &Block,
    known: &HashSet<String>,
    sources: &[project::ProjectSource],
    tokens: &HashMap<u32, Vec<Token>>,
    output: &mut Vec<Occurrence>,
) {
    for statement in &block.statements {
        match statement {
            Stmt::Let {
                ty, value, span, ..
            } => {
                if let Some(ty) = ty {
                    let annotation_span = Span {
                        end: value.span.start,
                        ..*span
                    };
                    push_type_references(ty, annotation_span, known, sources, tokens, output);
                }
                visit_expression(value, known, sources, tokens, output);
            }
            Stmt::BindOrAssign { value, .. }
            | Stmt::TupleBindOrAssign { value, .. }
            | Stmt::Expr(value) => {
                visit_expression(value, known, sources, tokens, output);
            }
            Stmt::Return {
                value: Some(value), ..
            } => visit_expression(value, known, sources, tokens, output),
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                visit_expression(condition, known, sources, tokens, output);
                visit_block(then_block, known, sources, tokens, output);
                if let Some(block) = else_block {
                    visit_block(block, known, sources, tokens, output);
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                visit_expression(condition, known, sources, tokens, output);
                for invariant in invariants {
                    visit_expression(invariant, known, sources, tokens, output);
                }
                visit_block(body, known, sources, tokens, output);
            }
            Stmt::For { iterable, body, .. } => {
                visit_expression(iterable, known, sources, tokens, output);
                visit_block(body, known, sources, tokens, output);
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    visit_expression(invariant, known, sources, tokens, output);
                }
                visit_block(body, known, sources, tokens, output);
            }
            Stmt::Match { value, arms, .. } => {
                visit_expression(value, known, sources, tokens, output);
                for arm in arms {
                    visit_record_pattern(&arm.pattern, known, sources, tokens, output);
                    if let Some(guard) = &arm.guard {
                        visit_expression(guard, known, sources, tokens, output);
                    }
                    visit_block(&arm.body, known, sources, tokens, output);
                }
            }
            Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
                visit_block(body, known, sources, tokens, output)
            }
            Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }
}

fn visit_expression(
    expression: &Expr,
    known: &HashSet<String>,
    sources: &[project::ProjectSource],
    tokens: &HashMap<u32, Vec<Token>>,
    output: &mut Vec<Occurrence>,
) {
    match &expression.kind {
        ExprKind::Call { name, args, .. } => {
            push_reference(name, expression.span, true, known, sources, tokens, output);
            for arg in args {
                visit_expression(arg, known, sources, tokens, output);
            }
        }
        ExprKind::FunctionValue(name) => {
            push_reference(name, expression.span, false, known, sources, tokens, output)
        }
        ExprKind::Array(items) => {
            for item in items {
                visit_expression(item, known, sources, tokens, output);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                visit_expression(key, known, sources, tokens, output);
                visit_expression(value, known, sources, tokens, output);
            }
        }
        ExprKind::Record {
            type_name, fields, ..
        } => {
            push_record_reference(type_name, expression.span, known, sources, tokens, output);
            for (_, value) in fields {
                visit_expression(value, known, sources, tokens, output);
            }
        }
        ExprKind::Lambda { body, .. }
        | ExprKind::Await(body)
        | ExprKind::Unary { expr: body, .. } => {
            visit_expression(body, known, sources, tokens, output);
        }
        ExprKind::Index { target, index } => {
            visit_expression(target, known, sources, tokens, output);
            visit_expression(index, known, sources, tokens, output);
        }
        ExprKind::Field { target, .. } => visit_expression(target, known, sources, tokens, output),
        ExprKind::Binary { left, right, .. } => {
            visit_expression(left, known, sources, tokens, output);
            visit_expression(right, known, sources, tokens, output);
        }
        ExprKind::Match { value, arms } => {
            visit_expression(value, known, sources, tokens, output);
            for arm in arms {
                visit_record_pattern(&arm.pattern, known, sources, tokens, output);
                if let Some(guard) = &arm.guard {
                    visit_expression(guard, known, sources, tokens, output);
                }
                visit_expression(&arm.value, known, sources, tokens, output);
            }
        }
        ExprKind::Int(_)
        | ExprKind::UInt(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::String(_)
        | ExprKind::Char(_)
        | ExprKind::Variable(_)
        | ExprKind::Sql { .. } => {}
    }
}

fn visit_record_pattern(
    pattern: &Pattern,
    known: &HashSet<String>,
    sources: &[project::ProjectSource],
    tokens: &HashMap<u32, Vec<Token>>,
    output: &mut Vec<Occurrence>,
) {
    match &pattern.kind {
        PatternKind::Record { type_name, fields } => {
            push_record_reference(type_name, pattern.span, known, sources, tokens, output);
            for (_, nested) in fields {
                visit_record_pattern(nested, known, sources, tokens, output);
            }
        }
        PatternKind::Constructor {
            inner: Some(inner), ..
        } => visit_record_pattern(inner, known, sources, tokens, output),
        PatternKind::EnumVariantPayload { payloads, .. } => {
            for nested in payloads {
                visit_record_pattern(nested, known, sources, tokens, output);
            }
        }
        PatternKind::Wildcard
        | PatternKind::Variable(_)
        | PatternKind::Constructor { inner: None, .. }
        | PatternKind::EnumVariant { .. }
        | PatternKind::Int(_)
        | PatternKind::Bool(_)
        | PatternKind::String(_)
        | PatternKind::Char(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_record_definition_and_literal_share_a_rename_identity() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-index-record-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("zelyra.toml"),
            "[project]\nname = \"record-rename\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        let source = "import \"src/models.zyl\" as model\nfn main() { customer = model::Customer { name: \"Ada\" } print(customer.name) }\n";
        fs::write(root.join("main.zyl"), source).unwrap();
        fs::write(
            root.join("src/models.zyl"),
            "pub struct Customer { name: String }\n",
        )
        .unwrap();
        let entry = root.join("main.zyl");
        let analysis = language_service::analyze(&entry, &[]);
        assert!(analysis.success, "{:?}", analysis.diagnostics);
        let index = SemanticIndex::from_analysis(&analysis);
        let literal_offset = source.find("Customer {").unwrap();
        let occurrence = index.at("main.zyl", literal_offset).unwrap();
        let symbol = index.symbol(&occurrence.key).unwrap();
        assert_eq!(symbol.name, "Customer");
        assert_eq!(symbol.path, "src/models.zyl");

        let changes = plan_symbol_rename(&entry, &analysis, &index, symbol, "Account", &[])
            .expect("the exact declaration and literal rename should compile");
        assert_eq!(changes.len(), 2);
        assert!(changes
            .iter()
            .any(|change| change.after.contains("Account {")));
        assert!(changes
            .iter()
            .any(|change| change.after.contains("model::Account {")));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn record_patterns_are_indexed_for_rename_and_navigation() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-index-record-pattern-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("zelyra.toml"),
            "[project]\nname = \"record-pattern\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        let source = "struct Customer { name: String }\nfn main() { customer = Customer { name: \"Ada\" } result_value = match customer { Customer { name } => name } print(result_value) }\n";
        fs::write(root.join("main.zyl"), source).unwrap();
        let entry = root.join("main.zyl");
        let analysis = language_service::analyze(&entry, &[]);
        assert!(analysis.success, "{:?}", analysis.diagnostics);
        let index = SemanticIndex::from_analysis(&analysis);
        let pattern_offset = source.find("Customer { name }").unwrap();
        let occurrence = index.at("main.zyl", pattern_offset).unwrap();
        let symbol = index.symbol(&occurrence.key).unwrap();
        let changes = plan_symbol_rename(&entry, &analysis, &index, symbol, "Account", &[])
            .expect("record declaration, literal, and pattern should rename together");
        assert!(changes[0].after.contains("Account { name }"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn record_rename_updates_type_annotations_and_constructors() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-index-record-type-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("zelyra.toml"),
            "[project]\nname = \"record-type-rename\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        let source = "import \"src/models.zyl\" as model\ntype CustomerAlias = model::Customer\nfn retain(customer: model::Customer) -> model::Customer { return customer }\nfn main() { customer: model::Customer = model::Customer { name: \"Ada\" } print(customer.name) }\n";
        fs::write(root.join("main.zyl"), source).unwrap();
        fs::write(
            root.join("src/models.zyl"),
            "pub struct Customer { name: String }\n",
        )
        .unwrap();
        let entry = root.join("main.zyl");
        let analysis = language_service::analyze(&entry, &[]);
        assert!(analysis.success, "{:?}", analysis.diagnostics);
        let index = SemanticIndex::from_analysis(&analysis);
        let occurrence = index
            .at("main.zyl", source.find("Customer {").unwrap())
            .unwrap();
        let symbol = index.symbol(&occurrence.key).unwrap();
        assert_eq!(
            index.references(&symbol.key, false).len(),
            5,
            "{:?}",
            index.references(&symbol.key, false)
        );
        let changes = plan_symbol_rename(&entry, &analysis, &index, symbol, "Account", &[])
            .expect("type annotation and record literal should be renamed together");
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().any(|change| change
            .after
            .contains("customer: model::Account = model::Account {")));
        assert!(changes.iter().any(|change| change
            .after
            .contains("fn retain(customer: model::Account) -> model::Account")));
        assert!(changes
            .iter()
            .any(|change| change.after.contains("type CustomerAlias = model::Account")));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recursion_graph_distinguishes_direct_and_mutual_cycles() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-recursion-index-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("main.zyl"), "fn direct() -> Int { return direct() }\nfn left() -> Int { return right() }\nfn right() -> Int { return left() }\nfn plain() -> Int { return 1 }\nfn main() { print(plain()) }\n").unwrap();
        let analysis = language_service::analyze(&root.join("main.zyl"), &[]);
        assert!(analysis.success, "{:?}", analysis.diagnostics);
        let (risks, incomplete) =
            SemanticIndex::from_analysis(&analysis).recursion_risks(&analysis);
        assert!(!incomplete);
        assert_eq!(risks.len(), 2, "{risks:?}");
        assert!(risks
            .iter()
            .any(|risk| risk["kind"] == "direct" && risk["functions"][0]["name"] == "direct"));
        assert!(risks.iter().any(
            |risk| risk["kind"] == "mutual" && risk["functions"].as_array().unwrap().len() == 2
        ));
        assert!(!serde_json::to_string(&risks).unwrap().contains("plain"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn qualified_calls_resolve_to_distinct_function_identities() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-index-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("main.zyl"), "import \"src/a.zyl\" as a\nimport \"src/b.zyl\" as b\nfn main() { print(a::answer()) print(b::answer()) }\n").unwrap();
        fs::write(
            root.join("src/a.zyl"),
            "pub fn answer() -> Int { return 1 }\n",
        )
        .unwrap();
        fs::write(
            root.join("src/b.zyl"),
            "pub fn answer() -> Int { return 2 }\n",
        )
        .unwrap();
        let analysis = language_service::analyze(&root.join("main.zyl"), &[]);
        assert!(analysis.success, "{:?}", analysis.diagnostics);
        let index = SemanticIndex::from_analysis(&analysis);
        let a = index
            .symbols
            .iter()
            .find(|symbol| symbol.path == "src/a.zyl" && symbol.name == "answer")
            .unwrap();
        let b = index
            .symbols
            .iter()
            .find(|symbol| symbol.path == "src/b.zyl" && symbol.name == "answer")
            .unwrap();
        assert_ne!(a.key, b.key);
        let a_refs = index.references(&a.key, false);
        let b_refs = index.references(&b.key, false);
        assert_eq!(a_refs.len(), 1, "{:?}", index.occurrences);
        assert_eq!(b_refs.len(), 1, "{:?}", index.occurrences);
        assert_ne!(a_refs[0].start, b_refs[0].start);
        assert_eq!(a_refs[0].path, "main.zyl");
        assert_eq!(b_refs[0].path, "main.zyl");
        fs::remove_dir_all(root).unwrap();
    }
}
