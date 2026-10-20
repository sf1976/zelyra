use super::*;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, HashMap},
    ops::Range,
    path::{Component, Path},
    sync::{Arc, Mutex, OnceLock},
};

mod authentication;
mod business;
mod console;
mod generation;
mod graph;
mod live_database;
mod service_helpers;
mod validation_forms;
mod view_links;

const MAX_SOURCE_FILES: usize = 10_000;
const MAX_SOURCE_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_TOTAL_SOURCE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_DIRECTORY_DEPTH: usize = 64;
const ANALYZER_VERSION: &str = "1.1.5";
const MAPPING_RULESET_VERSION: &str = "1.1.54";

fn compiled_regex(pattern: &'static str, description: &'static str) -> Arc<Regex> {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, Arc<Regex>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache
        .entry(pattern)
        .or_insert_with(|| Arc::new(Regex::new(pattern).expect(description)))
        .clone()
}

#[derive(Clone)]
struct Finding {
    kind: &'static str,
    status: &'static str,
    file: String,
    line: usize,
    source_span: Option<SourceSpan>,
    name: String,
    summary: String,
    next_step: &'static str,
}

struct ComposerMetadata {
    framework: Option<String>,
    relevant_dependencies: Vec<String>,
    php_constraint: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SourceSpan {
    start_byte: usize,
    end_byte: usize,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
}

impl SourceSpan {
    fn new(source: &str, range: Range<usize>) -> Self {
        let start = range.start.min(source.len());
        let end = range.end.max(start).min(source.len());
        let (start_line, start_column) = source_position(source, start);
        let (end_line, end_column) = source_position(source, end);
        Self {
            start_byte: start,
            end_byte: end,
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }
}

struct PhpSourceMap {
    code: Vec<bool>,
    line_starts: Vec<usize>,
}

impl PhpSourceMap {
    fn new(source: &str) -> Self {
        let bytes = source.as_bytes();
        let has_open_tag = source.contains("<?=")
            || bytes
                .windows(5)
                .any(|window| window.eq_ignore_ascii_case(b"<?php"));
        let mut code = vec![false; bytes.len()];
        let mut index = 0;
        let mut in_php = !has_open_tag;
        while index < bytes.len() {
            if !in_php {
                if bytes[index..].starts_with(b"<?=") {
                    in_php = true;
                    index += 3;
                } else if bytes[index..].len() >= 5
                    && bytes[index..index + 5].eq_ignore_ascii_case(b"<?php")
                {
                    in_php = true;
                    index += 5;
                } else {
                    index += 1;
                }
                continue;
            }
            if bytes[index..].starts_with(b"?>") {
                in_php = false;
                index += 2;
                continue;
            }
            if bytes[index..].starts_with(b"//")
                || (bytes[index] == b'#' && bytes.get(index + 1) != Some(&b'['))
            {
                index = skip_line(bytes, index);
                continue;
            }
            if bytes[index..].starts_with(b"/*") {
                index = skip_block_comment(bytes, index);
                continue;
            }
            if matches!(bytes[index], b'\'' | b'"' | b'`') {
                index = skip_quoted(bytes, index, bytes[index]);
                continue;
            }
            if bytes[index..].starts_with(b"<<<") {
                if let Some(end) = skip_heredoc(bytes, index) {
                    index = end;
                    continue;
                }
            }
            code[index] = true;
            index += 1;
        }
        let mut line_starts = vec![0];
        for (index, byte) in bytes.iter().enumerate() {
            if *byte == b'\n' {
                line_starts.push(index + 1);
            }
        }
        Self { code, line_starts }
    }

    fn is_code(&self, byte_offset: usize) -> bool {
        self.code.get(byte_offset).copied().unwrap_or(false)
    }

    fn position(&self, source: &str, byte_offset: usize) -> (usize, usize) {
        let offset = byte_offset.min(source.len());
        let line_index = self.line_starts.partition_point(|start| *start <= offset) - 1;
        let column = source[self.line_starts[line_index]..offset].chars().count() + 1;
        (line_index + 1, column)
    }

    fn span(&self, source: &str, range: Range<usize>) -> SourceSpan {
        let start = range.start.min(source.len());
        let end = range.end.max(start).min(source.len());
        let (start_line, start_column) = self.position(source, start);
        let (end_line, end_column) = self.position(source, end);
        SourceSpan {
            start_byte: start,
            end_byte: end,
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }
}

fn source_position(source: &str, byte_offset: usize) -> (usize, usize) {
    let offset = byte_offset.min(source.len());
    let line_start = source[..offset]
        .rfind('\n')
        .map_or(0, |index| index.saturating_add(1));
    let line = source[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    let column = source[line_start..offset].chars().count() + 1;
    (line, column)
}

fn skip_line(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(bytes.len(), |offset| start + offset + 1)
}

fn skip_block_comment(bytes: &[u8], start: usize) -> usize {
    bytes[start + 2..]
        .windows(2)
        .position(|window| window == b"*/")
        .map_or(bytes.len(), |offset| start + 2 + offset + 2)
}

fn skip_quoted(bytes: &[u8], start: usize, quote: u8) -> usize {
    let mut index = start + 1;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index = (index + 2).min(bytes.len());
        } else if bytes[index] == quote {
            return index + 1;
        } else {
            index += 1;
        }
    }
    bytes.len()
}

fn skip_heredoc(bytes: &[u8], start: usize) -> Option<usize> {
    let newline = bytes[start..].iter().position(|byte| *byte == b'\n')? + start;
    let mut marker = start + 3;
    while marker < newline && matches!(bytes[marker], b' ' | b'\t') {
        marker += 1;
    }
    let quote = matches!(bytes.get(marker), Some(b'\'' | b'"')).then(|| bytes[marker]);
    if quote.is_some() {
        marker += 1;
    }
    let label_start = marker;
    while marker < newline && (bytes[marker].is_ascii_alphanumeric() || bytes[marker] == b'_') {
        marker += 1;
    }
    if marker == label_start || quote.is_some_and(|quote| bytes.get(marker) != Some(&quote)) {
        return None;
    }
    let label = &bytes[label_start..marker];
    let mut line_start = newline + 1;
    while line_start < bytes.len() {
        let line_end = bytes[line_start..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(bytes.len(), |offset| line_start + offset);
        let mut token = line_start;
        while token < line_end && matches!(bytes[token], b' ' | b'\t') {
            token += 1;
        }
        if bytes[token..line_end].starts_with(label) {
            let after = token + label.len();
            if after == line_end
                || matches!(
                    bytes[after],
                    b';' | b',' | b')' | b']' | b' ' | b'\t' | b'\r'
                )
            {
                return Some(token);
            }
        }
        line_start = if line_end == bytes.len() {
            bytes.len()
        } else {
            line_end + 1
        };
    }
    Some(bytes.len())
}

#[derive(Clone, Copy, Debug)]
struct FindingCounts {
    total: usize,
    unresolved: usize,
    partial: usize,
}

#[derive(Debug)]
struct AnalysisOutput {
    report_path: PathBuf,
    counts: FindingCounts,
}

struct ReportDetails<'a> {
    source_files: &'a [String],
    graph: &'a serde_json::Value,
    console_analysis: &'a serde_json::Value,
    schema_generation: &'a serde_json::Value,
    business_generation: &'a serde_json::Value,
    service_generation: &'a serde_json::Value,
    view_links: &'a serde_json::Value,
    live_database: Option<&'a serde_json::Value>,
}

struct CommandOutput {
    report_path: Option<PathBuf>,
    scaffold_path: Option<PathBuf>,
    counts: FindingCounts,
    generated_schema_tables: usize,
    generated_business_functions: usize,
    generated_enum_adapters: usize,
    generated_validation_forms: usize,
    generated_service_helpers: usize,
    requested_tables: Vec<String>,
    included_tables: Vec<String>,
}

pub(super) fn command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    if arguments.next().as_deref() != Some("laravel") {
        eprintln!("Usage: zelyra import laravel <analyze|generate> <project> [--report <path>] [--output <target>] [--table <name>]... [--format markdown|json] [--inspect-database]");
        return ExitCode::from(2);
    }
    let Some(operation) = arguments.next() else {
        eprintln!("error[E-IMPORT-001]: choose `analyze` or `generate`");
        return ExitCode::from(2);
    };
    if !matches!(operation.as_str(), "analyze" | "generate") {
        eprintln!("error[E-IMPORT-001]: unknown Laravel import operation `{operation}`; choose `analyze` or `generate`");
        return ExitCode::from(2);
    }
    let Some(project) = arguments.next() else {
        eprintln!("error[E-IMPORT-001]: Laravel project directory is required");
        return ExitCode::from(2);
    };
    let mut report = None;
    let mut output = None;
    let mut format = None;
    let mut inspect_database = false;
    let mut requested_tables = Vec::new();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--report" => report = arguments.next(),
            "--output" => output = arguments.next(),
            "--table" => match arguments.next() {
                Some(table) => requested_tables.push(table),
                None => {
                    eprintln!("error[E-IMPORT-001]: `--table` requires a schema table name");
                    return ExitCode::from(2);
                }
            },
            "--format" => format = arguments.next(),
            "--inspect-database" => inspect_database = true,
            _ => {
                eprintln!("error[E-IMPORT-001]: unknown import option `{argument}`");
                return ExitCode::from(2);
            }
        }
    }
    if operation == "analyze" && report.is_none() {
        eprintln!("error[E-IMPORT-001]: `analyze` requires `--report <path>`");
        return ExitCode::from(2);
    }
    if operation == "analyze" && output.is_some() {
        eprintln!("error[E-IMPORT-001]: `--output` is valid only with `generate`");
        return ExitCode::from(2);
    }
    if operation == "generate" && output.is_none() {
        eprintln!("error[E-IMPORT-001]: `generate` requires `--output <new-directory>`");
        return ExitCode::from(2);
    }
    if operation == "analyze" && !requested_tables.is_empty() {
        eprintln!("error[E-IMPORT-001]: `--table` can only be used with `generate` and `--output`");
        return ExitCode::from(2);
    }
    let project = PathBuf::from(project);
    let live_database = if inspect_database {
        match live_database::inspect(&project) {
            Ok(schema) => Some(schema),
            Err(error) => Some(json!({
                "status": "unavailable",
                "error": error,
                "credentials_recorded": false,
                "application_rows_read": false,
                "tables": []
            })),
        }
    } else {
        None
    };
    let report = report.map(PathBuf::from);
    let output = output.map(PathBuf::from);
    let format = match format.as_deref() {
        None => match report
            .as_ref()
            .and_then(|path| path.extension())
            .and_then(|extension| extension.to_str())
        {
            Some("json") => "json",
            _ => "markdown",
        },
        Some("json") => "json",
        Some("markdown") | Some("md") => "markdown",
        Some(other) => {
            eprintln!("error[E-IMPORT-001]: unsupported report format `{other}`; choose `markdown` or `json`");
            return ExitCode::from(2);
        }
    };
    let result = if let Some(output) = output {
        generate_scaffold(
            &project,
            report.as_deref(),
            &output,
            format,
            &requested_tables,
            live_database.as_ref(),
        )
    } else {
        analyze_with_live_database(
            &project,
            report.as_deref().expect("report required without output"),
            format,
            live_database.as_ref(),
        )
        .map(|analysis| CommandOutput {
            report_path: Some(analysis.report_path),
            scaffold_path: None,
            counts: analysis.counts,
            generated_schema_tables: 0,
            generated_business_functions: 0,
            generated_enum_adapters: 0,
            generated_validation_forms: 0,
            generated_service_helpers: 0,
            requested_tables: Vec::new(),
            included_tables: Vec::new(),
        })
    };
    match result {
        Ok(output) => {
            if inspect_database {
                let status = live_database
                    .as_ref()
                    .and_then(|result| result["status"].as_str())
                    .unwrap_or("unknown");
                println!("Laravel source analyzed without executing PHP; read-only database inspection status: {status}.");
                if let Some(error) = live_database
                    .as_ref()
                    .and_then(|result| result["error"].as_str())
                {
                    println!("Database inspection note: {error}");
                }
            } else {
                println!(
                    "Laravel source analyzed without executing PHP or reading environment files."
                );
            }
            if let Some(path) = output.report_path {
                println!("Report: {}", path.display());
            }
            if let Some(path) = output.scaffold_path {
                println!("Separate Zelyra scaffold: {}", path.display());
                println!("Generated pure business projections: {}; review MIGRATION_TODO.md for assumptions and caller wiring.", output.generated_business_functions);
                println!("Generated typed Laravel enum adapters: {}; callers must use them explicitly; database constraints are not preserved.", output.generated_enum_adapters);
                println!("Generated standalone validation forms: {}; not connected to routes, handlers, or persistence.", output.generated_validation_forms);
                println!("Generated reusable pure service helpers: {}; inspect MIGRATION_TODO.md for static call sites and wiring review.", output.generated_service_helpers);
                println!("Generated schema declarations: {} (review before configuring or applying a database schema).", output.generated_schema_tables);
                if !output.requested_tables.is_empty() {
                    println!(
                        "Requested tables: {}; included with relation dependencies: {}.",
                        output.requested_tables.join(", "),
                        output.included_tables.join(", ")
                    );
                }
            }
            println!(
                "Findings: {} · unresolved/manual: {} · partial: {}",
                output.counts.total, output.counts.unresolved, output.counts.partial
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error[E-MIGRATE-002]: {error}");
            ExitCode::from(1)
        }
    }
}

