//! Resolved function and record occurrences from the compiler's linked AST.
//! The linker qualifies imported names, so identical spellings in different
//! modules have different identities. Lexing finds exact source token spans.

use super::*;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use zelyra_ast::{Block, Expr, ExprKind, Pattern, PatternKind, Program, Span, Stmt, Type};
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
    let local_function_span = symbol
        .key
        .starts_with("local:")
        .then(|| {
            let source_id = analysis
                .sources
                .iter()
                .position(|source| source.path == symbol.path)? as u32;
            analysis
                .program
                .as_ref()?
                .functions
                .iter()
                .find_map(|function| {
                    (function.span.source_id == source_id
                        && function.span.start <= symbol.declaration.start
                        && symbol.declaration.end <= function.span.end)
                        .then_some(function.span)
                })
        })
        .flatten();
    if symbol.key.starts_with("local:") && local_function_span.is_none() {
        return Err("local binding scope could not be resolved".into());
    }
    let tokens = lex(replacement).map_err(|_| "newName must be a Zelyra identifier")?;
    if !matches!((tokens.first(), tokens.get(1)),
        (Some(zelyra_lexer::Token { kind: TokenKind::Ident(name), .. }), Some(zelyra_lexer::Token { kind: TokenKind::Eof, .. })) if name == replacement)
    {
        return Err("newName must be a single non-keyword identifier".into());
    }
    if replacement == symbol.name
        || index.symbols.iter().any(|item| {
            if item.path != symbol.path || item.name != replacement {
                return false;
            }
            match (
                symbol.key.strip_prefix("local:"),
                item.key.strip_prefix("local:"),
            ) {
                (Some(target), Some(candidate)) => {
                    target.rsplit_once(':').map(|(scope, _)| scope)
                        == candidate.rsplit_once(':').map(|(scope, _)| scope)
                }
                (Some(_), None) | (None, Some(_)) => false,
                (None, None) => true,
            }
        })
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
        // Reject any same-spelled identifier until local binding identities
        // are complete across the full language scope model.
        let tokens = lex(&source.text).map_err(|_| "source is not lexable")?;
        let name_conflicts = tokens.iter().any(|token| {
            let in_scope = local_function_span
                .is_none_or(|scope| token.span.start >= scope.start && token.span.end <= scope.end);
            in_scope && matches!(&token.kind, TokenKind::Ident(name) if name == replacement)
        });
        if name_conflicts {
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
            for field in &record.fields {
                let Some(field_span) =
                    record_reference_span(source_tokens, field.span, &field.name)
                else {
                    continue;
                };
                let field_key = format!("record-field:{}:{}", record.name, field.name);
                let declaration = Occurrence {
                    key: field_key.clone(),
                    path: source.path.clone(),
                    start: field_span.start,
                    end: field_span.end,
                    declaration: true,
                };
                index.occurrences.push(declaration.clone());
                index.symbols.push(Symbol {
                    key: field_key,
                    name: field.name.clone(),
                    path: source.path.clone(),
                    detail: format!("field {name}.{}: {}", field.name, field.ty),
                    declaration,
                });
            }
        }
        let resources = program
            .tables
            .iter()
            .map(|item| ("table", item.name.as_str(), item.span))
            .chain(
                program
                    .views
                    .iter()
                    .map(|item| ("view", item.name.as_str(), item.span)),
            )
            .chain(
                program
                    .forms
                    .iter()
                    .map(|item| ("form", item.name.as_str(), item.span)),
            )
            .chain(
                program
                    .cruds
                    .iter()
                    .map(|item| ("crud", item.name.as_str(), item.span)),
            )
            .chain(
                program
                    .tableviews
                    .iter()
                    .map(|item| ("tableview", item.name.as_str(), item.span)),
            )
            .chain(
                program
                    .components
                    .iter()
                    .map(|item| ("component", item.name.as_str(), item.span)),
            )
            .collect::<Vec<_>>();
        let mut resource_name_counts = HashMap::<&str, usize>::new();
        for (_, full_name, _) in &resources {
            *resource_name_counts
                .entry(full_name.rsplit("::").next().unwrap_or(full_name))
                .or_default() += 1;
        }
        for (kind, full_name, declaration_span) in resources {
            let Some(source) = analysis.sources.get(declaration_span.source_id as usize) else {
                continue;
            };
            let Some(source_tokens) = tokens.get(&declaration_span.source_id) else {
                continue;
            };
            let name = full_name.rsplit("::").next().unwrap_or(full_name);
            let Some(declaration_span) = source_tokens
                .iter()
                .filter(|token| {
                    token.span.start >= declaration_span.start
                        && token.span.end <= declaration_span.end
                })
                .find_map(|token| match &token.kind {
                    TokenKind::Ident(found) if found == name => Some(token.span),
                    _ => None,
                })
            else {
                continue;
            };
            let key = format!("resource:{kind}:{}:{full_name}", source.path);
            let unique_name = resource_name_counts.get(name).copied().unwrap_or(0) == 1;
            if !unique_name {
                continue;
            }
            let mut references = Vec::new();
            for (source_id, candidate_source) in analysis.sources.iter().enumerate() {
                let Some(candidate_tokens) = tokens.get(&(source_id as u32)) else {
                    continue;
                };
                let candidate_spans = if kind == "component" {
                    edit::component_rename_spans(candidate_tokens, name)
                } else {
                    edit::resource_rename_spans(program, candidate_tokens, kind, name)
                };
                references.extend(
                    candidate_spans
                        .into_iter()
                        .map(|(start, end)| (candidate_source.path.clone(), start, end)),
                );
            }
            for (path, start, end) in references {
                let declaration_occurrence = path == source.path
                    && start == declaration_span.start
                    && end == declaration_span.end;
                index.occurrences.push(Occurrence {
                    key: key.clone(),
                    path,
                    start,
                    end,
                    declaration: declaration_occurrence,
                });
            }
            let declaration = Occurrence {
                key: key.clone(),
                path: source.path.clone(),
                start: declaration_span.start,
                end: declaration_span.end,
                declaration: true,
            };
            if !index.occurrences.iter().any(|item| {
                item.key == key && item.start == declaration.start && item.end == declaration.end
            }) {
                index.occurrences.push(declaration.clone());
            }
            index.symbols.push(Symbol {
                key,
                name: name.into(),
                path: source.path.clone(),
                detail: format!("{kind} {name}"),
                declaration,
            });
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
            let mut bindings = function
                .params
                .iter()
                .map(|parameter| (parameter.name.clone(), parameter.ty.clone()))
                .collect::<HashMap<_, _>>();
            visit_record_field_block(
                &function.body,
                &mut bindings,
                program,
                &analysis.sources,
                &tokens,
                &mut index.occurrences,
            );
            index_local_bindings(function, &analysis.sources, &tokens, &mut index);
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
            .filter(|symbol| {
                !symbol.key.starts_with("record:")
                    && !symbol.key.starts_with("record-field:")
                    && !symbol.key.starts_with("resource:")
                    && !symbol.key.starts_with("local:")
            })
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

/// Compiler-AST based local bindings visible at a byte offset. Studio and LSP
/// use this for ordinary completion; typed-hole completion remains stricter
/// and uses the compiler's expected-type information.
pub fn visible_local_symbols(program: &Program, source_id: u32, offset: usize) -> Vec<Value> {
    let Some(function) = program.functions.iter().find(|function| {
        function.span.source_id == source_id
            && function.body.span.start <= offset
            && offset <= function.body.span.end
    }) else {
        return Vec::new();
    };
    let mut output = function
        .params
        .iter()
        .map(|parameter| {
            json!({
                "label":parameter.name,
                "kind":"variable",
                "detail":"parameter",
                "valueType":parameter.ty.to_string(),
            })
        })
        .collect::<Vec<_>>();
    collect_visible_bindings(&function.body, offset, &mut output);
    output
}

fn collect_visible_bindings(block: &Block, offset: usize, output: &mut Vec<Value>) {
    if offset < block.span.start || offset > block.span.end {
        return;
    }
    for statement in &block.statements {
        if statement.span().start >= offset {
            continue;
        }
        match statement {
            Stmt::Let {
                name, ty, value, ..
            } => {
                collect_visible_expression_bindings(value, offset, output);
                if value.span.end <= offset {
                    output.push(json!({
                        "label":name,
                        "kind":"variable",
                        "detail":"local value",
                        "valueType":ty.as_ref().map(ToString::to_string),
                    }));
                }
            }
            Stmt::BindOrAssign { name, value, .. } => {
                collect_visible_expression_bindings(value, offset, output);
                if value.span.end <= offset
                    && !output
                        .iter()
                        .any(|item| item["label"].as_str() == Some(name.as_str()))
                {
                    output.push(json!({
                        "label":name,
                        "kind":"variable",
                        "detail":"local value",
                        "valueType":Value::Null,
                    }));
                }
            }
            Stmt::TupleBindOrAssign { names, value, .. } => {
                collect_visible_expression_bindings(value, offset, output);
                if value.span.end <= offset {
                    for name in names {
                        if !output
                            .iter()
                            .any(|item| item["label"].as_str() == Some(name.as_str()))
                        {
                            output.push(json!({
                                "label":name,
                                "kind":"variable",
                                "detail":"local value",
                                "valueType":Value::Null,
                            }));
                        }
                    }
                }
            }
            Stmt::Expr(expression) => {
                collect_visible_expression_bindings(expression, offset, output)
            }
            Stmt::Return {
                value: Some(expression),
                ..
            } => collect_visible_expression_bindings(expression, offset, output),
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                collect_visible_expression_bindings(condition, offset, output);
                collect_visible_bindings(then_block, offset, output);
                if let Some(block) = else_block {
                    collect_visible_bindings(block, offset, output);
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                collect_visible_expression_bindings(condition, offset, output);
                for invariant in invariants {
                    collect_visible_expression_bindings(invariant, offset, output);
                }
                collect_visible_bindings(body, offset, output);
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    collect_visible_expression_bindings(invariant, offset, output);
                }
                collect_visible_bindings(body, offset, output);
            }
            Stmt::For {
                name,
                additional_names,
                iterable,
                body,
                ..
            } => {
                collect_visible_expression_bindings(iterable, offset, output);
                if body.span.start <= offset && offset <= body.span.end {
                    for name in std::iter::once(name).chain(additional_names) {
                        output.push(json!({
                            "label":name,
                            "kind":"variable",
                            "detail":"loop value",
                            "valueType":Value::Null,
                        }));
                    }
                    collect_visible_bindings(body, offset, output);
                }
            }
            Stmt::Match { value, arms, .. } => {
                collect_visible_expression_bindings(value, offset, output);
                for arm in arms
                    .iter()
                    .filter(|arm| arm.span.start <= offset && offset <= arm.span.end)
                {
                    collect_pattern_bindings(&arm.pattern, output);
                    if let Some(guard) = &arm.guard {
                        collect_visible_expression_bindings(guard, offset, output);
                    }
                    collect_visible_bindings(&arm.body, offset, output);
                }
            }
            Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
                collect_visible_bindings(body, offset, output)
            }
            Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }
}

