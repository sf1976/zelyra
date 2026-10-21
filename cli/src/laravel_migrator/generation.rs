use super::{graph::matching_brace, PhpSourceMap, SourceSpan};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::Range;
use std::path::Path;
use zelyra_ast::{ReferentialAction, TextStorage};

type ParsedSchemaColumns = (
    Vec<Column>,
    Vec<Vec<String>>,
    Vec<Vec<String>>,
    Vec<String>,
    Option<String>,
);
type ParsedSqlType = (
    String,
    Option<u32>,
    Option<(u32, u32)>,
    Option<TextStorage>,
    String,
);
type ColumnSignature = (String, String, Option<u32>, Option<(u32, u32)>, bool);

struct ParseColumnsOptions<'a> {
    body_offset: usize,
    source: &'a str,
    source_map: &'a PhpSourceMap,
    table_variable: &'a str,
    table_name: &'a str,
    require_primary_key: bool,
    allow_external_index_columns: bool,
}

fn sql_code_mask(source: &str) -> Vec<bool> {
    let bytes = source.as_bytes();
    let mut code = vec![false; bytes.len()];
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"--") || bytes[index] == b'#' {
            index = bytes[index..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |offset| index + offset + 1);
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            index = bytes[index + 2..]
                .windows(2)
                .position(|window| window == b"*/")
                .map_or(bytes.len(), |offset| index + 2 + offset + 2);
            continue;
        }
        if matches!(bytes[index], b'\'' | b'"' | b'`') {
            let quote = bytes[index];
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else if bytes[index] == quote {
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
        code[index] = true;
        index += 1;
    }
    code
}

fn matching_sql_delimiter(source: &str, start: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut index = start;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"--") || bytes[index] == b'#' {
            index = bytes[index..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |offset| index + offset + 1);
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            index = bytes[index + 2..]
                .windows(2)
                .position(|window| window == b"*/")
                .map_or(bytes.len(), |offset| index + 2 + offset + 2);
            continue;
        }
        if matches!(bytes[index], b'\'' | b'"' | b'`') {
            let quote = bytes[index];
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else if bytes[index] == quote {
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
        if bytes[index] == open {
            depth += 1;
        } else if bytes[index] == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
        index += 1;
    }
    None
}

fn sql_statement_end(source: &str, code: &[bool], start: usize) -> usize {
    source.as_bytes()[start..]
        .iter()
        .enumerate()
        .find_map(|(offset, byte)| {
            let index = start + offset;
            (*byte == b';' && code.get(index).copied().unwrap_or(false)).then_some(index)
        })
        .unwrap_or(source.len())
}

fn split_sql_items(source: &str) -> Result<Vec<&str>, String> {
    let bytes = source.as_bytes();
    let mut items = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"--") || bytes[index] == b'#' {
            index = bytes[index..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |offset| index + offset + 1);
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            index = bytes[index + 2..]
                .windows(2)
                .position(|window| window == b"*/")
                .map_or(bytes.len(), |offset| index + 2 + offset + 2);
            continue;
        }
        if matches!(bytes[index], b'\'' | b'"' | b'`') {
            let quote = bytes[index];
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else if bytes[index] == quote {
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
        match bytes[index] {
            b'(' => depth += 1,
            b')' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or("SQL has unmatched parentheses")?
            }
            b',' if depth == 0 => {
                items.push(source[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }
    if depth != 0 {
        return Err("SQL has unmatched parentheses".into());
    }
    let last = source[start..].trim();
    if !last.is_empty() {
        items.push(last);
    }
    if items.iter().any(|item| item.is_empty()) {
        return Err("SQL contains an empty column or index declaration".into());
    }
    Ok(items)
}

fn sql_tokens(source: &str) -> Result<Vec<&str>, String> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut start = None;
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            if let Some(token_start) = start.take() {
                tokens.push(&source[token_start..index]);
            }
            index += 1;
            continue;
        }
        if start.is_none() {
            start = Some(index);
        }
        if matches!(bytes[index], b'\'' | b'"' | b'`') {
            let quote = bytes[index];
            index += 1;
            let mut closed = false;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else if bytes[index] == quote {
                    if bytes.get(index + 1) == Some(&quote) {
                        index += 2;
                    } else {
                        index += 1;
                        closed = true;
                        break;
                    }
                } else {
                    index += 1;
                }
            }
            if !closed {
                return Err("SQL contains an unterminated quoted value".into());
            }
        } else {
            index += 1;
        }
    }
    if let Some(token_start) = start {
        tokens.push(&source[token_start..]);
    }
    Ok(tokens)
}

fn sql_unquote_identifier(value: &str) -> Result<String, String> {
    let value = value.trim();
    let value = value
        .strip_prefix('`')
        .and_then(|inner| inner.strip_suffix('`'))
        .unwrap_or(value);
    if !is_identifier(value) {
        return Err(format!("SQL identifier `{value}` is not supported"));
    }
    Ok(value.to_owned())
}