fn generate_scaffold(
    project: &Path,
    external_report: Option<&Path>,
    output: &Path,
    format: &str,
    requested_tables: &[String],
    live_database: Option<&Value>,
) -> Result<CommandOutput, String> {
    let root = fs::canonicalize(project)
        .map_err(|_| "Laravel project directory does not exist or is not readable".to_owned())?;
    if !root.is_dir() {
        return Err("Laravel project path must be a directory".into());
    }
    let output = resolve_new_report_path(output)?;
    let output_parent = output.parent().ok_or("target needs a parent directory")?;
    let canonical_parent = fs::canonicalize(output_parent)
        .map_err(|_| "target parent directory does not exist".to_owned())?;
    let output_name = output
        .file_name()
        .ok_or("target directory name is required")?;
    let target = canonical_parent.join(output_name);
    if target.starts_with(&root) || root.starts_with(&target) {
        return Err("scaffold target must be separate from the Laravel source project".into());
    }
    if fs::symlink_metadata(&target).is_ok() {
        return Err(
            "scaffold target already exists; choose a new directory so files are never overwritten"
                .into(),
        );
    }
    let report_target = if let Some(report) = external_report {
        let report = resolve_new_report_path(report)?;
        let parent = report.parent().ok_or("report needs a parent directory")?;
        let parent = fs::canonicalize(parent)
            .map_err(|_| "report parent directory does not exist".to_owned())?;
        let name = report.file_name().ok_or("report filename is required")?;
        let report_target = parent.join(name);
        if report_target.starts_with(&root) || root.starts_with(&report_target) {
            return Err("report path must be outside the Laravel source project".into());
        }
        if report_target.starts_with(&target) || target.starts_with(&report_target) {
            return Err("report path and scaffold target must be separate".into());
        }
        if fs::symlink_metadata(&report_target).is_ok() {
            return Err(
                "report already exists; choose a new path so reports are never overwritten".into(),
            );
        }
        Some((report_target, report))
    } else {
        None
    };

    let mut staging = None;
    for attempt in 0..16_u32 {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "system clock is before the Unix epoch".to_owned())?
            .as_nanos();
        let path = canonical_parent.join(format!(
            ".zelyra-laravel-{}-{nonce}-{attempt}",
            std::process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => {
                staging = Some(path);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "could not create private staging directory: {error}"
                ))
            }
        }
    }
    let staging = staging.ok_or("could not allocate a unique staging directory")?;
    let mut external_report_temporary = None;
    let result = (|| {
        let staged_report = staging.join("MIGRATION_REPORT.json");
        let analysis = analyze_with_live_database(&root, &staged_report, "json", live_database)?;
        let counts = analysis.counts;
        let report_json: Value = zelyra_ast::json::parse_slice(
            &fs::read(&staged_report)
                .map_err(|error| format!("could not read staged report: {error}"))?,
        )
        .map_err(|_| "generated migration report is invalid JSON".to_owned())?;
        let console_inventory = migration_console_inventory(&report_json["console_analysis"]);
        let console_tasks_markdown = render_console_tasks_todo(&console_inventory);
        let schema_generation = report_json["schema_generation"].clone();
        let mut source_file_plan = report_json["source_file_inventory"].clone();
        let mut view_link_analysis = report_json["view_links"].clone();
        let table_scoped = !requested_tables.is_empty();
        let asset_copy_report = if table_scoped {
            json!({
                "copied_count": 0,
                "not_copied_count": 0,
                "copied_bytes": 0,
                "assets": [],
                "status": "not_selected",
                "reason": "--table selects schema-related artifacts only; project-wide view assets require an unfiltered generation"
            })
        } else {
            view_links::copy_referenced_assets(&root, &staging, &mut view_link_analysis)
        };
        let generated_layout_source = if table_scoped {
            None
        } else {
            view_links::render_zelyra_layout_source(&view_link_analysis)
        };
        let generated_layout_count = if generated_layout_source.is_some() {
            1
        } else {
            0
        };
        let (generated_static_view_source, view_translation_report) = if table_scoped {
            (
                None,
                json!({
                    "translated_count": 0,
                    "translations": [],
                    "status": "not_selected",
                    "reason": "--table selects schema-related artifacts only; project-wide view translation requires an unfiltered generation"
                }),
            )
        } else {
            view_links::translate_static_confirmation_views(
                &root,
                &view_link_analysis,
                &report_json["graph"],
            )
        };
        let generated_view_component_count = view_translation_report["translated_count"]
            .as_u64()
            .unwrap_or(0);
        if let Some(source_files) = source_file_plan.as_array_mut() {
            for source_file in source_files {
                let Some(file) = source_file["file"].as_str() else {
                    continue;
                };
                let translated = view_translation_report["translations"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|translation| {
                        translation["source_file"].as_str() == Some(file)
                            && translation["status"] == "translated_static_presentation_component"
                    });
                if let Some(translation) = translated {
                    source_file["translation_status"] =
                        json!("partially_translated_static_presentation_route_handler_not_wired");
                    source_file["translated_outputs"] = json!([{
                        "kind": "static_presentation_component",
                        "component": translation["component"],
                        "target_module": "src/imported_views.zyl",
                        "route_wired": false
                    }]);
                }
            }
        }
        let selection = select_schema_candidates(&schema_generation, requested_tables)?;
        let requested_table_names = selection.requested_names;
        let included_table_names = selection.included_names;
        let included_tables = selection.candidates;
        let route_target_tables = if requested_table_names.is_empty() {
            included_table_names.as_slice()
        } else {
            requested_table_names.as_slice()
        };
        let route_candidates =
            route_candidates_for_tables(&report_json["graph"], route_target_tables);
        let login_evidence = if table_scoped
            && !requested_table_names
                .iter()
                .any(|table| table.eq_ignore_ascii_case("users"))
        {
            None
        } else {
            authentication::detect_login_module(&report_json)
        };
        let users_candidate = included_tables
            .iter()
            .find(|candidate| candidate["table"].as_str() == Some("users"));
        let (native_auth, auth_mapping_issue) = if login_evidence.is_some() {
            match users_candidate {
                Some(users) => {
                    let session_table = generated_auth_session_table_name(&schema_generation);
                    match authentication::render_native_auth_module(users, &session_table) {
                        Ok(source) => (Some((session_table, source)), None),
                        Err(reason) => (None, Some(reason)),
                    }
                }
                None => (
                    None,
                    Some("the selected schema does not include the required `users` model".into()),
                ),
            }
        } else {
            (None, None)
        };
        let generated_schema_source = included_tables
            .iter()
            .filter(|candidate| {
                native_auth.is_none() || candidate["table"].as_str() != Some("users")
            })
            .filter_map(|candidate| candidate["candidate_source"].as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let generated_table_count = included_tables.len();
        let business_generation = report_json["business_logic_generation"].clone();
        let native_auth_controller_file = native_auth
            .as_ref()
            .and(login_evidence.as_ref())
            .map(|evidence| evidence.controller_file.as_str());
        let validation_forms = if table_scoped {
            validation_forms::ValidationForms {
                source: String::new(),
                mappings: Vec::new(),
                rejected: Vec::new(),
            }
        } else {
            validation_forms::generate(&report_json, native_auth_controller_file)
        };
        let generated_validation_form_count = validation_forms.mappings.len();
        let all_business_candidates = business_generation["candidates"]
            .as_array()
            .ok_or("business logic report has no candidates list")?;
        let included_business_candidates = if requested_table_names.is_empty() {
            all_business_candidates.iter().collect::<Vec<_>>()
        } else {
            all_business_candidates
                .iter()
                .filter(|candidate| {
                    candidate["schema_table"]
                        .as_str()
                        .is_some_and(|table| included_table_names.iter().any(|name| name == table))
                })
                .collect::<Vec<_>>()
        };
        let generated_business_source = included_business_candidates
            .iter()
            .filter_map(|candidate| candidate["candidate_source"].as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let generated_business_count = included_business_candidates.len();
        let service_generation = report_json["service_generation"].clone();
        let all_service_candidates = service_generation["candidates"]
            .as_array()
            .ok_or("service helper report has no candidates list")?;
        let included_service_candidates = if requested_table_names.is_empty() {
            all_service_candidates.iter().collect::<Vec<_>>()
        } else {
            all_service_candidates
                .iter()
                .filter(|candidate| {
                    candidate["schema_table"]
                        .as_str()
                        .is_some_and(|table| included_table_names.iter().any(|name| name == table))
                })
                .collect::<Vec<_>>()
        };
        let generated_service_count = included_service_candidates.len();
        let service_support_needs_address = included_service_candidates
            .iter()
            .any(|candidate| candidate["pattern"] == "static_tenant_address_projection");
        let generated_service_source = if generated_service_count == 0 {
            String::new()
        } else {
            format!(
                "{}\n{}\n",
                service_helpers::render_service_support(service_support_needs_address),
                included_service_candidates
                    .iter()
                    .filter_map(|candidate| candidate["function_source"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n\n")
            )
        };
        let included_enum_adapters = schema_generation["enum_adapters"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|adapter| {
                included_table_names
                    .iter()
                    .any(|table| adapter["table"].as_str() == Some(table.as_str()))
            })
            .collect::<Vec<_>>();
        let generated_enum_source = included_enum_adapters
            .iter()
            .filter_map(|adapter| adapter["candidate_source"].as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let generated_enum_adapter_count = included_enum_adapters.len();
        let authentication_selection = match (&login_evidence, &native_auth) {
            (Some(evidence), Some((session_table, _))) => json!({
                "status": "generated_native_module",
                "module": "packages/imported_identity/src/lib.zyl",
                "package": "imported_identity",
                "users_table": "users",
                "sessions_table": session_table,
                "source_route_file": evidence.route_file,
                "source_controller_file": evidence.controller_file,
                "source_view_file": evidence.view_file,
                "generated_native_login": true,
                "source_code_translated": false,
                "user_records_copied": false,
                "password_hashes_copied": false,
                "limitations": [
                    "Laravel registration, approval policy, tenant selection, 2FA, and controller behavior are not transferred.",
                    "Existing Laravel password hashes must be reset or deliberately converted before Zelyra login can use them.",
                    "The original Blade login page is inventoried but replaced by Zelyra's native login UI."
                ]
            }),
            (Some(evidence), None) => json!({
                "status": "not_generated",
                "source_route_file": evidence.route_file,
                "source_controller_file": evidence.controller_file,
                "source_view_file": evidence.view_file,
                "generated_native_login": false,
                "reason": auth_mapping_issue.as_deref().unwrap_or("the users schema is not eligible for native authentication")
            }),
            (None, _) => json!({
                "status": "not_detected",
                "generated_native_login": false
            }),
        };
        let generation_selection = json!({
            "schema_version": "1",
            "mode": if requested_table_names.is_empty() { "all_supported_candidates" } else { "requested_tables_with_relation_closure" },
            "project_wide_views_included": !table_scoped,
            "requested_tables": requested_table_names.clone(),
            "included_tables": included_table_names.clone(),
            "excluded_framework_tables": selection.excluded_framework_names,
            "relation_dependencies": included_table_names.iter().filter(|table| !requested_table_names.contains(table)).collect::<Vec<_>>(),
            "route_candidates": route_candidates.clone(),
            "route_candidates_generated": false,
            "business_helpers": included_business_candidates.iter().filter_map(|candidate| candidate["function"].as_str()).collect::<Vec<_>>(),
            "business_module": if generated_business_count > 0 { Some("src/imported_business.zyl") } else { None::<&str> },
            "service_helpers": included_service_candidates.iter().map(|candidate| json!({
                "class": candidate["class"],
                "method": candidate["method"],
                "function": candidate["function"],
                "pattern": candidate["pattern"],
                "model_type": candidate["model_type"],
                "schema_table": candidate["schema_table"],
                "fields": candidate["fields"],
                "source_file": candidate["source_file"],
                "source_span": candidate["source_span"],
                "schema_field_evidence": candidate["schema_field_evidence"],
                "static_call_sites": candidate["static_call_sites"],
                "translation_status": "generated_reusable_helper_call_sites_require_reviewed_wiring"
            })).collect::<Vec<_>>(),
            "service_module": if generated_service_count > 0 { Some("src/imported_services.zyl") } else { None::<&str> },
            "service_helpers_wired_to_call_sites": false,
            "enum_adapters": included_enum_adapters.iter().map(|adapter| json!({
                "table": adapter["table"],
                "column": adapter["column"],
                "type": adapter["zelyra_type"],
                "parse_function": adapter["parse_function"],
                "storage_function": adapter["storage_function"],
                "allowed_values": adapter["values"],
                "variants": adapter["variants"],
                "database_constraint_preserved": false
            })).collect::<Vec<_>>(),
            "enum_module": if generated_enum_adapter_count > 0 { Some("src/imported_enums.zyl") } else { None::<&str> },
            "validation_forms": validation_forms.mappings,
            "rejected_validation_forms": validation_forms.rejected,
            "validation_module": if generated_validation_form_count > 0 { Some("src/imported_forms.zyl") } else { None::<&str> },
            "validation_module_imported_by_default": false,
            "layout_module": if generated_layout_count > 0 { Some("src/imported_layouts.zyl") } else { None::<&str> },
            "layout_component": if generated_layout_count > 0 { Some("ImportedAppShell") } else { None::<&str> },
            "layout_slot_bindings_wired_to_views": false,
            "view_translations": view_translation_report,
            "view_translation_module": if generated_view_component_count > 0 { Some("src/imported_views.zyl") } else { None::<&str> },
            "view_components_wired_to_routes": false,
            "asset_copy": asset_copy_report.clone(),
            "authentication": authentication_selection,
            "validation_forms_wired_to_routes": false,
            "validation_forms_generate_persistence": false,
            "laravel_console_inventory": console_inventory,
            "laravel_console_tasks_imported": false,
            "source_file_plan": "SOURCE_FILE_PLAN.json",
            "database_changes_applied": false
        });
        let mut generation_selection_source =
            serde_json::to_string_pretty(&generation_selection)
                .map_err(|_| "could not serialize generation selection".to_owned())?;
        generation_selection_source.push('\n');
        write_new_file(
            &staging.join("GENERATION_SELECTION.json"),
            &generation_selection_source,
        )?;
        let mut source_file_plan_source = serde_json::to_string_pretty(&source_file_plan)
            .map_err(|_| "could not serialize source file migration plan".to_owned())?;
        source_file_plan_source.push('\n');
        write_new_file(
            &staging.join("SOURCE_FILE_PLAN.json"),
            &source_file_plan_source,
        )?;
        write_new_file(
            &staging.join("ROUTE_CANDIDATES.json"),
            &format!(
                "{}\n",
                serde_json::to_string_pretty(&route_candidates)
                    .map_err(|_| "could not serialize route candidates".to_owned())?
            ),
        )?;
        write_new_file(
            &staging.join("MIGRATION_FILES.md"),
            &render_source_file_plan_markdown(&source_file_plan),
        )?;
        let view_link_source = serde_json::to_string_pretty(&view_link_analysis)
            .map_err(|_| "could not serialize Blade view link inventory".to_owned())?;
        write_new_file(&staging.join("VIEW_LINKS.json"), &view_link_source)?;
        let asset_copy_source = serde_json::to_string_pretty(&asset_copy_report)
            .map_err(|_| "could not serialize referenced asset copy report".to_owned())?;
        write_new_file(&staging.join("ASSET_COPY.json"), &asset_copy_source)?;
        write_new_file(
            &staging.join("ASSET_COPY.md"),
            &view_links::render_asset_copy_markdown(&asset_copy_report),
        )?;
        write_new_file(
            &staging.join("VIEW_LAYOUTS.md"),
            &view_links::render_layout_mapping_markdown(&view_link_analysis),
        )?;
        let view_translation_source = serde_json::to_string_pretty(&view_translation_report)
            .map_err(|_| "could not serialize static view translation report".to_owned())?;
        write_new_file(
            &staging.join("VIEW_TRANSLATIONS.json"),
            &view_translation_source,
        )?;
        write_new_file(
            &staging.join("VIEW_TRANSLATIONS.md"),
            &view_links::render_view_translation_markdown(&view_translation_report),
        )?;
        let project_name = root
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| {
                name.chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
            })
            .unwrap_or("laravel-migration");
        write_new_file(
            &staging.join("zelyra.toml"),
            &render_scaffold_manifest(project_name, native_auth.is_some()),
        )?;
        if let Some((_session_table, source)) = &native_auth {
            let package_root = staging.join("packages/imported_identity");
            fs::create_dir_all(package_root.join("src")).map_err(|error| {
                format!("could not create reusable authentication package: {error}")
            })?;
            write_new_file(
                &package_root.join("zelyra.toml"),
                "[package]\nname = \"imported_identity\"\nversion = \"0.1.0\"\nentry = \"src/lib.zyl\"\n",
            )?;
            write_new_file(&package_root.join("src/lib.zyl"), source)?;
        }
        if generated_business_count > 0
            || generated_enum_adapter_count > 0
            || generated_validation_form_count > 0
            || generated_service_count > 0
            || generated_layout_count > 0
            || generated_view_component_count > 0
        {
            fs::create_dir(staging.join("src"))
                .map_err(|error| format!("could not create generated module directory: {error}"))?;
        }
        if generated_business_count > 0 {
            write_new_file(
                &staging.join("src/imported_business.zyl"),
                &format!("{}\n", generated_business_source),
            )?;
        }
        if generated_enum_adapter_count > 0 {
            write_new_file(
                &staging.join("src/imported_enums.zyl"),
                &format!("{}\n", generated_enum_source),
            )?;
        }
        if generated_validation_form_count > 0 {
            write_new_file(
                &staging.join("src/imported_forms.zyl"),
                &validation_forms.source,
            )?;
        }
        if generated_service_count > 0 {
            write_new_file(
                &staging.join("src/imported_services.zyl"),
                &generated_service_source,
            )?;
        }
        if let Some(source) = &generated_layout_source {
            write_new_file(&staging.join("src/imported_layouts.zyl"), source)?;
        }
        if let Some(source) = &generated_static_view_source {
            write_new_file(&staging.join("src/imported_views.zyl"), source)?;
        }
        let business_import = if generated_business_count > 0 {
            "import \"src/imported_business.zyl\" as imported_business\n\n"
        } else {
            ""
        };
        let enum_import = if generated_enum_adapter_count > 0 {
            "import \"src/imported_enums.zyl\" as imported_enums\n\n"
        } else {
            ""
        };
        let service_import = if generated_service_count > 0 {
            "import \"src/imported_services.zyl\" as imported_services\n\n"
        } else {
            ""
        };
        let layout_import = if generated_layout_count > 0 {
            "import \"src/imported_layouts.zyl\" as imported_layouts\n\n"
        } else {
            ""
        };
        let view_import = if generated_view_component_count > 0 {
            "import \"src/imported_views.zyl\" as imported_views\n\n"
        } else {
            ""
        };
        write_new_file(
            &staging.join("main.zyl"),
            &format!(
                "{}{}{}{}{}{}# Generated scaffold: business projections, service helpers, enum adapters, standalone validation form candidates, and static view components are emitted as regular Zelyra modules. Form candidates are intentionally not imported, so they do not create routes until explicitly wired. Review and wire generated behavior explicitly.\n# Schema declarations are exact-pattern candidates and require review.\n{}\nfn main() {{\n    print(\"Review MIGRATION_TODO.md before implementing this application.\")\n}}\n",
                business_import,
                enum_import,
                service_import,
                layout_import,
                view_import,
                if native_auth.is_some() { "import \"pkg:imported_identity\" as identity\n\n" } else { "" },
                generated_schema_source,
            ),
        )?;
        if native_auth.is_some() {
            let package_graph = super::packages::PackageGraph::load(&staging)
                .map_err(|error| {
                    format!(
                        "generated authentication package is invalid: {}",
                        error.message
                    )
                })?
                .ok_or("generated authentication package manifest was not recognized")?;
            super::packages::write_lockfile(&package_graph, false).map_err(|error| {
                format!(
                    "could not lock generated authentication package: {}",
                    error.message
                )
            })?;
        }
        write_new_file(
            &staging.join("README.md"),
            &format!(
                "# Zelyra migration scaffold\n\nThis is a separate, compiler-checkable starting point. It contains no copied Laravel PHP, configuration, secrets, personal records, or production data. It includes {generated_table_count} schema declaration candidate(s) from a restricted static migration subset; these declarations do not apply database changes and must be reviewed. It also contains {generated_business_count} pure model projection helper(s) translated from a narrow static PHP subset in `src/imported_business.zyl`, imported from `main.zyl` through Zelyra's normal module system. These helpers do not include model casts, call sites, authorization, or surrounding workflows; review their assumptions and wire them explicitly. It contains {generated_service_count} narrow pure service helper(s) in `src/imported_services.zyl`, imported through the normal module system. Their statically resolved callers are listed, but their controller/service workflows are not automatically rewired. It contains {generated_enum_adapter_count} typed adapters for literal Laravel enum values in `src/imported_enums.zyl`. They map stored strings to Zelyra enum variants and back, but do not add database constraints or wire application call sites; parse every value read from storage and validate writes explicitly. It contains {generated_validation_form_count} standalone form declaration candidate(s) from a strict subset of literal Laravel validation rules in `src/imported_forms.zyl`. This file is deliberately not imported by `main.zyl`, so the suggestions do not create endpoints until explicitly included and reviewed. They are not connected to Laravel routes or handlers and do not perform Laravel validation or persistence; omitted rules and fields are recorded in `GENERATION_SELECTION.json`. Laravel command and scheduler metadata is listed in `MIGRATION_TASKS.md`; PHP handlers are not imported. Every statically inventoried PHP source file and its proposed migration disposition is listed in `MIGRATION_FILES.md` and `SOURCE_FILE_PLAN.json`; entries are not claims that behavior was translated. `ROUTE_CANDIDATES.json` separates statically resolved routes from conventionally named resource controllers and routes that merely consume the selected model; neither means a route was generated. Blade URL, asset, and shared-layout references are recorded in `VIEW_LINKS.json`; `VIEW_LAYOUTS.md` maps each `@extends` consumer and its `@section` values to the layout's `@yield` slots. Zelyra already provides reusable named `view` layouts. When a shared Blade layout is found, the scaffold emits the Zelyra starter `view ImportedAppShell` in `src/imported_layouts.zyl` and imports that module; each translated page can bind it with `view: ImportedAppShell`. The shared frame is therefore reusable immediately, while original navigation/auth behavior and Blade page bodies still require review and translation. Two strict standalone notice shapes may also be emitted as presentation components in `src/imported_views.zyl`: a static confirmation and a module notice with one escaped `String` property. Inspect `VIEW_TRANSLATIONS.md` and `VIEW_TRANSLATIONS.json`. Neither emits a Laravel route, handler, or dynamic workflow. Referenced public assets are copied into the generated `public/` tree when they are regular files within the configured size limits; `ASSET_COPY.md` and `ASSET_COPY.json` list copied and skipped assets. URLs remain origin-relative, and host filesystem paths are never emitted as application URLs. The complete schema selection and added relation dependencies are listed in `GENERATION_SELECTION.json`; the migration report remains the full-project analysis.\n\nStart with `zelyra check main.zyl`, then review `MIGRATION_REPORT.md`, `MIGRATION_TODO.md`, `MIGRATION_TASKS.md`, `MIGRATION_FILES.md`, `VIEW_LINKS.json`, `VIEW_LAYOUTS.md`, `VIEW_TRANSLATIONS.md`, `VIEW_TRANSLATIONS.json`, `ASSET_COPY.md`, `ASSET_COPY.json`, `SOURCE_FILE_PLAN.json`, `ROUTE_CANDIDATES.json`, and `GENERATION_SELECTION.json` before connecting or changing a database.\n",
                generated_validation_form_count = generated_validation_form_count
            ),
        )?;
        if let Some((session_table, _)) = &native_auth {
            append_generated_text(
                &staging.join("README.md"),
                &format!(
                    "\n## Reusable native authentication module\n\nA statically linked Laravel `GET /login` page, `POST /login` handler, and `users` schema produced the local package `packages/imported_identity`. The package owns the projected `users` table, the Zelyra session table `{session_table}`, and a native `auth users` declaration. Zelyra supplies the login UI and routes; no Laravel controller, Blade template, session storage, or user data was copied. The generated package can be referenced from other Zelyra projects with a local package dependency.\n\nThe Laravel `password` field is projected to `password_hash`, and `remember_token` is omitted. This scaffold does not copy accounts or password hashes. Create new credentials with Zelyra or deliberately convert the source hashes after reviewing the source hashing scheme. Registration, account approval, tenant selection, 2FA, and custom controller policies are not transferred; review `GENERATION_SELECTION.json` before enabling access.\n"
                ),
            )?;
        }
        let findings = report_json["findings"]
            .as_array()
            .ok_or("generated migration report has no findings list")?;
        let ruleset = report_json["mapping_ruleset"].as_str().unwrap_or("unknown");
        let analysis_method = if report_json["env_files_read"] == true {
            "static and read-only; PHP/Artisan were not run; `.env` connection settings were read only because `--inspect-database` was explicitly requested; no application rows were read and no database changes were applied"
        } else {
            "static and read-only; PHP/Artisan and `.env` were not read; no database connection was opened"
        };
        let mut report_markdown = format!(
            "# Laravel migration inventory\n\n- Mapping ruleset: `{ruleset}`\n- Source: `{}`\n- Analysis: {analysis_method}.\n- Generated schema candidates: {generated_table_count} (review required; no database changes applied).\n- Generated pure model projections: {generated_business_count} (limited static subset; no model wiring or workflow transfer).\n- Generated enum adapters: {generated_enum_adapter_count} (typed conversion helpers only; SQL value constraints are not preserved).\n\n",
            report_json["source_project"].as_str().unwrap_or("Laravel project"),
        );
        let _ = writeln!(
            report_markdown,
            "- Generated standalone validation forms: {generated_validation_form_count} (strict literal-rule subset; not connected to routes, handlers, validation execution, or persistence).\n"
        );
        report_markdown.push_str(&format!(
            "## Scaffold generation selection\n\n- Mode: `{}`\n- Requested tables: `{}`\n- Included tables: `{}`\n- Generated business helpers: `{}`\n- Generated enum adapters: `{}`\n- Business module: `{}`\n- Enum module: `{}`\n- Database changes applied: no\n\nThe JSON analysis below covers the full Laravel project. Only the listed tables and helpers were emitted; helper functions live in regular Zelyra modules, and relation targets are included transitively. Enum adapters do not preserve a database allowed-value constraint and must be called explicitly.\n\n",
            generation_selection["mode"].as_str().unwrap_or("unknown"),
            requested_table_names.join(", "),
            included_table_names.join(", "),
            generation_selection["business_helpers"].as_array().map_or(0, Vec::len),
            generation_selection["enum_adapters"].as_array().map_or(0, Vec::len),
            generation_selection["business_module"].as_str().unwrap_or("none"),
            generation_selection["enum_module"].as_str().unwrap_or("none")
        ));
        let mut todo = format!(
            "# Migration work still required\n\n{} schema declaration candidate(s) were generated in `main.zyl` from a restricted static subset. Compare every field, nullability, default, unique constraint, and database-specific type with the original migrations before configuring or applying a schema. The exact table selection and relation dependencies are recorded in `GENERATION_SELECTION.json`; the analysis report still lists findings from the whole Laravel project. Narrowly supported pure model projections are isolated in `src/imported_business.zyl` and imported through the standard module system; their model fields must be passed explicitly. Typed Laravel enum adapters are isolated in `src/imported_enums.zyl`; they do not preserve SQL constraints or wire call sites, so explicitly decode stored values and validate before writes. Call sites, model casts, authorization, and workflows are not imported. Each mapping ID below identifies an analyzer finding, not a claim that behavior is supported. Review and implement authorization, tenant boundaries, validation, transactions, money calculations, file handling, jobs, integrations, and error paths before enabling a workflow. Laravel commands and scheduled tasks are inventory-only; translate their behavior and create reviewed Zelyra `command`/`schedule` declarations by following `MIGRATION_TASKS.md`.\n\n",
            generated_table_count
        );
        if generated_validation_form_count > 0 || !validation_forms.rejected.is_empty() {
            use std::fmt::Write as _;
            let _ = writeln!(
                todo,
                "## Validation form scaffolds\n\n{generated_validation_form_count} standalone form(s) were emitted in `src/imported_forms.zyl` from supported literal rules. They are compiler-checked declarations only: route/handler wiring, runtime validation, authorization, and persistence must be implemented separately. {} validation candidate(s) or field(s) were rejected; inspect `GENERATION_SELECTION.json` for the exact fields and reasons.\n",
                validation_forms.rejected.len()
            );
        }
        if generated_service_count > 0
            || service_generation["rejected"]
                .as_array()
                .is_some_and(|rejected| !rejected.is_empty())
        {
            use std::fmt::Write as _;
            let _ = writeln!(
                todo,
                "## Reusable service helper translations\n\n{generated_service_count} pure helper(s) were emitted to `src/imported_services.zyl` and imported as `imported_services`. They are not connected to Laravel controller/service workflows automatically. Review each static call site and add explicit Zelyra function calls in the owning feature module; preserve authorization, transactions, and error handling."
            );
            for helper in &included_service_candidates {
                let _ = writeln!(
                    todo,
                    "\n- Wire `imported_services.{}` for `{}::{}` with fields `{}`. Source: `{}:{}`; static caller count: {}. Verify the source flow before changing the caller.",
                    helper["function"].as_str().unwrap_or("unknown"),
                    helper["class"].as_str().unwrap_or("unknown"),
                    helper["method"].as_str().unwrap_or("unknown"),
                    helper["fields"].as_array().map(|fields| fields.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ")).unwrap_or_default(),
                    helper["source_file"].as_str().unwrap_or("unknown"),
                    helper["source_span"]["start"]["line"].as_u64().unwrap_or(0),
                    helper["static_call_sites"].as_array().map_or(0, Vec::len)
                );
            }
        }
        let view_count = view_link_analysis["view_count"].as_u64().unwrap_or(0);
        let link_count = view_link_analysis["reference_count"].as_u64().unwrap_or(0);
        let composition_count = view_link_analysis["composition_reference_count"]
            .as_u64()
            .unwrap_or(0);
        use std::fmt::Write as _;
        let _ = writeln!(
            todo,
            "## Views, shared layouts, and portable links\n\nThe analyzer found {view_count} Blade view file(s) with recognized URL/layout references, {composition_count} layout/include/slot composition reference(s), and {link_count} URL or asset reference(s). Review `VIEW_LINKS.json` and `VIEW_LAYOUTS.md`: they map each Laravel `@extends` page to its shared layout and each `@section` to the matching `@yield` slot. The generated shared-shell starter is in `src/imported_layouts.zyl`. Zelyra already supports reusable named `view` layouts through `layout: ViewName`; create the shared shell once as a Zelyra `view` and set `view: ImportedAppShell` in each translated `page`; do not duplicate headers and footers. Zelyra’s existing `view`/slot system is the native equivalent, so no second layout language is needed. Named Laravel routes must resolve to declared Zelyra routes. Referenced public assets are copied into the generated `public/` tree when they are regular files within the configured size limits; `ASSET_COPY.md` records copied and skipped files. Their URLs remain origin-relative. Dynamic Blade links are not guessed or translated automatically.\n"
        );
        let _ = writeln!(
            todo,
            "## Bounded standalone Blade notice translation\n\nThe scaffold generated {generated_view_component_count} presentation component(s) in `src/imported_views.zyl` from two strict standalone notice shapes: a static confirmation and a module notice with one escaped `String` property. Review `VIEW_TRANSLATIONS.md` and `VIEW_TRANSLATIONS.json`. Generated components are imported from `main.zyl`; they have no generated route, handler, dynamic document title, or persistence behavior. Other dynamic Blade templates remain untranslated. Map component names to their Blade sources in `VIEW_TRANSLATIONS.json`; implement route/handler behavior separately before replacing a Laravel response.\n"
        );
        todo.push_str(
            "## Review and apply schema changes\n\nThe generated declarations describe a reviewed target schema; scaffold generation never connects to or changes a database. For an existing database, use Zelyra's schema planner instead of `zelyra db setup`. First complete the source review and configure the intended target through the project's normal database configuration without committing credentials. Then run `zelyra check main.zyl` and `zelyra db plan main.zyl --format json`. Review the plan's backend, schema fingerprints, SQL steps, preflight checks, risk classifications, and unsupported changes; a plan is a preview and does not reserve the database or back up data. Take and verify an independent backup before applying changes. Apply only the reviewed plan with `zelyra db apply main.zyl --plan-id <plan-id>`; add `--allow-risky` or `--allow-destructive` only when the reviewed plan requires that explicit approval. Confirm the result with `zelyra db history main.zyl` and `zelyra db inspect main.zyl`.\n\nThis imports neither Laravel data migrations nor `down()` behavior. Zelyra applies SQL generated from its checked schema diff; editing SQL printed in a plan does not change what `db apply` executes. Unsupported Laravel DDL, triggers, data transformations, and hand-written SQL need a separately designed and reviewed migration procedure. MariaDB DDL can leave earlier steps applied if a later step fails; inspect the live schema and generate a fresh plan before recovery.\n\n",
        );
        report_markdown.push_str("| Classification | Status | Mapping ID | Source span | Effect indicators | Finding | Next step |\n|---|---|---|---|---|---|---|\n");
        for finding in findings {
            let kind = finding["kind"].as_str().unwrap_or("unknown");
            let id = finding["mapping_id"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("laravel/{kind}"));
            let file = finding["file"].as_str().unwrap_or("unknown");
            let line = finding["line"].as_u64().unwrap_or(0);
            let name = finding["name"].as_str().unwrap_or("unclassified finding");
            let summary = finding["summary"].as_str().unwrap_or("");
            let next_step = finding["next_step"].as_str().unwrap_or("Review manually.");
            let status = finding["status"].as_str().unwrap_or("unresolved");
            let classification = finding["classification"].as_str().unwrap_or("manual");
            let source_span = &finding["source_span"];
            let span_start = &source_span["start"];
            let span_end = &source_span["end"];
            let location = if span_start["line"].is_number() {
                format!(
                    "{}:{}:{}-{}:{} (bytes {}..{})",
                    file,
                    span_start["line"].as_u64().unwrap_or(line),
                    span_start["column"].as_u64().unwrap_or(1),
                    span_end["line"].as_u64().unwrap_or(line),
                    span_end["column"].as_u64().unwrap_or(1),
                    span_start["byte_offset"].as_u64().unwrap_or(0),
                    span_end["byte_offset"].as_u64().unwrap_or(0)
                )
            } else {
                format!("{file}:{line}")
            };
            let effects = finding["effect_categories"]
                .as_array()
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let safe_cell = |value: &str| value.replace('|', "\\|").replace(['\n', '\r'], " ");
            use std::fmt::Write as _;
            let _ = writeln!(
                report_markdown,
                "| {} | {} | `{}` | `{}` | {} | {}: {} | {} |",
                safe_cell(classification),
                safe_cell(status),
                id,
                safe_cell(&location),
                safe_cell(&effects),
                safe_cell(name),
                safe_cell(summary),
                safe_cell(next_step)
            );
            let _ = writeln!(
                todo,
                "- **`{}`** · `{}` · {} / {}: {}\n  - Effect indicators: {}\n  - {}\n  - Next: {}",
                safe_cell(&id),
                safe_cell(&location),
                safe_cell(classification),
                safe_cell(status),
                safe_cell(name),
                safe_cell(&effects),
                safe_cell(summary),
                safe_cell(next_step)
            );
        }
        for candidate in business_generation["candidates"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let _ = writeln!(
                report_markdown,
                "\n- Generated `{}` for `{}::{}` from `{}:{}`; explicit arguments: `{}`; schema evidence: `{}`.",
                candidate["function"].as_str().unwrap_or("unknown"),
                candidate["class"].as_str().unwrap_or("unknown"),
                candidate["method"].as_str().unwrap_or("unknown"),
                candidate["file"].as_str().unwrap_or("unknown"),
                candidate["source_span"]["start"]["line"]
                    .as_u64()
                    .unwrap_or(0),
                business_parameter_summary(candidate),
                business_schema_evidence_summary(candidate)
            );
        }
        for rejected in business_generation["rejected"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let _ = writeln!(
                report_markdown,
                "\n- Manually review `{}` from `{}:{}`: {}.",
                rejected["name"].as_str().unwrap_or("unknown predicate"),
                rejected["file"].as_str().unwrap_or("unknown"),
                rejected["source_span"]["start"]["line"]
                    .as_u64()
                    .unwrap_or(0),
                rejected["reason"].as_str().unwrap_or("unsupported syntax")
            );
        }
        let business_functions = business_generation["candidates"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if !business_functions.is_empty() {
            use std::fmt::Write as _;
            for function in business_functions {
                let _ = writeln!(
                    todo,
                    "\n- Wire `{}` for `{}::{}` with arguments `{}`. Compare the source at `{}:{}`, inspect schema evidence ({}), and verify PHP model casts and call-site behavior.",
                    function["function"].as_str().unwrap_or("unknown"),
                    function["class"].as_str().unwrap_or("unknown"),
                    function["method"].as_str().unwrap_or("unknown"),
                    business_parameter_summary(&function),
                    function["file"].as_str().unwrap_or("unknown"),
                    function["source_span"]["start"]["line"].as_u64().unwrap_or(0),
                    business_schema_evidence_summary(&function)
                );
            }
            for rejected in business_generation["rejected"]
                .as_array()
                .into_iter()
                .flatten()
            {
                let _ = writeln!(
                    todo,
                    "\n- Manually review `{}` in `{}:{}`: {}",
                    rejected["name"].as_str().unwrap_or("unknown predicate"),
                    rejected["file"].as_str().unwrap_or("unknown"),
                    rejected["source_span"]["start"]["line"]
                        .as_u64()
                        .unwrap_or(0),
                    rejected["reason"]
                        .as_str()
                        .unwrap_or("unsupported by the static predicate importer")
                );
            }
        }
        write_new_file(&staging.join("MIGRATION_REPORT.md"), &report_markdown)?;
        write_new_file(&staging.join("MIGRATION_TODO.md"), &todo)?;
        write_new_file(&staging.join("MIGRATION_TASKS.md"), &console_tasks_markdown)?;
        if native_auth.is_some() {
            append_generated_text(
                &staging.join("MIGRATION_TODO.md"),
                "\n## Native authentication model\n\nA reusable local Zelyra package was generated because the report resolves both conventional login routes and the controller's login view. Its native auth declaration replaces Laravel's login implementation. No user records or credential hashes are imported. Review account approval, tenant selection, registration, 2FA, and authorization before deployment; these Laravel behaviors are not implemented by this projection. The Laravel `sessions`, `password_reset_tokens`, and other framework-owned tables are intentionally excluded.\n",
            )?;
            append_generated_text(
                &staging.join("MIGRATION_REPORT.md"),
                "\n## Native authentication projection\n\nThe resolved Laravel login flow is represented by Zelyra's built-in login UI and auth routes through the reusable `imported_identity` package. This is a new Zelyra model, not translated Laravel controller or Blade code. The target stores `password_hash` and persistent Zelyra sessions; source users, password hashes, Laravel session payloads, and framework tables are not copied. Source-specific approval, tenant, registration, and 2FA behavior remains unimplemented and requires review.\n",
            )?;
        }
        let main = staging.join("main.zyl");
        super::validate(
            main.to_str()
                .ok_or("staged source path is not valid UTF-8")?,
        )
        .map_err(|_| "generated scaffold failed the Zelyra compiler check".to_owned())?;

        let staged_external_report = if let Some((destination, requested_path)) =
            report_target.as_ref()
        {
            let extension = if format == "json" { "json" } else { "md" };
            let report_parent = requested_path
                .parent()
                .ok_or("report needs a parent directory")?;
            let report_parent = fs::canonicalize(report_parent)
                .map_err(|_| "report parent directory does not exist".to_owned())?;
            let mut temporary_report_directory = None;
            for _ in 0..16 {
                let candidate = report_parent.join(format!(
                    ".zelyra-laravel-report-{}-{}",
                    std::process::id(),
                    OsRng.next_u64()
                ));
                match create_private_directory(&candidate) {
                    Ok(()) => {
                        temporary_report_directory = Some(candidate);
                        break;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => {
                        return Err(format!(
                            "could not create a private report staging directory: {error}"
                        ));
                    }
                }
            }
            let temporary_report_directory = temporary_report_directory
                .ok_or("could not allocate a temporary external report directory")?;
            external_report_temporary = Some(temporary_report_directory.clone());
            let temporary_report = temporary_report_directory.join(format!("report.{extension}"));
            let analysis =
                analyze_with_live_database(&root, &temporary_report, format, live_database)?;
            Some((destination.clone(), analysis.report_path))
        } else {
            None
        };
        super::module_cli::publish_directory_no_replace(&staging, &target).map_err(|error| {
            format!("could not publish scaffold without replacing an existing target: {error}")
        })?;
        let external_report_path = if let Some((destination, staged_report)) =
            staged_external_report
        {
            if let Err(error) = fs::hard_link(&staged_report, &destination) {
                let _ = fs::remove_dir_all(&target);
                return Err(format!(
                    "could not publish the external report without replacing an existing file: {error}"
                ));
            }
            if let Err(error) = fs::remove_file(&staged_report) {
                let _ = fs::remove_file(&destination);
                let _ = fs::remove_dir_all(&target);
                return Err(format!(
                    "could not finalize the scaffold report files: {error}"
                ));
            }
            if let Some(temporary_report_directory) = external_report_temporary.take() {
                if let Err(error) = fs::remove_dir_all(&temporary_report_directory) {
                    let _ = fs::remove_file(&destination);
                    let _ = fs::remove_dir_all(&target);
                    return Err(format!(
                        "could not remove the private report staging directory: {error}"
                    ));
                }
            }
            Some(destination)
        } else {
            None
        };
        Ok(CommandOutput {
            report_path: external_report_path,
            scaffold_path: Some(target.clone()),
            counts,
            generated_schema_tables: generated_table_count,
            generated_business_functions: generated_business_count,
            generated_enum_adapters: generated_enum_adapter_count,
            generated_validation_forms: generated_validation_form_count,
            generated_service_helpers: generated_service_count,
            requested_tables: requested_table_names,
            included_tables: included_table_names,
        })
    })();
    if staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    if let Some(temporary_report) = external_report_temporary {
        let _ = fs::remove_dir_all(temporary_report);
    }
    result
}

#[derive(Debug)]
struct SchemaCandidateSelection {
    requested_names: Vec<String>,
    included_names: Vec<String>,
    excluded_framework_names: Vec<String>,
    candidates: Vec<Value>,
}

const LARAVEL_FRAMEWORK_TABLES: &[&str] = &[
    "cache",
    "cache_locks",
    "failed_jobs",
    "job_batches",
    "jobs",
    "migrations",
    "password_reset_tokens",
    "sessions",
];

fn is_framework_owned_table(table: &str) -> bool {
    LARAVEL_FRAMEWORK_TABLES.contains(&table)
}

fn generated_auth_session_table_name(schema_generation: &Value) -> String {
    let existing = schema_generation["candidates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|candidate| candidate["table"].as_str())
        .collect::<BTreeSet<_>>();
    let base = "zelyra_auth_sessions";
    if !existing.contains(base) {
        return base.to_owned();
    }
    let mut suffix = 2_u32;
    loop {
        let candidate = format!("{base}_{suffix}");
        if !existing.contains(candidate.as_str()) {
            return candidate;
        }
        suffix = suffix.saturating_add(1);
    }
}

fn render_scaffold_manifest(project_name: &str, has_auth_package: bool) -> String {
    let mut manifest = format!("[project]\nname = \"{project_name}\"\nlanguage = \"0.1\"\n");
    if has_auth_package {
        manifest.push_str(
            "\n[package]\nname = \"laravel_import\"\nversion = \"1.0.0\"\nentry = \"main.zyl\"\n\n[dependencies.imported_identity]\npath = \"packages/imported_identity\"\nversion = \"^0.1.0\"\n",
        );
    }
    manifest
}

fn select_schema_candidates(
    schema_generation: &Value,
    requested_tables: &[String],
) -> Result<SchemaCandidateSelection, String> {
    let candidates = schema_generation["candidates"]
        .as_array()
        .ok_or("schema generation report has no candidate list")?;
    let candidate_tables = candidates
        .iter()
        .filter_map(|candidate| candidate["table"].as_str())
        .collect::<BTreeSet<_>>();
    let excluded_framework_names = candidate_tables
        .iter()
        .copied()
        .filter(|table| is_framework_owned_table(table))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let migratable_candidate_tables = candidate_tables
        .iter()
        .copied()
        .filter(|table| !is_framework_owned_table(table))
        .collect::<BTreeSet<_>>();
    let mut requested = BTreeSet::new();
    for table in requested_tables {
        if !is_safe_table_identifier(table) {
            return Err(format!("invalid Laravel schema table name `{table}`"));
        }
        if !requested.insert(table.as_str()) {
            return Err(format!(
                "Laravel schema table `{table}` was selected more than once"
            ));
        }
        if is_framework_owned_table(table) {
            return Err(format!(
                "Laravel framework-owned table `{table}` is intentionally excluded; map its application behavior to a native Zelyra feature instead"
            ));
        }
        if !candidate_tables.contains(table.as_str()) {
            return Err(format!(
                "Laravel schema table `{table}` has no accepted generated candidate; review the rejected entries in the full migration report"
            ));
        }
    }

    let included = if requested.is_empty() {
        migratable_candidate_tables.clone()
    } else {
        let mut included = requested.clone();
        let mut pending = requested.iter().copied().collect::<Vec<_>>();
        while let Some(table) = pending.pop() {
            let candidate = candidates
                .iter()
                .find(|candidate| candidate["table"].as_str() == Some(table))
                .ok_or_else(|| format!("schema dependency `{table}` has no generated candidate"))?;
            for dependency in candidate["columns"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|column| column["relation_table"].as_str())
            {
                if !candidate_tables.contains(dependency) {
                    return Err(format!(
                        "schema relation from `{table}` targets `{dependency}`, which has no generated candidate"
                    ));
                }
                if is_framework_owned_table(dependency) {
                    return Err(format!(
                        "schema relation from `{table}` targets Laravel framework-owned table `{dependency}`; review and model that behavior explicitly in Zelyra"
                    ));
                }
                if included.insert(dependency) {
                    pending.push(dependency);
                }
            }
        }
        included
    };

    let requested_names = requested.iter().map(|name| (*name).to_owned()).collect();
    let included_names = included.iter().map(|name| (*name).to_owned()).collect();
    let included_candidates = candidates
        .iter()
        .filter(|candidate| {
            candidate["table"]
                .as_str()
                .is_some_and(|table| included.contains(table))
        })
        .cloned()
        .collect();
    Ok(SchemaCandidateSelection {
        requested_names,
        included_names,
        excluded_framework_names,
        candidates: included_candidates,
    })
}

/// Trace selected table candidates to statically resolved controller routes
/// through conventional Laravel model/table names. This inventory is never
/// treated as route translation or behavioral equivalence.
fn route_candidates_for_tables(graph: &Value, selected_tables: &[String]) -> Value {
    let selected_models = selected_tables
        .iter()
        .filter_map(|table| conventional_model_name(table).map(|model| (model, table.clone())))
        .collect::<BTreeMap<_, _>>();
    let edges = graph["edges"].as_array().into_iter().flatten();
    let mut model_actions = BTreeMap::<String, BTreeSet<String>>::new();
    let mut model_controllers = BTreeMap::<String, BTreeSet<String>>::new();
    for edge in edges.clone() {
        if edge["kind"] != "controller_model_use" || edge["status"] != "resolved" {
            continue;
        }
        let (Some(action), Some(model)) =
            (edge["from"]["name"].as_str(), edge["to"]["name"].as_str())
        else {
            continue;
        };
        if selected_models.contains_key(model) {
            model_actions
                .entry(model.to_owned())
                .or_default()
                .insert(action.to_owned());
            let controller = action
                .split_once('@')
                .map(|(controller, _)| controller)
                .unwrap_or(action);
            model_controllers
                .entry(model.to_owned())
                .or_default()
                .insert(controller.to_owned());
        }
    }

    let route_nodes = graph["nodes"].as_array().into_iter().flatten();
    let route_nodes = route_nodes
        .filter(|node| node["kind"] == "route")
        .filter_map(|node| node["id"].as_str().map(|id| (id.to_owned(), node)))
        .collect::<BTreeMap<_, _>>();
    let mut candidates = Vec::<Value>::new();
    let mut consumers = Vec::<Value>::new();
    for edge in graph["edges"].as_array().into_iter().flatten() {
        if edge["kind"] != "route_controller_action" || edge["status"] != "resolved" {
            continue;
        }
        let (Some(route_id), Some(action)) =
            (edge["from"]["id"].as_str(), edge["to"]["name"].as_str())
        else {
            continue;
        };
        let Some(route) = route_nodes.get(route_id) else {
            continue;
        };
        for (model, table) in &selected_models {
            let controller_name = action
                .split_once('@')
                .map(|(name, _)| name)
                .unwrap_or(action);
            let direct_model_use = model_actions
                .get(model)
                .is_some_and(|actions| actions.contains(action));
            // Laravel route model binding often means show/edit/update/destroy
            // do not mention the model class in the method body. Once the
            // graph has direct model-use evidence for a conventionally named
            // resource controller, include its other resolved route actions
            // as candidates too, without claiming they were translated.
            let resource_controller_match = controller_name
                .eq_ignore_ascii_case(&format!("{model}Controller"))
                && model_controllers
                    .get(model)
                    .is_some_and(|controllers| controllers.contains(controller_name));
            if !direct_model_use && !resource_controller_match {
                continue;
            }
            let candidate = json!({
                "table": table,
                "model": model,
                "route": route["name"],
                "route_source_file": route["file"],
                "route_source_span": route["source_span"],
                "route_context": route["route_context"],
                "controller_action": action,
                "resource_controller_match": resource_controller_match,
                "controller_source_file": edge["to"]["file"],
                "controller_source_span": edge["to"]["source_span"],
                "resolution": if direct_model_use {
                    "statically_resolved_route_and_model_use"
                } else {
                    "statically_resolved_route_with_conventional_resource_controller_evidence"
                },
                "table_model_mapping": "conventional_laravel_pluralization_candidate_requires_confirmation",
                "translation_status": "analysis_only_route_handler_view_and_security_behavior_not_generated"
            });
            if resource_controller_match {
                candidates.push(candidate);
            } else {
                consumers.push(candidate);
            }
        }
    }
    let sort_routes = |routes: &mut Vec<Value>| {
        routes.sort_by(|left, right| {
            left["table"]
                .as_str()
                .cmp(&right["table"].as_str())
                .then_with(|| left["route"].as_str().cmp(&right["route"].as_str()))
        })
    };
    sort_routes(&mut candidates);
    sort_routes(&mut consumers);
    json!({
        "schema_version": "1",
        "semantics": "conventionally_named_resource_routes_are_candidates; related_routes_are_model_consumers; neither category is route_generation_or_behavioral_equivalence",
        "count": candidates.len(),
        "routes": candidates,
        "model_consumer_count": consumers.len(),
        "model_consumer_routes": consumers
    })
}

fn conventional_model_name(table: &str) -> Option<String> {
    if !is_safe_table_identifier(table) {
        return None;
    }
    let singular = if let Some(stem) = table.strip_suffix("ies") {
        format!("{stem}y")
    } else if ["ches", "shes", "xes", "zes", "sses"]
        .iter()
        .any(|suffix| table.ends_with(suffix))
    {
        table[..table.len().saturating_sub(2)].to_owned()
    } else if table.ends_with('s') && !table.ends_with("ss") {
        table[..table.len().saturating_sub(1)].to_owned()
    } else {
        return None;
    };
    let mut model = String::new();
    for segment in singular.split('_') {
        if segment.is_empty() {
            return None;
        }
        let mut characters = segment.chars();
        let first = characters.next()?;
        model.push(first.to_ascii_uppercase());
        model.push_str(characters.as_str());
    }
    Some(model)
}

fn is_safe_table_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn write_new_file(path: &Path, contents: &str) -> Result<(), String> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("could not create scaffold file: {error}"))?;
    if let Err(error) = file.write_all(contents.as_bytes()) {
        let _ = fs::remove_file(path);
        return Err(format!("could not write scaffold file: {error}"));
    }
    Ok(())
}

fn append_generated_text(path: &Path, contents: &str) -> Result<(), String> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(path)
        .map_err(|error| format!("could not open generated documentation file: {error}"))?;
    file.write_all(contents.as_bytes())
        .map_err(|error| format!("could not extend generated documentation file: {error}"))
}

fn create_private_directory(path: &Path) -> std::io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

#[cfg(test)]
fn analyze(project: &Path, report: &Path, format: &str) -> Result<AnalysisOutput, String> {
    analyze_with_live_database(project, report, format, None)
}

fn analyze_with_live_database(
    project: &Path,
    report: &Path,
    format: &str,
    live_database: Option<&Value>,
) -> Result<AnalysisOutput, String> {
    let root = fs::canonicalize(project)
        .map_err(|_| "Laravel project directory does not exist or is not readable".to_owned())?;
    if !root.is_dir() {
        return Err("Laravel project path must be a directory".into());
    }
    let output = resolve_new_report_path(report)?;
    let output_parent = output.parent().ok_or("report needs a parent directory")?;
    let canonical_parent = fs::canonicalize(output_parent)
        .map_err(|_| "report parent directory does not exist".to_owned())?;
    let output_name = output.file_name().ok_or("report filename is required")?;
    let canonical_output = canonical_parent.join(output_name);
    if canonical_output.starts_with(&root) || root.starts_with(&canonical_output) {
        return Err("report path must be outside the Laravel source project".into());
    }
    if canonical_output.exists() {
        return Err(
            "report already exists; choose a new path so source reports are never overwritten"
                .into(),
        );
    }
    let mut files = Vec::new();
    collect_php_files(&root, &root, &mut files)?;
    files.sort();
    let mut deployment_sql_files = Vec::new();
    collect_deploy_sql_files(&root, &mut deployment_sql_files)?;
    deployment_sql_files.sort();
    if files.is_empty() && deployment_sql_files.is_empty() {
        return Err("no readable PHP source files or deployment SQL files were found outside excluded environment and dependency directories".into());
    }
    let total_source_bytes =
        files
            .iter()
            .chain(deployment_sql_files.iter())
            .try_fold(0_u64, |total, (path, _)| {
                fs::metadata(path)
                    .map(|metadata| total.saturating_add(metadata.len()))
                    .map_err(|error| format!("could not inspect {}: {error}", path.display()))
            })?;
    if total_source_bytes > MAX_TOTAL_SOURCE_BYTES {
        return Err("project exceeds the 256 MiB total PHP source analysis limit".into());
    }
    let mut findings = Vec::new();
    let mut graph = graph::GraphBuilder::default();
    let mut console_analyzer = console::ConsoleAnalyzer::default();
    let mut schema_analyzer = generation::SchemaAnalyzer::default();
    let composer = read_composer_metadata(&root)?;
    let php_ucfirst_ascii_semantics = composer
        .php_constraint
        .as_deref()
        .is_some_and(php_constraint_guarantees_ucfirst_ascii_semantics);
    let mut business_analyzer =
        business::BusinessAnalyzer::with_php_ucfirst_ascii_semantics(php_ucfirst_ascii_semantics);
    let mut service_helper_analyzer = service_helpers::ServiceHelperAnalyzer::default();
    let mut view_link_analyzer = view_links::ViewLinkAnalyzer::default();
    if let Some(version) = &composer.framework {
        let source_span = composer_framework_span(&root).unwrap_or(SourceSpan {
            start_byte: 0,
            end_byte: 0,
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 1,
        });
        findings.push(Finding { kind: "framework", status: "partial", file: "composer.json".into(), line: source_span.start_line, source_span: Some(source_span), name: version.clone(), summary: "Laravel version constraint found in Composer metadata; installed package resolution was not executed.".into(), next_step: "Confirm the installed framework and package versions in a controlled review." });
    }
    for (path, relative) in &files {
        let source = fs::read_to_string(path)
            .map_err(|_| format!("{} is not valid UTF-8 source", relative))?;
        graph.scan(relative, &source);
        console_analyzer.scan(relative, &source);
        schema_analyzer.scan(relative, &source);
        business_analyzer.scan(relative, &source);
        service_helper_analyzer.scan(relative, &source);
        view_link_analyzer.scan(relative, &source);
        let finding_count = findings.len();
        scan_source(relative, &source, &mut findings);
        if findings.len() == finding_count {
            findings.push(Finding {
                kind: "source_file",
                status: "unresolved",
                file: relative.clone(),
                line: 1,
                source_span: Some(SourceSpan::new(&source, 0..source.len())),
                name: "no supported pattern recognized".into(),
                summary: "This PHP file contains syntax outside the analyzer's recognized pattern set; its behavior is not described by this report.".into(),
                next_step: "Review this source file for routes, business rules, authorization, or side effects that need manual mapping.",
            });
        }
    }
    let mut deployment_sql_evidence = Vec::with_capacity(deployment_sql_files.len());
    for (path, relative) in &deployment_sql_files {
        let source = fs::read_to_string(path)
            .map_err(|_| format!("{relative} is not valid UTF-8 source"))?;
        schema_analyzer.scan_deploy_sql(relative, &source);
        deployment_sql_evidence.push(console::SqlScriptEvidence {
            path: relative.clone(),
            sha256: format!("{:x}", Sha256::digest(source.as_bytes())),
        });
    }
    if findings.is_empty() {
        findings.push(Finding {
            kind: "project",
            status: "unresolved",
            file: "—".into(),
            line: 0,
            source_span: None,
            name: "no recognized Laravel patterns".into(),
            summary: "No supported Laravel patterns were recognized; this does not prove that the project has no routes or business behavior.".into(),
            next_step: "Review project structure and add an analyzer fixture for any important unsupported pattern.",
        });
    }
    findings.sort_by(|left, right| {
        (&left.file, left.line, left.kind, &left.name).cmp(&(
            &right.file,
            right.line,
            right.kind,
            &right.name,
        ))
    });
    let unresolved = findings
        .iter()
        .filter(|finding| finding.status == "manual" || finding.status == "unresolved")
        .count();
    let partial = findings
        .iter()
        .filter(|finding| finding.status == "partial")
        .count();
    let structural_graph = graph.finish();
    let console_analysis =
        console_analyzer.finish_with_deployment_sql_files(&deployment_sql_evidence);
    let schema_generation = schema_analyzer.finish();
    let business_generation = business_analyzer.finish(&schema_generation);
    let service_generation = service_helper_analyzer.finish(&schema_generation, &structural_graph);
    let view_links = view_link_analyzer.finish(&structural_graph);
    let source_files = files
        .iter()
        .map(|(_, relative)| relative.clone())
        .collect::<Vec<_>>();
    let mut live_database_report = live_database.cloned();
    if let Some(database) = live_database_report.as_mut() {
        if database["status"] == "connected_read_only" {
            database["schema_comparison"] =
                live_database::compare_candidates(database, &schema_generation);
        }
    }
    let report_details = ReportDetails {
        source_files: &source_files,
        graph: &structural_graph,
        console_analysis: &console_analysis,
        schema_generation: &schema_generation,
        business_generation: &business_generation,
        service_generation: &service_generation,
        view_links: &view_links,
        live_database: live_database_report.as_ref(),
    };
    let rendered = if format == "json" {
        json_report(
            &root,
            &composer.framework,
            &composer.relevant_dependencies,
            &findings,
            &report_details,
        )
    } else {
        markdown_report(
            &root,
            &composer.framework,
            &composer.relevant_dependencies,
            &findings,
            &report_details,
        )
    };
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&canonical_output)
        .map_err(|error| format!("could not create report: {error}"))?;
    use std::io::Write;
    file.write_all(rendered.as_bytes()).map_err(|error| {
        let _ = fs::remove_file(&canonical_output);
        format!("could not write report: {error}")
    })?;
    Ok(AnalysisOutput {
        report_path: canonical_output,
        counts: FindingCounts {
            total: findings.len(),
            unresolved,
            partial,
        },
    })
}

fn resolve_new_report_path(report: &Path) -> Result<PathBuf, String> {
    let path = if report.is_absolute() {
        report.to_owned()
    } else {
        env::current_dir()
            .map_err(|error| error.to_string())?
            .join(report)
    };
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("report paths containing `..` are not accepted".into());
    }
    Ok(path)
}

fn collect_php_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(PathBuf, String)>,
) -> Result<(), String> {
    if files.len() > MAX_SOURCE_FILES {
        return Err("project exceeds the 10,000 PHP source file analysis limit".into());
    }
    let entries = fs::read_dir(directory).map_err(|error| {
        format!(
            "could not read project directory {}: {error}",
            directory.display()
        )
    })?;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("could not read a project directory entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("could not inspect {}: {error}", entry.path().display()))?;
        if file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let relative_path = path
            .strip_prefix(root)
            .map_err(|_| "source path escaped the project root".to_owned())?
            .to_string_lossy()
            .replace('\\', "/");
        if relative_path.split('/').count() > MAX_DIRECTORY_DEPTH {
            return Err("project directory nesting exceeds the 64-level analysis limit".into());
        }
        if name == ".env"
            || name.starts_with(".env.")
            || matches!(
                name.as_str(),
                ".git" | "vendor" | "node_modules" | "storage"
            )
            || relative_path == "bootstrap/cache"
            || relative_path.starts_with("bootstrap/cache/")
        {
            continue;
        }
        if file_type.is_dir() {
            collect_php_files(root, &path, files)?;
        } else if file_type.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("php"))
        {
            let metadata = fs::metadata(&path)
                .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
            if metadata.len() > MAX_SOURCE_FILE_BYTES {
                return Err(format!(
                    "{} exceeds the 2 MiB source file limit",
                    path.display()
                ));
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "source path escaped the project root".to_owned())?
                .to_string_lossy()
                .replace('\\', "/");
            if relative
                .split('/')
                .any(|part| part == ".env" || part.starts_with(".env."))
            {
                continue;
            }
            files.push((path, relative));
        }
        if files.len() > MAX_SOURCE_FILES {
            return Err("project exceeds the 10,000 PHP source file analysis limit".into());
        }
    }
    Ok(())
}