fn collect_visible_expression_bindings(expression: &Expr, offset: usize, output: &mut Vec<Value>) {
    if expression.span.start > offset || offset > expression.span.end {
        return;
    }
    match &expression.kind {
        ExprKind::Lambda { params, body } => {
            if body.span.start <= offset && offset <= body.span.end {
                output.extend(params.iter().map(|parameter| {
                    json!({
                        "label":parameter.name,
                        "kind":"variable",
                        "detail":"parameter",
                        "valueType":parameter.ty.to_string(),
                    })
                }));
                collect_visible_expression_bindings(body, offset, output);
            }
        }
        ExprKind::Array(items) => {
            for item in items {
                collect_visible_expression_bindings(item, offset, output);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                collect_visible_expression_bindings(key, offset, output);
                collect_visible_expression_bindings(value, offset, output);
            }
        }
        ExprKind::Record { fields, .. } => {
            for (_, value) in fields {
                collect_visible_expression_bindings(value, offset, output);
            }
        }
        ExprKind::Index { target, index } => {
            collect_visible_expression_bindings(target, offset, output);
            collect_visible_expression_bindings(index, offset, output);
        }
        ExprKind::Field { target, .. }
        | ExprKind::Await(target)
        | ExprKind::Unary { expr: target, .. } => {
            collect_visible_expression_bindings(target, offset, output);
        }
        ExprKind::Call { args, .. } => {
            for argument in args {
                collect_visible_expression_bindings(argument, offset, output);
            }
        }
        ExprKind::Binary { left, right, .. } => {
            collect_visible_expression_bindings(left, offset, output);
            collect_visible_expression_bindings(right, offset, output);
        }
        ExprKind::Match { value, arms } => {
            collect_visible_expression_bindings(value, offset, output);
            for arm in arms
                .iter()
                .filter(|arm| arm.span.start <= offset && offset <= arm.span.end)
            {
                collect_pattern_bindings(&arm.pattern, output);
                if let Some(guard) = &arm.guard {
                    collect_visible_expression_bindings(guard, offset, output);
                }
                collect_visible_expression_bindings(&arm.value, offset, output);
            }
        }
        ExprKind::Int(_)
        | ExprKind::UInt(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::String(_)
        | ExprKind::Char(_)
        | ExprKind::FunctionValue(_)
        | ExprKind::Variable(_)
        | ExprKind::Sql { .. } => {}
    }
}

fn collect_pattern_bindings(pattern: &Pattern, output: &mut Vec<Value>) {
    match &pattern.kind {
        PatternKind::Variable(name) => output.push(json!({
            "label":name,"kind":"variable","detail":"match value","valueType":Value::Null,
        })),
        PatternKind::Constructor {
            inner: Some(inner), ..
        } => collect_pattern_bindings(inner, output),
        PatternKind::EnumVariantPayload { payloads, .. } => {
            for nested in payloads {
                collect_pattern_bindings(nested, output);
            }
        }
        PatternKind::Record { fields, .. } => {
            for (_, nested) in fields {
                collect_pattern_bindings(nested, output);
            }
        }
        PatternKind::Wildcard
        | PatternKind::Constructor { inner: None, .. }
        | PatternKind::EnumVariant { .. }
        | PatternKind::Int(_)
        | PatternKind::Bool(_)
        | PatternKind::String(_)
        | PatternKind::Char(_) => {}
    }
}

fn index_local_bindings(
    function: &zelyra_ast::Function,
    sources: &[project::ProjectSource],
    tokens: &HashMap<u32, Vec<Token>>,
    index: &mut SemanticIndex,
) {
    let Some(source) = sources.get(function.span.source_id as usize) else {
        return;
    };
    let Some(source_tokens) = tokens.get(&function.span.source_id) else {
        return;
    };
    let identity = LocalIdentity {
        function: function.name.rsplit("::").next().unwrap_or(&function.name),
        source,
        tokens: source_tokens,
    };
    let mut scopes = vec![HashMap::<String, String>::new()];
    for parameter in &function.params {
        declare_local(
            &parameter.name,
            parameter.span,
            &identity,
            &mut scopes,
            index,
            "parameter",
        );
    }
    for contract in function.requires.iter().chain(&function.ensures) {
        visit_local_expression(contract, &mut scopes, source, source_tokens, index);
    }
    visit_local_block(
        &function.body,
        false,
        function.name.rsplit("::").next().unwrap_or(&function.name),
        source,
        source_tokens,
        &mut scopes,
        index,
    );
}

struct LocalIdentity<'a> {
    function: &'a str,
    source: &'a project::ProjectSource,
    tokens: &'a [Token],
}