fn is_sensitive_schema_column(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    [
        "password",
        "secret",
        "token",
        "api_key",
        "private_key",
        "access_key",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn parse_sql_column(source: &str) -> Result<Column, String> {
    let tokens = sql_tokens(source)?;
    if tokens.len() < 2 {
        return Err("SQL column declaration is incomplete".into());
    }
    let name = sql_unquote_identifier(tokens[0])?;
    let raw_type = tokens[1].trim();
    let upper_type = raw_type.to_ascii_uppercase();
    let unsigned = tokens
        .get(2)
        .is_some_and(|token| token.eq_ignore_ascii_case("UNSIGNED"));
    let modifiers_start = 2 + usize::from(unsigned);
    let modifiers = tokens.get(modifiers_start..).unwrap_or_default();
    let mut nullable = true;
    let mut primary_key = false;
    let mut auto_increment = false;
    let mut unique = false;
    let mut default = None;
    let mut consumed = BTreeSet::new();
    let mut index = 0usize;
    while index < modifiers.len() {
        let token = modifiers[index];
        if token.eq_ignore_ascii_case("NOT")
            && modifiers
                .get(index + 1)
                .is_some_and(|next| next.eq_ignore_ascii_case("NULL"))
        {
            nullable = false;
            consumed.insert(index);
            consumed.insert(index + 1);
            index += 2;
        } else if token.eq_ignore_ascii_case("NULL") {
            nullable = true;
            consumed.insert(index);
            index += 1;
        } else if token.eq_ignore_ascii_case("AUTO_INCREMENT") {
            auto_increment = true;
            consumed.insert(index);
            index += 1;
        } else if token.eq_ignore_ascii_case("PRIMARY")
            && modifiers
                .get(index + 1)
                .is_some_and(|next| next.eq_ignore_ascii_case("KEY"))
        {
            primary_key = true;
            nullable = false;
            consumed.insert(index);
            consumed.insert(index + 1);
            index += 2;
        } else if token.eq_ignore_ascii_case("UNIQUE") {
            unique = true;
            consumed.insert(index);
            index += 1;
        } else if token.eq_ignore_ascii_case("DEFAULT") {
            let Some(value) = modifiers.get(index + 1).copied() else {
                return Err(format!("SQL column `{name}` has an empty DEFAULT clause"));
            };
            if default.is_some() {
                return Err(format!("SQL column `{name}` has duplicate DEFAULT clauses"));
            }
            let normalized = if value.eq_ignore_ascii_case("NULL") {
                None
            } else if value.to_ascii_uppercase().starts_with("CURRENT_TIMESTAMP") {
                Some("default now".to_owned())
            } else {
                Some(render_default(value, true)?)
            };
            default = normalized;
            consumed.insert(index);
            consumed.insert(index + 1);
            index += 2;
        } else {
            index += 1;
        }
    }
    let unknown = modifiers
        .iter()
        .enumerate()
        .filter(|(index, _)| !consumed.contains(index))
        .map(|(_, token)| *token)
        .collect::<Vec<_>>();
    if !unknown.is_empty() {
        return Err(format!(
            "SQL column `{name}` has unsupported modifiers: {}",
            unknown.join(" ")
        ));
    }

    let (ty, length, decimal_precision, text_storage, laravel_method) =
        parse_sql_type(&upper_type, unsigned)?;
    if default.is_some() && is_sensitive_schema_column(&name) {
        return Err(format!(
            "deployment SQL default for sensitive column `{name}` is not exported; review it manually"
        ));
    }
    if primary_key && nullable {
        return Err(format!("SQL primary key `{name}` cannot be nullable"));
    }
    if auto_increment && ty != "Id" && ty != "UInt" && ty != "Int" {
        return Err(format!(
            "SQL AUTO_INCREMENT column `{name}` is not an integer"
        ));
    }
    let mut final_type = ty;
    if final_type == "Bool" {
        default = default.map(|value| match value.as_str() {
            "default 1" => "default true".to_owned(),
            "default 0" => "default false".to_owned(),
            _ => value,
        });
    }
    if primary_key && auto_increment {
        final_type = "Id".into();
    }
    Ok(Column {
        name: name.clone(),
        source_name: name,
        laravel_method,
        ty: final_type,
        length,
        text_storage,
        storage_name: None,
        decimal_precision,
        enum_values: None,
        relation_table: None,
        references_table: None,
        unresolved_relation: None,
        nullable,
        primary_key,
        auto_increment,
        unique,
        indexed: false,
        default,
        on_delete: None,
        after: None,
    })
}

fn parse_sql_type(raw: &str, unsigned: bool) -> Result<ParsedSqlType, String> {
    let upper = raw.to_ascii_uppercase();
    let (base, args) = upper
        .split_once('(')
        .map(|(base, args)| (base, Some(args.trim_end_matches(')'))))
        .unwrap_or((upper.as_str(), None));
    if args.is_some_and(|args| args.contains(')')) {
        return Err(format!("SQL type `{raw}` has malformed parameters"));
    }
    let mut length = None;
    let mut precision = None;
    let mut text_storage = None;
    let ty = match base {
        "BIGINT" => {
            if unsigned {
                "UInt"
            } else {
                "Int"
            }
        }
        "INT" | "INTEGER" => {
            if unsigned {
                "UInt"
            } else {
                "Int"
            }
        }
        "SMALLINT" => {
            if unsigned {
                "UInt"
            } else {
                "Int"
            }
        }
        "TINYINT" => {
            if args == Some("1") && !unsigned {
                "Bool"
            } else if unsigned {
                "UInt"
            } else {
                "Int"
            }
        }
        "BOOL" | "BOOLEAN" if !unsigned => "Bool",
        "VARCHAR" | "CHAR" if !unsigned => {
            let value = args
                .and_then(|value| value.parse::<u32>().ok())
                .filter(|value| *value > 0 && *value <= 65_535)
                .ok_or_else(|| format!("SQL string type `{raw}` needs a supported length"))?;
            length = Some(value);
            "String"
        }
        "TEXT" if !unsigned => {
            text_storage = Some(TextStorage::Text);
            "String"
        }
        "MEDIUMTEXT" if !unsigned => {
            text_storage = Some(TextStorage::MediumText);
            "String"
        }
        "LONGTEXT" if !unsigned => {
            text_storage = Some(TextStorage::LongText);
            "String"
        }
        "JSON" if !unsigned && args.is_none() => "Json",
        "DATE" if !unsigned && args.is_none() => "Date",
        "DATETIME" | "TIMESTAMP" if !unsigned => "Timestamp",
        "TIME" if !unsigned && args.is_none() => "Time",
        "DECIMAL" | "NUMERIC" if !unsigned => {
            let pair = args
                .and_then(|value| value.split_once(','))
                .and_then(|(precision, scale)| {
                    Some((
                        precision.trim().parse::<u32>().ok()?,
                        scale.trim().parse::<u32>().ok()?,
                    ))
                })
                .filter(|(precision, scale)| {
                    (1..=39).contains(precision) && *scale <= 9 && scale < precision
                })
                .ok_or_else(|| {
                    format!("SQL decimal type `{raw}` has unsupported precision or scale")
                })?;
            precision = Some(pair);
            "Decimal"
        }
        _ => {
            return Err(format!(
                "SQL type `{raw}` is outside the supported deployment schema subset"
            ))
        }
    };
    let method = format!("raw SQL: {raw}{}", if unsigned { " UNSIGNED" } else { "" });
    Ok((ty.into(), length, precision, text_storage, method))
}

fn parse_sql_identifier_list(source: &str) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    for item in split_sql_items(source)? {
        let name = item.split_once('(').map_or(item, |(name, _)| name).trim();
        names.push(sql_unquote_identifier(name)?);
    }
    if names.is_empty() {
        return Err("SQL index declaration has no columns".into());
    }
    Ok(names)
}

fn parse_sql_create_body(source: &str, table: &str) -> Result<ParsedSchemaColumns, String> {
    let mut columns = Vec::<Column>::new();
    let mut indexes = Vec::new();
    let mut uniques = Vec::new();
    let mut notes = vec![format!(
        "Table `{table}` is imported from custom deployment SQL; verify its deployment ordering and backend-specific DDL."
    )];
    let item_regex = super::compiled_regex(
        r"(?i)^(?:(UNIQUE)\s+)?(?:KEY|INDEX)\s+`?([A-Za-z_]\w*)`?\s*\((.*)\)$",
        "deployment SQL key declaration regex",
    );
    let primary_regex = super::compiled_regex(
        r"(?i)^PRIMARY\s+KEY\s*\((.*)\)$",
        "deployment SQL primary key regex",
    );
    for item in split_sql_items(source)? {
        let normalized = item.trim();
        if let Some(capture) = primary_regex.captures(normalized) {
            let names =
                parse_sql_identifier_list(capture.get(1).expect("primary key columns").as_str())?;
            if names.len() != 1 {
                return Err(
                    "composite SQL primary keys are not supported by the generated table model"
                        .into(),
                );
            }
            let column = columns
                .iter_mut()
                .find(|column| column.source_name == names[0])
                .ok_or_else(|| {
                    format!("SQL primary key refers to undeclared column `{}`", names[0])
                })?;
            if column.primary_key {
                return Err(format!(
                    "SQL column `{}` declares the primary key more than once",
                    names[0]
                ));
            }
            column.primary_key = true;
            column.nullable = false;
            if column.auto_increment {
                column.ty = "Id".into();
            }
            continue;
        }
        if let Some(capture) = item_regex.captures(normalized) {
            let is_unique = capture.get(1).is_some();
            let key_name = capture.get(2).expect("SQL index name").as_str();
            let names =
                parse_sql_identifier_list(capture.get(3).expect("SQL index columns").as_str())?;
            notes.push(format!(
                "SQL index name `{key_name}` on `{table}` is not preserved by generated Zelyra schema."
            ));
            for name in &names {
                let column = columns
                    .iter_mut()
                    .find(|column| column.source_name == *name)
                    .ok_or_else(|| {
                        format!("SQL index `{key_name}` refers to undeclared column `{name}`")
                    })?;
                column.indexed = true;
            }
            if is_unique {
                uniques.push(names);
            } else {
                indexes.push(names);
            }
            continue;
        }
        if normalized.to_ascii_uppercase().starts_with("CONSTRAINT ") {
            return Err("inline SQL constraints are unsupported; declare foreign keys in a static ALTER TABLE statement".into());
        }
        let column = parse_sql_column(normalized)?;
        if column.primary_key && columns.iter().any(|existing| existing.primary_key) {
            return Err("composite or duplicate SQL primary keys are unsupported".into());
        }
        if column.laravel_method.starts_with("raw SQL: CHAR(") {
            notes.push(format!(
                "SQL CHAR column `{}` uses fixed-width storage; generated Zelyra String semantics may differ.",
                column.source_name
            ));
        }
        if column.laravel_method.starts_with("raw SQL: TINYINT(1)") {
            notes.push(format!(
                "SQL TINYINT(1) `{}` is mapped to Bool by convention; the DDL itself does not constrain values to 0/1, so validate existing data and application assumptions.",
                column.source_name
            ));
        }
        if column.laravel_method.starts_with("raw SQL: SMALLINT")
            || (column.laravel_method.starts_with("raw SQL: TINYINT")
                && !column.laravel_method.starts_with("raw SQL: TINYINT(1)"))
            || column.laravel_method.starts_with("raw SQL: INT")
        {
            notes.push(format!(
                "SQL integer width for `{}` may be wider or narrower than the generated Zelyra integer type.",
                column.source_name
            ));
        }
        if column
            .laravel_method
            .starts_with("raw SQL: BIGINT UNSIGNED")
            && !column.primary_key
        {
            notes.push(format!(
                "SQL unsigned BIGINT `{}` is mapped to UInt; verify backend range and signedness.",
                column.source_name
            ));
        }
        columns.push(column);
    }
    if columns.is_empty() || columns.iter().filter(|column| column.primary_key).count() != 1 {
        return Err(
            "deployment SQL table must declare exactly one supported primary-key column".into(),
        );
    }
    Ok((columns, indexes, uniques, notes, None))
}

fn apply_sql_foreign_constraint(
    columns: &mut [Column],
    constraint: &ForeignConstraint,
) -> Result<(), String> {
    if constraint.referenced_column != "id" {
        return Err(format!(
            "SQL foreign key `{}` references `{}`; only statically declared `id` targets are supported",
            constraint.source_column, constraint.referenced_column
        ));
    }
    let source_field = constraint
        .source_column
        .strip_suffix("_id")
        .filter(|field| !field.is_empty())
        .ok_or_else(|| {
            format!(
                "SQL foreign-key column `{}` must end in `_id` to preserve its database column",
                constraint.source_column
            )
        })?;
    let column = columns
        .iter_mut()
        .find(|column| column.source_name == constraint.source_column)
        .ok_or_else(|| {
            format!(
                "SQL foreign key refers to undeclared column `{}`",
                constraint.source_column
            )
        })?;
    if !matches!(column.ty.as_str(), "UInt" | "Int")
        || column.relation_table.is_some()
        || column.default.is_some()
        || column.primary_key
    {
        return Err(format!(
            "SQL foreign key `{}` requires a plain integer column without a default or existing relation",
            constraint.source_column
        ));
    }
    if constraint.on_delete == Some(ReferentialAction::SetNull) && !column.nullable {
        return Err(format!(
            "SQL `ON DELETE SET NULL` requires nullable column `{}`",
            constraint.source_column
        ));
    }
    if constraint.on_delete == Some(ReferentialAction::SetDefault) {
        return Err(
            "SQL `ON DELETE SET DEFAULT` is unsupported across the configured database backends"
                .into(),
        );
    }
    if constraint.source_column == "tenant_id" && constraint.target_table == "tenants" {
        column.name = constraint.source_column.clone();
        column.ty = "TenantId".into();
        column.references_table = Some(constraint.target_table.clone());
    } else {
        column.name = source_field.to_owned();
        column.ty = relation_type_for_table(&constraint.target_table)?;
        column.relation_table = Some(constraint.target_table.clone());
    }
    column.on_delete = constraint.on_delete;
    Ok(())
}

#[derive(Clone)]
struct Column {
    name: String,
    source_name: String,
    laravel_method: String,
    ty: String,
    length: Option<u32>,
    text_storage: Option<TextStorage>,
    storage_name: Option<String>,
    decimal_precision: Option<(u32, u32)>,
    enum_values: Option<Vec<String>>,
    relation_table: Option<String>,
    references_table: Option<String>,
    unresolved_relation: Option<String>,
    nullable: bool,
    primary_key: bool,
    auto_increment: bool,
    unique: bool,
    indexed: bool,
    default: Option<String>,
    on_delete: Option<ReferentialAction>,
    after: Option<String>,
}

#[derive(Clone)]
struct Candidate {
    table: String,
    file: String,
    span: SourceSpan,
    columns: Vec<Column>,
    indexes: Vec<Vec<String>>,
    uniques: Vec<Vec<String>>,
    backend_review_notes: Vec<String>,
    source_migrations: Vec<MigrationSource>,
}

#[derive(Clone)]
struct MigrationSource {
    file: String,
    span: SourceSpan,
}

#[derive(Clone)]
struct Alteration {
    table: String,
    file: String,
    span: SourceSpan,
    columns: Vec<Column>,
    indexes: Vec<Vec<String>>,
    uniques: Vec<Vec<String>>,
    backend_review_notes: Vec<String>,
    /// Set only when a single `if (!Schema::hasColumn(...))` block contains
    /// the entire statically parsed alteration callback.
    whole_callback_if_missing: Option<String>,
}

#[derive(Clone)]
struct Rejected {
    table: String,
    file: String,
    span: SourceSpan,
    reason: String,
}

struct ColumnModifiers {
    nullable: bool,
    unique: bool,
    indexed: bool,
    primary_key: bool,
    default: Option<String>,
    constrained_table: Option<String>,
    on_delete: Option<ReferentialAction>,
    after: Option<String>,
}

struct ForeignConstraint {
    source_column: String,
    constraint_name: Option<String>,
    target_table: String,
    referenced_column: String,
    on_delete: Option<ReferentialAction>,
}

struct SqlForeignConstraint {
    table: String,
    constraint: ForeignConstraint,
    file: String,
    span: SourceSpan,
}

#[derive(Default)]
pub(super) struct SchemaAnalyzer {
    candidates: Vec<Candidate>,
    alterations: Vec<Alteration>,
    alteration_rejected: Vec<Rejected>,
    rejected: Vec<Rejected>,
    sql_foreign_constraints: Vec<SqlForeignConstraint>,
    external_sql_review_notes: BTreeSet<String>,
    rollback_inventory: Vec<Value>,
}

impl SchemaAnalyzer {
    pub(super) fn scan(&mut self, file: &str, source: &str) {
        if !is_migration_file(file) {
            return;
        }
        let source_map = PhpSourceMap::new(source);
        let migration_methods = migration_method_ranges(source, &source_map);
        self.rollback_inventory.extend(rollback_inventory(
            file,
            source,
            &source_map,
            &migration_methods,
        ));
        let create = super::compiled_regex(
            r#"Schema\s*::\s*create\s*\(\s*['"]([A-Za-z_]\w*)['"]\s*,\s*function\s*\([^)]*\$([A-Za-z_]\w*)[^)]*\)\s*(?:use\s*\([^)]*\)\s*)?\{"#,
            "Laravel schema create regex",
        );
        let create_call = super::compiled_regex(
            r"Schema\s*::\s*create\s*\(",
            "Laravel schema create call regex",
        );
        let table_literal = super::compiled_regex(
            r#"^\s*(?:'([A-Za-z_]\w*)'|"([A-Za-z_]\w*)")"#,
            "Laravel schema table literal regex",
        );
        let mut supported_starts = BTreeMap::<usize, ()>::new();
        for capture in create.captures_iter(source) {
            let matched = capture.get(0).expect("schema create match");
            if !source_map.is_code(matched.start())
                || !is_forward_migration_operation(matched.start(), &migration_methods)
            {
                continue;
            }
            supported_starts.insert(matched.start(), ());
            let table = capture.get(1).expect("table name").as_str();
            let open_brace = matched.end().saturating_sub(1);
            let Some(close_brace) = matching_brace(source.as_bytes(), &source_map, open_brace)
            else {
                self.rejected.push(Rejected {
                    table: table.into(),
                    file: file.into(),
                    span: source_map.span(source, matched.range()),
                    reason: "schema creation callback has no statically matched closing brace"
                        .into(),
                });
                continue;
            };
            match parse_columns(
                &source[open_brace + 1..close_brace],
                ParseColumnsOptions {
                    body_offset: open_brace + 1,
                    source,
                    source_map: &source_map,
                    table_variable: capture
                        .get(2)
                        .expect("Blueprint callback variable")
                        .as_str(),
                    table_name: table,
                    require_primary_key: true,
                    allow_external_index_columns: false,
                },
            ) {
                Ok((columns, indexes, uniques, backend_review_notes, _)) => {
                    self.candidates.push(Candidate {
                        table: table.into(),
                        file: file.into(),
                        span: source_map.span(source, matched.range()),
                        columns,
                        indexes,
                        uniques,
                        backend_review_notes,
                        source_migrations: Vec::new(),
                    })
                }
                Err(reason) => self.rejected.push(Rejected {
                    table: table.into(),
                    file: file.into(),
                    span: source_map.span(source, matched.range()),
                    reason,
                }),
            }
        }
        for matched in create_call.find_iter(source) {
            if !source_map.is_code(matched.start())
                || !is_forward_migration_operation(matched.start(), &migration_methods)
                || supported_starts.contains_key(&matched.start())
            {
                continue;
            }
            let table = table_literal
                .captures(&source[matched.end()..])
                .and_then(|capture| capture.get(1).or_else(|| capture.get(2)))
                .map_or_else(|| "<dynamic>".to_owned(), |name| name.as_str().to_owned());
            self.rejected.push(Rejected {
                table,
                file: file.into(),
                span: source_map.span(source, matched.range()),
                reason: "Schema::create call does not match the supported static function-callback grammar".into(),
            });
        }

        let table = super::compiled_regex(
            r#"Schema\s*::\s*table\s*\(\s*['\"]([A-Za-z_]\w*)['\"]\s*,\s*function\s*\([^)]*\$([A-Za-z_]\w*)[^)]*\)\s*(?:use\s*\([^)]*\)\s*)?(?::\s*void\s*)?\{"#,
            "Laravel schema table regex",
        );
        let table_call = super::compiled_regex(
            r"Schema\s*::\s*table\s*\(",
            "Laravel schema table call regex",
        );
        let mut supported_table_starts = BTreeSet::new();
        for capture in table.captures_iter(source) {
            let matched = capture.get(0).expect("schema table match");
            if !source_map.is_code(matched.start())
                || !is_forward_migration_operation(matched.start(), &migration_methods)
            {
                continue;
            }
            supported_table_starts.insert(matched.start());
            let table_name = capture.get(1).expect("table name").as_str();
            let open_brace = matched.end().saturating_sub(1);
            let Some(close_brace) = matching_brace(source.as_bytes(), &source_map, open_brace)
            else {
                self.alteration_rejected.push(Rejected {
                    table: table_name.into(),
                    file: file.into(),
                    span: source_map.span(source, matched.range()),
                    reason: "schema alteration callback has no statically matched closing brace"
                        .into(),
                });
                continue;
            };
            match parse_columns(
                &source[open_brace + 1..close_brace],
                ParseColumnsOptions {
                    body_offset: open_brace + 1,
                    source,
                    source_map: &source_map,
                    table_variable: capture
                        .get(2)
                        .expect("Blueprint callback variable")
                        .as_str(),
                    table_name,
                    require_primary_key: false,
                    allow_external_index_columns: true,
                },
            ) {
                Ok((
                    columns,
                    indexes,
                    uniques,
                    mut backend_review_notes,
                    whole_callback_if_missing,
                )) => {
                    match outer_has_column_guard_notes(
                        source,
                        &source_map,
                        matched.start(),
                        table_name,
                    ) {
                        Ok(notes) => backend_review_notes.extend(notes),
                        Err(reason) => {
                            self.alteration_rejected.push(Rejected {
                                table: table_name.into(),
                                file: file.into(),
                                span: source_map.span(source, matched.range()),
                                reason,
                            });
                            continue;
                        }
                    }
                    self.alterations.push(Alteration {
                        table: table_name.into(),
                        file: file.into(),
                        span: source_map.span(source, matched.range()),
                        columns,
                        indexes,
                        uniques,
                        backend_review_notes,
                        whole_callback_if_missing,
                    })
                }
                Err(reason) => self.alteration_rejected.push(Rejected {
                    table: table_name.into(),
                    file: file.into(),
                    span: source_map.span(source, matched.range()),
                    reason,
                }),
            }
        }
        let arrow_table = super::compiled_regex(
            r#"Schema\s*::\s*table\s*\(\s*['\"]([A-Za-z_]\w*)['\"]\s*,\s*(?:static\s+)?fn\s*\([^)]*\$([A-Za-z_]\w*)[^)]*\)\s*=>\s*(\$([A-Za-z_]\w*)\s*->\s*[A-Za-z_]\w*\s*\([^()]*\)(?:\s*->\s*[A-Za-z_]\w*\s*\([^()]*\))*)\s*\)"#,
            "Laravel schema table arrow callback regex",
        );
        for capture in arrow_table.captures_iter(source) {
            let matched = capture.get(0).expect("schema table arrow match");
            if !source_map.is_code(matched.start())
                || !is_forward_migration_operation(matched.start(), &migration_methods)
            {
                continue;
            }
            supported_table_starts.insert(matched.start());
            let table_name = capture.get(1).expect("table name").as_str();
            let table_variable = capture.get(2).expect("arrow callback parameter").as_str();
            let expression = capture.get(3).expect("arrow callback expression");
            let expression_variable = capture.get(4).expect("Blueprint variable").as_str();
            if table_variable != expression_variable {
                self.alteration_rejected.push(Rejected {
                    table: table_name.into(),
                    file: file.into(),
                    span: source_map.span(source, matched.range()),
                    reason:
                        "schema table arrow callback must mutate its declared Blueprint parameter"
                            .into(),
                });
                continue;
            }
            let mut expression_body = expression.as_str().to_owned();
            expression_body.push(';');
            match parse_columns(
                &expression_body,
                ParseColumnsOptions {
                    body_offset: expression.start(),
                    source,
                    source_map: &source_map,
                    table_variable,
                    table_name,
                    require_primary_key: false,
                    allow_external_index_columns: true,
                },
            ) {
                Ok((
                    columns,
                    indexes,
                    uniques,
                    mut backend_review_notes,
                    whole_callback_if_missing,
                )) => {
                    match outer_has_column_guard_notes(
                        source,
                        &source_map,
                        matched.start(),
                        table_name,
                    ) {
                        Ok(notes) => backend_review_notes.extend(notes),
                        Err(reason) => {
                            self.alteration_rejected.push(Rejected {
                                table: table_name.into(),
                                file: file.into(),
                                span: source_map.span(source, matched.range()),
                                reason,
                            });
                            continue;
                        }
                    }
                    self.alterations.push(Alteration {
                        table: table_name.into(),
                        file: file.into(),
                        span: source_map.span(source, matched.range()),
                        columns,
                        indexes,
                        uniques,
                        backend_review_notes,
                        whole_callback_if_missing,
                    });
                }
                Err(reason) => self.alteration_rejected.push(Rejected {
                    table: table_name.into(),
                    file: file.into(),
                    span: source_map.span(source, matched.range()),
                    reason,
                }),
            }
        }
        for matched in table_call.find_iter(source) {
            if !source_map.is_code(matched.start())
                || !is_forward_migration_operation(matched.start(), &migration_methods)
                || supported_table_starts.contains(&matched.start())
            {
                continue;
            }
            let table_name = table_literal
                .captures(&source[matched.end()..])
                .and_then(|capture| capture.get(1).or_else(|| capture.get(2)))
                .map_or_else(
                    || "<dynamic-or-unsupported>".to_owned(),
                    |name| name.as_str().to_owned(),
                );
            self.alteration_rejected.push(Rejected {
                table: table_name,
                file: file.into(),
                span: source_map.span(source, matched.range()),
                reason: "Schema::table call does not match the supported static function-callback grammar".into(),
            });
        }
    }

    pub(super) fn scan_deploy_sql(&mut self, file: &str, source: &str) {
        if !file.starts_with("deploy/sql/") || !file.to_ascii_lowercase().ends_with(".sql") {
            return;
        }
        let code = sql_code_mask(source);
        let code_only = source
            .as_bytes()
            .iter()
            .zip(&code)
            .map(
                |(byte, is_code)| {
                    if *is_code {
                        char::from(*byte)
                    } else {
                        ' '
                    }
                },
            )
            .collect::<String>();
        let create = super::compiled_regex(
            r"(?i)CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?`?([A-Za-z_]\w*)`?\s*\(",
            "static deployment SQL CREATE TABLE regex",
        );
        let mut create_starts = BTreeSet::new();
        for capture in create.captures_iter(source) {
            let matched = capture.get(0).expect("SQL CREATE TABLE match");
            if !code.get(matched.start()).copied().unwrap_or(false) {
                continue;
            }
            create_starts.insert(matched.start());
            let table = capture.get(1).expect("SQL table name").as_str();
            let open = matched.end().saturating_sub(1);
            let Some(close) = matching_sql_delimiter(source, open, b'(', b')') else {
                self.rejected.push(Rejected {
                    table: table.into(),
                    file: file.into(),
                    span: SourceSpan::new(source, matched.range()),
                    reason:
                        "deployment SQL CREATE TABLE has no statically matched closing parenthesis"
                            .into(),
                });
                continue;
            };
            match parse_sql_create_body(&source[open + 1..close], table) {
                Ok((columns, indexes, uniques, backend_review_notes, _)) => {
                    self.candidates.push(Candidate {
                        table: table.into(),
                        file: file.into(),
                        span: SourceSpan::new(source, matched.range()),
                        columns,
                        indexes,
                        uniques,
                        backend_review_notes,
                        source_migrations: Vec::new(),
                    });
                }
                Err(reason) => self.rejected.push(Rejected {
                    table: table.into(),
                    file: file.into(),
                    span: SourceSpan::new(source, matched.range()),
                    reason,
                }),
            }
        }
        let create_detector = super::compiled_regex(
            r"(?i)\bCREATE\s+TABLE\b",
            "deployment SQL CREATE TABLE detector regex",
        );
        for matched in create_detector.find_iter(source) {
            if code.get(matched.start()).copied().unwrap_or(false)
                && !create_starts.contains(&matched.start())
            {
                self.rejected.push(Rejected {
                    table: "<dynamic-or-unsupported>".into(),
                    file: file.into(),
                    span: SourceSpan::new(source, matched.range()),
                    reason:
                        "deployment SQL CREATE TABLE does not match the supported static grammar"
                            .into(),
                });
            }
        }

        let alter_table = super::compiled_regex(
            r"(?i)ALTER\s+TABLE\s+`?([A-Za-z_]\w*)`?",
            "static deployment SQL ALTER TABLE target regex",
        );
        let add_column_clause = super::compiled_regex(
            r"(?is)^ADD\s+COLUMN\s+(?:IF\s+NOT\s+EXISTS\s+)?(.+)$",
            "static deployment SQL ADD COLUMN clause regex",
        );
        let foreign_key_clause = super::compiled_regex(
            r"(?is)^ADD\s+CONSTRAINT\s+`?[A-Za-z_]\w*`?\s+FOREIGN\s+KEY\s*\(\s*`?[A-Za-z_]\w*`?\s*\)\s+REFERENCES\s+`?[A-Za-z_]\w*`?\s*\(\s*`?[A-Za-z_]\w*`?\s*\)(?:\s+ON\s+DELETE\s+(?:CASCADE|RESTRICT|NO\s+ACTION|SET\s+NULL|SET\s+DEFAULT))?$",
            "static deployment SQL foreign-key clause validator regex",
        );
        let mut valid_add_column_starts = BTreeSet::new();
        let mut valid_foreign_alter_starts = BTreeSet::new();
        let mut invalid_alter_starts = BTreeSet::new();
        for capture in alter_table.captures_iter(source) {
            let matched = capture.get(0).expect("SQL ALTER TABLE match");
            if !code.get(matched.start()).copied().unwrap_or(false) {
                continue;
            }
            let table = capture.get(1).expect("SQL altered table").as_str();
            let statement_end = sql_statement_end(source, &code, matched.end());
            let body = source[matched.end()..statement_end].trim();
            let clauses = split_sql_items(body);
            let supported_add_column = clauses.as_ref().is_ok_and(|clauses| {
                clauses.len() == 1
                    && add_column_clause
                        .captures(clauses[0])
                        .and_then(|capture| capture.get(1))
                        .is_some_and(|definition| parse_sql_column(definition.as_str()).is_ok())
            });
            let supported_foreign_keys = clauses.as_ref().is_ok_and(|clauses| {
                !clauses.is_empty()
                    && clauses.iter().all(|clause| {
                        foreign_key_clause.find(clause).is_some_and(|matched| {
                            matched.start() == 0 && matched.end() == clause.len()
                        })
                    })
            });
            if supported_add_column {
                valid_add_column_starts.insert(matched.start());
            } else if supported_foreign_keys {
                valid_foreign_alter_starts.insert(matched.start());
            } else {
                invalid_alter_starts.insert(matched.start());
                self.alteration_rejected.push(Rejected {
                    table: table.into(),
                    file: file.into(),
                    span: SourceSpan::new(source, matched.range()),
                    reason: "deployment SQL ALTER TABLE statement contains a clause outside the supported static subset".into(),
                });
            }
        }

        let add_column = super::compiled_regex(
            r"(?i)ALTER\s+TABLE\s+`?([A-Za-z_]\w*)`?\s+ADD\s+COLUMN\s+(?:IF\s+NOT\s+EXISTS\s+)?([^;]+)",
            "static deployment SQL ALTER TABLE ADD COLUMN regex",
        );
        let mut recognized_alter_starts = BTreeSet::new();
        for capture in add_column.captures_iter(source) {
            let matched = capture.get(0).expect("SQL ADD COLUMN match");
            if !code.get(matched.start()).copied().unwrap_or(false)
                || !valid_add_column_starts.contains(&matched.start())
            {
                continue;
            }
            recognized_alter_starts.insert(matched.start());
            let table = capture.get(1).expect("SQL altered table").as_str();
            let definition = capture.get(2).expect("SQL column definition").as_str();
            match parse_sql_column(definition) {
                Ok(column) => self.alterations.push(Alteration {
                    table: table.into(),
                    file: file.into(),
                    span: SourceSpan::new(source, matched.range()),
                    columns: vec![column],
                    indexes: Vec::new(),
                    uniques: Vec::new(),
                    backend_review_notes: vec![format!(
                        "Column `{}` comes from a custom deployment SQL ALTER TABLE statement; review idempotence and backend equivalence.",
                        capture.get(2).map_or("<unknown>", |value| value.as_str())
                    )],
                    whole_callback_if_missing: None,
                }),
                Err(reason) => self.alteration_rejected.push(Rejected {
                    table: table.into(),
                    file: file.into(),
                    span: SourceSpan::new(source, matched.range()),
                    reason,
                }),
            }
        }

        let foreign_key = super::compiled_regex(
            r"(?i)ADD\s+CONSTRAINT\s+`?([A-Za-z_]\w*)`?\s+FOREIGN\s+KEY\s*\(\s*`?([A-Za-z_]\w*)`?\s*\)\s+REFERENCES\s+`?([A-Za-z_]\w*)`?\s*\(\s*`?([A-Za-z_]\w*)`?\s*\)(?:\s+ON\s+DELETE\s+(CASCADE|RESTRICT|NO\s+ACTION|SET\s+NULL|SET\s+DEFAULT))?",
            "static deployment SQL foreign-key regex",
        );
        let mut recognized_foreign_alter_starts = BTreeSet::new();
        for capture in foreign_key.captures_iter(source) {
            let matched = capture.get(0).expect("SQL foreign key match");
            if !code.get(matched.start()).copied().unwrap_or(false) {
                continue;
            }
            let Some((alter_start, table)) = alter_table
                .captures_iter(&source[..matched.start()])
                .filter_map(|table_capture| {
                    let table_match = table_capture.get(0)?;
                    code.get(table_match.start())
                        .copied()
                        .unwrap_or(false)
                        .then(|| {
                            (
                                table_match.start(),
                                table_capture.get(1).expect("alter table name").as_str(),
                            )
                        })
                })
                .last()
            else {
                self.external_sql_review_notes.insert(format!(
                    "Deployment SQL foreign key in `{file}` has no statically resolvable ALTER TABLE target."
                ));
                continue;
            };
            if !valid_foreign_alter_starts.contains(&alter_start) {
                continue;
            }
            recognized_foreign_alter_starts.insert(alter_start);
            let action = capture.get(5).and_then(|value| {
                let normalized = value.as_str().to_ascii_uppercase().replace(' ', "");
                match normalized.as_str() {
                    "CASCADE" => Some(ReferentialAction::Cascade),
                    "RESTRICT" => Some(ReferentialAction::Restrict),
                    "NOACTION" => Some(ReferentialAction::NoAction),
                    "SETNULL" => Some(ReferentialAction::SetNull),
                    "SETDEFAULT" => Some(ReferentialAction::SetDefault),
                    _ => None,
                }
            });
            self.sql_foreign_constraints.push(SqlForeignConstraint {
                table: table.to_owned(),
                constraint: ForeignConstraint {
                    source_column: capture
                        .get(2)
                        .expect("SQL FK source column")
                        .as_str()
                        .into(),
                    constraint_name: Some(capture.get(1).expect("SQL FK name").as_str().into()),
                    target_table: capture.get(3).expect("SQL FK target").as_str().into(),
                    referenced_column: capture
                        .get(4)
                        .expect("SQL FK target column")
                        .as_str()
                        .into(),
                    on_delete: action,
                },
                file: file.into(),
                span: SourceSpan::new(source, matched.range()),
            });
        }

        if code_only.to_ascii_uppercase().contains("CREATE TRIGGER") {
            self.external_sql_review_notes.insert(format!(
                "Trigger behavior in `{file}` is not translated into Zelyra schema or runtime logic."
            ));
        }
        let unsupported_alter = super::compiled_regex(
            r"(?i)ALTER\s+TABLE\s+`?([A-Za-z_]\w*)`?",
            "deployment SQL ALTER TABLE detector regex",
        );
        for capture in unsupported_alter.captures_iter(source) {
            let matched = capture.get(0).expect("SQL ALTER TABLE match");
            if !code.get(matched.start()).copied().unwrap_or(false)
                || invalid_alter_starts.contains(&matched.start())
                || recognized_alter_starts.contains(&matched.start())
                || recognized_foreign_alter_starts.contains(&matched.start())
            {
                continue;
            }
            self.alteration_rejected.push(Rejected {
                table: capture.get(1).expect("alter table name").as_str().into(),
                file: file.into(),
                span: SourceSpan::new(source, matched.range()),
                reason:
                    "deployment SQL ALTER TABLE operation is outside the supported static subset"
                        .into(),
            });
        }
        if !create_starts.is_empty()
            && code_only[create_starts.iter().next().copied().unwrap_or_default()..]
                .to_ascii_uppercase()
                .contains("ENGINE=")
        {
            self.external_sql_review_notes.insert(format!(
                "Storage engine, charset, and collation options in `{file}` are not preserved in generated Zelyra schema."
            ));
        }
    }

    pub(super) fn finish(mut self) -> Value {
        let mut field_evidence = self
            .candidates
            .iter()
            .flat_map(|candidate| {
                candidate.columns.iter().map(move |column| {
                    json!({
                        "table": candidate.table,
                        "source_column_name": column.source_name,
                        "zelyra_type": column.ty,
                        "nullable": column.nullable,
                        "source_file": candidate.file,
                        "source_span": span_json(candidate.span),
                        "origin": "create_candidate"
                    })
                })
            })
            .chain(self.alterations.iter().flat_map(|alteration| {
                alteration.columns.iter().map(move |column| {
                    json!({
                        "table": alteration.table,
                        "source_column_name": column.source_name,
                        "zelyra_type": column.ty,
                        "nullable": column.nullable,
                        "source_file": alteration.file,
                        "source_span": span_json(alteration.span),
                        "origin": "table_alteration"
                    })
                })
            }))
            .collect::<Vec<_>>();
        field_evidence.sort_by(|left, right| {
            (
                left["table"].as_str(),
                left["source_column_name"].as_str(),
                left["source_file"].as_str(),
            )
                .cmp(&(
                    right["table"].as_str(),
                    right["source_column_name"].as_str(),
                    right["source_file"].as_str(),
                ))
        });
        let mut occurrences = BTreeMap::<String, usize>::new();
        for candidate in &self.candidates {
            *occurrences.entry(candidate.table.clone()).or_default() += 1;
        }
        for rejected in &self.rejected {
            *occurrences.entry(rejected.table.clone()).or_default() += 1;
        }
        let duplicate_tables = occurrences
            .iter()
            .filter_map(|(table, count)| (*count > 1).then_some(table.clone()))
            .collect::<BTreeSet<_>>();
        for rejected in &mut self.rejected {
            if duplicate_tables.contains(&rejected.table) {
                rejected.reason = "multiple Schema::create declarations use this table name; no declaration was generated".into();
            }
        }
        let mut unique = Vec::new();
        for candidate in self.candidates {
            if occurrences.get(&candidate.table).copied().unwrap_or(0) == 1 {
                unique.push(candidate);
            } else {
                self.rejected.push(Rejected {
                    table: candidate.table.clone(),
                    file: candidate.file,
                    span: candidate.span,
                    reason: "multiple Schema::create declarations use this table name; no declaration was generated".into(),
                });
            }
        }
        let reject_all_candidates = self
            .alteration_rejected
            .iter()
            .any(|rejected| rejected.table == "<dynamic-or-unsupported>");
        let mut rejected_tables = self
            .alteration_rejected
            .iter()
            .filter(|rejected| rejected.table != "<dynamic-or-unsupported>")
            .map(|rejected| rejected.table.clone())
            .collect::<BTreeSet<_>>();
        self.rejected.append(&mut self.alteration_rejected);
        let mut alterations_by_table = BTreeMap::<String, Vec<Alteration>>::new();
        for alteration in self.alterations {
            alterations_by_table
                .entry(alteration.table.clone())
                .or_default()
                .push(alteration);
        }
        for (table_name, alterations) in alterations_by_table {
            let matching = unique
                .iter()
                .enumerate()
                .filter_map(|(index, candidate)| (candidate.table == table_name).then_some(index))
                .collect::<Vec<_>>();
            if matching.len() != 1 {
                let reason = if matching.is_empty() {
                    "Schema::table alteration has no unique supported Schema::create candidate; the change was not merged"
                } else {
                    "Schema::table alteration matches multiple schema candidates; the change was not merged"
                };
                for alteration in alterations {
                    self.rejected.push(Rejected {
                        table: alteration.table,
                        file: alteration.file,
                        span: alteration.span,
                        reason: reason.into(),
                    });
                }
                continue;
            }
            let candidate_index = matching[0];
            let mut merged = unique[candidate_index].clone();
            let mut error = None;
            for alteration in &alterations {
                if alteration
                    .whole_callback_if_missing
                    .as_ref()
                    .is_some_and(|guarded| {
                        merged
                            .columns
                            .iter()
                            .any(|existing| existing.source_name == *guarded)
                    })
                {
                    let guarded = alteration
                        .whole_callback_if_missing
                        .as_deref()
                        .expect("whole-callback guard checked above");
                    merged.backend_review_notes.push(format!(
                        "Laravel `Schema::table` callback in `{}` is guarded by `!Schema::hasColumn('{table_name}', '{guarded}')`; a previous migration statically declares the column, so this entire callback is skipped when replaying the migrations in order.",
                        alteration.file
                    ));
                    merged.source_migrations.push(MigrationSource {
                        file: alteration.file.clone(),
                        span: alteration.span,
                    });
                    continue;
                }
                if let Some(column) = alteration.columns.iter().find(|column| {
                    merged.columns.iter().any(|existing| {
                        existing.source_name == column.source_name || existing.name == column.name
                    })
                }) {
                    self.rejected.push(Rejected {
                        table: alteration.table.clone(),
                        file: alteration.file.clone(),
                        span: alteration.span,
                        reason: format!(
                            "Schema::table adds duplicate column `{}`; no changes from this alteration were merged",
                            column.source_name
                        ),
                    });
                    continue;
                }
                merged.columns.extend(alteration.columns.clone());
                merged.indexes.extend(alteration.indexes.clone());
                merged.uniques.extend(alteration.uniques.clone());
                merged
                    .backend_review_notes
                    .extend(alteration.backend_review_notes.clone());
                merged.source_migrations.push(MigrationSource {
                    file: alteration.file.clone(),
                    span: alteration.span,
                });
            }
            if error.is_none() {
                let source_to_field = merged
                    .columns
                    .iter()
                    .map(|column| (column.source_name.clone(), column.name.clone()))
                    .collect::<BTreeMap<_, _>>();
                for column_name in merged
                    .indexes
                    .iter_mut()
                    .chain(merged.uniques.iter_mut())
                    .flatten()
                {
                    if let Some(mapped_name) = source_to_field.get(column_name) {
                        *column_name = mapped_name.clone();
                    }
                }
            }
            if error.is_none() {
                if let Err(reason) = order_columns_by_after(&mut merged.columns) {
                    error = Some(reason);
                }
            }
            if error.is_none() {
                for (kind, index) in merged
                    .indexes
                    .iter()
                    .map(|index| ("index", index))
                    .chain(merged.uniques.iter().map(|unique| ("unique", unique)))
                {
                    if let Some(missing) = index.iter().find(|source_column| {
                        !merged
                            .columns
                            .iter()
                            .any(|column| column.name == **source_column)
                    }) {
                        error = Some(format!(
                            "{kind} declaration refers to unsupported or missing field `{missing}` after migration merge"
                        ));
                        break;
                    }
                }
            }
            if let Some(reason) = error {
                rejected_tables.insert(table_name);
                for alteration in alterations {
                    self.rejected.push(Rejected {
                        table: alteration.table,
                        file: alteration.file,
                        span: alteration.span,
                        reason: reason.clone(),
                    });
                }
            } else {
                merged.indexes.sort();
                merged.indexes.dedup();
                merged.uniques.sort();
                merged.uniques.dedup();
                unique[candidate_index] = merged;
            }
        }
        for foreign in std::mem::take(&mut self.sql_foreign_constraints) {
            let Some(candidate) = unique
                .iter_mut()
                .find(|candidate| candidate.table == foreign.table)
            else {
                self.rejected.push(Rejected {
                    table: foreign.table,
                    file: foreign.file,
                    span: foreign.span,
                    reason: "deployment SQL foreign key has no unique generated table candidate"
                        .into(),
                });
                continue;
            };
            match apply_sql_foreign_constraint(&mut candidate.columns, &foreign.constraint) {
                Ok(()) => {
                    if let Some(name) = foreign.constraint.constraint_name.as_deref() {
                        candidate.backend_review_notes.push(format!(
                            "SQL foreign-key constraint name `{name}` is not preserved by generated Zelyra schema."
                        ));
                    }
                    candidate.source_migrations.push(MigrationSource {
                        file: foreign.file,
                        span: foreign.span,
                    });
                }
                Err(reason) => {
                    rejected_tables.insert(foreign.table.clone());
                    self.rejected.push(Rejected {
                        table: foreign.table,
                        file: foreign.file,
                        span: foreign.span,
                        reason,
                    });
                }
            }
        }
        if reject_all_candidates {
            rejected_tables.extend(unique.iter().map(|candidate| candidate.table.clone()));
        }
        unique.retain(|candidate| !rejected_tables.contains(&candidate.table));
        let available = unique
            .iter()
            .map(|candidate: &Candidate| candidate.table.clone())
            .collect::<BTreeSet<_>>();
        let tenant_scoped_tables = unique
            .iter()
            .filter(|candidate| {
                candidate.columns.iter().any(|column| {
                    column.name == "tenant_id"
                        && !column.nullable
                        && column.references_table.as_deref() == Some("tenants")
                })
            })
            .map(|candidate| candidate.table.clone())
            .collect::<BTreeSet<_>>();
        for candidate in &mut unique {
            let mut changed_fields = BTreeMap::<String, String>::new();
            let source_is_tenant_scoped = tenant_scoped_tables.contains(&candidate.table);
            for column in &mut candidate.columns {
                if let Some(target) = column
                    .references_table
                    .as_ref()
                    .filter(|target| !available.contains(*target))
                    .cloned()
                {
                    column.unresolved_relation = Some(target);
                    column.ty = "UInt".into();
                    column.references_table = None;
                    column.on_delete = None;
                    continue;
                }
                if let Some(target) = column
                    .relation_table
                    .as_ref()
                    .filter(|target| !available.contains(*target))
                    .cloned()
                {
                    let field_name = column.name.clone();
                    column.unresolved_relation = Some(target);
                    column.name = column.source_name.clone();
                    column.ty = "UInt".into();
                    column.relation_table = None;
                    changed_fields.insert(field_name, column.name.clone());
                }
                if column.on_delete == Some(ReferentialAction::SetNull)
                    && source_is_tenant_scoped
                    && column
                        .relation_table
                        .as_ref()
                        .is_some_and(|target| tenant_scoped_tables.contains(target))
                {
                    candidate.backend_review_notes.push(format!(
                        "Laravel relation `{}.{}` declares `ON DELETE SET NULL`, but a tenant-safe composite foreign key cannot null the required `tenant_id` column. The generated candidate uses `RESTRICT`; review and implement equivalent deletion behavior explicitly.",
                        candidate.table, column.source_name
                    ));
                    column.on_delete = Some(ReferentialAction::Restrict);
                }
            }
            for field in candidate
                .indexes
                .iter_mut()
                .chain(candidate.uniques.iter_mut())
                .flatten()
            {
                if let Some(replacement) = changed_fields.get(field) {
                    *field = replacement.clone();
                }
            }
            let source_to_field = candidate
                .columns
                .iter()
                .map(|column| (column.source_name.as_str(), column.name.as_str()))
                .collect::<BTreeMap<_, _>>();
            for field in candidate
                .indexes
                .iter_mut()
                .chain(candidate.uniques.iter_mut())
                .flatten()
            {
                if let Some(replacement) = source_to_field.get(field.as_str()) {
                    *field = (*replacement).to_owned();
                }
            }
        }
        unique.sort_by(|left, right| left.table.cmp(&right.table));
        self.rejected.sort_by(|left, right| {
            (&left.table, &left.file, left.span.start_byte).cmp(&(
                &right.table,
                &right.file,
                right.span.start_byte,
            ))
        });
        self.rollback_inventory.sort_by(|left, right| {
            left["source_file"]
                .as_str()
                .cmp(&right["source_file"].as_str())
                .then_with(|| {
                    left["source_span"]["start"]["byte_offset"]
                        .as_u64()
                        .cmp(&right["source_span"]["start"]["byte_offset"].as_u64())
                })
        });
        let source = unique
            .iter()
            .map(render_table)
            .collect::<Vec<_>>()
            .join("\n\n");
        let source = if unique.iter().any(|candidate| {
            candidate.columns.iter().any(|column| {
                column.name == "tenant_id" && column.references_table.as_deref() == Some("tenants")
            })
        }) {
            format!("type TenantId = Id\n\n{source}")
        } else {
            source
        };
        json!({
            "schema_version": "7",
            "rollback_inventory": {
                "schema_version": "1",
                "semantics": "static_down_method_inventory_only_not_rollback_equivalence_or_execution",
                "migrations": self.rollback_inventory
            },
            "mode": "preview_only_no_database_changes",
            "semantic_parity": "not_proven_requires_backend_review",
            "requires_manual_review": true,
            "external_sql_review_notes": self.external_sql_review_notes,
            "generated_table_count": unique.len(),
            "field_evidence": field_evidence,
            "candidate_source": source,
            "enum_adapters": unique.iter().flat_map(|candidate| {
                candidate.columns.iter().filter_map(move |column| {
                    column.enum_values.as_ref().map(|values| {
                        let (
                            type_name,
                            parse_function,
                            storage_function,
                            candidate_source,
                            variants,
                        ) =
                            render_enum_adapter(&candidate.table, &column.name, values);
                        json!({
                            "table": candidate.table,
                            "column": column.source_name,
                            "zelyra_type": type_name,
                            "parse_function": parse_function,
                            "storage_function": storage_function,
                            "values": values,
                            "variants": variants.iter().map(|(name, value)| json!({
                                "name": name,
                                "storage_value": value
                            })).collect::<Vec<_>>(),
                            "classification": "assisted",
                            "storage_constraint_preserved": false,
                            "candidate_source": candidate_source
                        })
                    })
                })
            }).collect::<Vec<_>>(),
            "enum_adapter_source": unique.iter().flat_map(|candidate| {
                candidate.columns.iter().filter_map(move |column| {
                    column.enum_values.as_ref().map(|values| {
                        render_enum_adapter(&candidate.table, &column.name, values).3
                    })
                })
            }).collect::<Vec<_>>().join("\n\n"),
            "candidates": unique.iter().map(|candidate| json!({
                "table": candidate.table,
                "source_file": candidate.file,
                "source_kind": if candidate.file.starts_with("deploy/sql/") { "deployment_sql" } else { "laravel_migration" },
                "source_span": span_json(candidate.span),
                "candidate_source": render_table(candidate),
                "source_migrations": candidate.source_migrations.iter().map(|migration| json!({
                    "source_file": migration.file,
                    "source_span": span_json(migration.span)
                })).collect::<Vec<_>>(),
                "status": "candidate_requires_manual_backend_review",
                "columns": candidate.columns.iter().map(|column| {
                    let mut metadata = json!({
                        "name": column.name,
                        "source_column_name": column.source_name,
                        "laravel_blueprint_method": column.laravel_method,
                        "source_declaration": column.laravel_method,
                        "zelyra_type": column.ty,
                        "relation_table": column.relation_table,
                        "references_table": column.references_table,
                        "unresolved_relation_target": column.unresolved_relation,
                        "relation_status": if column.unresolved_relation.is_some() { "target_unavailable_emitted_as_unsigned_id" } else if column.references_table.is_some() { "scalar_id_reference" } else if column.relation_table.is_some() { "resolved" } else { "not_a_foreign_key" },
                        "mapping_review": mapping_review_note(&column.laravel_method),
                        "length": column.length,
                        "text_storage": column.text_storage.map(text_storage_name),
                        "unsigned_storage": unsigned_storage_for_method(&column.laravel_method),
                        "allowed_values": column.enum_values,
                        "decimal_precision": column.decimal_precision.map(|(precision, scale)| json!({ "precision": precision, "scale": scale })),
                        "nullable": column.nullable,
                        "primary_key": column.primary_key,
                        "auto_increment": column.auto_increment,
                        "unique": column.unique,
                        "indexed": column.indexed,
                        "default": column.default,
                        "on_delete": column.on_delete.map(|action| action.to_string()),
                        "after": column.after
                    });
                    metadata["storage_name"] = json!(column.storage_name);
                    metadata
                }).collect::<Vec<_>>(),
                "indexes": candidate.indexes,
                "uniques": candidate.uniques,
                "backend_review_notes": candidate.backend_review_notes
            })).collect::<Vec<_>>(),
            "rejected": self.rejected.iter().map(|rejected| json!({
                "table": rejected.table,
                "source_file": rejected.file,
                "source_span": span_json(rejected.span),
                "reason": rejected.reason
            })).collect::<Vec<_>>()
        })
    }
}

fn is_migration_file(file: &str) -> bool {
    let components = Path::new(file)
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();
    components.windows(2).any(|pair| {
        pair[0].eq_ignore_ascii_case("database") && pair[1].eq_ignore_ascii_case("migrations")
            || pair[0].eq_ignore_ascii_case("app") && pair[1].eq_ignore_ascii_case("migrations")
    })
}

#[derive(Default)]
struct MigrationMethodRanges {
    up: Vec<Range<usize>>,
    down: Vec<DownMethod>,
    has_up_or_down: bool,
}

struct DownMethod {
    body: Range<usize>,
    span: SourceSpan,
}

fn migration_method_ranges(source: &str, source_map: &PhpSourceMap) -> MigrationMethodRanges {
    let method = super::compiled_regex(
        r"\bfunction\s+(up|down)\s*\([^)]*\)\s*(?::\s*[^\{]+)?\s*\{",
        "Laravel migration method regex",
    );
    let mut ranges = MigrationMethodRanges::default();
    for capture in method.captures_iter(source) {
        let matched = capture.get(0).expect("migration method match");
        if !source_map.is_code(matched.start()) {
            continue;
        }
        ranges.has_up_or_down = true;
        let open_brace = matched.end().saturating_sub(1);
        let Some(close_brace) = matching_brace(source.as_bytes(), source_map, open_brace) else {
            continue;
        };
        if capture.get(1).is_some_and(|name| name.as_str() == "up") {
            ranges.up.push(open_brace + 1..close_brace);
        } else {
            ranges.down.push(DownMethod {
                body: open_brace + 1..close_brace,
                span: source_map.span(source, matched.start()..close_brace + 1),
            });
        }
    }
    ranges
}

fn rollback_inventory(
    file: &str,
    source: &str,
    source_map: &PhpSourceMap,
    methods: &MigrationMethodRanges,
) -> Vec<Value> {
    let operation = super::compiled_regex(
        r"\b(Schema|DB)\s*::\s*([A-Za-z_]\w*)\s*\(",
        "Laravel rollback operation regex",
    );
    let literal_table = super::compiled_regex(
        r#"^\s*(?:'([A-Za-z_]\w*)'|"([A-Za-z_]\w*)")"#,
        "Laravel rollback literal table regex",
    );
    if methods.down.is_empty() {
        return vec![json!({
            "source_file": file,
            "down_method": "missing",
            "rollback_status": "not_assessed_missing_down_method",
            "operations": []
        })];
    }
    methods
        .down
        .iter()
        .map(|down| {
            let mut operations = Vec::new();
            for capture in operation.captures_iter(source) {
                let matched = capture.get(0).expect("rollback operation match");
                if !down.body.contains(&matched.start()) || !source_map.is_code(matched.start()) {
                    continue;
                }
                let facade = capture.get(1).expect("rollback facade").as_str();
                let method = capture.get(2).expect("rollback method").as_str();
                let table = (facade == "Schema")
                    .then(|| {
                        literal_table
                            .captures(&source[matched.end()..])
                            .and_then(|capture| capture.get(1).or_else(|| capture.get(2)))
                            .map(|capture| capture.as_str())
                    })
                    .flatten();
                operations.push(json!({
                    "facade": facade,
                    "method": method,
                    "literal_table": table,
                    "source_span": span_json(source_map.span(source, matched.range()))
                }));
            }
            json!({
                "source_file": file,
                "down_method": "present",
                "source_span": span_json(down.span),
                "rollback_status": "static_operations_require_semantic_review",
                "operations": operations
            })
        })
        .collect()
}

fn is_forward_migration_operation(offset: usize, methods: &MigrationMethodRanges) -> bool {
    !methods.has_up_or_down || methods.up.iter().any(|range| range.contains(&offset))
}

fn outer_has_column_guard_notes(
    source: &str,
    source_map: &PhpSourceMap,
    schema_table_start: usize,
    table_name: &str,
) -> Result<Vec<String>, String> {
    let if_block = super::compiled_regex(
        r"if\s*\(([^{}]*)\)\s*\{",
        "Laravel enclosing if-block regex",
    );
    let static_guard = super::compiled_regex(
        r"^\s*(?:Schema\s*::\s*hasTable\s*\(\s*'([A-Za-z_]\w*)'\s*\)\s*&&\s*)?!\s*Schema\s*::\s*hasColumn\s*\(\s*'([A-Za-z_]\w*)'\s*,\s*'([A-Za-z_]\w*)'\s*\)\s*$",
        "static Laravel outer hasColumn guard regex",
    );
    let bytes = source.as_bytes();
    let mut notes = Vec::new();
    for capture in if_block.captures_iter(source) {
        let matched = capture.get(0).expect("if block match");
        if !source_map.is_code(matched.start()) {
            continue;
        }
        let open_brace = matched.end().saturating_sub(1);
        let Some(close_brace) = matching_brace(bytes, source_map, open_brace) else {
            continue;
        };
        if !(open_brace < schema_table_start && schema_table_start < close_brace) {
            continue;
        }
        let condition = capture.get(1).expect("if condition").as_str();
        if !condition.contains("hasColumn") {
            continue;
        }
        let Some(guard) = static_guard.captures(condition) else {
            return Err(
                "Schema::table is enclosed by a dynamic or unsupported hasColumn guard".into(),
            );
        };
        let has_table = guard.get(1).map(|value| value.as_str());
        let guarded_table = guard.get(2).expect("guard table").as_str();
        let guarded_column = guard.get(3).expect("guard column").as_str();
        if guarded_table != table_name || has_table.is_some_and(|name| name != table_name) {
            return Err(format!(
                "outer `hasColumn` guard does not consistently name schema table `{table_name}`"
            ));
        }
        notes.push(format!(
            "Laravel outer `hasColumn` guard for `{guarded_column}` is not evaluated; declarations in the guarded branch are included as candidates and must be reviewed against the target database."
        ));
    }
    Ok(notes)
}

fn parse_columns(
    body: &str,
    options: ParseColumnsOptions<'_>,
) -> Result<ParsedSchemaColumns, String> {
    let ParseColumnsOptions {
        body_offset,
        source,
        source_map,
        table_variable,
        table_name,
        require_primary_key,
        allow_external_index_columns,
    } = options;
    let call = super::compiled_regex(
        r"\$([A-Za-z_]\w*)\s*->\s*[A-Za-z_]\w*\s*\(",
        "Laravel table call detector regex",
    );
    let operation = super::compiled_regex(
        r#"\$([A-Za-z_]\w*)\s*->\s*([A-Za-z_]\w*)\s*\(\s*([^()]*)\s*\)((?:\s*->\s*[A-Za-z_]\w*\s*\(\s*[^()]*\s*\))*)\s*;"#,
        "Laravel table operation regex",
    );
    let column_guard = super::compiled_regex(
        r"if\s*\(\s*!\s*Schema\s*::\s*hasColumn\s*\(\s*'([A-Za-z_]\w*)'\s*,\s*'([A-Za-z_]\w*)'\s*\)\s*\)\s*\{",
        "static Laravel hasColumn guard regex",
    );
    let mut columns = Vec::new();
    let mut indexes = Vec::<Vec<String>>::new();
    let mut uniques = Vec::<Vec<String>>::new();
    let mut backend_review_notes = Vec::<String>::new();
    let mut foreign_constraints = Vec::<ForeignConstraint>::new();
    let mut foreign_columns = BTreeSet::<String>::new();
    let mut primary_key_count = 0;
    let mut seen_columns = BTreeMap::<String, usize>::new();
    let mut recognized_starts = BTreeMap::<usize, ()>::new();
    let mut recognized_bytes = vec![false; body.len()];
    let mut guards = Vec::<(Range<usize>, usize, String, String)>::new();
    let static_foreign_id_loop = super::compiled_regex(
        r"foreach\s*\(\s*\[([^\]]*)\]\s*as\s*\$([A-Za-z_]\w*)\s*=>\s*\$([A-Za-z_]\w*)\s*\)\s*(?:\{\s*)?\$([A-Za-z_]\w*)\s*->\s*foreignId\s*\(\s*\$([A-Za-z_]\w*)\s*\)((?:\s*->\s*[A-Za-z_]\w*\s*\(\s*[^()]*\s*\))*)\s*;\s*\}?",
        "static Laravel foreignId foreach regex",
    );
    let mut foreach_ranges = Vec::<Range<usize>>::new();
    let mut foreach_expansions = Vec::<String>::new();

    for capture in column_guard.captures_iter(body) {
        let matched = capture.get(0).expect("hasColumn guard match");
        let absolute_start = body_offset + matched.start();
        if !source_map.is_code(absolute_start) {
            continue;
        }
        let open_brace = body_offset + matched.end().saturating_sub(1);
        let Some(close_brace) = matching_brace(source.as_bytes(), source_map, open_brace) else {
            continue;
        };
        let close_relative = close_brace.saturating_sub(body_offset);
        if close_relative >= body.len() {
            continue;
        }
        guards.push((
            matched.start()..close_relative + 1,
            matched.end() - 1,
            capture.get(1).expect("guard table").as_str().to_owned(),
            capture.get(2).expect("guard column").as_str().to_owned(),
        ));
    }

    for capture in static_foreign_id_loop.captures_iter(body) {
        let matched = capture.get(0).expect("static foreignId foreach match");
        let matched_text = matched.as_str();
        if matched_text.contains('{') != matched_text.contains('}') {
            return Err(
                "static foreignId foreach block must contain exactly one supported statement"
                    .into(),
            );
        }
        let absolute_start = body_offset + matched.start();
        if !source_map.is_code(absolute_start) {
            continue;
        }
        let entries = capture.get(1).expect("foreach mapping entries").as_str();
        let column_variable = capture.get(2).expect("foreach column variable").as_str();
        let target_variable = capture.get(3).expect("foreach target variable").as_str();
        let blueprint_variable = capture.get(4).expect("foreach Blueprint variable").as_str();
        let argument_variable = capture.get(5).expect("foreach call variable").as_str();
        if blueprint_variable != table_variable || argument_variable != column_variable {
            return Err(
                "static foreignId foreach uses a different Blueprint or column variable".into(),
            );
        }
        let pairs = split_php_arguments(entries)?;
        if pairs.is_empty() {
            return Err(
                "static foreignId foreach needs at least one literal column-to-table pair".into(),
            );
        }
        let constrained_pattern = format!(
            r"->\s*constrained\s*\(\s*\${}\s*\)",
            regex::escape(target_variable)
        );
        let constrained_variable = regex::Regex::new(&constrained_pattern)
            .map_err(|_| "could not parse the static foreignId foreach target".to_owned())?;
        let modifiers = capture.get(6).expect("foreach modifiers").as_str();
        if constrained_variable.find_iter(modifiers).count() != 1 {
            return Err(
                "static foreignId foreach must use exactly one constrained target variable".into(),
            );
        }
        let mut seen_keys = BTreeSet::new();
        let mut expansions = Vec::new();
        for pair in pairs {
            let Some((key, value)) = pair.split_once("=>") else {
                return Err(
                    "static foreignId foreach entries must use literal column-to-table pairs"
                        .into(),
                );
            };
            let column = parse_php_static_string(key.trim()).ok_or_else(|| {
                "static foreignId foreach column must be a string literal".to_owned()
            })?;
            let target = parse_php_static_string(value.trim()).ok_or_else(|| {
                "static foreignId foreach target must be a string literal".to_owned()
            })?;
            if !is_identifier(&column) || !column.ends_with("_id") || !is_identifier(&target) {
                return Err("static foreignId foreach column and target must be identifiers; the column must end in `_id`".into());
            }
            if !seen_keys.insert(column.clone()) {
                return Err(format!(
                    "static foreignId foreach repeats column `{column}`"
                ));
            }
            let expanded_modifiers =
                constrained_variable.replace(modifiers, format!("->constrained('{target}')"));
            expansions.push(format!(
                "${table_variable}->foreignId('{column}'){};",
                expanded_modifiers
            ));
        }
        recognized_bytes[matched.range()].fill(true);
        foreach_ranges.push(matched.range());
        foreach_expansions.push(expansions.join("\n"));
    }

    for operation_capture in operation.captures_iter(body) {
        let matched = operation_capture.get(0).expect("table operation match");
        if foreach_ranges
            .iter()
            .any(|range| range.start <= matched.start() && matched.end() <= range.end)
        {
            continue;
        }
        let absolute_start = body_offset + matched.start();
        if !source_map.is_code(absolute_start) {
            continue;
        }
        if operation_capture
            .get(1)
            .expect("Blueprint variable")
            .as_str()
            != table_variable
        {
            continue;
        }
        recognized_starts.insert(matched.start(), ());
        recognized_bytes[matched.range()].fill(true);
        let method = operation_capture.get(2).expect("column method").as_str();
        let args = operation_capture.get(3).expect("column arguments").as_str();
        let modifiers = operation_capture.get(4).expect("column modifiers").as_str();
        if method == "index" {
            if !modifiers.is_empty() {
                return Err("index declarations cannot have chained modifiers".into());
            }
            let (columns, name) = parse_index_columns(args)?;
            indexes.push(columns);
            if let Some(name) = name {
                backend_review_notes.push(format!(
                    "Laravel custom index name `{name}` is not preserved by generated Zelyra schema; review migrations that later refer to this name."
                ));
            }
            continue;
        }
        if method == "unique" {
            if !modifiers.is_empty() {
                return Err("unique declarations cannot have chained modifiers".into());
            }
            let (columns, name) = parse_index_columns(args)?;
            uniques.push(columns);
            if let Some(name) = name {
                backend_review_notes.push(format!(
                    "Laravel custom unique-index name `{name}` is not preserved by generated Zelyra schema; review migrations that later refer to this name."
                ));
            }
            continue;
        }
        if method == "foreign" {
            let constraint = parse_foreign_constraint(args, modifiers)?;
            if !foreign_columns.insert(constraint.source_column.clone()) {
                return Err(format!(
                    "column `{}` has more than one foreign-key declaration",
                    constraint.source_column
                ));
            }
            foreign_constraints.push(constraint);
            continue;
        }
        if method == "timestamps" {
            if !args.trim().is_empty() || !modifiers.trim().is_empty() {
                return Err(
                    "`timestamps()` arguments and chained modifiers are not supported".into(),
                );
            }
            for source_name in ["created_at", "updated_at"] {
                push_column(
                    &mut columns,
                    &mut seen_columns,
                    Column {
                        name: source_name.into(),
                        source_name: source_name.into(),
                        laravel_method: "timestamps".into(),
                        ty: "Timestamp".into(),
                        length: None,
                        text_storage: None,
                        storage_name: None,
                        decimal_precision: None,
                        enum_values: None,
                        relation_table: None,
                        references_table: None,
                        unresolved_relation: None,
                        nullable: true,
                        primary_key: false,
                        auto_increment: false,
                        unique: false,
                        indexed: false,
                        default: None,
                        on_delete: None,
                        after: None,
                    },
                )?;
            }
            continue;
        }
        let (source_name, source_ty, length, decimal_precision, primary_id, enum_values) = if method
            == "enum"
        {
            let (name, values) = enum_signature(args)?;
            let max_length = values
                .iter()
                .map(|value| value.chars().count())
                .max()
                .unwrap_or(1);
            backend_review_notes.push(format!(
                    "Laravel enum `{table_name}.{name}` is stored as String({max_length}); its SQL allowed-value constraint is not preserved. A typed parse/storage adapter is generated, but callers must explicitly parse values read from the database and validate values before writes."
                ));
            (
                name,
                "String".to_owned(),
                Some(
                    u32::try_from(max_length)
                        .map_err(|_| "Laravel enum value is too long".to_owned())?,
                ),
                None,
                false,
                Some(values),
            )
        } else {
            let (name, ty, length, precision, primary) = column_signature(method, args)?;
            (name, ty, length, precision, primary, None)
        };
        let text_storage = match method {
            "text" => Some(TextStorage::Text),
            "mediumText" => Some(TextStorage::MediumText),
            "longText" => Some(TextStorage::LongText),
            _ => None,
        };
        let modifiers = parse_modifiers(
            modifiers,
            method == "decimal",
            matches!(method, "timestamp" | "dateTime"),
        )?;
        let ColumnModifiers {
            nullable: explicit_nullable,
            unique,
            indexed,
            primary_key,
            default,
            constrained_table,
            on_delete,
            after,
        } = modifiers;
        if let (Some(values), Some(default)) = (&enum_values, default.as_deref()) {
            let literal = default
                .strip_prefix("default ")
                .and_then(parse_php_static_string)
                .ok_or_else(|| "Laravel enum default must be a static string value".to_owned())?;
            if !values.contains(&literal) {
                return Err(format!(
                    "Laravel enum default `{literal}` is not in the declared allowed values"
                ));
            }
        }
        let nullable = explicit_nullable || method == "rememberToken";
        let (name, ty, relation_table, references_table) = if method == "foreignId"
            && constrained_table.is_some()
        {
            if let Some(source_field) = source_name.strip_suffix("_id") {
                let table = match constrained_table {
                    Some(table) if !table.is_empty() => table,
                    Some(_) => pluralize_table_stem(source_field),
                    None => unreachable!("constrained target checked above"),
                };
                if source_field.is_empty() || default.is_some() {
                    return Err(
                        "foreign-key columns need a nonempty relation field and no literal default"
                            .into(),
                    );
                }
                if on_delete == Some(ReferentialAction::SetNull) && !nullable {
                    return Err("`nullOnDelete` requires a nullable foreign-key column".into());
                }
                if on_delete == Some(ReferentialAction::SetDefault) {
                    return Err(
                        "`setDefaultOnDelete` is not supported across the configured database backends"
                            .into(),
                    );
                }
                if source_name == "tenant_id" && table == "tenants" {
                    (source_name.clone(), "TenantId".into(), None, Some(table))
                } else {
                    let relation_type = relation_type_for_table(&table)?;
                    (source_field.to_owned(), relation_type, Some(table), None)
                }
            } else {
                let target = constrained_table
                    .as_deref()
                    .expect("constrained table was checked above");
                let relation_type = relation_type_for_table(target)?;
                backend_review_notes.push(format!(
                    "Laravel foreign key `{table_name}.{source_name}` uses a nonstandard physical column name; the explicit `storage_name` mapping preserves it, while its target type and delete action are imported. Review backend width and signedness."
                ));
                (
                    source_name.clone(),
                    relation_type,
                    constrained_table.clone(),
                    None,
                )
            }
        } else if method == "foreignId" {
            if on_delete.is_some() {
                return Err(
                    "`foreignId` delete actions require a statically resolved `constrained` target"
                        .into(),
                );
            }
            backend_review_notes.push(format!(
                "Laravel `foreignId` column `{table_name}.{source_name}` has no `constrained()` call; it is emitted as an unsigned `UInt` field without a foreign-key relation. Review backend width and signedness."
            ));
            (source_name.clone(), "UInt".into(), None, None)
        } else {
            if constrained_table.is_some() || on_delete.is_some() {
                return Err(
                    "foreign-key modifiers are only supported on `foreignId` columns".into(),
                );
            }
            (source_name.clone(), source_ty, None, None)
        };
        if !is_identifier(&name) {
            return Err(format!(
                "column `{name}` is not a supported Zelyra identifier"
            ));
        }
        let primary_key = primary_key || primary_id;
        primary_key_count += usize::from(primary_key);
        let imported_delete_action = (relation_table.is_some() || references_table.is_some())
            .then_some(on_delete)
            .flatten();
        let storage_name = relation_table
            .as_ref()
            .filter(|_| !source_name.ends_with("_id"))
            .map(|_| source_name.clone());
        let column = Column {
            name,
            source_name,
            laravel_method: method.into(),
            ty,
            length,
            text_storage,
            storage_name,
            decimal_precision,
            enum_values,
            relation_table,
            references_table,
            unresolved_relation: None,
            nullable,
            primary_key,
            auto_increment: primary_id,
            unique,
            indexed,
            default,
            on_delete: imported_delete_action,
            after,
        };
        if column.indexed {
            indexes.push(vec![column.source_name.clone()]);
        }
        push_column(&mut columns, &mut seen_columns, column)?;
    }

    for expanded_source in foreach_expansions {
        let expanded_source_map = PhpSourceMap::new(&expanded_source);
        let (extra_columns, extra_indexes, extra_uniques, extra_notes, _) = parse_columns(
            &expanded_source,
            ParseColumnsOptions {
                body_offset: 0,
                source: &expanded_source,
                source_map: &expanded_source_map,
                table_variable,
                table_name,
                require_primary_key: false,
                allow_external_index_columns: false,
            },
        )?;
        for column in extra_columns {
            push_column(&mut columns, &mut seen_columns, column)?;
        }
        indexes.extend(extra_indexes);
        uniques.extend(extra_uniques);
        backend_review_notes.extend(extra_notes);
    }

    for call_match in call.captures_iter(body) {
        let matched = call_match.get(0).expect("Blueprint call match");
        let absolute_start = body_offset + matched.start();
        if source_map.is_code(absolute_start)
            && call_match.get(1).expect("Blueprint variable").as_str() == table_variable
            && !recognized_starts.contains_key(&matched.start())
            && !foreach_ranges
                .iter()
                .any(|range| range.contains(&matched.start()))
        {
            let method_start = matched.as_str().rfind("->").unwrap_or(0) + 2;
            let method = matched.as_str()[method_start..]
                .trim_start()
                .split('(')
                .next()
                .unwrap_or("unknown");
            return Err(format!(
                "Laravel Blueprint operation `{method}` is outside the supported schema-candidate subset"
            ));
        }
    }
    let whole_callback_if_missing = if !require_primary_key && guards.len() == 1 {
        let (range, _, _, guarded_column) = &guards[0];
        let only_guarded_code = body.bytes().enumerate().all(|(offset, byte)| {
            !source_map.is_code(body_offset + offset)
                || byte.is_ascii_whitespace()
                || byte == b';'
                || range.contains(&offset)
        });
        only_guarded_code.then(|| guarded_column.clone())
    } else {
        None
    };
    for (range, open_brace, guarded_table, guarded_column) in guards {
        if guarded_table != table_name {
            return Err(format!(
                "`hasColumn` guard names `{guarded_table}` inside a `{table_name}` schema callback"
            ));
        }
        if !columns
            .iter()
            .any(|column| column.source_name == guarded_column)
        {
            return Err(format!(
                "`hasColumn` guard for `{guarded_column}` does not contain a matching static column declaration"
            ));
        }
        let close_brace = range.end - 1;
        if (open_brace + 1..close_brace).any(|offset| {
            source_map.is_code(body_offset + offset)
                && !recognized_bytes[offset]
                && !body.as_bytes()[offset].is_ascii_whitespace()
        }) {
            return Err("`hasColumn` guard contains code outside the supported static Blueprint declarations".into());
        }
        recognized_bytes[range.start..=open_brace].fill(true);
        recognized_bytes[close_brace] = true;
        backend_review_notes.push(format!(
            "Laravel `hasColumn` guard for `{guarded_column}` is not evaluated; declarations in the guarded branch are included as candidates and must be reviewed against the target database."
        ));
    }
    if body.bytes().enumerate().any(|(offset, byte)| {
        source_map.is_code(body_offset + offset)
            && !recognized_bytes[offset]
            && !byte.is_ascii_whitespace()
            && byte != b';'
    }) {
        return Err(
            "schema callback contains statements outside the supported static column subset".into(),
        );
    }
    if columns.is_empty() && (require_primary_key || (indexes.is_empty() && uniques.is_empty())) {
        return Err("no supported Blueprint column declarations were found".into());
    }
    for constraint in foreign_constraints {
        if let Some(name) = constraint.constraint_name.as_deref() {
            backend_review_notes.push(format!(
                "Laravel foreign-key constraint name `{name}` is not preserved by generated Zelyra schema; review migrations that later refer to this name."
            ));
        }
        if constraint.referenced_column != "id" {
            return Err(format!(
                "foreign key `{}` references `{}`; only the statically declared `id` target is supported",
                constraint.source_column, constraint.referenced_column
            ));
        }
        let source_field = constraint
            .source_column
            .strip_suffix("_id")
            .filter(|field| !field.is_empty())
            .ok_or_else(|| {
                format!(
                    "foreign-key column `{}` must end in `_id` to preserve its database column",
                    constraint.source_column
                )
            })?;
        let column = columns
            .iter_mut()
            .find(|column| column.source_name == constraint.source_column)
            .ok_or_else(|| {
                format!(
                    "foreign key refers to `{}`, which is not a declared supported column",
                    constraint.source_column
                )
            })?;
        if column.ty != "UInt"
            || column.relation_table.is_some()
            || column.default.is_some()
            || column.primary_key
        {
            return Err(format!(
                "foreign key `{}` requires an unsigned integer storage column without a default or existing relation",
                constraint.source_column
            ));
        }
        if constraint.on_delete == Some(ReferentialAction::SetNull) && !column.nullable {
            return Err(format!(
                "`nullOnDelete` requires nullable column `{}`",
                constraint.source_column
            ));
        }
        if constraint.on_delete == Some(ReferentialAction::SetDefault) {
            return Err(
                "`setDefaultOnDelete` is not supported across the configured database backends"
                    .into(),
            );
        }
        column.name = source_field.to_owned();
        column.ty = relation_type_for_table(&constraint.target_table)?;
        column.relation_table = Some(constraint.target_table);
        column.on_delete = constraint.on_delete;
    }
    if require_primary_key && primary_key_count != 1 {
        return Err(
            "exactly one primary-key column is required by the supported mapping; composite or missing primary keys are unsupported".into(),
        );
    }
    for (kind, index) in indexes
        .iter()
        .map(|index| ("index", index))
        .chain(uniques.iter().map(|unique| ("unique", unique)))
    {
        for source_column in index {
            if !allow_external_index_columns
                && !columns
                    .iter()
                    .any(|column| column.source_name == *source_column)
            {
                return Err(format!(
                    "{kind} declaration refers to `{source_column}`, which is not a supported column in this table"
                ));
            }
        }
    }
    indexes.sort();
    indexes.dedup();
    uniques.sort();
    if uniques.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("duplicate unique declarations are unsupported".into());
    }
    let source_to_field = columns
        .iter()
        .map(|column| (column.source_name.as_str(), column.name.as_str()))
        .collect::<BTreeMap<_, _>>();
    let indexes = indexes
        .into_iter()
        .map(|index| {
            index
                .iter()
                .map(|source| {
                    source_to_field
                        .get(source.as_str())
                        .copied()
                        .unwrap_or(source)
                        .to_owned()
                })
                .collect()
        })
        .collect();
    let uniques = uniques
        .into_iter()
        .map(|unique| {
            unique
                .iter()
                .map(|source| {
                    source_to_field
                        .get(source.as_str())
                        .copied()
                        .unwrap_or(source)
                        .to_owned()
                })
                .collect()
        })
        .collect();
    Ok((
        columns,
        indexes,
        uniques,
        backend_review_notes,
        whole_callback_if_missing,
    ))
}