fn collect_deploy_sql_files(root: &Path, files: &mut Vec<(PathBuf, String)>) -> Result<(), String> {
    let directory = root.join("deploy/sql");
    let Ok(metadata) = fs::symlink_metadata(&directory) else {
        return Ok(());
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Ok(());
    }
    collect_deploy_sql_files_recursive(root, &directory, files)
}

fn collect_deploy_sql_files_recursive(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(PathBuf, String)>,
) -> Result<(), String> {
    if files.len() > MAX_SOURCE_FILES {
        return Err("project exceeds the 10,000 deployment SQL file analysis limit".into());
    }
    let entries = fs::read_dir(directory).map_err(|error| {
        format!(
            "could not read deployment SQL directory {}: {error}",
            directory.display()
        )
    })?;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("could not read deployment SQL entry: {error}"))?;
        let file_type = entry.file_type().map_err(|error| {
            format!(
                "could not inspect deployment SQL entry {}: {error}",
                entry.path().display()
            )
        })?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .map_err(|_| "deployment SQL source path escaped the project root".to_owned())?
            .to_string_lossy()
            .replace('\\', "/");
        if relative.split('/').count() > MAX_DIRECTORY_DEPTH {
            return Err(
                "deployment SQL directory nesting exceeds the 64-level analysis limit".into(),
            );
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == ".env" || name.starts_with(".env.") || name == ".git" {
            continue;
        }
        if file_type.is_dir() {
            collect_deploy_sql_files_recursive(root, &path, files)?;
        } else if file_type.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("sql"))
        {
            let metadata = fs::metadata(&path)
                .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
            if metadata.len() > MAX_SOURCE_FILE_BYTES {
                return Err(format!(
                    "{} exceeds the 2 MiB deployment SQL source file limit",
                    path.display()
                ));
            }
            files.push((path, relative));
        }
        if files.len() > MAX_SOURCE_FILES {
            return Err("project exceeds the 10,000 deployment SQL file analysis limit".into());
        }
    }
    Ok(())
}