fn local_identifier_span(
    tokens: &[Token],
    span: Span,
    name: &str,
    before: Option<usize>,
) -> Option<Span> {
    tokens
        .iter()
        .filter(|token| {
            token.span.start >= span.start
                && token.span.end <= span.end
                && before.is_none_or(|limit| token.span.end <= limit)
        })
        .find_map(|token| match &token.kind {
            TokenKind::Ident(found) if found == name => Some(token.span),
            _ => None,
        })
}

fn declare_local(
    name: &str,
    span: Span,
    identity: &LocalIdentity<'_>,
    scopes: &mut [HashMap<String, String>],
    index: &mut SemanticIndex,
    kind: &str,
) {
    let Some(name_span) = local_identifier_span(identity.tokens, span, name, None) else {
        return;
    };
    let key = format!(
        "local:{}:{}:{}",
        identity.source.path, identity.function, name_span.start
    );
    let occurrence = Occurrence {
        key: key.clone(),
        path: identity.source.path.clone(),
        start: name_span.start,
        end: name_span.end,
        declaration: true,
    };
    scopes
        .last_mut()
        .expect("a function always has a scope")
        .insert(name.into(), key.clone());
    if !index.symbols.iter().any(|symbol| symbol.key == key) {
        index.occurrences.push(occurrence.clone());
        index.symbols.push(Symbol {
            key,
            name: name.into(),
            path: identity.source.path.clone(),
            detail: format!("{kind} {name}"),
            declaration: occurrence,
        });
    }
}