fn push_column(
    columns: &mut Vec<Column>,
    seen_columns: &mut BTreeMap<String, usize>,
    column: Column,
) -> Result<(), String> {
    if !is_identifier(&column.name) {
        return Err(format!(
            "column `{}` is not a supported Zelyra identifier",
            column.name
        ));
    }
    if *seen_columns.entry(column.name.clone()).or_default() > 0 {
        return Err(format!(
            "column `{}` is declared more than once",
            column.name
        ));
    }
    *seen_columns.entry(column.name.clone()).or_default() += 1;
    columns.push(column);
    Ok(())
}

fn parse_index_columns(args: &str) -> Result<(Vec<String>, Option<String>), String> {
    let arguments = split_php_arguments(args)?;
    if !(1..=2).contains(&arguments.len()) {
        return Err("index accepts a column list and one optional static name".into());
    }
    let column_expression = arguments[0].trim();
    let columns = if let Some(array) = column_expression
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        split_php_arguments(array)?
            .iter()
            .map(|column| parse_name_only(column))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        vec![parse_name_only(column_expression)?]
    };
    let name = arguments
        .get(1)
        .map(|argument| {
            parse_php_static_string(argument.trim())
                .ok_or_else(|| "index name must be a static string literal".to_owned())
        })
        .transpose()?;
    if columns.is_empty() || columns.iter().any(|column| !is_identifier(column)) {
        return Err("index requires one or more static column-name literals".into());
    }
    let unique = columns.iter().collect::<BTreeSet<_>>();
    if unique.len() != columns.len() {
        return Err("index contains a duplicate column name".into());
    }
    Ok((columns, name))
}

