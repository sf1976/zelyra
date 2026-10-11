use super::*;
use regex::Regex;
use std::{
    ops::Range,
    path::{Component, Path},
};

const MAX_SOURCE_FILES: usize = 10_000;
const MAX_SOURCE_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_TOTAL_SOURCE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_DIRECTORY_DEPTH: usize = 64;

#[derive(Clone)]
struct Finding {
    kind: &'static str,
    status: &'static str,
    file: String,
    line: usize,
    name: String,
    summary: String,
    next_step: &'static str,
}

pub(super) fn command(mut arguments: impl Iterator<Item = String>) -> ExitCode {
    if arguments.next().as_deref() != Some("laravel") {
        eprintln!(
            "Usage: zelyra migrate laravel <project> --report <path> [--format markdown|json]"
        );
        return ExitCode::from(2);
    }
    let Some(project) = arguments.next() else {
        eprintln!("error[E-MIGRATE-001]: Laravel project directory is required");
        return ExitCode::from(2);
    };
    let mut report = None;
    let mut format = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--report" => report = arguments.next(),
            "--format" => format = arguments.next(),
            _ => {
                eprintln!("error[E-MIGRATE-001]: unknown migrate option `{argument}`");
                return ExitCode::from(2);
            }
        }
    }
    let Some(report) = report else {
        eprintln!("error[E-MIGRATE-001]: `--report <path>` is required; source analysis never writes into the Laravel project");
        return ExitCode::from(2);
    };
    let project = PathBuf::from(project);
    let report = PathBuf::from(report);
    let format = match format.as_deref() {
        None => match report.extension().and_then(|extension| extension.to_str()) {
            Some("json") => "json",
            _ => "markdown",
        },
        Some("json") => "json",
        Some("markdown") | Some("md") => "markdown",
        Some(other) => {
            eprintln!("error[E-MIGRATE-001]: unsupported report format `{other}`; choose `markdown` or `json`");
            return ExitCode::from(2);
        }
    };
    match analyze(&project, &report, format) {
        Ok((path, counts)) => {
            println!("Laravel source analyzed without executing PHP or reading environment files.");
            println!("Report: {}", path.display());
            println!(
                "Findings: {} · unresolved/manual: {} · partial: {}",
                counts.0, counts.1, counts.2
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error[E-MIGRATE-002]: {error}");
            ExitCode::from(1)
        }
    }
}