fn use_local(
    name: &str,
    span: Span,
    source: &project::ProjectSource,
    tokens: &[Token],
    scopes: &[HashMap<String, String>],
    index: &mut SemanticIndex,
) {
    let Some(key) = scopes.iter().rev().find_map(|scope| scope.get(name)) else {
        return;
    };
    let Some(name_span) = local_identifier_span(tokens, span, name, None) else {
        return;
    };
    index.occurrences.push(Occurrence {
        key: key.clone(),
        path: source.path.clone(),
        start: name_span.start,
        end: name_span.end,
        declaration: false,
    });
}

fn visit_local_block(
    block: &Block,
    create_scope: bool,
    function: &str,
    source: &project::ProjectSource,
    tokens: &[Token],
    scopes: &mut Vec<HashMap<String, String>>,
    index: &mut SemanticIndex,
) {
    if create_scope {
        scopes.push(HashMap::new());
    }
    for statement in &block.statements {
        match statement {
            Stmt::Let {
                name, value, span, ..
            } => {
                visit_local_expression(value, scopes, source, tokens, index);
                let declaration_span = Span {
                    end: value.span.start,
                    ..*span
                };
                if let Some(name_span) = local_identifier_span(tokens, declaration_span, name, None)
                {
                    declare_local(
                        name,
                        name_span,
                        &LocalIdentity {
                            function,
                            source,
                            tokens,
                        },
                        scopes,
                        index,
                        "local",
                    );
                }
            }
            Stmt::BindOrAssign { name, value, span } => {
                visit_local_expression(value, scopes, source, tokens, index);
                if let Some(name_span) =
                    local_identifier_span(tokens, *span, name, Some(value.span.start))
                {
                    if let Some(key) = scopes
                        .iter()
                        .rev()
                        .find_map(|scope| scope.get(name))
                        .cloned()
                    {
                        index.occurrences.push(Occurrence {
                            key,
                            path: source.path.clone(),
                            start: name_span.start,
                            end: name_span.end,
                            declaration: false,
                        });
                    } else {
                        declare_local(
                            name,
                            name_span,
                            &LocalIdentity {
                                function,
                                source,
                                tokens,
                            },
                            scopes,
                            index,
                            "local",
                        );
                    }
                }
            }
            Stmt::TupleBindOrAssign { names, value, span } => {
                visit_local_expression(value, scopes, source, tokens, index);
                let mut cursor = span.start;
                for name in names {
                    let name_span = Span {
                        start: cursor,
                        end: value.span.start,
                        ..*span
                    };
                    if let Some(name_span) = local_identifier_span(tokens, name_span, name, None) {
                        cursor = name_span.end;
                        if let Some(key) = scopes
                            .iter()
                            .rev()
                            .find_map(|scope| scope.get(name))
                            .cloned()
                        {
                            index.occurrences.push(Occurrence {
                                key,
                                path: source.path.clone(),
                                start: name_span.start,
                                end: name_span.end,
                                declaration: false,
                            });
                        } else {
                            declare_local(
                                name,
                                name_span,
                                &LocalIdentity {
                                    function,
                                    source,
                                    tokens,
                                },
                                scopes,
                                index,
                                "local",
                            );
                        }
                    }
                }
            }
            Stmt::Expr(expression) => {
                visit_local_expression(expression, scopes, source, tokens, index)
            }
            Stmt::Return {
                value: Some(expression),
                ..
            } => visit_local_expression(expression, scopes, source, tokens, index),
            Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                visit_local_expression(condition, scopes, source, tokens, index);
                visit_local_block(then_block, true, function, source, tokens, scopes, index);
                if let Some(else_block) = else_block {
                    visit_local_block(else_block, true, function, source, tokens, scopes, index);
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                visit_local_expression(condition, scopes, source, tokens, index);
                for invariant in invariants {
                    visit_local_expression(invariant, scopes, source, tokens, index);
                }
                visit_local_block(body, true, function, source, tokens, scopes, index);
            }
            Stmt::For {
                name,
                additional_names,
                iterable,
                body,
                span,
            } => {
                visit_local_expression(iterable, scopes, source, tokens, index);
                scopes.push(HashMap::new());
                let names = std::iter::once(name).chain(additional_names);
                let mut cursor = span.start;
                for name in names {
                    let declaration_span = Span {
                        start: cursor,
                        end: iterable.span.start,
                        ..*span
                    };
                    if let Some(name_span) =
                        local_identifier_span(tokens, declaration_span, name, None)
                    {
                        cursor = name_span.end;
                        declare_local(
                            name,
                            name_span,
                            &LocalIdentity {
                                function,
                                source,
                                tokens,
                            },
                            scopes,
                            index,
                            "loop variable",
                        );
                    }
                }
                visit_local_block(body, false, function, source, tokens, scopes, index);
                scopes.pop();
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    visit_local_expression(invariant, scopes, source, tokens, index);
                }
                visit_local_block(body, true, function, source, tokens, scopes, index);
            }
            Stmt::Match { value, arms, .. } => {
                visit_local_expression(value, scopes, source, tokens, index);
                for arm in arms {
                    scopes.push(HashMap::new());
                    visit_local_pattern(&arm.pattern, function, source, tokens, scopes, index);
                    if let Some(guard) = &arm.guard {
                        visit_local_expression(guard, scopes, source, tokens, index);
                    }
                    visit_local_block(&arm.body, false, function, source, tokens, scopes, index);
                    scopes.pop();
                }
            }
            Stmt::Transaction { body, .. } => {
                visit_local_block(body, true, function, source, tokens, scopes, index)
            }
            Stmt::Parallel { body, .. } => {
                visit_local_block(body, false, function, source, tokens, scopes, index)
            }
        }
    }
    if create_scope {
        scopes.pop();
    }
}