fn split_php_arguments(expression: &str) -> Result<Vec<&str>, String> {
    let bytes = expression.as_bytes();
    let mut arguments = Vec::new();
    let mut start = 0;
    let mut square_depth = 0usize;
    let mut round_depth = 0usize;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' => index = super::skip_quoted(bytes, index, bytes[index]),
            b'[' => {
                square_depth += 1;
                index += 1;
            }
            b']' => {
                let Some(depth) = square_depth.checked_sub(1) else {
                    return Err("index argument has an unmatched `]`".into());
                };
                square_depth = depth;
                index += 1;
            }
            b'(' => {
                round_depth += 1;
                index += 1;
            }
            b')' => {
                let Some(depth) = round_depth.checked_sub(1) else {
                    return Err("index argument has an unmatched `)`".into());
                };
                round_depth = depth;
                index += 1;
            }
            b',' if square_depth == 0 && round_depth == 0 => {
                arguments.push(expression[start..index].trim());
                start = index + 1;
                index += 1;
            }
            _ => index += 1,
        }
    }
    if square_depth != 0 || round_depth != 0 {
        return Err("index argument has unmatched delimiters".into());
    }
    arguments.push(expression[start..].trim());
    if arguments.iter().any(|argument| argument.is_empty()) {
        return Err("index argument contains an empty value".into());
    }
    Ok(arguments)
}