fn read_composer_metadata(root: &Path) -> Result<ComposerMetadata, String> {
    let path = root.join("composer.json");
    if !path.is_file() {
        return Ok(ComposerMetadata {
            framework: None,
            relevant_dependencies: Vec::new(),
            php_constraint: None,
        });
    }
    let source =
        fs::read_to_string(path).map_err(|_| "composer.json is not valid UTF-8".to_owned())?;
    if source.len() as u64 > MAX_SOURCE_FILE_BYTES {
        return Err("composer.json exceeds the 2 MiB file limit".into());
    }
    let value: Value = zelyra_ast::json::parse(&source)
        .map_err(|_| "composer.json is not valid JSON".to_owned())?;
    let dependencies = ["require", "require-dev"]
        .into_iter()
        .filter_map(|key| value.get(key).and_then(Value::as_object))
        .flat_map(|map| map.iter())
        .filter_map(|(name, version)| {
            let version = version.as_str()?;
            Some((name.clone(), version.to_owned()))
        })
        .collect::<Vec<_>>();
    let framework = dependencies
        .iter()
        .find(|(name, _)| name == "laravel/framework")
        .map(|(_, version)| version.clone());
    let php_constraint = value
        .get("require")
        .and_then(Value::as_object)
        .and_then(|requires| requires.get("php"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let relevant = dependencies
        .into_iter()
        .filter(|(name, _)| {
            name.starts_with("laravel/")
                || name.starts_with("illuminate/")
                || name.starts_with("spatie/")
                || name == "stripe/stripe-php"
                || name == "php"
                || name.starts_with("inertiajs/")
                || name.starts_with("livewire/")
        })
        .map(|(name, version)| format!("{name} {version}"))
        .collect();
    Ok(ComposerMetadata {
        framework,
        relevant_dependencies: relevant,
        php_constraint,
    })
}

/// Return true only when every version admitted by this conservative subset of
/// Composer constraints is PHP 8.2 or newer. Unknown syntax is not proof.
fn php_constraint_guarantees_ucfirst_ascii_semantics(constraint: &str) -> bool {
    let constraint = constraint.trim();
    if constraint.is_empty() || constraint.contains("||") || constraint.contains('|') {
        return false;
    }
    let lower_bound = constraint
        .split(|character: char| character.is_ascii_whitespace() || character == ',')
        .find(|part| !part.is_empty())
        .unwrap_or_default();
    let version = lower_bound
        .strip_prefix('^')
        .or_else(|| lower_bound.strip_prefix('~'))
        .or_else(|| lower_bound.strip_prefix(">="))
        .or_else(|| lower_bound.strip_prefix('>'));
    let Some(version) = version else {
        return false;
    };
    let mut components = version.split('.');
    let Some(major) = components.next().and_then(|part| part.parse::<u32>().ok()) else {
        return false;
    };
    let Some(minor) = components.next().and_then(|part| {
        part.chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse::<u32>()
            .ok()
    }) else {
        return false;
    };
    major > 8 || (major == 8 && minor >= 2)
}

fn composer_framework_span(root: &Path) -> Option<SourceSpan> {
    let source = fs::read_to_string(root.join("composer.json")).ok()?;
    let key_start = source.find("\"laravel/framework\"")?;
    let colon = source[key_start..].find(':')? + key_start;
    let value_start = source[colon + 1..].find('"')? + colon + 1;
    let value_end = source[value_start + 1..].find('"')? + value_start + 2;
    Some(SourceSpan::new(&source, key_start..value_end))
}

fn scan_source(file: &str, source: &str, findings: &mut Vec<Finding>) {
    let source_map = PhpSourceMap::new(source);
    if file.starts_with("resources/views/") && file.ends_with(".blade.php") {
        let source_span = source_map.span(source, 0..source.len());
        findings.push(Finding { kind: "view", status: "manual", file: file.into(), line: source_span.start_line, source_span: Some(source_span), name: file.rsplit('/').next().unwrap_or(file).into(), summary: "Blade view found; directives, escaping, components, and client behavior are not translated.".into(), next_step: "Review rendering, output escaping, forms, and JavaScript behavior." });
        return;
    }
    let route = compiled_regex(
        r#"Route\s*::\s*(get|post|put|patch|delete|options|any|resource|apiResource)\s*\(\s*['\"]([^'\"]+)['\"]"#,
        "static route regex",
    );
    for capture in route.captures_iter(source) {
        add_match(findings, file, source, &source_map, capture.get(0).unwrap().range(), "route", "partial", capture.get(2).unwrap().as_str(), format!("Laravel {} route; chained middleware, policy checks, and runtime route resolution need review.", capture.get(1).unwrap().as_str()), "Review middleware and authorization for this route, then map it to a Zelyra route.");
    }
    let model = compiled_regex(
        r"(?m)\bclass\s+(\w+)\s+extends\s+(?:\\?Illuminate\\Database\\Eloquent\\)?Model\b",
        "static model regex",
    );
    for capture in model.captures_iter(source) {
        add_match(
            findings,
            file,
            source,
            &source_map,
            capture.get(0).unwrap().range(),
            "model",
            "partial",
            capture.get(1).unwrap().as_str(),
            "Eloquent model detected; casts, scopes, events, and traits are not inferred.",
            "Review casts, scopes, events, traits, and relationships before mapping the model.",
        );
    }
    let controller = compiled_regex(
        r"(?m)\bclass\s+(\w*Controller)\b",
        "static controller regex",
    );
    for capture in controller.captures_iter(source) {
        add_match(findings, file, source, &source_map, capture.get(0).unwrap().range(), "controller", "manual", capture.get(1).unwrap().as_str(), "Controller behavior can include authorization, side effects, and branching that static analysis does not transfer.", "Review each action's business rules and security checks manually.");
    }
    let migration = compiled_regex(
        r#"Schema\s*::\s*create\s*\(\s*['\"]([^'\"]+)['\"]"#,
        "static migration regex",
    );
    for capture in migration.captures_iter(source) {
        add_match(findings, file, source, &source_map, capture.get(0).unwrap().range(), "migration", "partial", capture.get(1).unwrap().as_str(), "Schema::create table found; column modifiers, indexes, foreign keys, and rollback semantics require review.", "Compare the complete migration and existing data before generating a Zelyra schema.");
    }
    let alteration = compiled_regex(
        r#"Schema\s*::\s*table\s*\(\s*['\"]([^'\"]+)['\"]"#,
        "static migration alteration regex",
    );
    for capture in alteration.captures_iter(source) {
        add_match(
            findings,
            file,
            source,
            &source_map,
            capture.get(0).unwrap().range(),
            "migration",
            "partial",
            capture.get(1).unwrap().as_str(),
            "Schema::table alteration found; the base schema, migration order, existing database state, and rollback behavior require review.",
            "Compare this alteration with the complete table history and verify whether its assumptions match the target database.",
        );
    }
    let relation = compiled_regex(
        r"(?s)function\s+(\w+)\s*\([^)]*\)\s*\{\s*return\s+\$this\s*->\s*(belongsTo|hasOne|hasMany|belongsToMany|hasManyThrough)\s*\(",
        "static relationship regex",
    );
    for capture in relation.captures_iter(source) {
        add_match(
            findings,
            file,
            source,
            &source_map,
            capture.get(0).unwrap().range(),
            "relationship",
            "partial",
            capture.get(1).unwrap().as_str(),
            format!(
                "Eloquent {} relation found; keys and tenant constraints need review.",
                capture.get(2).unwrap().as_str()
            ),
            "Verify foreign-key names, delete behavior, and tenant ownership.",
        );
    }
    let validation = compiled_regex(
        r"(?m)->validate\s*\(|Validator\s*::\s*make\s*\(",
        "static validation regex",
    );
    for matched in validation.find_iter(source) {
        add_match(findings, file, source, &source_map, matched.range(), "validation", "manual", "validation rules", "Validation entry point found; individual rules and conditional behavior are not evaluated.", "Translate each rule into a Zelyra form/API contract and test invalid input.");
    }
    let queued = compiled_regex(
        r"(?m)\bclass\s+(\w+)\s+implements\s+[^\n{]*ShouldQueue",
        "static job regex",
    );
    for capture in queued.captures_iter(source) {
        add_match(findings, file, source, &source_map, capture.get(0).unwrap().range(), "job", "manual", capture.get(1).unwrap().as_str(), "Queued job found; retries, idempotency, transaction boundaries, and failure handling are not transferred.", "Review delivery guarantees and side effects before mapping the job.");
    }
    let authorization = compiled_regex(
        r"(?i)(\$this\s*->\s*authorize\s*\(|\bGate\s*::\s*(?:authorize|allows|denies)\s*\(|\bcan\s*:\s*[A-Za-z0-9_.-]+|\bclass\s+\w+Policy\b)",
        "static authorization regex",
    );
    for matched in authorization.find_iter(source) {
        add_match(
            findings,
            file,
            source,
            &source_map,
            matched.range(),
            "authorization",
            "manual",
            "authorization or policy check",
            "Laravel authorization syntax found; this scan does not prove which records or actions the rule protects.",
            "Trace the authenticated principal, resource ownership, tenant boundary, and every create/read/update/delete path.",
        );
    }
    let tenant = compiled_regex(
        r"(?i)\b(?:tenant_id|account_id|organization_id|workspace_id)\b",
        "static tenant identifier regex",
    );
    for matched in tenant.find_iter(source) {
        add_match(
            findings,
            file,
            source,
            &source_map,
            matched.range(),
            "tenant_boundary",
            "manual",
            "tenant-scoped field",
            "A tenant-like identifier appears in source; the analyzer cannot establish that every query and relation is scoped to it.",
            "Review list, detail, mutation, relationship, and background-job access for cross-tenant ID manipulation.",
        );
    }
    let financial = compiled_regex(
        r"(?i)(\b(?:subtotal|grand_total|total|tax|vat|price|amount|discount)\b\s*(?:=|\+=)|->\s*sum\s*\()",
        "static financial expression regex",
    );
    for matched in financial.find_iter(source) {
        add_match(
            findings,
            file,
            source,
            &source_map,
            matched.range(),
            "financial_rule",
            "manual",
            "money or total calculation",
            "A money-like field or aggregate is calculated; decimal precision, rounding, tax rules, and currency are not inferred.",
            "Specify currency, decimal precision, rounding mode, tax basis, and transaction boundaries, then add numeric acceptance tests.",
        );
    }
    let file_operation = compiled_regex(
        r"(?i)(UploadedFile|response\s*\(\s*\)\s*->\s*download|->\s*(?:file|store|storeAs|download)\s*\()",
        "static file-operation regex",
    );
    for matched in file_operation.find_iter(source) {
        add_match(
            findings,
            file,
            source,
            &source_map,
            matched.range(),
            "file_access",
            "manual",
            "file upload or download",
            "A file upload/download operation appears; path confinement, content validation, authorization, and storage exposure are not analyzed.",
            "Review size and content limits, path traversal, tenant ownership, authorization, and download response headers.",
        );
    }
    let payment = compiled_regex(
        r"(?i)(\b(?:charge|paymentMethod|checkout|subscribe|subscription|refund)\s*\(|Cashier\s*::|Stripe\s*::)",
        "static payment regex",
    );
    for matched in payment.find_iter(source) {
        add_match(
            findings,
            file,
            source,
            &source_map,
            matched.range(),
            "payment_rule",
            "manual",
            "payment or billing operation",
            "A payment, checkout, or billing operation appears; payment processor contracts, webhooks, idempotency keys, and refunds are not analyzed.",
            "Map payment flows to explicit external webhooks and verified transaction records with numeric precision.",
        );
    }
    let ambiguity = compiled_regex(
        r"(?i)(DB\s*::\s*(?:raw|selectRaw|whereRaw|havingRaw|orderByRaw)\s*\(|eval\s*\(|call_user_func(?:_array)?\s*\(|->\s*action\s*\(\s*\$)",
        "static ambiguity regex",
    );
    for matched in ambiguity.find_iter(source) {
        add_match(
            findings,
            file,
            source,
            &source_map,
            matched.range(),
            "ambiguity",
            "manual",
            "dynamic or unparsed SQL/execution ambiguity",
            "Dynamic dispatch, raw SQL expression, or runtime execution found; static type safety and dialect compatibility cannot be verified.",
            "Replace raw SQL or dynamic dispatch with structured Zelyra typed queries and explicit routes.",
        );
    }
    let env_call = compiled_regex(
        r#"(?i)\benv\s*\(\s*['"]([^'"]+)['"]"#,
        "static env call regex",
    );
    for capture in env_call.captures_iter(source) {
        let var_name = capture.get(1).unwrap().as_str();
        add_match(
            findings,
            file,
            source,
            &source_map,
            capture.get(0).unwrap().range(),
            "environment_access",
            "manual",
            format!("env('{var_name}') access"),
            "Direct env() access in source code; environment variables outside config files are fragile and values are never read or copied into reports.",
            "Define explicit Zelyra configuration or module capabilities instead of runtime env lookups.",
        );
    }
    for (needle, kind, label) in [
        ("Mail::", "integration", "mail integration"),
        ("Notification::", "integration", "notification integration"),
        ("Http::", "integration", "HTTP integration"),
        ("Storage::", "integration", "file storage integration"),
        ("DB::transaction", "transaction", "database transaction"),
    ] {
        let mut offset = 0;
        while let Some(index) = source[offset..].find(needle) {
            let start = offset + index;
            let matched = start..start + needle.len();
            add_match(findings, file, source, &source_map, matched, kind, "manual", label, format!("{label} call found; credentials, retries, transactions, and error handling require review."), "Identify external effects and their failure/rollback behavior.");
            offset = start + needle.len();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn add_match(
    findings: &mut Vec<Finding>,
    file: &str,
    source: &str,
    source_map: &PhpSourceMap,
    matched: Range<usize>,
    kind: &'static str,
    status: &'static str,
    name: impl Into<String>,
    summary: impl Into<String>,
    next_step: &'static str,
) {
    if !source_map.is_code(matched.start) {
        return;
    }
    let source_span = source_map.span(source, matched);
    let line = source_span.start_line;
    findings.push(Finding {
        kind,
        status,
        file: file.into(),
        line,
        source_span: Some(source_span),
        name: name.into(),
        summary: summary.into(),
        next_step,
    });
}

fn json_report(
    root: &Path,
    framework: &Option<String>,
    packages: &[String],
    findings: &[Finding],
    details: &ReportDetails<'_>,
) -> String {
    let ReportDetails {
        source_files,
        graph,
        console_analysis,
        schema_generation,
        business_generation,
        service_generation,
        view_links,
        live_database,
    } = details;
    let counts = report_counts(findings);
    let findings = findings
        .iter()
        .map(|finding| {
            let span = finding.source_span.map(|span| {
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
            });
            let finding_id = finding.source_span.map(|span| {
                format!(
                    "laravel:{}:{}:{}",
                    finding.kind, finding.file, span.start_byte
                )
            });
            json!({
                "finding_id": finding_id,
                "mapping_id": format!("laravel/{}", finding.kind),
                "kind": finding.kind,
                "status": finding.status,
                "classification": finding_classification(finding.status),
                "effect_categories": finding_effect_categories(finding.kind),
                "file": finding.file,
                "line": finding.line,
                "source_span": span,
                "name": finding.name,
                "summary": finding.summary,
                "next_step": finding.next_step
            })
        })
        .collect::<Vec<_>>();
    let source_files = source_file_inventory(source_files, &findings, graph, service_generation);
    let dependencies = packages
        .iter()
        .filter_map(|dependency| {
            let (name, constraint) = dependency.split_once(' ')?;
            Some(json!({
                "name": name,
                "constraint": constraint,
                "source": "composer.json",
                "installed_version": null,
                "resolution": "not_run"
            }))
        })
        .collect::<Vec<_>>();
    serde_json::to_string_pretty(&json!({
        "schema_version": "1",
        "analyzer": format!("Zelyra Laravel Migrator {ANALYZER_VERSION}"),
        "mapping_ruleset": MAPPING_RULESET_VERSION,
        "source_project": root.file_name().and_then(|name| name.to_str()).unwrap_or("Laravel project"),
        "laravel_constraint": framework,
        "laravel_packages": packages,
        "dependencies": dependencies,
        "execution": if live_database.is_some() { "static_source_scan_plus_read_only_database_metadata" } else { "static_read_only" },
        "env_files_read": live_database.is_some(),
        "live_database_inspection": live_database,
        "effect_categories_are_indicators": true,
        "summary": counts,
        "findings": findings,
        "source_file_inventory": source_files,
        "graph": graph,
        "console_analysis": console_analysis,
        "schema_generation": schema_generation,
        "business_logic_generation": business_generation,
        "service_generation": service_generation,
        "view_links": view_links
    })).expect("report JSON is serializable")
}

fn source_file_inventory(
    source_files: &[String],
    findings: &[Value],
    graph: &Value,
    service_generation: &Value,
) -> Vec<Value> {
    let mut files = source_files.to_vec();
    files.sort();
    files.dedup();

    files
        .into_iter()
        .map(|file| {
            let (category, action) = source_file_migration_action(&file);
            let file_findings = findings
                .iter()
                .filter(|finding| finding["file"].as_str() == Some(file.as_str()))
                .collect::<Vec<_>>();
            let has_unrecognized_source = file_findings.iter().any(|finding| {
                finding["kind"] == "source_file" && finding["status"] == "unresolved"
            });
            let finding_kinds = file_findings
                .iter()
                .filter_map(|finding| finding["kind"].as_str())
                .collect::<BTreeSet<_>>();
            let finding_statuses = file_findings
                .iter()
                .filter_map(|finding| finding["status"].as_str())
                .collect::<BTreeSet<_>>();
            let nodes = graph["nodes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|node| node["file"].as_str() == Some(file.as_str()))
                .map(|node| {
                    json!({
                        "id": node["id"],
                        "kind": node["kind"],
                        "name": node["name"],
                        "source_span": node["source_span"]
                    })
                })
                .collect::<Vec<_>>();
            let references = graph["edges"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|edge| {
                    let from = &edge["from"];
                    let to = &edge["to"];
                    let from_file = from["file"].as_str();
                    let to_file = to["file"].as_str();
                    if to_file == Some(file.as_str()) {
                        Some(json!({
                            "direction": "referenced_by",
                            "kind": edge["kind"],
                            "status": edge["status"],
                            "counterpart_file": from["file"],
                            "counterpart_name": from["name"],
                            "reason": edge["reason"]
                        }))
                    } else if from_file == Some(file.as_str()) {
                        Some(json!({
                            "direction": "references",
                            "kind": edge["kind"],
                            "status": edge["status"],
                            "counterpart_file": to["file"],
                            "counterpart_name": to["name"].as_str().or(edge["target"]["name"].as_str()),
                            "target_method": edge["target"]["method"],
                            "reason": edge["reason"]
                        }))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let reference_count = references.len();
            let translated_helpers = service_generation["candidates"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|candidate| candidate["source_file"].as_str() == Some(file.as_str()))
                .map(|candidate| json!({
                    "kind": "pure_service_helper",
                    "class": candidate["class"],
                    "method": candidate["method"],
                    "function": candidate["function"],
                    "target_module": "src/imported_services.zyl",
                    "source_span": candidate["source_span"],
                    "static_call_sites": candidate["static_call_sites"],
                    "call_sites_wired": false
                }))
                .collect::<Vec<_>>();
            let translation_status = if translated_helpers.is_empty() {
                "not_translated"
            } else {
                "partially_translated_helpers_call_sites_require_review"
            };
            json!({
                "file": file,
                "source_pattern_status": if has_unrecognized_source { "no_supported_pattern" } else if file_findings.is_empty() { "no_finding_emitted" } else { "findings_emitted_coverage_not_proven" },
                "category": category,
                "proposed_action": action,
                "finding_count": file_findings.len(),
                "finding_kinds": finding_kinds,
                "finding_statuses": finding_statuses,
                "graph_nodes": nodes,
                "static_references": references,
                "static_reference_count": reference_count,
                "translation_status": translation_status,
                "translated_helper_count": translated_helpers.len(),
                "translated_helpers": translated_helpers,
                "translated": false
            })
        })
        .collect()
}

fn source_file_migration_action(file: &str) -> (&'static str, &'static str) {
    if file.starts_with("tests/") {
        (
            "test",
            "port_behavioral_coverage_to_zelyra_tests_or_document_exclusion",
        )
    } else if file.starts_with("app/Services/") {
        (
            "service",
            "translate_referenced_behavior_to_reusable_zelyra_module",
        )
    } else if file.starts_with("app/Console/Commands/") {
        (
            "console_command",
            "translate_handler_and_register_zelyra_command",
        )
    } else if file.starts_with("app/Http/Middleware/") {
        (
            "middleware",
            "translate_security_and_request_behavior_to_zelyra",
        )
    } else if file.starts_with("app/Mail/") {
        (
            "mail_component",
            "translate_template_and_delivery_flow_to_zelyra_mail",
        )
    } else if file.starts_with("app/Models/") {
        (
            "model",
            "map_model_behavior_and_relations_to_zelyra_types_and_schema",
        )
    } else if file.starts_with("database/") {
        (
            "database_lifecycle",
            "review_as_schema_migration_fixture_or_seed_data",
        )
    } else if file.starts_with("config/") {
        (
            "project_configuration",
            "map_required_settings_to_zelyra_project_configuration",
        )
    } else if file.starts_with("bootstrap/") || file == "public/index.php" {
        (
            "laravel_runtime_entry",
            "replace_with_zelyra_runtime_and_preserve_required_wiring",
        )
    } else if file.starts_with("scripts/") {
        (
            "support_script",
            "review_usage_and_port_or_document_exclusion",
        )
    } else {
        (
            "application_source",
            "inspect_dependencies_and_translate_required_behavior",
        )
    }
}

fn render_source_file_plan_markdown(plan: &Value) -> String {
    let entries = plan.as_array().map(Vec::as_slice).unwrap_or_default();
    let unrecognized_count = entries
        .iter()
        .filter(|entry| entry["source_pattern_status"] == "no_supported_pattern")
        .count();
    let mut output = format!(
        "# Laravel source file migration plan\n\n{} PHP source file(s) were scanned; {} have no recognized source-analysis pattern. Every entry must be translated, mapped to a Zelyra runtime/module/test/configuration, or explicitly excluded with a reviewed reason. Static graph references are evidence for prioritization; they are not translated implementations.\n\n| File | Analysis status | Category | Proposed action | Static references | Translation status |\n|---|---|---|---|---:|---|\n",
        entries.len(), unrecognized_count
    );
    for entry in entries {
        use std::fmt::Write as _;
        let source_path =
            markdown_code_span(entry["file"].as_str().unwrap_or("unknown")).replace('|', "&#124;");
        let _ = writeln!(
            output,
            "| {} | `{}` | `{}` | `{}` | {} | `{}` |",
            source_path,
            entry["source_pattern_status"].as_str().unwrap_or("unknown"),
            entry["category"].as_str().unwrap_or("unknown"),
            entry["proposed_action"].as_str().unwrap_or("review"),
            entry["static_reference_count"].as_u64().unwrap_or(0),
            entry["translation_status"]
                .as_str()
                .unwrap_or("not_translated")
        );
    }
    output
}

fn finding_classification(status: &str) -> &'static str {
    match status {
        "direct" => "automatic",
        "partial" => "assisted",
        _ => "manual",
    }
}

fn migration_console_inventory(console: &Value) -> Value {
    let commands = console["commands"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|command| {
            json!({
                "id": command["id"],
                "name": command["command_name"],
                "origin": command["origin"],
                "file": command["file"],
                "line": command["source_span"]["start"]["line"],
                "parameters": command["parameters"],
                "handler_resolution": command["implementation"]["resolution"],
                "handler_method": command["implementation"]["method"],
                "sql_script_discovery": command["implementation"]["sql_script_discovery"],
                "handler_line": command["implementation"]["source_span"]["start"]["line"],
                "migration_status": "manual_handler_translation_required"
            })
        })
        .collect::<Vec<_>>();
    let schedules = console["scheduled_tasks"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|schedule| {
            let target = schedule["command_name"]
                .as_str()
                .or_else(|| schedule["target"]["class_name"].as_str())
                .or_else(|| schedule["target"]["target_name"].as_str());
            json!({
                "id": schedule["id"],
                "kind": schedule["kind"],
                "target": target,
                "file": schedule["file"],
                "line": schedule["source_span"]["start"]["line"],
                "cadence": schedule["modifiers"]["cadence"],
                "timezone": schedule["modifiers"]["timezone"],
                "without_overlapping": schedule["modifiers"]["without_overlapping"],
                "run_in_background": schedule["modifiers"]["run_in_background"],
                "resolution": schedule["resolution"],
                "migration_status": "manual_schedule_and_handler_review_required"
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema_version": "3",
        "commands": commands,
        "schedules": schedules
    })
}

fn render_console_tasks_todo(inventory: &Value) -> String {
    let commands = inventory["commands"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let schedules = inventory["schedules"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut output = format!(
        "# Laravel command and schedule migration checklist\n\n- Commands found: {}\n- Scheduled tasks found: {}\n- Imported PHP handlers: no\n- Executed during analysis: no\n\nThis checklist contains statically recognized metadata only. It does not copy PHP bodies, shell command text, or command-line arguments. Verify Laravel registration and behavior in the source project, then implement each handler in Zelyra and create reviewed `command`/`schedule` declarations. Scheduling and deployment remain host responsibilities. Machine-readable entries are in `GENERATION_SELECTION.json` under `laravel_console_inventory`.\n\n## Command definitions\n\n",
        commands.len(),
        schedules.len()
    );
    if commands.is_empty() {
        output.push_str("No command definitions were statically recognized.\n\n");
    } else {
        for command in commands {
            let method = command["handler_method"]
                .as_str()
                .map(|method| format!(" (method {})", markdown_code_span(method)))
                .unwrap_or_default();
            let script_files = command["sql_script_discovery"]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|discovery| discovery["files"].as_array().into_iter().flatten())
                .filter_map(Value::as_str)
                .map(markdown_code_span)
                .collect::<Vec<_>>();
            let script_note = if script_files.is_empty() {
                String::new()
            } else {
                format!(
                    "; statically matched deployment SQL: {}",
                    script_files.join(", ")
                )
            };
            let _ = writeln!(
                output,
                "- [ ] {} ({}) at {}:{}; handler resolution: {}{}{}; translate and review manually.",
                markdown_code_span(command["name"].as_str().unwrap_or("dynamic/unresolved")),
                markdown_code_span(command["origin"].as_str().unwrap_or("unknown origin")),
                markdown_code_span(command["file"].as_str().unwrap_or("unknown")),
                command["line"].as_u64().unwrap_or(0),
                markdown_code_span(command["handler_resolution"].as_str().unwrap_or("unknown")),
                method,
                script_note
            );
        }
        output.push('\n');
    }
    output.push_str("## Scheduled tasks\n\n");
    if schedules.is_empty() {
        output.push_str("No scheduled tasks were statically recognized.\n");
    } else {
        for schedule in schedules {
            let cadence = schedule["cadence"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|item| {
                    let name = item["name"].as_str().unwrap_or("unknown");
                    let arguments = item["literal_arguments"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>();
                    if arguments.is_empty() {
                        name.to_owned()
                    } else {
                        format!("{name}({})", arguments.join(", "))
                    }
                })
                .collect::<Vec<_>>()
                .join(" → ");
            let target = schedule["target"].as_str().unwrap_or("dynamic/unresolved");
            let timezone = schedule["timezone"].as_str().unwrap_or("host default");
            let _ = writeln!(
                output,
                "- [ ] {} ({}) at {}:{}; cadence: {}; timezone: {}; resolution: {}; review handler, overlap behavior, and deployment manually.",
                markdown_code_span(target),
                markdown_code_span(schedule["kind"].as_str().unwrap_or("unknown task")),
                markdown_code_span(schedule["file"].as_str().unwrap_or("unknown")),
                schedule["line"].as_u64().unwrap_or(0),
                markdown_code_span(if cadence.is_empty() { "not statically recognized" } else { &cadence }),
                markdown_code_span(timezone),
                markdown_code_span(schedule["resolution"].as_str().unwrap_or("unknown"))
            );
        }
    }
    output
}

fn markdown_code_span(value: &str) -> String {
    let safe = value
        .chars()
        .map(|character| match character {
            '`' => 'ˋ',
            character if character.is_control() => ' ',
            character => character,
        })
        .collect::<String>();
    format!("`{safe}`")
}

fn business_parameter_summary(candidate: &Value) -> String {
    candidate["parameters"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|parameter| {
            format!(
                "{} -> {}: {}",
                parameter["source_field"].as_str().unwrap_or("unknown"),
                parameter["function_parameter"]
                    .as_str()
                    .unwrap_or("unknown"),
                parameter["zelyra_type"].as_str().unwrap_or("unknown")
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn business_schema_evidence_summary(candidate: &Value) -> String {
    let source = candidate["schema_evidence_source"]
        .as_str()
        .unwrap_or("not_applicable");
    let entries = candidate["schema_field_evidence"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|entry| {
            format!(
                "{}.{} at {}:{}",
                entry["table"].as_str().unwrap_or("unknown_table"),
                entry["source_column_name"]
                    .as_str()
                    .unwrap_or("unknown_field"),
                entry["source_file"].as_str().unwrap_or("unknown_source"),
                entry["source_span"]["start"]["line"].as_u64().unwrap_or(0)
            )
        })
        .collect::<Vec<_>>();
    if entries.is_empty() {
        if source == "not_applicable" {
            "none (literal-only predicate)".into()
        } else {
            source.to_owned()
        }
    } else {
        format!("{source}: {}", entries.join(", "))
    }
}

fn finding_effect_categories(kind: &str) -> &'static [&'static str] {
    match kind {
        "route" => &["http", "authorization_review"],
        "controller" => &["business_logic", "side_effect_review"],
        "model" | "relationship" | "migration" | "transaction" => &["database"],
        "authorization" | "tenant_boundary" => &["security"],
        "validation" => &["input_validation"],
        "financial_rule" | "payment_rule" => &["financial", "external_service_review"],
        "file_access" => &["filesystem", "security_review"],
        "integration" => &["external_service", "network_or_mail_review"],
        "job" => &["background_execution", "side_effect_review"],
        "environment_access" => &["configuration", "secret_review"],
        "ambiguity" => &["dynamic_execution_or_sql", "security_review"],
        "view" => &["html_rendering", "escaping_review"],
        _ => &["behavior_unknown"],
    }
}

fn markdown_report(
    root: &Path,
    framework: &Option<String>,
    packages: &[String],
    findings: &[Finding],
    details: &ReportDetails<'_>,
) -> String {
    let ReportDetails {
        source_files: _,
        graph,
        console_analysis,
        schema_generation,
        business_generation,
        service_generation: _,
        view_links: _,
        live_database,
    } = details;
    let counts = report_counts(findings);
    let method = if live_database.is_some() {
        "static source scan plus explicitly requested read-only database metadata inspection; `.env` connection settings were read; PHP and Artisan were not run; no application rows were read"
    } else {
        "static, read-only source scan; PHP and Artisan were not run; `.env` files were not read"
    };
    let mut report = format!("# Zelyra Laravel migration analysis\n\n- Analyzer: Zelyra Laravel Migrator {ANALYZER_VERSION}\n- Mapping ruleset: {MAPPING_RULESET_VERSION}\n- Project: {}\n- Laravel constraint: {}\n- Laravel packages: {}\n- Method: {method}.\n- Findings: {} total · {} partial · {} manual · {} unresolved\n\n", root.file_name().and_then(|name| name.to_str()).unwrap_or("Laravel project"), framework.as_deref().unwrap_or("not detected"), if packages.is_empty() { "none detected".into() } else { packages.join(", ") }, findings.len(), counts["partial"], counts["manual"], counts["unresolved"]);
    if let Some(database) = live_database {
        if database["status"] == "unavailable" {
            let _ = writeln!(report, "## Live database schema metadata\n\n- Status: `unavailable`\n- Reason: {}\n- Credentials recorded: no\n- Application rows read: no\n\nThe static source analysis completed without live database evidence. Retry this option when the configured database host is reachable.\n", database["error"].as_str().unwrap_or("database inspection failed"));
        } else {
            let _ = writeln!(report, "## Live database schema metadata\n\n- Status: `{}`\n- Driver: `{}`\n- Tables: {}\n- Columns: {}\n- Credentials recorded: no\n- Application rows read: no\n\nThis inventory is evidence for review; it does not by itself generate or apply migrations.\n", database["status"], database["driver"], database["table_count"], database["column_count"]);
        }
    }
    report.push_str("| Status | Kind | Source location | Finding |\n|---|---|---|---|\n");
    for finding in findings {
        let summary = finding.summary.replace('|', "\\|").replace('\n', " ");
        let _ = writeln!(
            report,
            "| {} | {} | `{}:{}` | {} |",
            finding.status, finding.kind, finding.file, finding.line, summary
        );
        let _ = writeln!(report, "|  | Next step |  | {} |", finding.next_step);
    }
    if findings.is_empty() {
        report.push_str("| unresolved | project | — | No supported Laravel patterns were recognized. This does not prove that the project has no routes or business behavior. |\n");
    }
    let graph_summary = &graph["summary"];
    let _ = writeln!(
        report,
        "\n## Statically linked project graph\n\n- Nodes: {}\n- Resolved edges: {}\n- Unresolved edges: {}\n\nThese links only represent statically recognized source patterns; dynamic registration, runtime behavior, and inferred conventions are not resolved.\n",
        graph_summary["nodes"].as_u64().unwrap_or(0),
        graph_summary["resolved_edges"].as_u64().unwrap_or(0),
        graph_summary["unresolved_edges"].as_u64().unwrap_or(0)
    );
    let _ = writeln!(
        report,
        "\n## Console commands and schedules\n\n- Command definitions: {}\n- Scheduled command references: {}\n- Other statically recognized scheduled tasks: {}\n- References resolved to one static definition: {}\n- Definitions and schedules executed: no\n\nCommand bodies are not translated. Scheduled jobs, callbacks, and shell commands are inventoried but not executed or translated. Static source links do not prove Laravel runtime registration or behavioral parity. Review each entry before creating a Zelyra command or deployment schedule.\n",
        console_analysis["summary"]["commands"].as_u64().unwrap_or(0),
        console_analysis["summary"]["scheduled_commands"].as_u64().unwrap_or(0),
        console_analysis["summary"]["scheduled_tasks"].as_u64().unwrap_or(0)
            - console_analysis["summary"]["scheduled_commands"].as_u64().unwrap_or(0),
        console_analysis["summary"]["resolved_schedule_definitions"].as_u64().unwrap_or(0)
    );
    for command in console_analysis["commands"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let handler = command
            .get("implementation")
            .map(|implementation| {
                if implementation["resolution"] == "resolved_unique" {
                    format!(
                        "; local handler `{}` at `{}:{}`",
                        implementation["method"].as_str().unwrap_or("unknown"),
                        command["file"].as_str().unwrap_or("unknown"),
                        implementation["source_span"]["start"]["line"]
                            .as_u64()
                            .unwrap_or(0)
                    )
                } else {
                    format!(
                        "; handler `{}`",
                        implementation["resolution"]
                            .as_str()
                            .unwrap_or("unresolved")
                    )
                }
            })
            .unwrap_or_default();
        let _ = writeln!(
            report,
            "- Command `{}` ({}) at `{}:{}`; parameters: `{}`{}.",
            command["command_name"]
                .as_str()
                .unwrap_or("dynamic/unresolved"),
            command["origin"].as_str().unwrap_or("unknown origin"),
            command["file"].as_str().unwrap_or("unknown"),
            command["source_span"]["start"]["line"]
                .as_u64()
                .unwrap_or(0),
            command["parameters"],
            handler
        );
    }
    for schedule in console_analysis["scheduled_tasks"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let target = schedule["command_name"]
            .as_str()
            .or_else(|| schedule["target"]["class_name"].as_str())
            .or_else(|| schedule["target"]["target_name"].as_str())
            .unwrap_or("dynamic/unresolved");
        let _ = writeln!(report, "- Scheduled `{}` (`{}`) at `{}:{}`; cadence: `{}`; timezone: `{}`; overlap lock: {}; background: {}; unmodeled modifiers: `{}`; resolution: `{}`.",
            target,
            schedule["kind"].as_str().unwrap_or("unknown task"),
            schedule["file"].as_str().unwrap_or("unknown"),
            schedule["source_span"]["start"]["line"].as_u64().unwrap_or(0),
            schedule["modifiers"]["cadence"],
            schedule["modifiers"]["timezone"],
            schedule["modifiers"]["without_overlapping"],
            schedule["modifiers"]["run_in_background"],
            schedule["modifiers"]["unmodeled_calls"],
            schedule["resolution"].as_str().unwrap_or("unknown"));
    }
    let _ = writeln!(
        report,
        "\n## Lossless schema candidates\n\n- Candidate tables: {}\n- Rejected schema declarations: {}\n- Database changes applied: no\n\nCandidates require one static `Schema::create` declaration, optionally extended by additive static `Schema::table` callbacks in migration `up()` methods. `down()` methods are excluded. Unsupported or lossy declarations are reported with source positions and are not partially generated.\n",
        schema_generation["generated_table_count"].as_u64().unwrap_or(0),
        schema_generation["rejected"].as_array().map_or(0, Vec::len)
    );
    let _ = writeln!(
        report,
        "\n## Narrow pure predicate translations\n\n- Generated helpers: {}\n- Rejected candidates: {}\n\nOnly public zero-argument boolean methods with a literal `in_array` return, a schema-typed conjunction of `!empty($this->field)` checks, a static boolean lookup, or a static active-status/nullable-`isFuture()` pattern are translated. Time-dependent helpers receive an explicit reference timestamp. Generated helpers require explicit field arguments and do not import model casts, call sites, authorization, or workflows. Review each source span and schema assumption before use.\n",
        business_generation["generated_function_count"].as_u64().unwrap_or(0),
        business_generation["rejected"].as_array().map_or(0, Vec::len)
    );
    for candidate in business_generation["candidates"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let _ = writeln!(
            report,
            "- Generated `{}` from `{}:{}` for `{}::{}`; explicit arguments: `{}`; schema evidence: `{}`.",
            candidate["function"].as_str().unwrap_or("unknown"),
            candidate["file"].as_str().unwrap_or("unknown"),
            candidate["source_span"]["start"]["line"]
                .as_u64()
                .unwrap_or(0),
            candidate["class"].as_str().unwrap_or("unknown"),
            candidate["method"].as_str().unwrap_or("unknown"),
            business_parameter_summary(candidate),
            business_schema_evidence_summary(candidate)
        );
    }
    for rejected in business_generation["rejected"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let _ = writeln!(
            report,
            "- Not generated `{}` from `{}:{}`: {}.",
            rejected["name"].as_str().unwrap_or("unknown predicate"),
            rejected["file"].as_str().unwrap_or("unknown"),
            rejected["source_span"]["start"]["line"]
                .as_u64()
                .unwrap_or(0),
            rejected["reason"].as_str().unwrap_or("unsupported syntax")
        );
    }
    report.push_str("\n## Scope limits\n\nThis report inventories recognized syntax. It does not prove feature or security parity. Unrecognized syntax and dynamic behavior remain outside the report. Review authorization, tenant boundaries, transactions, money, file handling, and integration side effects before implementing the mapped behavior in Zelyra. The source project and its database were not changed.\n");
    report
}

fn report_counts(findings: &[Finding]) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::from([
        ("direct", 0),
        ("partial", 0),
        ("manual", 0),
        ("unresolved", 0),
    ]);
    for finding in findings {
        if let Some(count) = counts.get_mut(finding.status) {
            *count += 1;
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_task_checklist_keeps_source_metadata_on_one_safe_markdown_line() {
        let inventory = json!({
            "commands": [{
                "name": "danger`\n## injected heading",
                "origin": "inline\rcommand",
                "file": "routes/console.php\n- [ ] forged item",
                "line": 12,
                "handler_resolution": "source_span_only",
                "handler_method": "handle`\nforged"
            }],
            "schedules": [{
                "target": "danger`\n## second heading",
                "kind": "scheduled_callback",
                "file": "routes/console.php\n- [ ] forged schedule",
                "line": 18,
                "cadence": [{"name": "daily\n## cadence", "literal_arguments": ["`\nvalue"]}],
                "timezone": "Europe/Berlin\n- [ ] forged timezone",
                "resolution": "static_callback\nforged"
            }]
        });

        let checklist = render_console_tasks_todo(&inventory);
        assert_eq!(
            checklist
                .lines()
                .filter(|line| line.starts_with("- [ ]"))
                .count(),
            2,
            "untrusted values must not add checklist rows: {checklist}"
        );
        assert!(checklist.contains("dangerˋ ## injected heading"));
        assert!(checklist.contains("routes/console.php - [ ] forged item"));
        assert!(!checklist
            .lines()
            .any(|line| line.starts_with("## injected")));
        assert!(!checklist
            .lines()
            .any(|line| line.starts_with("- [ ] forged")));
    }

    #[test]
    fn php_ucfirst_semantics_require_a_proven_composer_lower_bound() {
        for constraint in ["^8.2", "^8.3", "~8.2.0", ">=8.2", ">=8.2.0 <9.0"] {
            assert!(
                php_constraint_guarantees_ucfirst_ascii_semantics(constraint),
                "expected {constraint:?} to prove PHP 8.2+"
            );
        }
        for constraint in ["^8.1", "^7.4 || ^8.3", "^7.4", "*", "8.2", "^8", ">= 8.2"] {
            assert!(
                !php_constraint_guarantees_ucfirst_ascii_semantics(constraint),
                "expected {constraint:?} not to prove PHP 8.2+"
            );
        }
    }

    #[test]
    fn targeted_schema_selection_includes_transitive_relation_dependencies() {
        let report = json!({
            "candidates": [
                {"table":"customers", "columns":[], "candidate_source":"table customers {}"},
                {"table":"orders", "columns":[{"relation_table":"customers"}], "candidate_source":"table orders {}"},
                {"table":"order_lines", "columns":[{"relation_table":"orders"}], "candidate_source":"table order_lines {}"},
                {"table":"events", "columns":[], "candidate_source":"table events {}"}
            ]
        });
        let selection = select_schema_candidates(&report, &["order_lines".into()]).unwrap();
        assert_eq!(selection.requested_names, ["order_lines"]);
        assert_eq!(
            selection.included_names,
            ["customers", "order_lines", "orders"]
        );
        assert_eq!(
            selection
                .candidates
                .iter()
                .filter_map(|candidate| candidate["table"].as_str())
                .collect::<Vec<_>>(),
            ["customers", "orders", "order_lines"]
        );
    }

    #[test]
    fn selected_tables_trace_conventional_model_uses_to_resolved_routes_without_claiming_import() {
        let graph = json!({
            "nodes": [{
                "id": "route:routes/web.php:42",
                "kind": "route",
                "name": "get /customers",
                "file": "routes/web.php",
                "source_span": { "start": { "line": 4 } },
                "route_context": { "inherited_middleware": ["auth"] }
            }, {
                "id": "route:routes/web.php:99",
                "kind": "route",
                "name": "get /invoices/new",
                "file": "routes/web.php",
                "source_span": { "start": { "line": 9 } },
                "route_context": { "inherited_middleware": ["auth"] }
            }, {
                "id": "route:routes/web.php:120",
                "kind": "route",
                "name": "get /customers/{customer}",
                "file": "routes/web.php",
                "source_span": { "start": { "line": 12 } },
                "route_context": { "inherited_middleware": ["auth"] }
            }],
            "edges": [
                {
                    "kind": "controller_model_use",
                    "status": "resolved",
                    "from": { "name": "CustomerController@index" },
                    "to": { "name": "Customer" }
                },
                {
                    "kind": "route_controller_action",
                    "status": "resolved",
                    "from": { "id": "route:routes/web.php:42" },
                    "to": {
                        "name": "CustomerController@index",
                        "file": "app/Http/Controllers/CustomerController.php",
                        "source_span": { "start": { "line": 8 } }
                    }
                },
                {
                    "kind": "route_controller_action",
                    "status": "resolved",
                    "from": { "id": "route:routes/web.php:120" },
                    "to": {
                        "name": "CustomerController@show",
                        "file": "app/Http/Controllers/CustomerController.php",
                        "source_span": { "start": { "line": 25 } }
                    }
                },
                {
                    "kind": "controller_model_use",
                    "status": "resolved",
                    "from": { "name": "InvoiceController@create" },
                    "to": { "name": "Customer" }
                },
                {
                    "kind": "route_controller_action",
                    "status": "resolved",
                    "from": { "id": "route:routes/web.php:99" },
                    "to": {
                        "name": "InvoiceController@create",
                        "file": "app/Http/Controllers/InvoiceController.php",
                        "source_span": { "start": { "line": 19 } }
                    }
                }
            ]
        });
        let candidates = route_candidates_for_tables(&graph, &["customers".into()]);
        assert_eq!(candidates["count"], 2);
        assert_eq!(candidates["model_consumer_count"], 1);
        assert_eq!(candidates["routes"][0]["route"], "get /customers");
        assert_eq!(
            candidates["routes"][1]["route"],
            "get /customers/{customer}"
        );
        assert_eq!(
            candidates["routes"][1]["resolution"],
            "statically_resolved_route_with_conventional_resource_controller_evidence"
        );
        assert_eq!(
            candidates["model_consumer_routes"][0]["route"],
            "get /invoices/new"
        );
        assert_eq!(candidates["routes"][0]["table"], "customers");
        assert_eq!(
            candidates["routes"][0]["translation_status"],
            "analysis_only_route_handler_view_and_security_behavior_not_generated"
        );
        assert_eq!(
            conventional_model_name("customer_groups").as_deref(),
            Some("CustomerGroup")
        );
        assert_eq!(conventional_model_name("people"), None);
    }

    #[test]
    fn targeted_schema_selection_rejects_invalid_duplicate_and_unavailable_tables() {
        let report = json!({
            "candidates": [{"table":"customers", "columns":[], "candidate_source":""}]
        });
        assert!(select_schema_candidates(&report, &["missing".into()])
            .unwrap_err()
            .contains("no accepted generated candidate"));
        assert!(
            select_schema_candidates(&report, &["customers".into(), "customers".into()])
                .unwrap_err()
                .contains("selected more than once")
        );
        assert!(
            select_schema_candidates(&report, &["customers;drop".into()])
                .unwrap_err()
                .contains("invalid Laravel schema table name")
        );
    }

    #[test]
    fn scaffold_selection_excludes_laravel_runtime_tables() {
        let report = json!({
            "candidates": [
                {"table":"users", "columns":[], "candidate_source":"table users {}"},
                {"table":"sessions", "columns":[], "candidate_source":"table sessions {}"},
                {"table":"jobs", "columns":[], "candidate_source":"table jobs {}"},
                {"table":"orders", "columns":[], "candidate_source":"table orders {}"}
            ]
        });
        let all = select_schema_candidates(&report, &[]).unwrap();
        assert_eq!(all.included_names, ["orders", "users"]);
        assert_eq!(all.excluded_framework_names, ["jobs", "sessions"]);
        assert!(select_schema_candidates(&report, &["sessions".into()])
            .unwrap_err()
            .contains("framework-owned"));
    }

    #[test]
    fn analyzer_ignores_env_and_writes_report_outside_source() {
        let root = env::temp_dir().join(format!(
            "zelyra-laravel-{}-{}",
            std::process::id(),
            rand_core::OsRng.next_u64()
        ));
        fs::create_dir_all(root.join("routes")).unwrap();
        fs::create_dir_all(root.join("vendor")).unwrap();
        fs::write(root.join(".env"), "APP_KEY=must-not-be-read-or-reported").unwrap();
        fs::write(
            root.join("routes/web.php"),
            "<?php\nRoute::get('/orders', [OrderController::class, 'index']);\n",
        )
        .unwrap();
        fs::write(
            root.join("vendor/ignored.php"),
            "Route::get('/secret', fn () => null);",
        )
        .unwrap();
        let report = env::temp_dir().join(format!(
            "zelyra-laravel-report-{}-{}.json",
            std::process::id(),
            rand_core::OsRng.next_u64()
        ));
        let analysis = analyze(&root, &report, "json").unwrap();
        let body = fs::read_to_string(&analysis.report_path).unwrap();
        assert!(body.contains("/orders"));
        assert!(body.contains("Zelyra Laravel Migrator 1.1.5"));
        assert!(body.contains("\"mapping_ruleset\": \"1.1.54\""));
        assert!(!body.contains("must-not-be-read-or-reported"));
        assert!(!body.contains("/secret"));
        assert!(analysis.counts.total >= 1);
        assert_eq!(
            fs::read_to_string(root.join(".env")).unwrap(),
            "APP_KEY=must-not-be-read-or-reported"
        );
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(analysis.report_path);
    }

    #[test]
    fn unavailable_live_database_is_recorded_without_discarding_source_analysis() {
        let root = env::temp_dir().join(format!(
            "zelyra-laravel-live-failure-{}-{}",
            std::process::id(),
            rand_core::OsRng.next_u64()
        ));
        fs::create_dir_all(root.join("routes")).unwrap();
        fs::write(
            root.join("routes/web.php"),
            "<?php Route::get('/orders', fn () => null);",
        )
        .unwrap();
        let report = env::temp_dir().join(format!(
            "zelyra-laravel-live-failure-report-{}-{}.json",
            std::process::id(),
            rand_core::OsRng.next_u64()
        ));
        let unavailable = json!({
            "status": "unavailable",
            "error": "database host could not be resolved",
            "credentials_recorded": false,
            "application_rows_read": false,
            "tables": []
        });
        let analysis =
            analyze_with_live_database(&root, &report, "json", Some(&unavailable)).unwrap();
        let body = fs::read_to_string(&analysis.report_path).unwrap();
        assert!(body.contains("/orders"));
        assert!(body.contains("\"status\": \"unavailable\""));
        assert!(body.contains("\"env_files_read\": true"));
        assert!(body.contains("database host could not be resolved"));
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(analysis.report_path);
    }

    #[test]
    fn scaffold_reports_preserve_explicit_live_inspection_metadata() {
        let root = env::temp_dir().join(format!(
            "zelyra-laravel-inspected-scaffold-{}-{}",
            std::process::id(),
            rand_core::OsRng.next_u64()
        ));
        fs::create_dir_all(root.join("source/routes")).unwrap();
        fs::write(
            root.join("source/routes/web.php"),
            "<?php Route::get('/orders', fn () => null);",
        )
        .unwrap();
        let unavailable = json!({
            "status": "unavailable",
            "error": "database host could not be resolved",
            "credentials_recorded": false,
            "application_rows_read": false,
            "tables": []
        });
        let output = root.join("scaffold");
        let report = root.join("report.json");
        generate_scaffold(
            &root.join("source"),
            Some(&report),
            &output,
            "json",
            &[],
            Some(&unavailable),
        )
        .unwrap();

        let embedded_report = fs::read_to_string(output.join("MIGRATION_REPORT.md")).unwrap();
        assert!(embedded_report.contains("`--inspect-database` was explicitly requested"));
        assert!(!embedded_report.contains("`.env` were not read"));
        let external_report: Value = serde_json::from_slice(&fs::read(&report).unwrap()).unwrap();
        assert_eq!(external_report["env_files_read"], true);
        assert_eq!(
            external_report["live_database_inspection"]["status"],
            "unavailable"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn generated_validation_suggestions_do_not_create_routes_by_default() {
        let root = env::temp_dir().join(format!(
            "zelyra-laravel-validation-scaffold-{}-{}",
            std::process::id(),
            rand_core::OsRng.next_u64()
        ));
        let source = root.join("source");
        fs::create_dir_all(source.join("app/Http/Controllers")).unwrap();
        fs::write(
            source.join("app/Http/Controllers/CustomerController.php"),
            "<?php class CustomerController { public function store(Request $request) { $data = $request->validate(['name' => 'required|string|max:100']); } }",
        )
        .unwrap();
        let output = root.join("scaffold");
        generate_scaffold(&source, None, &output, "json", &[], None).unwrap();

        let selection: Value =
            serde_json::from_slice(&fs::read(output.join("GENERATION_SELECTION.json")).unwrap())
                .unwrap();
        assert!(!selection["validation_forms"].as_array().unwrap().is_empty());
        assert_eq!(selection["validation_module_imported_by_default"], false);
        let main = fs::read_to_string(output.join("main.zyl")).unwrap();
        assert!(!main.contains("src/imported_forms.zyl"));
        assert!(output.join("src/imported_forms.zyl").is_file());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn analyzer_ignores_php_comments_strings_and_heredocs_and_records_exact_spans() {
        let source = "<?php\n// Müller: Route::get('/comment', fn () => null);\n$literal = \"Route::get('/quoted', fn () => null); DB::transaction()\";\n$template = <<<'TEXT'\nRoute::get('/heredoc', fn () => null); Mail::send();\nTEXT;\nRoute::get('/real', fn () => null);\n";
        let mut findings = Vec::new();
        scan_source("routes/web.php", source, &mut findings);

        let routes = findings
            .iter()
            .filter(|finding| finding.kind == "route")
            .collect::<Vec<_>>();
        assert_eq!(routes.len(), 1);
        let route = routes[0];
        assert_eq!(route.name, "/real");
        let span = route.source_span.unwrap();
        let expected = source.find("Route::get('/real'").unwrap();
        assert_eq!(span.start_byte, expected);
        assert_eq!(span.start_line, 7);
        assert_eq!(span.start_column, 1);
        assert_eq!(
            &source[span.start_byte..span.end_byte],
            "Route::get('/real'"
        );
        assert_eq!(finding_classification(route.status), "assisted");
        assert_eq!(
            finding_effect_categories(route.kind),
            &["http", "authorization_review"]
        );
        assert!(!findings.iter().any(|finding| {
            (finding.kind == "transaction" && finding.name == "database transaction")
                || (finding.kind == "integration" && finding.name == "mail integration")
        }));
    }

    #[test]
    fn analyzer_rejects_report_inside_source_and_existing_report() {
        let root = env::temp_dir().join(format!(
            "zelyra-laravel-{}-{}",
            std::process::id(),
            rand_core::OsRng.next_u64()
        ));
        fs::create_dir_all(&root).unwrap();
        let inside = root.join("report.json");
        assert!(analyze(&root, &inside, "json")
            .unwrap_err()
            .contains("outside"));
        let outside = env::temp_dir().join(format!(
            "zelyra-laravel-report-{}-{}.md",
            std::process::id(),
            rand_core::OsRng.next_u64()
        ));
        fs::write(&outside, "existing").unwrap();
        assert!(analyze(&root, &outside, "markdown")
            .unwrap_err()
            .contains("already exists"));
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(outside);
    }

    #[test]
    fn analyzer_labels_business_and_security_work_for_review() {
        let source = r#"<?php
class Customer extends Model {
    function invoices() { return $this->hasMany(Invoice::class); }
}
class CustomerController {}
Schema::create('customers', function ($table) {});
class SendInvoice implements ShouldQueue {}
class InvoicePolicy {}
Gate::authorize('update', $invoice);
$invoice->tenant_id = $tenantId;
$total = $invoice->price * $invoice->quantity;
$path = $request->file('attachment')->store('invoices');
$request->validate([]);
return $this->hasMany(Invoice::class);
Mail::to($email)->send($message);
$customer->charge(5000);
DB::raw("NOW()");
$key = env('STRIPE_SECRET');
"#;
        let mut findings = Vec::new();
        scan_source("app/Customer.php", source, &mut findings);
        assert!(findings
            .iter()
            .any(|item| item.kind == "model" && item.status == "partial"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "controller" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "migration" && item.name == "customers"));
        assert!(findings.iter().any(|item| {
            item.kind == "relationship"
                && item.name == "invoices"
                && item.summary.contains("hasMany")
        }));
        assert!(findings
            .iter()
            .any(|item| item.kind == "validation" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "job" && item.name == "SendInvoice"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "authorization" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "tenant_boundary" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "financial_rule" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "file_access" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "payment_rule" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "ambiguity" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "environment_access" && item.status == "manual"));
        assert!(findings
            .iter()
            .any(|item| item.kind == "integration" && item.name == "mail integration"));
    }

    #[test]
    fn static_schema_table_changes_are_partial_and_dynamic_or_quoted_forms_are_ignored() {
        let source = r#"<?php
Schema::table('booking_services', function ($table) { $table->boolean('active'); });
// Schema::table('comment_only', function ($table) {});
$example = "Schema::table('string_only', function ($table) {})";
Schema::table($runtimeTable, function ($table) { $table->boolean('dynamic'); });
"#;
        let mut findings = Vec::new();
        scan_source("app/Migrations/add_active.php", source, &mut findings);
        let migrations = findings
            .iter()
            .filter(|item| item.kind == "migration")
            .collect::<Vec<_>>();
        assert_eq!(migrations.len(), 1);
        assert_eq!(migrations[0].name, "booking_services");
        assert_eq!(migrations[0].status, "partial");
        assert!(migrations[0].summary.contains("base schema"));
    }

    #[test]
    fn analyzer_reports_unrecognized_php_as_unresolved() {
        let root = env::temp_dir().join(format!(
            "zelyra-laravel-{}-{}",
            std::process::id(),
            rand_core::OsRng.next_u64()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("custom.php"),
            "<?php function custom_pipeline() {}\n",
        )
        .unwrap();
        let report = env::temp_dir().join(format!(
            "zelyra-laravel-report-{}-{}.json",
            std::process::id(),
            rand_core::OsRng.next_u64()
        ));
        let analysis = analyze(&root, &report, "json").unwrap();
        let value: Value =
            serde_json::from_str(&fs::read_to_string(&analysis.report_path).unwrap()).unwrap();
        assert_eq!(value["summary"]["unresolved"], 1);
        assert_eq!(analysis.counts.total, 1);
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(analysis.report_path);
    }
}