fn visit_local_pattern(
    pattern: &Pattern,
    function: &str,
    source: &project::ProjectSource,
    tokens: &[Token],
    scopes: &mut Vec<HashMap<String, String>>,
    index: &mut SemanticIndex,
) {
    match &pattern.kind {
        PatternKind::Variable(name) => {
            declare_local(
                name,
                pattern.span,
                &LocalIdentity {
                    function,
                    source,
                    tokens,
                },
                scopes,
                index,
                "pattern binding",
            );
        }
        PatternKind::Constructor {
            inner: Some(inner), ..
        } => visit_local_pattern(inner, function, source, tokens, scopes, index),
        PatternKind::EnumVariantPayload { payloads, .. } => {
            for nested in payloads {
                visit_local_pattern(nested, function, source, tokens, scopes, index);
            }
        }
        PatternKind::Record { fields, .. } => {
            for (_, nested) in fields {
                visit_local_pattern(nested, function, source, tokens, scopes, index);
            }
        }
        PatternKind::Wildcard
        | PatternKind::Constructor { inner: None, .. }
        | PatternKind::EnumVariant { .. }
        | PatternKind::Int(_)
        | PatternKind::Bool(_)
        | PatternKind::String(_)
        | PatternKind::Char(_) => {}
    }
}

fn visit_local_expression(
    expression: &Expr,
    scopes: &mut Vec<HashMap<String, String>>,
    source: &project::ProjectSource,
    tokens: &[Token],
    index: &mut SemanticIndex,
) {
    match &expression.kind {
        ExprKind::Variable(name) => use_local(name, expression.span, source, tokens, scopes, index),
        ExprKind::Lambda { params, body } => {
            scopes.push(HashMap::new());
            for parameter in params {
                declare_local(
                    parameter.name.as_str(),
                    parameter.span,
                    &LocalIdentity {
                        function: "lambda",
                        source,
                        tokens,
                    },
                    scopes,
                    index,
                    "parameter",
                );
            }
            visit_local_expression(body, scopes, source, tokens, index);
            scopes.pop();
        }
        ExprKind::Array(items) => {
            for item in items {
                visit_local_expression(item, scopes, source, tokens, index);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                visit_local_expression(key, scopes, source, tokens, index);
                visit_local_expression(value, scopes, source, tokens, index);
            }
        }
        ExprKind::Record { fields, .. } => {
            for (_, value) in fields {
                visit_local_expression(value, scopes, source, tokens, index);
            }
        }
        ExprKind::Index {
            target,
            index: subscript,
        } => {
            visit_local_expression(target, scopes, source, tokens, index);
            visit_local_expression(subscript, scopes, source, tokens, index);
        }
        ExprKind::Field { target, .. }
        | ExprKind::Await(target)
        | ExprKind::Unary { expr: target, .. } => {
            visit_local_expression(target, scopes, source, tokens, index);
        }
        ExprKind::Call { args, .. } => {
            for argument in args {
                visit_local_expression(argument, scopes, source, tokens, index);
            }
        }
        ExprKind::Binary { left, right, .. } => {
            visit_local_expression(left, scopes, source, tokens, index);
            visit_local_expression(right, scopes, source, tokens, index);
        }
        ExprKind::Match { value, arms } => {
            visit_local_expression(value, scopes, source, tokens, index);
            for arm in arms {
                scopes.push(HashMap::new());
                visit_local_pattern(&arm.pattern, "match", source, tokens, scopes, index);
                if let Some(guard) = &arm.guard {
                    visit_local_expression(guard, scopes, source, tokens, index);
                }
                visit_local_expression(&arm.value, scopes, source, tokens, index);
                scopes.pop();
            }
        }
        ExprKind::Int(_)
        | ExprKind::UInt(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::String(_)
        | ExprKind::Char(_)
        | ExprKind::FunctionValue(_)
        | ExprKind::Sql { .. } => {}
    }
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

fn visit_record_field_block(
    block: &Block,
    bindings: &mut HashMap<String, Type>,
    program: &Program,
    sources: &[project::ProjectSource],
    tokens: &HashMap<u32, Vec<Token>>,
    output: &mut Vec<Occurrence>,
) {
    for statement in &block.statements {
        match statement {
            Stmt::Let {
                name, ty, value, ..
            } => {
                visit_record_field_expr(value, bindings, program, sources, tokens, output);
                if let Some(ty) = ty
                    .clone()
                    .or_else(|| expression_type(value, bindings, program))
                {
                    bindings.insert(name.clone(), ty);
                }
            }
            Stmt::BindOrAssign { name, value, .. } => {
                visit_record_field_expr(value, bindings, program, sources, tokens, output);
                if let Some(ty) = expression_type(value, bindings, program) {
                    bindings.insert(name.clone(), ty);
                }
            }
            Stmt::TupleBindOrAssign { value, .. } | Stmt::Expr(value) => {
                visit_record_field_expr(value, bindings, program, sources, tokens, output);
            }
            Stmt::Return {
                value: Some(value), ..
            } => visit_record_field_expr(value, bindings, program, sources, tokens, output),
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                visit_record_field_expr(condition, bindings, program, sources, tokens, output);
                let mut branch = bindings.clone();
                visit_record_field_block(then_block, &mut branch, program, sources, tokens, output);
                if let Some(otherwise) = else_block {
                    let mut branch = bindings.clone();
                    visit_record_field_block(
                        otherwise,
                        &mut branch,
                        program,
                        sources,
                        tokens,
                        output,
                    );
                }
            }
            Stmt::While {
                condition,
                invariants,
                body,
                ..
            } => {
                visit_record_field_expr(condition, bindings, program, sources, tokens, output);
                for invariant in invariants {
                    visit_record_field_expr(invariant, bindings, program, sources, tokens, output);
                }
                let mut nested = bindings.clone();
                visit_record_field_block(body, &mut nested, program, sources, tokens, output);
            }
            Stmt::For {
                name,
                iterable,
                body,
                ..
            } => {
                visit_record_field_expr(iterable, bindings, program, sources, tokens, output);
                let mut nested = bindings.clone();
                if let Some(Type::Array(inner) | Type::Set(inner)) =
                    expression_type(iterable, bindings, program)
                {
                    nested.insert(name.clone(), *inner);
                }
                visit_record_field_block(body, &mut nested, program, sources, tokens, output);
            }
            Stmt::Loop {
                invariants, body, ..
            } => {
                for invariant in invariants {
                    visit_record_field_expr(invariant, bindings, program, sources, tokens, output);
                }
                let mut nested = bindings.clone();
                visit_record_field_block(body, &mut nested, program, sources, tokens, output);
            }
            Stmt::Match { value, arms, .. } => {
                visit_record_field_expr(value, bindings, program, sources, tokens, output);
                for arm in arms {
                    let mut nested = bindings.clone();
                    if let Some(guard) = &arm.guard {
                        visit_record_field_expr(
                            guard,
                            &mut nested,
                            program,
                            sources,
                            tokens,
                            output,
                        );
                    }
                    visit_record_field_block(
                        &arm.body,
                        &mut nested,
                        program,
                        sources,
                        tokens,
                        output,
                    );
                }
            }
            Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
                let mut nested = bindings.clone();
                visit_record_field_block(body, &mut nested, program, sources, tokens, output);
            }
            Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }
}

fn visit_record_field_expr(
    expression: &Expr,
    bindings: &mut HashMap<String, Type>,
    program: &Program,
    sources: &[project::ProjectSource],
    tokens: &HashMap<u32, Vec<Token>>,
    output: &mut Vec<Occurrence>,
) {
    macro_rules! visit_child {
        ($child:expr) => {
            visit_record_field_expr($child, bindings, program, sources, tokens, output)
        };
    }
    match &expression.kind {
        ExprKind::Record {
            type_name, fields, ..
        } => {
            let record_type = Type::Named(type_name.clone());
            if let Some(record) = resolve_record_type(&record_type, program, 0) {
                for (name, value) in fields {
                    push_record_field_use(record, name, expression.span, sources, tokens, output);
                    visit_child!(value);
                }
            } else {
                for (_, value) in fields {
                    visit_child!(value);
                }
            }
        }
        ExprKind::Field { target, field } => {
            if let Some(ty) = expression_type(target, bindings, program) {
                if let Some(record) = resolve_record_type(&ty, program, 0) {
                    push_record_field_use(record, field, expression.span, sources, tokens, output);
                }
            }
            visit_child!(target);
        }
        ExprKind::Call { args, .. } | ExprKind::Array(args) => {
            for argument in args {
                visit_child!(argument);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                visit_child!(key);
                visit_child!(value);
            }
        }
        ExprKind::Lambda { params, body } => {
            let mut nested = bindings.clone();
            nested.extend(
                params
                    .iter()
                    .map(|parameter| (parameter.name.clone(), parameter.ty.clone())),
            );
            visit_record_field_expr(body, &mut nested, program, sources, tokens, output);
        }
        ExprKind::Await(inner) | ExprKind::Unary { expr: inner, .. } => visit_child!(inner),
        ExprKind::Index { target, index } => {
            visit_child!(target);
            visit_child!(index);
        }
        ExprKind::Binary { left, right, .. } => {
            visit_child!(left);
            visit_child!(right);
        }
        ExprKind::Match { value, arms } => {
            visit_child!(value);
            for arm in arms {
                let mut nested = bindings.clone();
                if let Some(guard) = &arm.guard {
                    visit_record_field_expr(guard, &mut nested, program, sources, tokens, output);
                }
                visit_record_field_expr(&arm.value, &mut nested, program, sources, tokens, output);
            }
        }
        ExprKind::Int(_)
        | ExprKind::UInt(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::String(_)
        | ExprKind::Char(_)
        | ExprKind::Variable(_)
        | ExprKind::FunctionValue(_)
        | ExprKind::Sql { .. } => {}
    }
}

fn expression_type(
    expression: &Expr,
    bindings: &HashMap<String, Type>,
    program: &Program,
) -> Option<Type> {
    match &expression.kind {
        ExprKind::Variable(name) => bindings.get(name).cloned(),
        ExprKind::Record { type_name, .. } => Some(Type::Named(type_name.clone())),
        ExprKind::Index { target, .. } => match expression_type(target, bindings, program)? {
            Type::Array(inner) | Type::Set(inner) => Some(*inner),
            _ => None,
        },
        ExprKind::Field { target, field } => {
            let record =
                resolve_record_type(&expression_type(target, bindings, program)?, program, 0)?;
            record
                .fields
                .iter()
                .find(|candidate| candidate.name == *field)
                .map(|candidate| candidate.ty.clone())
        }
        _ => None,
    }
}

fn resolve_record_type<'a>(
    ty: &Type,
    program: &'a Program,
    depth: usize,
) -> Option<&'a zelyra_ast::RecordDef> {
    if depth > 12 {
        return None;
    }
    let name = match ty {
        Type::Named(name) | Type::Applied { name, .. } => name,
        _ => return None,
    };
    if let Some(record) = program.records.iter().find(|record| record.name == *name) {
        return Some(record);
    }
    let short = name.rsplit("::").next().unwrap_or(name);
    let mut matches = program
        .records
        .iter()
        .filter(|record| record.name == short || record.name.ends_with(&format!("::{short}")));
    if let Some(record) = matches.next() {
        return matches.next().is_none().then_some(record);
    }
    if let Some(alias) = program.types.iter().find(|alias| alias.name == *name) {
        return resolve_record_type(&alias.target, program, depth + 1);
    }
    let mut aliases = program
        .types
        .iter()
        .filter(|alias| alias.name == short || alias.name.ends_with(&format!("::{short}")));
    let alias = aliases.next()?;
    if aliases.next().is_some() {
        return None;
    }
    resolve_record_type(&alias.target, program, depth + 1)
}