fn mapping_review_note(method: &str) -> &'static str {
    match method {
        "enum" => "Mapped to String(max allowed-value length) plus a typed enum adapter. The database allowed-value constraint is not preserved; callers must explicitly decode before using the typed value and validate before writing.",
        "uuid" => "Mapped to String(36); Zelyra does not enforce UUID syntax, and PostgreSQL UUID storage semantics may differ. Review backend DDL and validation before use.",
        "json" | "jsonb" => "Mapped to Json; verify the target backend's JSON storage, validation, and query semantics against the original Laravel schema.",
        "text" => "The String value uses each backend's native TEXT type; maximum value sizes differ by backend.",
        "mediumText" => "The String value uses Zelyra `text_storage medium`; MariaDB MEDIUMTEXT capacity is preserved. PostgreSQL and SQLite use their native TEXT type, whose actual size limit is backend-specific.",
        "longText" => "The String value uses Zelyra `text_storage long`; MariaDB LONGTEXT capacity is preserved. PostgreSQL and SQLite use their native TEXT type and have lower maximum value sizes than MariaDB LONGTEXT, so review data sizes before changing backends.",
        "rememberToken" => "Laravel defines remember_token as nullable VARCHAR(100); verify the target backend DDL.",
        "id" => "The generated field preserves Laravel's unsigned BIGINT storage on MariaDB while keeping Zelyra's nominal Id type. Other backends use NUMERIC(20,0); review their native range and constraints.",
        "integer" | "mediumInteger" | "smallInteger" | "tinyInteger" => {
            "Laravel integer width may be narrower than Zelyra Int; compare target-backend range and DDL."
        }
        "bigInteger" => "Compare signedness and target-backend DDL before treating the mapping as equivalent.",
        "unsignedBigInteger" | "unsignedInteger" | "unsignedMediumInteger" | "unsignedSmallInteger" | "unsignedTinyInteger" => "The generated UInt field preserves this Laravel unsigned width on MariaDB. PostgreSQL and SQLite use NUMERIC(20,0), so native width and range constraints differ; review before changing backends.",
        "timestamps" | "timestamp" | "dateTime" => "Mapped to Timestamp; compare precision, timezone, nullability, and Eloquent timestamp behavior against the target backend.",
        "decimal" => "Laravel precision and scale are carried to the generated Decimal field; review value-range and target-backend constraints.",
        "foreignId" => "The generated UInt or relation field preserves Laravel's unsigned BIGINT storage on MariaDB. PostgreSQL and SQLite use NUMERIC(20,0), so native width and range constraints differ; only an explicitly resolved constrained() target creates a relation.",
        _ if method.starts_with("raw SQL:") => "Mapped from a bounded static deployment SQL declaration; see the candidate backend_review_notes for width, collation, index-name, and deployment-order limitations.",
        _ => "Compiler validity does not prove target-backend storage equivalence; review the generated DDL.",
    }
}

fn column_signature(method: &str, args: &str) -> Result<ColumnSignature, String> {
    match method {
        "id" if args.trim().is_empty() => Ok(("id".into(), "Id".into(), None, None, true)),
        "foreignId" => Ok((parse_name_only(args)?, "UInt".into(), None, None, false)),
        "string" => {
            let (name, length) = parse_name_and_optional_length(args, 255)?;
            Ok((name, "String".into(), Some(length), None, false))
        }
        "rememberToken" if args.trim().is_empty() => Ok((
            "remember_token".into(),
            "String".into(),
            Some(100),
            None,
            false,
        )),
        "uuid" => Ok((parse_name_only(args)?, "String".into(), Some(36), None, false)),
        "text" | "mediumText" | "longText" => {
            Ok((parse_name_only(args)?, "String".into(), None, None, false))
        }
        "json" | "jsonb" => Ok((parse_name_only(args)?, "Json".into(), None, None, false)),
        "integer" | "bigInteger" | "mediumInteger" | "smallInteger" | "tinyInteger" => {
            Ok((parse_name_only(args)?, "Int".into(), None, None, false))
        }
        "boolean" => Ok((parse_name_only(args)?, "Bool".into(), None, None, false)),
        "date" => Ok((parse_name_only(args)?, "Date".into(), None, None, false)),
        "time" if args.split(',').count() == 1 => {
            Ok((parse_name_only(args)?, "Time".into(), None, None, false))
        }
        "dateTime" | "timestamp" => Ok((parse_name_only(args)?, "Timestamp".into(), None, None, false)),
        "unsignedBigInteger" | "unsignedInteger" | "unsignedMediumInteger" | "unsignedSmallInteger" | "unsignedTinyInteger" => Ok((parse_name_only(args)?, "UInt".into(), None, None, false)),
        "decimal" => {
            let (name, precision) = parse_decimal_signature(args)?;
            Ok((name, "Decimal".into(), None, Some(precision), false))
        }
        "enum" => Err("enum columns require a static literal value list".into()),
        "float" | "double" => Err(format!(
            "Laravel Blueprint operation `{method}` uses binary floating-point storage and cannot be represented by Zelyra Decimal without changing numeric semantics"
        )),
        other => Err(format!(
            "Laravel Blueprint operation `{other}` is outside the supported schema-candidate subset"
        )),
    }
}

