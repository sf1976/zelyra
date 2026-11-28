//! Static information-flow control (`@pii`, `@secret`, `@redact`) and GDPR inventory.
//!
//! Enforces fail-closed `E-PRIVACY-001` checks so secrets never flow to logs,
//! errors, redaction calls, or responses, and personal data never flows to
//! unauthenticated routes or plain logs without explicit `redact(...)`.

use std::collections::{HashMap, HashSet};
use zelyra_ast::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivacyError {
    pub code: &'static str,
    pub message: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivacyColumnEntry {
    pub table: String,
    pub column: String,
    pub ty: String,
    pub privacy: PrivacyClass,
    pub redact: bool,
    pub tenant_scoped: bool,
    pub is_ledger: bool,
    pub is_auditable: bool,
    pub has_delete_guard: bool,
    pub foreign_keys: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivacyRecordFieldEntry {
    pub record: String,
    pub field: String,
    pub ty: String,
    pub privacy: PrivacyClass,
    pub redact: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivacyRouteEntry {
    pub kind: &'static str,
    pub target: String,
    pub requires_auth: bool,
    pub permissions: Vec<String>,
    pub privacy_level: PrivacyClass,
    pub tables: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivacyExternalSinkEntry {
    pub function: String,
    pub capability: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivacyInventory {
    pub columns: Vec<PrivacyColumnEntry>,
    pub record_fields: Vec<PrivacyRecordFieldEntry>,
    pub routes: Vec<PrivacyRouteEntry>,
    pub external_sinks: Vec<PrivacyExternalSinkEntry>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct TaintVal {
    summary: PrivacyClass,
    fields: HashMap<String, PrivacyClass>,
    /// Tracks which function parameter indices contributed to this value
    /// for inter-procedural taint propagation.
    param_deps: HashSet<usize>,
}

impl TaintVal {
    fn clean() -> Self {
        Self::default()
    }

    fn of(summary: PrivacyClass) -> Self {
        Self {
            summary,
            fields: HashMap::new(),
            param_deps: HashSet::new(),
        }
    }

    fn join(a: &Self, b: &Self) -> Self {
        let mut fields = a.fields.clone();
        for (k, v) in &b.fields {
            fields
                .entry(k.clone())
                .and_modify(|cur| *cur = max_privacy(*cur, *v))
                .or_insert(*v);
        }
        let mut deps = a.param_deps.clone();
        deps.extend(b.param_deps.iter().copied());
        Self {
            summary: max_privacy(a.summary, b.summary),
            fields,
            param_deps: deps,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SinkFlags {
    log_sink: bool,
    error_sink: bool,
    external_sink: bool,
    redact_sink: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct FunctionSummary {
    return_intrinsic: PrivacyClass,
    return_param_deps: HashSet<usize>,
    param_sinks: Vec<SinkFlags>,
}

impl FunctionSummary {
    fn join(mut self, previous: &Self) -> Self {
        self.return_intrinsic = max_privacy(self.return_intrinsic, previous.return_intrinsic);
        self.return_param_deps.extend(&previous.return_param_deps);
        for (current, previous) in self.param_sinks.iter_mut().zip(&previous.param_sinks) {
            current.log_sink |= previous.log_sink;
            current.error_sink |= previous.error_sink;
            current.external_sink |= previous.external_sink;
            current.redact_sink |= previous.redact_sink;
        }
        self
    }
}

fn max_privacy(a: PrivacyClass, b: PrivacyClass) -> PrivacyClass {
    match (a, b) {
        (PrivacyClass::Secret, _) | (_, PrivacyClass::Secret) => PrivacyClass::Secret,
        (PrivacyClass::Pii, _) | (_, PrivacyClass::Pii) => PrivacyClass::Pii,
        _ => PrivacyClass::Public,
    }
}

fn unwrap_inner_type_name(ty: &Type) -> Option<&str> {
    match ty {
        Type::Named(name) => Some(name.as_str()),
        Type::Applied { name, args }
            if matches!(name.as_str(), "ApiResponse" | "Result" | "Option" | "Array") =>
        {
            args.first().and_then(unwrap_inner_type_name)
        }
        Type::Applied { name, .. } => Some(name.as_str()),
        Type::Option(inner) | Type::Array(inner) => unwrap_inner_type_name(inner),
        Type::Result(ok, _) => unwrap_inner_type_name(ok),
        _ => None,
    }
}

pub(crate) fn program_uses_privacy(program: &Program) -> bool {
    program.tables.iter().any(|t| {
        t.columns
            .iter()
            .any(|c| c.privacy != PrivacyClass::Public || c.redact)
    }) || program.records.iter().any(|r| {
        r.fields
            .iter()
            .any(|f| f.privacy != PrivacyClass::Public || f.redact)
    }) || program
        .functions
        .iter()
        .any(|f| f.params.iter().any(|p| p.privacy != PrivacyClass::Public))
}

struct PrivacySchemaIndex<'a> {
    tables: HashMap<String, &'a TableDef>,
    records: HashMap<String, &'a RecordDef>,
    known_tables: Vec<String>,
    has_classified_data: bool,
}

impl<'a> PrivacySchemaIndex<'a> {
    fn new(program: &'a Program) -> Self {
        let mut tables = HashMap::new();
        let mut known_tables = Vec::new();
        for t in &program.tables {
            tables.insert(t.name.to_ascii_lowercase(), t);
            known_tables.push(t.name.clone());
        }
        let mut records = HashMap::new();
        for r in &program.records {
            records.insert(r.name.clone(), r);
        }
        Self {
            tables,
            records,
            known_tables,
            has_classified_data: program_uses_privacy(program),
        }
    }

    fn type_taint(&self, ty: &Type) -> TaintVal {
        let Some(name) = unwrap_inner_type_name(ty) else {
            return TaintVal::clean();
        };
        if let Some(record) = self.records.get(name) {
            let mut fields = HashMap::new();
            let mut summary = PrivacyClass::Public;
            for f in &record.fields {
                summary = max_privacy(summary, f.privacy);
                fields.insert(f.name.clone(), f.privacy);
            }
            return TaintVal {
                summary,
                fields,
                param_deps: HashSet::new(),
            };
        }
        if let Some(table) = self.tables.get(&name.to_ascii_lowercase()) {
            let mut fields = HashMap::new();
            let mut summary = PrivacyClass::Public;
            for c in &table.columns {
                summary = max_privacy(summary, c.privacy);
                fields.insert(c.name.clone(), c.privacy);
            }
            return TaintVal {
                summary,
                fields,
                param_deps: HashSet::new(),
            };
        }
        TaintVal::clean()
    }

    fn sql_query_taint(&self, query: &str) -> TaintVal {
        let (op, accesses) = zelyra_database::sql::analyze_table_access(query, &self.known_tables);
        if !matches!(op, Some("SELECT")) {
            return TaintVal::clean();
        }
        let mut fields = HashMap::new();
        let mut all_table_summary = PrivacyClass::Public;
        for access in &accesses {
            if let Some(table) = self.tables.get(&access.table.to_ascii_lowercase()) {
                for col in &table.columns {
                    all_table_summary = max_privacy(all_table_summary, col.privacy);
                    fields
                        .entry(col.name.clone())
                        .and_modify(|cur| *cur = max_privacy(*cur, col.privacy))
                        .or_insert(col.privacy);
                }
            }
        }
        if all_table_summary == PrivacyClass::Public {
            return TaintVal {
                summary: PrivacyClass::Public,
                fields,
                param_deps: HashSet::new(),
            };
        }
        let projected = extract_select_projection(query);
        if projected.select_all || projected.columns.is_empty() {
            return TaintVal {
                summary: all_table_summary,
                fields,
                param_deps: HashSet::new(),
            };
        }
        let mut summary = PrivacyClass::Public;
        for col in &projected.columns {
            let projected_class = fields
                .iter()
                .filter(|(name, _)| name.eq_ignore_ascii_case(col))
                .map(|(_, class)| *class)
                .reduce(max_privacy)
                .unwrap_or(all_table_summary);
            summary = max_privacy(summary, projected_class);
        }
        TaintVal {
            summary,
            fields,
            param_deps: HashSet::new(),
        }
    }
}

struct SelectProjection {
    select_all: bool,
    columns: Vec<String>,
}

fn extract_select_projection(query: &str) -> SelectProjection {
    let upper = query.to_ascii_uppercase();
    // Skip any WITH ... prefix by finding the last outer top-level SELECT .. FROM
    let bytes = upper.as_bytes();
    let mut depth = 0isize;
    let mut in_single = false;
    let mut select_end = None;
    let mut from_start = None;
    let mut i = 0usize;
    while i < bytes.len() {
        let ch = bytes[i];
        if ch == b'\'' {
            in_single = !in_single;
            i += 1;
            continue;
        }
        if in_single {
            i += 1;
            continue;
        }
        if ch == b'(' {
            depth += 1;
        } else if ch == b')' {
            depth -= 1;
        } else if depth == 0 {
            if is_kw_at(bytes, i, b"SELECT") {
                select_end = Some(i + 6);
                from_start = None;
            } else if select_end.is_some() && from_start.is_none() && is_kw_at(bytes, i, b"FROM") {
                from_start = Some(i);
            }
        }
        i += 1;
    }
    let (Some(s_end), Some(f_start)) = (select_end, from_start) else {
        return SelectProjection {
            select_all: true,
            columns: Vec::new(),
        };
    };
    if s_end >= f_start {
        return SelectProjection {
            select_all: true,
            columns: Vec::new(),
        };
    }
    let list_slice = &query[s_end..f_start];
    let mut columns = Vec::new();
    let mut select_all = false;
    for part in split_top_level_commas(list_slice) {
        let trimmed = part.trim();
        if trimmed == "*" || trimmed.ends_with(".*") {
            select_all = true;
            continue;
        }
        // Handle `expr AS alias` -> inspect source expression before `AS` as well as tokens.
        let expr_part = trimmed
            .rsplit_once(" AS ")
            .or_else(|| trimmed.rsplit_once(" as "))
            .map_or(trimmed, |(lhs, _)| lhs)
            .trim();
        if let Some(ident) = trailing_column_ident(expr_part) {
            columns.push(ident);
        } else {
            // Compound expression (e.g. CONCAT(a, b)): collect all identifiers inside
            for token in expr_part.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
                if !token.is_empty()
                    && token
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphabetic())
                {
                    columns.push(token.to_owned());
                }
            }
        }
    }
    SelectProjection {
        select_all,
        columns,
    }
}

fn is_kw_at(bytes: &[u8], idx: usize, kw: &[u8]) -> bool {
    if idx + kw.len() > bytes.len() {
        return false;
    }
    let prev_ok = idx == 0 || !(bytes[idx - 1].is_ascii_alphanumeric() || bytes[idx - 1] == b'_');
    let next_ok = idx + kw.len() == bytes.len()
        || !(bytes[idx + kw.len()].is_ascii_alphanumeric() || bytes[idx + kw.len()] == b'_');
    prev_ok && next_ok && &bytes[idx..idx + kw.len()] == kw
}

fn split_top_level_commas(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0isize;
    let mut in_single = false;
    let mut start = 0usize;
    for (idx, ch) in input.char_indices() {
        if ch == '\'' {
            in_single = !in_single;
            continue;
        }
        if in_single {
            continue;
        }
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&input[start..idx]);
                start = idx + 1;
            }
            _ => {}
        }
    }
    parts.push(&input[start..]);
    parts
}

fn trailing_column_ident(expr: &str) -> Option<String> {
    let mut parts = expr.trim().split('.').map(str::trim);
    let candidate = parts.next_back()?;
    let is_ident = |name: &str| {
        !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    };
    (is_ident(candidate) && parts.all(is_ident)).then(|| candidate.to_owned())
}

fn summarize_functions(
    program: &Program,
    schema: &PrivacySchemaIndex<'_>,
) -> (HashMap<String, FunctionSummary>, Vec<PrivacyError>) {
    let mut fn_summaries: HashMap<String, FunctionSummary> = HashMap::new();
    for func in &program.functions {
        let init = FunctionSummary {
            return_intrinsic: func
                .return_type
                .as_ref()
                .map_or(PrivacyClass::Public, |t| schema.type_taint(t).summary),
            return_param_deps: HashSet::new(),
            param_sinks: vec![SinkFlags::default(); func.params.len()],
        };
        fn_summaries.insert(func.name.clone(), init);
    }

    // Summaries only gain facts. The lattice is finite, and the final pass
    // checks every function against the same set of converged summaries.
    let mut errors = Vec::new();
    loop {
        let mut changed = false;
        errors.clear();
        for func in &program.functions {
            let summary = analyze_function_privacy(func, schema, &fn_summaries, &mut errors);
            let previous = fn_summaries.get_mut(&func.name).expect("indexed function");
            let summary = summary.join(previous);
            if *previous != summary {
                *previous = summary;
                changed = true;
            }
        }
        if !changed {
            return (fn_summaries, errors);
        }
    }
}

/// Check the modeled information-flow rules in functions, APIs, and pages.
pub fn check_privacy(program: &Program) -> Result<(), Vec<PrivacyError>> {
    let schema = PrivacySchemaIndex::new(program);
    let (fn_summaries, mut errors) = summarize_functions(program, &schema);

    // Check APIs (handler return taint & output DTO schema).
    for api in &program.apis {
        let handler_summary = api.handler.as_ref().and_then(|name| fn_summaries.get(name));
        let handler_taint = handler_summary.map_or(PrivacyClass::Public, |s| s.return_intrinsic);
        let output_taint = schema.type_taint(&api.output).summary;
        let effective = max_privacy(handler_taint, output_taint);
        if effective == PrivacyClass::Secret {
            errors.push(PrivacyError {
                code: "E-PRIVACY-001",
                message: format!(
                    "API route `{} {}` must not expose `@secret` data",
                    api.method.to_ascii_uppercase(),
                    api.path
                ),
                span: api.span,
            });
        } else if effective == PrivacyClass::Pii && !api.requires_auth {
            errors.push(PrivacyError {
                code: "E-PRIVACY-001",
                message: format!(
                    "public API route `{} {}` exposes `@pii` data without `requires auth`; authenticate the route or use `redact(...)`",
                    api.method.to_ascii_uppercase(),
                    api.path
                ),
                span: api.span,
            });
        }
    }

    // Check pages (`data` queries and result types).
    for page in &program.pages {
        let protected = page.requires_auth || !page.permissions.is_empty();
        for data in &page.data {
            let query_taint = schema.sql_query_taint(&data.query).summary;
            let type_taint = schema.type_taint(&data.result_type).summary;
            let effective = max_privacy(query_taint, type_taint);
            if effective == PrivacyClass::Secret {
                errors.push(PrivacyError {
                    code: "E-PRIVACY-001",
                    message: format!(
                        "page `{}` data query `{}` accesses `@secret` data",
                        page.path, data.name
                    ),
                    span: data.span,
                });
            } else if effective == PrivacyClass::Pii && !protected {
                errors.push(PrivacyError {
                    code: "E-PRIVACY-001",
                    message: format!(
                        "public page `{}` exposes `@pii` data in `{}` without authentication or permissions",
                        page.path, data.name
                    ),
                    span: data.span,
                });
            }
        }
    }
    for view in &program.tableviews {
        let level = max_privacy(
            schema.sql_query_taint(&view.source).summary,
            schema.type_taint(&view.result_type).summary,
        );
        let protected = view.requires_auth || !view.permissions.is_empty();
        if level == PrivacyClass::Secret || (level == PrivacyClass::Pii && !protected) {
            errors.push(PrivacyError {
                code: "E-PRIVACY-001",
                message: format!(
                    "tableview `{}` exposes classified data outside its permitted route scope",
                    view.name
                ),
                span: view.span,
            });
        }
    }

    // Deduplicate identical errors produced cleanly at same span/message.
    let mut seen = HashSet::new();
    errors.retain(|err| seen.insert((err.span.start, err.span.end, err.message.clone())));

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn analyze_function_privacy(
    function: &Function,
    schema: &PrivacySchemaIndex<'_>,
    summaries: &HashMap<String, FunctionSummary>,
    errors: &mut Vec<PrivacyError>,
) -> FunctionSummary {
    let mut env: HashMap<String, TaintVal> = HashMap::new();
    for (idx, param) in function.params.iter().enumerate() {
        let mut taint = schema.type_taint(&param.ty);
        taint.summary = max_privacy(taint.summary, param.privacy);
        taint.param_deps.insert(idx);
        env.insert(param.name.clone(), taint);
    }

    let mut result = FunctionSummary {
        return_intrinsic: function
            .return_type
            .as_ref()
            .map_or(PrivacyClass::Public, |t| schema.type_taint(t).summary),
        return_param_deps: HashSet::new(),
        param_sinks: vec![SinkFlags::default(); function.params.len()],
    };

    analyze_block(
        &function.body,
        schema,
        summaries,
        &mut env,
        &mut result,
        errors,
    );
    result
}

fn analyze_block(
    block: &Block,
    schema: &PrivacySchemaIndex<'_>,
    summaries: &HashMap<String, FunctionSummary>,
    env: &mut HashMap<String, TaintVal>,
    out_summary: &mut FunctionSummary,
    errors: &mut Vec<PrivacyError>,
) {
    for stmt in &block.statements {
        match stmt {
            Stmt::Let {
                name, ty, value, ..
            } => {
                let mut val = analyze_expr(value, schema, summaries, env, out_summary, errors);
                if let Some(ty) = ty {
                    let type_taint = schema.type_taint(ty);
                    val = TaintVal::join(&val, &type_taint);
                }
                env.insert(name.clone(), val);
            }
            Stmt::BindOrAssign { name, value, .. } => {
                let val = analyze_expr(value, schema, summaries, env, out_summary, errors);
                let merged = env
                    .get(name)
                    .map_or(val.clone(), |prev| TaintVal::join(prev, &val));
                env.insert(name.clone(), merged);
            }
            Stmt::TupleBindOrAssign { names, value, .. } => {
                let val = analyze_expr(value, schema, summaries, env, out_summary, errors);
                for name in names {
                    env.insert(name.clone(), val.clone());
                }
            }
            Stmt::ArrayIndexAssign {
                name, index, value, ..
            } => {
                let idx = analyze_expr(index, schema, summaries, env, out_summary, errors);
                let val = analyze_expr(value, schema, summaries, env, out_summary, errors);
                let combined = TaintVal::join(&idx, &val);
                let merged = env
                    .get(name)
                    .map_or(combined.clone(), |prev| TaintVal::join(prev, &combined));
                env.insert(name.clone(), merged);
            }
            Stmt::Expr(expr) => {
                analyze_expr(expr, schema, summaries, env, out_summary, errors);
            }
            Stmt::Return {
                value: Some(expr), ..
            } => {
                let val = analyze_expr(expr, schema, summaries, env, out_summary, errors);
                out_summary.return_intrinsic =
                    max_privacy(out_summary.return_intrinsic, val.summary);
                out_summary.return_param_deps.extend(val.param_deps);
            }
            Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                analyze_expr(condition, schema, summaries, env, out_summary, errors);
                let mut then_env = env.clone();
                analyze_block(
                    then_block,
                    schema,
                    summaries,
                    &mut then_env,
                    out_summary,
                    errors,
                );
                let mut else_env = env.clone();
                if let Some(else_block) = else_block {
                    analyze_block(
                        else_block,
                        schema,
                        summaries,
                        &mut else_env,
                        out_summary,
                        errors,
                    );
                }
                for (k, v) in then_env.into_iter().chain(else_env) {
                    env.entry(k)
                        .and_modify(|cur| *cur = TaintVal::join(cur, &v))
                        .or_insert(v);
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                analyze_loop(
                    Some(condition),
                    body,
                    schema,
                    summaries,
                    env,
                    out_summary,
                    errors,
                );
            }
            Stmt::For {
                name,
                additional_names,
                iterable,
                body,
                ..
            } => {
                let iter_taint =
                    analyze_expr(iterable, schema, summaries, env, out_summary, errors);
                env.insert(name.clone(), iter_taint.clone());
                for extra in additional_names {
                    env.insert(extra.clone(), iter_taint.clone());
                }
                analyze_loop(None, body, schema, summaries, env, out_summary, errors);
            }
            Stmt::Loop { body, .. } => {
                analyze_loop(None, body, schema, summaries, env, out_summary, errors);
            }
            Stmt::Match { value, arms, .. } => {
                let subj = analyze_expr(value, schema, summaries, env, out_summary, errors);
                for arm in arms {
                    let mut arm_env = env.clone();
                    bind_pattern_names(&arm.pattern, &subj, &mut arm_env);
                    analyze_block(
                        &arm.body,
                        schema,
                        summaries,
                        &mut arm_env,
                        out_summary,
                        errors,
                    );
                    for (k, v) in arm_env {
                        env.entry(k)
                            .and_modify(|cur| *cur = TaintVal::join(cur, &v))
                            .or_insert(v);
                    }
                }
            }
            Stmt::Transaction { body, .. } | Stmt::Parallel { body, .. } => {
                analyze_block(body, schema, summaries, env, out_summary, errors);
            }
        }
    }
}

fn analyze_loop(
    condition: Option<&Expr>,
    body: &Block,
    schema: &PrivacySchemaIndex<'_>,
    summaries: &HashMap<String, FunctionSummary>,
    env: &mut HashMap<String, TaintVal>,
    out_summary: &mut FunctionSummary,
    errors: &mut Vec<PrivacyError>,
) {
    loop {
        if let Some(condition) = condition {
            analyze_expr(condition, schema, summaries, env, out_summary, errors);
        }
        let mut next = env.clone();
        analyze_block(body, schema, summaries, &mut next, out_summary, errors);
        let mut changed = false;
        for (name, value) in next {
            let merged = match env.get(&name) {
                Some(previous) => TaintVal::join(previous, &value),
                None => value,
            };
            if env.get(&name) != Some(&merged) {
                env.insert(name, merged);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

fn bind_pattern_names(pattern: &Pattern, val: &TaintVal, env: &mut HashMap<String, TaintVal>) {
    match &pattern.kind {
        PatternKind::Variable(name) => {
            env.insert(name.clone(), val.clone());
        }
        PatternKind::Constructor {
            inner: Some(inner), ..
        } => bind_pattern_names(inner, val, env),
        PatternKind::EnumVariantPayload { payloads, .. } => {
            for item in payloads {
                bind_pattern_names(item, val, env);
            }
        }
        PatternKind::Record { fields, .. } => {
            for (fname, fpat) in fields {
                let field_val = TaintVal {
                    summary: val.fields.get(fname).copied().unwrap_or(val.summary),
                    fields: HashMap::new(),
                    param_deps: val.param_deps.clone(),
                };
                bind_pattern_names(fpat, &field_val, env);
            }
        }
        _ => {}
    }
}

fn check_sink(
    taint: &TaintVal,
    sink_kind: &str,
    reject_pii: bool,
    reject_secret: bool,
    span: Span,
    out_summary: &mut FunctionSummary,
    errors: &mut Vec<PrivacyError>,
) {
    for &idx in &taint.param_deps {
        if let Some(flags) = out_summary.param_sinks.get_mut(idx) {
            match sink_kind {
                "print" => flags.log_sink = true,
                "Err" => flags.error_sink = true,
                "http_get"
                | "http_request"
                | "http_json"
                | "http_result"
                | "mail_queue"
                | "mail_queue_template"
                | "mail_deliver"
                | "external delivery" => flags.external_sink = true,
                "redact" => flags.redact_sink = true,
                _ => {}
            }
        }
    }
    if reject_secret && taint.summary == PrivacyClass::Secret {
        errors.push(PrivacyError {
            code: "E-PRIVACY-001",
            message: format!("cannot pass `@secret` value into `{sink_kind}`"),
            span,
        });
    } else if reject_pii && taint.summary == PrivacyClass::Pii {
        errors.push(PrivacyError {
            code: "E-PRIVACY-001",
            message: format!(
                "cannot pass `@pii` value into `{sink_kind}` without explicit `redact(...)`"
            ),
            span,
        });
    }
}

fn analyze_expr(
    expr: &Expr,
    schema: &PrivacySchemaIndex<'_>,
    summaries: &HashMap<String, FunctionSummary>,
    env: &HashMap<String, TaintVal>,
    out_summary: &mut FunctionSummary,
    errors: &mut Vec<PrivacyError>,
) -> TaintVal {
    match &expr.kind {
        ExprKind::Int(_)
        | ExprKind::UInt(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::String(_)
        | ExprKind::Char(_)
        | ExprKind::FunctionValue(_) => TaintVal::clean(),
        ExprKind::Variable(name) => env.get(name).cloned().unwrap_or_else(TaintVal::clean),
        ExprKind::Unary { expr: inner, .. } | ExprKind::Await(inner) | ExprKind::Try(inner) => {
            let mut cur = inner.as_ref();
            while let ExprKind::Unary { expr: next, .. }
            | ExprKind::Await(next)
            | ExprKind::Try(next) = &cur.kind
            {
                cur = next.as_ref();
            }
            analyze_expr(cur, schema, summaries, env, out_summary, errors)
        }
        ExprKind::Binary { left, right, .. } => {
            let mut stack: Vec<&Expr> = vec![right.as_ref(), left.as_ref()];
            let mut acc = TaintVal::clean();
            while let Some(node) = stack.pop() {
                let mut cur = node;
                loop {
                    match &cur.kind {
                        ExprKind::Binary { left, right, .. } => {
                            stack.push(right.as_ref());
                            cur = left.as_ref();
                        }
                        ExprKind::Unary { expr: inner, .. }
                        | ExprKind::Await(inner)
                        | ExprKind::Try(inner) => {
                            cur = inner.as_ref();
                        }
                        _ => {
                            let val =
                                analyze_expr(cur, schema, summaries, env, out_summary, errors);
                            acc = TaintVal::join(&acc, &val);
                            break;
                        }
                    }
                }
            }
            acc
        }
        ExprKind::Array(items) => {
            let mut acc = TaintVal::clean();
            for item in items {
                let t = analyze_expr(item, schema, summaries, env, out_summary, errors);
                acc = TaintVal::join(&acc, &t);
            }
            acc
        }
        ExprKind::Map(entries) => {
            let mut acc = TaintVal::clean();
            for (k, v) in entries {
                let kt = analyze_expr(k, schema, summaries, env, out_summary, errors);
                let vt = analyze_expr(v, schema, summaries, env, out_summary, errors);
                acc = TaintVal::join(&acc, &TaintVal::join(&kt, &vt));
            }
            acc
        }
        ExprKind::Index { target, index } => {
            let t = analyze_expr(target, schema, summaries, env, out_summary, errors);
            let i = analyze_expr(index, schema, summaries, env, out_summary, errors);
            TaintVal::join(&t, &i)
        }
        ExprKind::Field { target, field } => {
            let t = analyze_expr(target, schema, summaries, env, out_summary, errors);
            let field_class = if !t.fields.is_empty() {
                t.fields.get(field).copied().unwrap_or(PrivacyClass::Public)
            } else {
                t.summary
            };
            TaintVal {
                summary: field_class,
                fields: HashMap::new(),
                param_deps: t.param_deps,
            }
        }
        ExprKind::Record {
            type_name, fields, ..
        } => {
            let mut schema_taint =
                schema
                    .records
                    .get(type_name)
                    .map_or_else(TaintVal::clean, |r| {
                        let mut map = HashMap::new();
                        let mut sum = PrivacyClass::Public;
                        for f in &r.fields {
                            sum = max_privacy(sum, f.privacy);
                            map.insert(f.name.clone(), f.privacy);
                        }
                        TaintVal {
                            summary: sum,
                            fields: map,
                            param_deps: HashSet::new(),
                        }
                    });
            for (fname, fexpr) in fields {
                let ft = analyze_expr(fexpr, schema, summaries, env, out_summary, errors);
                schema_taint.summary = max_privacy(schema_taint.summary, ft.summary);
                schema_taint.param_deps.extend(ft.param_deps);
                schema_taint
                    .fields
                    .entry(fname.clone())
                    .and_modify(|cur| *cur = max_privacy(*cur, ft.summary))
                    .or_insert(ft.summary);
            }
            schema_taint
        }
        ExprKind::Sql { query, .. } => {
            let mut sql_taint = schema.sql_query_taint(query);
            if let Ok(param_names) = zelyra_database::sql_parameter_names(query) {
                for p in param_names {
                    if let Some(pt) = env.get(&p) {
                        sql_taint.param_deps.extend(pt.param_deps.iter().copied());
                    }
                }
            }
            sql_taint
        }
        ExprKind::Match { value, arms } => {
            let subj = analyze_expr(value, schema, summaries, env, out_summary, errors);
            let mut acc = TaintVal::clean();
            for arm in arms {
                let mut arm_env = env.clone();
                bind_pattern_names(&arm.pattern, &subj, &mut arm_env);
                let val =
                    analyze_expr(&arm.value, schema, summaries, &arm_env, out_summary, errors);
                acc = TaintVal::join(&acc, &val);
            }
            acc
        }
        ExprKind::Lambda { body, .. } => {
            analyze_expr(body, schema, summaries, env, out_summary, errors)
        }
        ExprKind::Call { name, args, .. } => {
            let arg_taints = args
                .iter()
                .map(|arg| analyze_expr(arg, schema, summaries, env, out_summary, errors))
                .collect::<Vec<_>>();

            if schema.has_classified_data
                && (env.contains_key(name)
                    || matches!(name.as_str(), "map" | "filter" | "find" | "reduce"))
            {
                errors.push(PrivacyError {
                    code: "E-PRIVACY-001",
                    message: format!(
                        "privacy checks cannot verify indirect call `{name}`; use a named helper"
                    ),
                    span: expr.span,
                });
            }

            match name.as_str() {
                "print" => {
                    if let Some(t) = arg_taints.first() {
                        check_sink(t, "print", true, true, args[0].span, out_summary, errors);
                    }
                    return TaintVal::clean();
                }
                "redact" if !summaries.contains_key(name) && !env.contains_key(name) => {
                    if let Some(t) = arg_taints.first() {
                        check_sink(t, "redact", false, true, args[0].span, out_summary, errors);
                    }
                    return TaintVal::clean();
                }
                "Err" => {
                    if let Some(t) = arg_taints.first() {
                        check_sink(t, "Err", true, true, args[0].span, out_summary, errors);
                    }
                    return TaintVal::of(
                        arg_taints
                            .first()
                            .map_or(PrivacyClass::Public, |t| t.summary),
                    );
                }
                "http_get"
                | "http_request"
                | "http_json"
                | "http_result"
                | "mail_queue"
                | "mail_queue_template"
                | "mail_deliver" => {
                    for (arg, taint) in args.iter().zip(&arg_taints) {
                        check_sink(taint, name, true, true, arg.span, out_summary, errors);
                    }
                    return TaintVal::clean();
                }
                "record_with" if args.len() == 3 => {
                    let mut base = arg_taints[0].clone();
                    let field_t = &arg_taints[2];
                    base.summary = max_privacy(base.summary, field_t.summary);
                    base.param_deps.extend(field_t.param_deps.iter().copied());
                    if let ExprKind::String(fname) = &args[1].kind {
                        base.fields.insert(fname.clone(), field_t.summary);
                    }
                    return base;
                }
                "record_update" => {
                    let mut acc = arg_taints.first().cloned().unwrap_or_else(TaintVal::clean);
                    for t in arg_taints.iter().skip(1) {
                        acc = TaintVal::join(&acc, t);
                    }
                    return acc;
                }
                _ => {}
            }

            if let Some(callee) = summaries.get(name).filter(|_| !env.contains_key(name)) {
                for (idx, arg_taint) in arg_taints.iter().enumerate() {
                    if let Some(flags) = callee.param_sinks.get(idx) {
                        let span = args.get(idx).map_or(expr.span, |a| a.span);
                        if flags.log_sink {
                            check_sink(arg_taint, "print", true, true, span, out_summary, errors);
                        }
                        if flags.error_sink {
                            check_sink(arg_taint, "Err", true, true, span, out_summary, errors);
                        }
                        if flags.redact_sink {
                            check_sink(arg_taint, "redact", false, true, span, out_summary, errors);
                        }
                        if flags.external_sink {
                            check_sink(
                                arg_taint,
                                "external delivery",
                                true,
                                true,
                                span,
                                out_summary,
                                errors,
                            );
                        }
                    }
                }
                let mut ret = TaintVal::of(callee.return_intrinsic);
                for &param_idx in &callee.return_param_deps {
                    if let Some(arg_taint) = arg_taints.get(param_idx) {
                        ret = TaintVal::join(&ret, arg_taint);
                    }
                }
                return ret;
            }

            // Generic pure/builtin propagation (json_encode, to_upper, string concat helpers, Some, Ok, etc.)
            let mut acc = TaintVal::clean();
            for t in &arg_taints {
                acc = TaintVal::join(&acc, t);
            }
            acc
        }
    }
}

/// Deterministic GDPR inventory derived from the AST.
pub fn inspect_privacy_inventory(program: &Program) -> PrivacyInventory {
    let schema = PrivacySchemaIndex::new(program);
    let storage = zelyra_database::build_schema(program).ok();
    let mut columns = Vec::new();
    for table in &program.tables {
        let mut foreign_keys = storage
            .as_ref()
            .and_then(|schema| schema.tables.iter().find(|item| item.name == table.name))
            .into_iter()
            .flat_map(|item| &item.foreign_keys)
            .map(|key| format!("{}.{}:{}", table.name, key.column, key.referenced_table))
            .collect::<Vec<_>>();
        foreign_keys.sort_unstable();
        foreign_keys.dedup();
        let has_delete_guard = program
            .cruds
            .iter()
            .filter(|c| c.table == table.name)
            .any(|c| {
                c.guards
                    .iter()
                    .any(|guard| guard.action == CrudRecordAction::Delete)
            });
        for col in &table.columns {
            if col.privacy == PrivacyClass::Public && !col.redact {
                continue;
            }
            columns.push(PrivacyColumnEntry {
                table: table.name.clone(),
                column: col.name.clone(),
                ty: col.ty.to_string(),
                privacy: col.privacy,
                redact: col.redact,
                tenant_scoped: table.tenant_scope.is_some(),
                is_ledger: table.is_ledger,
                is_auditable: table.is_auditable,
                has_delete_guard,
                foreign_keys: foreign_keys.clone(),
            });
        }
    }
    columns.sort_by(|a, b| a.table.cmp(&b.table).then(a.column.cmp(&b.column)));

    let mut record_fields = Vec::new();
    for rec in &program.records {
        for field in &rec.fields {
            if field.privacy == PrivacyClass::Public && !field.redact {
                continue;
            }
            record_fields.push(PrivacyRecordFieldEntry {
                record: rec.name.clone(),
                field: field.name.clone(),
                ty: field.ty.to_string(),
                privacy: field.privacy,
                redact: field.redact,
            });
        }
    }
    record_fields.sort_by(|a, b| a.record.cmp(&b.record).then(a.field.cmp(&b.field)));

    let mut routes = Vec::new();
    for api in &program.apis {
        let mut level = schema.type_taint(&api.output).summary;
        let mut tables = Vec::new();
        if let Some(handler_name) = &api.handler {
            if let Some(func) = program.functions.iter().find(|f| &f.name == handler_name) {
                let mut fn_tables = HashSet::new();
                collect_block_tables(&func.body, &schema.known_tables, &mut fn_tables);
                for tname in fn_tables {
                    if let Some(tbl) = schema.tables.get(&tname.to_ascii_lowercase()) {
                        for col in &tbl.columns {
                            level = max_privacy(level, col.privacy);
                        }
                    }
                    tables.push(tname);
                }
            }
        }
        tables.sort();
        tables.dedup();
        routes.push(PrivacyRouteEntry {
            kind: "api",
            target: format!("{} {}", api.method.to_ascii_uppercase(), api.path),
            requires_auth: api.requires_auth || !api.permissions.is_empty(),
            permissions: api.permissions.clone(),
            privacy_level: level,
            tables,
        });
    }
    for page in &program.pages {
        let mut level = PrivacyClass::Public;
        let mut tables = Vec::new();
        for data in &page.data {
            level = max_privacy(level, schema.sql_query_taint(&data.query).summary);
            level = max_privacy(level, schema.type_taint(&data.result_type).summary);
            let (_, accesses) =
                zelyra_database::sql::analyze_table_access(&data.query, &schema.known_tables);
            for access in accesses {
                tables.push(access.table);
            }
        }
        tables.sort();
        tables.dedup();
        routes.push(PrivacyRouteEntry {
            kind: "page",
            target: page.path.clone(),
            requires_auth: page.requires_auth || !page.permissions.is_empty(),
            permissions: page.permissions.clone(),
            privacy_level: level,
            tables,
        });
    }
    for crud in &program.cruds {
        let level =
            schema
                .tables
                .get(&crud.table.to_ascii_lowercase())
                .map_or(PrivacyClass::Public, |t| {
                    t.columns
                        .iter()
                        .fold(PrivacyClass::Public, |acc, c| max_privacy(acc, c.privacy))
                });
        routes.push(PrivacyRouteEntry {
            kind: "crud",
            target: format!("/crud/{}", crud.name),
            requires_auth: crud.requires_auth
                || crud.tenant_column.is_some()
                || crud.recipient_column.is_some()
                || !crud.permissions.is_empty(),
            permissions: crud.permissions.clone(),
            privacy_level: level,
            tables: vec![crud.table.clone()],
        });
    }
    routes.sort_by(|a, b| a.kind.cmp(b.kind).then(a.target.cmp(&b.target)));

    let mut external_sinks = Vec::new();
    for func in &program.functions {
        for cap in &func.capabilities {
            if cap.starts_with("Network")
                || cap.starts_with("Email")
                || cap.starts_with("FileSystem")
            {
                external_sinks.push(PrivacyExternalSinkEntry {
                    function: func.name.clone(),
                    capability: cap.clone(),
                });
            }
        }
    }
    external_sinks.sort_by(|a, b| {
        a.function
            .cmp(&b.function)
            .then(a.capability.cmp(&b.capability))
    });

    PrivacyInventory {
        columns,
        record_fields,
        routes,
        external_sinks,
    }
}

fn collect_block_tables(block: &Block, known: &[String], out: &mut HashSet<String>) {
    for stmt in &block.statements {
        match stmt {
            Stmt::Let { value, .. }
            | Stmt::BindOrAssign { value, .. }
            | Stmt::TupleBindOrAssign { value, .. }
            | Stmt::Expr(value)
            | Stmt::Return {
                value: Some(value), ..
            } => collect_expr_tables(value, known, out),
            Stmt::ArrayIndexAssign { index, value, .. } => {
                collect_expr_tables(index, known, out);
                collect_expr_tables(value, known, out);
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                collect_expr_tables(condition, known, out);
                collect_block_tables(then_block, known, out);
                if let Some(e) = else_block {
                    collect_block_tables(e, known, out);
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                collect_expr_tables(condition, known, out);
                collect_block_tables(body, known, out);
            }
            Stmt::For { iterable, body, .. } => {
                collect_expr_tables(iterable, known, out);
                collect_block_tables(body, known, out);
            }
            Stmt::Loop { body, .. }
            | Stmt::Transaction { body, .. }
            | Stmt::Parallel { body, .. } => {
                collect_block_tables(body, known, out);
            }
            Stmt::Match { value, arms, .. } => {
                collect_expr_tables(value, known, out);
                for arm in arms {
                    collect_block_tables(&arm.body, known, out);
                }
            }
            Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }
}

fn collect_expr_tables(expr: &Expr, known: &[String], out: &mut HashSet<String>) {
    match &expr.kind {
        ExprKind::Sql { query, .. } => {
            let (_, accesses) = zelyra_database::sql::analyze_table_access(query, known);
            for a in accesses {
                if !a.table.is_empty() {
                    out.insert(a.table);
                }
            }
        }
        ExprKind::Unary { expr, .. }
        | ExprKind::Await(expr)
        | ExprKind::Try(expr)
        | ExprKind::Field { target: expr, .. }
        | ExprKind::Lambda { body: expr, .. } => collect_expr_tables(expr, known, out),
        ExprKind::Binary { left, right, .. }
        | ExprKind::Index {
            target: left,
            index: right,
        } => {
            collect_expr_tables(left, known, out);
            collect_expr_tables(right, known, out);
        }
        ExprKind::Array(items) => {
            for item in items {
                collect_expr_tables(item, known, out);
            }
        }
        ExprKind::Map(entries) => {
            for (k, v) in entries {
                collect_expr_tables(k, known, out);
                collect_expr_tables(v, known, out);
            }
        }
        ExprKind::Record { fields, .. } => {
            for (_, v) in fields {
                collect_expr_tables(v, known, out);
            }
        }
        ExprKind::Call { args, .. } => {
            for a in args {
                collect_expr_tables(a, known, out);
            }
        }
        ExprKind::Match { value, arms } => {
            collect_expr_tables(value, known, out);
            for arm in arms {
                collect_expr_tables(&arm.value, known, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;

    #[test]
    fn function_summaries_converge_across_call_chains_and_loops() {
        let mut source = String::new();
        for index in 0..12 {
            writeln!(
                source,
                "fn step_{index}(value: String) -> String {{ return step_{}(value) }}",
                index + 1
            )
            .unwrap();
        }
        source.push_str(
            r#"
            fn step_12(value: String) -> String { return value }

            fn propagate(repeat: Bool, value: String) -> String {
                first = ""
                second = ""
                third = ""
                fourth = ""
                while repeat {
                    first = second
                    second = third
                    third = fourth
                    fourth = value
                }
                return first
            }
            "#,
        );
        let program = zelyra_parser::parse(&zelyra_lexer::lex(&source).unwrap()).unwrap();
        let schema = PrivacySchemaIndex::new(&program);
        let (summaries, errors) = summarize_functions(&program, &schema);
        assert!(errors.is_empty());
        assert_eq!(summaries["step_0"].return_param_deps, HashSet::from([0]));
        assert_eq!(summaries["propagate"].return_param_deps, HashSet::from([1]));
    }
}