fn push_record_field_use(
    record: &zelyra_ast::RecordDef,
    name: &str,
    span: Span,
    sources: &[project::ProjectSource],
    tokens: &HashMap<u32, Vec<Token>>,
    output: &mut Vec<Occurrence>,
) {
    let Some(field) = record.fields.iter().find(|field| field.name == name) else {
        return;
    };
    let Some(source) = sources.get(span.source_id as usize) else {
        return;
    };
    let Some(source_tokens) = tokens.get(&span.source_id) else {
        return;
    };
    let Some(field_span) = last_reference_span(source_tokens, span, name) else {
        return;
    };
    output.push(Occurrence {
        key: format!("record-field:{}:{}", record.name, field.name),
        path: source.path.clone(),
        start: field_span.start,
        end: field_span.end,
        declaration: false,
    });
}

fn last_reference_span(tokens: &[Token], expression: Span, name: &str) -> Option<Span> {
    tokens
        .iter()
        .filter(|token| token.span.start >= expression.start && token.span.end <= expression.end)
        .filter_map(|token| match &token.kind {
            TokenKind::Ident(found) if found == name => Some(token.span),
            _ => None,
        })
        .next_back()
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
    fn local_bindings_resolve_and_rename_only_their_lexical_references() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-index-local-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("zelyra.toml"),
            "[project]\nname = \"local-index\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        let source = "fn add(amount: Int) -> Int { result = amount + 1 return result }\nfn bump(initial: Int) -> Int { mutable state: Int = initial state = state + 1 return state }\nfn unwrap(value: Option<Int>) -> Int { match value { Some(item) => { return item } None => { return 0 } } }\nfn transform() -> Int { factor = 3 values = [1, 2] transformed = values |> map(|n| n * factor) return transformed[0] }\nfn main() { print(add(2)) }\n";
        fs::write(root.join("main.zyl"), source).unwrap();
        let entry = root.join("main.zyl");
        let analysis = language_service::analyze(&entry, &[]);
        assert!(analysis.success, "{:?}", analysis.diagnostics);
        let index = SemanticIndex::from_analysis(&analysis);

        let parameter = index
            .at("main.zyl", source.find("amount:").unwrap())
            .unwrap();
        let parameter_use = index
            .at("main.zyl", source.find("amount +").unwrap())
            .unwrap();
        assert!(parameter.key.starts_with("local:"));
        assert_eq!(parameter.key, parameter_use.key);
        let parameter_symbol = index.symbol(&parameter.key).unwrap();
        assert_eq!(index.references(&parameter.key, false).len(), 1);
        let parameter_change =
            plan_symbol_rename(&entry, &analysis, &index, parameter_symbol, "initial", &[])
                .unwrap();
        assert!(parameter_change[0].after.contains("initial: Int"));
        assert!(parameter_change[0].after.contains("initial + 1"));

        let local = index
            .at("main.zyl", source.find("result =").unwrap())
            .unwrap();
        let local_use = index
            .at("main.zyl", source.rfind("result").unwrap())
            .unwrap();
        assert_eq!(local.key, local_use.key);
        assert_eq!(index.references(&local.key, false).len(), 1);
        let local_change = plan_symbol_rename(
            &entry,
            &analysis,
            &index,
            index.symbol(&local.key).unwrap(),
            "total",
            &[],
        )
        .unwrap();
        assert!(local_change[0].after.contains("total = amount + 1"));
        assert!(local_change[0].after.contains("return total"));

        let mutable = index
            .at("main.zyl", source.find("state: Int").unwrap())
            .unwrap();
        let mutable_use = index
            .at("main.zyl", source.find("state = state").unwrap())
            .unwrap();
        let mutable_read = index
            .at(
                "main.zyl",
                source.find("return state").unwrap() + "return ".len(),
            )
            .unwrap();
        assert_eq!(mutable.key, mutable_use.key);
        assert_eq!(mutable.key, mutable_read.key);
        assert_eq!(index.references(&mutable.key, false).len(), 3);

        let pattern = index
            .at(
                "main.zyl",
                source.find("Some(item)").unwrap() + "Some(".len(),
            )
            .unwrap();
        let pattern_use = index
            .at(
                "main.zyl",
                source.find("return item").unwrap() + "return ".len(),
            )
            .unwrap();
        assert_eq!(pattern.key, pattern_use.key);
        assert_eq!(index.references(&pattern.key, false).len(), 1);

        let lambda_parameter = index
            .at("main.zyl", source.find("|n|").unwrap() + 1)
            .unwrap();
        let lambda_use = index
            .at("main.zyl", source.find("n * factor").unwrap())
            .unwrap();
        assert_eq!(lambda_parameter.key, lambda_use.key);
        assert_eq!(index.references(&lambda_parameter.key, false).len(), 1);
        let captured_value = index
            .at("main.zyl", source.find("factor").unwrap())
            .unwrap();
        let captured_use = index
            .at("main.zyl", source.rfind("factor").unwrap())
            .unwrap();
        assert_eq!(captured_value.key, captured_use.key);
        assert_eq!(index.references(&captured_value.key, false).len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn record_field_rename_follows_typed_receivers_and_record_literals() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-index-record-field-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("zelyra.toml"),
            "[project]\nname = \"record-field-rename\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        let source = "struct Customer { name: String }\nfn main() { customer: Customer = Customer { name: \"Ada\" } print(customer.name) }\n";
        fs::write(root.join("main.zyl"), source).unwrap();
        let entry = root.join("main.zyl");
        let analysis = language_service::analyze(&entry, &[]);
        assert!(analysis.success, "{:?}", analysis.diagnostics);
        let index = SemanticIndex::from_analysis(&analysis);
        let field_declaration = source.find("name: String").unwrap();
        let occurrence = index.at("main.zyl", field_declaration).unwrap();
        assert!(occurrence.key.starts_with("record-field:"));
        let symbol = index.symbol(&occurrence.key).unwrap();
        assert_eq!(index.references(&symbol.key, true).len(), 3);
        let changes = plan_symbol_rename(&entry, &analysis, &index, symbol, "full_name", &[])
            .expect("typed field access and record literal should rename together");
        assert_eq!(changes.len(), 1);
        assert!(changes[0]
            .after
            .contains("struct Customer { full_name: String }"));
        assert!(changes[0]
            .after
            .contains("Customer { full_name: \"Ada\" } print(customer.full_name)"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn record_field_rename_rejects_unindexed_dynamic_receiver() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-index-dynamic-field-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("zelyra.toml"),
            "[project]\nname = \"dynamic-field-rename\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        let source = "struct Customer { name: String }\nfn load_customer() -> Customer { return Customer { name: \"Eve\" } }\nfn main() { customer: Customer = Customer { name: \"Ada\" } print(customer.name) loaded_customer = load_customer() print(loaded_customer.name) }\n";
        fs::write(root.join("main.zyl"), source).unwrap();
        let entry = root.join("main.zyl");
        let analysis = language_service::analyze(&entry, &[]);
        assert!(analysis.success, "{:?}", analysis.diagnostics);
        let index = SemanticIndex::from_analysis(&analysis);
        let occurrence = index
            .at("main.zyl", source.find("name: String").unwrap())
            .unwrap();
        let symbol = index.symbol(&occurrence.key).unwrap();
        assert!(
            plan_symbol_rename(&entry, &analysis, &index, symbol, "full_name", &[])
                .unwrap_err()
                .contains("renamed project does not compile")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resource_rename_follows_same_module_form_references() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-index-table-rename-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("zelyra.toml"),
            "[project]\nname = \"table-rename\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        let source = "table customers { id: Id primary auto name: String(100) required }\nform CustomerCreate -> customers { fields { name } }\nfn main() {}\n";
        fs::write(root.join("main.zyl"), source).unwrap();
        let entry = root.join("main.zyl");
        let analysis = language_service::analyze(&entry, &[]);
        assert!(analysis.success, "{:?}", analysis.diagnostics);
        let index = SemanticIndex::from_analysis(&analysis);
        let occurrence = index
            .at("main.zyl", source.find("customers {").unwrap())
            .unwrap();
        assert!(occurrence.key.starts_with("resource:table:"));
        let symbol = index.symbol(&occurrence.key).unwrap();
        assert_eq!(index.references(&symbol.key, true).len(), 2);
        let changes = plan_symbol_rename(&entry, &analysis, &index, symbol, "clients", &[])
            .expect("table and form source should be renamed together");
        assert_eq!(changes.len(), 1);
        assert!(changes[0].after.contains("table clients {"));
        assert!(changes[0].after.contains("CustomerCreate -> clients"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resource_rename_follows_unique_table_references_across_imported_files() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-index-table-cross-file-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("zelyra.toml"),
            "[project]\nname = \"table-cross-file-rename\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        let main = "import \"src/schema.zyl\" as schema\nform CustomerCreate -> customers { fields { name } }\nfn main() {}\n";
        let schema = "table customers { id: Id primary auto name: String(100) required access { read_write: [\"main.zyl\"] } }\n";
        fs::write(root.join("main.zyl"), main).unwrap();
        fs::write(root.join("src/schema.zyl"), schema).unwrap();
        let entry = root.join("main.zyl");
        let analysis = language_service::analyze(&entry, &[]);
        assert!(analysis.success, "{:?}", analysis.diagnostics);
        let index = SemanticIndex::from_analysis(&analysis);
        let occurrence = index
            .at("src/schema.zyl", schema.find("customers").unwrap())
            .unwrap();
        assert!(occurrence.key.starts_with("resource:table:"));
        let symbol = index.symbol(&occurrence.key).unwrap();
        let references = index.references(&symbol.key, true);
        assert_eq!(references.len(), 2);
        assert!(references.iter().any(|item| item.path == "main.zyl"));
        let changes = plan_symbol_rename(&entry, &analysis, &index, symbol, "clients", &[])
            .expect("unique table and form references should rename together");
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().any(|change| {
            change.path == "src/schema.zyl" && change.after.contains("table clients {")
        }));
        assert!(changes.iter().any(|change| {
            change.path == "main.zyl" && change.after.contains("CustomerCreate -> clients")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn component_rename_follows_html_opening_tags_in_the_same_source() {
        let root = std::env::temp_dir().join(format!(
            "zelyra-index-component-rename-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("zelyra.toml"),
            "[project]\nname = \"component-rename\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        let source = "component Banner { html { <aside><slot /></aside> } }\npage \"/\" { html { <Banner /> } }\nfn main() {}\n";
        fs::write(root.join("main.zyl"), source).unwrap();
        let entry = root.join("main.zyl");
        let analysis = language_service::analyze(&entry, &[]);
        assert!(analysis.success, "{:?}", analysis.diagnostics);
        let index = SemanticIndex::from_analysis(&analysis);
        let occurrence = index
            .at("main.zyl", source.find("Banner").unwrap())
            .unwrap();
        assert!(occurrence.key.starts_with("resource:component:"));
        let symbol = index.symbol(&occurrence.key).unwrap();
        assert_eq!(index.references(&symbol.key, true).len(), 2);
        let changes = plan_symbol_rename(&entry, &analysis, &index, symbol, "Panel", &[])
            .expect("component declaration and HTML use should rename together");
        assert_eq!(changes.len(), 1);
        assert!(changes[0].after.contains("component Panel"));
        assert!(changes[0].after.contains("<Panel />"));
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