fn analyze(
    project: &Path,
    report: &Path,
    format: &str,
) -> Result<(PathBuf, (usize, usize, usize)), String> {
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
    if files.is_empty() {
        return Err("no readable PHP source files were found outside excluded environment and dependency directories".into());
    }
    let total_source_bytes = files.iter().try_fold(0_u64, |total, (path, _)| {
        fs::metadata(path)
            .map(|metadata| total.saturating_add(metadata.len()))
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))
    })?;
    if total_source_bytes > MAX_TOTAL_SOURCE_BYTES {
        return Err("project exceeds the 256 MiB total PHP source analysis limit".into());
    }
    let mut findings = Vec::new();
    let composer = read_composer_metadata(&root)?;
    if let Some(version) = &composer.0 {
        findings.push(Finding { kind: "framework", status: "partial", file: "composer.json".into(), line: 1, name: version.clone(), summary: "Laravel version constraint found in Composer metadata; installed package resolution was not executed.".into(), next_step: "Confirm the installed framework and package versions in a controlled review." });
    }
    for (path, relative) in &files {
        let source = fs::read_to_string(path)
            .map_err(|_| format!("{} is not valid UTF-8 source", relative))?;
        let finding_count = findings.len();
        scan_source(relative, &source, &mut findings);
        if findings.len() == finding_count {
            findings.push(Finding {
                kind: "source_file",
                status: "unresolved",
                file: relative.clone(),
                line: 1,
                name: "no supported pattern recognized".into(),
                summary: "This PHP file contains syntax outside the analyzer's recognized pattern set; its behavior is not described by this report.".into(),
                next_step: "Review this source file for routes, business rules, authorization, or side effects that need manual mapping.",
            });
        }
    }
    if findings.is_empty() {
        findings.push(Finding {
            kind: "project",
            status: "unresolved",
            file: "—".into(),
            line: 0,
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
    let rendered = if format == "json" {
        json_report(&root, &composer.0, &composer.1, &findings)
    } else {
        markdown_report(&root, &composer.0, &composer.1, &findings)
    };
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&canonical_output)
        .map_err(|error| format!("could not create report: {error}"))?;
    use std::io::Write;
    file.write_all(rendered.as_bytes()).map_err(|error| {
        let _ = fs::remove_file(&canonical_output);
        format!("could not write report: {error}")
    })?;
    Ok((canonical_output, (findings.len(), unresolved, partial)))
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

fn read_composer_metadata(root: &Path) -> Result<(Option<String>, Vec<String>), String> {
    let path = root.join("composer.json");
    if !path.is_file() {
        return Ok((None, Vec::new()));
    }
    let source =
        fs::read_to_string(path).map_err(|_| "composer.json is not valid UTF-8".to_owned())?;
    if source.len() as u64 > MAX_SOURCE_FILE_BYTES {
        return Err("composer.json exceeds the 2 MiB file limit".into());
    }
    let value: Value =
        serde_json::from_str(&source).map_err(|_| "composer.json is not valid JSON".to_owned())?;
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
    let relevant = dependencies
        .into_iter()
        .filter(|(name, _)| name.starts_with("laravel/") || name.starts_with("illuminate/"))
        .map(|(name, version)| format!("{name} {version}"))
        .collect();
    Ok((framework, relevant))
}

fn scan_source(file: &str, source: &str, findings: &mut Vec<Finding>) {
    let route = Regex::new(r#"Route\s*::\s*(get|post|put|patch|delete|options|any|resource|apiResource)\s*\(\s*['\"]([^'\"]+)['\"]"#).expect("static route regex");
    for capture in route.captures_iter(source) {
        add_match(findings, file, source, capture.get(0).unwrap().range(), "route", "partial", capture.get(2).unwrap().as_str(), format!("Laravel {} route; chained middleware, policy checks, and runtime route resolution need review.", capture.get(1).unwrap().as_str()), "Review middleware and authorization for this route, then map it to a Zelyra route.");
    }
    let model = Regex::new(
        r"(?m)\bclass\s+(\w+)\s+extends\s+(?:\\?Illuminate\\Database\\Eloquent\\)?Model\b",
    )
    .expect("static model regex");
    for capture in model.captures_iter(source) {
        add_match(
            findings,
            file,
            source,
            capture.get(0).unwrap().range(),
            "model",
            "partial",
            capture.get(1).unwrap().as_str(),
            "Eloquent model detected; casts, scopes, events, and traits are not inferred.",
            "Review casts, scopes, events, traits, and relationships before mapping the model.",
        );
    }
    let controller =
        Regex::new(r"(?m)\bclass\s+(\w*Controller)\b").expect("static controller regex");
    for capture in controller.captures_iter(source) {
        add_match(findings, file, source, capture.get(0).unwrap().range(), "controller", "manual", capture.get(1).unwrap().as_str(), "Controller behavior can include authorization, side effects, and branching that static analysis does not transfer.", "Review each action's business rules and security checks manually.");
    }
    let migration = Regex::new(r#"Schema\s*::\s*create\s*\(\s*['\"]([^'\"]+)['\"]"#)
        .expect("static migration regex");
    for capture in migration.captures_iter(source) {
        add_match(findings, file, source, capture.get(0).unwrap().range(), "migration", "partial", capture.get(1).unwrap().as_str(), "Schema::create table found; column modifiers, indexes, foreign keys, and rollback semantics require review.", "Compare the complete migration and existing data before generating a Zelyra schema.");
    }
    let relation = Regex::new(r"(?s)function\s+(\w+)\s*\([^)]*\)\s*\{\s*return\s+\$this\s*->\s*(belongsTo|hasOne|hasMany|belongsToMany|hasManyThrough)\s*\(").expect("static relationship regex");
    for capture in relation.captures_iter(source) {
        add_match(
            findings,
            file,
            source,
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
    let validation = Regex::new(r"(?m)->validate\s*\(|Validator\s*::\s*make\s*\(")
        .expect("static validation regex");
    for matched in validation.find_iter(source) {
        add_match(findings, file, source, matched.range(), "validation", "manual", "validation rules", "Validation entry point found; individual rules and conditional behavior are not evaluated.", "Translate each rule into a Zelyra form/API contract and test invalid input.");
    }
    let queued = Regex::new(r"(?m)\bclass\s+(\w+)\s+implements\s+[^\n{]*ShouldQueue")
        .expect("static job regex");
    for capture in queued.captures_iter(source) {
        add_match(findings, file, source, capture.get(0).unwrap().range(), "job", "manual", capture.get(1).unwrap().as_str(), "Queued job found; retries, idempotency, transaction boundaries, and failure handling are not transferred.", "Review delivery guarantees and side effects before mapping the job.");
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
            add_match(findings, file, source, matched, kind, "manual", label, format!("{label} call found; credentials, retries, transactions, and error handling require review."), "Identify external effects and their failure/rollback behavior.");
            offset = start + needle.len();
        }
    }
    if file.starts_with("resources/views/") && file.ends_with(".blade.php") {
        findings.push(Finding { kind: "view", status: "manual", file: file.into(), line: 1, name: file.rsplit('/').next().unwrap_or(file).into(), summary: "Blade view found; directives, escaping, components, and client behavior are not translated.".into(), next_step: "Review rendering, output escaping, forms, and JavaScript behavior." });
    }
}

#[allow(clippy::too_many_arguments)]
fn add_match(
    findings: &mut Vec<Finding>,
    file: &str,
    source: &str,
    matched: Range<usize>,
    kind: &'static str,
    status: &'static str,
    name: impl Into<String>,
    summary: impl Into<String>,
    next_step: &'static str,
) {
    let line = source[..matched.start.min(source.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    findings.push(Finding {
        kind,
        status,
        file: file.into(),
        line,
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
) -> String {
    let counts = report_counts(findings);
    let findings = findings.iter().map(|finding| json!({"kind": finding.kind, "status": finding.status, "file": finding.file, "line": finding.line, "name": finding.name, "summary": finding.summary, "next_step": finding.next_step})).collect::<Vec<_>>();
    serde_json::to_string_pretty(&json!({"schema_version": "1", "analyzer": "Zelyra Laravel Migrator 0.9.3", "source_project": root.file_name().and_then(|name| name.to_str()).unwrap_or("Laravel project"), "laravel_constraint": framework, "laravel_packages": packages, "execution": "static_read_only", "env_files_read": false, "summary": counts, "findings": findings})).expect("report JSON is serializable")
}

fn markdown_report(
    root: &Path,
    framework: &Option<String>,
    packages: &[String],
    findings: &[Finding],
) -> String {
    let counts = report_counts(findings);
    let mut report = format!("# Zelyra Laravel migration analysis\n\n- Analyzer: Zelyra Laravel Migrator 0.9.3\n- Project: {}\n- Laravel constraint: {}\n- Laravel packages: {}\n- Method: static, read-only source scan; PHP and Artisan were not run; `.env` files were not read.\n- Findings: {} total · {} partial · {} manual · {} unresolved\n\n", root.file_name().and_then(|name| name.to_str()).unwrap_or("Laravel project"), framework.as_deref().unwrap_or("not detected"), if packages.is_empty() { "none detected".into() } else { packages.join(", ") }, findings.len(), counts["partial"], counts["manual"], counts["unresolved"]);
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
        let (written, counts) = analyze(&root, &report, "json").unwrap();
        let body = fs::read_to_string(&written).unwrap();
        assert!(body.contains("/orders"));
        assert!(!body.contains("must-not-be-read-or-reported"));
        assert!(!body.contains("/secret"));
        assert!(counts.0 >= 1);
        assert_eq!(
            fs::read_to_string(root.join(".env")).unwrap(),
            "APP_KEY=must-not-be-read-or-reported"
        );
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(written);
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
$request->validate([]);
return $this->hasMany(Invoice::class);
Mail::to($email)->send($message);
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
            .any(|item| item.kind == "integration" && item.name == "mail integration"));
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
        let (written, counts) = analyze(&root, &report, "json").unwrap();
        let value: Value = serde_json::from_str(&fs::read_to_string(&written).unwrap()).unwrap();
        assert_eq!(value["summary"]["unresolved"], 1);
        assert_eq!(counts.0, 1);
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(written);
    }
}
