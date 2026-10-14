//! Resolved function occurrences from the compiler's linked AST. The linker
//! qualifies imported names, so identical spellings in different modules have
//! different identities. Lexing is used only to find exact source token spans.

use super::*;
use std::collections::HashMap;
use zelyra_ast::{Block, Expr, ExprKind, Span, Stmt};
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
            .map(|symbol| symbol.key.as_str())
            .collect::<HashSet<_>>();
        for function in &program.functions {
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
    known: &HashSet<&str>,
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
    known: &HashSet<&str>,
    sources: &[project::ProjectSource],
    tokens: &HashMap<u32, Vec<Token>>,
    output: &mut Vec<Occurrence>,
) {
    for statement in &block.statements {
        match statement {
            Stmt::Let { value, .. }
            | Stmt::BindOrAssign { value, .. }
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
    known: &HashSet<&str>,
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
        ExprKind::Record { fields, .. } => {
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

#[cfg(test)]
mod tests {
    use super::*;

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