fn enum_signature(args: &str) -> Result<(String, Vec<String>), String> {
    let arguments = split_php_arguments(args)?;
    if arguments.len() != 2 {
        return Err("Laravel enum expects a column name and a static string array".into());
    }
    let name = parse_name_only(arguments[0])?;
    let list = arguments[1].trim();
    let Some(list) = list
        .strip_prefix('[')
        .and_then(|list| list.strip_suffix(']'))
    else {
        return Err("Laravel enum values must use a static array literal".into());
    };
    let values = split_php_arguments(list)?
        .into_iter()
        .map(|value| {
            parse_php_static_string(value).ok_or_else(|| {
                "Laravel enum values must be plain static string literals without escapes or interpolation".to_owned()
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() > 64 {
        return Err("Laravel enum has more than 64 values; review and model it manually".into());
    }
    let mut unique_values = BTreeSet::new();
    if values.is_empty() || values.iter().any(|value| !unique_values.insert(value)) {
        return Err("Laravel enum must contain a non-empty list of unique string values".into());
    }
    Ok((name, values))
}

fn enum_variant_names(values: &[String]) -> Vec<String> {
    let bases = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            if !value.is_ascii() {
                return format!("Value{index}");
            }
            let mut name = String::from("Value");
            let mut capitalize = true;
            let mut has_character = false;
            for character in value.chars() {
                if character.is_ascii_alphanumeric() {
                    if capitalize {
                        name.push(character.to_ascii_uppercase());
                    } else {
                        name.push(character);
                    }
                    capitalize = false;
                    has_character = true;
                } else {
                    capitalize = true;
                }
            }
            if has_character {
                name
            } else {
                format!("Value{index}")
            }
        })
        .collect::<Vec<_>>();
    let mut counts = HashMap::new();
    for base in &bases {
        *counts.entry(base.clone()).or_insert(0usize) += 1;
    }
    bases
        .into_iter()
        .enumerate()
        .map(|(index, base)| {
            if counts.get(&base).copied().unwrap_or_default() > 1 {
                format!("{base}_{index}")
            } else {
                base
            }
        })
        .collect()
}

fn render_enum_adapter(
    table: &str,
    column: &str,
    values: &[String],
) -> (String, String, String, String, Vec<(String, String)>) {
    let stem = format!("{table}_{column}");
    let type_name = format!("LaravelEnum_{stem}");
    let parse_function = format!("laravel_enum_{stem}_from_storage");
    let storage_function = format!("laravel_enum_{stem}_to_storage");
    let variant_names = enum_variant_names(values);
    let variants = values
        .iter()
        .enumerate()
        .map(|(index, value)| (variant_names[index].clone(), value.clone()))
        .collect::<Vec<_>>();
    let mut source = format!(
        "# Values mirror a static Laravel enum declaration. The SQL column remains String; callers must parse untrusted stored values.\npub enum {type_name} {{\n{}\n}}\n\npub fn {parse_function}(value: String) -> Option<{type_name}> {{\n    return match value {{\n",
        variants
            .iter()
            .map(|(variant, _)| format!("    {variant}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    for (variant, value) in &variants {
        source.push_str(&format!(
            "        {} => Some({type_name}::{variant}),\n",
            zelyra_string_literal(value)
        ));
    }
    source.push_str(&format!(
        "        _ => None\n    }}\n}}\n\npub fn {storage_function}(value: {type_name}) -> String {{\n    return match value {{\n"
    ));
    for (variant, value) in &variants {
        source.push_str(&format!(
            "        {type_name}::{variant} => {},\n",
            zelyra_string_literal(value)
        ));
    }
    source.push_str("    }\n}");
    (
        type_name,
        parse_function,
        storage_function,
        source,
        variants,
    )
}

fn zelyra_string_literal(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{escaped}\"")
}

fn relation_type_for_table(table: &str) -> Result<String, String> {
    if !is_identifier(table) {
        return Err("foreign-key target table is not a supported static identifier".into());
    }
    let singular = if let Some(stem) = table.strip_suffix("ies") {
        format!("{stem}y")
    } else if let Some(stem) = table.strip_suffix('s') {
        stem.to_owned()
    } else {
        return Err(format!(
            "cannot infer a Zelyra relation type from foreign-key target `{table}`"
        ));
    };
    if pluralize_table_stem(&singular) != table {
        return Err(format!(
            "foreign-key target `{table}` uses an unsupported plural form; declare the relation manually"
        ));
    }
    Ok(singular
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join("_"))
}

fn pluralize_table_stem(stem: &str) -> String {
    if let Some(root) = stem.strip_suffix('y') {
        format!("{root}ies")
    } else {
        format!("{stem}s")
    }
}

fn parse_name_only(args: &str) -> Result<String, String> {
    let name = parse_php_static_string(args.trim())
        .ok_or_else(|| "column name is not a simple static string literal".to_owned())?;
    Ok(name)
}

fn parse_name_and_optional_length(
    args: &str,
    default_length: u32,
) -> Result<(String, u32), String> {
    let mut parts = args.splitn(2, ',');
    let name = parse_name_only(parts.next().unwrap_or_default())?;
    let length = match parts.next() {
        Some(length) => length.trim().parse::<u32>().ok(),
        None => Some(default_length),
    }
    .filter(|length| *length > 0 && *length <= 65_535)
    .ok_or_else(|| "string column length is not a supported positive integer".to_owned())?;
    Ok((name, length))
}

fn parse_decimal_signature(args: &str) -> Result<(String, (u32, u32)), String> {
    let mut parts = args.split(',');
    let name = parse_name_only(parts.next().unwrap_or_default())?;
    let mut precision = 8;
    let mut scale = 2;
    let mut positional = 0;
    let mut precision_seen = false;
    let mut scale_seen = false;
    for part in parts {
        let part = part.trim();
        let (key, value) = part.split_once(':').map_or((None, part), |(key, value)| {
            (Some(key.trim()), value.trim())
        });
        let parsed = value.parse::<u32>().map_err(|_| {
            "decimal precision and scale must be static integer literals".to_owned()
        })?;
        match key {
            Some("total" | "precision") if !precision_seen => {
                precision = parsed;
                precision_seen = true;
            }
            Some("places" | "scale") if !scale_seen => {
                scale = parsed;
                scale_seen = true;
            }
            Some(other) => {
                return Err(format!("unsupported Laravel decimal argument `{other}`"));
            }
            None if positional == 0 && !precision_seen => {
                precision = parsed;
                precision_seen = true;
                positional += 1;
            }
            None if positional <= 1 && !scale_seen => {
                scale = parsed;
                scale_seen = true;
                positional += 1;
            }
            None => return Err("too many or duplicate Laravel decimal arguments".into()),
        }
    }
    if !(1..=39).contains(&precision) || scale > 9 || scale >= precision {
        return Err(
            "Laravel decimal precision must fit Zelyra Decimal (precision 1–39, scale 0–9 below precision)".into(),
        );
    }
    Ok((name, (precision, scale)))
}

fn parse_php_static_string(value: &str) -> Option<String> {
    if value.len() < 2 {
        return None;
    }
    let bytes = value.as_bytes();
    let quote = *bytes.first()?;
    if !matches!(quote, b'\'' | b'"') || bytes.last().copied()? != quote {
        return None;
    }
    let inner = &value[1..value.len() - 1];
    if inner.contains('\\') || inner.contains('$') || inner.contains('\n') || inner.contains('\r') {
        return None;
    }
    Some(inner.to_owned())
}

fn parse_foreign_constraint(args: &str, modifiers: &str) -> Result<ForeignConstraint, String> {
    let arguments = split_php_arguments(args)?;
    if !(1..=2).contains(&arguments.len()) {
        return Err(
            "foreign declaration expects a column name and optional constraint name".into(),
        );
    }
    let source_column = parse_name_only(arguments[0])?;
    let constraint_name = arguments
        .get(1)
        .map(|argument| -> Result<String, String> {
            let name = parse_php_static_string(argument.trim()).ok_or_else(|| {
                "foreign-key constraint name must be a static string literal".to_owned()
            })?;
            if !is_identifier(&name) {
                return Err("foreign-key constraint name must be a simple identifier".to_owned());
            }
            Ok(name)
        })
        .transpose()?;
    let modifier = super::compiled_regex(
        r"->\s*([A-Za-z_]\w*)\s*\(([^()]*)\)",
        "Laravel foreign-key modifier regex",
    );
    let mut consumed = String::new();
    let mut referenced_column = None;
    let mut target_table = None;
    let mut on_delete = None;
    for capture in modifier.captures_iter(modifiers) {
        let matched = capture.get(0).expect("foreign-key modifier match");
        consumed.push_str(matched.as_str());
        let name = capture.get(1).expect("foreign-key modifier name").as_str();
        let value = capture.get(2).expect("foreign-key modifier value").as_str();
        match name {
            "references" if referenced_column.is_none() => {
                let column = parse_php_static_string(value.trim()).ok_or_else(|| {
                    "foreign-key reference must use a static column-name literal".to_owned()
                })?;
                if !is_identifier(&column) {
                    return Err(
                        "foreign-key referenced column is not a supported identifier".into(),
                    );
                }
                referenced_column = Some(column);
            }
            "on" if target_table.is_none() => {
                let table = parse_php_static_string(value.trim()).ok_or_else(|| {
                    "foreign-key target must use a static table-name literal".to_owned()
                })?;
                if !is_identifier(&table) {
                    return Err(
                        "foreign-key target table is not a supported static identifier".into(),
                    );
                }
                target_table = Some(table);
            }
            "cascadeOnDelete" if value.trim().is_empty() => {
                set_delete_action(&mut on_delete, ReferentialAction::Cascade)?
            }
            "restrictOnDelete" if value.trim().is_empty() => {
                set_delete_action(&mut on_delete, ReferentialAction::Restrict)?
            }
            "noActionOnDelete" if value.trim().is_empty() => {
                set_delete_action(&mut on_delete, ReferentialAction::NoAction)?
            }
            "nullOnDelete" if value.trim().is_empty() => {
                set_delete_action(&mut on_delete, ReferentialAction::SetNull)?
            }
            "setDefaultOnDelete" if value.trim().is_empty() => {
                set_delete_action(&mut on_delete, ReferentialAction::SetDefault)?
            }
            other => {
                return Err(format!(
                    "Laravel foreign-key modifier `{other}` is unsupported or duplicated"
                ))
            }
        }
    }
    if modifiers
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>()
        != consumed
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>()
    {
        return Err(
            "foreign-key modifier chain contains syntax outside the supported subset".into(),
        );
    }
    Ok(ForeignConstraint {
        source_column,
        constraint_name,
        target_table: target_table
            .ok_or_else(|| "foreign-key target must use a static `on('table')` call".to_owned())?,
        referenced_column: referenced_column.ok_or_else(|| {
            "foreign-key reference must use a static `references('id')` call".to_owned()
        })?,
        on_delete,
    })
}

fn parse_modifiers(
    modifiers: &str,
    allow_decimal_default: bool,
    allow_current_default: bool,
) -> Result<ColumnModifiers, String> {
    let modifier = super::compiled_regex(
        r"->\s*([A-Za-z_]\w*)\s*\(([^()]*)\)",
        "Laravel column modifier regex",
    );
    let mut consumed = String::new();
    let mut nullable = false;
    let mut unique = false;
    let mut indexed = false;
    let mut primary = false;
    let mut default = None;
    let mut constrained_table = None;
    let mut on_delete = None;
    let mut after = None;
    let mut current_seen = false;
    for capture in modifier.captures_iter(modifiers) {
        let matched = capture.get(0).expect("modifier match");
        consumed.push_str(matched.as_str());
        let name = capture.get(1).expect("modifier name").as_str();
        let args = capture.get(2).expect("modifier arguments").as_str().trim();
        match name {
            "nullable" if args.is_empty() => nullable = true,
            "unique" if args.is_empty() => unique = true,
            "index" if args.is_empty() => indexed = true,
            "primary" if args.is_empty() => primary = true,
            "default" => {
                if current_seen {
                    return Err(
                        "`useCurrent` cannot be combined with another default value".into(),
                    );
                }
                if default.is_some() {
                    return Err("a column may declare only one default value".into());
                }
                default = Some(render_default(args, allow_decimal_default)?);
            }
            "useCurrent" if args.is_empty() && allow_current_default => {
                if current_seen || default.is_some() {
                    return Err("`useCurrent` may be used only once and cannot be combined with another default".into());
                }
                current_seen = true;
                default = Some("default now".into());
            }
            "constrained" => {
                if constrained_table.is_some() {
                    return Err("`constrained` may be used only once on a foreign key".into());
                }
                constrained_table = Some(if args.is_empty() {
                    String::new()
                } else {
                    parse_php_static_string(args).ok_or_else(|| {
                        "foreign-key `constrained` target must be a static table-name literal".to_owned()
                    })?
                });
            }
            "cascadeOnDelete" if args.is_empty() => {
                set_delete_action(&mut on_delete, ReferentialAction::Cascade)?
            }
            "restrictOnDelete" if args.is_empty() => {
                set_delete_action(&mut on_delete, ReferentialAction::Restrict)?
            }
            "noActionOnDelete" if args.is_empty() => {
                set_delete_action(&mut on_delete, ReferentialAction::NoAction)?
            }
            "nullOnDelete" if args.is_empty() => {
                set_delete_action(&mut on_delete, ReferentialAction::SetNull)?
            }
            "setDefaultOnDelete" if args.is_empty() => {
                set_delete_action(&mut on_delete, ReferentialAction::SetDefault)?
            }
            "after" => {
                if after.is_some() {
                    return Err("`after` may be used only once on a column".into());
                }
                let name = parse_php_static_string(args)
                    .ok_or_else(|| "`after` requires a static column-name literal".to_owned())?;
                if !is_identifier(&name) {
                    return Err("`after` target is not a supported column identifier".into());
                }
                after = Some(name);
            }
            other => {
                return Err(format!(
                    "Laravel column modifier `{other}` is outside the supported schema-candidate subset"
                ))
            }
        }
    }
    if modifiers
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>()
        != consumed
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>()
    {
        return Err(
            "Laravel column modifier chain contains syntax outside the supported subset".into(),
        );
    }
    Ok(ColumnModifiers {
        nullable,
        unique,
        indexed,
        primary_key: primary,
        default,
        constrained_table,
        on_delete,
        after,
    })
}

fn order_columns_by_after(columns: &mut Vec<Column>) -> Result<(), String> {
    let positions = columns
        .iter()
        .enumerate()
        .map(|(index, column)| (column.source_name.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    if positions.len() != columns.len() {
        return Err("column ordering contains duplicate source column names".into());
    }
    let mut dependencies = vec![Vec::<usize>::new(); columns.len()];
    let mut incoming = vec![0usize; columns.len()];
    for (index, column) in columns.iter().enumerate() {
        let Some(anchor) = column.after.as_deref() else {
            continue;
        };
        let Some(&anchor_index) = positions.get(anchor) else {
            return Err(format!(
                "column `{}` uses `after('{anchor}')`, but that column is not present in the merged schema",
                column.source_name
            ));
        };
        if anchor_index == index {
            return Err(format!(
                "column `{}` cannot be ordered after itself",
                column.source_name
            ));
        }
        dependencies[anchor_index].push(index);
        incoming[index] += 1;
    }
    let mut ordered_indices = Vec::with_capacity(columns.len());
    let mut emitted = vec![false; columns.len()];
    while ordered_indices.len() < columns.len() {
        let Some(index) = (0..columns.len()).find(|&index| !emitted[index] && incoming[index] == 0)
        else {
            return Err("column `after(...)` constraints contain an ordering cycle".into());
        };
        emitted[index] = true;
        ordered_indices.push(index);
        for &dependent in &dependencies[index] {
            incoming[dependent] -= 1;
        }
    }
    let mut original = std::mem::take(columns)
        .into_iter()
        .map(Some)
        .collect::<Vec<_>>();
    *columns = ordered_indices
        .into_iter()
        .map(|index| original[index].take().expect("column index emitted once"))
        .collect();
    Ok(())
}

fn set_delete_action(
    action_slot: &mut Option<ReferentialAction>,
    action: ReferentialAction,
) -> Result<(), String> {
    if action_slot.replace(action).is_some() {
        return Err("only one foreign-key on-delete modifier is supported".into());
    }
    Ok(())
}

fn render_default(value: &str, allow_decimal: bool) -> Result<String, String> {
    if value == "true" || value == "false" || value.parse::<i64>().is_ok() {
        return Ok(format!("default {value}"));
    }
    if allow_decimal && is_decimal_literal(value) {
        return Ok(format!("default \"{value}\""));
    }
    let literal = parse_php_static_string(value).ok_or_else(|| {
        "column default is not a static string, integer, or boolean literal".to_owned()
    })?;
    let escaped = literal
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    Ok(format!("default \"{escaped}\""))
}

fn is_decimal_literal(value: &str) -> bool {
    let unsigned = value.strip_prefix('-').unwrap_or(value);
    let mut parts = unsigned.split('.');
    let integer = parts.next().unwrap_or_default();
    let fraction = parts.next();
    !integer.is_empty()
        && integer.bytes().all(|byte| byte.is_ascii_digit())
        && fraction.is_none_or(|fraction| {
            !fraction.is_empty() && fraction.bytes().all(|byte| byte.is_ascii_digit())
        })
        && parts.next().is_none()
}

fn text_storage_name(storage: TextStorage) -> &'static str {
    match storage {
        TextStorage::Text => "text",
        TextStorage::MediumText => "medium",
        TextStorage::LongText => "long",
    }
}

fn unsigned_storage_for_method(method: &str) -> Option<&'static str> {
    match method {
        "id" | "foreignId" | "unsignedBigInteger" => Some("bigint"),
        "unsignedInteger" => Some("int"),
        "unsignedMediumInteger" => Some("mediumint"),
        "unsignedSmallInteger" => Some("smallint"),
        "unsignedTinyInteger" => Some("tinyint"),
        _ => method
            .strip_prefix("raw SQL: ")
            .filter(|declaration| declaration.to_ascii_uppercase().contains("UNSIGNED"))
            .and_then(|declaration| {
                let base = declaration
                    .split(|character: char| !character.is_ascii_alphanumeric())
                    .next()?;
                match base.to_ascii_lowercase().as_str() {
                    "tinyint" => Some("tinyint"),
                    "smallint" => Some("smallint"),
                    "mediumint" => Some("mediumint"),
                    "int" | "integer" => Some("int"),
                    "bigint" => Some("bigint"),
                    _ => None,
                }
            }),
    }
}

fn render_table(candidate: &Candidate) -> String {
    let tenant_scope = candidate.columns.iter().find(|column| {
        column.name == "tenant_id"
            && !column.nullable
            && column.references_table.as_deref() == Some("tenants")
    });
    let header = if tenant_scope.is_some() {
        format!("table {} scoped by tenant_id: TenantId {{", candidate.table)
    } else {
        format!("table {} {{", candidate.table)
    };
    let mut lines = vec![header];
    for column in &candidate.columns {
        let mut ty = column.ty.clone();
        if column.length.is_some() {
            ty.push_str(&format!("({})", column.length.unwrap_or_default()));
        } else if let Some((precision, scale)) = column.decimal_precision {
            ty.push_str(&format!("({precision}, {scale})"));
        }
        if column.nullable {
            ty.push('?');
        }
        let mut attributes = Vec::new();
        if column.primary_key {
            attributes.push("primary".to_owned());
        }
        if column.auto_increment {
            attributes.push("auto".to_owned());
        } else if !column.nullable && !column.primary_key {
            attributes.push("required".into());
        }
        if column.unique {
            attributes.push("unique".into());
        }
        if let Some(storage_name) = &column.storage_name {
            attributes.push(format!("storage_name {storage_name:?}"));
        }
        if let Some(text_storage) = column.text_storage {
            attributes.push(format!("text_storage {}", text_storage_name(text_storage)));
        }
        if let Some(unsigned_storage) = unsigned_storage_for_method(&column.laravel_method) {
            attributes.push(format!("unsigned_storage {unsigned_storage}"));
        }
        if let Some(default) = &column.default {
            attributes.push(default.clone());
        }
        if let Some(action) = column
            .on_delete
            .filter(|_| column.relation_table.is_some() || column.references_table.is_some())
        {
            if action != ReferentialAction::NoAction {
                attributes.push(format!("on_delete {action}"));
            }
        }
        if let Some(target) = &column.references_table {
            attributes.push(format!("references {target}"));
        }
        let suffix = if attributes.is_empty() {
            String::new()
        } else {
            format!(" {}", attributes.join(" "))
        };
        lines.push(format!("    {}: {ty}{suffix}", column.name));
    }
    for index in &candidate.indexes {
        lines.push("\n    index {".into());
        lines.extend(index.iter().map(|column| format!("        {column}")));
        lines.push("    }".into());
    }
    for unique in &candidate.uniques {
        lines.push("\n    unique {".into());
        lines.extend(unique.iter().map(|column| format!("        {column}")));
        lines.push("    }".into());
    }
    lines.push("}".into());
    lines.join("\n")
}

fn is_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn span_json(span: SourceSpan) -> Value {
    json!({
        "start": {
            "byte_offset": span.start_byte,
            "line": span.start_line,
            "column": span.start_column
        },
        "end": {
            "byte_offset": span.end_byte,
            "line": span.end_line,
            "column": span.end_column
        },
        "end_exclusive": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_foreign_ids_generate_scoped_schema_with_scalar_references() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_tenants.php",
            "<?php Schema::create('tenants', function ($table) { $table->id(); $table->string('name'); });",
        );
        analyzer.scan(
            "database/migrations/create_customers.php",
            "<?php Schema::create('customer_groups', function ($table) { $table->id(); $table->foreignId('tenant_id')->constrained('tenants')->cascadeOnDelete(); $table->string('name'); });",
        );
        analyzer.scan(
            "database/migrations/create_customer_records.php",
            "<?php Schema::create('customers', function ($table) { $table->id(); $table->foreignId('tenant_id')->constrained('tenants')->cascadeOnDelete(); $table->foreignId('customer_group_id')->nullable()->constrained('customer_groups')->nullOnDelete(); $table->string('name'); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 3, "{plan}");
        let source = plan["candidate_source"].as_str().unwrap();
        assert!(source.contains("type TenantId = Id"), "{source}");
        assert!(
            source.contains("table customers scoped by tenant_id: TenantId {"),
            "{source}"
        );
        assert!(
            source.contains("tenant_id: TenantId required unsigned_storage bigint on_delete cascade references tenants"),
            "{source}"
        );
        assert!(
            source.contains(
                "customer_group: Customer_Group? unsigned_storage bigint on_delete restrict"
            ),
            "{source}"
        );
        let column = plan["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["table"] == "customers")
            .unwrap()["columns"]
            .as_array()
            .unwrap()
            .iter()
            .find(|column| column["source_column_name"] == "tenant_id")
            .unwrap();
        assert_eq!(column["zelyra_type"], "TenantId");
        assert_eq!(column["references_table"], "tenants");
        assert_eq!(column["relation_status"], "scalar_id_reference");
        let customers = plan["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["table"] == "customers")
            .unwrap();
        assert!(customers["backend_review_notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note.as_str().unwrap().contains("candidate uses `RESTRICT`")));
    }

    #[test]
    fn only_proven_scoped_tables_restrict_tenant_composite_set_null_relations() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_tenants.php",
            "<?php Schema::create('tenants', function ($table) { $table->id(); });",
        );
        analyzer.scan(
            "database/migrations/create_customers.php",
            "<?php Schema::create('customers', function ($table) { $table->id(); $table->foreignId('tenant_id')->constrained('tenants')->cascadeOnDelete(); $table->string('name'); });",
        );
        analyzer.scan(
            "database/migrations/create_orders.php",
            "<?php Schema::create('orders', function ($table) { $table->id(); $table->unsignedBigInteger('tenant_id'); $table->foreignId('customer_id')->nullable()->constrained('customers')->nullOnDelete(); });",
        );
        analyzer.scan(
            "database/migrations/create_scoped_orders.php",
            "<?php Schema::create('scoped_orders', function ($table) { $table->id(); $table->foreignId('tenant_id')->constrained('tenants')->cascadeOnDelete(); $table->foreignId('customer_id')->nullable()->constrained('customers')->nullOnDelete(); });",
        );

        let plan = analyzer.finish();
        let candidate = |table: &str| {
            plan["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .find(|candidate| candidate["table"] == table)
                .unwrap()
        };
        let order_relation = candidate("orders")["columns"]
            .as_array()
            .unwrap()
            .iter()
            .find(|column| column["source_column_name"] == "customer_id")
            .unwrap();
        assert_eq!(order_relation["on_delete"], "set_null");
        assert!(!candidate("orders")["backend_review_notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note.as_str().unwrap().contains("tenant-safe composite")));

        let scoped_order_relation = candidate("scoped_orders")["columns"]
            .as_array()
            .unwrap()
            .iter()
            .find(|column| column["source_column_name"] == "customer_id")
            .unwrap();
        assert_eq!(scoped_order_relation["on_delete"], "restrict");
        assert!(candidate("scoped_orders")["backend_review_notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note.as_str().unwrap().contains("tenant-safe composite")));
    }

    #[test]
    fn generates_exact_string_integer_boolean_and_date_fields() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_products.php",
            "<?php Schema::create('products', function (Blueprint $table) {\n$table->id();\n$table->string('sku', 64)->unique();\n$table->integer('quantity')->default(0);\n$table->boolean('active')->default(true);\n$table->date('available_on')->nullable();\n});",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1);
        assert_eq!(
            plan["candidates"][0]["columns"][0]["laravel_blueprint_method"],
            "id"
        );
        assert!(plan["candidates"][0]["columns"][0]["mapping_review"]
            .as_str()
            .unwrap()
            .contains("unsigned"));
        assert!(plan["candidate_source"]
            .as_str()
            .unwrap()
            .contains("id: Id primary auto"));
        assert!(plan["candidate_source"]
            .as_str()
            .unwrap()
            .contains("sku: String(64) required unique"));
        assert!(plan["candidate_source"]
            .as_str()
            .unwrap()
            .contains("quantity: Int required default 0"));
        assert!(plan["candidate_source"]
            .as_str()
            .unwrap()
            .contains("active: Bool required default true"));
        assert!(plan["candidate_source"]
            .as_str()
            .unwrap()
            .contains("available_on: Date?"));
    }

    #[test]
    fn preserves_large_text_storage_capacity_in_generated_zelyra_columns() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_notes.php",
            "<?php Schema::create('notes', function ($table) { $table->id(); $table->mediumText('summary'); $table->longText('body')->nullable(); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        let summary = &plan["candidates"][0]["columns"][1];
        assert_eq!(summary["laravel_blueprint_method"], "mediumText");
        assert_eq!(summary["zelyra_type"], "String");
        assert_eq!(summary["text_storage"], "medium");
        assert!(summary["mapping_review"]
            .as_str()
            .unwrap()
            .contains("MEDIUMTEXT capacity is preserved"));
        let body = &plan["candidates"][0]["columns"][2];
        assert_eq!(body["laravel_blueprint_method"], "longText");
        assert_eq!(body["zelyra_type"], "String");
        assert_eq!(body["text_storage"], "long");
        assert_eq!(body["nullable"], true);
        assert!(body["mapping_review"]
            .as_str()
            .unwrap()
            .contains("LONGTEXT capacity is preserved"));
        assert!(plan["candidate_source"]
            .as_str()
            .unwrap()
            .contains("body: String? text_storage long"));
        assert_eq!(
            plan["semantic_parity"],
            "not_proven_requires_backend_review"
        );
    }

    #[test]
    fn maps_uuid_columns_to_reviewed_string_storage_without_claiming_uuid_validation() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_inventory_movements.php",
            "<?php Schema::create('inventory_movements', function ($table) { $table->id(); $table->uuid('request_token'); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        let column = plan["candidates"][0]["columns"]
            .as_array()
            .unwrap()
            .iter()
            .find(|column| column["name"] == "request_token")
            .unwrap();
        assert_eq!(column["zelyra_type"], "String");
        assert_eq!(column["length"], 36);
        assert!(column["mapping_review"]
            .as_str()
            .unwrap()
            .contains("does not enforce UUID syntax"));
        assert!(plan["candidate_source"]
            .as_str()
            .unwrap()
            .contains("request_token: String(36) required"));
    }

    #[test]
    fn expands_static_foreign_id_loops_and_keeps_missing_targets_unresolved() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_orders.php",
            "<?php Schema::create('customers', function ($table) { $table->id(); }); Schema::create('orders', function ($table) { $table->id(); foreach (['customer_id' => 'customers', 'supplier_id' => 'suppliers'] as $column => $target) $table->foreignId($column)->nullable()->constrained($target)->nullOnDelete(); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 2, "{plan}");
        let order = plan["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["table"] == "orders")
            .unwrap();
        let columns = order["columns"].as_array().unwrap();
        assert!(columns.iter().any(|column| {
            column["source_column_name"] == "customer_id"
                && column["name"] == "customer"
                && column["relation_table"] == "customers"
                && column["relation_status"] == "resolved"
        }));
        assert!(columns.iter().any(|column| {
            column["source_column_name"] == "supplier_id"
                && column["name"] == "supplier_id"
                && column["zelyra_type"] == "UInt"
                && column["relation_status"] == "target_unavailable_emitted_as_unsigned_id"
        }));
    }

    #[test]
    fn rejects_dynamic_foreign_id_loop_targets() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_orders.php",
            "<?php Schema::create('orders', function ($table) use ($targets) { $table->id(); foreach ($targets as $column => $target) $table->foreignId($column)->constrained($target); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 0, "{plan}");
        assert_eq!(plan["rejected"].as_array().unwrap().len(), 1, "{plan}");
        assert!(plan["rejected"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("static string literal"));
    }

    #[test]
    fn maps_static_enum_storage_to_strings_and_emits_explicit_typed_adapters() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_enum_data.php",
            "<?php Schema::create('enum_data', function ($table) { $table->id(); $table->enum('status', ['draft', 'paid'])->default('draft'); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["schema_version"], "7", "{plan}");
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        let column = &plan["candidates"][0]["columns"][1];
        assert_eq!(column["zelyra_type"], "String");
        assert_eq!(column["length"], 5);
        assert_eq!(column["allowed_values"], json!(["draft", "paid"]));
        assert_eq!(column["default"], "default \"draft\"");
        let adapters = plan["enum_adapters"].as_array().unwrap();
        assert_eq!(adapters.len(), 1);
        assert_eq!(adapters[0]["table"], "enum_data");
        assert_eq!(adapters[0]["storage_constraint_preserved"], false);
        let source = adapters[0]["candidate_source"].as_str().unwrap();
        assert!(source.contains("pub enum LaravelEnum_enum_data_status"));
        assert!(source.contains("\"draft\" => Some(LaravelEnum_enum_data_status::ValueDraft)"));
        assert!(source.contains("LaravelEnum_enum_data_status::ValuePaid => \"paid\""));
        assert_eq!(
            adapters[0]["variants"],
            json!([
                {"name": "ValueDraft", "storage_value": "draft"},
                {"name": "ValuePaid", "storage_value": "paid"}
            ])
        );
        assert!(plan["candidates"][0]["backend_review_notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note
                .as_str()
                .unwrap()
                .contains("SQL allowed-value constraint is not preserved")));
    }

    #[test]
    fn enum_variant_names_are_readable_and_collision_safe() {
        let values = [
            "draft".to_owned(),
            "in-progress".to_owned(),
            "in_progress".to_owned(),
            "已完成".to_owned(),
            "---".to_owned(),
            "2fa".to_owned(),
        ];
        assert_eq!(
            enum_variant_names(&values),
            [
                "ValueDraft",
                "ValueInProgress_1",
                "ValueInProgress_2",
                "Value3",
                "Value4",
                "Value2fa"
            ]
        );
    }

    #[test]
    fn rejects_dynamic_duplicate_and_invalid_enum_values_or_defaults() {
        for enum_column in [
            "$table->enum('status', $statuses);",
            "$table->enum('status', ['draft', 'draft']);",
            "$table->enum('status', ['draft'])->default('paid');",
            "$table->enum('status', ['draft', $dynamic]);",
            r#"$table->enum('status', ['do\\'not']);"#,
            r#"$table->enum('status', ["$dynamic"]);"#,
        ] {
            let mut analyzer = SchemaAnalyzer::default();
            analyzer.scan(
                "database/migrations/create_enum_data.php",
                &format!("<?php Schema::create('enum_data', function ($table) {{ $table->id(); {enum_column} }});")
            );
            let plan = analyzer.finish();
            assert_eq!(plan["generated_table_count"], 0, "{enum_column}: {plan}");
            assert_eq!(
                plan["rejected"].as_array().unwrap().len(),
                1,
                "{enum_column}: {plan}"
            );
        }

        let too_many_values = (0..65)
            .map(|index| format!("'state_{index}'"))
            .collect::<Vec<_>>()
            .join(", ");
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_enum_data.php",
            &format!("<?php Schema::create('enum_data', function ($table) {{ $table->id(); $table->enum('status', [{too_many_values}]); }});"),
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 0, "{plan}");
        assert!(plan["rejected"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("more than 64 values"));
    }

    #[test]
    fn maps_json_columns_to_the_native_zelyra_json_type() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_json_data.php",
            "<?php Schema::create('json_data', function ($table) { $table->id(); $table->json('payload')->nullable(); $table->jsonb('metadata'); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        let columns = plan["candidates"][0]["columns"].as_array().unwrap();
        assert_eq!(columns[1]["name"], "payload");
        assert_eq!(columns[1]["zelyra_type"], "Json");
        assert_eq!(columns[1]["nullable"], true);
        assert_eq!(columns[2]["name"], "metadata");
        assert_eq!(columns[2]["zelyra_type"], "Json");
        assert!(plan["candidate_source"]
            .as_str()
            .unwrap()
            .contains("payload: Json?"));
    }

    #[test]
    fn maps_laravel_remember_token_to_nullable_string_storage() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_users.php",
            "<?php Schema::create('users', function ($table) { $table->id(); $table->rememberToken(); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        let candidate = &plan["candidates"][0];
        let token = &candidate["columns"][1];
        assert_eq!(token["name"], "remember_token");
        assert_eq!(token["zelyra_type"], "String");
        assert_eq!(token["length"], 100);
        assert_eq!(token["nullable"], true);
        assert!(token["mapping_review"]
            .as_str()
            .unwrap()
            .contains("VARCHAR(100)"));
        assert!(plan["candidate_source"]
            .as_str()
            .unwrap()
            .contains("remember_token: String(100)?"));
    }

    #[test]
    fn merges_static_additive_schema_table_migrations_into_the_create_candidate() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/001_create_orders.php",
            "<?php Schema::create('orders', function ($table) { $table->id(); $table->string('number'); });",
        );
        analyzer.scan(
            "database/migrations/002_add_paid_at_to_orders.php",
            "<?php Schema::table('orders', function ($table) { $table->timestamp('paid_at')->nullable(); $table->index('paid_at'); $table->unique('number'); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        let candidate = &plan["candidates"][0];
        assert_eq!(candidate["source_migrations"].as_array().unwrap().len(), 1);
        assert!(candidate["columns"]
            .as_array()
            .unwrap()
            .iter()
            .any(|column| column["name"] == "paid_at" && column["nullable"] == true));
        assert!(plan["candidate_source"]
            .as_str()
            .unwrap()
            .contains("paid_at: Timestamp?"));
        assert!(candidate["indexes"]
            .as_array()
            .unwrap()
            .contains(&json!(["paid_at"])));
        assert!(candidate["uniques"]
            .as_array()
            .unwrap()
            .contains(&json!(["number"])));
    }

    #[test]
    fn merges_typed_void_schema_table_callbacks_with_static_column_guards() {
        let source = "<?php return new class extends Migration { public function up(): void { Schema::create('users', function ($table) { $table->id(); }); if (!Schema::hasColumn('users', 'is_demo')) { Schema::table('users', function (Blueprint $table): void { $table->boolean('is_demo')->default(false)->index(); }); } } public function down(): void { if (Schema::hasColumn('users', 'is_demo')) { Schema::table('users', function (Blueprint $table): void { $table->dropIndex(['is_demo']); $table->dropColumn('is_demo'); }); } } };";
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/2026_10_06_230000_add_is_demo_to_users_table.php",
            source,
        );
        let result = analyzer.finish();
        assert_eq!(result["generated_table_count"], 1, "{result}");
        let users = &result["candidates"][0];
        assert!(
            users["columns"].as_array().unwrap().iter().any(|column| {
                column["source_column_name"] == "is_demo"
                    && column["zelyra_type"] == "Bool"
                    && column["indexed"] == true
            }),
            "{result}"
        );
        assert!(result["candidate_source"]
            .as_str()
            .unwrap()
            .contains("is_demo: Bool required default false"));
        assert!(
            result["rejected"].as_array().unwrap().is_empty(),
            "{result}"
        );
    }

    #[test]
    fn rejects_schema_table_callbacks_with_unsupported_return_types() {
        let source = "<?php Schema::create('users', function ($table) { $table->id(); }); Schema::table('users', function (Blueprint $table): int { $table->boolean('is_demo'); });";
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan("database/migrations/add_is_demo.php", source);
        let result = analyzer.finish();
        assert_eq!(result["generated_table_count"], 0, "{result}");
        assert!(result["rejected"].as_array().unwrap().iter().any(|entry| {
            entry["reason"]
                .as_str()
                .unwrap()
                .contains("supported static function-callback grammar")
        }));
        assert!(result["candidates"].as_array().unwrap().is_empty());
    }

    #[test]
    fn merges_static_arrow_schema_table_callbacks_without_executing_php() {
        let source = "<?php return new class extends Migration { public function up(): void { Schema::create('csv_mailer_campaigns', function ($table) { $table->id(); }); Schema::table('csv_mailer_campaigns', fn (Blueprint $table) => $table->json('booking_match_rules')->nullable()); } };";
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan("app/Migrations/add_booking_match_rules.php", source);
        let result = analyzer.finish();
        let candidates = result["candidates"].as_array().unwrap();
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0]["columns"]
            .as_array()
            .unwrap()
            .iter()
            .any(|column| {
                column["source_column_name"] == "booking_match_rules"
                    && column["zelyra_type"] == "Json"
                    && column["nullable"] == true
            }));
        assert!(result["rejected"].as_array().unwrap().is_empty());
    }

    #[test]
    fn rejects_arrow_schema_table_callbacks_that_mutate_another_blueprint() {
        let source = "<?php return new class extends Migration { public function up(): void { Schema::create('customers', function ($table) { $table->id(); }); Schema::table('customers', fn (Blueprint $schema) => $table->string('extra')); } };";
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan("database/migrations/add_extra.php", source);
        let result = analyzer.finish();
        assert_eq!(result["generated_table_count"], 0, "{result}");
        assert!(result["rejected"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["reason"]
                .as_str()
                .unwrap()
                .contains("declared Blueprint parameter")));
    }

    #[test]
    fn ignores_down_migration_operations_when_merging_up_alterations() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/001_create_orders.php",
            "<?php return new class extends Migration { public function up(): void { Schema::create('orders', function ($table) { $table->id(); }); } public function down(): void { Schema::dropIfExists('orders'); } };",
        );
        analyzer.scan(
            "database/migrations/002_add_status.php",
            "<?php return new class extends Migration { public function up(): void { Schema::table('orders', function ($table) { $table->string('status'); }); } public function down(): void { Schema::table('orders', function ($table) { $table->dropColumn('status'); }); } };",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        assert!(plan["candidates"][0]["columns"]
            .as_array()
            .unwrap()
            .iter()
            .any(|column| column["source_column_name"] == "status"));
        assert!(plan["rejected"].as_array().unwrap().is_empty(), "{plan}");
        let rollback = plan["rollback_inventory"]["migrations"].as_array().unwrap();
        assert_eq!(rollback.len(), 2);
        assert_eq!(rollback[0]["down_method"], "present");
        let drop_operation = &rollback[0]["operations"][0];
        assert_eq!(drop_operation["facade"], "Schema");
        assert_eq!(drop_operation["method"], "dropIfExists");
        assert_eq!(drop_operation["literal_table"], "orders");
        assert!(drop_operation["source_span"]["start"]["byte_offset"]
            .as_u64()
            .is_some());
        assert_eq!(
            plan["rollback_inventory"]["semantics"],
            "static_down_method_inventory_only_not_rollback_equivalence_or_execution"
        );
    }

    #[test]
    fn records_missing_down_methods_without_inferring_rollback_behavior() {
        let source = r#"<?php
// Schema::dropIfExists('comment_decoy');
$note = "DB::statement('secret sql is not reported')";
return new class extends Migration {
    public function up(): void { Schema::create('events', function ($table) { $table->id(); }); }
};
"#;
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan("database/migrations/create_events.php", source);
        analyzer.scan(
            "database/migrations/drop_legacy.php",
            "<?php return new class extends Migration { public function down(): void { Schema::dropIfExists($table_name); DB::statement('DROP TABLE private_data'); } };",
        );
        let report = analyzer.finish();
        assert_eq!(
            report["rollback_inventory"]["migrations"][0]["down_method"],
            "missing"
        );
        assert_eq!(
            report["rollback_inventory"]["migrations"][0]["rollback_status"],
            "not_assessed_missing_down_method"
        );
        assert!(report["rollback_inventory"]["migrations"][0]["operations"]
            .as_array()
            .unwrap()
            .is_empty());
        let serialized = report.to_string();
        assert!(!serialized.contains("comment_decoy"));
        assert!(!serialized.contains("secret sql"));
        assert!(!serialized.contains("private_data"));
        let dynamic_rollback = &report["rollback_inventory"]["migrations"][1];
        assert_eq!(dynamic_rollback["down_method"], "present");
        assert_eq!(dynamic_rollback["operations"][0]["method"], "dropIfExists");
        assert!(dynamic_rollback["operations"][0]["literal_table"].is_null());
        assert_eq!(dynamic_rollback["operations"][1]["facade"], "DB");
        assert_eq!(dynamic_rollback["operations"][1]["method"], "statement");
    }

    #[test]
    fn respects_static_after_column_order_when_merging_alterations() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/001_create_orders.php",
            "<?php Schema::create('orders', function ($table) { $table->id(); $table->string('status'); });",
        );
        analyzer.scan(
            "database/migrations/002_add_notes.php",
            "<?php Schema::table('orders', function ($table) { $table->string('notes')->after('status'); $table->string('memo')->after('status'); });",
        );
        let plan = analyzer.finish();
        let names = plan["candidates"][0]["columns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|column| column["source_column_name"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(names, ["id", "status", "notes", "memo"], "{plan}");
    }

    #[test]
    fn rejects_after_columns_with_missing_anchors() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/001_create_orders.php",
            "<?php Schema::create('orders', function ($table) { $table->id(); });",
        );
        analyzer.scan(
            "database/migrations/002_add_notes.php",
            "<?php Schema::table('orders', function ($table) { $table->string('notes')->after('missing'); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 0, "{plan}");
        assert!(plan["rejected"].as_array().unwrap().iter().any(|item| {
            item["reason"]
                .as_str()
                .unwrap()
                .contains("not present in the merged schema")
        }));
    }

    #[test]
    fn rejects_after_column_ordering_cycles() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/001_create_orders.php",
            "<?php Schema::create('orders', function ($table) { $table->id(); });",
        );
        analyzer.scan(
            "database/migrations/002_add_order_fields.php",
            "<?php Schema::table('orders', function ($table) { $table->string('first')->after('second'); $table->string('second')->after('first'); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 0, "{plan}");
        assert!(plan["rejected"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| { item["reason"].as_str().unwrap().contains("ordering cycle") }));
    }

    #[test]
    fn rejects_schema_table_changes_that_cannot_be_merged_losslessly() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/001_create_orders.php",
            "<?php Schema::create('orders', function ($table) { $table->id(); $table->string('number'); });",
        );
        analyzer.scan(
            "database/migrations/002_change_orders.php",
            "<?php Schema::table('orders', function ($table) { $table->renameColumn('number', 'reference'); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 0, "{plan}");
        assert!(plan["rejected"].as_array().unwrap().iter().any(|rejected| {
            rejected["table"] == "orders"
                && rejected["reason"]
                    .as_str()
                    .unwrap()
                    .contains("outside the supported schema-candidate subset")
        }));
    }

    #[test]
    fn rejects_schema_table_migrations_without_a_unique_create_candidate() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/002_add_flag.php",
            "<?php Schema::table('orders', function ($table) { $table->boolean('ready'); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 0);
        assert!(plan["rejected"].as_array().unwrap().iter().any(|rejected| {
            rejected["table"] == "orders"
                && rejected["reason"]
                    .as_str()
                    .unwrap()
                    .contains("no unique supported Schema::create candidate")
        }));
    }

    #[test]
    fn excludes_schema_table_examples_in_comments_and_strings() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/001_create_orders.php",
            "<?php Schema::create('orders', function ($table) { $table->id(); }); // Schema::table('orders', function ($table) { $table->string('fake'); });\n$example = \"Schema::table('orders', function ($table) { $table->string('also_fake'); });\";",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1);
        assert_eq!(
            plan["candidates"][0]["source_migrations"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert!(plan["rejected"].as_array().unwrap().is_empty());
    }

    #[test]
    fn rejects_lossy_columns_without_emitting_a_partial_table() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_invoices.php",
            "<?php Schema::create('invoices', function ($table) { $table->id(); $table->decimal('total', 40, 2); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 0);
        assert_eq!(plan["rejected"][0]["table"], "invoices");
        assert!(plan["rejected"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("precision"));
        assert_eq!(plan["candidate_source"], "");
    }

    #[test]
    fn does_not_read_blueprint_calls_from_comments_or_strings() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/ignored.php",
            "<?php // Schema::create('fake', function ($table) { $table->id(); });\n$value = \"Schema::create('also_fake', function ($table) { $table->id(); });\";",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 0);
        assert_eq!(plan["rejected"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn ignores_schema_examples_outside_migration_directories() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "tests/Smoke/demo.php",
            "<?php Schema::create('customers', function ($table) { $table->id(); });",
        );
        analyzer.scan(
            "resources/views/example.blade.php",
            "<?php Schema::create('customers', function ($table) { $table->id(); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 0);
        assert!(plan["rejected"].as_array().unwrap().is_empty());
    }

    #[test]
    fn rejects_conditional_schema_declarations_instead_of_omitting_the_condition() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_conditional.php",
            "<?php Schema::create('conditional_records', function ($table) { if (feature_enabled()) { $table->id(); $table->string('label'); } });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 0);
        assert_eq!(plan["rejected"][0]["table"], "conditional_records");
        assert!(plan["rejected"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("outside the supported static column subset"));
    }

    #[test]
    fn rejects_a_valid_table_candidate_when_another_declaration_is_ambiguous() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/first.php",
            "<?php Schema::create('records', function ($table) { $table->id(); $table->string('label'); });",
        );
        analyzer.scan(
            "database/migrations/second.php",
            "<?php Schema::create('records', function ($table) { $table->id(); $table->decimal('amount', 10, 2); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 0);
        assert!(plan["candidate_source"].as_str().unwrap().is_empty());
        assert_eq!(plan["rejected"].as_array().unwrap().len(), 2);
        assert!(plan["rejected"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["reason"]
                .as_str()
                .unwrap()
                .contains("multiple Schema::create declarations")));
    }

    #[test]
    fn reports_static_create_calls_with_unsupported_callback_syntax() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_arrow.php",
            "<?php Schema::create('arrow_records', fn ($table) => $table->id());",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 0);
        assert_eq!(plan["rejected"][0]["table"], "arrow_records");
        assert!(plan["rejected"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("static function-callback grammar"));
    }

    #[test]
    fn generates_only_resolved_foreign_ids_and_preserves_delete_actions() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/relationships.php",
            "<?php\nSchema::create('customers', function ($table) { $table->id(); });\nSchema::create('templates', function ($table) { $table->id(); });\nSchema::create('invoices', function ($table) { $table->id(); $table->foreignId('customer_id')->constrained()->cascadeOnDelete(); $table->foreignId('template_id')->nullable()->constrained('templates')->nullOnDelete(); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 3);
        let source = plan["candidate_source"].as_str().unwrap();
        assert!(source
            .contains("customer: Customer required unsigned_storage bigint on_delete cascade"));
        assert!(source.contains("template: Template? unsigned_storage bigint on_delete set_null"));
        let invoice = plan["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["table"] == "invoices")
            .unwrap();
        assert_eq!(invoice["columns"][1]["source_column_name"], "customer_id");
        assert_eq!(invoice["columns"][1]["relation_table"], "customers");
        assert_eq!(invoice["columns"][2]["on_delete"], "set_null");
    }

    #[test]
    fn rejects_unresolved_or_nonnullable_foreign_id_candidates() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/relationships.php",
            "<?php\nSchema::create('invoices', function ($table) { $table->id(); $table->foreignId('customer_id')->constrained()->nullOnDelete(); });\n",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 0);
        assert!(plan["rejected"]
            .as_array()
            .unwrap()
            .iter()
            .any(|rejected| rejected["reason"]
                .as_str()
                .unwrap()
                .contains("nullable foreign-key column")));
    }

    #[test]
    fn rejects_conflicting_delete_action_chains() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/conflicting_action.php",
            "<?php Schema::create('customers', function ($table) { $table->id(); }); Schema::create('invoices', function ($table) { $table->id(); $table->foreignId('customer_id')->constrained()->cascadeOnDelete()->nullOnDelete(); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1);
        assert_eq!(plan["candidates"][0]["table"], "customers");
        assert!(plan["rejected"]
            .as_array()
            .unwrap()
            .iter()
            .any(|rejected| rejected["table"] == "invoices"
                && rejected["reason"]
                    .as_str()
                    .unwrap()
                    .contains("only one foreign-key on-delete modifier")));
    }

    #[test]
    fn generates_reviewed_timestamps_unsigned_values_and_static_indexes() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_events.php",
            "<?php Schema::create('events', function ($table) { $table->id(); $table->unsignedBigInteger('external_id'); $table->string('kind')->index(); $table->timestamps(); $table->index(['external_id', 'kind']); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        let candidate = &plan["candidates"][0];
        let columns = candidate["columns"].as_array().unwrap();
        assert_eq!(columns.len(), 5);
        assert_eq!(columns[1]["zelyra_type"], "UInt");
        assert!(columns[1]["mapping_review"]
            .as_str()
            .unwrap()
            .contains("PostgreSQL and SQLite use NUMERIC(20,0)"));
        assert_eq!(columns[3]["name"], "created_at");
        assert_eq!(columns[4]["name"], "updated_at");
        assert_eq!(columns[3]["zelyra_type"], "Timestamp");
        assert!(columns[3]["nullable"].as_bool().unwrap());
        assert_eq!(candidate["indexes"].as_array().unwrap().len(), 2);
        let source = plan["candidate_source"].as_str().unwrap();
        assert!(source.contains("external_id: UInt required unsigned_storage bigint"));
        assert!(source.contains("created_at: Timestamp?"));
        assert!(source.contains("index {\n        external_id\n        kind\n    }"));
        assert!(source.contains("index {\n        kind\n    }"));
    }

    #[test]
    fn maps_laravel_use_current_to_a_timestamp_default() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_failed_jobs.php",
            "<?php Schema::create('failed_jobs', function ($table) { $table->id(); $table->timestamp('failed_at')->useCurrent(); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        assert!(plan["candidate_source"]
            .as_str()
            .unwrap()
            .contains("failed_at: Timestamp required default now"));
    }

    #[test]
    fn rejects_use_current_on_non_temporal_columns_and_conflicting_defaults() {
        for (signature, message) in [
            ("string('name')->useCurrent()", "modifier `useCurrent`"),
            (
                "timestamp('failed_at')->useCurrent()->default('later')",
                "cannot be combined with another default",
            ),
        ] {
            let mut analyzer = SchemaAnalyzer::default();
            analyzer.scan(
                "database/migrations/create_failed_jobs.php",
                &format!("<?php Schema::create('failed_jobs', function ($table) {{ $table->id(); $table->{signature}; }});"),
            );
            let plan = analyzer.finish();
            assert_eq!(plan["generated_table_count"], 0, "{plan}");
            assert!(plan["rejected"][0]["reason"]
                .as_str()
                .unwrap()
                .contains(message));
        }
    }

    #[test]
    fn preserves_custom_index_names_as_explicit_review_notes() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/custom_indexes.php",
            "<?php Schema::create('events', function ($table) { $table->id(); $table->string('tenant_id'); $table->string('slug'); $table->string('status'); $table->unique(['tenant_id', 'slug'], 'events_tenant_slug_unique'); $table->index('status', 'events_status_index'); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1);
        let candidate = &plan["candidates"][0];
        assert_eq!(candidate["uniques"][0][0], "tenant_id");
        assert_eq!(candidate["uniques"][0][1], "slug");
        assert_eq!(candidate["indexes"][0][0], "status");
        let notes = candidate["backend_review_notes"].as_array().unwrap();
        assert_eq!(notes.len(), 2);
        assert!(notes[0]
            .as_str()
            .unwrap()
            .contains("events_tenant_slug_unique"));
        assert!(notes[1].as_str().unwrap().contains("events_status_index"));
        let source = plan["candidate_source"].as_str().unwrap();
        assert!(source.contains("unique {\n        tenant_id\n        slug\n    }"));
        assert!(source.contains("index {\n        status\n    }"));

        let mut dynamic = SchemaAnalyzer::default();
        dynamic.scan(
            "database/migrations/dynamic_index.php",
            "<?php Schema::create('dynamic_events', function ($table) { $table->id(); $table->string('status'); $table->index('status', $indexName); });",
        );
        let plan = dynamic.finish();
        assert_eq!(plan["generated_table_count"], 0);
        assert!(plan["rejected"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("index name must be a static string literal"));
    }

    #[test]
    fn accepts_blueprint_variable_aliases_and_marks_static_has_column_guards() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/guarded_columns.php",
            "<?php\nSchema::create('customers', function ($blueprint) { $blueprint->id(); $blueprint->string('email'); });\nSchema::table('customers', function (Blueprint $schema) { if (!Schema::hasColumn('customers', 'display_name')) { $schema->string('display_name')->nullable()->after('email'); } });\n",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1);
        let candidate = &plan["candidates"][0];
        let names = candidate["columns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|column| column["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(names, ["id", "email", "display_name"]);
        assert_eq!(candidate["columns"][2]["nullable"], true);
        assert_eq!(candidate["columns"][2]["after"], "email");
        assert!(candidate["backend_review_notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note
                .as_str()
                .unwrap()
                .contains("guard for `display_name` is not evaluated")));

        let mut mismatched_guard = SchemaAnalyzer::default();
        mismatched_guard.scan(
            "database/migrations/mismatched_guard.php",
            "<?php Schema::create('customers', function ($t) { $t->id(); }); Schema::table('customers', function ($t) { if (!Schema::hasColumn('orders', 'display_name')) { $t->string('display_name'); } });",
        );
        let plan = mismatched_guard.finish();
        assert_eq!(plan["generated_table_count"], 0);
        assert!(plan["rejected"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("names `orders` inside a `customers` schema callback"));

        let mut dynamic_guard = SchemaAnalyzer::default();
        dynamic_guard.scan(
            "database/migrations/dynamic_guard.php",
            "<?php Schema::create('customers', function ($t) { $t->id(); }); Schema::table('customers', function ($t) use ($column) { if (!Schema::hasColumn('customers', $column)) { $t->string($column); } });",
        );
        let plan = dynamic_guard.finish();
        assert_eq!(plan["generated_table_count"], 0);
        assert!(!plan["rejected"].as_array().unwrap().is_empty());
    }

    #[test]
    fn skips_a_wholly_guarded_alteration_when_an_earlier_migration_added_its_guard_column() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/001_add_signature.php",
            "<?php Schema::create('quotes', function ($table) { $table->id(); }); Schema::table('quotes', function ($table) { $table->text('signature_data')->nullable(); });",
        );
        analyzer.scan(
            "database/migrations/002_add_quote_signature_metadata.php",
            "<?php Schema::table('quotes', function ($table) { if (!Schema::hasColumn('quotes', 'signature_data')) { $table->longText('signature_data')->nullable(); $table->dateTime('signed_at')->nullable(); $table->string('signed_by_name')->nullable(); } });",
        );

        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        let quote = &plan["candidates"][0];
        let columns = quote["columns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|column| column["source_column_name"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(columns, ["id", "signature_data"]);
        assert!(plan["rejected"].as_array().unwrap().is_empty(), "{plan}");
        assert!(quote["backend_review_notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note
                .as_str()
                .unwrap()
                .contains("a previous migration statically declares the column")));
    }

    #[test]
    fn applies_a_wholly_guarded_alteration_when_its_guard_column_is_absent() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/001_create_quotes.php",
            "<?php Schema::create('quotes', function ($table) { $table->id(); });",
        );
        analyzer.scan(
            "database/migrations/002_add_quote_signature_metadata.php",
            "<?php Schema::table('quotes', function ($table) { if (!Schema::hasColumn('quotes', 'signature_data')) { $table->longText('signature_data')->nullable(); $table->dateTime('signed_at')->nullable(); } });",
        );

        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        let quote = &plan["candidates"][0];
        let columns = quote["columns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|column| column["source_column_name"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(columns, ["id", "signature_data", "signed_at"]);
        assert!(plan["rejected"].as_array().unwrap().is_empty(), "{plan}");
    }

    #[test]
    fn does_not_treat_a_mixed_guarded_and_unguarded_callback_as_a_whole_guard() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/001_add_signature.php",
            "<?php Schema::create('quotes', function ($table) { $table->id(); }); Schema::table('quotes', function ($table) { $table->text('signature_data')->nullable(); });",
        );
        analyzer.scan(
            "database/migrations/002_mixed_quote_changes.php",
            "<?php Schema::table('quotes', function ($table) { if (!Schema::hasColumn('quotes', 'signature_data')) { $table->longText('signature_data')->nullable(); $table->dateTime('signed_at')->nullable(); } $table->string('review_note')->nullable(); });",
        );

        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        assert_eq!(
            plan["candidates"][0]["columns"].as_array().unwrap().len(),
            2
        );
        assert!(plan["rejected"].as_array().unwrap().iter().any(|entry| {
            entry["reason"]
                .as_str()
                .is_some_and(|reason| reason.contains("duplicate column `signature_data`"))
        }));
    }

    #[test]
    fn marks_static_outer_has_column_guards_and_rejects_dynamic_ones() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/outer_guard.php",
            "<?php Schema::create('customers', function ($table) { $table->id(); }); if (Schema::hasTable('customers') && !Schema::hasColumn('customers', 'display_name')) { Schema::table('customers', function (Blueprint $schema) { $schema->string('display_name')->nullable(); }); }",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1);
        let candidate = &plan["candidates"][0];
        assert!(candidate["columns"]
            .as_array()
            .unwrap()
            .iter()
            .any(|column| column["name"] == "display_name"));
        assert!(candidate["backend_review_notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note
                .as_str()
                .unwrap()
                .contains("outer `hasColumn` guard for `display_name` is not evaluated")));

        let mut dynamic = SchemaAnalyzer::default();
        dynamic.scan(
            "database/migrations/dynamic_outer_guard.php",
            "<?php Schema::create('customers', function ($table) { $table->id(); }); if (!Schema::hasColumn('customers', $column)) { Schema::table('customers', function ($table) { $table->string('display_name'); }); }",
        );
        let plan = dynamic.finish();
        assert_eq!(plan["generated_table_count"], 0);
        assert!(plan["rejected"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["reason"]
                .as_str()
                .unwrap()
                .contains("dynamic or unsupported hasColumn guard")));
        assert!(plan["candidates"].as_array().unwrap().is_empty());
    }

    #[test]
    fn maps_an_indexed_foreign_storage_column_to_its_relation_field() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_invoices.php",
            "<?php Schema::create('customers', function ($table) { $table->id(); }); Schema::create('invoices', function ($table) { $table->id(); $table->foreignId('customer_id')->constrained()->index(); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 2, "{plan}");
        assert!(plan["candidate_source"]
            .as_str()
            .unwrap()
            .contains("index {\n        customer\n    }"));
    }

    #[test]
    fn maps_static_foreign_declarations_to_zelyra_relations() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_delivery_notes.php",
            "<?php Schema::create('customers', function ($table) { $table->id(); }); Schema::create('delivery_notes', function ($table) { $table->id(); $table->unsignedBigInteger('customer_id')->nullable()->index(); $table->foreign('customer_id')->references('id')->on('customers')->nullOnDelete(); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 2, "{plan}");
        let columns = plan["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["table"] == "delivery_notes")
            .unwrap()["columns"]
            .as_array()
            .unwrap()
            .clone();
        let customer = &columns[1];
        assert_eq!(customer["source_column_name"], "customer_id");
        assert_eq!(customer["name"], "customer");
        assert_eq!(customer["zelyra_type"], "Customer");
        assert_eq!(customer["relation_table"], "customers");
        assert_eq!(customer["on_delete"], "set_null");
        assert_eq!(customer["nullable"], true);
        let source = plan["candidate_source"].as_str().unwrap();
        assert!(source.contains("customer: Customer? unsigned_storage bigint on_delete set_null"));
        assert!(source.contains("index {\n        customer\n    }"));
    }

    #[test]
    fn maps_named_foreign_constraints_and_records_name_loss() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_invoices.php",
            "<?php Schema::create('customers', function ($table) { $table->id(); }); Schema::create('invoices', function ($table) { $table->id(); $table->unsignedBigInteger('customer_id'); $table->foreign('customer_id', 'invoices_customer_fk')->references('id')->on('customers'); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 2, "{plan}");
        let invoice = plan["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["table"] == "invoices")
            .unwrap();
        assert!(invoice["columns"]
            .as_array()
            .unwrap()
            .iter()
            .any(|column| column["name"] == "customer" && column["relation_table"] == "customers"));
        assert!(invoice["backend_review_notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note
                .as_str()
                .unwrap()
                .contains("constraint name `invoices_customer_fk` is not preserved")));
    }

    #[test]
    fn maps_relation_column_indexes_added_by_later_migrations() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_orders.php",
            "<?php Schema::create('customers', function ($table) { $table->id(); }); Schema::create('orders', function ($table) { $table->id(); $table->foreignId('customer_id')->constrained('customers'); $table->string('state'); });",
        );
        analyzer.scan(
            "database/migrations/add_order_indexes.php",
            "<?php Schema::table('orders', function ($table) { $table->index(['customer_id', 'state'], 'orders_customer_state_idx'); $table->unique('customer_id', 'orders_customer_unique'); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 2, "{plan}");
        let orders = plan["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["table"] == "orders")
            .unwrap();
        assert!(orders["indexes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|index| index == &serde_json::json!(["customer", "state"])));
        assert!(orders["uniques"]
            .as_array()
            .unwrap()
            .iter()
            .any(|unique| unique == &serde_json::json!(["customer"])));
        let source = plan["candidate_source"].as_str().unwrap();
        assert!(source.contains("index {\n        customer\n        state\n    }"));
        assert!(source.contains("unique {\n        customer\n    }"));
        assert!(!source.contains("customer_id"));
    }

    #[test]
    fn skips_only_a_conflicting_schema_alteration_and_keeps_independent_changes() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/001_create_events.php",
            "<?php Schema::create('events', function ($table) { $table->id(); });",
        );
        analyzer.scan(
            "database/migrations/002_add_label.php",
            "<?php Schema::table('events', function ($table) { $table->string('label'); });",
        );
        analyzer.scan(
            "database/migrations/003_conflicting_label.php",
            "<?php Schema::table('events', function ($table) { $table->text('label'); $table->string('ignored_with_conflict'); });",
        );
        analyzer.scan(
            "database/migrations/004_add_state.php",
            "<?php Schema::table('events', function ($table) { $table->string('state'); });",
        );

        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        let columns = plan["candidates"][0]["columns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|column| column["source_column_name"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(columns, ["id", "label", "state"]);
        let rejected = plan["rejected"].as_array().unwrap();
        assert_eq!(rejected.len(), 1, "{plan}");
        assert_eq!(
            rejected[0]["source_file"],
            "database/migrations/003_conflicting_label.php"
        );
        assert!(rejected[0]["reason"]
            .as_str()
            .unwrap()
            .contains("no changes from this alteration were merged"));
        assert!(!plan["candidate_source"]
            .as_str()
            .unwrap()
            .contains("ignored_with_conflict"));
    }

    #[test]
    fn rejects_foreign_declarations_that_do_not_match_supported_columns() {
        for (fixture, expected) in [
            (
                "<?php Schema::create('customers', function ($table) { $table->id(); }); Schema::create('invoices', function ($table) { $table->id(); $table->unsignedBigInteger('customer_id'); $table->foreign('missing_id')->references('id')->on('customers'); });",
                "not a declared supported column",
            ),
            (
                "<?php Schema::create('customers', function ($table) { $table->id(); }); Schema::create('invoices', function ($table) { $table->id(); $table->unsignedBigInteger('customer_id'); $table->foreign('customer_id')->references('code')->on('customers'); });",
                "only the statically declared `id` target is supported",
            ),
            (
                "<?php Schema::create('customers', function ($table) { $table->id(); }); Schema::create('invoices', function ($table) { $table->id(); $table->unsignedBigInteger('customer_id'); $table->foreign('customer_id')->references('id')->on($target); });",
                "foreign-key target must use a static table-name literal",
            ),
            (
                "<?php Schema::create('customers', function ($table) { $table->id(); }); Schema::create('invoices', function ($table) { $table->id(); $table->unsignedBigInteger('customer_id'); $table->foreign('customer_id', $constraint)->references('id')->on('customers'); });",
                "constraint name must be a static string literal",
            ),
        ] {
            let mut analyzer = SchemaAnalyzer::default();
            analyzer.scan("database/migrations/create_invoices.php", fixture);
            let plan = analyzer.finish();
            assert_eq!(plan["generated_table_count"], 1, "{plan}");
            let rejected = plan["rejected"].as_array().unwrap();
            assert_eq!(rejected.len(), 1, "{plan}");
            assert!(rejected[0]["reason"].as_str().unwrap().contains(expected), "{plan}");
        }
    }

    #[test]
    fn maps_static_single_and_composite_unique_constraints() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_users.php",
            "<?php Schema::create('users', function ($table) { $table->id(); $table->string('email'); $table->string('tenant_code'); $table->unique('email'); $table->unique(['tenant_code', 'email']); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        assert_eq!(
            plan["candidates"][0]["uniques"].as_array().unwrap().len(),
            2
        );
        let source = plan["candidate_source"].as_str().unwrap();
        assert!(source.contains("unique {\n        email\n    }"));
        assert!(source.contains("unique {\n        tenant_code\n        email\n    }"));
    }

    #[test]
    fn retains_unresolved_foreign_ids_as_explicit_unsigned_storage_candidates() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_invoices.php",
            "<?php Schema::create('invoices', function ($table) { $table->id(); $table->foreignId('customer_id')->constrained('customers')->cascadeOnDelete()->index(); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        let column = &plan["candidates"][0]["columns"][1];
        assert_eq!(column["zelyra_type"], "UInt");
        assert_eq!(column["relation_table"], Value::Null);
        assert_eq!(column["unresolved_relation_target"], "customers");
        assert_eq!(
            column["relation_status"],
            "target_unavailable_emitted_as_unsigned_id"
        );
        let source = plan["candidate_source"].as_str().unwrap();
        assert!(source.contains("customer_id: UInt required unsigned_storage bigint"));
        assert!(source.contains("index {\n        customer_id\n    }"));
        assert!(!source.contains("on_delete cascade"));
    }

    #[test]
    fn rejects_unknown_indexes_and_composite_primary_keys_without_partial_output() {
        for (fixture, expected) in [
            (
                "<?php Schema::create('events', function ($table) { $table->id(); $table->index('missing'); });",
                "index declaration refers to `missing`",
            ),
            (
                "<?php Schema::create('events', function ($table) { $table->string('code')->primary(); $table->string('other')->primary(); });",
                "exactly one primary-key column",
            ),
        ] {
            let mut analyzer = SchemaAnalyzer::default();
            analyzer.scan("database/migrations/create_events.php", fixture);
            let plan = analyzer.finish();
            assert_eq!(plan["generated_table_count"], 0, "{plan}");
            assert!(plan["candidate_source"].as_str().unwrap().is_empty());
            assert!(plan["rejected"][0]["reason"]
                .as_str()
                .unwrap()
                .contains(expected));
        }
    }

    #[test]
    fn maps_a_single_non_auto_increment_string_primary_key() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/0001_create_cache.php",
            "<?php Schema::create('cache', function ($table) { $table->string('key')->primary(); $table->mediumText('value'); $table->bigInteger('expiration')->index(); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        let key = plan["candidates"][0]["columns"]
            .as_array()
            .unwrap()
            .iter()
            .find(|column| column["source_column_name"] == "key")
            .unwrap();
        assert_eq!(key["primary_key"], true);
        assert_eq!(key["auto_increment"], false);
        assert!(plan["candidate_source"]
            .as_str()
            .unwrap()
            .contains("key: String(255) primary"));
        assert!(!plan["candidate_source"]
            .as_str()
            .unwrap()
            .contains("key: String(255) primary auto"));
        assert!(plan["rejected"].as_array().unwrap().is_empty(), "{plan}");
    }

    #[test]
    fn preserves_unconstrained_foreign_id_as_unsigned_storage_without_relation() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/0001_create_sessions.php",
            "<?php Schema::create('sessions', function ($table) { $table->string('id')->primary(); $table->foreignId('user_id')->nullable()->index(); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        let session = &plan["candidates"][0];
        let user_id = session["columns"]
            .as_array()
            .unwrap()
            .iter()
            .find(|column| column["source_column_name"] == "user_id")
            .unwrap();
        assert_eq!(user_id["name"], "user_id");
        assert_eq!(user_id["zelyra_type"], "UInt");
        assert_eq!(user_id["nullable"], true);
        assert_eq!(user_id["relation_table"], Value::Null);
        assert!(session["backend_review_notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note
                .as_str()
                .unwrap()
                .contains("has no `constrained()` call")));

        let mut invalid_delete = SchemaAnalyzer::default();
        invalid_delete.scan(
            "database/migrations/invalid_sessions.php",
            "<?php Schema::create('sessions', function ($table) { $table->id(); $table->foreignId('user_id')->cascadeOnDelete(); });",
        );
        let rejected = invalid_delete.finish();
        assert_eq!(rejected["generated_table_count"], 0, "{rejected}");
        assert!(rejected["rejected"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("delete actions require a statically resolved `constrained` target"));
    }

    #[test]
    fn preserves_laravel_unsigned_integer_widths_in_source_and_candidate_metadata() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/0001_create_measurements.php",
            "<?php Schema::create('measurements', function ($table) { $table->id(); $table->unsignedTinyInteger('tiny'); $table->unsignedSmallInteger('small'); $table->unsignedMediumInteger('medium'); $table->unsignedInteger('normal'); $table->unsignedBigInteger('large'); $table->foreignId('owner_id'); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        let columns = plan["candidates"][0]["columns"].as_array().unwrap();
        for (name, expected) in [
            ("id", "bigint"),
            ("tiny", "tinyint"),
            ("small", "smallint"),
            ("medium", "mediumint"),
            ("normal", "int"),
            ("large", "bigint"),
            ("owner_id", "bigint"),
        ] {
            let column = columns
                .iter()
                .find(|column| column["source_column_name"] == name)
                .unwrap_or_else(|| panic!("missing candidate column {name}: {plan}"));
            assert_eq!(column["unsigned_storage"], expected, "{name}: {plan}");
        }
        let source = plan["candidate_source"].as_str().unwrap();
        for fragment in [
            "id: Id primary auto unsigned_storage bigint",
            "tiny: UInt required unsigned_storage tinyint",
            "small: UInt required unsigned_storage smallint",
            "medium: UInt required unsigned_storage mediumint",
            "normal: UInt required unsigned_storage int",
            "large: UInt required unsigned_storage bigint",
            "owner_id: UInt required unsigned_storage bigint",
        ] {
            assert!(
                source.contains(fragment),
                "missing {fragment} in:\n{source}"
            );
        }
        assert!(columns.iter().any(|column| {
            column["mapping_review"]
                .as_str()
                .is_some_and(|note| note.contains("PostgreSQL and SQLite use NUMERIC(20,0)"))
        }));
    }

    #[test]
    fn imports_custom_physical_foreign_id_relation_and_delete_action() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/create_users.php",
            "<?php Schema::create('users', function ($table) { $table->id(); });",
        );
        analyzer.scan(
            "database/migrations/create_vacation_requests.php",
            "<?php Schema::create('vacation_requests', function ($table) { $table->id(); $table->foreignId('approved_by')->nullable()->constrained('users')->nullOnDelete(); });",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 2, "{plan}");
        assert!(plan["rejected"].as_array().unwrap().is_empty(), "{plan}");
        let table = plan["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["table"] == "vacation_requests")
            .unwrap();
        let approver = table["columns"]
            .as_array()
            .unwrap()
            .iter()
            .find(|column| column["source_column_name"] == "approved_by")
            .unwrap();
        assert_eq!(approver["name"], "approved_by");
        assert_eq!(approver["zelyra_type"], "User");
        assert_eq!(approver["nullable"], true);
        assert_eq!(approver["storage_name"], "approved_by");
        assert_eq!(approver["relation_table"], "users");
        assert_eq!(approver["on_delete"], "set_null");
        assert!(table["backend_review_notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note
                .as_str()
                .unwrap()
                .contains("target type and delete action are imported")));
        let source = plan["candidate_source"].as_str().unwrap();
        assert!(
            source.contains("approved_by: User? storage_name \"approved_by\" unsigned_storage bigint on_delete set_null"),
            "{source}"
        );
        assert!(!source.contains("approved_by_id"), "{source}");
    }

    #[test]
    fn preserves_decimal_precision_scale_and_static_decimal_defaults() {
        for (signature, expected) in [
            (
                "decimal('gross_total', 12, 2)->default(0.00)",
                "gross_total: Decimal(12, 2) required default \"0.00\"",
            ),
            (
                "decimal('tax_rate', total: 5, places: 2)->default(19.00)",
                "tax_rate: Decimal(5, 2) required default \"19.00\"",
            ),
        ] {
            let mut analyzer = SchemaAnalyzer::default();
            analyzer.scan(
                "database/migrations/create_amounts.php",
                &format!("<?php Schema::create('amounts', function ($table) {{ $table->id(); $table->{signature}; }});"),
            );
            let plan = analyzer.finish();
            assert_eq!(plan["generated_table_count"], 1, "{plan}");
            assert!(plan["candidate_source"]
                .as_str()
                .unwrap()
                .contains(expected));
            let amount = &plan["candidates"][0]["columns"][1];
            assert_eq!(amount["zelyra_type"], "Decimal");
            assert_eq!(
                amount["decimal_precision"]["precision"],
                if signature.contains("tax_rate") {
                    5
                } else {
                    12
                }
            );
            assert_eq!(amount["decimal_precision"]["scale"], 2);
        }
    }

    #[test]
    fn imports_bounded_deployment_sql_schema_and_foreign_keys() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan(
            "database/migrations/0001_create_customers.php",
            "<?php Schema::create('customers', function ($table) { $table->id(); $table->string('name'); });",
        );
        analyzer.scan_deploy_sql(
            "deploy/sql/dispatch/001_dispatch.sql",
            "-- comment CREATE TABLE fake (id INT)\nCREATE TABLE IF NOT EXISTS dispatch_orders (\n id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,\n customer_id BIGINT UNSIGNED NULL,\n status VARCHAR(20) NOT NULL DEFAULT 'open',\n token CHAR(64) NOT NULL,\n UNIQUE KEY dispatch_token (token),\n KEY dispatch_status (status)\n) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;",
        );
        analyzer.scan_deploy_sql(
            "deploy/sql/dispatch/003_relations.sql",
            "ALTER TABLE dispatch_orders ADD CONSTRAINT dispatch_order_customer FOREIGN KEY (customer_id) REFERENCES customers(id) ON DELETE SET NULL;",
        );
        analyzer.scan_deploy_sql(
            "deploy/sql/dispatch/004_add_reference.sql",
            "ALTER TABLE dispatch_orders ADD COLUMN IF NOT EXISTS reference_date DATE NULL;",
        );

        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 2, "{plan}");
        assert_eq!(plan["rejected"].as_array().unwrap().len(), 0, "{plan}");
        let orders = plan["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["table"] == "dispatch_orders")
            .unwrap();
        assert_eq!(orders["source_kind"], "deployment_sql");
        let columns = orders["columns"].as_array().unwrap();
        let customer = columns
            .iter()
            .find(|column| column["source_column_name"] == "customer_id")
            .unwrap();
        assert_eq!(customer["name"], "customer");
        assert_eq!(customer["zelyra_type"], "Customer");
        assert_eq!(customer["relation_table"], "customers");
        assert_eq!(customer["nullable"], true);
        assert_eq!(customer["on_delete"], "set_null");
        assert!(columns
            .iter()
            .any(|column| column["source_column_name"] == "reference_date"));
        let generated = orders["candidate_source"].as_str().unwrap();
        assert!(
            generated.contains("customer: Customer? unsigned_storage bigint on_delete set_null")
        );
        assert!(generated.contains("reference_date: Date?"));
        assert!(generated.contains("unique {\n        token\n    }"));
        assert!(generated.contains("index {\n        status\n    }"));
        assert!(!generated.contains("fake"));
        assert!(orders["backend_review_notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note.as_str().unwrap().contains("fixed-width storage")));
        assert!(orders["source_migrations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["source_file"] == "deploy/sql/dispatch/003_relations.sql"));
        assert!(plan["external_sql_review_notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note.as_str().unwrap().contains("Storage engine, charset")));
    }

    #[test]
    fn unsupported_deployment_sql_alter_fails_the_table_closed() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan_deploy_sql(
            "deploy/sql/custom/001.sql",
            "CREATE TABLE records (id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY, value VARCHAR(20) NULL) ENGINE=InnoDB;",
        );
        analyzer.scan_deploy_sql(
            "deploy/sql/custom/002.sql",
            "ALTER TABLE records DROP COLUMN value;",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 0, "{plan}");
        assert!(plan["rejected"].as_array().unwrap().iter().any(|rejected| {
            rejected["table"] == "records"
                && rejected["reason"]
                    .as_str()
                    .unwrap()
                    .contains("outside the supported static subset")
        }));
    }

    #[test]
    fn mixed_deployment_sql_alter_does_not_keep_only_the_recognized_foreign_key() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan_deploy_sql(
            "deploy/sql/custom/001.sql",
            "CREATE TABLE records (id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY, owner_id BIGINT UNSIGNED NULL) ENGINE=InnoDB;",
        );
        analyzer.scan_deploy_sql(
            "deploy/sql/custom/002.sql",
            "ALTER TABLE records ADD CONSTRAINT fk_records_owner FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE SET NULL, DROP COLUMN stale;",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 0, "{plan}");
        assert!(plan["rejected"].as_array().unwrap().iter().any(|rejected| {
            rejected["table"] == "records"
                && rejected["reason"]
                    .as_str()
                    .unwrap()
                    .contains("contains a clause outside the supported static subset")
        }));
    }

    #[test]
    fn sensitive_deployment_sql_defaults_are_rejected_without_leaking_values() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan_deploy_sql(
            "deploy/sql/custom/secrets.sql",
            "CREATE TABLE credentials (id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY, api_secret VARCHAR(64) NOT NULL DEFAULT 'never-export-this-value') ENGINE=InnoDB;",
        );
        let plan = analyzer.finish();
        let serialized = serde_json::to_string(&plan).unwrap();
        assert_eq!(plan["generated_table_count"], 0, "{plan}");
        assert!(plan["rejected"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("sensitive column `api_secret`"));
        assert!(!serialized.contains("never-export-this-value"));
    }

    #[test]
    fn sql_trigger_and_engine_notes_ignore_comments_and_string_literals() {
        let mut analyzer = SchemaAnalyzer::default();
        analyzer.scan_deploy_sql(
            "deploy/sql/custom/001.sql",
            "-- CREATE TRIGGER fake; ENGINE=InnoDB\nCREATE TABLE notes (id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY, description TEXT DEFAULT 'CREATE TRIGGER ENGINE=InnoDB');",
        );
        let plan = analyzer.finish();
        assert_eq!(plan["generated_table_count"], 1, "{plan}");
        assert!(plan["rejected"].as_array().unwrap().is_empty(), "{plan}");
        assert!(
            plan["external_sql_review_notes"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{plan}"
        );
    }
}
